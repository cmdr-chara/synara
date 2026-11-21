//! Shared ECMAScript semantics for the Rust ports: helpers whose JS behaviour
//! differs from the Rust standard library or serde defaults.

#![forbid(unsafe_code)]

use serde_json::Value;

/// The JS `\s` whitespace set (Rust regex `\s` differs on `` and `﻿`).
pub fn is_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}'
        | '\u{3000}' | '\u{feff}')
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
                    _ => n.parse().map(|x: f64| number_to_string(x)).unwrap_or_default(),
                };
            }
            if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
                let units: Vec<u16> =
                    units.iter().filter_map(|u| u.as_u64().map(|u| u as u16)).collect();
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
