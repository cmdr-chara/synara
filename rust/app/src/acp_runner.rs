//! Provider runtime: drives a real agent session over ACP (Agent Client
//! Protocol, JSON-RPC on stdio). The transport and turn machinery are shared
//! across ACP-capable providers; provider-specific support supplies the binary,
//! arguments and environment needed to launch its agent process.
//!
//! A turn lives on a dedicated std::thread: spawn the provider ACP command,
//! initialize,
//! session/new (or session/load when the thread already has a provider
//! session), session/prompt, then translate `session/update` notifications
//! into orchestration events and projection writes until the prompt resolves.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex};

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::orchestration_engine::{open_state_rw, uuid_v4};
use crate::todo::{iso_now_ms, WsErr};

pub(crate) fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), None)
}

struct RunningTurn {
    stdin: Arc<Mutex<ChildStdin>>,
    session_id: String,
    cancelled: Arc<AtomicBool>,
}

static RUNNING: LazyLock<Mutex<HashMap<String, RunningTurn>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Whether a Devin turn is currently live for the thread.
pub fn drop_thread_resources(thread_id: &str) {
    if let Ok(mut parked_map) = PARKED.lock() {
        if let Some(parked) = parked_map.remove(thread_id) {
            drop_parked(parked);
        }
    }
}

/// Stop every parked provider process when the host shuts down. Parked
/// processes are in-memory only; leaving them behind would keep provider
/// sessions and local servers alive after Synara can no longer control them.
pub fn drop_all_resources() {
    let parked = if let Ok(mut parked_map) = PARKED.lock() {
        parked_map.drain().map(|(_, parked)| parked).collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    for parked in parked {
        drop_parked(parked);
    }
}

pub fn has_running_turn(thread_id: &str) -> bool {
    RUNNING
        .lock()
        .ok()
        .map(|m| m.contains_key(thread_id))
        .unwrap_or(false)
}

/// Append one event row (actor `provider`, no command receipt) so the
/// subscribeThread poller picks it up and clients refetch the snapshot.
pub(crate) fn append_event(
    conn: &Connection,
    thread_id: &str,
    event_type: &str,
    occurred_at: &str,
    payload: Value,
) -> Result<(), WsErr> {
    let stream_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(stream_version), 0) + 1 \
             FROM orchestration_events WHERE aggregate_kind = 'thread' AND stream_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .map_err(|e| err(e.to_string()))?;
    conn.execute(
        "INSERT INTO orchestration_events \
         (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at, \
          command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json) \
         VALUES (?1, 'thread', ?2, ?3, ?4, ?5, NULL, NULL, NULL, 'provider', ?6, '{}')",
        params![
            uuid_v4(),
            thread_id,
            stream_version,
            event_type,
            occurred_at,
            payload.to_string(),
        ],
    )
    .map_err(|e| err(format!("append {event_type}: {e}")))?;
    Ok(())
}

pub(crate) fn session_set_event(conn: &Connection, thread_id: &str, session: &Value) -> Result<(), WsErr> {
    append_event(
        conn,
        thread_id,
        "thread.session-set",
        &iso_now_ms(),
        json!({"threadId": thread_id, "session": session}),
    )
}

pub(crate) fn set_session(
    conn: &Connection,
    thread_id: &str,
    status: &str,
    active_turn: Option<&str>,
    last_error: Option<&str>,
    provider_session_id: Option<&str>,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    conn.execute(
        "INSERT OR IGNORE INTO projection_thread_sessions \
         (thread_id, status, updated_at, runtime_mode) VALUES (?1, ?2, ?3, 'full-access')",
        params![thread_id, status, now],
    )
    .map_err(|e| err(format!("session projection insert: {e}")))?;
    let mut v: Vec<rusqlite::types::Value> = vec![
        rusqlite::types::Value::Text(status.into()),
        rusqlite::types::Value::Text(now.clone()),
        rusqlite::types::Value::Text(thread_id.into()),
    ];
    let mut sets = String::from("status = ?1, updated_at = ?2, last_activity_at = ?2");
    if let Some(t) = active_turn {
        sets.push_str(&format!(", active_turn_id = ?{}", v.len() + 1));
        v.push(if t.is_empty() {
            rusqlite::types::Value::Null
        } else {
            rusqlite::types::Value::Text(t.into())
        });
    }
    if let Some(e) = last_error {
        sets.push_str(&format!(", last_error = ?{}", v.len() + 1));
        v.push(rusqlite::types::Value::Text(e.into()));
    }
    if let Some(s) = provider_session_id {
        sets.push_str(&format!(
            ", provider_session_id = ?{}, provider_thread_id = ?{}",
            v.len() + 1,
            v.len() + 2
        ));
        v.push(rusqlite::types::Value::Text(s.into()));
        v.push(rusqlite::types::Value::Text(s.into()));
    }
    let refs: Vec<&dyn rusqlite::ToSql> = v.iter().map(|x| x as _).collect();
    conn.execute(
        &format!("UPDATE projection_thread_sessions SET {sets} WHERE thread_id = ?3"),
        refs.as_slice(),
    )
    .map_err(|e| err(format!("session projection: {e}")))?;
    Ok(())
}

pub(crate) fn set_turn_state(
    conn: &Connection,
    thread_id: &str,
    turn_id: &str,
    state: &str,
    completed: bool,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    conn.execute(
        "UPDATE projection_turns SET state = ?3, completed_at = ?4 \
         WHERE thread_id = ?1 AND turn_id = ?2",
        params![
            thread_id,
            turn_id,
            state,
            if completed { now.as_str() } else { "" }
        ],
    )
    .map_err(|e| err(format!("turn projection: {e}")))?;
    conn.execute(
        "UPDATE projection_thread_messages SET is_streaming = 0, updated_at = ?3 \
         WHERE thread_id = ?1 AND turn_id = ?2",
        params![thread_id, turn_id, now],
    )
    .map_err(|e| err(format!("message settle: {e}")))?;
    Ok(())
}

/// Append one assistant delta (or the final empty-delta settle) to the
/// projection exactly like `project_message_sent`.
pub(crate) fn append_assistant_text(
    conn: &Connection,
    thread_id: &str,
    message_id: &str,
    turn_id: &str,
    delta: &str,
    streaming: bool,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    let existing: Option<String> = conn
        .query_row(
            "SELECT text FROM projection_thread_messages \
             WHERE thread_id = ?1 AND message_id = ?2",
            params![thread_id, message_id],
            |r| r.get(0),
        )
        .ok();
    let text = match (&existing, streaming) {
        (Some(t), true) => format!("{t}{delta}"),
        (Some(t), false) => t.clone(),
        (None, _) => delta.to_string(),
    };
    conn.execute(
        "INSERT INTO projection_thread_messages \
         (message_id, thread_id, turn_id, role, text, is_streaming, created_at, updated_at, \
          attachments_json, source, starts_new_turn) \
         VALUES (?1, ?2, ?3, 'assistant', ?4, ?5, ?6, ?6, '[]', 'provider', 0) \
         ON CONFLICT(thread_id, message_id) DO UPDATE SET \
          text=excluded.text, is_streaming=excluded.is_streaming, updated_at=excluded.updated_at",
        params![message_id, thread_id, turn_id, text, streaming as i64, now],
    )
    .map_err(|e| err(format!("assistant message: {e}")))?;
    append_event(
        conn,
        thread_id,
        "thread.message-sent",
        &now,
        json!({
            "threadId": thread_id,
            "messageId": message_id,
            "role": "assistant",
            "text": delta,
            "turnId": turn_id,
            "streaming": streaming,
            "source": "provider",
            "createdAt": now,
            "updatedAt": now,
        }),
    )?;
    Ok(())
}

pub(crate) fn append_activity(
    conn: &Connection,
    thread_id: &str,
    turn_id: &str,
    tone: &str,
    kind: &str,
    summary: &str,
    payload: &Value,
) -> Result<(), WsErr> {
    let now = iso_now_ms();
    let next_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(sequence), 0) + 1 FROM projection_thread_activities \
             WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or(1);
    let activity_id = uuid_v4();
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_activities \
         (activity_id, thread_id, turn_id, tone, kind, summary, payload_json, created_at, sequence) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            activity_id,
            thread_id,
            turn_id,
            tone,
            kind,
            summary,
            payload.to_string(),
            now,
            next_seq
        ],
    )
    .map_err(|e| err(format!("activity projection: {e}")))?;
    append_event(
        conn,
        thread_id,
        "thread.activity-appended",
        &now,
        json!({
            "threadId": thread_id,
            "activity": {
                "id": activity_id,
                "tone": tone,
                "kind": kind,
                "summary": summary,
                "payload": payload,
                "turnId": turn_id,
                "sequence": next_seq,
                "createdAt": now,
            }
        }),
    )?;
    Ok(())
}

/// Stop a running turn for the thread: send `session/cancel`, so the session
/// reports a cancelled prompt instead of being killed mid-write.
pub fn cancel_turn(thread_id: &str) {
    let (stdin, session_id, flag) = match RUNNING.lock() {
        Ok(m) => match m.get(thread_id) {
            Some(rt) => (rt.stdin.clone(), rt.session_id.clone(), rt.cancelled.clone()),
            None => return,
        },
        Err(_) => return,
    };
    flag.store(true, Ordering::SeqCst);
    let cancel = json!({
        "jsonrpc": "2.0",
        "method": "session/cancel",
        "params": {"sessionId": session_id},
    });
    if let Ok(mut s) = stdin.lock() {
        let _ = s.write_all(cancel.to_string().as_bytes());
        let _ = s.write_all(b"\n");
        let _ = s.flush();
    }
}

/// A live ACP process parked between turns of the same thread. Upstream's
/// shared runtime keeps one authenticated agent process per session alive;
/// parking the child, its RPC transport and the session id lets the next
/// turn prompt the same session directly instead of respawning the CLI and
/// replaying the transcript through `session/load` every time.
struct ParkedSession {
    child: Child,
    rpc: Rpc,
    session_id: String,
    /// provider|model|runtimeMode|cwd — any change invalidates the parked
    /// process because those are spawn-fixed in the CLI invocation.
    launch_key: String,
}

static PARKED: LazyLock<Mutex<HashMap<String, ParkedSession>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn drop_parked(mut parked: ParkedSession) {
    let _ = parked.child.kill();
    let _ = parked.child.wait();
}

/// True when an ACP error means the agent requires authentication before
/// the session can be created.
fn is_auth_required_error(e: &WsErr) -> bool {
    let msg = e.0.to_lowercase();
    msg.contains("auth") || msg.contains("login") || msg.contains("unauthorized")
}

/// Pick an authentication method the agent advertised in `authMethods`.
/// Provider preferences mirror the upstream support modules (non-interactive
/// cached/token methods first); otherwise fall back to the first advertised
/// method, which is what a fresh CLI login would use.
fn select_auth_method(provider: &str, methods: &[Value]) -> Option<String> {
    let preferred: &[&str] = match provider {
        "devin" => &["cached_token", "windsurf-api-key"],
        "cursor" => &["cursor_login"],
        "antigravity" => &["gemini-api-key", "oauth-personal"],
        _ => &[],
    };
    let id_of = |m: &Value| {
        m.get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    for want in preferred {
        if let Some(found) = methods.iter().find_map(|m| {
            let id = id_of(m)?;
            if id == *want { Some(id) } else { None }
        }) {
            return Some(found);
        }
    }
    methods.iter().find_map(id_of)
}

#[derive(Clone, Debug)]
struct AcpLaunchProfile {
    display_name: &'static str,
    binary: String,
    args: Vec<String>,
    env: Vec<(&'static str, &'static str)>,
}

fn executable_override(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    })
}

fn is_executable_file(path: &std::path::Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return metadata.permissions().mode() & 0o111 != 0;
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn executable_on_path(binary: &str) -> bool {
    if binary.contains('/') {
        return is_executable_file(std::path::Path::new(binary));
    }
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| is_executable_file(&dir.join(binary)))
        })
        .unwrap_or(false)
}

/// Package-manager global bin locations used by provider CLIs when the shell
/// that launched Synara did not export the user's interactive PATH.
fn package_manager_binary_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/home/linuxbrew/.linuxbrew/bin"),
        PathBuf::from("/opt/local/bin"),
    ];
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return dirs;
    };
    for rel in [
        ".bun/bin",
        ".local/bin",
        "bin",
        ".npm-global/bin",
        ".volta/bin",
        ".asdf/shims",
        ".local/share/mise/shims",
        ".proto/shims",
        ".deno/bin",
        ".local/share/pnpm",
        "Library/pnpm",
        ".yarn/bin",
    ] {
        dirs.push(home.join(rel));
    }
    if let Ok(entries) = std::fs::read_dir(home.join(".nvm/versions/node")) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                dirs.push(entry.path().join("bin"));
            }
        }
    }
    for fnm_root in [
        home.join("Library/Application Support/fnm/node-versions"),
        home.join(".local/share/fnm/node-versions"),
    ] {
        if let Ok(entries) = std::fs::read_dir(fnm_root) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    dirs.push(entry.path().join("installation/bin"));
                }
            }
        }
    }
    if let Some(pnpm_home) = std::env::var_os("PNPM_HOME") {
        dirs.push(PathBuf::from(pnpm_home));
    }
    if let Some(prefix) = std::env::var_os("npm_config_prefix") {
        dirs.push(PathBuf::from(prefix).join("bin"));
    }
    dirs
}

fn resolve_provider_binary(
    default_binary: &str,
    override_keys: &[&str],
    home_relative_candidates: &[&str],
) -> String {
    if let Some(path) = executable_override(override_keys) {
        return path;
    }
    if executable_on_path(default_binary) {
        return default_binary.to_string();
    }
    if let Some(home) = std::env::var_os("HOME") {
        for relative in home_relative_candidates {
            let candidate = PathBuf::from(&home).join(relative);
            if is_executable_file(&candidate) {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    for dir in package_manager_binary_dirs() {
        let candidate = dir.join(default_binary);
        if is_executable_file(&candidate) {
            return candidate.to_string_lossy().into_owned();
        }
    }
    default_binary.to_string()
}

fn devin_traits_requested(model_selection: &Value) -> bool {
    model_selection.pointer("/options/reasoningEffort").and_then(Value::as_str).is_some()
        || model_selection.pointer("/options/fastMode").and_then(Value::as_bool).is_some()
        || model_selection.pointer("/options/thinking").and_then(Value::as_bool).is_some()
        || model_selection.pointer("/options/contextWindow").and_then(Value::as_str).is_some()
}

fn resolve_devin_runtime_variant(model: &str, model_selection: &Value) -> Option<String> {
    if !devin_traits_requested(model_selection) {
        return None;
    }
    let discovery = crate::provider_cache::devin_discovery_result();
    let normalized_model = model.to_ascii_lowercase();
    let family = discovery.get("models").and_then(Value::as_array)?.iter().find(|candidate| {
        let slug = candidate.get("slug").and_then(Value::as_str).unwrap_or("").to_ascii_lowercase();
        slug == normalized_model
            || candidate
                .get("modelVariants")
                .and_then(Value::as_array)
                .is_some_and(|variants| variants.iter().any(|variant| {
                    variant.get("model").and_then(Value::as_str) == Some(model)
                }))
    })?;
    let effort = model_selection.pointer("/options/reasoningEffort").and_then(Value::as_str);
    let fast = model_selection.pointer("/options/fastMode").and_then(Value::as_bool);
    let thinking = model_selection.pointer("/options/thinking").and_then(Value::as_bool);
    let context = model_selection.pointer("/options/contextWindow").and_then(Value::as_str);
    family.get("modelVariants").and_then(Value::as_array)?.iter().find_map(|variant| {
        if let Some(effort) = effort {
            if variant.get("reasoningEffort").and_then(Value::as_str) != Some(effort) {
                return None;
            }
        }
        if let Some(fast) = fast {
            if variant.get("fastMode").and_then(Value::as_bool) != Some(fast) {
                return None;
            }
        }
        if let Some(thinking) = thinking {
            if variant.get("thinking").and_then(Value::as_bool) != Some(thinking) {
                return None;
            }
        }
        if let Some(context) = context {
            if variant.get("contextWindow").and_then(Value::as_str) != Some(context) {
                return None;
            }
        }
        variant.get("model").and_then(Value::as_str).map(str::to_string)
    })
}

fn resolve_devin_selected_model(model_selection: &Value) -> Option<String> {
    let model = model_selection.get("model").and_then(Value::as_str)?.trim();
    if model.is_empty() {
        return None;
    }
    if let Some(resolved) = resolve_devin_runtime_variant(model, model_selection) {
        return Some(resolved);
    }
    let fast_mode = model_selection.pointer("/options/fastMode").and_then(Value::as_bool);
    let resolved = match (model, fast_mode) {
        ("swe-1-6", Some(true)) => "swe-1-6-fast",
        ("swe-1-7", Some(true)) => "swe-1-7-lightning",
        ("swe-1-6-fast", Some(false)) => "swe-1-6",
        ("swe-1-7-lightning", Some(false)) => "swe-1-7",
        _ => model,
    };
    Some(resolved.to_string())
}

fn selected_acp_model(provider: &str, model_selection: &Value) -> Option<String> {
    if provider == "devin" {
        return resolve_devin_selected_model(model_selection);
    }
    model_selection
        .get("model")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn model_args(provider: &str, model: Option<&str>) -> Vec<String> {
    let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) else {
        return Vec::new();
    };
    match provider {
        "devin" => vec!["--model".into(), model.into()],
        "grok" | "droid" => vec!["-m".into(), model.into()],
        _ => Vec::new(),
    }
}

fn acp_launch_profile(
    provider: &str,
    runtime_mode: &str,
    model: Option<&str>,
) -> Option<AcpLaunchProfile> {
    acp_launch_profile_ex(provider, runtime_mode, model, None, &[])
}

fn acp_launch_profile_ex(
    provider: &str,
    runtime_mode: &str,
    model: Option<&str>,
    reasoning_effort: Option<&str>,
    additional_dirs: &[String],
) -> Option<AcpLaunchProfile> {
    let (display_name, default_binary, override_keys, home_candidates, mut args, env) = match provider {
        "devin" => (
            "Devin",
            "devin",
            &["SYNARA_DEVIN_CLI", "DEVIN_BINARY_PATH"][..],
            &[".local/bin/devin", ".devin/bin/devin"][..],
            vec!["acp".to_string()],
            Vec::new(),
        ),
        "omp" => (
            "Oh My Pi",
            "omp",
            &["SYNARA_OMP_CLI", "OMP_BINARY_PATH"][..],
            &[".local/bin/omp"][..],
            vec!["acp".to_string()],
            Vec::new(),
        ),
        "cursor" => (
            "Cursor",
            "cursor-agent",
            &["SYNARA_CURSOR_CLI", "CURSOR_AGENT_BINARY_PATH", "CURSOR_BINARY_PATH"][..],
            &[".local/bin/cursor-agent"][..],
            vec!["acp".to_string()],
            vec![("NO_BROWSER", "true"), ("BROWSER", "www-browser")],
        ),
        "grok" => (
            "Grok",
            "grok",
            &["SYNARA_GROK_CLI", "GROK_BINARY_PATH"][..],
            &[".local/bin/grok"][..],
            vec![
                "--permission-mode".into(),
                "default".into(),
                "agent".into(),
                "--no-leader".into(),
            ],
            Vec::new(),
        ),
        "droid" => (
            "Factory Droid",
            "droid",
            &["SYNARA_DROID_CLI", "DROID_BINARY_PATH"][..],
            &[".local/bin/droid"][..],
            vec!["exec".into(), "--output-format".into(), "acp".into()],
            Vec::new(),
        ),
        "antigravity" => (
            "Antigravity",
            "agy",
            &["SYNARA_ANTIGRAVITY_CLI", "ANTIGRAVITY_BINARY_PATH", "AGY_BINARY_PATH"][..],
            &[".local/bin/agy"][..],
            vec!["acp".to_string()],
            Vec::new(),
        ),
        _ => return None,
    };
    if provider == "grok" && runtime_mode == "full-access" {
        args.push("--always-approve".into());
    }
    args.extend(model_args(provider, model));
    if let Some(effort) = reasoning_effort.map(str::trim).filter(|v| !v.is_empty()) {
        match provider {
            "grok" => {
                args.push("--reasoning-effort".into());
                args.push(effort.to_string());
            }
            "droid" => {
                args.push("-r".into());
                args.push(effort.to_string());
            }
            _ => {}
        }
    }
    if provider == "antigravity" {
        for dir in additional_dirs {
            args.push("--add-dir".into());
            args.push(dir.clone());
        }
    }
    if provider == "grok" {
        args.push("stdio".into());
    }
    Some(AcpLaunchProfile {
        display_name,
        binary: resolve_provider_binary(default_binary, override_keys, home_candidates),
        args,
        env,
    })
}

/// Resolve the executable path Synara would use for an ACP provider without
/// spawning it. Discovery endpoints share the same override and package-manager
/// fallback logic as turn launch.
pub fn resolve_provider_cli(provider: &str) -> Option<String> {
    acp_launch_profile_ex(provider, "full-access", None, None, &[]).map(|profile| profile.binary)
}

/// Providers currently routed through the shared ACP stdio runtime. OpenCode is
/// deliberately absent: upstream drives it through its own HTTP/SSE server and
/// SDK rather than ACP stdio.
pub fn supports_provider(provider: &str) -> bool {
    matches!(
        provider,
        "devin" | "omp" | "cursor" | "grok" | "droid" | "antigravity"
    )
}



/// Start an ACP provider before the first user prompt, authenticate/session-
/// configure it, and park the live process. This is the generic prewarm path
/// for Devin, OMP, Cursor, Grok, Droid and Antigravity.
pub fn prewarm_thread(provider: &str, thread_id: &str) -> Result<bool, WsErr> {
    if has_running_turn(thread_id) {
        return Ok(false);
    }
    if let Ok(parked_map) = PARKED.lock() {
        if parked_map.contains_key(thread_id) {
            return Ok(false);
        }
    }
    let conn = open_state_rw()?;
    let (raw_cwd, interaction_mode, model_json): (String, String, Option<String>) = conn
        .query_row(
            "SELECT COALESCE(NULLIF(t.working_directory, ''), NULLIF(t.worktree_path, ''), NULLIF(t.associated_worktree_path, ''), p.workspace_root, ''), \
                    t.interaction_mode, t.model_selection_json \
             FROM projection_threads t JOIN projection_projects p ON p.project_id=t.project_id \
             WHERE t.thread_id=?1",
            params![thread_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| err(format!("acp prewarm thread lookup: {e}")))?;
    let stored_session: Option<String> = conn
        .query_row(
            "SELECT provider_thread_id FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();
    let turn_runtime_mode: String = conn
        .query_row(
            "SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "full-access".to_string());
    let cwd = if !raw_cwd.is_empty() && std::path::Path::new(&raw_cwd).is_dir() {
        raw_cwd
    } else {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    };
    let model_value = model_json
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = selected_acp_model(provider, &model_value);
    let reasoning_effort = model_value
        .pointer("/options/reasoningEffort")
        .and_then(Value::as_str)
        .or_else(|| model_value.get("reasoningEffort").and_then(Value::as_str))
        .map(str::to_string);
    let omp_thinking_level = model_value
        .pointer("/options/thinkingLevel")
        .and_then(Value::as_str)
        .or_else(|| model_value.get("thinkingLevel").and_then(Value::as_str))
        .or(reasoning_effort.as_deref())
        .map(str::to_string);
    let additional_dirs: Vec<String> = conn
        .query_row(
            "SELECT p.additional_folders_json FROM projection_projects p \
             JOIN projection_threads t ON t.project_id = p.project_id WHERE t.thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .or_else(|| entry.get("path").and_then(Value::as_str).map(str::to_string))
        })
        .collect();
    let launch = acp_launch_profile_ex(
        provider,
        &turn_runtime_mode,
        selected_model.as_deref(),
        reasoning_effort.as_deref(),
        &additional_dirs,
    )
    .ok_or_else(|| err(format!("Provider \"{provider}\" does not support ACP stdio")))?;
    let launch_key = format!(
        "{provider}|{}|{turn_runtime_mode}|{cwd}",
        selected_model.as_deref().unwrap_or("")
    );
    let mut command = Command::new(&launch.binary);
    command
        .args(&launch.args)
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in &launch.env {
        command.env(key, value);
    }
    let mut child = command
        .spawn()
        .map_err(|e| err(format!("could not launch `{}` for {} ACP: {e}", launch.binary, launch.display_name)))?;
    let child_stdin = child.stdin.take().ok_or_else(|| err("acp stdin"))?;
    let stdout = child.stdout.take().ok_or_else(|| err("acp stdout"))?;
    let stdin = Arc::new(Mutex::new(child_stdin));
    let mut rpc = Rpc {
        stdin,
        reader: BufReader::new(stdout),
        next_id: 1,
        pending: Vec::new(),
        approval_ctx: None,
    };
    let prepared = (|| -> Result<String, WsErr> {
        let initialize_result = rpc.call(
            "initialize",
            json!({
                "protocolVersion": 1,
                "clientCapabilities": {"fs":{"readTextFile":false,"writeTextFile":false},"terminal":false},
                "clientInfo": {"name":"synara","version":"1.0.0"},
            }),
        )?;
        let auth_methods = initialize_result
            .get("authMethods")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mcp_servers = if initialize_result
            .pointer("/agentCapabilities/mcpCapabilities/http")
            .and_then(Value::as_bool)
            == Some(true)
        {
            json!([{"type":"http","name":"synara","url":crate::agent_gateway_mcp::agent_gateway_url(),"headers":[{"name":"Authorization","value":format!("Bearer {}",crate::agent_gateway_mcp::agent_gateway_token())}]}])
        } else {
            json!([])
        };
        let try_setup = |rpc: &mut Rpc| -> Result<(String, Value), WsErr> {
            if let Some(sid) = stored_session.as_deref() {
                match rpc.call("session/load", json!({"sessionId":sid,"cwd":cwd,"mcpServers":mcp_servers})) {
                    Ok(res) => return Ok((sid.to_string(), res.get("modes").cloned().unwrap_or(Value::Null))),
                    Err(e) if is_auth_required_error(&e) => return Err(e),
                    Err(_) => {}
                }
            }
            let res = rpc.call("session/new", json!({"cwd":cwd,"mcpServers":mcp_servers}))?;
            let id = res.get("sessionId").and_then(Value::as_str)
                .ok_or_else(|| err("session/new returned no sessionId"))?.to_string();
            Ok((id, res.get("modes").cloned().unwrap_or(Value::Null)))
        };
        let (session_id, mode_state) = match try_setup(&mut rpc) {
            Ok(ok) => ok,
            Err(e) if is_auth_required_error(&e) && !auth_methods.is_empty() => {
                let Some(method) = select_auth_method(provider, &auth_methods) else { return Err(e) };
                rpc.call("authenticate", json!({"methodId":method}))?;
                try_setup(&mut rpc)?
            }
            Err(e) => return Err(e),
        };
        if let Ok(Some(mode_id)) = resolve_acp_mode(provider, &mode_state, &turn_runtime_mode, &interaction_mode) {
            let _ = rpc.call("session/set_mode", json!({"sessionId":session_id,"modeId":mode_id}));
        }
        if provider == "omp" {
            apply_omp_session_configuration(
                &mut rpc,
                &session_id,
                selected_model.as_deref(),
                omp_thinking_level.as_deref(),
            )?;
        }
        Ok(session_id)
    })();
    match prepared {
        Ok(session_id) => {
            if let Ok(mut parked_map) = PARKED.lock() {
                parked_map.insert(thread_id.to_string(), ParkedSession { child, rpc, session_id, launch_key });
            }
            Ok(true)
        }
        Err(e) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(e)
        }
    }
}

fn apply_omp_session_configuration(
    rpc: &mut Rpc,
    session_id: &str,
    model: Option<&str>,
    thinking_level: Option<&str>,
) -> Result<(), WsErr> {
    if let Some(model) = model.map(str::trim).filter(|value| !value.is_empty()) {
        rpc.call(
            "session/set_config_option",
            json!({"sessionId": session_id, "configId": "model", "value": model}),
        )?;
    }
    if let Some(thinking) = thinking_level.map(str::trim).filter(|value| !value.is_empty()) {
        rpc.call(
            "session/set_config_option",
            json!({"sessionId": session_id, "configId": "thinking", "value": thinking.to_ascii_lowercase()}),
        )?;
    }
    Ok(())
}

/// Compact a warm ACP session by sending the provider-native `/compact`
/// prompt. Devin and Grok expose compaction as a normal agent prompt rather
/// than a dedicated RPC method; compaction output is intentionally left in the
/// parked RPC pending queue so it cannot contaminate the next user turn.
pub fn compact_thread(thread_id: &str) -> Result<bool, WsErr> {
    if has_running_turn(thread_id) {
        return Err(err("cannot compact while a turn is running"));
    }
    let mut parked = {
        let mut parked_map = PARKED.lock().map_err(|_| err("parked map"))?;
        match parked_map.remove(thread_id) {
            Some(parked) => parked,
            None => return Ok(false),
        }
    };
    let session_id = parked.session_id.clone();
    let result = parked
        .rpc
        .call(
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": [{"type":"text","text":"/compact"}],
            }),
        )
        .map(|_| true);
    let alive = matches!(parked.child.try_wait(), Ok(None));
    if alive {
        if let Ok(mut parked_map) = PARKED.lock() {
            parked.rpc.approval_ctx = None;
            parked_map.insert(thread_id.to_string(), parked);
        }
    } else {
        drop_parked(parked);
    }
    result
}

/// Called after a `thread.turn-start-requested` event is committed for an
/// ACP-capable provider thread. Spawns the runner thread that drives the session.
pub fn start_turn(provider: String, thread_id: String, message_id: String, text: String, cwd: String) {
    std::thread::spawn(move || {
        if let Err(e) = run_turn(&provider, &thread_id, &message_id, &text, &cwd) {
            tracing_error(&thread_id, &e);
        }
    });
}

fn tracing_error(thread_id: &str, e: &WsErr) {
    eprintln!("[acp] turn on {thread_id} failed: {}", e.0);
    if let Ok(conn) = open_state_rw() {
        let _ = set_session(&conn, thread_id, "error", Some(""), Some(&e.0), None);
        let _ = session_set_event(
            &conn,
            thread_id,
            &json!({
                "threadId": thread_id,
                "status": "error",
                "lastError": e.0,
                "updatedAt": iso_now_ms(),
            }),
        );
    }
}

struct Rpc {
    stdin: Arc<Mutex<ChildStdin>>,
    reader: BufReader<std::process::ChildStdout>,
    next_id: i64,
    pending: Vec<Value>,
    /// Thread/turn the prompt runs for; enables upstream approval gating
    /// on `session/request_permission`.
    approval_ctx: Option<ApprovalCtx>,
}

#[derive(Clone)]
struct ApprovalCtx {
    thread_id: String,
    turn_id: String,
    cancelled: Arc<AtomicBool>,
    runtime_mode: String,
    interaction_mode: String,
    tool_calls: Arc<Mutex<HashMap<String, Value>>>,
}

fn approval_request_kind(acp_kind: &str) -> &'static str {
    match acp_kind {
        "execute" => "command",
        "read" => "file-read",
        "edit" | "delete" | "move" => "file-change",
        _ => "tool",
    }
}

fn merge_tracked_tool_call(request: &mut Value, tracked: &Value) {
    let (Some(request_obj), Some(tracked_obj)) =
        (request.as_object_mut(), tracked.as_object())
    else {
        return;
    };
    let tracked_kind = tracked_obj.get("kind").and_then(Value::as_str);
    let request_kind = request_obj.get("kind").and_then(Value::as_str);
    if tracked_kind == Some("execute")
        || request_kind.is_none()
        || request_kind == Some("unknown")
    {
        if let Some(kind) = tracked_kind {
            request_obj.insert("kind".to_string(), json!(kind));
        }
    }
    let request_title = request_obj.get("title").and_then(Value::as_str);
    if request_title.is_none() || matches!(request_title, Some("Tool" | "Tool call")) {
        if let Some(title) = tracked_obj.get("title") {
            request_obj.insert("title".to_string(), title.clone());
        }
    }
    for key in ["rawInput", "rawOutput", "content", "locations"] {
        if !request_obj.contains_key(key) {
            if let Some(value) = tracked_obj.get(key) {
                request_obj.insert(key.to_string(), value.clone());
            }
        }
    }
}

impl Rpc {
    fn write_raw(&self, msg: &Value) -> Result<(), WsErr> {
        let mut s = self.stdin.lock().map_err(|_| err("acp stdin lock"))?;
        s.write_all(msg.to_string().as_bytes())
            .and_then(|_| s.write_all(b"\n"))
            .and_then(|_| s.flush())
            .map_err(|e| err(format!("acp write: {e}")))
    }

    fn read_msg(&mut self) -> Result<Value, WsErr> {
        loop {
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .map_err(|e| err(format!("acp read: {e}")))?;
            if n == 0 {
                return Err(err("acp provider closed stdout"));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            match serde_json::from_str::<Value>(trimmed) {
                Ok(v) => return Ok(v),
                Err(_) => continue,
            }
        }
    }

    fn call(&mut self, method: &str, params: Value) -> Result<Value, WsErr> {
        let id = self.next_id;
        self.next_id += 1;
        self.write_raw(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        loop {
            let v = self.read_msg()?;
            if v.get("id").and_then(Value::as_i64) == Some(id)
                && (v.get("result").is_some() || v.get("error").is_some())
            {
                if let Some(e) = v.get("error") {
                    return Err(err(format!(
                        "acp {method} error: {}",
                        e.get("message").and_then(Value::as_str).unwrap_or("?")
                    )));
                }
                return Ok(v.get("result").cloned().unwrap_or(Value::Null));
            }
            self.pending.push(v);
        }
    }

    /// Read messages until the response to the call arrives, streaming
    /// notifications to `on_notify`. Returns the result value.
    fn call_streaming<F>(
        &mut self,
        method: &str,
        params: Value,
        cancelled: &Arc<AtomicBool>,
        mut on_notify: F,
    ) -> Result<Value, WsErr>
    where
        F: FnMut(&Value),
    {
        let id = self.next_id;
        self.next_id += 1;
        self.write_raw(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}))?;
        // `pending` holds notifications received while an earlier call waited
        // for its response — e.g. the transcript replay `session/load` emits.
        // Answer any queued server→client requests but drop the notifications:
        // replayed chunks must not leak into the new turn's message.
        for v in std::mem::take(&mut self.pending) {
            if v.get("method").is_some() && v.get("id").is_some() {
                self.dispatch_incoming(&v, &mut on_notify);
            }
        }
        loop {
            let v = self.read_msg()?;
            if v.get("id").and_then(Value::as_i64) == Some(id)
                && (v.get("result").is_some() || v.get("error").is_some())
            {
                if let Some(e) = v.get("error") {
                    if cancelled.load(Ordering::SeqCst) {
                        return Ok(json!({"stopReason": "cancelled"}));
                    }
                    return Err(err(format!(
                        "acp {method} error: {}",
                        e.get("message").and_then(Value::as_str).unwrap_or("?")
                    )));
                }
                return Ok(v.get("result").cloned().unwrap_or(Value::Null));
            }
            self.dispatch_incoming(&v, &mut on_notify);
        }
    }

    /// Server→client requests get a reply; notifications go to the streamer.
    fn dispatch_incoming<F>(&mut self, v: &Value, on_notify: &mut F)
    where
        F: FnMut(&Value),
    {
        let method = v.get("method").and_then(Value::as_str).unwrap_or("");
        match method {
            "session/request_permission" => {
                // Upstream gates these behind an approval card except in
                // full-access mode, which resolves them automatically
                // (resolveAcpFullAccessPermissionOutcome). With a turn
                // context the request becomes a pending interaction and
                // the runner blocks on the user's decision; without one
                // (replay) fall back to auto-approving.
                if let Some(ctx) = self.approval_ctx.clone() {
                    if ctx.runtime_mode == "full-access" {
                        self.reply_auto(v);
                    } else {
                        self.handle_permission(v, &ctx);
                    }
                } else {
                    self.reply_auto(v);
                }
            }
            "fs/read_text_file" | "fs/write_text_file" | "terminal/create"
            | "terminal/output" | "terminal/wait_for_exit" | "terminal/kill"
            | "terminal/release" => {
                if let Some(rid) = v.get("id").cloned() {
                    let _ = self.write_raw(&json!({
                        "jsonrpc": "2.0",
                        "id": rid,
                        "error": {"code": -32601, "message": "not supported by synara host"},
                    }));
                }
            }
            _ => on_notify(v),
        }
    }

    /// Reply to a request_permission picking the most permissive option —
    /// upstream's full-access auto-approval and the replay fallback.
    fn reply_auto(&mut self, v: &Value) {
        if let Some(rid) = v.get("id").cloned() {
            let option_id = auto_pick_option(v)
                .map(|o| o.get("optionId").cloned().unwrap_or(Value::Null))
                .unwrap_or(Value::Null);
            let _ = self.write_raw(&json!({
                "jsonrpc": "2.0",
                "id": rid,
                "result": {"outcome": {"outcome": "selected", "optionId": option_id}},
            }));
        }
    }

    /// Upstream approval gating (pendingInteractions + approval.requested/
    /// approval.resolved activities): persist the request, then block until
    /// the user's `thread.approval.respond` decision lands on the row, the
    /// turn is cancelled, or the wait times out. Declined/cancelled turns
    /// reply ACP `outcome: "cancelled"` unless a reject option exists.
    fn handle_permission(&mut self, v: &Value, ctx: &ApprovalCtx) {
        let params = v.get("params").cloned().unwrap_or(Value::Null);
        let mut tool_call = params.get("toolCall").cloned().unwrap_or(Value::Null);
        if let Some(tool_call_id) = tool_call
            .get("toolCallId")
            .and_then(Value::as_str)
            .map(str::to_string)
        {
            if let Ok(tool_calls) = ctx.tool_calls.lock() {
                if let Some(tracked) = tool_calls.get(&tool_call_id) {
                    merge_tracked_tool_call(&mut tool_call, tracked);
                }
            }
        }
        let request_id = tool_call
            .get("toolCallId")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(uuid_v4);
        let title = tool_call
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("Tool call")
            .to_string();
        let options = params
            .get("options")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Ok(c) = open_state_rw() {
            let _ = c.execute_batch("PRAGMA busy_timeout = 5000;");
            let now = iso_now_ms();
            if let Err(e) = c.execute(
                "INSERT OR REPLACE INTO projection_pending_interactions \
                 (interaction_kind, request_id, thread_id, turn_id, \
                  lifecycle_generation, status, decision, \
                  response_command_id, response_requested_at, \
                  created_at, resolved_at) \
                 VALUES ('approval', ?1, ?2, ?3, NULL, 'pending', NULL, \
                  NULL, NULL, ?4, NULL)",
                params![request_id, ctx.thread_id, ctx.turn_id, now],
            ) {
                eprintln!("acp_runner: pending approval insert: {e}");
            }
            let payload = json!({
                "requestId": request_id,
                "requestKind": approval_request_kind(
                    tool_call.get("kind").and_then(Value::as_str).unwrap_or(""),
                ),
                "toolName": title,
                "detail": title,
                "sessionApprovalAvailable": true,
            });
            if let Err(e) = append_activity(
                &c,
                &ctx.thread_id,
                &ctx.turn_id,
                "warn",
                "approval.requested",
                "Approval requested",
                &payload,
            ) {
                eprintln!("acp_runner: approval activity: {}", e.0);
            }
        }
        let deadline =
            std::time::Instant::now() + std::time::Duration::from_secs(600);
        let mut decision: Option<String> = None;
        while decision.is_none() {
            if ctx.cancelled.load(Ordering::SeqCst)
                || std::time::Instant::now() >= deadline
            {
                break;
            }
            if let Ok(c) = open_state_rw() {
                match c.query_row(
                    "SELECT decision FROM projection_pending_interactions \
                     WHERE thread_id=?1 AND request_id=?2 \
                     AND interaction_kind='approval'",
                    params![ctx.thread_id, request_id],
                    |r| r.get::<_, Option<String>>(0),
                ) {
                    Ok(d) => {
                        if let Some(d) = d {
                            decision = Some(d);
                        }
                    }
                    // Row gone without a decision — treat as cancelled.
                    Err(_) => decision = Some("cancel".to_string()),
                }
            }
            if decision.is_none() {
                std::thread::sleep(std::time::Duration::from_millis(250));
            }
        }
        let outcome = match decision.as_deref() {
            Some("accept") | Some("acceptForSession") => {
                let want = if decision.as_deref() == Some("acceptForSession") {
                    "allow_always"
                } else {
                    "allow_once"
                };
                options
                    .iter()
                    .find(|o| {
                        o.get("kind")
                            .and_then(Value::as_str)
                            .map(|k| k.contains(want))
                            .unwrap_or(false)
                    })
                    .or_else(|| {
                        options.iter().find(|o| {
                            o.get("kind")
                                .and_then(Value::as_str)
                                .map(|k| k.contains("allow"))
                                .unwrap_or(false)
                        })
                    })
                    .or_else(|| options.first())
                    .and_then(|o| o.get("optionId").cloned())
                    .map(|oid| json!({"outcome": "selected", "optionId": oid}))
                    .unwrap_or(json!({"outcome": "cancelled"}))
            }
            Some("decline") | Some("cancel") => options
                .iter()
                .find(|o| {
                    o.get("kind")
                        .and_then(Value::as_str)
                        .map(|k| k.contains("reject"))
                        .unwrap_or(false)
                })
                .and_then(|o| o.get("optionId").cloned())
                .map(|oid| json!({"outcome": "selected", "optionId": oid}))
                .unwrap_or(json!({"outcome": "cancelled"})),
            // Timeout or cancelled turn: keep the unattended behavior and
            // auto-pick an allow option so the prompt is not stranded.
            _ => options
                .iter()
                .find(|o| {
                    o.get("kind")
                        .and_then(Value::as_str)
                        .map(|k| k.contains("allow"))
                        .unwrap_or(false)
                })
                .or_else(|| options.first())
                .and_then(|o| o.get("optionId").cloned())
                .map(|oid| json!({"outcome": "selected", "optionId": oid}))
                .unwrap_or(json!({"outcome": "cancelled"})),
        };
        if let Ok(c) = open_state_rw() {
            let _ = c.execute(
                "DELETE FROM projection_pending_interactions \
                 WHERE thread_id=?1 AND request_id=?2 \
                 AND interaction_kind='approval'",
                params![ctx.thread_id, request_id],
            );
            let payload = json!({
                "requestId": request_id,
                "decision": decision,
            });
            if let Err(e) = append_activity(
                &c,
                &ctx.thread_id,
                &ctx.turn_id,
                "info",
                "approval.resolved",
                "Approval resolved",
                &payload,
            ) {
                eprintln!("acp_runner: approval resolved activity: {}", e.0);
            }
        }
        if let Some(rid) = v.get("id").cloned() {
            let _ = self.write_raw(&json!({
                "jsonrpc": "2.0",
                "id": rid,
                "result": {"outcome": outcome},
            }));
        }
    }
}

/// Fallback pick for replayed permission requests: the first "allow"
/// option so replay does not strand on a request that was already
/// answered in the original session.
fn auto_pick_option(v: &Value) -> Option<Value> {
    let options = v
        .pointer("/params/options")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    options
        .iter()
        .find(|o| {
            o.get("kind")
                .and_then(Value::as_str)
                .map(|k| k.contains("allow_always"))
                .unwrap_or(false)
        })
        .or_else(|| {
            options.iter().find(|o| {
                o.get("kind")
                    .and_then(Value::as_str)
                    .map(|k| k.contains("allow_once"))
                    .unwrap_or(false)
            })
        })
        .or_else(|| options.first())
        .cloned()
}

// Port of upstream resolveRequestedModeId (DevinAdapter): pick the ACP
// session mode matching the thread's runtime/interaction mode.
// plan -> ["plan", "architect"], approval-required -> ["accept-edits",
// "code"] with an "ask everytime" fallback, otherwise -> ["bypass",
// "full access"]. Exact normalized match first, then whole-token;
// plan only ever takes the exact match (fail-closed).
fn acp_mode_norm(s: &str) -> String {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn acp_mode_matches<'a>(modes: &'a [Value], aliases: &[&str], whole_token: bool) -> Option<&'a Value> {
    modes.iter().find(|m| {
        let id = acp_mode_norm(m.get("id").and_then(Value::as_str).unwrap_or(""));
        let name = acp_mode_norm(m.get("name").and_then(Value::as_str).unwrap_or(""));
        aliases.iter().any(|a| {
            let a = acp_mode_norm(a);
            if id == a || name == a {
                return true;
            }
            whole_token
                && a
                    .split(' ')
                    .all(|tok| id.split(' ').any(|t| t == tok) || name.split(' ').any(|t| t == tok))
        })
    })
}

fn resolve_acp_mode(
    provider: &str,
    mode_state: &Value,
    runtime_mode: &str,
    interaction_mode: &str,
) -> Result<Option<String>, String> {
    if runtime_mode == "auto" {
        return Err(
            format!(
                "Provider \"{provider}\" does not support Auto runtime mode. Auto is available only for Codex and Claude Code."
            ),
        );
    }
    let modes = mode_state
        .get("availableModes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let current = mode_state
        .get("currentModeId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let required_by = if interaction_mode == "plan" {
        Some("plan interaction mode")
    } else if runtime_mode == "approval-required" {
        Some("runtime mode \"approval-required\"")
    } else {
        None
    };
    if modes.is_empty() {
        return match required_by {
            Some(r) => Err(format!(
                "Requested {r} requires the ACP session to expose modes, but none were reported."
            )),
            None => Ok(None),
        };
    }
    let target = if interaction_mode == "plan" {
        acp_mode_matches(&modes, &["plan", "architect"], false)
    } else if runtime_mode == "approval-required" {
        acp_mode_matches(&modes, &["accept-edits", "code"], true).or_else(|| {
            acp_mode_matches(&modes, &["ask everytime", "ask every time"], true)
        })
    } else {
        acp_mode_matches(&modes, &["bypass", "full access"], true)
    };
    match target {
        Some(m) => {
            let id = m.get("id").and_then(Value::as_str).unwrap_or("").to_string();
            if id == current {
                Ok(None)
            } else {
                Ok(Some(id))
            }
        }
        None => match required_by {
            Some(r) => Err(format!(
                "Requested {r} does not match any available ACP mode. Available modes: {}",
                modes
                    .iter()
                    .map(|m| format!(
                        "{} ({})",
                        m.get("id").and_then(Value::as_str).unwrap_or("?"),
                        m.get("name").and_then(Value::as_str).unwrap_or("?")
                    ))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            None => Ok(None),
        },
    }
}

fn run_turn(provider: &str, thread_id: &str, message_id: &str, text: &str, cwd: &str) -> Result<(), WsErr> {
    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;

    let stored_session: Option<String> = conn
        .query_row(
            "SELECT provider_thread_id FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .ok()
        .flatten();

    let turn_id = uuid_v4();
    let assistant_msg_id = uuid_v4();
    let now = iso_now_ms();
    conn.execute(
        "INSERT INTO projection_turns \
         (thread_id, turn_id, pending_message_id, assistant_message_id, state, \
          requested_at, started_at, checkpoint_files_json) \
         VALUES (?1, ?2, ?3, ?4, 'running', ?5, ?5, '[]')",
        params![thread_id, turn_id, message_id, assistant_msg_id, now],
    )
    .map_err(|e| err(format!("insert turn: {e}")))?;
    // The turn-start event already persisted the command's runtimeMode on
    // the session row; reuse it for every session-set event this turn
    // emits so replay_session_projection keeps the value (an event
    // without runtimeMode would erase the column).
    let turn_runtime_mode: String = conn
        .query_row(
            "SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get(0),
        )
        .unwrap_or_else(|_| "full-access".to_string());
    conn.execute(
        "INSERT OR REPLACE INTO projection_thread_sessions \
         (thread_id, status, provider_name, provider_session_id, provider_thread_id, \
          active_turn_id, last_error, updated_at, runtime_mode) \
         VALUES (?1, 'running', ?5, NULL, ?2, ?3, NULL, ?4, \
          COALESCE((SELECT runtime_mode FROM projection_thread_sessions WHERE thread_id = ?1), 'full-access'))",
        params![thread_id, stored_session, turn_id, now, provider],
    )
    .map_err(|e| err(format!("session start: {e}")))?;
    session_set_event(
        &conn,
        thread_id,
        &json!({
            "threadId": thread_id,
            "status": "running",
            "providerName": provider,
            "activeTurnId": turn_id,
            "runtimeMode": turn_runtime_mode,
            "lastError": null,
            "updatedAt": now,
        }),
    )?;

    let cwd = if !cwd.is_empty() && !std::path::Path::new(&cwd).is_dir() {
        eprintln!("acp_runner: workspace missing on disk: {cwd} — falling back to home");
        std::env::var("HOME").unwrap_or_default()
    } else {
        cwd.to_string()
    };
    let cwd = if cwd.is_empty() {
        std::env::var("HOME").unwrap_or_else(|_| "/".to_string())
    } else {
        cwd
    };
    let model_value: Value = conn
        .query_row(
            "SELECT model_selection_json FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or(Value::Null);
    let selected_model = selected_acp_model(provider, &model_value);
    let reasoning_effort: Option<String> = model_value
        .pointer("/options/reasoningEffort")
        .and_then(Value::as_str)
        .or_else(|| model_value.get("reasoningEffort").and_then(Value::as_str))
        .map(str::to_string);
    let omp_thinking_level = model_value
        .pointer("/options/thinkingLevel")
        .and_then(Value::as_str)
        .or_else(|| model_value.get("thinkingLevel").and_then(Value::as_str))
        .or(reasoning_effort.as_deref())
        .map(str::to_string);
    let additional_dirs: Vec<String> = conn
        .query_row(
            "SELECT p.additional_folders_json FROM projection_projects p              JOIN projection_threads t ON t.project_id = p.project_id              WHERE t.thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, Option<String>>(0),
        )
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|v| v.as_array().cloned())
        .unwrap_or_default()
        .iter()
        .filter_map(|entry| {
            entry
                .as_str()
                .map(str::to_string)
                .or_else(|| entry.get("path").and_then(Value::as_str).map(str::to_string))
        })
        .collect();
    let launch = acp_launch_profile_ex(
        provider,
        &turn_runtime_mode,
        selected_model.as_deref(),
        reasoning_effort.as_deref(),
        &additional_dirs,
    )
    .ok_or_else(|| err(format!("Provider \"{provider}\" does not support ACP stdio")))?;
    let launch_key = format!(
        "{provider}|{}|{turn_runtime_mode}|{cwd}",
        selected_model.as_deref().unwrap_or("")
    );

    // Reuse the live process parked by the previous turn of this thread
    // when it is still alive and the spawn-fixed inputs are unchanged.
    let parked = PARKED
        .lock()
        .ok()
        .and_then(|mut map| map.remove(thread_id))
        .and_then(|mut p| {
            let alive = matches!(p.child.try_wait(), Ok(None));
            if alive && p.launch_key == launch_key {
                Some(p)
            } else {
                if alive {
                    eprintln!(
                        "acp_runner: discarding parked {provider} session \
                         (launch inputs changed)"
                    );
                }
                drop_parked(p);
                None
            }
        });

    let cancelled = Arc::new(AtomicBool::new(false));
    let interaction_mode = conn
        .query_row(
            "SELECT interaction_mode FROM projection_threads WHERE thread_id = ?1",
            params![thread_id],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "default".to_string());

    let (mut child, mut rpc, stdin, reused_session_id) = match parked {
        Some(mut p) => {
            let stdin = p.rpc.stdin.clone();
            p.rpc.approval_ctx = Some(ApprovalCtx {
                thread_id: thread_id.to_string(),
                turn_id: turn_id.clone(),
                cancelled: cancelled.clone(),
                runtime_mode: turn_runtime_mode.clone(),
                interaction_mode: interaction_mode.clone(),
                tool_calls: Arc::new(Mutex::new(HashMap::new())),
            });
            let session_id = p.session_id.clone();
            (p.child, p.rpc, stdin, Some(session_id))
        }
        None => {
            let mut command = Command::new(&launch.binary);
            command
                .args(&launch.args)
                .current_dir(&cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null());
            for (key, value) in &launch.env {
                command.env(key, value);
            }
            let spawn_result = command.spawn();
            let mut child: Child = match spawn_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
                    let msg = format!(
                        "could not launch `{}` for {} ACP: {e}",
                        launch.binary, launch.display_name
                    );
                    let _ = set_session(&conn, thread_id, "error", Some(""), Some(&msg), None);
                    return Err(err(msg));
                }
            };
            let child_stdin = child.stdin.take().ok_or_else(|| err("acp stdin"))?;
            let stdout = child.stdout.take().ok_or_else(|| err("acp stdout"))?;
            let stdin = Arc::new(Mutex::new(child_stdin));
            let rpc = Rpc {
                stdin: stdin.clone(),
                reader: BufReader::new(stdout),
                next_id: 1,
                pending: Vec::new(),
                approval_ctx: Some(ApprovalCtx {
                    thread_id: thread_id.to_string(),
                    turn_id: turn_id.clone(),
                    cancelled: cancelled.clone(),
                    runtime_mode: turn_runtime_mode.clone(),
                    interaction_mode: interaction_mode.clone(),
                    tool_calls: Arc::new(Mutex::new(HashMap::new())),
                }),
            };
            (child, rpc, stdin, None)
        }
    };

    let mut session_id_out = String::new();
    let result = (|| -> Result<String, WsErr> {
        let session_id = if let Some(sid) = reused_session_id.clone() {
            // The parked process already completed initialize/authenticate
            // and session setup; prompt the live session directly.
            sid
        } else {
            let initialize_result = rpc.call(
                "initialize",
                json!({
                    "protocolVersion": 1,
                    "clientCapabilities": {
                        "fs": {"readTextFile": false, "writeTextFile": false},
                        "terminal": false,
                    },
                    "clientInfo": {"name": "synara", "version": "1.0.0"},
                }),
            )?;
            let auth_methods = initialize_result
                .get("authMethods")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mcp_servers = if initialize_result
                .pointer("/agentCapabilities/mcpCapabilities/http")
                .and_then(Value::as_bool)
                == Some(true)
            {
                json!([{
                    "type": "http",
                    "name": "synara",
                    "url": crate::agent_gateway_mcp::agent_gateway_url(),
                    "headers": [{
                        "name": "Authorization",
                        "value": format!(
                            "Bearer {}",
                            crate::agent_gateway_mcp::agent_gateway_token()
                        ),
                    }],
                }])
            } else {
                json!([])
            };

            // Session setup: load the stored provider session when one
            // exists, otherwise create a fresh one. An auth-required failure
            // triggers one ACP `authenticate` round with the provider's
            // preferred method and a single retry, mirroring the upstream
            // runtime's authentication phase.
            let try_setup = |rpc: &mut Rpc| -> Result<(String, Value), WsErr> {
                if let Some(sid) = stored_session.as_deref() {
                    match rpc.call(
                        "session/load",
                        json!({"sessionId": sid, "cwd": cwd, "mcpServers": mcp_servers}),
                    ) {
                        Ok(res) => {
                            let mode_state = res.get("modes").cloned().unwrap_or(Value::Null);
                            return Ok((sid.to_string(), mode_state));
                        }
                        Err(e) => {
                            if is_auth_required_error(&e) {
                                return Err(e);
                            }
                            let res = rpc.call(
                                "session/new",
                                json!({"cwd": cwd, "mcpServers": mcp_servers}),
                            )?;
                            let mode_state = res.get("modes").cloned().unwrap_or(Value::Null);
                            let id = res
                                .get("sessionId")
                                .and_then(Value::as_str)
                                .ok_or_else(|| err("session/new returned no sessionId"))?
                                .to_string();
                            return Ok((id, mode_state));
                        }
                    }
                }
                let res = rpc.call("session/new", json!({"cwd": cwd, "mcpServers": mcp_servers}))?;
                let mode_state = res.get("modes").cloned().unwrap_or(Value::Null);
                let id = res
                    .get("sessionId")
                    .and_then(Value::as_str)
                    .ok_or_else(|| err("session/new returned no sessionId"))?
                    .to_string();
                Ok((id, mode_state))
            };
            let (session_id, mode_state) = match try_setup(&mut rpc) {
                Ok(ok) => ok,
                Err(e) if is_auth_required_error(&e) && !auth_methods.is_empty() => {
                    let Some(method) = select_auth_method(provider, &auth_methods) else {
                        return Err(e);
                    };
                    rpc.call("authenticate", json!({"methodId": method}))?;
                    try_setup(&mut rpc)?
                }
                Err(e) => return Err(e),
            };

            // Upstream applyDevinSessionConfiguration: translate the thread's
            // runtime/interaction mode into an ACP session mode before the
            // first prompt (plan -> plan/architect, approval-required ->
            // accept-edits/code/"ask everytime", otherwise bypass/full access).
            let ctx_mode = rpc
                .approval_ctx
                .as_ref()
                .map(|c| (c.runtime_mode.clone(), c.interaction_mode.clone()))
                .unwrap_or_default();
            match resolve_acp_mode(provider, &mode_state, &ctx_mode.0, &ctx_mode.1) {
                Ok(Some(mode_id)) => {
                    if let Err(e) = rpc.call(
                        "session/set_mode",
                        json!({"sessionId": session_id, "modeId": mode_id}),
                    ) {
                        eprintln!("acp_runner: session/set_mode failed: {}", e.0);
                    }
                }
                Ok(None) => {}
                Err(issue) => {
                    let _ = set_turn_state(&conn, thread_id, &turn_id, "error", true);
                    let _ = set_session(&conn, thread_id, "error", Some(""), Some(&issue), None);
                    return Err(err(issue));
                }
            }
            if provider == "omp" {
                apply_omp_session_configuration(
                    &mut rpc,
                    &session_id,
                    selected_model.as_deref(),
                    omp_thinking_level.as_deref(),
                )?;
            }
            session_id
        };
        session_id_out = session_id.clone();

        {
            let mut map = RUNNING.lock().map_err(|_| err("running map"))?;
            map.insert(
                thread_id.to_string(),
                RunningTurn {
                    stdin: stdin.clone(),
                    session_id: session_id.clone(),
                    cancelled: cancelled.clone(),
                },
            );
        }

        set_session(
            &conn,
            thread_id,
            "running",
            Some(&turn_id),
            None,
            Some(&session_id),
        )?;

        let mut text_buf = String::new();
        let mut last_flush = std::time::Instant::now();
        let tid = thread_id.to_string();
        let amsg = assistant_msg_id.clone();
        let t_id = turn_id.clone();
        let approval_tool_calls = rpc
            .approval_ctx
            .as_ref()
            .map(|ctx| ctx.tool_calls.clone());
        let attachments = crate::provider_attachments::message_attachments(thread_id, message_id)?;
        let prompt_text = crate::provider_attachments::append_file_attachments_prompt_block(
            text,
            &attachments,
            false,
        );
        let mut prompt_parts = Vec::new();
        if !prompt_text.trim().is_empty() {
            prompt_parts.push(json!({"type": "text", "text": prompt_text}));
        }
        prompt_parts.extend(crate::provider_attachments::image_blocks(&attachments)?);
        if prompt_parts.is_empty() {
            return Err(err("turn requires non-empty text or attachments"));
        }
        let result = rpc.call_streaming(
            "session/prompt",
            json!({
                "sessionId": session_id,
                "prompt": prompt_parts,
            }),
            &cancelled,
            |v| {
                if v.get("method").and_then(Value::as_str) != Some("session/update") {
                    return;
                }
                let update = v
                    .pointer("/params/update")
                    .cloned()
                    .unwrap_or(Value::Null);
                let kind = update
                    .get("sessionUpdate")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match kind {
                    "agent_message_chunk" => {
                        if let Some(t) =
                            update.pointer("/content/text").and_then(Value::as_str)
                        {
                            text_buf.push_str(t);
                        }
                    }
                    "tool_call" => {
                        if let (Some(tool_calls), Some(tool_call_id)) = (
                            approval_tool_calls.as_ref(),
                            update.get("toolCallId").and_then(Value::as_str),
                        ) {
                            if let Ok(mut map) = tool_calls.lock() {
                                map.insert(tool_call_id.to_string(), update.clone());
                            }
                        }
                        let title = update
                            .get("title")
                            .and_then(Value::as_str)
                            .unwrap_or("Tool call");
                        let kind_name = update
                            .get("kind")
                            .and_then(Value::as_str)
                            .unwrap_or("tool");
                        if let Ok(c) = open_state_rw() {
                            if let Err(e) = append_activity(&c, &tid, &t_id, "tool", kind_name, title, &update) { eprintln!("acp_runner: activity failed: {}", e.0); }
                        }
                    }
                    "plan" => {
                        // ACP plan snapshot → upstream `turn.tasks.updated`
                        // activity ({tasks:[{task,status}]}); statuses map
                        // pending/in_progress/completed → pending/inProgress/
                        // completed. Snapshots are cumulative, one row per turn.
                        if let Some(entries) =
                            update.get("entries").and_then(Value::as_array)
                        {
                            let tasks: Vec<Value> = entries
                                .iter()
                                .filter_map(|e| {
                                    let task = e
                                        .get("content")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string();
                                    if task.is_empty() {
                                        return None;
                                    }
                                    let status = match e
                                        .get("status")
                                        .and_then(Value::as_str)
                                        .unwrap_or("pending")
                                    {
                                        "in_progress" => "inProgress",
                                        "completed" => "completed",
                                        _ => "pending",
                                    };
                                    Some(json!({ "task": task, "status": status }))
                                })
                                .collect();
                            let payload = json!({ "tasks": tasks });
                            if let Ok(c) = open_state_rw() {
                                if let Err(e) = append_activity(&c, &tid, &t_id, "info", "turn.tasks.updated", "Task list updated", &payload) { eprintln!("acp_runner: plan activity failed: {}", e.0); }
                            }
                        }
                    }
                    "tool_call_update" => {
                        let status = update
                            .get("status")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if status == "failed" {
                            let title = update
                                .get("title")
                                .and_then(Value::as_str)
                                .unwrap_or("Tool call failed");
                            if let Ok(c) = open_state_rw() {
                                if let Err(e) = append_activity(&c, &tid, &t_id, "error", "tool", title, &update) { eprintln!("acp_runner: error activity failed: {}", e.0); }
                            }
                        }
                    }
                    _ => {}
                }
                if !text_buf.is_empty() && last_flush.elapsed().as_millis() >= 200 {
                    if let Ok(c) = open_state_rw() {
                        if let Err(e) = append_assistant_text(&c, &tid, &amsg, &t_id, &text_buf, true) { eprintln!("acp_runner: assistant flush failed: {}", e.0); }
                    }
                    text_buf.clear();
                    last_flush = std::time::Instant::now();
                }
            },
        )?;
        if let Ok(c) = open_state_rw() {
            if !text_buf.is_empty() {
                if let Err(e) = append_assistant_text(&c, &tid, &amsg, &t_id, &text_buf, true) { eprintln!("acp_runner: assistant flush failed: {}", e.0); }
            }
            if let Err(e) = append_assistant_text(&c, &tid, &amsg, &t_id, "", false) { eprintln!("acp_runner: final settle failed: {}", e.0); }
        }
        Ok(result
            .get("stopReason")
            .and_then(Value::as_str)
            .unwrap_or("end_turn")
            .to_string())
    })();

    if let Ok(mut map) = RUNNING.lock() {
        map.remove(thread_id);
    }
    let cancelled_flag = cancelled.load(Ordering::SeqCst);

    // Park the live process for the next turn when the prompt completed
    // normally: upstream keeps one authenticated agent process per session
    // alive, so the next turn prompts the same session instead of paying
    // spawn + initialize + authenticate + transcript replay again. On
    // errors or interruptions the session state is uncertain, so the
    // process is torn down and the next turn falls back to session/load.
    let parkable = matches!(&result, Ok(reason) if reason != "cancelled")
        && !cancelled_flag
        && !session_id_out.is_empty()
        && matches!(child.try_wait(), Ok(None));
    if parkable {
        if let Ok(mut map) = PARKED.lock() {
            map.insert(
                thread_id.to_string(),
                ParkedSession {
                    child,
                    rpc,
                    session_id: session_id_out.clone(),
                    launch_key,
                },
            );
        } else {
            let _ = child.kill();
            let _ = child.wait();
        }
    } else {
        let _ = child.kill();
        let _ = child.wait();
    }

    let conn = open_state_rw()?;
    conn.execute_batch("PRAGMA busy_timeout = 5000;")
        .map_err(|e| err(e.to_string()))?;
    match result {
        Ok(reason) => {
            let interrupted = cancelled_flag || reason == "cancelled";
            let state = if interrupted { "interrupted" } else { "completed" };
            let status = if interrupted { "interrupted" } else { "ready" };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), None, None)?;
            session_set_event(
                &conn,
                thread_id,
                &json!({
                    "threadId": thread_id,
                    "status": status,
                    "providerName": provider,
                    "activeTurnId": null,
                    "runtimeMode": turn_runtime_mode,
                    "lastError": null,
                    "updatedAt": iso_now_ms(),
                }),
            )?;
            crate::orchestration_engine::continue_queued_turn(thread_id);
            Ok(())
        }
        Err(e) => {
            let (state, status) = if cancelled_flag {
                ("interrupted", "interrupted")
            } else {
                ("error", "error")
            };
            set_turn_state(&conn, thread_id, &turn_id, state, true)?;
            set_session(&conn, thread_id, status, Some(""), Some(&e.0), None)?;
            session_set_event(
                &conn,
                thread_id,
                &json!({
                    "threadId": thread_id,
                    "status": status,
                    "providerName": provider,
                    "activeTurnId": null,
                    "runtimeMode": turn_runtime_mode,
                    "lastError": e.0,
                    "updatedAt": iso_now_ms(),
                }),
            )?;
            if cancelled_flag {
                crate::orchestration_engine::continue_queued_turn(thread_id);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devin_auto_runtime_mode_fails_closed() {
        let modes = json!({
            "availableModes": [
                {"id": "bypass", "name": "Bypass"},
                {"id": "plan", "name": "Plan"},
            ],
            "currentModeId": "bypass",
        });
        let err = resolve_acp_mode("devin", &modes, "auto", "").unwrap_err();
        assert_eq!(
            err,
            "Provider \"devin\" does not support Auto runtime mode. Auto is available only for Codex and Claude Code."
        );
    }



    #[test]
    fn acp_permission_kinds_map_to_projection_request_kinds() {
        assert_eq!(approval_request_kind("execute"), "command");
        assert_eq!(approval_request_kind("read"), "file-read");
        assert_eq!(approval_request_kind("edit"), "file-change");
        assert_eq!(approval_request_kind("delete"), "file-change");
        assert_eq!(approval_request_kind("other"), "tool");
    }

    #[test]
    fn sparse_permission_request_keeps_tracked_execute_kind() {
        let mut request = json!({"toolCallId": "call-1", "kind": "unknown", "title": "Tool"});
        let tracked = json!({
            "toolCallId": "call-1",
            "kind": "execute",
            "title": "Run tests",
            "rawInput": {"command": "cargo test"},
        });
        merge_tracked_tool_call(&mut request, &tracked);
        assert_eq!(
            request.get("kind").and_then(Value::as_str),
            Some("execute")
        );
        assert_eq!(request.get("title").and_then(Value::as_str), Some("Run tests"));
        assert_eq!(
            request.pointer("/rawInput/command").and_then(Value::as_str),
            Some("cargo test")
        );
    }


    #[test]
    fn acp_provider_profiles_cover_shared_stdio_providers() {
        for provider in ["devin", "omp", "cursor", "grok", "droid", "antigravity"] {
            assert!(supports_provider(provider));
            assert!(acp_launch_profile(provider, "full-access", None).is_some());
        }
        assert!(!supports_provider("opencode"));
        assert!(!supports_provider("codex"));
        assert!(acp_launch_profile("opencode", "full-access", None).is_none());
    }

    #[test]
    fn acp_provider_profiles_use_provider_native_commands() {
        let devin = acp_launch_profile("devin", "full-access", Some("devin-1")).unwrap();
        assert_eq!(devin.args, vec!["acp", "--model", "devin-1"]);

        let omp = acp_launch_profile("omp", "full-access", Some("gpt-5")).unwrap();
        assert_eq!(omp.args, vec!["acp"]);

        let cursor = acp_launch_profile("cursor", "full-access", None).unwrap();
        assert_eq!(cursor.args, vec!["acp"]);
        assert!(cursor.env.contains(&("NO_BROWSER", "true")));

        let grok = acp_launch_profile("grok", "full-access", Some("grok-code-fast-1")).unwrap();
        assert_eq!(
            grok.args,
            vec![
                "--permission-mode",
                "default",
                "agent",
                "--no-leader",
                "--always-approve",
                "-m",
                "grok-code-fast-1",
                "stdio"
            ]
        );

        let droid = acp_launch_profile("droid", "full-access", Some("droid-core")).unwrap();
        assert_eq!(
            droid.args,
            vec!["exec", "--output-format", "acp", "-m", "droid-core"]
        );

        let antigravity = acp_launch_profile("antigravity", "approval-required", None).unwrap();
        assert_eq!(antigravity.args, vec!["acp"]);
    }

    #[test]
    fn auth_method_selection_prefers_provider_cached_methods() {
        let methods = vec![
            json!({"id": "browser-oauth"}),
            json!({"id": "cached_token"}),
        ];
        assert_eq!(
            select_auth_method("devin", &methods),
            Some("cached_token".to_string())
        );
        // Unknown providers fall back to the first advertised method.
        assert_eq!(
            select_auth_method("omp", &methods),
            Some("browser-oauth".to_string())
        );
        assert_eq!(select_auth_method("omp", &[]), None);
    }

    #[test]
    fn auth_required_errors_are_detected() {
        assert!(is_auth_required_error(&err(
            "acp session/new error: authentication required"
        )));
        assert!(is_auth_required_error(&err("unauthorized")));
        assert!(!is_auth_required_error(&err("timeout waiting for response")));
    }

    #[test]
    fn provider_specializations_add_reasoning_and_dirs() {
        let grok = acp_launch_profile_ex("grok", "full-access", None, Some("high"), &[]).unwrap();
        assert!(grok.args.windows(2).any(|w| w == ["--reasoning-effort", "high"]));

        let droid = acp_launch_profile_ex("droid", "full-access", None, Some("medium"), &[]).unwrap();
        assert!(droid.args.windows(2).any(|w| w == ["-r", "medium"]));

        let agy = acp_launch_profile_ex(
            "antigravity",
            "full-access",
            None,
            None,
            &["/extra/dir".to_string()],
        )
        .unwrap();
        assert!(agy.args.windows(2).any(|w| w == ["--add-dir", "/extra/dir"]));
    }

    #[test]
    fn devin_model_selection_resolves_discovered_runtime_variants() {
        let script = std::env::temp_dir().join(format!(
            "synara-devin-models-test-{}",
            std::process::id()
        ));
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf '%s' '{\"models\":[{\"family_uid\":\"swe-2\",\"variants\":[{\"model_uid\":\"swe-2-medium\",\"label\":\"Medium\"},{\"model_uid\":\"swe-2-high\",\"label\":\"High\"},{\"model_uid\":\"swe-2-lightning\",\"label\":\"Lightning\"}]}]}'\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        unsafe {
            std::env::set_var("SYNARA_DEVIN_CLI", &script);
        }
        let selection = json!({"model":"swe-2","options":{"reasoningEffort":"high"}});
        assert_eq!(
            resolve_devin_selected_model(&selection),
            Some("swe-2-high".to_string())
        );
        let selection = json!({"model":"swe-2","options":{"fastMode":true}});
        assert_eq!(
            resolve_devin_selected_model(&selection),
            Some("swe-2-lightning".to_string())
        );
        unsafe {
            std::env::remove_var("SYNARA_DEVIN_CLI");
        }
        let _ = std::fs::remove_file(script);
    }

    #[test]
    fn devin_full_access_still_maps_to_bypass() {
        let modes = json!({
            "availableModes": [{"id": "bypass", "name": "Bypass"}],
            "currentModeId": "code",
        });
        assert_eq!(
            resolve_acp_mode("devin", &modes, "full-access", "").unwrap(),
            Some("bypass".to_string())
        );
    }
    #[test]
    fn executable_file_requires_unix_execute_bit() {
        let path = std::env::temp_dir().join(format!(
            "synara-acp-bin-test-{}",
            std::process::id()
        ));
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(!super::is_executable_file(&path));
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(super::is_executable_file(&path));
        }
        let _ = std::fs::remove_file(path);
    }

}
