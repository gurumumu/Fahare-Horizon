use std::collections::HashMap;

pub struct Round {
    deaths: HashMap<String, u32>,
    pub lives: u32,
    pub auto_reset: bool,
    pub any_death: bool,
    pub ended: bool,
}

impl Default for Round {
    fn default() -> Self {
        Self { deaths: HashMap::new(), lives: 1, auto_reset: true, any_death: false, ended: false }
    }
}

impl Round {
    pub fn deaths(&self, player: &str) -> u32 {
        self.deaths.get(player).copied().unwrap_or(0)
    }

    pub fn record_death(&mut self, player: String, online: &[String]) -> bool {
        let count = self.deaths.entry(player).or_default();
        *count = count.saturating_add(1);
        if self.ended || !self.auto_reset { return false; }
        let finished = self.any_death || (!online.is_empty() &&
            online.iter().all(|id| self.deaths(id) >= self.lives));
        self.ended = finished;
        finished
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waits_for_every_online_player() {
        let mut round = Round::default();
        let online = vec!["a".to_owned(), "b".to_owned()];
        assert!(!round.record_death("a".into(), &online));
        assert!(round.record_death("b".into(), &online));
        assert!(!round.record_death("b".into(), &online));
    }

    #[test]
    fn empty_online_list_does_not_end_round() {
        let mut round = Round::default();
        assert!(!round.record_death("a".into(), &[]));
    }

    #[test]
    fn respects_lives() {
        let mut round = Round { lives: 2, ..Round::default() };
        let online = vec!["a".to_owned()];
        assert!(!round.record_death("a".into(), &online));
        assert!(round.record_death("a".into(), &online));
    }

    #[test]
    fn respects_auto_reset() {
        let mut round = Round { auto_reset: false, any_death: true, ..Round::default() };
        assert!(!round.record_death("a".into(), &["a".into()]));
    }
}
