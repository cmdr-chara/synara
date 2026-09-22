//! Durable image content, independent of provider capabilities and UI caches.
use crate::{EventId, Role};
use serde::{Deserialize, Serialize};

pub const MAX_TRANSCRIPT_IMAGE_BYTES: usize = 2 * 1024 * 1024;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageSource {
    #[default]
    Uploaded,
    AgentReturned,
    DeviceCapture,
    BrowserCapture,
    AppSnap,
}
impl ImageSource {
    pub fn label(self) -> &'static str {
        match self {
            Self::Uploaded => "Uploaded image (local submission, not a delivery receipt)",
            Self::AgentReturned => "Agent-returned image (creation source unverified)",
            Self::DeviceCapture => "Device capture",
            Self::BrowserCapture => "Browser capture",
            Self::AppSnap => "AppSnap window capture",
        }
    }
    pub fn is_capture(self) -> bool {
        matches!(
            self,
            Self::DeviceCapture | Self::BrowserCapture | Self::AppSnap
        )
    }
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TranscriptImage {
    pub source: ImageSource,
    pub mime_type: String,
    pub base64: String,
}
impl std::fmt::Debug for TranscriptImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriptImage")
            .field("source", &self.source)
            .field("mime_type", &self.mime_type)
            .field("encoded_bytes", &self.base64.len())
            .finish_non_exhaustive()
    }
}
impl TranscriptImage {
    /// Decode, format and dimension checks belong to the existing media worker.
    pub fn bounded(&self) -> bool {
        matches!(self.mime_type.as_str(), "image/png" | "image/jpeg")
            && !self.base64.is_empty()
            && self.base64.len() <= MAX_TRANSCRIPT_IMAGE_BYTES.div_ceil(3) * 4
            && self.base64.len().is_multiple_of(4)
            && self
                .base64
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'='))
    }
}
#[derive(Clone, Debug)]
pub struct MessageImage {
    pub id: EventId,
    pub message_id: String,
    pub role: Role,
    pub image: TranscriptImage,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    fn image() -> TranscriptImage {
        TranscriptImage {
            source: ImageSource::Uploaded,
            mime_type: "image/png".into(),
            base64: "AAAA".into(),
        }
    }
    fn append(thread: &mut Thread, event: ThreadEvent) -> EventEnvelope {
        let envelope = EventEnvelope {
            id: EventId::new(),
            thread_id: thread.id,
            sequence: thread.last_sequence + 1,
            timestamp_ms: 10,
            event,
        };
        thread.apply(&envelope).unwrap();
        envelope
    }
    #[test]
    fn image_only_messages_attach_by_exact_role_and_replay_without_duplicates() {
        let mut thread = Thread::new(ThreadId::new());
        append(
            &mut thread,
            ThreadEvent::TextDelta {
                message_id: Some("same".into()),
                role: Role::User,
                text: "User text".into(),
            },
        );
        let envelope = append(
            &mut thread,
            ThreadEvent::ImageMessage {
                message_id: Some("same".into()),
                role: Role::User,
                image: image(),
            },
        );
        assert!(!thread.apply(&envelope).unwrap());
        assert_eq!(thread.messages.len(), 1);
        assert_eq!(thread.images.len(), 1);
        append(
            &mut thread,
            ThreadEvent::ImageMessage {
                message_id: Some("same".into()),
                role: Role::Assistant,
                image: image(),
            },
        );
        assert_eq!(thread.messages.len(), 2);
        assert_eq!(thread.images[1].role, Role::Assistant);
        let restored: ThreadEvent =
            serde_json::from_str(&serde_json::to_string(&envelope.event).unwrap()).unwrap();
        assert_eq!(envelope.event, restored);
    }
    #[test]
    fn anonymous_image_and_text_share_message_but_failed_history_restores_media() {
        let mut thread = Thread::new(ThreadId::new());
        append(
            &mut thread,
            ThreadEvent::ImageMessage {
                message_id: None,
                role: Role::Assistant,
                image: image(),
            },
        );
        append(
            &mut thread,
            ThreadEvent::TextDelta {
                message_id: None,
                role: Role::Assistant,
                text: "Caption".into(),
            },
        );
        assert_eq!(thread.messages.len(), 1);
        assert_eq!(thread.messages[0].text, "Caption");
        append(&mut thread, ThreadEvent::HistoryStarted);
        assert!(thread.images.is_empty());
        append(
            &mut thread,
            ThreadEvent::Error {
                message: "Replay failed".into(),
                recoverable: false,
            },
        );
        assert_eq!(thread.images.len(), 1);
    }
    #[test]
    fn media_bounds_and_sources_are_not_inferred_from_names() {
        let mut media = image();
        assert!(media.bounded());
        media.mime_type = "image/svg+xml".into();
        assert!(!media.bounded());
        media = image();
        media.base64 = "A".repeat(4 * 1024 * 1024);
        assert!(!media.bounded());
        assert_ne!(
            ImageSource::Uploaded.label(),
            ImageSource::AgentReturned.label()
        );
        assert!(!format!("{:?}", image()).contains("AAAA"));
    }
}
