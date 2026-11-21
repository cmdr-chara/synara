//! The Rust port of `apps/desktop/src/windowState.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::Value;

fn get<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if js::is_undefined(v) {
        return None;
    }
    v.as_object().and_then(|o| o.get(key))
}

fn num_of(v: Option<&Value>) -> f64 {
    v.and_then(js::as_number).unwrap_or(f64::NAN)
}

fn is_finite_integer(v: &Value) -> bool {
    match js::as_number(v) {
        Some(n) => n.is_finite() && n.fract() == 0.0,
        None => false,
    }
}

fn is_window_bounds(v: &Value) -> bool {
    if v.is_null() || js::is_undefined(v) || !(v.is_object() || v.is_array()) {
        return false;
    }
    is_finite_integer_opt(get(v, "x"))
        && is_finite_integer_opt(get(v, "y"))
        && is_finite_integer_opt(get(v, "width"))
        && num_of(get(v, "width")) > 0.0
        && is_finite_integer_opt(get(v, "height"))
        && num_of(get(v, "height")) > 0.0
}

fn is_finite_integer_opt(v: Option<&Value>) -> bool {
    v.is_some_and(is_finite_integer)
}

/// Port of `parseDesktopWindowState` (`apps/desktop/src/windowState.ts:40`).
pub fn parse_desktop_window_state(value: Value) -> Result<Option<Value>, String> {
    if value.is_null() || js::is_undefined(&value) || !(value.is_object() || value.is_array()) {
        return Ok(None);
    }
    if js::as_number(get(&value, "version").unwrap_or(&Value::Null)) != Some(1.0)
        || !is_window_bounds(get(&value, "bounds").unwrap_or(&Value::Null))
        || !matches!(get(&value, "isMaximized"), Some(Value::Bool(_)))
    {
        return Ok(None);
    }
    let mut out = serde_json::Map::new();
    out.insert("version".to_string(), Value::from(1));
    out.insert(
        "bounds".to_string(),
        get(&value, "bounds").cloned().unwrap_or(Value::Null),
    );
    out.insert(
        "isMaximized".to_string(),
        get(&value, "isMaximized").cloned().unwrap_or(Value::Null),
    );
    Ok(Some(Value::Object(out)))
}

/// Port of `readDesktopWindowState` (`apps/desktop/src/windowState.ts:59`).
pub fn read_desktop_window_state(file_path: String) -> Result<Option<Value>, String> {
    let parsed = std::fs::read_to_string(&file_path)
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    match parsed {
        Some(v) => parse_desktop_window_state(v),
        None => Ok(None),
    }
}

/// POSIX `path.dirname`.
fn dirname(path: &str) -> String {
    let stripped = path.trim_end_matches('/');
    match stripped.rfind('/') {
        Some(0) => "/".to_string(),
        Some(i) => stripped[..i].to_string(),
        None => ".".to_string(),
    }
}

fn js_escape_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{9}' => out.push_str("\\t"),
            '\u{a}' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\u{d}' => out.push_str("\\r"),
            c if c < '\u{20}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `JSON.stringify(value, null, 2)`.
fn js_stringify_2(v: &Value, depth: usize) -> String {
    let pad = "  ".repeat(depth + 1);
    let pad_end = "  ".repeat(depth);
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            if x.is_finite() {
                js::number_to_string(x)
            } else {
                "null".to_string()
            }
        }
        Value::String(s) => {
            let mut out = String::new();
            js_escape_string(s, &mut out);
            out
        }
        Value::Array(a) => {
            if a.is_empty() {
                return "[]".to_string();
            }
            let items: Vec<String> = a
                .iter()
                .map(|item| {
                    if js::is_undefined(item) {
                        "null".to_string()
                    } else {
                        js_stringify_2(item, depth + 1)
                    }
                })
                .collect();
            format!("[\n{pad}{}\n{pad_end}]", items.join(&format!(",\n{pad}")))
        }
        Value::Object(o) => {
            if js::is_undefined(v) {
                return "undefined".to_string();
            }
            if o.is_empty() {
                return "{}".to_string();
            }
            let items: Vec<String> = o
                .iter()
                .filter(|(_, val)| !js::is_undefined(val))
                .map(|(k, val)| {
                    let mut key = String::new();
                    js_escape_string(k, &mut key);
                    format!("{key}: {}", js_stringify_2(val, depth + 1))
                })
                .collect();
            format!("{{\n{pad}{}\n{pad_end}}}", items.join(&format!(",\n{pad}")))
        }
    }
}

fn node_io_error(e: &std::io::Error, op: &str, path: &str) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => format!("ENOENT: no such file or directory, {op} '{path}'"),
        std::io::ErrorKind::PermissionDenied => {
            format!("EACCES: permission denied, {op} '{path}'")
        }
        _ => {
            let raw = e.raw_os_error();
            if raw == Some(21) {
                format!("EISDIR: illegal operation on a directory, {op} '{path}'")
            } else if raw == Some(20) {
                format!("ENOTDIR: not a directory, {op} '{path}'")
            } else {
                e.to_string()
            }
        }
    }
}

/// Port of `writeDesktopWindowState` (`apps/desktop/src/windowState.ts:67`).
pub fn write_desktop_window_state(file_path: String, state: Value) -> Result<(), String> {
    if let Err(e) = std::fs::create_dir_all(dirname(&file_path)) {
        return Err(node_io_error(&e, "mkdir", &dirname(&file_path)));
    }
    let text = format!("{}\n", js_stringify_2(&state, 0));
    std::fs::write(&file_path, text).map_err(|e| node_io_error(&e, "open", &file_path))
}

/// `Math.min`/`Math.max` propagate NaN, unlike `f64::min`/`f64::max`.
fn jmin(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

fn jmax(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

fn intersection_area(left: &Value, right: &Value) -> f64 {
    let width = jmax(
        0.0,
        jmin(
            num_of(get(left, "x")) + num_of(get(left, "width")),
            num_of(get(right, "x")) + num_of(get(right, "width")),
        ) - jmax(num_of(get(left, "x")), num_of(get(right, "x"))),
    );
    let height = jmax(
        0.0,
        jmin(
            num_of(get(left, "y")) + num_of(get(left, "height")),
            num_of(get(right, "y")) + num_of(get(right, "height")),
        ) - jmax(num_of(get(left, "y")), num_of(get(right, "y"))),
    );
    width * height
}

fn clamp(value: f64, minimum: f64, maximum: f64) -> f64 {
    jmin(jmax(value, minimum), maximum)
}

/// Port of `resolveVisibleWindowBounds` (`apps/desktop/src/windowState.ts:91`).
pub fn resolve_visible_window_bounds(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err(
            "Cannot destructure property 'savedBounds' of 'input' as it is null.".to_string(),
        );
    }
    let saved_bounds = get(&input, "savedBounds").cloned().unwrap_or(Value::Null);
    let saved_bounds = if js::is_undefined(&saved_bounds) {
        Value::Null
    } else {
        saved_bounds
    };
    let work_areas: Vec<Value> = match get(&input, "displayWorkAreas") {
        None => {
            return Err(
                "undefined is not iterable (cannot read property Symbol(Symbol.iterator))"
                    .to_string(),
            );
        }
        Some(v) => match v {
            Value::Array(a) => a.clone(),
            Value::Null => {
                return Err(
                    "null is not iterable (cannot read property Symbol(Symbol.iterator))"
                        .to_string(),
                );
            }
            Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
            other => {
                let shown = if js::is_undefined(other) {
                    "undefined".to_string()
                } else {
                    match other {
                        Value::Bool(b) => b.to_string(),
                        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
                        _ => "[object Object]".to_string(),
                    }
                };
                return Err(format!(
                    "{shown} is not iterable (cannot read property Symbol(Symbol.iterator))"
                ));
            }
        },
    };

    let Some(first) = work_areas.first() else {
        return Ok(saved_bounds);
    };
    let mut target = first;
    let mut largest = intersection_area(&saved_bounds, target);
    for area_candidate in &work_areas[1..] {
        let area = intersection_area(&saved_bounds, area_candidate);
        if area > largest {
            target = area_candidate;
            largest = area;
        }
    }
    let target = target.clone();

    let width = jmin(
        num_of(get(&target, "width")),
        jmax(
            num_of(get(&input, "minimumWidth")),
            num_of(get(&saved_bounds, "width")),
        ),
    );
    let height = jmin(
        num_of(get(&target, "height")),
        jmax(
            num_of(get(&input, "minimumHeight")),
            num_of(get(&saved_bounds, "height")),
        ),
    );
    let centered_x =
        num_of(get(&target, "x")) + js::math_round((num_of(get(&target, "width")) - width) / 2.0);
    let centered_y =
        num_of(get(&target, "y")) + js::math_round((num_of(get(&target, "height")) - height) / 2.0);

    let zero = 0.0f64;
    let x = if largest == zero {
        centered_x
    } else {
        clamp(
            num_of(get(&saved_bounds, "x")),
            num_of(get(&target, "x")),
            num_of(get(&target, "x")) + num_of(get(&target, "width")) - width,
        )
    };
    let y = if largest == zero {
        centered_y
    } else {
        clamp(
            num_of(get(&saved_bounds, "y")),
            num_of(get(&target, "y")),
            num_of(get(&target, "y")) + num_of(get(&target, "height")) - height,
        )
    };

    let mut out = serde_json::Map::new();
    out.insert("x".to_string(), js::number(x));
    out.insert("y".to_string(), js::number(y));
    out.insert("width".to_string(), js::number(width));
    out.insert("height".to_string(), js::number(height));
    Ok(Value::Object(out))
}
