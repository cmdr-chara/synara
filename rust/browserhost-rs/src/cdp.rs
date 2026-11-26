//! Chrome DevTools Protocol backend. Synara launches the user's own Chromium browser
//! (Chrome/Edge/Brave/Chromium) with a dedicated profile and drives it over
//! CDP — nothing extra ships in the bundle, and the browser reads its own
//! profile; Synara never touches the user's real browser profile or cookies.

use crate::network::NetworkPolicy;
use crate::vault::BrowserVault;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};




/// Browser operating mode: obscura is the headless background browser,
/// ego lite the visible one. Both are real Chrome/CDP instances with
/// separate profiles so they never fight over one user-data-dir.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum CdpMode {
    Visible,
    Headless,
}

impl CdpMode {
    pub fn backend_name(self) -> &'static str {
        match self {
            Self::Visible => "ego-lite",
            Self::Headless => "obscura",
        }
    }
}

struct CdpProcess {
    child: Child,
    port: u16,
}

struct CdpConnection {
    tx: mpsc::UnboundedSender<String>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    next_id: Arc<AtomicU64>,
}

impl CdpConnection {
    async fn call(&self, method: &str, params: Value, session: Option<&str>) -> Result<Value, String> {
        let id = self.next_id.fetch_add(1, AtomicOrdering::SeqCst);
        let mut msg = Map::new();
        msg.insert("id".into(), json!(id));
        msg.insert("method".into(), json!(method));
        msg.insert("params".into(), params);
        if let Some(s) = session {
            msg.insert("sessionId".into(), json!(s));
        }
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .map_err(|_| "cdp pending lock".to_string())?
            .insert(id, tx);
        let payload = Value::Object(msg).to_string();
        self.tx
            .send(payload)
            .map_err(|_| "cdp websocket closed".to_string())?;
        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| format!("cdp {method} timed out"))?
            .map_err(|_| "cdp connection dropped".to_string())?;
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("cdp error");
            return Err(format!("cdp {method}: {message}"));
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }
}

fn cdp_state(mode: CdpMode) -> &'static Mutex<(Option<CdpProcess>, Option<Arc<CdpConnection>>)> {
    static VISIBLE: OnceLock<Mutex<(Option<CdpProcess>, Option<Arc<CdpConnection>>)>> = OnceLock::new();
    static HEADLESS: OnceLock<Mutex<(Option<CdpProcess>, Option<Arc<CdpConnection>>)>> = OnceLock::new();
    let slot = match mode {
        CdpMode::Visible => &VISIBLE,
        CdpMode::Headless => &HEADLESS,
    };
    slot.get_or_init(|| Mutex::new((None, None)))
}

/// Remembered per-thread mode: browser_open pins the thread to the visible
/// or headless browser so follow-up tools land in the same instance.
fn thread_modes() -> &'static Mutex<HashMap<String, CdpMode>> {
    static MODES: OnceLock<Mutex<HashMap<String, CdpMode>>> = OnceLock::new();
    MODES.get_or_init(|| Mutex::new(HashMap::new()))
}

fn resolve_mode(args: &Value, thread: Option<&str>) -> CdpMode {
    match args.get("backend").and_then(Value::as_str).unwrap_or("auto") {
        "obscura" | "headless" => return CdpMode::Headless,
        "ego-lite" | "ego" | "ego_lite" | "visible" => return CdpMode::Visible,
        _ => {}
    }
    if let Some(visible) = args
        .get("visible")
        .and_then(Value::as_bool)
        .or_else(|| args.get("show").and_then(Value::as_bool))
    {
        return if visible { CdpMode::Visible } else { CdpMode::Headless };
    }
    let key = thread.unwrap_or("").to_string();
    if let Some(mode) = thread_modes().lock().ok().and_then(|m| m.get(&key).copied()) {
        return mode;
    }
    // Background work defaults to the headless browser; only explicitly
    // visible work opens a window.
    CdpMode::Headless
}

fn chrome_path() -> Option<String> {
    if let Ok(p) = std::env::var("SYNARA_CDP_BROWSER") {
        if !p.trim().is_empty() && std::path::Path::new(&p).exists() {
            return Some(p);
        }
    }
    let candidates = [
        "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        "/Applications/Chromium.app/Contents/MacOS/Chromium",
        "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
        "/usr/bin/google-chrome",
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
    ];
    candidates
        .iter()
        .find(|p| std::path::Path::new(p).exists())
        .map(|p| p.to_string())
}

fn cdp_profile_dir(mode: CdpMode) -> std::path::PathBuf {
    let base = std::env::var("SYNARA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
                .join(".synara")
        });
    match mode {
        CdpMode::Visible => base.join("chrome-cdp-profile"),
        CdpMode::Headless => base.join("chrome-obscura-profile"),
    }
}

fn vault_capture_configured() -> bool {
    let Some(home) = std::env::var_os("SYNARA_HOME") else { return false };
    let text = std::fs::read_to_string(
        std::path::Path::new(&home)
            .join("browser-vault")
            .join("preferences.json"),
    );
    let Ok(text) = text else { return false };
    serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|preferences| {
            preferences
                .pointer("/settings/offerSave")
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

/// Shared owner-side vault instance for the desktop shell. This is not part
/// of the agent-facing browser tool surface.
pub fn shared_browser_vault() -> Result<&'static BrowserVault, String> {
    browser_vault()
}

/// Browser automation destinations already opened for a thread, exposed only
/// for the owner-side cookie import surface. Values are CDP backend names.
pub fn cookie_import_destinations(thread: &str) -> Vec<&'static str> {
    let suffix = format!(":{thread}");
    let mut backends: Vec<&'static str> = cdp_tabs()
        .lock()
        .map(|tabs| {
            tabs.keys()
                .filter_map(|key| {
                    key.strip_suffix(&suffix).and_then(|backend| match backend {
                        "ego-lite" => Some("ego-lite"),
                        "obscura" => Some("obscura"),
                        _ => None,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    backends.sort_unstable();
    backends.dedup();
    backends
}

fn browser_vault() -> Result<&'static BrowserVault, String> {
    static VAULT: OnceLock<Result<BrowserVault, String>> = OnceLock::new();
    VAULT
        .get_or_init(|| {
            let base = std::env::var("SYNARA_HOME")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| {
                    std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
                        .join(".synara")
                });
            BrowserVault::open(base.join("browser-vault"))
        })
        .as_ref()
        .map_err(Clone::clone)
}

const CREDENTIALS_BRIDGE_JS: &str = r#"(() => {
  if (window.__synaraCredentialBridgeInstalled) return true;
  window.__synaraCredentialBridgeInstalled = true;
  window.__synaraCredentialPending = {};
  window.__synaraCredentialResolve = (id, result, error) => {
    const pending = window.__synaraCredentialPending[id];
    if (!pending) return false;
    delete window.__synaraCredentialPending[id];
    if (error) pending.reject(new Error(error));
    else pending.resolve(result);
    return true;
  };
  const invoke = (action, payload) => new Promise((resolve, reject) => {
    const id = globalThis.crypto?.randomUUID?.() || `${Date.now()}-${Math.random()}`;
    window.__synaraCredentialPending[id] = { resolve, reject };
    window.__synaraCredentialBinding(JSON.stringify({
      id,
      action,
      payload: payload || {},
      origin: location.origin,
    }));
  });
  window.credentials = {
    list: (payload) => invoke('list', payload),
    listPending: (payload) => invoke('list-pending', payload),
  };
  return true;
})()"#;

fn credential_list_result(action: &str, origin: &str) -> Result<Value, String> {
    let vault = browser_vault()?;
    match action {
        "list" => vault.agent_list_metadata(origin).map(Value::Array),
        "list-pending" => vault.agent_list_pending_metadata(origin).map(Value::Array),
        _ => Err("credential action unavailable to browser scripts".into()),
    }
}


const VAULT_SENSOR_JS: &str = include_str!("../assets/vault-sensor.js");
const VAULT_WORLD: &str = "betterwright-vault";
const VAULT_BINDING: &str = "__bwVaultEvent";

fn agent_activity() -> &'static Mutex<HashMap<String, i64>> {
    static ACTIVITY: OnceLock<Mutex<HashMap<String, i64>>> = OnceLock::new();
    ACTIVITY.get_or_init(|| Mutex::new(HashMap::new()))
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn note_agent_activity(session: &str) {
    if let Ok(mut activity) = agent_activity().lock() {
        activity.insert(session.to_string(), now_ms());
    }
}

fn recent_agent_activity(session: &str) -> bool {
    agent_activity()
        .lock()
        .ok()
        .and_then(|activity| activity.get(session).copied())
        .is_some_and(|at| now_ms() - at <= 5_000)
}

fn strip_hash(url: &str) -> &str {
    url.split('#').next().unwrap_or(url)
}

fn host_label(origin: &str) -> Option<String> {
    url::Url::parse(origin)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
}

#[derive(Clone)]
struct PendingCapture {
    origin: String,
    href: String,
    username: String,
    password: String,
    context_id: u64,
    confirming: bool,
}

fn resolve_capture(
    capture: PendingCapture,
    session: &str,
    prompts: &mut HashMap<String, (String, PendingCapture)>,
    tx: &mpsc::UnboundedSender<String>,
    next_id: &AtomicU64,
) {
    let Ok(vault) = browser_vault() else { return };
    let (enabled, autosave) = vault.capture_policy();
    if !enabled
        || vault.is_never_origin(&capture.origin)
        || !vault
            .should_offer_save(&capture.origin, &capture.username, &capture.password)
            .unwrap_or(false)
    {
        return;
    }
    let agent_driven = recent_agent_activity(session);
    let source = if agent_driven {
        crate::vault::CredentialSource::Agent
    } else {
        crate::vault::CredentialSource::User
    };
    if autosave || agent_driven {
        let _ = vault.save_captured(
            &capture.origin,
            &capture.username,
            &capture.password,
            host_label(&capture.origin).as_deref(),
            source,
            true,
        );
        return;
    }
    let Ok(prompt) = vault.offer_save(&capture.origin, &capture.username, &capture.password) else {
        return;
    };
    let prompt_id = prompt
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if prompt_id.is_empty() {
        return;
    }
    prompts.insert(prompt_id.clone(), (session.to_string(), capture.clone()));
    let expression = format!(
        "globalThis.__bwVaultShowPrompt && globalThis.__bwVaultShowPrompt({})",
        json!({
            "promptId": prompt_id,
            "origin": capture.origin,
            "username": capture.username,
            "mode": prompt.get("mode").and_then(Value::as_str).unwrap_or("save"),
        })
    );
    fire_and_forget_command(
        tx,
        next_id,
        "Runtime.evaluate",
        json!({
            "expression": expression,
            "contextId": capture.context_id,
        }),
        Some(session),
    );
}

fn fire_and_forget_command(
    tx: &mpsc::UnboundedSender<String>,
    next_id: &AtomicU64,
    method: &str,
    params: Value,
    session: Option<&str>,
) {
    let id = next_id.fetch_add(1, AtomicOrdering::SeqCst);
    let mut msg = Map::new();
    msg.insert("id".into(), json!(id));
    msg.insert("method".into(), json!(method));
    msg.insert("params".into(), params);
    if let Some(session) = session {
        msg.insert("sessionId".into(), json!(session));
    }
    let _ = tx.send(Value::Object(msg).to_string());
}

async fn connect_ws(url: &str) -> Result<Arc<CdpConnection>, String> {
    use futures_util::{SinkExt, StreamExt};
    let (ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|e| format!("cdp websocket connect: {e}"))?;
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        while let Some(text) = rx.recv().await {
            if sink
                .send(tokio_tungstenite::tungstenite::Message::Text(text.clone().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });
    let pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(AtomicU64::new(1));
    let pending_reader = pending.clone();
    let event_tx = tx.clone();
    let event_next_id = next_id.clone();
    tokio::spawn(async move {
        // context key -> (origin, world name, frame id)
        let mut execution_contexts: HashMap<(String, u64), (String, String, String)> =
            HashMap::new();
        // (session, frame) -> (isolated-world context id, last committed URL)
        let mut vault_frames: HashMap<(String, String), (u64, String)> = HashMap::new();
        let mut pending_captures: HashMap<(String, String), PendingCapture> = HashMap::new();
        let mut vault_prompts: HashMap<String, (String, PendingCapture)> = HashMap::new();
        while let Some(msg) = stream.next().await {
            let Ok(msg) = msg else { break };
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                if let Ok(v) = serde_json::from_str::<Value>(&text) {
                    if let Some(id) = v.get("id").and_then(Value::as_u64) {
                        if let Ok(mut map) = pending_reader.lock() {
                            if let Some(sender) = map.remove(&id) {
                                let _ = sender.send(v);
                            }
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Runtime.executionContextCreated")
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let id = v
                            .pointer("/params/context/id")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let origin = v
                            .pointer("/params/context/origin")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let name = v
                            .pointer("/params/context/name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let frame_id = v
                            .pointer("/params/context/auxData/frameId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if id != 0 && !origin.is_empty() {
                            if name == VAULT_WORLD {
                                fire_and_forget_command(
                                    &event_tx,
                                    &event_next_id,
                                    "Runtime.addBinding",
                                    json!({"name": VAULT_BINDING, "executionContextName": VAULT_WORLD}),
                                    Some(&session),
                                );
                                vault_frames
                                    .entry((session.clone(), frame_id.clone()))
                                    .or_insert((id, String::new()));
                            }
                            execution_contexts.insert((session, id), (origin, name, frame_id));
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Runtime.executionContextDestroyed")
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let id = v
                            .pointer("/params/executionContextId")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        execution_contexts.remove(&(session, id));
                        vault_frames.retain(|_, (context_id, _)| *context_id != id);
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Runtime.bindingCalled")
                        && v.pointer("/params/name").and_then(Value::as_str) == Some(VAULT_BINDING)
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let context_id = v
                            .pointer("/params/executionContextId")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let payload_text = v
                            .pointer("/params/payload")
                            .and_then(Value::as_str)
                            .unwrap_or("{}");
                        let Ok(message) = serde_json::from_str::<Value>(payload_text) else {
                            continue;
                        };
                        let message_type = message.get("type").and_then(Value::as_str).unwrap_or("");
                        if message_type == "promptResponse" {
                            let prompt_id = message
                                .get("promptId")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            let choice = message.get("choice").and_then(Value::as_str).unwrap_or("");
                            if let Some((prompt_session, capture)) = vault_prompts.remove(prompt_id) {
                                if let Ok(vault) = browser_vault() {
                                    if choice == "never" {
                                        let _ = vault.never_origin(&capture.origin);
                                    }
                                    let _ = vault.respond(prompt_id, choice == "save");
                                }
                                let _ = prompt_session;
                            }
                            continue;
                        }
                        let frame_id = execution_contexts
                            .get(&(session.clone(), context_id))
                            .map(|(_, _, frame)| frame.clone())
                            .unwrap_or_default();
                        if session.is_empty() || frame_id.is_empty() {
                            continue;
                        }
                        let key = (session.clone(), frame_id.clone());
                        if message_type == "submitCapture" {
                            let origin = message
                                .get("origin")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                            let password = message
                                .get("password")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                            if !origin.is_empty() && !password.is_empty() {
                                if let Ok(vault) = browser_vault() {
                                    if vault.capture_enabled() {
                                        vault.track_secret(&password);
                                        pending_captures.insert(
                                            key,
                                            PendingCapture {
                                                origin,
                                                href: message
                                                    .get("href")
                                                    .and_then(Value::as_str)
                                                    .unwrap_or("")
                                                    .to_string(),
                                                username: message
                                                    .get("username")
                                                    .and_then(Value::as_str)
                                                    .unwrap_or("")
                                                    .to_string(),
                                                password,
                                                context_id,
                                                confirming: false,
                                            },
                                        );
                                    }
                                }
                            }
                        } else if message_type == "fieldsCleared" {
                            if let Some(capture) = pending_captures.remove(&key) {
                                resolve_capture(
                                    capture,
                                    &session,
                                    &mut vault_prompts,
                                    &event_tx,
                                    &event_next_id,
                                );
                            }
                        } else if message_type == "passwordFormScan" {
                            let present = message
                                .get("present")
                                .and_then(Value::as_bool)
                                .unwrap_or(false);
                            let resolving = pending_captures
                                .get(&key)
                                .is_some_and(|capture| capture.confirming);
                            if resolving {
                                if present {
                                    pending_captures.remove(&key);
                                } else if let Some(capture) = pending_captures.remove(&key) {
                                    resolve_capture(
                                        capture,
                                        &session,
                                        &mut vault_prompts,
                                        &event_tx,
                                        &event_next_id,
                                    );
                                }
                            }
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Runtime.bindingCalled")
                        && v.pointer("/params/name").and_then(Value::as_str)
                            == Some("__synaraCredentialBinding")
                    {
                        let session = v.get("sessionId").and_then(Value::as_str);
                        let context_id = v
                            .pointer("/params/executionContextId")
                            .and_then(Value::as_u64)
                            .unwrap_or(0);
                        let context_origin = session
                            .and_then(|session| {
                                execution_contexts
                                    .get(&(session.to_string(), context_id))
                                    .map(|(origin, _, _)| origin.clone())
                            })
                            .unwrap_or_default();
                        let payload_text = v
                            .pointer("/params/payload")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        let mut request_id = String::new();
                        let mut result = Value::Null;
                        let mut error: Option<String> = None;
                        if context_origin.is_empty() {
                            error = Some("credential origin is unavailable".into());
                        } else if let Ok(request) = serde_json::from_str::<Value>(payload_text) {
                            request_id = request
                                .get("id")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string();
                            let action = request
                                .get("action")
                                .and_then(Value::as_str)
                                .unwrap_or("");
                            let payload_origin = request
                                .get("origin")
                                .and_then(Value::as_str)
                                .unwrap_or(&context_origin);
                            if payload_origin != context_origin {
                                error = Some("credential request does not match this page origin".into());
                            } else {
                                match credential_list_result(action, &context_origin) {
                                    Ok(value) => result = value,
                                    Err(message) => error = Some(message),
                                }
                            }
                        } else {
                            error = Some("invalid credential request".into());
                        }
                        if !request_id.is_empty() {
                            let error_json = error
                                .as_deref()
                                .map(js_string)
                                .unwrap_or_else(|| "null".into());
                            let expression = format!(
                                "window.__synaraCredentialResolve && window.__synaraCredentialResolve({}, {}, {})",
                                js_string(&request_id),
                                if error.is_some() { "null".into() } else { result.to_string() },
                                error_json,
                            );
                            fire_and_forget_command(
                                &event_tx,
                                &event_next_id,
                                "Runtime.evaluate",
                                json!({
                                    "expression": expression,
                                    "awaitPromise": true,
                                    "returnByValue": true,
                                    "contextId": context_id,
                                }),
                                session,
                            );
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Page.frameNavigated")
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let frame_id = v
                            .pointer("/params/frame/id")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let url = v
                            .pointer("/params/frame/url")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if !session.is_empty() && !frame_id.is_empty() {
                            let key = (session.clone(), frame_id.clone());
                            vault_frames
                                .entry(key.clone())
                                .and_modify(|(_, old_url)| *old_url = url.clone())
                                .or_insert((0, url.clone()));
                            if let Some(capture) = pending_captures.get_mut(&key) {
                                if !url.is_empty() && strip_hash(&url) != strip_hash(&capture.href) {
                                    capture.confirming = true;
                                } else {
                                    pending_captures.remove(&key);
                                }
                            }
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Page.navigatedWithinDocument")
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let frame_id = v
                            .pointer("/params/frameId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let url = v
                            .pointer("/params/url")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let key = (session.clone(), frame_id.clone());
                        if let Some(capture) = pending_captures.get_mut(&key) {
                            if !url.is_empty() && strip_hash(&url) != strip_hash(&capture.href) {
                                capture.confirming = true;
                            }
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Page.frameDetached")
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let frame_id = v
                            .pointer("/params/frameId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let key = (session.clone(), frame_id);
                        if let Some(capture) = pending_captures.remove(&key) {
                            resolve_capture(
                                capture,
                                &session,
                                &mut vault_prompts,
                                &event_tx,
                                &event_next_id,
                            );
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Fetch.requestPaused")
                    {
                        let interception_id = v
                            .pointer("/params/requestId")
                            .or(v.pointer("/params/interceptionId"))
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        let request_url = v
                            .pointer("/params/request/url")
                            .and_then(Value::as_str)
                            .unwrap_or("");
                        if !interception_id.is_empty() {
                            let session = v.get("sessionId").and_then(Value::as_str);
                            match NetworkPolicy::default().check(request_url) {
                                Ok(()) => fire_and_forget_command(
                                    &event_tx,
                                    &event_next_id,
                                    "Fetch.continueRequest",
                                    json!({"requestId": interception_id}),
                                    session,
                                ),
                                Err(reason) => {
                                    eprintln!(
                                        "browser network guard: blocked {request_url}: {reason}"
                                    );
                                    fire_and_forget_command(
                                        &event_tx,
                                        &event_next_id,
                                        "Fetch.failRequest",
                                        json!({
                                            "requestId": interception_id,
                                            "errorReason": "AccessDenied",
                                        }),
                                        session,
                                    );
                                }
                            }
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Browser.downloadProgress")
                    {
                        let state = v
                            .pointer("/params/state")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let guid = v
                            .pointer("/params/guid")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        if state == "completed" {
                            if let Ok(mut dl) = downloads().lock() {
                                dl.insert(guid.clone(), json!({"guid": guid, "state": state}));
                            }
                        }
                    } else if v.get("method").and_then(Value::as_str)
                        == Some("Runtime.consoleAPICalled")
                    {
                        let session = v
                            .get("sessionId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let level = v
                            .pointer("/params/type")
                            .and_then(Value::as_str)
                            .unwrap_or("log")
                            .to_string();
                        let text_out: String = v
                            .pointer("/params/args")
                            .and_then(Value::as_array)
                            .map(|args| {
                                args.iter()
                                    .map(|a| {
                                        a.get("value")
                                            .and_then(Value::as_str)
                                            .map(str::to_string)
                                            .unwrap_or_else(|| {
                                                a.get("description")
                                                    .and_then(Value::as_str)
                                                    .unwrap_or("")
                                                    .to_string()
                                            })
                                    })
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        if let Ok(mut logs) = console_logs().lock() {
                            let entries = logs.entry(session).or_default();
                            entries.push(json!({"level": level, "text": text_out}));
                            if entries.len() > 500 {
                                entries.remove(0);
                            }
                        }
                    }
                }
            }
        }
    });
    Ok(Arc::new(CdpConnection {
        tx,
        pending,
        next_id,
    }))
}

/// Launch (or reuse) the CDP browser and return its connection.
async fn ensure_cdp(mode: CdpMode) -> Result<Arc<CdpConnection>, String> {
    if let Ok(state) = cdp_state(mode).lock() {
        if let Some(conn) = state.1.clone() {
            return Ok(conn);
        }
    }
    let binary = chrome_path().ok_or_else(|| {
        "no Chromium browser found for the CDP fallback (install Chrome, Edge, Brave or Chromium)".to_string()
    })?;
    let profile = cdp_profile_dir(mode);
    // Chrome refuses a second instance on the same user-data-dir. When a
    // browser is already running on this profile (whichever process launched
    // it), reuse it. Chrome only writes DevToolsActivePort for auto-assigned
    // ports, so our own launcher records the port in a sidecar file.
    for candidate in [
        profile.join("synara-cdp-port"),
        profile.join("DevToolsActivePort"),
    ] {
        if let Ok(contents) = std::fs::read_to_string(&candidate) {
            if let Some(port_str) = contents.lines().next() {
                if let Ok(existing_port) = port_str.trim().parse::<u16>() {
                let endpoint = format!("http://127.0.0.1:{existing_port}/json/version");
                if let Ok(resp) = reqwest_get(&endpoint).await {
                    if let Ok(version) = serde_json::from_str::<Value>(&resp) {
                        if let Some(ws_url) = version
                            .get("webSocketDebuggerUrl")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                        {
                            if let Ok(conn) = connect_ws(&ws_url).await {
                                if let Ok(mut state) = cdp_state(mode).lock() {
                                    state.1 = Some(conn.clone());
                                }
                                return Ok(conn);
                            }
                        }
                    }
                }
            }
        }
    }
    }
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .map_err(|e| format!("no free port for cdp: {e}"))?;
    let _ = std::fs::create_dir_all(&profile);
    let headless_args: Vec<&str> = match mode {
        CdpMode::Headless => vec!["--headless=new", "--disable-gpu"],
        CdpMode::Visible => vec![],
    };
    let mut child_command = Command::new(&binary);
    child_command.args([
            &format!("--remote-debugging-port={port}"),
            &format!("--user-data-dir={}", profile.display()),
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-session-crashed-bubble",
            "--hide-crash-restore-bubble",
        ])
        .args(&headless_args)
        .arg("about:blank")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let child = child_command
        .spawn()
        .map_err(|e| format!("could not launch chromium for cdp: {e}"))?;
    // Wait for the devtools endpoint.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    let mut version: Option<Value> = None;
    while std::time::Instant::now() < deadline {
        if let Ok(resp) = reqwest_get(&format!("http://127.0.0.1:{port}/json/version")).await {
            if let Ok(v) = serde_json::from_str::<Value>(&resp) {
                version = Some(v);
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    let version = version.ok_or_else(|| "chromium devtools endpoint did not come up".to_string())?;
    let _ = std::fs::write(profile.join("synara-cdp-port"), format!("{port}\n"));
    let ws_url = version
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| "chromium devtools endpoint missing websocket url".to_string())?
        .to_string();
    let conn = connect_ws(&ws_url).await?;
    if let Ok(mut state) = cdp_state(mode).lock() {
        state.0 = Some(CdpProcess { child, port });
        state.1 = Some(conn.clone());
    }
    Ok(conn)
}

/// Minimal HTTP GET through a raw TCP stream — enough for the localhost
/// devtools endpoints, no extra HTTP client dependency.
async fn reqwest_get(url: &str) -> Result<String, String> {
    let stripped = url.strip_prefix("http://").unwrap_or(url);
    let mut parts = stripped.splitn(2, '/');
    let host = parts.next().unwrap_or("");
    let path = format!("/{}", parts.next().unwrap_or(""));
    let mut stream = tokio::net::TcpStream::connect(host)
        .await
        .map_err(|e| format!("connect {host}: {e}"))?;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    // Chromium's devtools HTTP server ignores `Connection: close`, so read
    // the headers first and then exactly Content-Length body bytes.
    let mut buf = Vec::new();
    let mut header_end = None;
    let mut content_length = 0usize;
    while header_end.is_none() {
        let mut chunk = [0u8; 1024];
        let n = tokio::time::timeout(std::time::Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .map_err(|_| "devtools http read timed out".to_string())?
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("devtools http closed before headers".to_string());
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&buf[..pos]).to_lowercase();
            for line in headers.split("\r\n") {
                if let Some(value) = line.strip_prefix("content-length:") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
            header_end = Some(pos + 4);
        }
    }
    let start = header_end.unwrap_or(buf.len());
    while buf.len() - start < content_length {
        let mut chunk = [0u8; 4096];
        let n = tokio::time::timeout(std::time::Duration::from_secs(5), stream.read(&mut chunk))
            .await
            .map_err(|_| "devtools http body read timed out".to_string())?
            .map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(String::from_utf8_lossy(&buf[start..(start + content_length).min(buf.len())]).to_string())
}

pub async fn shutdown_cdp() {
    for mode in [CdpMode::Visible, CdpMode::Headless] {
        let child = cdp_state(mode)
            .lock()
            .ok()
            .and_then(|mut state| {
                state.1 = None;
                state.0.take().map(|mut p| {
                    let _ = p.child.start_kill();
                    p.child
                })
            });
        if let Some(mut child) = child {
            let _ = child.wait().await;
        }
    }
}

/// Per-thread tab: each calling thread gets its own target in the shared CDP
/// browser, mirroring upstream's per-thread browser target registry.
fn downloads() -> &'static Mutex<HashMap<String, Value>> {
    static DL: OnceLock<Mutex<HashMap<String, Value>>> = OnceLock::new();
    DL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn console_logs() -> &'static Mutex<HashMap<String, Vec<Value>>> {
    static LOGS: OnceLock<Mutex<HashMap<String, Vec<Value>>>> = OnceLock::new();
    LOGS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn cdp_tabs() -> &'static Mutex<HashMap<String, (String, String)>> {
    static TABS: OnceLock<Mutex<HashMap<String, (String, String)>>> = OnceLock::new();
    TABS.get_or_init(|| Mutex::new(HashMap::new()))
}

async fn thread_session(
    conn: &CdpConnection,
    thread: Option<&str>,
    mode: CdpMode,
) -> Result<(String, String), String> {
    let key = format!("{}:{}", mode.backend_name(), thread.unwrap_or(""));
    if let Some(pair) = cdp_tabs().lock().ok().and_then(|t| t.get(&key).cloned()) {
        return Ok(pair);
    }
    let target = conn
        .call("Target.createTarget", json!({"url": "about:blank"}), None)
        .await?;
    let target_id = target
        .get("targetId")
        .and_then(Value::as_str)
        .ok_or_else(|| "Target.createTarget missing targetId".to_string())?
        .to_string();
    let attached = conn
        .call(
            "Target.attachToTarget",
            json!({"targetId": target_id, "flatten": true}),
            None,
        )
        .await?;
    let session_id = attached
        .get("sessionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "Target.attachToTarget missing sessionId".to_string())?
        .to_string();
    let _ = conn
        .call("Page.enable", json!({}), Some(&session_id))
        .await;
    let _ = conn
        .call("Runtime.enable", json!({}), Some(&session_id))
        .await;
    let _ = conn
        .call(
            "Runtime.addBinding",
            json!({"name": "__synaraCredentialBinding", "executionContextName": "synara-agent"}),
            Some(&session_id),
        )
        .await;
    let _ = conn
        .call(
            "Fetch.enable",
            json!({"patterns": [{"urlPattern": "*", "requestStage": "Request"}]}),
            Some(&session_id),
        )
        .await;
    if vault_capture_configured() {
        if let Ok(vault) = browser_vault() {
            if vault.capture_enabled() {
                match conn
                    .call(
                        "Page.addScriptToEvaluateOnNewDocument",
                        json!({
                            "source": VAULT_SENSOR_JS,
                            "worldName": VAULT_WORLD,
                            "runImmediately": true,
                        }),
                        Some(&session_id),
                    )
                    .await
                {
                    Ok(_) => vault.report_capture_ready(),
                    Err(_) => vault.report_capture_failure(),
                }
            }
        }
    }
    if let Ok(mut tabs) = cdp_tabs().lock() {
        tabs.insert(key, (target_id.clone(), session_id.clone()));
    }
    Ok((target_id, session_id))
}

/// Read cookies from a local browser profile with rookie-cookies and install
/// them into an existing browser tab's cookie jar. This owner-only operation is
/// deliberately not exposed as an agent browser tool: importing cookies grants
/// session access and must be initiated by the human-facing vault UI.
pub async fn sync_cookie_import(
    thread: &str,
    browser: &str,
    profile: Option<&str>,
    domains: &[String],
    mode: CdpMode,
) -> Result<Value, String> {
    static IMPORT_BUSY: AtomicU64 = AtomicU64::new(0);
    if IMPORT_BUSY
        .compare_exchange(0, 1, AtomicOrdering::SeqCst, AtomicOrdering::SeqCst)
        .is_err()
    {
        return Err("another cookie import is running".into());
    }
    struct ImportGuard;
    impl Drop for ImportGuard {
        fn drop(&mut self) {
            IMPORT_BUSY.store(0, AtomicOrdering::SeqCst);
        }
    }
    let _import_guard = ImportGuard;
    let key = format!("{}:{}", mode.backend_name(), thread);
    let Some((_, session)) = cdp_tabs().lock().ok().and_then(|tabs| tabs.get(&key).cloned()) else {
        return Err("cookie import requires an existing destination tab".into());
    };
    let conn = ensure_cdp(mode).await?;
    let before_url = cdp_eval(&conn, &session, "location.href")
        .await
        .and_then(|value| value.as_str().map(str::to_string).ok_or_else(|| "destination URL unavailable".to_string()))?;
    let wait_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while recent_agent_activity(&session) && std::time::Instant::now() < wait_deadline {
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
    }
    if recent_agent_activity(&session) {
        return Err("wait for the agent to finish using the browser before importing cookies".into());
    }
    let Some(profile) = profile else {
        return Err("choose an available browser profile".into());
    };
    let profiles = crate::cookies::profiles(browser)?;
    if !profiles.iter().any(|candidate| candidate.get("id").and_then(Value::as_str) == Some(profile)) {
        return Err("choose an available browser profile".into());
    }
    let browser = browser.to_string();
    let profile = Some(profile.to_string());
    if domains.len() > 1 {
        return Err("cookie import supports one visible site at a time".into());
    }
    if let Some(domain) = domains.first() {
        let visible_host = url::Url::parse(&before_url)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .unwrap_or_default();
        let normalized = domain.trim_start_matches('.').to_ascii_lowercase();
        if visible_host != normalized && !visible_host.ends_with(&format!(".{normalized}")) {
            return Err("cookie import must match the visible site".into());
        }
    }
    let domains = domains.to_vec();
    let import = tokio::task::spawn_blocking(move || {
        crate::cookies::read_import(&browser, profile.as_deref(), &domains)
    })
    .await
    .map_err(|_| "cookie import worker failed".to_string())??;
    let mut imported = 0usize;
    let mut skipped = import.skipped;
    let _ = conn.call("Network.enable", json!({}), Some(&session)).await;
    for chunk in import.cookies.chunks(500) {
        let batch = conn
            .call(
                "Network.setCookies",
                json!({"cookies": chunk}),
                Some(&session),
            )
            .await;
        if batch.is_ok() {
            imported += chunk.len();
            continue;
        }
        // Older CDP implementations reject a batch when a row uses a newer
        // optional field (notably CHIPS partition keys). Retry row-by-row and
        // count only cookies the browser accepted. Values never enter errors,
        // logs, or returned metadata.
        for cookie in chunk {
            match conn
                .call("Network.setCookie", cookie.clone(), Some(&session))
                .await
            {
                Ok(result) if result.get("success").and_then(Value::as_bool) != Some(false) => {
                    imported += 1;
                }
                _ => skipped += 1,
            }
        }
    }
    let after_url = cdp_eval(&conn, &session, "location.href")
        .await
        .and_then(|value| value.as_str().map(str::to_string).ok_or_else(|| "destination URL unavailable".to_string()))?;
    if after_url != before_url {
        return Err("the cookie import destination changed during import".into());
    }
    Ok(json!({
        "ok": true,
        "imported": imported,
        "skipped": skipped,
        "domains": import.domains,
        "warnings": import.warnings,
    }))
}

/// Wait for the document to reach a usable ready state after navigation.
/// `Page.navigate` returns as soon as the navigation commits; interacting
/// with the page before the DOM exists races every follow-up tool.
async fn wait_for_load(conn: &CdpConnection, session: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if let Ok(Value::String(state)) = cdp_eval(conn, session, "document.readyState").await {
            if state == "interactive" || state == "complete" {
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    }
}

async fn cdp_eval_in_context(
    conn: &CdpConnection,
    session: &str,
    expression: &str,
    context_id: Option<u64>,
) -> Result<Value, String> {
    let mut params = json!({
        "expression": expression,
        "awaitPromise": true,
        "returnByValue": true,
    });
    if let Some(context_id) = context_id {
        params["contextId"] = json!(context_id);
    }
    let result = conn
        .call("Runtime.evaluate", params, Some(session))
        .await?;
    if let Some(exception) = result.pointer("/exceptionDetails/exception/description").and_then(Value::as_str) {
        return Err(exception.to_string());
    }
    if let Some(exception) = result.pointer("/exceptionDetails/text").and_then(Value::as_str) {
        if result.pointer("/result/value").is_none() {
            return Err(format!("evaluation failed: {exception}"));
        }
    }
    Ok(result
        .pointer("/result/value")
        .cloned()
        .unwrap_or(Value::Null))
}

async fn cdp_eval(
    conn: &CdpConnection,
    session: &str,
    expression: &str,
) -> Result<Value, String> {
    cdp_eval_in_context(conn, session, expression, None).await
}

async fn trusted_click(
    conn: &CdpConnection,
    session: &str,
    selector: &str,
) -> Result<(), String> {
    let document = conn
        .call("DOM.getDocument", json!({"depth": 1}), Some(session))
        .await?;
    let root = document
        .pointer("/root/nodeId")
        .and_then(Value::as_u64)
        .ok_or("DOM root unavailable")?;
    let node = conn
        .call(
            "DOM.querySelector",
            json!({"nodeId": root, "selector": selector}),
            Some(session),
        )
        .await?;
    let node_id = node
        .get("nodeId")
        .and_then(Value::as_u64)
        .filter(|id| *id != 0)
        .ok_or("click target not found")?;
    let quads = conn
        .call("DOM.getContentQuads", json!({"nodeId": node_id}), Some(session))
        .await?;
    let quad = quads
        .get("quads")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(Value::as_array)
        .ok_or("click target is not visible")?;
    let xs: Vec<f64> = [0, 2, 4, 6]
        .iter()
        .filter_map(|index| quad.get(*index).and_then(Value::as_f64))
        .collect();
    let ys: Vec<f64> = [1, 3, 5, 7]
        .iter()
        .filter_map(|index| quad.get(*index).and_then(Value::as_f64))
        .collect();
    if xs.len() != 4 || ys.len() != 4 {
        return Err("click target geometry is unavailable".into());
    }
    let x = xs.iter().sum::<f64>() / 4.0;
    let y = ys.iter().sum::<f64>() / 4.0;
    for event_type in ["mouseMoved", "mousePressed", "mouseReleased"] {
        let mut event = json!({
            "type": event_type,
            "x": x,
            "y": y,
            "button": "left",
            "clickCount": 1,
        });
        if event_type == "mouseMoved" {
            event.as_object_mut().unwrap().remove("clickCount");
        }
        conn.call("Input.dispatchMouseEvent", event, Some(session))
            .await?;
    }
    Ok(())
}

async fn create_agent_context(conn: &CdpConnection, session: &str) -> Result<u64, String> {
    let tree = conn.call("Page.getFrameTree", json!({}), Some(session)).await?;
    let frame_id = tree
        .pointer("/frameTree/frame/id")
        .and_then(Value::as_str)
        .ok_or_else(|| "Page.getFrameTree missing main frame".to_string())?;
    let created = conn
        .call(
            "Page.createIsolatedWorld",
            json!({"frameId": frame_id, "worldName": "synara-agent"}),
            Some(session),
        )
        .await?;
    created
        .get("executionContextId")
        .and_then(Value::as_u64)
        .ok_or_else(|| "Page.createIsolatedWorld missing executionContextId".to_string())
}

fn js_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "null".into())
}

fn selector_js(args: &Value) -> Result<String, String> {
    for key in ["ref", "selector", "locator"] {
        if let Some(sel) = args.get(key) {
            let sel = if sel.is_string() {
                sel.as_str().unwrap_or("").to_string()
            } else {
                sel.to_string()
            };
            if !sel.is_empty() {
                return Ok(sel);
            }
        }
    }
    Err("browser target requires ref, selector or locator".to_string())
}

pub fn cdp_available() -> bool {
    chrome_path().is_some()
}

pub async fn call_cdp_tool(
    name: &str,
    args: &Value,
    thread: Option<&str>,
) -> Result<Value, String> {
    let mode = resolve_mode(args, thread);
    if name == "browser_open" {
        if let Ok(mut modes) = thread_modes().lock() {
            modes.insert(thread.unwrap_or("").to_string(), mode);
        }
    }
    let conn = ensure_cdp(mode).await?;
    let backend = mode.backend_name();
    if name == "browser_status" {
        return Ok(json!({
            "available": true,
            "backend": backend,
        }));
    }
    if name == "browser_tabs" {
        let targets = conn.call("Target.getTargets", json!({}), None).await?;
        let list: Vec<Value> = targets
            .get("targetInfos")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter(|t| t.get("type").and_then(Value::as_str) == Some("page"))
                    .map(|t| {
                        json!({
                            "tabId": t.get("targetId").cloned().unwrap_or(Value::Null),
                            "url": t.get("url").cloned().unwrap_or(Value::Null),
                            "title": t.get("title").cloned().unwrap_or(Value::Null),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Ok(json!({"tabs": list, "backend": backend}));
    }
    let (_target_id, session) = thread_session(&conn, thread, mode).await?;
    if matches!(
        name,
        "browser_open"
            | "browser_navigate"
            | "browser_run"
            | "browser_click"
            | "browser_fill"
            | "browser_type"
            | "browser_press_key"
    ) {
        note_agent_activity(&session);
    }
    match name {
        "browser_open" | "browser_navigate" => {
            let url = args
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{name} requires url"))?;
            NetworkPolicy::default()
                .check_navigation(url)
                .map_err(|reason| format!("BrowserNavigationBlocked: {reason}"))?;
            conn.call("Page.navigate", json!({"url": url}), Some(&session))
                .await?;
            wait_for_load(&conn, &session).await;
            let info = cdp_eval(
                &conn,
                &session,
                "({url: location.href, title: document.title})",
            )
            .await?;
            let mut out = json!({"backend": backend});
            if let (Value::Object(o), Value::Object(info)) = (&mut out, info) {
                for (k, v) in info {
                    o.insert(k, v);
                }
            }
            Ok(out)
        }
        "browser_snapshot" => {
            let text = cdp_eval(
                &conn,
                &session,
                "document.body ? document.body.innerText : ''",
            )
            .await?;
            let meta = cdp_eval(
                &conn,
                &session,
                "({url: location.href, title: document.title})",
            )
            .await?;
            Ok(json!({
                "backend": backend,
                "url": meta.get("url").cloned().unwrap_or(Value::Null),
                "title": meta.get("title").cloned().unwrap_or(Value::Null),
                "snapshot": text,
            }))
        }
        "browser_evaluate" => {
            let expression = args
                .get("expression")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_evaluate requires expression".to_string())?;
            let value = cdp_eval(&conn, &session, expression).await?;
            Ok(json!({"backend": backend, "value": value}))
        }
        "browser_run" => {
            let code = args
                .get("code")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_run requires code".to_string())?;
            let context_id = create_agent_context(&conn, &session).await?;
            cdp_eval_in_context(&conn, &session, CREDENTIALS_BRIDGE_JS, Some(context_id)).await?;
            let expression = crate::runexpr::browser_run_expression(code);
            let value = cdp_eval_in_context(&conn, &session, &expression, Some(context_id)).await?;
            Ok(json!({"backend": backend, "value": value}))
        }
        "browser_click" => {
            let sel = selector_js(args)?;
            if trusted_click(&conn, &session, &sel).await.is_err() {
                let expr = format!(
                    "(()=>{{const el=document.querySelector({});if(!el)throw new Error('click target not found');el.click();return true}})()",
                    js_string(&sel)
                );
                cdp_eval(&conn, &session, &expr).await?;
            }
            Ok(json!({"backend": backend, "clicked": sel}))
        }
        "browser_fill" | "browser_type" => {
            let sel = selector_js(args)?;
            let text = args
                .get("value")
                .or_else(|| args.get("text"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let expr = format!(
                "(()=>{{const el=document.querySelector({});if(!el)throw new Error('type target not found');el.focus();el.value={};el.dispatchEvent(new Event('input',{{bubbles:true}}));return true}})()",
                js_string(&sel),
                js_string(text)
            );
            cdp_eval(&conn, &session, &expr).await?;
            Ok(json!({"backend": backend, "filled": sel}))
        }
        "browser_press_key" => {
            let key = args
                .get("key")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_press_key requires key".to_string())?;
            let expr = format!(
                "(()=>{{const el=document.activeElement||document.body;el.dispatchEvent(new KeyboardEvent('keydown',{{key:{},bubbles:true}}));el.dispatchEvent(new KeyboardEvent('keyup',{{key:{},bubbles:true}}));return true}})()",
                js_string(key),
                js_string(key)
            );
            cdp_eval(&conn, &session, &expr).await?;
            Ok(json!({"backend": backend, "pressed": key}))
        }
        "browser_scroll" => {
            let dx = args.get("dx").and_then(Value::as_i64).unwrap_or(0);
            let dy = args.get("dy").and_then(Value::as_i64).unwrap_or(400);
            cdp_eval(&conn, &session, &format!("window.scrollBy({dx},{dy}), true"))
                .await?;
            Ok(json!({"backend": backend, "scrolled": true}))
        }
        "browser_back" => {
            cdp_eval(&conn, &session, "history.back(), true").await?;
            Ok(json!({"backend": backend}))
        }
        "browser_forward" => {
            cdp_eval(&conn, &session, "history.forward(), true").await?;
            Ok(json!({"backend": backend}))
        }
        "browser_reload" => {
            conn.call("Page.reload", json!({}), Some(&session)).await?;
            Ok(json!({"backend": backend}))
        }
        "browser_screenshot" => {
            let result = conn
                .call(
                    "Page.captureScreenshot",
                    json!({"format": "png"}),
                    Some(&session),
                )
                .await?;
            let data = result.get("data").cloned().unwrap_or(Value::Null);
            let mut out = json!({
                "backend": backend,
                "image": {"data": data, "mimeType": "image/png"},
            });
            if let Some(path) = args.get("path").and_then(Value::as_str) {
                use base64::Engine as _;
                let bytes = data
                    .as_str()
                    .and_then(|b64| base64::engine::general_purpose::STANDARD.decode(b64).ok())
                    .ok_or_else(|| "screenshot data missing for file output".to_string())?;
                if let Some(parent) = std::path::Path::new(path).parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::write(path, &bytes)
                    .map_err(|e| format!("could not write screenshot to {path}: {e}"))?;
                if let Value::Object(ref mut obj) = out {
                    obj.insert("artifactPath".into(), json!(path));
                }
            }
            Ok(out)
        }
        "browser_wait_for" => {
            let sel = args
                .get("selector")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_wait_for requires selector".to_string())?;
            let timeout_ms = args
                .get("timeout")
                .and_then(Value::as_u64)
                .unwrap_or(30)
                * 1000;
            let start = std::time::Instant::now();
            loop {
                let found = cdp_eval(
                    &conn,
                    &session,
                    &format!("!!document.querySelector({})", js_string(sel)),
                )
                .await
                .unwrap_or(Value::Bool(false));
                if found.as_bool() == Some(true) {
                    return Ok(json!({"backend": backend, "found": sel}));
                }
                if start.elapsed().as_millis() as u64 > timeout_ms {
                    return Err(format!("browser_wait_for timed out waiting for {sel}"));
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
        }
        "browser_wait_for_text" => {
            let text = args
                .get("text")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_wait_for_text requires text".to_string())?;
            let timeout_ms = args
                .get("timeout")
                .and_then(Value::as_u64)
                .unwrap_or(30)
                * 1000;
            let start = std::time::Instant::now();
            loop {
                let found = cdp_eval(
                    &conn,
                    &session,
                    &format!(
                        "(document.body?document.body.innerText:'').includes({})",
                        js_string(text)
                    ),
                )
                .await
                .unwrap_or(Value::Bool(false));
                if found.as_bool() == Some(true) {
                    return Ok(json!({"backend": backend, "found": text}));
                }
                if start.elapsed().as_millis() as u64 > timeout_ms {
                    return Err(format!("browser_wait_for_text timed out waiting for {text}"));
                }
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
        }
        "browser_console" => {
            let entries = console_logs()
                .lock()
                .ok()
                .and_then(|logs| logs.get(&session).cloned())
                .unwrap_or_default();
            Ok(json!({"backend": backend, "entries": entries}))
        }
        "browser_select" => {
            let sel = selector_js(args)?;
            let value = args
                .get("value")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_select requires value".to_string())?;
            let expr = format!(
                "(()=>{{const el=document.querySelector({});if(!el)throw new Error('select target not found');el.value={};el.dispatchEvent(new Event('change',{{bubbles:true}}));return el.value}})()",
                js_string(&sel),
                js_string(value)
            );
            cdp_eval(&conn, &session, &expr).await?;
            Ok(json!({"backend": backend, "selected": value}))
        }
        "browser_upload" => {
            let sel = selector_js(args)?;
            let files: Vec<String> = args
                .get("files")
                .and_then(Value::as_array)
                .map(|f| {
                    f.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .or_else(|| {
                    args.get("path")
                        .and_then(Value::as_str)
                        .map(|p| vec![p.to_string()])
                })
                .ok_or_else(|| "browser_upload requires files or path".to_string())?;
            // DOM.setFileInputFiles needs the node id of the input element.
            let doc = conn
                .call("DOM.getDocument", json!({}), Some(&session))
                .await?;
            let root_id = doc
                .pointer("/root/nodeId")
                .and_then(Value::as_i64)
                .ok_or_else(|| "DOM.getDocument missing root".to_string())?;
            let node = conn
                .call(
                    "DOM.querySelector",
                    json!({"nodeId": root_id, "selector": sel}),
                    Some(&session),
                )
                .await?;
            let node_id = node.get("nodeId").and_then(Value::as_i64).unwrap_or(0);
            if node_id == 0 {
                return Err(format!("upload target '{sel}' not found"));
            }
            conn.call(
                "DOM.setFileInputFiles",
                json!({"files": files, "nodeId": node_id}),
                Some(&session),
            )
            .await?;
            Ok(json!({"backend": backend, "uploaded": files}))
        }
        "browser_drag" => {
            // HTML5 drag events are synthesized in-page; CDP mouse events only
            // cover simple pointer drags, which is what most pages implement.
            let from = args
                .get("from")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_drag requires from".to_string())?;
            let to = args
                .get("to")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_drag requires to".to_string())?;
            let expr = format!(
                r#"(()=>{{
const src=document.querySelector({});const dst=document.querySelector({});
if(!src)throw new Error('drag source not found');if(!dst)throw new Error('drag target not found');
const dataTransfer=new DataTransfer();
for(const type of ['dragstart']){{src.dispatchEvent(new DragEvent(type,{{bubbles:true,dataTransfer}}));}}
dst.dispatchEvent(new DragEvent('dragover',{{bubbles:true,dataTransfer}}));
dst.dispatchEvent(new DragEvent('drop',{{bubbles:true,dataTransfer}}));
src.dispatchEvent(new DragEvent('dragend',{{bubbles:true,dataTransfer}}));
return true}})()"#,
                js_string(from),
                js_string(to)
            );
            cdp_eval(&conn, &session, &expr).await?;
            Ok(json!({"backend": backend, "dragged": true}))
        }
        "browser_download" => {
            // Arms the download behavior for the browser and waits for the
            // next completed download. The caller is expected to trigger the
            // download (click) first or pass a selector to click here.
            let dir = cdp_profile_dir(mode).join("downloads");
            let _ = std::fs::create_dir_all(&dir);
            conn.call(
                "Browser.setDownloadBehavior",
                json!({
                    "behavior": "allow",
                    "downloadPath": dir.display().to_string(),
                    "eventsEnabled": true,
                }),
                None,
            )
            .await?;
            if let Ok(sel) = selector_js(args) {
                let expr = format!(
                    "(()=>{{const el=document.querySelector({});if(!el)throw new Error('download target not found');el.click();return true}})()",
                    js_string(&sel)
                );
                cdp_eval(&conn, &session, &expr).await?;
            }
            let timeout_ms = args
                .get("timeout")
                .and_then(Value::as_u64)
                .unwrap_or(15_000);
            let start = std::time::Instant::now();
            let baseline: Vec<String> = downloads()
                .lock()
                .map(|dl| dl.keys().cloned().collect())
                .unwrap_or_default();
            loop {
                let done = downloads().lock().ok().and_then(|dl| {
                    dl.iter()
                        .find(|(guid, _)| !baseline.contains(guid))
                        .map(|(_, v)| v.clone())
                });
                if let Some(entry) = done {
                    return Ok(json!({
                        "backend": backend,
                        "download": entry,
                        "directory": dir.display().to_string(),
                    }));
                }
                if start.elapsed().as_millis() as u64 > timeout_ms {
                    return Err("browser_download timed out waiting for a completed download".to_string());
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
        "browser_close" => {
            let key = format!("{}:{}", mode.backend_name(), thread.unwrap_or(""));
            let target = cdp_tabs()
                .lock()
                .ok()
                .and_then(|mut t| t.remove(&key).map(|pair| pair.0));
            if let Some(target_id) = target {
                let _ = conn
                    .call("Target.closeTarget", json!({"targetId": target_id}), None)
                    .await;
            }
            Ok(json!({"backend": backend, "closed": true}))
        }
        other => Err(format!("browser tool {other} is not implemented for chrome-cdp")),
    }
}
