//! Grayscale theme-mode artwork from the pinned Electron ThemeModePicker.
//! These deliberately fixed colors do not follow the selected application theme.
use super::*;
use synara_workspace::ThemePreference;

pub fn preview(mode:ThemePreference)->gpui::Div {
    div().absolute().size_full().overflow_hidden()
        .child(scene(mode==ThemePreference::Dark,false))
        .when(mode==ThemePreference::System,|view|view.child(
            div().absolute().right_0().top_0().bottom_0().w(gpui::relative(0.5)).overflow_hidden()
                .child(scene(true,true).right_0().w(gpui::relative(2.)))
        ))
}
fn scene(dark:bool,mirror:bool)->gpui::Div {
    let (backdrop,panel,header,soft,card,row,hairline)=if dark {
        (0x5f5f5f,0x2c2c2c,0xa6a6a6,0x7d7d7d,0x3a3a3a,0x707070,0x4d4d4d)
    } else {(0xe9e9e9,0xf6f6f6,0xcfcfcf,0xe0e0e0,0xffffff,0xe3e3e3,0xefefef)};
    div().absolute().size_full().bg(rgb(backdrop))
        .child(div().absolute().left(gpui::relative(0.08)).right(gpui::relative(0.08)).top(gpui::relative(0.14)).bottom_0()
            .rounded_t_lg().bg(rgb(panel)).flex().flex_col()
            .child(div().pt(gpui::relative(0.09)).flex().flex_col().items_center().gap(px(4.))
                .child(div().h(px(4.)).w(gpui::relative(0.38)).rounded_full().bg(rgb(header)))
                .child(div().h(px(3.)).w(gpui::relative(0.55)).rounded_full().bg(rgb(soft))))
            .child(div().min_h_0().flex_1().mx(gpui::relative(0.09)).mt(gpui::relative(0.07)).rounded_t_md().bg(rgb(card)).overflow_hidden()
                .children((0..3).map(|_|div().px(gpui::relative(0.10)).pt(gpui::relative(0.07)).flex().flex_col()
                    .when(mirror,|row|row.items_end())
                    .child(div().h(px(4.)).w(gpui::relative(0.36)).rounded_full().bg(rgb(row)))
                    .child(div().h(px(1.)).w_full().mt(gpui::relative(0.07)).bg(rgb(hairline))))))
}
