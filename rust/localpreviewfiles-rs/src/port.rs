use serde_json::Value;

const IMAGE_EXTENSIONS: &[&str] = &[
    ".avif", ".bmp", ".gif", ".heic", ".heif", ".ico", ".jpeg", ".jpg", ".png", ".svg", ".tiff",
    ".webp",
];
const VIDEO_EXTENSIONS: &[&str] = &[".m4v", ".mov", ".mp4", ".ogv", ".webm"];

/// `filePath` as a JS string or the member-access error `lastIndexOf` would raise.
fn path_str(v: &Value) -> Result<String, String> {
    match v {
        Value::Null => Err("Cannot read properties of null (reading 'lastIndexOf')".to_string()),
        Value::String(s) => Ok(s.clone()),
        _ => Err("filePath.lastIndexOf is not a function".to_string()),
    }
}

/// Port of `lowerCaseExtensionOf` (`packages/shared/src/localPreviewFiles.ts:37`).
pub fn lower_case_extension_of(
    file_path: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let path = path_str(&file_path)?;
    Ok(path
        .rfind('.')
        .map(|dot| Value::String(path[dot..].to_lowercase())))
}

/// Port of `isSupportedLocalImagePath` (`packages/shared/src/localPreviewFiles.ts:43`).
pub fn is_supported_local_image_path(file_path: serde_json::Value) -> Result<bool, String> {
    Ok(match lower_case_extension_of(file_path)? {
        Some(Value::String(ext)) => IMAGE_EXTENSIONS.contains(&ext.as_str()),
        _ => false,
    })
}

/// Port of `isSupportedLocalPdfPath` (`packages/shared/src/localPreviewFiles.ts:51`).
pub fn is_supported_local_pdf_path(file_path: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(
        lower_case_extension_of(file_path)?,
        Some(Value::String(ref ext)) if ext == ".pdf"
    ))
}

/// Port of `isSupportedLocalVideoPath` (`packages/shared/src/localPreviewFiles.ts:65`).
pub fn is_supported_local_video_path(file_path: serde_json::Value) -> Result<bool, String> {
    Ok(match lower_case_extension_of(file_path)? {
        Some(Value::String(ext)) => VIDEO_EXTENSIONS.contains(&ext.as_str()),
        _ => false,
    })
}

/// Port of `isSupportedLocalPreviewFilePath` (`packages/shared/src/localPreviewFiles.ts:71`).
pub fn is_supported_local_preview_file_path(file_path: serde_json::Value) -> Result<bool, String> {
    Ok(is_supported_local_image_path(file_path.clone())?
        || is_supported_local_video_path(file_path.clone())?
        || is_supported_local_pdf_path(file_path)?)
}
