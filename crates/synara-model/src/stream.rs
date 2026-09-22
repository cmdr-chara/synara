use crate::*;
use std::collections::{BTreeMap, HashSet};

/// Incremental SSE framing. UTF-8 may be split across TCP chunks. No retry fields
/// are acted on, and an unterminated frame is not treated as a completed answer.
#[derive(Default)]
pub(crate) struct SseDecoder {
    line: Vec<u8>,
    data: String,
    frame_bytes: usize,
}
impl SseDecoder {
    pub fn push(&mut self, bytes: &[u8]) -> ModelResult<Vec<String>> {
        let mut frames = Vec::new();
        for &byte in bytes {
            self.frame_bytes += 1;
            if self.frame_bytes > MAX_FRAME_BYTES {
                return Err(ModelError::Limit);
            }
            if byte != b'\n' {
                self.line.push(byte);
                continue;
            }
            if self.line.last() == Some(&b'\r') {
                self.line.pop();
            }
            let line = std::str::from_utf8(&self.line).map_err(|_| ModelError::Protocol)?;
            if line.is_empty() {
                if !self.data.is_empty() {
                    self.data.pop();
                    frames.push(std::mem::take(&mut self.data));
                }
                self.frame_bytes = 0;
            } else if let Some(data) = line.strip_prefix("data:") {
                self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
                self.data.push('\n');
            }
            self.line.clear();
        }
        Ok(frames)
    }
}
#[derive(Default)]
struct PendingTool {
    id: String,
    name: String,
    arguments: String,
}
pub(crate) struct ProtocolDecoder {
    family: ProtocolFamily,
    tools: BTreeMap<u64, PendingTool>,
    allowed_tools: HashSet<String>,
    reason: Option<String>,
    pub finished: bool,
    usage: ModelUsage,
    pub text: String,
    output_bytes: usize,
    started: bool,
}
impl ProtocolDecoder {
    pub fn new(family: ProtocolFamily, request: &ModelRequest) -> Self {
        Self {
            family,
            tools: BTreeMap::new(),
            allowed_tools: request.tools.iter().map(|t| t.name.clone()).collect(),
            reason: None,
            finished: false,
            usage: ModelUsage::default(),
            text: String::new(),
            output_bytes: 0,
            started: false,
        }
    }
    fn tool(&mut self, index: u64) -> ModelResult<&mut PendingTool> {
        if index >= 64 {
            return Err(ModelError::Limit);
        }
        Ok(self.tools.entry(index).or_default())
    }
    fn end(&mut self) -> ModelResult<Vec<ModelEvent>> {
        let reason = self.reason.take().ok_or(ModelError::Protocol)?;
        if !bounded_identifier(&reason) {
            return Err(ModelError::Protocol);
        }
        let mut events = Vec::new();
        let mut ids = HashSet::new();
        for (_, tool) in std::mem::take(&mut self.tools) {
            if !bounded_identifier(&tool.id)
                || !self.allowed_tools.contains(&tool.name)
                || !ids.insert(tool.id.clone())
            {
                return Err(ModelError::Protocol);
            }
            let arguments: Value =
                serde_json::from_str(&tool.arguments).map_err(|_| ModelError::Protocol)?;
            if !arguments.is_object() {
                return Err(ModelError::Protocol);
            }
            events.push(ModelEvent::ToolCall(ToolCall {
                id: tool.id,
                name: tool.name,
                arguments,
            }));
        }
        self.finished = true;
        events.push(ModelEvent::Finished { reason });
        Ok(events)
    }
    fn text_event(
        &mut self,
        text: &str,
        reasoning: bool,
        events: &mut Vec<ModelEvent>,
    ) -> ModelResult<()> {
        self.output_bytes = self
            .output_bytes
            .checked_add(text.len())
            .ok_or(ModelError::Limit)?;
        if self.output_bytes > MAX_RESPONSE_BYTES {
            return Err(ModelError::Limit);
        }
        if !text.is_empty() {
            if reasoning {
                events.push(ModelEvent::Reasoning(text.to_owned()));
            } else {
                self.text.push_str(text);
                events.push(ModelEvent::Text(text.to_owned()));
            }
        }
        Ok(())
    }
    pub fn push(&mut self, data: &str) -> ModelResult<Vec<ModelEvent>> {
        if self.finished {
            return Err(ModelError::Protocol);
        }
        if self.family == ProtocolFamily::OpenAiChat && data.trim() == "[DONE]" {
            return self.end();
        }
        let value: Value = serde_json::from_str(data).map_err(|_| ModelError::Protocol)?;
        if value.get("error").is_some() {
            return Err(ModelError::Protocol);
        }
        let mut events = Vec::new();
        match self.family {
            ProtocolFamily::OpenAiChat => {
                if let Some(usage) = value.get("usage").filter(|v| v.is_object()) {
                    self.usage.input_tokens = usage["prompt_tokens"].as_u64();
                    self.usage.output_tokens = usage["completion_tokens"].as_u64();
                    self.usage.cached_input_tokens =
                        usage["prompt_tokens_details"]["cached_tokens"].as_u64();
                    self.usage.reasoning_tokens =
                        usage["completion_tokens_details"]["reasoning_tokens"].as_u64();
                    events.push(ModelEvent::Usage(self.usage.clone()));
                }
                let choices = value["choices"].as_array().ok_or(ModelError::Protocol)?;
                if choices.len() > 1 {
                    return Err(ModelError::Unsupported("multiple response choices"));
                }
                if let Some(choice) = choices.first() {
                    if choice["index"].as_u64() != Some(0) || self.reason.is_some() {
                        return Err(ModelError::Protocol);
                    }
                    let delta = &choice["delta"];
                    if let Some(text) = delta["content"].as_str() {
                        self.text_event(text, false, &mut events)?;
                    }
                    if let Some(text) = delta["reasoning_content"].as_str() {
                        self.text_event(text, true, &mut events)?;
                    }
                    if let Some(text) = delta["refusal"].as_str() {
                        self.text_event(text, false, &mut events)?;
                    }
                    if let Some(calls) = delta["tool_calls"].as_array() {
                        for call in calls {
                            let tool =
                                self.tool(call["index"].as_u64().ok_or(ModelError::Protocol)?)?;
                            if let Some(id) = call["id"].as_str() {
                                tool.id.push_str(id);
                            }
                            if let Some(name) = call["function"]["name"].as_str() {
                                tool.name.push_str(name);
                            }
                            if let Some(args) = call["function"]["arguments"].as_str() {
                                tool.arguments.push_str(args);
                            }
                            if tool.arguments.len() > MAX_FRAME_BYTES
                                || tool.id.len() > 256
                                || tool.name.len() > 64
                            {
                                return Err(ModelError::Limit);
                            }
                        }
                    }
                    if let Some(reason) = choice["finish_reason"].as_str() {
                        self.reason = Some(reason.to_owned());
                    }
                }
            }
            ProtocolFamily::AnthropicMessages => {
                match value["type"].as_str().ok_or(ModelError::Protocol)? {
                    "message_start" => {
                        if self.started {
                            return Err(ModelError::Protocol);
                        }
                        self.started = true;
                        let usage = &value["message"]["usage"];
                        let uncached = token_count(usage, "input_tokens")?;
                        let cache_read = token_count(usage, "cache_read_input_tokens")?;
                        let cache_write = token_count(usage, "cache_creation_input_tokens")?;
                        // Anthropic excludes cached reads/writes from input_tokens.
                        // Normalize to total input context, not uncached billing units.
                        self.usage.input_tokens = uncached
                            .map(|tokens| {
                                tokens
                                    .checked_add(cache_read.unwrap_or(0))
                                    .and_then(|tokens| tokens.checked_add(cache_write.unwrap_or(0)))
                                    .ok_or(ModelError::Limit)
                            })
                            .transpose()?;
                        self.usage.output_tokens = token_count(usage, "output_tokens")?;
                        self.usage.cached_input_tokens = cache_read;
                        events.push(ModelEvent::Usage(self.usage.clone()));
                    }
                    "content_block_start" => {
                        if !self.started || self.reason.is_some() {
                            return Err(ModelError::Protocol);
                        }
                        let block = &value["content_block"];
                        if block["type"] == "tool_use" {
                            let index = value["index"].as_u64().ok_or(ModelError::Protocol)?;
                            if self.tools.contains_key(&index) {
                                return Err(ModelError::Protocol);
                            }
                            let tool = self.tool(index)?;
                            tool.id = block["id"].as_str().ok_or(ModelError::Protocol)?.into();
                            tool.name = block["name"].as_str().ok_or(ModelError::Protocol)?.into();
                            if let Some(input) = block
                                .get("input")
                                .filter(|v| v.as_object().is_some_and(|o| !o.is_empty()))
                            {
                                tool.arguments = input.to_string();
                            }
                        } else if let Some(text) = block["text"].as_str() {
                            self.text_event(text, false, &mut events)?;
                        }
                    }
                    "content_block_delta" => {
                        if !self.started || self.reason.is_some() {
                            return Err(ModelError::Protocol);
                        }
                        let delta = &value["delta"];
                        match delta["type"].as_str() {
                            Some("text_delta") => self.text_event(
                                delta["text"].as_str().ok_or(ModelError::Protocol)?,
                                false,
                                &mut events,
                            )?,
                            Some("thinking_delta") => self.text_event(
                                delta["thinking"].as_str().ok_or(ModelError::Protocol)?,
                                true,
                                &mut events,
                            )?,
                            Some("input_json_delta") => {
                                let tool = self
                                    .tools
                                    .get_mut(&value["index"].as_u64().ok_or(ModelError::Protocol)?)
                                    .ok_or(ModelError::Protocol)?;
                                tool.arguments.push_str(
                                    delta["partial_json"].as_str().ok_or(ModelError::Protocol)?,
                                );
                                if tool.arguments.len() > MAX_FRAME_BYTES {
                                    return Err(ModelError::Limit);
                                }
                            }
                            // Provider signatures and unknown extension blocks are not replayed
                            // as user-visible text or execution instructions.
                            _ => {}
                        }
                    }
                    "content_block_stop" => {
                        if let Some(tool) =
                            value["index"].as_u64().and_then(|i| self.tools.get_mut(&i))
                            && tool.arguments.is_empty()
                        {
                            tool.arguments = "{}".into();
                        }
                    }
                    "message_delta" => {
                        if !self.started {
                            return Err(ModelError::Protocol);
                        }
                        if let Some(reason) = value["delta"]["stop_reason"].as_str() {
                            self.reason = Some(reason.into());
                        }
                        if let Some(tokens) = value["usage"]["output_tokens"].as_u64() {
                            self.usage.output_tokens = Some(tokens);
                        }
                        events.push(ModelEvent::Usage(self.usage.clone()));
                    }
                    "message_stop" => return self.end(),
                    "error" => return Err(ModelError::Protocol),
                    _ => {}
                }
            }
        }
        Ok(events)
    }
}

fn token_count(usage: &Value, key: &str) -> ModelResult<Option<u64>> {
    match usage.get(key).filter(|value| !value.is_null()) {
        None => Ok(None),
        Some(value) => value.as_u64().map(Some).ok_or(ModelError::Protocol),
    }
}
