//! Single-writer bridge from the native theme editor to persisted settings.
//! Rapid color/font changes coalesce behind in-flight writes without replacing
//! unrelated profile, permission, device or workspace preferences.
use super::*;
use crate::ui::theme_editor::ThemeEditorEvent;

impl Shell {
    pub(in crate::shell) fn stage_theme_event(&mut self,event:&ThemeEditorEvent,cx:&mut Context<Self>) {
        match event {
            ThemeEditorEvent::Changed(value)=>self.settings.theme_pending=Some(value.clone()),
            ThemeEditorEvent::Mode(mode)=>self.settings.theme_mode_pending=Some(*mode),
            ThemeEditorEvent::Opened=>{self.settings.popup=None;cx.notify();return;}
        }
        self.flush_theme_write(cx);
    }
    pub(in crate::shell) fn flush_theme_write(&mut self,cx:&mut Context<Self>) {
        if self.settings.saving || self.close!=CloseState::Open {return;}
        let next=self.settings.theme_pending.take();
        let mode=self.settings.theme_mode_pending.take();
        if next.is_none() && mode.is_none() {return;}
        self.settings.theme_inflight=true;
        self.save_setting(move|settings| {
            if let Some(value)=next {
                // Keep the legacy two-choice projection for old native consumers.
                // The versioned pack, not this compatibility flag, owns new paint.
                settings.appearance.dark_theme=if value.dark.code_theme_id=="dracula" {DarkThemePreference::Dracula}else{DarkThemePreference::Synara};
                settings.appearance.electron_theme=Some(value);
            }
            if let Some(mode)=mode {settings.appearance.theme=mode;}
        },cx);
        if !self.settings.saving {
            self.settings.theme_inflight=false;
            let error=self.error.clone().unwrap_or_else(||"The theme could not be saved.".into());
            self.settings.theme_editor.update(cx,|editor,cx|editor.save_failed(error,cx));
        }
    }
    pub(in crate::shell) fn finish_theme_write(&mut self,was_theme:bool,error:Option<String>,cx:&mut Context<Self>) {
        if let Some(error)=error {
            // Do not loop on a failed database write. The editor retains its draft
            // and offers an explicit retry, while persisted application paint stays
            // at the last successful value.
            if was_theme || self.settings.theme_pending.is_some() || self.settings.theme_mode_pending.is_some() {
                self.settings.theme_pending=None;
                self.settings.theme_mode_pending=None;
                self.settings.theme_editor.update(cx,|editor,cx|editor.save_failed(error,cx));
            }
            return;
        }
        if was_theme {self.settings.theme_editor.update(cx,|editor,cx|editor.saved(cx));}
        else if self.settings.theme_pending.is_none() && self.settings.theme_mode_pending.is_none() {
            let appearance=&self.settings.value.appearance;
            self.settings.theme_editor.update(cx,|editor,cx|editor.synchronize(appearance,cx));
        }
        self.flush_theme_write(cx);
    }
    pub(super) fn appearance_settings(&self,cx:&mut Context<Self>)->gpui::AnyElement {
        let appearance=&self.settings.value.appearance;
        self.settings.theme_editor.update(cx,|editor,_|editor.set_active(ui::theme::active_variant()));
        let modes=[
            (ThemePreference::System,"System","theme-system"),
            (ThemePreference::Light,"Light","theme-light"),
            (ThemePreference::Dark,"Dark","theme-dark"),
        ];
        div().flex().flex_col().gap_3()
            .child(div().flex().gap_3().children(modes.into_iter().enumerate().map(|(index,(mode,label,id))| {
                let selected=appearance.theme==mode;
                div().flex_1().min_w_0().flex().flex_col().gap(px(6.)).items_center()
                    .child(ui::button_shell(id,label,selected).p(px(3.)).w_full().rounded(px(14.)).border_2()
                        .border_color(if selected {rgb(palette().text)}else{gpui::rgba(0)})
                        .bg(gpui::rgba(0)).relative().child(ui::layout_probe(id))
                        .child(div().w_full().aspect_ratio(10./7.).relative().rounded_lg().overflow_hidden().child(ui::theme_mode::preview(mode)))
                        .on_click(cx.listener(move|this,_,_,cx|this.stage_theme_event(&ThemeEditorEvent::Mode(mode),cx)))
                        .on_key_down(cx.listener(move|this,event:&gpui::KeyDownEvent,_,cx| {
                            if event.prefer_character_input {return;}
                            let next=match event.keystroke.key.as_str() {"left"|"up"=>(index+2)%3,"right"|"down"=>(index+1)%3,"home"=>0,"end"=>2,_=>return};
                            this.stage_theme_event(&ThemeEditorEvent::Mode(modes[next].0),cx);cx.stop_propagation();
                        })))
                    .child(div().text_size(px(ui::ui_font_size())).line_height(gpui::relative(1.375))
                        .when(selected,|text|text.font_weight(gpui::FontWeight::MEDIUM))
                        .text_color(rgb(if selected {palette().text}else{palette().muted})).child(label))
            })))
            .child(self.settings.theme_editor.clone())
            .child(heading("Accessibility"))
            .child(card()
                .child(row("Higher contrast","Use stronger secondary text and separators. Custom wallpaper can still affect contrast.",
                    self.toggle("high-contrast","Higher contrast",appearance.high_contrast,|settings|settings.appearance.high_contrast=!settings.appearance.high_contrast,cx)))
                .child(row("Reduce motion","Turn off sliding and fading animations.",
                    self.toggle("reduce-motion","Reduce motion",appearance.reduced_motion,|settings|settings.appearance.reduced_motion=!settings.appearance.reduced_motion,cx))))
            .child(self.personalization_settings(cx))
            .into_any_element()
    }
}
