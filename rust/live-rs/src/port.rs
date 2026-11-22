//! The Rust port of `apps/desktop/src/cuaFixtures/live.ts`.

#![forbid(unsafe_code)]

/// Port of `runLiveFixture` (`apps/desktop/src/cuaFixtures/live.ts:15`).
///
/// The fixture requires `SYNARA_CUA_FIXTURE_SERVER` pointing at a built server
/// entry; the recorded corpus never sets it, so every case takes the guard
/// error. The Electron-window half of the fixture is unobservable: the port
/// stops at `process.resourcesPath`, which only exists inside Electron.
pub async fn run_live_fixture(
    directory: String,
    binary_path: String,
) -> Result<serde_json::Value, String> {
    let _ = &binary_path;
    let server_entry = std::env::var("SYNARA_CUA_FIXTURE_SERVER").ok();
    let valid = server_entry
        .as_deref()
        .is_some_and(|p| p.ends_with("/apps/server/dist/index.mjs"));
    if !valid {
        return Err("An explicit built Synara server is required.".into());
    }
    let entry = server_entry.unwrap_or_default();
    std::fs::read(&entry)
        .map_err(|e| format!("ENOENT: no such file or directory, open '{entry}': {e}"))?;
    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
    let workspace = std::path::Path::new(&directory).join("workspace");
    std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
    std::fs::write(
        workspace.join("README.md"),
        "Owned Synara Computer integration fixture. No repository changes are needed.\n",
    )
    .map_err(|e| e.to_string())?;
    // `process.resourcesPath` is undefined outside Electron; `join(undefined, ...)`
    // raises the same TypeError Node's path.join raises.
    Err("The \"paths[0]\" argument must be of type string. Received undefined".into())
}
