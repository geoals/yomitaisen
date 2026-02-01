use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio_tungstenite::{connect_async, tungstenite::Message};

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
}

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

#[tokio::test]
async fn player_joins_and_receives_waiting() {
    let url = spawn_test_server().await;

    let (mut ws, _) = connect_async(&url).await.expect("Failed to connect");

    // Send Join message
    let join_msg = serde_json::to_string(&ClientMessage::Join {
        user_id: "user-1".to_string(),
    })
    .unwrap();
    ws.send(Message::Text(join_msg.into())).await.unwrap();

    // Expect Waiting response
    let msg = ws.next().await.unwrap().unwrap();
    let response: ServerMessage = serde_json::from_str(msg.to_text().unwrap()).unwrap();

    assert_eq!(response, ServerMessage::Waiting);
}

#[tokio::test]
async fn two_players_join_and_game_starts() {
    let url = spawn_test_server().await;

    // Player 1 connects and joins
    let (mut ws1, _) = connect_async(&url).await.expect("Failed to connect");
    let join_msg = serde_json::to_string(&ClientMessage::Join {
        user_id: "user-1".to_string(),
    })
    .unwrap();
    ws1.send(Message::Text(join_msg.into())).await.unwrap();

    // Player 1 receives Waiting
    let msg = ws1.next().await.unwrap().unwrap();
    let response: ServerMessage = serde_json::from_str(msg.to_text().unwrap()).unwrap();
    assert_eq!(response, ServerMessage::Waiting);

    // Player 2 connects and joins
    let (mut ws2, _) = connect_async(&url).await.expect("Failed to connect");
    let join_msg = serde_json::to_string(&ClientMessage::Join {
        user_id: "user-2".to_string(),
    })
    .unwrap();
    ws2.send(Message::Text(join_msg.into())).await.unwrap();

    // Both players receive GameStart
    let msg1 = ws1.next().await.unwrap().unwrap();
    let response1: ServerMessage = serde_json::from_str(msg1.to_text().unwrap()).unwrap();

    let msg2 = ws2.next().await.unwrap().unwrap();
    let response2: ServerMessage = serde_json::from_str(msg2.to_text().unwrap()).unwrap();

    assert!(matches!(response1, ServerMessage::GameStart { .. }));
    assert!(matches!(response2, ServerMessage::GameStart { .. }));
}
