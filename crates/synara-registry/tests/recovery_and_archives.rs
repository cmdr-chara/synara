//! End-to-end offline installer failure tests. Fixture bytes are never executed.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Cursor, Write},
    sync::{Mutex, mpsc},
    time::Duration,
};
use synara_registry::{
    AgentEntry, Downloader, InstallPlan, Platform, RegistryError, RegistryStore, Result,
};

struct Bytes(Vec<u8>);
impl Downloader for Bytes {
    fn download(&self, _: &str, output: &mut dyn Write, limit: u64) -> Result<()> {
        if self.0.len() as u64 > limit {
            return Err(RegistryError::Limit);
        }
        output.write_all(&self.0)?;
        Ok(())
    }
}

fn plan(bytes: &[u8], version: &str, suffix: &str) -> InstallPlan {
    let platform = Platform::current().unwrap();
    let entry: AgentEntry = serde_json::from_value(serde_json::json!({
        "id": "recovery-fixture",
        "name": "Recovery fixture",
        "version": version,
        "description": "Offline test bytes, never executed",
        "license_url": "https://example.test/license",
        "distribution": {"binary": {platform.key(): {
            "archive": format!("https://example.test/agent{suffix}"),
            "cmd": "agent",
            "sha256": hex::encode(Sha256::digest(bytes)),
            "args": ["acp"]
        }}}
    }))
    .unwrap();
    entry.plan(platform).unwrap()
}

fn assert_no_staging(store: &RegistryStore) {
    for item in fs::read_dir(store.root()).unwrap() {
        let item = item.unwrap();
        let name = item.file_name().to_string_lossy().into_owned();
        assert!(
            name == ".registry-lock" || name == "index.json" || name.starts_with("agent-"),
            "leftover temporary installer entry: {name}"
        );
    }
}

fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut builder = tar::Builder::new(encoder);
    for (name, bytes) in entries {
        // Set a raw short name to exercise paths that tar::Builder normally refuses
        // to construct. This is a hostile archive fixture, not an extraction helper.
        assert!(name.len() < 100);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(0o755);
        header.set_size(bytes.len() as u64);
        header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        header.set_cksum();
        builder.append(&header, Cursor::new(*bytes)).unwrap();
    }
    builder.into_inner().unwrap().finish().unwrap()
}

#[test]
fn partial_download_failure_keeps_previous_version_and_removes_staging() {
    struct Interrupted;
    impl Downloader for Interrupted {
        fn download(&self, _: &str, output: &mut dyn Write, _: u64) -> Result<()> {
            output.write_all(b"partial")?;
            Err(RegistryError::Network(
                "interrupted fixture download".into(),
            ))
        }
    }
    let root = tempfile::tempdir().unwrap();
    let store = RegistryStore::open(root.path()).unwrap();
    let old = store
        .install(&plan(b"old", "1.0.0", ""), &Bytes(b"old".to_vec()))
        .unwrap();
    let failed = store.install(&plan(b"new", "2.0.0", ""), &Interrupted);
    assert!(matches!(failed, Err(RegistryError::Network(_))));
    assert_eq!(store.installed().unwrap().len(), 1);
    assert_eq!(
        fs::read(old.reference.agent_spec().unwrap().launch.command).unwrap(),
        b"old"
    );
    assert_no_staging(&store);
    let new = store
        .install(&plan(b"new", "2.0.0", ""), &Bytes(b"new".to_vec()))
        .unwrap();
    assert_eq!(store.installed().unwrap().len(), 2);
    store.remove(&new.reference).unwrap();
    assert!(old.reference.agent_spec().is_ok());
}

#[test]
fn hostile_archive_paths_fail_before_publishing_a_new_version() {
    for name in [
        "../escape",
        "/absolute",
        "C:/escape",
        "a/../escape",
        "a\\..\\escape",
        "COM¹.exe",
        "LPT².txt",
        "a:stream",
        "bin/agent\u{202e}exe",
    ] {
        let root = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(root.path()).unwrap();
        let old = store
            .install(&plan(b"old", "1.0.0", ""), &Bytes(b"old".to_vec()))
            .unwrap();
        let bytes = archive(&[("agent", b"new"), (name, b"bad")]);
        assert!(
            store
                .install(&plan(&bytes, "2.0.0", ".tar.gz"), &Bytes(bytes))
                .is_err(),
            "accepted {name:?}"
        );
        assert_eq!(store.installed().unwrap().len(), 1);
        assert!(old.reference.agent_spec().is_ok());
        assert_no_staging(&store);
    }
}

#[test]
fn failure_after_a_valid_member_does_not_leave_a_partially_installed_agent() {
    let root = tempfile::tempdir().unwrap();
    let store = RegistryStore::open(root.path()).unwrap();
    let bytes = archive(&[("agent", b"first"), ("AGENT", b"duplicate")]);
    assert!(
        store
            .install(&plan(&bytes, "1.0.0", ".tgz"), &Bytes(bytes))
            .is_err()
    );
    assert!(store.installed().unwrap().is_empty());
    assert_no_staging(&store);
}

#[test]
fn renamed_installation_reference_cannot_revalidate_another_receipt() {
    let root = tempfile::tempdir().unwrap();
    let store = RegistryStore::open(root.path()).unwrap();
    let old = store
        .install(&plan(b"old", "1.0.0", ""), &Bytes(b"old".to_vec()))
        .unwrap();
    let new = store
        .install(&plan(b"new", "2.0.0", ""), &Bytes(b"new".to_vec()))
        .unwrap();
    let mut reference = old.reference.clone();
    reference.directory = new.reference.directory.clone();
    assert!(matches!(
        reference.agent_spec(),
        Err(RegistryError::Changed)
    ));
    assert!(matches!(
        store.remove(&reference),
        Err(RegistryError::Changed)
    ));
    assert!(old.reference.agent_spec().is_ok());
    assert!(new.reference.agent_spec().is_ok());
}

#[test]
fn concurrent_mutations_refuse_the_active_install_lock() {
    struct Paused {
        started: mpsc::SyncSender<()>,
        released: Mutex<mpsc::Receiver<()>>,
    }
    impl Downloader for Paused {
        fn download(&self, _: &str, output: &mut dyn Write, _: u64) -> Result<()> {
            self.started.send(()).unwrap();
            self.released
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .unwrap();
            output.write_all(b"new")?;
            Ok(())
        }
    }
    let root = tempfile::tempdir().unwrap();
    let store = RegistryStore::open(root.path()).unwrap();
    let old = store
        .install(&plan(b"old", "1.0.0", ""), &Bytes(b"old".to_vec()))
        .unwrap();
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker_store = store.clone();
    let handle = std::thread::spawn(move || {
        worker_store.install(
            &plan(b"new", "2.0.0", ""),
            &Paused {
                started: started_tx,
                released: Mutex::new(release_rx),
            },
        )
    });
    started_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let remove = store.remove(&old.reference);
    let refresh = store.refresh(&Bytes(br#"{"version":"1.0.0","agents":[]}"#.to_vec()));
    let install = store.install(&plan(b"other", "3.0.0", ""), &Bytes(b"other".to_vec()));
    // Release before assertions so a failed assertion does not strand the worker.
    release_tx.send(()).unwrap();
    let new = handle.join().unwrap().unwrap();
    assert!(matches!(remove, Err(RegistryError::Busy)));
    assert!(matches!(refresh, Err(RegistryError::Busy)));
    assert!(matches!(install, Err(RegistryError::Busy)));
    assert!(new.reference.agent_spec().is_ok());
    assert!(old.reference.agent_spec().is_ok());
    assert_eq!(store.installed().unwrap().len(), 2);
    assert_no_staging(&store);
}
