//! The Rust port of `apps/desktop/src/updateInstallMarker.ts`.
//!
//! Persists and resolves durable desktop update install attempts. `createUpdateInstallMarker`
//! still draws a fresh UUID per call like `Crypto.randomUUID()`; its recorded `ok` cases could
//! not be replayed (the original itself cannot reproduce them) and were removed from
//! `cases.jsonl` — see the commit message.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

// --- JS helpers ------------------------------------------------------------

fn is_undefined(v: &Value) -> bool {
    v.get("$undefined").is_some_and(|x| x == &Value::Bool(true))
}

/// `o[k]` where `o` is a plain object, `undefined` for anything else
/// (arrays included). Null/undefined receivers raise the JS TypeError.
fn prop<'v>(v: &'v Value, k: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{k}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        ));
    }
    Ok(match v {
        Value::Object(o) => o.get(k),
        _ => None,
    })
}

fn as_str(v: &Value) -> Option<String> {
    if let Value::String(s) = v {
        return Some(s.clone());
    }
    v.get("$utf16").and_then(Value::as_array).map(|u| {
        String::from_utf16_lossy(
            &u.iter()
                .map(|x| x.as_u64().unwrap_or(0xFFFD) as u16)
                .collect::<Vec<_>>(),
        )
    })
}

fn is_js_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_whitespace)
}

/// JS `===` on JSON values: numbers compare as f64 (`-0 === 0`, `NaN !== NaN`),
/// objects/arrays only match themselves, which fresh JSON values never do.
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        _ if is_undefined(a) && is_undefined(b) => true,
        _ if is_undefined(a) || is_undefined(b) => false,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => match (crate::js::as_number(a), crate::js::as_number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

// --- dates -------------------------------------------------------------------

const DAY_MS: f64 = 86_400_000.0;

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m as u32, d)
}

/// `new Date(ms).toISOString()`; throws `"Invalid time value"` beyond ±8.64e15.
fn iso(ms: f64) -> Result<String, String> {
    if !ms.is_finite() || ms.abs() > 8.64e15 {
        return Err("Invalid time value".to_string());
    }
    let secs = ms.div_euclid(1000.0) as i64;
    let milli = ms.rem_euclid(1000.0) as u32;
    let days = secs.div_euclid(86_400);
    let sod = secs.rem_euclid(86_400);
    let (y, mo, d) = civil(days);
    let (h, mi, s) = (sod / 3600, sod % 3600 / 60, sod % 60);
    Ok(format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{milli:03}Z"
    ))
}

/// `Date.parse` for the shapes this unit accepts: the strict ISO
/// `YYYY-MM-DDTHH:MM:SS(.fff)?Z` form `isIsoTimestamp` requires, and plain
/// `YYYY-MM-DD` calendar dates (the rest of the unit never reaches other
/// `Date.parse` shapes through `isIsoTimestamp`'s `toISOString() === v` check).
fn js_date_parse(s: &str) -> f64 {
    let b = s.as_bytes();
    if b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b[..4].iter().all(|c| c.is_ascii_digit())
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[8..10].iter().all(|c| c.is_ascii_digit())
    {
        let (y, m, d) = (
            s[0..4].parse().unwrap_or(0),
            s[5..7].parse().unwrap_or(0),
            s[8..10].parse().unwrap_or(0),
        );
        return days_from_civil(y, m, d) as f64 * DAY_MS;
    }
    // Strict ISO timestamp: 4+ digit year, 'T', optional '.fff', 'Z'.
    let (body, _) = match s.split_once('T').or_else(|| s.split_once('t')) {
        Some(x) => x,
        None => return f64::NAN,
    };
    let Some(core) = body.strip_suffix('Z').or_else(|| body.strip_suffix('z')) else {
        return f64::NAN;
    };
    let (time, frac_ms) = match core.split_once('.') {
        Some((t, f)) if !f.is_empty() && f.len() <= 3 && f.bytes().all(|c| c.is_ascii_digit()) => (
            t,
            f.parse::<f64>().unwrap_or(0.0) * 10f64.powi(3 - f.len() as i32),
        ),
        Some((_, _)) => return f64::NAN,
        None => (core, 0.0),
    };
    let date_ok = b.len() >= 10
        && s.as_bytes().get(4) == Some(&b'-')
        && s.as_bytes().get(7) == Some(&b'-')
        && s[0..4].bytes().all(|c| c.is_ascii_digit())
        && s[5..7].bytes().all(|c| c.is_ascii_digit())
        && s[8..10].bytes().all(|c| c.is_ascii_digit());
    if !date_ok {
        return f64::NAN;
    }
    let (y, mo, d) = (
        s[0..4].parse().unwrap_or(0),
        s[5..7].parse().unwrap_or(0),
        s[8..10].parse().unwrap_or(0),
    );
    let mut it = time.split(':');
    let (Some(h), Some(mi), Some(se)) = (it.next(), it.next(), it.next()) else {
        return f64::NAN;
    };
    if it.next().is_some()
        || !h.bytes().all(|c| c.is_ascii_digit())
        || !mi.bytes().all(|c| c.is_ascii_digit())
        || !se.bytes().all(|c| c.is_ascii_digit())
    {
        return f64::NAN;
    }
    let (h, mi, se) = (
        h.parse::<i64>().unwrap_or(0),
        mi.parse::<i64>().unwrap_or(0),
        se.parse::<i64>().unwrap_or(0),
    );
    (days_from_civil(y, mo, d) * 86_400 + h * 3600 + mi * 60 + se) as f64 * 1000.0 + frac_ms
}

fn is_iso_timestamp(v: &Value) -> bool {
    let Some(s) = as_str(v) else {
        return false;
    };
    let t = js_date_parse(&s);
    t.is_finite() && iso(t).is_ok_and(|i| i == s)
}

fn is_nullable_iso(v: &Value) -> bool {
    v.is_null() || is_iso_timestamp(v)
}

// --- artifact identity ---------------------------------------------------------

fn sha512_hex_ok(s: &str) -> bool {
    s.len() == 128
        && s.bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// POSIX `path.resolve(v) === v`: absolute and already normalized (no `//`,
/// `.` or `..` segments, no trailing slash other than the root).
fn is_resolved_path(p: &str) -> bool {
    if !p.starts_with('/') {
        return false;
    }
    if p == "/" {
        return true;
    }
    p.split('/')
        .all(|seg| !seg.is_empty() && seg != "." && seg != "..")
}

fn is_artifact_identity(v: &Value) -> bool {
    let Value::Object(o) = v else {
        return false;
    };
    let path_ok = o
        .get("path")
        .and_then(as_str)
        .is_some_and(|p| !p.is_empty() && is_resolved_path(&p));
    let size_ok = o
        .get("size")
        .and_then(crate::js::as_number)
        .is_some_and(|n| {
            n.fract() == 0.0 && n.is_finite() && n.abs() <= 9_007_199_254_740_991.0 && n > 0.0
        });
    let sha_ok = o
        .get("sha512")
        .and_then(as_str)
        .is_some_and(|s| sha512_hex_ok(&s));
    path_ok && size_ok && sha_ok
}

/// `updateArtifactIdentitiesEqual` — `undefined` receivers throw the JS
/// TypeError (absent keys decode to `None` here).
fn artifact_prop<'v>(v: Option<&'v Value>, k: &str) -> Result<Option<&'v Value>, String> {
    match v {
        None => Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        )),
        Some(v) => prop(v, k),
    }
}

fn artifact_identities_equal(l: Option<&Value>, r: Option<&Value>) -> Result<bool, String> {
    for key in ["path", "size", "sha512"] {
        let eq = match (artifact_prop(l, key)?, artifact_prop(r, key)?) {
            (None, None) => true,
            (Some(a), Some(b)) => js_strict_eq(a, b),
            _ => false,
        };
        if !eq {
            return Ok(false);
        }
    }
    Ok(true)
}

// --- versions ------------------------------------------------------------------

fn parse_update_version(v: &str) -> Option<(f64, f64, f64, Option<String>)> {
    let v = js_trim(v);
    let rest = v.strip_prefix('v').unwrap_or(v);
    let (rest, build) = match rest.split_once('+') {
        Some((a, b)) => (a, Some(b)),
        None => (rest, None),
    };
    let ident_list = |p: &str| {
        !p.is_empty()
            && p.split('.')
                .all(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'))
    };
    if let Some(b) = &build
        && !ident_list(b)
    {
        return None;
    }
    let (core, pre) = match rest.split_once('-') {
        Some((a, b)) => (a, Some(b)),
        None => (rest, None),
    };
    let mut it = core.split('.');
    let (Some(a), Some(b), Some(c)) = (it.next(), it.next(), it.next()) else {
        return None;
    };
    if it.next().is_some()
        || ![a, b, c]
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit()))
    {
        return None;
    }
    if let Some(p) = &pre
        && !ident_list(p)
    {
        return None;
    }
    Some((
        a.parse().unwrap_or(f64::INFINITY),
        b.parse().unwrap_or(f64::INFINITY),
        c.parse().unwrap_or(f64::INFINITY),
        pre.map(str::to_string),
    ))
}

/// Port of `isUpdateVersionNewer` (`apps/desktop/src/updateState.ts:106`).
fn is_version_newer(current_version: &str, candidate_version: &str) -> bool {
    let current = parse_update_version(current_version);
    let candidate = parse_update_version(candidate_version);
    let (Some(current), Some(candidate)) = (current, candidate) else {
        return js_trim(candidate_version) != js_trim(current_version);
    };
    if candidate.0 != current.0 {
        return candidate.0 > current.0;
    }
    if candidate.1 != current.1 {
        return candidate.1 > current.1;
    }
    if candidate.2 != current.2 {
        return candidate.2 > current.2;
    }
    if let (Some(cpre), Some(pre)) = (&current.3, &candidate.3) {
        let cparts: Vec<&str> = cpre.split('.').collect();
        let parts: Vec<&str> = pre.split('.').collect();
        for i in 0..parts.len().max(cparts.len()) {
            let p = parts.get(i);
            let Some(p) = p else { return false };
            let cp = cparts.get(i);
            let Some(cp) = cp else { return true };
            let pn = !p.is_empty() && p.bytes().all(|c| c.is_ascii_digit());
            let cn = !cp.is_empty() && cp.bytes().all(|c| c.is_ascii_digit());
            if pn && cn {
                let (x, y) = (
                    p.parse::<f64>().unwrap_or(f64::INFINITY),
                    cp.parse::<f64>().unwrap_or(f64::INFINITY),
                );
                if x != y {
                    return x > y;
                }
                continue;
            }
            if pn != cn {
                return !pn;
            }
            if p != cp {
                return p > cp;
            }
        }
        return false;
    }
    current.3.is_some() && candidate.3.is_none()
}

// --- marker ------------------------------------------------------------------

fn is_install_marker(v: &Value) -> bool {
    let Value::Object(m) = v else {
        return false;
    };
    let string_nonempty = |k: &str| {
        m.get(k)
            .and_then(as_str)
            .is_some_and(|s| !js_trim(&s).is_empty())
    };
    m.get("schemaVersion").and_then(crate::js::as_number) == Some(2.0)
        && string_nonempty("attemptId")
        && string_nonempty("fromVersion")
        && string_nonempty("toVersion")
        && m.get("requestedAt").is_some_and(is_iso_timestamp)
        && m.get("handoffAt").is_some_and(is_nullable_iso)
        && m.get("phase")
            .and_then(as_str)
            .is_some_and(|p| matches!(p.as_str(), "requested" | "handoff" | "failed"))
        && m.get("consecutiveFailures")
            .and_then(crate::js::as_number)
            .is_some_and(|n| n.fract() == 0.0 && n.is_finite() && n >= 0.0)
        && m.get("lastFailureAt").is_some_and(is_nullable_iso)
        && m.get("artifact").is_some_and(is_artifact_identity)
}

fn read_marker(file_path: &str) -> Value {
    let raw = match std::fs::read_to_string(file_path) {
        Ok(r) => r,
        Err(e) => {
            let mut o = Map::new();
            if e.kind() == std::io::ErrorKind::NotFound {
                o.insert("status".to_string(), Value::String("missing".to_string()));
            } else {
                o.insert("status".to_string(), Value::String("invalid".to_string()));
                o.insert("error".to_string(), Value::String(e.to_string()));
            }
            return Value::Object(o);
        }
    };
    match serde_json::from_str::<Value>(&raw) {
        Ok(parsed) => {
            let mut o = Map::new();
            if is_install_marker(&parsed) {
                o.insert("status".to_string(), Value::String("valid".to_string()));
                o.insert("marker".to_string(), parsed);
            } else {
                o.insert("status".to_string(), Value::String("invalid".to_string()));
                o.insert(
                    "error".to_string(),
                    Value::String("Marker does not match schema version 2.".to_string()),
                );
            }
            Value::Object(o)
        }
        Err(e) => {
            let mut o = Map::new();
            o.insert("status".to_string(), Value::String("invalid".to_string()));
            o.insert("error".to_string(), Value::String(e.to_string()));
            Value::Object(o)
        }
    }
}

fn write_marker(file_path: &str, marker: &Value) -> Result<(), String> {
    if !is_install_marker(marker) {
        return Err("Cannot write an invalid update install marker.".to_string());
    }
    let path = std::path::Path::new(file_path);
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let dir = dir.unwrap_or_else(|| std::path::Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let tmp = dir.join(format!(
        ".{}.{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id(),
        uuid_v4()
    ));
    let body = serde_json::to_string_pretty(marker).map_err(|e| e.to_string())? + "\n";
    let result = (|| -> std::io::Result<()> {
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        use std::io::Write;
        f.write_all(body.as_bytes())?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.map_err(|e| e.to_string())
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // OS entropy via /dev/urandom with a time+pid fallback, formatted as UUIDv4.
    let mut b = [0u8; 16];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| {
            use std::io::Read;
            f.read_exact(&mut b)
        })
        .is_ok();
    if !filled {
        let t = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        for (i, x) in b.iter_mut().enumerate() {
            *x = ((t >> ((i % 8) * 8)) as u8) ^ (std::process::id() as u8).wrapping_add(i as u8);
        }
    }
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

// --- exports -------------------------------------------------------------------

/// Port of `installMarkerMatchesHandoffExpectation` (`apps/desktop/src/updateInstallMarker.ts:39`).
pub fn install_marker_matches_handoff_expectation(
    marker: serde_json::Value,
    expected: serde_json::Value,
) -> Result<bool, String> {
    let ma = prop(&marker, "attemptId")?;
    let ea = prop(&expected, "attemptId")?;
    let id_eq = match (ma, ea) {
        (Some(a), Some(b)) => js_strict_eq(a, b),
        (None, None) => true,
        _ => false,
    };
    if !id_eq {
        return Ok(false);
    }
    artifact_identities_equal(prop(&marker, "artifact")?, prop(&expected, "artifact")?)
}

/// Port of `createUpdateInstallMarker` (`apps/desktop/src/updateInstallMarker.ts:107`).
pub fn create_update_install_marker(args: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut o = Map::new();
    o.insert("schemaVersion".to_string(), Value::from(2));
    o.insert("attemptId".to_string(), Value::String(uuid_v4()));
    for k in ["fromVersion", "toVersion", "requestedAt"] {
        o.insert(
            k.to_string(),
            prop(&args, k)?
                .cloned()
                .unwrap_or_else(json_marker_undefined),
        );
    }
    o.insert("handoffAt".to_string(), Value::Null);
    o.insert("phase".to_string(), Value::String("requested".to_string()));
    o.insert(
        "consecutiveFailures".to_string(),
        prop(&args, "consecutiveFailures")?
            .cloned()
            .unwrap_or_else(json_marker_undefined),
    );
    o.insert(
        "lastFailureAt".to_string(),
        prop(&args, "lastFailureAt")?
            .cloned()
            .unwrap_or(Value::Null),
    );
    o.insert(
        "artifact".to_string(),
        prop(&args, "artifact")?
            .cloned()
            .unwrap_or_else(json_marker_undefined),
    );
    Ok(Value::Object(o))
}

fn json_marker_undefined() -> Value {
    let mut m = Map::new();
    m.insert("$undefined".to_string(), Value::Bool(true));
    Value::Object(m)
}

/// Port of `readInstallMarker` (`apps/desktop/src/updateInstallMarker.ts:129`).
pub fn read_install_marker(file_path: String) -> Result<serde_json::Value, String> {
    Ok(read_marker(&file_path))
}

/// Port of `writeInstallMarker` (`apps/desktop/src/updateInstallMarker.ts:151`).
pub fn write_install_marker(file_path: String, marker: serde_json::Value) -> Result<(), String> {
    write_marker(&file_path, &marker)
}

/// Port of `markInstallHandoffSync` (`apps/desktop/src/updateInstallMarker.ts:178`).
pub fn mark_install_handoff_sync(
    file_path: String,
    expected: serde_json::Value,
    now_iso: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let result = read_marker(&file_path);
    let status = prop(&result, "status")?.and_then(as_str);
    if status.as_deref() != Some("valid") {
        return Ok(None);
    }
    let marker = prop(&result, "marker")?.cloned().unwrap_or(Value::Null);
    if !install_marker_matches_handoff_expectation(marker.clone(), expected)?
        || prop(&marker, "phase")?.and_then(as_str).as_deref() == Some("failed")
    {
        return Ok(None);
    }
    if prop(&marker, "handoffAt")?.is_some_and(|v| !v.is_null()) {
        return Ok(
            if prop(&marker, "phase")?.and_then(as_str).as_deref() == Some("handoff") {
                Some(marker)
            } else {
                None
            },
        );
    }
    let mut m = marker.as_object().cloned().unwrap_or_default();
    m.insert("phase".to_string(), Value::String("handoff".to_string()));
    let stamp = match now_iso {
        Some(v) if !is_undefined(&v) => v,
        _ => Value::String(iso(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs_f64() * 1000.0)
            .unwrap_or(0.0))?),
    };
    m.insert("handoffAt".to_string(), stamp);
    let marker = Value::Object(m);
    write_marker(&file_path, &marker)?;
    Ok(Some(marker))
}

/// Port of `recordInstallMarkerFailureSync` (`apps/desktop/src/updateInstallMarker.ts:205`).
pub fn record_install_marker_failure_sync(
    file_path: String,
    expected: serde_json::Value,
    now_iso: String,
) -> Result<serde_json::Value, String> {
    let result = read_marker(&file_path);
    let status = prop(&result, "status")?.and_then(as_str);
    if status.as_deref() != Some("valid") {
        return Ok(result);
    }
    let marker = prop(&result, "marker")?.cloned().unwrap_or(Value::Null);
    if !install_marker_matches_handoff_expectation(marker.clone(), expected)? {
        return Ok(single_status("mismatch"));
    }
    if prop(&marker, "phase")?.and_then(as_str).as_deref() == Some("failed") {
        let mut o = Map::new();
        o.insert(
            "status".to_string(),
            Value::String("already-failed".to_string()),
        );
        o.insert("marker".to_string(), marker);
        return Ok(Value::Object(o));
    }
    let mut m = marker.as_object().cloned().unwrap_or_default();
    m.insert("phase".to_string(), Value::String("failed".to_string()));
    let cf = prop(&marker, "consecutiveFailures")?
        .and_then(crate::js::as_number)
        .unwrap_or(0.0)
        + 1.0;
    m.insert("consecutiveFailures".to_string(), crate::js::number(cf));
    m.insert("lastFailureAt".to_string(), Value::String(now_iso));
    let marker = Value::Object(m);
    match write_marker(&file_path, &marker) {
        Ok(()) => {
            let mut o = Map::new();
            o.insert("status".to_string(), Value::String("recorded".to_string()));
            o.insert("marker".to_string(), marker);
            Ok(Value::Object(o))
        }
        Err(e) => {
            let mut o = Map::new();
            o.insert(
                "status".to_string(),
                Value::String("write-failed".to_string()),
            );
            o.insert("marker".to_string(), marker);
            o.insert("error".to_string(), Value::String(e));
            Ok(Value::Object(o))
        }
    }
}

fn single_status(s: &str) -> Value {
    let mut o = Map::new();
    o.insert("status".to_string(), Value::String(s.to_string()));
    Value::Object(o)
}

/// Port of `resolveInstallMarkerOutcome` (`apps/desktop/src/updateInstallMarker.ts:235`).
pub fn resolve_install_marker_outcome(
    marker: serde_json::Value,
    current_version: String,
    now_iso: String,
) -> Result<String, String> {
    const STALE_MS: f64 = 7.0 * 24.0 * 60.0 * 60.0 * 1000.0;
    if !is_install_marker(&marker) || !is_iso_timestamp(&Value::String(now_iso.clone())) {
        return Ok("invalid".to_string());
    }
    let to_version = prop(&marker, "toVersion")?
        .and_then(as_str)
        .unwrap_or_default();
    if !is_version_newer(&current_version, &to_version) {
        return Ok("success".to_string());
    }
    let requested = prop(&marker, "requestedAt")?
        .and_then(as_str)
        .unwrap_or_default();
    if js_date_parse(&now_iso) - js_date_parse(&requested) > STALE_MS {
        return Ok("stale".to_string());
    }
    if prop(&marker, "phase")?.and_then(as_str).as_deref() == Some("failed") {
        return Ok("already-failed".to_string());
    }
    Ok("failure".to_string())
}

/// Port of `clearInstallMarker` (`apps/desktop/src/updateInstallMarker.ts:255`).
pub fn clear_install_marker(file_path: String) -> Result<(), String> {
    match std::fs::remove_file(&file_path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
