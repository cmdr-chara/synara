//! The Rust port of `packages/shared/src/workspaceFileWrite.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

/// Port of `isWorkspaceFileWriteConflictError` (`packages/shared/src/workspaceFileWrite.ts`).
pub fn is_workspace_file_write_conflict_error(error: Value) -> Result<bool, String> {
    Ok(error.get("code").and_then(Value::as_str) == Some("WORKSPACE_FILE_CONFLICT"))
}
