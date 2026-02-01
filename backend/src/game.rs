use crate::messages::{ClientMessage, ServerMessage};
use crate::repository::{Word, WordRepository};
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct GameState {
    words: WordRepository,
    waiting_player: std::sync::Mutex<Option<(String, broadcast::Sender<ServerMessage>)>>,
    games: DashMap<String, Game>,
}

#[allow(dead_code)]
struct Game {
    player1: String,
    player2: String,
    tx: broadcast::Sender<ServerMessage>,
    current_round: u32,
    current_word: Option<Word>,
}

enum MatchResult {
    Waiting,
    Matched {
        opponent_id: String,
        opponent_tx: broadcast::Sender<ServerMessage>,
        game_id: String,
    },
}

impl GameState {
    pub fn new(words: WordRepository) -> Self {
        Self {
            words,
            waiting_player: std::sync::Mutex::new(None),
            games: DashMap::new(),
        }
    }

    fn try_match(&self, user_id: String, tx: broadcast::Sender<ServerMessage>) -> MatchResult {
        let mut waiting = self.waiting_player.lock().unwrap();

        let Some((opponent_id, opponent_tx)) = waiting.take() else {
            *waiting = Some((user_id, tx));
            return MatchResult::Waiting;
        };

        let game_id = uuid::Uuid::new_v4().to_string();
        let game = Game {
            player1: opponent_id.clone(),
            player2: user_id,
            tx: tx.clone(),
            current_round: 0,
            current_word: None,
        };
        self.games.insert(game_id.clone(), game);

        MatchResult::Matched {
            opponent_id,
            opponent_tx,
            game_id,
        }
    }
}

pub async fn handle_connection(socket: WebSocket, state: Arc<GameState>) {
    let (mut sender, receiver) = socket.split();
    let (tx, mut rx) = broadcast::channel::<ServerMessage>(16);

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(handle_incoming(receiver, tx, state));

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

async fn handle_incoming(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    tx: broadcast::Sender<ServerMessage>,
    state: Arc<GameState>,
) {
    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else { continue };

        let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) else {
            continue;
        };

        handle_message(client_msg, &tx, &state).await;
    }
}

async fn handle_message(
    msg: ClientMessage,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<GameState>,
) {
    match msg {
        ClientMessage::Join { user_id } => handle_join(user_id, tx, state).await,
        ClientMessage::Answer { answer: _ } => {
            // TODO: Handle answer
        }
    }
}

async fn handle_join(
    user_id: String,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<GameState>,
) {
    match state.try_match(user_id.clone(), tx.clone()) {
        MatchResult::Waiting => {
            let _ = tx.send(ServerMessage::Waiting);
        }
        MatchResult::Matched {
            opponent_id,
            opponent_tx,
            game_id,
        } => {
            let _ = tx.send(ServerMessage::GameStart {
                opponent: opponent_id,
            });
            let _ = opponent_tx.send(ServerMessage::GameStart { opponent: user_id });

            if let Some(word) = state.words.get_random().await {
                let round_msg = ServerMessage::RoundStart {
                    kanji: word.kanji.clone(),
                    round: 1,
                };
                let _ = tx.send(round_msg.clone());
                let _ = opponent_tx.send(round_msg);

                if let Some(mut game) = state.games.get_mut(&game_id) {
                    game.current_round = 1;
                    game.current_word = Some(word);
                }
            }
        }
    }
}
