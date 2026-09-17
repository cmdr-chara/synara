use crate::{RuntimeError, SpawnedProcess, process::spawn_owned};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};
use tokio::process::Command;

#[derive(Clone, Default)]
pub struct LaunchSpec {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
}
impl std::fmt::Debug for LaunchSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LaunchSpec")
            .field("command", &self.command)
            .field("argument_count", &self.args.len())
            .field("environment_keys", &self.env.keys().collect::<Vec<_>>())
            .finish()
    }
}
impl LaunchSpec {
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self {
            command: command.into(),
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.command.as_os_str().is_empty() || self.args.len() > 256 || self.env.len() > 128 {
            return Err(RuntimeError::Invalid("invalid launch configuration".into()));
        }
        if self.command.to_string_lossy().contains('\0')
            || self
                .args
                .iter()
                .any(|a| a.contains('\0') || a.len() > 1024 * 1024)
        {
            return Err(RuntimeError::Invalid("invalid command argument".into()));
        }
        for (key, value) in &self.env {
            if !valid_env_key(key) || value.contains('\0') || value.len() > 1024 * 1024 {
                return Err(RuntimeError::Invalid("invalid environment entry".into()));
            }
        }
        Ok(())
    }
}
pub fn valid_env_key(key: &str) -> bool {
    !key.is_empty()
        && !key.as_bytes()[0].is_ascii_digit()
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

#[async_trait]
pub trait ExecutionHost: Send + Sync {
    fn label(&self) -> String;
    fn is_local(&self) -> bool;
    async fn spawn(&self, launch: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError>;
}

#[derive(Clone, Debug, Default)]
pub struct LocalHost;
#[async_trait]
impl ExecutionHost for LocalHost {
    fn label(&self) -> String {
        "Local".into()
    }
    fn is_local(&self) -> bool {
        true
    }
    async fn spawn(&self, launch: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError> {
        launch.validate()?;
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(RuntimeError::Invalid(
                "working directory must be an existing absolute directory".into(),
            ));
        }
        let mut command = Command::new(&launch.command);
        command
            .args(&launch.args)
            .current_dir(cwd)
            .env_clear()
            .envs(base_environment())
            .envs(&launch.env);
        spawn_owned(command)
    }
}

/// Do not inherit arbitrary credential variables into an external process.
pub fn base_environment() -> BTreeMap<OsString, OsString> {
    const KEYS: &[&str] = &[
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "SHELL",
        "TMPDIR",
        "TMP",
        "TEMP",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "TERM",
        "COLORTERM",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "SystemRoot",
        "SYSTEMROOT",
        "WINDIR",
        "PATHEXT",
        "COMSPEC",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "XDG_RUNTIME_DIR",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "DBUS_SESSION_BUS_ADDRESS",
        "SSH_AUTH_SOCK",
    ];
    KEYS.iter()
        .filter_map(|key| std::env::var_os(key).map(|value| (OsString::from(key), value)))
        .collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SshTarget {
    pub host: String,
    #[serde(default = "default_ssh_port")]
    pub port: u16,
    pub user: Option<String>,
}
fn default_ssh_port() -> u16 {
    22
}
impl SshTarget {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if self.host.is_empty()
            || self.host.starts_with('-')
            || self.port == 0
            || !self
                .host
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-:[]".contains(&b))
        {
            return Err(RuntimeError::Invalid("invalid SSH target".into()));
        }
        if self.user.as_ref().is_some_and(|u| {
            u.is_empty()
                || u.starts_with('-')
                || !u
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
        }) {
            return Err(RuntimeError::Invalid("invalid SSH user".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct SshHost {
    pub target: SshTarget,
}
impl SshHost {
    pub fn command(&self, launch: &LaunchSpec, cwd: &Path) -> Result<LaunchSpec, RuntimeError> {
        self.target.validate()?;
        launch.validate()?;
        let cwd = cwd
            .to_str()
            .ok_or_else(|| RuntimeError::Invalid("remote directory must be UTF-8".into()))?;
        if !cwd.starts_with('/') || cwd.contains('\0') {
            return Err(RuntimeError::Invalid(
                "remote directory must be an absolute POSIX path".into(),
            ));
        }
        if !launch.env.is_empty() {
            return Err(RuntimeError::Unsupported("configure remote credentials on the host rather than placing environment values in SSH arguments".into()));
        }
        let executable = launch
            .command
            .to_str()
            .ok_or_else(|| RuntimeError::Invalid("remote command must be UTF-8".into()))?;
        let command = std::iter::once(executable)
            .chain(launch.args.iter().map(String::as_str))
            .map(shell_quote)
            .collect::<Vec<_>>()
            .join(" ");
        let remote = format!("cd -- {} && exec {}", shell_quote(cwd), command);
        let mut args = vec![
            "-T".into(),
            "-o".into(),
            "BatchMode=yes".into(),
            "-o".into(),
            "StrictHostKeyChecking=yes".into(),
            "-o".into(),
            "ConnectTimeout=15".into(),
            "-o".into(),
            "ServerAliveInterval=30".into(),
            "-o".into(),
            "ServerAliveCountMax=3".into(),
            "-p".into(),
            self.target.port.to_string(),
        ];
        if let Some(user) = &self.target.user {
            args.extend(["-l".into(), user.clone()]);
        }
        args.extend(["--".into(), self.target.host.clone(), remote]);
        Ok(LaunchSpec {
            command: "ssh".into(),
            args,
            env: BTreeMap::new(),
        })
    }
}
#[async_trait]
impl ExecutionHost for SshHost {
    fn label(&self) -> String {
        format!(
            "SSH {}@{}:{}",
            self.target.user.as_deref().unwrap_or("default"),
            self.target.host,
            self.target.port
        )
    }
    fn is_local(&self) -> bool {
        false
    }
    async fn spawn(&self, launch: &LaunchSpec, cwd: &Path) -> Result<SpawnedProcess, RuntimeError> {
        let spec = self.command(launch, cwd)?;
        let local_cwd = std::env::current_dir()?;
        LocalHost.spawn(&spec, &local_cwd).await
    }
}

pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ssh_rejects_option_injection_and_preserves_literals() {
        let bad = SshTarget {
            host: "-oProxyCommand=evil".into(),
            port: 22,
            user: None,
        };
        assert!(bad.validate().is_err());
        let host = SshHost {
            target: SshTarget {
                host: "example.test".into(),
                port: 22,
                user: Some("dev".into()),
            },
        };
        let mut spec = LaunchSpec::new("agent");
        spec.args = vec!["a'; touch /tmp/x; '".into()];
        let command = host.command(&spec, Path::new("/srv/a b")).unwrap();
        assert!(command.args.contains(&"StrictHostKeyChecking=yes".into()));
        assert!(
            command
                .args
                .last()
                .unwrap()
                .contains("'a'\"'\"'; touch /tmp/x; '\"'\"''")
        );
    }
    #[test]
    fn launch_debug_never_contains_values_or_arguments() {
        let mut spec = LaunchSpec::new("agent");
        spec.env.insert("API_TOKEN".into(), "super-secret".into());
        spec.args.push("secret-argument".into());
        let debug = format!("{spec:?}");
        assert!(!debug.contains("super-secret"));
        assert!(!debug.contains("secret-argument"));
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn subprocess_exit_is_observable_and_reaped() {
        let mut spec = LaunchSpec::new("sh");
        spec.args = vec!["-c".into(), "printf hello; exit 7".into()];
        let mut process = LocalHost.spawn(&spec, Path::new("/tmp")).await.unwrap();
        let mut output = String::new();
        use tokio::io::AsyncReadExt;
        process.stdout.read_to_string(&mut output).await.unwrap();
        assert_eq!(output, "hello");
        assert_eq!(process.handle.wait().await.unwrap().code, Some(7));
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn explicit_shutdown_terminates_a_process_group() {
        let mut spec = LaunchSpec::new("sh");
        spec.args = vec!["-c".into(), "sleep 60 & wait".into()];
        let process = LocalHost.spawn(&spec, Path::new("/tmp")).await.unwrap();
        assert!(!process.handle.shutdown().await.unwrap().success());
    }
}
