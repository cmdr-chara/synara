#!/usr/bin/env python3
"""One-shot, fail-closed application of the reviewed Electron settings parity delta.

This is a development bootstrap for an unavailable local executor, not an app
runtime, installer, updater, or general remote command interface. Every existing
input is pinned by its Git blob hash and all edits are prepared before any write.
"""
from __future__ import annotations

import hashlib
from pathlib import Path
import subprocess

EXPECTED = {
    'crates/synara-app/src/ui.rs': '7cf26537bff30a3878533e5aa2bc42d709a012b2',
    'crates/synara-app/src/ui/personalization.rs': 'dae80fb3d19d2a07e968d121825d2f5255820b04',
    'crates/synara-workspace/src/settings.rs': '84507affed64c134296ccdef797f4b95628ed5cf',
    'crates/synara-workspace/src/settings/personalization.rs': '8fde6d1f7191ea104d97d863ed4646ca43d99aea',
    'crates/synara-app/src/shell/settings.rs': '2b21e2c8832f229cf7b53a7f3c508f17d5af61cd',
    'crates/synara-app/src/shell/appsnap.rs': '48d7a6413d45438197362d695c4b80b5426e0038',
    'crates/synara-app/src/shell/review.rs': '86ca52e9b51770230177095608eac0d725141b15',
    'crates/synara-app/src/shell/review/repository.rs': 'f95cae0dd963107c05cdcf2bd38614bf630b0ad5',
    'crates/synara-app/src/shell/review/repository/view.rs': 'ae4d9d4c3b0f46d699ab87a6cff8904058da4e85',
    'crates/synara-app/src/shell/settings/desktop.rs': 'f79a2588d8770f70f04d5d0b9f6e8c56b62804a2',
}


def git_blob(data: bytes) -> str:
    return hashlib.sha1(f'blob {len(data)}\0'.encode() + data).hexdigest()


def replace(text: str, old: str, new: str, count: int = 1) -> str:
    found = text.count(old)
    if found != count:
        raise ValueError(f'Expected {count} exact matches, found {found}: {old[:100]!r}')
    return text.replace(old, new)


def region(text: str, start: str, end: str, transform) -> str:
    if text.count(start) != 1 or text.count(end) != 1:
        raise ValueError('Ambiguous source region')
    left = text.index(start)
    right = text.index(end, left)
    return text[:left] + transform(text[left:right]) + text[right:]


def prepare(root: Path) -> dict[str, str]:
    sources = {}
    for name, expected in EXPECTED.items():
        path = root / name
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(root):
            raise ValueError(f'Unsafe or missing source: {name}')
        data = path.read_bytes()
        if git_blob(data) != expected:
            raise ValueError(f'Source changed; re-review before applying: {name}')
        sources[name] = data.decode('utf-8')

    name = 'crates/synara-app/src/ui.rs'
    text = replace(sources[name], 'mod personalization;\n', 'mod personalization;\npub mod metrics;\n')
    text = replace(text, 'surface, terminal_font_size, ui_font_size,',
                   'density_metrics, settings_row_padding, surface, terminal_font_size, ui_font_size,')
    text = replace(text, 'pub const ROW_HEIGHT: f32 = 30.0;', 'pub const ROW_HEIGHT: f32 = 28.0;')
    text = replace(text, '.text_size(px(ui_font_size() + 1.))', '.text_size(px(ui_font_size()))', 2)
    text = replace(text, '.text_sm()', '.text_size(px(ui_font_size()))')
    sources[name] = text

    name = 'crates/synara-app/src/ui/personalization.rs'
    text = replace(sources[name], 'std::cell::Cell::new((14., 13.))',
                   'std::cell::Cell::new((super::metrics::DEFAULT_UI_FONT_SIZE, super::metrics::DEFAULT_CODE_FONT_SIZE))')
    text = region(text, 'pub fn row_height() -> f32 {', '\npub fn motion_multiplier()', lambda _: '''pub fn density_metrics() -> super::metrics::DensityMetrics {
    STYLE.with(|style| super::metrics::DensityMetrics::new(style.borrow().density))
}
pub fn row_height() -> f32 {
    density_metrics().row_height
}
pub fn settings_row_padding() -> f32 {
    density_metrics().settings_row_padding_y
}
''')
    text = replace(text, 'Colorway, DensityPreference, MotionPreference, Personalization, SurfaceMaterial,',
                   'Colorway, MotionPreference, Personalization, SurfaceMaterial,')
    sources[name] = text

    name = 'crates/synara-workspace/src/settings.rs'
    text = replace(sources[name], 'ui_size: 14.0,', 'ui_size: 13.0,')
    text = replace(text, 'code_size: 13.0,', 'code_size: 12.0,')
    sources[name] = text
    name = 'crates/synara-workspace/src/settings/personalization.rs'
    sources[name] = replace(sources[name], 'terminal_font_size: 14,', 'terminal_font_size: 12,')

    name = 'crates/synara-app/src/shell/settings.rs'
    text = replace(sources[name], 'mod chat;\n', 'mod chat;\nmod desktop;\nmod navigation;\nuse navigation::{SECTIONS, primary_section};\n')
    text = replace(text, '    AppSnap,\n', '    AppSnap,\n    Computer,\n')
    start = text.index('const SECTIONS: &[SectionInfo] = &[\n')
    end = text.index('\n];', start) + len('\n];')
    text = text[:start] + text[end:]
    text = replace(text, '.rounded_2xl()\n        .border_1()\n        .border_color(rgb(palette().border))\n        .overflow_hidden()',
                   '.rounded_xl()\n        .border_1()\n        .border_color(rgb(palette().border))\n        .overflow_hidden()')
    text = region(text, "fn heading(label: &'static str)", '\nfn row(', lambda body:
        replace(replace(body, '.mt_7()\n        .mb_3()', '.mt_4()\n        .mb(px(6.))'),
                '.text_color(rgb(palette().muted))', '.text_size(px(ui::metrics::Typography::from_base(ui::ui_font_size()).small))\n        .text_color(rgb(palette().muted))'))
    def update_row(body):
        body = replace(body, '.py_3()', '.py(px(ui::settings_row_padding()))')
        body = replace(body, '.gap_5()', '.gap(px(10.))')
        body = replace(body, '.gap_1()', '.gap(px(2.))')
        body = replace(body, '.child(title.into())', '.child(div().font_weight(gpui::FontWeight::MEDIUM).child(title.into()))')
        return replace(body, '.line_height(px(22.))', '.line_height(px(ui::ui_font_size() * 1.5))')
    text = region(text, 'fn row(', '\nfn empty(', update_row)
    text = replace(text, '        self.settings.section = section;\n',
                   '        self.settings.section = section;\n        if section == Section::Worktrees { self.prepare_worktree_settings(cx); }\n')
    text = replace(text, 'info.group == group && settings_match(info, &query)',
                   'info.group == group && settings_match(info, &query)\n                                            && (primary_section(info.section) || !query.is_empty() || self.settings.section == info.section)')
    text = region(text, '    pub(super) fn settings_sidebar(', '\n    fn choice_button(', lambda body:
        replace(replace(body, '.h(px(30.))', '.h(px(ui::row_height()))'),
                '.child(ui::layout_probe(info.id))',
                '.child(ui::layout_probe(info.id))\n                                            .children((section == Section::Computer).then(|| div().px(px(6.)).rounded_full().border_1().border_color(rgb(palette().border)).text_size(px(10.)).text_color(rgb(palette().muted)).child("Beta")))'))
    text = replace(text, 'Section::AppSnap => empty("AppSnap is not available yet", "Capturing another app’s window has not been ported to the native app. You can attach project files from the composer’s Add menu.").into_any_element(),',
                   'Section::AppSnap => self.appsnap_settings(cx),\n            Section::Computer => self.computer_settings(cx),')
    text = replace(text, 'Section::Worktrees => empty("Managed worktrees are not available yet", "Open an existing worktree as a project to use it. Creating and cleaning up managed worktrees from this page has not been ported yet.").into_any_element(),',
                   'Section::Worktrees => self.worktree_settings_view(cx),')
    text = replace(text, 'Section::System => self.system_settings(cx),',
                   'Section::System => div().child(self.system_settings(cx)).child(self.native_extensions_settings(cx)).into_any_element(),')
    text = region(text, '    pub(super) fn settings_panel(', '\n    fn general_settings(', lambda body:
        replace(body, '.text_size(px(15.))', '.text_size(px(ui::ui_font_size()))'))
    sources[name] = text

    name = 'crates/synara-app/src/shell/appsnap.rs'
    sources[name] = replace(sources[name], 'impl Shell {\n', '''impl Shell {
    pub(super) fn open_appsnap_from_settings(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_none() || self.close != CloseState::Open {
            return;
        }
        self.set_panel(Panel::Conversation, cx);
        self.appsnap.retire();
        self.appsnap.open = true;
        cx.notify();
    }
''')

    name = 'crates/synara-app/src/shell/review.rs'
    text = replace(sources[name], 'impl Shell {\n', '''impl Shell {
    pub(super) fn prepare_worktree_settings(&mut self, cx: &mut Context<Self>) {
        self.open_repository(cx);
        if let Some(scope) = self.review_scope()
            && let Some(panel) = self.review.repositories.get(&scope).cloned()
        {
            panel.update(cx, |panel, cx| panel.show_worktrees(cx));
        }
    }
    pub(super) fn worktree_settings_view(&self, _: &mut Context<Self>) -> gpui::AnyElement {
        let scope = self.review_scope();
        let panel = scope.as_ref().and_then(|scope| self.review.repositories.get(scope));
        match panel {
            Some(panel) => div().w_full().flex().flex_col().gap_3()
                .child("Worktrees in the selected repository. Existing execution and removal confirmations still apply.")
                .child(div().h(px(520.)).min_h(px(300.)).w_full().border_1().border_color(rgb(palette().border)).rounded_xl().overflow_hidden().child(panel.clone()))
                .into_any_element(),
            None => div().p_6().text_color(rgb(palette().muted))
                .child("Select a project before managing its worktrees. This page does not create a workspace or start a tool implicitly.")
                .into_any_element(),
        }
    }
''')
    text = replace(text, '        self.review.repository_open = true;',
                   '        if let Some(panel) = self.review.repositories.get(&scope).cloned() {\n            panel.update(cx, |panel, cx| panel.show_repository_tabs(cx));\n        }\n        self.review.repository_open = true;')
    # The map insertion must keep the scope available for the presentation update.
    text = replace(text, 'self.review.repositories.entry(scope).or_insert_with',
                   'self.review.repositories.entry(scope.clone()).or_insert_with')
    sources[name] = text

    name = 'crates/synara-app/src/shell/review/repository.rs'
    text = replace(sources[name], '    view: View,\n', '    view: View,\n    worktrees_only: bool,\n')
    text = replace(text, '            view: View::Branches,\n', '            view: View::Branches,\n            worktrees_only: false,\n')
    text = replace(text, 'impl RepositoryPanel {\n', '''impl RepositoryPanel {
    pub(super) fn show_worktrees(&mut self, cx: &mut Context<Self>) {
        if self.form.is_none() {
            self.view = View::Worktrees;
            self.worktrees_only = true;
            self.query.update(cx, |query, cx| query.clear(cx));
            cx.notify();
        }
    }
    pub(super) fn show_repository_tabs(&mut self, cx: &mut Context<Self>) {
        self.worktrees_only = false;
        cx.notify();
    }
''')
    sources[name] = text
    name = 'crates/synara-app/src/shell/review/repository/view.rs'
    sources[name] = replace(sources[name], '.map(|(index, view)| {',
        '.filter(|(_, view)| !self.worktrees_only || *view == View::Worktrees)\n                    .map(|(index, view)| {')
    name = 'crates/synara-app/src/shell/settings/desktop.rs'
    sources[name] = replace(sources[name], '("native-settings-extension", item.id)',
                           '("native-settings-extension", section as usize)')
    return sources


def main() -> None:
    root = Path.cwd().resolve()
    if subprocess.check_output(['git', 'status', '--porcelain'], text=True).strip():
        raise SystemExit('Refusing to overwrite a dirty checkout')
    sources = prepare(root)
    for name, content in sources.items():
        (root / name).write_text(content, encoding='utf-8', newline='')
    print('Applied reviewed source delta to:')
    print('\n'.join(sorted(sources)))


if __name__ == '__main__':
    main()
