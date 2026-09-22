//! Records actual locally observed native versions. This is not a release feed.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};

pub const NATIVE_VERSION_HISTORY_KEY: &str = "native-version-history";
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeVersionVisit {
    pub version: String,
    pub observed_at_ms: i64,
    pub read: bool,
}
impl NativeVersionVisit {
    pub fn observed_label(&self) -> String {
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(self.observed_at_ms)
            .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
            .unwrap_or_else(|| "Unknown observation date".into())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct NativeVersionHistory {
    pub format_version: u32,
    pub revision: u64,
    /// Chronological observations, bounded to the most recent 32 transitions.
    pub visits: Vec<NativeVersionVisit>,
}
impl Default for NativeVersionHistory {
    fn default() -> Self {
        Self {
            format_version: 1,
            revision: 0,
            visits: vec![],
        }
    }
}
impl NativeVersionHistory {
    fn validate(&self) -> WorkspaceResult<()> {
        if self.format_version != 1
            || self.visits.len() > 32
            || self
                .visits
                .iter()
                .any(|v| !valid_version(&v.version) || v.observed_at_ms < 0)
            || self.visits.windows(2).any(|v| v[0].version == v[1].version)
        {
            return Err(WorkspaceError::Invalid(
                "Saved native version history is invalid. It was not replaced.".into(),
            ));
        }
        Ok(())
    }
    pub fn current(&self) -> Option<&NativeVersionVisit> {
        self.visits.last()
    }
    fn observe(&mut self, version: &str, now: i64) -> WorkspaceResult<bool> {
        self.validate()?;
        if !valid_version(version) || now < 0 {
            return Err(StorageError::Limit.into());
        }
        if self.current().is_some_and(|v| v.version == version) {
            return Ok(false);
        }
        self.visits.push(NativeVersionVisit {
            version: version.into(),
            observed_at_ms: now,
            read: false,
        });
        if self.visits.len() > 32 {
            self.visits.remove(0);
        }
        self.revision = self.revision.checked_add(1).ok_or(StorageError::Limit)?;
        Ok(true)
    }
}
fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'+'))
}
fn read(connection: &Connection) -> WorkspaceResult<NativeVersionHistory> {
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [NATIVE_VERSION_HISTORY_KEY],
            |r| r.get(0),
        )
        .optional()?;
    if raw.as_ref().is_some_and(|r| r.len() > 16 * 1024) {
        return Err(StorageError::Limit.into());
    }
    let value: NativeVersionHistory = raw.as_deref().map(decode).transpose()?.unwrap_or_default();
    value.validate()?;
    Ok(value)
}
fn write(connection: &Connection, value: &NativeVersionHistory) -> WorkspaceResult<()> {
    value.validate()?;
    connection.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",params![NATIVE_VERSION_HISTORY_KEY,encode(value)?])?;
    Ok(())
}
impl WorkspaceService {
    /// Called with the compiled native application version, never a feed or environment override.
    pub async fn observe_native_version(
        &self,
        version: String,
    ) -> WorkspaceResult<NativeVersionHistory> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut history = read(&tx)?;
            if history.observe(&version, crate::now_ms())? {
                write(&tx, &history)?;
            }
            tx.commit()?;
            Ok(history)
        })
        .await
    }
    pub async fn mark_native_version_read(
        &self,
        version: String,
        revision: u64,
    ) -> WorkspaceResult<NativeVersionHistory> {
        self.access(move |store| {
            let tx = store
                .connection
                .transaction_with_behavior(TransactionBehavior::Immediate)?;
            let mut history = read(&tx)?;
            if history.revision != revision
                || history.current().is_none_or(|v| v.version != version)
            {
                return Err(WorkspaceError::Invalid(
                    "Native version history changed. Reload before dismissing.".into(),
                ));
            }
            history.visits.last_mut().unwrap().read = true;
            history.revision = history.revision.checked_add(1).ok_or(StorageError::Limit)?;
            write(&tx, &history)?;
            tx.commit()?;
            Ok(history)
        })
        .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn releases_transitions_are_bounded_and_do_not_claim_upgrade_order() {
        let mut h = NativeVersionHistory::default();
        assert!(h.observe("0.1.0", 1).unwrap());
        h.visits[0].read = true;
        assert!(!h.observe("0.1.0", 2).unwrap());
        assert!(h.current().unwrap().read);
        assert!(h.observe("0.0.9", 3).unwrap());
        assert!(!h.current().unwrap().read);
        for n in 0..50 {
            h.observe(&format!("0.2.{n}"), n + 4).unwrap();
        }
        assert_eq!(h.visits.len(), 32);
        h.validate().unwrap();
        assert!(h.observe("evil\nversion", 70).is_err());
    }
    #[tokio::test]
    async fn releases_read_state_survives_restart_and_stale_dismissal_is_rejected() {
        let d = tempfile::tempdir().unwrap();
        let path = d.path().join("state.db");
        let s = WorkspaceService::open(path.clone()).await.unwrap();
        let old = s.observe_native_version("0.1.0".into()).await.unwrap();
        let read = s
            .mark_native_version_read("0.1.0".into(), old.revision)
            .await
            .unwrap();
        drop(s);
        let s = WorkspaceService::open(path).await.unwrap();
        assert_eq!(
            s.observe_native_version("0.1.0".into()).await.unwrap(),
            read
        );
        let new = s.observe_native_version("0.2.0".into()).await.unwrap();
        assert!(!new.current().unwrap().read);
        assert!(
            s.mark_native_version_read("0.1.0".into(), read.revision)
                .await
                .is_err()
        );
        assert!(s.catalog().await.unwrap().tasks.is_empty());
    }
    #[test]
    fn releases_unknown_or_corrupt_history_is_not_replaced() {
        let mut h = NativeVersionHistory::default();
        h.format_version = 2;
        assert!(h.observe("0.1.0", 1).is_err());
        h.format_version = 1;
        h.visits.push(NativeVersionVisit {
            version: "<invalid>".into(),
            observed_at_ms: 0,
            read: false,
        });
        assert!(h.observe("0.1.0", 1).is_err());
        assert_eq!(h.visits[0].version, "<invalid>");
    }
}
