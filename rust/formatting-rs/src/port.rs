//! The Rust port of `packages/profile-ui/src/formatting.ts`.

#![forbid(unsafe_code)]

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Rough BCP-47 shape: alnum subtags joined by `-` (`en`, `en-US`, `abc`). Anything else
/// (`key_name-1` has `_`, `ns:key.path` has `:`) makes the Intl constructors throw.
fn valid_locale(locale: &Option<String>) -> Result<(), String> {
    let ok = |l: &str| {
        let mut it = l.split('-');
        it.next().is_some_and(|f| {
            (2..=8).contains(&f.len()) && f.bytes().all(|b| b.is_ascii_alphabetic())
        }) && it
            .all(|t| !t.is_empty() && t.len() <= 8 && t.bytes().all(|b| b.is_ascii_alphanumeric()))
    };
    if let Some(l) = locale
        && !ok(l)
    {
        return Err(format!("invalid language tag: {l}"));
    }
    Ok(())
}

/// JS `Math.round`: half toward +inf.
fn js_round(v: f64) -> f64 {
    let r = (v + 0.5).floor();
    if r == 0.0 && v.is_sign_negative() {
        -0.0
    } else {
        r
    }
}

/// Intl default rounding: half away from zero, to `digits` fraction digits.
fn half_expand(v: f64, digits: u32) -> f64 {
    let m = 10f64.powi(digits as i32);
    ((v * m).abs() + 0.5).floor() / m * v.signum()
}

/// The digits Intl prints at 0 fraction digits: Rust `{:.0}` below 1e21 (exact);
/// the shortest round-trip mantissa padded with zeros above it, like V8's repr expansion.
fn int_digits(v: f64) -> String {
    if v.abs() < 1e21 {
        return format!("{v:.0}");
    }
    let e = format!("{v:e}");
    let (m, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i64 = exp.parse().unwrap_or(0);
    let mut d: String = m.chars().filter(|c| c.is_ascii_digit()).collect();
    for _ in 0..(exp + 1 - d.len() as i64) {
        d.push('0');
    }
    if v.is_sign_negative() {
        format!("-{d}")
    } else {
        d
    }
}

/// `Intl.NumberFormat('en-US')` grouping: `,` every three integer digits.
fn grouped(v: f64, frac: u32) -> String {
    let s = if frac == 0 {
        int_digits(v)
    } else {
        format!("{:.*}", frac as usize, v)
    };
    let neg = s.starts_with('-');
    let s = s.trim_start_matches('-');
    let (int, tail) = s.split_once('.').map_or((s, ""), |(i, t)| (i, t));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if !tail.is_empty() {
        out.push('.');
        out.push_str(tail);
    }
    if neg { format!("-{out}") } else { out }
}

/// Port of `formatCompact` (`packages/profile-ui/src/formatting.ts:9`).
pub fn format_compact(value: Option<f64>, locale: Option<String>) -> Result<String, String> {
    let Some(v) = value.filter(|v| v.is_finite()) else {
        return Ok("—".into());
    };
    let a = v.abs();
    if a < 1000.0 {
        return Ok(crate::js::number_to_string(js_round(v)));
    }
    valid_locale(&locale)?;
    let (scale, suffix) = if a >= 1e9 {
        (1e9, "bn")
    } else if a >= 1e6 {
        (1e6, "m")
    } else {
        (1e3, "k")
    };
    let s = half_expand(v / scale, 1);
    let n = if s.fract() == 0.0 {
        format!("{s:.0}")
    } else {
        format!("{s:.1}")
    };
    Ok(format!("{n}{suffix}"))
}

/// Port of `formatNumber` (`packages/profile-ui/src/formatting.ts:34`).
pub fn format_number(value: Option<f64>, locale: Option<String>) -> Result<String, String> {
    let Some(v) = value.filter(|v| v.is_finite()) else {
        return Ok("—".into());
    };
    valid_locale(&locale)?;
    Ok(grouped(half_expand(v, 0), 0))
}

/// Port of `formatDays` (`packages/profile-ui/src/formatting.ts:41`).
pub fn format_days(value: f64, locale: Option<String>) -> Result<String, String> {
    let s = format_number(Some(value), locale)?;
    Ok(format!("{s} {}", if value == 1.0 { "day" } else { "days" }))
}

/// The JS `\s` class: unlike Rust's `is_whitespace` it excludes U+0085/U+180E/U+200B
/// and includes U+FEFF.
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

/// Words are runs split on `.`, `_`, `-` and JS whitespace; '/' and ':' are kept inside words.
fn words(s: &str) -> Vec<String> {
    s.split(|c: char| matches!(c, '.' | '_' | '-') || js_ws(c))
        .filter(|w| !w.is_empty())
        .map(String::from)
        .collect()
}

/// Port of `toDisplayName` (`packages/profile-ui/src/formatting.ts:46`).
pub fn to_display_name(basename: String) -> Result<String, String> {
    let ws = words(&basename);
    if ws.is_empty() {
        return Ok("Synara".into());
    }
    Ok(ws
        .iter()
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" "))
}

/// Port of `normalizeHandle` (`packages/profile-ui/src/formatting.ts:60`).
pub fn normalize_handle(value: String) -> Result<String, String> {
    let h: String = value
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .take(30)
        .collect();
    Ok(format!("@{}", if h.is_empty() { "synara" } else { &h }))
}

/// Howard Hinnant's `days_from_civil`: days since 1970-01-01 for a y/m(1-12)/d that may overflow.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = (m + 9).rem_euclid(12);
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    (
        if mp < 10 { y } else { y + 1 },
        if mp < 10 { mp + 3 } else { mp - 9 },
        doy - (153 * mp + 2) / 5 + 1,
    )
}

/// Port of `formatShortDate` (`packages/profile-ui/src/formatting.ts:71`).
pub fn format_short_date(
    day: serde_json::Value,
    locale: Option<String>,
) -> Result<serde_json::Value, String> {
    use serde_json::Value;
    // Falsy `day` returns null; a truthy non-string throws `day.split is not a function`.
    let falsy = crate::js::is_undefined(&day)
        || matches!(&day, Value::Null | Value::Bool(false))
        || matches!(&day, Value::String(s) if s.is_empty())
        || crate::js::as_number(&day).is_some_and(|v| v == 0.0 || v.is_nan());
    if falsy {
        return Ok(Value::Null);
    }
    let Value::String(day) = &day else {
        return Err("TypeError: day.split is not a function".into());
    };
    let parts: Vec<&str> = day.split('-').collect();
    if parts.len() < 3 {
        return Ok(Value::Null);
    }
    let num = |p: &str| -> Option<f64> {
        let t = p.trim();
        if t.is_empty() {
            None
        } else {
            t.parse::<f64>().ok()
        }
    };
    let (Some(y), Some(m), Some(d)) = (num(parts[0]), num(parts[1]), num(parts[2])) else {
        return Ok(Value::Null);
    };
    valid_locale(&locale)?;
    // `new Date(y, m - 1, d)` overflows months and days; years 0-99 map to 19xx.
    let y = y.trunc() as i64 + if (0.0..100.0).contains(&y) { 1900 } else { 0 };
    let mt = m.trunc() as i64 - 1;
    let (_, mo, da) = civil_from_days(
        days_from_civil(y + mt.div_euclid(12), mt.rem_euclid(12) + 1, 1) + d.trunc() as i64 - 1,
    );
    Ok(Value::from(format!("{} {}", MONTHS[(mo - 1) as usize], da)))
}

/// Port of `formatHourLabel` (`packages/profile-ui/src/formatting.ts:88`).
pub fn format_hour_label(hour: f64, locale: Option<String>) -> Result<String, String> {
    let h = ((hour % 24.0) + 24.0) % 24.0;
    if let Some(l) = &locale
        && !l.is_empty()
    {
        // Locale path: a real Date label — Intl validates the tag, then the date.
        valid_locale(&locale)?;
        if !hour.is_finite() {
            return Err("Invalid time value".into());
        }
        let hh = h.floor();
        let shown = if hh % 12.0 == 0.0 { 12.0 } else { hh % 12.0 };
        return Ok(format!(
            "{shown:.0} {}",
            if hh < 12.0 { "AM" } else { "PM" }
        ));
    }
    let shown = if h % 12.0 == 0.0 { 12.0 } else { h % 12.0 };
    Ok(format!(
        "{} {}",
        crate::js::number_to_string(shown),
        if h < 12.0 { "AM" } else { "PM" }
    ))
}

/// Port of `formatProviderLabel` (`packages/profile-ui/src/formatting.ts:100`).
pub fn format_provider_label(provider: serde_json::Value) -> Result<serde_json::Value, String> {
    let label = provider.as_str().and_then(|p| match p {
        "codex" => Some("Codex"),
        "claudeAgent" => Some("Claude"),
        "cursor" => Some("Cursor"),
        "antigravity" => Some("Antigravity"),
        "grok" => Some("Grok"),
        "droid" => Some("Droid"),
        "opencode" => Some("OpenCode"),
        "pi" => Some("Pi"),
        "devin" => Some("Devin"),
        "omp" => Some("Oh My Pi"),
        _ => None,
    });
    Ok(label.map_or_else(crate::js::undefined, serde_json::Value::from))
}

/// Port of `formatProfileUsageBasis` (`packages/profile-ui/src/formatting.ts:125`).
pub fn format_profile_usage_basis(metric: String) -> Result<String, String> {
    Ok(if metric == "tokens" {
        "tracked tokens"
    } else {
        "turns"
    }
    .into())
}

/// JS `toUpperCase` on a UTF-16 unit sequence, preserving lone surrogates.
fn upper_units(units: &[u16]) -> Vec<u16> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let w = units[i];
        if (0xD800..0xDC00).contains(&w)
            && i + 1 < units.len()
            && (0xDC00..0xE000).contains(&units[i + 1])
        {
            out.extend(
                String::from_utf16(&units[i..i + 2])
                    .unwrap_or_default()
                    .to_uppercase()
                    .encode_utf16(),
            );
            i += 2;
        } else if !(0xD800..0xE000).contains(&w) {
            let up: String = char::from_u32(w as u32)
                .unwrap_or_default()
                .to_uppercase()
                .collect();
            out.extend(up.encode_utf16());
            i += 1;
        } else {
            out.push(w);
            i += 1;
        }
    }
    out
}

/// Port of `deriveInitials` (`packages/profile-ui/src/formatting.ts:134`).
pub fn derive_initials(name: String) -> Result<serde_json::Value, String> {
    let mut it = words(&name).into_iter();
    let u = |w: &str| w.encode_utf16().collect::<Vec<u16>>();
    let units: Vec<u16> = match (it.next(), it.next()) {
        (Some(a), Some(b)) => [u(&a)[..1].to_vec(), u(&b)[..1].to_vec()].concat(),
        (Some(a), None) => u(&a)[..u(&a).len().min(2)].to_vec(),
        _ => "Sy".encode_utf16().collect(),
    };
    Ok(crate::js::utf16_string(&upper_units(&units)))
}
