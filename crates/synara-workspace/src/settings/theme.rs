//! Native representation of Electron's versioned appearance packs.
//!
//! Adapted from Synara eaa61eded31b6755d4f30ba8eabc5d905cf817cb,
//! apps/web/src/theme/theme.logic.ts. Copyright (c) 2026 T3 Tools Inc.
//! and Emanuele Di Pietro, MIT. See docs/ui/electron-reference-LICENSE.txt.
//!
//! Importing a theme changes presentation data only. It cannot select a local
//! file, install a font, execute a helper, or grant a provider permission.
use super::*;
use std::collections::BTreeMap;
use std::sync::OnceLock;

mod tokens;
pub use tokens::{ThemePaint, ThemeTokens};

pub const THEME_SHARE_PREFIX: &str = "codex-theme-v1:";
const MAX_SHARE_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeVariant {
    Light,
    Dark,
}
impl ThemeVariant {
    pub fn label(self) -> &'static str {
        match self { Self::Light => "Light", Self::Dark => "Dark" }
    }
}

/// Validated six-digit sRGB color, serialized in Electron's share-string form.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemeHex(u32);
impl ThemeHex {
    pub const fn rgb(value: u32) -> Self { Self(value & 0xffffff) }
    pub fn value(self) -> u32 { self.0 }
    pub fn parse(text: &str) -> WorkspaceResult<Self> {
        let value = text.trim();
        if value.len() != 7 || !value.starts_with('#') || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
            return Err(WorkspaceError::Invalid("Theme colors must be six hexadecimal digits preceded by #.".into()));
        }
        u32::from_str_radix(&value[1..], 16).map(Self)
            .map_err(|_| WorkspaceError::Invalid("Invalid theme color.".into()))
    }
}
impl std::fmt::Display for ThemeHex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "#{:06x}", self.0) }
}
impl Serialize for ThemeHex {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for ThemeHex {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::parse(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemeFonts { pub ui: Option<String>, pub code: Option<String> }

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeSemanticColors {
    pub diff_added: ThemeHex,
    pub diff_removed: ThemeHex,
    pub skill: ThemeHex,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChromeTheme {
    pub accent: ThemeHex,
    pub contrast: u8,
    pub fonts: ThemeFonts,
    pub ink: ThemeHex,
    pub opaque_windows: bool,
    pub semantic_colors: ThemeSemanticColors,
    pub surface: ThemeHex,
}
impl ChromeTheme {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.contrast > 100 {
            return Err(WorkspaceError::Invalid("Theme contrast must be an integer from 0 through 100.".into()));
        }
        validate_font_family(self.fonts.ui.as_deref())?;
        validate_font_family(self.fonts.code.as_deref())?;
        Ok(())
    }
    fn codex(variant: ThemeVariant) -> Self {
        let dark = variant == ThemeVariant::Dark;
        Self {
            accent: ThemeHex::rgb(0x0169cc),
            contrast: 0,
            fonts: ThemeFonts::default(),
            ink: ThemeHex::rgb(if dark { 0xfcfcfc } else { 0x0d0d0d }),
            opaque_windows: false,
            semantic_colors: ThemeSemanticColors {
                diff_added: ThemeHex::rgb(0x00a240),
                diff_removed: ThemeHex::rgb(0xe02e2a),
                skill: ThemeHex::rgb(if dark { 0xb06dff } else { 0x751ed9 }),
            },
            surface: ThemeHex::rgb(if dark { 0x111111 } else { 0xffffff }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemePack { pub code_theme_id: String, pub theme: ChromeTheme }
impl ThemePack {
    pub fn codex(variant: ThemeVariant) -> Self {
        Self { code_theme_id: "codex".into(), theme: ChromeTheme::codex(variant) }
    }
    pub fn validate(&self, variant: ThemeVariant) -> WorkspaceResult<()> {
        self.theme.validate()?;
        if !theme_catalog()?.get(&self.code_theme_id).is_some_and(|pack| pack.contains_key(&variant)) {
            return Err(WorkspaceError::Invalid(format!("Theme {} is unavailable in {} mode.", self.code_theme_id, variant.label())));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThemePreferences {
    pub version: u32,
    pub light: ThemePack,
    pub dark: ThemePack,
    pub system_ui_font: bool,
}
impl Default for ThemePreferences {
    fn default() -> Self {
        Self { version: 1, light: ThemePack::codex(ThemeVariant::Light), dark: ThemePack::codex(ThemeVariant::Dark), system_ui_font: true }
    }
}
impl ThemePreferences {
    pub fn validate(&self) -> WorkspaceResult<()> {
        if self.version != 1 {
            return Err(WorkspaceError::Invalid("Unsupported native theme preferences version.".into()));
        }
        self.light.validate(ThemeVariant::Light)?;
        self.dark.validate(ThemeVariant::Dark)
    }
    pub fn pack(&self, variant: ThemeVariant) -> &ThemePack {
        match variant { ThemeVariant::Light => &self.light, ThemeVariant::Dark => &self.dark }
    }
    pub fn pack_mut(&mut self, variant: ThemeVariant) -> &mut ThemePack {
        match variant { ThemeVariant::Light => &mut self.light, ThemeVariant::Dark => &mut self.dark }
    }
    pub fn reset(&mut self, variant: ThemeVariant) {
        *self.pack_mut(variant) = ThemePack::codex(variant);
    }
    /// Match Electron's seed-patch behavior: color presets normally preserve
    /// the user's contrast, fonts and material instead of resetting everything.
    pub fn select(&mut self, id: &str, variant: ThemeVariant) -> WorkspaceResult<()> {
        let seed = theme_catalog()?.get(id).and_then(|variants| variants.get(&variant))
            .ok_or_else(|| WorkspaceError::Invalid("The selected theme does not support this mode.".into()))?;
        let pack = self.pack_mut(variant);
        pack.code_theme_id = id.into();
        pack.theme.accent = seed.accent;
        pack.theme.ink = seed.ink;
        pack.theme.surface = seed.surface;
        pack.theme.semantic_colors = seed.semantic_colors.clone();
        if matches!(id, "synara" | "vercel") { pack.theme.contrast = seed.contrast; }
        if matches!(id, "linear" | "matrix" | "notion" | "proof" | "raycast" | "vercel") {
            pack.theme.opaque_windows = seed.opaque_windows;
        }
        if matches!(id, "linear" | "lobster" | "matrix" | "notion" | "proof" | "raycast" | "sentry" | "vercel") {
            pack.theme.fonts.ui = seed.fonts.ui.clone();
        }
        if matches!(id, "matrix" | "notion" | "proof" | "raycast" | "sentry" | "vercel") {
            pack.theme.fonts.code = seed.fonts.code.clone();
        }
        Ok(())
    }
    pub fn share(&self, variant: ThemeVariant) -> WorkspaceResult<String> {
        let pack = self.pack(variant);
        pack.validate(variant)?;
        let payload = ThemeShare { code_theme_id: pack.code_theme_id.clone(), theme: pack.theme.clone(), variant };
        serde_json::to_string(&payload).map(|json| format!("{THEME_SHARE_PREFIX}{json}"))
            .map_err(|error| WorkspaceError::Invalid(error.to_string()))
    }
    /// Validate the complete input before replacing one variant. A malformed
    /// value leaves both variants and every non-theme preference untouched.
    pub fn import(&mut self, text: &str, target: ThemeVariant) -> WorkspaceResult<()> {
        if text.len() > MAX_SHARE_BYTES {
            return Err(WorkspaceError::Invalid("Theme share strings are limited to 64 KiB.".into()));
        }
        let payload = text.trim().strip_prefix(THEME_SHARE_PREFIX)
            .ok_or_else(|| WorkspaceError::Invalid("Theme share strings must begin with codex-theme-v1:.".into()))?;
        let decoded;
        let json = if payload.starts_with('{') { payload } else { decoded = decode_uri_component(payload)?; &decoded };
        let mut share: ThemeShare = serde_json::from_str(json)
            .map_err(|error| WorkspaceError::Invalid(format!("Invalid theme share string: {error}")))?;
        share.code_theme_id = share.code_theme_id.trim().to_lowercase();
        for font in [&mut share.theme.fonts.ui, &mut share.theme.fonts.code] {
            *font = font.take().and_then(|value| {
                let value = value.trim();
                (!value.is_empty()).then(|| value.to_owned())
            });
        }
        if share.variant != target {
            return Err(WorkspaceError::Invalid(format!("Theme variant mismatch: expected {}.", target.label())));
        }
        let pack = ThemePack { code_theme_id: share.code_theme_id, theme: share.theme };
        pack.validate(target)?;
        *self.pack_mut(target) = pack;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ThemeShare { code_theme_id: String, theme: ChromeTheme, variant: ThemeVariant }

type Catalog = BTreeMap<String, BTreeMap<ThemeVariant, ChromeTheme>>;
static CATALOG: OnceLock<Result<Catalog, String>> = OnceLock::new();
pub fn theme_catalog() -> WorkspaceResult<&'static Catalog> {
    CATALOG.get_or_init(|| {
        let catalog: Catalog = serde_json::from_str(include_str!("theme/catalog.json")).map_err(|error| error.to_string())?;
        if catalog.len() != THEME_OPTIONS.len() || THEME_OPTIONS.iter().any(|(id, _)| !catalog.contains_key(*id)) {
            return Err("Bundled theme catalog does not match its pinned manifest.".into());
        }
        for variants in catalog.values() {
            if variants.is_empty() { return Err("Theme catalog has an empty variant set.".into()); }
            for theme in variants.values() { theme.validate().map_err(|error| error.to_string())?; }
        }
        Ok(catalog)
    }).as_ref().map_err(|error| WorkspaceError::Invalid(error.clone()))
}

pub const THEME_OPTIONS: &[(&str, &str)] = &[
    ("absolutely", "Absolutely"), ("ayu", "Ayu"), ("catppuccin", "Catppuccin"),
    ("codex", "Codex"), ("synara", "Synara"), ("dracula", "Dracula"),
    ("everforest", "Everforest"), ("github", "GitHub"), ("gruvbox", "Gruvbox"),
    ("linear", "Linear"), ("lobster", "Lobster"), ("material", "Material"),
    ("matrix", "Matrix"), ("monokai", "Monokai"), ("night-owl", "Night Owl"),
    ("nord", "Nord"), ("notion", "Notion"), ("one", "One"),
    ("oscurange", "Oscurange"), ("proof", "Proof"), ("raycast", "Raycast"),
    ("rose-pine", "Rose Pine"), ("sentry", "Sentry"), ("solarized", "Solarized"),
    ("temple", "Temple"), ("tokyo-night", "Tokyo Night"), ("vercel", "Vercel"),
    ("vscode-plus", "VS Code Plus"),
];
pub fn available_theme_options(variant: ThemeVariant) -> WorkspaceResult<Vec<(&'static str, &'static str)>> {
    let catalog = theme_catalog()?;
    Ok(THEME_OPTIONS.iter().copied().filter(|(id, _)| catalog.get(*id).is_some_and(|variants| variants.contains_key(&variant))).collect())
}
pub fn theme_label(id: &str) -> &str {
    THEME_OPTIONS.iter().find(|(candidate, _)| *candidate == id).map_or(id, |(_, label)| *label)
}

fn decode_uri_component(text: &str) -> WorkspaceResult<String> {
    let bytes = text.as_bytes();
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len() { return Err(WorkspaceError::Invalid("Truncated theme URI escape.".into())); }
            let digit = |byte: u8| (byte as char).to_digit(16).map(|value| value as u8);
            let a = digit(bytes[index + 1]);
            let b = digit(bytes[index + 2]);
            let (Some(a), Some(b)) = (a, b) else { return Err(WorkspaceError::Invalid("Invalid theme URI escape.".into())); };
            result.push(a * 16 + b);
            index += 3;
        } else { result.push(bytes[index]); index += 1; }
    }
    String::from_utf8(result).map_err(|_| WorkspaceError::Invalid("Theme share string is not valid UTF-8.".into()))
}

#[cfg(test)]
mod tests;
