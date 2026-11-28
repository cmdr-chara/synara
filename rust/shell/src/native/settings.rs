//! Extracted from native/mod.rs during the surface split — see mod.rs.
#![allow(unused_imports)]

use super::*;
use gpui_kit::component::{
    button::{Button, ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants as _},
    h_flex,
    input::{Editor, EditorState, Input, InputEvent, InputState},
    menu::{DropdownMenu as _, PopupMenuItem},
    slider::{Slider, SliderEvent, SliderState},
    scroll::ScrollableElement as _,
    spinner::Spinner,
    text::TextView,
    theme::Theme,
    v_flex,
    Icon,
    ActiveTheme as _, IconName, Sizable as _,
};
use gpui_kit::*;
use gpui_kit::component::WindowExt as _;
use gpui_kit::base::Disableable as _;
use gpui_kit::assets::IconName as AssetIcon;
use gpui::prelude::FluentBuilder as _;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

impl Native {
    pub(super) fn toggle_inbox(&mut self, cx: &mut Context<Self>) {
        self.view_inbox = !self.view_inbox;
        if self.view_inbox {
            self.view_files = false;
            self.view_dev = false;
            self.view_settings = false;
            self.view_tasks = false;
            self.view_kanban = false;
            self.view_pullreq = false;
            self.view_autom = false;
            self.view_studio = false;
            self.view_spaces = false;
            self.fetch_inbox(cx);
        }
        cx.notify();
    }

    pub(super) fn autom_summary(schedule: &Value) -> String {
        let ty = str_at(schedule, "type");
        let time = str_at(schedule, "timeOfDay").to_string();
        match ty {
            "once" => format!("Once · {}", str_at(schedule, "runAt")),
            "interval" => {
                let s = schedule["everySeconds"].as_i64().unwrap_or(0);
                if s % 60 == 0 && s >= 60 {
                    format!("Every {} min", s / 60)
                } else {
                    format!("Every {}s", s)
                }
            }
            "daily" => format!("Daily at {time}"),
            "weekdays" => format!("Weekdays at {time}"),
            "weekly" => {
                let d = schedule["dayOfWeek"].as_i64().unwrap_or(0).clamp(0, 6) as usize;
                format!("Weekly {} at {}", AUTOM_DAYS[d], time)
            }
            "cron" => str_at(schedule, "expression").to_string(),
            "project-event" => "On project events".to_string(),
            other => other.to_string(),
        }
    }

    pub(super) fn open_autom_dialog(
        &mut self,
        edit: Option<Value>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut mk = |cx: &mut Context<Self>, ph: &'static str, val: &str| {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder(ph));
            if !val.is_empty() {
                let v = val.to_string();
                input.update(cx, |s, cx| s.set_value(v, window, cx));
            }
            input
        };
        let (mut edit_id, mut kind, mut day, mut enabled, mut time_v, mut extra_v) =
            (None, 2usize, 1usize, true, String::new(), String::new());
        let (mut name_v, mut prompt_v, mut proj_id) =
            (String::new(), String::new(), String::new());
        if let Some(def) = &edit {
            edit_id = Some(str_at(def, "id").to_string());
            name_v = str_at(def, "name").to_string();
            prompt_v = str_at(def, "prompt").to_string();
            enabled = get(def, "enabled").as_bool().unwrap_or(true);
            proj_id = str_at(def, "projectId").to_string();
            let sched = get(def, "schedule");
            match str_at(sched, "type") {
                "once" => {
                    kind = 0;
                    extra_v = str_at(sched, "runAt").to_string();
                }
                "interval" => {
                    kind = 1;
                    extra_v = (sched["everySeconds"].as_i64().unwrap_or(0) / 60).to_string();
                }
                "daily" => {
                    kind = 2;
                    time_v = str_at(sched, "timeOfDay").to_string();
                }
                "weekdays" => {
                    kind = 3;
                    time_v = str_at(sched, "timeOfDay").to_string();
                }
                "weekly" => {
                    kind = 4;
                    day = sched["dayOfWeek"].as_i64().unwrap_or(1).clamp(0, 6) as usize;
                    time_v = str_at(sched, "timeOfDay").to_string();
                }
                "cron" => {
                    kind = 5;
                    extra_v = str_at(sched, "expression").to_string();
                }
                _ => {}
            }
        }
        let projects = autom_projects(self.snapshot.as_ref().unwrap_or(&Value::Null));
        let mut proj = 0usize;
        if !proj_id.is_empty() {
            if let Some(i) = projects
                .iter()
                .position(|p| str_at(p, "id") == proj_id.as_str())
            {
                proj = i;
            }
        }
        let name = mk(cx, "Automation name", &name_v);
        let prompt = mk(cx, "What should this automation do?", &prompt_v);
        let time = mk(cx, "HH:MM", &time_v);
        let extra_ph = match kind {
            0 => "Run at (YYYY-MM-DD HH:MM)",
            1 => "Every N minutes",
            5 => "Cron expression",
            _ => "Value",
        };
        let extra = mk(cx, extra_ph, &extra_v);
        self.autom_dialog = Some(AutomForm {
            edit_id,
            name,
            prompt,
            time,
            extra,
            kind,
            day,
            proj,
            enabled,
        });
        if let Some(f) = &self.autom_dialog {
            let focus = f.name.read(cx).focus_handle(cx);
            window.focus(&focus, cx);
        }
        cx.notify();
    }

    pub(super) fn close_autom_dialog(&mut self, cx: &mut Context<Self>) {
        self.autom_dialog = None;
        cx.notify();
    }

    pub(super) fn autom_run_now(&mut self, id: String, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call("automation.runNow", json!({"automationId": id})) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.fetch_automations(cx);
            });
        })
        .detach();
    }

    pub(super) fn autom_delete(&mut self, id: String, cx: &mut Context<Self>) {
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call("automation.delete", json!({"automationId": id})) })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.autom_sel = None;
                this.fetch_automations(cx);
            });
        })
        .detach();
    }

    pub(super) fn submit_autom_dialog(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.autom_dialog.take() else {
            return;
        };
        let name = form.name.read(cx).value().trim().to_string();
        let prompt = form.prompt.read(cx).value().trim().to_string();
        let time = form.time.read(cx).value().trim().to_string();
        let extra = form.extra.read(cx).value().trim().to_string();
        let projects = autom_projects(self.snapshot.as_ref().unwrap_or(&Value::Null));
        let project_id = projects
            .get(form.proj)
            .map(|p| str_at(p, "id").to_string())
            .unwrap_or_default();
        if name.is_empty() || prompt.is_empty() || project_id.is_empty() {
            self.autom_dialog = Some(form);
            cx.notify();
            return;
        }
        let time_of_day = if time.is_empty() {
            "09:00".to_string()
        } else {
            time
        };
        let schedule = match form.kind {
            0 => {
                let mut at = extra.replace(' ', "T");
                if at.len() == 16 {
                    at.push_str(":00Z");
                } else if !at.ends_with('Z') && !at.contains('+') {
                    at.push('Z');
                }
                json!({"type":"once","runAt":at})
            }
            1 => json!({
                "type":"interval",
                "everySeconds": extra.parse::<i64>().unwrap_or(60).max(1) * 60,
            }),
            2 => json!({"type":"daily","timeOfDay":time_of_day}),
            3 => json!({"type":"weekdays","timeOfDay":time_of_day}),
            4 => json!({"type":"weekly","dayOfWeek":form.day,"timeOfDay":time_of_day}),
            _ => json!({"type":"cron","expression":extra,"timezone":"UTC"}),
        };
        let payload = json!({
            "projectId": project_id,
            "name": name,
            "prompt": prompt,
            "schedule": schedule,
            "enabled": form.enabled,
        });
        let (tag, payload) = if let Some(id) = &form.edit_id {
            let mut p = payload;
            p["id"] = json!(id);
            ("automation.update", p)
        } else {
            ("automation.create", payload)
        };
        let cell = self.client.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let client = cx
                .background_executor()
                .spawn(async move { ensure_client(&cell) })
                .await;
            let Some(client) = client else { return };
            let c = client.clone();
            cx.background_executor()
                .spawn(async move { c.call(tag, payload) })
                .detach();
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.fetch_automations(cx);
            });
        })
        .detach();
        cx.notify();
    }

    /// Applies the stored theme-pack overrides (upstream ThemePackEditor's
    /// per-variant pack) for the currently resolved variant: reset the
    /// registered theme, then layer accent/background/foreground on top.
    pub(super) fn apply_theme_pack(&mut self, cx: &mut Context<Self>) {
        let mode = match self.appearance_mode {
            1 => gpui_kit::component::ThemeMode::Light,
            2 => gpui_kit::component::ThemeMode::Dark,
            _ => gpui_kit::component::ThemeMode::from(cx.window_appearance()),
        };
        gpui_kit::component::Theme::change(mode, None, cx);
        let dark = matches!(
            gpui_kit::component::Theme::global(cx).mode,
            gpui_kit::component::ThemeMode::Dark
        );
        let pack = self
            .theme_packs
            .get(if dark { "dark" } else { "light" })
            .cloned()
            .unwrap_or_else(|| json!({}));
        let window_background = crate::window_material::background_for_pack(
            crate::window_material::current_translucency(),
            &pack,
        );
        for handle in cx.windows() {
            let _ = handle.update(cx, |_, window, _| {
                window.set_background_appearance(window_background);
            });
        }
        // Contrast (upstream theme.contrast 0-100): pushes the resolved
        // surface toward the variant's extreme and the ink the opposite
        // way. The effective swatches are always emitted at contrast > 0 so
        // the slider acts on default packs too.
        let contrast = pack
            .get("contrast")
            .and_then(Value::as_f64)
            .unwrap_or(0.)
            .clamp(0., 100.)
            / 100.;
        let mix_hex = |hex: &str, toward: u32, t: f64| -> String {
            let c = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0);
            let m = |a: u32, b: u32| -> u32 { (a as f64 + (b as f64 - a as f64) * t).round() as u32 };
            format!(
                "#{:02X}{:02X}{:02X}",
                m((c >> 16) & 0xff, (toward >> 16) & 0xff),
                m((c >> 8) & 0xff, (toward >> 8) & 0xff),
                m(c & 0xff, toward & 0xff)
            )
        };
        let mut colors = serde_json::Map::new();
        for (k, fallback, extreme) in [
            ("background", if dark { "#111111" } else { "#FFFFFF" }, if dark { 0x000000u32 } else { 0xffffffu32 }),
            ("foreground", if dark { "#FCFCFC" } else { "#0D0D0D" }, if dark { 0xffffffu32 } else { 0x000000u32 }),
        ] {
            let v = pack
                .get(k)
                .and_then(Value::as_str)
                .filter(|s| s.len() == 7 && s.starts_with('#'));
            let eff = match (v, contrast > 0.) {
                (Some(hex), true) => Some(mix_hex(hex, extreme, contrast)),
                (Some(hex), false) => Some(hex.to_string()),
                (None, true) => Some(mix_hex(fallback, extreme, contrast)),
                (None, false) => None,
            };
            if let Some(eff) = eff {
                colors.insert(k.to_string(), json!(eff));
            }
        }
        // Upstream's Accent is the brand color: it drives both accent and
        // primary surfaces (toggles/buttons read `primary`), with a
        // luminance-picked foreground. Falls back to Synara's brand accent so
        // `theme.primary` always means "the accent the user sees".
        let acc_override = pack.get("accent").and_then(Value::as_str);
        let acc = match acc_override {
            Some(v) if v.len() == 7 && v.starts_with('#') => v.to_string(),
            _ => {
                if dark {
                    "#F2612D".to_string()
                } else {
                    "#C74614".to_string()
                }
            }
        };
        {
            let c = u32::from_str_radix(&acc[1..], 16).unwrap_or(0);
            let (r, g, b) = (
                ((c >> 16) & 0xff) as f32,
                ((c >> 8) & 0xff) as f32,
                (c & 0xff) as f32,
            );
            let fg = if 0.2126 * r + 0.7152 * g + 0.0722 * b > 140. {
                "#000000"
            } else {
                "#FFFFFF"
            };
            let _ = fg;
            // Upstream at zero contrast: primary surfaces are the ink color
            // (measured light #0D0D0D / dark #FCFCFC — buttons render black,
            // not brand-orange), while `accent` is the soft tint wash.
            let ink_primary = if dark { "#FCFCFC" } else { "#0D0D0D" };
            let ink_primary_fg = if dark { "#0D0D0D" } else { "#FFFFFF" };
            let soft_accent = if dark { "#230E06" } else { "#FAEFEA" };
            colors.insert("accent".to_string(), json!(soft_accent));
            colors.insert("primary".to_string(), json!(ink_primary));
            colors.insert("primary_foreground".to_string(), json!(ink_primary_fg));
            colors.insert("accent_foreground".to_string(), json!(acc));
        }
        // Upstream's zero-contrast ink-derived tokens, measured on the live
        // web app: the alpha channels sit on the variant's ink color. Without
        // these the gpui defaults leak visibly different grays into hover
        // states, borders and secondary text.
        let ink_tokens: [(&str, &str, &str); 5] = [
            // token, light value, dark value
            ("muted_foreground", "rgba(13, 13, 13, 0.598)", "rgba(252, 252, 252, 0.58)"),
            ("muted", "rgba(13, 13, 13, 0.04)", "rgba(252, 252, 252, 0.006)"),
            ("secondary", "rgba(13, 13, 13, 0.03)", "rgba(252, 252, 252, 0.026)"),
            ("border", "rgba(13, 13, 13, 0.069)", "rgba(252, 252, 252, 0.072)"),
            ("input", "rgba(13, 13, 13, 0.069)", "rgba(252, 252, 252, 0.072)"),
        ];
        for (k, light_v, dark_v) in ink_tokens {
            colors.insert(k.to_string(), json!(if dark { dark_v } else { light_v }));
        }
        // The ring is the text-accent in both variants (measured), and the
        // dark accent/primary differ from the shell's earlier guesses.
        colors.insert(
            "ring".to_string(),
            json!(if dark { "#F58056" } else { "#C74614" }),
        );
        // Upstream theme fonts: pack.fonts.ui drives the UI family,
        // pack.fonts.code the monospace family (empty = default).
        let font_of = |sub: &str| -> Option<String> {
            pack.get("fonts")
                .and_then(|f| f.get(sub))
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
                .map(|s| s.trim().to_string())
        };
        let mut typography = serde_json::Map::new();
        if let Some(f) = font_of("ui") {
            typography.insert("sans".to_string(), json!(f));
        }
        if let Some(f) = font_of("code") {
            typography.insert("mono".to_string(), json!(f));
        }
        if !colors.is_empty() || !typography.is_empty() {
            let cfg = json!({
                "tokens": {
                    "colors": Value::Object(colors),
                    "typography": Value::Object(typography),
                }
            })
            .to_string();
            let _ = gpui_kit::component::Theme::global_mut(cx)
                .apply_semantic_config_str(&cfg);
        }
        gpui_kit::component::Theme::sync_base(cx);
        cx.refresh_windows();
    }

    /// Opens the hex editor for one pack token (upstream's swatch popover).
    pub(super) fn open_theme_pack_edit(
        &mut self,
        dark: bool,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let pack = self
            .theme_packs
            .get(if dark { "dark" } else { "light" })
            .cloned()
            .unwrap_or_else(|| json!({}));
        let cur = match key {
            // Upstream FontInput rows edit pack.fonts.ui / pack.fonts.code;
            // empty means "use the default family".
            "fontUi" | "fontCode" => pack
                .get("fonts")
                .and_then(|f| f.get(if key == "fontUi" { "ui" } else { "code" }))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            _ => pack
                .get(key)
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_else(|| {
                    // Effective value = the built-in swatch of that variant.
                    match (dark, key) {
                        (true, "accent") => "#F2612D",
                        (true, "background") => "#111111",
                        (true, _) => "#FCFCFC",
                        (false, "accent") => "#C74614",
                        (false, "background") => "#FFFFFF",
                        (false, _) => "#0D0D0D",
                    }
                    .to_string()
                }),
        };
        let placeholder = match key {
            "fontUi" => "System default",
            "fontCode" => "JetBrains Mono",
            _ => "#RRGGBB",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let _ = input.update(cx, |i, cx| {
            i.set_value(&cur, window, cx);
        });
        self.tp_input = Some(input);
        self.tp_edit = Some((dark, key.to_string()));
        cx.notify();
    }

    /// Writes one theme pack field for a variant (upstream
    /// updateThemePack/setWindowTranslucency/setCodeThemeId) and persists.
    pub(super) fn tp_set_pack(&mut self, dark: bool, key: &str, value: Value, cx: &mut Context<Self>) {
        let variant = if dark { "dark" } else { "light" };
        if !self.theme_packs.is_object() {
            self.theme_packs = json!({});
        }
        let packs = self.theme_packs.as_object_mut().unwrap();
        let pack = packs
            .entry(variant.to_string())
            .or_insert_with(|| json!({}));
        if let Some(o) = pack.as_object_mut() {
            o.insert(key.to_string(), value);
        }
        self.persist_appearance(cx);
        self.apply_theme_pack(cx);
    }

    /// Upstream resetThemeVariant: drop every override of one variant pack.
    pub(super) fn tp_reset_pack(&mut self, dark: bool, cx: &mut Context<Self>) {
        let variant = if dark { "dark" } else { "light" };
        if let Some(o) = self.theme_packs.as_object_mut() {
            o.remove(variant);
        }
        self.persist_appearance(cx);
        self.apply_theme_pack(cx);
    }

    /// Slider write-back for opacity/blur/contrast (upstream
    /// setWindowTranslucency / updateThemePack contrast).
    pub(super) fn tp_slider_change(&mut self, key: &str, v: f32, cx: &mut Context<Self>) {
        let Some((variant, what)) = key.split_once('-') else {
            return;
        };
        let variant = if variant == "dark" { "dark" } else { "light" };
        if !self.theme_packs.is_object() {
            self.theme_packs = json!({});
        }
        let packs = self.theme_packs.as_object_mut().unwrap();
        let pack = packs
            .entry(variant.to_string())
            .or_insert_with(|| json!({}));
        if let Some(o) = pack.as_object_mut() {
            match what {
                "contrast" => {
                    o.insert("contrast".to_string(), json!(v.round() as i64));
                }
                "opacity" | "blur" => {
                    let t = o
                        .entry("translucency".to_string())
                        .or_insert_with(|| json!({}));
                    if let Some(t) = t.as_object_mut() {
                        t.insert(what.to_string(), json!(v.round() as i64));
                    }
                }
                _ => {}
            }
        }
        self.persist_appearance(cx);
        self.apply_theme_pack(cx);
    }

    pub(super) fn commit_theme_pack_edit(&mut self, reset: bool, cx: &mut Context<Self>) {
        let Some((dark, key)) = self.tp_edit.clone() else {
            return;
        };
        let variant = if dark { "dark" } else { "light" };
        if !self.theme_packs.is_object() {
            self.theme_packs = json!({});
        }
        let packs = self.theme_packs.as_object_mut().unwrap();
        let pack = packs
            .entry(variant.to_string())
            .or_insert_with(|| json!({}));
        if key == "fontUi" || key == "fontCode" {
            // Upstream FontInput: empty input clears the override back to
            // the default family; non-empty stores the family name as-is.
            let sub = if key == "fontUi" { "ui" } else { "code" };
            let v = if reset {
                String::new()
            } else {
                self.tp_input
                    .as_ref()
                    .map(|i| i.read(cx).value().to_string())
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            };
            if let Some(o) = pack.as_object_mut() {
                let fonts = o.entry("fonts".to_string()).or_insert_with(|| json!({}));
                if let Some(f) = fonts.as_object_mut() {
                    if v.is_empty() {
                        f.remove(sub);
                    } else {
                        f.insert(sub.to_string(), json!(v));
                    }
                }
            }
        } else if reset {
            if let Some(o) = pack.as_object_mut() {
                o.remove(&key);
            }
        } else {
            let v = self
                .tp_input
                .as_ref()
                .map(|i| i.read(cx).value().to_string())
                .unwrap_or_default()
                .trim()
                .to_string();
            let norm = if v.starts_with('#') { v.clone() } else { format!("#{v}") };
            if norm.len() == 7
                && norm[1..].chars().all(|c| c.is_ascii_hexdigit())
            {
                if let Some(o) = pack.as_object_mut() {
                    o.insert(key.clone(), json!(norm));
                }
            } else {
                return;
            }
        }
        self.tp_edit = None;
        self.tp_input = None;
        self.persist_appearance(cx);
        self.apply_theme_pack(cx);
    }

    /// Exports the variant's effective pack as upstream's theme JSON string.
    pub(super) fn copy_theme_pack(&self, dark: bool, cx: &mut App) {
        let pack = self
            .theme_packs
            .get(if dark { "dark" } else { "light" })
            .cloned()
            .unwrap_or_else(|| json!({}));
        let get_hex = |k: &str, fallback: &str| -> String {
            pack.get(k)
                .and_then(Value::as_str)
                .map(|v| v.to_string())
                .unwrap_or_else(|| fallback.to_string())
        };
        let (da, db, df) = if dark {
            ("#F2612D", "#111111", "#FCFCFC")
        } else {
            ("#C74614", "#FFFFFF", "#0D0D0D")
        };
        let out = json!({
            "tokens": {
                "colors": {
                    "accent": get_hex("accent", da),
                    "background": get_hex("background", db),
                    "foreground": get_hex("foreground", df),
                }
            }
        });
        cx.write_to_clipboard(ClipboardItem::new_string(
            serde_json::to_string_pretty(&out).unwrap_or_default(),
        ));
    }

    /// Imports a `{"tokens":{"colors":{...}}}` string into the variant pack:
    /// accent/background/foreground land in settings.appearance.themePacks.
    pub(super) fn commit_theme_pack_import(&mut self, dark: bool, cx: &mut Context<Self>) {
        let text = self
            .tp_input
            .as_ref()
            .map(|i| i.read(cx).value().to_string())
            .unwrap_or_default();
        let Ok(v) = serde_json::from_str::<Value>(&text) else {
            self.tp_import = None;
            self.tp_input = None;
            cx.notify();
            return;
        };
        let colors = v
            .get("tokens")
            .and_then(|t| t.get("colors"))
            .cloned()
            .unwrap_or_else(|| v.clone());
        if !self.theme_packs.is_object() {
            self.theme_packs = json!({});
        }
        let packs = self.theme_packs.as_object_mut().unwrap();
        let pack = packs
            .entry(if dark { "dark" } else { "light" }.to_string())
            .or_insert_with(|| json!({}));
        if let Some(o) = pack.as_object_mut() {
            for k in ["accent", "background", "foreground"] {
                if let Some(hex) = colors.get(k).and_then(Value::as_str) {
                    if hex.len() == 7 && hex.starts_with('#') {
                        o.insert(k.to_string(), json!(hex));
                    }
                }
            }
        }
        self.tp_import = None;
        self.tp_input = None;
        self.persist_appearance(cx);
        self.apply_theme_pack(cx);
    }

    pub(super) fn density_scale(&self) -> f32 {
        [0.85_f32, 1.0, 1.15][self.ui_density as usize]
    }

    /// Upstream CHAT_MAX_WIDTH_BY_MODE: standard 46rem, wide 72rem, full 100%.
    pub(super) fn chat_max_w(&self) -> Pixels {
        match self.chat_width_mode {
            0 => px(736.),
            1 => px(1152.),
            _ => px(100_000.),
        }
    }

    /// Persists the appearance map under settings.appearance, like
    /// upstream's appSettings write-through (server.updateSettings patch).
    pub(super) fn persist_appearance(&mut self, cx: &mut Context<Self>) {
        let mode = ["system", "light", "dark"][self.appearance_mode as usize];
        let dens = ["compact", "comfortable", "spacious"][self.ui_density as usize];
        let width = ["standard", "wide", "full"][self.chat_width_mode as usize];
        let appearance = json!({
            "mode": mode,
            "uiDensity": dens,
            "chatWidth": width,
            "showAsciiBackground": self.show_ascii_bg,
            "atmosphereFullWindow": self.backdrop_full,
            "themePacks": self.theme_packs.clone(),
        });
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert("appearance".to_string(), appearance.clone());
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"appearance": appearance});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    /// Settings key read with the upstream default (ChatsSettingsView rows).
    pub(super) fn setting_bool(&self, key: &str, default: bool) -> bool {
        get(get(&self.settings_data, "settings"), key)
            .as_bool()
            .unwrap_or(default)
    }

    pub(super) fn setting_str(&self, key: &str, default: &'static str) -> String {
        get(get(&self.settings_data, "settings"), key)
            .as_str()
            .unwrap_or(default)
            .to_string()
    }

    /// One-key appSettings write-through (same path as set_keep_awake):
    /// mirror locally, then server.updateSettings {key: value}.
    pub(super) fn set_setting_key(&mut self, key: &'static str, value: Value, cx: &mut Context<Self>) {
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert(key.to_string(), value.clone());
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({ key: value });
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    /// Settings switch bound to a boolean key (same visual as the shared
    /// settings `toggle`).
    pub(super) fn setting_switch(
        cx: &mut Context<Self>,
        key: &'static str,
        on: bool,
        accent: gpui::Hsla,
    ) -> gpui::AnyElement {
        switch_track(on, accent, gpui_kit::component::Theme::global(cx))
            .id(ElementId::Name(format!("setting-switch-{key}").into()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _ev, _w, cx| {
            this.set_setting_key(key, json!(!on), cx);
        }))
        .into_any_element()
    }

    /// Segmented control bound to a string key (same visual as `seg_pick`).
    pub(super) fn setting_seg(
        cx: &mut Context<Self>,
        key: &'static str,
        opts: &[(&'static str, &'static str)],
        current: &str,
        theme: &gpui_kit::component::Theme,
    ) -> gpui::AnyElement {
        let mut s = h_flex().items_center().gap(px(4.));
        for (value, label) in opts.iter().copied() {
            let is_active = value == current;
            let mut c = div()
                .id(ElementId::Name(format!("setting-seg-{key}-{value}").into()))
                .h(px(32.))
                .px(px(11.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer();
            if is_active {
                c = c.bg(theme.secondary);
            }
            let tc = if is_active { theme.foreground } else { theme.muted_foreground };
            s = s.child(
                c.child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(tc).child(label))
                    .on_click(cx.listener(move |this, _ev, _w, cx| {
                        this.set_setting_key(key, json!(value), cx);
                    })),
            );
        }
        s.into_any_element()
    }

    /// Width of the left panel card next to the rail. Upstream's Code review
    /// list is its own section, 416px by default (min 16rem); every other
    /// surface uses the 257px sidebar card.
    pub(super) fn side_panel_w(&self) -> f32 {
        if self.view_pullreq && !self.view_settings {
            416.
        } else {
            257.
        }
    }

    /// Upstream keep-awake segmented control: writes settings.keepAwakeMode
    /// ("on" | "agent" | "off"); the server turns that into caffeinate state.
    pub(super) fn set_keep_awake(&mut self, i: usize, cx: &mut Context<Self>) {
        self.keep_awake_mode = i;
        let mode = ["always", "agent", "off"][i.min(2)];
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert("keepAwakeMode".to_string(), json!(mode));
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"keepAwakeMode": mode});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    /// Upstream keepAwakeStatusLabel: "<Mode> · <Active|Idle>".
    pub(super) fn keep_awake_status(&self) -> String {
        let active = self.keep_awake_mode == 0
            || (self.keep_awake_mode == 1
                && arr(self.snapshot.as_ref().unwrap_or(&Value::Null), "threads")
                    .iter()
                    .any(|t| thread_running_for_activity(t)));
        format!(
            "{} \u{00b7} {}",
            ["On", "Agent", "Off"][self.keep_awake_mode.min(2)],
            if active { "Active" } else { "Idle" }
        )
    }

    /// Upstream DebugFeatureFlagsMenu toggle — flips flag `i` and persists
    /// the whole map under settings.sidebarUi.featureFlags.
    pub(super) fn toggle_feature_flag(&mut self, i: usize, cx: &mut Context<Self>) {
        self.feature_flags[i] = !self.feature_flags[i];
        // "Pin git progress toast" keeps a visible progress toast while on.
        if i == 1 {
            self.feedback_toast = if self.feature_flags[1] {
                Some(("Pushing changes\u{2026}".to_string(), false))
            } else {
                None
            };
        }
        self.persist_sidebar_ui(cx);
    }

    /// Persists dismissal/visited maps under settings.sidebarUi, like
    /// upstream's localStorage write-through on dismiss / mark-visited.
    pub(super) fn persist_sidebar_ui(&mut self, cx: &mut Context<Self>) {
        let sidebar_ui = json!({
            "dismissedStatusKeyByThreadId": self.dismissed_status,
            "lastVisitedAtByThreadId": self.visited_threads,
            "coachmarks": self
                .coachmarks_seen
                .iter()
                .map(|k| (k.clone(), json!("seen")))
                .collect::<serde_json::Map<String, Value>>(),
            "featureFlags": {
                "persist-action-failed-debug-toasts": self.feature_flags[0],
                "pin-git-progress-toast-preview": self.feature_flags[1],
                "show-debug-task-banner": self.feature_flags[2],
            },
            "whatsNewLastSeenVersion": self.wn_seen.clone(),
        });
        if let Some(obj) = get_mut(&mut self.settings_data, "settings")
            .and_then(|s| s.as_object_mut())
        {
            obj.insert("sidebarUi".to_string(), sidebar_ui.clone());
        }
        cx.notify();
        let cell = self.client.clone();
        let payload = json!({"sidebarUi": sidebar_ui});
        cx.spawn(async move |_this: WeakEntity<Self>, cx: &mut AsyncApp| {
            cx.background_executor()
                .spawn(async move {
                    let _ = ensure_client(&cell)
                        .and_then(|c| c.call("server.updateSettings", payload));
                })
                .await;
        })
        .detach();
    }

    pub(super) fn apply_appearance_mode(&mut self, w: Option<&mut Window>, cx: &mut Context<Self>) {
        let mode = match self.appearance_mode {
            1 => gpui_kit::component::ThemeMode::Light,
            2 => gpui_kit::component::ThemeMode::Dark,
            _ => gpui_kit::component::ThemeMode::from(cx.window_appearance()),
        };
        gpui_kit::component::Theme::change(mode, w, cx);
    }

    pub(super) fn set_appearance_mode(&mut self, mode: u8, w: &mut Window, cx: &mut Context<Self>) {
        self.appearance_mode = mode;
        self.apply_appearance_mode(Some(w), cx);
        self.apply_theme_pack(cx);
        self.persist_appearance(cx);
    }

    pub(super) fn set_ui_density(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.ui_density = ix as u8;
        self.persist_appearance(cx);
    }

    pub(super) fn set_chat_width(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.chat_width_mode = ix as u8;
        self.persist_appearance(cx);
    }

    pub(super) fn toggle_ascii_bg(&mut self, cx: &mut Context<Self>) {
        self.show_ascii_bg = !self.show_ascii_bg;
        self.persist_appearance(cx);
    }

    pub(super) fn toggle_backdrop_full(&mut self, cx: &mut Context<Self>) {
        self.backdrop_full = !self.backdrop_full;
        self.persist_appearance(cx);
    }

    /// Segmented control with real selection (the static `seg` renders the
    /// same visual; this one drives `act(ix)` on click).
    pub(super) fn seg_pick(
        cx: &mut Context<Self>,
        idp: &'static str,
        opts: &[&'static str],
        active: usize,
        theme: &gpui_kit::component::Theme,
        act: fn(&mut Self, usize, &mut Context<Self>),
    ) -> gpui::AnyElement {
        let mut s = h_flex().items_center().gap(px(4.));
        for (i, o) in opts.iter().enumerate() {
            let is_active = i == active;
            let mut c = div()
                .id(ElementId::Name(format!("{idp}-{i}").into()))
                .h(px(32.))
                .px(px(11.))
                .flex()
                .items_center()
                .rounded(px(8.))
                .cursor_pointer();
            if is_active {
                c = c.bg(theme.secondary);
            }
            let tc = if is_active { theme.foreground } else { theme.muted_foreground };
            s = s.child(
                c.child(div().text_size(px(13.)).font_weight(FontWeight::MEDIUM).text_color(tc).child(*o))
                    .on_click(cx.listener(move |this, _ev, _w, cx| act(this, i, cx))),
            );
        }
        s.into_any_element()
    }
}
