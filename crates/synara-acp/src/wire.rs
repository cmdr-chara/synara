use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
};
use synara_agent::{AgentError, AgentResult, ContextServer, Prompt, PromptPart, SessionOptions};
use synara_core::*;

pub(crate) fn invalid(message: impl Into<String>) -> AgentError {
    AgentError::Invalid(message.into())
}
pub(crate) fn string<'a>(value: &'a Value, key: &str) -> AgentResult<&'a str> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(format!("missing or invalid {key}")))
}
pub(crate) fn id(value: &Value, key: &str) -> AgentResult<String> {
    let text = string(value, key)?;
    if text.is_empty() || text.len() > 512 || text.contains('\0') {
        return Err(invalid(format!("invalid {key}")));
    }
    Ok(text.into())
}
pub(crate) fn optional_string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_owned)
}
pub(crate) fn array<'a>(value: &'a Value, key: &str, max: usize) -> AgentResult<&'a [Value]> {
    let items = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| invalid(format!("missing or invalid {key}")))?;
    if items.len() > max {
        return Err(AgentError::Limit);
    }
    Ok(items)
}
fn optional_array<'a>(value: &'a Value, key: &str, max: usize) -> AgentResult<&'a [Value]> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(&[]),
        _ => array(value, key, max),
    }
}
pub(crate) fn path_text(path: &Path) -> AgentResult<&str> {
    let text = path.to_str().ok_or_else(|| invalid("path is not UTF-8"))?;
    if text.is_empty() || text.contains('\0') || text.len() > 32_768 {
        return Err(invalid("invalid path"));
    }
    Ok(text)
}

pub(crate) fn initialize_params(local_callbacks: bool) -> Value {
    json!({
        "protocolVersion":1,
        "clientInfo":{"name":"synara", "title":"Synara", "version":env!("CARGO_PKG_VERSION")},
        "clientCapabilities":{
            "fs":{"readTextFile":local_callbacks, "writeTextFile":local_callbacks},
            "terminal":local_callbacks,
            "elicitation":{"form":{}, "url":{}},
            "session":{"configOptions":{"boolean":{}}}
        }
    })
}

pub(crate) fn initialization(
    value: &Value,
) -> AgentResult<(AgentCapabilities, Option<AgentIdentity>, Vec<AuthMethod>)> {
    if value.get("protocolVersion").and_then(Value::as_u64) != Some(1) {
        return Err(invalid("agent did not negotiate stable protocol version 1"));
    }
    let source = value
        .get("agentCapabilities")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !source.is_object() {
        return Err(invalid("invalid agent capabilities"));
    }
    let supports = |pointer: &str| source.pointer(pointer).is_some_and(Value::is_object);
    let enabled = |pointer: &str| source.pointer(pointer).and_then(Value::as_bool) == Some(true);
    let capabilities = AgentCapabilities {
        load_session: enabled("/loadSession"),
        resume_session: supports("/sessionCapabilities/resume"),
        close_session: supports("/sessionCapabilities/close"),
        list_sessions: supports("/sessionCapabilities/list"),
        delete_session: supports("/sessionCapabilities/delete"),
        logout: supports("/auth/logout"),
        image_prompts: enabled("/promptCapabilities/image"),
        audio_prompts: enabled("/promptCapabilities/audio"),
        embedded_context: enabled("/promptCapabilities/embeddedContext"),
        mcp_http: enabled("/mcpCapabilities/http"),
        mcp_sse: enabled("/mcpCapabilities/sse"),
        additional_directories: supports("/sessionCapabilities/additionalDirectories"),
    };
    let identity = value
        .get("agentInfo")
        .filter(|item| !item.is_null())
        .map(|item| {
            Ok::<_, AgentError>(AgentIdentity {
                name: id(item, "name")?,
                title: optional_string(item, "title"),
                version: string(item, "version")?.into(),
            })
        })
        .transpose()?;
    let mut seen = HashSet::new();
    let mut methods = Vec::new();
    for method in optional_array(value, "authMethods", 64)? {
        // Interactive login requires a separate terminal launch and reconnect. Do not claim it here.
        if method
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind != "agent")
        {
            continue;
        }
        let id = id(method, "id")?;
        if !seen.insert(id.clone()) {
            return Err(invalid("duplicate authentication method ID"));
        }
        methods.push(AuthMethod {
            id,
            name: string(method, "name")?.into(),
            description: optional_string(method, "description"),
        });
    }
    Ok((capabilities, identity, methods))
}

pub(crate) fn session_params(
    options: &SessionOptions,
    capabilities: &AgentCapabilities,
) -> AgentResult<Value> {
    if options.additional_directories.len() > 32 || options.context_servers.len() > 32 {
        return Err(AgentError::Limit);
    }
    if !options.additional_directories.is_empty() && !capabilities.additional_directories {
        return Err(AgentError::Unsupported(
            "additional working directories".into(),
        ));
    }
    let mut servers = Vec::new();
    for server in &options.context_servers {
        let value = match server {
            ContextServer::Process {
                name,
                command,
                args,
                env,
            } => {
                if !Path::new(command).is_absolute() {
                    return Err(invalid("context-server command must be absolute"));
                }
                json!({"name":name,"command":command,"args":args,"env":pairs(env)})
            }
            ContextServer::Http { name, url, headers } => {
                if !capabilities.mcp_http {
                    return Err(AgentError::Unsupported("HTTP context servers".into()));
                }
                json!({"type":"http","name":name,"url":url,"headers":pairs(headers)})
            }
            ContextServer::ServerSentEvents { name, url, headers } => {
                if !capabilities.mcp_sse {
                    return Err(AgentError::Unsupported("SSE context servers".into()));
                }
                json!({"type":"sse","name":name,"url":url,"headers":pairs(headers)})
            }
        };
        servers.push(value);
    }
    let mut params = json!({"cwd":path_text(&options.cwd)?, "mcpServers":servers});
    if !options.additional_directories.is_empty() {
        let dirs: Vec<_> = options
            .additional_directories
            .iter()
            .map(|path| path_text(path))
            .collect::<AgentResult<_>>()?;
        params["additionalDirectories"] = json!(dirs);
    }
    Ok(params)
}
fn pairs(map: &BTreeMap<String, String>) -> Vec<Value> {
    map.iter()
        .map(|(name, value)| json!({"name":name,"value":value}))
        .collect()
}

pub(crate) fn prompt_parts(
    prompt: &Prompt,
    capabilities: &AgentCapabilities,
) -> AgentResult<Value> {
    if prompt.parts.is_empty() || prompt.parts.len() > 64 {
        return Err(invalid("prompt must contain 1 to 64 parts"));
    }
    let mut total = 0_usize;
    let mut values = Vec::new();
    for part in &prompt.parts {
        let value = match part {
            PromptPart::Text(text) => {
                total = total.saturating_add(text.len());
                json!({"type":"text","text":text})
            }
            PromptPart::Image { base64, mime_type } => {
                if !capabilities.image_prompts {
                    return Err(AgentError::Unsupported("image prompts".into()));
                }
                total = total.saturating_add(base64.len());
                json!({"type":"image","data":base64,"mimeType":mime_type})
            }
            PromptPart::Audio { base64, mime_type } => {
                if !capabilities.audio_prompts {
                    return Err(AgentError::Unsupported("audio prompts".into()));
                }
                total = total.saturating_add(base64.len());
                json!({"type":"audio","data":base64,"mimeType":mime_type})
            }
            PromptPart::Context {
                uri,
                text,
                mime_type,
            } => {
                if !capabilities.embedded_context {
                    return Err(AgentError::Unsupported("embedded context".into()));
                }
                total = total.saturating_add(text.len());
                json!({"type":"resource","resource":{"uri":uri,"text":text,"mimeType":mime_type}})
            }
        };
        if total > 4 * 1024 * 1024 {
            return Err(AgentError::Limit);
        }
        values.push(value);
    }
    Ok(json!(values))
}

pub(crate) fn configuration(
    value: &Value,
    previous: &SessionConfiguration,
) -> AgentResult<SessionConfiguration> {
    let mut configuration = previous.clone();
    if value
        .get("configOptions")
        .is_some_and(|value| !value.is_null())
    {
        let mut seen = HashSet::new();
        configuration.options = array(value, "configOptions", 128)?
            .iter()
            .filter_map(|item| {
                let kind = item.get("type").and_then(Value::as_str);
                if !matches!(kind, Some("select" | "boolean")) {
                    return None;
                }
                Some((|| {
                    let id = id(item, "id")?;
                    if !seen.insert(id.clone()) {
                        return Err(invalid("duplicate configuration ID"));
                    }
                    let (current, choices) = match kind {
                        Some("boolean") => (
                            ConfigValue::Boolean {
                                value: item
                                    .get("currentValue")
                                    .and_then(Value::as_bool)
                                    .ok_or_else(|| invalid("invalid boolean option"))?,
                            },
                            vec![],
                        ),
                        _ => (
                            ConfigValue::Select {
                                value: string(item, "currentValue")?.into(),
                            },
                            choices(array(item, "options", 512)?, None)?,
                        ),
                    };
                    Ok(SessionOption {
                        id,
                        name: string(item, "name")?.into(),
                        description: optional_string(item, "description"),
                        category: optional_string(item, "category"),
                        current,
                        choices,
                    })
                })())
            })
            .collect::<AgentResult<_>>()?;
    }
    if let Some(modes) = value.get("modes").filter(|item| item.is_object()) {
        configuration.current_mode = optional_string(modes, "currentModeId");
        configuration.modes = array(modes, "availableModes", 128)?
            .iter()
            .map(|item| {
                Ok(SessionMode {
                    id: id(item, "id")?,
                    name: string(item, "name")?.into(),
                    description: optional_string(item, "description"),
                })
            })
            .collect::<AgentResult<_>>()?;
    }
    if let Some(mode) = value.get("currentModeId").and_then(Value::as_str) {
        configuration.current_mode = Some(mode.into());
    }
    Ok(configuration)
}
fn choices(items: &[Value], group: Option<&str>) -> AgentResult<Vec<SelectChoice>> {
    let mut output = Vec::new();
    for item in items {
        if let Some(group_items) = item.get("options") {
            if group.is_some() {
                return Err(invalid("nested option groups are not supported"));
            }
            let group_items = group_items
                .as_array()
                .ok_or_else(|| invalid("invalid option group"))?;
            if group_items.len() > 512 {
                return Err(AgentError::Limit);
            }
            output.extend(choices(group_items, Some(string(item, "name")?))?);
        } else {
            output.push(SelectChoice {
                value: id(item, "value")?,
                label: string(item, "name")?.into(),
                group: group.map(str::to_owned),
            });
        }
        if output.len() > 512 {
            return Err(AgentError::Limit);
        }
    }
    let mut unique = HashSet::new();
    if output.iter().any(|item| !unique.insert(item.value.clone())) {
        return Err(invalid("duplicate option value"));
    }
    Ok(output)
}

pub(crate) fn update(
    value: &Value,
    configuration_state: &SessionConfiguration,
) -> AgentResult<Vec<ThreadEvent>> {
    let mut events = Vec::new();
    match string(value, "sessionUpdate")? {
        "agent_message_chunk" | "agent_thought_chunk" | "user_message_chunk" => {
            let role = match string(value, "sessionUpdate")? {
                "user_message_chunk" => Role::User,
                "agent_thought_chunk" => Role::Reasoning,
                _ => Role::Assistant,
            };
            let content = value
                .get("content")
                .ok_or_else(|| invalid("message content missing"))?;
            if content.get("type").and_then(Value::as_str) == Some("text") {
                events.push(ThreadEvent::TextDelta {
                    message_id: optional_string(value, "messageId"),
                    role,
                    text: string(content, "text")?.into(),
                });
            } else {
                events.push(ThreadEvent::Notice { message: "The agent sent non-text message content. This message type is not yet rendered.".into() });
            }
        }
        "tool_call" | "tool_call_update" => {
            let mut patch = tool_patch(value)?;
            if string(value, "sessionUpdate")? == "tool_call" {
                if patch.title.is_none() {
                    return Err(invalid("new tool call has no title"));
                }
                if patch.status.is_none() {
                    patch.status = Some(ToolStatus::Pending);
                }
            }
            events.push(ThreadEvent::ToolChanged { patch });
        }
        "plan" => {
            let entries = array(value, "entries", 1024)?
                .iter()
                .map(|item| {
                    Ok(PlanEntry {
                        text: string(item, "content")?.into(),
                        status: string(item, "status")?.into(),
                        priority: string(item, "priority")?.into(),
                    })
                })
                .collect::<AgentResult<_>>()?;
            events.push(ThreadEvent::PlanChanged { entries });
        }
        "usage_update" => {
            let usage = Usage {
                context_used: value.get("used").and_then(Value::as_u64),
                context_limit: value.get("size").and_then(Value::as_u64),
                cost_amount: value.pointer("/cost/amount").and_then(Value::as_f64),
                cost_currency: value
                    .pointer("/cost/currency")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                ..Usage::default()
            };
            events.push(ThreadEvent::UsageChanged { usage });
        }
        "config_option_update" | "current_mode_update" => {
            events.push(ThreadEvent::ConfigurationChanged {
                configuration: configuration(value, configuration_state)?,
            })
        }
        "available_commands_update" => {
            let commands = array(value, "availableCommands", 512)?
                .iter()
                .map(|item| {
                    Ok(SlashCommand {
                        name: string(item, "name")?.into(),
                        description: string(item, "description")?.into(),
                        argument_hint: item
                            .pointer("/input/hint")
                            .and_then(Value::as_str)
                            .map(str::to_owned),
                    })
                })
                .collect::<AgentResult<_>>()?;
            events.push(ThreadEvent::CommandsChanged { commands });
        }
        "session_info_update" if value.get("title").is_some() => {
            events.push(ThreadEvent::TitleChanged {
                title: optional_string(value, "title").unwrap_or_default(),
            });
        }
        _ => {} // Future or draft extensions do not leak into the domain API.
    }
    Ok(events)
}
pub(crate) fn tool_patch(value: &Value) -> AgentResult<ToolPatch> {
    let status = value
        .get("status")
        .filter(|item| !item.is_null())
        .map(|status| match status.as_str() {
            Some("pending") => Ok(ToolStatus::Pending),
            Some("in_progress") => Ok(ToolStatus::Running),
            Some("completed") => Ok(ToolStatus::Completed),
            Some("failed") => Ok(ToolStatus::Failed),
            _ => Err(invalid("invalid tool status")),
        })
        .transpose()?;
    let output = value
        .get("content")
        .filter(|item| !item.is_null())
        .map(|_| {
            array(value, "content", 512)?
                .iter()
                .map(|content| {
                    Ok(match string(content, "type")? {
                        "content" => {
                            let block = content
                                .get("content")
                                .ok_or_else(|| invalid("tool content missing"))?;
                            match block.get("type").and_then(Value::as_str) {
                                Some("text") => ToolOutput::Text {
                                    text: string(block, "text")?.into(),
                                },
                                Some("resource_link") => ToolOutput::Resource {
                                    uri: string(block, "uri")?.into(),
                                    name: string(block, "name")?.into(),
                                },
                                _ => ToolOutput::Text {
                                    text: "Non-text tool output is available from the agent."
                                        .into(),
                                },
                            }
                        }
                        "diff" => ToolOutput::Diff {
                            path: string(content, "path")?.into(),
                            before: optional_string(content, "oldText"),
                            after: optional_string(content, "newText"),
                        },
                        "terminal" => ToolOutput::Terminal {
                            id: id(content, "terminalId")?,
                        },
                        _ => ToolOutput::Text {
                            text: "Unsupported tool output type.".into(),
                        },
                    })
                })
                .collect::<AgentResult<Vec<_>>>()
        })
        .transpose()?;
    Ok(ToolPatch {
        id: id(value, "toolCallId")?,
        title: optional_string(value, "title"),
        status,
        kind: optional_string(value, "kind"),
        output,
    })
}
pub(crate) fn permission(value: &Value, request_id: String) -> AgentResult<PermissionRequest> {
    let tool = value
        .get("toolCall")
        .ok_or_else(|| invalid("permission tool missing"))?;
    let choices = array(value, "options", 32)?
        .iter()
        .map(|item| {
            let kind = match string(item, "kind")? {
                "allow_once" => PermissionKind::AllowOnce,
                "allow_always" => PermissionKind::AllowAlways,
                "reject_once" => PermissionKind::DenyOnce,
                "reject_always" => PermissionKind::DenyAlways,
                _ => return Err(invalid("unsupported permission kind")),
            };
            Ok(PermissionChoice {
                id: id(item, "optionId")?,
                label: string(item, "name")?.into(),
                kind,
            })
        })
        .collect::<AgentResult<Vec<_>>>()?;
    let mut seen = HashSet::new();
    if choices.is_empty() || choices.iter().any(|choice| !seen.insert(choice.id.clone())) {
        return Err(invalid("invalid permission choices"));
    }
    Ok(PermissionRequest {
        id: request_id,
        tool_id: Some(id(tool, "toolCallId")?),
        title: optional_string(tool, "title").unwrap_or_else(|| "Agent requests permission".into()),
        choices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capabilities_use_protocol_advertisements_not_agent_names() {
        let (caps, _, methods) = initialization(&json!({"protocolVersion":1,"agentCapabilities":{"loadSession":true,"sessionCapabilities":{"list":{},"resume":null},"auth":{"logout":{}}},"authMethods":[{"id":"login","name":"Login"},{"id":"terminal","type":"terminal","name":"Terminal login"}]})).unwrap();
        assert!(caps.load_session && caps.list_sessions && caps.logout);
        assert!(!caps.resume_session && !caps.image_prompts);
        assert_eq!(methods.len(), 1);
        assert!(initialization(&json!({"protocolVersion":2})).is_err());
    }
    #[test]
    fn grouped_and_boolean_configuration_normalizes_without_raw_schema() {
        let config = configuration(&json!({"configOptions":[{"id":"model","name":"Model","type":"select","currentValue":"a","options":[{"group":"g","name":"Local","options":[{"value":"a","name":"A"}]}]},{"id":"fast","name":"Fast","type":"boolean","currentValue":false}]}), &SessionConfiguration::default()).unwrap();
        assert_eq!(config.options[0].choices[0].group.as_deref(), Some("Local"));
        assert_eq!(
            config.options[1].current,
            ConfigValue::Boolean { value: false }
        );
    }
    #[test]
    fn removed_legacy_model_fields_do_not_enter_the_domain_configuration() {
        let previous = SessionConfiguration {
            current_model: Some("domain-model".into()),
            models: vec![SelectChoice {
                value: "domain-model".into(),
                label: "Domain model".into(),
                group: None,
            }],
            ..SessionConfiguration::default()
        };
        let config = configuration(
            &json!({
                "models":{
                    "currentModelId":"legacy",
                    "availableModels":[{"modelId":"legacy","name":"Legacy"}]
                },
                "configOptions":[{
                    "id":"model",
                    "name":"Model",
                    "category":"model",
                    "type":"select",
                    "currentValue":"stable",
                    "options":[{"value":"stable","name":"Stable"}]
                }]
            }),
            &previous,
        )
        .unwrap();
        assert_eq!(config.current_model.as_deref(), Some("domain-model"));
        assert_eq!(config.models, previous.models);
        assert_eq!(config.options[0].category.as_deref(), Some("model"));
    }

    #[test]
    fn negotiated_session_parameters_cover_directories_and_mcp_transports() {
        let root = std::env::temp_dir().join("synara-acp-session-params");
        let mut options = SessionOptions::new(ThreadId::new(), root.clone());
        options.additional_directories.push(root.join("extra"));
        options.context_servers.push(ContextServer::Process {
            name: "process".into(),
            command: std::env::current_exe()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            args: vec!["--fixture".into()],
            env: BTreeMap::from([("MODE".into(), "test".into())]),
        });
        options.context_servers.push(ContextServer::Http {
            name: "http".into(),
            url: "https://example.invalid/mcp".into(),
            headers: BTreeMap::from([("X-Test".into(), "value".into())]),
        });
        options.context_servers.push(ContextServer::ServerSentEvents {
            name: "sse".into(),
            url: "https://example.invalid/events".into(),
            headers: BTreeMap::new(),
        });
        let capabilities = AgentCapabilities {
            additional_directories: true,
            mcp_http: true,
            mcp_sse: true,
            ..AgentCapabilities::default()
        };
        let params = session_params(&options, &capabilities).unwrap();
        assert_eq!(params["additionalDirectories"].as_array().unwrap().len(), 1);
        assert_eq!(params["mcpServers"].as_array().unwrap().len(), 3);
        assert_eq!(params["mcpServers"][1]["type"], "http");
        assert_eq!(params["mcpServers"][2]["type"], "sse");

        let mut unsupported = capabilities.clone();
        unsupported.mcp_http = false;
        assert!(matches!(
            session_params(&options, &unsupported),
            Err(AgentError::Unsupported(_))
        ));
        unsupported.mcp_http = true;
        unsupported.additional_directories = false;
        assert!(matches!(
            session_params(&options, &unsupported),
            Err(AgentError::Unsupported(_))
        ));
    }

    #[test]
    fn dynamic_configuration_and_commands_translate_without_protocol_types() {
        let previous = configuration(
            &json!({
                "configOptions":[{
                    "id":"review",
                    "name":"Review",
                    "type":"boolean",
                    "currentValue":true
                }]
            }),
            &SessionConfiguration::default(),
        )
        .unwrap();
        let events = update(
            &json!({
                "sessionUpdate":"config_option_update",
                "configOptions":[{
                    "id":"review",
                    "name":"Review",
                    "type":"boolean",
                    "currentValue":false
                }]
            }),
            &previous,
        )
        .unwrap();
        let ThreadEvent::ConfigurationChanged { configuration } = &events[0] else {
            panic!("configuration update expected")
        };
        assert_eq!(
            configuration.options[0].current,
            ConfigValue::Boolean { value: false }
        );

        let events = update(
            &json!({
                "sessionUpdate":"available_commands_update",
                "availableCommands":[{
                    "name":"review",
                    "description":"Review changes",
                    "input":{"hint":"path"}
                }]
            }),
            configuration,
        )
        .unwrap();
        let ThreadEvent::CommandsChanged { commands } = &events[0] else {
            panic!("commands update expected")
        };
        assert_eq!(commands[0].name, "review");
        assert_eq!(commands[0].argument_hint.as_deref(), Some("path"));
    }

    #[test]
    fn partial_tool_patch_does_not_invent_empty_content() {
        let patch = tool_patch(&json!({"toolCallId":"a","status":"completed"})).unwrap();
        assert!(patch.output.is_none() && patch.title.is_none());
        assert_eq!(patch.status, Some(ToolStatus::Completed));
    }
    #[test]
    fn unsupported_prompt_content_is_rejected_before_transport() {
        let prompt = Prompt {
            parts: vec![PromptPart::Image {
                base64: "abc".into(),
                mime_type: "image/png".into(),
            }],
        };
        assert!(matches!(
            prompt_parts(&prompt, &AgentCapabilities::default()),
            Err(AgentError::Unsupported(_))
        ));
    }
}
