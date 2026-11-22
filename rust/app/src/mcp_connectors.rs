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
    )
}

pub fn handle(tag: &str, _payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "server.listMcpConnectors" => Ok(Value::Array(list_connectors())),
        "server.listMcpConnectorProposals" => Ok(Value::Array(list_proposals())),
        "server.getMcpConnectorSetup" => Ok(json!({
            "redirectUrl": format!("http://127.0.0.1:{}/oauth/mcp/callback", server_port()),
        })),
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
