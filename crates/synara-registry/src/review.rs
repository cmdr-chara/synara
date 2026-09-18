//! Pure update review. Planning never downloads, executes, activates or removes an agent.
use crate::{InstallPlan, InstalledAgent, Platform, Registry, RegistryError, Result};
use sha2::{Digest, Sha256};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VersionRelation {
    Unchanged,
    Newer,
    Older,
    Revised,
}

#[derive(Clone, Debug)]
pub struct UpdateReview {
    pub relation: VersionRelation,
    pub previous_version: String,
    pub origin_changed: bool,
    pub arguments_changed: bool,
    pub environment_changed: bool,
    pub integrity_changed: bool,
    fingerprint: String,
    candidate: InstallPlan,
}
impl UpdateReview {
    pub fn plan(&self) -> &InstallPlan {
        &self.candidate
    }

    /// Bind the review UI to the exact plan, including public environment and arguments.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }

    /// Call before committing a reviewed selection after a catalog refresh.
    /// A matching digest is not a signature or a substitute for user consent.
    pub fn matches(&self, plan: &InstallPlan) -> Result<bool> {
        Ok(self.fingerprint == fingerprint(plan)?)
    }
}
impl Registry {
    /// Plan only the same agent on the same platform. Explicit downgrade or revised
    /// same-version metadata must be presented as such, not silently treated as current.
    pub fn review_update(
        &self,
        installed: &InstalledAgent,
        platform: Platform,
    ) -> Result<Option<UpdateReview>> {
        installed.plan.validate()?;
        if installed.plan.platform() != platform {
            return Err(RegistryError::Unsupported(
                "updates cannot change the installation platform".into(),
            ));
        }
        let mut entries = self
            .agents
            .iter()
            .filter(|entry| entry.id == installed.plan.id());
        let Some(entry) = entries.next() else {
            return Ok(None);
        };
        if entries.next().is_some() {
            return Err(RegistryError::Invalid("duplicate agent ID".into()));
        }
        let candidate = entry.plan(platform)?;
        let previous = &installed.plan;
        let digest = fingerprint(&candidate)?;
        let relation = if digest == fingerprint(previous)? {
            VersionRelation::Unchanged
        } else {
            match candidate.version_key()?.cmp(&previous.version_key()?) {
                std::cmp::Ordering::Greater => VersionRelation::Newer,
                std::cmp::Ordering::Less => VersionRelation::Older,
                std::cmp::Ordering::Equal => VersionRelation::Revised,
            }
        };
        Ok(Some(UpdateReview {
            relation,
            previous_version: previous.version().into(),
            origin_changed: previous.origin() != candidate.origin(),
            arguments_changed: previous.args() != candidate.args(),
            environment_changed: previous.environment() != candidate.environment(),
            integrity_changed: previous.checksum() != candidate.checksum()
                || previous.package_managed() != candidate.package_managed(),
            fingerprint: digest,
            candidate,
        }))
    }
}

fn fingerprint(plan: &InstallPlan) -> Result<String> {
    plan.validate()?;
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(plan)?)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentEntry, Downloader};
    use std::io::Write;

    struct Never;
    impl Downloader for Never {
        fn download(&self, _: &str, _: &mut dyn Write, _: u64) -> Result<()> {
            panic!("reviewing or registering a package must not download");
        }
    }
    fn entry(version: &str) -> AgentEntry {
        serde_json::from_value(serde_json::json!({
            "id": "review-test",
            "name": "Review test",
            "version": version,
            "description": "Fixture only",
            "license_url": "https://example.test/license",
            "distribution": {"npx": {"package": "@example/test", "args": ["acp"]}}
        }))
        .unwrap()
    }
    fn catalog(entry: AgentEntry) -> Registry {
        Registry {
            version: "1.0.0".into(),
            agents: vec![entry],
        }
    }
    fn installed(root: &std::path::Path) -> InstalledAgent {
        crate::RegistryStore::open(root)
            .unwrap()
            .install(
                &entry("1.9.0").plan(Platform::current().unwrap()).unwrap(),
                &Never,
            )
            .unwrap()
    }

    #[test]
    fn reviews_numeric_upgrade_downgrade_and_unchanged_without_execution() {
        let directory = tempfile::tempdir().unwrap();
        let installed = installed(directory.path());
        let platform = Platform::current().unwrap();
        for (version, relation) in [
            ("1.10.0", VersionRelation::Newer),
            ("1.8.0", VersionRelation::Older),
            ("1.9.0", VersionRelation::Unchanged),
        ] {
            let review = catalog(entry(version))
                .review_update(&installed, platform)
                .unwrap()
                .unwrap();
            assert_eq!(review.relation, relation);
            assert_eq!(review.previous_version, "1.9.0");
            assert!(review.matches(review.plan()).unwrap());
        }
        assert!(installed.reference.agent_spec().is_ok());
        assert_eq!(
            crate::RegistryStore::open(directory.path())
                .unwrap()
                .installed()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn same_version_launch_changes_require_new_review() {
        let directory = tempfile::tempdir().unwrap();
        let installed = installed(directory.path());
        let mut changed = entry("1.9.0");
        let target = changed.distribution.npx.as_mut().unwrap();
        target.args.push("--different".into());
        target.env.insert("PUBLIC_MODE".into(), "different".into());
        let review = catalog(changed)
            .review_update(&installed, Platform::current().unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(review.relation, VersionRelation::Revised);
        assert!(review.arguments_changed);
        assert!(review.environment_changed);
        assert!(!review.matches(&installed.plan).unwrap());
    }

    #[test]
    fn missing_agent_is_not_an_uninstall_instruction() {
        let directory = tempfile::tempdir().unwrap();
        let installed = installed(directory.path());
        let empty = Registry {
            version: "1.0.0".into(),
            agents: vec![],
        };
        assert!(
            empty
                .review_update(&installed, Platform::current().unwrap())
                .unwrap()
                .is_none()
        );
        assert!(installed.reference.agent_spec().is_ok());
    }

    #[test]
    fn platform_switch_and_duplicate_identity_are_not_updates() {
        let directory = tempfile::tempdir().unwrap();
        let installed = installed(directory.path());
        let platform = Platform::current().unwrap();
        let other = if platform == Platform::MacArm {
            Platform::WindowsIntel
        } else {
            Platform::MacArm
        };
        let mut registry = catalog(entry("1.10.0"));
        assert!(registry.review_update(&installed, other).is_err());
        registry.agents.push(entry("1.11.0"));
        assert!(registry.review_update(&installed, platform).is_err());
    }
}
