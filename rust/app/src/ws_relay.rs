//! WebSocket relay for the RPC upgrade paths (`/ws`, `/ws/bootstrap`,
//! device/computer frame sockets). Each client upgrade is answered by opening
//! a matching upgrade against the original and piping frames both ways — the
//! byte-level equivalent of the HTTP proxy.
//!
//! `/ws/bootstrap` additionally intercepts `bootstrap.negotiate` requests and
//! answers them natively (same `WsBootstrapNegotiateResult` the HTTP
//! `/ws/negotiate` route produces); every other frame relays upstream.

use axum::Router;
use axum::extract::State;
use axum::extract::ws::{Message as ClientMessage, WebSocket, WebSocketUpgrade};
use axum::http::{HeaderMap, Uri};
use axum::response::Response;
use axum::routing::get;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::mpsc::unbounded_channel;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message as UpstreamMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use crate::original::Original;

/// Forward the headers that matter to the original's admission and auth.
/// `origin` is deliberately excluded: the original gates upgrades on a trusted
/// origin list and the webview's dev-server origin (e.g. localhost:4577) is not
/// on it — omitting the header passes admission while `token` still auths.
const FORWARDED: &[&str] = &[
    "cookie",
    "authorization",
    "sec-websocket-protocol",
    "user-agent",
    "x-forwarded-for",
    "x-forwarded-proto",
];

const BOOTSTRAP_NEGOTIATE: &str = "bootstrap.negotiate";

pub fn router() -> Router<Original> {
    Router::new()
        .route("/ws", get(|s, w, u, h| upgrade(s, w, u, h, false)))
        .route("/ws/bootstrap", get(|s, w, u, h| upgrade(s, w, u, h, true)))
        .route(
            "/ws/device-frames",
            get(|s, w, u, h| upgrade(s, w, u, h, false)),
        )
        .route(
            "/ws/computer-frames",
            get(|s, w, u, h| upgrade(s, w, u, h, false)),
        )
        .route("/ws/host/v2", get(|s, w, u, h| upgrade(s, w, u, h, false)))
}

async fn upgrade(
    State(original): State<Original>,
    ws: WebSocketUpgrade,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
) -> Response {
    ws.on_upgrade(move |socket| relay(socket, original, uri, headers, intercept))
}

/// `todo.*` requests served by the native store; the string check is a fast
/// prefilter so ordinary traffic still takes the relay path unparsed.
/// `Some(None)` marks a handled stream request (the reply is the chunk flow).
fn intercept_todo(
    text: &str,
    todos: &std::sync::Arc<crate::todo::TodoStore>,
    native_tx: &tokio::sync::mpsc::UnboundedSender<ClientMessage>,
    streams: &std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    >,
) -> Option<Option<ClientMessage>> {
    if !text.contains("\"todo.")
        && !text.contains("\"server.get")
        && !text.contains("\"shell.openInEditor\"")
        && !text.contains("\"git.")
        && !text.contains("\"filesystem.browse")
        && !text.contains("server.listWorktrees")
        && !text.contains("projects.listDirectories")
        && !text.contains("projects.discoverScripts")
        && !text.contains("projects.prewarmSearchIndex")
        && !text.contains("projects.searchEntries")
        && !text.contains("projects.searchLocalEntries")
        && !text.contains("projects.resolveWorkspaceFileReferences")
        && !text.contains("projects.resolveOutOfRootFileReference")
        && !text.contains("projects.readFile")
        && !text.contains("projects.writeFile")
        && !text.contains("projects.searchContent")
        && !text.contains("projects.createLocalFilePreviewGrant")
        && !text.contains("projects.listDevServers")
        && !text.contains("projects.runDevServer")
        && !text.contains("projects.stopDevServer")
        && !text.contains("projects.subscribeDevServerEvents")
        && !text.contains("projects.subscribeFileChange")
        && !text.contains("server.upsertKeybinding")
        && !text.contains("server.editKeybindings")
        && !text.contains("server.listLocalServers")
        && !text.contains("server.stopLocalServer")
        && !text.contains("server.getDiagnostics")
        && !text.contains("server.desktopUpdateStatus")
        && !text.contains("server.desktopUpdate")
        && !text.contains("server.getConfig")
        && !text.contains("server.getSettings")
        && !text.contains("server.updateSettings")
        && !text.contains("server.subscribeSettings")
        && !text.contains("server.subscribeConfig")
        && !text.contains("server.subscribeProviderStatuses")
        && !text.contains("server.subscribeLifecycle")
        && !text.contains("server.subscribeKeepAwake")
        && !text.contains("stats.")
        && !text.contains("terminal.")
        && !text.contains("orchestration.getSnapshot")
        && !text.contains("orchestration.getShellSnapshot")
        && !text.contains("orchestration.getThreadDetailSnapshot")
        && !text.contains("orchestration.searchThreads")
        && !text.contains("githubInbox.")
    {
        return None;
    }
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    if msg.get("_tag")?.as_str()? != "Request" {
        return None;
    }
    let tag = msg.get("tag")?.as_str()?;
    if !tag.starts_with("todo.")
        && tag != "server.getEnvironment"
        && tag != "server.getRuntimeStatus"
        && tag != "shell.openInEditor"
        && !crate::git_domain::handles(tag)
        && tag != "filesystem.browse"
        && tag != "server.listWorktrees"
        && tag != "projects.listDirectories"
        && tag != "projects.discoverScripts"
        && tag != "projects.prewarmSearchIndex"
        && tag != "projects.searchEntries"
        && tag != "projects.searchLocalEntries"
        && tag != "projects.resolveWorkspaceFileReferences"
        && tag != "projects.resolveOutOfRootFileReference"
        && tag != "projects.readFile"
        && tag != "projects.writeFile"
        && tag != "projects.searchContent"
        && tag != "projects.createLocalFilePreviewGrant"
        && tag != "projects.listDevServers"
        && tag != "projects.runDevServer"
        && tag != "projects.stopDevServer"
        && tag != "projects.subscribeDevServerEvents"
        && tag != "projects.subscribeFileChange"
        && tag != "server.upsertKeybinding"
        && tag != "server.editKeybindings"
        && tag != "server.listLocalServers"
        && tag != "server.stopLocalServer"
        && tag != "server.getDiagnostics"
        && tag != "server.desktopUpdateStatus"
        && tag != "server.desktopUpdate"
        && tag != "server.getConfig"
        && tag != "server.getSettings"
        && tag != "server.updateSettings"
        && tag != "server.subscribeSettings"
        && tag != "server.subscribeConfig"
        && tag != "server.subscribeProviderStatuses"
        && tag != "server.subscribeLifecycle"
        && tag != "server.subscribeKeepAwake"
        && tag != "orchestration.getSnapshot"
        && tag != "orchestration.getShellSnapshot"
        && tag != "orchestration.getThreadDetailSnapshot"
        && tag != "orchestration.searchThreads"
        && !tag.starts_with("githubInbox.")
        && !tag.starts_with("stats.")
        && !tag.starts_with("terminal.")
    {
        return None;
    }
    let request_id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = msg
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    if tag == "projects.subscribeDevServerEvents" {
        let mut rx = crate::projects_devserver::subscribe();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let chunk = serde_json::json!({
                "_tag": "Chunk", "requestId": rid,
                "values": [crate::projects_devserver::current_snapshot()],
            });
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            while let Ok(event) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "projects.subscribeFileChange" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let cwd = payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(std::path::PathBuf::from);
        let rel = payload
            .get("relativePath")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let (Some(cwd), Some(rel)) = (cwd, rel) else {
            let exit = serde_json::json!({
                "_tag": "Failure",
                "cause": [{
                    "_tag": "Fail",
                    "error": error_json(crate::todo::WsErr(
                        "cwd and relativePath are required".to_string(),
                        Some("VALIDATION"),
                    )),
                }],
            });
            let reply = serde_json::json!({
                "_tag": "Exit", "requestId": request_id, "exit": exit,
            });
            return Some(Some(ClientMessage::Text(reply.to_string().into())));
        };
        let task = tokio::spawn(async move {
            let tx2 = tx.clone();
            let rid2 = rid.clone();
            let _ = crate::projects_devserver::poll_watch(cwd, rel, move |event| {
                let chunk = serde_json::json!({
                    "_tag": "Chunk", "requestId": rid2, "values": [event],
                });
                tx2.send(ClientMessage::Text(chunk.to_string().into()))
                    .is_ok()
            })
            .await;
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "terminal.subscribeEvents" {
        let mut rx = crate::terminal::subscribe();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "server.subscribeSettings" {
        let (snapshot, mut rx) = crate::server_settings::subscribe();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            // Upstream subscribeServerSettings streams raw {settings} objects
            // (the typed settingsUpdated envelope belongs to subscribeConfig).
            let chunk =
                serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [{"settings": snapshot}]});
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            while let Ok(settings) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [{"settings": settings}]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "server.subscribeLifecycle" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let events = crate::server_lifecycle::snapshot_events();
            let chunk = serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": events});
            let _ = tx.send(ClientMessage::Text(chunk.to_string().into()));
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "server.subscribeConfig" {
        let mut rx = crate::server_settings::changes();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let event = serde_json::json!({
                "type": "snapshot",
                "config": crate::server_lifecycle::get_config(),
            });
            let chunk = serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            while let Ok(settings) = rx.recv().await {
                let event = serde_json::json!({"type": "settingsUpdated", "payload": {"settings": settings}});
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "server.subscribeProviderStatuses" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let chunk = serde_json::json!({
                "_tag": "Chunk", "requestId": rid,
                "values": [{"providers": []}],
            });
            // Provider statuses never change natively yet; hold the stream open.
            if tx
                .send(ClientMessage::Text(chunk.to_string().into()))
                .is_err()
            {
                return;
            }
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "server.subscribeKeepAwake" {
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut last_mode: Option<String> = None;
            let mut rx = crate::server_settings::changes();
            let send_state = |mode_hint: &mut Option<String>| -> Option<String> {
                let state = crate::server_lifecycle::keep_awake_state();
                let mode = state
                    .get("mode")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned);
                if mode == *mode_hint {
                    return None;
                }
                *mode_hint = mode;
                Some(serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [{"keepAwake": state}]}).to_string())
            };
            if let Some(chunk) = send_state(&mut last_mode) {
                if tx.send(ClientMessage::Text(chunk.into())).is_err() {
                    return;
                }
            }
            while let Ok(_settings) = rx.recv().await {
                if let Some(chunk) = send_state(&mut last_mode) {
                    if tx.send(ClientMessage::Text(chunk.into())).is_err() {
                        return;
                    }
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    if tag == "todo.subscribe" {
        // Snapshot first, then every change — mirrors `streamChanges`
        // (subscribe-then-read ordering is preserved by the store lock).
        let todos = todos.clone();
        let tx = native_tx.clone();
        let rid = request_id.clone();
        let task = tokio::spawn(async move {
            let mut rx = todos.subscribe();
            if let Ok(serde_json::Value::Object(mut snapshot)) =
                todos.handle("todo.list", serde_json::Value::Null)
            {
                let event = serde_json::json!({"type": "snapshot", "todos": snapshot.remove("todos").unwrap_or(serde_json::json!([]))});
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
            while let Ok(event) = rx.recv().await {
                let chunk =
                    serde_json::json!({"_tag": "Chunk", "requestId": rid, "values": [event]});
                if tx
                    .send(ClientMessage::Text(chunk.to_string().into()))
                    .is_err()
                {
                    return;
                }
            }
        });
        if let Ok(mut map) = streams.lock() {
            map.insert(request_id, task);
        }
        return Some(None);
    }
    let outcome = if tag == "filesystem.browse" {
        crate::fs_browse::browse(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.listWorktrees" {
        crate::list_worktrees::list_worktrees()
            .map(Some)
            .map_err(error_json)
    } else if tag.starts_with("terminal.") {
        crate::terminal::handle(tag, &payload).map_err(error_json)
    } else if tag == "server.upsertKeybinding" {
        crate::server_keybindings::upsert_keybinding(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.editKeybindings" {
        crate::server_keybindings::edit_keybindings(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getDiagnostics" {
        crate::server_diagnostics::get_diagnostics()
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.listLocalServers" {
        crate::server_local_servers::list_local_servers()
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.stopLocalServer" {
        crate::server_local_servers::stop_local_server(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.desktopUpdateStatus" {
        crate::server_desktop_update::desktop_update_status()
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.desktopUpdate" {
        crate::server_desktop_update::desktop_update(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getConfig" {
        Ok(Some(crate::server_lifecycle::get_config()))
    } else if tag.starts_with("stats.") {
        crate::stats::handle(tag, &payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getSettings" {
        crate::server_settings::get_settings(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.updateSettings" {
        crate::server_settings::update_settings(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag.starts_with("projects.readFile")
        || tag == "projects.writeFile"
        || tag == "projects.searchContent"
        || tag == "projects.createLocalFilePreviewGrant"
    {
        crate::projects_fileops::handle(tag, &payload).map_err(error_json)
    } else if tag == "projects.listDevServers" {
        crate::projects_devserver::list_dev_servers()
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.runDevServer" {
        crate::projects_devserver::run_dev_server(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.stopDevServer" {
        crate::projects_devserver::stop_dev_server(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.resolveWorkspaceFileReferences" {
        crate::projects_refs::resolve_workspace_file_references(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.resolveOutOfRootFileReference" {
        crate::projects_refs::resolve_out_of_root_file_reference(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.prewarmSearchIndex" {
        crate::projects_search::prewarm_search_index(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.searchEntries" {
        crate::projects_search::search_entries(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.searchLocalEntries" {
        crate::projects_search::search_local_entries(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.discoverScripts" {
        crate::projects_fs::discover_scripts(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "projects.listDirectories" {
        crate::projects_fs::list_directories(&payload)
            .map(Some)
            .map_err(error_json)
    } else if crate::git_domain::handles(tag) {
        match crate::git_domain::handle(tag, &payload) {
            Ok(value) => Ok(value),
            Err(e) => Err(error_json(e)),
        }
    } else if tag == "server.getEnvironment" {
        crate::server_env::get_environment(&crate::todo::state_dir())
            .map_err(crate::todo::err_pub)
            .map(Some)
            .map_err(error_json)
    } else if tag == "server.getRuntimeStatus" {
        Ok(Some(crate::runtime_status::get_runtime_status()))
    } else if tag == "shell.openInEditor" {
        crate::shell_editor::open_in_editor(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.getSnapshot" {
        crate::orchestration_snapshot::get_snapshot()
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.getShellSnapshot" {
        crate::orchestration_snapshot::get_shell_snapshot()
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.getThreadDetailSnapshot" {
        crate::orchestration_snapshot::get_thread_detail_snapshot(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag == "orchestration.searchThreads" {
        crate::orchestration_snapshot::search_threads(&payload)
            .map(Some)
            .map_err(error_json)
    } else if tag.starts_with("githubInbox.") {
        crate::github_inbox::handle(tag, &payload)
            .map_err(error_json)
            .and_then(|v| match v {
                Some(v) => Ok(Some(v)),
                None => Err(error_json(crate::todo::WsErr(
                    format!("unhandled native tag {tag}"),
                    Some("DISPATCH"),
                ))),
            })
    } else {
        todos.handle(tag, payload).map(Some).map_err(error_json)
    };
    let exit = match outcome {
        // `Schema.Void` successes encode without a `value` key.
        Ok(value) => match value {
            Some(value) => serde_json::json!({"_tag": "Success", "value": value}),
            None => serde_json::json!({"_tag": "Success"}),
        },
        Err(error) => serde_json::json!({
            "_tag": "Failure",
            "cause": [{
                "_tag": "Fail",
                "error": error,
            }]
        }),
    };
    let reply = serde_json::json!({"_tag": "Exit", "requestId": request_id, "exit": exit});
    Some(Some(ClientMessage::Text(reply.to_string().into())))
}

fn error_json(crate::todo::WsErr(message, code): crate::todo::WsErr) -> serde_json::Value {
    serde_json::json!({
        "_tag": "WsRpcError",
        "message": message,
        "code": code,
        "retryable": false,
    })
}

/// A `bootstrap.negotiate` request answered natively, or `None` to relay.
fn intercept_bootstrap(text: &str) -> Option<(String, ClientMessage)> {
    let msg: serde_json::Value = serde_json::from_str(text).ok()?;
    if msg.get("_tag")?.as_str()? != "Request" || msg.get("tag")?.as_str()? != BOOTSTRAP_NEGOTIATE {
        return None;
    }
    let request_id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = msg
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let get_int = |k: &str| payload.get(k).and_then(|v| v.as_i64()).unwrap_or(-1);
    let required: Vec<String> = payload
        .get("requiredCapabilities")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|c| c.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    let exit = match crate::native_ws::negotiate(
        get_int("protocolEpoch"),
        get_int("minRevision"),
        get_int("maxRevision"),
        &required,
    ) {
        Ok(value) => json!({"_tag": "Success", "value": value}),
        Err(err) => json!({
            "_tag": "Failure",
            "cause": [{"_tag": "Fail", "error": err}]
        }),
    };
    let reply = json!({"_tag": "Exit", "requestId": request_id, "exit": exit});
    Some((request_id, ClientMessage::Text(reply.to_string().into())))
}

/// Replaces the `x-synara-server-instance` query value in a socket target.
fn rewrite_server_instance(target: &str, id: &str) -> String {
    let Some((base, query)) = target.split_once('?') else {
        return target.to_string();
    };
    let mut params: Vec<String> = query
        .split('&')
        .filter(|p| !p.starts_with("x-synara-server-instance="))
        .map(str::to_string)
        .collect();
    params.push(format!("x-synara-server-instance={id}"));
    format!("{base}?{}", params.join("&"))
}

/// Client request for the upstream socket with the admission headers copied.
fn build_upstream_request(
    target: &str,
    headers: &HeaderMap,
) -> Option<tokio_tungstenite::tungstenite::handshake::client::Request> {
    let mut request = target.into_client_request().ok()?;
    for name in FORWARDED {
        if let Some(value) = headers.get(*name)
            && let Ok(header_name) =
                tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(
                    name.as_bytes(),
                )
            && let Ok(header_value) =
                tokio_tungstenite::tungstenite::http::header::HeaderValue::from_bytes(
                    value.as_bytes(),
                )
        {
            request.headers_mut().insert(header_name, header_value);
        }
    }
    Some(request)
}

async fn relay(
    client: WebSocket,
    original: Original,
    uri: Uri,
    headers: HeaderMap,
    intercept: bool,
) {
    let todos = original.todos.clone();
    // The native `/ws/negotiate` issues OUR server-instance id, but upstream
    // gates upgrades on its own — swap the param so relayed sockets pass the
    // generation check (fetched lazily; refetched once after a 426).
    let mut target = format!(
        "{}{}",
        original.ws_base(),
        uri.path_and_query().map_or("/ws", |p| p.as_str())
    );
    let mut refreshed = false;
    if target.contains("x-synara-server-instance=")
        && let Some(id) = original.upstream_instance(false).await
    {
        target = rewrite_server_instance(&target, &id);
    }
    let (upstream, resp) = loop {
        let request = match build_upstream_request(&target, &headers) {
            Some(r) => r,
            None => {
                eprintln!("ws_relay: request build failed for {target}");
                return;
            }
        };
        match connect_async(request).await {
            Ok(pair) => break pair,
            Err(e) => {
                if !refreshed
                    && target.contains("x-synara-server-instance=")
                    && let Some(id) = original.upstream_instance(true).await
                {
                    refreshed = true;
                    target = rewrite_server_instance(&target, &id);
                    continue;
                }
                eprintln!("ws_relay: upstream connect to {target} failed: {e}");
                return;
            }
        }
    };
    let _ = resp;
    let (mut client_tx, mut client_rx) = client.split();
    let (mut up_tx, mut up_rx) = upstream.split();
    // Debug tap: SYNARA_TAP_WS=/path appends every text frame (">" to
    // upstream, "<" from upstream) so relayed wire shapes can be studied.
    let tap_path = std::env::var("SYNARA_TAP_WS").ok();
    let tap = std::sync::Arc::new(move |dir: &str, t: &str| {
        if let Some(p) = &tap_path
            && let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(p)
        {
            use std::io::Write;
            let _ = writeln!(f, "{dir}{t}");
        }
    });
    let (native_tx, mut native_rx) = unbounded_channel::<ClientMessage>();
    let streams: std::sync::Arc<
        std::sync::Mutex<std::collections::HashMap<String, tokio::task::JoinHandle<()>>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(std::collections::HashMap::new()));
    let streams_in = streams.clone();

    let tap_out = tap.clone();
    let to_upstream = async move {
        while let Some(msg) = client_rx.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("ws_relay: client read error: {e}");
                    break;
                }
            };
            let out = match msg {
                ClientMessage::Text(t) => {
                    if intercept && t.contains(BOOTSTRAP_NEGOTIATE) {
                        if let Some((_, reply)) = intercept_bootstrap(&t) {
                            eprintln!("ws_relay: native bootstrap.negotiate reply sent");
                            let _ = native_tx.send(reply);
                            continue;
                        }
                    }
                    if let Some(reply) = intercept_todo(&t, &todos, &native_tx, &streams_in) {
                        eprintln!("ws_relay: native todo reply");
                        if let Some(reply) = reply {
                            let _ = native_tx.send(reply);
                        }
                        continue;
                    }
                    // `Ack`/`Interrupt`/`Eof` addressed at a native stream stay
                    // on this side: upstream never saw the request id, so the
                    // frame would be a protocol violation there and poison the
                    // whole socket. Interrupt aborts the pump; Eof drains all.
                    if (t.contains("\"Interrupt\"") || t.contains("\"Eof\"") || t.contains("\"Ack\""))
                        && let Ok(msg) = serde_json::from_str::<serde_json::Value>(&t)
                    {
                        let tag = msg.get("_tag").and_then(|v| v.as_str()).unwrap_or("");
                        if tag == "Interrupt" || tag == "Eof" || tag == "Ack" {
                            let rid = msg.get("requestId").and_then(|v| v.as_str()).unwrap_or("");
                            let owned = if let Ok(mut map) = streams_in.lock() {
                                if tag == "Eof" {
                                    map.drain().map(|(_, h)| h).collect::<Vec<_>>()
                                } else if tag == "Interrupt" {
                                    map.remove(rid).into_iter().collect::<Vec<_>>()
                                } else {
                                    Vec::new()
                                }
                            } else {
                                Vec::new()
                            };
                            let known_native = if let Ok(map) = streams_in.lock() {
                                map.contains_key(rid)
                            } else {
                                false
                            };
                            if !owned.is_empty() {
                                for h in owned {
                                    h.abort();
                                }
                                continue;
                            }
                            if known_native {
                                continue;
                            }
                        }
                    }
                    tap_out(">", &t);
                    UpstreamMessage::Text(t.to_string().into())
                }
                ClientMessage::Binary(b) => {
                    tap_out(">", &format!(
                        "[bin {}B] {}",
                        b.len(),
                        b.iter()
                            .take(160)
                            .map(|x| format!("{x:02x}"))
                            .collect::<String>()
                    ));
                    UpstreamMessage::Binary(b)
                },
                ClientMessage::Ping(p) => UpstreamMessage::Ping(p),
                ClientMessage::Pong(p) => UpstreamMessage::Pong(p),
                ClientMessage::Close(c) => {
                    eprintln!("ws_relay: client closed socket: {c:?}");
                    let _ = up_tx
                        .send(UpstreamMessage::Close(c.map(|f| {
                            tokio_tungstenite::tungstenite::protocol::CloseFrame {
                                code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::from(
                                    f.code,
                                ),
                                reason: f.reason.to_string().into(),
                            }
                        })))
                        .await;
                    break;
                }
            };
            if up_tx.send(out).await.is_err() {
                eprintln!("ws_relay: upstream send channel closed");
                break;
            }
        }
        eprintln!("ws_relay: client->upstream loop ended");
    };

    let to_client = async move {
        loop {
            let out = tokio::select! {
                msg = up_rx.next() => match msg {
                    Some(Err(e)) => {
                        eprintln!("ws_relay: upstream read error: {e}");
                        break;
                    }
                    None => {
                        eprintln!("ws_relay: upstream socket ended");
                        break;
                    }
                    Some(Ok(m)) => match m {
                        UpstreamMessage::Text(t) => {
                            tap("<", &t);
                            ClientMessage::Text(t.to_string().into())
                        },
                        UpstreamMessage::Binary(b) => {
                            tap("<", &format!(
                                "[bin {}B] {}",
                                b.len(),
                                b.iter()
                                    .take(160)
                                    .map(|x| format!("{x:02x}"))
                                    .collect::<String>()
                            ));
                            ClientMessage::Binary(b)
                        },
                        UpstreamMessage::Ping(p) => ClientMessage::Ping(p),
                        UpstreamMessage::Pong(p) => ClientMessage::Pong(p),
                        UpstreamMessage::Frame(f) => {
                            let p = f.into_payload();
                            tap("<", &format!("[frame {}B]", p.len()));
                            ClientMessage::Binary(p)
                        },
                        UpstreamMessage::Close(c) => {
                            let _ = client_tx
                                .send(ClientMessage::Close(c.map(|f| {
                                    axum::extract::ws::CloseFrame {
                                        code: u16::from(f.code),
                                        reason: f.reason.to_string().into(),
                                    }
                                })))
                                .await;
                            break;
                        }
                    },
                },
                msg = native_rx.recv() => match msg {
                    Some(m) => {
                        if let ClientMessage::Text(t) = &m {
                            tap("<", t);
                        }
                        m
                    }
                    None => continue,
                },
            };
            if client_tx.send(out).await.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = to_upstream => {}
        _ = to_client => {}
    }
}
