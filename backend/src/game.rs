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
    player_games: DashMap<String, String>, // user_id -> game_id
}

#[allow(dead_code)]
struct Game {
    player1_id: String,
    player2_id: String,
    player1_tx: broadcast::Sender<ServerMessage>,
    player2_tx: broadcast::Sender<ServerMessage>,
    current_round: u32,
    current_word: Option<Word>,
}

impl Game {
    fn broadcast(&self, msg: ServerMessage) {
        let _ = self.player1_tx.send(msg.clone());
        let _ = self.player2_tx.send(msg);
    }
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
            player_games: DashMap::new(),
        }
    }

    fn try_match(
        &self,
        user_id: String,
        tx: broadcast::Sender<ServerMessage>,
    ) -> MatchResult {
        let mut waiting = self.waiting_player.lock().unwrap();

        let Some((opponent_id, opponent_tx)) = waiting.take() else {
            *waiting = Some((user_id, tx));
            return MatchResult::Waiting;
        };

        let game_id = uuid::Uuid::new_v4().to_string();
        let game = Game {
            player1_id: opponent_id.clone(),
            player2_id: user_id.clone(),
            player1_tx: opponent_tx.clone(),
            player2_tx: tx.clone(),
            current_round: 0,
            current_word: None,
        };

        self.games.insert(game_id.clone(), game);
        self.player_games.insert(opponent_id.clone(), game_id.clone());
        self.player_games.insert(user_id, game_id.clone());

        MatchResult::Matched {
            opponent_id,
            opponent_tx,
            game_id,
        }
    }

    fn check_answer(&self, user_id: &str, answer: &str) -> Option<ServerMessage> {
        let game_id = self.player_games.get(user_id)?;
        let game = self.games.get(&*game_id)?;

        let word = game.current_word.as_ref()?;
        if answer != word.reading {
            return None; // Wrong answer, ignore
        }

        Some(ServerMessage::RoundResult {
            winner: Some(user_id.to_string()),
            correct_reading: word.reading.clone(),
        })
    }

    fn broadcast_to_game(&self, user_id: &str, msg: ServerMessage) {
        let Some(game_id) = self.player_games.get(user_id) else { return };
        let Some(game) = self.games.get(&*game_id) else { return };
        game.broadcast(msg);
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

struct ConnectionContext {
    user_id: Option<String>,
}

async fn handle_incoming(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    tx: broadcast::Sender<ServerMessage>,
    state: Arc<GameState>,
) {
    let mut ctx = ConnectionContext { user_id: None };

    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else { continue };

        let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) else {
            continue;
        };

        handle_message(client_msg, &tx, &state, &mut ctx).await;
    }
}

async fn handle_message(
    msg: ClientMessage,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<GameState>,
    ctx: &mut ConnectionContext,
) {
    match msg {
        ClientMessage::Join { user_id } => {
            ctx.user_id = Some(user_id.clone());
            handle_join(user_id, tx, state).await;
        }
        ClientMessage::Answer { answer } => {
            let Some(user_id) = &ctx.user_id else { return };
            handle_answer(user_id, &answer, state);
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

fn handle_answer(user_id: &str, answer: &str, state: &Arc<GameState>) {
    let Some(result) = state.check_answer(user_id, answer) else {
        return; // Wrong answer or no game
    };
    state.broadcast_to_game(user_id, result);
}
