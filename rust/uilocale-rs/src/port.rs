//! The Rust port of `packages/shared/src/uiLocale.ts`.
//!
//! Behavior inferred clean-room from recorded cases and black-box probes:
//! `ns:key` shape — ns matches `^[a-z][a-z0-9_-]*$`, key matches
//! `^[A-Za-z0-9_][A-Za-z0-9_.-]*$`, and `namespaces` must be a plain object
//! that has `ns` as an own/inherited key.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_ns(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

fn is_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

/// Port of `isUiMessageKey` (`packages/shared/src/uiLocale.ts:7`).
pub fn is_ui_message_key(message: String, namespaces: Value) -> Result<bool, String> {
    let Value::Object(map) = &namespaces else {
        return Ok(false);
    };
    let mut parts = message.split(':');
    let (Some(ns), Some(key), None) = (parts.next(), parts.next(), parts.next()) else {
        return Ok(false);
    };
    Ok(is_ns(ns) && is_key(key) && map.contains_key(ns))
}
