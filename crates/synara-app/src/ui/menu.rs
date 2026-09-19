//! Virtualized native choices with one focus owner and release-based activation.
use super::*;
use gpui::{
    EventEmitter, FocusHandle, Focusable, KeyDownEvent, KeyUpEvent, ScrollStrategy, Subscription,
    UniformListScrollHandle, uniform_list,
};

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

pub struct ChoiceMenu {
    title: String,
    unavailable_reason: Option<String>,
    choices: Vec<Choice>,
    focus: FocusHandle,
    navigation: Navigation,
    scroll: UniformListScrollHandle,
    blur: Option<Subscription>,
    opened_at: std::time::Instant,
    add_bounds: Option<std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>>,
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
            unavailable_reason: None,
            choices,
            focus: cx.focus_handle(),
            navigation: Navigation {
                active,
                armed: None,
            },
            scroll,
            blur: None,
            opened_at: std::time::Instant::now(),
            add_bounds: None,
        }
    }
    pub fn add_layout(
        mut self,
        bounds: std::rc::Rc<std::cell::Cell<gpui::Bounds<gpui::Pixels>>>,
    ) -> Self {
        self.add_bounds = Some(bounds);
        self
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
        if self.blur.is_none() {
            self.blur = Some(cx.on_blur(&self.focus.clone(), window, |this, _, cx| {
                this.navigation.armed = None;
                cx.notify();
            }));
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
        // Scale the finite menu's actual layout and text together; GPUI's generic
        // elements have no CSS transform. Hit testing follows the painted bounds.
        let scale = 0.98 + 0.02 * progress;
        let scaled = move |value| px(value * scale);
        tracing::debug!(target: "synara_ui_layout", surface = "session-menu", progress, scale, "motion-frame");
        let compact = self.add_bounds.is_some();
        let width = self.add_bounds.as_ref().map_or(MENU_WIDTH, |bounds| {
            (f32::from(bounds.get().size.width) - 8.).max(240.)
        });
        let row_height = if compact { 28. } else { MENU_ROW_HEIGHT };
        let height = (self.choices.len() as f32 * row_height).min(MENU_MAX_HEIGHT);
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
            .font_family(UI_FONT)
            .text_size(scaled(14.))
            .when(compact, |el| el.line_height(scaled(16.)))
            .opacity(progress)
            .occlude()
            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_key_down(cx.listener(Self::key_down))
            .on_key_up(cx.listener(Self::key_up))
            .child(super::layout_probe("session-menu"))
            .child(
                div()
                    .px(scaled(8.))
                    .py(scaled(if compact { 4. } else { 8. }))
                    .text_size(scaled(12.))
                    .text_color(rgb(palette().muted))
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
                                    .role(if compact {
                                        gpui::Role::MenuItem
                                    } else {
                                        gpui::Role::MenuItemRadio
                                    })
                                    .aria_label(format!("{} {}", choice.label, choice.detail))
                                    .aria_selected(choice.selected)
                                    .when_some(choice.unavailable.clone(), |el, reason| {
                                        el.aria_description(reason).cursor_default()
                                    })
                                    .when(index == this.navigation.active, |el| {
                                        el.aria_active_descendant()
                                    })
                                    .tab_stop(false)
                                    .h(scaled(row_height))
                                    .w_full()
                                    .px(scaled(8.))
                                    .flex()
                                    .items_center()
                                    .gap(scaled(8.))
                                    .rounded(scaled(6.))
                                    .bg(rgba(if index == this.navigation.active {
                                        (palette().hover << 8) | 255
                                    } else {
                                        0
                                    }))
                                    .cursor_pointer()
                                    .hover(|style| style.bg(rgb(palette().hover)))
                                    .child(if let Some(glyph) = choice.icon {
                                        super::icon(glyph).size(scaled(14.)).into_any_element()
                                    } else {
                                        div()
                                            .w(scaled(14.))
                                            .children(choice.selected.then(|| {
                                                super::icon(super::Glyph::Check).size(scaled(14.))
                                            }))
                                            .into_any_element()
                                    })
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .flex()
                                            .when(compact, |el| el.items_center().gap(scaled(6.)))
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
                                            .when(!choice.detail.is_empty(), |el| {
                                                el.child(
                                                    div()
                                                        .min_w_0()
                                                        .text_size(scaled(11.))
                                                        .text_color(rgb(palette().muted))
                                                        .text_ellipsis()
                                                        .child(choice.detail.clone()),
                                                )
                                            }),
                                    )
                                    .children((choice.selected && choice.icon.is_some()).then(
                                        || super::icon(super::Glyph::Check).size(scaled(14.)),
                                    ))
                                    .when_some(choice.unavailable.clone(), |el, reason| {
                                        el.tooltip(move |_, cx| {
                                            cx.new(|_| super::Tooltip(reason.clone().into())).into()
                                        })
                                    })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.navigation.armed = None;
                                        window.focus(&this.focus, cx);
                                        this.select(index, cx);
                                        cx.stop_propagation();
                                    }))
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .h(scaled(height.max(row_height)))
                .w_full()
                .track_scroll(&self.scroll),
            )
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
}
