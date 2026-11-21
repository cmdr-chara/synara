//! The Rust port of `apps/desktop/src/updateState.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

fn member(v: &Value, key: &str) -> Value {
    v.as_object()
        .and_then(|o| o.get(key))
        .cloned()
        .unwrap_or_else(js::undefined)
}

fn null_err(key: &str) -> String {
    format!("Cannot read properties of null (reading '{key}')")
}

fn undef_err(key: &str) -> String {
    format!("Cannot read properties of undefined (reading '{key}')")
}

fn read_or_err(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(null_err(key));
    }
    if js::is_undefined(v) {
        return Err(undef_err(key));
    }
    Ok(member(v, key))
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

fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| {
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
    })
}

/// Port of `canCheckForDesktopUpdates` (`apps/desktop/src/updateState.ts:11`).
pub fn can_check_for_desktop_updates(state: serde_json::Value) -> Result<bool, String> {
    let status = read_or_err(&state, "status")?;
    Ok(status.as_str() != Some("checking") && status.as_str() != Some("downloading"))
}

/// Port of `getDownloadStallTimeoutMessage` (`apps/desktop/src/updateState.ts:15`).
pub fn get_download_stall_timeout_message(timeout_ms: f64) -> Result<String, String> {
    let rounded = js::math_round(timeout_ms / 1000.0);
    let seconds = if rounded.is_nan() || rounded < 1.0 {
        if rounded.is_nan() { f64::NAN } else { 1.0 }
    } else {
        rounded
    };
    Ok(format!(
        "Download stalled after {} seconds without progress. Try again.",
        js::number_to_string(seconds)
    ))
}

/// Port of `isExpectedStalledDownloadCancellationError` (`apps/desktop/src/updateState.ts:20`).
pub fn is_expected_stalled_download_cancellation_error(
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let armed = read_or_err(&args, "suppressionArmed")?;
    if !truthy(&armed) {
        return Ok(armed);
    }
    let context = member(&args, "errorContext");
    if context.as_str() != Some("download") {
        return Ok(Value::Bool(false));
    }
    let message = member(&args, "message");
    match &message {
        Value::String(s) => Ok(Value::Bool(trim_js(s).to_lowercase() == "cancelled")),
        Value::Null => Err(null_err("trim")),
        _ if js::is_undefined(&message) => Err(undef_err("trim")),
        _ => Err("args.message.trim is not a function".to_string()),
    }
}

fn finite_number(v: &Value) -> Option<f64> {
    js::as_number(v).filter(|n| n.is_finite())
}

/// Port of `hasDownloadProgressAdvanced` (`apps/desktop/src/updateState.ts:36`).
pub fn has_download_progress_advanced(
    previous: Option<serde_json::Value>,
    next: serde_json::Value,
) -> Result<bool, String> {
    let next_transferred = finite_number(&read_or_err(&next, "transferred")?);
    let next_percent = finite_number(&member(&next, "percent"));
    if next_transferred.is_none() && next_percent.is_none() {
        return Ok(false);
    }
    let Some(previous) = previous else {
        return Ok(true);
    };
    let prev_transferred = finite_number(&member(&previous, "transferred"));
    let prev_percent = finite_number(&member(&previous, "percent"));
    let transferred_advanced = match prev_transferred {
        None => next_transferred.is_some(),
        Some(p) => next_transferred.is_some_and(|n| n > p),
    };
    let percent_advanced = match prev_percent {
        None => next_percent.is_some(),
        Some(p) => next_percent.is_some_and(|n| n > p),
    };
    Ok(transferred_advanced || percent_advanced)
}

/// Port of `shouldBroadcastDownloadProgress` (`apps/desktop/src/updateState.ts:64`).
pub fn should_broadcast_download_progress(
    current_state: serde_json::Value,
    next_percent: f64,
) -> Result<bool, String> {
    let status = read_or_err(&current_state, "status")?;
    if status.as_str() != Some("downloading") {
        return Ok(true);
    }
    let current_percent = member(&current_state, "downloadPercent");
    if current_percent.is_null() {
        return Ok(true);
    }
    let prev_step = num(&current_percent).floor();
    let next_step = next_percent.floor();
    Ok(next_step != prev_step || next_percent == 100.0)
}

struct ParsedUpdateVersion {
    major: f64,
    minor: f64,
    patch: f64,
    prerelease: Option<String>,
}

fn semver_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn dotted_idents(s: &str) -> bool {
    s.split('.').all(semver_ident)
}

fn parse_update_version(version: &str) -> Option<ParsedUpdateVersion> {
    let mut rest = trim_js(version);
    rest = rest.strip_prefix('v').unwrap_or(rest);
    let take_number = |s: &mut &str| -> Option<f64> {
        let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
        if end == 0 {
            return None;
        }
        let n: f64 = s[..end].parse().ok()?;
        *s = &s[end..];
        Some(n)
    };
    let major = take_number(&mut rest)?;
    rest = rest.strip_prefix('.')?;
    let minor = take_number(&mut rest)?;
    rest = rest.strip_prefix('.')?;
    let patch = take_number(&mut rest)?;
    let mut prerelease = None;
    if let Some(tail) = rest.strip_prefix('-') {
        let end = tail.find('+').unwrap_or(tail.len());
        let ident = &tail[..end];
        if !dotted_idents(ident) {
            return None;
        }
        prerelease = Some(ident.to_string());
        rest = &tail[end..];
    }
    if let Some(build) = rest.strip_prefix('+') {
        if !dotted_idents(build) {
            return None;
        }
        rest = "";
    }
    if !rest.is_empty() {
        return None;
    }
    Some(ParsedUpdateVersion {
        major,
        minor,
        patch,
        prerelease,
    })
}

fn is_all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// Port of `isUpdateVersionNewer` (`apps/desktop/src/updateState.ts:106`).
pub fn is_update_version_newer(
    current_version: String,
    candidate_version: String,
) -> Result<bool, String> {
    let current = parse_update_version(&current_version);
    let candidate = parse_update_version(&candidate_version);
    let (Some(current), Some(candidate)) = (current, candidate) else {
        return Ok(trim_js(&candidate_version) != trim_js(&current_version));
    };
    if candidate.major != current.major {
        return Ok(candidate.major > current.major);
    }
    if candidate.minor != current.minor {
        return Ok(candidate.minor > current.minor);
    }
    if candidate.patch != current.patch {
        return Ok(candidate.patch > current.patch);
    }
    if let (Some(cur_pre), Some(cand_pre)) = (&current.prerelease, &candidate.prerelease) {
        let cand_parts: Vec<&str> = cand_pre.split('.').collect();
        let cur_parts: Vec<&str> = cur_pre.split('.').collect();
        for index in 0..cand_parts.len().max(cur_parts.len()) {
            let cand = cand_parts.get(index);
            let cur = cur_parts.get(index);
            if cand.is_none() {
                return Ok(false);
            }
            if cur.is_none() {
                return Ok(true);
            }
            let (Some(&cand), Some(&cur)) = (cand, cur) else {
                return Ok(false);
            };
            let cand_numeric = is_all_digits(cand);
            let cur_numeric = is_all_digits(cur);
            if cand_numeric && cur_numeric {
                let cn: f64 = cand.parse().unwrap_or(f64::NAN);
                let cu: f64 = cur.parse().unwrap_or(f64::NAN);
                if cn != cu {
                    return Ok(cn > cu);
                }
                continue;
            }
            if cand_numeric != cur_numeric {
                return Ok(!cand_numeric);
            }
            if cand != cur {
                return Ok(cand > cur);
            }
        }
        return Ok(false);
    }
    Ok(current.prerelease.is_some() && candidate.prerelease.is_none())
}

/// Port of `isUpdateVersionAllowedForFlavor` (`apps/desktop/src/updateState.ts:155`).
pub fn is_update_version_allowed_for_flavor(
    candidate_version: String,
    flavor: String,
) -> Result<bool, String> {
    let candidate = parse_update_version(&candidate_version);
    let Some(candidate) = candidate else {
        return Ok(flavor != "beta" && flavor != "remote" && flavor != "production");
    };
    if flavor == "beta" || flavor == "remote" {
        return Ok(match &candidate.prerelease {
            Some(pre) => pre == &flavor || pre.starts_with(&format!("{flavor}.")),
            None => false,
        });
    }
    if flavor == "production" {
        return Ok(candidate.prerelease.is_none());
    }
    Ok(true)
}

/// Port of `nextStatusAfterDownloadFailure` (`apps/desktop/src/updateState.ts:177`).
pub fn next_status_after_download_failure(
    current_state: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let available = read_or_err(&current_state, "availableVersion")?;
    Ok(Value::String(
        if truthy(&available) {
            "available"
        } else {
            "error"
        }
        .to_string(),
    ))
}

/// Port of `getCanRetryAfterDownloadFailure` (`apps/desktop/src/updateState.ts:183`).
pub fn get_can_retry_after_download_failure(
    current_state: serde_json::Value,
) -> Result<bool, String> {
    let available = read_or_err(&current_state, "availableVersion")?;
    Ok(!available.is_null())
}

/// Days-from-civil for a strict `YYYY-MM-DD` prefix; `None` on anything else.
fn iso_date_ms(s: &str) -> f64 {
    let bytes = s.as_bytes();
    if bytes.len() < 10
        || !bytes[..4].iter().all(|b| b.is_ascii_digit())
        || bytes[4] != b'-'
        || !bytes[5..7].iter().all(|b| b.is_ascii_digit())
        || bytes[7] != b'-'
        || !bytes[8..10].iter().all(|b| b.is_ascii_digit())
    {
        return f64::NAN;
    }
    let y: i64 = s[..4].parse().unwrap_or(0);
    let m: i64 = s[5..7].parse().unwrap_or(0);
    let d: i64 = s[8..10].parse().unwrap_or(0);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return f64::NAN;
    }
    let y_adj = if m <= 2 { y - 1 } else { y };
    let era = y_adj.div_euclid(400);
    let yoe = y_adj.rem_euclid(400);
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;
    (days * 86_400_000) as f64
}

/// Port of `shouldCheckForUpdatesOnForeground` (`apps/desktop/src/updateState.ts:187`).
pub fn should_check_for_updates_on_foreground(args: serde_json::Value) -> Result<bool, String> {
    if args.is_null() {
        return Err("Cannot destructure property 'checkedAt' of 'args' as it is null.".to_string());
    }
    if js::is_undefined(&args) {
        return Err(
            "Cannot destructure property 'checkedAt' of 'args' as it is undefined.".to_string(),
        );
    }
    let checked_at = member(&args, "checkedAt");
    let backgrounded = member(&args, "backgroundedAtMs");
    let foregrounded = num(&member(&args, "foregroundedAtMs"));
    let min_background = num(&member(&args, "minBackgroundDurationMs"));
    let min_interval = num(&member(&args, "minIntervalMs"));
    if backgrounded.is_null() {
        return Ok(false);
    }
    let backgrounded_n = num(&backgrounded);
    if !backgrounded_n.is_nan() && foregrounded <= backgrounded_n {
        return Ok(false);
    }
    let diff = foregrounded - backgrounded_n;
    if !diff.is_nan() && !min_background.is_nan() && diff < min_background {
        return Ok(false);
    }
    if checked_at.is_null() {
        return Ok(true);
    }
    let last_checked_ms = match &checked_at {
        Value::String(s) => iso_date_ms(s),
        _ => f64::NAN,
    };
    if !last_checked_ms.is_finite() {
        return Ok(true);
    }
    Ok(foregrounded - last_checked_ms >= min_interval)
}

/// Port of `getAutoUpdateDisabledReason` (`apps/desktop/src/updateState.ts:217`).
pub fn get_auto_update_disabled_reason(args: serde_json::Value) -> Result<Option<String>, String> {
    if args.is_null() {
        return Err(null_err("hasUpdateFeedConfig"));
    }
    if js::is_undefined(&args) {
        return Err(undef_err("hasUpdateFeedConfig"));
    }
    if !truthy(&member(&args, "hasUpdateFeedConfig")) {
        return Ok(Some(
            "Automatic updates are not available because no update feed is configured.".to_string(),
        ));
    }
    if truthy(&member(&args, "isDevelopment")) || !truthy(&member(&args, "isPackaged")) {
        return Ok(Some(
            "Automatic updates are only available in packaged production builds.".to_string(),
        ));
    }
    if truthy(&member(&args, "disabledByEnv")) {
        return Ok(Some(
            "Automatic updates are disabled by the SYNARA_DISABLE_AUTO_UPDATE setting.".to_string(),
        ));
    }
    if member(&args, "platform").as_str() == Some("linux") && !truthy(&member(&args, "appImage")) {
        return Ok(Some(
            "Automatic updates on Linux require running the AppImage build.".to_string(),
        ));
    }
    Ok(None)
}

fn payload_basename(value: &str) -> String {
    // `new URL(value, "https://payload.invalid/").pathname` basename.
    let mut path = value.trim_start();
    if let Some(i) = path.find(':') {
        let (scheme, rest) = path.split_at(i);
        if scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
            && scheme
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
            && rest.len() > 1
        {
            let rest = &rest[1..];
            let rest = rest.strip_prefix("//").unwrap_or(rest);
            let rest = rest.trim_start_matches(|c| c != '/' && c != '?' && c != '#');
            path = rest;
            if path.is_empty() {
                return String::new();
            }
        }
    }
    let end = path.find(['?', '#']).unwrap_or(path.len());
    let path = &path[..end];
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    out.last().copied().unwrap_or_default().to_string()
}

fn ends_with_ci(s: &str, suffix: &str) -> bool {
    s.len() >= suffix.len() && s[s.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// Port of `updateFilesMatchRuntimeArch` (`apps/desktop/src/updateState.ts:253`).
pub fn update_files_match_runtime_arch(
    info: serde_json::Value,
    platform: serde_json::Value,
    arch: String,
) -> Result<bool, String> {
    if platform.as_str() != Some("linux") {
        return Ok(true);
    }
    if info.is_null() {
        return Err(null_err("path"));
    }
    if js::is_undefined(&info) {
        return Err(undef_err("path"));
    }
    let mut candidates: Vec<Value> = vec![member(&info, "path")];
    if let Value::Array(files) = member(&info, "files") {
        for file in files {
            candidates.push(member(&file, "url"));
        }
    }
    let mut app_images: Vec<String> = Vec::new();
    for url in &candidates {
        if let Value::String(text) = url {
            let basename = payload_basename(text);
            if ends_with_ci(&basename, ".appimage") {
                app_images.push(basename);
            }
        }
    }
    if app_images.is_empty() {
        return Ok(false);
    }
    Ok(app_images.iter().all(|name| {
        if arch == "arm64" {
            ends_with_ci(name, "arm64.appimage")
        } else {
            !fold_contains(name, "arm64")
        }
    }))
}

fn fold_contains(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

/// Port of `shouldAcceptAvailableUpdate` (`apps/desktop/src/updateState.ts:291`).
pub fn should_accept_available_update(input: serde_json::Value) -> Result<bool, String> {
    let version = read_or_err(&input, "version")?;
    let Some(version) = version.as_str() else {
        return Ok(false);
    };
    let flavor = member(&input, "flavor");
    let current = member(&input, "currentVersion");
    let info = member(&input, "info");
    let platform = member(&input, "platform");
    let arch = member(&input, "arch");
    Ok(is_update_version_allowed_for_flavor(
        version.to_string(),
        flavor.as_str().unwrap_or_default().to_string(),
    )? && is_update_version_newer(
        current.as_str().unwrap_or_default().to_string(),
        version.to_string(),
    )? && update_files_match_runtime_arch(
        info,
        platform,
        arch.as_str().unwrap_or_default().to_string(),
    )?)
}
