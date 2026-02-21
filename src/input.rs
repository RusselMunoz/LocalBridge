use std::sync::Arc;
use serde::Deserialize;
use tracing::{debug, warn};
use webrtc::data_channel::RTCDataChannel;

/// 'InputEvent' represents the different types of mouse and keyboard actions 
/// that can be sent from the browser.
/// We use 'serde' to automatically convert JSON from the browser into this Enum.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    MouseMove   { x: f64, y: f64 },
    MouseDown   { x: f64, y: f64, button: u8 },
    MouseUp     { x: f64, y: f64, button: u8 },
    MouseScroll { dx: f64, dy: f64 },
    KeyDown     { code: String },
    KeyUp       { code: String },
}

/// Sets up the handler for messages arriving on the WebRTC data channel.
pub async fn handle_data_channel(dc: Arc<RTCDataChannel>) {
    // This callback is triggered whenever the browser sends a message through the data channel.
    dc.on_message(Box::new(|msg| {
        Box::pin(async move {
            // Convert the raw bytes from the message into a UTF-8 string.
            if let Ok(text) = std::str::from_utf8(&msg.data) {
                // Try to parse the string as a JSON 'InputEvent'.
                match serde_json::from_str::<InputEvent>(text) {
                    Ok(ev) => inject(ev), // If successful, "inject" it into the system.
                    Err(e) => warn!("Bad input: {e}"),
                }
            }
        })
    }));
}

/// 'inject' is where we would actually simulate mouse and keyboard events on the host computer.
fn inject(event: InputEvent) {
    // For now, we just print the event to the debug console.
    // To actually move the mouse, you would use a library like 'enigo'.
    debug!("Input → {:?}", event);
}