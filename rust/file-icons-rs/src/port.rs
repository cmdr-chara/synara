//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the JavaScript function would throw. Use `crate::js` helpers for JS coercions
//! (utf16 strings, numbers, undefined) so every exported signature stays
//! (safe JSON inputs) -> Result<safe JSON output, String>.

use serde_json::Value;

const DEFAULT_FILE_ICON: &str = "code-brackets";
const DEFAULT_ATTACHMENT_ICON: &str = "file-text";

fn icon_by_basename(b: &str) -> Option<&'static str> {
    Some(match b {
        "package.json"
        | "package-lock.json"
        | "npm-shrinkwrap.json"
        | ".npmrc"
        | ".npmignore"
        | "yarn.lock"
        | ".yarnrc"
        | ".yarnrc.yml"
        | "pnpm-lock.yaml"
        | "pnpm-workspace.yaml" => "npm",
        "bun.lockb" | "bun.lock" => "bun",
        ".gitignore" | ".gitattributes" | ".gitmodules" | ".gitkeep" | ".gitconfig" => "git",
        "tsconfig.json"
        | "tsconfig.base.json"
        | "tsconfig.build.json"
        | "tsconfig.node.json"
        | "tsconfig.eslint.json" => "typescript",
        "cargo.toml" | "cargo.lock" => "rust",
        "requirements.txt" | "pipfile" | "pyproject.toml" | "setup.py" | "setup.cfg" => "phyton",
        "readme.md" => "markdown",
        "license" | "license.md" | "license.txt" => "file-text",
        "vercel.json" => "vercel",
        ".env" | ".env.local" | ".env.development" | ".env.production" | ".env.test"
        | ".env.example" => "settings-gear-1",
        _ => return None,
    })
}

fn icon_by_extension(e: &str) -> Option<&'static str> {
    Some(match e {
        "ts" | "mts" | "cts" | "d.ts" => "typescript",
        "tsx" | "jsx" => "react",
        "js" | "mjs" | "cjs" => "javascript",
        "json" | "json5" | "jsonc" => "json",
        "md" | "mdx" | "mdc" | "markdown" => "markdown",
        "py" | "pyi" | "pyc" | "pyw" => "phyton",
        "rs" => "rust",
        "php" | "phtml" => "php",
        "java" => "java",
        "c" | "h" => "c",
        "vue" => "vue",
        "svelte" => "svelte",
        "yml" | "yaml" | "toml" | "ini" | "conf" | "cfg" | "env" => "settings-gear-1",
        "txt" | "log" => "file-text",
        "csv" | "tsv" => "file-chart",
        "rtf" | "doc" | "docx" | "odt" | "ppt" | "pptx" | "odp" => "page-text",
        "xls" | "xlsx" | "ods" => "file-chart",
        "ics" | "ifb" | "vcs" => "calendar-days",
        "sh" | "bash" | "zsh" | "fish" | "bat" | "ps1" | "psm1" | "psd1" => "cmd",
        "lock" => "lock",
        "png" => "file-png",
        "jpg" | "jpeg" => "file-jpg",
        "gif" | "webp" | "bmp" | "tiff" | "avif" | "ico" | "svg" => "image-alt-text",
        "pdf" => "file-pdf",
        "zip" | "tar" | "gz" | "tgz" | "rar" | "7z" | "bz2" | "xz" => "file-zip",
        "mp4" | "m4v" | "mov" | "webm" | "mkv" | "avi" => "video",
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => "audio",
        _ => return None,
    })
}

fn icon_by_mime(m: &str) -> Option<&'static str> {
    Some(match m {
        "application/pdf" => "file-pdf",
        "application/json" => "json",
        "application/xml" | "text/html" | "text/xml" => "code-brackets",
        "application/zip"
        | "application/gzip"
        | "application/x-tar"
        | "application/x-7z-compressed" => "file-zip",
        "application/vnd.ms-excel"
        | "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        | "application/vnd.oasis.opendocument.spreadsheet"
        | "text/csv"
        | "text/tab-separated-values" => "file-chart",
        "application/msword"
        | "application/vnd.ms-word"
        | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        | "application/vnd.oasis.opendocument.text"
        | "application/vnd.ms-powerpoint"
        | "application/vnd.openxmlformats-officedocument.presentationml.presentation"
        | "application/vnd.oasis.opendocument.presentation"
        | "application/rtf" => "page-text",
        "text/calendar" => "calendar-days",
        "text/markdown" => "markdown",
        _ => return None,
    })
}

fn basename(path: &str) -> &str {
    let slash = path.rfind('/').max(path.rfind('\\'));
    match slash {
        None => path,
        Some(i) => &path[i + 1..],
    }
}

/// Suffixes after every non-trailing `.`, longest first.
fn extension_candidates(name: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for (i, &b) in name.as_bytes().iter().enumerate() {
        if b == b'.' && i + 1 < name.len() {
            out.push(&name[i + 1..]);
        }
    }
    out
}

pub fn basename_of_path(path_value: String) -> Result<String, String> {
    Ok(basename(&path_value).to_string())
}

pub fn path_looks_like_known_file(path_value: String) -> Result<bool, String> {
    let basename = basename(&path_value).to_lowercase();
    if icon_by_basename(&basename).is_some() {
        return Ok(true);
    }
    Ok(extension_candidates(&basename)
        .iter()
        .any(|&c| icon_by_extension(c).is_some()))
}

pub fn infer_entry_kind_from_path(path_value: String) -> Result<String, String> {
    let base = basename(&path_value);
    let lower = base.to_lowercase();
    let known = icon_by_basename(&lower).is_some()
        || extension_candidates(&lower)
            .iter()
            .any(|&c| icon_by_extension(c).is_some());
    if known {
        return Ok("file".into());
    }
    if base.starts_with('.') && !base[1..].contains('.') {
        return Ok("directory".into());
    }
    if base.contains('.') {
        return Ok("file".into());
    }
    Ok("directory".into())
}

pub fn get_file_icon_name(path_value: String) -> Result<String, String> {
    let basename = basename(&path_value).to_lowercase();
    if let Some(icon) = icon_by_basename(&basename) {
        return Ok(icon.into());
    }
    for candidate in extension_candidates(&basename) {
        if let Some(icon) = icon_by_extension(candidate) {
            return Ok(icon.into());
        }
    }
    Ok(DEFAULT_FILE_ICON.into())
}

/// `basenameOfPath(attachment.name).toLowerCase()` on a runtime value.
fn attachment_basename_lower(attachment: &Value) -> Result<String, String> {
    if attachment.is_null() {
        return Err("Cannot read properties of null (reading 'name')".into());
    }
    if crate::js::is_undefined(attachment) {
        return Err("Cannot read properties of undefined (reading 'name')".into());
    }
    match crate::js::prop(attachment, "name")? {
        None => Err("Cannot read properties of undefined (reading 'lastIndexOf')".into()),
        Some(Value::Null) => Err("Cannot read properties of null (reading 'lastIndexOf')".into()),
        Some(v) if crate::js::string_units(v).is_some() => {
            let s = String::from_utf16_lossy(&crate::js::string_units(v).unwrap_or_default());
            Ok(basename(&s).to_lowercase())
        }
        // `[].lastIndexOf` exists (element search) -> returns the array, then
        // `.toLowerCase` throws on it.
        Some(Value::Array(_)) => Err("basenameOfPath(...).toLowerCase is not a function".into()),
        Some(_) => Err("pathValue.lastIndexOf is not a function".into()),
    }
}

pub fn get_attachment_icon_name(attachment: Value) -> Result<String, String> {
    let basename = attachment_basename_lower(&attachment)?;
    if let Some(icon) = icon_by_basename(&basename) {
        return Ok(icon.into());
    }
    for candidate in extension_candidates(&basename) {
        if let Some(icon) = icon_by_extension(candidate) {
            return Ok(icon.into());
        }
    }

    // `attachment.mimeType?.trim().toLowerCase() ?? ""`
    let mime = match crate::js::prop(&attachment, "mimeType")? {
        None | Some(Value::Null) => String::new(),
        Some(v) => match crate::js::string_units(v) {
            Some(units) => String::from_utf16_lossy(&units).trim().to_lowercase(),
            None => return Err("attachment.mimeType.trim is not a function".into()),
        },
    };
    if !mime.is_empty() {
        if let Some(icon) = icon_by_mime(&mime) {
            return Ok(icon.into());
        }
        match mime.split('/').next().unwrap_or_default() {
            "image" => return Ok("image-alt-text".into()),
            "audio" => return Ok("audio".into()),
            "video" => return Ok("video".into()),
            "text" => return Ok("file-text".into()),
            _ => {}
        }
    }
    Ok(DEFAULT_ATTACHMENT_ICON.into())
}
