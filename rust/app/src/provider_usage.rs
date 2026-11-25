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
        "server.listProviderUsage" | "server.getProviderUsageSnapshot" | "server.consumeCodexResetCredit"
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
        "server.consumeCodexResetCredit" => consume_codex_reset_credit(payload).map(|outcome| json!({"outcome": outcome})),
        _ => Err(err("unhandled provider usage tag")),
    }
}

// ---- Codex reset-credit consumption ----
//
// Mirrors upstream consumeCodexResetCredit: a short-lived `codex app-server`
// process verifies the account still matches, checks whether any rate-limit
// window is at >=90% used, then consumes the reset credit idempotently. The
// official CLI owns all credential material; Synara never sees a token.

fn can_use_reset_credit(usage: &Value) -> Option<bool> {
    let core = usage
        .pointer("/rateLimitsByLimitId/codex")
        .cloned()
        .or_else(|| usage.get("rateLimits").cloned())?;
    let mut saw_window = false;
    let mut exhausted = false;
    for key in ["primary", "secondary"] {
        if let Some(percent) = core
            .get(key)
            .and_then(|w| w.get("usedPercent"))
            .and_then(Value::as_f64)
        {
            saw_window = true;
            if percent >= 90.0 {
                exhausted = true;
            }
        }
    }
    if saw_window { Some(exhausted) } else { None }
}

struct ShortLivedRpc {
    stdin: std::process::ChildStdin,
    lines: std::sync::mpsc::Receiver<String>,
    next_id: i64,
}

impl ShortLivedRpc {
    fn spawn() -> Result<(Self, std::process::Child), WsErr> {
        use std::io::BufRead;
        use std::process::{Command, Stdio};
        let binary = crate::codex_runner::codex_binary();
        let mut child = Command::new(&binary)
            .args(["app-server"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| err(&format!("could not launch `codex app-server`: {e}")))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| err("codex app-server stdin unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| err("codex app-server stdout unavailable"))?;
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            let mut reader = std::io::BufReader::new(stdout);
            loop {
                let mut line = String::new();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if tx.send(line).is_err() {
                            break;
                        }
                    }
                }
            }
        });
        Ok((Self { stdin, lines: rx, next_id: 1 }, child))
    }

    fn write(&mut self, msg: &Value) -> Result<(), WsErr> {
        use std::io::Write;
        self.stdin
            .write_all(msg.to_string().as_bytes())
            .and_then(|_| self.stdin.write_all(b"\n"))
            .and_then(|_| self.stdin.flush())
            .map_err(|e| err(&format!("codex write: {e}")))
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, WsErr> {
        let id = self.next_id;
        self.next_id += 1;
        self.write(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        loop {
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if remaining.is_zero() {
                return Err(err(&format!("codex app-server timed out on {method}")));
            }
            let line = self
                .lines
                .recv_timeout(remaining)
                .map_err(|_| err(&format!("codex app-server timed out on {method}")))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(trimmed) else {
                continue;
            };
            if v.get("id").and_then(Value::as_i64) != Some(id) {
                continue;
            }
            if let Some(error) = v.get("error") {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("codex app-server error");
                return Err(err(&format!("{method} failed: {message}")));
            }
            return Ok(v.get("result").cloned().unwrap_or(Value::Null));
        }
    }
}

fn consume_codex_reset_credit(payload: &Value) -> Result<&'static str, WsErr> {
    let account_id = payload
        .get("accountId")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("server.consumeCodexResetCredit requires accountId"))?;
    let idempotency_key = payload
        .get("idempotencyKey")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err("server.consumeCodexResetCredit requires idempotencyKey"))?;
    let credit_id = payload.get("creditId").and_then(Value::as_str);

    let (mut rpc, mut child) = ShortLivedRpc::spawn()?;
    let result = (|| -> Result<&'static str, WsErr> {
        rpc.call(
            "initialize",
            json!({
                "clientInfo": {"name": "synara_desktop", "title": "Synara Desktop", "version": "0.1.0"},
                "capabilities": {"experimentalApi": true},
            }),
        )?;
        rpc.write(&json!({"jsonrpc":"2.0","method":"initialized"}))?;

        let usage = rpc.call("account/rateLimits/read", json!({}))?;
        let usage_account = usage.get("accountId").and_then(Value::as_str).unwrap_or("");
        if usage_account != account_id {
            return Err(err("The Codex account changed. Refresh usage before using a reset."));
        }
        let Some(can_use) = can_use_reset_credit(&usage) else {
            return Err(err(
                "Current Codex usage is unavailable. Retry the same attempt when it returns.",
            ));
        };
        if !can_use {
            return Ok("nothingToReset");
        }
        let mut params = json!({"idempotencyKey": idempotency_key});
        if let Some(cid) = credit_id {
            params["creditId"] = json!(cid);
        }
        let result = rpc.call("account/rateLimitResetCredit/consume", params)?;
        match result.get("outcome").and_then(Value::as_str) {
            Some("reset") => Ok("reset"),
            Some("nothingToReset") => Ok("nothingToReset"),
            Some("noCredit") => Ok("noCredit"),
            Some("alreadyRedeemed") => Ok("alreadyRedeemed"),
            _ => Err(err("Codex returned an unknown reset result. Retry the same attempt.")),
        }
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}
