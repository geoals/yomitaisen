use super::messages::{ClientMessage, ServerMessage};
use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::future::Future;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

/// Context for a WebSocket connection, tracking the connected user
pub struct ConnectionContext {
    pub user_id: Option<String>,
}

impl ConnectionContext {
    pub fn new() -> Self {
        Self { user_id: None }
    }
}

/// Trait for handling WebSocket messages and disconnections.
/// Implement this for each game mode (ephemeral, matchmaking).
pub trait ConnectionHandler: Send + Sync + 'static {
    /// Handle an incoming client message
    fn handle_message(
        self: Arc<Self>,
        msg: ClientMessage,
        tx: broadcast::Sender<ServerMessage>,
        ctx: &mut ConnectionContext,
    ) -> impl Future<Output = ()> + Send;

    /// Handle client disconnection
    fn handle_disconnect(&self, user_id: &str);

    /// Name for logging purposes
    fn name(&self) -> &'static str;
}

/// Run a WebSocket connection with the given handler.
/// This handles the boilerplate of splitting the socket, spawning send/receive tasks,
/// and coordinating shutdown.
pub async fn run_connection<H: ConnectionHandler>(socket: WebSocket, handler: Arc<H>) {
    info!("New {} WebSocket connection", handler.name());
    let (mut sender, receiver) = socket.split();
    let (tx, mut rx) = broadcast::channel::<ServerMessage>(16);

    // Task to send messages from the broadcast channel to the WebSocket
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            debug!(?msg, "Sending message to client");
            let json = serde_json::to_string(&msg).unwrap();
            if sender.send(Message::Text(json)).await.is_err() {
                break;
            }
        }
    });

    // Task to receive messages from the WebSocket and dispatch to handler
    let handler_clone = handler.clone();
    let recv_task = tokio::spawn(receive_loop(receiver, tx, handler_clone));

    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        result = recv_task => {
            if let Ok(Some(user_id)) = result {
                handler.handle_disconnect(&user_id);
            }
        },
    }

    info!("{} WebSocket connection closed", handler.name());
}

async fn receive_loop<H: ConnectionHandler>(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    tx: broadcast::Sender<ServerMessage>,
    handler: Arc<H>,
) -> Option<String> {
    let mut ctx = ConnectionContext::new();

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

        handler.clone().handle_message(client_msg, tx.clone(), &mut ctx).await;
    }

    ctx.user_id
}
