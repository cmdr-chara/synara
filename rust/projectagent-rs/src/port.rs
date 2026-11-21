//! The Rust port of `packages/shared/src/projectAgent.ts`.

#![forbid(unsafe_code)]

use crate::{js, utf16};
use regex::Regex;
use serde_json::{Value, json};

const WORKER_INBOX_PREFIX: &str = "inbox/";
const MEMORY_DOCUMENT_PREFIX: &str = "memory/";
const MEMORY_AUTO_DOCUMENT_PATH: &str = "memory/MEMORY.md";
const MEMORY_THREAD_DOCUMENT_PREFIX: &str = "memory/threads/";
const MEMORY_NOTES_DOCUMENT_PREFIX: &str = "memory/notes/";
const INITIAL_PROJECT_DIGEST_SUMMARY: &str = "Coordinator is ready.";
const CONTEXT_BUDGET_CHARS: f64 = 32000.0;
const CONTEXT_PREVIEW_PATHS: [&str; 4] = [
    "instructions.md",
    "notes.md",
    "decisions.md",
    "docs/project-bot.md",
];

/// JS `\s` as a regex class: WhiteSpace + LineTerminator + ZWNBSP, no U+0085.
const JS_SPC: &str = "[\\t\\n\\x0b\\x0c\\r \\u{a0}\\u{1680}\\u{2000}-\\u{200a}\\u{2028}\\u{2029}\\u{202f}\\u{205f}\\u{3000}\\u{feff}]";

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// An ASCII-insensitive literal for a JS `i`-flag regex on ASCII text.
fn ai(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                format!("[{}{}]", c.to_ascii_lowercase(), c.to_ascii_uppercase())
            } else {
                regex::escape(&c.to_string())
            }
        })
        .collect()
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(js::undefined)),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        _ => Ok(js::undefined()),
    }
}

/// JS ToString for `${x}` and `String(x)` on the values these helpers see.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if matches!(e, Value::Null) || js::is_undefined(e) {
                    String::new()
                } else {
                    js_to_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".into()
            } else if let Some(n) = js::as_number(v) {
                js::number_to_string(n)
            } else {
                "[object Object]".into()
            }
        }
    }
}

/// JS `Number(string)` conversion.
fn js_number_parse(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // Hex/octal/binary literals take no sign in JS `Number`.
    let rest = t;
    if let Some(hex) = rest.strip_prefix("0x").or(rest.strip_prefix("0X")) {
        if !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return u64::from_str_radix(hex, 16)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
        }
        return f64::NAN;
    }
    if let Some(oct) = rest.strip_prefix("0o").or(rest.strip_prefix("0O")) {
        if !oct.is_empty() && oct.chars().all(|c| matches!(c, '0'..='7')) {
            return u64::from_str_radix(oct, 8)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
        }
        return f64::NAN;
    }
    if let Some(bin) = rest.strip_prefix("0b").or(rest.strip_prefix("0B")) {
        if !bin.is_empty() && bin.chars().all(|c| matches!(c, '0' | '1')) {
            return u64::from_str_radix(bin, 2)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
        }
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// JS ToNumber.
fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => u8::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_number_parse(s),
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => js_to_number(&a[0]),
            _ => f64::NAN,
        },
        Value::Object(_) => js::as_number(v).unwrap_or(f64::NAN),
    }
}

/// JS `slice(start, end)` argument coercion for length-style offsets.
fn to_offset(x: f64, len: usize) -> usize {
    if x.is_nan() {
        return 0;
    }
    if x.is_infinite() {
        return if x > 0.0 { len } else { 0 };
    }
    let i = x.trunc();
    if i < 0.0 {
        (len as f64 + i).max(0.0) as usize
    } else {
        (i as usize).min(len)
    }
}

/// `logicalPath.slice(prefix.len(), -3)` on UTF-16 units.
fn slice_mid(path: &str, prefix_len: usize) -> String {
    let units = utf16::units(path);
    let end = units.len().saturating_sub(3);
    if end < prefix_len {
        return String::new();
    }
    utf16::slice(&units, prefix_len, end)
}

fn is_prefixed_md(path: &str, prefix: &str) -> bool {
    path.starts_with(prefix)
        && path.ends_with(".md")
        && !slice_mid(path, prefix.len()).contains('/')
}

/// Port of `normalizeProjectDocumentPath` (`packages/shared/src/projectAgent.ts:22`).
pub fn normalize_project_document_path(raw_path: String) -> Result<String, String> {
    let trimmed = js_trim(&raw_path).replace('\\', "/");
    if trimmed.is_empty() {
        return Err("Document path is empty.".to_string());
    }
    let is_drive = trimmed.len() >= 2
        && trimmed.as_bytes()[0].is_ascii_alphabetic()
        && trimmed.as_bytes()[1] == b':';
    if trimmed.starts_with('/') || is_drive || trimmed.starts_with("//") {
        return Err("Document path must be relative.".to_string());
    }
    let segments: Vec<&str> = trimmed
        .split('/')
        .filter(|s| !s.is_empty() && *s != ".")
        .collect();
    if segments.is_empty() {
        return Err("Document path is empty.".to_string());
    }
    for segment in &segments {
        if *segment == ".." || segment.contains('\0') {
            return Err(format!("Document path rejects segment \"{segment}\"."));
        }
    }
    Ok(segments.join("/"))
}

/// Port of `isInboxDocumentPath` (`packages/shared/src/projectAgent.ts:42`).
pub fn is_inbox_document_path(logical_path: String) -> Result<bool, String> {
    Ok(logical_path.starts_with(WORKER_INBOX_PREFIX))
}

/// Port of `isUserOwnedDocumentPath` (`packages/shared/src/projectAgent.ts:46`).
pub fn is_user_owned_document_path(logical_path: String) -> Result<bool, String> {
    Ok(logical_path == "instructions.md" || logical_path == "notes.md")
}

/// Port of `isCoordinatorCuratedDocumentPath` (`packages/shared/src/projectAgent.ts:50`).
pub fn is_coordinator_curated_document_path(logical_path: String) -> Result<bool, String> {
    Ok(logical_path == "decisions.md" || logical_path.starts_with("docs/"))
}

/// Port of `isGeneratedDocumentPath` (`packages/shared/src/projectAgent.ts:54`).
pub fn is_generated_document_path(logical_path: String) -> Result<bool, String> {
    Ok(logical_path == "overview.md" || logical_path == "archived.md")
}

/// Port of `isMemoryDocumentPath` (`packages/shared/src/projectAgent.ts:63`).
pub fn is_memory_document_path(logical_path: String) -> Result<bool, String> {
    Ok(logical_path.starts_with(MEMORY_DOCUMENT_PREFIX))
}

/// Port of `memoryThreadDocumentPath` (`packages/shared/src/projectAgent.ts:67`).
pub fn memory_thread_document_path(thread_id: String) -> Result<String, String> {
    Ok(format!("{MEMORY_THREAD_DOCUMENT_PREFIX}{thread_id}.md"))
}

/// Port of `isMemoryThreadDocumentPath` (`packages/shared/src/projectAgent.ts:71`).
pub fn is_memory_thread_document_path(logical_path: String) -> Result<bool, String> {
    Ok(is_prefixed_md(&logical_path, MEMORY_THREAD_DOCUMENT_PREFIX))
}

/// Port of `memoryThreadIdFromDocumentPath` (`packages/shared/src/projectAgent.ts:79`).
pub fn memory_thread_id_from_document_path(logical_path: String) -> Result<Option<String>, String> {
    if !is_prefixed_md(&logical_path, MEMORY_THREAD_DOCUMENT_PREFIX) {
        return Ok(None);
    }
    Ok(Some(slice_mid(
        &logical_path,
        MEMORY_THREAD_DOCUMENT_PREFIX.len(),
    )))
}

/// Port of `isMemoryNoteDocumentPath` (`packages/shared/src/projectAgent.ts:85`).
pub fn is_memory_note_document_path(logical_path: String) -> Result<bool, String> {
    Ok(is_prefixed_md(&logical_path, MEMORY_NOTES_DOCUMENT_PREFIX))
}

fn starts_with_err(v: &Value, prefix: &str) -> Result<bool, String> {
    match v {
        Value::String(s) => Ok(s.starts_with(prefix)),
        Value::Null => Err("Cannot read properties of null (reading 'startsWith')".into()),
        _ if js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'startsWith')".into())
        }
        _ => Err("logicalPath.startsWith is not a function".into()),
    }
}

fn is_memory_note_path_value(v: &Value) -> Result<bool, String> {
    match v {
        Value::String(s) => Ok(is_prefixed_md(s, MEMORY_NOTES_DOCUMENT_PREFIX)),
        _ => starts_with_err(v, MEMORY_NOTES_DOCUMENT_PREFIX).map(|_| false),
    }
}

/// Port of `canWriteMemoryDocument` (`packages/shared/src/projectAgent.ts:93`).
pub fn can_write_memory_document(input: serde_json::Value) -> Result<bool, String> {
    let logical_path = member(&input, "logicalPath")?;
    if matches!(&logical_path, Value::String(s) if s == MEMORY_AUTO_DOCUMENT_PATH) {
        let kind = member(&input, "principalKind")?;
        return Ok(matches!(&kind, Value::String(s) if s == "user" || s == "coordinator"));
    }
    if is_memory_note_path_value(&logical_path)? {
        let kind = member(&input, "principalKind")?;
        return Ok(matches!(&kind, Value::String(s) if s == "user"));
    }
    let thread_id = match &logical_path {
        Value::String(s) if is_prefixed_md(s, MEMORY_THREAD_DOCUMENT_PREFIX) => {
            slice_mid(s, MEMORY_THREAD_DOCUMENT_PREFIX.len())
        }
        Value::String(_) => return Ok(false),
        _ => {
            starts_with_err(&logical_path, MEMORY_THREAD_DOCUMENT_PREFIX)?;
            return Ok(false);
        }
    };
    let kind = member(&input, "principalKind")?;
    if !matches!(&kind, Value::String(s) if s == "worker" || s == "coordinator" || s == "group-member")
    {
        return Ok(false);
    }
    let principal_thread = member(&input, "principalThreadId")?;
    Ok(matches!(&principal_thread, Value::String(s) if *s == thread_id))
}

/// Port of `isProjectContextPreviewPath` (`packages/shared/src/projectAgent.ts:123`).
pub fn is_project_context_preview_path(logical_path: String) -> Result<bool, String> {
    Ok(CONTEXT_PREVIEW_PATHS.contains(&logical_path.as_str()))
}

/// Port of `sanitizeProjectDigestSummary` (`packages/shared/src/projectAgent.ts:136`).
pub fn sanitize_project_digest_summary(summary: Option<String>) -> Result<Option<String>, String> {
    let Some(summary) = summary else {
        return Ok(None);
    };
    let mut next = js_trim(&summary).to_string();
    if next.is_empty() {
        return Ok(None);
    }
    let gates = [
        format!("{}\\.?", ai("Start a goal to begin bounded coordination")),
        format!(
            "{}\\.?",
            ai("Assigned work starts only after a goal is started")
        ),
        format!(
            "{JS_SPC}*[^.]*?(?-u:\\b){}(?-u:\\b)[^.]*?{}.*?{}\\.",
            ai("and"),
            ai("both reported failure, while "),
            ai("written")
        ),
        format!("{}\\.?", ai("No worker completion is confirmed")),
    ];
    for pattern in gates {
        let re = Regex::new(&pattern).map_err(|e| e.to_string())?;
        next = re.replace_all(&next, "").into_owned();
    }
    let multi = Regex::new(&format!("{JS_SPC}{{2,}}")).map_err(|e| e.to_string())?;
    next = multi.replace_all(&next, " ").into_owned();
    let spc_dot = Regex::new(&format!("{JS_SPC}+\\.")).map_err(|e| e.to_string())?;
    next = spc_dot.replace_all(&next, ".").into_owned();
    let next = js_trim(&next).to_string();
    if next == "Coordinator is configured." || next == "Coordinator is configured" {
        return Ok(Some(INITIAL_PROJECT_DIGEST_SUMMARY.into()));
    }
    Ok(Some(if next.is_empty() {
        INITIAL_PROJECT_DIGEST_SUMMARY.into()
    } else {
        next
    }))
}

/// Port of `sanitizeProjectDigestFocusTitle` (`packages/shared/src/projectAgent.ts:153`).
pub fn sanitize_project_digest_focus_title(title: String) -> Result<String, String> {
    let re = Regex::new(&format!(
        "{JS_SPC}+{}{}\\.?$",
        ai("reported"),
        ai(" failed")
    ))
    .map_err(|e| e.to_string())?;
    Ok(js_trim(re.replace(&title, "").as_ref()).to_string())
}

fn same_value_zero(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (
                x.as_f64().unwrap_or(f64::NAN),
                y.as_f64().unwrap_or(f64::NAN),
            );
            (x.is_nan() && y.is_nan()) || x == y
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => js::is_undefined(a) && js::is_undefined(b),
    }
}

/// Port of `detectProjectTaskDependencyCycle` (`packages/shared/src/projectAgent.ts:157`).
pub fn detect_project_task_dependency_cycle(input: serde_json::Value) -> Result<bool, String> {
    let edges = member(&input, "edges")?;
    let mut adjacency: Vec<(Value, Value)> = Vec::new();
    if let Value::Array(pairs) = &edges {
        for pair in pairs {
            if let Value::Array(kv) = pair {
                let key = kv.first().cloned().unwrap_or_else(js::undefined);
                let val = kv.get(1).cloned().unwrap_or_else(js::undefined);
                if let Some(slot) = adjacency.iter_mut().find(|(k, _)| same_value_zero(k, &key)) {
                    slot.1 = val;
                } else {
                    adjacency.push((key, val));
                }
            }
        }
    }
    let task_id = member(&input, "taskId")?;
    let depends = member(&input, "dependsOnTaskIds")?;
    adjacency.retain(|(k, _)| !same_value_zero(k, &task_id));
    adjacency.push((task_id.clone(), depends));

    let mut visiting: Vec<Value> = Vec::new();
    let mut visited: Vec<Value> = Vec::new();
    fn visit(
        node: &Value,
        adjacency: &[(Value, Value)],
        visiting: &mut Vec<Value>,
        visited: &mut Vec<Value>,
    ) -> bool {
        if visiting.iter().any(|v| same_value_zero(v, node)) {
            return true;
        }
        if visited.iter().any(|v| same_value_zero(v, node)) {
            return false;
        }
        visiting.push(node.clone());
        if let Some((_, nexts)) = adjacency.iter().find(|(k, _)| same_value_zero(k, node)) {
            // `for..of` over the stored value: arrays yield items, strings yield chars.
            let iter: Vec<Value> = match nexts {
                Value::Array(items) => items.clone(),
                Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
                Value::Null | Value::Object(_) => Vec::new(),
                _ => Vec::new(),
            };
            for next in iter {
                if visit(&next, adjacency, visiting, visited) {
                    return true;
                }
            }
        }
        visiting.retain(|v| !same_value_zero(v, node));
        visited.push(node.clone());
        false
    }
    Ok(visit(&task_id, &adjacency, &mut visiting, &mut visited))
}

/// Port of `truncateToContextBudget` (`packages/shared/src/projectAgent.ts:180`).
pub fn truncate_to_context_budget(
    sections: serde_json::Value,
    budget: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let items = match &sections {
        Value::Array(a) => a,
        Value::Null => return Err("Cannot read properties of null (reading 'map')".into()),
        _ if js::is_undefined(&sections) => {
            return Err("Cannot read properties of undefined (reading 'map')".into());
        }
        _ => return Err("sections.map is not a function".into()),
    };
    let mut full = String::new();
    for (i, section) in items.iter().enumerate() {
        let label = member(section, "label")?;
        let text = member(section, "text")?;
        if i > 0 {
            full.push_str("\n\n");
        }
        full.push_str("## ");
        full.push_str(&js_to_string(&label));
        full.push('\n');
        full.push_str(&js_to_string(&text));
    }
    let budget = if js::is_undefined(&budget) {
        CONTEXT_BUDGET_CHARS
    } else {
        js_to_number(&budget)
    };
    let full_len = utf16::units(&full).len();
    if (full_len as f64) <= budget {
        return Ok(json!({
            "packet": full,
            "characterCount": js::number(full_len as f64),
            "truncated": false,
        }));
    }
    let marker = "\n\n[truncated]";
    let keep = (budget - marker.len() as f64).max(0.0);
    let units = utf16::units(&full);
    let keep_n = to_offset(keep, units.len());
    let mut packet = utf16::slice(&units, 0, keep_n);
    packet.push_str(marker);
    let packet_units = utf16::units(&packet);
    let end = to_offset(budget, packet_units.len());
    let packet = utf16::slice(&packet_units, 0, end);
    // JS Math.min propagates NaN; f64::min does not.
    let character_count = if budget.is_nan() {
        f64::NAN
    } else {
        (packet_units.len() as f64).min(budget)
    };
    Ok(json!({
        "packet": packet,
        "characterCount": js::number(character_count),
        "truncated": true,
    }))
}

fn b64_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'-' => Some(62),
        b'_' => Some(63),
        _ => None,
    }
}

/// Lenient base64url decode as `Buffer.from(s, "base64url")`: non-alphabet bytes are
/// skipped, `=` ends the data and a trailing partial group of one char is dropped.
fn base64url_decode(s: &str) -> Vec<u8> {
    let mut chars = Vec::new();
    for c in s.chars() {
        if c == '=' {
            break;
        }
        if let Some(v) = c.is_ascii().then(|| b64_val(c as u8)).flatten() {
            chars.push(v);
        }
    }
    let mut out = Vec::with_capacity(chars.len() * 3 / 4);
    let (chunks, rem) = chars.as_chunks::<4>();
    for g in chunks {
        out.push((g[0] << 2) | (g[1] >> 4));
        out.push((g[1] << 4) | (g[2] >> 2));
        out.push((g[2] << 6) | g[3]);
    }
    if rem.len() == 3 {
        out.push((rem[0] << 2) | (rem[1] >> 4));
        out.push((rem[1] << 4) | (rem[2] >> 2));
    } else if rem.len() == 2 {
        out.push((rem[0] << 2) | (rem[1] >> 4));
    }
    out
}

fn base64url_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let (chunks, rem) = bytes.as_chunks::<3>();
    for g in chunks {
        let n = u32::from(g[0]) << 16 | u32::from(g[1]) << 8 | u32::from(g[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        out.push(ALPHABET[n as usize & 63] as char);
    }
    match rem.len() {
        1 => {
            let n = u32::from(rem[0]) << 16;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        }
        2 => {
            let n = u32::from(rem[0]) << 16 | u32::from(rem[1]) << 8;
            out.push(ALPHABET[(n >> 18) as usize & 63] as char);
            out.push(ALPHABET[(n >> 12) as usize & 63] as char);
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        _ => {}
    }
    out
}

/// Port of `encodeProjectAgentListCursor` (`packages/shared/src/projectAgent.ts:202`).
pub fn encode_project_agent_list_cursor(input: serde_json::Value) -> Result<String, String> {
    let sequence = member(&input, "sequence")?;
    let sequence = if js::is_undefined(&sequence) {
        String::new()
    } else {
        js_to_string(&sequence)
    };
    let created_at = js_to_string(&member(&input, "createdAt")?);
    let id = js_to_string(&member(&input, "id")?);
    Ok(base64url_encode(
        format!("{created_at}\t{id}\t{sequence}").as_bytes(),
    ))
}

/// Port of `decodeProjectAgentListCursor` (`packages/shared/src/projectAgent.ts:211`).
pub fn decode_project_agent_list_cursor(
    cursor: Option<String>,
) -> Result<Option<serde_json::Value>, String> {
    let Some(cursor) = cursor else {
        return Ok(None);
    };
    if cursor.is_empty() {
        return Ok(None);
    }
    let decoded = String::from_utf8_lossy(&base64url_decode(&cursor)).into_owned();
    let Some(split) = decoded.find('\t') else {
        return Ok(None);
    };
    if split == 0 {
        return Ok(None);
    }
    let remainder = &decoded[split + 1..];
    let (id, sequence_raw) = match remainder.find('\t') {
        None => (remainder, ""),
        Some(i) => (&remainder[..i], &remainder[i + 1..]),
    };
    let mut out = serde_json::Map::new();
    out.insert("createdAt".into(), decoded[..split].into());
    out.insert("id".into(), id.into());
    if !sequence_raw.is_empty() {
        let sequence = js_number_parse(sequence_raw);
        if sequence.is_finite() {
            out.insert("sequence".into(), js::number(sequence));
        }
    }
    Ok(Some(Value::Object(out)))
}
