//! Unofficial native adaptation of Fahare. Rollover only on a dedicated server,
//! after the engine completes its normal save and shutdown. Never delete a save.
mod round;
mod rollover;

use std::{path::PathBuf, sync::{Mutex, atomic::{AtomicBool, Ordering}}};
use mod_api::{self, Command, Flow, Hooks, PlayerDiedEvent, PlayerRespawnedEvent,
    RegisterCommandsEvent, RunModCommandEvent, ServerHost, ServerStartedEvent, Text};
use round::{Round, Rules};

static ARMED: AtomicBool = AtomicBool::new(false);
static STOP_REQUEST: AtomicBool = AtomicBool::new(false);
static CANDIDATE: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn take_stop_request() -> bool { STOP_REQUEST.swap(false, Ordering::AcqRel) }

pub fn capture_shutdown(world: Option<PathBuf>) {
    if let Ok(mut slot) = CANDIDATE.lock() {
        *slot = if ARMED.load(Ordering::Acquire) { world } else { None };
    }
}

pub fn finish_shutdown(ok: bool) {
    let armed = ARMED.swap(false, Ordering::AcqRel);
    STOP_REQUEST.store(false, Ordering::Release);
    let world = CANDIDATE.lock().ok().and_then(|mut slot| slot.take());
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
struct State { dedicated: bool, round: Round }
#[derive(Default)]
struct Mod { state: Mutex<State> }

// Deliberately strict settings. The default matches the original plugin.
fn rules() -> Rules {
    let mut rules = Rules::default();
    if let Ok(s) = std::fs::read_to_string("fahare.conf") {
        for line in s.lines() {
            let Some((key, value)) = line.split_once('=') else { continue; };
            match (key.trim(), value.trim()) {
                ("lives", value) => if let Ok(n) = value.parse::<u32>() { rules.lives = n.clamp(1, 1000); },
                ("auto-reset", "true") => rules.auto_reset = true,
                ("auto-reset", "false") => rules.auto_reset = false,
                ("any-death", "true") => rules.any_death = true,
                ("any-death", "false") => rules.any_death = false,
                _ => {}
            }
        }
    }
    rules
}

fn arm(host: &mut dyn ServerHost) {
    if !ARMED.swap(true, Ordering::AcqRel) {
        let _ = host.messages().broadcast(Text::literal(
            "[Fahare] Round ended. The dedicated server will save and shut down. Restart to play a fresh world."
        ));
        STOP_REQUEST.store(true, Ordering::Release);
    }
}

fn spectator(host: &mut dyn ServerHost, id: u128) {
    // UUID is fixed hexadecimal, never user-controlled command text.
    let h = format!("{id:032x}");
    let uuid = format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32]);
    let _ = host.commands().run_command(&format!("gamemode spectator {uuid}"));
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_started(&self, _: &mut dyn ServerHost, event: &mut ServerStartedEvent<'_>) {
        ARMED.store(false, Ordering::Release);
        STOP_REQUEST.store(false, Ordering::Release);
        if let Ok(mut state) = self.state.lock() {
            *state = State { dedicated: event.dedicated, round: Round::default() };
            state.round.rules = rules();
        }
    }

    fn player_died(&self, host: &mut dyn ServerHost, event: &mut PlayerDiedEvent<'_>) {
        let Some(player) = host.player(event.player) else { return; };
        let id = player.uuid();
        let online: Vec<u128> = host.players().server_players().iter().map(|p| p.uuid()).collect();
        let finished = match self.state.lock() {
            Ok(mut state) if state.dedicated => state.round.record_death(id, &online),
            _ => false,
        };
        if finished { arm(host); }
    }

    fn player_respawned(&self, host: &mut dyn ServerHost, event: &mut PlayerRespawnedEvent<'_>) {
        let Some(player) = host.player(event.player) else { return; };
        let id = player.uuid();
        let dead = self.state.lock().is_ok_and(|s| s.dedicated && s.round.dead(id));
        if dead { spectator(host, id); }
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
                } else if self.state.lock().is_ok_and(|s| s.dedicated) {
                    arm(host);
                } else {
                    let _ = host.commands().feedback(Text::literal("Dedicated server only."));
                }
            }
            Some("status") => {
                let message = match self.state.lock() {
                    Ok(s) => format!("Fahare 0.2 | dedicated: {} | lives: {} | auto-reset: {} | any-death: {} | reset pending: {}",
                        s.dedicated, s.round.rules.lives, s.round.rules.auto_reset, s.round.rules.any_death, ARMED.load(Ordering::Acquire)),
                    Err(_) => "Fahare state unavailable.".to_owned(),
                };
                let _ = host.commands().feedback(Text::literal(message));
            }
            _ => return Flow::Continue,
        }
        Flow::Cancel
    }
}
