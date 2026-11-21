//! The Rust port of `apps/web/src/composer-editor-mentions.ts`.

#![forbid(unsafe_code)]

use regex::Regex;
use serde_json::{Value, json};

const INLINE_TERMINAL_CONTEXT_PLACEHOLDER: char = '\u{FFFC}';

const CHIP_COMMANDS: [&str; 3] = ["automation", "goal", "computer-use"];

const BUILT_IN_COMMANDS: [&str; 18] = [
    "clear",
    "compact",
    "model",
    "plan",
    "debug",
    "default",
    "review",
    "fork",
    "side",
    "status",
    "subagents",
    "computer-use",
    "fast",
    "export",
    "goal",
    "rename",
    "feedback",
    "automation",
];

// resolveAgentAlias: codex aliases are consulted before claudeAgent ones.
const AGENT_ALIAS_COLORS: [(&str, &str); 17] = [
    ("5.5", "violet"),
    ("5.4", "violet"),
    ("mini", "fuchsia"),
    ("5.4-mini", "fuchsia"),
    ("codex", "teal"),
    ("5.3-codex", "teal"),
    ("spark", "cyan"),
    ("5.3-spark", "cyan"),
    ("5.2", "amber"),
    ("5.2-codex", "orange"),
    ("explore", "cyan"),
    ("review", "amber"),
    ("reviewer", "amber"),
    ("build", "violet"),
    ("implement", "violet"),
    ("plan", "fuchsia"),
    ("planner", "fuchsia"),
];

const COMMON_PUBLIC_BARE_DOMAIN_TLDS: [&str; 8] =
    ["ai", "app", "co", "com", "dev", "io", "net", "org"];

const COMMON_FILE_EXTENSION_TLDS: [&str; 29] = [
    "c", "cc", "conf", "cpp", "css", "go", "h", "hpp", "html", "java", "js", "json", "jsx", "kt",
    "lock", "md", "mjs", "py", "rb", "rs", "sql", "swift", "toml", "ts", "tsx", "txt", "xml",
    "yaml", "yml",
];

// ---------- UTF-16 helpers (JS indices are code units) ----------

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn byte_to_u16(s: &str, byte_off: usize) -> usize {
    utf16_len(&s[..byte_off])
}

fn u16_slice(s: &str, start: usize, end: usize) -> String {
    let mut out = String::new();
    let mut pos = 0usize;
    for c in s.chars() {
        let next = pos + c.len_utf16();
        if next > start && pos < end {
            out.push(c);
        }
        pos = next;
        if pos >= end {
            break;
        }
    }
    out
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{09}'..='\u{0D}'
            | '\u{20}'
            | '\u{A0}'
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

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

/// The char after a byte offset, for `(?=\s)` / `(?=\s|$)` lookahead checks.
fn char_after(s: &str, byte_end: usize) -> Option<char> {
    s.get(byte_end..).and_then(|t| t.chars().next())
}

// ---------- link helpers (lib/linkChips.ts) ----------

fn trim_trailing_link_punctuation(url: &str) -> &str {
    url.trim_end_matches(['.', ',', ';', ':', '!', '?', '\'', '"'])
}

const LINK_BODY: &str = r#"[^\s<>()\[\]]+"#;

fn link_regex() -> Regex {
    Regex::new(&format!(
        r"(?:https?://{0}|(?:[A-Za-z0-9](?:[A-Za-z0-9-]{{0,61}}[A-Za-z0-9])?\.)+[A-Za-z]{{2,63}}(?::\d{{2,5}})?(?:[/?#]{0}*)?)",
        LINK_BODY
    ))
    .unwrap()
}

const LOOKBEHIND_FORBIDDEN: &[char] = &[
    '@', '.', '_', '/', '-', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

fn lookbehind_ok(text: &str, byte_start: usize) -> bool {
    let prev = byte_start
        .checked_sub(1)
        .and_then(|i| text.get(..i + 1)?.chars().next_back());
    match prev {
        None => true,
        Some(c) => !(c.is_ascii_alphanumeric() || LOOKBEHIND_FORBIDDEN.contains(&c)),
    }
}

/// One link-token match candidate (byte offsets), honoring the `(?!\s)`-style
/// trailing boundary and the bare-domain lookbehind.
fn link_match_ok(text: &str, m: &regex::Match, include_end: bool) -> bool {
    // (?<![A-Za-z0-9@._/-]) only applies to the bare-domain branch: http(s)
    // matches are never preceded-filtered.
    let is_http = m.as_str().len() >= 5 && m.as_str()[..5].eq_ignore_ascii_case("http:")
        || m.as_str().len() >= 6 && m.as_str()[..6].eq_ignore_ascii_case("https:");
    if !is_http && !lookbehind_ok(text, m.start()) {
        return false;
    }
    match char_after(text, m.end()) {
        Some(c) if is_js_ws(c) => true,
        None => include_end,
        _ => false,
    }
}

/// `new URL(...)`: only what this file reads — hostname and pathname validity.
fn url_hostname(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| {
            let i = url.find("://")?;
            url.get(i + 3..)
        })?;
    if rest.is_empty() {
        return None;
    }
    let authority: String = rest
        .chars()
        .take_while(|&c| c != '/' && c != '?' && c != '#')
        .collect();
    if authority.is_empty() || authority.chars().any(|c| c.is_whitespace()) {
        return None;
    }
    let no_user = authority.rsplit('@').next().unwrap_or(&authority);
    let host = if let Some(inner) = no_user.strip_prefix('[').and_then(|s| s.split(']').next()) {
        inner.to_string()
    } else {
        no_user.split(':').next().unwrap_or(no_user).to_string()
    };
    if host.is_empty() {
        return None;
    }
    Some(host.to_lowercase())
}

fn is_likely_bare_domain_link(url: &str) -> bool {
    let probe = format!("https://{url}");
    let Some(host) = url_hostname(&probe) else {
        return false;
    };
    if !host.contains('.') {
        return false;
    }
    let labels: Vec<&str> = host.split('.').collect();
    let tld = labels.last().copied().unwrap_or("");
    if tld.len() < 2 || tld.len() > 63 || !tld.chars().all(|c| c.is_ascii_lowercase()) {
        return false;
    }
    if labels
        .iter()
        .any(|l| l.is_empty() || l.starts_with('-') || l.ends_with('-'))
    {
        return false;
    }
    let has_path_or_query = url.chars().any(|c| matches!(c, '/' | '?' | '#'));
    if !has_path_or_query && COMMON_FILE_EXTENSION_TLDS.contains(&tld) {
        return false;
    }
    if !has_path_or_query
        && !host.starts_with("www.")
        && !COMMON_PUBLIC_BARE_DOMAIN_TLDS.contains(&tld)
    {
        return false;
    }
    true
}

fn normalize_composer_link_url(raw_url: &str) -> Option<String> {
    let url = trim_trailing_link_punctuation(js_trim(raw_url));
    if url.is_empty() {
        return None;
    }
    let lower = url.to_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(url.to_string());
    }
    if is_likely_bare_domain_link(url) {
        Some(format!("https://{url}"))
    } else {
        None
    }
}

// ---------- agent aliases (packages/contracts/src/agentMentions.ts) ----------

fn resolve_agent_alias_color(alias: &str) -> Option<&'static str> {
    let normalized = alias.to_lowercase();
    AGENT_ALIAS_COLORS
        .iter()
        .find(|(name, _)| *name == normalized)
        .map(|(_, color)| *color)
}

// ---------- mention helpers (lib/composerMentions.ts) ----------

fn decode_quoted_path(path: &str) -> String {
    let mut out = String::new();
    let mut chars = path.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('"') | Some('\\') => {
                    out.push(chars.next().unwrap());
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn normalize_mention_key(name: &str) -> String {
    js_trim(name).to_lowercase()
}

fn mention_token_keys(mention: &Value) -> Vec<String> {
    let mut keys = Vec::new();
    let name = normalize_mention_key(mention.get("name").and_then(|v| v.as_str()).unwrap_or(""));
    if !name.is_empty() {
        keys.push(name);
    }
    let path = normalize_mention_key(mention.get("path").and_then(|v| v.as_str()).unwrap_or(""));
    if !path.is_empty() {
        keys.push(path.clone());
        if let Some(spec) = path.strip_prefix("plugin://").filter(|s| !s.is_empty()) {
            keys.push(spec.to_string());
            if let Some(plugin_name) = spec.split('@').next().filter(|s| !s.is_empty()) {
                keys.push(plugin_name.to_string());
            }
        }
    }
    keys
}

fn provider_mention_matches_token(mention: &Value, token: &str) -> bool {
    let normalized = normalize_mention_key(token);
    !normalized.is_empty() && mention_token_keys(mention).contains(&normalized)
}

fn mention_path(mention: &Value) -> &str {
    mention.get("path").and_then(|v| v.as_str()).unwrap_or("")
}

// ---------- token collection ----------

struct TokenMatch {
    kind: &'static str,
    value: String,
    extra: Option<String>,
    start: usize,
    end: usize,
}

fn mention_regex() -> Regex {
    Regex::new(r#"(^|\s)@(?:"((?:\\.|[^"\\])*)"|([^\s@]+))"#).unwrap()
}

fn skill_regex() -> Regex {
    Regex::new(r"(^|\s)([$/])([a-zA-Z][a-zA-Z0-9_:-]*)").unwrap()
}

fn agent_mention_regex() -> Regex {
    Regex::new(r"(^|\s)@([a-zA-Z0-9._-]+)").unwrap()
}

fn collect_inline_token_matches(
    text: &str,
    include_trailing: bool,
    include_slash_chips: bool,
) -> Vec<TokenMatch> {
    let mut matches: Vec<TokenMatch> = Vec::new();
    let mut reserved: Vec<(usize, usize)> = Vec::new();
    fn is_reserved(reserved: &[(usize, usize)], pos: usize) -> bool {
        reserved.iter().any(|&(s, e)| pos >= s && pos < e)
    }

    // Links win first.
    let link_re = link_regex();
    for m in link_re.find_iter(text) {
        if !link_match_ok(text, &m, include_trailing) {
            continue;
        }
        let raw = trim_trailing_link_punctuation(m.as_str());
        let Some(url) = normalize_composer_link_url(raw) else {
            continue;
        };
        let start = byte_to_u16(text, m.start());
        let end = start + utf16_len(raw);
        reserved.push((start, end));
        matches.push(TokenMatch {
            kind: "link",
            value: url,
            extra: None,
            start,
            end,
        });
    }

    // Agent mentions: @alias followed by `(`.
    let mut agent_ranges: Vec<(usize, usize)> = Vec::new();
    let agent_re = agent_mention_regex();
    for caps in agent_re.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        let alias = caps.get(2).unwrap().as_str();
        // (?=\() lookahead
        if char_after(text, whole.end()) != Some('(') {
            continue;
        }
        let ws = caps.get(1).map(|g| g.as_str()).unwrap_or("");
        let start = byte_to_u16(text, whole.start()) + utf16_len(ws);
        let end = start + 1 + utf16_len(alias);
        if is_reserved(&reserved, start) {
            continue;
        }
        let Some(color) = resolve_agent_alias_color(alias) else {
            continue;
        };
        agent_ranges.push((start, end));
        matches.push(TokenMatch {
            kind: "agent-mention",
            value: alias.to_string(),
            extra: Some(color.to_string()),
            start,
            end,
        });
    }
    let in_agent = |pos: usize| agent_ranges.iter().any(|&(s, e)| pos >= s && pos < e);

    // @mentions.
    let mention_re = mention_regex();
    for caps in mention_re.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        match char_after(text, whole.end()) {
            Some(c) if is_js_ws(c) => {}
            None if include_trailing => {}
            _ => continue,
        }
        let path = match caps.get(2) {
            Some(g) => decode_quoted_path(g.as_str()),
            None => caps
                .get(3)
                .map(|g| g.as_str().to_string())
                .unwrap_or_default(),
        };
        let prefix = caps.get(1).map(|g| g.as_str()).unwrap_or("");
        let start = byte_to_u16(text, whole.start()) + utf16_len(prefix);
        let end = start + utf16_len(whole.as_str()) - utf16_len(prefix);
        if in_agent(start) || is_reserved(&reserved, start) {
            continue;
        }
        if !path.is_empty() {
            matches.push(TokenMatch {
                kind: "mention",
                value: path,
                extra: None,
                start,
                end,
            });
        }
    }

    // $skill and /slash tokens.
    let skill_re = skill_regex();
    for caps in skill_re.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        match char_after(text, whole.end()) {
            Some(c) if is_js_ws(c) => {}
            None if include_trailing => {}
            _ => continue,
        }
        let ws = caps.get(1).map(|g| g.as_str()).unwrap_or("");
        let skill_prefix = caps.get(2).map(|g| g.as_str()).unwrap_or("$");
        let name = caps.get(3).map(|g| g.as_str()).unwrap_or("");
        let start = byte_to_u16(text, whole.start()) + utf16_len(ws);
        let end = start + utf16_len(whole.as_str()) - utf16_len(ws);
        if in_agent(start) || is_reserved(&reserved, start) {
            continue;
        }
        if name.is_empty() {
            continue;
        }
        let normalized = name.to_lowercase();
        if skill_prefix == "/" && BUILT_IN_COMMANDS.contains(&normalized.as_str()) {
            if include_slash_chips && CHIP_COMMANDS.contains(&normalized.as_str()) {
                matches.push(TokenMatch {
                    kind: "slash-command",
                    value: normalized,
                    extra: None,
                    start,
                    end,
                });
            }
            continue;
        }
        matches.push(TokenMatch {
            kind: "skill",
            value: name.to_string(),
            extra: Some(skill_prefix.to_string()),
            start,
            end,
        });
    }

    matches.sort_by_key(|m| m.start);
    matches
}

fn append_text_segment(segments: &mut Vec<Value>, text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(Value::Object(obj)) = segments
        .last_mut()
        .filter(|v| v.get("type").and_then(|t| t.as_str()) == Some("text"))
    {
        let prev = obj
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .to_string();
        obj.insert("text".to_string(), json!(prev + text));
        return;
    }
    segments.push(json!({"type": "text", "text": text}));
}

fn thread_id_from_mention_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix("thread://")?;
    let trimmed = js_trim(rest);
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn find_mention_reference<'a>(
    token: &str,
    mentions: Option<&'a [Value]>,
    prefix: &str,
) -> Result<Option<&'a Value>, String> {
    match mentions {
        Some(list) => Ok(list.iter().find(|m| {
            mention_path(m).starts_with(prefix) && provider_mention_matches_token(m, token)
        })),
        // `mentions?.find` on a non-array non-nullish value throws in JS.
        None => Err("mentions.find is not a function".to_string()),
    }
}

fn refs_value(refs: &Value) -> Option<&[Value]> {
    match refs {
        Value::Array(a) => Some(a.as_slice()),
        Value::Null => Some(&[]),
        v if crate::js::is_undefined(v) => Some(&[]),
        _ => None,
    }
}

fn split_text_into_prompt_segments(
    text: &str,
    include_trailing: bool,
    mention_references: &Value,
) -> Result<Vec<Value>, String> {
    let mut segments = Vec::new();
    if text.is_empty() {
        return Ok(segments);
    }
    let refs = refs_value(mention_references);
    let matches = collect_inline_token_matches(text, include_trailing, true);
    let mut cursor = 0usize;
    for m in matches {
        if m.start < cursor {
            continue;
        }
        if m.start > cursor {
            append_text_segment(&mut segments, &u16_slice(text, cursor, m.start));
        }
        match m.kind {
            "link" => segments.push(json!({"type": "link", "url": m.value})),
            "agent-mention" => segments.push(json!({
                "type": "agent-mention",
                "alias": m.value,
                "color": m.extra.unwrap_or_default(),
            })),
            "mention" => {
                let token_len = m.end - m.start;
                let thread_ref = find_mention_reference(&m.value, refs, "thread://")?;
                let is_plugin = match refs {
                    Some(list) => list.iter().any(|mention| {
                        mention_path(mention).starts_with("plugin://")
                            && provider_mention_matches_token(mention, &m.value)
                    }),
                    None => {
                        return Err("options.mentionReferences.some is not a function".to_string());
                    }
                };
                let thread_id =
                    thread_ref.and_then(|r| thread_id_from_mention_path(mention_path(r)));
                let is_connector = thread_ref.is_none()
                    && find_mention_reference(&m.value, refs, "connector://")?.is_some();
                if is_connector {
                    segments.push(json!({
                        "type": "mention", "path": m.value, "kind": "connector",
                        "tokenLength": token_len,
                    }));
                } else if let Some(r) = thread_ref {
                    let mut seg = json!({
                        "type": "mention", "path": m.value, "kind": "thread",
                    });
                    if let Some(id) = thread_id {
                        seg["threadId"] = json!(id);
                    }
                    seg["tokenLength"] = json!(token_len);
                    let _ = r;
                    segments.push(seg);
                } else if is_plugin {
                    segments.push(json!({
                        "type": "mention", "path": m.value, "kind": "plugin",
                        "tokenLength": token_len,
                    }));
                } else {
                    segments.push(json!({
                        "type": "mention", "path": m.value, "tokenLength": token_len,
                    }));
                }
            }
            "slash-command" => segments.push(json!({"type": "slash-command", "command": m.value})),
            _ => {
                if let Some(prefix) = m.extra {
                    segments.push(json!({"type": "skill", "name": m.value, "prefix": prefix}));
                } else {
                    segments.push(json!({"type": "skill", "name": m.value}));
                }
            }
        }
        cursor = m.end;
    }
    if cursor < utf16_len(text) {
        append_text_segment(&mut segments, &u16_slice(text, cursor, utf16_len(text)));
    }
    Ok(segments)
}

// ---------- exported ports ----------

/// Port of `matchComposerLinkToken` (`apps/web/src/composer-editor-mentions.ts:100`).
pub fn match_composer_link_token(
    text: String,
    options: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if !text.contains("http") && !text.contains('.') {
        return Ok(None);
    }
    let js_opt = match &options {
        Value::Null => "null",
        v if crate::js::is_undefined(v) => "undefined",
        _ => "",
    };
    if !js_opt.is_empty() {
        return Err(format!(
            "{js_opt} is not an object (evaluating 'options.includeTrailingTokenAtEnd')"
        ));
    }
    let include_end = match options.get("includeTrailingTokenAtEnd") {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(true),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    };
    let link_re = link_regex();
    for m in link_re.find_iter(&text) {
        if !link_match_ok(&text, &m, include_end) {
            continue;
        }
        let raw = trim_trailing_link_punctuation(m.as_str());
        let Some(url) = normalize_composer_link_url(raw) else {
            continue;
        };
        let start = byte_to_u16(&text, m.start());
        return Ok(Some(json!({
            "url": url,
            "start": start,
            "end": start + utf16_len(raw),
        })));
    }
    Ok(None)
}

/// Port of `matchComposerSlashCommandChipToken` (`apps/web/src/composer-editor-mentions.ts:167`).
pub fn match_composer_slash_command_chip_token(
    text: String,
) -> Result<Option<serde_json::Value>, String> {
    let re = Regex::new(r"(?i)(^|\s)/([a-zA-Z][a-zA-Z0-9_-]*)").unwrap();
    for caps in re.captures_iter(&text) {
        let whole = caps.get(0).unwrap();
        // (?=\s) lookahead
        if !matches!(char_after(&text, whole.end()), Some(c) if is_js_ws(c)) {
            continue;
        }
        let ws = caps.get(1).map(|g| g.as_str()).unwrap_or("");
        let command = caps
            .get(2)
            .map(|g| g.as_str().to_lowercase())
            .unwrap_or_default();
        if !BUILT_IN_COMMANDS.contains(&command.as_str())
            || !CHIP_COMMANDS.contains(&command.as_str())
        {
            // JS `.exec` returns only the first match; a non-chip name yields null
            // even when a later chip command exists in the same text.
            return Ok(None);
        }
        let start = byte_to_u16(&text, whole.start()) + utf16_len(ws);
        return Ok(Some(json!({
            "command": command,
            "start": start,
            "end": start + utf16_len(&command) + 1,
        })));
    }
    Ok(None)
}

/// Port of `splitPromptIntoDisplaySegments` (`apps/web/src/composer-editor-mentions.ts:382`).
pub fn split_prompt_into_display_segments(
    prompt: String,
    mention_references: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    split_text_into_prompt_segments(&prompt, true, &mention_references)
}

/// Port of `splitPromptIntoComposerSegments` (`apps/web/src/composer-editor-mentions.ts:396`).
pub fn split_prompt_into_composer_segments(
    prompt: String,
    terminal_contexts: serde_json::Value,
    mention_references: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    if prompt.is_empty() {
        return Ok(Vec::new());
    }
    let contexts = match &terminal_contexts {
        Value::Array(a) => a.clone(),
        _ => Vec::new(),
    };
    let refs = &mention_references;
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    let mut ctx_index = 0usize;
    let mut pos = 0usize;
    for c in prompt.chars() {
        let next = pos + c.len_utf16();
        if c == INLINE_TERMINAL_CONTEXT_PLACEHOLDER {
            if pos > cursor {
                segments.extend(split_text_into_prompt_segments(
                    &u16_slice(&prompt, cursor, pos),
                    false,
                    refs,
                )?);
            }
            let context = contexts.get(ctx_index).cloned().unwrap_or(Value::Null);
            segments.push(json!({"type": "terminal-context", "context": context}));
            ctx_index += 1;
            cursor = next;
        }
        pos = next;
    }
    if cursor < utf16_len(&prompt) {
        segments.extend(split_text_into_prompt_segments(
            &u16_slice(&prompt, cursor, utf16_len(&prompt)),
            false,
            refs,
        )?);
    }
    Ok(segments)
}
