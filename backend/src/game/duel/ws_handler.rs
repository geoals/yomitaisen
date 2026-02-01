use super::game_id::generate_unique_game_id;
use super::matchmaking::{Lobby, MatchOutcome};
use super::messages::{ClientMessage, ServerMessage};
use super::pending_game::PendingGame;
use super::player::EphemeralPlayer;
use super::session::GameSession;
use crate::game::core::WordRepository;
use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

const DEFAULT_ROUND_TIMEOUT: Duration = Duration::from_secs(15);

pub struct DuelState {
    words: WordRepository,
    lobby: Lobby,
    player_channels: DashMap<String, broadcast::Sender<ServerMessage>>,
    games: DashMap<String, ActiveGame>,
    player_games: DashMap<String, String>, // user_id -> game_id
    pending_games: DashMap<String, PendingGame>, // game_id -> pending game
    round_timeout: Duration,
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

/// Result of a correct answer submission
struct AnswerResult {
    round_result: ServerMessage,
    game_winner: Option<String>,
    round_number: u32,
}

/// Result of joining an ephemeral game
struct JoinedGame {
    game_id: String,
    host_name: String,
    guest_name: String,
    host_tx: broadcast::Sender<ServerMessage>,
}

impl DuelState {
    pub fn new(words: WordRepository) -> Self {
        Self {
            words,
            lobby: Lobby::new(),
            player_channels: DashMap::new(),
            games: DashMap::new(),
            player_games: DashMap::new(),
            pending_games: DashMap::new(),
            round_timeout: DEFAULT_ROUND_TIMEOUT,
        }
    }

    pub fn with_round_timeout(mut self, timeout: Duration) -> Self {
        self.round_timeout = timeout;
        self
    }

    /// Create a new ephemeral game and return the game ID
    fn create_game(
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
    fn join_game(
        &self,
        game_id: &str,
        player_name: String,
        tx: broadcast::Sender<ServerMessage>,
    ) -> Option<JoinedGame> {
        let (_, pending) = self.pending_games.remove(game_id)?;

        // Use display names as player IDs (consistent with authenticated flow)
        let host_name = pending.host.display_name.clone();
        // Ensure unique name - append discriminator if same as host
        let guest_name = if player_name == host_name {
            format!("{} (2)", player_name)
        } else {
            player_name
        };

        let session = GameSession::new(host_name.clone(), guest_name.clone());
        let game = ActiveGame {
            session,
            player1_tx: pending.host_tx.clone(),
            player2_tx: tx,
        };

        self.games.insert(game_id.to_string(), game);
        self.player_games
            .insert(host_name.clone(), game_id.to_string());
        self.player_games
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

    fn register_player(&self, user_id: &str, tx: broadcast::Sender<ServerMessage>) {
        debug!(user_id, "Registering player channel");
        self.player_channels.insert(user_id.to_string(), tx);
    }

    fn try_join(&self, user_id: String, tx: broadcast::Sender<ServerMessage>) -> JoinResult {
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

    /// Returns result if answer is correct, None if wrong
    fn submit_answer(&self, user_id: &str, answer: &str) -> Option<AnswerResult> {
        let game_id = self.player_games.get(user_id)?;
        let mut game = self.games.get_mut(&*game_id)?;

        debug!(user_id, answer, "Player submitting answer");

        let outcome = game.session.submit_answer(user_id, answer)?;

        // Record the win
        if let Some(winner) = &outcome.winner {
            game.session.record_win(winner);
        }

        let scores = game.session.scores();
        let game_winner = game.session.game_winner().map(|s| s.to_string());

        info!(user_id, round_winner = ?outcome.winner, scores = ?scores, game_winner = ?game_winner, "Round ended");

        let round_number = scores.0 + scores.1;

        Some(AnswerResult {
            round_result: ServerMessage::RoundResult {
                winner: outcome.winner,
                correct_reading: outcome.correct_reading,
            },
            game_winner,
            round_number,
        })
    }

    fn broadcast_to_game(&self, user_id: &str, msg: ServerMessage) {
        let Some(game_id) = self.player_games.get(user_id) else {
            return;
        };
        let Some(game) = self.games.get(&*game_id) else {
            return;
        };
        debug!(?game_id, "Broadcasting to game");
        game.broadcast(msg);
    }

    fn handle_disconnect(&self, user_id: &str) {
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
}

pub async fn handle_connection(socket: WebSocket, state: Arc<DuelState>) {
    info!("New WebSocket connection");
    let (mut sender, receiver) = socket.split();
    let (tx, mut rx) = broadcast::channel::<ServerMessage>(16);

    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            debug!(?msg, "Sending message to client");
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let recv_task = tokio::spawn(handle_incoming(receiver, tx, state_clone));

    tokio::select! {
        _ = send_task => {},
        result = recv_task => {
            if let Ok(Some(user_id)) = result {
                state.handle_disconnect(&user_id);
            }
        },
    }

    info!("WebSocket connection closed");
}

struct ConnectionContext {
    user_id: Option<String>,
}

async fn handle_incoming(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    tx: broadcast::Sender<ServerMessage>,
    state: Arc<DuelState>,
) -> Option<String> {
    let mut ctx = ConnectionContext { user_id: None };

    while let Some(Ok(msg)) = receiver.next().await {
        let Message::Text(text) = msg else {
            debug!("Received non-text message, ignoring");
            continue;
        };

        debug!(raw = %text, "Received message");

        let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) else {
            warn!(raw = %text, "Failed to parse client message");
            continue;
        };

        handle_message(client_msg, &tx, &state, &mut ctx).await;
    }

    ctx.user_id
}

async fn handle_message(
    msg: ClientMessage,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<DuelState>,
    ctx: &mut ConnectionContext,
) {
    match msg {
        ClientMessage::Join { user_id } => {
            info!(user_id, "Player joining");
            ctx.user_id = Some(user_id.clone());
            handle_join(user_id, tx, state).await;
        }
        ClientMessage::CreateGame { player_name } => {
            ctx.user_id = Some(player_name.clone());
            let game_id = state.create_game(player_name, tx.clone());
            let _ = tx.send(ServerMessage::GameCreated { game_id });
            let _ = tx.send(ServerMessage::WaitingForOpponent);
        }
        ClientMessage::JoinGame { game_id, player_name } => {
            let Some(joined) = state.join_game(&game_id, player_name, tx.clone()) else {
                let _ = tx.send(ServerMessage::GameNotFound);
                return;
            };
            // Set user_id to the (possibly modified) guest name
            ctx.user_id = Some(joined.guest_name.clone());

            // Notify host that opponent joined
            let _ = joined.host_tx.send(ServerMessage::OpponentJoined {
                opponent_name: joined.guest_name.clone(),
            });

            // Send GameStart to both players
            let _ = joined.host_tx.send(ServerMessage::GameStart {
                opponent: joined.guest_name,
            });
            let _ = tx.send(ServerMessage::GameStart {
                opponent: joined.host_name,
            });

            // Start round 1
            if let Some(word) = state.words.get_random().await {
                let round_msg = ServerMessage::RoundStart {
                    kanji: word.kanji.clone(),
                    round: 1,
                };
                let _ = joined.host_tx.send(round_msg.clone());
                let _ = tx.send(round_msg);

                if let Some(mut game) = state.games.get_mut(&joined.game_id) {
                    game.session.start_round(1, word);
                }
                spawn_round_timeout(state.clone(), joined.game_id, 1);
            }
        }
        ClientMessage::Answer { answer } => {
            let Some(user_id) = &ctx.user_id else {
                warn!("Received answer from unknown user");
                return;
            };
            debug!(user_id, answer, "Player answered");
            handle_answer(user_id, &answer, state, tx).await;
        }
    }
}

async fn handle_join(
    user_id: String,
    tx: &broadcast::Sender<ServerMessage>,
    state: &Arc<DuelState>,
) {
    match state.try_join(user_id.clone(), tx.clone()) {
        JoinResult::Waiting => {
            debug!(user_id, "Sending Waiting message");
            let _ = tx.send(ServerMessage::Waiting);
        }
        JoinResult::Matched {
            opponent_id,
            opponent_tx,
            game_id,
        } => {
            info!(game_id, user_id, opponent_id, "Game starting");

            let _ = tx.send(ServerMessage::GameStart {
                opponent: opponent_id.clone(),
            });
            let _ = opponent_tx.send(ServerMessage::GameStart {
                opponent: user_id.clone(),
            });

            if let Some(word) = state.words.get_random().await {
                info!(
                    game_id,
                    kanji = word.kanji,
                    reading = word.reading,
                    "Round 1 starting"
                );

                let round_msg = ServerMessage::RoundStart {
                    kanji: word.kanji.clone(),
                    round: 1,
                };
                let _ = tx.send(round_msg.clone());
                let _ = opponent_tx.send(round_msg);

                if let Some(mut game) = state.games.get_mut(&game_id) {
                    game.session.start_round(1, word);
                }
                spawn_round_timeout(state.clone(), game_id, 1);
            }
        }
    }
}

async fn handle_answer(
    user_id: &str,
    answer: &str,
    state: &Arc<DuelState>,
    tx: &broadcast::Sender<ServerMessage>,
) {
    let Some(result) = state.submit_answer(user_id, answer) else {
        debug!(user_id, answer, "Wrong answer");
        let _ = tx.send(ServerMessage::WrongAnswer);
        return;
    };

    // Broadcast round result to both players
    state.broadcast_to_game(user_id, result.round_result);

    // Get game_id for continue_or_end_game
    let Some(game_id) = state.player_games.get(user_id).map(|r| r.clone()) else {
        return;
    };

    continue_or_end_game(state, &game_id, result.game_winner, result.round_number).await;
}

fn spawn_round_timeout(state: Arc<DuelState>, game_id: String, round_number: u32) {
    let timeout = state.round_timeout;
    tokio::spawn(async move {
        tokio::time::sleep(timeout).await;
        handle_round_timeout(state, game_id, round_number).await;
    });
}

async fn handle_round_timeout(state: Arc<DuelState>, game_id: String, round_number: u32) {
    // Check if the round is still active and timeout it
    let timeout_result = {
        let Some(mut game) = state.games.get_mut(&game_id) else {
            return;
        };

        // Only timeout if we're still on the same round
        if game.session.current_round_number() != Some(round_number) {
            return;
        }

        let Some(outcome) = game.session.timeout_round() else {
            return;
        };

        info!(game_id, round_number, "Round timed out");

        let game_winner = game.session.game_winner().map(|s| s.to_string());

        game.broadcast(ServerMessage::RoundResult {
            winner: outcome.winner,
            correct_reading: outcome.correct_reading,
        });

        Some(game_winner)
    };

    let Some(game_winner) = timeout_result else {
        return;
    };

    // Use the round_number parameter (the round that timed out), not scores
    continue_or_end_game(&state, &game_id, game_winner, round_number).await;
}

async fn continue_or_end_game(
    state: &Arc<DuelState>,
    game_id: &str,
    game_winner: Option<String>,
    round_number: u32,
) {
    match game_winner {
        Some(winner) => {
            info!(winner, "Game ended");
            if let Some(game) = state.games.get(game_id) {
                game.broadcast(ServerMessage::GameEnd { winner });
            }
        }
        None => {
            let Some(word) = state.words.get_random().await else {
                return;
            };

            let next_round = round_number + 1;
            info!(round = next_round, kanji = word.kanji, "Starting next round");

            if let Some(mut game) = state.games.get_mut(game_id) {
                game.broadcast(ServerMessage::RoundStart {
                    kanji: word.kanji.clone(),
                    round: next_round,
                });
                game.session.start_round(next_round, word);
            }

            spawn_round_timeout(state.clone(), game_id.to_string(), next_round);
        }
    }
}
