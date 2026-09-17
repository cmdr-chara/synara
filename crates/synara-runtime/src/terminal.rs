use crate::{BoundedBytes, LaunchSpec, RuntimeError, base_environment, process::signal_tree};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{io::{Read, Write}, path::Path, sync::{Arc, Mutex}};
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct TerminalSnapshot {
    pub text: String,
    pub raw_tail: Vec<u8>,
    pub truncated: bool,
    pub exit_code: Option<u32>,
    pub error: Option<String>,
    pub revision: u64,
}
struct TerminalOutput { tail: BoundedBytes, parser: vt100::Parser, revision: u64, error: Option<String> }

pub struct NativeTerminal {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn portable_pty::ChildKiller + Send + Sync>>,
    output: Arc<Mutex<TerminalOutput>>,
    exit: watch::Receiver<Option<u32>>,
    pid: Option<u32>,
}
impl NativeTerminal {
    /// Call on a worker thread. PTY creation and platform process setup can block.
    pub fn spawn(launch: &LaunchSpec, cwd: &Path, rows: u16, columns: u16) -> Result<Self, RuntimeError> {
        launch.validate()?;
        if rows == 0 || columns == 0 || rows > 500 || columns > 1000 || !cwd.is_absolute() { return Err(RuntimeError::Invalid("invalid terminal size or directory".into())); }
        let system = native_pty_system();
        let pair = system.openpty(PtySize { rows, cols: columns, pixel_width: 0, pixel_height: 0 }).map_err(pty_error)?;
        let mut command = CommandBuilder::new(&launch.command);
        command.args(&launch.args); command.cwd(cwd); command.env_clear();
        for (key, value) in base_environment() { command.env(key, value); }
        command.env("TERM", "xterm-256color");
        for (key, value) in &launch.env { command.env(key, value); }
        // Acquire both I/O handles before spawning, so an I/O setup failure cannot leak a child.
        let mut reader = pair.master.try_clone_reader().map_err(pty_error)?;
        let writer = pair.master.take_writer().map_err(pty_error)?;
        let mut child = pair.slave.spawn_command(command).map_err(pty_error)?;
        let pid = child.process_id();
        let killer = child.clone_killer();
        drop(pair.slave);
        let output = Arc::new(Mutex::new(TerminalOutput { tail: BoundedBytes::new(1024 * 1024), parser: vt100::Parser::new(rows, columns, 5_000), revision: 0, error: None }));
        let read_output = output.clone();
        std::thread::spawn(move || {
            let mut buffer = [0u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(n) => {
                        let Ok(mut out) = read_output.lock() else { break };
                        out.tail.push(&buffer[..n]); out.parser.process(&buffer[..n]); out.revision += 1;
                    }
                    Err(error) => {
                        #[cfg(unix)]
                        if error.raw_os_error() == Some(5) { break; }
                        if error.kind() == std::io::ErrorKind::Interrupted { continue; }
                        if let Ok(mut out) = read_output.lock() { out.error = Some(error.to_string()); out.revision += 1; }
                        break;
                    }
                }
            }
        });
        let (exit_tx, exit) = watch::channel(None);
        let wait_output = output.clone();
        std::thread::spawn(move || {
            let result = child.wait();
            // A shell can exit while descendants still hold the PTY open.
            // Clean up its process group immediately, not when historical output is removed.
            #[cfg(unix)]
            if let Some(pid) = pid { signal_tree(pid, true); }
            match result {
                Ok(status) => { let _ = exit_tx.send(Some(status.exit_code())); }
                Err(error) => {
                    if let Ok(mut out) = wait_output.lock() { out.error = Some(error.to_string()); out.revision += 1; }
                    let _ = exit_tx.send(Some(1));
                }
            }
        });
        Ok(Self { master: Mutex::new(pair.master), writer: Mutex::new(writer), killer: Mutex::new(killer), output, exit, pid })
    }
    pub fn input(&self, bytes: &[u8]) -> Result<(), RuntimeError> {
        if bytes.len() > 1024 * 1024 { return Err(RuntimeError::Limit); }
        let mut writer = self.writer.lock().map_err(|_| RuntimeError::Closed)?;
        writer.write_all(bytes)?; writer.flush()?; Ok(())
    }
    pub fn resize(&self, rows: u16, columns: u16) -> Result<(), RuntimeError> {
        if rows == 0 || columns == 0 || rows > 500 || columns > 1000 { return Err(RuntimeError::Invalid("invalid terminal size".into())); }
        self.master.lock().map_err(|_| RuntimeError::Closed)?.resize(PtySize { rows, cols: columns, pixel_width: 0, pixel_height: 0 }).map_err(pty_error)?;
        let mut output = self.output.lock().map_err(|_| RuntimeError::Closed)?;
        output.parser.set_size(rows, columns); output.revision += 1; Ok(())
    }
    pub fn snapshot(&self) -> Result<TerminalSnapshot, RuntimeError> {
        let output = self.output.lock().map_err(|_| RuntimeError::Closed)?;
        Ok(TerminalSnapshot { text: output.parser.screen().contents(), raw_tail: output.tail.bytes(), truncated: output.tail.truncated(), exit_code: *self.exit.borrow(), error: output.error.clone(), revision: output.revision })
    }
    pub fn kill(&self) -> Result<(), RuntimeError> {
        if self.exit.borrow().is_none() {
            if let Some(pid) = self.pid { signal_tree(pid, true); }
            self.killer.lock().map_err(|_| RuntimeError::Closed)?.kill()?;
        }
        Ok(())
    }
    pub async fn wait(&self) -> Result<u32, RuntimeError> {
        let mut exit = self.exit.clone();
        loop {
            if let Some(code) = *exit.borrow() { return Ok(code); }
            exit.changed().await.map_err(|_| RuntimeError::Closed)?;
        }
    }
}
impl Drop for NativeTerminal { fn drop(&mut self) { let _ = self.kill(); } }
fn pty_error(error: impl std::fmt::Display) -> RuntimeError { RuntimeError::Invalid(format!("terminal: {error}")) }

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[tokio::test]
    async fn terminal_executes_and_retains_historical_output() {
        let mut launch = LaunchSpec::new("sh"); launch.args = vec!["-c".into(), "printf terminal-ok".into()];
        let terminal = NativeTerminal::spawn(&launch, Path::new("/tmp"), 24, 80).unwrap();
        assert_eq!(terminal.wait().await.unwrap(), 0);
        for _ in 0..100 {
            if terminal.snapshot().unwrap().text.contains("terminal-ok") { return; }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!("terminal output was not retained");
    }
}
