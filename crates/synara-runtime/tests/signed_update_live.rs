//! Opt-in acceptance against a real Sigstore keyless bundle produced by GitHub Actions.
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::Command,
};
use synara_runtime::{RuntimeError, UpdateManifest, UpdateSignatureVerifier};

struct CosignBundleVerifier {
    identity: String,
    issuer: String,
}

impl UpdateSignatureVerifier for CosignBundleVerifier {
    fn verify(&self, manifest: &[u8], signature: &[u8]) -> Result<(), RuntimeError> {
        let root = tempfile::tempdir()?;
        let manifest_path = root.path().join("manifest.json");
        let bundle_path = root.path().join("manifest.bundle");
        fs::write(&manifest_path, manifest)?;
        fs::write(&bundle_path, signature)?;
        let status = Command::new("cosign")
            .arg("verify-blob")
            .arg("--bundle")
            .arg(&bundle_path)
            .arg("--certificate-identity")
            .arg(&self.identity)
            .arg("--certificate-oidc-issuer")
            .arg(&self.issuer)
            .arg(&manifest_path)
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(RuntimeError::Denied(
                "Sigstore rejected the update manifest identity or signature".into(),
            ))
        }
    }
}

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("missing {name}")))
}

fn required(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("missing {name}"))
}

fn sha256(path: &Path) -> String {
    let mut source = fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = source.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    hex::encode(digest.finalize())
}

#[test]
#[ignore = "requires a fresh GitHub OIDC Sigstore bundle and packaged release candidate"]
fn github_oidc_signed_update_installs_and_rolls_back() {
    assert_eq!(
        std::env::var("SYNARA_RELEASE_ACCEPTANCE").as_deref(),
        Ok("github-oidc")
    );
    let manifest_path = required_path("SYNARA_UPDATE_MANIFEST");
    let bundle_path = required_path("SYNARA_UPDATE_BUNDLE");
    let artifact_path = required_path("SYNARA_UPDATE_ARTIFACT");
    let platform = required("SYNARA_UPDATE_PLATFORM");
    let architecture = required("SYNARA_UPDATE_ARCH");
    let verifier = CosignBundleVerifier {
        identity: required("SYNARA_COSIGN_IDENTITY"),
        issuer: required("SYNARA_COSIGN_ISSUER"),
    };
    let manifest = fs::read(&manifest_path).unwrap();
    let bundle = fs::read(&bundle_path).unwrap();

    let update =
        UpdateManifest::verify_signed(&manifest, &bundle, &verifier, &platform, &architecture, 1)
            .unwrap();

    let mut tampered = manifest.clone();
    tampered.push(b' ');
    assert!(
        UpdateManifest::verify_signed(&tampered, &bundle, &verifier, &platform, &architecture, 1,)
            .is_err()
    );

    // A cryptographically valid bundle is insufficient without the exact
    // reviewed workflow identity and issuer.
    let wrong_identity = CosignBundleVerifier {
        identity: format!("{}-untrusted", verifier.identity),
        issuer: verifier.issuer.clone(),
    };
    assert!(wrong_identity.verify(&manifest, &bundle).is_err());
    let wrong_issuer = CosignBundleVerifier {
        identity: verifier.identity.clone(),
        issuer: "https://untrusted.invalid".into(),
    };
    assert!(wrong_issuer.verify(&manifest, &bundle).is_err());

    let root = tempfile::tempdir().unwrap();
    let current = root.path().join("current-package");
    let rollback = root.path().join("rollback-package");
    let staged = root.path().join("staged-package");
    let previous = b"previous signed release placeholder";
    fs::write(&current, previous).unwrap();

    // Keep the signed byte length but corrupt the download. A failed stage
    // must remove its partial destination and leave the installed bytes alone.
    let mut source = fs::File::open(&artifact_path).unwrap();
    let mut first = [0_u8; 1];
    source.read_exact(&mut first).unwrap();
    let corrupt = root.path().join("corrupt-package");
    let changed_prefix = std::io::Cursor::new([first[0] ^ 1]);
    assert!(
        update
            .stage(changed_prefix.chain(source), &corrupt)
            .is_err()
    );
    assert!(!corrupt.exists());
    assert_eq!(fs::read(&current).unwrap(), previous);

    let source = fs::File::open(&artifact_path).unwrap();
    update.stage(source, &staged).unwrap();

    // Replacement must never be authorized from bytes that changed after
    // staging, even if their path and size still match.
    let mut staged_file = fs::OpenOptions::new().write(true).open(&staged).unwrap();
    staged_file.write_all(&[first[0] ^ 1]).unwrap();
    staged_file.sync_all().unwrap();
    assert!(
        update
            .handoff(staged.clone(), current.clone(), rollback.clone())
            .is_err()
    );
    assert_eq!(fs::read(&current).unwrap(), previous);
    assert!(!rollback.exists());
    staged_file.seek(SeekFrom::Start(0)).unwrap();
    staged_file.write_all(&first).unwrap();
    staged_file.sync_all().unwrap();
    drop(staged_file);
    let handoff = update
        .handoff(staged.clone(), current.clone(), rollback.clone())
        .unwrap();
    assert_eq!(handoff.release_version, update.release_version());
    assert_eq!(handoff.expected_sha256, sha256(&artifact_path));

    fs::copy(&current, &rollback).unwrap();
    fs::remove_file(&current).unwrap();
    fs::rename(&staged, &current).unwrap();
    assert_eq!(sha256(&current), handoff.expected_sha256);
    assert_eq!(
        fs::metadata(&current).unwrap().len(),
        handoff.expected_byte_length
    );

    fs::remove_file(&current).unwrap();
    fs::rename(&rollback, &current).unwrap();
    assert_eq!(fs::read(&current).unwrap(), previous);
    assert!(!rollback.exists());

    if let Some(path) = std::env::var_os("SYNARA_RELEASE_EVIDENCE") {
        let document = serde_json::json!({
            "scope": "signed-staging-and-handoff-with-test-directory-replacement",
            "production_installer_accepted": false,
            "candidate_commit": std::env::var("GITHUB_SHA").ok(),
            "release_version": update.release_version(),
            "platform": update.platform(),
            "architecture": update.architecture(),
            "artifact_sha256": handoff.expected_sha256,
            "checks": [
                "sigstore-github-oidc-manifest-verification",
                "tampered-manifest-rejected",
                "wrong-workflow-identity-rejected",
                "wrong-oidc-issuer-rejected",
                "corrupt-artifact-rejected-and-partial-stage-removed",
                "staged-artifact-tampering-rejected-with-current-preserved",
                "runtime-artifact-staging",
                "runtime-handoff-reverification",
                "test-directory-package-replacement",
                "test-directory-rollback-restoration"
            ]
        });
        fs::write(path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();
    }
}
