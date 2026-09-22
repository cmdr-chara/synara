//! Terminal layout metadata only. Processes, commands, output and credentials are
//! never persisted or replayed. Restoring a tab must not start a shell.
use super::*;
use crate::{WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

pub const MAX_TERMINAL_TABS: usize = 12;
const MAX_LAYOUT_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalScope {
    pub project: ProjectId,
    pub root: PathBuf,
}
impl TerminalScope {
    fn key(&self) -> WorkspaceResult<String> {
        if !self.root.has_root() || self.root.as_os_str().len() > 8192 {
            return Err(WorkspaceError::Invalid(
                "Invalid terminal workspace root".into(),
            ));
        }
        Ok(format!("terminal-layout:{}", encode(self)?))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalTabLayout {
    pub id: u64,
    pub name: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalLayout {
    pub version: u32,
    pub revision: u64,
    pub tabs: Vec<TerminalTabLayout>,
    pub active: Option<u64>,
    pub secondary: Option<u64>,
    pub stacked: bool,
    pub primary_percent: u8,
    pub next_id: u64,
}
impl Default for TerminalLayout {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            tabs: vec![TerminalTabLayout {
                id: 1,
                name: "Terminal 1".into(),
            }],
            active: Some(1),
            secondary: None,
            stacked: false,
            primary_percent: 50,
            next_id: 2,
        }
    }
}
impl TerminalLayout {
    pub fn validate(&self) -> WorkspaceResult<()> {
        let ids: HashSet<_> = self.tabs.iter().map(|tab| tab.id).collect();
        if self.version != 1
            || self.tabs.len() > MAX_TERMINAL_TABS
            || ids.len() != self.tabs.len()
            || !(25..=75).contains(&self.primary_percent)
            || self.tabs.iter().any(|tab| {
                tab.id == 0
                    || tab.id >= self.next_id
                    || tab.name.trim().is_empty()
                    || tab.name.len() > 160
                    || tab.name.chars().any(char::is_control)
            })
            || self.active.is_none() != self.tabs.is_empty()
            || self.active.is_some_and(|id| !ids.contains(&id))
            || self
                .secondary
                .is_some_and(|id| !ids.contains(&id) || Some(id) == self.active)
        {
            return Err(WorkspaceError::Invalid(
                "Unsupported or invalid terminal layout. The saved value was not replaced.".into(),
            ));
        }
        Ok(())
    }
}
fn read(connection: &Connection, scope: &TerminalScope) -> WorkspaceResult<TerminalLayout> {
    let exists: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
            [scope.project.to_string()],
            |row| row.get(0),
        )
        .map_err(StorageError::from)?;
    if !exists {
        return Err(WorkspaceError::NotFound);
    }
    let raw: Option<String> = connection
        .query_row(
            "SELECT data FROM preferences WHERE key=?1",
            [scope.key()?],
            |row| row.get(0),
        )
        .optional()
        .map_err(StorageError::from)?;
    let value = match raw {
        Some(raw) if raw.len() > MAX_LAYOUT_BYTES => return Err(StorageError::Limit.into()),
        Some(raw) => decode::<TerminalLayout>(&raw)?,
        None => TerminalLayout::default(),
    };
    value.validate()?;
    Ok(value)
}
impl WorkspaceService {
    pub async fn terminal_layout(&self, scope: TerminalScope) -> WorkspaceResult<TerminalLayout> {
        self.access(move |store| read(&store.connection, &scope))
            .await
    }
    pub async fn save_terminal_layout(
        &self,
        scope: TerminalScope,
        mut value: TerminalLayout,
    ) -> WorkspaceResult<TerminalLayout> {
        value.validate()?;
        self.access(move |store| {
            let tx = store.connection.transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(StorageError::from)?;
            let previous = read(&tx, &scope)?;
            if previous.revision != value.revision {
                return Err(WorkspaceError::Invalid("Terminal layout changed in another window. Local tabs are retained. Stop the shells before reloading the saved layout.".into()));
            }
            if previous != value {
                value.revision = value.revision.checked_add(1).ok_or(StorageError::Limit)?;
                let encoded = encode(&value)?;
                if encoded.len() > MAX_LAYOUT_BYTES { return Err(StorageError::Limit.into()); }
                tx.execute("INSERT INTO preferences(key,data) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET data=excluded.data",
                    params![scope.key()?, encoded]).map_err(StorageError::from)?;
            }
            tx.commit().map_err(StorageError::from)?;
            Ok(value)
        }).await
    }
}
