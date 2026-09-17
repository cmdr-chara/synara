use crate::{INDEX_LIMIT, RegistryError, Result, paths};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use synara_runtime::valid_env_key;
use url::Url;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    #[serde(rename = "darwin-aarch64")]
    MacArm,
    #[serde(rename = "darwin-x86_64")]
    MacIntel,
    #[serde(rename = "linux-aarch64")]
    LinuxArm,
    #[serde(rename = "linux-x86_64")]
    LinuxIntel,
    #[serde(rename = "windows-aarch64")]
    WindowsArm,
    #[serde(rename = "windows-x86_64")]
    WindowsIntel,
}
impl Platform {
    pub fn current() -> Result<Self> {
        match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => Ok(Self::MacArm),
            ("macos", "x86_64") => Ok(Self::MacIntel),
            ("linux", "aarch64") => Ok(Self::LinuxArm),
            ("linux", "x86_64") => Ok(Self::LinuxIntel),
            ("windows", "aarch64") => Ok(Self::WindowsArm),
            ("windows", "x86_64") => Ok(Self::WindowsIntel),
            _ => Err(RegistryError::Unsupported(
                "this platform is not in the registry".into(),
            )),
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::MacArm => "darwin-aarch64",
            Self::MacIntel => "darwin-x86_64",
            Self::LinuxArm => "linux-aarch64",
            Self::LinuxIntel => "linux-x86_64",
            Self::WindowsArm => "windows-aarch64",
            Self::WindowsIntel => "windows-x86_64",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Registry {
    pub version: String,
    pub agents: Vec<AgentEntry>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub website: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub license_url: Option<String>,
    pub distribution: Distribution,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Distribution {
    #[serde(default)]
    pub binary: BTreeMap<String, BinaryTarget>,
    #[serde(default)]
    pub npx: Option<PackageTarget>,
    #[serde(default)]
    pub uvx: Option<PackageTarget>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BinaryTarget {
    pub archive: String,
    pub cmd: String,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageTarget {
    pub package: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Delivery {
    Binary { target: BinaryTarget },
    Npx { target: PackageTarget },
    Uvx { target: PackageTarget },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstallPlan {
    pub(crate) entry: AgentEntry,
    pub(crate) platform: Platform,
    pub(crate) delivery: Delivery,
}
impl Registry {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        if bytes.len() as u64 > INDEX_LIMIT {
            return Err(RegistryError::Limit);
        }
        let registry: Self = serde_json::from_slice(bytes)?;
        if registry.version != "1.0.0" {
            return Err(RegistryError::Unsupported(
                "unknown registry schema version".into(),
            ));
        }
        if registry.agents.len() > 2048 {
            return Err(RegistryError::Limit);
        }
        let mut ids = BTreeSet::new();
        for entry in &registry.agents {
            entry.validate()?;
            if !ids.insert(&entry.id) {
                return Err(RegistryError::Invalid("duplicate agent ID".into()));
            }
        }
        Ok(registry)
    }
}
impl AgentEntry {
    pub fn version_key(&self) -> Result<[u64; 3]> {
        version_key(&self.version)
    }
    fn validate(&self) -> Result<()> {
        if !paths::id(&self.id) || !label(&self.name, 160) || !label(&self.description, 8192) {
            return Err(RegistryError::Invalid(
                "invalid agent identity or description".into(),
            ));
        }
        self.version_key()?;
        if self.id != "dimcode" && self.license_url.is_none() {
            return Err(RegistryError::Invalid(
                "license or terms URL missing".into(),
            ));
        }
        for uri in [&self.repository, &self.website, &self.license_url]
            .into_iter()
            .flatten()
        {
            metadata_url(uri)?;
        }
        if self.license.as_ref().is_some_and(|s| !label(s, 128)) {
            return Err(RegistryError::Invalid("invalid license label".into()));
        }
        let dist = &self.distribution;
        if dist.binary.len() > 6
            || (dist.binary.is_empty() && dist.npx.is_none() && dist.uvx.is_none())
        {
            return Err(RegistryError::Invalid(
                "missing or oversized distribution".into(),
            ));
        }
        for (platform, target) in &dist.binary {
            if !matches!(
                platform.as_str(),
                "darwin-aarch64"
                    | "darwin-x86_64"
                    | "linux-aarch64"
                    | "linux-x86_64"
                    | "windows-aarch64"
                    | "windows-x86_64"
            ) {
                return Err(RegistryError::Invalid("unknown binary platform".into()));
            }
            https_url(&target.archive)?;
            paths::relative_path(&target.cmd)?;
            if target.sha256.as_ref().is_some_and(|s| !paths::digest(s)) {
                return Err(RegistryError::Invalid("invalid checksum".into()));
            }
            arguments(&target.args, &target.env)?;
        }
        for target in [&dist.npx, &dist.uvx].into_iter().flatten() {
            if !label(&target.package, 256) {
                return Err(RegistryError::Invalid("invalid package name".into()));
            }
            arguments(&target.args, &target.env)?;
        }
        Ok(())
    }
    /// Prefer verified binaries, then pinned packages. Missing checksums never mean approval.
    pub fn plan(&self, platform: Platform) -> Result<InstallPlan> {
        self.validate()?;
        let delivery = if let Some(target) = self
            .distribution
            .binary
            .get(platform.key())
            .filter(|t| t.sha256.is_some())
        {
            let extension = std::path::Path::new(&target.cmd)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if matches!(extension.as_str(), "cmd" | "bat" | "ps1") {
                return Err(RegistryError::Unsupported(
                    "shell-script distributions require a custom profile".into(),
                ));
            }
            Delivery::Binary {
                target: target.clone(),
            }
        } else if let Some(target) = &self.distribution.npx {
            let mut target = target.clone();
            target.package = pinned_package(&target.package, &self.version, true)?;
            Delivery::Npx { target }
        } else if let Some(target) = &self.distribution.uvx {
            let mut target = target.clone();
            target.package = pinned_package(&target.package, &self.version, false)?;
            Delivery::Uvx { target }
        } else {
            return Err(RegistryError::Unsupported(
                if self.distribution.binary.contains_key(platform.key()) {
                    "the publisher has not provided a SHA-256 checksum for this binary".into()
                } else {
                    format!("no distribution for {}", platform.key())
                },
            ));
        };
        Ok(InstallPlan {
            entry: self.clone(),
            platform,
            delivery,
        })
    }
}
impl InstallPlan {
    pub fn id(&self) -> &str {
        &self.entry.id
    }
    pub fn name(&self) -> &str {
        &self.entry.name
    }
    pub fn version(&self) -> &str {
        &self.entry.version
    }
    pub fn version_key(&self) -> Result<[u64; 3]> {
        self.entry.version_key()
    }
    pub fn platform(&self) -> Platform {
        self.platform
    }
    pub fn origin(&self) -> String {
        match &self.delivery {
            Delivery::Binary { target } => target.archive.clone(),
            Delivery::Npx { target } => format!("npm: {}", target.package),
            Delivery::Uvx { target } => format!("PyPI: {}", target.package),
        }
    }
    pub fn license_url(&self) -> Option<&str> {
        self.entry.license_url.as_deref()
    }
    pub fn checksum(&self) -> Option<&str> {
        match &self.delivery {
            Delivery::Binary { target } => target.sha256.as_deref(),
            _ => None,
        }
    }
    pub fn package_managed(&self) -> bool {
        !matches!(self.delivery, Delivery::Binary { .. })
    }
    pub fn args(&self) -> &[String] {
        match &self.delivery {
            Delivery::Binary { target } => &target.args,
            Delivery::Npx { target } | Delivery::Uvx { target } => &target.args,
        }
    }
    pub fn environment(&self) -> &BTreeMap<String, String> {
        match &self.delivery {
            Delivery::Binary { target } => &target.env,
            Delivery::Npx { target } | Delivery::Uvx { target } => &target.env,
        }
    }
    pub(crate) fn validate(&self) -> Result<()> {
        let canonical = self.entry.plan(self.platform)?;
        if serde_json::to_vec(&canonical)? != serde_json::to_vec(self)? {
            return Err(RegistryError::Changed);
        }
        Ok(())
    }
}
fn label(value: &str, limit: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= limit
        && !value.chars().any(|c| c.is_control() && c != '\n')
}
fn version_key(value: &str) -> Result<[u64; 3]> {
    let parts: Vec<_> = value.split('.').collect();
    if value.len() > 64
        || parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
    {
        return Err(RegistryError::Invalid(
            "expected a stable numeric version".into(),
        ));
    }
    let mut result = [0; 3];
    for (i, part) in parts.iter().enumerate() {
        result[i] = part
            .parse()
            .map_err(|_| RegistryError::Invalid("version component too large".into()))?;
    }
    Ok(result)
}
fn metadata_url(value: &str) -> Result<Url> {
    if value.len() > 4096 || value.chars().any(char::is_control) {
        return Err(RegistryError::Invalid("invalid metadata URL".into()));
    }
    let url =
        Url::parse(value).map_err(|_| RegistryError::Invalid("invalid metadata URL".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(RegistryError::Invalid(
            "metadata URLs must be credential-free HTTP(S)".into(),
        ));
    }
    Ok(url)
}
pub(crate) fn https_url(value: &str) -> Result<Url> {
    let url = metadata_url(value)?;
    if url.scheme() != "https"
        || url.fragment().is_some()
        || url.port_or_known_default() != Some(443)
    {
        return Err(RegistryError::Invalid(
            "downloads require HTTPS on port 443 without fragments".into(),
        ));
    }
    Ok(url)
}
fn arguments(args: &[String], env: &BTreeMap<String, String>) -> Result<()> {
    if args.len() > 128
        || args.iter().any(|v| v.len() > 8192 || v.contains('\0'))
        || env.len() > 128
    {
        return Err(RegistryError::Limit);
    }
    for (key, value) in env {
        let key_lower = key.to_ascii_lowercase();
        if !valid_env_key(key)
            || key.len() > 128
            || value.len() > 8192
            || value.contains('\0')
            || [
                "token",
                "password",
                "secret",
                "api_key",
                "private_key",
                "credential",
            ]
            .iter()
            .any(|word| key_lower.contains(word))
        {
            return Err(RegistryError::Invalid(
                "registry environment must contain public configuration, not credentials".into(),
            ));
        }
    }
    Ok(())
}
fn pinned_package(value: &str, version: &str, npm: bool) -> Result<String> {
    let (name, pinned) = if npm {
        value
            .rsplit_once('@')
            .filter(|(n, _)| !n.is_empty())
            .map_or((value, None), |(n, v)| (n, Some(v)))
    } else {
        value
            .split_once("==")
            .map_or((value, None), |(n, v)| (n, Some(v)))
    };
    let atom = |s: &str| {
        !s.is_empty()
            && s.as_bytes()[0].is_ascii_alphanumeric()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
    };
    let name_ok = if npm && name.starts_with('@') {
        name[1..]
            .split_once('/')
            .is_some_and(|(scope, name)| atom(scope) && atom(name))
    } else {
        atom(name)
    };
    let pinned = pinned.unwrap_or(version);
    if !name_ok || pinned != version {
        return Err(RegistryError::Unsupported(
            "package must be pinned to the declared version, not a URL, range or tag".into(),
        ));
    }
    version_key(pinned)?;
    Ok(format!("{name}{}{pinned}", if npm { "@" } else { "==" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn entry() -> AgentEntry {
        serde_json::from_value(serde_json::json!({"id":"test-agent","name":"Test Agent","version":"1.2.3","description":"Fixture","license_url":"https://example.com/license","distribution":{"npx":{"package":"@sample/agent@1.2.3","args":["--acp"]}}})).unwrap()
    }
    #[test]
    fn scoped_packages_are_pinned_without_shell_parsing() {
        let plan = entry().plan(Platform::LinuxIntel).unwrap();
        assert_eq!(plan.origin(), "npm: @sample/agent@1.2.3");
        assert_eq!(
            pinned_package("my-agent", "1.2.3", true).unwrap(),
            "my-agent@1.2.3"
        );
        assert_eq!(
            pinned_package("agent", "1.2.3", false).unwrap(),
            "agent==1.2.3"
        );
    }
    #[test]
    fn package_flags_urls_ranges_and_version_mismatches_are_refused() {
        for name in [
            "--version",
            "https://example.com/x",
            "git+https://example.com/a",
            "agent@latest",
            "agent@^1.2.3",
            "agent@1.2.4",
            "agent;echo",
            "@x/y/z@1.2.3",
        ] {
            assert!(pinned_package(name, "1.2.3", true).is_err(), "{name}");
        }
        assert!(pinned_package("agent>=1.2.3", "1.2.3", false).is_err());
    }
    #[test]
    fn schema_duplicates_and_credential_metadata_fail_closed() {
        let entry = entry();
        let json = serde_json::to_vec(&Registry {
            version: "1.0.0".into(),
            agents: vec![entry.clone(), entry.clone()],
        })
        .unwrap();
        assert!(Registry::parse(&json).is_err());
        let mut bad = entry;
        bad.distribution
            .npx
            .as_mut()
            .unwrap()
            .env
            .insert("API_TOKEN".into(), "secret".into());
        assert!(bad.plan(Platform::LinuxIntel).is_err());
    }
    #[test]
    fn numeric_versions_match_the_registry_schema_including_calendar_versions() {
        assert_eq!(version_key("2026.09.10").unwrap(), [2026, 9, 10]);
        assert!(version_key("1.2.3-preview.1").is_err());
        assert!(version_key("1.10.0").unwrap() > version_key("1.9.0").unwrap());
    }
    #[test]
    fn missing_checksums_are_not_silently_approved() {
        let mut a = entry();
        a.distribution.npx = None;
        a.distribution.binary.insert(
            "linux-x86_64".into(),
            BinaryTarget {
                archive: "https://example.com/agent.tar.gz".into(),
                cmd: "./agent".into(),
                sha256: None,
                args: vec![],
                env: BTreeMap::new(),
            },
        );
        assert!(a.plan(Platform::LinuxIntel).is_err());
        assert!(a.plan(Platform::MacArm).is_err());
        a.distribution
            .binary
            .get_mut("linux-x86_64")
            .unwrap()
            .sha256 = Some("00".repeat(32));
        assert!(!a.plan(Platform::LinuxIntel).unwrap().package_managed());
    }
}
