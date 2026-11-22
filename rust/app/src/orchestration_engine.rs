// Orchestration engine: native orchestration.dispatchCommand.
//
// Intercepted only when SYNARA_NO_UPSTREAM is set: in that mode the front door
// owns state.sqlite directly (the original server is not started, so there is
// no EXCLUSIVE lock), and commands are decided, appended to
// orchestration_events, receipted and projected in one transaction.
// Without the flag every command keeps relaying to the original server.

use rusqlite::{params, Connection};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

use crate::todo::{iso_now_ms, state_dir, WsErr};

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn uuid_v4() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut bytes);
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

pub fn handles(tag: &str) -> bool {
    tag == "orchestration.dispatchCommand" && std::env::var("SYNARA_NO_UPSTREAM").is_ok()
}

fn open_state_rw() -> Result<Connection, WsErr> {
    let db = PathBuf::from(state_dir()).join("state.sqlite");
    Connection::open(&db).map_err(|e| err(format!("could not open state.sqlite: {e}")))
}

fn s(command: &Map<String, Value>, key: &str) -> Option<String> {
    command.get(key).and_then(Value::as_str).map(str::to_string)
}

fn opt(command: &Map<String, Value>, key: &str) -> Value {
    command.get(key).cloned().unwrap_or(Value::Null)
}

fn is_group_container_kind(kind: &str) -> bool {
    kind == "group" || kind == "studio"
}

pub fn handle(_tag: &str, payload: &Value) -> Result<Value, WsErr> {
    let command = payload
        .as_object()
        .ok_or_else(|| err("orchestration.dispatchCommand expects a command object"))?;
    let command_type = command
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| err("orchestration.dispatchCommand payload is missing type"))?;
    match command_type {
        "thread.create" => thread_create(command),
        other => Err(err(format!(
            "orchestration.dispatchCommand {other} is not ported yet (native engine slice)"
        ))),
    }
}

fn thread_create(command: &Map<String, Value>) -> Result<Value, WsErr> {
    let command_id = s(command, "commandId").ok_or_else(|| err("thread.create missing commandId"))?;
    let thread_id = s(command, "threadId").ok_or_else(|| err("thread.create missing threadId"))?;
    let project_id = s(command, "projectId").ok_or_else(|| err("thread.create missing projectId"))?;
    let title = s(command, "title").ok_or_else(|| err("thread.create missing title"))?;
    let created_at =
        s(command, "createdAt").ok_or_else(|| err("thread.create missing createdAt"))?;

    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    // requireProject: the project must exist and not be deleted.
    let project_kind: String = conn
        .query_row(
            "SELECT kind FROM projection_projects WHERE project_id = ?1 AND deleted_at IS NULL",
            params![project_id],
            |r| r.get(0),
        )
        .map_err(|_| err(format!("project {project_id} not found")))?;

    // requireThreadAbsent.
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        return Err(err(format!("thread {thread_id} already exists")));
    }

    let is_group = is_group_container_kind(&project_kind);
    let interaction_mode = s(command, "interactionMode").unwrap_or_else(|| "default".into());
    let env_mode = if is_group {
        "local".to_string()
    } else {
        s(command, "envMode").unwrap_or_else(|| "local".into())
    };
    let branch = if is_group { Value::Null } else { opt(command, "branch") };
    let worktree_path = if is_group {
        Value::Null
    } else {
        opt(command, "worktreePath")
    };
    let working_directory = if is_group {
        if command.contains_key("workingDirectory") {
            opt(command, "workingDirectory")
        } else {
            opt(command, "worktreePath")
        }
    } else {
        opt(command, "workingDirectory")
    };
    // deriveAssociatedWorktreeMetadata.
    let (associated_path, associated_branch, associated_ref) = if is_group {
        (Value::Null, Value::Null, Value::Null)
    } else {
        let path = if command.contains_key("associatedWorktreePath") {
            opt(command, "associatedWorktreePath")
        } else {
            worktree_path.clone()
        };
        let branch_v = if command.contains_key("associatedWorktreeBranch") {
            opt(command, "associatedWorktreeBranch")
        } else if !worktree_path.is_null() {
            branch.clone()
        } else {
            Value::Null
        };
        let ref_v = if command.contains_key("associatedWorktreeRef") {
            opt(command, "associatedWorktreeRef")
        } else if command.contains_key("associatedWorktreeBranch") {
            opt(command, "associatedWorktreeBranch")
        } else if !worktree_path.is_null() {
            branch.clone()
        } else {
            Value::Null
        };
        (path, branch_v, ref_v)
    };
    let create_branch_flow_completed = if is_group {
        false
    } else {
        command
            .get("createBranchFlowCompleted")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    let is_pinned = command
        .get("isPinned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let last_known_pr = opt(command, "lastKnownPr");
    let pull_requests = if last_known_pr.is_null() {
        json!([])
    } else {
        json!([{
            "pullRequest": last_known_pr,
            "source": "checkout",
            "linkedAt": created_at,
        }])
    };

    let mut ep = Map::new();
    let put = |ep: &mut Map<String, Value>, k: &str, v: Value| {
        ep.insert(k.to_string(), v);
    };
    put(&mut ep, "threadId", json!(thread_id));
    put(&mut ep, "projectId", json!(project_id));
    put(&mut ep, "title", json!(title));
    put(&mut ep, "modelSelection", opt(command, "modelSelection"));
    put(&mut ep, "runtimeMode", opt(command, "runtimeMode"));
    put(&mut ep, "interactionMode", json!(interaction_mode));
    put(&mut ep, "envMode", json!(env_mode));
    put(&mut ep, "branch", branch.clone());
    put(&mut ep, "worktreePath", worktree_path.clone());
    put(&mut ep, "workingDirectory", working_directory.clone());
    put(&mut ep, "associatedWorktreePath", associated_path.clone());
    put(&mut ep, "associatedWorktreeBranch", associated_branch.clone());
    put(&mut ep, "associatedWorktreeRef", associated_ref.clone());
    put(&mut ep, "createBranchFlowCompleted", json!(create_branch_flow_completed));
    put(&mut ep, "isPinned", json!(is_pinned));
    put(&mut ep, "parentThreadId", opt(command, "parentThreadId"));
    if let Some(cs) = s(command, "creationSource") {
        put(&mut ep, "creationSource", json!(cs));
        put(&mut ep, "sourceThreadId", opt(command, "sourceThreadId"));
        put(&mut ep, "sourceTurnId", opt(command, "sourceTurnId"));
        put(&mut ep, "gatewayOperationId", opt(command, "gatewayOperationId"));
        put(&mut ep, "gatewayOperationIndex", opt(command, "gatewayOperationIndex"));
    }
    put(&mut ep, "subagentAgentId", opt(command, "subagentAgentId"));
    put(&mut ep, "subagentNickname", opt(command, "subagentNickname"));
    put(&mut ep, "subagentRole", opt(command, "subagentRole"));
    put(&mut ep, "forkSourceThreadId", Value::Null);
    let sidechat = opt(command, "sidechatContext");
    if !sidechat.is_null() {
        put(&mut ep, "sidechatContext", sidechat.clone());
        put(&mut ep, "sidechatLastActivityAt", json!(created_at));
        put(&mut ep, "sidechatExpiredAt", Value::Null);
    }
    put(&mut ep, "lastKnownPr", last_known_pr.clone());
    put(&mut ep, "pullRequests", pull_requests.clone());
    put(&mut ep, "handoff", Value::Null);
    put(&mut ep, "createdAt", json!(created_at));
    put(&mut ep, "updatedAt", json!(created_at));

    let tx = conn
        .unchecked_transaction()
        .map_err(|e| err(e.to_string()))?;

    let stream_version: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(stream_version), 0) + 1 \
             FROM orchestration_events WHERE aggregate_kind = 'thread' AND stream_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;

    tx.execute(
        "INSERT INTO orchestration_events \
         (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at, \
          command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json) \
         VALUES (?1, 'thread', ?2, ?3, 'thread.created', ?4, ?5, NULL, ?5, 'client', ?6, '{}')",
        params![
            uuid_v4(),
            thread_id,
            stream_version,
            created_at,
            command_id,
            Value::Object(ep).to_string(),
        ],
    )
    .map_err(|e| err(format!("append thread.created event: {e}")))?;
    let sequence = tx.last_insert_rowid();

    tx.execute(
        "INSERT INTO orchestration_command_receipts \
         (command_id, aggregate_kind, aggregate_id, accepted_at, result_sequence, status, error, \
          fingerprint_version, command_fingerprint) \
         VALUES (?1, 'thread', ?2, ?3, ?4, 'accepted', NULL, NULL, NULL)",
        params![command_id, thread_id, iso_now_ms(), sequence],
    )
    .map_err(|e| err(format!("write command receipt: {e}")))?;

    let str_or_null = |v: &Value| -> Option<String> { v.as_str().map(str::to_string) };
    tx.execute(
        "INSERT INTO projection_threads (\
           thread_id, project_id, title, branch, worktree_path, latest_turn_id, \
           created_at, updated_at, deleted_at, runtime_mode, interaction_mode, \
           model_selection_json, handoff_json, env_mode, fork_source_thread_id, \
           associated_worktree_path, associated_worktree_branch, associated_worktree_ref, \
           archived_at, parent_thread_id, subagent_agent_id, subagent_nickname, subagent_role, \
           latest_user_message_at, pending_approval_count, pending_user_input_count, \
           has_actionable_proposed_plan, last_known_pr_json, create_branch_flow_completed, \
           sidechat_source_thread_id, is_pinned, pinned_messages_json, notes, \
           creation_source, source_thread_id, source_turn_id, gateway_operation_id, \
           gateway_operation_index, working_directory, settled_at, goal, goal_started_at, \
           goal_paused_at, goal_achievements_json, sidechat_last_activity_at, sidechat_expired_at, \
           claude_cache_review_json, latest_human_message_at, sidechat_context_json, \
           snoozed_until, snooze_reminder_at, fork_source_message_id, \
           usage_limit_recovery_source_json, usage_limit_recovery_json, \
           usage_limit_recovery_cancelled_at, usage_limit_recovery_cancelled_turn_id, \
           usage_limit_recovery_last_reset_at, pull_requests_json\
         ) VALUES (\
           ?1, ?2, ?3, ?4, ?5, NULL, ?6, ?6, NULL, ?7, ?8, ?9, NULL, ?10, NULL, \
           ?11, ?12, ?13, NULL, ?14, ?15, ?16, ?17, NULL, 0, 0, 0, ?18, ?19, \
           NULL, ?20, NULL, NULL, ?21, ?22, ?23, ?24, ?25, ?26, NULL, NULL, NULL, \
           NULL, NULL, ?27, NULL, NULL, NULL, ?28, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?29)",
        params![
            thread_id,                                                             // 1
            project_id,                                                            // 2
            title,                                                                 // 3
            str_or_null(&branch),                                                  // 4
            str_or_null(&worktree_path),                                           // 5
            created_at,                                                            // 6
            s(command, "runtimeMode").unwrap_or_else(|| "full-access".into()),      // 7
            interaction_mode,                                                      // 8
            opt(command, "modelSelection").to_string(),                            // 9
            env_mode,                                                              // 10
            str_or_null(&associated_path),                                         // 11
            str_or_null(&associated_branch),                                       // 12
            str_or_null(&associated_ref),                                          // 13
            s(command, "parentThreadId"),                                          // 14
            s(command, "subagentAgentId"),                                         // 15
            s(command, "subagentNickname"),                                        // 16
            s(command, "subagentRole"),                                            // 17
            if last_known_pr.is_null() { None } else { Some(last_known_pr.to_string()) }, // 18
            create_branch_flow_completed as i64,                                   // 19
            is_pinned as i64,                                                      // 20
            s(command, "creationSource"),                                          // 21
            s(command, "sourceThreadId"),                                          // 22
            s(command, "sourceTurnId"),                                            // 23
            s(command, "gatewayOperationId"),                                      // 24
            command.get("gatewayOperationIndex").and_then(Value::as_i64),          // 25
            str_or_null(&working_directory),                                       // 26
            if sidechat.is_null() { None } else { Some(created_at.clone()) },       // 27 sidechat_last_activity_at
            if sidechat.is_null() { None } else { Some(sidechat.to_string()) },     // 28 sidechat_context_json
            pull_requests.to_string(),                                             // 29 pull_requests_json
        ],
    )
    .map_err(|e| err(format!("project thread: {e}")))?;

    tx.commit().map_err(|e| err(e.to_string()))?;
    Ok(json!({ "sequence": sequence }))
}
