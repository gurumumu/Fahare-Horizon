use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct Rules {
    pub lives: u32,
    pub auto_reset: bool,
    pub any_death: bool,
}
impl Default for Rules {
    fn default() -> Self {
        Self { lives: 1, auto_reset: true, any_death: false }
    }
}

#[derive(Default)]
pub struct Round {
    deaths: HashMap<u128, u32>,
    pub rules: Rules,
    pub ended: bool,
}
impl Round {
    pub fn deaths(&self, id: u128) -> u32 {
        self.deaths.get(&id).copied().unwrap_or(0)
    }
    pub fn dead(&self, id: u128) -> bool {
        self.deaths(id) >= self.rules.lives
    }
    pub fn replay_death(&mut self, id: u128) {
        let count = self.deaths.entry(id).or_default();
        *count = count.saturating_add(1);
    }
    pub fn record_death(&mut self, player: u128, online: &[u128]) -> bool {
        if self.ended { return false; }
        self.replay_death(player);
        if !self.rules.auto_reset { return false; }
        let finished = self.dead(player) && (self.rules.any_death ||
            (!online.is_empty() && online.iter().all(|id| self.dead(*id))));
        self.ended = finished;
        finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn all_online() {
        let mut r = Round::default();
        assert!(!r.record_death(1, &[1,2]));
        assert!(r.record_death(2, &[1,2]));
        assert!(!r.record_death(2, &[1,2]));
    }
    #[test] fn no_empty_reset() {
        let mut r = Round::default();
        assert!(!r.record_death(1, &[]));
    }
    #[test] fn lives() {
        let mut r = Round {rules: Rules {lives: 2, ..Rules::default()}, ..Round::default()};
        assert!(!r.record_death(1, &[1]));
        assert!(r.record_death(1, &[1]));
    }
    #[test] fn disabled() {
        let mut r = Round {rules: Rules {auto_reset: false, any_death: true, ..Rules::default()}, ..Round::default()};
        assert!(!r.record_death(1, &[1]));
        assert!(r.dead(1));
    }
    #[test] fn any_death_respects_lives() {
        let mut r = Round {rules: Rules {lives: 2, any_death: true, ..Rules::default()}, ..Round::default()};
        assert!(!r.record_death(1, &[1,2]));
        assert!(r.record_death(1, &[1,2]));
    }
    #[test] fn restored_lives() {
        let mut r = Round::default();
        r.replay_death(1);
        assert!(r.record_death(2, &[1,2]));
    }
}
