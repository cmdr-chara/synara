//! User-owned chat behavior, independent of provider approval policies.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceEnterBehavior {
    #[default]
    Stop,
    Send,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChatSettings {
    pub send_on_enter: bool,
    pub voice_enter_behavior: VoiceEnterBehavior,
    pub show_timestamps: bool,
    pub show_recent_attachments: bool,
}
impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            send_on_enter: true,
            voice_enter_behavior: VoiceEnterBehavior::Stop,
            show_timestamps: true,
            show_recent_attachments: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppSettings, WorkspaceService};
    #[test]
    fn old_settings_get_compatible_chat_defaults() {
        let old = serde_json::json!({"version":1});
        let settings: AppSettings = serde_json::from_value(old).unwrap();
        assert_eq!(settings.chat, ChatSettings::default());
        settings.validate().unwrap();
    }
    #[test]
    fn malformed_chat_settings_are_not_coerced() {
        for chat in [
            serde_json::json!({"send_on_enter":"false"}),
            serde_json::json!({"voice_enter_behavior":"unexpected"}),
            serde_json::json!({"show_timestamps":null}),
            serde_json::json!({"unknown":true}),
        ] {
            assert!(
                serde_json::from_value::<AppSettings>(serde_json::json!({"version":1,"chat":chat}))
                    .is_err()
            );
        }
    }
    #[tokio::test]
    async fn chat_behavior_survives_reopen_and_resets_without_changing_profile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.db");
        let service = WorkspaceService::open(path.clone()).await.unwrap();
        let mut settings = service.settings().await.unwrap().settings;
        settings.profile.name = "Test profile".into();
        settings.chat.send_on_enter = false;
        settings.chat.voice_enter_behavior = VoiceEnterBehavior::Send;
        settings.chat.show_timestamps = false;
        service.save_settings(settings).await.unwrap();
        drop(service);
        let service = WorkspaceService::open(path).await.unwrap();
        let mut settings = service.settings().await.unwrap().settings;
        assert!(
            !settings.chat.send_on_enter
                && settings.chat.voice_enter_behavior == VoiceEnterBehavior::Send
                && !settings.chat.show_timestamps
        );
        settings.chat = ChatSettings::default();
        service.save_settings(settings).await.unwrap();
        let saved = service.settings().await.unwrap().settings;
        assert_eq!(saved.profile.name, "Test profile");
        assert_eq!(saved.chat, ChatSettings::default());
    }

    #[test]
    fn voice_enter_behavior_defaults_when_older_chat_settings_are_recovered() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({
            "version": 1,
            "chat": {
                "send_on_enter": false,
                "show_timestamps": true,
                "show_recent_attachments": true
            }
        }))
        .unwrap();

        assert_eq!(settings.chat.voice_enter_behavior, VoiceEnterBehavior::Stop);
        settings.validate().unwrap();
    }
}
