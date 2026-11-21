use serde_json::{Value, json};

use crate::js;

fn is_undefined(v: &Value) -> bool {
    js::is_undefined(v)
}

fn read_member<'a>(obj: &'a Value, key: &str, what: &str) -> Result<Option<&'a Value>, String> {
    match obj {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key).filter(|v| !is_undefined(v))),
        _ => Err(format!(
            "Cannot read properties of undefined (reading '{what}')"
        )),
    }
}

fn field<'a>(obj: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    match obj {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key)),
        _ => Ok(None),
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(_) => js::as_number(v)
            .map(js::number_to_string)
            .unwrap_or_default(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_to_string).collect::<Vec<_>>().join(","),
        Value::Object(o) => {
            if let Some(u) = o.get("$utf16").and_then(|x| x.as_array()) {
                let units: Vec<u16> = u.iter().map(|x| x.as_u64().unwrap_or(0) as u16).collect();
                String::from_utf16_lossy(&units)
            } else if o.get("$num").is_some() {
                js::as_number(v)
                    .map(js::number_to_string)
                    .unwrap_or_default()
            } else if let Some(b) = o.get("$bigint").and_then(|x| x.as_str()) {
                b.to_string()
            } else if o.get("$undefined").is_some() {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn is_likely_windows_root(value: &str) -> bool {
    let b = value.as_bytes();
    if b.len() >= 2
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b.len() == 2 || b[2] == b'\\' || b[2] == b'/')
    {
        return true;
    }
    value.starts_with("\\\\") || value.starts_with("//")
}

/// `normalizeWorkspaceRootForComparison(value)` (no options) — `value` must be a string.
fn normalize_root(value: &Value) -> Result<String, String> {
    let raw = match value {
        Value::String(s) => s.clone(),
        _ => return Err("value.trim is not a function".to_string()),
    };
    let trimmed = raw.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let fwd = trimmed.replace('\\', "/");
    let bytes = fwd.as_bytes();
    let is_win_drive_root = bytes.len() >= 3
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
    let body = body.trim_end_matches('/');
    let normalized = if prefix.is_empty() {
        body.to_string()
    } else {
        format!("{prefix}{body}")
    };
    let final_value = if is_win_drive_root {
        format!("{}/", &fwd[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };
    Ok(if is_likely_windows_root(trimmed) {
        final_value.to_lowercase()
    } else {
        final_value
    })
}

/// `isWorkspaceRootWithin(candidate, ancestorRoot)` (no options).
fn root_within(candidate: &Value, ancestor: &Value) -> Result<bool, String> {
    let nc = normalize_root(candidate)?;
    let na = normalize_root(ancestor)?;
    if nc.is_empty() || na.is_empty() {
        return Ok(false);
    }
    if nc == na {
        return Ok(true);
    }
    let prefix = if na.ends_with('/') {
        na
    } else {
        format!("{na}/")
    };
    Ok(nc.starts_with(&prefix))
}

fn first_address_port(server: &Value) -> Result<Vec<Value>, String> {
    let addresses = field(server, "addresses")?;
    let items: &[Value] = match addresses {
        Some(Value::Array(a)) => a,
        Some(Value::Null) | None => &[],
        _ => return Err("server.addresses is not iterable".to_string()),
    };
    for address in items {
        let port = match address {
            Value::Object(m) => m.get("port"),
            _ => None,
        };
        if let Some(p) = port
            && js::as_number(p).is_some_and(|n| n > 0.0)
        {
            return Ok(vec![p.clone()]);
        }
    }
    Ok(vec![])
}

/// Port of `localServerAddressLabel` (`packages/shared/src/localServers.ts:23`).
pub fn local_server_address_label(server: serde_json::Value) -> Result<String, String> {
    let ports_v = read_member(&server, "ports", "length")?;
    let ports: Vec<Value> = match ports_v {
        Some(Value::Array(a)) if !a.is_empty() => a.clone(),
        Some(Value::Array(_)) => first_address_port(&server)?,
        Some(Value::String(s)) if !s.is_empty() => {
            return Err("ports.map is not a function".to_string());
        }
        Some(_) => first_address_port(&server)?,
        None => {
            return Err("Cannot read properties of undefined (reading 'length')".to_string());
        }
    };
    if ports.is_empty() {
        return Ok("localhost".to_string());
    }
    Ok(ports
        .iter()
        .map(|p| format!("localhost:{}", js_to_string(p)))
        .collect::<Vec<_>>()
        .join(", "))
}

/// Port of `localServerPrimaryLabel` (`packages/shared/src/localServers.ts:35`).
pub fn local_server_primary_label(server: serde_json::Value) -> Result<serde_json::Value, String> {
    let page_title = field(&server, "pageTitle")?;
    if let Some(v) = page_title
        && !matches!(v, Value::Null)
        && !is_undefined(v)
    {
        return Ok(v.clone());
    }
    Ok(field(&server, "displayName")?
        .cloned()
        .unwrap_or_else(js::undefined))
}

/// Port of `localServerFolderLabel` (`packages/shared/src/localServers.ts:45`).
pub fn local_server_folder_label(
    server: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let cwd_v = field(&server, "cwd")?;
    let cwd = match cwd_v {
        Some(Value::String(s)) => s.trim_matches(is_js_space).to_string(),
        Some(Value::Null) | None => return Ok(None),
        Some(v) if is_undefined(v) => return Ok(None),
        Some(_) => return Err("server.cwd?.trim is not a function".to_string()),
    };
    if cwd.is_empty() {
        return Ok(None);
    }
    let last = cwd.split(['/', '\\']).rfind(|s| !s.is_empty());
    Ok(last.map(|s| json!(s)))
}

/// Port of `localServerMatchesRun` (`packages/shared/src/localServers.ts:57`).
pub fn local_server_matches_run(
    server: serde_json::Value,
    run: serde_json::Value,
) -> Result<bool, String> {
    let run_pid = field(&run, "pid")?;
    if !matches!(run_pid, Some(Value::Null)) {
        let same = |key: &str| -> Result<bool, String> {
            let sp = field(&server, key)?;
            Ok(match (sp, run_pid) {
                (Some(a), Some(b)) => strict_eq(a, b),
                (None, None) => true,
                (None, Some(b)) => is_undefined(b),
                (Some(a), None) => is_undefined(a),
            })
        };
        if same("pid")? || same("ppid")? {
            return Ok(true);
        }
    }
    let run_cwd = field(&run, "cwd")?.cloned().unwrap_or(Value::Null);
    match field(&server, "cwd")? {
        Some(v) if truthy_for(v) => root_within(v, &run_cwd),
        _ => Ok(false),
    }
}

/// `a === b` for the values this unit can see (plus `$num`/`$utf16` markers).
fn strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => match (js::as_number(a), js::as_number(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

fn truthy_for(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Object(o) if o.get("$num").is_some() => js::as_number(v).is_some_and(|x| x != 0.0),
        Value::Object(o) if o.get("$undefined").is_some() => false,
        _ => true,
    }
}
