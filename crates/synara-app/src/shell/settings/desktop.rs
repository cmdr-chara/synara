//! Desktop settings connect to existing native owners, not duplicate runtimes.
use super::*;
use super::navigation::{SECTIONS, primary_section};
use synara_runtime::SnapTools;

impl Shell {
    pub(super) fn appsnap_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let support = SnapTools::support();
        let supported = support.is_ok();
        let has_task = self.selected.is_some();
        let status = support.map_or_else(|error| error.to_string(), |_| {
            "Linux/X11 window capture is supported. Setup and window selection are explicit.".into()
        });
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
        // Do not reuse AppSnap's capture consent as desktop-control authority.
        // The reference section is present, but an unimplemented controller must
        // not be represented by a toggle that appears to enable agent control.
        div()
            .child(heading("Computer control"))
            .child(card()
                .child(row("Desktop control", "A permissioned native desktop-control backend is not configured in this build. Agents cannot use this page to send keyboard or pointer input.", "Unavailable"))
                .child(row("Backend status", "Window capture and simulator helpers are separate capabilities. Neither grants desktop-control authority.", "No control backend")))
            .child(heading("Available native tools"))
            .child(card()
                .child(row("AppSnap", "Capture a reviewed application window into a pending chat attachment.",
                    ui::button("computer-open-appsnap", "Open AppSnap", false)
                        .on_click(cx.listener(|this, _, _, cx| this.open_settings_section(Section::AppSnap, cx)))))
                .child(row("Devices", "Inspect supported device helpers and their existing permission controls.",
                    ui::button("computer-open-devices", "Open devices", false)
                        .on_click(cx.listener(|this, _, _, cx| this.open_settings_section(Section::Device, cx))))))
            .into_any_element()
    }

    pub(super) fn native_extensions_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div()
            .child(heading("Native workspace tools"))
            .child(card().children(SECTIONS.iter().filter(|item| !primary_section(item.section)).map(|item| {
                let section = item.section;
                row(item.label, item.description,
                    ui::button(("native-settings-extension", item.id), "Open", false)
                        .on_click(cx.listener(move |this, _, _, cx| this.open_settings_section(section, cx))))
            })))
            .into_any_element()
    }
}
