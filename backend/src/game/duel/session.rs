use crate::game::core::Word;

/// A single round in the game
pub struct Round {
    pub number: u32,
    pub word: Word,
}

impl Round {
    pub fn check_answer(&self, answer: &str) -> bool {
        answer == self.word.reading
    }
}

/// Result of a round ending
#[derive(Debug, PartialEq)]
pub struct RoundOutcome {
    pub winner: Option<String>,
    pub correct_reading: String,
}

/// A game session between two players (pure logic, no I/O)
pub struct GameSession {
    pub player1: String,
    pub player2: String,
    current_round: Option<Round>,
}

impl GameSession {
    pub fn new(player1: String, player2: String) -> Self {
        Self {
            player1,
            player2,
            current_round: None,
        }
    }

    pub fn has_player(&self, player_id: &str) -> bool {
        self.player1 == player_id || self.player2 == player_id
    }

    pub fn opponent_of(&self, player_id: &str) -> Option<&str> {
        if player_id == self.player1 {
            Some(&self.player2)
        } else if player_id == self.player2 {
            Some(&self.player1)
        } else {
            None
        }
    }

    /// Start a new round with the given word
    pub fn start_round(&mut self, round_number: u32, word: Word) {
        self.current_round = Some(Round {
            number: round_number,
            word,
        });
    }

    /// Submit an answer. Returns Some(outcome) if this ends the round.
    pub fn submit_answer(&mut self, player_id: &str, answer: &str) -> Option<RoundOutcome> {
        let round = self.current_round.as_ref()?;

        if !round.check_answer(answer) {
            return None;
        }

        // Correct answer - end the round
        let correct_reading = round.word.reading.clone();
        self.current_round = None;

        Some(RoundOutcome {
            winner: Some(player_id.to_string()),
            correct_reading,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_session_tracks_players() {
        let session = GameSession::new("alice".to_string(), "bob".to_string());

        assert!(session.has_player("alice"));
        assert!(session.has_player("bob"));
        assert!(!session.has_player("charlie"));
    }

    #[test]
    fn test_opponent_of_returns_other_player() {
        let session = GameSession::new("alice".to_string(), "bob".to_string());

        assert_eq!(session.opponent_of("alice"), Some("bob"));
        assert_eq!(session.opponent_of("bob"), Some("alice"));
        assert_eq!(session.opponent_of("charlie"), None);
    }

    #[test]
    fn test_correct_answer_wins_round() {
        let mut session = GameSession::new("alice".to_string(), "bob".to_string());
        let word = Word {
            kanji: "日本".to_string(),
            reading: "にほん".to_string(),
        };

        session.start_round(1, word);

        // Wrong answer - round continues
        let result = session.submit_answer("alice", "にっぽん");
        assert!(result.is_none());

        // Correct answer - round ends
        let result = session.submit_answer("bob", "にほん");
        assert!(result.is_some());

        let outcome = result.unwrap();
        assert_eq!(outcome.winner, Some("bob".to_string()));
        assert_eq!(outcome.correct_reading, "にほん");
    }
}
