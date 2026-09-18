use crate::RuntimeError;
use cap_fs_ext::{DirExt, FollowSymlinks, OpenOptionsFollowExt};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsString,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileVersion(pub String);
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub text: String,
    pub version: FileVersion,
    pub utf8_bom: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub relative_path: PathBuf,
    pub directory: bool,
    pub symlink: bool,
}

/// Every traversed directory is opened without following symlinks. Handles, rather than
/// canonicalize-then-open strings, retain the workspace boundary during concurrent renames.
pub struct WorkspaceFs {
    root: PathBuf,
    requested_root: PathBuf,
    dir: Dir,
    max_bytes: usize,
    write_lock: Mutex<()>,
}
impl WorkspaceFs {
    pub fn open(root: &Path) -> Result<Self, RuntimeError> {
        let requested_root = std::path::absolute(root)?;
        let root = root.canonicalize()?;
        let dir = Dir::open_ambient_dir(&root, ambient_authority())?;
        Ok(Self {
            root,
            requested_root,
            dir,
            max_bytes: 8 * 1024 * 1024,
            write_lock: Mutex::new(()),
        })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn relative(&self, path: &Path) -> Result<PathBuf, RuntimeError> {
        let relative = if path.is_absolute() {
            path.strip_prefix(&self.root)
                .or_else(|_| path.strip_prefix(&self.requested_root))
                .map_err(|_| RuntimeError::Denied("path is outside this workspace".into()))?
        } else {
            path
        };
        validate_relative(relative)?;
        Ok(relative.to_owned())
    }
    fn directory(&self, relative: &Path) -> Result<Dir, RuntimeError> {
        let mut current = self.dir.try_clone()?;
        for component in relative.components() {
            match component {
                Component::Normal(name) => current = current.open_dir_nofollow(name)?,
                Component::CurDir => {}
                _ => return Err(RuntimeError::Denied("invalid directory component".into())),
            }
        }
        Ok(current)
    }
    fn parent(&self, path: &Path) -> Result<(Dir, OsString), RuntimeError> {
        let relative = self.relative(path)?;
        let name = relative
            .file_name()
            .ok_or_else(|| RuntimeError::Denied("expected a file".into()))?
            .to_owned();
        let dir = self.directory(relative.parent().unwrap_or(Path::new("")))?;
        Ok((dir, name))
    }
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>, RuntimeError> {
        let (dir, name) = self.parent(path)?;
        if dir.symlink_metadata(&name)?.file_type().is_symlink() {
            return Err(RuntimeError::Denied(
                "symbolic links are not readable workspace files".into(),
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true).follow(FollowSymlinks::No);
        let file = dir.open_with(name, &options)?;
        if !file.metadata()?.is_file() {
            return Err(RuntimeError::Denied(
                "only regular files are readable".into(),
            ));
        }
        if file.metadata()?.len() > self.max_bytes as u64 {
            return Err(RuntimeError::Limit);
        }
        let mut bytes = Vec::new();
        file.take(self.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > self.max_bytes {
            return Err(RuntimeError::Limit);
        }
        Ok(bytes)
    }
    pub fn read(&self, path: &Path) -> Result<FileSnapshot, RuntimeError> {
        let bytes = self.read_bytes(path)?;
        let version = FileVersion(hex::encode(Sha256::digest(&bytes)));
        let bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
        let content = if bom { &bytes[3..] } else { &bytes[..] };
        if content.contains(&0) {
            return Err(RuntimeError::Unsupported("binary file".into()));
        }
        let text = std::str::from_utf8(content)
            .map_err(|_| RuntimeError::Unsupported("file is not UTF-8".into()))?
            .to_owned();
        Ok(FileSnapshot {
            text,
            version,
            utf8_bom: bom,
        })
    }
    pub fn read_lines(
        &self,
        path: &Path,
        first: Option<u64>,
        count: Option<u64>,
    ) -> Result<String, RuntimeError> {
        let snapshot = self.read(path)?;
        let first = first.unwrap_or(1);
        if first == 0 || count == Some(0) {
            return Err(RuntimeError::Invalid(
                "line numbers and limits must be positive".into(),
            ));
        }
        let start = usize::try_from(first - 1).map_err(|_| RuntimeError::Limit)?;
        let count = count
            .map(usize::try_from)
            .transpose()
            .map_err(|_| RuntimeError::Limit)?
            .unwrap_or(usize::MAX);
        Ok(snapshot
            .text
            .split_inclusive('\n')
            .skip(start)
            .take(count)
            .collect())
    }
    pub fn write(
        &self,
        path: &Path,
        text: &str,
        expected: Option<&FileVersion>,
        bom: bool,
    ) -> Result<FileVersion, RuntimeError> {
        self.write_impl(path, text, expected, bom, false)
    }
    /// Atomically publishes a new file without replacing a file created concurrently.
    pub fn write_new(
        &self,
        path: &Path,
        text: &str,
        bom: bool,
    ) -> Result<FileVersion, RuntimeError> {
        self.write_impl(path, text, None, bom, true)
    }
    fn write_impl(
        &self,
        path: &Path,
        text: &str,
        expected: Option<&FileVersion>,
        bom: bool,
        must_be_absent: bool,
    ) -> Result<FileVersion, RuntimeError> {
        if text.len().saturating_add(if bom { 3 } else { 0 }) > self.max_bytes {
            return Err(RuntimeError::Limit);
        }
        let _lock = self.write_lock.lock().map_err(|_| RuntimeError::Closed)?;
        let (dir, name) = self.parent(path)?;
        if let Some(expected) = expected
            && &self.read(path)?.version != expected
        {
            return Err(RuntimeError::Conflict);
        }
        if dir
            .symlink_metadata(&name)
            .is_ok_and(|metadata| metadata.is_symlink())
        {
            return Err(RuntimeError::Denied(
                "writing through a symlink is disabled".into(),
            ));
        }
        let temporary = format!(".synara-write-{}", uuid::Uuid::new_v4());
        let mut options = OpenOptions::new();
        options
            .write(true)
            .create_new(true)
            .follow(FollowSymlinks::No);
        let mut file = dir.open_with(&temporary, &options)?;
        let result = (|| {
            if bom {
                file.write_all(&[0xef, 0xbb, 0xbf])?;
            }
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            if let Ok(metadata) = dir.metadata(&name) {
                file.set_permissions(metadata.permissions())?;
            }
            if let Some(expected) = expected
                && &self.read(path)?.version != expected
            {
                return Err(RuntimeError::Conflict);
            }
            if must_be_absent {
                dir.hard_link(&temporary, &dir, &name)?;
                dir.remove_file(&temporary)?;
            } else {
                dir.rename(&temporary, &dir, &name)?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = dir.remove_file(&temporary);
        }
        result?;
        let mut hash = Sha256::new();
        if bom {
            hash.update([0xef, 0xbb, 0xbf]);
        }
        hash.update(text.as_bytes());
        Ok(FileVersion(hex::encode(hash.finalize())))
    }
    pub fn entries(&self, path: &Path) -> Result<Vec<FileEntry>, RuntimeError> {
        let relative = if path.as_os_str().is_empty() {
            PathBuf::new()
        } else {
            self.relative(path)?
        };
        let dir = self.directory(&relative)?;
        let mut entries = vec![];
        for entry in dir.entries()? {
            let entry = entry?;
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            let path = relative.join(name);
            if validate_relative(&path).is_err() {
                continue;
            }
            let kind = entry.file_type()?;
            entries.push(FileEntry {
                name: name.into(),
                relative_path: path,
                directory: kind.is_dir(),
                symlink: kind.is_symlink(),
            });
            if entries.len() >= 10_000 {
                return Err(RuntimeError::Limit);
            }
        }
        entries.sort_by(|a, b| {
            b.directory
                .cmp(&a.directory)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        Ok(entries)
    }
}

pub fn validate_relative(path: &Path) -> Result<(), RuntimeError> {
    if path.as_os_str().is_empty() {
        return Err(RuntimeError::Denied("empty path".into()));
    }
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                let name = name
                    .to_str()
                    .ok_or_else(|| RuntimeError::Unsupported("path is not UTF-8".into()))?;
                let lower = name.to_ascii_lowercase();
                if name.contains(['\0', '\\', ':'])
                    || lower == ".git"
                    || lower == ".ssh"
                    || lower == ".env"
                    || lower.starts_with(".env.")
                {
                    return Err(RuntimeError::Denied("protected path".into()));
                }
            }
            Component::CurDir => {}
            _ => return Err(RuntimeError::Denied("path traversal is not allowed".into())),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn traversal_and_protected_paths_are_rejected() {
        for name in [
            "../secret",
            "/etc/passwd",
            "src/../../x",
            ".git/config",
            "x/.env",
            "C:\\secret",
            "file:stream",
        ] {
            assert!(validate_relative(Path::new(name)).is_err(), "{name}");
        }
    }
    #[test]
    fn guarded_write_detects_external_changes() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a"), "first").unwrap();
        let fs = WorkspaceFs::open(root.path()).unwrap();
        let initial = fs.read(Path::new("a")).unwrap();
        std::fs::write(root.path().join("a"), "external").unwrap();
        assert!(matches!(
            fs.write(Path::new("a"), "changed", Some(&initial.version), false),
            Err(RuntimeError::Conflict)
        ));
        assert_eq!(fs.read(Path::new("a")).unwrap().text, "external");
    }
    #[test]
    fn utf8_and_line_endings_survive_read_and_write() {
        let root = tempfile::tempdir().unwrap();
        let fs = WorkspaceFs::open(root.path()).unwrap();
        fs.write(Path::new("a"), "æ\r\n中\r\n", None, true).unwrap();
        assert_eq!(
            fs.read_lines(Path::new("a"), Some(2), Some(1)).unwrap(),
            "中\r\n"
        );
        assert!(fs.read(Path::new("a")).unwrap().utf8_bom);
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_cannot_escape_or_alias_protected_paths() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "private").unwrap();
        symlink(outside.path(), root.path().join("link")).unwrap();
        let fs = WorkspaceFs::open(root.path()).unwrap();
        assert!(fs.read(Path::new("link/secret")).is_err());
        assert!(fs.write(Path::new("link/new"), "bad", None, false).is_err());
        assert!(!outside.path().join("new").exists());
    }
}

#[cfg(all(test, unix))]
mod root_alias_tests {
    use super::*;
    #[test]
    fn chosen_root_alias_is_bound_to_the_open_directory_not_later_alias_changes() {
        let directory = tempfile::tempdir().unwrap();
        let original = directory.path().join("original");
        let outside = directory.path().join("outside");
        std::fs::create_dir(&original).unwrap();
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(original.join("file"), "original").unwrap();
        std::fs::write(outside.join("file"), "outside").unwrap();
        let alias = directory.path().join("alias");
        std::os::unix::fs::symlink(&original, &alias).unwrap();
        let fs = WorkspaceFs::open(&alias).unwrap();
        assert_eq!(fs.read(&alias.join("file")).unwrap().text, "original");
        std::fs::remove_file(&alias).unwrap();
        std::os::unix::fs::symlink(&outside, &alias).unwrap();
        assert_eq!(fs.read(&alias.join("file")).unwrap().text, "original");
        assert!(fs.read(&outside.join("file")).is_err());
        assert!(fs.read(&alias.join("../outside/file")).is_err());
    }
}
