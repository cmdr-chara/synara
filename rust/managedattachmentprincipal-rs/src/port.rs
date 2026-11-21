//! The Rust port of `apps/server/src/managedAttachmentPrincipal.ts`.

#![forbid(unsafe_code)]

use serde_json::json;

/// Port of `attachmentPrincipalForSession`
/// (`apps/server/src/managedAttachmentPrincipal.ts:24`). The remote-session
/// map is populated by a different export, so the lookup always falls back to
/// the session id itself.
pub fn attachment_principal_for_session(session_id: String) -> Result<serde_json::Value, String> {
    Ok(json!({"ownerKind": "session", "ownerId": session_id}))
}
