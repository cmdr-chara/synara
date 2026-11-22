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
    ) || (std::env::var("SYNARA_NO_UPSTREAM").is_ok()
        && matches!(
            tag,
            "server.disconnectMcpConnector" | "server.resolveMcpConnectorProposal"
        ))
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

/// `server.disconnectMcpConnector` — runtime.ts `disconnect`: a stdio entry
/// is removed first (process teardown is upstream's job; ours owns the doc),
/// otherwise the HTTP connector row is dropped when present. OAuth token
/// revocation is a best-effort network call upstream does after saving;
/// removal itself is the persisted truth. Missing id -> disconnected:false.
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
    if let Some(list) = doc.get_mut("stdio").and_then(|v| v.as_array_mut()) {
        let before = list.len();
        list.retain(|e| e.get("connectorId").and_then(|v| v.as_str()) != Some(id.as_str()));
        removed = list.len() < before;
    }
    if !removed {
        if let Some(list) = doc.get_mut("connectors").and_then(|v| v.as_array_mut()) {
            let before = list.len();
            list.retain(|e| e.get("connectorId").and_then(|v| v.as_str()) != Some(id.as_str()));
            removed = list.len() < before;
        }
    }
    if removed {
        write_document(&doc)?;
    }
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
                    // connectStdio: dedupe on (command, cwd, args); keep an
                    // existing connectorId/serverName. Status stays
                    // "unreachable" — no stdio probe runtime here.
                    let command = i.get("command").cloned().unwrap_or(Value::Null);
                    let cwd = i.get("cwd").cloned().unwrap_or(Value::Null);
                    let args = i.get("args").cloned().unwrap_or(json!([]));
                    let name = i.get("name").and_then(|v| v.as_str()).unwrap_or("Connector");
                    let env = i.get("env").cloned().unwrap_or(json!({}));
                    let mut env_keys: Vec<String> = env
                        .as_object()
                        .map(|o| o.keys().cloned().collect())
                        .unwrap_or_default();
                    env_keys.sort();
                    let list_len = doc
                        .get("stdio")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0)
                        + doc
                            .get("connectors")
                            .and_then(|v| v.as_array())
                            .map(|a| a.len())
                            .unwrap_or(0);
                    let existing_idx = doc
                        .get("stdio")
                        .and_then(|v| v.as_array())
                        .and_then(|a| {
                            a.iter().position(|e| {
                                e.get("command") == Some(&command)
                                    && e.get("cwd").cloned().unwrap_or(Value::Null) == cwd
                                    && e.get("args").cloned().unwrap_or(json!([])) == args
                            })
                        });
                    if existing_idx.is_none() && list_len >= 30 {
                        return Err(err("Too many connectors."));
                    }
                    let (existing_id, existing_name, existing_created) = existing_idx
                        .and_then(|ix| doc.pointer(&format!("/stdio/{ix}")).cloned())
                        .map(|e| {
                            (
                                e.get("connectorId").and_then(|v| v.as_str()).map(String::from),
                                e.get("serverName").and_then(|v| v.as_str()).map(String::from),
                                e.get("createdAt").and_then(|v| v.as_str()).map(String::from),
                            )
                        })
                        .unwrap_or((None, None, None));
                    let server_name = existing_name.unwrap_or_else(|| {
                        unique_server_name(&doc, &mcp_connector_slug(name))
                    });
                    let now = chrono_iso_now();
                    let saved = json!({
                        "name": name,
                        "command": command,
                        "args": args,
                        "cwd": cwd,
                        "env": env,
                        "transport": "stdio",
                        "connectorId": existing_id
                            .unwrap_or_else(|| format!("mcp_conn_{}", crate::orchestration_engine::uuid_v4())),
                        "presetId": Value::Null,
                        "serverName": server_name,
                        "authKind": "none",
                        "status": "unreachable",
                        "envKeys": env_keys,
                        "createdAt": existing_created.unwrap_or_else(|| now.clone()),
                        "updatedAt": now,
                    });
                    let list = doc
                        .get_mut("stdio")
                        .and_then(|v| v.as_array_mut())
                        .ok_or_else(|| err("malformed connectors document"))?;
                    match existing_idx {
                        Some(ix) => list[ix] = saved,
                        None => list.push(saved),
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

/// `mcpConnectorSlug` (packages/shared): lowercase alnum + single
/// underscores, must start with a letter, `synara…` names get `custom_`,
/// max 24 chars.
fn mcp_connector_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut last_us = true;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_us = false;
        } else if !last_us {
            slug.push('_');
            last_us = true;
        }
    }
    let slug = slug.trim_matches('_').to_string();
    let slug = if !slug.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false) {
        format!("mcp_{}", slug.trim_end_matches('_'))
    } else {
        slug
    };
    let slug = if slug.starts_with("synara") {
        format!("custom_{slug}")
    } else {
        slug
    };
    slug.chars().take(24).collect::<String>().trim_end_matches('_').to_string()
}

/// `uniqueServerName`: `<slug>_connector`, suffixed `_2`, `_3`, ... until free.
fn unique_server_name(doc: &Value, slug: &str) -> String {
    let taken: Vec<String> = ["connectors", "stdio"]
        .iter()
        .flat_map(|k| doc.get(*k).and_then(|v| v.as_array()).cloned().unwrap_or_default())
        .filter_map(|e| e.get("serverName").and_then(|v| v.as_str()).map(String::from))
        .collect();
    let mut candidate = format!("{slug}_connector");
    let mut suffix = 2;
    while taken.iter().any(|t| t == &candidate) {
        candidate = format!("{slug}_{suffix}_connector");
        suffix += 1;
    }
    candidate
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
