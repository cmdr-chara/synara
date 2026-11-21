//! Shared ECMAScript semantics for the Rust ports: helpers whose JS behaviour
//! differs from the Rust standard library or serde defaults.

#![forbid(unsafe_code)]

use serde_json::Value;

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
