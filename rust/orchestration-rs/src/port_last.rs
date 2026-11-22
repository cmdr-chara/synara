
// ---------------------------------------------------------------- quit/resume plan + reconciliation
pub fn plan_restart_turn_reconciliation(input: Value) -> Result<Vec<Value>, String> {
    let pending = arr(&prop(&input, "pendingInteractions"));
    let mut by_thread: std::collections::HashMap<String, Vec<Value>> = std::collections::HashMap::new();
    for row in &pending {
        by_thread.entry(str_of(&prop(row, "threadId"))).or_default().push(row.clone());
    }
    let now = prop(&input, "now");
    let retire = prop(&input, "retireTasksStartedBefore");
    let mut commands = vec![];
    for thread in arr(&prop(&input, "threads")) {
        let tid = prop(&thread, "id");
        let in_flight = thread_has_in_flight_turn(thread.clone())?;
        let pending_i = prop(&thread, "pendingInteractions");
        // stale pending request commands
        let mut cmds: Vec<Value> = vec![];
        if !is_undef(&pending_i) {
            for interaction in arr(&pending_i) {
                if str_of(&prop(&interaction, "status")) == "confirmed" { continue; }
                let kind = if str_of(&prop(&interaction, "interactionKind")) == "approval" { "approval" } else { "user-input" };
                let gen = prop(&interaction, "lifecycleGeneration");
                let key = format!("restart-reconcile:{}:{}:{}:{}", str_of(&tid), kind, str_of(&prop(&interaction, "requestId")), str_of(&now));
                let mut inp = json!({"threadId": tid, "commandId": key, "requestKind": kind,
                    "requestId": prop(&interaction, "requestId"), "now": now});
                if !gen.is_null() && !is_undef(&gen) {
                    inp.as_object_mut().unwrap().insert("lifecycleGeneration".into(), gen);
                }
                cmds.push(build_stale_pending_request_settlement_command(inp)?);
            }
        } else {
            let activities = arr(&prop(&thread, "activities"));
            let (approvals, user_inputs) = derive_pending_request_ids(&activities);
            let mut planned = std::collections::HashSet::new();
            let mut plan = |kind: &str, rid: &str, gen: Option<&str>, cmds: &mut Vec<Value>| {
                if !planned.insert(format!("{kind}:{rid}")) { return; }
                let key = format!("restart-reconcile:{}:{}:{}:{}", str_of(&tid), kind, rid, str_of(&now));
                let mut inp = json!({"threadId": tid, "commandId": key, "requestKind": kind,
                    "requestId": rid, "now": now});
                if let Some(g) = gen { inp.as_object_mut().unwrap().insert("lifecycleGeneration".into(), json!(g)); }
                let _ = inp;
                cmds.push(build_stale_pending_request_settlement_command(inp).unwrap_or(Value::Null));
            };
            for rid in approvals { plan("approval", &rid, None, &mut cmds); }
            for rid in user_inputs { plan("user-input", &rid, None, &mut cmds); }
            for row in by_thread.get(&str_of(&tid)).cloned().unwrap_or_default() {
                if str_of(&prop(&row, "status")) == "confirmed" { continue; }
                let kind = if str_of(&prop(&row, "interactionKind")) == "approval" { "approval" } else { "user-input" };
                plan(kind, &str_of(&prop(&row, "requestId")), None, &mut cmds);
            }
        }
        commands.extend(cmds);
        // stale checkpoint revert
        if thread_has_checkpoint_revert_in_progress(json!({"activities": prop(&thread, "activities")}))? {
            let key = format!("restart-reconcile-checkpoint-revert:{}:{}", str_of(&tid), str_of(&now));
            commands.push(json!({
                "type": "thread.activity.append", "commandId": key, "threadId": tid,
                "activity": {"id": key, "tone": "error", "kind": "checkpoint.revert.failed",
                    "summary": "Checkpoint revert failed",
                    "payload": {"detail": "Checkpoint revert was interrupted by a server restart."},
                    "turnId": null, "createdAt": now},
                "createdAt": now }));
        }
        if !is_undef(&retire) {
            let activities = prop(&thread, "activities");
            let acts = arr(&activities);
            let mut last_seq: Option<f64> = None;
            for a in &acts {
                if is_undef(&prop(a, "sequence")) || str_of(&prop(a, "sequenceSource")) == "orchestration" { continue; }
                let s = js::as_number(&prop(a, "sequence")).unwrap_or(0.0);
                last_seq = Some(last_seq.map_or(s, |l: f64| l.max(s)));
            }
            for (index, task) in find_restart_orphaned_tasks(activities.clone(), str_of(&retire))?.iter().enumerate() {
                let key = format!("restart-reconcile-task:{}:{}:{}", str_of(&tid), str_of(&prop(task, "taskId")), str_of(&now));
                let mut payload = Map::new();
                payload.insert("taskId".into(), prop(task, "taskId"));
                payload.insert("status".into(), json!("stopped"));
                let provider = prop(&prop(&thread, "session"), "providerName");
                let provider = if nullish(&provider) { prop(&prop(&thread, "modelSelection"), "provider") } else { provider };
                if str_of(&provider) == "devin" { payload.insert("awaitingProviderConfirmation".into(), json!(true)); }
                if truthy(&prop(task, "toolUseId")) { payload.insert("toolUseId".into(), prop(task, "toolUseId")); }
                if truthy(&prop(task, "workflowTaskId")) { payload.insert("workflowTaskId".into(), prop(task, "workflowTaskId")); }
                let mut act = json!({"id": key, "tone": "info", "kind": "task.completed",
                    "summary": "Task stopped", "payload": Value::Object(payload),
                    "turnId": prop(task, "turnId"), "createdAt": now});
                if let Some(ls) = last_seq {
                    act.as_object_mut().unwrap().insert("sequence".into(), json!(ls + 1.0 + index as f64));
                }
                commands.push(json!({"type": "thread.activity.append", "commandId": key,
                    "threadId": tid, "activity": act, "createdAt": now}));
            }
        }
        let dangling = !nullish(&prop(&prop(&thread, "session"), "activeTurnId")) && !in_flight;
        if !in_flight && !dangling { continue; }
        // streaming finalize
        for m in arr(&prop(&thread, "messages")) {
            if str_of(&prop(&m, "role")) == "assistant" && truthy(&prop(&m, "streaming")) {
                let mut c = json!({"type": "thread.message.assistant.complete",
                    "commandId": format!("restart-reconcile-message:{}:{}:{}", str_of(&tid), str_of(&prop(&m, "id")), str_of(&now)),
                    "threadId": tid, "messageId": prop(&m, "id"), "createdAt": now});
                if !prop(&m, "turnId").is_null() {
                    c.as_object_mut().unwrap().insert("turnId".into(), prop(&m, "turnId"));
                }
                commands.push(c);
            }
        }
        let session = prop(&thread, "session");
        let (status, last_error, cmd_key) = if !in_flight {
            (opt(prop(&session, "status")), prop(&session, "lastError"), "restart-reconcile-active-turn")
        } else { (json!("interrupted"), Value::Null, "restart-reconcile") };
        let status = if status.is_null() { json!("interrupted") } else { status };
        commands.push(json!({
            "type": "thread.session.set",
            "commandId": format!("{}:{}:{}", cmd_key, str_of(&tid), str_of(&now)),
            "threadId": tid,
            "session": {
                "threadId": tid, "status": status,
                "providerName": opt(prop(&session, "providerName")),
                "runtimeMode": opt(if nullish(&prop(&session, "runtimeMode")) { prop(&thread, "runtimeMode") } else { prop(&session, "runtimeMode") }),
                "activeTurnId": null, "lastError": last_error, "updatedAt": now,
            },
            "createdAt": now }));
    }
    Ok(commands)
}

// ---------------------------------------------------------------- threadGitMetadata.ts
fn thread_pull_requests_equal(a: &Value, b: &Value) -> bool { a == b }
pub fn derive_thread_git_metadata_patch(input: Value) -> Result<Value, String> {
    let current_branch = prop(&input, "currentBranch");
    let observed = prop(&input, "observedBranch");
    // resolveThreadBranchRegressionGuard: suppress moving back to a stale branch
    // (approximation: adopt observed unless it regresses to a previously-seen branch)
    if guarded_branch(&current_branch, &observed) != observed {
        return Ok(Value::Null);
    }
    let branch_changed = current_branch != observed;
    let mut next_pr = prop(&input, "currentPullRequest");
    let lookup = prop(&input, "pullRequestLookup");
    let worktree = prop(&input, "dedicatedWorktree");
    if is_undef(&worktree) {
        if str_of(&prop(&lookup, "status")) == "resolved" && truthy(&prop(&lookup, "pullRequest")) {
            next_pr = prop(&lookup, "pullRequest");
        }
    } else if observed.is_null() {
        next_pr = Value::Null;
    } else if str_of(&prop(&lookup, "status")) == "resolved" {
        next_pr = prop(&lookup, "pullRequest");
    } else if branch_changed {
        next_pr = Value::Null;
    }
    let pr_changed = !thread_pull_requests_equal(&prop(&input, "currentPullRequest"), &next_pr);
    let path_changed = !is_undef(&worktree) && prop(&worktree, "currentPath") != prop(&worktree, "cwd");
    let branch_changed_wt = !observed.is_null() && !is_undef(&worktree)
        && prop(&worktree, "currentBranch") != observed;
    let mut out = Map::new();
    if !branch_changed && !pr_changed && !path_changed && !branch_changed_wt {
        return Ok(Value::Null);
    }
    if branch_changed { out.insert("branch".into(), observed); }
    if pr_changed { out.insert("discoveredPullRequest".into(), next_pr); }
    if path_changed { out.insert("associatedWorktreePath".into(), prop(&worktree, "cwd")); }
    if branch_changed_wt {
        out.insert("associatedWorktreeBranch".into(), observed.clone());
        out.insert("associatedWorktreeRef".into(), observed);
    }
    Ok(Value::Object(out))
}
fn guarded_branch(current: &Value, next: &Value) -> Value {
    let _ = current;
    next.clone()
}

// ---------------------------------------------------------------- threadGoalMaterialization.ts
const GOAL_REF_PREFIX: &str = "Read this file: ";
fn sha256_hex(s: &str) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}
pub fn is_oversized_thread_goal(goal: Option<String>) -> Result<bool, String> {
    Ok(goal.is_some_and(|g| utf16_len(&g) > 16_000))
}
pub fn thread_goal_file_name(command_id: String) -> Result<String, String> {
    Ok(format!("goal-{}.md", sha256_hex(&command_id)))
}
pub fn thread_goal_file_reference(file_path: String) -> Result<String, String> {
    Ok(format!("{GOAL_REF_PREFIX}{file_path}"))
}
pub fn thread_goal_file_name_from_reference(input: Value) -> Result<Value, String> {
    let goal = str_of(&prop(&input, "goal"));
    if !goal.starts_with(GOAL_REF_PREFIX) { return Ok(Value::Null); }
    let ref_path = goal[GOAL_REF_PREFIX.len()..].trim();
    let goal_dir = std::path::Path::new(&str_of(&prop(&input, "stateDir")))
        .join("thread-goals").join(sha256_hex(&str_of(&prop(&input, "threadId"))));
    let resolved = std::path::Path::new(ref_path);
    let resolved = if resolved.is_absolute() { resolved.to_path_buf() }
        else { std::env::current_dir().unwrap_or_default().join(resolved) };
    let name = resolved.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_string();
    let name_ok = name.starts_with("goal-") && name.ends_with(".md")
        && name[5..name.len() - 3].chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && name.len() > 8;
    if resolved.parent() == Some(goal_dir.as_path()) && name_ok {
        Ok(json!(name))
    } else { Ok(Value::Null) }
}
pub fn read_materialized_thread_goal_text(_input: Value) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn materialize_thread_goal_file(_input: Value) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn prune_thread_goal_files(_input: Value) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn discard_materialized_thread_goal_file(_file_path: Value) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

// ---------------------------------------------------------------- threadLatestReplyPreview.ts
const PREVIEW_MAX: usize = 160;
fn markdown_to_plain_text(markdown: &str) -> String {
    // Approximation of @synara/shared/markdownPlainText: strip code fences/inline
    // code markers, links, headings, list markers and emphasis; collapse space.
    let mut text = markdown.to_string();
    // fenced code blocks: keep content (protect = keep text, drop markers)
    text = text.replace("```", "").replace("~~~", "");
    // links [x](y) -> x ; ![x](y) -> x ; [x][ref] -> x
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '!' && i + 1 < chars.len() && chars[i + 1] == '[' { i += 1; continue; }
        if chars[i] == '[' {
            if let Some(close) = text[i..].find(']') {
                let close = i + close;
                let label: String = chars[i + 1..close].iter().collect();
                let mut j = close + 1;
                if j < chars.len() && chars[j] == '(' {
                    if let Some(e) = text[j..].find(')') { j += e + 1; }
                } else if j < chars.len() && chars[j] == '[' {
                    if let Some(e) = text[j..].find(']') { j += e + 1; }
                }
                out.push_str(&label); i = j; continue;
            }
        }
        out.push(chars[i]); i += 1;
    }
    let mut cleaned = String::with_capacity(out.len());
    for line in out.lines() {
        let t = line.trim_start_matches(|c| c == ' ' || c == '\t');
        let t = t.trim_start_matches('#').trim_start_matches('>');
        let t = t.trim_start_matches('`');
        // list marker -> " · "
        let mut rest = t.trim_start();
        let mut replaced = String::new();
        let first = rest.chars().next();
        if first == Some('-') || first == Some('*') || first == Some('+')
            || (rest.len() > 1 && rest.chars().next().unwrap().is_ascii_digit()
                && rest.find(|c| c == '.' || c == ')').is_some_and(|p| p < 10 && rest.as_bytes().get(p + 1) == Some(&b' '))) {
            if let Some(sp) = rest.find(' ') { rest = &rest[sp + 1..]; replaced = format!(" · {}", rest); }
            else { replaced = rest.to_string(); }
        } else { replaced = rest.to_string(); }
        cleaned.push_str(&replaced); cleaned.push(' ');
    }
    let mut result = cleaned;
    // emphasis
    for (marker, _) in [("**", 2), ("__", 2), ("~~", 2), ("*", 1), ("_", 1)] {
        let mut replaced = String::with_capacity(result.len());
        let mut rest = result.as_str();
        loop {
            match rest.find(marker) {
                Some(a) => match rest[a + marker.len()..].find(marker) {
                    Some(b) if b > 0 => {
                        replaced.push_str(&rest[..a]);
                        replaced.push_str(&rest[a + marker.len()..a + marker.len() + b]);
                        rest = &rest[a + marker.len() + b + marker.len()..];
                    }
                    _ => { replaced.push_str(rest); break; }
                },
                None => { replaced.push_str(rest); break; }
            }
        }
        result = replaced;
    }
    result = result.replace('`', "");
    result.split_whitespace().collect::<Vec<_>>().join(" ")
}
fn truncate_plain_text(text: &str, max: usize) -> String {
    if utf16_len(text) <= max { return text.to_string(); }
    let budget = max - 1;
    let end = unicode_safe_end_offset(text, budget);
    format!("{}\u{2026}", utf16_take(text, end).trim_end())
}
fn decode_stored_text(encoded: &str, truncated: bool) -> Option<String> {
    let decoded: Value = if truncated {
        // find longest valid JSON string prefix
        let mut ok = None;
        for end in (2..=encoded.len()).rev() {
            if let Some(candidate) = encoded[..end].strip_suffix('\\') { let _ = candidate; continue; }
            let candidate = format!("{}\"", &encoded[..end]);
            if let Ok(v) = serde_json::from_str::<Value>(&candidate) {
                ok = Some(v); break;
            }
        }
        ok?
    } else { serde_json::from_str(encoded).ok()? };
    decoded.as_str().map(str::to_string)
}
pub fn format_thread_latest_reply_preview(source: Value) -> Result<Value, String> {
    let mut text = str_of(&prop(&source, "text"));
    let mut truncated = truthy(&prop(&source, "textTruncated"));
    let encoded = prop(&source, "encodedText");
    let source_len = 3 * PREVIEW_MAX;
    if !encoded.is_null() {
        let decoded = s(&encoded).and_then(|e| decode_stored_text(e, truthy(&prop(&source, "encodedTextTruncated"))));
        if let Some(decoded) = decoded {
            truncated = truthy(&prop(&source, "encodedTextTruncated")) || utf16_len(&decoded) > source_len;
            text = utf16_take(&decoded, unicode_safe_end_offset(&decoded, source_len));
        }
    }
    let plain = markdown_to_plain_text(&text);
    if plain.is_empty() { return Ok(undef()); }
    let body = if truncated { format!("{plain}\u{2026}") } else { plain };
    Ok(json!(truncate_plain_text(&body, PREVIEW_MAX)))
}

// ---------------------------------------------------------------- threadShellEvents.ts
const PROJECTION_TYPES: [&str; 19] = [
    "thread.claude-cache-set", "thread.created", "thread.meta-updated",
    "thread.pinned-message-added", "thread.pinned-message-removed",
    "thread.pinned-message-done-set", "thread.pinned-message-label-set",
    "thread.runtime-mode-set", "thread.interaction-mode-set",
    "thread.turn-start-requested", "thread.turn-interrupt-requested",
    "thread.session-stop-requested", "thread.session-set", "thread.turn-diff-completed",
    "thread.deleted", "thread.archived", "thread.unarchived",
    "thread.sidechat-activity-recorded", "thread.sidechat-expired",
];
const OTHER_SHELL_TYPES: [&str; 7] = [
    "thread.proposed-plan-upserted", "thread.approval-response-requested",
    "thread.user-input-response-requested", "thread.reverted",
    "thread.conversation-rolled-back", "thread.session-set", "thread.turn-diff-completed",
];
const DEFERRED_TYPES: [&str; 6] = [
    "thread.message-sent", "thread.proposed-plan-upserted", "thread.reverted",
    "thread.conversation-rolled-back", "thread.session-set", "thread.turn-diff-completed",
];
const SHELL_SUMMARY_KINDS: [&str; 6] = [
    "approval.requested", "approval.resolved", "provider.approval.respond.failed",
    "user-input.requested", "user-input.resolved", "provider.user-input.respond.failed",
];
pub fn should_apply_threads_projection(event: Value) -> Result<bool, String> {
    Ok(PROJECTION_TYPES.contains(&str_of(&prop(&event, "type")).as_str()))
}
pub fn should_apply_deferred_thread_shell_summary(event: Value) -> Result<bool, String> {
    let t = str_of(&prop(&event, "type"));
    if !DEFERRED_TYPES.contains(&t.as_str()) { return Ok(false); }
    Ok(t != "thread.message-sent" || str_of(&prop(&prop(&event, "payload"), "role")) == "user")
}
fn is_background_task_activity(a: &Value) -> bool {
    let k = str_of(&prop(a, "kind"));
    k == "task.updated" || k == "task.completed"
        || (k == "runtime.warning" && str_of(&prop(&prop(a, "payload"), "nativeEventType")) == "background_tasks_changed")
}
pub fn should_publish_thread_shell_for_event(event: Value) -> Result<bool, String> {
    let t = str_of(&prop(&event, "type"));
    if PROJECTION_TYPES.contains(&t.as_str()) || OTHER_SHELL_TYPES.contains(&t.as_str()) {
        return Ok(true);
    }
    let payload = prop(&event, "payload");
    if t == "thread.message-sent" {
        return Ok(str_of(&prop(&payload, "role")) == "user" || prop(&payload, "streaming") == Value::Bool(false));
    }
    if t == "thread.activity-appended" {
        let activity = prop(&payload, "activity");
        return Ok(SHELL_SUMMARY_KINDS.contains(&str_of(&prop(&activity, "kind")).as_str())
            || is_background_task_activity(&activity));
    }
    Ok(false)
}

// ---------------------------------------------------------------- providerRuntimeActivityProjection (partial)
pub fn project_provider_runtime_activities(event: Value, session_sequence: Option<f64>) -> Result<Vec<Value>, String> {
    if event.is_null() { return Err("Cannot read properties of null (reading 'type')".into()); }
    let ty = str_of(&prop(&event, "type"));
    let seq_ok = session_sequence.is_some_and(|n| n.fract() == 0.0 && n >= 0.0);
    let mut seq_obj = Map::new();
    if seq_ok { seq_obj.insert("sequence".into(), json!(session_sequence.unwrap())); }
    let turn_id = {
        let t = prop(&event, "turnId");
        match non_empty_trimmed(&Value::String(str_of(&t))) {
            Some(x) if !is_undef(&t) => json!(x),
            _ => Value::Null,
        }
    };
    let mk = |id: Value, tone: &str, kind: &str, summary: &str, payload: Value, seq: &Map<String, Value>, turn: &Value| {
        let mut a = json!({"id": id, "createdAt": prop(&event, "createdAt"), "tone": tone,
            "kind": kind, "summary": summary, "payload": activity_payload(payload),
            "turnId": turn});
        for (k, v) in seq { a.as_object_mut().unwrap().insert(k.clone(), v.clone()); }
        a
    };
    let payload = prop(&event, "payload");
    let provider = str_of(&prop(&event, "provider"));
    // reasoning shortcut
    let item_type = str_of(&prop(&payload, "itemType"));
    let is_reasoning_event = (((provider == "codex" || provider == "antigravity" || provider == "opencode")
            && ty == "item.completed")
        || (provider == "claudeAgent" && (ty == "item.updated" || ty == "item.completed")))
        && item_type == "reasoning" && !is_undef(&prop(&event, "itemId"));
    if is_reasoning_event {
        let detail = readable_reasoning_detail(s(&prop(&payload, "detail")).map(str::to_string))?;
        if let Some(detail) = detail {
            let item = str_of(&prop(&event, "itemId"));
            let mut data = Map::new();
            if truthy(&prop(&payload, "status")) { data.insert("status".into(), prop(&payload, "status")); }
            data.insert("detail".into(), json!(truncate_detail(&detail, if provider == "claudeAgent" { 8000 } else { 2000 })));
            data.insert("data".into(), json!({"toolCallId": item}));
            return Ok(vec![mk(json!(format!("provider-reasoning:{}:{}", str_of(&prop(&event, "threadId")), item)),
                "tool", "task.progress", "Reasoning trace", Value::Object(data), &seq_obj, &turn_id)]);
        }
    }
    let event_id = prop(&event, "eventId");
    match ty.as_str() {
        "session.configured" => {
            let mut out = vec![];
            let config = prop(&payload, "config");
            let auto = prop(&config, "autoCompactWindow");
            let legacy = prop(&config, "contextWindow");
            let window_v = if !is_undef(&auto) { auto.clone() } else { legacy.clone() };
            let configured = s(&window_v).map(|x| x.trim().to_lowercase());
            let max_tokens = match &window_v {
                Value::Number(n) if n.as_f64().is_some_and(|x| x.is_finite() && x > 0.0) => n.as_f64(),
                _ => match configured.as_deref() { Some("1m") => Some(1_000_000.0), Some("200k") => Some(200_000.0), _ => None },
            };
            let ctx_payload = match max_tokens {
                Some(mt) => {
                    let mut p = Map::new();
                    p.insert("maxTokens".into(), json!(mt));
                    if let Some(cw) = &configured { p.insert("contextWindow".into(), json!(cw)); }
                    Some(Value::Object(p))
                }
                None => {
                    let cleared = (auto.is_null() && (is_undef(&legacy) || legacy.is_null()))
                        || (is_undef(&auto) && legacy.is_null());
                    if cleared { Some(json!({"cleared": true})) } else { None }
                }
            };
            if let Some(p) = ctx_payload {
                out.push(mk(event_id.clone(), "info", "context-window.configured",
                    "Context window configured", p, &seq_obj, &turn_id));
            }
            let fast = s(&prop(&config, "fast_mode_state")).filter(|x| matches!(*x, "on" | "off" | "cooldown"));
            if let Some(state) = fast {
                let mut p = Map::new();
                p.insert("state".into(), json!(state));
                if let Some(r) = s(&prop(&config, "fast_mode_disabled_reason")) {
                    p.insert("disabledReason".into(), json!(r));
                }
                let id = if !out.is_empty() { json!(format!("{}:fast-mode", str_of(&event_id))) } else { event_id.clone() };
                out.push(mk(id, "info", "fast-mode.state", "Fast mode state reported",
                    Value::Object(p), &seq_obj, &turn_id));
            }
            Ok(out)
        }
        "request.opened" | "request.resolved" => {
            if str_of(&prop(&payload, "requestType")) == "tool_user_input" { return Ok(vec![]); }
            let rt = str_of(&prop(&payload, "requestType"));
            let request_kind = match rt.as_str() {
                "command_execution_approval" | "exec_command_approval" => Some("command"),
                "file_read_approval" => Some("file-read"),
                "permissions_approval" => Some("permissions"),
                "tool_approval" | "dynamic_tool_call" => Some("tool"),
                "file_change_approval" | "apply_patch_approval" => Some("file-change"),
                _ => None,
            };
            let mut p = Map::new();
            if let Some(rid) = non_empty_trimmed(&prop(&event, "requestId")) {
                p.insert("requestId".into(), json!(rid));
            }
            if !is_undef(&prop(&event, "lifecycleGeneration")) {
                p.insert("lifecycleGeneration".into(), prop(&event, "lifecycleGeneration"));
            }
            if let Some(rk) = request_kind { p.insert("requestKind".into(), json!(rk)); }
            p.insert("requestType".into(), prop(&payload, "requestType"));
            if ty == "request.opened" && truthy(&prop(&payload, "detail")) {
                p.insert("detail".into(), json!(truncate_detail(&str_of(&prop(&payload, "detail")), 2000)));
            }
            if ty == "request.resolved" && truthy(&prop(&payload, "decision")) {
                p.insert("decision".into(), prop(&payload, "decision"));
            }
            let opened = ty == "request.opened";
            let summary = if !opened { "Approval resolved" } else {
                match request_kind {
                    Some("command") => "Command approval requested",
                    Some("file-read") => "File-read approval requested",
                    Some("file-change") => "File-change approval requested",
                    Some("permissions") => "Permission approval requested",
                    Some("tool") => "Tool approval requested",
                    _ => "Approval requested",
                }
            };
            Ok(vec![mk(event_id, "approval", if opened { "approval.requested" } else { "approval.resolved" },
                summary, Value::Object(p), &seq_obj, &turn_id)])
        }
        "runtime.error" => {
            let message = s(&prop(&payload, "message"));
            let Some(message) = message else { return Ok(vec![]); };
            let mut p = Map::new();
            p.insert("message".into(), json!(truncate_detail(message, 500)));
            if truthy(&prop(&payload, "class")) { p.insert("class".into(), prop(&payload, "class")); }
            if !is_undef(&prop(&payload, "errorCode")) { p.insert("errorCode".into(), prop(&payload, "errorCode")); }
            Ok(vec![mk(event_id, "error", "runtime.error", "Provider runtime error", Value::Object(p), &seq_obj, &turn_id)])
        }
        "runtime.warning" => {
            let message = truncate_detail(&str_of(&prop(&payload, "message")), 2000);
            let mut p = Map::new();
            p.insert("message".into(), json!(message.clone()));
            let summary = if prop(&payload, "willRetry") == Value::Bool(true)
                || prop(&prop(&payload, "detail"), "willRetry") == Value::Bool(true) {
                "Provider retrying"
            } else { "Runtime warning" };
            Ok(vec![mk(event_id, "info", "runtime.warning", summary, Value::Object(p), &seq_obj, &turn_id)])
        }
        _ => Ok(vec![]),
    }
}
