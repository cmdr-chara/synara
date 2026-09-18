//! Hostile transport and resource-lifetime regressions using in-memory pipes.
use crate::rpc::{Incoming, RpcId, RpcPeer};
use serde_json::{Value, json};
use std::{
    io,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
use synara_agent::{AgentError, AgentResult};
use synara_runtime::ProcessReader;
use tokio::{
    io::{
        AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, DuplexStream, ReadBuf,
        duplex, split,
    },
    sync::mpsc,
};

fn pair(stderr: ProcessReader) -> (RpcPeer, mpsc::Receiver<Incoming>, DuplexStream) {
    let (client, agent) = duplex(65536);
    let (reader, writer) = split(client);
    let (peer, incoming) = RpcPeer::start(Box::new(reader), Box::new(writer), stderr);
    (peer, incoming, agent)
}

async fn next_line(agent: &mut BufReader<DuplexStream>) -> Value {
    let mut line = String::new();
    let bytes = tokio::time::timeout(Duration::from_secs(5), agent.read_line(&mut line))
        .await
        .unwrap()
        .unwrap();
    assert!(bytes > 0);
    serde_json::from_str(&line).unwrap()
}

async fn send(agent: &mut DuplexStream, value: Value) {
    let mut frame = serde_json::to_vec(&value).unwrap();
    frame.push(b'\n');
    tokio::time::timeout(Duration::from_secs(5), agent.write_all(&frame))
        .await
        .unwrap()
        .unwrap();
}

async fn failure(peer: &RpcPeer) -> String {
    tokio::time::timeout(Duration::from_secs(5), peer.cancelled().cancelled())
        .await
        .unwrap();
    peer.failures().borrow().clone().unwrap()
}

fn request(peer: &RpcPeer, method: &'static str) -> tokio::task::JoinHandle<AgentResult<Value>> {
    let peer = peer.clone();
    tokio::spawn(async move { peer.request(method, json!({}), Duration::from_secs(30)).await })
}

#[tokio::test]
async fn invalid_envelopes_fail_closed() {
    let cases = [
        json!([]),
        json!(true),
        json!({"jsonrpc":"1.0","method":"session/update","params":{}}),
        json!({"jsonrpc":"2.0","method":42,"params":{}}),
        json!({"jsonrpc":"2.0","method":"invalid method","params":{}}),
        json!({"jsonrpc":"2.0","method":"session/update","params":[]}),
        json!({"jsonrpc":"2.0","method":"session/update","result":{}}),
        json!({"jsonrpc":"2.0","method":"session/update","error":{}}),
        json!({"jsonrpc":"2.0","method":"session/update","id":null}),
        json!({"jsonrpc":"2.0","method":"session/update","id":1.5}),
        json!({"jsonrpc":"2.0","method":"session/update","id":""}),
        json!({"jsonrpc":"2.0","method":"session/update","id":"x".repeat(513)}),
        json!({"jsonrpc":"2.0","id":1}),
        json!({"jsonrpc":"2.0","result":{}}),
        json!({"jsonrpc":"2.0","id":1,"result":{},"error":{}}),
        json!({"jsonrpc":"2.0","id":1,"result":{},"params":{}}),
        json!({"jsonrpc":"2.0","id":1,"error":{"code":"bad","message":"x"}}),
        json!({"jsonrpc":"2.0","id":1,"error":{"code":-1,"message":3}}),
    ];
    for value in cases {
        let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
        send(&mut agent, value).await;
        assert_eq!(failure(&peer).await, "invalid JSON-RPC envelope");
    }
}

#[tokio::test]
async fn stdout_contamination_malformed_json_and_invalid_utf8_fail_closed() {
    for bytes in [b"startup banner\n".as_slice(), b"{\n", b"\xff\n"] {
        let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
        agent.write_all(bytes).await.unwrap();
        assert_eq!(failure(&peer).await, "malformed or oversized protocol frame");
    }
}

#[tokio::test]
async fn oversized_unterminated_stdout_is_bounded() {
    let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
    let bytes = vec![b'x'; 8 * 1024 * 1024 + 2];
    let _ = tokio::time::timeout(Duration::from_secs(5), agent.write_all(&bytes))
        .await
        .unwrap();
    assert_eq!(failure(&peer).await, "malformed or oversized protocol frame");
}

#[tokio::test]
async fn fragmented_utf8_and_multiple_frames_preserve_notification_order() {
    let (peer, mut incoming, mut agent) = pair(Box::new(tokio::io::empty()));
    for text in ["æ🦀", "second"] {
        let frame = format!(
            "{}\r\n",
            json!({"jsonrpc":"2.0","method":"session/update","params":{"text":text}})
        );
        for byte in frame.bytes() {
            agent.write_all(&[byte]).await.unwrap();
            tokio::task::yield_now().await;
        }
    }
    for expected in ["æ🦀", "second"] {
        let Some(Incoming::Notification { params, .. }) =
            tokio::time::timeout(Duration::from_secs(5), incoming.recv())
                .await
                .unwrap()
        else {
            panic!("expected notification")
        };
        assert_eq!(params["text"], expected);
    }
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn incomplete_frame_at_eof_is_not_accepted_or_silently_discarded() {
    let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
    agent.write_all(b"{\"jsonrpc\":\"2.0\"").await.unwrap();
    drop(agent);
    assert_eq!(failure(&peer).await, "incomplete protocol frame at exit");
}

#[tokio::test]
async fn duplicate_incoming_request_ids_are_rejected() {
    let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
    for _ in 0..2 {
        send(
            &mut agent,
            json!({"jsonrpc":"2.0","id":"same","method":"terminal/output","params":{}}),
        )
        .await;
    }
    assert_eq!(failure(&peer).await, "invalid JSON-RPC envelope");
}

#[tokio::test]
async fn numeric_and_string_callback_ids_have_distinct_ownership() {
    let (peer, mut incoming, agent) = pair(Box::new(tokio::io::empty()));
    let mut agent = BufReader::new(agent);
    for id in [json!(7), json!("7")] {
        send(
            agent.get_mut(),
            json!({"jsonrpc":"2.0","id":id,"method":"terminal/output","params":{}}),
        )
        .await;
    }
    for expected in [RpcId::Number(7), RpcId::String("7".into())] {
        let Some(Incoming::Request { id, .. }) =
            tokio::time::timeout(Duration::from_secs(5), incoming.recv())
                .await
                .unwrap()
        else {
            panic!("expected callback")
        };
        assert_eq!(id, expected);
        peer.reply(id, Ok(json!({}))).await.unwrap();
        assert_eq!(next_line(&mut agent).await["id"], expected.value());
    }
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn out_of_order_late_and_unknown_responses_cannot_complete_other_requests() {
    let (peer, _incoming, agent) = pair(Box::new(tokio::io::empty()));
    let mut agent = BufReader::new(agent);
    let first = request(&peer, "session/new");
    let first_id = next_line(&mut agent).await["id"].clone();
    let second = request(&peer, "session/list");
    let second_id = next_line(&mut agent).await["id"].clone();
    send(
        agent.get_mut(),
        json!({"jsonrpc":"2.0","id":"unknown","result":{"value":"wrong"}}),
    )
    .await;
    send(
        agent.get_mut(),
        json!({"jsonrpc":"2.0","id":second_id,"result":{"value":"second"}}),
    )
    .await;
    assert_eq!(second.await.unwrap().unwrap()["value"], "second");
    assert!(!first.is_finished());
    send(
        agent.get_mut(),
        json!({"jsonrpc":"2.0","id":second_id,"result":{"value":"duplicate"}}),
    )
    .await;
    send(
        agent.get_mut(),
        json!({"jsonrpc":"2.0","id":first_id,"result":{"value":"first"}}),
    )
    .await;
    assert_eq!(first.await.unwrap().unwrap()["value"], "first");
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn incoming_notification_backpressure_fails_instead_of_growing_unbounded() {
    let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
    let frame = b"{\"jsonrpc\":\"2.0\",\"method\":\"session/update\",\"params\":{}}\n";
    agent.write_all(&frame.repeat(257)).await.unwrap();
    assert_eq!(failure(&peer).await, "incoming protocol queue exhausted");
}

#[tokio::test]
async fn incoming_callback_ownership_has_an_independent_limit() {
    let (peer, _incoming, mut agent) = pair(Box::new(tokio::io::empty()));
    let mut frames = String::new();
    for id in 0..65 {
        frames.push_str(&format!(
            "{}\n",
            json!({"jsonrpc":"2.0","id":id,"method":"terminal/output","params":{}})
        ));
    }
    agent.write_all(frames.as_bytes()).await.unwrap();
    assert_eq!(failure(&peer).await, "incoming protocol queue exhausted");
}

#[tokio::test]
async fn pending_request_limit_does_not_disconnect_healthy_requests() {
    let (peer, _incoming, agent) = pair(Box::new(tokio::io::empty()));
    let mut agent = BufReader::new(agent);
    let mut requests = Vec::new();
    for _ in 0..128 {
        requests.push(request(&peer, "session/list"));
        next_line(&mut agent).await;
    }
    assert!(matches!(
        peer.request("session/list", json!({}), Duration::from_secs(1))
            .await,
        Err(AgentError::Limit)
    ));
    assert!(!peer.cancelled().is_cancelled());
    for request in requests {
        request.abort();
        let _ = request.await;
    }
    let recovered = request(&peer, "session/list");
    let id = next_line(&mut agent).await["id"].clone();
    send(
        agent.get_mut(),
        json!({"jsonrpc":"2.0","id":id,"result":{}}),
    )
    .await;
    recovered.await.unwrap().unwrap();
}

#[tokio::test]
async fn unsupported_callback_replies_use_protocol_errors_not_host_details() {
    let (peer, mut incoming, agent) = pair(Box::new(tokio::io::empty()));
    let mut agent = BufReader::new(agent);
    send(
        agent.get_mut(),
        json!({"jsonrpc":"2.0","id":1,"method":"vendor/unknown","params":{}}),
    )
    .await;
    let Some(Incoming::Request { id, .. }) =
        tokio::time::timeout(Duration::from_secs(5), incoming.recv())
            .await
            .unwrap()
    else {
        panic!("expected callback")
    };
    peer.reply(id, Err(AgentError::Unsupported("private-host-canary".into())))
        .await
        .unwrap();
    let reply = next_line(&mut agent).await;
    assert_eq!(reply["error"]["code"], -32601);
    assert!(!reply.to_string().contains("private-host-canary"));
    assert!(!peer.cancelled().is_cancelled());
}

#[tokio::test]
async fn stderr_floods_remain_bounded_redacted_and_independent_of_protocol_output() {
    let (stderr_reader, mut stderr_writer) = duplex(65536);
    let (peer, _incoming, agent) = pair(Box::new(stderr_reader));
    let flood = tokio::spawn(async move {
        let chunk = b"stderr-secret-canary".repeat(1024);
        for _ in 0..64 {
            stderr_writer.write_all(&chunk).await.unwrap();
        }
    });
    peer.notify("session/cancel", json!({"sessionId":"active"}))
        .await
        .unwrap();
    let mut agent = BufReader::new(agent);
    assert_eq!(next_line(&mut agent).await["method"], "session/cancel");
    tokio::time::timeout(Duration::from_secs(5), flood)
        .await
        .unwrap()
        .unwrap();
    let trace = peer.trace();
    assert!(trace.len() <= 512);
    assert!(trace.iter().any(|entry| entry.kind == "stderr"));
    assert!(!serde_json::to_string(&trace).unwrap().contains("stderr-secret"));
    assert!(!peer.cancelled().is_cancelled());
}

struct PendingReader(Arc<AtomicBool>);

impl Drop for PendingReader {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl AsyncRead for PendingReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

struct PendingShutdownWriter(Arc<AtomicBool>);

impl Drop for PendingShutdownWriter {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl AsyncWrite for PendingShutdownWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Pending
    }
}

#[tokio::test]
async fn last_owner_drop_releases_all_pipes_even_when_shutdown_never_completes() {
    let reader = Arc::new(AtomicBool::new(false));
    let writer = Arc::new(AtomicBool::new(false));
    let stderr = Arc::new(AtomicBool::new(false));
    let (peer, incoming) = RpcPeer::start(
        Box::new(PendingReader(reader.clone())),
        Box::new(PendingShutdownWriter(writer.clone())),
        Box::new(PendingReader(stderr.clone())),
    );
    let stopped = peer.cancelled();
    drop(peer);
    drop(incoming);
    assert!(stopped.is_cancelled());
    tokio::time::timeout(Duration::from_secs(3), async {
        while ![&reader, &writer, &stderr]
            .iter()
            .all(|dropped| dropped.load(Ordering::Acquire))
        {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
