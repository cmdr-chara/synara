//! Editor icon serving for `/api/editor-icon`, lean macOS slice of upstream
//! editorAppIcons.ts. Exact semantics where they apply: the icon comes from
//! the installed .app bundle (never bundled with Synara), conversion uses the
//! platform `sips` tool capped at 128px, results are cached on disk with a
//! 24h cache-control and negative lookups are cached 5 minutes. Windows
//! package icon extraction is a documented non-goal on this platform.

use axum::{
    extract::RawQuery,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::attachments_http::attachments_http_query;
use crate::todo::state_dir;

const ICON_MAX_DIMENSION_PX: u32 = 128;
const NEGATIVE_CACHE_TTL: Duration = Duration::from_secs(300);

/// Editor id to macOS application names, exact subset of upstream EDITORS.
fn editor_mac_applications(editor_id: &str) -> Option<&'static [&'static str]> {
    match editor_id {
        "cursor" => Some(&["Cursor"]),
        "trae" => Some(&["Trae"]),
        "vscode" => Some(&["Visual Studio Code"]),
        "vscode-insiders" => Some(&["Visual Studio Code - Insiders"]),
        "vscodium" => Some(&["VSCodium"]),
        "zed" => Some(&["Zed"]),
        "windsurf" => Some(&["Windsurf"]),
        "sublime" => Some(&["Sublime Text"]),
        "antigravity" => Some(&["Antigravity"]),
        "jetbrains-toolbox" => Some(&["JetBrains Toolbox"]),
        _ => None,
    }
}

fn negative_cache() -> &'static Mutex<std::collections::HashMap<String, Instant>> {
    static CACHE: OnceLock<Mutex<std::collections::HashMap<String, Instant>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn find_app_bundle(app_name: &str) -> Option<PathBuf> {
    let bundle = if app_name.ends_with(".app") {
        app_name.to_string()
    } else {
        format!("{app_name}.app")
    };
    let mut candidates = vec![
        PathBuf::from("/Applications").join(&bundle),
        PathBuf::from("/Applications/Utilities").join(&bundle),
        PathBuf::from("/Applications/JetBrains Toolbox").join(&bundle),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join("Applications").join(&bundle));
    }
    candidates.into_iter().find(|c| c.is_dir())
}

/// Resolves the bundle icon file: CFBundleIconFile from Info.plist when
/// declared, else the first .icns in Resources.
fn find_bundle_icon(app_path: &PathBuf) -> Option<PathBuf> {
    let resources = app_path.join("Contents/Resources");
    let plist = app_path.join("Contents/Info.plist");
    if let Ok(content) = std::fs::read_to_string(&plist) {
        if let Some(pos) = content.find("CFBundleIconFile") {
            let rest = &content[pos..];
            if let Some(start) = rest.find("<string>") {
                if let Some(end) = rest[start..].find("</string>") {
                    let name = &rest[start + 8..start + end];
                    let name = if name.ends_with(".icns") {
                        name.to_string()
                    } else {
                        format!("{name}.icns")
                    };
                    let candidate = resources.join(&name);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
    }
    let entries = std::fs::read_dir(&resources).ok()?;
    let mut icns: Vec<PathBuf> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("icns"))
        .collect();
    icns.sort();
    icns.into_iter().next()
}

fn cache_dir() -> PathBuf {
    PathBuf::from(state_dir()).join("editor-icon-cache")
}

fn convert_with_sips(icon_path: &PathBuf, editor_id: &str) -> Option<PathBuf> {
    let dir = cache_dir();
    std::fs::create_dir_all(&dir).ok()?;
    let out = dir.join(format!("{editor_id}.png"));
    if out.is_file() {
        return Some(out);
    }
    let status = std::process::Command::new("sips")
        .args([
            "-s",
            "format",
            "png",
            "-Z",
            &ICON_MAX_DIMENSION_PX.to_string(),
            icon_path.to_str()?,
            "--out",
            out.to_str()?,
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .ok()?;
    if status.success() && out.is_file() {
        Some(out)
    } else {
        None
    }
}

pub async fn editor_icon(RawQuery(query): RawQuery) -> Response {
    let query = query.unwrap_or_default();
    let editor_id = attachments_http_query(&query, "id").unwrap_or_default();
    if editor_id.trim().is_empty() {
        return (StatusCode::BAD_REQUEST, "Missing id parameter").into_response();
    }
    // Negative cache: a missing editor stays missing for 5 minutes so the UI
    // rescan does not hit the filesystem on every render.
    {
        let cache = negative_cache().lock().unwrap_or_else(|e| e.into_inner());
        if let Some(at) = cache.get(&editor_id) {
            if at.elapsed() < NEGATIVE_CACHE_TTL {
                return (StatusCode::NOT_FOUND, "Not Found").into_response();
            }
        }
    }
    let resolved = editor_mac_applications(&editor_id).and_then(|names| {
        names.iter().find_map(|name| {
            let bundle = find_app_bundle(name)?;
            let icon = find_bundle_icon(&bundle)?;
            convert_with_sips(&icon, &editor_id)
        })
    });
    let Some(png_path) = resolved else {
        negative_cache()
            .lock()
            .unwrap()
            .insert(editor_id.clone(), Instant::now());
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    };
    match std::fs::read(&png_path) {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/png".to_string()),
                (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
            ],
            bytes,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}
