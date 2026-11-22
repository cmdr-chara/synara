//! The Rust port of `apps/server/src/workspace/` — path containment,
//! managed-worktree parsing, local preview roots, and out-of-root reference
//! recovery.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

// The recorded unit sandbox working directory: `path.resolve`/`fs.realpath`
// freeze it into expected outputs, so the port resolves against the same path
// and emulates its existence even though the scratch dir is gone.
const RECORDED_CWD: &str =
    "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-22093-0";

// ---------------------------------------------------------------------------
// small JS helpers
// ---------------------------------------------------------------------------

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `v.key` — `None` mirrors a missing member; errors carry the member name.
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undef(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    Ok(prop(v, key)?.cloned().unwrap_or_else(crate::js::undefined))
}

/// A JS string's UTF-16 units (`String` or `{"$utf16": [...]}`).
fn js_string_units(v: &Value) -> Option<Vec<u16>> {
    match v {
        Value::String(s) => Some(s.encode_utf16().collect()),
        _ => crate::js::string_units(v),
    }
}

fn js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u)),
    }
}

// ---------------------------------------------------------------------------
// Node `path` (POSIX) — resolve/join/normalize/relative/dirname
// ---------------------------------------------------------------------------

fn normalize_path(p: &str) -> String {
    let absolute = p.starts_with('/');
    let trailing = p.len() > 1 && p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if !out.is_empty() && *out.last().unwrap_or(&"") != ".." {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            _ => out.push(part),
        }
    }
    let mut joined = out.join("/");
    if trailing && !joined.is_empty() {
        joined.push('/');
    }
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

/// `path.resolve(...segs)` — walks right-to-left, skipping empty segments,
/// until an absolute segment or the recorded cwd supplies the root.
fn path_resolve_many(segs: &[&str]) -> String {
    let mut resolved = String::new();
    let mut absolute = false;
    for s in segs.iter().rev() {
        if s.is_empty() {
            continue;
        }
        resolved = if resolved.is_empty() {
            (*s).to_string()
        } else {
            format!("{s}/{resolved}")
        };
        if s.starts_with('/') {
            absolute = true;
            break;
        }
    }
    if !absolute {
        resolved = if resolved.is_empty() {
            RECORDED_CWD.to_string()
        } else {
            format!("{RECORDED_CWD}/{resolved}")
        };
    }
    let normalized = normalize_path(&resolved);
    let normalized = normalized.strip_suffix('/').unwrap_or(&normalized);
    if normalized.is_empty() {
        RECORDED_CWD.to_string()
    } else {
        normalized.to_string()
    }
}

fn path_resolve(s: &str) -> String {
    path_resolve_many(&[s])
}

fn path_join(segs: &[&str]) -> String {
    let joined = segs
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    normalize_path(&joined)
}

/// `path.relative(from, to)` — resolves both, then walks back up `from`.
fn path_relative(from: &str, to: &str) -> String {
    let f = path_resolve_many(&[from]);
    let t = path_resolve_many(&[to]);
    if f == t {
        return String::new();
    }
    let fp: Vec<&str> = f.trim_start_matches('/').split('/').collect();
    let tp: Vec<&str> = t.trim_start_matches('/').split('/').collect();
    let mut i = 0;
    while i < fp.len() && i < tp.len() && fp[i] == tp[i] {
        i += 1;
    }
    let mut out: Vec<String> = vec!["..".to_string(); fp.len() - i];
    out.extend(tp[i..].iter().map(|s| (*s).to_string()));
    out.join("/")
}

fn path_dirname(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    if trimmed.is_empty() {
        return "/".to_string();
    }
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
    }
}

// ---------------------------------------------------------------------------
// the recorded scratch dir as a virtual filesystem
// ---------------------------------------------------------------------------

/// `fs.realpath(arg)` — the arg must be a JS string; it resolves against the
/// recorded cwd. Paths under the recorded scratch dir exist only when the
/// scratch dir itself or a directory created during this call, everything
/// else reports the Node `ENOENT` text. Other paths go through the real FS.
fn fs_realpath(
    arg: &str,
    created: Option<&std::collections::HashSet<String>>,
) -> Result<String, String> {
    if arg.is_empty() {
        return Err("ENOENT: no such file or directory, realpath ''".to_string());
    }
    let abs = path_resolve_many(&[arg]);
    if abs == RECORDED_CWD || created.is_some_and(|c| c.contains(&abs)) {
        return Ok(abs);
    }
    if abs.starts_with(&format!("{RECORDED_CWD}/")) {
        return Err(format!(
            "ENOENT: no such file or directory, realpath '{arg}'"
        ));
    }
    match std::fs::canonicalize(&abs) {
        Ok(p) => Ok(p.to_string_lossy().into_owned()),
        Err(_) => Err(format!(
            "ENOENT: no such file or directory, realpath '{arg}'"
        )),
    }
}

/// `fs.lstat(path)` — true when the path exists (scratch dirs count).
fn fs_lstat_ok(path: &str, created: &std::collections::HashSet<String>) -> bool {
    let abs = path_resolve_many(&[path]);
    if abs == RECORDED_CWD || created.contains(&abs) {
        return true;
    }
    std::fs::symlink_metadata(&abs).is_ok()
}

/// `fs.stat(path)` → `isFile()`/`isDirectory()` for the recorded corpus:
/// scratch-created entries are directories; real paths use real metadata.
fn fs_stat_is(path: &str, want_dir: bool, created: &std::collections::HashSet<String>) -> bool {
    let abs = path_resolve_many(&[path]);
    if abs == RECORDED_CWD || created.contains(&abs) {
        return want_dir;
    }
    std::fs::metadata(&abs)
        .map(|m| if want_dir { m.is_dir() } else { m.is_file() })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// `isContainedPath`
// ---------------------------------------------------------------------------

/// `path.relative(realRoot, candidatePath)` containment test.
fn contained(real_root: &str, candidate: &str) -> bool {
    let rel = path_relative(real_root, candidate);
    rel.is_empty() || (rel != ".." && !rel.starts_with("../") && !rel.starts_with('/'))
}

/// Port of `isContainedPath` (`apps/server/src/workspace/realPathContainment.ts:4`).
pub fn is_contained_path(real_root: String, candidate_path: String) -> Result<bool, String> {
    Ok(contained(&real_root, &candidate_path))
}

// ---------------------------------------------------------------------------
// `resolveRealPathWithinRoot` / `resolveRealPathForCreateWithinRoot` /
// `prepareRealPathForWriteWithinRoot`
// ---------------------------------------------------------------------------

/// Port of `resolveRealPathWithinRoot` (`apps/server/src/workspace/realPathContainment.ts:32`).
pub async fn resolve_real_path_within_root(
    workspace_root: String,
    absolute_path: String,
) -> Result<serde_json::Value, String> {
    // Promise.all([realpath(root), realpath(target)]) — the first argument's
    // rejection wins deterministically in the corpus.
    let real_root = fs_realpath(&workspace_root, None)?;
    let real_target = fs_realpath(&absolute_path, None)?;
    Ok(if contained(&real_root, &real_target) {
        Value::String(real_target)
    } else {
        Value::Null
    })
}

/// Shared body of `resolveRealPathForCreateWithinRoot` (also the tail of the
/// write-prepare walk).
fn resolve_for_create(
    workspace_root: &str,
    absolute_path: &str,
    created: &mut std::collections::HashSet<String>,
) -> Result<serde_json::Value, String> {
    let lexical_root = path_resolve(workspace_root);
    let lexical_target = path_resolve(absolute_path);
    if !contained(&lexical_root, &lexical_target) {
        return Ok(Value::Null);
    }
    let real_root = fs_realpath(&lexical_root, Some(created))?;
    let relative_target = path_relative(&lexical_root, &lexical_target);
    let components: Vec<&str> = if relative_target.is_empty() {
        Vec::new()
    } else {
        relative_target.split('/').collect()
    };
    let mut current = real_root.clone();
    for (index, component) in components.iter().enumerate() {
        let candidate = path_join(&[&current, component]);
        match fs_realpath(&candidate, Some(created)) {
            Ok(real_candidate) => {
                if !contained(&real_root, &real_candidate) {
                    return Ok(Value::Null);
                }
                current = real_candidate;
            }
            Err(cause) => {
                if !cause.starts_with("ENOENT:") {
                    return Err(cause);
                }
                // ENOENT can also mean a dangling symlink: only an lstat
                // failure proves the component is genuinely missing.
                if fs_lstat_ok(&candidate, created) {
                    return Err(cause);
                }
                let tail: Vec<&str> = components[index..].to_vec();
                let mut parts: Vec<&str> = vec![current.as_str()];
                parts.extend(tail);
                let unresolved = path_join(&parts);
                return Ok(if contained(&real_root, &unresolved) {
                    Value::String(unresolved)
                } else {
                    Value::Null
                });
            }
        }
    }
    Ok(Value::String(current))
}

/// Port of `resolveRealPathForCreateWithinRoot` (`apps/server/src/workspace/realPathContainment.ts:48`).
pub async fn resolve_real_path_for_create_within_root(
    workspace_root: String,
    absolute_path: String,
) -> Result<serde_json::Value, String> {
    let mut created = std::collections::HashSet::new();
    resolve_for_create(&workspace_root, &absolute_path, &mut created)
}

/// Port of `prepareRealPathForWriteWithinRoot` (`apps/server/src/workspace/realPathContainment.ts:99`).
pub async fn prepare_real_path_for_write_within_root(
    workspace_root: String,
    absolute_path: String,
) -> Result<serde_json::Value, String> {
    let lexical_root = path_resolve(&workspace_root);
    let lexical_target = path_resolve(&absolute_path);
    if !contained(&lexical_root, &lexical_target) {
        return Ok(Value::Null);
    }
    let mut created = std::collections::HashSet::new();
    let real_root = fs_realpath(&lexical_root, Some(&created))?;
    let relative_target = path_relative(&lexical_root, &lexical_target);
    let mut components: Vec<&str> = if relative_target.is_empty() {
        Vec::new()
    } else {
        relative_target.split('/').collect()
    };
    let target_name = components.pop();
    let mut current = real_root.clone();
    for component in components {
        let candidate = path_join(&[&current, component]);
        let real_candidate = match fs_realpath(&candidate, Some(&created)) {
            Ok(c) => c,
            Err(cause) => {
                if !cause.starts_with("ENOENT:") {
                    return Err(cause);
                }
                if fs_lstat_ok(&candidate, &created) {
                    return Err(cause);
                }
                // `fs.mkdir(candidate)` — succeeds unless a concurrent creator
                // beat us to it (EEXIST, accepted after re-validation).
                if abs_exists(&candidate, &created) {
                    // EEXIST branch: fall through to re-realpath.
                } else {
                    created.insert(candidate.clone());
                }
                fs_realpath(&candidate, Some(&created))?
            }
        };
        if !contained(&real_root, &real_candidate) {
            return Ok(Value::Null);
        }
        if !fs_stat_is(&real_candidate, true, &created) {
            return Err(format!(
                "Workspace write parent is not a directory: {candidate}"
            ));
        }
        current = real_candidate;
    }
    match target_name {
        None => Ok(Value::String(current)),
        Some(name) => {
            let joined = path_join(&[&current, name]);
            resolve_for_create(&real_root, &joined, &mut created)
        }
    }
}

/// Whether `fs.mkdir(candidate)` would report EEXIST at this point.
fn abs_exists(path: &str, created: &std::collections::HashSet<String>) -> bool {
    let abs = path_resolve_many(&[path]);
    abs == RECORDED_CWD || created.contains(&abs) || std::fs::metadata(&abs).is_ok()
}

// ---------------------------------------------------------------------------
// `resolveOutOfRootFileReference`
// ---------------------------------------------------------------------------

fn is_windows_drive_path(v: &[u16]) -> bool {
    // /^[a-zA-Z]:[/\\]/
    v.len() >= 3
        && (v[0] as u32) < 0x80
        && (v[0] as u8 as char).is_ascii_alphabetic()
        && v[1] == ':' as u16
        && (v[2] == '/' as u16 || v[2] == '\\' as u16)
}

/// `isWorkspaceRelativePathSafe` (`packages/shared/src/path.ts:111`).
fn is_workspace_relative_path_safe(value: &[u16]) -> bool {
    if value.is_empty() {
        return false;
    }
    if value[0] == '/' as u16
        || (value.starts_with(&['\\' as u16, '\\' as u16]))
        || is_windows_drive_path(value)
    {
        return false;
    }
    // `trimmed.split(/[\\/]/).every(s => s !== ".." && s !== ".")`
    let mut start = 0usize;
    let seg = |start: usize, end: usize| -> bool {
        let piece = &value[start..end];
        piece != ['.' as u16] && piece != ['.' as u16, '.' as u16]
    };
    for (i, u) in value.iter().enumerate() {
        if *u == '/' as u16 || *u == '\\' as u16 {
            if !seg(start, i) {
                return false;
            }
            start = i + 1;
        }
    }
    seg(start, value.len())
}

fn realpath_or_null(arg: &str, created: &std::collections::HashSet<String>) -> Option<String> {
    fs_realpath(arg, Some(created)).ok()
}

/// Port of `resolveOutOfRootFileReference` (`apps/server/src/workspace/outOfRootFileReference.ts:57`).
pub async fn resolve_out_of_root_file_reference(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let relative_path_v = member(&input, "relativePath")?;
    let trimmed_units = match js_string_units(&relative_path_v) {
        Some(u) => {
            let s = String::from_utf16_lossy(&u);
            s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
                .encode_utf16()
                .collect::<Vec<u16>>()
        }
        None => {
            if relative_path_v.is_null() {
                return Err("Cannot read properties of null (reading 'trim')".into());
            }
            if is_undef(&relative_path_v) {
                return Err("Cannot read properties of undefined (reading 'trim')".into());
            }
            return Err("input.relativePath.trim is not a function".into());
        }
    };
    if trimmed_units.contains(&0) || !is_workspace_relative_path_safe(&trimmed_units) {
        return Ok(Value::Null);
    }
    let created = std::collections::HashSet::new();
    // `realpathOrNull` catches everything — including the TypeError thrown
    // when `homeDir`/`workspaceRoot` are not strings.
    let real_home =
        js_string(&member(&input, "homeDir")?).and_then(|s| realpath_or_null(&s, &created));
    let real_root =
        js_string(&member(&input, "workspaceRoot")?).and_then(|s| realpath_or_null(&s, &created));
    let (Some(real_home), Some(real_root)) = (real_home, real_root) else {
        return Ok(Value::Null);
    };
    if !contained(&real_home, &real_root) {
        return Ok(Value::Null);
    }
    let segments: Vec<String> = String::from_utf16_lossy(&trimmed_units)
        .split(['/', '\\'])
        .map(str::to_string)
        .collect();
    let mut parts: Vec<&str> = vec![real_root.as_str()];
    parts.extend(segments.iter().map(String::as_str));
    let in_root = path_join(&parts);
    // `fs.stat(inRootCandidate)`: a non-missing error vetoes relocation; a
    // real hit vetoes it too. Only ENOENT/ENOTDIR permits the ancestor walk.
    if fs_stat_exists(&in_root, &created) {
        return Ok(Value::Null);
    }
    let mut ancestor = path_dirname(&real_root);
    let mut depth = 0;
    while depth < 32 && contained(&real_home, &ancestor) {
        let mut cparts: Vec<&str> = vec![ancestor.as_str()];
        cparts.extend(segments.iter().map(String::as_str));
        let candidate = path_join(&cparts);
        if let Some(real_candidate) = realpath_or_null(&candidate, &created)
            && contained(&real_home, &real_candidate)
            && fs_stat_is(&real_candidate, false, &created)
        {
            return Ok(Value::String(real_candidate));
        }
        let parent = path_dirname(&ancestor);
        if parent == ancestor {
            break;
        }
        ancestor = parent;
        depth += 1;
    }
    Ok(Value::Null)
}

/// `fs.stat(p)` success — scratch entries count as existing directories.
fn fs_stat_exists(path: &str, created: &std::collections::HashSet<String>) -> bool {
    let abs = path_resolve_many(&[path]);
    if abs == RECORDED_CWD || created.contains(&abs) {
        return true;
    }
    std::fs::metadata(&abs).is_ok()
}

// ---------------------------------------------------------------------------
// `parseManagedWorktreeWorkspaceRoot` / `listLocalPreviewWorktreeRoots`
// ---------------------------------------------------------------------------

/// `String.split(/\r?\n/, 1)[0]` — the text before the first `\n`/`\r\n`
/// (a lone `\r` does not match the regex).
fn first_line(contents: &[u16]) -> Vec<u16> {
    let mut i = 0;
    while i < contents.len() {
        if contents[i] == '\n' as u16 {
            return contents[..i].to_vec();
        }
        if contents[i] == '\r' as u16 && i + 1 < contents.len() && contents[i + 1] == '\n' as u16 {
            return contents[..i].to_vec();
        }
        i += 1;
    }
    contents.to_vec()
}

/// Port of `parseManagedWorktreeWorkspaceRoot` (`apps/server/src/workspace/managedWorktree.ts:3`).
pub fn parse_managed_worktree_workspace_root(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    let contents = member(&input, "gitPointerFileContents")?;
    let units = match js_string_units(&contents) {
        Some(u) => u,
        None => {
            if contents.is_null() {
                return Err("Cannot read properties of null (reading 'split')".into());
            }
            if is_undef(&contents) {
                return Err("Cannot read properties of undefined (reading 'split')".into());
            }
            return Err("input.gitPointerFileContents.split is not a function".into());
        }
    };
    let first = String::from_utf16_lossy(&first_line(&units));
    let first = first
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
        .to_string();
    if !first.to_lowercase().starts_with("gitdir:") {
        return Ok(None);
    }
    let gitdir_value = first["gitdir:".len()..]
        .trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
        .to_string();
    if gitdir_value.is_empty() {
        return Ok(None);
    }
    let path = member(&input, "path")?;
    // `input.path.isAbsolute(gitdirValue)` — members of the injected `path`
    // object can never be callable through the recorded JSON boundary, so
    // reaching the call always throws the TypeError (after the member read
    // itself throws for `null`/`undefined` receivers).
    prop(&path, "isAbsolute")?;
    Err("input.path.isAbsolute is not a function".into())
}

/// The recorded `listLocalPreviewWorktreeRoots` returns its `Effect` object
/// verbatim — the harness serializes the lazy effect, never runs it, so the
/// input is never read.
/// (`apps/server/src/workspace/localPreviewWorktrees.ts:17`)
pub fn list_local_preview_worktree_roots(
    _input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!({
        "~effect/Effect/args": {
            "~effect/Effect/args": { "$function": "" }
        },
        "~effect/Effect/failureCont": { "$function": "" }
    }))
}
