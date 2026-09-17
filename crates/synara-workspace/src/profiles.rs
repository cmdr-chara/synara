use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};
use synara_agent::{AgentError, AgentResult, AgentSpec};
use synara_runtime::{LaunchSpec, valid_env_key};

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
}
impl AgentProfile {
    pub fn validate(&self) -> AgentResult<()> {
        if self.name.len() > 120
            || self.name.chars().any(char::is_control)
            || self.inherit_env.len() > 128
            || self
                .inherit_env
                .iter()
                .any(|key| key.len() > 128 || !valid_env_key(key))
            || self.inherit_env.iter().collect::<BTreeSet<_>>().len() != self.inherit_env.len()
        {
            return Err(AgentError::Invalid("invalid agent profile".into()));
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
        })
    }
    pub fn launch_spec(&self) -> AgentResult<AgentSpec> {
        self.validate()?;
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
        },
        AgentProfile {
            registry: None,
            id: "gemini".into(),
            name: "Gemini CLI".into(),
            command: "gemini".into(),
            args: vec!["--acp".into()],
            inherit_env: vec![],
        },
    ]
}
pub fn parse_profiles(text: &str) -> AgentResult<Vec<AgentProfile>> {
    if text.len() > 1024 * 1024 {
        return Err(AgentError::Limit);
    }
    let profiles: Vec<AgentProfile> = serde_json::from_str(text)
        .map_err(|_| AgentError::Invalid("agent profiles must be a JSON array with id, name, command, args and optional inherit_env".into()))?;
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
}
