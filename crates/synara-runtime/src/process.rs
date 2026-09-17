use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    process::{Child, Command},
    sync::watch,
};
use tokio_util::sync::CancellationToken;

pub type ProcessReader = Box<dyn AsyncRead + Unpin + Send>;
pub type ProcessWriter = Box<dyn AsyncWrite + Unpin + Send>;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProcessExit {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub error: Option<String>,
}
impl ProcessExit {
    pub fn success(&self) -> bool {
        self.code == Some(0) && self.error.is_none()
    }
}

pub struct SpawnedProcess {
    pub stdin: ProcessWriter,
    pub stdout: ProcessReader,
    pub stderr: ProcessReader,
    pub handle: ProcessHandle,
}

#[derive(Clone)]
pub struct ProcessHandle(Arc<ProcessOwner>);
struct ProcessOwner {
    pid: u32,
    stop: CancellationToken,
    exit: watch::Receiver<Option<ProcessExit>>,
}
impl Drop for ProcessOwner {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}
impl ProcessHandle {
    pub fn pid(&self) -> u32 {
        self.0.pid
    }
    pub fn exit(&self) -> Option<ProcessExit> {
        self.0.exit.borrow().clone()
    }
    pub async fn wait(&self) -> Result<ProcessExit, RuntimeError> {
        let mut rx = self.0.exit.clone();
        loop {
            if let Some(exit) = rx.borrow().clone() {
                return Ok(exit);
            }
            rx.changed().await.map_err(|_| RuntimeError::Closed)?;
        }
    }
    pub fn request_stop(&self) {
        self.0.stop.cancel();
    }
    pub async fn shutdown(&self) -> Result<ProcessExit, RuntimeError> {
        self.request_stop();
        tokio::time::timeout(Duration::from_secs(8), self.wait())
            .await
            .map_err(|_| RuntimeError::Timeout)?
    }
}

/// Configure ownership before spawning, so cleanup never depends on the GUI remaining alive.
pub(crate) fn spawn_owned(mut command: Command) -> Result<SpawnedProcess, RuntimeError> {
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.as_std_mut().process_group(0);
    }
    let mut child = command.spawn()?;
    let pid = child.id().ok_or(RuntimeError::Closed)?;
    let guard = ProcessTreeGuard { pid };
    let stdin = child.stdin.take().ok_or(RuntimeError::Closed)?;
    let stdout = child.stdout.take().ok_or(RuntimeError::Closed)?;
    let stderr = child.stderr.take().ok_or(RuntimeError::Closed)?;
    let stop = CancellationToken::new();
    let (exit_tx, exit_rx) = watch::channel(None);
    let handle = ProcessHandle(Arc::new(ProcessOwner {
        pid,
        stop: stop.clone(),
        exit: exit_rx,
    }));
    tokio::spawn(async move {
        let _guard = guard;
        let status = tokio::select! {
            status = child.wait() => status,
            () = stop.cancelled() => stop_child(&mut child, pid).await,
        };
        let exit = match status {
            Ok(status) => {
                #[cfg(unix)]
                let signal = {
                    use std::os::unix::process::ExitStatusExt;
                    status.signal()
                };
                #[cfg(not(unix))]
                let signal = None;
                ProcessExit {
                    code: status.code(),
                    signal,
                    error: None,
                }
            }
            Err(error) => ProcessExit {
                error: Some(error.to_string()),
                ..ProcessExit::default()
            },
        };
        let _ = exit_tx.send(Some(exit));
    });
    Ok(SpawnedProcess {
        stdin: Box::new(stdin),
        stdout: Box::new(stdout),
        stderr: Box::new(stderr),
        handle,
    })
}

async fn stop_child(child: &mut Child, pid: u32) -> std::io::Result<std::process::ExitStatus> {
    signal_tree(pid, false);
    match tokio::time::timeout(Duration::from_secs(2), child.wait()).await {
        Ok(status) => status,
        Err(_) => {
            signal_tree(pid, true);
            let _ = child.start_kill();
            child.wait().await
        }
    }
}

/// Drop is also reached when the executor aborts the supervising task.
struct ProcessTreeGuard {
    pid: u32,
}
impl Drop for ProcessTreeGuard {
    fn drop(&mut self) {
        signal_tree(self.pid, true);
    }
}

pub(crate) fn signal_tree(pid: u32, force: bool) {
    #[cfg(unix)]
    {
        use nix::{
            sys::signal::{Signal, killpg},
            unistd::Pid,
        };
        if let Ok(raw) = i32::try_from(pid)
            && raw > 0
        {
            let _ = killpg(
                Pid::from_raw(raw),
                if force {
                    Signal::SIGKILL
                } else {
                    Signal::SIGTERM
                },
            );
        }
    }
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("taskkill");
        command.args(["/PID", &pid.to_string(), "/T"]);
        if force {
            command.arg("/F");
        }
        let _ = command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (pid, force);
    }
}
