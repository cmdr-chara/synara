//! Upstream `EditProjectDialog` + `ProjectAppearancePicker`: one place to edit a
//! project's local name and its look — an emoji, or a Central icon and color.
//! The picker renders inline in the dialog instead of a separate popover.
use super::*;
use crate::ui::{self, central_icon, palette};
use gpui::{EventEmitter, FocusHandle, KeyDownEvent};

pub(super) enum DialogEvent {
    Save {
        project: ProjectId,
        name: String,
        appearance: Option<ProjectAppearance>,
    },
    Dismiss,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum PickerTab {
    Emoji,
    Icons,
}

/// Upstream `matchesQuery`: every query word must prefix-match an option word.
fn matches_query(query: &str, fields: &[&str]) -> bool {
    if query.is_empty() {
        return true;
    }
    let words: Vec<String> = fields
        .join(" ")
        .to_lowercase()
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    query
        .to_lowercase()
        .split_whitespace()
        .all(|word| words.iter().any(|candidate| candidate.starts_with(word)))
}

pub(in crate::shell) struct EditProjectDialog {
    project: Project,
    favicon: Option<Arc<gpui::Image>>,
    name: Entity<TextEntry>,
    query: Entity<TextEntry>,
    tab: PickerTab,
    appearance: Option<ProjectAppearance>,
    /// Color swatch preview (upstream `color` state): tints the icon grid and
    /// lands on the project only once an icon is the choice.
    color: Option<ProjectColor>,
    focus: FocusHandle,
    needs_focus: bool,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<DialogEvent> for EditProjectDialog {}
impl EditProjectDialog {
    pub fn new(
        project: Project,
        ui: ProjectUi,
        favicon: Option<Arc<gpui::Image>>,
        cx: &mut Context<Self>,
    ) -> Self {
        // Upstream: the name field's placeholder is the folder name, and its
        // initial value is the local name.
        let name = cx.new(|cx| TextEntry::new(&project.name, EntryMode::SingleLine, 36., cx));
        if let Some(local) = &ui.name {
            name.update(cx, |entry, cx| entry.set_text(local.clone(), cx));
        }
        let query = cx.new(|cx| TextEntry::new("Search", EntryMode::SingleLine, 32., cx));
        let subscriptions = vec![
            cx.subscribe(&name, |this, _, event, cx| {
                if matches!(event, EntryEvent::Submit) {
                    this.save(cx);
                }
                cx.notify();
            }),
            cx.subscribe(&query, |_, _, _, cx| cx.notify()),
        ];
        let appearance = ui.appearance;
        let (tab, color) = match &appearance {
            Some(ProjectAppearance::Emoji { .. }) => (PickerTab::Emoji, None),
            Some(ProjectAppearance::Icon { color, .. }) => (
                PickerTab::Icons,
                color.as_ref().and_then(|color| color.color()),
            ),
            None => (PickerTab::Icons, None),
        };
        Self {
            project,
            favicon,
            name,
            query,
            tab,
            appearance,
            color,
            focus: cx.focus_handle(),
            needs_focus: true,
            _subscriptions: subscriptions,
        }
    }
    fn save(&mut self, cx: &mut Context<Self>) {
        // Upstream `save()`: the trimmed name; empty clears the local name.
        cx.emit(DialogEvent::Save {
            project: self.project.id,
            name: self.name.read(cx).text().trim().to_owned(),
            appearance: self.appearance.clone(),
        });
    }
    fn dismiss(&mut self, cx: &mut Context<Self>) {
        cx.emit(DialogEvent::Dismiss);
    }
    fn key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if event.prefer_character_input
            || self.name.read(cx).is_composing()
            || self.query.read(cx).is_composing()
        {
            return;
        }
        if event.keystroke.key == "escape" {
            self.dismiss(cx);
            cx.stop_propagation();
        }
    }
    fn pick_tab(&mut self, tab: PickerTab, cx: &mut Context<Self>) {
        self.tab = tab;
        self.query.update(cx, |entry, cx| entry.clear(cx));
        cx.notify();
    }
    fn pick_color(&mut self, color: Option<ProjectColor>, cx: &mut Context<Self>) {
        self.color = color;
        // Upstream `pickColor`: trying a color while an emoji is set previews
        // it but does not replace the emoji.
        if !matches!(self.appearance, Some(ProjectAppearance::Emoji { .. })) {
            let icon = match &self.appearance {
                Some(ProjectAppearance::Icon { icon, .. }) => icon.clone(),
                _ => DEFAULT_PROJECT_ICON.to_owned(),
            };
            self.appearance = Some(ProjectAppearance::Icon {
                icon,
                color: color.map(ProjectColorName::from),
            });
        }
        cx.notify();
    }
    fn pick_icon(&mut self, icon: &str, cx: &mut Context<Self>) {
        self.appearance = Some(ProjectAppearance::Icon {
            icon: icon.to_owned(),
            color: self.color.map(ProjectColorName::from),
        });
        cx.notify();
    }
    fn pick_emoji(&mut self, emoji: &str, cx: &mut Context<Self>) {
        self.appearance = Some(ProjectAppearance::Emoji {
            emoji: emoji.to_owned(),
        });
        cx.notify();
    }
}
impl gpui::Render for EditProjectDialog {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.needs_focus {
            window.focus(&self.name.read(cx).focus_handle(cx), cx);
            self.needs_focus = false;
        }
        let dark = palette().dark;
        let query = self.query.read(cx).text().trim().to_lowercase();
        let icons: Vec<_> = PROJECT_ICON_OPTIONS
            .iter()
            .filter(|option| matches_query(&query, &[option.label, option.keywords]))
            .collect();
        let emoji: Vec<SharedString> = {
            let mut matches: Vec<SharedString> = PROJECT_EMOJI_OPTIONS
                .iter()
                .filter(|option| option.emoji == query || matches_query(&query, &[option.keywords]))
                .map(|option| SharedString::from(option.emoji))
                .collect();
            // Any emoji typed or pasted into the search is offered too, so the
            // bundled list is a shortcut rather than a limit.
            if let Some(typed) = first_emoji(&query)
                && !matches.iter().any(|option| option.as_str() == typed)
            {
                matches.insert(0, SharedString::from(typed));
            }
            matches
        };
        let selected_icon = match &self.appearance {
            Some(ProjectAppearance::Emoji { .. }) => None,
            Some(ProjectAppearance::Icon { icon, .. }) => Some(icon.as_str()),
            None => Some(DEFAULT_PROJECT_ICON),
        };
        let selected_emoji = match &self.appearance {
            Some(ProjectAppearance::Emoji { emoji }) => Some(emoji.as_str()),
            _ => None,
        };
        let tint = self
            .color
            .map_or_else(|| rgb(palette().muted), |color| rgb(color.rgb(dark)));
        let cell = |id: SharedString, label: &str, selected: bool| {
            ui::button_shell(id, SharedString::from(label.to_owned()), selected)
                .size(px(32.))
                .p_0()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(10.))
        };
        let grid = |children: Vec<AnyElement>| div().grid().grid_cols(8).gap_1().children(children);
        let picker = div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        ui::button("project-tab-emoji", "Emoji", self.tab == PickerTab::Emoji)
                            .rounded_full()
                            .text_size(px(12.))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.pick_tab(PickerTab::Emoji, cx)),
                            ),
                    )
                    .child(
                        ui::button("project-tab-icons", "Icons", self.tab == PickerTab::Icons)
                            .rounded_full()
                            .text_size(px(12.))
                            .on_click(
                                cx.listener(|this, _, _, cx| this.pick_tab(PickerTab::Icons, cx)),
                            ),
                    ),
            )
            .child(self.query.clone())
            .when(self.tab == PickerTab::Icons, |el| {
                el.child(
                    div()
                        .id("project-color-group")
                        .role(gpui::Role::RadioGroup)
                        .aria_label("Color")
                        .flex()
                        .gap_1()
                        .children(
                            std::iter::once(None)
                                .chain(PROJECT_COLORS.iter().map(|color| Some(*color)))
                                .enumerate()
                                .map(|(index, option)| {
                                    let selected = self.color == option;
                                    cell(
                                        SharedString::from(format!("project-color-{index}")),
                                        option.map_or("Default color", |color| color.label()),
                                        selected,
                                    )
                                    .rounded_full()
                                    .child(div().size(px(20.)).rounded_full().bg(
                                        option.map_or_else(
                                            || rgb(palette().muted),
                                            |color| rgb(color.rgb(dark)),
                                        ),
                                    ))
                                    .on_click(cx.listener(
                                        move |this, _, _, cx| this.pick_color(option, cx),
                                    ))
                                }),
                        ),
                )
            })
            .child(
                div()
                    .id("project-appearance-scroll")
                    .max_h(px(176.))
                    .overflow_y_scroll()
                    .child(if self.tab == PickerTab::Icons {
                        if icons.is_empty() {
                            div()
                                .py_6()
                                .flex()
                                .justify_center()
                                .text_size(px(12.))
                                .text_color(rgb(palette().muted))
                                .child(format!("No icons match “{}”.", query))
                                .into_any_element()
                        } else {
                            grid(
                                icons
                                    .iter()
                                    .map(|option| {
                                        let name = option.name;
                                        cell(
                                            SharedString::from(format!("project-icon-{name}")),
                                            option.label,
                                            selected_icon == Some(name),
                                        )
                                        .child(central_icon(name).size(px(20.)).text_color(tint))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.pick_icon(name, cx)
                                        }))
                                        .into_any_element()
                                    })
                                    .collect(),
                            )
                            .into_any_element()
                        }
                    } else if emoji.is_empty() {
                        div()
                            .py_6()
                            .flex()
                            .justify_center()
                            .text_size(px(12.))
                            .text_color(rgb(palette().muted))
                            .child("No emoji match. Paste any emoji to use it.")
                            .into_any_element()
                    } else {
                        grid(
                            emoji
                                .iter()
                                .enumerate()
                                .map(|(index, option)| {
                                    let emoji = option.clone();
                                    cell(
                                        SharedString::from(format!("project-emoji-{index}")),
                                        option.as_str(),
                                        selected_emoji == Some(option.as_str()),
                                    )
                                    .child(div().text_size(px(19.)).child(emoji.clone()))
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.pick_emoji(&emoji, cx)
                                    }))
                                    .into_any_element()
                                })
                                .collect(),
                        )
                        .into_any_element()
                    }),
            );
        let preview = project_glyph(
            self.appearance.as_ref(),
            self.favicon.as_ref(),
            false,
            ProjectGlyphPresentation::Badge,
        );
        let modal = div()
            .id("project-edit-dialog")
            .role(gpui::Role::Dialog)
            .aria_label("Edit project")
            .track_focus(&self.focus)
            .tab_group()
            .tab_stop(true)
            .occlude()
            .relative()
            .w_full()
            .max_w(px(420.))
            .max_h(window.viewport_size().height - px(40.))
            .flex()
            .flex_col()
            .min_h_0()
            .rounded(px(16.))
            .border_1()
            .border_color(rgb(palette().border))
            .bg(rgb(palette().overlay))
            .shadow_lg()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_key_down(cx.listener(Self::key))
            .child(ui::layout_probe("project-edit-dialog"))
            .child(
                div()
                    .p_4()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().flex_1().text_size(px(16.)).child("Edit project")),
            )
            .child(
                div()
                    .px_4()
                    .pb_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette().border))
                            .child(
                                div()
                                    .w(px(40.))
                                    .h_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .border_r_1()
                                    .border_color(rgb(palette().border))
                                    .child(preview),
                            )
                            .child(div().flex_1().child(self.name.clone())),
                    )
                    .child(picker)
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(
                                ui::button("project-edit-cancel", "Cancel", false)
                                    .on_click(cx.listener(|this, _, _, cx| this.dismiss(cx))),
                            )
                            .child(
                                ui::button("project-edit-save", "Save", true)
                                    .relative()
                                    .child(ui::layout_probe("project-edit-save"))
                                    .on_click(cx.listener(|this, _, _, cx| this.save(cx))),
                            ),
                    ),
            );
        div()
            .absolute()
            .inset_0()
            .size_full()
            .p_4()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000088))
            .occlude()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.dismiss(cx);
                    cx.stop_propagation();
                }),
            )
            .child(modal)
    }
}
