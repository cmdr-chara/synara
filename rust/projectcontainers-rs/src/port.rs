use serde_json::Value;

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

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `obj.key` where `obj` may be a non-object (undefined member) and null throws.
fn field<'a>(obj: &'a Value, key: &str) -> Result<Option<&'a Value>, String> {
    match obj {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => Ok(m.get(key)),
        _ => Ok(None),
    }
}

fn platform_of(options: Option<&Value>) -> Result<Option<String>, String> {
    match options {
        Some(Value::Object(m)) => Ok(m
            .get("platform")
            .and_then(|v| v.as_str())
            .map(str::to_string)),
        Some(Value::Null) | None => Ok(None),
        Some(_) => Ok(None),
    }
}

/// `value.trim()` with JS member-access errors for null/undefined/non-strings.
fn js_trim_value(value: &Value, who: &str) -> Result<String, String> {
    match value {
        Value::Null => Err("Cannot read properties of null (reading 'trim')".to_string()),
        Value::String(s) => Ok(s.trim_matches(is_js_space).to_string()),
        v if is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'trim')".to_string())
        }
        _ => Err(format!("{who}.trim is not a function")),
    }
}

/// `paths.chatWorkspaceRoot?.trim() || paths.homeDir?.trim() || null`
fn resolve_root(paths: &Value) -> Result<Option<String>, String> {
    let chat = field(paths, "chatWorkspaceRoot")?;
    if let Some(v) = chat {
        let t = match v {
            Value::Null => None,
            _ if is_undefined(v) => None,
            _ => Some(js_trim_value(v, "paths.chatWorkspaceRoot?")?),
        };
        if let Some(t) = t
            && !t.is_empty()
        {
            return Ok(Some(t));
        }
    }
    let home = field(paths, "homeDir")?;
    if let Some(v) = home {
        let t = match v {
            Value::Null => None,
            _ if is_undefined(v) => None,
            _ => Some(js_trim_value(v, "paths.homeDir?")?),
        };
        if let Some(t) = t
            && !t.is_empty()
        {
            return Ok(Some(t));
        }
    }
    Ok(None)
}

fn is_likely_windows_root(value: &str, platform: Option<&str>) -> bool {
    match platform {
        Some("win32") => return true,
        Some(_) => return false,
        None => {}
    }
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

/// `normalizeWorkspaceRootForComparison(value, options)`.
fn normalize_root(value: &str, platform: Option<&str>) -> String {
    let trimmed = value.trim_matches(is_js_space);
    if trimmed.is_empty() {
        return String::new();
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
    let mut final_value = if is_win_drive_root {
        format!("{}/", &fwd[..2])
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix.to_string()
    };
    if platform == Some("darwin")
        && (final_value.starts_with("/private/var/") || final_value.starts_with("/private/tmp/"))
    {
        final_value = final_value["/private".len()..].to_string();
    }
    if is_likely_windows_root(trimmed, platform) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}

/// Port of `resolveChatContainerWorkspaceRoot` (`packages/shared/src/projectContainers.ts:22`).
pub fn resolve_chat_container_workspace_root(
    paths: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    Ok(resolve_root(&paths)?.map(Value::String))
}

/// Port of `matchesLegacyHomeChatWorkspaceRoot` (`packages/shared/src/projectContainers.ts:32`).
pub fn matches_legacy_home_chat_workspace_root(
    workspace_root: serde_json::Value,
    paths: serde_json::Value,
    options: serde_json::Value,
) -> Result<bool, String> {
    let home = field(&paths, "homeDir")?;
    let home_dir = match home {
        Some(Value::String(s)) => s.trim_matches(is_js_space).to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) if is_undefined(v) => String::new(),
        Some(_) => return Err("paths.homeDir?.trim is not a function".to_string()),
    };
    let chat = resolve_root(&paths)?;
    if home_dir.is_empty() || chat.is_none() {
        return Ok(false);
    }
    let platform = platform_of(Some(&options))?;
    let root = js_trim_value(&workspace_root, "value")?;
    let chat = chat.unwrap_or_default();
    Ok(
        normalize_root(&root, platform.as_deref()) == normalize_root(&chat, platform.as_deref())
            || normalize_root(&root, platform.as_deref())
                == normalize_root(&home_dir, platform.as_deref()),
    )
}

/// Port of `isLegacyHomeChatContainerRow` (`packages/shared/src/projectContainers.ts:55`).
pub fn is_legacy_home_chat_container_row(input: serde_json::Value) -> Result<bool, String> {
    let title = field(&input, "projectTitle")?;
    if !matches!(title, Some(Value::String(s)) if s == "Home") {
        return Ok(false);
    }
    let root = field(&input, "projectWorkspaceRoot")?
        .cloned()
        .unwrap_or(Value::Null);
    let paths = field(&input, "paths")?.cloned().unwrap_or(Value::Null);
    let options = field(&input, "comparisonOptions")?
        .cloned()
        .unwrap_or(Value::Null);
    matches_legacy_home_chat_workspace_root(root, paths, options)
}

/// Port of `isGroupContainerKind` (`packages/shared/src/projectContainers.ts:71`).
pub fn is_group_container_kind(kind: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(kind, Value::String(ref s) if s == "group" || s == "studio"))
}

/// Port of `isOrdinaryProjectRow` (`packages/shared/src/projectContainers.ts:80`).
pub fn is_ordinary_project_row(input: serde_json::Value) -> Result<bool, String> {
    let kind = field(&input, "projectKind")?;
    let effective = match kind {
        Some(Value::Null) | None => "project",
        Some(v) if is_undefined(v) => "project",
        Some(v) => match v {
            Value::String(s) => s.as_str(),
            _ => "",
        },
    };
    if effective != "project" {
        return Ok(false);
    }
    Ok(!is_legacy_home_chat_container_row(input)?)
}
