//! File-backed server secret store, port of upstream auth/Layers/
//! ServerSecretStore.ts. Exact invariants: secrets live in a dedicated
//! directory chmodded 0700, file names are sanitized to [a-zA-Z0-9_.-], writes
//! are atomic (temp file + rename), and a missing secret reads as None rather
//! than an error. This is the foundation of the auth control plane: session
//! signing keys and pairing credentials are stored through it.

use std::path::PathBuf;

use crate::todo::state_dir;

fn secrets_dir() -> PathBuf {
    PathBuf::from(state_dir()).join("secrets")
}

fn secret_file_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("{sanitized}.bin")
}

fn ensure_dir() -> std::io::Result<PathBuf> {
    let dir = secrets_dir();
    std::fs::create_dir_all(&dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
    }
    Ok(dir)
}

/// Reads a secret by name; missing secrets are None, exactly like upstream.
pub fn get_secret(name: &str) -> Option<Vec<u8>> {
    let dir = ensure_dir().ok()?;
    std::fs::read(dir.join(secret_file_name(name))).ok()
}

/// Writes a secret atomically: temp file in the same directory then rename,
/// so a crash never leaves a half-written key.
pub fn put_secret(name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let dir = ensure_dir()?;
    let final_path = dir.join(secret_file_name(name));
    let tmp_path = dir.join(format!(".{}.tmp", secret_file_name(name)));
    std::fs::write(&tmp_path, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

/// Get-or-create for cryptographic keys: generates 32 random bytes when the
/// secret does not exist yet, mirroring how upstream bootstraps signing keys.
pub fn get_or_create_secret(name: &str) -> std::io::Result<Vec<u8>> {
    if let Some(existing) = get_secret(name) {
        return Ok(existing);
    }
    let bytes: Vec<u8> = (0..32).map(|_| rand_byte()).collect();
    put_secret(name, &bytes)?;
    Ok(bytes)
}

fn rand_byte() -> u8 {
    // Cryptographic randomness from the OS — /dev/urandom on every unix this
    // server targets. Signing keys must never come from a weaker source.
    use std::io::Read;
    let mut byte = [0u8; 1];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        if f.read_exact(&mut byte).is_ok() {
            return byte[0];
        }
    }
    // Fallback is unreachable in practice; panic rather than emit weak key
    // material silently.
    panic!("OS cryptographic randomness unavailable");
}

pub fn delete_secret(name: &str) -> std::io::Result<()> {
    let dir = ensure_dir()?;
    let path = dir.join(secret_file_name(name));
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_sanitizes() {
        assert_eq!(secret_file_name("session-key"), "session-key.bin");
        assert_eq!(secret_file_name("a/b\\c d"), "a_b_c_d.bin");
    }
}
