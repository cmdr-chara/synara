//! The Rust port of `apps/server/src/codexTurnInput.ts`.
//!
//! Reconstructed clean-room from cases.jsonl (13 cases, 100% line coverage) and the
//! exported signature: build one turn's input items — text first when `input` is
//! truthy, then image/localImage attachments, then skills, then mentions.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

/// JS truthiness for recorded `input` values.
fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Object(_) if is_undefined(v) => false,
        _ => true,
    }
}

/// Object property read; `None` = JS `undefined`.
fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    match v {
        Value::Object(m) if !is_undefined(v) => m.get(name),
        _ => None,
    }
}

/// Port of `buildCodexTurnInput` (`apps/server/src/codexTurnInput.ts:13`).
pub fn build_codex_turn_input(input: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'input')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'input')".into());
    }
    let mut out: Vec<Value> = Vec::new();

    if let Some(text) = field(&input, "input")
        && js_truthy(text)
    {
        out.push(json!({"type": "text", "text": text, "text_elements": []}));
    }

    if let Some(Value::Array(items)) = field(&input, "attachments") {
        for a in items {
            if let Some("image") | Some("localImage") = field(a, "type").and_then(Value::as_str) {
                out.push(a.clone());
            }
        }
    }

    for (key, kind) in [("skills", "skill"), ("mentions", "mention")] {
        if let Some(Value::Array(items)) = field(&input, key) {
            for item in items {
                let mut obj = Map::with_capacity(3);
                obj.insert("type".into(), json!(kind));
                if let Some(name) = field(item, "name") {
                    obj.insert("name".into(), name.clone());
                }
                if let Some(path) = field(item, "path") {
                    obj.insert("path".into(), path.clone());
                }
                out.push(Value::Object(obj));
            }
        }
    }

    Ok(out)
}
