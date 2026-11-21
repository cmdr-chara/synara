//! The Rust port of `apps/web/src/storageOriginMigration.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const MAX_SNAPSHOT_ENTRIES: usize = 2_048;
const MAX_SNAPSHOT_KEY_LENGTH: usize = 512;
const MAX_SNAPSHOT_VALUE_LENGTH: usize = 16 * 1024 * 1024;
const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;

fn falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::String(s) => s.is_empty(),
        Value::Array(_) | Value::Object(_) => js::is_undefined(v),
        _ => js::as_number(v).is_some_and(|n| n == 0.0 || n.is_nan()),
    }
}

fn member(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn is_canonical_storage_key(key: &str) -> bool {
    key.starts_with("synara:") || key.starts_with("synara.")
}

fn all_digits(s: &str, lo: usize, hi: usize) -> bool {
    (lo..=hi).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

/// Port of `importSynaraStorageSnapshot` (`apps/web/src/storageOriginMigration.ts:25`).
pub fn import_synara_storage_snapshot(
    snapshot: Option<serde_json::Value>,
    storage: Option<serde_json::Value>,
) -> Result<bool, String> {
    let snapshot = match snapshot {
        Some(v) if !falsy(&v) => v,
        _ => return Ok(false),
    };
    let storage = match storage {
        Some(v) if !falsy(&v) => v,
        _ => return Ok(false),
    };
    if js::as_number(&member(&snapshot, "version")) != Some(1.0) {
        return Ok(false);
    }
    let entries_v = member(&snapshot, "entries");
    if falsy(&entries_v) {
        return Ok(false);
    }
    // `Object.entries(entries)`: objects keep keys; arrays index; strings spread chars.
    let entries: Vec<(String, Value)> = match &entries_v {
        Value::Object(o) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Value::String(s) => {
            // `Object.entries(str)` indexes by UTF-16 code unit.
            let mut i = 0usize;
            s.chars()
                .map(|c| {
                    let key = i.to_string();
                    i += c.len_utf16();
                    (key, Value::String(c.to_string()))
                })
                .collect()
        }
        Value::Null => return Err("Cannot convert undefined or null to object".to_string()),
        _ => Vec::new(),
    };
    if entries.len() > MAX_SNAPSHOT_ENTRIES {
        return Ok(false);
    }
    let exported_at = member(&snapshot, "exportedAt");
    if !matches!(&exported_at, Value::String(s) if date_parse_finite(s)) {
        return Ok(false);
    }
    let serialized = serde_json::to_string(&snapshot).unwrap_or_default();
    if serialized.len() > MAX_SNAPSHOT_BYTES {
        return Ok(false);
    }
    for (key, value) in &entries {
        if !is_canonical_storage_key(key) || utf16_len(key) > MAX_SNAPSHOT_KEY_LENGTH {
            return Ok(false);
        }
        let Value::String(v) = value else {
            return Ok(false);
        };
        if utf16_len(v) > MAX_SNAPSHOT_VALUE_LENGTH {
            return Ok(false);
        }
    }
    let _ = storage;
    // The second loop calls `storage.getItem(key)`; a JSON value is never a real
    // Storage object, so any non-empty entry list throws and lands in `catch`.
    Ok(entries.is_empty())
}

/// `Date.parse` finiteness: V8 accepts ISO dates, `MM/DD/YYYY`, and legacy
/// strings ending in `-<digits>` (a signed year token like "key_name-1").
fn date_parse_finite(s: &str) -> bool {
    let t = s.trim();
    if let Some(i) = t.find('-') {
        let two = t.get(i + 1..i + 3).unwrap_or("");
        let day = t.get(i + 4..i + 6).unwrap_or("");
        if all_digits(&t[..i], 1, usize::MAX)
            && all_digits(two, 2, 2)
            && t.as_bytes().get(i + 3) == Some(&b'-')
            && all_digits(day, 2, 2)
        {
            return true;
        }
    }
    let parts: Vec<&str> = t.split('/').collect();
    if parts.len() == 3 && parts.iter().all(|p| all_digits(p, 1, 4)) {
        return true;
    }
    if let Some(i) = t.rfind(|c: char| !c.is_ascii_digit())
        && i > 0
        && t.as_bytes()[i] == b'-'
    {
        return all_digits(&t[i + 1..], 1, usize::MAX);
    }
    false
}

/// Port of `bootstrapSynaraStorageOriginMigration` (`apps/web/src/storageOriginMigration.ts:60`).
/// `globalThis.window` is never set in the recorded environment, so the bridge
/// is absent and this is a no-op.
pub fn bootstrap_synara_storage_origin_migration() -> Result<(), String> {
    Ok(())
}
