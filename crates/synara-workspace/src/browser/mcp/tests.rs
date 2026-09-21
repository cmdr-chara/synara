use super::*;
use std::io::{Read, Write};
use synara_agent::{AgentResult, AgentSession, AgentError, ConnectionInfo, SessionOptions};
use synara_core::{AgentCapabilities, ConnectionId};
use synara_browser::session::{Command, Capabilities, Event};
struct Connection(tokio::sync::watch::Sender<ConnectionInfo>);
#[async_trait::async_trait]
impl AgentConnection for Connection {
    fn info(&self) -> ConnectionInfo { self.0.borrow().clone() }
    fn observe(&self) -> tokio::sync::watch::Receiver<ConnectionInfo> { self.0.subscribe() }
    async fn new_session(&self, _: SessionOptions) -> AgentResult<Arc<dyn AgentSession>> { Err(AgentError::Unsupported("fixture".into())) }
    async fn disconnect(&self) -> AgentResult<()> { self.0.send_modify(|v| v.state = ConnectionState::Disconnected); Ok(()) }
}
struct Port(Arc<Mutex<Vec<Command>>>);
impl NativePort for Port {
    fn capabilities(&self) -> Capabilities { Capabilities { navigation: true, document: true, input: true, ..Default::default() } }
    fn send(&mut self, command: Command) -> Result<()> { self.0.lock().unwrap().push(command); Ok(()) }
}
async fn rpc(context: &ContextServer, method: &str, params: Value, extra: &str) -> (u16, Value) {
    let ContextServer::Http { url, headers, .. } = context else { panic!() };
    let host = url.trim_start_matches("http://").trim_end_matches("/mcp").to_owned();
    let auth = headers["Authorization"].clone(); let extra = extra.to_owned();
    let body = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}).to_string();
    tokio::task::spawn_blocking(move || {
        let mut socket = TcpStream::connect(&host).unwrap(); socket.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let wire = format!("POST /mcp HTTP/1.1\r\nHost: {host}\r\nAuthorization: {auth}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: {}\r\n{extra}\r\n{body}", body.len());
        socket.write_all(wire.as_bytes()).unwrap(); let mut response = String::new(); socket.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        (head.split_whitespace().nth(1).unwrap().parse().unwrap(), serde_json::from_str(body).unwrap_or(Value::Null))
    }).await.unwrap()
}
fn content(v: &Value) -> Value { serde_json::from_str(v["result"]["content"][0]["text"].as_str().unwrap()).unwrap() }
#[tokio::test(flavor="multi_thread", worker_threads=2)]
async fn real_loopback_transport_queues_once_requires_native_approval_and_revokes() {
    let directory = tempfile::tempdir().unwrap(); let workspace = WorkspaceService::memory().unwrap();
    let project = workspace.add_local_workspace(directory.path().into()).await.unwrap();
    let profile = workspace.profiles().await.unwrap()[0].clone();
    let task = workspace.create_task(project.id, "Browser fixture".into(), profile.id.clone()).await.unwrap();
    let (tx, _) = tokio::sync::watch::channel(ConnectionInfo { id: ConnectionId::new(), state: ConnectionState::Connected,
        identity: None, capabilities: AgentCapabilities { mcp_http: true, ..Default::default() }, authentication: vec![], host: "Local".into(), error: None });
    let connection = Arc::new(Connection(tx)); let commands = Arc::new(Mutex::new(vec![]));
    let owner = BrowserService::new(Box::new(Port(commands.clone())));
    let endpoint = start(owner.clone(), workspace.clone(), task.id, profile, connection.clone()).await.unwrap();
    let context = endpoint.context().unwrap();
    assert_eq!(rpc(&context, "initialize", json!({"protocolVersion":"2025-06-18"}), "").await.0, 200);
    assert_eq!(rpc(&context, "tools/list", json!({}), "Origin: null\r\n").await.0, 403);
    let (_, tabs) = rpc(&context, "tools/call", json!({"name":"browser_tabs","arguments":{}}), "").await;
    let tab = content(&tabs)["tabs"][0].clone();
    let params = json!({"name":"browser_request","arguments":{"nonce":"nav1","tab":tab,"operation":{"operation":"navigate","url":"https://example.test/"}}});
    let (_, first) = rpc(&context, "tools/call", params.clone(), "").await;
    let request = content(&first)["request"].clone();
    let (_, repeat) = rpc(&context, "tools/call", params, "").await; assert_eq!(content(&repeat)["request"], request);
    assert_eq!(commands.lock().unwrap().len(), 1, "No navigation before consent");
    let (_, rejected) = rpc(&context, "tools/call", json!({"name":"browser_approve","arguments":{"request":request}}), "").await;
    assert!(rejected.get("error").is_some());
    let id: HostRequestId = serde_json::from_value(request.clone()).unwrap();
    owner.with(|s, n| s.decide(id, true, n)).unwrap();
    let command = commands.lock().unwrap().last().unwrap().clone();
    let Command::Navigate { tab, navigation, .. } = command else { panic!() };
    owner.with(|s, _| s.event(Event::Committed { tab, navigation, url: "https://example.test/".into(), title: "Fixture".into() })).unwrap();
    let (_, result) = rpc(&context, "tools/call", json!({"name":"browser_result","arguments":{"request":request}}), "").await;
    assert_eq!(content(&result)["receipt"]["state"]["state"], "complete");
    let old = owner.bind(99).unwrap(); let new = owner.bind(99).unwrap();
    owner.revoke_generation(99, old.generation); assert!(old.tabs().is_err()); assert_eq!(new.tabs().unwrap().len(), 1);
    drop(endpoint); tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(owner.with(|s, _| Ok(s.task_tabs(task.id.0.as_u128()))).unwrap().is_empty());
    owner.shutdown();
}
