use super::*;
use crate::ui;

impl Shell {
    pub(super) fn help_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().id("help-panel").relative().child(ui::layout_probe("help-panel")).flex_1().min_h_0().overflow_y_scroll().p_6().flex().flex_col().gap_4()
            .child(div().font_family("Cal Sans").text_size(px(28.)).child("Synara"))
            .child(self.releases_panel(cx))
            .child("Native application · MIT license")
            .child(div().text_size(px(12.)).child(include_str!("../../../../LICENSE")))
            .child("Keyboard shortcuts")
            .children(NAVIGATION_COMMANDS.iter().map(|command| div().child(format!("{}: {}", command.label, navigation_binding(&self.settings.value.keybindings, command)))))
            .child("Find a project file: Ctrl/Cmd+P · Search file contents: Ctrl/Cmd+Shift+F")
            .child("Agent approval requests are shown for your confirmation. Selecting a model or agent does not send a prompt.")
            .child(div().mt_4().text_size(px(20.)).child("Fonts and icons"))
            .child("Cal Sans · SIL Open Font License 1.1")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/CalSans-OFL.txt")))
            .child("Synara's Central icons and provider artwork")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/NOTICE.md")))
            .child("Tabler icons · MIT license")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/Tabler-MIT.txt")))
            .child("OpenAI glyph · Simple Icons (CC0), via React Icons (MIT)")
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/Simple-Icons-CC0.txt")))
            .child(div().text_size(px(12.)).child(include_str!("../../assets/licenses/React-Icons-MIT.txt")))
            .into_any_element()
    }
}
