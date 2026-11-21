//! The Rust port of `packages/contracts/src/baseSchemas.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn undefined() -> Value {
    crate::js::undefined()
}

fn func() -> Value {
    json!({ "$function": "" })
}

fn schema_meta(expected: Value) -> Value {
    json!({
        "~effect/Schema": "~effect/Schema",
        "annotations": expected,
        "checks": undefined(),
        "encoding": undefined(),
        "context": undefined(),
        "_tag": "String",
    })
}

fn schema_shell(ast: Value) -> Value {
    json!({
        "ast": ast,
        "rebuild": func(),
        "makeUnsafe": func(),
        "makeOption": func(),
    })
}

fn filter(annotations: Value) -> Value {
    json!({
        "_tag": "Filter",
        "run": func(),
        "annotations": annotations,
        "aborted": false,
    })
}

fn trimmed_filter() -> Value {
    const PATTERN: &str = "^\\S[\\s\\S]*\\S$|^\\S$|^$";
    filter(json!({
        "expected": "a string with no leading or trailing whitespace",
        "meta": {
            "_tag": "isTrimmed",
            "regExp": { "$regexp": format!("/{PATTERN}/") },
        },
        "toArbitraryConstraint": { "string": { "patterns": [PATTERN] } },
    }))
}

fn min_length_filter() -> Value {
    filter(json!({
        "expected": "a value with a length of at least 1",
        "meta": {
            "_tag": "isMinLength",
            "minLength": 1,
        },
        "~structural": true,
        "toArbitraryConstraint": {
            "string": { "minLength": 1 },
            "array": { "minLength": 1 },
        },
    }))
}

fn max_length_filter(max_length: f64) -> Value {
    let n = crate::js::number(max_length);
    filter(json!({
        "expected": format!("a value with a length of at most {}", crate::js::number_to_string(max_length)),
        "meta": {
            "_tag": "isMaxLength",
            "maxLength": n,
        },
        "~structural": true,
        "toArbitraryConstraint": {
            "string": { "maxLength": n },
            "array": { "maxLength": n },
        },
    }))
}

/// Port of `boundedTrimmedNonEmptyString` (`packages/contracts/src/baseSchemas.ts`).
pub fn bounded_trimmed_non_empty_string(max_length: f64) -> Result<serde_json::Value, String> {
    let max_length = if max_length.is_nan() {
        f64::NAN
    } else {
        max_length.floor().max(0.0)
    };
    let inner = schema_meta(json!({
        "expected": "a string that will be decoded as a trimmed string",
    }));
    let checks = json!([
        trimmed_filter(),
        min_length_filter(),
        max_length_filter(max_length),
    ]);
    Ok(json!({
        "from": schema_shell(inner.clone()),
        "to": schema_shell(json!({
            "~effect/Schema": "~effect/Schema",
            "annotations": undefined(),
            "checks": [trimmed_filter()],
            "encoding": undefined(),
            "context": undefined(),
            "_tag": "String",
        })),
        "ast": {
            "~effect/Schema": "~effect/Schema",
            "annotations": undefined(),
            "checks": checks,
            "encoding": [{
                "to": inner,
                "transformation": {
                    "~effect/SchemaTransformation/Transformation": "~effect/SchemaTransformation/Transformation",
                    "_tag": "Transformation",
                    "decode": { "run": func() },
                    "encode": { "run": func() },
                },
            }],
            "context": undefined(),
            "_tag": "String",
        },
        "rebuild": func(),
        "makeUnsafe": func(),
        "makeOption": func(),
    }))
}
