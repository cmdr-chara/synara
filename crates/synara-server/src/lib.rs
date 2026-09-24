mod execution;
mod interactions;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};
use synara_acp::AcpBackend;
use synara_core::{Project, Task, TaskScope, Thread, Workspace, WorkspaceLocation};
use synara_runtime::WorkspaceOwnerLock;
use synara_workspace::{AgentProfile, Controller, WorkspaceService};
use tokio::sync::{RwLock, Semaphore, watch};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::{JoinHandle, JoinSet},
    time::timeout,
};
use tokio_util::sync::CancellationToken;

const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_BODY_BYTES: usize = 64 * 1024;
const MAX_WEB_DRAFT_BYTES: usize = 16 * 1024;
const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_CATALOG_ROWS: usize = 32;
const MAX_CATALOG_TEXT_CHARS: usize = 128;
const MAX_THREAD_MESSAGES: usize = 24;
const MAX_THREAD_MESSAGE_CHARS: usize = 1800;
const MAX_CONNECTIONS: usize = 128;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Lifecycle {
    Starting,
    Ready,
    Stopping,
    Failed,
}

/// Configuration for one local server process. `token` is intentionally omitted from Debug.
pub struct ServerConfig {
    pub bind: IpAddr,
    pub port: u16,
    pub database_path: PathBuf,
    private_database_directory: bool,
    token: String,
}

impl ServerConfig {
    pub fn new(bind: IpAddr, port: u16, database_path: PathBuf, token: String) -> Self {
        Self {
            bind,
            port,
            database_path,
            private_database_directory: false,
            token,
        }
    }

    /// Mark the parent directory as server-managed private storage. Explicit paths stay untouched.
    pub fn with_private_database_directory(mut self) -> Self {
        self.private_database_directory = true;
        self
    }
}

struct RuntimeServices {
    workspace: WorkspaceService,
    controller: Arc<Controller>,
    _owner_lock: Option<WorkspaceOwnerLock>,
}

struct AppState {
    lifecycle: StdMutex<Lifecycle>,
    lifecycle_tx: watch::Sender<Lifecycle>,
    token_digest: [u8; 32],
    runtime: RwLock<Option<RuntimeServices>>,
    execution: execution::ExecutionOwner,
    interactions: interactions::Owner,
}

impl AppState {
    fn new(token: &str) -> Result<Self> {
        validate_token(token)?;
        let digest: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        let (lifecycle_tx, _) = watch::channel(Lifecycle::Starting);
        Ok(Self {
            lifecycle: StdMutex::new(Lifecycle::Starting),
            lifecycle_tx,
            token_digest: digest,
            runtime: RwLock::new(None),
            execution: execution::ExecutionOwner::default(),
            interactions: interactions::Owner::default(),
        })
    }

    fn state(&self) -> Lifecycle {
        *self
            .lifecycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn transition(&self, next: Lifecycle) {
        let mut current = self
            .lifecycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *current == Lifecycle::Stopping && next != Lifecycle::Stopping {
            return;
        }
        *current = next;
        self.lifecycle_tx.send_replace(next);
    }

    fn subscribe(&self) -> watch::Receiver<Lifecycle> {
        self.lifecycle_tx.subscribe()
    }

    fn authorized(&self, authorization: Option<&str>) -> bool {
        let Some(value) = authorization else {
            return false;
        };
        let Some(token) = value.strip_prefix("Bearer ") else {
            return false;
        };
        let candidate: [u8; 32] = Sha256::digest(token.as_bytes()).into();
        constant_time_eq(&self.token_digest, &candidate)
    }

    async fn install_runtime(
        &self,
        workspace: WorkspaceService,
        owner_lock: Option<WorkspaceOwnerLock>,
    ) {
        let mut runtime = self.runtime.write().await;
        let mut lifecycle = self
            .lifecycle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if *lifecycle != Lifecycle::Starting {
            return;
        }
        let controller = Arc::new(Controller::new(
            workspace.clone(),
            Arc::new(AcpBackend::default()),
            self.interactions.broker.clone(),
        ));
        *runtime = Some(RuntimeServices {
            workspace,
            controller,
            _owner_lock: owner_lock,
        });
        *lifecycle = Lifecycle::Ready;
        self.lifecycle_tx.send_replace(Lifecycle::Ready);
    }
}

/// A running listener. Startup and recovery happen after bind, so readiness stays 503 meanwhile.
pub struct RunningServer {
    address: SocketAddr,
    state: Arc<AppState>,
    stop: CancellationToken,
    listener_task: Option<JoinHandle<Result<()>>>,
    startup_task: Option<JoinHandle<Result<()>>>,
    lifecycle: watch::Receiver<Lifecycle>,
}

impl RunningServer {
    pub async fn start(config: ServerConfig) -> Result<Self> {
        validate_bind_address(config.bind)?;
        validate_token(&config.token)?;

        let listener = TcpListener::bind(SocketAddr::new(config.bind, config.port))
            .await
            .context("could not bind the local Synara server")?;
        let address = listener
            .local_addr()
            .context("could not read bound address")?;
        let state = Arc::new(AppState::new(&config.token)?);
        let lifecycle = state.subscribe();
        let stop = CancellationToken::new();
        let listener_task = tokio::spawn(serve(listener, state.clone(), stop.clone()));

        let startup_state = state.clone();
        let startup_task = tokio::spawn(async move {
            let result = initialize(config.database_path, config.private_database_directory).await;
            match result {
                Ok((workspace, owner_lock)) => {
                    startup_state
                        .install_runtime(workspace, Some(owner_lock))
                        .await;
                    Ok(())
                }
                Err(error) => {
                    tracing::error!(error = %error, "Synara server startup failed");
                    startup_state.transition(Lifecycle::Failed);
                    Err(error)
                }
            }
        });

        Ok(Self {
            address,
            state,
            stop,
            listener_task: Some(listener_task),
            startup_task: Some(startup_task),
            lifecycle,
        })
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub async fn wait_ready(&mut self) -> Result<()> {
        loop {
            match *self.lifecycle.borrow() {
                Lifecycle::Ready => return Ok(()),
                Lifecycle::Failed => bail!("workspace startup failed; see the server log"),
                Lifecycle::Stopping => bail!("server stopped before becoming ready"),
                Lifecycle::Starting => {}
            }
            self.lifecycle
                .changed()
                .await
                .context("server lifecycle channel closed")?;
        }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.state.transition(Lifecycle::Stopping);
        self.stop.cancel();
        let listener_result = match self.listener_task.take() {
            Some(listener) => match listener.await {
                Ok(result) => result,
                Err(error) => Err(error.into()),
            },
            None => Ok(()),
        };
        if let Some(startup) = self.startup_task.take() {
            // Startup owns the database lock across open/recovery. Let it finish so no
            // spawn_blocking database operation can outlive that lock during shutdown.
            let _ = startup.await;
        }
        listener_result
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.state.transition(Lifecycle::Stopping);
        self.stop.cancel();
    }
}

async fn initialize(
    database_path: PathBuf,
    private_database_directory: bool,
) -> Result<(WorkspaceService, WorkspaceOwnerLock)> {
    prepare_database_parent(&database_path, private_database_directory).await?;
    let owner_lock = WorkspaceOwnerLock::acquire(&database_path)
        .context("workspace database is already owned by another process")?;
    let workspace = WorkspaceService::open(database_path)
        .await
        .context("could not open the workspace database")?;
    workspace
        .recover_interrupted()
        .await
        .context("could not recover interrupted workspace tasks")?;
    Ok((workspace, owner_lock))
}

async fn prepare_database_parent(database_path: &Path, private: bool) -> Result<()> {
    if let Some(parent) = database_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        tokio::fs::create_dir_all(parent)
            .await
            .context("could not create the database directory")?;
        if private {
            let metadata = tokio::fs::symlink_metadata(parent)
                .await
                .context("could not inspect the private database directory")?;
            ensure!(
                !metadata.file_type().is_symlink() && metadata.is_dir(),
                "private database directory must be a real directory, not a symlink"
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                tokio::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))
                    .await
                    .context("could not restrict database directory permissions")?;
            }
        }
    }
    Ok(())
}

fn validate_token(token: &str) -> Result<()> {
    ensure!(
        (32..=4096).contains(&token.len())
            && token.is_ascii()
            && token
                .bytes()
                .all(|byte| !byte.is_ascii_whitespace() && !byte.is_ascii_control()),
        "bearer token must contain 32 to 4096 printable ASCII bytes"
    );
    Ok(())
}

/// This server deliberately has no remote-access mode. Every bind must be an IP loopback address.
pub fn validate_bind_address(address: IpAddr) -> Result<()> {
    ensure!(
        address.is_loopback(),
        "Synara server only accepts loopback bind addresses"
    );
    Ok(())
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    left.iter()
        .zip(right)
        .fold(0u8, |difference, (a, b)| difference | (a ^ b))
        == 0
}

#[derive(Debug)]
struct Request {
    method: String,
    target: String,
    version: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    fn header(&self, name: &str) -> Option<&str> {
        let mut values = self
            .headers
            .iter()
            .filter(|(header, _)| header.eq_ignore_ascii_case(name));
        let first = values.next()?.1.as_str();
        if values.next().is_some() {
            None
        } else {
            Some(first)
        }
    }

    fn has_duplicate(&self, name: &str) -> bool {
        self.headers
            .iter()
            .filter(|(header, _)| header.eq_ignore_ascii_case(name))
            .count()
            > 1
    }
}

#[derive(Clone, Debug)]
struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
    extra_headers: Vec<(&'static str, &'static str)>,
}

impl Response {
    fn json(status: u16, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: body.into(),
            extra_headers: vec![],
        }
    }

    fn text(status: u16, content_type: &'static str, body: &'static str) -> Self {
        Self {
            status,
            content_type,
            body: body.as_bytes().to_vec(),
            extra_headers: vec![],
        }
    }

    fn with_header(mut self, name: &'static str, value: &'static str) -> Self {
        self.extra_headers.push((name, value));
        self
    }

    fn status_text(&self) -> &'static str {
        match self.status {
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            409 => "Conflict",
            429 => "Too Many Requests",
            400 => "Bad Request",
            401 => "Unauthorized",
            404 => "Not Found",
            405 => "Method Not Allowed",
            408 => "Request Timeout",
            413 => "Payload Too Large",
            415 => "Unsupported Media Type",
            421 => "Misdirected Request",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Error",
        }
    }

    async fn write_to(&self, stream: &mut TcpStream) -> Result<()> {
        let mut headers = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'\r\n",
            self.status,
            self.status_text(),
            self.content_type,
            self.body.len()
        );
        for (name, value) in &self.extra_headers {
            headers.push_str(name);
            headers.push_str(": ");
            headers.push_str(value);
            headers.push_str("\r\n");
        }
        headers.push_str("\r\n");
        stream.write_all(headers.as_bytes()).await?;
        stream.write_all(&self.body).await?;
        stream.flush().await?;
        Ok(())
    }
}

async fn serve(listener: TcpListener, state: Arc<AppState>, stop: CancellationToken) -> Result<()> {
    let permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let mut connections = JoinSet::new();
    let listener_result: Result<()> = loop {
        tokio::select! {
            _ = stop.cancelled() => break Ok(()),
            Some(_) = connections.join_next(), if !connections.is_empty() => {},
            accepted = listener.accept() => {
                match accepted {
                    Ok((stream, peer)) => {
                        if !peer.ip().is_loopback() {
                            continue;
                        }
                        let Ok(permit) = permits.clone().try_acquire_owned() else {
                            continue;
                        };
                        let state = state.clone();
                        connections.spawn(async move {
                            let _permit = permit;
                            let _ = handle_connection(stream, state).await;
                        });
                    }
                    Err(error) => {
                        state.transition(Lifecycle::Failed);
                        tracing::error!(error = %error, "Synara HTTP listener stopped unexpectedly");
                        break Err(error.into());
                    }
                }
            }
        }
    };
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    state.interactions.close().await;
    let controller_result = if let Some(runtime) = state.runtime.write().await.take() {
        state.execution.shutdown(&runtime.controller).await
    } else {
        Ok(())
    };
    listener_result?;
    controller_result
}

async fn handle_connection(mut stream: TcpStream, state: Arc<AppState>) -> Result<()> {
    // Bound untrusted socket reads independently. Once an authenticated write
    // starts, cancelling its future can leave a committed task with no response.
    let request = match timeout(REQUEST_TIMEOUT, read_request(&mut stream)).await {
        Err(_) => {
            return Response::json(408, br#"{"error":"request_timeout"}"#.to_vec())
                .write_to(&mut stream)
                .await;
        }
        Ok(result) => match result {
            Ok(request) => request,
            Err(ReadRequestError::TooLarge) => {
                return Response::json(413, br#"{"error":"request_too_large"}"#.to_vec())
                    .write_to(&mut stream)
                    .await;
            }
            Err(ReadRequestError::Invalid) => {
                return Response::json(400, br#"{"error":"bad_request"}"#.to_vec())
                    .write_to(&mut stream)
                    .await;
            }
            Err(ReadRequestError::Io(error)) => return Err(error.into()),
        },
    };
    let response = dispatch(&request, &state, stream.local_addr()?.port()).await;
    timeout(REQUEST_TIMEOUT, response.write_to(&mut stream)).await??;
    Ok(())
}

enum ReadRequestError {
    TooLarge,
    Invalid,
    Io(std::io::Error),
}

async fn read_request(stream: &mut TcpStream) -> std::result::Result<Request, ReadRequestError> {
    let mut bytes = Vec::with_capacity(2048);
    let mut buffer = [0u8; 1024];
    loop {
        if bytes.len() >= MAX_HEADER_BYTES {
            return Err(ReadRequestError::TooLarge);
        }
        let read_size = (MAX_HEADER_BYTES - bytes.len()).min(buffer.len());
        let read = stream
            .read(&mut buffer[..read_size])
            .await
            .map_err(ReadRequestError::Io)?;
        if read == 0 {
            return Err(ReadRequestError::Invalid);
        }
        bytes.extend_from_slice(&buffer[..read]);
        if let Some(end) = find_header_end(&bytes) {
            if end > MAX_HEADER_BYTES {
                return Err(ReadRequestError::TooLarge);
            }
            let mut request = parse_request(&bytes[..end]).ok_or(ReadRequestError::Invalid)?;
            if request.has_duplicate("content-length")
                || request
                    .headers
                    .iter()
                    .any(|(name, _)| name == "transfer-encoding")
            {
                return Err(ReadRequestError::Invalid);
            }
            let body_len = request
                .header("content-length")
                .map(str::parse::<usize>)
                .transpose()
                .map_err(|_| ReadRequestError::Invalid)?
                .unwrap_or(0);
            if body_len > MAX_BODY_BYTES {
                return Err(ReadRequestError::TooLarge);
            }
            let expected = end + body_len;
            if bytes.len() > expected {
                return Err(ReadRequestError::Invalid);
            }
            while bytes.len() < expected {
                let read_size = (expected - bytes.len()).min(buffer.len());
                let read = stream
                    .read(&mut buffer[..read_size])
                    .await
                    .map_err(ReadRequestError::Io)?;
                if read == 0 {
                    return Err(ReadRequestError::Invalid);
                }
                bytes.extend_from_slice(&buffer[..read]);
            }
            request.body = bytes[end..].to_vec();
            return Ok(request);
        }
    }
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
}

fn parse_request(bytes: &[u8]) -> Option<Request> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut lines = text.strip_suffix("\r\n\r\n")?.split("\r\n");
    let mut request_line = lines.next()?.split_ascii_whitespace();
    let method = request_line.next()?.to_owned();
    let target = request_line.next()?.to_owned();
    let version = request_line.next()?.to_owned();
    if request_line.next().is_some() || !matches!(version.as_str(), "HTTP/1.0" | "HTTP/1.1") {
        return None;
    }
    let mut headers = Vec::new();
    for line in lines {
        let (name, value) = line.split_once(':')?;
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte))
        {
            return None;
        }
        let value = value.trim_matches([' ', '\t']);
        if value
            .bytes()
            .any(|byte| byte.is_ascii_control() && byte != b'\t')
        {
            return None;
        }
        headers.push((name.to_ascii_lowercase(), value.to_owned()));
    }
    Some(Request {
        method,
        target,
        version,
        headers,
        body: Vec::new(),
    })
}

async fn dispatch(request: &Request, state: &AppState, local_port: u16) -> Response {
    if request.has_duplicate("host")
        || request.has_duplicate("origin")
        || request.has_duplicate("authorization")
        || request.has_duplicate("content-length")
    {
        return bad_request();
    }
    let Some(host) = request.header("host").and_then(parse_authority) else {
        return Response::json(421, br#"{"error":"invalid_host"}"#.to_vec());
    };
    if host.port != local_port || !is_loopback_name(&host.host) {
        return Response::json(421, br#"{"error":"invalid_host"}"#.to_vec());
    }
    if let Some(origin) = request.header("origin") {
        let Some(origin) = parse_origin(origin) else {
            return Response::json(421, br#"{"error":"invalid_origin"}"#.to_vec());
        };
        if origin.port != host.port || origin.host != host.host {
            return Response::json(421, br#"{"error":"invalid_origin"}"#.to_vec());
        }
    }
    if request
        .headers
        .iter()
        .any(|(name, _)| name == "transfer-encoding")
    {
        return bad_request();
    }
    if request.has_duplicate("content-length")
        || request
            .header("content-length")
            .is_some_and(|length| length.parse::<usize>().ok() != Some(request.body.len()))
        || (request.header("content-length").is_none() && !request.body.is_empty())
    {
        return bad_request();
    }
    if !matches!(request.version.as_str(), "HTTP/1.0" | "HTTP/1.1") {
        return bad_request();
    }
    if request.method != "GET" && request.method != "POST" {
        return Response::json(405, br#"{"error":"method_not_allowed"}"#.to_vec())
            .with_header("Allow", "GET, POST");
    }
    let path = request
        .target
        .split_once('?')
        .map_or(request.target.as_str(), |(path, _)| path);
    if request.method == "POST" {
        return dispatch_post(path, request, state).await;
    }
    if !request.body.is_empty() {
        return bad_request();
    }
    match path {
        "/health" | "/ready" => health_response(state.state()),
        "/" => Response::text(200, "text/html; charset=utf-8", INDEX_HTML),
        "/app.js" => Response::text(200, "text/javascript; charset=utf-8", APP_JS),
        "/style.css" => Response::text(200, "text/css; charset=utf-8", STYLE_CSS),
        "/api/catalog" => {
            if !state.authorized(request.header("authorization")) {
                return unauthorized();
            }
            if state.state() != Lifecycle::Ready {
                return health_response(state.state());
            }
            let runtime = state.runtime.read().await;
            let Some(runtime) = runtime.as_ref() else {
                return health_response(Lifecycle::Starting);
            };
            match runtime.workspace.catalog().await {
                Ok(catalog) => {
                    match catalog_view(catalog.workspaces, catalog.projects, catalog.tasks) {
                        Ok(body) => Response::json(200, body),
                        Err(_) => {
                            Response::json(500, br#"{"error":"catalog_unavailable"}"#.to_vec())
                        }
                    }
                }
                Err(_) => Response::json(503, br#"{"error":"catalog_unavailable"}"#.to_vec()),
            }
        }
        "/api/profiles" => {
            if !state.authorized(request.header("authorization")) {
                return unauthorized();
            }
            if state.state() != Lifecycle::Ready {
                return health_response(state.state());
            }
            let runtime = state.runtime.read().await;
            let Some(runtime) = runtime.as_ref() else {
                return health_response(Lifecycle::Starting);
            };
            match runtime.workspace.profiles().await {
                Ok(profiles) => match profiles_view(profiles) {
                    Ok(body) => Response::json(200, body),
                    Err(_) => Response::json(500, br#"{"error":"profiles_unavailable"}"#.to_vec()),
                },
                Err(_) => Response::json(503, br#"{"error":"profiles_unavailable"}"#.to_vec()),
            }
        }
        _ if path.starts_with("/api/tasks/")
            && (path.ends_with("/run") || path.ends_with("/interactions")) =>
        {
            if !state.authorized(request.header("authorization")) {
                return unauthorized();
            }
            if state.state() != Lifecycle::Ready {
                return health_response(state.state());
            }
            if request.target != path {
                return bad_request();
            }
            if path.ends_with("/interactions") {
                interactions::get(path, state).await
            } else {
                execution::dispatch_get(path, state).await
            }
        }
        _ if path.starts_with("/api/tasks/") && path.ends_with("/draft") => {
            if !state.authorized(request.header("authorization")) {
                return unauthorized();
            }
            if state.state() != Lifecycle::Ready {
                return health_response(state.state());
            }
            if request.target != path {
                return bad_request();
            }
            let Some(task_id) = path
                .strip_prefix("/api/tasks/")
                .and_then(|value| value.strip_suffix("/draft"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            let runtime = state.runtime.read().await;
            let Some(runtime) = runtime.as_ref() else {
                return health_response(Lifecycle::Starting);
            };
            let task = match runtime.workspace.catalog().await {
                Ok(catalog) => catalog
                    .tasks
                    .into_iter()
                    .find(|task| task.id.to_string() == task_id),
                Err(_) => {
                    return Response::json(503, br#"{"error":"draft_unavailable"}"#.to_vec());
                }
            };
            let Some(task) = task else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            match runtime.workspace.task_draft(task.id).await {
                Ok(draft) => match draft_view(&draft) {
                    Ok(body) => Response::json(200, body),
                    Err(_) => Response::json(500, br#"{"error":"draft_unavailable"}"#.to_vec()),
                },
                Err(_) => Response::json(503, br#"{"error":"draft_unavailable"}"#.to_vec()),
            }
        }
        _ if path.starts_with("/api/tasks/") && path.ends_with("/thread") => {
            if !state.authorized(request.header("authorization")) {
                return unauthorized();
            }
            if state.state() != Lifecycle::Ready {
                return health_response(state.state());
            }
            let Some(task_id) = path
                .strip_prefix("/api/tasks/")
                .and_then(|value| value.strip_suffix("/thread"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            let before = match request.target.split_once('?') {
                None => None,
                Some((_, query)) => {
                    let Some(value) = query.strip_prefix("before=") else {
                        return bad_request();
                    };
                    let Ok(index) = value.parse::<usize>() else {
                        return bad_request();
                    };
                    Some(index)
                }
            };
            let runtime = state.runtime.read().await;
            let Some(runtime) = runtime.as_ref() else {
                return health_response(Lifecycle::Starting);
            };
            let task = match runtime.workspace.catalog().await {
                Ok(catalog) => catalog
                    .tasks
                    .into_iter()
                    .find(|task| task.id.to_string() == task_id),
                Err(_) => {
                    return Response::json(503, br#"{"error":"thread_unavailable"}"#.to_vec());
                }
            };
            let Some(task) = task else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            match runtime.workspace.thread(task.thread_id).await {
                Ok(thread) => match thread_view(&task, &thread, before) {
                    Ok(body) => Response::json(200, body),
                    Err(_) => Response::json(500, br#"{"error":"thread_unavailable"}"#.to_vec()),
                },
                Err(_) => Response::json(503, br#"{"error":"thread_unavailable"}"#.to_vec()),
            }
        }
        _ => Response::json(404, br#"{"error":"not_found"}"#.to_vec()),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewWorkspaceRequest {
    root: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NewTaskRequest {
    title: String,
    agent_id: String,
    #[serde(default)]
    draft: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SaveDraftRequest {
    text: String,
}

async fn dispatch_post(path: &str, request: &Request, state: &AppState) -> Response {
    if !state.authorized(request.header("authorization")) {
        return unauthorized();
    }
    if state.state() != Lifecycle::Ready {
        return health_response(state.state());
    }
    if request.target != path || request.body.is_empty() || request.has_duplicate("content-type") {
        return bad_request();
    }
    if request.header("content-type") != Some("application/json") {
        return Response::json(415, br#"{"error":"json_required"}"#.to_vec());
    }
    let runtime = state.runtime.read().await;
    let Some(runtime) = runtime.as_ref() else {
        return health_response(Lifecycle::Starting);
    };
    match path {
        _ if path.starts_with("/api/tasks/") && path.ends_with("/interactions") => {
            interactions::post(path, request, state, runtime).await
        }
        _ if path.starts_with("/api/tasks/")
            && (path.ends_with("/run") || path.ends_with("/stop")) =>
        {
            execution::dispatch_post(path, request, state, runtime).await
        }

        "/api/workspaces" => {
            let Ok(payload) = serde_json::from_slice::<NewWorkspaceRequest>(&request.body) else {
                return bad_request();
            };
            let root = PathBuf::from(payload.root);
            if !root.is_absolute() || root.as_os_str().len() > 4096 {
                return bad_request();
            }
            match runtime.workspace.add_local_workspace(root).await {
                Ok(project) => match serde_json::to_vec(&project_view(project)) {
                    Ok(body) => Response::json(201, body),
                    Err(_) => Response::json(500, br#"{"error":"workspace_unavailable"}"#.to_vec()),
                },
                Err(_) => Response::json(400, br#"{"error":"workspace_rejected"}"#.to_vec()),
            }
        }
        _ if path.starts_with("/api/tasks/") && path.ends_with("/draft") => {
            let Some(task_id) = path
                .strip_prefix("/api/tasks/")
                .and_then(|value| value.strip_suffix("/draft"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            let Ok(payload) = serde_json::from_slice::<SaveDraftRequest>(&request.body) else {
                return bad_request();
            };
            if payload.text.len() > MAX_WEB_DRAFT_BYTES {
                return bad_request();
            }
            let task = match runtime.workspace.catalog().await {
                Ok(catalog) => catalog
                    .tasks
                    .into_iter()
                    .find(|task| task.id.to_string() == task_id),
                Err(_) => {
                    return Response::json(503, br#"{"error":"catalog_unavailable"}"#.to_vec());
                }
            };
            let Some(task) = task else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            let _mutation = state.execution.mutation.lock().await;
            match runtime
                .workspace
                .save_task_draft(task.id, payload.text.clone())
                .await
            {
                Ok(()) => match draft_view(&payload.text) {
                    Ok(body) => Response::json(200, body),
                    Err(_) => Response::json(500, br#"{"error":"draft_unavailable"}"#.to_vec()),
                },
                Err(_) => Response::json(400, br#"{"error":"draft_rejected"}"#.to_vec()),
            }
        }
        _ if path.starts_with("/api/projects/") && path.ends_with("/tasks") => {
            let Some(project_id) = path
                .strip_prefix("/api/projects/")
                .and_then(|value| value.strip_suffix("/tasks"))
                .filter(|value| !value.is_empty() && !value.contains('/'))
            else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            let Ok(payload) = serde_json::from_slice::<NewTaskRequest>(&request.body) else {
                return bad_request();
            };
            if payload.draft.len() > MAX_WEB_DRAFT_BYTES {
                return bad_request();
            }
            let project = match runtime.workspace.catalog().await {
                Ok(catalog) => catalog
                    .projects
                    .into_iter()
                    .find(|project| project.id.to_string() == project_id),
                Err(_) => {
                    return Response::json(503, br#"{"error":"catalog_unavailable"}"#.to_vec());
                }
            };
            let Some(project) = project else {
                return Response::json(404, br#"{"error":"not_found"}"#.to_vec());
            };
            match runtime
                .workspace
                .create_scoped_task_with_draft(
                    project.id,
                    payload.title,
                    payload.agent_id,
                    TaskScope::Project,
                    payload.draft,
                )
                .await
            {
                Ok(task) => match serde_json::to_vec(&task_view(task)) {
                    Ok(body) => Response::json(201, body),
                    Err(_) => Response::json(500, br#"{"error":"task_unavailable"}"#.to_vec()),
                },
                Err(_) => Response::json(400, br#"{"error":"task_rejected"}"#.to_vec()),
            }
        }
        _ => Response::json(404, br#"{"error":"not_found"}"#.to_vec()),
    }
}

fn bad_request() -> Response {
    Response::json(400, br#"{"error":"bad_request"}"#.to_vec())
}

fn unauthorized() -> Response {
    Response::json(401, br#"{"error":"unauthorized"}"#.to_vec())
        .with_header("WWW-Authenticate", "Bearer realm=\"Synara\"")
}

fn health_response(lifecycle: Lifecycle) -> Response {
    let (status, body) = match lifecycle {
        Lifecycle::Ready => (200, br#"{"status":"ok"}"#.as_slice()),
        Lifecycle::Starting => (503, br#"{"status":"starting"}"#.as_slice()),
        Lifecycle::Stopping => (503, br#"{"status":"stopping"}"#.as_slice()),
        Lifecycle::Failed => (503, br#"{"status":"unavailable"}"#.as_slice()),
    };
    Response::json(status, body.to_vec())
}

#[derive(Debug)]
struct Authority {
    host: String,
    port: u16,
}

fn parse_authority(authority: &str) -> Option<Authority> {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let (host, suffix) = bracketed.split_once(']')?;
        let ip: IpAddr = host.parse().ok()?;
        let port = suffix.strip_prefix(':')?.parse().ok()?;
        return Some(Authority {
            host: ip.to_string(),
            port,
        });
    }
    let (host, port) = authority.rsplit_once(':')?;
    if host.contains(':') || host.is_empty() {
        return None;
    }
    Some(Authority {
        host: host.to_ascii_lowercase(),
        port: port.parse().ok()?,
    })
}

fn parse_origin(origin: &str) -> Option<Authority> {
    let authority = origin.strip_prefix("http://")?;
    if authority.contains('/') || authority.contains('?') || authority.contains('#') {
        return None;
    }
    let authority = parse_authority(authority)?;
    is_loopback_name(&authority.host).then_some(authority)
}

fn is_loopback_name(host: &str) -> bool {
    host == "localhost"
        || host
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

#[derive(Serialize)]
struct CatalogView {
    workspaces: Vec<WorkspaceView>,
    projects: Vec<ProjectView>,
    tasks: Vec<TaskView>,
    truncated: Truncated,
}

#[derive(Serialize)]
struct WorkspaceView {
    id: String,
    name: String,
    kind: &'static str,
}

#[derive(Serialize)]
struct ProjectView {
    id: String,
    workspace_id: String,
    name: String,
}

fn project_view(project: Project) -> ProjectView {
    ProjectView {
        id: project.id.to_string(),
        workspace_id: project.workspace_id.to_string(),
        name: bounded_text(&project.name),
    }
}

#[derive(Serialize)]
struct TaskView {
    id: String,
    project_id: String,
    title: String,
    state: synara_core::TaskState,
    agent_id: String,
    updated_at_ms: i64,
}

fn task_view(task: Task) -> TaskView {
    TaskView {
        id: task.id.to_string(),
        project_id: task.project_id.to_string(),
        title: bounded_text(&task.title),
        state: task.state,
        agent_id: bounded_text(&task.agent_id),
        updated_at_ms: task.updated_at_ms,
    }
}

#[derive(Serialize)]
struct ProfilesView {
    profiles: Vec<ProfileView>,
    truncated: bool,
}

#[derive(Serialize)]
struct ProfileView {
    id: String,
    name: String,
}

#[derive(Serialize)]
struct DraftView {
    text: String,
    truncated: bool,
}

fn draft_view(draft: &str) -> Result<Vec<u8>> {
    let mut view = DraftView {
        text: draft.chars().take(16_000).collect(),
        truncated: draft.chars().count() > 16_000,
    };
    loop {
        let json = serde_json::to_vec(&view)?;
        if json.len() <= MAX_RESPONSE_BYTES {
            return Ok(json);
        }
        let reduced = view.text.chars().count() / 2;
        ensure!(reduced > 0, "draft response exceeded bound");
        view.text = view.text.chars().take(reduced).collect();
        view.truncated = true;
    }
}

fn profiles_view(profiles: Vec<AgentProfile>) -> Result<Vec<u8>> {
    let view = ProfilesView {
        truncated: profiles.len() > MAX_CATALOG_ROWS,
        profiles: profiles
            .into_iter()
            .take(MAX_CATALOG_ROWS)
            .map(|profile| ProfileView {
                id: bounded_text(&profile.id),
                name: bounded_text(&profile.name),
            })
            .collect(),
    };
    let json = serde_json::to_vec(&view)?;
    ensure!(
        json.len() <= MAX_RESPONSE_BYTES,
        "bounded profiles exceeded response limit"
    );
    Ok(json)
}

#[derive(Serialize)]
struct Truncated {
    workspaces: bool,
    projects: bool,
    tasks: bool,
}

#[derive(Serialize)]
struct ThreadView {
    task_id: String,
    title: String,
    state: synara_core::TaskState,
    messages: Vec<ThreadMessageView>,
    truncated: bool,
    next_before: Option<usize>,
}

#[derive(Serialize)]
struct ThreadMessageView {
    id: String,
    role: synara_core::Role,
    text: String,
    timestamp_ms: Option<i64>,
    truncated: bool,
}

fn thread_view(task: &Task, thread: &Thread, before: Option<usize>) -> Result<Vec<u8>> {
    let visible_messages: Vec<_> = thread
        .messages
        .iter()
        .filter(|message| message.role != synara_core::Role::Reasoning)
        .collect();
    let end = before
        .unwrap_or(visible_messages.len())
        .min(visible_messages.len());
    let mut start = end.saturating_sub(MAX_THREAD_MESSAGES);
    let messages = visible_messages[start..end]
        .iter()
        .map(|message| ThreadMessageView {
            id: bounded_text(&message.id),
            role: message.role,
            text: message
                .text
                .chars()
                .take(MAX_THREAD_MESSAGE_CHARS)
                .collect(),
            timestamp_ms: thread.message_timestamps.get(&message.id).copied(),
            truncated: message.text.chars().count() > MAX_THREAD_MESSAGE_CHARS,
        })
        .collect();
    let mut view = ThreadView {
        task_id: task.id.to_string(),
        title: bounded_text(&task.title),
        state: task.state,
        messages,
        truncated: start > 0,
        next_before: (start > 0).then_some(start),
    };
    loop {
        let json = serde_json::to_vec(&view)?;
        if json.len() <= MAX_RESPONSE_BYTES {
            return Ok(json);
        }
        ensure!(
            view.messages.len() > 1,
            "bounded thread exceeded response limit"
        );
        view.messages.remove(0);
        start += 1;
        view.truncated = true;
        view.next_before = Some(start);
    }
}

fn bounded_text(value: &str) -> String {
    value.chars().take(MAX_CATALOG_TEXT_CHARS).collect()
}

fn catalog_view(
    workspaces: Vec<Workspace>,
    projects: Vec<Project>,
    tasks: Vec<Task>,
) -> Result<Vec<u8>> {
    let view = CatalogView {
        truncated: Truncated {
            workspaces: workspaces.len() > MAX_CATALOG_ROWS,
            projects: projects.len() > MAX_CATALOG_ROWS,
            tasks: tasks.len() > MAX_CATALOG_ROWS,
        },
        workspaces: workspaces
            .into_iter()
            .take(MAX_CATALOG_ROWS)
            .map(|workspace| WorkspaceView {
                id: workspace.id.to_string(),
                name: bounded_text(&workspace.name),
                kind: match workspace.location {
                    WorkspaceLocation::Local { .. } => "local",
                    WorkspaceLocation::Ssh { .. } => "ssh",
                },
            })
            .collect(),
        projects: projects
            .into_iter()
            .take(MAX_CATALOG_ROWS)
            .map(project_view)
            .collect(),
        tasks: tasks
            .into_iter()
            .take(MAX_CATALOG_ROWS)
            .map(task_view)
            .collect(),
    };
    let json = serde_json::to_vec(&view)?;
    ensure!(
        json.len() <= MAX_RESPONSE_BYTES,
        "bounded catalog exceeded response limit"
    );
    Ok(json)
}

/// Load a token from exactly one source. File based credentials must not be group/world readable.
pub fn load_bearer_token(token_file: Option<&Path>) -> Result<String> {
    let environment = std::env::var("SYNARA_SERVER_TOKEN").ok();
    match (environment, token_file) {
        (Some(_), Some(_)) => {
            bail!("configure the bearer token using either SYNARA_SERVER_TOKEN or --token-file")
        }
        (None, None) => bail!("set SYNARA_SERVER_TOKEN or provide --token-file"),
        (Some(token), None) => {
            validate_token(&token)?;
            Ok(token)
        }
        (None, Some(path)) => {
            let mut options = std::fs::OpenOptions::new();
            options.read(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
            }
            let mut file = options
                .open(path)
                .with_context(|| format!("could not safely open token file {}", path.display()))?;
            let metadata = file
                .metadata()
                .context("could not inspect opened token file")?;
            ensure!(
                metadata.file_type().is_file(),
                "token path must be a regular file"
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                ensure!(
                    metadata.permissions().mode() & 0o077 == 0,
                    "token file permissions must exclude group and other access"
                );
            }
            ensure!(metadata.len() <= 4096, "token file is too large");
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            std::io::Read::take(&mut file, 4097)
                .read_to_end(&mut bytes)
                .context("could not read opened token file")?;
            ensure!(bytes.len() <= 4096, "token file is too large");
            let token = String::from_utf8(bytes).context("token file must contain UTF-8 text")?;
            let token = token.trim_end_matches(['\r', '\n']).to_owned();
            validate_token(&token)?;
            Ok(token)
        }
    }
}

pub fn default_database_path() -> Result<PathBuf> {
    if let Some(path) = std::env::var_os("SYNARA_SERVER_DB") {
        return Ok(PathBuf::from(path));
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(PathBuf::from)
                .map(|p| p.join("AppData/Local"))
        })
        .context("could not determine a local application data directory")?;
    #[cfg(not(target_os = "windows"))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".local/share"))
        })
        .context("could not determine a user data directory")?;
    Ok(headless_database_path(&base))
}

fn headless_database_path(base: &Path) -> PathBuf {
    base.join("synara").join("headless-server.sqlite3")
}

pub const DEFAULT_BIND: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);
pub const DEFAULT_PORT: u16 = 17341;

const INDEX_HTML: &str = include_str!("index.html");

const STYLE_CSS: &str = include_str!("style.css");

const APP_JS: &str = include_str!("app.js");

#[cfg(test)]
mod tests {
    use super::*;
    use std::{net::SocketAddr, time::Duration};
    use synara_core::WorkspaceLocation;

    const TOKEN: &str = "test-token-with-at-least-thirty-two-bytes";

    fn make_request(path: &str, port: u16, authorization: Option<&str>) -> Request {
        let mut headers = vec![("host".into(), format!("127.0.0.1:{port}"))];
        if let Some(authorization) = authorization {
            headers.push(("authorization".into(), authorization.into()));
        }
        Request {
            method: "GET".into(),
            target: path.into(),
            version: "HTTP/1.1".into(),
            headers,
            body: Vec::new(),
        }
    }

    async fn ready_state(token: &str) -> Arc<AppState> {
        let state = Arc::new(AppState::new(token).unwrap());
        let workspace = WorkspaceService::memory().unwrap();
        state.install_runtime(workspace, None).await;
        state
    }

    #[test]
    fn bind_policy_accepts_loopback_and_rejects_wildcard_or_remote_addresses() {
        assert!(validate_bind_address("127.0.0.1".parse().unwrap()).is_ok());
        assert!(validate_bind_address("::1".parse().unwrap()).is_ok());
        assert!(validate_bind_address("0.0.0.0".parse().unwrap()).is_err());
        assert!(validate_bind_address("::".parse().unwrap()).is_err());
        assert!(validate_bind_address("192.0.2.10".parse().unwrap()).is_err());
    }

    #[tokio::test]
    async fn catalog_requires_valid_bearer_token() {
        let state = ready_state(TOKEN).await;
        let port = 17341;
        for authorization in [None, Some("Bearer wrong-token-with-32-or-more-characters")] {
            let response = dispatch(
                &make_request("/api/catalog", port, authorization),
                &state,
                port,
            )
            .await;
            assert_eq!(response.status, 401);
        }
        let response = dispatch(
            &make_request("/api/catalog", port, Some(&format!("Bearer {TOKEN}"))),
            &state,
            port,
        )
        .await;
        assert_eq!(response.status, 200);
        let catalog: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(catalog["workspaces"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn authenticated_browser_creation_preserves_unsent_draft() {
        let directory = tempfile::tempdir().unwrap();
        let state = ready_state(TOKEN).await;
        let port = 17341;
        let post = |path: &str, body: serde_json::Value, authorization: Option<&str>| {
            let mut request = make_request(path, port, authorization);
            request.method = "POST".into();
            request.body = serde_json::to_vec(&body).unwrap();
            request
                .headers
                .push(("content-length".into(), request.body.len().to_string()));
            request
                .headers
                .push(("content-type".into(), "application/json".into()));
            request
        };
        let root = serde_json::json!({"root":directory.path().to_str().unwrap()});
        assert_eq!(
            dispatch(&post("/api/workspaces", root.clone(), None), &state, port)
                .await
                .status,
            401
        );
        let authorization = format!("Bearer {TOKEN}");
        let project_response = dispatch(
            &post("/api/workspaces", root, Some(&authorization)),
            &state,
            port,
        )
        .await;
        assert_eq!(project_response.status, 201);
        let project: serde_json::Value = serde_json::from_slice(&project_response.body).unwrap();
        let path = format!("/api/projects/{}/tasks", project["id"].as_str().unwrap());
        let task_response = dispatch(
            &post(
                &path,
                serde_json::json!({"title":"Browser task","agent_id":"opencode","draft":"Keep this unsent"}),
                Some(&authorization),
            ),
            &state,
            port,
        )
        .await;
        assert_eq!(task_response.status, 201);
        let task: serde_json::Value = serde_json::from_slice(&task_response.body).unwrap();
        let draft_path = format!("/api/tasks/{}/draft", task["id"].as_str().unwrap());
        let draft_response = dispatch(
            &make_request(&draft_path, port, Some(&authorization)),
            &state,
            port,
        )
        .await;
        assert_eq!(draft_response.status, 200);
        let draft: serde_json::Value = serde_json::from_slice(&draft_response.body).unwrap();
        assert_eq!(draft["text"], "Keep this unsent");
        assert_eq!(draft["truncated"], false);
        let saved = dispatch(
            &post(
                &draft_path,
                serde_json::json!({"text":"Edited in the browser"}),
                Some(&authorization),
            ),
            &state,
            port,
        )
        .await;
        assert_eq!(saved.status, 200);
        let reloaded = dispatch(
            &make_request(&draft_path, port, Some(&authorization)),
            &state,
            port,
        )
        .await;
        let draft: serde_json::Value = serde_json::from_slice(&reloaded.body).unwrap();
        assert_eq!(draft["text"], "Edited in the browser");
    }

    #[tokio::test]
    async fn task_thread_requires_auth_and_returns_bounded_recent_messages() {
        let directory = tempfile::tempdir().unwrap();
        let workspace = WorkspaceService::memory().unwrap();
        let project = workspace
            .add_local_workspace(directory.path().to_path_buf())
            .await
            .unwrap();
        let task = workspace
            .create_task(project.id, "A task".into(), "opencode".into())
            .await
            .unwrap();
        for index in 0..MAX_THREAD_MESSAGES + 1 {
            workspace
                .record(
                    task.thread_id,
                    synara_core::ThreadEvent::TextDelta {
                        message_id: Some(format!("message-{index}")),
                        role: synara_core::Role::User,
                        text: if index == MAX_THREAD_MESSAGES {
                            "x".repeat(MAX_THREAD_MESSAGE_CHARS + 1)
                        } else {
                            format!("text-{index}")
                        },
                    },
                )
                .await
                .unwrap();
        }
        workspace
            .record(
                task.thread_id,
                synara_core::ThreadEvent::TextDelta {
                    message_id: Some("private-reasoning".into()),
                    role: synara_core::Role::Reasoning,
                    text: "hidden private thought".into(),
                },
            )
            .await
            .unwrap();
        let state = Arc::new(AppState::new(TOKEN).unwrap());
        state.install_runtime(workspace.clone(), None).await;
        let path = format!("/api/tasks/{}/thread", task.id);
        assert_eq!(
            dispatch(&make_request(&path, 17341, None), &state, 17341)
                .await
                .status,
            401
        );
        assert_eq!(
            dispatch(
                &make_request(
                    "/api/tasks/not-a-task/thread",
                    17341,
                    Some(&format!("Bearer {TOKEN}")),
                ),
                &state,
                17341,
            )
            .await
            .status,
            404
        );
        let response = dispatch(
            &make_request(&path, 17341, Some(&format!("Bearer {TOKEN}"))),
            &state,
            17341,
        )
        .await;
        assert_eq!(response.status, 200);
        assert!(response.body.len() < MAX_RESPONSE_BYTES);
        assert!(!String::from_utf8_lossy(&response.body).contains("hidden private thought"));
        let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert_eq!(json["task_id"], task.id.to_string());
        assert_eq!(json["truncated"], true);
        assert_eq!(json["next_before"], 1);
        assert_eq!(
            json["messages"].as_array().unwrap().len(),
            MAX_THREAD_MESSAGES
        );
        assert_eq!(json["messages"][0]["text"], "text-1");
        assert_eq!(json["messages"][MAX_THREAD_MESSAGES - 1]["truncated"], true);
        assert_eq!(
            json["messages"][MAX_THREAD_MESSAGES - 1]["text"]
                .as_str()
                .unwrap()
                .chars()
                .count(),
            MAX_THREAD_MESSAGE_CHARS
        );

        let earlier = dispatch(
            &make_request(
                &format!("{path}?before=1"),
                17341,
                Some(&format!("Bearer {TOKEN}")),
            ),
            &state,
            17341,
        )
        .await;
        assert_eq!(earlier.status, 200);
        let earlier: serde_json::Value = serde_json::from_slice(&earlier.body).unwrap();
        assert_eq!(earlier["messages"].as_array().unwrap().len(), 1);
        assert_eq!(earlier["messages"][0]["text"], "text-0");
        assert!(earlier["next_before"].is_null());
        assert_eq!(
            dispatch(
                &make_request(
                    &format!("{path}?before=bogus"),
                    17341,
                    Some(&format!("Bearer {TOKEN}")),
                ),
                &state,
                17341,
            )
            .await
            .status,
            400
        );

        for index in 0..MAX_THREAD_MESSAGES {
            workspace
                .record(
                    task.thread_id,
                    synara_core::ThreadEvent::TextDelta {
                        message_id: Some(format!("emoji-{index}")),
                        role: synara_core::Role::Assistant,
                        text: "😀".repeat(MAX_THREAD_MESSAGE_CHARS),
                    },
                )
                .await
                .unwrap();
        }
        let response = dispatch(
            &make_request(&path, 17341, Some(&format!("Bearer {TOKEN}"))),
            &state,
            17341,
        )
        .await;
        assert_eq!(response.status, 200);
        assert!(response.body.len() <= MAX_RESPONSE_BYTES);
        let json: serde_json::Value = serde_json::from_slice(&response.body).unwrap();
        assert!(json["messages"].as_array().unwrap().len() < MAX_THREAD_MESSAGES);
        assert_eq!(
            json["messages"].as_array().unwrap().last().unwrap()["id"],
            "emoji-23"
        );
        assert_eq!(json["truncated"], true);
        assert!(json["next_before"].as_u64().unwrap() > 0);
    }

    #[tokio::test]
    async fn health_is_generic_and_redacts_token_and_database_location() {
        let state = AppState::new(TOKEN).unwrap();
        let path = "/secret/private/workspace.sqlite3";
        let response = dispatch(&make_request("/health", 17341, None), &state, 17341).await;
        assert_eq!(response.status, 503);
        let body = String::from_utf8(response.body).unwrap();
        assert!(body.contains("starting"));
        assert!(!body.contains(TOKEN));
        assert!(!body.contains(path));
        assert!(!body.contains("sqlite"));
    }

    #[tokio::test]
    async fn readiness_is_503_while_starting_and_after_shutdown() {
        let state = AppState::new(TOKEN).unwrap();
        assert_eq!(
            dispatch(&make_request("/ready", 17341, None), &state, 17341)
                .await
                .status,
            503
        );
        state
            .install_runtime(WorkspaceService::memory().unwrap(), None)
            .await;
        assert_eq!(
            dispatch(&make_request("/ready", 17341, None), &state, 17341)
                .await
                .status,
            200
        );
        state.transition(Lifecycle::Stopping);
        let response = dispatch(&make_request("/ready", 17341, None), &state, 17341).await;
        assert_eq!(response.status, 503);
        assert!(
            String::from_utf8(response.body)
                .unwrap()
                .contains("stopping")
        );
    }

    #[tokio::test]
    async fn a_late_startup_completion_cannot_restore_readiness_after_shutdown() {
        let state = AppState::new(TOKEN).unwrap();
        state.transition(Lifecycle::Stopping);
        state
            .install_runtime(WorkspaceService::memory().unwrap(), None)
            .await;
        assert_eq!(state.state(), Lifecycle::Stopping);
        assert!(state.runtime.read().await.is_none());
        assert_eq!(
            dispatch(&make_request("/ready", 17341, None), &state, 17341)
                .await
                .status,
            503
        );
    }

    #[tokio::test]
    async fn host_and_origin_checks_reject_non_loopback_or_cross_origin_requests() {
        let state = ready_state(TOKEN).await;
        let mut request = make_request("/health", 17341, None);
        request.headers[0].1 = "example.test:17341".into();
        assert_eq!(dispatch(&request, &state, 17341).await.status, 421);
        let mut request = make_request("/health", 17341, None);
        request
            .headers
            .push(("origin".into(), "http://localhost:17341".into()));
        assert_eq!(dispatch(&request, &state, 17341).await.status, 421);
        let mut request = make_request("/health", 17341, None);
        request
            .headers
            .push(("origin".into(), "https://127.0.0.1:17341".into()));
        assert_eq!(dispatch(&request, &state, 17341).await.status, 421);
    }

    #[tokio::test]
    async fn duplicate_content_length_is_rejected_even_for_zero_length_gets() {
        let state = ready_state(TOKEN).await;
        let mut request = make_request("/api/catalog", 17341, Some(&format!("Bearer {TOKEN}")));
        request.headers.push(("content-length".into(), "0".into()));
        request.headers.push(("content-length".into(), "0".into()));
        assert_eq!(dispatch(&request, &state, 17341).await.status, 400);
    }

    #[test]
    fn catalog_is_row_and_field_bounded_and_omits_workspace_paths() {
        let workspaces = (0..MAX_CATALOG_ROWS + 1)
            .map(|_| Workspace {
                id: synara_core::WorkspaceId::new(),
                name: "W".repeat(1000),
                location: WorkspaceLocation::Local {
                    root: PathBuf::from("/private/workspace/path"),
                },
            })
            .collect();
        let bytes = catalog_view(workspaces, vec![], vec![]).unwrap();
        assert!(bytes.len() < MAX_RESPONSE_BYTES);
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            json["workspaces"].as_array().unwrap().len(),
            MAX_CATALOG_ROWS
        );
        assert_eq!(json["truncated"]["workspaces"], true);
        assert!(
            !String::from_utf8(bytes)
                .unwrap()
                .contains("/private/workspace/path")
        );
    }

    #[tokio::test]
    async fn occupied_port_is_reported_and_server_shutdown_releases_listener() {
        let occupied = TcpListener::bind(SocketAddr::new(DEFAULT_BIND, 0))
            .await
            .unwrap();
        let port = occupied.local_addr().unwrap().port();
        let config = ServerConfig::new(
            DEFAULT_BIND,
            port,
            PathBuf::from("unused.sqlite3"),
            TOKEN.into(),
        );
        assert!(RunningServer::start(config).await.is_err());
        drop(occupied);

        let directory = tempfile::tempdir().unwrap();
        let config = ServerConfig::new(
            DEFAULT_BIND,
            0,
            directory.path().join("server.sqlite3"),
            TOKEN.into(),
        );
        let mut server = RunningServer::start(config).await.unwrap();
        let address = server.address();
        let state = server.state.clone();
        server.wait_ready().await.unwrap();
        let response = http_get(address, "/health", None).await;
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        server.shutdown().await.unwrap();
        assert_eq!(health_response(state.state()).status, 503);
        assert!(state.runtime.read().await.is_none());
        assert!(TcpStream::connect(address).await.is_err());
    }

    #[tokio::test]
    async fn startup_refuses_a_database_owned_by_another_process() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("shared.sqlite3");
        let owner = WorkspaceOwnerLock::acquire(&database_path).unwrap();
        let mut server = RunningServer::start(ServerConfig::new(
            DEFAULT_BIND,
            0,
            database_path,
            TOKEN.into(),
        ))
        .await
        .unwrap();
        assert!(server.wait_ready().await.is_err());
        assert_eq!(health_response(server.state.state()).status, 503);
        server.shutdown().await.unwrap();
        drop(owner);
    }

    #[tokio::test]
    async fn one_server_owns_a_database_until_shutdown_then_releases_it() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("shared.sqlite3");
        let mut first = RunningServer::start(ServerConfig::new(
            DEFAULT_BIND,
            0,
            database_path.clone(),
            TOKEN.into(),
        ))
        .await
        .unwrap();
        first.wait_ready().await.unwrap();

        let mut second = RunningServer::start(ServerConfig::new(
            DEFAULT_BIND,
            0,
            database_path.clone(),
            TOKEN.into(),
        ))
        .await
        .unwrap();
        assert!(second.wait_ready().await.is_err());
        second.shutdown().await.unwrap();
        first.shutdown().await.unwrap();

        let mut third = RunningServer::start(ServerConfig::new(
            DEFAULT_BIND,
            0,
            database_path,
            TOKEN.into(),
        ))
        .await
        .unwrap();
        third.wait_ready().await.unwrap();
        third.shutdown().await.unwrap();
    }

    async fn http_get(address: SocketAddr, path: &str, authorization: Option<&str>) -> String {
        let mut stream = TcpStream::connect(address).await.unwrap();
        let host = if address.is_ipv6() {
            format!("[{}]:{}", address.ip(), address.port())
        } else {
            format!("{}:{}", address.ip(), address.port())
        };
        let mut request = format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n");
        if let Some(token) = authorization {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        request.push_str("\r\n");
        stream.write_all(request.as_bytes()).await.unwrap();
        let mut response = Vec::new();
        timeout(Duration::from_secs(2), stream.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        String::from_utf8(response).unwrap()
    }

    #[tokio::test]
    async fn http_api_returns_json_without_exposing_server_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("private.sqlite3");
        let server = RunningServer::start(ServerConfig::new(
            DEFAULT_BIND,
            0,
            database_path.clone(),
            TOKEN.into(),
        ))
        .await
        .unwrap();
        let address = server.address();
        let ready = server.state.clone();
        let mut lifecycle = ready.subscribe();
        while *lifecycle.borrow() == Lifecycle::Starting {
            lifecycle.changed().await.unwrap();
        }
        let response = http_get(address, "/health", None).await;
        assert!(response.contains("Content-Type: application/json; charset=utf-8"));
        assert!(!response.contains(TOKEN));
        assert!(!response.contains(database_path.to_str().unwrap()));
        server.shutdown().await.unwrap();
    }

    #[test]
    fn default_database_is_named_and_separate_from_the_native_app_database() {
        assert_eq!(
            headless_database_path(Path::new("/data")),
            PathBuf::from("/data/synara/headless-server.sqlite3")
        );
        assert_ne!(
            headless_database_path(Path::new("/data")),
            PathBuf::from("/data/synara/native/native-workspace.sqlite3")
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn private_default_directory_is_owner_only_and_explicit_parent_is_untouched() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let private_database = directory
            .path()
            .join("private")
            .join("headless-server.sqlite3");
        prepare_database_parent(&private_database, true)
            .await
            .unwrap();
        assert_eq!(
            std::fs::metadata(private_database.parent().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );

        let explicit_parent = directory.path().join("explicit");
        std::fs::create_dir(&explicit_parent).unwrap();
        std::fs::set_permissions(&explicit_parent, std::fs::Permissions::from_mode(0o755)).unwrap();
        prepare_database_parent(&explicit_parent.join("workspace.sqlite3"), false)
            .await
            .unwrap();
        assert_eq!(
            std::fs::metadata(&explicit_parent)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }

    #[cfg(unix)]
    #[test]
    fn token_file_must_be_private_and_symlink_free() {
        use std::os::unix::{fs::PermissionsExt, fs::symlink};
        let directory = tempfile::tempdir().unwrap();
        let token_file = directory.path().join("token");
        std::fs::write(&token_file, TOKEN).unwrap();
        std::fs::set_permissions(&token_file, std::fs::Permissions::from_mode(0o600)).unwrap();
        assert_eq!(load_bearer_token(Some(&token_file)).unwrap(), TOKEN);
        std::fs::set_permissions(&token_file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(load_bearer_token(Some(&token_file)).is_err());
        let link = directory.path().join("token-link");
        symlink(&token_file, &link).unwrap();
        assert!(load_bearer_token(Some(&link)).is_err());
    }
}
