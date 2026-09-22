//! Native bounded slider with one pointer/keyboard event path and real values.
use super::*;
use gpui::{Bounds, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Pixels, Point};
use std::{cell::Cell, rc::Rc};

#[derive(Clone, Copy, Debug)]
pub struct SliderChanged(pub f32);

pub struct ValueSlider {
    label: &'static str,
    probe: &'static str,
    minimum: f32,
    maximum: f32,
    step: f32,
    value: f32,
    hue: bool,
    focus: FocusHandle,
    dragging: bool,
    bounds: Rc<Cell<Bounds<Pixels>>>,
}
impl EventEmitter<SliderChanged> for ValueSlider {}
impl Focusable for ValueSlider {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle { self.focus.clone() }
}
impl ValueSlider {
    pub fn new(label: &'static str, probe: &'static str, maximum: f32, value: f32, hue: bool, cx: &mut Context<Self>) -> Self {
        Self { label, probe, minimum: 0.0, maximum, step: 1.0, value: value.clamp(0.0, maximum), hue,
            focus: cx.focus_handle(), dragging: false, bounds: Rc::new(Cell::new(Bounds::default())) }
    }
    pub fn value(&self) -> f32 { self.value }
    pub fn set_value(&mut self, value: f32, cx: &mut Context<Self>) {
        let value = normalized(value, self.minimum, self.maximum, self.step);
        if self.value != value { self.value = value; cx.notify(); }
    }
    fn change(&mut self, value: f32, cx: &mut Context<Self>) {
        let value = normalized(value, self.minimum, self.maximum, self.step);
        if self.value != value {
            self.value = value;
            cx.emit(SliderChanged(value));
            cx.notify();
        }
    }
    fn pointer(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let bounds = self.bounds.get();
        let width = f32::from(bounds.size.width);
        if width > 0.0 {
            let fraction = (f32::from(point.x - bounds.origin.x) / width).clamp(0.0, 1.0);
            self.change(self.minimum + fraction * (self.maximum - self.minimum), cx);
        }
    }
}
fn normalized(value: f32, minimum: f32, maximum: f32, step: f32) -> f32 {
    if !value.is_finite() { return minimum; }
    ((value - minimum) / step).round().mul_add(step, minimum).clamp(minimum, maximum)
}
impl Render for ValueSlider {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = self.bounds.clone();
        let fraction = (self.value - self.minimum) / (self.maximum - self.minimum);
        let hue_colors = [0xff0000, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0xff00ff, 0xff0000];
        div().id(self.probe).role(gpui::Role::Slider).aria_label(self.label)
            .aria_value(format!("{} of {}", self.value as u32, self.maximum as u32))
            .track_focus(&self.focus).tab_index(0).w(px(176.)).h(px(20.)).relative()
            .cursor_pointer().rounded_md().border_1().border_color(gpui::rgba(0))
            .focus_visible(|style| style.border_color(rgb(palette().focus)))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.prefer_character_input || event.keystroke.modifiers.control || event.keystroke.modifiers.platform { return; }
                let value = match event.keystroke.key.as_str() {
                    "left" | "down" => this.value - this.step,
                    "right" | "up" => this.value + this.step,
                    "pageup" => this.value + 10.0 * this.step,
                    "pagedown" => this.value - 10.0 * this.step,
                    "home" => this.minimum,
                    "end" => this.maximum,
                    _ => return,
                };
                this.change(value, cx);
                cx.stop_propagation();
            }))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                this.dragging = true;
                window.focus(&this.focus, cx);
                this.pointer(event.position, cx);
                cx.stop_propagation();
            }))
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if this.dragging && event.dragging() { this.pointer(event.position, cx); }
            }))
            .on_mouse_up(MouseButton::Left, cx.listener(|this, _, _, _| this.dragging = false))
            .on_mouse_up_out(MouseButton::Left, cx.listener(|this, _, _, _| this.dragging = false))
            .child(canvas(move |actual, _, _| bounds.set(actual), |_, _, _, _| {}).absolute().size_full())
            .child(layout_probe(self.probe))
            .child(div().absolute().left_0().right_0().top(px(7.)).h(px(6.)).rounded_full().overflow_hidden()
                .when(!self.hue, |track| track.bg(rgb(palette().border)).child(div().h_full().w(gpui::relative(fraction)).bg(rgb(palette().text))))
                .when(self.hue, |track| track.flex().children(hue_colors.windows(2).map(|colors| {
                    div().flex_1().h_full().bg(gpui::linear_gradient(90., gpui::linear_color_stop(rgb(colors[0]), 0.), gpui::linear_color_stop(rgb(colors[1]), 1.)))
                }))))
            .child(div().absolute().left(gpui::relative(fraction)).top(px(3.)).ml(px(-6.)).size(px(12.)).rounded_full()
                .bg(rgb(if self.hue { super::color_picker::Hsv::new(self.value, 1., 1.).rgb() } else { palette().text }))
                .border_2().border_color(rgb(if self.hue { 0xffffff } else { palette().canvas })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pointer_keyboard_and_external_values_share_the_same_bounded_quantization() {
        for (value, expected) in [(-4.,0.), (0.4,0.), (0.5,1.), (13.6,14.), (100.5,100.)] {
            assert_eq!(normalized(value,0.,100.,1.), expected);
        }
        assert_eq!(normalized(f32::NAN,0.,100.,1.),0.);
        assert_eq!(normalized(f32::INFINITY,0.,100.,1.),0.);
        assert_eq!(normalized(361.,0.,360.,1.),360.);
    }
}
