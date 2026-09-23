//! Native HSV surface and hexadecimal entry. No bitmap or web renderer.
//! Color commits use the reference editor's 220 ms idle delay; previews are local.
use super::*;
use crate::input::{EntryEvent, EntryMode, TextEntry};
use super::slider::{SliderChanged, ValueSlider};
use gpui::{Bounds, Entity, EventEmitter, FocusHandle, Focusable, MouseButton, Pixels, Point, Subscription};
use std::{cell::Cell, rc::Rc, time::Duration};
use synara_workspace::ThemeHex;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsv { pub hue: f32, pub saturation: f32, pub value: f32 }
impl Hsv {
    pub fn new(hue: f32, saturation: f32, value: f32) -> Self {
        Self { hue: if hue.is_finite() { hue.clamp(0.,360.) } else { 0. },
            saturation: if saturation.is_finite() { saturation.clamp(0.,1.) } else { 0. },
            value: if value.is_finite() { value.clamp(0.,1.) } else { 0. } }
    }
    pub fn from_rgb(rgb: u32) -> Self {
        let r = ((rgb >> 16) & 255) as f32 / 255.;
        let g = ((rgb >> 8) & 255) as f32 / 255.;
        let b = (rgb & 255) as f32 / 255.;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let difference = max - min;
        let hue = if difference <= f32::EPSILON { 0. }
            else if max == r { 60. * ((g - b) / difference).rem_euclid(6.) }
            else if max == g { 60. * ((b - r) / difference + 2.) }
            else { 60. * ((r - g) / difference + 4.) };
        Self::new(hue, if max > 0. { difference / max } else { 0. }, max)
    }
    pub fn rgb(self) -> u32 {
        let value = Self::new(self.hue,self.saturation,self.value);
        let h = value.hue.rem_euclid(360.) / 60.;
        let c = value.value * value.saturation;
        let x = c * (1. - (h.rem_euclid(2.) - 1.).abs());
        let m = value.value - c;
        let (r,g,b) = match h as u32 { 0 => (c,x,0.), 1 => (x,c,0.), 2 => (0.,c,x), 3 => (0.,x,c), 4 => (x,0.,c), _ => (c,0.,x) };
        let component = |v: f32| ((v + m) * 255.).round().clamp(0.,255.) as u32;
        (component(r) << 16) | (component(g) << 8) | component(b)
    }
    pub fn position(&mut self, x: f32, y: f32) {
        self.saturation = x.clamp(0.,1.);
        self.value = 1. - y.clamp(0.,1.);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ColorEvent { Preview(ThemeHex), Commit(ThemeHex) }

pub struct ColorPicker {
    hsv: Hsv,
    committed: u32,
    pending: Option<u32>,
    generation: u64,
    hex: Entity<TextEntry>,
    hue: Entity<ValueSlider>,
    focus: FocusHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    dragging: bool,
    _subscriptions: Vec<Subscription>,
}
impl EventEmitter<ColorEvent> for ColorPicker {}
impl Focusable for ColorPicker {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle { self.focus.clone() }
}
impl ColorPicker {
    pub fn new(color: ThemeHex, cx: &mut Context<Self>) -> Self {
        let hsv = Hsv::from_rgb(color.value());
        let hex = cx.new(|cx| {
            let mut entry = TextEntry::new("#RRGGBB", EntryMode::SingleLine, 32., cx);
            entry.set_text(color.to_string().to_uppercase(), cx);
            entry
        });
        let hue = cx.new(|cx| ValueSlider::new("Color hue", "theme-color-hue", 360., hsv.hue, true, cx));
        let subscriptions = vec![
            cx.subscribe(&hex, |this, input, event, cx| {
                if matches!(event, EntryEvent::Changed | EntryEvent::Submit)
                    && let Ok(color) = ThemeHex::parse(input.read(cx).text())
                {
                    this.hsv = Hsv::from_rgb(color.value());
                    this.hue.update(cx, |slider,cx| slider.set_value(this.hsv.hue,cx));
                    this.preview(cx);
                    if matches!(event, EntryEvent::Submit) { this.flush(cx); }
                }
            }),
            cx.subscribe(&hue, |this, _, SliderChanged(hue), cx| {
                this.hsv.hue = *hue;
                this.sync_hex(cx);
                this.preview(cx);
            }),
        ];
        Self { hsv, committed: color.value(), pending: None, generation: 0, hex, hue,
            focus: cx.focus_handle(), bounds: Rc::new(Cell::new(Bounds::default())), dragging: false, _subscriptions: subscriptions }
    }
    fn sync_hex(&mut self, cx: &mut Context<Self>) {
        let text = format!("#{:06X}", self.hsv.rgb());
        self.hex.update(cx, |entry,cx| entry.set_text(text,cx));
    }
    fn preview(&mut self, cx: &mut Context<Self>) {
        let rgb = self.hsv.rgb();
        self.pending = (rgb != self.committed).then_some(rgb);
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        cx.emit(ColorEvent::Preview(ThemeHex::rgb(rgb)));
        cx.notify();
        let timer = cx.background_executor().timer(Duration::from_millis(220));
        cx.spawn(async move |this,cx| {
            timer.await;
            let _ = this.update(cx, |this,cx| {
                if this.generation == generation { this.flush(cx); }
            });
        }).detach();
    }
    /// Returning the last valid value also lets the owner flush before dropping
    /// a popup subscription. Invalid partially typed hex never becomes a theme.
    pub fn flush(&mut self, cx: &mut Context<Self>) -> Option<ThemeHex> {
        self.generation = self.generation.wrapping_add(1);
        let rgb = self.pending.take()?;
        self.committed = rgb;
        let color = ThemeHex::rgb(rgb);
        cx.emit(ColorEvent::Commit(color));
        Some(color)
    }
    fn pointer(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let bounds = self.bounds.get();
        let width = f32::from(bounds.size.width);
        let height = f32::from(bounds.size.height);
        if width > 0. && height > 0. {
            self.hsv.position(f32::from(point.x - bounds.origin.x)/width, f32::from(point.y - bounds.origin.y)/height);
            self.sync_hex(cx);
            self.preview(cx);
        }
    }
}
impl Render for ColorPicker {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = self.bounds.clone();
        let hsv = self.hsv;
        div().w(px(224.)).p_3().flex().flex_col().gap_3()
            .child(div().id("theme-color-saturation").role(gpui::Role::Slider)
                .aria_label("Color saturation and brightness").aria_value(format!("Saturation {}%, brightness {}%", (hsv.saturation*100.).round(), (hsv.value*100.).round()))
                .track_focus(&self.focus).tab_index(0).w_full().h(px(144.)).relative().rounded_lg()
                .bg(rgb(Hsv::new(hsv.hue,1.,1.).rgb())).border_1().border_color(rgb(palette().border))
                .focus_visible(|style| style.border_color(rgb(palette().focus)))
                .cursor_pointer()
                .on_key_down(cx.listener(|this,event: &gpui::KeyDownEvent,_,cx| {
                    if event.prefer_character_input { return; }
                    let delta = if event.keystroke.modifiers.shift { 0.1 } else { 0.01 };
                    match event.keystroke.key.as_str() {
                        "left" => this.hsv.saturation = (this.hsv.saturation-delta).max(0.),
                        "right" => this.hsv.saturation = (this.hsv.saturation+delta).min(1.),
                        "up" => this.hsv.value = (this.hsv.value+delta).min(1.),
                        "down" => this.hsv.value = (this.hsv.value-delta).max(0.),
                        "home" => this.hsv.saturation = 0.,
                        "end" => this.hsv.saturation = 1.,
                        _ => return,
                    }
                    this.sync_hex(cx); this.preview(cx); cx.stop_propagation();
                }))
                .on_mouse_down(MouseButton::Left,cx.listener(|this,event: &gpui::MouseDownEvent,window,cx| {
                    window.focus(&this.focus,cx); this.dragging=true; this.pointer(event.position,cx); cx.stop_propagation();
                }))
                .on_mouse_move(cx.listener(|this,event: &gpui::MouseMoveEvent,_,cx| {
                    if this.dragging && event.dragging() { this.pointer(event.position,cx); }
                }))
                .on_mouse_up(MouseButton::Left,cx.listener(|this,_,_,_| this.dragging=false))
                .on_mouse_up_out(MouseButton::Left,cx.listener(|this,_,_,_| this.dragging=false))
                .child(canvas(move|actual,_,_|bounds.set(actual),|_,_,_,_|{}).absolute().size_full())
                .child(div().absolute().size_full().rounded_lg().bg(gpui::linear_gradient(90.,gpui::linear_color_stop(rgb(0xffffff),0.),gpui::linear_color_stop(gpui::rgba(0xffffff00),1.))))
                .child(div().absolute().size_full().rounded_lg().bg(gpui::linear_gradient(180.,gpui::linear_color_stop(gpui::rgba(0),0.),gpui::linear_color_stop(rgb(0),1.))))
                .child(layout_probe("theme-color-saturation"))
                .child(div().absolute().left(gpui::relative(hsv.saturation)).top(gpui::relative(1.-hsv.value)).ml(px(-7.)).mt(px(-7.)).size(px(14.)).rounded_full().border_2().border_color(rgb(0xffffff)).bg(rgb(hsv.rgb()))))
            .child(div().w_full().flex().justify_center().child(self.hue.clone()))
            .child(div().relative().child(layout_probe("theme-color-hex")).child(self.hex.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hsv_round_trips_srgb_grid_and_primary_colors_without_drift() {
        for r in (0..=255_u32).step_by(17) {
            for g in (0..=255_u32).step_by(17) {
                for b in (0..=255_u32).step_by(17) {
                    let rgb = (r<<16)|(g<<8)|b;
                    assert_eq!(Hsv::from_rgb(rgb).rgb(),rgb);
                }
            }
        }
        assert_eq!(Hsv::new(0.,1.,1.).rgb(),0xff0000);
        assert_eq!(Hsv::new(360.,1.,1.).rgb(),0xff0000);
        assert_eq!(Hsv::new(120.,1.,1.).rgb(),0x00ff00);
        assert_eq!(Hsv::new(240.,1.,1.).rgb(),0x0000ff);
    }
    #[test]
    fn pointer_bounds_and_achromatic_values_stay_finite() {
        let mut hsv = Hsv::from_rgb(0x808080);
        hsv.position(-1.,2.);
        assert_eq!(hsv.rgb(),0);
        hsv.position(2.,-1.);
        assert_eq!(hsv.rgb(),0xff0000);
        assert_eq!(Hsv::new(f32::NAN,f32::INFINITY,f32::NAN).rgb(),0);
    }
}
