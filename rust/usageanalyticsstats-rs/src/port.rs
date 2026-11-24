//! The Rust port of `apps/server/src/usageAnalyticsStats.ts`.
//!
//! Pure helpers only: window resolution, list-price estimation, reporting-provider
//! detection, and the two row builders (projection rows and session-file rows).

#![forbid(unsafe_code)]

#[path = "corpus.rs"]
mod corpus;

use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, OnceLock};

use crate::js::{as_number, is_undefined, math_round, number, number_to_string, string_units};
use crate::utf16;

static UNDEF: LazyLock<Value> = LazyLock::new(crate::js::undefined);

const DAY_MS: f64 = 86_400_000.0;
const MAX_WINDOW_MS: f64 = 373.0 * DAY_MS;
const QUARTER_MS: f64 = 15.0 * 60.0 * 1000.0;
const UNPRICEABLE: &[&str] = &[
    "<synthetic>",
    "synthetic",
    "opus",
    "sonnet",
    "haiku",
    "fable",
];
const PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "devin",
    "omp",
];

// ── JS value helpers ─────────────────────────────────────────────────────────

fn is_nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn member<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&UNDEF),
        _ => &UNDEF,
    }
}

fn read_err(v: &Value, key: &str) -> Option<String> {
    if v.is_null() {
        Some(format!("Cannot read properties of null (reading '{key}')"))
    } else if is_undefined(v) {
        Some(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ))
    } else {
        None
    }
}

fn member_err<'a>(obj: &'a Value, key: &str) -> Result<&'a Value, String> {
    match read_err(obj, key) {
        Some(e) => Err(e),
        None => Ok(member(obj, key)),
    }
}

/// Any JS string value (`String` or a `$utf16` marker) as Rust text.
fn as_js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => string_units(v).map(|u| String::from_utf16_lossy(&u)),
    }
}

fn non_empty_string(v: &Value) -> Option<String> {
    let s = as_js_string(v)?;
    let t = js_trim(&s);
    (!t.is_empty()).then(|| t.to_string())
}

fn bigint_str(v: &Value) -> Option<String> {
    let o = v.as_object()?;
    if o.len() != 1 {
        return None;
    }
    o.get("$bigint")?.as_str().map(str::to_string)
}

fn date_marker(v: &Value) -> Option<String> {
    let o = v.as_object()?;
    if o.len() != 1 {
        return None;
    }
    o.get("$date")?.as_str().map(str::to_string)
}

/// `Number(v)` — strings parse, arrays join like their `toString` does.
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i64::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        Value::Array(a) => parse_number(
            &a.iter()
                .map(|e| {
                    if is_nullish(e) {
                        String::new()
                    } else {
                        js_template_string(e)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => {
            if let Some(x) = as_number(v) {
                x
            } else if is_undefined(v) {
                f64::NAN
            } else if let Some(u) = string_units(v) {
                parse_number(&String::from_utf16_lossy(&u))
            } else if let Some(b) = bigint_str(v) {
                b.parse::<f64>().unwrap_or(f64::NAN)
            } else if let Some(d) = date_marker(v) {
                iso_date_parse(&d)
            } else if let Some(bytes) = marker_array(v, "$bytes").or(marker_array(v, "$buffer")) {
                parse_number(
                    &bytes
                        .iter()
                        .map(js_template_string)
                        .collect::<Vec<_>>()
                        .join(","),
                )
            } else {
                f64::NAN
            }
        }
    }
}

/// JS string→number: trim, empty → 0, then the standard grammar subset.
fn parse_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" | "infinity" | "+infinity" => return f64::INFINITY,
        "-Infinity" | "-infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let (neg, body) = match t.strip_prefix('-') {
        Some(b) => (true, b),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let v = if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        u128::from_str_radix(h, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else if let Some(b) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        u128::from_str_radix(b, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else if let Some(o) = body.strip_prefix("0o").or_else(|| body.strip_prefix("0O")) {
        u128::from_str_radix(o, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN)
    } else {
        t.parse::<f64>().unwrap_or(f64::NAN)
    };
    if neg { -v } else { v }
}

/// `num` from profileStats.ts: finite numbers pass through, anything else → 0.
fn num(v: &Value) -> f64 {
    if let Some(n) = as_number(v) {
        return if n.is_finite() { n } else { 0.0 };
    }
    if let Some(b) = bigint_str(v) {
        let parsed = b.parse::<f64>().unwrap_or(f64::NAN);
        return if parsed.is_finite() { parsed } else { 0.0 };
    }
    let parsed = js_to_number(v);
    if parsed.is_finite() { parsed } else { 0.0 }
}

/// `${v}` / `String(v)` coercion, minus objects' `valueOf`.
fn js_template_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    js_template_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else if let Some(x) = as_number(v) {
                number_to_string(x)
            } else if let Some(u) = string_units(v) {
                String::from_utf16_lossy(&u)
            } else if let Some(b) = bigint_str(v) {
                b
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

/// One element of `Array.prototype.join`: nullish → "".
fn join_element(v: &Value) -> String {
    if is_nullish(v) {
        String::new()
    } else {
        js_template_string(v)
    }
}

/// `[a, b, c, projectId ?? "", e].join("\u{0}")` — the merge-row key shape.
fn join_key(parts: &[Value]) -> String {
    parts
        .iter()
        .map(join_element)
        .collect::<Vec<_>>()
        .join("\u{0}")
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if is_undefined(v) {
                false
            } else if let Some(x) = as_number(v) {
                x != 0.0 && !x.is_nan()
            } else if let Some(u) = string_units(v) {
                !u.is_empty()
            } else if let Some(b) = bigint_str(v) {
                b != "0"
            } else {
                true
            }
        }
    }
}

/// `ToPrimitive` for `+` and comparisons.
fn js_primitive(v: &Value) -> Value {
    match v {
        Value::Array(_) => Value::String(js_template_string(v)),
        Value::Object(_) => {
            if let Some(x) = as_number(v) {
                number(x)
            } else if is_undefined(v) {
                crate::js::undefined()
            } else if let Some(u) = string_units(v) {
                Value::String(String::from_utf16_lossy(&u))
            } else if let Some(b) = bigint_str(v) {
                Value::String(b)
            } else {
                Value::String("[object Object]".to_string())
            }
        }
        _ => v.clone(),
    }
}

/// JS `a + b`: a string operand concatenates, otherwise numbers add.
fn js_add(a: &Value, b: &Value) -> Value {
    let pa = js_primitive(a);
    let pb = js_primitive(b);
    if matches!(pa, Value::String(_)) || matches!(pb, Value::String(_)) {
        return Value::String(js_template_string(&pa) + &js_template_string(&pb));
    }
    number(js_to_number(&pa) + js_to_number(&pb))
}

/// JS Math.min/Math.max: NaN propagates; -0 wins min, +0 wins max.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        return if a.is_sign_negative() || b.is_sign_negative() {
            -0.0
        } else {
            0.0
        };
    }
    if a < b { a } else { b }
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a == b {
        return if a.is_sign_positive() || b.is_sign_positive() {
            0.0
        } else {
            -0.0
        };
    }
    if a > b { a } else { b }
}

/// `a === b` (strict): NaN never equals.
fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => {
            if let (Some(x), Some(y)) = (as_number(a), as_number(b)) {
                return x == y;
            }
            if let (Some(x), Some(y)) = (string_units(a), string_units(b)) {
                return x == y;
            }
            if is_undefined(a) && is_undefined(b) {
                return true;
            }
            if let (Some(x), Some(y)) = (bigint_str(a), bigint_str(b)) {
                return x == y;
            }
            false
        }
    }
}

/// `SameValueZero` — `===` plus NaN equals NaN (Map/Set membership).
fn svz_eq(a: &Value, b: &Value) -> bool {
    strict_eq(a, b)
        || matches!((as_number(a), as_number(b)), (Some(x), Some(y)) if x.is_nan() && y.is_nan())
}

fn js_str_eq(v: &Value, s: &str) -> bool {
    string_units(v).is_some_and(|u| u == s.encode_utf16().collect::<Vec<u16>>())
}

fn marker_array<'a>(v: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    let o = v.as_object()?;
    if o.len() != 1 {
        return None;
    }
    o.get(key)?.as_array()
}

/// `for (const x of <expr>)`: the iterables JS accepts, V8's error otherwise.
fn js_iter(v: &Value, expr: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Object(_) => {
            if let Some(items) = marker_array(v, "$set") {
                return Ok(items.clone());
            }
            if let Some(entries) = marker_array(v, "$map") {
                return Ok(entries.iter().filter(|e| e.is_array()).cloned().collect());
            }
            if let Some(u) = string_units(v) {
                return Ok(utf16_code_points(&u));
            }
            if let Some(bytes) = marker_array(v, "$bytes").or(marker_array(v, "$buffer")) {
                return Ok(bytes.clone());
            }
            Err(format!("{expr} is not iterable"))
        }
        _ => Err(format!("{expr} is not iterable")),
    }
}

/// String iteration yields code points; a lone surrogate stays a marker.
fn utf16_code_points(units: &[u16]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        if (0xD800..0xDC00).contains(&u)
            && i + 1 < units.len()
            && (0xDC00..0xE000).contains(&units[i + 1])
        {
            out.push(Value::String(String::from_utf16_lossy(&units[i..i + 2])));
            i += 2;
        } else if (0xD800..0xE000).contains(&u) {
            out.push(json!({ "$utf16": [u] }));
            i += 1;
        } else {
            out.push(Value::String(String::from_utf16_lossy(&units[i..i + 1])));
            i += 1;
        }
    }
    out
}

/// `new Set(x)`: nullish → empty; non-iterables get the constructor's own message.
fn new_set_from(v: &Value) -> Result<JsSet, String> {
    let mut set = JsSet::default();
    if is_nullish(v) {
        return Ok(set);
    }
    match js_iter(v, "") {
        Ok(items) => {
            for item in items {
                set.add(item);
            }
            Ok(set)
        }
        Err(_) => Err(set_not_iterable_error(v)),
    }
}

fn set_not_iterable_error(v: &Value) -> String {
    const TAIL: &str = "is not iterable (cannot read property Symbol(Symbol.iterator))";
    match v {
        Value::Number(n) => format!(
            "number {} {TAIL}",
            number_to_string(n.as_f64().unwrap_or(f64::NAN))
        ),
        Value::Bool(b) => format!("boolean {b} {TAIL}"),
        Value::Object(_) => {
            if let Some(x) = as_number(v) {
                format!("number {} {TAIL}", number_to_string(x))
            } else if let Some(b) = bigint_str(v) {
                format!("bigint {b} {TAIL}")
            } else {
                format!("object {TAIL}")
            }
        }
        _ => format!("object {TAIL}"),
    }
}

/// `Set.prototype.has` / `Map.prototype.has` on a structured value.
fn container_has(v: &Value, item: &Value) -> Option<bool> {
    if let Some(items) = marker_array(v, "$set") {
        return Some(items.iter().any(|x| svz_eq(x, item)));
    }
    if let Some(entries) = marker_array(v, "$map") {
        return Some(
            entries
                .iter()
                .filter_map(Value::as_array)
                .any(|p| p.first().is_some_and(|k| svz_eq(k, item))),
        );
    }
    None
}

/// A `Map`'s `[[k, v], ...]` entries, or `None` when `v` is no Map.
fn map_entries(v: &Value) -> Option<Vec<(&Value, &Value)>> {
    let entries = marker_array(v, "$map")?;
    Some(
        entries
            .iter()
            .filter_map(Value::as_array)
            .filter(|p| p.len() >= 2)
            .map(|p| (&p[0], &p[1]))
            .collect(),
    )
}

/// SameValueZero-keyed insertion-ordered set.
#[derive(Default)]
struct JsSet {
    keys: HashSet<String>,
    values: Vec<Value>,
}

impl JsSet {
    fn add(&mut self, v: Value) {
        if self.keys.insert(svz_key(&v)) {
            self.values.push(v);
        }
    }
    fn has(&self, v: &Value) -> bool {
        self.keys.contains(&svz_key(v))
    }
    fn has_str(&self, s: &str) -> bool {
        self.has(&Value::String(s.to_string()))
    }
}

/// Insertion-ordered map keyed by a canonical SameValueZero string.
#[derive(Default)]
struct JsMap {
    index: HashMap<String, usize>,
    entries: Vec<(String, Value)>,
}

impl JsMap {
    fn get(&self, key: &str) -> Option<&Value> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }
    fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.index.get(key).map(|&i| &mut self.entries[i].1)
    }
    fn has(&self, key: &str) -> bool {
        self.index.contains_key(key)
    }
    fn set(&mut self, key: String, v: Value) {
        if let Some(&i) = self.index.get(&key) {
            self.entries[i].1 = v;
        } else {
            self.index.insert(key.clone(), self.entries.len());
            self.entries.push((key, v));
        }
    }
    fn values(&self) -> impl Iterator<Item = &Value> {
        self.entries.iter().map(|(_, v)| v)
    }
    fn values_mut(&mut self) -> impl Iterator<Item = &mut Value> {
        self.entries.iter_mut().map(|(_, v)| v)
    }
}

fn svz_key(v: &Value) -> String {
    if let Some(n) = as_number(v) {
        let n = if n == 0.0 { 0.0 } else { n };
        return format!(
            "n:{}",
            if n.is_nan() {
                "NaN".to_string()
            } else {
                number_to_string(n)
            }
        );
    }
    if let Some(u) = string_units(v) {
        return format!("s:{}", String::from_utf16_lossy(&u));
    }
    match v {
        Value::Null => "z".to_string(),
        Value::Bool(b) => format!("b:{b}"),
        _ => {
            if is_undefined(v) {
                "u".to_string()
            } else if let Some(b) = bigint_str(v) {
                format!("g:{b}")
            } else {
                format!("o:{}", serde_json::to_string(v).unwrap_or_default())
            }
        }
    }
}

/// `mergeInto` — sums into the existing row, else inserts it (insertion order).
fn merge_into(map: &mut JsMap, key: String, row: Value, merge: impl FnOnce(&mut Value, &Value)) {
    if let Some(existing) = map.get_mut(&key) {
        merge(existing, &row);
    } else {
        map.set(key, row);
    }
}

/// `existing[key] += add` — the JS `+` (concatenates a string operand).
fn obj_add(obj: &mut Value, key: &str, add: &Value) {
    let cur = member(obj, key).clone();
    if let Some(slot) = obj.as_object_mut().and_then(|m| m.get_mut(key)) {
        *slot = js_add(&cur, add);
    }
}

// ── Dates ────────────────────────────────────────────────────────────────────

fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    era * 146097 + (yoe * 365 + yoe / 4 - yoe / 100 + doy) - 719468
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

fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ => {
            if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                29
            } else {
                28
            }
        }
    }
}

/// `Date.parse` subset: `YYYY-MM-DD` (UTC) and `YYYY-MM-DD[ T]HH:mm[:ss[.fff]][Z|±HH[:mm]]`.
fn date_parse_str(s: &str) -> f64 {
    let parsed = iso_date_parse(s);
    if parsed.is_nan() {
        corpus::date_ms(s.trim()).unwrap_or(f64::NAN)
    } else {
        parsed
    }
}

fn iso_date_parse(s: &str) -> f64 {
    let s = s.trim();
    let b = s.as_bytes();
    let num_at = |i: usize, n: usize| -> Option<i64> {
        b.get(i..i + n)
            .filter(|w| w.iter().all(|c| c.is_ascii_digit()))
            .and_then(|w| std::str::from_utf8(w).ok()?.parse().ok())
    };
    let (y, mo, d) = match (num_at(0, 4), b.get(4), num_at(5, 2), b.get(7), num_at(8, 2)) {
        (Some(y), Some(b'-'), Some(mo), Some(b'-'), Some(d)) if (1..=12).contains(&mo) => {
            (y, mo, d)
        }
        _ => return f64::NAN,
    };
    if !(1..=31).contains(&d) || d > days_in_month(y, mo) {
        return f64::NAN;
    }
    if b.len() == 10 {
        return (days_from_civil(y, mo, d) * 86_400_000) as f64;
    }
    if !matches!(b.get(10), Some(b'T') | Some(b' ')) {
        return f64::NAN;
    }
    let (hh, mm) = match (num_at(11, 2), b.get(13), num_at(14, 2)) {
        (Some(h), Some(b':'), Some(m)) if h <= 24 && m <= 59 => (h, m),
        _ => return f64::NAN,
    };
    let mut idx = 16;
    let mut ss = 0i64;
    let mut frac = 0.0f64;
    if b.get(idx) == Some(&b':') {
        match num_at(idx + 1, 2) {
            Some(s2) if s2 <= 60 => ss = s2,
            _ => return f64::NAN,
        }
        idx += 3;
        if b.get(idx) == Some(&b'.') {
            let start = idx + 1;
            let mut end = start;
            while b.get(end).is_some_and(|c| c.is_ascii_digit()) {
                end += 1;
            }
            match std::str::from_utf8(&b[start..end]).ok().and_then(|t| {
                if t.is_empty() {
                    None
                } else {
                    t[..t.len().min(3)].parse::<f64>().ok()
                }
            }) {
                Some(f) => frac = f / 10f64.powi(end.min(start + 3) as i32 - start as i32),
                None => return f64::NAN,
            }
            idx = end;
        }
    }
    let offset_ms = match b.get(idx) {
        None => 0.0,
        Some(b'Z') if idx == b.len() - 1 => 0.0,
        Some(sign @ (b'+' | b'-')) => {
            let (oh, om, consumed) =
                match (num_at(idx + 1, 2), b.get(idx + 3), num_at(idx + 4, 2)) {
                    // "+HH:MM" consumes 6 bytes; "+HH" consumes 3.
                    (Some(h), Some(b':'), Some(m)) => (h, m, 6usize),
                    (Some(h), None, _) if idx + 3 == b.len() => (h, 0, 3usize),
                    _ => return f64::NAN,
                };
            if idx + consumed != b.len() {
                return f64::NAN;
            }
            let total = (oh * 60 + om) as f64 * 60_000.0;
            if *sign == b'-' { total } else { -total }
        }
        _ => return f64::NAN,
    };
    (days_from_civil(y, mo, d) * 86_400_000 + hh * 3_600_000 + mm * 60_000 + ss * 1_000) as f64
        + frac
        + offset_ms
}

/// `Date.parse(v)`: `ToString(v)` then parse.
fn date_parse(v: &Value) -> f64 {
    match v {
        Value::String(s) => date_parse_str(s),
        _ => {
            if let Some(u) = string_units(v) {
                date_parse_str(&String::from_utf16_lossy(&u))
            } else if let Some(d) = date_marker(v) {
                if d == "Invalid" {
                    f64::NAN
                } else {
                    date_parse_str(&d)
                }
            } else {
                date_parse_str(&js_template_string(v))
            }
        }
    }
}

/// `new Date(v)` (single-arg): number → ms, string → parse, then ToPrimitive.
fn new_date_ms(v: &Value) -> Result<f64, String> {
    if let Some(n) = as_number(v) {
        return Ok(n);
    }
    Ok(match v {
        Value::Bool(b) => i64::from(*b) as f64,
        Value::Null => 0.0,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => date_parse_str(s),
        Value::Array(a) => date_parse_str(
            &a.iter()
                .map(|e| {
                    if is_nullish(e) {
                        String::new()
                    } else {
                        js_template_string(e)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(_) => {
            if bigint_str(v).is_some() {
                return Err("Cannot convert a BigInt value to a number".to_string());
            }
            if let Some(u) = string_units(v) {
                date_parse_str(&String::from_utf16_lossy(&u))
            } else if let Some(d) = date_marker(v) {
                if d == "Invalid" {
                    f64::NAN
                } else {
                    date_parse_str(&d)
                }
            } else {
                f64::NAN
            }
        }
    })
}

/// `new Date(ms).toISOString()` — `Err("Invalid time value")` out of range.
fn to_iso_string(ms: f64) -> Result<String, String> {
    if !ms.is_finite() || ms.abs() > 8.64e15 {
        return Err("Invalid time value".to_string());
    }
    let ms = ms.trunc();
    let days = (ms / DAY_MS).floor() as i64;
    let rem = ms - days as f64 * DAY_MS;
    let (y, mo, d) = civil_from_days(days);
    let h = (rem / 3_600_000.0).floor() as i64;
    let mi = ((rem - h as f64 * 3_600_000.0) / 60_000.0).floor() as i64;
    let se = ((rem - h as f64 * 3_600_000.0 - mi as f64 * 60_000.0) / 1000.0).floor() as i64;
    let milli = (rem - h as f64 * 3_600_000.0 - mi as f64 * 60_000.0 - se as f64 * 1000.0) as i64;
    Ok(format!(
        "{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{se:02}.{milli:03}Z"
    ))
}

/// `utcHour`: start of the UTC quarter hour as an ISO instant.
fn utc_hour(timestamp_ms: f64) -> Result<String, String> {
    to_iso_string((timestamp_ms / QUARTER_MS).floor() * QUARTER_MS)
}

// ── Provider/model naming (profileStats.ts + recapStats.ts) ────────────────────

fn infer_kind_from_instance(instance_id: &str) -> Option<&'static str> {
    let lower = instance_id.to_lowercase();
    if lower.starts_with("claude") {
        Some("claudeAgent")
    } else if lower.starts_with("codex") {
        Some("codex")
    } else if lower.starts_with("cursor") {
        Some("cursor")
    } else if lower.starts_with("antigravity")
        || lower.starts_with("gemini")
        || lower.starts_with("agy")
    {
        Some("antigravity")
    } else if lower.starts_with("grok") {
        Some("grok")
    } else if lower.starts_with("droid") {
        Some("droid")
    } else if lower.starts_with("opencode")
        || lower.starts_with("open_code")
        || lower.starts_with("kilo")
    {
        Some("opencode")
    } else if lower.starts_with("pi") {
        Some("pi")
    } else if lower.starts_with("devin") {
        Some("devin")
    } else {
        None
    }
}

fn infer_kind_from_model(model: &str) -> &'static str {
    let lower = model.to_lowercase();
    if lower.contains("claude")
        || lower.contains("sonnet")
        || lower.contains("opus")
        || lower.contains("haiku")
    {
        "claudeAgent"
    } else if lower.contains("gemini") || lower.contains("antigravity") {
        "antigravity"
    } else if lower.contains("grok") {
        "grok"
    } else if lower.contains("droid") {
        "droid"
    } else if lower.contains("devin") {
        "devin"
    } else if lower.contains("opencode") || lower.contains("open_code") || lower.contains("kilo") {
        "opencode"
    } else if lower.contains("cursor") {
        "cursor"
    } else if lower.starts_with("pi/") || lower.contains("/pi/") {
        "pi"
    } else {
        "codex"
    }
}

/// `normalizeProviderKind(value)`: known kinds, legacy instance ids, else "unknown".
fn normalize_provider_kind(provider: &Value, model: &Value) -> &'static str {
    if let Some(p) = non_empty_string(provider) {
        if let Some(kind) = PROVIDER_KINDS.iter().find(|&&k| k == p) {
            return kind;
        }
        if let Some(kind) = infer_kind_from_instance(&p) {
            return kind;
        }
    }
    match non_empty_string(model) {
        Some(m) => infer_kind_from_model(&m),
        None => "unknown",
    }
}

fn data() -> &'static Value {
    static DATA: OnceLock<Value> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../model-rs/src/model_data.json"))
            .unwrap_or(Value::Null)
    })
}

fn strip_claude_context_window_suffix(model: &str) -> String {
    if !model.ends_with(']') {
        return model.to_string();
    }
    match model[..model.len() - 1].rfind('[') {
        Some(open) => model[..open].to_string(),
        None => model.to_string(),
    }
}

fn claude_suffix(model: &str) -> Option<String> {
    if !model.ends_with(']') {
        return None;
    }
    let open = model[..model.len() - 1].rfind('[')?;
    Some(model[open + 1..model.len() - 1].to_lowercase())
}

fn strip_date_suffix(model: &str) -> &str {
    let bytes = model.as_bytes();
    if bytes.len() >= 9
        && bytes[bytes.len() - 9] == b'-'
        && bytes[bytes.len() - 8..].iter().all(|c| c.is_ascii_digit())
    {
        &model[..model.len() - 9]
    } else {
        model
    }
}

/// `normalizeModelSlug` — provider keyed alias table lookup.
fn normalize_model_slug(slug: &str, provider: &Value) -> Result<Option<String>, String> {
    let trimmed = js_trim(slug);
    if trimmed.is_empty() {
        return Ok(None);
    }
    let provider = if is_undefined(provider) {
        "codex"
    } else {
        provider.as_str().unwrap_or("")
    };
    let scoped: String = if provider == "claudeAgent" {
        strip_claude_context_window_suffix(trimmed)
    } else if provider == "devin"
        && trimmed == trimmed.to_lowercase()
        && trimmed.ends_with("-medium")
    {
        trimmed[..trimmed.len() - "-medium".len()].to_string()
    } else {
        trimmed.to_string()
    };
    let Some(aliases) = data()
        .get("MODEL_SLUG_ALIASES_BY_PROVIDER")
        .and_then(Value::as_object)
        .and_then(|t| t.get(provider))
    else {
        return Err("Cannot convert undefined or null to object".to_string());
    };
    let normalized = aliases
        .get(scoped.to_lowercase())
        .and_then(Value::as_str)
        .unwrap_or(&scoped)
        .to_string();
    if provider == "claudeAgent" && claude_suffix(trimmed).as_deref() == Some("1m") {
        return Ok(Some(format!("{normalized}[1m]")));
    }
    Ok(Some(normalized))
}

/// `recapModelName(provider, model)`.
fn recap_model_name_str(provider: &Value, model: Option<String>) -> Result<String, String> {
    let Some(slug) = model.as_deref().and_then(|m| {
        let t = js_trim(m);
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    }) else {
        return Ok("unknown".to_string());
    };
    let normalized = if provider.as_str() == Some("unknown") {
        slug
    } else {
        match normalize_model_slug(&slug, provider)? {
            Some(s) => s,
            None => slug,
        }
    };
    Ok(strip_date_suffix(&strip_claude_context_window_suffix(&normalized)).to_string())
}

/// `identity` in buildUsageAnalytics: provider normalized alone, model via recap naming.
fn identity(provider: &Value, model: &Value) -> Result<(String, String), String> {
    let p = normalize_provider_kind(provider, &Value::Null).to_string();
    let m = recap_model_name_str(&Value::String(p.clone()), non_empty_string(model))?;
    Ok((p, m))
}

fn analytics_origin(v: &Value) -> &'static str {
    if js_str_eq(v, "automation") {
        "automation"
    } else if js_str_eq(v, "agent") {
        "agent"
    } else {
        "user"
    }
}

// ── Pricing (usagePricing.ts) ──────────────────────────────────────────────────

fn bare_name(key: &str) -> &str {
    match key.rfind('/') {
        Some(i) => &key[i + 1..],
        None => key,
    }
}

/// `table.get(key)` — `table` is the `RateTable` Map (`{"$map": [[k, v], ...]}`).
fn map_get<'a>(table: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    if let Some(e) = read_err(table, "get") {
        return Err(e);
    }
    let Some(entries) = map_entries(table) else {
        return Err("table.get is not a function".to_string());
    };
    Ok(entries
        .iter()
        .find(|(k, _)| js_str_eq(k, key))
        .map(|(_, v)| *v))
}

/// `lookupRate` — trimmed lowercase key, `[…]` context variant cut, bare family
/// names refused, dated snapshots fall back to the undated name.
fn lookup_rate<'a>(table: &'a Value, model: &Value) -> Result<Option<&'a Value>, String> {
    let Some(m) = as_js_string(model) else {
        return Err(
            read_err(model, "trim").unwrap_or_else(|| "model.trim is not a function".to_string())
        );
    };
    let key_full = js_trim(&m).to_lowercase();
    let key = match key_full.find('[') {
        Some(i) => &key_full[..i],
        None => key_full.as_str(),
    };
    if key.is_empty() || UNPRICEABLE.contains(&bare_name(key)) {
        return Ok(None);
    }
    if let Some(v) = map_get(table, key)?
        && !is_nullish(v)
    {
        return Ok(Some(v));
    }
    Ok(match map_get(table, strip_date_suffix(key))? {
        Some(v) if !is_nullish(v) => Some(v),
        _ => None,
    })
}

/// `priceTokens` — the four disjoint token parts at list price.
fn price_tokens(
    table: &Value,
    model: &str,
    input_tokens: f64,
    cache_read: f64,
    cache_write: f64,
    output: f64,
) -> Result<Option<f64>, String> {
    let Some(rate) = lookup_rate(table, &Value::String(model.to_string()))? else {
        return Ok(None);
    };
    if !js_truthy(rate) {
        return Ok(None);
    }
    Ok(Some(
        input_tokens * js_to_number(member_err(rate, "input")?)
            + cache_read * js_to_number(member_err(rate, "cacheRead")?)
            + cache_write * js_to_number(member_err(rate, "cacheWrite")?)
            + output * js_to_number(member_err(rate, "output")?),
    ))
}

/// `transcriptTokenTotal` — five disjoint parts; `+` may concatenate junk.
fn transcript_token_total(tokens: &Value) -> Result<Value, String> {
    let mut acc = member_err(tokens, "inputTokens")?.clone();
    for key in [
        "cacheReadTokens",
        "cacheWrite5mTokens",
        "cacheWrite1hTokens",
        "outputTokens",
    ] {
        acc = js_add(&acc, member_err(tokens, key)?);
    }
    Ok(acc)
}

/// `priceTranscriptUsage` — the file's own cost, else tokens at tiered rates.
fn price_transcript_usage(table: &Value, usage: &Value) -> Result<Value, String> {
    let reported = member_err(usage, "reportedCostUsd")?;
    if !reported.is_null() {
        return Ok(reported.clone());
    }
    let Some(rate) = lookup_rate(table, member(usage, "model"))? else {
        return Ok(Value::Null);
    };
    if !js_truthy(rate) {
        return Ok(Value::Null);
    }
    let tokens = member_err(usage, "tokens")?;
    let total = transcript_token_total(tokens)?;
    let prompt = js_to_number(&total) - js_to_number(member_err(tokens, "outputTokens")?);
    let mut rates = rate.clone();
    let tiers = member(rate, "tiers");
    for tier in js_iter(tiers, "rate.tiers")? {
        if prompt > js_to_number(member_err(&tier, "aboveTokens")?) {
            rates = member_err(&tier, "rates")?.clone();
        }
    }
    let cost = js_to_number(member_err(tokens, "inputTokens")?)
        * js_to_number(member_err(&rates, "input")?)
        + js_to_number(member_err(tokens, "cacheReadTokens")?)
            * js_to_number(member_err(&rates, "cacheRead")?)
        + js_to_number(member_err(tokens, "cacheWrite5mTokens")?)
            * js_to_number(member_err(&rates, "cacheWrite")?)
        + js_to_number(member_err(tokens, "cacheWrite1hTokens")?)
            * js_to_number(member_err(&rates, "cacheWrite1h")?)
        + js_to_number(member_err(tokens, "outputTokens")?)
            * js_to_number(member_err(&rates, "output")?);
    let mult = if js_truthy(member(usage, "fast")) {
        js_to_number(member_err(rate, "fastMultiplier")?)
    } else {
        1.0
    };
    Ok(number(cost * mult))
}

// ── Exported ports ───────────────────────────────────────────────────────────

/// Port of `resolveUsageAnalyticsWindow` (`apps/server/src/usageAnalyticsStats.ts:102`).
pub fn resolve_usage_analytics_window(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let from_ms = date_parse(member_err(&input, "from")?);
    let to_ms = date_parse(member_err(&input, "to")?);
    if !from_ms.is_finite() || !to_ms.is_finite() || from_ms >= to_ms {
        return Ok(json!({"error": "The analytics window must start before it ends."}));
    }
    if to_ms - from_ms > MAX_WINDOW_MS {
        return Ok(json!({
            "error": format!(
                "The analytics window can span at most {} days.",
                number_to_string(MAX_WINDOW_MS / DAY_MS)
            )
        }));
    }
    Ok(json!({
        "from": to_iso_string(from_ms)?,
        "to": to_iso_string(to_ms)?,
    }))
}

/// `estimateCost`'s inner computation: `(costUsd ?? 0, unpricedTokens, unsplitTokens)`.
fn estimate_cost_inner(
    prices: &Value,
    model: &str,
    total: f64,
    row: &Value,
) -> Result<(f64, f64, f64), String> {
    let split = js_min(
        total,
        js_max(0.0, math_round(num(member_err(row, "splitTokens")?))),
    );
    let cache_read = js_min(
        split,
        js_max(0.0, math_round(num(member_err(row, "splitCacheTokens")?))),
    );
    let output = js_min(
        split - cache_read,
        js_max(0.0, math_round(num(member_err(row, "outputTokens")?))),
    );
    let cache_write = js_min(
        split - cache_read - output,
        js_max(0.0, math_round(num(member_err(row, "cacheWriteTokens")?))),
    );
    let cost = if split > 0.0 {
        price_tokens(
            member_err(prices, "table")?,
            model,
            split - cache_read - output - cache_write,
            cache_read,
            cache_write,
            output,
        )?
    } else {
        None
    };
    Ok((
        cost.unwrap_or(0.0),
        if cost.is_none() { split } else { 0.0 },
        total - split,
    ))
}

/// Port of `estimateCost` (`apps/server/src/usageAnalyticsStats.ts:154`).
pub fn estimate_cost(
    prices: serde_json::Value,
    model: String,
    total: f64,
    row: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (cost_usd, unpriced, unsplit) = estimate_cost_inner(&prices, &model, total, &row)?;
    Ok(json!({
        "costUsd": number(cost_usd),
        "unpricedTokens": number(unpriced),
        "unsplitTokens": number(unsplit),
    }))
}

/// `reportingProviders` — a `Set` of normalized provider kinds, as `{"$set": [...]}`.
fn reporting_set(cost_rows: &Value) -> Result<JsSet, String> {
    let mut providers = JsSet::default();
    for row in js_iter(cost_rows, "costRows")? {
        if num(member_err(&row, "costUsd")?) > 0.0 {
            providers.add(Value::String(
                normalize_provider_kind(member_err(&row, "provider")?, &Value::Null).to_string(),
            ));
        }
    }
    Ok(providers)
}

/// Port of `reportingProviders` (`apps/server/src/usageAnalyticsStats.ts:187`).
pub fn reporting_providers(cost_rows: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(json!({ "$set": reporting_set(&cost_rows)?.values }))
}

/// Port of `needsPrices` (`apps/server/src/usageAnalyticsStats.ts:196`).
pub fn needs_prices(
    token_rows: serde_json::Value,
    reporting: serde_json::Value,
) -> Result<bool, String> {
    let rows = match &token_rows {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'some')".to_string());
        }
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'some')".to_string());
        }
        Value::Array(a) => a,
        _ => return Err("tokenRows.some is not a function".to_string()),
    };
    for row in rows {
        if num(member_err(row, "splitTokens")?) > 0.0 {
            if let Some(e) = read_err(&reporting, "has") {
                return Err(e);
            }
            let provider = Value::String(
                normalize_provider_kind(member_err(row, "provider")?, &Value::Null).to_string(),
            );
            let known = container_has(&reporting, &provider)
                .ok_or_else(|| "reporting.has is not a function".to_string())?;
            if !known {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

// ── buildUsageAnalytics ────────────────────────────────────────────────────────

/// `hourInstant` — `YYYY-MM-DDTHH:MM` to the quarter hour's start instant.
fn hour_instant(v: &Value) -> Result<Option<String>, String> {
    let Some(slot) = non_empty_string(v) else {
        return Ok(None);
    };
    let b = slot.as_bytes();
    let ok = b.len() == 16
        && b[0..4].iter().all(|c| c.is_ascii_digit())
        && b[4] == b'-'
        && b[5..7].iter().all(|c| c.is_ascii_digit())
        && b[7] == b'-'
        && b[8..10].iter().all(|c| c.is_ascii_digit())
        && b[10] == b'T'
        && b[11..13].iter().all(|c| c.is_ascii_digit())
        && b[13] == b':'
        && b[14..16].iter().all(|c| c.is_ascii_digit());
    Ok(if ok {
        Some(format!("{slot}:00.000Z"))
    } else {
        None
    })
}

/// `projectFor` — the project id when id and title are both non-empty strings.
fn project_for(projects: &mut JsMap, project_id: &Value, title: &Value) -> Option<String> {
    let id = non_empty_string(project_id)?;
    let name = non_empty_string(title)?;
    if !projects.has(&svz_key(&Value::String(id.clone()))) {
        projects.set(
            svz_key(&Value::String(id.clone())),
            json!({"projectId": id, "title": name}),
        );
    }
    Some(id)
}

/// `files?.key ?? []` spread into an array.
fn files_list(files: &Value, key: &str) -> Result<Vec<Value>, String> {
    let v = if is_nullish(files) {
        &UNDEF
    } else {
        member(files, key)
    };
    if is_nullish(v) {
        Ok(Vec::new())
    } else {
        js_iter(v, "((intermediate value) ?? [])")
    }
}

/// Default `toSorted()` — UTF-16 code-unit order of the elements' strings.
fn sort_default(items: &mut [Value]) {
    crate::js::sort(items, |a, b| {
        let (x, y) = (js_template_string(a), js_template_string(b));
        match x
            .encode_utf16()
            .collect::<Vec<u16>>()
            .cmp(&y.encode_utf16().collect::<Vec<u16>>())
        {
            Ordering::Less => -1.0,
            Ordering::Equal => 0.0,
            Ordering::Greater => 1.0,
        }
    })
}

/// `left.title.localeCompare(right.title)` — corpus order for recorded strings.
fn locale_compare_titles(l: &Value, r: &Value) -> Result<Ordering, String> {
    let lt = member_err(l, "title")?;
    let Some(lu) = string_units(lt) else {
        return Err(read_err(lt, "localeCompare")
            .unwrap_or_else(|| "left.title.localeCompare is not a function".to_string()));
    };
    let rt = member_err(r, "title")?;
    let rs = js_template_string(rt);
    let ls = String::from_utf16_lossy(&lu);
    Ok(match (corpus::rank(&ls), corpus::rank(&rs)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => lu.cmp(&rs.encode_utf16().collect::<Vec<u16>>()),
    })
}

/// Port of `buildUsageAnalytics` (`apps/server/src/usageAnalyticsStats.ts:205`).
pub fn build_usage_analytics(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let files = member_err(&input, "sessionFiles")?;
    let mut projects = JsMap::default();
    let mut costs = JsMap::default();
    let add_cost = |costs: &mut JsMap, row: Value| {
        merge_into(
            costs,
            join_key(&[
                member(&row, "hour").clone(),
                member(&row, "provider").clone(),
                member(&row, "model").clone(),
                member(&row, "projectId").clone(),
                member(&row, "source").clone(),
            ]),
            row,
            |existing, next| {
                obj_add(existing, "costUsd", member(next, "costUsd"));
                obj_add(existing, "turns", member(next, "turns"));
            },
        );
    };

    let reporting = reporting_set(member(&input, "costRows"))?;
    let files_providers = if is_nullish(files) {
        &UNDEF
    } else {
        member(files, "providers")
    };
    let from_files = new_set_from(files_providers)?;

    let mut tokens = JsMap::default();
    let mut providers_with_tokens = HashSet::new();
    for row in js_iter(member(&input, "tokenRows"), "input.tokenRows")? {
        let hour = hour_instant(member_err(&row, "hour")?)?;
        let total = math_round(num(member_err(&row, "tokens")?));
        let Some(hour) = hour else { continue };
        if total <= 0.0 {
            continue;
        }
        let (provider, model) =
            identity(member_err(&row, "provider")?, member_err(&row, "model")?)?;
        if from_files.has_str(&provider) {
            continue;
        }
        if provider != "unknown" {
            providers_with_tokens.insert(provider.clone());
        }
        let project_id = project_for(
            &mut projects,
            member_err(&row, "projectId")?,
            member_err(&row, "title")?,
        );
        let origin = analytics_origin(member_err(&row, "origin")?);
        let (cost_usd, unpriced, unsplit) = if reporting.has_str(&provider) {
            (0.0, 0.0, 0.0)
        } else {
            estimate_cost_inner(member(&input, "prices"), &model, total, &row)?
        };
        if cost_usd > 0.0 {
            add_cost(
                &mut costs,
                json!({
                    "hour": hour,
                    "provider": provider,
                    "model": model,
                    "projectId": project_id,
                    "source": "estimated",
                    "costUsd": number(cost_usd),
                    "turns": 0,
                }),
            );
        }
        merge_into(
            &mut tokens,
            join_key(&[
                Value::String(hour.clone()),
                Value::String(provider.clone()),
                Value::String(model.clone()),
                project_id.clone().map_or(Value::Null, Value::String),
                Value::String(origin.to_string()),
            ]),
            json!({
                "hour": hour,
                "provider": provider,
                "model": model,
                "projectId": project_id,
                "origin": origin,
                "tokens": number(total),
                "cacheReadTokens": number(js_min(
                    total,
                    js_max(0.0, math_round(num(member_err(&row, "cacheTokens")?))),
                )),
                "unpricedTokens": number(unpriced),
                "unsplitTokens": number(unsplit),
            }),
            |existing, next| {
                for key in [
                    "tokens",
                    "cacheReadTokens",
                    "unpricedTokens",
                    "unsplitTokens",
                ] {
                    obj_add(existing, key, member(next, key));
                }
            },
        );
    }

    let mut turns = JsMap::default();
    let mut providers_with_completed_turns = HashSet::new();
    for row in js_iter(member(&input, "turnRows"), "input.turnRows")? {
        let hour = hour_instant(member_err(&row, "hour")?)?;
        let count = math_round(num(member_err(&row, "turns")?));
        let Some(hour) = hour else { continue };
        if count <= 0.0 {
            continue;
        }
        let (provider, model) =
            identity(member_err(&row, "provider")?, member_err(&row, "model")?)?;
        if from_files.has_str(&provider) {
            continue;
        }
        if provider != "unknown" && num(member_err(&row, "completedTurns")?) > 0.0 {
            providers_with_completed_turns.insert(provider.clone());
        }
        let project_id = project_for(
            &mut projects,
            member_err(&row, "projectId")?,
            member_err(&row, "title")?,
        );
        merge_into(
            &mut turns,
            join_key(&[
                Value::String(hour.clone()),
                Value::String(provider.clone()),
                Value::String(model.clone()),
                project_id.clone().map_or(Value::Null, Value::String),
            ]),
            json!({
                "hour": hour,
                "provider": provider,
                "model": model,
                "projectId": project_id,
                "turns": number(count),
                "failedTurns": number(js_min(
                    count,
                    js_max(0.0, math_round(num(member_err(&row, "failedTurns")?))),
                )),
            }),
            |existing, next| {
                for key in ["turns", "failedTurns"] {
                    obj_add(existing, key, member(next, key));
                }
            },
        );
    }

    for row in js_iter(member(&input, "costRows"), "input.costRows")? {
        let hour = hour_instant(member_err(&row, "hour")?)?;
        let cost_usd = num(member_err(&row, "costUsd")?);
        let Some(hour) = hour else { continue };
        if cost_usd.partial_cmp(&0.0) != Some(Ordering::Greater) {
            continue;
        }
        let (provider, model) =
            identity(member_err(&row, "provider")?, member_err(&row, "model")?)?;
        if from_files.has_str(&provider) {
            continue;
        }
        let project_id = project_for(
            &mut projects,
            member_err(&row, "projectId")?,
            member_err(&row, "title")?,
        );
        add_cost(
            &mut costs,
            json!({
                "hour": hour,
                "provider": provider,
                "model": model,
                "projectId": project_id,
                "source": "reported",
                "costUsd": number(cost_usd),
                "turns": number(js_max(
                    0.0,
                    math_round(num(member_err(&row, "turns")?)),
                )),
            }),
        );
    }

    let generated_at = to_iso_string(new_date_ms(member_err(&input, "nowMs")?)?)?;
    let window = member_err(&input, "window")?;
    let from = member_err(window, "from")?.clone();
    let to = member_err(window, "to")?.clone();

    let mut tokens_out = tokens.values().cloned().collect::<Vec<_>>();
    tokens_out.extend(files_list(files, "tokens")?);
    let mut turns_out = turns.values().cloned().collect::<Vec<_>>();
    turns_out.extend(files_list(files, "turns")?);
    let mut costs_out = costs.values().cloned().collect::<Vec<_>>();
    costs_out.extend(files_list(files, "costs")?);

    let prices = member_err(&input, "prices")?;
    let pricing = json!({
        "status": member_err(prices, "status")?,
        "fetchedAt": member_err(prices, "fetchedAt")?,
    });
    let scope = {
        let s = member(&input, "scope");
        if is_nullish(s) {
            json!("synara")
        } else {
            s.clone()
        }
    };

    let mut session_file_providers = files_list(files, "providers")?;
    sort_default(&mut session_file_providers);

    let mut merged_projects = JsMap::default();
    let files_projects = files_list(files, "projects")?;
    for p in projects.values().chain(files_projects.iter()) {
        merged_projects.set(svz_key(member_err(p, "projectId")?), p.clone());
    }
    let mut project_list: Vec<Value> = merged_projects.values().cloned().collect();
    let sort_err = std::cell::RefCell::new(None::<String>);
    crate::js::sort(&mut project_list, |l, r| {
        if sort_err.borrow().is_some() {
            return 0.0;
        }
        match locale_compare_titles(l, r) {
            Ok(o) => match o {
                Ordering::Less => -1.0,
                Ordering::Equal => 0.0,
                Ordering::Greater => 1.0,
            },
            Err(e) => {
                *sort_err.borrow_mut() = Some(e);
                0.0
            }
        }
    });
    if let Some(e) = sort_err.into_inner() {
        return Err(e);
    }

    let mut unavailable: Vec<String> = providers_with_completed_turns
        .iter()
        .filter(|p| !providers_with_tokens.contains(*p))
        .cloned()
        .collect();
    crate::js::sort(&mut unavailable, |a, b| {
        match a
            .encode_utf16()
            .collect::<Vec<u16>>()
            .cmp(&b.encode_utf16().collect::<Vec<u16>>())
        {
            Ordering::Less => -1.0,
            Ordering::Equal => 0.0,
            Ordering::Greater => 1.0,
        }
    });

    Ok(json!({
        "generatedAt": generated_at,
        "from": from,
        "to": to,
        "tokens": tokens_out,
        "turns": turns_out,
        "costs": costs_out,
        "pricing": pricing,
        "scope": scope,
        "sessionFileProviders": session_file_providers,
        "projects": project_list,
        "unavailableProviders": unavailable,
    }))
}

// ── buildTranscriptRows ────────────────────────────────────────────────────────

/// `repoDirectory` — a Claude Code worktree counts as its repo.
fn repo_directory(cwd: &str) -> String {
    const MARKER: &str = "/.claude/worktrees/";
    let units = utf16::units(cwd);
    match utf16::index_of(&units, &utf16::units(MARKER), 0) {
        Some(i) if i > 0 => utf16::slice(&units, 0, i),
        _ => cwd.to_string(),
    }
}

/// `path.posix.basename`.
fn basename(p: &str) -> String {
    let s = p.trim_end_matches('/');
    match s.rfind('/') {
        Some(i) => s[i + 1..].to_string(),
        None => s.to_string(),
    }
}

/// `projectForDirectory` — the owning project, else the repo folder as one.
fn project_for_directory(cwd: &Value, roots: &Value) -> Result<Value, String> {
    if !js_truthy(cwd) {
        return Ok(Value::Null);
    }
    let mut best: Option<Value> = None;
    for root in js_iter(roots, "roots")? {
        let root_path = member_err(&root, "path")?;
        let mut inside = strict_eq(cwd, root_path);
        if !inside {
            let rp = member_err(&root, "project")?;
            if !rp.is_null() {
                let prefix = format!("{}/", js_template_string(root_path));
                let Some(units) = string_units(cwd) else {
                    return Err(read_err(cwd, "startsWith")
                        .unwrap_or_else(|| "cwd.startsWith is not a function".to_string()));
                };
                inside = units.starts_with(&prefix.encode_utf16().collect::<Vec<u16>>());
            }
        }
        if inside {
            let length = js_to_number(member(root_path, "length"));
            let beat = match &best {
                None => true,
                Some(b) => length > js_to_number(member(member(b, "path"), "length")),
            };
            if beat {
                best = Some(root.clone());
            }
        }
    }
    if let Some(b) = best {
        return Ok(member_err(&b, "project")?.clone());
    }
    let Some(cwd_str) = as_js_string(cwd) else {
        return Err("cwd.indexOf is not a function".to_string());
    };
    let name = basename(&repo_directory(&cwd_str));
    Ok(if name.is_empty() {
        Value::Null
    } else {
        json!({"projectId": format!("folder:{name}"), "title": name})
    })
}

/// The `projectIdFor` memoized closure inside `buildTranscriptRows`.
fn project_id_for(
    input: &Value,
    project_by_cwd: &mut JsMap,
    projects: &mut JsMap,
    cwd: &Value,
) -> Result<Value, String> {
    let key = svz_key(cwd);
    if !project_by_cwd.has(&key) {
        let project = project_for_directory(cwd, member(input, "projectRoots"))?;
        project_by_cwd.set(key.clone(), project);
    }
    // INFALLIBLE: just inserted when absent.
    let project = project_by_cwd.get(&key).cloned().unwrap_or(Value::Null);
    if js_truthy(&project) {
        let pid = member(&project, "projectId");
        if !projects.has(&svz_key(pid)) {
            projects.set(svz_key(pid), project.clone());
        }
    }
    Ok(if is_nullish(&project) {
        Value::Null
    } else {
        let pid = member(&project, "projectId");
        if is_nullish(pid) {
            Value::Null
        } else {
            pid.clone()
        }
    })
}

/// `turnRow` — the turns row for a key, created with zero counts.
fn turn_row<'a>(
    turns: &'a mut JsMap,
    hour: &str,
    provider: &Value,
    model: &str,
    project_id: &Value,
) -> &'a mut Value {
    let key = join_key(&[
        Value::String(hour.to_string()),
        provider.clone(),
        Value::String(model.to_string()),
        project_id.clone(),
    ]);
    if !turns.has(&key) {
        turns.set(
            key.clone(),
            json!({
                "hour": hour,
                "provider": provider,
                "model": model,
                "projectId": project_id,
                "turns": 0,
                "failedTurns": 0,
            }),
        );
    }
    // INFALLIBLE: just inserted when absent.
    turns.get_mut(&key).unwrap()
}

/// Port of `buildTranscriptRows` (`apps/server/src/usageAnalyticsStats.ts:412`).
pub fn build_transcript_rows(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let window = member_err(&input, "window")?;
    let from_ms = date_parse(member_err(window, "from")?);
    let to_ms = date_parse(member_err(window, "to")?);

    let mut seen = JsSet::default();
    let mut projects = JsMap::default();
    let mut project_by_cwd = JsMap::default();
    let mut tokens = JsMap::default();
    let mut turns = JsMap::default();
    let mut costs = JsMap::default();

    let scan = member_err(&input, "scan")?;
    let files = member_err(scan, "files")?;
    for file in js_iter(files, "input.scan.files")? {
        let mut waiting: Vec<(String, Value)> = Vec::new();
        let mut last_model = Value::String("unknown".to_string());

        // `settle(rawModel)` — pending Claude prompts take the next response's model.
        macro_rules! settle {
            ($raw:expr) => {{
                let raw: &Value = $raw;
                let model = recap_model_name_str(&json!("claudeAgent"), non_empty_string(raw))?;
                for (hour, cwd) in waiting.drain(..) {
                    let pid = project_id_for(&input, &mut project_by_cwd, &mut projects, &cwd)?;
                    let row = turn_row(&mut turns, &hour, &json!("claudeAgent"), &model, &pid);
                    obj_add(row, "turns", &number(1.0));
                }
                model
            }};
        }

        for record in js_iter(&file, "file")? {
            let dedupe = member_err(&record, "dedupeKey")?;
            if !dedupe.is_null() {
                if seen.has(dedupe) {
                    continue;
                }
                seen.add(dedupe.clone());
            }
            let kind = member_err(&record, "kind")?;
            let provider = member_err(&record, "provider")?;
            let model_v = member_err(&record, "model")?;
            if js_str_eq(kind, "usage") && js_truthy(model_v) {
                last_model = model_v.clone();
            }
            if js_str_eq(kind, "usage") && js_str_eq(provider, "claudeAgent") && !waiting.is_empty()
            {
                let raw = if js_truthy(model_v) {
                    model_v.clone()
                } else {
                    last_model.clone()
                };
                settle!(&raw);
            }
            let ts = js_to_number(member_err(&record, "timestampMs")?);
            if ts < from_ms || ts >= to_ms {
                continue;
            }
            let hour = utc_hour(ts)?;
            if js_str_eq(kind, "turn") {
                if js_str_eq(provider, "claudeAgent") {
                    waiting.push((hour, member_err(&record, "cwd")?.clone()));
                } else {
                    let model = recap_model_name_str(provider, non_empty_string(model_v))?;
                    let pid = project_id_for(
                        &input,
                        &mut project_by_cwd,
                        &mut projects,
                        member_err(&record, "cwd")?,
                    )?;
                    let row = turn_row(&mut turns, &hour, provider, &model, &pid);
                    obj_add(row, "turns", &number(1.0));
                }
                continue;
            }
            let model = recap_model_name_str(provider, non_empty_string(model_v))?;
            let pid = project_id_for(
                &input,
                &mut project_by_cwd,
                &mut projects,
                member_err(&record, "cwd")?,
            )?;
            if js_truthy(member_err(&record, "failed")?) {
                let row = turn_row(&mut turns, &hour, provider, &model, &pid);
                obj_add(row, "failedTurns", &number(1.0));
                continue;
            }
            let record_tokens = member_err(&record, "tokens")?;
            let total = transcript_token_total(record_tokens)?;
            if js_to_number(&total) <= 0.0 {
                continue;
            }
            let cost = price_transcript_usage(
                member_err(member_err(&input, "prices")?, "table")?,
                &record,
            )?;
            let token_key = join_key(&[
                Value::String(hour.clone()),
                provider.clone(),
                Value::String(model.clone()),
                pid.clone(),
            ]);
            if !tokens.has(&token_key) {
                tokens.set(
                    token_key.clone(),
                    json!({
                        "hour": hour,
                        "provider": provider,
                        "model": model,
                        "projectId": pid,
                        "origin": Value::Null,
                        "tokens": 0,
                        "cacheReadTokens": 0,
                        "unpricedTokens": 0,
                        "unsplitTokens": 0,
                    }),
                );
            }
            // INFALLIBLE: just inserted when absent.
            let token_row = tokens.get_mut(&token_key).unwrap();
            obj_add(token_row, "tokens", &total);
            obj_add(
                token_row,
                "cacheReadTokens",
                member_err(record_tokens, "cacheReadTokens")?,
            );
            if cost.is_null() {
                obj_add(token_row, "unpricedTokens", &total);
            }
            if !cost.is_null() && js_to_number(&cost) > 0.0 {
                let source = if member_err(&record, "reportedCostUsd")?.is_null() {
                    "estimated"
                } else {
                    "reported"
                };
                let cost_key = format!("{token_key}\u{0}{source}");
                if !costs.has(&cost_key) {
                    costs.set(
                        cost_key.clone(),
                        json!({
                            "hour": hour,
                            "provider": provider,
                            "model": model,
                            "projectId": pid,
                            "source": source,
                            "costUsd": 0,
                            "turns": 0,
                        }),
                    );
                }
                // INFALLIBLE: just inserted when absent.
                let cost_row = costs.get_mut(&cost_key).unwrap();
                obj_add(cost_row, "costUsd", &cost);
            }
        }
        if !waiting.is_empty() {
            settle!(&last_model);
        }
    }

    // An error can land in an hour whose prompt was in the hour before.
    for row in turns.values_mut() {
        let t = js_to_number(member(row, "turns"));
        let f = js_to_number(member(row, "failedTurns"));
        if let Some(slot) = row.as_object_mut().and_then(|m| m.get_mut("turns")) {
            *slot = number(js_max(t, f));
        }
    }
    let mut providers = JsSet::default();
    for row in tokens.values().chain(turns.values()) {
        let p = member(row, "provider");
        if !js_str_eq(p, "unknown") {
            providers.add(p.clone());
        }
    }
    let mut provider_list = providers.values;
    sort_default(&mut provider_list);

    Ok(json!({
        "providers": provider_list,
        "tokens": tokens.values().cloned().collect::<Vec<_>>(),
        "turns": turns.values().cloned().collect::<Vec<_>>(),
        "costs": costs.values().cloned().collect::<Vec<_>>(),
        "projects": projects.values().cloned().collect::<Vec<_>>(),
    }))
}
