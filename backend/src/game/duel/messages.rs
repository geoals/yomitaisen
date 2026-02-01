use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join { user_id: String },
    Answer { answer: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Waiting,
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
    OpponentDisconnected,
    GameEnd {
        winner: String,
    },
    #[allow(dead_code)]
    Error {
        message: String,
    },
}
