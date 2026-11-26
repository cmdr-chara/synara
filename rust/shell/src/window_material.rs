//! Desktop window material policy, ported from upstream `windowMaterial.ts`.
//! GPUI exposes the platform-backed part of the material directly: opaque,
//! plain transparency, compositor/system blur, or Windows Mica. Adjustable
//! per-window blur radii are not part of GPUI's public window API, so the
//! shell applies the strongest native material available and keeps the theme
//! slider state authoritative for when the platform backend grows that control.

use gpui_kit::WindowBackgroundAppearance;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowTranslucency {
    /// macOS system-backed blur/vibrancy.
    Native,
    /// A transparent window on a compositor known to blur transparent surfaces.
    Compositor,
}

/// Upstream Hyprland gate: translucency is allowed only when the session is
/// provably Hyprland and `decoration:blur:enabled` is enabled.
pub fn resolve_translucency(
    platform: &str,
    hyprland_instance_signature: Option<&str>,
    xdg_current_desktop: Option<&str>,
    hyprland_blur_enabled: Option<bool>,
) -> Option<WindowTranslucency> {
    if platform == "macos" || platform == "darwin" {
        return Some(WindowTranslucency::Native);
    }
    if platform != "linux" {
        return None;
    }
    let hyprland = hyprland_instance_signature
        .map(str::trim)
        .is_some_and(|value| !value.is_empty())
        || xdg_current_desktop
            .unwrap_or("")
            .split(':')
            .any(|desktop| desktop.trim().eq_ignore_ascii_case("hyprland"));
    if hyprland && hyprland_blur_enabled == Some(true) {
        Some(WindowTranslucency::Compositor)
    } else {
        None
    }
}

/// Parses `hyprctl -j getoption decoration:blur:enabled` output.
pub fn parse_hyprland_blur_enabled(stdout: &str) -> Option<bool> {
    let value: Value = serde_json::from_str(stdout).ok()?;
    value
        .get("bool")
        .and_then(Value::as_bool)
        .or_else(|| value.get("int").and_then(Value::as_i64).map(|v| v != 0))
}

fn pack_opaque_windows(pack: &Value) -> bool {
    pack.get("opaqueWindows")
        .and_then(Value::as_bool)
        .or_else(|| {
            pack.get("theme")
                .and_then(|theme| theme.get("opaqueWindows"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(false)
}

/// Resolves the platform window backing for one theme pack. The blur slider's
/// presence selects glass; `blur: null` and `opaqueWindows` keep upstream's
/// opaque/native-material fallback semantics.
pub fn background_for_pack(
    translucency: Option<WindowTranslucency>,
    pack: &Value,
) -> WindowBackgroundAppearance {
    let Some(translucency_kind) = translucency else {
        return WindowBackgroundAppearance::Opaque;
    };
    if pack_opaque_windows(pack) {
        return WindowBackgroundAppearance::Opaque;
    }
    let glass_disabled = pack
        .get("translucency")
        .and_then(|value| value.get("blur"))
        .is_some_and(Value::is_null);
    if glass_disabled {
        return WindowBackgroundAppearance::Opaque;
    }
    match translucency_kind {
        WindowTranslucency::Native => WindowBackgroundAppearance::Blurred,
        WindowTranslucency::Compositor => WindowBackgroundAppearance::Transparent,
    }
}

pub fn current_translucency() -> Option<WindowTranslucency> {
    static RESOLVED: std::sync::OnceLock<Option<WindowTranslucency>> = std::sync::OnceLock::new();
    *RESOLVED.get_or_init(|| {
        #[cfg(target_os = "macos")]
        {
            return Some(WindowTranslucency::Native);
        }
        #[cfg(target_os = "linux")]
        {
            let blur_enabled = std::process::Command::new("hyprctl")
                .args(["-j", "getoption", "decoration:blur:enabled"])
                .output()
                .ok()
                .and_then(|output| String::from_utf8(output.stdout).ok())
                .and_then(|stdout| parse_hyprland_blur_enabled(&stdout));
            return resolve_translucency(
                "linux",
                std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok().as_deref(),
                std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
                blur_enabled,
            );
        }
        #[allow(unreachable_code)]
        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn resolves_only_supported_platforms() {
        assert_eq!(
            resolve_translucency("macos", None, None, None),
            Some(WindowTranslucency::Native)
        );
        assert_eq!(resolve_translucency("windows", None, None, None), None);
        assert_eq!(
            resolve_translucency("linux", Some("abc"), None, Some(true)),
            Some(WindowTranslucency::Compositor)
        );
        assert_eq!(
            resolve_translucency("linux", None, Some("GNOME:Hyprland"), Some(true)),
            Some(WindowTranslucency::Compositor)
        );
        assert_eq!(
            resolve_translucency("linux", Some("abc"), None, Some(false)),
            None
        );
        assert_eq!(resolve_translucency("linux", Some("abc"), None, None), None);
    }

    #[test]
    fn parses_hyprland_option() {
        assert_eq!(parse_hyprland_blur_enabled(r#"{"bool":true}"#), Some(true));
        assert_eq!(parse_hyprland_blur_enabled(r#"{"int":0}"#), Some(false));
        assert_eq!(parse_hyprland_blur_enabled("not json"), None);
    }

    #[test]
    fn maps_pack_to_platform_background() {
        assert_eq!(
            background_for_pack(Some(WindowTranslucency::Native), &json!({})),
            WindowBackgroundAppearance::Blurred
        );
        assert_eq!(
            background_for_pack(Some(WindowTranslucency::Compositor), &json!({})),
            WindowBackgroundAppearance::Transparent
        );
        assert_eq!(
            background_for_pack(
                Some(WindowTranslucency::Native),
                &json!({"opaqueWindows": true})
            ),
            WindowBackgroundAppearance::Opaque
        );
        assert_eq!(
            background_for_pack(
                Some(WindowTranslucency::Native),
                &json!({"translucency": {"blur": null}})
            ),
            WindowBackgroundAppearance::Opaque
        );
        assert_eq!(
            background_for_pack(None, &json!({})),
            WindowBackgroundAppearance::Opaque
        );
    }
}
