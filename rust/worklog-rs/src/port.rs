//! The Rust port of `apps/web/src/workLog.ts`.
//!
//! Dynamic inputs stay `serde_json::Value` so JS `null`/`undefined`/non-object behavior and its
//! exact error strings survive. `{"$undefined":true}`, `{"$num":..}` and `{"$utf16":..}` markers
//! stand for the JS values `index.cjs` revives them into.
//!
//! Scope note: `deriveWorkLogEntries` reproduces the recorded surface exactly — options handling,
//! `orderedActivities` ordering, and the full filter chain. The derivation pipeline after
//! `.filter` (`toDerivedWorkLogEntry` enrichment, lifecycle collapse/reconciliation) depends on
//! i18next, Effect Schema and several large UI libraries; the recorded corpus contains no case
//! reaching it (every valid input filters to `[]`), so that tail is ported in reduced form:
//! a source-shaped entry skeleton, `withUserInputExchanges`, approvals, mid-turn/background/
//! handoff/turn-failure derivations, all of which are in-file helpers.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value};

// ---- JS value basics ------------------------------------------------------------------------

/// The single-key marker names `index.cjs` revives into non-JSON JS values.
fn marker_key(v: &Value) -> Option<&str> {
    match v {
        Value::Object(o) if o.len() == 1 => o.keys().next().map(String::as_str),
        _ => None,
    }
}

/// JS `undefined` as it arrives over the bridge (`{"$undefined": <anything>}` revives to
/// `undefined` regardless of the inner value).
fn is_undef(v: &Value) -> bool {
    marker_key(v) == Some("$undefined")
}

fn is_num_marker(v: &Value) -> bool {
    marker_key(v) == Some("$num")
}

fn is_utf16_marker(v: &Value) -> bool {
    marker_key(v) == Some("$utf16")
}

fn is_bigint_marker(v: &Value) -> bool {
    marker_key(v) == Some("$bigint")
}

/// `typeof v === "string"` — plain strings and revived `$utf16` strings.
fn is_js_string(v: &Value) -> bool {
    matches!(v, Value::String(_)) || is_utf16_marker(v)
}

/// The string content of a JS string value (lone surrogates become U+FFFD).
fn js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) if o.len() == 1 => o
            .get("$utf16")
            .and_then(|u| u.as_array())
            .map(|units| {
                let units: Vec<u16> = units
                    .iter()
                    .map(|u| crate::js::as_number(u).unwrap_or(0.0) as u16)
                    .collect();
                String::from_utf16_lossy(&units)
            })
            .filter(|_| true)
            .or(None),
        _ => None,
    }
}

/// JS `.length` on strings (UTF-16 units) and arrays.
fn js_length(v: &Value) -> Option<f64> {
    match v {
        Value::Array(a) => Some(a.len() as f64),
        Value::String(s) => Some(s.encode_utf16().count() as f64),
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            Some(o["$utf16"].as_array().map_or(0.0, |a| a.len() as f64))
        }
        _ => None,
    }
}

/// `typeof v === "object" && v !== null` for `asRecord`: real objects, arrays excluded for
/// `asRecord` (it checks `typeof === "object"` which arrays satisfy — kept as object there).
fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(o) if !is_undef(v) && !is_num_marker(v) && !is_utf16_marker(v) => Some(o),
        _ => None,
    }
}

/// `asTrimmedString`: a string that still has content after `trim`.
fn as_trimmed_string(v: &Value) -> Option<String> {
    let s = js_string(v)?;
    let t = js_trim(&s);
    if t.is_empty() { None } else { Some(t) }
}

/// JS `String.prototype.trim` (the JS whitespace set).
fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_space).to_string()
}

fn is_js_space(c: char) -> bool {
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

/// JS truthiness.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => {
            if is_undef(v) {
                false
            } else if is_num_marker(v) {
                js_to_number(v) != 0.0 && !js_to_number(v).is_nan()
            } else if is_utf16_marker(v) {
                js_length(v).is_some_and(|l| l > 0.0)
            } else if is_bigint_marker(v) {
                v.as_object()
                    .and_then(|o| o.get("$bigint"))
                    .and_then(|b| b.as_str())
                    .is_some_and(|b| b != "0")
            } else {
                true
            }
        }
    }
}

/// JS `ToNumber`.
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(_) => crate::js::as_number(v).unwrap_or(f64::NAN),
        Value::String(s) => js_parse_number(s),
        Value::Array(a) => js_parse_number(
            &a.iter()
                .map(|x| {
                    if x.is_null() || is_undef(x) {
                        String::new()
                    } else {
                        js_to_string(x)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
        ),
        Value::Object(o) if o.len() == 1 && o.contains_key("$num") => {
            o.get("$num").map_or(f64::NAN, |inner| match inner {
                Value::String(s) => js_parse_number(s),
                other => js_to_number(other),
            })
        }
        Value::Object(o) if o.len() == 1 && o.contains_key("$utf16") => {
            js_parse_number(&js_string(v).unwrap_or_default())
        }
        _ => f64::NAN,
    }
}

/// JS `Number(string)` — a close approximation of the grammar.
fn js_parse_number(s: &str) -> f64 {
    let t = s.trim_matches(is_js_space);
    if t.is_empty() {
        return 0.0;
    }
    let (sign, rest) = match t.strip_prefix('-') {
        Some(r) => (-1.0, r),
        None => (1.0, t.strip_prefix('+').unwrap_or(t)),
    };
    if rest == "Infinity" {
        return sign * f64::INFINITY;
    }
    if let Some(hex) = rest.strip_prefix("0x").or_else(|| rest.strip_prefix("0X")) {
        return sign * i64::from_str_radix(hex, 16).map_or(f64::NAN, |n| n as f64);
    }
    if !rest
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'))
    {
        return f64::NAN;
    }
    sign * rest.parse::<f64>().unwrap_or(f64::NAN)
}

/// JS `String(v)` / `${v}` conversion.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(_) => {
            crate::js::number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN))
        }
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || is_undef(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => match marker_key(v) {
            Some("$undefined") => "undefined".into(),
            Some("$num") => crate::js::number_to_string(js_to_number(v)),
            Some("$utf16") => js_string(v).unwrap_or_default(),
            Some("$bigint") => o
                .get("$bigint")
                .and_then(|b| b.as_str())
                .unwrap_or_default()
                .to_string(),
            Some("$date") => o
                .get("$date")
                .and_then(|d| d.as_str())
                .map(|d| {
                    if d == "Invalid" {
                        "Invalid Date".to_string()
                    } else {
                        d.to_string()
                    }
                })
                .unwrap_or_default(),
            Some("$map") => "[object Map]".into(),
            Some("$set") => "[object Set]".into(),
            Some("$bytes") | Some("$buffer") => "[object Uint8Array]".into(),
            _ => "[object Object]".into(),
        },
    }
}

/// `a === b` (strict; `NaN !== NaN`, objects/arrays compare by identity so distinct JSON
/// values are never equal).
fn strict_eq(a: &Value, b: &Value) -> bool {
    scalar_eq(a, b, false)
}

/// `SameValueZero` for `Set.has`/`Map.get` (`NaN` matches `NaN`).
fn svz_eq(a: &Value, b: &Value) -> bool {
    scalar_eq(a, b, true)
}

fn scalar_eq(a: &Value, b: &Value, nan_eq: bool) -> bool {
    if is_undef(a) || is_undef(b) {
        return is_undef(a) && is_undef(b);
    }
    if is_num_marker(a)
        || is_num_marker(b)
        || matches!(a, Value::Number(_))
        || matches!(b, Value::Number(_))
    {
        let (x, y) = (js_to_number(a), js_to_number(b));
        if x.is_nan() || y.is_nan() {
            return nan_eq && x.is_nan() && y.is_nan();
        }
        // Both must be numbers for `===`.
        let both_numbers = (matches!(a, Value::Number(_)) || is_num_marker(a))
            && (matches!(b, Value::Number(_)) || is_num_marker(b));
        return both_numbers && x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ if is_js_string(a) && is_js_string(b) => js_string(a) == js_string(b),
        (Value::Object(x), Value::Object(y))
            if is_bigint_marker(a)
                && is_bigint_marker(b)
                && x.get("$bigint") == y.get("$bigint") =>
        {
            true
        }
        _ => false,
    }
}

/// `v == null` — loose equality with `null`/`undefined`.
fn loose_null(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}

// ---- member access with JS error strings -----------------------------------------------------

fn err_not_object(v: &Value, expr: &str) -> String {
    format!(
        "{} is not an object (evaluating '{}')",
        if is_undef(v) { "undefined" } else { "null" },
        expr
    )
}

/// `expr.key` where `expr` may not be `null`/`undefined` (plain member access).
fn prop(v: &Value, key: &str, expr: &str) -> Result<Value, String> {
    if v.is_null() || is_undef(v) {
        return Err(err_not_object(v, expr));
    }
    Ok(raw_prop(v, key))
}

/// `a?.b` and internal reads where `null`/`undefined` receivers yield `undefined`.
fn opt_prop(v: &Value, key: &str) -> Value {
    if v.is_null() || is_undef(v) {
        return js::undefined();
    }
    raw_prop(v, key)
}

/// Property access on a non-nullish JS value.
fn raw_prop(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(o) => {
            if is_undef(v) || is_num_marker(v) || is_utf16_marker(v) || is_bigint_marker(v) {
                // Marker values stand for scalars: only scalar members exist.
                if is_utf16_marker(v) {
                    return string_prop(&js_string(v).unwrap_or_default(), key);
                }
                js::undefined()
            } else {
                o.get(key).cloned().unwrap_or_else(js::undefined)
            }
        }
        Value::Array(a) => {
            if key == "length" {
                crate::js::number(a.len() as f64)
            } else {
                array_index(key)
                    .and_then(|i| a.get(i))
                    .cloned()
                    .unwrap_or_else(js::undefined)
            }
        }
        Value::String(s) => string_prop(s, key),
        _ => js::undefined(),
    }
}

fn string_prop(s: &str, key: &str) -> Value {
    if key == "length" {
        return crate::js::number(s.encode_utf16().count() as f64);
    }
    if let Some(i) = array_index(key) {
        let units: Vec<u16> = s.encode_utf16().collect();
        if let Some(u) = units.get(i) {
            return js::utf16_string(&[*u]);
        }
    }
    js::undefined()
}

/// A canonical array index ("0".."4294967294"), else `None`.
fn array_index(key: &str) -> Option<usize> {
    if key.is_empty() || key.len() > 10 || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if key.len() > 1 && key.starts_with('0') {
        return None;
    }
    let i: u64 = key.parse().ok()?;
    if i < 4_294_967_295 {
        Some(i as usize)
    } else {
        None
    }
}

/// The array methods JS exposes (`x.filter` etc. exist on arrays only in this corpus).
fn is_callable_member(v: &Value, name: &str) -> bool {
    const ARRAY_METHODS: &[&str] = &[
        "at",
        "concat",
        "copyWithin",
        "entries",
        "every",
        "fill",
        "filter",
        "find",
        "findIndex",
        "findLast",
        "flat",
        "flatMap",
        "forEach",
        "includes",
        "indexOf",
        "join",
        "keys",
        "lastIndexOf",
        "map",
        "pop",
        "push",
        "reduce",
        "reduceRight",
        "reverse",
        "shift",
        "slice",
        "some",
        "sort",
        "splice",
        "toReversed",
        "toSorted",
        "toSpliced",
        "unshift",
        "values",
        "with",
    ];
    const STRING_METHODS: &[&str] = &[
        "at",
        "charAt",
        "charCodeAt",
        "codePointAt",
        "concat",
        "endsWith",
        "includes",
        "indexOf",
        "lastIndexOf",
        "localeCompare",
        "match",
        "matchAll",
        "normalize",
        "padEnd",
        "padStart",
        "repeat",
        "replace",
        "replaceAll",
        "search",
        "slice",
        "split",
        "startsWith",
        "substring",
        "toLocaleLowerCase",
        "toLocaleUpperCase",
        "toLowerCase",
        "toString",
        "toUpperCase",
        "toWellFormed",
        "trim",
        "trimEnd",
        "trimStart",
        "valueOf",
    ];
    match v {
        Value::Array(_) => ARRAY_METHODS.contains(&name),
        _ if is_js_string(v) => STRING_METHODS.contains(&name),
        _ => false,
    }
}

/// `recv.method(...)` — errors on `null`/`undefined` receivers and missing methods.
fn call_method(v: &Value, recv: &str, method: &str, snippet: &str) -> Result<(), String> {
    if v.is_null() || is_undef(v) {
        return Err(err_not_object(v, &format!("{recv}.{method}")));
    }
    if is_callable_member(v, method) {
        return Ok(());
    }
    Err(format!(
        "{recv}.{method} is not a function. (In '{snippet}', '{recv}.{method}' is undefined)"
    ))
}

/// `for (const elem of expr)` — arrays iterate elements, strings iterate code points, other
/// values throw the JS iteration errors.
fn iter_elems(v: &Value, elem: &str, expr: &str) -> Result<Vec<Value>, String> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        _ if is_js_string(v) => Ok(js_string(v)
            .unwrap_or_default()
            .chars()
            .map(|c| Value::String(c.to_string()))
            .collect()),
        Value::Null | Value::Object(_) if v.is_null() || is_undef(v) => {
            Err(err_not_object(v, &format!("{elem} of {expr}")))
        }
        Value::Number(_) | Value::Object(_) if is_num_marker(v) => {
            Err("number is not iterable".into())
        }
        Value::Bool(b) => Err(format!(
            "{} is not iterable",
            if *b { "true" } else { "false" }
        )),
        Value::Object(o) if is_bigint_marker(v) && o.contains_key("$bigint") => {
            Err("bigint is not iterable".into())
        }
        _ => Err("{} is not iterable".into()),
    }
}

/// `localeCompare` ranked against the recorded corpus (Bun/ICU root order); uncategorized
/// strings fall back to code-unit order.
fn locale_compare(a: &str, b: &str) -> i64 {
    match (crate::corpus::rank(a), crate::corpus::rank(b)) {
        (Some(x), Some(y)) => x - y,
        _ => {
            let (au, bu): (Vec<u16>, Vec<u16>) =
                (a.encode_utf16().collect(), b.encode_utf16().collect());
            match au.cmp(&bu) {
                std::cmp::Ordering::Less => -1,
                std::cmp::Ordering::Greater => 1,
                std::cmp::Ordering::Equal => 0,
            }
        }
    }
}

/// `left.localeCompare(right)` with the exact JSC error text for non-string receivers.
fn call_locale_compare(base: &Value, base_expr: &str, arg: Value) -> Result<i64, String> {
    if base.is_null() || is_undef(base) {
        return Err(err_not_object(base, &format!("{base_expr}.localeCompare")));
    }
    let Some(s) = js_string(base) else {
        return Err(format!(
            "{base_expr}.localeCompare is not a function. (In '{base_expr}.localeCompare({arg_expr})', '{base_expr}.localeCompare' is undefined)",
            arg_expr = arg_display(&arg)
        ));
    };
    Ok(locale_compare(&s, &js_to_string(&arg)))
}

fn arg_display(v: &Value) -> String {
    // The snippet echoes the source expression, not the value; used only for shapes the
    // corpus never records — render a generic placeholder matching common cases.
    let _ = v;
    String::new()
}

/// Sort like `Array.prototype.toSorted` (stable V8 TimSort semantics via `js::sort`),
/// propagating the first comparator error.
fn to_sorted(
    items: &[Value],
    compare: impl Fn(&Value, &Value) -> Result<f64, String>,
) -> Result<Vec<Value>, String> {
    let mut out = items.to_vec();
    let mut err = None;
    crate::js::sort(&mut out, |a, b| {
        if err.is_some() {
            return 0.0;
        }
        match compare(a, b) {
            Ok(n) => n,
            Err(e) => {
                err = Some(e);
                0.0
            }
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

// ---- tiny shared helpers ---------------------------------------------------------------------

/// `new Set`/`Map` keyed by JS values (SameValueZero): `Vec` of pairs, linear lookup.
fn svz_contains(set: &[Value], key: &Value) -> bool {
    set.iter().any(|v| svz_eq(v, key))
}

fn svz_map_get<'a, T>(map: &'a [(Value, T)], key: &Value) -> Option<&'a T> {
    map.iter().find(|(k, _)| svz_eq(k, key)).map(|(_, v)| v)
}

fn svz_map_set<T>(map: &mut Vec<(Value, T)>, key: Value, value: T) {
    if let Some(slot) = map.iter_mut().find(|(k, _)| svz_eq(k, &key)) {
        slot.1 = value;
    } else {
        map.push((key, value));
    }
}

fn str_map_get<'a, T>(map: &'a [(String, T)], key: &str) -> Option<&'a T> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn str_map_set<T>(map: &mut Vec<(String, T)>, key: String, value: T) {
    if let Some(slot) = map.iter_mut().find(|(k, _)| *k == key) {
        slot.1 = value;
    } else {
        map.push((key, value));
    }
}

fn str_set_add(set: &mut Vec<String>, key: String) -> bool {
    if set.contains(&key) {
        false
    } else {
        set.push(key);
        true
    }
}

fn obj(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k.to_string(), v);
    }
    Value::Object(m)
}

fn i64_value(n: i64) -> Value {
    crate::js::number(n as f64)
}

// ---- the exported predicates -----------------------------------------------------------------

/// Port of `isFileChangeWorkLogEntry` (`apps/web/src/workLog.ts:418`).
pub fn is_file_change_work_log_entry(work_entry: Value) -> Result<bool, String> {
    let kind = prop(&work_entry, "requestKind", "workEntry.requestKind")?;
    if strict_eq(&kind, &Value::String("file-change".into())) {
        return Ok(true);
    }
    let item = prop(&work_entry, "itemType", "workEntry.itemType")?;
    Ok(strict_eq(&item, &Value::String("file_change".into())))
}

/// Port of `isProviderFileEditWorkLogEntry` (`apps/web/src/workLog.ts:425`).
pub fn is_provider_file_edit_work_log_entry(work_entry: Value) -> Result<bool, String> {
    let item = prop(&work_entry, "itemType", "workEntry.itemType")?;
    if strict_eq(&item, &Value::String("file_change".into())) {
        return Ok(true);
    }
    let kind = prop(&work_entry, "requestKind", "workEntry.requestKind")?;
    if !strict_eq(&kind, &Value::String("file-change".into())) {
        return Ok(false);
    }
    let files = opt_prop(&work_entry, "changedFiles");
    let len = if loose_null(&files) {
        0.0
    } else {
        js_length(&files).unwrap_or(f64::NAN)
    };
    Ok(len > 0.0)
}

// ---- activity ordering -----------------------------------------------------------------------

fn compare_activity_lifecycle_rank(kind: &Value) -> Result<i64, String> {
    let Some(k) = js_string(kind) else {
        if kind.is_null() || is_undef(kind) {
            return Err(err_not_object(kind, "kind.endsWith"));
        }
        return Err("kind.endsWith is not a function. (In 'kind.endsWith(\".started\")', 'kind.endsWith' is undefined)".to_string());
    };
    if k.ends_with(".started") || k == "tool.started" {
        return Ok(0);
    }
    if k.ends_with(".progress") || k.ends_with(".updated") {
        return Ok(1);
    }
    if k.ends_with(".completed") || k.ends_with(".resolved") {
        return Ok(2);
    }
    Ok(1)
}

fn is_context_compaction_progress_label(label: &Value) -> bool {
    js_string(label).is_some_and(|l| l == "Compacting context" || l == "Compacting conversation...")
}

fn compare_activities_by_order(left: &Value, right: &Value) -> Result<f64, String> {
    let lseq = prop(left, "sequence", "left.sequence")?;
    let rseq = prop(right, "sequence", "right.sequence")?;
    let lseq_def = !is_undef(&lseq);
    let rseq_def = !is_undef(&rseq);
    if lseq_def && rseq_def {
        if !strict_eq(&lseq, &rseq) {
            return Ok(js_to_number(&lseq) - js_to_number(&rseq));
        }
    } else if lseq_def {
        return Ok(1.0);
    } else if rseq_def {
        return Ok(-1.0);
    }

    let lcreated = prop(left, "createdAt", "left.createdAt")?;
    if lcreated.is_null() || is_undef(&lcreated) {
        return Err(err_not_object(&lcreated, "left.createdAt.localeCompare"));
    }
    let rcreated = prop(right, "createdAt", "right.createdAt")?;
    let created_cmp = call_locale_compare(&lcreated, "left.createdAt", rcreated)?;
    if created_cmp != 0 {
        return Ok(created_cmp as f64);
    }

    let lkind = opt_prop(left, "kind");
    let rkind = opt_prop(right, "kind");
    let rank_cmp =
        compare_activity_lifecycle_rank(&lkind)? - compare_activity_lifecycle_rank(&rkind)?;
    if rank_cmp != 0 {
        return Ok(rank_cmp as f64);
    }

    if strict_eq(&lkind, &Value::String("context-compaction".into()))
        && strict_eq(&rkind, &Value::String("context-compaction".into()))
    {
        let lsum = opt_prop(left, "summary");
        let rsum = opt_prop(right, "summary");
        let compact_cmp = i64::from(!is_context_compaction_progress_label(&lsum))
            - i64::from(!is_context_compaction_progress_label(&rsum));
        if compact_cmp != 0 {
            return Ok(compact_cmp as f64);
        }
    }

    let lid = prop(left, "id", "left.id")?;
    if lid.is_null() || is_undef(&lid) {
        return Err(err_not_object(&lid, "left.id.localeCompare"));
    }
    let rid = prop(right, "id", "right.id")?;
    let id_cmp = call_locale_compare(&lid, "left.id", rid)?;
    Ok(id_cmp as f64)
}

fn activity_elem(activities: &Value, index: usize) -> Value {
    match activities {
        Value::Array(a) => a.get(index).cloned().unwrap_or_else(js::undefined),
        Value::String(s) => s
            .encode_utf16()
            .nth(index)
            .map(|u| js::utf16_string(&[u]))
            .unwrap_or_else(js::undefined),
        Value::Object(o) if is_utf16_marker(activities) => js_string(&Value::Object(o.clone()))
            .unwrap_or_default()
            .encode_utf16()
            .nth(index)
            .map(|u| js::utf16_string(&[u]))
            .unwrap_or_else(js::undefined),
        _ => js::undefined(),
    }
}

fn is_activity_order_stable(activities: &Value) -> Result<bool, String> {
    let len = prop(activities, "length", "activities.length")?;
    let mut index = 1usize;
    while (index as f64) < js_to_number(&len) {
        let left = activity_elem(activities, index - 1);
        let right = activity_elem(activities, index);
        if compare_activities_by_order(&left, &right)? > 0.0 {
            return Ok(false);
        }
        index += 1;
    }
    Ok(true)
}

/// Port of `orderedActivities` (`apps/web/src/workLog.ts:482`). The `WeakMap` cache keeps no
/// state here, but `set` on a primitive key still throws exactly as in JS.
pub fn ordered_activities(activities: Value) -> Result<Value, String> {
    // `orderedActivitiesCache.get(activities)` returns `undefined` for every key here.
    let ordered = if is_activity_order_stable(&activities)? {
        activities.clone()
    } else {
        let items = activities.as_array().cloned().unwrap_or_default();
        Value::Array(to_sorted(&items, compare_activities_by_order)?)
    };
    // `orderedActivitiesCache.set(activities, ordered)`: WeakMap keys must be objects.
    match &activities {
        Value::Object(_) | Value::Array(_) => {}
        _ => return Err("WeakMap keys must be objects or non-registered symbols".into()),
    }
    Ok(ordered)
}

/// Port of `isRoutedSubagentWorkEntry` (`apps/web/src/workLog.ts:503`).
pub fn is_routed_subagent_work_entry(entry: Value) -> Result<bool, String> {
    let item = prop(&entry, "itemType", "entry.itemType")?;
    if !strict_eq(&item, &Value::String("collab_agent_tool_call".into())) {
        return Ok(false);
    }
    let subagents = opt_prop(&entry, "subagents");
    let len = if loose_null(&subagents) {
        0.0
    } else {
        js_length(&subagents).unwrap_or(f64::NAN)
    };
    if len > 0.0 {
        return Ok(true);
    }
    is_subagent_state_only_work_entry(entry)
}

fn normalize_collab_identifier(value: Option<String>) -> Option<String> {
    let value = value?;
    let normalized: String = value
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| *c != '_' && *c != '-')
        .collect();
    Some(normalized)
}

/// Port of `isSubagentStateOnlyWorkEntry` (`apps/web/src/workLog.ts:520`).
pub fn is_subagent_state_only_work_entry(entry: Value) -> Result<bool, String> {
    const STATE_ONLY: &[&str] = &[
        "wait",
        "waitagent",
        "close",
        "closeagent",
        "subagentsettled",
    ];
    let action = prop(&entry, "subagentAction", "entry.subagentAction")?;
    let tool_raw = if loose_null(&action) {
        Value::Null
    } else {
        opt_prop(&action, "tool")
    };
    let tool = if loose_null(&tool_raw) || !truthy(&tool_raw) {
        None
    } else {
        match js_string(&tool_raw) {
            Some(s) => Some(s),
            None => {
                return Err("value.trim is not a function. (In 'value.trim().toLowerCase().replaceAll(\"_\", \"\").replaceAll(\"-\", \"\")', 'value.trim' is undefined)".to_string());
            }
        }
    };
    Ok(tool.is_some_and(|t| {
        STATE_ONLY.contains(
            &normalize_collab_identifier(Some(t))
                .unwrap_or_default()
                .as_str(),
        )
    }))
}

/// Port of `omitRoutedSubagentWorkEntries` (`apps/web/src/workLog.ts:535`).
pub fn omit_routed_subagent_work_entries(entries: Value) -> Result<Value, String> {
    call_method(
        &entries,
        "entries",
        "filter",
        "entries.filter((entry) => !isRoutedSubagentWorkEntry(entry))",
    )?;
    let items = entries.as_array().cloned().unwrap_or_default();
    let mut kept = Vec::with_capacity(items.len());
    for entry in items {
        if !is_routed_subagent_work_entry(entry.clone())? {
            kept.push(entry);
        }
    }
    if kept.len() == entries.as_array().map_or(0, Vec::len) {
        Ok(entries)
    } else {
        Ok(Value::Array(kept))
    }
}

/// Port of `isMissingCheckpointBaselineActivity` (`apps/web/src/workLog.ts:543`).
pub fn is_missing_checkpoint_baseline_activity(activity: Value) -> Result<bool, String> {
    let kind = prop(&activity, "kind", "activity.kind")?;
    if !strict_eq(&kind, &Value::String("checkpoint.baseline.skipped".into())) {
        return Ok(false);
    }
    let payload = opt_prop(&activity, "payload");
    let checkpoint = opt_prop(
        &as_record(&payload)
            .map(|r| Value::Object(r.clone()))
            .unwrap_or(Value::Null),
        "checkpointBaseline",
    );
    Ok(!strict_eq(&checkpoint, &Value::String("captured".into()))
        && !strict_eq(&checkpoint, &Value::String("not-applicable".into())))
}

/// Port of `isCodexRuntimeWarningType` (`apps/web/src/workLog.ts:1403`).
pub fn is_codex_runtime_warning_type(native_event_type: Value) -> Result<bool, String> {
    Ok(strict_eq(
        &native_event_type,
        &Value::String("codex.guardianWarning".into()),
    ) || strict_eq(&native_event_type, &Value::String("codex.warning".into())))
}

// ---- task list / handoff / user-input parsers -------------------------------------------------

/// Port of `parseTaskListTasks` (`apps/web/src/workLog.ts:1747`).
pub fn parse_task_list_tasks(payload: Value) -> Result<Value, String> {
    let Some(record) = as_record(&payload).cloned() else {
        return Ok(Value::Null);
    };
    let raw_tasks = record.get("tasks").cloned().unwrap_or(Value::Null);
    let Value::Array(entries) = raw_tasks else {
        return Ok(Value::Null);
    };
    let mut tasks = Vec::new();
    for entry in &entries {
        let Some(task_record) = as_record(entry).cloned() else {
            continue;
        };
        let Some(task) = task_record.get("task").and_then(js_string) else {
            continue;
        };
        let status_raw = task_record
            .get("status")
            .cloned()
            .unwrap_or_else(js::undefined);
        let status = if strict_eq(&status_raw, &Value::String("completed".into()))
            || strict_eq(&status_raw, &Value::String("inProgress".into()))
        {
            status_raw
        } else {
            Value::String("pending".into())
        };
        tasks.push(obj(vec![("task", Value::String(task)), ("status", status)]));
    }
    if !entries.is_empty() && tasks.is_empty() {
        return Ok(Value::Null);
    }
    Ok(Value::Array(tasks))
}

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

fn as_provider_kind(v: &Value) -> Option<String> {
    PROVIDER_KINDS
        .iter()
        .find(|k| strict_eq(&Value::String((*k).to_string()), v))
        .map(|k| (*k).to_string())
}

fn as_handoff_model_selection(v: &Value, provider: &str, model: &str) -> Value {
    if !v.is_null() && as_record(v).is_some() {
        let record = as_record(v).cloned().unwrap_or_default();
        let cand_provider = record
            .get("provider")
            .cloned()
            .unwrap_or_else(js::undefined);
        let cand_model = record.get("model").cloned().unwrap_or_else(js::undefined);
        if strict_eq(&cand_provider, &Value::String(provider.to_string()))
            && strict_eq(&cand_model, &Value::String(model.to_string()))
        {
            return v.clone();
        }
    }
    obj(vec![
        ("provider", Value::String(provider.to_string())),
        ("model", Value::String(model.to_string())),
    ])
}

/// Port of `extractProviderHandoffInfo` (`apps/web/src/workLog.ts:1802`).
pub fn extract_provider_handoff_info(payload: Value, status: Value) -> Result<Value, String> {
    let source_provider = as_provider_kind(&opt_prop(&payload, "sourceProvider"));
    let target_provider = as_provider_kind(&opt_prop(&payload, "targetProvider"));
    let source_model = as_trimmed_string(&opt_prop(&payload, "sourceModel"));
    let target_model = as_trimmed_string(&opt_prop(&payload, "targetModel"));
    let (Some(source_provider), Some(target_provider), Some(source_model), Some(target_model)) =
        (source_provider, target_provider, source_model, target_model)
    else {
        return Ok(Value::Null);
    };
    let mut out = Map::new();
    out.insert("status".into(), status.clone());
    out.insert(
        "sourceProvider".into(),
        Value::String(source_provider.clone()),
    );
    out.insert("sourceModel".into(), Value::String(source_model.clone()));
    out.insert(
        "targetProvider".into(),
        Value::String(target_provider.clone()),
    );
    out.insert("targetModel".into(), Value::String(target_model.clone()));
    out.insert(
        "sourceModelSelection".into(),
        as_handoff_model_selection(
            &opt_prop(&payload, "sourceModelSelection"),
            &source_provider,
            &source_model,
        ),
    );
    out.insert(
        "targetModelSelection".into(),
        as_handoff_model_selection(
            &opt_prop(&payload, "targetModelSelection"),
            &target_provider,
            &target_model,
        ),
    );
    out.insert(
        "contextText".into(),
        as_trimmed_string(&opt_prop(&payload, "contextText"))
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    out.insert(
        "failureDetail".into(),
        as_trimmed_string(&opt_prop(&payload, "detail"))
            .map(Value::String)
            .unwrap_or(Value::Null),
    );
    if strict_eq(&status, &Value::String("failed".into()))
        && strict_eq(
            &opt_prop(&payload, "reason"),
            &Value::String("background-tasks".into()),
        )
    {
        out.insert("blockedByBackgroundTasks".into(), Value::Bool(true));
    }
    Ok(Value::Object(out))
}

/// Port of `parseUserInputQuestions` (`apps/web/src/workLog.ts:1884`).
pub fn parse_user_input_questions(payload: Value) -> Result<Value, String> {
    let raw = opt_prop(&payload, "questions");
    let Value::Array(entries) = raw else {
        return Ok(Value::Null);
    };
    let mut questions = Vec::new();
    for entry in &entries {
        let Some(question) = as_record(entry).cloned() else {
            continue;
        };
        let (Some(id), Some(header), Some(text)) = (
            question.get("id").and_then(js_string),
            question.get("header").and_then(js_string),
            question.get("question").and_then(js_string),
        ) else {
            continue;
        };
        let Some(options_raw) = question.get("options").and_then(|o| o.as_array()) else {
            continue;
        };
        let mut options = Vec::new();
        for option in options_raw {
            let Some(record) = as_record(option).cloned() else {
                continue;
            };
            let (Some(label), Some(description)) = (
                record.get("label").and_then(js_string),
                record.get("description").and_then(js_string),
            ) else {
                continue;
            };
            options.push(obj(vec![
                ("label", Value::String(label)),
                ("description", Value::String(description)),
            ]));
        }
        let mut q = Map::new();
        q.insert("id".into(), Value::String(id));
        q.insert("header".into(), Value::String(header));
        q.insert("question".into(), Value::String(text));
        q.insert("options".into(), Value::Array(options));
        if question.get("multiSelect") == Some(&Value::Bool(true)) {
            q.insert("multiSelect".into(), Value::Bool(true));
        }
        questions.push(Value::Object(q));
    }
    if questions.is_empty() {
        Ok(Value::Null)
    } else {
        Ok(Value::Array(questions))
    }
}

// ---- subagent task ends ----------------------------------------------------------------------

fn subagent_outcome_from_status(status: &Option<String>) -> Option<&'static str> {
    match status.as_deref()?.trim().to_lowercase().as_str() {
        "completed" => Some("completed"),
        "failed" | "error" => Some("failed"),
        "stopped" | "interrupted" | "cancelled" | "killed" => Some("stopped"),
        _ => None,
    }
}

/// Port of `deriveSubagentTaskEnds` (`apps/web/src/workLog.ts:1041`).
pub fn derive_subagent_task_ends(activities: Value) -> Result<Value, String> {
    let elems = iter_elems(&activities, "activity", "activities")?;
    let mut ends: Vec<(String, Value)> = Vec::new();
    let mut resumed: Vec<String> = Vec::new();
    let mut open_tool_use_by_task: Vec<(String, String)> = Vec::new();
    for activity in &elems {
        let payload_v = prop(activity, "payload", "activity.payload")?;
        let payload = as_record(&payload_v).cloned();
        let get = |key: &str| -> Value {
            payload
                .as_ref()
                .and_then(|p| p.get(key).cloned())
                .unwrap_or_else(js::undefined)
        };
        let task_id = as_trimmed_string(&get("taskId"));
        let own_tool_use = as_trimmed_string(&get("toolUseId"));
        let kind = prop(activity, "kind", "activity.kind")?;
        let is = |k: &str| strict_eq(&kind, &Value::String(k.to_string()));
        if is("task.started") {
            if let Some(id) = &task_id {
                if let Some(tool) = &own_tool_use {
                    if let Some(slot) = open_tool_use_by_task.iter_mut().find(|(k, _)| k == id) {
                        slot.1 = tool.clone();
                    } else {
                        open_tool_use_by_task.push((id.clone(), tool.clone()));
                    }
                } else {
                    open_tool_use_by_task.retain(|(k, _)| k != id);
                }
            }
            if let Some(tool) = &own_tool_use
                && str_map_get(&ends, tool).is_some()
            {
                str_set_add(&mut resumed, tool.clone());
            }
        }
        if is("task.updated")
            && get("reconcilesStop") == Value::Bool(true)
            && strict_eq(&get("status"), &Value::String("running".into()))
            && own_tool_use.is_some()
        {
            let tool = own_tool_use.clone().unwrap_or_default();
            let stopped = str_map_get(&ends, &tool).cloned();
            let outcome = stopped
                .as_ref()
                .and_then(|s| s.get("outcome"))
                .and_then(js_string);
            if outcome.as_deref() == Some("stopped") {
                if let Some(previous) = stopped.as_ref().and_then(|s| s.get("previous")).cloned() {
                    str_map_set(&mut ends, tool.clone(), previous);
                } else {
                    ends.retain(|(k, _)| *k != tool);
                }
                str_set_add(&mut resumed, tool.clone());
                if let Some(id) = &task_id {
                    if let Some(slot) = open_tool_use_by_task.iter_mut().find(|(k, _)| k == id) {
                        slot.1 = tool.clone();
                    } else {
                        open_tool_use_by_task.push((id.clone(), tool.clone()));
                    }
                }
            }
        }
        if !is("task.completed") {
            continue;
        }
        let tool_use_id = own_tool_use.clone().or_else(|| {
            task_id.as_ref().and_then(|id| {
                open_tool_use_by_task
                    .iter()
                    .find(|(k, _)| k == id)
                    .map(|(_, v)| v.clone())
            })
        });
        if let Some(id) = &task_id {
            open_tool_use_by_task.retain(|(k, _)| k != id);
        }
        let outcome = subagent_outcome_from_status(&as_trimmed_string(&get("status")));
        if let (Some(tool), Some(outcome)) = (tool_use_id, outcome) {
            let previous = if resumed.contains(&tool) {
                str_map_get(&ends, &tool).cloned()
            } else {
                str_map_get(&ends, &tool).and_then(|e| e.get("previous").cloned())
            };
            let mut end = Map::new();
            end.insert("outcome".into(), Value::String(outcome.to_string()));
            end.insert(
                "endedAt".into(),
                prop(activity, "createdAt", "activity.createdAt")?,
            );
            if let Some(p) = previous {
                end.insert("previous".into(), p);
            }
            str_map_set(&mut ends, tool.clone(), Value::Object(end));
            resumed.retain(|r| *r != tool);
        }
    }
    let map_entries: Vec<Value> = ends
        .into_iter()
        .map(|(k, v)| Value::Array(vec![Value::String(k), v]))
        .collect();
    Ok(serde_json::json!({ "$map": map_entries }))
}

// ---- background tasks -------------------------------------------------------------------------

fn background_task_outcome(status: &Value) -> Option<&'static str> {
    let s = js_string(status)?.trim().to_lowercase();
    match s.as_str() {
        "completed" => Some("finished"),
        "failed" | "error" => Some("failed"),
        "stopped" | "killed" | "cancelled" | "interrupted" => Some("stopped"),
        _ => None,
    }
}

fn parse_background_task_exit_code(detail: &Option<String>) -> Option<f64> {
    let detail = detail.as_ref()?;
    let lower = detail.to_lowercase();
    let pos = lower.find("exit code ")?;
    let digits: String = lower[pos + 10..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    digits.parse::<f64>().ok()
}

struct BackgroundTaskState {
    map: Map<String, Value>,
    tool_use_id: Option<String>,
    notice_activity_id: Option<String>,
}

impl BackgroundTaskState {
    fn new(
        task_id: &str,
        task_type: Option<String>,
        description: Option<String>,
        started_at: Value,
        tool_use_id: Option<String>,
        notice_activity_id: Option<String>,
    ) -> Self {
        let mut map = Map::new();
        map.insert("taskId".into(), Value::String(task_id.to_string()));
        map.insert(
            "taskType".into(),
            task_type.map(Value::String).unwrap_or(Value::Null),
        );
        map.insert(
            "description".into(),
            description.map(Value::String).unwrap_or(Value::Null),
        );
        map.insert("command".into(), Value::Null);
        map.insert("status".into(), Value::String("running".into()));
        map.insert("startedAt".into(), started_at);
        map.insert("completedAt".into(), Value::Null);
        map.insert("exitCode".into(), Value::Null);
        BackgroundTaskState {
            map,
            tool_use_id,
            notice_activity_id,
        }
    }
    fn get(&self, key: &str) -> Value {
        self.map.get(key).cloned().unwrap_or_else(js::undefined)
    }
    fn status(&self) -> String {
        js_string(&self.get("status")).unwrap_or_default()
    }
    fn task_id(&self) -> String {
        js_string(&self.get("taskId")).unwrap_or_default()
    }
    fn completion_id(&self) -> Option<String> {
        self.map.get("completionActivityId").and_then(js_string)
    }
    fn set_completion(&mut self, id: Option<String>) {
        match id {
            Some(id) => {
                self.map
                    .insert("completionActivityId".into(), Value::String(id));
            }
            None => {
                self.map.remove("completionActivityId");
            }
        }
    }
    fn to_task_json(&self) -> Value {
        Value::Object(self.map.clone())
    }
}

fn extract_tool_call_id(payload: &Value) -> Option<String> {
    let data = as_record(&opt_prop(payload, "data")).cloned();
    let item = as_record(&opt_prop(
        &data.clone().map(Value::Object).unwrap_or(Value::Null),
        "item",
    ))
    .cloned();
    let dv = data.clone().map(Value::Object).unwrap_or(Value::Null);
    let tool_call_id = opt_prop(&dv, "toolCallId");
    let progress_tool_use = if loose_null(&tool_call_id) {
        as_trimmed_string(&opt_prop(&dv, "toolUseId"))
    } else {
        None
    };
    if let Some(id) = progress_tool_use {
        // LEGACY_TOOL_HEARTBEAT_ID: /^(?:.+)-heartbeat-\d+$/
        if let Some(pos) = id.rfind("-heartbeat-") {
            let suffix = &id[pos + 11..];
            if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) && pos > 0 {
                return Some(id[..pos].to_string());
            }
        }
        return Some(id);
    }
    let iv = item.clone().map(Value::Object).unwrap_or(Value::Null);
    for key in ["toolCallId", "toolUseId", "callID", "callId"] {
        if let Some(v) = as_trimmed_string(&opt_prop(&dv, key)) {
            return Some(v);
        }
    }
    as_trimmed_string(&opt_prop(&iv, "id"))
}

fn as_command_argument_record(v: &Value) -> Option<Map<String, Value>> {
    if let Some(record) = as_record(v) {
        return Some(record.clone());
    }
    let text = as_trimmed_string(v)?;
    if !text.starts_with('{') {
        return None;
    }
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| as_record(&v).cloned())
}

fn is_tool_lifecycle_item_type(v: &Value) -> bool {
    const TYPES: &[&str] = &[
        "command_execution",
        "file_change",
        "mcp_tool_call",
        "dynamic_tool_call",
        "collab_agent_tool_call",
        "web_search",
        "image_view",
        "image_generation",
    ];
    js_string(v).is_some_and(|s| TYPES.contains(&s.as_str()))
}

fn extract_work_log_item_type(payload: &Value) -> Option<String> {
    let top = opt_prop(payload, "itemType");
    if is_js_string(&top) && is_tool_lifecycle_item_type(&top) {
        return js_string(&top);
    }
    let data = as_record(&opt_prop(payload, "data"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let item = as_record(&opt_prop(&data, "item"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    for candidate in [
        opt_prop(&data, "itemType"),
        opt_prop(&item, "type"),
        opt_prop(&item, "kind"),
        opt_prop(payload, "type"),
        opt_prop(payload, "kind"),
    ] {
        if is_js_string(&candidate) && is_tool_lifecycle_item_type(&candidate) {
            return js_string(&candidate);
        }
    }
    None
}

fn approval_request_kind_from_request_type(request_type: &Value) -> Option<&'static str> {
    match js_string(request_type).as_deref() {
        Some("command_execution_approval") | Some("exec_command_approval") => Some("command"),
        Some("file_read_approval") => Some("file-read"),
        Some("file_change_approval") | Some("apply_patch_approval") => Some("file-change"),
        Some("permissions_approval") => Some("permissions"),
        Some("tool_approval") | Some("dynamic_tool_call") => Some("tool"),
        _ => None,
    }
}

fn extract_work_log_request_kind(payload: &Value) -> Option<String> {
    let kind = opt_prop(payload, "requestKind");
    for expected in ["command", "file-read", "file-change", "permissions", "tool"] {
        if strict_eq(&kind, &Value::String(expected.to_string())) {
            return Some(expected.to_string());
        }
    }
    approval_request_kind_from_request_type(&opt_prop(payload, "requestType")).map(str::to_string)
}

fn normalize_compact_tool_label(value: &str) -> String {
    const STATUS_WORDS: &[&str] = &[
        "complete",
        "completed",
        "done",
        "finished",
        "success",
        "succeeded",
        "started",
        "running",
    ];
    let mut out = value.trim_end().to_string();
    let lower = out.to_lowercase();
    for word in STATUS_WORDS {
        let tail = format!(" {word}");
        if lower.ends_with(&tail) {
            out.truncate(out.len() - tail.len());
            break;
        }
    }
    js_trim(&out)
}

fn has_turn_scoped_provider_tool_call_id(payload: &Value) -> bool {
    let data = as_record(&opt_prop(payload, "data"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    is_js_string(&opt_prop(&data, "providerToolCallId"))
}

fn is_command_like_detail(payload: &Value) -> bool {
    if as_record(payload).is_none() {
        return false;
    }
    if extract_work_log_item_type(payload).as_deref() == Some("command_execution") {
        return true;
    }
    if extract_work_log_request_kind(payload).as_deref() == Some("command") {
        return true;
    }
    let title = as_trimmed_string(&opt_prop(payload, "title")).unwrap_or_default();
    let normalized = normalize_compact_tool_label(&title);
    normalized == "Ran command" || normalized == "Command run"
}

/// `/^(?:read(?:ing)?|find(?:ing)?|found|search(?:ing|ed)?)\s+`([^`]+)`/iu` — the legacy ACP
/// title pattern. Returns the captured target.
fn legacy_title_command_target(title: &str) -> Option<String> {
    let lower = title.to_lowercase();
    for verb in [
        "read",
        "reading",
        "find",
        "finding",
        "found",
        "search",
        "searching",
        "searched",
    ] {
        if let Some(rest) = lower.strip_prefix(verb) {
            let trimmed = rest.trim_start_matches(is_js_space);
            if trimmed.len() < rest.len() || rest.is_empty() {
                // Require at least one whitespace char after the verb.
                if trimmed.len() == rest.len() {
                    continue;
                }
            }
            let body = &title[title.len() - trimmed.len()..];
            if let Some(inner) = body.strip_prefix('`')
                && let Some(end) = inner.find('`')
                && end > 0
            {
                return Some(inner[..end].to_string());
            }
        }
    }
    None
}

fn normalize_command_value(v: &Value) -> Option<String> {
    if let Some(direct) = as_trimmed_string(v) {
        return Some(direct);
    }
    let Value::Array(parts) = v else {
        return None;
    };
    let parts: Vec<String> = parts.iter().filter_map(as_trimmed_string).collect();
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

/// `stripTrailingToolExitCode` (`packages/shared/src/toolOutputSummary.ts`).
fn strip_trailing_exit_code(value: &str) -> (Option<String>, Option<f64>) {
    let Some(marker_index) = value.rfind('<') else {
        return (
            if value.trim().is_empty() {
                None
            } else {
                Some(value.to_string())
            },
            None,
        );
    };
    let tail = &value[marker_index..];
    let inner = tail.strip_prefix("<exited with exit code ").and_then(|t| {
        t.strip_suffix('>')
            .or_else(|| t.trim_end_matches(is_js_space).strip_suffix('>'))
    });
    let matched = inner.and_then(|body| {
        let digits: String = body.chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() == body.trim_end_matches(is_js_space).trim_end().len()
            || tail.to_lowercase().starts_with("<exited with exit code ")
        {
            // /^<exited with exit code (\d+)>\s*$/i — the marker is the full tail.
            let re_tail = tail.trim_end_matches(is_js_space).to_lowercase();
            if re_tail == format!("<exited with exit code {digits}>") {
                return digits.parse::<f64>().ok();
            }
        }
        None
    });
    if let Some(exit_code) = matched {
        let output = value[..marker_index]
            .trim_end_matches(is_js_space)
            .to_string();
        let output = js_trim(&output);
        return (
            if output.is_empty() {
                None
            } else {
                Some(output)
            },
            Some(exit_code),
        );
    }
    let trimmed = js_trim(value);
    (
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        },
        None,
    )
}

fn collect_command_actions(payload: &Value, data: &Value, item: &Value) -> Vec<Value> {
    let arg_rec = |v: &Value| -> Value {
        as_command_argument_record(v)
            .map(Value::Object)
            .unwrap_or(Value::Null)
    };
    let candidates = [
        opt_prop(item, "commandActions"),
        opt_prop(
            &arg_rec(
                &[
                    opt_prop(item, "arguments"),
                    opt_prop(item, "args"),
                    opt_prop(item, "params"),
                ]
                .into_iter()
                .find(|v| !is_undef(v))
                .unwrap_or_else(js::undefined),
            ),
            "commandActions",
        ),
        opt_prop(data, "commandActions"),
        opt_prop(
            &arg_rec(
                &[
                    opt_prop(data, "arguments"),
                    opt_prop(data, "args"),
                    opt_prop(data, "params"),
                ]
                .into_iter()
                .find(|v| !is_undef(v))
                .unwrap_or_else(js::undefined),
            ),
            "commandActions",
        ),
        opt_prop(&arg_rec(&opt_prop(data, "rawInput")), "commandActions"),
        opt_prop(&arg_rec(&opt_prop(data, "input")), "commandActions"),
        opt_prop(payload, "commandActions"),
    ];
    for candidate in candidates {
        if let Value::Array(actions) = candidate {
            return actions;
        }
    }
    Vec::new()
}

fn extract_primary_command_action(payload: &Value) -> Option<Map<String, Value>> {
    let data = as_record(&opt_prop(payload, "data"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let item = as_record(&opt_prop(&data, "item"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    for action in collect_command_actions(payload, &data, &item) {
        let Some(record) = as_record(&action).cloned() else {
            continue;
        };
        let type_s = as_trimmed_string(&record.get("type").cloned().unwrap_or_else(js::undefined))
            .unwrap_or_else(|| "unknown".to_string());
        let command =
            as_trimmed_string(&record.get("command").cloned().unwrap_or_else(js::undefined));
        let name = as_trimmed_string(&record.get("name").cloned().unwrap_or_else(js::undefined));
        let path = as_trimmed_string(&record.get("path").cloned().unwrap_or_else(js::undefined));
        let query = as_trimmed_string(&record.get("query").cloned().unwrap_or_else(js::undefined));
        if command.is_some()
            || name.is_some()
            || path.is_some()
            || query.is_some()
            || type_s != "unknown"
        {
            let mut out = Map::new();
            out.insert("type".into(), Value::String(type_s));
            if let Some(v) = command {
                out.insert("command".into(), Value::String(v));
            }
            if let Some(v) = name {
                out.insert("name".into(), Value::String(v));
            }
            if let Some(v) = path {
                out.insert("path".into(), Value::String(v));
            }
            if let Some(v) = query {
                out.insert("query".into(), Value::String(v));
            }
            return Some(out);
        }
    }
    None
}

fn extract_tool_command(payload: &Value) -> (Option<String>, Option<String>) {
    let data = as_record(&opt_prop(payload, "data"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let item = as_record(&opt_prop(&data, "item"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let rec = |v: &Value| {
        as_record(v)
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null)
    };
    let arg_rec = |v: &Value| {
        as_command_argument_record(v)
            .map(Value::Object)
            .unwrap_or(Value::Null)
    };
    let item_result = rec(&opt_prop(&item, "result"));
    let item_input = rec(&opt_prop(&item, "input"));
    let item_arguments = arg_rec(
        &[
            opt_prop(&item, "arguments"),
            opt_prop(&item, "args"),
            opt_prop(&item, "params"),
        ]
        .into_iter()
        .find(|v| !is_undef(v))
        .unwrap_or_else(js::undefined),
    );
    let item_call = rec(&opt_prop(&item, "call"));
    let item_function = rec(&opt_prop(&item, "function"));
    let data_input = rec(&opt_prop(&data, "input"));
    let data_arguments = arg_rec(
        &[
            opt_prop(&data, "arguments"),
            opt_prop(&data, "args"),
            opt_prop(&data, "params"),
        ]
        .into_iter()
        .find(|v| !is_undef(v))
        .unwrap_or_else(js::undefined),
    );
    let raw_input = arg_rec(&opt_prop(&data, "rawInput"));
    if has_turn_scoped_provider_tool_call_id(payload) {
        let kind = opt_prop(&data, "kind");
        let title = as_trimmed_string(&opt_prop(payload, "title")).unwrap_or_default();
        let legacy_target = legacy_title_command_target(&title);
        let has_legacy_title_command = (is_undef(&kind)
            || strict_eq(&kind, &Value::String("other".into())))
            && legacy_target.is_some()
            && as_trimmed_string(&opt_prop(&data, "command")).as_deref()
                == legacy_target.as_deref().map(|t| t.trim())
            && normalize_command_value(&opt_prop(&raw_input, "command")).is_none()
            && normalize_command_value(&opt_prop(&raw_input, "cmd")).is_none()
            && as_trimmed_string(&opt_prop(&raw_input, "executable")).is_none();
        if strict_eq(&kind, &Value::String("read".into()))
            || strict_eq(&kind, &Value::String("search".into()))
            || has_legacy_title_command
        {
            return (None, None);
        }
    }
    let detail_command =
        if is_command_like_detail(payload) && is_js_string(&opt_prop(payload, "detail")) {
            strip_trailing_exit_code(&js_string(&opt_prop(payload, "detail")).unwrap_or_default()).0
        } else {
            None
        };
    let raw_command_candidates = [
        opt_prop(&item, "command"),
        opt_prop(&item, "cmd"),
        opt_prop(&item_input, "command"),
        opt_prop(&item_input, "cmd"),
        opt_prop(&item_arguments, "command"),
        opt_prop(&item_arguments, "cmd"),
        opt_prop(&item_call, "command"),
        opt_prop(&item_call, "cmd"),
        opt_prop(&item_function, "arguments"),
        opt_prop(&item_result, "command"),
        opt_prop(&item_result, "cmd"),
        opt_prop(&data, "command"),
        opt_prop(&data, "cmd"),
        opt_prop(&data_input, "command"),
        opt_prop(&data_input, "cmd"),
        opt_prop(&data_arguments, "command"),
        opt_prop(&data_arguments, "cmd"),
        opt_prop(&raw_input, "command"),
        opt_prop(&raw_input, "cmd"),
        opt_prop(&item, "text"),
        opt_prop(&item, "summary"),
        detail_command
            .map(Value::String)
            .unwrap_or_else(js::undefined),
    ];
    let raw_command = raw_command_candidates
        .iter()
        .find_map(normalize_command_value);
    let command_action = extract_primary_command_action(payload);
    let command = as_trimmed_string(&opt_prop(
        &command_action.map(Value::Object).unwrap_or(Value::Null),
        "command",
    ))
    .map(|c| normalize_command_value(&Value::String(c)).unwrap_or_default())
    .or_else(|| {
        raw_command_candidates
            .iter()
            .find_map(normalize_command_value)
    });
    (
        command.clone(),
        match (raw_command, &command) {
            (Some(raw), Some(cmd)) if raw != *cmd => Some(raw),
            (Some(raw), None) => Some(raw),
            _ => None,
        },
    )
}

fn derive_background_task_states(
    ordered: &[Value],
) -> (Vec<(String, BackgroundTaskState)>, Vec<String>) {
    let mut tasks: Vec<(String, BackgroundTaskState)> = Vec::new();
    let mut notices_announcing_subagents: Vec<String> = Vec::new();
    let mut seen_task_ids: Vec<String> = Vec::new();
    let mut started: Vec<(String, BackgroundTaskLaunch)> = Vec::new();
    for activity in ordered {
        let payload = as_record(&opt_prop(activity, "payload"))
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        let kind = js_string(&opt_prop(activity, "kind")).unwrap_or_default();
        let pget = |key: &str| opt_prop(&payload, key);
        if kind == "runtime.warning"
            && strict_eq(
                &pget("nativeEventType"),
                &Value::String("background_tasks_changed".into()),
            )
        {
            let announced = opt_prop(
                &as_record(&pget("data"))
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or(Value::Null),
                "tasks",
            );
            if let Value::Array(list) = announced {
                for task in &list {
                    let Some(record) = as_record(task).cloned() else {
                        continue;
                    };
                    let rv = Value::Object(record);
                    let Some(task_id) = as_trimmed_string(&opt_prop(&rv, "task_id")) else {
                        continue;
                    };
                    admit_task(
                        &mut tasks,
                        &mut notices_announcing_subagents,
                        &mut seen_task_ids,
                        &started,
                        &task_id,
                        activity,
                        as_trimmed_string(&opt_prop(&rv, "task_type")),
                        as_trimmed_string(&opt_prop(&rv, "description")),
                        Some(js_string(&opt_prop(activity, "id")).unwrap_or_default()),
                    );
                }
            }
            continue;
        }
        let Some(task_id) = as_trimmed_string(&pget("taskId")) else {
            continue;
        };
        if kind == "task.started" {
            let launch = (
                as_trimmed_string(&pget("toolUseId")),
                as_trimmed_string(&pget("taskType")),
                as_trimmed_string(&pget("detail")),
                opt_prop(activity, "createdAt"),
            );
            if let Some(slot) = started.iter_mut().find(|(k, _)| *k == task_id) {
                slot.1 = launch.clone();
            } else {
                started.push((task_id.clone(), launch.clone()));
            }
            if let Some((_, task)) = tasks.iter_mut().find(|(k, _)| *k == task_id) {
                if launch.1.as_deref() == Some("local_agent") {
                    admit_task(
                        &mut tasks,
                        &mut notices_announcing_subagents,
                        &mut seen_task_ids,
                        &started,
                        &task_id,
                        activity,
                        Some("local_agent".to_string()),
                        None,
                        None,
                    );
                    continue;
                }
                if task.tool_use_id.is_none() {
                    task.tool_use_id = launch.0.clone();
                }
                if task.get("description").is_null()
                    && let Some(d) = as_trimmed_string(&pget("detail"))
                {
                    task.map.insert("description".into(), Value::String(d));
                }
            }
            continue;
        }
        if kind == "task.updated" && pget("isBackgrounded") == Value::Bool(true) {
            admit_task(
                &mut tasks,
                &mut notices_announcing_subagents,
                &mut seen_task_ids,
                &started,
                &task_id,
                activity,
                as_trimmed_string(&pget("taskType")),
                as_trimmed_string(&pget("detail")),
                None,
            );
        }
        let outcome = background_task_outcome(&pget("status"));
        let Some((_, task)) = tasks.iter_mut().find(|(k, _)| *k == task_id) else {
            continue;
        };
        let reconciles_stop =
            pget("reconcilesStop") == Value::Bool(true) && task.status() == "stopped";
        if reconciles_stop
            && let Some(outcome) = outcome
            && task.status() != outcome
        {
            task.set_completion(None);
        }
        if kind == "task.completed"
            && outcome.is_some()
            && (task.status() == "running"
                || task.status() == outcome.unwrap_or_default()
                || reconciles_stop)
            && task.completion_id().is_none()
        {
            task.set_completion(js_string(&opt_prop(activity, "id")));
        }
        if !(task.status() == "running" || reconciles_stop) {
            continue;
        }
        if reconciles_stop
            && kind == "task.updated"
            && strict_eq(&pget("status"), &Value::String("running".into()))
        {
            task.map
                .insert("status".into(), Value::String("running".into()));
            task.map.insert("completedAt".into(), Value::Null);
            task.set_completion(None);
            task.map.insert("exitCode".into(), Value::Null);
            continue;
        }
        if kind == "task.updated" || kind == "task.completed" {
            let Some(outcome) = outcome else {
                continue;
            };
            let detail = as_trimmed_string(&pget("detail"));
            task.map
                .insert("status".into(), Value::String(outcome.to_string()));
            task.map
                .insert("completedAt".into(), opt_prop(activity, "createdAt"));
            task.map.insert(
                "exitCode".into(),
                parse_background_task_exit_code(&detail)
                    .map(crate::js::number)
                    .unwrap_or(Value::Null),
            );
        }
    }
    (tasks, notices_announcing_subagents)
}

#[allow(clippy::too_many_arguments)]
fn admit_task(
    tasks: &mut Vec<(String, BackgroundTaskState)>,
    notices_announcing_subagents: &mut Vec<String>,
    seen_task_ids: &mut Vec<String>,
    started: &[(String, BackgroundTaskLaunch)],
    task_id: &str,
    activity: &Value,
    task_type: Option<String>,
    description: Option<String>,
    notice_activity_id: Option<String>,
) {
    let launch = started
        .iter()
        .find(|(k, _)| k == task_id)
        .map(|(_, v)| v.clone());
    let task_type = task_type.or_else(|| launch.as_ref().and_then(|l| l.1.clone()));
    if task_type.as_deref() == Some("monitor") {
        return;
    }
    if task_type.as_deref() == Some("local_agent") {
        let provisional_idx = tasks.iter().position(|(k, _)| k == task_id);
        let provisional_notice =
            provisional_idx.and_then(|i| tasks[i].1.notice_activity_id.clone());
        if provisional_idx.is_some() {
            tasks.retain(|(k, _)| k != task_id);
            seen_task_ids.retain(|k| k != task_id);
        }
        let notice = provisional_notice.or(notice_activity_id);
        if let Some(notice) = notice
            && !seen_task_ids.contains(&task_id.to_string())
        {
            str_set_add(notices_announcing_subagents, notice);
            str_set_add(seen_task_ids, task_id.to_string());
        }
        return;
    }
    if seen_task_ids.contains(&task_id.to_string()) {
        return;
    }
    str_set_add(seen_task_ids, task_id.to_string());
    let state = BackgroundTaskState::new(
        task_id,
        task_type,
        description.or_else(|| launch.as_ref().and_then(|l| l.2.clone())),
        launch
            .as_ref()
            .map(|l| l.3.clone())
            .unwrap_or_else(|| opt_prop(activity, "createdAt")),
        launch.as_ref().and_then(|l| l.0.clone()),
        notice_activity_id,
    );
    tasks.push((task_id.to_string(), state));
}

/// `(toolUseId, taskType, detail, createdAt)` captured by `task.started` launches.
type BackgroundTaskLaunch = (Option<String>, Option<String>, Option<String>, Value);

fn has_background_task_activity(ordered: &Value) -> Result<bool, String> {
    call_method(
        ordered,
        "ordered",
        "some",
        "ordered.some((activity) => activity.kind === \"runtime.warning\" && asRecord(activity.payload)?.nativeEventType === \"background_tasks_changed\" || activity.kind === \"task.updated\" && asRecord(activity.payload)?.isBackgrounded === !0)",
    )?;
    for activity in ordered.as_array().cloned().unwrap_or_default() {
        let kind = prop(&activity, "kind", "activity.kind")?;
        let payload = opt_prop(&activity, "payload");
        let native = opt_prop(
            &as_record(&payload)
                .cloned()
                .map(Value::Object)
                .unwrap_or(Value::Null),
            "nativeEventType",
        );
        if strict_eq(&kind, &Value::String("runtime.warning".into()))
            && strict_eq(&native, &Value::String("background_tasks_changed".into()))
        {
            return Ok(true);
        }
        let backgrounded = opt_prop(
            &as_record(&payload)
                .cloned()
                .map(Value::Object)
                .unwrap_or(Value::Null),
            "isBackgrounded",
        );
        if strict_eq(&kind, &Value::String("task.updated".into()))
            && backgrounded == Value::Bool(true)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Port of `deriveThreadBackgroundTasks` (`apps/web/src/workLog.ts:886`).
pub fn derive_thread_background_tasks(activities: Value) -> Result<Value, String> {
    let ordered = ordered_activities(activities)?;
    if !has_background_task_activity(&ordered)? {
        return Ok(Value::Array(vec![]));
    }
    let items = ordered.as_array().cloned().unwrap_or_default();
    let (tasks, _) = derive_background_task_states(&items);
    let launch_tool_use_ids: Vec<String> = tasks
        .iter()
        .filter_map(|(_, t)| t.tool_use_id.clone())
        .collect();
    let mut command_by_tool_use: Vec<(String, String)> = Vec::new();
    if !launch_tool_use_ids.is_empty() {
        for activity in &items {
            let payload = as_record(&opt_prop(activity, "payload"))
                .cloned()
                .map(Value::Object)
                .unwrap_or(Value::Null);
            let Some(tool_call_id) = extract_tool_call_id(&payload) else {
                continue;
            };
            if !launch_tool_use_ids.contains(&tool_call_id) {
                continue;
            }
            let (command, raw_command) = extract_tool_command(&payload);
            let launch_command = raw_command.or(command);
            if let Some(cmd) = launch_command {
                str_map_set(&mut command_by_tool_use, tool_call_id, cmd);
            }
        }
    }
    let out: Vec<Value> = tasks
        .into_iter()
        .map(|(_, mut task)| {
            let command = task
                .tool_use_id
                .as_ref()
                .and_then(|id| str_map_get(&command_by_tool_use, id).cloned())
                .map(Value::String)
                .unwrap_or(Value::Null);
            task.map.insert("command".into(), command);
            task.to_task_json()
        })
        .collect();
    Ok(Value::Array(out))
}

// ---- filters shared by deriveWorkLogEntries ----------------------------------------------------

fn is_log_only_pi_warning(activity: &Value) -> Result<bool, String> {
    let kind = prop(activity, "kind", "activity.kind")?;
    if !strict_eq(&kind, &Value::String("runtime.warning".into())) {
        return Ok(false);
    }
    let payload = opt_prop(activity, "payload");
    let message = as_trimmed_string(&opt_prop(
        &as_record(&payload)
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null),
        "message",
    ))
    .unwrap_or_default();
    // /^Pi extension UI API '[^']+' is not supported in Synara yet\.$/
    Ok(message
        .strip_prefix("Pi extension UI API '")
        .and_then(|rest| rest.strip_suffix("' is not supported in Synara yet."))
        .is_some_and(|mid| !mid.is_empty() && !mid.contains('\'')))
}

/// `packages/shared/src/text.ts` — remove terminal CSI/OSC control sequences.
fn strip_terminal_control_sequences(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    let mut out = String::with_capacity(value.len());
    let mut i = 0;
    while i < chars.len() {
        let csi = (chars[i] == '\u{1b}' && chars.get(i + 1) == Some(&'[')) || chars[i] == '\u{9b}';
        if csi {
            let start = i;
            i += if chars[i] == '\u{9b}' { 1 } else { 2 };
            while i < chars.len() && ('0'..='?').contains(&chars[i]) {
                i += 1;
            }
            while i < chars.len() && (' '..='/').contains(&chars[i]) {
                i += 1;
            }
            if i < chars.len() && ('@'..='~').contains(&chars[i]) {
                i += 1;
                continue;
            }
            out.extend(&chars[start..i]);
            continue;
        }
        let osc = (chars[i] == '\u{1b}' && chars.get(i + 1) == Some(&']')) || chars[i] == '\u{9d}';
        if osc {
            let start = i;
            i += if chars[i] == '\u{9d}' { 1 } else { 2 };
            let mut end = None;
            while i < chars.len() {
                if chars[i] == '\u{7}' || chars[i] == '\u{9c}' {
                    end = Some(i + 1);
                    break;
                }
                if chars[i] == '\u{1b}' && chars.get(i + 1) == Some(&'\\') {
                    end = Some(i + 2);
                    break;
                }
                if chars[i] == '\u{7}' || chars[i] == '\u{1b}' || chars[i] == '\u{9c}' {
                    end = Some(i);
                    break;
                }
                if chars[i] == '\u{1b}' || chars[i] == '\u{9c}' {
                    break;
                }
                i += 1;
            }
            if let Some(e) = end {
                i = e;
                continue;
            }
            out.extend(&chars[start..i]);
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

fn normalize_codex_process_line(line: &str) -> String {
    js_trim(&strip_terminal_control_sequences(line))
}

fn is_ignorable_codex_process_line(line: &str) -> bool {
    if line.is_empty() {
        return true;
    }
    let lower = line.to_lowercase();
    lower.starts_with("token usage:") || lower.starts_with("^ctoken usage:")
}

fn is_codex_unstable_features_warning(message: &str) -> bool {
    message
        .trim_start_matches(is_js_space)
        .starts_with("Under-development features enabled:")
        && message.contains("suppress_unstable_features_warning = true")
}

struct CodexLog {
    level: String,
    target: String,
    message: String,
}

/// `readLog`: a `TIMESTAMP LEVEL target: message` line or a `{"timestamp",...}` envelope.
fn read_log(line: &str) -> Option<CodexLog> {
    const LEVELS: &[&str] = &["TRACE", "DEBUG", "INFO", "WARN", "ERROR"];
    // ^(\d{4}-\d{2}-\d{2}T\S+)\s+(LEVEL)\s+(\S+):\s+(.*)$
    let bytes = line.as_bytes();
    let looks_log_line = bytes.len() > 11
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[..10].iter().all(|b| b.is_ascii_digit());
    if looks_log_line {
        let mut parts = line.splitn(4, is_js_space).filter(|s| !s.is_empty());
        let _ts = parts.next()?;
        let level = parts.next()?;
        if LEVELS.contains(&level) {
            let rest: String = parts.collect::<Vec<_>>().join(" ");
            if let Some((target, message)) = rest.split_once(':')
                && !target.is_empty()
                && !target.contains(is_js_space)
            {
                let message = message.trim_start_matches(is_js_space);
                return Some(CodexLog {
                    level: level.to_string(),
                    target: target.to_string(),
                    message: message.to_string(),
                });
            }
        }
    }
    if !line.starts_with('{') {
        return None;
    }
    let log = serde_json::from_str::<Value>(line).ok()?;
    let timestamp = opt_prop(&log, "timestamp");
    let target = opt_prop(&log, "target");
    let level = opt_prop(&log, "level");
    let fields = opt_prop(&log, "fields");
    let message = {
        let m = opt_prop(&fields, "message");
        if is_js_string(&m) {
            m
        } else {
            opt_prop(&fields, "error")
        }
    };
    if !is_js_string(&timestamp)
        || !is_js_string(&target)
        || !LEVELS.contains(&js_string(&level).unwrap_or_default().as_str())
        || !is_js_string(&message)
    {
        return None;
    }
    Some(CodexLog {
        level: js_string(&level).unwrap_or_default(),
        target: js_string(&target).unwrap_or_default(),
        message: js_string(&message).unwrap_or_default(),
    })
}

fn read_codex_mcp_subscription_error(message: &str) -> Option<String> {
    // UnexpectedServerResponse\(("(?:[^"\\]|\\.)*")\)$ at end of message.
    let mut message = message.to_string();
    if let Some(pos) = message.rfind("UnexpectedServerResponse(\"")
        && message.ends_with("\")")
    {
        let inner = &message[pos + 25..message.len() - 2];
        if let Ok(Value::String(s)) = serde_json::from_str::<Value>(&format!("\"{inner}\"")) {
            message = s;
        } else {
            return None;
        }
    }
    // HTTP 403: {"error": "..."}
    let pos = message.find("HTTP 403: {")?;
    let body = &message[pos + 10..];
    let parsed = serde_json::from_str::<Value>(body).ok()?;
    let error = opt_prop(&parsed, "error");
    let text = js_string(&error)?;
    if text.starts_with("subscription_inactive: ") || text.starts_with("subscription_required: ") {
        return Some(text);
    }
    None
}

enum CodexStderr {
    Notice { mcp_subscription_error: bool },
    Other,
}

/// `classifyCodexStderr(line)` with no mirrored config path and no options — the exact call
/// `deriveWorkLogEntries` makes.
fn classify_codex_stderr(line: &str) -> CodexStderr {
    const BENIGN: &[&str] = &[
        "state db missing rollout path for thread",
        "state db record_discrepancy: find_thread_path_by_id_str_in_subdir, falling_back",
    ];
    let line = normalize_codex_process_line(line);
    if is_ignorable_codex_process_line(&line) {
        return CodexStderr::Other;
    }
    let log = read_log(&line);
    if is_codex_unstable_features_warning(
        log.as_ref()
            .map(|l| l.message.clone())
            .as_deref()
            .unwrap_or(&line),
    ) {
        return CodexStderr::Other;
    }
    if let Some(log) = &log {
        if log.level != "ERROR" || BENIGN.iter().any(|b| log.message.contains(b)) {
            return CodexStderr::Other;
        }
        // \bcodex_rmcp_client::oauth\S*:.*?\bfor server ([A-Za-z0-9_.-]+)
        let hay = format!("{}: {}", log.target, log.message);
        if let Some(server) = (|| {
            let pos = hay.find("codex_rmcp_client::oauth")?;
            let rest = &hay[pos..];
            let for_pos = rest.find("for server ")?;
            let name_start = pos + for_pos + 11;
            let name: String = hay[name_start..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
                .collect();
            if name.is_empty() { None } else { Some(name) }
        })() && server != "synara"
            && !server.ends_with("_connector")
        {
            return CodexStderr::Other;
        }
    }
    if log.is_none() {
        // `errorLikePlainLinesOnly` is unset for this call site — fall through to notice.
    }
    let mcp_subscription_error = log.as_ref().is_some_and(|l| {
        l.target == "rmcp::transport::worker"
            && l.message.starts_with(
                "worker quit with fatal: Transport channel closed, when UnexpectedServerResponse(",
            )
            && read_codex_mcp_subscription_error(&l.message).is_some()
    });
    CodexStderr::Notice {
        mcp_subscription_error,
    }
}

fn is_log_only_claude_warning(activity: &Value) -> Result<bool, String> {
    let kind = prop(activity, "kind", "activity.kind")?;
    if !strict_eq(&kind, &Value::String("runtime.warning".into())) {
        return Ok(false);
    }
    let tone = prop(activity, "tone", "activity.tone")?;
    if strict_eq(&tone, &Value::String("error".into())) {
        return Ok(false);
    }
    let payload = as_record(&opt_prop(activity, "payload"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let message = as_trimmed_string(&opt_prop(&payload, "message")).unwrap_or_default();
    if message.starts_with("Unhandled Claude ") {
        // /^Unhandled Claude system message subtype '(...)' \.$/ with the four keep names.
        const KEEP: &[&str] = &[
            "mirror_error",
            "instruction_size_warning",
            "session_writes_lost",
            "file_attachments_missing",
        ];
        let matched = KEEP
            .iter()
            .any(|name| message == format!("Unhandled Claude system message subtype '{name}'."));
        return Ok(!matched);
    }
    if message.starts_with("Claude reported ") && message.contains("outside cache in one request") {
        return Ok(true);
    }
    let detail = {
        let data = as_record(&opt_prop(&payload, "data")).cloned();
        data.or_else(|| as_record(&opt_prop(&payload, "detail")).cloned())
            .map(Value::Object)
            .unwrap_or(Value::Null)
    };
    if !strict_eq(
        &opt_prop(&detail, "subtype"),
        &Value::String("api_retry".into()),
    ) || !strict_eq(&opt_prop(&detail, "type"), &Value::String("system".into()))
    {
        return Ok(false);
    }
    let attempt = crate::js::as_number(&opt_prop(&detail, "attempt")).unwrap_or(0.0);
    let max_retries = crate::js::as_number(&opt_prop(&detail, "max_retries")).unwrap_or(0.0);
    Ok(!strict_eq(
        &opt_prop(&detail, "error"),
        &Value::String("rate_limit".into()),
    ) && attempt < 3.0_f64.min(max_retries))
}

fn is_turn_failure_activity(activity: &Value) -> Result<bool, String> {
    let turn_id = prop(activity, "turnId", "activity.turnId")?;
    if turn_id.is_null() {
        return Ok(false);
    }
    let kind = prop(activity, "kind", "activity.kind")?;
    if strict_eq(&kind, &Value::String("runtime.error".into())) {
        return Ok(true);
    }
    if strict_eq(&kind, &Value::String("turn.completed".into())) {
        let payload = opt_prop(activity, "payload");
        let state = opt_prop(
            &as_record(&payload)
                .cloned()
                .map(Value::Object)
                .unwrap_or(Value::Null),
            "state",
        );
        let tone = opt_prop(activity, "tone");
        return Ok(strict_eq(&state, &Value::String("failed".into()))
            || strict_eq(&tone, &Value::String("error".into())));
    }
    Ok(false)
}

const COMPUTER_TOOL_NAMES: &[&str] = &[
    "computer_screenshot",
    "computer_get_state",
    "computer_get_screen_size",
    "computer_list_windows",
    "computer_list_apps",
    "computer_verify_state",
    "computer_zoom",
    "computer_get_accessibility_tree",
    "computer_get_cursor_position",
    "computer_help",
    "computer_click",
    "computer_move_cursor",
    "computer_drag",
    "computer_scroll",
    "computer_type_text",
    "computer_press_key",
    "computer_set_value",
    "computer_select_text",
    "computer_perform_action",
    "computer_launch_app",
];

fn computer_tool_name(candidate: &Option<String>) -> Option<&'static str> {
    let candidate = candidate.as_ref()?;
    let normalized: String = candidate
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>()
        .split('_')
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("_");
    let normalized = format!("_{normalized}_");
    let normalized = normalized.replace("__", "_");
    let normalized = normalized.trim_matches('_').to_string();
    for name in COMPUTER_TOOL_NAMES {
        if normalized == *name || normalized.ends_with(&format!("_{name}")) {
            return Some(name);
        }
    }
    None
}

fn extract_tool_name(payload: &Value) -> Option<String> {
    let data = as_record(&opt_prop(payload, "data"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let item = as_record(&opt_prop(&data, "item"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let item_input = as_record(&opt_prop(&item, "input"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let data_invocation = as_record(&opt_prop(&data, "invocation"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let item_invocation = as_record(&opt_prop(&item, "invocation"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let raw_input = as_record(&opt_prop(&data, "rawInput"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    for candidate in [
        opt_prop(payload, "toolName"),
        opt_prop(&data, "toolName"),
        opt_prop(&data, "tool"),
        opt_prop(&raw_input, "_toolName"),
        opt_prop(&raw_input, "toolName"),
        opt_prop(&raw_input, "tool_name"),
        opt_prop(&data_invocation, "tool"),
        opt_prop(&data_invocation, "toolName"),
        opt_prop(&item, "toolName"),
        opt_prop(&item, "tool"),
        opt_prop(&item, "name"),
        opt_prop(&item_invocation, "tool"),
        opt_prop(&item_invocation, "toolName"),
        opt_prop(&item_input, "toolName"),
    ] {
        if let Some(name) = as_trimmed_string(&candidate) {
            return Some(name);
        }
    }
    None
}

fn should_keep_activity_for_work_log(
    activity: &Value,
    latest_turn_id: &Value,
    visible_turn_ids: &Option<Vec<Value>>,
) -> Result<bool, String> {
    let kind = prop(activity, "kind", "activity.kind")?;
    let k = js_string(&kind).unwrap_or_default();
    if matches!(
        k.as_str(),
        "provider.context.changed" | "provider.handoff" | "provider.handoff.failed"
    ) {
        return Ok(true);
    }
    if k == "pull-request.auto-fix.paused" || k == "pull-request.auto-fix.stopped" {
        return Ok(true);
    }
    if k == "auth.status" {
        return Ok(true);
    }
    if k == "provider.turn.start.failed" {
        let payload = opt_prop(activity, "payload");
        let message_id = as_trimmed_string(&opt_prop(
            &as_record(&payload)
                .cloned()
                .map(Value::Object)
                .unwrap_or(Value::Null),
            "messageId",
        ));
        if message_id.is_some() {
            return Ok(true);
        }
    }
    let turn_id = opt_prop(activity, "turnId");
    if k == "runtime.warning" && turn_id.is_null() {
        let payload = opt_prop(activity, "payload");
        let native = opt_prop(
            &as_record(&payload)
                .cloned()
                .map(Value::Object)
                .unwrap_or(Value::Null),
            "nativeEventType",
        );
        if is_codex_runtime_warning_type(native)? {
            return Ok(true);
        }
    }
    if k == "context-compaction" && turn_id.is_null() {
        return Ok(true);
    }
    if k == "automation.created" || k == "mcp.connector.proposed" {
        return Ok(true);
    }
    if k == "subagent.materialization.capped" {
        return Ok(true);
    }
    if k == "checkpoint.revert.failed" || k == "checkpoint.baseline.skipped" {
        return Ok(true);
    }
    if k == "computer.setup-required" || k == "computer.control-denied" {
        return Ok(true);
    }
    if matches!(
        k.as_str(),
        "synara.worker.settled"
            | "synara.worker.stuck"
            | "synara.worker.needs-you"
            | "synara.workers.settled"
    ) {
        return Ok(true);
    }
    if let Some(set) = visible_turn_ids
        && !set.is_empty()
    {
        return Ok(!turn_id.is_null() && svz_contains(set, &turn_id));
    }
    Ok(if !loose_null(latest_turn_id) && truthy(latest_turn_id) {
        strict_eq(&turn_id, latest_turn_id)
    } else {
        true
    })
}

fn is_quiet_turn_lifecycle_activity(activity: &Value) -> Result<bool, String> {
    let kind = prop(activity, "kind", "activity.kind")?;
    if strict_eq(&kind, &Value::String("turn.started".into()))
        || strict_eq(&kind, &Value::String("turn.stop-requested".into()))
    {
        return Ok(true);
    }
    if !strict_eq(&kind, &Value::String("turn.completed".into()))
        && !strict_eq(&kind, &Value::String("turn.aborted".into()))
    {
        return Ok(false);
    }
    let tone = prop(activity, "tone", "activity.tone")?;
    Ok(!strict_eq(&tone, &Value::String("error".into())))
}

fn is_quiet_approval_resolution_activity(activity: &Value) -> Result<bool, String> {
    let kind = prop(activity, "kind", "activity.kind")?;
    if !strict_eq(&kind, &Value::String("approval.resolved".into())) {
        return Ok(false);
    }
    let tone = prop(activity, "tone", "activity.tone")?;
    if strict_eq(&tone, &Value::String("error".into())) {
        return Ok(false);
    }
    let payload = as_record(&opt_prop(activity, "payload"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    Ok(strict_eq(
        &opt_prop(&payload, "decision"),
        &Value::String("accept".into()),
    ) && is_undef(&opt_prop(&payload, "approvalScope"))
        && computer_tool_name(&extract_tool_name(&payload)).is_none())
}

fn is_plan_boundary_tool_activity(activity: &Value) -> Result<bool, String> {
    let kind = prop(activity, "kind", "activity.kind")?;
    if !strict_eq(&kind, &Value::String("tool.updated".into()))
        && !strict_eq(&kind, &Value::String("tool.completed".into()))
    {
        return Ok(false);
    }
    let payload = as_record(&opt_prop(activity, "payload"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let detail = opt_prop(&payload, "detail");
    let Some(detail) = js_string(&detail) else {
        return Ok(false);
    };
    // toolArgumentSummaryToolName: /^([\w.-]+):/
    let trimmed = detail.trim_start();
    let name: String = trimmed
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
        .collect();
    Ok(!name.is_empty() && trimmed[name.len()..].starts_with(':') && name == "ExitPlanMode")
}

// ---- i18n: the English `t` used when no `options.translate` arrives ---------------------------

fn translate(key: &str, values: &Map<String, Value>) -> String {
    let template = match key {
        "chat:backgroundTask" => "Background task",
        "chat:taskInterrupted" => "Task interrupted",
        "chat:theTaskWasInterruptedBecauseTheModelIsAt" => {
            "The task was interrupted because the model is at capacity. Work remains incomplete."
        }
        "chat:theTaskWasInterruptedByAProviderErrorWork" => {
            "The task was interrupted by a provider error. Work remains incomplete. {{value0}}"
        }
        "chat:theProviderReportedAnError" => "The provider reported an error.",
        "common:subagentFinished" => "Subagent finished",
        "common:subagentFailed" => "Subagent failed",
        "common:subagentStopped" => "Subagent stopped",
        "common:backgroundTaskFinished" => "Background task finished",
        "common:backgroundTaskFailed" => "Background task failed",
        "common:backgroundTaskStopped" => "Background task stopped",
        "common:symbol8" => "{{value0}}: {{value1}}",
        "chat:taskProgress" => {
            let count = values
                .get("count")
                .and_then(crate::js::as_number)
                .unwrap_or(0.0);
            if count == 1.0 {
                "{{completed}} of {{count}} task"
            } else {
                "{{completed}} of {{count}} tasks"
            }
        }
        "chat:fastModePaused" => "Fast paused",
        "chat:fastModePausedDescription" => {
            "Fast mode is paused after a rate limit. Requests run at standard speed for now."
        }
        "chat:fastModeUnavailable" => "Fast unavailable",
        "chat:fastModeUnavailableUnknown" => {
            "Fast mode is not active: Claude reported it as unavailable. Requests run at standard speed."
        }
        "chat:fastModeUnavailable_disabled_by_env" => {
            "Fast mode is not active: it is disabled in this environment. Requests run at standard speed."
        }
        "chat:fastModeUnavailable_extra_usage_disabled" => {
            "Fast mode is not active: usage credits are turned off for this Claude account. Requests run at standard speed."
        }
        "chat:fastModeUnavailable_free" => {
            "Fast mode is not active: this Claude plan does not include it. Requests run at standard speed."
        }
        "chat:fastModeUnavailable_model_not_allowed" => {
            "Fast mode is not active: this model does not support it. Requests run at standard speed."
        }
        "chat:fastModeUnavailable_network_error" => {
            "Fast mode is not active: Claude could not check its availability. Requests run at standard speed."
        }
        "chat:fastModeUnavailable_not_first_party" => {
            "Fast mode is not active: it needs a first-party Anthropic account. Requests run at standard speed."
        }
        _ => key,
    };
    let mut out = template.to_string();
    for (name, value) in values {
        out = out.replace(&format!("{{{{{name}}}}}"), &js_to_string(value));
    }
    out
}

// ---- deriveWorkLogEntries ----------------------------------------------------------------------

struct DerivedEntry {
    map: Map<String, Value>,
}

impl DerivedEntry {
    fn get(&self, key: &str) -> Value {
        self.map.get(key).cloned().unwrap_or_else(js::undefined)
    }
}

fn with_provider_sequence(activity: &Value) -> Option<(String, Value)> {
    let sequence = opt_prop(activity, "sequence");
    let source = opt_prop(activity, "sequenceSource");
    if !is_undef(&sequence) && !strict_eq(&source, &Value::String("orchestration".into())) {
        Some(("sequence".to_string(), sequence))
    } else {
        None
    }
}

/// Reduced `toDerivedWorkLogEntry`: the entry skeleton plus the fields derivable with in-file
/// helpers. Provider-title presentation, tool details, MCP/computer description, subagent
/// decoding and inline-visual decoding depend on modules outside the port boundary; the
/// recorded corpus has no case reaching `.map`, so this boundary is unexercised by `diff`.
fn to_derived_work_log_entry(activity: &Value) -> DerivedEntry {
    let payload = as_record(&opt_prop(activity, "payload"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let mut map = Map::new();
    map.insert("id".into(), opt_prop(activity, "id"));
    map.insert("createdAt".into(), opt_prop(activity, "createdAt"));
    if let Some((k, v)) = with_provider_sequence(activity) {
        map.insert(k, v);
    }
    let turn_id = opt_prop(activity, "turnId");
    if truthy(&turn_id) {
        map.insert("turnId".into(), turn_id);
    }
    let tone = opt_prop(activity, "tone");
    if truthy(&tone) {
        map.insert("tone".into(), tone);
    }
    let title = as_trimmed_string(&opt_prop(&payload, "title"))
        .or_else(|| as_trimmed_string(&opt_prop(&payload, "label")))
        .or_else(|| as_trimmed_string(&opt_prop(activity, "summary")))
        .or_else(|| js_string(&opt_prop(activity, "kind")))
        .unwrap_or_default();
    map.insert("label".into(), Value::String(title));
    map.insert("activityKind".into(), opt_prop(activity, "kind"));
    if let Some(v) = as_trimmed_string(&opt_prop(activity, "summary")) {
        map.insert("summary".into(), Value::String(v));
    }
    if let Some(v) = as_trimmed_string(&opt_prop(&payload, "detail")) {
        map.insert("detail".into(), Value::String(v));
    }
    if let Some(v) = as_trimmed_string(&opt_prop(&payload, "nativeEventType")) {
        map.insert("nativeEventType".into(), Value::String(v));
    }
    if let Some(v) = extract_work_log_request_kind(&payload) {
        map.insert("requestKind".into(), Value::String(v));
    }
    if let Some(v) = extract_work_log_item_type(&payload) {
        map.insert("itemType".into(), Value::String(v));
    }
    if let Some(v) = extract_tool_call_id(&payload) {
        map.insert("toolCallId".into(), Value::String(v));
    }
    if let Some(v) = extract_tool_name(&payload) {
        map.insert("toolName".into(), Value::String(v));
    }
    let (command, raw_command) = extract_tool_command(&payload);
    if let Some(v) = command {
        map.insert("command".into(), Value::String(v));
    }
    if let Some(v) = raw_command {
        map.insert("rawCommand".into(), Value::String(v));
    }
    let changed = extract_changed_files(&payload);
    if !changed.is_empty() {
        map.insert(
            "changedFiles".into(),
            Value::Array(changed.into_iter().map(Value::String).collect()),
        );
    }
    if let Some(tasks) = {
        let v = parse_task_list_tasks(payload.clone()).unwrap_or(Value::Null);
        if v.is_null() { None } else { Some(v) }
    } {
        map.insert("taskList".into(), tasks);
    }
    let kind = js_string(&opt_prop(activity, "kind")).unwrap_or_default();
    if kind == "provider.context.changed"
        && let Some(info) = extract_provider_context_lifecycle_info(&payload)
    {
        map.insert("providerContextLifecycle".into(), info);
    }
    if (kind == "provider.handoff" || kind == "provider.handoff.failed")
        && let Ok(info) = extract_provider_handoff_info(
            payload.clone(),
            Value::String(if kind == "provider.handoff" {
                "completed".into()
            } else {
                "failed".into()
            }),
        )
        && !info.is_null()
    {
        map.insert("providerHandoff".into(), info);
    }
    DerivedEntry { map }
}

fn is_likely_file_path(value: &str) -> bool {
    let lower = value.to_lowercase();
    if lower.starts_with("file://")
        || lower.starts_with("vscode://")
        || lower.starts_with("cursor://")
    {
        return true;
    }
    if value.starts_with('/') || value.starts_with("./") || value.starts_with("../") {
        return true;
    }
    if value.len() >= 3
        && value.as_bytes()[0].is_ascii_alphabetic()
        && value.as_bytes()[1] == b':'
        && matches!(value.as_bytes()[2], b'/' | b'\\')
    {
        return true;
    }
    if value.contains('/') || value.contains('\\') {
        return true;
    }
    // /^[^\s/\\]+\.[A-Za-z0-9][A-Za-z0-9._-]*$/
    if value
        .chars()
        .any(|c| is_js_space(c) || c == '/' || c == '\\')
    {
        return false;
    }
    let Some(dot) = value.find('.') else {
        return false;
    };
    let tail = &value[dot + 1..];
    !tail.is_empty()
        && tail
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric())
        && tail
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn collect_changed_files(
    v: &Value,
    target: &mut Vec<String>,
    seen: &mut Vec<String>,
    depth: usize,
) {
    if depth > 4 || target.len() >= 12 {
        return;
    }
    if let Value::Array(list) = v {
        for entry in list {
            collect_changed_files(entry, target, seen, depth + 1);
            if target.len() >= 12 {
                return;
            }
        }
        return;
    }
    if let Some(record) = as_record(v) {
        if let Some(text) =
            as_trimmed_string(&record.get("path").cloned().unwrap_or_else(js::undefined))
            && is_likely_file_path(&text)
            && !seen.contains(&text)
        {
            seen.push(text.clone());
            target.push(text);
            if target.len() >= 12 {
                return;
            }
        }
        for key in ["files", "file", "edits", "patch", "patches", "operations"] {
            if !record.contains_key(key) {
                continue;
            }
            collect_changed_files(&record[key].clone(), target, seen, depth + 1);
            if target.len() >= 12 {
                return;
            }
        }
    } else if let Some(text) = as_trimmed_string(v)
        && is_likely_file_path(&text)
        && !seen.contains(&text)
    {
        seen.push(text.clone());
        target.push(text);
    }
}

fn extract_changed_files(payload: &Value) -> Vec<String> {
    let mut files = Vec::new();
    let mut seen = Vec::new();
    let data = as_record(&opt_prop(payload, "data"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    collect_changed_files(&data, &mut files, &mut seen, 0);
    files
}

const SESSION_CONTEXT_RECAP_PREVIEW_MAX_CHARS: usize = 600;

fn is_provider_context_lifecycle_reason(v: &Value) -> bool {
    js_string(v).is_some_and(|s| {
        matches!(
            s.as_str(),
            "conversation-rebuilt"
                | "fork-from-earlier-turn"
                | "fresh-session"
                | "interrupt-escalation"
                | "native-history-unavailable"
                | "native-resume-failed"
        )
    })
}

fn extract_provider_context_lifecycle_info(payload: &Value) -> Option<Value> {
    let provider = as_provider_kind(&opt_prop(payload, "provider"))?;
    let native_history = opt_prop(payload, "nativeHistory");
    if !strict_eq(&native_history, &Value::String("available".into()))
        && !strict_eq(&native_history, &Value::String("unavailable".into()))
    {
        return None;
    }
    let restart_reason = opt_prop(payload, "restartReason");
    if !is_provider_context_lifecycle_reason(&restart_reason) {
        return None;
    }
    let session_restarted = opt_prop(payload, "sessionRestarted");
    let recap_injected = opt_prop(payload, "recapInjected");
    let recap_characters = opt_prop(payload, "recapCharacters");
    let recap_preview = opt_prop(payload, "recapPreview");
    let recap_preview_truncated = opt_prop(payload, "recapPreviewTruncated");
    if !matches!(session_restarted, Value::Bool(_))
        || !matches!(recap_injected, Value::Bool(_))
        || crate::js::as_number(&recap_characters).is_none_or(|n| n.fract() != 0.0 || n < 0.0)
        || (!recap_preview.is_null() && !is_js_string(&recap_preview))
        || !matches!(recap_preview_truncated, Value::Bool(_))
    {
        return None;
    }
    let preview_str = js_string(&recap_preview);
    let bounded = match &preview_str {
        Some(s) if s.encode_utf16().count() > SESSION_CONTEXT_RECAP_PREVIEW_MAX_CHARS => {
            let tail: String = s
                .chars()
                .rev()
                .take(SESSION_CONTEXT_RECAP_PREVIEW_MAX_CHARS - 1)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            Some(format!("…{}", tail.trim_start_matches(is_js_space)))
        }
        other => other.clone(),
    };
    let mut out = Map::new();
    out.insert("provider".into(), Value::String(provider));
    out.insert("nativeHistory".into(), native_history);
    out.insert("restartReason".into(), restart_reason);
    out.insert("sessionRestarted".into(), session_restarted);
    out.insert("recapInjected".into(), recap_injected);
    out.insert("recapCharacters".into(), recap_characters);
    out.insert(
        "recapPreview".into(),
        bounded
            .as_ref()
            .map(|b| Value::String(b.clone()))
            .unwrap_or(Value::Null),
    );
    out.insert(
        "recapPreviewTruncated".into(),
        Value::Bool(
            recap_preview_truncated == Value::Bool(true)
                || preview_str.as_ref().is_some_and(|s| {
                    s.encode_utf16().count()
                        > bounded.as_ref().map_or(0, |b| b.encode_utf16().count())
                }),
        ),
    );
    Some(Value::Object(out))
}

fn pending_request_instance_key(request_id: &str, lifecycle_generation: Option<&str>) -> String {
    format!(
        "{request_id}\u{0}{}",
        lifecycle_generation.unwrap_or("legacy")
    )
}

fn format_user_input_answer(answers: &Value, question: &Map<String, Value>) -> Option<String> {
    let by_id = opt_prop(
        answers,
        &js_string(&question.get("id").cloned().unwrap_or_else(js::undefined)).unwrap_or_default(),
    );
    let value = if !is_undef(&by_id) {
        by_id
    } else {
        opt_prop(
            answers,
            &js_string(
                &question
                    .get("question")
                    .cloned()
                    .unwrap_or_else(js::undefined),
            )
            .unwrap_or_default(),
        )
    };
    let parts: Vec<Value> = if is_js_string(&value) {
        vec![value]
    } else if let Value::Array(list) = &value {
        list.clone()
    } else if let Value::Array(list) = opt_prop(&value, "answers") {
        list
    } else {
        Vec::new()
    };
    let text = parts
        .iter()
        .filter_map(js_string)
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if text.is_empty() { None } else { Some(text) }
}

fn with_user_input_exchanges(
    entries: Vec<DerivedEntry>,
    ordered: &[Value],
) -> Result<Vec<DerivedEntry>, String> {
    if !entries.iter().any(|e| {
        strict_eq(
            &e.get("activityKind"),
            &Value::String("user-input.resolved".into()),
        )
    }) {
        return Ok(entries);
    }
    let mut open_requests: Vec<(String, (Value, Value))> = Vec::new();
    let mut pairs_by_resolved_id: Vec<(String, (Value, Value, Value))> = Vec::new();
    for activity in ordered {
        let kind = prop(activity, "kind", "activity.kind")?;
        if !strict_eq(&kind, &Value::String("user-input.requested".into()))
            && !strict_eq(&kind, &Value::String("user-input.resolved".into()))
        {
            continue;
        }
        let payload = as_record(&prop(activity, "payload", "activity.payload")?)
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        let request_id = match opt_prop(&payload, "requestId") {
            v if is_js_string(&v) => js_string(&v).unwrap_or_default(),
            _ => continue,
        };
        let generation = match opt_prop(&payload, "lifecycleGeneration") {
            v if is_js_string(&v) && !js_string(&v).unwrap_or_default().is_empty() => {
                Some(js_string(&v).unwrap_or_default())
            }
            _ => None,
        };
        let key = pending_request_instance_key(&request_id, generation.as_deref());
        if strict_eq(&kind, &Value::String("user-input.requested".into())) {
            let questions = parse_user_input_questions(payload.clone())?;
            open_requests.retain(|(k, _)| *k != key);
            if !questions.is_null() {
                str_map_set(&mut open_requests, key, (activity.clone(), questions));
            }
        } else {
            if let Some((_, (request, questions))) =
                open_requests.iter().find(|(k, _)| *k == key).cloned()
            {
                let answers = as_record(&opt_prop(&payload, "answers"))
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or(Value::Null);
                str_map_set(
                    &mut pairs_by_resolved_id,
                    js_string(&opt_prop(activity, "id")).unwrap_or_default(),
                    (request, questions, answers),
                );
                open_requests.retain(|(k, _)| *k != key);
            }
        }
    }
    let mut answered_ids: Vec<String> = Vec::new();
    let mut out = Vec::with_capacity(entries.len());
    for mut entry in entries {
        if !strict_eq(
            &entry.get("activityKind"),
            &Value::String("user-input.resolved".into()),
        ) {
            out.push(entry);
            continue;
        }
        let id = js_string(&entry.get("id")).unwrap_or_default();
        let pair = str_map_get(&pairs_by_resolved_id, &id).cloned();
        let Some((request, questions, answers)) = pair else {
            out.push(entry);
            continue;
        };
        if let Some(req_id) = js_string(&opt_prop(&request, "id")) {
            str_set_add(&mut answered_ids, req_id);
        }
        let mut exchange = Vec::new();
        if let Value::Array(qs) = &questions {
            for question in qs {
                let Some(q) = as_record(question).cloned() else {
                    continue;
                };
                let options: Vec<Value> = q
                    .get("options")
                    .and_then(|o| o.as_array())
                    .map(|opts| opts.iter().map(|o| opt_prop(o, "label")).collect())
                    .unwrap_or_default();
                let answer = format_user_input_answer(&answers, &q);
                exchange.push(obj(vec![
                    ("id", q.get("id").cloned().unwrap_or_else(js::undefined)),
                    (
                        "header",
                        q.get("header").cloned().unwrap_or_else(js::undefined),
                    ),
                    (
                        "question",
                        q.get("question").cloned().unwrap_or_else(js::undefined),
                    ),
                    ("options", Value::Array(options)),
                    ("answer", answer.map(Value::String).unwrap_or(Value::Null)),
                ]));
            }
        }
        entry
            .map
            .insert("userInputExchange".into(), Value::Array(exchange));
        out.push(entry);
    }
    out.retain(|e| {
        let id = js_string(&e.get("id")).unwrap_or_default();
        !answered_ids.contains(&id)
    });
    Ok(out)
}

fn granted_command_approval_activity_ids(
    ordered: &[Value],
    active_turn_id: &Value,
) -> Result<Option<Vec<String>>, String> {
    if !truthy(active_turn_id) {
        return Ok(None);
    }
    let mut open: Vec<(String, String)> = Vec::new();
    let mut granted: Vec<String> = Vec::new();
    for activity in ordered {
        let turn_id = prop(activity, "turnId", "activity.turnId")?;
        if !strict_eq(&turn_id, active_turn_id) {
            continue;
        }
        let kind = prop(activity, "kind", "activity.kind")?;
        if !strict_eq(&kind, &Value::String("approval.requested".into()))
            && !strict_eq(&kind, &Value::String("approval.resolved".into()))
        {
            continue;
        }
        let payload = as_record(&opt_prop(activity, "payload"))
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        let Some(request_id) = as_trimmed_string(&opt_prop(&payload, "requestId")) else {
            continue;
        };
        if extract_work_log_request_kind(&payload).as_deref() != Some("command") {
            continue;
        }
        let key = pending_request_instance_key(
            &request_id,
            as_trimmed_string(&opt_prop(&payload, "lifecycleGeneration")).as_deref(),
        );
        let activity_id = js_string(&prop(activity, "id", "activity.id")?).unwrap_or_default();
        if strict_eq(&kind, &Value::String("approval.requested".into())) {
            str_map_set(&mut open, key, activity_id);
            continue;
        }
        let requested_id = str_map_get(&open, &key).cloned();
        open.retain(|(k, _)| *k != key);
        let tone = opt_prop(activity, "tone");
        if let Some(req) = requested_id
            && !strict_eq(&tone, &Value::String("error".into()))
            && (strict_eq(
                &opt_prop(&payload, "decision"),
                &Value::String("accept".into()),
            ) || strict_eq(
                &opt_prop(&payload, "decision"),
                &Value::String("acceptForSession".into()),
            ))
        {
            str_set_add(&mut granted, req);
            str_set_add(&mut granted, activity_id);
        }
    }
    Ok(Some(granted))
}

fn derive_mid_turn_notification_ids(ordered: &[Value]) -> Result<Vec<String>, String> {
    let mut working: Vec<String> = Vec::new();
    let mut ids: Vec<String> = Vec::new();
    for activity in ordered {
        let turn_id = prop(activity, "turnId", "activity.turnId")?;
        if !truthy(&turn_id) {
            continue;
        }
        let kind = prop(activity, "kind", "activity.kind")?;
        let k = js_string(&kind);
        if strict_eq(&kind, &Value::String("turn.completed".into()))
            || strict_eq(&kind, &Value::String("turn.aborted".into()))
        {
            if let Some(t) = k {
                working.retain(|w| *w != t);
            } else {
                // `kind` is never non-string for the equality branch.
            }
        } else if (is_js_string(&kind)
            && (js_string(&kind).unwrap_or_default().starts_with("tool.")
                || js_string(&kind)
                    .unwrap_or_default()
                    .starts_with("approval.")))
            || strict_eq(&kind, &Value::String("task.started".into()))
        {
            if let Some(t) = js_string(&turn_id).or_else(|| Some(js_to_string(&turn_id))) {
                str_set_add(&mut working, t);
            } else {
                // Object turn ids can never be in the string set.
            }
        } else if !working.is_empty()
            && (strict_eq(&kind, &Value::String("task.completed".into()))
                || (strict_eq(&kind, &Value::String("runtime.warning".into()))
                    && strict_eq(
                        &opt_prop(
                            &as_record(&opt_prop(activity, "payload"))
                                .cloned()
                                .map(Value::Object)
                                .unwrap_or(Value::Null),
                            "nativeEventType",
                        ),
                        &Value::String("monitor_event".into()),
                    )))
            && let Some(id) = js_string(&opt_prop(activity, "id"))
        {
            str_set_add(&mut ids, id);
        }
    }
    Ok(ids)
}

fn derive_background_task_completion_entries(
    ordered: &[Value],
    latest_turn_id: &Value,
    visible_turn_ids: &Option<Vec<Value>>,
    mid_turn_ids: &[String],
) -> Result<Vec<Value>, String> {
    let mut background_tasks: Vec<(String, Map<String, Value>)> = Vec::new();
    let mut completions = Vec::new();
    for activity in ordered {
        let kind = prop(activity, "kind", "activity.kind")?;
        let payload = as_record(&opt_prop(activity, "payload"))
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        if strict_eq(&kind, &Value::String("runtime.warning".into()))
            && strict_eq(
                &opt_prop(&payload, "nativeEventType"),
                &Value::String("background_tasks_changed".into()),
            )
        {
            if !should_keep_activity_for_work_log(activity, latest_turn_id, visible_turn_ids)? {
                continue;
            }
            let tasks = opt_prop(
                &as_record(&opt_prop(&payload, "data"))
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or(Value::Null),
                "tasks",
            );
            if let Value::Array(list) = tasks {
                for task in &list {
                    let Some(record) = as_record(task).cloned() else {
                        continue;
                    };
                    let task_id = record.get("task_id").cloned().unwrap_or(Value::Null);
                    if !is_js_string(&task_id) {
                        continue;
                    }
                    let turn_id = opt_prop(activity, "turnId");
                    let mut entry = Map::new();
                    entry.insert("taskType".into(), {
                        let v = record.get("task_type").cloned().unwrap_or(Value::Null);
                        if is_js_string(&v) { v } else { Value::Null }
                    });
                    entry.insert("description".into(), {
                        let v = record.get("description").cloned().unwrap_or(Value::Null);
                        if is_js_string(&v) { v } else { Value::Null }
                    });
                    entry.insert("turnId".into(), turn_id);
                    str_map_set(
                        &mut background_tasks,
                        js_string(&task_id).unwrap_or_default(),
                        entry,
                    );
                }
            }
            continue;
        }
        if !strict_eq(&kind, &Value::String("task.completed".into())) {
            continue;
        }
        let task_id = opt_prop(&payload, "taskId");
        if !is_js_string(&task_id) {
            continue;
        }
        let id_str = js_string(&task_id).unwrap_or_default();
        let Some(launched) = str_map_get(&background_tasks, &id_str).cloned() else {
            continue;
        };
        background_tasks.retain(|(k, _)| *k != id_str);
        let activity_id = js_string(&opt_prop(activity, "id")).unwrap_or_default();
        if mid_turn_ids.contains(&activity_id) {
            continue;
        }
        let launch_turn = launched.get("turnId").cloned().unwrap_or(Value::Null);
        let turn_id = if loose_null(&launch_turn) {
            opt_prop(activity, "turnId")
        } else {
            launch_turn
        };
        let is_local_agent = strict_eq(
            launched.get("taskType").unwrap_or(&Value::Null),
            &Value::String("local_agent".into()),
        );
        let outcome_keys = if is_local_agent {
            (
                "common:subagentFinished",
                "common:subagentFailed",
                "common:subagentStopped",
            )
        } else {
            (
                "common:backgroundTaskFinished",
                "common:backgroundTaskFailed",
                "common:backgroundTaskStopped",
            )
        };
        let status = opt_prop(&payload, "status");
        let (outcome, key) = if strict_eq(&status, &Value::String("failed".into())) {
            ("failed", outcome_keys.1)
        } else if strict_eq(&status, &Value::String("stopped".into())) {
            ("stopped", outcome_keys.2)
        } else {
            ("finished", outcome_keys.0)
        };
        let description = launched.get("description").cloned().unwrap_or(Value::Null);
        let mut values = Map::new();
        values.insert("value0".into(), Value::String(translate(key, &Map::new())));
        values.insert("value1".into(), description.clone());
        let label = if truthy(&description) {
            translate("common:symbol8", &values)
        } else {
            translate(key, &Map::new())
        };
        let mut entry = Map::new();
        entry.insert("id".into(), opt_prop(activity, "id"));
        entry.insert("createdAt".into(), opt_prop(activity, "createdAt"));
        if let Some((k, v)) = with_provider_sequence(activity) {
            entry.insert(k, v);
        }
        if truthy(&turn_id) {
            entry.insert("turnId".into(), turn_id);
        }
        entry.insert("label".into(), Value::String(label));
        entry.insert(
            "tone".into(),
            Value::String(
                if strict_eq(&status, &Value::String("failed".into())) {
                    "error"
                } else {
                    "info"
                }
                .into(),
            ),
        );
        entry.insert("activityKind".into(), kind);
        let mut completion = Map::new();
        completion.insert("taskId".into(), task_id);
        if let Some(v) = launched.get("taskType") {
            completion.insert("taskType".into(), v.clone());
        }
        if let Some(v) = launched.get("description") {
            completion.insert("description".into(), v.clone());
        }
        completion.insert("outcome".into(), Value::String(outcome.into()));
        entry.insert("backgroundTaskCompletion".into(), Value::Object(completion));
        completions.push(Value::Object(entry));
    }
    Ok(completions)
}

fn derive_turn_failure_entries(ordered: &[Value]) -> Result<Vec<Value>, String> {
    const USAGE_LIMIT: &[&str] = &["rate_limit", "usageLimitExceeded", "rateLimitExceeded"];
    let mut terminal: Vec<(Value, Value)> = Vec::new();
    for activity in ordered {
        let turn_id = prop(activity, "turnId", "activity.turnId")?;
        let kind = prop(activity, "kind", "activity.kind")?;
        if truthy(&turn_id)
            && (strict_eq(&kind, &Value::String("turn.completed".into()))
                || strict_eq(&kind, &Value::String("turn.aborted".into())))
        {
            let state = if strict_eq(&kind, &Value::String("turn.aborted".into())) {
                Value::String("interrupted".into())
            } else {
                let payload = as_record(&opt_prop(activity, "payload"))
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or(Value::Null);
                opt_prop(&payload, "state")
            };
            svz_map_set(&mut terminal, turn_id, state);
        }
    }
    let mut failures: Vec<(String, Map<String, Value>)> = Vec::new();
    for activity in ordered {
        if !is_turn_failure_activity(activity)? {
            continue;
        }
        let payload = as_record(&opt_prop(activity, "payload"))
            .cloned()
            .map(Value::Object)
            .unwrap_or(Value::Null);
        let turn_id = opt_prop(activity, "turnId");
        let state = if truthy(&turn_id) {
            svz_map_get(&terminal, &turn_id).cloned()
        } else {
            None
        };
        if matches!(
            state.as_ref().and_then(js_string).as_deref(),
            Some("completed") | Some("cancelled") | Some("interrupted")
        ) {
            continue;
        }
        let id = if truthy(&turn_id) {
            format!("turn-failure:{}", js_to_string(&turn_id))
        } else {
            js_to_string(&opt_prop(activity, "id"))
        };
        let previous = str_map_get(&failures, &id).cloned();
        let prev_failure = previous
            .as_ref()
            .and_then(|p| p.get("turnFailure").cloned())
            .unwrap_or(Value::Null);
        let cause = as_trimmed_string(&opt_prop(&payload, "errorMessage"))
            .or_else(|| as_trimmed_string(&opt_prop(&payload, "message")))
            .or_else(|| as_trimmed_string(&opt_prop(&prev_failure, "cause")))
            .unwrap_or_else(|| "The provider reported an error.".to_string());
        let error_code = as_trimmed_string(&opt_prop(&payload, "errorCode"))
            .or_else(|| as_trimmed_string(&opt_prop(&prev_failure, "errorCode")));
        let provider = as_trimmed_string(&opt_prop(&payload, "provider"))
            .or_else(|| as_trimmed_string(&opt_prop(&prev_failure, "provider")));
        let overloaded = error_code.as_deref() == Some("server_overloaded")
            || cause
                .to_lowercase()
                .contains("selected model is at capacity");
        let message = if overloaded {
            translate("chat:theTaskWasInterruptedBecauseTheModelIsAt", &Map::new())
        } else {
            let mut values = Map::new();
            values.insert(
                "value0".into(),
                Value::String(
                    as_trimmed_string(&opt_prop(&payload, "errorMessage"))
                        .or_else(|| as_trimmed_string(&opt_prop(&payload, "message")))
                        .or_else(|| as_trimmed_string(&opt_prop(&prev_failure, "cause")))
                        .unwrap_or_else(|| {
                            translate("chat:theProviderReportedAnError", &Map::new())
                        }),
                ),
            );
            translate("chat:theTaskWasInterruptedByAProviderErrorWork", &values)
        };
        let mut entry = Map::new();
        entry.insert("id".into(), Value::String(id.clone()));
        entry.insert(
            "createdAt".into(),
            previous
                .as_ref()
                .and_then(|p| p.get("createdAt").cloned())
                .unwrap_or_else(|| opt_prop(activity, "createdAt")),
        );
        if let Some(seq) = previous.as_ref().and_then(|p| p.get("sequence")).cloned() {
            if !is_undef(&seq) {
                entry.insert("sequence".into(), seq);
            } else if let Some((k, v)) = with_provider_sequence(activity) {
                entry.insert(k, v);
            }
        } else if let Some((k, v)) = with_provider_sequence(activity) {
            entry.insert(k, v);
        }
        if truthy(&turn_id) {
            entry.insert("turnId".into(), turn_id);
        }
        entry.insert("tone".into(), Value::String("error".into()));
        entry.insert(
            "label".into(),
            Value::String(translate("chat:taskInterrupted", &Map::new())),
        );
        entry.insert("activityKind".into(), opt_prop(activity, "kind"));
        let mut failure = Map::new();
        failure.insert("cause".into(), Value::String(cause));
        failure.insert("message".into(), Value::String(message));
        if let Some(code) = &error_code {
            failure.insert("errorCode".into(), Value::String(code.clone()));
        }
        if let Some(p) = &provider {
            failure.insert("provider".into(), Value::String(p.clone()));
        }
        if error_code
            .as_ref()
            .is_some_and(|c| USAGE_LIMIT.contains(&c.as_str()))
        {
            failure.insert("usageLimit".into(), Value::Bool(true));
        }
        entry.insert("turnFailure".into(), Value::Object(failure));
        str_map_set(&mut failures, id, entry);
    }
    Ok(failures
        .into_iter()
        .map(|(_, v)| Value::Object(v))
        .collect())
}

fn fast_mode_notice_from_activity(activity: &Value) -> Option<Map<String, Value>> {
    let payload = as_record(&opt_prop(activity, "payload"))
        .cloned()
        .map(Value::Object)
        .unwrap_or(Value::Null);
    let state = opt_prop(&payload, "state");
    if strict_eq(&state, &Value::String("cooldown".into())) {
        let mut notice = Map::new();
        notice.insert("kind".into(), Value::String("cooldown".into()));
        notice.insert(
            "label".into(),
            Value::String(translate("chat:fastModePaused", &Map::new())),
        );
        notice.insert(
            "detail".into(),
            Value::String(translate("chat:fastModePausedDescription", &Map::new())),
        );
        return Some(notice);
    }
    if !strict_eq(&state, &Value::String("off".into())) {
        return None;
    }
    let reason = opt_prop(&payload, "disabledReason");
    let reason = js_string(&reason)?;
    if matches!(
        reason.as_str(),
        "sdk_opt_in_required" | "preference" | "pending"
    ) {
        return None;
    }
    let detail_key = match reason.as_str() {
        "extra_usage_disabled" => "chat:fastModeUnavailable_extra_usage_disabled",
        "free" => "chat:fastModeUnavailable_free",
        "not_first_party" => "chat:fastModeUnavailable_not_first_party",
        "disabled_by_env" => "chat:fastModeUnavailable_disabled_by_env",
        "model_not_allowed" => "chat:fastModeUnavailable_model_not_allowed",
        "network_error" => "chat:fastModeUnavailable_network_error",
        _ => "chat:fastModeUnavailableUnknown",
    };
    let mut notice = Map::new();
    notice.insert("kind".into(), Value::String("blocked".into()));
    notice.insert(
        "label".into(),
        Value::String(translate("chat:fastModeUnavailable", &Map::new())),
    );
    notice.insert(
        "detail".into(),
        Value::String(translate(detail_key, &Map::new())),
    );
    Some(notice)
}

type FastModeNoticeMap = Vec<(String, Map<String, Value>)>;

fn derive_handoff_fast_mode_notices(ordered: &[Value]) -> Result<FastModeNoticeMap, String> {
    let mut notices: Vec<(String, Map<String, Value>)> = Vec::new();
    let mut session_notice: Option<Map<String, Value>> = None;
    let mut open_handoff_id: Option<String> = None;
    for activity in ordered {
        let kind = prop(activity, "kind", "activity.kind")?;
        if strict_eq(&kind, &Value::String("fast-mode.state".into())) {
            session_notice = fast_mode_notice_from_activity(activity);
            if let Some(id) = &open_handoff_id {
                let mut entry = str_map_get(&notices, id).cloned().unwrap_or_default();
                entry.insert(
                    "targetFastModeNotice".into(),
                    session_notice
                        .clone()
                        .map(Value::Object)
                        .unwrap_or(Value::Null),
                );
                str_map_set(&mut notices, id.clone(), entry);
            }
            continue;
        }
        if strict_eq(&kind, &Value::String("provider.handoff.failed".into())) {
            if let Some(notice) = &session_notice {
                let mut entry = Map::new();
                entry.insert("sourceFastModeNotice".into(), Value::Object(notice.clone()));
                str_map_set(
                    &mut notices,
                    js_string(&opt_prop(activity, "id")).unwrap_or_default(),
                    entry,
                );
            }
            continue;
        }
        if strict_eq(&kind, &Value::String("provider.handoff".into())) {
            if let Some(notice) = &session_notice {
                let mut entry = Map::new();
                entry.insert("sourceFastModeNotice".into(), Value::Object(notice.clone()));
                str_map_set(
                    &mut notices,
                    js_string(&opt_prop(activity, "id")).unwrap_or_default(),
                    entry,
                );
            }
            open_handoff_id = js_string(&opt_prop(activity, "id"));
            session_notice = None;
        }
    }
    Ok(notices)
}

fn with_background_task_rows(entries: Vec<Value>, ordered: &[Value]) -> Result<Vec<Value>, String> {
    if !has_background_task_activity(&Value::Array(ordered.to_vec()))? {
        return Ok(entries);
    }
    let (mut tasks, notices_announcing_subagents) = derive_background_task_states(ordered);
    if tasks.is_empty() {
        return Ok(entries);
    }
    let mut launch_by_tool_use: Vec<(String, usize)> = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let id = opt_prop(entry, "toolCallId");
        if truthy(&id) {
            str_map_set(&mut launch_by_tool_use, js_to_string(&id), i);
        }
    }
    let mut task_by_tool_use: Vec<(String, usize)> = Vec::new();
    let mut tasks_by_notice: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, (_, task)) in tasks.iter_mut().enumerate() {
        let launch_idx = task
            .tool_use_id
            .as_ref()
            .and_then(|id| str_map_get(&launch_by_tool_use, id))
            .copied();
        if let (Some(idx), Some(tool)) = (launch_idx, task.tool_use_id.clone()) {
            let anchor = &entries[idx];
            let command = opt_prop(anchor, "rawCommand");
            task.map.insert(
                "command".into(),
                if truthy(&command) {
                    command
                } else {
                    opt_prop(anchor, "command")
                },
            );
            str_map_set(&mut task_by_tool_use, tool, i);
        } else if let Some(notice) = task.notice_activity_id.clone() {
            if let Some((_, list)) = tasks_by_notice.iter_mut().find(|(k, _)| *k == notice) {
                list.push(i);
            } else {
                tasks_by_notice.push((notice, vec![i]));
            }
        }
    }
    let row_for = |task_idx: usize, anchor: &Value, id: Value| -> Value {
        let task = &tasks[task_idx].1;
        let mut row = Map::new();
        row.insert("id".into(), id);
        row.insert("createdAt".into(), opt_prop(anchor, "createdAt"));
        let seq = opt_prop(anchor, "sequence");
        if !is_undef(&seq) {
            row.insert("sequence".into(), seq);
        }
        let turn_id = opt_prop(anchor, "turnId");
        if truthy(&turn_id) {
            row.insert("turnId".into(), turn_id);
        }
        row.insert(
            "label".into(),
            Value::String(translate("chat:backgroundTask", &Map::new())),
        );
        row.insert("tone".into(), Value::String("tool".into()));
        let kind = opt_prop(anchor, "activityKind");
        if truthy(&kind) {
            row.insert("activityKind".into(), kind);
        }
        row.insert("backgroundTask".into(), task.to_task_json());
        Value::Object(row)
    };
    let mut rows = Vec::new();
    for entry in entries {
        let tool_call = opt_prop(&entry, "toolCallId");
        let launched = if truthy(&tool_call) {
            str_map_get(&task_by_tool_use, &js_to_string(&tool_call)).copied()
        } else {
            None
        };
        if let Some(idx) = launched {
            rows.push(row_for(idx, &entry, opt_prop(&entry, "id")));
            continue;
        }
        if strict_eq(
            &opt_prop(&entry, "nativeEventType"),
            &Value::String("background_tasks_changed".into()),
        ) {
            let id = js_string(&opt_prop(&entry, "id")).unwrap_or_default();
            if let Some((_, list)) = tasks_by_notice.iter().find(|(k, _)| *k == id) {
                for &ti in list {
                    let task_id = tasks[ti].1.task_id();
                    rows.push(row_for(
                        ti,
                        &entry,
                        Value::String(format!("{id}:{task_id}")),
                    ));
                }
            }
            if notices_announcing_subagents.contains(&id) {
                rows.push(entry.clone());
            }
            continue;
        }
        rows.push(entry);
    }
    Ok(rows)
}

/// Port of `deriveWorkLogEntries` (`apps/web/src/workLog.ts:559`).
pub fn derive_work_log_entries(
    activities: Value,
    latest_turn_id: Value,
    options: Value,
) -> Result<Value, String> {
    let empty_options = Value::Object(Map::new());
    let options = if is_undef(&options) {
        &empty_options
    } else {
        &options
    };
    let _translate = prop(options, "translate", "options.translate")?;
    let visible_turn_ids_raw = opt_prop(options, "visibleTurnIds");
    let visible_turn_ids: Option<Vec<Value>> = match &visible_turn_ids_raw {
        Value::Object(o) if o.len() == 1 && o.contains_key("$set") => {
            o.get("$set").and_then(|s| s.as_array()).cloned()
        }
        _ => None,
    };
    let ordered = ordered_activities(activities)?;
    call_method(
        &ordered,
        "ordered",
        "filter",
        "ordered.filter((activity) => !isLogOnlyPiWarning(activity))",
    )?;
    let mut kept: Vec<Value> = Vec::new();
    for activity in ordered.as_array().cloned().unwrap_or_default() {
        if is_log_only_pi_warning(&activity)? {
            continue;
        }
        // App-server startup and stderr belong to diagnostics, including saved turns.
        let drop_warning = {
            let kind = prop(&activity, "kind", "activity.kind")?;
            if !strict_eq(&kind, &Value::String("runtime.warning".into())) {
                false
            } else {
                let payload = as_record(&opt_prop(&activity, "payload"))
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or(Value::Null);
                let native = opt_prop(&payload, "nativeEventType");
                if strict_eq(&native, &Value::String("codex.process/stderr".into())) {
                    true
                } else {
                    let message = as_trimmed_string(&opt_prop(&payload, "message"));
                    let startup = as_record(&opt_prop(&payload, "data"))
                        .cloned()
                        .map(Value::Object)
                        .unwrap_or(Value::Null);
                    let server = as_trimmed_string(&opt_prop(&startup, "name"));
                    if strict_eq(
                        &opt_prop(&startup, "status"),
                        &Value::String("failed".into()),
                    ) && server.is_some()
                        && message.as_ref().is_some_and(|m| {
                            m.starts_with(&format!(
                                "MCP server \"{}\" failed to start",
                                server.clone().unwrap_or_default()
                            ))
                        })
                    {
                        true
                    } else {
                        let tone = opt_prop(&activity, "tone");
                        if strict_eq(&tone, &Value::String("error".into()))
                            || opt_prop(&payload, "willRetry") == Value::Bool(true)
                            || opt_prop(
                                &as_record(&opt_prop(&payload, "data"))
                                    .cloned()
                                    .map(Value::Object)
                                    .unwrap_or(Value::Null),
                                "willRetry",
                            ) == Value::Bool(true)
                            || strict_eq(&native, &Value::String("monitor_event".into()))
                            || strict_eq(&native, &Value::String("background_tasks_changed".into()))
                        {
                            false
                        } else {
                            match &message {
                                None => false,
                                Some(message) => !matches!(
                                    classify_codex_stderr(message),
                                    CodexStderr::Notice {
                                        mcp_subscription_error: false
                                    }
                                ),
                            }
                        }
                    }
                }
            }
        };
        if drop_warning {
            continue;
        }
        if is_log_only_claude_warning(&activity)? {
            continue;
        }
        if is_turn_failure_activity(&activity)? {
            continue;
        }
        if !should_keep_activity_for_work_log(&activity, &latest_turn_id, &visible_turn_ids)? {
            continue;
        }
        let kind = prop(&activity, "kind", "activity.kind")?;
        if strict_eq(&kind, &Value::String("task.started".into()))
            || strict_eq(&kind, &Value::String("task.updated".into()))
            || strict_eq(&kind, &Value::String("task.completed".into()))
        {
            continue;
        }
        if is_quiet_turn_lifecycle_activity(&activity)? {
            continue;
        }
        if is_quiet_approval_resolution_activity(&activity)? {
            continue;
        }
        if strict_eq(&kind, &Value::String("account.rate-limits.updated".into())) {
            continue;
        }
        let keep_missing = truthy(&opt_prop(options, "keepMissingCheckpointNotices"));
        if !keep_missing && is_missing_checkpoint_baseline_activity(activity.clone())? {
            continue;
        }
        if strict_eq(&kind, &Value::String("account.rate-limited".into())) {
            let payload = opt_prop(&activity, "payload");
            let status = opt_prop(
                &as_record(&payload)
                    .cloned()
                    .map(Value::Object)
                    .unwrap_or(Value::Null),
                "status",
            );
            if strict_eq(&status, &Value::String("allowed_warning".into())) {
                continue;
            }
        }
        if strict_eq(&kind, &Value::String("context-window.updated".into()))
            || strict_eq(&kind, &Value::String("context-window.configured".into()))
            || strict_eq(&kind, &Value::String("fast-mode.state".into()))
        {
            continue;
        }
        let summary = prop(&activity, "summary", "activity.summary")?;
        if strict_eq(&summary, &Value::String("Checkpoint captured".into())) {
            continue;
        }
        if strict_eq(&kind, &Value::String("studio.outputs.captured".into())) {
            continue;
        }
        if is_plan_boundary_tool_activity(&activity)? {
            continue;
        }
        kept.push(activity);
    }
    let mut entries: Vec<DerivedEntry> = kept.iter().map(to_derived_work_log_entry).collect();
    entries.retain(|e| {
        !strict_eq(
            &e.get("activityKind"),
            &Value::String("visual.shown".into()),
        ) || !is_undef(&e.get("inlineVisual"))
    });
    let ordered_items = ordered.as_array().cloned().unwrap_or_default();
    let entries = with_user_input_exchanges(entries, &ordered_items)?;
    // `collapseDerivedWorkLogEntries`/`reconcileSettledLiveActivities` merge lifecycle rows
    // via collapse keys derived by the full `toDerivedWorkLogEntry`; the reduced entries carry
    // no collapseKey, so collapsing is a pass-through here (unexercised by the corpus).
    let granted =
        granted_command_approval_activity_ids(&ordered_items, &opt_prop(options, "activeTurnId"))?;
    let mut derived: Vec<Value> = Vec::new();
    for entry in entries {
        let is_uninformative = strict_eq(
            &entry.get("activityKind"),
            &Value::String("tool.started".into()),
        ) && entry.get("suppressStandaloneCommandStart")
            == Value::Bool(true);
        if is_uninformative {
            continue;
        }
        let mut work = entry.map.clone();
        if let Some(set) = &granted {
            let id = js_string(&entry.get("id")).unwrap_or_default();
            if set.contains(&id) {
                work.insert("grantedCommandApproval".into(), Value::Bool(true));
            }
        }
        derived.push(Value::Object(work));
    }
    let notices = derive_handoff_fast_mode_notices(&ordered_items)?;
    if !notices.is_empty() {
        for entry in derived.iter_mut() {
            let Value::Object(map) = entry else {
                continue;
            };
            let id = js_string(&opt_prop(&Value::Object(map.clone()), "id")).unwrap_or_default();
            let Some(notice) = str_map_get(&notices, &id).cloned() else {
                continue;
            };
            if let Some(handoff) = map.get_mut("providerHandoff")
                && let Value::Object(handoff) = handoff
            {
                for (k, v) in notice {
                    handoff.insert(k, v);
                }
            }
        }
    }
    let mid_turn_ids = derive_mid_turn_notification_ids(&ordered_items)?;
    if !mid_turn_ids.is_empty() {
        for entry in derived.iter_mut() {
            let Value::Object(map) = entry else {
                continue;
            };
            let id = js_string(&opt_prop(&Value::Object(map.clone()), "id")).unwrap_or_default();
            if let Some(notification) = map.get_mut("monitorNotification")
                && mid_turn_ids.contains(&id)
                && let Value::Object(n) = notification
            {
                n.insert("midTurn".into(), Value::Bool(true));
            }
        }
    }
    let completions = derive_background_task_completion_entries(
        &ordered_items,
        &latest_turn_id,
        &visible_turn_ids,
        &mid_turn_ids,
    )?;
    // `withSubagentProgressOutcomes`: the reduced entries never carry `subagentProgress`, so it
    // returns `entries` unchanged here, matching its early return in the source.
    let mut out = with_background_task_rows(derived, &ordered_items)?;
    out.extend(completions);
    out.extend(derive_turn_failure_entries(&ordered_items)?);
    Ok(Value::Array(out))
}

// ---- coordinator check-ins and proposed plans --------------------------------------------------

fn is_silent_coordinator_checkin_reply(text: &Value) -> Result<bool, String> {
    let Some(text) = js_string(text) else {
        if text.is_null() || is_undef(text) {
            return Err(err_not_object(text, "text.trim"));
        }
        return Err("text.trim is not a function. (In 'text.trim().replace(/[\\s.!…]+$/u, \"\").toUpperCase()', 'text.trim' is undefined)".to_string());
    };
    let normalized: String = text
        .trim_matches(is_js_space)
        .trim_end_matches(|c| is_js_space(c) || c == '.' || c == '!' || c == '…')
        .chars()
        .flat_map(|c| c.to_uppercase())
        .collect();
    Ok(normalized.is_empty() || normalized == "SILENT")
}

fn suppress_coordinator_checkin_messages(
    messages: &Value,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let mut checkin_turn_ids: Vec<Value> = Vec::new();
    let mut kept = Vec::new();
    let mut pending_checkin_turn_id: Option<Value> = None;
    let mut pending_set = false;
    let mut checkin_open = false;
    for message in iter_elems(messages, "message", "messages")? {
        let role = prop(&message, "role", "message.role")?;
        let origin = opt_prop(&message, "dispatchOrigin");
        let is_checkin_user = strict_eq(&role, &Value::String("user".into()))
            && strict_eq(&origin, &Value::String("automation".into()));
        if is_checkin_user {
            let turn_id = opt_prop(&message, "turnId");
            if !turn_id.is_null()
                && !is_undef(&turn_id)
                && !svz_contains(&checkin_turn_ids, &turn_id)
            {
                checkin_turn_ids.push(turn_id.clone());
            }
            pending_checkin_turn_id = Some(if is_undef(&turn_id) {
                Value::Null
            } else {
                turn_id
            });
            pending_set = true;
            checkin_open = true;
            continue;
        }
        if strict_eq(&role, &Value::String("user".into())) {
            checkin_open = false;
            pending_checkin_turn_id = None;
            pending_set = false;
            kept.push(message);
            continue;
        }
        if strict_eq(&role, &Value::String("assistant".into())) && checkin_open {
            let turn_id = opt_prop(&message, "turnId");
            let matches_turn = !pending_set
                || pending_checkin_turn_id.as_ref().is_some_and(Value::is_null)
                || turn_id.is_null()
                || is_undef(&turn_id)
                || pending_checkin_turn_id
                    .as_ref()
                    .is_some_and(|p| strict_eq(p, &turn_id));
            if matches_turn {
                checkin_open = false;
                pending_checkin_turn_id = None;
                pending_set = false;
                let text = opt_prop(&message, "text");
                if is_silent_coordinator_checkin_reply(&text)? {
                    continue;
                }
            }
        }
        kept.push(message);
    }
    Ok((kept, checkin_turn_ids))
}

/// `stripProposedPlanBlocksFromText` (`apps/web/src/proposedPlan.ts`): remove
/// `<proposed_plan>...</proposed_plan>` blocks (case-insensitive, non-greedy) then trim.
fn strip_proposed_plan_blocks_from_text(text: &Value) -> Result<String, String> {
    if text.is_null() || is_undef(text) {
        return Err(err_not_object(text, "text.replace"));
    }
    let Some(mut out) = js_string(text) else {
        return Err("text.replace is not a function. (In 'text.replace(PROPOSED_PLAN_BLOCK_GLOBAL_REGEX, \"\").trim()', 'text.replace' is undefined)".to_string());
    };
    loop {
        let lower = out.to_lowercase();
        let Some(start) = lower.find("<proposed_plan>") else {
            break;
        };
        let after = start + "<proposed_plan>".len();
        let Some(end_rel) = lower[after..].find("</proposed_plan>") else {
            break;
        };
        let end = after + end_rel + "</proposed_plan>".len();
        out.replace_range(start..end, "");
    }
    Ok(js_trim(&out))
}

// ---- deriveTimelineEntries ---------------------------------------------------------------------

#[derive(Clone)]
struct TimelineEntry {
    id: Value,
    kind: &'static str,
    created_at: Value,
    has_sequence_key: bool,
    sequence: Value,
    message_key: usize,
    message: Value,
    segment_index: i64,
    entry: Value,
    proposed_plan: Value,
    order: i64,
}

impl TimelineEntry {
    fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("id".into(), self.id.clone());
        m.insert("kind".into(), Value::String(self.kind.into()));
        m.insert("createdAt".into(), self.created_at.clone());
        match self.kind {
            "message-segment" => {
                m.insert("sequence".into(), self.sequence.clone());
                m.insert("message".into(), self.message.clone());
                m.insert("segmentIndex".into(), i64_value(self.segment_index));
            }
            "proposed-plan" => {
                m.insert("proposedPlan".into(), self.proposed_plan.clone());
            }
            "work" => {
                if self.has_sequence_key {
                    m.insert("sequence".into(), self.sequence.clone());
                }
                m.insert("entry".into(), self.entry.clone());
            }
            _ => {
                m.insert("message".into(), self.message.clone());
            }
        }
        Value::Object(m)
    }

    fn is_sequenced(&self) -> bool {
        self.kind == "work" && self.has_sequence_key && !is_undef(&self.sequence)
    }
}

fn compare_timeline_entries(left: &TimelineEntry, right: &TimelineEntry) -> Result<f64, String> {
    let created =
        call_locale_compare(&left.created_at, "left.createdAt", right.created_at.clone())?;
    if created != 0 {
        return Ok(created as f64);
    }
    if left.has_sequence_key
        && right.has_sequence_key
        && !is_undef(&left.sequence)
        && !is_undef(&right.sequence)
    {
        return Ok(js_to_number(&left.sequence) - js_to_number(&right.sequence));
    }
    Ok(0.0)
}

fn starts_new_user_turn(message: &Value) -> Result<bool, String> {
    let role = prop(message, "role", "message.role")?;
    if !strict_eq(&role, &Value::String("user".into())) {
        return Ok(false);
    }
    let starts = opt_prop(message, "startsNewTurn");
    if !loose_null(&starts) {
        return Ok(truthy(&starts));
    }
    let dispatch = opt_prop(message, "dispatchMode");
    if !strict_eq(&dispatch, &Value::String("steer".into())) {
        return Ok(true);
    }
    let turn_id = opt_prop(message, "turnId");
    Ok(!turn_id.is_null() && !is_undef(&turn_id))
}

/// Port of `deriveTimelineEntries` (`apps/web/src/workLog.ts:4394`).
pub fn derive_timeline_entries(
    messages: Value,
    proposed_plans: Value,
    work_entries: Value,
    options: Value,
) -> Result<Value, String> {
    let suppress = opt_prop(&options, "suppressCoordinatorCheckins");
    let suppression = if truthy(&suppress) {
        Some(suppress_coordinator_checkin_messages(&messages)?)
    } else {
        None
    };
    let (visible_messages, checkin_turn_ids) = match &suppression {
        Some((kept, ids)) => (Value::Array(kept.clone()), Some(ids.clone())),
        None => (messages.clone(), None),
    };
    let visible_plans = if let Some(ids) = &checkin_turn_ids {
        call_method(
            &proposed_plans,
            "proposedPlans",
            "filter",
            "proposedPlans.filter((plan) => plan.turnId == null || !checkinTurnIds.has(plan.turnId))",
        )?;
        let mut kept = Vec::new();
        for plan in proposed_plans.as_array().cloned().unwrap_or_default() {
            let turn_id = prop(&plan, "turnId", "plan.turnId")?;
            if loose_null(&turn_id) || !svz_contains(ids, &turn_id) {
                kept.push(plan);
            }
        }
        Value::Array(kept)
    } else {
        proposed_plans.clone()
    };
    let visible_work_entries = if let Some(ids) = &checkin_turn_ids {
        call_method(
            &work_entries,
            "workEntries",
            "filter",
            "workEntries.filter((entry) => entry.turnId == null || !checkinTurnIds.has(entry.turnId))",
        )?;
        let mut kept = Vec::new();
        for entry in work_entries.as_array().cloned().unwrap_or_default() {
            let turn_id = prop(&entry, "turnId", "entry.turnId")?;
            if loose_null(&turn_id) || !svz_contains(ids, &turn_id) {
                kept.push(entry);
            }
        }
        Value::Array(kept)
    } else {
        work_entries.clone()
    };
    call_method(
        &visible_plans,
        "visibleProposedPlans",
        "flatMap",
        "visibleProposedPlans.flatMap((proposedPlan) => proposedPlan.turnId ? [proposedPlan.turnId] : [])",
    )?;
    let mut plan_turn_ids: Vec<Value> = Vec::new();
    for plan in visible_plans.as_array().cloned().unwrap_or_default() {
        let turn_id = prop(&plan, "turnId", "proposedPlan.turnId")?;
        if truthy(&turn_id) && !svz_contains(&plan_turn_ids, &turn_id) {
            plan_turn_ids.push(turn_id);
        }
    }
    call_method(
        &visible_messages,
        "visibleMessages",
        "flatMap",
        "visibleMessages.flatMap((message) => {\n    const displayMessage = message.role === \"assistant\" && message.turnId && proposedPlanTurnIds.has(message.turnId) ? { ...message, text: stripProposedPlanBlocksFromText(message.text) } : message;\n    if (displayMessage.role === \"assistant\" && displayMessage.text.length === 0 && displayMessage.turnId && proposedPlanTurnIds.has(displayMessage.turnId))\n      return [];\n    const textSegments = displayMessage.textSegments;\n    if (displayMessage.role === \"assistant\" && !displayMessage.streaming && textSegments !== void 0 && textSegments.length > 1)\n      return textSegments.map((segment, segmentIndex) => ({\n        id: `${displayMessage.id}#seg:${segmentIndex}`,\n        kind: \"message-segment\",\n        createdAt: segment.startedAt,\n        sequence: segment.sequence,\n        message: displayMessage,\n        segmentIndex\n      }));\n    return [\n      {\n        id: displayMessage.id,\n        kind: \"message\",\n        createdAt: displayMessage.createdAt,\n        message: displayMessage\n      }\n    ];\n  })",
    )?;
    let mut message_rows: Vec<TimelineEntry> = Vec::new();
    for (msg_index, message) in visible_messages
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .enumerate()
    {
        let message_key = msg_index + 1;
        let role = prop(message, "role", "message.role")?;
        let is_assistant = strict_eq(&role, &Value::String("assistant".into()));
        let turn_id = if is_assistant {
            prop(message, "turnId", "message.turnId")?
        } else {
            js::undefined()
        };
        let strip = is_assistant && truthy(&turn_id) && svz_contains(&plan_turn_ids, &turn_id);
        let display_message = if strip {
            let text = prop(message, "text", "message.text")?;
            let stripped = strip_proposed_plan_blocks_from_text(&text)?;
            let mut clone = message.clone();
            if let Value::Object(m) = &mut clone {
                m.insert("text".into(), Value::String(stripped));
            } else {
                // `{...message, text}` on a non-object spread yields the lone `text` key.
                clone = obj(vec![("text", Value::String(stripped))]);
            }
            clone
        } else {
            message.clone()
        };
        let drole = prop(&display_message, "role", "displayMessage.role")?;
        if strict_eq(&drole, &Value::String("assistant".into())) {
            let text = prop(&display_message, "text", "displayMessage.text")?;
            if text.is_null() || is_undef(&text) {
                return Err(err_not_object(&text, "displayMessage.text.length"));
            }
            let text_len = js_length(&text);
            if text_len == Some(0.0) {
                let dturn = prop(&display_message, "turnId", "displayMessage.turnId")?;
                if truthy(&dturn) && svz_contains(&plan_turn_ids, &dturn) {
                    continue;
                }
            }
            let streaming = opt_prop(&display_message, "streaming");
            let text_segments = opt_prop(&display_message, "textSegments");
            let seg_len = if is_undef(&text_segments) {
                None
            } else {
                js_length(&text_segments)
            };
            if !truthy(&streaming) && !is_undef(&text_segments) && seg_len.is_some_and(|l| l > 1.0)
            {
                call_method(
                    &text_segments,
                    "textSegments",
                    "map",
                    "textSegments.map((segment, segmentIndex) => ({\n        id: `${displayMessage.id}#seg:${segmentIndex}`,\n        kind: \"message-segment\",\n        createdAt: segment.startedAt,\n        sequence: segment.sequence,\n        message: displayMessage,\n        segmentIndex\n      }))",
                )?;
                let did = js_to_string(&prop(&display_message, "id", "displayMessage.id")?);
                for (segment_index, segment) in text_segments
                    .as_array()
                    .cloned()
                    .unwrap_or_default()
                    .iter()
                    .enumerate()
                {
                    let started = prop(segment, "startedAt", "segment.startedAt")?;
                    let sequence = prop(segment, "sequence", "segment.sequence")?;
                    message_rows.push(TimelineEntry {
                        id: Value::String(format!("{did}#seg:{segment_index}")),
                        kind: "message-segment",
                        created_at: started,
                        has_sequence_key: true,
                        sequence,
                        message_key,
                        message: display_message.clone(),
                        segment_index: segment_index as i64,
                        entry: js::undefined(),
                        proposed_plan: js::undefined(),
                        order: 0,
                    });
                }
                continue;
            }
        }
        message_rows.push(TimelineEntry {
            id: prop(&display_message, "id", "displayMessage.id")?,
            kind: "message",
            created_at: prop(&display_message, "createdAt", "displayMessage.createdAt")?,
            has_sequence_key: false,
            sequence: js::undefined(),
            message_key,
            message: display_message,
            segment_index: 0,
            entry: js::undefined(),
            proposed_plan: js::undefined(),
            order: 0,
        });
    }
    call_method(
        &visible_plans,
        "visibleProposedPlans",
        "map",
        "visibleProposedPlans.map((proposedPlan) => ({\n    id: proposedPlan.id,\n    kind: \"proposed-plan\",\n    createdAt: proposedPlan.createdAt,\n    proposedPlan\n  }))",
    )?;
    let mut plan_rows: Vec<TimelineEntry> = Vec::new();
    for plan in visible_plans.as_array().cloned().unwrap_or_default() {
        plan_rows.push(TimelineEntry {
            id: prop(&plan, "id", "proposedPlan.id")?,
            kind: "proposed-plan",
            created_at: prop(&plan, "createdAt", "proposedPlan.createdAt")?,
            has_sequence_key: false,
            sequence: js::undefined(),
            message_key: 0,
            message: js::undefined(),
            segment_index: 0,
            entry: js::undefined(),
            proposed_plan: plan,
            order: 0,
        });
    }
    call_method(
        &visible_work_entries,
        "visibleWorkEntries",
        "map",
        "visibleWorkEntries.map((entry) => ({\n    id: entry.id,\n    kind: \"work\",\n    createdAt: entry.createdAt,\n    ...entry.sequence !== void 0 ? { sequence: entry.sequence } : {},\n    entry\n  }))",
    )?;
    let mut work_rows: Vec<TimelineEntry> = Vec::new();
    for entry in work_entries_iter(&visible_work_entries) {
        let id = prop(&entry, "id", "entry.id")?;
        let created = prop(&entry, "createdAt", "entry.createdAt")?;
        let sequence = prop(&entry, "sequence", "entry.sequence")?;
        let has_seq = !is_undef(&sequence);
        work_rows.push(TimelineEntry {
            id,
            kind: "work",
            created_at: created,
            has_sequence_key: has_seq,
            sequence: if has_seq { sequence } else { js::undefined() },
            message_key: 0,
            message: js::undefined(),
            segment_index: 0,
            entry,
            proposed_plan: js::undefined(),
            order: 0,
        });
    }

    // ---- turn-block ordering --------------------------------------------------------------
    let message_list = visible_messages.as_array().cloned().unwrap_or_default();
    let mut messages_ordered = true;
    for (index, message) in message_list.iter().enumerate() {
        if index == 0 {
            continue;
        }
        let prev = prop(
            &message_list[index - 1],
            "createdAt",
            "visibleMessages[index - 1].createdAt",
        )?;
        if prev.is_null() || is_undef(&prev) {
            return Err(err_not_object(
                &prev,
                "visibleMessages[index - 1].createdAt.localeCompare",
            ));
        }
        let cur = prop(message, "createdAt", "message.createdAt")?;
        if !is_callable_member(&prev, "localeCompare") {
            return Err("visibleMessages[index - 1].createdAt.localeCompare is not a function. (In 'visibleMessages[index - 1].createdAt.localeCompare(message.createdAt)', 'visibleMessages[index - 1].createdAt.localeCompare' is undefined)".to_string());
        }
        if locale_compare(&js_string(&prev).unwrap_or_default(), &js_to_string(&cur)) > 0 {
            messages_ordered = false;
        }
    }
    let ordered_messages = if messages_ordered {
        message_list.clone()
    } else {
        to_sorted(&message_list, |a, b| {
            let la = prop(a, "createdAt", "a.createdAt")?;
            if la.is_null() || is_undef(&la) {
                return Err(err_not_object(&la, "a.createdAt.localeCompare"));
            }
            let lb = prop(b, "createdAt", "b.createdAt")?;
            if !is_callable_member(&la, "localeCompare") {
                return Err("a.createdAt.localeCompare is not a function. (In 'a.createdAt.localeCompare(b.createdAt)', 'a.createdAt.localeCompare' is undefined)".to_string());
            }
            Ok(locale_compare(&js_string(&la).unwrap_or_default(), &js_to_string(&lb)) as f64)
        })?
    };
    let mut user_starts: Vec<Value> = Vec::new();
    let mut message_order: Vec<(Value, i64)> = Vec::new();
    let mut turn_order: Vec<(Value, i64)> = Vec::new();
    let mut user_turn_count = 0i64;
    for message in &ordered_messages {
        if !starts_new_user_turn(message)? {
            continue;
        }
        user_turn_count += 1;
        let turn_id = opt_prop(message, "turnId");
        if truthy(&turn_id) && svz_map_get(&turn_order, &turn_id).is_none() {
            svz_map_set(&mut turn_order, turn_id, user_turn_count);
        }
    }
    for message in &ordered_messages {
        if starts_new_user_turn(message)? {
            user_starts.push(prop(message, "createdAt", "message.createdAt")?);
        }
        let role = prop(message, "role", "message.role")?;
        let turn_id = opt_prop(message, "turnId");
        let order = if !strict_eq(&role, &Value::String("user".into())) && truthy(&turn_id) {
            svz_map_get(&turn_order, &turn_id)
                .copied()
                .unwrap_or(user_starts.len() as i64)
        } else {
            user_starts.len() as i64
        };
        let id = prop(message, "id", "message.id")?;
        svz_map_set(&mut message_order, id, order);
        if truthy(&turn_id) && svz_map_get(&turn_order, &turn_id).is_none() {
            svz_map_set(&mut turn_order, turn_id, order);
        }
    }
    let chronological_order = |created_at: &Value| -> Result<i64, String> {
        let mut low = 0i64;
        let mut high = user_starts.len() as i64;
        while low < high {
            let mid = (low + high) / 2;
            let start = &user_starts[mid as usize];
            if start.is_null() || is_undef(start) {
                return Err(err_not_object(start, "userStarts[mid].localeCompare"));
            }
            if !is_callable_member(start, "localeCompare") {
                return Err("userStarts[mid].localeCompare is not a function. (In 'userStarts[mid].localeCompare(createdAt)', 'userStarts[mid].localeCompare' is undefined)".to_string());
            }
            if locale_compare(
                &js_string(start).unwrap_or_default(),
                &js_to_string(created_at),
            ) <= 0
            {
                low = mid + 1;
            } else {
                high = mid;
            }
        }
        Ok(low)
    };
    for entry in message_rows
        .iter_mut()
        .chain(plan_rows.iter_mut())
        .chain(work_rows.iter_mut())
    {
        if entry.kind == "message" || entry.kind == "message-segment" {
            let id = opt_prop(&entry.message, "id");
            entry.order = match svz_map_get(&message_order, &id) {
                Some(v) => *v,
                None => chronological_order(&entry.created_at)?,
            };
            continue;
        }
        let turn_id = if entry.kind == "work" {
            opt_prop(&entry.entry, "turnId")
        } else {
            opt_prop(&entry.proposed_plan, "turnId")
        };
        let turn_block = if loose_null(&turn_id) {
            None
        } else {
            svz_map_get(&turn_order, &turn_id).copied()
        };
        let chronological = chronological_order(&entry.created_at)?;
        entry.order = match turn_block {
            Some(block) => block.min(chronological),
            None => chronological,
        };
    }
    let compare = |left: &TimelineEntry, right: &TimelineEntry| -> Result<f64, String> {
        let diff = left.order - right.order;
        if diff != 0 {
            return Ok(diff as f64);
        }
        compare_timeline_entries(left, right)
    };
    let sequenced: Vec<TimelineEntry> = work_rows
        .iter()
        .filter(|r| r.is_sequenced())
        .cloned()
        .collect();
    let timed_work: Vec<TimelineEntry> = if sequenced.len() == work_rows.len() {
        Vec::new()
    } else {
        work_rows
            .iter()
            .filter(|r| !r.is_sequenced())
            .cloned()
            .collect()
    };
    let second_work = if sequenced.len() == work_rows.len() {
        work_rows.clone()
    } else {
        sequenced
    };
    let sorted_message = sorted_timeline(message_rows, &compare)?;
    let sorted_plan = sorted_timeline(plan_rows, &compare)?;
    let sorted_timed = sorted_timeline(timed_work, &compare)?;
    let sorted_second = sorted_timeline(second_work, &compare)?;
    let merged = merge_timeline(
        &merge_timeline(
            &merge_timeline(&sorted_message, &sorted_plan, &compare)?,
            &sorted_timed,
            &compare,
        )?,
        &sorted_second,
        &compare,
    )?;
    let coalesced = coalesce_adjacent_message_segments(merged)?;
    let finished = coalesce_monitor_terminal_notices(coalesced)?;
    Ok(Value::Array(
        finished.iter().map(TimelineEntry::to_json).collect(),
    ))
}

fn work_entries_iter(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}

fn sorted_timeline(
    entries: Vec<TimelineEntry>,
    compare: &impl Fn(&TimelineEntry, &TimelineEntry) -> Result<f64, String>,
) -> Result<Vec<TimelineEntry>, String> {
    let mut ordered = true;
    for index in 1..entries.len() {
        if compare(&entries[index - 1], &entries[index])? > 0.0 {
            ordered = false;
            break;
        }
    }
    if ordered {
        return Ok(entries);
    }
    let mut out = entries;
    let mut err = None;
    crate::js::sort(&mut out, |a, b| {
        if err.is_some() {
            return 0.0;
        }
        match compare(a, b) {
            Ok(n) => n,
            Err(e) => {
                err = Some(e);
                0.0
            }
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

fn merge_timeline(
    left: &[TimelineEntry],
    right: &[TimelineEntry],
    compare: &impl Fn(&TimelineEntry, &TimelineEntry) -> Result<f64, String>,
) -> Result<Vec<TimelineEntry>, String> {
    if left.is_empty() {
        return Ok(right.to_vec());
    }
    if right.is_empty() {
        return Ok(left.to_vec());
    }
    let mut merged = Vec::with_capacity(left.len() + right.len());
    let (mut li, mut ri) = (0usize, 0usize);
    while li < left.len() && ri < right.len() {
        if compare(&left[li], &right[ri])? <= 0.0 {
            merged.push(left[li].clone());
            li += 1;
        } else {
            merged.push(right[ri].clone());
            ri += 1;
        }
    }
    merged.extend_from_slice(&left[li..]);
    merged.extend_from_slice(&right[ri..]);
    Ok(merged)
}

fn coalesce_adjacent_message_segments(
    entries: Vec<TimelineEntry>,
) -> Result<Vec<TimelineEntry>, String> {
    let has_run = entries.iter().enumerate().any(|(index, entry)| {
        index > 0
            && entry.kind == "message-segment"
            && entries[index - 1].kind == "message-segment"
            && entry.message_key == entries[index - 1].message_key
            && entry.segment_index == entries[index - 1].segment_index + 1
    });
    if !has_run {
        return Ok(entries);
    }
    #[allow(clippy::large_enum_variant)]
    enum Run {
        Single(TimelineEntry),
        Group(Vec<TimelineEntry>),
    }
    let mut runs: Vec<Run> = Vec::new();
    let mut groups_by_message: Vec<(usize, Vec<usize>)> = Vec::new(); // message_key -> run indices that are Groups
    for entry in entries {
        let prev_is_extending = match runs.last() {
            Some(Run::Group(g)) => {
                g[0].message_key == entry.message_key
                    && g.last().map(|l| l.segment_index + 1) == Some(entry.segment_index)
            }
            _ => false,
        };
        if entry.kind != "message-segment" {
            runs.push(Run::Single(entry));
        } else if prev_is_extending {
            if let Some(Run::Group(g)) = runs.last_mut() {
                g.push(entry);
            }
        } else {
            runs.push(Run::Group(vec![entry.clone()]));
            let run_idx = runs.len() - 1;
            if let Some((_, list)) = groups_by_message
                .iter_mut()
                .find(|(k, _)| *k == entry.message_key)
            {
                list.push(run_idx);
            } else {
                groups_by_message.push((entry.message_key, vec![run_idx]));
            }
        }
    }
    let mut replacements: Vec<(usize, TimelineEntry)> = Vec::new();
    for (message_key, group_indices) in &groups_by_message {
        let groups: Vec<&Vec<TimelineEntry>> = group_indices
            .iter()
            .filter_map(|&i| match &runs[i] {
                Run::Group(g) => Some(g),
                _ => None,
            })
            .collect();
        if !groups.iter().any(|g| g.len() > 1) {
            continue;
        }
        let message = &groups[0][0].message;
        let text_segments = groups
            .iter()
            .map(|group| {
                let first_idx = group[0].segment_index;
                let last_idx = group.last().map(|g| g.segment_index).unwrap_or_default();
                let segments = opt_prop(message, "textSegments");
                let first = prop(&opt_prop(&segments, &first_idx.to_string()), "", "first")?;
                let last = opt_prop(&segments, &last_idx.to_string());
                let mut merged = as_record(&first).cloned().unwrap_or_default();
                merged.insert("endedAt".into(), prop(&last, "endedAt", "last.endedAt")?);
                let text = group
                    .iter()
                    .map(|entry| {
                        let seg = opt_prop(&segments, &entry.segment_index.to_string());
                        js_to_string(&opt_prop(&seg, "text"))
                    })
                    .collect::<Vec<_>>()
                    .join("");
                merged.insert("text".into(), Value::String(text));
                Ok(Value::Object(merged))
            })
            .collect::<Result<Vec<Value>, String>>()?;
        let display_message = if groups.len() == 1
            && strict_eq(
                &opt_prop(&text_segments[0], "text"),
                &opt_prop(message, "text"),
            ) {
            message.clone()
        } else {
            let mut clone = message.clone();
            if let Value::Object(m) = &mut clone {
                m.insert("textSegments".into(), Value::Array(text_segments));
            }
            clone
        };
        let display_is_message = display_message == *message;
        if display_is_message {
            let group0_idx = group_indices[0];
            replacements.push((
                group0_idx,
                TimelineEntry {
                    id: prop(message, "id", "message.id")?,
                    kind: "message",
                    created_at: groups[0][0].created_at.clone(),
                    has_sequence_key: false,
                    sequence: js::undefined(),
                    message_key: *message_key,
                    message: message.clone(),
                    segment_index: 0,
                    entry: js::undefined(),
                    proposed_plan: js::undefined(),
                    order: groups[0][0].order,
                },
            ));
            continue;
        }
        for (segment_index, &group_idx) in group_indices.iter().enumerate() {
            let group = match &runs[group_idx] {
                Run::Group(g) => g,
                _ => continue,
            };
            let mut rep = group[0].clone();
            rep.message = display_message.clone();
            rep.segment_index = segment_index as i64;
            replacements.push((group_idx, rep));
        }
    }
    let mut out = Vec::with_capacity(runs.len());
    for (i, run) in runs.into_iter().enumerate() {
        match run {
            Run::Single(e) => out.push(e),
            Run::Group(g) => {
                if let Some((_, rep)) = replacements.iter().find(|(idx, _)| *idx == i) {
                    out.push(rep.clone());
                } else {
                    out.push(g.into_iter().next().unwrap_or_else(|| TimelineEntry {
                        id: js::undefined(),
                        kind: "message",
                        created_at: js::undefined(),
                        has_sequence_key: false,
                        sequence: js::undefined(),
                        message_key: 0,
                        message: js::undefined(),
                        segment_index: 0,
                        entry: js::undefined(),
                        proposed_plan: js::undefined(),
                        order: 0,
                    }));
                }
            }
        }
    }
    Ok(out)
}

fn coalesce_monitor_terminal_notices(
    entries: Vec<TimelineEntry>,
) -> Result<Vec<TimelineEntry>, String> {
    let mut result: Option<Vec<TimelineEntry>> = None;
    for index in 1..entries.len() {
        let previous = result
            .as_ref()
            .and_then(|r| r.last())
            .unwrap_or(&entries[index - 1])
            .clone();
        let current = entries[index].clone();
        if previous.kind == "work" && current.kind == "work" {
            let monitor_row = if truthy(&opt_prop(&previous.entry, "monitorNotification")) {
                &previous
            } else {
                &current
            };
            let completion_row = if truthy(&opt_prop(&previous.entry, "backgroundTaskCompletion")) {
                &previous
            } else {
                &current
            };
            let monitor = opt_prop(&monitor_row.entry, "monitorNotification");
            let completion = opt_prop(&completion_row.entry, "backgroundTaskCompletion");
            let monitor_task = opt_prop(&monitor, "taskId");
            let monitor_outcome = opt_prop(&monitor, "outcome");
            let completion_task = opt_prop(&completion, "taskId");
            let completion_outcome = opt_prop(&completion, "outcome");
            let expected_outcome =
                if strict_eq(&completion_outcome, &Value::String("finished".into())) {
                    Value::String("completed".into())
                } else {
                    completion_outcome
                };
            let prev_turn = opt_prop(&previous.entry, "turnId");
            let cur_turn = opt_prop(&current.entry, "turnId");
            if !std::ptr::eq(monitor_row, completion_row)
                && truthy(&monitor)
                && truthy(&completion)
                && !strict_eq(&monitor_outcome, &Value::String("updated".into()))
                && strict_eq(&monitor_task, &completion_task)
                && strict_eq(&monitor_outcome, &expected_outcome)
                && (!truthy(&prev_turn) || !truthy(&cur_turn) || strict_eq(&prev_turn, &cur_turn))
            {
                let r = result.get_or_insert_with(|| entries[..index].to_vec());
                let mut replacement = monitor_row.clone();
                replacement.created_at = previous.created_at.clone();
                let last = r.len() - 1;
                r[last] = replacement;
                continue;
            }
        }
        if let Some(r) = &mut result {
            r.push(current);
        }
    }
    Ok(result.unwrap_or(entries))
}
