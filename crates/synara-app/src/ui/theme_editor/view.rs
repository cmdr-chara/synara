//! Two-slot native theme editor, matching the reference's card and control roles.
use super::*;
use gpui::{MouseButton, KeyDownEvent};

fn text_action(id:impl Into<gpui::ElementId>,label:impl Into<SharedString>)->gpui::Stateful<gpui::Div> {
    button_shell(id,label,false).bg(gpui::rgba(0)).border_0().px_2().py_1()
        .text_color(theme::paint("textForegroundSecondary",palette().muted))
}
fn setting_row(label:impl Into<SharedString>,control:impl IntoElement)->gpui::Div {
    div().min_h(px(48.)).px_3().py(px(settings_row_padding())).border_t_1()
        .border_color(theme::paint("border",palette().border)).flex().items_center().justify_between().gap_3()
        .child(div().min_w_0().child(label.into()))
        .child(div().flex_shrink_0().max_w(px(224.)).child(control))
}
fn capture(bounds:Rc<Cell<Bounds<Pixels>>>)->impl IntoElement {
    canvas(move|value,_,_|bounds.set(value),|_,_,_,_|{}).absolute().size_full()
}
fn readable(color:u32)->u32 {
    let luminance=(0.299*((color>>16)&255) as f32+0.587*((color>>8)&255) as f32+0.114*(color&255) as f32)/255.;
    if luminance>0.6 {0x1a1c1f} else {0xffffff}
}
fn switch(id:impl Into<gpui::ElementId>,label:impl Into<SharedString>,enabled:bool)->gpui::Stateful<gpui::Div> {
    let label=label.into();
    button_shell(id,label.clone(),enabled).aria_label(format!("{label}: {}",if enabled {"on"}else{"off"}))
        .w(px(32.)).h(px(20.)).p(px(2.)).rounded_full().border_0()
        .bg(rgb(if enabled {palette().focus}else{palette().border})).flex().items_center()
        .when(enabled,|button|button.justify_end())
        .child(div().size(px(16.)).rounded_full().bg(rgb(0xffffff)))
}
impl ThemeEditor {
    fn color_control(&self,variant:ThemeVariant,field:ColorField,cx:&mut Context<Self>)->gpui::AnyElement {
        let index=slot(variant);
        let value=field.get(self.preferences.pack(variant));
        let foreground=readable(value.value());
        let changed=value!=field.get(&ThemePack::codex(variant));
        div().flex().items_center().gap_1()
            .children(changed.then(||text_action(("theme-color-reset",index*3+field.index()),"Reset")
                .text_size(px(metrics::Typography::from_base(ui_font_size()).small))
                .aria_label(format!("Reset {} theme {}",variant.label(),field.label()))
                .on_click(cx.listener(move|this,_,_,cx|this.reset_color(variant,field,cx)))))
            .child(button_shell((field.probe(),index),format!("{} theme {} color",variant.label(),field.label()),false)
                .w(px(176.)).h(px(32.)).px_2().rounded_lg().bg(rgb(value.value())).text_color(rgb(foreground))
                .border_1().border_color(rgb(foreground).alpha(0.32)).flex().items_center().gap_2().relative()
                .child(capture(self.bounds[index][field.index()].clone()))
                .child(layout_probe_slot(field.probe(),index))
                .child(div().size(px(20.)).rounded_full().border_1().border_color(rgb(foreground).alpha(0.32)))
                .child(value.to_string().to_uppercase())
                .on_click(cx.listener(move|this,event:&gpui::ClickEvent,window,cx| {
                    let position=this.position(variant,field.index(),event.position());
                    this.open_color(variant,field,position,window,cx);
                })))
            .into_any_element()
    }
    fn card(&self,variant:ThemeVariant,cx:&mut Context<Self>)->gpui::AnyElement {
        let index=slot(variant);
        let pack=self.preferences.pack(variant);
        let tokens=ThemeTokens::derive(&pack.theme,variant);
        let changed=*pack!=ThemePack::codex(variant);
        let active=self.active==variant;
        let label=theme_label(&pack.code_theme_id).to_owned();
        let row_font=metrics::Typography::from_base(ui_font_size());
        div().w_full().rounded_xl().border_1().border_color(theme::paint("border",palette().border)).overflow_hidden()
            .child(div().px_4().py_3().flex().flex_wrap().items_center().justify_between().gap_3()
                .child(div().flex().items_center().gap_2()
                    .child(div().text_size(px(row_font.large)).font_weight(gpui::FontWeight::MEDIUM).child(format!("{} theme",variant.label())))
                    .children(changed.then(||text_action(("theme-pack-reset",index),"Reset")
                        .relative().child(layout_probe_slot("theme-pack-reset",index))
                        .on_click(cx.listener(move|this,_,_,cx|this.reset(variant,cx))))))
                .child(div().flex().items_center().gap_1()
                    .child(text_action(("theme-pack-import",index),"Import").relative().child(layout_probe_slot("theme-pack-import",index))
                        .on_click(cx.listener(move|this,_,window,cx|this.open_import(variant,window,cx))))
                    .child(text_action(("theme-pack-copy",index),"Copy").relative().child(layout_probe_slot("theme-pack-copy",index))
                        .on_click(cx.listener(move|this,_,_,cx| {
                            match this.preferences.share(variant) {
                                Ok(value)=>{cx.write_to_clipboard(gpui::ClipboardItem::new_string(value));this.notice=Some("Theme copied.".into());}
                                Err(error)=>this.error=Some(error.to_string()),
                            }
                            cx.notify();
                        })))
                    .child(button_shell(("theme-pack-preset",index),format!("{} theme code theme",variant.label()),false)
                        .w(px(208.)).h(px(32.)).rounded_lg().border_1().border_color(theme::paint("border",palette().border))
                        .bg(gpui::rgba(0)).flex().items_center().gap_2().relative()
                        .child(capture(self.bounds[index][3].clone())).child(layout_probe_slot("theme-pack-preset",index))
                        .child(div().size(px(20.)).rounded_md().bg(rgb(pack.theme.surface.value())).text_color(rgb(pack.theme.accent.value()))
                            .border_1().border_color(rgb(tokens.color_on_surface("border").unwrap_or(pack.theme.ink.value()))).text_size(px(row_font.extra_small)).flex().items_center().justify_center().child("Aa"))
                        .child(div().flex_1().min_w_0().text_ellipsis().child(label.clone()))
                        .child(icon(Glyph::Chevron).size(px(12.)))
                        .on_click(cx.listener(move|this,event:&gpui::ClickEvent,window,cx| {
                            let position=this.position(variant,3,event.position());this.open_choices(variant,position,window,cx);
                        })))))
            .child(div().px_4().pb_3().flex().items_center().justify_between().gap_2().text_size(px(row_font.small))
                .text_color(theme::paint("textForegroundSecondary",palette().muted))
                .child(if active {"This is the active theme right now.".to_owned()}else{format!("Used when the app switches to {}.",variant.label().to_lowercase())})
                .children((!active).then(||text_action(("theme-pack-use",index),format!("Use {} theme",variant.label().to_lowercase()))
                    .border_1().border_color(theme::paint("border",palette().border))
                    .on_click(cx.listener(move|_,_,_,cx|cx.emit(ThemeEditorEvent::Mode(if variant==ThemeVariant::Dark {ThemePreference::Dark}else{ThemePreference::Light})))))))
            .child(div().px_4().pb_3().child(div().w_full().p_3().rounded_lg().border_1()
                .border_color(rgb(tokens.color_on_surface("border").unwrap_or(pack.theme.ink.value())))
                .bg(rgb(pack.theme.surface.value())).text_color(rgb(pack.theme.ink.value())).flex().items_center().justify_between().gap_3()
                .child(div().text_size(px(row_font.large)).font_weight(gpui::FontWeight::MEDIUM).child(label))
                .child(div().rounded_md().px_3().py_1().bg(rgb(tokens.color_on_surface("accentBackground").unwrap_or(pack.theme.accent.value())))
                    .text_color(rgb(tokens.color_on_surface("textAccent").unwrap_or(pack.theme.accent.value()))).child("Accent preview"))))
            .child(setting_row("Accent",self.color_control(variant,ColorField::Accent,cx)))
            .child(setting_row("Background",self.color_control(variant,ColorField::Surface,cx)))
            .child(setting_row("Foreground",self.color_control(variant,ColorField::Ink,cx)))
            .child(setting_row("UI font",div().w(px(224.)).flex().flex_col().gap_1()
                .child(div().relative().child(layout_probe_slot("theme-pack-ui-font",index)).child(self.fonts[index][0].clone()))
                .children(self.preferences.system_ui_font.then(||div().text_size(px(row_font.small)).text_color(theme::paint("textForegroundSecondary",palette().muted))
                    .child("Use system UI font is on; theme fonts are not applied.")))))
            .child(setting_row("Code font",div().w(px(224.)).relative().child(layout_probe_slot("theme-pack-code-font",index)).child(self.fonts[index][1].clone())))
            .child(setting_row("Translucent sidebar",switch(("theme-pack-translucent",index),format!("{} theme translucent sidebar",variant.label()),!pack.theme.opaque_windows)
                .relative().child(layout_probe_slot("theme-pack-translucent",index))
                .on_click(cx.listener(move|this,_,_,cx| {
                    let mut next=this.preferences.clone();let theme=&mut next.pack_mut(variant).theme;theme.opaque_windows=!theme.opaque_windows;this.accept(next,cx);
                }))))
            .child(setting_row("Contrast",div().flex().items_center().gap_3().child(self.contrast[index].clone())
                .child(div().w(px(28.)).text_right().font_family(code_font()).text_color(theme::paint("textForegroundSecondary",palette().muted)).child(pack.theme.contrast.to_string()))))
            .into_any_element()
    }
    pub fn overlay(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let Some(popup)=&self.popup else {return div().into_any_element();};
        let is_import=matches!(popup.content,PopupContent::Import{..});
        let body=match &popup.content {
            PopupContent::Choices(view,_)=>div().child(view.clone()).into_any_element(),
            PopupContent::Color{picker,..}=>div().id("theme-color-popup").rounded_lg().border_1()
                .border_color(theme::paint("border",palette().border)).bg(theme::paint("elevatedPrimaryOpaque",palette().overlay))
                .child(picker.clone()).into_any_element(),
            PopupContent::Import{text,variant,error,..}=>{
                let empty=text.read(cx).text().trim().is_empty();
                div().id("theme-import-dialog").role(gpui::Role::Dialog).aria_label(format!("Import {} theme",variant.label().to_lowercase()))
                    .w(px(448.)).max_w_full().rounded_xl().border_1().border_color(theme::paint("border",palette().border))
                    .bg(theme::paint("elevatedPrimaryOpaque",palette().overlay)).p_5().flex().flex_col().gap_3()
                    .child(div().text_size(px(18.)).font_weight(gpui::FontWeight::SEMIBOLD).child(format!("Import {} theme",variant.label().to_lowercase())))
                    .child(div().text_color(theme::paint("textForegroundSecondary",palette().muted)).child(format!("Paste a codex-theme-v1: share string. Its embedded variant must match {}, and its code theme must exist for that variant.",variant.label().to_lowercase())))
                    .child(div().h(px(140.)).relative().child(layout_probe("theme-import-text")).child(text.clone()))
                    .children(error.as_ref().map(|error|div().text_color(rgb(palette().error)).child(error.clone())))
                    .child(div().flex().justify_end().gap_2()
                        .child(button("theme-import-cancel","Cancel",false).track_focus(&popup.controls[0]).relative().child(layout_probe("theme-import-cancel"))
                            .on_click(cx.listener(|this,_,window,cx|this.dismiss(window,cx))))
                        .child(button("theme-import-confirm","Import",true).track_focus(&popup.controls[1]).relative().child(layout_probe_enabled("theme-import-confirm",!empty))
                            .when(empty,|button|button.opacity(0.45).cursor_default())
                            .on_click(cx.listener(move|this,_,window,cx|{if !empty {this.import(window,cx);}}))))
                    .into_any_element()
            }
        };
        let body=div().occlude().on_mouse_down(MouseButton::Left,|_,_,cx|cx.stop_propagation()).child(body);
        div().id("theme-editor-overlay").absolute().top_0().left_0().size_full().occlude()
            .when(is_import,|view|view.bg(gpui::rgba(0x00000066)).flex().items_center().justify_center().p_4())
            .on_mouse_down(MouseButton::Left,cx.listener(|this,_,window,cx| {this.dismiss(window,cx);cx.stop_propagation();}))
            .capture_key_down(cx.listener(|this,event:&KeyDownEvent,window,cx| {
                if event.prefer_character_input {return;}
                let Some(popup)=&this.popup else{return;};
                if event.keystroke.key=="escape" {
                    if let PopupContent::Import{text,..}=&popup.content && text.read(cx).is_composing() {return;}
                    this.dismiss(window,cx);cx.stop_propagation();return;
                }
                if event.keystroke.key=="tab" && let PopupContent::Import{text,..}=&popup.content {
                    let focus=[text.read(cx).focus_handle(cx),popup.controls[0].clone(),popup.controls[1].clone()];
                    let current=focus.iter().position(|focus|focus.is_focused(window)).unwrap_or(0);
                    let next=if event.keystroke.modifiers.shift {(current+2)%3}else{(current+1)%3};
                    window.focus(&focus[next],cx);cx.stop_propagation();
                }
            }))
            .child(if is_import {body.into_any_element()} else {
                gpui::anchored().anchor(gpui::Anchor::TopRight).position(popup.position).snap_to_window_with_margin(px(8.)).child(body).into_any_element()
            })
            .into_any_element()
    }
}
impl Render for ThemeEditor {
    fn render(&mut self,_:&mut Window,cx:&mut Context<Self>)->impl IntoElement {
        let order=if self.active==ThemeVariant::Dark {[ThemeVariant::Dark,ThemeVariant::Light]}else{[ThemeVariant::Light,ThemeVariant::Dark]};
        div().w_full().flex().flex_col().gap_3()
            .children(self.legacy.then(||div().text_color(theme::paint("textForegroundSecondary",palette().muted))
                .child("Your saved native appearance is preserved. Editing a theme adopts the versioned theme controls.")))
            .children(order.into_iter().map(|variant|self.card(variant,cx)))
            .child(div().rounded_xl().border_1().border_color(theme::paint("border",palette().border))
                .child(setting_row("Use system UI font",switch("theme-system-ui-font","Use system UI font",self.preferences.system_ui_font)
                    .relative().child(layout_probe("theme-system-ui-font"))
                    .on_click(cx.listener(|this,_,_,cx| {let mut next=this.preferences.clone();next.system_ui_font=!next.system_ui_font;this.accept(next,cx);}))))
            .children(self.error.as_ref().map(|error|div().text_color(rgb(palette().error)).child(error.clone())))
            .children(self.notice.as_ref().map(|notice|div().text_color(theme::paint("textForegroundSecondary",palette().muted)).child(notice.clone())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readable_color_uses_the_reference_pill_threshold() {
        assert_eq!(readable(0xffffff),0x1a1c1f);
        assert_eq!(readable(0x000000),0xffffff);
        assert_eq!(readable(0x0169cc),0xffffff);
    }
    #[test]
    fn color_fields_only_modify_the_selected_theme_property() {
        let original=ThemePack::codex(ThemeVariant::Light);
        for field in [ColorField::Accent,ColorField::Surface,ColorField::Ink] {
            let mut pack=original.clone();field.set(&mut pack,ThemeHex::rgb(0x123456));
            assert_eq!(field.get(&pack).value(),0x123456);
            assert_eq!(pack.theme.semantic_colors,original.theme.semantic_colors);
            assert_eq!(pack.theme.fonts,original.theme.fonts);
        }
    }
}
