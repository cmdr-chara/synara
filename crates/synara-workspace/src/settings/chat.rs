//! User-owned chat behavior, independent of provider approval policies.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ChatSettings {
    pub send_on_enter: bool,
    pub show_timestamps: bool,
}
impl Default for ChatSettings {
    fn default() -> Self {
        Self {
            send_on_enter: true,
            show_timestamps: true,
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
        settings.chat.show_timestamps = false;
        service.save_settings(settings).await.unwrap();
        drop(service);
        let service = WorkspaceService::open(path).await.unwrap();
        let mut settings = service.settings().await.unwrap().settings;
        assert!(!settings.chat.send_on_enter && !settings.chat.show_timestamps);
        settings.chat = ChatSettings::default();
        service.save_settings(settings).await.unwrap();
        let saved = service.settings().await.unwrap().settings;
        assert_eq!(saved.profile.name, "Test profile");
        assert_eq!(saved.chat, ChatSettings::default());
    }
}
