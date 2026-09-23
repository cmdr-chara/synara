use crate::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use synara_runtime::SecretReference;
use url::{Host, Url};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolFamily {
    OpenAiChat,
    AnthropicMessages,
    GoogleGenerateContent,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    Supported,
    Unsupported,
    #[default]
    Unknown,
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModelCapabilities {
    pub tools: Support,
    pub structured_output: Support,
    pub images: Support,
    /// Only explicitly documented effort values. A reasoning flag is insufficient.
    pub reasoning_efforts: Vec<String>,
    pub context_window: Option<u64>,
    pub max_output_tokens: Option<u64>,
    pub source: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub capabilities: ModelCapabilities,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderProfile {
    pub id: String,
    pub name: String,
    pub protocol: ProtocolFamily,
    /// API base, including /v1 where required. Not a completion URL.
    pub endpoint: String,
    /// An explicit opt-in restricted to numeric loopback addresses.
    #[serde(default)]
    pub allow_loopback_http: bool,
    /// False is useful for an explicitly selected local server. Not inferred from the name.
    pub requires_key: bool,
    pub models: Vec<ModelInfo>,
}
impl ProviderProfile {
    pub fn validate(&self) -> ModelResult<()> {
        if !tool_name(&self.id)
            || !bounded_identifier(&self.name)
            || self.models.is_empty()
            || self.models.len() > 4096
        {
            return Err(ModelError::Invalid("profile identity or model count"));
        }
        self.base_url()?;
        let mut seen = HashSet::new();
        for model in &self.models {
            if !bounded_identifier(&model.id)
                || (self.protocol == ProtocolFamily::GoogleGenerateContent
                    && crate::google::model_id(&model.id).is_err())
                || !bounded_identifier(&model.name)
                || !seen.insert(&model.id)
                || model.capabilities.source.len() > 1024
                || model.capabilities.reasoning_efforts.len() > 16
                || model
                    .capabilities
                    .reasoning_efforts
                    .iter()
                    .any(|v| !tool_name(v))
                || model.capabilities.context_window == Some(0)
                || model.capabilities.max_output_tokens == Some(0)
            {
                return Err(ModelError::Invalid("model metadata"));
            }
        }
        Ok(())
    }
    pub fn base_url(&self) -> ModelResult<Url> {
        if self.endpoint.len() > 2048 {
            return Err(ModelError::Invalid("endpoint length"));
        }
        let mut url =
            Url::parse(&self.endpoint).map_err(|_| ModelError::Invalid("endpoint URL"))?;
        if url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
            || url.path().contains('%')
        {
            return Err(ModelError::Invalid(
                "endpoint must not contain credentials, query, fragment or encoded path",
            ));
        }
        let loopback = match url.host() {
            Some(Host::Ipv4(ip)) => ip.is_loopback(),
            Some(Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if url.scheme() != "https"
            && !(url.scheme() == "http" && self.allow_loopback_http && loopback)
        {
            return Err(ModelError::Invalid(
                "HTTPS required, except explicitly approved numeric loopback HTTP",
            ));
        }
        let path = format!("{}/", url.path().trim_end_matches('/'));
        url.set_path(&path);
        Ok(url)
    }
    /// The complete canonical base URL and transport bind the secret. Editing an
    /// endpoint never carries an existing key to another destination automatically.
    pub fn secret_reference(&self) -> ModelResult<SecretReference> {
        let scope = format!("{}\n{:?}\n{}", self.id, self.protocol, self.base_url()?);
        SecretReference::new(
            "synara.direct-model",
            hex::encode(Sha256::digest(scope.as_bytes())),
        )
        .map_err(|_| ModelError::Invalid("secret reference"))
    }
    pub fn model(&self, id: &str) -> ModelResult<&ModelInfo> {
        self.models
            .iter()
            .find(|m| m.id == id)
            .ok_or(ModelError::Invalid("model is not in the reviewed profile"))
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProviderSettings {
    pub revision: u64,
    pub providers: Vec<ProviderProfile>,
}
impl ProviderSettings {
    pub fn validate(&self) -> ModelResult<()> {
        if self.providers.len() > 256 {
            return Err(ModelError::Limit);
        }
        let mut ids = HashSet::new();
        for provider in &self.providers {
            provider.validate()?;
            if !ids.insert(&provider.id) {
                return Err(ModelError::Invalid("duplicate provider profile"));
            }
        }
        if serde_json::to_vec(self)
            .map_err(|_| ModelError::Invalid("settings"))?
            .len()
            > MAX_REQUEST_BYTES
        {
            return Err(ModelError::Limit);
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSelection {
    /// None retains all history. Some(0) sends only the new prompt.
    /// A positive value retains complete prior user turns, never hidden state.
    #[serde(default)]
    pub history_turns: Option<u16>,
    pub provider_id: String,
    pub model_id: String,
    pub max_output_tokens: u32,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub output: OutputFormat,
}
impl ModelSelection {
    pub fn request(&self, messages: Vec<Message>) -> ModelRequest {
        ModelRequest {
            model: self.model_id.clone(),
            messages,
            tools: vec![],
            output: self.output.clone(),
            reasoning_effort: self.reasoning_effort.clone(),
            max_output_tokens: self.max_output_tokens,
        }
    }
}

pub fn validate_request(profile: &ProviderProfile, request: &ModelRequest) -> ModelResult<()> {
    profile.validate()?;
    let caps = &profile.model(&request.model)?.capabilities;
    if request.messages.is_empty()
        || request.messages.len() > 4096
        || request.tools.len() > 64
        || request.max_output_tokens == 0
        || request.max_output_tokens > 131072
        || caps
            .max_output_tokens
            .is_some_and(|n| u64::from(request.max_output_tokens) > n)
        || serde_json::to_vec(request)
            .map_err(|_| ModelError::Invalid("request"))?
            .len()
            > MAX_INPUT_BYTES
    {
        return Err(ModelError::Limit);
    }
    if request
        .reasoning_effort
        .as_ref()
        .is_some_and(|v| !caps.reasoning_efforts.contains(v))
    {
        return Err(ModelError::Unsupported("reasoning effort"));
    }
    if request.reasoning_effort.is_some() && profile.protocol != ProtocolFamily::OpenAiChat {
        return Err(ModelError::Unsupported("reasoning effort transport"));
    }
    if !matches!(request.output, OutputFormat::Text) {
        if caps.structured_output != Support::Supported
            || profile.protocol == ProtocolFamily::AnthropicMessages
        {
            return Err(ModelError::Unsupported("JSON schema output"));
        }
        if let OutputFormat::JsonSchema { name, schema } = &request.output
            && (!tool_name(name) || !schema.is_object())
        {
            return Err(ModelError::Invalid("output schema"));
        }
        if let OutputFormat::JsonSchema { schema, .. } = &request.output {
            crate::schema::check_schema(schema)?;
        }
    }
    let mut tools = HashSet::new();
    for tool in &request.tools {
        if !tool_name(&tool.name)
            || !tool.parameters.is_object()
            || tool.description.len() > 8192
            || !tools.insert(&tool.name)
        {
            return Err(ModelError::Invalid("tool definition"));
        }
    }
    let uses_tools = !request.tools.is_empty()
        || request
            .messages
            .iter()
            .any(|m| m.role == MessageRole::Tool || !m.tool_calls.is_empty());
    // Google tool history requires opaque thought signatures. Do not silently
    // drop that protocol state or claim this text runtime can replay its tools.
    if uses_tools && profile.protocol == ProtocolFamily::GoogleGenerateContent {
        return Err(ModelError::Unsupported("Google tool/signature round trips"));
    }
    if uses_tools && caps.tools != Support::Supported {
        return Err(ModelError::Unsupported("tools"));
    }
    let mut outstanding = HashSet::new();
    let mut all_ids = HashSet::new();
    for message in &request.messages {
        if message.content.is_empty() && message.tool_calls.is_empty() {
            return Err(ModelError::Invalid("empty message"));
        }
        if message.role == MessageRole::Tool {
            let id = message
                .tool_call_id
                .as_ref()
                .ok_or(ModelError::Invalid("tool result requires an ID"))?;
            if !outstanding.remove(id) || !message.tool_calls.is_empty() {
                return Err(ModelError::Invalid("unmatched tool result"));
            }
        } else if message.tool_call_id.is_some() || !outstanding.is_empty() {
            return Err(ModelError::Invalid(
                "tool results must precede the next message",
            ));
        }
        if !message.tool_calls.is_empty() && message.role != MessageRole::Assistant {
            return Err(ModelError::Invalid("tool call role"));
        }
        for call in &message.tool_calls {
            if !bounded_identifier(&call.id)
                || !tool_name(&call.name)
                || !call.arguments.is_object()
                || !all_ids.insert(call.id.clone())
            {
                return Err(ModelError::Invalid("tool call"));
            }
            outstanding.insert(call.id.clone());
        }
        for content in &message.content {
            if let Content::Image { media_type, base64 } = content {
                if caps.images != Support::Supported {
                    return Err(ModelError::Unsupported("images"));
                }
                if message.role != MessageRole::User
                    || !matches!(media_type.as_str(), "image/png" | "image/jpeg")
                    || !crate::input::canonical_base64(base64)
                {
                    return Err(ModelError::Invalid("inline image"));
                }
            }
        }
    }
    if !outstanding.is_empty() {
        return Err(ModelError::Invalid("missing tool results"));
    }
    Ok(())
}
