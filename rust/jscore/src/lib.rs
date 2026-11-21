//! Shared ECMAScript semantics for the Rust ports: helpers whose JS behaviour
//! differs from the Rust standard library or serde defaults.

#![forbid(unsafe_code)]

use serde_json::Value;

/// The JS `\s` whitespace set (Rust regex `\s` differs on `` and `﻿`).
pub fn is_space(c: char) -> bool {
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

/// `SERVER_DESKTOP_FLAVOR`: the hosting desktop's flavor from the
/// `SYNARA_DESKTOP_BUNDLE_ID` env var, resolved once like the module-level const.
pub fn synara_desktop_flavor() -> &'static str {
    use std::sync::OnceLock;
    static FLAVOR: OnceLock<&'static str> = OnceLock::new();
    FLAVOR.get_or_init(|| {
        match std::env::var("SYNARA_DESKTOP_BUNDLE_ID")
            .ok()
            .map(|v| v.trim_matches(is_space).to_string())
            .as_deref()
        {
            Some("com.emanueledipietro.synara") => "production",
            Some("com.emanueledipietro.synara.dev") => "development",
            Some("com.emanueledipietro.synara.canary") => "canary",
            Some("com.emanueledipietro.synara.cua") => "cua",
            Some("com.emanueledipietro.synara.remote") => "remote",
            Some("com.emanueledipietro.synara.beta") => "beta",
            _ => "unknown",
        }
    })
}

/// `application/x-www-form-urlencoded` encoding (WHATWG `URLSearchParams`):
/// ASCII alphanumerics and `*-._` pass through, space becomes `+`,
/// everything else becomes `%XX` on UTF-8 bytes.
pub fn form_url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// JS truthiness on a wire `Value` (marker objects are unwrapped first):
/// undefined/null/false/0/NaN/"" are falsy, everything else truthy.
pub fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Object(m) => {
            if m.get("$undefined").is_some() {
                return false;
            }
            if let Some(n) = m.get("$num").and_then(Value::as_str) {
                return match n {
                    "NaN" | "-0" => false,
                    _ => n.parse::<f64>().map(|x| x != 0.0).unwrap_or(true),
                };
            }
            if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
                return !units.is_empty();
            }
            true
        }
        Value::Array(_) => true,
    }
}

/// JS `String(v)` / template-literal interpolation of a JSON value:
/// `undefined`→`"undefined"`, `null`→`"null"`, arrays join by `,`,
/// plain objects become `"[object Object]"`, numbers use `number_to_string`.
pub fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => {
            // Array.prototype.join: null/undefined elements become "".
            a.iter()
                .map(|e| match e {
                    Value::Null => String::new(),
                    Value::Object(m) if m.get("$undefined").is_some() => String::new(),
                    _ => js_to_string(e),
                })
                .collect::<Vec<_>>()
                .join(",")
        }
        Value::Object(m) => {
            if m.get("$undefined").is_some() {
                return "undefined".to_string();
            }
            if let Some(n) = m.get("$num").and_then(Value::as_str) {
                return match n {
                    "NaN" => "NaN".to_string(),
                    "Infinity" => "Infinity".to_string(),
                    "-Infinity" => "-Infinity".to_string(),
                    "-0" => "0".to_string(),
                    _ => n
                        .parse()
                        .map(|x: f64| number_to_string(x))
                        .unwrap_or_default(),
                };
            }
            if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
                let units: Vec<u16> = units
                    .iter()
                    .filter_map(|u| u.as_u64().map(|u| u as u16))
                    .collect();
                return String::from_utf16_lossy(&units);
            }
            "[object Object]".to_string()
        }
    }
}

/// `Number::toString` (ECMA-262): `NaN`/`Infinity` keep their JS spellings,
/// `-0` is `"0"`, integral values print without a fraction, and the fixed/
/// exponential cutovers are at `1e21` and `1e-6`.
pub fn number_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if v == 0.0 {
        return "0".into();
    }
    if v.fract() == 0.0 && v.abs() < 1e21 {
        return format!("{v:.0}");
    }
    // Rust Display gives shortest round-trip decimal digits without an
    // exponent; split them and re-wrap in the JS n/digits form.
    let s = format!("{v}");
    let (sign, s) = match s.strip_prefix('-') {
        Some(r) => ("-", r),
        None => ("", s.as_str()),
    };
    let (int, frac) = s.split_once('.').unwrap_or((s, ""));
    let (n, digits) = if int != "0" {
        (int.len() as i64, format!("{int}{frac}"))
    } else {
        let d = frac.trim_start_matches('0');
        (-((frac.len() - d.len()) as i64), d.to_string())
    };
    let digits = digits.trim_end_matches('0');
    let k = digits.len() as i64;
    let mut out = sign.to_string();
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.push_str(&"0".repeat((n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat((-n) as usize));
        out.push_str(digits);
    } else {
        let e = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.abs().to_string());
    }
    out
}

/// `JSON.stringify(v)` for a `serde_json::Value`: numbers go through
/// `number_to_string` (every JSON number is a double, so `5.0` prints `5`,
/// `-0` prints `0`, `1e21` prints `1e+21`), object keys keep insertion order
/// (`preserve_order` map), strings escape like `serde_json` already does.
pub fn json_stringify(v: &Value) -> String {
    let mut out = String::new();
    write_json(v, &mut out);
    out
}

fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => match n.as_f64() {
            Some(f) if f.is_finite() => out.push_str(&number_to_string(f)),
            // JSON.stringify serializes non-finite numbers as null.
            _ => out.push_str("null"),
        },
        Value::String(_) => out.push_str(&serde_json::to_string(v).unwrap_or_default()),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_json(x, out);
            }
            out.push('}');
        }
    }
}

// JS `\s` for the comma lookahead (the Unicode space set JS regexes use).
const JS_SPACE: &str = "\t\n\u{b}\u{c}\r \u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

// `contents.replace(/"(?:[^"\\]|\\.)*"|,(?=\s*[}\]])/g, t => t === "," ? "" : t)`
fn strip_trailing_commas(input: &str) -> String {
    let b: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == '"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != '"' {
                if b[j] == '\\' {
                    match b.get(j + 1) {
                        Some(&c) if !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') => j += 2,
                        _ => break,
                    }
                } else {
                    j += 1;
                }
            }
            if j < b.len() {
                out.extend(&b[i..=j]);
                i = j + 1;
                continue;
            }
        }
        if b[i] == ',' {
            let mut j = i + 1;
            while j < b.len() && JS_SPACE.contains(b[j]) {
                j += 1;
            }
            if j < b.len() && matches!(b[j], '}' | ']') {
                i += 1;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

// `normalizeBunLockfile` from `scripts/bun-lockfile.ts`, shared by the ports
// that hash workspace state.
pub fn normalize_bun_lockfile(contents: &str) -> String {
    let Ok(mut lockfile) = serde_json::from_str::<Value>(&strip_trailing_commas(contents)) else {
        return contents.to_string();
    };
    let is_v3 = matches!(lockfile.get("lockfileVersion"), Some(v) if v.as_f64() == Some(3.0));
    let falsy_ws = match lockfile.get("workspaces") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => true,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        _ => false,
    };
    if !is_v3 || falsy_ws {
        return contents.to_string();
    }
    let members: Vec<&mut Value> = match lockfile.get_mut("workspaces") {
        Some(Value::Object(m)) => m.values_mut().collect(),
        Some(Value::Array(a)) => a.iter_mut().collect(),
        _ => Vec::new(),
    };
    for member in members {
        if member.is_null() {
            return contents.to_string();
        }
        if let Value::Object(m) = member {
            if m.get("version").is_some_and(Value::is_string) {
                m.shift_remove("version");
            }
        }
    }
    json_stringify(&lockfile)
}

// Expand `{a,b}` alternates in a glob pattern (Node fs.globSync/minimatch).
pub fn glob_brace_expand(pattern: &str) -> Vec<String> {
    let Some(open) = pattern.find('{') else {
        return vec![pattern.to_string()];
    };
    let mut depth = 0;
    let mut close = None;
    for (i, c) in pattern.char_indices().skip(open) {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return vec![pattern.to_string()];
    };
    let inner = &pattern[open + 1..close];
    let mut alts = Vec::new();
    let mut cur = String::new();
    let mut d = 0;
    for c in inner.chars() {
        if c == ',' && d == 0 {
            alts.push(std::mem::take(&mut cur));
        } else {
            if c == '{' {
                d += 1;
            }
            if c == '}' {
                d -= 1;
            }
            cur.push(c);
        }
    }
    alts.push(cur);
    let mut out = Vec::new();
    for alt in alts {
        out.extend(glob_brace_expand(&format!(
            "{}{}{}",
            &pattern[..open],
            alt,
            &pattern[close + 1..]
        )));
    }
    out
}

// One path segment against a pattern segment: `*`, `?`, `[a-z]`, `[!a-z]`.
fn segment_match(pat: &[char], seg: &[char]) -> bool {
    if pat.is_empty() {
        return seg.is_empty();
    }
    match pat[0] {
        '*' => (0..=seg.len()).any(|skip| segment_match(&pat[1..], &seg[skip..])),
        '?' => !seg.is_empty() && segment_match(&pat[1..], &seg[1..]),
        '[' => {
            let neg = pat.get(1) == Some(&'!') || pat.get(1) == Some(&'^');
            let mut i = if neg { 2 } else { 1 };
            if seg.is_empty() {
                return false;
            }
            let c = seg[0];
            let mut matched = false;
            while i < pat.len() && pat[i] != ']' {
                if i + 2 < pat.len() && pat[i + 1] == '-' && pat[i + 2] != ']' {
                    if pat[i] <= c && c <= pat[i + 2] {
                        matched = true;
                    }
                    i += 3;
                } else {
                    if pat[i] == c {
                        matched = true;
                    }
                    i += 1;
                }
            }
            i < pat.len() && (matched != neg) && segment_match(&pat[i + 1..], &seg[1..])
        }
        c => !seg.is_empty() && seg[0] == c && segment_match(&pat[1..], &seg[1..]),
    }
}

fn glob_walk(dir: &std::path::Path, segments: &[String], out: &mut Vec<String>, rel: &str) {
    if segments.is_empty() {
        return;
    }
    let (head, rest) = (&segments[0], &segments[1..]);
    if head == "**" {
        glob_walk(dir, rest, out, rel);
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == "node_modules" || name == ".git" {
                    continue;
                }
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let child = if rel.is_empty() {
                        name
                    } else {
                        format!("{rel}/{name}")
                    };
                    if rest.is_empty() {
                        collect_all_files(&entry.path(), out, &child);
                    } else {
                        glob_walk(&entry.path(), segments, out, &child);
                    }
                }
            }
        }
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let pat_chars: Vec<char> = head.chars().collect();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let seg_chars: Vec<char> = name.chars().collect();
        if !segment_match(&pat_chars, &seg_chars) {
            continue;
        }
        let child = if rel.is_empty() {
            name
        } else {
            format!("{rel}/{name}")
        };
        if rest.is_empty() {
            out.push(child);
        } else if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            glob_walk(&entry.path(), rest, out, &child);
        }
    }
}

fn collect_all_files(dir: &std::path::Path, out: &mut Vec<String>, rel: &str) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "node_modules" || name == ".git" {
                continue;
            }
            let child = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                collect_all_files(&entry.path(), out, &child);
            } else {
                out.push(child);
            }
        }
    }
}

// `globSync(patterns, {cwd})` — matches relative to `cwd`, returns
// '/'-separated relative paths.
pub fn glob_sync(patterns: &[String], cwd: &std::path::Path) -> Vec<String> {
    let mut out = Vec::new();
    for pattern in patterns {
        for expanded in glob_brace_expand(pattern) {
            let segments: Vec<String> = expanded.split('/').map(String::from).collect();
            glob_walk(cwd, &segments, &mut out, "");
        }
    }
    out
}
