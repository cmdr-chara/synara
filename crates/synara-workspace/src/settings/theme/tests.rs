use super::*;

#[test]
fn all_twenty_eight_reference_themes_have_valid_supported_variants() {
    let catalog = theme_catalog().unwrap();
    assert_eq!(catalog.len(), 28);
    for (id, _) in THEME_OPTIONS {
        for (&variant, theme) in &catalog[*id] {
            theme.validate().unwrap();
            ThemePack { code_theme_id: (*id).into(), theme: theme.clone() }.validate(variant).unwrap();
        }
    }
    assert_eq!(catalog["codex"][&ThemeVariant::Light], ThemePack::codex(ThemeVariant::Light).theme);
    assert_eq!(catalog["codex"][&ThemeVariant::Dark], ThemePack::codex(ThemeVariant::Dark).theme);
    assert!(!catalog["dracula"].contains_key(&ThemeVariant::Light));
    assert!(!catalog["proof"].contains_key(&ThemeVariant::Dark));
    assert!(available_theme_options(ThemeVariant::Dark).unwrap().contains(&("dracula", "Dracula")));
    assert!(!available_theme_options(ThemeVariant::Light).unwrap().iter().any(|(id, _)| *id == "dracula"));
}

#[test]
fn shares_round_trip_every_reference_seed_and_never_modify_the_other_variant() {
    for (id, _) in THEME_OPTIONS {
        for (&variant, theme) in &theme_catalog().unwrap()[*id] {
            let mut from = ThemePreferences::default();
            *from.pack_mut(variant) = ThemePack { code_theme_id: (*id).into(), theme: theme.clone() };
            let share = from.share(variant).unwrap();
            assert!(share.starts_with(THEME_SHARE_PREFIX));
            let mut to = ThemePreferences::default();
            to.import(&share, variant).unwrap();
            assert_eq!(to, from);
        }
    }
}

#[test]
fn selecting_a_color_seed_preserves_user_fonts_contrast_and_material_unless_the_reference_opts_in() {
    let mut settings = ThemePreferences::default();
    let theme = &mut settings.dark.theme;
    theme.contrast = 70;
    theme.fonts.ui = Some("Operator UI".into());
    theme.fonts.code = Some("Operator Mono".into());
    theme.opaque_windows = true;
    settings.select("dracula", ThemeVariant::Dark).unwrap();
    assert_eq!(settings.dark.theme.contrast, 70);
    assert_eq!(settings.dark.theme.fonts.ui.as_deref(), Some("Operator UI"));
    assert_eq!(settings.dark.theme.fonts.code.as_deref(), Some("Operator Mono"));
    assert!(settings.dark.theme.opaque_windows);
    assert_eq!(settings.dark.theme.surface.value(), 0x282a36);
    settings.select("synara", ThemeVariant::Dark).unwrap();
    assert_eq!(settings.dark.theme.contrast, 0);
    settings.select("matrix", ThemeVariant::Dark).unwrap();
    assert_eq!(settings.dark.theme.fonts, theme_catalog().unwrap()["matrix"][&ThemeVariant::Dark].fonts);
    assert_eq!(settings.dark.theme.opaque_windows, theme_catalog().unwrap()["matrix"][&ThemeVariant::Dark].opaque_windows);
}

#[test]
fn malformed_or_wrong_variant_import_is_atomic() {
    let baseline = ThemePreferences::default();
    let valid = baseline.share(ThemeVariant::Light).unwrap();
    let mut malformed = vec![
        String::new(),
        "codex-theme-v1:{".into(),
        "codex-theme-v1:%".into(),
        "codex-theme-v1:%GG".into(),
        "codex-theme-v1:%ff".into(),
        format!("codex-theme-v1:{}", "x".repeat(MAX_SHARE_BYTES)),
    ];
    for (field, value) in [
        ("contrast", serde_json::json!(101)),
        ("contrast", serde_json::json!(-1)),
        ("contrast", serde_json::json!(2.5)),
        ("accent", serde_json::json!("#fff")),
        ("accent", serde_json::json!("url(https://example.invalid)")),
        ("opaqueWindows", serde_json::json!("true")),
        ("fonts", serde_json::json!({"ui":"bad\nfont","code":null})),
    ] {
        let mut payload: serde_json::Value = serde_json::from_str(valid.strip_prefix(THEME_SHARE_PREFIX).unwrap()).unwrap();
        payload["theme"][field] = value;
        malformed.push(format!("{THEME_SHARE_PREFIX}{payload}"));
    }
    for text in malformed {
        let mut settings = baseline.clone();
        assert!(settings.import(&text, ThemeVariant::Light).is_err(), "{text}");
        assert_eq!(settings, baseline);
    }
    let mut settings = baseline.clone();
    assert!(settings.import(&valid, ThemeVariant::Dark).is_err());
    assert!(settings.select("dracula", ThemeVariant::Light).is_err());
    assert_eq!(settings, baseline);
}

#[test]
fn uri_encoded_theme_share_and_hex_normalization_match_the_reference() {
    let baseline = ThemePreferences::default();
    let json = baseline.share(ThemeVariant::Dark).unwrap().trim_start_matches(THEME_SHARE_PREFIX).to_owned();
    let encoded: String = json.bytes().map(|byte| format!("%{byte:02X}")).collect();
    let mut settings = baseline.clone();
    settings.import(&format!("{THEME_SHARE_PREFIX}{encoded}"), ThemeVariant::Dark).unwrap();
    assert_eq!(settings, baseline);
    assert_eq!(ThemeHex::parse(" #AABBCC ").unwrap().to_string(), "#aabbcc");
    for value in ["#abcd", "#00000000", "#GGGGGG", "red", "#é1234"] {
        assert!(ThemeHex::parse(value).is_err());
    }
}

#[test]
fn legacy_appearance_remains_legacy_and_new_profiles_use_the_electron_default() {
    let old: AppSettings = serde_json::from_value(serde_json::json!({
        "version": 1,
        "appearance": {"theme":"dark", "dark_theme":"dracula", "fonts":{"ui_size":16.0,"code_size":15.0}}
    })).unwrap();
    old.validate().unwrap();
    assert!(old.appearance.electron_theme.is_none());
    assert_eq!(old.appearance.dark_theme, DarkThemePreference::Dracula);
    assert_eq!(old.appearance.fonts.ui_size, 16.0);
    let fresh = AppSettings::default();
    fresh.validate().unwrap();
    assert_eq!(fresh.appearance.electron_theme, Some(ThemePreferences::default()));
}

#[tokio::test]
async fn theme_persistence_and_sharing_do_not_change_wallpaper_drafts_profile_or_permissions() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("theme-test.db");
    let service = WorkspaceService::open(path.clone()).await.unwrap();
    let mut settings = service.settings().await.unwrap().settings;
    settings.profile.name = "Theme operator".into();
    settings.chat.send_on_enter = false;
    settings.appearance.personalization.zen_mode = true;
    let mut themes = ThemePreferences::default();
    themes.select("dracula", ThemeVariant::Dark).unwrap();
    let share = themes.share(ThemeVariant::Dark).unwrap();
    settings.appearance.electron_theme = Some(themes.clone());
    service.save_settings(settings.clone()).await.unwrap();
    drop(service);
    let service = WorkspaceService::open(path).await.unwrap();
    let saved = service.settings().await.unwrap().settings;
    assert_eq!(saved, settings);
    let mut candidate = saved.clone();
    candidate.appearance.electron_theme.as_mut().unwrap().import(&share, ThemeVariant::Dark).unwrap();
    assert_eq!(candidate, saved);
    candidate.appearance.electron_theme.as_mut().unwrap().reset(ThemeVariant::Dark);
    assert_eq!(candidate.profile, saved.profile);
    assert_eq!(candidate.chat, saved.chat);
    assert_eq!(candidate.appearance.personalization, saved.appearance.personalization);
    assert_eq!(candidate.device, saved.device);
}
