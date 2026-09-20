//! Virtualized native choices with searchable platform text input and release activation.
use super::*;
mod models;
use crate::input::{EntryEvent, EntryMode, TextEntry};
use gpui::{
    Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable, KeyDownEvent, KeyUpEvent,
    ScrollStrategy, Subscription, UniformListScrollHandle, uniform_list,
};
pub use models::{ModelRow, ModelSource};

#[derive(Default)]
pub struct Choice {
    pub label: String,
    pub detail: String,
    pub selected: bool,
    pub icon: Option<Glyph>,
    pub unavailable: Option<String>,
}
#[derive(Clone, Copy)]
pub enum ChoiceEvent {
    Selected(usize),
    Dismissed,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ActivationKey {
    Enter,
    Space,
}
impl ActivationKey {
    fn from_key(key: &str) -> Option<Self> {
        match key {
            "enter" => Some(Self::Enter),
            "space" => Some(Self::Space),
            _ => None,
        }
    }
}
#[derive(Default)]
struct Navigation {
    active: usize,
    armed: Option<(usize, ActivationKey)>,
}
impl Navigation {
    fn move_to(&mut self, index: usize, count: usize) {
        self.active = index.min(count.saturating_sub(1));
        self.armed = None;
    }
    fn release(&mut self, key: ActivationKey) -> Option<usize> {
        if self.armed.is_some_and(|(_, armed)| armed == key) {
            self.armed
                .take()
                .map(|(index, _)| index)
                .filter(|index| *index == self.active)
        } else {
            None
        }
    }
}

/// Filtering never renumbers the controller's action array. Match every query
/// word against the combined label and description, preserving advertised order.
fn matching_choices(choices: &[Choice], query: &str) -> Vec<usize> {
    let query = query.to_lowercase();
    let words: Vec<_> = query.split_whitespace().collect();
    choices
        .iter()
        .enumerate()
        .filter(|(_, choice)| {
            if words.is_empty() {
                return true;
            }
            let text = format!("{} {}", choice.label, choice.detail).to_lowercase();
            words.iter().all(|word| text.contains(word))
        })
        .map(|(index, _)| index)
        .collect()
}

fn active_after_filter(visible: &[usize], previous: Option<usize>, choices: &[Choice]) -> usize {
    previous
        .and_then(|index| visible.iter().position(|candidate| *candidate == index))
        .or_else(|| visible.iter().position(|index| choices[*index].selected))
        .unwrap_or(0)
}

pub struct ChoiceMenu {
    models: Option<models::ModelState>,
    title: String,
    unavailable_reason: Option<String>,
    choices: Vec<Choice>,
    visible: Vec<usize>,
    query: String,
    search: Entity<TextEntry>,
    searchable: bool,
    _search_events: Subscription,
    focus: FocusHandle,
    navigation: Navigation,
    scroll: UniformListScrollHandle,
    blur: Vec<Subscription>,
    opened_at: std::time::Instant,
    add_bounds: Option<std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>>,
}
impl EventEmitter<ChoiceEvent> for ChoiceMenu {}
impl Focusable for ChoiceMenu {
    fn focus_handle(&self, cx: &gpui::App) -> FocusHandle {
        if self.searchable {
            self.search.read(cx).focus_handle(cx)
        } else {
            self.focus.clone()
        }
    }
}
impl ChoiceMenu {
    pub fn new(title: String, choices: Vec<Choice>, cx: &mut Context<Self>) -> Self {
        let active = choices
            .iter()
            .position(|choice| choice.selected)
            .unwrap_or(0);
        let scroll = UniformListScrollHandle::new();
        scroll.scroll_to_item(active, ScrollStrategy::Center);
        let search = cx.new(|cx| {
            TextEntry::new("Search choices...", EntryMode::SingleLine, 32., cx)
                .with_leading_icon(Glyph::Search)
        });
        let search_events = cx.subscribe(&search, |this, entry, event, cx| {
            if matches!(event, EntryEvent::Changed) {
                let query = entry.read(cx).text().to_owned();
                this.filter(query, cx);
            }
        });
        Self {
            models: None,
            title,
            unavailable_reason: None,
            visible: (0..choices.len()).collect(),
            searchable: choices.len() > 1,
            choices,
            query: String::new(),
            search,
            _search_events: search_events,
            focus: cx.focus_handle(),
            navigation: Navigation {
                active,
                armed: None,
            },
            scroll,
            blur: Vec::new(),
            opened_at: std::time::Instant::now(),
            add_bounds: None,
        }
    }
    pub fn add_layout(
        mut self,
        bounds: std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>,
    ) -> Self {
        self.add_bounds = Some(bounds);
        self.searchable = false;
        self
    }
    fn filter(&mut self, query: String, cx: &mut Context<Self>) {
        let previous = self.visible.get(self.navigation.active).copied();
        self.visible = matching_choices(&self.choices, &query);
        if let Some(models) = &self.models {
            self.visible.retain(|index| models.includes(*index));
        }
        self.query = query;
        let active = active_after_filter(&self.visible, previous, &self.choices);
        self.navigation.move_to(active, self.visible.len());
        self.scroll.scroll_to_item(active, ScrollStrategy::Center);
        self.unavailable_reason = None;
        cx.notify();
    }
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.unavailable_reason = self
            .choices
            .get(index)
            .and_then(|choice| choice.unavailable.clone());
        cx.notify();
        if self
            .choices
            .get(index)
            .is_some_and(|choice| choice.unavailable.is_none())
        {
            cx.emit(ChoiceEvent::Selected(index));
        }
    }
    fn search_focused(&self, window: &Window, cx: &gpui::App) -> bool {
        self.searchable && self.search.read(cx).focus_handle(cx).is_focused(window)
    }
    fn key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let search_focused = self.search_focused(window, cx);
        // Native IME confirmation/cancellation belongs to the text entry, not
        // the menu. In particular, committing preedit must never pick a model.
        let composing = search_focused
            && self.search.update(cx, |entry, cx| {
                entry.marked_text_range(window, cx).is_some()
            });
        if event.prefer_character_input || composing {
            self.navigation.armed = None;
            return;
        }
        if self.model_source_key(event, window, cx) {
            return;
        }
        if self.models.is_some()
            && !search_focused
            && !self.focus.is_focused(window)
            && event.keystroke.key != "escape"
        {
            return;
        }
        if self.models.is_some() && event.keystroke.key == "tab" {
            self.navigation.armed = None;
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt {
            self.navigation.armed = None;
            return;
        }
        let count = self.visible.len();
        let active = self.navigation.active;
        match event.keystroke.key.as_str() {
            "escape" => {
                self.navigation.armed = None;
                if self.searchable && !self.query.is_empty() {
                    self.search.update(cx, |entry, cx| {
                        entry.set_text(String::new(), cx);
                    });
                    self.filter(String::new(), cx);
                } else {
                    cx.emit(ChoiceEvent::Dismissed);
                }
            }
            "space" if search_focused => return,
            "enter" | "space" => {
                if count > 0 && !event.is_held {
                    self.navigation.armed =
                        ActivationKey::from_key(&event.keystroke.key).map(|key| (active, key));
                }
            }
            "up" | "down" | "tab" | "home" | "end" if count > 0 => {
                let next = match event.keystroke.key.as_str() {
                    "home" => 0,
                    "end" => count - 1,
                    "up" => (active + count - 1) % count,
                    "tab" if modifiers.shift => (active + count - 1) % count,
                    _ => (active + 1) % count,
                };
                self.navigation.move_to(next, count);
                self.scroll.scroll_to_item(next, ScrollStrategy::Center);
            }
            _ => return,
        }
        cx.notify();
        cx.stop_propagation();
    }
    fn key_up(&mut self, event: &KeyUpEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(key) = ActivationKey::from_key(&event.keystroke.key) else {
            return;
        };
        let search_focused = self.search_focused(window, cx);
        if self.models.is_some() && !search_focused && !self.focus.is_focused(window) {
            self.navigation.armed = None;
            return;
        }
        if key == ActivationKey::Space && search_focused {
            return;
        }
        let modifiers = event.keystroke.modifiers;
        if (self.focus.is_focused(window) || search_focused)
            && !modifiers.control
            && !modifiers.platform
            && !modifiers.alt
        {
            if let Some(index) = self
                .navigation
                .release(key)
                .and_then(|position| self.visible.get(position).copied())
            {
                self.select(index, cx);
            }
        } else {
            self.navigation.armed = None;
        }
        cx.stop_propagation();
    }
}
impl Render for ChoiceMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur.is_empty() {
            for focus in [self.focus.clone(), self.search.read(cx).focus_handle(cx)] {
                self.blur.push(cx.on_blur(&focus, window, |this, _, cx| {
                    this.navigation.armed = None;
                    cx.notify();
                }));
            }
        }
        let entity = cx.entity();
        let progress = if cx.reduce_motion() {
            1.0
        } else {
            super::motion::ease_out((self.opened_at.elapsed().as_secs_f32() / 0.15).min(1.0))
        };
        if progress < 1.0 {
            cx.on_next_frame(window, |_, _, cx| cx.notify());
        }
        // Filtering keeps this entity alive and does not restart its entrance.
        let scale = 0.98 + 0.02 * progress;
        let scaled = move |value| px(value * scale);
        tracing::debug!(target: "synara_ui_layout", surface = "session-menu", progress, scale, visible = self.visible.len(), total = self.choices.len(), "motion-frame");
        let compact = self.add_bounds.is_some();
        let width = self.add_bounds.as_ref().map_or(MENU_WIDTH, |bounds| {
            (f32::from(bounds.get().size.width) - 8.).max(240.)
        });
        let model_menu = self.models.is_some();
        let row_height = if compact {
            28.
        } else if model_menu {
            32.
        } else {
            MENU_ROW_HEIGHT
        };
        let height = (self.visible.len() as f32 * row_height).min(MENU_MAX_HEIGHT);
        div()
            .id("choice-menu")
            .relative()
            .role(gpui::Role::Menu)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .w(scaled(width))
            .p(scaled(4.))
            .rounded(scaled(if compact { 18. } else { 8. }))
            .border(scaled(1.))
            .border_color(rgb(palette().border))
            .bg(rgb(palette().overlay))
            .text_color(rgb(palette().text))
            .font_family(ui_font())
            .text_size(scaled(14.))
            .when(compact, |el| el.line_height(scaled(16.)))
            .opacity(progress)
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .capture_key_down(cx.listener(Self::key_down))
            .capture_key_up(cx.listener(Self::key_up))
            .child(super::layout_probe("session-menu"))
            .when(model_menu, |el| el.tab_group().child(self.model_tabs(cx)))
            .child(
                div()
                    .px(scaled(8.))
                    .py(scaled(if compact { 4. } else { 8. }))
                    .text_size(scaled(12.))
                    .text_color(rgb(palette().muted))
                    .child(self.model_title()),
            )
            .when(self.searchable, |el| {
                el.child(
                    div()
                        .id("choice-search")
                        .relative()
                        .aria_label("Search choices")
                        .px(scaled(4.))
                        .pb(scaled(4.))
                        .child(super::layout_probe("choice-search"))
                        .child(self.search.clone()),
                )
            })
            .when(self.visible.is_empty(), |el| {
                el.child(
                    div()
                        .p_3()
                        .text_size(px(12.))
                        .text_color(rgb(palette().muted))
                        .child("No matching choices. Press Escape to clear the search."),
                )
            })
            .when(!self.visible.is_empty(), |el| {
                el.child(
                    uniform_list("choices", self.visible.len(), move |range, _, cx| {
                        entity.update(cx, |this, cx| {
                            range
                                .filter_map(|position| {
                                    let index = *this.visible.get(position)?;
                                    let favorite = this.model_favorite_button(index, position, cx);
                                    let choice = &this.choices[index];
                                    Some(
                                        div()
                                            .id(("choice", index))
                                            .role(if compact {
                                                gpui::Role::MenuItem
                                            } else {
                                                gpui::Role::MenuItemRadio
                                            })
                                            .aria_label(format!(
                                                "{} {}",
                                                choice.label, choice.detail
                                            ))
                                            .aria_selected(choice.selected)
                                            .when_some(choice.unavailable.clone(), |el, reason| {
                                                el.aria_description(reason).cursor_default()
                                            })
                                            .when(position == this.navigation.active, |el| {
                                                el.aria_active_descendant()
                                            })
                                            .relative()
                                            .child(super::layout_probe_slot(
                                                "model-choice",
                                                position,
                                            ))
                                            .tab_stop(false)
                                            .h(scaled(row_height))
                                            .w_full()
                                            .px(scaled(8.))
                                            .flex()
                                            .items_center()
                                            .gap(scaled(8.))
                                            .rounded(scaled(6.))
                                            .bg(rgba(if position == this.navigation.active {
                                                (palette().hover << 8) | 255
                                            } else {
                                                0
                                            }))
                                            .cursor_pointer()
                                            .hover(|style| style.bg(rgb(palette().hover)))
                                            .child(if let Some(glyph) = choice.icon {
                                                super::icon(glyph)
                                                    .size(scaled(14.))
                                                    .into_any_element()
                                            } else {
                                                div()
                                                    .w(scaled(14.))
                                                    .children(choice.selected.then(|| {
                                                        super::icon(Glyph::Check).size(scaled(14.))
                                                    }))
                                                    .into_any_element()
                                            })
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w_0()
                                                    .flex()
                                                    .when(compact, |el| {
                                                        el.items_center().gap(scaled(6.))
                                                    })
                                                    .when(!compact, |el| el.flex_col())
                                                    .child(
                                                        div()
                                                            .text_size(scaled(if compact {
                                                                12.
                                                            } else {
                                                                13.
                                                            }))
                                                            .flex_shrink_0()
                                                            .text_ellipsis()
                                                            .child(choice.label.clone()),
                                                    )
                                                    .when(
                                                        !model_menu && !choice.detail.is_empty(),
                                                        |el| {
                                                            el.child(
                                                                div()
                                                                    .min_w_0()
                                                                    .text_size(scaled(11.))
                                                                    .text_color(
                                                                        rgb(palette().muted),
                                                                    )
                                                                    .text_ellipsis()
                                                                    .child(choice.detail.clone()),
                                                            )
                                                        },
                                                    ),
                                            )
                                            .children(
                                                (choice.selected && choice.icon.is_some()).then(
                                                    || super::icon(Glyph::Check).size(scaled(14.)),
                                                ),
                                            )
                                            .when_some(choice.unavailable.clone(), |el, reason| {
                                                el.tooltip(move |_, cx| {
                                                    cx.new(|_| {
                                                        super::Tooltip(reason.clone().into())
                                                    })
                                                    .into()
                                                })
                                            })
                                            .children(favorite)
                                            .on_click(cx.listener(move |this, _, window, cx| {
                                                this.navigation.armed = None;
                                                let focus = this.focus_handle(cx);
                                                window.focus(&focus, cx);
                                                this.select(index, cx);
                                                cx.stop_propagation();
                                            })),
                                    )
                                })
                                .collect::<Vec<_>>()
                        })
                    })
                    .h(scaled(height.max(row_height)))
                    .w_full()
                    .track_scroll(&self.scroll),
                )
            })
            .child(self.model_status())
            .children(self.unavailable_reason.clone().map(|reason| {
                div()
                    .p_2()
                    .text_size(px(12.))
                    .text_color(rgb(palette().muted))
                    .child(reason)
            }))
            .children((!compact).then(|| {
                div()
                    .px(scaled(8.))
                    .py(scaled(4.))
                    .text_size(scaled(10.))
                    .text_color(rgb(palette().muted))
                    .child("↑ ↓ to navigate · Enter to select · Esc to close")
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn activation_requires_one_matching_release() {
        let mut nav = Navigation::default();
        assert_eq!(nav.release(ActivationKey::Enter), None);
        nav.armed = Some((0, ActivationKey::Enter));
        assert_eq!(nav.release(ActivationKey::Enter), Some(0));
        assert_eq!(nav.release(ActivationKey::Enter), None);
    }
    #[test]
    fn a_different_activation_key_cannot_commit_a_held_press() {
        let mut nav = Navigation {
            active: 0,
            armed: Some((0, ActivationKey::Enter)),
        };
        assert_eq!(nav.release(ActivationKey::Space), None);
        assert_eq!(nav.release(ActivationKey::Enter), Some(0));
    }
    #[test]
    fn moving_or_retiring_a_choice_cancels_armed_activation() {
        let mut nav = Navigation {
            active: 0,
            armed: Some((0, ActivationKey::Enter)),
        };
        nav.move_to(1, 10000);
        assert_eq!(nav.release(ActivationKey::Enter), None);
        nav.armed = Some((1, ActivationKey::Enter));
        nav.move_to(9000, 2);
        assert_eq!(nav.active, 1);
        assert_eq!(nav.release(ActivationKey::Enter), None);
        nav.move_to(0, 0);
        assert_eq!(nav.release(ActivationKey::Enter), None);
    }
    fn choices() -> Vec<Choice> {
        vec![
            Choice {
                label: "Alpha".into(),
                detail: "Coding agent".into(),
                ..Default::default()
            },
            Choice {
                label: "Caffè fast".into(),
                detail: "Acme reasoning".into(),
                selected: true,
                ..Default::default()
            },
            Choice {
                label: "Alternate".into(),
                detail: "Acme reasoning".into(),
                unavailable: Some("Not connected".into()),
                ..Default::default()
            },
        ]
    }
    #[test]
    fn filtering_preserves_controller_indices_and_matches_all_words() {
        let choices = choices();
        assert_eq!(matching_choices(&choices, " acme  ALTERNATE "), vec![2]);
        assert_eq!(matching_choices(&choices, "CAFFÈ reasoning"), vec![1]);
        assert_eq!(matching_choices(&choices, "coding"), vec![0]);
        assert!(matching_choices(&choices, "fast alternate").is_empty());
        assert_eq!(matching_choices(&choices, " \t "), vec![0, 1, 2]);
        assert!(choices[2].unavailable.is_some());
    }
    #[test]
    fn filtering_keeps_active_identity_or_falls_back_to_applied_choice() {
        let choices = choices();
        assert_eq!(active_after_filter(&[1, 2], Some(2), &choices), 1);
        assert_eq!(active_after_filter(&[0, 1, 2], None, &choices), 1);
        assert_eq!(active_after_filter(&[2], Some(1), &choices), 0);
        assert_eq!(active_after_filter(&[], Some(1), &choices), 0);
    }
    #[test]
    fn changing_filter_cancels_an_armed_choice_even_at_the_same_row() {
        let mut nav = Navigation {
            active: 0,
            armed: Some((0, ActivationKey::Enter)),
        };
        nav.move_to(0, 1);
        assert_eq!(nav.release(ActivationKey::Enter), None);
    }
    #[test]
    fn large_catalog_filter_retains_the_original_action_identity() {
        let choices: Vec<_> = (0..10000)
            .map(|index| Choice {
                label: format!("Model {index}"),
                detail: "Provider".into(),
                ..Default::default()
            })
            .collect();
        assert_eq!(matching_choices(&choices, "provider 9999"), vec![9999]);
    }
}
