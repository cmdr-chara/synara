//! The Rust port of `apps/web/src/threadDetailResumeCursors.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Mutex;

/// `resumeCursorByThreadId`, persisting across bridge calls like the module
/// state does across the recorded session.
static CURSORS: Mutex<Option<HashMap<String, Value>>> = Mutex::new(None);
static NEXT_OBJECT_KEY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// The canonical SameValueZero key for a JS value, or `None` for objects and
/// arrays: every fresh object literal is a distinct `Map` key in JS, so a
/// structurally equal arg can never hit a stored entry.
fn map_key(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(format!("s\u{0}{s}")),
        Value::Number(n) => Some(format!(
            "n\u{0}{}",
            n.as_f64().unwrap_or(f64::NAN).to_bits()
        )),
        Value::Bool(b) => Some(format!("b\u{0}{b}")),
        Value::Null => Some("l".to_string()),
        Value::Object(m) if m.contains_key("$undefined") => Some("u".to_string()),
        Value::Object(m) if m.contains_key("$num") => {
            crate::js::as_number(v).map(|n| format!("n\u{0}{}", n.to_bits()))
        }
        _ => None,
    }
}

/// A `Map` key for a fresh object/array argument: identity-unique in JS, so it
/// is stored but can never be hit by a later (structurally equal) arg.
fn fresh_object_key() -> String {
    format!(
        "o\u{0}{}",
        NEXT_OBJECT_KEY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}

/// The key a write uses: scalars by SameValueZero, objects/arrays by identity.
fn write_key(v: &Value) -> String {
    map_key(v).unwrap_or_else(fresh_object_key)
}

fn cursors() -> std::sync::MutexGuard<'static, Option<HashMap<String, Value>>> {
    match CURSORS.lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

/// Port of `advanceThreadDetailResumeCursor` (`apps/web/src/threadDetailResumeCursors.ts:22`).
pub fn advance_thread_detail_resume_cursor(
    thread_id: serde_json::Value,
    sequence: f64,
) -> Result<(), String> {
    let key = write_key(&thread_id);
    let mut guard = cursors();
    let map = guard.get_or_insert_with(HashMap::new);
    let current = map.get(&key).and_then(crate::js::as_number);
    if current.is_none() || sequence > current.unwrap_or(f64::NAN) {
        map.insert(key, crate::js::number(sequence));
    }
    Ok(())
}

/// Port of `setThreadDetailResumeCursor` (`apps/web/src/threadDetailResumeCursors.ts:34`).
pub fn set_thread_detail_resume_cursor(
    thread_id: serde_json::Value,
    sequence: f64,
) -> Result<(), String> {
    let key = write_key(&thread_id);
    let mut guard = cursors();
    guard
        .get_or_insert_with(HashMap::new)
        .insert(key, crate::js::number(sequence));
    Ok(())
}

/// Port of `getThreadDetailResumeCursor` (`apps/web/src/threadDetailResumeCursors.ts:38`).
pub fn get_thread_detail_resume_cursor(
    thread_id: serde_json::Value,
) -> Result<Option<f64>, String> {
    let key = match map_key(&thread_id) {
        Some(k) => k,
        None => return Ok(None),
    };
    let guard = cursors();
    Ok(guard
        .as_ref()
        .and_then(|m| m.get(&key))
        .and_then(crate::js::as_number))
}

/// Port of `hasThreadDetailResumeCursor` (`apps/web/src/threadDetailResumeCursors.ts:42`).
pub fn has_thread_detail_resume_cursor(thread_id: serde_json::Value) -> Result<bool, String> {
    let key = match map_key(&thread_id) {
        Some(k) => k,
        None => return Ok(false),
    };
    let guard = cursors();
    Ok(guard.as_ref().is_some_and(|m| m.contains_key(&key)))
}

/// Port of `clearThreadDetailResumeCursor` (`apps/web/src/threadDetailResumeCursors.ts:46`).
pub fn clear_thread_detail_resume_cursor(thread_id: serde_json::Value) -> Result<(), String> {
    if let Some(key) = map_key(&thread_id)
        && let Some(m) = cursors().as_mut()
    {
        m.remove(&key);
    }
    Ok(())
}

/// Port of `retainThreadDetailResumeCursors` (`apps/web/src/threadDetailResumeCursors.ts:56`).
///
/// `threadIds` is a `ReadonlySet` — a shape the wire cannot carry, so every
/// bridged input throws the way the original does: `null`/`undefined` fail on
/// property access, anything else fails calling `.has` (only while stored
/// keys exist, since the loop is over map keys).
pub fn retain_thread_detail_resume_cursors(thread_ids: serde_json::Value) -> Result<(), String> {
    let guard = cursors();
    let has_keys = guard.as_ref().is_some_and(|m| !m.is_empty());
    if !has_keys {
        return Ok(());
    }
    match &thread_ids {
        Value::Null => Err("Cannot read properties of null (reading 'has')".to_string()),
        Value::Object(m) if m.contains_key("$undefined") => {
            Err("Cannot read properties of undefined (reading 'has')".to_string())
        }
        _ => Err("threadIds.has is not a function".to_string()),
    }
}

/// Port of `resetThreadDetailResumeCursors` (`apps/web/src/threadDetailResumeCursors.ts:71`).
pub fn reset_thread_detail_resume_cursors() -> Result<(), String> {
    if let Some(m) = cursors().as_mut() {
        m.clear();
    }
    Ok(())
}

/// Port of `buildThreadSubscribeInput` (`apps/web/src/threadDetailResumeCursors.ts:81`).
pub fn build_thread_subscribe_input(
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let after = map_key(&thread_id).and_then(|key| {
        cursors()
            .as_ref()
            .and_then(|m| m.get(&key))
            .and_then(crate::js::as_number)
    });
    Ok(match after {
        Some(seq) => {
            json!({"threadId": thread_id, "afterSequence": crate::js::number(seq), "batchReplay": true})
        }
        None => json!({"threadId": thread_id}),
    })
}

/// Port of `resetThreadDetailResumeCursorsForTests` (`apps/web/src/threadDetailResumeCursors.ts:88`).
pub fn reset_thread_detail_resume_cursors_for_tests() -> Result<(), String> {
    reset_thread_detail_resume_cursors()
}
