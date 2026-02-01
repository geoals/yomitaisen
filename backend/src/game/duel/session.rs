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

const WINS_NEEDED: u32 = 2;

/// A game session between two players (pure logic, no I/O)
pub struct GameSession {
    pub player1: String,
    pub player2: String,
    scores: (u32, u32),
    current_round: Option<Round>,
}

impl GameSession {
    pub fn new(player1: String, player2: String) -> Self {
        Self {
            player1,
            player2,
            scores: (0, 0),
            current_round: None,
        }
    }

    pub fn scores(&self) -> (u32, u32) {
        self.scores
    }

    pub fn record_win(&mut self, player_id: &str) {
        match player_id {
            id if id == self.player1 => self.scores.0 += 1,
            id if id == self.player2 => self.scores.1 += 1,
            _ => {}
        }
    }

    pub fn game_winner(&self) -> Option<&str> {
        match self.scores {
            (p1, _) if p1 >= WINS_NEEDED => Some(&self.player1),
            (_, p2) if p2 >= WINS_NEEDED => Some(&self.player2),
            _ => None,
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

    /// Timeout the current round. Returns Some(outcome) if there was an active round.
    pub fn timeout_round(&mut self) -> Option<RoundOutcome> {
        let round = self.current_round.take()?;

        Some(RoundOutcome {
            winner: None,
            correct_reading: round.word.reading,
        })
    }

    /// Get the current round number, if any
    pub fn current_round_number(&self) -> Option<u32> {
        self.current_round.as_ref().map(|r| r.number)
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

    #[test]
    fn test_timeout_round_ends_with_no_winner() {
        let mut session = GameSession::new("alice".to_string(), "bob".to_string());
        let word = Word {
            kanji: "日本".to_string(),
            reading: "にほん".to_string(),
        };

        session.start_round(1, word);

        let result = session.timeout_round();
        assert!(result.is_some());

        let outcome = result.unwrap();
        assert_eq!(outcome.winner, None);
        assert_eq!(outcome.correct_reading, "にほん");
    }

    #[test]
    fn test_timeout_round_returns_none_if_no_active_round() {
        let mut session = GameSession::new("alice".to_string(), "bob".to_string());

        let result = session.timeout_round();
        assert!(result.is_none());
    }

    #[test]
    fn test_first_to_two_wins_game() {
        let mut session = GameSession::new("alice".to_string(), "bob".to_string());

        assert_eq!(session.scores(), (0, 0));
        assert_eq!(session.game_winner(), None);

        session.record_win("alice");
        assert_eq!(session.scores(), (1, 0));
        assert_eq!(session.game_winner(), None);

        session.record_win("bob");
        assert_eq!(session.scores(), (1, 1));
        assert_eq!(session.game_winner(), None);

        session.record_win("alice");
        assert_eq!(session.scores(), (2, 1));
        assert_eq!(session.game_winner(), Some("alice"));
    }
}
