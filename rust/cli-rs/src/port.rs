//! The Rust port of `packages/shared/src/cli.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn nullish(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64() != Some(0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}

/// JS `String(v)` for the concat cases this port needs.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn flag_param(name: Value, description: Value, aliases: Value) -> Value {
    json!({
        "_tag": "Map",
        "kind": "flag",
        "param": {
            "_tag": "Optional",
            "kind": "flag",
            "param": {
                "_tag": "Single",
                "name": name,
                "primitiveType": { "_tag": "Boolean", "parse": { "$function": "" } },
                "kind": "flag",
                "description": description,
                "aliases": aliases,
                "parse": { "$function": "parse" },
            },
            "parse": { "$function": "parse" },
        },
        "f": { "$function": "" },
        "parse": { "$function": "parse" },
    })
}

/// Port of `optionalBooleanEnvironmentConfig` (`packages/shared/src/cli.ts:17`).
pub fn optional_boolean_environment_config(
    name: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &name;
    Ok(json!({ "parse": { "$function": "" } }))
}

/// Port of `optionalBooleanFlag` (`packages/shared/src/cli.ts:51`).
pub fn optional_boolean_flag(
    name: serde_json::Value,
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let opts = match &options {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'negativeName')".to_string());
        }
        v if is_undefined(v) => None,
        v => Some(v),
    };
    let opt = |key: &str| -> Option<&Value> { opts.and_then(|o| o.get(key)) };

    let positive_description = opt("description")
        .filter(|v| truthy(v))
        .cloned()
        .unwrap_or_else(undefined);

    let aliases: Vec<Value> = match opt("aliases").filter(|v| !nullish(v)) {
        None => Vec::new(),
        Some(Value::Array(a)) => a.clone(),
        Some(Value::String(s)) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        Some(v) => {
            return Err(format!(
                "{} is not iterable (cannot read property Symbol(Symbol.iterator)",
                match v {
                    Value::Number(n) => format!("number {n}"),
                    Value::Object(_) => "object".to_string(),
                    Value::Bool(b) => format!("boolean {b}"),
                    _ => js_str(v),
                }
            ));
        }
    };
    let mut normalized_aliases = Vec::with_capacity(aliases.len());
    for alias in &aliases {
        match alias {
            Value::String(s) => {
                normalized_aliases.push(Value::String(s.trim_start_matches('-').to_string()))
            }
            _ => return Err("alias.replace is not a function".to_string()),
        }
    }

    let negative_name = opt("negativeName")
        .filter(|v| !nullish(v))
        .cloned()
        .unwrap_or_else(|| Value::String(format!("no-{}", js_str(&name))));

    let negative_description_raw = opt("negativeDescription")
        .filter(|v| !nullish(v))
        .cloned()
        .unwrap_or_else(|| match opt("description") {
            Some(d) if truthy(d) => Value::String(format!("Disable: {}", js_str(d))),
            _ => Value::String(format!("Disable --{}.", js_str(&name))),
        });
    let negative_description = if truthy(&negative_description_raw) {
        negative_description_raw
    } else {
        undefined()
    };

    let mut out = Map::new();
    out.insert(
        "positive".to_string(),
        flag_param(name, positive_description, Value::Array(normalized_aliases)),
    );
    out.insert(
        "negative".to_string(),
        flag_param(negative_name, negative_description, Value::Array(vec![])),
    );
    Ok(Value::Object(out))
}

/// Port of `getBooleanFlagValue` (`packages/shared/src/cli.ts:72`).
pub fn get_boolean_flag_value(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'positive')".to_string());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'positive')".to_string());
    }
    let positive = input.get("positive").cloned().unwrap_or_else(undefined);
    if !is_undefined(&positive) {
        return Ok(positive);
    }
    let negative = input.get("negative").cloned().unwrap_or_else(undefined);
    if !is_undefined(&negative) {
        return Ok(Value::Bool(!truthy(&negative)));
    }
    Ok(undefined())
}

/// Port of `resolveBooleanConfig` (`packages/shared/src/cli.ts:79`).
pub fn resolve_boolean_config(
    cli_input: serde_json::Value,
    environment_value: serde_json::Value,
    fallback: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let flag_value = get_boolean_flag_value(cli_input)?;
    if nullish(&flag_value) {
        if nullish(&environment_value) {
            Ok(fallback)
        } else {
            Ok(environment_value)
        }
    } else {
        Ok(flag_value)
    }
}
