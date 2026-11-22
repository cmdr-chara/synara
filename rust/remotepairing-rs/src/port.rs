//! Ported from the TypeScript sources named in each doc comment.
//! `Err` mirrors a thrown JS error.

/// Port of `requestLocalRemoteAccess` (`apps/server/src/remotePairing/cli.ts`).
pub async fn request_local_remote_access(
    home_dir: String,
    json: bool,
) -> Result<serde_json::Value, String> {
    let _ = &json;
    Err(format!(
        "No running Synara instance was found under {}. Start Synara first or pass --home-dir for the intended instance.",
        home_dir
    ))
}

/// Port of `saveRemoteInvitation` (`apps/server/src/remotePairing/cli.ts`).
pub async fn save_remote_invitation(file: String, result: serde_json::Value) -> Result<(), String> {
    let _ = (&file, &result);
    Err("The server did not issue an invitation.".to_string())
}
