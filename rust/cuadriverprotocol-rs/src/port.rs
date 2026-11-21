//! The Rust port of `packages/shared/src/cuaDriverProtocol.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::{js, utf16};

const CUA_SHIELD_MAX_EXTENT: f64 = 32_768.0;
const CUA_SHIELD_MAX_COORDINATE: f64 = 1_000_000.0;
const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

fn is_safe_integer(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0 && n.abs() <= MAX_SAFE_INTEGER
}

/// `value.length` for a string — UTF-16 code units.
fn utf16_len(s: &str) -> usize {
    utf16::units(s).len()
}

/// `String.prototype.slice(0, n)` in UTF-16 units.
fn utf16_slice0(s: &str, n: usize) -> String {
    utf16::slice(&utf16::units(s), 0, n)
}

/// Unicode general categories `Cc` and `Cf` (per `/[\p{Cc}\p{Cf}]/gu`).
fn is_control_or_format(c: char) -> bool {
    let cp = c as u32;
    // Cc
    if cp <= 0x1f || (0x7f..=0x9f).contains(&cp) {
        return true;
    }
    // Cf
    matches!(cp,
        0x00ad | 0x0600..=0x0605 | 0x061c | 0x06dd | 0x070f | 0x0890..=0x0891
        | 0x08e2 | 0x180e | 0x200b..=0x200f | 0x202a..=0x202e | 0x2060..=0x2064
        | 0x2066..=0x206f | 0xfeff | 0xfff9..=0xfffb | 0x110bd | 0x110cd
        | 0x13430..=0x13455 | 0x1bca0..=0x1bca3 | 0x1d173..=0x1d17a | 0xe0001
        | 0xe0020..=0xe007f)
}

/// Port of `cuaComputerTaskKey` (`packages/shared/src/cuaDriverProtocol.ts:110`).
pub fn cua_computer_task_key(task: serde_json::Value) -> Result<String, String> {
    if task.is_null() {
        return Err("Cannot read properties of null (reading 'threadId')".to_string());
    }
    if js::is_undefined(&task) {
        return Err("Cannot read properties of undefined (reading 'threadId')".to_string());
    }
    let thread_id = task.get("threadId").cloned().unwrap_or_else(js::undefined);
    let turn_id = task
        .get("turnId")
        .cloned()
        .filter(|v| !v.is_null() && !js::is_undefined(v))
        .unwrap_or(Value::Null);
    // Array elements that are `undefined` serialize as `null`.
    let array_slot = |v: Value| {
        if js::is_undefined(&v) { Value::Null } else { v }
    };
    Ok(jscore::json_stringify(&Value::Array(vec![
        array_slot(thread_id),
        array_slot(turn_id),
    ])))
}

fn shield_coordinate(value: Option<&Value>) -> Option<f64> {
    let n = js::as_number(value?)?;
    if n.is_finite() && n.abs() <= CUA_SHIELD_MAX_COORDINATE {
        Some(n)
    } else {
        None
    }
}

fn shield_extent(value: Option<&Value>) -> Option<f64> {
    let n = js::as_number(value?)?;
    if n.is_finite() && n > 0.0 && n <= CUA_SHIELD_MAX_EXTENT {
        Some(n)
    } else {
        None
    }
}

/// Port of `parseCuaShieldArgs` (`packages/shared/src/cuaDriverProtocol.ts:154`).
pub fn parse_cua_shield_args(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let undef = js::undefined();
    if !jscore::js_truthy(&value) || !value.is_object() && !value.is_array() {
        return Ok(undef);
    }
    let get = |key: &str| value.get(key);
    if get("action") == Some(&Value::String("release_all".to_string())) {
        let mut out = Map::new();
        out.insert(
            "action".to_string(),
            Value::String("release_all".to_string()),
        );
        return Ok(Value::Object(out));
    }
    let shield_id = get("shield_id").and_then(Value::as_str);
    let valid_id = shield_id.is_some_and(|id| {
        !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    });
    let Some(shield_id) = shield_id.filter(|_| valid_id) else {
        return Ok(undef);
    };
    if get("action") == Some(&Value::String("release".to_string())) {
        let mut out = Map::new();
        out.insert("action".to_string(), Value::String("release".to_string()));
        out.insert("shieldId".to_string(), Value::String(shield_id.to_string()));
        return Ok(Value::Object(out));
    }
    if get("action") != Some(&Value::String("engage".to_string())) {
        return Ok(undef);
    }
    let frame = get("frame");
    let x = shield_coordinate(frame.and_then(|f| f.get("x")));
    let y = shield_coordinate(frame.and_then(|f| f.get("y")));
    let width = shield_extent(frame.and_then(|f| f.get("width")));
    let height = shield_extent(frame.and_then(|f| f.get("height")));
    let window_id = get("window_id").and_then(js::as_number);
    let pid = get("pid").and_then(js::as_number);
    let (Some(x), Some(y), Some(width), Some(height)) = (x, y, width, height) else {
        return Ok(undef);
    };
    let (Some(window_id), Some(pid)) = (window_id, pid) else {
        return Ok(undef);
    };
    if !is_safe_integer(window_id)
        || window_id <= 0.0
        || window_id > 0xffffffffu64 as f64
        || !is_safe_integer(pid)
        || pid <= 0.0
        || pid > 0x7fffffff_u64 as f64
    {
        return Ok(undef);
    }
    let label = get("label").and_then(Value::as_str).and_then(|l| {
        let cleaned: String = l.chars().filter(|c| !is_control_or_format(*c)).collect();
        let trimmed = cleaned.trim_matches(jscore::is_space);
        let sliced = utf16_slice0(trimmed, 160);
        if sliced.is_empty() {
            None
        } else {
            Some(sliced)
        }
    });
    let mut frame_obj = Map::new();
    frame_obj.insert("x".to_string(), js::number(x));
    frame_obj.insert("y".to_string(), js::number(y));
    frame_obj.insert("width".to_string(), js::number(width));
    frame_obj.insert("height".to_string(), js::number(height));
    let mut out = Map::new();
    out.insert("action".to_string(), Value::String("engage".to_string()));
    out.insert("shieldId".to_string(), Value::String(shield_id.to_string()));
    out.insert("frame".to_string(), Value::Object(frame_obj));
    out.insert("windowId".to_string(), js::number(window_id));
    out.insert("pid".to_string(), js::number(pid));
    if let Some(label) = label {
        out.insert("label".to_string(), Value::String(label));
    }
    Ok(Value::Object(out))
}

/// Port of `parseCuaComputerTask` (`packages/shared/src/cuaDriverProtocol.ts:204`).
pub fn parse_cua_computer_task(value: serde_json::Value) -> Result<serde_json::Value, String> {
    let undef = js::undefined();
    if !jscore::js_truthy(&value) || !value.is_object() && !value.is_array() {
        return Ok(undef);
    }
    let identifier = |v: Option<&Value>| {
        v.and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty() && utf16_len(s) <= 256)
    };
    let task = &value;
    if !identifier(task.get("threadId")) {
        return Ok(undef);
    }
    let turn_id = task.get("turnId");
    if turn_id.is_some_and(|v| !js::is_undefined(v)) && !identifier(turn_id) {
        return Ok(undef);
    }
    let mut out = Map::new();
    out.insert(
        "threadId".to_string(),
        task.get("threadId").cloned().unwrap_or(Value::Null),
    );
    if let Some(t) = turn_id.filter(|v| !js::is_undefined(v)) {
        out.insert("turnId".to_string(), t.clone());
    }
    if let Some(label) = task.get("label").and_then(Value::as_str) {
        out.insert("label".to_string(), Value::String(utf16_slice0(label, 160)));
    }
    Ok(Value::Object(out))
}

/// Port of `cuaCleanupAcknowledged` (`packages/shared/src/cuaDriverProtocol.ts:361`).
pub fn cua_cleanup_acknowledged(
    result: serde_json::Value,
    expected_pid: serde_json::Value,
) -> Result<bool, String> {
    if js::is_undefined(&expected_pid) {
        return Ok(false);
    }
    // `cleanup?.pid === expectedPid` — strict equality.
    let pid = result.get("pid");
    let pid_equals = match (pid, &expected_pid) {
        (Some(p), e) => match (js::as_number(p), js::as_number(e)) {
            (Some(a), Some(b)) => a == b,
            _ => p == e,
        },
        _ => false,
    };
    Ok(pid_equals
        && result.get("input_admission_closed") == Some(&Value::Bool(true))
        && result.get("cleanup_complete") == Some(&Value::Bool(true))
        && result.get("pending_input").and_then(js::as_number) == Some(0.0))
}
