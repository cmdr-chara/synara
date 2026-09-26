use crate::{
    protocol::encode,
    stream::{ProtocolDecoder, SseDecoder},
    *,
};
mod discovery;
use async_trait::async_trait;
use futures::StreamExt;
use reqwest::{
    Client, RequestBuilder,
    header::{HeaderMap, HeaderValue},
};
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
            ProtocolFamily::GoogleGenerateContent => (
                "x-goog-api-key",
                HeaderValue::from_str(key).map_err(|_| ModelError::Credential)?,
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
            ProtocolFamily::OpenAiChat => "chat/completions".into(),
            ProtocolFamily::AnthropicMessages => "messages".into(),
            ProtocolFamily::GoogleGenerateContent => format!(
                "models/{}:streamGenerateContent",
                crate::google::model_id(&request.model)?
            ),
        };
        let mut url = profile
            .base_url()?
            .join(&path)
            .map_err(|_| ModelError::Invalid("request URL"))?;
        if profile.protocol == ProtocolFamily::GoogleGenerateContent {
            url.query_pairs_mut().append_pair("alt", "sse");
        }
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
                    if let ModelEvent::Finished { reason } = &event
                        && let OutputFormat::JsonSchema { schema, .. } = &request.output
                    {
                        if !matches!(reason.as_str(), "stop" | "end_turn") {
                            return Err(ModelError::Incomplete);
                        }
                        crate::schema::validate_output(schema, &decoder.text)?;
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
    /// Explicit live provider/account probe. This performs one authenticated
    /// metadata request against the reviewed provider endpoint and reports only
    /// bounded rate/quota headers actually returned by that provider. It never
    /// infers billing, subscription tier, credits or account identity.
    pub async fn account_telemetry(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
        cancellation: CancellationToken,
    ) -> ModelResult<ProviderTelemetry> {
        profile.validate()?;
        let mut url = profile
            .base_url()?
            .join("models")
            .map_err(|_| ModelError::Invalid("models URL"))?;
        match profile.protocol {
            ProtocolFamily::GoogleGenerateContent => {
                url.query_pairs_mut().append_pair("pageSize", "1");
            }
            ProtocolFamily::AnthropicMessages => {
                url.query_pairs_mut().append_pair("limit", "1");
            }
            ProtocolFamily::OpenAiChat => {}
        }
        let mut builder = self.client.get(url).header("accept", "application/json");
        if profile.protocol == ProtocolFamily::AnthropicMessages {
            builder = builder.header("anthropic-version", "2023-06-01");
        }
        let response = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(ModelError::Cancelled),
            response = self.authorize(builder, profile, secrets) => {
                let builder = response?;
                tokio::time::timeout(Duration::from_secs(30), builder.send())
                    .await
                    .map_err(|_| ModelError::Transport)?
                    .map_err(|_| ModelError::Transport)?
            }
        };
        if !response.status().is_success() {
            return Err(ModelError::Http(response.status().as_u16()));
        }
        telemetry_from_headers(profile, response.headers())
    }

    async fn models(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
    ) -> ModelResult<Vec<ModelInfo>> {
        profile.validate()?;
        if profile.protocol == ProtocolFamily::GoogleGenerateContent {
            return self.google_models(profile, secrets).await;
        }
        self.standard_models(profile, secrets).await
    }
    async fn google_models(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
    ) -> ModelResult<Vec<ModelInfo>> {
        let mut token: Option<String> = None;
        let mut tokens = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        let mut result = Vec::new();
        // Pagination follows data tokens, never arbitrary links from a response.
        // All pages retain the same reviewed origin, credentials and cancellation.
        for _ in 0..8 {
            let mut url = profile
                .base_url()?
                .join("models")
                .map_err(|_| ModelError::Invalid("models URL"))?;
            url.query_pairs_mut().append_pair("pageSize", "1000");
            if let Some(token) = &token {
                url.query_pairs_mut().append_pair("pageToken", token);
            }
            let response = self
                .authorize(
                    self.client.get(url).header("accept", "application/json"),
                    profile,
                    secrets,
                )
                .await?
                .send()
                .await
                .map_err(|_| ModelError::Transport)?;
            let (models, next) =
                crate::google::models(bounded_json(response, 2 * MAX_REQUEST_BYTES).await?)?;
            for model in models {
                if !ids.insert(model.id.clone()) {
                    return Err(ModelError::Protocol);
                }
                result.push(model);
                if result.len() > 4096 {
                    return Err(ModelError::Limit);
                }
            }
            match next {
                None => return Ok(result),
                Some(next) if tokens.insert(next.clone()) => token = Some(next),
                _ => return Err(ModelError::Protocol),
            }
        }
        // Do not present a truncated collection as complete discovery.
        Err(ModelError::Limit)
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
            result = tokio::time::timeout(Duration::from_secs(120), self.models(profile, secrets)) => result.map_err(|_| ModelError::Transport)?,
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

fn telemetry_from_headers(
    profile: &ProviderProfile,
    headers: &HeaderMap,
) -> ModelResult<ProviderTelemetry> {
    fn text(headers: &HeaderMap, names: &[&str]) -> ModelResult<Option<String>> {
        for name in names {
            let Some(value) = headers.get(*name) else {
                continue;
            };
            let value = value.to_str().map_err(|_| ModelError::Protocol)?.trim();
            if value.is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
                return Err(ModelError::Protocol);
            }
            return Ok(Some(value.to_owned()));
        }
        Ok(None)
    }
    fn count(headers: &HeaderMap, names: &[&str]) -> ModelResult<Option<u64>> {
        match text(headers, names)? {
            None => Ok(None),
            Some(value) => value
                .parse::<u64>()
                .map(Some)
                .map_err(|_| ModelError::Protocol),
        }
    }

    let (
        request_limit,
        request_remaining,
        request_reset,
        token_limit,
        token_remaining,
        token_reset,
    ) = match profile.protocol {
        ProtocolFamily::AnthropicMessages => (
            &["anthropic-ratelimit-requests-limit", "ratelimit-limit"][..],
            &[
                "anthropic-ratelimit-requests-remaining",
                "ratelimit-remaining",
            ][..],
            &["anthropic-ratelimit-requests-reset", "ratelimit-reset"][..],
            &["anthropic-ratelimit-tokens-limit"][..],
            &["anthropic-ratelimit-tokens-remaining"][..],
            &["anthropic-ratelimit-tokens-reset"][..],
        ),
        ProtocolFamily::OpenAiChat => (
            &["x-ratelimit-limit-requests", "ratelimit-limit"][..],
            &["x-ratelimit-remaining-requests", "ratelimit-remaining"][..],
            &["x-ratelimit-reset-requests", "ratelimit-reset"][..],
            &["x-ratelimit-limit-tokens"][..],
            &["x-ratelimit-remaining-tokens"][..],
            &["x-ratelimit-reset-tokens"][..],
        ),
        ProtocolFamily::GoogleGenerateContent => (
            &["x-ratelimit-limit-requests", "ratelimit-limit"][..],
            &["x-ratelimit-remaining-requests", "ratelimit-remaining"][..],
            &["x-ratelimit-reset-requests", "ratelimit-reset"][..],
            &["x-ratelimit-limit-tokens"][..],
            &["x-ratelimit-remaining-tokens"][..],
            &["x-ratelimit-reset-tokens"][..],
        ),
    };

    Ok(ProviderTelemetry {
        provider_id: profile.id.clone(),
        credentialed: profile.requires_key,
        requests: RateLimitTelemetry {
            limit: count(headers, request_limit)?,
            remaining: count(headers, request_remaining)?,
            reset: text(headers, request_reset)?,
        },
        tokens: RateLimitTelemetry {
            limit: count(headers, token_limit)?,
            remaining: count(headers, token_remaining)?,
            reset: text(headers, token_reset)?,
        },
        retry_after: text(headers, &["retry-after"])?,
    })
}
