//! Bounded task-scoped presentation cache. Originals belong to durable events.
use super::*;
use crate::ui::{self, Glyph, palette};
const CACHE: usize = 12;
pub(super) enum Reply {
    Preview(TaskId, EventId, bool, Result<AttachmentPreview, String>),
    Exported(Result<(), String>),
}
enum Preview {
    Loading(bool),
    Ready(Arc<gpui::Image>, bool, (u32, u32)),
    Failed(String),
}
#[derive(Default)]
pub(super) struct MediaState {
    task: Option<TaskId>,
    cache: HashMap<EventId, Preview>,
    order: Vec<EventId>,
    automatic: HashSet<EventId>,
    pub saving: bool,
}
impl Shell {
    pub(super) fn sync_transcript_media(&mut self, cx: &mut Context<Self>) {
        if self.media.task != self.selected {
            self.media.task = self.selected;
            self.media.cache.clear();
            self.media.order.clear();
            self.media.automatic.clear();
        }
        let candidates: Vec<_> = self
            .thread
            .as_ref()
            .map(|t| {
                t.images
                    .iter()
                    .rev()
                    .take(CACHE)
                    .filter(|m| !self.media.automatic.contains(&m.id))
                    .map(|m| m.id)
                    .collect()
            })
            .unwrap_or_default();
        for id in candidates {
            self.media.automatic.insert(id);
            self.load_transcript_image(id, false, cx);
        }
    }
    fn load_transcript_image(&mut self, id: EventId, expanded: bool, cx: &mut Context<Self>) {
        let Some(task) = self.selected else { return };
        if !self
            .thread
            .as_ref()
            .is_some_and(|t| t.images.iter().any(|m| m.id == id))
        {
            return;
        }
        if matches!(self.media.cache.get(&id), Some(Preview::Loading(_))) {
            return;
        }
        self.media.order.retain(|key| *key != id);
        self.media.order.push(id);
        while self.media.order.len() > CACHE {
            let retired = self.media.order.remove(0);
            self.media.cache.remove(&retired);
        }
        self.media.cache.insert(id, Preview::Loading(expanded));
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::RichMedia(Box::new(Reply::Preview(
                task,
                id,
                expanded,
                workspace
                    .transcript_image_preview(task, id, expanded)
                    .await
                    .map_err(|e| e.to_string()),
            ))))
        });
        self.transcript.invalidate_media();
        cx.notify();
    }
    pub(super) fn media_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Preview(task, id, expanded, result) => {
                if self.selected != Some(task)
                    || self.media.task != Some(task)
                    || !matches!(self.media.cache.get(&id),Some(Preview::Loading(requested)) if *requested==expanded)
                {
                    return;
                }
                let value = match result {
                    Ok(preview) => Preview::Ready(
                        Arc::new(gpui::Image::from_bytes(
                            gpui::ImageFormat::Png,
                            preview.bytes,
                        )),
                        expanded,
                        preview.info.dimensions.unwrap_or_default(),
                    ),
                    Err(error) => Preview::Failed(error),
                };
                self.media.cache.insert(id, value);
                self.transcript.invalidate_media();
            }
            Reply::Exported(result) => {
                self.media.saving = false;
                match result {
                Ok(())=>self.notice=Some("The original transcript image was saved to the explicitly selected new file.".into()),
                Err(error)=>self.error=Some(format!("Image export failed without replacing the destination: {error}")),
            }
            }
        }
        cx.notify();
    }
    fn save_transcript_image(&mut self, id: EventId, cx: &mut Context<Self>) {
        if self.media.saving || self.close != CloseState::Open {
            return;
        }
        let Some(task) = self.selected else { return };
        let Some(media) = self
            .thread
            .as_ref()
            .and_then(|t| t.images.iter().find(|m| m.id == id))
        else {
            return;
        };
        let name = format!(
            "synara-image-{id}.{}",
            if media.image.mime_type == "image/png" {
                "png"
            } else {
                "jpg"
            }
        );
        self.media.saving = true;
        let picker = cx.prompt_for_new_path(&self.scratch_directory, Some(&name));
        cx.spawn(async move |view,cx| {
            let chosen=picker.await;
            let _=view.update(cx,|this,cx| {
                match chosen {
                    Ok(Ok(Some(path)))=>{
                        let workspace=this.controller.workspace.clone();
                        this.job(async move {Ok(Update::RichMedia(Box::new(Reply::Exported(
                            workspace.export_transcript_image(task,id,path).await.map_err(|e|e.to_string())))))});
                    }
                    Ok(Ok(None))=>this.media.saving=false,
                    _=>{this.media.saving=false;this.error=Some("The system save dialog is unavailable. The original image remains in this transcript.".into());},
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    pub(super) fn message_media(
        &self,
        message: &Message,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut root = div().flex().flex_col().flex_shrink_0().gap_2().min_w_0();
        if let Some(thread) = &self.thread {
            for (slot, item) in thread
                .images
                .iter()
                .enumerate()
                .filter(|(_, m)| m.message_id == message.id && m.role == message.role)
            {
                let id = item.id;
                let expanded =
                    matches!(self.media.cache.get(&id), Some(Preview::Ready(_, true, _)));
                let mut row = div()
                    .relative()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .min_w_0()
                    .flex_shrink_0()
                    .child(ui::layout_probe_slot("transcript-media", slot))
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette().muted))
                            .child(item.image.source.label()),
                    );
                row = row.child(match self.media.cache.get(&id) {
                    Some(Preview::Ready(image, large, (w, h))) => div()
                        .flex().flex_col().flex_shrink_0().gap_1()
                        .relative()
                        .child(ui::layout_probe_slot("transcript-image-ready", slot))
                        .when(*large, |el| {
                            el.child(ui::layout_probe_slot("transcript-image-expanded", slot))
                        })
                        .w_full()
                        .max_w(px(if *large { 900. } else { 480. }))
                        .child(
                            div().relative().w_full().h(px(if *large {420.} else {160.}))
                                .flex_shrink_0().overflow_hidden()
                                .child(ui::layout_probe_slot("transcript-image-frame",slot))
                                .child(gpui::img(image.clone()).size_full().object_fit(gpui::ObjectFit::Contain)),
                        )
                        .child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette().muted))
                                .child(format!("Preview {w} x {h} | Save exports the original")),
                        )
                        .into_any_element(),
                    Some(Preview::Loading(_)) => div()
                        .child("Loading bounded image preview...")
                        .into_any_element(),
                    Some(Preview::Failed(error)) => div()
                        .relative()
                        .child(ui::layout_probe_slot("transcript-image-error", slot))
                        .text_color(rgb(palette().error))
                        .child(format!("Image unavailable: {error}"))
                        .into_any_element(),
                    None => div()
                        .child("Preview unloaded to bound memory. Load to view the saved image.")
                        .into_any_element(),
                });
                root = root.child(
                    row.child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_2()
                            .child(
                                div()
                                    .relative()
                                    .child(ui::layout_probe_slot("transcript-image-expand", slot))
                                    .child(ui::action(
                                        format!("expand-media-{id}"),
                                        if expanded {
                                            "Reduce image"
                                        } else {
                                            "Expand / load image"
                                        },
                                        Some(Glyph::Capture),
                                        false,
                                        cx.listener(move |this, _: &(), _, cx| {
                                            this.load_transcript_image(id, !expanded, cx)
                                        }),
                                    )),
                            )
                            .child(
                                div()
                                    .relative()
                                    .child(ui::layout_probe_slot("transcript-image-save", slot))
                                    .child(ui::action(
                                        format!("save-media-{id}"),
                                        "Save original...",
                                        Some(Glyph::Down),
                                        false,
                                        cx.listener(move |this, _: &(), _, cx| {
                                            this.save_transcript_image(id, cx)
                                        }),
                                    )),
                            ),
                    ),
                );
            }
        }
        root.into_any_element()
    }
}
