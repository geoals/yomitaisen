use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use yomitaisen::messages::{ClientMessage, ServerMessage};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn spawn_test_server() -> String {
    spawn_test_server_with_timeout(None).await
}

async fn spawn_test_server_with_timeout(round_timeout: Option<Duration>) -> String {
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let app = yomitaisen::app_with_config(pool, round_timeout);
        axum::serve(listener, app).await.unwrap();
    });

    format!("ws://{}/ws", addr)
}

fn join_msg(user_id: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::Join {
        user_id: user_id.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

fn answer_msg(answer: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::Answer {
        answer: answer.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

fn create_game_msg(player_name: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::CreateGame {
        player_name: player_name.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

fn join_game_msg(game_id: &str, player_name: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::JoinGame {
        game_id: game_id.to_string(),
        player_name: player_name.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

async fn recv(ws: &mut WsStream) -> ServerMessage {
    let msg = ws.next().await.unwrap().unwrap();
    serde_json::from_str(msg.to_text().unwrap()).unwrap()
}

#[tokio::test]
async fn player_joins_and_receives_waiting() {
    let url = spawn_test_server().await;
    let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");

    ws.send(join_msg("user-1")).await.unwrap();

    assert_eq!(recv(&mut ws).await, ServerMessage::Waiting);
}

#[tokio::test]
async fn two_players_join_and_game_starts() {
    let url = spawn_test_server().await;

    let (mut ws1, _) = connect_async(&url).await.expect("Failed to connect");
    ws1.send(join_msg("user-1")).await.unwrap();
    assert_eq!(recv(&mut ws1).await, ServerMessage::Waiting);

    let (mut ws2, _) = connect_async(&url).await.expect("Failed to connect");
    ws2.send(join_msg("user-2")).await.unwrap();

    assert!(matches!(recv(&mut ws1).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut ws2).await, ServerMessage::GameStart { .. }));
}

#[tokio::test]
async fn game_start_is_followed_by_round_start() {
    let url = spawn_test_server().await;

    let (mut ws1, _) = connect_async(&url).await.unwrap();
    let (mut ws2, _) = connect_async(&url).await.unwrap();

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
    let url = spawn_test_server().await;

    let (mut ws1, _) = connect_async(&url).await.unwrap();
    let (mut ws2, _) = connect_async(&url).await.unwrap();

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

    // Look up correct reading from seed data
    let correct_reading = match kanji.as_str() {
        "日本" => "にほん",
        "学校" => "がっこう",
        "電話" => "でんわ",
        "先生" => "せんせい",
        "時間" => "じかん",
        "食べる" => "たべる",
        "飲む" => "のむ",
        "書く" => "かく",
        "読む" => "よむ",
        "聞く" => "きく",
        _ => panic!("Unknown kanji: {}", kanji),
    };

    // Player 1 answers correctly
    ws1.send(answer_msg(correct_reading)).await.unwrap();

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
    let url = spawn_test_server().await;

    let (mut ws1, _) = connect_async(&url).await.unwrap();
    let (mut ws2, _) = connect_async(&url).await.unwrap();

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
    let url = spawn_test_server_with_timeout(Some(Duration::from_millis(100))).await;

    let (mut ws1, _) = connect_async(&url).await.unwrap();
    let (mut ws2, _) = connect_async(&url).await.unwrap();

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

    assert!(matches!(
        next1,
        ServerMessage::RoundStart { round: 2, .. }
    ));
    assert!(matches!(
        next2,
        ServerMessage::RoundStart { round: 2, .. }
    ));
}

// ============ Ephemeral game tests ============

#[tokio::test]
async fn create_game_returns_game_id_and_waits() {
    let url = spawn_test_server().await;
    let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");

    ws.send(create_game_msg("Alice")).await.unwrap();

    // Should receive GameCreated with a game_id
    let msg = recv(&mut ws).await;
    let game_id = match msg {
        ServerMessage::GameCreated { game_id } => {
            assert_eq!(game_id.len(), 6); // Short code
            game_id
        }
        other => panic!("Expected GameCreated, got {:?}", other),
    };

    // Should then receive WaitingForOpponent
    assert_eq!(recv(&mut ws).await, ServerMessage::WaitingForOpponent);

    // Game ID should be reusable for joining
    assert!(!game_id.is_empty());
}

#[tokio::test]
async fn join_game_starts_match() {
    let url = spawn_test_server().await;

    // Host creates game
    let (mut host_ws, _) = connect_async(&url).await.unwrap();
    host_ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut host_ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut host_ws).await, ServerMessage::WaitingForOpponent);

    // Guest joins with game ID
    let (mut guest_ws, _) = connect_async(&url).await.unwrap();
    guest_ws.send(join_game_msg(&game_id, "Bob")).await.unwrap();

    // Host receives OpponentJoined then GameStart
    assert!(matches!(
        recv(&mut host_ws).await,
        ServerMessage::OpponentJoined { opponent_name } if opponent_name == "Bob"
    ));
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::GameStart { .. }));

    // Guest receives GameStart
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::GameStart { .. }));

    // Both receive RoundStart
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::RoundStart { round: 1, .. }));
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::RoundStart { round: 1, .. }));
}

#[tokio::test]
async fn join_nonexistent_game_returns_not_found() {
    let url = spawn_test_server().await;
    let (mut ws, _) = connect_async(&url).await.unwrap();

    ws.send(join_game_msg("xyz999", "Bob")).await.unwrap();

    assert_eq!(recv(&mut ws).await, ServerMessage::GameNotFound);
}
