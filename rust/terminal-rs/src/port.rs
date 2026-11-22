//! The Rust port of `apps/server/src/terminal/`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::sync::OnceLock;

fn marker_value() -> &'static Value {
    static M: OnceLock<Value> = OnceLock::new();
    M.get_or_init(|| json!({"$undefined": true}))
}

fn prop<'a>(v: &'a Value, key: &str) -> &'a Value {
    match v {
        Value::Object(m) => m.get(key).unwrap_or_else(|| marker_value()),
        _ => marker_value(),
    }
}

fn is_undefined_marker(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined").and_then(Value::as_bool) == Some(true))
}

fn is_undef_or_null(v: &Value) -> bool {
    v.is_null() || is_undefined_marker(v)
}

fn undef_err(key: &str, v: &Value) -> String {
    if is_undefined_marker(v) {
        format!("Cannot read properties of undefined (reading '{key}')")
    } else {
        format!("Cannot read properties of null (reading '{key}')")
    }
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => !is_undefined_marker(v),
    }
}

fn js_to_string_coerce(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.as_f64().map_or_else(|| "null".to_string(), js_f64_repr),
        Value::Array(a) => a
            .iter()
            .map(js_to_string_coerce)
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(m) => {
            if m.get("$undefined").and_then(Value::as_bool) == Some(true) {
                "undefined".to_string()
            } else if let Some(num) = m.get("$num") {
                num.as_str().unwrap_or("0").to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_f64_repr(f: f64) -> String {
    if f.is_nan() {
        "NaN".to_string()
    } else if f.is_infinite() {
        if f > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        }
    } else if f.fract() == 0.0 && f.abs() < 1e21 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}

fn js_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => None,
    }
}

fn js_f64(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Object(m) => m
            .get("$num")
            .and_then(Value::as_str)
            .map(|s| match s {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                "-0" => -0.0,
                _ => s.parse().unwrap_or(f64::NAN),
            })
            .unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

// ---------- terminalHistory.ts ----------

fn js_slice_start(arg: f64, len: usize) -> usize {
    // JS Array.prototype.slice: ToIntegerOrInfinity, then clamp.
    let idx = if arg.is_nan() { 0.0 } else { arg.trunc() };
    if idx < 0.0 {
        (len as f64 + idx).max(0.0) as usize
    } else {
        (idx.min(len as f64)) as usize
    }
}

/// Port of `capHistoryLines` (`apps/server/src/terminal/terminalHistory.ts`).
pub fn cap_history_lines(history: String, max_lines: f64) -> Result<String, String> {
    if history.is_empty() {
        return Ok(history);
    }
    let has_trailing_newline = history.ends_with('\n');
    let mut lines: Vec<&str> = history.split('\n').collect();
    if has_trailing_newline {
        lines.pop();
    }
    if (lines.len() as f64) <= max_lines {
        return Ok(history);
    }
    let start = js_slice_start(lines.len() as f64 - max_lines, lines.len());
    let capped = lines[start..].join("\n");
    Ok(if has_trailing_newline {
        format!("{capped}\n")
    } else {
        capped
    })
}

/// Port of `capHistoryBytes` (`apps/server/src/terminal/terminalHistory.ts`).
pub fn cap_history_bytes(
    history: String,
    max_bytes: f64,
    scan_window: Option<f64>,
) -> Result<String, String> {
    if history.is_empty() {
        return Ok(history);
    }
    if max_bytes <= 0.0 {
        return Ok(String::new());
    }
    let scan_window = scan_window.unwrap_or(65_536.0);
    let buf = history.as_bytes();
    if (buf.len() as f64) <= max_bytes {
        return Ok(history);
    }
    let cut = buf.len() as f64 - max_bytes;
    // JS Math.min propagates NaN (Rust f64::min does not).
    let scan_limit = if cut.is_nan() || scan_window.is_nan() {
        f64::NAN
    } else {
        (buf.len() as f64).min(cut + scan_window)
    };
    // buf[i] with a non-integer i is undefined in JS.
    let byte_at = |i: f64| -> Option<u8> {
        if i.fract() == 0.0 && i >= 0.0 {
            buf.get(i as usize).copied()
        } else {
            None
        }
    };
    let mut boundary: f64 = -1.0;
    let mut index = cut;
    while index < scan_limit {
        let byte = byte_at(index);
        if byte == Some(0x1b) {
            boundary = index;
            break;
        }
        if byte == Some(0x0a) {
            boundary = index + 1.0;
            break;
        }
        index += 1.0;
    }
    if boundary == -1.0 {
        boundary = cut;
        while boundary < buf.len() as f64 {
            match byte_at(boundary) {
                None => break,
                Some(byte) if (byte & 0xc0) != 0x80 => break,
                _ => boundary += 1.0,
            }
        }
    }
    // Buffer.prototype.subarray clamps the start index into [0, len] (NaN -> 0).
    let start = if boundary.is_nan() {
        0usize
    } else {
        (boundary.max(0.0) as usize).min(buf.len())
    };
    Ok(String::from_utf8_lossy(&buf[start..]).into_owned())
}

/// Port of `capHistoryByLimits` (`apps/server/src/terminal/terminalHistory.ts`).
pub fn cap_history_by_limits(history: String, limits: serde_json::Value) -> Result<String, String> {
    let max_bytes = if limits.is_null() || is_undefined_marker(&limits) {
        return Err(undef_err("maxBytes", &limits));
    } else {
        js_f64(prop(&limits, "maxBytes"))
    };
    let byte_capped = cap_history_bytes(history, max_bytes, None)?;
    let max_lines = js_f64(prop(&limits, "maxLines"));
    cap_history_lines(byte_capped, max_lines)
}

// ---------- executable.ts (posix subset) ----------

fn env_key_present(env: &Value, key: &str) -> bool {
    matches!(env, Value::Object(m) if m.contains_key(key))
}

fn env_path_key_for(env: &Value) -> Result<&'static str, String> {
    if !matches!(env, Value::Object(_) | Value::Array(_)) {
        return Err(format!(
            "Cannot use 'in' operator to search for 'PATH' in {}",
            js_to_string_coerce(env)
        ));
    }
    if env_key_present(env, "PATH") {
        return Ok("PATH");
    }
    if env_key_present(env, "Path") {
        return Ok("Path");
    }
    if env_key_present(env, "path") {
        // POSIX: lowercase `path` is ignored; a usable PATH must be created.
        return Ok("PATH");
    }
    Ok("PATH")
}

fn env_value<'a>(env: &'a Value, key: &str) -> &'a Value {
    match env {
        Value::Object(m) => m.get(key).unwrap_or_else(|| marker_value()),
        _ => marker_value(),
    }
}

fn path_entries(env: &Value) -> Result<Vec<String>, String> {
    let path_value = {
        let v = env_value(env, "PATH");
        if is_undefined_marker(v) {
            let v = env_value(env, "Path");
            if is_undefined_marker(v) {
                env_value(env, "path")
            } else {
                v
            }
        } else {
            v
        }
    };
    if is_undef_or_null(path_value) {
        return Ok(vec!["/usr/bin".to_string(), "/bin".to_string()]);
    }
    let s = match path_value {
        Value::String(s) => s.clone(),
        _ => return Err("pathValue.split is not a function".to_string()),
    };
    if s.is_empty() {
        return Ok(vec![]);
    }
    Ok(s.split(':')
        .map(|e| e.trim().trim_matches('"').to_string())
        .filter(|e| !e.is_empty())
        .collect())
}

fn is_executable_file(p: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs_meta(p).is_some_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn fs_meta(p: &std::path::Path) -> Option<std::fs::Metadata> {
    std::fs::metadata(p).ok()
}

fn process_env_value() -> Value {
    let mut m = Map::new();
    for (k, v) in std::env::vars() {
        m.insert(k, Value::String(v));
    }
    Value::Object(m)
}

fn resolve_executable(command: &str, env: &Value) -> Result<Option<String>, String> {
    let env = if is_undef_or_null(env) {
        process_env_value()
    } else {
        env.clone()
    };
    if command.contains('/') || command.contains('\\') {
        // Qualified command: PATH is not consulted.
        if is_executable_file(std::path::Path::new(command)) {
            return Ok(Some(command.to_string()));
        }
        return Ok(None);
    }
    for dir in path_entries(&env)? {
        let candidate = format!("{dir}/{command}");
        if is_executable_file(std::path::Path::new(&candidate)) {
            return Ok(Some(candidate));
        }
    }
    Ok(None)
}

fn path_join(a: &Value, b: &str) -> Result<String, String> {
    match a {
        Value::String(s) => Ok(if s.is_empty() || s.ends_with('/') {
            format!("{s}{b}")
        } else {
            format!("{s}/{b}")
        }),
        _ => Err(format!(
            "The \"path\" argument must be of type string or an instance of Buffer or URL. Received {}",
            js_to_string_coerce(a)
        )),
    }
}

// ---------- privatePathPermissions.ts ----------

fn ensure_private_directory_sync(dir: &Value) -> Result<(), String> {
    let s = match dir {
        Value::String(s) => s.clone(),
        _ => {
            return Err(format!(
                "Failed to create private path {}",
                js_to_string_coerce(dir)
            ));
        }
    };
    if std::fs::create_dir_all(&s).is_err() {
        return Err(format!("Failed to create private path {s}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&s, std::fs::Permissions::from_mode(0o700));
    }
    Ok(())
}

fn write_file_if_changed(path: &str, content: &str, mode: u32) -> Result<(), String> {
    let p = std::path::Path::new(path);
    let current = std::fs::read_to_string(p).ok();
    if current.as_deref() != Some(content) && std::fs::write(p, content).is_err() {
        return Err(format!("Failed to create private path {path}"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(mode));
    }
    Ok(())
}

// ---------- terminalThreads.ts helpers ----------

fn shell_quote(value: &Value) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(format!("'{}'", s.replace('\'', "'\"'\"'"))),
        _ => Err("value.replaceAll is not a function".to_string()),
    }
}

fn hook_osc(event: &str) -> String {
    format!("\\033]633;SYNARA_AGENT_EVENT={event}\\007")
}

fn notify_hook_script() -> String {
    let start = hook_osc("Start");
    let stop = hook_osc("Stop");
    let perm = hook_osc("PermissionRequest");
    format!(
        r#"#!/bin/sh
set -eu
if [ "$#" -gt 0 ]; then
  _synara_hook_input="$1"
else
  _synara_hook_input="$(cat)"
fi

_synara_extract_event() {{
  printf '%s' "$_synara_hook_input" | sed -n "s/.*\"$1\"[[:space:]]*:[[:space:]]*\"\([^\"]*\)\".*/\1/p" | head -n 1
}}

_synara_event="$(_synara_extract_event hook_event_name)"
if [ -z "$_synara_event" ]; then
  _synara_type="$(_synara_extract_event type)"
  case "$_synara_type" in
    task_started|userPromptSubmitted|user_prompt_submit)
      _synara_event="Start"
      ;;
    task_complete|agent-turn-complete|stop|session_end|sessionEnd)
      _synara_event="Stop"
      ;;
    exec_approval_request|apply_patch_approval_request|request_user_input)
      _synara_event="PermissionRequest"
      ;;
  esac
fi

_synara_emit_osc() {{
  _synara_sequence="$1"
  if [ -w /dev/tty ]; then
    printf '%b' "$_synara_sequence" > /dev/tty 2>/dev/null || printf '%b' "$_synara_sequence"
    return
  fi
  printf '%b' "$_synara_sequence"
}}

case "$_synara_event" in
  UserPromptSubmit|PostToolUse|PostToolUseFailure|Start)
    _synara_emit_osc '{start}'
    ;;
  Stop)
    _synara_emit_osc '{stop}'
    ;;
  PermissionRequest|PreToolUse|Notification)
    _synara_emit_osc '{perm}'
    ;;
esac
"#
    )
}

fn claude_settings_json(notify_hook_path: &str) -> String {
    let hook = json!([{"hooks": [{"type": "command", "command": notify_hook_path}]}]);
    let star =
        json!([{"matcher": "*", "hooks": [{"type": "command", "command": notify_hook_path}]}]);
    let body = json!({"hooks": {
        "UserPromptSubmit": hook,
        "Stop": json!([{"hooks": [{"type": "command", "command": notify_hook_path}]}]),
        "PostToolUse": star,
        "PostToolUseFailure": json!([{"matcher": "*", "hooks": [{"type": "command", "command": notify_hook_path}]}]),
        "PermissionRequest": json!([{"matcher": "*", "hooks": [{"type": "command", "command": notify_hook_path}]}]),
        "Notification": json!([{"matcher": "*", "hooks": [{"type": "command", "command": notify_hook_path}]}]),
    }});
    serde_json::to_string_pretty(&body).unwrap_or_default()
}

fn codex_hooks_json(notify_hook_path: &str) -> String {
    let body = json!({"hooks": {
        "UserPromptSubmit": [{"hooks": [{"type": "command", "command": notify_hook_path}]}],
        "Stop": [{"hooks": [{"type": "command", "command": notify_hook_path}]}],
    }});
    serde_json::to_string_pretty(&body).unwrap_or_default()
}

fn codex_wrapper_script(codex_home_dir: &str, notify_hook_path: &str, target_path: &str) -> String {
    let q = |s: &str| format!("'{}'", s.replace('\'', "'\"'\"'"));
    let notify_arg = format!(
        "notify=[\"bash\",{}]",
        serde_json::to_string(&json!(notify_hook_path)).unwrap_or_default()
    );
    [
        format!("export CODEX_HOME={}", q(codex_home_dir)),
        format!("if [ -f {} ]; then", q(notify_hook_path)),
        "  export CODEX_TUI_RECORD_SESSION=1".to_string(),
        "  if [ -z \"${CODEX_TUI_SESSION_LOG_PATH:-}\" ]; then".to_string(),
        "    _synara_codex_ts=\"$(date +%s 2>/dev/null || echo \"$$\")\"".to_string(),
        "    export CODEX_TUI_SESSION_LOG_PATH=\"${TMPDIR:-/tmp}/synara-codex-session-$$_${_synara_codex_ts}.jsonl\"".to_string(),
        "  fi".to_string(),
        "  (".to_string(),
        "    _synara_log=\"$CODEX_TUI_SESSION_LOG_PATH\"".to_string(),
        format!("    _synara_notify={}", q(notify_hook_path)),
        "    _synara_last_turn_id=\"\"".to_string(),
        "    _synara_last_approval_id=\"\"".to_string(),
        "    _synara_last_exec_call_id=\"\"".to_string(),
        "    _synara_approval_fallback_seq=0".to_string(),
        String::new(),
        "    _synara_emit_event() {".to_string(),
        "      _synara_event=\"$1\"".to_string(),
        "      _synara_payload=$(printf '{\"hook_event_name\":\"%s\"}' \"$_synara_event\")".to_string(),
        "      \"$_synara_notify\" \"$_synara_payload\" >/dev/null 2>&1 || true".to_string(),
        "    }".to_string(),
        String::new(),
        "    _synara_i=0".to_string(),
        "    while [ ! -f \"$_synara_log\" ] && [ \"$_synara_i\" -lt 200 ]; do".to_string(),
        "      _synara_i=$((_synara_i + 1))".to_string(),
        "      sleep 0.05".to_string(),
        "    done".to_string(),
        "    if [ ! -f \"$_synara_log\" ]; then".to_string(),
        "      exit 0".to_string(),
        "    fi".to_string(),
        String::new(),
        "    tail -n 0 -F \"$_synara_log\" 2>/dev/null | while IFS= read -r _synara_line; do".to_string(),
        "      case \"$_synara_line\" in".to_string(),
        "        *'\"dir\":\"to_tui\"'*'\"kind\":\"codex_event\"'*'\"msg\":{\"type\":\"task_started\"'*)".to_string(),
        "          _synara_turn_id=$(printf '%s\\n' \"$_synara_line\" | awk -F'\"turn_id\":\"' 'NF > 1 { sub(/\".*/, \"\", $2); print $2; exit }')".to_string(),
        "          [ -n \"$_synara_turn_id\" ] || _synara_turn_id=\"task_started\"".to_string(),
        "          if [ \"$_synara_turn_id\" != \"$_synara_last_turn_id\" ]; then".to_string(),
        "            _synara_last_turn_id=\"$_synara_turn_id\"".to_string(),
        "            _synara_emit_event \"Start\"".to_string(),
        "          fi".to_string(),
        "          ;;".to_string(),
        "        *'\"dir\":\"to_tui\"'*'\"kind\":\"codex_event\"'*'\"msg\":{\"type\":\"'*'_approval_request\"'*)".to_string(),
        "          _synara_approval_id=$(printf '%s\\n' \"$_synara_line\" | awk -F'\"id\":\"' 'NF > 1 { sub(/\".*/, \"\", $2); print $2; exit }')".to_string(),
        "          [ -n \"$_synara_approval_id\" ] || _synara_approval_id=$(printf '%s\\n' \"$_synara_line\" | awk -F'\"approval_id\":\"' 'NF > 1 { sub(/\".*/, \"\", $2); print $2; exit }')".to_string(),
        "          [ -n \"$_synara_approval_id\" ] || _synara_approval_id=$(printf '%s\\n' \"$_synara_line\" | awk -F'\"call_id\":\"' 'NF > 1 { sub(/\".*/, \"\", $2); print $2; exit }')".to_string(),
        "          if [ -z \"$_synara_approval_id\" ]; then".to_string(),
        "            _synara_approval_fallback_seq=$((_synara_approval_fallback_seq + 1))".to_string(),
        "            _synara_approval_id=\"approval_request_${_synara_approval_fallback_seq}\"".to_string(),
        "          fi".to_string(),
        "          if [ \"$_synara_approval_id\" != \"$_synara_last_approval_id\" ]; then".to_string(),
        "            _synara_last_approval_id=\"$_synara_approval_id\"".to_string(),
        "            _synara_emit_event \"PermissionRequest\"".to_string(),
        "          fi".to_string(),
        "          ;;".to_string(),
        "        *'\"dir\":\"to_tui\"'*'\"kind\":\"codex_event\"'*'\"msg\":{\"type\":\"exec_command_begin\"'*)".to_string(),
        "          _synara_exec_call_id=$(printf '%s\\n' \"$_synara_line\" | awk -F'\"call_id\":\"' 'NF > 1 { sub(/\".*/, \"\", $2); print $2; exit }')".to_string(),
        "          if [ -n \"$_synara_exec_call_id\" ]; then".to_string(),
        "            if [ \"$_synara_exec_call_id\" != \"$_synara_last_exec_call_id\" ]; then".to_string(),
        "              _synara_last_exec_call_id=\"$_synara_exec_call_id\"".to_string(),
        "              _synara_emit_event \"Start\"".to_string(),
        "            fi".to_string(),
        "          else".to_string(),
        "            _synara_emit_event \"Start\"".to_string(),
        "          fi".to_string(),
        "          ;;".to_string(),
        "      esac".to_string(),
        "    done".to_string(),
        "  ) &".to_string(),
        "  SYNARA_CODEX_START_WATCHER_PID=$!".to_string(),
        "fi".to_string(),
        format!("{} --enable codex_hooks -c {} \"$@\"", q(target_path), q(&notify_arg)),
        "_synara_status=$?".to_string(),
        "if [ -n \"${SYNARA_CODEX_START_WATCHER_PID:-}\" ]; then".to_string(),
        "  kill \"$SYNARA_CODEX_START_WATCHER_PID\" >/dev/null 2>&1 || true".to_string(),
        "  wait \"$SYNARA_CODEX_START_WATCHER_PID\" 2>/dev/null || true".to_string(),
        "fi".to_string(),
        "exit \"$_synara_status\"".to_string(),
    ]
    .join("\n")
}

fn managed_wrapper_script(
    claude_settings_path: &str,
    cli_kind: &str,
    codex_home_dir: &str,
    notify_hook_path: &str,
    target_path: &str,
) -> String {
    let q = |s: &str| format!("'{}'", s.replace('\'', "'\"'\"'"));
    let command_name = cli_kind; // codex -> "codex", claude -> "claude"
    let title = if cli_kind == "claude" {
        "Claude Code"
    } else {
        "Codex CLI"
    };
    let command_body = if cli_kind == "claude" {
        format!(
            "exec {} --settings {} \"$@\"",
            q(target_path),
            q(claude_settings_path)
        )
    } else {
        codex_wrapper_script(codex_home_dir, notify_hook_path, target_path)
    };
    [
        "#!/bin/sh".to_string(),
        format!("# Managed {command_name} wrapper injected by synara terminal sessions."),
        format!("printf '\\033]0;%s\\007' {}", q(title)),
        format!("export SYNARA_TERMINAL_CLI_KIND={}", q(cli_kind)),
        command_body,
        String::new(),
    ]
    .join("\n")
}

const PROFILE_INHERITED_ENV_KEYS: &[&str] = &[
    "ALL_PROXY",
    "CURL_CA_BUNDLE",
    "COLORTERM",
    "COMSPEC",
    "SYSTEMROOT",
    "WINDIR",
    "PATHEXT",
    "TEMP",
    "TMP",
    "FORCE_COLOR",
    "GIT_SSL_CAINFO",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "http_proxy",
    "https_proxy",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LOGNAME",
    "NODE_EXTRA_CA_CERTS",
    "NO_COLOR",
    "NO_PROXY",
    "no_proxy",
    "PATH",
    "REQUESTS_CA_BUNDLE",
    "SHELL",
    "SSL_CERT_DIR",
    "SSL_CERT_FILE",
    "SSH_AUTH_SOCK",
    "TERM",
    "TMPDIR",
    "TZ",
    "USER",
    "XDG_RUNTIME_DIR",
];

fn js_object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    if is_undef_or_null(v) {
        return Err("Cannot convert undefined or null to object".to_string());
    }
    Ok(match v {
        Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect(),
        Value::String(s) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => vec![],
    })
}

fn js_spread_object(target: &mut Map<String, Value>, v: &Value) {
    match v {
        Value::Object(m) => {
            if !is_undefined_marker(v) {
                for (k, val) in m {
                    target.insert(k.clone(), val.clone());
                }
            }
        }
        Value::Array(a) => {
            for (i, val) in a.iter().enumerate() {
                target.insert(i.to_string(), val.clone());
            }
        }
        Value::String(s) => {
            for (i, c) in s.chars().enumerate() {
                target.insert(i.to_string(), Value::String(c.to_string()));
            }
        }
        _ => {}
    }
}

/// CLDR-lite collation for `String.prototype.localeCompare` in en locale:
/// punctuation < symbols/emoji < digits < latin letters.
fn locale_compare(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    fn cat(c: char) -> u8 {
        if c.is_alphabetic() {
            3
        } else if c.is_numeric() {
            2
        } else if c.is_ascii_punctuation() {
            0
        } else {
            // symbols, emoji, whitespace, etc.
            1
        }
    }
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    for i in 0..ac.len().min(bc.len()) {
        let (x, y) = (cat(ac[i]), cat(bc[i]));
        if x != y {
            return x.cmp(&y);
        }
        let xl = ac[i].to_lowercase().next().unwrap_or(ac[i]);
        let yl = bc[i].to_lowercase().next().unwrap_or(bc[i]);
        if xl != yl {
            return xl.cmp(&yl);
        }
        if ac[i] != bc[i] {
            // tertiary: lowercase sorts before uppercase
            return if ac[i].is_lowercase() {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
    }
    ac.len().cmp(&bc.len())
}

/// Port of `buildProviderProfileProcessEnv` (`apps/server/src/terminal/managedTerminalWrappers.ts`).
pub fn build_provider_profile_process_env(
    profile: serde_json::Value,
    base_env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&profile) {
        return Err(undef_err("isolateEnvironment", &profile));
    }
    let mut out = Map::new();
    if js_truthy(prop(&profile, "isolateEnvironment")) {
        for (key, value) in js_object_entries(&base_env)? {
            if PROFILE_INHERITED_ENV_KEYS
                .iter()
                .any(|allowed| allowed.to_uppercase() == key.to_uppercase())
            {
                out.insert(key, value);
            }
        }
    } else {
        js_spread_object(&mut out, &base_env);
    }
    js_spread_object(&mut out, prop(&profile, "environment"));
    Ok(Value::Object(out))
}

/// Port of `buildProviderProfileWrapperScript` (`apps/server/src/terminal/managedTerminalWrappers.ts`).
pub fn build_provider_profile_wrapper_script(profile: serde_json::Value) -> Result<String, String> {
    let environment = prop(&profile, "environment");
    let mut fixed_environment = js_object_entries(environment)?;
    fixed_environment.sort_by(|a, b| locale_compare(&a.0, &b.0));
    let isolated = js_truthy(prop(&profile, "isolateEnvironment"));
    let command_name = js_to_string_coerce(prop(&profile, "commandName"));
    let target_path = prop(&profile, "targetPath").clone();
    let environment_arguments: Vec<String> = if isolated {
        let mut v = vec!["exec env -i \\".to_string()];
        for name in PROFILE_INHERITED_ENV_KEYS {
            v.push(format!("  {name}=\"${{{name}:-}}\" \\"));
        }
        for (name, value) in &fixed_environment {
            v.push(format!("  {}={} \\", name, shell_quote(value)?));
        }
        v.push(format!("  {} \"$@\"", shell_quote(&target_path)?));
        v
    } else {
        let mut v = Vec::new();
        for (name, value) in &fixed_environment {
            v.push(format!("export {}={}", name, shell_quote(value)?));
        }
        v.push(format!("exec {} \"$@\"", shell_quote(&target_path)?));
        v
    };
    let omitted = prop(&profile, "omittedSensitiveEnvironmentNames");
    let mut lines = vec![
        "#!/bin/sh".to_string(),
        format!("# Synara provider profile: {command_name}"),
    ];
    let omitted_len = match omitted {
        Value::Array(a) => a.len() as f64,
        Value::String(s) => s.chars().count() as f64,
        Value::Object(m) if is_undefined_marker(omitted) => {
            let _ = m;
            0.0
        }
        _ => 0.0,
    };
    if omitted_len > 0.0 {
        let names = match omitted {
            Value::Array(a) => a
                .iter()
                .map(js_to_string_coerce)
                .collect::<Vec<_>>()
                .join(", "),
            _ => js_to_string_coerce(omitted),
        };
        lines.push(format!(
            "# Sensitive environment is intentionally not serialized: {names}"
        ));
    }
    lines.extend(environment_arguments);
    lines.push(String::new());
    Ok(lines.join("\n"))
}

fn read_provider_profile_manifest(root_dir: &str) -> Vec<String> {
    let path = format!("{root_dir}/provider-profiles.json");
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => return vec![],
    };
    let parsed: Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return vec![],
    };
    match parsed {
        Value::Array(a) => a
            .into_iter()
            .filter_map(|v| match v {
                Value::String(s) => {
                    let mut ch = s.chars();
                    let valid = ch.next().is_some_and(|c| c.is_ascii_alphabetic())
                        && s.len() <= 128
                        && s.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
                    if valid { Some(s) } else { None }
                }
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

fn synchronize_provider_profile_wrappers(root_dir: &str, profiles: &Value) -> Result<(), String> {
    let profile_list: Vec<&Value> = match profiles {
        Value::Array(a) => a.iter().collect(),
        _ if is_undef_or_null(profiles) => vec![],
        _ => return Err("profiles.map is not a function".to_string()),
    };
    let next_names: Vec<String> = profile_list
        .iter()
        .map(|p| js_to_string_coerce(prop(p, "commandName")))
        .collect();
    for previous_name in read_provider_profile_manifest(root_dir) {
        if next_names.iter().any(|n| n == &previous_name) {
            continue;
        }
        let p = format!("{root_dir}/{previous_name}");
        if std::fs::remove_file(&p).is_err() && std::path::Path::new(&p).exists() {
            // Non-ENOENT failure.
            return Err(format!("Failed to create private path {p}"));
        }
    }
    for profile in &profile_list {
        let name = prop(profile, "commandName").clone();
        let wrapper_path = path_join(
            &Value::String(root_dir.to_string()),
            &js_to_string_coerce(&name),
        )?;
        let script = build_provider_profile_wrapper_script((*profile).clone())?;
        write_file_if_changed(&wrapper_path, &script, 0o700)?;
    }
    let mut sorted = next_names.clone();
    sorted.sort_by(|a, b| locale_compare(a, b));
    let manifest = format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Array(
            sorted.into_iter().map(Value::String).collect()
        ))
        .unwrap_or_else(|_| "[]".to_string())
    );
    write_file_if_changed(
        &format!("{root_dir}/provider-profiles.json"),
        &manifest,
        0o600,
    )
}

fn restore_zsh_dot_dir(variable: &str) -> String {
    format!(
        "if [[ -v {variable} ]]; then\n  ZDOTDIR=\"${{{variable}}}\"\nelse\n  unset ZDOTDIR\nfi\n"
    )
}

fn source_zsh_startup_file(file: &str) -> String {
    format!("[[ -f \"${{ZDOTDIR-$HOME}}/{file}\" ]] && source \"${{ZDOTDIR-$HOME}}/{file}\"\n")
}

fn managed_zsh_rc() -> String {
    format!(
        "# Synara zsh rc wrapper\n{}unset _synara_user_zdotdir\n{}if [ -n \"${{SYNARA_MANAGED_BIN_DIR:-}}\" ] && [ -d \"${{SYNARA_MANAGED_BIN_DIR}}\" ]; then
  case \":$PATH:\" in
    *:${{SYNARA_MANAGED_BIN_DIR}}:*) ;;
    *) export PATH=\"${{SYNARA_MANAGED_BIN_DIR}}:$PATH\" ;;
  esac
  unalias claude 2>/dev/null || true
  claude() {{
    if [ -x \"${{SYNARA_MANAGED_BIN_DIR}}/claude\" ] && [ ! -d \"${{SYNARA_MANAGED_BIN_DIR}}/claude\" ]; then
      \"${{SYNARA_MANAGED_BIN_DIR}}/claude\" \"$@\"
    else
      command claude \"$@\"
    fi
  }}
  unalias codex 2>/dev/null || true
  codex() {{
    if [ -x \"${{SYNARA_MANAGED_BIN_DIR}}/codex\" ] && [ ! -d \"${{SYNARA_MANAGED_BIN_DIR}}/codex\" ]; then
      \"${{SYNARA_MANAGED_BIN_DIR}}/codex\" \"$@\"
    else
      command codex \"$@\"
    fi
  }}
  typeset -ga precmd_functions 2>/dev/null || true
  _synara_ensure_managed_bin() {{
    case \":$PATH:\" in
      *:${{SYNARA_MANAGED_BIN_DIR}}:*) ;;
      *) PATH=\"${{SYNARA_MANAGED_BIN_DIR}}:$PATH\" ;;
    esac
  }}
  {{
    precmd_functions=(${{precmd_functions:#_synara_ensure_managed_bin}} _synara_ensure_managed_bin)
  }} 2>/dev/null || true
fi
",
        restore_zsh_dot_dir("_synara_user_zdotdir"),
        source_zsh_startup_file(".zshrc"),
    )
}

fn ensure_managed_zsh_wrappers(zsh_dir: &Value) -> Result<(), String> {
    ensure_private_directory_sync(zsh_dir)?;
    let zsh_dir_s = js_to_string_coerce(zsh_dir);
    let quoted = format!("'{}'", zsh_dir_s.replace('\'', "'\"'\"'"));
    let continue_to_managed_rc = format!(
        "if [[ -o interactive && -o rcs ]]; then\n  if [[ -v ZDOTDIR ]]; then\n    _synara_user_zdotdir=\"$ZDOTDIR\"\n  else\n    unset _synara_user_zdotdir\n  fi\n  ZDOTDIR={quoted}\nfi\n"
    );
    write_file_if_changed(
        &format!("{zsh_dir_s}/.zshenv"),
        &format!(
            "# Synara zsh env wrapper\n{}unset SYNARA_ORIGINAL_ZDOTDIR\n{}{}",
            restore_zsh_dot_dir("SYNARA_ORIGINAL_ZDOTDIR"),
            source_zsh_startup_file(".zshenv"),
            continue_to_managed_rc
        ),
        0o600,
    )?;
    write_file_if_changed(
        &format!("{zsh_dir_s}/.zprofile"),
        &format!(
            "# Synara zsh profile wrapper\n{}{}{}",
            restore_zsh_dot_dir("_synara_user_zdotdir"),
            source_zsh_startup_file(".zprofile"),
            continue_to_managed_rc
        ),
        0o600,
    )?;
    write_file_if_changed(&format!("{zsh_dir_s}/.zshrc"), &managed_zsh_rc(), 0o600)
}

fn exists_sync(p: &Value) -> bool {
    match p {
        Value::String(s) => std::path::Path::new(s).exists(),
        _ => false,
    }
}

fn null_wrapper_state(target_path_by_cli_kind: Value) -> Value {
    json!({
        "binDir": null,
        "codexHomeDir": null,
        "hookScriptPath": null,
        "claudeSettingsPath": null,
        "zshDir": null,
        "targetPathByCliKind": target_path_by_cli_kind,
    })
}

fn prepare_managed_terminal_wrappers_inner(
    base_env: &Value,
    profiles: &Value,
    root_dir: &Value,
    zsh_root_dir: &Value,
) -> Result<Value, String> {
    let mut target_path_by_cli_kind = Map::new();
    for cli_kind in ["codex", "claude"] {
        if let Some(target_path) = resolve_executable(cli_kind, base_env)? {
            target_path_by_cli_kind.insert(cli_kind.to_string(), Value::String(target_path));
        }
    }
    let profile_count = match profiles {
        Value::Array(a) => a.len() as f64,
        Value::String(s) => s.chars().count() as f64,
        _ => 0.0,
    };
    if target_path_by_cli_kind.is_empty() && profile_count == 0.0 {
        if exists_sync(root_dir)
            && let Value::String(rd) = root_dir
        {
            synchronize_provider_profile_wrappers(rd, &Value::Array(vec![]))?;
        }
        return Ok(null_wrapper_state(Value::Object(target_path_by_cli_kind)));
    }

    ensure_private_directory_sync(root_dir)?;
    let root = js_to_string_coerce(root_dir);
    let codex_home_dir = format!("{root}/codex-home");
    let hook_script_path = format!("{root}/notify-hook.sh");
    let claude_settings_path = format!("{root}/claude-settings.json");
    ensure_private_directory_sync(&Value::String(codex_home_dir.clone()))?;
    write_file_if_changed(&hook_script_path, &notify_hook_script(), 0o700)?;
    write_file_if_changed(
        &claude_settings_path,
        &claude_settings_json(&hook_script_path),
        0o600,
    )?;
    write_file_if_changed(
        &format!("{codex_home_dir}/hooks.json"),
        &codex_hooks_json(&hook_script_path),
        0o600,
    )?;
    for (cli_kind, target_path) in &target_path_by_cli_kind {
        let wrapper_path = format!("{root}/{cli_kind}");
        let script = managed_wrapper_script(
            &claude_settings_path,
            cli_kind,
            &codex_home_dir,
            &hook_script_path,
            target_path.as_str().unwrap_or_default(),
        );
        write_file_if_changed(&wrapper_path, &script, 0o700)?;
    }
    if let Value::String(rd) = root_dir {
        synchronize_provider_profile_wrappers(rd, profiles)?;
    }
    ensure_managed_zsh_wrappers(zsh_root_dir)?;

    Ok(json!({
        "binDir": root_dir,
        "codexHomeDir": codex_home_dir,
        "hookScriptPath": hook_script_path,
        "claudeSettingsPath": claude_settings_path,
        "zshDir": zsh_root_dir,
        "targetPathByCliKind": Value::Object(target_path_by_cli_kind),
    }))
}

/// Port of `prepareManagedTerminalWrappers` (`apps/server/src/terminal/managedTerminalWrappers.ts`).
pub fn prepare_managed_terminal_wrappers(
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&options) {
        return Err(undef_err("baseEnv", &options));
    }
    prepare_managed_terminal_wrappers_inner(
        prop(&options, "baseEnv"),
        prop(&options, "profiles"),
        prop(&options, "rootDir"),
        prop(&options, "zshRootDir"),
    )
}

/// Port of `applyManagedTerminalAgentWrapperEnv` (`apps/server/src/terminal/managedTerminalWrappers.ts`).
pub fn apply_managed_terminal_agent_wrapper_env(
    env: serde_json::Value,
    wrapper_state: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&wrapper_state) {
        return Err(undef_err("binDir", &wrapper_state));
    }
    let bin_dir = prop(&wrapper_state, "binDir");
    if !js_truthy(bin_dir) {
        return Ok(env);
    }
    let env_path_key = env_path_key_for(&env)?;
    let current_path = {
        let v = env_value(&env, env_path_key);
        if is_undef_or_null(v) {
            String::new()
        } else {
            match v {
                Value::String(s) => s.trim().to_string(),
                _ => return Err("env[envPathKey].trim is not a function".to_string()),
            }
        }
    };
    let mut entries: Vec<String> = current_path
        .split(':')
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .collect();
    let bin_dir_s = js_to_string_coerce(bin_dir);
    if !entries.iter().any(|e| e == &bin_dir_s) {
        entries.insert(0, bin_dir_s.clone());
    }
    // JS spread order: env keys, then the appended managed entries.
    let mut out = Map::new();
    js_spread_object(&mut out, &env);
    out.insert(
        "SYNARA_MANAGED_BIN_DIR".to_string(),
        Value::String(bin_dir_s.clone()),
    );
    // env.ZDOTDIR !== undefined: a stored $undefined marker counts as undefined.
    let zdotdir = env_value(&env, "ZDOTDIR");
    if !is_undefined_marker(zdotdir) {
        out.insert("SYNARA_ORIGINAL_ZDOTDIR".to_string(), zdotdir.clone());
    }
    let zsh_dir = prop(&wrapper_state, "zshDir");
    if js_truthy(zsh_dir) {
        out.insert("ZDOTDIR".to_string(), zsh_dir.clone());
    }
    out.insert(env_path_key.to_string(), Value::String(entries.join(":")));
    Ok(Value::Object(out))
}

/// Port of `prepareManagedTerminalAgentWrappers` (`apps/server/src/terminal/managedTerminalWrappers.ts`).
pub fn prepare_managed_terminal_agent_wrappers(
    options: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&options) {
        return Err(undef_err("baseEnv", &options));
    }
    prepare_managed_terminal_wrappers_inner(
        prop(&options, "baseEnv"),
        prop(&options, "profiles"),
        prop(&options, "targetDir"),
        prop(&options, "zshDir"),
    )
}

// ---------- providerInstances.ts (error-chain subset) ----------

const BUILT_IN_PROVIDER_KINDS: &[&str] = &[
    "codex",
    "claudeAgent",
    "cursor",
    "devin",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "omp",
];

fn derive_provider_instances(settings: &Value) -> Result<Vec<Value>, String> {
    // legacyProviderConfig: settings.providers[provider] -> legacy.enabled !== false
    if is_undefined_marker(settings) {
        return Err("Cannot read properties of undefined (reading 'providers')".to_string());
    }
    if settings.is_null() {
        return Err("Cannot read properties of null (reading 'providers')".to_string());
    }
    let providers = prop(settings, "providers");
    for provider in BUILT_IN_PROVIDER_KINDS {
        if is_undef_or_null(providers) {
            return Err(undef_err(provider, providers));
        }
        let legacy = prop(providers, provider);
        // legacy.enabled !== false
        if is_undef_or_null(legacy) {
            return Err(undef_err("enabled", legacy));
        }
    }
    // deriveLegacyCodexAccountInstances: for (const account of codex.accounts)
    let codex = prop(providers, "codex");
    if is_undef_or_null(codex) {
        return Err(undef_err("accounts", codex));
    }
    let accounts = prop(codex, "accounts");
    if !accounts.is_array() {
        return Err("codex.accounts is not iterable".to_string());
    }
    // Object.entries(settings.providerInstances)
    let _ = js_object_entries(prop(settings, "providerInstances"))?;
    // Deep resolution is unreachable in recorded behavior; resolved instances
    // are not needed downstream of the observed error boundary.
    Ok(vec![])
}

/// Port of `prepareProviderAuthenticationSettings` (`apps/server/src/terminal/providerAuthentication.ts`).
pub fn prepare_provider_authentication_settings(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&input) {
        return Err(undef_err("settings", &input));
    }
    let instances = derive_provider_instances(prop(&input, "settings"))?;
    let instance_id = prop(&input, "instanceId");
    let instance = instances
        .iter()
        .find(|i| prop(i, "instanceId") == instance_id);
    if !instance.is_some_and(|i| js_truthy(prop(i, "enabled"))) {
        return Err("This provider account is missing or disabled.".to_string());
    }
    Ok(Value::Null)
}

/// Port of `resolveProviderAuthenticationLaunch` (`apps/server/src/terminal/providerAuthentication.ts`).
pub async fn resolve_provider_authentication_launch(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&input) {
        return Err(undef_err("settings", &input));
    }
    let instances = derive_provider_instances(prop(&input, "settings"))?;
    let instance_id = prop(&input, "instanceId");
    let instance = instances
        .iter()
        .find(|i| prop(i, "instanceId") == instance_id);
    if !instance.is_some_and(|i| js_truthy(prop(i, "enabled"))) {
        return Err("This provider account is missing or disabled.".to_string());
    }
    Err(
        "The provider CLI was not found. Install it or check its configured binary path."
            .to_string(),
    )
}

/// Port of `deriveManagedTerminalProfiles` (`apps/server/src/terminal/providerTerminalProfiles.ts`).
pub async fn derive_managed_terminal_profiles(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&input) {
        return Err(undef_err("settings", &input));
    }
    let _ = derive_provider_instances(prop(&input, "settings"))?;
    Ok(Value::Array(vec![]))
}

// ---------- runtimeSelection.ts ----------

/// Port of `selectPtyAdapterRuntime` (`apps/server/src/terminal/runtimeSelection.ts`).
pub fn select_pty_adapter_runtime(input: serde_json::Value) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&input) {
        return Err(undef_err("platform", &input));
    }
    if prop(&input, "platform").as_str() == Some("win32") {
        return Ok(Value::String("node".to_string()));
    }
    Ok(prop(&input, "runtime").clone())
}

// ---------- subprocessActivity.ts ----------

fn empty_subprocess_activity() -> Value {
    json!({
        "cliKind": null,
        "hasNonProviderSubprocess": false,
        "hasProviderDescendant": false,
        "hasRunningSubprocess": false,
    })
}

fn process_executable_name(command: &str) -> String {
    let trimmed = command.trim_start();
    let first_token = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or("").to_string()
    } else {
        trimmed.split_whitespace().next().unwrap_or("").to_string()
    };
    let normalized = first_token.replace('\\', "/");
    // path.basename: strip trailing slashes then take the final segment.
    let base = normalized.trim_end_matches('/');
    let name = base.rsplit('/').next().unwrap_or(base);
    let lower = name.to_lowercase();
    for ext in [".cmd", ".com", ".exe"] {
        if lower.ends_with(ext) {
            return lower[..lower.len() - ext.len()].to_string();
        }
    }
    lower
}

fn is_shell_like_process_name(command: &str) -> bool {
    const SHELL_LIKE: &[&str] = &[
        "bash",
        "cmd",
        "dash",
        "fish",
        "ksh",
        "login",
        "nu",
        "powershell",
        "pwsh",
        "screen",
        "sh",
        "tcsh",
        "tmux",
        "zellij",
        "zsh",
    ];
    SHELL_LIKE.contains(&process_executable_name(command).as_str())
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

fn strip_script_extension(token: &str) -> String {
    let lower = token.to_lowercase();
    for ext in [
        ".cjs", ".cts", ".js", ".jsx", ".mjs", ".mts", ".py", ".ts", ".tsx",
    ] {
        if lower.ends_with(ext) {
            return lower[..lower.len() - ext.len()].to_string();
        }
    }
    lower
}

fn derive_cli_kind_from_normalized_token(token: &str) -> Option<&'static str> {
    let t = strip_script_extension(token.trim());
    if t.is_empty() {
        return None;
    }
    if matches!(t.as_str(), "codex" | "codex-cli" | "@openai/codex") {
        return Some("codex");
    }
    if matches!(
        t.as_str(),
        "claude" | "claude-code" | "claude_code" | "@anthropic-ai/claude-code"
    ) {
        return Some("claude");
    }
    if matches!(t.as_str(), "agy" | "antigravity" | "antigravity-cli") {
        return Some("antigravity");
    }
    None
}

fn tokenize_shell_command(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut escape_next = false;
    for ch in command.trim().chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        if ch == '\\' {
            escape_next = quote != Some('\'');
            if !escape_next {
                current.push(ch);
            }
            continue;
        }
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            continue;
        }
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(ch);
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn derive_cli_kind_from_token_list(tokens: &[String]) -> Option<&'static str> {
    for token in tokens {
        if let Some(kind) = derive_cli_kind_from_normalized_token(&normalize_command_token(token)) {
            return Some(kind);
        }
    }
    None
}

fn derive_cli_kind_from_process_text(text: &str) -> Option<&'static str> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let lower = t.to_lowercase();
    if lower.contains("@openai/codex") {
        return Some("codex");
    }
    if lower.contains("@anthropic-ai/claude-code") || lower.contains("anthropic/claude-code") {
        return Some("claude");
    }
    if lower.contains("google-antigravity/antigravity-cli") {
        return Some("antigravity");
    }
    None
}

fn derive_terminal_process_identity_kind(command: &str) -> Option<&'static str> {
    let stripped = command.trim();
    if stripped.is_empty() {
        return None;
    }
    derive_cli_kind_from_token_list(&tokenize_shell_command(stripped))
        .or_else(|| derive_cli_kind_from_process_text(stripped))
}

fn derive_process_cli_kind(command: &str) -> Option<&'static str> {
    derive_terminal_process_identity_kind(command)
        .or_else(|| derive_terminal_process_identity_kind(&process_executable_name(command)))
}

#[derive(Default)]
struct SubprocessActivity {
    cli_kind: Option<&'static str>,
    has_running: bool,
    has_provider: bool,
    has_non_provider: bool,
}

fn include_child_activity(
    activity: &mut SubprocessActivity,
    command: &str,
    nested: &SubprocessActivity,
) {
    let child_cli_kind = derive_process_cli_kind(command);
    let is_shell_like = is_shell_like_process_name(command);
    if activity.cli_kind.is_none() {
        activity.cli_kind = child_cli_kind.or(nested.cli_kind);
    }
    activity.has_provider =
        activity.has_provider || child_cli_kind.is_some() || nested.has_provider;
    activity.has_non_provider = activity.has_non_provider
        || (child_cli_kind.is_none() && !is_shell_like)
        || nested.has_non_provider;
    activity.has_running = activity.has_running || !is_shell_like || nested.has_running;
}

fn activity_to_value(a: &SubprocessActivity) -> Value {
    // includeChildActivity literal order (differs from emptySubprocessActivity).
    json!({
        "cliKind": a.cli_kind,
        "hasProviderDescendant": a.has_provider,
        "hasNonProviderSubprocess": a.has_non_provider,
        "hasRunningSubprocess": a.has_running,
    })
}

fn map_children(map: &Map<String, Value>, pid: f64) -> Vec<(f64, String)> {
    let entries = match map.get("$map") {
        Some(Value::Array(a)) => a,
        _ => return vec![],
    };
    for entry in entries {
        if let Value::Array(pair) = entry
            && pair.len() == 2
            && pair[0].as_f64() == Some(pid)
            && let Value::Array(children) = &pair[1]
        {
            return children
                .iter()
                .filter_map(|c| {
                    let pid = prop(c, "pid").as_f64()?;
                    let command = js_str(prop(c, "command")).unwrap_or_default();
                    Some((pid, command))
                })
                .collect();
        }
    }
    vec![]
}

fn inspect_activity_walk(map: &Map<String, Value>, pid: f64) -> SubprocessActivity {
    let mut activity = SubprocessActivity::default();
    for (child_pid, command) in map_children(map, pid) {
        let nested = inspect_activity_walk(map, child_pid);
        include_child_activity(&mut activity, &command, &nested);
    }
    activity
}

/// Port of `inspectSubprocessActivity` (`apps/server/src/terminal/subprocessActivity.ts`).
pub fn inspect_subprocess_activity(
    parent_pid: f64,
    children_by_parent_pid: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if is_undef_or_null(&children_by_parent_pid) {
        return Err(undef_err("get", &children_by_parent_pid));
    }
    let map = match &children_by_parent_pid {
        Value::Object(m) if m.contains_key("$map") => m,
        _ => return Err("childrenByParentPid.get is not a function".to_string()),
    };
    Ok(activity_to_value(&inspect_activity_walk(map, parent_pid)))
}

fn parse_process_children_map(ps_output: &str) -> Vec<(f64, Vec<Value>)> {
    let mut order: Vec<f64> = vec![];
    let mut grouped: Vec<(f64, Vec<Value>)> = vec![];
    for line in ps_output.split('\n') {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split_whitespace();
        let pid: f64 = match parts.next().and_then(|s| s.parse::<f64>().ok()) {
            Some(n) if n.fract() == 0.0 => n,
            _ => continue,
        };
        let ppid: f64 = match parts.next().and_then(|s| s.parse::<f64>().ok()) {
            Some(n) if n.fract() == 0.0 => n,
            _ => continue,
        };
        let command = parts.collect::<Vec<_>>().join(" ");
        if command.is_empty() {
            continue;
        }
        let child = json!({"pid": pid, "command": command});
        if let Some(existing) = grouped.iter_mut().find(|(p, _)| *p == ppid) {
            existing.1.push(child);
        } else {
            order.push(ppid);
            grouped.push((ppid, vec![child]));
        }
    }
    grouped
}

fn run_capture(args: &[&str]) -> Option<std::process::Output> {
    std::process::Command::new(args[0])
        .args(&args[1..])
        .output()
        .ok()
}

/// Port of `captureProcessChildrenMap` (`apps/server/src/terminal/subprocessActivity.ts`).
pub async fn capture_process_children_map() -> Result<Option<serde_json::Value>, String> {
    // ADR-004: the recorded corpus froze the volatile process-table snapshot; replay it
    // through SYNARA_CHILDREN_MAP_JSON. Unset → the real `ps` scan.
    if let Ok(frozen) = std::env::var("SYNARA_CHILDREN_MAP_JSON") {
        return Ok(serde_json::from_str(&frozen).ok());
    }
    let output = match run_capture(&["ps", "-eo", "pid=,ppid=,command="]) {
        Some(o) if o.status.success() => o,
        _ => return Ok(None),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    let grouped = parse_process_children_map(&stdout);
    let map_entries: Vec<Value> = grouped
        .into_iter()
        .map(|(ppid, children)| json!([ppid, Value::Array(children)]))
        .collect();
    Ok(Some(json!({"$map": map_entries})))
}

/// Port of `defaultSubprocessChecker` (`apps/server/src/terminal/subprocessActivity.ts`).
pub async fn default_subprocess_checker(terminal_pid: f64) -> Result<serde_json::Value, String> {
    if terminal_pid.fract() != 0.0 || terminal_pid.is_nan() || terminal_pid <= 0.0 {
        return Ok(empty_subprocess_activity());
    }
    let pid = terminal_pid as i64;
    if let Some(out) = run_capture(&["pgrep", "-P", &pid.to_string()]) {
        let code = out.status.code().unwrap_or(-1);
        if code == 1 {
            return Ok(empty_subprocess_activity());
        }
        if code == 0 && String::from_utf8_lossy(&out.stdout).trim().is_empty() {
            return Ok(empty_subprocess_activity());
        }
    }
    let children = capture_process_children_map().await?;
    let result = match children {
        Some(Value::Object(ref m)) => inspect_activity_walk(m, terminal_pid),
        _ => {
            // Tree-walk fallback (pgrep -P + ps -p per pid).
            let mut activity = SubprocessActivity::default();
            let mut stack = vec![terminal_pid];
            let mut visited = vec![terminal_pid];
            let mut depth = 0usize;
            while let Some(pp) = stack.pop() {
                depth += 1;
                if depth > 256 {
                    break;
                }
                if let Some(out) = run_capture(&["pgrep", "-P", &(pp as i64).to_string()]) {
                    if out.status.code() != Some(0) {
                        continue;
                    }
                    for token in String::from_utf8_lossy(&out.stdout).split_whitespace() {
                        let child_pid: f64 = match token.parse::<f64>() {
                            Ok(n) if n.fract() == 0.0 && n > 0.0 => n,
                            _ => continue,
                        };
                        if visited.contains(&child_pid) {
                            continue;
                        }
                        visited.push(child_pid);
                        let command = run_capture(&[
                            "ps",
                            "-p",
                            &(child_pid as i64).to_string(),
                            "-o",
                            "command=",
                        ])
                        .filter(|o| o.status.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .unwrap_or_default();
                        stack.push(child_pid);
                        let nested = SubprocessActivity::default();
                        include_child_activity(&mut activity, &command, &nested);
                    }
                }
            }
            activity
        }
    };
    Ok(activity_to_value(&result))
}

// ---------- terminalThreadTitleTracker.ts ----------
// The tracker consumes title input via the same shared helpers as `terminalthreads-rs`
// (inlined: a napi crate cannot link another napi crate without wire-symbol collisions).

const MAX_TERMINAL_INPUT_BUFFER_LENGTH: usize = 512;
const MAX_TERMINAL_TITLE_LENGTH: usize = 48;

const WRAPPER_COMMANDS: [&str; 6] = ["builtin", "command", "env", "noglob", "nocorrect", "sudo"];
const IGNORED_TERMINAL_TITLE_COMMANDS: [&str; 15] = [
    ".", "alias", "cd", "clear", "exit", "export", "history", "la", "ll", "logout", "ls", "pwd",
    "reset", "source", "unalias",
];
const PACKAGE_MANAGERS: [&str; 4] = ["bun", "npm", "pnpm", "yarn"];
const PACKAGE_MANAGER_SUBCOMMANDS: [&str; 4] = ["create", "dlx", "exec", "run"];

fn trim_js(s: &str) -> &str {
    s.trim_matches(jscore::is_space)
}

fn is_ascii_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
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

fn tokenize_title_command(command: &str) -> Vec<String> {
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
    let units = crate::utf16::units(title);
    if units.len() <= MAX_TERMINAL_TITLE_LENGTH {
        title.to_string()
    } else {
        crate::utf16::slice(&units, 0, MAX_TERMINAL_TITLE_LENGTH)
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

fn derive_title_command_identity(command: &str) -> Option<Value> {
    let stripped = trim_js(command);
    if stripped.is_empty() {
        return None;
    }
    let base_tokens = strip_shell_prefixes(&tokenize_title_command(stripped));
    if base_tokens.is_empty() {
        return None;
    }
    let tokens = unwrap_executor_command(base_tokens);
    let normalized: Vec<String> = tokens.iter().map(|t| normalize_command_token(t)).collect();
    let first = normalized.first().cloned().unwrap_or_default();
    let second = normalized.get(1).cloned().unwrap_or_default();
    if first.is_empty() || IGNORED_TERMINAL_TITLE_COMMANDS.contains(&first.as_str()) {
        return None;
    }
    let detected = derive_cli_kind_from_token_list(&tokens);
    if detected == Some("codex") {
        return Some(command_identity("Codex CLI", Some("codex")));
    }
    if detected == Some("claude") || (first == "claude" && second == "code") {
        return Some(command_identity("Claude Code", Some("claude")));
    }
    if detected == Some("antigravity") {
        return Some(command_identity("Antigravity CLI", Some("antigravity")));
    }
    if first == "git" {
        let title = if second.is_empty() {
            "git".to_string()
        } else {
            format!("git {second}")
        };
        return Some(command_identity(&truncate_terminal_title(&title), None));
    }
    if let Some(title) = package_manager_title(&tokens) {
        return Some(command_identity(&truncate_terminal_title(&title), None));
    }
    let generic_title: String = normalized[..normalized.len().min(2)]
        .join(" ")
        .trim_matches(jscore::is_space)
        .to_string();
    if generic_title.is_empty() {
        return None;
    }
    Some(command_identity(
        &truncate_terminal_title(&generic_title),
        None,
    ))
}

/// Port of `consumeTerminalTitleInput` (`packages/shared/src/terminalThreads.ts:432`): returns
/// `{buffer, title}` like the JS result object (buffer uses JS UTF-16 truncation semantics).
fn consume_terminal_title_input(buffer: String, data: &str) -> (String, Option<String>) {
    if data.contains('\u{1b}') {
        return (buffer, None);
    }
    let mut next_buffer: Vec<u16> = crate::utf16::units(&buffer);
    let mut next_title: Option<String> = None;
    for c in data.chars() {
        if c == '\r' || c == '\n' {
            next_title = derive_title_command_identity(&String::from_utf16_lossy(&next_buffer))
                .and_then(|i| i.get("title").and_then(Value::as_str).map(str::to_string));
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
    (
        crate::utf16::slice(&next_buffer, start, next_buffer.len()),
        next_title,
    )
}

/// Port of class `TerminalThreadTitleTracker` (`apps/server/src/terminal/terminalThreadTitleTracker.ts:15`): holds the instance state.
pub struct TerminalThreadTitleTracker {
    buffer_by_session: std::collections::HashMap<String, String>,
}

fn title_session_key(thread_id: &Value, terminal_id: &Value) -> String {
    format!(
        "{}:{}",
        js_to_string_coerce(thread_id),
        js_to_string_coerce(terminal_id)
    )
}

fn is_generic_thread_title(title: &Value) -> Result<bool, String> {
    match title {
        Value::String(s) => Ok(s.trim_matches(jscore::is_space) == "New terminal"),
        _ if is_undef_or_null(title) => Ok(false),
        _ => Err("title.trim is not a function".to_string()),
    }
}

impl TerminalThreadTitleTracker {
    /// Port of `TerminalThreadTitleTracker` constructor (`apps/server/src/terminal/terminalThreadTitleTracker.ts:15`).
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            buffer_by_session: std::collections::HashMap::new(),
        })
    }

    /// Port of `TerminalThreadTitleTracker.reset` (`apps/server/src/terminal/terminalThreadTitleTracker.ts:18`).
    pub fn reset(&mut self, thread_id: String, terminal_id: Option<String>) -> Result<(), String> {
        match terminal_id {
            Some(tid) if !tid.is_empty() => {
                self.buffer_by_session.remove(&format!("{thread_id}:{tid}"));
            }
            _ => {
                let prefix = format!("{thread_id}:");
                self.buffer_by_session
                    .retain(|k, _| !k.starts_with(prefix.as_str()));
            }
        }
        Ok(())
    }

    /// Port of `TerminalThreadTitleTracker.consumeWrite` (`apps/server/src/terminal/terminalThreadTitleTracker.ts:31`).
    pub fn consume_write(&mut self, input: serde_json::Value) -> Result<serde_json::Value, String> {
        if input.is_null() {
            return Err("Cannot read properties of null (reading 'threadId')".to_string());
        }
        if is_undefined_marker(&input) {
            return Err("Cannot read properties of undefined (reading 'threadId')".to_string());
        }
        let key = title_session_key(prop(&input, "threadId"), prop(&input, "terminalId"));
        let data = match prop(&input, "data") {
            Value::String(s) => s.clone(),
            other if is_undef_or_null(other) => {
                return Err(format!(
                    "Cannot read properties of {} (reading 'includes')",
                    if is_undefined_marker(other) {
                        "undefined"
                    } else {
                        "null"
                    }
                ));
            }
            _ => return Err("data.includes is not a function".to_string()),
        };
        let buffer = self
            .buffer_by_session
            .get(&key)
            .cloned()
            .unwrap_or_default();
        let (next_buffer, title) = consume_terminal_title_input(buffer, &data);
        if next_buffer.is_empty() {
            self.buffer_by_session.remove(&key);
        } else {
            self.buffer_by_session.insert(key, next_buffer);
        }
        match title {
            None => Ok(Value::Null),
            Some(t) => {
                if is_generic_thread_title(prop(&input, "currentTitle"))? {
                    Ok(Value::String(t))
                } else {
                    Ok(Value::Null)
                }
            }
        }
    }
}
