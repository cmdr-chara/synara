use crate::{
    protocol::encode,
    stream::{ProtocolDecoder, SseDecoder},
    *,
};
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{Client, RequestBuilder, header::HeaderValue};
use std::time::Duration;
use synara_runtime::SecretStore;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    /// Runs one request, never retries, never executes returned tools. Dropping the
    /// future or cancelling closes its response body and all request-owned work.
    async fn stream(
        &self,
        profile: &ProviderProfile,
        request: ModelRequest,
        secrets: &dyn SecretStore,
        cancellation: CancellationToken,
        events: mpsc::Sender<ModelEvent>,
    ) -> ModelResult<()>;
    async fn discover_models(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
        cancellation: CancellationToken,
    ) -> ModelResult<Vec<ModelInfo>>;
}
#[derive(Clone)]
pub struct HttpModelProvider {
    client: Client,
}
impl HttpModelProvider {
    pub fn new() -> ModelResult<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .pool_max_idle_per_host(2)
            .build()
            .map_err(|_| ModelError::Transport)?;
        Ok(Self { client })
    }
    async fn authorize(
        &self,
        request: RequestBuilder,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
    ) -> ModelResult<RequestBuilder> {
        if !profile.requires_key {
            return Ok(request);
        }
        let secret = secrets
            .read(&profile.secret_reference()?)
            .await
            .map_err(|_| ModelError::Credential)?
            .ok_or(ModelError::Credential)?;
        let key = std::str::from_utf8(secret.expose()).map_err(|_| ModelError::Credential)?;
        if key.trim() != key
            || key.is_empty()
            || key.len() > 8192
            || !key.bytes().all(|b| (33..=126).contains(&b))
        {
            return Err(ModelError::Credential);
        }
        let (name, mut header) = match profile.protocol {
            ProtocolFamily::OpenAiChat => (
                "authorization",
                HeaderValue::from_str(&format!("Bearer {key}"))
                    .map_err(|_| ModelError::Credential)?,
            ),
            ProtocolFamily::AnthropicMessages => (
                "x-api-key",
                HeaderValue::from_str(key).map_err(|_| ModelError::Credential)?,
            ),
        };
        header.set_sensitive(true);
        Ok(request.header(name, header))
    }
    async fn run(
        &self,
        profile: &ProviderProfile,
        request: ModelRequest,
        secrets: &dyn SecretStore,
        events: mpsc::Sender<ModelEvent>,
    ) -> ModelResult<()> {
        let body = encode(profile, &request)?;
        let path = match profile.protocol {
            ProtocolFamily::OpenAiChat => "chat/completions",
            ProtocolFamily::AnthropicMessages => "messages",
        };
        let url = profile
            .base_url()?
            .join(path)
            .map_err(|_| ModelError::Invalid("request URL"))?;
        let mut builder = self
            .client
            .post(url)
            .header("accept", "text/event-stream")
            .json(&body);
        if profile.protocol == ProtocolFamily::AnthropicMessages {
            builder = builder.header("anthropic-version", "2023-06-01");
        }
        let response = self
            .authorize(builder, profile, secrets)
            .await?
            .send()
            .await
            .map_err(|_| ModelError::Transport)?;
        if !response.status().is_success() {
            return Err(ModelError::Http(response.status().as_u16()));
        }
        if response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .is_none_or(|s| {
                !s.split(';')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .eq_ignore_ascii_case("text/event-stream")
            })
        {
            return Err(ModelError::Protocol);
        }
        let mut stream = response.bytes_stream();
        let mut sse = SseDecoder::default();
        let mut decoder = ProtocolDecoder::new(profile.protocol, &request);
        let mut received = 0usize;
        let mut event_count = 0usize;
        while let Some(chunk) = tokio::time::timeout(Duration::from_secs(45), stream.next())
            .await
            .map_err(|_| ModelError::Transport)?
        {
            let chunk = chunk.map_err(|_| ModelError::Transport)?;
            received = received.checked_add(chunk.len()).ok_or(ModelError::Limit)?;
            if received > MAX_RESPONSE_BYTES * 4 {
                return Err(ModelError::Limit);
            }
            for data in sse.push(&chunk)? {
                for event in decoder.push(&data)? {
                    event_count += 1;
                    if event_count > 50000 {
                        return Err(ModelError::Limit);
                    }
                    if matches!(event, ModelEvent::Finished { .. })
                        && !matches!(request.output, OutputFormat::Text)
                        && serde_json::from_str::<Value>(&decoder.text).is_err()
                    {
                        return Err(ModelError::Protocol);
                    }
                    events
                        .send(event)
                        .await
                        .map_err(|_| ModelError::Cancelled)?;
                }
                if decoder.finished {
                    return Ok(());
                }
            }
        }
        Err(ModelError::Protocol)
    }
    async fn models(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
    ) -> ModelResult<Vec<ModelInfo>> {
        profile.validate()?;
        let url = profile
            .base_url()?
            .join("models")
            .map_err(|_| ModelError::Invalid("models URL"))?;
        let mut builder = self.client.get(url).header("accept", "application/json");
        if profile.protocol == ProtocolFamily::AnthropicMessages {
            builder = builder.header("anthropic-version", "2023-06-01");
        }
        let response = self
            .authorize(builder, profile, secrets)
            .await?
            .send()
            .await
            .map_err(|_| ModelError::Transport)?;
        let value = bounded_json(response, 2 * MAX_REQUEST_BYTES).await?;
        let data = value["data"].as_array().ok_or(ModelError::Protocol)?;
        if data.len() > 4096 {
            return Err(ModelError::Limit);
        }
        let mut result = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for item in data {
            let id = item["id"]
                .as_str()
                .filter(|v| bounded_identifier(v))
                .ok_or(ModelError::Protocol)?;
            if !ids.insert(id) {
                return Err(ModelError::Protocol);
            }
            let name = item["display_name"]
                .as_str()
                .filter(|v| bounded_identifier(v))
                .unwrap_or(id);
            // /models generally reports identities, not tools, context or image support.
            result.push(ModelInfo {
                id: id.into(),
                name: name.into(),
                capabilities: ModelCapabilities {
                    source: "provider /models (identity only, bounded first page)".into(),
                    ..Default::default()
                },
            });
        }
        Ok(result)
    }
    /// Explicit user-requested community catalog refresh. No account keys, cookies,
    /// workspace context or provider requests are sent to models.dev.
    pub async fn catalog(&self, cancellation: CancellationToken) -> ModelResult<ProviderCatalog> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(ModelError::Cancelled),
            result = async {
                let response = self.client.get("https://models.dev/api.json?type=all").send().await.map_err(|_| ModelError::Transport)?;
                parse_catalog(bounded_json(response, 20 * MAX_REQUEST_BYTES).await?)
            } => result,
        }
    }
}
#[async_trait]
impl ModelProvider for HttpModelProvider {
    async fn stream(
        &self,
        profile: &ProviderProfile,
        request: ModelRequest,
        secrets: &dyn SecretStore,
        cancellation: CancellationToken,
        events: mpsc::Sender<ModelEvent>,
    ) -> ModelResult<()> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(ModelError::Cancelled),
            result = self.run(profile, request, secrets, events) => result,
        }
    }
    async fn discover_models(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
        cancellation: CancellationToken,
    ) -> ModelResult<Vec<ModelInfo>> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(ModelError::Cancelled),
            result = self.models(profile, secrets) => result,
        }
    }
}
async fn bounded_json(response: reqwest::Response, limit: usize) -> ModelResult<Value> {
    if !response.status().is_success() {
        return Err(ModelError::Http(response.status().as_u16()));
    }
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err(ModelError::Limit);
    }
    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = tokio::time::timeout(Duration::from_secs(30), stream.next())
        .await
        .map_err(|_| ModelError::Transport)?
    {
        let chunk = chunk.map_err(|_| ModelError::Transport)?;
        if body.len().saturating_add(chunk.len()) > limit {
            return Err(ModelError::Limit);
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| ModelError::Protocol)
}
