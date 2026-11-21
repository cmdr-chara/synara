//! The Rust port of `apps/web/src/providerOrdering.ts`.
//!
//! Clean-room implementation (ADR-001): written from the recorded behavior in
//! `cases.jsonl` and the exported signatures, not from the source body.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Canonical provider order — what `normalizeProviderOrder` returns for any
/// input without valid kinds.
const PROVIDER_ORDER: [&str; 10] = [
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

/// A decoded JS value, so `undefined`, `NaN` and `-0` behave like they did
/// on the JS side of the bridge.
#[derive(Clone)]
enum Js {
    Undefined,
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Js>),
    Obj(Vec<(String, Js)>),
}

fn decode(v: &Value) -> Js {
    match v {
        Value::Null => Js::Null,
        Value::Bool(b) => Js::Bool(*b),
        Value::Number(n) => Js::Num(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => Js::Str(s.clone()),
        Value::Array(items) => Js::Arr(items.iter().map(decode).collect()),
        Value::Object(map) if map.get("$undefined").and_then(Value::as_bool) == Some(true) => {
            Js::Undefined
        }
        Value::Object(map) if map.len() == 1 && map.contains_key("$num") => {
            Js::Num(match map["$num"].as_str() {
                Some("NaN") => f64::NAN,
                Some("Infinity") => f64::INFINITY,
                Some("-Infinity") => f64::NEG_INFINITY,
                Some("-0") => -0.0,
                _ => f64::NAN,
            })
        }
        Value::Object(map) => Js::Obj(map.iter().map(|(k, v)| (k.clone(), decode(v))).collect()),
    }
}

/// JS `===`: objects and arrays only match by identity, so two decoded
/// values never equal; NaN matches nothing.
fn js_eq(a: &Js, b: &Js, nan_matches: bool) -> bool {
    match (a, b) {
        (Js::Undefined, Js::Undefined) | (Js::Null, Js::Null) => true,
        (Js::Bool(x), Js::Bool(y)) => x == y,
        (Js::Num(x), Js::Num(y)) => x == y || (nan_matches && x.is_nan() && y.is_nan()),
        (Js::Str(x), Js::Str(y)) => x == y,
        _ => false,
    }
}

/// `value.length`: null/undefined throw; strings and arrays report a
/// length; anything else yields `undefined` (`None`).
fn js_length(v: &Js) -> Result<Option<usize>, &'static str> {
    match v {
        Js::Null => Err("null"),
        Js::Undefined => Err("undefined"),
        Js::Str(s) => Ok(Some(s.encode_utf16().count())),
        Js::Arr(items) => Ok(Some(items.len())),
        _ => Ok(None),
    }
}

/// `container.indexOf(needle)`; strings coerce the needle with
/// `String(needle)`. `None` when `indexOf` isn't callable.
fn js_index_of(container: &Js, needle: &Js) -> Option<f64> {
    match container {
        Js::Arr(items) => Some(
            items
                .iter()
                .position(|item| js_eq(item, needle, true))
                .map(|i| i as f64)
                .unwrap_or(-1.0),
        ),
        Js::Str(haystack) => {
            let s = match needle {
                Js::Str(s) => s.clone(),
                Js::Undefined => "undefined".into(),
                Js::Null => "null".into(),
                Js::Bool(b) => b.to_string(),
                Js::Num(n) => format!("{}", n),
                _ => "[object Object]".into(),
            };
            Some(match haystack.find(s.as_str()) {
                Some(b) => haystack[..b].encode_utf16().count() as f64,
                None => -1.0,
            })
        }
        _ => None,
    }
}

/// Port of `isProviderKind` (`apps/web/src/providerOrdering.ts:15`).
pub fn is_provider_kind(value: String) -> Result<bool, String> {
    Ok(PROVIDER_ORDER.contains(&value.as_str()))
}

/// Port of `normalizeHiddenProviders` (`apps/web/src/providerOrdering.ts:19`).
pub fn normalize_hidden_providers(
    hidden_providers: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut seen = Vec::new();
    for kind in hidden_providers {
        if PROVIDER_ORDER.contains(&kind.as_str()) && !seen.contains(&kind) {
            seen.push(kind);
        }
    }
    Ok(seen.into_iter().map(Value::from).collect())
}

/// Port of `normalizeProviderOrder` (`apps/web/src/providerOrdering.ts:31`).
pub fn normalize_provider_order(
    provider_order: Vec<String>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut seen: Vec<&str> = Vec::new();
    for kind in &provider_order {
        if PROVIDER_ORDER.contains(&kind.as_str()) && !seen.contains(&kind.as_str()) {
            seen.push(kind.as_str());
        }
    }
    for kind in PROVIDER_ORDER {
        if !seen.contains(&kind) {
            seen.push(kind);
        }
    }
    Ok(seen.into_iter().map(Value::from).collect())
}

/// Port of `sameProviderOrder` (`apps/web/src/providerOrdering.ts:48`).
pub fn same_provider_order(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    let left = decode(&left);
    let right = decode(&right);
    let len_of = |v: &Js| {
        js_length(v).map_err(|w| format!("Cannot read properties of {w} (reading 'length')"))
    };
    if len_of(&left)? != len_of(&right)? {
        return Ok(false);
    }
    let Js::Arr(items) = &left else {
        return Err("left.every is not a function".to_string());
    };
    Ok(items.iter().enumerate().all(|(i, v)| {
        let other = match &right {
            Js::Arr(r) => r.get(i).cloned().unwrap_or(Js::Undefined),
            Js::Str(s) => s
                .encode_utf16()
                .nth(i)
                .and_then(|u| String::from_utf16(&[u]).ok())
                .map(Js::Str)
                .unwrap_or(Js::Undefined),
            _ => Js::Undefined,
        };
        js_eq(v, &other, false)
    }))
}

/// Port of `compareProvidersByOrder` (`apps/web/src/providerOrdering.ts:55`).
pub fn compare_providers_by_order(
    provider_order: serde_json::Value,
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<f64, String> {
    let order = decode(&provider_order);
    match js_index_of(&order, &Js::Undefined) {
        Some(_) => {}
        None => {
            return Err(match order {
                Js::Null => "Cannot read properties of null (reading 'indexOf')",
                Js::Undefined => "Cannot read properties of undefined (reading 'indexOf')",
                _ => "providerOrder.indexOf is not a function",
            }
            .to_string());
        }
    }
    // `value?.kind` — a property that only exists on provider objects.
    let kind_of = |v: &Value| match decode(v) {
        Js::Obj(fields) => fields
            .into_iter()
            .find(|(k, _)| k == "kind")
            .map(|(_, v)| v)
            .unwrap_or(Js::Undefined),
        _ => Js::Undefined,
    };
    Ok(js_index_of(&order, &kind_of(&left)).unwrap_or(-1.0)
        - js_index_of(&order, &kind_of(&right)).unwrap_or(-1.0))
}
