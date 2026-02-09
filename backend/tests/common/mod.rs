use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
use yomitaisen::messages::{ClientMessage, ServerMessage};

pub type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub struct TestServer {
    base_url: String,
}

impl TestServer {
    pub fn ephemeral_url(&self) -> String {
        format!("{}/ws/ephemeral", self.base_url)
    }

    pub fn matchmaking_url(&self) -> String {
        format!("{}/ws/matchmaking", self.base_url)
    }

    pub fn http_url(&self, path: &str) -> String {
        format!(
            "http://{}{}",
            self.base_url.strip_prefix("ws://").unwrap(),
            path
        )
    }
}

pub async fn spawn_test_server() -> TestServer {
    spawn_test_server_with_timeout(None).await
}

pub async fn spawn_test_server_with_timeout(round_timeout: Option<Duration>) -> TestServer {
    let pool = sqlx::SqlitePool::connect(":memory:").await.unwrap();
    sqlx::migrate!().run(&pool).await.unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        let app = yomitaisen::app_with_config(pool, round_timeout);
        axum::serve(listener, app).await.unwrap();
    });

    TestServer {
        base_url: format!("ws://{}", addr),
    }
}

pub async fn connect_ephemeral(server: &TestServer) -> WsStream {
    let (ws, _) = connect_async(&server.ephemeral_url()).await.expect("Failed to connect");
    ws
}

pub async fn connect_matchmaking(server: &TestServer) -> WsStream {
    let (ws, _) = connect_async(&server.matchmaking_url()).await.expect("Failed to connect");
    ws
}

pub fn join_msg(user_id: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::Join {
        user_id: user_id.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

pub fn answer_msg(answer: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::Answer {
        answer: answer.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

pub fn create_game_msg(player_name: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::CreateGame {
        player_name: player_name.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

pub fn join_game_msg(game_id: &str, player_name: &str) -> Message {
    let json = serde_json::to_string(&ClientMessage::JoinGame {
        game_id: game_id.to_string(),
        player_name: player_name.to_string(),
    })
    .unwrap();
    Message::Text(json.into())
}

pub fn rematch_msg() -> Message {
    let json = serde_json::to_string(&ClientMessage::RequestRematch).unwrap();
    Message::Text(json.into())
}

pub async fn recv(ws: &mut WsStream) -> ServerMessage {
    let msg = ws.next().await.unwrap().unwrap();
    serde_json::from_str(msg.to_text().unwrap()).unwrap()
}

/// Look up correct reading from seed data
pub fn get_reading(kanji: &str) -> &'static str {
    match kanji {
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
    }
}
