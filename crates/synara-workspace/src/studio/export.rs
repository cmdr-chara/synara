//! User-reviewed, bounded Library exports reuse the existing no-overwrite writer.
use super::*;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct StudioExportReview {
    task: TaskId,
    root: PathBuf,
    path: PathBuf,
    bytes: Arc<[u8]>,
    reviewed_at: Instant,
}
impl StudioExportReview {
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn bytes(&self) -> usize {
        self.bytes.len()
    }
}
impl WorkspaceService {
    pub async fn review_studio_export(
        &self,
        task: TaskId,
        path: PathBuf,
    ) -> WorkspaceResult<StudioExportReview> {
        let root = self.local_studio_root(task).await?;
        if !visible(&path) {
            return Err(RuntimeError::Denied("This is not a visible Library file.".into()).into());
        }
        tokio::task::spawn_blocking(move || {
            let bytes = WorkspaceFs::open(&root)?.read_blob(&path)?.into();
            Ok(StudioExportReview {
                task,
                root,
                path,
                bytes,
                reviewed_at: Instant::now(),
            })
        })
        .await
        .map_err(|_| WorkspaceError::Worker)?
    }
    pub async fn export_studio_file(
        &self,
        review: StudioExportReview,
        destination: PathBuf,
        cancel: &CancellationToken,
    ) -> WorkspaceResult<()> {
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed.into());
        }
        if self.local_studio_root(review.task).await? != review.root
            || review.reviewed_at.elapsed() > Duration::from_secs(300)
        {
            return Err(WorkspaceError::Invalid(
                "The Library export review expired or its owner changed. Review the file again."
                    .into(),
            ));
        }
        let cancel = cancel.clone();
        tokio::task::spawn_blocking(move || {
            if cancel.is_cancelled() { return Err(RuntimeError::Closed.into()); }
            let current = WorkspaceFs::open(&review.root)?.read_blob(&review.path)?;
            if current.as_slice() != review.bytes.as_ref() {
                return Err(WorkspaceError::Invalid("The Library file changed after review. Nothing was exported. Review the current file again.".into()));
            }
            if cancel.is_cancelled() { return Err(RuntimeError::Closed.into()); }
            crate::storage::write_new_export(&destination, &review.bytes)?;
            Ok(())
        }).await.map_err(|_| WorkspaceError::Worker)?
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn library_export_preserves_bytes_refuses_stale_symlink_and_existing_destinations() {
        let root = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let bytes = b"\0Binary\xff\n";
        std::fs::write(root.path().join("result.bin"), bytes).unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(root.path().into())
            .await
            .unwrap();
        let agent = service.profiles().await.unwrap()[0].id.clone();
        let task = service
            .create_scoped_task(project.id, "Output".into(), agent, TaskScope::Studio)
            .await
            .unwrap();
        let review = service
            .review_studio_export(task.id, "result.bin".into())
            .await
            .unwrap();
        let destination = out.path().join("copy.bin");
        let cancel = CancellationToken::new();
        service
            .export_studio_file(review.clone(), destination.clone(), &cancel)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        assert!(
            service
                .export_studio_file(review.clone(), destination.clone(), &cancel)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&destination).unwrap(), bytes);
        std::fs::write(root.path().join("result.bin"), b"New disk revision").unwrap();
        assert!(
            service
                .export_studio_file(review, out.path().join("stale.bin"), &cancel)
                .await
                .is_err()
        );
        assert!(!out.path().join("stale.bin").exists());
        assert!(
            service
                .review_studio_export(task.id, "../result.bin".into())
                .await
                .is_err()
        );
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("result.bin", root.path().join("link.bin")).unwrap();
            assert!(
                service
                    .review_studio_export(task.id, "link.bin".into())
                    .await
                    .is_err()
            );
        }
        let review = service
            .review_studio_export(task.id, "result.bin".into())
            .await
            .unwrap();
        cancel.cancel();
        assert!(
            service
                .export_studio_file(review, out.path().join("cancelled.bin"), &cancel)
                .await
                .is_err()
        );
        assert!(!out.path().join("cancelled.bin").exists());
    }
}
