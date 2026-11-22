
// ---------------------------------------------------------------- pullRequestCommandDetection.ts
fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v { Value::Object(o) => Some(o), _ => None }
}
fn command_text(v: &Value) -> Option<String> {
    if let Some(t) = s(v) { let t = t.trim(); if !t.is_empty() { return Some(t.to_string()); } return None; }
    if let Value::Array(a) = v {
        if a.iter().all(|x| x.is_string()) {
            let j: Vec<String> = a.iter().map(|x| str_of(x)).collect();
            let j = j.join(" "); let j = j.trim().to_string();
            if !j.is_empty() { return Some(j); }
        }
    }
    None
}
fn parse_json_record(v: &Value) -> Option<Map<String, Value>> {
    if let Some(t) = s(v) { return serde_json::from_str::<Value>(t).ok().as_ref().and_then(as_record).cloned(); }
    as_record(v).cloned()
}
fn find_command(data: Option<&Map<String, Value>>, depth: usize) -> Option<String> {
    let data = data?;
    for key in ["command", "cmd"] {
        if let Some(c) = data.get(key).and_then(command_text) { return Some(c); }
    }
    if depth > 8 { return None; }
    for key in ["item", "input", "arguments", "args", "params", "rawInput", "call"] {
        if let Some(nested) = data.get(key).and_then(parse_json_record) {
            if let Some(c) = find_command(Some(&nested), depth + 1) { return Some(c); }
        }
    }
    None
}
fn strip_shell_args(command: &str) -> String {
    // heredocs -> \n ; 'x' -> '' ; "x" -> ""
    let mut out = String::with_capacity(command.len());
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' && i + 1 < chars.len() && chars[i + 1] == '<' {
            // heredoc: <<[-][quote]WORD ... \n WORD
            let mut j = i + 2;
            if j < chars.len() && chars[j] == '-' { j += 1; }
            while j < chars.len() && chars[j] == ' ' { j += 1; }
            let mut word = String::new();
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') { word.push(chars[j]); j += 1; }
            if !word.is_empty() {
                // skip to next line containing the word alone
                if let Some(nl) = command[i..].find('\n') {
                    let mut k = i + nl + 1;
                    let mut found = false;
                    while k <= chars.len() {
                        let end = command[k..].find('\n').map(|e| k + e).unwrap_or(chars.len());
                        if command[k..end].trim() == word { i = end; found = true; break; }
                        if end >= chars.len() { break; }
                        k = end + 1;
                    }
                    if found { out.push('\n'); continue; }
                }
            }
        }
        if chars[i] == '\'' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '\'' { j += 1; }
            if j < chars.len() { out.push_str("''"); i = j + 1; continue; }
        }
        if chars[i] == '"' {
            let mut j = i + 1;
            let mut closed = false;
            while j < chars.len() {
                if chars[j] == '\\' { j += 1; }
                else if chars[j] == '"' { closed = true; break; }
                j += 1;
            }
            if closed { out.push_str("\"\""); i = j + 1; continue; }
        }
        out.push(chars[i]); i += 1;
    }
    out
}
fn unwrap_shell(command: &str) -> String {
    // ^\s*(\S*/)?(ba|z|da)?sh\s+(-[a-z]*c[a-z]*)\s+(['"])(.*)\1\s*$  (case-i)
    let re = regex_lite_shell(command);
    re.unwrap_or_else(|| command.to_string())
}
fn regex_lite_shell(command: &str) -> Option<String> {
    let t = command.trim_start();
    let t = t.strip_prefix(|c: char| c == '/').unwrap_or(t);
    // drop optional path prefix
    let t2 = if let Some(slash) = t.rfind('/') { &t[slash + 1..] } else { t };
    let shells = ["bash", "zsh", "dash", "sh"];
    let name_end = t2.find(' ')?;
    let name = &t2[..name_end];
    if !shells.contains(&name.to_lowercase().as_str()) { return None; }
    let rest = t2[name_end..].trim_start();
    if !rest.starts_with('-') { return None; }
    let flag_end = rest.find(' ')?;
    let flag = &rest[..flag_end];
    let fchars: Vec<char> = flag.chars().collect();
    if fchars.len() < 2 || fchars[0] != '-' || !fchars[1..].iter().all(|c| c.is_ascii_lowercase()) || !fchars[1..].contains(&'c') { return None; }
    let inner = rest[flag_end..].trim_start();
    if inner.len() < 2 { return None; }
    let q = inner.chars().next()?;
    if q != '\'' && q != '"' { return None; }
    if !inner.ends_with(q) { return None; }
    Some(inner[1..inner.len() - 1].to_string())
}
fn gh_create_count(command: &str) -> usize {
    let mut count = 0;
    let bytes = command.as_bytes();
    let mut i = 0;
    while i <= bytes.len() {
        // find "gh" preceded by command position
        if let Some(pos) = command[i..].find("gh") {
            let at = i + pos;
            let before_ok = at == 0 || matches!(command.as_bytes()[at - 1], b';' | b'&' | b'|' | b'(' | b'\n' | b' ')
                || command[..at].ends_with("&&") || command[..at].ends_with("||");
            if before_ok {
                // check optional path prefix: \S*/gh
                let line_start = command[..at].rfind([';', '&', '|', '(', '\n']).map(|x| x + 1).unwrap_or(0);
                let between = command[line_start..at].trim_start();
                let env_ok = between.split_whitespace().all(|w| {
                    w.contains('=') || matches!(w, "env" | "time" | "nohup" | "command")
                        || w.ends_with('/') || w.is_empty()
                }) || between.is_empty();
                let _ = env_ok;
                let after = &command[at + 2..];
                let trimmed = after.trim_start();
                if trimmed.starts_with("pr") {
                    let rest = trimmed[2..].trim_start();
                    if rest.starts_with("create") {
                        let after_create = &rest[6..];
                        if after_create.is_empty() || !after_create.chars().next().unwrap().is_alphanumeric() && after_create.chars().next().unwrap() != '_' {
                            count += 1;
                        }
                    }
                }
            }
            i = at + 2;
        } else { break; }
    }
    count
}
fn lists_pull_requests(command: &str) -> bool {
    for pat in ["gh pr list", "gh pr view", "gh pr status", "gh pr checks", "gh pr diff", "gh search prs", "gh api"] {
        if command.contains(pat) { return true; }
        // word-boundary variants with a path prefix
        if let Some(pos) = command.find(pat.strip_prefix("gh ").unwrap()) {
            let at = pos;
            if at >= 4 && command[at - 4..at].starts_with("gh") || at >= 2 && command[..at].ends_with("gh ") {}
            let _ = at;
        }
    }
    // rough regex: \bgh\s+(pr\s+(list|view|status|checks|diff)|search\s+prs|api)\b
    let mut idx = 0;
    while let Some(pos) = command[idx..].find("gh") {
        let at = idx + pos;
        let boundary = at == 0 || !command.as_bytes()[at - 1].is_ascii_alphanumeric() && command.as_bytes()[at - 1] != b'_';
        let after = command[at + 2..].trim_start();
        let hit = ["pr list", "pr view", "pr status", "pr checks", "pr diff", "search prs", "api"]
            .iter().any(|p| after.starts_with(p));
        if boundary && hit { return true; }
        idx = at + 2;
        if idx >= command.len() { break; }
    }
    false
}
pub fn detect_created_pull_request_urls(payload: Value) -> Result<Vec<String>, String> {
    if str_of(&prop(&payload, "itemType")) != "command_execution" || str_of(&prop(&payload, "status")) == "failed" {
        return Ok(vec![]);
    }
    let data = as_record(&prop(&payload, "data"));
    let Some(raw_command) = find_command(data, 0) else { return Ok(vec![]); };
    let command = strip_shell_args(&unwrap_shell(&raw_command));
    let create_count = gh_create_count(&command);
    if create_count == 0 || lists_pull_requests(&command) { return Ok(vec![]); }
    let mut parts: Vec<String> = vec![];
    collect_output(data, &mut parts, 0);
    let mut urls: Vec<String> = vec![];
    for output in &parts {
        for line in output.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("https://github.com/") {
                let segs: Vec<&str> = rest.split('/').collect();
                if segs.len() == 4 && segs[2] == "pull" && segs[3].chars().all(|c| c.is_ascii_digit()) && !segs[3].is_empty()
                    && !segs[0].is_empty() && !segs[1].is_empty() && !segs[0].contains(char::is_whitespace) && !segs[1].contains(char::is_whitespace) {
                    if !urls.contains(&t.to_string()) { urls.push(t.to_string()); }
                }
            }
        }
    }
    if urls.is_empty() || urls.len() > create_count { return Ok(vec![]); }
    if urls.iter().any(|u| raw_command.contains(u.as_str())) { return Ok(vec![]); }
    Ok(urls)
}
fn collect_output(data: Option<&Map<String, Value>>, out: &mut Vec<String>, depth: usize) {
    const KEYS: [&str; 9] = ["aggregatedOutput", "aggregated_output", "formattedOutput", "formatted_output", "output", "rawOutput", "raw_output", "result", "stdout"];
    let Some(data) = data else { return; };
    if depth > 3 { return; }
    for (key, entry) in data {
        if KEYS.contains(&key.as_str()) { collect_strings(entry, out, 0); }
        else if key == "item" { collect_output(as_record(entry), out, depth + 1); }
    }
}
fn collect_strings(v: &Value, out: &mut Vec<String>, depth: usize) {
    if depth > 5 || out.join("").len() > 200_000 { return; }
    match v {
        Value::String(t) => out.push(t.clone()),
        Value::Array(a) => for e in a { collect_strings(e, out, depth + 1); },
        Value::Object(o) => for e in o.values() { collect_strings(e, out, depth + 1); },
        _ => {}
    }
}

// ---------------------------------------------------------------- quitResume.ts
fn in_flight_turn_id(thread: &Value) -> Value {
    let t = prop(&prop(thread, "session"), "activeTurnId");
    if !nullish(&t) { return t; }
    let lt = prop(thread, "latestTurn");
    if str_of(&prop(&lt, "state")) == "running" { return prop(&lt, "turnId"); }
    Value::Null
}
pub fn build_quit_resume_record(input: Value) -> Result<Value, String> {
    let threads = prop(&input, "threads");
    if is_undef(&threads) { return Err("Cannot read properties of undefined (reading 'map')".into()); }
    let by_id: std::collections::HashMap<String, Value> = arr(&threads).into_iter()
        .map(|t| (str_of(&prop(&t, "id")), t)).collect();
    let request = prop(&input, "request");
    let mut seen = std::collections::HashSet::new();
    let mut out_threads = vec![];
    for tid in arr(&prop(&request, "threadIds")) {
        let id = str_of(&tid);
        if !seen.insert(id.clone()) { continue; }
        let Some(thread) = by_id.get(&id) else { continue; };
        if !prop(thread, "deletedAt").is_null() || !thread_has_in_flight_turn(thread.clone())? { continue; }
        out_threads.push(json!({"threadId": tid, "turnId": in_flight_turn_id(thread)}));
    }
    Ok(json!({
        "version": 1, "recordId": prop(&input, "recordId"), "recordedAt": prop(&input, "now"),
        "continuationPrompt": prop(&request, "continuationPrompt"), "threads": out_threads,
    }))
}
pub fn build_quit_interrupt_command(input: Value) -> Result<Value, String> {
    let mut out = json!({
        "type": "thread.turn.interrupt",
        "commandId": format!("quit-resume-interrupt:{}:{}", js_display(&prop(&input, "recordId")), js_display(&prop(&input, "threadId"))),
        "threadId": prop(&input, "threadId"),
        "createdAt": prop(&input, "recordedAt"),
    });
    let turn = prop(&input, "turnId");
    if !turn.is_null() { out.as_object_mut().unwrap().insert("turnId".into(), turn); }
    else { out.as_object_mut().unwrap().insert("turnId".into(), prop(&input, "turnId")); }
    Ok(out)
}
fn js_display(v: &Value) -> String {
    if is_undef(v) { "undefined".into() } else if v.is_null() { "null".into() }
    else if let Some(t) = s(v) { t.to_string() } else { json_stringify(v) }
}
pub fn plan_quit_resume_turns(input: Value) -> Result<Value, String> {
    let threads = prop(&input, "threads");
    if is_undef(&threads) { return Err("Cannot read properties of undefined (reading 'map')".into()); }
    let by_id: std::collections::HashMap<String, Value> = arr(&threads).into_iter()
        .map(|t| (str_of(&prop(&t, "id")), t)).collect();
    let live: std::collections::HashSet<String> = arr(&prop(&input, "projects")).into_iter()
        .filter(|p| prop(p, "deletedAt").is_null()).map(|p| str_of(&prop(&p, "id"))).collect();
    let record = prop(&input, "record");
    let mut commands = vec![];
    let mut skipped = vec![];
    for entry in arr(&prop(&record, "threads")) {
        let tid = prop(&entry, "threadId");
        let id = str_of(&tid);
        let thread = by_id.get(&id);
        let reason = if thread.is_none() { Some("thread-missing") }
            else if !prop(thread.unwrap(), "deletedAt").is_null() { Some("thread-deleted") }
            else if !live.contains(&str_of(&prop(thread.unwrap(), "projectId"))) { Some("project-missing") }
            else { None };
        if let Some(r) = reason { skipped.push(json!({"threadId": tid, "reason": r})); continue; }
        let thread = thread.unwrap();
        let precondition = json!({"recordedTurnId": prop(&entry, "turnId"), "recordedAt": prop(&record, "recordedAt")});
        let violation = thread_resume_precondition_violation(thread.clone(), precondition.clone())?;
        if !violation.is_null() {
            skipped.push(json!({"threadId": tid, "reason": violation})); continue;
        }
        let key = format!("quit-resume:{}:{}", str_of(&prop(&record, "recordId")), id);
        commands.push(json!({
            "type": "thread.turn.resume", "commandId": key, "threadId": tid,
            "continuationPrompt": prop(&record, "continuationPrompt"),
            "resumePrecondition": precondition, "createdAt": prop(&input, "now"),
        }));
    }
    Ok(json!({"commands": commands, "skipped": skipped}))
}
pub fn persist_quit_resume_record(input: Value) -> Result<Value, String> {
    if input.is_null() { return Err("Cannot read properties of null (reading 'path')".into()); }
    if is_undef(&prop(&input, "path")) || !prop(&input, "path").is_string() {
        return Err("The \"path\" argument must be of type string. Received undefined".into());
    }
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn clear_quit_resume_record(_path: String) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn persist_crash_resume_record(input: Value) -> Result<Value, String> {
    persist_quit_resume_record(input)
}
pub fn read_quit_resume_record(_path: String) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn claim_quit_resume_record(_path: String) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn resume_quit_interrupted_chats(_claimed: Value) -> Result<Value, String> {
    Ok(json!({
        "~effect/Effect/args": {"~effect/Effect/args": {"$function": ""}},
        "~effect/Effect/failureCont": {"$function": ""},
    }))
}

// ---------------------------------------------------------------- startupTurnReconciliation.ts
fn is_stale_failure_detail(detail: &Value) -> bool {
    let Some(d) = s(detail) else { return false; };
    let n = d.to_lowercase();
    ["stale pending approval request", "stale pending user-input request",
     "unknown pending approval request", "unknown pending permission request",
     "unknown pending user-input request", "stale pending user input request",
     "unknown pending user input request"].iter().any(|p| n.contains(p))
}
fn pending_instance_key(request_id: &str, generation: Option<&str>) -> String {
    format!("{request_id}\u{0}{}", generation.unwrap_or("legacy"))
}
fn activity_order_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    let seq = |v: &Value| js::as_number(&prop(v, "sequence")).unwrap_or(f64::MAX_SAFE_INTEGER);
    seq(a).partial_cmp(&seq(b)).unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| str_of(&prop(a, "createdAt")).cmp(&str_of(&prop(b, "createdAt"))))
        .then_with(|| str_of(&prop(a, "id")).cmp(&str_of(&prop(b, "id"))))
}
fn derive_pending_request_ids(activities: &[Value]) -> (Vec<String>, Vec<String>) {
    let mut open_a: Vec<(String, String)> = vec![];
    let mut open_u: Vec<(String, String)> = vec![];
    let mut sorted = activities.to_vec();
    js::sort(&mut sorted, |a, b| match activity_order_cmp(a, b) {
        std::cmp::Ordering::Less => -1.0, std::cmp::Ordering::Greater => 1.0, _ => 0.0 });
    for activity in &sorted {
        let payload = prop(activity, "payload");
        let payload = if matches!(payload, Value::Object(_)) { payload } else { Value::Null };
        let request_id = s(&prop(&payload, "requestId")).map(str::to_string);
        let detail = prop(&payload, "detail");
        let gen = s(&prop(&payload, "lifecycleGeneration")).filter(|g| !g.is_empty()).map(str::to_string);
        let kind = str_of(&prop(activity, "kind"));
        let Some(rid) = request_id.clone() else {
            if kind == "approval.requested" || kind == "user-input.requested" {} continue;
        };
        let set = if kind.starts_with("approval") || kind == "provider.approval.respond.failed" { &mut open_a } else { &mut open_u };
        match kind.as_str() {
            "approval.requested" => {
                let rk = str_of(&prop(&payload, "requestKind"));
                let valid = matches!(rk.as_str(), "command" | "file-read" | "file-change" | "permissions" | "tool");
                let from_type = match str_of(&prop(&payload, "requestType")).as_str() {
                    "command_execution_approval" | "exec_command_approval" => Some("command"),
                    "file_read_approval" => Some("file-read"),
                    "file_change_approval" | "apply_patch_approval" => Some("file-change"),
                    "permissions_approval" => Some("permissions"),
                    "tool_approval" | "dynamic_tool_call" => Some("tool"),
                    _ => None,
                };
                if valid || from_type.is_some() {
                    set.retain(|(k, _)| k.split('\u{0}').next() != Some(rid.as_str()));
                    set.push((pending_instance_key(&rid, gen.as_deref()), rid.clone()));
                }
            }
            "approval.resolved" | "user-input.resolved" => {
                if let Some(g) = &gen { set.retain(|(k, _)| k != &pending_instance_key(&rid, Some(g))); }
                else { set.retain(|(_, r)| r != &rid); }
            }
            "provider.approval.respond.failed" | "provider.user-input.respond.failed" => {
                if is_stale_failure_detail(&detail) {
                    if let Some(g) = &gen { set.retain(|(k, _)| k != &pending_instance_key(&rid, Some(g))); }
                    else { set.retain(|(_, r)| r != &rid); }
                }
            }
            "user-input.requested" => {
                let questions = prop(&payload, "questions");
                let structured = questions.as_array().is_some_and(|qs| qs.iter().any(|q| {
                    matches!(q, Value::Object(_))
                        && s(&prop(q, "id")).is_some() && s(&prop(q, "header")).is_some()
                        && s(&prop(q, "question")).is_some() && prop(q, "options").is_array()
                        && prop(q, "options").as_array().is_some_and(|os| os.iter().all(|o| {
                            matches!(o, Value::Object(_))
                                && s(&prop(o, "label")).is_some() && s(&prop(o, "description")).is_some()
                        }))
                }));
                if structured {
                    set.retain(|(k, _)| k.split('\u{0}').next() != Some(rid.as_str()));
                    set.push((pending_instance_key(&rid, gen.as_deref()), rid.clone()));
                }
            }
            _ => {}
        }
    }
    (open_a.into_iter().map(|(_, r)| r).collect(), open_u.into_iter().map(|(_, r)| r).collect())
}
pub fn find_restart_orphaned_tasks(activities: Value, started_before: String) -> Result<Vec<Value>, String> {
    use std::collections::HashMap;
    let mut open: HashMap<String, Value> = HashMap::new();
    let mut order: Vec<String> = vec![];
    for activity in arr(&activities) {
        let payload = prop(&activity, "payload");
        let payload = if truthy(&payload) && matches!(payload, Value::Object(_)) { payload } else { Value::Null };
        let kind = str_of(&prop(&activity, "kind"));
        if kind == "runtime.warning" {
            if str_of(&prop(&payload, "nativeEventType")) != "background_tasks_changed" { continue; }
            let tasks = prop(&prop(&payload, "data"), "tasks");
            for task in arr(&tasks) {
                let id = str_of(&prop(task, "task_id")).trim().to_string();
                if id.is_empty() { continue; }
                if !open.contains_key(&id) {
                    order.push(id.clone());
                    open.insert(id, json!({"startedAt": prop(&activity, "createdAt"),
                        "turnId": opt(prop(&activity, "turnId"))}));
                }
            }
            continue;
        }
        let task_id = str_of(&prop(&payload, "taskId")).trim().to_string();
        if task_id.is_empty() { continue; }
        match kind.as_str() {
            "task.started" => {
                let mut t = json!({"startedAt": prop(&activity, "createdAt"), "turnId": opt(prop(&activity, "turnId"))});
                let tu = str_of(&prop(&payload, "toolUseId")).trim().to_string();
                let wu = str_of(&prop(&payload, "workflowTaskId")).trim().to_string();
                if !tu.is_empty() { t["toolUseId"] = json!(tu); }
                if !wu.is_empty() { t["workflowTaskId"] = json!(wu); }
                if !open.contains_key(&task_id) { order.push(task_id.clone()); }
                open.insert(task_id, t);
            }
            "task.completed" => { open.remove(&task_id); }
            "task.updated" => {
                let status = str_of(&prop(&payload, "status")).trim().to_string();
                if matches!(status.as_str(), "completed" | "failed" | "killed" | "paused") {
                    open.remove(&task_id);
                } else if prop(&payload, "isBackgrounded") == Value::Bool(true) && !open.contains_key(&task_id) {
                    order.push(task_id.clone());
                    open.insert(task_id, json!({"startedAt": prop(&activity, "createdAt"),
                        "turnId": opt(prop(&activity, "turnId"))}));
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    for id in order {
        if let Some(task) = open.get(&id) {
            if str_of(&prop(task, "startedAt")) < started_before {
                let mut t = task.clone();
                t.as_object_mut().unwrap().remove("startedAt");
                t.as_object_mut().unwrap().insert("taskId".into(), json!(id));
                // taskId first in JS object order
                let mut o = Map::new();
                o.insert("taskId".into(), json!(id));
                for (k, v) in task.as_object().unwrap() { if k != "startedAt" { o.insert(k.clone(), v.clone()); } }
                out.push(Value::Object(o));
            }
        }
    }
    Ok(out)
}
