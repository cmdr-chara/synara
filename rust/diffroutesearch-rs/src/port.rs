//! The Rust port of `apps/web/src/diffRouteSearch.ts` —
//! `diffRouteSearchEquals` and `parseDiffRouteSearch`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `search[key]` with JS semantics: null/undefined receivers throw TypeError.
fn prop<'v>(v: &'v Value, k: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() || is_undefined(v) {
        return Err("Cannot read properties of null or undefined".to_string());
    }
    match v {
        Value::Object(o) => Ok(o.get(k).filter(|x| !is_undefined(x))),
        _ => Ok(None),
    }
}

fn str_units(v: &Value) -> Option<Vec<u16>> {
    crate::js::string_units(v)
}

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// `a === b` for route-search fields (strings, numbers, booleans, null/undefined;
/// distinct object references are never `===`).
fn js_strict_eq(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            if x.is_null() && y.is_null() {
                return true;
            }
            if is_undefined(x) || is_undefined(y) {
                return false;
            }
            if let (Some(u), Some(v)) = (str_units(x), str_units(y)) {
                return u == v;
            }
            if let (Some(u), Some(v)) = (crate::js::as_number(x), crate::js::as_number(y)) {
                return u == v;
            }
            matches!((x, y), (Value::Bool(u), Value::Bool(v)) if u == v)
        }
        _ => false,
    }
}

const FIELDS: [&str; 7] = [
    "splitViewId",
    "view",
    "editorFilePath",
    "panel",
    "diff",
    "diffTurnId",
    "diffFilePath",
];

/// Port of `diffRouteSearchEquals` (`apps/web/src/diffRouteSearch.ts:19`).
pub fn diff_route_search_equals(
    left: serde_json::Value,
    right: serde_json::Value,
) -> Result<bool, String> {
    for field in FIELDS {
        if !js_strict_eq(prop(&left, field)?, prop(&right, field)?) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `value.trim()` non-empty string, else undefined.
fn normalize_search_string(v: Option<&Value>) -> Option<String> {
    let units = str_units(v?)?;
    let s = String::from_utf16_lossy(&units);
    let trimmed = s.trim_matches(is_js_whitespace);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// `value === "1" || value === 1 || value === true`.
fn is_diff_open_value(v: Option<&Value>) -> bool {
    match v {
        Some(Value::Bool(true)) => true,
        Some(x) if str_units(x).is_some() => {
            str_units(x) == Some("1".encode_utf16().collect::<Vec<u16>>())
        }
        Some(x) => crate::js::as_number(x) == Some(1.0),
        None => false,
    }
}

/// Port of `parseDiffRouteSearch` (`apps/web/src/diffRouteSearch.ts:56`).
pub fn parse_diff_route_search(search: serde_json::Value) -> Result<serde_json::Value, String> {
    let split_view_id = normalize_search_string(prop(&search, "splitViewId")?);
    let view = match normalize_search_string(prop(&search, "view")?) {
        Some(v) if v == "editor" => Some("editor"),
        _ => None,
    };
    let editor_file_path = if view.is_some() {
        normalize_search_string(prop(&search, "editorFilePath")?)
    } else {
        None
    };
    let panel = match normalize_search_string(prop(&search, "panel")?).as_deref() {
        Some("browser") => Some("browser"),
        Some("diff") => Some("diff"),
        _ => None,
    };
    let diff = panel == Some("diff") || is_diff_open_value(prop(&search, "diff")?);
    let resolved_panel = panel.or(if diff { Some("diff") } else { None });
    let diff_turn_id = if diff {
        normalize_search_string(prop(&search, "diffTurnId")?)
    } else {
        None
    };
    let diff_file_path = if diff {
        normalize_search_string(prop(&search, "diffFilePath")?)
    } else {
        None
    };

    let mut out = serde_json::Map::new();
    if let Some(v) = split_view_id {
        out.insert("splitViewId".into(), json!(v));
    }
    if let Some(v) = view {
        out.insert("view".into(), json!(v));
    }
    if let Some(v) = editor_file_path {
        out.insert("editorFilePath".into(), json!(v));
    }
    if let Some(v) = resolved_panel {
        out.insert("panel".into(), json!(v));
    }
    if diff {
        out.insert("diff".into(), json!("1"));
    }
    if let Some(v) = diff_turn_id {
        out.insert("diffTurnId".into(), json!(v));
    }
    if let Some(v) = diff_file_path {
        out.insert("diffFilePath".into(), json!(v));
    }
    Ok(Value::Object(out))
}
