//! Device, shortcut, notification and privacy settings with real service owners.
use super::*;
use synara_runtime::{
    DeviceBackend, NotificationRequest, desktop_notification_status,
    desktop_notifications_available, desktop_notify,
};

pub(super) struct NativeSettings {
    bindings: Vec<Entity<TextEntry>>,
    contextual_bindings: Vec<Entity<TextEntry>>,
    confirmation: Entity<TextEntry>,
    deleting: Option<TaskId>,
    busy: bool,
    picker: bool,
    notification_busy: bool,
    error: Option<String>,
    status: Option<String>,
    _subscriptions: Vec<Subscription>,
}
impl NativeSettings {
    pub fn new(value: &AppSettings, cx: &mut Context<Shell>) -> Self {
        let mut bindings = Vec::new();
        for command in NAVIGATION_COMMANDS {
            let entry = cx.new(|cx| TextEntry::new(command.label, EntryMode::SingleLine, 32., cx));
            entry.update(cx, |input, cx| {
                input.set_text(navigation_binding(&value.keybindings, command).into(), cx)
            });
            bindings.push(entry);
        }
        let contextual_bindings = CONTEXTUAL_COMMANDS
            .iter()
            .map(|command| {
                let entry =
                    cx.new(|cx| TextEntry::new(command.label, EntryMode::SingleLine, 32., cx));
                entry.update(cx, |input, cx| {
                    input.set_text(
                        contextual_binding(&value.keybindings, command.id).unwrap_or_default(),
                        cx,
                    )
                });
                entry
            })
            .collect();
        let confirmation =
            cx.new(|cx| TextEntry::new("Type DELETE to confirm", EntryMode::SingleLine, 32., cx));
        let subscription = cx.subscribe(&confirmation, |_, _, _, cx| cx.notify());
        Self {
            bindings,
            contextual_bindings,
            confirmation,
            deleting: None,
            busy: false,
            picker: false,
            notification_busy: false,
            error: None,
            status: None,
            _subscriptions: vec![subscription],
        }
    }
}
pub(in crate::shell) enum Reply {
    Notification(Result<(), String>),
    Deleted(TaskId, Result<(), String>),
    TraceCleared(TaskId, Result<Vec<TraceEntry>, String>),
}
impl Shell {
    pub(in crate::shell) fn native_settings_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        match reply {
            Reply::Notification(result) => {
                self.settings.native.notification_busy = false;
                match result {
                    Ok(()) => self.settings.native.status = Some("Notification submitted to the OS. Verify that it appeared. Successful submission is not proof of visible delivery.".into()),
                    Err(error) => self.settings.native.error = Some(error),
                }
            }
            Reply::Deleted(task, result) => {
                self.settings.native.busy = false;
                match result {
                    Ok(()) => {
                        self.catalog.tasks.retain(|candidate| candidate.id != task);
                        self.drafts.remove(&task);
                        self.draft_state.forget_task(task);
                        self.settings.native.deleting = None;
                        self.settings
                            .native
                            .confirmation
                            .update(cx, |entry, cx| entry.clear(cx));
                        self.settings.native.status = Some("Archived thread deleted from this database. Workspace files and external backups were not touched.".into());
                        self.settings.activity = None;
                        self.load_profile_activity();
                        self.hydrate();
                    }
                    Err(error) => self.settings.native.error = Some(error),
                }
            }
            Reply::TraceCleared(task, result) => {
                self.settings.native.busy = false;
                match result {
                    Ok(_) => {
                        if self.selected == Some(task) {
                            self.trace.clear();
                        }
                        self.settings.native.status = Some("The connection's in-memory protocol trace was cleared. Other chats sharing this connection share that trace. New activity can create new trace entries.".into());
                    }
                    Err(error) => self.settings.native.error = Some(error),
                }
            }
        }
        cx.notify();
    }
    pub(in crate::shell) fn sync_navigation_bindings(&mut self, cx: &mut Context<Self>) {
        for (entry, command) in self
            .settings
            .native
            .bindings
            .iter()
            .zip(NAVIGATION_COMMANDS)
        {
            let text = navigation_binding(&self.settings.value.keybindings, command).to_owned();
            entry.update(cx, |input, cx| input.set_text(text, cx));
        }
        for (entry, command) in self
            .settings
            .native
            .contextual_bindings
            .iter()
            .zip(CONTEXTUAL_COMMANDS)
        {
            let text = contextual_binding(&self.settings.value.keybindings, command.id)
                .unwrap_or_default();
            entry.update(cx, |input, cx| input.set_text(text, cx));
        }
    }
    fn save_navigation_bindings(&mut self, cx: &mut Context<Self>) {
        let mut bindings: Vec<_> = self
            .settings
            .value
            .keybindings
            .iter()
            .filter(|binding| {
                !NAVIGATION_COMMANDS
                    .iter()
                    .any(|command| command.id == binding.command)
                    && !CONTEXTUAL_COMMANDS
                        .iter()
                        .any(|command| command.id == binding.command)
            })
            .cloned()
            .collect();
        for (entry, command) in self
            .settings
            .native
            .bindings
            .iter()
            .zip(NAVIGATION_COMMANDS)
        {
            let key = match NavigationKeystroke::parse(entry.read(cx).text().trim()) {
                Ok(key) => key.display(),
                Err(error) => {
                    self.settings.native.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            };
            if key != command.default {
                bindings.push(KeyBinding {
                    command: command.id.into(),
                    shortcut: key,
                });
            }
        }
        for (entry, command) in self
            .settings
            .native
            .contextual_bindings
            .iter()
            .zip(CONTEXTUAL_COMMANDS)
        {
            let text = entry.read(cx).text().trim();
            if text.is_empty() {
                continue;
            }
            let key = match parse_contextual_shortcut(command.id, text) {
                Ok(key) => key,
                Err(error) => {
                    self.settings.native.error = Some(error.to_string());
                    cx.notify();
                    return;
                }
            };
            if command.default != Some(key.as_str()) {
                bindings.push(KeyBinding {
                    command: command.id.into(),
                    shortcut: key,
                });
            }
        }
        if let Err(error) = validate_navigation_bindings(&bindings) {
            self.settings.native.error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.settings.native.error = None;
        self.save_setting(|settings| settings.keybindings = bindings, cx);
    }
    pub(super) fn editable_keybindings_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().flex().flex_col().gap_3()
            .child("Primary means Control on Linux/Windows and Command on macOS. Navigation and focused Composer/Editor actions have separate allowed chords. Leave Send blank to follow Chat settings.")
            .children(self.settings.native.bindings.iter().zip(NAVIGATION_COMMANDS).map(|(entry, command)|
                row(command.label, format!("Default: {}", command.default), div().w(px(235.)).child(entry.clone()))))
            .children(self.settings.native.contextual_bindings.iter().zip(CONTEXTUAL_COMMANDS).map(|(entry, command)|
                row(command.label, format!("{:?} · Default: {}", command.context, command.default.unwrap_or("Chat setting")), div().w(px(235.)).child(entry.clone()))))
            .child(div().flex().gap_2()
                .child(ui::action("apply-navigation-bindings", "Save shortcuts", None, false, cx.listener(|this, _: &(), _, cx| this.save_navigation_bindings(cx))))
                .child(ui::action("restore-navigation-bindings", "Restore default shortcuts", None, false, cx.listener(|this, _: &(), _, cx| {
                    this.settings.native.error = None;
                    this.save_setting(|settings| settings.keybindings.retain(|binding| !NAVIGATION_COMMANDS.iter().any(|command| command.id == binding.command) && !CONTEXTUAL_COMMANDS.iter().any(|command| command.id == binding.command)), cx);
                    // Also discard an unsaved draft when defaults were already active.
                    this.sync_navigation_bindings(cx);
                }))))
            .child("Navigation shortcuts appear in the command palette. Composer and editor shortcuts run only while that input has focus. Terminal, Zen and Space shortcuts are not editable here.")
            .children(self.settings.value.keybindings.iter().any(|binding| !NAVIGATION_COMMANDS.iter().any(|command| command.id == binding.command) && !CONTEXTUAL_COMMANDS.iter().any(|command| command.id == binding.command)).then(|| div().text_size(px(12.)).child("Legacy or unimplemented bindings are preserved in settings but are not executed by this editor.")))
            .children(self.settings.native.error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
            .into_any_element()
    }
    pub(super) fn device_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let settings = &self.settings.value.device;
        div().flex().flex_col().gap_3()
            .child(heading("Device helper"))
            .child(div().flex().gap_2().children([(DeviceBackend::Android, "Android / ADB"), (DeviceBackend::AppleSimulator, "Apple Simulator")].into_iter().enumerate().map(|(index, (backend, label))|
                ui::action(("device-backend", index), label, None, settings.backend == backend, cx.listener(move |this, _: &(), _, cx| this.save_setting(|settings| settings.device.backend = backend, cx))))))
            .child(row("ADB executable", settings.adb_path.as_ref().map_or("Not configured".into(), |path| path.display().to_string()),
                ui::action("choose-adb", if self.settings.native.picker { "Choosing..." } else { "Choose executable" }, Some(Glyph::Files), false, cx.listener(|this, _: &(), _, cx| this.choose_adb(cx)))))
            .child(row("Apple Simulator helper", settings.apple_helper_path.as_ref().map_or("Not configured".into(), |path| path.display().to_string()),
                ui::action("choose-apple-device-helper", if self.settings.native.picker { "Choosing..." } else { "Choose helper" }, Some(Glyph::Files), false, cx.listener(|this, _: &(), _, cx| this.choose_apple_device_helper(cx)))))
            .child("Only select adb from your trusted Android SDK installation or a trusted synara-device-helper built for your Xcode installation. Helpers are never downloaded or started during settings load.")
            .child(row("Capture while visible", "Capture every two seconds after your first explicit capture. Hidden viewers stop capturing. No frames are saved or attached automatically.",
                self.toggle("device-auto-capture", "Capture while visible", settings.auto_capture, |settings| settings.device.auto_capture = !settings.device.auto_capture, cx)))
            .child(row("Authority", "Input is off by default, scoped to the selected device and revoked on disconnect, hiding, configuration changes, errors or orientation changes.", "Ask each session"))
            .child(row("Supported paths", "Android: connected targets, screenshots, tap/swipe/text/key input and emulator shutdown. macOS: installed iOS simulators, boot/shutdown, screenshots, recording, app lifecycle, helper-backed touch/swipe/text/keys/hardware buttons, accessibility inspection and semantic targeting. Physical iOS devices and Android cold boot remain outside this backend.", ""))
            .children((settings.backend == DeviceBackend::AppleSimulator && !cfg!(target_os = "macos")).then(|| div().text_color(rgb(palette().error)).child("Unsupported on this host: Apple Simulator requires macOS and Xcode. No helper will run here.")))
            .child(div().flex().gap_2()
                .child(ui::action("open-device-viewer", "Open Device viewer", Some(Glyph::Window), false, cx.listener(|this, _: &(), _, cx| this.set_panel(Panel::Device, cx))))
                .child(ui::action("forget-device-helper", "Reset device preferences", None, false, cx.listener(|this, _: &(), _, cx| this.save_setting(|settings| settings.device = DeviceSettings::default(), cx)))))
            .child(heading("Window capture / AppSnap"))
            .child("Window and screen capture, OS capture permission inspection and AppSnap shortcuts are not implemented. Device screenshots use the selected helper and do not imply desktop capture permission.")
            .children(self.settings.native.error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
            .into_any_element()
    }
    fn choose_adb(&mut self, cx: &mut Context<Self>) {
        if self.settings.native.picker || self.settings.saving {
            return;
        }
        self.settings.native.picker = true;
        let before = self.settings.value.device.clone();
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose the trusted Android SDK adb executable".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                this.settings.native.picker = false;
                if this.settings.value.device != before || this.settings.saving {
                    this.settings.native.error = Some("Settings changed during selection. The late executable choice was ignored.".into()); cx.notify(); return;
                }
                match result {
                    Ok(Ok(Some(paths))) => if let Some(path) = paths.into_iter().next() {
                        if path.is_absolute() && path.is_file() {
                            this.save_setting(|settings| settings.device.adb_path = Some(path), cx);
                        } else { this.settings.native.error = Some("Choose an existing absolute executable path.".into()); }
                    },
                    Ok(Ok(None)) => {},
                    _ => this.settings.native.error = Some("The native file picker could not open. Device configuration is unchanged.".into()),
                }
                cx.notify();
            });
        }).detach();
        cx.notify();
    }
    fn choose_apple_device_helper(&mut self, cx: &mut Context<Self>) {
        if self.settings.native.picker || self.settings.saving {
            return;
        }
        self.settings.native.picker = true;
        let before = self.settings.value.device.clone();
        let picker = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose the trusted synara-device-helper executable".into()),
        });
        cx.spawn(async move |view, cx| {
            let result = picker.await;
            let _ = view.update(cx, |this, cx| {
                this.settings.native.picker = false;
                if this.settings.value.device != before || this.settings.saving {
                    this.settings.native.error = Some(
                        "Settings changed during selection. The late helper choice was ignored."
                            .into(),
                    );
                    cx.notify();
                    return;
                }
                match result {
                    Ok(Ok(Some(paths))) => {
                        if let Some(path) = paths.into_iter().next() {
                            if path.is_absolute() && path.is_file() {
                                this.save_setting(
                                    |settings| settings.device.apple_helper_path = Some(path),
                                    cx,
                                );
                            } else {
                                this.settings.native.error =
                                    Some("Choose an existing absolute executable path.".into());
                            }
                        }
                    }
                    Ok(Ok(None)) => {}
                    _ => {
                        this.settings.native.error = Some(
                            "The native file picker could not open. Device configuration is unchanged."
                                .into(),
                        )
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(super) fn notification_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().flex().flex_col().gap_3()
            .child(row("Background chat completion", "Notify when a chat other than the selected chat finishes. Messages contain no prompt, task title, path or response text. Default: off.",
                self.toggle("notify-background-completion", "Background chat completion", self.settings.value.notifications.background_completion, |settings| settings.notifications.background_completion = !settings.notifications.background_completion, cx)))
            .child(desktop_notification_status())
            .children(desktop_notifications_available().then(|| ui::action("test-notification", "Send test notification", Some(Glyph::Bell), false, cx.listener(|this, _: &(), _, cx| this.send_desktop_notification(true, cx)))))
            .child("OS do-not-disturb and notification permissions can suppress delivery. No permission is silently requested or assumed. In-app activity and permission controls remain available regardless of this preference.")
            .children(self.settings.native.notification_busy.then(|| div().child("Submitting notification...")))
            .children(self.settings.native.status.as_ref().map(|status| div().child(status.clone())))
            .children(self.settings.native.error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
            .into_any_element()
    }
    pub(in crate::shell) fn send_desktop_notification(
        &mut self,
        test: bool,
        cx: &mut Context<Self>,
    ) {
        if self.settings.native.notification_busy
            || (!test && !self.settings.value.notifications.background_completion)
        {
            return;
        }
        self.settings.native.notification_busy = true;
        self.settings.native.status = None;
        self.settings.native.error = None;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = desktop_notify(NotificationRequest {
                title: "Synara".into(),
                body: if test {
                    "Test notification. No conversation content is included."
                } else {
                    "A background chat has finished. Open Synara to review the result."
                }
                .into(),
            })
            .await
            .map_err(|e| e.to_string());
            let _ = sender
                .send(Update::NativeSettings(Box::new(Reply::Notification(
                    result,
                ))))
                .await;
        });
        cx.notify();
    }
    pub(super) fn privacy_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().flex().flex_col().gap_3()
            .child(heading("Data on this device"))
            .child(row("Local data", format!("{} projects and {} threads in the current catalog. Conversation history, drafts, attachments and preferences are stored in this installation's database.", self.catalog.projects.len(), self.catalog.tasks.len()), ""))
            .child(row("Secret store", "Actual controller adapter status. Unavailable credentials are never replaced by a plaintext fallback. Agent-owned login state is separate.", format!("{:?}", self.controller.secret_store_state())))
            .child(row("Protocol diagnostics", "Bounded, redacted connection metadata is retained in memory. Clear affects all chats sharing the selected connection. It is not a deletion of transcript data or external logs.",
                ui::action("privacy-clear-trace", "Clear selected connection trace", None, false, cx.listener(|this, _: &(), _, cx| {
                    if this.settings.native.busy { return; }
                    let Some(task) = this.selected else { this.settings.native.error = Some("Select a chat with a connection first.".into()); cx.notify(); return; };
                    this.settings.native.busy = true;
                    let controller = this.controller.clone(); let sender = this.sender.clone();
                    this.runtime.spawn(async move { let result = controller.trace(task, true).await.map_err(|e| e.to_string()); let _ = sender.send(Update::NativeSettings(Box::new(Reply::TraceCleared(task, result)))).await; });
                    cx.notify();
                }))))
            .child(row("Device privacy", "Device frames and input grants are memory-only. Disconnecting discards the current frame. Neither frames nor device control are automatically exposed to agents.",
                ui::action("privacy-disconnect-device", "Discard device frame and authority", None, false, cx.listener(|this, _: &(), _, cx| { this.device.retire(); cx.notify(); }))))
            .child(row("Archived data", "Archived threads are retained until explicitly deleted. Deletion is irreversible within this database. SQLite free pages, backups, OS snapshots and agent-owned remote history are not securely erased.",
                ui::action("privacy-archived", "Review archived threads", Some(Glyph::Archive), false, cx.listener(|this, _: &(), _, cx| this.open_settings_section(Section::Archived, cx)))))
            .child("Project directories are never deleted by these controls. This build does not provide archived-project bulk deletion, backup management or automatic retention expiry.")
            .children(self.settings.native.status.as_ref().map(|status| div().child(status.clone())))
            .children(self.settings.native.error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
            .into_any_element()
    }
    pub(super) fn archived_deletion_controls(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let Some(task) = self
            .settings
            .native
            .deleting
            .and_then(|id| self.catalog.tasks.iter().find(|task| task.id == id))
        else {
            return div()
                .children(
                    self.settings
                        .native
                        .status
                        .as_ref()
                        .map(|status| div().child(status.clone())),
                )
                .into_any_element();
        };
        let id = task.id;
        div().id("archived-delete-confirmation").role(gpui::Role::Group).aria_label("Confirm permanent archived thread deletion")
            .flex().flex_col().gap_2().p_3().border_1().border_color(rgb(palette().error))
            .child(format!("Permanently delete \"{}\"?", task.title))
            .child("Deletes this thread's transcript, sessions, drafts, attachment snapshots, follow-ups and task preferences. Shared Hub knowledge promoted earlier remains. Does not delete project files, agent-owned history or backups. Type DELETE, then confirm. No automatic retention timer runs.")
            .child(self.settings.native.confirmation.clone())
            .child(div().flex().gap_2()
                .child(ui::action("cancel-archive-delete", "Keep archived thread", None, false, cx.listener(|this, _: &(), _, cx| {
                    if !this.settings.native.busy { this.settings.native.deleting = None; this.settings.native.error = None; cx.notify(); }
                })))
                .children((self.settings.native.confirmation.read(cx).text() == "DELETE" && !self.settings.native.busy).then(|| ui::action("confirm-archive-delete", "Permanently delete thread", None, false, cx.listener(move |this, _: &(), _, cx| this.delete_archived_thread(id, cx))))))
            .children(self.settings.native.busy.then(|| div().child("Closing the thread session and deleting its archived data...")))
            .children(self.settings.native.error.as_ref().map(|error| div().text_color(rgb(palette().error)).child(error.clone())))
            .into_any_element()
    }
    pub(super) fn begin_archived_deletion(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.settings.native.busy {
            return;
        }
        self.settings.native.deleting = Some(id);
        self.settings.native.error = None;
        self.settings
            .native
            .confirmation
            .update(cx, |entry, cx| entry.clear(cx));
        cx.notify();
    }
    fn delete_archived_thread(&mut self, id: TaskId, cx: &mut Context<Self>) {
        if self.settings.native.busy
            || self.settings.native.deleting != Some(id)
            || self.settings.native.confirmation.read(cx).text() != "DELETE"
        {
            return;
        }
        if self.selected == Some(id)
            || self.loading_task == Some(id)
            || self.busy.contains(&id)
            || self.connecting.contains(&id)
            || self.controls.is_pending(id)
            || self.chat_tools.pending_write()
            || self.draft_state.pending_for(id)
            || self.attachments.close_pending()
            || self.followups.pending(cx)
            || self.hubs.pending(cx)
        {
            self.settings.native.error = Some("Select another chat and finish pending work, saves or imports before permanently deleting this archived thread.".into());
            cx.notify();
            return;
        }
        self.settings.native.busy = true;
        self.settings.native.error = None;
        let controller = self.controller.clone();
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = controller
                .delete_archived_task(id)
                .await
                .map_err(|e| e.to_string());
            let _ = sender
                .send(Update::NativeSettings(Box::new(Reply::Deleted(id, result))))
                .await;
        });
        cx.notify();
    }
    pub(in crate::shell) fn native_settings_pending(&self) -> bool {
        self.settings.native.busy || self.settings.native.picker
    }
}

impl Shell {
    pub(in crate::shell) fn native_navigation_shortcut(
        &mut self,
        event: &gpui::KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let modifiers = event.keystroke.modifiers;
        let primary_only = if cfg!(target_os = "macos") {
            modifiers.platform && !modifiers.control
        } else {
            modifiers.control && !modifiers.platform
        };
        if event.is_held
            || event.prefer_character_input
            || !primary_only
            || self.close != CloseState::Open
            || self.composer.read(cx).is_composing()
            || self.editor.read(cx).is_composing()
            || self.controls.is_open()
            || self.environment.menu_open()
            || self.chat_tools.menu_open()
            || self.settings.popup.is_some()
        {
            return false;
        }
        let key = NavigationKeystroke {
            key: event.keystroke.key.to_lowercase(),
            alt: modifiers.alt,
            shift: modifiers.shift,
        };
        for command in NAVIGATION_COMMANDS {
            if NavigationKeystroke::parse(navigation_binding(
                &self.settings.value.keybindings,
                command,
            ))
            .ok()
            .as_ref()
                != Some(&key)
            {
                continue;
            }
            let panel = match command.id {
                "navigation.chat" => Panel::Conversation,
                "navigation.files" => Panel::Files,
                "navigation.changes" => Panel::Changes,
                "navigation.terminal" => Panel::Terminal,
                "navigation.inspector" => Panel::Inspector,
                "navigation.settings" => Panel::Settings,
                "navigation.agents" => Panel::Registry,
                "navigation.remote" => Panel::Remote,
                "navigation.kanban" => Panel::Kanban,
                "navigation.device" => Panel::Device,
                _ => return false,
            };
            if panel == Panel::Conversation && self.dock_open() {
                self.hide_environment(cx);
            } else {
                self.set_panel(panel, cx);
            }
            return true;
        }
        false
    }
    pub(in crate::shell) fn panel_shortcut_label(&self, panel: Panel) -> Option<&str> {
        let id = match panel {
            Panel::Conversation => "navigation.chat",
            Panel::Files => "navigation.files",
            Panel::Changes => "navigation.changes",
            Panel::Terminal => "navigation.terminal",
            Panel::Inspector => "navigation.inspector",
            Panel::Settings => "navigation.settings",
            Panel::Registry => "navigation.agents",
            Panel::Remote => "navigation.remote",
            Panel::Kanban => "navigation.kanban",
            Panel::Device => "navigation.device",
            _ => return None,
        };
        NAVIGATION_COMMANDS
            .iter()
            .find(|command| command.id == id)
            .map(|command| navigation_binding(&self.settings.value.keybindings, command))
    }
}
