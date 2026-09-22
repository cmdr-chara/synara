//! Bounded local validation of an explicit JSON Schema subset. Unsupported
//! keywords fail before network I/O, never silently weaken the reviewed schema.
//! No references, remote resolution, regexes, code generation or unbounded loops.
use crate::*;
use std::collections::HashSet;

const MAX_SCHEMA: usize = 64 * 1024;
const MAX_NODES: usize = 4096;
const MAX_DEPTH: usize = 32;
const MAX_STEPS: usize = 65536;

pub(crate) fn check_schema(schema: &Value) -> ModelResult<()> {
    if serde_json::to_vec(schema)
        .map_err(|_| ModelError::Protocol)?
        .len()
        > MAX_SCHEMA
    {
        return Err(ModelError::Limit);
    }
    let mut budget = MAX_NODES;
    inspect(schema, 0, &mut budget)
}
fn step(budget: &mut usize) -> ModelResult<()> {
    *budget = budget.checked_sub(1).ok_or(ModelError::Limit)?;
    Ok(())
}
fn types(value: &Value) -> ModelResult<Vec<&str>> {
    let values = match value {
        Value::String(v) => vec![v.as_str()],
        Value::Array(v) if !v.is_empty() && v.len() <= 7 => v
            .iter()
            .map(|v| v.as_str().ok_or(ModelError::Invalid("schema type")))
            .collect::<ModelResult<Vec<_>>>()?,
        _ => return Err(ModelError::Invalid("schema type")),
    };
    let mut seen = HashSet::new();
    for v in &values {
        if !matches!(
            *v,
            "null" | "boolean" | "number" | "integer" | "string" | "array" | "object"
        ) || !seen.insert(*v)
        {
            return Err(ModelError::Invalid("schema type"));
        }
    }
    Ok(values)
}
fn inspect(schema: &Value, depth: usize, budget: &mut usize) -> ModelResult<()> {
    step(budget)?;
    if depth > MAX_DEPTH {
        return Err(ModelError::Limit);
    }
    if schema.is_boolean() {
        return Ok(());
    }
    let object = schema
        .as_object()
        .ok_or(ModelError::Invalid("schema must be object or boolean"))?;
    for (key, value) in object {
        match key.as_str() {
            "type" => {
                types(value)?;
            }
            "title" | "description" | "$comment" => {
                if !value.is_string() {
                    return Err(ModelError::Invalid("schema annotation"));
                }
            }
            "$schema" => {
                if value != "https://json-schema.org/draft/2020-12/schema" {
                    return Err(ModelError::Unsupported("JSON schema dialect"));
                }
            }
            "default" | "const" => {}
            "enum" => {
                let values = value.as_array().ok_or(ModelError::Invalid("schema enum"))?;
                if values.is_empty() || values.len() > 256 {
                    return Err(ModelError::Invalid("schema enum size"));
                }
            }
            "required" => {
                let values = value
                    .as_array()
                    .ok_or(ModelError::Invalid("schema required"))?;
                let mut names = HashSet::new();
                if values.len() > 256 {
                    return Err(ModelError::Limit);
                }
                for value in values {
                    let name = value
                        .as_str()
                        .ok_or(ModelError::Invalid("schema required name"))?;
                    if !names.insert(name) {
                        return Err(ModelError::Invalid("duplicate schema required name"));
                    }
                }
            }
            "properties" => {
                let properties = value
                    .as_object()
                    .ok_or(ModelError::Invalid("schema properties"))?;
                if properties.len() > 256 {
                    return Err(ModelError::Limit);
                }
                for value in properties.values() {
                    inspect(value, depth + 1, budget)?;
                }
            }
            "items" | "additionalProperties" | "not" => inspect(value, depth + 1, budget)?,
            "allOf" | "anyOf" | "oneOf" => {
                let values = value
                    .as_array()
                    .ok_or(ModelError::Invalid("schema alternatives"))?;
                if values.is_empty() || values.len() > 32 {
                    return Err(ModelError::Invalid("schema alternative count"));
                }
                for value in values {
                    inspect(value, depth + 1, budget)?;
                }
            }
            "minItems" | "maxItems" | "minLength" | "maxLength" | "minProperties"
            | "maxProperties" => {
                if value.as_u64().is_none() {
                    return Err(ModelError::Invalid("schema size constraint"));
                }
            }
            // Numeric range, regex/format, references and unevaluated vocabularies
            // are deliberately not approximated. Report the limitation up front.
            _ => {
                return Err(ModelError::Unsupported(
                    "JSON schema keyword (see direct-model schema subset)",
                ));
            }
        }
    }
    Ok(())
}
fn has_type(value: &Value, ty: &str) -> bool {
    match ty {
        "null" => value.is_null(),
        "boolean" => value.is_boolean(),
        "number" => value.is_number(),
        "integer" => value.as_number().is_some_and(|n| {
            n.is_i64() || n.is_u64() || n.as_f64().is_some_and(|n| n.fract() == 0.)
        }),
        "string" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        _ => false,
    }
}
/// Canonical decimal digits/exponent for JSON numeric equality, including 1 vs
/// 1.0 and integer values above f64's exact range. No coercion of strings/bools.
fn number_key(number: &serde_json::Number) -> (bool, String, i32) {
    let text = number.to_string();
    let (negative, text) = text
        .strip_prefix('-')
        .map_or((false, text.as_str()), |t| (true, t));
    let (mantissa, mut exponent) = text.split_once(['e', 'E']).map_or((text, 0), |(m, e)| {
        (
            m,
            e.parse::<i32>().expect("serialized finite JSON exponent"),
        )
    });
    let mut digits = mantissa.replace('.', "");
    if let Some((_, fraction)) = mantissa.split_once('.') {
        exponent -= fraction.len() as i32;
    }
    let trimmed = digits.trim_start_matches('0').to_owned();
    digits = trimmed;
    if digits.is_empty() {
        return (false, "0".into(), 0);
    }
    while digits.ends_with('0') {
        digits.pop();
        exponent += 1;
    }
    (negative, digits, exponent)
}
fn equal(a: &Value, b: &Value, budget: &mut usize, depth: usize) -> ModelResult<bool> {
    step(budget)?;
    if depth > MAX_DEPTH {
        return Err(ModelError::Limit);
    }
    Ok(match (a, b) {
        (Value::Number(a), Value::Number(b)) => number_key(a) == number_key(b),
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (a, b) in a.iter().zip(b) {
                if !equal(a, b, budget, depth + 1)? {
                    return Ok(false);
                }
            }
            true
        }
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() {
                return Ok(false);
            }
            for (key, a) in a {
                let Some(b) = b.get(key) else {
                    return Ok(false);
                };
                if !equal(a, b, budget, depth + 1)? {
                    return Ok(false);
                }
            }
            true
        }
        _ => a == b,
    })
}
fn bounds(schema: &Value, size: usize, min: &str, max: &str) -> bool {
    schema
        .get(min)
        .and_then(Value::as_u64)
        .is_none_or(|n| size as u64 >= n)
        && schema
            .get(max)
            .and_then(Value::as_u64)
            .is_none_or(|n| size as u64 <= n)
}
fn matches(schema: &Value, value: &Value, depth: usize, budget: &mut usize) -> ModelResult<bool> {
    step(budget)?;
    if depth > MAX_DEPTH {
        return Err(ModelError::Limit);
    }
    if let Some(boolean) = schema.as_bool() {
        return Ok(boolean);
    }
    if let Some(ty) = schema.get("type") {
        if !types(ty)?.iter().any(|ty| has_type(value, ty)) {
            return Ok(false);
        }
    }
    if let Some(constant) = schema.get("const") {
        if !equal(constant, value, budget, 0)? {
            return Ok(false);
        }
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let mut found = false;
        for candidate in values {
            if equal(candidate, value, budget, 0)? {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(false);
        }
    }
    for key in ["allOf", "anyOf", "oneOf"] {
        if let Some(choices) = schema.get(key).and_then(Value::as_array) {
            let mut successes = 0;
            for choice in choices {
                if matches(choice, value, depth + 1, budget)? {
                    successes += 1;
                }
            }
            if (key == "allOf" && successes != choices.len())
                || (key == "anyOf" && successes == 0)
                || (key == "oneOf" && successes != 1)
            {
                return Ok(false);
            }
        }
    }
    if let Some(not) = schema.get("not") {
        if matches(not, value, depth + 1, budget)? {
            return Ok(false);
        }
    }
    if let Some(object) = value.as_object() {
        if !bounds(schema, object.len(), "minProperties", "maxProperties") {
            return Ok(false);
        }
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            if required
                .iter()
                .any(|key| !object.contains_key(key.as_str().expect("checked required name")))
            {
                return Ok(false);
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        for (key, value) in object {
            step(budget)?;
            if let Some(property) = properties.and_then(|p| p.get(key)) {
                if !matches(property, value, depth + 1, budget)? {
                    return Ok(false);
                }
            } else if let Some(additional) = schema.get("additionalProperties") {
                if !matches(additional, value, depth + 1, budget)? {
                    return Ok(false);
                }
            }
        }
    }
    if let Some(array) = value.as_array() {
        if !bounds(schema, array.len(), "minItems", "maxItems") {
            return Ok(false);
        }
        if let Some(items) = schema.get("items") {
            for value in array {
                if !matches(items, value, depth + 1, budget)? {
                    return Ok(false);
                }
            }
        }
    }
    if let Some(text) = value.as_str() {
        if !bounds(schema, text.chars().count(), "minLength", "maxLength") {
            return Ok(false);
        }
    }
    Ok(true)
}

pub(crate) fn validate_output(schema: &Value, text: &str) -> ModelResult<()> {
    check_schema(schema)?;
    if text.len() > MAX_RESPONSE_BYTES {
        return Err(ModelError::Limit);
    }
    let value: Value = serde_json::from_str(text).map_err(|_| ModelError::SchemaMismatch)?;
    let mut budget = MAX_STEPS;
    if matches(schema, &value, 0, &mut budget)? {
        Ok(())
    } else {
        Err(ModelError::SchemaMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn schema_validates_required_types_additional_nested_arrays_and_unicode() {
        let schema = json!({"type":"object","required":["name","items"],"additionalProperties":false,"properties":{
            "name":{"type":"string","minLength":2,"maxLength":3},"items":{"type":"array","minItems":1,"maxItems":2,"items":{"type":"integer"}}
        }});
        for value in [
            json!({"name":"日本","items":[1.0]}),
            json!({"name":"a😀","items":[1,2]}),
        ] {
            validate_output(&schema, &value.to_string()).unwrap();
        }
        for value in [
            json!({"name":"日","items":[1]}),
            json!({"name":"ok","items":[1.5]}),
            json!({"name":"ok","items":[]}),
            json!({"name":"ok"}),
            json!({"name":"ok","items":[1],"leak":true}),
        ] {
            assert!(matches!(
                validate_output(&schema, &value.to_string()),
                Err(ModelError::SchemaMismatch)
            ));
        }
    }
    #[test]
    fn schema_unions_const_enum_and_keyword_applicability() {
        validate_output(&json!({"type":["string","null"]}), "null").unwrap();
        validate_output(&json!({"enum":[{"n":1}]}), "{\"n\":1.0}").unwrap();
        validate_output(
            &json!({"const":10000000000000000001u64}),
            "10000000000000000001",
        )
        .unwrap();
        assert!(
            validate_output(
                &json!({"const":10000000000000000001u64}),
                "10000000000000000002"
            )
            .is_err()
        );
        validate_output(
            &json!({"oneOf":[{"type":"string"},{"type":"null"}]}),
            "null",
        )
        .unwrap();
        assert!(validate_output(&json!({"oneOf":[{},{}]}), "null").is_err());
        assert!(validate_output(&json!({"anyOf":[false,{"not":true}]}), "null").is_err());
        validate_output(
            &json!({"allOf":[{"type":"number"},{"not":{"const":0}}]}),
            "1",
        )
        .unwrap();
        // Object/string/array keywords do not implicitly require that type.
        validate_output(
            &json!({"required":["x"],"minItems":4,"minLength":4}),
            "true",
        )
        .unwrap();
    }
    #[test]
    fn schema_fails_closed_before_io_for_unknown_or_malformed_keywords() {
        for schema in [
            json!({"$ref":"https://evil.invalid/x"}),
            json!({"format":"email"}),
            json!({"pattern":"(a+)+"}),
            json!({"minimum":0}),
            json!({"type":["string","string"]}),
            json!({"required":[3]}),
            json!({"additionalProperties":3}),
            json!({"properties":{"a":{"typo":"string"}}}),
        ] {
            assert!(check_schema(&schema).is_err(), "{schema}");
        }
        assert!(matches!(
            validate_output(&json!({}), "{bad"),
            Err(ModelError::SchemaMismatch)
        ));
    }
    #[test]
    fn schema_enforces_depth_size_and_evaluation_budgets() {
        let mut value = json!({});
        for _ in 0..40 {
            value = json!({"items":value});
        }
        assert!(matches!(check_schema(&value), Err(ModelError::Limit)));
        assert!(check_schema(&json!({"description":"x".repeat(MAX_SCHEMA)})).is_err());
        let many = json!(vec![0; MAX_STEPS]);
        assert!(matches!(
            validate_output(&json!({"items":true}), &many.to_string()),
            Err(ModelError::Limit)
        ));
    }
}
