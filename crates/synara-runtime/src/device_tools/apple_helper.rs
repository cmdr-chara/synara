//! Client for Synara's macOS CoreSimulator helper.
//!
//! The helper is intentionally user-selected, like adb. Rust owns validation,
//! cancellation and result bounds; the helper owns the private CoreSimulator/HID
//! and accessibility calls that simctl does not expose.

use crate::{ExecutionHost, LaunchSpec, LocalHost, RuntimeError};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::Path, time::Duration};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio_util::sync::CancellationToken;

const MAX_RPC_LINE: usize = 4 * 1024 * 1024;
const MAX_RPC_LINES: usize = 256;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct AttachResult {
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub point_width: f64,
    pub point_height: f64,
    pub capabilities: Capabilities,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct Capabilities {
    #[serde(default)]
    pub input: bool,
    #[serde(default)]
    pub accessibility: bool,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    message: String,
}

async fn write_request<W: AsyncWrite + Unpin>(
    writer: &mut W,
    id: u64,
    method: &str,
    params: Value,
) -> Result<(), RuntimeError> {
    let mut line = serde_json::to_vec(&json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params,
    }))
    .map_err(|_| RuntimeError::Invalid("device helper request is not encodable".into()))?;
    if line.len() > MAX_RPC_LINE {
        return Err(RuntimeError::Limit);
    }
    line.push(b'\n');
    writer.write_all(&line).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_response<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    id: u64,
    cancel: &CancellationToken,
) -> Result<Value, RuntimeError> {
    let mut seen = 0usize;
    let mut total = 0usize;
    loop {
        if seen >= MAX_RPC_LINES {
            return Err(RuntimeError::Limit);
        }
        let mut line = String::new();
        let count = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(RuntimeError::Closed),
            read = reader.read_line(&mut line) => read?,
        };
        if count == 0 {
            return Err(RuntimeError::Closed);
        }
        seen += 1;
        total = total.checked_add(count).ok_or(RuntimeError::Limit)?;
        if total > MAX_RPC_LINE || line.len() > MAX_RPC_LINE {
            return Err(RuntimeError::Limit);
        }
        let value: Value = serde_json::from_str(&line)
            .map_err(|_| RuntimeError::Invalid("device helper returned invalid JSON-RPC".into()))?;
        if value.get("id").and_then(Value::as_u64) != Some(id) {
            continue;
        }
        if let Some(error) = value.get("error") {
            let parsed: RpcError = serde_json::from_value(error.clone())
                .map_err(|_| RuntimeError::Denied("device helper rejected the request".into()))?;
            let message = parsed.message.trim();
            let safe = if message.is_empty()
                || message.len() > 512
                || message.chars().any(char::is_control)
            {
                "device helper rejected the request".to_owned()
            } else {
                message.to_owned()
            };
            return Err(RuntimeError::Denied(safe));
        }
        return value
            .get("result")
            .cloned()
            .ok_or_else(|| RuntimeError::Invalid("device helper response has no result".into()));
    }
}

fn validate_helper(executable: &Path) -> Result<(), RuntimeError> {
    if !cfg!(target_os = "macos") {
        return Err(RuntimeError::Unsupported(
            "Apple Simulator input requires macOS".into(),
        ));
    }
    if !executable.is_absolute() {
        return Err(RuntimeError::Invalid(
            "Apple device helper must be an absolute executable path".into(),
        ));
    }
    let metadata = std::fs::symlink_metadata(executable)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(RuntimeError::Denied(
            "Apple device helper must be a regular non-symlink file".into(),
        ));
    }
    Ok(())
}

pub(super) async fn invoke(
    executable: &Path,
    udid: &str,
    method: &str,
    params: Value,
    cancel: &CancellationToken,
) -> Result<(AttachResult, Value), RuntimeError> {
    validate_helper(executable)?;
    if cancel.is_cancelled() {
        return Err(RuntimeError::Closed);
    }
    let cwd = executable
        .parent()
        .ok_or_else(|| RuntimeError::Invalid("device helper has no parent directory".into()))?;
    let mut launch = LaunchSpec::new(executable);
    launch.env.insert("LC_ALL".into(), "C".into());
    let process = LocalHost.spawn(&launch, cwd).await?;
    let handle = process.handle.clone();
    let mut stdin = process.stdin;
    let mut stdout = BufReader::new(process.stdout);
    let mut stderr = process.stderr;
    let stderr_drain = tokio::spawn(async move {
        let mut buffer = [0_u8; 4096];
        loop {
            match stderr.read(&mut buffer).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    });

    let transaction = async {
        write_request(&mut stdin, 1, "attach", json!({ "udid": udid })).await?;
        let attached: AttachResult = serde_json::from_value(
            read_response(&mut stdout, 1, cancel).await?,
        )
        .map_err(|_| RuntimeError::Invalid("device helper returned invalid attach geometry".into()))?;
        if attached.pixel_width == 0
            || attached.pixel_height == 0
            || !attached.point_width.is_finite()
            || !attached.point_height.is_finite()
            || attached.point_width <= 0.0
            || attached.point_height <= 0.0
            || attached.point_width > 20_000.0
            || attached.point_height > 20_000.0
        {
            return Err(RuntimeError::Invalid(
                "device helper returned invalid display geometry".into(),
            ));
        }
        write_request(&mut stdin, 2, method, params).await?;
        let result = read_response(&mut stdout, 2, cancel).await?;
        Ok::<_, RuntimeError>((attached, result))
    };

    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(RuntimeError::Closed),
        result = tokio::time::timeout(Duration::from_secs(45), transaction) => {
            result.unwrap_or(Err(RuntimeError::Timeout))
        }
    };

    drop(stdin);
    let _ = handle.shutdown().await;
    let _ = tokio::time::timeout(Duration::from_secs(1), stderr_drain).await;
    result
}

pub(super) async fn probe(
    executable: &Path,
    udid: &str,
    cancel: &CancellationToken,
) -> Result<AttachResult, RuntimeError> {
    invoke(executable, udid, "ping", json!({}), cancel)
        .await
        .map(|(attached, _)| attached)
}
