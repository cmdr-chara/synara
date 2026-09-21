//! User-owned presentation preferences. These never configure an agent or launch a tool.
use super::*;
use std::path::PathBuf;
mod wallpaper;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Colorway { #[default] Original, Graphite, Midnight, Ocean, Forest, Ember, Sand }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceMaterial { #[default] Solid, Transparent, Frosted, Glass }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MotionPreference { Off, Subtle, #[default] Standard, Expressive }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DensityPreference { Compact, #[default] Comfortable, Spacious }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperFit { #[default] Cover, Contain }

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Personalization {
    pub version: u32,
    pub zen_mode: bool,
    pub colorway: Colorway,
    pub material: SurfaceMaterial,
    pub canvas_opacity: u8,
    pub panel_opacity: u8,
    pub wallpaper: Option<PathBuf>,
    pub wallpaper_dim: u8,
    pub wallpaper_blur: u8,
    pub wallpaper_fit: WallpaperFit,
    pub motion: MotionPreference,
    pub density: DensityPreference,
    pub chat_width: u16,
    pub accent: Option<u32>,
    pub terminal_font_size: u8,
}
impl Default for Personalization {
    fn default() -> Self {
        Self {
            version: 1, zen_mode: false, colorway: Colorway::Original,
            material: SurfaceMaterial::Solid, canvas_opacity: 85, panel_opacity: 92,
            wallpaper: None, wallpaper_dim: 55, wallpaper_blur: 24, wallpaper_fit: WallpaperFit::Cover,
            motion: MotionPreference::Standard, density: DensityPreference::Comfortable,
            chat_width: 736, accent: None, terminal_font_size: 14,
        }
    }
}
impl Personalization {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1 || !(35..=100).contains(&self.canvas_opacity)
            || !(60..=100).contains(&self.panel_opacity) || self.wallpaper_dim > 95 || self.wallpaper_blur > 64
            || !(560..=1200).contains(&self.chat_width)
            || !(10..=24).contains(&self.terminal_font_size)
            || self.accent.is_some_and(|value| value > 0xffffff)
        {
            return Err(WorkspaceError::Invalid("Invalid appearance values or unsupported personalization version.".into()));
        }
        if let Some(path) = &self.wallpaper { validate_wallpaper_path(path)?; }
        Ok(())
    }
}
fn validate_wallpaper_path(path: &std::path::Path) -> WorkspaceResult<()> {
    let text = path.to_str().ok_or_else(|| WorkspaceError::Invalid("Choose an image with a UTF-8 file path.".into()))?;
    if !path.is_absolute() || text.len() > 8192 || text.chars().any(char::is_control)
        || text.starts_with("//") || text.starts_with("\\\\") || path.file_name().is_none()
    {
        return Err(WorkspaceError::Invalid("Wallpaper must be a local absolute file path, not a URL or network share.".into()));
    }
    Ok(())
}

/// An appearance-only transfer format. Import never changes wallpaper or Zen state.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceProfile {
    pub version: u32,
    pub appearance: AppearanceSettings,
}
impl AppearanceProfile {
    pub fn export(value: &AppearanceSettings) -> WorkspaceResult<String> {
        let mut appearance = value.clone();
        appearance.personalization.wallpaper = None;
        appearance.personalization.zen_mode = false;
        serde_json::to_string_pretty(&Self { version: 1, appearance })
            .map_err(|error| WorkspaceError::Invalid(error.to_string()))
    }
    pub fn import(text: &str, current: &AppearanceSettings) -> WorkspaceResult<AppearanceSettings> {
        if text.len() > 64 * 1024 {
            return Err(WorkspaceError::Invalid("Appearance profiles are limited to 64 KiB.".into()));
        }
        let mut profile: Self = serde_json::from_str(text)
            .map_err(|error| WorkspaceError::Invalid(format!("Invalid appearance profile: {error}")))?;
        if profile.version != 1 {
            return Err(WorkspaceError::Invalid("Unsupported appearance profile version. Settings were not changed.".into()));
        }
        // Imported files have no authority to select a local file or change task presentation.
        profile.appearance.personalization.wallpaper = current.personalization.wallpaper.clone();
        profile.appearance.personalization.zen_mode = current.personalization.zen_mode;
        let candidate = AppSettings { appearance: profile.appearance.clone(), ..AppSettings::default() };
        candidate.validate()?;
        Ok(profile.appearance)
    }
}

pub struct WallpaperAsset {
    pub bytes: Vec<u8>,
    pub format: crate::PreviewImageFormat,
    pub width: u32,
    pub height: u32,
}
impl WorkspaceService {
    pub async fn read_wallpaper(path: PathBuf) -> WorkspaceResult<WallpaperAsset> {
        Self::render_wallpaper(path, 0).await
    }
    pub async fn render_wallpaper(path: PathBuf, blur: u8) -> WorkspaceResult<WallpaperAsset> {
        validate_wallpaper_path(&path)?;
        if blur > 64 { return Err(WorkspaceError::Invalid("Wallpaper blur exceeds 64.".into())); }
        wallpaper::read(path, blur).await
    }
}
