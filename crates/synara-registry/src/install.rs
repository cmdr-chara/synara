use crate::{
    DOWNLOAD_LIMIT, Downloader, ENTRY_LIMIT, EXPANDED_LIMIT, FILE_LIMIT, INDEX_LIMIT, InstallPlan,
    Platform, REGISTRY_URL, Registry, RegistryError, Result, download::copy_limited,
    model::Delivery, paths,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use synara_agent::AgentSpec;
use synara_runtime::LaunchSpec;

const RECEIPT: &str = "receipt.json";
const RECEIPT_LIMIT: u64 = 128 * 1024;

/// Binds user approval to exact immutable receipt bytes, not just an executable path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryReference {
    pub directory: PathBuf,
    pub receipt_sha256: String,
}
#[derive(Clone, Debug)]
pub struct InstalledAgent {
    pub reference: RegistryReference,
    pub plan: InstallPlan,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    format: u32,
    plan: InstallPlan,
    executable_sha256: Option<String>,
}
#[derive(Clone, Debug)]
pub struct RegistryStore {
    root: PathBuf,
}
impl RegistryStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref();
        if !root.is_absolute() {
            return Err(RegistryError::Invalid(
                "agent storage must be absolute".into(),
            ));
        }
        fs::create_dir_all(root)?;
        private_directory(root)?;
        Ok(Self {
            root: root.canonicalize()?,
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    fn lock(&self) -> Result<File> {
        checked_directory(&self.root)?;
        let path = self.root.join(".registry-lock");
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_symlink() || !metadata.is_file() => {
                return Err(RegistryError::Changed);
            }
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.into()),
            _ => {}
        }
        let mut options = OpenOptions::new();
        options.create(true).truncate(false).read(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = options.open(path)?;
        file.try_lock().map_err(|_| RegistryError::Busy)?;
        Ok(file)
    }
    pub fn cached(&self) -> Result<Option<Registry>> {
        let path = self.root.join("index.json");
        if !path.try_exists()? {
            return Ok(None);
        }
        Ok(Some(Registry::parse(&read_regular(&path, INDEX_LIMIT)?)?))
    }
    pub fn refresh(&self, downloader: &dyn Downloader) -> Result<Registry> {
        let _lock = self.lock()?;
        let mut file = tempfile::NamedTempFile::new_in(&self.root)?;
        downloader.download(REGISTRY_URL, &mut file, INDEX_LIMIT)?;
        file.flush()?;
        file.seek(SeekFrom::Start(0))?;
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(INDEX_LIMIT + 1)
            .read_to_end(&mut bytes)?;
        let registry = Registry::parse(&bytes)?;
        file.as_file().sync_all()?;
        file.persist(self.root.join("index.json"))
            .map_err(|e| RegistryError::Io(e.error))?;
        Ok(registry)
    }
    pub fn install(
        &self,
        plan: &InstallPlan,
        downloader: &dyn Downloader,
    ) -> Result<InstalledAgent> {
        plan.validate()?;
        if plan.platform != Platform::current()? {
            return Err(RegistryError::Unsupported(
                "cannot install for a different operating system".into(),
            ));
        }
        let _lock = self.lock()?;
        let hash = hex::encode(Sha256::digest(serde_json::to_vec(plan)?));
        let directory = self.root.join(format!(
            "agent-{}-{}-{}",
            plan.id(),
            plan.version(),
            &hash[..16]
        ));
        if fs::symlink_metadata(&directory).is_ok() {
            return Err(RegistryError::Invalid(
                "this version is already installed. Remove it explicitly before reinstalling"
                    .into(),
            ));
        }
        let staging = tempfile::Builder::new()
            .prefix(".staging-")
            .tempdir_in(&self.root)?;
        private_directory(staging.path())?;
        let payload = staging.path().join("payload");
        fs::create_dir(&payload)?;
        private_directory(&payload)?;
        let executable_sha256 = match &plan.delivery {
            Delivery::Binary { target } => {
                let mut download = tempfile::NamedTempFile::new_in(&self.root)?;
                downloader.download(&target.archive, &mut download, DOWNLOAD_LIMIT)?;
                download.flush()?;
                if download.as_file().metadata()?.len() > DOWNLOAD_LIMIT {
                    return Err(RegistryError::Limit);
                }
                let expected = target.sha256.as_deref().ok_or(RegistryError::Checksum)?;
                download.seek(SeekFrom::Start(0))?;
                if hash_reader(download.as_file_mut())? != expected.to_ascii_lowercase() {
                    return Err(RegistryError::Checksum);
                }
                download.seek(SeekFrom::Start(0))?;
                extract(
                    download.as_file_mut(),
                    &target.archive,
                    &target.cmd,
                    &payload,
                )?;
                let executable = contained_file(&payload, &paths::relative_path(&target.cmd)?)?;
                let mut file = File::open(&executable)?;
                let digest = hash_reader(&mut file)?;
                executable_permissions(&executable, true)?;
                Some(digest)
            }
            Delivery::Npx { .. } | Delivery::Uvx { .. } => None,
        };
        let receipt = Receipt {
            format: 1,
            plan: plan.clone(),
            executable_sha256,
        };
        let bytes = serde_json::to_vec_pretty(&receipt)?;
        let reference = RegistryReference {
            directory: directory.clone(),
            receipt_sha256: hex::encode(Sha256::digest(&bytes)),
        };
        let mut file = File::create(staging.path().join(RECEIPT))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(staging.path(), &directory)?;
        Ok(InstalledAgent {
            reference,
            plan: plan.clone(),
        })
    }
    pub fn installed(&self) -> Result<Vec<InstalledAgent>> {
        checked_directory(&self.root)?;
        let mut installed = Vec::new();
        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;
            if !entry.file_name().to_string_lossy().starts_with("agent-") {
                continue;
            }
            if installed.len() >= 512 {
                return Err(RegistryError::Limit);
            }
            checked_directory(&entry.path())?;
            let bytes = read_regular(&entry.path().join(RECEIPT), RECEIPT_LIMIT)?;
            let reference = RegistryReference {
                directory: entry.path(),
                receipt_sha256: hex::encode(Sha256::digest(&bytes)),
            };
            let receipt = reference.receipt()?;
            installed.push(InstalledAgent {
                reference,
                plan: receipt.plan,
            });
        }
        installed.sort_by(|a, b| {
            a.plan
                .name()
                .cmp(b.plan.name())
                .then(a.plan.version().cmp(b.plan.version()))
        });
        Ok(installed)
    }
    /// Removes only an installation directly owned by this store. Package caches belong
    /// to npm/uv and are intentionally not deleted by Synara.
    pub fn remove(&self, reference: &RegistryReference) -> Result<()> {
        let _lock = self.lock()?;
        if reference.directory.parent() != Some(self.root.as_path()) {
            return Err(RegistryError::Invalid(
                "installation is outside this agent store".into(),
            ));
        }
        reference.receipt()?;
        fs::remove_dir_all(&reference.directory)?;
        Ok(())
    }
}
impl RegistryReference {
    pub fn validate(&self) -> Result<()> {
        if !self.directory.is_absolute()
            || self
                .directory
                .components()
                .any(|p| matches!(p, std::path::Component::ParentDir))
            || !paths::digest(&self.receipt_sha256)
        {
            return Err(RegistryError::Invalid(
                "invalid installed agent reference".into(),
            ));
        }
        Ok(())
    }
    fn receipt(&self) -> Result<Receipt> {
        self.validate()?;
        checked_directory(&self.directory)?;
        let bytes = read_regular(&self.directory.join(RECEIPT), RECEIPT_LIMIT)?;
        if hex::encode(Sha256::digest(&bytes)) != self.receipt_sha256.to_ascii_lowercase() {
            return Err(RegistryError::Changed);
        }
        let receipt: Receipt = serde_json::from_slice(&bytes)?;
        if receipt.format != 1 {
            return Err(RegistryError::Unsupported(
                "unknown installed agent metadata version".into(),
            ));
        }
        receipt.plan.validate()?;
        if receipt.plan.platform != Platform::current()? {
            return Err(RegistryError::Unsupported(
                "this installation belongs to another platform".into(),
            ));
        }
        Ok(receipt)
    }
    pub fn agent_spec(&self) -> Result<AgentSpec> {
        let receipt = self.receipt()?;
        let plan = &receipt.plan;
        let (command, args) = match &plan.delivery {
            Delivery::Binary { target } => {
                let payload = self.directory.join("payload");
                let path = contained_file(&payload, &paths::relative_path(&target.cmd)?)?;
                if fs::metadata(&path)?.len() > FILE_LIMIT {
                    return Err(RegistryError::Limit);
                }
                if Some(hash_reader(&mut File::open(&path)?)?) != receipt.executable_sha256 {
                    return Err(RegistryError::Changed);
                }
                (path, target.args.clone())
            }
            Delivery::Npx { target } => {
                let mut args = vec!["--yes".into(), "--".into(), target.package.clone()];
                args.extend(target.args.clone());
                (PathBuf::from("npx"), args)
            }
            Delivery::Uvx { target } => {
                let mut args = vec!["--".into(), target.package.clone()];
                args.extend(target.args.clone());
                (PathBuf::from("uvx"), args)
            }
        };
        let spec = AgentSpec {
            // Package runners must not discover node_modules or project configuration in
            // an untrusted repository. Session directories still come from the ACP host.
            launch_directory: Some(self.directory.clone()),
            id: format!("registry-{}", plan.id()),
            name: plan.name().into(),
            origin: plan.origin(),
            launch: LaunchSpec {
                command,
                args,
                env: plan.environment().clone(),
            },
        };
        spec.validate()
            .map_err(|e| RegistryError::Invalid(e.to_string()))?;
        Ok(spec)
    }
}
fn checked_directory(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(RegistryError::Changed);
    }
    Ok(())
}
fn private_directory(path: &Path) -> Result<()> {
    checked_directory(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}
fn executable_permissions(path: &Path, executable: bool) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(
            path,
            fs::Permissions::from_mode(if executable { 0o700 } else { 0o600 }),
        )?;
    }
    #[cfg(not(unix))]
    let _ = (path, executable);
    Ok(())
}
fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.is_symlink() {
        return Err(RegistryError::Changed);
    }
    if metadata.len() > limit {
        return Err(RegistryError::Limit);
    }
    let mut data = Vec::new();
    File::open(path)?.take(limit + 1).read_to_end(&mut data)?;
    if data.len() as u64 > limit {
        return Err(RegistryError::Limit);
    }
    Ok(data)
}
fn contained_file(root: &Path, relative: &Path) -> Result<PathBuf> {
    checked_directory(root)?;
    let mut path = root.to_owned();
    for part in relative.components() {
        if !matches!(part, std::path::Component::Normal(_)) {
            return Err(RegistryError::Changed);
        }
        path.push(part);
        if fs::symlink_metadata(&path)?.is_symlink() {
            return Err(RegistryError::Changed);
        }
    }
    if !fs::metadata(&path)?.is_file() || !path.canonicalize()?.starts_with(root.canonicalize()?) {
        return Err(RegistryError::Changed);
    }
    Ok(path)
}
fn hash_reader(reader: &mut impl Read) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0; 32 * 1024];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}
struct Extractor<'a> {
    root: &'a Path,
    seen: BTreeSet<String>,
    bytes: u64,
}
impl<'a> Extractor<'a> {
    fn entry(
        &mut self,
        name: &str,
        directory: bool,
        size: u64,
        executable: bool,
        reader: impl Read,
    ) -> Result<()> {
        let relative = paths::relative_path(name)?;
        let key = relative.to_string_lossy().replace('\\', "/").to_lowercase();
        if self.seen.len() >= ENTRY_LIMIT || !self.seen.insert(key) {
            return Err(RegistryError::Invalid(
                "duplicate archive member or too many entries".into(),
            ));
        }
        if size > FILE_LIMIT || size > EXPANDED_LIMIT.saturating_sub(self.bytes) {
            return Err(RegistryError::Limit);
        }
        let path = self.root.join(relative);
        let parent = path.parent().ok_or(RegistryError::Changed)?;
        fs::create_dir_all(parent)?;
        if directory {
            fs::create_dir_all(&path)?;
            private_directory(&path)?;
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)?;
        let copied = copy_limited(reader, &mut file, size)?;
        if copied != size {
            return Err(RegistryError::Invalid(
                "archive member size mismatch".into(),
            ));
        }
        file.sync_all()?;
        self.bytes += copied;
        executable_permissions(&path, executable)?;
        Ok(())
    }
}
fn extract(file: &mut File, uri: &str, cmd: &str, destination: &Path) -> Result<()> {
    let url = crate::model::https_url(uri)?;
    let name = url.path().to_ascii_lowercase();
    let mut extractor = Extractor {
        root: destination,
        seen: BTreeSet::new(),
        bytes: 0,
    };
    if name.ends_with(".zip") {
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|_| RegistryError::Invalid("invalid ZIP archive".into()))?;
        if archive.len() > ENTRY_LIMIT {
            return Err(RegistryError::Limit);
        }
        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|_| RegistryError::Invalid("encrypted or invalid ZIP member".into()))?;
            let mode = entry.unix_mode().unwrap_or(0);
            if !matches!(mode & 0o170000, 0 | 0o040000 | 0o100000) {
                return Err(RegistryError::Invalid(
                    "ZIP links and special files are not allowed".into(),
                ));
            }
            let path = entry.name().to_owned();
            extractor.entry(
                &path,
                entry.is_dir(),
                entry.size(),
                mode & 0o111 != 0,
                &mut entry,
            )?;
        }
    } else if [".tar.gz", ".tgz", ".tar.bz2", ".tbz2"]
        .iter()
        .any(|ext| name.ends_with(ext))
    {
        let decoder: Box<dyn Read + '_> = if name.ends_with(".bz2") || name.ends_with(".tbz2") {
            Box::new(bzip2::read::BzDecoder::new(file))
        } else {
            Box::new(flate2::read::GzDecoder::new(file))
        };
        let mut archive = tar::Archive::new(decoder.take(EXPANDED_LIMIT + 1));
        for (index, entry) in archive.entries()?.enumerate() {
            if index >= ENTRY_LIMIT {
                return Err(RegistryError::Limit);
            }
            let mut entry = entry?;
            let kind = entry.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err(RegistryError::Invalid(
                    "TAR links and special files are not allowed".into(),
                ));
            }
            let path = entry
                .path()?
                .to_str()
                .ok_or_else(|| RegistryError::Invalid("archive paths must be UTF-8".into()))?
                .to_owned();
            // A conventional root directory header does not name a destination member.
            if kind.is_dir() && matches!(path.as_str(), "." | "./") {
                continue;
            }
            let size = entry.size();
            let executable = entry.header().mode()? & 0o111 != 0;
            extractor.entry(&path, kind.is_dir(), size, executable, &mut entry)?;
        }
    } else {
        if [
            ".dmg",
            ".pkg",
            ".deb",
            ".rpm",
            ".msi",
            ".appimage",
            ".tar",
            ".xz",
            ".gz",
            ".bz2",
            ".7z",
        ]
        .iter()
        .any(|ext| name.ends_with(ext))
        {
            return Err(RegistryError::Unsupported(
                "installer or archive format is not supported".into(),
            ));
        }
        let size = file.metadata()?.len();
        extractor.entry(cmd, false, size, true, file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentEntry, BinaryTarget, Distribution};
    use std::{collections::BTreeMap, io::Cursor};
    struct Bytes(Vec<u8>);
    impl Downloader for Bytes {
        fn download(&self, _: &str, w: &mut dyn Write, limit: u64) -> Result<()> {
            copy_limited(Cursor::new(&self.0), w, limit)?;
            Ok(())
        }
    }
    fn plan(bytes: &[u8], suffix: &str) -> InstallPlan {
        let mut binary = BTreeMap::new();
        binary.insert(
            Platform::current().unwrap().key().into(),
            BinaryTarget {
                archive: format!("https://example.com/agent{suffix}"),
                cmd: "agent".into(),
                sha256: Some(hex::encode(Sha256::digest(bytes))),
                args: vec!["acp".into()],
                env: BTreeMap::new(),
            },
        );
        AgentEntry {
            id: "fixture".into(),
            name: "Fixture".into(),
            version: "1.0.0".into(),
            description: "Test executable".into(),
            license: Some("MIT".into()),
            license_url: Some("https://example.com/license".into()),
            repository: None,
            website: None,
            distribution: Distribution {
                binary,
                ..Default::default()
            },
        }
        .plan(Platform::current().unwrap())
        .unwrap()
    }
    fn tar(entries: &[(&str, &[u8], tar::EntryType)]) -> Vec<u8> {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        for (path, content, kind) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_entry_type(*kind);
            header.set_size(content.len() as u64);
            header.set_mode(0o755);
            if kind.is_symlink() || kind.is_hard_link() {
                header.set_link_name("/tmp/escape").unwrap();
            }
            header.set_cksum();
            builder.append_data(&mut header, path, *content).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap()
    }
    #[test]
    fn verified_binary_install_launch_and_remove_are_explicit() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        let bytes = b"independent test bytes, never executed";
        let installed = store
            .install(&plan(bytes, ""), &Bytes(bytes.to_vec()))
            .unwrap();
        let spec = installed.reference.agent_spec().unwrap();
        assert_eq!(spec.id, "registry-fixture");
        assert_eq!(spec.launch.args, vec!["acp"]);
        assert_eq!(fs::read(spec.launch.command).unwrap(), bytes);
        assert_eq!(store.installed().unwrap().len(), 1);
        assert!(
            store
                .install(&plan(bytes, ""), &Bytes(bytes.to_vec()))
                .is_err()
        );
        store.remove(&installed.reference).unwrap();
        assert!(store.installed().unwrap().is_empty());
    }
    #[test]
    fn checksum_mismatch_never_publishes_an_installation() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        assert!(matches!(
            store.install(&plan(b"good", ""), &Bytes(b"bad".to_vec())),
            Err(RegistryError::Checksum)
        ));
        assert!(store.installed().unwrap().is_empty());
    }
    #[test]
    fn tar_files_are_extracted_but_links_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        for kind in [
            tar::EntryType::Symlink,
            tar::EntryType::Link,
            tar::EntryType::Fifo,
        ] {
            let bytes = tar(&[("agent", b"", kind)]);
            assert!(
                store
                    .install(&plan(&bytes, ".tar.gz"), &Bytes(bytes))
                    .is_err()
            );
            assert!(store.installed().unwrap().is_empty());
        }
        let bytes = tar(&[("agent", b"test", tar::EntryType::Regular)]);
        assert!(
            store
                .install(&plan(&bytes, ".tar.gz"), &Bytes(bytes))
                .is_ok()
        );
    }
    #[test]
    fn duplicate_case_collisions_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        let bytes = tar(&[
            ("agent", b"a", tar::EntryType::Regular),
            ("AGENT", b"b", tar::EntryType::Regular),
        ]);
        assert!(store.install(&plan(&bytes, ".tgz"), &Bytes(bytes)).is_err());
        assert!(store.installed().unwrap().is_empty());
    }
    #[test]
    fn tampered_executable_and_receipt_are_not_launched() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        let installed = store
            .install(&plan(b"original", ""), &Bytes(b"original".to_vec()))
            .unwrap();
        let command = installed.reference.agent_spec().unwrap().launch.command;
        fs::write(command, b"tampered").unwrap();
        assert!(matches!(
            installed.reference.agent_spec(),
            Err(RegistryError::Changed)
        ));
        fs::write(installed.reference.directory.join(RECEIPT), b"{}").unwrap();
        assert!(matches!(
            installed.reference.agent_spec(),
            Err(RegistryError::Changed)
        ));
    }
    #[test]
    fn package_registration_is_pinned_and_does_not_download_or_execute() {
        struct Never;
        impl Downloader for Never {
            fn download(&self, _: &str, _: &mut dyn Write, _: u64) -> Result<()> {
                panic!("package registration must not execute or download")
            }
        }
        let entry:AgentEntry=serde_json::from_value(serde_json::json!({"id":"sample","name":"Sample","version":"1.2.3","description":"Test","license_url":"https://example.com/license","distribution":{"npx":{"package":"@sample/agent@1.2.3","args":["--acp","argument with spaces"]}}})).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        let installed = store
            .install(&entry.plan(Platform::current().unwrap()).unwrap(), &Never)
            .unwrap();
        assert_eq!(
            installed.reference.agent_spec().unwrap().launch.args,
            vec![
                "--yes",
                "--",
                "@sample/agent@1.2.3",
                "--acp",
                "argument with spaces"
            ]
        );
    }
    #[test]
    fn extraction_limits_are_checked_before_writing() {
        let dir = tempfile::tempdir().unwrap();
        let mut extractor = Extractor {
            root: dir.path(),
            seen: BTreeSet::new(),
            bytes: 0,
        };
        assert!(matches!(
            extractor.entry("huge", false, FILE_LIMIT + 1, false, Cursor::new([])),
            Err(RegistryError::Limit)
        ));
        assert!(!dir.path().join("huge").exists());
    }
    #[test]
    fn removal_cannot_target_another_store() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        let installed = RegistryStore::open(a.path())
            .unwrap()
            .install(&plan(b"x", ""), &Bytes(b"x".to_vec()))
            .unwrap();
        assert!(
            RegistryStore::open(b.path())
                .unwrap()
                .remove(&installed.reference)
                .is_err()
        );
        assert!(installed.reference.agent_spec().is_ok());
    }
    #[test]
    fn invalid_refresh_preserves_the_previous_cache() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        let good = br#"{"version":"1.0.0","agents":[]}"#;
        store.refresh(&Bytes(good.to_vec())).unwrap();
        assert!(store.refresh(&Bytes(b"bad json".to_vec())).is_err());
        assert_eq!(store.cached().unwrap().unwrap().version, "1.0.0");
    }
    #[cfg(unix)]
    #[test]
    fn installed_path_symlinks_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path()).unwrap();
        let installed = store
            .install(&plan(b"x", ""), &Bytes(b"x".to_vec()))
            .unwrap();
        let exe = installed.reference.directory.join("payload/agent");
        fs::remove_file(&exe).unwrap();
        std::os::unix::fs::symlink("/bin/sh", exe).unwrap();
        assert!(installed.reference.agent_spec().is_err());
    }
    #[test]
    fn zip_traversal_and_symlinks_cannot_leave_the_staging_directory() {
        for (name, link) in [
            ("../outside", false),
            ("C:/outside", false),
            ("agent", true),
        ] {
            let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
            let options = zip::write::SimpleFileOptions::default();
            if link {
                zip.add_symlink(name, "../outside", options).unwrap();
            } else {
                zip.start_file(name, options).unwrap();
                zip.write_all(b"test").unwrap();
            }
            let bytes = zip.finish().unwrap().into_inner();
            let dir = tempfile::tempdir().unwrap();
            let store = RegistryStore::open(dir.path().join("owned")).unwrap();
            assert!(store.install(&plan(&bytes, ".zip"), &Bytes(bytes)).is_err());
            assert!(!dir.path().join("outside").exists());
            assert!(store.installed().unwrap().is_empty());
        }
    }
    #[test]
    fn regular_zip_and_bzip_tar_distributions_are_supported() {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file("agent", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"test").unwrap();
        let zip_bytes = zip.finish().unwrap().into_inner();
        let encoder = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, "agent", &b"test"[..])
            .unwrap();
        let bz_bytes = builder.into_inner().unwrap().finish().unwrap();
        for (bytes, ext) in [(zip_bytes, ".zip"), (bz_bytes, ".tar.bz2")] {
            let dir = tempfile::tempdir().unwrap();
            let store = RegistryStore::open(dir.path()).unwrap();
            let installed = store.install(&plan(&bytes, ext), &Bytes(bytes)).unwrap();
            assert_eq!(
                fs::read(installed.reference.agent_spec().unwrap().launch.command).unwrap(),
                b"test"
            );
        }
    }
    #[test]
    fn raw_tar_header_traversal_is_rejected() {
        let encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        let mut builder = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_size(4);
        header.set_mode(0o755);
        header.as_mut_bytes()[..10].copy_from_slice(b"../outside");
        header.set_cksum();
        builder.append(&header, &b"test"[..]).unwrap();
        let bytes = builder.into_inner().unwrap().finish().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path().join("owned")).unwrap();
        assert!(
            store
                .install(&plan(&bytes, ".tar.gz"), &Bytes(bytes))
                .is_err()
        );
        assert!(!dir.path().join("outside").exists());
    }
    #[test]
    fn missing_executable_and_installers_do_not_publish_receipts() {
        for (bytes, ext) in [
            (tar(&[("other", b"x", tar::EntryType::Regular)]), ".tgz"),
            (b"fake installer".to_vec(), ".msi"),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let store = RegistryStore::open(dir.path()).unwrap();
            assert!(store.install(&plan(&bytes, ext), &Bytes(bytes)).is_err());
            assert!(store.installed().unwrap().is_empty());
        }
    }
    #[test]
    fn installation_lock_serializes_independent_store_instances() {
        let dir = tempfile::tempdir().unwrap();
        let one = RegistryStore::open(dir.path()).unwrap();
        let two = RegistryStore::open(dir.path()).unwrap();
        let guard = one.lock().unwrap();
        assert!(matches!(
            two.install(&plan(b"x", ""), &Bytes(b"x".to_vec())),
            Err(RegistryError::Busy)
        ));
        drop(guard);
        assert!(two.install(&plan(b"x", ""), &Bytes(b"x".to_vec())).is_ok());
    }
    #[cfg(unix)]
    #[test]
    fn dangling_lock_symlinks_do_not_create_outside_files() {
        let dir = tempfile::tempdir().unwrap();
        let store = RegistryStore::open(dir.path().join("owned")).unwrap();
        std::os::unix::fs::symlink(
            dir.path().join("outside"),
            store.root().join(".registry-lock"),
        )
        .unwrap();
        assert!(
            store
                .install(&plan(b"x", ""), &Bytes(b"x".to_vec()))
                .is_err()
        );
        assert!(!dir.path().join("outside").exists());
    }
}
