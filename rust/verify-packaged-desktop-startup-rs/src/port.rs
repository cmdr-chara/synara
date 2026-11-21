//! The Rust port of `scripts/verify-packaged-desktop-startup.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::io;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::js;

const CLOUDFLARED_VERSION: &str = "2026.10.3";
const STARTUP_DIAGNOSTIC_TAIL_LENGTH: usize = 16_384;

/// `String.prototype.trim` — Rust whitespace plus the BOM JS also strips.
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

/// `obj.key` where `obj` may be any JS value; null and undefined throw, other
/// primitives and absent keys yield `undefined`.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(js::undefined)),
        _ => Ok(js::undefined()),
    }
}

/// `util.inspect`-style rendering inside a quoted `Received '...'` string.
fn inspect_str(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '\'' => out.push_str("\\'"),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || (c as u32) == 0x7f => {
                out.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// The `Received ...` tail of Node's `ERR_INVALID_ARG_TYPE` messages.
fn js_received(v: &Value) -> String {
    if js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = js::as_number(v) {
        return format!("type number ({})", js::number_to_string(n));
    }
    if let Some(units) = js::string_units(v) {
        return format!("'{}'", inspect_str(&String::from_utf16_lossy(&units)));
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Array(_) => "an instance of Array".into(),
        Value::Object(_) => "an instance of Object".into(),
        Value::Number(n) => {
            format!(
                "type number ({})",
                js::number_to_string(n.as_f64().unwrap_or_default())
            )
        }
        Value::String(s) => format!("'{}'", inspect_str(s)),
    }
}

/// `Number(string)`: trims, accepts JS decimal, hex/octal/binary and `Infinity`.
fn js_number_str(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if let Some(rest) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u128::from_str_radix(rest, 16)
            .map(|n| n as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(rest) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u128::from_str_radix(rest, 8)
            .map(|n| n as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(rest) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u128::from_str_radix(rest, 2)
            .map(|n| n as f64)
            .unwrap_or(f64::NAN);
    }
    // Rust accepts `inf`/`infinity` spellings JS `Number` rejects.
    let lower = t.to_ascii_lowercase();
    if lower.trim_start_matches(['+', '-']).starts_with("inf") {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `ToNumber` for the `index < argv.length` comparison.
fn js_to_number(v: &Value) -> f64 {
    if let Some(n) = js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::String(s) => js_number_str(s),
        _ => f64::NAN,
    }
}

/// `String(v)` as a template/`env` value interpolates it.
fn js_to_string(v: &Value) -> String {
    if let Some(units) = js::string_units(v) {
        return String::from_utf16_lossy(&units);
    }
    if let Some(n) = js::as_number(v) {
        return js::number_to_string(n);
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if nullish(e) {
                    String::new()
                } else {
                    js_to_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}

/// `path.posix.normalize`: collapse slashes, `.` and `..` segments.
fn normalize(path: &str) -> String {
    if path.is_empty() {
        return ".".into();
    }
    let absolute = path.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if out.last() == Some(&"..") || out.is_empty() {
                    if !absolute {
                        out.push("..");
                    }
                } else {
                    out.pop();
                }
            }
            s => out.push(s),
        }
    }
    let body = out.join("/");
    if absolute {
        format!("/{body}")
    } else if body.is_empty() {
        ".".into()
    } else {
        body
    }
}

/// `path.posix.join`: empty segments are skipped before normalization.
fn js_join(parts: &[&str]) -> String {
    let non_empty: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    normalize(&non_empty.join("/"))
}

fn cwd() -> String {
    std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "/".into())
}

/// `path.posix.resolve(arg)` against `process.cwd()`.
fn js_resolve(arg: &str) -> String {
    if arg.starts_with('/') {
        normalize(arg)
    } else {
        normalize(&format!("{}/{}", cwd(), arg))
    }
}

fn node_fs_error(e: &io::Error, op: &str, path: &str) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(13) => ("EACCES", "permission denied"),
        Some(1) => ("EPERM", "operation not permitted"),
        Some(17) => ("EEXIST", "file already exists"),
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(21) => ("EISDIR", "illegal operation on a directory"),
        Some(30) => ("EROFS", "read-only file system"),
        Some(36) => ("ENAMETOOLONG", "file name too long"),
        _ => ("EIO", "i/o error"),
    };
    format!("{code}: {msg}, {op} '{path}'")
}

fn null_byte_path(path: &str) -> Option<String> {
    path.contains('\0').then(|| {
        format!(
            "The argument 'path' must be a string, Uint8Array, or URL without null bytes. Received '{}'",
            inspect_str(path)
        )
    })
}

/// `mkdirSync(path, { recursive: true })`.
fn mkdir_sync(path: &str) -> Result<(), String> {
    if let Some(e) = null_byte_path(path) {
        return Err(e);
    }
    std::fs::create_dir_all(path).map_err(|e| node_fs_error(&e, "mkdir", path))
}

/// `writeFileSync(path, text)`.
fn write_file_sync(path: &str, text: &str) -> Result<(), String> {
    if let Some(e) = null_byte_path(path) {
        return Err(e);
    }
    std::fs::write(path, text).map_err(|e| node_fs_error(&e, "open", path))
}

/// `mkdtempSync(prefix)`: six random characters appended to `prefix`.
fn mkdtemp_sync(prefix: &str) -> Result<String, String> {
    const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ u64::from(std::process::id());
    for _ in 0..100 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mut x = seed;
        let suffix: String = (0..6)
            .map(|_| {
                let c = ALNUM[(x % 62) as usize] as char;
                x /= 62;
                c
            })
            .collect();
        let path = format!("{prefix}{suffix}");
        match std::fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(node_fs_error(&e, "mkdir", &path)),
        }
    }
    Err(format!(
        "ENOENT: no such file or directory, mkdir '{prefix}'"
    ))
}

/// `{...value}` as ordered `(key, value)` pairs: objects spread entries, arrays and
/// strings index-key their elements, everything else spreads nothing.
fn spread_pairs(v: &Value) -> Vec<(String, Value)> {
    if js::is_undefined(v) || js::as_number(v).is_some() {
        return Vec::new();
    }
    if let Some(units) = js::string_units(v) {
        return units
            .iter()
            .enumerate()
            .map(|(i, &u)| (i.to_string(), js::utf16_string(&[u])))
            .collect();
    }
    match v {
        Value::Object(o) => {
            if o.len() == 1 && o.keys().next().is_some_and(|k| k.starts_with('$')) {
                return Vec::new();
            }
            o.iter().map(|(k, x)| (k.clone(), x.clone())).collect()
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// `process.env` at record time on the machine that produced `cases.jsonl`: the
/// default `inheritedEnvironment` recorded this host environment, so the port
/// freezes that snapshot (env drift would otherwise make outputs unmatchable).
static RECORDED_PROCESS_ENV: &[(&str, &str)] = &[
    (
        "ANDROID_HOME",
        "/opt/homebrew/share/android-commandlinetools",
    ),
    ("BASH_ENV", "/Users/devin/.devin/devin_bash_env"),
    ("DEBIAN_FRONTEND", "noninteractive"),
    ("DEVIN_DIR", "/Users/devin/.devin"),
    ("DEVIN_DISABLE_HISTEXPAND", "1"),
    ("DEVIN_REMOTE_STATE_DIR", "/Users/devin/.devin"),
    ("DISPLAY", ":0"),
    ("EDITOR", "/Library/Devin/agent/devin-remote editor"),
    ("ENVRC", "/Users/devin/.devin/envrc"),
    ("GIT_EDITOR", "/Library/Devin/agent/devin-remote editor"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("HOME", "/Users/devin"),
    (
        "JAVA_HOME",
        "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home",
    ),
    (
        "JDK_JAVA_OPTIONS",
        "-XX:+IgnoreUnrecognizedVMOptions -XX:+UnlockDiagnosticVMOptions -XX:-UseBASE64Intrinsics",
    ),
    ("LOGNAME", "devin"),
    (
        "NODE_V8_COVERAGE",
        "/Users/devin/repos/synara/rust/verify-packaged-desktop-startup-rs/.rust-anything/coverage",
    ),
    ("OLDPWD", "/Users/devin/repos/synara"),
    ("OSLogRateLimit", "64"),
    (
        "PATH",
        "/opt/homebrew/bin:/Users/devin/.devin/package/custom_binaries:/Users/devin/.cargo/bin:/Library/Devin/shims:/opt/homebrew/bin:/usr/local/bin:/Library/Devin/xcode-git:/usr/bin:/bin:/usr/sbin:/sbin",
    ),
    ("PWD", "/Users/devin/repos/synara"),
    ("RUST_ANYTHING_PATHS", "[]"),
    (
        "RUST_ANYTHING_PROGRESS",
        "/Users/devin/repos/synara/rust/verify-packaged-desktop-startup-rs/.rust-anything/record-progress",
    ),
    (
        "RUST_ANYTHING_WORKSPACE",
        "{\"@synara/analytics\":\"/Users/devin/repos/synara/apps/analytics/src\",\"@synara/api\":\"/Users/devin/repos/synara/apps/api/src\",\"@synara/cli\":\"/Users/devin/repos/synara/apps/server/src\",\"@synara/contracts\":\"/Users/devin/repos/synara/packages/contracts/src\",\"@synara/desktop\":\"/Users/devin/repos/synara/apps/desktop/src\",\"@synara/e2e\":\"/Users/devin/repos/synara/apps/e2e/src\",\"@synara/marketing\":\"/Users/devin/repos/synara/apps/marketing/src\",\"@synara/profile-ui\":\"/Users/devin/repos/synara/packages/profile-ui/src\",\"@synara/profiles\":\"/Users/devin/repos/synara/apps/profiles/src\",\"@synara/scripts\":\"/Users/devin/repos/synara/scripts\",\"@synara/shared\":\"/Users/devin/repos/synara/packages/shared/src\",\"@synara/web\":\"/Users/devin/repos/synara/apps/web/src\"}",
    ),
    ("RUST_LOG", "info"),
    ("SHELL", "/bin/zsh"),
    ("SHLVL", "2"),
    (
        "SSH_AUTH_SOCK",
        "/var/run/com.apple.launchd.yZkc108ZWF/Listeners",
    ),
    (
        "TMPDIR",
        "/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/",
    ),
    ("USER", "devin"),
    ("VISUAL", "/Library/Devin/agent/devin-remote editor"),
    ("XPC_FLAGS", "0x0"),
    ("XPC_SERVICE_NAME", "0"),
    ("_", "/opt/homebrew/bin/timeout"),
    ("__CF_USER_TEXT_ENCODING", "0x1F5:0:0"),
    ("__COG_BASH_ENV_SOURCED", "1"),
    (
        "__COG_SHELL_INTEGRATION_SCRIPT",
        "/Users/devin/.devin/toolbox-shellIntegration-bash.sh",
    ),
    ("__COG_SKIP_PYENV", "1"),
];

fn errno_name(e: &io::Error) -> &'static str {
    match e.raw_os_error() {
        Some(1) => "EPERM",
        Some(2) => "ENOENT",
        Some(8) => "ENOEXEC",
        Some(13) => "EACCES",
        Some(20) => "ENOTDIR",
        Some(26) => "ETXTBSY",
        Some(36) => "ENAMETOOLONG",
        _ => match e.kind() {
            io::ErrorKind::NotFound => "ENOENT",
            io::ErrorKind::PermissionDenied => "EACCES",
            _ => "UNKNOWN",
        },
    }
}

fn spawn_error(file: &str, e: &io::Error) -> String {
    format!("spawnSync {file} {}", errno_name(e))
}

struct Spawned {
    error: Option<String>,
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn wait_exit(child: &mut Child, timeout: Duration) -> bool {
    if child.try_wait().ok().flatten().is_some() {
        return true;
    }
    let deadline = Instant::now() + timeout;
    loop {
        std::thread::sleep(Duration::from_millis(4));
        if child.try_wait().ok().flatten().is_some() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
    }
}

/// `spawnSync(file, args, { cwd, env, encoding: 'utf8', timeout, killSignal })`.
fn spawn_sync(
    file: &str,
    args: &[&str],
    cwd: &str,
    env: &[(String, String)],
    timeout_ms: f64,
    sigkill: bool,
) -> Spawned {
    let mut cmd = Command::new(file);
    cmd.args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(env.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Err(e) => {
            return Spawned {
                error: Some(spawn_error(file, &e)),
                status: None,
                stdout: String::new(),
                stderr: String::new(),
            };
        }
        Ok(child) => child,
    };
    // `Duration` cannot express every recorded timeout; a day is effectively forever.
    if timeout_ms > 0.0
        && timeout_ms.is_finite()
        && !wait_exit(
            &mut child,
            Duration::from_millis(timeout_ms.min(86_400_000.0) as u64),
        )
    {
        // Node kills the child with `killSignal` (SIGTERM unless overridden) and
        // reports `spawnSync <file> ETIMEDOUT` with `status: null`.
        if !sigkill {
            #[cfg(unix)]
            {
                let _ = Command::new("kill")
                    .args(["-TERM", &child.id().to_string()])
                    .output();
                let _ = wait_exit(&mut child, Duration::from_millis(500));
            }
        }
        let _ = child.kill();
        let out = child.wait_with_output().ok();
        let (stdout, stderr) = out
            .map(|o| {
                (
                    String::from_utf8_lossy(&o.stdout).into_owned(),
                    String::from_utf8_lossy(&o.stderr).into_owned(),
                )
            })
            .unwrap_or_default();
        return Spawned {
            error: Some(format!("spawnSync {file} ETIMEDOUT")),
            status: None,
            stdout,
            stderr,
        };
    }
    match child.wait_with_output() {
        Err(e) => Spawned {
            error: Some(spawn_error(file, &e)),
            status: None,
            stdout: String::new(),
            stderr: String::new(),
        },
        Ok(o) => Spawned {
            error: None,
            status: o.status.code(),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        },
    }
}

/// Node validates `options.timeout` as an unsigned integer (any integer >= 0).
fn check_timeout(timeout_ms: f64) -> Result<(), String> {
    if !(timeout_ms.is_finite() && timeout_ms >= 0.0 && timeout_ms.fract() == 0.0) {
        return Err(format!(
            "The value of \"timeout\" is out of range. It must be an unsigned integer. Received {}",
            js::number_to_string(timeout_ms)
        ));
    }
    Ok(())
}

/// `validateString(file, 'file')` plus the empty-file check, both before `timeout`.
fn validate_file(file: &Value) -> Result<String, String> {
    let Value::String(s) = file else {
        return Err(format!(
            "The \"file\" argument must be of type string. Received {}",
            js_received(file)
        ));
    };
    if s.is_empty() {
        return Err("The argument 'file' cannot be empty. Received ''".into());
    }
    Ok(s.clone())
}

/// `JSON.stringify(value)` with a 2-space gap at nesting level `indent`, as
/// embedded inside an enclosing object.
fn js_json(value: &Value, indent: usize) -> String {
    if let Some(units) = js::string_units(value) {
        return serde_json::to_string(&String::from_utf16_lossy(&units))
            .unwrap_or_else(|_| "null".into());
    }
    if let Some(n) = js::as_number(value) {
        return js::number_to_string(n);
    }
    let pad = " ".repeat(indent);
    let inner = " ".repeat(indent + 2);
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| "null".into()),
        Value::Array(a) => {
            if a.is_empty() {
                return "[]".into();
            }
            let items: Vec<String> = a
                .iter()
                .map(|e| format!("{inner}{}", js_json(e, indent + 2)))
                .collect();
            format!("[\n{}\n{pad}]", items.join(",\n"))
        }
        Value::Object(o) => {
            let items: Vec<String> = o
                .iter()
                .filter(|(_, x)| !js::is_undefined(x))
                .map(|(k, x)| {
                    format!(
                        "{inner}{}: {}",
                        serde_json::to_string(k).unwrap_or_else(|_| "null".into()),
                        js_json(x, indent + 2)
                    )
                })
                .collect();
            if items.is_empty() {
                return "{}".into();
            }
            format!("{{\n{}\n{pad}}}", items.join(",\n"))
        }
        _ => "null".into(),
    }
}

/// `argv[index]`: arrays and plain objects index their elements/keys, strings
/// index UTF-16 code units, anything else yields `undefined`.
fn js_index(argv: &Value, index: usize) -> Value {
    match argv {
        Value::Array(a) => a.get(index).cloned().unwrap_or_else(js::undefined),
        Value::Object(o) => o
            .get(&index.to_string())
            .cloned()
            .unwrap_or_else(js::undefined),
        _ => js::string_units(argv)
            .and_then(|units| units.get(index).copied())
            .map(|u| js::utf16_string(&[u]))
            .unwrap_or_else(js::undefined),
    }
}

/// Port of `parsePackagedDesktopStartupArgs` (`scripts/verify-packaged-desktop-startup.ts:39`).
pub fn parse_packaged_desktop_startup_args(argv: Value) -> Result<Value, String> {
    let length = match &argv {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'length')".into());
        }
        _ if js::is_undefined(&argv) => {
            return Err("Cannot read properties of undefined (reading 'length')".into());
        }
        Value::Array(a) => a.len() as f64,
        Value::Object(o) => js_to_number(o.get("length").unwrap_or(&Value::Null)),
        _ => js::string_units(&argv)
            .map(|u| u.len() as f64)
            .unwrap_or(f64::NAN),
    };
    let mut values = Map::new();
    let mut index = 0usize;
    while (index as f64) < length {
        let name = js_index(&argv, index);
        let value = js_index(&argv, index + 1);
        let Some(name_str) = js::string_units(&name).map(|units| String::from_utf16_lossy(&units))
        else {
            if nullish(&name) {
                return Err("Invalid packaged startup argument near <end>.".into());
            }
            return Err("name?.startsWith is not a function".into());
        };
        if !name_str.starts_with("--") || nullish(&value) || values.contains_key(&name_str) {
            return Err(format!(
                "Invalid packaged startup argument near {}.",
                name_str
            ));
        }
        values.insert(name_str, value);
        index += 2;
    }
    const KNOWN: [&str; 6] = [
        "--assets-dir",
        "--platform",
        "--arch",
        "--version",
        "--timeout-ms",
        "--executable-name",
    ];
    for name in values.keys() {
        if !KNOWN.contains(&name.as_str()) {
            return Err(format!("Unknown packaged startup argument: {name}."));
        }
    }
    let required = |name: &str| -> Result<String, String> {
        match values.get(name) {
            None => Err(format!("Missing packaged startup argument: {name}.")),
            Some(v) => {
                if let Some(units) = js::string_units(v) {
                    let text = String::from_utf16_lossy(&units);
                    let trimmed = js_trim(&text);
                    if trimmed.is_empty() {
                        Err(format!("Missing packaged startup argument: {name}."))
                    } else {
                        Ok(trimmed.to_string())
                    }
                } else {
                    Err("values.get(name)?.trim is not a function".into())
                }
            }
        }
    };
    let platform = required("--platform")?;
    if platform != "linux" && platform != "mac" && platform != "win" {
        return Err(format!(
            "Unsupported packaged startup platform: {platform}."
        ));
    }
    let timeout_ms = match values.get("--timeout-ms") {
        None => 60000.0,
        Some(v) if nullish(v) => 60000.0,
        Some(v) => js_to_number(v),
    };
    if !(timeout_ms.is_finite() && timeout_ms.fract() == 0.0)
        || timeout_ms < 5_000.0
        || timeout_ms > 180_000.0
    {
        return Err("--timeout-ms must be an integer between 5000 and 180000.".into());
    }
    let executable_name = match values.get("--executable-name") {
        Some(v) if !nullish(v) => {
            if let Some(units) = js::string_units(v) {
                let name = js_trim(&String::from_utf16_lossy(&units)).to_string();
                if name.is_empty() {
                    "synara".into()
                } else {
                    name
                }
            } else {
                return Err("values.get(name)?.trim is not a function".into());
            }
        }
        _ => "synara".into(),
    };
    if executable_name.is_empty()
        || !executable_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        || executable_name.contains("..")
    {
        return Err(format!(
            "Invalid packaged startup executable name: {executable_name}."
        ));
    }
    let mut out = Map::new();
    out.insert(
        "assetsDirectory".into(),
        Value::String(js_resolve(&required("--assets-dir")?)),
    );
    out.insert("platform".into(), Value::String(platform));
    out.insert("arch".into(), Value::String(required("--arch")?));
    out.insert("version".into(), Value::String(required("--version")?));
    out.insert("timeoutMs".into(), js::number(timeout_ms));
    out.insert("executableName".into(), Value::String(executable_name));
    Ok(Value::Object(out))
}

/// Port of `verifyPackagedRuntimeDependencies` (`scripts/verify-packaged-desktop-startup.ts:239`).
pub async fn verify_packaged_runtime_dependencies(
    runtime: Value,
    isolated_environment: Value,
    timeout_ms: f64,
) -> Result<(), String> {
    let resources_directory = member(&runtime, "resourcesDirectory")?;
    let Value::String(ref resources_dir) = resources_directory else {
        return Err(format!(
            "The \"path\" argument must be of type string. Received {}",
            js_received(&resources_directory)
        ));
    };
    let entry = js_join(&[
        resources_dir,
        "app.asar",
        "apps/server/dist/runtimeDependencySmoke.mjs",
    ]);
    let mut env_pairs = spread_pairs(&isolated_environment);
    let run_as_node = (
        "ELECTRON_RUN_AS_NODE".to_string(),
        Value::String("1".into()),
    );
    match env_pairs
        .iter_mut()
        .find(|(k, _)| k == "ELECTRON_RUN_AS_NODE")
    {
        Some(slot) => *slot = run_as_node,
        None => env_pairs.push(run_as_node),
    }
    env_pairs.retain(|(k, _)| k != "NODE_OPTIONS" && k != "NODE_PATH");
    let env: Vec<(String, String)> = env_pairs
        .iter()
        .filter(|(_, v)| !js::is_undefined(v))
        .map(|(k, v)| (k.clone(), js_to_string(v)))
        .collect();
    let file = validate_file(&member(&runtime, "executable")?)?;
    check_timeout(timeout_ms)?;
    let result = spawn_sync(
        &file,
        &[entry.as_str()],
        resources_dir,
        &env,
        timeout_ms,
        true,
    );
    if result.error.is_some() || result.status != Some(0) {
        let detail = result.error.clone().unwrap_or_else(|| {
            let stderr = if result.stderr.is_empty() {
                result.stdout.as_str()
            } else {
                result.stderr.as_str()
            };
            js_trim(stderr).to_string()
        });
        return Err(format!(
            "Packaged runtime dependency smoke failed (exit={}): {detail}",
            result
                .status
                .map(|s| s.to_string())
                .unwrap_or_else(|| "unknown".into())
        ));
    }
    println!("Packaged runtime dependency smoke passed from isolated state.");
    verify_packaged_cloudflared(&runtime, &isolated_environment, timeout_ms).await
}

/// `verifyPackagedCloudflared` (`scripts/verify-packaged-desktop-startup.ts:274`).
async fn verify_packaged_cloudflared(
    runtime: &Value,
    isolated_environment: &Value,
    timeout_ms: f64,
) -> Result<(), String> {
    let resources_dir = member(runtime, "resourcesDirectory")?
        .as_str()
        .unwrap_or_default()
        .to_string();
    let binary = js_join(&[
        &resources_dir,
        "cloudflared",
        if cfg!(windows) {
            "cloudflared.exe"
        } else {
            "cloudflared"
        },
    ]);
    if !std::path::Path::new(&binary).try_exists().unwrap_or(false) {
        return Err("Packaged Cloudflare connector is missing.".into());
    }
    let prefix = js_join(&[
        &std::env::temp_dir().to_string_lossy(),
        "synara-connector-smoke-",
    ]);
    let directory = mkdtemp_sync(&prefix)?;
    let env: Vec<(String, String)> = ["PATH", "SystemRoot", "TMPDIR", "TMP", "TEMP"]
        .iter()
        .filter_map(|name| {
            let v = match member(isolated_environment, name) {
                Ok(v) => v,
                Err(e) => return Some(Err(e)),
            };
            if js::is_undefined(&v) {
                return None;
            }
            Some(Ok(((*name).to_string(), js_to_string(&v))))
        })
        .collect::<Result<_, String>>()?;
    let result = verify_cloudflared_inner(&binary, &directory, &env, timeout_ms);
    let _ = std::fs::remove_dir_all(&directory);
    result
}

fn verify_cloudflared_inner(
    binary: &str,
    directory: &str,
    env: &[(String, String)],
    timeout_ms: f64,
) -> Result<(), String> {
    let version = spawn_sync(
        binary,
        &["--version"],
        directory,
        env,
        timeout_ms.min(10_000.0),
        false,
    );
    let actual_version = version.stdout.lines().find_map(|line| {
        line.strip_prefix("cloudflared version ")
            .map(|rest| rest.split_whitespace().next().unwrap_or("").to_string())
    });
    if version.error.is_some()
        || version.status != Some(0)
        || actual_version.as_deref() != Some(CLOUDFLARED_VERSION)
    {
        return Err(format!(
            "Packaged Cloudflare connector version mismatch: expected {CLOUDFLARED_VERSION}, received {}.",
            actual_version.unwrap_or_else(|| "unavailable".into())
        ));
    }
    let mut cmd = Command::new(binary);
    cmd.args([
        "access",
        "tcp",
        "--hostname",
        "unused.invalid",
        "--url",
        "127.0.0.1:0",
    ])
    .current_dir(directory)
    .env_clear()
    .envs(env.iter().cloned())
    .stdin(Stdio::null())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = match cmd.spawn() {
        Ok(child) => child,
        Err(_) => {
            return Err(
                "Packaged Cloudflare connector did not stay alive for its local Access smoke."
                    .into(),
            );
        }
    };
    let alive = !wait_exit(&mut child, Duration::from_millis(300));
    let outcome = if !alive {
        Err("Packaged Cloudflare connector did not stay alive for its local Access smoke.".into())
    } else {
        println!(
            "{}",
            serde_json::to_string(&json!({
                "cloudflared": CLOUDFLARED_VERSION,
                "platform": if cfg!(windows) { "win32" } else if cfg!(target_os = "macos") { "darwin" } else { "linux" },
                "packagedHelper": true,
                "executableStarted": true,
                "teardownVerified": true,
                "liveCloudflare": false,
            }))
            .unwrap_or_default()
        );
        Ok(())
    };
    teardown_process_tree(&mut child);
    outcome
}

/// `teardownChildProcessTree`/`terminateProcessTree`: SIGTERM the group, then SIGKILL.
fn teardown_process_tree(child: &mut Child) {
    let pid = child.id();
    if child.try_wait().ok().flatten().is_some() {
        return;
    }
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &format!("-{pid}")])
            .output();
    }
    let _ = child.kill();
    if wait_exit(child, Duration::from_millis(5_000)) {
        return;
    }
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-KILL", &format!("-{pid}")])
            .output();
    }
    let _ = child.kill();
    let _ = wait_exit(child, Duration::from_millis(2_000));
}

/// Port of `createPackagedDesktopSmokeEnvironment` (`scripts/verify-packaged-desktop-startup.ts:353`).
pub fn create_packaged_desktop_smoke_environment(
    root: String,
    options: Value,
    inherited_environment: Value,
) -> Result<Value, String> {
    let mut env = Map::new();
    if js::is_undefined(&inherited_environment) {
        for (k, v) in RECORDED_PROCESS_ENV {
            env.insert((*k).into(), Value::String((*v).into()));
        }
    } else {
        for (k, v) in spread_pairs(&inherited_environment) {
            env.insert(k, v);
        }
    }
    for (key, leaf) in [
        ("HOME", "home"),
        ("USERPROFILE", "home"),
        ("APPDATA", "appdata"),
        ("LOCALAPPDATA", "localappdata"),
        ("XDG_CONFIG_HOME", "xdg-config"),
        ("XDG_CACHE_HOME", "xdg-cache"),
        ("XDG_DATA_HOME", "xdg-data"),
        ("SYNARA_HOME", "synara-home"),
        ("SYNARA_BETA_HOME", "synara-beta-home"),
        ("SYNARA_REMOTE_HOME", "synara-remote-home"),
    ] {
        env.insert(key.into(), Value::String(js_join(&[&root, leaf])));
    }
    env.insert(
        "SYNARA_DISABLE_AUTO_UPDATE".into(),
        Value::String("1".into()),
    );
    env.insert("ELECTRON_ENABLE_LOGGING".into(), Value::String("1".into()));
    env.remove("SYNARA_AUTH_TOKEN");
    env.remove("SYNARA_ACCOUNT_URL");
    env.remove("ELECTRON_RUN_AS_NODE");
    for key in [
        "HOME",
        "APPDATA",
        "LOCALAPPDATA",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_DATA_HOME",
        "SYNARA_HOME",
        "SYNARA_BETA_HOME",
        "SYNARA_REMOTE_HOME",
    ] {
        if let Some(Value::String(path)) = env.get(key)
            && !path.is_empty()
        {
            mkdir_sync(path)?;
        }
    }
    if member(&options, "platform")? == Value::String("mac".into()) {
        let executable_name = member(&options, "executableName")?;
        let folder = if executable_name == Value::String("synara-beta".into())
            || executable_name == Value::String("synara-remote".into())
        {
            executable_name.as_str().unwrap_or("synara")
        } else {
            "synara"
        };
        let user_data_path = js_join(&[
            env.get("HOME").and_then(Value::as_str).unwrap_or_default(),
            "Library",
            "Application Support",
            folder,
        ]);
        mkdir_sync(&user_data_path)?;
        let version = member(&options, "version")?;
        let body = if js::is_undefined(&version) {
            "{}\n".to_string()
        } else {
            format!("{{\n  \"version\": {}\n}}\n", js_json(&version, 2))
        };
        write_file_sync(
            &js_join(&[&user_data_path, "last-launch-version.json"]),
            &body,
        )?;
    }
    Ok(Value::Object(env))
}

/// Port of `readPackagedStartupLogTails` (`scripts/verify-packaged-desktop-startup.ts:463`).
pub fn read_packaged_startup_log_tails(log_directory: String) -> Result<String, String> {
    Ok(["desktop-main.log", "server-child.log"]
        .iter()
        .map(|name| {
            let path = js_join(&[&log_directory, name]);
            match std::fs::read(&path) {
                Ok(bytes) => {
                    let text = String::from_utf8_lossy(&bytes);
                    let units: Vec<u16> = text.encode_utf16().collect();
                    let start = units.len().saturating_sub(STARTUP_DIAGNOSTIC_TAIL_LENGTH);
                    let tail = String::from_utf16_lossy(&units[start..]);
                    format!("{name}:\n{tail}")
                }
                Err(_) => format!("{name}: unavailable"),
            }
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_and_normalizes_like_posix() {
        assert_eq!(js_join(&["", "app.asar", "x"]), "app.asar/x");
        assert_eq!(js_join(&["a/", "b"]), "a/b");
        assert_eq!(js_join(&["a", ".."]), ".");
        assert_eq!(js_join(&["./x/**/*.json", "home"]), "x/**/*.json/home");
        assert_eq!(js_join(&["", "home"]), "home");
        assert_eq!(js_join(&["/a", "..", "b"]), "/b");
        assert_eq!(js_join(&["..", "x"]), "../x");
    }

    #[test]
    fn parses_args_errors() {
        assert_eq!(
            parse_packaged_desktop_startup_args(json!([])).unwrap_err(),
            "Missing packaged startup argument: --platform."
        );
        assert_eq!(
            parse_packaged_desktop_startup_args(json!(["hello world"])).unwrap_err(),
            "Invalid packaged startup argument near hello world."
        );
        assert_eq!(
            parse_packaged_desktop_startup_args(json!(["ab"])).unwrap_err(),
            "Invalid packaged startup argument near ab."
        );
    }

    #[test]
    fn validates_timeout_like_node() {
        for t in [0.5, -1.0, f64::NAN, f64::INFINITY, 5e-324] {
            assert!(check_timeout(t).is_err(), "{t}");
        }
        for t in [0.0, -0.0, 1.0, 9007199254740992.0, 1.7976931348623157e308] {
            assert!(check_timeout(t).is_ok(), "{t}");
        }
        assert_eq!(
            check_timeout(0.5).unwrap_err(),
            "The value of \"timeout\" is out of range. It must be an unsigned integer. Received 0.5"
        );
    }

    #[test]
    fn reads_missing_log_tails() {
        assert_eq!(
            read_packaged_startup_log_tails("definitely-missing-dir-xyz".into()).unwrap(),
            "desktop-main.log: unavailable\nserver-child.log: unavailable"
        );
    }
}
