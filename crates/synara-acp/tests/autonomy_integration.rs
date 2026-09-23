//! Real owned ACP child processes and authenticated loopback MCP. No paid account,
//! production credentials, ambient network destination or external project.
use serde_json::{Value, json};
use std::{
    io::{Read, Write},
    sync::Arc,
    time::Duration,
};
use synara_acp::AcpBackend;
use synara_agent::DenyInteractions;
use synara_core::*;
use synara_workspace::*;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

async fn setup() -> (tempfile::TempDir, Arc<Controller>, Task) {
    let root = tempfile::tempdir().unwrap();
    let workspace = WorkspaceService::open(root.path().join("autonomy.sqlite3"))
        .await
        .unwrap();
    let project = workspace
        .add_local_workspace(root.path().into())
        .await
        .unwrap();
    workspace
        .save_profiles(
            ["alpha", "beta"]
                .into_iter()
                .map(|id| AgentProfile {
                    registry: None,
                    id: id.into(),
                    name: id.into(),
                    command: env!("CARGO_BIN_EXE_synara-acp-fixture").into(),
                    args: vec![
                        "--integration-fixture".into(),
                        id.into(),
                        "--gateway-http".into(),
                    ],
                    inherit_env: vec![],
                    secret_env: Default::default(),
                })
                .collect(),
        )
        .await
        .unwrap();
    let task = workspace
        .create_task(project.id, "Workflow parent".into(), "alpha".into())
        .await
        .unwrap();
    let controller = Arc::new(Controller::new(
        workspace,
        Arc::new(AcpBackend::default()),
        Arc::new(DenyInteractions),
    ));
    (root, controller, task)
}
fn spec() -> WorkflowSpec {
    WorkflowSpec {
        title: "Owned DAG".into(),
        concurrency: 2,
        steps: vec![
            WorkflowStepSpec {
                title: "Alpha result".into(),
                agent_id: "alpha".into(),
                instruction: "hello".into(),
                depends_on: vec![],
            },
            WorkflowStepSpec {
                title: "Beta independent result".into(),
                agent_id: "beta".into(),
                instruction: "hello".into(),
                depends_on: vec![],
            },
            WorkflowStepSpec {
                title: "Synthesis".into(),
                agent_id: "alpha".into(),
                instruction: "hello".into(),
                depends_on: vec![0, 1],
            },
        ],
    }
}
#[tokio::test]
async fn autonomy_dag_executes_real_children_and_visible_dependencies_without_parent_replay() {
    let (root, controller, parent) = setup().await;
    controller
        .workspace
        .record(
            parent.thread_id,
            ThreadEvent::TextDelta {
                message_id: Some("private-parent".into()),
                role: Role::User,
                text: "Private parent text must not be implicitly shared".into(),
            },
        )
        .await
        .unwrap();
    let before = controller
        .workspace
        .thread(parent.thread_id)
        .await
        .unwrap()
        .last_sequence;
    let graph = controller
        .workspace
        .create_workflow(parent.id, spec())
        .await
        .unwrap();
    let finished = tokio::time::timeout(
        Duration::from_secs(20),
        controller.run_workflow(parent.id, graph.id, graph.revision),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(finished.phase, WorkflowPhase::Completed);
    assert!(
        finished
            .steps
            .iter()
            .all(|s| s.state == WorkflowStepState::Completed && s.attempts == 1)
    );
    let synthesis = controller
        .workspace
        .task(finished.steps[2].task)
        .await
        .unwrap();
    let thread = controller
        .workspace
        .thread(synthesis.thread_id)
        .await
        .unwrap();
    let user = thread
        .messages
        .iter()
        .find(|m| m.role == Role::User)
        .unwrap();
    assert!(user.text.contains("Hello from alpha") && user.text.contains("Hello from beta"));
    assert!(!user.text.contains("Private parent text"));
    assert_eq!(
        controller
            .workspace
            .thread(parent.thread_id)
            .await
            .unwrap()
            .last_sequence,
        before
    );
    assert!(
        controller
            .workspace
            .session(parent.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    let a = controller
        .details(finished.steps[0].task)
        .await
        .unwrap()
        .unwrap();
    let c = controller
        .details(finished.steps[2].task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(a.connection.id, c.connection.id);
    assert_ne!(a.session_id, c.session_id);
    let status = controller.gateway_status(parent.id).await.unwrap();
    assert!(!status.to_string().contains("Private parent text"));
    controller.shutdown().await.unwrap();
    let reopened = WorkspaceService::open(root.path().join("autonomy.sqlite3"))
        .await
        .unwrap();
    assert_eq!(
        reopened.workflow(parent.id).await.unwrap().unwrap().phase,
        WorkflowPhase::Completed
    );
}
#[tokio::test]
async fn autonomy_pause_stops_active_child_and_never_launches_dependency_or_auto_retries() {
    let (_root, controller, parent) = setup().await;
    let mut plan = spec();
    plan.concurrency = 1;
    plan.steps.truncate(2);
    plan.steps[0].instruction = "hold".into();
    plan.steps[1].depends_on = vec![0];
    let graph = controller
        .workspace
        .create_workflow(parent.id, plan)
        .await
        .unwrap();
    let run = {
        let c = controller.clone();
        let id = graph.id;
        let rev = graph.revision;
        tokio::spawn(async move { c.run_workflow(parent.id, id, rev).await })
    };
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let t = controller
                .workspace
                .task(graph.steps[0].task)
                .await
                .unwrap();
            if controller
                .workspace
                .thread(t.thread_id)
                .await
                .unwrap()
                .messages
                .iter()
                .any(|m| m.role == Role::Assistant)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert!(controller.workspace.archive_task(parent.id).await.is_err());
    assert!(
        controller
            .workspace
            .archive_task(graph.steps[1].task)
            .await
            .is_err()
    );
    controller.pause_workflow(parent.id, false).await.unwrap();
    let paused = tokio::time::timeout(Duration::from_secs(10), run)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(paused.phase, WorkflowPhase::Paused);
    assert_eq!(paused.steps[0].state, WorkflowStepState::Interrupted);
    let dependent = controller
        .workspace
        .task(paused.steps[1].task)
        .await
        .unwrap();
    assert!(
        controller
            .workspace
            .thread(dependent.thread_id)
            .await
            .unwrap()
            .messages
            .is_empty()
    );
    assert!(
        controller
            .workspace
            .session(dependent.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(!controller.autonomy.workflow_running(parent.id));
    let resumed = controller
        .run_workflow(parent.id, paused.id, paused.revision)
        .await
        .unwrap();
    assert_ne!(resumed.phase, WorkflowPhase::Completed);
    assert_eq!(resumed.steps[0].attempts, 1);
    controller.shutdown().await.unwrap();
}
#[tokio::test]
async fn autonomy_precancel_and_stale_graph_never_launch_children() {
    let (_root, controller, parent) = setup().await;
    let graph = controller
        .workspace
        .create_workflow(parent.id, spec())
        .await
        .unwrap();
    assert!(
        controller
            .run_workflow(parent.id, Uuid::new_v4(), graph.revision)
            .await
            .is_err()
    );
    let token = CancellationToken::new();
    token.cancel();
    assert!(
        controller
            .run_workflow_interruptible(parent.id, graph.id, graph.revision, token)
            .await
            .is_err()
    );
    for step in graph.steps {
        let task = controller.workspace.task(step.task).await.unwrap();
        assert!(
            controller
                .workspace
                .session(task.thread_id)
                .await
                .unwrap()
                .is_none()
        );
    }
    controller.shutdown().await.unwrap();
}
#[tokio::test]
async fn autonomy_changed_child_and_unknown_route_fail_without_overwriting_human_edits() {
    let (_root, controller, parent) = setup().await;
    let mut plan = spec();
    plan.concurrency = 1;
    let graph = controller
        .workspace
        .create_workflow(parent.id, plan)
        .await
        .unwrap();
    controller
        .workspace
        .save_task_draft(graph.steps[0].task, "human changed this".into())
        .await
        .unwrap();
    let value = controller
        .run_workflow(parent.id, graph.id, graph.revision)
        .await
        .unwrap();
    assert_ne!(value.phase, WorkflowPhase::Completed);
    assert_eq!(
        controller
            .workspace
            .task_draft(graph.steps[0].task)
            .await
            .unwrap(),
        "human changed this"
    );
    let child = controller
        .workspace
        .task(graph.steps[0].task)
        .await
        .unwrap();
    assert!(
        controller
            .workspace
            .session(child.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    controller.shutdown().await.unwrap();
}
struct Client {
    address: String,
    authorization: String,
}
impl Client {
    fn from_config(value: Value) -> Self {
        let config = &value["mcpServers"]["synara"];
        Self {
            address: config["url"]
                .as_str()
                .unwrap()
                .trim_start_matches("http://")
                .trim_end_matches("/mcp")
                .into(),
            authorization: config["headers"]["Authorization"].as_str().unwrap().into(),
        }
    }
    async fn rpc(&self, method: &str, params: Value) -> Value {
        let address = self.address.clone();
        let authorization = self.authorization.clone();
        let body = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}).to_string();
        tokio::task::spawn_blocking(move || {
            let mut socket = std::net::TcpStream::connect(&address).unwrap();
            socket.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
            socket.write_all(format!("POST /mcp HTTP/1.1\r\nHost: {address}\r\nAuthorization: {authorization}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMCP-Protocol-Version: 2025-11-25\r\nContent-Length: {}\r\n\r\n{body}", body.len()).as_bytes()).unwrap();
            let mut result = String::new(); socket.take(4 * 1024 * 1024).read_to_string(&mut result).unwrap();
            assert!(result.starts_with("HTTP/1.1 200"));
            serde_json::from_str(result.split_once("\r\n\r\n").unwrap().1).unwrap()
        }).await.unwrap()
    }
    async fn tool(&self, name: &str, arguments: Value) -> Value {
        self.rpc("tools/call", json!({"name":name,"arguments":arguments}))
            .await
    }
}
fn payload(value: &Value) -> Value {
    serde_json::from_str(
        value["result"]["content"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["type"] == "text")
            .unwrap()["text"]
            .as_str()
            .unwrap(),
    )
    .unwrap()
}
#[tokio::test]
async fn autonomy_external_mcp_requires_native_approval_and_never_crosses_task_or_receipt_scope() {
    let (_root, controller, parent) = setup().await;
    let info = controller
        .enable_gateway(
            parent.id,
            GatewayClientKind::External,
            "Owned test client".into(),
        )
        .await
        .unwrap();
    let client = Client::from_config(
        controller
            .autonomy
            .gateway
            .configuration(parent.id, info.id)
            .unwrap(),
    );
    let init = client.rpc("initialize", json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"fixture","version":"1"}})).await;
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    let list = client.rpc("tools/list", json!({})).await;
    assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 4);
    let requested = client
        .tool(
            "synara_request",
            json!({"nonce":"create","operation":{"operation":"create_workflow","spec":spec()}}),
        )
        .await;
    let receipt: Uuid = serde_json::from_value(payload(&requested)["request"].clone()).unwrap();
    assert!(
        controller
            .workspace
            .workflow(parent.id)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        controller
            .workspace
            .session(parent.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        payload(
            &client
                .tool("synara_result", json!({"request":receipt}))
                .await
        )["state"],
        "pending"
    );
    assert!(
        client
            .tool("approve", json!({"request":receipt}))
            .await
            .get("error")
            .is_some()
    );
    assert!(
        controller
            .approve_gateway(TaskId::new(), receipt)
            .await
            .is_err()
    );
    controller
        .approve_gateway(parent.id, receipt)
        .await
        .unwrap();
    assert!(
        controller
            .approve_gateway(parent.id, receipt)
            .await
            .is_err()
    );
    assert_eq!(
        payload(
            &client
                .tool("synara_result", json!({"request":receipt}))
                .await
        )["state"],
        "completed"
    );
    let graph = controller
        .workspace
        .workflow(parent.id)
        .await
        .unwrap()
        .unwrap();
    let count = controller.workspace.catalog().await.unwrap().tasks.len();
    let repeated = client
        .tool(
            "synara_request",
            json!({"nonce":"create","operation":{"operation":"create_workflow","spec":spec()}}),
        )
        .await;
    assert_eq!(payload(&repeated)["request"], receipt.to_string());
    assert_eq!(payload(&repeated)["state"], "completed");
    assert_eq!(payload(&repeated)["requires_native_approval"], false);
    assert_eq!(
        controller.workspace.catalog().await.unwrap().tasks.len(),
        count
    );
    let run = client.tool("synara_request", json!({"nonce":"run","operation":{"operation":"run_workflow","workflow":graph.id,"revision":graph.revision}})).await;
    let run_id: Uuid = serde_json::from_value(payload(&run)["request"].clone()).unwrap();
    controller.approve_gateway(parent.id, run_id).await.unwrap();
    let report = payload(
        &client
            .tool("synara_result", json!({"request":run_id}))
            .await,
    );
    assert_eq!(report["result"]["reports"].as_array().unwrap().len(), 3);
    assert!(
        report["result"]["reports"][0]["output"]
            .as_str()
            .unwrap()
            .contains("Hello from alpha")
    );
    assert_eq!(report["result"]["outputs_are_untrusted"], true);
    assert_eq!(
        controller
            .workspace
            .workflow(parent.id)
            .await
            .unwrap()
            .unwrap()
            .phase,
        WorkflowPhase::Completed
    );
    let other = controller
        .workspace
        .create_task(parent.project_id, "Different root".into(), "beta".into())
        .await
        .unwrap();
    let b = controller
        .enable_gateway(
            other.id,
            GatewayClientKind::External,
            "Other test client".into(),
        )
        .await
        .unwrap();
    let other_client = Client::from_config(
        controller
            .autonomy
            .gateway
            .configuration(other.id, b.id)
            .unwrap(),
    );
    assert_eq!(
        other_client
            .tool("synara_result", json!({"request":receipt}))
            .await["result"]["isError"],
        true
    );
    controller.autonomy.gateway.revoke(parent.id, Some(info.id));
    assert!(controller.autonomy.gateway.clients(parent.id).is_empty());
    assert!(!controller.autonomy.gateway.clients(other.id).is_empty());
    controller.shutdown().await.unwrap();
}
#[tokio::test]
async fn autonomy_agent_gateway_requires_negotiation_and_restart_revokes_it() {
    let (_root, controller, parent) = setup().await;
    assert!(
        controller
            .enable_gateway(parent.id, GatewayClientKind::Agent, "Agent".into())
            .await
            .is_err()
    );
    assert!(
        controller
            .workspace
            .session(parent.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    controller.connect(parent.id).await.unwrap();
    let info = controller
        .enable_gateway(parent.id, GatewayClientKind::Agent, "Agent".into())
        .await
        .unwrap();
    assert!(
        controller
            .workspace
            .session(parent.thread_id)
            .await
            .unwrap()
            .is_none()
    );
    controller
        .submit(parent.id, "gateway-context".into())
        .await
        .unwrap();
    let transcript = controller.workspace.thread(parent.thread_id).await.unwrap();
    assert!(
        transcript
            .messages
            .iter()
            .any(|m| m.text == "Scoped HTTP gateway received in this fresh ACP session")
    );
    let client = Client::from_config(
        controller
            .autonomy
            .gateway
            .configuration(parent.id, info.id)
            .unwrap(),
    );
    assert_eq!(
        client.tool("synara_status", json!({})).await["result"]["isError"],
        false
    );
    controller.restart(parent.id).await.unwrap();
    assert!(controller.autonomy.gateway.clients(parent.id).is_empty());
    controller.shutdown().await.unwrap();
}
