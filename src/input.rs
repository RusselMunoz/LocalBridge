use std::sync::Arc;
use serde::Deserialize;
use tracing::{debug, warn};
use webrtc::data_channel::RTCDataChannel;
use enigo::{Enigo, KeyboardControllable, MouseControllable, MouseButton, Key};

/// 'InputEvent' represents the different types of mouse and keyboard actions
/// that can be sent from the browser.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum InputEvent {
    MouseMove { x: f64, y: f64 },
    MouseDown { x: f64, y: f64, button: u8 },
    MouseUp { x: f64, y: f64, button: u8 },
    MouseScroll { dx: f64, dy: f64 },
    KeyDown { code: String },
    KeyUp { code: String },
}

/// Sets up the handler for messages arriving on the WebRTC data channel.
pub async fn handle_data_channel(dc: Arc<RTCDataChannel>) {
    let enigo = Arc::new(std::sync::Mutex::new(Enigo::new()));

    dc.on_message(Box::new(move |msg| {
        let enigo = Arc::clone(&enigo);
        Box::pin(async move {
            if let Ok(text) = std::str::from_utf8(&msg.data) {
                match serde_json::from_str::<InputEvent>(text) {
                    Ok(ev) => {
                        if let Ok(mut enigo_guard) = enigo.lock() {
                            inject(&mut *enigo_guard, ev);
                        }
                    }
                    Err(e) => warn!("Bad input: {e}"),
                }
            }
        })
    }));
}

extern "system" {
    fn SetCursorPos(x: i32, y: i32) -> i32;
}

/// 'inject' simulates mouse and keyboard events on the host computer.
fn inject(enigo: &mut Enigo, event: InputEvent) {
    match event {
        InputEvent::MouseMove { x, y } => {
            if let Some((w, h)) = get_monitor_dimensions() {
                let abs_x = (x * w as f64).round() as i32;
                let abs_y = (y * h as f64).round() as i32;
                debug!("MouseMove -> x={} y={}", abs_x, abs_y);
                unsafe {
                    SetCursorPos(abs_x, abs_y);
                }
            }
        }
        InputEvent::MouseDown { x, y, button } => {
            if let Some((w, h)) = get_monitor_dimensions() {
                let abs_x = (x * w as f64).round() as i32;
                let abs_y = (y * h as f64).round() as i32;
                unsafe {
                    SetCursorPos(abs_x, abs_y);
                }
            }
            if let Some(btn) = map_button(button) {
                debug!("MouseDown -> button={:?}", btn);
                enigo.mouse_down(btn);
            }
        }
        InputEvent::MouseUp { x, y, button } => {
            if let Some((w, h)) = get_monitor_dimensions() {
                let abs_x = (x * w as f64).round() as i32;
                let abs_y = (y * h as f64).round() as i32;
                unsafe {
                    SetCursorPos(abs_x, abs_y);
                }
            }
            if let Some(btn) = map_button(button) {
                debug!("MouseUp -> button={:?}", btn);
                enigo.mouse_up(btn);
            }
        }
        InputEvent::MouseScroll { dx, dy } => {
            if dx != 0.0 {
                let ticks = dx.round() as i32;
                debug!("MouseScroll X -> {}", ticks);
                enigo.mouse_scroll_x(ticks);
            }
            if dy != 0.0 {
                let ticks = dy.round() as i32;
                debug!("MouseScroll Y -> {}", ticks);
                enigo.mouse_scroll_y(ticks);
            }
        }
        InputEvent::KeyDown { code } => {
            if let Some(key) = map_key(&code) {
                debug!("KeyDown -> code={} key={:?}", code, key);
                enigo.key_down(key);
            }
        }
        InputEvent::KeyUp { code } => {
            if let Some(key) = map_key(&code) {
                debug!("KeyUp -> code={} key={:?}", code, key);
                enigo.key_up(key);
            }
        }
    }
}

fn get_monitor_dimensions() -> Option<(usize, usize)> {
    if let Ok(mon) = windows_capture::monitor::Monitor::primary() {
        if let (Ok(w), Ok(h)) = (mon.width(), mon.height()) {
            return Some((w as usize, h as usize));
        }
    }
    None
}

fn map_button(button: u8) -> Option<MouseButton> {
    match button {
        0 => Some(MouseButton::Left),
        1 => Some(MouseButton::Middle),
        2 => Some(MouseButton::Right),
        _ => None,
    }
}

fn map_key(code: &str) -> Option<Key> {
    match code {
        "KeyA" => Some(Key::Layout('a')),
        "KeyB" => Some(Key::Layout('b')),
        "KeyC" => Some(Key::Layout('c')),
        "KeyD" => Some(Key::Layout('d')),
        "KeyE" => Some(Key::Layout('e')),
        "KeyF" => Some(Key::Layout('f')),
        "KeyG" => Some(Key::Layout('g')),
        "KeyH" => Some(Key::Layout('h')),
        "KeyI" => Some(Key::Layout('i')),
        "KeyJ" => Some(Key::Layout('j')),
        "KeyK" => Some(Key::Layout('k')),
        "KeyL" => Some(Key::Layout('l')),
        "KeyM" => Some(Key::Layout('m')),
        "KeyN" => Some(Key::Layout('n')),
        "KeyO" => Some(Key::Layout('o')),
        "KeyP" => Some(Key::Layout('p')),
        "KeyQ" => Some(Key::Layout('q')),
        "KeyR" => Some(Key::Layout('r')),
        "KeyS" => Some(Key::Layout('s')),
        "KeyT" => Some(Key::Layout('t')),
        "KeyU" => Some(Key::Layout('u')),
        "KeyV" => Some(Key::Layout('v')),
        "KeyW" => Some(Key::Layout('w')),
        "KeyX" => Some(Key::Layout('x')),
        "KeyY" => Some(Key::Layout('y')),
        "KeyZ" => Some(Key::Layout('z')),

        "Digit0" => Some(Key::Layout('0')),
        "Digit1" => Some(Key::Layout('1')),
        "Digit2" => Some(Key::Layout('2')),
        "Digit3" => Some(Key::Layout('3')),
        "Digit4" => Some(Key::Layout('4')),
        "Digit5" => Some(Key::Layout('5')),
        "Digit6" => Some(Key::Layout('6')),
        "Digit7" => Some(Key::Layout('7')),
        "Digit8" => Some(Key::Layout('8')),
        "Digit9" => Some(Key::Layout('9')),

        "Enter" => Some(Key::Return),
        "Space" => Some(Key::Space),
        "Backspace" => Some(Key::Backspace),
        "Tab" => Some(Key::Tab),
        "Escape" => Some(Key::Escape),
        
        "ShiftLeft" | "ShiftRight" => Some(Key::Shift),
        "ControlLeft" | "ControlRight" => Some(Key::Control),
        "AltLeft" | "AltRight" => Some(Key::Alt),
        "MetaLeft" | "MetaRight" => Some(Key::Meta),

        "ArrowLeft" => Some(Key::LeftArrow),
        "ArrowRight" => Some(Key::RightArrow),
        "ArrowUp" => Some(Key::UpArrow),
        "ArrowDown" => Some(Key::DownArrow),

        "Delete" => Some(Key::Delete),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        
        "Minus" => Some(Key::Layout('-')),
        "Equal" => Some(Key::Layout('=')),
        "BracketLeft" => Some(Key::Layout('[')),
        "BracketRight" => Some(Key::Layout(']')),
        "Backslash" => Some(Key::Layout('\\')),
        "Semicolon" => Some(Key::Layout(';')),
        "Quote" => Some(Key::Layout('\'')),
        "Comma" => Some(Key::Layout(',')),
        "Period" => Some(Key::Layout('.')),
        "Slash" => Some(Key::Layout('/')),
        "Backquote" => Some(Key::Layout('`')),

        _ => {
            if code.starts_with("Key") && code.len() == 4 {
                code.chars().nth(3).map(|c| Key::Layout(c.to_ascii_lowercase()))
            } else if code.starts_with("Digit") && code.len() == 6 {
                code.chars().nth(5).map(Key::Layout)
            } else {
                None
            }
        }
    }
}