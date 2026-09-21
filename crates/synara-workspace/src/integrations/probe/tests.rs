use super::*;
use std::{
    io::Write,
    net::TcpListener,
    sync::{Arc, Mutex},
    thread,
};

struct Response {
    status: u16,
    mime: &'static str,
    body: String,
    headers: Vec<(&'static str, &'static str)>,
}
fn json_response(body: Value) -> Response {
    Response {
        status: 200,
        mime: "application/json",
        body: body.to_string(),
        headers: vec![],
    }
}
fn fixture(
    responses: Vec<Response>,
) -> (ManagedMcp, Arc<Mutex<Vec<String>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    listener.set_nonblocking(true).unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let handle = thread::spawn(move || {
        for response in responses {
            let deadline = Instant::now() + Duration::from_secs(5);
            let (mut socket, _) = loop {
                match listener.accept() {
                    Ok(socket) => break socket,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            Instant::now() < deadline,
                            "expected another fixture request"
                        );
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("fixture accept: {error}"),
                }
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut reader = BufReader::new(socket.try_clone().unwrap());
            let mut request = String::new();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line.to_ascii_lowercase().starts_with("content-length:") {
                    length = line
                        .split_once(':')
                        .unwrap()
                        .1
                        .trim()
                        .parse::<usize>()
                        .unwrap();
                }
                request.push_str(&line);
                if line == "\r\n" {
                    break;
                }
            }
            assert!(length < 64 * 1024);
            let mut body = vec![0; length];
            reader.read_exact(&mut body).unwrap();
            request.push_str(&String::from_utf8(body).unwrap());
            captured.lock().unwrap().push(request);
            write!(socket,"HTTP/1.1 {} Fixture\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",response.status,response.mime,response.body.len()).unwrap();
            for (key, value) in response.headers {
                write!(socket, "{key}: {value}\r\n").unwrap();
            }
            write!(socket, "\r\n{}", response.body).unwrap();
        }
    });
    let mut config = ManagedMcp::new(TaskId::new(), "generic-fixture".into());
    config.name = "Fixture tools".into();
    config.endpoint = format!("http://127.0.0.1:{port}/mcp");
    (config, requests, handle)
}
fn discovery(tools: bool) -> Value {
    json!({"jsonrpc":"2.0","id":1,"result":{"resultType":"complete","supportedVersions":[MODERN],"capabilities":if tools {json!({"tools":{}})}else{json!({})},"_meta":{"io.modelcontextprotocol/serverInfo":{"name":"fixture-canary-token","version":"1"}}}})
}
fn tool(name: &str) -> Value {
    json!({"name":name,"description":"Fixture only","inputSchema":{"type":"object","properties":{}}})
}
#[test]
fn integrations_probe_modern_http_validates_metadata_and_negotiates_paginated_tools() {
    let (config, captured, server) = fixture(vec![
        json_response(discovery(true)),
        json_response(
            json!({"jsonrpc":"2.0","id":10,"result":{"resultType":"complete","tools":[tool("first")],"nextCursor":"page-two"}}),
        ),
        json_response(
            json!({"jsonrpc":"2.0","id":11,"result":{"resultType":"complete","tools":[tool("second")]}}),
        ),
    ]);
    let report = probe(
        config,
        Some(SecretValue::new(b"fixture-canary-token".to_vec()).unwrap()),
    )
    .unwrap();
    server.join().unwrap();
    assert_eq!(report.protocol, MODERN);
    assert_eq!(report.tools.len(), 2);
    assert_eq!(report.server, "[REDACTED]");
    assert!(!report.cleanup_warning);
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for request in requests.iter() {
        let (headers, body) = request.split_once("\r\n\r\n").unwrap();
        let rpc: Value = serde_json::from_str(body).unwrap();
        assert!(
            headers
                .to_ascii_lowercase()
                .contains("mcp-protocol-version: 2026-07-28")
        );
        assert!(
            headers
                .to_ascii_lowercase()
                .contains(&format!("mcp-method: {}", rpc["method"].as_str().unwrap()))
        );
        assert_eq!(
            rpc["params"]["_meta"]["io.modelcontextprotocol/protocolVersion"],
            MODERN
        );
        assert_eq!(
            rpc["params"]["_meta"]["io.modelcontextprotocol/clientCapabilities"],
            json!({})
        );
        assert!(!body.contains("tools/call"));
    }
    assert!(requests[2].contains("page-two"));
}
#[test]
fn integrations_probe_legacy_initializes_notifies_and_deletes_its_temporary_session() {
    let init=Response{status:200,mime:"application/json",body:json!({"jsonrpc":"2.0","id":2,"result":{"protocolVersion":"2025-06-18","serverInfo":{"name":"legacy fixture","version":"1"},"capabilities":{"tools":{}}}}).to_string(),headers:vec![("Mcp-Session-Id","fixture-session")]};
    let (config, captured, server) = fixture(vec![
        Response {
            status: 400,
            mime: "text/plain",
            body: "Initialize first".into(),
            headers: vec![],
        },
        init,
        Response {
            status: 202,
            mime: "application/json",
            body: String::new(),
            headers: vec![],
        },
        json_response(json!({"jsonrpc":"2.0","id":10,"result":{"tools":[]}})),
        Response {
            status: 204,
            mime: "application/json",
            body: String::new(),
            headers: vec![],
        },
    ]);
    let report = probe(config, None).unwrap();
    server.join().unwrap();
    assert_eq!(report.protocol, "2025-06-18");
    assert!(!report.cleanup_warning);
    let requests = captured.lock().unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests[1].contains("initialize"));
    assert!(requests[2].contains("notifications/initialized"));
    for request in &requests[2..] {
        assert!(
            request
                .to_ascii_lowercase()
                .contains("mcp-session-id: fixture-session")
        );
        assert!(
            request
                .to_ascii_lowercase()
                .contains("mcp-protocol-version: 2025-06-18")
        );
    }
    assert!(requests[4].starts_with("DELETE /mcp "));
}
#[test]
fn integrations_probe_never_lists_unadvertised_tools_or_treats_http_success_as_mcp() {
    let (config, captured, server) = fixture(vec![json_response(discovery(false))]);
    assert!(probe(config, None).unwrap().tools.is_empty());
    server.join().unwrap();
    assert_eq!(captured.lock().unwrap().len(), 1);
    for body in [
        json!({"ok":true}),
        json!({"jsonrpc":"2.0","id":999,"result":{}}),
        json!({"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":"credential-canary"}}),
    ] {
        let (config, _, server) = fixture(vec![json_response(body)]);
        let error = probe(config, None).unwrap_err().to_string();
        server.join().unwrap();
        assert!(!error.contains("credential-canary"));
    }
}
#[test]
fn integrations_probe_auth_redirect_and_modern_errors_never_fallback_or_follow() {
    for (status, code) in [
        (401, -1),
        (403, -1),
        (302, -1),
        (400, -32022),
        (400, -32020),
        (400, -32021),
        (404, -32601),
    ] {
        let (config, captured, server) = fixture(vec![Response {
            status,
            mime: "application/json",
            body:
                json!({"jsonrpc":"2.0","id":1,"error":{"code":code,"message":"credential-canary"}})
                    .to_string(),
            headers: vec![("Location", "http://127.0.0.1:1/leak")],
        }]);
        let error = probe(config, None).unwrap_err().to_string();
        server.join().unwrap();
        assert!(!error.contains("credential-canary"));
        assert_eq!(captured.lock().unwrap().len(), 1);
    }
}
#[test]
fn integrations_probe_sse_handles_comments_and_refuses_callbacks_malformed_or_oversized_streams() {
    let valid = format!(
        ": keepalive\n\ndata: {{\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\",\"params\":{{}}}}\n\ndata: {}\n\n",
        discovery(false)
    );
    let (config, _, server) = fixture(vec![Response {
        status: 200,
        mime: "text/event-stream",
        body: valid,
        headers: vec![],
    }]);
    assert!(probe(config, None).is_ok());
    server.join().unwrap();
    for data in [
        "data: broken\n\n".into(),
        "data: {\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"roots/list\"}\n\n".into(),
        format!(":{}\n\n", "x".repeat(MAX_BODY as usize + 1)),
        "data: {\"jsonrpc\":\"2.0\",\"id\":999,\"result\":{}}\n\n".into(),
    ] {
        assert!(read_sse(data.as_bytes(), 1).is_err());
    }
}
#[test]
fn integrations_probe_rejects_bad_versions_duplicates_and_unbounded_pagination() {
    let mut bad = discovery(false);
    bad["result"]["supportedVersions"] = json!(["2099-01-01"]);
    let (config, _, server) = fixture(vec![json_response(bad)]);
    assert!(probe(config, None).is_err());
    server.join().unwrap();
    for tools in [
        json!([tool("same"), tool("same")]),
        json!([{"name":"missing-schema"}]),
    ] {
        let (config, _, server) = fixture(vec![
            json_response(discovery(true)),
            json_response(json!({"jsonrpc":"2.0","id":10,"result":{"tools":tools}})),
        ]);
        assert!(probe(config, None).is_err());
        server.join().unwrap();
    }
    let (config, _, server) = fixture(vec![
        json_response(discovery(true)),
        json_response(json!({"jsonrpc":"2.0","id":10,"result":{"tools":[],"nextCursor":"loop"}})),
        json_response(json!({"jsonrpc":"2.0","id":11,"result":{"tools":[],"nextCursor":"loop"}})),
    ]);
    assert!(probe(config, None).is_err());
    server.join().unwrap();
}
