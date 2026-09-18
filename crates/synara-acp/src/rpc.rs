use crate::trace::TraceLog;
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use synara_agent::{AgentError, AgentResult, TraceEntry};
use synara_runtime::{ProcessReader, ProcessWriter};
#[cfg(test)]
use tokio::io::AsyncWriteExt;
use tokio::{
    io::AsyncReadExt,
    sync::{mpsc, oneshot, watch},
};
use tokio_util::sync::CancellationToken;

#[path = "rpc_outbound.rs"]
mod outbound;
use outbound::{OutboundQueue, write_loop};

const MAX_FRAME: usize = 8 * 1024 * 1024;
const MAX_PENDING: usize = 128;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum RpcId {
    String(String),
    Number(i64),
}
impl RpcId {
    pub fn parse(value: &Value) -> AgentResult<Self> {
        match value {
            Value::String(text) if !text.is_empty() && text.len() <= 512 => {
                Ok(Self::String(text.clone()))
            }
            Value::Number(number) => number
                .as_i64()
                .map(Self::Number)
                .ok_or_else(|| invalid("request ID must be an integer")),
            _ => Err(invalid("request ID must be a bounded string or integer")),
        }
    }
    pub fn value(&self) -> Value {
        match self {
            Self::String(text) => json!(text),
            Self::Number(number) => json!(number),
        }
    }
}

pub(crate) enum Incoming {
    Request {
        id: RpcId,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    /// Serialized after preceding notifications, so response completion cannot overtake replay.
    Barrier(oneshot::Sender<()>),
}
struct Pending {
    sender: oneshot::Sender<AgentResult<Value>>,
    method: String,
    lifetime: RequestLifetime,
}
struct RequestLifetime(CancellationToken);
impl Drop for RequestLifetime {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
struct State {
    pending: Mutex<HashMap<RpcId, Pending>>,
    incoming_ids: Mutex<HashSet<RpcId>>,
    trace: Mutex<TraceLog>,
    stop: CancellationToken,
    failure: watch::Sender<Option<String>>,
}
impl State {
    fn fail(&self, reason: &str) {
        // First failure owns the diagnostic even when EOF, shutdown and a writer
        // error race. Do not replace it with a later teardown symptom.
        if !self.failure.send_if_modified(|failure| {
            if failure.is_some() {
                return false;
            }
            *failure = Some(reason.into());
            true
        }) {
            return;
        }
        self.trace.lock().unwrap().lifecycle(reason);
        self.stop.cancel();
        for (_, pending) in self.pending.lock().unwrap().drain() {
            let _ = pending
                .sender
                .send(Err(AgentError::Disconnected(reason.into())));
        }
    }
}
struct Owner {
    state: Arc<State>,
}
impl Drop for Owner {
    fn drop(&mut self) {
        self.state.fail("connection released");
    }
}
#[derive(Clone)]
pub(crate) struct RpcPeer {
    owner: Arc<Owner>,
    writer: OutboundQueue,
    incoming: mpsc::Sender<Incoming>,
    next: Arc<AtomicU64>,
}
impl RpcPeer {
    pub fn start(
        reader: ProcessReader,
        writer: ProcessWriter,
        stderr: ProcessReader,
    ) -> (Self, mpsc::Receiver<Incoming>) {
        let (failure, _) = watch::channel(None);
        let state = Arc::new(State {
            pending: Mutex::new(HashMap::new()),
            incoming_ids: Mutex::new(HashSet::new()),
            trace: Mutex::new(TraceLog::default()),
            stop: CancellationToken::new(),
            failure,
        });
        let (outgoing_tx, outgoing_rx) = OutboundQueue::new();
        let (incoming_tx, incoming_rx) = mpsc::channel(256);
        tokio::spawn(read_loop(reader, state.clone(), incoming_tx.clone()));
        tokio::spawn(write_loop(writer, state.clone(), outgoing_rx));
        tokio::spawn(stderr_loop(stderr, state.clone()));
        let peer = Self {
            owner: Arc::new(Owner { state }),
            writer: outgoing_tx,
            incoming: incoming_tx,
            next: Arc::new(AtomicU64::new(1)),
        };
        (peer, incoming_rx)
    }
    fn state(&self) -> &Arc<State> {
        &self.owner.state
    }
    pub fn cancelled(&self) -> CancellationToken {
        self.state().stop.clone()
    }
    pub fn request_lifetime(&self, id: &RpcId) -> Option<CancellationToken> {
        self.state()
            .pending
            .lock()
            .unwrap()
            .get(id)
            .map(|pending| pending.lifetime.0.clone())
    }
    pub fn failures(&self) -> watch::Receiver<Option<String>> {
        self.state().failure.subscribe()
    }
    pub fn fail(&self, reason: &str) {
        self.state().fail(reason);
    }
    pub fn trace(&self) -> Vec<TraceEntry> {
        self.state().trace.lock().unwrap().snapshot()
    }
    pub fn clear_trace(&self) {
        self.state().trace.lock().unwrap().clear();
    }
    pub async fn request(
        &self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> AgentResult<Value> {
        validate_method(method)?;
        let id = RpcId::String(format!(
            "synara-{}",
            self.next.fetch_add(1, Ordering::Relaxed)
        ));
        let (sender, receiver) = oneshot::channel();
        let lifetime = self.state().stop.child_token();
        {
            let mut pending = self.state().pending.lock().unwrap();
            if self.state().stop.is_cancelled() {
                return Err(AgentError::Disconnected("connection closed".into()));
            }
            if pending.len() >= MAX_PENDING {
                return Err(AgentError::Limit);
            }
            pending.insert(
                id.clone(),
                Pending {
                    sender,
                    method: method.into(),
                    lifetime: RequestLifetime(lifetime.clone()),
                },
            );
        }
        let _guard = PendingGuard {
            state: Arc::downgrade(self.state()),
            id: id.clone(),
        };
        let operation = async {
            self.send(
                json!({"jsonrpc":"2.0", "id":id.value(), "method":method, "params":params}),
                Some(lifetime),
            )
            .await?;
            receiver
                .await
                .map_err(|_| AgentError::Disconnected("response channel closed".into()))?
        };
        tokio::time::timeout(timeout, operation)
            .await
            .map_err(|_| AgentError::Timeout)?
    }
    pub async fn notify(&self, method: &str, params: Value) -> AgentResult<()> {
        validate_method(method)?;
        self.send(
            json!({"jsonrpc":"2.0", "method":method, "params":params}),
            None,
        )
        .await
    }
    pub async fn reply(&self, id: RpcId, result: AgentResult<Value>) -> AgentResult<()> {
        if !self.state().incoming_ids.lock().unwrap().remove(&id) {
            return Err(invalid("response ID does not own an incoming request"));
        }
        let value = match result {
            Ok(value) => json!({"jsonrpc":"2.0", "id":id.value(), "result":value}),
            Err(error) => {
                let (code, message) = match error {
                    AgentError::Invalid(_) => (-32602, "Invalid request parameters"),
                    AgentError::Unsupported(_) => (-32601, "Method is not supported"),
                    AgentError::Cancelled => (-32800, "Request cancelled"),
                    AgentError::Limit => (-32001, "Resource limit reached"),
                    _ => (-32603, "Client operation failed or was denied"),
                };
                json!({"jsonrpc":"2.0", "id":id.value(), "error":{"code":code,"message":message}})
            }
        };
        self.send(value, None).await
    }
    pub async fn barrier(&self) -> AgentResult<()> {
        let (sender, receiver) = oneshot::channel();
        tokio::select! {
            () = self.state().stop.cancelled() => Err(AgentError::Disconnected("connection closed".into())),
            result = async {
                self.incoming.send(Incoming::Barrier(sender)).await.map_err(|_| AgentError::EventDelivery)?;
                receiver.await.map_err(|_| AgentError::EventDelivery)
            } => result,
        }
    }
    async fn send(&self, value: Value, lifetime: Option<CancellationToken>) -> AgentResult<()> {
        self.writer.send(value, self.state(), lifetime).await
    }
}
struct PendingGuard {
    state: Weak<State>,
    id: RpcId,
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        if let Some(state) = self.state.upgrade() {
            state.pending.lock().unwrap().remove(&self.id);
        }
    }
}
fn invalid(message: &str) -> AgentError {
    AgentError::Invalid(message.into())
}
fn validate_method(method: &str) -> AgentResult<()> {
    if method.is_empty()
        || method.len() > 128
        || !method
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_/$.-".contains(&b))
    {
        return Err(invalid("invalid method name"));
    }
    Ok(())
}

struct Frames {
    pending: Vec<u8>,
}
impl Frames {
    fn push(&mut self, input: &[u8]) -> AgentResult<Vec<Value>> {
        let mut messages = Vec::new();
        for segment in input.split_inclusive(|byte| *byte == b'\n') {
            if self.pending.len() + segment.len() > MAX_FRAME + 1 {
                return Err(AgentError::Limit);
            }
            self.pending.extend_from_slice(segment);
            if self.pending.last() == Some(&b'\n') {
                let value: Value = serde_json::from_slice(&self.pending)
                    .map_err(|_| invalid("malformed JSON on protocol stdout"))?;
                self.pending.clear();
                messages.push(value);
            }
        }
        Ok(messages)
    }
}
async fn read_loop(mut reader: ProcessReader, state: Arc<State>, incoming: mpsc::Sender<Incoming>) {
    let mut frames = Frames {
        pending: Vec::new(),
    };
    let mut chunk = [0; 8192];
    loop {
        let read = tokio::select! { () = state.stop.cancelled() => return, result = reader.read(&mut chunk) => result };
        let count = match read {
            Ok(0) => {
                state.fail(if frames.pending.is_empty() {
                    "protocol stdout closed"
                } else {
                    "incomplete protocol frame at exit"
                });
                return;
            }
            Ok(count) => count,
            Err(_) => {
                state.fail("protocol stdout read failed");
                return;
            }
        };
        let messages = match frames.push(&chunk[..count]) {
            Ok(messages) => messages,
            Err(_) => {
                state.fail("malformed or oversized protocol frame");
                return;
            }
        };
        for value in messages {
            if let Err(error) = dispatch(value, &state, &incoming) {
                let reason = match error {
                    AgentError::Limit => "incoming protocol queue exhausted",
                    _ => "invalid JSON-RPC envelope",
                };
                state.fail(reason);
                return;
            }
        }
    }
}
fn dispatch(value: Value, state: &State, incoming: &mpsc::Sender<Incoming>) -> AgentResult<()> {
    let object = value
        .as_object()
        .ok_or_else(|| invalid("batch messages are not supported"))?;
    if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return Err(invalid("invalid JSON-RPC version"));
    }
    state.trace.lock().unwrap().push("in", &value);
    if let Some(method) = object.get("method") {
        let method = method.as_str().ok_or_else(|| invalid("invalid method"))?;
        validate_method(method)?;
        if object.contains_key("result") || object.contains_key("error") {
            return Err(invalid("request includes response fields"));
        }
        let params = object.get("params").cloned().unwrap_or_else(|| json!({}));
        if !params.is_object() {
            return Err(invalid("ACP parameters must be an object"));
        }
        let message = if let Some(id) = object.get("id") {
            let id = RpcId::parse(id)?;
            let mut ids = state.incoming_ids.lock().unwrap();
            if ids.len() >= 64 {
                return Err(AgentError::Limit);
            }
            if !ids.insert(id.clone()) {
                return Err(invalid("duplicate incoming request ID"));
            }
            Incoming::Request {
                id,
                method: method.into(),
                params,
            }
        } else {
            Incoming::Notification {
                method: method.into(),
                params,
            }
        };
        incoming.try_send(message).map_err(|_| AgentError::Limit)?;
    } else {
        if object.contains_key("result") == object.contains_key("error")
            || object.contains_key("params")
        {
            return Err(invalid("response must have exactly one result or error"));
        }
        let id = RpcId::parse(
            object
                .get("id")
                .ok_or_else(|| invalid("response ID missing"))?,
        )?;
        let result = if let Some(error) = object.get("error") {
            let code = error
                .get("code")
                .and_then(Value::as_i64)
                .ok_or_else(|| invalid("error code missing"))?;
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("error message missing"))?;
            // Remote errors are user-visible, but are never included verbatim in debug traces.
            Err(AgentError::Remote {
                code,
                message: message.chars().take(4096).collect(),
            })
        } else {
            Ok(object.get("result").cloned().unwrap_or(Value::Null))
        };
        if let Some(pending) = state.pending.lock().unwrap().remove(&id) {
            pending.lifetime.0.cancel();
            let _method = pending.method;
            let _ = pending.sender.send(result);
        }
    }
    Ok(())
}
async fn stderr_loop(mut stderr: ProcessReader, state: Arc<State>) {
    let mut bytes = [0; 4096];
    loop {
        let read = tokio::select! { () = state.stop.cancelled() => return, result = stderr.read(&mut bytes) => result };
        match read {
            Ok(0) | Err(_) => return,
            Ok(count) => state.trace.lock().unwrap().stderr(count),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, BufReader, duplex, split};
    fn pair() -> (RpcPeer, mpsc::Receiver<Incoming>, tokio::io::DuplexStream) {
        let (client, agent) = duplex(65536);
        let (reader, writer) = split(client);
        let (peer, incoming) = RpcPeer::start(
            Box::new(reader),
            Box::new(writer),
            Box::new(tokio::io::empty()),
        );
        (peer, incoming, agent)
    }
    #[test]
    fn framing_handles_split_utf8_crlf_and_multiple_messages() {
        let mut frames = Frames { pending: vec![] };
        let wire = "{\"text\":\"æ🦀\"}\r\n{\"value\":2}\n".as_bytes();
        let mut messages = vec![];
        for byte in wire {
            messages.extend(frames.push(&[*byte]).unwrap());
        }
        assert_eq!(messages, vec![json!({"text":"æ🦀"}), json!({"value":2})]);
    }
    #[test]
    fn malformed_and_oversized_frames_are_rejected() {
        assert!(Frames { pending: vec![] }.push(b"not JSON\n").is_err());
        assert!(
            Frames { pending: vec![] }
                .push(&vec![b'x'; MAX_FRAME + 2])
                .is_err()
        );
    }
    #[tokio::test]
    async fn request_response_and_notification_are_independent() {
        let (peer, mut incoming, agent) = pair();
        let task = tokio::spawn({
            let peer = peer.clone();
            async move {
                peer.request("initialize", json!({}), Duration::from_secs(2))
                    .await
            }
        });
        let mut agent = BufReader::new(agent);
        let mut line = String::new();
        agent.read_line(&mut line).await.unwrap();
        let request: Value = serde_json::from_str(&line).unwrap();
        agent
            .get_mut()
            .write_all(
                format!(
                    "{}\n{}\n",
                    json!({"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"a"}}),
                    json!({"jsonrpc":"2.0","id":request["id"],"result":{"protocolVersion":1}})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        assert_eq!(task.await.unwrap().unwrap()["protocolVersion"], 1);
        assert!(matches!(
            incoming.recv().await,
            Some(Incoming::Notification { .. })
        ));
    }
    #[tokio::test]
    async fn timeout_and_task_cancellation_remove_pending_ids() {
        let (peer, _incoming, _agent) = pair();
        assert!(matches!(
            peer.request("initialize", json!({}), Duration::from_millis(2))
                .await,
            Err(AgentError::Timeout)
        ));
        assert!(peer.state().pending.lock().unwrap().is_empty());
        let task = tokio::spawn({
            let peer = peer.clone();
            async move {
                peer.request("session/new", json!({}), Duration::from_secs(100))
                    .await
            }
        });
        tokio::task::yield_now().await;
        task.abort();
        let _ = task.await;
        assert!(peer.state().pending.lock().unwrap().is_empty());
    }
    #[tokio::test]
    async fn exit_resolves_pending_request_instead_of_hanging() {
        let (peer, _incoming, agent) = pair();
        let task = tokio::spawn({
            let peer = peer.clone();
            async move {
                peer.request("initialize", json!({}), Duration::from_secs(30))
                    .await
            }
        });
        tokio::task::yield_now().await;
        drop(agent);
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(1), task)
                .await
                .unwrap()
                .unwrap(),
            Err(AgentError::Disconnected(_))
        ));
    }
    #[tokio::test]
    async fn client_callbacks_require_an_owned_request_id() {
        let (peer, mut incoming, mut agent) = pair();
        agent
            .write_all(
                b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"terminal/output\",\"params\":{}}\n",
            )
            .await
            .unwrap();
        let Some(Incoming::Request { id, .. }) = incoming.recv().await else {
            panic!("request expected")
        };
        peer.reply(id.clone(), Ok(json!({"output":""})))
            .await
            .unwrap();
        assert!(peer.reply(id, Ok(json!({}))).await.is_err());
        let mut agent = BufReader::new(agent);
        let mut line = String::new();
        agent.read_line(&mut line).await.unwrap();
        assert_eq!(serde_json::from_str::<Value>(&line).unwrap()["id"], 7);
    }
}

#[cfg(test)]
#[path = "rpc_lifecycle_tests.rs"]
mod lifecycle_tests;
