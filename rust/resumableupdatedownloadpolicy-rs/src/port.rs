//! The Rust port of `apps/desktop/src/resumableUpdateDownloadPolicy.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value};

fn member(v: &Value, key: &str) -> Value {
    v.as_object()
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or_else(js::undefined)
}

fn num(v: &Value) -> f64 {
    if let Some(n) = js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::String(s) => js_parse_number(s.trim()),
        _ => f64::NAN,
    }
}

fn js_parse_number(s: &str) -> f64 {
    match s {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => {
            let t = s.trim();
            if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                return u64::from_str_radix(hex, 16)
                    .map(|v| v as f64)
                    .unwrap_or(f64::NAN);
            }
            t.parse::<f64>().unwrap_or(f64::NAN)
        }
    }
}

fn js_gt(a: f64, b: f64) -> bool {
    !a.is_nan() && !b.is_nan() && a > b
}

fn strict_eq(a: &Value, b: f64) -> bool {
    js::as_number(a).is_some_and(|n| n == b)
}

fn js_num_out(n: f64) -> Value {
    js::number(n)
}

/// Port of `computeProgressInfo` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:42`).
pub fn compute_progress_info(args: serde_json::Value) -> Result<serde_json::Value, String> {
    if args.is_null() {
        return Err("Cannot read properties of null (reading 'elapsedMs')".to_string());
    }
    if js::is_undefined(&args) {
        return Err("Cannot read properties of undefined (reading 'elapsedMs')".to_string());
    }
    let elapsed_ms = member(&args, "elapsedMs");
    let elapsed_seconds = if js_gt(num(&elapsed_ms), 0.0) {
        num(&elapsed_ms) / 1000.0
    } else {
        0.001
    };
    let total = member(&args, "total");
    let delta = member(&args, "delta");
    let transferred = member(&args, "transferred");
    let percent = if js_gt(num(&total), 0.0) {
        (num(&transferred) / num(&total)) * 100.0
    } else {
        0.0
    };
    let bytes_per_second = js::math_round(num(&transferred) / elapsed_seconds);
    let mut out = Map::new();
    out.insert("total".to_string(), total);
    out.insert("delta".to_string(), delta);
    out.insert("transferred".to_string(), transferred);
    out.insert("percent".to_string(), js_num_out(percent));
    out.insert("bytesPerSecond".to_string(), js_num_out(bytes_per_second));
    Ok(Value::Object(out))
}

fn js_ws(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// Port of `parseContentRangeTotal` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:59`).
pub fn parse_content_range_total(header_value: Option<String>) -> Result<Option<f64>, String> {
    let Some(header) = header_value.filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    // /\/\s*(\d+)\s*$/
    let trimmed = header.trim_end_matches(js_ws);
    let digits_start = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    if digits_start == trimmed.len() {
        return Ok(None);
    }
    let before = trimmed[..digits_start].trim_end_matches(js_ws);
    if !before.ends_with('/') {
        return Ok(None);
    }
    let total: f64 = trimmed[digits_start..].parse().unwrap_or(f64::NAN);
    if total.is_finite() && total > 0.0 {
        Ok(Some(total))
    } else {
        Ok(None)
    }
}

/// Port of `selectSha512Encoding` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:73`).
pub fn select_sha512_encoding(sha512: String) -> Result<String, String> {
    let utf16_len: usize = sha512.chars().map(|c| c.len_utf16()).sum();
    if utf16_len == 128 && !sha512.contains('+') && !sha512.contains('Z') && !sha512.contains('=') {
        Ok("hex".to_string())
    } else {
        Ok("base64".to_string())
    }
}

/// Port of `classifyDownloadResponse` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:94`).
pub fn classify_download_response(args: serde_json::Value) -> Result<serde_json::Value, String> {
    if args.is_null() {
        return Err(
            "Cannot destructure property 'statusCode' of 'args' as it is null.".to_string(),
        );
    }
    if js::is_undefined(&args) {
        return Err(
            "Cannot destructure property 'statusCode' of 'args' as it is undefined.".to_string(),
        );
    }
    let status = member(&args, "statusCode");
    let content_range = member(&args, "contentRange");
    let content_length = member(&args, "contentLength");
    let already = member(&args, "bytesAlreadyDownloaded");

    let mut out = Map::new();
    if strict_eq(&status, 206.0) {
        let total = match content_range.as_str() {
            Some(s) => parse_content_range_total(Some(s.to_string()))?,
            None => None,
        };
        let total = match total {
            Some(t) => js_num_out(t),
            None => {
                if js::is_undefined(&content_length) || content_length.is_null() {
                    Value::Null
                } else {
                    js_num_out(num(&already) + num(&content_length))
                }
            }
        };
        out.insert("kind".to_string(), Value::String("append".to_string()));
        out.insert("total".to_string(), total);
    } else if strict_eq(&status, 200.0) {
        out.insert("kind".to_string(), Value::String("fromStart".to_string()));
        out.insert("total".to_string(), content_length);
    } else if strict_eq(&status, 416.0) {
        out.insert("kind".to_string(), Value::String("complete".to_string()));
    } else {
        let n = js::as_number(&status);
        if strict_eq(&status, 429.0) || n.is_some_and(|v| (500.0..=599.0).contains(&v)) {
            out.insert("kind".to_string(), Value::String("retryable".to_string()));
            out.insert("statusCode".to_string(), status);
        } else {
            out.insert("kind".to_string(), Value::String("fatal".to_string()));
            out.insert("statusCode".to_string(), status);
        }
    }
    Ok(Value::Object(out))
}

/// Port of `computeRetryDelayMs` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:119`).
pub fn compute_retry_delay_ms(
    consecutive_stall_count: f64,
    config: serde_json::Value,
) -> Result<f64, String> {
    if consecutive_stall_count <= 1.0 {
        return Ok(0.0);
    }
    if config.is_null() {
        return Err("Cannot read properties of null (reading 'retryBaseDelayMs')".to_string());
    }
    if js::is_undefined(&config) {
        return Err("Cannot read properties of undefined (reading 'retryBaseDelayMs')".to_string());
    }
    let base = num(&member(&config, "retryBaseDelayMs"));
    let max = num(&member(&config, "retryMaxDelayMs"));
    let delay = base * 2f64.powf(consecutive_stall_count - 2.0);
    // Math.min preserves NaN.
    if delay.is_nan() || max.is_nan() {
        return Ok(f64::NAN);
    }
    Ok(delay.min(max))
}

/// Port of `shouldGiveUp` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:131`).
pub fn should_give_up(args: serde_json::Value) -> Result<bool, String> {
    if args.is_null() {
        return Err("Cannot read properties of null (reading 'consecutiveStallCount')".to_string());
    }
    if js::is_undefined(&args) {
        return Err(
            "Cannot read properties of undefined (reading 'consecutiveStallCount')".to_string(),
        );
    }
    let stalls = num(&member(&args, "consecutiveStallCount"));
    let config = member(&args, "config");
    if config.is_null() {
        return Err(
            "Cannot read properties of null (reading 'maxConsecutiveStallRetries')".to_string(),
        );
    }
    if js::is_undefined(&config) {
        return Err(
            "Cannot read properties of undefined (reading 'maxConsecutiveStallRetries')"
                .to_string(),
        );
    }
    let max_stalls = num(&member(&config, "maxConsecutiveStallRetries"));
    let attempts = num(&member(&args, "totalAttempts"));
    let max_attempts = num(&member(&config, "maxTotalAttempts"));
    let elapsed = num(&member(&args, "elapsedMs"));
    let overall_timeout = num(&member(&config, "overallTimeoutMs"));
    Ok(js_gt(stalls, max_stalls)
        || js_gt(attempts, max_attempts)
        || js_gt(elapsed, overall_timeout))
}

fn protocol_of(v: &Value) -> Result<Value, String> {
    if v.is_null() {
        return Err("Cannot read properties of null (reading 'protocol')".to_string());
    }
    if js::is_undefined(v) {
        return Err("Cannot read properties of undefined (reading 'protocol')".to_string());
    }
    Ok(member(v, "protocol"))
}

fn lower_member(v: &Value, path: &str) -> Result<String, String> {
    let h = member(v, "hostname");
    match &h {
        Value::String(s) => Ok(s.to_lowercase()),
        Value::Null => Err("Cannot read properties of null (reading 'toLowerCase')".to_string()),
        _ if js::is_undefined(&h) => {
            Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string())
        }
        _ => Err(format!("{path}.toLowerCase is not a function")),
    }
}

fn effective_port(url: &Value) -> Result<String, String> {
    let port = member(url, "port");
    match &port {
        Value::Null => Err("Cannot read properties of null (reading 'length')".to_string()),
        _ if js::is_undefined(&port) => {
            Err("Cannot read properties of undefined (reading 'length')".to_string())
        }
        Value::String(s) => {
            if !s.is_empty() {
                return Ok(s.clone());
            }
            let proto = member(url, "protocol");
            match proto.as_str() {
                Some("https:") => Ok("443".to_string()),
                Some("http:") => Ok("80".to_string()),
                _ => Ok(String::new()),
            }
        }
        _ => {
            // Non-string port: `.length` is undefined; `undefined > 0` is false.
            let proto = member(url, "protocol");
            match proto.as_str() {
                Some("https:") => Ok("443".to_string()),
                Some("http:") => Ok("80".to_string()),
                _ => Ok(String::new()),
            }
        }
    }
}

/// Port of `isCrossOrigin` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:159`).
pub fn is_cross_origin(a: serde_json::Value, b: serde_json::Value) -> Result<bool, String> {
    let pa = protocol_of(&a)?;
    let pb = protocol_of(&b)?;
    if !loose_strict_eq(&pa, &pb) {
        return Ok(true);
    }
    let ha = lower_member(&a, "a.hostname")?;
    let hb = lower_member(&b, "b.hostname")?;
    if ha != hb {
        return Ok(true);
    }
    Ok(effective_port(&a)? != effective_port(&b)?)
}

fn loose_strict_eq(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) && js::is_undefined(b) {
        return true;
    }
    match (js::as_number(a), js::as_number(b)) {
        (Some(x), Some(y)) => return x == y,
        (Some(_), None) | (None, Some(_)) => return false,
        _ => {}
    }
    match (a, b) {
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !js::is_undefined(v),
    }
}

/// Port of `buildDownloadHeaders` (`apps/desktop/src/resumableUpdateDownloadPolicy.ts:170`).
pub fn build_download_headers(args: serde_json::Value) -> Result<serde_json::Value, String> {
    if args.is_null() {
        return Err("Cannot read properties of null (reading 'callHeaders')".to_string());
    }
    if js::is_undefined(&args) {
        return Err("Cannot read properties of undefined (reading 'callHeaders')".to_string());
    }
    let call_headers = member(&args, "callHeaders");
    let attach_auth = member(&args, "attachAuth");
    let start_offset = member(&args, "startOffset");

    let mut headers = Map::new();
    if let Value::Object(entries) = &call_headers
        && !js::is_undefined(&call_headers)
    {
        for (key, value) in entries {
            let lower = key.to_lowercase();
            if !truthy(&attach_auth) && (lower == "authorization" || lower == "proxy-authorization")
            {
                continue;
            }
            headers.insert(key.clone(), value.clone());
        }
    }
    if !headers.contains_key("User-Agent") {
        headers.insert(
            "User-Agent".to_string(),
            Value::String("electron-builder".to_string()),
        );
    }
    headers.insert(
        "Cache-Control".to_string(),
        Value::String("no-cache".to_string()),
    );
    if js_gt(num(&start_offset), 0.0) {
        headers.insert(
            "Range".to_string(),
            Value::String(format!(
                "bytes={}-",
                js::number_to_string(num(&start_offset))
            )),
        );
    }
    Ok(Value::Object(headers))
}
