//! Native build observations and unavailable production-release state stay distinct.
use super::*;
use crate::ui::{self, palette};
#[derive(Default)]
pub(super) struct ReleasesState {
    pub busy: bool,
    history: Option<NativeVersionHistory>,
    error: Option<String>,
    integrity_busy: bool,
    integrity: Option<NativeBuildIntegrity>,
    integrity_error: Option<String>,
}
impl Shell {
    pub(super) fn load_releases(&mut self, cx: &mut Context<Self>) {
        if self.releases.busy {
            return;
        }
        self.releases.busy = true;
        self.releases.error = None;
        if !self.releases.integrity_busy {
            self.releases.integrity_busy = true;
            self.releases.integrity = None;
            self.releases.integrity_error = None;
            let workspace = self.controller.workspace.clone();
            self.job(async move {
                Ok(Update::NativeBuildIntegrity(
                    workspace
                        .native_build_integrity()
                        .await
                        .map_err(|e| e.to_string()),
                ))
            });
        }
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Releases(
                workspace
                    .observe_native_version(env!("CARGO_PKG_VERSION").into())
                    .await
                    .map_err(|e| e.to_string()),
            ))
        });
        cx.notify();
    }
    pub(super) fn native_build_integrity_reply(
        &mut self,
        result: Result<NativeBuildIntegrity, String>,
        cx: &mut Context<Self>,
    ) {
        self.releases.integrity_busy = false;
        match result {
            Ok(value) => {
                self.releases.integrity = Some(value);
                self.releases.integrity_error = None;
            }
            Err(error) => {
                self.releases.integrity = None;
                self.releases.integrity_error = Some(error);
            }
        }
        cx.notify();
    }
    pub(super) fn releases_reply(
        &mut self,
        result: Result<NativeVersionHistory, String>,
        cx: &mut Context<Self>,
    ) {
        self.releases.busy = false;
        match result {
            Ok(value) => self.releases.history = Some(value),
            Err(error) => self.releases.error = Some(error),
        }
        cx.notify();
    }
    fn dismiss_build_notes(&mut self, cx: &mut Context<Self>) {
        if self.releases.busy {
            return;
        }
        let Some(history) = &self.releases.history else {
            return;
        };
        let revision = history.revision;
        self.releases.busy = true;
        let workspace = self.controller.workspace.clone();
        self.job(async move {
            Ok(Update::Releases(
                workspace
                    .mark_native_version_read(env!("CARGO_PKG_VERSION").into(), revision)
                    .await
                    .map_err(|e| e.to_string()),
            ))
        });
        cx.notify();
    }
    pub(super) fn releases_unread(&self) -> bool {
        self.releases
            .history
            .as_ref()
            .and_then(|h| h.current())
            .is_some_and(|v| !v.read)
    }
    pub(super) fn releases_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let mut pane=div().id("releases-surface").flex().flex_col().gap_2().border_b_1().border_color(rgb(palette().border)).pb_4()
            .child(div().relative().child(ui::layout_probe("native-build-version")).child(format!("Native build {} | {} / {}",env!("CARGO_PKG_VERSION"),std::env::consts::OS,std::env::consts::ARCH)))
            .child(div().text_size(px(18.)).child("What's New / Releases"))
            .child("This is a development build. Locally observed versions below are not published release announcements.")
            .child(div().text_size(px(12.)).child(include_str!("../../../../docs/ui/native-build-notes.md")))
            .child(div().relative().child(ui::layout_probe("native-update-unconfigured"))
                .child("Automatic updates unavailable: no production endpoint, trusted signing identity or platform replacement helper is configured. No update check or download was performed."))
            .child(div().text_size(px(12.)).child("Local executable integrity (on disk; SHA-256 only, not a signature or publisher verification):"));
        if self.releases.integrity_busy {
            pane = pane.child(
                div()
                    .text_size(px(12.))
                    .child("Fingerprinting the current executable…"),
            );
        }
        if let Some(integrity) = &self.releases.integrity {
            let file_name = integrity
                .path
                .file_name()
                .unwrap_or(integrity.path.as_os_str())
                .to_string_lossy();
            pane = pane.child(
                div()
                    .relative()
                    .child(ui::layout_probe("native-build-integrity"))
                    .text_size(px(12.))
                    .child(format!(
                        "{} | {} bytes | SHA-256 {}",
                        file_name, integrity.size_bytes, integrity.sha256
                    )),
            );
        }
        pane = pane.child("Published native release history: not configured. Electron releases and fork tags are not presented as native releases.");
        if let Some(error) = &self.releases.integrity_error {
            pane = pane.child(div().text_color(rgb(palette().error)).child(format!(
                "Could not fingerprint the current executable: {error}"
            )));
        }
        if self.releases_unread() {
            pane = pane
                .child(
                    div()
                        .relative()
                        .child(ui::layout_probe("native-build-unread"))
                        .child(
                            "This native version has not been marked read on this installation.",
                        ),
                )
                .child(
                    ui::action(
                        "native-build-read",
                        if self.releases.busy {
                            "Saving..."
                        } else {
                            "Mark this version read"
                        },
                        None,
                        false,
                        cx.listener(|this, _: &(), _, cx| this.dismiss_build_notes(cx)),
                    )
                    .relative()
                    .child(ui::layout_probe("native-build-read")),
                );
        }
        if let Some(history) = &self.releases.history {
            pane = pane
                .child("Versions observed on this installation (most recent first)")
                .child(
                    div()
                        .id("native-version-visits")
                        .max_h(px(160.))
                        .overflow_y_scroll()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .children(history.visits.iter().rev().map(|v| {
                            div().text_size(px(12.)).child(format!(
                                "{} | observed {} | {}",
                                v.version,
                                v.observed_label(),
                                if v.read { "read" } else { "unread" }
                            ))
                        })),
                );
        }
        if let Some(error) = &self.releases.error {
            pane = pane.child(div().text_color(rgb(palette().error)).child(error.clone()));
        }
        pane.child(ui::action(
            "native-build-reload",
            "Reload local version state",
            None,
            false,
            cx.listener(|this, _: &(), _, cx| this.load_releases(cx)),
        ))
        .into_any_element()
    }
}
