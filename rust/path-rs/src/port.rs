//! The Rust port of `packages/shared/src/path.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn normalize_separators(s: &str) -> String {
    s.replace('\\', "/")
}

fn js_lowercase(s: &str) -> String {
    s.chars().flat_map(char::to_lowercase).collect()
}

/// Port of `isWindowsDrivePath` (`packages/shared/src/path.ts:6`).
pub fn is_windows_drive_path(value: String) -> Result<bool, String> {
    let b = value.as_bytes();
    Ok(b.len() >= 3
        && b[0].is_ascii_alphabetic()
        && b[1] == b':'
        && (b[2] == b'/' || b[2] == b'\\'))
}

/// Port of `isUncPath` (`packages/shared/src/path.ts:10`).
pub fn is_unc_path(value: String) -> Result<bool, String> {
    Ok(value.starts_with("\\\\"))
}

/// Port of `isWindowsAbsolutePath` (`packages/shared/src/path.ts:14`).
pub fn is_windows_absolute_path(value: String) -> Result<bool, String> {
    Ok(is_windows_drive_path(value.clone())? || is_unc_path(value)?)
}

/// Port of `isLocalAbsolutePath` (`packages/shared/src/path.ts:18`).
pub fn is_local_absolute_path(value: String, options: serde_json::Value) -> Result<bool, String> {
    let allow = if crate::js::is_undefined(&options) {
        true
    } else if options.is_null() {
        return Err("Cannot read properties of null (reading 'allowWindowsPaths')".to_string());
    } else {
        options.get("allowWindowsPaths") != Some(&Value::Bool(false))
    };
    if value.starts_with('/') {
        return Ok(true);
    }
    Ok(is_windows_absolute_path(value)? && allow)
}

/// Port of `isExplicitRelativePath` (`packages/shared/src/path.ts:27`).
pub fn is_explicit_relative_path(value: String) -> Result<bool, String> {
    Ok(value.starts_with("./")
        || value.starts_with("../")
        || value.starts_with(".\\")
        || value.starts_with("..\\"))
}

/// Port of `isNormalizedWindowsAbsolutePath` (`packages/shared/src/path.ts:43`).
pub fn is_normalized_windows_absolute_path(value: String) -> Result<bool, String> {
    if value.starts_with("//") {
        return Ok(true);
    }
    let b = value.as_bytes();
    Ok(b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'/')
}

/// Port of `localPathsEqual` (`packages/shared/src/path.ts:47`).
pub fn local_paths_equal(left: String, right: String) -> Result<bool, String> {
    Ok(normalize_separators(&left) == normalize_separators(&right))
}

/// Port of `workspaceRelativePathOf` (`packages/shared/src/path.ts:74`).
pub fn workspace_relative_path_of(
    target_path: String,
    workspace_root: String,
) -> Result<Option<String>, String> {
    let target_n = normalize_separators(&target_path);
    let root_n = normalize_separators(&workspace_root);
    let windows_mode = is_normalized_windows_absolute_path(target_n.clone())?
        || is_normalized_windows_absolute_path(root_n.clone())?;
    let target_segs: Vec<&str> = target_n.split('/').collect();
    let root_trim = root_n.trim_end_matches('/');
    if root_trim.is_empty() {
        return Ok(None);
    }
    let root_segs: Vec<&str> = root_trim.split('/').collect();
    if target_segs.len() <= root_segs.len() {
        return Ok(None);
    }
    for (t, r) in target_segs.iter().zip(root_segs.iter()) {
        let eq = if windows_mode {
            js_lowercase(t) == js_lowercase(r)
        } else {
            t == r
        };
        if !eq {
            return Ok(None);
        }
    }
    let rel = target_segs[root_segs.len()..].join("/");
    if !is_workspace_relative_path_safe(rel.clone())? {
        return Ok(None);
    }
    Ok(Some(rel))
}

/// Port of `joinWorkspaceRelativePath` (`packages/shared/src/path.ts:102`).
pub fn join_workspace_relative_path(
    workspace_root: String,
    relative_path: String,
) -> Result<String, String> {
    let sep = if workspace_root.contains('\\') {
        '\\'
    } else {
        '/'
    };
    let rel = relative_path
        .chars()
        .map(|c| if c == '/' || c == '\\' { sep } else { c })
        .collect::<String>();
    if workspace_root.ends_with('/') || workspace_root.ends_with('\\') {
        return Ok(format!("{workspace_root}{rel}"));
    }
    Ok(format!("{workspace_root}{sep}{rel}"))
}

/// Port of `isWorkspaceRelativePathSafe` (`packages/shared/src/path.ts:111`).
pub fn is_workspace_relative_path_safe(value: String) -> Result<bool, String> {
    if value.trim().is_empty() {
        return Ok(false);
    }
    if value.starts_with('/') || is_windows_absolute_path(value.clone())? {
        return Ok(false);
    }
    Ok(!value
        .split(['/', '\\'])
        .any(|seg| seg == "." || seg == ".."))
}
