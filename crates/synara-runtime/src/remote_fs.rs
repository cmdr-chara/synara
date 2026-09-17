//! Remote workspace filesystem over a pinned SSH execution boundary.
//!
//! The remote helper reuses WorkspaceFs, so traversal and guarded-write rules are
//! identical to local workspaces. Every request binds to the canonical root seen
//! during enrollment. If a write acknowledgement is lost, Synara reports an
//! unknown outcome instead of retrying a possibly completed write.
use crate::{
    ExecutionHost, FileEntry, FileSnapshot, FileVersion, LaunchSpec, PinnedSshHost, RuntimeError,
    WorkspaceFs,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const RESPONSE_LIMIT: usize = 12 * 1024 * 1024;
const DIAGNOSTIC_LIMIT: usize = 64 * 1024;
const REQUEST_LIMIT: usize = 10 * 1024 * 1024;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
enum RemoteFsOperation {
    Identity,
    Entries { path: PathBuf },
    Read { path: PathBuf },
    Write {
        path: PathBuf,
        text: String,
        expected: Option<FileVersion>,
        bom: bool,
        create_new: bool,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RemoteFsRequest {
    expected_root: Option<String>,
    operation: RemoteFsOperation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RemoteFsResponse {
    root: String,
    result: Result<RemoteFsValue, RemoteFsFailure>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "value", content = "data", rename_all = "snake_case")]
enum RemoteFsValue {
    Identity,
    Entries(Vec<FileEntry>),
    Snapshot(FileSnapshot),
    Version(FileVersion),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RemoteFsFailure {
    kind: RemoteFsFailureKind,
    message: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RemoteFsFailureKind {
    Invalid,
    Denied,
    Conflict,
    Limit,
    Unsupported,
    Io,
    Closed,
}

impl RemoteFsFailure {
    fn from_runtime(error: RuntimeError) -> Self {
        let kind = match error {
            RuntimeError::Invalid(_) => RemoteFsFailureKind::Invalid,
            RuntimeError::Denied(_) => RemoteFsFailureKind::Denied,
            RuntimeError::Conflict => RemoteFsFailureKind::Conflict,
            RuntimeError::Limit => RemoteFsFailureKind::Limit,
            RuntimeError::Unsupported(_) => RemoteFsFailureKind::Unsupported,
            RuntimeError::Io(_) => RemoteFsFailureKind::Io,
            RuntimeError::Closed | RuntimeError::Timeout | RuntimeError::WriteOutcomeUnknown => {
                RemoteFsFailureKind::Closed
            }
        };
        Self {
            kind,
            message: error.to_string(),
        }
    }

    fn into_runtime(self) -> RuntimeError {
        match self.kind {
            RemoteFsFailureKind::Invalid => RuntimeError::Invalid(self.message),
            RemoteFsFailureKind::Denied => RuntimeError::Denied(self.message),
            RemoteFsFailureKind::Conflict => RuntimeError::Conflict,
            RemoteFsFailureKind::Limit => RuntimeError::Limit,
            RemoteFsFailureKind::Unsupported => RuntimeError::Unsupported(self.message),
            RemoteFsFailureKind::Io => {
                RuntimeError::Invalid(format!("remote filesystem I/O failed: {}", self.message))
            }
            RemoteFsFailureKind::Closed => RuntimeError::Closed,
        }
    }
}

/// A workspace-bound filesystem whose operations execute only on the approved SSH host.
#[derive(Clone, Debug)]
pub struct RemoteWorkspaceFs {
    host: PinnedSshHost,
    root: PathBuf,
    helper: PathBuf,
    root_identity: String,
}

impl RemoteWorkspaceFs {
    pub async fn connect(
        host: PinnedSshHost,
        root: impl Into<PathBuf>,
        helper: impl Into<PathBuf>,
    ) -> Result<Self, RuntimeError> {
        let root = root.into();
        check_remote_root(&root)?;
        let helper = helper.into();
        if helper.as_os_str().is_empty() {
            return Err(RuntimeError::Invalid(
                "remote filesystem helper command is empty".into(),
            ));
        }
        let candidate = Self {
            host,
            root,
            helper,
            root_identity: String::new(),
        };
        let response = candidate
            .exchange(RemoteFsRequest {
                expected_root: None,
                operation: RemoteFsOperation::Identity,
            })
            .await?;
        response.result.map_err(RemoteFsFailure::into_runtime)?;
        if response.root.is_empty() || !response.root.starts_with('/') || response.root.contains('\0')
        {
            return Err(RuntimeError::Denied(
                "remote helper returned an invalid workspace identity".into(),
            ));
        }
        Ok(Self {
            root_identity: response.root,
            ..candidate
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn root_identity(&self) -> &str {
        &self.root_identity
    }

    pub fn relative(&self, path: &Path) -> Result<PathBuf, RuntimeError> {
        let relative = if path.is_absolute() {
            path.strip_prefix(&self.root)
                .map_err(|_| RuntimeError::Denied("path is outside this remote workspace".into()))?
        } else {
            path
        };
        validate_remote_relative(relative)?;
        Ok(relative.to_owned())
    }

    pub async fn entries(&self, path: &Path) -> Result<Vec<FileEntry>, RuntimeError> {
        let path = self.relative(path)?;
        match self
            .operation(RemoteFsOperation::Entries { path }, false)
            .await?
        {
            RemoteFsValue::Entries(entries) => Ok(entries),
            _ => Err(RuntimeError::Invalid(
                "remote helper returned the wrong response type".into(),
            )),
        }
    }

    pub async fn read(&self, path: &Path) -> Result<FileSnapshot, RuntimeError> {
        let path = self.relative(path)?;
        match self.operation(RemoteFsOperation::Read { path }, false).await? {
            RemoteFsValue::Snapshot(snapshot) => Ok(snapshot),
            _ => Err(RuntimeError::Invalid(
                "remote helper returned the wrong response type".into(),
            )),
        }
    }

    pub async fn write(
        &self,
        path: &Path,
        text: &str,
        expected: Option<&FileVersion>,
        bom: bool,
    ) -> Result<FileVersion, RuntimeError> {
        self.write_impl(path, text, expected, bom, false).await
    }

    pub async fn write_new(
        &self,
        path: &Path,
        text: &str,
        bom: bool,
    ) -> Result<FileVersion, RuntimeError> {
        self.write_impl(path, text, None, bom, true).await
    }

    async fn write_impl(
        &self,
        path: &Path,
        text: &str,
        expected: Option<&FileVersion>,
        bom: bool,
        create_new: bool,
    ) -> Result<FileVersion, RuntimeError> {
        if text.len().saturating_add(if bom { 3 } else { 0 }) > 8 * 1024 * 1024 {
            return Err(RuntimeError::Limit);
        }
        let path = self.relative(path)?;
        let operation = RemoteFsOperation::Write {
            path,
            text: text.to_owned(),
            expected: expected.cloned(),
            bom,
            create_new,
        };
        match self.operation(operation, true).await? {
            RemoteFsValue::Version(version) => Ok(version),
            _ => Err(RuntimeError::WriteOutcomeUnknown),
        }
    }

    async fn operation(
        &self,
        operation: RemoteFsOperation,
        write: bool,
    ) -> Result<RemoteFsValue, RuntimeError> {
        let request = RemoteFsRequest {
            expected_root: Some(self.root_identity.clone()),
            operation,
        };
        let response = match self.exchange(request).await {
            Ok(response) => response,
            Err(_) if write => return Err(RuntimeError::WriteOutcomeUnknown),
            Err(error) => return Err(error),
        };
        if response.root != self.root_identity {
            return Err(if write {
                RuntimeError::WriteOutcomeUnknown
            } else {
                RuntimeError::Denied("remote workspace identity changed".into())
            });
        }
        response.result.map_err(RemoteFsFailure::into_runtime)
    }

    async fn exchange(&self, request: RemoteFsRequest) -> Result<RemoteFsResponse, RuntimeError> {
        let mut payload = serde_json::to_vec(&request)
            .map_err(|_| RuntimeError::Invalid("could not encode remote filesystem request".into()))?;
        if payload.len() > REQUEST_LIMIT {
            return Err(RuntimeError::Limit);
        }
        payload.push(b'\n');
        let mut launch = LaunchSpec::new(&self.helper);
        launch.args = vec!["--root".into(), self.root.to_string_lossy().into_owned()];
        let process = self.host.spawn(&launch, &self.root).await?;
        let handle = process.handle.clone();
        let operation = async move {
            let mut stdin = process.stdin;
            stdin.write_all(&payload).await?;
            stdin.shutdown().await?;
            drop(stdin);

            let mut stdout = process.stdout.take(RESPONSE_LIMIT as u64 + 1);
            let mut stderr = process.stderr.take(DIAGNOSTIC_LIMIT as u64 + 1);
            let mut response = Vec::new();
            let mut diagnostic = Vec::new();
            let (out, err, exit) = tokio::join!(
                stdout.read_to_end(&mut response),
                stderr.read_to_end(&mut diagnostic),
                process.handle.wait()
            );
            out?;
            err?;
            let exit = exit?;
            if response.len() > RESPONSE_LIMIT || diagnostic.len() > DIAGNOSTIC_LIMIT {
                return Err(RuntimeError::Limit);
            }
            if !exit.success() {
                let message = String::from_utf8_lossy(&diagnostic);
                return Err(RuntimeError::Invalid(format!(
                    "remote filesystem helper exited unsuccessfully: {}",
                    message.trim()
                )));
            }
            serde_json::from_slice::<RemoteFsResponse>(&response).map_err(|_| {
                RuntimeError::Invalid("remote filesystem helper returned malformed data".into())
            })
        };
        match tokio::time::timeout(OPERATION_TIMEOUT, operation).await {
            Ok(result) => result,
            Err(_) => {
                let _ = handle.shutdown().await;
                Err(RuntimeError::Timeout)
            }
        }
    }
}

fn check_remote_root(root: &Path) -> Result<(), RuntimeError> {
    let root = root
        .to_str()
        .ok_or_else(|| RuntimeError::Invalid("remote workspace root must be UTF-8".into()))?;
    if !root.starts_with('/') || root.contains('\0') {
        return Err(RuntimeError::Invalid(
            "remote workspace root must be an absolute POSIX path".into(),
        ));
    }
    Ok(())
}

fn validate_remote_relative(path: &Path) -> Result<(), RuntimeError> {
    if path.is_absolute() {
        return Err(RuntimeError::Denied(
            "remote workspace path must be relative".into(),
        ));
    }
    for component in path.components() {
        if !matches!(component, Component::Normal(_) | Component::CurDir) {
            return Err(RuntimeError::Denied(
                "remote workspace path contains traversal".into(),
            ));
        }
    }
    Ok(())
}

/// Entrypoint for the small helper installed on a remote development host.
pub fn remote_fs_helper_main() -> Result<(), RuntimeError> {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(std::ffi::OsStr::new("--root")) {
        return Err(RuntimeError::Invalid(
            "usage: synara-remote-fs --root /absolute/workspace".into(),
        ));
    }
    let root = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| RuntimeError::Invalid("missing remote workspace root".into()))?;
    if args.next().is_some() {
        return Err(RuntimeError::Invalid(
            "unexpected remote filesystem helper arguments".into(),
        ));
    }
    check_remote_root(&root)?;
    let fs = WorkspaceFs::open(&root)?;
    let root_identity = fs
        .root()
        .to_str()
        .ok_or_else(|| RuntimeError::Unsupported("remote workspace path is not UTF-8".into()))?
        .to_owned();

    let mut line = String::new();
    let read = std::io::stdin().lock().take(REQUEST_LIMIT as u64 + 2);
    let mut read = std::io::BufReader::new(read);
    if read.read_line(&mut line)? == 0 || line.len() > REQUEST_LIMIT + 1 {
        return Err(RuntimeError::Limit);
    }
    let request: RemoteFsRequest = serde_json::from_str(line.trim_end()).map_err(|_| {
        RuntimeError::Invalid("malformed remote filesystem helper request".into())
    })?;
    let result = if request
        .expected_root
        .as_ref()
        .is_some_and(|expected| expected != &root_identity)
    {
        Err(RuntimeError::Denied(
            "remote workspace identity changed since enrollment".into(),
        ))
    } else {
        execute_helper_operation(&fs, request.operation)
    };
    let response = RemoteFsResponse {
        root: root_identity,
        result: result.map_err(RemoteFsFailure::from_runtime),
    };
    let encoded = serde_json::to_vec(&response)
        .map_err(|_| RuntimeError::Invalid("could not encode remote filesystem response".into()))?;
    if encoded.len() > RESPONSE_LIMIT {
        return Err(RuntimeError::Limit);
    }
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&encoded)?;
    stdout.write_all(b"\n")?;
    stdout.flush()?;
    Ok(())
}

fn execute_helper_operation(
    fs: &WorkspaceFs,
    operation: RemoteFsOperation,
) -> Result<RemoteFsValue, RuntimeError> {
    match operation {
        RemoteFsOperation::Identity => Ok(RemoteFsValue::Identity),
        RemoteFsOperation::Entries { path } => {
            Ok(RemoteFsValue::Entries(fs.entries(&path)?))
        }
        RemoteFsOperation::Read { path } => Ok(RemoteFsValue::Snapshot(fs.read(&path)?)),
        RemoteFsOperation::Write {
            path,
            text,
            expected,
            bom,
            create_new,
        } => {
            let version = if create_new {
                fs.write_new(&path, &text, bom)?
            } else {
                fs.write(&path, &text, expected.as_ref(), bom)?
            };
            Ok(RemoteFsValue::Version(version))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_relative_paths_reject_escape_and_absolute_values() {
        assert!(validate_remote_relative(Path::new("src/lib.rs")).is_ok());
        assert!(validate_remote_relative(Path::new("../secret")).is_err());
        assert!(validate_remote_relative(Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn helper_failure_round_trip_keeps_security_categories() {
        for error in [
            RuntimeError::Denied("no".into()),
            RuntimeError::Conflict,
            RuntimeError::Limit,
            RuntimeError::Unsupported("binary".into()),
        ] {
            let failure = RemoteFsFailure::from_runtime(error);
            assert!(matches!(
                failure.into_runtime(),
                RuntimeError::Denied(_)
                    | RuntimeError::Conflict
                    | RuntimeError::Limit
                    | RuntimeError::Unsupported(_)
            ));
        }
    }
}
