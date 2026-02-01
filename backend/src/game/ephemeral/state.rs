use super::game_id::generate_unique_game_id;
use super::pending_game::PendingGame;
use super::player::EphemeralPlayer;
use crate::game::core::WordRepository;
use crate::game::duel::active_game::ActiveGame;
use crate::game::duel::messages::ServerMessage;
use crate::game::duel::registry::GameRegistry;
use crate::game::duel::session::GameSession;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::info;

/// Result of joining an ephemeral game
pub struct JoinedGame {
    pub game_id: String,
    pub host_name: String,
    pub guest_name: String,
    pub host_tx: broadcast::Sender<ServerMessage>,
}

pub struct EphemeralState {
    pub registry: Arc<GameRegistry>,
    pub pending_games: DashMap<String, PendingGame>,
}

impl EphemeralState {
    pub fn new(words: WordRepository, round_timeout: Option<Duration>) -> Self {
        Self {
            registry: Arc::new(GameRegistry::new(words, round_timeout)),
            pending_games: DashMap::new(),
        }
    }

    /// Create a new ephemeral game and return the game ID
    pub fn create_game(
        &self,
        player_name: String,
        tx: broadcast::Sender<ServerMessage>,
    ) -> String {
        let game_id = generate_unique_game_id(|id| self.pending_games.contains_key(id));
        let host = EphemeralPlayer::new(&player_name);
        let pending = PendingGame::new(game_id.clone(), host, tx);
        self.pending_games.insert(game_id.clone(), pending);
        info!(game_id, player_name, "Created pending game");
        game_id
    }

    /// Join an existing pending game. Returns None if game not found.
    pub fn join_game(
        &self,
        game_id: &str,
        player_name: String,
        tx: broadcast::Sender<ServerMessage>,
    ) -> Option<JoinedGame> {
        let (_, pending) = self.pending_games.remove(game_id)?;

        // Use display names as player IDs
        let host_name = pending.host.display_name.clone();
        // Ensure unique name - append discriminator if same as host
        let guest_name = if player_name == host_name {
            format!("{} (2)", player_name)
        } else {
            player_name
        };

        let session = GameSession::new(host_name.clone(), guest_name.clone());
        let game = ActiveGame::new(session, pending.host_tx.clone(), tx);

        self.registry.games.insert(game_id.to_string(), game);
        self.registry
            .player_games
            .insert(host_name.clone(), game_id.to_string());
        self.registry
            .player_games
            .insert(guest_name.clone(), game_id.to_string());

        info!(
            game_id,
            host = host_name,
            guest = guest_name,
            "Game joined"
        );

        Some(JoinedGame {
            game_id: game_id.to_string(),
            host_name,
            guest_name,
            host_tx: pending.host_tx,
        })
    }

    pub fn handle_disconnect(&self, user_id: &str) {
        info!(user_id, "Player disconnected");

        // If in a game, notify opponent and clean up
        let Some((_, game_id)) = self.registry.player_games.remove(user_id) else {
            return;
        };

        // Get opponent and broadcast before removing game
        let Some((_, game)) = self.registry.games.remove(&game_id) else {
            return;
        };

        let Some(opponent_id) = game.session.opponent_of(user_id) else {
            return;
        };

        // Notify opponent and clean up their mapping
        self.registry.player_games.remove(opponent_id);
        game.broadcast(ServerMessage::OpponentDisconnected);
    }
}
