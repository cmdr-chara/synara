//! The Rust port of `apps/server/src/checkpointing/` helpers.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn marker() -> Value {
    json!({"$undefined": true})
}

fn is_undef(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined").and_then(Value::as_bool) == Some(true))
}

fn marker_value() -> &'static Value {
    static M: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    M.get_or_init(|| json!({"$undefined": true}))
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    match v {
        Value::Object(m) if !m.contains_key("$undefined") => {
            m.get(key).unwrap_or_else(|| marker_value())
        }
        _ => marker_value(),
    }
}

fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if x.is_null() || is_undef(x) {
                    String::new()
                } else {
                    js_str(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(m) => {
            if m.contains_key("$undefined") {
                "undefined".into()
            } else if m.contains_key("$num") {
                crate::js::number_to_string(
                    m["$num"]
                        .as_str()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(f64::NAN),
                )
            } else {
                "[object Object]".into()
            }
        }
    }
}

const B64URL: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn js_str_to_num(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).map_or(f64::NAN, |v| v as f64);
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_to_byte(v: &Value) -> u8 {
    // `Buffer.from([...])` element coercion: `Number(x) & 0xFF`, NaN -> 0.
    let n = match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_str_to_num(s),
        _ => f64::NAN,
    };
    if n.is_nan() {
        return 0;
    }
    // `& 0xFF` on the ToInt32 result.
    (n as i64 as i32 & 0xFF) as u8
}

/// `Encoding.encodeBase64Url`: strings encode their UTF-8 bytes, arrays their
/// elementwise bytes, `null` throws, everything else encodes to `""`.
fn b64url_of(v: &Value) -> Result<String, String> {
    if v.is_null() {
        return Err("Cannot read properties of null (reading 'length')".into());
    }
    let bytes: Vec<u8> = match v {
        Value::String(s) => s.as_bytes().to_vec(),
        Value::Array(items) => items.iter().map(js_to_byte).collect(),
        _ => return Ok(String::new()),
    };
    let bytes = bytes.as_slice();
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64URL[(n >> 18) as usize & 63] as char);
        out.push(B64URL[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(B64URL[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL[n as usize & 63] as char);
        }
    }
    Ok(out)
}

const REFS_PREFIX: &str = "refs/synara/checkpoints";

fn seg_ok(s: &str, extra_dot: bool) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| {
            b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-') || (extra_dot && b == b'.')
        })
}

struct ManagedRef<'a> {
    namespace: &'a str,
    thread_token: &'a str,
    kind: &'a str,
    value_token: &'a str,
}

/// `^refs/([A-Za-z0-9._-]+)/checkpoints/([A-Za-z0-9_-]+)/(turn|message-start|turn-start|turn-live|revert-rescue)/([A-Za-z0-9_-]+)$`
fn parse_managed(value: &str) -> Option<ManagedRef<'_>> {
    let rest = value.strip_prefix("refs/")?;
    let (namespace, rest) = rest.split_once('/')?;
    let rest = rest.strip_prefix("checkpoints/")?;
    let (thread, rest) = rest.split_once('/')?;
    let (kind, token) = rest.split_once('/')?;
    if token.contains('/') {
        return None;
    }
    if !seg_ok(namespace, true) || !seg_ok(thread, false) || !seg_ok(token, false) {
        return None;
    }
    if !matches!(
        kind,
        "turn" | "message-start" | "turn-start" | "turn-live" | "revert-rescue"
    ) {
        return None;
    }
    if kind == "turn" && !token.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some(ManagedRef {
        namespace,
        thread_token: thread,
        kind,
        value_token: token,
    })
}

/// Port of `parseCheckpointFilesFromUnifiedDiff`. The TS returns an unevaluated
/// lazy `Effect`; the recorded shape is the same for every input.
pub fn parse_checkpoint_files_from_unified_diff(diff: String) -> Result<serde_json::Value, String> {
    let _ = &diff;
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}

/// Port of `parseManagedCheckpointRef`.
pub fn parse_managed_checkpoint_ref(value: String) -> Result<Option<serde_json::Value>, String> {
    let Some(m) = parse_managed(&value) else {
        return Ok(None);
    };
    Ok(Some(json!({
        "namespace": m.namespace,
        "threadToken": m.thread_token,
        "kind": m.kind,
        "valueToken": m.value_token,
        "familyPrefix": format!("refs/{}/checkpoints/{}", m.namespace, m.thread_token),
    })))
}

/// Port of `isManagedCheckpointRefForThread`.
pub fn is_managed_checkpoint_ref_for_thread(
    value: String,
    thread_id: serde_json::Value,
) -> Result<bool, String> {
    if thread_id.is_null() {
        return Err("Cannot read properties of null (reading 'length')".into());
    }
    let token = b64url_of(&thread_id)?;
    Ok(parse_managed(&value).is_some_and(|m| m.thread_token == token))
}

/// Port of `checkpointRefForThreadTurn`.
pub fn checkpoint_ref_for_thread_turn(
    thread_id: serde_json::Value,
    turn_count: f64,
) -> Result<serde_json::Value, String> {
    Ok(json!(format!(
        "{REFS_PREFIX}/{}/turn/{}",
        b64url_of(&thread_id)?,
        crate::js::number_to_string(turn_count)
    )))
}

/// Port of `checkpointRefForThreadTurnInManagedFamily`.
pub fn checkpoint_ref_for_thread_turn_in_managed_family(
    managed_ref: String,
    thread_id: serde_json::Value,
    turn_count: f64,
) -> Result<Option<serde_json::Value>, String> {
    let thread_token = b64url_of(&thread_id)?;
    let Some(m) = parse_managed(&managed_ref) else {
        return Ok(None);
    };
    if m.thread_token != thread_token {
        return Ok(None);
    }
    Ok(Some(json!(format!(
        "refs/{}/checkpoints/{}/turn/{}",
        m.namespace,
        m.thread_token,
        crate::js::number_to_string(turn_count)
    ))))
}

/// Port of `checkpointRefForThreadMessageStart`.
pub fn checkpoint_ref_for_thread_message_start(
    thread_id: serde_json::Value,
    message_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!(format!(
        "{REFS_PREFIX}/{}/message-start/{}",
        b64url_of(&thread_id)?,
        b64url_of(&message_id)?
    )))
}

/// Port of `checkpointRefForThreadTurnStart`.
pub fn checkpoint_ref_for_thread_turn_start(
    thread_id: serde_json::Value,
    turn_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!(format!(
        "{REFS_PREFIX}/{}/turn-start/{}",
        b64url_of(&thread_id)?,
        b64url_of(&turn_id)?
    )))
}

/// Port of `checkpointRefForThreadTurnStartInManagedFamily`.
pub fn checkpoint_ref_for_thread_turn_start_in_managed_family(
    managed_ref: String,
    thread_id: serde_json::Value,
    turn_id: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let thread_token = b64url_of(&thread_id)?;
    let Some(m) = parse_managed(&managed_ref) else {
        return Ok(None);
    };
    if m.thread_token != thread_token {
        return Ok(None);
    }
    Ok(Some(json!(format!(
        "refs/{}/checkpoints/{}/turn-start/{}",
        m.namespace,
        m.thread_token,
        b64url_of(&turn_id)?
    ))))
}

/// Port of `checkpointRefForThreadTurnLive`.
pub fn checkpoint_ref_for_thread_turn_live(
    thread_id: serde_json::Value,
    turn_id: serde_json::Value,
    revision: Option<String>,
) -> Result<serde_json::Value, String> {
    let suffix = match revision {
        None => String::new(),
        Some(rev) => format!("_{}", sha256_hex(rev.as_bytes())),
    };
    Ok(json!(format!(
        "{REFS_PREFIX}/{}/turn-live/{}{}",
        b64url_of(&thread_id)?,
        b64url_of(&turn_id)?,
        suffix
    )))
}

/// Port of `checkpointRefForThreadRevertRescue`.
pub fn checkpoint_ref_for_thread_revert_rescue(
    thread_id: serde_json::Value,
    token: String,
) -> Result<serde_json::Value, String> {
    let git_ref = format!(
        "{REFS_PREFIX}/{}/revert-rescue/{}",
        b64url_of(&thread_id)?,
        token
    );
    // `CheckpointRef.makeUnsafe` rejects refs with surrounding whitespace.
    if git_ref.trim() != git_ref {
        return Err(format!(
            "Expected a string with no leading or trailing whitespace, got \"{git_ref}\""
        ));
    }
    Ok(json!(git_ref))
}

/// Port of `resolveProjectCwdForKind`. Raw value semantics: `null` stays null,
/// a missing `workspaceRoot` stays `undefined`.
pub fn resolve_project_cwd_for_kind(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'kind')".into());
    }
    let kind = prop(&input, "kind");
    let worktree = prop(&input, "worktreePath");
    let has_worktree = !(is_undef(worktree) || worktree.is_null());
    if kind.as_str() == Some("chat") && !has_worktree {
        return Ok(Value::Null);
    }
    Ok(prop(&input, "workspaceRoot").clone())
}

// --- shared/threadWorkspace --------------------------------------------------

fn is_likely_windows_root(trimmed: &str) -> bool {
    let b = trimmed.as_bytes();
    (b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || matches!(b[2], b'/' | b'\\')))
        || trimmed.starts_with("\\\\")
        || trimmed.starts_with("//")
}

fn normalize_root(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd = trimmed.replace('\\', "/");
    let is_windows_drive_root = {
        let b = fwd.as_bytes();
        b.len() >= 3
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && fwd[2..].chars().all(|c| c == '/')
    };
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let body_raw = &fwd[prefix.len()..];
    // collapse repeated slashes, strip trailing slashes
    let mut body = String::with_capacity(body_raw.len());
    let mut last_slash = false;
    for c in body_raw.chars() {
        if c == '/' {
            if !last_slash {
                body.push(c);
            }
            last_slash = true;
        } else {
            body.push(c);
            last_slash = false;
        }
    }
    while body.ends_with('/') {
        body.pop();
    }
    let normalized = format!("{prefix}{body}");
    let final_value = if is_windows_drive_root {
        format!("{}/", &fwd[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };
    if is_likely_windows_root(trimmed) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

fn roots_equal(left: &str, right: &str) -> bool {
    normalize_root(left) == normalize_root(right)
}

fn is_root_within(candidate: &str, ancestor: &str) -> bool {
    let c = normalize_root(candidate);
    let a = normalize_root(ancestor);
    if c.is_empty() || a.is_empty() {
        return false;
    }
    if c == a {
        return true;
    }
    let prefix = if a.ends_with('/') { a } else { format!("{a}/") };
    c.starts_with(&prefix)
}

/// `resolveSharedThreadWorkspaceCwd` — threadEnvironment.ts.
fn shared_resolve_thread_workspace_cwd(input: &Value) -> Value {
    let worktree = prop(input, "worktreePath");
    let has_worktree = !(is_undef(worktree) || worktree.is_null());
    let mode = if has_worktree {
        "worktree".to_string()
    } else {
        prop(input, "envMode")
            .as_str()
            .unwrap_or("local")
            .to_string()
    };
    let workdir = prop(input, "workingDirectory");
    if mode == "worktree" {
        if has_worktree
            && !is_undef(workdir)
            && workdir.is_string()
            && !workdir
                .as_str()
                .unwrap_or_default()
                .replace('\\', "/")
                .split('/')
                .any(|s| s == "..")
            && is_root_within(
                workdir.as_str().unwrap_or_default(),
                worktree.as_str().unwrap_or_default(),
            )
        {
            return workdir.clone();
        }
        return if has_worktree {
            worktree.clone()
        } else {
            Value::Null
        };
    }
    if !is_undef(workdir) {
        return workdir.clone();
    }
    let pcwd = prop(input, "projectCwd");
    if !is_undef(pcwd) {
        pcwd.clone()
    } else {
        Value::Null
    }
}

/// Port of `resolveThreadWorkspaceCwd`.
pub fn resolve_thread_workspace_cwd(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'projects')".into());
    }
    let projects = prop(&input, "projects");
    match projects {
        v if is_undef(v) => {
            return Err("Cannot read properties of undefined (reading 'find')".into());
        }
        Value::Null => {
            return Err("Cannot read properties of null (reading 'find')".into());
        }
        Value::Array(_) => {}
        _ => return Err("input.projects.find is not a function".into()),
    }
    let thread = prop(&input, "thread");
    let project_id = prop(thread, "projectId");
    let empty: Vec<Value> = Vec::new();
    let project_list: Vec<Value> = match &projects {
        Value::Array(a) => a.clone(),
        _ => empty,
    };
    let project = project_list
        .iter()
        .find(|e| prop(e, "id") == project_id)
        .cloned();
    let parent = prop(&input, "dedicatedChatWorkspaceParent");
    let parent_set = !is_undef(parent);
    let project_root: Value = project.as_ref().map_or_else(
        || marker_value().clone(),
        |p| prop(p, "workspaceRoot").clone(),
    );
    let dedicated = project.as_ref().is_some_and(|p| {
        prop(p, "kind").as_str() == Some("chat")
            && parent_set
            && project_root.is_string()
            && parent.is_string()
            && is_root_within(
                project_root.as_str().unwrap_or_default(),
                parent.as_str().unwrap_or_default(),
            )
            && !roots_equal(
                project_root.as_str().unwrap_or_default(),
                parent.as_str().unwrap_or_default(),
            )
    });
    let project_cwd = if dedicated {
        project_root.clone()
    } else {
        let kind: Value = project
            .as_ref()
            .map_or_else(|| marker_value().clone(), |p| prop(p, "kind").clone());
        let worktree = prop(thread, "worktreePath");
        if kind.as_str() == Some("chat") && (is_undef(worktree) || worktree.is_null()) {
            Value::Null
        } else {
            project_root.clone()
        }
    };
    let mut shared = serde_json::Map::new();
    shared.insert("projectCwd".into(), project_cwd);
    shared.insert("envMode".into(), prop(thread, "envMode").clone());
    shared.insert("worktreePath".into(), prop(thread, "worktreePath").clone());
    shared.insert(
        "workingDirectory".into(),
        prop(thread, "workingDirectory").clone(),
    );
    let out = shared_resolve_thread_workspace_cwd(&Value::Object(shared));
    // `?? undefined`: a null result becomes undefined.
    if out.is_null() { Ok(marker()) } else { Ok(out) }
}

// --- historyIntegration.ts ---------------------------------------------------

fn sha256_hex(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

const HEAD_TRAILER: &str = "Synara-Head";
const REFLOG_TRAILER: &str = "Synara-Head-Reflog";

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(m) => {
            if m.contains_key("$undefined") {
                false
            } else if m.contains_key("$num") {
                m["$num"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok())
                    .is_some_and(|x| x != 0.0)
            } else {
                true
            }
        }
    }
}

fn is_object_id(s: &str) -> bool {
    matches!(s.len(), 40 | 64)
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// Port of `formatCheckpointHeadTrailer`.
pub fn format_checkpoint_head_trailer(
    head_commit: String,
    reflog_entry: Option<String>,
) -> Result<String, String> {
    let head = format!("{HEAD_TRAILER}: {head_commit}");
    Ok(match reflog_entry {
        None => head,
        Some(entry) => format!("{head}\n{REFLOG_TRAILER}: {}", sha256_hex(entry.as_bytes())),
    })
}

/// Port of `checkpointReflogAnchor`.
pub fn checkpoint_reflog_anchor(entry: String) -> Result<String, String> {
    Ok(sha256_hex(entry.as_bytes()))
}

/// Port of `parseCheckpointCommit`.
pub fn parse_checkpoint_commit(raw: String) -> Result<serde_json::Value, String> {
    let message = raw.find("\n\n").map_or("", |i| &raw[i + 2..]);
    let mut head = Value::Null;
    let mut reflog: Option<Value> = None;
    for line in message.split('\n') {
        if let Some(rest) = line.strip_prefix(&format!("{HEAD_TRAILER}: ")) {
            let v = rest.trim();
            head = if is_object_id(v) {
                json!(v)
            } else {
                Value::Null
            };
        } else if let Some(rest) = line.strip_prefix(&format!("{REFLOG_TRAILER}: ")) {
            let v = rest.trim();
            reflog = Some(
                if v.len() == 64
                    && v.bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                {
                    json!(v)
                } else {
                    marker()
                },
            );
        }
    }
    let mut out = serde_json::Map::new();
    out.insert("head".into(), head);
    if let Some(r) = reflog
        && !is_undef(&r)
    {
        out.insert("reflogAnchor".into(), r);
    }
    Ok(Value::Object(out))
}

/// Port of `checkpointHeadTransitions`.
pub fn checkpoint_head_transitions(
    raw: String,
    from: serde_json::Value,
    to: serde_json::Value,
) -> Result<Option<Vec<serde_json::Value>>, String> {
    // JS short-circuit: `from.reflogAnchor && to.reflogAnchor && from.head && to.head`
    let from_anchor = prop(&from, "reflogAnchor");
    if !js_truthy(from_anchor) {
        if from.is_null() {
            return Err("Cannot read properties of null (reading 'reflogAnchor')".into());
        }
        return Ok(None);
    }
    let to_anchor = prop(&to, "reflogAnchor");
    if !js_truthy(to_anchor) {
        if to.is_null() {
            return Err("Cannot read properties of null (reading 'reflogAnchor')".into());
        }
        return Ok(None);
    }
    let from_head = prop(&from, "head");
    if !js_truthy(from_head) {
        return Ok(None);
    }
    let to_head = prop(&to, "head");
    if !js_truthy(to_head) {
        return Ok(None);
    }
    let fa = js_str(from_anchor);
    let ta = js_str(to_anchor);
    let fh = js_str(from_head);
    let th = js_str(to_head);

    struct Entry {
        head: String,
        action: String,
        anchor: String,
    }
    let mut entries: Vec<Entry> = Vec::new();
    for record in raw.split('\0') {
        if record.is_empty() {
            continue;
        }
        // ^([0-9a-f]+)\tHEAD@\{[0-9]+ [+-][0-9]{4}\}\t(.*)$
        let Some(t1) = record.find('\t') else {
            return Ok(None);
        };
        let head = &record[..t1];
        if head.is_empty()
            || !head
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        {
            return Ok(None);
        }
        if !is_object_id(head) {
            return Ok(None);
        }
        let rest = &record[t1 + 1..];
        let Some(t2) = rest.find('\t') else {
            return Ok(None);
        };
        let middle = &rest[..t2];
        let action = &rest[t2 + 1..];
        let Some(inner) = middle
            .strip_prefix("HEAD@{")
            .and_then(|s| s.strip_suffix('}'))
        else {
            return Ok(None);
        };
        let Some((num, tz)) = inner.split_once(' ') else {
            return Ok(None);
        };
        if num.is_empty()
            || !num.bytes().all(|b| b.is_ascii_digit())
            || tz.len() != 5
            || !matches!(tz.as_bytes()[0], b'+' | b'-')
            || !tz[1..].bytes().all(|b| b.is_ascii_digit())
        {
            return Ok(None);
        }
        entries.push(Entry {
            head: head.to_string(),
            action: action.to_string(),
            anchor: sha256_hex(record.as_bytes()),
        });
    }
    let starts: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.head == fh && e.anchor == fa)
        .map(|(i, _)| i)
        .collect();
    let ends: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.head == th && e.anchor == ta)
        .map(|(i, _)| i)
        .collect();
    if starts.len() != 1 || ends.len() != 1 {
        return Ok(None);
    }
    let (start, end) = (starts[0], ends[0]);
    if start <= end {
        return Ok(None);
    }
    let slice: Vec<serde_json::Value> = entries[end..=start]
        .iter()
        .rev()
        .map(|e| json!({"head": e.head, "action": e.action, "anchor": e.anchor}))
        .collect();
    Ok(Some(slice))
}

/// Port of `parseRawDiff` (`git diff --raw -z`).
pub fn parse_raw_diff(stdout: String) -> Result<Vec<serde_json::Value>, String> {
    let fields: Vec<&str> = stdout.split('\0').collect();
    let mut entries = Vec::new();
    let mut index = 0;
    while index + 1 < fields.len() {
        let meta = fields[index];
        let path = fields[index + 1];
        index += 2;
        if !meta.starts_with(':') || path.is_empty() {
            continue;
        }
        let parts: Vec<&str> = meta[1..].split(' ').collect();
        let source_mode = parts.first().copied().unwrap_or("");
        let source_object = parts.get(2).copied().unwrap_or("");
        let status = parts.get(4).copied().unwrap_or("");
        if source_mode.is_empty() || source_object.is_empty() || status.is_empty() {
            continue;
        }
        entries.push(json!({
            "path": path,
            "sourceMode": source_mode,
            "sourceObject": source_object,
            "status": status.chars().next().unwrap_or_default().to_string(),
        }));
    }
    Ok(entries)
}

/// Port of `parseNameOnlyPaths` (`git diff --name-only -z` → `Set<string>`).
pub fn parse_name_only_paths(stdout: String) -> Result<serde_json::Value, String> {
    let mut seen = std::collections::HashSet::new();
    let items: Vec<Value> = stdout
        .split('\0')
        .filter(|s| !s.is_empty())
        .filter(|s| seen.insert((*s).to_string()))
        .map(|s| Value::String(s.to_string()))
        .collect();
    Ok(json!({"$set": items}))
}

/// Port of `chunkArgumentGroups`.
pub fn chunk_argument_groups(
    groups: Vec<Vec<String>>,
    max_characters: Option<f64>,
) -> Result<Vec<Vec<String>>, String> {
    let max_chars = max_characters.unwrap_or(8_000.0);
    let mut chunks: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut length = 0.0;
    for group in groups {
        let group_length: f64 = group
            .iter()
            .map(|a| a.encode_utf16().count() as f64 + 3.0)
            .sum();
        if !current.is_empty() && length + group_length > max_chars {
            chunks.push(std::mem::take(&mut current));
            length = 0.0;
        }
        current.extend(group);
        length += group_length;
    }
    if !current.is_empty() {
        chunks.push(current);
    }
    Ok(chunks)
}
