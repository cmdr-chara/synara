//! Virtualized native choices with one focus owner and release-based activation.
use super::*;
use gpui::{
    EventEmitter, FocusHandle, Focusable, KeyDownEvent, KeyUpEvent, ScrollStrategy, Subscription,
    UniformListScrollHandle, uniform_list,
};

pub struct Choice {
    pub label: String,
    pub detail: String,
    pub selected: bool,
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

pub struct ChoiceMenu {
    title: String,
    choices: Vec<Choice>,
    focus: FocusHandle,
    navigation: Navigation,
    scroll: UniformListScrollHandle,
    blur: Option<Subscription>,
}
impl EventEmitter<ChoiceEvent> for ChoiceMenu {}
impl Focusable for ChoiceMenu {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
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
        Self {
            title,
            choices,
            focus: cx.focus_handle(),
            navigation: Navigation {
                active,
                armed: None,
            },
            scroll,
            blur: None,
        }
    }
    fn key_down(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.platform || modifiers.alt {
            return;
        }
        let count = self.choices.len();
        let active = self.navigation.active;
        match event.keystroke.key.as_str() {
            "escape" => {
                self.navigation.armed = None;
                cx.emit(ChoiceEvent::Dismissed);
            }
            "enter" | "space" if count > 0 => {
                if !event.is_held {
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
        let modifiers = event.keystroke.modifiers;
        if self.focus.is_focused(window)
            && !modifiers.control
            && !modifiers.platform
            && !modifiers.alt
        {
            if let Some(index) = self.navigation.release(key) {
                cx.emit(ChoiceEvent::Selected(index));
            }
        } else {
            self.navigation.armed = None;
        }
        cx.stop_propagation();
    }
}
impl Render for ChoiceMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blur.is_none() {
            self.blur = Some(cx.on_blur(&self.focus.clone(), window, |this, _, cx| {
                this.navigation.armed = None;
                cx.notify();
            }));
        }
        let entity = cx.entity();
        let height = (self.choices.len() as f32 * MENU_ROW_HEIGHT).min(MENU_MAX_HEIGHT);
        div()
            .id("choice-menu")
            .relative()
            .role(gpui::Role::Menu)
            .aria_label(self.title.clone())
            .track_focus(&self.focus)
            .tab_index(0)
            .w(px(MENU_WIDTH))
            .p_1()
            .rounded_lg()
            .border_1()
            .border_color(rgb(DARK.border))
            .bg(rgb(DARK.overlay))
            .text_color(rgb(DARK.text))
            .font_family(UI_FONT)
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .child(super::layout_probe("session-menu"))
            .child(
                div()
                    .px_2()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(DARK.muted))
                    .child(self.title.clone()),
            )
            .child(
                uniform_list("choices", self.choices.len(), move |range, _, cx| {
                    entity.update(cx, |this, cx| {
                        range
                            .map(|index| {
                                let choice = &this.choices[index];
                                div()
                                    .id(("choice", index))
                                    .role(gpui::Role::MenuItemRadio)
                                    .aria_label(format!("{} {}", choice.label, choice.detail))
                                    .aria_selected(choice.selected)
                                    .when(index == this.navigation.active, |el| {
                                        el.aria_active_descendant()
                                    })
                                    .tab_stop(false)
                                    .h(px(MENU_ROW_HEIGHT))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .rounded_md()
                                    .bg(rgba(if index == this.navigation.active {
                                        (DARK.hover << 8) | 255
                                    } else {
                                        0
                                    }))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(DARK.hover)))
                                    .child(
                                        div()
                                            .w(px(14.))
                                            .text_color(rgb(DARK.text))
                                            .child(if choice.selected { "✓" } else { "" }),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .flex_col()
                                            .child(
                                                div()
                                                    .text_size(px(13.))
                                                    .text_ellipsis()
                                                    .child(choice.label.clone()),
                                            )
                                            .when(!choice.detail.is_empty(), |el| {
                                                el.child(
                                                    div()
                                                        .text_size(px(11.))
                                                        .text_color(rgb(DARK.muted))
                                                        .text_ellipsis()
                                                        .child(choice.detail.clone()),
                                                )
                                            }),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.navigation.armed = None;
                                        window.focus(&this.focus, cx);
                                        cx.emit(ChoiceEvent::Selected(index));
                                        cx.stop_propagation();
                                    }))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .h(px(height.max(MENU_ROW_HEIGHT)))
                .w_full()
                .track_scroll(&self.scroll),
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(10.))
                    .text_color(rgb(DARK.muted))
                    .child("↑ ↓ to navigate · Enter to select · Esc to close"),
            )
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
}
