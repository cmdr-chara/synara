//! Browser-host client: the server-side caller, mirroring
//! `browserHostRpcClient.ts` — connect per call, `getInfo` handshake with
//! protocol assertions, then `executeTool`.

use serde_json::{json, Value};
use tokio::net::UnixStream;

use crate::protocol::{self, HOST_TYPE, PROTOCOL_VERSION};

pub struct HostClient {
    pub pipe_path: String,
    pub capability: String,
}

impl HostClient {
    pub fn from_discovery(discovery: &crate::discovery::Discovery) -> Self {
        Self {
            pipe_path: discovery.pipe_path.clone(),
            capability: discovery.capability.clone(),
        }
    }

    pub async fn execute_tool(
        &self,
        session_id: &str,
        thread_id: Option<&str>,
        name: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let stream = UnixStream::connect(&self.pipe_path)
            .await
            .map_err(|e| format!("browser host connect: {e}"))?;
        let (mut reader, mut writer) = stream.into_split();

        // getInfo handshake.
        let info_req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getInfo",
            "params": {"session_id": session_id, "capability": self.capability},
        });
        protocol::write_frame(&mut writer, &info_req).await?;
        let info = protocol::read_frame(&mut reader)
            .await?
            .ok_or("browser host closed during getInfo")?;
        if info.get("error").is_some() {
            let message = info
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or("browser host rejected getInfo");
            return Err(message.to_string());
        }
        let result = info.get("result").cloned().unwrap_or(Value::Null);
        let version = result
            .pointer("/metadata/protocolVersion")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let methods = result
            .pointer("/metadata/methods")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let has_execute = methods.iter().any(|m| m.as_str() == Some("executeTool"));
        let host_type = result.get("type").and_then(Value::as_str).unwrap_or(HOST_TYPE);
        if version != PROTOCOL_VERSION || host_type != HOST_TYPE || !has_execute {
            return Err("The visible browser host uses an incompatible protocol.".into());
        }

        // executeTool.
        let exec_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "executeTool",
            "params": {
                "session_id": session_id,
                "capability": self.capability,
                "thread_id": thread_id,
                "name": name,
                "arguments": arguments,
            },
        });
        protocol::write_frame(&mut writer, &exec_req).await?;
        let response = protocol::read_frame(&mut reader)
            .await?
            .ok_or("browser host closed during executeTool")?;
        if let Some(error) = response.get("error") {
            let message = error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("browser host rejected the request");
            return Err(message.to_string());
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }
}
