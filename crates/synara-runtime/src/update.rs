//! Authenticated update planning and artifact staging.
//!
//! This module deliberately does not choose a signing identity, signature
//! algorithm, update endpoint, or release policy. The product owner supplies a
//! verifier for exact manifest bytes.

use crate::RuntimeError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
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
    pub release_version: String,
    pub artifact_byte_length: u64,
    pub artifact_sha256: [u8; 32],
    pub platform: String,
    pub architecture: String,
    pub current_data_schema: u32,
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
            return Err(RuntimeError::Invalid("invalid update manifest fields".into()));
        }
        if self.artifact.platform != platform || self.artifact.architecture != architecture {
            return Err(RuntimeError::Unsupported(
                "update artifact does not match this target".into(),
            ));
        }
        if current_data_schema < self.min_data_schema || current_data_schema > self.max_data_schema {
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
    /// Stage a verified download into a caller-chosen new path. The destination
    /// is never overwritten. Any incomplete or mismatched file is removed.
    pub fn stage<R: Read>(&self, mut reader: R, destination: &Path) -> Result<PathBuf, RuntimeError> {
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
                total = total
                    .checked_add(count as u64)
                    .ok_or(RuntimeError::Limit)?;
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
        Ok(UpdateHandoff {
            release_version: self.release_version.clone(),
            staged_artifact,
            current_executable,
            rollback_copy,
            expected_sha256: hex::encode(self.artifact_sha256),
            current_data_schema: self.current_data_schema,
        })
    }
}

/// Serializable handoff to a future platform replacement helper. It contains no
/// update URL, credential or signing key.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateHandoff {
    pub release_version: String,
    pub staged_artifact: PathBuf,
    pub current_executable: PathBuf,
    pub rollback_copy: PathBuf,
    pub expected_sha256: String,
    pub current_data_schema: u32,
}

fn valid_label(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_TEXT_BYTES
        && !value.chars().any(char::is_control)
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
        update
            .stage(Cursor::new(payload), &destination)
            .unwrap();
        assert_eq!(fs::read(&destination).unwrap(), payload);
        assert!(update.stage(Cursor::new(payload), &destination).is_err());

        let bad = root.path().join("bad.bin");
        assert!(update.stage(Cursor::new(b"wrong"), &bad).is_err());
        assert!(!bad.exists());
    }

    #[test]
    fn handoff_requires_distinct_absolute_paths_and_contains_no_trust_material() {
        let root = tempfile::tempdir().unwrap();
        let (_, update) = signed(b"artifact", 3);
        let staged = root.path().join("staged");
        let current = root.path().join("current");
        let rollback = root.path().join("rollback");
        let handoff = update
            .handoff(staged.clone(), current.clone(), rollback.clone())
            .unwrap();
        let encoded = serde_json::to_string(&handoff).unwrap();
        assert!(encoded.contains("0.2.0-dev"));
        assert!(!encoded.contains("signature"));
        assert!(!encoded.contains("endpoint"));
        assert!(
            update
                .handoff(staged.clone(), staged, rollback)
                .is_err()
        );
    }
}
