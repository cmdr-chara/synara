//! The Rust port of `apps/web/src/markdown-links.ts`.

#![forbid(unsafe_code)]

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

// JavaScript `\s` covers \u{FEFF} on top of Unicode White_Space; Rust regex `\s` does not.
const JS_WS: &str = "\\t\\n\\x0b\\f\\r \\u{a0}\\u{1680}\\u{2000}-\\u{200a}\\u{2028}\\u{2029}\\u{202f}\\u{205f}\\u{3000}\\u{feff}";

static WINDOWS_DRIVE_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z]:[\\/]").unwrap());
static EXTERNAL_SCHEME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z0-9+.-]*):(.*)$").unwrap());
static RELATIVE_PATH_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(~/|\.{1,2}/)").unwrap());
static RELATIVE_FILE_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._-]+(?:/[A-Za-z0-9._-]+)+(?::\d+){0,2}$").unwrap());
static RELATIVE_FILE_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._-]+\.[A-Za-z0-9_-]+(?::\d+){0,2}$").unwrap());
static POSITION_SUFFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r":\d+(?::\d+)?$").unwrap());
static POSITION_ONLY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d+(?::\d+)?$").unwrap());
static BASENAME_EXT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\.[A-Za-z0-9_-]+$").unwrap());
static HASH_LINE_COL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^#L(\d+)(?:C(\d+))?$").unwrap());
static COLLAPSED_SEGMENT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:\.{3}|…)/+").unwrap());
static BACKTICK_SPAN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"`([^`]+)`").unwrap());
static DRIVE_SLASH_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^/[A-Za-z]:[\\/]").unwrap());
static DRIVE_SEGMENT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^[a-z]:$").unwrap());

const POSIX_FILE_ROOT_PREFIXES: [&str; 10] = [
    "/Users/",
    "/home/",
    "/tmp/",
    "/var/",
    "/etc/",
    "/opt/",
    "/mnt/",
    "/Volumes/",
    "/private/",
    "/root/",
];

fn bare_posix_path_re() -> &'static Regex {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(&format!(
            r#"(?:^|[{JS_WS}(])(/(?:Users|home|tmp|var|etc|opt|mnt|Volumes|private|root)/[^{JS_WS}`'\")]*)"#
        ))
        .unwrap()
    });
    &RE
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn js_replace_all(s: &str, from: char, to: char) -> String {
    s.chars().map(|c| if c == from { to } else { c }).collect()
}

// --- decodeURIComponent / encodeURIComponent --------------------------------

fn safe_decode(value: &str) -> String {
    decode_uri_component(value).unwrap_or_else(|| value.to_string())
}

fn decode_uri_component(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return None;
            }
            out.push(u8::from_str_radix(&value[i + 1..i + 3], 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn is_uri_unescaped(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
}

fn encode_uri_component(segment: &str) -> String {
    let mut out = String::new();
    for c in segment.chars() {
        if is_uri_unescaped(c) {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).as_bytes() {
                out.push('%');
                out.push_str(&format!("{b:02X}"));
            }
        }
    }
    out
}

// --- shared/path.ts helpers ---------------------------------------------------

fn is_windows_drive_path(value: &str) -> bool {
    WINDOWS_DRIVE_PATH_RE.is_match(value)
}

fn is_unc_path(value: &str) -> bool {
    value.starts_with("\\\\")
}

fn is_windows_absolute_path(value: &str) -> bool {
    is_unc_path(value) || is_windows_drive_path(value)
}

fn is_local_absolute_path(value: &str) -> bool {
    value.starts_with('/') || is_windows_absolute_path(value)
}

fn is_normalized_windows_absolute_path(value: &str) -> bool {
    is_windows_drive_path(value) || value.starts_with("//")
}

fn local_paths_equal(left: &str, right: &str) -> bool {
    let l = js_replace_all(js_trim(left), '\\', '/');
    let r = js_replace_all(js_trim(right), '\\', '/');
    let lt = l.trim_end_matches('/');
    let rt = r.trim_end_matches('/');
    if is_normalized_windows_absolute_path(&l) && is_normalized_windows_absolute_path(&r) {
        lt.to_lowercase() == rt.to_lowercase()
    } else {
        lt == rt
    }
}

fn windows_relative_path_of(target: &str, root: &str) -> Option<String> {
    let t: Vec<&str> = target.split('/').collect();
    let r: Vec<&str> = root.split('/').collect();
    if t.len() <= r.len() {
        return None;
    }
    if r.iter()
        .enumerate()
        .all(|(i, s)| s.to_lowercase() == t[i].to_lowercase())
    {
        Some(t[r.len()..].join("/"))
    } else {
        None
    }
}

fn is_workspace_relative_path_safe(value: &str) -> bool {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with('/') || is_windows_absolute_path(trimmed) {
        return false;
    }
    trimmed
        .split(['\\', '/'])
        .all(|seg| seg != ".." && seg != ".")
}

fn workspace_relative_path_of(target: &str, root: &str) -> Option<String> {
    let normalized_target = js_replace_all(js_trim(target), '\\', '/');
    let normalized_root = js_replace_all(js_trim(root), '\\', '/')
        .trim_end_matches('/')
        .to_string();
    if normalized_root.is_empty() || normalized_target.is_empty() {
        return None;
    }
    let case_insensitive = is_normalized_windows_absolute_path(&normalized_target)
        && is_normalized_windows_absolute_path(&normalized_root);
    let relative = if case_insensitive {
        windows_relative_path_of(&normalized_target, &normalized_root)
    } else {
        normalized_target
            .strip_prefix(&format!("{normalized_root}/"))
            .map(str::to_string)
    }?;
    let relative = relative.trim_end_matches('/').to_string();
    if relative.is_empty() {
        return None;
    }
    is_workspace_relative_path_safe(&relative).then_some(relative)
}

// --- terminal-links resolvePathLinkTarget (clean-room from recorded cases) ----

fn split_path_and_position(raw: &str) -> (String, Option<String>, Option<String>) {
    // Greedy `.*` keeps the longest path portion before the `:\d+(:\d+)?` suffix.
    let m = Regex::new(r"^(.*):(\d+)(?::(\d+))?$")
        .unwrap()
        .captures(raw);
    match m {
        Some(c) => {
            let path = c.get(1).map_or("", |g| g.as_str()).to_string();
            let line = c.get(2).map(|g| g.as_str().to_string());
            let col = c.get(3).map(|g| g.as_str().to_string());
            (path, line, col)
        }
        None => (raw.to_string(), None, None),
    }
}

fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/') || is_windows_absolute_path(path)
}

fn is_windows_path_style(path: &str) -> bool {
    path.contains('\\')
}

fn infer_home_from_cwd(cwd: &str) -> Option<String> {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(/(?:Users|home)/[^/]+)").unwrap());
    static WIN: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)^([a-zA-Z]:[\\/]+Users[\\/]+[^\\/]+)").unwrap());
    RE.captures(cwd)
        .or_else(|| WIN.captures(cwd))
        .map(|c| c[1].to_string())
}

fn join_path(base: &str, rel: &str, sep: char) -> String {
    format!("{}{sep}{}", base.trim_end_matches(['/', '\\']), rel)
}

fn resolve_path_link_target(raw_path: &str, cwd: &str) -> String {
    let (path, line, column) = split_path_and_position(raw_path);
    let mut resolved = path.clone();
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = infer_home_from_cwd(cwd) {
            let sep = if is_windows_path_style(&home) {
                '\\'
            } else {
                '/'
            };
            resolved = join_path(&home, rest, sep);
        }
    } else if !is_absolute_path(&path) {
        let sep = if is_windows_path_style(cwd) {
            '\\'
        } else {
            '/'
        };
        resolved = join_path(cwd, &path, sep);
    }
    match line {
        Some(line) => format!(
            "{resolved}:{line}{}",
            column.map(|c| format!(":{c}")).unwrap_or_default()
        ),
        None => resolved,
    }
}

// --- markdown-links.ts ----------------------------------------------------------

fn parse_file_url_href(href: &str, decode_path: bool) -> Option<(String, String)> {
    // WHATWG `new URL` for the `file:` special scheme only.
    let cleaned: String = href
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let cleaned = js_trim(&cleaned);
    let scheme_end = cleaned.find(':')?;
    let scheme = &cleaned[..scheme_end];
    if !scheme
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic())
        || !scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
    {
        return None;
    }
    if !scheme.eq_ignore_ascii_case("file") {
        return None;
    }
    let rest = &cleaned[scheme_end + 1..];
    // Split off the fragment first, then the query.
    let (before_hash, hash) = match rest.find('#') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let before_query = before_hash.split('?').next().unwrap_or("");
    let normalized = js_replace_all(before_query, '\\', '/');
    let (mut hostname, mut path) = if let Some(after_slashes) = normalized.strip_prefix("//") {
        let end = after_slashes.find('/').unwrap_or(after_slashes.len());
        (after_slashes[..end].to_lowercase(), &after_slashes[end..])
    } else {
        (String::new(), normalized.as_str())
    };
    if hostname == "localhost" {
        hostname.clear();
    }
    if hostname.is_empty() && !path.starts_with('/') && !path.is_empty() {
        // `file:` is a special scheme: a non-empty path is rooted at `/`.
        path = &normalized;
    }
    let raw_path = if hostname.is_empty() {
        path.to_string()
    } else {
        format!("//{hostname}{path}")
    };
    if raw_path.is_empty() {
        return None;
    }
    // The browser parser encodes "C:/foo" as "/C:/foo"; strip the leading slash.
    let normalized_path = if DRIVE_SLASH_RE.is_match(&raw_path) {
        raw_path[1..].to_string()
    } else {
        raw_path
    };
    Some((
        if decode_path {
            safe_decode(&normalized_path)
        } else {
            normalized_path
        },
        hash.to_string(),
    ))
}

fn strip_search_and_hash(value: &str) -> (String, String) {
    let (path_with_search, hash) = match value.find('#') {
        Some(i) => (&value[..i], &value[i..]),
        None => (value, ""),
    };
    let path = path_with_search.split('?').next().unwrap_or("").to_string();
    (path, hash.to_string())
}

/// Port of `markdownFilePathHref` (`apps/web/src/markdown-links.ts:74`).
pub fn markdown_file_path_href(path: String) -> Result<String, String> {
    let normalized = js_replace_all(&path, '\\', '/');
    let encoded = normalized
        .split('/')
        .enumerate()
        .map(|(i, seg)| {
            if i == 0 && DRIVE_SEGMENT_RE.is_match(seg) {
                seg.to_string()
            } else {
                encode_uri_component(seg)
            }
        })
        .collect::<Vec<_>>()
        .join("/");
    Ok(if encoded.starts_with("//") {
        format!("file:{encoded}")
    } else {
        format!(
            "file://{}{encoded}",
            if encoded.starts_with('/') { "" } else { "/" }
        )
    })
}

/// Port of `rewriteMarkdownFileUriHref` (`apps/web/src/markdown-links.ts:87`).
pub fn rewrite_markdown_file_uri_href(href: Option<String>) -> Result<Option<String>, String> {
    let Some(href) = href.filter(|h| !h.is_empty()) else {
        return Ok(None);
    };
    Ok(parse_file_url_href(js_trim(&href), false).map(|(path, hash)| format!("{path}{hash}")))
}

fn looks_like_posix_filesystem_path(path: &str) -> bool {
    if !path.starts_with('/') {
        return false;
    }
    if POSIX_FILE_ROOT_PREFIXES.iter().any(|p| path.starts_with(p)) {
        return true;
    }
    if POSITION_SUFFIX_RE.is_match(path) {
        return true;
    }
    let basename = &path[path.rfind('/').map_or(0, |i| i + 1)..];
    BASENAME_EXT_RE.is_match(basename)
}

fn append_line_column_from_hash(path: &str, hash: &str) -> String {
    if hash.is_empty() || POSITION_SUFFIX_RE.is_match(path) {
        return path.to_string();
    }
    let Some(m) = HASH_LINE_COL_RE.captures(hash) else {
        return path.to_string();
    };
    let line = m.get(1).map_or("", |g| g.as_str());
    let column = m.get(2).map(|g| g.as_str());
    format!(
        "{path}:{line}{}",
        column.map(|c| format!(":{c}")).unwrap_or_default()
    )
}

fn is_likely_path_candidate(path: &str) -> bool {
    if is_windows_drive_path(path) || is_unc_path(path) {
        return true;
    }
    if RELATIVE_PATH_PREFIX_RE.is_match(path) {
        return true;
    }
    if path.starts_with('/') {
        return looks_like_posix_filesystem_path(path);
    }
    RELATIVE_FILE_PATH_RE.is_match(path) || RELATIVE_FILE_NAME_RE.is_match(path)
}

fn is_relative_path(path: &str) -> bool {
    RELATIVE_PATH_PREFIX_RE.is_match(path)
        || (!path.starts_with('/') && !is_windows_drive_path(path) && !is_unc_path(path))
}

fn has_external_scheme(path: &str) -> bool {
    let Some(m) = EXTERNAL_SCHEME_RE.captures(path) else {
        return false;
    };
    let rest = m.get(2).map_or("", |g| g.as_str());
    if rest.starts_with("//") {
        return true;
    }
    !POSITION_ONLY_RE.is_match(rest)
}

fn path_without_position_suffix(value: &str) -> String {
    POSITION_SUFFIX_RE.replace(js_trim(value), "").to_string()
}

fn strip_collapsed_relative_prefix(path: &str) -> String {
    let mut next = path.to_string();
    while COLLAPSED_SEGMENT_RE.is_match(&next) {
        next = COLLAPSED_SEGMENT_RE.replace(&next, "").to_string();
    }
    next
}

fn path_basename(path: &str) -> &str {
    let idx = path.rfind('/').max(path.rfind('\\'));
    match idx {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

fn unique_absolute_path_ending_with(suffix: &str, known: &[String]) -> Option<String> {
    let mut found: Option<String> = None;
    for raw in known {
        let candidate = js_replace_all(&path_without_position_suffix(raw), '\\', '/');
        if !is_local_absolute_path(&candidate) {
            continue;
        }
        if candidate != suffix && !candidate.ends_with(&format!("/{suffix}")) {
            continue;
        }
        if found.as_ref().is_some_and(|m| *m != candidate) {
            return None;
        }
        found = Some(candidate);
    }
    found
}

fn looks_like_absolute_file_path(path: &str) -> bool {
    let basename = path_basename(path);
    basename.contains('.') && RELATIVE_FILE_NAME_RE.is_match(basename)
}

fn join_absolute_directory(directory: &str, relative: &str) -> String {
    let dir = js_replace_all(directory, '\\', '/')
        .trim_end_matches('/')
        .to_string();
    let suffix = js_replace_all(relative, '\\', '/')
        .trim_start_matches('/')
        .to_string();
    format!("{dir}/{suffix}")
}

fn unique_join_against_known_directories(suffix: &str, known: &[String]) -> Option<String> {
    let mut matches = HashSet::new();
    for raw in known {
        let candidate = js_replace_all(&path_without_position_suffix(raw), '\\', '/');
        if !is_local_absolute_path(&candidate) || looks_like_absolute_file_path(&candidate) {
            continue;
        }
        matches.insert(join_absolute_directory(&candidate, suffix));
    }
    if matches.len() == 1 {
        matches.into_iter().next()
    } else {
        None
    }
}

/// Port of `extractAbsoluteFilesystemPaths` (`apps/web/src/markdown-links.ts:188`).
pub fn extract_absolute_filesystem_paths(text: String) -> Result<Vec<String>, String> {
    let mut found: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    let mut consider = |raw: &str| {
        let trimmed = js_trim(raw).trim_end_matches(['.', ',', ';']);
        if trimmed.is_empty() {
            return;
        }
        let candidate = path_without_position_suffix(trimmed)
            .trim_end_matches('/')
            .to_string();
        if candidate.is_empty() || has_external_scheme(&candidate) {
            return;
        }
        if !is_local_absolute_path(&candidate) && !looks_like_posix_filesystem_path(&candidate) {
            return;
        }
        if seen.insert(candidate.clone()) {
            found.push(candidate);
        }
    };
    for m in BACKTICK_SPAN_RE.captures_iter(&text) {
        consider(m.get(1).map_or("", |g| g.as_str()));
    }
    for m in bare_posix_path_re().captures_iter(&text) {
        consider(m.get(1).map_or("", |g| g.as_str()));
    }
    Ok(found)
}

/// Port of `resolveUniqueAbsoluteSuffixTarget` (`apps/web/src/markdown-links.ts:226`).
pub fn resolve_unique_absolute_suffix_target(
    reference: String,
    known_absolute_paths: Vec<String>,
) -> Result<Option<String>, String> {
    let trimmed = js_trim(&reference);
    if trimmed.is_empty() {
        return Ok(None);
    }
    let suffix = strip_collapsed_relative_prefix(&js_replace_all(
        &path_without_position_suffix(trimmed),
        '\\',
        '/',
    ));
    if suffix.is_empty()
        || suffix.contains('\0')
        || !is_relative_path(&suffix)
        || !is_workspace_relative_path_safe(&suffix)
    {
        return Ok(None);
    }
    let basename = path_basename(&suffix).to_string();
    let m = unique_absolute_path_ending_with(&suffix, &known_absolute_paths)
        .or_else(|| unique_join_against_known_directories(&suffix, &known_absolute_paths))
        .or_else(|| {
            (basename != suffix)
                .then(|| unique_absolute_path_ending_with(&basename, &known_absolute_paths))
                .flatten()
        });
    let Some(found) = m else { return Ok(None) };
    let position = POSITION_SUFFIX_RE
        .find(trimmed)
        .map(|m| m.as_str())
        .unwrap_or("");
    Ok(Some(format!("{found}{position}")))
}

/// Port of `resolveChatFileChipTarget` (`apps/web/src/markdown-links.ts:257`).
pub fn resolve_chat_file_chip_target(
    reference: Option<String>,
    cwd: Option<String>,
    known_absolute_paths: Option<Vec<String>>,
) -> Result<Option<String>, String> {
    let Some(reference) = reference.filter(|r| !r.is_empty()) else {
        return Ok(None);
    };
    let known = known_absolute_paths.unwrap_or_default();
    let target = if !known.is_empty() {
        resolve_unique_absolute_suffix_target(reference.clone(), known)?
    } else {
        None
    };
    match target {
        Some(t) => Ok(Some(t)),
        None => resolve_markdown_file_link_target(Some(reference), cwd),
    }
}

/// Port of `resolveMarkdownFileLinkTarget` (`apps/web/src/markdown-links.ts:270`).
pub fn resolve_markdown_file_link_target(
    href: Option<String>,
    cwd: Option<String>,
) -> Result<Option<String>, String> {
    let Some(href) = href.filter(|h| !h.is_empty()) else {
        return Ok(None);
    };
    let raw_href = js_trim(&href);
    if raw_href.is_empty() || raw_href.starts_with('#') {
        return Ok(None);
    }
    let file_url_target = if raw_href.to_lowercase().starts_with("file:") {
        parse_file_url_href(raw_href, true)
    } else {
        None
    };
    let (source_path, source_hash) = match &file_url_target {
        Some((p, h)) => (p.clone(), h.clone()),
        None => strip_search_and_hash(raw_href),
    };
    let decoded_path = if file_url_target.is_some() {
        js_trim(&source_path).to_string()
    } else {
        safe_decode(js_trim(&source_path))
    };
    let decoded_hash = safe_decode(js_trim(&source_hash));
    if decoded_path.is_empty() {
        return Ok(None);
    }
    if !is_windows_drive_path(&decoded_path)
        && !is_unc_path(&decoded_path)
        && has_external_scheme(&decoded_path)
    {
        return Ok(None);
    }
    let path_without_position = POSITION_SUFFIX_RE.replace(&decoded_path, "").to_string();
    let is_explicit_relative_directory = Regex::new(r"[\\/]$")
        .unwrap()
        .is_match(&path_without_position)
        && is_workspace_relative_path_safe(path_without_position.trim_end_matches(['/', '\\']));
    let is_workspace_path = cwd.as_ref().is_some_and(|cwd| {
        local_paths_equal(&path_without_position, cwd)
            || workspace_relative_path_of(&path_without_position, cwd).is_some()
    });
    if file_url_target.is_none()
        && !is_likely_path_candidate(&decoded_path)
        && !is_explicit_relative_directory
        && !is_workspace_path
    {
        return Ok(None);
    }
    let path_with_position = append_line_column_from_hash(&decoded_path, &decoded_hash);
    if !is_relative_path(&path_with_position) {
        return Ok(Some(path_with_position));
    }
    let Some(cwd) = cwd.filter(|c| !c.is_empty()) else {
        return Ok(None);
    };
    Ok(Some(resolve_path_link_target(&path_with_position, &cwd)))
}
