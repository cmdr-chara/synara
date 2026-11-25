//! Passive provider usage snapshots.
//!
//! Deliberately unlike upstream: upstream's live usage fetchers read provider
//! OAuth tokens out of credential stores and replay them against provider web
//! APIs, the pattern that gets user accounts banned. Synara never touches
//! credential material: the only usage evidence served here is what the
//! official provider CLIs emit themselves while running turns (Codex
//! `account/rateLimits/updated`, Claude `rate_limit_info`). Runners persist the
//! raw payloads per provider; this module shapes them into the shared
//! `ServerProviderUsageSnapshot` contract.

use serde_json::{json, Value};
use std::path::PathBuf;

use crate::todo::WsErr;

fn err(message: &str) -> WsErr {
    WsErr(message.to_string(), None)
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "server.listProviderUsage" | "server.getProviderUsageSnapshot"
    )
}

fn usage_dir() -> PathBuf {
    let home = std::env::var("SYNARA_HOME")
        .ok()
        .filter(|h| !h.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
                .join(".synara")
        });
    home.join("provider-usage")
}

/// Persist one raw usage payload observed from a provider CLI. Best-effort:
/// usage capture must never break a turn.
pub fn persist(provider: &str, payload: &Value) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let record = json!({
        "provider": provider,
        "capturedAt": crate::todo::iso_from_ms(now_ms),
        "payload": payload,
    });
    let dir = usage_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(format!("{provider}.json"));
    let tmp = dir.join(format!("{provider}.json.tmp"));
    if let Ok(text) = serde_json::to_string(&record) {
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

fn load_record(path: &std::path::Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn epoch_to_iso(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    let secs = v.as_i64()?;
    Some(crate::todo::iso_from_ms(secs * 1000))
}

/// Shape a persisted raw payload into the ServerProviderUsageSnapshot contract:
/// {provider, updatedAt, limits[], usageLines[], source, status?, stale?}.
fn snapshot_from(provider: &str, record: &Value) -> Value {
    let captured = record
        .get("capturedAt")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let payload = record.get("payload").cloned().unwrap_or(Value::Null);
    let mut limits: Vec<Value> = Vec::new();
    let mut usage_lines: Vec<Value> = Vec::new();

    if provider == "codex" {
        // app-server notification shape: {rateLimits:{primary,secondary}} or
        // {rateLimitsByLimitId:{codex:{primary,secondary,credits}}}
        let rates = payload
            .pointer("/rateLimitsByLimitId/codex")
            .cloned()
            .or_else(|| payload.get("rateLimits").cloned())
            .unwrap_or(payload.clone());
        for (key, label, fallback_mins) in [
            ("primary", "5h", 300i64),
            ("secondary", "Weekly", 10_080i64),
        ] {
            if let Some(window) = rates.get(key).and_then(Value::as_object) {
                let percent = window.get("usedPercent").and_then(Value::as_f64);
                let resets = window.get("resetsAt").and_then(epoch_to_iso);
                let duration = window
                    .get("windowDurationMins")
                    .and_then(Value::as_i64)
                    .unwrap_or(fallback_mins);
                let mut entry = json!({"window": label, "windowDurationMins": duration});
                if let Some(p) = percent {
                    entry["usedPercent"] = json!(p.clamp(0.0, 100.0));
                }
                if let Some(r) = resets {
                    entry["resetsAt"] = json!(r);
                }
                limits.push(entry);
            }
        }
        if let Some(credits) = rates.get("credits").and_then(Value::as_object) {
            if let Some(balance) = credits
                .get("balance")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| credits.get("balance").and_then(Value::as_f64).map(|n| n.to_string()))
            {
                usage_lines.push(json!({"label": "Credits", "value": format!("{balance} remaining")}));
            }
        }
    } else {
        // Claude (and any other passive provider): rate_limit_info events only
        // carry rejection/reset signals, so limits stay sparse by design.
        let info = payload
            .get("rate_limit_info")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_else(|| payload.as_object().cloned().unwrap_or_default());
        let kind = info
            .get("rateLimitType")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if let Some(resets) = info.get("resetsAt").and_then(epoch_to_iso) {
            limits.push(json!({"window": kind, "usedPercent": 100, "resetsAt": resets}));
        }
    }

    let mut snapshot = json!({
        "provider": provider,
        "updatedAt": captured,
        "limits": limits,
        "usageLines": usage_lines,
        "source": "passive-cli-evidence",
        "stale": true,
    });
    if limits.is_empty() && usage_lines.is_empty() {
        snapshot["detail"] = json!(
            "No live usage evidence yet; passive snapshots update while provider sessions run."
        );
    }
    snapshot
}

fn all_snapshots() -> Vec<Value> {
    let dir = usage_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(provider) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if let Some(record) = load_record(&path) {
                out.push(snapshot_from(provider, &record));
            }
        }
    }
    out.sort_by(|a, b| {
        a.get("provider")
            .and_then(Value::as_str)
            .cmp(&b.get("provider").and_then(Value::as_str))
    });
    out
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "server.listProviderUsage" => {
            let filter: Vec<String> = payload
                .get("providers")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            let mut snapshots = all_snapshots();
            if !filter.is_empty() {
                snapshots.retain(|s| {
                    s.get("provider")
                        .and_then(Value::as_str)
                        .is_some_and(|p| filter.iter().any(|f| f == p))
                });
            }
            Ok(Value::Array(snapshots))
        }
        "server.getProviderUsageSnapshot" => {
            let provider = payload
                .get("provider")
                .and_then(Value::as_str)
                .ok_or_else(|| err("server.getProviderUsageSnapshot requires a provider"))?;
            let path = usage_dir().join(format!("{provider}.json"));
            let Some(record) = load_record(&path) else {
                return Err(err(&format!(
                    "No passive usage evidence captured for provider '{provider}' yet"
                )));
            };
            Ok(snapshot_from(provider, &record))
        }
        _ => Err(err("unhandled provider usage tag")),
    }
}
