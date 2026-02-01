use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join {
        user_id: String,
    },
    #[allow(dead_code)]
    Answer {
        answer: String,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(dead_code)]
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
    GameEnd {
        winner: String,
    },
    Error {
        message: String,
    },
}
