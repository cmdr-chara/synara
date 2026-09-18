//! ConPTY/portable worker ownership. Rendering owns snapshots, never OS handles.
use super::{TerminalRenderSnapshot, TerminalSnapshot};
use crate::{
    LaunchSpec, PasteDecision, PreparedPaste, RuntimeError, TerminalKey, TerminalModifiers,
    base_environment, encode_terminal_key, encode_terminal_text,
    terminal_screen::{TerminalScreen, check_terminal_size},
};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    io::{Read, Write},
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, Receiver, SyncSender},
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tokio::sync::watch;

const QUEUE_BYTES: usize = 1024 * 1024;
const QUEUE_MESSAGES: usize = 256;

struct Output {
    screen: TerminalScreen,
    error: Option<String>,
}
struct Signals {
    stop: AtomicBool,
    queued: AtomicUsize,
    resize: Mutex<Option<(u16, u16)>>,
}
struct Reservation {
    signals: Arc<Signals>,
    bytes: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        self.signals.queued.fetch_sub(self.bytes, Ordering::AcqRel);
    }
}
enum Input {
    Bytes(Vec<u8>),
    Key(TerminalKey, TerminalModifiers),
    Paste(PreparedPaste, PasteDecision),
}
struct Message {
    input: Input,
    reservation: Reservation,
}

pub struct NativeTerminal {
    input: SyncSender<Message>,
    signals: Arc<Signals>,
    output: Arc<Mutex<Output>>,
    exit: watch::Receiver<Option<u32>>,
    worker: std::thread::Thread,
}
impl NativeTerminal {
    /// PTY allocation and process creation must run off the rendering thread.
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
        let screen = TerminalScreen::new(rows, columns)?;
        let pair = native_pty_system()
            .openpty(size(rows, columns))
            .map_err(pty_error)?;
        let reader = pair.master.try_clone_reader().map_err(pty_error)?;
        let writer = pair.master.take_writer().map_err(pty_error)?;
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
        let signals = Arc::new(Signals {
            stop: AtomicBool::new(false),
            queued: AtomicUsize::new(0),
            resize: Mutex::new(None),
        });
        let child = pair.slave.spawn_command(command).map_err(pty_error)?;
        // This guard already owns the child if any subsequent thread setup fails.
        let mut owned = OwnedPty {
            child,
            master: Some(pair.master),
            signals: signals.clone(),
            workers: Vec::new(),
            reaped: false,
        };
        drop(pair.slave);
        let output = Arc::new(Mutex::new(Output {
            screen,
            error: None,
        }));
        let (input, receive) = mpsc::sync_channel(QUEUE_MESSAGES);
        let read_input = input.clone();
        let read_signals = signals.clone();
        let read_output = output.clone();
        owned.workers.push(
            std::thread::Builder::new()
                .name("synara-conpty-read".into())
                .spawn(move || read_output_loop(reader, read_input, read_signals, read_output))?,
        );
        let write_signals = signals.clone();
        let write_output = output.clone();
        owned.workers.push(
            std::thread::Builder::new()
                .name("synara-conpty-write".into())
                .spawn(move || write_input_loop(writer, receive, write_signals, write_output))?,
        );
        let (exit_tx, exit) = watch::channel(None);
        let worker_output = output.clone();
        let worker = std::thread::Builder::new()
            .name("synara-conpty-owner".into())
            .spawn(move || {
                let code = supervise(&mut owned, &worker_output);
                // Keep reading while ClosePseudoConsole drains its output pipe.
                owned.master.take();
                let deadline = Instant::now() + Duration::from_secs(2);
                while owned.workers.iter().any(|worker| !worker.is_finished())
                    && Instant::now() < deadline
                {
                    std::thread::sleep(Duration::from_millis(5));
                }
                for worker in owned.workers.drain(..) {
                    if worker.is_finished() {
                        if worker.join().is_err() {
                            record_error(&worker_output, "terminal I/O worker failed");
                        }
                    } else {
                        record_error(&worker_output, "terminal I/O cleanup exceeded its deadline");
                    }
                }
                // Completion includes handle cleanup, not just process exit.
                let _ = exit_tx.send(Some(code));
            })?;
        Ok(Self {
            input,
            signals,
            output,
            exit,
            worker: worker.thread().clone(),
        })
    }
    fn enqueue(&self, input: Input, reserved: usize) -> Result<(), RuntimeError> {
        enqueue(&self.input, &self.signals, input, reserved)?;
        self.worker.unpark();
        Ok(())
    }
    /// Trusted raw control bytes. Clipboard content must use paste().
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
        let reserved = paste.text().len() + 12;
        self.enqueue(Input::Paste(paste, decision), reserved)
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
    /// Cancellation never waits for a full queue or a child reading stdin.
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

fn enqueue(
    sender: &SyncSender<Message>,
    signals: &Arc<Signals>,
    input: Input,
    reserved: usize,
) -> Result<(), RuntimeError> {
    if signals.stop.load(Ordering::Acquire) {
        return Err(RuntimeError::Closed);
    }
    signals
        .queued
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
            n.checked_add(reserved).filter(|n| *n <= QUEUE_BYTES)
        })
        .map_err(|_| RuntimeError::Limit)?;
    let message = Message {
        input,
        reservation: Reservation {
            signals: signals.clone(),
            bytes: reserved,
        },
    };
    sender.try_send(message).map_err(|error| match error {
        mpsc::TrySendError::Full(_) => RuntimeError::Limit,
        mpsc::TrySendError::Disconnected(_) => RuntimeError::Closed,
    })
}

fn read_output_loop(
    mut reader: Box<dyn Read + Send>,
    input: SyncSender<Message>,
    signals: Arc<Signals>,
    output: Arc<Mutex<Output>>,
) {
    let mut buffer = [0u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => return,
            Ok(n) => {
                let replies = match output.lock() {
                    Ok(mut out) => out.screen.process(&buffer[..n]),
                    Err(_) => {
                        signals.stop.store(true, Ordering::Release);
                        return;
                    }
                };
                if !replies.is_empty() && !signals.stop.load(Ordering::Acquire) {
                    let size = replies.len();
                    if enqueue(&input, &signals, Input::Bytes(replies), size).is_err() {
                        record_error(&output, "terminal reply queue exceeded its bound");
                        signals.stop.store(true, Ordering::Release);
                    }
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => {
                if !signals.stop.load(Ordering::Acquire) {
                    record_error(&output, "terminal output pipe failed");
                    signals.stop.store(true, Ordering::Release);
                }
                return;
            }
        }
    }
}

fn write_input_loop(
    mut writer: Box<dyn Write + Send>,
    receive: Receiver<Message>,
    signals: Arc<Signals>,
    output: Arc<Mutex<Output>>,
) {
    while !signals.stop.load(Ordering::Acquire) {
        let message = match receive.recv_timeout(Duration::from_millis(10)) {
            Ok(message) => message,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        let Message { input, reservation } = message;
        let bytes = (|| -> Result<Vec<u8>, RuntimeError> {
            let out = output.lock().map_err(|_| RuntimeError::Closed)?;
            match input {
                Input::Bytes(bytes) => Ok(bytes),
                Input::Key(key, modifiers) => Ok(encode_terminal_key(
                    key,
                    modifiers,
                    out.screen.application_cursor(),
                )
                .unwrap_or_default()),
                Input::Paste(paste, decision) => paste.encode(decision, out.screen.bracketed_paste()),
            }
        })();
        // No screen mutex is held across a potentially blocked pipe write.
        let result = bytes.and_then(|bytes| {
            writer.write_all(&bytes)?;
            writer.flush()?;
            Ok(())
        });
        drop(reservation);
        if result.is_err() {
            if !signals.stop.load(Ordering::Acquire) {
                record_error(&output, "terminal input pipe failed");
                signals.stop.store(true, Ordering::Release);
            }
            return;
        }
    }
}

struct OwnedPty {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: Option<Box<dyn MasterPty + Send>>,
    signals: Arc<Signals>,
    workers: Vec<JoinHandle<()>>,
    reaped: bool,
}
impl Drop for OwnedPty {
    fn drop(&mut self) {
        self.signals.stop.store(true, Ordering::Release);
        if !self.reaped {
            let _ = self.child.kill();
            let deadline = Instant::now() + Duration::from_secs(1);
            while Instant::now() < deadline {
                match self.child.try_wait() {
                    Ok(Some(_)) | Err(_) => break,
                    Ok(None) => std::thread::sleep(Duration::from_millis(5)),
                }
            }
        }
    }
}

fn supervise(owned: &mut OwnedPty, output: &Mutex<Output>) -> u32 {
    let mut stopping = None;
    loop {
        match owned.child.try_wait() {
            Ok(Some(status)) => {
                owned.reaped = true;
                owned.signals.stop.store(true, Ordering::Release);
                return status.exit_code();
            }
            Err(_) => {
                record_error(output, "terminal process status unavailable");
                owned.signals.stop.store(true, Ordering::Release);
            }
            Ok(None) => {}
        }
        if owned.signals.stop.load(Ordering::Acquire) {
            let deadline = stopping.get_or_insert_with(|| {
                // Existing process-tree fallback is confined to this worker.
                // Job Object containment is a separate acceptance requirement.
                if let Some(pid) = owned.child.process_id() {
                    crate::process::signal_tree(pid, true);
                }
                let _ = owned.child.kill();
                Instant::now() + Duration::from_secs(1)
            });
            if Instant::now() >= *deadline {
                record_error(output, "terminal process cleanup exceeded its deadline");
                return 1;
            }
        } else if let Ok(mut resize) = owned.signals.resize.lock()
            && let Some((rows, columns)) = resize.take()
        {
            let resized = owned
                .master
                .as_ref()
                .ok_or(RuntimeError::Closed)
                .and_then(|master| master.resize(size(rows, columns)).map_err(pty_error))
                .and_then(|()| {
                    output
                        .lock()
                        .map_err(|_| RuntimeError::Closed)?
                        .screen
                        .resize(rows, columns)
                });
            if resized.is_err() {
                record_error(output, "terminal resize failed");
                owned.signals.stop.store(true, Ordering::Release);
            }
        }
        std::thread::park_timeout(Duration::from_millis(10));
    }
}

fn record_error(output: &Mutex<Output>, message: &str) {
    if let Ok(mut output) = output.lock()
        && output.error.is_none()
    {
        output.error = Some(message.to_owned());
    }
}
fn size(rows: u16, columns: u16) -> PtySize {
    PtySize {
        rows,
        cols: columns,
        pixel_width: 0,
        pixel_height: 0,
    }
}
fn pty_error(error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::Invalid(format!("terminal: {error}"))
}
