//! The Rust port of `apps/server/src/codexWorkingDirectory.ts`.

#![forbid(unsafe_code)]

/// Port of `formatMissingCodexWorkingDirectoryError` (`apps/server/src/codexWorkingDirectory.ts:8`).
pub fn format_missing_codex_working_directory_error(cwd: String) -> Result<String, String> {
    Ok(format!(
        "Project working directory no longer exists: {cwd}. Relocate or reconnect the project in Synara."
    ))
}

/// Port of `assertCodexWorkingDirectoryExists` (`apps/server/src/codexWorkingDirectory.ts:12`).
pub fn assert_codex_working_directory_exists(cwd: String) -> Result<(), String> {
    match std::fs::metadata(&cwd) {
        Ok(stats) if !stats.is_dir() => Err(format!(
            "Project working directory is not a directory: {cwd}. Relocate or reconnect the project in Synara."
        )),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            Err(format_missing_codex_working_directory_error(cwd)?)
        }
        Err(e) => Err(e.to_string()),
    }
}
