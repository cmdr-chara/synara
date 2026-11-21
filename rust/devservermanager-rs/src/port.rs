//! The Rust port of `apps/server/src/devServerManager.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\u{0009}'..='\u{000D}'
            | '\u{0020}'
            | '\u{00A0}'
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

fn js_truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
        Some(Value::Object(m)) if crate::js::is_undefined(&Value::Object(m.clone())) => false,
        Some(v) => crate::js::as_number(v)
            .map(|x| x != 0.0 && !x.is_nan())
            .unwrap_or(true),
    }
}

/// JS `===` on wire values (objects compare by identity → never equal here).
fn strict_eq(a: Option<&Value>, b: Option<&Value>) -> bool {
    let undef = |v: Option<&Value>| match v {
        None => true,
        Some(Value::Object(m)) => m.get("$undefined") == Some(&Value::Bool(true)),
        _ => false,
    };
    if undef(a) && undef(b) {
        return true;
    }
    if undef(a) || undef(b) {
        return false;
    }
    let (a, b) = (a.unwrap(), b.unwrap());
    if a.is_null() && b.is_null() {
        return true;
    }
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

/// Property read on a wire value: null/undefined throw, primitives give undefined.
fn prop<'a>(v: &'a Value, key: &str, ctx: &str) -> Result<Option<&'a Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{ctx}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!("undefined is not an object (evaluating '{ctx}')"));
    }
    Ok(v.get(key))
}

/// `String.prototype.trim()` on a JS value.
fn trim_str(v: Option<&Value>) -> Result<String, String> {
    match v {
        Some(Value::String(s)) => Ok(s.trim_matches(is_js_ws).to_string()),
        _ => Err("value.trim is not a function".into()),
    }
}

/// `normalizeWorkspaceRootForComparison` with no options.
fn normalize_root(value: &str) -> String {
    let trimmed = value.trim_matches(is_js_ws);
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd: String = trimmed.replace('\\', "/");
    let bytes = fwd.as_bytes();
    let is_windows_drive_root = bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && fwd[2..].chars().all(|c| c == '/');
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = String::with_capacity(fwd.len());
    let mut last_slash = false;
    for c in fwd[prefix.len()..].chars() {
        if c == '/' {
            if !last_slash {
                body.push(c);
            }
            last_slash = true;
        } else {
            body.push(c);
            last_slash = false;
        }
    }
    while body.ends_with('/') {
        body.pop();
    }
    let normalized = if body.is_empty() {
        prefix.to_string()
    } else {
        format!("{prefix}{body}")
    };
    let final_value = if is_windows_drive_root {
        format!("{}/", &fwd[..2])
    } else {
        normalized
    };
    let likely_windows = {
        let b = trimmed.as_bytes();
        (b.len() >= 2
            && b[0].is_ascii_alphabetic()
            && b[1] == b':'
            && (b.len() == 2 || b[2] == b'/' || b[2] == b'\\'))
            || trimmed.starts_with("\\\\")
            || trimmed.starts_with("//")
    };
    if likely_windows {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

/// `isWorkspaceRootWithin(candidate, ancestor)`.
fn root_within(candidate: Option<&Value>, ancestor: Option<&Value>) -> Result<bool, String> {
    let c = normalize_root(&trim_str(candidate)?);
    let a = normalize_root(&trim_str(ancestor)?);
    if c.is_empty() || a.is_empty() {
        return Ok(false);
    }
    if c == a {
        return Ok(true);
    }
    let prefix = if a.ends_with('/') { a } else { format!("{a}/") };
    Ok(c.starts_with(&prefix))
}

/// `localServerMatchesRun(server, run)` from `@synara/shared/localServers`.
fn local_server_matches_run(server: Option<&Value>, run: &Value) -> Result<bool, String> {
    let run_pid = prop(run, "pid", "run.pid")?;
    // `run.pid !== null` — undefined !== null is true, so the branch almost always runs.
    if !matches!(run_pid, Some(Value::Null)) {
        let server_pid = match server {
            Some(s) => prop(s, "pid", "server.pid")?,
            None => return Err("undefined is not an object (evaluating 'server.pid')".into()),
        };
        if strict_eq(server_pid, run_pid) {
            return Ok(true);
        }
        let server_ppid = prop(server.unwrap(), "ppid", "server.ppid")?;
        if strict_eq(server_ppid, run_pid) {
            return Ok(true);
        }
    }
    let server_cwd = match server {
        Some(s) => prop(s, "cwd", "server.cwd")?,
        None => return Err("undefined is not an object (evaluating 'server.cwd')".into()),
    };
    if !js_truthy(server_cwd) {
        return Ok(false);
    }
    let run_cwd = prop(run, "cwd", "run.cwd")?;
    root_within(server_cwd, run_cwd)
}

/// `input.key` where `input` is the first positional argument.
fn arg_get<'a>(input: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    if input.is_null() {
        return Err(format!("null is not an object (evaluating 'input.{key}')"));
    }
    if crate::js::is_undefined(input) {
        return Err(format!(
            "undefined is not an object (evaluating 'input.{key}')"
        ));
    }
    Ok(input.get(key))
}

/// Port of `findProjectDevServerForLocalServer` (`apps/server/src/devServerManager.ts:47`).
pub fn find_project_dev_server_for_local_server(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let dev_servers = arg_get(&input, "devServers")?;
    // `for (const devServer of input.devServers)` — non-iterable values throw.
    let items: Vec<Value> = match dev_servers {
        Some(Value::Array(items)) => items.clone(),
        Some(Value::String(s)) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => {
            return Err(
                "undefined is not an object (evaluating 'devServer of input.devServers')".into(),
            );
        }
    };
    for dev_server in &items {
        let local_server = arg_get(&input, "localServer")?;
        if local_server_matches_run(local_server, dev_server)? {
            return Ok(Some(dev_server.clone()));
        }
    }
    Ok(None)
}
