//! Backend process supervision for the desktop shell.
//!
//! The shell owns the native backend lifecycle in desktop mode: reuse an
//! already healthy server, otherwise spawn the sidecar/backend, wait for
//! readiness, restart it after unexpected exits, and stop it when the shell
//! shuts down.

use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};


const LOG_FILE_MAX_BYTES: u64 = 10 * 1024 * 1024;
const LOG_FILE_MAX_FILES: usize = 10;
const BACKEND_FAILURE_OUTPUT_TAIL_BYTES: usize = 8 * 1024;
const BACKEND_RESTART_MAX_DELAY: Duration = Duration::from_secs(10);


/// Rolling byte tail of one backend session's output. Chunks are appended as
/// they are forwarded to the durable log; only the bounded tail is kept in memory.
#[derive(Clone)]
struct OutputTail {
    bytes: Arc<Mutex<std::collections::VecDeque<u8>>>,
}

impl OutputTail {
    fn new() -> Self {
        Self { bytes: Arc::new(Mutex::new(std::collections::VecDeque::new())) }
    }

    fn push(&self, chunk: &[u8]) {
        if chunk.is_empty() { return; }
        let Ok(mut bytes) = self.bytes.lock() else { return; };
        if chunk.len() >= BACKEND_FAILURE_OUTPUT_TAIL_BYTES {
            bytes.clear();
            bytes.extend(chunk[chunk.len() - BACKEND_FAILURE_OUTPUT_TAIL_BYTES..].iter().copied());
            return;
        }
        while bytes.len() + chunk.len() > BACKEND_FAILURE_OUTPUT_TAIL_BYTES {
            bytes.pop_front();
        }
        bytes.extend(chunk.iter().copied());
    }

    fn read(&self) -> String {
        let Ok(bytes) = self.bytes.lock() else { return String::new() };
        String::from_utf8_lossy(&bytes.iter().copied().collect::<Vec<_>>())
            .replace('\r', "")
            .trim()
            .to_string()
    }
}

fn backend_restart_delay(attempt: u32) -> Duration {
    let shift = attempt.saturating_sub(1).min(5);
    let millis = 500u64.saturating_mul(1u64 << shift);
    Duration::from_millis(millis).min(BACKEND_RESTART_MAX_DELAY)
}

/// Shared rotating backend log sink. The child owns neither the rotation nor
/// the file descriptor, so output remains bounded across crashes/restarts.
struct RotatingFile {
    path: PathBuf,
    max_bytes: u64,
    max_files: usize,
    current_size: u64,
    file: Option<fs::File>,
}

impl RotatingFile {
    fn open(path: PathBuf, max_bytes: u64, max_files: usize) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        prune_overflow_backups(&path, max_files);
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let current_size = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
        Ok(Self { path, max_bytes, max_files, current_size, file: Some(file) })
    }

    fn write_chunk(&mut self, chunk: &[u8]) -> io::Result<()> {
        if chunk.is_empty() { return Ok(()); }
        if self.current_size > 0 && self.current_size + chunk.len() as u64 > self.max_bytes {
            self.rotate()?;
        }
        if self.file.is_none() {
            self.file = Some(OpenOptions::new().create(true).append(true).open(&self.path)?);
        }
        let file = self.file.as_mut().expect("rotating log file");
        file.write_all(chunk)?;
        self.current_size += chunk.len() as u64;
        if self.current_size > self.max_bytes {
            self.rotate()?;
        }
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        self.file = None;
        let oldest = self.with_suffix(self.max_files);
        if oldest.exists() {
            fs::remove_file(&oldest)?;
        }
        for index in (1..self.max_files).rev() {
            let source = self.with_suffix(index);
            let target = self.with_suffix(index + 1);
            if source.exists() {
                fs::rename(source, target)?;
            }
        }
        if self.path.exists() {
            fs::rename(&self.path, self.with_suffix(1))?;
        }
        self.current_size = 0;
        Ok(())
    }

    fn with_suffix(&self, index: usize) -> PathBuf {
        PathBuf::from(format!("{}.{}", self.path.display(), index))
    }
}

fn prune_overflow_backups(path: &Path, max_files: usize) {
    let Some(parent) = path.parent() else { return };
    let Some(base) = path.file_name().and_then(|name| name.to_str()) else { return };
    let Ok(entries) = fs::read_dir(parent) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let Some(suffix) = name.strip_prefix(&format!("{base}.")) else { continue };
        let Ok(index) = suffix.parse::<usize>() else { continue };
        if index > max_files {
            let _ = fs::remove_file(entry.path());
        }
    }
}

fn forward_backend_output(mut reader: impl Read + Send + 'static, sink: Arc<Mutex<RotatingFile>>, tail: OutputTail) {
    thread::spawn(move || {
        let mut buffer = [0u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => return,
                Ok(count) => {
                    tail.push(&buffer[..count]);
                    if let Ok(mut sink) = sink.lock() {
                        let _ = sink.write_chunk(&buffer[..count]);
                    }
                }
                Err(_) => return,
            }
        }
    });
}

const READY_TIMEOUT: Duration = Duration::from_secs(20);
const READY_POLL: Duration = Duration::from_millis(100);
const STOP_GRACE: Duration = Duration::from_secs(4);
const MAX_RESTARTS: u32 = 5;

#[derive(Clone, Debug)]
struct BackendConfig {
    binary: PathBuf,
    port: u16,
    home_dir: PathBuf,
    log_path: PathBuf,
}

struct SharedState {
    child: Option<Child>,
    config: BackendConfig,
    start_failures: u32,
    output_tail: OutputTail,
    restart_pending: bool,
}

struct SupervisorInner {
    state: Mutex<SharedState>,
    running: AtomicBool,
    watchdog: Mutex<Option<JoinHandle<()>>>,
}

static MANAGED_SUPERVISOR: std::sync::OnceLock<Mutex<Option<BackendSupervisor>>> = std::sync::OnceLock::new();

fn managed_supervisor_slot() -> &'static Mutex<Option<BackendSupervisor>> {
    MANAGED_SUPERVISOR.get_or_init(|| Mutex::new(None))
}

fn register_managed_supervisor(supervisor: &BackendSupervisor) {
    if let Ok(mut slot) = managed_supervisor_slot().lock() {
        *slot = Some(supervisor.clone());
    }
}

/// Stop the backend owned by this shell. Update installation exits with
/// `process::exit`, whose destructors do not run, so the updater explicitly
/// uses this hook instead of leaking the sidecar across the bundle swap.
pub fn shutdown_managed_backend() {
    let supervisor = managed_supervisor_slot()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(supervisor) = supervisor {
        supervisor.shutdown();
    }
}

#[derive(Clone)]
pub struct BackendSupervisor {
    inner: Arc<SupervisorInner>,
}

pub enum BackendLaunch {
    External,
    Managed(BackendSupervisor),
    Unavailable(String),
}

impl BackendLaunch {
    pub fn supervisor(&self) -> Option<BackendSupervisor> {
        match self {
            Self::Managed(supervisor) => Some(supervisor.clone()),
            _ => None,
        }
    }
}

impl BackendSupervisor {
    fn start(config: BackendConfig) -> Result<Self, String> {
        let (mut child, output_tail) = spawn_backend(&config)
            .map_err(|e| {
                crate::reliability::track("backend.spawn_failed");
                format!("avvio backend {} fallito: {e}", config.binary.display())
            })?;
        wait_until_ready(&mut child, config.port).map_err(|e| {
            crate::reliability::track("backend.exit_before_ready");
            let _ = terminate_child(&mut child);
            let detail = output_tail.read();
            if detail.is_empty() {
                format!("backend non pronto sulla porta {}: {e}", config.port)
            } else {
                format!(
                    "backend non pronto sulla porta {}: {e}\n{}",
                    config.port,
                    summarize_backend_failure_output(&detail)
                )
            }
        })?;

        let state = Arc::new(SupervisorInner {
            state: Mutex::new(SharedState {
                child: Some(child),
                config,
                start_failures: 0,
                output_tail,
                restart_pending: false,
            }),
            running: AtomicBool::new(true),
            watchdog: Mutex::new(None),
        });

        let thread_state = state.clone();
        let watchdog = thread::spawn(move || watchdog_loop(thread_state));
        *state.watchdog.lock().expect("supervisor watchdog slot") = Some(watchdog);
        Ok(Self { inner: state })
    }

    pub fn shutdown(&self) {
        self.inner.running.store(false, Ordering::SeqCst);
        let child = {
            let mut state = self.inner.state.lock().expect("supervisor state");
            state.child.take()
        };
        if let Some(mut child) = child {
            let _ = terminate_child(&mut child);
        }
        if let Some(watchdog) = self
            .inner
            .watchdog
            .lock()
            .expect("supervisor watchdog slot")
            .take()
        {
            let _ = watchdog.join();
        }
    }
}

impl Drop for BackendSupervisor {
    fn drop(&mut self) {
        // `shutdown` is idempotent. The signal thread intentionally retains a
        // clone for the whole process lifetime, so strong-count ownership is
        // not a usable shutdown signal here.
        self.shutdown();
    }
}

fn watchdog_loop(inner: Arc<SupervisorInner>) {
    let mut last_stall_count: Option<u64> = None;
    let mut last_health_poll = Instant::now() - Duration::from_secs(1);
    while inner.running.load(Ordering::SeqCst) {
        let health_port = {
            let state = inner.state.lock().expect("supervisor state");
            state.child.as_ref().map(|_| state.config.port)
        };
        if let Some(port) = health_port {
            if last_health_poll.elapsed() >= Duration::from_secs(1) {
                last_health_poll = Instant::now();
                if let Some(payload) = backend_health_payload(port) {
                    let count = payload
                        .get("eventLoop")
                        .and_then(|event_loop| event_loop.get("stallWindowCount"))
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    match last_stall_count {
                        None => last_stall_count = Some(count),
                        Some(previous) if count > previous => {
                            for _ in previous..count {
                                crate::reliability::track("backend.event_loop_stall");
                            }
                            last_stall_count = Some(count);
                        }
                        Some(previous) if count < previous => last_stall_count = Some(count),
                        _ => {}
                    }
                }
            }
        } else {
            last_stall_count = None;
        }
        let exit_reason = {
            let mut state = inner.state.lock().expect("supervisor state");
            if let Some(child) = state.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        state.child = None;
                        state.restart_pending = true;
                        Some(status.to_string())
                    }
                    Ok(None) => None,
                    Err(e) => {
                        state.child = None;
                        state.restart_pending = true;
                        Some(format!("stato processo non leggibile: {e}"))
                    }
                }
            } else if state.restart_pending {
                Some("backend assente dopo un restart fallito".to_string())
            } else {
                return;
            }
        };

        let Some(reason) = exit_reason else {
            thread::sleep(Duration::from_millis(250));
            continue;
        };
        if !inner.running.load(Ordering::SeqCst) {
            return;
        }

        crate::reliability::track("backend.exit_after_ready");
        let (restart_index, config) = {
            let mut state = inner.state.lock().expect("supervisor state");
            if state.start_failures >= MAX_RESTARTS - 1 {
                eprintln!(
                    "backend supervision: processo uscito ({reason}); limite di {MAX_RESTARTS} restart raggiunto
{}",
                    summarize_backend_failure_output(&state.output_tail.read())
                );
                return;
            }
            state.start_failures += 1;
            (state.start_failures, state.config.clone())
        };
        eprintln!(
            "backend supervision: processo uscito ({reason}), restart {restart_index}/{MAX_RESTARTS}"
        );

        interruptible_sleep(&inner, backend_restart_delay(restart_index));
        if !inner.running.load(Ordering::SeqCst) {
            return;
        }

        let restart = spawn_backend(&config)
            .map_err(|e| {
                crate::reliability::track("backend.spawn_failed");
                e.to_string()
            })
            .and_then(
                |(mut child, output_tail)| match wait_until_ready(&mut child, config.port) {
                    Ok(()) => Ok((child, output_tail)),
                    Err(error) => {
                        crate::reliability::track("backend.exit_before_ready");
                        let _ = terminate_child(&mut child);
                        Err(error)
                    }
                },
            );
        match restart {
            Ok((child, output_tail)) => {
                let mut state = inner.state.lock().expect("supervisor state");
                state.child = Some(child);
                state.output_tail = output_tail;
                state.start_failures = 0;
                state.restart_pending = false;
                eprintln!(
                    "backend supervision: backend riavviato sulla porta {}",
                    config.port
                );
            }
            Err(e) => {
                eprintln!("backend supervision: restart fallito: {e}");
                interruptible_sleep(&inner, Duration::from_secs(1));
            }
        }
    }
}

fn interruptible_sleep(inner: &SupervisorInner, duration: Duration) {
    let deadline = Instant::now() + duration;
    while inner.running.load(Ordering::SeqCst) && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(100));
    }
}

fn spawn_backend(config: &BackendConfig) -> io::Result<(Child, OutputTail)> {
    if let Some(parent) = config.log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir_all(&config.home_dir)?;
    let rotating_log = RotatingFile::open(
        config.log_path.clone(),
        LOG_FILE_MAX_BYTES,
        LOG_FILE_MAX_FILES,
    );

    let mut command = Command::new(&config.binary);
    command
        .arg("--home-dir")
        .arg(&config.home_dir)
        .env("PORT", config.port.to_string())
        .env("SYNARA_NO_UPSTREAM", "1")
        .env("SYNARA_MODE", "desktop")
        .env("SYNARA_DESKTOP_FLAVOR", crate::desktop_identity::runtime_flavor())
        .stdin(Stdio::null());
    match rotating_log {
        Ok(sink) => {
            command.stdout(Stdio::piped()).stderr(Stdio::piped());
            let sink = Arc::new(Mutex::new(sink));
            let output_tail = OutputTail::new();
            let mut child = command.spawn()?;
            if let Some(stdout) = child.stdout.take() {
                forward_backend_output(stdout, sink.clone(), output_tail.clone());
            }
            if let Some(stderr) = child.stderr.take() {
                forward_backend_output(stderr, sink, output_tail.clone());
            }
            eprintln!(
                "backend supervision: avviato pid {} sulla porta {} (log {})",
                child.id(),
                config.port,
                config.log_path.display()
            );
            return Ok((child, output_tail));
        }
        Err(error) => {
            eprintln!(
                "backend supervision: rotating log unavailable at {}: {error}; using direct append",
                config.log_path.display()
            );
            let stdout = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&config.log_path)?;
            let stderr = stdout.try_clone()?;
            command.stdout(Stdio::from(stdout)).stderr(Stdio::from(stderr));
        }
    }

    let child = command.spawn()?;
    eprintln!(
        "backend supervision: avviato pid {} sulla porta {} (log {})",
        child.id(),
        config.port,
        config.log_path.display()
    );
    Ok((child, OutputTail::new()))
}

fn wait_until_ready(child: &mut Child, port: u16) -> Result<(), String> {
    let deadline = Instant::now() + READY_TIMEOUT;
    loop {
        if backend_ready(port) {
            return Ok(());
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|e| format!("stato processo non leggibile: {e}"))?
        {
            return Err(format!("processo uscito prima della readiness ({status})"));
        }
        if Instant::now() >= deadline {
            return Err("timeout readiness /health".to_string());
        }
        thread::sleep(READY_POLL);
    }
}

fn backend_health_payload(port: u16) -> Option<serde_json::Value> {
    let url = format!("http://127.0.0.1:{port}/health");
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_millis(250))
        .timeout_read(Duration::from_millis(250))
        .build()
        .get(&url)
        .call()
        .ok()
        .filter(|response| response.status() == 200)
        .and_then(|response| response.into_json::<serde_json::Value>().ok())
}

fn backend_ready(port: u16) -> bool {
    backend_health_payload(port)
        .is_some_and(|payload| payload.get("startupReady").and_then(serde_json::Value::as_bool) == Some(true))
}

fn terminate_child(child: &mut Child) -> io::Result<()> {
    if child.try_wait()?.is_some() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        // SAFETY: `kill` is called with a live child process id and a standard
        // signal number; it does not mutate memory or transfer ownership.
        unsafe {
            libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
        }
    }
    #[cfg(not(unix))]
    {
        child.kill()?;
    }

    let deadline = Instant::now() + STOP_GRACE;
    loop {
        if child.try_wait()?.is_some() {
            return Ok(());
        }
        if Instant::now() >= deadline {
            child.kill()?;
            let _ = child.wait();
            return Ok(());
        }
        thread::sleep(Duration::from_millis(80));
    }
}

pub fn ensure_backend() -> BackendLaunch {
    if std::env::var("SYNARA_BACKEND_AUTOSTART").ok().as_deref() == Some("0") {
        return BackendLaunch::External;
    }
    let port = configured_port();
    if backend_ready(port) {
        eprintln!("backend supervision: backend esistente riutilizzato sulla porta {port}");
        return BackendLaunch::External;
    }

    let Some(binary) = resolve_backend_binary() else {
        return BackendLaunch::Unavailable(
            "backend non trovato: imposta SYNARA_BACKEND_BIN o avvia rust/app manualmente".into(),
        );
    };
    let home_dir = configured_home_dir();
    let log_path = home_dir.join("userdata").join("logs").join("backend.log");
    let config = BackendConfig {
        binary,
        port,
        home_dir,
        log_path,
    };

    match BackendSupervisor::start(config) {
        Ok(supervisor) => {
            register_managed_supervisor(&supervisor);
            BackendLaunch::Managed(supervisor)
        }
        Err(error) => BackendLaunch::Unavailable(error),
    }
}

fn configured_port() -> u16 {
    if let Ok(url) = std::env::var("SYNARA_HTTP") {
        if let Some(port) = port_from_http_url(&url) {
            return port;
        }
    }
    std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8080)
}

fn port_from_http_url(value: &str) -> Option<u16> {
    let uri = http::Uri::try_from(value).ok()?;
    if let Some(port) = uri.authority().and_then(|authority| authority.port_u16()) {
        return Some(port);
    }
    match uri.scheme_str() {
        Some("https") => Some(443),
        Some("http") => Some(80),
        _ => None,
    }
}

pub(crate) fn configured_home_dir() -> PathBuf {
    crate::desktop_identity::configured_home_dir()
}

fn resolve_backend_binary() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("SYNARA_BACKEND_BIN") {
        let path = PathBuf::from(path);
        return path.is_file().then_some(path);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for name in packaged_backend_names() {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        if let Some(candidate) = dev_backend_binary_from_exe(&exe) {
            return Some(candidate);
        }
    }
    None
}

fn packaged_backend_names() -> &'static [&'static str] {
    #[cfg(target_os = "windows")]
    {
        &["synara-backend.exe", "index.exe"]
    }
    #[cfg(not(target_os = "windows"))]
    {
        &["synara-backend", "index"]
    }
}

fn dev_backend_binary_from_exe(exe: &Path) -> Option<PathBuf> {
    let binary_name = if cfg!(target_os = "windows") {
        "index.exe"
    } else {
        "index"
    };
    for ancestor in exe.ancestors() {
        let candidate = ancestor
            .join("rust")
            .join("app")
            .join("target")
            .join("debug")
            .join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub fn install_signal_shutdown(supervisor: BackendSupervisor) {
    thread::spawn(move || {
        #[cfg(unix)]
        let signal_list = [signal_hook::consts::SIGINT, signal_hook::consts::SIGTERM];
        #[cfg(not(unix))]
        let signal_list = [signal_hook::consts::SIGINT];
        let mut signals = match signal_hook::iterator::Signals::new(signal_list) {
            Ok(signals) => signals,
            Err(e) => {
                eprintln!("backend supervision: signal handler non installato: {e}");
                return;
            }
        };
        if signals.forever().next().is_some() {
            supervisor.shutdown();
            std::process::exit(130);
        }
    });
}

pub fn launch_summary(launch: &BackendLaunch) -> serde_json::Value {
    match launch {
        BackendLaunch::External => json!({"mode": "external"}),
        BackendLaunch::Managed(_) => json!({"mode": "managed"}),
        BackendLaunch::Unavailable(error) => json!({"mode": "unavailable", "error": error}),
    }
}


fn summarize_backend_failure_output(output: &str) -> String {
    let lines: Vec<&str> = output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if lines.is_empty() { return String::new(); }
    lines.iter().rev().take(8).rev().copied().collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn rotating_log_rotates_and_prunes_bounded_files() {
        let root = std::env::temp_dir().join(format!("synara-log-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("server-child.log");
        let mut sink = RotatingFile::open(path.clone(), 16, 2).unwrap();
        sink.write_chunk(b"0123456789").unwrap();
        sink.write_chunk(b"abcdefgh").unwrap();
        sink.write_chunk(b"ij").unwrap();
        assert!(root.join("server-child.log.1").is_file());
        sink.write_chunk(b"klmnopqrstuv").unwrap();
        sink.write_chunk(b"wx").unwrap();
        sink.write_chunk(b"yz").unwrap();
        assert!(root.join("server-child.log.2").is_file());
        assert!(!root.join("server-child.log.3").exists());
        assert!(fs::metadata(&path).unwrap().len() <= 16);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rotating_log_prunes_preexisting_overflow_backups() {
        let root = std::env::temp_dir().join(format!("synara-log-prune-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("server-child.log");
        fs::write(root.join("server-child.log.1"), b"1").unwrap();
        fs::write(root.join("server-child.log.2"), b"2").unwrap();
        fs::write(root.join("server-child.log.3"), b"3").unwrap();
        let _sink = RotatingFile::open(path, 16, 2).unwrap();
        assert!(root.join("server-child.log.1").is_file());
        assert!(root.join("server-child.log.2").is_file());
        assert!(!root.join("server-child.log.3").exists());
        let _ = fs::remove_dir_all(root);
    }


    #[test]
    fn restart_delay_doubles_and_caps() {
        assert_eq!(backend_restart_delay(1), Duration::from_millis(500));
        assert_eq!(backend_restart_delay(2), Duration::from_secs(1));
        assert_eq!(backend_restart_delay(5), Duration::from_secs(8));
        assert_eq!(backend_restart_delay(6), Duration::from_secs(10));
        assert_eq!(backend_restart_delay(99), Duration::from_secs(10));
    }

    #[test]
    fn output_tail_keeps_bounded_newest_bytes() {
        let tail = OutputTail::new();
        tail.push(&vec![b'a'; BACKEND_FAILURE_OUTPUT_TAIL_BYTES - 2]);
        tail.push(b"bcdef");
        let read = tail.read();
        assert_eq!(read.len(), BACKEND_FAILURE_OUTPUT_TAIL_BYTES);
        assert!(read.ends_with("bcdef"));
    }

    #[test]
    fn health_payload_requires_successful_json_response() {
        use std::io::{Read as _, Write as _};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buffer = [0_u8; 1024];
            let _ = stream.read(&mut buffer);
            let body = r#"{"startupReady":true,"eventLoop":{"stallWindowCount":2}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let payload = backend_health_payload(port).unwrap();
        assert_eq!(payload["startupReady"], serde_json::json!(true));
        server.join().unwrap();
    }

    #[test]
    fn parses_explicit_http_ports() {
        assert_eq!(port_from_http_url("http://127.0.0.1:8081"), Some(8081));
        assert_eq!(
            port_from_http_url("https://example.test:4443/x"),
            Some(4443)
        );
    }

    #[test]
    fn parses_scheme_default_ports() {
        assert_eq!(port_from_http_url("http://example.test"), Some(80));
        assert_eq!(port_from_http_url("https://example.test"), Some(443));
    }

    #[test]
    fn rejects_non_urls() {
        assert_eq!(port_from_http_url("not a url"), None);
        assert_eq!(port_from_http_url("ws://example.test"), None);
    }

    #[test]
    fn resolves_dev_backend_from_shell_target_path() {
        let root = std::env::temp_dir().join(format!("synara-backend-test-{}", std::process::id()));
        let exe = root
            .join("rust")
            .join("shell")
            .join("target")
            .join("debug")
            .join("synara-shell");
        let backend = root
            .join("rust")
            .join("app")
            .join("target")
            .join("debug")
            .join(if cfg!(windows) { "index.exe" } else { "index" });
        fs::create_dir_all(backend.parent().unwrap()).unwrap();
        fs::create_dir_all(exe.parent().unwrap()).unwrap();
        fs::write(&backend, b"").unwrap();
        assert_eq!(dev_backend_binary_from_exe(&exe), Some(backend));
        let _ = fs::remove_dir_all(root);
    }
}
