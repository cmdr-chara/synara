use std::ffi::OsString;
use std::io::Write;
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");
const RUN: &[&str] = &[
    "bun",
    "run",
    "apps/server/src/index.ts",
    "--port",
    "{port}",
    "--host",
    "127.0.0.1",
    "--no-browser",
    "--home-dir",
    "./.rec-home",
    "--auth-token",
    "rectoken",
];
const FALLBACK_LOG: &str = "RUST_ANYTHING_FALLBACK_LOG";

fn resolve(word: &str, port: u16) -> OsString {
    let word = word.replace("{port}", &port.to_string());
    if word.starts_with('-') {
        return word.into();
    }
    let path = Path::new(ROOT).join(&word);
    let looks_like_path = word.contains('/') || word.contains('\\') || path.is_file();
    if looks_like_path && path.exists() {
        return std::path::absolute(&path).unwrap_or(path).into_os_string();
    }
    word.into()
}

fn log() -> Option<std::fs::File> {
    let path = std::env::var_os(FALLBACK_LOG)?;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

/// Tells `rust-anything app replay` this build counts its fallbacks, even when it makes none.
pub fn report_fallbacks() {
    let _ = log();
}

/// The original server, started on a free port of its own.
#[derive(Clone)]
pub struct Original {
    base: String,
    client: reqwest::Client,
    child: Arc<Mutex<Option<Child>>>,
    /// The original's `x-synara-server-instance`, fetched on first use; a child
    /// restart mints a fresh UUID, so callers refetch after a 426.
    instance: Arc<Mutex<Option<String>>>,
    /// Native-owned domains: the `todo.*` WS-RPC handlers.
    pub todos: Arc<crate::todo::TodoStore>,
}

impl Original {
    pub async fn start() -> Self {
        // Fixed upstream port: session cookies are named per-port
        // (`synara_session_<port>`), so an ephemeral port would invalidate
        // pairing on every restart. `SYNARA_UPSTREAM_PORT` overrides.
        let port = std::env::var("SYNARA_UPSTREAM_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(4773u16);
        let (program, rest) = RUN.split_first().expect("the original's command");
        let mut child = Command::new(resolve(program, port))
            .args(rest.iter().map(|w| resolve(w, port)))
            .args(std::env::args_os().skip(1))
            .current_dir(ROOT)
            .env("PORT", port.to_string())
            .env("SYNARA_MODE", "desktop")
            .env("SYNARA_DESKTOP_PARENT_STDIN", "1")
            .env("SYNARA_STATIC_DIR", Path::new(ROOT).join("apps/web/dist"))
            .env_remove(FALLBACK_LOG)
            // Desktop-parent marker: stdin stays open for the child's lifetime
            // (its watchdog exits when this closes) and enables desktop mode,
            // where the startup token is a bootstrap credential.
            .stdin(Stdio::piped())
            .spawn()
            .expect("starting the original");
        let deadline = Instant::now() + Duration::from_secs(60);
        while tokio::net::TcpStream::connect(("127.0.0.1", port))
            .await
            .is_err()
        {
            if let Ok(Some(status)) = child.try_wait() {
                panic!("the original exited ({status}) before listening on port {port}");
            }
            assert!(
                Instant::now() < deadline,
                "the original did not listen on port {port}"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("an HTTP client");
        Original {
            base: format!("http://127.0.0.1:{port}"),
            client,
            child: Arc::new(Mutex::new(Some(child))),
            instance: Arc::new(Mutex::new(None)),
            todos: crate::todo::TodoStore::open(&crate::todo::state_dir()),
        }
    }

    /// The original's server-instance UUID, negotiated over HTTP. `refresh`
    /// forces a refetch after a generation change (426).
    pub async fn upstream_instance(&self, refresh: bool) -> Option<String> {
        if !refresh
            && let Ok(guard) = self.instance.lock()
            && let Some(cached) = guard.as_ref()
        {
            return Some(cached.clone());
        }
        let url = format!(
            "{}/ws/negotiate?x-synara-client-build=rust&x-synara-protocol-epoch=1\
             &x-synara-protocol-min-revision=3&x-synara-protocol-max-revision=3",
            self.base
        );
        let id = self.client.get(url).send().await.ok()?.text().await.ok()?;
        let id = serde_json::from_str::<serde_json::Value>(&id)
            .ok()?
            .get("serverInstanceId")
            .and_then(|v| v.as_str())
            .map(str::to_string)?;
        if let Ok(mut guard) = self.instance.lock() {
            *guard = Some(id.clone());
        }
        Some(id)
    }

    /// WebSocket URL of the original (the `base` with a `ws` scheme).
    pub fn ws_base(&self) -> String {
        self.base.replacen("http", "ws", 1)
    }

    pub fn stop(&self) {
        if let Ok(mut slot) = self.child.lock()
            && let Some(mut child) = slot.take()
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Sends the request to the original and passes its answer through. Each call is one fallback.
pub async fn proxy(State(original): State<Original>, req: Request) -> Response {
    let (parts, body) = req.into_parts();
    let target = parts
        .uri
        .path_and_query()
        .map_or("/", |p| p.as_str())
        .to_string();
    if let Some(mut file) = log() {
        let _ = writeln!(file, "{} {target}", parts.method);
    }
    let Ok(body) = axum::body::to_bytes(body, usize::MAX).await else {
        return (
            StatusCode::BAD_REQUEST,
            "front door: unreadable request body",
        )
            .into_response();
    };
    let mut request = original
        .client
        .request(parts.method, format!("{}{target}", original.base))
        .body(body);
    for (name, value) in &parts.headers {
        // Keep the client-facing Host: upstream derives requestOrigin from it
        // for the CSRF origin gate, which must match the browser's Origin.
        request = request.header(name, value);
    }
    match request.send().await {
        Ok(answer) => {
            let mut response = Response::builder().status(answer.status());
            for (name, value) in answer.headers() {
                if name != header::TRANSFER_ENCODING && name != header::CONNECTION {
                    response = response.header(name, value);
                }
            }
            let body = answer.bytes().await.unwrap_or_default();
            response
                .body(Body::from(body))
                .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("front door: the original did not answer: {e}"),
        )
            .into_response(),
    }
}

/// Ctrl-C or SIGTERM: stop taking requests and finish the open ones.
pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("a SIGTERM handler");
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = term.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
