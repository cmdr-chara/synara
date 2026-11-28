//! Provider-owned quota evidence, ported from upstream `provider/usageLimit.ts`.
//! Unknown or stale reset times stay manual: a window only auto-resumes when
//! every observed window carries a valid future reset timestamp.

use serde_json::{Value, json};

fn parse_reset_at(value: &Value, now_ms: i64) -> Option<String> {
    let ms = if let Some(secs) = value.as_f64() {
        (secs * 1_000.0) as i64
    } else if let Some(text) = value.as_str() {
        if text.len() < 10 || !text.bytes().take(4).all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Millisecond-precision ISO timestamps sort and compare correctly as
        // strings; validate loosely and keep the original text.
        return if text.contains('T') {
            Some(text.to_string())
        } else {
            None
        };
    } else {
        return None;
    };
    if ms > now_ms && ms <= 8_640_000_000_000_000 {
        Some(crate::todo::iso_from_ms(ms))
    } else {
        None
    }
}

/// Latest reset across the observed windows, or None when evidence is
/// incomplete/stale or the account has credit headroom — those stay manual.
pub fn usage_limit_reset_at(evidence: &Value, _now_ms: i64) -> Option<String> {
    let credits = evidence.get("codexCredits");
    if evidence.get("codexAllowed").and_then(Value::as_bool) == Some(true)
        || credits
            .and_then(|c| c.get("hasCredits"))
            .and_then(Value::as_bool)
            == Some(true)
        || credits
            .and_then(|c| c.get("unlimited"))
            .and_then(Value::as_bool)
            == Some(true)
    {
        return None;
    }
    let windows = evidence.get("windows")?.as_object()?;
    let mut latest: Option<String> = None;
    for value in windows.values() {
        if value.is_null() {
            return None;
        }
        let reset = value.as_str()?.to_string();
        if latest.as_deref().is_none_or(|l| reset.as_str() > l) {
            latest = Some(reset);
        }
    }
    latest
}

const CLAUDE_KNOWN_WINDOWS: &[&str] = &[
    "five_hour",
    "seven_day",
    "seven_day_opus",
    "seven_day_sonnet",
    "seven_day_overage_included",
    "overage",
];

/// Merge one Claude `rate_limit_info` payload into the accumulated windows,
/// mirroring upstream claudeUsageLimitWindows.
pub fn claude_usage_limit_windows(previous: &Value, value: &Value, now_ms: i64) -> Option<Value> {
    let root = value.as_object()?;
    let info = root
        .get("rate_limit_info")
        .and_then(Value::as_object)
        .map(|m| m.clone())
        .unwrap_or_else(|| root.clone());
    let str_field = |k: &str| info.get(k).and_then(Value::as_str);
    let bool_field = |k: &str| info.get(k).and_then(Value::as_bool) == Some(true);
    // Account-wide overage bypass clears every window.
    if str_field("overageStatus") == Some("allowed")
        || str_field("overageStatus") == Some("allowed_warning")
        || bool_field("isUsingOverage")
        || bool_field("overageInUse")
    {
        return Some(json!({"windows": {}}));
    }
    let mut windows = previous
        .get("windows")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    match str_field("status") {
        Some("allowed") | Some("allowed_warning") => {
            let Some(kind) = str_field("rateLimitType") else {
                return Some(json!({"windows": {}}));
            };
            windows.remove(kind);
            return Some(json!({"windows": windows}));
        }
        Some("rejected") => {}
        _ => return None,
    }
    let kind = str_field("rateLimitType").unwrap_or("");
    let name = if CLAUDE_KNOWN_WINDOWS.contains(&kind) {
        kind
    } else {
        "unknown"
    };
    let manual = str_field("errorCode") == Some("credits_required")
        || (name == "overage" && str_field("overageDisabledReason") == Some("out_of_credits"));
    let reset = if manual {
        None
    } else {
        let raw = if name == "overage" {
            info.get("overageResetsAt").or_else(|| info.get("resetsAt"))
        } else {
            info.get("resetsAt")
        };
        raw.and_then(|v| parse_reset_at(v, now_ms))
    };
    match reset {
        Some(r) => {
            windows.insert(name.to_string(), json!(r));
        }
        None => {
            windows.insert(name.to_string(), Value::Null);
        }
    }
    Some(json!({"windows": windows}))
}

/// Merge one Codex `account/rateLimits/updated` snapshot into the
/// accumulated windows, mirroring upstream codexUsageLimitWindows: only
/// windows at 100% with a valid future reset keep a timestamp; everything
/// else clears or stays manual.
pub fn codex_usage_limit_windows(previous: &Value, value: &Value, now_ms: i64) -> Option<Value> {
    let root = value.as_object()?;
    let by_id = root.get("rateLimitsByLimitId").and_then(Value::as_object);
    let snapshot = if let Some(by_id) = by_id {
        if !by_id.contains_key("codex") {
            return Some(previous.clone());
        }
        by_id.get("codex").and_then(Value::as_object).cloned()
    } else if root.contains_key("rateLimits") {
        root.get("rateLimits").and_then(Value::as_object).cloned()
    } else {
        Some(root.clone())
    };
    let snapshot = snapshot?;
    if snapshot
        .get("limitId")
        .is_some_and(|l| l.as_str() != Some("codex"))
    {
        return Some(previous.clone());
    }
    let mut windows = previous
        .get("windows")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let mut observed = previous.get("windows").is_some();
    let mut out = json!({"windows": Value::Null});
    for name in ["primary", "secondary"] {
        if !snapshot.contains_key(name) {
            continue;
        }
        observed = true;
        if snapshot.get(name).is_some_and(Value::is_null) {
            windows.remove(name);
            continue;
        }
        let Some(window) = snapshot.get(name).and_then(Value::as_object) else {
            if windows.contains_key(name) {
                windows.insert(name.to_string(), Value::Null);
            }
            continue;
        };
        if let Some(percent) = window.get("usedPercent").and_then(Value::as_f64) {
            if percent < 100.0 {
                windows.remove(name);
            } else {
                let reset = window
                    .get("resetsAt")
                    .and_then(|v| parse_reset_at(v, now_ms))
                    .or_else(|| {
                        windows
                            .get(name)
                            .and_then(Value::as_str)
                            .map(str::to_string)
                    });
                match reset {
                    Some(r) => windows.insert(name.to_string(), json!(r)),
                    None => windows.insert(name.to_string(), Value::Null),
                };
            }
        } else if window.contains_key("resetsAt") && windows.contains_key(name) {
            if let Some(r) = window
                .get("resetsAt")
                .and_then(|v| parse_reset_at(v, now_ms))
            {
                windows.insert(name.to_string(), json!(r));
            }
        }
    }
    if let Some(credits) = snapshot.get("credits").and_then(Value::as_object) {
        observed = true;
        let mut c = serde_json::Map::new();
        if let Some(h) = credits.get("hasCredits").and_then(Value::as_bool) {
            c.insert("hasCredits".into(), json!(h));
        }
        if let Some(u) = credits.get("unlimited").and_then(Value::as_bool) {
            c.insert("unlimited".into(), json!(u));
        }
        if !c.is_empty() {
            out["codexCredits"] = Value::Object(c);
        }
    }
    if let Some(allowed) = snapshot.get("allowed").and_then(Value::as_bool) {
        observed = true;
        out["codexAllowed"] = json!(allowed);
    }
    if !observed {
        return None;
    }
    out["windows"] = Value::Object(windows);
    Some(out)
}

/// Codex error classification, mirroring upstream isCodexUsageLimitError.
pub fn is_codex_usage_limit_error(code: Option<&str>, value: &Value) -> bool {
    if matches!(code, Some("usageLimitExceeded") | Some("rateLimitExceeded")) {
        return true;
    }
    if code != Some("responseTooManyFailedAttempts") {
        return false;
    }
    let Some(error) = value.as_object() else {
        return false;
    };
    let info = error
        .get("codexErrorInfo")
        .or_else(|| error.get("codex_error_info"))
        .and_then(Value::as_object);
    let Some(info) = info else { return false };
    let attempts = info
        .get("responseTooManyFailedAttempts")
        .and_then(Value::as_object);
    attempts
        .and_then(|a| a.get("httpStatusCode"))
        .and_then(Value::as_i64)
        == Some(429)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_rejected_five_hour_records_reset() {
        let now = 1_000_000i64;
        let evidence = claude_usage_limit_windows(
            &json!({"windows": {}}),
            &json!({"status":"rejected","rateLimitType":"five_hour","resetsAt":"2026-11-25T15:00:00.000Z"}),
            now,
        )
        .unwrap();
        assert_eq!(
            evidence["windows"]["five_hour"],
            json!("2026-11-25T15:00:00.000Z")
        );
    }

    #[test]
    fn claude_overage_bypass_clears_windows() {
        let previous = json!({"windows": {"five_hour": "x"}});
        let evidence = claude_usage_limit_windows(
            &previous,
            &json!({"status":"rejected","rateLimitType":"five_hour","isUsingOverage":true}),
            0,
        )
        .unwrap();
        assert_eq!(evidence["windows"].as_object().unwrap().len(), 0);
    }

    #[test]
    fn claude_allowed_clears_only_that_window() {
        let previous = json!({"windows": {"five_hour": "a", "seven_day": "b"}});
        let evidence = claude_usage_limit_windows(
            &previous,
            &json!({"status":"allowed","rateLimitType":"five_hour"}),
            0,
        )
        .unwrap();
        let windows = evidence["windows"].as_object().unwrap();
        assert!(!windows.contains_key("five_hour"));
        assert!(windows.contains_key("seven_day"));
    }

    #[test]
    fn claude_credits_required_stays_manual() {
        let evidence = claude_usage_limit_windows(
            &json!({"windows": {}}),
            &json!({"status":"rejected","rateLimitType":"overage","errorCode":"credits_required"}),
            0,
        )
        .unwrap();
        assert!(evidence["windows"]["overage"].is_null());
    }

    #[test]
    fn reset_at_none_when_any_window_unknown() {
        let evidence = json!({"windows": {"five_hour": null}});
        assert_eq!(usage_limit_reset_at(&evidence, 0), None);
    }

    #[test]
    fn reset_at_latest_window() {
        let evidence = json!({"windows": {"five_hour": "2026-11-25T15:00:00.000Z", "seven_day": "2026-11-28T15:00:00.000Z"}});
        assert_eq!(
            usage_limit_reset_at(&evidence, 0).as_deref(),
            Some("2026-11-28T15:00:00.000Z")
        );
    }

    #[test]
    fn codex_rate_limit_codes() {
        assert!(is_codex_usage_limit_error(
            Some("usageLimitExceeded"),
            &json!({})
        ));
        assert!(is_codex_usage_limit_error(
            Some("rateLimitExceeded"),
            &json!({})
        ));
        assert!(!is_codex_usage_limit_error(
            Some("serverOverloaded"),
            &json!({})
        ));
        let too_many =
            json!({"codexErrorInfo":{"responseTooManyFailedAttempts":{"httpStatusCode":429}}});
        assert!(is_codex_usage_limit_error(
            Some("responseTooManyFailedAttempts"),
            &too_many
        ));
        let other =
            json!({"codexErrorInfo":{"responseTooManyFailedAttempts":{"httpStatusCode":500}}});
        assert!(!is_codex_usage_limit_error(
            Some("responseTooManyFailedAttempts"),
            &other
        ));
    }
}
