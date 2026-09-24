use crate::{StorageResult, Store, WorkspaceError, WorkspaceResult, WorkspaceService};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

mod personalization;
pub use personalization::*;
mod keybindings;
pub use keybindings::*;
mod device;
pub use device::DeviceSettings;
mod chat;
pub use chat::ChatSettings;

pub const SETTINGS_VERSION: u32 = 1;
const MAX_FONT_FAMILY_BYTES: usize = 256;
const MAX_KEYBINDINGS: usize = 256;
const MAX_BINDING_BYTES: usize = 128;
const MAX_PROVIDER_ORDER: usize = 128;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FontPreferences {
    pub ui_family: Option<String>,
    pub ui_size: f32,
    pub code_family: Option<String>,
    pub code_size: f32,
}

impl Default for FontPreferences {
    fn default() -> Self {
        Self {
            ui_family: None,
            ui_size: 13.0,
            code_family: None,
            code_size: 12.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DarkThemePreference {
    #[default]
    Synara,
    Dracula,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceSettings {
    #[serde(default)]
    pub personalization: Personalization,
    #[serde(default)]
    pub dark_theme: DarkThemePreference,
    #[serde(default)]
    pub theme: ThemePreference,
    #[serde(default)]
    pub fonts: FontPreferences,
    #[serde(default)]
    pub reduced_motion: bool,
    #[serde(default)]
    pub high_contrast: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeyBinding {
    pub command: String,
    pub shortcut: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppSettings {
    #[serde(default)]
    pub notifications: NotificationSettings,
    #[serde(default)]
    pub device: DeviceSettings,
    pub version: u32,
    #[serde(default)]
    pub appearance: AppearanceSettings,
    #[serde(default)]
    pub keybindings: Vec<KeyBinding>,
    #[serde(default)]
    pub general: GeneralSettings,
    #[serde(default)]
    pub profile: ProfileSettings,
    #[serde(default)]
    pub chat: ChatSettings,
    #[serde(default)]
    pub onboarding: OnboardingSettings,
}

/// A started tour is persisted before the first window opens. Older settings
/// deserialize with `started = false`, so an existing installation is exempt.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OnboardingSettings {
    pub started: bool,
    pub completed: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GeneralSettings {
    pub default_provider: Option<String>,
    /// User ordering for provider pickers; unlisted providers retain their
    /// registry order and are appended when discovered.
    pub provider_order: Vec<String>,
    pub show_chats: bool,
    pub show_studio: bool,
    pub alphabetical_projects: bool,
    pub oldest_threads_first: bool,
    pub restore_last_chat: bool,
}
impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            default_provider: None,
            provider_order: Vec::new(),
            show_chats: true,
            show_studio: true,
            alphabetical_projects: false,
            oldest_threads_first: false,
            restore_last_chat: true,
        }
    }
}

/// Off by default. Notifications never include prompts, paths or task titles.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NotificationSettings {
    pub background_completion: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProfileSettings {
    pub name: String,
    pub username: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            notifications: NotificationSettings::default(),
            device: DeviceSettings::default(),
            version: SETTINGS_VERSION,
            appearance: AppearanceSettings::default(),
            keybindings: Vec::new(),
            general: GeneralSettings::default(),
            profile: ProfileSettings::default(),
            chat: ChatSettings::default(),
            onboarding: OnboardingSettings::default(),
        }
    }
}

impl AppSettings {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != SETTINGS_VERSION {
            return Err(WorkspaceError::Invalid(
                "unsupported settings schema version".into(),
            ));
        }
        if self
            .general
            .default_provider
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 128 || id.chars().any(char::is_control))
            || self.general.provider_order.len() > MAX_PROVIDER_ORDER
            || self
                .general
                .provider_order
                .iter()
                .any(|id| id.is_empty() || id.len() > 128 || id.chars().any(char::is_control))
            || self
                .general
                .provider_order
                .iter()
                .enumerate()
                .any(|(index, id)| self.general.provider_order[..index].contains(id))
            || [&self.profile.name, &self.profile.username]
                .iter()
                .any(|value| value.len() > 120 || value.chars().any(char::is_control))
        {
            return Err(WorkspaceError::Invalid(
                "invalid general or profile settings".into(),
            ));
        }
        validate_navigation_bindings(&self.keybindings)?;
        self.device.validate()?;
        self.appearance.personalization.validate()?;
        validate_font_family(self.appearance.fonts.ui_family.as_deref())?;
        validate_font_family(self.appearance.fonts.code_family.as_deref())?;
        for size in [
            self.appearance.fonts.ui_size,
            self.appearance.fonts.code_size,
        ] {
            if !size.is_finite() || !(8.0..=72.0).contains(&size) {
                return Err(WorkspaceError::Invalid(
                    "font sizes must be finite values from 8 through 72".into(),
                ));
            }
        }
        if self.keybindings.len() > MAX_KEYBINDINGS {
            return Err(WorkspaceError::Invalid(
                "too many custom keybindings".into(),
            ));
        }
        let mut commands = HashSet::new();
        let mut legacy_shortcuts = HashSet::new();
        for binding in &self.keybindings {
            if !valid_binding_text(&binding.command) || !valid_binding_text(&binding.shortcut) {
                return Err(WorkspaceError::Invalid("invalid custom keybinding".into()));
            }
            if !commands.insert(binding.command.as_str()) {
                return Err(WorkspaceError::Invalid(
                    "custom keybindings must have unique commands".into(),
                ));
            }
            // Implemented shortcuts are checked by active context above, so a
            // composer action can share an editor chord. Retained legacy rows
            // are still subject to the old duplicate check.
            if !NAVIGATION_COMMANDS.iter().any(|command| command.id == binding.command)
                && !CONTEXTUAL_COMMANDS.iter().any(|command| command.id == binding.command)
                && !legacy_shortcuts.insert(binding.shortcut.as_str())
            {
                return Err(WorkspaceError::Invalid(
                    "legacy keybindings must have unique shortcuts".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Restore only a valid saved selection, never an arbitrary archived/first task.
/// The caller loads a transcript, not an agent process or device helper.
pub fn startup_task(
    settings: &AppSettings,
    selection: &crate::Selection,
    catalog: &crate::Catalog,
) -> Option<synara_core::TaskId> {
    if !settings.general.restore_last_chat {
        return None;
    }
    selection.task.filter(|id| {
        catalog.tasks.iter().any(|task| {
            task.id == *id
                && Some(task.project_id) == selection.project
                && task.state != synara_core::TaskState::Archived
        })
    })
}

fn validate_font_family(value: Option<&str>) -> WorkspaceResult<()> {
    if value.is_some_and(|value| {
        value.is_empty()
            || value.len() > MAX_FONT_FAMILY_BYTES
            || value.chars().any(char::is_control)
    }) {
        return Err(WorkspaceError::Invalid("invalid font family".into()));
    }
    Ok(())
}

fn valid_binding_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_BINDING_BYTES && !value.chars().any(char::is_control)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SettingsRecovery {
    Malformed,
    UnsupportedVersion { found: u64 },
    InvalidCurrentVersion,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoadedSettings {
    pub settings: AppSettings,
    pub recovery: Option<SettingsRecovery>,
    /// Whether a settings value was already present in the workspace store.
    pub existed: bool,
}

impl LoadedSettings {
    fn defaults(reason: SettingsRecovery) -> Self {
        Self {
            settings: AppSettings::default(),
            recovery: Some(reason),
            existed: true,
        }
    }
}

fn load(store: &Store) -> StorageResult<LoadedSettings> {
    let Some(raw) = store.preference_raw("settings")? else {
        return Ok(LoadedSettings {
            settings: AppSettings::default(),
            recovery: None,
            existed: false,
        });
    };
    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(_) => return Ok(LoadedSettings::defaults(SettingsRecovery::Malformed)),
    };
    let Some(version) = value.get("version").and_then(serde_json::Value::as_u64) else {
        return Ok(LoadedSettings::defaults(SettingsRecovery::Malformed));
    };
    if version != u64::from(SETTINGS_VERSION) {
        return Ok(LoadedSettings::defaults(
            SettingsRecovery::UnsupportedVersion { found: version },
        ));
    }
    let settings: AppSettings = match serde_json::from_value(value) {
        Ok(settings) => settings,
        Err(_) => {
            return Ok(LoadedSettings::defaults(
                SettingsRecovery::InvalidCurrentVersion,
            ));
        }
    };
    if settings.validate().is_err() {
        return Ok(LoadedSettings::defaults(
            SettingsRecovery::InvalidCurrentVersion,
        ));
    }
    Ok(LoadedSettings {
        settings,
        recovery: None,
        existed: true,
    })
}

impl WorkspaceService {
    /// Load non-secret application settings. Invalid/newer data is preserved in
    /// SQLite and reported through `recovery`; reading never overwrites it.
    pub async fn settings(&self) -> WorkspaceResult<LoadedSettings> {
        self.access(|store| Ok(load(store)?)).await
    }

    /// Persist validated non-secret settings only. Credentials and tokens must
    /// be stored through the platform credential boundary, never in settings.
    pub async fn save_settings(&self, settings: AppSettings) -> WorkspaceResult<()> {
        settings.validate()?;
        self.access(move |store| Ok(store.set_preference("settings", &settings)?))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_settings_keep_theme_and_fonts_when_new_sections_are_absent() {
        let old = serde_json::json!({"version":1, "appearance":{"theme":"dark", "fonts":{"ui_family":"Liberation Sans", "ui_size":14.0, "code_family":null, "code_size":13.0}, "reduced_motion":true}, "keybindings":[]});
        let settings: AppSettings = serde_json::from_value(old).unwrap();
        settings.validate().unwrap();
        assert_eq!(settings.appearance.theme, ThemePreference::Dark);
        assert_eq!(settings.appearance.dark_theme, DarkThemePreference::Synara);
        assert!(settings.appearance.reduced_motion);
        assert!(settings.general.show_chats && settings.general.show_studio);
        assert_eq!(settings.profile, ProfileSettings::default());
        assert_eq!(settings.onboarding, OnboardingSettings::default());
    }

    #[tokio::test]
    async fn defaults_are_explicit_and_round_trip() {
        let service = WorkspaceService::memory().unwrap();
        let loaded = service.settings().await.unwrap();
        assert_eq!(loaded.settings.version, SETTINGS_VERSION);
        assert!(loaded.recovery.is_none());
        assert!(!loaded.existed);

        let mut changed = loaded.settings;
        changed.appearance.theme = ThemePreference::Dark;
        changed.appearance.dark_theme = DarkThemePreference::Dracula;
        changed.general.alphabetical_projects = true;
        changed.general.show_studio = false;
        changed.general.provider_order = vec!["claude".into(), "codex".into()];
        changed.profile = ProfileSettings {
            name: "Native user".into(),
            username: "native".into(),
        };
        changed.appearance.fonts.ui_size = 16.0;
        changed.keybindings.push(KeyBinding {
            command: "conversation.cancel".into(),
            shortcut: "ctrl+escape".into(),
        });
        service.save_settings(changed.clone()).await.unwrap();
        assert_eq!(service.settings().await.unwrap().settings, changed);
        assert!(service.settings().await.unwrap().existed);
    }

    #[tokio::test]
    async fn invalid_save_does_not_replace_last_good_settings() {
        let service = WorkspaceService::memory().unwrap();
        let mut good = AppSettings::default();
        good.appearance.theme = ThemePreference::Light;
        service.save_settings(good.clone()).await.unwrap();

        let mut invalid = good.clone();
        invalid.appearance.fonts.code_size = f32::INFINITY;
        assert!(matches!(
            service.save_settings(invalid).await,
            Err(WorkspaceError::Invalid(_))
        ));
        assert_eq!(service.settings().await.unwrap().settings, good);
    }

    #[test]
    fn malformed_newer_and_invalid_current_data_recover_without_overwriting() {
        let store = Store::memory().unwrap();

        store.set_preference("settings", &"not an object").unwrap();
        let malformed = load(&store).unwrap();
        assert_eq!(malformed.recovery, Some(SettingsRecovery::Malformed));
        assert_eq!(
            store.preference::<String>("settings").unwrap().as_deref(),
            Some("not an object")
        );

        store
            .set_preference("settings", &serde_json::json!({"version":99}))
            .unwrap();
        let newer = load(&store).unwrap();
        assert_eq!(
            newer.recovery,
            Some(SettingsRecovery::UnsupportedVersion { found: 99 })
        );
        assert_eq!(
            store
                .preference::<serde_json::Value>("settings")
                .unwrap()
                .unwrap()["version"],
            99
        );

        store
            .set_preference(
                "settings",
                &serde_json::json!({
                    "version":1,
                    "appearance":{
                        "theme":"system",
                        "fonts":{
                            "ui_family":null,
                            "ui_size":1000.0,
                            "code_family":null,
                            "code_size":13.0
                        },
                        "reduced_motion":false
                    },
                    "keybindings":[]
                }),
            )
            .unwrap();
        assert_eq!(
            load(&store).unwrap().recovery,
            Some(SettingsRecovery::InvalidCurrentVersion)
        );
    }

    #[test]
    fn duplicate_or_hostile_keybindings_are_rejected() {
        let mut settings = AppSettings {
            keybindings: vec![
                KeyBinding {
                    command: "conversation.send".into(),
                    shortcut: "ctrl+enter".into(),
                },
                KeyBinding {
                    command: "conversation.cancel".into(),
                    shortcut: "ctrl+enter".into(),
                },
            ],
            ..Default::default()
        };
        assert!(settings.validate().is_err());
        settings.keybindings[1].shortcut = "ctrl+escape\n".into();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn provider_order_is_bounded_and_unique() {
        let mut settings = AppSettings::default();
        settings.general.provider_order = vec!["codex".into(), "claude".into()];
        assert!(settings.validate().is_ok());
        settings.general.provider_order.push("codex".into());
        assert!(settings.validate().is_err());
        settings.general.provider_order = vec!["bad\nprovider".into()];
        assert!(settings.validate().is_err());
        settings.general.provider_order = vec!["p".repeat(129)];
        assert!(settings.validate().is_err());
    }
}
