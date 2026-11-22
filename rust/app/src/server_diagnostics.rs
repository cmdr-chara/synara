//! `server.getDiagnostics` — process memory/uptime, descendant process tree,
//! and projection counts. The state DB is exclusively locked by the upstream
//! process it is fronting, so counts are read from a byte copy.

use serde_json::{Value, json};
use std::collections::HashMap;
use std::process::Command;
use std::sync::LazyLock;
use std::time::Instant;

use crate::todo::WsErr;

static STARTED_AT: LazyLock<Instant> = LazyLock::new(Instant::now);

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), Some("DIAGNOSTICS"))
}

struct ProcRow {
    pid: u32,
    ppid: u32,
    rss_bytes: u64,
    vsz_bytes: u64,
    command: String,
    args: String,
}

fn process_table() -> Vec<ProcRow> {
    let Ok(out) = Command::new("ps")
        .args(["-Ao", "pid=,ppid=,rss=,vsz=,comm=,args="])
        .output()
    else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let mut it = line.split_whitespace();
            let pid = it.next()?.parse().ok()?;
            let ppid = it.next()?.parse().ok()?;
            let rss: u64 = it.next()?.parse().ok()?;
            let vsz: u64 = it.next()?.parse().ok()?;
            let command = it.next()?.to_string();
            let args = it.collect::<Vec<_>>().join(" ");
            Some(ProcRow {
                pid,
                ppid,
                rss_bytes: rss * 1024,
                vsz_bytes: vsz * 1024,
                command,
                args,
            })
        })
        .collect()
}

fn descendants(root: u32, table: &[ProcRow]) -> Vec<&ProcRow> {
    let mut by_ppid: HashMap<u32, Vec<u32>> = HashMap::new();
    for p in table {
        by_ppid.entry(p.ppid).or_default().push(p.pid);
    }
    let mut out = Vec::new();
    let mut stack: Vec<u32> = by_ppid.get(&root).cloned().unwrap_or_default();
    while let Some(pid) = stack.pop() {
        if let Some(row) = table.iter().find(|p| p.pid == pid) {
            out.push(row);
            stack.extend(by_ppid.get(&pid).cloned().unwrap_or_default());
        }
    }
    out.sort_by_key(|p| p.pid);
    out
}

/// Byte-copy the sqlite DB (+WAL) so the counts work while upstream holds the
/// exclusive lock. Returns zeros when unavailable.
fn projection_counts() -> (u64, u64) {
    let db = std::path::PathBuf::from(crate::todo::state_dir()).join("state.sqlite");
    let tmp = std::env::temp_dir().join(format!("synara-diag-{}.sqlite", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let tmp_wal = std::path::PathBuf::from(format!("{}-wal", tmp.display()));
    let _ = std::fs::remove_file(&tmp_wal);
    if std::fs::copy(&db, &tmp).is_err() {
        return (0, 0);
    }
    let wal = db.with_file_name("state.sqlite-wal");
    if wal.exists() {
        let _ = std::fs::copy(&wal, &tmp_wal);
    }
    let (mut projects, mut threads) = (0u64, 0u64);
    if let Ok(conn) = rusqlite::Connection::open(&tmp) {
        projects = conn
            .query_row("SELECT COUNT(*) FROM projection_projects", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|n| n.max(0) as u64)
            .unwrap_or(0);
        threads = conn
            .query_row("SELECT COUNT(*) FROM projection_threads", [], |r| {
                r.get::<_, i64>(0)
            })
            .map(|n| n.max(0) as u64)
            .unwrap_or(0);
    }
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp_wal);
    (projects, threads)
}

pub fn get_diagnostics() -> Result<Value, WsErr> {
    let self_pid = std::process::id();
    let table = process_table();
    let self_row = table.iter().find(|p| p.pid == self_pid);
    let rss = self_row.map(|r| r.rss_bytes).unwrap_or(0);
    let children = descendants(self_pid, &table);
    let total_rss: u64 = children.iter().map(|c| c.rss_bytes).sum();
    let (projects, threads) = projection_counts();
    Ok(json!({
        "generatedAt": crate::server_local_servers::iso_now(),
        "process": {
            "pid": self_pid,
            "uptimeSeconds": STARTED_AT.elapsed().as_secs(),
            "memory": {
                "rssBytes": rss,
                "heapTotalBytes": 0,
                "heapUsedBytes": 0,
                "externalBytes": 0,
                "arrayBuffersBytes": 0,
            },
        },
        "childProcesses": children.iter().map(|c| json!({
            "pid": c.pid,
            "ppid": c.ppid,
            "rssBytes": c.rss_bytes,
            "virtualSizeBytes": c.vsz_bytes,
            "command": c.command,
            "args": c.args,
        })).collect::<Vec<_>>(),
        "childProcessTotalCount": children.len(),
        "childProcessTotalRssBytes": total_rss,
        "projection": {
            "projectCount": projects,
            "threadCount": threads,
        },
    }))
}

pub fn handle(tag: &str, _payload: &Value) -> Result<Option<Value>, WsErr> {
    match tag {
        "server.getDiagnostics" => get_diagnostics().map(Some),
        _ => Err(err("unknown diagnostics tag")),
    }
}
