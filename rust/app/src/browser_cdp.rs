//! Chrome DevTools Protocol fallback backend. When neither Obscura nor Ego
//! lite is installed, Synara launches the user's own Chromium browser
//! (Chrome/Edge/Brave/Chromium) with a dedicated profile and drives it over
//! CDP — nothing extra ships in the bundle, and the browser reads its own
//! profile; Synara never touches the user's real browser profile or cookies.

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};

use crate::todo::WsErr;



struct CdpProcess {
    child: Child,
    port: u16,
}

struct CdpConnection {
    tx: mpsc::UnboundedSender<String>,
    pending: Arc<Mutex<HashMap<u64, oneshot::Sender<Value>>>>,
    next_id: AtomicU64,
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

fn cdp_state() -> &'static Mutex<(Option<CdpProcess>, Option<Arc<CdpConnection>>)> {
    static STATE: OnceLock<Mutex<(Option<CdpProcess>, Option<Arc<CdpConnection>>)>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new((None, None)))
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

fn cdp_profile_dir() -> std::path::PathBuf {
    let base = std::env::var("SYNARA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into()))
                .join(".synara")
        });
    base.join("chrome-cdp-profile")
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
    let pending_reader = pending.clone();
    tokio::spawn(async move {
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
        next_id: AtomicU64::new(1),
    }))
}

/// Launch (or reuse) the CDP browser and return its connection.
async fn ensure_cdp() -> Result<Arc<CdpConnection>, String> {
    if let Ok(state) = cdp_state().lock() {
        if let Some(conn) = state.1.clone() {
            return Ok(conn);
        }
    }
    let binary = chrome_path().ok_or_else(|| {
        "no Chromium browser found for the CDP fallback (install Chrome, Edge, Brave or Chromium)".to_string()
    })?;
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|l| l.local_addr())
        .map(|a| a.port())
        .map_err(|e| format!("no free port for cdp: {e}"))?;
    let profile = cdp_profile_dir();
    let _ = std::fs::create_dir_all(&profile);
    let child = Command::new(&binary)
        .args([
            &format!("--remote-debugging-port={port}"),
            &format!("--user-data-dir={}", profile.display()),
            "--no-first-run",
            "--no-default-browser-check",
            "--disable-session-crashed-bubble",
            "--hide-crash-restore-bubble",
            "about:blank",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
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
    let ws_url = version
        .get("webSocketDebuggerUrl")
        .and_then(Value::as_str)
        .ok_or_else(|| "chromium devtools endpoint missing websocket url".to_string())?
        .to_string();
    let conn = connect_ws(&ws_url).await?;
    if let Ok(mut state) = cdp_state().lock() {
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

pub(crate) async fn shutdown_cdp() {
    let child = cdp_state()
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

/// Per-thread tab: each calling thread gets its own target in the shared CDP
/// browser, mirroring upstream's per-thread browser target registry.
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
) -> Result<(String, String), String> {
    let key = thread.unwrap_or("").to_string();
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
    if let Ok(mut tabs) = cdp_tabs().lock() {
        tabs.insert(key, (target_id.clone(), session_id.clone()));
    }
    Ok((target_id, session_id))
}

async fn cdp_eval(
    conn: &CdpConnection,
    session: &str,
    expression: &str,
) -> Result<Value, String> {
    let result = conn
        .call(
            "Runtime.evaluate",
            json!({
                "expression": expression,
                "awaitPromise": true,
                "returnByValue": true,
            }),
            Some(session),
        )
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

pub(crate) fn cdp_available() -> bool {
    chrome_path().is_some()
}

pub(crate) async fn call_cdp_tool(
    name: &str,
    args: &Value,
    thread: Option<&str>,
) -> Result<Value, String> {
    let conn = ensure_cdp().await?;
    if name == "browser_status" {
        return Ok(json!({
            "available": true,
            "backend": "chrome-cdp",
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
        return Ok(json!({"tabs": list, "backend": "chrome-cdp"}));
    }
    let (_target_id, session) = thread_session(&conn, thread).await?;
    match name {
        "browser_open" | "browser_navigate" => {
            let url = args
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| format!("{name} requires url"))?;
            conn.call("Page.navigate", json!({"url": url}), Some(&session))
                .await?;
            let info = cdp_eval(
                &conn,
                &session,
                "({url: location.href, title: document.title})",
            )
            .await?;
            let mut out = json!({"backend": "chrome-cdp"});
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
                "backend": "chrome-cdp",
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
            Ok(json!({"backend": "chrome-cdp", "value": value}))
        }
        "browser_run" => {
            let code = args
                .get("code")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_run requires code".to_string())?;
            let expression = crate::browser_automation::browser_run_expression_pub(code);
            let value = cdp_eval(&conn, &session, &expression).await?;
            Ok(json!({"backend": "chrome-cdp", "value": value}))
        }
        "browser_click" => {
            let sel = selector_js(args)?;
            let expr = format!(
                "(()=>{{const el=document.querySelector({});if(!el)throw new Error('click target not found');el.click();return true}})()",
                js_string(&sel)
            );
            cdp_eval(&conn, &session, &expr).await?;
            Ok(json!({"backend": "chrome-cdp", "clicked": sel}))
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
            Ok(json!({"backend": "chrome-cdp", "filled": sel}))
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
            Ok(json!({"backend": "chrome-cdp", "pressed": key}))
        }
        "browser_scroll" => {
            let dx = args.get("dx").and_then(Value::as_i64).unwrap_or(0);
            let dy = args.get("dy").and_then(Value::as_i64).unwrap_or(400);
            cdp_eval(&conn, &session, &format!("window.scrollBy({dx},{dy}), true"))
                .await?;
            Ok(json!({"backend": "chrome-cdp", "scrolled": true}))
        }
        "browser_back" => {
            cdp_eval(&conn, &session, "history.back(), true").await?;
            Ok(json!({"backend": "chrome-cdp"}))
        }
        "browser_forward" => {
            cdp_eval(&conn, &session, "history.forward(), true").await?;
            Ok(json!({"backend": "chrome-cdp"}))
        }
        "browser_reload" => {
            conn.call("Page.reload", json!({}), Some(&session)).await?;
            Ok(json!({"backend": "chrome-cdp"}))
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
            Ok(json!({
                "backend": "chrome-cdp",
                "image": {"data": data, "mimeType": "image/png"},
            }))
        }
        "browser_wait_for" => {
            let sel = args
                .get("selector")
                .and_then(Value::as_str)
                .ok_or_else(|| "browser_wait_for requires selector".to_string())?;
            let timeout_ms = args
                .get("timeout")
                .and_then(Value::as_u64)
                .unwrap_or(5000);
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
                    return Ok(json!({"backend": "chrome-cdp", "found": sel}));
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
                .unwrap_or(5000);
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
                    return Ok(json!({"backend": "chrome-cdp", "found": text}));
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
            Ok(json!({"backend": "chrome-cdp", "entries": entries}))
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
            Ok(json!({"backend": "chrome-cdp", "selected": value}))
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
            Ok(json!({"backend": "chrome-cdp", "uploaded": files}))
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
            Ok(json!({"backend": "chrome-cdp", "dragged": true}))
        }
        "browser_close" => {
            let key = thread.unwrap_or("").to_string();
            let target = cdp_tabs()
                .lock()
                .ok()
                .and_then(|mut t| t.remove(&key).map(|pair| pair.0));
            if let Some(target_id) = target {
                let _ = conn
                    .call("Target.closeTarget", json!({"targetId": target_id}), None)
                    .await;
            }
            Ok(json!({"backend": "chrome-cdp", "closed": true}))
        }
        other => Err(format!("browser tool {other} is not implemented for chrome-cdp")),
    }
}
