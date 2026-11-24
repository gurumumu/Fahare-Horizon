//! Experimental, non-destructive Rust port of Fahare (MIT).
//! Does not regenerate vanilla worlds or clear player data.
mod round;

use std::sync::Mutex;
use mod_api::{self, Command, Flow, Hooks, PlayerRespawnedEvent,
    RegisterCommandsEvent, RunModCommandEvent, ServerHost, ServerStartedEvent, Text};
use round::Round;

#[derive(Default)]
struct Mod {
    round: Mutex<Round>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn server_started(&self, _: &mut dyn ServerHost, _: &mut ServerStartedEvent<'_>) {
        if let Ok(mut round) = self.round.lock() {
            *round = Round::default();
        }
    }

    fn player_respawned(&self, host: &mut dyn ServerHost, event: &mut PlayerRespawnedEvent<'_>) {
        let Some(player) = host.player(event.player) else { return; };
        let id = format!("{:?}", player.uuid());
        let online: Vec<String> = host.players().server_players().iter()
            .map(|p| format!("{:?}", p.uuid())).collect();
        let finished = match self.round.lock() {
            Ok(mut round) => round.record_death(id, &online),
            Err(_) => return,
        };
        if finished {
            let _ = host.messages().broadcast(Text::literal(
                "[Fahare] Round ended. Automatic world regeneration is not supported by this preview. No world files were changed."
            ));
        }
    }

    fn register_commands(&self, _: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::op("fahare reset", 4));
        event.commands.push(Command::new("fahare status"));
    }

    fn run_mod_command(&self, host: &mut dyn ServerHost, event: &mut RunModCommandEvent<'_>) -> Flow {
        let literals = &event.command.literals;
        if literals.first().map(|s| s.as_str()) != Some("fahare") {
            return Flow::Continue;
        }
        let message = match literals.get(1).map(|s| s.as_str()) {
            Some("reset") => "World reset is disabled: this preview never deletes or modifies world files.".to_owned(),
            Some("status") => match self.round.lock() {
                Ok(round) => format!("Fahare preview | lives: {} | round ended: {} | world reset unavailable", round.lives, round.ended),
                Err(_) => "Fahare state unavailable.".to_owned(),
            },
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(message));
        Flow::Cancel
    }
}
