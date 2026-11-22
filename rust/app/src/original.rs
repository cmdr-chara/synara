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
    let tap_client = std::env::var_os("SYNARA_TAP_CLIENT").is_some();
    for (name, value) in &parts.headers {
        if tap_client
            && (name == header::IF_NONE_MATCH
                || name.as_str() == "if-modified-since"
                || name == header::ACCEPT_ENCODING)
        {
            continue; // rewritten bodies must not be served from the browser's cache
        }
        // Keep the client-facing Host: upstream derives requestOrigin from it
        // for the CSRF origin gate, which must match the browser's Origin.
        request = request.header(name, value);
    }
    match request.send().await {
        Ok(answer) => {
            let status = answer.status();
            let content_type = answer_content_type(answer.headers());
            let mut headers = axum::http::HeaderMap::new();
            for (name, value) in answer.headers() {
                if name == header::TRANSFER_ENCODING || name == header::CONNECTION {
                    continue;
                }
                if name == header::CONTENT_LENGTH || name == header::ETAG || name.as_str() == "last-modified" {
                    continue; // body may be rewritten; stale validators/length must not leak
                }
                headers.insert(name, value.clone());
            }
            let body = maybe_inject_client_tap(content_type, answer.bytes().await.unwrap_or_default());
            let mut out = Response::builder()
                .status(status)
                .body(Body::from(body))
                .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response());
            out.headers_mut().clone_from(&headers);
            out
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            format!("front door: the original did not answer: {e}"),
        )
            .into_response(),
    }
}

fn answer_content_type(headers: &axum::http::HeaderMap) -> String {
    headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string()
}

/// Debug aid (SYNARA_TAP_CLIENT=1): wraps window.WebSocket and console.warn/
/// console.error in every HTML page we serve, posting frames and failures to
/// POST /__tap_client. Lets us see exactly what the client receives before it
/// defects.
const CLIENT_TAP_SCRIPT: &str = r##"<script>
(() => {
  const send = (line) => { try { navigator.sendBeacon('/__tap_client', line); } catch(e){} };
  const OW = window.WebSocket;
  let n = 0;
  window.WebSocket = class extends OW {
    constructor(url, p) {
      super(url, p);
      this.__tag = 'ws' + (++n);
      this.binaryType = 'arraybuffer';
      send('OPEN ' + this.__tag + ' ' + url);
      this.addEventListener('open', () => send('open ' + this.__tag));
      this.addEventListener('error', () => send('error ' + this.__tag));
      this.addEventListener('close', (e) => send('close ' + this.__tag + ' ' + e.code + ' ' + (e.reason||'')));
      this.addEventListener('message', (e) => {
        let d = e.data;
        if (d instanceof ArrayBuffer) {
          const u = new Uint8Array(d);
          d = '[bin ' + u.length + 'B] ' + Array.from(u.slice(0, 80)).map(b => b.toString(16).padStart(2, '0')).join('');
        } else if (d instanceof Blob) {
          d = '[blob ' + d.size + 'B]';
        } else {
          d = String(d).slice(0, 400);
        }
        send('< ' + this.__tag + ' ' + d);
      });
    }
    send(d) {
      let s;
      if (typeof d === 'string') s = d.slice(0, 400);
      else s = '[bin ' + (d.byteLength || d.size || 0) + 'B]';
      send('> ' + this.__tag + ' ' + s);
      return super.send(d);
    }
  };
  for (const k of ['warn', 'error']) {
    const orig = console[k].bind(console);
    console[k] = (...a) => {
      try {
        send(k + ' ' + a.map(x => { try { return typeof x === 'object' && x !== null ? JSON.stringify(x) : String(x); } catch (e) { return 'unprintable'; } }).join(' '));
      } catch (e) {}
      return orig(...a);
    };
  }
  window.addEventListener('unhandledrejection', (e) => send('unhandledrejection ' + String(e.reason && (e.reason.stack || e.reason.message || e.reason)).slice(0, 400)));
  window.addEventListener('error', (e) => send('windowerror ' + String(e.message).slice(0, 300)));
})();
</script>"##;

fn maybe_inject_client_tap(content_type: String, body: axum::body::Bytes) -> axum::body::Bytes {
    if std::env::var_os("SYNARA_TAP_CLIENT").is_none() || !content_type.contains("text/html") {
        return body;
    }
    let Ok(mut html) = String::from_utf8(body.to_vec()) else {
        return body;
    };
    if let Some(pos) = html.find("<script") {
        html.insert_str(pos, CLIENT_TAP_SCRIPT);
    } else {
        html = format!("{CLIENT_TAP_SCRIPT}{html}");
    }
    axum::body::Bytes::from(html)
}

/// POST /__tap_client: client-side tap lines appended by the injected script.
pub async fn tap_client(body: Body) -> StatusCode {
    let Ok(bytes) = axum::body::to_bytes(body, 64 * 1024).await else {
        return StatusCode::BAD_REQUEST;
    };
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/ws_client_tap.log")
    {
        let _ = f.write_all(&bytes);
        let _ = f.write_all(b"\n");
    }
    StatusCode::NO_CONTENT
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
