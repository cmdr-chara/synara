use super::*;
use crate::stream::ProtocolDecoder;
use crate::tests::{http_sse, profile, request, server};
use synara_runtime::{
    RuntimeError, SecretReference, SecretStore, SecretStoreState, SecretValue,
    UnavailableSecretStore,
};

fn google() -> ProviderProfile {
    let mut p = profile();
    p.protocol = ProtocolFamily::GoogleGenerateContent;
    p
}
fn chunk(text: &str, finish: bool) -> String {
    let mut value =
        json!({"candidates":[{"index":0,"content":{"role":"model","parts":[{"text":text}]}}]});
    if finish {
        value["candidates"][0]["finishReason"] = json!("STOP");
    }
    value.to_string()
}
#[test]
fn google_model_paths_cannot_escape_the_reviewed_endpoint() {
    for bad in [
        "../secret",
        "models/../secret",
        "models/a/b",
        "a?key=leak",
        "a%2fb",
        "a:otherMethod",
        "https://evil.invalid",
        "",
        "models/",
    ] {
        assert!(model_id(bad).is_err(), "{bad}");
        let mut p = google();
        p.models[0].id = bad.into();
        assert!(p.validate().is_err());
    }
    assert_eq!(model_id("models/test-model.001").unwrap(), "test-model.001");
}
#[test]
fn google_encodes_messages_inline_images_and_explicit_schema_without_credentials() {
    let mut p = google();
    p.models[0].capabilities.images = Support::Supported;
    p.models[0].capabilities.structured_output = Support::Supported;
    let mut r = request();
    r.messages.insert(
        0,
        Message::text(MessageRole::System, "Reviewed instructions".into()),
    );
    r.messages[1].content.push(Content::Image {
        media_type: "image/png".into(),
        base64: "AAAA".into(),
    });
    r.messages.push(Message::text(
        MessageRole::Assistant,
        "Previous reply".into(),
    ));
    r.output = OutputFormat::JsonSchema {
        name: "reply".into(),
        schema: json!({"type":"object"}),
    };
    let v = crate::protocol::encode(&p, &r).unwrap();
    assert_eq!(
        v["systemInstruction"]["parts"][0]["text"],
        "Reviewed instructions"
    );
    assert_eq!(v["contents"][0]["parts"][1]["inlineData"]["data"], "AAAA");
    assert_eq!(v["contents"][1]["role"], "model");
    assert_eq!(
        v["generationConfig"]["responseJsonSchema"],
        json!({"type":"object"})
    );
    assert!(v.get("key").is_none());
    assert!(v.get("tools").is_none());
    p.models[0].capabilities.tools = Support::Supported;
    r.tools.push(ToolDefinition {
        name: "read".into(),
        description: "Read".into(),
        parameters: json!({"type":"object"}),
    });
    assert!(matches!(
        crate::protocol::encode(&p, &r),
        Err(ModelError::Unsupported(_))
    ));
}
#[test]
fn google_stream_usage_thoughts_and_terminal_state_are_normalized() {
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::GoogleGenerateContent, &request());
    assert_eq!(
        decoder.push(&chunk("Hello", false)).unwrap(),
        vec![ModelEvent::Text("Hello".into())]
    );
    assert!(!decoder.finished);
    let v = json!({"candidates":[{"content":{"parts":[{"text":"summary","thought":true}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":100,"cachedContentTokenCount":20,"candidatesTokenCount":2,"thoughtsTokenCount":3}});
    let events = decoder.push(&v.to_string()).unwrap();
    assert_eq!(events[0], ModelEvent::Reasoning("summary".into()));
    assert_eq!(
        events[1],
        ModelEvent::Usage(ModelUsage {
            input_tokens: Some(100),
            cached_input_tokens: Some(20),
            output_tokens: Some(5),
            reasoning_tokens: Some(3)
        })
    );
    assert!(matches!(events.last(),Some(ModelEvent::Finished{reason}) if reason=="stop"));
    assert_eq!(decoder.text, "Hello");
    assert!(decoder.push(&chunk("extra", true)).is_err());
}
#[test]
fn google_rejects_blocked_truncated_unknown_and_overflow_output() {
    for v in [
        json!({"promptFeedback":{"blockReason":"SAFETY"}}),
        json!({"candidates":[{"finishReason":"MAX_TOKENS"}]}),
        json!({"candidates":[{"index":1}]}),
        json!({"candidates":[{},{}]}),
        json!({"candidates":[{"content":{"parts":[{"functionCall":{"name":"steal"}}]}}]}),
        json!({"usageMetadata":{"promptTokenCount":2,"cachedContentTokenCount":3}}),
        json!({"usageMetadata":{"candidatesTokenCount":u64::MAX,"thoughtsTokenCount":1}}),
        json!({"usageMetadata":{"promptTokenCount":-1}}),
    ] {
        assert!(
            ProtocolDecoder::new(ProtocolFamily::GoogleGenerateContent, &request())
                .push(&v.to_string())
                .is_err(),
            "{v}"
        );
    }
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::GoogleGenerateContent, &request());
    decoder.push(&chunk("partial", false)).unwrap();
    assert!(!decoder.finished);
}
#[test]
fn google_catalog_maps_explicit_sdk_not_provider_names_and_discovery_reports_only_metadata() {
    let source = json!({"arbitrary-name":{"name":"Unrelated","npm":"@ai-sdk/google","api":"https://example.invalid/v1beta","env":["KEY"],"models":{"test-model":{"name":"Test","tool_call":true,"structured_output":true}}},"google-name":{"name":"Google","npm":"@ai-sdk/google-vertex","models":{}}});
    let c = parse_catalog(source).unwrap();
    let p = c
        .providers
        .iter()
        .find(|p| p.id == "arbitrary-name")
        .unwrap()
        .profile
        .as_ref()
        .unwrap();
    assert_eq!(p.protocol, ProtocolFamily::GoogleGenerateContent);
    assert!(
        c.providers
            .iter()
            .find(|p| p.id == "google-name")
            .unwrap()
            .profile
            .is_none()
    );
    let (m,next)=models(json!({"models":[{"name":"models/test-model","displayName":"Test","supportedGenerationMethods":["generateContent"],"inputTokenLimit":8192,"outputTokenLimit":2048},{"name":"models/embedding","supportedGenerationMethods":["embedContent"]}],"nextPageToken":"a&b"})).unwrap();
    assert_eq!(m.len(), 1);
    assert_eq!(next.as_deref(), Some("a&b"));
    assert_eq!(m[0].capabilities.context_window, Some(8192));
    assert_eq!(m[0].capabilities.images, Support::Unknown);
    assert_eq!(m[0].capabilities.tools, Support::Unknown);
}
struct Key;
#[async_trait::async_trait]
impl SecretStore for Key {
    fn state(&self) -> SecretStoreState {
        SecretStoreState::Available
    }
    async fn read(&self, _: &SecretReference) -> Result<Option<SecretValue>, RuntimeError> {
        Ok(Some(SecretValue::new(b"owned-test-secret".to_vec())?))
    }
    async fn write(&self, _: &SecretReference, _: SecretValue) -> Result<(), RuntimeError> {
        unreachable!()
    }
    async fn delete(&self, _: &SecretReference) -> Result<(), RuntimeError> {
        unreachable!()
    }
}
#[tokio::test]
async fn google_live_http_uses_sensitive_header_not_url_and_validates_structured_completion() {
    let data = format!("data: {}\n\n", chunk("{\"ok\":true}", true));
    let (endpoint, server) = server(http_sse(&data)).await;
    let mut p = google();
    p.endpoint = endpoint;
    p.requires_key = true;
    p.models[0].capabilities.structured_output = Support::Supported;
    let mut r = request();
    r.output = OutputFormat::JsonSchema {
        name: "result".into(),
        schema: json!({"type":"object","properties":{"ok":{"const":true}},"required":["ok"],"additionalProperties":false}),
    };
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    HttpModelProvider::new()
        .unwrap()
        .stream(&p, r, &Key, Default::default(), tx)
        .await
        .unwrap();
    let wire = server.await.unwrap();
    let header = wire.split("\r\n\r\n").next().unwrap();
    assert!(
        header.starts_with("POST /v1/models/test-model:streamGenerateContent?alt=sse HTTP/1.1")
    );
    assert!(
        header
            .to_lowercase()
            .contains("x-goog-api-key: owned-test-secret")
    );
    assert!(!wire.lines().next().unwrap().contains("secret"));
    assert!(matches!(rx.recv().await, Some(ModelEvent::Text(_))));
    assert!(matches!(rx.recv().await, Some(ModelEvent::Finished { .. })));
}
#[tokio::test]
async fn structured_output_wrong_schema_retains_partial_text_without_finished_for_both_families() {
    for family in [
        ProtocolFamily::OpenAiChat,
        ProtocolFamily::GoogleGenerateContent,
    ] {
        let data = if family == ProtocolFamily::GoogleGenerateContent {
            format!("data: {}\n\n", chunk("{\"ok\":false}", true))
        } else {
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"{\\\"ok\\\":false}\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n".into()
        };
        let (endpoint, server) = server(http_sse(&data)).await;
        let mut p = google();
        p.protocol = family;
        p.endpoint = endpoint;
        p.models[0].capabilities.structured_output = Support::Supported;
        let mut r = request();
        r.output = OutputFormat::JsonSchema {
            name: "result".into(),
            schema: json!({"type":"object","properties":{"ok":{"const":true}},"required":["ok"]}),
        };
        let (tx, mut rx) = tokio::sync::mpsc::channel(8);
        assert!(matches!(
            HttpModelProvider::new()
                .unwrap()
                .stream(
                    &p,
                    r,
                    &UnavailableSecretStore::unavailable(),
                    Default::default(),
                    tx
                )
                .await,
            Err(ModelError::SchemaMismatch)
        ));
        assert!(matches!(rx.recv().await, Some(ModelEvent::Text(_))));
        assert_eq!(rx.recv().await, None);
        server.await.unwrap();
    }
}
#[tokio::test]
async fn google_discovery_never_follows_a_remote_url_and_rejects_repeated_page_tokens() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mut p = google();
    p.endpoint = format!("http://{}/v1beta", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        for page in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut bytes = Vec::new();
            let mut byte = [0];
            while !bytes.ends_with(b"\r\n\r\n") {
                socket.read_exact(&mut byte).await.unwrap();
                bytes.push(byte[0]);
                assert!(bytes.len() < 16384);
            }
            let wire = String::from_utf8(bytes).unwrap();
            assert!(wire.starts_with("GET /v1beta/models?pageSize=1000"));
            if page == 1 {
                assert!(wire.contains("pageToken=https%3A%2F%2Fevil.invalid%2F"));
            }
            let body = json!({"models":[],"nextPageToken":"https://evil.invalid/"}).to_string();
            socket.write_all(format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).as_bytes()).await.unwrap();
        }
    });
    assert!(matches!(
        HttpModelProvider::new()
            .unwrap()
            .discover_models(
                &p,
                &UnavailableSecretStore::unavailable(),
                Default::default()
            )
            .await,
        Err(ModelError::Protocol)
    ));
    server.await.unwrap();
}
