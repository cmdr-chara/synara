//! Per-buffer, opt-in idle saves use the same compare-and-swap filesystem writer.
//! Nothing is enabled on restart, task switch, or a reopened retained buffer.
use super::*;
use std::{
    collections::{BTreeMap, HashSet},
    time::{Duration, Instant},
};

const IDLE: Duration = Duration::from_secs(1);
#[derive(Default)]
pub(super) struct Autosave {
    enabled: HashSet<u64>,
    due: BTreeMap<u64, Instant>,
    timer: Option<gpui::Task<()>>,
    pub(super) writing: bool,
}
impl Autosave {
    pub(super) fn forget(&mut self, id: u64) {
        self.enabled.remove(&id);
        self.due.remove(&id);
    }
    fn edit(&mut self, id: u64, now: Instant) {
        if self.enabled.contains(&id) {
            self.due.insert(id, now + IDLE);
        }
    }
    fn next(&self, now: Instant) -> Option<u64> {
        self.due
            .iter()
            .find_map(|(id, due)| (*due <= now && self.enabled.contains(id)).then_some(*id))
    }
}
impl Shell {
    pub(super) fn autosave_enabled(&self) -> bool {
        self.editors
            .active
            .is_some_and(|id| self.editors.autosave.enabled.contains(&id))
    }
    pub(super) fn toggle_editor_autosave(&mut self, cx: &mut Context<Self>) {
        let Some(id) = self.editors.active else {
            return;
        };
        if self.editors.autosave.enabled.contains(&id) {
            self.editors.autosave.forget(id);
            self.notice = Some(
                "Auto-save off for this buffer. A write already in progress may finish.".into(),
            );
        } else {
            if self.editor_batch_blocked(cx) || self.editors.close.is_some() {
                return;
            }
            if !matches!(self.workspace_target(), Some(WorkspaceTarget::Local { .. })) {
                self.error = Some("Auto-save currently requires a local workspace. Remote files still support explicit Save.".into());
                cx.notify();
                return;
            }
            self.editors.autosave.enabled.insert(id);
            self.editors.autosave.edit(id, Instant::now());
            self.notice = Some("Auto-save on for this buffer after one second idle. Disk conflicts pause it without overwriting. Closing the buffer turns it off.".into());
            if self.editors.autosave.timer.is_none() {
                self.editors.autosave.timer = Some(cx.spawn(async move |weak, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(200))
                            .await;
                        if weak
                            .update(cx, |this, cx| this.poll_editor_autosave(cx))
                            .is_err()
                        {
                            break;
                        }
                    }
                }));
            }
        }
        cx.notify();
    }
    pub(super) fn editor_autosave_changed(&mut self, input: gpui::EntityId) {
        if let Some(tab) = self
            .editors
            .tabs
            .iter()
            .find(|tab| tab.input.entity_id() == input)
        {
            self.editors.autosave.edit(tab.id, Instant::now());
        }
    }
    fn poll_editor_autosave(&mut self, cx: &mut Context<Self>) {
        if self.editor_batch_blocked(cx)
            || self.editors.close.is_some()
            || self.editors.loading.is_some()
        {
            return;
        }
        let Some(id) = self.editors.autosave.next(Instant::now()) else {
            return;
        };
        self.editors.autosave.due.remove(&id);
        self.sync_editor_document();
        let Some(tab) = self.editors.tabs.iter().find(|tab| tab.id == id) else {
            self.editors.autosave.forget(id);
            return;
        };
        if !tab.dirty(cx) {
            return;
        }
        let Some(WorkspaceTarget::Local { root }) = self.workspace_target() else {
            self.editors.autosave.forget(id);
            return;
        };
        let document = tab.document.clone();
        let path = document.path.clone();
        let text = tab.input.read(cx).text().to_owned();
        let project = self.project;
        let epoch = self.editors.save_epoch;
        let target = root.clone();
        self.saving = true;
        self.editors.autosave.writing = true;
        self.clear_conflict_reload_confirmation();
        let write = self.runtime.spawn(async move {
            let result = save_document(target, document, text.clone()).await;
            (text, result)
        });
        cx.spawn(async move |weak, cx| {
            let result = write.await;
            let _ = weak.update(cx, |this, cx| {
                if this.editors.save_epoch != epoch { return; }
                this.saving = false;
                this.editors.autosave.writing = false;
                if this.project != project || this.root().as_ref() != Some(&root) { return; }
                match result {
                    Ok((text, Ok(version))) => {
                        if let Some(tab) = this.editors.tabs.iter_mut().find(|tab| tab.id == id && tab.document.path == path) {
                            // Advance only the disk baseline. Later keystrokes keep their
                            // undo history, selection and dirty state in the live input.
                            tab.document.snapshot.text = text;
                            tab.document.snapshot.version = version;
                            if this.editors.active == Some(id) { this.document = Some(tab.document.clone()); }
                            if tab.dirty(cx) { this.editors.autosave.edit(id, Instant::now()); }
                        }
                    }
                    failed => {
                        this.editors.autosave.forget(id);
                        let reason = match failed {
                            Ok((_, Err(error))) => error.to_string(),
                            Err(_) => "file writer stopped unexpectedly".into(),
                            _ => unreachable!(),
                        };
                        this.error = Some(format!("Auto-save stopped at {}: {reason}. Auto-save is off for this buffer; your edits were kept.", path.display()));
                        this.notice = None;
                    }
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_saves_are_opt_in_debounced_and_retired_per_buffer() {
        let now = Instant::now();
        let mut owner = Autosave::default();
        owner.edit(1, now);
        assert_eq!(owner.next(now + IDLE), None);
        owner.enabled.extend([1, 2]);
        owner.edit(1, now);
        owner.edit(2, now);
        assert_eq!(owner.next(now), None);
        owner.edit(1, now + IDLE / 2);
        assert_eq!(owner.next(now + IDLE), Some(2));
        owner.forget(2);
        assert_eq!(owner.next(now + IDLE), None);
        assert_eq!(owner.next(now + IDLE * 2), Some(1));
        owner.forget(1);
        assert_eq!(owner.next(now + IDLE * 3), None);
    }
}
