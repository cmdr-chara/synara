//! The Rust port of `apps/web/src/browserStateStore.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const BROWSER_HISTORY_LIMIT: usize = 12;

fn is_plain_object(v: &Value) -> bool {
    matches!(v, Value::Object(_))
}

fn unsafe_key(key: &str) -> bool {
    matches!(key, "__proto__" | "constructor" | "prototype")
}

fn sanitize_browser_history_entry(raw: &Value) -> Option<Value> {
    let obj = raw.as_object()?;
    let url = obj.get("url")?.as_str()?;
    let title = obj.get("title")?.as_str()?;
    let tab_id = obj.get("tabId")?.as_str()?;
    Some(serde_json::json!({ "url": url, "title": title, "tabId": tab_id }))
}

/// Port of `sanitizeRecentHistoryByThreadId` (`apps/web/src/browserStateStore.ts:99`).
pub fn sanitize_recent_history_by_thread_id(value: Value) -> Result<Value, String> {
    if !is_plain_object(&value) {
        return Ok(Value::Object(serde_json::Map::new()));
    }
    let obj = value.as_object().unwrap();
    let mut result = serde_json::Map::new();
    for (key, raw_entries) in obj {
        if unsafe_key(key) {
            continue;
        }
        let Some(entries) = raw_entries.as_array() else {
            continue;
        };
        let cleaned: Vec<Value> = entries
            .iter()
            .filter_map(sanitize_browser_history_entry)
            .take(BROWSER_HISTORY_LIMIT)
            .collect();
        if !cleaned.is_empty() {
            result.insert(key.clone(), Value::Array(cleaned));
        }
    }
    Ok(Value::Object(result))
}
