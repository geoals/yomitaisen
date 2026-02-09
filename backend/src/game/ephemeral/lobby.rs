use serde::Serialize;

/// A game visible in the lobby (pending, waiting for opponent)
#[derive(Debug, Serialize)]
pub struct LobbyGame {
    pub game_id: String,
    pub host_name: String,
    /// Seconds since the game was created
    pub created_at_secs: u64,
}

/// List of games available to join
#[derive(Debug, Serialize)]
pub struct LobbyList {
    pub games: Vec<LobbyGame>,
}
