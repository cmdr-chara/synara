//! The Rust port of `apps/web/src/terminalStateStore.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

/// `v.name` — throws on null/undefined receivers like JS property access.
fn member<'a>(v: &'a Value, name: &str) -> Result<&'a Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(name).unwrap_or(&NULL),
        _ => &NULL,
    })
}

static NULL: Value = Value::Null;
static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();

fn undef() -> &'static Value {
    UNDEF.get_or_init(crate::js::undefined)
}

/// `ToPropertyKey(v)` — the string a bracket lookup on `v` coerces to.
fn key_of(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(key_of).collect::<Vec<_>>().join(","),
        Value::Object(_) => {
            if crate::js::is_undefined(v) {
                return "undefined".to_string();
            }
            if let Some(x) = crate::js::as_number(v) {
                return crate::js::number_to_string(x);
            }
            if let Some(units) = crate::js::string_units(v) {
                return String::from_utf16_lossy(&units);
            }
            "[object Object]".to_string()
        }
    }
}

/// `v.length` — strings their UTF-16 length, arrays theirs, objects the
/// `length` property, primitives undefined.
fn length_of(v: &Value) -> &Value {
    match v {
        Value::Array(a) => Box::leak(Box::new(crate::js::number(a.len() as f64))),
        Value::String(s) => Box::leak(Box::new(crate::js::number(s.encode_utf16().count() as f64))),
        Value::Object(m) => {
            if let Some(units) = crate::js::string_units(v) {
                return Box::leak(Box::new(crate::js::number(units.len() as f64)));
            }
            m.get("length").unwrap_or_else(|| undef())
        }
        _ => undef(),
    }
}

/// `map[key]` — objects their own property, arrays/strings indexed elements,
/// anything else undefined. Nullish receivers fail the JS way.
fn bracket<'a>(map: &'a Value, key: &Value) -> Result<&'a Value, String> {
    if map.is_null() || crate::js::is_undefined(map) {
        let which = if map.is_null() { "null" } else { "undefined" };
        // V8 prints no property suffix for array keys, and `#<Object>` for
        // plain objects.
        return Err(match key {
            Value::Array(_) => format!("Cannot read properties of {which}"),
            Value::Object(o)
                if !crate::js::is_undefined(key)
                    && o.get("$num").is_none()
                    && o.get("$utf16").is_none() =>
            {
                format!("Cannot read properties of {which} (reading '#<Object>')")
            }
            _ => format!(
                "Cannot read properties of {which} (reading '{}')",
                key_of(key)
            ),
        });
    }
    let k = key_of(key);
    Ok(match map {
        Value::Object(m) => m.get(&k).unwrap_or_else(|| undef()),
        Value::Array(a) => crate::js::array_index_key(&k)
            .and_then(|i| a.get(i as usize))
            .unwrap_or_else(|| undef()),
        Value::String(s) => crate::js::array_index_key(&k)
            .and_then(|i| s.encode_utf16().nth(i as usize))
            .map(|u| Box::leak(Box::new(crate::js::utf16_string(&[u]))) as &Value)
            .unwrap_or_else(|| undef()),
        _ => undef(),
    })
}

/// The settled terminal state a missing thread resolves to.
fn settled_state() -> Value {
    serde_json::json!({
        "entryPoint": "chat",
        "terminalOpen": false,
        "hasSession": false,
        "presentationMode": "workspace",
        "workspaceLayout": "both",
        "workspaceActiveTab": "terminal",
        "terminalIds": ["default"],
        "terminalLabelsById": {"default": "Terminal 1"},
        "terminalTitleOverridesById": {},
        "terminalCliKindsById": {},
        "terminalAttentionStatesById": {},
        "runningTerminalIds": [],
        "activeTerminalId": "default",
    })
}

/// Port of `sanitizePersistedTerminalStateByThreadId` (`apps/web/src/terminalStateStore.ts:347`).
///
/// Iterates the persisted record's entries; each state's `terminalIds` is
/// mapped, so a state missing that field fails on `.map`.
pub fn sanitize_persisted_terminal_state_by_thread_id(
    terminal_state_by_thread_id: Option<Value>,
) -> Result<Value, String> {
    let input = terminal_state_by_thread_id.unwrap_or(Value::Null);
    let mut out = Map::new();
    for (key, state) in crate::js::entries(&input) {
        let ids = member(&state, "terminalIds")?;
        match ids {
            Value::Array(list) => {
                let _: Vec<Value> = list.clone();
            }
            Value::Null => {
                return Err("Cannot read properties of null (reading 'map')".to_string());
            }
            v if crate::js::is_undefined(v) => {
                return Err("Cannot read properties of undefined (reading 'map')".to_string());
            }
            _ => return Err("state.terminalIds.map is not a function".to_string()),
        }
        out.insert(key, state);
    }
    Ok(Value::Object(out))
}

/// Port of `selectThreadTerminalState` (`apps/web/src/terminalStateStore.ts:792`).
///
/// `threadId.length === 0` settles early; otherwise `map[threadId]` is looked
/// up and a missing entry falls back to the settled state.
pub fn select_thread_terminal_state(
    terminal_state_by_thread_id: Value,
    thread_id: Value,
) -> Result<Value, String> {
    member(&thread_id, "length")?;
    let len = length_of(&thread_id);
    if crate::js::js_eq(Some(len), Some(&crate::js::number(0.0))) {
        return Ok(settled_state());
    }
    let entry = bracket(&terminal_state_by_thread_id, &thread_id)?;
    Ok(if nullish(entry) {
        settled_state()
    } else {
        entry.clone()
    })
}
