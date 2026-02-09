use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use yomitaisen::messages::{ClientMessage, ServerMessage};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

struct TestServer {
    base_url: String,
}

impl TestServer {
    fn ws_url(&self) -> String {
        format!("{}/ws/ephemeral", self.base_url)
    }

    fn http_url(&self, path: &str) -> String {
        format!(
            "http://{}{}",
            self.base_url.strip_prefix("ws://").unwrap(),
            path
        )
    }
}

async fn spawn_test_server() -> TestServer {
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let app = yomitaisen::app(pool);
        axum::serve(listener, app).await.unwrap();
    });

    TestServer {
        base_url: format!("ws://{}", addr),
    }
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

// ============ Lobby REST API tests ============

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
    let (mut ws, _) = connect_async(&server.ws_url()).await.unwrap();
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
    let (mut host_ws, _) = connect_async(&server.ws_url()).await.unwrap();
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
    let (mut guest_ws, _) = connect_async(&server.ws_url()).await.unwrap();
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
