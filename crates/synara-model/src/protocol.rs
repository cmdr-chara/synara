use crate::*;
use serde_json::json;

pub(crate) fn encode(profile: &ProviderProfile, request: &ModelRequest) -> ModelResult<Value> {
    validate_request(profile, request)?;
    let mut body = match profile.protocol {
        ProtocolFamily::OpenAiChat => {
            let messages = request.messages.iter().map(|m| {
                let content: Vec<Value> = m.content.iter().map(|part| match part {
                    Content::Text { text } => json!({"type":"text", "text":text}),
                    Content::Image { media_type, base64 } => json!({"type":"image_url", "image_url":{"url":format!("data:{media_type};base64,{base64}")}}),
                }).collect();
                let mut value = json!({"role":m.role,"content":content});
                // Tool results use the text form accepted by compatible servers.
                if m.role == MessageRole::Tool {
                    let text = m.content.iter().filter_map(|c| match c { Content::Text { text } => Some(text.as_str()), _ => None }).collect::<Vec<_>>().join("\n");
                    value["content"] = json!(text);
                    value["tool_call_id"] = json!(m.tool_call_id);
                }
                if !m.tool_calls.is_empty() {
                    value["tool_calls"] = json!(m.tool_calls.iter().map(|call| json!({"id":call.id,"type":"function","function":{"name":call.name,"arguments":call.arguments.to_string()}})).collect::<Vec<_>>());
                }
                value
            }).collect::<Vec<_>>();
            let mut value = json!({"model":request.model,"messages":messages,"stream":true,"stream_options":{"include_usage":true}});
            // Some compatible servers only accept max_tokens. Effort-capable profiles
            // explicitly select the OpenAI completion-token field.
            let limit_key = if request.reasoning_effort.is_some() {
                "max_completion_tokens"
            } else {
                "max_tokens"
            };
            value[limit_key] = json!(request.max_output_tokens);
            if !request.tools.is_empty() {
                value["tools"] = json!(request.tools.iter().map(|t| json!({"type":"function","function":{"name":t.name,"description":t.description,"parameters":t.parameters}})).collect::<Vec<_>>());
            }
            if let Some(effort) = &request.reasoning_effort {
                value["reasoning_effort"] = json!(effort);
            }
            if let OutputFormat::JsonSchema { name, schema } = &request.output {
                value["response_format"] = json!({"type":"json_schema","json_schema":{"name":name,"strict":true,"schema":schema}});
            }
            value
        }
        ProtocolFamily::GoogleGenerateContent => crate::google::encode(request)?,
        ProtocolFamily::AnthropicMessages => {
            let mut system = Vec::new();
            let mut messages = Vec::new();
            for m in &request.messages {
                let mut content = m.content.iter().map(|part| match part {
                    Content::Text { text } => json!({"type":"text","text":text}),
                    Content::Image { media_type, base64 } => json!({"type":"image","source":{"type":"base64","media_type":media_type,"data":base64}}),
                }).collect::<Vec<_>>();
                match m.role {
                    MessageRole::System => {
                        system.extend(content);
                        continue;
                    }
                    MessageRole::Tool => {
                        content = vec![
                            json!({"type":"tool_result","tool_use_id":m.tool_call_id,"content":content}),
                        ];
                    }
                    MessageRole::Assistant => {
                        for call in &m.tool_calls {
                            content.push(json!({"type":"tool_use","id":call.id,"name":call.name,"input":call.arguments}));
                        }
                    }
                    MessageRole::User => {}
                }
                messages.push(json!({"role":if m.role == MessageRole::Assistant {"assistant"} else {"user"},"content":content}));
            }
            let mut value = json!({"model":request.model,"messages":messages,"max_tokens":request.max_output_tokens,"stream":true});
            if !system.is_empty() {
                value["system"] = json!(system);
            }
            if !request.tools.is_empty() {
                value["tools"] = json!(request.tools.iter().map(|t| json!({"name":t.name,"description":t.description,"input_schema":t.parameters})).collect::<Vec<_>>());
            }
            value
        }
    };
    // Never add credentials to a serializable request body.
    if serde_json::to_vec(&body)
        .map_err(|_| ModelError::Protocol)?
        .len()
        > MAX_REQUEST_BYTES
    {
        return Err(ModelError::Limit);
    }
    Ok(body.take())
}
