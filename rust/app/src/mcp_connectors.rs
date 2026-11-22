//! `server.listMcpConnectors` / `server.listMcpConnectorProposals` /
//! `server.getMcpConnectorSetup` — read the connectors document upstream
//! stores as the `mcp-connectors` secret (`secrets/mcp-connectors.bin`,
//! JSON `{version:1, connectors, stdio, proposals, unreadable}`), and report
//! the OAuth callback URL this server answers at `/oauth/mcp/callback`.

use serde_json::{Value, json};

use crate::todo::{WsErr, state_dir};

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("MCP"))
}

pub fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "server.listMcpConnectors"
            | "server.listMcpConnectorProposals"
            | "server.getMcpConnectorSetup"
            | "server.disconnectMcpConnector"
            | "server.resolveMcpConnectorProposal"
    )
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "server.listMcpConnectors" => Ok(Value::Array(list_connectors())),
        "server.listMcpConnectorProposals" => Ok(Value::Array(list_proposals())),
        "server.getMcpConnectorSetup" => Ok(json!({
            "redirectUrl": format!("http://127.0.0.1:{}/oauth/mcp/callback", server_port()),
        })),
        "server.disconnectMcpConnector" => disconnect(payload),
        "server.resolveMcpConnectorProposal" => resolve_proposal(payload),
        _ => Err(err("unhandled mcp connector tag")),
    }
}

fn server_port() -> u16 {
    std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080)
}

fn document() -> Option<Value> {
    let path = std::path::PathBuf::from(state_dir())
        .join("secrets")
        .join("mcp-connectors.bin");
    let text = std::fs::read_to_string(path).ok()?;
    let doc: Value = serde_json::from_str(&text).ok()?;
    if doc.get("version")?.as_i64()? != 1 {
        return None;
    }
    Some(doc)
}

/// HTTP connectors: drop the `oauth` credentials block, map url→transport.
/// Stdio connectors: drop the `env` secret values (the public `envKeys`
/// field stays).
fn list_connectors() -> Vec<Value> {
    let mut out = Vec::new();
    let Some(doc) = document() else {
        return out;
    };
    if let Some(entries) = doc.get("connectors").and_then(|c| c.as_array()) {
        for entry in entries {
            let mut c = entry.clone();
            c.as_object_mut().map(|o| o.remove("oauth"));
            let transport = c
                .get("url")
                .and_then(|u| u.as_str())
                .map(|u| {
                    if u.starts_with("http://") {
                        "local-http"
                    } else {
                        "remote-http"
                    }
                })
                .unwrap_or("remote-http");
            c["transport"] = json!(transport);
            c["usesOwnOAuthApp"] = json!(
                entry
                    .pointer("/oauth/userClient")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            );
            out.push(c);
        }
    }
    if let Some(entries) = doc.get("stdio").and_then(|c| c.as_array()) {
        for entry in entries {
            let mut c = entry.clone();
            c.as_object_mut().map(|o| o.remove("env"));
            out.push(c);
        }
    }
    out
}

fn list_proposals() -> Vec<Value> {
    document()
        .and_then(|d| d.get("proposals").cloned())
        .and_then(|p| p.as_array().cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.get("proposal").cloned())
        .collect()
}

fn doc_path() -> std::path::PathBuf {
    std::path::PathBuf::from(state_dir())
        .join("secrets")
        .join("mcp-connectors.bin")
}

fn write_document(doc: &Value) -> Result<(), WsErr> {
    let text = serde_json::to_string(doc).map_err(|e| err(e.to_string()))?;
    std::fs::write(doc_path(), text).map_err(|e| err(e.to_string()))
}

/// `server.disconnectMcpConnector`: drop the connector from both the HTTP and
/// stdio lists and persist the document. Missing id -> disconnected:false.
fn disconnect(payload: &Value) -> Result<Value, WsErr> {
    let id = payload
        .get("connectorId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if id.is_empty() {
        return Err(WsErr("connectorId is required".into(), Some("VALIDATION")));
    }
    let mut doc = document().unwrap_or(json!({
        "version": 1, "connectors": [], "stdio": [], "proposals": [], "unreadable": []
    }));
    let mut removed = false;
    for key in ["connectors", "stdio"] {
        if let Some(list) = doc.get_mut(key).and_then(|v| v.as_array_mut()) {
            let before = list.len();
            list.retain(|e| e.get("connectorId").and_then(|v| v.as_str()) != Some(id.as_str()));
            removed |= list.len() < before;
        }
    }
    write_document(&doc)?;
    Ok(json!({"disconnected": removed}))
}

/// `server.resolveMcpConnectorProposal`: mark a pending proposal accepted or
/// dismissed. Accepting a "remove" disconnects the connector; accepting an
/// "add" applies its stored stdio input (HTTP adds need the OAuth sign-in
/// flow, which requires the connector runtime).
fn resolve_proposal(payload: &Value) -> Result<Value, WsErr> {
    let id = payload
        .get("proposalId")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let resolution = payload
        .get("resolution")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    if id.is_empty() || (resolution != "accepted" && resolution != "dismissed") {
        return Err(WsErr(
            "proposalId and resolution (accepted|dismissed) are required".into(),
            Some("VALIDATION"),
        ));
    }
    let Some(mut doc) = document() else {
        return Err(err("no connectors document"));
    };
    let proposals = doc
        .get_mut("proposals")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| err("malformed connectors document"))?;
    let idx = proposals
        .iter()
        .position(|p| p.pointer("/proposal/proposalId").and_then(|v| v.as_str()) == Some(id.as_str()))
        .ok_or_else(|| err("Proposal not found."))?;
    let entry = proposals[idx].clone();
    let mut proposal = entry
        .get("proposal")
        .cloned()
        .ok_or_else(|| err("malformed proposal"))?;
    if proposal.get("state").and_then(|v| v.as_str()) != Some("pending") {
        return Ok(json!({"proposal": proposal, "authorizationUrl": Value::Null}));
    }
    if resolution == "accepted" {
        let action = proposal
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if action == "remove" {
            let cid = proposal
                .pointer("/connector/connectorId")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            for key in ["connectors", "stdio"] {
                if let Some(list) = doc.get_mut(key).and_then(|v| v.as_array_mut()) {
                    list.retain(|e| {
                        e.get("connectorId").and_then(|v| v.as_str()) != Some(cid.as_str())
                    });
                }
            }
        } else if action == "add" {
            let input = entry.get("input").cloned();
            match input {
                Some(i) if i.get("command").is_some() => {
                    // stdio add: store the public connector + secret env.
                    let env = i.get("env").cloned().unwrap_or(json!({}));
                    let env_keys: Vec<String> = env
                        .as_object()
                        .map(|o| o.keys().cloned().collect())
                        .unwrap_or_default();
                    let mut c = proposal
                        .get("connector")
                        .cloned()
                        .unwrap_or(json!({}));
                    c["status"] = json!("connected");
                    c["updatedAt"] = json!(chrono_iso_now());
                    c["envKeys"] = json!(env_keys);
                    c["env"] = env;
                    if let Some(list) = doc.get_mut("stdio").and_then(|v| v.as_array_mut()) {
                        list.push(c);
                    }
                }
                Some(_) => {
                    return Err(err(
                        "accepting an HTTP connector proposal requires the OAuth sign-in flow",
                    ));
                }
                None => return Err(err("This proposal cannot be applied.")),
            }
        }
    }
    proposal["state"] = json!(resolution);
    if let Some(list) = doc.get_mut("proposals").and_then(|v| v.as_array_mut()) {
        list[idx]["proposal"] = proposal.clone();
    }
    write_document(&doc)?;
    Ok(json!({"proposal": proposal, "authorizationUrl": Value::Null}))
}

fn chrono_iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let t = secs % 86400;
    let (y, m, d) = civil(days as i64);
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, t / 3600, (t % 3600) / 60, t % 60)
}

fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
