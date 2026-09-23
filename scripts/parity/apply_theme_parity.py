#!/usr/bin/env python3
"""One-shot integration of the reviewed native theme modules into an exact base.

All input hashes and replacement counts are checked before any source is written.
The normal application never runs this development bootstrap.
"""
from __future__ import annotations
import hashlib
from pathlib import Path
import subprocess

EXPECTED={
 'crates/synara-workspace/src/settings.rs':'93d4631bbcd03915c754904a8951d1d65c64df39',
 'crates/synara-workspace/src/settings/theme.rs':'0cf57477dec229d5a2e380a9e5865364e45b22ee',
 'crates/synara-app/src/ui.rs':'a10f3592da7888bf453158733d47be5176ee2c51',
 'crates/synara-app/src/ui/theme.rs':'abaccb1a44dbef5422e98029fca9c112cb530f58',
 'crates/synara-app/src/ui/theme_editor/view.rs':'3e66a5420206ee55a5103a30757f523de9f07e74',
 'crates/synara-app/src/shell/settings.rs':'bec78978c3948e0e5c4e37d1d4272340f2fbc16f',
 'crates/synara-app/src/shell.rs':'38710e55ca2b696747219c67833f019ffdff3a94',
 'crates/synara-app/src/main.rs':'26050414bf3cf2c7d0ffce44005c19c06df42795',
}

def replace(text,old,new,count=1):
    if text.count(old)!=count:raise ValueError(f'Expected {count} exact matches: {old[:100]!r}; found {text.count(old)}')
    return text.replace(old,new)

def region(text,start,end,transform):
    if text.count(start)!=1 or text.count(end)!=1:raise ValueError('Ambiguous integration region')
    a=text.index(start);b=text.index(end,a)
    return text[:a]+transform(text[a:b])+text[b:]

def prepare(root:Path):
    sources={}
    for name,expected in EXPECTED.items():
        path=root/name
        if not path.is_file() or path.is_symlink() or not path.resolve().is_relative_to(root):raise ValueError(f'Unsafe source {name}')
        data=path.read_bytes()
        actual=hashlib.sha1(f'blob {len(data)}\0'.encode()+data).hexdigest()
        if actual!=expected:raise ValueError(f'Source changed, re-review {name}: {actual}')
        sources[name]=data.decode('utf-8')

    name='crates/synara-workspace/src/settings.rs'
    text=replace(sources[name],'mod chat;\npub use chat::ChatSettings;','mod chat;\npub use chat::ChatSettings;\nmod theme;\npub use theme::*;')
    text=replace(text,'#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct AppearanceSettings {',
        '#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct AppearanceSettings {\n    #[serde(default)]\n    pub electron_theme: Option<ThemePreferences>,')
    text=replace(text,'#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]\n#[serde(deny_unknown_fields)]\npub struct KeyBinding {','''impl Default for AppearanceSettings {
    fn default() -> Self {
        Self { electron_theme: Some(ThemePreferences::default()), personalization: Personalization::default(),
            dark_theme: DarkThemePreference::default(), theme: ThemePreference::default(), fonts: FontPreferences::default(),
            reduced_motion: false, high_contrast: false }
    }
}
fn legacy_appearance_defaults() -> AppearanceSettings {
    AppearanceSettings { electron_theme: None, ..AppearanceSettings::default() }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyBinding {''')
    text=replace(text,'    #[serde(default)]\n    pub appearance: AppearanceSettings,','    #[serde(default = "legacy_appearance_defaults")]\n    pub appearance: AppearanceSettings,')
    text=replace(text,'        self.appearance.personalization.validate()?;','        self.appearance.personalization.validate()?;\n        if let Some(theme) = &self.appearance.electron_theme { theme.validate()?; }')
    sources[name]=text

    name='crates/synara-workspace/src/settings/theme.rs'
    text=replace(sources[name],'pub struct ThemeFonts { pub ui: Option<String>, pub code: Option<String> }','''pub struct ThemeFonts {
    #[serde(deserialize_with = "required_nullable_font")]
    pub ui: Option<String>,
    #[serde(deserialize_with = "required_nullable_font")]
    pub code: Option<String>,
}
fn required_nullable_font<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}''')
    text=replace(text,'#[cfg(test)]\nmod tests;','#[cfg(test)]\nmod tests;\n#[cfg(test)]\nmod oracle;')
    sources[name]=text

    name='crates/synara-app/src/ui.rs'
    text=replace(sources[name],'pub mod task_dialog;','pub mod task_dialog;\npub mod theme;\npub mod theme_editor;\npub mod theme_mode;\npub mod slider;\npub mod color_picker;\npub const CODE_FONT: &str = "JetBrains Mono";')
    text=replace(text,'    let palette = if !dark {','    let palette = if let Some(palette) = theme::configure(appearance, dark) {\n        palette\n    } else if !dark {')
    text=region(text,'    UI_FAMILY.with(|family| {\n        *family.borrow_mut() = appearance','\n}\n\n/// GPUI maps',lambda _:'''    let selected = appearance.electron_theme.as_ref().map(|themes| (themes, themes.pack(theme::active_variant())));
    let ui_family = match selected {
        Some((themes, pack)) if !themes.system_ui_font => pack.theme.fonts.ui.as_deref().map(|value| theme::family(value, UI_FONT)).unwrap_or_else(|| UI_FONT.into()),
        Some(_) => UI_FONT.into(),
        None => appearance.fonts.ui_family.clone().unwrap_or_else(|| UI_FONT.into()),
    };
    let code_family = match selected {
        Some((_, pack)) => pack.theme.fonts.code.as_deref().map(|value| theme::family(value, CODE_FONT)).unwrap_or_else(|| CODE_FONT.into()),
        None => appearance.fonts.code_family.clone().unwrap_or_else(|| "DejaVu Sans Mono".into()),
    };
    UI_FAMILY.with(|family| *family.borrow_mut() = ui_family.into());
    CODE_FAMILY.with(|family| *family.borrow_mut() = code_family.into());''')
    text=text.replace('.border_color(rgb(palette().border))','.border_color(theme::paint("border", palette().border))')
    text=text.replace('.text_color(rgb(palette().text))','.text_color(theme::paint("textForeground", palette().text))')
    sources[name]=text

    name='crates/synara-app/src/ui/theme.rs'
    text=replace(sources[name],'    static EXACT_ROLES: Cell<bool> = const { Cell::new(false) };','    static EXACT_ROLES: Cell<bool> = const { Cell::new(false) };\n    static FONT_NAMES: RefCell<std::collections::BTreeSet<String>> = const { RefCell::new(std::collections::BTreeSet::new()) };')
    text=region(text,'pub fn family(value: &str, default: &str) -> String {','\n#[cfg(test)]',lambda _:'''pub fn register_font_names(names: Vec<String>) {
    FONT_NAMES.with(|fonts| *fonts.borrow_mut() = names.into_iter().map(|name| name.to_lowercase()).collect());
}
pub fn family(value: &str, default: &str) -> String {
    let mut parts = Vec::new();
    let mut quote = None;
    let mut start = 0;
    for (index, character) in value.char_indices() {
        if let Some(active) = quote { if character == active { quote = None; } }
        else if character == '\'' || character == '"' { quote = Some(character); }
        else if character == ',' { parts.push(&value[start..index]); start = index + 1; }
    }
    parts.push(&value[start..]);
    for part in parts {
        let family = part.trim().trim_matches(['\'', '"']).trim();
        if family.is_empty() || family.contains('(') { continue; }
        if matches!(family, "inherit" | "initial" | "unset" | "revert" | "revert-layer" | "system-ui" | "sans-serif" | "serif" | "monospace" | "ui-monospace" | "ui-sans-serif") { return default.into(); }
        if FONT_NAMES.with(|fonts| fonts.borrow().is_empty() || fonts.borrow().contains(&family.to_lowercase())) { return family.into(); }
    }
    default.into()
}
''')
    sources[name]=text

    name='crates/synara-app/src/ui/theme_editor/view.rs'
    text=replace(sources[name],'.children(self.error.as_ref().map(|error|div().text_color(rgb(palette().error)).child(error.clone())))',
        '.children(self.error.as_ref().map(|error|div().flex().items_center().gap_3().text_color(rgb(palette().error)).child(error.clone()).child(button("theme-save-retry", "Retry saving theme", false).on_click(cx.listener(|this, _, _, cx| this.persist_preview(cx))))))')
    sources[name]=text

    name='crates/synara-app/src/shell/settings.rs'
    text=replace(sources[name],'mod navigation;','mod navigation;\nmod theme_bridge;')
    text=replace(text,'    pub saving: bool,','    pub saving: bool,\n    pub theme_editor: Entity<ui::theme_editor::ThemeEditor>,\n    pub theme_pending: Option<synara_workspace::ThemePreferences>,\n    pub theme_mode_pending: Option<ThemePreference>,\n    pub theme_inflight: bool,')
    text=replace(text,'    ui_font: Entity<TextEntry>,\n    code_font: Entity<TextEntry>,\n','')
    text=region(text,'        let ui_font = cx.new(', '        let subscriptions = ',lambda _:'''        for (entry, text) in [(&name, value.profile.name.clone()), (&username, value.profile.username.clone())] {
            entry.update(cx, |entry, cx| entry.set_text(text, cx));
        }
        let theme_editor = cx.new(|cx| ui::theme_editor::ThemeEditor::new(&value.appearance, cx));
''')
    text=replace(text,'        let subscriptions = vec![','        let mut subscriptions = vec![')
    text=replace(text,'        Self {\n            native:', '''        subscriptions.push(cx.subscribe(&theme_editor, |this, _, event, cx| this.stage_theme_event(event, cx)));
        subscriptions.push(cx.observe(&theme_editor, |_, _, cx| cx.notify()));
        Self {
            theme_editor,
            theme_pending: None,
            theme_mode_pending: None,
            theme_inflight: false,
            native:''')
    text=replace(text,'            ui_font,\n            code_font,\n','')
    text=region(text,'    fn appearance_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {','    fn profile_settings(',lambda _:'')
    text=replace(text,'    pub(super) fn settings_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {',
        '    pub(super) fn settings_overlay(&self, cx: &mut Context<Self>) -> gpui::AnyElement {\n        if self.settings.theme_editor.read(cx).has_popup() { return self.settings.theme_editor.update(cx, |editor, cx| editor.overlay(cx)); }')
    text=replace(text,'        self.settings.popup = None;\n        self.settings.scroll.set_offset',
        '        self.settings.popup = None;\n        self.settings.theme_editor.update(cx, |editor, cx| editor.retire(cx));\n        self.settings.scroll.set_offset')
    text=text[:text.index('\nfn color_swatch(')]+'\n'
    sources[name]=text

    name='crates/synara-app/src/shell.rs'
    text=replace(sources[name],'            Update::SettingsSaved(settings, error) => {\n                self.settings.saving = false;',
        '            Update::SettingsSaved(settings, error) => {\n                let theme_write = std::mem::take(&mut self.settings.theme_inflight);\n                let theme_error = error.clone();\n                self.settings.saving = false;')
    text=replace(text,'            }\n            Update::ProfileActivity(activity) => {',
        '                self.finish_theme_write(theme_write, theme_error, cx);\n            }\n            Update::ProfileActivity(activity) => {')
    text=region(text,'    fn set_panel(', '\n}\nfn button(', lambda body: replace(body,'        self.settings.popup = None;',
        '        self.settings.popup = None;\n        self.settings.theme_editor.update(cx, |editor, cx| editor.retire(cx));'))
    sources[name]=text

    name='crates/synara-app/src/main.rs'
    text=replace(sources[name],'''                    .add_fonts(vec![std::borrow::Cow::Borrowed(include_bytes!(
                        "../assets/fonts/CalSans-Regular.ttf"
                    ))])''','''                    .add_fonts(vec![
                        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/CalSans-Regular.ttf") as &'static [u8]),
                        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Variable.ttf") as &'static [u8]),
                        std::borrow::Cow::Borrowed(include_bytes!("../assets/fonts/JetBrainsMono-Italic-Variable.ttf") as &'static [u8]),
                    ])''')
    text=replace(text,'Could not load the bundled Synara wordmark font','Could not load the bundled Synara fonts')
    text=replace(text,'            cx.set_reduce_motion(bootstrap.settings.appearance.reduced_motion);',
        '            ui::theme::register_font_names(cx.text_system().all_font_names());\n            cx.set_reduce_motion(bootstrap.settings.appearance.reduced_motion);')
    sources[name]=text
    return sources

def main():
    root=Path.cwd().resolve()
    if subprocess.check_output(['git','status','--porcelain'],text=True).strip():raise SystemExit('Refusing a dirty checkout')
    changes=prepare(root)
    for name,text in changes.items(): (root/name).write_text(text,encoding='utf-8',newline='')
    print('\n'.join(sorted(changes)))

if __name__=='__main__':main()
