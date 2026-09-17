use crate::wire::{array, id, invalid, optional_string, string};
use serde_json::Value;
use std::collections::HashSet;
use synara_agent::{AgentError, AgentResult};
use synara_core::{InputField, InputFieldKind, SelectChoice, UserInputRequest};

/// Validation never fetches the URL or starts a browser. Opening it is an explicit UI action.
pub(crate) fn safe_web_url(value: &str) -> AgentResult<()> {
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| invalid("elicitation URL must use HTTP or HTTPS"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if value.len() > 8192
        || authority.is_empty()
        || authority.contains(['@', '\\', '%'])
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_whitespace())
    {
        return Err(invalid("invalid elicitation URL"));
    }
    Ok(())
}

pub(crate) fn parse(value: &Value, request_id: String) -> AgentResult<UserInputRequest> {
    let message = string(value, "message")?.to_owned();
    if message.len() > 64 * 1024 {
        return Err(AgentError::Limit);
    }
    match value.get("mode").and_then(Value::as_str).unwrap_or("form") {
        "url" => {
            id(value, "elicitationId")?;
            let url = string(value, "url")?;
            safe_web_url(url)?;
            Ok(UserInputRequest {
                id: request_id,
                message,
                fields: vec![],
                url: Some(url.into()),
            })
        }
        "form" => {
            let schema = value
                .get("requestedSchema")
                .ok_or_else(|| invalid("elicitation form schema missing"))?;
            if schema.get("type").and_then(Value::as_str) != Some("object") {
                return Err(invalid("elicitation schema must be an object"));
            }
            let properties = schema
                .get("properties")
                .and_then(Value::as_object)
                .ok_or_else(|| invalid("elicitation properties missing"))?;
            if properties.len() > 32 {
                return Err(AgentError::Limit);
            }
            let required: HashSet<_> = match schema.get("required") {
                None => HashSet::new(),
                Some(_) => array(schema, "required", 32)?
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .ok_or_else(|| invalid("invalid required field"))
                    })
                    .collect::<AgentResult<_>>()?,
            };
            if required.iter().any(|name| !properties.contains_key(*name)) {
                return Err(invalid("required field does not exist"));
            }
            let mut fields = Vec::new();
            for (name, property) in properties {
                if name.is_empty() || name.len() > 128 || name.chars().any(char::is_control) {
                    return Err(invalid("invalid form field ID"));
                }
                let lower = name.to_ascii_lowercase();
                if [
                    "password",
                    "passwd",
                    "secret",
                    "access_token",
                    "api_key",
                    "private_key",
                    "authorization",
                ]
                .iter()
                .any(|token| lower.contains(token))
                {
                    return Err(invalid(
                        "sensitive values must use a URL authentication flow, not a form",
                    ));
                }
                let kind = match property.get("type").and_then(Value::as_str) {
                    Some("string") | None
                        if property.get("enum").is_some() || property.get("oneOf").is_some() =>
                    {
                        InputFieldKind::Choice {
                            options: enum_choices(property)?,
                        }
                    }
                    Some("string") => InputFieldKind::Text {
                        min_length: bound(property, "minLength")?,
                        max_length: bound(property, "maxLength")?,
                        format: optional_string(property, "format"),
                    },
                    Some("boolean") => InputFieldKind::Boolean,
                    Some(kind @ ("integer" | "number")) => InputFieldKind::Number {
                        integer: kind == "integer",
                        minimum: number_bound(property, "minimum")?,
                        maximum: number_bound(property, "maximum")?,
                    },
                    Some("array") => InputFieldKind::MultiChoice {
                        options: enum_choices(
                            property
                                .get("items")
                                .ok_or_else(|| invalid("array choices missing"))?,
                        )?,
                        minimum: bound(property, "minItems")?,
                        maximum: bound(property, "maxItems")?,
                    },
                    _ => return Err(invalid("unsupported elicitation form field")),
                };
                fields.push(InputField {
                    id: name.clone(),
                    label: optional_string(property, "title").unwrap_or_else(|| name.clone()),
                    required: required.contains(name.as_str()),
                    kind,
                });
            }
            Ok(UserInputRequest {
                id: request_id,
                message,
                fields,
                url: None,
            })
        }
        _ => Err(invalid("unsupported elicitation mode")),
    }
}
fn bound(value: &Value, key: &str) -> AgentResult<Option<usize>> {
    value
        .get(key)
        .map(|value| {
            value
                .as_u64()
                .and_then(|value| usize::try_from(value).ok())
                .ok_or_else(|| invalid("invalid form size bound"))
        })
        .transpose()
}
fn number_bound(value: &Value, key: &str) -> AgentResult<Option<f64>> {
    value
        .get(key)
        .map(|value| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| invalid("invalid numeric bound"))
        })
        .transpose()
}
fn enum_choices(value: &Value) -> AgentResult<Vec<SelectChoice>> {
    let choices = if value.get("oneOf").is_some() || value.get("anyOf").is_some() {
        let key = if value.get("oneOf").is_some() {
            "oneOf"
        } else {
            "anyOf"
        };
        array(value, key, 512)?
            .iter()
            .map(|item| {
                let value = string(item, "const")?.to_owned();
                Ok(SelectChoice {
                    label: optional_string(item, "title").unwrap_or_else(|| value.clone()),
                    value,
                    group: None,
                })
            })
            .collect::<AgentResult<Vec<_>>>()?
    } else {
        array(value, "enum", 512)?
            .iter()
            .map(|item| {
                let value = item
                    .as_str()
                    .ok_or_else(|| invalid("form choices must be strings"))?
                    .to_owned();
                Ok(SelectChoice {
                    label: value.clone(),
                    value,
                    group: None,
                })
            })
            .collect::<AgentResult<Vec<_>>>()?
    };
    let mut seen = HashSet::new();
    if choices.is_empty()
        || choices
            .iter()
            .any(|choice| !seen.insert(choice.value.clone()))
    {
        return Err(invalid("form choices must be nonempty and unique"));
    }
    Ok(choices)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn forms_support_primitive_and_multiple_choice_fields() {
        let schema = json!({"mode":"form","message":"Choose","requestedSchema":{"type":"object","required":["name"],"properties":{"name":{"type":"string","minLength":1},"features":{"type":"array","items":{"type":"string","enum":["a","b"]},"minItems":1},"confirm":{"type":"boolean"}}}});
        let request = parse(&schema, "a".into()).unwrap();
        assert_eq!(request.fields.len(), 3);
        assert!(
            request
                .fields
                .iter()
                .find(|field| field.id == "name")
                .unwrap()
                .required
        );
        assert!(matches!(
            request
                .fields
                .iter()
                .find(|field| field.id == "features")
                .unwrap()
                .kind,
            InputFieldKind::MultiChoice { .. }
        ));
    }
    #[test]
    fn secret_forms_and_unsafe_urls_are_rejected() {
        assert!(parse(&json!({"message":"Login","requestedSchema":{"type":"object","properties":{"password":{"type":"string"}}}}),"a".into()).is_err());
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://user:secret@example.com",
            "https://example.com\nother",
            "https://",
        ] {
            assert!(safe_web_url(url).is_err());
        }
        assert!(safe_web_url("http://127.0.0.1:9876/login").is_ok());
    }
}
