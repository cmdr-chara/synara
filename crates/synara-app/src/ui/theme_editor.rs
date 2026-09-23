//! Native versioned theme workbench. Persistence remains owned by Shell.
//! Reuses native text input, searchable choices, HSV picker and bounded slider.
use super::*;
use crate::input::{EntryEvent, EntryMode, TextEntry};
use super::color_picker::{ColorEvent, ColorPicker};
use super::menu::{Choice, ChoiceEvent, ChoiceMenu};
use super::slider::{SliderChanged, ValueSlider};
use gpui::{Bounds, Entity, EventEmitter, FocusHandle, Focusable, Pixels, Point, Subscription};
use std::{cell::Cell, rc::Rc};
use synara_workspace::{AppearanceSettings, DarkThemePreference, ThemeHex, ThemePack, ThemePreference,
    ThemePreferences, ThemeVariant, ThemeTokens, available_theme_options, theme_label};

mod view;

#[derive(Clone)]
pub enum ThemeEditorEvent {
    Changed(ThemePreferences),
    Mode(ThemePreference),
    Opened,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColorField { Accent, Surface, Ink }
impl ColorField {
    fn index(self) -> usize { match self { Self::Accent=>0,Self::Surface=>1,Self::Ink=>2 } }
    fn label(self) -> &'static str { match self { Self::Accent=>"Accent",Self::Surface=>"Background",Self::Ink=>"Foreground" } }
    fn probe(self) -> &'static str { match self { Self::Accent=>"theme-pack-accent",Self::Surface=>"theme-pack-background",Self::Ink=>"theme-pack-foreground" } }
    fn get(self, pack: &ThemePack) -> ThemeHex { match self { Self::Accent=>pack.theme.accent,Self::Surface=>pack.theme.surface,Self::Ink=>pack.theme.ink } }
    fn set(self, pack: &mut ThemePack, value: ThemeHex) { match self { Self::Accent=>pack.theme.accent=value,Self::Surface=>pack.theme.surface=value,Self::Ink=>pack.theme.ink=value } }
}
fn slot(variant: ThemeVariant) -> usize { if variant==ThemeVariant::Light { 0 } else { 1 } }
fn variant(index: usize) -> ThemeVariant { if index==0 { ThemeVariant::Light } else { ThemeVariant::Dark } }

enum PopupContent {
    Choices(Entity<ChoiceMenu>, Subscription),
    Color { picker: Entity<ColorPicker>, variant: ThemeVariant, field: ColorField, _subscription: Subscription },
    Import { text: Entity<TextEntry>, variant: ThemeVariant, error: Option<String>, _subscription: Subscription },
}
struct Popup {
    content: PopupContent,
    position: Point<Pixels>,
    return_focus: FocusHandle,
    controls: [FocusHandle; 2],
}

pub struct ThemeEditor {
    preferences: ThemePreferences,
    legacy: bool,
    active: ThemeVariant,
    fonts: [[Entity<TextEntry>; 2]; 2],
    contrast: [Entity<ValueSlider>; 2],
    bounds: [[Rc<Cell<Bounds<Pixels>>>; 5]; 2],
    popup: Option<Popup>,
    error: Option<String>,
    notice: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ThemeEditorEvent> for ThemeEditor {}
impl ThemeEditor {
    pub fn new(appearance: &AppearanceSettings, cx: &mut Context<Self>) -> Self {
        let preferences = from_appearance(appearance);
        let fonts: [[Entity<TextEntry>;2];2] = std::array::from_fn(|index| std::array::from_fn(|field| {
            cx.new(|cx| {
                let mut entry = TextEntry::new(if field==0 {"System default"} else {"JetBrains Mono"},EntryMode::SingleLine,32.,cx);
                let font = if field==0 { &preferences.pack(variant(index)).theme.fonts.ui } else { &preferences.pack(variant(index)).theme.fonts.code };
                entry.set_text(font.clone().unwrap_or_default(),cx);
                entry
            })
        }));
        let contrast: [Entity<ValueSlider>;2] = std::array::from_fn(|index| cx.new(|cx| {
            ValueSlider::new(if index==0 {"Light theme contrast"} else {"Dark theme contrast"},
                if index==0 {"theme-light-contrast"} else {"theme-dark-contrast"},100.,f32::from(preferences.pack(variant(index)).theme.contrast),false,cx)
        }));
        let mut subscriptions = Vec::new();
        for index in 0..2 {
            let variant = variant(index);
            for field in 0..2 {
                subscriptions.push(cx.subscribe(&fonts[index][field],move|this,entry,event,cx| {
                    if !matches!(event,EntryEvent::Changed) { return; }
                    let font = entry.read(cx).text().trim().to_owned();
                    let font = (!font.is_empty()).then_some(font);
                    let mut candidate = this.preferences.clone();
                    if field==0 {candidate.pack_mut(variant).theme.fonts.ui=font;}
                    else {candidate.pack_mut(variant).theme.fonts.code=font;}
                    this.accept(candidate,cx);
                }));
            }
            subscriptions.push(cx.subscribe(&contrast[index],move|this,_,SliderChanged(value),cx| {
                let mut candidate=this.preferences.clone();
                candidate.pack_mut(variant).theme.contrast=value.round().clamp(0.,100.) as u8;
                this.accept(candidate,cx);
            }));
        }
        Self { preferences,legacy:appearance.electron_theme.is_none(),active:ThemeVariant::Light,fonts,contrast,
            bounds:std::array::from_fn(|_|std::array::from_fn(|_|Rc::new(Cell::new(Bounds::default())))),
            popup:None,error:None,notice:None,_subscriptions:subscriptions }
    }
    pub fn set_active(&mut self, active: ThemeVariant) { self.active=active; }
    pub fn has_popup(&self) -> bool { self.popup.is_some() }
    pub fn synchronize(&mut self, appearance:&AppearanceSettings,cx:&mut Context<Self>) {
        let next=from_appearance(appearance);
        if self.preferences==next && self.legacy==appearance.electron_theme.is_none() {return;}
        self.preferences=next;
        self.legacy=appearance.electron_theme.is_none();
        self.sync_inputs(cx);
        cx.notify();
    }
    pub fn save_failed(&mut self,error:String,cx:&mut Context<Self>) {self.error=Some(error);cx.notify();}
    pub fn saved(&mut self,cx:&mut Context<Self>) {self.error=None;cx.notify();}
    fn sync_inputs(&mut self,cx:&mut Context<Self>) {
        for index in 0..2 {
            let theme=&self.preferences.pack(variant(index)).theme;
            for field in 0..2 {
                let text=if field==0 {theme.fonts.ui.clone()} else {theme.fonts.code.clone()}.unwrap_or_default();
                self.fonts[index][field].update(cx,|entry,cx| {
                    if entry.text()!=text {entry.set_text(text,cx);}
                });
            }
            self.contrast[index].update(cx,|slider,cx|slider.set_value(f32::from(theme.contrast),cx));
        }
    }
    fn accept(&mut self,candidate:ThemePreferences,cx:&mut Context<Self>) {
        if let Err(error)=candidate.validate() {self.error=Some(error.to_string());cx.notify();return;}
        if candidate==self.preferences && !self.legacy {return;}
        self.preferences=candidate;
        self.legacy=false;
        self.error=None;
        self.notice=None;
        cx.emit(ThemeEditorEvent::Changed(self.preferences.clone()));
        cx.notify();
    }
    fn persist_preview(&mut self,cx:&mut Context<Self>) {
        self.legacy=false;
        self.error=None;
        cx.emit(ThemeEditorEvent::Changed(self.preferences.clone()));
        cx.notify();
    }
    fn position(&self,variant:ThemeVariant,field:usize,fallback:Point<Pixels>)->Point<Pixels> {
        let bounds=self.bounds[slot(variant)][field].get();
        if bounds.size.width>px(0.) {gpui::point(bounds.right(),bounds.bottom()+px(8.))} else {fallback}
    }
    fn reset(&mut self,variant:ThemeVariant,cx:&mut Context<Self>) {
        let mut next=self.preferences.clone();next.reset(variant);self.accept(next,cx);self.sync_inputs(cx);
    }
    fn reset_color(&mut self,variant:ThemeVariant,field:ColorField,cx:&mut Context<Self>) {
        let mut next=self.preferences.clone();field.set(next.pack_mut(variant),field.get(&ThemePack::codex(variant)));self.accept(next,cx);
    }
    fn open_choices(&mut self,variant:ThemeVariant,position:Point<Pixels>,window:&mut Window,cx:&mut Context<Self>) {
        self.dismiss(window,cx);
        let options=match available_theme_options(variant) {Ok(options)=>options,Err(error)=>{self.error=Some(error.to_string());cx.notify();return;}};
        let selected=&self.preferences.pack(variant).code_theme_id;
        let choices=options.iter().map(|(id,label)|Choice{label:(*label).into(),selected:*id==selected,..Default::default()}).collect();
        let view=cx.new(|cx|ChoiceMenu::new(format!("{} theme",variant.label()),choices,cx));
        let return_focus=window.focused(cx).unwrap_or_else(||view.read(cx).focus_handle(cx));
        let restore=return_focus.clone();
        let handle=window.window_handle();
        let subscription=cx.subscribe(&view,move|this,_,event,cx| {
            if let ChoiceEvent::Selected(index)=event && let Some((id,_))=options.get(*index) {
                let mut next=this.preferences.clone();
                match next.select(id,variant) {Ok(())=>{this.accept(next,cx);this.sync_inputs(cx);},Err(error)=>this.error=Some(error.to_string())}
            }
            this.popup=None;
            let focus=restore.clone();
            cx.defer(move|cx|{cx.update_window(handle,|_,window,cx|window.focus(&focus,cx)).ok();});
            cx.notify();
        });
        window.focus(&view.read(cx).focus_handle(cx),cx);
        self.popup=Some(Popup{position,return_focus,content:PopupContent::Choices(view,subscription),controls:std::array::from_fn(|_|cx.focus_handle())});
        cx.emit(ThemeEditorEvent::Opened);cx.notify();
    }
    fn open_color(&mut self,variant:ThemeVariant,field:ColorField,position:Point<Pixels>,window:&mut Window,cx:&mut Context<Self>) {
        self.dismiss(window,cx);
        let value=field.get(self.preferences.pack(variant));
        let picker=cx.new(|cx|ColorPicker::new(value,cx));
        let return_focus=window.focused(cx).unwrap_or_else(||picker.read(cx).focus_handle(cx));
        let subscription=cx.subscribe(&picker,move|this,_,event,cx| {
            if !matches!(&this.popup,Some(Popup{content:PopupContent::Color{variant:active,field:active_field,..},..}) if *active==variant && *active_field==field) {return;}
            match *event {
                ColorEvent::Preview(value)=>{field.set(this.preferences.pack_mut(variant),value);cx.notify();}
                ColorEvent::Commit(value)=>{field.set(this.preferences.pack_mut(variant),value);this.persist_preview(cx);}
            }
        });
        window.focus(&picker.read(cx).focus_handle(cx),cx);
        self.popup=Some(Popup{position,return_focus,content:PopupContent::Color{picker,variant,field,_subscription:subscription},controls:std::array::from_fn(|_|cx.focus_handle())});
        cx.emit(ThemeEditorEvent::Opened);cx.notify();
    }
    fn open_import(&mut self,variant:ThemeVariant,window:&mut Window,cx:&mut Context<Self>) {
        self.dismiss(window,cx);
        let text=cx.new(|cx|TextEntry::new("codex-theme-v1:",EntryMode::Editor,140.,cx));
        let return_focus=window.focused(cx).unwrap_or_else(||text.read(cx).focus_handle(cx));
        let subscription=cx.subscribe(&text,|this,_,_,cx| {
            if let Some(Popup{content:PopupContent::Import{error,..},..})=&mut this.popup {*error=None;}
            cx.notify();
        });
        window.focus(&text.read(cx).focus_handle(cx),cx);
        self.popup=Some(Popup{position:gpui::point(px(0.),px(0.)),return_focus,
            content:PopupContent::Import{text,variant,error:None,_subscription:subscription},controls:std::array::from_fn(|_|cx.focus_handle())});
        cx.emit(ThemeEditorEvent::Opened);cx.notify();
    }
    fn import(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        let Some(Popup{content:PopupContent::Import{text,variant,..},..})=&self.popup else{return;};
        let mut next=self.preferences.clone();
        match next.import(text.read(cx).text(),*variant) {
            Ok(())=>{self.accept(next,cx);self.sync_inputs(cx);self.dismiss(window,cx);self.notice=Some("Theme imported.".into());}
            Err(error)=>{if let Some(Popup{content:PopupContent::Import{error:slot,..},..})=&mut self.popup {*slot=Some(error.to_string());}cx.notify();}
        }
    }
    pub fn dismiss(&mut self,window:&mut Window,cx:&mut Context<Self>) {
        let Some(popup)=self.popup.take() else{return;};
        if let PopupContent::Color{picker,variant,field,..}=&popup.content
            && let Some(value)=picker.update(cx,|picker,cx|picker.flush(cx)) {
            field.set(self.preferences.pack_mut(*variant),value);self.persist_preview(cx);
        }
        window.focus(&popup.return_focus,cx);cx.notify();
    }
    /// Panel navigation can retire an overlay without stealing focus back to a
    /// now-hidden trigger. A valid outstanding color still follows the write queue.
    pub fn retire(&mut self,cx:&mut Context<Self>) {
        let Some(popup)=self.popup.take() else{return;};
        if let PopupContent::Color{picker,variant,field,..}=&popup.content
            && let Some(value)=picker.update(cx,|picker,cx|picker.flush(cx)) {
            field.set(self.preferences.pack_mut(*variant),value);self.persist_preview(cx);
        }
        cx.notify();
    }
}

fn from_appearance(appearance:&AppearanceSettings)->ThemePreferences {
    if let Some(preferences)=&appearance.electron_theme {return preferences.clone();}
    let mut preferences=ThemePreferences::default();
    // Legacy profiles keep their saved native appearance until the user edits a
    // theme. The editor reflects those colors rather than claiming they are Codex.
    for variant in [ThemeVariant::Light,ThemeVariant::Dark] {
        let dark=variant==ThemeVariant::Dark;
        let pack=preferences.pack_mut(variant);
        pack.code_theme_id=if dark && appearance.dark_theme==DarkThemePreference::Dracula {"dracula"} else {"synara"}.into();
        let colors=if !dark {LIGHT} else if appearance.dark_theme==DarkThemePreference::Dracula {
            Palette{canvas:0x282a36,text:0xf8f8f2,focus:0xff79c6,..DARK}
        } else {DARK};
        pack.theme.surface=ThemeHex::rgb(colors.canvas);
        pack.theme.ink=ThemeHex::rgb(colors.text);
        pack.theme.accent=ThemeHex::rgb(appearance.personalization.accent.unwrap_or(colors.focus));
        pack.theme.fonts.ui=appearance.fonts.ui_family.clone();
        pack.theme.fonts.code=appearance.fonts.code_family.clone();
    }
    preferences.system_ui_font=appearance.fonts.ui_family.is_none();
    preferences
}
