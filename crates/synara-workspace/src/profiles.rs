use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use synara_agent::{AgentError, AgentResult, AgentSpec};
use synara_runtime::{LaunchSpec, SecretReference, SecretStore, valid_env_key};

/// Persist variable names, never credential values. Vendor login remains vendor-owned.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<synara_registry::RegistryReference>,
    pub id: String,
    pub name: String,
    pub command: PathBuf,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub inherit_env: Vec<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub secret_env: BTreeMap<String, SecretReference>,
}
impl AgentProfile {
    pub fn validate(&self) -> AgentResult<()> {
        if self.name.len() > 120
            || self.name.chars().any(char::is_control)
            || self.inherit_env.len().saturating_add(self.secret_env.len()) > 128
            || self
                .inherit_env
                .iter()
                .any(|key| key.len() > 128 || !valid_env_key(key))
            || self
                .secret_env
                .keys()
                .any(|key| key.len() > 128 || !valid_env_key(key))
            || self.inherit_env.iter().collect::<BTreeSet<_>>().len() != self.inherit_env.len()
            || self
                .secret_env
                .keys()
                .any(|key| self.inherit_env.iter().any(|inherited| inherited == key))
            || (self.registry.is_some() && !self.secret_env.is_empty())
        {
            return Err(AgentError::Invalid("invalid agent profile".into()));
        }
        for reference in self.secret_env.values() {
            reference.validate()?;
        }
        if let Some(reference) = &self.registry {
            reference
                .validate()
                .map_err(|error| AgentError::Invalid(error.to_string()))?;
        }
        self.spec_with_environment(|_| None)?.validate()
    }
    fn spec_with_environment(
        &self,
        read: impl Fn(&str) -> Option<String>,
    ) -> AgentResult<AgentSpec> {
        let mut env = BTreeMap::new();
        for key in &self.inherit_env {
            if let Some(value) = read(key) {
                env.insert(key.clone(), value);
            }
        }
        Ok(AgentSpec {
            launch_directory: None,
            id: self.id.clone(),
            name: self.name.clone(),
            origin: "User-configured local executable".into(),
            launch: LaunchSpec {
                command: self.command.clone(),
                args: self.args.clone(),
                env,
            },
        })
    }
    pub fn from_registry(reference: synara_registry::RegistryReference) -> AgentResult<Self> {
        let spec = reference
            .agent_spec()
            .map_err(|error| AgentError::Invalid(error.to_string()))?;
        Ok(Self {
            registry: Some(reference),
            id: spec.id,
            name: spec.name,
            command: spec.launch.command,
            args: spec.launch.args,
            inherit_env: vec![],
            secret_env: BTreeMap::new(),
        })
    }
    fn launch_spec_without_secrets(&self) -> AgentResult<AgentSpec> {
        let spec = if let Some(reference) = &self.registry {
            let mut spec = reference
                .agent_spec()
                .map_err(|error| AgentError::Invalid(error.to_string()))?;
            if spec.id != self.id
                || spec.launch.command != self.command
                || spec.launch.args != self.args
            {
                return Err(AgentError::Invalid("managed launch fields differ from the approved installation. Use a custom profile for overrides".into()));
            }
            for key in &self.inherit_env {
                if let Ok(value) = std::env::var(key) {
                    spec.launch.env.insert(key.clone(), value);
                }
            }
            spec
        } else {
            self.spec_with_environment(|key| std::env::var(key).ok())?
        };
        spec.validate()?;
        Ok(spec)
    }

    pub fn launch_spec(&self) -> AgentResult<AgentSpec> {
        self.validate()?;
        if !self.secret_env.is_empty() {
            return Err(AgentError::Unsupported(
                "this profile requires an operating-system credential store".into(),
            ));
        }
        self.launch_spec_without_secrets()
    }

    pub async fn launch_spec_with_secret_store(
        &self,
        secrets: &dyn SecretStore,
    ) -> AgentResult<AgentSpec> {
        self.validate()?;
        let mut spec = self.launch_spec_without_secrets()?;
        for (key, reference) in &self.secret_env {
            let value = secrets
                .read(reference)
                .await?
                .ok_or_else(|| AgentError::Invalid("referenced credential is missing".into()))?;
            let value = std::str::from_utf8(value.expose())
                .map_err(|_| AgentError::Invalid("credential value is not UTF-8".into()))?;
            if value.contains('\0') {
                return Err(AgentError::Invalid(
                    "credential value cannot contain NUL".into(),
                ));
            }
            spec.launch.env.insert(key.clone(), value.to_owned());
        }
        spec.launch.validate()?;
        Ok(spec)
    }
}
/// Launch presets only. Models, modes and authentication are discovered from each connection.
pub fn default_profiles() -> Vec<AgentProfile> {
    vec![
        AgentProfile {
            registry: None,
            id: "opencode".into(),
            name: "OpenCode".into(),
            command: "opencode".into(),
            args: vec!["acp".into()],
            inherit_env: vec![],
            secret_env: BTreeMap::new(),
        },
        AgentProfile {
            registry: None,
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            command: "gemini".into(),
            args: vec!["--acp".into()],
            inherit_env: vec![],
            secret_env: BTreeMap::new(),
        },
        AgentProfile {
            registry: None,
            id: "omp".into(),
            name: "Oh My Pi".into(),
            command: "omp".into(),
            args: vec!["acp".into()],
            inherit_env: vec![],
            secret_env: BTreeMap::new(),
        },
    ]
}
pub fn parse_profiles(text: &str) -> AgentResult<Vec<AgentProfile>> {
    if text.len() > 1024 * 1024 {
        return Err(AgentError::Limit);
    }
    let profiles: Vec<AgentProfile> = serde_json::from_str(text)
        .map_err(|_| AgentError::Invalid("agent profiles must be a JSON array with id, name, command, args and optional inherit_env/secret_env".into()))?;
    validate_profiles(&profiles)?;
    Ok(profiles)
}
pub fn validate_profiles(profiles: &[AgentProfile]) -> AgentResult<()> {
    if profiles.is_empty() || profiles.len() > 64 {
        return Err(AgentError::Limit);
    }
    let mut ids = BTreeSet::new();
    for profile in profiles {
        profile.validate()?;
        if !ids.insert(&profile.id) {
            return Err(AgentError::Invalid("agent IDs must be unique".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use synara_runtime::{SecretStoreState, SecretValue, UnavailableSecretStore};

    struct FixtureSecrets;
    #[async_trait]
    impl SecretStore for FixtureSecrets {
        fn state(&self) -> SecretStoreState {
            SecretStoreState::Available
        }
        async fn read(
            &self,
            reference: &SecretReference,
        ) -> Result<Option<SecretValue>, synara_runtime::RuntimeError> {
            reference.validate()?;
            Ok(Some(SecretValue::new(b"secret-canary".to_vec())?))
        }
        async fn write(
            &self,
            _reference: &SecretReference,
            _value: SecretValue,
        ) -> Result<(), synara_runtime::RuntimeError> {
            unreachable!()
        }
        async fn delete(
            &self,
            _reference: &SecretReference,
        ) -> Result<(), synara_runtime::RuntimeError> {
            unreachable!()
        }
    }
    #[test]
    fn custom_launch_arguments_are_not_shell_parsed() {
        let profiles=parse_profiles(r#"[{"id":"custom","name":"Custom","command":"/opt/My Agent/bin","args":["acp","a b; echo injected"],"inherit_env":["MY_AGENT_TOKEN"]}]"#).unwrap();
        let spec = profiles[0]
            .spec_with_environment(|_| Some("secret".into()))
            .unwrap();
        assert_eq!(spec.launch.args, vec!["acp", "a b; echo injected"]);
        assert_eq!(spec.launch.env["MY_AGENT_TOKEN"], "secret");
        assert!(!format!("{:?}", spec.launch).contains("secret"));
        assert!(!serde_json::to_string(&profiles).unwrap().contains("secret"));
    }
    #[test]
    fn default_profiles_launch_acp_and_validate() {
        let profiles = default_profiles();
        let omp = profiles.iter().find(|p| p.id == "omp").unwrap();
        assert_eq!(omp.command, PathBuf::from("omp"));
        assert_eq!(omp.args, vec!["acp"]);
        for profile in &profiles {
            profile.validate().unwrap();
        }
    }

    #[test]
    fn duplicate_ids_invalid_env_and_unknown_plaintext_fields_are_rejected() {
        let mut profiles = default_profiles();
        profiles.push(profiles[0].clone());
        assert!(validate_profiles(&profiles).is_err());
        let mut profile = default_profiles().remove(0);
        profile.inherit_env = vec!["TOKEN=secret".into()];
        assert!(profile.validate().is_err());
        assert!(
            parse_profiles(r#"[{"id":"x","name":"X","command":"x","env":{"KEY":"secret"}}]"#)
                .is_err()
        );
    }

    #[tokio::test]
    async fn secret_references_are_persisted_but_values_exist_only_at_launch() {
        let profiles = parse_profiles(
            r#"[{"id":"custom","name":"Custom","command":"/opt/agent","secret_env":{"API_TOKEN":{"service":"dev.synara","account":"agent/custom"}}}]"#,
        )
        .unwrap();
        let encoded = serde_json::to_string(&profiles).unwrap();
        assert!(encoded.contains("agent/custom"));
        assert!(!encoded.contains("secret-canary"));
        assert!(profiles[0].launch_spec().is_err());
        let spec = profiles[0]
            .launch_spec_with_secret_store(&FixtureSecrets)
            .await
            .unwrap();
        assert_eq!(spec.launch.env["API_TOKEN"], "secret-canary");
        assert!(!format!("{:?}", spec.launch).contains("secret-canary"));
        assert!(matches!(
            profiles[0]
                .launch_spec_with_secret_store(&UnavailableSecretStore::unavailable())
                .await,
            Err(AgentError::Runtime(
                synara_runtime::RuntimeError::Unsupported(_)
            ))
        ));
    }

    #[test]
    fn secret_and_inherited_environment_keys_cannot_overlap() {
        let mut profile = default_profiles().remove(0);
        profile.inherit_env.push("API_TOKEN".into());
        profile.secret_env.insert(
            "API_TOKEN".into(),
            SecretReference::new("dev.synara", "agent").unwrap(),
        );
        assert!(profile.validate().is_err());
    }
}
