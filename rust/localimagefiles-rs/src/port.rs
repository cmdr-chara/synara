//! The Rust port of `apps/server/src/localImageFiles.ts` (posix, darwin).

#![forbid(unsafe_code)]

#[allow(dead_code)]
#[path = "../../codexhomepaths-rs/src/port.rs"]
mod codex_home;
#[allow(dead_code)]
#[path = "../../localpreviewfiles-rs/src/port.rs"]
mod preview_files;

use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const GRANT_TTL_MS: u64 = 2 * 60 * 1000;
const SCRATCH_WORKSPACES_DIRNAME: &str = "synara-codex-workspaces";
const MAX_ANCESTOR_WALK_DEPTH: usize = 32;
const DAY_MS: f64 = 86_400_000.0;

struct Grant {
    real_file_path: String,
    expires_at_ms: u64,
}

static GRANTS: LazyLock<Mutex<HashMap<String, Grant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn js_trim(s: &str) -> String {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
        .to_string()
}

/// `value?.trim()` for `input.<key>`: `undefined`/`null` short-circuit to
/// `None`, a non-string non-nullish value throws the `?.trim is not a
/// function` TypeError.
fn member_trim(v: &Value, expr: &str) -> Result<Option<String>, String> {
    match v {
        _ if is_undefined(v) || v.is_null() => Ok(None),
        Value::String(s) => Ok(Some(js_trim(s))),
        _ => Err(format!("{expr}?.trim is not a function")),
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        _ if is_undefined(v) => false,
        Value::Null | Value::Bool(false) => false,
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

// --- node:path posix helpers ---

fn posix_is_absolute(p: &str) -> bool {
    p.starts_with('/')
}

fn posix_normalize(p: &str) -> String {
    let absolute = posix_is_absolute(p);
    let trailing = p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if let Some(last) = out.last()
                    && *last != ".."
                {
                    out.pop();
                    continue;
                }
                if !absolute {
                    out.push("..");
                }
            }
            _ => out.push(seg),
        }
    }
    let mut s = if absolute {
        format!("/{}", out.join("/"))
    } else {
        out.join("/")
    };
    if s.is_empty() {
        s = if absolute { "/".into() } else { ".".into() };
    }
    if trailing && s != "/" {
        s.push('/');
    }
    s
}

fn cwd_string() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "/".to_string())
}

/// `path.resolve(...args)`: right-to-left until absolute, empty args skipped.
fn posix_resolve_args(args: &[&str]) -> String {
    let joined: Vec<&str> = args.iter().copied().filter(|a| !a.is_empty()).collect();
    let joined = joined.join("/");
    if posix_is_absolute(&joined) {
        posix_normalize(&joined)
    } else {
        posix_normalize(&format!("{}/{}", cwd_string(), joined))
    }
}

fn posix_resolve(p: &str) -> String {
    posix_resolve_args(&[p])
}

fn posix_dirname(p: &str) -> String {
    let s = p.trim_end_matches('/');
    let s = if s.is_empty() {
        if p.is_empty() {
            return ".".to_string();
        }
        "/"
    } else {
        s
    };
    match s.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => s[..i].to_string(),
    }
}

fn posix_basename(p: &str) -> String {
    let s = p.trim_end_matches('/');
    let s = if s.is_empty() { "/" } else { s };
    s.rsplit('/').next().unwrap_or("").to_string()
}

fn posix_join(parts: &[&str]) -> String {
    let joined: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    if joined.is_empty() {
        return ".".to_string();
    }
    posix_normalize(&joined.join("/"))
}

/// `path.relative(from, to)` on posix.
fn posix_relative(from: &str, to: &str) -> String {
    let f = posix_resolve(from);
    let t = posix_resolve(to);
    let fseg: Vec<&str> = f.split('/').filter(|s| !s.is_empty()).collect();
    let tseg: Vec<&str> = t.split('/').filter(|s| !s.is_empty()).collect();
    let mut i = 0;
    while i < fseg.len() && i < tseg.len() && fseg[i] == tseg[i] {
        i += 1;
    }
    let mut out: Vec<String> = Vec::new();
    for _ in i..fseg.len() {
        out.push("..".to_string());
    }
    for s in &tseg[i..] {
        out.push((*s).to_string());
    }
    out.join("/")
}

fn is_path_inside(candidate: &str, root: &str) -> bool {
    let rel = posix_relative(root, candidate);
    rel.is_empty() || (rel != ".." && !rel.starts_with("../") && !posix_is_absolute(&rel))
}

// --- fs helpers ---

fn realpath_or_null(candidate: &str) -> Option<String> {
    if candidate.is_empty() {
        return None;
    }
    std::fs::canonicalize(candidate)
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

fn stat_is_file(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

enum StatResult {
    Exists,
    Missing,
    OtherError,
}

fn stat_result(p: &str) -> StatResult {
    match std::fs::metadata(p) {
        Ok(_) => StatResult::Exists,
        Err(e) => match e.raw_os_error() {
            // ENOENT / ENOTDIR are the only "genuinely missing" failures.
            Some(2) | Some(20) => StatResult::Missing,
            _ => StatResult::OtherError,
        },
    }
}

fn tmpdir() -> String {
    std::env::temp_dir().to_string_lossy().to_string()
}

fn find_git_root(start_path: &str) -> Option<String> {
    let mut current = posix_resolve(start_path);
    loop {
        let marker = format!("{current}/.git");
        if let Ok(m) = std::fs::metadata(&marker)
            && (m.is_dir() || m.is_file())
        {
            return Some(current);
        }
        let parent = posix_dirname(&current);
        if parent == current {
            return None;
        }
        current = parent;
    }
}

fn temporary_directory_roots() -> Vec<String> {
    let mut candidates = vec![tmpdir()];
    if let Ok(t) = std::env::var("TMPDIR") {
        candidates.push(t);
    }
    if cfg!(target_os = "macos") {
        candidates.push("/tmp".to_string());
    }
    let mut seen = HashSet::new();
    let unique: Vec<String> = candidates
        .into_iter()
        .filter(|c| seen.insert(c.clone()))
        .collect();
    let mut roots = Vec::new();
    let mut seen2 = HashSet::new();
    for c in unique {
        if let Some(r) = realpath_or_null(&c)
            && seen2.insert(r.clone())
        {
            roots.push(r);
        }
    }
    roots
}

fn resolve_workspace_root(cwd: Option<&str>) -> Option<String> {
    let cwd = cwd?;
    let real_cwd = realpath_or_null(cwd)?;
    match find_git_root(&real_cwd) {
        Some(root) => realpath_or_null(&root).or(Some(real_cwd)),
        None => Some(real_cwd),
    }
}

// --- @synara/shared/path helpers ---

fn is_unc_path(v: &str) -> bool {
    v.starts_with("\\\\")
}

fn is_windows_drive_path(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'/' || b[2] == b'\\')
}

fn is_windows_absolute_path(v: &str) -> bool {
    is_unc_path(v) || is_windows_drive_path(v)
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

// --- outOfRootFileReference (bounded ancestor relocation) ---

fn resolve_out_of_root_file_reference(
    workspace_root: &str,
    relative_path: &str,
    home_dir: &str,
) -> Option<String> {
    let relative_path = js_trim(relative_path);
    if relative_path.contains('\0') || !is_workspace_relative_path_safe(&relative_path) {
        return None;
    }
    let real_home = realpath_or_null(home_dir)?;
    let real_root = realpath_or_null(workspace_root)?;
    if !is_path_inside(&real_root, &real_home) {
        return None;
    }
    let segments: Vec<&str> = relative_path.split(['/', '\\']).collect();
    let mut in_root = vec![real_root.as_str()];
    in_root.extend(segments.iter().copied());
    let in_root_candidate = posix_join(&in_root);
    match stat_result(&in_root_candidate) {
        StatResult::OtherError | StatResult::Exists => return None,
        StatResult::Missing => {}
    }
    let mut ancestor = posix_dirname(&real_root);
    let mut depth = 0;
    while depth < MAX_ANCESTOR_WALK_DEPTH && is_path_inside(&ancestor, &real_home) {
        let mut parts = vec![ancestor.as_str()];
        parts.extend(segments.iter().copied());
        let candidate = posix_join(&parts);
        if let Some(real_candidate) = realpath_or_null(&candidate)
            && is_path_inside(&real_candidate, &real_home)
            && stat_is_file(&real_candidate)
        {
            return Some(real_candidate);
        }
        let parent = posix_dirname(&ancestor);
        if parent == ancestor {
            break;
        }
        ancestor = parent;
        depth += 1;
    }
    None
}

// --- codexGeneratedImages helpers ---

fn process_env_value() -> Value {
    let mut m = Map::new();
    for (k, v) in std::env::vars() {
        m.insert(k, Value::String(v));
    }
    Value::Object(m)
}

/// `{...process.env, ...context.environment}` — object entries spread by key,
/// string values spread as indexed characters, everything else contributes
/// nothing (JS object spread of a primitive is a no-op).
fn spread_env(environment: &Value) -> Value {
    let mut m = match process_env_value() {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    match environment {
        Value::Object(o) => {
            for (k, v) in o {
                m.insert(k.clone(), v.clone());
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                m.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
    Value::Object(m)
}

/// `codexHomePathsInputFromContext` — builds the CodexHomePathsInput that the
/// shared overlay resolver consumes.
fn codex_home_paths_input(candidate: &Value) -> Result<Value, String> {
    let context = if candidate.is_string() {
        json!({ "homePath": candidate.clone() })
    } else if candidate.is_null() || is_undefined(candidate) {
        json!({})
    } else {
        candidate.clone()
    };
    let mut out = Map::new();
    for key in ["homePath", "shadowHomePath", "accountId"] {
        let v = member(&context, key)?;
        if let Some(t) = member_trim(&v, &format!("context.{key}"))?
            && !t.is_empty()
        {
            out.insert(key.to_string(), v);
        }
    }
    let environment = member(&context, "environment")?;
    if truthy(&environment) {
        out.insert("env".to_string(), spread_env(&environment));
    }
    Ok(Value::Object(out))
}

fn resolve_codex_generated_image_homes(candidate: &Value) -> Result<Vec<String>, String> {
    codex_home::resolve_codex_home_allowlist_candidates(codex_home_paths_input(candidate)?)
}

// --- grants ---

fn prune_expired(now: u64) {
    if let Ok(mut map) = GRANTS.lock() {
        map.retain(|_, g| g.expires_at_ms > now);
    }
}

fn random_uuid() -> String {
    let mut b = std::fs::read("/dev/urandom")
        .ok()
        .filter(|v| v.len() >= 16)
        .map(|v| v[..16].to_vec())
        .unwrap_or_else(|| {
            let t = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            let mut v = Vec::with_capacity(16);
            v.extend_from_slice(&t.to_le_bytes());
            v.extend_from_slice(&(t ^ 0x9e3779b97f4a7c15).rotate_left(23).to_le_bytes());
            v
        });
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

/// Days-from-epoch to (year, month, day) — Howard Hinnant's civil algorithm.
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// `new Date(ms).toISOString()` for an always-in-range timestamp.
fn iso(ms: f64) -> String {
    let days = (ms / DAY_MS).floor() as i64;
    let rem = (ms - days as f64 * DAY_MS) as i64;
    let (y, mo, d) = civil(days);
    let (hh, mm, ss, mss) = (
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1_000 % 60,
        rem % 1_000,
    );
    let year = if (0..=9999).contains(&y) {
        format!("{y:04}")
    } else if y < 0 {
        format!("-{:06}", -y)
    } else {
        format!("+{y:06}")
    };
    format!("{year}-{mo:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{mss:03}Z")
}

// --- ports ---

/// Port of `resolveLocalPreviewGrantRealPath` (`apps/server/src/localImageFiles.ts:59`).
pub fn resolve_local_preview_grant_real_path(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    let token = member_trim(&member(&input, "token")?, "input.token")?;
    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => return Ok(None),
    };
    let now = now_ms();
    prune_expired(now);
    let found = GRANTS.lock().ok().and_then(|map| {
        map.get(&token)
            .filter(|g| g.expires_at_ms > now)
            .map(|g| g.real_file_path.clone())
    });
    Ok(found)
}

fn has_valid_preview_grant(token: &Value, real_file_path: &str) -> Result<bool, String> {
    let grant_input = json!({ "token": token.clone() });
    Ok(resolve_local_preview_grant_real_path(grant_input)?.as_deref() == Some(real_file_path))
}

/// Port of `resolveAllowedLocalPreviewFile` (`apps/server/src/localImageFiles.ts:133`).
pub async fn resolve_allowed_local_preview_file(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let requested = member(&input, "requestedPath")?;
    let requested_path = member_trim(&requested, "input.requestedPath")?;
    let requested_path = match requested_path {
        Some(p) if !p.is_empty() && !p.contains('\0') => p,
        _ => return Ok(None),
    };
    if !preview_files::is_supported_local_preview_file_path(Value::String(requested_path.clone()))?
    {
        return Ok(None);
    }

    let cwd = member(&input, "cwd")?;
    let cwd_str: Option<String> = match &cwd {
        Value::String(s) => Some(s.clone()),
        _ if is_undefined(&cwd) || cwd.is_null() => None,
        // `path.resolve(nonString, ...)` raises a TypeError the call does not catch.
        Value::Number(_) => {
            return Err(
                "The \"path\" argument must be of type string. Received type number".into(),
            );
        }
        Value::Bool(b) => {
            return Err(format!(
                "The \"path\" argument must be of type string. Received type boolean ({b})"
            ));
        }
        Value::Array(_) => {
            return Err(
                "The \"path\" argument must be of type string. Received an instance of Array"
                    .into(),
            );
        }
        Value::Object(_) => {
            return Err(
                "The \"path\" argument must be of type string. Received an instance of Object"
                    .into(),
            );
        }
        _ => None,
    };

    let resolved_requested = if posix_is_absolute(&requested_path) {
        posix_resolve(&requested_path)
    } else {
        let base = cwd_str.clone().unwrap_or_else(cwd_string);
        posix_resolve_args(&[&base, &requested_path])
    };
    let Some(real_file_path) = realpath_or_null(&resolved_requested) else {
        return Ok(None);
    };
    if !preview_files::is_supported_local_preview_file_path(Value::String(real_file_path.clone()))?
    {
        return Ok(None);
    }
    let Some(meta) = std::fs::metadata(&real_file_path).ok() else {
        return Ok(None);
    };
    if !meta.is_file() {
        return Ok(None);
    }
    let resolved = json!({
        "path": real_file_path,
        "fileName": posix_basename(&real_file_path),
        "sizeBytes": meta.len(),
    });

    if let Some(root) = resolve_workspace_root(cwd_str.as_deref())
        && is_path_inside(&real_file_path, &root)
    {
        return Ok(Some(resolved));
    }

    let temp_roots = temporary_directory_roots();
    let scratch_root = member(&input, "scratchWorkspacesRoot")?;
    let configured_scratch = match &scratch_root {
        Value::String(s) => realpath_or_null(s),
        _ => None,
    };
    let mut scratch_roots: Vec<String> = Vec::new();
    if let Some(r) = configured_scratch {
        scratch_roots.push(r);
    }
    for root in &temp_roots {
        scratch_roots.push(posix_join(&[root, SCRATCH_WORKSPACES_DIRNAME]));
    }
    if scratch_roots
        .iter()
        .any(|r| is_path_inside(&real_file_path, r))
    {
        return Ok(Some(resolved));
    }

    let allow_absolute = truthy(&member(&input, "allowAbsoluteLocalPreviewFile")?);
    if allow_absolute && posix_is_absolute(&requested_path) {
        let grant = member(&input, "previewGrant")?;
        if has_valid_preview_grant(&grant, &real_file_path)? {
            return Ok(Some(resolved));
        }
    }

    let is_image =
        preview_files::is_supported_local_image_path(Value::String(real_file_path.clone()))?;
    if !is_image
        && !preview_files::is_supported_local_video_path(Value::String(real_file_path.clone()))?
    {
        return Ok(None);
    }
    if temp_roots
        .iter()
        .any(|r| is_path_inside(&real_file_path, r))
    {
        return Ok(Some(resolved));
    }
    if !is_image {
        return Ok(None);
    }

    let codex_home_path = member(&input, "codexHomePath")?;
    let codex_home_paths = member(&input, "codexHomePaths")?;
    let candidates: Vec<Value> = if is_undefined(&codex_home_paths) {
        vec![codex_home_path]
    } else if is_undefined(&codex_home_path) {
        match &codex_home_paths {
            Value::Array(a) => a.clone(),
            Value::Null => return Err("Cannot read properties of null (reading 'flatMap')".into()),
            _ => return Err("codexHomeCandidates.flatMap is not a function".into()),
        }
    } else {
        let mut v = vec![codex_home_path];
        match &codex_home_paths {
            Value::Array(a) => v.extend(a.iter().cloned()),
            Value::Null => return Err("codexHomeCandidates.flatMap is not a function".into()),
            _ => return Err("codexHomeCandidates.flatMap is not a function".into()),
        }
        v
    };

    let mut homes: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for c in &candidates {
        for home in resolve_codex_generated_image_homes(c)? {
            if seen.insert(home.clone()) {
                homes.push(home);
            }
        }
    }
    for home in homes {
        let real_home = realpath_or_null(&home);
        let root = posix_join(&[&home, "generated_images"]);
        let real_root = realpath_or_null(&root);
        if let (Some(h), Some(r)) = (real_home, real_root)
            && is_path_inside(&r, &h)
            && is_path_inside(&real_file_path, &r)
        {
            return Ok(Some(resolved));
        }
    }
    Ok(None)
}

/// Port of `createLocalPreviewGrant` (`apps/server/src/localImageFiles.ts:244`).
pub async fn create_local_preview_grant(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let requested = member(&input, "requestedPath")?;
    let requested_path = match &requested {
        _ if is_undefined(&requested) => {
            return Err("Cannot read properties of undefined (reading 'trim')".to_string());
        }
        Value::Null => return Err("Cannot read properties of null (reading 'trim')".to_string()),
        Value::String(s) => js_trim(s),
        _ => return Err("input.requestedPath.trim is not a function".to_string()),
    };
    if requested_path.is_empty() || requested_path.contains('\0') {
        return Err("Invalid local preview path.".to_string());
    }

    let mut real_file_path: Option<String>;
    if posix_is_absolute(&requested_path) {
        real_file_path = realpath_or_null(&posix_resolve(&requested_path));
    } else {
        let cwd = member(&input, "cwd")?;
        let cwd_is_abs =
            match &cwd {
                _ if is_undefined(&cwd) || cwd.is_null() => false,
                Value::String(s) => posix_is_absolute(s),
                // `input.cwd && path.isAbsolute(input.cwd)` — a truthy non-string
                // reaches `path.isAbsolute`, which raises a TypeError.
                Value::Bool(false) => false,
                Value::Number(_) => {
                    return Err(
                        "The \"path\" argument must be of type string. Received type number".into(),
                    );
                }
                _ => return Err(
                    "The \"path\" argument must be of type string. Received an instance of Object"
                        .into(),
                ),
            };
        let real_cwd = if cwd_is_abs {
            match &cwd {
                Value::String(s) => realpath_or_null(s),
                _ => None,
            }
        } else {
            None
        };
        let Some(real_cwd) = real_cwd else {
            return Err("Relative local files need an available workspace.".to_string());
        };
        let candidate = posix_resolve_args(&[&real_cwd, &requested_path]);
        real_file_path = realpath_or_null(&candidate);
        if let Some(found) = real_file_path.clone() {
            let mut allowed =
                is_path_inside(&candidate, &real_cwd) && is_path_inside(&found, &real_cwd);
            if !allowed {
                let roots_value = member(&input, "worktreeRoots")?;
                let mut worktree_roots = Vec::new();
                if let Value::Array(roots) = &roots_value {
                    for root in roots {
                        if let Value::String(s) = root {
                            if posix_is_absolute(s)
                                && let Some(r) = realpath_or_null(s)
                            {
                                worktree_roots.push(r);
                            }
                        } else if !(is_undefined(root) || root.is_null()) {
                            // `path.isAbsolute(nonString)` throws before the filter drops it.
                            return Err(
                                "The \"path\" argument must be of type string. Received an instance of Object"
                                    .into(),
                            );
                        }
                    }
                }
                allowed = worktree_roots.iter().any(|r| is_path_inside(&real_cwd, r))
                    && worktree_roots
                        .iter()
                        .any(|r| is_path_inside(&candidate, r) && is_path_inside(&found, r));
            }
            if !allowed {
                return Err(
                    "Preview file is outside the workspace and registered worktrees.".to_string(),
                );
            }
        } else {
            let home_dir = member(&input, "homeDir")?;
            if let Value::String(home) = &home_dir {
                real_file_path =
                    resolve_out_of_root_file_reference(&real_cwd, &requested_path, home);
            }
        }
    }
    let Some(real_file_path) = real_file_path else {
        return Err("Preview file not found.".to_string());
    };
    if !stat_is_file(&real_file_path) {
        return Err("Preview path is not a file.".to_string());
    }

    let expires_at_ms = now_ms() + GRANT_TTL_MS;
    let grant = random_uuid();
    if let Ok(mut map) = GRANTS.lock() {
        map.insert(
            grant.clone(),
            Grant {
                real_file_path: real_file_path.clone(),
                expires_at_ms,
            },
        );
    }
    prune_expired(now_ms());
    Ok(json!({
        "grant": grant,
        "expiresAt": iso(expires_at_ms as f64),
        "canonicalPath": real_file_path,
    }))
}
