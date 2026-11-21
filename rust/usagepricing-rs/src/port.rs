//! The Rust port of `apps/server/src/usagePricing.ts`.
//!
//! Rates tables cross the bridge as `{"$map": [[key, rate], ...]}` marker objects; the
//! port reads and writes that encoding, so a `Map` argument is any value that marker shape
//! produces — everything else fails the way `table.get` on a non-Map does in JS.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::collections::HashMap;

// ── JS value helpers ─────────────────────────────────────────────────────────

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `typeof value === "object" && value !== null`, with bridge markers decoded:
/// `$undefined`, `$num`, `$utf16` and `$bigint` stand for non-object values.
fn is_js_object(v: &Value) -> bool {
    match v {
        Value::Array(_) => true,
        Value::Object(o) => {
            !(o.len() == 1
                && o.keys().next().is_some_and(|k| {
                    matches!(k.as_str(), "$undefined" | "$num" | "$utf16" | "$bigint")
                }))
        }
        _ => false,
    }
}

/// `value.key`, or the JS TypeError when `value` is null or undefined.
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

/// `value.key` where the caller already knows `value` is an object or array:
/// the member is the stored value or `undefined`, never a TypeError.
fn prop(v: &Value, key: &str) -> Value {
    v.as_object()
        .and_then(|m| m.get(key))
        .cloned()
        .unwrap_or_else(crate::js::undefined)
}

/// JS WhiteSpace + LineTerminator, by code unit: everything `String.prototype.trim` removes.
/// Unlike `char::is_whitespace` this excludes U+0085 and includes U+FEFF.
fn js_ws(u: u16) -> bool {
    matches!(
        u,
        0x09..=0x0d | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f
            | 0x3000 | 0xfeff
    )
}

fn js_trim_units(u: &[u16]) -> &[u16] {
    let mut a = 0;
    let mut b = u.len();
    while a < b && js_ws(u[a]) {
        a += 1;
    }
    while b > a && js_ws(u[b - 1]) {
        b -= 1;
    }
    &u[a..b]
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c as u32,
            0x09..=0x0d | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f
                | 0x205f | 0x3000 | 0xfeff
        )
    })
}

/// `String.prototype.toLowerCase` on UTF-16 code units. `to_lowercase` is the Unicode
/// default case conversion both sides use, final sigma included; U+FFFD stands in for
/// lone surrogates, which are not cased and do not change the context either.
fn lower_units(u: &[u16]) -> Vec<u16> {
    String::from_utf16_lossy(u)
        .to_lowercase()
        .encode_utf16()
        .collect()
}

fn bare_name_str(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

fn bare_name(key: &[u16]) -> &[u16] {
    match key.iter().rposition(|&u| u == u16::from(b'/')) {
        Some(i) => &key[i + 1..],
        None => key,
    }
}

fn units_are(u: &[u16], s: &str) -> bool {
    u.iter().copied().eq(s.encode_utf16())
}

const UNPRICEABLE: [&str; 6] = [
    "<synthetic>",
    "synthetic",
    "opus",
    "sonnet",
    "haiku",
    "fable",
];

/// `Number(string)`: trims, then hex/binary/octal/infinity/decimal.
fn parse_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    let lower = t.strip_prefix('+').unwrap_or(t);
    let lower = lower
        .strip_prefix('-')
        .map(|r| (r, true))
        .unwrap_or((lower, false));
    let (digits, neg) = lower;
    let parsed = if let Some(h) = digits.strip_prefix("0x").or(digits.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok().map(|v| v as f64)
    } else if let Some(b) = digits.strip_prefix("0b").or(digits.strip_prefix("0B")) {
        u64::from_str_radix(b, 2).ok().map(|v| v as f64)
    } else if let Some(o) = digits.strip_prefix("0o").or(digits.strip_prefix("0O")) {
        u64::from_str_radix(o, 8).ok().map(|v| v as f64)
    } else {
        None
    };
    if let Some(v) = parsed {
        return if neg { -v } else { v };
    }
    t.parse().unwrap_or(f64::NAN)
}

/// `String(value)` for the shapes that reach arithmetic through an array join;
/// every name that cannot parse is harmless — it only feeds `Number(...)`.
fn join_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(join_text).collect::<Vec<_>>().join(","),
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$undefined") {
                    return String::new();
                }
                if let Some(x) = o.get("$num").and_then(Value::as_str) {
                    return match x {
                        "-0" => "0".to_string(),
                        other => other.to_string(),
                    };
                }
                if let Some(u) = o.get("$utf16").and_then(Value::as_array) {
                    let units = utf16_marker(u);
                    return String::from_utf16_lossy(&units);
                }
                if let Some(b) = o.get("$bigint").and_then(Value::as_str) {
                    return b.to_string();
                }
            }
            "[object Object]".to_string()
        }
    }
}

fn utf16_marker(units: &[Value]) -> Vec<u16> {
    units
        .iter()
        .filter_map(|u| u.as_u64().and_then(|u| u16::try_from(u).ok()))
        .collect()
}

/// JS `ToNumber` on a bridge value.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        Value::Array(_) => parse_number(&join_text(v)),
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$undefined") {
                    return f64::NAN;
                }
                if let Some(x) = o.get("$num").and_then(Value::as_str) {
                    return match x {
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        "-0" => -0.0,
                        _ => f64::NAN,
                    };
                }
                if let Some(u) = o.get("$utf16").and_then(Value::as_array) {
                    return parse_number(&String::from_utf16_lossy(&utf16_marker(u)));
                }
                if let Some(b) = o.get("$bigint").and_then(Value::as_str) {
                    return parse_number(b);
                }
                if let Some(a) = o
                    .get("$bytes")
                    .or_else(|| o.get("$buffer"))
                    .and_then(Value::as_array)
                {
                    return parse_number(
                        &a.iter()
                            .map(|x| crate::js::number_to_string(x.as_f64().unwrap_or(f64::NAN)))
                            .collect::<Vec<_>>()
                            .join(","),
                    );
                }
            }
            f64::NAN
        }
    }
}

/// JS `ToBoolean` on a bridge value.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$undefined") {
                    return false;
                }
                if let Some(x) = o.get("$num").and_then(Value::as_str) {
                    return !matches!(x, "NaN" | "-0");
                }
                if let Some(u) = o.get("$utf16").and_then(Value::as_array) {
                    return !u.is_empty();
                }
                if let Some(b) = o.get("$bigint").and_then(Value::as_str) {
                    return !b.is_empty() && b != "0" && b != "-0";
                }
            }
            true
        }
    }
}

/// A JS primitive after `ToPrimitive`, for `+` and `>` — only number-or-string matters.
enum Prim {
    Num(f64),
    Str(String),
}

fn to_prim(v: &Value) -> Prim {
    match v {
        Value::String(s) => Prim::Str(s.clone()),
        Value::Array(_) => Prim::Str(join_text(v)),
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$undefined") {
                    return Prim::Num(f64::NAN);
                }
                if let Some(x) = o.get("$num").and_then(Value::as_str) {
                    return Prim::Num(match x {
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        "-0" => -0.0,
                        _ => f64::NAN,
                    });
                }
                if let Some(u) = o.get("$utf16").and_then(Value::as_array) {
                    return Prim::Str(String::from_utf16_lossy(&utf16_marker(u)));
                }
                if let Some(b) = o.get("$bigint").and_then(Value::as_str) {
                    return Prim::Str(b.to_string());
                }
            }
            Prim::Str("[object Object]".to_string())
        }
        _ => Prim::Num(to_number(v)),
    }
}

fn prim_num(p: &Prim) -> f64 {
    match p {
        Prim::Num(n) => *n,
        Prim::Str(s) => parse_number(s),
    }
}

fn prim_str(p: &Prim) -> String {
    match p {
        Prim::Num(n) => crate::js::number_to_string(*n),
        Prim::Str(s) => s.clone(),
    }
}

/// JS `a + b` on primitives: numeric unless either side is a string.
fn js_add(a: Prim, b: Prim) -> Prim {
    match (&a, &b) {
        (Prim::Num(x), Prim::Num(y)) => Prim::Num(x + y),
        _ => Prim::Str(format!("{}{}", prim_str(&a), prim_str(&b))),
    }
}

/// JS `a > b` on primitives: lexicographic on code units when both are strings.
fn js_gt(a: &Prim, b: &Prim) -> bool {
    match (a, b) {
        (Prim::Str(x), Prim::Str(y)) => x.encode_utf16().gt(y.encode_utf16()),
        _ => prim_num(a) > prim_num(b),
    }
}

// ── the rate table ───────────────────────────────────────────────────────────

/// `finite(v)`: a number that is finite and non-negative; `-0` passes, as in JS.
fn finite(v: &Value) -> Option<f64> {
    let n = crate::js::as_number(v)?;
    (n.is_finite() && n >= 0.0).then_some(n)
}

#[derive(Clone, PartialEq)]
struct Rates {
    input: f64,
    output: f64,
    cache_read: f64,
    cache_write: f64,
    cache_write1h: f64,
}

#[derive(Clone, PartialEq)]
struct ModelRate {
    rates: Rates,
    tiers: Vec<(f64, Rates)>,
    fast: f64,
}

/// `^(.+)_above_(\d+)k_tokens$` on a property name: the last `_above_` wins, the prefix
/// must hold at least one non-line-terminator code unit, the digits are ASCII.
fn tier_key(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_suffix("k_tokens")?;
    let i = rest.rfind("_above_")?;
    let digits = &rest[i + "_above_".len()..];
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let prefix = &rest[..i];
    if prefix.is_empty() || prefix.contains(['\n', '\r', '\u{2028}', '\u{2029}']) {
        return None;
    }
    Some((prefix, digits))
}

/// `readRates`: LiteLLM field names, falling back to `base` where a tier omits one.
fn read_rates(
    key: &str,
    field: &dyn Fn(&str) -> Option<f64>,
    base: Option<&Rates>,
) -> Option<Rates> {
    let input = field("input_cost_per_token").or(base.map(|b| b.input))?;
    let output = field("output_cost_per_token").or(base.map(|b| b.output))?;
    let cache_write = field("cache_creation_input_token_cost")
        .or(base.map(|b| b.cache_write))
        .unwrap_or(input);
    let cache_read = field("cache_read_input_token_cost")
        .or(base.map(|b| b.cache_read))
        .unwrap_or(input);
    let cache_write1h = field("cache_creation_input_token_cost_above_1hr")
        .or(base.map(|b| b.cache_write1h))
        .unwrap_or_else(|| {
            if bare_name_str(key).starts_with("claude") {
                input * 2.0
            } else {
                cache_write
            }
        });
    Some(Rates {
        input,
        output,
        cache_read,
        cache_write,
        cache_write1h,
    })
}

/// `parseEntry`: base rates plus `*_above_Nk_tokens` tier fields and the fast multiplier.
fn parse_entry(key: &str, raw: &Value) -> Option<ModelRate> {
    let entry = raw.as_object()?;
    let base = read_rates(key, &|n| entry.get(n).and_then(finite), None)?;
    let mut seen: HashMap<u64, usize> = HashMap::new();
    let mut tiers: Vec<(f64, Vec<(String, f64)>)> = Vec::new();
    for (name, value) in entry {
        let Some((field, digits)) = tier_key(name) else {
            continue;
        };
        let Some(rate) = finite(value) else {
            continue;
        };
        let threshold = digits.parse::<f64>().unwrap_or(f64::INFINITY) * 1000.0;
        let bits = if threshold == 0.0 { 0.0 } else { threshold }.to_bits();
        match seen.get(&bits) {
            Some(&i) => tiers[i].1.push((field.to_string(), rate)),
            None => {
                seen.insert(bits, tiers.len());
                tiers.push((threshold, vec![(field.to_string(), rate)]));
            }
        }
    }
    // `toSorted(([l], [r]) => l - r)`: ascending, stable.
    tiers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let tiers: Vec<(f64, Rates)> = tiers
        .iter()
        .filter_map(|(above, fields)| {
            read_rates(
                key,
                &|n| fields.iter().find(|(k, _)| k == n).map(|(_, v)| *v),
                Some(&base),
            )
            .map(|r| (*above, r))
        })
        .collect();
    let fast = entry
        .get("provider_specific_entry")
        .filter(|v| is_js_object(v))
        .map(|v| prop(v, "fast"))
        .and_then(|v| finite(&v));
    let fast = match fast {
        Some(f) if f > 0.0 => f,
        _ => 1.0,
    };
    Some(ModelRate {
        rates: base,
        tiers,
        fast,
    })
}

fn rates_json(r: &Rates) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("input".into(), crate::js::number(r.input));
    m.insert("output".into(), crate::js::number(r.output));
    m.insert("cacheRead".into(), crate::js::number(r.cache_read));
    m.insert("cacheWrite".into(), crate::js::number(r.cache_write));
    m.insert("cacheWrite1h".into(), crate::js::number(r.cache_write1h));
    m
}

fn rate_json(r: &ModelRate) -> Value {
    let mut m = rates_json(&r.rates);
    m.insert(
        "tiers".into(),
        Value::Array(
            r.tiers
                .iter()
                .map(|(above, rates)| {
                    let mut t = Map::new();
                    t.insert("aboveTokens".into(), crate::js::number(*above));
                    t.insert("rates".into(), Value::Object(rates_json(rates)));
                    Value::Object(t)
                })
                .collect(),
        ),
    );
    m.insert("fastMultiplier".into(), crate::js::number(r.fast));
    Value::Object(m)
}

/// Port of `parseRateTable` (`apps/server/src/usagePricing.ts:126`).
pub fn parse_rate_table(document: Value) -> Result<Value, String> {
    let mut table: Vec<(String, ModelRate)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    {
        let mut entry = |name: &str, raw: &Value| {
            if !is_js_object(raw) {
                return;
            }
            let key = js_trim(name).to_lowercase();
            if key.is_empty() {
                return;
            }
            if let Some(rate) = parse_entry(&key, raw) {
                match index.get(&key) {
                    Some(&i) => table[i].1 = rate,
                    None => {
                        index.insert(key.clone(), table.len());
                        table.push((key, rate));
                    }
                }
            }
        };
        match &document {
            Value::Object(doc) if is_js_object(&document) => {
                for (name, raw) in doc {
                    entry(name, raw);
                }
            }
            // `Object.entries` on an array lists its indexes as keys.
            Value::Array(items) => {
                for (i, raw) in items.iter().enumerate() {
                    entry(&i.to_string(), raw);
                }
            }
            _ => {}
        }
    }
    // Bare-name aliases, only when unclaimed and every qualified entry agrees on the rate.
    let mut alias_pos: HashMap<String, usize> = HashMap::new();
    let mut aliases: Vec<(String, Option<ModelRate>)> = Vec::new();
    for (key, rate) in &table {
        let alias = bare_name_str(key);
        if alias.is_empty() || alias == key || index.contains_key(alias) {
            continue;
        }
        match alias_pos.get(alias) {
            None => {
                alias_pos.insert(alias.to_string(), aliases.len());
                aliases.push((alias.to_string(), Some(rate.clone())));
            }
            Some(&i) => {
                if let Some(held) = &aliases[i].1
                    && held != rate
                {
                    aliases[i].1 = None;
                }
            }
        }
    }
    for (alias, rate) in aliases {
        if let Some(rate) = rate {
            index.insert(alias.clone(), table.len());
            table.push((alias, rate));
        }
    }
    let entries: Vec<Value> = table
        .iter()
        .map(|(k, r)| json!([k, rate_json(r)]))
        .collect();
    Ok(json!({ "$map": entries }))
}

// ── lookups ──────────────────────────────────────────────────────────────────

fn map_get<'a>(table: &'a Value, key: &[u16]) -> Result<Option<&'a Value>, String> {
    match table {
        Value::Null => Err("Cannot read properties of null (reading 'get')".into()),
        v if is_undefined(v) => Err("Cannot read properties of undefined (reading 'get')".into()),
        Value::Object(o) if o.len() == 1 && o.contains_key("$map") => {
            let Some(entries) = o["$map"].as_array() else {
                return Ok(None);
            };
            for pair in entries {
                if let Some(a) = pair.as_array()
                    && a.len() == 2
                    && crate::js::string_units(&a[0]).is_some_and(|u| u == key)
                {
                    return Ok(Some(&a[1]));
                }
            }
            Ok(None)
        }
        _ => Err("table.get is not a function".into()),
    }
}

/// `key.replace(/-\d{8}$/u, "")`: a trailing `-` plus eight ASCII digits marks a dated
/// snapshot, which falls back to the undated name.
fn undated(key: &[u16]) -> &[u16] {
    let n = key.len();
    if n >= 9
        && key[n - 9] == u16::from(b'-')
        && key[n - 8..].iter().all(|u| matches!(*u, 0x30..=0x39))
    {
        &key[..n - 9]
    } else {
        key
    }
}

fn lookup_rate_units(table: &Value, model: &[u16]) -> Result<Option<Value>, String> {
    let mut key = lower_units(js_trim_units(model));
    if let Some(i) = key.iter().position(|&u| u == u16::from(b'[')) {
        key.truncate(i);
    }
    let bare = bare_name(&key);
    if key.is_empty() || UNPRICEABLE.iter().any(|u| units_are(bare, u)) {
        return Ok(None);
    }
    // `table.get(key) ?? table.get(undated) ?? null`: a nullish stored value falls through.
    let mut found = map_get(table, &key)?;
    if !matches!(found, Some(v) if !v.is_null() && !is_undefined(v)) {
        found = map_get(table, undated(&key))?;
    }
    Ok(match found {
        Some(v) if !v.is_null() && !is_undefined(v) => Some(v.clone()),
        _ => None,
    })
}

/// Port of `lookupRate` (`apps/server/src/usagePricing.ts:151`).
pub fn lookup_rate(table: Value, model: String) -> Result<Option<Value>, String> {
    let units: Vec<u16> = model.encode_utf16().collect();
    lookup_rate_units(&table, &units)
}

/// `usage.model` for the internal call: `model.trim()` fails on non-strings the way JS does.
fn model_units(v: &Value) -> Result<Vec<u16>, String> {
    match crate::js::string_units(v) {
        Some(u) => Ok(u),
        None => match v {
            Value::Null => Err("Cannot read properties of null (reading 'trim')".into()),
            _ if is_undefined(v) => {
                Err("Cannot read properties of undefined (reading 'trim')".into())
            }
            _ => Err("model.trim is not a function".into()),
        },
    }
}

/// Port of `priceTokens` (`apps/server/src/usagePricing.ts:162`).
pub fn price_tokens(table: Value, model: String, tokens: Value) -> Result<Option<f64>, String> {
    let units: Vec<u16> = model.encode_utf16().collect();
    let Some(rate) = lookup_rate_units(&table, &units)? else {
        return Ok(None);
    };
    if !truthy(&rate) {
        return Ok(None);
    }
    let cost = to_number(&member(&tokens, "inputTokens")?) * to_number(&member(&rate, "input")?)
        + to_number(&member(&tokens, "cacheReadTokens")?) * to_number(&member(&rate, "cacheRead")?)
        + to_number(&member(&tokens, "cacheWriteTokens")?)
            * to_number(&member(&rate, "cacheWrite")?)
        + to_number(&member(&tokens, "outputTokens")?) * to_number(&member(&rate, "output")?);
    Ok(Some(cost))
}

/// `transcriptPromptTokens`: `(input + cacheRead + cacheWrite5m + cacheWrite1h + output)`
/// minus `output` — the `+` chain follows JS addition, which concatenates on a string.
fn prompt_tokens(tokens: &Value) -> Result<Prim, String> {
    let mut total = to_prim(&member(tokens, "inputTokens")?);
    for key in [
        "cacheReadTokens",
        "cacheWrite5mTokens",
        "cacheWrite1hTokens",
        "outputTokens",
    ] {
        total = js_add(total, to_prim(&member(tokens, key)?));
    }
    let output = to_prim(&member(tokens, "outputTokens")?);
    Ok(Prim::Num(prim_num(&total) - prim_num(&output)))
}

/// `for (const tier of rate.tiers)`: the value must be iterable to read it at all.
fn is_iterable(v: &Value) -> bool {
    match v {
        Value::String(_) | Value::Array(_) => true,
        Value::Object(o) => {
            o.len() == 1
                && o.keys()
                    .next()
                    .is_some_and(|k| matches!(k.as_str(), "$map" | "$set" | "$bytes" | "$buffer"))
        }
        _ => false,
    }
}

/// The values `for..of` yields: code points for strings, entries for a Map marker.
fn iterable_items(v: &Value) -> Vec<Value> {
    match v {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Value::Object(o) => {
            for key in ["$set", "$bytes", "$buffer", "$map"] {
                if let Some(a) = o.get(key).and_then(Value::as_array) {
                    return a.clone();
                }
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

/// Port of `priceTranscriptUsage` (`apps/server/src/usagePricing.ts:178`).
///
/// Returns the JSON encoding of the result: `{"$undefined": true}` when the usage record
/// carries no `reportedCostUsd` (the original returns `undefined`, not `null`).
pub fn price_transcript_usage(table: Value, usage: Value) -> Result<Value, String> {
    let reported = member(&usage, "reportedCostUsd")?;
    if !reported.is_null() {
        return Ok(reported);
    }
    let model = member(&usage, "model")?;
    let units = model_units(&model)?;
    let Some(rate) = lookup_rate_units(&table, &units)? else {
        return Ok(Value::Null);
    };
    if !truthy(&rate) {
        return Ok(Value::Null);
    }
    let tokens = member(&usage, "tokens")?;
    let prompt = prompt_tokens(&tokens)?;
    let mut rates = rate.clone();
    let tiers = member(&rate, "tiers")?;
    if is_iterable(&tiers) {
        for tier in iterable_items(&tiers) {
            if js_gt(&prompt, &to_prim(&member(&tier, "aboveTokens")?)) {
                rates = member(&tier, "rates")?;
            }
        }
    } else {
        return Err("rate.tiers is not iterable".into());
    }
    let cost = to_number(&member(&tokens, "inputTokens")?) * to_number(&member(&rates, "input")?)
        + to_number(&member(&tokens, "cacheReadTokens")?)
            * to_number(&member(&rates, "cacheRead")?)
        + to_number(&member(&tokens, "cacheWrite5mTokens")?)
            * to_number(&member(&rates, "cacheWrite")?)
        + to_number(&member(&tokens, "cacheWrite1hTokens")?)
            * to_number(&member(&rates, "cacheWrite1h")?)
        + to_number(&member(&tokens, "outputTokens")?) * to_number(&member(&rates, "output")?);
    let multiplier = if truthy(&member(&usage, "fast")?) {
        to_number(&member(&rate, "fastMultiplier")?)
    } else {
        1.0
    };
    Ok(crate::js::number(cost * multiplier))
}
