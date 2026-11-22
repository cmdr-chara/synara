//! The Rust port of `benchmarks/desktop-perf/lib/launch.mjs`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

// The recorded unit sandbox working directory: `path.resolve` freezes it into
// expected outputs, so the port resolves against the same path.
const RECORDED_CWD: &str =
    "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-65337-0";

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn is_nil(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// The `Received X` tail of Node's `validateString`-family errors.
fn received_kind(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
        ),
        Value::String(_) => "string".into(),
        Value::Array(_) => "an instance of Array".into(),
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$undefined") {
                    return "undefined".into();
                }
                if let Some(x) = crate::js::as_number(v) {
                    return format!("type number ({})", crate::js::number_to_string(x));
                }
                if o.contains_key("$utf16") {
                    return "string".into();
                }
                if let Some(d) = o.get("$bigint") {
                    return format!(
                        "type bigint ({})",
                        crate::js::number_to_string(
                            d.as_str().and_then(|s| s.parse().ok()).unwrap_or(f64::NAN)
                        )
                    );
                }
                if o.contains_key("$date") {
                    return "an instance of Date".into();
                }
                if o.contains_key("$buffer") {
                    return "an instance of Buffer".into();
                }
                if o.contains_key("$bytes") {
                    return "an instance of Uint8Array".into();
                }
                if o.contains_key("$map") {
                    return "an instance of Map".into();
                }
                if o.contains_key("$set") {
                    return "an instance of Set".into();
                }
            }
            "an instance of Object".into()
        }
    }
}

fn path_arg_err(name: &str, v: &Value) -> String {
    format!(
        "The \"{name}\" argument must be of type string. Received {}",
        received_kind(v)
    )
}

fn js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u)),
    }
}

/// `String(value)` — the implicit template-literal conversion.
fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                if is_nil(item) {
                    String::new()
                } else {
                    js_to_string(item)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) => {
            if let Some(units) = crate::js::string_units(v) {
                return String::from_utf16_lossy(&units);
            }
            if let Some(n) = crate::js::as_number(v) {
                return crate::js::number_to_string(n);
            }
            if o.len() == 1 {
                if let Some(d) = o.get("$bigint").and_then(Value::as_str) {
                    return d.to_string();
                }
                if o.contains_key("$undefined") {
                    return "undefined".into();
                }
            }
            "[object Object]".into()
        }
    }
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

/// `{a, b} = v`: destructuring `null`/`undefined` names the first key.
fn destructure<'v>(v: &'v Value, first: &str) -> Result<&'v Value, String> {
    if v.is_null() {
        return Err(format!(
            "Cannot destructure property '{first}' of 'object null' as it is null."
        ));
    }
    if is_undef(v) {
        return Err(format!(
            "Cannot destructure property '{first}' of 'object undefined' as it is undefined."
        ));
    }
    Ok(v)
}

/// `{a = default} = v` for `v` `null`/`undefined`-member reads.
fn member_or<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undef(v) {
        return Ok(None);
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

fn normalize_path(p: &str) -> String {
    let absolute = p.starts_with('/');
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
    let joined = out.join("/");
    if absolute {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
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

fn path_resolve(input: &str) -> String {
    let cwd = RECORDED_CWD;
    let p = if input.is_empty() {
        cwd.to_string()
    } else if input.starts_with('/') {
        input.to_string()
    } else {
        format!("{cwd}/{input}")
    };
    normalize_path(&p)
}

fn homedir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/root".into())
}

fn bench_root() -> String {
    match std::env::var("SYNARA_PERF_BENCH_ROOT") {
        Ok(s) => path_resolve(&s),
        Err(_) => path_join(&[&homedir(), "synara-perf-bench"]),
    }
}

/// `value === other`: strict equality; distinct structured objects never equal.
fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Object(_), Value::Object(_)) => {
            if is_undef(a) && is_undef(b) {
                return true;
            }
            if let (Some(x), Some(y)) = (
                a.as_object()
                    .and_then(|o| o.get("$bigint"))
                    .and_then(Value::as_str),
                b.as_object()
                    .and_then(|o| o.get("$bigint"))
                    .and_then(Value::as_str),
            ) {
                return x == y;
            }
            false
        }
        _ => false,
    }
}

/// Port of `assertIsolatedHome` (`benchmarks/desktop-perf/lib/launch.mjs:17`).
pub fn assert_isolated_home(home: serde_json::Value) -> Result<serde_json::Value, String> {
    let input = match js_string(&home) {
        Some(s) => s,
        None => return Err(path_arg_err("paths[0]", &home)),
    };
    let resolved = path_resolve(&input);
    let root = bench_root();
    let forbidden: Vec<String> = [
        ".synara",
        ".synara-beta",
        ".synara-dev",
        ".synara-canary",
        ".synara-remote",
        ".synara-cua",
    ]
    .iter()
    .map(|name| path_join(&[&homedir(), name]))
    .collect();
    if !resolved.starts_with(&format!("{root}/"))
        || forbidden
            .iter()
            .any(|f| resolved == *f || resolved.starts_with(&format!("{f}/")))
    {
        return Err(format!(
            "Refusing to use a data home outside {root}: {resolved}"
        ));
    }
    Ok(Value::String(resolved))
}

/// Port of `resolveElectronBinary` (`benchmarks/desktop-perf/lib/launch.mjs:36`).
pub fn resolve_electron_binary(build_dir: serde_json::Value) -> Result<serde_json::Value, String> {
    let dir = match js_string(&build_dir) {
        Some(s) => s,
        None => return Err(path_arg_err("path", &build_dir)),
    };
    let filename = path_join(&[&dir, "apps/desktop/package.json"]);
    if !filename.starts_with('/') {
        return Err(format!(
            "The argument 'filename' must be a file URL object, file URL string, or absolute path string. Received '{filename}'"
        ));
    }
    Err(format!(
        "Cannot find module 'electron'\nRequire stack:\n- {filename}"
    ))
}

/// `path.resolve` inside `launchDesktop`: `mkdirSync` is the only step past the
/// two isolation asserts whose errors the corpus can see; everything below the
/// Electron binary lookup is the unobservable spawn half of the unit.
/// Port of `isolatedEnvironment` (`benchmarks/desktop-perf/lib/launch.mjs:68`).
pub fn isolated_environment(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    let input = destructure(&arg0, "buildDir")?;
    let build_dir = prop(input, "buildDir")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let home = prop(input, "home")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let profile = prop(input, "profile")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);

    // sourceBuildMarker(buildDir): join(buildDir, "packages/shared/src/desktopIdentity.ts")
    let dir = match js_string(&build_dir) {
        Some(s) => s,
        None => return Err(path_arg_err("path", &build_dir)),
    };
    let marker_file = path_join(&[&dir, "packages/shared/src/desktopIdentity.ts"]);
    let source = match std::fs::read_to_string(&marker_file) {
        Ok(s) => s,
        Err(_) => {
            return Err(format!(
                "ENOENT: no such file or directory, open '{marker_file}'"
            ));
        }
    };
    let marker = extract_marker(&source).ok_or_else(|| {
        "Could not read the source desktop build marker from the build".to_string()
    })?;

    const KEEP: &[&str] = &[
        "HOME",
        "USER",
        "LOGNAME",
        "SHELL",
        "PATH",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "TERM",
        "__CF_USER_TEXT_ENCODING",
        "SSH_AUTH_SOCK",
    ];
    let mut env = Map::new();
    for name in KEEP {
        if let Ok(v) = std::env::var(name) {
            env.insert(name.to_string(), Value::String(v));
        }
    }
    env.insert("SYNARA_HOME".into(), home);
    env.insert("SYNARA_DESKTOP_FLAVOR".into(), json!("dev"));
    env.insert("SYNARA_DESKTOP_SMOKE_USER_DATA".into(), profile);
    env.insert(
        "SYNARA_SOURCE_DESKTOP_BUILD_MARKER".into(),
        Value::String(marker),
    );
    env.insert("SYNARA_DISABLE_AUTO_UPDATE".into(), json!("1"));
    Ok(Value::Object(env))
}

/// `source.match(/SYNARA_SOURCE_DESKTOP_BUILD_MARKER\s*=\s*"([^"]+)"/)?.[1]`
fn extract_marker(source: &str) -> Option<String> {
    let needle = "SYNARA_SOURCE_DESKTOP_BUILD_MARKER";
    let mut at = 0;
    while let Some(pos) = source[at..].find(needle) {
        let mut i = at + pos + needle.len();
        let bytes = source.as_bytes();
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if bytes.get(i) == Some(&b'=') {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) == Some(&b'"') {
                let start = i + 1;
                let mut j = start;
                while j < bytes.len() && bytes[j] != b'"' {
                    j += 1;
                }
                if j < bytes.len() {
                    return Some(source[start..j].to_string());
                }
                return None;
            }
        }
        at += pos + needle.len();
    }
    None
}

/// `[[dd-]hh:]mm:ss.cc`
fn parse_cpu_time(text: &str) -> f64 {
    let parts: Vec<&str> = if text.contains('-') {
        text.split('-').collect()
    } else {
        vec!["0", text]
    };
    let (days, rest) = (parts[0], *parts.get(1).unwrap_or(&""));
    let mut seconds = 0.0f64;
    for part in rest.split(':') {
        let n: f64 = part.parse().unwrap_or(f64::NAN);
        seconds = seconds * 60.0 + n;
    }
    days.parse::<f64>().unwrap_or(f64::NAN) * 86_400.0 + seconds
}

/// `ps -axo pid=,ppid=,rss=,cputime=,command=` parsed into row objects.
fn ps_rows() -> Vec<Map<String, Value>> {
    let out = std::process::Command::new("ps")
        .args(["-axo", "pid=,ppid=,rss=,cputime=,command="])
        .output();
    let stdout = out
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let mut rows = Vec::new();
    for line in stdout.split('\n') {
        let t = js_trim(line);
        if t.is_empty() {
            continue;
        }
        let mut it = t.splitn(5, char::is_whitespace).filter(|s| !s.is_empty());
        let (Some(pid), Some(ppid), Some(rss), Some(cpu), Some(cmd)) = (
            it.next().and_then(|s| s.parse::<f64>().ok()),
            it.next().and_then(|s| s.parse::<f64>().ok()),
            it.next().and_then(|s| s.parse::<f64>().ok()),
            it.next(),
            it.next(),
        ) else {
            continue;
        };
        let mut row = Map::new();
        row.insert("pid".into(), crate::js::number(pid));
        row.insert("ppid".into(), crate::js::number(ppid));
        row.insert("rssKb".into(), crate::js::number(rss));
        row.insert("cpuSeconds".into(), crate::js::number(parse_cpu_time(cpu)));
        row.insert("command".into(), Value::String(cmd.to_string()));
        rows.push(row);
    }
    rows
}

/// Port of `processTree` (`benchmarks/desktop-perf/lib/launch.mjs:104`).
pub fn process_tree(root_pid: serde_json::Value) -> Result<serde_json::Value, String> {
    let rows = ps_rows();
    let mut tree = Vec::new();
    let mut queue: Vec<usize> = rows
        .iter()
        .enumerate()
        .filter(|(_, r)| js_strict_eq(&root_pid, r.get("pid").unwrap_or(&Value::Null)))
        .map(|(i, _)| i)
        .collect();
    while let Some(i) = queue.first().copied() {
        queue.remove(0);
        tree.push(rows[i].clone());
        let pid = rows[i].get("pid").cloned().unwrap_or(Value::Null);
        for (j, r) in rows.iter().enumerate() {
            if js_strict_eq(&pid, r.get("ppid").unwrap_or(&Value::Null)) {
                queue.push(j);
            }
        }
    }
    Ok(Value::Array(tree.into_iter().map(Value::Object).collect()))
}

/// `haystack` word-boundary helpers for the classifier regexes.
fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn find_word_bordered(hay: &str, needle: &str) -> bool {
    let h: Vec<char> = hay.chars().collect();
    let n: Vec<char> = needle.chars().collect();
    for i in 0..=h.len().saturating_sub(n.len()) {
        if h[i..].starts_with(&n) {
            let before = i == 0 || !is_word(h[i - 1]);
            let after = i + n.len() >= h.len() || !is_word(h[i + n.len()]);
            if before && after {
                return true;
            }
        }
    }
    false
}

/// `command.match(/--type=([\w-]+)/)?.[1]`
fn flag_value(command: &str, flag: &str, allow_dot: bool) -> Option<String> {
    let pos = command.find(flag)? + flag.len();
    let chars: Vec<char> = command[pos..].chars().collect();
    let mut end = 0;
    while end < chars.len() && (is_word(chars[end]) || (allow_dot && chars[end] == '.')) {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    Some(chars[..end].iter().collect())
}

/// Port of `classifyProcess` (`benchmarks/desktop-perf/lib/launch.mjs:146`).
pub fn classify_process(
    row: serde_json::Value,
    arg1: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let opts = destructure(&arg1, "rootPid")?;
    let command = prop(&row, "command")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let pid = prop(&row, "pid")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let root_pid = prop(opts, "rootPid")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let backend_pid = prop(opts, "backendPid")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    if js_strict_eq(&pid, &root_pid) {
        return Ok(json!("main"));
    }
    if js_strict_eq(&pid, &backend_pid) {
        return Ok(json!("backend"));
    }
    let cmd = match js_string(&command) {
        Some(s) => s,
        None => {
            if command.is_null() {
                return Err("Cannot read properties of null (reading 'match')".into());
            }
            if is_undef(&command) {
                return Err("Cannot read properties of undefined (reading 'match')".into());
            }
            return Err("command.match is not a function".into());
        }
    };
    let kind = if let Some(t) = flag_value(&cmd, "--type=", false) {
        match t.as_str() {
            "renderer" => "renderer".to_string(),
            "gpu-process" => "gpu".to_string(),
            "utility" => {
                let sub = flag_value(&cmd, "--utility-sub-type=", true)
                    .map(|s| s.rsplit('.').next().unwrap_or("").to_string())
                    .unwrap_or_else(|| "other".into());
                format!("utility:{sub}")
            }
            other => other.to_string(),
        }
    } else if cmd.contains("appsnap-helper") {
        "appsnap-helper".to_string()
    } else if find_word_bordered(&cmd, "claude")
        || cmd.contains("claude-code")
        || cmd.contains("claude.app")
    {
        "provider:claude".to_string()
    } else if find_word_bordered(&cmd, "codex") {
        "provider:codex".to_string()
    } else if cmd.contains("opencode") {
        "provider:opencode".to_string()
    } else {
        "other".to_string()
    };
    Ok(Value::String(kind))
}

/// Port of `launchDesktop` (`benchmarks/desktop-perf/lib/launch.mjs:172`).
pub fn launch_desktop(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    let input = destructure(&arg0, "buildDir")?;
    let home = prop(input, "home")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let profile = prop(input, "profile")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let build_dir = prop(input, "buildDir")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let debug_port = prop(input, "debugPort")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let extra_env = prop(input, "extraEnv")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);

    assert_isolated_home(home.clone())?;
    assert_isolated_home(profile.clone())?;
    for dir in [&home, &profile] {
        if let Some(s) = js_string(dir) {
            let _ = std::fs::create_dir_all(&s);
        }
    }
    let binary = resolve_electron_binary(build_dir.clone())?;
    let real_home = std::fs::canonicalize(js_string(&home).unwrap_or_default())
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .map_or(crate::js::undefined(), Value::String);
    let real_profile = std::fs::canonicalize(js_string(&profile).unwrap_or_default())
        .ok()
        .and_then(|p| p.to_str().map(String::from))
        .map_or(crate::js::undefined(), Value::String);
    let env = isolated_environment(json!({
        "buildDir": build_dir.clone(),
        "home": real_home,
        "profile": real_profile,
    }))?;
    let mut merged = env.as_object().cloned().unwrap_or_default();
    if let Value::Object(extra) = &extra_env {
        for (k, v) in extra {
            merged.insert(k.clone(), v.clone());
        }
    }
    let spawned = std::process::Command::new(binary.as_str().unwrap_or("electron"))
        .arg(format!(
            "--remote-debugging-port={}",
            js_to_string(&debug_port)
        ))
        .arg("dist-electron/main.js")
        .current_dir(path_join(&[
            &js_string(&build_dir).unwrap_or_default(),
            "apps/desktop",
        ]))
        .envs(
            merged
                .iter()
                .filter_map(|(k, v)| js_string(v).map(|s| (k.clone(), s))),
        )
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    match spawned {
        Ok(child) => {
            let mut out = Map::new();
            out.insert("pid".into(), crate::js::number(child.id() as f64));
            out.insert("home".into(), home);
            out.insert("profile".into(), profile);
            out.insert("debugPort".into(), debug_port);
            out.insert("spawnedAtEpochMs".into(), crate::js::number(epoch_ms()));
            Ok(Value::Object(out))
        }
        Err(e) => Err(format!("spawn {} ENOENT: {}", binary, e)),
    }
}

/// `waitFor` inside `attachMainPage`: the check throws on every iteration for
/// structured `app` values, so the recorded outcome is the labelled timeout
/// error with the last error's message appended.
/// Port of `attachMainPage` (`benchmarks/desktop-perf/lib/launch.mjs:209`).
pub async fn attach_main_page(
    app: serde_json::Value,
    arg1: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let options = arg1;
    let timeout_ms = member_or(&options, "timeoutMs")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let timeout_ms = if is_undef(&timeout_ms) {
        json!(60_000)
    } else {
        timeout_ms
    };
    // deadline = Date.now() + timeoutMs; non-numeric operands make the string
    // concat, which `Date.now() <` then compares as a number.
    let runs_loop = match crate::js::as_number(&timeout_ms) {
        Some(n) => n > 0.0 || n == f64::INFINITY,
        None => {
            let concat = format!(
                "{}{}",
                crate::js::number_to_string(epoch_ms()),
                js_to_string(&timeout_ms)
            );
            concat.parse::<f64>().is_ok_and(|d| epoch_ms() < d)
        }
    };
    let check_error = if app.is_null() {
        "Cannot read properties of null (reading 'exited')".to_string()
    } else if is_undef(&app) {
        "Cannot read properties of undefined (reading 'exited')".to_string()
    } else {
        "app.exited is not a function".to_string()
    };
    let base = format!(
        "Timed out after {} ms waiting for main window target",
        js_to_string(&timeout_ms)
    );
    if runs_loop {
        Err(format!("{base}: {check_error}"))
    } else {
        Err(base)
    }
}

fn epoch_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// Port of `quitDesktop` (`benchmarks/desktop-perf/lib/launch.mjs:233`).
pub async fn quit_desktop(
    app: serde_json::Value,
    arg1: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let options = arg1;
    let _timeout_ms = member_or(&options, "timeoutMs")?;
    let pid = prop(&app, "pid")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let _before = process_tree(pid)?;
    if app.is_null() {
        return Err("Cannot read properties of null (reading 'exited')".into());
    }
    if is_undef(&app) {
        return Err("Cannot read properties of undefined (reading 'exited')".into());
    }
    Err("app.exited is not a function".into())
}

fn spawn_stdout(cmd: &str, args: &[&str]) -> String {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default()
}

fn front_pid() -> Option<f64> {
    let front = spawn_stdout("lsappinfo", &["front"]);
    let front = js_trim(&front);
    if front.is_empty() {
        return None;
    }
    let info = spawn_stdout("lsappinfo", &["info", "-only", "pid", front]);
    let bytes: Vec<char> = info.chars().collect();
    let mut i = 0;
    while i + 3 <= bytes.len() {
        if bytes[i..].starts_with(&['p', 'i', 'd']) {
            let mut j = i + 3;
            while j < bytes.len() && bytes[j].is_whitespace() {
                j += 1;
            }
            if bytes.get(j) == Some(&'=') {
                j += 1;
                while j < bytes.len() && bytes[j].is_whitespace() {
                    j += 1;
                }
                if bytes.get(j) == Some(&'"') {
                    j += 1;
                }
                let start = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j > start {
                    return bytes[start..j].iter().collect::<String>().parse().ok();
                }
                return None;
            }
        }
        i += 1;
    }
    None
}

/// `isFrontmost` shared by the two exports below (`benchmarks/desktop-perf/lib/launch.mjs:264`).
fn is_frontmost_impl(app: &Value) -> Result<Option<bool>, String> {
    let Some(pid) = front_pid() else {
        return Ok(None);
    };
    let app_pid = prop(app, "pid")?;
    if pid.is_finite() && pid > 0.0 {
        return Ok(Some(js_strict_eq(
            &crate::js::number(pid),
            app_pid.unwrap_or(&Value::Null),
        )));
    }
    Ok(None)
}

/// Port of `isFrontmost` (`benchmarks/desktop-perf/lib/launch.mjs:264`).
pub fn is_frontmost(app: serde_json::Value) -> Result<serde_json::Value, String> {
    match is_frontmost_impl(&app)? {
        Some(b) => Ok(Value::Bool(b)),
        None => Ok(Value::Null),
    }
}

/// Port of `secondsSinceUserInput` (`benchmarks/desktop-perf/lib/launch.mjs:273`).
pub fn seconds_since_user_input() -> Result<serde_json::Value, String> {
    let out = spawn_stdout("ioreg", &["-c", "IOHIDSystem", "-d", "4"]);
    let needle = "\"HIDIdleTime\" = ";
    if let Some(pos) = out.find(needle) {
        let digits: String = out[pos + needle.len()..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if let Ok(ns) = digits.parse::<f64>() {
            return Ok(crate::js::number(ns / 1e9));
        }
    }
    Ok(Value::Null)
}

/// Port of `waitForQuietMac` (`benchmarks/desktop-perf/lib/launch.mjs:285`).
pub async fn wait_for_quiet_mac(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    let options = arg0;
    let quiet = member_or(&options, "quietSeconds")?
        .cloned()
        .unwrap_or_else(|| json!(45));
    let max_wait = member_or(&options, "maxWaitSeconds")?
        .cloned()
        .unwrap_or_else(|| json!(1800));
    let log = member_or(&options, "log")?
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let quiet_seconds = crate::js::as_number(&quiet).unwrap_or(45.0);
    let max_wait_seconds = crate::js::as_number(&max_wait).unwrap_or(1800.0);
    let started = std::time::Instant::now();
    let mut announced = false;
    while started.elapsed().as_secs_f64() < max_wait_seconds {
        let idle = seconds_since_user_input()?;
        let idle = crate::js::as_number(&idle);
        if idle.is_none() || idle.is_some_and(|s| s >= quiet_seconds) {
            let mut out = Map::new();
            out.insert(
                "waitedSeconds".into(),
                crate::js::number(crate::js::math_round(started.elapsed().as_secs_f64())),
            );
            out.insert("quiet".into(), Value::Bool(true));
            return Ok(Value::Object(out));
        }
        if !announced {
            if !is_undef(&log) {
                return Err("log is not a function".into());
            }
            announced = true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5000));
    }
    let mut out = Map::new();
    out.insert("waitedSeconds".into(), crate::js::number(max_wait_seconds));
    out.insert("quiet".into(), Value::Bool(false));
    Ok(Value::Object(out))
}

/// Port of `screenLocked` (`benchmarks/desktop-perf/lib/launch.mjs:306`).
pub fn screen_locked() -> Result<serde_json::Value, String> {
    let out = spawn_stdout("ioreg", &["-n", "Root", "-d", "1", "-a"]);
    let locked = {
        let needle = "<key>CGSSessionScreenIsLocked</key>";
        match out.find(needle) {
            Some(pos) => {
                let tail: String = out[pos + needle.len()..]
                    .chars()
                    .skip_while(|c| c.is_whitespace())
                    .collect();
                tail.starts_with("<true/>")
            }
            None => false,
        }
    };
    Ok(Value::Bool(locked))
}

/// Port of `frontmostProblem` (`benchmarks/desktop-perf/lib/launch.mjs:321`).
pub fn frontmost_problem(
    app: serde_json::Value,
    when: serde_json::Value,
) -> Result<serde_json::Value, String> {
    match is_frontmost_impl(&app)? {
        Some(false) => {}
        _ => return Ok(Value::Null),
    }
    let locked = screen_locked()? == Value::Bool(true);
    if locked {
        if std::env::var("BENCH_ALLOW_LOCKED_SCREEN").as_deref() == Ok("1") {
            return Ok(Value::Null);
        }
        return Ok(Value::String(format!(
            "invalid sample: the screen was locked {}",
            js_to_string(&when)
        )));
    }
    Ok(Value::String(format!(
        "invalid sample: the app window was not frontmost {} (another app was in use)",
        js_to_string(&when)
    )))
}

/// Port of `freePort` (`benchmarks/desktop-perf/lib/launch.mjs:52`).
pub async fn free_port() -> Result<serde_json::Value, String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    drop(listener);
    Ok(crate::js::number(port as f64))
}
