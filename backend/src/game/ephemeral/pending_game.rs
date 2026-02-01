use tokio::sync::broadcast;

use crate::game::core::messages::ServerMessage;
use super::player::EphemeralPlayer;

pub struct PendingGame {
    pub game_id: String,
    pub host: EphemeralPlayer,
    pub host_tx: broadcast::Sender<ServerMessage>,
    pub created_at: std::time::Instant,
}

impl PendingGame {
    pub fn new(
        game_id: impl Into<String>,
        host: EphemeralPlayer,
        host_tx: broadcast::Sender<ServerMessage>,
    ) -> Self {
        Self {
            game_id: game_id.into(),
            host,
            host_tx,
            created_at: std::time::Instant::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_game_stores_host_info() {
        let (tx, _rx) = broadcast::channel(16);
        let host = EphemeralPlayer::new("Alice");
        let pending = PendingGame::new("abc123", host, tx);

        assert_eq!(pending.game_id, "abc123");
        assert_eq!(pending.host.display_name, "Alice");
    }

    #[test]
    fn pending_game_tracks_creation_time() {
        let (tx, _rx) = broadcast::channel(16);
        let host = EphemeralPlayer::new("Alice");
        let before = std::time::Instant::now();
        let pending = PendingGame::new("abc123", host, tx);
        let after = std::time::Instant::now();

        assert!(pending.created_at >= before);
        assert!(pending.created_at <= after);
    }
}
