//! Per-task attachment presentation. Selected bytes stay in the workspace store,
//! not the text editor, and late picker/results never attach to another task.
use super::*;
use crate::ui::{self, Glyph, palette};
mod view;

pub(super) enum Reply {
    Loaded(TaskId, Result<AttachmentDraft, String>),
    Imported(TaskId, bool, Result<AttachmentDraft, String>),
    Changed(TaskId, Result<AttachmentDraft, String>),
    Preview(TaskId, String, Result<AttachmentPreview, String>),
}
struct Submitted {
    display: String,
    ids: Vec<String>,
    after_sequence: u64,
}
enum Preview {
    Text(String),
    Image(Arc<gpui::Image>, (u32, u32)),
}
#[derive(Default)]
pub(super) struct AttachmentState {
    task: Option<TaskId>,
    value: Option<AttachmentDraft>,
    loading: bool,
    writes: HashMap<TaskId, usize>,
    picking: bool,
    imports: HashMap<TaskId, Vec<AttachmentInput>>,
    errors: HashMap<TaskId, String>,
    submitted: HashMap<TaskId, Submitted>,
    preview_id: Option<String>,
    preview: Option<Preview>,
    preview_loading: bool,
    recent_open: bool,
}
impl AttachmentState {
    pub fn close_pending(&self) -> bool {
        self.picking || !self.writes.is_empty() || !self.imports.is_empty()
    }
    fn changing(&self, task: TaskId) -> bool {
        self.writes.contains_key(&task) || self.imports.contains_key(&task) || self.picking
    }
    fn begin(&mut self, task: TaskId) {
        *self.writes.entry(task).or_default() += 1;
    }
    fn end(&mut self, task: TaskId) {
        if let Some(n) = self.writes.get_mut(&task) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.writes.remove(&task);
            }
        }
    }
}
impl Shell {
    pub(super) fn load_attachments(&mut self, task: TaskId) {
        self.attachments.task = Some(task);
        self.attachments.value = None;
        self.attachments.loading = true;
        self.attachments.preview = None;
        self.attachments.preview_id = None;
        self.attachments.preview_loading = false;
        self.attachments.recent_open = false;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Attachments(Box::new(Reply::Loaded(
                task,
                workspace
                    .attachment_draft(task)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
    }
    pub(super) fn attachments_have_pending(&self) -> bool {
        self.attachments
            .value
            .as_ref()
            .is_some_and(|v| !v.pending.is_empty())
    }
    pub(super) fn attachment_send_blocked(&self) -> bool {
        self.selected.is_none_or(|task| {
            self.attachments.task != Some(task)
                || self.attachments.loading
                || self.attachments.value.is_none()
                || self.attachments.changing(task)
        })
    }
    pub(super) fn attachment_capability_error(&self) -> Option<&'static str> {
        if self.uses_direct_model() {
            return self.direct_attachment_error(
                self.attachments
                    .value
                    .as_ref()
                    .is_some_and(|v| v.pending.iter().any(|a| a.kind.is_image())),
            );
        }
        self.details.as_ref().and_then(|details| {
            self.attachments
                .value
                .as_ref()
                .and_then(|value| value.unsupported(&details.connection.capabilities))
        })
    }
    pub(super) fn attachment_submission(&mut self, text: &str) -> Option<(u64, String)> {
        let task = self.selected?;
        let value = self.attachments.value.as_ref()?;
        if value.pending.is_empty() {
            return None;
        }
        let display = value.transcript_text(text);
        self.attachments.submitted.insert(
            task,
            Submitted {
                display: display.clone(),
                ids: value.pending.iter().map(|a| a.id.clone()).collect(),
                after_sequence: self
                    .thread
                    .as_ref()
                    .map_or(0, |thread| thread.last_sequence),
            },
        );
        Some((value.revision, display))
    }
    pub(super) fn finish_attachment_submission(&mut self, task: TaskId, succeeded: bool) {
        let Some(sent) = self.attachments.submitted.remove(&task) else {
            return;
        };
        // A lagged broadcast may have skipped the local echo. Successful prompt
        // completion retires those same IDs. Errors unlock edits but keep them pending.
        if succeeded {
            self.attachments.begin(task);
            let workspace = self.controller.workspace.clone();
            self.job(async move {
                Ok(Update::Attachments(Box::new(Reply::Changed(
                    task,
                    workspace
                        .acknowledge_attachments(task, sent.ids)
                        .await
                        .map_err(|e| e.to_string()),
                ))))
            });
        }
    }
    pub(super) fn acknowledge_attachment_event(&mut self, envelope: &EventEnvelope) {
        let ThreadEvent::TextDelta {
            role: Role::User,
            text,
            ..
        } = &envelope.event
        else {
            return;
        };
        let Some(task) = self
            .catalog
            .tasks
            .iter()
            .find(|t| t.thread_id == envelope.thread_id)
            .map(|t| t.id)
        else {
            return;
        };
        if !self
            .attachments
            .submitted
            .get(&task)
            .is_some_and(|sent| sent.display == *text && envelope.sequence > sent.after_sequence)
        {
            return;
        }
        let sent = self
            .attachments
            .submitted
            .remove(&task)
            .expect("submission checked");
        self.attachments.begin(task);
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Attachments(Box::new(Reply::Changed(
                task,
                workspace
                    .acknowledge_attachments(task, sent.ids)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
    }
    pub(super) fn attachment_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Self>) {
        self.import_selected(
            paths
                .into_iter()
                .map(|path| {
                    if path.is_dir() {
                        AttachmentInput::Folder(path)
                    } else {
                        AttachmentInput::File(path)
                    }
                })
                .collect(),
            cx,
        );
    }
    pub(super) fn attachment_paste(
        &mut self,
        images: Vec<(String, Vec<u8>)>,
        cx: &mut Context<Self>,
    ) {
        self.import_selected(
            images
                .into_iter()
                .map(|(name, bytes)| AttachmentInput::Bytes { name, bytes })
                .collect(),
            cx,
        );
    }
    pub(super) fn attach_capture(
        &mut self,
        name: String,
        bytes: Vec<u8>,
        source: ImageSource,
        cx: &mut Context<Self>,
    ) {
        self.import_selected(
            vec![AttachmentInput::Capture {
                name,
                bytes,
                source,
            }],
            cx,
        );
    }
    fn import_selected(&mut self, inputs: Vec<AttachmentInput>, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        if self.close != CloseState::Open
            || self.hubs.pending(cx)
            || self.attachment_send_blocked()
            || self.attachments.submitted.contains_key(&task)
        {
            self.error =
                Some("Wait for attachment loading/saving before adding more files.".into());
            cx.notify();
            return;
        }
        let revision = self
            .attachments
            .value
            .as_ref()
            .expect("loaded attachments")
            .revision;
        self.import_attachments(task, revision, inputs, cx);
    }
    fn import_attachments(
        &mut self,
        task: TaskId,
        revision: u64,
        inputs: Vec<AttachmentInput>,
        cx: &mut Context<Self>,
    ) {
        if inputs.is_empty() {
            return;
        }
        if self.attachments.imports.len() >= 8 && !self.attachments.imports.contains_key(&task) {
            self.error=Some("Resolve another pending attachment import before adding more. The clipboard and source files are unchanged.".into());
            cx.notify();
            return;
        }
        if inputs.len() > 8
            || inputs
                .iter()
                .filter_map(|i| match i {
                    AttachmentInput::Bytes { bytes, .. }
                    | AttachmentInput::Capture { bytes, .. } => Some(bytes.len()),
                    _ => None,
                })
                .sum::<usize>()
                > MAX_ATTACHMENT_BATCH_BYTES
        {
            self.error = Some(
                "Choose up to eight files, at most 2 MiB combined. Nothing was attached.".into(),
            );
            cx.notify();
            return;
        }
        self.attachments.imports.insert(task, inputs.clone());
        self.attachments.errors.remove(&task);
        self.attachments.begin(task);
        let preview_folder = inputs
            .iter()
            .any(|input| matches!(input, AttachmentInput::Folder(_)));
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Attachments(Box::new(Reply::Imported(
                task,
                preview_folder,
                workspace
                    .add_attachments(task, revision, inputs)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    pub(super) fn choose_attachments(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        if self.attachment_send_blocked() || self.close != CloseState::Open || self.hubs.pending(cx)
        {
            return;
        }
        let revision = self
            .attachments
            .value
            .as_ref()
            .expect("loaded attachments")
            .revision;
        self.attachments.picking = true;
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: true,
            multiple: true,
            prompt: Some(
                "Attach PDF/DOCX documents, images or UTF-8 files, or choose folders for names-only snapshots".into(),
            ),
        });
        cx.spawn(async move |view,cx| {
            let result=picker.await;
            let _=view.update(cx,|this,cx| {
                this.attachments.picking=false;
                match result {
                    Ok(Ok(Some(paths)))=>this.import_attachments(task,revision,paths.into_iter().map(|path|if path.is_dir(){AttachmentInput::Folder(path)}else{AttachmentInput::File(path)}).collect(),cx),
                    Ok(Ok(None))=>{},
                    _=>{this.attachments.errors.insert(task,"The native file picker could not open. Drop files on the composer instead.".into());},
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    fn change_attachments(&mut self, edit: AttachmentEdit, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        if self.attachment_send_blocked()
            || self.close != CloseState::Open
            || self.attachments.submitted.contains_key(&task)
        {
            return;
        }
        let revision = self
            .attachments
            .value
            .as_ref()
            .expect("loaded attachments")
            .revision;
        self.attachments.begin(task);
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Attachments(Box::new(Reply::Changed(
                task,
                workspace
                    .edit_attachments(task, revision, edit)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        cx.notify();
    }
    fn preview_attachment(&mut self, id: String, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        self.attachments.preview_id = Some(id.clone());
        self.attachments.preview = None;
        self.attachments.preview_loading = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = workspace
                .attachment_preview(task, id.clone())
                .await
                .map_err(|e| e.to_string());
            Ok(Update::Attachments(Box::new(Reply::Preview(
                task, id, result,
            ))))
        });
        cx.notify();
    }
    pub(super) fn attachment_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Preview(task, id, result) => {
                if self.attachments.task != Some(task)
                    || self.attachments.preview_id.as_ref() != Some(&id)
                {
                    return;
                }
                self.attachments.preview_loading = false;
                match result {
                    Ok(asset) => {
                        self.attachments.preview = Some(match asset.info.kind {
                            AttachmentKind::Text
                            | AttachmentKind::Pdf
                            | AttachmentKind::Docx
                            | AttachmentKind::Odt
                            | AttachmentKind::Odp
                            | AttachmentKind::Ods
                            | AttachmentKind::Pptx
                            | AttachmentKind::Xlsx => {
                                Preview::Text(String::from_utf8(asset.bytes).unwrap_or_default())
                            }
                            kind => Preview::Image(
                                Arc::new(gpui::Image::from_bytes(
                                    if kind == AttachmentKind::Png {
                                        gpui::ImageFormat::Png
                                    } else if kind == AttachmentKind::Webp {
                                        gpui::ImageFormat::Webp
                                    } else {
                                        gpui::ImageFormat::Jpeg
                                    },
                                    asset.bytes,
                                )),
                                asset.info.dimensions.unwrap_or_default(),
                            ),
                        })
                    }
                    Err(error) => {
                        self.attachments.errors.insert(task, error);
                    }
                }
            }
            reply => {
                let mut preview_folder = false;
                let (task, result) = match reply {
                    Reply::Loaded(task, result) => {
                        if self.attachments.task == Some(task) {
                            self.attachments.loading = false;
                        }
                        (task, result)
                    }
                    Reply::Imported(task, contains_folder, result) => {
                        preview_folder = contains_folder && self.attachments.task == Some(task);
                        self.attachments.end(task);
                        if result.is_ok() {
                            self.attachments.imports.remove(&task);
                        }
                        (task, result)
                    }
                    Reply::Changed(task, result) => {
                        self.attachments.end(task);
                        (task, result)
                    }
                    Reply::Preview(..) => unreachable!(),
                };
                let mut auto_preview = None;
                match result {
                    Ok(value) => {
                        if !self.attachments.imports.contains_key(&task) {
                            self.attachments.errors.remove(&task);
                        }
                        if self.attachments.task == Some(task)
                            && self
                                .attachments
                                .value
                                .as_ref()
                                .is_none_or(|v| v.revision <= value.revision)
                        {
                            if self.attachments.preview_id.as_ref().is_some_and(|id| {
                                !value
                                    .pending
                                    .iter()
                                    .chain(&value.recent)
                                    .any(|a| &a.id == id)
                            }) {
                                self.attachments.preview_id = None;
                                self.attachments.preview = None;
                            }
                            if preview_folder {
                                // Newly added snapshots are appended after existing
                                // ones. Open the newest so its exact names-only
                                // payload is visible before the user sends it.
                                auto_preview = value
                                    .pending
                                    .iter()
                                    .rev()
                                    .find(|item| item.is_folder_snapshot())
                                    .map(|item| item.id.clone());
                            }
                            self.attachments.value = Some(value);
                        }
                    }
                    Err(error) => {
                        self.attachments.errors.insert(task, error);
                    }
                }
                if let Some(id) = auto_preview {
                    self.preview_attachment(id, cx);
                }
            }
        }
        cx.notify();
    }
}
