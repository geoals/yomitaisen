use crate::game::core::Word;

/// A single round in the game
pub struct Round {
    pub number: u32,
    pub word: Word,
    pub player1_skipped: bool,
    pub player2_skipped: bool,
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

/// Result of a player attempting to skip
#[derive(Debug, PartialEq)]
pub enum SkipResult {
    /// Player already skipped this round
    AlreadySkipped,
    /// Waiting for opponent to also skip
    WaitingForOpponent,
    /// Both players skipped - round ends
    BothSkipped(RoundOutcome),
}

const WINS_NEEDED: u32 = 15;

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
            player1_skipped: false,
            player2_skipped: false,
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

    /// Record a player skipping the round. Returns the result of the skip attempt.
    pub fn record_skip(&mut self, player_id: &str) -> Option<SkipResult> {
        let round = self.current_round.as_mut()?;

        // Mark this player as skipped
        let (already_skipped, opponent_skipped) = if player_id == self.player1 {
            let already = round.player1_skipped;
            round.player1_skipped = true;
            (already, round.player2_skipped)
        } else if player_id == self.player2 {
            let already = round.player2_skipped;
            round.player2_skipped = true;
            (already, round.player1_skipped)
        } else {
            return None;
        };

        if already_skipped {
            return Some(SkipResult::AlreadySkipped);
        }

        if opponent_skipped {
            // Both players have now skipped - end the round
            let round = self.current_round.take()?;
            Some(SkipResult::BothSkipped(RoundOutcome {
                winner: None,
                correct_reading: round.word.reading,
            }))
        } else {
            Some(SkipResult::WaitingForOpponent)
        }
    }

    /// Get the current round number, if any
    pub fn current_round_number(&self) -> Option<u32> {
        self.current_round.as_ref().map(|r| r.number)
    }

    /// Get the current kanji being tested, if there's an active round
    pub fn current_kanji(&self) -> Option<&str> {
        self.current_round.as_ref().map(|r| r.word.kanji.as_str())
    }

    /// Accept a correct answer without validation (used when answer was validated externally).
    /// Returns Some(outcome) if there was an active round.
    pub fn accept_correct_answer(&mut self, player_id: &str) -> Option<RoundOutcome> {
        let round = self.current_round.take()?;
        let correct_reading = round.word.reading;

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
    fn test_skip_requires_both_players() {
        let mut session = GameSession::new("alice".to_string(), "bob".to_string());
        let word = Word {
            kanji: "日本".to_string(),
            reading: "にほん".to_string(),
        };

        session.start_round(1, word);

        // First player skips - should wait for opponent
        let result = session.record_skip("alice");
        assert_eq!(result, Some(SkipResult::WaitingForOpponent));

        // Same player skips again - already skipped
        let result = session.record_skip("alice");
        assert_eq!(result, Some(SkipResult::AlreadySkipped));

        // Second player skips - round ends
        let result = session.record_skip("bob");
        assert!(matches!(result, Some(SkipResult::BothSkipped(_))));

        if let Some(SkipResult::BothSkipped(outcome)) = result {
            assert_eq!(outcome.winner, None);
            assert_eq!(outcome.correct_reading, "にほん");
        }
    }

    #[test]
    fn test_first_to_fifteen_wins_game() {
        let mut session = GameSession::new("alice".to_string(), "bob".to_string());

        assert_eq!(session.scores(), (0, 0));
        assert_eq!(session.game_winner(), None);

        // Record 14 wins for alice - should not trigger game end yet
        for i in 1..=14 {
            session.record_win("alice");
            assert_eq!(session.scores(), (i, 0));
            assert_eq!(session.game_winner(), None);
        }

        // Bob gets some wins but alice is still ahead
        session.record_win("bob");
        assert_eq!(session.scores(), (14, 1));
        assert_eq!(session.game_winner(), None);

        // 15th win for alice triggers game end
        session.record_win("alice");
        assert_eq!(session.scores(), (15, 1));
        assert_eq!(session.game_winner(), Some("alice"));
    }
}
