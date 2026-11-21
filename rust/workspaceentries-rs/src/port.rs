//! The Rust port of `apps/server/src/workspaceEntries.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};

const WORKSPACE_CACHE_TTL_MS: i64 = 15_000;
const WORKSPACE_CACHE_MAX_KEYS: usize = 4;
const WORKSPACE_INDEX_MAX_ENTRIES: usize = 25_000;
const PROJECT_SCRIPT_DISCOVERY_DEFAULT_DEPTH: f64 = 2.0;
const PROJECT_PACKAGE_JSON_MAX_BYTES: u64 = 1024 * 1024;
const PROJECT_PACKAGE_SCAN_MAX_TARGETS: usize = 80;
const CONTENT_SEARCH_DEFAULT_LIMIT: f64 = 50.0;
const CONTENT_SEARCH_MAX_LIMIT: f64 = 100.0;
const CONTENT_SEARCH_MIN_QUERY_LENGTH: usize = 2;
const CONTENT_SEARCH_MAX_FILE_BYTES: u64 = 512 * 1024;
const CONTENT_SEARCH_MAX_MATCHES_PER_FILE: usize = 5;
const CONTENT_SEARCH_TIME_BUDGET_MS: u128 = 4_000;
const CONTENT_SEARCH_MAX_LINE_LENGTH: usize = 1024;
const CONTENT_SEARCH_BINARY_SNIFF_BYTES: usize = 8 * 1024;
const LOCAL_SEARCH_MAX_DEPTH: u32 = 6;
const LOCAL_SEARCH_DEFAULT_LIMIT: f64 = 50.0;
const LOCAL_SEARCH_TIME_BUDGET_MS: u128 = 600;

static IGNORED_DIRECTORY_NAMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        ".git",
        ".convex",
        "node_modules",
        ".next",
        ".turbo",
        "dist",
        "build",
        "out",
        ".cache",
    ]
    .into_iter()
    .collect()
});

static LOCAL_SEARCH_IGNORED_DIRECTORY_NAMES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| {
        [
            ".git",
            ".hg",
            ".svn",
            ".DS_Store",
            ".Trash",
            "node_modules",
            ".next",
            ".turbo",
            ".cache",
            ".convex",
            ".pnpm-store",
            ".yarn",
            ".gradle",
            ".m2",
            ".nuget",
            ".bundle",
            "Library",
            "Pods",
            "dist",
            "build",
            "out",
            "target",
            "vendor",
            "__pycache__",
            ".venv",
            "venv",
        ]
        .into_iter()
        .collect()
    });

struct WorkspaceIndex {
    scanned_at_ms: i64,
    entries: Vec<Value>,
    truncated: bool,
}

struct CacheState {
    indexes: HashMap<String, WorkspaceIndex>,
    order: Vec<String>,
    generations: HashMap<String, u64>,
}

static CACHE: LazyLock<Mutex<CacheState>> = LazyLock::new(|| {
    Mutex::new(CacheState {
        indexes: HashMap::new(),
        order: Vec::new(),
        generations: HashMap::new(),
    })
});

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn read_err(input: &Value, key: &str) -> Result<Value, String> {
    match input {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn path_arg_error() -> String {
    "The \"path\" argument must be of type string or an instance of Buffer or URL. Received undefined"
        .to_string()
}

fn scandir_error(path: &str) -> String {
    format!("ENOENT: no such file or directory, scandir '{path}'")
}

fn scan_error(cwd: &Value) -> String {
    let cwd_label = match cwd {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        v => crate::js::as_number(v)
            .map(crate::js::number_to_string)
            .unwrap_or_else(|| "undefined".to_string()),
    };
    let inner = match cwd {
        Value::String(s) => scandir_error(s),
        _ => path_arg_error(),
    };
    format!("Unable to scan workspace entries at '{cwd_label}': {inner}")
}

// --- posix path helpers -------------------------------------------------

fn to_posix(input: &str) -> String {
    input.split('\\').collect::<Vec<_>>().join("/")
}

fn posix_normalize(input: &str) -> String {
    let is_abs = input.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for seg in input.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if parts.last() == Some(&"..") || parts.is_empty() {
                    if !is_abs {
                        parts.push("..");
                    }
                } else {
                    parts.pop();
                }
            }
            s => parts.push(s),
        }
    }
    let trailing = input.ends_with('/') && input != "/";
    let mut out = String::new();
    if is_abs {
        out.push('/');
    }
    out.push_str(&parts.join("/"));
    if out.is_empty() {
        out.push('.');
    }
    if trailing && out != "/" {
        out.push('/');
    }
    out
}

fn posix_join(parts: &[&str]) -> String {
    posix_normalize(&parts.join("/"))
}

fn posix_resolve(parts: &[&str]) -> String {
    let mut resolved = String::new();
    for part in parts.iter().rev() {
        if part.is_empty() {
            continue;
        }
        resolved = if resolved.is_empty() {
            part.to_string()
        } else {
            format!("{part}/{resolved}")
        };
        if part.starts_with('/') {
            break;
        }
    }
    if !resolved.starts_with('/') {
        let cwd = std::env::current_dir()
            .map(|p| to_posix(&p.to_string_lossy()))
            .unwrap_or_else(|_| "/".to_string());
        resolved = format!("{cwd}/{resolved}");
    }
    posix_normalize(&resolved)
}

fn parent_path_of(input: &str) -> Option<String> {
    input.rfind('/').map(|i| input[..i].to_string())
}

fn basename_of(input: &str) -> String {
    match input.rfind('/') {
        None => input.to_string(),
        Some(i) => input[i + 1..].to_string(),
    }
}

fn posix_dirname(input: &str) -> String {
    let trimmed = input.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
    }
}

fn posix_relative(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0;
    while common < from_parts.len()
        && common < to_parts.len()
        && from_parts[common] == to_parts[common]
    {
        common += 1;
    }
    let mut out: Vec<String> = Vec::new();
    for _ in common..from_parts.len() {
        out.push("..".to_string());
    }
    for p in &to_parts[common..] {
        out.push(p.to_string());
    }
    out.join("/")
}

fn expand_home_path(input: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
    if input == "~" {
        return home;
    }
    if input.starts_with("~/") || input.starts_with("~\\") {
        return posix_join(&[&home, &input[2..]]);
    }
    input.to_string()
}

fn is_windows_drive_path(value: &str) -> bool {
    let b = value.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\')
}

fn is_windows_absolute_path(value: &str) -> bool {
    value.starts_with("\\\\") || is_windows_drive_path(value)
}

fn is_explicit_relative_path(value: &str) -> bool {
    value == "."
        || value == ".."
        || value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with(".\\")
        || value.starts_with("..\\")
}

fn is_workspace_relative_path_safe(value: &str) -> bool {
    let trimmed = js_trim(value);
    if trimmed.is_empty() || trimmed.starts_with('/') || is_windows_absolute_path(&trimmed) {
        return false;
    }
    trimmed
        .split(['/', '\\'])
        .all(|seg| seg != ".." && seg != ".")
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200A}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
            | '\u{FEFF}'
    )
}

fn js_trim(s: &str) -> String {
    s.trim_matches(is_js_ws).to_string()
}

// --- search scoring -----------------------------------------------------

fn rank_subsequence_match(value: &str, query: &str) -> Option<f64> {
    if query.is_empty() {
        return Some(0.0);
    }
    let v: Vec<char> = value.chars().collect();
    let q: Vec<char> = query.chars().collect();
    let mut qi = 0usize;
    let mut first: i64 = -1;
    let mut prev: i64 = -1;
    let mut gap = 0i64;
    for (vi, ch) in v.iter().enumerate() {
        if *ch != q[qi] {
            continue;
        }
        if first == -1 {
            first = vi as i64;
        }
        if prev != -1 {
            gap += vi as i64 - prev - 1;
        }
        prev = vi as i64;
        qi += 1;
        if qi == q.len() {
            let span = vi as i64 - first + 1 - q.len() as i64;
            let len_pen = (v.len() as i64 - q.len() as i64).min(64);
            return Some((first * 2 + gap * 3 + span + len_pen) as f64);
        }
    }
    None
}

fn to_searchable_entry(path: &str, kind: &str) -> Value {
    let normalized_path = path.to_lowercase();
    let depth = normalized_path.matches('/').count() as f64 + 1.0;
    let normalized_name = basename_of(&normalized_path);
    let mut obj = Map::new();
    obj.insert("path".into(), json!(path));
    obj.insert("kind".into(), json!(kind));
    if let Some(p) = parent_path_of(path) {
        obj.insert("parentPath".into(), json!(p));
    }
    obj.insert("normalizedPath".into(), json!(normalized_path));
    obj.insert("normalizedName".into(), json!(normalized_name));
    obj.insert("depth".into(), json!(depth));
    Value::Object(obj)
}

fn entry_str<'a>(entry: &'a Value, key: &str) -> &'a str {
    entry.get(key).and_then(Value::as_str).unwrap_or("")
}

fn entry_f64(entry: &Value, key: &str) -> f64 {
    entry.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn score_entry(entry: &Value, query: &str) -> Option<f64> {
    if query.is_empty() {
        return Some(if entry_str(entry, "kind") == "directory" {
            0.0
        } else {
            1.0
        });
    }
    let normalized_path = entry_str(entry, "normalizedPath");
    let normalized_name = entry_str(entry, "normalizedName");
    if normalized_name == query {
        return Some(0.0);
    }
    if normalized_path == query {
        return Some(1.0);
    }
    if normalized_name.starts_with(query) {
        return Some(2.0);
    }
    if normalized_name.contains(query) {
        return Some(3.0);
    }
    if let Some(s) = rank_subsequence_match(normalized_name, query) {
        return Some(100.0 + s);
    }
    if normalized_path.starts_with(query) {
        return Some(1000.0);
    }
    if normalized_path.contains(&format!("/{query}")) {
        return Some(1001.0);
    }
    if normalized_path.contains(query) {
        return Some(1002.0);
    }
    if let Some(s) = rank_subsequence_match(normalized_path, query) {
        return Some(1100.0 + s);
    }
    None
}

fn compare_ranked(a: &(f64, Value), b: &(f64, Value)) -> std::cmp::Ordering {
    if a.0 != b.0 {
        return a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal);
    }
    let da = entry_f64(&a.1, "depth");
    let db = entry_f64(&b.1, "depth");
    if da != db {
        return da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal);
    }
    entry_str(&a.1, "path").cmp(entry_str(&b.1, "path"))
}

fn insert_ranked(ranked: &mut Vec<(f64, Value)>, candidate: (f64, Value), limit: f64) {
    if limit <= 0.0 {
        return;
    }
    let idx = {
        let (mut lo, mut hi) = (0usize, ranked.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if compare_ranked(&candidate, &ranked[mid]) == std::cmp::Ordering::Less {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo
    };
    // JS: `rankedEntries.length < limit` — false when limit is NaN.
    if (ranked.len() as f64) < limit {
        ranked.insert(idx, candidate);
        return;
    }
    if idx as f64 >= limit {
        return;
    }
    ranked.insert(idx, candidate);
    ranked.pop();
}

// --- workspace index build ----------------------------------------------

fn is_path_in_ignored_dir(relative: &str) -> bool {
    match relative.split('/').next() {
        Some(first) if !first.is_empty() => IGNORED_DIRECTORY_NAMES.contains(first),
        _ => false,
    }
}

fn build_workspace_index(cwd: &str) -> Result<WorkspaceIndex, String> {
    let mut pending: Vec<String> = vec![String::new()];
    let mut entries: Vec<Value> = Vec::new();
    let mut truncated = false;
    let mut scanned_dirs = 0usize;

    while !pending.is_empty() && !truncated {
        let current = std::mem::take(&mut pending);
        for relative_dir in current {
            if truncated {
                break;
            }
            let absolute_dir = if relative_dir.is_empty() {
                cwd.to_string()
            } else {
                posix_join(&[cwd, &relative_dir])
            };
            let dirents = match std::fs::read_dir(&absolute_dir) {
                Ok(rd) => rd
                    .flatten()
                    .filter_map(|ent| {
                        let ft = ent.file_type().ok()?;
                        Some((
                            ent.file_name().to_string_lossy().to_string(),
                            ft.is_dir(),
                            ft.is_file(),
                        ))
                    })
                    .collect::<Vec<_>>(),
                Err(e) => {
                    if relative_dir.is_empty() {
                        return Err(if e.kind() == std::io::ErrorKind::NotFound {
                            format!(
                                "Unable to scan workspace entries at '{cwd}': {}",
                                scandir_error(cwd)
                            )
                        } else {
                            format!("Unable to scan workspace entries at '{cwd}': {e}")
                        });
                    }
                    continue;
                }
            };
            let mut dirents = dirents;
            dirents.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, is_dir, is_file) in dirents {
                if name.is_empty() || name == "." || name == ".." {
                    continue;
                }
                if is_dir && IGNORED_DIRECTORY_NAMES.contains(name.as_str()) {
                    continue;
                }
                if !is_dir && !is_file {
                    continue;
                }
                let relative_path = to_posix(&if relative_dir.is_empty() {
                    name.clone()
                } else {
                    posix_join(&[&relative_dir, &name])
                });
                if is_path_in_ignored_dir(&relative_path) {
                    continue;
                }
                entries.push(to_searchable_entry(
                    &relative_path,
                    if is_dir { "directory" } else { "file" },
                ));
                if is_dir {
                    pending.push(relative_path.clone());
                }
                if entries.len() >= WORKSPACE_INDEX_MAX_ENTRIES {
                    truncated = true;
                    break;
                }
            }
            scanned_dirs += 1;
            if scanned_dirs > 200_000 {
                truncated = true;
                break;
            }
        }
    }

    Ok(WorkspaceIndex {
        scanned_at_ms: now_ms(),
        entries,
        truncated,
    })
}

fn get_workspace_index(cwd: &Value) -> Result<WorkspaceIndex, String> {
    let cwd_str = match cwd {
        Value::String(s) => s.clone(),
        _ => return Err(scan_error(cwd)),
    };
    {
        let cache = CACHE.lock().map_err(|_| "cache lock".to_string())?;
        if let Some(index) = cache.indexes.get(&cwd_str)
            && now_ms() - index.scanned_at_ms < WORKSPACE_CACHE_TTL_MS
        {
            return Ok(WorkspaceIndex {
                scanned_at_ms: index.scanned_at_ms,
                entries: index.entries.clone(),
                truncated: index.truncated,
            });
        }
    }
    let built = build_workspace_index(&cwd_str)?;
    let mut cache = CACHE.lock().map_err(|_| "cache lock".to_string())?;
    cache.indexes.insert(
        cwd_str.clone(),
        WorkspaceIndex {
            scanned_at_ms: built.scanned_at_ms,
            entries: built.entries.clone(),
            truncated: built.truncated,
        },
    );
    if !cache.order.contains(&cwd_str) {
        cache.order.push(cwd_str.clone());
    }
    while cache.indexes.len() > WORKSPACE_CACHE_MAX_KEYS {
        let Some(oldest) = cache.order.first().cloned() else {
            break;
        };
        cache.indexes.remove(&oldest);
        cache.order.remove(0);
    }
    Ok(built)
}

// --- query normalizers --------------------------------------------------

fn normalize_entry_search_query(input: &str) -> String {
    js_trim(input)
        .trim_start_matches(['@', '.', '/'])
        .to_lowercase()
}

fn normalize_local_search_query(input: &str) -> String {
    let mut query = js_trim(input);
    loop {
        if let Some(rest) = query.strip_prefix("./") {
            query = rest.to_string();
        } else if query.starts_with('@') || query.starts_with('/') {
            query = query[1..].to_string();
        } else {
            break;
        }
    }
    query.to_lowercase()
}

// --- exports ------------------------------------------------------------

/// Port of `clearWorkspaceIndexCache` (`apps/server/src/workspaceEntries.ts:811`).
pub fn clear_workspace_index_cache(cwd: String) -> Result<(), String> {
    let mut cache = CACHE.lock().map_err(|_| "cache lock".to_string())?;
    cache.indexes.remove(&cwd);
    cache.order.retain(|k| k != &cwd);
    *cache.generations.entry(cwd).or_insert(0) += 1;
    Ok(())
}

/// Port of `prewarmWorkspaceSearchIndex` (`apps/server/src/workspaceEntries.ts:821`).
pub fn prewarm_workspace_search_index(input: Value) -> Result<Value, String> {
    read_err(&input, "cwd")?;
    Ok(json!({ "started": true }))
}

/// Port of `browseWorkspaceEntries` (`apps/server/src/workspaceEntries.ts:854`).
pub async fn browse_workspace_entries(input: Value) -> Result<Value, String> {
    let partial_path_v = read_err(&input, "partialPath")?;
    let partial_path = match &partial_path_v {
        Value::String(s) => s.clone(),
        _ if crate::js::is_undefined(&partial_path_v) => {
            return Err("Cannot read properties of undefined (reading 'startsWith')".to_string());
        }
        _ => return Err("Cannot read properties of undefined (reading 'startsWith')".to_string()),
    };
    if is_windows_absolute_path(&partial_path) {
        return Err("Windows-style paths are only supported on Windows.".to_string());
    }
    let resolved = if !is_explicit_relative_path(&partial_path) {
        posix_resolve(&[&expand_home_path(&partial_path)])
    } else {
        let cwd_v = read_err(&input, "cwd")?;
        let cwd = cwd_v.as_str().unwrap_or("");
        if cwd.is_empty() {
            return Err("Relative filesystem browse paths require a current project.".to_string());
        }
        posix_resolve(&[&expand_home_path(cwd), &partial_path])
    };
    let ends_with_sep =
        partial_path.ends_with('/') || partial_path.ends_with('\\') || partial_path == "~";
    let parent_path = if ends_with_sep {
        resolved.clone()
    } else {
        posix_dirname(&resolved)
    };
    let prefix = if ends_with_sep {
        String::new()
    } else {
        basename_of(&resolved)
    };

    let dirents = match std::fs::read_dir(&parent_path) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|ent| {
                let ft = ent.file_type().ok()?;
                Some((ent.file_name().to_string_lossy().to_string(), ft.is_dir()))
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            return Err(if e.kind() == std::io::ErrorKind::NotFound {
                scandir_error(&parent_path)
            } else {
                e.to_string()
            });
        }
    };
    let show_hidden = ends_with_sep || prefix.starts_with('.');
    let lower_prefix = prefix.to_lowercase();
    let mut entries: Vec<(String, String)> = dirents
        .into_iter()
        .filter(|(name, is_dir)| {
            *is_dir
                && name.to_lowercase().starts_with(&lower_prefix)
                && (show_hidden || !name.starts_with('.'))
        })
        .map(|(name, _)| (name.clone(), posix_join(&[&parent_path, &name])))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let arr: Vec<Value> = entries
        .into_iter()
        .map(|(name, full)| json!({ "name": name, "fullPath": full }))
        .collect();
    Ok(json!({ "parentPath": parent_path, "entries": arr }))
}

/// Port of `searchWorkspaceEntries` (`apps/server/src/workspaceEntries.ts:884`).
pub async fn search_workspace_entries(input: Value) -> Result<Value, String> {
    let cwd = read_err(&input, "cwd")?;
    let index = get_workspace_index(&cwd)?;
    let query_v = read_err(&input, "query")?;
    let query_raw = match &query_v {
        Value::String(s) => s.clone(),
        _ => return Err("Cannot read properties of undefined (reading 'trim')".to_string()),
    };
    let normalized_query = normalize_entry_search_query(&query_raw);
    let kind = read_err(&input, "kind")?;
    let kind_str = kind.as_str().map(str::to_string);
    let limit_v = read_err(&input, "limit")?;
    let limit = crate::js::as_number(&limit_v)
        .map(f64::floor)
        .unwrap_or(f64::NAN)
        .max(0.0);
    let limit = if limit.is_nan() { f64::NAN } else { limit };

    let mut ranked: Vec<(f64, Value)> = Vec::new();
    let mut matched = 0usize;
    for entry in &index.entries {
        if let Some(k) = &kind_str
            && entry_str(entry, "kind") != k
        {
            continue;
        }
        let Some(score) = score_entry(entry, &normalized_query) else {
            continue;
        };
        matched += 1;
        insert_ranked(&mut ranked, (score, entry.clone()), limit);
    }
    let out: Vec<Value> = ranked.into_iter().map(|(_, e)| e).collect();
    Ok(json!({
        "entries": out,
        "truncated": index.truncated || (matched as f64) > limit,
    }))
}

fn build_content_line_text(line: &str) -> String {
    let trimmed = line.trim_end_matches([' ', '\t', '\r']);
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= CONTENT_SEARCH_MAX_LINE_LENGTH {
        return trimmed.to_string();
    }
    let cut: String = chars[..CONTENT_SEARCH_MAX_LINE_LENGTH - 1].iter().collect();
    format!("{cut}…")
}

fn search_file_content(cwd: &str, relative: &str, query: &str) -> Option<Vec<Value>> {
    let absolute = posix_join(&[cwd, relative]);
    let real_root = std::fs::canonicalize(cwd).ok()?;
    let real_target = std::fs::canonicalize(&absolute).ok()?;
    let rel = posix_relative(
        &to_posix(&real_root.to_string_lossy()),
        &to_posix(&real_target.to_string_lossy()),
    );
    if rel == ".." || rel.starts_with("../") || rel.starts_with('/') {
        return None;
    }
    let meta = std::fs::metadata(&real_target).ok()?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > CONTENT_SEARCH_MAX_FILE_BYTES {
        return None;
    }
    let bytes = std::fs::read(&real_target).ok()?;
    let sniff = &bytes[..bytes.len().min(CONTENT_SEARCH_BINARY_SNIFF_BYTES)];
    if sniff.contains(&0) {
        return None;
    }
    let contents = String::from_utf8_lossy(&bytes);
    if !contents.to_lowercase().contains(query) {
        return Some(Vec::new());
    }
    let mut matches: Vec<Value> = Vec::new();
    for (idx, line) in contents.split('\n').enumerate() {
        if line.is_empty() || !line.to_lowercase().contains(query) {
            continue;
        }
        matches.push(json!({
            "path": relative,
            "lineNumber": (idx + 1) as f64,
            "lineText": build_content_line_text(line),
        }));
        if matches.len() >= CONTENT_SEARCH_MAX_MATCHES_PER_FILE {
            break;
        }
    }
    Some(matches)
}

/// Port of `searchWorkspaceContent` (`apps/server/src/workspaceEntries.ts:1009`).
pub async fn search_workspace_content(input: Value) -> Result<Value, String> {
    let query_v = read_err(&input, "query")?;
    let query_raw = match &query_v {
        Value::String(s) => s.clone(),
        _ => return Err("Cannot read properties of undefined (reading 'trim')".to_string()),
    };
    let normalized_query = js_trim(&query_raw).to_lowercase();
    if normalized_query.len() < CONTENT_SEARCH_MIN_QUERY_LENGTH {
        return Ok(json!({ "matches": [], "truncated": false }));
    }
    let limit_v = read_err(&input, "limit")?;
    let limit_raw = crate::js::as_number(&limit_v).unwrap_or(CONTENT_SEARCH_DEFAULT_LIMIT);
    let limit = limit_raw.clamp(1.0, CONTENT_SEARCH_MAX_LIMIT);

    let cwd = read_err(&input, "cwd")?;
    let index = get_workspace_index(&cwd)?;
    let cwd_str = cwd.as_str().unwrap_or("").to_string();
    let file_paths: Vec<String> = index
        .entries
        .iter()
        .filter(|e| entry_str(e, "kind") == "file")
        .map(|e| entry_str(e, "path").to_string())
        .collect();

    let deadline = std::time::Instant::now()
        + std::time::Duration::from_millis(CONTENT_SEARCH_TIME_BUDGET_MS as u64);
    let mut collected: Vec<Value> = Vec::new();
    let mut scanned = 0usize;
    let mut truncated = index.truncated;
    for path in &file_paths {
        if std::time::Instant::now() > deadline {
            truncated = true;
            break;
        }
        scanned += 1;
        if let Some(ms) = search_file_content(&cwd_str, path, &normalized_query) {
            collected.extend(ms);
        }
    }
    if std::time::Instant::now() > deadline {
        truncated = true;
    }
    let mut keyed: Vec<(String, f64, Value)> = collected
        .into_iter()
        .map(|m| {
            (
                entry_str(&m, "path").to_string(),
                entry_f64(&m, "lineNumber"),
                m,
            )
        })
        .collect();
    keyed.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
    });
    let total = keyed.len();
    let ordered: Vec<Value> = keyed
        .into_iter()
        .take(if limit.is_nan() { 0 } else { limit as usize })
        .map(|(_, _, m)| m)
        .collect();
    Ok(json!({
        "matches": ordered,
        "truncated": truncated || (total as f64) > limit || scanned < file_paths.len(),
    }))
}

fn normalized_file_reference(reference: &str) -> Option<String> {
    let trimmed = js_trim(reference);
    if !is_workspace_relative_path_safe(&trimmed) {
        return None;
    }
    let normalized = posix_normalize(&to_posix(&trimmed))
        .trim_start_matches("./")
        .to_string();
    if !normalized.is_empty() && normalized != "." {
        Some(normalized)
    } else {
        None
    }
}

fn file_paths_by_basename(index: &WorkspaceIndex) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for entry in &index.entries {
        if entry_str(entry, "kind") != "file" {
            continue;
        }
        let path = entry_str(entry, "path").to_string();
        map.entry(basename_of(&path)).or_default().push(path);
    }
    map
}

fn resolve_reference_from_index(
    reference: &Value,
    by_basename: &HashMap<String, Vec<String>>,
) -> Value {
    let Some(reference_str) = reference.as_str() else {
        return Value::Null;
    };
    let Some(normalized) = normalized_file_reference(reference_str) else {
        return Value::Null;
    };
    let basename = basename_of(&normalized);
    let Some(candidates) = by_basename.get(&basename) else {
        return Value::Null;
    };
    let suffix = format!("/{normalized}");
    let mut found: Option<&String> = None;
    for candidate in candidates {
        if candidate != &normalized && !candidate.ends_with(&suffix) {
            continue;
        }
        if found.is_some() {
            return Value::Null;
        }
        found = Some(candidate);
    }
    match found {
        Some(m) => json!(m),
        None => Value::Null,
    }
}

/// Port of `resolveWorkspaceFileReferences` (`apps/server/src/workspaceEntries.ts:1154`).
pub async fn resolve_workspace_file_references(input: Value) -> Result<Value, String> {
    let cwd = read_err(&input, "cwd")?;
    let index = get_workspace_index(&cwd)?;
    let references = read_err(&input, "relativePaths")?;
    let refs: Vec<Value> = match &references {
        Value::Array(a) => a.clone(),
        _ if crate::js::is_undefined(&references) => {
            return Err("Cannot read properties of undefined (reading 'map')".to_string());
        }
        _ => return Err("input.relativePaths.map is not a function".to_string()),
    };
    let by_basename = file_paths_by_basename(&index);
    let resolved: Vec<Value> = refs
        .iter()
        .map(|r| resolve_reference_from_index(r, &by_basename))
        .collect();
    Ok(json!({ "relativePaths": resolved }))
}

/// Port of `resolveWorkspaceFileBySuffix` (`apps/server/src/workspaceEntries.ts:1166`).
pub async fn resolve_workspace_file_by_suffix(input: Value) -> Result<Option<String>, String> {
    let cwd = read_err(&input, "cwd")?;
    let rel = read_err(&input, "relativePath")?;
    let inner = json!({ "cwd": cwd, "relativePaths": [rel] });
    let result = resolve_workspace_file_references(inner).await?;
    Ok(result
        .get("relativePaths")
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .map(str::to_string))
}

/// Port of `discoverProjectScripts` (`apps/server/src/workspaceEntries.ts:1177`).
pub async fn discover_project_scripts(input: Value) -> Result<Value, String> {
    let cwd_v = read_err(&input, "cwd")?;
    let cwd_raw = match &cwd_v {
        Value::String(s) => s.clone(),
        _ if crate::js::is_undefined(&cwd_v) => {
            return Err("Cannot read properties of undefined (reading 'startsWith')".to_string());
        }
        _ => return Err("input.startsWith is not a function".to_string()),
    };
    let cwd = posix_resolve(&[&expand_home_path(&cwd_raw)]);
    let depth_v = read_err(&input, "depth")?;
    let depth_raw =
        crate::js::as_number(&depth_v).unwrap_or(PROJECT_SCRIPT_DISCOVERY_DEFAULT_DEPTH);
    let max_depth = depth_raw.floor().clamp(0.0, 3.0);

    let mut candidates: Vec<(String, String)> = Vec::new();
    let mut pending: Vec<(String, String, u32)> = vec![(cwd.clone(), String::new(), 0)];
    while !pending.is_empty() && candidates.len() < PROJECT_PACKAGE_SCAN_MAX_TARGETS {
        let current = std::mem::take(&mut pending);
        let mut next: Vec<(String, String, u32)> = Vec::new();
        for (absolute_dir, relative_path, depth) in current {
            if candidates.len() >= PROJECT_PACKAGE_SCAN_MAX_TARGETS {
                break;
            }
            let Ok(rd) = std::fs::read_dir(&absolute_dir) else {
                continue;
            };
            let mut dirents: Vec<(String, bool, bool)> = rd
                .flatten()
                .filter_map(|ent| {
                    let ft = ent.file_type().ok()?;
                    Some((
                        ent.file_name().to_string_lossy().to_string(),
                        ft.is_dir(),
                        ft.is_file(),
                    ))
                })
                .collect();
            if dirents.iter().any(|(n, _, f)| *f && n == "package.json") {
                candidates.push((absolute_dir.clone(), relative_path.clone()));
            }
            if (depth as f64) >= max_depth {
                continue;
            }
            dirents.sort_by(|a, b| a.0.cmp(&b.0));
            for (name, is_dir, _) in dirents {
                if !is_dir || IGNORED_DIRECTORY_NAMES.contains(name.as_str()) {
                    continue;
                }
                if name == "." || name == ".." {
                    continue;
                }
                let child_relative = to_posix(&if relative_path.is_empty() {
                    name.clone()
                } else {
                    posix_join(&[&relative_path, &name])
                });
                if is_path_in_ignored_dir(&child_relative) {
                    continue;
                }
                next.push((
                    posix_join(&[&absolute_dir, &name]),
                    child_relative,
                    depth + 1,
                ));
            }
        }
        pending = next;
    }

    let lockfiles: [(&str, &[&str]); 4] = [
        ("bun", &["bun.lock", "bun.lockb"]),
        ("pnpm", &["pnpm-lock.yaml"]),
        ("yarn", &["yarn.lock"]),
        ("npm", &["package-lock.json", "npm-shrinkwrap.json"]),
    ];
    let mut targets: Vec<(String, Value)> = Vec::new();
    for (absolute_dir, relative_path) in candidates {
        let package_json_path = Path::new(&absolute_dir).join("package.json");
        let Ok(meta) = std::fs::metadata(&package_json_path) else {
            continue;
        };
        if !meta.is_file() || meta.len() > PROJECT_PACKAGE_JSON_MAX_BYTES {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&package_json_path) else {
            continue;
        };
        let Ok(parsed) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(scripts_obj) = parsed.get("scripts").and_then(Value::as_object) else {
            continue;
        };
        let manager = lockfiles
            .iter()
            .find(|(_, names)| {
                names
                    .iter()
                    .any(|n| Path::new(&absolute_dir).join(n).exists())
            })
            .map(|(m, _)| *m)
            .unwrap_or("npm");
        let mut scripts: Vec<(String, String)> = Vec::new();
        for (name, command) in scripts_obj {
            let trimmed_name = js_trim(name);
            if !command.is_string() || trimmed_name.is_empty() {
                continue;
            }
            let Some(command_str) = command.as_str() else {
                continue;
            };
            if js_trim(command_str).is_empty() {
                continue;
            }
            let full = if manager == "yarn" {
                format!("yarn {trimmed_name}")
            } else {
                format!("{manager} run {trimmed_name}")
            };
            scripts.push((trimmed_name, full));
        }
        scripts.sort_by(|a, b| a.0.cmp(&b.0));
        if scripts.is_empty() {
            continue;
        }
        let package_name = parsed
            .get("name")
            .and_then(Value::as_str)
            .map(js_trim)
            .filter(|s| !s.is_empty());
        let mut target = Map::new();
        target.insert("cwd".into(), json!(absolute_dir));
        target.insert("relativePath".into(), json!(relative_path));
        target.insert(
            "packageJsonPath".into(),
            json!(package_json_path.to_string_lossy()),
        );
        if let Some(name) = package_name {
            target.insert("packageName".into(), json!(name));
        }
        target.insert(
            "scripts".into(),
            Value::Array(
                scripts
                    .into_iter()
                    .map(|(name, command)| json!({ "name": name, "command": command }))
                    .collect(),
            ),
        );
        targets.push((relative_path.clone(), Value::Object(target)));
    }
    targets.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(json!({
        "targets": targets.into_iter().map(|(_, t)| t).collect::<Vec<_>>(),
    }))
}

fn is_contained_path(root: &str, candidate: &str) -> bool {
    let rel = posix_relative(root, candidate);
    rel.is_empty() || (rel != ".." && !rel.starts_with("../") && !rel.starts_with('/'))
}

fn realpath(path: &str) -> Result<String, String> {
    std::fs::canonicalize(path)
        .map(|p| to_posix(&p.to_string_lossy()))
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                format!("ENOENT: no such file or directory, realpath '{path}'")
            } else {
                e.to_string()
            }
        })
}

fn resolve_real_path_within_root(root: &Value, target: &Value) -> Result<Option<String>, String> {
    let (Value::String(root_str), Value::String(target_str)) = (root, target) else {
        return Err(path_arg_error());
    };
    let real_root = realpath(root_str)?;
    let real_target = realpath(target_str)?;
    Ok(if is_contained_path(&real_root, &real_target) {
        Some(real_target)
    } else {
        None
    })
}

fn resolve_directory_within_root(cwd: &str, relative: &str) -> Result<String, String> {
    if relative.starts_with('/') || is_windows_absolute_path(relative) {
        return Err("Directory path is outside the workspace root.".to_string());
    }
    let absolute = posix_resolve(&[cwd, relative]);
    let rel = posix_relative(cwd, &absolute);
    if rel == ".." || rel.starts_with("../") || rel.starts_with('/') {
        return Err("Directory path is outside the workspace root.".to_string());
    }
    Ok(absolute)
}

/// Port of `listWorkspaceDirectories` (`apps/server/src/workspaceEntries.ts:1231`).
pub async fn list_workspace_directories(input: Value) -> Result<Value, String> {
    let relative_v = read_err(&input, "relativePath")?;
    let relative_path = relative_v.as_str().map(js_trim).unwrap_or_default();
    let cwd = read_err(&input, "cwd")?;
    let resolved_target = if relative_path.is_empty() {
        cwd.clone()
    } else {
        let cwd_str = match &cwd {
            Value::String(s) => s.clone(),
            _ => return Err(path_arg_error()),
        };
        json!(resolve_directory_within_root(&cwd_str, &relative_path)?)
    };
    let Some(target_directory) = resolve_real_path_within_root(&cwd, &resolved_target)? else {
        return Err("Directory path is outside the workspace root.".to_string());
    };
    let cwd_str = cwd.as_str().unwrap_or("");
    let include_files = matches!(read_err(&input, "includeFiles")?, Value::Bool(true));

    let mut dirents = match std::fs::read_dir(&target_directory) {
        Ok(rd) => rd
            .flatten()
            .filter_map(|ent| {
                let ft = ent.file_type().ok()?;
                Some((
                    ent.file_name().to_string_lossy().to_string(),
                    ft.is_dir(),
                    ft.is_file(),
                ))
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            return Err(if e.kind() == std::io::ErrorKind::NotFound {
                scandir_error(&target_directory)
            } else {
                e.to_string()
            });
        }
    };
    dirents.retain(|(name, is_dir, is_file)| {
        !name.is_empty()
            && name != "."
            && name != ".."
            && name != ".git"
            && (*is_dir || (include_files && *is_file))
    });
    dirents.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut entries: Vec<Value> = Vec::new();
    for (name, is_dir, _) in dirents {
        let child_relative = to_posix(&if relative_path.is_empty() {
            name.clone()
        } else {
            posix_join(&[&relative_path, &name])
        });
        let mut obj = Map::new();
        obj.insert("path".into(), json!(child_relative));
        obj.insert("name".into(), json!(name));
        obj.insert(
            "kind".into(),
            json!(if is_dir { "directory" } else { "file" }),
        );
        if !relative_path.is_empty() {
            obj.insert("parentPath".into(), json!(relative_path));
        }
        if is_dir {
            let child_abs = PathBuf::from(cwd_str).join(&child_relative);
            let has_children = std::fs::read_dir(&child_abs)
                .map(|rd| {
                    rd.flatten().any(|e| {
                        e.file_name() != "."
                            && e.file_name() != ".."
                            && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            obj.insert("hasChildren".into(), json!(has_children));
        }
        entries.push(Value::Object(obj));
    }
    Ok(json!({ "entries": entries }))
}

fn score_local_name(name: &str, query: &str) -> Option<f64> {
    let normalized = name.to_lowercase();
    if normalized == query {
        return Some(0.0);
    }
    if normalized.starts_with(query) {
        return Some(2.0);
    }
    if normalized.contains(query) {
        return Some(5.0);
    }
    rank_subsequence_match(&normalized, query).map(|s| 100.0 + s)
}

fn insert_ranked_local(ranked: &mut Vec<(f64, Value)>, candidate: (f64, Value), limit: f64) {
    if limit <= 0.0 {
        return;
    }
    let idx = {
        let (mut lo, mut hi) = (0usize, ranked.len());
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            let current = &ranked[mid];
            let ord = candidate
                .0
                .partial_cmp(&current.0)
                .unwrap_or(std::cmp::Ordering::Equal);
            let less = if ord == std::cmp::Ordering::Equal {
                entry_str(&candidate.1, "path") < entry_str(&current.1, "path")
            } else {
                ord == std::cmp::Ordering::Less
            };
            if less {
                hi = mid;
            } else {
                lo = mid + 1;
            }
        }
        lo
    };
    if (ranked.len() as f64) < limit {
        ranked.insert(idx, candidate);
        return;
    }
    if idx as f64 >= limit {
        return;
    }
    ranked.insert(idx, candidate);
    ranked.pop();
}

/// Port of `searchLocalEntries` (`apps/server/src/workspaceEntries.ts:1376`).
pub async fn search_local_entries(input: Value) -> Result<Value, String> {
    let query_v = read_err(&input, "query")?;
    let query_raw = match &query_v {
        Value::String(s) => s.clone(),
        _ => return Err("Cannot read properties of undefined (reading 'trim')".to_string()),
    };
    let normalized_query = normalize_local_search_query(&query_raw);
    if normalized_query.is_empty() {
        return Ok(json!({ "entries": [], "truncated": false }));
    }
    let limit_v = read_err(&input, "limit")?;
    let limit_raw = crate::js::as_number(&limit_v).unwrap_or(LOCAL_SEARCH_DEFAULT_LIMIT);
    let limit = limit_raw.clamp(1.0, LOCAL_SEARCH_DEFAULT_LIMIT);
    let include_files = !matches!(read_err(&input, "includeFiles")?, Value::Bool(false));
    let include_dotfiles = normalized_query.starts_with('.');
    let root = read_err(&input, "rootPath")?;
    let root_str = root.as_str().unwrap_or("").to_string();

    let deadline = std::time::Instant::now()
        + std::time::Duration::from_millis(LOCAL_SEARCH_TIME_BUDGET_MS as u64);
    let mut ranked: Vec<(f64, Value)> = Vec::new();
    let mut truncated = false;
    let mut level: Vec<(String, u32)> = vec![(root_str, 0)];

    while !level.is_empty() {
        if std::time::Instant::now() > deadline {
            truncated = true;
            break;
        }
        let mut next: Vec<(String, u32)> = Vec::new();
        for (absolute_path, depth) in level {
            if std::time::Instant::now() > deadline {
                truncated = true;
                break;
            }
            let Ok(rd) = std::fs::read_dir(&absolute_path) else {
                continue;
            };
            for ent in rd.flatten() {
                let name = ent.file_name().to_string_lossy().to_string();
                if name.is_empty() || name == "." || name == ".." {
                    continue;
                }
                if LOCAL_SEARCH_IGNORED_DIRECTORY_NAMES.contains(name.as_str()) {
                    continue;
                }
                if !include_dotfiles && name.starts_with('.') {
                    continue;
                }
                let Ok(ft) = ent.file_type() else {
                    continue;
                };
                let (is_dir, is_file) = (ft.is_dir(), ft.is_file());
                if !is_dir && !is_file {
                    continue;
                }
                if !include_files && !is_dir {
                    continue;
                }
                let child_abs = format!("{absolute_path}/{name}");
                if let Some(score) = score_local_name(&name, &normalized_query) {
                    let entry = json!({
                        "path": child_abs,
                        "name": name,
                        "kind": if is_dir { "directory" } else { "file" },
                        "parentPath": absolute_path,
                    });
                    insert_ranked_local(&mut ranked, (score, entry), limit);
                }
                if is_dir && depth + 1 < LOCAL_SEARCH_MAX_DEPTH {
                    next.push((child_abs, depth + 1));
                }
            }
        }
        level = next;
    }
    let out: Vec<Value> = ranked.into_iter().map(|(_, e)| e).collect();
    Ok(json!({ "entries": out, "truncated": truncated }))
}
