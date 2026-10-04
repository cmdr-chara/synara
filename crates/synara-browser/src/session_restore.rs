//! Explicit, manual-profile tab restoration.
//!
//! This snapshot stores only committed HTTP(S) URLs. It never reads or exports
//! WebKit cookies, and it cannot restore AgentTask or Authentication partitions.
//! Query strings and fragments are removed because they commonly contain OAuth
//! codes, invite tokens, or other short-lived secrets.

use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const MAX_TABS: usize = 16;
const MAX_URL_BYTES: usize = 2_048;
const MAX_SNAPSHOT_BYTES: u64 = 64 * 1024;
const SNAPSHOT_NAME: &str = "manual-tabs.json";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum RestoreError {
    #[error("manual browser restore storage is unavailable")]
    Storage,
    #[error("manual browser restore data is invalid or exceeds its limits")]
    Invalid,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    version: u8,
    urls: Vec<String>,
}

/// Snapshot store for URLs in the human-operated Manual profile only.
///
/// The caller must opt in before this store writes anything. Storage is kept
/// in the app-private browser directory with owner-only permissions; the
/// session cookies remain inside WebKit's existing profile.
#[derive(Debug)]
pub struct ManualTabRestoreStore {
    root: PathBuf,
    enabled: bool,
    urls: Vec<String>,
}

impl ManualTabRestoreStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RestoreError> {
        let root = root.into();
        let path = root.join(SNAPSHOT_NAME);
        match fs::symlink_metadata(&root) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    root,
                    enabled: false,
                    urls: Vec::new(),
                });
            }
            Err(_) => return Err(RestoreError::Storage),
            Ok(_) => (),
        }
        secure_directory(&root)?;
        let file = match open_read_nofollow(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self {
                    root,
                    enabled: false,
                    urls: Vec::new(),
                });
            }
            Err(_) => return Err(RestoreError::Storage),
        };
        let metadata = file.metadata().map_err(|_| RestoreError::Storage)?;
        if !metadata.is_file() || metadata.len() > MAX_SNAPSHOT_BYTES {
            return Err(RestoreError::Invalid);
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_SNAPSHOT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| RestoreError::Storage)?;
        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(RestoreError::Invalid);
        }
        let decoded = serde_json::from_slice(&bytes);
        bytes.fill(0);
        let snapshot: Snapshot = decoded.map_err(|_| RestoreError::Invalid)?;
        if snapshot.version != 1 || snapshot.urls.len() > MAX_TABS {
            return Err(RestoreError::Invalid);
        }
        let urls = validate_snapshot_urls(snapshot.urls)?;
        Ok(Self {
            root,
            enabled: true,
            urls,
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Committed, privacy-filtered URLs to restore after the browser panel is opened.
    pub fn urls(&self) -> &[String] {
        &self.urls
    }

    /// Enabling persists the current manual tabs; disabling removes the snapshot.
    pub fn set_enabled(
        &mut self,
        enabled: bool,
        current_urls: &[String],
    ) -> Result<(), RestoreError> {
        if enabled {
            let urls = validate_urls(current_urls)?;
            self.write_snapshot(&urls)?;
            self.urls = urls;
            self.enabled = true;
        } else {
            remove_snapshot(&self.root)?;
            self.urls.clear();
            self.enabled = false;
        }
        Ok(())
    }

    /// Update the enabled snapshot after manual tab navigation or close.
    pub fn save_current(&mut self, current_urls: &[String]) -> Result<(), RestoreError> {
        if !self.enabled {
            return Ok(());
        }
        let urls = validate_urls(current_urls)?;
        if urls == self.urls {
            return Ok(());
        }
        self.write_snapshot(&urls)?;
        self.urls = urls;
        Ok(())
    }

    fn write_snapshot(&self, urls: &[String]) -> Result<(), RestoreError> {
        secure_directory(&self.root)?;
        let path = self.root.join(SNAPSHOT_NAME);
        if path_exists_nofollow(&path)? {
            let metadata = fs::symlink_metadata(&path).map_err(|_| RestoreError::Storage)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(RestoreError::Storage);
            }
        }
        let bytes = serde_json::to_vec(&Snapshot {
            version: 1,
            urls: urls.to_vec(),
        })
        .map_err(|_| RestoreError::Invalid)?;
        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err(RestoreError::Invalid);
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = self.root.join(format!(
            ".manual-tabs-{}-{sequence}.tmp",
            std::process::id()
        ));
        let mut file = create_temp_nofollow(&temporary).map_err(|_| RestoreError::Storage)?;
        let write_result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            sync_directory(&self.root)?;
            Ok::<_, std::io::Error>(())
        })();
        let mut bytes = bytes;
        bytes.fill(0);
        if write_result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(RestoreError::Storage);
        }
        Ok(())
    }
}

/// Keep HTTP(S) path state but drop query and fragment values before persistence.
/// These fields frequently hold credentials or one-time authorization material.
pub fn restorable_manual_url(raw: &str) -> Result<String, RestoreError> {
    let document = crate::CommittedDocument::parse(raw).map_err(|_| RestoreError::Invalid)?;
    let mut url = url::Url::parse(&document.canonical_url).map_err(|_| RestoreError::Invalid)?;
    url.set_query(None);
    url.set_fragment(None);
    let value = url.to_string();
    if value.len() > MAX_URL_BYTES {
        return Err(RestoreError::Invalid);
    }
    Ok(value)
}

fn validate_urls(raw: &[String]) -> Result<Vec<String>, RestoreError> {
    if raw.len() > MAX_TABS {
        return Err(RestoreError::Invalid);
    }
    raw.iter().map(|url| restorable_manual_url(url)).collect()
}

fn validate_snapshot_urls(urls: Vec<String>) -> Result<Vec<String>, RestoreError> {
    let validated = validate_urls(&urls)?;
    if validated != urls {
        return Err(RestoreError::Invalid);
    }
    Ok(validated)
}

fn secure_directory(root: &Path) -> Result<(), RestoreError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => (),
        Ok(_) => return Err(RestoreError::Storage),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(root).map_err(|_| RestoreError::Storage)?;
        }
        Err(_) => return Err(RestoreError::Storage),
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| RestoreError::Storage)?;
    }
    Ok(())
}

fn open_read_nofollow(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(open_no_follow_flag());
    }
    options.open(path)
}

fn create_temp_nofollow(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(open_no_follow_flag());
    }
    options.open(path)
}

#[cfg(target_os = "linux")]
fn open_no_follow_flag() -> i32 {
    // Linux O_NOFOLLOW; the browser adapter itself is Linux/X11-only.
    0x2_0000
}

#[cfg(target_os = "macos")]
fn open_no_follow_flag() -> i32 {
    // macOS O_NOFOLLOW.
    0x0000_0100
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
fn open_no_follow_flag() -> i32 {
    0
}

fn path_exists_nofollow(path: &Path) -> Result<bool, RestoreError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(RestoreError::Storage),
    }
}

fn remove_snapshot(root: &Path) -> Result<(), RestoreError> {
    match fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(RestoreError::Storage),
        Ok(_) => (),
    }
    secure_directory(root)?;
    let path = root.join(SNAPSHOT_NAME);
    if path_exists_nofollow(&path)? {
        let metadata = fs::symlink_metadata(&path).map_err(|_| RestoreError::Storage)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(RestoreError::Storage);
        }
        fs::remove_file(path).map_err(|_| RestoreError::Storage)?;
        sync_directory(root).map_err(|_| RestoreError::Storage)?;
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(root: &Path) -> std::io::Result<()> {
    File::open(root)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_: &Path) -> std::io::Result<()> {
    Ok(())
}

const OWNED_SNAPSHOT_NAME: &str = "owned-tabs.json";
const MAX_OWNERS: usize = 128;
const MAX_OWNED_TABS: usize = 128;
const MAX_OWNED_SNAPSHOT_BYTES: u64 = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthSnapshot {
    origin: String,
    url: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnedSnapshot {
    version: u8,
    tasks: std::collections::BTreeMap<String, Vec<String>>,
    auth: std::collections::BTreeMap<String, AuthSnapshot>,
}

#[derive(Debug)]
pub struct OwnedTabRestoreStore {
    root: PathBuf,
    value: OwnedSnapshot,
}

impl OwnedTabRestoreStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, RestoreError> {
        let root = root.into();
        let path = root.join(OWNED_SNAPSHOT_NAME);
        let value = match open_read_nofollow(&path) {
            Ok(file) => {
                let metadata = file.metadata().map_err(|_| RestoreError::Storage)?;
                if !metadata.is_file() || metadata.len() > MAX_OWNED_SNAPSHOT_BYTES {
                    return Err(RestoreError::Invalid);
                }
                let mut bytes = Vec::with_capacity(metadata.len() as usize);
                file.take(MAX_OWNED_SNAPSHOT_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| RestoreError::Storage)?;
                if bytes.len() as u64 > MAX_OWNED_SNAPSHOT_BYTES {
                    return Err(RestoreError::Invalid);
                }
                let decoded = serde_json::from_slice(&bytes).map_err(|_| RestoreError::Invalid)?;
                bytes.fill(0);
                decoded
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => OwnedSnapshot {
                version: 1,
                ..OwnedSnapshot::default()
            },
            Err(_) => return Err(RestoreError::Storage),
        };
        validate_owned(&value)?;
        Ok(Self { root, value })
    }

    pub fn task_urls(&self, task: u128) -> Vec<String> {
        self.value
            .tasks
            .get(&owner_key(task))
            .cloned()
            .unwrap_or_default()
    }

    pub fn save_task(&mut self, task: u128, urls: &[String]) -> Result<(), RestoreError> {
        let urls = validate_urls(urls)?;
        let key = owner_key(task);
        if urls.is_empty() {
            self.value.tasks.remove(&key);
        } else {
            self.value.tasks.insert(key, urls);
        }
        self.write()
    }

    /// Return a previous sanitized auth path only for a fresh request owned by
    /// the same task and exact HTTP(S) origin. Cookies and request authority are
    /// deliberately not restored.
    pub fn auth_url(&self, task: u128, request_url: &str) -> Option<String> {
        let request = crate::CommittedDocument::parse(request_url).ok()?;
        let saved = self.value.auth.get(&owner_key(task))?;
        let saved_doc = crate::CommittedDocument::parse(&saved.url).ok()?;
        (saved.origin == origin_key(&request) && saved_doc.origin == request.origin)
            .then(|| saved.url.clone())
    }

    pub fn save_auth(
        &mut self,
        task: u128,
        request_url: &str,
        committed_url: &str,
    ) -> Result<(), RestoreError> {
        let request =
            crate::CommittedDocument::parse(request_url).map_err(|_| RestoreError::Invalid)?;
        let committed =
            crate::CommittedDocument::parse(committed_url).map_err(|_| RestoreError::Invalid)?;
        if request.origin != committed.origin {
            return Err(RestoreError::Invalid);
        }
        let url = restorable_manual_url(&committed.canonical_url)?;
        self.value.auth.insert(
            owner_key(task),
            AuthSnapshot {
                origin: origin_key(&request),
                url,
            },
        );
        self.write()
    }

    pub fn clear_task(&mut self, task: u128) -> Result<(), RestoreError> {
        let key = owner_key(task);
        self.value.tasks.remove(&key);
        self.value.auth.remove(&key);
        self.write()
    }

    fn write(&self) -> Result<(), RestoreError> {
        validate_owned(&self.value)?;
        secure_directory(&self.root)?;
        let path = self.root.join(OWNED_SNAPSHOT_NAME);
        if path_exists_nofollow(&path)? {
            let metadata = fs::symlink_metadata(&path).map_err(|_| RestoreError::Storage)?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(RestoreError::Storage);
            }
        }
        let bytes = serde_json::to_vec(&self.value).map_err(|_| RestoreError::Invalid)?;
        if bytes.len() as u64 > MAX_OWNED_SNAPSHOT_BYTES {
            return Err(RestoreError::Invalid);
        }
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .root
            .join(format!(".owned-tabs-{}-{sequence}.tmp", std::process::id()));
        let mut file = create_temp_nofollow(&temporary).map_err(|_| RestoreError::Storage)?;
        let result = (|| {
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&temporary, &path)?;
            sync_directory(&self.root)?;
            Ok::<_, std::io::Error>(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
            return Err(RestoreError::Storage);
        }
        Ok(())
    }
}

fn owner_key(task: u128) -> String {
    format!("{task:032x}")
}
fn origin_key(document: &crate::CommittedDocument) -> String {
    format!(
        "{:?}://{}:{}",
        document.origin.scheme, document.origin.host, document.origin.port
    )
}
fn validate_owned(value: &OwnedSnapshot) -> Result<(), RestoreError> {
    if value.version != 1
        || value.tasks.len() > MAX_OWNERS
        || value.auth.len() > MAX_OWNERS
        || value.tasks.values().map(Vec::len).sum::<usize>() > MAX_OWNED_TABS
    {
        return Err(RestoreError::Invalid);
    }
    for (owner, urls) in &value.tasks {
        if owner.len() != 32 || !owner.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(RestoreError::Invalid);
        }
        if validate_snapshot_urls(urls.clone())? != *urls {
            return Err(RestoreError::Invalid);
        }
    }
    for (owner, auth) in &value.auth {
        if owner.len() != 32
            || !owner.bytes().all(|b| b.is_ascii_hexdigit())
            || auth.origin.is_empty()
            || auth.origin.len() > 512
            || restorable_manual_url(&auth.url)? != auth.url
        {
            return Err(RestoreError::Invalid);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "synara-browser-restore-{}-{}",
            std::process::id(),
            TEST_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn strips_query_and_fragment_and_rejects_non_web_urls() {
        assert_eq!(
            restorable_manual_url("https://example.test/path?code=secret#access_token").unwrap(),
            "https://example.test/path"
        );
        for value in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://user:password@example.test/",
            "https://example.test/\n",
        ] {
            assert_eq!(restorable_manual_url(value), Err(RestoreError::Invalid));
        }
    }

    #[test]
    fn opt_in_round_trips_bounded_manual_urls_and_disable_removes_them() {
        let root = temp_root();
        let mut store = ManualTabRestoreStore::open(&root).unwrap();
        assert!(!store.enabled());
        let tabs = vec!["https://example.test/one?token=private".to_owned()];
        store.set_enabled(true, &tabs).unwrap();
        assert_eq!(store.urls(), &["https://example.test/one"]);
        let saved = fs::read(root.join(SNAPSHOT_NAME)).unwrap();
        assert!(!String::from_utf8_lossy(&saved).contains("private"));
        assert!(!String::from_utf8_lossy(&saved).contains("cookie"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(root.join(SNAPSHOT_NAME))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }

        let restored = ManualTabRestoreStore::open(&root).unwrap();
        assert!(restored.enabled());
        assert_eq!(restored.urls(), &["https://example.test/one"]);

        store.set_enabled(false, &[]).unwrap();
        assert!(!root.join(SNAPSHOT_NAME).exists());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn enforces_tab_and_url_limits_and_rejects_symlink_snapshots() {
        let root = temp_root();
        let mut store = ManualTabRestoreStore::open(&root).unwrap();
        let too_many = vec!["https://example.test/".to_owned(); MAX_TABS + 1];
        assert_eq!(
            store.set_enabled(true, &too_many),
            Err(RestoreError::Invalid)
        );
        let too_long = format!("https://example.test/{}", "x".repeat(MAX_URL_BYTES));
        assert_eq!(restorable_manual_url(&too_long), Err(RestoreError::Invalid));

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let target = root.join("outside");
            fs::write(&target, br#"{"version":1,"urls":[]}"#).unwrap();
            symlink(&target, root.join(SNAPSHOT_NAME)).unwrap();
            assert_eq!(
                ManualTabRestoreStore::open(&root).err(),
                Some(RestoreError::Storage)
            );
        }
        let _ = fs::remove_dir_all(root);
    }
}
