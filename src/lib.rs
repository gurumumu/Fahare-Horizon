//! Experimental native Fahare for dedicated Horizon servers.
mod round;
mod journal;
mod rollover;

use std::{path::PathBuf, sync::{Mutex, atomic::{AtomicBool, Ordering}}};
use mod_api::{self, Command, Flow, Hooks, PlayerDiedEvent, PlayerLoggedInEvent, PlayerRespawnedEvent,
    RegisterCommandsEvent, RunModCommandEvent, ServerHost, ServerStartedEvent, Text};
use round::{Round, Rules};
use journal::Journal;

static ARMED: AtomicBool = AtomicBool::new(false);
static STOP_REQUEST: AtomicBool = AtomicBool::new(false);
static CANDIDATE: Mutex<Option<PathBuf>> = Mutex::new(None);
static EXPECTED: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn take_stop_request() -> bool { STOP_REQUEST.swap(false, Ordering::AcqRel) }

pub fn capture_shutdown(world: Option<PathBuf>) {
    let verified = if ARMED.load(Ordering::Acquire) {
        world.and_then(|w| w.canonicalize().ok()).filter(|w|
            EXPECTED.lock().is_ok_and(|p| p.as_ref() == Some(w)))
    } else { None };
    if let Ok(mut slot) = CANDIDATE.lock() { *slot = verified; }
}

pub fn finish_shutdown(ok: bool) {
    let armed = ARMED.swap(false, Ordering::AcqRel);
    STOP_REQUEST.store(false, Ordering::Release);
    let world = CANDIDATE.lock().ok().and_then(|mut slot| slot.take());
    if let Ok(mut expected) = EXPECTED.lock() { *expected = None; }
    if !armed { return; }
    if !ok {
        eprintln!("[Fahare] Shutdown/save failed; original world retained.");
        return;
    }
    let Some(world) = world else {
        eprintln!("[Fahare] No verified clean dedicated world; original retained.");
        return;
    };
    match rollover::archive(&world) {
        Ok(path) => eprintln!("[Fahare] Archived to {}. Restart the server to generate a new world.", path.display()),
        Err(error) => eprintln!("[Fahare] Archive failed; no world deleted: {error}"),
    }
}

#[derive(Default)]
struct State { dedicated: bool, round: Round, journal: Option<Journal>, error: Option<String> }
#[derive(Default)]
struct Mod { state: Mutex<State> }

fn rules() -> Result<Rules, String> {
    let mut rules = Rules::default();
    let s = match std::fs::read_to_string("fahare.conf") {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(rules),
        Err(e) => return Err(e.to_string()),
    };
    for (i, line) in s.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let error = || format!("invalid fahare.conf line {}", i+1);
        let (key, value) = line.split_once('=').ok_or_else(error)?;
        match (key.trim(), value.trim()) {
            ("lives", value) => {
                rules.lives = value.parse::<u32>().map_err(|_| error())?;
                if !(1..=1000).contains(&rules.lives) { return Err(error()); }
            }
            ("auto-reset", "true") => rules.auto_reset = true,
            ("auto-reset", "false") => rules.auto_reset = false,
            ("any-death", "true") => rules.any_death = true,
            ("any-death", "false") => rules.any_death = false,
            ("backup", "true") => {},
            _ => return Err(error()),
        }
    }
    Ok(rules)
}

fn arm(host: &mut dyn ServerHost) {
    if !ARMED.swap(true, Ordering::AcqRel) {
        let _ = host.messages().broadcast(Text::literal(
            "[Fahare] Round ended. Saving and archiving the world. Restart to play a fresh world."
        ));
        STOP_REQUEST.store(true, Ordering::Release);
    }
}

fn spectator(host: &mut dyn ServerHost, id: u128) {
    let h = format!("{id:032x}");
    let uuid = format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]);
    let _ = host.commands().run_command(&format!("gamemode spectator {uuid}"));
}

impl Mod {
    fn enforce(&self, host: &mut dyn ServerHost, id: u128) {
        let dead = self.state.lock().is_ok_and(|s| s.dedicated && s.error.is_none() && s.round.dead(id));
        if dead { spectator(host, id); }
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_started(&self, host: &mut dyn ServerHost, event: &mut ServerStartedEvent<'_>) {
        ARMED.store(false, Ordering::Release);
        STOP_REQUEST.store(false, Ordering::Release);
        if let Ok(mut p) = CANDIDATE.lock() { *p = None; }
        if let Ok(mut p) = EXPECTED.lock() { *p = None; }
        if let Ok(mut state) = self.state.lock() {
            *state = State { dedicated: event.dedicated, ..State::default() };
            if !event.dedicated { return; }
            let init = (|| -> Result<(PathBuf, Journal, Round), String> {
                let world = host.world_dir().ok_or("missing active world path")?
                    .canonicalize().map_err(|e| e.to_string())?;
                let (journal, round) = Journal::open(&world, rules()?).map_err(|e| e.to_string())?;
                Ok((world, journal, round))
            })();
            match init {
                Ok((world, journal, round)) => {
                    if let Ok(mut p) = EXPECTED.lock() { *p = Some(world); }
                    else { state.error = Some("world identity lock failed".into()); return; }
                    state.journal = Some(journal);
                    state.round = round;
                }
                Err(error) => {
                    eprintln!("[Fahare] Reset disabled: {error}");
                    state.error = Some(error);
                }
            }
        }
    }

    fn player_died(&self, host: &mut dyn ServerHost, event: &mut PlayerDiedEvent<'_>) {
        let id = event.player;
        let online: Vec<u128> = host.players().server_players().iter().map(|p| p.uuid()).collect();
        let (finished, failure) = match self.state.lock() {
            Ok(mut s) if s.dedicated && s.error.is_none() && !s.round.ended && !ARMED.load(Ordering::Acquire) => {
                match s.journal.as_ref().ok_or("missing journal".to_owned()).and_then(|j| j.append(id).map_err(|e| e.to_string())) {
                    Ok(()) => (s.round.record_death(id, &online), None),
                    Err(e) => { s.error = Some(e.clone()); (false, Some(e)) }
                }
            }
            _ => (false, None),
        };
        if let Some(error) = failure {
            eprintln!("[Fahare] Reset disabled: {error}");
            let _ = host.messages().broadcast(Text::literal("[Fahare] Life journal failed. Automatic reset disabled."));
        }
        if finished { arm(host); }
    }

    fn player_respawned(&self, host: &mut dyn ServerHost, event: &mut PlayerRespawnedEvent<'_>) {
        self.enforce(host, event.player);
    }

    fn player_logged_in(&self, host: &mut dyn ServerHost, event: &mut PlayerLoggedInEvent<'_>) {
        self.enforce(host, event.player);
    }

    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::op("fahare reset", 4));
        event.commands.push(Command::new("fahare status"));
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        let literals = &event.command.literals;
        if literals.first().map(|s| s.as_str()) != Some("fahare") { return Flow::Continue; }
        match literals.get(1).map(|s| s.as_str()) {
            Some("reset") => {
                if event.source.permission().level() < 4 {
                    let _ = host.commands().feedback(Text::literal("Permission denied."));
                } else if self.state.lock().is_ok_and(|s| s.dedicated && s.error.is_none() && s.journal.is_some()) {
                    arm(host);
                } else {
                    let _ = host.commands().feedback(Text::literal("Reset disabled: dedicated server and valid life journal required."));
                }
            }
            Some("status") => {
                let message = match self.state.lock() {
                    Ok(s) => format!("Fahare 0.3.1 | dedicated: {} | lives: {} | auto-reset: {} | any-death: {} | reset pending: {} | journal: {}",
                        s.dedicated, s.round.rules.lives, s.round.rules.auto_reset, s.round.rules.any_death,
                        ARMED.load(Ordering::Acquire), s.error.as_deref().unwrap_or(if s.journal.is_some() {"ready"} else {"inactive"})),
                    Err(_) => "Fahare state unavailable.".to_owned(),
                };
                let _ = host.commands().feedback(Text::literal(message));
            }
            _ => return Flow::Continue,
        }
        Flow::Cancel
    }
}
