//! Per-task observation leases, with one global input lane for the shared pointer.
//! Selection and frame authority are never persisted or restored.
use super::workflow::invalid;
use crate::{WorkspaceError, WorkspaceResult};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use synara_core::TaskId;
use synara_runtime::{ComputerAction, ComputerTools, SnapWindow};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub struct ComputerFrame {
    pub id: Uuid,
    pub window: SnapWindow,
    pub png: Arc<Vec<u8>>,
    pub captured: Instant,
    digest: [u8; 32],
}
struct Target {
    epoch: Uuid,
    window: SnapWindow,
    tools: ComputerTools,
    cancel: CancellationToken,
    expires: Instant,
    frame: Option<ComputerFrame>,
}
#[derive(Clone, Default)]
pub struct ComputerService {
    targets: Arc<Mutex<HashMap<TaskId, Target>>>,
    input: Arc<tokio::sync::Mutex<()>>,
}
impl ComputerService {
    pub fn select(
        &self,
        task: TaskId,
        tools: ComputerTools,
        window: SnapWindow,
    ) -> WorkspaceResult<()> {
        if window.pid == std::process::id() {
            return Err(invalid("Synara's own window cannot be selected"));
        }
        let mut targets = self.targets.lock().map_err(|_| WorkspaceError::Worker)?;
        if targets.len() >= 8 && !targets.contains_key(&task) {
            return Err(invalid("Computer target limit reached"));
        }
        if let Some(old) = targets.remove(&task) {
            old.cancel.cancel();
        }
        targets.insert(
            task,
            Target {
                epoch: Uuid::new_v4(),
                window,
                tools,
                cancel: CancellationToken::new(),
                expires: Instant::now() + Duration::from_secs(15 * 60),
                frame: None,
            },
        );
        Ok(())
    }
    pub fn selected(&self, task: TaskId) -> Option<SnapWindow> {
        let targets = self.targets.lock().ok()?;
        targets
            .get(&task)
            .filter(|v| !v.cancel.is_cancelled() && Instant::now() < v.expires)
            .map(|v| v.window.clone())
    }
    pub fn selection_id(&self, task: TaskId) -> Option<Uuid> {
        let targets = self.targets.lock().ok()?;
        targets
            .get(&task)
            .filter(|v| !v.cancel.is_cancelled() && Instant::now() < v.expires)
            .map(|v| v.epoch)
    }
    pub fn frame(&self, task: TaskId) -> Option<ComputerFrame> {
        let targets = self.targets.lock().ok()?;
        targets
            .get(&task)
            .filter(|v| !v.cancel.is_cancelled() && Instant::now() < v.expires)
            .and_then(|v| v.frame.clone())
    }
    fn snapshot(
        &self,
        task: TaskId,
    ) -> WorkspaceResult<(Uuid, SnapWindow, ComputerTools, CancellationToken)> {
        let mut targets = self.targets.lock().map_err(|_| WorkspaceError::Worker)?;
        let target = targets
            .get_mut(&task)
            .ok_or_else(|| invalid("Select an application window in Computer Use first"))?;
        if Instant::now() >= target.expires {
            target.cancel.cancel();
            target.frame = None;
        }
        if target.cancel.is_cancelled() {
            return Err(invalid(
                "Computer control was revoked or expired. Select the target again",
            ));
        }
        Ok((
            target.epoch,
            target.window.clone(),
            target.tools.clone(),
            target.cancel.child_token(),
        ))
    }
    pub async fn observe(
        &self,
        task: TaskId,
        expected_target: Uuid,
        cancel: CancellationToken,
    ) -> WorkspaceResult<ComputerFrame> {
        let (epoch, window, tools, operation) = self.snapshot(task)?;
        if epoch != expected_target {
            return Err(invalid("Selected target changed after observation request"));
        }
        let _lane = tokio::select! { biased;
            _ = cancel.cancelled() => return Err(invalid("Observation cancelled")),
            _ = operation.cancelled() => return Err(invalid("Target revoked")),
            lane = self.input.lock() => lane,
        };
        let result = tools.observe(&window, &operation);
        tokio::pin!(result);
        let png = tokio::select! { biased;
            _ = cancel.cancelled() => { operation.cancel(); let _ = result.await; return Err(invalid("Observation cancelled")); }
            result = &mut result => result?,
        };
        // Independently validate decoding before passing image bytes to GPUI/MCP.
        let owned = png.clone();
        let (width, height) = (window.width, window.height);
        tokio::task::spawn_blocking(move || {
            let image = image::load_from_memory_with_format(&owned, image::ImageFormat::Png)
                .map_err(|_| invalid("Capture is not a valid PNG"))?;
            if image.width() != width || image.height() != height {
                return Err(invalid("Captured dimensions changed"));
            }
            Ok(())
        })
        .await
        .map_err(|_| WorkspaceError::Worker)??;
        let frame = ComputerFrame {
            id: Uuid::new_v4(),
            window: window.clone(),
            digest: Sha256::digest(&png).into(),
            png: Arc::new(png),
            captured: Instant::now(),
        };
        let mut targets = self.targets.lock().map_err(|_| WorkspaceError::Worker)?;
        let target = targets
            .get_mut(&task)
            .ok_or_else(|| invalid("Target revoked during capture"))?;
        if target.epoch != epoch
            || target.cancel.is_cancelled()
            || cancel.is_cancelled()
            || Instant::now() >= target.expires
        {
            return Err(invalid("Late observation discarded"));
        }
        target.frame = Some(frame.clone());
        Ok(frame)
    }
    pub fn validate_action(
        &self,
        task: TaskId,
        frame: Uuid,
        action: &ComputerAction,
    ) -> WorkspaceResult<()> {
        let targets = self.targets.lock().map_err(|_| WorkspaceError::Worker)?;
        let target = targets
            .get(&task)
            .ok_or_else(|| invalid("No selected computer target"))?;
        let observed = target
            .frame
            .as_ref()
            .ok_or_else(|| invalid("Capture the target before requesting input"))?;
        validate_frame(
            observed,
            frame,
            target.expires,
            target.cancel.is_cancelled(),
        )?;
        action.validate(target.window.width, target.window.height)?;
        Ok(())
    }
    pub async fn act(
        &self,
        task: TaskId,
        frame_id: Uuid,
        action: ComputerAction,
        cancel: CancellationToken,
    ) -> WorkspaceResult<()> {
        self.validate_action(task, frame_id, &action)?;
        let (epoch, window, tools, operation) = self.snapshot(task)?;

        let _lane = tokio::select! { biased;
            _ = cancel.cancelled() => return Err(invalid("Input cancelled before dispatch")),
            _ = operation.cancelled() => return Err(invalid("Target revoked")),
            lane = self.input.lock() => lane,
        };
        let frame = {
            let mut targets = self.targets.lock().map_err(|_| WorkspaceError::Worker)?;
            let target = targets
                .get_mut(&task)
                .ok_or_else(|| invalid("Target revoked"))?;
            if target.epoch != epoch {
                return Err(invalid("Selected target changed"));
            }
            let observed = target
                .frame
                .as_ref()
                .ok_or_else(|| invalid("Frame already consumed"))?;
            validate_frame(
                observed,
                frame_id,
                target.expires,
                target.cancel.is_cancelled(),
            )?;
            target
                .frame
                .take()
                .ok_or_else(|| invalid("Frame already consumed"))?
        };
        // Consume first, then compare a fresh observation. Dynamic/stale screens
        // fail closed and require another explicit observation, never an auto retry.
        let work = async {
            let fresh = tools.observe(&window, &operation).await?;
            let digest: [u8; 32] = Sha256::digest(&fresh).into();
            if digest != frame.digest || frame.captured.elapsed() > Duration::from_secs(60) {
                return Err(invalid(
                    "Window pixels changed after observation. Observe and review again",
                ));
            }
            {
                let targets = self.targets.lock().map_err(|_| WorkspaceError::Worker)?;
                let target = targets
                    .get(&task)
                    .ok_or_else(|| invalid("Target revoked during fresh observation"))?;
                if target.epoch != epoch {
                    return Err(invalid("Target changed before input"));
                }
                validate_frame(
                    &frame,
                    frame_id,
                    target.expires,
                    target.cancel.is_cancelled() || cancel.is_cancelled(),
                )?;
            }
            tools.act(&window, &action, &operation).await?;
            Ok(())
        };
        tokio::pin!(work);
        tokio::select! { biased;
            _ = cancel.cancelled() => { operation.cancel(); let _ = work.await; Err(invalid("Input interrupted. Some events may already have been delivered")) }
            result = &mut work => result,
        }
    }
    pub fn revoke(&self, task: TaskId) {
        if let Ok(mut targets) = self.targets.lock()
            && let Some(target) = targets.remove(&task)
        {
            target.cancel.cancel();
        }
    }
    pub fn shutdown(&self) {
        if let Ok(mut targets) = self.targets.lock() {
            for (_, target) in targets.drain() {
                target.cancel.cancel();
            }
        }
    }
}
fn validate_frame(
    frame: &ComputerFrame,
    expected: Uuid,
    expires: Instant,
    cancelled: bool,
) -> WorkspaceResult<()> {
    if frame.id != expected
        || cancelled
        || Instant::now() >= expires
        || frame.captured.elapsed() > Duration::from_secs(60)
    {
        return Err(invalid(
            "Observation is stale, consumed, revoked or belongs to another window",
        ));
    }
    Ok(())
}
pub(crate) fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = *chunk.get(1).unwrap_or(&0);
        let c = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[usize::from(a >> 2)] as char);
        out.push(TABLE[usize::from(((a & 3) << 4) | (b >> 4))] as char);
        out.push(if chunk.len() > 1 {
            TABLE[usize::from(((b & 15) << 2) | (c >> 6))] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[usize::from(c & 63)] as char
        } else {
            '='
        });
    }
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_target_has_no_implicit_desktop_authority() {
        let owner = ComputerService::default();
        let task = TaskId::new();
        assert!(owner.selected(task).is_none());
        assert!(owner.frame(task).is_none());
        assert!(
            owner
                .validate_action(
                    task,
                    Uuid::new_v4(),
                    &ComputerAction::Key {
                        key: synara_runtime::ComputerKey::Enter
                    }
                )
                .is_err()
        );
        owner.revoke(task);
        owner.shutdown();
    }
    #[test]
    fn image_encoding_keeps_exact_padding() {
        for (bytes, expected) in [
            (b"".as_slice(), ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (&[0, 255, 128], "AP+A"),
        ] {
            assert_eq!(base64(bytes), expected);
        }
    }
}
