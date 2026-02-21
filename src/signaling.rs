use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::IntoResponse,
};
use crate::AppState;

/// This handler handles upgrading an HTTP connection to a WebSocket connection.
/// WebSockets allow for two-way, real-time communication between the browser and the server.
pub async fn ws_input_handler(
    ws:            WebSocketUpgrade,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // If the browser wants to upgrade, we call 'handle_ws'.
    ws.on_upgrade(handle_ws)
}

/// Once the WebSocket is established, this function runs.
async fn handle_ws(mut socket: WebSocket) {
    // This is a simple loop that waits for messages from the browser.
    // Currently, we don't use this much because most communication happens via WebRTC data channels,
    // but it's here for future features like multi-monitor support or advanced signaling.
    while let Some(Ok(msg)) = socket.recv().await {
        tracing::debug!("WS: {:?}", msg);
    }
}