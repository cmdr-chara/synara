//! Explicit SSH trust and identity files for managed, non-interactive connections.
use crate::{
    ExecutionHost, LaunchSpec, LocalHost, RuntimeError, SpawnedProcess, SshHost, SshTarget,
};
use async_trait::async_trait;
use std::path::{Path, PathBuf};

/// Uses only the supplied trust store and identity, never ambient SSH configuration.
///
/// This is a transport boundary, not a remote filesystem or process sandbox.
/// The caller must obtain host keys through a separately trusted enrollment path.
#[derive(Clone, Debug)]
pub struct PinnedSshHost {
    target: SshTarget,
    known_hosts: PathBuf,
    identity_file: PathBuf,
}

impl PinnedSshHost {
    pub fn new(
        target: SshTarget,
        known_hosts: impl Into<PathBuf>,
        identity_file: impl Into<PathBuf>,
    ) -> Result<Self, RuntimeError> {
        target.validate()?;
        let host = Self {
            target,
            known_hosts: known_hosts.into(),
            identity_file: identity_file.into(),
        };
        host.connection_files()?;
        Ok(host)
    }

    fn connection_files(&self) -> Result<(String, String), RuntimeError> {
        let known_hosts = checked_file(&self.known_hosts, false)?;
        let identity = checked_file(&self.identity_file, true)?;
        if known_hosts == identity {
            return Err(RuntimeError::Invalid(
                "SSH identity and host trust must use separate files".into(),
            ));
        }
        Ok((known_hosts, identity))
    }

    pub fn command(&self, launch: &LaunchSpec, cwd: &Path) -> Result<LaunchSpec, RuntimeError> {
        // Revalidate on every spawn, not just when a settings/profile object is made.
        let (known_hosts, identity) = self.connection_files()?;
        let mut spec = SshHost {
            target: self.target.clone(),
        }
        .command(launch, cwd)?;
        let mut args = vec![
            "-F".into(),
            "none".into(),
            "-i".into(),
            identity,
            "-o".into(),
            format!("UserKnownHostsFile=\"{known_hosts}\""),
        ];
        for option in [
            "GlobalKnownHostsFile=none",
            "KnownHostsCommand=none",
            "VerifyHostKeyDNS=no",
            "UpdateHostKeys=no",
            "IdentitiesOnly=yes",
            "IdentityAgent=none",
            "AddKeysToAgent=no",
            "CanonicalizeHostname=no",
        ] {
            args.extend(["-o".into(), option.into()]);
        }
        args.append(&mut spec.args);
        spec.args = args;
        Ok(spec)
    }

    /// Build an interactive SSH transport whose local process is owned by Synara's
    /// native PTY. OpenSSH escape processing is disabled so terminal input cannot
    /// mutate connection state outside the reviewed Synara controls.
    pub fn pty_command(
        &self,
        launch: &LaunchSpec,
        cwd: &Path,
    ) -> Result<LaunchSpec, RuntimeError> {
        let mut spec = self.command(launch, cwd)?;
        let Some(no_tty) = spec.args.iter().position(|argument| argument == "-T") else {
            return Err(RuntimeError::Invalid(
                "SSH command is missing its non-interactive TTY guard".into(),
            ));
        };
        spec.args.remove(no_tty);
        let Some(request_tty) = spec
            .args
            .iter_mut()
            .find(|argument| argument.as_str() == "RequestTTY=no")
        else {
            return Err(RuntimeError::Invalid(
                "SSH command is missing its TTY policy".into(),
            ));
        };
        *request_tty = "RequestTTY=force".into();
        let boundary = spec
            .args
            .iter()
            .position(|argument| argument == "--")
            .ok_or_else(|| RuntimeError::Invalid("SSH command boundary is missing".into()))?;
        spec.args.splice(
            boundary..boundary,
            [
                "-tt".into(),
                "-o".into(),
                "EscapeChar=none".into(),
            ],
        );
        Ok(spec)
    }
}

#[async_trait]
impl ExecutionHost for PinnedSshHost {
    fn label(&self) -> String {
        SshHost {
            target: self.target.clone(),
        }
        .label()
    }

    fn is_local(&self) -> bool {
        false
    }

    async fn spawn(&self, launch: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError> {
        let spec = self.command(launch, cwd)?;
        LocalHost.spawn(&spec, &std::env::current_dir()?).await
    }
}

fn checked_file(path: &Path, private: bool) -> Result<String, RuntimeError> {
    if !path.is_absolute() {
        return Err(RuntimeError::Invalid(
            "SSH connection files must use absolute paths".into(),
        ));
    }
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(RuntimeError::Denied(
            "SSH connection file must be a regular non-symlink file".into(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let forbidden = if private { 0o077 } else { 0o022 };
        if metadata.permissions().mode() & forbidden != 0 {
            return Err(RuntimeError::Denied(
                "SSH connection file permissions are too permissive".into(),
            ));
        }
    }
    #[cfg(not(unix))]
    let _ = private;
    let canonical = path.canonicalize()?;
    let value = canonical
        .to_str()
        .ok_or_else(|| RuntimeError::Invalid("SSH connection path must be UTF-8".into()))?;
    // OpenSSH expands percent tokens and environment references even inside quotes.
    // Refuse those names rather than silently opening a different trust/identity file.
    if value
        .chars()
        .any(|c| c.is_control() || matches!(c, '%' | '$' | '"'))
    {
        return Err(RuntimeError::Invalid(
            "SSH connection path contains configuration expansion characters".into(),
        ));
    }
    #[cfg(windows)]
    let value = value.replace('\\', "/");
    #[cfg(not(windows))]
    let value = {
        if value.contains('\\') {
            return Err(RuntimeError::Invalid(
                "SSH connection path contains a configuration escape".into(),
            ));
        }
        value.to_owned()
    };
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let known = root.path().join("known hosts");
        let identity = root.path().join("identity");
        for path in [&known, &identity] {
            std::fs::write(path, "fixture, not a credential\n").unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
        }
        (root, known, identity)
    }

    fn target() -> SshTarget {
        SshTarget {
            host: "127.0.0.1".into(),
            port: 2222,
            user: Some("developer".into()),
        }
    }

    #[test]
    fn interactive_transport_disables_openssh_escapes() {
        let (_root, known, identity) = files();
        let host = PinnedSshHost::new(target(), &known, &identity).unwrap();
        let launch = host
            .pty_command(&LaunchSpec::new("/bin/sh"), Path::new("/project"))
            .unwrap();
        assert!(launch.args.iter().any(|argument| argument == "-tt"));
        assert!(
            launch
                .args
                .iter()
                .any(|argument| argument == "EscapeChar=none")
        );
        assert!(
            launch
                .args
                .iter()
                .any(|argument| argument == "RequestTTY=force")
        );
        assert!(!launch.args.iter().any(|argument| argument == "-T"));
        assert!(
            !launch
                .args
                .iter()
                .any(|argument| argument == "RequestTTY=no")
        );
    }

    #[test]
    fn pinned_connection_disables_ambient_trust_and_authentication() {
        let (_root, known, identity) = files();
        let host = PinnedSshHost::new(target(), &known, &identity).unwrap();
        let launch = host
            .command(&LaunchSpec::new("agent"), Path::new("/project"))
            .unwrap();
        for option in [
            "StrictHostKeyChecking=yes",
            "GlobalKnownHostsFile=none",
            "KnownHostsCommand=none",
            "VerifyHostKeyDNS=no",
            "UpdateHostKeys=no",
            "IdentityAgent=none",
            "IdentitiesOnly=yes",
        ] {
            assert!(launch.args.iter().any(|arg| arg == option), "{option}");
        }
        assert_eq!(&launch.args[..2], ["-F", "none"]);
        assert!(launch.args.iter().any(|arg| {
            arg.starts_with("UserKnownHostsFile=\"") && arg.ends_with("known hosts\"")
        }));
        assert!(!host.is_local());
    }

    #[test]
    fn connection_files_must_be_absolute_regular_and_distinct() {
        let (root, known, identity) = files();
        assert!(PinnedSshHost::new(target(), "relative", &identity).is_err());
        assert!(PinnedSshHost::new(target(), root.path(), &identity).is_err());
        assert!(PinnedSshHost::new(target(), &known, &known).is_err());
        assert!(PinnedSshHost::new(target(), root.path().join("missing"), &identity).is_err());
    }

    #[test]
    fn connection_files_are_rechecked_before_spawn() {
        let (_root, known, identity) = files();
        let host = PinnedSshHost::new(target(), &known, &identity).unwrap();
        std::fs::remove_file(&known).unwrap();
        assert!(
            host.command(&LaunchSpec::new("agent"), Path::new("/project"))
                .is_err()
        );
    }

    #[test]
    fn configuration_expansion_in_connection_paths_is_rejected() {
        let (root, known, identity) = files();
        for name in ["trust%h", "trust${HOME}"] {
            let expanded = root.path().join(name);
            std::fs::copy(&known, &expanded).unwrap();
            assert!(PinnedSshHost::new(target(), expanded, &identity).is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn loose_private_key_and_writable_trust_store_are_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let (_root, known, identity) = files();
        std::fs::set_permissions(&identity, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(PinnedSshHost::new(target(), &known, &identity).is_err());
        std::fs::set_permissions(&identity, std::fs::Permissions::from_mode(0o600)).unwrap();
        std::fs::set_permissions(&known, std::fs::Permissions::from_mode(0o666)).unwrap();
        assert!(PinnedSshHost::new(target(), &known, &identity).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_cannot_replace_a_selected_connection_file() {
        let (root, known, identity) = files();
        let host = PinnedSshHost::new(target(), &known, &identity).unwrap();
        let replacement = root.path().join("replacement");
        std::fs::rename(&known, &replacement).unwrap();
        std::os::unix::fs::symlink(replacement, &known).unwrap();
        assert!(
            host.command(&LaunchSpec::new("agent"), Path::new("/project"))
                .is_err()
        );
    }
}
