mod common;

use common::*;
use futures_util::SinkExt;
use yomitaisen::messages::ServerMessage;

#[tokio::test]
async fn lobby_returns_empty_when_no_games() {
    let server = spawn_test_server().await;

    let response = reqwest::get(&server.http_url("/lobby")).await.unwrap();
    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["games"], serde_json::json!([]));
}

#[tokio::test]
async fn lobby_returns_pending_game() {
    let server = spawn_test_server().await;

    // Create a game via WebSocket
    let mut ws = connect_ephemeral(&server).await;
    ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut ws).await, ServerMessage::WaitingForOpponent);

    // Fetch lobby
    let response = reqwest::get(&server.http_url("/lobby")).await.unwrap();
    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.unwrap();
    let games = body["games"].as_array().unwrap();
    assert_eq!(games.len(), 1);
    assert_eq!(games[0]["game_id"], game_id);
    assert_eq!(games[0]["host_name"], "Alice");
}

#[tokio::test]
async fn lobby_excludes_game_after_join() {
    let server = spawn_test_server().await;

    // Create a game
    let mut host_ws = connect_ephemeral(&server).await;
    host_ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut host_ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut host_ws).await, ServerMessage::WaitingForOpponent);

    // Verify game is in lobby
    let response = reqwest::get(&server.http_url("/lobby")).await.unwrap();
    let body: serde_json::Value = response.json().await.unwrap();
    assert_eq!(body["games"].as_array().unwrap().len(), 1);

    // Join the game
    let mut guest_ws = connect_ephemeral(&server).await;
    guest_ws.send(join_game_msg(&game_id, "Bob")).await.unwrap();

    // Wait for game to start
    assert!(matches!(
        recv(&mut host_ws).await,
        ServerMessage::OpponentJoined { .. }
    ));
    assert!(matches!(
        recv(&mut host_ws).await,
        ServerMessage::GameStart { .. }
    ));

    // Verify game is no longer in lobby
    let response = reqwest::get(&server.http_url("/lobby")).await.unwrap();
    let body: serde_json::Value = response.json().await.unwrap();
    assert!(body["games"].as_array().unwrap().is_empty());
}
