use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::IntoResponse,
};
use crate::AppState;

pub async fn ws_input_handler(
    ws:            WebSocketUpgrade,
    State(_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    // Reserved for future trickle-ICE / multi-display coordination.
    while let Some(Ok(msg)) = socket.recv().await {
        tracing::debug!("WS: {:?}", msg);
    }
}