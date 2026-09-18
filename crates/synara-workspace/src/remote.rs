use crate::{WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use synara_core::{Workspace, WorkspaceId, WorkspaceLocation};
use synara_runtime::{PinnedSshHost, RemoteWorkspaceFs, SshTarget};

/// Local connection metadata for an SSH workspace. These are paths to user-owned
/// trust/authentication files, never private-key or known-host contents.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SshWorkspaceProfile {
    pub workspace_id: WorkspaceId,
    pub known_hosts: PathBuf,
    pub identity_file: PathBuf,
    pub helper: PathBuf,
}

impl SshWorkspaceProfile {
    pub fn host(&self, workspace: &Workspace) -> WorkspaceResult<PinnedSshHost> {
        if workspace.id != self.workspace_id {
            return Err(WorkspaceError::Invalid(
                "SSH connection profile does not belong to this workspace".into(),
            ));
        }
        let WorkspaceLocation::Ssh {
            host, port, user, ..
        } = &workspace.location
        else {
            return Err(WorkspaceError::Invalid(
                "SSH connection profile was requested for a local workspace".into(),
            ));
        };
        Ok(PinnedSshHost::new(
            SshTarget {
                host: host.clone(),
                port: *port,
                user: user.clone(),
            },
            &self.known_hosts,
            &self.identity_file,
        )?)
    }

    pub async fn filesystem(
        &self,
        workspace: &Workspace,
        root: &Path,
    ) -> WorkspaceResult<RemoteWorkspaceFs> {
        Ok(RemoteWorkspaceFs::connect(self.host(workspace)?, root, &self.helper).await?)
    }
}

#[derive(Clone, Debug)]
pub struct NewSshWorkspace {
    pub name: String,
    pub target: SshTarget,
    pub root: String,
    pub known_hosts: PathBuf,
    pub identity_file: PathBuf,
    pub helper: PathBuf,
}

impl NewSshWorkspace {
    pub fn validate(&self) -> WorkspaceResult<()> {
        self.target.validate()?;
        if self.name.len() > 200 || self.name.chars().any(char::is_control) {
            return Err(WorkspaceError::Invalid(
                "remote workspace name is invalid".into(),
            ));
        }
        if !self.root.starts_with('/') || self.root.contains('\0') {
            return Err(WorkspaceError::Invalid(
                "remote workspace root must be an absolute POSIX path".into(),
            ));
        }
        if self.helper.as_os_str().is_empty() {
            return Err(WorkspaceError::Invalid(
                "remote helper command is required".into(),
            ));
        }
        Ok(())
    }
}

pub(crate) fn upsert_ssh_profile(
    profiles: &mut Vec<SshWorkspaceProfile>,
    profile: SshWorkspaceProfile,
) -> WorkspaceResult<()> {
    if profiles.len() >= 256
        && !profiles
            .iter()
            .any(|item| item.workspace_id == profile.workspace_id)
    {
        return Err(synara_runtime::RuntimeError::Limit.into());
    }
    if let Some(existing) = profiles
        .iter_mut()
        .find(|item| item.workspace_id == profile.workspace_id)
    {
        *existing = profile;
    } else {
        profiles.push(profile);
    }
    Ok(())
}
