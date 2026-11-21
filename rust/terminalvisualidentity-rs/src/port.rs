//! The Rust port of `apps/web/src/terminalVisualIdentity.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Value, json};

fn is_js_ws(c: char) -> bool {
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

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

/// JS ToString used for object-key lookup (`obj[key]`) and `Map` keys.
fn to_key(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || js::is_undefined(x) {
                    String::new()
                } else {
                    to_key(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) if js::is_undefined(v) => "undefined".to_string(),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

/// `record[key]` where `record` may be any JS value: plain objects hit the
/// key, primitives give `undefined`, `null`/`undefined` raise TypeErrors.
fn index<'a>(record: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    match record {
        Value::Object(o) => {
            if js::is_undefined(record) {
                return Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ));
            }
            Ok(o.get(key))
        }
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ => Ok(None),
    }
}

/// `value ?? fallback` (nullish coalescing).
fn nullish(v: Option<&Value>) -> bool {
    v.is_none() || v.is_some_and(|x| x.is_null() || js::is_undefined(x))
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    index(v, key).ok().flatten()
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !js::is_undefined(v),
    }
}

fn js_error_iterable(record: &Value) -> String {
    if record.is_null() {
        "null is not iterable (cannot read property Symbol(Symbol.iterator))".to_string()
    } else if js::is_undefined(record) {
        "undefined is not iterable (cannot read property Symbol(Symbol.iterator))".to_string()
    } else {
        format!(
            "{} is not iterable (cannot read property Symbol(Symbol.iterator))",
            to_key(record)
        )
    }
}

fn to_string_list(v: Option<&Value>, ctx: &str) -> Result<Vec<String>, String> {
    match v {
        None => Err("Cannot read properties of undefined (reading 'map')".to_string()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'map')".to_string()),
        Some(Value::Array(a)) => {
            let mut out = Vec::with_capacity(a.len());
            for item in a {
                match item {
                    Value::String(s) => out.push(s.clone()),
                    _ if js::is_undefined(item) => {
                        return Err("id.trim is not a function".to_string());
                    }
                    _ => return Err(format!("{ctx} is not a function")),
                }
            }
            Ok(out)
        }
        Some(_) => Err(format!("{ctx} is not a function")),
    }
}

fn default_title_for_cli_kind(cli_kind: &Value) -> String {
    match cli_kind.as_str() {
        Some("codex") => "Codex CLI".to_string(),
        Some("claude") => "Claude Code".to_string(),
        _ => "Antigravity CLI".to_string(),
    }
}

fn icon_key_for(cli_kind: &Value) -> &'static str {
    match cli_kind.as_str() {
        Some("codex") => "openai",
        Some("claude") => "claude",
        Some("antigravity") => "antigravity",
        _ => "terminal",
    }
}

fn state_priority(state: Option<&Value>) -> i64 {
    match state.and_then(Value::as_str) {
        Some("attention") => 4,
        Some("running") => 3,
        Some("review") => 2,
        _ => 1,
    }
}

/// Port of `resolveTerminalVisualIdentityMap` (`apps/web/src/terminalVisualIdentity.ts:49`).
pub fn resolve_terminal_visual_identity_map(input: Value) -> Result<Value, String> {
    let running_ids = match index(&input, "runningTerminalIds") {
        Err(_) if input.is_null() => {
            return Err(
                "Cannot read properties of null (reading 'runningTerminalIds')".to_string(),
            );
        }
        Err(e) => return Err(e),
        Ok(v) => v,
    };
    let running_list = to_string_list(running_ids, "input.runningTerminalIds.map")?;
    let running_set: Vec<String> = running_list
        .iter()
        .map(|id| js_trim(id).to_string())
        .filter(|id| !id.is_empty())
        .collect();

    let terminal_ids = index(&input, "terminalIds")?;
    let ids: Vec<Value> = match terminal_ids {
        None => {
            return Err("Cannot read properties of undefined (reading 'map')".to_string());
        }
        Some(Value::Null) => {
            return Err("Cannot read properties of null (reading 'map')".to_string());
        }
        Some(Value::Array(a)) => a.clone(),
        Some(_) => return Err("input.terminalIds.map is not a function".to_string()),
    };

    let attention_map = prop(&input, "terminalAttentionStatesById").unwrap_or(&Value::Null);
    let cli_kinds = prop(&input, "terminalCliKindsById").unwrap_or(&Value::Null);
    let labels = prop(&input, "terminalLabelsById").unwrap_or(&Value::Null);
    let overrides = prop(&input, "terminalTitleOverridesById").unwrap_or(&Value::Null);

    let mut entries: Vec<(Value, Value)> = Vec::new();
    for (index_i, terminal_id) in ids.iter().enumerate() {
        let key = to_key(terminal_id);

        let attention_state = index(attention_map, &key)?.cloned();
        let state = if attention_state.as_ref().and_then(Value::as_str) == Some("attention") {
            "attention"
        } else if matches!(terminal_id, Value::String(s) if running_set.iter().any(|r| r == s)) {
            "running"
        } else if attention_state.as_ref().and_then(Value::as_str) == Some("review") {
            "review"
        } else {
            "idle"
        };

        let cli_kind = match index(cli_kinds, &key)? {
            Some(v) if !v.is_null() && !js::is_undefined(v) => v.clone(),
            _ => Value::Null,
        };

        let title_raw = {
            let from_override = index(overrides, &key)?;
            if nullish(from_override) {
                index(labels, &key)?.cloned()
            } else {
                from_override.cloned()
            }
        };
        let trimmed = match &title_raw {
            None => None,
            Some(v) if v.is_null() || js::is_undefined(v) => None,
            Some(Value::String(s)) => Some(js_trim(s).to_string()),
            Some(_) => return Err("input.title?.trim is not a function".to_string()),
        };
        let title = match trimmed {
            Some(t) if !t.is_empty() => t,
            _ => {
                if truthy(&cli_kind) {
                    default_title_for_cli_kind(&cli_kind)
                } else {
                    format!("Terminal {}", index_i + 1)
                }
            }
        };

        entries.push((
            terminal_id.clone(),
            json!({
                "cliKind": cli_kind,
                "iconKey": icon_key_for(&cli_kind),
                "state": state,
                "title": title,
            }),
        ));
    }

    // `new Map(...)`: later duplicate keys overwrite the value but keep the
    // first insertion position (SameValueZero on scalar keys).
    let mut merged: Vec<(Value, Value)> = Vec::new();
    for (k, v) in entries {
        if let Some(slot) = merged.iter_mut().find(|(ek, _)| json_scalar_eq(ek, &k)) {
            slot.1 = v;
        } else {
            merged.push((k, v));
        }
    }
    Ok(json!({ "$map": merged }))
}

fn json_scalar_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64() == y.as_f64()
                || (x.as_f64().is_some_and(|n| n.is_nan())
                    && y.as_f64().is_some_and(|n| n.is_nan()))
        }
        _ => false,
    }
}

/// Port of `selectRepresentativeTerminalVisualIdentity` (`apps/web/src/terminalVisualIdentity.ts:79`).
pub fn select_representative_terminal_visual_identity(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'activeTerminalId')".to_string());
    }
    let active = prop(&input, "activeTerminalId");
    let terminal_ids = prop(&input, "terminalIds");

    let fallback: Option<Value> = {
        let use_active = match active {
            Some(a) if truthy(a) => match terminal_ids {
                None => {
                    return Err(
                        "Cannot read properties of undefined (reading 'includes')".to_string()
                    );
                }
                Some(Value::Null) => {
                    return Err("Cannot read properties of null (reading 'includes')".to_string());
                }
                Some(Value::Array(ids)) => ids.iter().any(|id| json_scalar_eq(id, a)),
                Some(_) => return Err("input.terminalIds.includes is not a function".to_string()),
            },
            _ => false,
        };
        if use_active {
            active.cloned()
        } else {
            match terminal_ids {
                None => return Err("Cannot read properties of undefined (reading '0')".to_string()),
                Some(Value::Null) => {
                    return Err("Cannot read properties of null (reading '0')".to_string());
                }
                Some(Value::Array(ids)) => ids.first().cloned(),
                Some(other) => {
                    return Err(js_error_iterable(other));
                }
            }
        }
    };

    let Some(fallback_id) = fallback else {
        return Ok(Value::Null);
    };
    if !truthy(&fallback_id) {
        return Ok(Value::Null);
    }

    let ids_list: &[Value] = match terminal_ids {
        Some(Value::Array(ids)) => ids,
        _ => &[],
    };

    let map = prop(&input, "terminalVisualIdentityById");
    let map_get = |key: &Value| -> Result<Option<Value>, String> {
        match map {
            None => Err("input.terminalVisualIdentityById.get is not a function".to_string()),
            Some(Value::Null) => Err("Cannot read properties of null (reading 'get')".to_string()),
            Some(Value::Object(o)) => {
                if let Some(Value::Array(pairs)) = o.get("$map") {
                    for pair in pairs {
                        if let Value::Array(kv) = pair
                            && kv.len() == 2
                            && json_scalar_eq(&kv[0], key)
                        {
                            return Ok(Some(kv[1].clone()));
                        }
                    }
                    Ok(None)
                } else {
                    Err("input.terminalVisualIdentityById.get is not a function".to_string())
                }
            }
            Some(_) => Err("input.terminalVisualIdentityById.get is not a function".to_string()),
        }
    };

    let mut representative = fallback_id;
    for terminal_id in ids_list {
        let current = map_get(&representative)?
            .and_then(|id| prop(&id, "state").cloned())
            .unwrap_or(json!("idle"));
        let next = map_get(terminal_id)?
            .and_then(|id| prop(&id, "state").cloned())
            .unwrap_or(json!("idle"));
        if state_priority(Some(&next)) > state_priority(Some(&current)) {
            representative = terminal_id.clone();
        }
    }

    match map_get(&representative)? {
        Some(identity) => Ok(json!({
            "terminalId": representative,
            "identity": identity,
        })),
        None => Ok(Value::Null),
    }
}
