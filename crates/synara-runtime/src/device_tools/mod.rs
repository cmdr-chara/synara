//! Real device CLI adapters behind the existing runtime device boundary.
//!
//! These are not agent tools. No helper starts during settings load, discovery
//! never grants input, and neither discovery nor a screenshot proves native
//! input support. Physical Apple devices and Android cold boot are unsupported.
mod apple;
mod apple_helper;
pub(crate) mod command;
mod recording;
mod tests;
use crate::{
    DeviceDescriptor, DeviceId, DeviceInput, DeviceInputConsent, DeviceKind, DeviceState,
    RuntimeError, validate_discovery,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
pub use tokio_util::sync::CancellationToken as DeviceCancellation;

const DISCOVERY_LIMIT: usize = 1024 * 1024;
const CAPTURE_LIMIT: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceBackend {
    #[default]
    Android,
    AppleSimulator,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceAvailability {
    Ready,
    Stopped,
    Unauthorized,
    Disconnected,
    Unsupported,
}
impl DeviceAvailability {
    pub fn label(self) -> &'static str {
        match self {
            Self::Ready => "Ready",
            Self::Stopped => "Stopped",
            Self::Unauthorized => "Authorize on the device",
            Self::Disconnected => "Disconnected / stale",
            Self::Unsupported => "Unsupported runtime",
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolDevice {
    pub descriptor: DeviceDescriptor,
    pub availability: DeviceAvailability,
    pub runtime: Option<String>,
}
impl ToolDevice {
    pub fn stale(&mut self) {
        self.availability = DeviceAvailability::Disconnected;
        self.descriptor.state = DeviceState::Disconnected;
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceUiFrame {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceUiPoint {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceUiNode {
    pub role: String,
    #[serde(default)]
    pub subrole: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub identifier: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub frame: DeviceUiFrame,
    #[serde(default)]
    pub activation_point: Option<DeviceUiPoint>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub children: Vec<DeviceUiNode>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceAccessibilityTree {
    pub point_width: f64,
    pub point_height: f64,
    pub root: DeviceUiNode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceAccessibilityTarget {
    pub label: String,
    pub role: String,
    pub value: Option<String>,
}

impl DeviceAccessibilityTree {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if !self.point_width.is_finite()
            || !self.point_height.is_finite()
            || self.point_width <= 0.0
            || self.point_height <= 0.0
            || self.point_width > 20_000.0
            || self.point_height > 20_000.0
        {
            return Err(RuntimeError::Invalid(
                "invalid accessibility display geometry".into(),
            ));
        }
        let mut count = 0usize;
        validate_ui_node(&self.root, 0, &mut count)
    }

    pub fn targets(&self, limit: usize) -> Vec<DeviceAccessibilityTarget> {
        let mut out = Vec::new();
        collect_ui_targets(&self.root, limit.min(128), &mut out);
        out
    }

    pub fn semantic_pixel_point(
        &self,
        label: &str,
        role: Option<&str>,
        pixel_width: u32,
        pixel_height: u32,
    ) -> Option<(u32, u32)> {
        if pixel_width == 0 || pixel_height == 0 || label.trim().is_empty() {
            return None;
        }
        let node = find_ui_node(&self.root, label.trim(), role.map(str::trim))?;
        let point = node.activation_point.clone().unwrap_or(DeviceUiPoint {
            x: node.frame.x + node.frame.width / 2.0,
            y: node.frame.y + node.frame.height / 2.0,
        });
        if !point.x.is_finite()
            || !point.y.is_finite()
            || point.x < 0.0
            || point.y < 0.0
            || point.x > self.point_width
            || point.y > self.point_height
        {
            return None;
        }
        let x = ((point.x / self.point_width) * f64::from(pixel_width))
            .floor()
            .clamp(0.0, f64::from(pixel_width.saturating_sub(1))) as u32;
        let y = ((point.y / self.point_height) * f64::from(pixel_height))
            .floor()
            .clamp(0.0, f64::from(pixel_height.saturating_sub(1))) as u32;
        Some((x, y))
    }
}

fn bounded_ui_text(value: &str, max: usize) -> bool {
    value.len() <= max && !value.chars().any(|character| character == '\0')
}

fn validate_ui_node(
    node: &DeviceUiNode,
    depth: usize,
    count: &mut usize,
) -> Result<(), RuntimeError> {
    *count = count.checked_add(1).ok_or(RuntimeError::Limit)?;
    if *count > 4096 || depth > 64 || !bounded_ui_text(&node.role, 128) {
        return Err(RuntimeError::Limit);
    }
    for value in [
        node.subrole.as_deref(),
        node.label.as_deref(),
        node.value.as_deref(),
        node.identifier.as_deref(),
        node.title.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !bounded_ui_text(value, 1024) {
            return Err(RuntimeError::Limit);
        }
    }
    for number in [
        node.frame.x,
        node.frame.y,
        node.frame.width,
        node.frame.height,
    ] {
        if !number.is_finite() || number.abs() > 100_000.0 {
            return Err(RuntimeError::Invalid(
                "invalid accessibility frame".into(),
            ));
        }
    }
    if node.frame.width < 0.0 || node.frame.height < 0.0 || node.children.len() > 512 {
        return Err(RuntimeError::Invalid(
            "invalid accessibility node".into(),
        ));
    }
    if let Some(point) = &node.activation_point
        && (!point.x.is_finite()
            || !point.y.is_finite()
            || point.x.abs() > 100_000.0
            || point.y.abs() > 100_000.0)
    {
        return Err(RuntimeError::Invalid(
            "invalid accessibility activation point".into(),
        ));
    }
    for child in &node.children {
        validate_ui_node(child, depth + 1, count)?;
    }
    Ok(())
}

fn node_label(node: &DeviceUiNode) -> Option<&str> {
    node.label
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| node.title.as_deref().filter(|value| !value.trim().is_empty()))
        .or_else(|| {
            node.identifier
                .as_deref()
                .filter(|value| !value.trim().is_empty())
        })
}

fn collect_ui_targets(
    node: &DeviceUiNode,
    limit: usize,
    out: &mut Vec<DeviceAccessibilityTarget>,
) {
    if out.len() >= limit {
        return;
    }
    if let Some(label) = node_label(node)
        && node.enabled.unwrap_or(true)
        && node.frame.width > 0.0
        && node.frame.height > 0.0
    {
        out.push(DeviceAccessibilityTarget {
            label: label.to_owned(),
            role: node.role.clone(),
            value: node.value.clone(),
        });
    }
    for child in &node.children {
        collect_ui_targets(child, limit, out);
        if out.len() >= limit {
            break;
        }
    }
}

fn find_ui_node<'a>(
    node: &'a DeviceUiNode,
    label: &str,
    role: Option<&str>,
) -> Option<&'a DeviceUiNode> {
    let label_matches = node_label(node).is_some_and(|candidate| candidate.eq_ignore_ascii_case(label));
    let role_matches = role
        .filter(|value| !value.is_empty())
        .is_none_or(|expected| node.role.eq_ignore_ascii_case(expected));
    if label_matches && role_matches && node.enabled.unwrap_or(true) {
        return Some(node);
    }
    node.children
        .iter()
        .find_map(|child| find_ui_node(child, label, role))
}

/// A user-configured executable, not a PATH lookup or downloaded helper.
#[derive(Clone, Debug)]
pub struct DeviceTools {
    pub backend: DeviceBackend,
    executable: PathBuf,
    apple_helper: Option<PathBuf>,
}
impl DeviceTools {
    pub fn new(
        backend: DeviceBackend,
        adb: Option<&Path>,
        apple_helper: Option<&Path>,
    ) -> Result<Self, RuntimeError> {
        let executable = match backend {
            DeviceBackend::Android => adb
                .ok_or_else(|| {
                    RuntimeError::Unsupported(
                        "Select your Android SDK adb executable in Device settings".into(),
                    )
                })?
                .to_path_buf(),
            DeviceBackend::AppleSimulator => apple::executable()?,
        };
        if !executable.is_absolute() || !executable.is_file() {
            return Err(RuntimeError::Unsupported(
                "The configured device helper is missing or is not an absolute executable path"
                    .into(),
            ));
        }
        Ok(Self {
            backend,
            executable,
            apple_helper: apple_helper.map(Path::to_path_buf),
        })
    }
    pub async fn discover(
        &self,
        cancel: &CancellationToken,
    ) -> Result<Vec<ToolDevice>, RuntimeError> {
        let args = match self.backend {
            DeviceBackend::Android => vec!["devices".into(), "-l".into()],
            DeviceBackend::AppleSimulator => apple::discovery_args(),
        };
        let bytes = command::run(&self.executable, args, DISCOVERY_LIMIT, cancel).await?;
        match self.backend {
            DeviceBackend::Android => parse_adb(&bytes),
            DeviceBackend::AppleSimulator => apple::parse(&bytes),
        }
    }
    fn address(&self, device: &ToolDevice) -> Result<String, RuntimeError> {
        device.descriptor.validate()?;
        let address = device.descriptor.id.as_str();
        let valid = match self.backend {
            DeviceBackend::Android => {
                valid_adb_serial(address) && device.descriptor.platform == "Android"
            }
            DeviceBackend::AppleSimulator => {
                apple::valid_id(address) && device.descriptor.platform == "iOS Simulator"
            }
        };
        if !valid {
            return Err(RuntimeError::Invalid(
                "device identity does not belong to this helper".into(),
            ));
        }
        Ok(address.into())
    }
    pub async fn capture(
        &self,
        device: &ToolDevice,
        cancel: &CancellationToken,
    ) -> Result<Vec<u8>, RuntimeError> {
        let id = self.address(device)?;
        if device.availability != DeviceAvailability::Ready {
            return Err(RuntimeError::Closed);
        }
        let args = match self.backend {
            DeviceBackend::Android => vec![
                "-s".into(),
                id,
                "exec-out".into(),
                "screencap".into(),
                "-p".into(),
            ],
            DeviceBackend::AppleSimulator => apple::capture_args(id),
        };
        let bytes = command::run(&self.executable, args, CAPTURE_LIMIT, cancel).await?;
        if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(RuntimeError::Invalid(
                "helper returned no PNG capture".into(),
            ));
        }
        Ok(bytes)
    }
    pub fn can_boot(&self, device: &ToolDevice) -> bool {
        self.backend == DeviceBackend::AppleSimulator
            && device.availability == DeviceAvailability::Stopped
    }
    pub fn can_shutdown(&self, device: &ToolDevice) -> bool {
        device.availability == DeviceAvailability::Ready
            && (self.backend == DeviceBackend::AppleSimulator
                || (device.descriptor.kind == DeviceKind::Simulator
                    && is_android_emulator(device.descriptor.id.as_str())))
    }
    pub async fn set_running(
        &self,
        device: &ToolDevice,
        running: bool,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        let id = self.address(device)?;
        if !(if running {
            self.can_boot(device)
        } else {
            self.can_shutdown(device)
        }) {
            return Err(RuntimeError::Unsupported(
                "This target does not support the requested lifecycle action".into(),
            ));
        }
        let args = match self.backend {
            DeviceBackend::AppleSimulator => apple::lifecycle_args(id, running),
            DeviceBackend::Android => vec!["-s".into(), id, "emu".into(), "kill".into()],
        };
        command::run(&self.executable, args, DISCOVERY_LIMIT, cancel).await?;
        Ok(())
    }

    /// Validate a user-entered web URL without touching the simulator helper.
    pub fn validate_open_url(
        &self,
        device: &ToolDevice,
        raw_url: &str,
    ) -> Result<String, RuntimeError> {
        if self.backend != DeviceBackend::AppleSimulator {
            return Err(RuntimeError::Unsupported(
                "Opening a URL is available for iOS Simulator only".into(),
            ));
        }
        self.address(device)?;
        if device.availability != DeviceAvailability::Ready {
            return Err(RuntimeError::Closed);
        }
        if raw_url.len() > 2_048
            || raw_url.trim() != raw_url
            || raw_url.chars().any(char::is_control)
        {
            return Err(RuntimeError::Invalid("Invalid simulator URL".into()));
        }
        let url = url::Url::parse(raw_url)
            .map_err(|_| RuntimeError::Invalid("Invalid simulator URL".into()))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(RuntimeError::Invalid(
                "Use an HTTP(S) URL without credentials".into(),
            ));
        }
        Ok(url.to_string())
    }

    /// Open an explicit web URL in a booted iOS Simulator. This is a local,
    /// user-initiated helper action and does not grant screenshot/input authority.
    pub async fn open_url(
        &self,
        device: &ToolDevice,
        raw_url: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        let url = self.validate_open_url(device, raw_url)?;
        let id = self.address(device)?;
        command::run(
            &self.executable,
            apple::open_url_args(id, url),
            DISCOVERY_LIMIT,
            cancel,
        )
        .await?;
        Ok(())
    }

    /// Validate an installed-app bundle ID without touching the simulator helper.
    pub fn validate_launch_app(
        &self,
        device: &ToolDevice,
        bundle_id: &str,
    ) -> Result<(), RuntimeError> {
        if self.backend != DeviceBackend::AppleSimulator {
            return Err(RuntimeError::Unsupported(
                "Launching an app is available for iOS Simulator only".into(),
            ));
        }
        self.address(device)?;
        if device.availability != DeviceAvailability::Ready {
            return Err(RuntimeError::Closed);
        }
        if !apple::valid_bundle_id(bundle_id) {
            return Err(RuntimeError::Invalid(
                "Invalid installed app bundle ID".into(),
            ));
        }
        Ok(())
    }

    /// Launch an already installed app in a selected booted iOS Simulator.
    pub async fn launch_app(
        &self,
        device: &ToolDevice,
        bundle_id: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        self.validate_launch_app(device, bundle_id)?;
        let id = self.address(device)?;
        command::run(
            &self.executable,
            apple::launch_args(id, bundle_id.to_owned()),
            DISCOVERY_LIMIT,
            cancel,
        )
        .await?;
        Ok(())
    }
    /// A reviewed, explicit local application bundle, not an archive or URL.
    pub fn validate_install_app(
        &self,
        device: &ToolDevice,
        path: &Path,
    ) -> Result<(), RuntimeError> {
        if self.backend != DeviceBackend::AppleSimulator {
            return Err(RuntimeError::Unsupported(
                "App installation is available for iOS Simulator only".into(),
            ));
        }
        self.address(device)?;
        if device.availability != DeviceAvailability::Ready {
            return Err(RuntimeError::Closed);
        }
        let Some(text) = path.to_str() else {
            return Err(RuntimeError::Invalid("Use a UTF-8 app bundle path".into()));
        };
        if !path.is_absolute()
            || text.len() > 4096
            || text.chars().any(char::is_control)
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
            || !path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("app"))
        {
            return Err(RuntimeError::Invalid(
                "Choose an absolute local .app bundle without parent traversal".into(),
            ));
        }
        Ok(())
    }
    /// Installation never launches the app or grants device input. The OS may
    /// finish an already-queued install after helper cancellation; no rollback is claimed.
    pub async fn install_app(
        &self,
        device: &ToolDevice,
        path: &Path,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        self.validate_install_app(device, path)?;
        if cancel.is_cancelled() {
            return Err(RuntimeError::Closed);
        }
        let metadata = tokio::fs::symlink_metadata(path).await?;
        let plist = tokio::fs::symlink_metadata(path.join("Info.plist")).await?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || !plist.is_file()
            || plist.file_type().is_symlink()
            || plist.len() > 1024 * 1024
        {
            return Err(RuntimeError::Invalid(
                "Use a real .app directory with a regular Info.plist of at most 1 MiB".into(),
            ));
        }
        let path = tokio::fs::canonicalize(path).await?;
        let path = path
            .to_str()
            .ok_or_else(|| RuntimeError::Invalid("Use a UTF-8 app bundle path".into()))?
            .to_owned();
        command::run(
            &self.executable,
            apple::install_args(self.address(device)?, path),
            DISCOVERY_LIMIT,
            cancel,
        )
        .await?;
        Ok(())
    }
    pub async fn terminate_app(
        &self,
        device: &ToolDevice,
        bundle_id: &str,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        self.validate_launch_app(device, bundle_id)?;
        command::run(
            &self.executable,
            apple::terminate_args(self.address(device)?, bundle_id.to_owned()),
            DISCOVERY_LIMIT,
            cancel,
        )
        .await?;
        Ok(())
    }
    fn configured_apple_helper(&self) -> Result<&Path, RuntimeError> {
        let helper = self.apple_helper.as_deref().ok_or_else(|| {
            RuntimeError::Unsupported(
                "Select a trusted synara-device-helper executable in Device settings".into(),
            )
        })?;
        if !helper.is_absolute() || !helper.is_file() {
            return Err(RuntimeError::Unsupported(
                "The configured Apple device helper is missing or is not an absolute executable path"
                    .into(),
            ));
        }
        Ok(helper)
    }

    pub fn can_input(&self, device: &ToolDevice) -> bool {
        device.availability == DeviceAvailability::Ready
            && match self.backend {
                DeviceBackend::Android => true,
                DeviceBackend::AppleSimulator => self
                    .apple_helper
                    .as_deref()
                    .is_some_and(|path| path.is_absolute() && path.is_file()),
            }
    }

    pub fn can_inspect_accessibility(&self, device: &ToolDevice) -> bool {
        self.backend == DeviceBackend::AppleSimulator
            && device.availability == DeviceAvailability::Ready
            && self
                .apple_helper
                .as_deref()
                .is_some_and(|path| path.is_absolute() && path.is_file())
    }

    /// Probe the selected native input owner, then scope authority to this
    /// exact device/backend/helper tuple. Grants are intentionally
    /// non-serializable and are never restored.
    pub async fn approve_input(
        &self,
        device: &ToolDevice,
        _consent: &DeviceInputConsent,
        cancel: &CancellationToken,
    ) -> Result<DeviceInputGrant, RuntimeError> {
        let id = self.address(device)?;
        if device.availability != DeviceAvailability::Ready {
            return Err(RuntimeError::Closed);
        }
        let helper = match self.backend {
            DeviceBackend::Android => {
                command::run(
                    &self.executable,
                    vec![
                        "-s".into(),
                        id.clone(),
                        "shell".into(),
                        "test".into(),
                        "-x".into(),
                        "/system/bin/input".into(),
                    ],
                    4096,
                    cancel,
                )
                .await?;
                None
            }
            DeviceBackend::AppleSimulator => {
                let helper = self.configured_apple_helper()?.to_path_buf();
                let attached = apple_helper::probe(&helper, &id, cancel).await?;
                if !attached.capabilities.input {
                    return Err(RuntimeError::Unsupported(
                        "The selected Simulator helper reports no HID input capability".into(),
                    ));
                }
                Some(helper)
            }
        };
        Ok(DeviceInputGrant {
            backend: self.backend,
            device: id,
            executable: self.executable.clone(),
            apple_helper: helper,
        })
    }

    pub async fn input(
        &self,
        device: &ToolDevice,
        input: DeviceInput,
        grant: &DeviceInputGrant,
        width: u32,
        height: u32,
        cancel: &CancellationToken,
    ) -> Result<(), RuntimeError> {
        let id = self.address(device)?;
        if device.availability != DeviceAvailability::Ready
            || id != grant.device
            || self.backend != grant.backend
            || self.executable != grant.executable
        {
            return Err(RuntimeError::Denied(
                "Device input authority is missing or belongs to another target".into(),
            ));
        }
        input.validate()?;
        match self.backend {
            DeviceBackend::Android => {
                if grant.apple_helper.is_some() {
                    return Err(RuntimeError::Denied(
                        "Device input authority belongs to another backend".into(),
                    ));
                }
                let mut args =
                    vec!["-s".into(), id, "shell".into(), "/system/bin/input".into()];
                args.extend(android_input_args(&input, width, height)?);
                command::run(&self.executable, args, 4096, cancel).await?;
            }
            DeviceBackend::AppleSimulator => {
                if width == 0 || height == 0 || width > 16_384 || height > 16_384 {
                    return Err(RuntimeError::Limit);
                }
                let helper = grant.apple_helper.as_deref().ok_or_else(|| {
                    RuntimeError::Denied(
                        "Simulator input authority has no native helper".into(),
                    )
                })?;
                if self.configured_apple_helper()? != helper {
                    return Err(RuntimeError::Denied(
                        "Simulator helper changed after input was approved".into(),
                    ));
                }
                let normalized = |value: u32, bound: u32| -> Result<f64, RuntimeError> {
                    if value >= bound {
                        return Err(RuntimeError::Invalid(
                            "Input coordinates are outside the latest capture".into(),
                        ));
                    }
                    Ok(f64::from(value) / f64::from(bound))
                };
                let (method, params) = match input {
                    DeviceInput::Tap { x, y } => (
                        "tap",
                        json!({
                            "x": normalized(x, width)?,
                            "y": normalized(y, height)?,
                        }),
                    ),
                    DeviceInput::Swipe {
                        from_x,
                        from_y,
                        to_x,
                        to_y,
                        duration_ms,
                    } => {
                        if duration_ms > 10_000 {
                            return Err(RuntimeError::Limit);
                        }
                        (
                            "swipe",
                            json!({
                                "startX": normalized(from_x, width)?,
                                "startY": normalized(from_y, height)?,
                                "endX": normalized(to_x, width)?,
                                "endY": normalized(to_y, height)?,
                                "durationMs": duration_ms,
                            }),
                        )
                    }
                    DeviceInput::Text { text } => ("text", json!({ "text": text })),
                    DeviceInput::Key { key } => (
                        "key",
                        json!({ "usage": apple_key_usage(&key)? }),
                    ),
                    DeviceInput::Button { button } => (
                        "button",
                        json!({ "name": button }),
                    ),
                };
                let (attached, _) = apple_helper::invoke(helper, &id, method, params, cancel).await?;
                if !attached.capabilities.input {
                    return Err(RuntimeError::Unsupported(
                        "The Simulator helper lost HID input capability".into(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub async fn describe_ui(
        &self,
        device: &ToolDevice,
        cancel: &CancellationToken,
    ) -> Result<DeviceAccessibilityTree, RuntimeError> {
        if self.backend != DeviceBackend::AppleSimulator
            || device.availability != DeviceAvailability::Ready
        {
            return Err(RuntimeError::Unsupported(
                "Accessibility inspection is available for a booted iOS Simulator only".into(),
            ));
        }
        let id = self.address(device)?;
        let helper = self.configured_apple_helper()?;
        let (attached, result) =
            apple_helper::invoke(helper, &id, "describe-ui", json!({ "maxDepth": 40 }), cancel)
                .await?;
        if !attached.capabilities.accessibility {
            return Err(RuntimeError::Unsupported(
                "The selected Simulator helper reports no accessibility capability".into(),
            ));
        }
        let tree_value = result
            .get("tree")
            .cloned()
            .unwrap_or(result);
        let root: DeviceUiNode = serde_json::from_value(tree_value)
            .map_err(|_| RuntimeError::Invalid("invalid Simulator accessibility tree".into()))?;
        let tree = DeviceAccessibilityTree {
            point_width: attached.point_width,
            point_height: attached.point_height,
            root,
        };
        tree.validate()?;
        Ok(tree)
    }
}
#[derive(Debug)]
pub struct DeviceInputGrant {
    backend: DeviceBackend,
    device: String,
    executable: PathBuf,
    apple_helper: Option<PathBuf>,
}

fn apple_key_usage(key: &str) -> Result<u16, RuntimeError> {
    match key {
        "enter" => Ok(0x28),
        "escape" => Ok(0x29),
        "backspace" => Ok(0x2a),
        "tab" => Ok(0x2b),
        "space" => Ok(0x2c),
        "right" => Ok(0x4f),
        "left" => Ok(0x50),
        "down" => Ok(0x51),
        "up" => Ok(0x52),
        _ => Err(RuntimeError::Unsupported(
            "This Simulator key is not supported".into(),
        )),
    }
}
fn android_input_args(
    input: &DeviceInput,
    width: u32,
    height: u32,
) -> Result<Vec<String>, RuntimeError> {
    input.validate()?;
    if width == 0 || height == 0 || width > 8192 || height > 8192 {
        return Err(RuntimeError::Limit);
    }
    let point = |x: u32, y: u32| x < width && y < height;
    // The Android shell only receives fixed literals and decimal integers, never
    // user text, command names, quotes or shell metacharacters.
    match input {
        DeviceInput::Tap { x, y } if point(*x, *y) => {
            Ok(vec!["tap".into(), x.to_string(), y.to_string()])
        }
        DeviceInput::Swipe {
            from_x,
            from_y,
            to_x,
            to_y,
            duration_ms,
        } if point(*from_x, *from_y) && point(*to_x, *to_y) => Ok(vec![
            "swipe".into(),
            from_x.to_string(),
            from_y.to_string(),
            to_x.to_string(),
            to_y.to_string(),
            duration_ms.to_string(),
        ]),
        DeviceInput::Key { key } => {
            let code = match key.as_str() {
                "home" => "3",
                "back" => "4",
                "enter" => "66",
                "backspace" => "67",
                "up" => "19",
                "down" => "20",
                "left" => "21",
                "right" => "22",
                _ => {
                    return Err(RuntimeError::Unsupported(
                        "This device key is not supported".into(),
                    ));
                }
            };
            Ok(vec!["keyevent".into(), code.into()])
        }
        DeviceInput::Text { text } => {
            if text.is_empty() {
                return Err(RuntimeError::Invalid("Device text must not be empty".into()));
            }
            Ok(vec!["text".into(), text.replace(' ', "%s")])
        }
        DeviceInput::Button { .. } => Err(RuntimeError::Unsupported(
            "Hardware buttons are available for iOS Simulator only".into(),
        )),
        _ => Err(RuntimeError::Invalid(
            "Input coordinates are outside the latest capture".into(),
        )),
    }
}
fn valid_adb_serial(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-[]".contains(&b))
}
fn is_android_emulator(value: &str) -> bool {
    value
        .strip_prefix("emulator-")
        .is_some_and(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()))
}
fn parse_adb(bytes: &[u8]) -> Result<Vec<ToolDevice>, RuntimeError> {
    if bytes.len() > DISCOVERY_LIMIT {
        return Err(RuntimeError::Limit);
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| RuntimeError::Invalid("ADB discovery was not UTF-8".into()))?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    if lines.next().map(str::trim) != Some("List of devices attached") {
        return Err(RuntimeError::Invalid(
            "ADB discovery header is missing".into(),
        ));
    }
    let mut devices = Vec::new();
    for line in lines {
        let fields: Vec<_> = line.split_whitespace().collect();
        if fields.len() < 2 || !valid_adb_serial(fields[0]) {
            return Err(RuntimeError::Invalid("Invalid ADB device entry".into()));
        }
        let availability = match fields[1] {
            "device" => DeviceAvailability::Ready,
            "offline" => DeviceAvailability::Disconnected,
            "unauthorized" | "no" => DeviceAvailability::Unauthorized,
            _ => DeviceAvailability::Unsupported,
        };
        let kind = if is_android_emulator(fields[0]) {
            DeviceKind::Simulator
        } else if fields.iter().any(|field| field.starts_with("usb:")) {
            DeviceKind::Physical
        } else {
            DeviceKind::Unknown
        };
        let name = fields
            .iter()
            .find_map(|f| f.strip_prefix("model:"))
            .unwrap_or(fields[0])
            .replace('_', " ");
        devices.push(ToolDevice {
            descriptor: DeviceDescriptor {
                id: DeviceId::new(fields[0])?,
                name,
                platform: "Android".into(),
                kind,
                state: if availability == DeviceAvailability::Ready {
                    DeviceState::Discovered
                } else {
                    DeviceState::Disconnected
                },
            },
            availability,
            runtime: None,
        });
        if devices.len() > 256 {
            return Err(RuntimeError::Limit);
        }
    }
    validate_discovery(
        &devices
            .iter()
            .map(|d| d.descriptor.clone())
            .collect::<Vec<_>>(),
    )?;
    Ok(devices)
}
/// A successful refresh retains absent rows as stale. Failed refreshes must not
/// be passed here: the caller retains the previous list and displays the error.
pub fn reconcile_devices(previous: &[ToolDevice], mut fresh: Vec<ToolDevice>) -> Vec<ToolDevice> {
    for old in previous {
        if fresh.len() == 256 {
            break;
        }
        if !fresh
            .iter()
            .any(|device| device.descriptor.id == old.descriptor.id)
        {
            let mut stale = old.clone();
            stale.stale();
            fresh.push(stale);
        }
    }
    fresh
}

#[cfg(test)]
mod install_tests;
