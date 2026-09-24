//! Single snapshot, single rendered page, task/generation-fenced native PDF UI.
use super::*;

pub(super) struct PdfView {
    document: StudioPdf,
    number: u32,
    image: Arc<gpui::Image>,
    width: u32,
    height: u32,
    page_text: Option<String>,
    text_loading: bool,
    text_error: Option<String>,
}
impl PdfView {
    fn new(document: StudioPdf, page: StudioPdfPage) -> Self {
        Self {
            document,
            number: page.number,
            width: page.width,
            height: page.height,
            image: Arc::new(gpui::Image::from_bytes(gpui::ImageFormat::Png, page.png)),
            page_text: None,
            text_loading: false,
            text_error: None,
        }
    }
}
impl Shell {
    pub(super) fn start_studio_pdf(
        &mut self,
        task: TaskId,
        path: PathBuf,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let workspace = self.controller.workspace.clone();
        let cancel = self.studio.preview_cancel.clone();
        self.job(async move {
            let result = async {
                let document = workspace.studio_pdf(task, path.clone(), &cancel).await?;
                let page = document.page(1, &cancel).await?;
                Ok::<_, WorkspaceError>((document, page))
            }
            .await
            .map_err(|e| e.to_string());
            Ok(Update::Studio(Box::new(StudioReply::PdfLoaded {
                task,
                generation,
                path,
                result,
            })))
        });
        cx.notify();
    }
    fn turn_studio_pdf(&mut self, next: bool, cx: &mut Context<Self>) {
        if self.studio.preview_loading || self.close != CloseState::Open {
            return;
        }
        let Some(task) = self.selected.filter(|t| Some(*t) == self.studio.task) else {
            return;
        };
        let Some(path) = self.studio.selected.clone() else {
            return;
        };
        let Some(Preview::Pdf(view)) = &mut self.studio.preview else {
            return;
        };
        let number = if next {
            view.number.saturating_add(1)
        } else {
            view.number.saturating_sub(1)
        };
        if number == 0 || number > view.document.pages {
            return;
        }
        let document = view.document.clone();
        view.text_loading = false;
        self.studio.preview_generation = self.studio.preview_generation.wrapping_add(1);
        let generation = self.studio.preview_generation;
        self.studio.preview_cancel.cancel();
        self.studio.preview_cancel = Default::default();
        let cancel = self.studio.preview_cancel.clone();
        self.studio.preview_loading = true;
        self.studio.error = None;
        self.job(async move {
            let result = document
                .page(number, &cancel)
                .await
                .map(|page| (document, page))
                .map_err(|e| e.to_string());
            Ok(Update::Studio(Box::new(StudioReply::PdfLoaded {
                task,
                generation,
                path,
                result,
            })))
        });
        cx.notify();
    }
    fn extract_studio_pdf_text(&mut self, cx: &mut Context<Self>) {
        if self.studio.preview_loading || self.close != CloseState::Open {
            return;
        }
        let Some(task) = self.selected.filter(|task| Some(*task) == self.studio.task) else {
            return;
        };
        let Some(path) = self.studio.selected.clone() else {
            return;
        };
        if self.studio.preview_cancel.is_cancelled() {
            self.studio.preview_cancel = Default::default();
        }
        let Some(Preview::Pdf(view)) = &mut self.studio.preview else {
            return;
        };
        if view.text_loading || view.page_text.is_some() {
            return;
        }
        view.text_loading = true;
        view.text_error = None;
        let document = view.document.clone();
        let number = view.number;
        let generation = self.studio.preview_generation;
        let cancel = self.studio.preview_cancel.clone();
        let workspace_task = task;
        let expected_path = path.clone();
        cx.spawn(async move |weak, cx| {
            let result = document
                .page_text(number, &cancel)
                .await
                .map_err(|error| error.to_string());
            let _ = weak.update(cx, |this, cx| {
                if this.selected != Some(workspace_task)
                    || this.studio.task != Some(workspace_task)
                    || this.studio.preview_generation != generation
                    || this.studio.selected.as_ref() != Some(&expected_path)
                {
                    return;
                }
                let Some(Preview::Pdf(view)) = &mut this.studio.preview else {
                    return;
                };
                if view.number != number {
                    return;
                }
                view.text_loading = false;
                match result {
                    Ok(text) if text.trim().is_empty() => {
                        view.text_error = Some(
                            "No text was found on this page. Scanned pages are not OCR processed."
                                .into(),
                        );
                    }
                    Ok(text) => view.page_text = Some(text),
                    Err(error) => view.text_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
    pub(super) fn studio_pdf_loaded(
        &mut self,
        task: TaskId,
        generation: u64,
        path: PathBuf,
        result: Result<(StudioPdf, StudioPdfPage), String>,
    ) {
        if self.selected != Some(task)
            || self.studio.task != Some(task)
            || self.studio.preview_generation != generation
            || self.studio.selected.as_ref() != Some(&path)
            || self.studio.preview_cancel.is_cancelled()
        {
            return;
        }
        self.studio.preview_loading = false;
        match result {
            Ok((doc, page)) => self.studio.preview = Some(Preview::Pdf(PdfView::new(doc, page))),
            Err(error) => self.studio.error = Some(error),
        }
    }
    pub(super) fn studio_pdf_panel(
        &self,
        view: &PdfView,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div().id("studio-pdf-preview").relative().child(ui::layout_probe("studio-pdf-preview")).when(!self.studio.preview_loading, |el| el.child(ui::layout_probe_slot("studio-pdf-page", view.number as usize)))
            .flex().flex_col().flex_1().min_h_0().gap_2()
            .child(div().flex().items_center().gap_1().flex_wrap()
                .child(ui::button("studio-pdf-previous", "Previous page", false)
                    .relative().child(ui::layout_probe("studio-pdf-previous"))
                    .on_click(cx.listener(|this, _, _, cx| this.turn_studio_pdf(false, cx))))
                .child(div().text_size(px(12.)).child(format!("Page {} of {}{}", view.number, view.document.pages, if self.studio.preview_loading { " · Loading..." } else { "" })))
                .child(ui::button("studio-pdf-next", "Next page", false)
                    .relative().child(ui::layout_probe("studio-pdf-next"))
                    .on_click(cx.listener(|this, _, _, cx| this.turn_studio_pdf(true, cx))))
                .child(ui::button("studio-pdf-fit", "Fit", self.studio.image_zoom.is_none()).on_click(cx.listener(|this, _, _, cx| {this.studio.image_zoom=None;cx.notify();})))
                .child(ui::button("studio-pdf-in", "+", false).relative().child(ui::layout_probe("studio-pdf-in"))
                    .on_click(cx.listener(|this, _, _, cx| {this.studio.image_zoom=Some((this.studio.image_zoom.unwrap_or(0.5)*1.25).min(2.));cx.notify();})))
                .child(ui::button("studio-pdf-out", "-", false).on_click(cx.listener(|this, _, _, cx| {this.studio.image_zoom=Some((this.studio.image_zoom.unwrap_or(0.5)/1.25).max(0.125));cx.notify();})))
                .child(ui::button("studio-pdf-reload", "Reload file", false).relative().child(ui::layout_probe("studio-pdf-reload"))
                    .on_click(cx.listener(|this, _, _, cx| {if let Some(path)=this.studio.selected.clone(){this.preview_studio_file(path,cx);}}))))
            .child(div().text_size(px(11.)).text_color(rgb(palette().muted))
                .child("Read-only PDF snapshot · Reload to see disk changes. Extracted text is inert. PDF links and form fields are not exposed, and scripts or embedded files are never run."))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        ui::button(
                            "studio-pdf-extract-text",
                            if view.text_loading {
                                "Extracting page text..."
                            } else {
                                "Extract page text"
                            },
                            view.page_text.is_some(),
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.extract_studio_pdf_text(cx)
                        })),
                    )
                    .when(view.page_text.is_some(), |el| {
                        el.child(
                            ui::button("studio-pdf-copy-text", "Copy page text", false)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    if let Some(Preview::Pdf(view)) = &this.studio.preview
                                        && let Some(text) = &view.page_text
                                    {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(
                                            text.clone(),
                                        ));
                                    }
                                })),
                        )
                    }),
            )
            .when_some(view.text_error.as_ref(), |el, error| el.child(div().text_size(px(11.)).text_color(rgb(palette().error)).child(error.clone())))
            .when_some(view.page_text.as_ref(), |el, text| el.child(div().id("studio-pdf-page-text").max_h(px(180.)).overflow_y_scroll().border_1().border_color(rgb(palette().border)).p_2()
                .child(div().text_size(px(12.)).child(text.clone()))))
            .child(div().id("studio-pdf-scroll").flex_1().min_h_0().overflow_y_scroll().overflow_x_scroll()
                .child(if let Some(zoom)=self.studio.image_zoom {
                    div().relative().child(ui::layout_probe("studio-pdf-zoomed"))
                        .w(px(view.width as f32*zoom)).h(px(view.height as f32*zoom)).flex_shrink_0()
                        .child(gpui::img(view.image.clone()).size_full().object_fit(gpui::ObjectFit::Contain)).into_any_element()
                } else {
                    gpui::img(view.image.clone()).w_full().h(px(300.)).object_fit(gpui::ObjectFit::Contain).into_any_element()
                }))
            .into_any_element()
    }
}
