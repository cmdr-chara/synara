//! Native appearance workbench. File access is explicit and background-only.
use super::*;

pub(in crate::shell) struct PersonalizationState {
    pub navigation_shown: bool,
    pub tools_shown: bool,
    pub details_shown: bool,
    pub attention_open: bool,
    pub attention_focus: FocusHandle,
    pub profile_open: bool,
    pub profile_text: Entity<TextEntry>,
    accent_text: Entity<TextEntry>,
    pub busy: bool,
    image_path: Option<PathBuf>,
    image_blur: u8,
    metrics: Option<(u16, u32, Option<String>)>,
    image: Option<Arc<gpui::Image>>,
    image_size: Option<(u32, u32)>,
    generation: u64,
    loading: bool,
    error: Option<String>,
    applied_material: Option<SurfaceMaterial>,
    _subscriptions: Vec<Subscription>,
}
impl PersonalizationState {
    pub fn new(value: &AppearanceSettings, cx: &mut Context<Shell>) -> Self {
        let profile_text = cx.new(|cx| TextEntry::new("Paste an exported appearance profile", EntryMode::Editor, 140., cx));
        let accent_text = cx.new(|cx| TextEntry::new("#RRGGBB", EntryMode::SingleLine, 32., cx));
        if let Some(accent) = value.personalization.accent {
            accent_text.update(cx, |entry, cx| entry.set_text(format!("#{accent:06x}"), cx));
        }
        let subscriptions = vec![cx.subscribe(&profile_text, |_, _, _, cx| cx.notify())];
        Self {
            navigation_shown: false, tools_shown: false, details_shown: false,
            attention_open: false, attention_focus: cx.focus_handle(), profile_open: false, profile_text, accent_text,
            busy: false, image_path: None, image_blur: 0, metrics: None, image: None, image_size: None,
            generation: 0, loading: false, error: None, applied_material: None,
            _subscriptions: subscriptions,
        }
    }
}
#[derive(Clone, Copy)]
enum Adjust { Canvas, Panels, Dim, Blur, Width, UiFont, CodeFont, TerminalFont }
impl Shell {
    pub(in crate::shell) fn appearance_pending(&self, cx: &App) -> bool {
        self.settings.saving || self.settings.personalization.busy
            || !self.settings.personalization.profile_text.read(cx).text().trim().is_empty()
    }
    pub(in crate::shell) fn open_appearance(&mut self, cx: &mut Context<Self>) {
        self.set_panel(Panel::Settings, cx);
        self.open_settings_section(Section::Appearance, cx);
    }
    pub(in crate::shell) fn prepare_personalization(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let appearance = &self.settings.value.appearance;
        let reduced = appearance.reduced_motion || appearance.personalization.motion == MotionPreference::Off;
        if cx.reduce_motion() != reduced { cx.set_reduce_motion(reduced); }
        let material = appearance.personalization.material;
        if self.settings.personalization.applied_material != Some(material) {
            window.set_background_appearance(match material {
                SurfaceMaterial::Solid => gpui::WindowBackgroundAppearance::Opaque,
                SurfaceMaterial::Transparent => gpui::WindowBackgroundAppearance::Transparent,
                SurfaceMaterial::Frosted | SurfaceMaterial::Glass => gpui::WindowBackgroundAppearance::Blurred,
            });
            self.settings.personalization.applied_material = Some(material);
        }
        let metrics = (appearance.personalization.chat_width, appearance.fonts.ui_size.to_bits(), appearance.fonts.ui_family.clone());
        if self.settings.personalization.metrics.as_ref() != Some(&metrics) {
            self.settings.personalization.metrics = Some(metrics);
            self.transcript.list.remeasure_items(0..self.transcript.list.item_count());
        }
        let path = appearance.personalization.wallpaper.clone();
        let blur = if matches!(material, SurfaceMaterial::Frosted | SurfaceMaterial::Glass) { appearance.personalization.wallpaper_blur } else { 0 };
        if self.settings.personalization.image_path == path && self.settings.personalization.image_blur == blur { return; }
        let state = &mut self.settings.personalization;
        state.image_path = path.clone();
        state.image_blur = blur;
        state.image = None;
        state.image_size = None;
        state.error = None;
        state.generation = state.generation.wrapping_add(1);
        state.loading = path.is_some();
        let generation = state.generation;
        let Some(path) = path else { return };
        let (sender, receiver) = async_channel::bounded(1);
        self.runtime.spawn(async move {
            let result = WorkspaceService::render_wallpaper(path, blur).await.map_err(|error| error.to_string());
            let _ = sender.send(result).await;
        });
        cx.spawn(async move |view, cx| {
            let Ok(result) = receiver.recv().await else { return };
            let _ = view.update(cx, |this, cx| {
                let state = &mut this.settings.personalization;
                if state.generation != generation { return; }
                state.loading = false;
                match result {
                    Ok(asset) => {
                        let format = match asset.format {
                            PreviewImageFormat::Png => gpui::ImageFormat::Png,
                            PreviewImageFormat::Jpeg => gpui::ImageFormat::Jpeg,
                        };
                        state.image_size = Some((asset.width, asset.height));
                        state.image = Some(Arc::new(gpui::Image::from_bytes(format, asset.bytes)));
                    }
                    Err(error) => state.error = Some(error),
                }
                cx.notify();
            });
        }).detach();
    }
    pub(in crate::shell) fn appearance_background(&self) -> gpui::AnyElement {
        let style = &self.settings.value.appearance.personalization;
        let image = self.settings.personalization.image.clone();
        let fit = match style.wallpaper_fit {
            WallpaperFit::Cover => gpui::ObjectFit::Cover,
            WallpaperFit::Contain => gpui::ObjectFit::Contain,
        };
        // A single tint over the desktop or local image. Child text stays crisp.
        div().absolute().inset_0().overflow_hidden()
            .children(image.map(|image| gpui::img(image).absolute().inset_0().size_full().object_fit(fit)))
            .child(div().absolute().inset_0().bg(ui::canvas_background()))
            .children((style.material == SurfaceMaterial::Glass).then(|| {
                div().absolute().inset_0().border_1().border_color(ui::glass_edge())
            }))
            .into_any_element()
    }
    fn choose_wallpaper(&mut self, cx: &mut Context<Self>) {
        if self.settings.saving || self.settings.personalization.busy { return; }
        self.settings.personalization.busy = true;
        let before = self.settings.value.appearance.clone();
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true, directories: false, multiple: false, prompt: Some("Choose a PNG or JPEG wallpaper".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                this.settings.personalization.busy = false;
                if this.settings.value.appearance != before {
                    this.settings.personalization.error = Some("Appearance changed during image selection. The late choice was ignored.".into());
                    cx.notify(); return;
                }
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() { this.validate_wallpaper_choice(path, cx); }
                    }
                    Ok(Ok(None)) => {},
                    _ => this.settings.personalization.error = Some("The image picker could not open. The previous wallpaper is unchanged.".into()),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    fn validate_wallpaper_choice(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.settings.personalization.busy = true;
        let before = self.settings.value.appearance.clone();
        let selected = path.clone();
        let (sender, receiver) = async_channel::bounded(1);
        self.runtime.spawn(async move {
            let result = WorkspaceService::read_wallpaper(selected).await.map(|_| ()).map_err(|error| error.to_string());
            let _ = sender.send(result).await;
        });
        cx.spawn(async move |view, cx| {
            let result = receiver.recv().await;
            let _ = view.update(cx, |this, cx| {
                this.settings.personalization.busy = false;
                if this.settings.value.appearance != before {
                    this.settings.personalization.error = Some("Appearance changed during image selection. The late choice was ignored.".into());
                    cx.notify(); return;
                }
                match result {
                    Ok(Ok(())) if !this.settings.saving => {
                        this.save_setting(|settings| settings.appearance.personalization.wallpaper = Some(path), cx);
                    }
                    Ok(Ok(())) => this.settings.personalization.error = Some("Wait for the current settings save, then choose the image again. The wallpaper is unchanged.".into()),
                    Ok(Err(error)) => this.settings.personalization.error = Some(error),
                    Err(_) => this.settings.personalization.error = Some("The image reader stopped. The previous wallpaper is unchanged.".into()),
                }
                cx.notify();
            });
        }).detach();
    }
    fn choose_material(&mut self, material: SurfaceMaterial, cx: &mut Context<Self>) {
        self.save_setting(|s| {
            let p = &mut s.appearance.personalization;
            p.material = material;
            let (canvas, panels) = match material {
                SurfaceMaterial::Solid => (85, 92),
                SurfaceMaterial::Transparent | SurfaceMaterial::Glass => (70, 82),
                SurfaceMaterial::Frosted => (76, 86),
            };
            p.canvas_opacity = canvas; p.panel_opacity = panels;
        }, cx);
    }
    fn adjust_appearance(&mut self, setting: Adjust, amount: i16, cx: &mut Context<Self>) {
        self.save_setting(|settings| {
            let a = &mut settings.appearance;
            let p = &mut a.personalization;
            let bounded = |value: u8, min: i16, max: i16| (i16::from(value) + amount).clamp(min, max) as u8;
            match setting {
                Adjust::Canvas => p.canvas_opacity = bounded(p.canvas_opacity, 35, 100),
                Adjust::Panels => p.panel_opacity = bounded(p.panel_opacity, 60, 100),
                Adjust::Dim => p.wallpaper_dim = bounded(p.wallpaper_dim, 0, 95),
                Adjust::Blur => p.wallpaper_blur = bounded(p.wallpaper_blur, 0, 64),
                Adjust::Width => p.chat_width = (i32::from(p.chat_width) + i32::from(amount)).clamp(560, 1200) as u16,
                Adjust::UiFont => a.fonts.ui_size = (a.fonts.ui_size + f32::from(amount)).clamp(10., 24.),
                Adjust::CodeFont => a.fonts.code_size = (a.fonts.code_size + f32::from(amount)).clamp(10., 24.),
                Adjust::TerminalFont => p.terminal_font_size = bounded(p.terminal_font_size, 10, 24),
            }
        }, cx);
    }
    fn appearance_stepper(&self, id: &'static str, label: &str, value: String, kind: Adjust, step: i16, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().flex().flex_wrap().items_center().gap_2().py_2()
            .child(div().flex_1().min_w(px(120.)).text_size(px(13.)).child(label.to_owned()))
            .child(ui::action((id, 0), "-", None, false, cx.listener(move |this, _: &(), _, cx| this.adjust_appearance(kind, -step, cx))).aria_label(format!("Decrease {label}")))
            .child(div().w(px(72.)).text_center().text_size(px(12.)).child(value))
            .child(ui::action((id, 1), "+", None, false, cx.listener(move |this, _: &(), _, cx| this.adjust_appearance(kind, step, cx))).aria_label(format!("Increase {label}")))
            .into_any_element()
    }
    fn apply_accent(&mut self, cx: &mut Context<Self>) {
        let value = self.settings.personalization.accent_text.read(cx).text().trim().trim_start_matches('#').to_owned();
        let color = if value.is_empty() { None } else {
            match u32::from_str_radix(&value, 16).ok().filter(|_| value.len() == 6) {
                Some(color) => Some(color),
                None => { self.error = Some("Use exactly six hexadecimal digits, such as #91b9d8, or leave the field blank.".into()); cx.notify(); return; }
            }
        };
        self.save_setting(|settings| settings.appearance.personalization.accent = color, cx);
    }
    fn import_appearance_profile(&mut self, cx: &mut Context<Self>) {
        let text = self.settings.personalization.profile_text.read(cx).text();
        match AppearanceProfile::import(text, &self.settings.value.appearance) {
            Ok(appearance) => self.save_setting(|settings| settings.appearance = appearance, cx),
            Err(error) => { self.error = Some(error.to_string()); cx.notify(); }
        }
        // Keep the pasted profile until the user explicitly clears it, including on save failure.
    }
    pub(super) fn personalization_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let a = &self.settings.value.appearance;
        let p = &a.personalization;
        let state = &self.settings.personalization;
        let mut page = div().mt_4().flex().flex_col().gap_2()
            .child(heading("Focus and color"))
            .child(row("Zen mode", "A quieter presentation of Synara and Hubs, not a separate chat history. Ctrl/Cmd+Alt+Z toggles it.",
                ui::action("zen-preference", if p.zen_mode { "On" } else { "Off" }, None, p.zen_mode,
                    cx.listener(|this, _: &(), _, cx| this.toggle_zen(cx)))
                    .role(gpui::Role::Switch).aria_label("Zen mode")
                    .aria_toggled(if p.zen_mode { gpui::Toggled::True } else { gpui::Toggled::False })))
            .child(div().flex().flex_wrap().gap_2().children([
                (Colorway::Original, "Original"), (Colorway::Graphite, "Graphite"),
                (Colorway::Midnight, "Midnight"), (Colorway::Ocean, "Ocean"),
                (Colorway::Forest, "Forest"), (Colorway::Ember, "Ember"), (Colorway::Sand, "Sand"),
            ].into_iter().enumerate().map(|(index, (colorway, label))| {
                ui::action(("colorway", index), label, None, p.colorway == colorway,
                    cx.listener(move |this, _: &(), _, cx| this.save_setting(|s| s.appearance.personalization.colorway = colorway, cx)))
            })))
            .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Each colorway has a light and dark treatment. Original uses the Synara/Dracula choice above."))
            .child(div().flex().flex_wrap().gap_2().items_center()
                .child(div().flex_1().min_w(px(120.)).child(state.accent_text.clone()))
                .child(ui::action("apply-accent", "Apply accent", None, false, cx.listener(|this, _: &(), _, cx| this.apply_accent(cx))))
                .child(ui::action("reset-accent", "Theme accent", None, false, cx.listener(|this, _: &(), _, cx| {
                    this.settings.personalization.accent_text.update(cx, |entry, cx| entry.clear(cx));
                    this.save_setting(|s| s.appearance.personalization.accent = None, cx);
                }))))
            .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Focus-ring colors are adjusted when needed to remain visible on the selected canvas."))
            .child(heading("Material"))
            .child(div().flex().flex_wrap().gap_2().children([
                (SurfaceMaterial::Solid, "Solid"), (SurfaceMaterial::Transparent, "Transparent"),
                (SurfaceMaterial::Frosted, "Frosted"), (SurfaceMaterial::Glass, "Glass"),
            ].into_iter().enumerate().map(|(index, (material, label))| ui::action(("material", index), label, None, p.material == material,
                cx.listener(move |this, _: &(), _, cx| this.choose_material(material, cx))))))
            .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Frosted and Glass request native desktop blur. The OS/compositor decides availability. Glass adds translucent surfaces and edge highlights, not a refractive shader. Menus and approval text retain solid contrast."))
            .child(self.appearance_stepper("canvas-opacity", "Window tint", format!("{}%", p.canvas_opacity), Adjust::Canvas, 5, cx))
            .child(self.appearance_stepper("panel-opacity", "Panel tint", format!("{}%", p.panel_opacity), Adjust::Panels, 5, cx))
            .child(heading("Wallpaper"))
            .child(ui::action("desktop-glass", "Use desktop glass", Some(Glyph::Window), false, cx.listener(|this, _: &(), _, cx| {
                this.save_setting(|s| {
                    let p = &mut s.appearance.personalization;
                    p.wallpaper = None; p.material = SurfaceMaterial::Glass;
                    p.canvas_opacity = 70; p.panel_opacity = 82;
                }, cx);
            })))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("choose-wallpaper", if state.busy { "Reading image..." } else { "Choose image..." }, Some(Glyph::Files), false, cx.listener(|this, _: &(), _, cx| this.choose_wallpaper(cx))))
                .child(ui::action("remove-wallpaper", "Remove", None, false, cx.listener(|this, _: &(), _, cx| {
                    this.save_setting(|s| s.appearance.personalization.wallpaper = None, cx);
                })))
                .child(ui::action("reload-wallpaper", "Reload", None, false, cx.listener(|this, _: &(), _, cx| {
                    this.settings.personalization.image_path = None;
                    this.settings.personalization.generation = this.settings.personalization.generation.wrapping_add(1);
                    cx.notify();
                }))))
            .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child(
                p.wallpaper.as_ref().map_or_else(|| "No wallpaper. Local PNG/JPEG, up to 8 MiB and 16 megapixels. Never attached to a chat automatically.".into(),
                    |path| format!("{}{}", path.file_name().unwrap_or_default().to_string_lossy(),
                        state.image_size.map_or_else(String::new, |(w, h)| format!(" · {w} x {h}"))))))
            .children(state.loading.then(|| div().text_size(px(12.)).child("Loading wallpaper...")))
            .children(state.error.as_ref().map(|error| div().text_size(px(12.)).text_color(rgb(palette().error)).child(error.clone())))
            .child(div().flex().gap_2().children([(WallpaperFit::Cover, "Fill"), (WallpaperFit::Contain, "Fit")].into_iter().enumerate().map(|(index, (fit, label))| {
                ui::action(("wallpaper-fit", index), label, None, p.wallpaper_fit == fit,
                    cx.listener(move |this, _: &(), _, cx| this.save_setting(|s| s.appearance.personalization.wallpaper_fit = fit, cx)))
            })))
            .child(self.appearance_stepper("wallpaper-dim", "Wallpaper dimming", format!("{}%", p.wallpaper_dim), Adjust::Dim, 5, cx))
            .child(self.appearance_stepper("wallpaper-blur", "Local image blur", format!("{}", p.wallpaper_blur), Adjust::Blur, 4, cx))
            .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Local image blur applies only to wallpapers in Frosted/Glass. Desktop blur is controlled by your compositor."))
            .child(heading("Space and motion"))
            .child(div().flex().flex_wrap().gap_2().children([
                (DensityPreference::Compact, "Compact"), (DensityPreference::Comfortable, "Comfortable"), (DensityPreference::Spacious, "Spacious"),
            ].into_iter().enumerate().map(|(index, (density, label))| ui::action(("density", index), label, None, p.density == density,
                cx.listener(move |this, _: &(), _, cx| this.save_setting(|s| s.appearance.personalization.density = density, cx))))))
            .child(self.appearance_stepper("chat-width", "Conversation width", format!("{} px", p.chat_width), Adjust::Width, 40, cx))
            .child(self.appearance_stepper("ui-font-size", "UI and chat size", format!("{:.0} px", a.fonts.ui_size), Adjust::UiFont, 1, cx))
            .child(self.appearance_stepper("code-font-size", "Editor text size", format!("{:.0} px", a.fonts.code_size), Adjust::CodeFont, 1, cx))
            .child(self.appearance_stepper("terminal-font-size", "Terminal text size", format!("{} px", p.terminal_font_size), Adjust::TerminalFont, 1, cx))
            .child(div().flex().flex_wrap().gap_2().children([
                (MotionPreference::Off, "Off"), (MotionPreference::Subtle, "Subtle"), (MotionPreference::Standard, "Standard"), (MotionPreference::Expressive, "Expressive"),
            ].into_iter().enumerate().map(|(index, (motion, label))| ui::action(("motion-style", index), label, None, p.motion == motion,
                cx.listener(move |this, _: &(), _, cx| this.save_setting(|s| s.appearance.personalization.motion = motion, cx))))))
            .child(div().text_size(px(12.)).text_color(rgb(palette().muted)).child("Motion adjusts panel and message transitions, not idle animation. Reduce motion above always takes priority."))
            .child(heading("Appearance profiles"))
            .child(div().flex().flex_wrap().gap_2()
                .child(ui::action("copy-appearance-profile", "Copy profile JSON", Some(Glyph::Copy), false, cx.listener(|this, _: &(), _, cx| {
                    match AppearanceProfile::export(&this.settings.value.appearance) {
                        Ok(text) => { cx.write_to_clipboard(gpui::ClipboardItem::new_string(text)); this.notice = Some("Appearance profile copied without wallpaper paths or chat data.".into()); },
                        Err(error) => this.error = Some(error.to_string()),
                    }
                    cx.notify();
                })))
                .child(ui::action("show-profile-import", if state.profile_open { "Hide importer" } else { "Import profile..." }, None, false, cx.listener(|this, _: &(), _, cx| {
                    this.settings.personalization.profile_open = !this.settings.personalization.profile_open; cx.notify();
                }))));
        if state.profile_open {
            page = page.child(div().flex().flex_col().gap_2()
                .child(state.profile_text.clone())
                .child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Appearance only, up to 64 KiB. Current wallpaper and Zen state are preserved. Clear the field to discard the pasted profile."))
                .child(div().flex().gap_2()
                    .child(ui::action("import-appearance-profile", "Apply profile", None, false, cx.listener(|this, _: &(), _, cx| this.import_appearance_profile(cx))))
                    .child(ui::action("clear-appearance-profile", "Clear", None, false, cx.listener(|this, _: &(), _, cx| {
                        this.settings.personalization.profile_text.update(cx, |entry, cx| entry.clear(cx)); cx.notify();
                    })))));
        }
        page.into_any_element()
    }
}
