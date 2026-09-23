use super::*;
use crate::{
    protocol::encode,
    stream::{ProtocolDecoder, SseDecoder},
};
use serde_json::json;
use synara_runtime::UnavailableSecretStore;

pub(super) fn profile() -> ProviderProfile {
    let mut p = custom_profile_example();
    p.models[0].id = "test-model".into();
    p
}
pub(super) fn request() -> ModelRequest {
    ModelRequest {
        model: "test-model".into(),
        messages: vec![Message::text(MessageRole::User, "Hello".into())],
        tools: vec![],
        output: OutputFormat::Text,
        reasoning_effort: None,
        max_output_tokens: 32,
    }
}
#[test]
fn direct_endpoint_security_and_canonical_secret_binding() {
    let p = profile();
    p.validate().unwrap();
    let mut equivalent = p.clone();
    equivalent.endpoint.push('/');
    assert_eq!(
        p.secret_reference().unwrap(),
        equivalent.secret_reference().unwrap()
    );
    let mut changed = p.clone();
    changed.endpoint = "http://127.0.0.1:9999/v1".into();
    assert_ne!(
        p.secret_reference().unwrap(),
        changed.secret_reference().unwrap()
    );
    changed = p.clone();
    changed.protocol = ProtocolFamily::AnthropicMessages;
    assert_ne!(
        p.secret_reference().unwrap(),
        changed.secret_reference().unwrap()
    );
    for url in [
        "http://example.com/v1",
        "http://localhost/v1",
        "http://127.0.0.1.evil.invalid/v1",
        "https://user:secret@example.com/v1",
        "https://example.com/v1?key=secret",
        "https://example.com/v1#secret",
        "file:///tmp/secret",
        "https://example.com/%2fv1",
    ] {
        changed.endpoint = url.into();
        assert!(changed.validate().is_err(), "{url}");
    }
    changed = p.clone();
    changed.allow_loopback_http = false;
    assert!(changed.validate().is_err());
}
#[test]
fn direct_registry_scales_without_provider_specific_code_or_duplicate_ids() {
    let mut settings = ProviderSettings::default();
    for i in 0..100 {
        let mut p = profile();
        p.id = format!("provider-{i}");
        settings.providers.push(p);
    }
    settings.validate().unwrap();
    settings.providers.push(settings.providers[0].clone());
    assert!(settings.validate().is_err());
    let mut value = serde_json::to_value(profile()).unwrap();
    value["api_key"] = json!("never-persist");
    assert!(serde_json::from_value::<ProviderProfile>(value).is_err());
}
#[test]
fn direct_unknown_capabilities_are_not_inferred_from_a_provider_name() {
    let mut p = profile();
    p.name = "OpenAI Anthropic supports everything".into();
    let mut r = request();
    r.reasoning_effort = Some("high".into());
    assert!(matches!(
        validate_request(&p, &r),
        Err(ModelError::Unsupported(_))
    ));
    r = request();
    r.tools.push(ToolDefinition {
        name: "read".into(),
        description: "Read".into(),
        parameters: json!({"type":"object"}),
    });
    assert!(matches!(
        validate_request(&p, &r),
        Err(ModelError::Unsupported(_))
    ));
    r = request();
    r.output = OutputFormat::JsonSchema {
        name: "result".into(),
        schema: json!({"type":"object"}),
    };
    assert!(matches!(
        validate_request(&p, &r),
        Err(ModelError::Unsupported(_))
    ));
    r = request();
    r.messages[0].content.push(Content::Image {
        media_type: "image/png".into(),
        base64: "AAAA".into(),
    });
    assert!(matches!(
        validate_request(&p, &r),
        Err(ModelError::Unsupported(_))
    ));
}
#[test]
fn direct_catalog_uses_explicit_protocol_and_marks_cloud_auth_unsupported() {
    let source = json!({
        "custom":{"name":"Unrelated name","npm":"@ai-sdk/openai-compatible","api":"https://models.example/v1","env":["TOKEN"],"models":{"m":{"name":"Model","tool_call":true,"reasoning":true,"limit":{"context":8192,"output":1024},"modalities":{"input":["text","image"]}}}},
        "unsupported":{"name":"OpenAI","npm":"unknown-sdk","api":"https://example.com","models":{}},
        "cloud":{"name":"Cloud","npm":"@ai-sdk/anthropic","api":"https://example.com","env":["KEY","PROJECT"],"models":{}}
    });
    let catalog = parse_catalog(source).unwrap();
    assert_eq!(catalog.providers.len(), 3);
    let p = catalog
        .providers
        .iter()
        .find(|p| p.id == "custom")
        .unwrap()
        .profile
        .as_ref()
        .unwrap();
    assert_eq!(p.models[0].capabilities.tools, Support::Supported);
    assert_eq!(p.models[0].capabilities.images, Support::Supported);
    assert_eq!(p.models[0].capabilities.structured_output, Support::Unknown);
    assert!(p.models[0].capabilities.reasoning_efforts.is_empty());
    assert_eq!(
        catalog
            .providers
            .iter()
            .filter(|p| p.unsupported_reason.is_some())
            .count(),
        2
    );
}
#[test]
fn direct_sse_handles_split_unicode_crlf_comments_and_multiple_data_lines() {
    let source = ": ping\r\ndata: {\r\ndata: \"text\":\"日本語\"}\r\n\r\n";
    let mut decoder = SseDecoder::default();
    let mut frames = vec![];
    for b in source.as_bytes() {
        frames.extend(decoder.push(&[*b]).unwrap());
    }
    assert_eq!(frames, vec!["{\n\"text\":\"日本語\"}"]);
    assert!(
        SseDecoder::default()
            .push(&vec![b'x'; MAX_FRAME_BYTES + 1])
            .is_err()
    );
}
#[test]
fn direct_openai_requires_a_terminal_marker_and_normalizes_usage() {
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::OpenAiChat, &request());
    assert_eq!(
        decoder
            .push(r#"{"choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#)
            .unwrap(),
        vec![ModelEvent::Text("Hello".into())]
    );
    decoder
        .push(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#)
        .unwrap();
    assert!(!decoder.finished);
    let events=decoder.push(r#"{"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":2,"prompt_tokens_details":{"cached_tokens":5}}}"#).unwrap();
    assert!(matches!(
        &events[0],
        ModelEvent::Usage(ModelUsage {
            input_tokens: Some(10),
            output_tokens: Some(2),
            cached_input_tokens: Some(5),
            ..
        })
    ));
    assert_eq!(
        decoder.push("[DONE]").unwrap(),
        vec![ModelEvent::Finished {
            reason: "stop".into()
        }]
    );
    assert!(decoder.push("[DONE]").is_err());
    assert!(
        ProtocolDecoder::new(ProtocolFamily::OpenAiChat, &request())
            .push("[DONE]")
            .is_err()
    );
}
#[test]
fn direct_tool_fragments_are_proposals_and_must_match_offered_tools() {
    let mut r = request();
    r.tools.push(ToolDefinition {
        name: "read_file".into(),
        description: "Read".into(),
        parameters: json!({"type":"object"}),
    });
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::OpenAiChat, &r);
    for delta in [
        json!({"tool_calls":[{"index":0,"id":"call-1","function":{"name":"read_file","arguments":"{\"path\":"}}]}),
        json!({"tool_calls":[{"index":0,"function":{"arguments":"\"a.rs\"}"}}]}),
    ] {
        decoder
            .push(&json!({"choices":[{"index":0,"delta":delta,"finish_reason":null}]}).to_string())
            .unwrap();
    }
    decoder
        .push(r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#)
        .unwrap();
    let events = decoder.push("[DONE]").unwrap();
    assert_eq!(
        events[0],
        ModelEvent::ToolCall(ToolCall {
            id: "call-1".into(),
            name: "read_file".into(),
            arguments: json!({"path":"a.rs"})
        })
    );
    let mut unsolicited = ProtocolDecoder::new(ProtocolFamily::OpenAiChat, &request());
    unsolicited.push(r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"execute","arguments":"{}"}}]},"finish_reason":"tool_calls"}]}"#).unwrap();
    assert!(unsolicited.push("[DONE]").is_err());
}
#[test]
fn direct_anthropic_text_and_usage_have_honest_completion() {
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::AnthropicMessages, &request());
    decoder
        .push(
            r#"{"type":"message_start","message":{"usage":{"input_tokens":7,"output_tokens":1}}}"#,
        )
        .unwrap();
    assert_eq!(decoder.push(r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#).unwrap(),vec![ModelEvent::Text("Hi".into())]);
    let events=decoder.push(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}"#).unwrap();
    assert!(matches!(
        &events[0],
        ModelEvent::Usage(ModelUsage {
            input_tokens: Some(7),
            output_tokens: Some(3),
            ..
        })
    ));
    assert!(!decoder.finished);
    assert_eq!(
        decoder.push(r#"{"type":"message_stop"}"#).unwrap(),
        vec![ModelEvent::Finished {
            reason: "end_turn".into()
        }]
    );
}
#[test]
fn direct_anthropic_tool_json_is_assembled_without_executing_it() {
    let mut r = request();
    r.tools.push(ToolDefinition {
        name: "lookup".into(),
        description: "".into(),
        parameters: json!({}),
    });
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::AnthropicMessages, &r);
    for frame in [
        json!({"type":"message_start","message":{"usage":{}}}),
        json!({"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"a","name":"lookup","input":{}}}),
        json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"id\":1}"}}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{}}),
    ] {
        decoder.push(&frame.to_string()).unwrap();
    }
    let events = decoder.push(r#"{"type":"message_stop"}"#).unwrap();
    assert_eq!(
        events[0],
        ModelEvent::ToolCall(ToolCall {
            id: "a".into(),
            name: "lookup".into(),
            arguments: json!({"id":1})
        })
    );
}
#[test]
fn direct_transport_encodes_only_explicit_supported_options() {
    let mut p = profile();
    p.models[0].capabilities = ModelCapabilities {
        tools: Support::Supported,
        images: Support::Supported,
        structured_output: Support::Supported,
        reasoning_efforts: vec!["high".into()],
        ..Default::default()
    };
    let mut r = request();
    r.reasoning_effort = Some("high".into());
    r.output = OutputFormat::JsonSchema {
        name: "result".into(),
        schema: json!({"type":"object"}),
    };
    r.messages[0].content.push(Content::Image {
        media_type: "image/png".into(),
        base64: "AAAA".into(),
    });
    r.tools.push(ToolDefinition {
        name: "lookup".into(),
        description: "Find".into(),
        parameters: json!({"type":"object"}),
    });
    let encoded = encode(&p, &r).unwrap();
    assert_eq!(encoded["reasoning_effort"], "high");
    assert_eq!(encoded["max_completion_tokens"], 32);
    assert_eq!(encoded["response_format"]["json_schema"]["strict"], true);
    assert_eq!(
        encoded["messages"][0]["content"][1]["image_url"]["url"],
        "data:image/png;base64,AAAA"
    );
    p.protocol = ProtocolFamily::AnthropicMessages;
    assert!(encode(&p, &r).is_err());
    r.reasoning_effort = None;
    r.output = OutputFormat::Text;
    let encoded = encode(&p, &r).unwrap();
    assert_eq!(
        encoded["messages"][0]["content"][1]["source"]["data"],
        "AAAA"
    );
    assert!(encoded.get("api_key").is_none());
}
#[test]
fn direct_request_rejects_unmatched_tool_results_and_oversized_prompts() {
    let mut p = profile();
    p.models[0].capabilities.tools = Support::Supported;
    let mut r = request();
    r.messages.push(Message {
        role: MessageRole::Tool,
        content: vec![Content::Text {
            text: "result".into(),
        }],
        tool_calls: vec![],
        tool_call_id: Some("unknown".into()),
    });
    assert!(validate_request(&p, &r).is_err());
    r = request();
    r.messages[0] = Message::text(MessageRole::User, "x".repeat(MAX_INPUT_BYTES));
    assert!(validate_request(&p, &r).is_err());
    r = request();
    r.max_output_tokens = 0;
    assert!(validate_request(&p, &r).is_err());
}

pub(super) async fn server(response: String) -> (String, tokio::task::JoinHandle<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = Vec::new();
        let mut byte = [0u8; 1];
        while !request.ends_with(b"\r\n\r\n") {
            socket.read_exact(&mut byte).await.unwrap();
            request.push(byte[0]);
            assert!(request.len() < 16384);
        }
        let header = String::from_utf8(request.clone()).unwrap();
        let length = header
            .lines()
            .find_map(|l| {
                l.to_lowercase()
                    .strip_prefix("content-length:")
                    .and_then(|v| v.trim().parse::<usize>().ok())
            })
            .unwrap_or(0);
        assert!(length < MAX_REQUEST_BYTES);
        let mut body = vec![0; length];
        socket.read_exact(&mut body).await.unwrap();
        request.extend(body);
        socket.write_all(response.as_bytes()).await.unwrap();
        String::from_utf8(request).unwrap()
    });
    (format!("http://{address}/v1"), task)
}
pub(super) fn http_sse(data: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{data}",
        data.len()
    )
}
#[tokio::test]
async fn direct_live_loopback_request_streams_without_acp_or_credentials() {
    let data = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Hello\"},\"finish_reason\":null}]}\n\ndata: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n";
    let (endpoint, server) = server(http_sse(data)).await;
    let mut p = profile();
    p.endpoint = endpoint;
    let provider = HttpModelProvider::new().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);
    provider
        .stream(
            &p,
            request(),
            &UnavailableSecretStore::unavailable(),
            Default::default(),
            tx,
        )
        .await
        .unwrap();
    assert_eq!(rx.recv().await, Some(ModelEvent::Text("Hello".into())));
    assert_eq!(
        rx.recv().await,
        Some(ModelEvent::Finished {
            reason: "stop".into()
        })
    );
    let wire = server.await.unwrap();
    assert!(wire.starts_with("POST /v1/chat/completions"));
    assert!(!wire.to_lowercase().contains("authorization:"));
}
#[tokio::test]
async fn direct_truncated_stream_is_not_success() {
    let data = "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"partial\"},\"finish_reason\":null}]}\n\n";
    let (endpoint, server) = server(http_sse(data)).await;
    let mut p = profile();
    p.endpoint = endpoint;
    let (tx, _rx) = tokio::sync::mpsc::channel(8);
    let result = HttpModelProvider::new()
        .unwrap()
        .stream(
            &p,
            request(),
            &UnavailableSecretStore::unavailable(),
            Default::default(),
            tx,
        )
        .await;
    assert!(matches!(result, Err(ModelError::Protocol)));
    server.await.unwrap();
}
#[tokio::test]
async fn direct_redirect_is_not_followed_and_error_body_is_not_exposed() {
    let (endpoint,server)=server("HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1:1/steal\r\nContent-Length: 16\r\nConnection: close\r\n\r\nsecret-test-body".into()).await;
    let mut p = profile();
    p.endpoint = endpoint;
    let (tx, _) = tokio::sync::mpsc::channel(8);
    let error = HttpModelProvider::new()
        .unwrap()
        .stream(
            &p,
            request(),
            &UnavailableSecretStore::unavailable(),
            Default::default(),
            tx,
        )
        .await
        .unwrap_err();
    assert!(matches!(error, ModelError::Http(302)));
    assert!(!error.to_string().contains("secret-test-body"));
    server.await.unwrap();
}
#[tokio::test]
async fn direct_pre_cancel_and_missing_key_do_not_make_a_request() {
    let mut p = profile();
    p.endpoint = "http://127.0.0.1:1/v1".into();
    let token = tokio_util::sync::CancellationToken::new();
    token.cancel();
    let provider = HttpModelProvider::new().unwrap();
    let (tx, _) = tokio::sync::mpsc::channel(8);
    assert!(matches!(
        provider
            .stream(
                &p,
                request(),
                &UnavailableSecretStore::unavailable(),
                token,
                tx
            )
            .await,
        Err(ModelError::Cancelled)
    ));
    p.requires_key = true;
    let (tx, _) = tokio::sync::mpsc::channel(8);
    assert!(matches!(
        provider
            .stream(
                &p,
                request(),
                &UnavailableSecretStore::unavailable(),
                Default::default(),
                tx
            )
            .await,
        Err(ModelError::Credential)
    ));
}

#[test]
fn direct_anthropic_usage_counts_cached_context_and_rejects_overflow() {
    let usage = json!({"input_tokens":5,"cache_read_input_tokens":1000,"cache_creation_input_tokens":25,"output_tokens":1});
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::AnthropicMessages, &request());
    let events = decoder
        .push(&json!({"type":"message_start","message":{"usage":usage}}).to_string())
        .unwrap();
    assert!(matches!(
        &events[0],
        ModelEvent::Usage(ModelUsage {
            input_tokens: Some(1030),
            cached_input_tokens: Some(1000),
            output_tokens: Some(1),
            ..
        })
    ));
    for usage in [
        json!({"input_tokens":u64::MAX,"cache_read_input_tokens":1}),
        json!({"input_tokens":3,"cache_creation_input_tokens":-1}),
    ] {
        let mut decoder = ProtocolDecoder::new(ProtocolFamily::AnthropicMessages, &request());
        assert!(
            decoder
                .push(&json!({"type":"message_start","message":{"usage":usage}}).to_string())
                .is_err()
        );
    }
    let mut decoder = ProtocolDecoder::new(ProtocolFamily::AnthropicMessages, &request());
    let events = decoder
        .push(r#"{"type":"message_start","message":{"usage":{"cache_read_input_tokens":1000}}}"#)
        .unwrap();
    assert!(matches!(
        &events[0],
        ModelEvent::Usage(ModelUsage {
            input_tokens: None,
            ..
        })
    ));
}
