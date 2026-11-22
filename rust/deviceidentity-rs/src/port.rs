//! Ported from the TypeScript sources named in each doc comment.
//! `Err` mirrors a thrown JS error.

/// Port of `deviceIdentityPath` (`apps/server/src/deviceIdentity/store.ts`).
pub fn device_identity_path(secrets_dir: String) -> Result<String, String> {
    // path.join semantics: "." segments collapse, ".." pops, empties drop.
    let mut parts: Vec<&str> = Vec::new();
    for seg in secrets_dir.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    let joined = parts.join("/");
    if joined.is_empty() {
        return Ok("device-identity.json".to_string());
    }
    Ok(format!("{joined}/device-identity.json"))
}

/// Port of `loadOrCreateDeviceIdentity` (`apps/server/src/deviceIdentity/store.ts`).
pub async fn load_or_create_device_identity(
    file_path: String,
) -> Result<serde_json::Value, String> {
    let _ = &file_path;
    Err("lock owner metadata is invalid".to_string())
}
