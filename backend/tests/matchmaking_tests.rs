mod common;

use common::*;
use futures_util::SinkExt;
use std::time::Duration;
use yomitaisen::messages::ServerMessage;

#[tokio::test]
async fn player_joins_and_receives_waiting() {
    let server = spawn_test_server().await;
    let mut ws = connect_matchmaking(&server).await;

    ws.send(join_msg("user-1")).await.unwrap();

    assert_eq!(recv(&mut ws).await, ServerMessage::Waiting);
}

#[tokio::test]
async fn two_players_join_and_game_starts() {
    let server = spawn_test_server().await;

    let mut ws1 = connect_matchmaking(&server).await;
    ws1.send(join_msg("user-1")).await.unwrap();
    assert_eq!(recv(&mut ws1).await, ServerMessage::Waiting);

    let mut ws2 = connect_matchmaking(&server).await;
    ws2.send(join_msg("user-2")).await.unwrap();

    assert!(matches!(recv(&mut ws1).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws2).await, ServerMessage::GameStart { .. }));
}

#[tokio::test]
async fn game_start_is_followed_by_round_start() {
    let server = spawn_test_server().await;

    let mut ws1 = connect_matchmaking(&server).await;
    let mut ws2 = connect_matchmaking(&server).await;

    ws1.send(join_msg("user-1")).await.unwrap();
    ws2.send(join_msg("user-2")).await.unwrap();

    // Player 1: Waiting, then GameStart, then RoundStart
    assert!(matches!(recv(&mut ws1).await, ServerMessage::Waiting));
    assert!(matches!(recv(&mut ws1).await, ServerMessage::GameStart { .. }));

    let round = recv(&mut ws1).await;
    assert!(matches!(round, ServerMessage::RoundStart { round: 1, .. }));

    // Player 2: GameStart, then RoundStart
    assert!(matches!(recv(&mut ws2).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws2).await, ServerMessage::RoundStart { round: 1, .. }));
}

#[tokio::test]
async fn correct_answer_wins_round() {
    let server = spawn_test_server().await;

    let mut ws1 = connect_matchmaking(&server).await;
    let mut ws2 = connect_matchmaking(&server).await;

    ws1.send(join_msg("user-1")).await.unwrap();
    ws2.send(join_msg("user-2")).await.unwrap();

    // Skip to RoundStart
    assert!(matches!(recv(&mut ws1).await, ServerMessage::Waiting));
    assert!(matches!(recv(&mut ws1).await, ServerMessage::GameStart { .. }));
    let ServerMessage::RoundStart { kanji, .. } = recv(&mut ws1).await else {
        panic!("Expected RoundStart");
    };

    assert!(matches!(recv(&mut ws2).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws2).await, ServerMessage::RoundStart { .. }));

    // Player 1 answers correctly
    ws1.send(answer_msg(get_reading(&kanji))).await.unwrap();

    // Both receive RoundResult
    let result1 = recv(&mut ws1).await;
    let result2 = recv(&mut ws2).await;

    assert!(matches!(
        result1,
        ServerMessage::RoundResult { winner: Some(ref w), .. } if w == "user-1"
    ));
    assert!(matches!(
        result2,
        ServerMessage::RoundResult { winner: Some(ref w), .. } if w == "user-1"
    ));
}

#[tokio::test]
async fn opponent_disconnect_notifies_remaining_player() {
    let server = spawn_test_server().await;

    let mut ws1 = connect_matchmaking(&server).await;
    let mut ws2 = connect_matchmaking(&server).await;

    ws1.send(join_msg("user-1")).await.unwrap();
    ws2.send(join_msg("user-2")).await.unwrap();

    // Skip to game started
    assert!(matches!(recv(&mut ws1).await, ServerMessage::Waiting));
    assert!(matches!(recv(&mut ws1).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws1).await, ServerMessage::RoundStart { .. }));

    assert!(matches!(recv(&mut ws2).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws2).await, ServerMessage::RoundStart { .. }));

    // Player 2 disconnects
    ws2.close(None).await.unwrap();

    // Player 1 should receive OpponentDisconnected
    let msg = recv(&mut ws1).await;
    assert!(matches!(msg, ServerMessage::OpponentDisconnected));
}

#[tokio::test]
async fn round_times_out_with_no_winner() {
    // Use a short timeout for testing
    let server = spawn_test_server_with_timeout(Some(Duration::from_millis(100))).await;

    let mut ws1 = connect_matchmaking(&server).await;
    let mut ws2 = connect_matchmaking(&server).await;

    ws1.send(join_msg("user-1")).await.unwrap();
    ws2.send(join_msg("user-2")).await.unwrap();

    // Skip to round start
    assert!(matches!(recv(&mut ws1).await, ServerMessage::Waiting));
    assert!(matches!(recv(&mut ws1).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws1).await, ServerMessage::RoundStart { .. }));

    assert!(matches!(recv(&mut ws2).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws2).await, ServerMessage::RoundStart { .. }));

    // Don't answer - wait for timeout (100ms)
    let result1 = recv(&mut ws1).await;
    let result2 = recv(&mut ws2).await;

    // Both should receive RoundResult with no winner
    assert!(matches!(
        result1,
        ServerMessage::RoundResult { winner: None, .. }
    ));
    assert!(matches!(
        result2,
        ServerMessage::RoundResult { winner: None, .. }
    ));

    // After timeout, next round should start (round 2, not round 1 again)
    let next1 = recv(&mut ws1).await;
    let next2 = recv(&mut ws2).await;

    assert!(matches!(next1, ServerMessage::RoundStart { round: 2, .. }));
    assert!(matches!(next2, ServerMessage::RoundStart { round: 2, .. }));
}
