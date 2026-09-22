//! Google Generative Language REST/SSE, separate from ACP and Vertex/cloud auth.
//! Contract: https://ai.google.dev/api/generate-content and /api/models.
use crate::*;
use serde_json::json;

/// Treat a reviewed model name as one path segment, never as a URL or a route.
/// Both discovery's `models/id` and a catalog's plain `id` are accepted.
pub(crate) fn model_id(value: &str) -> ModelResult<&str> {
    let id = value.strip_prefix("models/").unwrap_or(value);
    if id.is_empty()
        || id.len() > 128
        || id.starts_with('.')
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    {
        return Err(ModelError::Invalid("Google model identifier"));
    }
    Ok(id)
}

pub(crate) fn encode(request: &ModelRequest) -> ModelResult<Value> {
    let mut system = Vec::new();
    let mut contents = Vec::new();
    for message in &request.messages {
        let parts: Vec<Value> = message
            .content
            .iter()
            .map(|part| match part {
                Content::Text { text } => json!({"text":text}),
                Content::Image { media_type, base64 } => {
                    json!({"inlineData":{"mimeType":media_type,"data":base64}})
                }
            })
            .collect();
        match message.role {
            MessageRole::System => system.extend(parts),
            MessageRole::User | MessageRole::Assistant => contents.push(json!({
                "role": if message.role == MessageRole::User { "user" } else { "model" }, "parts":parts
            })),
            MessageRole::Tool => return Err(ModelError::Unsupported("Google tool/signature round trips")),
        }
    }
    if contents.is_empty() {
        return Err(ModelError::Invalid("Google conversation contents"));
    }
    let mut body = json!({"contents":contents,"generationConfig":{
        "candidateCount":1,"maxOutputTokens":request.max_output_tokens,
        "responseModalities":["TEXT"]
    }});
    if !system.is_empty() {
        body["systemInstruction"] = json!({"parts":system});
    }
    if let OutputFormat::JsonSchema { schema, .. } = &request.output {
        body["generationConfig"]["responseMimeType"] = json!("application/json");
        body["generationConfig"]["responseJsonSchema"] = schema.clone();
    }
    Ok(body)
}

type Chunk<'a> = (Vec<(&'a str, bool)>, Option<ModelUsage>, bool);
pub(crate) fn decode(value: &Value) -> ModelResult<Chunk<'_>> {
    if value
        .get("promptFeedback")
        .and_then(|v| v.get("blockReason"))
        .is_some()
    {
        return Err(ModelError::Incomplete);
    }
    let usage = value
        .get("usageMetadata")
        .map(|usage| {
            let read = |key| optional_tokens(usage, key);
            let thoughts = read("thoughtsTokenCount")?;
            // Candidates exclude thinking tokens. Normalize to total generated output,
            // with a separate reasoning subset, as for the OpenAI transport family.
            let output = read("candidatesTokenCount")?
                .map(|n| {
                    n.checked_add(thoughts.unwrap_or(0))
                        .ok_or(ModelError::Limit)
                })
                .transpose()?;
            let input = read("promptTokenCount")?;
            let cached = read("cachedContentTokenCount")?;
            if cached.zip(input).is_some_and(|(c, i)| c > i) {
                return Err(ModelError::Protocol);
            }
            Ok(ModelUsage {
                input_tokens: input,
                output_tokens: output,
                cached_input_tokens: cached,
                reasoning_tokens: thoughts,
            })
        })
        .transpose()?;
    let Some(candidates) = value.get("candidates") else {
        return if usage.is_some() {
            Ok((vec![], usage, false))
        } else {
            Err(ModelError::Protocol)
        };
    };
    let candidates = candidates.as_array().ok_or(ModelError::Protocol)?;
    if candidates.len() > 1 {
        return Err(ModelError::Unsupported("multiple response choices"));
    }
    let mut parts = Vec::new();
    let mut finished = false;
    if let Some(candidate) = candidates.first() {
        if candidate
            .get("index")
            .is_some_and(|v| v.as_u64() != Some(0))
        {
            return Err(ModelError::Protocol);
        }
        if let Some(content) = candidate.get("content") {
            if content.get("role").is_some_and(|r| r != "model") {
                return Err(ModelError::Protocol);
            }
            for part in content["parts"].as_array().ok_or(ModelError::Protocol)? {
                let object = part.as_object().ok_or(ModelError::Protocol)?;
                // No automatic file fetch, tool execution, image decode, signature
                // replay or silently discarded unsupported output kinds.
                if object
                    .keys()
                    .any(|k| !matches!(k.as_str(), "text" | "thought" | "thoughtSignature"))
                {
                    return Err(ModelError::Unsupported("Google non-text output"));
                }
                let thought = match part.get("thought") {
                    None => false,
                    Some(v) => v.as_bool().ok_or(ModelError::Protocol)?,
                };
                if let Some(text) = part.get("text") {
                    parts.push((text.as_str().ok_or(ModelError::Protocol)?, thought));
                } else if part.get("thoughtSignature").is_none() {
                    return Err(ModelError::Protocol);
                }
            }
        }
        if let Some(reason) = candidate.get("finishReason") {
            match reason.as_str() {
                Some("STOP") => finished = true,
                Some("FINISH_REASON_UNSPECIFIED") => {}
                Some(_) => return Err(ModelError::Incomplete),
                None => return Err(ModelError::Protocol),
            }
        }
    }
    Ok((parts, usage, finished))
}

fn optional_tokens(value: &Value, key: &str) -> ModelResult<Option<u64>> {
    match value.get(key) {
        None => Ok(None),
        Some(v) => v.as_u64().map(Some).ok_or(ModelError::Protocol),
    }
}

pub(crate) fn models(value: Value) -> ModelResult<(Vec<ModelInfo>, Option<String>)> {
    let data = value["models"].as_array().ok_or(ModelError::Protocol)?;
    if data.len() > 1000 {
        return Err(ModelError::Limit);
    }
    let next = match value.get("nextPageToken") {
        None => None,
        Some(value) => {
            let value = value.as_str().ok_or(ModelError::Protocol)?;
            if value.len() > 4096 || value.chars().any(char::is_control) {
                return Err(ModelError::Limit);
            }
            if value.is_empty() {
                None
            } else {
                Some(value.into())
            }
        }
    };
    let mut models = Vec::new();
    for item in data {
        let methods = item["supportedGenerationMethods"]
            .as_array()
            .ok_or(ModelError::Protocol)?;
        if !methods
            .iter()
            .any(|method| method == "generateContent" || method == "streamGenerateContent")
        {
            continue;
        }
        let id = model_id(item["name"].as_str().ok_or(ModelError::Protocol)?)?;
        let name = item["displayName"]
            .as_str()
            .filter(|name| bounded_identifier(name))
            .unwrap_or(id);
        models.push(ModelInfo {
            id: id.into(),
            name: name.into(),
            capabilities: ModelCapabilities {
                context_window: optional_tokens(item, "inputTokenLimit")?.filter(|n| *n > 0),
                max_output_tokens: optional_tokens(item, "outputTokenLimit")?.filter(|n| *n > 0),
                // listModels reports token limits and generation methods, not all
                // tools/vision/schema/effort capabilities. Preserve that distinction.
                source: "provider listModels: generation method and reported token limits".into(),
                ..Default::default()
            },
        });
    }
    Ok((models, next))
}

#[cfg(test)]
mod tests;
