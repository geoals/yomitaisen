use super::lobby::{Lobby, MatchOutcome};
use crate::game::core::WordRepository;
use crate::game::duel::active_game::{ActiveGame, DEFAULT_ROUND_TIMEOUT};
use crate::game::duel::messages::ServerMessage;
use crate::game::duel::session::GameSession;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, info};

/// Internal result after matching + channel lookup
pub enum JoinResult {
    Waiting,
    Matched {
        opponent_id: String,
        opponent_tx: broadcast::Sender<ServerMessage>,
        game_id: String,
    },
}

pub struct MatchmakingState {
    pub words: WordRepository,
    pub lobby: Lobby,
    pub player_channels: DashMap<String, broadcast::Sender<ServerMessage>>,
    pub games: Arc<DashMap<String, ActiveGame>>,
    pub player_games: Arc<DashMap<String, String>>, // user_id -> game_id
    pub round_timeout: Duration,
}

impl MatchmakingState {
    pub fn new(words: WordRepository) -> Self {
        Self {
            words,
            lobby: Lobby::new(),
            player_channels: DashMap::new(),
            games: Arc::new(DashMap::new()),
            player_games: Arc::new(DashMap::new()),
            round_timeout: DEFAULT_ROUND_TIMEOUT,
        }
    }

    pub fn with_round_timeout(mut self, timeout: Duration) -> Self {
        self.round_timeout = timeout;
        self
    }

    fn register_player(&self, user_id: &str, tx: broadcast::Sender<ServerMessage>) {
        debug!(user_id, "Registering player channel");
        self.player_channels.insert(user_id.to_string(), tx);
    }

    pub fn try_join(&self, user_id: String, tx: broadcast::Sender<ServerMessage>) -> JoinResult {
        // Register this player's channel
        self.register_player(&user_id, tx.clone());

        // Try matchmaking
        match self.lobby.try_match(user_id.clone()) {
            MatchOutcome::Waiting => {
                info!(user_id, "Player waiting for opponent");
                JoinResult::Waiting
            }
            MatchOutcome::Matched { opponent_id } => {
                info!(user_id, opponent_id, "Players matched");

                // Look up opponent's channel
                let opponent_tx = self
                    .player_channels
                    .get(&opponent_id)
                    .map(|r| r.clone())
                    .expect("opponent should have registered channel");

                // Create game
                let game_id = uuid::Uuid::new_v4().to_string();
                debug!(game_id, user_id, opponent_id, "Creating game");

                let session = GameSession::new(opponent_id.clone(), user_id.clone());
                let game = ActiveGame::new(session, opponent_tx.clone(), tx);

                self.games.insert(game_id.clone(), game);
                self.player_games
                    .insert(opponent_id.clone(), game_id.clone());
                self.player_games.insert(user_id, game_id.clone());

                JoinResult::Matched {
                    opponent_id,
                    opponent_tx,
                    game_id,
                }
            }
        }
    }

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

    pub fn handle_disconnect(&self, user_id: &str) {
        info!(user_id, "Player disconnected");

        // Remove from lobby if waiting
        self.lobby.remove_waiting(user_id);

        // Remove player channel
        self.player_channels.remove(user_id);

        // If in a game, notify opponent and clean up
        let Some((_, game_id)) = self.player_games.remove(user_id) else {
            return;
        };

        // Get opponent before removing game
        let opponent_id = {
            let Some(game) = self.games.get(&game_id) else {
                return;
            };
            game.session.opponent_of(user_id).map(|s| s.to_string())
        };

        // Remove game
        self.games.remove(&game_id);

        // Notify opponent and clean up their mapping
        if let Some(opponent_id) = opponent_id {
            self.player_games.remove(&opponent_id);

            if let Some(opponent_tx) = self.player_channels.get(&opponent_id) {
                let _ = opponent_tx.send(ServerMessage::OpponentDisconnected);
            }
        }
    }

    /// Clean up game state after game ends
    pub fn cleanup_game(&self, game_id: &str) {
        if let Some((_, game)) = self.games.remove(game_id) {
            self.player_games.remove(&game.session.player1);
            self.player_games.remove(&game.session.player2);
        }
    }
}
