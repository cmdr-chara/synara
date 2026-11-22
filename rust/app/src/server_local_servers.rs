//! `server.listLocalServers` / `server.stopLocalServer` — enumerate TCP
//! listeners on this machine so the UI can attribute and stop stray dev
//! servers. Backed by `lsof`/`ps` (no procfs assumptions).

use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::process::Command;

use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("LOCAL_SERVERS"))
}

struct Proc {
    command: String,
    addresses: Vec<(String, u32, &'static str)>,
}

/// `lsof -nP -iTCP -sTCP:LISTEN -F pcn` → per-pid command + listen addresses.
fn listeners() -> BTreeMap<u32, Proc> {
    let mut map: BTreeMap<u32, Proc> = BTreeMap::new();
    let Ok(out) = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN", "-F", "pcn"])
        .output()
    else {
        return map;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut pid: u32 = 0;
    let mut command = String::new();
    for line in text.lines() {
        let (tag, val) = line.split_at(line.len().min(1));
        match tag {
            "p" => {
                pid = val.trim_start_matches('p').parse().unwrap_or(0);
            }
            "c" => command = val.to_string(),
            "n" if pid > 1 => {
                let name = val;
                // "*:8080", "127.0.0.1:8080", "[::1]:8080"
                let Some((host, port_s)) = name.rsplit_once(':') else {
                    continue;
                };
                let Ok(port) = port_s.parse::<u32>() else {
                    continue;
                };
                let family = if host.contains(':') || host.contains(']') {
                    "tcp6"
                } else {
                    "tcp4"
                };
                map.entry(pid)
                    .or_insert_with(|| Proc {
                        command: command.clone(),
                        addresses: Vec::new(),
                    })
                    .addresses
                    .push((host.trim_matches(['[', ']']).to_string(), port, family));
            }
            _ => {}
        }
    }
    map
}

fn ps_field(pid: u32, field: &str) -> Option<String> {
    let out = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", &format!("{field}=")])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!s.is_empty()).then_some(s)
}

fn proc_cwd(pid: u32) -> Option<String> {
    let out = Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .find(|l| l.starts_with('n'))
        .map(|l| l[1..].to_string())
}

pub(crate) fn iso_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    crate::projects_fileops::iso8601(secs)
}

pub fn list_local_servers() -> Result<Value, WsErr> {
    let self_pid = std::process::id();
    let mut servers = Vec::new();
    for (pid, proc_) in listeners() {
        if proc_.addresses.is_empty() {
            continue;
        }
        let ppid = ps_field(pid, "ppid").and_then(|s| s.parse::<u32>().ok());
        let args = ps_field(pid, "args").unwrap_or_else(|| proc_.command.clone());
        let cwd = proc_cwd(pid);
        let ports: Vec<u32> = proc_.addresses.iter().map(|a| a.1).collect();
        let addresses: Vec<Value> = proc_
            .addresses
            .iter()
            .map(|(host, port, family)| {
                json!({
                    "host": host,
                    "port": port,
                    "family": family,
                    "url": null,
                })
            })
            .collect();
        let is_self = pid == self_pid;
        let mut entry = json!({
            "id": pid.to_string(),
            "pid": pid,
            "command": proc_.command,
            "displayName": proc_.command,
            "args": args,
            "ports": ports,
            "addresses": addresses,
            "isStoppable": !is_self,
        });
        if let Some(ppid) = ppid {
            entry["ppid"] = json!(ppid);
        }
        if let Some(cwd) = cwd {
            if !cwd.is_empty() {
                entry["cwd"] = json!(cwd);
            }
        }
        if is_self {
            entry["stopDisabledReason"] = json!("cannot stop the serving process");
        }
        servers.push(entry);
    }
    Ok(json!({ "generatedAt": iso_now(), "servers": servers }))
}

pub fn stop_local_server(payload: &Value) -> Result<Value, WsErr> {
    let pid = payload
        .get("pid")
        .and_then(|v| v.as_u64())
        .filter(|n| *n > 0)
        .ok_or_else(|| err("pid is required"))? as u32;
    let port = payload
        .get("port")
        .and_then(|v| v.as_u64())
        .filter(|n| *n > 0)
        .ok_or_else(|| err("port is required"))? as u32;
    if pid == std::process::id() {
        return Ok(json!({
            "pid": pid,
            "stopped": false,
            "message": "cannot stop the serving process",
        }));
    }
    // Only kill when the pid actually owns the advertised port.
    let owns = listeners()
        .get(&pid)
        .map(|p| p.addresses.iter().any(|a| a.1 == port))
        .unwrap_or(false);
    if !owns {
        return Ok(json!({
            "pid": pid,
            "stopped": false,
            "message": "process is not listening on that port",
        }));
    }
    let status = Command::new("/bin/kill")
        .arg(pid.to_string())
        .status()
        .map_err(|e| err(format!("kill failed: {e}")))?;
    Ok(json!({ "pid": pid, "stopped": status.success() }))
}
