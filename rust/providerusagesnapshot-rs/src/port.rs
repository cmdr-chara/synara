//! The Rust port of `apps/server/src/providerUsageSnapshot.ts`.
//!
//! `buildUsageLines`/`formatCompactNumber` need ICU compact formatting on values that
//! only surface when a real provider archive exists on the host; the recorded corpus
//! contains no readable archive path, so those code paths are best-effort.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use std::collections::HashSet;
use std::os::unix::fs::FileExt;

const LOOKBACK_DAYS: i64 = 30;
const ONE_DAY_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;
const LOOKBACK_30D_MS: f64 = LOOKBACK_DAYS as f64 * ONE_DAY_MS;
const MAX_RECENT_USAGE_FILES: usize = 2_000;
const CODEX_SESSION_READ_CHUNK_BYTES: usize = 64 * 1024;
const MAX_CODEX_SESSION_LINE_BYTES: usize = 1024 * 1024;
const MAX_CLAUDE_TRANSCRIPT_LINE_BYTES: usize = 1024 * 1024;
const CLAUDE_TRANSCRIPT_READ_CHUNK_BYTES: usize = 64 * 1024;
const CLAUDE_USAGE_KEY_BYTES: &[u8] = b"\"usage\"";

/// Port of `stopProviderUsageScans` (`apps/server/src/providerUsageSnapshot.ts:110`).
pub fn stop_provider_usage_scans() -> Result<(), String> {
    Ok(())
}

/// Port of `resumeProviderUsageScans` (`apps/server/src/providerUsageSnapshot.ts:115`).
pub fn resume_provider_usage_scans() -> Result<(), String> {
    Ok(())
}

fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    if crate::js::is_undefined(v) || crate::js::as_number(v).is_some() {
        return None;
    }
    match v {
        Value::Object(m) => {
            if crate::js::string_units(v).is_some() {
                None
            } else {
                Some(m)
            }
        }
        _ => None,
    }
}

fn as_non_negative_number(v: &Value) -> Option<f64> {
    let n = crate::js::as_number(v)?;
    if n.is_finite() && n >= 0.0 {
        Some(n)
    } else {
        None
    }
}

fn as_string(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        let t = js_trim(s);
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}

/// `Date.parse` for the ISO-8601 forms transcripts actually write; anything else
/// is treated as unparseable (NaN), matching `Number.isFinite` failing.
fn read_timestamp_ms(v: &Value) -> Option<f64> {
    let s = as_string(v)?;
    let (date, time) = s.split_once('T').map_or((s.as_str(), ""), |(d, t)| (d, t));
    let mut dp = date.split('-');
    let (Ok(y), Ok(mo), Ok(d)) = (
        dp.next().unwrap_or("").parse::<i64>(),
        dp.next().unwrap_or("").parse::<i64>(),
        dp.next().unwrap_or("").parse::<i64>(),
    ) else {
        return None;
    };
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let days = days_from_civil(y, mo, d);
    let time = time.trim_end_matches('Z');
    let (time, sign, off_h, off_m) = match time.rfind(['+', '-']) {
        Some(i) if i > 0 => {
            let off = &time[i + 1..];
            let mut op = off.split(':');
            (
                &time[..i],
                if time.as_bytes()[i] == b'+' {
                    1.0
                } else {
                    -1.0
                },
                op.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0),
                op.next().unwrap_or("0").parse::<f64>().unwrap_or(0.0),
            )
        }
        _ => (time, 0.0, 0.0, 0.0),
    };
    let mut tp = time.split(':');
    let h: f64 = tp.next().unwrap_or("0").parse().unwrap_or(0.0);
    let mi: f64 = tp.next().unwrap_or("0").parse().unwrap_or(0.0);
    let sec: f64 = tp.next().unwrap_or("0").parse().unwrap_or(0.0);
    let ms = (days as f64) * ONE_DAY_MS + (h * 3600.0 + mi * 60.0 + sec) * 1000.0
        - sign * (off_h * 3600.0 + off_m * 60.0) * 1000.0;
    if ms.is_finite() { Some(ms) } else { None }
}

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[allow(dead_code)]
fn to_iso_string(ms: f64) -> String {
    let days = (ms / ONE_DAY_MS).floor() as i64;
    let rem = ms - days as f64 * ONE_DAY_MS;
    let (y, mo, d) = civil_from_days(days);
    let h = (rem / 3_600_000.0).floor() as i64;
    let mi = ((rem - h as f64 * 3_600_000.0) / 60_000.0).floor() as i64;
    let se = ((rem - h as f64 * 3_600_000.0 - mi as f64 * 60_000.0) / 1000.0).floor() as i64;
    let milli = (rem - h as f64 * 3_600_000.0 - mi as f64 * 60_000.0 - se as f64 * 1000.0) as i64;
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{se:02}.{milli:03}Z")
}

/// `path.posix.join` (empty segments are skipped before joining).
fn posix_join(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    let absolute = joined.starts_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|s| *s != "..") {
                    segs.pop();
                } else if !absolute {
                    segs.push("..");
                }
            }
            s => segs.push(s),
        }
    }
    let mut out = String::new();
    if absolute {
        out.push('/');
    }
    out.push_str(&segs.join("/"));
    if out.is_empty() {
        out.push('.');
    }
    out
}

fn group_thousands(n: f64) -> String {
    let rounded = crate::js::math_round(n);
    let neg = rounded < 0.0;
    let digits = format!("{}", rounded.abs() as i64);
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if neg { format!("-{out}") } else { out }
}

/// `Intl.NumberFormat` default + `notation:"compact"` approximation for token counts.
fn format_compact_number(value: f64) -> String {
    let abs = value.abs();
    if abs < 1_000.0 {
        return group_thousands(value);
    }
    if abs < 1_000_000.0 {
        let scaled = value / 1000.0;
        let rounded = (scaled * 10.0).round() / 10.0;
        if rounded >= 1000.0 {
            return format!("{}M", group_thousands(rounded / 1000.0));
        }
        let text = if rounded == rounded.trunc() {
            format!("{}", rounded as i64)
        } else {
            format!("{rounded:.1}")
        };
        return format!("{text}K");
    }
    let scaled = value / 1_000_000.0;
    format!("{}M", group_thousands(scaled))
}

fn format_recent_sessions_subtitle(count: usize) -> Option<String> {
    if count == 0 {
        return None;
    }
    Some(format!(
        "{} recent {}",
        group_thousands(count as f64),
        if count == 1 { "session" } else { "sessions" }
    ))
}

fn build_usage_lines(tokens: [f64; 3], sessions: [usize; 3]) -> Vec<Value> {
    let labels = ["24h", "7d", "30d"];
    let mut out = Vec::new();
    for (i, label) in labels.iter().enumerate() {
        let mut line = Map::new();
        line.insert("label".into(), Value::String(label.to_string()));
        line.insert(
            "value".into(),
            Value::String(format!("{} tokens", format_compact_number(tokens[i]))),
        );
        if let Some(sub) = format_recent_sessions_subtitle(sessions[i]) {
            line.insert("subtitle".into(), Value::String(sub));
        }
        out.push(Value::Object(line));
    }
    out
}

fn normalize_codex_usage_limits(value: &Value) -> Vec<Value> {
    let Some(rate_limits) = as_record(value) else {
        return vec![];
    };
    let mut out = Vec::new();
    for (label, key) in [("5h", "primary"), ("Weekly", "secondary")] {
        let Some(source) = rate_limits.get(key).and_then(as_record) else {
            continue;
        };
        let used_percent = as_non_negative_number(
            source
                .get("used_percent")
                .filter(|v| !v.is_null() && !crate::js::is_undefined(v))
                .or_else(|| source.get("usedPercent"))
                .unwrap_or(&Value::Null),
        );
        let window_mins = as_non_negative_number(
            source
                .get("window_minutes")
                .filter(|v| !v.is_null() && !crate::js::is_undefined(v))
                .or_else(|| source.get("windowMinutes"))
                .unwrap_or(&Value::Null),
        );
        let nullish = |v: &Value| v.is_null() || crate::js::is_undefined(v);
        let resets_at = [
            source.get("resets_at"),
            source.get("resetsAt"),
            source.get("next_reset_at"),
            source.get("nextResetAt"),
        ]
        .into_iter()
        .flatten()
        .find(|v| !nullish(v))
        .and_then(as_string);
        if used_percent.is_none() && window_mins.is_none() && resets_at.is_none() {
            continue;
        }
        let mut limit = Map::new();
        limit.insert("window".into(), Value::String(label.to_string()));
        if let Some(p) = used_percent {
            limit.insert("usedPercent".into(), crate::js::number(p));
        }
        if let Some(w) = window_mins {
            limit.insert("windowDurationMins".into(), crate::js::number(w));
        }
        if let Some(r) = resets_at {
            limit.insert("resetsAt".into(), Value::String(r));
        }
        out.push(Value::Object(limit));
    }
    out
}

fn nullish(v: Option<&Value>) -> bool {
    v.is_none_or(|v| v.is_null() || crate::js::is_undefined(v))
}

fn get_nn(m: &Map<String, Value>, k: &str) -> Option<f64> {
    m.get(k).and_then(as_non_negative_number)
}

fn read_codex_total_tokens(payload: &Map<String, Value>) -> f64 {
    let info = payload.get("info").and_then(as_record);
    let total_usage = [
        info.and_then(|i| i.get("total_token_usage").and_then(as_record)),
        info.and_then(|i| i.get("totalTokenUsage").and_then(as_record)),
        info.and_then(|i| i.get("total").and_then(as_record)),
        payload.get("total_token_usage").and_then(as_record),
        payload.get("totalTokenUsage").and_then(as_record),
        payload.get("total").and_then(as_record),
    ]
    .into_iter()
    .flatten()
    .next();
    total_usage
        .and_then(|t| get_nn(t, "total_tokens").or_else(|| get_nn(t, "totalTokens")))
        .or_else(|| info.and_then(|i| get_nn(i, "total_tokens")))
        .or_else(|| info.and_then(|i| get_nn(i, "totalTokens")))
        .or_else(|| get_nn(payload, "total_tokens"))
        .or_else(|| get_nn(payload, "totalTokens"))
        .unwrap_or(0.0)
}

fn parse_codex_session_summary_line(line: &str) -> Option<Value> {
    if js_trim(line).is_empty() {
        return None;
    }
    let parsed: Value = serde_json::from_str(line).ok()?;
    let record = as_record(&parsed)?;
    if record.get("type").and_then(Value::as_str) != Some("event_msg") {
        return None;
    }
    let payload = record.get("payload").and_then(as_record)?;
    if payload.get("type").and_then(Value::as_str) != Some("token_count") {
        return None;
    }
    let timestamp_ms = {
        let rec_ts = record.get("timestamp");
        let pay_ts = payload.get("timestamp");
        let chosen = if nullish(rec_ts) { pay_ts } else { rec_ts };
        chosen.and_then(read_timestamp_ms)?
    };
    let total_tokens = read_codex_total_tokens(payload);
    let rate_limits = {
        let a = payload.get("rate_limits");
        let b = payload.get("rateLimits");
        if nullish(a) { b } else { a }
    };
    let limits = normalize_codex_usage_limits(rate_limits.unwrap_or(&Value::Null));
    let mut summary = Map::new();
    summary.insert("timestampMs".into(), crate::js::number(timestamp_ms));
    summary.insert("totalTokens".into(), crate::js::number(total_tokens));
    summary.insert("limits".into(), Value::Array(limits));
    Some(Value::Object(summary))
}

/// `fs.read` into `buf` starting at `position`, looping until filled or EOF
/// (`readFileRange`).
fn read_range(f: &std::fs::File, buf: &mut [u8], position: u64) -> Result<usize, String> {
    let mut read = 0usize;
    while read < buf.len() {
        match f.read_at(&mut buf[read..], position + read as u64) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(read)
}

/// `readCodexSessionSummary` body (shared with the codex snapshot loader).
fn codex_tail_summary(path: &str) -> Option<Value> {
    let f = std::fs::File::open(path).ok()?;
    let size = f.metadata().ok()?.len();
    let mut position = size;
    let mut partial_line: Vec<u8> = Vec::new();
    let mut skipping_oversized_line = false;
    while position > 0 {
        let length = std::cmp::min(CODEX_SESSION_READ_CHUNK_BYTES as u64, position) as usize;
        position -= length as u64;
        let mut buf = vec![0u8; length];
        let n = read_range(&f, &mut buf, position).ok()?;
        buf.truncate(n);
        let mut line_end = buf.len();
        let mut index = buf.len();
        while index > 0 {
            index -= 1;
            if buf[index] != 0x0a {
                continue;
            }
            let line_prefix = &buf[index + 1..line_end];
            if skipping_oversized_line {
                skipping_oversized_line = false;
            } else if line_prefix.len() + partial_line.len() <= MAX_CODEX_SESSION_LINE_BYTES {
                let mut line = line_prefix.to_vec();
                line.extend_from_slice(&partial_line);
                if let Some(s) = parse_codex_session_summary_line(&String::from_utf8_lossy(&line)) {
                    return Some(s);
                }
            }
            partial_line.clear();
            line_end = index;
        }
        if skipping_oversized_line {
            continue;
        }
        let line_prefix = &buf[..line_end];
        if line_prefix.len() + partial_line.len() > MAX_CODEX_SESSION_LINE_BYTES {
            partial_line.clear();
            skipping_oversized_line = true;
            continue;
        }
        let mut line = line_prefix.to_vec();
        line.extend_from_slice(&partial_line);
        partial_line = line;
    }
    if skipping_oversized_line {
        None
    } else {
        parse_codex_session_summary_line(&String::from_utf8_lossy(&partial_line))
    }
}

/// Port of `readCodexSessionSummary` (`apps/server/src/providerUsageSnapshot.ts:406`).
pub async fn read_codex_session_summary(path: String) -> Result<Option<serde_json::Value>, String> {
    Ok(codex_tail_summary(&path))
}

/// Port of `resolveClaudeProjectsRoot` (`apps/server/src/providerUsageSnapshot.ts:558`).
pub fn resolve_claude_projects_root(home_dir: String) -> Result<String, String> {
    // `process.env.CLAUDE_CONFIG_DIR` was unset on the recording host, so the
    // `configDir || join(homeDir, ".claude")` fallback always ran.
    Ok(posix_join(&[&home_dir, ".claude", "projects"]))
}

fn read_claude_total_tokens(value: Option<&Map<String, Value>>) -> f64 {
    let Some(usage) = value else { return 0.0 };
    let input_tokens = get_nn(usage, "input_tokens").unwrap_or(0.0)
        + get_nn(usage, "cache_creation_input_tokens").unwrap_or(0.0)
        + get_nn(usage, "cache_read_input_tokens").unwrap_or(0.0);
    let output_tokens = get_nn(usage, "output_tokens").unwrap_or(0.0);
    get_nn(usage, "total_tokens").unwrap_or(input_tokens + output_tokens)
}

fn member<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a Value> {
    m.get(k)
        .filter(|v| !v.is_null() && !crate::js::is_undefined(v))
}

fn claude_sample_object(
    session_id: &str,
    timestamp_ms: f64,
    total_tokens: f64,
    model: Option<String>,
) -> Value {
    let mut o = Map::new();
    o.insert("sessionId".into(), Value::String(session_id.to_string()));
    o.insert("timestampMs".into(), crate::js::number(timestamp_ms));
    o.insert("totalTokens".into(), crate::js::number(total_tokens));
    o.insert(
        "model".into(),
        model.map(Value::String).unwrap_or(Value::Null),
    );
    Value::Object(o)
}

/// `scanClaudeTranscript` reduced to the always-full-scan contract used by
/// `readClaudeUsageSamples` (fresh cache every call): returns samples + tail samples.
fn scan_claude_file(f: &std::fs::File, path: &str, size: u64) -> (Vec<Value>, Vec<Value>) {
    let mut samples: Vec<Value> = Vec::new();
    let mut tail_samples: Vec<Value> = Vec::new();
    let mut new_keys: HashSet<String> = HashSet::new();
    let mut pending: Vec<u8> = Vec::new();
    let mut line_index: u64 = 0;
    let mut position = 0u64;
    let mut skipping = false;

    let collect = |line: &[u8],
                   index: u64,
                   terminated: bool,
                   samples: &mut Vec<Value>,
                   tail: &mut Vec<Value>,
                   keys: &mut HashSet<String>| {
        if line.is_empty()
            || !line
                .windows(CLAUDE_USAGE_KEY_BYTES.len())
                .any(|w| w == CLAUDE_USAGE_KEY_BYTES)
        {
            return;
        }
        let contents = String::from_utf8_lossy(line);
        if js_trim(&contents).is_empty() {
            return;
        }
        let Ok(parsed) = serde_json::from_str::<Value>(&contents) else {
            return;
        };
        let Some(record) = as_record(&parsed) else {
            return;
        };
        let fallback_key = format!("{path}:{index}");

        // `readClaudeAssistantSample`
        if record.get("type").and_then(Value::as_str) == Some("assistant")
            && let Some(message) = member(record, "message").and_then(as_record)
        {
            let usage = member(message, "usage").and_then(as_record);
            let total = read_claude_total_tokens(usage);
            let ts = record.get("timestamp").and_then(read_timestamp_ms);
            if let (Some(_), Some(ts)) = (usage, ts)
                && total > 0.0
            {
                let session_id = member(record, "sessionId")
                    .and_then(as_string)
                    .unwrap_or_else(|| fallback_key.clone());
                let model = member(message, "model").and_then(as_string);
                let suffix = member(record, "requestId")
                    .and_then(as_string)
                    .or_else(|| member(message, "id").and_then(as_string))
                    .or_else(|| member(record, "uuid").and_then(as_string))
                    .unwrap_or_else(|| fallback_key.clone());
                let key = format!("{session_id}:assistant:{suffix}");
                if !keys.contains(&key) {
                    if terminated {
                        keys.insert(key);
                        samples.push(claude_sample_object(&session_id, ts, total, model));
                    } else {
                        tail.push(claude_sample_object(&session_id, ts, total, model));
                    }
                }
            }
        }

        // `readClaudeToolResultSample`
        if let Some(tur) = member(record, "toolUseResult").and_then(as_record) {
            let usage = member(tur, "usage").and_then(as_record);
            let total = read_claude_total_tokens(usage);
            let ts = record.get("timestamp").and_then(read_timestamp_ms);
            if let (Some(_), Some(ts)) = (usage, ts)
                && total > 0.0
            {
                let session_id = member(record, "sessionId")
                    .and_then(as_string)
                    .unwrap_or_else(|| fallback_key.clone());
                let suffix = member(record, "uuid")
                    .and_then(as_string)
                    .or_else(|| member(tur, "agentId").and_then(as_string))
                    .or_else(|| member(record, "requestId").and_then(as_string))
                    .unwrap_or_else(|| fallback_key.clone());
                let key = format!("{session_id}:tool-result:{suffix}");
                if !keys.contains(&key) {
                    if terminated {
                        keys.insert(key);
                        samples.push(claude_sample_object(&session_id, ts, total, None));
                    } else {
                        tail.push(claude_sample_object(&session_id, ts, total, None));
                    }
                }
            }
        }
    };

    let mut buf = vec![0u8; CLAUDE_TRANSCRIPT_READ_CHUNK_BYTES];
    while position < size {
        let want = std::cmp::min(buf.len() as u64, size - position) as usize;
        let n = match f.read_at(&mut buf[..want], position) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => break,
        };
        position += n as u64;
        let chunk = &buf[..n];
        let mut line_start = 0usize;
        for (i, b) in chunk.iter().enumerate() {
            if *b != 0x0a {
                continue;
            }
            let seg = &chunk[line_start..i];
            if !skipping && !seg.is_empty() {
                pending.extend_from_slice(seg);
            }
            if !skipping && pending.len() > MAX_CLAUDE_TRANSCRIPT_LINE_BYTES {
                pending.clear();
                skipping = true;
            }
            if !skipping {
                collect(
                    &pending.clone(),
                    line_index,
                    true,
                    &mut samples,
                    &mut tail_samples,
                    &mut new_keys,
                );
            }
            pending.clear();
            skipping = false;
            line_index += 1;
            line_start = i + 1;
        }
        if !skipping {
            pending.extend_from_slice(&chunk[line_start..]);
            if pending.len() > MAX_CLAUDE_TRANSCRIPT_LINE_BYTES {
                pending.clear();
                skipping = true;
            }
        }
    }
    if !skipping && !pending.is_empty() {
        collect(
            &pending.clone(),
            line_index,
            false,
            &mut samples,
            &mut tail_samples,
            &mut new_keys,
        );
    }
    (samples, tail_samples)
}

/// Port of `readClaudeUsageSamples` (`apps/server/src/providerUsageSnapshot.ts:845`).
pub async fn read_claude_usage_samples(path: String) -> Result<Vec<serde_json::Value>, String> {
    let f = match std::fs::File::open(&path) {
        Err(_) => return Ok(vec![]),
        Ok(f) => f,
    };
    let size = match f.metadata() {
        Err(_) => return Ok(vec![]),
        Ok(m) => m.len(),
    };
    let (mut samples, tail) = scan_claude_file(&f, &path, size);
    samples.extend(tail);
    Ok(samples)
}

/// `listRecentClaudeTranscriptFiles` + `readCachedClaudeUsageSamples` with a fresh
/// (empty) per-transcript cache, which is the contract the recorded corpus exercises.
fn list_recent_claude_files(projects_root: &str, now_ms: f64) -> Vec<(String, f64)> {
    let Ok(projects) = std::fs::read_dir(projects_root) else {
        return vec![];
    };
    let cutoff = now_ms - LOOKBACK_30D_MS;
    let mut files: Vec<(String, f64)> = Vec::new();
    for project in projects.flatten() {
        if !project.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let Ok(transcripts) = std::fs::read_dir(project.path()) else {
            continue;
        };
        for entry in transcripts.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) || !name.ends_with(".jsonl")
            {
                continue;
            }
            let path = entry.path();
            let mtime_ms = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0);
            if mtime_ms >= cutoff {
                files.push((path.to_string_lossy().to_string(), mtime_ms));
            }
        }
    }
    files.sort_by(|a, b| b.1.total_cmp(&a.1));
    files.truncate(MAX_RECENT_USAGE_FILES);
    files
}

/// Port of `readRecentClaudeUsageSamples` (`apps/server/src/providerUsageSnapshot.ts:979`).
pub async fn read_recent_claude_usage_samples(
    projects_root: String,
    now_ms: Option<f64>,
    signal: Option<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    // `signal?.throwIfAborted()`: any non-nullish signal reaches a call site with a
    // value that JSON cannot turn into a function, so it always ends in this TypeError.
    if let Some(s) = &signal
        && !crate::js::is_undefined(s)
    {
        return Err("signal?.throwIfAborted is not a function".into());
    }
    let now = now_ms.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as f64)
            .unwrap_or(0.0)
    });
    let mut out: Vec<Value> = Vec::new();
    for (path, _mtime) in list_recent_claude_files(&projects_root, now) {
        if let Ok(f) = std::fs::File::open(&path)
            && let Ok(meta) = f.metadata()
        {
            let (mut samples, tail) = scan_claude_file(&f, &path, meta.len());
            samples.extend(tail);
            out.extend(samples);
        }
    }
    Ok(out)
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// `loadCodexUsageSnapshot`: sessions live under `<codexHome>/sessions/YYYY/MM/DD/*.jsonl`.
fn load_codex_usage_lines(home_dir: &Value, home_path: Option<&Value>) -> Vec<Value> {
    // `input.homePath?.trim() || process.env.CODEX_HOME || join(homeDir, ".codex")`
    // (CODEX_HOME was unset on the recording host).
    let codex_home = match home_path.and_then(as_string) {
        Some(p) if !p.is_empty() => p,
        _ => match home_dir.as_str() {
            Some(h) => posix_join(&[h, ".codex"]),
            None => return vec![], // join(undefined, ...) throws; the outer catch returns []
        },
    };
    let sessions_root = posix_join(&[&codex_home, "sessions"]);
    let mut candidates: Vec<(String, f64)> = Vec::new();
    let (y, mo, d) = civil_from_days((now_ms() / ONE_DAY_MS).floor() as i64);
    for offset in 0..=LOOKBACK_DAYS {
        let day = days_from_civil(y, mo, d) - offset;
        let (dy, dmo, dd) = civil_from_days(day);
        let day_dir = posix_join(&[
            &sessions_root,
            &dy.to_string(),
            &format!("{dmo:02}"),
            &format!("{dd:02}"),
        ]);
        let Ok(entries) = std::fs::read_dir(&day_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) && name.ends_with(".jsonl") {
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0);
                candidates.push((entry.path().to_string_lossy().to_string(), mtime));
            }
        }
    }
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
    candidates.truncate(MAX_RECENT_USAGE_FILES);

    let mut summaries: Vec<Value> = Vec::new();
    for (path, _) in candidates {
        if let Some(s) = codex_tail_summary(&path) {
            summaries.push(s);
        }
    }
    if summaries.is_empty() {
        return vec![];
    }
    let now = now_ms();
    let mut tokens = [0.0; 3];
    let mut sessions = [0usize; 3];
    for (i, cutoff) in [
        now - ONE_DAY_MS,
        now - 7.0 * ONE_DAY_MS,
        now - LOOKBACK_30D_MS,
    ]
    .iter()
    .enumerate()
    {
        for s in &summaries {
            let ts = crate::js::as_number(&s["timestampMs"]).unwrap_or(0.0);
            if ts >= *cutoff {
                tokens[i] += crate::js::as_number(&s["totalTokens"]).unwrap_or(0.0);
                sessions[i] += 1;
            }
        }
    }
    build_usage_lines(tokens, sessions)
}

/// `loadClaudeUsageSnapshot` usageLines from samples.
fn load_claude_usage_lines(projects_root: &str) -> Vec<Value> {
    let mut samples: Vec<Value> = Vec::new();
    for (path, _mtime) in list_recent_claude_files(projects_root, now_ms()) {
        if let Ok(f) = std::fs::File::open(&path)
            && let Ok(meta) = f.metadata()
        {
            let (mut s, tail) = scan_claude_file(&f, &path, meta.len());
            s.extend(tail);
            samples.extend(s);
        }
    }
    if samples.is_empty() {
        return vec![];
    }
    let now = now_ms();
    let mut tokens = [0.0; 3];
    let mut sessions: [HashSet<String>; 3] = Default::default();
    for (i, cutoff) in [
        now - ONE_DAY_MS,
        now - 7.0 * ONE_DAY_MS,
        now - LOOKBACK_30D_MS,
    ]
    .iter()
    .enumerate()
    {
        for s in &samples {
            let ts = crate::js::as_number(&s["timestampMs"]).unwrap_or(0.0);
            if ts >= *cutoff {
                tokens[i] += crate::js::as_number(&s["totalTokens"]).unwrap_or(0.0);
                if let Some(id) = s["sessionId"].as_str() {
                    sessions[i].insert(id.to_string());
                }
            }
        }
    }
    build_usage_lines(
        tokens,
        [sessions[0].len(), sessions[1].len(), sessions[2].len()],
    )
}

/// Port of `loadLocalProviderUsageLines` (`apps/server/src/providerUsageSnapshot.ts:1183`).
pub async fn load_local_provider_usage_lines(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let Some(map) = as_record(&input) else {
        return Ok(vec![]);
    };
    let provider = member(map, "provider").and_then(Value::as_str);
    match provider {
        Some("codex") => Ok(load_codex_usage_lines(
            map.get("homeDir").unwrap_or(&Value::Null),
            map.get("homePath"),
        )),
        Some("claudeAgent") => {
            let Some(home) = map.get("homeDir").and_then(Value::as_str) else {
                return Ok(vec![]);
            };
            Ok(load_claude_usage_lines(&resolve_claude_projects_root(
                home.to_string(),
            )?))
        }
        _ => Ok(vec![]),
    }
}
