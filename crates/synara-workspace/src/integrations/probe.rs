//! Explicit, bounded Streamable HTTP discovery. This is not a tool runtime.
//! No tools/call, prompt/resource read, sampling, roots, or execution callbacks.
use super::*;
use serde_json::{Value, json};
use std::{io::{BufRead, BufReader, Read}, time::{Duration, Instant}};
use synara_runtime::{SecretStore, SecretStoreState, SecretValue};

const MODERN: &str = "2026-07-28";
const LEGACY: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];
const MAX_BODY: u64 = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpToolInfo {
    pub name: String,
    pub description: String,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct McpProbeReport {
    pub protocol: String,
    /// Unverified server-reported identity, never used to authorize anything.
    pub server: String,
    pub capabilities: Vec<String>,
    pub tools: Vec<McpToolInfo>,
    pub elapsed_ms: u128,
    pub cleanup_warning: bool,
}

pub(crate) async fn credential(config: &ManagedMcp, store: &dyn SecretStore) -> WorkspaceResult<Option<SecretValue>> {
    let Some(reference) = &config.bearer else { return Ok(None); };
    match store.state() {
        SecretStoreState::Locked => return Err(invalid("The credential store is locked. Unlock it before testing or sharing this connection.")),
        SecretStoreState::Unavailable => return Err(invalid("The OS credential store is unavailable. No plaintext fallback is permitted.")),
        SecretStoreState::Available => {},
    }
    let value = store.read(reference).await.map_err(|_| invalid("Cannot read this credential-store reference. No credential was persisted."))?
        .ok_or_else(|| invalid("The referenced MCP credential was not found."))?;
    // RFC 6750 b64token grammar. Reject CRLF/header injection and empty tokens.
    let token = std::str::from_utf8(value.expose()).map_err(|_| invalid("The referenced bearer credential is not valid text."))?;
    let body = token.trim_end_matches('=');
    if body.is_empty() || token.len() > 8192 || !body.bytes().all(|c|c.is_ascii_alphanumeric() || b"-._~+/".contains(&c)) {
        return Err(invalid("The referenced bearer credential is not a valid HTTP bearer token."));
    }
    Ok(Some(value))
}

struct Probe {
    http: ureq::Agent,
    url: String,
    token: Option<SecretValue>,
    session: Option<String>,
    protocol: String,
    start: Instant,
}
struct Reply { status: u16, rpc: Option<Value> }
impl Probe {
    fn authorization(&self) -> WorkspaceResult<Option<ureq::http::HeaderValue>> {
        self.token.as_ref().map(|token| {
            let mut bytes = b"Bearer ".to_vec();
            bytes.extend_from_slice(token.expose());
            let result = ureq::http::HeaderValue::from_bytes(&bytes)
                .map_err(|_| invalid("Invalid bearer header."));
            bytes.fill(0);
            result.map(|mut value| {value.set_sensitive(true);value})
        }).transpose()
    }
    fn post(&mut self, method: &str, id: Option<u64>, mut params: Value) -> WorkspaceResult<Reply> {
        if self.start.elapsed() > Duration::from_secs(45) {
            return Err(invalid("MCP discovery exceeded its total time limit."));
        }
        if self.protocol == MODERN {
            params["_meta"] = json!({
                "io.modelcontextprotocol/protocolVersion":MODERN,
                "io.modelcontextprotocol/clientInfo":{"name":"Synara","version":env!("CARGO_PKG_VERSION")},
                "io.modelcontextprotocol/clientCapabilities":{}
            });
        }
        let mut message = json!({"jsonrpc":"2.0","method":method,"params":params});
        if let Some(id) = id {message["id"] = json!(id);}
        let mut request = self.http.post(&self.url)
            .header("Content-Type","application/json")
            .header("Accept","application/json, text/event-stream")
            .header("MCP-Protocol-Version",self.protocol.as_str());
        if self.protocol == MODERN {request = request.header("Mcp-Method",method);}
        if let Some(auth) = self.authorization()? {request = request.header("Authorization",auth);}
        if let Some(session) = &self.session {request = request.header("MCP-Session-Id",session.as_str());}
        let mut response = request.send(message.to_string()).map_err(|_| invalid("MCP network, TLS or timeout failure. No connection was confirmed."))?;
        let status = response.status().as_u16();
        if status == 401 || status == 403 {
            return Err(invalid("MCP authentication was rejected. Check the credential reference and provider authorization."));
        }
        if response.status().is_redirection() {return Err(invalid("MCP redirects are not followed. Review and configure the final endpoint explicitly."));}
        if method == "initialize" {
            if let Some(header) = response.headers().get("MCP-Session-Id") {
                let value = header.to_str().map_err(|_|invalid("Invalid MCP session header."))?;
                if value.is_empty() || value.len() > 256 || !value.bytes().all(|c| (0x21..=0x7e).contains(&c)) {
                    return Err(invalid("Invalid MCP session header."));
                }
                self.session = Some(value.into());
            }
        }
        if id.is_none() {
            return if status == 202 {Ok(Reply {status,rpc:None})}
                else {Err(invalid("MCP initialization notification was not accepted."))};
        }
        let mime = response.headers().get("content-type").and_then(|v|v.to_str().ok())
            .unwrap_or("").split(';').next().unwrap_or("").trim().to_ascii_lowercase();
        let rpc = match mime.as_str() {
            "application/json" => {
                let mut bytes = Vec::new();
                response.body_mut().as_reader().take(MAX_BODY + 1).read_to_end(&mut bytes)
                    .map_err(|_|invalid("MCP response read failed or timed out."))?;
                if bytes.len() as u64 > MAX_BODY {return Err(invalid("MCP response exceeds 1 MiB."));}
                serde_json::from_slice(&bytes).ok()
            },
            "text/event-stream" => Some(read_sse(response.body_mut().as_reader(), id.expect("request id"))?),
            _ if matches!(status,400|404|405) => None,
            _ => return Err(invalid("The endpoint did not return MCP JSON or an event stream.")),
        };
        Ok(Reply {status,rpc})
    }
    fn result(reply: Reply, id: u64) -> WorkspaceResult<Value> {
        if !(200..300).contains(&reply.status) {return Err(invalid(&format!("MCP HTTP {}. No connection was confirmed.",reply.status)));}
        let rpc = reply.rpc.ok_or_else(||invalid("Invalid MCP JSON response."))?;
        if rpc.get("jsonrpc").and_then(Value::as_str)!=Some("2.0") || rpc.get("id")!=Some(&json!(id)) || rpc.get("error").is_some() {
            return Err(invalid("MCP returned an error or a mismatched JSON-RPC response. Remote error bodies are not displayed because they may contain credentials."));
        }
        let result = rpc.get("result").filter(|v|v.is_object()).ok_or_else(||invalid("MCP response has no result object."))?;
        if result.get("resultType").is_some_and(|v|v.as_str()!=Some("complete")) {
            return Err(invalid("MCP discovery requires an unsupported interaction. No callbacks or tool execution were authorized."));
        }
        Ok(result.clone())
    }
    fn discover(&mut self) -> WorkspaceResult<McpProbeReport> {
        let reply = self.post("server/discover",Some(1),json!({}))?;
        let code = reply.rpc.as_ref().and_then(|r|r.pointer("/error/code")).and_then(Value::as_i64);
        // A modern protocol error is evidence of the modern era, not permission
        // to downgrade. Legacy HTTP rejects unknown/pre-initialize requests.
        let fallback = matches!(reply.status,400|404|405) && !matches!(code,Some(-32020|-32021|-32022)) && !(reply.status==404 && code==Some(-32601));
        let result = if fallback {
            self.protocol = LEGACY[0].into();
            let reply = self.post("initialize",Some(2),json!({"protocolVersion":LEGACY[0],"capabilities":{},"clientInfo":{"name":"Synara","version":env!("CARGO_PKG_VERSION")}}))?;
            let result = Self::result(reply,2)?;
            let version = result.get("protocolVersion").and_then(Value::as_str).ok_or_else(||invalid("MCP initialization omitted its protocol version."))?;
            if !LEGACY.contains(&version) {return Err(invalid("The MCP server selected an unsupported protocol version."));}
            self.protocol = version.into();
            self.post("notifications/initialized",None,json!({}))?;
            result
        } else {
            let result = Self::result(reply,1)?;
            if !result.get("supportedVersions").and_then(Value::as_array).is_some_and(|versions|versions.iter().any(|v|v.as_str()==Some(MODERN))) {
                return Err(invalid("The server did not advertise the requested MCP protocol version."));
            }
            result
        };
        let capabilities = result.get("capabilities").and_then(Value::as_object).ok_or_else(||invalid("MCP discovery omitted capability negotiation."))?;
        if capabilities.len() > 64 {return Err(invalid("Too many MCP capabilities."));}
        let server = if self.protocol == MODERN {result.pointer("/_meta/io.modelcontextprotocol~1serverInfo/name")} else {result.pointer("/serverInfo/name")};
        let mut report = McpProbeReport {
            protocol:self.protocol.clone(),server:self.display(server,160),
            capabilities:capabilities.keys().map(|k|self.display(Some(&Value::String(k.clone())),128)).collect(),
            tools:vec![],elapsed_ms:0,cleanup_warning:false,
        };
        if capabilities.contains_key("tools") {
            if !capabilities["tools"].is_object() {return Err(invalid("Invalid MCP tools capability."));}
            let mut cursor = None;
            let mut seen = HashSet::new();
            let mut names = HashSet::new();
            for page in 0..4 {
                let params = cursor.as_ref().map_or_else(||json!({}), |value|json!({"cursor":value}));
                let id = 10 + page;
                let response = self.post("tools/list",Some(id),params)?;
                let result = Self::result(response,id)?;
                let tools = result.get("tools").and_then(Value::as_array).ok_or_else(||invalid("MCP tool list is malformed."))?;
                if report.tools.len()+tools.len()>256 {return Err(invalid("MCP discovery is limited to 256 tools."));}
                for tool in tools {
                    let name = tool.get("name").and_then(Value::as_str).filter(|name|text(name,256))
                        .ok_or_else(||invalid("Invalid MCP tool name."))?;
                    if !names.insert(name.to_string()) || !tool.get("inputSchema").is_some_and(Value::is_object) {
                        return Err(invalid("Duplicate tool name or missing MCP input schema."));
                    }
                    report.tools.push(McpToolInfo {name:self.display(tool.get("name"),256),description:self.display(tool.get("description"),1024)});
                }
                cursor = match result.get("nextCursor") {
                    None => None,
                    Some(value) => Some(value.as_str().filter(|v|text(v,1024)).ok_or_else(||invalid("Invalid MCP pagination cursor."))?.to_string()),
                };
                let Some(next) = cursor.as_ref() else {break};
                if page==3 || !seen.insert(next.clone()) {return Err(invalid("MCP pagination exceeded its bound or repeated a cursor."));}
            }
        }
        report.elapsed_ms = self.start.elapsed().as_millis();
        Ok(report)
    }
    fn display(&self, value: Option<&Value>, limit: usize) -> String {
        let value = value.and_then(Value::as_str).unwrap_or("");
        let value = self.token.as_ref().and_then(|secret|std::str::from_utf8(secret.expose()).ok())
            .map_or_else(||value.to_owned(), |token|value.replace(token,"[REDACTED]"));
        value.chars().take(limit).map(|c|if c.is_control(){' '}else{c}).collect()
    }
    fn cleanup(&self) -> bool {
        let Some(session) = &self.session else {return true};
        let mut request = self.http.delete(&self.url)
            .header("MCP-Protocol-Version",self.protocol.as_str())
            .header("MCP-Session-Id",session.as_str());
        match self.authorization() {
            Ok(Some(auth)) => request = request.header("Authorization",auth),
            Ok(None) => {},
            Err(_) => return false,
        }
        request.call().is_ok_and(|response|response.status().is_success())
    }
}
/// Bound the entire event stream including ignored comments/notifications. Read
/// only until the matching reply, not EOF on a persistent server connection.
fn read_sse(reader: impl Read, id: u64) -> WorkspaceResult<Value> {
    let mut reader = BufReader::new(reader.take(MAX_BODY + 1));
    let mut line = String::new();
    let mut data = String::new();
    let mut total = 0;
    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|_|invalid("Invalid or timed-out MCP event stream."))?;
        total += n;
        if total as u64 > MAX_BODY {return Err(invalid("MCP event stream exceeds 1 MiB."));}
        if n==0 {return Err(invalid("MCP event stream ended without a matching result."));}
        if line.trim_end_matches(['\r','\n']).is_empty() {
            if !data.is_empty() {
                let rpc: Value = serde_json::from_str(&data).map_err(|_|invalid("Malformed MCP event data."))?;
                if rpc.get("id")==Some(&json!(id)) && rpc.get("method").is_none() {return Ok(rpc);}
                if rpc.get("method").is_some() && rpc.get("id").is_some() {return Err(invalid("MCP requested an unauthorized client callback."));}
                data.clear();
            }
        } else if let Some(value) = line.strip_prefix("data:") {
            data.push_str(value.strip_prefix(' ').unwrap_or(value));
        }
    }
}

pub(crate) fn probe(config: ManagedMcp, token: Option<SecretValue>) -> WorkspaceResult<McpProbeReport> {
    config.validate()?;
    let http = ureq::Agent::config_builder().proxy(None).max_redirects(0)
        .http_status_as_error(false).timeout_global(Some(Duration::from_secs(10)))
        .timeout_connect(Some(Duration::from_secs(5))).timeout_resolve(Some(Duration::from_secs(5)))
        .max_response_header_size(16*1024).build().new_agent();
    let mut probe = Probe {http,url:endpoint(&config.endpoint)?.into(),token,session:None,protocol:MODERN.into(),start:Instant::now()};
    let result = probe.discover();
    let cleanup = probe.cleanup();
    result.map(|mut report| {report.cleanup_warning = !cleanup;report})
}

#[cfg(test)]
mod tests;
