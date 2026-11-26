//! Browser-host server: unix-socket JSON-RPC endpoint the Synara server calls
//! for visible browser automation. Protocol-compatible with the upstream
//! TypeScript desktop host (`browserUsePipeServer.ts`).

use serde_json::{json, Value};
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};

use crate::protocol::{self, HOST_TYPE, PROTOCOL_VERSION};

/// Executes one browser tool call. Implemented by the shell (or anything else
/// that owns a visible browser). Arguments arrive exactly as upstream passes
/// them: `name` is the browser tool, `arguments` the MCP argument object.
pub trait HostExecutor: Send + Sync {
    fn execute(
        &self,
        session_id: &str,
        thread_id: Option<&str>,
        name: &str,
        arguments: Value,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Value, String>> + Send>>;
}

pub struct HostServer {
    pub pipe_path: String,
    pub capability: String,
}

impl HostServer {
    /// Bind the socket and serve forever on the current tokio runtime.
    /// `getInfo`/`executeTool` mirror the upstream methods exactly.
    pub async fn serve(self, executor: Arc<dyn HostExecutor>) -> Result<(), String> {
        let path = std::path::Path::new(&self.pipe_path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
            }
        }
        let _ = std::fs::remove_file(path);
        let listener = UnixListener::bind(path).map_err(|e| format!("bind {}: {e}", self.pipe_path))?;
        let capability = Arc::new(self.capability);
        loop {
            let (stream, _) = match listener.accept().await {
                Ok(pair) => pair,
                Err(_) => continue,
            };
            let cap = capability.clone();
            let exec = executor.clone();
            tokio::spawn(async move {
                let _ = handle_connection(stream, cap, exec).await;
            });
        }
    }
}

async fn handle_connection(
    stream: UnixStream,
    capability: Arc<String>,
    executor: Arc<dyn HostExecutor>,
) -> Result<(), String> {
    let (mut reader, mut writer) = stream.into_split();
    while let Some(message) = protocol::read_frame(&mut reader).await? {
        let id = message.get("id").cloned().unwrap_or(Value::Null);
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or_else(|| json!({}));
        // Capability gate, mirroring upstream: every method carries it.
        let offered = params.get("capability").and_then(Value::as_str).unwrap_or("");
        if offered != capability.as_str() {
            let response = protocol::error_response(&id, "browser host capability mismatch");
            protocol::write_frame(&mut writer, &response).await?;
            continue;
        }
        let response = match method {
            "getInfo" => {
                let session = params.get("session_id").and_then(Value::as_str).unwrap_or("");
                protocol::result_response(
                    &id,
                    json!({
                        "type": HOST_TYPE,
                        "metadata": {
                            "protocolVersion": PROTOCOL_VERSION,
                            "sessionId": session,
                            "methods": ["getInfo", "executeTool"],
                        },
                    }),
                )
            }
            "executeTool" => {
                let session = params.get("session_id").and_then(Value::as_str).unwrap_or("").to_string();
                let thread = params.get("thread_id").and_then(Value::as_str).map(str::to_string);
                let name = params.get("name").and_then(Value::as_str).unwrap_or("").to_string();
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                match executor.execute(&session, thread.as_deref(), &name, args).await {
                    Ok(result) => protocol::result_response(&id, result),
                    Err(message) => protocol::error_response(&id, &message),
                }
            }
            other => protocol::error_response(&id, &format!("unknown method: {other}")),
        };
        protocol::write_frame(&mut writer, &response).await?;
    }
    Ok(())
}
