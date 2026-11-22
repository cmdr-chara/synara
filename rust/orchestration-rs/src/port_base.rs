//! The Rust port of `apps/server/src/orchestration/` — pure projection/decision helpers.
//!
//! Values cross the bridge as JSON with markers (`{"$undefined":true}`, `{"$num":..}`).

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

fn undef() -> Value {
    js::undefined()
}
fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}
fn nullish(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}
fn s(v: &Value) -> Option<&str> {
    v.as_str()
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(t) => !t.is_empty(),
        _ => !is_undef(v),
    }
}
/// JS property read: `v[key]` — undefined member when absent/non-object.
fn prop<'v>(v: &'v Value, k: &str) -> Value {
    match v {
        Value::Object(o) => o.get(k).cloned().unwrap_or_else(undef),
        _ => undef(),
    }
}
/// `a === b` over decoded values (markers compare by their revived shape).
fn eq(a: &Value, b: &Value) -> bool {
    a == b
}
fn str_of(v: &Value) -> String {
    s(v).unwrap_or_default().to_string()
}
fn num(v: &Value) -> f64 {
    js::as_number(v).unwrap_or(f64::NAN)
}
fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}
fn is_arr(v: &Value) -> bool {
    v.is_array()
}
fn opt_string(v: &Value) -> Option<String> {
    s(v).map(str::to_string)
}
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.cmp(b)
}
fn invariant_err(command_type: &Value, detail: &str) -> String {
    format!(
        "Orchestration command invariant failed ({}): {}",
        str_of(command_type),
        detail
    )
}

// ---------------------------------------------------------------- Errors.ts

/// `threadModelSelectionChangedDetail`.
pub fn thread_model_selection_changed_detail(thread_id: String) -> Result<String, String> {
    Ok(format!(
        "Thread '{thread_id}' model selection changed before the conditional update."
    ))
}

/// `toProjectorDecodeError` — returns a TaggedError shape the callers serialize.
pub fn to_projector_decode_error(event_type: String) -> Result<Value, String> {
    Ok(json!({
        "_tag": "OrchestrationProjectorDecodeError",
        "eventType": event_type,
        "issue": "",
    }))
}

// ---------------------------------------------------------------- commandFingerprint.ts

fn canonicalize(v: &Value) -> Value {
    match v {
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        Value::Object(o) => {
            if is_undef(v) {
                return v.clone();
            }
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                let x = &o[k];
                if is_undef(x) {
                    continue;
                }
                out.insert(k.clone(), canonicalize(x));
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}

fn json_stringify(v: &Value) -> String {
    let mut out = String::new();
    write_json(v, &mut out);
    out
}

fn esc(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&js::number_to_string(n.as_f64().unwrap_or(f64::NAN))),
        Value::String(t) => esc(t, out),
        Value::Array(items) => {
            out.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if is_undef(x) {
                    out.push_str("null");
                } else {
                    write_json(x, out);
                }
            }
            out.push(']');
        }
        Value::Object(o) => {
            if is_undef(v) {
                out.push_str("null");
            } else if let Some(x) = o.get("$num").and_then(js::as_number) {
                let _ = x;
                out.push_str("null");
            } else if let Some(units) = o.get("$utf16").and_then(Value::as_array) {
                let u: Vec<u16> = units
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u16)
                    .collect();
                esc(&String::from_utf16_lossy(&u), out);
            } else {
                out.push('{');
                let mut first = true;
                for (k, x) in o {
                    if is_undef(x) {
                        continue;
                    }
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    esc(k, out);
                    out.push(':');
                    write_json(x, out);
                }
                out.push('}');
            }
        }
    }
}

/// `fingerprintOrchestrationCommand` — sha256 of the canonical intent JSON.
pub fn fingerprint_orchestration_command(command: Value) -> Result<Value, String> {
    let mut intent = match &command {
        Value::Object(o) => {
            let mut m = o.clone();
            m.remove("commandId");
            Value::Object(m)
        }
        _ => command.clone(),
    };
    if str_of(&prop(&intent, "type")) == "thread.turn.start" {
        let message = prop(&intent, "message");
        let attachments = arr(&prop(&message, "attachments"));
        let mapped: Vec<Value> = attachments
            .iter()
            .map(|a| match str_of(&prop(a, "type")).as_str() {
                "assistant-selection" => json!({
                    "type": prop(a, "type"),
                    "assistantMessageId": prop(a, "assistantMessageId"),
                    "text": prop(a, "text"),
                }),
                _ => json!({ "type": prop(a, "type"), "id": prop(a, "id") }),
            })
            .collect();
        if let Value::Object(ref mut m) = intent {
            let mut msg = match message {
                Value::Object(o) => o,
                _ => Map::new(),
            };
            msg.insert("attachments".into(), json!(mapped));
            m.insert("message".into(), Value::Object(msg));
        }
    }
    let canonical = json_stringify(&canonicalize(&json!({
        "version": 1,
        "command": intent,
    })));
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(canonical.as_bytes());
    Ok(json!({ "version": 1, "value": hex::encode(h.finalize()) }))
}

// ---------------------------------------------------------------- commandInvariants.ts

/// `threadHasInFlightTurn`.
pub fn thread_has_in_flight_turn(thread: Value) -> Result<bool, String> {
    let session = prop(&thread, "session");
    let status = prop(&session, "status");
    let active = prop(&session, "activeTurnId");
    let latest_state = prop(&prop(&thread, "latestTurn"), "state");
    Ok((str_of(&status) != "error" && !nullish(&active))
        || str_of(&status) == "starting"
        || str_of(&status) == "running"
        || str_of(&latest_state) == "running")
}

/// `checkpointRevertActiveTurnDetail`.
pub fn checkpoint_revert_active_turn_detail(thread_id: Value) -> Result<String, String> {
    Ok(format!(
        "Thread '{}' has an active turn. Interrupt the current turn before reverting checkpoints.",
        str_of(&thread_id)
    ))
}

const REVERT_KINDS: [&str; 3] = [
    "checkpoint.revert.started",
    "checkpoint.revert.succeeded",
    "checkpoint.revert.failed",
];

/// `threadHasCheckpointRevertInProgress`.
pub fn thread_has_checkpoint_revert_in_progress(thread: Value) -> Result<bool, String> {
    let mut lifecycle: Vec<Value> = arr(&prop(&thread, "activities"))
        .into_iter()
        .filter(|a| REVERT_KINDS.contains(&str_of(&prop(a, "kind")).as_str()))
        .collect();
    js::sort(&mut lifecycle, |a, b| {
        let seq = |v: &Value| js::as_number(&prop(v, "sequence")).unwrap_or(-1.0);
        let d = seq(b) - seq(a);
        if d != 0.0 {
            return d;
        }
        let c = locale_cmp(
            &str_of(&prop(a, "createdAt")),
            &str_of(&prop(b, "createdAt")),
        );
        match c {
            std::cmp::Ordering::Less => 1.0,
            std::cmp::Ordering::Greater => -1.0,
            _ => match locale_cmp(&str_of(&prop(a, "id")), &str_of(&prop(b, "id"))) {
                std::cmp::Ordering::Less => 1.0,
                std::cmp::Ordering::Greater => -1.0,
                _ => 0.0,
            },
        }
    });
    Ok(lifecycle
        .first()
        .is_some_and(|a| str_of(&prop(a, "kind")) == "checkpoint.revert.started"))
}

/// `checkpointRevertInProgressDetail`.
pub fn checkpoint_revert_in_progress_detail(thread_id: Value) -> Result<String, String> {
    Ok(format!(
        "Thread '{}' has a checkpoint revert in progress. Wait for it to finish before starting a turn.",
        str_of(&thread_id)
    ))
}

/// `checkpointRevertDeleteInProgressDetail`.
pub fn checkpoint_revert_delete_in_progress_detail(thread_id: Value) -> Result<String, String> {
    Ok(format!(
        "Thread '{}' has a checkpoint revert in progress. Wait for it to finish before deleting the thread.",
        str_of(&thread_id)
    ))
}

fn find_by_id(list: &Value, id: &Value) -> Value {
    arr(list)
        .into_iter()
        .find(|x| eq(&prop(x, "id"), id))
        .unwrap_or_else(undef)
}

/// `findThreadById`.
pub fn find_thread_by_id(read_model: Value, thread_id: Value) -> Result<Value, String> {
    Ok(find_by_id(&prop(&read_model, "threads"), &thread_id))
}

/// `findProjectById`.
pub fn find_project_by_id(read_model: Value, project_id: Value) -> Result<Value, String> {
    Ok(find_by_id(&prop(&read_model, "projects"), &project_id))
}

/// `findSpaceById`.
pub fn find_space_by_id(read_model: Value, space_id: Value) -> Result<Value, String> {
    Ok(find_by_id(&prop(&read_model, "spaces"), &space_id))
}

/// `listActiveSpaces` — deletedAt === null, sorted by sortOrder then id.
pub fn list_active_spaces(read_model: Value) -> Result<Vec<Value>, String> {
    let mut spaces: Vec<Value> = arr(&prop(&read_model, "spaces"))
        .into_iter()
        .filter(|sp| prop(sp, "deletedAt").is_null())
        .collect();
    js::sort(&mut spaces, |a, b| {
        let d = num(&prop(a, "sortOrder")) - num(&prop(b, "sortOrder"));
        if d != 0.0 {
            return d;
        }
        match locale_cmp(&str_of(&prop(a, "id")), &str_of(&prop(b, "id"))) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Greater => 1.0,
            _ => 0.0,
        }
    });
    Ok(spaces)
}

/// `requireSpace`.
pub fn require_space(input: Value) -> Result<Value, String> {
    let space = find_space_by_id(prop(&input, "readModel"), prop(&input, "spaceId"));
    let ty = prop(&prop(&input, "command"), "type");
    let sid = str_of(&prop(&input, "spaceId"));
    if !is_undef(&space) && prop(&space, "deletedAt").is_null() {
        return Ok(space);
    }
    Err(invariant_err(
        &ty,
        &if !is_undef(&space) {
            format!(
                "Space '{sid}' was deleted and cannot handle command '{}'.",
                str_of(&ty)
            )
        } else {
            format!(
                "Space '{sid}' does not exist for command '{}'.",
                str_of(&ty)
            )
        },
    ))
}

/// `requireSpaceAbsent`.
pub fn require_space_absent(input: Value) -> Result<Value, String> {
    let space = find_space_by_id(prop(&input, "readModel"), prop(&input, "spaceId"));
    let ty = prop(&prop(&input, "command"), "type");
    if is_undef(&space) {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Space '{}' already exists and cannot be created twice.",
            str_of(&prop(&input, "spaceId"))
        ),
    ))
}

/// `requireSpaceNameAvailable`.
pub fn require_space_name_available(input: Value) -> Result<Value, String> {
    let ty = prop(&prop(&input, "command"), "type");
    let name = str_of(&prop(&input, "name"));
    let normalized = name.trim().to_lowercase();
    if normalized == "void" {
        return Err(invariant_err(
            &ty,
            "'Void' is reserved for unassigned projects.",
        ));
    }
    let exclude = prop(&input, "excludeSpaceId");
    let conflict = arr(&prop(&prop(&input, "readModel"), "spaces"))
        .into_iter()
        .find(|sp| {
            prop(sp, "deletedAt").is_null()
                && !eq(&prop(sp, "id"), &exclude)
                && str_of(&prop(sp, "name")).trim().to_lowercase() == normalized
        });
    match conflict {
        None => Ok(undef()),
        Some(_) => Err(invariant_err(
            &ty,
            &format!("A space named '{name}' already exists."),
        )),
    }
}

// ---- shared workspace-root helpers (threadWorkspace.ts)

fn likely_windows_root(value: &str, platform: &str) -> bool {
    if platform == "win32" {
        return true;
    }
    if !platform.is_empty() && platform != "win32" {
        return false;
    }
    value.len() >= 2
        && value.as_bytes()[1] == b':'
        && value.as_bytes()[0].is_ascii_alphabetic()
        && (value.len() == 2 || matches!(value.as_bytes()[2], b'/' | b'\\'))
        || value.starts_with("\\\\")
        || value.starts_with("//")
}

fn normalize_root(value: &str, platform: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd: String = trimmed
        .chars()
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    let is_drive_root = fwd.len() == 3 && fwd.as_bytes()[1] == b':' && fwd.ends_with('/');
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = fwd[prefix.len()..].to_string();
    while body.contains("//") {
        body = body.replace("//", "/");
    }
    while body.ends_with('/') && !body.is_empty() {
        body.pop();
    }
    let mut final_value = if is_drive_root {
        format!("{}/", &fwd[..2])
    } else if !body.is_empty() {
        format!("{prefix}{body}")
    } else {
        prefix.to_string()
    };
    if platform == "darwin"
        && (final_value.starts_with("/private/var/") || final_value.starts_with("/private/tmp/"))
    {
        final_value = final_value["/private".len()..].to_string();
    }
    if likely_windows_root(trimmed, platform) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

fn roots_equal(a: &str, b: &str, platform: &str) -> bool {
    normalize_root(a, platform) == normalize_root(b, platform)
}

fn root_within(candidate: &str, ancestor: &str, platform: &str) -> bool {
    let c = normalize_root(candidate, platform);
    let a = normalize_root(ancestor, platform);
    if c.is_empty() || a.is_empty() {
        return false;
    }
    if c == a {
        return true;
    }
    let prefix = if a.ends_with('/') { a } else { format!("{a}/") };
    c.starts_with(&prefix)
}

// ---- projectContainers.ts

fn chat_container_root(paths: &Value) -> Option<String> {
    let chat = str_of(&prop(paths, "chatWorkspaceRoot"));
    if !chat.trim().is_empty() {
        return Some(chat.trim().to_string());
    }
    let home = str_of(&prop(paths, "homeDir"));
    if !home.trim().is_empty() {
        return Some(home.trim().to_string());
    }
    None
}

fn matches_legacy_home_root(root: &str, paths: &Value, platform: &str) -> bool {
    let home = str_of(&prop(paths, "homeDir"));
    let home = home.trim();
    let Some(chat_root) = chat_container_root(paths) else {
        return false;
    };
    if home.is_empty() || chat_root.is_empty() {
        return false;
    }
    roots_equal(root, &chat_root, platform) || roots_equal(root, home, platform)
}

fn is_legacy_home_row(title: &str, root: &str, paths: &Value, platform: &str) -> bool {
    title == "Home" && matches_legacy_home_root(root, paths, platform)
}

fn workspace_paths_of(input: &Value) -> Value {
    let wp = prop(input, "workspacePaths");
    json!({
        "homeDir": if is_undef(&prop(&wp, "homeDir")) { Value::Null } else { prop(&wp, "homeDir") },
        "chatWorkspaceRoot": if is_undef(&prop(&wp, "chatWorkspaceRoot")) { Value::Null } else { prop(&wp, "chatWorkspaceRoot") },
    })
}

const PLATFORM: &str = std::env::consts::OS;

fn platform_name() -> &'static str {
    match PLATFORM {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

/// `isLegacyHomeChatContainerRow`.
pub fn is_legacy_home_chat_container_row(input: Value) -> Result<bool, String> {
    Ok(is_legacy_home_row(
        &str_of(&prop(&input, "projectTitle")),
        &str_of(&prop(&input, "projectWorkspaceRoot")),
        &workspace_paths_of(&input),
        platform_name(),
    ))
}

fn is_group_container_kind(kind: &Value) -> bool {
    let k = str_of(kind);
    k == "group" || k == "studio"
}

/// `isOrdinaryProjectRow`.
pub fn is_ordinary_project_row(input: Value) -> Result<bool, String> {
    if is_group_container_kind(&prop(&input, "projectKind")) {
        return Ok(false);
    }
    let kind = prop(&input, "projectKind");
    let kind = if is_undef(&kind) || kind.is_null() {
        "project".to_string()
    } else {
        str_of(&kind)
    };
    Ok(kind == "project"
        && !is_legacy_home_row(
            &str_of(&prop(&input, "projectTitle")),
            &str_of(&prop(&input, "projectWorkspaceRoot")),
            &workspace_paths_of(&input),
            platform_name(),
        ))
}

/// `requireSpaceAssignableProject`.
pub fn require_space_assignable_project(input: Value) -> Result<Value, String> {
    let legacy = is_legacy_home_row(
        &str_of(&prop(&input, "projectTitle")),
        &str_of(&prop(&input, "projectWorkspaceRoot")),
        &workspace_paths_of(&input),
        platform_name(),
    );
    if !legacy {
        return Ok(undef());
    }
    Err(invariant_err(
        &prop(&prop(&input, "command"), "type"),
        "The Chats container is reachable from every space and cannot be assigned to one.",
    ))
}

/// `listActiveProjectsByWorkspaceRoot`.
pub fn list_active_projects_by_workspace_root(
    read_model: Value,
    workspace_root: String,
    options: Value,
) -> Result<Vec<Value>, String> {
    let normalized = normalize_root(&workspace_root, platform_name());
    let kinds: Vec<String> = match &options {
        Value::Object(o) if o.contains_key("kinds") => {
            arr(&prop(&options, "kinds")).iter().map(str_of).collect()
        }
        _ => vec!["project".into()],
    };
    let kinds = if kinds.is_empty() {
        match marker_set(&prop(&options, "kinds")) {
            Some(set) => set.iter().map(str_of).collect(),
            None => vec!["project".into()],
        }
    } else {
        kinds
    };
    Ok(arr(&prop(&read_model, "projects"))
        .into_iter()
        .filter(|p| {
            prop(p, "deletedAt").is_null()
                && kinds.contains(&{
                    let k = prop(p, "kind");
                    if nullish(&k) {
                        "project".to_string()
                    } else {
                        str_of(&k)
                    }
                })
                && normalize_root(&str_of(&prop(p, "workspaceRoot")), platform_name()) == normalized
        })
        .collect())
}

fn marker_set(v: &Value) -> Option<Vec<Value>> {
    v.as_object()?.get("$set")?.as_array().cloned()
}

/// `listThreadsByProjectId`.
pub fn list_threads_by_project_id(
    read_model: Value,
    project_id: Value,
) -> Result<Vec<Value>, String> {
    Ok(arr(&prop(&read_model, "threads"))
        .into_iter()
        .filter(|t| eq(&prop(t, "projectId"), &project_id))
        .collect())
}

/// `requireProject`.
pub fn require_project(input: Value) -> Result<Value, String> {
    let project = find_project_by_id(prop(&input, "readModel"), prop(&input, "projectId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if !is_undef(&project) {
        return Ok(project);
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Project '{}' does not exist for command '{}'.",
            str_of(&prop(&input, "projectId")),
            str_of(&ty)
        ),
    ))
}

/// `requireProjectAbsent`.
pub fn require_project_absent(input: Value) -> Result<Value, String> {
    let project = find_project_by_id(prop(&input, "readModel"), prop(&input, "projectId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if is_undef(&project) {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Project '{}' already exists and cannot be created twice.",
            str_of(&prop(&input, "projectId"))
        ),
    ))
}

fn absolute_folder(path: &str) -> bool {
    let t = path.trim();
    t.starts_with('/')
        || (t.len() >= 3
            && t.as_bytes()[1] == b':'
            && matches!(t.as_bytes()[2], b':' | b'/' | b'\\')
            && t.as_bytes()[0].is_ascii_alphabetic())
        || t.starts_with("\\\\")
}

fn folder_name(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .filter(|s| !s.is_empty())
        .last()
        .unwrap_or(path)
        .to_string()
}

/// `findProjectFolderProblem` (projectFolders.ts).
fn folder_problem(paths: &[String], platform: &str) -> Option<String> {
    if paths.is_empty() {
        return Some("Add at least one folder.".into());
    }
    for p in paths {
        if !absolute_folder(p) {
            return Some(format!("Use an absolute path: {p}"));
        }
    }
    for outer in 0..paths.len() {
        for inner in outer + 1..paths.len() {
            let (l, r) = (&paths[outer], &paths[inner]);
            if normalize_root(l, platform) == normalize_root(r, platform) {
                return Some(format!("{} is already in this project.", folder_name(r)));
            }
            if root_within(r, l, platform) {
                return Some(format!(
                    "{} is inside {}. Add only one of them.",
                    folder_name(r),
                    folder_name(l)
                ));
            }
            if root_within(l, r, platform) {
                return Some(format!(
                    "{} is inside {}. Add only one of them.",
                    folder_name(l),
                    folder_name(r)
                ));
            }
        }
    }
    None
}

/// `requireValidProjectFolders`.
pub fn require_valid_project_folders(input: Value) -> Result<Value, String> {
    let ty = prop(&prop(&input, "command"), "type");
    let additional = arr(&prop(&input, "additionalFolders"));
    if additional.is_empty() {
        return Ok(undef());
    }
    if str_of(&prop(&input, "kind")) != "project" {
        return Err(invariant_err(
            &ty,
            "Only ordinary projects can have additional folders.",
        ));
    }
    let mut paths = vec![str_of(&prop(&input, "workspaceRoot"))];
    paths.extend(additional.iter().map(str_of));
    match folder_problem(&paths, platform_name()) {
        None => Ok(undef()),
        Some(problem) => Err(invariant_err(&ty, &problem)),
    }
}

/// `requireProjectWorkspaceRootAvailable`.
pub fn require_project_workspace_root_available(input: Value) -> Result<Value, String> {
    let kinds_v = prop(&input, "kinds");
    let options = if is_undef(&kinds_v) {
        Value::Null
    } else {
        json!({"kinds": kinds_v})
    };
    let existing = list_active_projects_by_workspace_root(
        prop(&input, "readModel"),
        str_of(&prop(&input, "workspaceRoot")),
        options,
    )?
    .into_iter()
    .find(|p| !eq(&prop(p, "id"), &prop(&input, "excludeProjectId")));
    let ty = prop(&prop(&input, "command"), "type");
    match existing {
        None => Ok(undef()),
        Some(p) => Err(invariant_err(
            &ty,
            &format!(
                "Project '{}' already uses workspace root '{}'.",
                str_of(&prop(&p, "id")),
                str_of(&prop(&p, "workspaceRoot"))
            ),
        )),
    }
}

/// `requireProjectHasNoThreads`.
pub fn require_project_has_no_threads(input: Value) -> Result<Value, String> {
    let remaining: Vec<Value> =
        list_threads_by_project_id(prop(&input, "readModel"), prop(&input, "projectId"))?
            .into_iter()
            .filter(|t| prop(t, "deletedAt").is_null())
            .collect();
    let ty = prop(&prop(&input, "command"), "type");
    if remaining.is_empty() {
        return Ok(undef());
    }
    let n = remaining.len();
    Err(invariant_err(
        &ty,
        &format!(
            "Project '{}' still has {n} thread{} and cannot be deleted.",
            str_of(&prop(&input, "projectId")),
            if n == 1 { "" } else { "s" }
        ),
    ))
}

/// `requireThread`.
pub fn require_thread(input: Value) -> Result<Value, String> {
    let thread = find_thread_by_id(prop(&input, "readModel"), prop(&input, "threadId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if !is_undef(&thread) && prop(&thread, "deletedAt").is_null() {
        return Ok(thread);
    }
    let tid = str_of(&prop(&input, "threadId"));
    Err(invariant_err(
        &ty,
        &if !is_undef(&thread) {
            format!(
                "Thread '{tid}' was deleted and cannot handle command '{}'.",
                str_of(&ty)
            )
        } else {
            format!(
                "Thread '{tid}' does not exist for command '{}'.",
                str_of(&ty)
            )
        },
    ))
}

fn pending_awaiting_response(status: &Value) -> bool {
    let st = str_of(status);
    st == "pending" || st == "retryable"
}

/// `requireApprovalNotResponded`.
pub fn require_approval_not_responded(input: Value) -> Result<Value, String> {
    let thread = find_thread_by_id(prop(&input, "readModel"), prop(&input, "threadId"))?;
    let request_id = prop(&input, "requestId");
    let generation = prop(&input, "lifecycleGeneration");
    let interaction = arr(&prop(&thread, "pendingInteractions"))
        .into_iter()
        .find(|e| {
            str_of(&prop(e, "interactionKind")) == "approval"
                && eq(&prop(e, "requestId"), &request_id)
                && (is_undef(&generation) || eq(&prop(e, "lifecycleGeneration"), &generation))
        });
    let ty = prop(&prop(&input, "command"), "type");
    if interaction.is_none()
        || pending_awaiting_response(&prop(&interaction.clone().unwrap_or(Value::Null), "status"))
    {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Approval request '{}' on thread '{}' was already answered.",
            str_of(&request_id),
            str_of(&prop(&input, "threadId"))
        ),
    ))
}

/// `requireThreadAbsent`.
pub fn require_thread_absent(input: Value) -> Result<Value, String> {
    let thread = find_thread_by_id(prop(&input, "readModel"), prop(&input, "threadId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if is_undef(&thread) {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Thread '{}' already exists and cannot be created twice.",
            str_of(&prop(&input, "threadId"))
        ),
    ))
}

/// `requireThreadArchived`.
pub fn require_thread_archived(input: Value) -> Result<Value, String> {
    let thread = require_thread(input.clone())?;
    if !nullish(&prop(&thread, "archivedAt")) {
        return Ok(thread);
    }
    let ty = prop(&prop(&input, "command"), "type");
    Err(invariant_err(
        &ty,
        &format!(
            "Thread '{}' is not archived for command '{}'.",
            str_of(&prop(&input, "threadId")),
            str_of(&ty)
        ),
    ))
}

/// `requireThreadNotArchived`.
pub fn require_thread_not_archived(input: Value) -> Result<Value, String> {
    let thread = require_thread(input.clone())?;
    if nullish(&prop(&thread, "archivedAt")) {
        return Ok(thread);
    }
    let ty = prop(&prop(&input, "command"), "type");
    Err(invariant_err(
        &ty,
        &format!(
            "Thread '{}' is already archived and cannot handle command '{}'.",
            str_of(&prop(&input, "threadId")),
            str_of(&ty)
        ),
    ))
}

/// `threadResumePreconditionViolation`.
pub fn thread_resume_precondition_violation(
    thread: Value,
    precondition: Value,
) -> Result<Value, String> {
    if !nullish(&prop(&thread, "archivedAt")) {
        return Ok(json!("thread-archived"));
    }
    if thread_has_in_flight_turn(thread.clone())? {
        return Ok(json!("turn-in-flight"));
    }
    let latest = prop(&thread, "latestTurn");
    let recorded_turn = prop(&precondition, "recordedTurnId");
    let recorded_at = str_of(&prop(&precondition, "recordedAt"));
    if !latest.is_null()
        && !is_undef(&latest)
        && str_of(&prop(&latest, "state")) == "completed"
        && (eq(&prop(&latest, "turnId"), &recorded_turn)
            || prop(&latest, "completedAt").is_null()
            || str_of(&prop(&latest, "completedAt")).as_str() >= recorded_at.as_str())
    {
        return Ok(json!("turn-completed"));
    }
    if !latest.is_null()
        && !is_undef(&latest)
        && str_of(&prop(&latest, "state")) != "completed"
        && !eq(&prop(&latest, "turnId"), &recorded_turn)
        && (str_of(&prop(&latest, "requestedAt")).as_str() >= recorded_at.as_str()
            || (!recorded_turn.is_null()
                && !prop(&latest, "startedAt").is_null()
                && str_of(&prop(&latest, "startedAt")).as_str() >= recorded_at.as_str()))
    {
        return Ok(json!("turn-replaced"));
    }
    Ok(Value::Null)
}

/// `threadResumePreconditionDetail`.
pub fn thread_resume_precondition_detail(
    thread_id: Value,
    violation: String,
) -> Result<String, String> {
    let tid = str_of(&thread_id);
    Ok(match violation.as_str() {
        "thread-archived" => {
            format!("Thread '{tid}' was archived after it was remembered for resume.")
        }
        "turn-completed" => {
            format!("Thread '{tid}' finished on its own; there is nothing to resume.")
        }
        "turn-in-flight" => format!("Thread '{tid}' already has a turn in flight."),
        _ => format!("Thread '{tid}' started another turn after it was remembered for resume."),
    })
}

// ---------------------------------------------------------------- dispatchCommandNormalization.ts

/// `withoutRemoteResumeComputerGrant`.
pub fn without_remote_resume_computer_grant(
    command: Value,
    principal_owner_id: String,
) -> Result<Value, String> {
    if str_of(&prop(&command, "type")) != "thread.turn.resume"
        || is_undef(&prop(&command, "computerControlGeneration"))
        || !principal_owner_id.starts_with("remote-device:")
    {
        return Ok(command);
    }
    if let Value::Object(mut o) = command {
        o.remove("computerControlGeneration");
        return Ok(Value::Object(o));
    }
    Ok(command)
}

// ---------------------------------------------------------------- forkSourceCutoff.ts

/// `resolveForkSourceCutoff`.
pub fn resolve_fork_source_cutoff(input: Value) -> Result<Value, String> {
    let through = prop(&input, "throughMessageId");
    if nullish(&through) {
        return Ok(json!({"kind": "latest"}));
    }
    let messages = prop(&input, "sourceMessages");
    if !is_arr(&messages) {
        return Ok(json!({"kind": "unavailable", "reason": "source thread is unavailable"}));
    }
    let list = arr(&messages);
    let index = list.iter().position(|m| eq(&prop(m, "id"), &through));
    let Some(index) = index else {
        return Ok(
            json!({"kind": "unavailable", "reason": "fork message is no longer in the source thread"}),
        );
    };
    let message = &list[index];
    let later: Vec<&Value> = list[index + 1..]
        .iter()
        .filter(|m| {
            let r = str_of(&prop(m, "role"));
            r == "user" || r == "assistant"
        })
        .collect();
    if str_of(&prop(message, "role")) != "assistant" {
        return Ok(
            json!({"kind": "unavailable", "reason": "fork message is not an assistant turn boundary"}),
        );
    }
    let turn_id = prop(message, "turnId");
    if nullish(&turn_id) {
        return Ok(json!({"kind": "unavailable", "reason": "fork message has no provider turn"}));
    }
    if later.iter().any(|l| eq(&prop(l, "turnId"), &turn_id)) {
        return Ok(
            json!({"kind": "unavailable", "reason": "fork message is not the end of its turn"}),
        );
    }
    Ok(json!({"kind": "turn", "turnId": turn_id}))
}

// ---------------------------------------------------------------- forkThreadTitle.ts

fn is_sidechat_thread(t: &Value) -> bool {
    truthy(&prop(t, "sidechatSourceThreadId")) || !nullish(&prop(t, "sidechatContext"))
}

fn find_lineage_root(
    source: &Value,
    by_id: &std::collections::HashMap<String, Value>,
) -> (Value, bool) {
    let mut visited = std::collections::HashSet::from([str_of(&prop(source, "id"))]);
    let mut current = source.clone();
    while !nullish(&prop(&current, "forkSourceThreadId")) {
        let pid = str_of(&prop(&current, "forkSourceThreadId"));
        match by_id.get(&pid) {
            Some(parent)
                if eq(&prop(parent, "projectId"), &prop(source, "projectId"))
                    && !visited.contains(&str_of(&prop(parent, "id"))) =>
            {
                visited.insert(str_of(&prop(parent, "id")));
                current = parent.clone();
            }
            _ => return (current, false),
        }
    }
    (current, true)
}

fn parse_fork_version(title: &str) -> (String, f64) {
    if let Some(at) = title.rfind(" (") {
        if title.ends_with(')') {
            let num_str = &title[at + 2..title.len() - 1];
            if let Ok(v) = num_str.parse::<f64>() {
                if v.fract() == 0.0 && v.abs() < 9.007_199_254_740_992e15 && v >= 2.0 {
                    return (title[..at].to_string(), v);
                }
            }
        }
    }
    (title.to_string(), 1.0)
}

/// `buildForkThreadTitle`.
pub fn build_fork_thread_title(
    source: Value,
    project_threads: Vec<Value>,
) -> Result<String, String> {
    let by_id: std::collections::HashMap<String, Value> = project_threads
        .iter()
        .map(|t| (str_of(&prop(t, "id")), t.clone()))
        .collect();
    let (root, complete) = find_lineage_root(&source, &by_id);
    let fallback = parse_fork_version(&str_of(&prop(&source, "title")));
    let lineage = if complete {
        parse_fork_version(&str_of(&prop(&root, "title")))
    } else {
        fallback
    };
    let family: Vec<&Value> = project_threads
        .iter()
        .filter(|t| {
            if !eq(&prop(t, "projectId"), &prop(&source, "projectId")) || is_sidechat_thread(t) {
                return false;
            }
            let (r, c) = find_lineage_root(t, &by_id);
            c && complete && eq(&prop(&r, "id"), &prop(&root, "id"))
        })
        .collect();
    let mut latest = lineage.1;
    for t in &family {
        let cand = parse_fork_version(&str_of(&prop(t, "title")));
        if cand.0 == lineage.0 && cand.1 > latest {
            latest = cand.1;
        }
    }
    let next = (2.0f64).max(family.len() as f64 + 1.0).max(latest + 1.0);
    Ok(format!("{} ({})", lineage.0, js::number_to_string(next)))
}

// ---------------------------------------------------------------- messageTurnId.ts

/// `resolveStableMessageTurnId` — `existing ?? incoming ?? null`.
pub fn resolve_stable_message_turn_id(input: Value) -> Result<Value, String> {
    let existing = prop(&input, "existingTurnId");
    if !nullish(&existing) {
        return Ok(existing);
    }
    let incoming = prop(&input, "incomingTurnId");
    if !nullish(&incoming) {
        return Ok(incoming);
    }
    Ok(Value::Null)
}

// ---------------------------------------------------------------- orchestrationAdmission.ts

const RESERVED: [&str; 11] = [
    "thread.turn.interrupt",
    "thread.task.stop",
    "thread.task.background",
    "thread.approval.respond",
    "thread.user-input.respond",
    "thread.session.stop",
    "thread.turn.dispatch-queued",
    "thread.session.set",
    "thread.message.assistant.complete",
    "thread.turn.diff.complete",
    "thread.revert.complete",
];

/// `usesReservedCommandAdmission`.
pub fn uses_reserved_command_admission(r#type: Value) -> Result<bool, String> {
    Ok(RESERVED.contains(&str_of(&r#type).as_str()))
}

/// `isQuiescingCommandAdmissible`.
pub fn is_quiescing_command_admissible(r#type: Value) -> Result<bool, String> {
    let t = str_of(&r#type);
    Ok(RESERVED.contains(&t.as_str())
        || t == "thread.activity.append"
        || t == "thread.conversation.rollback.complete")
}

/// `orchestrationCommandLane`.
pub fn orchestration_command_lane(r#type: Value) -> Result<String, String> {
    let t = str_of(&r#type);
    if RESERVED.contains(&t.as_str()) || t == "thread.conversation.rollback.complete" {
        return Ok("control".into());
    }
    Ok(match t.as_str() {
        "thread.create"
        | "thread.turn.start"
        | "thread.turn.resume"
        | "thread.checkpoint.revert"
        | "thread.conversation.rollback"
        | "thread.message.edit-and-resend" => "user",
        _ => "normal",
    }
    .into())
}

// ---------------------------------------------------------------- sidechatLifecycle.ts

/// `isExpiredSidechat`.
pub fn is_expired_sidechat(thread: Value) -> Result<bool, String> {
    Ok(!nullish(&prop(&thread, "sidechatExpiredAt")))
}

fn parse_ts(value: &str) -> Option<f64> {
    if value.len() < 20 {
        return None;
    }
    let num = |a: usize, n: usize| value.get(a..a + n)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (h, mi, sec) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    let days = days_from_civil(y, mo as u32, d as u32);
    Some(
        (days * 86_400 + h * 3600 + mi * 60 + sec) as f64 * 1000.0 + num(20, 3).unwrap_or(0) as f64,
    )
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn iso_from_ms(ms: f64) -> String {
    let secs = (ms / 1000.0).floor() as i64;
    let millis = (ms % 1000.0) as i64;
    let days = secs.div_euclid(86_400);
    let ds = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        ds / 3600,
        ds / 60 % 60,
        ds % 60
    )
}

/// `latestSidechatActivityAt`.
pub fn latest_sidechat_activity_at(
    current: Option<String>,
    candidate: String,
) -> Result<String, String> {
    let current_ts = current.as_deref().and_then(parse_ts);
    let candidate_ts = parse_ts(&candidate);
    Ok(match (current_ts, candidate_ts) {
        (_, None) => current.clone().unwrap_or(candidate),
        (None, Some(c)) => iso_from_ms(c),
        (Some(cur), Some(cand)) => iso_from_ms(cur.max(cand)),
    })
}

/// `sidechatActivityInstantsEqual`.
pub fn sidechat_activity_instants_equal(left: String, right: String) -> Result<bool, String> {
    match (parse_ts(&left), parse_ts(&right)) {
        (Some(l), Some(r)) => Ok(l == r),
        _ => Ok(left == right),
    }
}

// ---------------------------------------------------------------- turnLifecycle.ts

/// `settleTurnStateFromSession`.
pub fn settle_turn_state_from_session(
    session: Value,
    existing_state: String,
) -> Result<Value, String> {
    let status = str_of(&prop(&session, "status"));
    let active = prop(&session, "activeTurnId");
    if !active.is_null() && status != "error" {
        return Ok(Value::Null);
    }
    Ok(match status.as_str() {
        "error" => json!("error"),
        "interrupted" | "stopped" => json!("interrupted"),
        "ready" => json!(match existing_state.as_str() {
            "error" => "error",
            "interrupted" => "interrupted",
            _ => "completed",
        }),
        _ => Value::Null,
    })
}

/// `maxIso`.
pub fn max_iso(left: Option<String>, right: String) -> Result<String, String> {
    Ok(match left {
        None => right,
        Some(l) if right > l => right,
        Some(l) => l,
    })
}

// ---------------------------------------------------------------- turnStartSession.ts

/// `deriveTurnStartModelSelection`.
pub fn derive_turn_start_model_selection(input: Value) -> Result<Value, String> {
    let requested = prop(&input, "requestedModelSelection");
    let current = prop(&input, "currentModelSelection");
    if !is_undef(&requested)
        && (eq(&prop(&requested, "provider"), &prop(&current, "provider"))
            || truthy(&prop(&input, "canAdoptRequestedProvider")))
    {
        return Ok(requested);
    }
    Ok(current)
}

/// `countNativeTurnStartMessages`.
pub fn count_native_turn_start_messages(messages: Vec<Value>) -> Result<f64, String> {
    Ok(messages
        .iter()
        .filter(|m| {
            let src = str_of(&prop(m, "source"));
            src != "fork-import" && src != "handoff-import"
        })
        .count() as f64)
}

/// `canAdoptFirstTurnProvider`.
pub fn can_adopt_first_turn_provider(input: Value) -> Result<bool, String> {
    Ok(!truthy(&prop(&input, "hasLatestTurn"))
        && !truthy(&prop(&input, "hasSession"))
        && count_native_turn_start_messages(arr(&prop(&input, "messages")))? <= 1.0)
}

/// `deriveTurnStartSession`.
pub fn derive_turn_start_session(input: Value) -> Result<Value, String> {
    let current = prop(&input, "currentSession");
    let status = str_of(&prop(&current, "status"));
    if status == "starting" || status == "running" {
        return Ok(Value::Null);
    }
    let established = prop(&input, "sessionProviderEstablished");
    let established = !matches!(established, Value::Bool(false));
    let session_name = if !nullish(&prop(&current, "providerName")) && established {
        prop(&current, "providerName")
    } else {
        undef()
    };
    let session_instance = if !nullish(&prop(&current, "providerInstanceId")) && established {
        prop(&current, "providerInstanceId")
    } else {
        prop(&input, "providerInstanceId")
    };
    let mut out = Map::new();
    out.insert("threadId".into(), prop(&input, "threadId"));
    out.insert("status".into(), json!("starting"));
    out.insert(
        "providerName".into(),
        if nullish(&session_name) {
            prop(&input, "providerName")
        } else {
            session_name
        },
    );
    if !is_undef(&session_instance) {
        out.insert("providerInstanceId".into(), session_instance);
    }
    let runtime = prop(&current, "runtimeMode");
    out.insert(
        "runtimeMode".into(),
        if nullish(&runtime) {
            prop(&input, "requestedRuntimeMode")
        } else {
            runtime
        },
    );
    out.insert("activeTurnId".into(), Value::Null);
    out.insert("lastError".into(), Value::Null);
    out.insert("updatedAt".into(), prop(&input, "requestedAt"));
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------- projectRelocationPaths.ts

/// `relocateProjectPath`.
pub fn relocate_project_path(
    value: Option<String>,
    previous_root: String,
    next_root: String,
    platform: Value,
) -> Result<Value, String> {
    let Some(v) = value.clone() else {
        return Ok(if value.is_none() {
            Value::Null
        } else {
            undef()
        });
    };
    let platform_name = match &platform {
        Value::String(p) => p.clone(),
        _ => self::platform_name().to_string(),
    };
    let absolute = |p: &str| p.starts_with('/');
    if !absolute(&v) || !absolute(&previous_root) || !absolute(&next_root) {
        return Ok(json!(v));
    }
    let comparable = |x: &str| {
        if platform_name == "darwin" {
            normalize_root(x, "darwin")
        } else {
            x.to_string()
        }
    };
    let (prev_c, v_c) = (comparable(&previous_root), comparable(&v));
    let rel = path_relative(&prev_c, &v_c);
    if rel == ".." || rel.starts_with("../") || rel.starts_with('/') {
        return Ok(json!(v));
    }
    Ok(json!(format!(
        "{}/{}",
        next_root.trim_end_matches('/'),
        rel
    )))
}

fn path_relative(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0;
    while common < from_parts.len()
        && common < to_parts.len()
        && from_parts[common] == to_parts[common]
    {
        common += 1;
    }
    let ups = from_parts.len() - common;
    let mut out: Vec<String> = vec!["..".into(); ups];
    out.extend(to_parts[common..].iter().map(|s| s.to_string()));
    out.join("/")
}

/// `providerWorkspaceChanged`.
pub fn provider_workspace_changed(
    current_cwd: Option<String>,
    requested_cwd: Option<String>,
    platform: Value,
) -> Result<bool, String> {
    let (Some(cur), Some(req)) = (current_cwd, requested_cwd) else {
        return Ok(false);
    };
    let platform_name = match &platform {
        Value::String(p) => p.clone(),
        _ => self::platform_name().to_string(),
    };
    Ok(!roots_equal(&cur, &req, &platform_name))
}

// ---------------------------------------------------------------- providerIntentClassification.ts

const INTENT_TYPES: [&str; 19] = [
    "thread.created",
    "thread.deleted",
    "thread.archived",
    "thread.unarchived",
    "thread.meta-updated",
    "thread.session-set",
    "thread.runtime-mode-set",
    "thread.interaction-mode-set",
    "thread.turn-queued",
    "thread.turn-start-requested",
    "thread.claude-cache-response-requested",
    "thread.goal-continuation-requested",
    "thread.turn-interrupt-requested",
    "thread.task-stop-requested",
    "thread.task-background-requested",
    "thread.approval-response-requested",
    "thread.user-input-response-requested",
    "thread.conversation-rollback-requested",
    "thread.message-edit-resend-requested",
];

/// `isProviderIntentEventType`.
pub fn is_provider_intent_event_type(event_type: String) -> Result<bool, String> {
    Ok(INTENT_TYPES.contains(&event_type.as_str()))
}

/// `isProviderIntentEvent`.
pub fn is_provider_intent_event(event: Value) -> Result<bool, String> {
    Ok(INTENT_TYPES.contains(&str_of(&prop(&event, "type")).as_str()))
}

fn event_type_of(event: &Value) -> String {
    str_of(&prop(event, "type"))
}

/// `isReplaySafeClaimedProviderIntent`.
pub fn is_replay_safe_claimed_provider_intent(event: Value) -> Result<bool, String> {
    let t = event_type_of(&event);
    Ok(t == "thread.created"
        || t == "thread.archived"
        || t == "thread.unarchived"
        || t == "thread.turn-queued")
}

/// `isProviderSideEffectIntent`.
pub fn is_provider_side_effect_intent(event: Value) -> Result<bool, String> {
    let t = event_type_of(&event);
    Ok(t != "thread.created"
        && t != "thread.deleted"
        && t != "thread.unarchived"
        && t != "thread.session-set"
        && t != "thread.turn-queued")
}

/// `isClaimedProviderIntent`.
pub fn is_claimed_provider_intent(event: Value) -> Result<bool, String> {
    Ok(is_replay_safe_claimed_provider_intent(event.clone())?
        || is_provider_side_effect_intent(event)?)
}

/// `isQuarantineExemptProviderIntent`.
pub fn is_quarantine_exempt_provider_intent(event: Value) -> Result<bool, String> {
    let t = event_type_of(&event);
    Ok(t == "thread.turn-interrupt-requested"
        || t == "thread.session-stop-requested"
        || t == "thread.archived")
}

// ---------------------------------------------------------------- stalePendingInteractions.ts

/// `isUnsettledPendingInteraction`.
pub fn is_unsettled_pending_interaction(row: Value) -> Result<bool, String> {
    Ok(str_of(&prop(&row, "status")) != "confirmed")
}

/// `pendingInteractionRequestKind`.
pub fn pending_interaction_request_kind(interaction_kind: Value) -> Result<String, String> {
    Ok(if str_of(&interaction_kind) == "approval" {
        "approval".into()
    } else {
        "user-input".into()
    })
}

fn stale_failure_detail(request_kind: &str, request_id: &str) -> String {
    format!(
        "The provider session ended before this {request_kind} request '{request_id}' was answered."
    )
}

/// `buildStalePendingRequestSettlementCommand`.
pub fn build_stale_pending_request_settlement_command(input: Value) -> Result<Value, String> {
    let kind = str_of(&prop(&input, "requestKind"));
    let is_approval = kind == "approval";
    let request_id = prop(&input, "requestId");
    let generation = prop(&input, "lifecycleGeneration");
    let now = prop(&input, "now");
    let mut payload = Map::new();
    payload.insert(
        "detail".into(),
        json!(stale_failure_detail(&kind, &str_of(&request_id))),
    );
    payload.insert("requestId".into(), request_id);
    if truthy(&generation) {
        payload.insert("lifecycleGeneration".into(), generation);
    }
    Ok(json!({
        "type": "thread.activity.append",
        "commandId": prop(&input, "commandId"),
        "threadId": prop(&input, "threadId"),
        "activity": {
            "id": prop(&input, "commandId"),
            "tone": "error",
            "kind": if is_approval { "provider.approval.respond.failed" } else { "provider.user-input.respond.failed" },
            "summary": if is_approval { "Provider approval response failed" } else { "Provider user input response failed" },
            "payload": Value::Object(payload),
            "turnId": Value::Null,
            "createdAt": now,
        },
        "createdAt": now,
    }))
}

// ---------------------------------------------------------------- projectImportHistory.ts

/// `nativeImportId`.
pub fn native_import_id(provider: Value, cursor: Value) -> Result<Value, String> {
    if !truthy(&cursor) || !matches!(cursor, Value::Object(_)) {
        return Ok(Value::Null);
    }
    let value = if str_of(&provider) == "codex" {
        prop(&cursor, "threadId")
    } else {
        prop(&cursor, "resume")
    };
    match s(&value) {
        Some(v) if !v.is_empty() => Ok(json!(v)),
        _ => Ok(Value::Null),
    }
}

// ---------------------------------------------------------------- projectImportDestinations.ts

/// `makeProjectImportDestinations` — returns the live-find closure's result shape as data:
/// `{ findThread, findOrigin }` are functions in JS; across the bridge only their
/// precomputed maps are meaningful. Returns `{ projects, threads }` the callers check.
pub fn make_project_import_destinations(model: Value) -> Result<Value, String> {
    let projects: Vec<Value> = arr(&prop(&model, "projects"))
        .into_iter()
        .filter(|p| prop(p, "deletedAt").is_null())
        .map(|p| prop(&p, "id"))
        .collect();
    Ok(json!({ "projects": projects, "threads": prop(&model, "threads") }))
}

// ---------------------------------------------------------------- stale helpers for later sections

fn opt(v: Value) -> Value {
    if is_undef(&v) { Value::Null } else { v }
}
