//! Prevent two native processes from running startup recovery against one database.
use crate::RuntimeError;
use cap_fs_ext::{FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use std::{fs::File, path::Path};

/// A process-lifetime advisory lock for the database's canonical parent and filename.
/// Keep this value alive until the controller and workspace service have shut down.
pub struct WorkspaceOwnerLock {
    _file: File,
}

impl WorkspaceOwnerLock {
    pub fn acquire(database_path: &Path) -> Result<Self, RuntimeError> {
        let absolute = std::path::absolute(database_path)?;
        let is_symlink = match absolute.symlink_metadata() {
            Ok(metadata) => metadata.file_type().is_symlink(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        let resolved = match absolute.canonicalize() {
            Ok(path) => path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !is_symlink => absolute,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Err(RuntimeError::Denied(
                    "database symlink target does not exist".into(),
                ));
            }
            Err(error) => return Err(error.into()),
        };
        let parent = resolved
            .parent()
            .ok_or_else(|| RuntimeError::Invalid("database path has no parent".into()))?
            .canonicalize()?;
        let name = resolved
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| RuntimeError::Invalid("database filename is not UTF-8".into()))?;
        let lock_name = format!(".{name}.synara-owner.lock");
        let directory = Dir::open_ambient_dir(parent, ambient_authority())?;
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(true)
            .follow(FollowSymlinks::No);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = directory.open_with(&lock_name, &options)?.into_std();
        if !file.metadata()?.is_file() {
            return Err(RuntimeError::Denied(
                "workspace owner lock is not a regular file".into(),
            ));
        }
        file.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => RuntimeError::Denied(
                "this workspace database is already open in another Synara process".into(),
            ),
            std::fs::TryLockError::Error(error) => RuntimeError::Io(error),
        })?;
        Ok(Self { _file: file })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_process_owner_can_recover_a_database() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        let first = WorkspaceOwnerLock::acquire(&database).unwrap();
        assert!(WorkspaceOwnerLock::acquire(&database).is_err());
        drop(first);
        assert!(WorkspaceOwnerLock::acquire(&database).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn owner_lock_refuses_a_symlink_sidecar() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target");
        std::fs::write(&target, b"").unwrap();
        symlink(
            &target,
            directory
                .path()
                .join(".workspace.sqlite3.synara-owner.lock"),
        )
        .unwrap();
        assert!(WorkspaceOwnerLock::acquire(&directory.path().join("workspace.sqlite3")).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn database_symlink_alias_uses_the_same_owner_lock() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("workspace.sqlite3");
        std::fs::write(&database, b"").unwrap();
        let alias = directory.path().join("alias.sqlite3");
        symlink(&database, &alias).unwrap();
        let owner = WorkspaceOwnerLock::acquire(&database).unwrap();
        assert!(WorkspaceOwnerLock::acquire(&alias).is_err());
        drop(owner);
        assert!(WorkspaceOwnerLock::acquire(&alias).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn dangling_database_symlink_cannot_bypass_owner_lock() {
        use std::os::unix::fs::symlink;
        let directory = tempfile::tempdir().unwrap();
        let alias = directory.path().join("alias.sqlite3");
        symlink(directory.path().join("workspace.sqlite3"), &alias).unwrap();
        assert!(WorkspaceOwnerLock::acquire(&alias).is_err());
    }
}
