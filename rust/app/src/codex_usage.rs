//! Live Codex (ChatGPT) usage fetcher — port of upstream
//! `providerUsage/providers/codex.ts`, user-approved ("Codex non banna").
//!
//! The discipline that keeps the CLI login alive is copied exactly: rotating
//! single-use refresh tokens are re-read before redemption (out-of-band
//! rotations win), redeemed at most once per fetch, and persisted back
//! atomically with unknown fields preserved. Keychain-sourced credentials are
//! never redeemed — read-only access means a burned rotation would log the
//! CLI out. Claude/Antigravity tokens remain untouchable; this module is
//! strictly Codex.
use crate::provider_usage::err_usage as err;
use crate::todo::WsErr;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

const REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";
const OAUTH_CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const KEYCHAIN_SERVICE: &str = "Codex Auth";
const ACCESS_TOKEN_REFRESH_WINDOW_MS: i64 = 5 * 60 * 1000;
const LAST_REFRESH_MAX_AGE_MS: i64 = 8 * 24 * 60 * 60 * 1000;
const CREDIT_LIST_PRICE_USD: f64 = 0.04;

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn auth_file_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::new();
    let mut push = |p: PathBuf| {
        if !paths.contains(&p) {
            paths.push(p);
        }
    };
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        push(PathBuf::from(home).join("auth.json"));
    }
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        push(PathBuf::from(xdg).join("codex").join("auth.json"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        push(home.join(".config").join("codex").join("auth.json"));
        push(home.join(".codex").join("auth.json"));
    }
    paths
}

#[derive(Clone)]
pub struct CodexOAuth {
    pub record: Value,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub account_id: Option<String>,
    /// File path when file-sourced; None when keychain-sourced (read-only).
    pub source_path: Option<PathBuf>,
}

fn parse_auth_record(record: &Value, source_path: Option<PathBuf>) -> Option<CodexOAuth> {
    let tokens = record.get("tokens")?;
    let access_token = tokens.get("access_token").and_then(Value::as_str)?.to_string();
    if access_token.is_empty() {
        return None;
    }
    Some(CodexOAuth {
        record: record.clone(),
        access_token,
        refresh_token: tokens.get("refresh_token").and_then(Value::as_str).map(str::to_string),
        account_id: tokens.get("account_id").and_then(Value::as_str).map(str::to_string),
        source_path,
    })
}

fn read_auth_file(path: &std::path::Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn read_keychain_auth() -> Option<Value> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let out = std::process::Command::new("security")
        .args(["find-generic-password", "-s", KEYCHAIN_SERVICE, "-w"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    // Keychain payloads may be base64-wrapped depending on the codex version.
    let trimmed = text.trim();
    serde_json::from_str(trimmed)
        .ok()
        .or_else(|| {
            use base64::Engine;
            let decoded = base64::engine::general_purpose::STANDARD.decode(trimmed).ok()?;
            serde_json::from_slice(&decoded).ok()
        })
}

/// Resolve the best available codex auth: first readable auth.json wins,
/// keychain as fallback. Never returns credentials for api-key-only records
/// (those have no usage endpoint).
pub fn resolve_codex_auth() -> Option<CodexOAuth> {
    for path in auth_file_paths() {
        if let Some(record) = read_auth_file(&path) {
            if let Some(auth) = parse_auth_record(&record, Some(path.clone())) {
                return Some(auth);
            }
        }
    }
    if let Some(record) = read_keychain_auth() {
        return parse_auth_record(&record, None);
    }
    None
}

fn decode_jwt_exp_ms(token: &str) -> Option<i64> {
    use base64::Engine;
    let payload = token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    let json: Value = serde_json::from_slice(&decoded).ok()?;
    json.get("exp").and_then(Value::as_i64).map(|exp| exp * 1000)
}

fn iso_to_ms(iso: &str) -> Option<i64> {
    // lenient ISO8601: enough for the CLI's own last_refresh writes
    let digits: String = iso.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    if digits.len() < 14 {
        return None;
    }
    let year: i64 = digits.get(0..4)?.parse().ok()?;
    let month: i64 = digits.get(4..6)?.parse().ok()?;
    let day: i64 = digits.get(6..8)?.parse().ok()?;
    let hour: i64 = digits.get(8..10)?.parse().ok()?;
    let min: i64 = digits.get(10..12)?.parse().ok()?;
    let sec: i64 = digits.get(12..14)?.parse().ok()?;
    // days-from-civil (Howard Hinnant)
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    Some((days * 86400 + hour * 3600 + min * 60 + sec) * 1000)
}

fn needs_refresh(auth: &CodexOAuth, now: i64) -> bool {
    if let Some(exp) = decode_jwt_exp_ms(&auth.access_token) {
        return exp - now < ACCESS_TOKEN_REFRESH_WINDOW_MS;
    }
    if let Some(last) = auth.record.get("last_refresh").and_then(Value::as_str) {
        if let Some(ms) = iso_to_ms(last) {
            return now - ms > LAST_REFRESH_MAX_AGE_MS;
        }
    }
    false
}

/// Serializes refresh attempts per auth.json path, so concurrent fetches
/// cannot race into redeeming the same single-use token twice.
static REFRESH_LOCKS: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(Vec::new()));

fn write_json_atomic(path: &std::path::Path, value: &Value) -> Result<(), WsErr> {
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(value).map_err(|e| err(&e.to_string()))?;
    std::fs::write(&tmp, text).map_err(|e| err(&format!("auth write: {e}")))?;
    std::fs::rename(&tmp, path).map_err(|e| err(&format!("auth rename: {e}")))?;
    Ok(())
}

fn http_form_post(url: &str, fields: &[(&str, &str)]) -> Result<(u16, Value), WsErr> {
    let url_owned = url.to_string();
    let fields_owned: Vec<(String, String)> = fields.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    std::thread::scope(|scope| {
        scope
            .spawn(move || -> Result<(u16, Value), String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|e| e.to_string())?;
                let response = client
                    .post(&url_owned)
                    .form(&fields_owned)
                    .send()
                    .map_err(|e| e.to_string())?;
                let status = response.status().as_u16();
                let body: Value = serde_json::from_slice(&response.bytes().map_err(|e| e.to_string())?).unwrap_or(json!({}));
                Ok((status, body))
            })
            .join()
            .map_err(|_| err("refresh thread panicked"))?
            .map_err(|e| err(&e))
    })
}

fn http_get_json(url: &str, auth: &CodexOAuth) -> Result<(u16, Value, std::collections::HashMap<String, String>), WsErr> {
    let url_owned = url.to_string();
    let token = auth.access_token.clone();
    let account = auth.account_id.clone();
    std::thread::scope(|scope| {
        scope
            .spawn(move || -> Result<(u16, Value, std::collections::HashMap<String, String>), String> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .redirect(reqwest::redirect::Policy::none())
                    .build()
                    .map_err(|e| e.to_string())?;
                let mut req = client
                    .get(&url_owned)
                    .header("Authorization", format!("Bearer {token}"))
                    .header("Accept", "application/json")
                    .header("User-Agent", "Synara");
                if let Some(account) = account {
                    req = req.header("ChatGPT-Account-Id", account);
                }
                let response = req.send().map_err(|e| e.to_string())?;
                let status = response.status().as_u16();
                let headers: std::collections::HashMap<String, String> = response
                    .headers()
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.as_str().to_lowercase(), v.to_string())))
                    .collect();
                let body: Value = serde_json::from_slice(&response.bytes().map_err(|e| e.to_string())?).unwrap_or(json!({}));
                Ok((status, body, headers))
            })
            .join()
            .map_err(|_| err("usage fetch thread panicked"))?
            .map_err(|e| err(&e))
    })
}

enum RefreshOutcome {
    Updated(CodexOAuth, bool /* redeemed */),
    NeedsAuth,
    Unavailable,
}

fn refresh_auth(auth: &CodexOAuth, allow_redeem: bool, now: i64) -> RefreshOutcome {
    let Some(path) = auth.source_path.clone() else {
        // Keychain-sourced: adopt an out-of-band rotation, never redeem.
        if let Some(live) = resolve_codex_auth() {
            if live.access_token != auth.access_token {
                return RefreshOutcome::Updated(live, false);
            }
        }
        return RefreshOutcome::Unavailable;
    };
    // Per-path serialization: hold the lock key for the whole critical section.
    let key = path.display().to_string();
    {
        let mut locks = REFRESH_LOCKS.lock().unwrap_or_else(|e| e.into_inner());
        if locks.contains(&key) {
            // Another thread is mid-rotation: adopt whatever lands in the file.
        } else {
            locks.push(key.clone());
        }
    }
    let guard = scopeguard(key.clone());
    let live = read_auth_file(&path)
        .and_then(|record| parse_auth_record(&record, Some(path.clone())))
        .unwrap_or_else(|| auth.clone());
    if live.access_token != auth.access_token && !needs_refresh(&live, now) {
        drop(guard);
        return RefreshOutcome::Updated(live, false);
    }
    if !allow_redeem {
        drop(guard);
        return if live.access_token != auth.access_token {
            RefreshOutcome::Updated(live, false)
        } else {
            RefreshOutcome::NeedsAuth
        };
    }
    let Some(refresh_token) = live.refresh_token.clone() else {
        drop(guard);
        return RefreshOutcome::NeedsAuth;
    };
    let outcome = http_form_post(
        REFRESH_URL,
        &[
            ("grant_type", "refresh_token"),
            ("client_id", OAUTH_CLIENT_ID),
            ("refresh_token", &refresh_token),
        ],
    );
    drop(guard);
    let Ok((status, body)) = outcome else {
        return RefreshOutcome::Unavailable;
    };
    if status == 200 {
        let Some(access) = body.get("access_token").and_then(Value::as_str) else {
            return RefreshOutcome::Unavailable;
        };
        let new_refresh = body.get("refresh_token").and_then(Value::as_str).map(str::to_string);
        let new_id = body.get("id_token").and_then(Value::as_str).map(str::to_string);
        // Persist the rotation atomically, preserving unknown fields.
        let mut record = live.record.clone();
        let mut tokens = record.get("tokens").cloned().unwrap_or(json!({}));
        if let Some(obj) = tokens.as_object_mut() {
            obj.insert("access_token".to_string(), json!(access));
            if let Some(r) = &new_refresh {
                obj.insert("refresh_token".to_string(), json!(r));
            }
            if let Some(i) = &new_id {
                obj.insert("id_token".to_string(), json!(i));
            }
        }
        if let Some(obj) = record.as_object_mut() {
            obj.insert("tokens".to_string(), tokens);
            let iso = ms_to_iso(now);
            obj.insert("last_refresh".to_string(), json!(iso));
        }
        // Persistence failure is loud but non-fatal: the fresh token still
        // works for this pass; a stranded rotation is the real loss.
        let _ = write_json_atomic(&path, &record);
        return RefreshOutcome::Updated(
            CodexOAuth {
                record,
                access_token: access.to_string(),
                refresh_token: new_refresh.or(live.refresh_token),
                account_id: live.account_id,
                source_path: Some(path),
            },
            true,
        );
    }
    let error_code = body.get("error").and_then(|e| {
        e.as_str().map(str::to_string).or_else(|| e.get("code").and_then(Value::as_str).map(str::to_string))
    });
    match error_code.as_deref() {
        Some("refresh_token_expired") | Some("refresh_token_invalidated") => RefreshOutcome::NeedsAuth,
        Some("refresh_token_reused") => {
            // Already redeemed elsewhere: the file likely holds newer credentials.
            if let Some(fresh) = read_auth_file(&path).and_then(|r| parse_auth_record(&r, Some(path.clone()))) {
                if fresh.access_token != auth.access_token {
                    return RefreshOutcome::Updated(fresh, false);
                }
            }
            RefreshOutcome::NeedsAuth
        }
        _ => RefreshOutcome::Unavailable,
    }
}

fn scopeguard(key: String) -> impl Drop {
    struct Guard(String);
    impl Drop for Guard {
        fn drop(&mut self) {
            if let Ok(mut locks) = REFRESH_LOCKS.lock() {
                locks.retain(|k| k != &self.0);
            }
        }
    }
    Guard(key)
}

fn ms_to_iso(ms: i64) -> String {
    let secs = ms / 1000;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    // civil-from-days (Howard Hinnant)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, m, d,
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

fn clamp_percent(v: Option<f64>) -> Option<f64> {
    v.filter(|n| n.is_finite() && *n >= 0.0).map(|n| n.min(100.0))
}

fn reset_from_window(window: &Value, now: i64) -> Option<String> {
    if let Some(explicit) = window.get("reset_at").and_then(Value::as_str) {
        return Some(explicit.to_string());
    }
    if let Some(at_ms) = window.get("reset_at_ms").and_then(Value::as_i64) {
        return Some(ms_to_iso(at_ms));
    }
    if let Some(after) = window.get("reset_after_seconds").and_then(Value::as_i64) {
        if after > 0 {
            return Some(ms_to_iso(now + after * 1000));
        }
    }
    None
}

/// Port of upstream `parseCodexUsage`: shape the wham/usage response into
/// the shared snapshot's limits + usage lines.
pub fn parse_codex_usage(body: &Value, headers: &std::collections::HashMap<String, String>, now: i64) -> (Vec<Value>, Vec<Value>, Option<String>) {
    let rate_limit = body.get("rate_limit");
    let mut limits: Vec<Value> = Vec::new();
    let mut usage_lines: Vec<Value> = Vec::new();
    let mut push_window = |label: &str, window_value: Option<&Value>, header_name: &str, fallback_mins: i64| {
        let Some(window) = window_value else { return };
        if !window.is_object() {
            return;
        }
        let used = clamp_percent(headers.get(header_name).and_then(|v| v.parse::<f64>().ok()))
            .or_else(|| clamp_percent(window.get("used_percent").and_then(Value::as_f64)));
        let resets = reset_from_window(window, now);
        let window_mins = window
            .get("limit_window_seconds")
            .and_then(Value::as_i64)
            .map(|secs| secs / 60)
            .unwrap_or(fallback_mins);
        if used.is_none() && resets.is_none() {
            return;
        }
        let mut entry = json!({ "window": label, "windowDurationMins": window_mins });
        if let Some(u) = used {
            entry["usedPercent"] = json!(u);
        }
        if let Some(r) = resets {
            entry["resetsAt"] = json!(r);
        }
        limits.push(entry);
    };
    push_window("5h", rate_limit.and_then(|r| r.get("primary_window")), "x-codex-primary-used-percent", 300);
    push_window("Weekly", rate_limit.and_then(|r| r.get("secondary_window")), "x-codex-secondary-used-percent", 10_080);
    let credits = body.get("credits");
    let balance = headers
        .get("x-codex-credits-balance")
        .and_then(|v| v.parse::<f64>().ok())
        .or_else(|| credits.and_then(|c| c.get("balance")).and_then(Value::as_f64));
    let has_credits = credits.and_then(|c| c.get("has_credits")).and_then(Value::as_bool);
    if let Some(balance) = balance {
        if has_credits != Some(false) || balance > 0.0 {
            let estimate = balance * CREDIT_LIST_PRICE_USD;
            usage_lines.push(json!({
                "label": "Credits",
                "value": format!("{balance:.2} remaining (≈ ${estimate:.2})"),
            }));
        }
    }
    let plan = body.get("plan_type").and_then(Value::as_str).map(|p| {
        let mut chars = p.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
        }
    });
    (limits, usage_lines, plan)
}

fn is_auth_failure(status: u16) -> bool {
    status == 401 || status == 403
}

/// Live usage fetch with the full rotation discipline. Returns the shaped
/// snapshot body or an error string; the caller combines it with passive
/// evidence exactly like upstream does.
pub fn fetch_live_codex_usage() -> Result<Value, WsErr> {
    let now = now_ms();
    let mut auth = resolve_codex_auth().ok_or_else(|| err("codex auth required"))?;
    let mut allow_redeem = true;
    if needs_refresh(&auth, now) {
        match refresh_auth(&auth, allow_redeem, now) {
            RefreshOutcome::Updated(updated, redeemed) => {
                allow_redeem = allow_redeem && !redeemed;
                auth = updated;
            }
            RefreshOutcome::NeedsAuth => return Err(err("codex auth required")),
            RefreshOutcome::Unavailable => {}
        }
    }
    let (mut status, mut body, mut headers) = http_get_json(USAGE_URL, &auth)?;
    if is_auth_failure(status) {
        // Stored expiry can lag reality: refresh/re-read and retry once with
        // a genuinely different token — a second redemption can only burn.
        if let RefreshOutcome::Updated(updated, _) = refresh_auth(&auth, allow_redeem, now) {
            if updated.access_token != auth.access_token {
                auth = updated;
                let retry = http_get_json(USAGE_URL, &auth)?;
                status = retry.0;
                body = retry.1;
                headers = retry.2;
            }
        }
    }
    if status != 200 {
        return Err(err(&format!("codex usage unavailable ({status})")));
    }
    let (limits, usage_lines, plan) = parse_codex_usage(&body, &headers, now);
    Ok(json!({
        "limits": limits,
        "usageLines": usage_lines,
        "planName": plan,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_parsing_matches_upstream_shape() {
        let body = json!({
            "rate_limit": {
                "primary_window": {"used_percent": 42.0, "limit_window_seconds": 18000, "reset_after_seconds": 3600},
                "secondary_window": {"used_percent": 7.5, "limit_window_seconds": 604800},
            },
            "credits": {"has_credits": true, "balance": 125.0},
            "plan_type": "plus",
        });
        let headers = std::collections::HashMap::new();
        let (limits, lines, plan) = parse_codex_usage(&body, &headers, 1_000_000);
        assert_eq!(limits.len(), 2);
        assert_eq!(limits[0]["window"], "5h");
        assert_eq!(limits[0]["usedPercent"], 42.0);
        assert_eq!(limits[0]["windowDurationMins"], 300);
        assert!(limits[0]["resetsAt"].as_str().unwrap().ends_with('Z'));
        assert_eq!(limits[1]["window"], "Weekly");
        assert_eq!(lines.len(), 1);
        assert!(lines[0]["value"].as_str().unwrap().contains("125.00"));
        assert_eq!(plan.as_deref(), Some("Plus"));
    }

    #[test]
    fn header_values_win_over_body() {
        let body = json!({
            "rate_limit": {"primary_window": {"used_percent": 10.0}},
            "credits": {"balance": 50.0},
        });
        let mut headers = std::collections::HashMap::new();
        headers.insert("x-codex-primary-used-percent".to_string(), "77.0".to_string());
        headers.insert("x-codex-credits-balance".to_string(), "99.0".to_string());
        let (limits, lines, _) = parse_codex_usage(&body, &headers, 0);
        assert_eq!(limits[0]["usedPercent"], 77.0);
        assert!(lines[0]["value"].as_str().unwrap().contains("99.00"));
    }

    #[test]
    fn jwt_exp_decode_works() {
        use base64::Engine;
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(br#"{"exp": 1700000000}"#);
        let token = format!("header.{payload}.sig");
        assert_eq!(decode_jwt_exp_ms(&token), Some(1_700_000_000_000));
    }

    #[test]
    fn iso_roundtrip() {
        let ms = 1_700_000_000_000i64;
        let iso = ms_to_iso(ms);
        assert!(iso.starts_with("2023-11-14T"));
        assert_eq!(iso_to_ms(&iso), Some(ms - ms % 1000));
    }
}
