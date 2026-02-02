use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    // Authenticated matchmaking
    Join {
        user_id: String,
    },

    // Ephemeral create/join
    CreateGame {
        player_name: String,
    },
    JoinGame {
        game_id: String,
        player_name: String,
    },

    // Shared
    Answer {
        answer: String,
    },
    Skip,
    RequestRematch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    // Authenticated matchmaking
    Waiting,

    // Ephemeral create/join
    GameCreated {
        game_id: String,
    },
    WaitingForOpponent,
    OpponentJoined {
        opponent_name: String,
    },
    GameFull,
    GameNotFound,

    // Shared game flow
    GameStart {
        opponent: String,
    },
    RoundStart {
        kanji: String,
        round: u32,
    },
    RoundResult {
        winner: Option<String>,
        correct_reading: String,
    },
    WrongAnswer,
    SkipWaiting,
    RematchWaiting,
    OpponentDisconnected,
    GameEnd {
        winner: Option<String>,
    },
    #[allow(dead_code)]
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_create_game() {
        let json = r#"{"type": "create_game", "player_name": "Alice"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert_eq!(
            msg,
            ClientMessage::CreateGame {
                player_name: "Alice".to_string()
            }
        );
    }

    #[test]
    fn deserialize_join_game() {
        let json = r#"{"type": "join_game", "game_id": "abc123", "player_name": "Bob"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert_eq!(
            msg,
            ClientMessage::JoinGame {
                game_id: "abc123".to_string(),
                player_name: "Bob".to_string()
            }
        );
    }

    #[test]
    fn serialize_game_created() {
        let msg = ServerMessage::GameCreated {
            game_id: "abc123".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"game_created""#));
        assert!(json.contains(r#""game_id":"abc123""#));
    }

    #[test]
    fn serialize_waiting_for_opponent() {
        let msg = ServerMessage::WaitingForOpponent;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"waiting_for_opponent"}"#);
    }

    #[test]
    fn serialize_game_full() {
        let msg = ServerMessage::GameFull;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"game_full"}"#);
    }

    #[test]
    fn serialize_game_not_found() {
        let msg = ServerMessage::GameNotFound;
        let json = serde_json::to_string(&msg).unwrap();
        assert_eq!(json, r#"{"type":"game_not_found"}"#);
    }
}
