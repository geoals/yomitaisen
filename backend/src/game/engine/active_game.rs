use crate::game::core::WordRepository;
use crate::game::core::messages::ServerMessage;
use crate::game::core::session::GameSession;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tracing::info;

pub const DEFAULT_ROUND_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_ROUNDS: u32 = 30;

/// An active game: combines pure game logic with transport channels
pub struct ActiveGame {
    pub session: GameSession,
    pub player1_tx: broadcast::Sender<ServerMessage>,
    pub player2_tx: broadcast::Sender<ServerMessage>,
}

impl ActiveGame {
    pub fn new(
        session: GameSession,
        player1_tx: broadcast::Sender<ServerMessage>,
        player2_tx: broadcast::Sender<ServerMessage>,
    ) -> Self {
        Self {
            session,
            player1_tx,
            player2_tx,
        }
    }

    pub fn broadcast(&self, msg: ServerMessage) {
        let _ = self.player1_tx.send(msg.clone());
        let _ = self.player2_tx.send(msg);
    }
}

/// Result of a correct answer submission
pub struct AnswerResult {
    pub round_result: ServerMessage,
    pub game_winner: Option<String>,
    pub round_number: u32,
}

/// Trait for cleanup functions that can be used across async boundaries
pub trait CleanupGame: Send + Sync + 'static {
    fn cleanup(&self, game_id: &str);
}

/// Implementation for closures
impl<F> CleanupGame for F
where
    F: Fn(&str) + Send + Sync + 'static,
{
    fn cleanup(&self, game_id: &str) {
        self(game_id)
    }
}

/// Spawns a timeout task for the current round
pub fn spawn_round_timeout(
    timeout: Duration,
    games: Arc<DashMap<String, ActiveGame>>,
    words: WordRepository,
    game_id: String,
    round_number: u32,
    player_games: Arc<DashMap<String, String>>,
    cleanup_game: Arc<dyn CleanupGame>,
) {
    tokio::spawn(async move {
        tokio::time::sleep(timeout).await;
        handle_round_timeout(
            games,
            words,
            timeout,
            game_id,
            round_number,
            player_games,
            cleanup_game,
        )
        .await;
    });
}

async fn handle_round_timeout(
    games: Arc<DashMap<String, ActiveGame>>,
    words: WordRepository,
    timeout: Duration,
    game_id: String,
    round_number: u32,
    player_games: Arc<DashMap<String, String>>,
    cleanup_game: Arc<dyn CleanupGame>,
) {
    // Check if the round is still active and timeout it
    let timeout_result = {
        let Some(mut game) = games.get_mut(&game_id) else {
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

    continue_or_end_game(
        &games,
        &words,
        timeout,
        &game_id,
        game_winner,
        round_number,
        &player_games,
        cleanup_game,
    )
    .await;
}

pub async fn continue_or_end_game(
    games: &Arc<DashMap<String, ActiveGame>>,
    words: &WordRepository,
    timeout: Duration,
    game_id: &str,
    game_winner: Option<String>,
    round_number: u32,
    player_games: &Arc<DashMap<String, String>>,
    cleanup_game: Arc<dyn CleanupGame>,
) {
    // Check for winner or max rounds reached
    // Note: We don't cleanup here to allow rematch. Cleanup happens on disconnect.
    if let Some(winner) = game_winner {
        info!(winner, "Game ended - winner by score");
        if let Some(game) = games.get(game_id) {
            game.broadcast(ServerMessage::GameEnd {
                winner: Some(winner),
            });
        }
        return;
    }

    if round_number >= MAX_ROUNDS {
        info!(round_number, "Game ended - max rounds reached");
        if let Some(game) = games.get(game_id) {
            let (p1_score, p2_score) = game.session.scores();
            let winner = match p1_score.cmp(&p2_score) {
                std::cmp::Ordering::Greater => Some(game.session.player1.clone()),
                std::cmp::Ordering::Less => Some(game.session.player2.clone()),
                std::cmp::Ordering::Equal => None, // Draw
            };
            game.broadcast(ServerMessage::GameEnd { winner });
        }
        return;
    }

    // Start next round
    let Some(word) = words.get_random().await else {
        return;
    };

    let next_round = round_number + 1;
    let readings = words.get_readings_for_kanji(&word.kanji).await;
    info!(
        round = next_round,
        kanji = word.kanji,
        "Starting next round"
    );

    if let Some(mut game) = games.get_mut(game_id) {
        game.broadcast(ServerMessage::RoundStart {
            kanji: word.kanji.clone(),
            round: next_round,
            readings,
        });
        game.session.start_round(next_round, word);
    }

    spawn_round_timeout(
        timeout,
        games.clone(),
        words.clone(),
        game_id.to_string(),
        next_round,
        player_games.clone(),
        cleanup_game,
    );
}
