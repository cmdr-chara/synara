//! Upstream browser-host wire protocol: JSON-RPC 2.0 over a unix socket with
//! 4-byte little-endian length-prefixed frames (browserUsePipeServer.ts /
//! browserHostRpcClient.ts in the TypeScript implementation).

use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const MAX_FRAME_BYTES: usize = 12 * 1024 * 1024;
pub const PROTOCOL_VERSION: u64 = 1;
pub const HOST_TYPE: &str = "synara-browser-host";

pub fn encode_frame(message: &Value) -> Result<Vec<u8>, String> {
    let payload = serde_json::to_vec(message).map_err(|e| e.to_string())?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err("browser host frame exceeds 12 MiB".into());
    }
    let mut out = Vec::with_capacity(4 + payload.len());
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Read one length-prefixed frame from the stream. Returns None on clean EOF
/// before any byte is read.
pub async fn read_frame<R: AsyncReadExt + Unpin>(reader: &mut R) -> Result<Option<Value>, String> {
    let mut header = [0u8; 4];
    match reader.read_exact(&mut header).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(format!("browser host frame header: {e}")),
    }
    let length = u32::from_le_bytes(header) as usize;
    if length > MAX_FRAME_BYTES {
        return Err("browser host frame exceeds 12 MiB".into());
    }
    let mut payload = vec![0u8; length];
    reader
        .read_exact(&mut payload)
        .await
        .map_err(|e| format!("browser host frame payload: {e}"))?;
    let value: Value = serde_json::from_slice(&payload).map_err(|_| "browser host invalid JSON".to_string())?;
    Ok(Some(value))
}

pub async fn write_frame<W: AsyncWriteExt + Unpin>(writer: &mut W, message: &Value) -> Result<(), String> {
    let frame = encode_frame(message)?;
    writer
        .write_all(&frame)
        .await
        .map_err(|e| format!("browser host write: {e}"))
}

pub fn result_response(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

pub fn error_response(id: &Value, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"message": message}})
}
