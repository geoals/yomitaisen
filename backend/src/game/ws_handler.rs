use super::matchmaking::{Lobby, MatchOutcome};
use super::session::GameSession;
use crate::messages::{ClientMessage, ServerMessage};
use crate::repository::WordRepository;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::sync::broadcast;

pub struct GameState {
    words: WordRepository,
    lobby: Lobby,
    player_channels: DashMap<String, broadcast::Sender<ServerMessage>>,
    games: DashMap<String, ActiveGame>,
    player_games: DashMap<String, String>, // user_id -> game_id
}

/// An active game: combines pure game logic with transport channels
struct ActiveGame {
    session: GameSession,
    player1_tx: broadcast::Sender<ServerMessage>,
    player2_tx: broadcast::Sender<ServerMessage>,
}

impl ActiveGame {
    fn broadcast(&self, msg: ServerMessage) {
        let _ = self.player1_tx.send(msg.clone());
        let _ = self.player2_tx.send(msg);
    }
}

/// Internal result after matching + channel lookup
enum JoinResult {
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
            lobby: Lobby::new(),
            player_channels: DashMap::new(),
            games: DashMap::new(),
            player_games: DashMap::new(),
        }
    }

    fn register_player(&self, user_id: &str, tx: broadcast::Sender<ServerMessage>) {
        self.player_channels.insert(user_id.to_string(), tx);
    }

    fn try_join(&self, user_id: String, tx: broadcast::Sender<ServerMessage>) -> JoinResult {
        // Register this player's channel
        self.register_player(&user_id, tx.clone());

        // Try matchmaking
        match self.lobby.try_match(user_id.clone()) {
            MatchOutcome::Waiting => JoinResult::Waiting,
            MatchOutcome::Matched { opponent_id } => {
                // Look up opponent's channel
                let opponent_tx = self
                    .player_channels
                    .get(&opponent_id)
                    .map(|r| r.clone())
                    .expect("opponent should have registered channel");

                // Create game
                let game_id = uuid::Uuid::new_v4().to_string();
                let session = GameSession::new(opponent_id.clone(), user_id.clone());
                let game = ActiveGame {
                    session,
                    player1_tx: opponent_tx.clone(),
                    player2_tx: tx,
                };

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

    fn submit_answer(&self, user_id: &str, answer: &str) -> Option<ServerMessage> {
        let game_id = self.player_games.get(user_id)?;
        let mut game = self.games.get_mut(&*game_id)?;

        let outcome = game.session.submit_answer(user_id, answer)?;

        Some(ServerMessage::RoundResult {
            winner: outcome.winner,
            correct_reading: outcome.correct_reading,
        })
    }

    fn broadcast_to_game(&self, user_id: &str, msg: ServerMessage) {
        let Some(game_id) = self.player_games.get(user_id) else {
            return;
        };
        let Some(game) = self.games.get(&*game_id) else {
            return;
        };
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
    match state.try_join(user_id.clone(), tx.clone()) {
        JoinResult::Waiting => {
            let _ = tx.send(ServerMessage::Waiting);
        }
        JoinResult::Matched {
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
                    game.session.start_round(1, word);
                }
            }
        }
    }
}

fn handle_answer(user_id: &str, answer: &str, state: &Arc<GameState>) {
    let Some(result) = state.submit_answer(user_id, answer) else {
        return;
    };
    state.broadcast_to_game(user_id, result);
}
