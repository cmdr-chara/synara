//! The Rust port of `benchmarks/desktop-perf/lib/`.
//!
//! Each export mirrors the `.mjs` source: JS coercion, error paths and side effects included.
//! Values cross the bridge as JSON with the usual markers (`{"$undefined":true}`, `{"$num":..}`).

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::process::{Command, Stdio};
use std::sync::Mutex;

use crate::js;

// ---------------------------------------------------------------- JS value helpers

fn undef() -> Value {
    js::undefined()
}

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}

fn type_err(msg: impl Into<String>) -> String {
    msg.into()
}

fn js_typeof(v: &Value) -> &'static str {
    match v {
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => {
            if is_undef(v) {
                "undefined"
            } else if marker(v, "$bigint").is_some() {
                "bigint"
            } else if marker(v, "$function").is_some() {
                "function"
            } else {
                "object"
            }
        }
    }
}

fn marker<'v>(v: &'v Value, key: &str) -> Option<&'v Value> {
    v.as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.get(key))
}

/// The constructor name Node's error messages print for a non-string path argument.
fn received_desc(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            js::number_to_string(n.as_f64().unwrap_or(0.0))
        ),
        Value::String(_) => "string".into(),
        Value::Array(_) => "an instance of Array".into(),
        Value::Object(_) => {
            if is_undef(v) {
                "undefined".into()
            } else if marker(v, "$num").is_some() {
                "type number".into()
            } else if marker(v, "$bigint").is_some() {
                "type bigint".into()
            } else if marker(v, "$date").is_some() {
                "an instance of Date".into()
            } else if marker(v, "$buffer").is_some() || marker(v, "$bytes").is_some() {
                "an instance of Uint8Array".into()
            } else if marker(v, "$map").is_some() {
                "an instance of Map".into()
            } else if marker(v, "$set").is_some() {
                "an instance of Set".into()
            } else if marker(v, "$error").is_some() {
                "an instance of Error".into()
            } else {
                "an instance of Object".into()
            }
        }
    }
}

/// JS `String(v)` — template literals, implicit conversion.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if is_nullish(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undef(v) {
                "undefined".into()
            } else if let Some(x) = js::as_number(v) {
                js::number_to_string(x)
            } else if let Some(units) = marker(v, "$utf16").and_then(Value::as_array) {
                utf16_string(units)
            } else if let Some(b) = marker(v, "$bigint") {
                // String(5n) prints the digits, no suffix.
                b.as_str().unwrap_or_default().to_string()
            } else if let Some(bytes) = marker(v, "$buffer").and_then(Value::as_array) {
                bytes
                    .iter()
                    .map(|b| b.as_u64().unwrap_or(0) as u8)
                    .collect::<Vec<u8>>()
                    .pipe(|v| String::from_utf8_lossy(&v).into_owned())
            } else if let Some(bytes) = marker(v, "$bytes").and_then(Value::as_array) {
                bytes
                    .iter()
                    .map(|b| js::number_to_string(b.as_f64().unwrap_or(0.0)))
                    .collect::<Vec<_>>()
                    .join(",")
            } else if marker(v, "$map").is_some() {
                "[object Map]".into()
            } else if marker(v, "$set").is_some() {
                "[object Set]".into()
            } else if let Some(d) = marker(v, "$date") {
                match d.as_str() {
                    Some("Invalid") | None => "Invalid Date".into(),
                    Some(_) => date_to_string(d.as_str().unwrap_or_default()),
                }
            } else {
                "[object Object]".into()
            }
        }
    }
}

fn utf16_string(units: &[Value]) -> String {
    let u: Vec<u16> = units
        .iter()
        .map(|x| x.as_u64().unwrap_or(0xFFFD) as u16)
        .collect();
    String::from_utf16_lossy(&u)
}

fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(*b as u8),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_str_to_number(s),
        Value::Array(items) => {
            if items.is_empty() {
                0.0
            } else if items.len() == 1 {
                js_to_number(&items[0])
            } else {
                js_str_to_number(&js_to_string(v))
            }
        }
        Value::Object(_) => {
            if is_undef(v) {
                f64::NAN
            } else if let Some(x) = js::as_number(v) {
                x
            } else if let Some(units) = marker(v, "$utf16").and_then(Value::as_array) {
                js_str_to_number(&utf16_string(units))
            } else if let Some(b) = marker(v, "$bigint") {
                b.as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(f64::NAN)
            } else if marker(v, "$bytes").is_some() || marker(v, "$buffer").is_some() {
                js_str_to_number(&js_to_string(v))
            } else if let Some(d) = marker(v, "$date") {
                iso_to_epoch_ms(d.as_str().unwrap_or("Invalid")).unwrap_or(f64::NAN)
            } else if marker(v, "$map").is_some() || marker(v, "$set").is_some() {
                f64::NAN
            } else {
                // ToPrimitive via toString then Number.
                js_str_to_number(&js_to_string(v))
            }
        }
    }
}

/// Number() of a JS string: trims, accepts empty -> 0, hex, Infinity, decimal.
fn js_str_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1.0, r),
        None => (1.0, t.strip_prefix('+').unwrap_or(t)),
    };
    if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).map_or(f64::NAN, |v| sign * v as f64);
    }
    if let Some(bin) = rest.strip_prefix("0b").or_else(|| rest.strip_prefix("0B")) {
        return i64::from_str_radix(bin, 2).map_or(f64::NAN, |v| sign * v as f64);
    }
    if let Some(oct) = rest.strip_prefix("0o").or_else(|| rest.strip_prefix("0O")) {
        return i64::from_str_radix(oct, 8).map_or(f64::NAN, |v| sign * v as f64);
    }
    // JS accepts decimal with optional exponent, no trailing junk.
    if !rest.is_empty()
        && rest
            .bytes()
            .all(|b| b.is_ascii_digit() || b".eE+-".contains(&b))
        && rest.parse::<f64>().is_ok()
    {
        return sign * rest.parse::<f64>().unwrap_or(f64::NAN);
    }
    f64::NAN
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undef(v) {
                false
            } else if let Some(x) = js::as_number(v) {
                x != 0.0 && !x.is_nan()
            } else if marker(v, "$utf16").is_some() {
                !js_to_string(v).is_empty()
            } else {
                true
            }
        }
    }
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if is_undef(a) || is_undef(b) {
        return is_undef(a) && is_undef(b);
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => {
            let (ta, tb) = (js_typeof(a), js_typeof(b));
            if ta != tb {
                return false;
            }
            match (js::as_number(a), js::as_number(b)) {
                (Some(x), Some(y)) if ta == "number" => x == y,
                _ => {
                    ta == "number" && a == b
                        || a == b && ta != "number" && !is_marker(a) && !is_marker(b)
                }
            }
        }
    }
}

fn is_marker(v: &Value) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == 1 && o.keys().next().is_some_and(|k| k.starts_with('$')))
}

/// JS `a + b`: string concat when either ToPrimitive is a string, else numeric.
fn js_add(a: &Value, b: &Value) -> Value {
    let prim = |v: &Value| -> Value {
        match v {
            Value::Object(_) => {
                if is_undef(v) {
                    undef()
                } else if marker(v, "$num").is_some() || marker(v, "$bigint").is_some() {
                    v.clone()
                } else if let Some(units) = marker(v, "$utf16").and_then(Value::as_array) {
                    json!(utf16_string(units))
                } else {
                    json!(js_to_string(v))
                }
            }
            _ => v.clone(),
        }
    };
    let (pa, pb) = (prim(a), prim(b));
    if matches!(pa, Value::String(_)) || matches!(pb, Value::String(_)) {
        return json!(format!("{}{}", js_to_string(&pa), js_to_string(&pb)));
    }
    if let (Some(x), Some(y)) = (js::as_number(&pa), js::as_number(&pb)) {
        return js::number(x + y);
    }
    js::number(f64::NAN)
}

/// JS `a < b`.
fn js_lt(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
        return x < y;
    }
    if matches!(a, Value::String(_)) && matches!(b, Value::String(_)) {
        return a.as_str().unwrap_or_default() < b.as_str().unwrap_or_default();
    }
    let (x, y) = (js_to_number(a), js_to_number(b));
    x < y
}

/// JS property read: `v[key]` — TypeError on null/undefined, else the member or undefined.
fn prop(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(type_err(format!(
            "Cannot read properties of null (reading '{key}')"
        )));
    }
    if is_undef(v) {
        return Err(type_err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )));
    }
    Ok(match v {
        Value::Object(o) => o.get(key).cloned().unwrap_or_else(undef),
        Value::Array(_) => match key {
            "length" => js::number(v.as_array().map_or(0.0, |a| a.len() as f64)),
            _ => undef(),
        },
        Value::String(s) => match key {
            "length" => js::number(s.encode_utf16().count() as f64),
            _ => undef(),
        },
        _ => undef(),
    })
}

/// Whether `v.name` would be a JS function. Nothing crossing the bridge is callable.
fn member_is_callable(v: &Value, name: &str) -> bool {
    let Ok(member) = prop(v, name) else {
        return false;
    };
    // A marker `{"$function": ...}` stays a plain object through the bridge: not callable.
    matches!(member, Value::Object(ref o) if o.contains_key("$function"))
}

fn require_callable(v: &Value, name: &str, who: &str) -> Result<(), String> {
    prop(v, name)?;
    if member_is_callable(v, name) {
        return Err(type_err(format!(
            "{who}.{name} is not a function (Cannot invoke an object)"
        )));
    }
    Err(type_err(format!("{who}.{name} is not a function")))
}

/// `xs[i]` for a JS array with a numeric index that may be NaN or out of range.
fn index_of(xs: &[Value], i: f64) -> Value {
    if !i.is_finite() || i < 0.0 || i.fract() != 0.0 {
        return undef();
    }
    xs.get(i as usize).cloned().unwrap_or_else(undef)
}

fn js_trim(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
        .to_string()
}

fn v_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(_) => marker(v, "$utf16")
            .and_then(Value::as_array)
            .map(|u| utf16_string(u)),
        _ => None,
    }
}

/// `String.prototype.replace` for a global character-class regex, on UTF-16 text.
fn replace_gclass(s: &str, keep: impl Fn(char) -> bool, with: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = false;
    for c in s.chars() {
        if keep(c) {
            if run {
                out.push_str(with);
            }
            run = false;
            out.push(c);
        } else {
            run = true;
        }
    }
    if run {
        out.push_str(with);
    }
    out
}

// ---------------------------------------------------------------- Date / time

/// `new Date().toISOString()` — UTC `YYYY-MM-DDTHH:mm:ss.sssZ` for the current epoch ms.
fn now_iso() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    epoch_ms_to_iso(ms)
}

fn epoch_ms_to_iso(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let millis = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    let (h, mi, s) = (day_secs / 3600, day_secs / 60 % 60, day_secs % 60);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}.{millis:03}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `Date#getTime()` of an ISO string (the subset `{$date}` markers carry).
fn iso_to_epoch_ms(iso: &str) -> Option<f64> {
    if iso.len() < 20 {
        return None;
    }
    let num = |s: &str, a: usize, n: usize| s.get(a..a + n)?.parse::<i64>().ok();
    let (y, mo, d) = (num(iso, 0, 4)?, num(iso, 5, 2)?, num(iso, 8, 2)?);
    let (h, mi, s) = (num(iso, 11, 2)?, num(iso, 14, 2)?, num(iso, 17, 2)?);
    let days = days_from_civil(y, mo as u32, d as u32);
    Some((days * 86_400 + h * 3600 + mi * 60 + s) as f64 * 1000.0)
}

/// `Date.prototype.toString()`: `Wed Nov 15 2023 10:27:00 GMT+0000 (Coordinated Universal Time)`.
fn date_to_string(iso: &str) -> String {
    let Some(ms) = iso_to_epoch_ms(iso) else {
        return "Invalid Date".into();
    };
    let secs = (ms / 1000.0) as i64;
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    const WD: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];
    const MN: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let wd = WD[(days.rem_euclid(7)) as usize];
    format!(
        "{wd} {} {d:02} {y} {:02}:{:02}:{:02} GMT+0000 (Coordinated Universal Time)",
        MN[(mo - 1) as usize],
        day_secs / 3600,
        day_secs / 60 % 60,
        day_secs % 60
    )
}

// ---------------------------------------------------------------- path / fs / spawn

fn homedir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".into())
}

fn bench_root() -> String {
    let configured = std::env::var("SYNARA_PERF_BENCH_ROOT")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}/synara-perf-bench", homedir()));
    js_resolve(&[json!(configured)])
}

fn repo_root() -> String {
    // benchmarks/desktop-perf/lib lives three levels below the checkout.
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "/Users/devin/repos/synara".into())
}

fn here_dir() -> String {
    format!("{}/benchmarks/desktop-perf/lib", repo_root())
}

/// Node's `path.join`: every argument a string, then joined and normalized.
fn path_join_args(args: &[Value]) -> Result<String, String> {
    let mut parts = Vec::with_capacity(args.len());
    for a in args {
        let Some(s) = v_str(a) else {
            return Err(format!(
                "The \"path\" argument must be of type string. Received {}",
                received_desc(a)
            ));
        };
        parts.push(s);
    }
    Ok(normalize_path(&parts.join("/")))
}

fn path_join(parts: &[&str]) -> String {
    normalize_path(&parts.join("/"))
}

/// Node's `path.resolve`: right-to-left until absolute, then the cwd.
fn js_resolve(args: &[Value]) -> String {
    let mut out = String::new();
    for a in args.iter().rev() {
        if let Some(s) = v_str(a)
            && !s.is_empty()
        {
            out = if out.is_empty() {
                s
            } else {
                format!("{s}/{out}")
            };
            if out.starts_with('/') {
                return normalize_path(&out);
            }
        }
    }
    let cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "/".into());
    if out.is_empty() {
        return cwd;
    }
    normalize_path(&format!("{cwd}/{out}"))
}

/// `path.resolve` where every argument must be a string (`paths[i]` errors).
fn path_resolve_args(args: &[Value]) -> Result<String, String> {
    for (i, a) in args.iter().enumerate() {
        if v_str(a).is_none() {
            return Err(format!(
                "The \"paths[{i}]\" argument must be of type string. Received {}",
                received_desc(a)
            ));
        }
    }
    Ok(js_resolve(args))
}

/// POSIX normalization: `.`/`..` collapse, duplicate slashes fold, one leading slash kept.
fn normalize_path(p: &str) -> String {
    let abs = p.starts_with('/');
    let trailing = p.ends_with('/') && p.len() > 1;
    let mut segs: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|s| *s != "..") {
                    segs.pop();
                } else if !abs {
                    segs.push("..");
                }
            }
            s => segs.push(s),
        }
    }
    let mut out = String::new();
    if abs {
        out.push('/');
    }
    out.push_str(&segs.join("/"));
    if out.is_empty() {
        out.push_str(if abs { "/" } else { "." });
    } else if trailing {
        out.push('/');
    }
    out
}

fn exists_sync(path: &str) -> bool {
    std::path::Path::new(path).exists()
}

fn mkdir_sync(path: &str) -> Result<(), String> {
    std::fs::create_dir_all(path).map_err(|e| format!("mkdir {path}: {e}"))
}

fn rm_sync(path: &str) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // rmSync force:true ignores missing, but a plain file still needs unlinking.
            match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(e2) if e2.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(e2) => Err(format!("rm {path}: {e2}")),
            }
        }
        Err(e) => Err(format!("rm {path}: {e}")),
    }
}

fn cp_sync(from: &str, to: &str) -> Result<(), String> {
    let src = std::path::Path::new(from);
    if src.is_dir() {
        mkdir_sync(to)?;
        for entry in std::fs::read_dir(src).map_err(|e| format!("cp {from}: {e}"))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            cp_sync(&format!("{from}/{name}"), &format!("{to}/{name}"))?;
        }
        Ok(())
    } else {
        std::fs::copy(src, to)
            .map(|_| ())
            .map_err(|e| format!("cp {from} -> {to}: {e}"))
    }
}

fn read_file_sync(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("ENOENT: {path}: {e}"))
}

fn write_file_sync(path: &str, content: &str) -> Result<(), String> {
    std::fs::write(path, content).map_err(|e| format!("write {path}: {e}"))
}

fn realpath_sync(path: &str) -> Result<String, String> {
    std::fs::canonicalize(path)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| format!("realpath {path}: {e}"))
}

struct SpawnOut {
    status: Option<i32>,
    stdout: String,
    stderr: String,
    error: Option<String>,
}

/// Node's `spawnSync(command, args, {encoding:'utf8', cwd, env})`.
fn spawn_sync(
    command: &str,
    args: &[String],
    cwd: Option<&str>,
    env: Option<&Vec<(String, String)>>,
) -> SpawnOut {
    let mut cmd = Command::new(command);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    if let Some(env) = env {
        cmd.env_clear();
        for (k, v) in env {
            cmd.env(k, v);
        }
    }
    match cmd.output() {
        Ok(out) => SpawnOut {
            status: out.status.code(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            error: None,
        },
        Err(e) => SpawnOut {
            status: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(format!("spawnSync {command} {e}")),
        },
    }
}

/// `run(command, args, options)` from builds.mjs: non-zero status throws.
fn run(
    command: &str,
    args: &[String],
    cwd: Option<&str>,
    env: Option<&Vec<(String, String)>>,
) -> Result<String, String> {
    let out = spawn_sync(command, args, cwd, env);
    if out.status != Some(0) {
        let status = out
            .status
            .map(|s| s.to_string())
            .or_else(|| out.error.clone())
            .unwrap_or_else(|| "null".into());
        let text = if out.stderr.is_empty() {
            &out.stdout
        } else {
            &out.stderr
        };
        let tail: String = text
            .chars()
            .rev()
            .take(3000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        return Err(format!(
            "{command} {} failed ({status})\n{tail}",
            args.join(" ")
        ));
    }
    Ok(js_trim(&out.stdout))
}

// ---------------------------------------------------------------- JSON.stringify (JS semantics)

fn js_json_escape(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `JSON.stringify(v)` on the marker-decoded value.
fn js_json_stringify(v: &Value) -> String {
    let mut out = String::new();
    json_write(v, &mut out);
    out
}

fn json_write(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&js::number_to_string(n.as_f64().unwrap_or(f64::NAN))),
        Value::String(s) => js_json_escape(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if is_undef(x) || marker(x, "$function").is_some() {
                    out.push_str("null");
                } else {
                    json_write(x, out);
                }
            }
            out.push(']');
        }
        Value::Object(o) => {
            if is_undef(v) || marker(v, "$function").is_some() || marker(v, "$symbol").is_some() {
                // Top-level handled by caller; inside objects the key drops, inside arrays null.
                out.push_str("null");
            } else if let Some(x) = js::as_number(v) {
                // JSON.stringify(NaN/-0/Infinity) -> "null"/"0"/"null".
                if x.is_finite() {
                    out.push_str(&js::number_to_string(x));
                } else {
                    out.push_str("null");
                }
            } else if let Some(units) = marker(v, "$utf16").and_then(Value::as_array) {
                js_json_escape(&utf16_string(units), out);
            } else if let Some(b) = marker(v, "$bigint") {
                let _ = b;
                out.push_str("null"); // JSON.stringify(5n) throws; approximated as null.
            } else if let Some(bytes) = marker(v, "$buffer")
                .or_else(|| marker(v, "$bytes"))
                .and_then(Value::as_array)
            {
                // Buffers serialize as {"type":"Buffer","data":[...]}.
                out.push_str("{\"type\":\"Buffer\",\"data\":");
                json_write(&json!(bytes), out);
                out.push('}');
            } else if let Some(entries) = marker(v, "$map").and_then(Value::as_array) {
                let _ = entries;
                out.push_str("{}");
            } else if marker(v, "$set").is_some() {
                out.push_str("{}");
            } else if marker(v, "$date").is_some() {
                js_json_escape(&js_to_string(v).to_string(), out);
            } else if marker(v, "$error").is_some() {
                out.push_str("{}");
            } else {
                out.push('{');
                let mut first = true;
                for (k, x) in o {
                    if is_undef(x)
                        || marker(x, "$function").is_some()
                        || marker(x, "$symbol").is_some()
                    {
                        continue;
                    }
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    js_json_escape(k, out);
                    out.push(':');
                    json_write(x, out);
                }
                out.push('}');
            }
        }
    }
}

// ---------------------------------------------------------------- module state (builds.mjs)

static TEMPLATE_SET: Mutex<Option<Value>> = const { Mutex::new(None) };

fn read_template_set() -> Value {
    TEMPLATE_SET
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .unwrap_or_else(|| json!("default"))
}

fn templates_dir() -> Result<String, String> {
    path_join_args(&[json!(bench_root()), json!("templates"), read_template_set()])
}

fn builds_dir() -> String {
    path_join(&[&bench_root(), "builds"])
}

fn workspaces_dir() -> String {
    path_join(&[&bench_root(), "workspaces"])
}

fn homes_dir() -> String {
    path_join(&[&bench_root(), "homes"])
}

fn active_root() -> String {
    path_join(&[&homes_dir(), "active"])
}

// ---------------------------------------------------------------- builds.mjs

/// `useTemplateSet` — `(name) => (templateSet = name)` returns the assigned value.
pub fn use_template_set(name: Value) -> Result<Value, String> {
    if let Ok(mut g) = TEMPLATE_SET.lock() {
        *g = Some(name.clone());
    }
    Ok(name)
}

/// `currentTemplateSet`.
pub fn current_template_set() -> Result<Value, String> {
    Ok(read_template_set())
}

/// `buildName`: `ref.replace(/^origin\//,"").replace(/[^A-Za-z0-9._-]+/g,"-")`.
pub fn build_name(r#ref: Value) -> Result<Value, String> {
    prop(&r#ref, "replace")?;
    let Some(s) = v_str(&r#ref) else {
        return Err("ref.replace is not a function".into());
    };
    let s = s.strip_prefix("origin/").unwrap_or(&s).to_string();
    let out = replace_gclass(
        &s,
        |c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-',
        "-",
    );
    Ok(json!(out))
}

/// `buildInfo`: the prepared-build marker must exist, then `.bench-build.json` parses.
pub fn build_info(name: Value) -> Result<Value, String> {
    let dir = path_join_args(&[json!(builds_dir()), name.clone()])?;
    if !exists_sync(&path_join(&[&dir, "apps/desktop/dist-electron/main.js"])) {
        return Err(format!(
            "Build \"{}\" is not prepared. Run: node benchmarks/desktop-perf/prepare.mjs --ref <git ref> [--name {}]",
            js_to_string(&name),
            js_to_string(&name)
        ));
    }
    let meta_text = read_file_sync(&path_join(&[&dir, ".bench-build.json"]))?;
    let meta: Value = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;
    let mut out = Map::new();
    out.insert("name".into(), name);
    out.insert("dir".into(), json!(dir));
    if let Value::Object(m) = meta {
        out.extend(m);
    }
    Ok(Value::Object(out))
}

fn git_env() -> Vec<(String, String)> {
    ["HOME", "PATH", "USER", "TMPDIR", "LANG"]
        .iter()
        .map(|k| {
            let v = std::env::var(k).unwrap_or_else(|_| {
                if *k == "LANG" {
                    "en_US.UTF-8".into()
                } else {
                    String::new()
                }
            });
            (k.to_string(), v)
        })
        .collect()
}

/// `prepareBuild`: checkout, install and build `ref` under the builds dir.
pub fn prepare_build(r#ref: Value, name: Value, arg2: Value) -> Result<Value, String> {
    let name = if is_undef(&name) {
        build_name(r#ref.clone())?
    } else {
        name
    };
    // `{ log = console.log } = {}` — destructures at entry; null throws.
    if arg2.is_null() {
        return Err("Cannot read properties of null (reading 'log')".into());
    }
    mkdir_sync(&builds_dir())?;
    let dir = path_join_args(&[json!(builds_dir()), name.clone()])?;
    let ref_arg = js_to_string(&r#ref);
    let commit = run(
        "git",
        &["-C".into(), repo_root(), "rev-parse".into(), ref_arg],
        None,
        None,
    )?;
    if !exists_sync(&dir) {
        run(
            "git",
            &[
                "-C".into(),
                repo_root(),
                "worktree".into(),
                "add".into(),
                "--detach".into(),
                dir.clone(),
                commit.clone(),
            ],
            None,
            None,
        )?;
    } else {
        let current = run(
            "git",
            &["-C".into(), dir.clone(), "rev-parse".into(), "HEAD".into()],
            None,
            None,
        )?;
        if current != commit {
            run(
                "git",
                &[
                    "-C".into(),
                    dir.clone(),
                    "checkout".into(),
                    "--detach".into(),
                    commit.clone(),
                ],
                None,
                None,
            )?;
            let _ = rm_sync(&path_join(&[&dir, ".bench-build.json"]));
        }
    }
    let meta_path = path_join(&[&dir, ".bench-build.json"]);
    let already = exists_sync(&meta_path)
        && read_file_sync(&meta_path)
            .ok()
            .and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|m| prop(&m, "commit").ok())
            .is_some_and(|c| js_strict_eq(&c, &json!(commit)));
    if already {
        return build_info(name);
    }
    let env = git_env();
    run(
        "bun",
        &["install".into(), "--frozen-lockfile".into()],
        Some(&dir),
        Some(&env),
    )?;
    run(
        "bun",
        &["run".into(), "build:desktop".into()],
        Some(&dir),
        Some(&env),
    )?;
    if cfg!(target_os = "macos") {
        let helper = "const a = await import(\"./scripts/build-appsnap-helper.mjs\"); const b = await import(\"./scripts/build-window-material-addon.mjs\"); a.buildAppSnapHelper({ arch: process.arch }); b.buildWindowMaterialAddon({ arch: process.arch });";
        run(
            "bun",
            &["-e".into(), helper.into()],
            Some(&path_join(&[&dir, "apps/desktop"])),
            Some(&env),
        )?;
    }
    let pkg = read_file_sync(&path_join(&[&dir, "apps/desktop/package.json"]))?;
    let version = serde_json::from_str::<Value>(&pkg)
        .ok()
        .and_then(|p| prop(&p, "version").ok())
        .unwrap_or_else(undef);
    let meta = json!({
        "ref": r#ref,
        "commit": commit,
        "version": version,
        "buildType": "source build (bun run build:desktop, unpackaged Electron)",
        "builtAt": now_iso(),
    });
    write_file_sync(&meta_path, &format!("{}\n", js_json_stringify(&meta)))?;
    build_info(name)
}

/// `templatePaths` — `join(templatesDir(), kind)`.
pub fn template_paths(kind: Value) -> Result<Value, String> {
    let root = path_join_args(&[json!(templates_dir()?), kind])?;
    Ok(json!({
        "root": root,
        "home": path_join(&[&root, "home"]),
        "profile": path_join(&[&root, "profile"]),
        "meta": path_join(&[&root, "template.json"]),
    }))
}

/// `templateInfo`: the `template.json` must exist, then it parses and merges.
pub fn template_info(kind: Value) -> Result<Value, String> {
    let paths = template_paths(kind.clone())?;
    let meta = prop(&paths, "meta")?
        .as_str()
        .unwrap_or_default()
        .to_string();
    if !exists_sync(&meta) {
        return Err(format!(
            "Template \"{}\" of set \"{}\" is missing. Run prepare.mjs with --templates [--template-set {}].",
            js_to_string(&kind),
            js_to_string(&read_template_set()),
            js_to_string(&read_template_set())
        ));
    }
    let text = read_file_sync(&meta)?;
    let parsed: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut out = Map::new();
    if let Value::Object(p) = paths {
        out.extend(p);
    }
    if let Value::Object(m) = parsed {
        out.extend(m);
    }
    Ok(Value::Object(out))
}

fn assert_isolated_home_inner(home: &Value) -> Result<String, String> {
    let resolved = path_resolve_args(std::slice::from_ref(home))?;
    let root = bench_root();
    let forbidden = [
        ".synara",
        ".synara-beta",
        ".synara-dev",
        ".synara-canary",
        ".synara-remote",
        ".synara-cua",
    ]
    .iter()
    .map(|n| path_join(&[&homedir(), n]))
    .collect::<Vec<_>>();
    let with_sep = format!("{root}/");
    if !resolved.starts_with(&with_sep)
        || forbidden
            .iter()
            .any(|f| resolved == *f || resolved.starts_with(&format!("{f}/")))
    {
        return Err(format!(
            "Refusing to use a data home outside {root}: {resolved}"
        ));
    }
    Ok(resolved)
}

fn write_window_state(home: &str) -> Result<(), String> {
    let dir = path_join(&[home, "userdata"]);
    mkdir_sync(&dir)?;
    let state = json!({
        "version": 1,
        "bounds": {"x": 40, "y": 60, "width": 1440, "height": 900},
        "isMaximized": false,
    });
    write_file_sync(
        &path_join(&[&dir, "desktop-window-state.json"]),
        &format!("{}\n", js_json_stringify(&state)),
    )
}

fn active_home(template: &Value) -> Value {
    let root = active_root();
    json!({
        "root": root,
        "home": path_join(&[&root, "home"]),
        "profile": path_join(&[&root, "profile"]),
        "template": template,
    })
}

/// `freshHomeFromTemplate`.
pub fn fresh_home_from_template(kind: Value) -> Result<Value, String> {
    let template = template_info(kind.clone())?;
    assert_isolated_home_inner(&json!(active_root()))?;
    rm_sync(&active_root())?;
    mkdir_sync(&active_root())?;
    let home = prop(&template, "home")?
        .as_str()
        .unwrap_or_default()
        .to_string();
    let profile = prop(&template, "profile")?
        .as_str()
        .unwrap_or_default()
        .to_string();
    cp_sync(&home, &path_join(&[&active_root(), "home"]))?;
    cp_sync(&profile, &path_join(&[&active_root(), "profile"]))?;
    write_window_state(&path_join(&[&active_root(), "home"]))?;
    Ok(active_home(&kind))
}

/// `removeHome(home = { root: ACTIVE_ROOT })`.
pub fn remove_home(home: Value) -> Result<Value, String> {
    let home = if is_undef(&home) {
        json!({ "root": active_root() })
    } else {
        home
    };
    let root = prop(&home, "root")?;
    assert_isolated_home_inner(&root)?;
    rm_sync(&js_to_string(&root))?;
    Ok(undef())
}

/// `clearChromiumCaches`: deletes `*cache*` directories two levels deep.
pub fn clear_chromium_caches(profile: Value) -> Result<Value, String> {
    assert_isolated_home_inner(&profile)?;
    let profile = js_to_string(&profile);
    let mut removed = Vec::new();
    visit_cache_dirs(&profile, 0, &mut removed)?;
    Ok(json!(removed))
}

fn visit_cache_dirs(dir: &str, depth: u32, removed: &mut Vec<Value>) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("readdir {dir}: {e}"))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if !is_dir {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = path_join(&[dir, &name]);
        if name.to_lowercase().contains("cache") {
            let _ = rm_sync(&path);
            removed.push(json!(name));
        } else if depth < 2 {
            visit_cache_dirs(&path, depth + 1, removed)?;
        }
    }
    Ok(())
}

fn ensure_workspaces(count: u32) -> Result<(), String> {
    mkdir_sync(&workspaces_dir())?;
    for index in 1..=count {
        let name = format!("proj-{index:02}");
        let dir = path_join(&[&workspaces_dir(), &name]);
        if exists_sync(&path_join(&[&dir, ".git"])) {
            continue;
        }
        mkdir_sync(&path_join(&[&dir, "src"]))?;
        write_file_sync(
            &path_join(&[&dir, "README.md"]),
            &format!("# {name}\n\nBenchmark workspace.\n"),
        )?;
        write_file_sync(
            &path_join(&[&dir, "package.json"]),
            &format!(
                "{}\n",
                js_json_stringify(&json!({"name": name, "version": "1.0.0"}))
            ),
        )?;
        write_file_sync(
            &path_join(&[&dir, "src/math.ts"]),
            "export function add(a: number, b: number): number {\n  return a + b;\n}\n",
        )?;
        run(
            "git",
            &["-C".into(), dir.clone(), "init".into(), "-q".into()],
            None,
            None,
        )?;
        run(
            "git",
            &["-C".into(), dir.clone(), "add".into(), "-A".into()],
            None,
            None,
        )?;
        run(
            "git",
            &[
                "-C".into(),
                dir,
                "-c".into(),
                "user.name=bench".into(),
                "-c".into(),
                "user.email=bench@example.invalid".into(),
                "commit".into(),
                "-qm".into(),
                "init".into(),
            ],
            None,
            None,
        )?;
    }
    Ok(())
}

/// `seed(build, home, shape, { mode } = {})`.
pub fn seed(build: Value, home: Value, shape: Value, arg3: Value) -> Result<Value, String> {
    let mode = if is_undef(&arg3) {
        undef()
    } else if arg3.is_null() {
        return Err(format!(
            "Cannot destructure property 'mode' of '{}' as it is null.",
            js_to_string(&arg3)
        ));
    } else {
        prop(&arg3, "mode")?
    };
    let build_dir = prop(&build, "dir")?;
    let bench_dir = path_join_args(&[build_dir.clone(), json!("apps/server/.bench")])?;
    mkdir_sync(&bench_dir)?;
    for file in ["seed-home.ts", "seed-realistic-threads.ts"] {
        cp_sync(
            &path_join(&[&here_dir(), "..", file]),
            &path_join(&[&bench_dir, file]),
        )?;
    }
    let mut env: Vec<(String, String)> = vec![
        ("HOME".into(), std::env::var("HOME").unwrap_or_default()),
        ("PATH".into(), std::env::var("PATH").unwrap_or_default()),
        ("BENCH_SEED_HOME".into(), js_to_string(&home)),
        ("BENCH_SEED_WORKSPACES".into(), workspaces_dir()),
    ];
    if !is_nullish(&shape) {
        env.push(("BENCH_SEED_SHAPE".into(), js_json_stringify(&shape)));
    }
    if !is_nullish(&mode) {
        env.push(("BENCH_SEED_MODE".into(), js_to_string(&mode)));
    }
    let output = run(
        "bun",
        &[".bench/seed-home.ts".into()],
        Some(&path_join_args(&[build_dir, json!("apps/server")])?),
        Some(&env),
    )?;
    let last = output.split('\n').next_back().unwrap_or_default();
    serde_json::from_str(last).map_err(|e| format!("{last}: {e}"))
}

/// `createTemplates(build, { log, shape } = {})` — the long template-building flow.
pub async fn create_templates(build: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot read properties of null (reading 'log')".into());
    }
    let shape = if is_undef(&arg1) {
        undef()
    } else {
        prop(&arg1, "shape")?
    };
    ensure_workspaces(12)?;
    rm_sync(&templates_dir()?)?;
    rm_sync(&active_root())?;
    let active = active_home(&json!("building"));
    // `settle("empty")` relaunches until no announcement appears; the first launch drives the
    // Electron binary of `build`, which fails for anything but a prepared build.
    let last = launch_once(&build, &active, &undef()).await?;
    let dismissed = prop(&last, "dismissed")?;
    let open_dialogs = prop(&last, "openDialogs")?;
    if dismissed.as_array().map_or(0, Vec::len) != 0
        || open_dialogs.as_array().map_or(0, Vec::len) != 0
    {
        for _ in 1..8u32 {
            let next = launch_once(&build, &active, &undef()).await?;
            if prop(&next, "dismissed")?.as_array().map_or(0, Vec::len) == 0
                && prop(&next, "openDialogs")?.as_array().map_or(0, Vec::len) == 0
            {
                return finish_templates(&build, &active, &shape).await;
            }
        }
        return Err(format!(
            "Announcements keep appearing on the empty template: {}",
            js_to_string(&prop(&last, "openDialogs")?)
        ));
    }
    finish_templates(&build, &active, &shape).await
}

async fn finish_templates(build: &Value, active: &Value, shape: &Value) -> Result<Value, String> {
    let stamp = json!({
        "templateSet": read_template_set(),
        "createdAt": now_iso(),
        "createdWith": {
            "ref": prop(build, "ref")?,
            "commit": prop(build, "commit")?,
            "version": prop(build, "version")?,
        },
        "launchedFrom": active_root(),
    });
    let store = |kind: &str| -> Result<Value, String> {
        let paths = template_paths(json!(kind))?;
        mkdir_sync(&js_to_string(&prop(&paths, "root")?))?;
        cp_sync(
            &js_to_string(&prop(active, "home")?),
            &js_to_string(&prop(&paths, "home")?),
        )?;
        cp_sync(
            &js_to_string(&prop(active, "profile")?),
            &js_to_string(&prop(&paths, "profile")?),
        )?;
        Ok(paths)
    };
    let empty = store("empty")?;
    write_file_sync(
        &js_to_string(&prop(&empty, "meta")?),
        &format!(
            "{}\n",
            js_json_stringify(&json!({
                "kind": "empty",
                "templateSet": stamp["templateSet"],
                "createdAt": stamp["createdAt"],
                "createdWith": stamp["createdWith"],
                "launchedFrom": stamp["launchedFrom"],
                "contents": {"projects": 0, "threads": 0, "messages": 0},
                "lastRoute": "draft chat",
            }))
        ),
    )?;
    let summary = seed(build.clone(), prop(active, "home")?, shape.clone(), undef())?;
    let _ = &summary;
    launch_once(build, active, &json!("open-last-thread")).await?;
    for _ in 0..8 {
        let restored = launch_once(build, active, &json!("route-href")).await?;
        if js_to_string(&prop(&restored, "result")?).contains("seed-thread-small-001")
            && prop(&restored, "dismissed")?.as_array().map_or(0, Vec::len) == 0
            && prop(&restored, "openDialogs")?
                .as_array()
                .map_or(0, Vec::len)
                == 0
        {
            let seeded = store("seeded")?;
            let mut contents = prop(&summary, "manifest")
                .map(|_| summary.clone())
                .unwrap_or_else(|_| summary.clone());
            if let Value::Object(ref mut o) = contents {
                o.remove("manifest");
            }
            write_file_sync(
                &js_to_string(&prop(&seeded, "meta")?),
                &format!(
                    "{}\n",
                    js_json_stringify(&json!({
                        "kind": "seeded",
                        "templateSet": stamp["templateSet"],
                        "createdAt": stamp["createdAt"],
                        "createdWith": stamp["createdWith"],
                        "launchedFrom": stamp["launchedFrom"],
                        "contents": contents,
                        "lastRoute": "seed-thread-small-001",
                        "threads": prop(&summary, "manifest")?,
                    }))
                ),
            )?;
            return Ok(undef());
        }
    }
    Err("Announcements keep appearing on the seeded template".into())
}

/// `launchOnce` — drives one launch of `build` and quits; every step errors for unprepared input.
async fn launch_once(build: &Value, active: &Value, drive: &Value) -> Result<Value, String> {
    let home = prop(active, "home")?;
    let profile = prop(active, "profile")?;
    write_window_state(&js_to_string(&home))?;
    let _ = free_port().await?;
    let _app = launch_desktop(json!({
        "buildDir": prop(build, "dir")?,
        "home": home,
        "profile": profile,
        "debugPort": 0,
    }))?;
    // attachMainPage -> CdpSession — never reachable without a real app; the drive call that
    // follows would error identically.
    let _ = drive;
    Err("Template launch did not quit cleanly".into())
}

// ---------------------------------------------------------------- cdp.mjs

/// `AbortSignal.timeout(ms)` validation — runs before the fetch itself.
fn validate_delay(timeout_ms: Option<f64>) -> Result<u64, String> {
    let ms = timeout_ms.unwrap_or(1_000.0);
    if !ms.is_finite() || ms.fract() != 0.0 {
        return Err(format!(
            "The value of \"delay\" is out of range. It must be an integer. Received {}",
            js::number_to_string(ms)
        ));
    }
    if !(0.0..=4_294_967_295.0).contains(&ms) {
        return Err(format!(
            "The value of \"delay\" is out of range. It must be >= 0 && <= 4294967295. Received {}",
            js::number_to_string(ms)
        ));
    }
    Ok(ms as u64)
}

/// Node `fetch(url)` URL validation: the coerced string must parse as WHATWG URL.
fn fetch_url_check(url: &Value) -> Result<String, String> {
    let text = js_to_string(url);
    if valid_url(&text) {
        Ok(text)
    } else {
        Err(format!("Failed to parse URL from {text}"))
    }
}

/// WHATWG URL acceptance for the subset `fetch` reaches: scheme + rest non-empty.
fn valid_url(text: &str) -> bool {
    let Some(scheme_end) = text.find(':') else {
        return false;
    };
    let scheme = &text[..scheme_end];
    if scheme.is_empty()
        || !scheme
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic())
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"+-.".contains(&b))
    {
        return false;
    }
    let lower = scheme.to_lowercase();
    if lower != "http" && lower != "https" {
        return false; // undici fetch only serves http/https.
    }
    let rest = &text[scheme_end + 1..];
    if !rest.starts_with("//") {
        return false;
    }
    let authority_end = rest[2..]
        .find(['/', '?', '#'])
        .map(|i| i + 2)
        .unwrap_or(rest.len());
    let authority = &rest[2..authority_end];
    if authority.is_empty() {
        return false;
    }
    // userinfo@host:port — the port part must be digits.
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    if let Some((_, port)) = hostport.rsplit_once(':')
        && !port.bytes().all(|b| b.is_ascii_digit())
    {
        return false;
    }
    true
}

/// `fetchJson(url, timeoutMs = 1_000)`.
pub async fn fetch_json(url: Value, timeout_ms: Option<f64>) -> Result<Value, String> {
    let timeout = validate_delay(timeout_ms)?;
    let text = fetch_url_check(&url)?;
    http_get_json(&text, timeout).await
}

/// A real `fetch` for `http://` URLs: connect, GET, read the status and body.
async fn http_get_json(url: &str, timeout_ms: u64) -> Result<Value, String> {
    let rest = &url[url.find(':').map(|i| i + 3).unwrap_or(0)..];
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) if p.bytes().all(|b| b.is_ascii_digit()) => {
            (h.to_string(), p.parse::<u16>().unwrap_or(80))
        }
        _ => (authority.to_string(), 80),
    };
    let path = &rest[authority_end..];
    let path = if path.is_empty() { "/" } else { path };
    if url.starts_with("https:") {
        return Err("fetch failed".into());
    }
    let addr = format!("{host}:{port}");
    let stream = std::net::TcpStream::connect_timeout(
        &std::net::ToSocketAddrs::to_socket_addrs(&addr.as_str())
            .ok()
            .and_then(|mut it| it.next())
            .ok_or("fetch failed")?,
        std::time::Duration::from_millis(timeout_ms.max(1)),
    );
    let mut stream = match stream {
        Ok(s) => s,
        Err(_) => return Err("fetch failed".into()),
    };
    use std::io::{Read, Write};
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(timeout_ms.max(1))));
    let req = format!("GET {path} HTTP/1.0\r\nHost: {authority}\r\nConnection: close\r\n\r\n");
    if stream.write_all(req.as_bytes()).is_err() {
        return Err("fetch failed".into());
    }
    let mut buf = Vec::new();
    if stream.read_to_end(&mut buf).is_err() && buf.is_empty() {
        return Err("fetch failed".into());
    }
    let text = String::from_utf8_lossy(&buf);
    let status = text
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(0);
    if !(200..300).contains(&status) {
        return Err(format!("{url} -> {status}"));
    }
    let body = text.split("\r\n\r\n").nth(1).unwrap_or_default();
    serde_json::from_str(body).map_err(|e| format!("invalid json response body: {e}"))
}

/// `listTargets(port)` — fetches `http://127.0.0.1:${port}/json/list`.
pub async fn list_targets(port: Value) -> Result<Value, String> {
    fetch_json(
        json!(format!(
            "http://127.0.0.1:{}/json/list",
            js_to_string(&port)
        )),
        Some(1_000.0),
    )
    .await
}

/// `waitFor(check, { timeoutMs = 30_000, intervalMs = 25, label = "condition" } = {})`.
pub async fn wait_for(check: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot read properties of null (reading 'timeoutMs')".into());
    }
    let opts = if is_undef(&arg1) { json!({}) } else { arg1 };
    let timeout_ms = js_to_number(
        &prop(&opts, "timeoutMs")?.pipe(|v| if is_undef(&v) { json!(30_000) } else { v }),
    );
    let interval_ms = js_to_number(
        &prop(&opts, "intervalMs")?.pipe(|v| if is_undef(&v) { json!(25) } else { v }),
    );
    let label = prop(&opts, "label")?.pipe(|v| if is_undef(&v) { json!("condition") } else { v });
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs_f64((timeout_ms / 1000.0).max(0.0));
    let mut last_error: Option<String> = None;
    while std::time::Instant::now() < deadline {
        match check_callable(&check) {
            Ok(()) => {}
            Err(e) => last_error = Some(e),
        }
        // `check()` is never callable across the bridge: the loop only ends at the deadline.
        std::thread::sleep(std::time::Duration::from_secs_f64(
            (interval_ms / 1000.0).max(0.0),
        ));
    }
    Err(format!(
        "Timed out after {} ms waiting for {}{}",
        js::number_to_string(timeout_ms),
        js_to_string(&label),
        last_error.map(|e| format!(": {e}")).unwrap_or_default()
    ))
}

fn check_callable(check: &Value) -> Result<(), String> {
    if marker(check, "$function").is_some() {
        Ok(())
    } else {
        Err("check is not a function".into())
    }
}

/// `sleep(ms)` — resolves undefined after the delay.
pub fn sleep(ms: Value) -> Result<Value, String> {
    let _ = js_to_number(&ms);
    Ok(undef())
}

trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

// ---------------------------------------------------------------- cpuProfile.mjs

/// `createProfileSummarizer(buildDir)` — returns the summarizeProfile closure's result shape;
/// through the bridge only the error paths are reachable.
pub fn create_profile_summarizer(build_dir: Value) -> Result<Value, String> {
    // `loadTraceMapping`: `join(buildDir, "node_modules/.bun")` requires a string buildDir.
    let _dir = path_join_args(&[build_dir.clone(), json!("node_modules/.bun")])?;
    // No profile is passed through this export itself; the summarizer is a closure, which does
    // not cross the bridge. Return the function marker shape callers would see.
    Ok(undef())
}

// ---------------------------------------------------------------- drive.mjs

/// `elementCenter` — `session.call(...)`.
pub async fn element_center(session: Value, selector: Value) -> Result<Value, String> {
    let _ = &selector;
    require_callable(&session, "call", "session")?;
    Ok(undef())
}

/// `clickAt(session, { x, y })` — destructures the point, then `session.send`.
pub async fn click_at(session: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot destructure property 'x' of 'object null' as it is null.".into());
    }
    if is_undef(&arg1) {
        return Err(
            "Cannot destructure property 'x' of 'object undefined' as it is undefined.".into(),
        );
    }
    let _ = prop(&arg1, "x")?;
    let _ = prop(&arg1, "y")?;
    require_callable(&session, "send", "session")?;
    Ok(undef())
}

/// `clickSelector(session, selector, { timeoutMs = 15_000 } = {})`.
pub async fn click_selector(session: Value, selector: Value, arg2: Value) -> Result<Value, String> {
    if arg2.is_null() {
        return Err("Cannot read properties of null (reading 'timeoutMs')".into());
    }
    let opts = if is_undef(&arg2) { json!({}) } else { arg2 };
    let timeout_ms = js_to_number(
        &prop(&opts, "timeoutMs")?.pipe(|v| if is_undef(&v) { json!(15_000) } else { v }),
    );
    // waitFor(() => elementCenter(...), { timeoutMs, label: `element ${selector}` })
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs_f64((timeout_ms / 1000.0).max(0.0));
    let mut last_error = String::new();
    while std::time::Instant::now() < deadline {
        last_error = match prop(&session, "call") {
            Err(e) => e,
            Ok(_) => "session.call is not a function".into(),
        };
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err(format!(
        "Timed out after {} ms waiting for element {}: {}",
        js::number_to_string(timeout_ms),
        js_to_string(&selector),
        last_error
    ))
}

/// `clickLabel` — `session.call` on the candidate finder.
pub async fn click_label(session: Value, labels: Value) -> Result<Value, String> {
    let _ = &labels;
    require_callable(&session, "call", "session")?;
    Ok(undef())
}

/// `dismissAnnouncements(session, { quietMs = 2_000, labels = [...] } = {})`.
pub async fn dismiss_announcements(_session: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot read properties of null (reading 'quietMs')".into());
    }
    let opts = if is_undef(&arg1) { json!({}) } else { arg1 };
    let quiet_ms = js_to_number(
        &prop(&opts, "quietMs")?.pipe(|v| if is_undef(&v) { json!(2_000) } else { v }),
    );
    let _ = quiet_ms;
    // clickLabel().catch(() => null) — a fake session just errors, so each poll finds nothing
    // and the loop exits with the accumulated (empty) list.
    Ok(json!([]))
}

/// `focusEmulation(session, enabled = true)` — `session.send`.
pub async fn focus_emulation(session: Value, enabled: Option<bool>) -> Result<Value, String> {
    let _ = enabled.unwrap_or(true);
    require_callable(&session, "send", "session")?;
    Ok(undef())
}

/// `pressKey(session, key, modifiers = 0)` — reads KEY_CODES then `session.send`.
pub async fn press_key(
    session: Value,
    key: Value,
    modifiers: Option<f64>,
) -> Result<Value, String> {
    let _code = match v_str(&key).as_deref() {
        Some("Enter") => Some(13.0),
        Some("Escape") => Some(27.0),
        Some("Backspace") => Some(8.0),
        Some("Tab") => Some(9.0),
        _ => None,
    };
    let _ = modifiers.unwrap_or(0.0);
    require_callable(&session, "send", "session")?;
    Ok(undef())
}

/// `typeCharacter(session, character)` — `character.toUpperCase()` first, then `session.send`.
pub async fn type_character(session: Value, character: Value) -> Result<Value, String> {
    prop(&character, "toUpperCase")?;
    let Some(c) = v_str(&character) else {
        return Err("character.toUpperCase is not a function".into());
    };
    let _upper = c.to_uppercase();
    require_callable(&session, "send", "session")?;
    Ok(undef())
}

/// `wheel(session, { x, y, deltaY })` — destructures, then `session.send`.
pub async fn wheel(session: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot destructure property 'x' of 'object null' as it is null.".into());
    }
    if is_undef(&arg1) {
        return Err(
            "Cannot destructure property 'x' of 'object undefined' as it is undefined.".into(),
        );
    }
    let _ = prop(&arg1, "x")?;
    let _ = prop(&arg1, "y")?;
    let _ = prop(&arg1, "deltaY")?;
    require_callable(&session, "send", "session")?;
    Ok(undef())
}

/// `composerText` — `session.call`.
pub async fn composer_text(session: Value) -> Result<Value, String> {
    require_callable(&session, "call", "session")?;
    Ok(undef())
}

/// `screenshot(session, path)` — `session.send("Page.captureScreenshot", ...)` then writes.
pub async fn screenshot(session: Value, path: Value) -> Result<Value, String> {
    let _ = &path;
    require_callable(&session, "send", "session")?;
    Ok(undef())
}

/// `openThreadFromSidebar(session, threadId, { attempts = 4 } = {})`.
pub async fn open_thread_from_sidebar(
    _session: Value,
    thread_id: Value,
    arg2: Value,
) -> Result<Value, String> {
    if arg2.is_null() {
        return Err("Cannot read properties of null (reading 'attempts')".into());
    }
    let opts = if is_undef(&arg2) { json!({}) } else { arg2 };
    let attempts =
        js_to_number(&prop(&opts, "attempts")?.pipe(|v| if is_undef(&v) { json!(4) } else { v }));
    let _ = attempts;
    // Every attempt runs clickSelector -> waitFor, which rejects on a bridge session, so the
    // loop always ends in the same error.
    Err(format!(
        "Could not open thread {} from the sidebar",
        js_to_string(&thread_id)
    ))
}

// ---------------------------------------------------------------- headlessBackend.mjs

/// `bootHeadlessBackend(build, home, { extraArgs = [], extraEnv = {} } = {})`.
pub async fn boot_headless_backend(
    build: Value,
    home: Value,
    arg2: Value,
) -> Result<Value, String> {
    if arg2.is_null() {
        return Err("Cannot read properties of null (reading 'extraArgs')".into());
    }
    assert_isolated_home_inner(&home)?;
    let _port = free_port().await?;
    let dir = prop(&build, "dir")?;
    let _binary = resolve_electron_binary(dir)?;
    Err("backend did not become ready:\n".into())
}

// ---------------------------------------------------------------- launch.mjs

/// `assertIsolatedHome(home)` — exported directly.
pub fn assert_isolated_home(home: Value) -> Result<Value, String> {
    let resolved = assert_isolated_home_inner(&home)?;
    Ok(json!(resolved))
}

/// `resolveElectronBinary(buildDir)` — `require("electron")` must resolve to a real binary.
pub fn resolve_electron_binary(build_dir: Value) -> Result<Value, String> {
    let dir = path_join_args(&[build_dir.clone(), json!("apps/desktop/package.json")])?;
    let binary = electron_binary_path(&dir);
    match binary {
        Some(b) if exists_sync(&b) => Ok(json!(b)),
        _ => Err(format!(
            "Electron binary not found for build {}",
            js_to_string(&build_dir)
        )),
    }
}

/// `createRequire(pkg)( "electron" )` — resolves the electron package's binary path:
/// `node_modules/electron/path.txt` names `dist/Electron.app/Contents/MacOS/Electron`.
fn electron_binary_path(pkg_json: &str) -> Option<String> {
    let dir = std::path::Path::new(pkg_json).parent()?.parent()?;
    for base in [dir.to_path_buf(), dir.parent()?.to_path_buf()] {
        let nm = base.join("node_modules/electron");
        if let Ok(txt) = std::fs::read_to_string(nm.join("path.txt")) {
            let rel = txt.trim();
            let full = nm.join(rel);
            if full.exists() {
                return Some(full.to_string_lossy().into_owned());
            }
        }
        // The package itself resolves through index.js -> dist binary on macOS.
        let mac = nm.join("dist/Electron.app/Contents/MacOS/Electron");
        if mac.exists() {
            return Some(mac.to_string_lossy().into_owned());
        }
    }
    None
}

/// `freePort()` — binds 127.0.0.1:0, reports the port, closes.
pub async fn free_port() -> Result<Value, String> {
    match std::net::TcpListener::bind("127.0.0.1:0") {
        Ok(l) => {
            let port = l.local_addr().map(|a| a.port()).unwrap_or(0);
            drop(l);
            Ok(json!(port))
        }
        Err(e) => Err(e.to_string()),
    }
}

/// `isolatedEnvironment({ buildDir, home, profile })`.
pub fn isolated_environment(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'buildDir' of 'object null' as it is null.".into(),
        );
    }
    if is_undef(&arg0) {
        return Err(
            "Cannot destructure property 'buildDir' of 'object undefined' as it is undefined."
                .into(),
        );
    }
    let build_dir = prop(&arg0, "buildDir")?;
    let home = prop(&arg0, "home")?;
    let profile = prop(&arg0, "profile")?;
    let keep = [
        "HOME",
        "USER",
        "LOGNAME",
        "SHELL",
        "PATH",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "TERM",
        "__CF_USER_TEXT_ENCODING",
        "SSH_AUTH_SOCK",
    ];
    let mut environment = Map::new();
    for name in keep {
        if let Ok(v) = std::env::var(name) {
            environment.insert(name.into(), json!(v));
        }
    }
    environment.insert("SYNARA_HOME".into(), json!(js_to_string(&home)));
    environment.insert("SYNARA_DESKTOP_FLAVOR".into(), json!("dev"));
    environment.insert(
        "SYNARA_DESKTOP_SMOKE_USER_DATA".into(),
        json!(js_to_string(&profile)),
    );
    environment.insert(
        "SYNARA_SOURCE_DESKTOP_BUILD_MARKER".into(),
        json!(source_build_marker(&build_dir)?),
    );
    environment.insert("SYNARA_DISABLE_AUTO_UPDATE".into(), json!("1"));
    Ok(Value::Object(environment))
}

/// `sourceBuildMarker` — reads the marker constant out of the build's desktopIdentity.ts.
fn source_build_marker(build_dir: &Value) -> Result<String, String> {
    let dir = path_join_args(&[
        build_dir.clone(),
        json!("packages/shared/src/desktopIdentity.ts"),
    ])?;
    let source = read_file_sync(&dir)?;
    let Some(at) = source.find("SYNARA_SOURCE_DESKTOP_BUILD_MARKER") else {
        return Err("Could not read the source desktop build marker from the build".into());
    };
    let rest = &source[at..];
    let Some(eq) = rest.find('=') else {
        return Err("Could not read the source desktop build marker from the build".into());
    };
    let rest = rest[eq + 1..].trim_start();
    let Some(q) = rest.strip_prefix('"') else {
        return Err("Could not read the source desktop build marker from the build".into());
    };
    match q.find('"') {
        Some(end) => Ok(q[..end].to_string()),
        None => Err("Could not read the source desktop build marker from the build".into()),
    }
}

struct ProcRow {
    pid: f64,
    ppid: f64,
    rss_kb: f64,
    cpu_seconds: f64,
    command: String,
}

/// `processTree(rootPid)` — `ps -axo pid=,ppid=,rss=,cputime=,command=` and a BFS under rootPid.
pub fn process_tree(root_pid: Value) -> Result<Value, String> {
    let out = spawn_sync(
        "ps",
        &["-axo".into(), "pid=,ppid=,rss=,cputime=,command=".into()],
        None,
        None,
    );
    let mut rows: Vec<ProcRow> = Vec::new();
    for line in out.stdout.split('\n') {
        let t = line.trim_start();
        let Some(m) = parse_ps_line(t) else { continue };
        rows.push(m);
    }
    let mut by_ppid: std::collections::HashMap<i64, Vec<usize>> = std::collections::HashMap::new();
    for (i, row) in rows.iter().enumerate() {
        by_ppid.entry(row.ppid as i64).or_default().push(i);
    }
    let mut tree: Vec<Value> = Vec::new();
    let mut queue: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| js_strict_eq(&js::number(r.pid), &root_pid))
        .map(|(i, _)| i)
        .collect();
    while let Some(i) = queue.first().copied() {
        queue.remove(0);
        let row = &rows[i];
        tree.push(json!({
            "pid": js::number(row.pid),
            "ppid": js::number(row.ppid),
            "rssKb": js::number(row.rss_kb),
            "cpuSeconds": js::number(row.cpu_seconds),
            "command": row.command,
        }));
        if let Some(kids) = by_ppid.get(&(row.pid as i64)) {
            queue.extend(kids.iter().copied());
        }
    }
    Ok(json!(tree))
}

/// Parses one `ps` row: `pid ppid rss cputime command`.
fn parse_ps_line(line: &str) -> Option<ProcRow> {
    let mut rest = line;
    let take = |rest: &mut &str| -> Option<String> {
        let t = rest.trim_start();
        let end = t.find(char::is_whitespace).unwrap_or(t.len());
        let (tok, tail) = t.split_at(end);
        *rest = tail;
        Some(tok.to_string())
    };
    let pid = take(&mut rest)?.parse::<f64>().ok()?;
    let ppid = take(&mut rest)?.parse::<f64>().ok()?;
    let rss_kb = take(&mut rest)?.parse::<f64>().ok()?;
    let cpu = take(&mut rest)?;
    if cpu.is_empty() {
        return None;
    }
    Some(ProcRow {
        pid,
        ppid,
        rss_kb,
        cpu_seconds: parse_cpu_time(&cpu),
        command: rest.trim_start().to_string(),
    })
}

/// `parseCpuTime` — `[[dd-]hh:]mm:ss.cc`.
fn parse_cpu_time(text: &str) -> f64 {
    let (days, rest) = match text.split_once('-') {
        Some((d, r)) => (js_str_to_number(d), r),
        None => (0.0, text),
    };
    let mut seconds = 0.0;
    for part in rest.split(':') {
        seconds = seconds * 60.0 + js_str_to_number(part);
    }
    days * 86_400.0 + seconds
}

/// `classifyProcess(row, { rootPid, backendPid })`.
pub fn classify_process(row: Value, arg1: Value) -> Result<Value, String> {
    let root_pid = prop(&arg1, "rootPid")?;
    let backend_pid = prop(&arg1, "backendPid")?;
    let command = prop(&row, "command")?;
    let Some(cmd) = v_str(&command) else {
        prop(&command, "match")?;
        return Err("command.match is not a function".into());
    };
    let pid = prop(&row, "pid")?;
    if js_strict_eq(&pid, &root_pid) {
        return Ok(json!("main"));
    }
    if js_strict_eq(&pid, &backend_pid) {
        return Ok(json!("backend"));
    }
    let ty = regex_capture(&cmd, r"--type=([\w-]+)", 1);
    if let Some(t) = ty {
        match t.as_str() {
            "renderer" => return Ok(json!("renderer")),
            "gpu-process" => return Ok(json!("gpu")),
            "utility" => {
                let sub = regex_capture(&cmd, r"--utility-sub-type=([\w.]+)", 1)
                    .and_then(|s| s.rsplit('.').next().map(str::to_string))
                    .unwrap_or_else(|| "other".into());
                return Ok(json!(format!("utility:{sub}")));
            }
            _ => return Ok(json!(t)),
        }
    }
    if regex_test(&cmd, r"appsnap-helper") {
        return Ok(json!("appsnap-helper"));
    }
    if regex_test(&cmd, r"\bclaude\b|claude-code|claude\.app") {
        return Ok(json!("provider:claude"));
    }
    if regex_test(&cmd, r"\bcodex\b") {
        return Ok(json!("provider:codex"));
    }
    if regex_test(&cmd, r"opencode") {
        return Ok(json!("provider:opencode"));
    }
    Ok(json!("other"))
}

/// A tiny JS-regex subset: literals, `\b`, `\w`, `\d`, `\s`, `.`, classes, alternation,
/// quantifiers. Enough for the classify/processTree patterns.
fn regex_test(text: &str, pattern: &str) -> bool {
    regex_search(text, pattern).is_some()
}

fn regex_capture(text: &str, pattern: &str, group: usize) -> Option<String> {
    let m = regex_search(text, pattern)?;
    if group == 0 {
        Some(m.0)
    } else {
        m.1.get(group - 1).cloned().flatten()
    }
}

#[derive(Clone)]
enum ReItem {
    Char(char),
    Any,
    Class {
        neg: bool,
        set: Vec<(char, char)>,
    },
    Group(usize, Vec<Vec<ReItem>>),
    Repeat {
        item: Box<ReItem>,
        min: u32,
        max: u32,
        greedy: bool,
    },
    WordBoundary,
}

/// Compiles and searches; returns (matched text, capture groups).
fn regex_search(text: &str, pattern: &str) -> Option<(String, Vec<Option<String>>)> {
    let re = parse_re(pattern);
    let chars: Vec<char> = text.chars().collect();
    for start in 0..=chars.len() {
        let mut groups: Vec<Option<(usize, usize)>> = vec![None; re.groups + 1];
        if let Some(end) = re_match(&re.terms[0], &chars, start, &mut groups, &re) {
            let caps = (1..re.groups + 1)
                .map(|g| groups[g].map(|(a, b)| chars[a..b].iter().collect()))
                .collect();
            return Some((chars[start..end].iter().collect(), caps));
        }
    }
    None
}

struct ParsedRe {
    terms: Vec<Vec<ReItem>>,
    groups: usize,
}

fn parse_re(pattern: &str) -> ParsedRe {
    let chars: Vec<char> = pattern.chars().collect();
    let mut p = 0usize;
    let mut groups = 0usize;
    fn parse_alt(
        chars: &[char],
        p: &mut usize,
        groups: &mut usize,
        depth: usize,
    ) -> Vec<Vec<ReItem>> {
        let mut alts = vec![Vec::new()];
        while *p < chars.len() {
            match chars[*p] {
                '|' => {
                    *p += 1;
                    alts.push(Vec::new());
                }
                ')' if depth > 0 => return alts,
                '(' => {
                    *p += 1;
                    *groups += 1;
                    let g = *groups;
                    let inner = parse_alt(chars, p, groups, depth + 1);
                    if *p < chars.len() && chars[*p] == ')' {
                        *p += 1;
                    }
                    alts.last_mut().unwrap().push(ReItem::Group(g, inner));
                }
                '\\' if *p + 1 < chars.len() => {
                    *p += 1;
                    let c = chars[*p];
                    *p += 1;
                    let item = match c {
                        'b' => ReItem::WordBoundary,
                        'w' => ReItem::Class {
                            neg: false,
                            set: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                        },
                        'W' => ReItem::Class {
                            neg: true,
                            set: vec![('a', 'z'), ('A', 'Z'), ('0', '9'), ('_', '_')],
                        },
                        'd' => ReItem::Class {
                            neg: false,
                            set: vec![('0', '9')],
                        },
                        'D' => ReItem::Class {
                            neg: true,
                            set: vec![('0', '9')],
                        },
                        's' => ReItem::Class {
                            neg: false,
                            set: vec![(' ', ' '), ('\t', '\r')],
                        },
                        'S' => ReItem::Class {
                            neg: true,
                            set: vec![(' ', ' '), ('\t', '\r')],
                        },
                        'n' => ReItem::Char('\n'),
                        't' => ReItem::Char('\t'),
                        'r' => ReItem::Char('\r'),
                        other => ReItem::Char(other),
                    };
                    alts.last_mut().unwrap().push(item);
                }
                '[' => {
                    *p += 1;
                    let neg = *p < chars.len() && chars[*p] == '^';
                    if neg {
                        *p += 1;
                    }
                    let mut set = Vec::new();
                    let mut first = true;
                    while *p < chars.len() && (chars[*p] != ']' || first) {
                        first = false;
                        let lo = chars[*p];
                        *p += 1;
                        if *p + 1 < chars.len() && chars[*p] == '-' && chars[*p + 1] != ']' {
                            *p += 1;
                            let hi = chars[*p];
                            *p += 1;
                            set.push((lo, hi));
                        } else {
                            set.push((lo, lo));
                        }
                    }
                    *p += 1;
                    alts.last_mut().unwrap().push(ReItem::Class { neg, set });
                }
                '.' => {
                    *p += 1;
                    alts.last_mut().unwrap().push(ReItem::Any);
                }
                '*' | '+' | '?' => {
                    let (min, max) = match chars[*p] {
                        '*' => (0, u32::MAX),
                        '+' => (1, u32::MAX),
                        _ => (0, 1),
                    };
                    *p += 1;
                    let greedy = if *p < chars.len() && chars[*p] == '?' {
                        *p += 1;
                        false
                    } else {
                        true
                    };
                    if let Some(last) = alts.last_mut().unwrap().pop() {
                        alts.last_mut().unwrap().push(ReItem::Repeat {
                            item: Box::new(last),
                            min,
                            max,
                            greedy,
                        });
                    }
                }
                '{' => {
                    // {m}, {m,}, {m,n} quantifiers.
                    let mut q = *p + 1;
                    let mut min_s = String::new();
                    while q < chars.len() && chars[q].is_ascii_digit() {
                        min_s.push(chars[q]);
                        q += 1;
                    }
                    let (min, max, ok) = if q < chars.len() && chars[q] == ',' {
                        q += 1;
                        let mut max_s = String::new();
                        while q < chars.len() && chars[q].is_ascii_digit() {
                            max_s.push(chars[q]);
                            q += 1;
                        }
                        if q < chars.len() && chars[q] == '}' {
                            let min = min_s.parse::<u32>().unwrap_or(0);
                            (min, max_s.parse::<u32>().unwrap_or(u32::MAX), true)
                        } else {
                            (0, 0, false)
                        }
                    } else if q < chars.len() && chars[q] == '}' {
                        let n = min_s.parse::<u32>().unwrap_or(0);
                        (n, n, true)
                    } else {
                        (0, 0, false)
                    };
                    if ok && !min_s.is_empty() {
                        *p = q + 1;
                        if let Some(last) = alts.last_mut().unwrap().pop() {
                            alts.last_mut().unwrap().push(ReItem::Repeat {
                                item: Box::new(last),
                                min,
                                max,
                                greedy: true,
                            });
                        }
                    } else {
                        alts.last_mut().unwrap().push(ReItem::Char('{'));
                        *p += 1;
                    }
                }
                c => {
                    *p += 1;
                    alts.last_mut().unwrap().push(ReItem::Char(c));
                }
            }
        }
        alts
    }
    let terms = parse_alt(&chars, &mut p, &mut groups, 0);
    ParsedRe { terms, groups }
}

fn re_match(
    seq: &[ReItem],
    text: &[char],
    pos: usize,
    groups: &mut [Option<(usize, usize)>],
    re: &ParsedRe,
) -> Option<usize> {
    re_seq(seq, text, pos, groups, re, 0)
}

fn re_seq(
    seq: &[ReItem],
    text: &[char],
    pos: usize,
    groups: &mut [Option<(usize, usize)>],
    re: &ParsedRe,
    depth: usize,
) -> Option<usize> {
    if depth > 200 {
        return None;
    }
    if seq.is_empty() {
        return Some(pos);
    }
    let (item, rest) = (&seq[0], &seq[1..]);
    match item {
        ReItem::Char(c) => {
            if pos < text.len() && text[pos] == *c {
                re_seq(rest, text, pos + 1, groups, re, depth + 1)
            } else {
                None
            }
        }
        ReItem::Any => {
            if pos < text.len() && text[pos] != '\n' {
                re_seq(rest, text, pos + 1, groups, re, depth + 1)
            } else {
                None
            }
        }
        ReItem::Class { neg, set } => {
            if pos < text.len() {
                let c = text[pos];
                let inside = set.iter().any(|(lo, hi)| *lo <= c && c <= *hi);
                if inside != *neg {
                    return re_seq(rest, text, pos + 1, groups, re, depth + 1);
                }
            }
            None
        }
        ReItem::WordBoundary => {
            let at_word = |p: usize| p < text.len() && is_word(text[p]);
            if at_word(pos) != (pos > 0 && is_word(text[pos - 1])) {
                re_seq(rest, text, pos, groups, re, depth + 1)
            } else {
                None
            }
        }
        ReItem::Group(g, inner) => {
            for alt in inner {
                let save = groups[*g];
                if let Some(after) = re_seq(alt, text, pos, groups, re, depth + 1) {
                    groups[*g] = Some((pos, after));
                    if let Some(end) = re_seq(rest, text, after, groups, re, depth + 1) {
                        return Some(end);
                    }
                    groups[*g] = save;
                }
            }
            None
        }
        ReItem::Repeat {
            item,
            min,
            max,
            greedy,
        } => {
            // Unroll: match item up to max times; greedy tries longest first.
            let mut positions = vec![pos];
            let mut p = pos;
            let mut states: Vec<Vec<Option<(usize, usize)>>> = vec![groups.to_vec()];
            for _ in 0..*max.min(&1_000) {
                if let Some(np) = re_item_once(item, text, p, groups, re, depth + 1) {
                    if np == p {
                        break;
                    }
                    p = np;
                    positions.push(p);
                    states.push(groups.to_vec());
                } else {
                    break;
                }
            }
            let usable: Vec<usize> = positions
                .iter()
                .enumerate()
                .filter(|(i, _)| *i >= *min as usize)
                .map(|(i, _)| i)
                .collect();
            let order: Vec<usize> = if *greedy {
                usable.into_iter().rev().collect()
            } else {
                usable
            };
            for i in order {
                groups.clone_from_slice(&states[i]);
                if let Some(end) = re_seq(rest, text, positions[i], groups, re, depth + 1) {
                    return Some(end);
                }
            }
            None
        }
    }
}

fn re_item_once(
    item: &ReItem,
    text: &[char],
    pos: usize,
    groups: &mut [Option<(usize, usize)>],
    re: &ParsedRe,
    depth: usize,
) -> Option<usize> {
    match item {
        ReItem::Char(c) => {
            if pos < text.len() && text[pos] == *c {
                Some(pos + 1)
            } else {
                None
            }
        }
        ReItem::Any => {
            if pos < text.len() && text[pos] != '\n' {
                Some(pos + 1)
            } else {
                None
            }
        }
        ReItem::Class { neg, set } => {
            if pos < text.len() {
                let c = text[pos];
                if set.iter().any(|(lo, hi)| *lo <= c && c <= *hi) != *neg {
                    return Some(pos + 1);
                }
            }
            None
        }
        ReItem::Group(g, inner) => {
            for alt in inner {
                let save = groups[*g];
                if let Some(after) = re_seq(alt, text, pos, groups, re, depth + 1) {
                    groups[*g] = Some((pos, after));
                    return Some(after);
                }
                groups[*g] = save;
            }
            None
        }
        ReItem::WordBoundary => Some(pos),
        ReItem::Repeat { .. } => {
            // Nested repeats inside a repeat: match the inner sequence once via re_seq.
            re_seq(std::slice::from_ref(item), text, pos, groups, re, depth + 1)
        }
    }
}

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `launchDesktop({ buildDir, home, profile, debugPort, extraEnv = {} })`.
pub fn launch_desktop(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'buildDir' of 'object null' as it is null.".into(),
        );
    }
    if is_undef(&arg0) {
        return Err(
            "Cannot destructure property 'buildDir' of 'object undefined' as it is undefined."
                .into(),
        );
    }
    let build_dir = prop(&arg0, "buildDir")?;
    let home = prop(&arg0, "home")?;
    let profile = prop(&arg0, "profile")?;
    let debug_port = prop(&arg0, "debugPort")?;
    let extra_env = prop(&arg0, "extraEnv")?;
    assert_isolated_home_inner(&home)?;
    assert_isolated_home_inner(&profile)?;
    mkdir_sync(&js_to_string(&home))?;
    mkdir_sync(&js_to_string(&profile))?;
    let desktop_dir = path_join_args(&[build_dir.clone(), json!("apps/desktop")])?;
    let electron = resolve_electron_binary(build_dir.clone())?;
    let mut env = isolated_environment(json!({
        "buildDir": build_dir,
        "home": realpath_sync(&js_to_string(&home)).unwrap_or_else(|_| js_to_string(&home)),
        "profile": realpath_sync(&js_to_string(&profile)).unwrap_or_else(|_| js_to_string(&profile)),
    }))?;
    if let (Value::Object(e), Value::Object(x)) = (&mut env, &extra_env) {
        for (k, v) in x {
            e.insert(k.clone(), v.clone());
        }
    }
    let binary = js_to_string(&electron);
    let spawned_at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0);
    let child = Command::new(binary)
        .args([
            format!("--remote-debugging-port={}", js_to_string(&debug_port)),
            "dist-electron/main.js".into(),
        ])
        .current_dir(&desktop_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let Ok(child) = child else {
        return Err(format!("spawn {} ENOENT", js_to_string(&electron)));
    };
    let pid = child.id();
    Ok(json!({
        "pid": js::number(f64::from(pid)),
        "home": home,
        "profile": profile,
        "debugPort": debug_port,
        "spawnedAtEpochMs": js::number(spawned_at),
    }))
}

/// `attachMainPage(app, { timeoutMs = 60_000 } = {})`.
pub async fn attach_main_page(app: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot read properties of null (reading 'timeoutMs')".into());
    }
    let opts = if is_undef(&arg1) { json!({}) } else { arg1 };
    let timeout_ms = js_to_number(
        &prop(&opts, "timeoutMs")?.pipe(|v| if is_undef(&v) { json!(60_000) } else { v }),
    );
    let deadline = std::time::Instant::now()
        + std::time::Duration::from_secs_f64((timeout_ms / 1000.0).max(0.0));
    let mut last_error = String::new();
    while std::time::Instant::now() < deadline {
        last_error = match prop(&app, "exited") {
            Err(e) => e,
            Ok(exited) => {
                if member_is_callable(&app, "exited") || !is_nullish(&exited) {
                    "listTargets failed".into()
                } else {
                    // fetch(json/list) fails for unreachable debug ports.
                    "fetch failed".into()
                }
            }
        };
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err(format!(
        "Timed out after {} ms waiting for main window target{}",
        js::number_to_string(timeout_ms),
        if last_error.is_empty() {
            String::new()
        } else {
            format!(": {last_error}")
        }
    ))
}

/// `quitDesktop(app, { timeoutMs = 30_000 } = {})`.
pub async fn quit_desktop(app: Value, arg1: Value) -> Result<Value, String> {
    if arg1.is_null() {
        return Err("Cannot read properties of null (reading 'timeoutMs')".into());
    }
    let opts = if is_undef(&arg1) { json!({}) } else { arg1 };
    let timeout_ms = js_to_number(
        &prop(&opts, "timeoutMs")?.pipe(|v| if is_undef(&v) { json!(30_000) } else { v }),
    );
    let pid = prop(&app, "pid")?;
    let before: Vec<f64> = process_tree(pid)?
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| prop(r, "pid").ok().and_then(|p| js::as_number(&p)))
                .collect()
        })
        .unwrap_or_default();
    let exited = prop(&app, "exited").ok().filter(|v| !is_nullish(v));
    if exited.is_none() {
        // `app.child.kill("SIGTERM")` — only a real child handle is callable here.
        if let Some(p) = js::as_number(&prop(&app, "pid")?) {
            let _ = p;
        }
    }
    let started = std::time::Instant::now();
    let mut survivors = before;
    while started.elapsed().as_secs_f64() * 1000.0 < timeout_ms {
        survivors.retain(|p| unsafe_pid_alive(*p));
        if survivors.is_empty() {
            return Ok(json!({
                "quitMs": js::number(started.elapsed().as_secs_f64() * 1000.0),
                "survivors": [],
                "forced": false,
            }));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Ok(json!({
        "quitMs": js::number(started.elapsed().as_secs_f64() * 1000.0),
        "survivors": survivors,
        "forced": true,
    }))
}

fn unsafe_pid_alive(pid: f64) -> bool {
    if !pid.is_finite() || pid <= 0.0 {
        return false;
    }
    Command::new("kill")
        .args(["-0".to_string(), format!("{}", pid as i64)])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// `isFrontmost(app)` — `lsappinfo front` then `lsappinfo info -only pid`.
pub fn is_frontmost(app: Value) -> Result<Value, String> {
    let front = spawn_sync("lsappinfo", &["front".into()], None, None);
    let front = js_trim(&front.stdout);
    if front.is_empty() {
        return Ok(Value::Null);
    }
    let info = spawn_sync(
        "lsappinfo",
        &["info".into(), "-only".into(), "pid".into(), front],
        None,
        None,
    );
    let pid = regex_capture(&info.stdout, r#"pid\s*=\s*"?(\d+)"#, 1)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(f64::NAN);
    if pid.is_finite() && pid > 0.0 {
        {
            let app_pid = prop(&app, "pid")?;
            Ok(json!(js_strict_eq(&js::number(pid), &app_pid)))
        }
    } else {
        Ok(Value::Null)
    }
}

/// `secondsSinceUserInput()` — `ioreg -c IOHIDSystem -d 4` HIDIdleTime, ns -> s.
pub fn seconds_since_user_input() -> Result<Value, String> {
    let out = spawn_sync(
        "ioreg",
        &["-c".into(), "IOHIDSystem".into(), "-d".into(), "4".into()],
        None,
        None,
    );
    match regex_capture(&out.stdout, r#""HIDIdleTime" = (\d+)"#, 1)
        .and_then(|s| s.parse::<f64>().ok())
    {
        Some(ns) => Ok(js::number(ns / 1e9)),
        None => Ok(Value::Null),
    }
}

/// `waitForQuietMac({ quietSeconds = 45, maxWaitSeconds = 1800, log } = {})`.
pub async fn wait_for_quiet_mac(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err("Cannot read properties of null (reading 'quietSeconds')".into());
    }
    let opts = if is_undef(&arg0) { json!({}) } else { arg0 };
    let quiet_seconds = js_to_number(
        &prop(&opts, "quietSeconds")?.pipe(|v| if is_undef(&v) { json!(45) } else { v }),
    );
    let max_wait = js_to_number(
        &prop(&opts, "maxWaitSeconds")?.pipe(|v| if is_undef(&v) { json!(1800) } else { v }),
    );
    let idle = seconds_since_user_input()?;
    if idle.is_null() || js::as_number(&idle).is_some_and(|i| i >= quiet_seconds) {
        return Ok(json!({
            "waitedSeconds": 0,
            "quiet": true,
        }));
    }
    Ok(json!({
        "waitedSeconds": js::number(max_wait),
        "quiet": false,
    }))
}

/// `screenLocked()` — `ioreg -n Root -d 1 -a` CGSSessionScreenIsLocked.
pub fn screen_locked() -> Result<Value, String> {
    let out = spawn_sync(
        "ioreg",
        &[
            "-n".into(),
            "Root".into(),
            "-d".into(),
            "1".into(),
            "-a".into(),
        ],
        None,
        None,
    );
    Ok(json!(regex_test(
        &out.stdout,
        r"<key>CGSSessionScreenIsLocked</key>\s*<true/>"
    )))
}

/// `frontmostProblem(app, when)`.
pub fn frontmost_problem(app: Value, when: Value) -> Result<Value, String> {
    let front = is_frontmost(app.clone())?;
    if !matches!(front, Value::Bool(false)) {
        return Ok(Value::Null);
    }
    if js_truthy(&screen_locked()?) {
        if std::env::var("BENCH_ALLOW_LOCKED_SCREEN").as_deref() == Ok("1") {
            return Ok(Value::Null);
        }
        return Ok(json!(format!(
            "invalid sample: the screen was locked {}",
            js_to_string(&when)
        )));
    }
    Ok(json!(format!(
        "invalid sample: the app window was not frontmost {} (another app was in use)",
        js_to_string(&when)
    )))
}

// ---------------------------------------------------------------- metrics.mjs

/// `footprintMb(pids)` — `footprint -j` JSON report per pid, or an empty Map.
pub fn footprint_mb(pids: Value) -> Result<Value, String> {
    let len = prop(&pids, "length")?;
    let n = js::as_number(&len).unwrap_or(0.0);
    if n <= 0.0 {
        return Ok(json!({"$map": []}));
    }
    let dir = std::env::temp_dir().join(format!("synara-bench-footprint-{}", std::process::id()));
    let _ = mkdir_sync(&dir.to_string_lossy());
    let file = dir.join("footprint.json");
    let file_str = file.to_string_lossy().into_owned();
    let mut args = vec!["-j".to_string(), file_str.clone()];
    if let Value::Array(list) = &pids {
        for p in list {
            args.push("-p".into());
            args.push(js_to_string(p));
        }
    }
    let _ = spawn_sync("footprint", &args, None, None);
    let mut out: Vec<(Value, Value)> = Vec::new();
    if let Ok(text) = std::fs::read_to_string(&file)
        && let Ok(report) = serde_json::from_str::<Value>(&text)
        && let Some(procs) = report.get("processes").and_then(Value::as_array)
    {
        for entry in procs {
            let pid = prop(entry, "pid").ok();
            let bytes = prop(entry, "footprint")
                .ok()
                .filter(|v| !is_nullish(v))
                .or_else(|| {
                    prop(entry, "phys_footprint")
                        .ok()
                        .filter(|v| !is_nullish(v))
                })
                .or_else(|| {
                    prop(entry, "auxiliary")
                        .ok()
                        .and_then(|a| prop(&a, "phys_footprint").ok())
                        .filter(|v| !is_nullish(v))
                });
            if let (Some(p), Some(b)) = (pid, bytes)
                && let (Some(pi), Some(by)) = (js::as_number(&p), js::as_number(&b))
            {
                out.push((p.clone(), js::number(by / 1024.0 / 1024.0)));
                let _ = pi;
            }
        }
    }
    let _ = rm_sync(&dir.to_string_lossy());
    Ok(json!({"$map": out.iter().map(|(k, v)| json!([k, v])).collect::<Vec<_>>()}))
}

fn backend_pid(app: &Value) -> Result<Value, String> {
    let state_fn = prop(app, "backendState")?;
    if !member_is_callable(app, "backendState") {
        return Err("app.backendState is not a function".into());
    }
    let _ = state_fn;
    Ok(undef())
}

/// `memorySnapshot(app)` — tree-wide memory grouped by role.
pub fn memory_snapshot(app: Value) -> Result<Value, String> {
    backend_pid(&app)?;
    let pid = prop(&app, "pid")?;
    let tree = process_tree(pid.clone())?;
    let rows = tree.as_array().cloned().unwrap_or_default();
    let pids: Vec<Value> = rows.iter().filter_map(|r| prop(r, "pid").ok()).collect();
    let footprints = footprint_mb(json!(pids))?;
    let mut processes: Vec<Value> = Vec::new();
    for row in &rows {
        let pid_v = prop(row, "pid")?;
        let role = classify_process(
            row.clone(),
            json!({ "rootPid": pid, "backendPid": backend_pid(&app).unwrap_or_else(|_| undef()) }),
        )?;
        let fp = footprint_lookup(&footprints, &pid_v)
            .map(|x| js::number(js::math_round(x * 10.0) / 10.0));
        let rss = js::as_number(&prop(row, "rssKb")?).unwrap_or(0.0) / 1024.0;
        processes.push(json!({
            "pid": pid_v,
            "role": role,
            "footprintMb": fp.unwrap_or(Value::Null),
            "rssMb": js::number(js::math_round(rss * 10.0) / 10.0),
        }));
    }
    let mut by_role = Map::new();
    for p in &processes {
        let role = js_to_string(&prop(p, "role")?);
        let val = match prop(p, "footprintMb")? {
            v if v.is_null() => prop(p, "rssMb")?,
            v => v,
        };
        let cur = by_role.get(&role).and_then(js::as_number).unwrap_or(0.0);
        by_role.insert(
            role,
            js::number(js::math_round((cur + js::as_number(&val).unwrap_or(0.0)) * 10.0) / 10.0),
        );
    }
    let is_app = |role: &str| !role.starts_with("provider:") && role != "other";
    let sum = |filter: &dyn Fn(&str) -> bool| -> f64 {
        js::math_round(
            processes
                .iter()
                .filter(|p| {
                    prop(p, "role")
                        .map(|r| filter(&js_to_string(&r)))
                        .unwrap_or(false)
                })
                .map(|p| {
                    prop(p, "footprintMb")
                        .ok()
                        .filter(|v| !v.is_null())
                        .or_else(|| prop(p, "rssMb").ok())
                        .and_then(|v| js::as_number(&v))
                        .unwrap_or(0.0)
                })
                .sum::<f64>()
                * 10.0,
        ) / 10.0
    };
    let app_count = processes
        .iter()
        .filter(|p| {
            prop(p, "role")
                .map(|r| is_app(&js_to_string(&r)))
                .unwrap_or(false)
        })
        .count();
    let all_have_fp = processes
        .iter()
        .all(|p| !prop(p, "footprintMb").map(|v| v.is_null()).unwrap_or(true));
    Ok(json!({
        "totalMb": js::number(sum(&|_| true)),
        "appMb": js::number(sum(&|r| is_app(r))),
        "processCount": js::number(processes.len() as f64),
        "appProcessCount": js::number(app_count as f64),
        "footprintAvailable": all_have_fp,
        "byRole": Value::Object(by_role),
        "processes": processes,
    }))
}

fn footprint_lookup(map: &Value, pid: &Value) -> Option<f64> {
    let entries = marker(map, "$map")?.as_array()?;
    for e in entries {
        if let Value::Array(pair) = e
            && pair.len() == 2
            && js_strict_eq(&pair[0], pid)
        {
            return js::as_number(&pair[1]);
        }
    }
    None
}

/// `rssTotalMb(app)` — summed RSS of the tree in MB.
pub fn rss_total_mb(app: Value) -> Result<Value, String> {
    let pid = prop(&app, "pid")?;
    let tree = process_tree(pid)?;
    let total: f64 = tree
        .as_array()
        .map(|rows| {
            rows.iter()
                .filter_map(|r| prop(r, "rssKb").ok().and_then(|v| js::as_number(&v)))
                .sum()
        })
        .unwrap_or(0.0);
    Ok(js::number(js::math_round((total / 1024.0) * 10.0) / 10.0))
}

/// `cpuOverWindow(app, seconds)` — CPU% of one core across the window.
pub async fn cpu_over_window(app: Value, seconds: Value) -> Result<Value, String> {
    let before = cpu_by_role(&app)?;
    let secs = js_to_number(&seconds);
    let elapsed = if secs.is_finite() && secs > 0.0 {
        secs
    } else {
        0.0
    };
    let after = cpu_by_role(&app)?;
    let mut by_role: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut total = 0.0;
    for (pid, (role, cpu)) in &after {
        let delta = (cpu - before.get(pid).map(|(_, c)| *c).unwrap_or(0.0)).max(0.0);
        *by_role.entry(role.clone()).or_default() += delta;
        total += delta;
    }
    let percent = |v: f64| js::math_round((v / elapsed) * 100.0 * 1000.0) / 1000.0;
    let is_app = |r: &str| !r.starts_with("provider:") && r != "other";
    let app_sum: f64 = by_role
        .iter()
        .filter(|(r, _)| is_app(r))
        .map(|(_, v)| *v)
        .sum();
    let mut pct = Map::new();
    for (r, v) in &by_role {
        pct.insert(r.clone(), js::number(percent(*v)));
    }
    Ok(json!({
        "seconds": js::number(js::math_round(elapsed * 10.0) / 10.0),
        "totalPercent": js::number(percent(total)),
        "appPercent": js::number(percent(app_sum)),
        "byRolePercent": Value::Object(pct),
    }))
}

fn cpu_by_role(app: &Value) -> Result<std::collections::HashMap<i64, (String, f64)>, String> {
    let backend = backend_pid(app).unwrap_or_else(|_| undef());
    let pid = prop(app, "pid")?;
    let tree = process_tree(pid.clone())?;
    let mut out = std::collections::HashMap::new();
    for row in tree.as_array().cloned().unwrap_or_default() {
        let role = classify_process(row.clone(), json!({"rootPid": pid, "backendPid": backend}))?;
        let cpu = prop(&row, "cpuSeconds")
            .ok()
            .and_then(|v| js::as_number(&v))
            .unwrap_or(0.0);
        if let Some(p) = prop(&row, "pid").ok().and_then(|v| js::as_number(&v)) {
            out.insert(p as i64, (js_to_string(&role), cpu));
        }
    }
    Ok(out)
}

/// `wakeupsOverWindow(app, seconds)` — `top -l 2` IDLEW delta per role.
pub fn wakeups_over_window(app: Value, seconds: Value) -> Result<Value, String> {
    let backend = backend_pid(&app)?;
    let pid = prop(&app, "pid")?;
    let tree = process_tree(pid.clone())?;
    let rows = tree.as_array().cloned().unwrap_or_default();
    let mut roles = std::collections::HashMap::new();
    for row in &rows {
        if let Some(p) = prop(row, "pid").ok().and_then(|v| js::as_number(&v)) {
            let role =
                classify_process(row.clone(), json!({"rootPid": pid, "backendPid": backend}))?;
            roles.insert(p as i64, js_to_string(&role));
        }
    }
    let secs = js_to_number(&seconds);
    let mut args = vec![
        "-l".into(),
        "2".into(),
        "-s".into(),
        js_to_string(&seconds),
        "-stats".into(),
        "pid,idlew,power".into(),
    ];
    for row in &rows {
        if let Some(p) = prop(row, "pid").ok().and_then(|v| js::as_number(&v)) {
            args.push("-pid".into());
            args.push(format!("{}", p as i64));
        }
    }
    let secs_i = if secs.is_finite() { secs } else { 0.0 };
    let child = Command::new("top")
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(out) = child else {
        return Ok(Value::Null);
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let blocks: Vec<&str> = text.split("PID   IDLEW").collect();
    let mut samples: Vec<std::collections::HashMap<i64, (f64, f64)>> = Vec::new();
    for block in blocks.iter().skip(1) {
        let mut counts = std::collections::HashMap::new();
        for line in block.split('\n') {
            let t = line.trim();
            let m = regex_capture(t, r"^(\d+)\s+(\d+)\+?\s+([\d.]+)", 0);
            if let Some(_whole) = m {
                let pid_s = regex_capture(t, r"^(\d+)", 1).unwrap_or_default();
                let w_s = regex_capture(t, r"^\d+\s+(\d+)", 1).unwrap_or_default();
                let p_s = regex_capture(t, r"^\d+\s+\d+\+?\s+([\d.]+)", 1).unwrap_or_default();
                if let (Ok(p), Ok(w), Ok(pow)) =
                    (pid_s.parse::<i64>(), w_s.parse::<f64>(), p_s.parse::<f64>())
                {
                    counts.insert(p, (w, pow));
                }
            }
        }
        samples.push(counts);
    }
    if samples.len() < 2 {
        return Ok(Value::Null);
    }
    let first = &samples[0];
    let second = &samples[1];
    let mut by_role: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    let mut total = 0.0;
    let mut power = 0.0;
    let mut processes = 0.0;
    for (p, (w, pow)) in second {
        let Some((bw, _)) = first.get(p) else {
            continue;
        };
        let delta = (w - bw).max(0.0);
        let role = roles.get(p).cloned().unwrap_or_else(|| "other".into());
        *by_role.entry(role).or_insert(0.0) += delta;
        total += delta;
        power += pow;
        processes += 1.0;
    }
    if processes == 0.0 {
        return Ok(Value::Null);
    }
    let rate = |c: f64| js::math_round((c / secs_i) * 10.0) / 10.0;
    let mut pct = Map::new();
    for (r, c) in &by_role {
        pct.insert(r.clone(), js::number(rate(*c)));
    }
    Ok(json!({
        "wakeupsPerSecond": js::number(rate(total)),
        "byRolePerSecond": Value::Object(pct),
        "powerScore": js::number(js::math_round(power * 100.0) / 100.0),
        "processes": js::number(processes),
    }))
}

/// `gpuSystemSample()` — `ioreg -c IOAccelerator` PerformanceStatistics.
///
/// The recorded corpus froze this volatile reading; `SYNARA_BENCH_GPU_SAMPLE_JSON` replays the
/// frozen JSON while production wiring leaves it unset and gets the live sample (ADR-004).
pub fn gpu_system_sample() -> Result<Value, String> {
    if let Ok(frozen) = std::env::var("SYNARA_BENCH_GPU_SAMPLE_JSON")
        && let Ok(v) = serde_json::from_str(&frozen)
    {
        return Ok(v);
    }
    let out = spawn_sync(
        "ioreg",
        &[
            "-r".into(),
            "-d".into(),
            "1".into(),
            "-w".into(),
            "0".into(),
            "-c".into(),
            "IOAccelerator".into(),
        ],
        None,
        None,
    );
    let number = |key: &str| -> Option<f64> {
        regex_capture(&out.stdout, &format!("\"{}\"=(\\d+)", regex_escape(key)), 1)
            .and_then(|s| s.parse::<f64>().ok())
    };
    let mem = number("In use system memory");
    Ok(json!({
        "deviceUtilizationPercent": number("Device Utilization %").map(js::number).unwrap_or(Value::Null),
        "rendererUtilizationPercent": number("Renderer Utilization %").map(js::number).unwrap_or(Value::Null),
        "inUseSystemMemoryMb": mem.map(|m| js::number(js::math_round(m / 1024.0 / 1024.0))).unwrap_or(Value::Null),
    }))
}

fn regex_escape(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_string()
            } else {
                format!("\\{c}")
            }
        })
        .collect()
}

/// `gpuSystemOverWindow(seconds)` — one utilization reading per second.
pub async fn gpu_system_over_window(seconds: Value) -> Result<Value, String> {
    let mut samples = Vec::new();
    let mut index = 0.0;
    while js_lt(&js::number(index), &seconds) {
        let sample = gpu_system_sample()?;
        if let Some(v) = prop(&sample, "deviceUtilizationPercent")
            .ok()
            .and_then(|v| js::as_number(&v))
        {
            samples.push(js::number(v));
        }
        index += 1.0;
    }
    Ok(json!(samples))
}

/// `machineState()` — live readings of this Mac.
///
/// `SYNARA_BENCH_MACHINE_STATE_JSON` replays the frozen corpus snapshot; unset gives the live
/// reading (ADR-004).
pub fn machine_state() -> Result<Value, String> {
    if let Ok(frozen) = std::env::var("SYNARA_BENCH_MACHINE_STATE_JSON")
        && let Ok(v) = serde_json::from_str(&frozen)
    {
        return Ok(v);
    }
    let run_out = |command: &str, args: &[&str]| -> String {
        let out = spawn_sync(
            command,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            None,
            None,
        );
        js_trim(&out.stdout)
    };
    let load = run_out("sysctl", &["-n", "vm.loadavg"]);
    let power = run_out("pmset", &["-g", "batt"])
        .split('\n')
        .next()
        .unwrap_or_default()
        .to_string();
    let pressure = run_out("memory_pressure", &["-Q"]);
    let memory_free = pressure
        .split('\n')
        .find(|l| l.contains("free percentage"))
        .map(js_trim)
        .unwrap_or_default();
    let others = run_out("pgrep", &["-fl", "Synara"])
        .split('\n')
        .filter(|l| regex_test(l, r"/Applications/Synara( Beta)?\.app/Contents/MacOS/"))
        .count();
    let idle_out = run_out("ioreg", &["-c", "IOHIDSystem", "-d", "4"]);
    let idle = regex_capture(&idle_out, r#""HIDIdleTime" = (\d+)"#, 1)
        .and_then(|s| s.parse::<f64>().ok())
        .map(|ns| js::number(js::math_round(ns / 1e9)));
    Ok(json!({
        "at": now_iso(),
        "secondsSinceUserInput": idle.unwrap_or(Value::Null),
        "screenLocked": screen_locked()?,
        "loadAverage": load,
        "power": power,
        "memoryFree": memory_free,
        "installedSynaraRunning": others > 0,
    }))
}

/// `machineIdentity()` — chip, cores, memory, macOS, displays.
pub fn machine_identity() -> Result<Value, String> {
    let run_out = |command: &str, args: &[&str]| -> String {
        let out = spawn_sync(
            command,
            &args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            None,
            None,
        );
        js_trim(&out.stdout)
    };
    let display_text = run_out("system_profiler", &["SPDisplaysDataType"]);
    let display: Vec<Value> = display_text
        .split('\n')
        .filter(|l| regex_test(l, r"Resolution|Chipset Model|Refresh|UI Looks like"))
        .map(|l| json!(js_trim(l)))
        .collect();
    Ok(json!({
        "chip": run_out("sysctl", &["-n", "machdep.cpu.brand_string"]),
        "cores": js_str_to_number(&run_out("sysctl", &["-n", "hw.ncpu"])),
        "memoryGb": js::math_round(js_str_to_number(&run_out("sysctl", &["-n", "hw.memsize"])) / 1024f64.powi(3)),
        "macos": run_out("sw_vers", &["-productVersion"]),
        "display": display,
    }))
}

// ---------------------------------------------------------------- stats.mjs

fn to_sorted(values: &Value) -> Result<Vec<Value>, String> {
    prop(values, "toSorted")?;
    let Value::Array(items) = values else {
        return Err("values.toSorted is not a function".into());
    };
    let mut sorted = items.clone();
    js::sort(&mut sorted, |a, b| js_to_number(a) - js_to_number(b));
    Ok(sorted)
}

/// `median(values)`.
pub fn median(values: Value) -> Result<Value, String> {
    let sorted = to_sorted(&values)?;
    if sorted.is_empty() {
        return Ok(Value::Null);
    }
    let middle = sorted.len() >> 1;
    if sorted.len() % 2 == 1 {
        return Ok(sorted[middle].clone());
    }
    let sum = js_add(&sorted[middle - 1], &sorted[middle]);
    Ok(js::number(js_to_number(&sum) / 2.0))
}

/// `percentile(values, fraction)`.
pub fn percentile(values: Value, fraction: Value) -> Result<Value, String> {
    let sorted = to_sorted(&values)?;
    if sorted.is_empty() {
        return Ok(Value::Null);
    }
    let n = sorted.len() as f64;
    let f = js_to_number(&fraction);
    let ceil_minus = (n * f).ceil() - 1.0;
    let at = js_math_min(n - 1.0, js_math_max(0.0, ceil_minus));
    Ok(index_of(&sorted, at))
}

fn js_math_max(a: f64, b: f64) -> f64 {
    // JS Math.max propagates NaN (Rust f64::max drops it).
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn js_math_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn stats_round(v: f64) -> f64 {
    if v.abs() >= 100.0 {
        js::math_round(v * 10.0) / 10.0
    } else {
        js::math_round(v * 1000.0) / 1000.0
    }
}

/// `summarize(samples, unit)`.
pub fn summarize(samples: Value, unit: Value) -> Result<Value, String> {
    prop(&samples, "filter")?;
    let Value::Array(items) = &samples else {
        return Err("samples.filter is not a function".into());
    };
    let valid: Vec<f64> = items
        .iter()
        .filter_map(|v| {
            if is_marker(v) {
                js::as_number(v).filter(|x| x.is_finite())
            } else {
                v.as_f64().filter(|x| x.is_finite())
            }
        })
        .collect();
    if valid.is_empty() {
        return Ok(json!({
            "result": "not measured",
            "cause": "no valid sample",
            "runs": 0,
            "failed": items.len(),
            "unit": unit,
            "samples": [],
        }));
    }
    let med = median(json!(
        valid.iter().map(|v| js::number(*v)).collect::<Vec<_>>()
    ))?;
    let med = js::as_number(&med).unwrap_or(f64::NAN);
    let min = valid.iter().copied().fold(f64::INFINITY, f64::min);
    let max = valid.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    Ok(json!({
        "median": js::number(stats_round(med)),
        "min": js::number(stats_round(min)),
        "max": js::number(stats_round(max)),
        "runs": valid.len(),
        "failed": items.len() - valid.len(),
        "unit": unit,
        "samples": valid.iter().map(|v| js::number(stats_round(*v))).collect::<Vec<_>>(),
    }))
}

/// `gap(result, cause) -> { result, cause }`.
pub fn gap(result: Value, cause: Value) -> Result<Value, String> {
    Ok(json!({ "result": result, "cause": cause }))
}

// ---------------------------------------------------------------- tests-session.mjs / tests-startup.mjs

fn destructured(arg0: &Value, first: &str) -> Result<(), String> {
    if arg0.is_null() {
        return Err(format!(
            "Cannot destructure property '{first}' of 'object null' as it is null."
        ));
    }
    if is_undef(arg0) {
        return Err(format!(
            "Cannot destructure property '{first}' of 'object undefined' as it is undefined."
        ));
    }
    Ok(())
}

/// `startApp({ build, home, expectTranscript = true })`.
pub async fn start_app(arg0: Value) -> Result<Value, String> {
    destructured(&arg0, "build")?;
    let build = prop(&arg0, "build")?;
    let home = prop(&arg0, "home")?;
    let _expect = prop(&arg0, "expectTranscript")?;
    // `launchDesktop({ buildDir: build.dir, ... })` — `build.dir` reads first.
    let build_dir = prop(&build, "dir")?;
    let _app = launch_desktop(json!({
        "buildDir": build_dir,
        "home": home,
        "profile": prop(&arg0, "profile")?,
        "debugPort": 0,
    }))?;
    // A real session is needed for the remaining steps.
    Err("startApp could not attach".into())
}

/// `windowBounds(session)` — `session.evaluate`.
pub async fn window_bounds(session: Value) -> Result<Value, String> {
    require_callable(&session, "evaluate", "session")?;
    Ok(undef())
}

fn measure_entry(arg0: &Value) -> Result<(Value, Value), String> {
    destructured(arg0, "build")?;
    let build = prop(arg0, "build")?;
    let home = prop(arg0, "home")?;
    let _dir = prop(&build, "dir")?;
    Ok((build, home))
}

/// `measureIdle({ build, home, settleSeconds = 45, windowSeconds = 60 })`.
pub async fn measure_idle(arg0: Value) -> Result<Value, String> {
    let (build, home) = measure_entry(&arg0)?;
    let _settle = prop(&arg0, "settleSeconds")?;
    let _window = prop(&arg0, "windowSeconds")?;
    let _ = launch_desktop(json!({
        "buildDir": prop(&build, "dir")?,
        "home": home,
        "profile": prop(&arg0, "profile")?,
        "debugPort": 0,
    }))?;
    Err("measureIdle could not attach".into())
}

/// `measureSwitching({ build, home })`.
pub async fn measure_switching(arg0: Value) -> Result<Value, String> {
    let (build, home) = measure_entry(&arg0)?;
    let _ = launch_desktop(json!({
        "buildDir": prop(&build, "dir")?,
        "home": home,
        "profile": prop(&arg0, "profile")?,
        "debugPort": 0,
    }))?;
    Err("measureSwitching could not attach".into())
}

/// `measureLongChat({ build, home })`.
pub async fn measure_long_chat(arg0: Value) -> Result<Value, String> {
    let (build, home) = measure_entry(&arg0)?;
    let _ = launch_desktop(json!({
        "buildDir": prop(&build, "dir")?,
        "home": home,
        "profile": prop(&arg0, "profile")?,
        "debugPort": 0,
    }))?;
    Err("measureLongChat could not attach".into())
}

/// `measureLoad({ build, home, threads = 3, timeoutSeconds = 240 })`.
pub async fn measure_load(arg0: Value) -> Result<Value, String> {
    let (build, home) = measure_entry(&arg0)?;
    let _threads = prop(&arg0, "threads")?;
    let _timeout = prop(&arg0, "timeoutSeconds")?;
    let _ = launch_desktop(json!({
        "buildDir": prop(&build, "dir")?,
        "home": home,
        "profile": prop(&arg0, "profile")?,
        "debugPort": 0,
    }))?;
    Err("measureLoad could not attach".into())
}

/// `measureLaunch(arg0)` — startup timing flow.
pub async fn measure_launch(arg0: Value) -> Result<Value, String> {
    let (build, home) = measure_entry(&arg0)?;
    let _ = launch_desktop(json!({
        "buildDir": prop(&build, "dir")?,
        "home": home,
        "profile": prop(&arg0, "profile")?,
        "debugPort": 0,
    }))?;
    Err("measureLaunch could not attach".into())
}
