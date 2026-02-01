use super::active_game::{
    continue_or_end_game, spawn_round_timeout, ActiveGame, AnswerResult, DEFAULT_ROUND_TIMEOUT,
};
use crate::game::core::messages::ServerMessage;
use crate::game::core::WordRepository;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Info returned when a player is removed from a game due to disconnect
pub struct DisconnectInfo {
    pub game: ActiveGame,
    pub opponent_id: String,
}

/// Shared game state management used by both ephemeral and matchmaking modes.
/// Handles active games, player-to-game mapping, answer submission, and cleanup.
pub struct GameRegistry {
    pub words: WordRepository,
    pub games: Arc<DashMap<String, ActiveGame>>,
    pub player_games: Arc<DashMap<String, String>>, // player_id -> game_id
    pub round_timeout: Duration,
}

impl GameRegistry {
    pub fn new(words: WordRepository, round_timeout: Option<Duration>) -> Self {
        Self {
            words,
            games: Arc::new(DashMap::new()),
            player_games: Arc::new(DashMap::new()),
            round_timeout: round_timeout.unwrap_or(DEFAULT_ROUND_TIMEOUT),
        }
    }

    /// Broadcast a message to both players in a game
    pub fn broadcast_to_game(&self, user_id: &str, msg: ServerMessage) {
        let Some(game_id) = self.player_games.get(user_id) else {
            return;
        };
        let Some(game) = self.games.get(&*game_id) else {
            return;
        };
        debug!(?game_id, "Broadcasting to game");
        game.broadcast(msg);
    }

    /// Clean up game state after game ends
    pub fn cleanup_game(&self, game_id: &str) {
        if let Some((_, game)) = self.games.remove(game_id) {
            self.player_games.remove(&game.session.player1);
            self.player_games.remove(&game.session.player2);
        }
    }

    /// Remove a player from their game due to disconnect.
    /// Returns the game and opponent info so the caller can send notifications.
    pub fn remove_player_from_game(&self, user_id: &str) -> Option<DisconnectInfo> {
        let (_, game_id) = self.player_games.remove(user_id)?;
        let (_, game) = self.games.remove(&game_id)?;
        let opponent_id = game.session.opponent_of(user_id)?.to_string();
        self.player_games.remove(&opponent_id);
        Some(DisconnectInfo { game, opponent_id })
    }

    /// Submit an answer and return the result if correct
    pub fn submit_answer(&self, user_id: &str, answer: &str) -> Option<AnswerResult> {
        let game_id = self.player_games.get(user_id)?;
        let mut game = self.games.get_mut(&*game_id)?;

        debug!(user_id, answer, "Player submitting answer");

        let outcome = game.session.submit_answer(user_id, answer)?;

        // Record the win
        if let Some(winner) = &outcome.winner {
            game.session.record_win(winner);
        }

        let scores = game.session.scores();
        let game_winner = game.session.game_winner().map(|s| s.to_string());

        info!(
            user_id,
            round_winner = ?outcome.winner,
            scores = ?scores,
            game_winner = ?game_winner,
            "Round ended"
        );

        let round_number = scores.0 + scores.1;

        Some(AnswerResult {
            round_result: ServerMessage::RoundResult {
                winner: outcome.winner,
                correct_reading: outcome.correct_reading,
            },
            game_winner,
            round_number,
        })
    }

    /// Handle answer submission: check answer, broadcast result, continue or end game
    pub async fn handle_answer(
        self: &Arc<Self>,
        user_id: &str,
        answer: &str,
        tx: &broadcast::Sender<ServerMessage>,
    ) {
        let Some(result) = self.submit_answer(user_id, answer) else {
            debug!(user_id, answer, "Wrong answer");
            let _ = tx.send(ServerMessage::WrongAnswer);
            return;
        };

        // Broadcast round result to both players
        self.broadcast_to_game(user_id, result.round_result);

        // Get game_id for continue_or_end_game
        let Some(game_id) = self.player_games.get(user_id).map(|r| r.clone()) else {
            return;
        };

        let registry = self.clone();
        continue_or_end_game(
            &self.games,
            &self.words,
            self.round_timeout,
            &game_id,
            result.game_winner,
            result.round_number,
            &self.player_games,
            Arc::new(move |id: &str| registry.cleanup_game(id)),
        )
        .await;
    }

    /// Start round 1 for a newly created game
    pub async fn start_first_round(
        self: &Arc<Self>,
        game_id: &str,
        player1_tx: &broadcast::Sender<ServerMessage>,
        player2_tx: &broadcast::Sender<ServerMessage>,
    ) {
        let Some(word) = self.words.get_random().await else {
            return;
        };

        info!(
            game_id,
            kanji = word.kanji,
            reading = word.reading,
            "Round 1 starting"
        );

        let round_msg = ServerMessage::RoundStart {
            kanji: word.kanji.clone(),
            round: 1,
        };
        let _ = player1_tx.send(round_msg.clone());
        let _ = player2_tx.send(round_msg);

        if let Some(mut game) = self.games.get_mut(game_id) {
            game.session.start_round(1, word);
        }

        let registry = self.clone();
        spawn_round_timeout(
            self.round_timeout,
            self.games.clone(),
            self.words.clone(),
            game_id.to_string(),
            1,
            self.player_games.clone(),
            Arc::new(move |id: &str| registry.cleanup_game(id)),
        );
    }
}
