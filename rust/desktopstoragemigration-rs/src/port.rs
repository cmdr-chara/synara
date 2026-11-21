//! The Rust port of `apps/desktop/src/desktopStorageMigration.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
const MAX_SNAPSHOT_ENTRIES: usize = 2_048;
const MAX_SNAPSHOT_KEY_LENGTH: usize = 512;
const MAX_SNAPSHOT_VALUE_LENGTH: usize = 16 * 1024 * 1024;

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn posix_normalize(path: &str) -> String {
    let absolute = path.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.last().is_some_and(|p| *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            s => parts.push(s),
        }
    }
    let joined = parts.join("/");
    match (absolute, joined.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{joined}"),
        (false, true) => ".".to_string(),
        (false, false) => joined,
    }
}

fn posix_join(parts: &[String]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("/");
    posix_normalize(&joined)
}

fn all_digits(s: &str, lo: usize, hi: usize) -> bool {
    s.len() >= lo && s.len() <= hi && s.bytes().all(|b| b.is_ascii_digit())
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

/// Port of `isSynaraStorageKey` (`apps/desktop/src/desktopStorageMigration.ts:20`).
pub fn is_synara_storage_key(key: String) -> Result<bool, String> {
    Ok(key.starts_with("synara:") || key.starts_with("synara."))
}

/// Port of `validateSynaraStorageSnapshot` (`apps/desktop/src/desktopStorageMigration.ts:28`).
pub fn validate_synara_storage_snapshot(value: Value) -> Result<Value, String> {
    let null = Ok(Value::Null);
    let Value::Object(o) = &value else {
        return null;
    };
    // `value.version !== 1` and `!isPlainRecord(value.entries)`.
    if o.get("version").and_then(js::as_number) != Some(1.0) {
        return null;
    }
    let Value::Object(entries) = o.get("entries").cloned().unwrap_or(Value::Null) else {
        return null;
    };
    let Value::String(exported_at) = o.get("exportedAt").cloned().unwrap_or(Value::Null) else {
        return null;
    };
    if !date_parse_finite(&exported_at) {
        return null;
    }
    if entries.len() > MAX_SNAPSHOT_ENTRIES {
        return null;
    }
    for (key, entry_value) in &entries {
        if !(key.starts_with("synara:") || key.starts_with("synara."))
            || key.is_empty()
            || utf16_len(key) > MAX_SNAPSHOT_KEY_LENGTH
        {
            return null;
        }
        let Value::String(v) = entry_value else {
            return null;
        };
        if utf16_len(v) > MAX_SNAPSHOT_VALUE_LENGTH {
            return null;
        }
    }
    match serde_json::to_string(&value) {
        Ok(s) if s.len() <= MAX_SNAPSHOT_BYTES => Ok(value.clone()),
        _ => null,
    }
}

/// Port of `resolveSynaraStorageSnapshotPath` (`apps/desktop/src/desktopStorageMigration.ts:63`).
pub fn resolve_synara_storage_snapshot_path(user_data_path: String) -> Result<String, String> {
    Ok(posix_join(&[
        user_data_path,
        "synara-storage-origin-v1.json".to_string(),
    ]))
}

/// Port of `readSynaraStorageSnapshot` (`apps/desktop/src/desktopStorageMigration.ts:67`).
pub fn read_synara_storage_snapshot(snapshot_path: String) -> Result<Value, String> {
    let read = (|| -> Option<Value> {
        let stats = std::fs::metadata(&snapshot_path).ok()?;
        if !stats.is_file() || stats.len() as usize > MAX_SNAPSHOT_BYTES {
            return None;
        }
        let contents = std::fs::read_to_string(&snapshot_path).ok()?;
        let parsed: Value = serde_json::from_str(&contents).ok()?;
        validate_synara_storage_snapshot(parsed)
            .ok()
            .filter(|v| !v.is_null())
    })();
    Ok(read.unwrap_or(Value::Null))
}

/// Port of `acknowledgeSynaraStorageSnapshot` (`apps/desktop/src/desktopStorageMigration.ts:79`).
pub async fn acknowledge_synara_storage_snapshot(snapshot_path: String) -> Result<(), String> {
    // FS.promises.rm(path, { force: true }).catch(() => undefined)
    let _ = std::fs::remove_file(&snapshot_path);
    Ok(())
}
