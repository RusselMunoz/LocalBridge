use std::sync::Arc;
use serde::Deserialize;
use tracing::{debug, warn};
use webrtc::data_channel::RTCDataChannel;

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

pub async fn handle_data_channel(dc: Arc<RTCDataChannel>) {
    dc.on_message(Box::new(|msg| {
        Box::pin(async move {
            if let Ok(text) = std::str::from_utf8(&msg.data) {
                match serde_json::from_str::<InputEvent>(text) {
                    Ok(ev) => inject(ev),
                    Err(e) => warn!("Bad input: {e}"),
                }
            }
        })
    }));
}

fn inject(event: InputEvent) {
    // TODO: add `enigo = "0.2"` to Cargo.toml and wire up real input injection.
    // Example:
    //   use enigo::{Enigo, Mouse, Settings};
    //   let mut e = Enigo::new(&Settings::default()).unwrap();
    //   e.move_mouse(x as i32, y as i32, enigo::Coordinate::Abs).unwrap();
    debug!("Input → {:?}", event);
}