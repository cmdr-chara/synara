use synara_workspace::AppSettings;

#[test]
fn first_run_uses_the_electron_ui_base_size() {
    let settings = AppSettings::default();
    assert_eq!(settings.appearance.fonts.ui_size, 13.0);
    assert_eq!(settings.appearance.fonts.code_size, 13.0);
    settings.validate().unwrap();
}

#[test]
fn existing_explicit_font_sizes_are_not_migrated_or_overwritten() {
    let mut original = AppSettings::default();
    original.appearance.fonts.ui_size = 14.0;
    original.appearance.fonts.code_size = 17.0;
    original.appearance.fonts.ui_family = Some("Operator font".into());
    let decoded: AppSettings =
        serde_json::from_str(&serde_json::to_string(&original).unwrap()).unwrap();
    assert_eq!(decoded, original);
    decoded.validate().unwrap();
}

#[test]
fn old_minimal_settings_receive_defaults_without_changing_the_schema_version() {
    let decoded: AppSettings = serde_json::from_str(r#"{"version":1}"#).unwrap();
    assert_eq!(decoded.appearance.fonts.ui_size, 13.0);
    assert_eq!(decoded.version, 1);
    decoded.validate().unwrap();
}
