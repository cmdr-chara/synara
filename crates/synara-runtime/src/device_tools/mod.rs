//! Real device CLI adapters behind the existing runtime device boundary.
//!
//! These are not agent tools. No helper starts during settings load, discovery
//! never grants input, and neither discovery nor a screenshot proves native
//! input support. Physical Apple devices and Android cold boot are unsupported.
mod apple;
pub(crate) mod command;
mod tests;
use crate::{
    DeviceDescriptor, DeviceId, DeviceInput, DeviceInputConsent, DeviceKind, DeviceState,
    RuntimeError, validate_discovery,
};
use serde::{Deserialize, Serialize};
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

/// A user-configured executable, not a PATH lookup or downloaded helper.
#[derive(Clone, Debug)]
pub struct DeviceTools {
    pub backend: DeviceBackend,
    executable: PathBuf,
}
impl DeviceTools {
    pub fn new(backend: DeviceBackend, adb: Option<&Path>) -> Result<Self, RuntimeError> {
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
    /// Probe the real Android executable, then scope authority to this target.
    /// This value is intentionally non-serializable and is never restored.
    pub async fn approve_input(
        &self,
        device: &ToolDevice,
        _consent: &DeviceInputConsent,
        cancel: &CancellationToken,
    ) -> Result<DeviceInputGrant, RuntimeError> {
        let id = self.address(device)?;
        if self.backend != DeviceBackend::Android
            || device.availability != DeviceAvailability::Ready
        {
            return Err(RuntimeError::Unsupported(
                "Input is only implemented for an authorized Android target with /system/bin/input"
                    .into(),
            ));
        }
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
        Ok(DeviceInputGrant {
            device: id,
            executable: self.executable.clone(),
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
        if self.backend != DeviceBackend::Android
            || device.availability != DeviceAvailability::Ready
            || id != grant.device
            || self.executable != grant.executable
        {
            return Err(RuntimeError::Denied(
                "Device input authority is missing or belongs to another target".into(),
            ));
        }
        let mut args = vec!["-s".into(), id, "shell".into(), "/system/bin/input".into()];
        args.extend(android_input_args(&input, width, height)?);
        command::run(&self.executable, args, 4096, cancel).await?;
        Ok(())
    }
}
#[derive(Debug)]
pub struct DeviceInputGrant {
    device: String,
    executable: PathBuf,
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
        DeviceInput::Text { .. } => Err(RuntimeError::Unsupported(
            "Text injection is not implemented".into(),
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
