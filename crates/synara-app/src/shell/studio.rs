//! Studio output browser. All file data comes from the bounded workspace service.
use super::*;
use crate::ui::{self, Glyph, palette};
use std::path::Path;

pub(super) enum StudioReply {
    Listed {
        task: TaskId,
        generation: u64,
        result: Result<StudioFiles, String>,
    },
    Previewed {
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<StudioPreview, String>,
    },
}
enum Preview {
    Text {
        text: String,
        markdown: bool,
    },
    Image {
        image: Arc<gpui::Image>,
        width: u32,
        height: u32,
    },
    Unsupported(u64),
}
pub(super) struct StudioState {
    pub open: bool,
    task: Option<TaskId>,
    query: Entity<TextEntry>,
    listing: StudioFiles,
    generation: u64,
    preview_generation: u64,
    loading: bool,
    preview_loading: bool,
    selected: Option<PathBuf>,
    reopen_after_navigation: Option<(TaskId, PathBuf)>,
    preview: Option<Preview>,
    error: Option<String>,
    only_outputs: bool,
    only_images: bool,
    raw_text: bool,
    image_zoom: Option<f32>,
    _subscription: Subscription,
}
impl StudioState {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        let query = cx.new(|cx| {
            TextEntry::new("Find Library files...", EntryMode::SingleLine, 32., cx)
                .with_leading_icon(Glyph::Search)
        });
        let subscription = cx.subscribe(&query, |_, _, _, cx| cx.notify());
        Self {
            open: false,
            task: None,
            query,
            listing: StudioFiles::default(),
            generation: 0,
            preview_generation: 0,
            loading: false,
            preview_loading: false,
            selected: None,
            reopen_after_navigation: None,
            preview: None,
            error: None,
            only_outputs: false,
            only_images: false,
            raw_text: false,
            image_zoom: None,
            _subscription: subscription,
        }
    }
    pub fn reset(&mut self) {
        self.open = false;
        self.task = None;
        self.generation = self.generation.wrapping_add(1);
        self.preview_generation = self.preview_generation.wrapping_add(1);
        self.preview = None;
        self.selected = None;
        self.reopen_after_navigation = None;
        self.listing = StudioFiles::default();
        self.loading = false;
        self.preview_loading = false;
        self.error = None;
        self.raw_text = false;
        self.image_zoom = None;
    }
}
fn image_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|x| x.to_str())
        .is_some_and(|x| matches!(x.to_ascii_lowercase().as_str(), "png" | "jpg" | "jpeg"))
}
fn take_reopen_path(task: TaskId, pending: Option<(TaskId, PathBuf)>) -> Option<PathBuf> {
    pending.and_then(|(expected_task, path)| (expected_task == task).then_some(path))
}
fn restored_output_is_attributed(task: TaskId, path: &Path, listing: &[StudioFile]) -> bool {
    listing
        .iter()
        .any(|entry| entry.path == path && entry.source_task == Some(task))
}
impl Shell {
    pub(super) fn open_studio_outputs(&mut self, cx: &mut Context<Self>) {
        if self
            .task()
            .is_none_or(|task| task.scope != TaskScope::Studio)
        {
            return;
        }
        self.set_panel(Panel::Files, cx);
        self.studio.open = true;
        self.refresh_studio_outputs(cx);
    }
    fn refresh_studio_outputs(&mut self, cx: &mut Context<Self>) {
        let Some(task) = self
            .task()
            .filter(|task| task.scope == TaskScope::Studio)
            .map(|task| task.id)
        else {
            return;
        };
        self.studio.generation = self.studio.generation.wrapping_add(1);
        let generation = self.studio.generation;
        self.studio.task = Some(task);
        self.studio.loading = true;
        self.studio.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Studio(Box::new(StudioReply::Listed {
                task,
                generation,
                result: workspace
                    .studio_files(task)
                    .await
                    .map_err(|error| error.to_string()),
            })))
        });
        cx.notify();
    }
    fn preview_studio_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(task) = self.selected.filter(|task| Some(*task) == self.studio.task) else {
            return;
        };
        self.studio.preview_generation = self.studio.preview_generation.wrapping_add(1);
        let generation = self.studio.preview_generation;
        self.studio.selected = Some(path.clone());
        self.studio.image_zoom = None;
        self.studio.preview = None;
        self.studio.preview_loading = true;
        self.studio.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Studio(Box::new(StudioReply::Previewed {
                task,
                generation,
                result: workspace
                    .studio_preview(task, path.clone())
                    .await
                    .map_err(|error| error.to_string()),
                path,
            })))
        });
        cx.notify();
    }
    fn open_studio_output_with_source(
        &mut self,
        source: TaskId,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let Some(current) = self.task() else { return };
        let Some(source_task) = self.catalog.tasks.iter().find(|task| task.id == source) else {
            self.studio.error = Some("This output's source chat is no longer available.".into());
            cx.notify();
            return;
        };
        let output_is_current = self.studio.open
            && self.studio.task == Some(current.id)
            && current.scope == TaskScope::Studio
            && source_task.scope == TaskScope::Studio
            && current.project_id == source_task.project_id
            && current.working_directory == source_task.working_directory
            && self
                .studio
                .listing
                .entries
                .iter()
                .any(|entry| entry.path == path && entry.source_task == Some(source));
        if !output_is_current {
            self.studio.error = Some(
                "This output's source link is stale. Refresh the Library and choose the output again."
                    .into(),
            );
            cx.notify();
            return;
        }
        if self.selected != Some(source) {
            if !self.select_task(source, cx) {
                return;
            }
            self.studio.reopen_after_navigation = Some((source, path));
            self.open_studio_outputs(cx);
        }
        // When already in the reporting thread, the selected output is already
        // open in the Library alongside this conversation.
    }
    pub(super) fn studio_reply(&mut self, reply: StudioReply, cx: &mut Context<Self>) {
        match reply {
            StudioReply::Listed {
                task,
                generation,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.generation != generation
                {
                    return;
                }
                self.studio.loading = false;
                match result {
                    Ok(listing) => {
                        self.studio.listing = listing;
                        let restored =
                            take_reopen_path(task, self.studio.reopen_after_navigation.take());
                        if let Some(path) =
                            restored.clone().or_else(|| self.studio.selected.clone())
                        {
                            let output_exists = if restored.is_some() {
                                restored_output_is_attributed(
                                    task,
                                    &path,
                                    &self.studio.listing.entries,
                                )
                            } else {
                                self.studio
                                    .listing
                                    .entries
                                    .iter()
                                    .any(|entry| entry.path == path)
                            };
                            if output_exists {
                                self.preview_studio_file(path, cx);
                            } else {
                                self.studio.preview = None;
                                self.studio.selected = None;
                                if restored.is_some() {
                                    self.studio.error = Some(
                                        "This output is no longer attributed to its reporting Hub."
                                            .into(),
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => self.studio.error = Some(error),
                }
            }
            StudioReply::Previewed {
                task,
                generation,
                path,
                result,
            } => {
                if self.selected != Some(task)
                    || self.studio.task != Some(task)
                    || self.studio.preview_generation != generation
                    || self.studio.selected.as_ref() != Some(&path)
                {
                    return;
                }
                self.studio.preview_loading = false;
                match result {
                    Ok(StudioPreview::Text { text, markdown }) => {
                        self.studio.preview = Some(Preview::Text { text, markdown })
                    }
                    Ok(StudioPreview::Image {
                        bytes,
                        format,
                        width,
                        height,
                    }) => {
                        let format = match format {
                            PreviewImageFormat::Png => gpui::ImageFormat::Png,
                            PreviewImageFormat::Jpeg => gpui::ImageFormat::Jpeg,
                        };
                        self.studio.preview = Some(Preview::Image {
                            image: Arc::new(gpui::Image::from_bytes(format, bytes)),
                            width,
                            height,
                        });
                    }
                    Ok(StudioPreview::Unsupported { bytes }) => {
                        self.studio.preview = Some(Preview::Unsupported(bytes))
                    }
                    Err(error) => self.studio.error = Some(error),
                }
            }
        }
        cx.notify();
    }
    pub(super) fn studio_outputs_button(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        ui::chrome_button(
            "studio-outputs",
            "Hub Library and workspace files",
            Glyph::Files,
            false,
            cx.listener(|this, _: &(), _, cx| this.open_studio_outputs(cx)),
        )
        .size(px(26.))
        .into_any_element()
    }
    fn studio_reference_to_draft(&mut self, cx: &mut Context<Self>) {
        if !self.studio.open
            || self.studio.task != self.selected
            || self.close != CloseState::Open
            || self.loading_task.is_some()
        {
            return;
        }
        let Some(path) = self.studio.selected.as_ref() else {
            return;
        };
        let reference = format!(
            "Hub Library file: {}",
            serde_json::to_string(&path.to_string_lossy()).unwrap_or_default()
        );
        let text = self.composer.read(cx).text();
        let separator = if text.is_empty() { "" } else { "\n\n" };
        if text
            .len()
            .saturating_add(reference.len())
            .saturating_add(separator.len())
            > 1024 * 1024
        {
            self.studio.error = Some("The draft is too large. Nothing was inserted.".into());
        } else {
            let text = format!("{text}{separator}{reference}");
            self.composer
                .update(cx, |entry, cx| entry.set_text(text, cx));
            self.remember_draft(cx);
            self.notice = Some("File path added to the draft without sending. This is a path reference, not an image attachment.".into());
        }
        cx.notify();
    }
    pub(super) fn studio_files_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let query = self.studio.query.read(cx).text().trim().to_lowercase();
        let matches = self
            .studio
            .listing
            .entries
            .iter()
            .filter(|file| {
                (!self.studio.only_outputs || file.reported_output)
                    && (!self.studio.only_images || image_path(&file.path))
                    && (query.is_empty()
                        || file.path.to_string_lossy().to_lowercase().contains(&query))
            })
            .collect::<Vec<_>>();
        let selected = self.studio.selected.clone();
        let source_task = selected
            .as_ref()
            .and_then(|path| {
                self.studio
                    .listing
                    .entries
                    .iter()
                    .find(|entry| &entry.path == path)
            })
            .and_then(|entry| entry.source_task);
        let preview = match &self.studio.preview {
            Some(Preview::Text { text, markdown }) => {
                div().id("studio-text-preview").relative().child(ui::layout_probe("studio-text-preview"))
                    .flex_1().min_h_0().overflow_y_scroll().p_3().text_size(px(13.))
                    .when(text.len() > 128*1024, |el| el.child(
                        div().text_size(px(11.)).text_color(rgb(palette().muted)).pb_2()
                            .child("Preview limited to 128 KiB. Copy text includes the full loaded file.")))
                    .child(if *markdown && !self.studio.raw_text { ui::markdown::render(&truncate(text, 128*1024), "studio-file-preview") }
                        else { div().relative().child(ui::layout_probe("studio-raw-preview"))
                            .font_family(ui::code_font()).child(truncate(text,128*1024)).into_any_element() })
                    .into_any_element()
            }
            Some(Preview::Image { image, width, height }) => div().id("studio-image-preview").relative().child(ui::layout_probe("studio-image-preview"))
                .flex_1().min_h_0().min_w_0().flex().flex_col().gap_2()
                .child(div().flex().items_center().flex_wrap().gap_1()
                    .child(ui::button("studio-image-fit", "Fit", self.studio.image_zoom.is_none()).text_size(px(11.))
                        .relative().child(ui::layout_probe("studio-image-fit"))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = None; cx.notify(); })))
                    .child(ui::button("studio-image-actual", "100%", self.studio.image_zoom == Some(1.)).text_size(px(11.))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = Some(1.); cx.notify(); })))
                    .child(ui::button("studio-image-out", "-", false).aria_label("Zoom image out").text_size(px(11.))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = Some((this.studio.image_zoom.unwrap_or(1.) / 1.25).clamp(0.125, 4.)); cx.notify(); })))
                    .child(ui::button("studio-image-in", "+", false).aria_label("Zoom image in").text_size(px(11.))
                        .relative().child(ui::layout_probe("studio-image-in"))
                        .on_click(cx.listener(|this, _, _, cx| { this.studio.image_zoom = Some((this.studio.image_zoom.unwrap_or(1.) * 1.25).clamp(0.125, 4.)); cx.notify(); })))
                    .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{width} × {height}{}", self.studio.image_zoom.map_or(String::new(), |zoom| format!(" · {:.0}%", zoom * 100.))))))
                .child(div().id("studio-image-viewport").flex_1().min_h_0().min_w_0().overflow_scroll().p_2()
                    .child(if let Some(zoom) = self.studio.image_zoom {
                        div().relative().child(ui::layout_probe("studio-zoomed-image"))
                            .w(px(*width as f32 * zoom)).h(px(*height as f32 * zoom)).flex_shrink_0()
                            .child(gpui::img(image.clone()).size_full().object_fit(gpui::ObjectFit::Contain)).into_any_element()
                    } else { gpui::img(image.clone()).w_full().h(px(240.)).object_fit(gpui::ObjectFit::Contain).into_any_element() }))
                .into_any_element(),
            Some(Preview::Unsupported(bytes)) => div().flex_1().min_h_0().p_4().text_size(px(12.)).text_color(rgb(palette().muted))
                .child(format!("Preview unavailable for this file ({bytes} bytes). PNG/JPEG and UTF-8 text are supported within the preview limits. The file has not been executed or opened externally.")).into_any_element(),
            None => div().flex_1().min_h_0().p_4().text_size(px(13.)).text_color(rgb(palette().muted))
                .child(if self.studio.preview_loading { "Loading preview..." } else { "Select a file to preview it." }).into_any_element(),
        };
        div().id("studio-files-panel").relative().child(ui::layout_probe("studio-files-panel"))
            .flex().flex_col().flex_1().min_h_0().min_w_0().gap_2().p_3()
            .child(div().flex().items_center().gap_2().child(ui::icon(Glyph::Files))
                .child(div().flex_1().text_size(px(14.)).child("Hub Library"))
                .child(ui::button("studio-refresh", if self.studio.loading { "Refreshing..." } else { "Refresh" }, false)
                    .relative().child(ui::layout_probe("studio-refresh"))
                    .on_click(cx.listener(|this,_,_,cx| { if !this.studio.loading { this.refresh_studio_outputs(cx); } }))))
            .child(div().relative().child(ui::layout_probe("studio-search")).child(self.studio.query.clone()))
            .child(div().flex().items_center().gap_1().flex_wrap()
                .child(ui::button("studio-all-files","All files",!self.studio.only_outputs).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.only_outputs=false;cx.notify(); })))
                .child(ui::button("studio-reported","Reported outputs",self.studio.only_outputs).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.only_outputs=true;cx.notify(); })))
                .child(ui::button("studio-images","Images",self.studio.only_images).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| { this.studio.only_images = !this.studio.only_images;cx.notify(); }))))
            .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child(format!("{} files · Reported by completed tools in this Hub, not proof of authorship.",matches.len())))
        .children(source_task.zip(selected.clone()).map(|(source, path)| {
            ui::action(
                "library-source-thread",
                "Open reporting chat with output",
                Some(Glyph::Chat),
                false,
                cx.listener(move |this, _: &(), _, cx| {
                    this.open_studio_output_with_source(source, path.clone(), cx)
                }),
            )
        }))
            .children(self.studio.error.clone().map(|error| div().text_size(px(12.)).text_color(rgb(palette().error)).child(error)))
            .when(self.studio.listing.limited || self.studio.listing.unreadable>0, |el| el.child(div().text_size(px(11.)).text_color(rgb(palette().muted))
                .child(format!("Bounded listing{} · {} unreadable directories/files",if self.studio.listing.limited { " reached its limit" } else { "" },self.studio.listing.unreadable))))
            .child(div().id("studio-file-list").h(px(190.)).flex_shrink_0().overflow_y_scroll().flex().flex_col()
                .children(matches.iter().enumerate().map(|(index,file)| {
                    let path=file.path.clone(); let active=selected.as_ref()==Some(&path);
                    ui::action(("studio-file",index),file.path.to_string_lossy().into_owned(),Some(if image_path(&path) { Glyph::Capture } else { Glyph::Files }),active,
                        cx.listener(move |this, _: &(),_,cx| this.preview_studio_file(path.clone(),cx)))
                        .h(px(28.)).text_size(px(12.)).relative().child(ui::layout_probe_slot("studio-file",index))
                }))
                .when(matches.is_empty(), |el| el.child(div().p_3().text_size(px(12.)).text_color(rgb(palette().muted))
                    .child(if self.studio.loading { "Looking for files..." } else if self.studio.only_outputs { "No completed tool changes reference a visible file yet. All files shows other workspace content." } else { "No files match. Files appear here after they are created in this Hub working folder." }))))
            .child(div().text_size(px(12.)).text_ellipsis().child(selected.as_ref().map(|p|p.to_string_lossy().into_owned()).unwrap_or_default()))
            .when(matches!(self.studio.preview, Some(Preview::Text { markdown: true, .. })), |el| el.child(
                ui::button("studio-raw-toggle", if self.studio.raw_text { "Show rendered Markdown" } else { "Show raw text" }, self.studio.raw_text)
                    .text_size(px(11.)).aria_label("Toggle raw Markdown source")
                    .relative().child(ui::layout_probe("studio-raw-toggle"))
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.studio.raw_text = !this.studio.raw_text;
                        cx.notify();
                    }))))
            .child(preview)
            .child(div().flex().items_center().gap_1().flex_wrap().border_t_1().border_color(rgb(palette().border)).pt_2()
                .child(ui::button("studio-copy-path","Copy path",false).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                    if let Some(path)=&this.studio.selected { cx.write_to_clipboard(gpui::ClipboardItem::new_string(path.to_string_lossy().into_owned())); }
                })))
                .child(ui::button("studio-reference","Add path to draft",false).text_size(px(11.)).relative().child(ui::layout_probe("studio-reference"))
                    .on_click(cx.listener(|this,_,_,cx| this.studio_reference_to_draft(cx))))
                .when(matches!(self.studio.preview,Some(Preview::Text {..})), |el| el
                    .child(ui::button("studio-copy-text","Copy text",false).text_size(px(11.)).on_click(cx.listener(|this,_,_,cx| {
                        if let Some(Preview::Text {text,..})=&this.studio.preview {cx.write_to_clipboard(gpui::ClipboardItem::new_string(text.clone()));}
                    })))
                    .child(ui::button("studio-edit","Open in editor",false).text_size(px(11.)).relative().child(ui::layout_probe("studio-edit"))
                        .on_click(cx.listener(|this,_,_,cx| {
                            if let Some(path)=this.studio.selected.clone() { this.set_panel(Panel::Files,cx); this.open_file(path,cx); }
                        })))))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{restored_output_is_attributed, take_reopen_path};
    use std::path::PathBuf;
    use synara_core::TaskId;
    use synara_workspace::StudioFile;

    #[test]
    fn reopening_output_is_scoped_to_the_requested_source_task() {
        let task = TaskId::new();
        let other = TaskId::new();
        let path = PathBuf::from("reports/result.md");

        assert_eq!(
            take_reopen_path(task, Some((task, path.clone()))),
            Some(path)
        );
        assert_eq!(
            take_reopen_path(task, Some((other, PathBuf::from("reports/other.md")))),
            None
        );
    }

    #[test]
    fn restored_output_must_still_be_attributed_to_the_reporting_task() {
        let task = TaskId::new();
        let other = TaskId::new();
        let path = PathBuf::from("reports/result.md");
        let entry = |source_task| StudioFile {
            path: path.clone(),
            bytes: 32,
            reported_output: true,
            source_task,
        };

        assert!(restored_output_is_attributed(
            task,
            &path,
            &[entry(Some(task))]
        ));
        assert!(!restored_output_is_attributed(
            task,
            &path,
            &[entry(Some(other))]
        ));
        assert!(!restored_output_is_attributed(task, &path, &[entry(None)]));
    }
}
