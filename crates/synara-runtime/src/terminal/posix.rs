use super::{TerminalRenderSnapshot, TerminalSnapshot};
use crate::{
    LaunchSpec, PasteDecision, PreparedPaste, RuntimeError, TerminalKey, TerminalModifiers,
    base_environment, encode_terminal_key, encode_terminal_text,
    terminal_screen::{TerminalScreen, check_terminal_size},
};
use filedescriptor::FileDescriptor;
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    os::fd::{AsRawFd, RawFd},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    time::{Duration, Instant},
};
use tokio::sync::watch;

const QUEUE_BYTES: usize = 1024 * 1024;
const QUEUE_MESSAGES: usize = 256;
const IO_BUDGET: usize = 128 * 1024;

struct Output {
    screen: TerminalScreen,
    error: Option<String>,
}
struct Signals {
    stop: AtomicBool,
    queued: AtomicUsize,
    resize: Mutex<Option<(u16, u16)>>,
}
enum Input {
    Bytes(Vec<u8>),
    Key(TerminalKey, TerminalModifiers),
    Paste(PreparedPaste, PasteDecision),
}
struct Message {
    input: Input,
    reserved: usize,
}

/// Only the worker owns the child and OS handles. All UI-facing control methods
/// enqueue bounded work or signal cancellation, never wait for the PTY to drain.
pub struct NativeTerminal {
    input: SyncSender<Message>,
    signals: Arc<Signals>,
    output: Arc<Mutex<Output>>,
    exit: watch::Receiver<Option<u32>>,
    worker: std::thread::Thread,
}
impl NativeTerminal {
    /// PTY allocation and initial process creation must run off the rendering thread.
    pub fn spawn(
        launch: &LaunchSpec,
        cwd: &Path,
        rows: u16,
        columns: u16,
    ) -> Result<Self, RuntimeError> {
        launch.validate()?;
        check_terminal_size(rows, columns)?;
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(RuntimeError::Invalid(
                "terminal directory must exist and be absolute".into(),
            ));
        }
        let pair = native_pty_system()
            .openpty(size(rows, columns))
            .map_err(pty_error)?;
        // Duplication is safe while master remains owned here. No raw descriptor
        // is adopted, and no child has started if any I/O setup fails.
        struct BorrowedRaw(RawFd);
        impl AsRawFd for BorrowedRaw {
            fn as_raw_fd(&self) -> RawFd {
                self.0
            }
        }
        let raw = BorrowedRaw(pair.master.as_raw_fd().ok_or_else(|| {
            RuntimeError::Unsupported("PTY does not expose a POSIX descriptor".into())
        })?);
        let mut io = FileDescriptor::dup(&raw).map_err(pty_error)?;
        io.set_non_blocking(true).map_err(pty_error)?;
        let mut command = CommandBuilder::new(&launch.command);
        command.args(&launch.args);
        command.cwd(cwd);
        command.env_clear();
        for (key, value) in base_environment() {
            command.env(key, value);
        }
        command.env("TERM", "xterm-256color");
        for (key, value) in &launch.env {
            command.env(key, value);
        }
        let child = pair.slave.spawn_command(command).map_err(pty_error)?;
        let owned = OwnedPty {
            child,
            master: pair.master,
            io,
            reaped: false,
        };
        drop(pair.slave);
        let output = Arc::new(Mutex::new(Output {
            screen: TerminalScreen::new(rows, columns)?,
            error: None,
        }));
        let signals = Arc::new(Signals {
            stop: AtomicBool::new(false),
            queued: AtomicUsize::new(0),
            resize: Mutex::new(None),
        });
        let (input, receive) = mpsc::sync_channel(QUEUE_MESSAGES);
        let (exit_tx, exit) = watch::channel(None);
        let worker_output = output.clone();
        let worker_signals = signals.clone();
        // If OS thread creation fails, dropping the captured OwnedPty performs
        // cleanup on this already-background startup thread.
        let thread = std::thread::Builder::new()
            .name("synara-pty".into())
            .spawn(move || {
                let (code, error) = run(owned, &receive, &worker_signals, &worker_output);
                worker_signals.stop.store(true, Ordering::Release);
                if let Ok(mut out) = worker_output.lock()
                    && let Some(error) = error
                {
                    out.error = Some(error);
                }
                // run has dropped every PTY descriptor and reaped the child before
                // publishing completion. Historical snapshots own no dead handles.
                let _ = exit_tx.send(Some(code));
            })?;
        Ok(Self {
            input,
            signals,
            output,
            exit,
            worker: thread.thread().clone(),
        })
    }
    fn enqueue(&self, input: Input, reserved: usize) -> Result<(), RuntimeError> {
        if self.signals.stop.load(Ordering::Acquire) {
            return Err(RuntimeError::Closed);
        }
        self.signals
            .queued
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                n.checked_add(reserved).filter(|n| *n <= QUEUE_BYTES)
            })
            .map_err(|_| RuntimeError::Limit)?;
        if let Err(error) = self.input.try_send(Message { input, reserved }) {
            self.signals.queued.fetch_sub(reserved, Ordering::AcqRel);
            return Err(match error {
                mpsc::TrySendError::Full(_) => RuntimeError::Limit,
                mpsc::TrySendError::Disconnected(_) => RuntimeError::Closed,
            });
        }
        self.worker.unpark();
        Ok(())
    }
    /// Raw input is reserved for trusted protocol/control paths. Clipboard data
    /// must go through paste(), not through this escape-capable API.
    pub fn input(&self, bytes: &[u8]) -> Result<(), RuntimeError> {
        if bytes.len() > crate::MAX_TERMINAL_INPUT_BYTES {
            return Err(RuntimeError::Limit);
        }
        self.enqueue(Input::Bytes(bytes.to_vec()), bytes.len())
    }
    pub fn text(&self, text: &str) -> Result<(), RuntimeError> {
        self.input(&encode_terminal_text(text)?)
    }
    pub fn key(&self, key: TerminalKey, modifiers: TerminalModifiers) -> Result<(), RuntimeError> {
        self.enqueue(Input::Key(key, modifiers), 32)
    }
    pub fn paste(&self, paste: PreparedPaste, decision: PasteDecision) -> Result<(), RuntimeError> {
        if paste.requires_review() && decision != PasteDecision::Approved {
            return Err(RuntimeError::Denied(
                "clipboard paste requires review".into(),
            ));
        }
        let size = paste.text().len() + 12;
        self.enqueue(Input::Paste(paste, decision), size)
    }
    pub fn resize(&self, rows: u16, columns: u16) -> Result<(), RuntimeError> {
        check_terminal_size(rows, columns)?;
        if self.signals.stop.load(Ordering::Acquire) {
            return Err(RuntimeError::Closed);
        }
        *self
            .signals
            .resize
            .lock()
            .map_err(|_| RuntimeError::Closed)? = Some((rows, columns));
        self.worker.unpark();
        Ok(())
    }
    /// Explicit user scrolling is independent from incoming output or typing.
    pub fn scrollback(&self, offset: usize) -> Result<(), RuntimeError> {
        self.output
            .lock()
            .map_err(|_| RuntimeError::Closed)?
            .screen
            .scroll(offset);
        Ok(())
    }
    pub fn snapshot(&self) -> Result<TerminalSnapshot, RuntimeError> {
        let out = self.output.lock().map_err(|_| RuntimeError::Closed)?;
        Ok(TerminalSnapshot {
            text: out.screen.text(),
            raw_tail: out.screen.raw_tail(),
            truncated: out.screen.truncated(),
            exit_code: *self.exit.borrow(),
            error: out.error.clone(),
            revision: out.screen.revision(),
        })
    }
    pub fn render_snapshot(&self) -> Result<TerminalRenderSnapshot, RuntimeError> {
        let out = self.output.lock().map_err(|_| RuntimeError::Closed)?;
        Ok(TerminalRenderSnapshot {
            grid: out.screen.grid(),
            exit_code: *self.exit.borrow(),
            error: out.error.clone(),
        })
    }
    /// Idempotent and independent of a full input queue or blocked child reader.
    pub fn kill(&self) -> Result<(), RuntimeError> {
        self.signals.stop.store(true, Ordering::Release);
        self.worker.unpark();
        Ok(())
    }
    pub async fn wait(&self) -> Result<u32, RuntimeError> {
        let mut exit = self.exit.clone();
        loop {
            if let Some(code) = *exit.borrow() {
                return Ok(code);
            }
            exit.changed().await.map_err(|_| RuntimeError::Closed)?;
        }
    }
    pub async fn shutdown(&self) -> Result<u32, RuntimeError> {
        self.kill()?;
        tokio::time::timeout(Duration::from_secs(3), self.wait())
            .await
            .map_err(|_| RuntimeError::Timeout)?
    }
}
impl Drop for NativeTerminal {
    fn drop(&mut self) {
        let _ = self.kill();
    }
}

struct OwnedPty {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Box<dyn MasterPty + Send>,
    io: FileDescriptor,
    reaped: bool,
}
impl OwnedPty {
    fn exited(&mut self) -> Result<bool, RuntimeError> {
        #[cfg(target_os = "linux")]
        {
            super::session::exited_without_reaping(
                self.child.process_id().ok_or(RuntimeError::Closed)?,
            )
        }
        #[cfg(not(target_os = "linux"))]
        {
            Ok(self.child.try_wait().map_err(pty_error)?.is_some())
        }
    }
    fn cleanup(&mut self) -> Result<(), RuntimeError> {
        #[cfg(target_os = "linux")]
        {
            super::session::cleanup_session(self.child.process_id().ok_or(RuntimeError::Closed)?)
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Other POSIX systems retain the library's foreground-PTY kill path.
            // Linux session-wide ownership evidence must not be claimed here.
            if let Some(group) = self.master.process_group_leader()
                && group > 1
            {
                crate::process::signal_tree(group as u32, true);
            }
            if let Some(pid) = self.child.process_id() {
                crate::process::signal_tree(pid, true);
            }
            self.child.kill().map_err(RuntimeError::Io)
        }
    }
}
impl Drop for OwnedPty {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = self.cleanup();
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn run(
    mut owned: OwnedPty,
    receive: &Receiver<Message>,
    signals: &Signals,
    output: &Mutex<Output>,
) -> (u32, Option<String>) {
    let mut error = None;
    let mut pending: Option<(Vec<u8>, usize, usize)> = None;
    let mut replies = Vec::new();
    loop {
        if signals.stop.load(Ordering::Acquire) {
            break;
        }
        match owned.exited() {
            Ok(true) => break,
            Err(e) => {
                error = Some(e.to_string());
                break;
            }
            Ok(false) => {}
        }
        let result = (|| -> Result<(), RuntimeError> {
            if let Some((rows, cols)) = signals
                .resize
                .lock()
                .map_err(|_| RuntimeError::Closed)?
                .take()
            {
                owned.master.resize(size(rows, cols)).map_err(pty_error)?;
                output
                    .lock()
                    .map_err(|_| RuntimeError::Closed)?
                    .screen
                    .resize(rows, cols)?;
            }
            drain(&mut owned.io, output, &mut replies)?;
            if pending.is_none() {
                if !replies.is_empty() {
                    pending = Some((std::mem::take(&mut replies), 0, 0));
                } else if let Ok(message) = receive.try_recv() {
                    let out = output.lock().map_err(|_| RuntimeError::Closed)?;
                    let bytes = match message.input {
                        Input::Bytes(bytes) => bytes,
                        Input::Key(key, modifiers) => {
                            encode_terminal_key(key, modifiers, out.screen.application_cursor())
                                .unwrap_or_default()
                        }
                        Input::Paste(paste, decision) => {
                            paste.encode(decision, out.screen.bracketed_paste())?
                        }
                    };
                    pending = Some((bytes, 0, message.reserved));
                }
            }
            if let Some((bytes, offset, reserved)) = &mut pending {
                if *offset < bytes.len() {
                    let end = bytes.len().min(offset.saturating_add(IO_BUDGET));
                    match owned.io.write(&bytes[*offset..end]) {
                        Ok(0) => return Err(RuntimeError::Closed),
                        Ok(written) => *offset += written,
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted
                            ) => {}
                        Err(e) => return Err(e.into()),
                    }
                }
                if *offset == bytes.len() {
                    signals.queued.fetch_sub(*reserved, Ordering::AcqRel);
                    pending = None;
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            error = Some(e.to_string());
            break;
        }
        // Fairness: a continuous output flood cannot starve stop, resize or input.
        std::thread::park_timeout(Duration::from_millis(4));
    }
    signals.stop.store(true, Ordering::Release);
    if let Err(e) = owned.cleanup() {
        error = Some(format!("terminal cleanup incomplete: {e}"));
    }
    // Kill is a fallback for partial startup/unsupported ownership primitives.
    // Keep an explicit error if session cleanup was not proven.
    let _ = owned.child.kill();
    let code = match owned.child.wait() {
        Ok(status) => {
            owned.reaped = true;
            status.exit_code()
        }
        Err(e) => {
            error = Some(e.to_string());
            1
        }
    };
    // Once owned jobs are gone, nonblocking reads consume final buffered output.
    // A deliberately detached process cannot hold teardown hostage indefinitely.
    let deadline = Instant::now() + Duration::from_millis(100);
    loop {
        match drain(&mut owned.io, output, &mut replies) {
            Ok(false) => break,
            Ok(true) if Instant::now() < deadline => continue,
            Ok(true) => break,
            Err(e) => {
                if error.is_none() {
                    error = Some(e.to_string());
                }
                break;
            }
        }
    }
    drop(owned);
    (code, error)
}

/// Returns whether output was read. PTY EIO after slave close is normal EOF.
fn drain(
    io: &mut FileDescriptor,
    output: &Mutex<Output>,
    replies: &mut Vec<u8>,
) -> Result<bool, RuntimeError> {
    let mut buffer = [0u8; 8192];
    let mut total = 0;
    while total < IO_BUDGET {
        match io.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                total += n;
                let response = output
                    .lock()
                    .map_err(|_| RuntimeError::Closed)?
                    .screen
                    .process(&buffer[..n]);
                if replies.len().saturating_add(response.len()) <= 16_384 {
                    replies.extend(response);
                }
            }
            Err(e) if e.raw_os_error() == Some(5) || e.kind() == std::io::ErrorKind::WouldBlock => {
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(total > 0)
}
fn size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}
fn pty_error(error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::Invalid(format!("terminal: {error}"))
}
