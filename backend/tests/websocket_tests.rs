use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Join { user_id: String },
}

#[derive(Debug, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessage {
    Waiting,
    GameStart { opponent: String },
    RoundStart { kanji: String, round: u32 },
}

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn spawn_test_server() -> String {
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let app = yomitaisen::app(pool);
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
