//! Bounded, cancellation-owned native command execution. No shell evaluation.
use crate::{ExecutionHost, LaunchSpec, LocalHost, RuntimeError};
use std::{path::Path, time::Duration};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    executable: &Path,
    args: Vec<String>,
    limit: usize,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, RuntimeError> {
    if cancel.is_cancelled() {
        return Err(RuntimeError::Closed);
    }
    if !executable.is_absolute() || !executable.is_file() {
        return Err(RuntimeError::Unsupported("Choose an installed, absolute helper executable in Device settings".into()));
    }
    let mut launch = LaunchSpec::new(executable);
    launch.args = args;
    // Native helpers must not inherit a project directory or its executable search path.
    let cwd = executable.parent().ok_or_else(|| RuntimeError::Invalid("helper has no parent directory".into()))?;
    let process = LocalHost.spawn(&launch, cwd).await?;
    let handle = process.handle;
    drop(process.stdin);
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(RuntimeError::Closed),
        result = tokio::time::timeout(Duration::from_secs(45), async {
            let read = async {
                let mut bytes = Vec::new();
                process.stdout.take((limit + 1) as u64).read_to_end(&mut bytes).await?;
                if bytes.len() > limit { return Err(RuntimeError::Limit); }
                Ok(bytes)
            };
            let stderr = async {
                // Drain bounded chunks, not an unbounded diagnostic buffer. Do not leak
                // device paths, serials or native output into persistent diagnostics.
                let mut reader = process.stderr;
                let mut buffer = [0_u8; 4096];
                while reader.read(&mut buffer).await? != 0 {}
                Ok::<_, RuntimeError>(())
            };
            let (bytes, (), status) = tokio::try_join!(read, stderr, handle.wait())?;
            if !status.success() {
                return Err(RuntimeError::Denied("Native helper failed. Check operating-system permissions and helper setup, then retry".into()));
            }
            Ok(bytes)
        }) => result.unwrap_or(Err(RuntimeError::Timeout)),
    };
    if result.is_err() {
        // Cancellation/timeout waits for the process owner to reap its process tree.
        let _ = handle.shutdown().await;
    }
    result
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn native_command_output_is_bounded_and_failure_is_not_success() {
        let cancel = CancellationToken::new();
        let bytes = run(Path::new("/bin/sh"), vec!["-c".into(), "printf ok".into()], 8, &cancel).await.unwrap();
        assert_eq!(bytes, b"ok");
        assert!(matches!(run(Path::new("/bin/sh"), vec!["-c".into(), "printf 123456789".into()], 4, &cancel).await, Err(RuntimeError::Limit)));
        assert!(run(Path::new("/bin/sh"), vec!["-c".into(), "exit 17".into()], 8, &cancel).await.is_err());
    }
    #[tokio::test]
    async fn cancelling_a_live_native_command_returns_without_waiting_for_its_sleep() {
        let cancel = CancellationToken::new();
        let signal = cancel.clone();
        tokio::spawn(async move { tokio::time::sleep(Duration::from_millis(60)).await; signal.cancel(); });
        let result = tokio::time::timeout(Duration::from_secs(10), run(Path::new("/bin/sh"), vec!["-c".into(), "sleep 60".into()], 32, &cancel)).await.unwrap();
        assert!(matches!(result, Err(RuntimeError::Closed)));
    }
}
