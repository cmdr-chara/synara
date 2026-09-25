use super::*;
fn studio_version_export_name(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("studio-version");
    let path = Path::new(name);
    match (
        path.file_stem().and_then(|stem| stem.to_str()),
        path.extension().and_then(|extension| extension.to_str()),
    ) {
        (Some(stem), Some(extension)) if !stem.is_empty() && !extension.is_empty() => {
            format!("{stem}-version.{extension}")
        }
        _ => format!("{name}-version.txt"),
    }
}

impl Shell {
    fn review_studio_file_export(&mut self, cx: &mut Context<Self>) {
        if self.studio.exporting
            || self.close != CloseState::Open
            || self.studio.task != self.selected
        {
            return;
        }
        let (Some(task), Some(path)) = (self.selected, self.studio.selected.clone()) else {
            return;
        };
        self.studio.exporting = true;
        self.studio.export_review = None;
        let generation = self.studio.preview_generation;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            let result = workspace
                .review_studio_export(task, path)
                .await
                .map_err(|e| e.to_string());
            Ok(Update::Studio(Box::new(StudioReply::ExportReviewed {
                task,
                generation,
                result,
            })))
        });
        cx.notify();
    }
    pub(super) fn studio_export_reviewed(
        &mut self,
        task: TaskId,
        generation: u64,
        result: Result<StudioExportReview, String>,
    ) {
        self.studio.exporting = false;
        if self.selected != Some(task)
            || self.studio.task != Some(task)
            || self.studio.preview_generation != generation
        {
            return;
        }
        match result {
            Ok(review) => self.studio.export_review = Some(review),
            Err(error) => self.studio.error = Some(error),
        }
    }
    fn save_studio_file(&mut self, cx: &mut Context<Self>) {
        if self.studio.exporting
            || self.close != CloseState::Open
            || self.studio.task != self.selected
        {
            return;
        }
        let Some(review) = self.studio.export_review.clone() else {
            return;
        };
        let task = self.selected;
        let generation = self.studio.preview_generation;
        let name = review
            .path()
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.studio.exporting = true;
        let picker = cx.prompt_for_new_path(&self.scratch_directory, Some(&name));
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                if this.close != CloseState::Open
                    || this.selected != task
                    || this.studio.preview_generation != generation
                {
                    this.studio.exporting = false;
                    this.studio.export_review = None;
                    this.notice =
                        Some("Library export cancelled because its selection changed.".into());
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(Some(destination))) => {
                        this.studio.export_review = None;
                        let workspace = this.controller.workspace.clone();
                        // After explicit destination confirmation this write owns its outcome,
                        // independently of further preview navigation. Close waits for it.
                        this.job(async move {
                            let result = workspace
                                .export_studio_file(review, destination, &Default::default())
                                .await
                                .map_err(|e| e.to_string());
                            Ok(Update::Studio(Box::new(StudioReply::Exported(result))))
                        });
                    }
                    Ok(Ok(None)) => this.studio.exporting = false,
                    _ => {
                        this.studio.exporting = false;
                        this.studio.error = Some(
                            "The system save dialog is unavailable. No file was exported.".into(),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn save_selected_studio_version(&mut self, cx: &mut Context<Self>) {
        if self.studio.exporting
            || self.close != CloseState::Open
            || self.studio.task != self.selected
        {
            return;
        }
        let (Some(task), Some(path), Some(index)) = (
            self.selected,
            self.studio.selected.clone(),
            self.studio.selected_snapshot,
        ) else {
            return;
        };
        let Some(snapshot) = self.studio.snapshots.get(index).cloned() else {
            return;
        };
        if snapshot.task != task || snapshot.path != path {
            return;
        }

        self.studio.exporting = true;
        self.studio.export_review = None;
        let generation = self.studio.preview_generation;
        let name = studio_version_export_name(&path);
        let picker = cx.prompt_for_new_path(&self.scratch_directory, Some(&name));
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                let selection_matches = this
                    .studio
                    .selected_snapshot
                    .and_then(|selected| this.studio.snapshots.get(selected))
                    .is_some_and(|current| current == &snapshot);
                if this.close != CloseState::Open
                    || this.selected != Some(task)
                    || this.studio.task != Some(task)
                    || this.studio.preview_generation != generation
                    || this.studio.selected.as_ref() != Some(&path)
                    || !selection_matches
                {
                    this.studio.exporting = false;
                    this.notice = Some(
                        "Studio version export cancelled because its selection changed.".into(),
                    );
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(Some(destination))) => {
                        let workspace = this.controller.workspace.clone();
                        this.job(async move {
                            Ok(Update::Studio(Box::new(StudioReply::VersionExported(
                                workspace
                                    .export_studio_text_version(snapshot, destination)
                                    .await
                                    .map_err(|error| error.to_string()),
                            ))))
                        });
                    }
                    Ok(Ok(None)) => this.studio.exporting = false,
                    _ => {
                        this.studio.exporting = false;
                        this.studio.error = Some(
                            "The system save dialog is unavailable. No Studio version was exported."
                                .into(),
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn studio_export_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut row = div().flex().flex_col().gap_1();
        if self.studio.selected.is_some() {
            row = row.child(
                ui::button(
                    "studio-export-review",
                    if self.studio.exporting {
                        "Preparing/saving file..."
                    } else {
                        "Save current file as..."
                    },
                    false,
                )
                .relative()
                .child(ui::layout_probe("studio-export-review"))
                .on_click(cx.listener(|this, _, _, cx| this.review_studio_file_export(cx))),
            );
        }
        if let Some(review) = &self.studio.export_review {
            row = row.child(div().text_size(px(11.)).whitespace_normal().child(format!("Export {} ({} bytes). This copies the current original file, not its preview. Existing destinations are never replaced.", review.path().display(), review.bytes())))
                .child(div().flex().gap_1()
                    .child(ui::button("studio-export-confirm", "Choose new destination", false).relative().child(ui::layout_probe("studio-export-confirm"))
                        .on_click(cx.listener(|this, _, _, cx| this.save_studio_file(cx))))
                    .child(ui::button("studio-export-cancel", "Cancel export", false).on_click(cx.listener(|this, _, _, cx| {
                        if !this.studio.exporting { this.studio.export_review=None; cx.notify(); }
                    }))));
        }
        row.into_any_element()
    }
}
