//! Persistent desktop window state.
//!
//! Stores both portable logical bounds and the platform-native restoration
//! blob when GPUI exposes one. Writes are atomic so a crash cannot leave a
//! truncated state file.

use gpui_kit::{point, px, size, Bounds, Window, WindowBounds};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

pub struct PersistedWindowState {
    pub window_bounds: Option<WindowBounds>,
    pub native_state: Option<Vec<u8>>,
}

pub fn load() -> PersistedWindowState {
    let Ok(text) = fs::read_to_string(state_path()) else {
        return PersistedWindowState {
            window_bounds: None,
            native_state: None,
        };
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return PersistedWindowState {
            window_bounds: None,
            native_state: None,
        };
    };
    PersistedWindowState {
        window_bounds: value.get("bounds").and_then(decode_bounds),
        native_state: value
            .get("native")
            .and_then(Value::as_str)
            .and_then(decode_hex),
    }
}

pub fn save_from_window(window: &Window) {
    let native_state = window.native_window_state();
    let bounds = encode_bounds(window.window_bounds());
    let payload = serde_json::json!({
        "version": 1,
        "bounds": bounds,
        "native": native_state.as_deref().map(encode_hex),
    });
    write_atomic(&state_path(), payload.to_string().as_bytes());
}

fn state_path() -> PathBuf {
    crate::backend::configured_home_dir()
        .join("userdata")
        .join("window-state.json")
}

fn write_atomic(path: &PathBuf, bytes: &[u8]) {
    let Some(parent) = path.parent() else { return };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let temp = path.with_extension("json.tmp");
    if fs::write(&temp, bytes).is_ok() {
        let _ = fs::rename(&temp, path);
    }
}

fn encode_bounds(bounds: WindowBounds) -> Value {
    let (kind, bounds) = match bounds {
        WindowBounds::Windowed(bounds) => ("windowed", bounds),
        WindowBounds::Maximized(bounds) => ("maximized", bounds),
        WindowBounds::Fullscreen(bounds) => ("fullscreen", bounds),
    };
    serde_json::json!({
        "kind": kind,
        "x": f32::from(bounds.origin.x),
        "y": f32::from(bounds.origin.y),
        "width": f32::from(bounds.size.width),
        "height": f32::from(bounds.size.height),
    })
}

fn decode_bounds(value: &Value) -> Option<WindowBounds> {
    let number = |key: &str| value.get(key).and_then(Value::as_f64).map(|v| v as f32);
    let bounds = Bounds {
        origin: point(px(number("x")?), px(number("y")?)),
        size: size(px(number("width")?), px(number("height")?)),
    };
    if f32::from(bounds.size.width) <= 0.0 || f32::from(bounds.size.height) <= 0.0 {
        return None;
    }
    match value.get("kind").and_then(Value::as_str) {
        Some("windowed") => Some(WindowBounds::Windowed(bounds)),
        Some("maximized") => Some(WindowBounds::Maximized(bounds)),
        Some("fullscreen") => Some(WindowBounds::Fullscreen(bounds)),
        _ => None,
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 || text.len() > 1024 * 1024 {
        return None;
    }
    let value = |byte: u8| -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    };
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| Some(value(pair[0])? << 4 | value(pair[1])?))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        let bytes = [0x00, 0x7f, 0x80, 0xff];
        assert_eq!(decode_hex(&encode_hex(&bytes)), Some(bytes.to_vec()));
    }

    #[test]
    fn rejects_invalid_hex() {
        assert_eq!(decode_hex("abc"), None);
        assert_eq!(decode_hex("zz"), None);
    }

    #[test]
    fn bounds_round_trip() {
        let original = WindowBounds::Windowed(Bounds {
            origin: point(px(20.), px(30.)),
            size: size(px(1440.), px(900.)),
        });
        let decoded = decode_bounds(&encode_bounds(original));
        assert_eq!(decoded, Some(original));
    }

    #[test]
    fn rejects_zero_sized_bounds() {
        let value = serde_json::json!({
            "kind": "windowed",
            "x": 0,
            "y": 0,
            "width": 0,
            "height": 800,
        });
        assert_eq!(decode_bounds(&value), None);
    }
}
