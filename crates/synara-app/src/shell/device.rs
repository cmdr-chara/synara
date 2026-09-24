//! Native viewer for real helper results. All device commands stay in runtime.
use super::*;
use crate::ui::{self, Glyph, palette};
use gpui::{Bounds, FocusHandle, MouseButton, Pixels};
use std::{
    cell::Cell,
    rc::Rc,
    time::{Duration, Instant},
};
use synara_runtime::{
    DeviceAvailability, DeviceBackend, DeviceCancellation, DeviceId, DeviceInput,
    DeviceInputConsent, DeviceInputGrant, DeviceKind, DeviceTools, ToolDevice, reconcile_devices,
};

pub(super) struct DeviceView {
    epoch: u64,
    cancel: DeviceCancellation,
    busy: bool,
    active: bool,
    devices: Vec<ToolDevice>,
    selected: Option<DeviceId>,
    image: Option<Arc<gpui::Image>>,
    image_bytes: Option<Vec<u8>>,
    dimensions: Option<(u32, u32)>,
    captured: Option<Instant>,
    grant: Option<Arc<DeviceInputGrant>>,
    error: Option<String>,
    message: String,
    focus: FocusHandle,
    bounds: Rc<Cell<Bounds<Pixels>>>,
    shutdown_confirmation: bool,
    url: Entity<TextEntry>,
    bundle_id: Entity<TextEntry>,
    app_path: Entity<TextEntry>,
    install_confirmation: Option<(DeviceId, PathBuf)>,
}
impl DeviceView {
    pub fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            epoch: 0,
            cancel: DeviceCancellation::new(),
            busy: false,
            active: false,
            devices: Vec::new(),
            selected: None,
            image: None,
            image_bytes: None,
            dimensions: None,
            captured: None,
            grant: None,
            error: None,
            message:
                "Choose a helper in Device settings, then Refresh. Nothing starts automatically."
                    .into(),
            focus: cx.focus_handle(),
            bounds: Rc::new(Cell::new(Bounds::default())),
            shutdown_confirmation: false,
            url: cx.new(|cx| TextEntry::new("https://...", EntryMode::SingleLine, 34., cx)),
            install_confirmation: None,
            app_path: cx.new(|cx| {
                TextEntry::new("/absolute/path/to/App.app", EntryMode::SingleLine, 34., cx)
            }),
            bundle_id: cx
                .new(|cx| TextEntry::new("com.example.App", EntryMode::SingleLine, 34., cx)),
        }
    }
    fn target(&self) -> Option<&ToolDevice> {
        self.devices
            .iter()
            .find(|device| Some(&device.descriptor.id) == self.selected.as_ref())
    }
    /// Invalidate every late reply and revoke transient input authority. This
    /// never shuts down a simulator, issues ADB kill-server, or changes chats.
    pub fn retire(&mut self) {
        self.cancel.cancel();
        self.cancel = DeviceCancellation::new();
        self.epoch = self.epoch.wrapping_add(1);
        self.busy = false;
        self.active = false;
        self.image = None;
        self.image_bytes = None;
        self.dimensions = None;
        self.captured = None;
        self.grant = None;
        self.shutdown_confirmation = false;
        self.install_confirmation = None;
    }
    pub fn configuration_changed(&mut self) {
        self.retire();
        self.devices.clear();
        self.selected = None;
        self.error = None;
        self.message = "Device configuration changed. Refresh to discover targets.".into();
    }
}
impl Drop for DeviceView {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}
pub(super) struct Reply {
    epoch: u64,
    result: Result<Outcome, String>,
}
enum Outcome {
    Discovery(Vec<ToolDevice>),
    Capture(DeviceCapture),
    InputApproved(DeviceInputGrant),
    InputSent,
    Lifecycle,
    UrlOpened,
    AppLaunched,
    AppInstalled,
    AppTerminated,
}
impl Shell {
    fn device_tools(&self) -> Result<DeviceTools, String> {
        DeviceTools::new(
            self.settings.value.device.backend,
            self.settings.value.device.adb_path.as_deref(),
        )
        .map_err(|error| error.to_string())
    }
    fn device_job(
        &mut self,
        work: impl std::future::Future<Output = Result<Outcome, String>> + Send + 'static,
    ) {
        self.device.busy = true;
        self.device.error = None;
        let epoch = self.device.epoch;
        let sender = self.sender.clone();
        self.runtime.spawn(async move {
            let result = work.await;
            let _ = sender
                .send(Update::Device(Box::new(Reply { epoch, result })))
                .await;
        });
    }
    pub(super) fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        self.device.retire();
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let cancel = self.device.cancel.clone();
        self.device.message =
            "Discovering devices. Authorize USB debugging on Android when requested.".into();
        self.device_job(async move {
            tools
                .discover(&cancel)
                .await
                .map(Outcome::Discovery)
                .map_err(|e| e.to_string())
        });
        cx.notify();
    }
    fn select_device(&mut self, id: DeviceId, cx: &mut Context<Self>) {
        self.device.retire();
        self.device.selected = Some(id);
        self.device.error = None;
        self.device.message = "Selected. Capture to view the current display.".into();
        cx.notify();
    }
    fn capture_device(&mut self, cx: &mut Context<Self>) {
        if self.device.busy {
            return;
        }
        let Some(device) = self
            .device
            .target()
            .cloned()
            .filter(|d| d.availability == DeviceAvailability::Ready)
        else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        self.device.active = true;
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            let png = tools
                .capture(&device, &cancel)
                .await
                .map_err(|e| e.to_string())?;
            let capture = tokio::task::spawn_blocking(move || DeviceCapture::decode(png))
                .await
                .map_err(|_| "Capture decoder stopped".to_string())?
                .map_err(|e| e.to_string())?;
            if cancel.is_cancelled() {
                return Err("Capture cancelled".into());
            }
            Ok(Outcome::Capture(capture))
        });
        cx.notify();
    }
    fn device_running(&mut self, running: bool, cx: &mut Context<Self>) {
        if self.device.busy {
            return;
        }
        let Some(device) = self.device.target().cloned() else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        if !running && !self.device.shutdown_confirmation {
            self.device.shutdown_confirmation = true;
            cx.notify();
            return;
        }
        self.device.retire();
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .set_running(&device, running, &cancel)
                .await
                .map(|_| Outcome::Lifecycle)
                .map_err(|e| e.to_string())
        });
        cx.notify();
    }
    fn open_device_url(&mut self, cx: &mut Context<Self>) {
        if self.device.busy || self.panel != Panel::Device {
            return;
        }
        let Some(device) = self
            .device
            .target()
            .cloned()
            .filter(|device| device.availability == DeviceAvailability::Ready)
        else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let url = self.device.url.read(cx).text().trim().to_owned();
        if let Err(error) = tools.validate_open_url(&device, &url) {
            self.device.error = Some(error.to_string());
            cx.notify();
            return;
        }
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .open_url(&device, &url, &cancel)
                .await
                .map(|_| Outcome::UrlOpened)
                .map_err(|error| error.to_string())
        });
        cx.notify();
    }
    fn launch_device_app(&mut self, cx: &mut Context<Self>) {
        if self.device.busy || self.panel != Panel::Device {
            return;
        }
        let Some(device) = self
            .device
            .target()
            .cloned()
            .filter(|device| device.availability == DeviceAvailability::Ready)
        else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let bundle_id = self.device.bundle_id.read(cx).text().trim().to_owned();
        if let Err(error) = tools.validate_launch_app(&device, &bundle_id) {
            self.device.error = Some(error.to_string());
            cx.notify();
            return;
        }
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .launch_app(&device, &bundle_id, &cancel)
                .await
                .map(|_| Outcome::AppLaunched)
                .map_err(|error| error.to_string())
        });
        cx.notify();
    }
    fn install_device_app(&mut self, cx: &mut Context<Self>) {
        if self.device.busy
            || self.panel != Panel::Device
            || self.device.app_path.read(cx).is_composing()
        {
            return;
        }
        let Some(device) = self
            .device
            .target()
            .cloned()
            .filter(|device| device.availability == DeviceAvailability::Ready)
        else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let path = PathBuf::from(self.device.app_path.read(cx).text().trim());
        if let Err(error) = tools.validate_install_app(&device, &path) {
            self.device.error = Some(error.to_string());
            cx.notify();
            return;
        }
        let review = (device.descriptor.id.clone(), path.clone());
        if self.device.install_confirmation.as_ref() != Some(&review) {
            self.device.install_confirmation = Some(review);
            self.device.message = format!(
                "Install {} into {}? This may replace an installed app. Press Confirm install to proceed. It will not launch the app.",
                path.display(),
                device.descriptor.name
            );
            cx.notify();
            return;
        }
        self.device.install_confirmation = None;
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .install_app(&device, &path, &cancel)
                .await
                .map(|_| Outcome::AppInstalled)
                .map_err(|error| error.to_string())
        });
        cx.notify();
    }
    fn terminate_device_app(&mut self, cx: &mut Context<Self>) {
        if self.device.busy
            || self.panel != Panel::Device
            || self.device.bundle_id.read(cx).is_composing()
        {
            return;
        }
        let Some(device) = self
            .device
            .target()
            .cloned()
            .filter(|device| device.availability == DeviceAvailability::Ready)
        else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let bundle_id = self.device.bundle_id.read(cx).text().trim().to_owned();
        if let Err(error) = tools.validate_launch_app(&device, &bundle_id) {
            self.device.error = Some(error.to_string());
            cx.notify();
            return;
        }
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .terminate_app(&device, &bundle_id, &cancel)
                .await
                .map(|_| Outcome::AppTerminated)
                .map_err(|error| error.to_string())
        });
        cx.notify();
    }
    fn enable_device_input(&mut self, cx: &mut Context<Self>) {
        if self.device.grant.is_some() {
            self.device.retire();
            self.device.message = "Input revoked. Capture again to resume viewing.".into();
            cx.notify();
            return;
        }
        if self.device.busy || self.device.image.is_none() {
            return;
        }
        let Some(device) = self.device.target().cloned() else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .approve_input(&device, &DeviceInputConsent::user_approved(), &cancel)
                .await
                .map(Outcome::InputApproved)
                .map_err(|e| e.to_string())
        });
        cx.notify();
    }
    fn send_device_input(&mut self, input: DeviceInput, cx: &mut Context<Self>) {
        if self.device.busy || self.panel != Panel::Device {
            return;
        }
        let Some(grant) = self.device.grant.clone() else {
            return;
        };
        let Some((width, height)) = self.device.dimensions else {
            return;
        };
        if self
            .device
            .captured
            .is_none_or(|time| time.elapsed() > Duration::from_secs(10))
        {
            self.device.error = Some("Capture a fresh frame before sending input. The last frame is more than 10 seconds old.".into());
            cx.notify();
            return;
        }
        let Some(device) = self.device.target().cloned() else {
            return;
        };
        let tools = match self.device_tools() {
            Ok(tools) => tools,
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        };
        let cancel = self.device.cancel.clone();
        self.device_job(async move {
            tools
                .input(&device, input, &grant, width, height, &cancel)
                .await
                .map(|_| Outcome::InputSent)
                .map_err(|e| e.to_string())
        });
        cx.notify();
    }
    pub(super) fn device_reply(&mut self, reply: Reply, cx: &mut Context<Self>) {
        if reply.epoch != self.device.epoch {
            return;
        }
        self.device.busy = false;
        match reply.result {
            Ok(Outcome::Discovery(devices)) => {
                self.device.message = if devices.is_empty() {
                    "No devices reported. Start an Android emulator externally or connect and authorize a device.".into()
                } else {
                    format!(
                        "{} targets reported. Input is off until explicitly enabled for a selected Android device.",
                        devices.len()
                    )
                };
                self.device.devices = reconcile_devices(&self.device.devices, devices);
            }
            Ok(Outcome::Capture(capture)) => {
                let dimensions = (capture.width, capture.height);
                // A rotation invalidates a grant: never reinterpret an old target
                // coordinate system as authority over the new orientation.
                if self
                    .device
                    .dimensions
                    .is_some_and(|previous| previous != dimensions)
                {
                    self.device.grant = None;
                }
                self.device.message = format!(
                    "{} x {} pixels | {} | Screenshot, not a video stream",
                    capture.width,
                    capture.height,
                    capture.orientation()
                );
                self.device.dimensions = Some(dimensions);
                self.device.image_bytes =
                    (capture.png.len() <= MAX_ATTACHMENT_BATCH_BYTES).then(|| capture.png.clone());
                self.device.image = Some(Arc::new(gpui::Image::from_bytes(
                    gpui::ImageFormat::Png,
                    capture.png,
                )));
                self.device.captured = Some(Instant::now());
            }
            Ok(Outcome::InputApproved(grant)) => {
                self.device.grant = Some(Arc::new(grant));
                self.device.message = "Input enabled for this selected Android target only. Escape or Disable input revokes it.".into();
            }
            Ok(Outcome::InputSent) => self.capture_device(cx),
            Ok(Outcome::Lifecycle) => {
                self.refresh_devices(cx);
                return;
            }
            Ok(Outcome::UrlOpened) => {
                self.device.retire();
                self.device.message =
                    "URL opened in the selected simulator. Capture to inspect its screen.".into();
            }
            Ok(Outcome::AppLaunched) => {
                self.device.retire();
                self.device.message =
                    "App launched in the selected simulator. Capture to inspect its screen.".into();
            }
            Ok(Outcome::AppInstalled) => {
                self.device.retire();
                self.device.message =
                    "App installed in the selected simulator. Launch remains explicit.".into();
            }
            Ok(Outcome::AppTerminated) => {
                self.device.retire();
                self.device.message =
                    "App terminated in the selected simulator. Capture to inspect its screen."
                        .into();
            }
            Err(error) => {
                self.fail_device(error, cx);
                return;
            }
        }
        cx.notify();
    }
    fn fail_device(&mut self, error: String, cx: &mut Context<Self>) {
        self.device.retire();
        // This also covers a helper disappearing before a command can start.
        // Old metadata stays inspectable but never remains actionable.
        for device in &mut self.device.devices {
            device.stale();
        }
        self.device.error = Some(error);
        self.device.message =
            "Operation failed. Refresh to reconnect and re-check device state. Input is off."
                .into();
        cx.notify();
    }
    pub(super) fn tick_devices(&mut self, cx: &mut Context<Self>) {
        let visible = self.panel == Panel::Device
            && (!self.zen_active() || self.settings.personalization.tools_shown);
        if !visible {
            if self.device.active || self.device.busy || self.device.grant.is_some() {
                self.device.retire();
                self.device.message =
                    "Viewer paused. Capture to resume. The device itself was left running.".into();
                cx.notify();
            }
            return;
        }
        if self.settings.value.device.auto_capture
            && self.device.active
            && !self.device.busy
            && self
                .device
                .captured
                .is_some_and(|time| time.elapsed() >= Duration::from_secs(2))
        {
            self.capture_device(cx);
        }
    }
    pub(super) fn device_panel(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let target = self.device.target();
        let ready = target.is_some_and(|d| d.availability == DeviceAvailability::Ready);
        let tools = self.device_tools().ok();
        let can_boot = target
            .zip(tools.as_ref())
            .is_some_and(|(device, tools)| tools.can_boot(device));
        let can_stop = target
            .zip(tools.as_ref())
            .is_some_and(|(device, tools)| tools.can_shutdown(device));
        let bounds = self.device.bounds.clone();
        let mut viewer = div()
            .id("device-frame")
            .role(gpui::Role::Group)
            .aria_label("Device screenshot. Enable input before clicking or using arrow keys.")
            .tab_index(0)
            .track_focus(&self.device.focus)
            .relative()
            .flex_1()
            .min_h(px(120.))
            .min_w_0()
            .overflow_hidden()
            .child(
                gpui::canvas(move |area, _, _| bounds.set(area), |_, _, _, _| {})
                    .absolute()
                    .inset_0()
                    .size_full(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseDownEvent, window, cx| {
                    window.focus(&this.device.focus, cx);
                    let Some((width, height)) = this.device.dimensions else {
                        return;
                    };
                    let bounds = this.device.bounds.get();
                    if let Some((x, y)) = device_viewport_point(
                        f32::from(event.position.x - bounds.origin.x),
                        f32::from(event.position.y - bounds.origin.y),
                        f32::from(bounds.size.width),
                        f32::from(bounds.size.height),
                        width,
                        height,
                    ) {
                        this.send_device_input(DeviceInput::Tap { x, y }, cx);
                    }
                }),
            )
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _, cx| {
                if event.is_held {
                    return;
                }
                if event.keystroke.key == "escape" {
                    this.device.retire();
                    this.device.message = "Input revoked and capture paused.".into();
                    cx.notify();
                    cx.stop_propagation();
                    return;
                }
                let modifiers = event.keystroke.modifiers;
                if modifiers.control || modifiers.platform || modifiers.alt || modifiers.shift {
                    return;
                }
                if this.device.grant.is_some()
                    && matches!(
                        event.keystroke.key.as_str(),
                        "up" | "down" | "left" | "right" | "enter" | "backspace"
                    )
                {
                    this.send_device_input(
                        DeviceInput::Key {
                            key: event.keystroke.key.to_string(),
                        },
                        cx,
                    );
                    cx.stop_propagation();
                }
            }));
        viewer = if let Some(image) = self.device.image.as_ref() {
            viewer.child(
                gpui::img(image.clone())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .object_fit(gpui::ObjectFit::Contain),
            )
        } else {
            viewer.child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .p_4()
                    .text_color(rgb(palette().muted))
                    .child("No captured frame. Select a ready device, then Capture."),
            )
        };
        let mut root = div().id("device-view").flex_1().min_h_0().min_w_0().flex().flex_col().gap_2().p_3()
            .child(div().flex().flex_wrap().items_center().gap_2()
                .child(ui::action("device-refresh", "Refresh", Some(Glyph::Restore), false, cx.listener(|this, _: &(), _, cx| this.refresh_devices(cx))))
                .child(ui::action("device-settings", "Device settings", Some(Glyph::Settings), false, cx.listener(|this, _: &(), _, cx| {
                    this.set_panel(Panel::Settings, cx); this.open_settings_section(settings::Section::Device, cx);
                })))
                .child(ui::action("device-detach", "Disconnect viewer", Some(Glyph::Close), false, cx.listener(|this, _: &(), _, cx| {
                    this.device.retire(); this.device.selected = None; this.device.message = "Viewer disconnected. Devices were left running.".into(); cx.notify();
                }))))
            .child(div().id("device-list").max_h(px(160.)).overflow_y_scroll().flex().flex_col()
                .children(self.device.devices.iter().enumerate().map(|(index, device)| {
                    let id = device.descriptor.id.clone();
                    let kind = match device.descriptor.kind { DeviceKind::Physical => "Physical", DeviceKind::Simulator => "Simulator/emulator", DeviceKind::Unknown => "Hardware type unknown" };
                    ui::action(("device-row", index), format!("{} | {} | {}", device.descriptor.name, kind, device.availability.label()), Some(Glyph::Window), Some(&device.descriptor.id) == self.device.selected.as_ref(),
                        cx.listener(move |this, _: &(), _, cx| this.select_device(id.clone(), cx))).text_size(px(12.))
                })))
            .children(target.map(|device| div().text_size(px(11.)).text_color(rgb(palette().muted))
                .child(format!("{} | {} | {}", device.descriptor.platform, device.descriptor.id.as_str(), device.runtime.as_deref().unwrap_or("OS version not reported")))))
            .child(div().text_size(px(12.)).child(self.device.message.clone()))
            .children(self.device.error.as_ref().map(|error| div().text_size(px(12.)).text_color(rgb(palette().error)).child(error.clone())))
            .children(self.device.busy.then(|| div().text_size(px(12.)).child("Working... Refresh or Disconnect cancels the current request.")))
            .child(div().flex().flex_wrap().gap_2()
                .children((ready && !self.device.busy).then(|| ui::action("device-capture", "Capture", Some(Glyph::Capture), false, cx.listener(|this, _: &(), _, cx| this.capture_device(cx)))))
                .children((self.device.image_bytes.is_some() && self.selected.is_some() && !self.device.busy).then(||
                    ui::action("device-attach-frame","Attach frame to selected conversation",Some(Glyph::Attach),false,
                        cx.listener(|this,_:&(),_,cx| {
                            if let Some(bytes)=this.device.image_bytes.clone() {
                                this.attach_capture("device-capture.png".into(),bytes,ImageSource::DeviceCapture,cx);
                                this.set_panel(Panel::Conversation,cx);
                            }
                        })).relative().child(ui::layout_probe("device-attach-frame"))))
                .children((can_boot && !self.device.busy).then(|| ui::action("device-boot", "Boot simulator", None, false, cx.listener(|this, _: &(), _, cx| this.device_running(true, cx)))))
                .children((can_stop && !self.device.busy).then(|| ui::action("device-stop", if self.device.shutdown_confirmation { "Confirm shutdown" } else { "Shut down simulator" }, None, false, cx.listener(|this, _: &(), _, cx| this.device_running(false, cx)))))
                .children((ready && self.device.image.is_some() && self.settings.value.device.backend == DeviceBackend::Android && !self.device.busy).then(|| ui::action("device-consent", if self.device.grant.is_some() { "Disable input" } else { "Enable input for this device" }, None, self.device.grant.is_some(), cx.listener(|this, _: &(), _, cx| this.enable_device_input(cx))))))
            .children((ready && self.settings.value.device.backend == DeviceBackend::AppleSimulator).then(||
                div().flex().items_center().gap_2()
                    .child(div().text_size(px(12.)).child("Web URL"))
                    .child(self.device.url.clone())
                    .child(ui::action("device-open-url", "Open URL", None, false,
                        cx.listener(|this, _: &(), _, cx| this.open_device_url(cx))))))
            .children((ready && self.settings.value.device.backend == DeviceBackend::AppleSimulator).then(||
                div().flex().items_center().gap_2()
                    .child(div().text_size(px(12.)).child("Bundle ID"))
                    .child(self.device.bundle_id.clone())
                    .child(ui::action("device-launch-app", "Launch installed app", None, false,
                        cx.listener(|this, _: &(), _, cx| this.launch_device_app(cx))))
                    .child(ui::action("device-terminate-app", "Terminate app", None, false,
                        cx.listener(|this, _: &(), _, cx| this.terminate_device_app(cx))))))
            .children((ready && self.settings.value.device.backend == DeviceBackend::AppleSimulator).then(||
                div().flex().items_center().gap_2()
                    .child(div().text_size(px(12.)).child("Local app bundle"))
                    .child(self.device.app_path.clone())
                    .child(ui::action("device-install-app", if self.device.install_confirmation.is_some() { "Confirm install" } else { "Install app..." }, None, false,
                        cx.listener(|this, _: &(), _, cx| this.install_device_app(cx))))))
            .children(self.device.shutdown_confirmation.then(|| div().text_size(px(12.)).child("Shutdown stops the simulator, including work started outside Synara. Select another device or Disconnect to cancel.")))
            .child(viewer);
        if self.device.grant.is_some() {
            root = root
                .child(
                    div().flex().flex_wrap().gap_2().children(
                        [
                            ("home", "Home"),
                            ("back", "Back"),
                            ("enter", "Enter"),
                            ("backspace", "Backspace"),
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(index, (key, label))| {
                            ui::action(
                                ("device-key", index),
                                label,
                                None,
                                false,
                                cx.listener(move |this, _: &(), _, cx| {
                                    this.send_device_input(DeviceInput::Key { key: key.into() }, cx)
                                }),
                            )
                        }),
                    ),
                )
                .child(
                    div().flex().gap_2().children(
                        [(true, "Swipe up"), (false, "Swipe down")]
                            .into_iter()
                            .enumerate()
                            .map(|(index, (up, label))| {
                                ui::action(
                                    ("device-swipe", index),
                                    label,
                                    None,
                                    false,
                                    cx.listener(move |this, _: &(), _, cx| {
                                        let Some((width, height)) = this.device.dimensions else {
                                            return;
                                        };
                                        let (a, b) = (height / 4, height * 3 / 4);
                                        this.send_device_input(
                                            DeviceInput::Swipe {
                                                from_x: width / 2,
                                                to_x: width / 2,
                                                from_y: if up { b } else { a },
                                                to_y: if up { a } else { b },
                                                duration_ms: 300,
                                            },
                                            cx,
                                        );
                                    }),
                                )
                            }),
                    ),
                );
        }
        root.child(div().text_size(px(11.)).text_color(rgb(palette().muted)).child("Input is never restored or exposed to agents. Captures stay in memory unless explicitly attached (2 MiB maximum). Attaching does not send or grant input authority. Apple input, physical iOS devices and Android cold boot are not implemented."))
            .into_any_element()
    }
}
