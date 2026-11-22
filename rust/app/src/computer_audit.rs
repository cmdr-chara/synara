// computer.getAuditHistory — native port of
// apps/server/src/computer/computerAuditHistory.ts +
// computerAuditFile.ts. Reads state_dir/computer-audit.jsonl plus the two
// conventional rotations (.1, .2) with the same bounded-tail and cursor
// semantics, never a full-file read.

use serde_json::{json, Value};
use sha2::Digest;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

use crate::todo::WsErr;

const MAX_BYTES: usize = 2 * 1024 * 1024;
const MAX_ROWS: usize = 10_000;
const MAX_LINE_BYTES: usize = 64 * 1024;
const MAX_LIMIT: i64 = 100;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), None)
}

fn digest(value: &[u8]) -> String {
    let mut h = sha2::Sha256::new();
    h.update(value);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn base64url(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(T[(b2 & 63) as usize] as char);
        }
    }
    out
}

fn base64url_decode(s: &str) -> Option<Vec<u8>> {
    fn v(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 4 <= bytes.len() {
        let n: u32 = (v(bytes[i])? as u32) << 18
            | (v(bytes[i + 1])? as u32) << 12
            | (v(bytes[i + 2])? as u32) << 6
            | (v(bytes[i + 3])? as u32);
        out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8, n as u8]);
        i += 4;
    }
    match bytes.len() - i {
        2 => {
            let n: u32 = (v(bytes[i])? as u32) << 18 | (v(bytes[i + 1])? as u32) << 12;
            out.push((n >> 16) as u8);
        }
        3 => {
            let n: u32 = (v(bytes[i])? as u32) << 18
                | (v(bytes[i + 1])? as u32) << 12
                | (v(bytes[i + 2])? as u32) << 6;
            out.extend_from_slice(&[(n >> 16) as u8, (n >> 8) as u8]);
        }
        0 => {}
        _ => return None,
    }
    Some(out)
}

struct Tail {
    contents: Vec<u8>,
    start: u64,
    requested: u64,
    device: u64,
    inode: u64,
}

/// computerAuditFile.ts readComputerAuditFileTail — identity-checked bounded
/// tail read, never the whole file.
fn read_tail(path: &Path, max_bytes: usize) -> Result<Option<Tail>, WsErr> {
    let read_err = || err("Could not read Computer activity history.");
    let entry = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(read_err()),
    };
    if !entry.is_file() {
        return Err(read_err());
    }
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(read_err()),
    };
    let stat = file.metadata().map_err(|_| read_err())?;
    let entry2 = std::fs::symlink_metadata(path).map_err(|_| read_err())?;
    if !stat.is_file()
        || !entry2.is_file()
        || entry2.dev() != stat.dev()
        || entry2.ino() != stat.ino()
    {
        return Err(read_err());
    }
    let length = std::cmp::min(stat.size(), max_bytes as u64);
    let start = stat.size() - length;
    let mut contents = vec![0u8; length as usize];
    use std::io::{Read, Seek};
    let mut f = file;
    f.seek(std::io::SeekFrom::Start(start)).map_err(|_| read_err())?;
    let mut read = 0usize;
    while read < contents.len() {
        match f.read(&mut contents[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(read_err()),
        }
    }
    contents.truncate(read);
    Ok(Some(Tail {
        contents,
        start,
        requested: length,
        device: stat.dev(),
        inode: stat.ino(),
    }))
}

/// computerAuditTailLines — complete newline-terminated records, newest first.
fn tail_lines(tail: &Tail) -> Vec<(Vec<u8>, u64)> {
    let mut out = Vec::new();
    let c = &tail.contents;
    let mut end = c.iter().rposition(|&b| b == b'\n');
    while let Some(e) = end {
        let prev = if e > 0 { c[..e].iter().rposition(|&b| b == b'\n') } else { None };
        let line_start = prev.map(|p| p + 1).unwrap_or(0);
        if prev.is_none() && tail.start > 0 {
            break;
        }
        out.push((c[line_start..=e].to_vec(), tail.start + line_start as u64));
        end = prev;
    }
    out
}

#[derive(Clone)]
struct Position {
    file: String,
    offset: u64,
    fingerprint: String,
}

fn cursor_for(p: &Position) -> String {
    base64url(
        json!({"file": p.file, "offset": p.offset, "fingerprint": p.fingerprint})
            .to_string()
            .as_bytes(),
    )
}

fn parse_cursor(cursor: Option<&str>) -> Result<Option<Position>, WsErr> {
    let invalid = || err("Invalid Computer activity cursor. Refresh the history to continue.");
    let Some(c) = cursor else { return Ok(None) };
    if c.len() > 512 || !c.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return Err(invalid());
    }
    let decoded = base64url_decode(c)
        .and_then(|b| String::from_utf8(b).ok())
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let Some(v) = decoded else { return Err(invalid()) };
    let fp_ok = |s: &str| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    match (
        v.get("file").and_then(Value::as_str),
        v.get("offset").and_then(Value::as_u64),
        v.get("fingerprint").and_then(Value::as_str),
    ) {
        (Some(f), Some(o), Some(fp)) if fp_ok(f) && fp_ok(fp) => Ok(Some(Position {
            file: f.to_string(),
            offset: o,
            fingerprint: fp.to_string(),
        })),
        _ => Err(invalid()),
    }
}

fn safe_identifier(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    if s.len() <= 128
        && !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
    {
        Some(s.to_string())
    } else {
        None
    }
}

fn is_fingerprint(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn valid_ts(ts: &str) -> bool {
    // ^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,3})?Z$ + Date.parse finite
    let b = ts.as_bytes();
    if b.len() < 20 || b.len() > 24 {
        return false;
    }
    let d = |i: usize| b.get(i).map_or(false, |c| c.is_ascii_digit());
    let dig = |i: usize| (b[i] - b'0') as u32;
    if !(d(0) && d(1) && d(2) && d(3) && b[4] == b'-' && d(5) && d(6) && b[7] == b'-'
        && d(8) && d(9) && b[10] == b'T' && d(11) && d(12) && b[13] == b':'
        && d(14) && d(15) && b[16] == b':' && d(17) && d(18))
    {
        return false;
    }
    let month = dig(5) * 10 + dig(6);
    let day = dig(8) * 10 + dig(9);
    let hour = dig(11) * 10 + dig(12);
    let min = dig(14) * 10 + dig(15);
    let sec = dig(17) * 10 + dig(18);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || min > 59 || sec > 60 {
        return false;
    }
    match &ts[19..] {
        "Z" => true,
        rest if rest.starts_with('.') && rest.ends_with('Z') => {
            let f = &rest[1..rest.len() - 1];
            !f.is_empty() && f.len() <= 3 && f.bytes().all(|c| c.is_ascii_digit())
        }
        _ => false,
    }
}

fn project_entry(line: &[u8], position: &Position) -> Option<Value> {
    let v: Value = serde_json::from_slice(line).ok()?;
    if !v.is_object() {
        return None;
    }
    let ts = v.get("ts")?.as_str()?;
    let tool = v.get("tool")?.as_str()?;
    let effect = v.get("effect")?.as_str()?;
    if !valid_ts(ts)
        || !(tool.starts_with("computer_")
            && tool.len() - 9 >= 1
            && tool.len() - 9 <= 80
            && tool[9..].bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))
        || !matches!(
            effect,
            "verified" | "dispatched-unknown" | "not-dispatched" | "refused" | "error"
        )
    {
        return None;
    }
    let code = v
        .get("code")
        .and_then(Value::as_str)
        .filter(|c| {
            let b = c.as_bytes();
            !b.is_empty()
                && b.len() <= 96
                && b[0].is_ascii_lowercase()
                && b.iter().all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || *x == b'_')
        })
        .map(String::from);
    let mut entry = serde_json::Map::new();
    entry.insert("id".into(), json!(digest(cursor_for(position).as_bytes())));
    entry.insert("ts".into(), json!(ts));
    entry.insert("tool".into(), json!(tool));
    entry.insert("effect".into(), json!(effect));
    if let Some(id) = safe_identifier(v.get("threadId")) {
        entry.insert("threadId".into(), json!(id));
    }
    if let Some(id) = safe_identifier(v.get("turnId")) {
        entry.insert("turnId".into(), json!(id));
    }
    if let Some(id) = safe_identifier(v.get("gatewayRequestId")) {
        entry.insert("gatewayRequestId".into(), json!(id));
    }
    if let Some(c) = code {
        entry.insert("code".into(), json!(c));
    }
    Some(Value::Object(entry))
}

// wsComputerHandlers.ts unsupported branch — mirrored verbatim for the
// no-backend front door. With the original running, everything except
// getAuditHistory relays to the real manager.
const UNSUPPORTED_MESSAGE: &str = "No computer backend is available on this server.";

fn unsupported_status() -> Value {
    json!({
        "computerId": "desktop",
        "availability": {"kind": "backend-unavailable", "message": UNSUPPORTED_MESSAGE},
        "health": {
            "status": "unavailable",
            "consecutiveFailures": 0,
            "reconnects": 0,
            "captureAvailable": false,
        },
        "capabilities": {
            "windows": false, "windowBounds": false, "stacking": false,
            "capture": false, "input": false, "clipboard": false,
            "focus": false, "raise": false, "ghostCursor": false,
            "visibleDesktop": false,
        },
    })
}

pub fn handles(tag: &str) -> bool {
    tag == "computer.getAuditHistory"
        || (std::env::var("SYNARA_NO_UPSTREAM").is_ok() && tag.starts_with("computer."))
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "computer.getAuditHistory" => get_audit_history(payload),
        "computer.getStatus" => Ok(unsupported_status()),
        "computer.getThreadState" => {
            let mut v = unsupported_status();
            let map = v.as_object_mut().expect("status object");
            map.insert(
                "threadId".into(),
                payload.get("threadId").cloned().unwrap_or(Value::Null),
            );
            map.insert("version".into(), json!(0));
            map.insert("windows".into(), json!([]));
            map.insert(
                "screenSize".into(),
                json!({"width": 1, "height": 1}),
            );
            map.insert("agentActive".into(), json!(false));
            map.insert("controlledByOtherThread".into(), json!(false));
            map.insert("lastError".into(), Value::Null);
            Ok(v)
        }
        _ if tag.starts_with("computer.") => Err(err(UNSUPPORTED_MESSAGE)),
        _ => Err(err("unhandled computer tag")),
    }
}

fn get_audit_history(payload: &Value) -> Result<Value, WsErr> {
    let limit = payload
        .get("limit")
        .and_then(Value::as_i64)
        .unwrap_or(30);
    // Reject non-integer limits the way the schema does.
    if payload.get("limit").is_some_and(|l| !l.is_i64() && !l.is_u64()) {
        return Err(err(
            "Computer activity history supports between 1 and 100 entries per page.",
        ));
    }
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(err(
            "Computer activity history supports between 1 and 100 entries per page.",
        ));
    }
    let before = parse_cursor(payload.get("before").and_then(Value::as_str))?;
    let file_path = Path::new(&crate::todo::state_dir()).join("computer-audit.jsonl");

    let mut records: Vec<(Position, Value)> = Vec::new();
    let mut seen_files: std::collections::HashSet<String> = Default::default();
    let mut remaining_bytes = MAX_BYTES;
    let mut scanned_rows = 0usize;
    let mut found_file = false;
    let mut truncated = false;
    for path in [
        file_path.clone(),
        Path::new(&format!("{}.1", file_path.display())).to_path_buf(),
        Path::new(&format!("{}.2", file_path.display())).to_path_buf(),
    ] {
        let bound = if scanned_rows < MAX_ROWS { remaining_bytes } else { 0 };
        let Some(tail) = read_tail(&path, bound)? else { continue };
        remaining_bytes = remaining_bytes.saturating_sub(tail.requested as usize);
        found_file = true;
        let file_id = digest(format!("{}:{}", tail.device, tail.inode).as_bytes());
        if !seen_files.insert(file_id.clone()) {
            continue;
        }
        truncated |= tail.start > 0
            || tail.contents.len() as u64 != tail.requested
            || tail.contents.last().is_some_and(|&b| b != b'\n');
        for (terminated, offset) in tail_lines(&tail) {
            if scanned_rows >= MAX_ROWS {
                truncated = true;
                break;
            }
            scanned_rows += 1;
            let line = &terminated[..terminated.len() - 1];
            if line.is_empty() {
                continue;
            }
            if line.len() > MAX_LINE_BYTES {
                truncated = true;
                continue;
            }
            let position = Position {
                file: file_id.clone(),
                offset,
                fingerprint: digest(line),
            };
            match project_entry(line, &position) {
                Some(e) => records.push((position, e)),
                None => truncated = true,
            }
        }
    }

    let status = if found_file { "available" } else { "missing" };
    let mut first = 0usize;
    if let Some(b) = before {
        let mut index = records.iter().position(|(p, _)| {
            p.file == b.file && p.offset == b.offset && p.fingerprint == b.fingerprint
        });
        if index.is_none() {
            let matches: Vec<usize> = records
                .iter()
                .enumerate()
                .filter(|(_, (p, _))| p.fingerprint == b.fingerprint)
                .map(|(i, _)| i)
                .collect();
            if matches.len() == 1 {
                index = Some(matches[0]);
            }
        }
        let Some(idx) = index else {
            return Ok(json!({"entries": [], "nextCursor": Value::Null, "truncated": true, "status": status}));
        };
        first = idx + 1;
    }
    let page: Vec<Value> = records
        .iter()
        .skip(first)
        .take(limit as usize)
        .map(|(_, e)| e.clone())
        .collect();
    let next_cursor = if !page.is_empty() && first + page.len() < records.len() {
        Value::from(cursor_for(&records[first + page.len() - 1].0))
    } else {
        Value::Null
    };
    Ok(json!({
        "entries": page,
        "nextCursor": next_cursor,
        "truncated": truncated,
        "status": status,
    }))
}

// is_fingerprint referenced in parse_cursor via fp_ok; kept for parity with the
// FINGERPRINT contract pattern.
#[allow(dead_code)]
fn _fingerprint_parity(s: &str) -> bool {
    is_fingerprint(s)
}
