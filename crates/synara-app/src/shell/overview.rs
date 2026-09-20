use super::*;
use crate::ui;

impl Shell {
    pub(super) fn help_panel(&self) -> gpui::AnyElement {
        div().id("help-panel").relative().child(ui::layout_probe("help-panel")).flex_1().min_h_0().overflow_y_scroll().p_6().flex().flex_col().gap_4()
            .child(div().font_family("Cal Sans").text_size(px(28.)).child("Synara"))
            .child("Native application · MIT license")
            .child(div().text_size(px(12.)).child(include_str!("../../../../LICENSE")))
            .child("Keyboard shortcuts")
            .child("Ctrl/Cmd + 1: Conversation · 2: Files · 3: Changes · 4: Terminal · 5: Inspector · 6: Settings · 7: Agents · 8: Remote · 9: Kanban")
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
