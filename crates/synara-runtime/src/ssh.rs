//! Explicit SSH trust and identity files for managed, non-interactive connections.
use crate::{
    ExecutionHost, LaunchSpec, LocalHost, RuntimeError, SpawnedProcess, SshHost, SshTarget,
};
use async_trait::async_trait;
use std::{
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Uses only the supplied trust store and identity, never ambient SSH configuration.
///
/// This is a transport boundary, not a remote filesystem or process sandbox.
/// The caller must obtain host keys through a separately trusted enrollment path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ForwardApproval {
    Manual,
    Agent { interaction_id: String },
}

impl ForwardApproval {
    pub fn agent(interaction_id: impl Into<String>) -> Result<Self, RuntimeError> {
        let interaction_id = interaction_id.into();
        if interaction_id.is_empty()
            || interaction_id.len() > 256
            || interaction_id.chars().any(char::is_control)
        {
            return Err(RuntimeError::Invalid(
                "forward approval interaction ID is invalid".into(),
            ));
        }
        Ok(Self::Agent { interaction_id })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApprovedPortForward {
    local_port: u16,
    remote_port: u16,
    approval: ForwardApproval,
}

impl ApprovedPortForward {
    pub fn manual(local_port: u16, remote_port: u16) -> Result<Self, RuntimeError> {
        Self::new(local_port, remote_port, ForwardApproval::Manual)
    }

    pub fn agent_approved(
        local_port: u16,
        remote_port: u16,
        interaction_id: impl Into<String>,
    ) -> Result<Self, RuntimeError> {
        Self::new(
            local_port,
            remote_port,
            ForwardApproval::agent(interaction_id)?,
        )
    }

    fn new(
        local_port: u16,
        remote_port: u16,
        approval: ForwardApproval,
    ) -> Result<Self, RuntimeError> {
        if local_port == 0 || remote_port == 0 {
            return Err(RuntimeError::Invalid(
                "forwarded ports must be explicit non-zero TCP ports".into(),
            ));
        }
        Ok(Self {
            local_port,
            remote_port,
            approval,
        })
    }

    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn remote_port(&self) -> u16 {
        self.remote_port
    }

    pub fn approval(&self) -> &ForwardApproval {
        &self.approval
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevServerCandidate {
    pub scheme: String,
    pub remote_port: u16,
}

pub fn discover_loopback_dev_servers(output: &str) -> Result<Vec<DevServerCandidate>, RuntimeError> {
    if output.len() > 64 * 1024 {
        return Err(RuntimeError::Limit);
    }
    let mut candidates = Vec::new();
    for scheme in ["http", "https"] {
        for host in ["127.0.0.1", "localhost"] {
            let prefix = format!("{scheme}://{host}:");
            let mut remaining = output;
            while let Some(index) = remaining.find(&prefix) {
                let port_text = remaining[index + prefix.len()..]
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .take(5)
                    .collect::<String>();
                if let Ok(port) = port_text.parse::<u16>()
                    && port != 0
                    && !candidates.iter().any(|candidate: &DevServerCandidate| {
                        candidate.scheme == scheme && candidate.remote_port == port
                    })
                {
                    candidates.push(DevServerCandidate {
                        scheme: scheme.into(),
                        remote_port: port,
                    });
                    if candidates.len() >= 32 {
                        return Ok(candidates);
                    }
                }
                remaining = &remaining[index + prefix.len()..];
            }
        }
    }
    candidates.sort_by(|left, right| {
        left.remote_port
            .cmp(&right.remote_port)
            .then(left.scheme.cmp(&right.scheme))
    });
    Ok(candidates)
}

pub fn loopback_browser_url(
    local_port: u16,
    scheme: &str,
    path: &str,
) -> Result<String, RuntimeError> {
    if local_port == 0
        || !matches!(scheme, "http" | "https")
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.chars().any(char::is_control)
    {
        return Err(RuntimeError::Invalid(
            "browser handoff URL is not a safe loopback HTTP path".into(),
        ));
    }
    Ok(format!("{}://127.0.0.1:{}{}", scheme, local_port, path))
}

pub struct PortForward {
    handle: crate::ProcessHandle,
    local_port: u16,
}

impl PortForward {
    pub fn local_port(&self) -> u16 {
        self.local_port
    }

    pub fn browser_url(&self, scheme: &str, path: &str) -> Result<String, RuntimeError> {
        loopback_browser_url(self.local_port, scheme, path)
    }

    pub fn alive(&self) -> bool {
        self.handle.exit().is_none()
    }

    pub async fn close(self) -> Result<crate::ProcessExit, RuntimeError> {
        self.handle.shutdown().await
    }
}

impl Drop for PortForward {
    fn drop(&mut self) {
        self.handle.request_stop();
    }
}

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

    pub fn forward_command(
        &self,
        request: &ApprovedPortForward,
    ) -> Result<LaunchSpec, RuntimeError> {
        let (known_hosts, identity) = self.connection_files()?;
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
            "BatchMode=yes",
            "StrictHostKeyChecking=yes",
            "ConnectTimeout=15",
            "ServerAliveInterval=30",
            "ServerAliveCountMax=3",
            "ForwardAgent=no",
            "ForwardX11=no",
            "PermitLocalCommand=no",
            "ControlMaster=no",
            "ControlPath=none",
            "ControlPersist=no",
            "RequestTTY=no",
            "ForkAfterAuthentication=no",
            "ExitOnForwardFailure=yes",
            "GatewayPorts=no",
        ] {
            args.extend(["-o".into(), option.into()]);
        }
        args.extend([
            "-T".into(),
            "-N".into(),
            "-L".into(),
            format!(
                "127.0.0.1:{}:127.0.0.1:{}",
                request.local_port, request.remote_port
            ),
            "-p".into(),
            self.target.port.to_string(),
        ]);
        if let Some(user) = &self.target.user {
            args.extend(["-l".into(), user.clone()]);
        }
        args.extend(["--".into(), self.target.host.clone()]);
        Ok(LaunchSpec {
            command: "ssh".into(),
            args,
            env: Default::default(),
        })
    }

    pub async fn open_forward(
        &self,
        request: ApprovedPortForward,
    ) -> Result<PortForward, RuntimeError> {
        let local = SocketAddr::from((Ipv4Addr::LOCALHOST, request.local_port));
        tokio::task::spawn_blocking(move || TcpListener::bind(local))
            .await
            .map_err(|_| RuntimeError::Closed)??;

        let spec = self.forward_command(&request)?;
        let cwd = std::env::current_dir()?;
        let process = LocalHost.spawn(&spec, &cwd).await?;
        let handle = process.handle.clone();
        drop(process.stdin);
        tokio::spawn(async move {
            let mut output = process.stdout;
            let _ = tokio::io::copy(&mut output, &mut tokio::io::sink()).await;
        });
        tokio::spawn(async move {
            let mut diagnostic = process.stderr;
            let _ = tokio::io::copy(&mut diagnostic, &mut tokio::io::sink()).await;
        });
        let forward = PortForward {
            handle,
            local_port: request.local_port,
        };
        if let Err(error) = wait_forward_ready(&forward, local, Duration::from_secs(16)).await {
            let _ = forward.close().await;
            return Err(error);
        }
        Ok(forward)
    }

    /// Build an interactive SSH transport whose local process is owned by Synara's
    /// native PTY. OpenSSH escape processing is disabled so terminal input cannot
    /// mutate connection state outside the reviewed Synara controls.
    pub fn pty_command(&self, launch: &LaunchSpec, cwd: &Path) -> Result<LaunchSpec, RuntimeError> {
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
            ["-tt".into(), "-o".into(), "EscapeChar=none".into()],
        );
        Ok(spec)
    }
}

async fn wait_forward_ready(
    forward: &PortForward,
    local: SocketAddr,
    timeout: Duration,
) -> Result<(), RuntimeError> {
    let deadline = Instant::now() + timeout;
    loop {
        if forward.handle.exit().is_some() {
            return Err(RuntimeError::Invalid(
                "SSH port forwarding failed before becoming ready".into(),
            ));
        }
        let connected = tokio::task::spawn_blocking(move || {
            TcpStream::connect_timeout(&local, Duration::from_millis(100)).is_ok()
        })
        .await
        .map_err(|_| RuntimeError::Closed)?;
        if connected {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if forward.handle.exit().is_none() {
                return Ok(());
            }
            return Err(RuntimeError::Invalid(
                "SSH port forwarding exited during setup".into(),
            ));
        }
        if Instant::now() >= deadline {
            return Err(RuntimeError::Timeout);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
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
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        RuntimeError::Invalid(format!(
            "SSH {} file is unavailable at {}: {error}",
            if private { "identity" } else { "known-hosts" },
            path.display()
        ))
    })?;
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
    fn forwarding_is_explicit_loopback_only_and_disables_ambient_configuration() {
        let (_root, known, identity) = files();
        let host = PinnedSshHost::new(target(), &known, &identity).unwrap();
        let request = ApprovedPortForward::agent_approved(43123, 3000, "approval-1").unwrap();
        let launch = host.forward_command(&request).unwrap();
        assert_eq!(&launch.args[..2], ["-F", "none"]);
        assert!(launch.args.iter().any(|argument| argument == "-N"));
        assert!(launch.args.iter().any(|argument| argument == "-T"));
        assert!(
            launch
                .args
                .iter()
                .any(|argument| argument == "127.0.0.1:43123:127.0.0.1:3000")
        );
        for policy in [
            "ExitOnForwardFailure=yes",
            "GatewayPorts=no",
            "ForwardAgent=no",
            "ForwardX11=no",
            "ControlMaster=no",
        ] {
            assert!(launch.args.iter().any(|argument| argument == policy));
        }
        assert!(
            !launch
                .args
                .iter()
                .any(|argument| argument == "ClearAllForwardings=yes")
        );
        assert!(ApprovedPortForward::manual(0, 3000).is_err());
        assert!(ApprovedPortForward::agent_approved(4000, 3000, "").is_err());
    }

    #[test]
    fn dev_server_discovery_and_browser_handoff_accept_only_loopback_http() {
        let candidates = discover_loopback_dev_servers(
            "ready at http://localhost:3000/a and https://127.0.0.1:8443/\nhttp://example.com:9000",
        )
        .unwrap();
        assert_eq!(
            candidates,
            vec![
                DevServerCandidate {
                    scheme: "http".into(),
                    remote_port: 3000,
                },
                DevServerCandidate {
                    scheme: "https".into(),
                    remote_port: 8443,
                }
            ]
        );

        assert_eq!(
            loopback_browser_url(43123, "http", "/").unwrap(),
            "http://127.0.0.1:43123/"
        );
        assert!(loopback_browser_url(43123, "file", "/etc/passwd").is_err());
        assert!(loopback_browser_url(43123, "http", "//example.com").is_err());
        assert!(loopback_browser_url(0, "http", "/").is_err());
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
