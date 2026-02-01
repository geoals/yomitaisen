use std::sync::Mutex;

/// Result of attempting to join matchmaking
#[derive(Debug, PartialEq)]
pub enum MatchOutcome {
    Waiting,
    Matched { opponent_id: String },
}

/// Matchmaking queue (pure, no transport concerns)
pub struct Lobby {
    waiting: Mutex<Option<String>>,
}

impl Lobby {
    pub fn new() -> Self {
        Self {
            waiting: Mutex::new(None),
        }
    }

    /// Try to match a player. Returns outcome.
    pub fn try_match(&self, user_id: String) -> MatchOutcome {
        let mut waiting = self.waiting.lock().unwrap();

        let Some(opponent_id) = waiting.take() else {
            *waiting = Some(user_id);
            return MatchOutcome::Waiting;
        };

        MatchOutcome::Matched { opponent_id }
    }

    /// Remove a player from waiting (on disconnect)
    pub fn remove_waiting(&self, user_id: &str) {
        let mut waiting = self.waiting.lock().unwrap();
        if waiting.as_deref() == Some(user_id) {
            *waiting = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_player_waits() {
        let lobby = Lobby::new();

        let result = lobby.try_match("alice".to_string());

        assert_eq!(result, MatchOutcome::Waiting);
    }

    #[test]
    fn test_second_player_matches_with_first() {
        let lobby = Lobby::new();

        lobby.try_match("alice".to_string());
        let result = lobby.try_match("bob".to_string());

        assert_eq!(
            result,
            MatchOutcome::Matched {
                opponent_id: "alice".to_string()
            }
        );
    }

    #[test]
    fn test_third_player_waits_after_match() {
        let lobby = Lobby::new();

        lobby.try_match("alice".to_string());
        lobby.try_match("bob".to_string()); // matches with alice

        let result = lobby.try_match("charlie".to_string());
        assert_eq!(result, MatchOutcome::Waiting);
    }

    #[test]
    fn test_remove_waiting_clears_queue() {
        let lobby = Lobby::new();

        lobby.try_match("alice".to_string());
        lobby.remove_waiting("alice");

        // bob should wait, not match
        let result = lobby.try_match("bob".to_string());
        assert_eq!(result, MatchOutcome::Waiting);
    }
}
