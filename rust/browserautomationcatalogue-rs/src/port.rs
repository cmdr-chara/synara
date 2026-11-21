//! The Rust port of `packages/shared/src/browserAutomationCatalogue.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

const DOC_KEYS: [&str; 2] = ["examples", "title"];
const CHILD_KEYS: [&str; 5] = [
    "properties",
    "patternProperties",
    "$defs",
    "definitions",
    "dependentSchemas",
];
const LITERAL_KEYS: [&str; 5] = ["enum", "const", "default", "required", "dependentRequired"];
const OBJECT_KEYWORDS: [&str; 4] = [
    "properties",
    "patternProperties",
    "additionalProperties",
    "unevaluatedProperties",
];

fn is_object(v: &Value) -> bool {
    matches!(v, Value::Object(_))
}

/// JS object key enumeration: canonical array indices (uint32 < 2^32-1, written in
/// decimal) enumerate first, in numeric order; other keys keep insertion order.
fn array_index(key: &str) -> Option<u32> {
    if key.is_empty() || key == "4294967295" {
        return None;
    }
    let n: u32 = key.parse().ok()?;
    if n.to_string() == key { Some(n) } else { None }
}

fn hoist_index_keys(keys: Vec<&String>) -> Vec<&String> {
    let mut indices: Vec<(u32, &String)> = keys
        .iter()
        .filter_map(|k| array_index(k).map(|n| (n, *k)))
        .collect();
    indices.sort_by_key(|(n, _)| *n);
    indices
        .into_iter()
        .map(|(_, k)| k)
        .chain(keys.into_iter().filter(|k| array_index(k).is_none()))
        .collect()
}

/// Sorts keys like JS `Array.prototype.sort()` — by UTF-16 code units — then applies
/// the integer-index hoisting `Object.fromEntries` gives the resulting object.
fn sort_keys(map: &Map<String, Value>) -> Vec<&String> {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by(|a, b| {
        let ua: Vec<u16> = a.encode_utf16().collect();
        let ub: Vec<u16> = b.encode_utf16().collect();
        ua.cmp(&ub)
    });
    hoist_index_keys(keys)
}

fn canonicalize(value: &Value) -> Result<Value, String> {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => Ok(value.clone()),
        Value::Number(_) => Ok(value.clone()),
        Value::Array(items) => items
            .iter()
            .map(canonicalize)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(map) => {
            // Wire markers stand for JS values that are not JSON.
            if map.len() == 1 {
                if let Some(n) = map.get("$num").and_then(Value::as_str) {
                    if n == "-0" {
                        return Ok(value.clone());
                    }
                    return Err("Canonical JSON numbers must be finite".to_string());
                }
                if map.contains_key("$undefined") {
                    return Err("Value is not canonical JSON".to_string());
                }
                if map.contains_key("$utf16") {
                    return Ok(value.clone());
                }
            }
            let mut out = Map::new();
            for key in sort_keys(map) {
                // INFALLIBLE: keys come from the same map
                let child = map.get(key).expect("key from map");
                out.insert(key.clone(), canonicalize(child)?);
            }
            Ok(Value::Object(out))
        }
    }
}

/// Port of `canonicalizeJson` (`packages/shared/src/browserAutomationCatalogue.ts:318`).
pub fn canonicalize_json(value: serde_json::Value) -> Result<serde_json::Value, String> {
    canonicalize(&value)
}

fn js_number(n: &serde_json::Number) -> String {
    if let Some(f) = n.as_f64()
        && n.as_i64().is_none()
        && n.as_u64().is_none()
    {
        return crate::js::number_to_string(f);
    }
    n.to_string()
}

/// JS `JSON.stringify` over a canonical JSON value.
fn js_stringify(value: &Value, out: &mut String) -> Result<(), String> {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&js_number(n)),
        Value::String(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\u{8}' => out.push_str("\\b"),
                    '\t' => out.push_str("\\t"),
                    '\n' => out.push_str("\\n"),
                    '\u{c}' => out.push_str("\\f"),
                    '\r' => out.push_str("\\r"),
                    c if (c as u32) < 0x20 => {
                        out.push_str(&format!("\\u{:04x}", c as u32));
                    }
                    c => out.push(c),
                }
            }
            out.push('"');
        }
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                js_stringify(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            if map.len() == 1 {
                if let Some(u) = map.get("$utf16").and_then(Value::as_array) {
                    out.push('"');
                    for u in u {
                        let unit = u.as_u64().unwrap_or(0) as u32;
                        if (0xd800..0xe000).contains(&unit) {
                            out.push_str(&format!("\\u{unit:04x}"));
                        } else if let Some(c) = char::from_u32(unit) {
                            match c {
                                '"' => out.push_str("\\\""),
                                '\\' => out.push_str("\\\\"),
                                c if (c as u32) < 0x20 => {
                                    out.push_str(&format!("\\u{:04x}", c as u32));
                                }
                                c => out.push(c),
                            }
                        }
                    }
                    out.push('"');
                    return Ok(());
                }
                if let Some(n) = map.get("$num").and_then(Value::as_str) {
                    // JSON.stringify(-0) is "0"; NaN/Infinity serialize as null, which
                    // canonicalizeJson already rejects before we get here.
                    if n == "-0" {
                        out.push('0');
                        return Ok(());
                    }
                    out.push_str("null");
                    return Ok(());
                }
                if map.contains_key("$undefined") {
                    return Ok(());
                }
            }
            out.push('{');
            let mut first = true;
            for (key, child) in map {
                if !first {
                    out.push(',');
                }
                first = false;
                js_stringify(&Value::String(key.clone()), out)?;
                out.push(':');
                js_stringify(child, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

/// Port of `stableJsonStringify` (`packages/shared/src/browserAutomationCatalogue.ts:322`).
pub fn stable_json_stringify(value: serde_json::Value) -> Result<String, String> {
    let canonical = canonicalize(&value)?;
    let mut out = String::new();
    js_stringify(&canonical, &mut out)?;
    Ok(out)
}

fn flatten_nested_any_of(branches: &[Value]) -> Vec<Value> {
    let mut out = Vec::new();
    for branch in branches {
        let spread = matches!(branch, Value::Object(map)
            if map.len() == 1 && map.get("anyOf").is_some_and(Value::is_array));
        if spread {
            if let Some(Value::Array(inner)) = branch.get("anyOf") {
                out.extend(inner.iter().cloned());
            }
        } else {
            out.push(branch.clone());
        }
    }
    out
}

fn merge_literal_union_branches(branches: &[Value]) -> Vec<Value> {
    let mut enum_values_by_type: Vec<(String, Vec<Value>)> = Vec::new();
    let mut remaining: Vec<Value> = Vec::new();

    for branch in branches {
        let literal = match branch {
            Value::Object(map)
                if map.get("type").is_some_and(Value::is_string)
                    && map.get("enum").is_some_and(Value::is_array)
                    && map.keys().all(|k| k == "enum" || k == "type") =>
            {
                Some((
                    // INFALLIBLE: checked by the guard above
                    branch["type"].as_str().expect("string type").to_string(),
                    // INFALLIBLE: checked by the guard above
                    branch["enum"].as_array().expect("array enum").clone(),
                ))
            }
            _ => None,
        };
        match literal {
            Some((ty, values)) => match enum_values_by_type.iter_mut().find(|(t, _)| *t == ty) {
                Some((_, list)) => list.extend(values),
                None => enum_values_by_type.push((ty, values)),
            },
            None => remaining.push(branch.clone()),
        }
    }

    for (ty, values) in enum_values_by_type {
        // JS `new Set(values)`: SameValueZero dedupe; objects compare by identity,
        // so only scalars can collapse.
        let mut seen: Vec<Value> = Vec::new();
        let mut unique: Vec<Value> = Vec::new();
        for v in values {
            let scalar = matches!(
                v,
                Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
            );
            if scalar && seen.iter().any(|s| s == &v) {
                continue;
            }
            seen.push(v.clone());
            unique.push(v);
        }
        let mut obj = Map::new();
        obj.insert("enum".to_string(), Value::Array(unique));
        obj.insert("type".to_string(), Value::String(ty));
        remaining.push(Value::Object(obj));
    }
    remaining
}

fn unwrap_single_all_of(schema: Map<String, Value>) -> Map<String, Value> {
    let all_of_len = schema.get("allOf").and_then(Value::as_array).map(Vec::len);
    if all_of_len != Some(1) {
        return schema;
    }
    let only_branch = match schema["allOf"].as_array().and_then(|a| a.first()) {
        Some(Value::Object(m)) => m.clone(),
        _ => return schema,
    };
    if OBJECT_KEYWORDS
        .iter()
        .any(|key| schema.contains_key(*key) || only_branch.contains_key(*key))
    {
        return schema;
    }
    let mut outer = schema;
    outer.remove("allOf");
    if only_branch.keys().any(|key| outer.contains_key(key)) {
        let mut restored = outer;
        restored.insert(
            "allOf".to_string(),
            Value::Array(vec![Value::Object(only_branch.clone())]),
        );
        return restored;
    }
    for (key, value) in only_branch {
        outer.insert(key.clone(), value.clone());
    }
    outer
}

fn compact_schema(value: &Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.iter().map(compact_schema).collect()),
        Value::Object(map) => {
            let mut compacted_entries = Map::new();
            for (key, child) in map {
                if DOC_KEYS.contains(&key.as_str()) {
                    continue;
                }
                let projected = if CHILD_KEYS.contains(&key.as_str()) && is_object(child) {
                    // Keys under `properties` are parameter names, not schema keywords.
                    let mut inner = Map::new();
                    if let Value::Object(names) = child {
                        for (name, schema) in names {
                            inner.insert(name.clone(), compact_schema(schema));
                        }
                    }
                    Value::Object(inner)
                } else if LITERAL_KEYS.contains(&key.as_str()) {
                    child.clone()
                } else {
                    compact_schema(child)
                };
                compacted_entries.insert(key.clone(), projected);
            }
            let compacted = unwrap_single_all_of(compacted_entries);
            let any_of_is_array = compacted.get("anyOf").is_some_and(Value::is_array);
            if !any_of_is_array {
                return Value::Object(compacted);
            }
            let branches = compacted["anyOf"].as_array().cloned().unwrap_or_default();
            let any_of = merge_literal_union_branches(&flatten_nested_any_of(&branches));
            if compacted.len() == 1 && any_of.len() == 1 {
                return any_of.into_iter().next().unwrap_or(Value::Null);
            }
            let mut out = compacted;
            out.insert("anyOf".to_string(), Value::Array(any_of));
            Value::Object(out)
        }
        other => other.clone(),
    }
}

/// Port of `compactToolInputSchema` (`packages/shared/src/browserAutomationCatalogue.ts:398`).
pub fn compact_tool_input_schema(value: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(compact_schema(&value))
}

/// Port of `projectBrowserToolDefinitions`
/// (`packages/shared/src/browserAutomationCatalogue.ts:454`).
///
/// `projectSchema` calls `Schema.toJsonSchemaDocument`, which needs a live Effect Schema
/// value; those never cross the wire, so every non-empty input raises like the original.
pub fn project_browser_tool_definitions(
    definitions: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    if definitions.is_empty() {
        return Ok(vec![]);
    }
    let input = definitions[0].get("input");
    match input {
        Some(Value::Null) => Err("Cannot read properties of null (reading 'ast')".to_string()),
        _ => Err("Cannot read properties of undefined (reading 'encoding')".to_string()),
    }
}
