//! Portable device/simulator domain and bounded helper protocol.
//!
//! Apple-specific APIs belong in a narrow helper. This module owns the Rust
//! lifecycle, packet bounds and user-directed input contract.

use crate::{ProcessHandle, RuntimeError};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::watch;

const MAX_CONTROL_BYTES: usize = 64 * 1024;
const MAX_FRAME_BYTES: usize = 32 * 1024 * 1024;
const MAX_DEVICES: usize = 256;
const MAX_ID_BYTES: usize = 256;
const MAX_LABEL_BYTES: usize = 512;
const MAX_TEXT_INPUT_BYTES: usize = 64 * 1024;
const MAX_DIMENSION: u32 = 8192;

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn new(value: impl Into<String>) -> Result<Self, RuntimeError> {
        let value = value.into();
        if !valid_text(&value, MAX_ID_BYTES) {
            return Err(RuntimeError::Invalid("invalid device identifier".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    /// A transport without enough evidence to classify the hardware.
    Unknown,
    Simulator,
    Physical,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceState {
    Discovered,
    Attaching,
    Attached,
    Disconnected,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceDescriptor {
    pub id: DeviceId,
    pub name: String,
    pub platform: String,
    pub kind: DeviceKind,
    pub state: DeviceState,
}

impl DeviceDescriptor {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        if !valid_text(self.id.as_str(), MAX_ID_BYTES)
            || !valid_text(&self.name, MAX_LABEL_BYTES)
            || !valid_text(&self.platform, MAX_LABEL_BYTES)
        {
            return Err(RuntimeError::Invalid("invalid device metadata".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameFormat {
    Rgba8,
    Bgra8,
}

#[derive(Clone, Debug)]
pub struct DeviceFrame {
    pub width: u32,
    pub height: u32,
    pub format: FrameFormat,
    pub bytes: Arc<[u8]>,
    pub sequence: u64,
}

impl DeviceFrame {
    pub fn new(
        width: u32,
        height: u32,
        format: FrameFormat,
        bytes: Vec<u8>,
        sequence: u64,
    ) -> Result<Self, RuntimeError> {
        validate_dimensions(width, height)?;
        let expected = usize::try_from(width)
            .ok()
            .and_then(|width| {
                usize::try_from(height)
                    .ok()
                    .and_then(|height| width.checked_mul(height))
            })
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(RuntimeError::Limit)?;
        if expected > MAX_FRAME_BYTES || bytes.len() != expected {
            return Err(RuntimeError::Invalid(
                "device frame size does not match its dimensions".into(),
            ));
        }
        Ok(Self {
            width,
            height,
            format,
            bytes: bytes.into(),
            sequence,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeviceInput {
    Tap {
        x: u32,
        y: u32,
    },
    Swipe {
        from_x: u32,
        from_y: u32,
        to_x: u32,
        to_y: u32,
        duration_ms: u32,
    },
    Text {
        text: String,
    },
    Key {
        key: String,
    },
}

impl DeviceInput {
    pub fn validate(&self) -> Result<(), RuntimeError> {
        match self {
            Self::Tap { x, y } => validate_point(*x, *y),
            Self::Swipe {
                from_x,
                from_y,
                to_x,
                to_y,
                duration_ms,
            } => {
                validate_point(*from_x, *from_y)?;
                validate_point(*to_x, *to_y)?;
                if !(1..=60_000).contains(duration_ms) {
                    return Err(RuntimeError::Invalid("invalid swipe duration".into()));
                }
                Ok(())
            }
            Self::Text { text } => {
                if text.len() > MAX_TEXT_INPUT_BYTES || text.contains('\0') {
                    Err(RuntimeError::Limit)
                } else {
                    Ok(())
                }
            }
            Self::Key { key } => {
                if valid_text(key, 128) {
                    Ok(())
                } else {
                    Err(RuntimeError::Invalid("invalid device key".into()))
                }
            }
        }
    }
}

/// Deliberately non-serializable proof that the application accepted this input
/// from an explicit user/permission boundary.
#[derive(Debug)]
pub struct DeviceInputConsent {
    _private: (),
}

impl DeviceInputConsent {
    pub fn user_approved() -> Self {
        Self { _private: () }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeviceControl {
    Discover,
    Attach {
        id: DeviceId,
    },
    Resize {
        session_id: String,
        width: u32,
        height: u32,
    },
    Detach {
        session_id: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeviceControlEvent {
    Devices {
        devices: Vec<DeviceDescriptor>,
    },
    Attached {
        session_id: String,
        device: DeviceDescriptor,
    },
    Frame {
        session_id: String,
        width: u32,
        height: u32,
        format: FrameFormat,
        byte_length: usize,
        sequence: u64,
    },
    Disconnected {
        session_id: String,
    },
    Error {
        code: String,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum DevicePacketKind {
    Control = 1,
    Frame = 2,
    Input = 3,
}

impl TryFrom<u8> for DevicePacketKind {
    type Error = RuntimeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Control),
            2 => Ok(Self::Frame),
            3 => Ok(Self::Input),
            _ => Err(RuntimeError::Invalid("unknown device packet kind".into())),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevicePacket {
    pub kind: DevicePacketKind,
    pub payload: Vec<u8>,
}

impl DevicePacket {
    pub fn control<T: Serialize>(value: &T) -> Result<Self, RuntimeError> {
        let payload = serde_json::to_vec(value)
            .map_err(|_| RuntimeError::Invalid("device control message is not encodable".into()))?;
        if payload.len() > MAX_CONTROL_BYTES {
            return Err(RuntimeError::Limit);
        }
        Ok(Self {
            kind: DevicePacketKind::Control,
            payload,
        })
    }

    pub fn input(
        session_id: &str,
        input: &DeviceInput,
        _consent: &DeviceInputConsent,
    ) -> Result<Self, RuntimeError> {
        validate_session_id(session_id)?;
        input.validate()?;
        #[derive(Serialize)]
        struct InputMessage<'a> {
            session_id: &'a str,
            input: &'a DeviceInput,
        }
        let payload = serde_json::to_vec(&InputMessage { session_id, input })
            .map_err(|_| RuntimeError::Invalid("device input is not encodable".into()))?;
        if payload.len() > MAX_CONTROL_BYTES {
            return Err(RuntimeError::Limit);
        }
        Ok(Self {
            kind: DevicePacketKind::Input,
            payload,
        })
    }

    pub fn frame(bytes: Vec<u8>) -> Result<Self, RuntimeError> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(RuntimeError::Limit);
        }
        Ok(Self {
            kind: DevicePacketKind::Frame,
            payload: bytes,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, RuntimeError> {
        let limit = match self.kind {
            DevicePacketKind::Frame => MAX_FRAME_BYTES,
            DevicePacketKind::Control | DevicePacketKind::Input => MAX_CONTROL_BYTES,
        };
        if self.payload.len() > limit {
            return Err(RuntimeError::Limit);
        }
        let length = u32::try_from(self.payload.len()).map_err(|_| RuntimeError::Limit)?;
        let mut encoded = Vec::with_capacity(5 + self.payload.len());
        encoded.push(self.kind as u8);
        encoded.extend_from_slice(&length.to_be_bytes());
        encoded.extend_from_slice(&self.payload);
        Ok(encoded)
    }

    pub fn decode(encoded: &[u8]) -> Result<Self, RuntimeError> {
        if encoded.len() < 5 {
            return Err(RuntimeError::Invalid("truncated device packet".into()));
        }
        let kind = DevicePacketKind::try_from(encoded[0])?;
        let length = u32::from_be_bytes(encoded[1..5].try_into().unwrap()) as usize;
        let limit = match kind {
            DevicePacketKind::Frame => MAX_FRAME_BYTES,
            DevicePacketKind::Control | DevicePacketKind::Input => MAX_CONTROL_BYTES,
        };
        if length > limit || encoded.len() != 5 + length {
            return Err(RuntimeError::Limit);
        }
        Ok(Self {
            kind,
            payload: encoded[5..].to_vec(),
        })
    }

    pub fn decode_control<T: for<'de> Deserialize<'de>>(&self) -> Result<T, RuntimeError> {
        if self.kind != DevicePacketKind::Control {
            return Err(RuntimeError::Invalid(
                "expected device control packet".into(),
            ));
        }
        serde_json::from_slice(&self.payload)
            .map_err(|_| RuntimeError::Invalid("invalid device control payload".into()))
    }
}

#[derive(Clone, Debug)]
pub struct DeviceSessionState {
    pub session_id: String,
    pub device: DeviceDescriptor,
    pub state: DeviceState,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub last_sequence: Option<u64>,
}

impl DeviceSessionState {
    pub fn attached(
        session_id: impl Into<String>,
        mut device: DeviceDescriptor,
    ) -> Result<Self, RuntimeError> {
        let session_id = session_id.into();
        validate_session_id(&session_id)?;
        device.validate()?;
        device.state = DeviceState::Attached;
        Ok(Self {
            session_id,
            device,
            state: DeviceState::Attached,
            width: None,
            height: None,
            last_sequence: None,
        })
    }

    pub fn accept_frame(&mut self, frame: &DeviceFrame) -> Result<(), RuntimeError> {
        if self.state != DeviceState::Attached {
            return Err(RuntimeError::Closed);
        }
        if self
            .last_sequence
            .is_some_and(|sequence| frame.sequence <= sequence)
        {
            return Err(RuntimeError::Invalid(
                "stale or duplicate device frame".into(),
            ));
        }
        self.width = Some(frame.width);
        self.height = Some(frame.height);
        self.last_sequence = Some(frame.sequence);
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) -> Result<DeviceControl, RuntimeError> {
        if self.state != DeviceState::Attached {
            return Err(RuntimeError::Closed);
        }
        validate_dimensions(width, height)?;
        Ok(DeviceControl::Resize {
            session_id: self.session_id.clone(),
            width,
            height,
        })
    }

    pub fn disconnect(&mut self) {
        self.state = DeviceState::Disconnected;
        self.device.state = DeviceState::Disconnected;
    }
}

/// Owns a helper process independently of the UI. Drop requests helper teardown;
/// process-tree semantics remain provided by the shared runtime supervisor.
pub struct DeviceHelperOwner {
    process: ProcessHandle,
    state: watch::Sender<DeviceState>,
}

impl DeviceHelperOwner {
    pub fn new(process: ProcessHandle) -> Self {
        let (state, _) = watch::channel(DeviceState::Attaching);
        Self { process, state }
    }

    pub fn process_id(&self) -> u32 {
        self.process.pid()
    }

    pub fn observe(&self) -> watch::Receiver<DeviceState> {
        self.state.subscribe()
    }

    pub fn mark_attached(&self) {
        let _ = self.state.send(DeviceState::Attached);
    }

    pub fn request_stop(&self) {
        self.process.request_stop();
        let _ = self.state.send(DeviceState::Disconnected);
    }

    pub async fn shutdown(&self) -> Result<(), RuntimeError> {
        self.request_stop();
        self.process.shutdown().await?;
        Ok(())
    }
}

impl Drop for DeviceHelperOwner {
    fn drop(&mut self) {
        self.process.request_stop();
        let _ = self.state.send(DeviceState::Disconnected);
    }
}

pub fn validate_discovery(devices: &[DeviceDescriptor]) -> Result<(), RuntimeError> {
    if devices.len() > MAX_DEVICES {
        return Err(RuntimeError::Limit);
    }
    let mut ids = std::collections::HashSet::new();
    for device in devices {
        device.validate()?;
        if !ids.insert(device.id.as_str()) {
            return Err(RuntimeError::Invalid("duplicate device identifier".into()));
        }
    }
    Ok(())
}

fn validate_dimensions(width: u32, height: u32) -> Result<(), RuntimeError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(RuntimeError::Invalid("invalid device dimensions".into()));
    }
    Ok(())
}

fn validate_point(x: u32, y: u32) -> Result<(), RuntimeError> {
    if x > MAX_DIMENSION || y > MAX_DIMENSION {
        return Err(RuntimeError::Invalid(
            "device input coordinate is out of range".into(),
        ));
    }
    Ok(())
}

fn validate_session_id(session_id: &str) -> Result<(), RuntimeError> {
    if valid_text(session_id, MAX_ID_BYTES) {
        Ok(())
    } else {
        Err(RuntimeError::Invalid(
            "invalid device session identifier".into(),
        ))
    }
}

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn descriptor(id: &str) -> DeviceDescriptor {
        DeviceDescriptor {
            id: DeviceId::new(id).unwrap(),
            name: "iPhone Simulator".into(),
            platform: "iOS".into(),
            kind: DeviceKind::Simulator,
            state: DeviceState::Discovered,
        }
    }

    #[test]
    fn discovery_is_bounded_unique_and_validated() {
        assert!(validate_discovery(&[descriptor("one"), descriptor("two")]).is_ok());
        assert!(validate_discovery(&[descriptor("same"), descriptor("same")]).is_err());
        let mut devices = Vec::new();
        for index in 0..=MAX_DEVICES {
            devices.push(descriptor(&format!("device-{index}")));
        }
        assert!(matches!(
            validate_discovery(&devices),
            Err(RuntimeError::Limit)
        ));
    }

    #[test]
    fn packet_framing_is_exact_and_bounded() {
        let packet = DevicePacket::control(&DeviceControl::Discover).unwrap();
        let encoded = packet.encode().unwrap();
        assert_eq!(DevicePacket::decode(&encoded).unwrap(), packet);
        assert!(DevicePacket::decode(&encoded[..4]).is_err());
        let mut trailing = encoded.clone();
        trailing.push(0);
        assert!(DevicePacket::decode(&trailing).is_err());
        assert!(matches!(
            DevicePacket::frame(vec![0; MAX_FRAME_BYTES + 1]),
            Err(RuntimeError::Limit)
        ));
    }

    #[test]
    fn device_input_requires_a_non_serializable_consent_boundary() {
        let input = DeviceInput::Text {
            text: "hello".into(),
        };
        let consent = DeviceInputConsent::user_approved();
        let packet = DevicePacket::input("session-1", &input, &consent).unwrap();
        assert_eq!(packet.kind, DevicePacketKind::Input);
        assert!(String::from_utf8(packet.payload).unwrap().contains("hello"));
        assert!(
            DeviceInput::Text {
                text: "\0".repeat(MAX_TEXT_INPUT_BYTES)
            }
            .validate()
            .is_err()
        );
    }

    #[test]
    fn frame_state_rejects_stale_sequences_and_disconnected_updates() {
        let mut state = DeviceSessionState::attached("session-1", descriptor("sim-1")).unwrap();
        let frame = DeviceFrame::new(2, 2, FrameFormat::Rgba8, vec![0; 16], 1).unwrap();
        state.accept_frame(&frame).unwrap();
        assert_eq!((state.width, state.height), (Some(2), Some(2)));
        assert!(state.accept_frame(&frame).is_err());
        assert!(matches!(
            state.resize(320, 640).unwrap(),
            DeviceControl::Resize { .. }
        ));
        state.disconnect();
        assert!(matches!(
            state.accept_frame(&frame),
            Err(RuntimeError::Closed)
        ));
        assert!(matches!(state.resize(320, 640), Err(RuntimeError::Closed)));
    }

    #[test]
    fn frame_dimensions_and_byte_counts_are_consistent() {
        assert!(DeviceFrame::new(0, 1, FrameFormat::Rgba8, vec![], 1).is_err());
        assert!(DeviceFrame::new(2, 2, FrameFormat::Bgra8, vec![0; 15], 1).is_err());
        assert!(DeviceFrame::new(2, 2, FrameFormat::Bgra8, vec![0; 16], 1).is_ok());
    }
}
