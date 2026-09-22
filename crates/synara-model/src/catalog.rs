use crate::*;

#[derive(Clone, Debug)]
pub struct CatalogProvider {
    pub id: String,
    pub name: String,
    pub profile: Option<ProviderProfile>,
    pub unsupported_reason: Option<&'static str>,
}
#[derive(Clone, Debug, Default)]
pub struct ProviderCatalog {
    pub providers: Vec<CatalogProvider>,
}

/// Normalize registry metadata by explicit SDK/protocol identifiers, never by a
/// provider name. Unknown transports and multi-field/cloud auth stay unsupported.
/// A catalog entry is a review candidate, not installed or authenticated support.
pub fn parse_catalog(value: Value) -> ModelResult<ProviderCatalog> {
    let providers = value.as_object().ok_or(ModelError::Protocol)?;
    if providers.len() > 2048 {
        return Err(ModelError::Limit);
    }
    let mut catalog = ProviderCatalog::default();
    for (id, provider) in providers {
        if !tool_name(id) {
            continue;
        }
        let name = provider["name"]
            .as_str()
            .filter(|v| bounded_identifier(v))
            .unwrap_or(id);
        let protocol = match provider["npm"].as_str() {
            Some("@ai-sdk/openai-compatible" | "@ai-sdk/openai") => {
                Some(ProtocolFamily::OpenAiChat)
            }
            Some("@ai-sdk/anthropic") => Some(ProtocolFamily::AnthropicMessages),
            _ => None,
        };
        let mut entry = CatalogProvider {
            id: id.clone(),
            name: name.into(),
            profile: None,
            unsupported_reason: None,
        };
        if let Some(protocol) = protocol {
            let candidate = (|| {
                let endpoint = provider["api"]
                    .as_str()
                    .ok_or("no explicit base URL in catalog")?;
                if provider["env"].as_array().is_some_and(|v| v.len() > 1) {
                    return Err("multi-field authentication requires another adapter");
                }
                let source = provider["models"].as_object().ok_or("no model metadata")?;
                if source.len() > 4096 {
                    return Err("provider model catalog exceeds review limit");
                }
                let models = source.iter().filter_map(|(id, m)| {
                    if !bounded_identifier(id) { return None; }
                    let flag = |v: &Value| match v.as_bool() { Some(true) => Support::Supported, Some(false) => Support::Unsupported, None => Support::Unknown };
                    let images = m["modalities"]["input"].as_array().map_or(Support::Unknown, |inputs| if inputs.iter().any(|v| v == "image") { Support::Supported } else { Support::Unsupported });
                    Some(ModelInfo { id:id.clone(), name:m["name"].as_str().filter(|v| bounded_identifier(v)).unwrap_or(id).into(), capabilities:ModelCapabilities {
                        tools:flag(&m["tool_call"]), structured_output:flag(&m["structured_output"]), images,
                        reasoning_efforts:vec![], // Boolean reasoning metadata does not report effort values.
                        context_window:m["limit"]["context"].as_u64().filter(|n| *n > 0),
                        max_output_tokens:m["limit"]["output"].as_u64().filter(|n| *n > 0),
                        source:"models.dev community metadata, not live capability verification".into(),
                    } })
                }).collect();
                let profile = ProviderProfile {
                    id: id.clone(),
                    name: name.into(),
                    protocol,
                    endpoint: endpoint.into(),
                    allow_loopback_http: false,
                    requires_key: true,
                    models,
                };
                profile
                    .validate()
                    .map_err(|_| "endpoint or model metadata needs manual configuration")?;
                Ok(profile)
            })();
            match candidate {
                Ok(profile) => entry.profile = Some(profile),
                Err(reason) => entry.unsupported_reason = Some(reason),
            }
        } else {
            entry.unsupported_reason =
                Some("transport is not implemented by this direct-runtime slice");
        }
        catalog.providers.push(entry);
    }
    catalog
        .providers
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(catalog)
}

pub fn custom_profile_example() -> ProviderProfile {
    ProviderProfile {
        id: "local-compatible".into(),
        name: "Local compatible server".into(),
        protocol: ProtocolFamily::OpenAiChat,
        endpoint: "http://127.0.0.1:11434/v1".into(),
        allow_loopback_http: true,
        requires_key: false,
        models: vec![ModelInfo {
            id: "replace-with-model-id".into(),
            name: "Replace with installed model".into(),
            capabilities: ModelCapabilities::default(),
        }],
    }
}
