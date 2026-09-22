//! Local build notes and acknowledgement. No endpoint, trust key, updater
//! transport, installer or release metadata is invented by this module.
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService, now_ms};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// The fingerprint identifies bundled notes. It is not a release signature.
const KEY: &str = "native-release-experience-v1";
const MAX_HISTORY: usize = 24;
const MAX_JOURNAL_BYTES: usize = 64 * 1024;
pub const BUNDLED_BUILD_NOTES: &str = include_str!("release_notes.md");

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedBuild {
    pub version: String,
    pub notes_sha256: String,
    /// First observed in this installation, not a release/publication date.
    pub observed_at_ms: i64,
}
impl ObservedBuild {
    fn valid(&self) -> bool {
        !self.version.is_empty()
            && self.version.len() <= 128
            && self
                .version
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b".-+".contains(&c))
            && self.notes_sha256.len() == 64
            && self
                .notes_sha256
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            && self.observed_at_ms >= 0
    }
    fn same_build(&self, other: &Self) -> bool {
        self.version == other.version && self.notes_sha256 == other.notes_sha256
    }
}
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseJournal {
    pub revision: u64,
    pub history: Vec<ObservedBuild>,
    pub acknowledged: Option<ObservedBuild>,
}
impl ReleaseJournal {
    pub(crate) fn validate(&self) -> WorkspaceResult<()> {
        if self.history.len() > MAX_HISTORY
            || self.history.iter().any(|b| !b.valid())
            || self.acknowledged.as_ref().is_some_and(|b| !b.valid())
        {
            return Err(invalid(
                "Saved build history is invalid. It was not overwritten.",
            ));
        }
        Ok(())
    }
    pub fn unread(&self) -> bool {
        self.history.last().is_some_and(|current| {
            !self
                .acknowledged
                .as_ref()
                .is_some_and(|seen| seen.same_build(current))
        })
    }
    fn advance(&mut self) -> WorkspaceResult<()> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| invalid("Build-history revision exhausted."))?;
        Ok(())
    }
    fn observe(&mut self, current: ObservedBuild) -> WorkspaceResult<bool> {
        self.validate()?;
        if !current.valid() {
            return Err(invalid("Invalid compiled build identity."));
        }
        if self
            .history
            .last()
            .is_some_and(|last| last.same_build(&current))
        {
            return Ok(false);
        }
        self.advance()?;
        self.history.push(current);
        if self.history.len() > MAX_HISTORY {
            self.history.remove(0);
        }
        Ok(true)
    }
    fn acknowledge(
        &mut self,
        expected_revision: u64,
        current: &ObservedBuild,
    ) -> WorkspaceResult<()> {
        self.validate()?;
        if self.revision != expected_revision
            || !self
                .history
                .last()
                .is_some_and(|last| last.same_build(current))
        {
            return Err(invalid(
                "Build notes changed. Reload before marking them read.",
            ));
        }
        if self.unread() {
            self.advance()?;
            self.acknowledged = Some(current.clone());
        }
        Ok(())
    }
}
fn invalid(message: &str) -> WorkspaceError {
    WorkspaceError::Invalid(message.into())
}
fn compiled_build() -> ObservedBuild {
    ObservedBuild {
        version: env!("CARGO_PKG_VERSION").into(),
        notes_sha256: hex::encode(Sha256::digest(BUNDLED_BUILD_NOTES.as_bytes())),
        observed_at_ms: now_ms().max(0),
    }
}
fn load(store: &crate::Store) -> WorkspaceResult<ReleaseJournal> {
    let Some(raw) = store.preference_raw(KEY)? else {
        return Ok(ReleaseJournal::default());
    };
    if raw.len() > MAX_JOURNAL_BYTES {
        return Err(invalid(
            "Saved build history is oversized. It was not overwritten.",
        ));
    }
    let journal: ReleaseJournal = serde_json::from_str(&raw)
        .map_err(|_| invalid("Saved build history cannot be read. It was not overwritten."))?;
    journal.validate()?;
    Ok(journal)
}
impl WorkspaceService {
    pub async fn observe_current_build(&self) -> WorkspaceResult<ReleaseJournal> {
        self.access(|store| {
            let mut journal = load(store)?;
            if journal.observe(compiled_build())? {
                store.set_preference(KEY, &journal)?;
            }
            Ok(journal)
        })
        .await
    }
    pub async fn acknowledge_build_notes(
        &self,
        expected_revision: u64,
    ) -> WorkspaceResult<ReleaseJournal> {
        self.access(move |store| {
            let mut journal = load(store)?;
            journal.acknowledge(expected_revision, &compiled_build())?;
            store.set_preference(KEY, &journal)?;
            Ok(journal)
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn build(version: &str, notes: &str, now: i64) -> ObservedBuild {
        ObservedBuild {
            version: version.into(),
            notes_sha256: hex::encode(Sha256::digest(notes.as_bytes())),
            observed_at_ms: now,
        }
    }
    #[test]
    fn first_observation_is_unread_and_duplicate_launch_does_not_change_history() {
        let mut j = ReleaseJournal::default();
        assert!(j.observe(build("0.1.0", "notes", 10)).unwrap());
        assert!(j.unread());
        let before = j.clone();
        assert!(!j.observe(build("0.1.0", "notes", 20)).unwrap());
        assert_eq!(before, j);
    }
    #[test]
    fn explicit_acknowledgement_survives_launch_and_changed_notes_are_unread() {
        let mut j = ReleaseJournal::default();
        let b = build("0.1.0", "notes", 10);
        j.observe(b.clone()).unwrap();
        j.acknowledge(j.revision, &b).unwrap();
        assert!(!j.unread());
        j.observe(build("0.1.0", "notes", 30)).unwrap();
        assert!(!j.unread());
        j.observe(build("0.1.0", "new development notes", 40))
            .unwrap();
        assert!(j.unread());
    }
    #[test]
    fn stale_acknowledgements_cannot_hide_another_build() {
        let mut j = ReleaseJournal::default();
        let a = build("0.1.0", "a", 1);
        j.observe(a.clone()).unwrap();
        let old = j.revision;
        j.observe(build("0.2.0", "b", 2)).unwrap();
        let before = j.clone();
        assert!(j.acknowledge(old, &a).is_err());
        assert!(j.acknowledge(j.revision, &a).is_err());
        assert_eq!(before, j);
    }
    #[test]
    fn repeated_upgrades_and_rollbacks_have_bounded_local_history() {
        let mut j = ReleaseJournal::default();
        for i in 0..100 {
            j.observe(build("0.1.0", &format!("notes {i}"), i)).unwrap();
        }
        assert_eq!(j.history.len(), MAX_HISTORY);
        assert_eq!(j.history[0].observed_at_ms, 76);
        j.observe(build("0.0.9", "rollback", 101)).unwrap();
        assert!(j.unread());
        assert_eq!(j.history.len(), MAX_HISTORY);
    }
    #[test]
    fn malformed_or_excessive_saved_history_is_rejected() {
        let mut j = ReleaseJournal::default();
        assert!(j.observe(build("../other", "notes", 1)).is_err());
        assert!(j.observe(build("0.1.0", "notes", -1)).is_err());
        j.history = vec![build("0.1.0", "notes", 1); MAX_HISTORY + 1];
        assert!(j.validate().is_err());
        j.history = vec![ObservedBuild {
            notes_sha256: "fake".into(),
            ..build("0.1.0", "notes", 1)
        }];
        assert!(j.validate().is_err());
    }
    #[tokio::test]
    async fn durable_acknowledgement_and_history_survive_restart_without_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("workspace.sqlite3");
        let service = WorkspaceService::open(path.clone()).await.unwrap();
        let observed = service.observe_current_build().await.unwrap();
        assert!(observed.unread());
        let seen = service
            .acknowledge_build_notes(observed.revision)
            .await
            .unwrap();
        assert!(!seen.unread());
        assert!(service.catalog().await.unwrap().tasks.is_empty());
        drop(service);
        let service = WorkspaceService::open(path).await.unwrap();
        assert_eq!(service.observe_current_build().await.unwrap(), seen);
        assert!(service.catalog().await.unwrap().tasks.is_empty());
    }
    #[tokio::test]
    async fn corrupt_history_is_preserved_not_silently_reset() {
        let service = WorkspaceService::memory().unwrap();
        service
            .access(|store| {
                store.set_preference(KEY, &"not a journal")?;
                Ok(())
            })
            .await
            .unwrap();
        assert!(service.observe_current_build().await.is_err());
        let saved = service
            .access(|store| Ok(store.preference::<String>(KEY)?))
            .await
            .unwrap();
        assert_eq!(saved.as_deref(), Some("not a journal"));
    }
}
