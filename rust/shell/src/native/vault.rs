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
    pub(super) fn configure_vault(&mut self, key: &'static str, enabled: bool, cx: &mut Context<Self>) {
        let Some(snapshot) = self.vault_snapshot.clone() else { return };
        let current = snapshot.get("settings").cloned().unwrap_or(Value::Null);
        let mut settings = browserhost_rs::vault::VaultSettings {
            agent_use: current.get("agentUse").and_then(Value::as_bool).unwrap_or(true),
            offer_save: current.get("offerSave").and_then(Value::as_bool).unwrap_or(false),
            autosave: current.get("autosave").and_then(Value::as_bool).unwrap_or(false),
        };
        match key {
            "agentUse" => settings.agent_use = enabled,
            "offerSave" => settings.offer_save = enabled,
            "autosave" => settings.autosave = enabled,
            _ => return,
        }
        if !settings.offer_save {
            settings.autosave = false;
        }
        match browserhost_rs::cdp::shared_browser_vault()
            .and_then(|vault| vault.configure(settings))
        {
            Ok(snapshot) => {
                self.vault_snapshot = Some(snapshot);
                self.vault_error = None;
            }
            Err(error) => self.vault_error = Some(error),
        }
        cx.notify();
    }

    pub(super) fn vault_switch(
        cx: &mut Context<Self>,
        key: &'static str,
        on: bool,
        accent: gpui::Hsla,
    ) -> gpui::AnyElement {
        switch_track(on, accent, gpui_kit::component::Theme::global(cx))
            .id(ElementId::Name(format!("vault-switch-{key}").into()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _ev, _w, cx| {
                this.configure_vault(key, !on, cx);
            }))
            .into_any_element()
    }

    pub(super) fn submit_vault_master(&mut self, setup: bool, window: &mut Window, cx: &mut Context<Self>) {
        let password = self.vault_master_input.read(cx).value().to_string();
        let confirmation = self.vault_confirm_input.read(cx).value().to_string();
        if password.is_empty() {
            return;
        }
        if setup && password != confirmation {
            self.vault_error = Some("Use at least 12 characters and enter the same master password twice.".into());
            cx.notify();
            return;
        }
        let result = browserhost_rs::cdp::shared_browser_vault().and_then(|vault| {
            if setup {
                vault.setup_master(&password)
            } else {
                vault.unlock(&password)
            }
        });
        self.vault_master_input.update(cx, |state, cx| state.set_value("", window, cx));
        self.vault_confirm_input.update(cx, |state, cx| state.set_value("", window, cx));
        match result {
            Ok(()) => {
                self.vault_error = None;
                self.refresh_vault(cx);
            }
            Err(error) => {
                self.vault_error = Some(error);
                cx.notify();
            }
        }
    }

    pub(super) fn lock_vault(&mut self, cx: &mut Context<Self>) {
        match browserhost_rs::cdp::shared_browser_vault().and_then(|vault| vault.lock()) {
            Ok(()) => {
                self.vault_revealed = None;
                self.vault_reveal_id = None;
                self.refresh_vault(cx);
            }
            Err(error) => {
                self.vault_error = Some(error);
                cx.notify();
            }
        }
    }

    pub(super) fn submit_vault_reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(id) = self.vault_reveal_id.clone() else { return };
        let password = self.vault_reveal_input.read(cx).value().to_string();
        if password.is_empty() {
            return;
        }
        let result = browserhost_rs::cdp::shared_browser_vault()
            .and_then(|vault| vault.reveal(&id, &password));
        self.vault_reveal_input.update(cx, |state, cx| state.set_value("", window, cx));
        match result {
            Ok(revealed) => {
                let password = revealed
                    .get("password")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                self.vault_revealed = Some((id.clone(), password));
                self.vault_error = None;
                cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(20))
                        .await;
                    let _ = this.update(cx, |this: &mut Self, cx| {
                        if this.vault_revealed.as_ref().is_some_and(|(open, _)| open == &id) {
                            this.vault_revealed = None;
                            this.vault_reveal_id = None;
                            cx.notify();
                        }
                    });
                })
                .detach();
            }
            Err(error) => self.vault_error = Some(error),
        }
        cx.notify();
    }

    pub(super) fn respond_vault_prompt(&mut self, id: &str, save: bool, cx: &mut Context<Self>) {
        match browserhost_rs::cdp::shared_browser_vault()
            .and_then(|vault| vault.respond(id, save))
        {
            Ok(snapshot) => {
                self.vault_snapshot = Some(snapshot);
                self.vault_error = None;
            }
            Err(error) => self.vault_error = Some(error),
        }
        cx.notify();
    }

    pub(super) fn remove_vault_login(&mut self, id: &str, cx: &mut Context<Self>) {
        match browserhost_rs::cdp::shared_browser_vault().and_then(|vault| vault.remove(id)) {
            Ok(snapshot) => {
                self.vault_snapshot = Some(snapshot);
                self.vault_delete_id = None;
                self.vault_error = None;
            }
            Err(error) => self.vault_error = Some(error),
        }
        cx.notify();
    }

    pub(super) fn import_cookies(&mut self, cx: &mut Context<Self>) {
        let Some(browser) = self.cookie_browser.clone() else {
            self.cookie_error = Some("Choose a browser profile before importing cookies.".into());
            cx.notify();
            return;
        };
        let profile = self.cookie_profile.clone();
        if profile.is_none() {
            self.cookie_error = Some("Choose the browser profile to import from.".into());
            cx.notify();
            return;
        }
        let thread = self.selected_thread.clone().unwrap_or_default();
        let destination = browserhost_rs::cdp::cookie_import_destinations(&thread)
            .first()
            .copied();
        let Some(destination) = destination else {
            self.cookie_error = Some("Open a Synara browser tab in this chat before importing cookies.".into());
            cx.notify();
            return;
        };
        let mode = match destination {
            "ego-lite" => browserhost_rs::cdp::CdpMode::Visible,
            _ => browserhost_rs::cdp::CdpMode::Headless,
        };
        let domains: Vec<String> = self
            .cookie_domain_input
            .read(cx)
            .value()
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|part| !part.is_empty())
            .map(str::to_string)
            .collect();
        if domains.len() > 1 {
            self.cookie_error = Some("Enter one visible site, or leave the field empty for a whole-profile import.".into());
            cx.notify();
            return;
        }
        if domains.is_empty() && !self.cookie_profile_confirmed {
            self.cookie_error = Some("Confirm whole-profile session access before importing all cookies.".into());
            cx.notify();
            return;
        }
        self.cookie_importing = true;
        self.cookie_error = None;
        self.cookie_result = None;
        cx.notify();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    browserhost_rs::cdp::sync_cookie_import(
                        &thread,
                        &browser,
                        profile.as_deref(),
                        &domains,
                        mode,
                    )
                    .await
                })
                .await;
            let _ = this.update(cx, |this: &mut Self, cx| {
                this.cookie_importing = false;
                this.cookie_profile_confirmed = false;
                match result {
                    Ok(result) => {
                        let imported = result.get("imported").and_then(Value::as_u64).unwrap_or(0);
                        let skipped = result.get("skipped").and_then(Value::as_u64).unwrap_or(0);
                        this.cookie_result = Some(format!("Imported {imported} cookies; skipped {skipped}."));
                        this.cookie_error = None;
                    }
                    Err(error) => this.cookie_error = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
    }
}
