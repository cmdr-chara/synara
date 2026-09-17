//! Interactive remote PTY built by placing the pinned OpenSSH client inside
//! Synara's already-owned native PTY. Resize and input therefore use the same
//! terminal parser and lifecycle code as local shells.
use crate::{
    ExecutionHost, LaunchSpec, NativeTerminal, PasteDecision, PinnedSshHost, PreparedPaste, RuntimeError,
    TerminalKey, TerminalModifiers, TerminalRenderSnapshot, TerminalSnapshot,
};
use std::path::Path;
use uuid::Uuid;

pub struct RemoteTerminal {
    inner: NativeTerminal,
    session_id: Uuid,
    host_label: String,
}

impl RemoteTerminal {
    pub fn spawn(
        host: &PinnedSshHost,
        launch: &LaunchSpec,
        cwd: &Path,
        rows: u16,
        columns: u16,
    ) -> Result<Self, RuntimeError> {
        let ssh = host.pty_command(launch, cwd)?;
        let local_cwd = std::env::current_dir()?;
        let inner = NativeTerminal::spawn(&ssh, &local_cwd, rows, columns)?;
        Ok(Self {
            inner,
            session_id: Uuid::new_v4(),
            host_label: host.label(),
        })
    }

    pub fn session_id(&self) -> Uuid {
        self.session_id
    }

    pub fn host_label(&self) -> &str {
        &self.host_label
    }

    pub fn disconnect_warning(&self) -> &'static str {
        "Disconnecting closes Synara's SSH client. If the network is already lost, remote descendants may continue until the server detects the closed session."
    }

    pub fn input(&self, bytes: &[u8]) -> Result<(), RuntimeError> {
        self.inner.input(bytes)
    }

    pub fn text(&self, text: &str) -> Result<(), RuntimeError> {
        self.inner.text(text)
    }

    pub fn key(
        &self,
        key: TerminalKey,
        modifiers: TerminalModifiers,
    ) -> Result<(), RuntimeError> {
        self.inner.key(key, modifiers)
    }

    pub fn paste(
        &self,
        paste: PreparedPaste,
        decision: PasteDecision,
    ) -> Result<(), RuntimeError> {
        self.inner.paste(paste, decision)
    }

    pub fn resize(&self, rows: u16, columns: u16) -> Result<(), RuntimeError> {
        self.inner.resize(rows, columns)
    }

    pub fn scrollback(&self, offset: usize) -> Result<(), RuntimeError> {
        self.inner.scrollback(offset)
    }

    pub fn snapshot(&self) -> Result<TerminalSnapshot, RuntimeError> {
        self.inner.snapshot()
    }

    pub fn render_snapshot(&self) -> Result<TerminalRenderSnapshot, RuntimeError> {
        self.inner.render_snapshot()
    }

    pub fn kill(&self) -> Result<(), RuntimeError> {
        self.inner.kill()
    }

    pub async fn wait(&self) -> Result<u32, RuntimeError> {
        self.inner.wait().await
    }

    pub async fn shutdown(&self) -> Result<u32, RuntimeError> {
        self.inner.shutdown().await
    }
}
