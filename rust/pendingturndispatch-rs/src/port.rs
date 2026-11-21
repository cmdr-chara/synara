//! The Rust port of `apps/web/src/pendingTurnDispatch.ts`.
//!
//! Cross-component signal that a thread has a composer turn dispatch the event
//! stream has not yet acknowledged.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashMap;

const PENDING_TURN_DISPATCH_MAX_AGE_MS: f64 = 30_000.0;

thread_local! {
    static ARMED_AT: RefCell<HashMap<String, f64>> = RefCell::new(HashMap::new());
}

/// `Map` keys compare by SameValueZero: only primitives survive the bridge,
/// objects can never be looked up again so they do not need storing.
fn key(v: &Value) -> Option<String> {
    match v {
        Value::Null => Some("null".into()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => Some(format!("n:{}", n.as_f64().unwrap_or(f64::NAN))),
        Value::String(s) => Some(format!("s:{s}")),
        _ => None,
    }
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(f64::NAN)
}

/// Port of `markPendingTurnDispatch` (`apps/web/src/pendingTurnDispatch.ts:31`).
pub fn mark_pending_turn_dispatch(thread_id: serde_json::Value) -> Result<(), String> {
    if let Some(k) = key(&thread_id) {
        ARMED_AT.with(|m| {
            m.borrow_mut().insert(k, now_ms());
        });
    }
    Ok(())
}

/// Port of `clearPendingTurnDispatch` (`apps/web/src/pendingTurnDispatch.ts:35`).
pub fn clear_pending_turn_dispatch(thread_id: serde_json::Value) -> Result<(), String> {
    if let Some(k) = key(&thread_id) {
        ARMED_AT.with(|m| {
            m.borrow_mut().remove(&k);
        });
    }
    Ok(())
}

/// Port of `hasPendingTurnDispatch` (`apps/web/src/pendingTurnDispatch.ts:39`).
pub fn has_pending_turn_dispatch(thread_id: serde_json::Value) -> Result<bool, String> {
    let Some(k) = key(&thread_id) else {
        return Ok(false);
    };
    ARMED_AT.with(|m| {
        let mut map = m.borrow_mut();
        let Some(armed_at) = map.get(&k).copied() else {
            return Ok(false);
        };
        if now_ms() - armed_at > PENDING_TURN_DISPATCH_MAX_AGE_MS {
            map.remove(&k);
            return Ok(false);
        }
        Ok(true)
    })
}
