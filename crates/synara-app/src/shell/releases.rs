//! Read-only local build notes plus an explicit durable acknowledgement.
use super::*;
use crate::ui::{self, Glyph, palette};
const NOTES_NOTICE: &str = "New native build notes are available in Settings > What\'s New.";

pub(super) enum Reply {
    Loaded(Result<ReleaseJournal, String>),
}
#[derive(Default)]
pub(super) struct ReleaseState {
    journal: Option<ReleaseJournal>,
    busy: bool,
    error: Option<String>,
}
impl ReleaseState {
    pub fn busy(&self) -> bool { self.busy }
}
impl Shell {
    pub(super) fn load_release_notes(&mut self, cx: &mut Context<Self>) {
        if self.releases.busy || self.close != CloseState::Open { return; }
        self.releases.busy = true;
        self.releases.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move { Ok(Update::Releases(Box::new(Reply::Loaded(
            workspace.observe_current_build().await.map_err(|e| e.to_string()))))) });
        cx.notify();
    }
    fn acknowledge_release_notes(&mut self, cx: &mut Context<Self>) {
        if self.releases.busy || self.close != CloseState::Open { return; }
        let Some(journal) = &self.releases.journal else { return; };
        let revision = journal.revision;
        self.releases.busy = true;
        self.releases.error = None;
        let workspace = self.controller.workspace.clone();
        self.job(async move { Ok(Update::Releases(Box::new(Reply::Loaded(
            workspace.acknowledge_build_notes(revision).await.map_err(|e| e.to_string()))))) });
        cx.notify();
    }
    pub(super) fn release_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        self.releases.busy = false;
        match reply {
            Reply::Loaded(Ok(journal)) => {
                if journal.unread() && self.notice.is_none() {
                    self.notice = Some(NOTES_NOTICE.into());
                }
                if !journal.unread() && self.notice.as_deref() == Some(NOTES_NOTICE) { self.notice = None; }
                self.releases.journal = Some(journal);
            }
            Reply::Loaded(Err(error)) => self.releases.error = Some(error),
        }
        cx.notify();
    }
    pub(super) fn release_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let unread = self.releases.journal.as_ref().is_some_and(ReleaseJournal::unread);
        let mut page = div().flex().flex_col().gap_3()
            .child(div().relative().child(format!("Synara {} · development build", env!("CARGO_PKG_VERSION")))
                .child(ui::layout_probe("release-current-version")))
            .child(div().relative().child("Update status: unavailable. No production endpoint, signing authority or installation helper is configured. No update check was made.")
                .child(ui::layout_probe("release-update-unavailable")))
            .child(div().border_b_1().border_color(rgb(palette().border)).pb_3()
                .child("Published release history: no verified release catalog is bundled. The local observations below are not publication dates or proof of installed updates."))
            .child(div().relative().child(if unread { "Unread build notes" } else { "Build notes" })
                .child(ui::layout_probe(if unread { "release-unread" } else { "release-read" })))
            .child(div().relative().flex().flex_col().gap_1().children(BUNDLED_BUILD_NOTES.lines().map(|line| div().child(line.to_owned())))
                .child(ui::layout_probe("release-bundled-notes")))
            .child(div().flex().gap_2()
                .child(ui::action("release-ack", if self.releases.busy { "Saving..." } else { "Mark these notes read" }, Some(Glyph::Check), false,
                    cx.listener(|this, _: &(), _, cx| this.acknowledge_release_notes(cx))).relative()
                    .child(ui::layout_probe("release-ack")))
                .child(ui::action("release-reload", "Reload local history", None, false,
                    cx.listener(|this, _: &(), _, cx| this.load_release_notes(cx))).relative()
                    .child(ui::layout_probe("release-reload"))))
            .children(self.releases.error.as_ref().map(|error| div().relative().child(error.clone()).child(ui::layout_probe("release-error"))))
            .child(div().mt_3().border_t_1().border_color(rgb(palette().border)).pt_3().child("Builds observed in this installation (latest first, at most 24)"));
        if let Some(journal) = &self.releases.journal {
            for observed in journal.history.iter().rev() {
                page = page.child(div().text_xs().child(format!("{} · first observed Unix ms {} · notes SHA-256 {}",
                    observed.version, observed.observed_at_ms, observed.notes_sha256)));
            }
        }
        page.into_any_element()
    }
}
