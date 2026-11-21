//! Clean-room port of `packages/contracts/src/browserAutomationToolCommon.ts`,
//! written from recorded behavior only.
use serde_json::{Value, json};

fn func(name: &str) -> Value {
    json!({ "$function": name })
}

fn filter(annotations: Value) -> Value {
    json!({ "_tag": "Filter", "run": func(""), "annotations": annotations, "aborted": false })
}

/// Port of `browserBoundedInt`: a fixed effect-schema shell whose isBetween
/// filter interpolates the raw bounds verbatim (no validation; `min > max`,
/// NaN and infinities all pass through).
pub fn browser_bounded_int(minimum: f64, maximum: f64) -> Result<Value, String> {
    let (min, max) = (crate::js::number(minimum), crate::js::number(maximum));
    Ok(json!({
        "ast": {
            "~effect/Schema": "~effect/Schema", "annotations": crate::js::undefined(),
            "checks": [
                filter(json!({ "expected": "an integer", "meta": { "_tag": "isInt" },
                    "toArbitraryConstraint": { "number": { "isInteger": true } } })),
                filter(json!({
                    "expected": format!("a value between {} and {}",
                        crate::js::number_to_string(minimum), crate::js::number_to_string(maximum)),
                    "meta": { "_tag": "isBetween", "minimum": min, "maximum": max },
                    "toArbitraryConstraint": { "number": { "min": min, "max": max } } })),
            ],
            "encoding": crate::js::undefined(), "context": crate::js::undefined(), "_tag": "Number"
        },
        "rebuild": func(""), "makeUnsafe": func(""), "makeOption": func("")
    }))
}

/// AST tags that go through effect's `ast.recur` default-resolution —
/// unreachable through the wire, which only ships plain data.
const RECUR_TAGS: [&str; 3] = ["Objects", "Suspend", "Arrays"];

/// One `Reflect.ownKeys` entry of `fields` turned into a property signature.
fn entry(key: &str, value: &Value) -> Result<Value, String> {
    if crate::js::is_undefined(value) {
        return Err("Cannot read properties of undefined (reading 'ast')".into());
    }
    if value.is_null() {
        return Err("Cannot read properties of null (reading 'ast')".into());
    }
    let Some(ast) = value.get("ast").filter(|a| !crate::js::is_undefined(a)) else {
        return Err("Cannot read properties of undefined (reading 'encoding')".into());
    };
    if let Some(tag) = ast.get("_tag").and_then(|t| t.as_str())
        && RECUR_TAGS.contains(&tag)
    {
        return Err("ast.recur is not a function".into());
    }
    Ok(json!({ "name": key, "type": ast }))
}

/// Port of `browserClosedStruct`: `Schema.Struct`-like shell over plain-data
/// field descriptors. Non-object inputs fail `Reflect.ownKeys`; each entry
/// needs a `.ast` to land in `propertySignatures`.
pub fn browser_closed_struct(fields: Value) -> Result<Value, String> {
    // Arrays are objects too: ownKeys yields indices then "length".
    let entries: Vec<(String, Value)> = match &fields {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, w)| (i.to_string(), w.clone()))
            .chain([("length".into(), crate::js::number(items.len() as f64))])
            .collect(),
        Value::Object(map) => map.iter().map(|(k, w)| (k.clone(), w.clone())).collect(),
        _ => return Err("Reflect.ownKeys called on non-object".into()),
    };
    let mut signatures = Vec::with_capacity(entries.len());
    for (key, value) in &entries {
        signatures.push(entry(key, value)?);
    }
    Ok(json!({
        "fields": fields, "mapFields": func("mapFields"),
        "ast": {
            "~effect/Schema": "~effect/Schema",
            "annotations": { "parseOptions": { "onExcessProperty": "error" } },
            "checks": crate::js::undefined(), "encoding": crate::js::undefined(),
            "context": crate::js::undefined(), "_tag": "Objects",
            "propertySignatures": signatures, "indexSignatures": []
        },
        "rebuild": func(""), "makeUnsafe": func(""), "makeOption": func("")
    }))
}
