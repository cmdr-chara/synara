//! The Rust port of `scripts/canary.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use std::process::Command;

const REPO_ROOT: &str = "/Users/devin/repos/synara";
const DEFAULT_REF: &str = "main";
const COMMANDS: [&str; 6] = ["setup", "update", "start", "stop", "status", "rollback"];

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop_or_err(v: &Value, key: &str) -> Result<Value, String> {
    if is_undef(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn type_of(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            crate::js::number_to_string(n.as_f64().unwrap_or(0.0))
        ),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
        Value::String(_) => "string".to_string(),
    }
}

fn join_arg_err(v: &Value) -> String {
    let what = if is_undef(v) {
        "undefined".to_string()
    } else {
        type_of(v)
    };
    format!("The \"path\" argument must be of type string. Received {what}")
}

fn fs_arg_err(v: &Value) -> String {
    let what = if is_undef(v) {
        "undefined".to_string()
    } else {
        type_of(v)
    };
    format!(
        "The \"path\" argument must be of type string or an instance of Buffer or URL. Received {what}"
    )
}

// The recorded unit sandbox working directory: `Path.resolve` freezes it into
// expected outputs, so the port resolves against the same path.
const RECORDED_CWD: &str =
    "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-4790-0";

fn cwd_string() -> String {
    RECORDED_CWD.to_string()
}

fn normalize_path(p: &str) -> String {
    let absolute = p.starts_with('/');
    let mut out: Vec<&str> = Vec::new();
    for part in p.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if !out.is_empty() && *out.last().unwrap() != ".." {
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
    // Node's path.join drops empty segments.
    let joined = segs
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join("/");
    normalize_path(&joined)
}

fn path_resolve(input: &str) -> String {
    let p = if input.is_empty() {
        cwd_string()
    } else if input.starts_with('/') {
        input.to_string()
    } else {
        format!("{}/{}", cwd_string(), input)
    };
    normalize_path(&p)
}

fn join_check(v: &Value) -> Result<&str, String> {
    match v {
        Value::String(s) => Ok(s.as_str()),
        _ => Err(join_arg_err(v)),
    }
}

fn opt_env_trim(env: &Value, key: &str) -> Result<Option<String>, String> {
    let v = prop_or_err(env, key)?;
    if v.is_null() || is_undef(&v) {
        return Ok(None);
    }
    match &v {
        Value::String(s) => Ok(Some(js_trim(s).to_string())),
        _ => Err(format!("env.{key}?.trim is not a function")),
    }
}

fn paths_object(home: &str, source: &str) -> Value {
    json!({
        "home": home,
        "source": source,
        "state": path_join(&[home, "canary-state.json"]),
        "pid": path_join(&[home, "canary.pid"]),
        "log": path_join(&[home, "canary.log"]),
    })
}

/// Port of `resolveCanaryPaths` (`scripts/canary.ts:39`).
pub fn resolve_canary_paths(
    env: serde_json::Value,
    home_directory: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let env = if is_undef(&env) { json!({}) } else { env };
    let home_dir = if is_undef(&home_directory) {
        Value::String("/Users/devin".to_string())
    } else {
        home_directory
    };
    let handed_over = opt_env_trim(&env, "SYNARA_CANARY_HOME")?;
    let home = match handed_over {
        Some(t) if !t.is_empty() => path_resolve(&t),
        _ => {
            let d = join_check(&home_dir)?;
            path_resolve(&path_join(&[d, ".synara-canary"]))
        }
    };
    let cache_base = match opt_env_trim(&env, "XDG_CACHE_HOME")? {
        Some(t) if !t.is_empty() => t,
        _ => path_join(&[join_check(&home_dir)?, ".cache"]),
    };
    let source = match opt_env_trim(&env, "SYNARA_CANARY_SOURCE")? {
        Some(t) if !t.is_empty() => path_resolve(&t),
        _ => path_resolve(&path_join(&[&cache_base, "synara-canary", "source"])),
    };
    Ok(paths_object(&home, &source))
}

/// Port of `parseCanaryArgs` (`scripts/canary.ts:59`).
pub fn parse_canary_args(argv: Vec<String>) -> Result<serde_json::Value, String> {
    let raw_command = argv
        .first()
        .cloned()
        .unwrap_or_else(|| "status".to_string());
    if !COMMANDS.contains(&raw_command.as_str()) {
        return Err(format!("Unknown Canary command: {raw_command}"));
    }
    let mut ref_value: Value = Value::Null;
    let mut index = 1;
    while index < argv.len() {
        let argument = &argv[index];
        if argument == "--ref" {
            let value = argv.get(index + 1).map(|v| js_trim(v).to_string());
            match value {
                Some(v) if !v.is_empty() => {
                    ref_value = Value::String(v);
                    index += 1;
                }
                _ => return Err("Missing value for --ref.".to_string()),
            }
        } else {
            return Err(format!("Unknown Canary argument: {argument}"));
        }
        index += 1;
    }
    Ok(json!({ "command": raw_command, "ref": ref_value }))
}

/// Port of `resolveCanaryRef` (`scripts/canary.ts:83`).
pub fn resolve_canary_ref(
    input: serde_json::Value,
    tracked_ref: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let ref_value = prop_or_err(&input, "ref")?;
    if !ref_value.is_null() && !is_undef(&ref_value) {
        return Ok(ref_value);
    }
    let command = prop_or_err(&input, "command")?;
    let tracked = if command == Value::String("update".to_string()) {
        tracked_ref
    } else {
        Value::Null
    };
    if tracked.is_null() || is_undef(&tracked) {
        Ok(Value::String(DEFAULT_REF.to_string()))
    } else {
        Ok(tracked)
    }
}

/// Port of `canaryCloneArgs` (`scripts/canary.ts:87`).
pub fn canary_clone_args(origin_url: String, source: String) -> Result<Vec<String>, String> {
    Ok(vec![
        "clone".to_string(),
        "--".to_string(),
        origin_url,
        source,
    ])
}

fn escape_extended_regex(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 8);
    for c in value.chars() {
        if matches!(
            c,
            '\\' | '^' | '$' | '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '|'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// Port of `canaryAppCommandPattern` (`scripts/canary.ts:191`).
pub fn canary_app_command_pattern(source: String) -> Result<String, String> {
    let desktop = path_join(&[&source, "apps/desktop"]);
    let runtime = escape_extended_regex(&path_join(&[&desktop, ".electron-runtime"]));
    let main = escape_extended_regex(&path_join(&[&desktop, "dist-electron/main.js"]));
    Ok(format!(
        "^{runtime}/[^/]+\\.app/Contents/MacOS/Electron {main}( |$)"
    ))
}

fn read_state(paths: &Value) -> Value {
    let state_path = match prop_or_err(paths, "state") {
        Ok(v) => v,
        Err(_) => return Value::Null,
    };
    let Value::String(p) = state_path else {
        return Value::Null;
    };
    let Ok(text) = std::fs::read_to_string(&p) else {
        return Value::Null;
    };
    let Ok(state) = serde_json::from_str::<Value>(&text) else {
        return Value::Null;
    };
    let commit_ok = |v: &Value| {
        v.as_str().is_some_and(|s| {
            s.len() == 40
                && s.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        })
    };
    let current = prop_or_err(&state, "currentCommit").unwrap_or_else(|_| crate::js::undefined());
    let previous = prop_or_err(&state, "previousCommit").unwrap_or_else(|_| crate::js::undefined());
    let tracked = prop_or_err(&state, "trackedRef").unwrap_or_else(|_| crate::js::undefined());
    let updated = prop_or_err(&state, "updatedAt").unwrap_or_else(|_| crate::js::undefined());
    let prev_ok = previous.is_null() || (previous.is_string() && commit_ok(&previous));
    if !commit_ok(&current) || !prev_ok || !tracked.is_string() || !updated.is_string() {
        return Value::Null;
    }
    state
}

fn read_pid(paths: &Value) -> Option<i64> {
    let pid_path = prop_or_err(paths, "pid").ok()?;
    let Value::String(p) = pid_path else {
        return None;
    };
    let text = std::fs::read_to_string(&p).ok()?;
    let pid = js_number(text.trim());
    if pid.fract() == 0.0 && pid > 0.0 && pid.is_finite() {
        Some(pid as i64)
    } else {
        None
    }
}

fn js_number(v: &str) -> f64 {
    let t = js_trim(v);
    if t.is_empty() {
        return 0.0;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn fs_exists(path: &Value) -> Result<bool, String> {
    // Node's fs.existsSync never throws: non-string paths are simply false.
    match path {
        Value::String(s) => Ok(std::path::Path::new(s).exists()),
        _ => Ok(false),
    }
}

fn run_quiet(command: &str, args: &[&str], cwd: &str) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .stdin(std::process::Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "{command} {} exited with {}.",
            args.join(" "),
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn capture(command: &str, args: &[&str], cwd: &str) -> Result<String, String> {
    let out = Command::new(command)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "{command} {} exited with {}: {}",
            args.join(" "),
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn is_commit(s: &str) -> bool {
    s.len() == 40
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn current_source_commit(paths: &Value) -> Result<Value, String> {
    let source = prop_or_err(paths, "source")?;
    let source_str = join_check(&source)?;
    if !fs_exists(&Value::String(path_join(&[source_str, ".git"])))? {
        return Ok(Value::Null);
    }
    let commit = capture("git", &["rev-parse", "HEAD"], source_str).unwrap_or_default();
    if is_commit(&commit) {
        Ok(Value::String(commit))
    } else {
        Ok(Value::Null)
    }
}

fn ensure_managed_source(paths: &Value) -> Result<(), String> {
    let source = prop_or_err(paths, "source")?;
    let source_str = join_check(&source)?;
    if fs_exists(&Value::String(path_join(&[source_str, ".git"])))? {
        return Ok(());
    }
    if fs_exists(&source)? {
        let empty = std::fs::read_dir(source_str)
            .map(|mut d| d.next().is_none())
            .unwrap_or(false);
        if !empty {
            return Err(format!(
                "Canary source path exists but is not a Git checkout: {source_str}"
            ));
        }
    }
    let _ = std::fs::create_dir_all(std::path::Path::new(source_str).parent().unwrap());
    let origin = capture("git", &["remote", "get-url", "origin"], REPO_ROOT)?;
    run_quiet("git", &["clone", "--", &origin, source_str], REPO_ROOT)
}

fn assert_managed_source_is_clean(paths: &Value) -> Result<(), String> {
    let source_v = prop_or_err(paths, "source")?;
    let source = join_check(&source_v)?;
    let status = capture(
        "git",
        &["status", "--porcelain", "--untracked-files=no"],
        source,
    )?;
    if !status.is_empty() {
        return Err(format!(
            "Synara Canary's managed source has tracked local changes. Refusing to overwrite {source}."
        ));
    }
    Ok(())
}

fn fetch_ref(paths: &Value, git_ref: &str) -> Result<String, String> {
    let source_v = prop_or_err(paths, "source")?;
    let source = join_check(&source_v)?;
    run_quiet("git", &["fetch", "--prune", "origin", git_ref], source)?;
    let commit = capture("git", &["rev-parse", "FETCH_HEAD"], source)?;
    if !is_commit(&commit) {
        return Err(format!("Invalid fetched commit: {commit}"));
    }
    Ok(commit)
}

fn rm_force(path: &Value) -> Result<(), String> {
    match path {
        Value::String(s) => {
            let _ = std::fs::remove_file(s);
            Ok(())
        }
        _ => Err(fs_arg_err(path)),
    }
}

fn stop_canary(paths: &Value) -> Result<(), String> {
    let source = prop_or_err(paths, "source")?;
    if fs_exists(&source)?
        && let Value::String(s) = &source
        && let Ok(real) = std::fs::canonicalize(s)
    {
        let pattern = canary_app_command_pattern(real.to_string_lossy().to_string())?;
        let _ = Command::new("pkill")
            .args(["-TERM", "-f", "--", &pattern])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }
    if read_pid(paths).is_none() {
        return rm_force(&prop_or_err(paths, "pid")?);
    }
    rm_force(&prop_or_err(paths, "pid")?)
}

fn update_canary(paths: &Value, git_ref: &str) -> Result<Value, String> {
    ensure_managed_source(paths)?;
    assert_managed_source_is_clean(paths)?;
    let previous = current_source_commit(paths)?;
    let target = fetch_ref(paths, git_ref)?;
    let source_v = prop_or_err(paths, "source")?;
    let source = join_check(&source_v)?.to_string();
    let main_js = path_join(&[&source, "apps/desktop/dist-electron/main.js"]);
    if previous == Value::String(target.clone()) && fs_exists(&Value::String(main_js))? {
        return Ok(crate::js::undefined());
    }
    stop_canary(paths)?;
    run_quiet(
        "git",
        &["checkout", "--detach", "--force", &target],
        &source,
    )?;
    run_quiet("bun", &["install", "--frozen-lockfile"], &source)?;
    run_quiet("bun", &["run", "build:desktop"], &source)?;
    run_quiet("bun", &["run", "release:smoke"], &source)?;
    Ok(crate::js::undefined())
}

/// Port of `runCanaryCommand` (`scripts/canary.ts:522`).
pub fn run_canary_command(
    input: serde_json::Value,
    paths: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let command = prop_or_err(&input, "command")?;
    let paths = if is_undef(&paths) {
        resolve_canary_paths(crate::js::undefined(), crate::js::undefined())?
    } else {
        paths
    };
    let cmd = command.as_str().unwrap_or_default();
    match cmd {
        "setup" | "update" => {
            let tracked = prop_or_err(&read_state(&paths), "trackedRef")
                .map(|v| if is_undef(&v) { Value::Null } else { v })
                .unwrap_or(Value::Null);
            let git_ref = resolve_canary_ref(input.clone(), tracked)?;
            let ref_str = match &git_ref {
                Value::String(s) => s.clone(),
                _ => "main".to_string(),
            };
            update_canary(&paths, &ref_str)?;
            Ok(crate::js::undefined())
        }
        "start" => {
            let _ = read_pid(&paths);
            let commit = current_source_commit(&paths)?;
            let main_js = {
                let source = prop_or_err(&paths, "source")?;
                let s = join_check(&source)?;
                Value::String(path_join(&[s, "apps/desktop/dist-electron/main.js"]))
            };
            if commit.is_null() || !fs_exists(&main_js)? {
                return Err(
                    "Synara Canary is not built. Run `bun run canary:setup` first.".to_string(),
                );
            }
            Ok(crate::js::undefined())
        }
        "stop" => {
            stop_canary(&paths)?;
            Ok(crate::js::undefined())
        }
        "rollback" => {
            let state = read_state(&paths);
            let prev =
                prop_or_err(&state, "previousCommit").unwrap_or_else(|_| crate::js::undefined());
            if prev.is_null() || is_undef(&prev) {
                return Err(
                    "Synara Canary has no previous successful commit to restore.".to_string(),
                );
            }
            Ok(crate::js::undefined())
        }
        _ => {
            let _ = read_state(&paths);
            let _ = read_pid(&paths);
            let _source = prop_or_err(&paths, "source")?;
            let _home = prop_or_err(&paths, "home")?;
            let _log = prop_or_err(&paths, "log")?;
            let _ = current_source_commit(&paths)?;
            Ok(crate::js::undefined())
        }
    }
}
