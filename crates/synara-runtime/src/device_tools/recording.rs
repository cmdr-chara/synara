//! User-owned Simulator recordings. A stopped recording is published only after
//! simctl finalizes it. Cancellation and dropped futures discard the private file.
use super::{DeviceAvailability, DeviceBackend, DeviceTools, ToolDevice};
use crate::{ExecutionHost, LaunchSpec, LocalHost, ProcessHandle, RuntimeError};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::sync::CancellationToken;

const RECORDING_LIMIT: u64 = 512 * 1024 * 1024;
const RECORDING_DURATION: Duration = Duration::from_secs(5 * 60);
const FINALIZE_TIMEOUT: Duration = Duration::from_secs(10);

impl DeviceTools {
    pub fn can_record_video(&self, device: &ToolDevice) -> bool {
        cfg!(target_os = "macos")
            && self.backend == DeviceBackend::AppleSimulator
            && device.availability == DeviceAvailability::Ready
            && self.address(device).is_ok()
    }

    /// Record a booted iOS Simulator to a new MOV file. `stop` finalizes the
    /// recording, while `cancel` discards it. Recording also stops after five
    /// minutes and is discarded if it exceeds 512 MiB. Existing files are never
    /// replaced, including files created while recording is in progress.
    pub async fn record_video(
        &self,
        device: &ToolDevice,
        destination: PathBuf,
        stop: &CancellationToken,
        cancel: &CancellationToken,
    ) -> Result<PathBuf, RuntimeError> {
        if cancel.is_cancelled() || stop.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        if !self.can_record_video(device) {
            return Err(RuntimeError::Unsupported(
                "Recording requires a booted iOS Simulator on macOS".into(),
            ));
        }
        let id = self.address(device)?;
        if !destination.is_absolute()
            || !destination
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("mov"))
        {
            return Err(RuntimeError::Invalid(
                "Choose an absolute destination with the .mov extension".into(),
            ));
        }
        match std::fs::symlink_metadata(&destination) {
            Ok(_) => return Err(RuntimeError::Conflict),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let parent = destination.parent().ok_or_else(|| {
            RuntimeError::Invalid("Recording destination has no parent directory".into())
        })?;
        // Use the destination filesystem so the final hard link publishes the
        // complete movie atomically without replacing a file or following a link.
        let scratch = tempfile::Builder::new()
            .prefix(".synara-recording-")
            .tempdir_in(parent)?;
        let movie = scratch.path().join("recording.mov");
        let movie_argument = movie.to_str().ok_or_else(|| {
            RuntimeError::Invalid("Recording destination is not a valid UTF-8 path".into())
        })?;
        if !self.executable.is_absolute() || !self.executable.is_file() {
            return Err(RuntimeError::Unsupported(
                "The Simulator helper is unavailable".into(),
            ));
        }
        let mut launch = LaunchSpec::new(&self.executable);
        launch.args = vec![
            "simctl".into(),
            "io".into(),
            id,
            "recordVideo".into(),
            "--codec=h264".into(),
            movie_argument.into(),
        ];
        launch.env.insert("LC_ALL".into(), "C".into());
        let cwd = self.executable.parent().ok_or_else(|| {
            RuntimeError::Invalid("Simulator helper has no parent directory".into())
        })?;
        let process = LocalHost.spawn(&launch, cwd).await?;
        let handle = process.handle;
        drop(process.stdin);
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(RuntimeError::Closed),
            result = tokio::time::timeout(RECORDING_DURATION + FINALIZE_TIMEOUT + Duration::from_secs(2), async {
                let (_, _, ()) = tokio::try_join!(
                    discard_output(process.stdout),
                    discard_output(process.stderr),
                    record_until_stopped(&handle, &movie, stop),
                )?;
                Ok::<_, RuntimeError>(())
            }) => result.unwrap_or(Err(RuntimeError::Timeout)),
        };
        if let Err(error) = result {
            // The process owner also handles future/drop cancellation. Explicit
            // cancellation waits for process-tree cleanup before deleting staging.
            let _ = handle.shutdown().await;
            return Err(error);
        }
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        let metadata = std::fs::symlink_metadata(&movie)?;
        if !metadata.file_type().is_file() || metadata.len() == 0 {
            return Err(RuntimeError::Invalid(
                "Simulator produced no recording".into(),
            ));
        }
        if metadata.len() > RECORDING_LIMIT {
            return Err(RuntimeError::Limit);
        }
        std::fs::File::open(&movie)?.sync_all()?;
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        std::fs::hard_link(&movie, &destination).map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                RuntimeError::Conflict
            } else {
                RuntimeError::Io(error)
            }
        })?;
        Ok(destination)
    }
}

async fn discard_output(mut reader: impl AsyncRead + Unpin) -> Result<(), RuntimeError> {
    // Keep native diagnostics out of persistent logs and bound memory use.
    let mut buffer = [0_u8; 4096];
    while reader.read(&mut buffer).await? != 0 {}
    Ok(())
}

async fn record_until_stopped(
    handle: &ProcessHandle,
    movie: &Path,
    stop: &CancellationToken,
) -> Result<(), RuntimeError> {
    let deadline = tokio::time::sleep(RECORDING_DURATION);
    tokio::pin!(deadline);
    let mut monitor = tokio::time::interval(Duration::from_millis(250));
    loop {
        tokio::select! {
            biased;
            status = handle.wait() => {
                if status?.success() { return Ok(()); }
                return Err(RuntimeError::Denied(
                    "Simulator recording failed. Check Xcode and simulator permissions, then retry".into(),
                ));
            }
            _ = stop.cancelled() => break,
            _ = &mut deadline => break,
            _ = monitor.tick() => {
                match tokio::fs::symlink_metadata(movie).await {
                    Ok(metadata) if !metadata.file_type().is_file() || metadata.len() > RECORDING_LIMIT => {
                        return Err(RuntimeError::Limit);
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
    }
    // simctl writes the MOV trailer on SIGINT. SIGTERM/KILL are cleanup paths,
    // never a successful finalization. The existing supervisor owns the group.
    interrupt_recording(handle)?;
    let status = tokio::time::timeout(FINALIZE_TIMEOUT, handle.wait())
        .await
        .map_err(|_| RuntimeError::Timeout)??;
    if !status.success() {
        return Err(RuntimeError::Denied(
            "Simulator could not finalize the recording".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn interrupt_recording(handle: &ProcessHandle) -> Result<(), RuntimeError> {
    use nix::{
        sys::signal::{Signal, killpg},
        unistd::Pid,
    };
    if handle.exit().is_some() {
        return Ok(());
    }
    let pid = i32::try_from(handle.pid()).map_err(|_| RuntimeError::Closed)?;
    if pid <= 0 {
        return Err(RuntimeError::Closed);
    }
    match killpg(Pid::from_raw(pid), Signal::SIGINT) {
        Ok(()) | Err(nix::errno::Errno::ESRCH) => Ok(()),
        Err(error) => Err(std::io::Error::from_raw_os_error(error as i32).into()),
    }
}

#[cfg(not(unix))]
fn interrupt_recording(_handle: &ProcessHandle) -> Result<(), RuntimeError> {
    Err(RuntimeError::Unsupported(
        "Simulator recording requires macOS".into(),
    ))
}
