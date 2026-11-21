//! The Rust port of `apps/web/src/editorViewState.ts` —
//! `readEditorViewState` and `storeEditorViewState`. The persisted map lives in
//! process memory; `executionStorage` is replaced by a process-wide store.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::sync::{Mutex, OnceLock};

const MAX_PERSISTED_THREADS: usize = 50;

fn store() -> &'static Mutex<Vec<(String, Value)>> {
    static STORE: OnceLock<Mutex<Vec<(String, Value)>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

/// Port of `readEditorViewState` (`apps/web/src/editorViewState.ts:37`).
pub fn read_editor_view_state(thread_id: String) -> Result<Option<serde_json::Value>, String> {
    let map = store().lock().map_err(|e| e.to_string())?;
    let Some(entry) = map.iter().find(|(k, _)| k == &thread_id).map(|(_, v)| v) else {
        return Ok(None);
    };
    let expanded = entry
        .get("expandedDirectories")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter(|p| p.is_string())
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let center_mode = match entry.get("centerMode").and_then(Value::as_str) {
        Some("file") => "file",
        _ => "diff",
    };
    Ok(Some(json!({
        "expandedDirectories": expanded,
        "centerMode": center_mode,
    })))
}

/// Port of `storeEditorViewState` (`apps/web/src/editorViewState.ts:50`).
pub fn store_editor_view_state(
    thread_id: String,
    snapshot: serde_json::Value,
) -> Result<(), String> {
    let mut map = store().lock().map_err(|e| e.to_string())?;
    let updated_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0);
    let entry = match snapshot {
        Value::Object(mut o) => {
            o.insert("updatedAt".into(), crate::js::number(updated_at));
            Value::Object(o)
        }
        other => other,
    };
    map.retain(|(k, _)| k != &thread_id);
    map.push((thread_id, entry));
    if map.len() > MAX_PERSISTED_THREADS {
        // Evict the least recently updated threads, keeping insertion order.
        let mut by_age: Vec<usize> = (0..map.len()).collect();
        by_age.sort_by(|a, b| {
            let ua = map[*a].1.get("updatedAt").and_then(crate::js::as_number);
            let ub = map[*b].1.get("updatedAt").and_then(crate::js::as_number);
            ua.unwrap_or(0.0)
                .partial_cmp(&ub.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let stale: Vec<usize> = by_age[..map.len() - MAX_PERSISTED_THREADS].to_vec();
        for i in stale.into_iter().rev() {
            map.remove(i);
        }
    }
    Ok(())
}
