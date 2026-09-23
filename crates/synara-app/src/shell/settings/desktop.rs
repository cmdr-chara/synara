//! Desktop settings connect to existing native owners, not duplicate runtimes.
use super::navigation::{SECTIONS, primary_section};
use super::*;
use synara_runtime::SnapTools;

impl Shell {
    pub(super) fn appsnap_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let support = SnapTools::support();
        let supported = support.is_ok();
        let has_task = self.selected.is_some();
        let status = support.map_or_else(
            |error| error.to_string(),
            |_| {
                "Linux/X11 window capture is supported. Setup and window selection are explicit."
                    .into()
            },
        );
        div()
            .child(heading("Screen capture"))
            .child(card()
                .child(row("AppSnap", "Capture another app's window into the current chat. Captures remain pending attachments until you send them.",
                    ui::button("settings-appsnap-open", "Choose a window", false)
                        .when(!supported || !has_task, |button| button.opacity(0.45).cursor_default())
                        .relative()
                        .child(ui::layout_probe_enabled("settings-appsnap-open", supported && has_task))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if supported && this.selected.is_some() {
                                this.open_appsnap_from_settings(cx);
                            }
                        }))))
                .child(row("Backend status", status, if supported { "Available" } else { "Unavailable" }))
                .child(row("Capture consent", "Select and review one window before each capture. Discovery reads window identities, not pixels. Switching tasks discards pending capture consent.", "Per capture")))
            .children((!has_task).then(|| {
                div().mt_3().text_color(rgb(palette().muted))
                    .child("Open a chat first so the captured image has an explicit destination.")
            }))
            .child(heading("Permissions"))
            .child(card()
                .child(row("Operating system permission", "X11 does not report a per-application screen-recording permission. Synara does not interpret that absence as user consent.", "Not reported"))
                .child(row("Computer control", "AppSnap captures one reviewed window. It never authorizes keyboard or pointer input, desktop-wide capture, or background recording.", "Not granted")))
            .into_any_element()
    }

    pub(super) fn computer_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        self.autonomy_computer_settings(cx)
    }

    pub(super) fn native_extensions_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .child(heading("Native workspace tools"))
            .child(
                card().children(
                    SECTIONS
                        .iter()
                        .filter(|item| !primary_section(item.section))
                        .map(|item| {
                            let section = item.section;
                            row(
                                item.label,
                                item.description,
                                ui::button(
                                    ("native-settings-extension", section as usize),
                                    "Open",
                                    false,
                                )
                                .on_click(cx.listener(
                                    move |this, _, _, cx| this.open_settings_section(section, cx),
                                )),
                            )
                        }),
                ),
            )
            .into_any_element()
    }
}
