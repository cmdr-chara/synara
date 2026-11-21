//! The Rust port of `apps/server/src/serverStatusCli.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

/// `v.key`: reading a member of `null`/`undefined` is a TypeError, of anything else `undefined`.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    if let Some(units) = crate::js::string_units(v) {
        return !units.is_empty();
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) => !is_undefined(v),
        Value::Array(_) => true,
        Value::Number(_) => false,
    }
}

/// `String(v)` in a template literal; an array joins the same way `Array#toString` does.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| if nullish(e) { String::new() } else { js_str(e) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if let Some(x) = crate::js::as_number(v) {
                crate::js::number_to_string(x)
            } else if let Some(units) = crate::js::string_units(v) {
                String::from_utf16_lossy(&units)
            } else if is_undefined(v) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

/// A JS `StrDecimalLiteral` mantissa (no sign): `12`, `12.`, `12.5`, `.5`.
fn decimal_body(s: &str) -> bool {
    let (int, frac) = match s.split_once('.') {
        Some((i, f)) => (i, Some(f)),
        None => (s, None),
    };
    if int.is_empty() && frac.is_none() {
        return false;
    }
    int.bytes().all(|b| b.is_ascii_digit())
        && frac
            .map(|f| !f.is_empty() || !int.is_empty())
            .unwrap_or(true)
        && frac
            .map(|f| f.bytes().all(|b| b.is_ascii_digit()))
            .unwrap_or(true)
        && !(int.is_empty() && frac == Some(""))
}

fn parse_decimal(t: &str) -> f64 {
    let (mantissa, exp) = match t.find(['e', 'E']) {
        Some(i) => (&t[..i], Some(&t[i + 1..])),
        None => (t, None),
    };
    let valid = decimal_body(mantissa)
        && exp.map(|e| {
            let e = e.strip_prefix(['+', '-']).unwrap_or(e);
            !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit())
        }) != Some(false);
    if !valid {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(s)`: whitespace-trimmed decimal/`Infinity`/`0x`/`0b`/`0o` literals; a sign is part of
/// the decimal/`Infinity` forms only (`"-0x10"` is `NaN`).
fn parse_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    let (neg, body) = match t.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let radix = |digits: &str, base: u32| {
        (!digits.is_empty())
            .then(|| {
                digits.chars().try_fold(0.0f64, |a, c| {
                    c.to_digit(base).map(|d| a * base as f64 + d as f64)
                })
            })
            .flatten()
            .unwrap_or(f64::NAN)
    };
    let v = if body == "Infinity" {
        f64::INFINITY
    } else if neg {
        parse_decimal(body)
    } else if let Some(d) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        radix(d, 16)
    } else if let Some(d) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        radix(d, 2)
    } else if let Some(d) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
        radix(d, 8)
    } else {
        parse_decimal(body)
    };
    if neg { -v } else { v }
}

/// `ToNumber`, for `Math.round(v)` and `v * 100`.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => u8::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        Value::Array(_) => parse_number(&js_str(v)),
        Value::Object(_) => {
            if let Some(x) = crate::js::as_number(v) {
                x
            } else if let Some(units) = crate::js::string_units(v) {
                parse_number(&String::from_utf16_lossy(&units))
            } else {
                f64::NAN
            }
        }
    }
}

/// `${Math.round(v)}`.
fn round_str(v: &Value) -> String {
    crate::js::number_to_string(crate::js::math_round(to_number(v)))
}

/// `x.toFixed(1)`: picks the `n` with `n/10` nearest `x`, the larger one on a tie — an exact-tie
/// case Rust's `{:.1}` (round-half-even) gets wrong. Decided on the double's exact decimal
/// expansion (`{:.1100}` shows all of it: a binary fraction's expansion ends by digit 1074).
fn to_fixed1(x: f64) -> String {
    if x.is_nan() {
        return "NaN".to_string();
    }
    if x.is_infinite() || x.abs() >= 1e21 {
        return crate::js::number_to_string(x);
    }
    let neg = x < 0.0;
    let text = format!("{:.1100}", x.abs());
    let (int, frac) = text.split_once('.').unwrap_or((text.as_str(), ""));
    let d1 = frac.as_bytes().first().copied().unwrap_or(b'0');
    let d2 = frac.as_bytes().get(1).copied().unwrap_or(b'0');
    let rest_nonzero = frac.as_bytes().iter().skip(2).any(|&b| b != b'0');
    let up = if neg {
        d2 > b'5' || (d2 == b'5' && rest_nonzero)
    } else {
        d2 >= b'5'
    };
    let mut int = int.to_string();
    let mut d = d1;
    if up {
        if d == b'9' {
            d = b'0';
            let mut digits = int.into_bytes();
            let mut i = digits.len();
            loop {
                if i == 0 {
                    digits.insert(0, b'1');
                    break;
                }
                i -= 1;
                if digits[i] == b'9' {
                    digits[i] = b'0';
                } else {
                    digits[i] += 1;
                    break;
                }
            }
            // The digits are ASCII by construction.
            int = String::from_utf8_lossy(&digits).into_owned();
        } else {
            d += 1;
        }
    }
    format!("{}{}.{}", if neg { "-" } else { "" }, int, char::from(d))
}

/// Port of `formatSynaraServerStatus` (`apps/server/src/serverStatusCli.ts:149`).
pub fn format_synara_server_status(result: serde_json::Value) -> Result<String, String> {
    let reachable = member(&result, "reachable")?;
    if !truthy(&reachable) {
        return Ok(format!(
            "Synara server: unreachable\nURL: {}\nError: {}",
            js_str(&member(&result, "url")?),
            js_str(&member(&result, "error")?),
        ));
    }

    let health = member(&result, "health")?;
    let projection = member(&health, "projection")?;
    let projection_state = if nullish(&projection) {
        crate::js::undefined()
    } else {
        member(&projection, "state")?
    };
    let status = if truthy(&member(&result, "ready")?) {
        "ready"
    } else if truthy(&member(&health, "startupReady")?) {
        "not ready"
    } else {
        "starting"
    };
    let loop_status = member(&health, "eventLoop")?;
    let mut lines = vec![
        format!("Synara server: {status}"),
        format!("URL: {}", js_str(&member(&result, "url")?)),
    ];
    if truthy(&projection_state) {
        lines.push(format!("Projection: {}", js_str(&projection_state)));
    }
    if truthy(&loop_status) {
        if truthy(&member(&loop_status, "available")?) {
            lines.push(format!(
                "Event loop: p50 {}ms / p99 {}ms / max {}ms; ELU {}%; stall windows {}",
                round_str(&member(&loop_status, "delayP50Ms")?),
                round_str(&member(&loop_status, "delayP99Ms")?),
                round_str(&member(&loop_status, "delayMaxMs")?),
                to_fixed1(to_number(&member(&loop_status, "utilization")?) * 100.0),
                js_str(&member(&loop_status, "stallWindowCount")?),
            ));
            let gap_count = member(&loop_status, "discardedIdleGapCount")?;
            if truthy(&gap_count) {
                let gap_ms = member(&loop_status, "discardedIdleGapMs")?;
                let gap_ms = if nullish(&gap_ms) {
                    Value::from(0)
                } else {
                    gap_ms
                };
                lines.push(format!(
                    "Ambiguous idle gaps: {} ({}ms total; suspend or scheduling pressure)",
                    js_str(&gap_count),
                    round_str(&gap_ms),
                ));
            }
            let last_stall = member(&loop_status, "lastStall")?;
            if truthy(&last_stall) {
                lines.push(format!(
                    "Last stall: {}ms ({}ms ago)",
                    round_str(&member(&last_stall, "durationMs")?),
                    round_str(&member(&last_stall, "ageMs")?),
                ));
            }
        } else {
            lines.push("Event loop: monitoring unavailable".to_string());
        }
    }
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fmt(v: Value) -> Result<String, String> {
        format_synara_server_status(v)
    }

    #[test]
    fn unreachable_renders_url_and_error() {
        let v = json!({"reachable": false, "ready": false, "url": "http://a:1", "error": "nope"});
        assert_eq!(
            fmt(v).unwrap(),
            "Synara server: unreachable\nURL: http://a:1\nError: nope"
        );
        let v = json!({});
        assert_eq!(
            fmt(v).unwrap(),
            "Synara server: unreachable\nURL: undefined\nError: undefined"
        );
    }

    #[test]
    fn null_arg_is_the_member_typeerror() {
        assert_eq!(
            fmt(Value::Null).unwrap_err(),
            "Cannot read properties of null (reading 'reachable')"
        );
    }

    #[test]
    fn status_lines_follow_ready_and_startup_ready() {
        for (ready, startup_ready, want) in [
            (true, false, "ready"),
            (false, true, "not ready"),
            (false, false, "starting"),
        ] {
            let v = json!({"reachable": true, "ready": ready, "url": "u",
                "health": {"status": "x", "startupReady": startup_ready}});
            assert_eq!(fmt(v).unwrap(), format!("Synara server: {want}\nURL: u"));
        }
    }

    #[test]
    fn null_health_throws_on_projection() {
        let v = json!({"reachable": true, "ready": true, "url": "u", "health": null});
        assert_eq!(
            fmt(v).unwrap_err(),
            "Cannot read properties of null (reading 'projection')"
        );
    }

    #[test]
    fn event_loop_unavailable_and_projection() {
        let v = json!({"reachable": true, "ready": false, "url": "u",
            "health": {"status": "x", "startupReady": true, "eventLoop": [],
                "projection": {"state": "catching up"}}});
        assert_eq!(
            fmt(v).unwrap(),
            "Synara server: not ready\nURL: u\nProjection: catching up\n\
             Event loop: monitoring unavailable"
        );
        let v = json!({"reachable": true, "ready": true, "url": "u",
            "health": {"status": "x", "startupReady": true, "eventLoop": null}});
        assert_eq!(fmt(v).unwrap(), "Synara server: ready\nURL: u");
    }

    #[test]
    fn event_loop_detail_lines() {
        let v = json!({"reachable": true, "ready": true, "url": "u",
            "health": {"status": "ok", "startupReady": true,
                "eventLoop": {"available": true, "delayP50Ms": 1.4, "delayP99Ms": 9.6,
                    "delayMaxMs": 42, "utilization": 0.255, "stallWindowCount": 3,
                    "discardedIdleGapCount": 2, "discardedIdleGapMs": 7.5,
                    "lastStall": {"durationMs": 12.4, "ageMs": 99.6}}}});
        assert_eq!(
            fmt(v).unwrap(),
            "Synara server: ready\nURL: u\n\
             Event loop: p50 1ms / p99 10ms / max 42ms; ELU 25.5%; stall windows 3\n\
             Ambiguous idle gaps: 2 (8ms total; suspend or scheduling pressure)\n\
             Last stall: 12ms (100ms ago)"
        );
    }

    #[test]
    fn to_fixed1_matches_js() {
        for (x, want) in [
            (25.5, "25.5"),
            (0.25, "0.3"),
            (-0.25, "-0.2"),
            (0.0, "0.0"),
            (-0.0, "0.0"),
            (-0.04, "-0.0"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (1e21, "1e+21"),
        ] {
            assert_eq!(to_fixed1(x), want, "{x}");
        }
    }
}
