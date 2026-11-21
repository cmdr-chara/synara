use crate::js;
use serde_json::{Map, Value, json};
use std::sync::LazyLock;

static PATH_LINK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?:~/|\.\.?/)[^\s]*|[A-Za-z0-9._~-]+(?:/[A-Za-z0-9._~-]+)+|/[^\s]*")
        .expect("terminal link pattern")
});
static POSITION_SUFFIX_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(.*):(\d+)(?::(\d+))?$").expect("position suffix"));
static WINDOWS_ABS_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(?:[A-Za-z]:[\\/]|\\\\)").expect("windows absolute"));
static POSIX_HOME_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^(/(?:Users|home)/[^/]+)").expect("posix home"));
static WINDOWS_HOME_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    regex::Regex::new(r"(?i)^([a-zA-Z]:[\\/]+Users[\\/]+[^\\/]+)").expect("windows home")
});

fn prop(value: &Value, key: &str) -> Value {
    value.get(key).cloned().unwrap_or_else(js::undefined)
}

fn prop_or_err(value: &Value, key: &str) -> Result<Value, String> {
    if value.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if js::is_undefined(value) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(prop(value, key))
}

fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !js::is_undefined(value),
    }
}

fn as_f64(value: &Value) -> Option<f64> {
    js::as_number(value)
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn utf16_offset(text: &str, byte_idx: usize) -> usize {
    text[..byte_idx].encode_utf16().count()
}

/// Port of `extractTerminalLinks` (`apps/web/src/terminal-links.ts:170`).
pub fn extract_terminal_links(line: String) -> Result<Vec<serde_json::Value>, String> {
    let mut links = Vec::new();
    for m in PATH_LINK_RE.find_iter(&line) {
        let mut entry = Map::new();
        entry.insert("kind".to_string(), json!("path"));
        entry.insert("text".to_string(), json!(m.as_str()));
        entry.insert(
            "start".to_string(),
            js::number(utf16_offset(&line, m.start()) as f64),
        );
        entry.insert(
            "end".to_string(),
            js::number(utf16_offset(&line, m.end()) as f64),
        );
        links.push(Value::Object(entry));
    }
    Ok(links)
}

fn locate_point(segments: &[Value], offset: f64) -> Result<(f64, f64), String> {
    let mut last_text_len = 0f64;
    let mut last_line = 1f64;
    for segment in segments {
        let start_index = as_f64(&prop_or_err(segment, "startIndex")?).unwrap_or(f64::NAN);
        let end_index = as_f64(&prop_or_err(segment, "endIndex")?).unwrap_or(f64::NAN);
        let buffer_line = as_f64(&prop_or_err(segment, "bufferLineNumber")?).unwrap_or(1.0);
        let text = prop(segment, "text");
        let text_len = text.as_str().map_or(0, utf16_len) as f64;
        if start_index <= offset && offset < end_index {
            return Ok((offset - start_index + 1.0, buffer_line));
        }
        last_text_len = text_len;
        last_line = buffer_line;
    }
    Ok((last_text_len.max(1.0), last_line))
}

/// Port of `resolveWrappedTerminalLinkRange` (`apps/web/src/terminal-links.ts:243`).
pub fn resolve_wrapped_terminal_link_range(
    wrapped_line: serde_json::Value,
    r#match: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let start_offset = as_f64(&prop_or_err(&r#match, "start")?).unwrap_or(f64::NAN);
    let end_offset = as_f64(&prop_or_err(&r#match, "end")?).unwrap_or(f64::NAN);
    let segments = prop_or_err(&wrapped_line, "segments")?;
    let segments = match &segments {
        Value::Array(items) => items.clone(),
        _ => return Err("segments is not iterable".to_string()),
    };
    let (start_x, start_y) = locate_point(&segments, start_offset)?;
    let (end_x, end_y) = locate_point(&segments, end_offset)?;
    Ok(json!({
        "start": { "x": js::number(start_x), "y": js::number(start_y) },
        "end": { "x": js::number(end_x), "y": js::number(end_y) },
    }))
}

fn js_less_equal(a: &Value, b: f64) -> bool {
    let n = match js::as_number(a) {
        Some(n) => n,
        None => {
            if a.is_null() {
                0.0
            } else if let Value::Bool(v) = a {
                if *v { 1.0 } else { 0.0 }
            } else if let Value::String(s) = a {
                s.trim().parse::<f64>().unwrap_or(f64::NAN)
            } else {
                return false;
            }
        }
    };
    n <= b
}

/// Port of `wrappedTerminalLinkRangeIntersectsBufferLine` (`apps/web/src/terminal-links.ts:253`).
pub fn wrapped_terminal_link_range_intersects_buffer_line(
    range: serde_json::Value,
    buffer_line_number: f64,
) -> Result<bool, String> {
    let start = prop_or_err(&range, "start")?;
    let start_y = prop_or_err(&start, "y")?;
    if !js_less_equal(&start_y, buffer_line_number) {
        return Ok(false);
    }
    let end = prop_or_err(&range, "end")?;
    let end_y = prop_or_err(&end, "y")?;
    Ok(buffer_line_number <= as_f64(&end_y).unwrap_or(f64::NAN))
}

/// Port of `isTerminalLinkActivation` (`apps/web/src/terminal-links.ts:260`).
pub fn is_terminal_link_activation(
    event: serde_json::Value,
    platform: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let platform = if js::is_undefined(&platform) {
        json!("MacIntel")
    } else {
        platform
    };
    let length = match &platform {
        Value::String(s) => utf16_len(s) as f64,
        Value::Array(a) => a.len() as f64,
        Value::Null => {
            return Err("Cannot read properties of null (reading 'length')".to_string());
        }
        _ => f64::NAN,
    };
    if length == 0.0 {
        return Ok(json!(false));
    }
    let is_mac = platform
        .as_str()
        .is_some_and(|s| s.to_lowercase().contains("mac"));
    let (first_key, second_key) = if is_mac {
        ("metaKey", "ctrlKey")
    } else {
        ("ctrlKey", "metaKey")
    };
    let first = prop_or_err(&event, first_key)?;
    if !truthy(&first) {
        return Ok(first);
    }
    let second = prop_or_err(&event, second_key)?;
    Ok(json!(!truthy(&second)))
}

fn is_windows_path_style(path: &str) -> bool {
    path.contains('\\')
}

fn is_absolute_path(path: &str) -> bool {
    path.starts_with('/') || WINDOWS_ABS_RE.is_match(path)
}

fn join_path(base: &str, rel: &str, separator: char) -> String {
    let trimmed = base.trim_end_matches(['/', '\\']);
    format!("{trimmed}{separator}{rel}")
}

fn infer_home_from_cwd(cwd: &str) -> Option<String> {
    if let Some(caps) = POSIX_HOME_RE.captures(cwd) {
        return Some(caps[1].to_string());
    }
    if let Some(caps) = WINDOWS_HOME_RE.captures(cwd) {
        return Some(caps[1].to_string());
    }
    None
}

/// Port of `resolvePathLinkTarget` (`apps/web/src/terminal-links.ts:270`).
pub fn resolve_path_link_target(raw_path: String, cwd: String) -> Result<String, String> {
    let mut path = raw_path.as_str();
    let mut position = String::new();
    if let Some(caps) = POSITION_SUFFIX_RE.captures(path) {
        path = caps.get(1).map_or(path, |m| m.as_str());
        position = match caps.get(3) {
            Some(col) => format!(":{}:{}", &caps[2], col.as_str()),
            None => format!(":{}", &caps[2]),
        };
    }
    let home = infer_home_from_cwd(&cwd);
    let resolved = if let Some(rest) = path.strip_prefix("~/") {
        match &home {
            Some(h) => {
                let sep = if is_windows_path_style(h) { '\\' } else { '/' };
                join_path(h, rest, sep)
            }
            None => path.to_string(),
        }
    } else if is_absolute_path(path) {
        path.to_string()
    } else {
        let sep = if is_windows_path_style(&cwd) {
            '\\'
        } else {
            '/'
        };
        join_path(&cwd, path, sep)
    };
    Ok(format!("{resolved}{position}"))
}
