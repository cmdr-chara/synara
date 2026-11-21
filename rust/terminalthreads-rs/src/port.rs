//! The Rust port of `packages/shared/src/terminalThreads.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

use crate::{js, utf16};

const GENERIC_TERMINAL_THREAD_TITLE: &str = "New terminal";
const DOCK_TERMINAL_SCOPE_PREFIX: &str = "dock-terminal:";
const MAX_TERMINAL_INPUT_BUFFER_LENGTH: usize = 512;
const MAX_TERMINAL_TITLE_LENGTH: usize = 48;

const WRAPPER_COMMANDS: [&str; 6] = ["builtin", "command", "env", "noglob", "nocorrect", "sudo"];
const IGNORED_TERMINAL_TITLE_COMMANDS: [&str; 15] = [
    ".", "alias", "cd", "clear", "exit", "export", "history", "la", "ll", "logout", "ls", "pwd",
    "reset", "source", "unalias",
];
const PACKAGE_MANAGERS: [&str; 4] = ["bun", "npm", "pnpm", "yarn"];
const PACKAGE_MANAGER_SUBCOMMANDS: [&str; 4] = ["create", "dlx", "exec", "run"];
const SCRIPT_EXTENSIONS: [&str; 9] = [
    ".cjs", ".cts", ".js", ".jsx", ".mjs", ".mts", ".py", ".ts", ".tsx",
];

fn trim_js(s: &str) -> &str {
    s.trim_matches(jscore::is_space)
}

fn is_ascii_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn boundary_before(bytes: &[u8], pos: usize) -> bool {
    pos == 0 || !is_ascii_word_byte(bytes[pos - 1])
}

fn boundary_after(bytes: &[u8], pos: usize) -> bool {
    pos >= bytes.len() || !is_ascii_word_byte(bytes[pos])
}

/// `/<phrase>/i` wrapped in ASCII `\b` boundaries.
fn contains_word_phrase(lower: &str, phrase: &str) -> bool {
    let bytes = lower.as_bytes();
    lower
        .match_indices(phrase)
        .any(|(i, _)| boundary_before(bytes, i) && boundary_after(bytes, i + phrase.len()))
}

/// `/\bantigravity(?: cli)?\b/i`
fn contains_antigravity(lower: &str) -> bool {
    let bytes = lower.as_bytes();
    const NAME: &str = "antigravity";
    lower.match_indices(NAME).any(|(i, _)| {
        if !boundary_before(bytes, i) {
            return false;
        }
        let after = i + NAME.len();
        if bytes[after..].starts_with(b" cli") && boundary_after(bytes, after + 4) {
            return true;
        }
        boundary_after(bytes, after)
    })
}

/// Strips a trailing ` \d+` group (for `^(?:...)(?: \d+)?$` patterns).
fn strip_trailing_space_digits(s: &str) -> &str {
    if let Some(pos) = s.rfind(' ') {
        let digits = &s[pos + 1..];
        if !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()) {
            return &s[..pos];
        }
    }
    s
}

/// `/^[A-Za-z_][A-Za-z0-9_]*=.*/`
fn is_env_assignment_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes
        .first()
        .is_none_or(|b| !(b.is_ascii_alphabetic() || *b == b'_'))
    {
        return false;
    }
    let mut i = 1;
    while i < bytes.len() && is_ascii_word_byte(bytes[i]) {
        i += 1;
    }
    bytes.get(i) == Some(&b'=')
}

fn normalize_command_token(token: &str) -> String {
    let normalized_path = token.replace('\\', "/");
    for segment in normalized_path.split('/').rev() {
        if !segment.is_empty() {
            return segment.to_lowercase();
        }
    }
    normalized_path.to_lowercase()
}

fn strip_script_extension(token: &str) -> &str {
    for ext in SCRIPT_EXTENSIONS {
        if let Some(stripped) = token.strip_suffix(ext) {
            return stripped;
        }
    }
    token
}

fn cli_kind_from_normalized_token(token: &str) -> Option<&'static str> {
    let lowered = trim_js(token).to_lowercase();
    let normalized = strip_script_extension(&lowered);
    if normalized.is_empty() {
        return None;
    }
    match normalized {
        "codex" | "codex-cli" | "@openai/codex" => Some("codex"),
        "claude" | "claude-code" | "claude_code" | "@anthropic-ai/claude-code" => Some("claude"),
        "agy" | "antigravity" | "antigravity-cli" => Some("antigravity"),
        _ => None,
    }
}

fn cli_kind_from_token_list(tokens: &[String]) -> Option<&'static str> {
    tokens
        .iter()
        .find_map(|t| cli_kind_from_normalized_token(&normalize_command_token(t)))
}

fn cli_kind_from_process_text(text: Option<&str>) -> Option<&'static str> {
    let normalized = trim_js(text?).to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    if normalized.contains("@openai/codex") {
        return Some("codex");
    }
    if normalized.contains("@anthropic-ai/claude-code")
        || normalized.contains("anthropic/claude-code")
    {
        return Some("claude");
    }
    if normalized.contains("google-antigravity/antigravity-cli") {
        return Some("antigravity");
    }
    None
}

fn tokenize_shell_command(command: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escape_next = false;

    for c in trim_js(command).chars() {
        if escape_next {
            current.push(c);
            escape_next = false;
            continue;
        }
        if c == '\\' {
            escape_next = quote != Some('\'');
            if !escape_next {
                current.push(c);
            }
            continue;
        }
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                current.push(c);
            }
            continue;
        }
        if c == '\'' || c == '"' {
            quote = Some(c);
            continue;
        }
        if jscore::is_space(c) {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(c);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn strip_shell_prefixes(tokens: &[String]) -> Vec<String> {
    let mut start = 0;
    while start < tokens.len() && is_env_assignment_token(&tokens[start]) {
        start += 1;
    }
    while start < tokens.len()
        && WRAPPER_COMMANDS.contains(&normalize_command_token(&tokens[start]).as_str())
    {
        start += 1;
        while start < tokens.len() && is_env_assignment_token(&tokens[start]) {
            start += 1;
        }
    }
    tokens[start..].to_vec()
}

fn unwrap_executor_command(tokens: Vec<String>) -> Vec<String> {
    let first = tokens
        .first()
        .map_or(String::new(), |t| normalize_command_token(t));
    let second = tokens
        .get(1)
        .map_or(String::new(), |t| normalize_command_token(t));
    if (first == "npx" || first == "bunx") && tokens.get(1).is_some_and(|s| !s.is_empty()) {
        return tokens[1..].to_vec();
    }
    if first == "pnpm" && second == "dlx" && tokens.get(2).is_some_and(|s| !s.is_empty()) {
        return tokens[2..].to_vec();
    }
    if first == "npm" && second == "exec" && tokens.get(2).is_some_and(|s| !s.is_empty()) {
        return tokens[2..].to_vec();
    }
    tokens
}

fn package_manager_title(tokens: &[String]) -> Option<String> {
    let first = tokens.first().map(|t| normalize_command_token(t))?;
    if !PACKAGE_MANAGERS.contains(&first.as_str()) {
        return None;
    }
    let second = tokens.get(1).map(|t| normalize_command_token(t));
    let third = tokens.get(2).map(|t| normalize_command_token(t));
    match (&second, &third) {
        (Some(s), Some(t))
            if PACKAGE_MANAGER_SUBCOMMANDS.contains(&s.as_str()) && !t.is_empty() =>
        {
            Some(format!("{first} {s} {t}"))
        }
        (Some(s), _) if !s.is_empty() => Some(format!("{first} {s}")),
        _ => Some(first),
    }
}

fn truncate_terminal_title(title: &str) -> String {
    let units = utf16::units(title);
    if units.len() <= MAX_TERMINAL_TITLE_LENGTH {
        title.to_string()
    } else {
        utf16::slice(&units, 0, MAX_TERMINAL_TITLE_LENGTH)
            .trim_end_matches(jscore::is_space)
            .to_string()
    }
}

fn command_identity(title: &str, cli_kind: Option<&str>) -> Value {
    let mut m = Map::new();
    m.insert(
        "cliKind".to_string(),
        cli_kind.map_or(Value::Null, |s| Value::String(s.to_string())),
    );
    let icon_key = match cli_kind {
        Some("codex") => "openai",
        Some("claude") => "claude",
        Some("antigravity") => "antigravity",
        _ => "terminal",
    };
    m.insert("iconKey".to_string(), Value::String(icon_key.to_string()));
    m.insert("title".to_string(), Value::String(title.to_string()));
    Value::Object(m)
}

fn default_title_for_cli_kind(cli_kind: &str) -> &'static str {
    match cli_kind {
        "codex" => "Codex CLI",
        "claude" => "Claude Code",
        _ => "Antigravity CLI",
    }
}

fn infer_cli_kind_from_title(title: Option<&str>) -> Option<&'static str> {
    let normalized = trim_js(title?).to_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let base = strip_trailing_space_digits(&normalized);
    if base == "codex" || base == "codex cli" {
        return Some("codex");
    }
    if base == "claude" || base == "claude code" || normalized == "claude-code" {
        return Some("claude");
    }
    if matches!(base, "antigravity" | "antigravity cli" | "agy" | "agy cli") {
        return Some("antigravity");
    }
    if contains_word_phrase(&normalized, "openai codex")
        || contains_word_phrase(&normalized, "codex cli")
    {
        return Some("codex");
    }
    if contains_word_phrase(&normalized, "claude code") {
        return Some("claude");
    }
    if contains_antigravity(&normalized) || normalized == "agy" || normalized == "agy cli" {
        return Some("antigravity");
    }
    None
}

/// Port of `dockTerminalScopeId` (`packages/shared/src/terminalThreads.ts:9`).
pub fn dock_terminal_scope_id(host_thread_id: String) -> Result<String, String> {
    Ok(format!("{DOCK_TERMINAL_SCOPE_PREFIX}{host_thread_id}"))
}

/// Port of `terminalScopeIdsForThread` (`packages/shared/src/terminalThreads.ts:13`).
pub fn terminal_scope_ids_for_thread(host_thread_id: String) -> Result<Vec<String>, String> {
    Ok(vec![
        host_thread_id.clone(),
        format!("{DOCK_TERMINAL_SCOPE_PREFIX}{host_thread_id}"),
    ])
}

/// Port of `isGenericTerminalThreadTitle` (`packages/shared/src/terminalThreads.ts:40`).
pub fn is_generic_terminal_thread_title(title: Option<String>) -> Result<bool, String> {
    Ok(trim_js(title.as_deref().unwrap_or("")) == GENERIC_TERMINAL_THREAD_TITLE)
}

/// Port of `defaultTerminalTitleForCliKind` (`packages/shared/src/terminalThreads.ts:270`).
pub fn default_terminal_title_for_cli_kind(cli_kind: String) -> Result<String, String> {
    Ok(default_title_for_cli_kind(&cli_kind).to_string())
}

/// Port of `managedTerminalCommandNameForCliKind` (`packages/shared/src/terminalThreads.ts:278`).
pub fn managed_terminal_command_name_for_cli_kind(
    cli_kind: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(match cli_kind.as_str() {
        Some("codex") => Value::String("codex".to_string()),
        Some("claude") => Value::String("claude".to_string()),
        _ => js::undefined(),
    })
}

/// Port of `terminalCliKindFromValue` (`packages/shared/src/terminalThreads.ts:282`).
pub fn terminal_cli_kind_from_value(value: Option<String>) -> Result<Option<String>, String> {
    let normalized = value
        .map(|v| trim_js(&v).to_lowercase())
        .unwrap_or_default();
    Ok(match normalized.as_str() {
        "codex" | "claude" | "antigravity" => Some(normalized),
        _ => None,
    })
}

/// Port of `deriveTerminalProcessIdentity` (`packages/shared/src/terminalThreads.ts:292`).
pub fn derive_terminal_process_identity(
    command: Option<String>,
) -> Result<Option<serde_json::Value>, String> {
    let stripped = trim_js(&command.unwrap_or_default()).to_string();
    if stripped.is_empty() {
        return Ok(None);
    }
    let cli_kind = cli_kind_from_token_list(&tokenize_shell_command(&stripped))
        .or_else(|| cli_kind_from_process_text(Some(&stripped)));
    Ok(cli_kind.map(|kind| command_identity(default_title_for_cli_kind(kind), Some(kind))))
}

/// Port of `deriveTerminalCommandIdentity` (`packages/shared/src/terminalThreads.ts:339`).
pub fn derive_terminal_command_identity(
    command: String,
) -> Result<Option<serde_json::Value>, String> {
    let stripped = trim_js(&command);
    if stripped.is_empty() {
        return Ok(None);
    }
    let base_tokens = strip_shell_prefixes(&tokenize_shell_command(stripped));
    if base_tokens.is_empty() {
        return Ok(None);
    }
    let tokens = unwrap_executor_command(base_tokens);
    let normalized: Vec<String> = tokens.iter().map(|t| normalize_command_token(t)).collect();
    let first = normalized.first().cloned().unwrap_or_default();
    let second = normalized.get(1).cloned().unwrap_or_default();
    if first.is_empty() || IGNORED_TERMINAL_TITLE_COMMANDS.contains(&first.as_str()) {
        return Ok(None);
    }
    let detected = cli_kind_from_token_list(&tokens);
    if detected == Some("codex") {
        return Ok(Some(command_identity("Codex CLI", Some("codex"))));
    }
    if detected == Some("claude") || (first == "claude" && second == "code") {
        return Ok(Some(command_identity("Claude Code", Some("claude"))));
    }
    if detected == Some("antigravity") {
        return Ok(Some(command_identity(
            "Antigravity CLI",
            Some("antigravity"),
        )));
    }
    if first == "git" {
        let title = if second.is_empty() {
            "git".to_string()
        } else {
            format!("git {second}")
        };
        return Ok(Some(command_identity(
            &truncate_terminal_title(&title),
            None,
        )));
    }
    if let Some(title) = package_manager_title(&tokens) {
        return Ok(Some(command_identity(
            &truncate_terminal_title(&title),
            None,
        )));
    }
    let generic_title: String = normalized[..normalized.len().min(2)]
        .join(" ")
        .trim_matches(jscore::is_space)
        .to_string();
    if generic_title.is_empty() {
        return Ok(None);
    }
    Ok(Some(command_identity(
        &truncate_terminal_title(&generic_title),
        None,
    )))
}

/// Port of `deriveTerminalTitleFromCommand` (`packages/shared/src/terminalThreads.ts:387`).
pub fn derive_terminal_title_from_command(command: String) -> Result<Option<String>, String> {
    Ok(derive_terminal_command_identity(command)?
        .and_then(|v| v.get("title").and_then(Value::as_str).map(str::to_string)))
}

/// Port of `consumeTerminalIdentityInput` (`packages/shared/src/terminalThreads.ts:392`).
pub fn consume_terminal_identity_input(
    buffer: String,
    data: String,
) -> Result<serde_json::Value, String> {
    if data.contains('\u{1b}') {
        let mut m = Map::new();
        m.insert("buffer".to_string(), Value::String(buffer));
        m.insert("identity".to_string(), Value::Null);
        return Ok(Value::Object(m));
    }
    let mut next_buffer: Vec<u16> = utf16::units(&buffer);
    let mut next_identity: Option<Value> = None;
    for c in data.chars() {
        if c == '\r' || c == '\n' {
            next_identity =
                derive_terminal_command_identity(String::from_utf16_lossy(&next_buffer))?;
            next_buffer.clear();
            continue;
        }
        if c == '\u{8}' || c == '\u{7f}' {
            next_buffer.pop();
            continue;
        }
        if c == '\t' {
            next_buffer.push(' ' as u16);
            continue;
        }
        if c == '\u{3}' || c == '\u{4}' || c == '\u{15}' {
            next_buffer.clear();
            continue;
        }
        if c >= ' ' {
            let mut pair = [0u16; 2];
            next_buffer.extend_from_slice(c.encode_utf16(&mut pair));
        }
    }
    let start = next_buffer
        .len()
        .saturating_sub(MAX_TERMINAL_INPUT_BUFFER_LENGTH);
    let mut m = Map::new();
    m.insert(
        "buffer".to_string(),
        js::utf16_string(&next_buffer[start..]),
    );
    m.insert("identity".to_string(), next_identity.unwrap_or(Value::Null));
    Ok(Value::Object(m))
}

/// Port of `consumeTerminalTitleInput` (`packages/shared/src/terminalThreads.ts:432`).
pub fn consume_terminal_title_input(
    buffer: String,
    data: String,
) -> Result<serde_json::Value, String> {
    let state = consume_terminal_identity_input(buffer, data)?;
    let title = state
        .get("identity")
        .and_then(|i| i.get("title"))
        .cloned()
        .unwrap_or(Value::Null);
    let mut m = Map::new();
    m.insert(
        "buffer".to_string(),
        state.get("buffer").cloned().unwrap_or(Value::Null),
    );
    m.insert("title".to_string(), title);
    Ok(Value::Object(m))
}

/// Port of `resolveTerminalVisualIdentity` (`packages/shared/src/terminalThreads.ts:444`).
pub fn resolve_terminal_visual_identity(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'cliKind')".to_string());
    }
    if js::is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'cliKind')".to_string());
    }
    let cli_kind_raw = input.get("cliKind").cloned().unwrap_or_else(js::undefined);
    let resolved_cli_kind = if js::is_undefined(&cli_kind_raw) {
        infer_cli_kind_from_title(input.get("title").and_then(Value::as_str))
            .map(|k| Value::String(k.to_string()))
            .unwrap_or(Value::Null)
    } else {
        cli_kind_raw
    };
    let title = match input.get("title") {
        Some(Value::String(s)) => Some(trim_js(s).to_string()),
        Some(v) if js::is_undefined(v) || v.is_null() => None,
        Some(_) => {
            return Err("input.title?.trim is not a function".to_string());
        }
        None => None,
    };
    let title = match title.filter(|t| !t.is_empty()) {
        Some(t) => Value::String(t),
        None => {
            if jscore::js_truthy(&resolved_cli_kind) {
                Value::String(
                    default_title_for_cli_kind(resolved_cli_kind.as_str().unwrap_or(""))
                        .to_string(),
                )
            } else {
                input
                    .get("fallbackTitle")
                    .cloned()
                    .unwrap_or_else(js::undefined)
            }
        }
    };
    let cli_kind = if js::is_undefined(&resolved_cli_kind) {
        Value::Null
    } else {
        resolved_cli_kind.clone()
    };
    let icon_key = match cli_kind.as_str() {
        Some("codex") => "openai",
        Some("claude") => "claude",
        Some("antigravity") => "antigravity",
        _ => "terminal",
    };
    let state = input
        .get("state")
        .filter(|v| !v.is_null() && !js::is_undefined(v))
        .cloned()
        .unwrap_or_else(|| {
            if jscore::js_truthy(input.get("isRunning").unwrap_or(&Value::Null)) {
                Value::String("running".to_string())
            } else {
                Value::String("idle".to_string())
            }
        });
    let mut m = Map::new();
    m.insert("cliKind".to_string(), cli_kind);
    m.insert("iconKey".to_string(), Value::String(icon_key.to_string()));
    m.insert("state".to_string(), state);
    m.insert("title".to_string(), title);
    Ok(Value::Object(m))
}
