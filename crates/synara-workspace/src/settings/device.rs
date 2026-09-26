//! Preferences are not capabilities and never grant device authority.
use crate::{WorkspaceError, WorkspaceResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use synara_runtime::DeviceBackend;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DeviceSettings {
    pub backend: DeviceBackend,
    pub adb_path: Option<PathBuf>,
    /// User-selected macOS CoreSimulator helper. It is never downloaded or
    /// executed until an explicit Simulator input/accessibility action.
    pub apple_helper_path: Option<PathBuf>,
    /// Opt-in screenshots every two seconds, only while the viewer is visible.
    pub auto_capture: bool,
}
impl DeviceSettings {
    pub fn validate(&self) -> WorkspaceResult<()> {
        for (path, label) in [
            (self.adb_path.as_ref(), "ADB"),
            (self.apple_helper_path.as_ref(), "Apple device helper"),
        ] {
            if path.is_some_and(|path| {
                !path.is_absolute()
                    || path.as_os_str().len() > 4096
                    || path.to_string_lossy().chars().any(char::is_control)
            }) {
                return Err(WorkspaceError::Invalid(format!(
                    "{label} must be an absolute executable path, without control characters"
                )));
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_never_start_or_authorize_tools() {
        let settings = DeviceSettings::default();
        assert!(!settings.auto_capture);
        assert!(settings.adb_path.is_none());
        assert!(settings.apple_helper_path.is_none());
        assert!(settings.validate().is_ok());
        let mut bad = settings.clone();
        bad.adb_path = Some(PathBuf::from("adb"));
        assert!(bad.validate().is_err());
        let mut bad_helper = settings;
        bad_helper.apple_helper_path = Some(PathBuf::from("synara-device-helper"));
        assert!(bad_helper.validate().is_err());
    }
}

#[cfg(test)]
mod persistence_tests {
    use crate::{AppSettings, KeyBinding, WorkspaceService};
    #[tokio::test]
    async fn startup_restoration_respects_opt_out_archiving_and_missing_selection() {
        let root = tempfile::tempdir().unwrap();
        let service = WorkspaceService::memory().unwrap();
        let project = service
            .add_local_workspace(root.path().to_path_buf())
            .await
            .unwrap();
        let task = service
            .create_task(
                project.id,
                "Restore".into(),
                crate::default_profiles()[0].id.clone(),
            )
            .await
            .unwrap();
        let selection = crate::Selection {
            project: Some(project.id),
            task: Some(task.id),
        };
        let mut settings = AppSettings::default();
        let catalog = service.catalog().await.unwrap();
        assert_eq!(
            crate::startup_task(&settings, &selection, &catalog),
            Some(task.id)
        );
        assert_eq!(
            crate::startup_task(&settings, &crate::Selection::default(), &catalog),
            None
        );
        settings.general.restore_last_chat = false;
        assert_eq!(crate::startup_task(&settings, &selection, &catalog), None);
        settings.general.restore_last_chat = true;
        service.archive_task(task.id).await.unwrap();
        assert_eq!(
            crate::startup_task(&settings, &selection, &service.catalog().await.unwrap()),
            None
        );
    }
    #[tokio::test]
    async fn device_settings_and_native_preferences_survive_reopen_without_authority() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("settings.db");
        let service = WorkspaceService::open(db.clone()).await.unwrap();
        let mut settings = AppSettings::default();
        settings.device.adb_path = Some(root.path().join("adb"));
        settings.device.apple_helper_path = Some(root.path().join("synara-device-helper"));
        settings.device.auto_capture = true;
        settings.general.restore_last_chat = false;
        settings.appearance.high_contrast = true;
        settings.notifications.background_completion = true;
        settings.chat.show_recent_attachments = false;
        settings.keybindings.push(KeyBinding {
            command: "navigation.device".into(),
            shortcut: "Primary+Alt+D".into(),
        });
        service.save_settings(settings.clone()).await.unwrap();
        drop(service);
        let reopened = WorkspaceService::open(db)
            .await
            .unwrap()
            .settings()
            .await
            .unwrap()
            .settings;
        assert_eq!(reopened, settings);
        let encoded = serde_json::to_string(&reopened).unwrap();
        assert!(!encoded.contains("grant") && !encoded.contains("consent"));
        assert!(!encoded.contains("quota") && !encoded.contains("subscription"));
    }
}
