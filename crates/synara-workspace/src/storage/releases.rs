//! Records actual locally observed native versions. This is not a release feed.
use super::*;
use crate::{StorageError, WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

pub const NATIVE_VERSION_HISTORY_KEY: &str = "native-version-history";

/// Fingerprint of the executable file currently on disk. This identifies local bytes only;
/// it does not establish a trusted publisher or match those bytes against a release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeBuildIntegrity {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: String,
}

fn fingerprint_current_executable() -> WorkspaceResult<NativeBuildIntegrity> {
    let executable = std::env::current_exe().map_err(StorageError::from)?;
    let path = fs::canonicalize(executable).map_err(StorageError::from)?;
    fingerprint_regular_file(&path)
}

fn fingerprint_regular_file(path: &Path) -> WorkspaceResult<NativeBuildIntegrity> {
    let path = path.to_path_buf();
    let path_before = fs::symlink_metadata(&path).map_err(StorageError::from)?;
    if !path_before.is_file() || path_before.file_type().is_symlink() {
        return Err(WorkspaceError::Invalid(
            "The current executable is not a regular file.".into(),
        ));
    }

    let mut file = File::open(&path).map_err(StorageError::from)?;
    let opened_before = file.metadata().map_err(StorageError::from)?;
    if !opened_before.is_file()
        || !same_file_entry(&path, &file, &path_before, &opened_before)?
        || opened_before.len() != path_before.len()
        || opened_before.modified().ok() != path_before.modified().ok()
    {
        return Err(WorkspaceError::Invalid(
            "The current executable changed while it was opened.".into(),
        ));
    }

    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(StorageError::from)?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }

    let opened_after = file.metadata().map_err(StorageError::from)?;
    let path_after = fs::symlink_metadata(&path).map_err(StorageError::from)?;
    if opened_after.len() != opened_before.len()
        || opened_after.modified().ok() != opened_before.modified().ok()
        || !path_after.is_file()
        || path_after.file_type().is_symlink()
        || !same_file_entry(&path, &file, &path_after, &opened_after)?
        || path_after.len() != opened_after.len()
        || path_after.modified().ok() != opened_after.modified().ok()
    {
        return Err(WorkspaceError::Invalid(
            "The current executable changed while it was fingerprinted.".into(),
        ));
    }

    Ok(NativeBuildIntegrity {
        path,
        size_bytes: opened_after.len(),
        sha256: hex::encode(digest.finalize()),
    })
}

#[cfg(unix)]
fn same_file_entry(
    _path: &Path,
    _file: &File,
    left: &std::fs::Metadata,
    right: &std::fs::Metadata,
) -> WorkspaceResult<bool> {
    use std::os::unix::fs::MetadataExt;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(windows)]
fn same_file_entry(
    path: &Path,
    file: &File,
    _left: &std::fs::Metadata,
    _right: &std::fs::Metadata,
) -> WorkspaceResult<bool> {
    let opened = same_file::Handle::from_file(file.try_clone().map_err(StorageError::from)?)
        .map_err(StorageError::from)?;
    let current = same_file::Handle::from_path(path).map_err(StorageError::from)?;
    Ok(opened == current)
}

#[cfg(not(any(unix, windows)))]
fn same_file_entry(
    _path: &Path,
    _file: &File,
    _left: &std::fs::Metadata,
    _right: &std::fs::Metadata,
) -> WorkspaceResult<bool> {
    Ok(true)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeVersionVisit {
    pub version: String,
    pub observed_at_ms: i64,
    pub read: bool,
}
impl NativeVersionVisit {
    pub fn observed_label(&self) -> String {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(self.observed_at_ms)
            .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "Unknown observation date".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeVersionHistory {
    pub format_version: u32,
    pub revision: u64,
    /// Chronological observations, bounded to the most recent 32 transitions.
    pub visits: Vec<NativeVersionVisit>,
}
impl Default for NativeVersionHistory {
    fn default() -> Self {
        Self {
            format_version: 1,
            revision: 0,
            visits: vec![],
        }
    }
}
impl NativeVersionHistory {
    fn validate(&self) -> WorkspaceResult<()> {
        if self.format_version != 1
            || self.visits.len() > 32
            || self
                .visits
                .iter()
                .any(|v| !valid_version(&v.version) || v.observed_at_ms < 0)
            || self.visits.windows(2).any(|v| v[0].version == v[1].version)
        {
            return Err(WorkspaceError::Invalid(
                "Saved native version history is invalid. It was not replaced.".into(),
            ));
        }
        Ok(())
    }
    pub fn current(&self) -> Option<&NativeVersionVisit> {
        self.visits.last()
    }
    fn observe(&mut self, version: &str, now: i64) -> WorkspaceResult<bool> {
        self.validate()?;
        if !valid_version(version) || now < 0 {
            return Err(StorageError::Limit.into());
        }
        if self.current().is_some_and(|v| v.version == version) {
            return Ok(false);
        }
        self.visits.push(NativeVersionVisit {
            version: version.into(),
            observed_at_ms: now,
            read: false,
        });
        if self.visits.len() > 32 {
            self.visits.remove(0);
        }
        self.revision = self.revision.checked_add(1).ok_or(StorageError::Limit)?;
        Ok(true)
    }
}
fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
}
fn read(connection: &Connection) -> WorkspaceResult<NativeVersionHistory> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [NATIVE_VERSION_HISTORY_KEY],
            |r| r.get(0),
        )
        .optional()?;
    if raw.as_ref().is_some_and(|r| r.len() > 16 * 1024) {
        return Err(StorageError::Limit.into());
    }
    let value: NativeVersionHistory = raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    value.validate()?;
    Ok(value)
}
fn write(connection: &Connection, value: &NativeVersionHistory) -> WorkspaceResult<()> {
    value.validate()?;
    connection.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",params![NATIVE_VERSION_HISTORY_KEY,encode(value)?])?;
    Ok(())
}
impl WorkspaceService {
    /// Fingerprint the running application's executable file without blocking the workspace
    /// storage worker. This is local integrity metadata, not a release signature check.
    pub async fn native_build_integrity(&self) -> WorkspaceResult<NativeBuildIntegrity> {
        tokio::task::spawn_blocking(fingerprint_current_executable)
            .await
            .map_err(|_| WorkspaceError::Worker)?
    }

    /// Called with the compiled native application version, never a feed or environment override.
    pub async fn observe_native_version(
        &self,
        version: String,
    ) -> WorkspaceResult<NativeVersionHistory> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut history = read(&tx)?;
            if history.observe(&version, crate::now_ms())? {
                write(&tx, &history)?;
            }
            tx.commit()?;
            Ok(history)
        })
        .await
    }
    pub async fn mark_native_version_read(
        &self,
        version: String,
        revision: u64,
    ) -> WorkspaceResult<NativeVersionHistory> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut history = read(&tx)?;
            if history.revision != revision
                || history.current().is_none_or(|v| v.version != version)
            {
                return Err(WorkspaceError::Invalid(
                    "Native version history changed. Reload before dismissing.".into(),
                ));
            }
            history.visits.last_mut().unwrap().read = true;
            history.revision = history.revision.checked_add(1).ok_or(StorageError::Limit)?;
            write(&tx, &history)?;
            tx.commit()?;
            Ok(history)
        })
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn executable_fingerprint_reports_size_and_sha256() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native-build");
        let bytes = b"Synara native build fingerprint fixture";
        File::create(&path).unwrap().write_all(bytes).unwrap();

        let fingerprint = fingerprint_regular_file(&path).unwrap();
        assert_eq!(fingerprint.size_bytes, bytes.len() as u64);
        assert_eq!(
            fingerprint.sha256,
            "0fafb575d74ac7b55a610e7c602f16b870b4b28683d68e6b40f62631eab4182e"
        );
    }

    #[cfg(unix)]
    #[test]
    fn executable_fingerprint_rejects_symlink_paths() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target");
        let link = dir.path().join("link");
        File::create(&target)
            .unwrap()
            .write_all(b"payload")
            .unwrap();
        symlink(&target, &link).unwrap();

        assert!(fingerprint_regular_file(&link).is_err());
    }

    #[tokio::test]
    async fn workspace_fingerprint_identifies_current_executable_file() {
        let expected_path = fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
        let expected_size = fs::metadata(&expected_path).unwrap().len();
        let workspace = WorkspaceService::memory().unwrap();

        let fingerprint = workspace.native_build_integrity().await.unwrap();
        assert_eq!(fingerprint.path, expected_path);
        assert_eq!(fingerprint.size_bytes, expected_size);
        assert_eq!(fingerprint.sha256.len(), 64);
        assert!(fingerprint.sha256.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn releases_transitions_are_bounded_and_do_not_claim_upgrade_order() {
        let mut h = NativeVersionHistory::default();
        assert!(h.observe("0.1.0", 1).unwrap());
        h.visits[0].read = true;
        assert!(!h.observe("0.1.0", 2).unwrap());
        assert!(h.current().unwrap().read);
        assert!(h.observe("0.0.9", 3).unwrap());
        assert!(!h.current().unwrap().read);
        for n in 0..50 {
            h.observe(&format!("0.2.{n}"), n + 4).unwrap();
        }
        assert_eq!(h.visits.len(), 32);
        h.validate().unwrap();
        assert!(h.observe("evil\nversion", 70).is_err());
    }
    #[tokio::test]
    async fn releases_read_state_survives_restart_and_stale_dismissal_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("state.db");
        let s = WorkspaceService::open(path.clone()).await.unwrap();
        let old = s.observe_native_version("0.1.0".into()).await.unwrap();
        let read = s
            .mark_native_version_read("0.1.0".into(), old.revision)
            .await
            .unwrap();
        drop(s);
        let s = WorkspaceService::open(path).await.unwrap();
        assert_eq!(
            s.observe_native_version("0.1.0".into()).await.unwrap(),
            read
        );
        let new = s.observe_native_version("0.2.0".into()).await.unwrap();
        assert!(!new.current().unwrap().read);
        assert!(
            s.mark_native_version_read("0.1.0".into(), read.revision)
                .await
                .is_err()
        );
        assert!(s.catalog().await.unwrap().tasks.is_empty());
    }
    #[test]
    fn releases_unknown_or_corrupt_history_is_not_replaced() {
        let mut h = NativeVersionHistory {
            format_version: 2,
            ..NativeVersionHistory::default()
        };
        assert!(h.observe("0.1.0", 1).is_err());
        h.format_version = 1;
        h.visits.push(NativeVersionVisit {
            version: "<invalid>".into(),
            observed_at_ms: 0,
            read: false,
        });
        assert!(h.observe("0.1.0", 1).is_err());
        assert_eq!(h.visits[0].version, "<invalid>");
    }
}
