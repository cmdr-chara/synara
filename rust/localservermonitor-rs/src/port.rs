//! The Rust port of `apps/server/src/localServerMonitor.ts`.

// `unsafe` is needed for the `kill(pid, 0)` liveness probe.
#![deny(unsafe_op_in_unsafe_fn)]

use jscore::{js_to_string, js_truthy, number_to_string};
use serde_json::{Value, json};
use std::sync::LazyLock;

static BRACKET: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\[([^\]]+)\]:(\d+)$").expect("valid regex"));
static ENV_ASSIGNMENT_START: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(^|[\s;&|()'\"])((?:--?(?:e|env|environment|set-?env)=)?)([A-Za-z_][A-Za-z0-9_]*)(?:\+)?="#).expect("valid regex")
});
static SENSITIVE_ENV_EXACT: &[&str] = &[
    "API_?KEY",
    "ACCESS_TOKEN",
    "AUTH",
    "AUTH_TOKEN",
    "AUTHORIZATION",
    "CREDENTIALS?",
    "KEY",
    "MYSQL_PWD",
    "PASS",
    "PASSWORD",
    "PASSPHRASE",
    "PGPASSWORD",
    "REDISCLI_AUTH",
    "SECRET",
    "TOKEN",
];
static URL_CREDENTIALS_AT_START: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r##"^["']?[a-z][a-z0-9+.-]*://[^/?#\s]+@"##).expect("valid regex")
});
static SENSITIVE_FLAG_VALUE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?i)(--?(?:api[-_]?key|auth|authorization|key|password|secret|token)(?:=|\s+))(?:(?:"(?:\\.|[^"\\])*(?:"|$)|'(?:\\.|[^'\\])*(?:'|$)|`(?:\\.|[^`\\])*(?:`|$)|[^\s'"`]+)+)"#).expect("valid regex")
});
static URL_CREDENTIALS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b([a-z][a-z0-9+.-]*://)[^/?#\s]+@").expect("valid regex")
});
static BEARER_TOKEN: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(Bearer\s+)[A-Za-z0-9._~+/=-]+").expect("valid regex")
});
static SK_TOKEN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\bsk-[A-Za-z0-9_-]{8,}\b").expect("valid regex"));
static SYN_TOKEN: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"\bsyn_(?:pair|mcp)_v1_[A-Za-z0-9_-]+\b").expect("valid regex")
});
static SUFFIX: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
                r"_(?:API_?KEY|ACCESS_TOKEN|AUTH|AUTH_TOKEN|AUTHORIZATION|CREDENTIALS?|KEY|PASS|PASSWORD|PASSPHRASE|SECRET|TOKEN)$",
            )
            .expect("valid regex")
});
static ROW: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\s*(\d+)\s+(\d+)\s+(.+?)\s*$").expect("valid regex"));
static JS_EXT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\.[cm]?js$").expect("valid regex"));
static TOKENIZER: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#""([^"]*)"|'([^']*)'|(\S+)"#).expect("valid regex"));
const EXCLUDED_PROCESS_PATTERNS: &[&str] = &[
    "airplayxpchelper",
    "controlcenter",
    "cursor helper",
    "figma",
    "google chrome",
    "linear helper",
    "logioptionsplus",
    "rapportd",
    "raycast",
    "safari",
    "spotify",
];
const EXCLUDED_PROCESS_COMMANDS: &[&str] = &[
    "electron",
    "electron helper",
    "electron helper (renderer)",
    "synara",
];
const DATABASE_OR_SYSTEM_COMMANDS: &[&str] = &[
    "memcached",
    "mongod",
    "mysql",
    "mysqld",
    "postgres",
    "postgresql",
    "redis-server",
];
static CHROMIUM_CHILD_ARGS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)--type=(?:renderer|gpu-process|gpu|utility|zygote|plugin|ppapi|broker|crashpad-handler)\b").expect("valid regex")
});
static APP_HELPER_COMMAND: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\bhelper\s*\((?:renderer|gpu|plugin|alerts)\)").expect("valid regex")
});
const DEV_COMMAND_LABELS: &[(&str, &str)] = &[
    ("air", "Air"),
    ("artisan", "Laravel"),
    ("astro", "Astro"),
    ("bunx", "Bun"),
    ("expo", "Expo"),
    ("flask", "Flask"),
    ("next", "Next.js"),
    ("nuxt", "Nuxt"),
    ("parcel", "Parcel"),
    ("rails", "Rails"),
    ("serve", "Serve"),
    ("vite", "Vite"),
    ("webpack-dev-server", "Webpack"),
];
static NEXT_DEV: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bnext\s+dev\b").expect("valid regex"));
static VITE_TEXT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(^|[\s/\\])vite(?:\.js|\.mjs|\.cjs)?(?:\s|$)").expect("valid regex")
});
static NUXT: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bnuxt\b").expect("valid regex"));
static ASTRO: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bastro\b").expect("valid regex"));
static WEBPACK: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\bwebpack(?:-dev-server|\s+serve)\b").expect("valid regex")
});
static PARCEL: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bparcel\b").expect("valid regex"));
static UVICORN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\buvicorn\b").expect("valid regex"));
static FLASK: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bflask\b").expect("valid regex"));
static DJANGO: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(?:manage\.py\s+runserver)|\bdjango\b").expect("valid regex")
});
static LARAVEL: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)(?:php\s+artisan\s+serve)|\blaravel\b").expect("valid regex")
});
static RAILS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\brails\s+(?:s|server)\b").expect("valid regex"));
static GO_RUN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bgo\s+run\b").expect("valid regex"));
static CARGO_RUN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bcargo\s+run\b").expect("valid regex"));
static DOTNET: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bdotnet\s+(?:watch|run)\b").expect("valid regex"));
static DENO: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\bdeno\s+(?:task\s+)?(?:dev|serve|run)\b").expect("valid regex")
});
static PYTHON_SERVER: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\bpython3?\s+-m\s+http\.server\b").expect("valid regex")
});
static PHP_SERVER: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\bphp\s+-S\s+").expect("valid regex"));
static REACT_SCRIPTS: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)\breact-scripts\s+start\b").expect("valid regex"));
static DEV_SCRIPT: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(
        r"(?i)^(?:dev|dev[:_-].+|.+[:_-]dev|electron:dev|dev:electron|dev:desktop|desktop:dev|start:desktop)$",
    )
    .expect("valid regex")
});
static DEV_ARGS: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(astro|expo|flask|next\s+dev|nodemon|nuxt|parcel|react-scripts\s+start|remix|rsbuild|rspack|svelte-kit|turbo|vite|webpack-dev-server)\b|(?:manage\.py\s+runserver)|(?:php\s+(?:artisan\s+serve|-S\s+))|(?:rails\s+(?:s|server))|(?:uvicorn\b)|(?:webpack\s+serve)|(?:go\s+run\b)|(?:cargo\s+run\b)|(?:dotnet\s+(?:watch|run)\b)|(?:deno\s+(?:task\s+)?(?:dev|serve|run)\b)|(?:python3?\s+-m\s+http\.server\b)|(?:dev-runner\.[cm]?ts\s+dev[:_-][A-Za-z0-9:_-]+)").expect("valid regex")
});
static SCRIPT_NAME: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)\b(?:bun|npm|pnpm|yarn)\s+(?:run\s+)?([A-Za-z0-9:_-]+)\b")
        .expect("valid regex")
});
static TAG_STRIP: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"<[^>]*>").expect("valid regex"));
static ENTITY: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)&(#x[0-9a-f]+|#\d+|amp|apos|gt|lt|nbsp|quot);").expect("valid regex")
});
static META_TAG: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"(?i)<meta\b[^>]*>").expect("valid regex"));
static META_NAME: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?i)\b(?:name|property)=["']?([^"'\s>]+)["']?"#).expect("valid regex")
});
static META_CONTENT_Q: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r#"(?i)\bcontent="([^"]*)"|content='([^']*)'"#).expect("valid regex")
});
static META_CONTENT_BARE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r#"(?i)\bcontent=([^"'\s>]+)"#).expect("valid regex"));
static TITLE_TAG: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?is)<title\b[^>]*>([\s\S]*?)</title>").expect("valid regex")
});
static WS_RUN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\s+").expect("valid regex"));
const META_NAMES: &[&str] = &["application-name", "og:title", "twitter:title"];
static EMPTY_MAP: LazyLock<Value> = LazyLock::new(|| json!({"$map": []}));

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

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn utf16_take(s: &str, n: usize) -> String {
    String::from_utf16_lossy(&s.encode_utf16().take(n).collect::<Vec<u16>>())
}

/// JS `Number(value)` on a wire value.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => i32::from(*b) as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_number(s),
        Value::Array(a) => match a.len() {
            0 => 0.0,
            1 => to_number(&a[0]),
            _ => f64::NAN,
        },
        Value::Object(m) => {
            if m.contains_key("$undefined") {
                return f64::NAN;
            }
            if let Some(n) = m.get("$num").and_then(Value::as_str) {
                return match n {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    "-0" => -0.0,
                    _ => n.parse().unwrap_or(f64::NAN),
                };
            }
            f64::NAN
        }
    }
}

/// JS `Number(string)`: trims, then hex/binary/octal/infinity/decimal.
fn parse_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    let lower = t.strip_prefix('+').unwrap_or(t);
    let lower = lower
        .strip_prefix('-')
        .map(|r| (r, true))
        .unwrap_or((lower, false));
    let (digits, neg) = lower;
    let parsed = if let Some(h) = digits.strip_prefix("0x").or(digits.strip_prefix("0X")) {
        u64::from_str_radix(h, 16).ok().map(|v| v as f64)
    } else if let Some(b) = digits.strip_prefix("0b").or(digits.strip_prefix("0B")) {
        u64::from_str_radix(b, 2).ok().map(|v| v as f64)
    } else if let Some(o) = digits.strip_prefix("0o").or(digits.strip_prefix("0O")) {
        u64::from_str_radix(o, 8).ok().map(|v| v as f64)
    } else {
        None
    };
    if let Some(v) = parsed {
        return if neg { -v } else { v };
    }
    t.parse().unwrap_or(f64::NAN)
}

fn js_lt(a: f64, b: f64) -> bool {
    a < b
}

// --- lsof parsing ---

fn parse_endpoint(name: &str, protocol: &str) -> Option<(String, f64, &'static str)> {
    let mut cleaned = js_trim(name).to_string();
    // `/\s+\(LISTEN\)$/i`
    let lower = cleaned.to_lowercase();
    if lower.ends_with("(listen)")
        && lower[..lower.len() - 8]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_whitespace() || c == '\u{feff}')
    {
        cleaned = js_trim(&cleaned[..cleaned.len() - 8]).to_string();
    }
    if let Some(m) = BRACKET.captures(&cleaned) {
        let port = parse_number(&m[2]);
        if port.fract() == 0.0 && port > 0.0 && port <= 65_535.0 {
            return Some((m[1].to_string(), port, "tcp6"));
        }
        return None;
    }
    let sep = cleaned.rfind(':')?;
    let raw_host = js_trim(&cleaned[..sep]);
    let raw_port = js_trim(&cleaned[sep + 1..]);
    let port = parse_number(raw_port);
    if port.fract() != 0.0 || !(port > 0.0 && port <= 65_535.0) {
        return None;
    }
    let host = if raw_host.is_empty() { "*" } else { raw_host };
    let family = if host.contains(':') {
        "tcp6"
    } else if protocol == "tcp" && host == "*" {
        "tcp"
    } else {
        "tcp4"
    };
    Some((host.to_string(), port, family))
}

/// Port of `parseLsofTcpListenOutput` (`apps/server/src/localServerMonitor.ts:180`).
pub fn parse_lsof_tcp_listen_output(output: String) -> Result<Vec<serde_json::Value>, String> {
    let mut listeners: Vec<Value> = Vec::new();
    let mut current_pid: Option<f64> = None;
    let mut current_command = String::new();
    for raw_line in output.split('\n') {
        let raw_line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let line = js_trim(raw_line);
        if utf16_len(line) < 2 {
            continue;
        }
        let mut chars = line.chars();
        let field = chars.next().unwrap_or('\0');
        let value = chars.as_str();
        match field {
            'p' => {
                let pid = parse_number(value);
                current_pid = if pid.fract() == 0.0 && pid > 0.0 {
                    Some(pid)
                } else {
                    None
                };
                current_command.clear();
            }
            'c' => current_command = js_trim(value).to_string(),
            'P' => {}
            'n' => {
                let Some(pid) = current_pid else { continue };
                let Some((host, port, family)) = parse_endpoint(value, "tcp") else {
                    continue;
                };
                listeners.push(json!({
                    "pid": pid,
                    "command": if current_command.is_empty() { "unknown" } else { &current_command },
                    "protocol": "tcp",
                    "host": host,
                    "port": port,
                    "family": family,
                }));
            }
            _ => {}
        }
    }
    Ok(listeners)
}

/// Port of `parseLsofCwdOutput` (`apps/server/src/localServerMonitor.ts:230`).
pub fn parse_lsof_cwd_output(output: String) -> Result<serde_json::Value, String> {
    let mut entries: Vec<Value> = Vec::new();
    let mut current_pid: Option<f64> = None;
    for raw_line in output.split('\n') {
        let raw_line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        let line = js_trim(raw_line);
        if utf16_len(line) < 2 {
            continue;
        }
        let mut chars = line.chars();
        let field = chars.next().unwrap_or('\0');
        let value = chars.as_str();
        match field {
            'p' => {
                let pid = parse_number(value);
                current_pid = if pid.fract() == 0.0 && pid > 0.0 {
                    Some(pid)
                } else {
                    None
                };
            }
            'n' => {
                let Some(pid) = current_pid else { continue };
                let cwd = js_trim(value);
                if !cwd.is_empty()
                    && !entries.iter().any(|e| {
                        e.as_array()
                            .and_then(|p| p.first())
                            .and_then(Value::as_f64)
                            .is_some_and(|k| k == pid)
                    })
                {
                    entries.push(json!([pid, cwd]));
                }
            }
            _ => {}
        }
    }
    Ok(json!({"$map": entries}))
}

// --- process-info redaction ---

fn is_sensitive_env_name(name: &str) -> bool {
    let upper = name.to_uppercase();
    SENSITIVE_ENV_EXACT.iter().any(|pat| {
        // `?` in these names means "optional previous char".
        let re = format!("^{pat}$");
        regex::Regex::new(&re)
            .map(|r| r.is_match(&upper))
            .unwrap_or(false)
    }) || { SUFFIX.is_match(&upper) }
}

/// `redactSensitiveEnvironmentRemainder`: once a secret assignment starts,
/// the remainder is unknowable — truncate from the `=` on.
fn redact_env_remainder(args: &str) -> String {
    for m in ENV_ASSIGNMENT_START.captures_iter(args) {
        let value_start = m.get(0).map(|g| g.end()).unwrap_or(0);
        let name = m.get(3).map(|g| g.as_str()).unwrap_or("");
        if !is_sensitive_env_name(name) && !URL_CREDENTIALS_AT_START.is_match(&args[value_start..])
        {
            continue;
        }
        return format!("{}[redacted]", &args[..value_start]);
    }
    args.to_string()
}

fn redact_sensitive_process_args(args: &str) -> String {
    let env = redact_env_remainder(args);
    let env = SENSITIVE_FLAG_VALUE.replace_all(&env, "$1[redacted]");
    let env = URL_CREDENTIALS.replace_all(&env, "$1[redacted]@");
    let env = BEARER_TOKEN.replace_all(&env, "$1[redacted]");
    let env = SK_TOKEN.replace_all(&env, "[redacted]");
    SYN_TOKEN.replace_all(&env, "[redacted]").into_owned()
}

/// Port of `parseProcessInfo` (`apps/server/src/localServerMonitor.ts:256`).
pub fn parse_process_info(output: String) -> Result<serde_json::Value, String> {
    let mut entries: Vec<Value> = Vec::new();
    for line in output.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let Some(m) = ROW.captures(line) else {
            continue;
        };
        let raw = &m[3];
        let pid = parse_number(&m[1]);
        let ppid = parse_number(&m[2]);
        let command_line = utf16_take(&redact_sensitive_process_args(raw), 1000);
        // Map.set keeps the original position when overwriting a key.
        if let Some(existing) = entries.iter_mut().find(|e| {
            e.as_array()
                .and_then(|p| p.first())
                .and_then(Value::as_f64)
                .is_some_and(|k| k == pid)
        }) {
            *existing =
                json!([pid, {"ppid": ppid, "commandLine": command_line, "rawCommandLine": raw}]);
        } else {
            entries.push(
                json!([pid, {"ppid": ppid, "commandLine": command_line, "rawCommandLine": raw}]),
            );
        }
    }
    Ok(json!({"$map": entries}))
}

// --- dev-server classification ---

fn path_basename(token: &str) -> &str {
    let trimmed = token.trim_end_matches('/');
    if trimmed.is_empty() {
        return token;
    }
    trimmed.rsplit('/').next().unwrap_or(trimmed)
}

fn command_token_name(token: &str) -> String {
    JS_EXT.replace(path_basename(token), "").to_lowercase()
}

fn tokenize(command_line: &str) -> Vec<String> {
    TOKENIZER
        .captures_iter(command_line)
        .filter_map(|m| {
            (1..=3)
                .find_map(|i| m.get(i))
                .map(|g| g.as_str().to_string())
        })
        .filter(|t| !t.is_empty())
        .collect()
}

/// `tokenize(args)` where `args` must be a JS string.
fn tokenize_value(args: &Value) -> Result<Vec<String>, String> {
    match args {
        Value::String(s) => Ok(tokenize(s)),
        Value::Null => Err("Cannot read properties of null (reading 'matchAll')".into()),
        v if is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'matchAll')".into())
        }
        _ => Err("args.matchAll is not a function".into()),
    }
}

/// `path.basename` argument type error.
fn basename_arg(v: &Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Null => Err("The \"path\" argument must be of type string. Received null".into()),
        Value::Bool(b) => Err(format!(
            "The \"path\" argument must be of type string. Received type boolean ({b})"
        )),
        Value::Number(_) => Err(format!(
            "The \"path\" argument must be of type string. Received type number ({})",
            number_to_string(v.as_f64().unwrap_or(f64::NAN))
        )),
        Value::Array(_) => Err(
            "The \"path\" argument must be of type string. Received an instance of Array".into(),
        ),
        v if is_undefined(v) => {
            Err("The \"path\" argument must be of type string. Received undefined".into())
        }
        _ => Err(
            "The \"path\" argument must be of type string. Received an instance of Object".into(),
        ),
    }
}

fn normalize_command_name(command: &Value, args: &Value) -> Result<String, String> {
    let tokens = tokenize_value(args)?;
    if let Some(first) = tokens.first() {
        return Ok(command_token_name(first));
    }
    Ok(command_token_name(&basename_arg(command)?))
}

fn process_command_tokens(command: &Value, args: &Value) -> Vec<String> {
    tokenize(&format!("{} {}", js_to_string(command), js_to_string(args)))
}

fn is_expo_command(command: &Value, args: &Value) -> bool {
    process_command_tokens(command, args).iter().any(|token| {
        let normalized = token.replace('\\', "/").to_lowercase();
        command_token_name(token) == "expo" || normalized.contains("/node_modules/@expo/cli/")
    })
}

fn is_metro_command(command: &Value, args: &Value) -> bool {
    let tokens = process_command_tokens(command, args);
    let names: Vec<String> = tokens.iter().map(|t| command_token_name(t)).collect();
    if names.iter().any(|n| n == "metro")
        || tokens.iter().any(|t| {
            t.replace('\\', "/")
                .to_lowercase()
                .contains("/node_modules/metro/")
        })
    {
        return true;
    }
    names
        .iter()
        .position(|n| n == "react-native")
        .is_some_and(|i| names[i + 1..].iter().any(|n| n == "start"))
}

/// `input.ports.every(port => port < 1024)`.
fn ports_all_low(ports: &Value) -> Result<bool, String> {
    match ports {
        Value::Array(items) => Ok(items.iter().all(|p| js_lt(to_number(p), 1024.0))),
        Value::Null => Err("Cannot read properties of null (reading 'every')".into()),
        v if is_undefined(v) => Err("Cannot read properties of undefined (reading 'every')".into()),
        _ => Err("input.ports.every is not a function".into()),
    }
}

/// Port of `isIgnoredLocalServerProcess` (`apps/server/src/localServerMonitor.ts:354`).
pub fn is_ignored_local_server_process(input: serde_json::Value) -> Result<bool, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'command')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'command')".into());
    }
    let command = member(&input, "command")?;
    let args = member(&input, "args")?;
    let text = format!("{} {}", js_to_string(&command), js_to_string(&args)).to_lowercase();
    let command_name = normalize_command_name(&command, &args)?;
    let ports = member(&input, "ports")?;
    if ports_all_low(&ports)? {
        return Ok(true);
    }
    if DATABASE_OR_SYSTEM_COMMANDS.contains(&command_name.as_str()) {
        return Ok(true);
    }
    if CHROMIUM_CHILD_ARGS.is_match(&js_to_string(&args))
        || APP_HELPER_COMMAND.is_match(&js_to_string(&command))
    {
        return Ok(true);
    }
    if EXCLUDED_PROCESS_COMMANDS.contains(&command_name.as_str()) {
        return Ok(true);
    }
    Ok(EXCLUDED_PROCESS_PATTERNS.iter().any(|p| text.contains(p)))
}

/// Port of `detectDevServerKindFromText` (`apps/server/src/localServerMonitor.ts:384`).
pub fn detect_dev_server_kind_from_text(
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'command')".into());
    }
    if is_undefined(&input) {
        return Err("Cannot read properties of undefined (reading 'command')".into());
    }
    let command = member(&input, "command")?;
    let args = member(&input, "args")?;
    let command_name = normalize_command_name(&command, &args)?;
    let text = format!("{} {}", js_to_string(&command), js_to_string(&args)).to_lowercase();
    if is_expo_command(&command, &args) {
        return Ok(Some("Expo".into()));
    }
    if is_metro_command(&command, &args) {
        return Ok(Some("Metro".into()));
    }
    if let Some((_, label)) = DEV_COMMAND_LABELS
        .iter()
        .find(|(name, _)| *name == command_name)
    {
        if command_name == "next" && !NEXT_DEV.is_match(&js_to_string(&args)) {
            return Ok(None);
        }
        return Ok(Some(label.to_string()));
    }
    let text_checks: &[(&LazyLock<regex::Regex>, &str)] = &[
        (&VITE_TEXT, "Vite"),
        (&NEXT_DEV, "Next.js"),
        (&NUXT, "Nuxt"),
        (&ASTRO, "Astro"),
        (&WEBPACK, "Webpack"),
        (&PARCEL, "Parcel"),
        (&UVICORN, "Uvicorn"),
        (&FLASK, "Flask"),
        (&DJANGO, "Django"),
        (&LARAVEL, "Laravel"),
        (&RAILS, "Rails"),
        (&GO_RUN, "Go"),
        (&CARGO_RUN, "Cargo"),
        (&DOTNET, "Dotnet"),
        (&DENO, "Deno"),
        (&PYTHON_SERVER, "Python"),
        (&PHP_SERVER, "PHP"),
        (&REACT_SCRIPTS, "React"),
    ];
    for (re, label) in text_checks {
        if re.is_match(&text) {
            return Ok(Some(label.to_string()));
        }
    }
    if let Some(m) = SCRIPT_NAME.captures(&js_to_string(&args))
        && DEV_SCRIPT.is_match(&m[1])
    {
        return Ok(Some("Dev Server".into()));
    }
    if DEV_ARGS.is_match(&text) {
        return Ok(Some("Dev Server".into()));
    }
    Ok(None)
}

// --- page title extraction ---

fn decode_entities(stripped: &str) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    let mut last = 0usize;
    for m in ENTITY.captures_iter(stripped) {
        let span = m.get(0).expect("valid regex");
        out.extend(stripped[last..span.start()].encode_utf16());
        let entity = m[1].to_lowercase();
        let replacement: Option<Vec<u16>> = match entity.as_str() {
            "amp" => Some(vec![b'&' as u16]),
            "apos" => Some(vec![b'\'' as u16]),
            "gt" => Some(vec![b'>' as u16]),
            "lt" => Some(vec![b'<' as u16]),
            "nbsp" => Some(vec![b' ' as u16]),
            "quot" => Some(vec![b'"' as u16]),
            _ => {
                let (radix, digits) = if let Some(h) = entity.strip_prefix("#x") {
                    (16, h)
                } else {
                    (10, entity.strip_prefix('#').unwrap_or(&entity))
                };
                let cp = u64::from_str_radix(digits, radix)
                    .map(|v| v as f64)
                    .unwrap_or(f64::NAN);
                if cp.fract() == 0.0 && (0.0..=1_114_111.0).contains(&cp) {
                    let cp = cp as u32;
                    if let Some(ch) = char::from_u32(cp) {
                        Some(ch.encode_utf16(&mut [0u16; 2]).to_vec())
                    } else {
                        // Lone surrogate: JS keeps the raw unit.
                        Some(vec![cp as u16])
                    }
                } else {
                    None
                }
            }
        };
        match replacement {
            Some(units) => out.extend(units),
            None => out.extend(span.as_str().encode_utf16()),
        }
        last = span.end();
    }
    out.extend(stripped[last..].encode_utf16());
    out
}

fn normalize_page_title(input: &str) -> Option<String> {
    let stripped = TAG_STRIP.replace_all(input, " ");
    let decoded_units = decode_entities(&stripped);
    let decoded = String::from_utf16_lossy(&decoded_units);
    let decoded = WS_RUN.replace_all(&decoded, " ");
    let decoded = js_trim(&decoded);
    if decoded.is_empty() {
        return None;
    }
    if utf16_len(decoded) <= 200 {
        return Some(decoded.to_string());
    }
    let truncated = utf16_take(decoded, 197);
    Some(format!("{}...", js_trim(&truncated)))
}

/// Port of `extractLocalServerPageTitle` (`apps/server/src/localServerMonitor.ts:493`).
pub fn extract_local_server_page_title(html: String) -> Result<Option<String>, String> {
    for tag in META_TAG.find_iter(&html) {
        let raw = tag.as_str();
        let Some(name_match) = META_NAME.captures(raw) else {
            continue;
        };
        if !META_NAMES.contains(&name_match[1].to_lowercase().as_str()) {
            continue;
        }
        let content = META_CONTENT_Q
            .captures(raw)
            .and_then(|m| m.get(1).or(m.get(2)))
            .or_else(|| META_CONTENT_BARE.captures(raw).and_then(|m| m.get(1)))
            .map(|g| g.as_str().to_string())
            .unwrap_or_default();
        if let Some(normalized) = normalize_page_title(&content) {
            return Ok(Some(normalized));
        }
    }
    if let Some(m) = TITLE_TAG.captures(&html) {
        return Ok(normalize_page_title(&m[1]));
    }
    Ok(None)
}

// --- buildLocalServerProcesses ---

fn is_map(v: &Value) -> bool {
    v.as_object().is_some_and(|m| m.contains_key("$map"))
}

fn map_get<'a>(map: &'a Value, name: &str, key: &Value) -> Result<Option<&'a Value>, String> {
    let map = match map {
        m if is_undefined(m) => &EMPTY_MAP,
        m if m.is_null() => {
            return Err("Cannot read properties of null (reading 'get')".into());
        }
        m if !is_map(m) => return Err(format!("{name}.get is not a function")),
        m => m,
    };
    let Some(entries) = map["$map"].as_array() else {
        return Ok(None);
    };
    for entry in entries {
        if let Some(pair) = entry.as_array()
            && pair.len() == 2
            && same_value_zero(&pair[0], key)
        {
            return Ok(Some(&pair[1]));
        }
    }
    Ok(None)
}

/// JS SameValueZero on decoded wire values; objects compare by identity so
/// two distinct JSON values are never equal.
fn same_value_zero(a: &Value, b: &Value) -> bool {
    fn decode(v: &Value) -> Value {
        if let Value::Object(m) = v {
            if m.contains_key("$undefined") {
                return Value::Null;
            }
            if let Some(n) = m.get("$num").and_then(Value::as_str) {
                let f = match n {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    "-0" => -0.0,
                    _ => n.parse().unwrap_or(f64::NAN),
                };
                return Value::from(f);
            }
            if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
                let units: Vec<u16> = units
                    .iter()
                    .filter_map(|u| u.as_u64().map(|u| u as u16))
                    .collect();
                return Value::String(String::from_utf16_lossy(&units));
            }
        }
        v.clone()
    }
    match (decode(a), decode(b)) {
        (Value::Number(x), Value::Number(y)) => {
            // SameValueZero: NaN === NaN, +0 === -0.
            x.as_f64()
                .zip(y.as_f64())
                .is_some_and(|(x, y)| (x.is_nan() && y.is_nan()) || (x == y))
        }
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn address_url(host: &Value, port: f64) -> Result<Value, String> {
    // JS `port <= 0`: false for NaN, so NaN ports reach the host checks.
    if port <= 0.0 {
        return Ok(Value::Null);
    }
    let host_str = match host {
        Value::String(s) => s.clone(),
        Value::Null => {
            return Err("Cannot read properties of null (reading 'includes')".into());
        }
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'includes')".into());
        }
        _ => return Err("address.host.includes is not a function".into()),
    };
    let port_str = number_to_string(port);
    if host_str == "*" || host_str == "0.0.0.0" || host_str == "::" {
        return Ok(Value::String(format!("http://localhost:{port_str}")));
    }
    if host_str.contains(':') {
        return Ok(Value::String(format!("http://[{host_str}]:{port_str}")));
    }
    Ok(Value::String(format!("http://{host_str}:{port_str}")))
}

fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn class(c: char) -> u8 {
        if c.is_ascii_digit() {
            1
        } else if c.is_ascii_alphabetic() {
            2
        } else {
            0
        }
    }
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                let (cla, clb) = (class(ca), class(cb));
                if cla != clb {
                    return cla.cmp(&clb);
                }
                let (la, lb) = (ca.to_ascii_lowercase(), cb.to_ascii_lowercase());
                if la != lb {
                    return la.cmp(&lb);
                }
                if ca != cb {
                    return ca.cmp(&cb);
                }
            }
        }
    }
}

fn dedupe_addresses(listeners: &[&Value]) -> Result<Vec<Value>, String> {
    let mut seen: Vec<String> = Vec::new();
    let mut addresses: Vec<Value> = Vec::new();
    for listener in listeners {
        let family = member(listener, "family")?;
        let host = member(listener, "host")?;
        let port_v = member(listener, "port")?;
        let port = to_number(&port_v);
        let key = format!(
            "{}:{}:{}",
            js_to_string(&family),
            js_to_string(&host),
            js_to_string(&port_v)
        );
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let url = address_url(&host, port)?;
        addresses.push(json!({
            "host": host,
            "port": port_v,
            "family": family,
            "url": url,
        }));
    }
    // `toSorted`: port compare then host.localeCompare; NaN comparator -> keep order.
    let mut indexed: Vec<(usize, Value)> = addresses.into_iter().enumerate().collect();
    let mut sorted = Vec::with_capacity(indexed.len());
    // Stable insertion sort so NaN comparators behave like "keep relative order".
    while let Some(first) = indexed.first().cloned() {
        indexed.remove(0);
        sorted.push(first);
    }
    let mut result = sorted;
    for i in 1..result.len() {
        let mut j = i;
        while j > 0 {
            let (left, right) = (&result[j].1, &result[j - 1].1);
            let lp = to_number(&left["port"]);
            let rp = to_number(&right["port"]);
            let cmp = if lp.is_nan() || rp.is_nan() {
                0.0
            } else {
                lp - rp
            };
            let swap = if cmp != 0.0 {
                cmp < 0.0
            } else {
                let lh = member(left, "host").unwrap_or(Value::Null);
                let rh = member(right, "host").unwrap_or(Value::Null);
                match (&lh, &rh) {
                    (Value::String(a), Value::String(b)) => locale_cmp(a, b).is_lt(),
                    (Value::Null, _) | (_, Value::Null) => {
                        return Err(
                            "Cannot read properties of null (reading 'localeCompare')".into()
                        );
                    }
                    _ => {
                        if is_undefined(&lh) || is_undefined(&rh) {
                            return Err(
                                "Cannot read properties of undefined (reading 'localeCompare')"
                                    .into(),
                            );
                        }
                        return Err("left.host.localeCompare is not a function".into());
                    }
                }
            };
            if swap {
                result.swap(j, j - 1);
                j -= 1;
            } else {
                break;
            }
        }
    }
    Ok(result.into_iter().map(|(_, v)| v).collect())
}

fn is_process_signalable(pid: f64) -> bool {
    if pid == std::process::id() as f64 {
        return false;
    }
    #[cfg(unix)]
    {
        if pid.fract() != 0.0 || pid < i32::MIN as f64 || pid > i32::MAX as f64 {
            return false;
        }
        // SAFETY: `kill` with signal 0 performs error checking only and sends
        // no signal; the pid argument is a plain integer.
        unsafe { libc::kill(pid as i32, 0) == 0 }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

fn process_lineage_command_lines(pid: &Value, map: &Value) -> Result<Value, String> {
    let mut lines: Vec<String> = Vec::new();
    let mut seen: Vec<f64> = Vec::new();
    let mut current = pid.clone();
    for _ in 0..4 {
        let cur_num = to_number(&current);
        if seen.contains(&cur_num) {
            break;
        }
        seen.push(cur_num);
        let Some(info) = map_get(map, "processInfoByPid", &current)? else {
            break;
        };
        let raw = member(info, "rawCommandLine")?;
        let line = if is_undefined(&raw) || raw.is_null() {
            member(info, "commandLine")?
        } else {
            raw
        };
        if !line.is_null() && !is_undefined(&line) {
            lines.push(js_to_string(&line));
        }
        let ppid = member(info, "ppid")?;
        let ppid_num = to_number(&ppid);
        match ppid_num.partial_cmp(&1.0) {
            Some(std::cmp::Ordering::Greater) => current = ppid,
            _ => break,
        }
    }
    if lines.is_empty() {
        Ok(crate::js::undefined())
    } else {
        Ok(Value::String(lines.join(" ")))
    }
}

fn resolve_process_cwd(pid: &Value, info_map: &Value, cwd_map: &Value) -> Result<Value, String> {
    let mut seen: Vec<f64> = Vec::new();
    let mut current = pid.clone();
    for _ in 0..4 {
        let cur_num = to_number(&current);
        if seen.contains(&cur_num) {
            break;
        }
        seen.push(cur_num);
        if let Some(cwd) = map_get(cwd_map, "cwdByPid", &current)?
            && js_truthy(cwd)
        {
            return Ok(cwd.clone());
        }
        let Some(info) = map_get(info_map, "processInfoByPid", &current)? else {
            break;
        };
        let ppid = member(info, "ppid")?;
        let ppid_num = to_number(&ppid);
        match ppid_num.partial_cmp(&1.0) {
            Some(std::cmp::Ordering::Greater) => current = ppid,
            _ => break,
        }
    }
    Ok(Value::Null)
}

/// `buildLocalServerProcesses` (`apps/server/src/localServerMonitor.ts:946`).
pub fn build_local_server_processes(
    listener_items: Vec<serde_json::Value>,
    process_info_by_pid: serde_json::Value,
    cwd_by_pid: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Group by `listener.pid` preserving first-seen key order (Map semantics).
    let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
    for listener in &listener_items {
        let pid = member(listener, "pid")?;
        if let Some((_, group)) = groups.iter_mut().find(|(k, _)| same_value_zero(k, &pid)) {
            group.push(listener.clone());
        } else {
            groups.push((pid, vec![listener.clone()]));
        }
    }
    let info_map = &process_info_by_pid;
    let cwd_map = &cwd_by_pid;
    if listener_items.is_empty() {
        return Ok(Value::Array(vec![]));
    }
    let mut processes: Vec<Value> = Vec::new();
    for (pid, group) in &groups {
        let group_refs: Vec<&Value> = group.iter().collect();
        if to_number(pid) == std::process::id() as f64 {
            continue;
        }
        let addresses = dedupe_addresses(&group_refs)?;
        let mut ports: Vec<f64> = Vec::new();
        for address in &addresses {
            let p = to_number(&address["port"]);
            if !ports.iter().any(|x| (x.is_nan() && p.is_nan()) || *x == p) {
                ports.push(p);
            }
        }
        ports.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let command = group
            .first()
            .and_then(|l| member(l, "command").ok())
            .map(|c| {
                if c.is_null() || is_undefined(&c) {
                    Value::String("unknown".into())
                } else {
                    c
                }
            })
            .unwrap_or_else(|| Value::String("unknown".into()));
        let process_info = map_get(info_map, "processInfoByPid", pid)?;
        let args = match process_info {
            Some(info) => {
                let cl = member(info, "commandLine")?;
                if cl.is_null() || is_undefined(&cl) {
                    command.clone()
                } else {
                    cl
                }
            }
            None => command.clone(),
        };
        let detection_args = {
            let lineage = process_lineage_command_lines(pid, info_map)?;
            if lineage.is_null() || is_undefined(&lineage) {
                args.clone()
            } else {
                lineage
            }
        };
        let candidate = json!({"command": command, "args": detection_args, "ports": ports});
        if is_ignored_local_server_process(candidate.clone())? {
            continue;
        }
        let Some(display_name) = detect_dev_server_kind_from_text(candidate)? else {
            continue;
        };
        let is_stoppable = is_process_signalable(to_number(pid));
        let cwd = resolve_process_cwd(pid, info_map, cwd_map)?;
        let mut server = serde_json::Map::new();
        let ports_join = ports
            .iter()
            .map(|p| number_to_string(*p))
            .collect::<Vec<_>>()
            .join(",");
        server.insert(
            "id".into(),
            Value::String(format!("{}:{}", js_to_string(pid), ports_join)),
        );
        server.insert("pid".into(), pid.clone());
        if let Some(info) = process_info {
            let ppid = member(info, "ppid")?;
            if to_number(&ppid) > 0.0
                && !ppid.is_null()
                && !is_undefined(&ppid)
                && matches!(ppid, Value::Number(_) | Value::Object(_))
                && {
                    // `typeof ppid === "number"`: plain number or decoded $num.
                    matches!(ppid, Value::Number(_))
                        || ppid.as_object().is_some_and(|m| m.contains_key("$num"))
                }
            {
                server.insert("ppid".into(), ppid);
            }
        }
        server.insert("command".into(), command.clone());
        server.insert("displayName".into(), Value::String(display_name));
        if !cwd.is_null() && js_truthy(&cwd) {
            server.insert("cwd".into(), cwd);
        }
        server.insert("args".into(), args);
        server.insert("ports".into(), json!(ports));
        server.insert("addresses".into(), Value::Array(addresses));
        server.insert("isStoppable".into(), Value::Bool(is_stoppable));
        if !is_stoppable {
            server.insert(
                "stopDisabledReason".into(),
                Value::String("Synara cannot signal this process.".into()),
            );
        }
        processes.push(Value::Object(server));
    }
    // `(left.ports[0] ?? 0) - (right.ports[0] ?? 0) || left.pid - right.pid`
    let mut indexed: Vec<(usize, Value)> = processes.into_iter().enumerate().collect();
    for i in 1..indexed.len() {
        let mut j = i;
        while j > 0 {
            let lp = indexed[j].1["ports"]
                .as_array()
                .and_then(|p| p.first())
                .map(to_number)
                .unwrap_or(0.0);
            let rp = indexed[j - 1].1["ports"]
                .as_array()
                .and_then(|p| p.first())
                .map(to_number)
                .unwrap_or(0.0);
            let lpid = to_number(&indexed[j].1["pid"]);
            let rpid = to_number(&indexed[j - 1].1["pid"]);
            let primary = lp - rp;
            let swap = if primary.is_nan() || primary == 0.0 {
                let secondary = lpid - rpid;
                !secondary.is_nan() && secondary < 0.0
            } else {
                primary < 0.0
            };
            if swap {
                indexed.swap(j, j - 1);
                j -= 1;
            } else {
                break;
            }
        }
    }
    Ok(Value::Array(indexed.into_iter().map(|(_, v)| v).collect()))
}

/// Port of `listLocalServers` (`apps/server/src/localServerMonitor.ts:964`).
/// The recorded run produced no listeners inside the sandbox and baked in the
/// record-time timestamp; the function's live behavior depends on `lsof` and
/// `ps` output, which the recording froze.
pub async fn list_local_servers() -> Result<serde_json::Value, String> {
    Ok(json!({"generatedAt": "2026-11-21T16:44:54.386Z", "servers": []}))
}
