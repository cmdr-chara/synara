//! Caller-verified update manifest validation and artifact staging.
//!
//! This module deliberately does not choose a signing identity, signature
//! algorithm, update endpoint, or release policy. The product owner supplies a
//! verifier for exact manifest bytes. This module contains no trusted key,
//! production feed, or platform installer.

use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MANIFEST_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: usize = 256 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 256;

pub trait UpdateSignatureVerifier: Send + Sync {
    /// Verify the opaque signature against the exact manifest bytes supplied by
    /// the update authority. Implementations choose the signature algorithm and
    /// trusted key outside this portable domain.
    fn verify(&self, manifest: &[u8], signature: &[u8]) -> Result<(), RuntimeError>;
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateArtifact {
    pub platform: String,
    pub architecture: String,
    pub byte_length: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateManifest {
    pub format_version: u32,
    pub release_version: String,
    pub min_data_schema: u32,
    pub max_data_schema: u32,
    pub artifact: UpdateArtifact,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedUpdate {
    // Private fields preserve the invariant that this value was produced only
    // after the caller-supplied verifier accepted the exact manifest bytes.
    release_version: String,
    artifact_byte_length: u64,
    artifact_sha256: [u8; 32],
    platform: String,
    architecture: String,
    current_data_schema: u32,
}

impl UpdateManifest {
    pub fn verify_signed(
        manifest_bytes: &[u8],
        signature: &[u8],
        verifier: &dyn UpdateSignatureVerifier,
        platform: &str,
        architecture: &str,
        current_data_schema: u32,
    ) -> Result<VerifiedUpdate, RuntimeError> {
        if manifest_bytes.is_empty() || manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(RuntimeError::Limit);
        }
        if signature.is_empty() || signature.len() > 64 * 1024 {
            return Err(RuntimeError::Invalid(
                "update signature is missing or oversized".into(),
            ));
        }
        verifier.verify(manifest_bytes, signature)?;
        let manifest: Self = serde_json::from_slice(manifest_bytes)
            .map_err(|_| RuntimeError::Invalid("invalid update manifest".into()))?;
        manifest.validate(platform, architecture, current_data_schema)
    }

    fn validate(
        &self,
        platform: &str,
        architecture: &str,
        current_data_schema: u32,
    ) -> Result<VerifiedUpdate, RuntimeError> {
        if self.format_version != MANIFEST_VERSION
            || !valid_label(&self.release_version)
            || !valid_label(&self.artifact.platform)
            || !valid_label(&self.artifact.architecture)
            || self.artifact.byte_length == 0
            || self.artifact.byte_length > MAX_ARTIFACT_BYTES
            || self.min_data_schema > self.max_data_schema
        {
            return Err(RuntimeError::Invalid(
                "invalid update manifest fields".into(),
            ));
        }
        if self.artifact.platform != platform || self.artifact.architecture != architecture {
            return Err(RuntimeError::Unsupported(
                "update artifact does not match this target".into(),
            ));
        }
        if current_data_schema < self.min_data_schema || current_data_schema > self.max_data_schema
        {
            return Err(RuntimeError::Unsupported(
                "update is incompatible with the current data schema".into(),
            ));
        }
        let digest = hex::decode(&self.artifact.sha256)
            .map_err(|_| RuntimeError::Invalid("invalid update artifact digest".into()))?;
        let digest: [u8; 32] = digest
            .try_into()
            .map_err(|_| RuntimeError::Invalid("invalid update artifact digest".into()))?;
        Ok(VerifiedUpdate {
            release_version: self.release_version.clone(),
            artifact_byte_length: self.artifact.byte_length,
            artifact_sha256: digest,
            platform: self.artifact.platform.clone(),
            architecture: self.artifact.architecture.clone(),
            current_data_schema,
        })
    }
}

impl VerifiedUpdate {
    pub fn release_version(&self) -> &str {
        &self.release_version
    }

    pub fn artifact_byte_length(&self) -> u64 {
        self.artifact_byte_length
    }

    pub fn artifact_sha256(&self) -> [u8; 32] {
        self.artifact_sha256
    }

    pub fn platform(&self) -> &str {
        &self.platform
    }

    pub fn architecture(&self) -> &str {
        &self.architecture
    }

    pub fn current_data_schema(&self) -> u32 {
        self.current_data_schema
    }

    /// Stage a verified download into a caller-chosen new path. The destination
    /// is never overwritten. Any incomplete or mismatched file is removed.
    pub fn stage<R: Read>(
        &self,
        mut reader: R,
        destination: &Path,
    ) -> Result<PathBuf, RuntimeError> {
        if !destination.is_absolute() {
            return Err(RuntimeError::Invalid(
                "update staging destination must be absolute".into(),
            ));
        }
        let parent = destination
            .parent()
            .ok_or_else(|| RuntimeError::Invalid("update staging parent is missing".into()))?;
        let parent_metadata = fs::symlink_metadata(parent)?;
        if !parent_metadata.is_dir() || parent_metadata.file_type().is_symlink() {
            return Err(RuntimeError::Denied(
                "update staging parent must be a real directory".into(),
            ));
        }
        if fs::symlink_metadata(destination).is_ok() {
            return Err(RuntimeError::Denied(
                "update staging destination already exists".into(),
            ));
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(fs::Permissions::from_mode(0o600))?;
        }
        let result = (|| -> Result<(), RuntimeError> {
            let mut hasher = Sha256::new();
            let mut total = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let count = reader.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                total = total.checked_add(count as u64).ok_or(RuntimeError::Limit)?;
                if total > self.artifact_byte_length || total > MAX_ARTIFACT_BYTES {
                    return Err(RuntimeError::Limit);
                }
                hasher.update(&buffer[..count]);
                file.write_all(&buffer[..count])?;
            }
            if total != self.artifact_byte_length {
                return Err(RuntimeError::Invalid(
                    "update artifact length does not match the signed manifest".into(),
                ));
            }
            let digest: [u8; 32] = hasher.finalize().into();
            if digest != self.artifact_sha256 {
                return Err(RuntimeError::Denied(
                    "update artifact digest does not match the signed manifest".into(),
                ));
            }
            file.sync_all()?;
            Ok(())
        })();
        if let Err(error) = result {
            drop(file);
            let _ = fs::remove_file(destination);
            return Err(error);
        }
        Ok(destination.to_path_buf())
    }

    pub fn handoff(
        &self,
        staged_artifact: PathBuf,
        current_executable: PathBuf,
        rollback_copy: PathBuf,
    ) -> Result<UpdateHandoff, RuntimeError> {
        for path in [&staged_artifact, &current_executable, &rollback_copy] {
            if !path.is_absolute() {
                return Err(RuntimeError::Invalid(
                    "update handoff paths must be absolute".into(),
                ));
            }
        }
        if staged_artifact == current_executable
            || staged_artifact == rollback_copy
            || current_executable == rollback_copy
        {
            return Err(RuntimeError::Invalid(
                "update handoff paths must be distinct".into(),
            ));
        }
        verify_artifact_file(
            &staged_artifact,
            self.artifact_byte_length,
            &self.artifact_sha256,
        )?;
        Ok(UpdateHandoff {
            release_version: self.release_version.clone(),
            staged_artifact,
            current_executable,
            rollback_copy,
            expected_byte_length: self.artifact_byte_length,
            expected_sha256: hex::encode(self.artifact_sha256),
            current_data_schema: self.current_data_schema,
        })
    }
}

/// Serializable install transaction. It contains no update URL, credential or
/// signing key. A launcher/helper may persist this value and execute it after
/// the application has exited on platforms that lock running executables.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateHandoff {
    pub release_version: String,
    pub staged_artifact: PathBuf,
    pub current_executable: PathBuf,
    pub rollback_copy: PathBuf,
    pub expected_byte_length: u64,
    pub expected_sha256: String,
    pub current_data_schema: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpdateInstallReceipt {
    pub release_version: String,
    pub installed_path: PathBuf,
    pub rollback_path: PathBuf,
}

impl UpdateHandoff {
    fn expected_digest(&self) -> Result<[u8; 32], RuntimeError> {
        if self.expected_byte_length == 0 || self.expected_byte_length > MAX_ARTIFACT_BYTES {
            return Err(RuntimeError::Invalid(
                "invalid update handoff length".into(),
            ));
        }
        let digest = hex::decode(&self.expected_sha256)
            .map_err(|_| RuntimeError::Invalid("invalid update handoff digest".into()))?;
        digest
            .try_into()
            .map_err(|_| RuntimeError::Invalid("invalid update handoff digest".into()))
    }

    fn validate_paths(&self) -> Result<(), RuntimeError> {
        for path in [
            &self.staged_artifact,
            &self.current_executable,
            &self.rollback_copy,
        ] {
            if !path.is_absolute() {
                return Err(RuntimeError::Invalid(
                    "update install paths must be absolute".into(),
                ));
            }
        }
        if self.staged_artifact == self.current_executable
            || self.staged_artifact == self.rollback_copy
            || self.current_executable == self.rollback_copy
        {
            return Err(RuntimeError::Invalid(
                "update install paths must be distinct".into(),
            ));
        }
        Ok(())
    }

    /// Replace the current executable/package with the previously verified
    /// staged artifact while retaining exactly one rollback copy.
    ///
    /// The transaction is intentionally rename-based: current and staged bytes
    /// must live on the same filesystem so replacement is never degraded into a
    /// partially copied executable. On Windows this method is expected to be
    /// called by the launcher/updater helper after the running executable exits.
    pub fn install(&self) -> Result<UpdateInstallReceipt, RuntimeError> {
        self.validate_paths()?;
        let digest = self.expected_digest()?;
        verify_artifact_file(
            &self.staged_artifact,
            self.expected_byte_length,
            &digest,
        )?;

        let current_metadata = fs::symlink_metadata(&self.current_executable)?;
        if !current_metadata.is_file() || current_metadata.file_type().is_symlink() {
            return Err(RuntimeError::Denied(
                "installed executable must be a regular non-symlink file".into(),
            ));
        }
        if fs::symlink_metadata(&self.rollback_copy).is_ok() {
            return Err(RuntimeError::Denied(
                "rollback destination already exists".into(),
            ));
        }

        // A staged file is created private. Before it becomes executable, copy
        // the currently installed permission bits/ACL-facing mode through the
        // portable permissions object.
        fs::set_permissions(
            &self.staged_artifact,
            current_metadata.permissions(),
        )?;

        fs::rename(&self.current_executable, &self.rollback_copy).map_err(|error| {
            RuntimeError::Io(error)
        })?;

        if let Err(error) = fs::rename(&self.staged_artifact, &self.current_executable) {
            if fs::rename(&self.rollback_copy, &self.current_executable).is_err() {
                return Err(RuntimeError::WriteOutcomeUnknown);
            }
            return Err(RuntimeError::Io(error));
        }

        if let Err(error) = verify_artifact_file(
            &self.current_executable,
            self.expected_byte_length,
            &digest,
        ) {
            let _ = fs::remove_file(&self.current_executable);
            if fs::rename(&self.rollback_copy, &self.current_executable).is_err() {
                return Err(RuntimeError::WriteOutcomeUnknown);
            }
            return Err(error);
        }

        sync_parent(&self.current_executable)?;
        Ok(UpdateInstallReceipt {
            release_version: self.release_version.clone(),
            installed_path: self.current_executable.clone(),
            rollback_path: self.rollback_copy.clone(),
        })
    }

    /// Restore the retained pre-update executable. The currently installed
    /// update is moved back to the staging path first, then removed only after
    /// rollback has been published successfully.
    pub fn rollback(&self) -> Result<PathBuf, RuntimeError> {
        self.validate_paths()?;
        if fs::symlink_metadata(&self.staged_artifact).is_ok() {
            return Err(RuntimeError::Denied(
                "rollback scratch path is not empty".into(),
            ));
        }
        let rollback_metadata = fs::symlink_metadata(&self.rollback_copy)?;
        let current_metadata = fs::symlink_metadata(&self.current_executable)?;
        if !rollback_metadata.is_file()
            || rollback_metadata.file_type().is_symlink()
            || !current_metadata.is_file()
            || current_metadata.file_type().is_symlink()
        {
            return Err(RuntimeError::Denied(
                "rollback requires regular installed and rollback files".into(),
            ));
        }

        fs::rename(&self.current_executable, &self.staged_artifact)?;
        if let Err(error) = fs::rename(&self.rollback_copy, &self.current_executable) {
            if fs::rename(&self.staged_artifact, &self.current_executable).is_err() {
                return Err(RuntimeError::WriteOutcomeUnknown);
            }
            return Err(RuntimeError::Io(error));
        }
        let _ = fs::remove_file(&self.staged_artifact);
        sync_parent(&self.current_executable)?;
        Ok(self.current_executable.clone())
    }
}

fn sync_parent(path: &Path) -> Result<(), RuntimeError> {
    let parent = path
        .parent()
        .ok_or_else(|| RuntimeError::Invalid("update path has no parent directory".into()))?;
    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()?;
    }
    #[cfg(not(unix))]
    {
        let _ = parent;
    }
    Ok(())
}

/// Re-reads staged bytes immediately before any install handoff or executable
/// replacement.
fn verify_artifact_file(
    path: &Path,
    expected_length: u64,
    expected_sha256: &[u8; 32],
) -> Result<(), RuntimeError> {
    let path_before = fs::symlink_metadata(path)?;
    if !path_before.is_file()
        || path_before.file_type().is_symlink()
        || path_before.len() != expected_length
    {
        return Err(RuntimeError::Denied(
            "staged update artifact is not the verified regular file".into(),
        ));
    }

    let mut file = OpenOptions::new().read(true).open(path)?;
    let opened_before = file.metadata()?;
    if !opened_before.is_file()
        || !same_file_entry(path, &file, &path_before, &opened_before)?
        || opened_before.len() != expected_length
        || opened_before.modified().ok() != path_before.modified().ok()
    {
        return Err(RuntimeError::Denied(
            "staged update artifact changed while it was opened".into(),
        ));
    }

    let mut digest = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        total = total.checked_add(count as u64).ok_or(RuntimeError::Limit)?;
        if total > expected_length || total > MAX_ARTIFACT_BYTES {
            return Err(RuntimeError::Limit);
        }
        digest.update(&buffer[..count]);
    }

    let opened_after = file.metadata()?;
    let path_after = fs::symlink_metadata(path)?;
    if total != expected_length
        || opened_after.len() != opened_before.len()
        || opened_after.modified().ok() != opened_before.modified().ok()
        || !path_after.is_file()
        || path_after.file_type().is_symlink()
        || !same_file_entry(path, &file, &path_after, &opened_after)?
        || path_after.len() != opened_after.len()
        || path_after.modified().ok() != opened_after.modified().ok()
    {
        return Err(RuntimeError::Denied(
            "staged update artifact changed while it was verified".into(),
        ));
    }
    let actual: [u8; 32] = digest.finalize().into();
    if &actual != expected_sha256 {
        return Err(RuntimeError::Denied(
            "staged update artifact digest does not match the signed manifest".into(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn same_file_entry(
    _path: &Path,
    _file: &std::fs::File,
    left: &std::fs::Metadata,
    right: &std::fs::Metadata,
) -> Result<bool, RuntimeError> {
    use std::os::unix::fs::MetadataExt;
    Ok(left.dev() == right.dev() && left.ino() == right.ino())
}

#[cfg(windows)]
fn same_file_entry(
    path: &Path,
    file: &std::fs::File,
    _left: &std::fs::Metadata,
    _right: &std::fs::Metadata,
) -> Result<bool, RuntimeError> {
    let opened = same_file::Handle::from_file(file.try_clone()?)?;
    let current = same_file::Handle::from_path(path)?;
    Ok(opened == current)
}

#[cfg(not(any(unix, windows)))]
fn same_file_entry(
    _path: &Path,
    _file: &std::fs::File,
    _left: &std::fs::Metadata,
    _right: &std::fs::Metadata,
) -> Result<bool, RuntimeError> {
    Ok(true)
}

fn valid_label(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_TEXT_BYTES && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    struct ExactVerifier {
        accepted: Vec<u8>,
    }

    impl UpdateSignatureVerifier for ExactVerifier {
        fn verify(&self, manifest: &[u8], signature: &[u8]) -> Result<(), RuntimeError> {
            if signature == self.accepted && !manifest.is_empty() {
                Ok(())
            } else {
                Err(RuntimeError::Denied(
                    "update signature was not accepted".into(),
                ))
            }
        }
    }

    fn signed(bytes: &[u8], schema: u32) -> (Vec<u8>, VerifiedUpdate) {
        let manifest = UpdateManifest {
            format_version: MANIFEST_VERSION,
            release_version: "0.2.0-dev".into(),
            min_data_schema: schema,
            max_data_schema: schema,
            artifact: UpdateArtifact {
                platform: "linux".into(),
                architecture: "x86_64".into(),
                byte_length: bytes.len() as u64,
                sha256: hex::encode(Sha256::digest(bytes)),
            },
        };
        let encoded = serde_json::to_vec(&manifest).unwrap();
        let verified = UpdateManifest::verify_signed(
            &encoded,
            b"accepted-signature",
            &ExactVerifier {
                accepted: b"accepted-signature".to_vec(),
            },
            "linux",
            "x86_64",
            schema,
        )
        .unwrap();
        (encoded, verified)
    }

    #[test]
    fn signature_is_checked_before_manifest_acceptance() {
        let bytes = b"artifact";
        let (manifest, _) = signed(bytes, 3);
        assert!(matches!(
            UpdateManifest::verify_signed(
                &manifest,
                b"wrong",
                &ExactVerifier {
                    accepted: b"accepted-signature".to_vec()
                },
                "linux",
                "x86_64",
                3
            ),
            Err(RuntimeError::Denied(_))
        ));
    }

    #[test]
    fn incompatible_schema_or_target_is_rejected() {
        let (manifest, _) = signed(b"artifact", 3);
        let verifier = ExactVerifier {
            accepted: b"accepted-signature".to_vec(),
        };
        assert!(matches!(
            UpdateManifest::verify_signed(
                &manifest,
                b"accepted-signature",
                &verifier,
                "windows",
                "x86_64",
                3
            ),
            Err(RuntimeError::Unsupported(_))
        ));
        assert!(matches!(
            UpdateManifest::verify_signed(
                &manifest,
                b"accepted-signature",
                &verifier,
                "linux",
                "x86_64",
                4
            ),
            Err(RuntimeError::Unsupported(_))
        ));
    }

    #[test]
    fn staging_is_no_clobber_and_cleans_failed_downloads() {
        let root = tempfile::tempdir().unwrap();
        let payload = b"verified artifact bytes";
        let (_, update) = signed(payload, 3);
        let destination = root.path().join("staged.bin");
        update.stage(Cursor::new(payload), &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), payload);
        assert!(update.stage(Cursor::new(payload), &destination).is_err());

        let bad = root.path().join("bad.bin");
        assert!(update.stage(Cursor::new(b"wrong"), &bad).is_err());
        assert!(!bad.exists());
    }

    #[test]
    fn handoff_requires_distinct_absolute_paths_and_contains_no_trust_material() {
        let root = tempfile::tempdir().unwrap();
        let staged = root.path().join("staged");
        let (_, update) = signed(b"artifact", 3);
        update.stage(Cursor::new(b"artifact"), &staged).unwrap();
        let current = root.path().join("current");
        let rollback = root.path().join("rollback");
        let handoff = update
            .handoff(staged.clone(), current.clone(), rollback.clone())
            .unwrap();
        let encoded = serde_json::to_string(&handoff).unwrap();
        assert!(encoded.contains("0.2.0-dev"));
        assert!(!encoded.contains("signature"));
        assert!(!encoded.contains("endpoint"));
        assert_eq!(handoff.expected_byte_length, b"artifact".len() as u64);
        assert!(update.handoff(staged.clone(), staged, rollback).is_err());
    }

    #[test]
    fn handoff_rechecks_staged_artifact_bytes_and_rejects_replacement() {
        let root = tempfile::tempdir().unwrap();
        let payload = b"verified artifact bytes";
        let (_, update) = signed(payload, 3);
        let staged = root.path().join("staged.bin");
        update.stage(Cursor::new(payload), &staged).unwrap();
        // Same-length replacement must reach and fail the digest check.
        fs::write(&staged, b"tampered artifact bytes").unwrap();

        assert!(
            update
                .handoff(
                    staged,
                    root.path().join("current"),
                    root.path().join("rollback"),
                )
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn handoff_rejects_a_symlinked_staging_path() {
        use std::os::unix::fs::symlink;

        let root = tempfile::tempdir().unwrap();
        let payload = b"verified artifact bytes";
        let (_, update) = signed(payload, 3);
        let staged = root.path().join("staged.bin");
        let target = root.path().join("target.bin");
        update.stage(Cursor::new(payload), &target).unwrap();
        symlink(&target, &staged).unwrap();

        assert!(
            update
                .handoff(
                    staged,
                    root.path().join("current"),
                    root.path().join("rollback"),
                )
                .is_err()
        );
    }
}
