//! Bounded pagination follows a data cursor, never a URL supplied by a provider.
//! Anthropic contract: https://platform.claude.com/docs/en/api/models/list
use super::*;
use std::collections::HashSet;

impl HttpModelProvider {
    pub(super) async fn standard_models(
        &self,
        profile: &ProviderProfile,
        secrets: &dyn SecretStore,
    ) -> ModelResult<Vec<ModelInfo>> {
        let mut after: Option<String> = None;
        let mut cursors = HashSet::new();
        let mut ids = HashSet::new();
        let mut result = Vec::new();
        for _ in 0..8 {
            let mut url = profile
                .base_url()?
                .join("models")
                .map_err(|_| ModelError::Invalid("models URL"))?;
            if profile.protocol == ProtocolFamily::AnthropicMessages {
                url.query_pairs_mut().append_pair("limit", "1000");
                if let Some(cursor) = &after {
                    url.query_pairs_mut().append_pair("after_id", cursor);
                }
            }
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
            let (models, next) = page(
                profile.protocol,
                bounded_json(response, 2 * MAX_REQUEST_BYTES).await?,
            )?;
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
                Some(next) if cursors.insert(next.clone()) => after = Some(next),
                _ => return Err(ModelError::Protocol),
            }
        }
        Err(ModelError::Limit)
    }
}
fn positive(value: &Value) -> ModelResult<Option<u64>> {
    match value {
        Value::Null => Ok(None),
        Value::Number(number) => number
            .as_u64()
            .map(|n| (n > 0).then_some(n))
            .ok_or(ModelError::Protocol),
        _ => Err(ModelError::Protocol),
    }
}
fn image_support(value: Option<&Value>) -> ModelResult<Support> {
    match value {
        None | Some(Value::Null) => Ok(Support::Unknown),
        Some(Value::Bool(true)) => Ok(Support::Supported),
        Some(Value::Bool(false)) => Ok(Support::Unsupported),
        _ => Err(ModelError::Protocol),
    }
}
fn page(protocol: ProtocolFamily, value: Value) -> ModelResult<(Vec<ModelInfo>, Option<String>)> {
    let data = value["data"].as_array().ok_or(ModelError::Protocol)?;
    if data.len() > 4096 {
        return Err(ModelError::Limit);
    }
    let more = match value.get("has_more") {
        None => false,
        Some(Value::Bool(value)) => *value,
        _ => return Err(ModelError::Protocol),
    };
    if more && protocol != ProtocolFamily::AnthropicMessages {
        return Err(ModelError::Unsupported(
            "this compatible endpoint reports additional model pages but has no reviewed pagination contract",
        ));
    }
    let mut models = Vec::new();
    let mut ids = HashSet::new();
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
        let capabilities = if protocol == ProtocolFamily::AnthropicMessages {
            ModelCapabilities {
                images: image_support(item.pointer("/capabilities/image_input/supported"))?,
                context_window: positive(&item["max_input_tokens"])?,
                max_output_tokens: positive(&item["max_tokens"])?,
                source: "provider /models (reported image/token metadata, bounded pagination)"
                    .into(),
                ..Default::default()
            }
        } else {
            ModelCapabilities {
                source: "provider /models (identity only, complete reported collection)".into(),
                ..Default::default()
            }
        };
        models.push(ModelInfo {
            id: id.into(),
            name: name.into(),
            capabilities,
        });
    }
    let next = if more {
        let cursor = value["last_id"]
            .as_str()
            .filter(|v| bounded_identifier(v))
            .ok_or(ModelError::Protocol)?;
        if models.last().is_none_or(|model| model.id != cursor) {
            return Err(ModelError::Protocol);
        }
        Some(cursor.to_owned())
    } else {
        None
    };
    Ok((models, next))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use synara_runtime::UnavailableSecretStore;
    #[test]
    fn anthropic_page_reads_only_explicit_supported_metadata() {
        let (models, cursor) = page(
            ProtocolFamily::AnthropicMessages,
            json!({"data":[{
            "id":"model-a", "display_name":"A", "max_input_tokens":200000, "max_tokens":8192,
            "capabilities":{"image_input":{"supported":true},"thinking":{"supported":true}}
        }], "has_more":true, "last_id":"model-a"}),
        )
        .unwrap();
        assert_eq!(cursor.as_deref(), Some("model-a"));
        let caps = &models[0].capabilities;
        assert_eq!(caps.images, Support::Supported);
        assert_eq!(caps.context_window, Some(200000));
        assert_eq!(caps.max_output_tokens, Some(8192));
        assert_eq!(caps.tools, Support::Unknown);
        assert_eq!(caps.structured_output, Support::Unknown);
        assert!(caps.reasoning_efforts.is_empty());
    }
    #[test]
    fn identities_do_not_imply_capabilities() {
        for protocol in [
            ProtocolFamily::OpenAiChat,
            ProtocolFamily::AnthropicMessages,
        ] {
            let (models, next) =
                page(protocol, json!({"data":[{"id":"image-thinking-model"}]})).unwrap();
            assert!(next.is_none());
            assert_eq!(models[0].capabilities.images, Support::Unknown);
            assert_eq!(models[0].capabilities.context_window, None);
        }
    }
    #[test]
    fn malformed_or_unreviewed_pagination_never_returns_partial_success() {
        for value in [
            json!({"data":[],"has_more":true,"last_id":"a"}),
            json!({"data":[{"id":"a"}],"has_more":true,"last_id":"wrong"}),
            json!({"data":[{"id":"a"}],"has_more":"yes"}),
            json!({"data":[{"id":"a"},{"id":"a"}],"has_more":false}),
            json!({"data":[{"id":"a","max_tokens":-1}],"has_more":false}),
            json!({"data":[{"id":"a","capabilities":{"image_input":{"supported":"yes"}}}]}),
        ] {
            assert!(page(ProtocolFamily::AnthropicMessages, value).is_err());
        }
        assert!(matches!(
            page(
                ProtocolFamily::OpenAiChat,
                json!({"data":[{"id":"a"}],"has_more":true,"last_id":"a"})
            ),
            Err(ModelError::Unsupported(_))
        ));
    }
    async fn fixture(pages: Vec<Value>) -> (ProviderProfile, tokio::task::JoinHandle<Vec<String>>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut paths = Vec::new();
            for page in pages {
                let (mut stream, _) =
                    tokio::time::timeout(Duration::from_secs(5), listener.accept())
                        .await
                        .unwrap()
                        .unwrap();
                let mut headers = Vec::new();
                let mut byte = [0];
                while !headers.ends_with(b"\r\n\r\n") {
                    stream.read_exact(&mut byte).await.unwrap();
                    headers.push(byte[0]);
                    assert!(headers.len() < 16384);
                }
                let headers = String::from_utf8(headers).unwrap();
                assert!(!headers.to_lowercase().contains("authorization:"));
                assert!(!headers.to_lowercase().contains("x-api-key:"));
                assert!(
                    headers
                        .to_lowercase()
                        .contains("anthropic-version: 2023-06-01")
                );
                paths.push(headers.lines().next().unwrap().to_owned());
                let body = page.to_string();
                stream.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            }
            paths
        });
        let mut profile = custom_profile_example();
        profile.protocol = ProtocolFamily::AnthropicMessages;
        profile.endpoint = format!("http://{address}/v1");
        (profile, handle)
    }
    #[tokio::test]
    async fn discovery_reads_second_page_at_same_reviewed_endpoint() {
        let (profile, server) = fixture(vec![
            json!({"data":[{"id":"first"}],"has_more":true,"last_id":"first"}),
            json!({"data":[{"id":"second"}],"has_more":false,"last_id":"second"}),
        ])
        .await;
        let models = HttpModelProvider::new()
            .unwrap()
            .discover_models(
                &profile,
                &UnavailableSecretStore::unavailable(),
                CancellationToken::new(),
            )
            .await
            .unwrap();
        assert_eq!(
            models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            vec!["first", "second"]
        );
        let paths = server.await.unwrap();
        assert_eq!(paths[0], "GET /v1/models?limit=1000 HTTP/1.1");
        assert_eq!(
            paths[1],
            "GET /v1/models?limit=1000&after_id=first HTTP/1.1"
        );
    }
    #[tokio::test]
    async fn repeated_identity_across_pages_is_an_error() {
        let (profile, server) = fixture(vec![
            json!({"data":[{"id":"first"}],"has_more":true,"last_id":"first"}),
            json!({"data":[{"id":"first"}],"has_more":false}),
        ])
        .await;
        assert!(matches!(
            HttpModelProvider::new()
                .unwrap()
                .discover_models(
                    &profile,
                    &UnavailableSecretStore::unavailable(),
                    CancellationToken::new()
                )
                .await,
            Err(ModelError::Protocol)
        ));
        server.await.unwrap();
    }
    #[tokio::test]
    async fn page_ceiling_rejects_truncated_collection() {
        let pages = (0..8).map(|i| json!({"data":[{"id":format!("m{i}")}],"has_more":true,"last_id":format!("m{i}")})).collect();
        let (profile, server) = fixture(pages).await;
        assert!(matches!(
            HttpModelProvider::new()
                .unwrap()
                .discover_models(
                    &profile,
                    &UnavailableSecretStore::unavailable(),
                    CancellationToken::new()
                )
                .await,
            Err(ModelError::Limit)
        ));
        assert_eq!(server.await.unwrap().len(), 8);
    }
    #[tokio::test]
    async fn cancelled_discovery_never_connects() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let mut profile = custom_profile_example();
        profile.protocol = ProtocolFamily::AnthropicMessages;
        profile.endpoint = format!("http://{}/v1", listener.local_addr().unwrap());
        let cancel = CancellationToken::new();
        cancel.cancel();
        assert!(matches!(
            HttpModelProvider::new()
                .unwrap()
                .discover_models(&profile, &UnavailableSecretStore::unavailable(), cancel)
                .await,
            Err(ModelError::Cancelled)
        ));
        assert!(
            tokio::time::timeout(Duration::from_millis(20), listener.accept())
                .await
                .is_err()
        );
    }
}
