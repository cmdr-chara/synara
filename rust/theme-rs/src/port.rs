use serde_json::{Map, Value};
use std::sync::OnceLock;

const THEME_SHARE_PREFIX: &str = "codex-theme-v1:";
const CONTRAST_BASELINE_DARK: f64 = 60.0;
const CONTRAST_BASELINE_LIGHT: f64 = 45.0;
const BLUR_MIN: f64 = 1.0;
const BLUR_MAX: f64 = 64.0;

const CODE_THEME_OPTIONS: [(&str, &str, &str); 28] = [
    ("absolutely", "Absolutely", "light,dark"),
    ("ayu", "Ayu", "dark"),
    ("catppuccin", "Catppuccin", "light,dark"),
    ("codex", "Codex", "light,dark"),
    ("synara", "Synara", "light,dark"),
    ("dracula", "Dracula", "dark"),
    ("everforest", "Everforest", "light,dark"),
    ("github", "GitHub", "light,dark"),
    ("gruvbox", "Gruvbox", "light,dark"),
    ("linear", "Linear", "light,dark"),
    ("lobster", "Lobster", "dark"),
    ("material", "Material", "dark"),
    ("matrix", "Matrix", "dark"),
    ("monokai", "Monokai", "dark"),
    ("night-owl", "Night Owl", "dark"),
    ("nord", "Nord", "dark"),
    ("notion", "Notion", "light,dark"),
    ("one", "One", "light,dark"),
    ("oscurange", "Oscurange", "dark"),
    ("proof", "Proof", "light"),
    ("raycast", "Raycast", "light,dark"),
    ("rose-pine", "Rose Pine", "light,dark"),
    ("sentry", "Sentry", "dark"),
    ("solarized", "Solarized", "light,dark"),
    ("temple", "Temple", "dark"),
    ("tokyo-night", "Tokyo Night", "dark"),
    ("vercel", "Vercel", "light,dark"),
    ("vscode-plus", "VS Code Plus", "light,dark"),
];

const MONO_STACK: &str = "\"JetBrains Mono Variable\", \"JetBrains Mono\", \"SF Mono\", \"SFMono-Regular\", Consolas, \"Liberation Mono\", Menlo, monospace";

fn seed_catalog() -> &'static Value {
    static CATALOG: OnceLock<Value> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(include_str!("seed.json")).unwrap_or(Value::Null))
}

fn member<'a>(v: &'a Value, key: &str) -> &'a Value {
    v.get(key).unwrap_or(&Value::Null)
}

fn member_u(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or_else(crate::js::undefined)
}

fn is_record(v: &Value) -> bool {
    v.is_object()
}

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        _ => !crate::js::is_undefined(v),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if crate::js::is_undefined(a) && crate::js::is_undefined(b) {
        return true;
    }
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    a == b
}

fn is_theme_variant_str(v: &Value) -> bool {
    v.as_str() == Some("light") || v.as_str() == Some("dark")
}

pub fn is_theme_mode(value: Value) -> Result<bool, String> {
    Ok(matches!(
        value.as_str(),
        Some("light") | Some("dark") | Some("system")
    ))
}

pub fn is_theme_variant(value: Value) -> Result<bool, String> {
    Ok(is_theme_variant_str(&value))
}

fn theme_variants(id_variants: &str) -> Vec<&str> {
    id_variants.split(',').collect()
}

pub fn get_available_code_themes(variant: String) -> Result<Vec<Value>, String> {
    let mut out = Vec::new();
    for (id, label, variants) in CODE_THEME_OPTIONS {
        if theme_variants(variants).contains(&variant.as_str()) {
            out.push(serde_json::json!({
                "id": id,
                "label": label,
                "variants": theme_variants(variants),
            }));
        }
    }
    Ok(out)
}

fn is_available(id: &str, variant: &str) -> bool {
    CODE_THEME_OPTIONS
        .iter()
        .any(|(o, _, v)| *o == id && theme_variants(v).contains(&variant))
}

pub fn is_code_theme_available(code_theme_id: String, variant: String) -> Result<bool, String> {
    let normalized = js_trim(&code_theme_id).to_lowercase();
    Ok(is_available(&normalized, &variant))
}

pub fn normalize_code_theme_id(
    code_theme_id: Value,
    variant: String,
    fallback: Value,
) -> Result<Value, String> {
    let normalized = code_theme_id
        .as_str()
        .map(|s| js_trim(s).to_lowercase())
        .unwrap_or_default();
    if is_available(&normalized, &variant) {
        return Ok(Value::String(normalized));
    }
    if crate::js::is_undefined(&fallback) {
        return Ok(default_theme_state()["codeThemeIds"]
            .get(&variant)
            .cloned()
            .unwrap_or_else(crate::js::undefined));
    }
    Ok(fallback)
}

fn normalize_font_selection(v: &Value) -> Value {
    match v.as_str() {
        Some(s) if !js_trim(s).is_empty() => Value::String(js_trim(s).to_string()),
        _ => Value::Null,
    }
}

fn normalize_theme_fonts_inner(v: &Value) -> Value {
    serde_json::json!({
        "code": normalize_font_selection(member(v, "code")),
        "ui": normalize_font_selection(member(v, "ui")),
    })
}

pub fn normalize_theme_fonts(value: Value) -> Result<Value, String> {
    Ok(normalize_theme_fonts_inner(if is_record(&value) {
        &value
    } else {
        &Value::Null
    }))
}

fn normalize_hex_color(v: &Value) -> Option<String> {
    let s = js_trim(v.as_str()?).to_lowercase();
    let b = s.as_bytes();
    if b.len() == 7 && b[0] == b'#' && b[1..].iter().all(|c| c.is_ascii_hexdigit()) {
        Some(s)
    } else {
        None
    }
}

fn normalize_semantic_colors_inner(v: &Value, fallback: &Value) -> Result<Value, String> {
    if fallback.is_null() {
        return Err("Cannot read properties of null (reading 'diffAdded')".into());
    }
    if crate::js::is_undefined(fallback) {
        return Err("Cannot read properties of undefined (reading 'diffAdded')".into());
    }
    let sem = if is_record(v) { v } else { &Value::Null };
    Ok(serde_json::json!({
        "diffAdded": normalize_hex_color(member(sem, "diffAdded")).map(Value::String).unwrap_or_else(|| member_u(fallback, "diffAdded")),
        "diffRemoved": normalize_hex_color(member(sem, "diffRemoved")).map(Value::String).unwrap_or_else(|| member_u(fallback, "diffRemoved")),
        "skill": normalize_hex_color(member(sem, "skill")).map(Value::String).unwrap_or_else(|| member_u(fallback, "skill")),
    }))
}

pub fn normalize_semantic_colors(value: Value, fallback: Value) -> Result<Value, String> {
    normalize_semantic_colors_inner(&value, &fallback)
}

fn default_chrome(variant: &str) -> Value {
    if variant == "light" {
        serde_json::json!({
            "accent": "#339cff", "contrast": 0, "fonts": {"code": null, "ui": null},
            "ink": "#1a1c1f", "opaqueWindows": false,
            "semanticColors": {"diffAdded": "#00a240", "diffRemoved": "#ba2623", "skill": "#924ff7"},
            "surface": "#ffffff",
        })
    } else {
        serde_json::json!({
            "accent": "#339cff", "contrast": 0, "fonts": {"code": null, "ui": null},
            "ink": "#ffffff", "opaqueWindows": false,
            "semanticColors": {"diffAdded": "#40c977", "diffRemoved": "#fa423e", "skill": "#ad7bf9"},
            "surface": "#181818",
        })
    }
}

fn normalize_stored_contrast(v: &Value, fallback: f64) -> f64 {
    match crate::js::as_number(v) {
        Some(n) if n.is_finite() => js_round(n.clamp(0.0, 100.0)),
        _ => fallback,
    }
}

fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

fn normalize_integer_in_range(v: &Value, min: f64, max: f64, fallback: f64) -> f64 {
    match crate::js::as_number(v) {
        Some(n) if n.is_finite() => js_round(n.clamp(min, max)),
        _ => fallback,
    }
}

fn normalize_chrome_theme_inner(value: &Value, variant: &str) -> Result<Value, String> {
    if variant != "light" && variant != "dark" {
        return Err("Cannot read properties of undefined (reading 'accent')".into());
    }
    let fallback = default_chrome(variant);
    let theme = if is_record(value) {
        value
    } else {
        &Value::Null
    };
    let opaque = member(theme, "opaqueWindows");
    Ok(serde_json::json!({
        "accent": normalize_hex_color(member(theme, "accent")).map(Value::String).unwrap_or_else(|| member(&fallback, "accent").clone()),
        "contrast": normalize_stored_contrast(member(theme, "contrast"), member(&fallback, "contrast").as_f64().unwrap_or(0.0)),
        "fonts": normalize_theme_fonts_inner(member(theme, "fonts")),
        "ink": normalize_hex_color(member(theme, "ink")).map(Value::String).unwrap_or_else(|| member(&fallback, "ink").clone()),
        "opaqueWindows": if opaque.is_boolean() { opaque.clone() } else { member(&fallback, "opaqueWindows").clone() },
        "semanticColors": normalize_semantic_colors_inner(member(theme, "semanticColors"), member(&fallback, "semanticColors")).unwrap_or_else(|_| member(&fallback, "semanticColors").clone()),
        "surface": normalize_hex_color(member(theme, "surface")).map(Value::String).unwrap_or_else(|| member(&fallback, "surface").clone()),
    }))
}

pub fn normalize_chrome_theme(value: Value, variant: String) -> Result<Value, String> {
    normalize_chrome_theme_inner(&value, &variant)
}

fn normalize_window_translucency_inner(value: &Value, variant: &str) -> Result<Value, String> {
    if variant != "light" && variant != "dark" {
        return Err("Cannot read properties of undefined (reading 'opacity')".into());
    }
    let t = if is_record(value) {
        value
    } else {
        &Value::Null
    };
    let blur_raw = member_u(t, "blur");
    let blur = match crate::js::as_number(&blur_raw) {
        Some(n) if n.is_finite() => Value::from(normalize_integer_in_range(
            &blur_raw, BLUR_MIN, BLUR_MAX, BLUR_MIN,
        )),
        _ if blur_raw.is_null() => Value::Null,
        _ => Value::from(64.0),
    };
    let sidebar = member(t, "sidebarOnly");
    Ok(serde_json::json!({
        "opacity": normalize_integer_in_range(member(t, "opacity"), 15.0, 100.0, 90.0),
        "blur": blur,
        "sidebarOnly": if sidebar.is_boolean() { sidebar.clone() } else { Value::Bool(false) },
    }))
}

pub fn normalize_window_translucency(value: Value, variant: String) -> Result<Value, String> {
    normalize_window_translucency_inner(&value, &variant)
}

fn normalize_theme_pack_inner(value: &Value, variant: &str) -> Result<Value, String> {
    let pack = if is_record(value) {
        value
    } else {
        &Value::Null
    };
    let theme = normalize_chrome_theme_inner(member(pack, "theme"), variant)?;
    Ok(serde_json::json!({
        "codeThemeId": normalize_code_theme_id(
            member(pack, "codeThemeId").clone(),
            variant.to_string(),
            Value::String("codex".to_string()),
        )?,
        "theme": theme,
    }))
}

pub fn normalize_theme_pack(value: Value, variant: String) -> Result<Value, String> {
    normalize_theme_pack_inner(&value, &variant)
}

fn seed_theme(id: &str, variant: &str) -> Value {
    let seed = seed_catalog()
        .get(id)
        .and_then(|v| v.get(variant))
        .unwrap_or(&Value::Null);
    if seed.is_null() {
        default_chrome(variant)
    } else {
        normalize_chrome_theme_inner(seed, variant).unwrap_or_else(|_| default_chrome(variant))
    }
}

pub fn get_code_theme_seed(code_theme_id: String, variant: String) -> Result<Value, String> {
    let theme_seed = seed_catalog()
        .get(&code_theme_id)
        .and_then(|v| v.get(&variant));
    match theme_seed {
        Some(seed) => normalize_chrome_theme_inner(seed, &variant),
        None => {
            if variant == "light" || variant == "dark" {
                Ok(default_chrome(&variant))
            } else {
                Ok(crate::js::undefined())
            }
        }
    }
}

fn default_translucency() -> Value {
    serde_json::json!({"opacity": 90, "blur": 64, "sidebarOnly": false})
}

fn default_theme_state() -> Value {
    serde_json::json!({
        "chromeThemes": {
            "dark": seed_theme("synara", "dark"),
            "light": seed_theme("synara", "light"),
        },
        "codeThemeIds": {"dark": "synara", "light": "synara"},
        "systemUiFont": true,
        "mode": "system",
        "translucency": {"dark": default_translucency(), "light": default_translucency()},
    })
}

fn has_stored_custom_ui_font(state: &Value) -> bool {
    let chrome_themes = member(state, "chromeThemes");
    let packs = member(state, "packs");
    ["dark", "light"].iter().any(|variant| {
        let chrome = member(chrome_themes, variant);
        let fonts = member(chrome, "fonts");
        if !normalize_font_selection(member(fonts, "ui")).is_null() {
            return true;
        }
        let pack = member(packs, variant);
        let theme = member(pack, "theme");
        let legacy_fonts = member(theme, "fonts");
        !normalize_font_selection(member(legacy_fonts, "ui")).is_null()
    })
}

pub fn normalize_theme_state(value: Value) -> Result<Value, String> {
    let state = if is_record(&value) {
        &value
    } else {
        &Value::Null
    };
    let code_theme_ids = member(state, "codeThemeIds");
    let chrome_themes = member(state, "chromeThemes");
    let packs = member(state, "packs");
    let translucency = member(state, "translucency");
    let legacy_dark = normalize_theme_pack_inner(member(packs, "dark"), "dark")?;
    let legacy_light = normalize_theme_pack_inner(member(packs, "light"), "light")?;
    let dark_chrome = if is_record(member(chrome_themes, "dark")) {
        normalize_chrome_theme_inner(member(chrome_themes, "dark"), "dark")?
    } else if is_record(member(packs, "dark")) {
        member(&legacy_dark, "theme").clone()
    } else {
        seed_theme("codex", "dark")
    };
    let light_chrome = if is_record(member(chrome_themes, "light")) {
        normalize_chrome_theme_inner(member(chrome_themes, "light"), "light")?
    } else if is_record(member(packs, "light")) {
        member(&legacy_light, "theme").clone()
    } else {
        seed_theme("codex", "light")
    };
    let dark_id_input = {
        let v = member(code_theme_ids, "dark");
        if v.is_null() {
            member(&legacy_dark, "codeThemeId").clone()
        } else {
            v.clone()
        }
    };
    let light_id_input = {
        let v = member(code_theme_ids, "light");
        if v.is_null() {
            member(&legacy_light, "codeThemeId").clone()
        } else {
            v.clone()
        }
    };
    let mode = member(state, "mode");
    let system_ui_font = member(state, "systemUiFont");
    Ok(serde_json::json!({
        "chromeThemes": {"dark": dark_chrome, "light": light_chrome},
        "codeThemeIds": {
            "dark": normalize_code_theme_id(dark_id_input, "dark".to_string(), Value::String("codex".to_string()))?,
            "light": normalize_code_theme_id(light_id_input, "light".to_string(), Value::String("codex".to_string()))?,
        },
        "mode": if matches!(mode.as_str(), Some("light") | Some("dark") | Some("system")) { mode.clone() } else { Value::String("system".to_string()) },
        "systemUiFont": if system_ui_font.is_boolean() { system_ui_font.clone() } else { Value::Bool(!has_stored_custom_ui_font(state)) },
        "translucency": {
            "dark": normalize_window_translucency_inner(member(translucency, "dark"), "dark")?,
            "light": normalize_window_translucency_inner(member(translucency, "light"), "light")?,
        },
    }))
}

pub fn parse_stored_theme_state(raw_value: Option<String>) -> Result<Value, String> {
    let Some(raw) = raw_value else {
        return Ok(default_theme_state());
    };
    if raw.is_empty() {
        return Ok(default_theme_state());
    }
    if matches!(raw.as_str(), "light" | "dark" | "system") {
        return normalize_theme_state(serde_json::json!({"mode": raw}));
    }
    match serde_json::from_str::<Value>(&raw) {
        Ok(v) => normalize_theme_state(v),
        Err(_) => Ok(default_theme_state()),
    }
}

fn js_stringify(v: &Value) -> String {
    let mut out = String::new();
    write_json(v, &mut out);
    out
}

fn write_json(v: &Value, out: &mut String) {
    if crate::js::is_undefined(v) {
        out.push_str("null");
        return;
    }
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            if x.is_nan() || x.is_infinite() {
                out.push_str("null");
            } else {
                out.push_str(&crate::js::number_to_string(x));
            }
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap_or_default()),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_json(item, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            if let Some(inner) = marker_inner(m) {
                write_json(&inner, out);
                return;
            }
            out.push('{');
            let mut first = true;
            for (k, val) in m {
                if crate::js::is_undefined(val) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_json(val, out);
            }
            out.push('}');
        }
    }
}

fn marker_inner(m: &Map<String, Value>) -> Option<Value> {
    if m.len() != 1 {
        return None;
    }
    if let Some(Value::String(s)) = m.get("$num") {
        return Some(match s.as_str() {
            "NaN" => Value::Null,
            "Infinity" | "-Infinity" => Value::Null,
            _ => Value::from(s.parse::<f64>().unwrap_or(0.0)),
        });
    }
    if let Some(Value::String(s)) = m.get("$bigint") {
        return Some(Value::String(s.clone()));
    }
    if let Some(units) = m.get("$utf16").and_then(Value::as_array) {
        let s: String = units
            .iter()
            .filter_map(|u| u.as_u64())
            .map(|u| char::from_u32(u as u32).unwrap_or('\u{FFFD}'))
            .collect();
        return Some(Value::String(s));
    }
    None
}

pub fn serialize_theme_state(state: Value) -> Result<String, String> {
    Ok(js_stringify(&state))
}

pub fn create_theme_share_string(variant: String, pack: Value) -> Result<String, String> {
    if pack.is_null() {
        return Err("Cannot read properties of null (reading 'codeThemeId')".into());
    }
    if crate::js::is_undefined(&pack) {
        return Err("Cannot read properties of undefined (reading 'codeThemeId')".into());
    }
    let payload = serde_json::json!({
        "codeThemeId": member_u(&pack, "codeThemeId"),
        "theme": member_u(&pack, "theme"),
        "variant": variant,
    });
    Ok(format!("{THEME_SHARE_PREFIX}{}", js_stringify(&payload)))
}

fn decode_uri_component(s: &str) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err("URI malformed".into());
            }
            let hex = &s[i + 1..i + 3];
            let byte = u8::from_str_radix(hex, 16).map_err(|_| "URI malformed".to_string())?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "URI malformed".to_string())
}

fn parse_strict_fonts(v: &Value) -> Result<Value, String> {
    if !is_record(v) {
        return Err("Theme fonts must be an object.".into());
    }
    Ok(serde_json::json!({
        "code": parse_nullable_string(member(v, "code"), "Theme code font")?,
        "ui": parse_nullable_string(member(v, "ui"), "Theme UI font")?,
    }))
}

fn parse_nullable_string(v: &Value, label: &str) -> Result<Value, String> {
    if v.is_null() {
        return Ok(Value::Null);
    }
    match v.as_str() {
        Some(_) => Ok(normalize_font_selection(v)),
        None => Err(format!("{label} must be a string or null.")),
    }
}

fn parse_required_hex(v: &Value, label: &str) -> Result<String, String> {
    normalize_hex_color(v).ok_or_else(|| format!("{label} must be a 6-digit hex color."))
}

fn parse_strict_semantic_colors(v: &Value) -> Result<Value, String> {
    if !is_record(v) {
        return Err("Theme semanticColors must be an object.".into());
    }
    Ok(serde_json::json!({
        "diffAdded": parse_required_hex(member(v, "diffAdded"), "Theme diffAdded")?,
        "diffRemoved": parse_required_hex(member(v, "diffRemoved"), "Theme diffRemoved")?,
        "skill": parse_required_hex(member(v, "skill"), "Theme skill")?,
    }))
}

fn parse_strict_chrome_theme(v: &Value) -> Result<Value, String> {
    if !is_record(v) {
        return Err("Theme share theme must be an object.".into());
    }
    let contrast = match crate::js::as_number(member(v, "contrast")) {
        Some(n) if n.fract() == 0.0 && (0.0..=100.0).contains(&n) => n,
        _ => return Err("Theme contrast must be an integer between 0 and 100.".into()),
    };
    let opaque = member(v, "opaqueWindows");
    if !opaque.is_boolean() {
        return Err("Theme opaqueWindows must be a boolean.".into());
    }
    Ok(serde_json::json!({
        "accent": parse_required_hex(member(v, "accent"), "Theme accent")?,
        "contrast": contrast,
        "fonts": parse_strict_fonts(member(v, "fonts"))?,
        "ink": parse_required_hex(member(v, "ink"), "Theme ink")?,
        "opaqueWindows": opaque,
        "semanticColors": parse_strict_semantic_colors(member(v, "semanticColors"))?,
        "surface": parse_required_hex(member(v, "surface"), "Theme surface")?,
    }))
}

fn parse_theme_share_payload(v: &Value) -> Result<Value, String> {
    if !is_record(v) {
        return Err("Theme share payload must be an object.".into());
    }
    let raw_id = member(v, "codeThemeId");
    let trimmed = match raw_id.as_str() {
        Some(s) if !js_trim(s).is_empty() => js_trim(s).to_lowercase(),
        Some(_) => return Err("Theme share codeThemeId must not be empty.".into()),
        None => return Err("Theme share codeThemeId must be a string.".into()),
    };
    let variant = member(v, "variant");
    if !is_theme_variant_str(variant) {
        return Err("Theme share variant must be either light or dark.".into());
    }
    let theme = parse_strict_chrome_theme(member(v, "theme"))?;
    Ok(serde_json::json!({
        "codeThemeId": trimmed,
        "theme": theme,
        "variant": variant,
    }))
}

pub fn parse_theme_share_string(raw_value: String) -> Result<Value, String> {
    let value = js_trim(&raw_value);
    if !value.starts_with(THEME_SHARE_PREFIX) {
        return Err("Theme share string must start with codex-theme-v1:".into());
    }
    let payload_text = &value[THEME_SHARE_PREFIX.len()..];
    let json_text = if payload_text.starts_with('{') {
        payload_text.to_string()
    } else {
        decode_uri_component(payload_text)
            .map_err(|_| "Theme share string does not contain valid JSON.".to_string())?
    };
    let payload: Value = serde_json::from_str(&json_text)
        .map_err(|_| "Theme share string does not contain valid JSON.".to_string())?;
    let share = parse_theme_share_payload(&payload)?;
    let id = share["codeThemeId"].as_str().unwrap_or_default();
    let variant = share["variant"].as_str().unwrap_or_default();
    if !is_available(id, variant) {
        return Err(format!(
            "Code theme \"{id}\" is not available for {variant}."
        ));
    }
    let theme = normalize_chrome_theme_inner(&share["theme"], variant)?;
    Ok(serde_json::json!({
        "codeThemeId": share["codeThemeId"],
        "theme": theme,
        "variant": share["variant"],
    }))
}

pub fn can_parse_theme_share_string(
    value: String,
    target_variant: Option<String>,
) -> Result<bool, String> {
    Ok(parse_theme_share_string_for_variant(value, target_variant).is_ok())
}

pub fn parse_theme_share_string_for_variant(
    value: String,
    target_variant: Option<String>,
) -> Result<Value, String> {
    let payload = parse_theme_share_string(value)?;
    if let Some(target) = target_variant
        && !target.is_empty()
        && payload["variant"].as_str() != Some(target.as_str())
    {
        return Err(format!(
            "Theme variant mismatch. Expected {target}, received {}.",
            payload["variant"].as_str().unwrap_or_default()
        ));
    }
    Ok(payload)
}

fn spread_obj(v: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match v {
        Value::Object(m) => out.extend(m.iter().map(|(k, x)| (k.clone(), x.clone()))),
        Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                out.insert(i.to_string(), x.clone());
            }
        }
        Value::String(s) => {
            for (i, u) in s.encode_utf16().enumerate() {
                out.insert(
                    i.to_string(),
                    Value::String(char::from_u32(u as u32).unwrap_or('\u{FFFD}').to_string()),
                );
            }
        }
        _ => {}
    }
    out
}

static UNDEFINED_VAL: std::sync::LazyLock<Value> =
    std::sync::LazyLock::new(|| serde_json::json!({"$undefined": true}));

fn obj_member<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => {
            if crate::js::is_undefined(v) {
                Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ))
            } else {
                Ok(m.get(key).unwrap_or(&UNDEFINED_VAL))
            }
        }
        _ => Ok(&UNDEFINED_VAL),
    }
}

fn set_variant_key(state: &Value, map_key: &str, variant: &str, new_val: Value) -> Value {
    let mut out = spread_obj(state);
    let mut inner = spread_obj(member(state, map_key));
    inner.insert(variant.to_string(), new_val);
    out.insert(map_key.to_string(), Value::Object(inner));
    Value::Object(out)
}

pub fn update_theme_pack_from_share_string(
    state: Value,
    value: String,
    target_variant: String,
) -> Result<Value, String> {
    let payload = parse_theme_share_string_for_variant(value, Some(target_variant.clone()))?;
    let mut out = set_variant_key(
        &state,
        "chromeThemes",
        &target_variant,
        payload["theme"].clone(),
    );
    if let Value::Object(ref mut m) = out {
        let mut ids = spread_obj(member(&state, "codeThemeIds"));
        ids.insert(target_variant, payload["codeThemeId"].clone());
        m.insert("codeThemeIds".to_string(), Value::Object(ids));
    }
    Ok(out)
}

fn merge_theme_seed_patch(current: &Value, patch: &Value) -> Value {
    let mut out = spread_obj(current);
    for (k, v) in spread_obj(patch) {
        out.insert(k, v);
    }
    let patch_fonts = member(patch, "fonts");
    if js_truthy(patch_fonts) {
        let mut fonts = spread_obj(member(current, "fonts"));
        for (k, v) in spread_obj(patch_fonts) {
            fonts.insert(k, v);
        }
        out.insert("fonts".to_string(), Value::Object(fonts));
    } else {
        out.insert("fonts".to_string(), member(current, "fonts").clone());
    }
    let patch_sem = member(patch, "semanticColors");
    if js_truthy(patch_sem) {
        let mut sem = spread_obj(member(current, "semanticColors"));
        for (k, v) in spread_obj(patch_sem) {
            sem.insert(k, v);
        }
        out.insert("semanticColors".to_string(), Value::Object(sem));
    } else {
        out.insert(
            "semanticColors".to_string(),
            member(current, "semanticColors").clone(),
        );
    }
    Value::Object(out)
}

pub fn update_chrome_theme(state: Value, variant: String, patch: Value) -> Result<Value, String> {
    let previous = member(member(&state, "chromeThemes"), &variant);
    let merged = merge_theme_seed_patch(previous, &patch);
    Ok(set_variant_key(
        &state,
        "chromeThemes",
        &variant,
        normalize_chrome_theme_inner(&merged, &variant)?,
    ))
}

fn seed_patch_metadata(id: &str, variant: &str) -> (bool, bool, bool, bool) {
    // (contrast, opaqueWindows, fonts.code, fonts.ui)
    match (id, variant) {
        ("linear", "dark" | "light") => (false, true, false, true),
        ("lobster", "dark") => (false, false, false, true),
        ("matrix", "dark") | ("notion", "dark" | "light") | ("raycast", "dark" | "light") => {
            (false, true, true, true)
        }
        ("proof", "light") => (false, true, true, true),
        ("sentry", "dark") => (false, false, true, true),
        ("vercel", "dark" | "light") | ("synara", "dark" | "light") => (true, true, true, true),
        _ => (false, false, false, false),
    }
}

fn get_code_theme_seed_patch(id: &str, variant: &str) -> Value {
    let Some(seed) = seed_catalog().get(id).and_then(|v| v.get(variant)) else {
        return Value::Object(Map::new());
    };
    let normalized =
        normalize_chrome_theme_inner(seed, variant).unwrap_or_else(|_| default_chrome(variant));
    let (contrast_meta, opaque_meta, code_font, ui_font) = seed_patch_metadata(id, variant);
    let mut patch = Map::new();
    patch.insert("accent".to_string(), member(&normalized, "accent").clone());
    patch.insert("ink".to_string(), member(&normalized, "ink").clone());
    patch.insert(
        "semanticColors".to_string(),
        member(&normalized, "semanticColors").clone(),
    );
    patch.insert(
        "surface".to_string(),
        member(&normalized, "surface").clone(),
    );
    if contrast_meta {
        patch.insert(
            "contrast".to_string(),
            member(&normalized, "contrast").clone(),
        );
    }
    if opaque_meta {
        patch.insert(
            "opaqueWindows".to_string(),
            member(&normalized, "opaqueWindows").clone(),
        );
    }
    if code_font || ui_font {
        let mut fonts = Map::new();
        if code_font {
            fonts.insert(
                "code".to_string(),
                member(member(&normalized, "fonts"), "code").clone(),
            );
        }
        if ui_font {
            fonts.insert(
                "ui".to_string(),
                member(member(&normalized, "fonts"), "ui").clone(),
            );
        }
        patch.insert("fonts".to_string(), Value::Object(fonts));
    }
    Value::Object(patch)
}

fn resolve_theme_pack_inner(state: &Value, variant: &str) -> Result<Value, String> {
    let ids = obj_member(state, "codeThemeIds")?;
    if crate::js::is_undefined(ids) {
        return Err("Cannot read properties of undefined".into());
    }
    let themes = obj_member(state, "chromeThemes")?;
    if crate::js::is_undefined(themes) {
        return Err("Cannot read properties of undefined".into());
    }
    let theme = normalize_chrome_theme_inner(member(themes, variant), variant)?;
    Ok(serde_json::json!({
        "codeThemeId": normalize_code_theme_id(member(ids, variant).clone(), variant.to_string(), crate::js::undefined())?,
        "theme": theme,
    }))
}

pub fn set_theme_code_theme_id(
    state: Value,
    variant: String,
    code_theme_id: String,
) -> Result<Value, String> {
    let normalized = normalize_code_theme_id(
        Value::String(code_theme_id),
        variant.clone(),
        crate::js::undefined(),
    )?;
    let patch = get_code_theme_seed_patch(normalized.as_str().unwrap_or(""), &variant);
    let previous = member(&resolve_theme_pack_inner(&state, &variant)?, "theme").clone();
    let merged = merge_theme_seed_patch(&previous, &patch);
    let next = normalize_chrome_theme_inner(&merged, &variant)?;
    let mut out = set_variant_key(&state, "chromeThemes", &variant, next);
    if let Value::Object(ref mut m) = out {
        let mut ids = spread_obj(member(&state, "codeThemeIds"));
        ids.insert(variant, normalized);
        m.insert("codeThemeIds".to_string(), Value::Object(ids));
    }
    Ok(out)
}

pub fn set_theme_fonts(state: Value, variant: String, patch: Value) -> Result<Value, String> {
    let previous = member(member(&state, "chromeThemes"), &variant);
    let mut merged = spread_obj(previous);
    let mut fonts = spread_obj(member(previous, "fonts"));
    for (k, v) in spread_obj(&patch) {
        fonts.insert(k, v);
    }
    merged.insert("fonts".to_string(), Value::Object(fonts));
    Ok(set_variant_key(
        &state,
        "chromeThemes",
        &variant,
        normalize_chrome_theme_inner(&Value::Object(merged), &variant)?,
    ))
}

pub fn reset_theme_variant(state: Value, variant: String) -> Result<Value, String> {
    obj_member(&state, "chromeThemes")?;
    let valid = variant == "light" || variant == "dark";
    let theme_val = if valid {
        seed_theme("synara", &variant)
    } else {
        crate::js::undefined()
    };
    let mut out = set_variant_key(&state, "chromeThemes", &variant, theme_val);
    if let Value::Object(ref mut m) = out {
        let mut ids = spread_obj(member(&state, "codeThemeIds"));
        ids.insert(
            variant.clone(),
            if valid {
                Value::String("synara".to_string())
            } else {
                crate::js::undefined()
            },
        );
        m.insert("codeThemeIds".to_string(), Value::Object(ids));
        let mut tr = spread_obj(member(&state, "translucency"));
        tr.insert(
            variant,
            if valid {
                default_translucency()
            } else {
                crate::js::undefined()
            },
        );
        m.insert("translucency".to_string(), Value::Object(tr));
    }
    Ok(out)
}

pub fn set_window_translucency(
    state: Value,
    variant: String,
    patch: Value,
) -> Result<Value, String> {
    let current = member(member(&state, "translucency"), &variant);
    let mut merged = spread_obj(current);
    for (k, v) in spread_obj(&patch) {
        merged.insert(k, v);
    }
    Ok(set_variant_key(
        &state,
        "translucency",
        &variant,
        normalize_window_translucency_inner(&Value::Object(merged), &variant)?,
    ))
}

pub fn are_window_translucencies_equal(left: Value, right: Value) -> Result<bool, String> {
    Ok(strict_eq(
        obj_member(&left, "opacity")?,
        obj_member(&right, "opacity")?,
    ) && strict_eq(obj_member(&left, "blur")?, obj_member(&right, "blur")?)
        && strict_eq(
            obj_member(&left, "sidebarOnly")?,
            obj_member(&right, "sidebarOnly")?,
        ))
}

pub fn resolve_theme_pack(state: Value, variant: String) -> Result<Value, String> {
    resolve_theme_pack_inner(&state, &variant)
}

pub fn resolve_profile_theme_accent(state: Value) -> Result<Value, String> {
    let light = resolve_theme_pack_inner(&state, "light")?;
    let dark = resolve_theme_pack_inner(&state, "dark")?;
    Ok(serde_json::json!({
        "light": member(member(&light, "theme"), "accent"),
        "dark": member(member(&dark, "theme"), "accent"),
    }))
}

pub fn are_theme_packs_equal(left: Value, right: Value) -> Result<bool, String> {
    let lt = obj_member(&left, "theme")?;
    let rt = obj_member(&right, "theme")?;
    Ok(strict_eq(
        obj_member(&left, "codeThemeId")?,
        obj_member(&right, "codeThemeId")?,
    ) && strict_eq(obj_member(lt, "accent")?, obj_member(rt, "accent")?)
        && strict_eq(obj_member(lt, "contrast")?, obj_member(rt, "contrast")?)
        && strict_eq(
            obj_member(obj_member(lt, "fonts")?, "code")?,
            obj_member(obj_member(rt, "fonts")?, "code")?,
        )
        && strict_eq(
            obj_member(obj_member(lt, "fonts")?, "ui")?,
            obj_member(obj_member(rt, "fonts")?, "ui")?,
        )
        && strict_eq(obj_member(lt, "ink")?, obj_member(rt, "ink")?)
        && strict_eq(
            obj_member(lt, "opaqueWindows")?,
            obj_member(rt, "opaqueWindows")?,
        )
        && strict_eq(
            obj_member(obj_member(lt, "semanticColors")?, "diffAdded")?,
            obj_member(obj_member(rt, "semanticColors")?, "diffAdded")?,
        )
        && strict_eq(
            obj_member(obj_member(lt, "semanticColors")?, "diffRemoved")?,
            obj_member(obj_member(rt, "semanticColors")?, "diffRemoved")?,
        )
        && strict_eq(
            obj_member(obj_member(lt, "semanticColors")?, "skill")?,
            obj_member(obj_member(rt, "semanticColors")?, "skill")?,
        )
        && strict_eq(obj_member(lt, "surface")?, obj_member(rt, "surface")?))
}

pub fn resolve_theme_variant(mode: String, system_dark: bool) -> Result<String, String> {
    Ok(if mode == "system" {
        if system_dark { "dark" } else { "light" }.to_string()
    } else {
        mode
    })
}

// ─── Color math ───────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Rgb {
    red: f64,
    green: f64,
    blue: f64,
}

fn parse_hex_color(v: &Value) -> Result<Rgb, String> {
    let Some(s) = v.as_str() else {
        return Err("Cannot read properties of undefined (reading 'slice')".into());
    };
    let h = s.strip_prefix('#').unwrap_or(s);
    let byte = |i: usize| -> f64 {
        u8::from_str_radix(h.get(i..i + 2).unwrap_or(""), 16).unwrap_or(0) as f64
    };
    Ok(Rgb {
        red: byte(0),
        green: byte(2),
        blue: byte(4),
    })
}

fn mix_channel(from: f64, to: f64, amount: f64) -> f64 {
    js_round(from + (to - from) * amount)
}

fn mix_rgb(from: Rgb, to: Rgb, amount: f64) -> Rgb {
    let a = amount.clamp(0.0, 1.0);
    Rgb {
        red: mix_channel(from.red, to.red, a),
        green: mix_channel(from.green, to.green, a),
        blue: mix_channel(from.blue, to.blue, a),
    }
}

fn format_hex_channel(v: f64) -> String {
    if !v.is_finite() {
        return "NaN".to_string();
    }
    format!("{:02x}", v as i64)
}

fn format_hex(c: Rgb) -> String {
    format!(
        "#{}{}{}",
        format_hex_channel(c.red),
        format_hex_channel(c.green),
        format_hex_channel(c.blue)
    )
}

fn format_opaque_rgb(c: Rgb) -> String {
    format!(
        "rgb({}, {}, {})",
        crate::js::number_to_string(c.red),
        crate::js::number_to_string(c.green),
        crate::js::number_to_string(c.blue)
    )
}

fn format_alpha(v: f64) -> String {
    let clamped = v.clamp(0.0, 1.0);
    let s = format!("{clamped:.3}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

fn format_rgba(c: Rgb, opacity: f64) -> String {
    format!(
        "rgba({}, {}, {}, {})",
        crate::js::number_to_string(c.red),
        crate::js::number_to_string(c.green),
        crate::js::number_to_string(c.blue),
        format_alpha(opacity)
    )
}

fn mix_hex(from: &str, to: &str, amount: f64) -> Result<String, String> {
    let f = parse_hex_color(&Value::String(from.to_string()))?;
    let t = parse_hex_color(&Value::String(to.to_string()))?;
    Ok(format_hex(mix_rgb(f, t, amount)))
}

fn normalize_contrast_strength(value: f64, variant: &str) -> f64 {
    let baseline = if variant == "dark" {
        CONTRAST_BASELINE_DARK
    } else {
        CONTRAST_BASELINE_LIGHT
    };
    let baseline_ratio = baseline / 100.0;
    let curved = value / 100.0 + ((value - baseline) / 60.0) * 0.7;
    if value <= baseline {
        curved
    } else {
        baseline_ratio + (curved - baseline_ratio) * 2.0
    }
}

struct ComputedTheme {
    accent: Rgb,
    contrast: f64,
    editor_background: Rgb,
    ink: Rgb,
    surface_under: String,
    theme: Value,
}

fn build_computed_theme(theme: &Value, variant: &str) -> Result<ComputedTheme, String> {
    let raw_contrast = member(theme, "contrast");
    let contrast = normalize_contrast_strength(
        crate::js::as_number(raw_contrast).unwrap_or(f64::NAN),
        variant,
    );
    let surface = parse_hex_color(member(theme, "surface"))?;
    let ink = parse_hex_color(member(theme, "ink"))?;
    let accent = parse_hex_color(member(theme, "accent"))?;
    let white = Rgb {
        red: 255.0,
        green: 255.0,
        blue: 255.0,
    };
    let black = Rgb {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };
    let editor_background = if variant == "light" {
        mix_rgb(surface, white, 0.12)
    } else {
        mix_rgb(surface, ink, 0.07)
    };
    let baseline = if variant == "dark" {
        CONTRAST_BASELINE_DARK
    } else {
        CONTRAST_BASELINE_LIGHT
    };
    let (base_alpha, step) = if variant == "light" {
        (0.04, 0.0012)
    } else {
        (0.16, 0.0015)
    };
    let mix_amount =
        base_alpha + (crate::js::as_number(raw_contrast).unwrap_or(f64::NAN) - baseline) * step;
    let surface_under = if variant == "light" {
        format_hex(mix_rgb(surface, ink, mix_amount))
    } else {
        format_hex(mix_rgb(surface, black, mix_amount))
    };
    Ok(ComputedTheme {
        accent,
        contrast,
        editor_background,
        ink,
        surface_under,
        theme: theme.clone(),
    })
}

fn warning_color(variant: &str) -> &'static str {
    if variant == "light" {
        "#d97706"
    } else {
        "#f5b44a"
    }
}

fn build_light_derived(theme: &ComputedTheme) -> Map<String, Value> {
    let white = Rgb {
        red: 255.0,
        green: 255.0,
        blue: 255.0,
    };
    let black = Rgb {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };
    let surface = parse_hex_color(member(&theme.theme, "surface")).unwrap_or(white);
    let accent_hex = member(&theme.theme, "accent").as_str().unwrap_or("");
    let ink_hex = member(&theme.theme, "ink").as_str().unwrap_or("");
    let control_base = mix_rgb(surface, white, 0.09 + theme.contrast * 0.04);
    let elevated_secondary_base = mix_rgb(surface, white, 0.08 + theme.contrast * 0.08);
    let elevated_primary_base = mix_rgb(surface, white, 0.16 + theme.contrast * 0.12);
    let c = theme.contrast;
    let ink = theme.ink;
    let mh = |from: &str, to: &str, a: f64| -> Value {
        match mix_hex(from, to, a) {
            Ok(s) => Value::String(s),
            Err(_) => Value::Null,
        }
    };
    let mut d = Map::new();
    d.insert(
        "accentBackground".into(),
        mh("#ffffff", accent_hex, 0.11 + c * 0.04),
    );
    let _ = mh("#ffffff", accent_hex, 0.0);
    d.insert(
        "accentBackground".into(),
        mh(&format_hex(surface), accent_hex, 0.11 + c * 0.04),
    );
    d.insert(
        "accentBackgroundActive".into(),
        mh(&format_hex(surface), accent_hex, 0.13 + c * 0.05),
    );
    d.insert(
        "accentBackgroundHover".into(),
        mh(&format_hex(surface), accent_hex, 0.12 + c * 0.045),
    );
    d.insert(
        "border".into(),
        Value::String(format_rgba(ink, 0.09 + c * 0.04)),
    );
    d.insert("borderFocus".into(), Value::String(accent_hex.to_string()));
    d.insert(
        "borderHeavy".into(),
        Value::String(format_rgba(ink, 0.09 + c * 0.06)),
    );
    d.insert(
        "borderLight".into(),
        Value::String(format_rgba(ink, 0.07 + c * 0.02)),
    );
    d.insert(
        "buttonPrimaryBackground".into(),
        Value::String(ink_hex.to_string()),
    );
    d.insert(
        "buttonPrimaryBackgroundActive".into(),
        Value::String(format_rgba(ink, 0.1 + c * 0.12)),
    );
    d.insert(
        "buttonPrimaryBackgroundHover".into(),
        Value::String(format_rgba(ink, 0.05 + c * 0.06)),
    );
    d.insert(
        "buttonPrimaryBackgroundInactive".into(),
        Value::String(format_rgba(ink, 0.18 + c * 0.14)),
    );
    d.insert(
        "buttonSecondaryBackground".into(),
        Value::String(format_rgba(ink, 0.03)),
    );
    d.insert(
        "buttonSecondaryBackgroundActive".into(),
        Value::String(format_rgba(ink, 0.03 + c * 0.02)),
    );
    d.insert(
        "buttonSecondaryBackgroundHover".into(),
        Value::String(format_rgba(ink, 0.03)),
    );
    d.insert(
        "buttonSecondaryBackgroundInactive".into(),
        Value::String(format_rgba(ink, 0.01 + c * 0.02)),
    );
    d.insert(
        "buttonTertiaryBackground".into(),
        Value::String(format_rgba(ink, 0.0)),
    );
    d.insert(
        "buttonTertiaryBackgroundActive".into(),
        Value::String(format_rgba(ink, 0.16 + c * 0.08)),
    );
    d.insert(
        "buttonTertiaryBackgroundHover".into(),
        Value::String(format_rgba(ink, 0.08 + c * 0.04)),
    );
    d.insert(
        "controlBackground".into(),
        Value::String(format_rgba(control_base, 0.96)),
    );
    d.insert(
        "controlBackgroundOpaque".into(),
        Value::String(format_opaque_rgb(control_base)),
    );
    d.insert(
        "elevatedPrimary".into(),
        Value::String(format_rgba(elevated_primary_base, 0.96)),
    );
    d.insert(
        "elevatedPrimaryOpaque".into(),
        Value::String(format_opaque_rgb(elevated_primary_base)),
    );
    d.insert(
        "elevatedSecondary".into(),
        Value::String(format_rgba(ink, 0.04)),
    );
    d.insert(
        "elevatedSecondaryOpaque".into(),
        Value::String(format_opaque_rgb(elevated_secondary_base)),
    );
    d.insert("iconAccent".into(), Value::String(accent_hex.to_string()));
    d.insert("iconPrimary".into(), Value::String(ink_hex.to_string()));
    d.insert(
        "iconSecondary".into(),
        Value::String(format_rgba(ink, 0.65 + c * 0.1)),
    );
    d.insert(
        "iconTertiary".into(),
        Value::String(format_rgba(ink, 0.45 + c * 0.1)),
    );
    d.insert(
        "simpleScrim".into(),
        Value::String(format_rgba(black, 0.08 + c * 0.04)),
    );
    d.insert("textAccent".into(), Value::String(accent_hex.to_string()));
    d.insert(
        "textButtonPrimary".into(),
        member(&theme.theme, "surface").clone(),
    );
    d.insert(
        "textButtonSecondary".into(),
        Value::String(ink_hex.to_string()),
    );
    d.insert(
        "textButtonTertiary".into(),
        Value::String(format_rgba(ink, 0.45 + c * 0.1)),
    );
    d.insert("textForeground".into(), Value::String(ink_hex.to_string()));
    d.insert(
        "textForegroundSecondary".into(),
        Value::String(format_rgba(ink, 0.65 + c * 0.1)),
    );
    d.insert(
        "textForegroundTertiary".into(),
        Value::String(format_rgba(ink, 0.45 + c * 0.1)),
    );
    d
}

fn build_dark_derived(theme: &ComputedTheme) -> Map<String, Value> {
    let white = Rgb {
        red: 255.0,
        green: 255.0,
        blue: 255.0,
    };
    let accent_hex = member(&theme.theme, "accent").as_str().unwrap_or("");
    let ink_hex = member(&theme.theme, "ink").as_str().unwrap_or("");
    let surface_hex = member(&theme.theme, "surface").as_str().unwrap_or("");
    let control_base = mix_rgb(
        theme_surface(theme),
        theme.ink,
        0.06 + theme.contrast * 0.05,
    );
    let focus_base = mix_rgb(theme.accent, white, 0.3 + theme.contrast * 0.15);
    let elevated_primary_base = mix_rgb(
        theme_surface(theme),
        theme.ink,
        0.08 + theme.contrast * 0.08,
    );
    let c = theme.contrast;
    let ink = theme.ink;
    let mh = |from: &str, to: &str, a: f64| -> Value {
        match mix_hex(from, to, a) {
            Ok(s) => Value::String(s),
            Err(_) => Value::Null,
        }
    };
    let mut d = Map::new();
    d.insert(
        "accentBackground".into(),
        mh("#000000", accent_hex, 0.2 + c * 0.08),
    );
    d.insert(
        "accentBackgroundActive".into(),
        mh("#000000", accent_hex, 0.22 + c * 0.12),
    );
    d.insert(
        "accentBackgroundHover".into(),
        mh("#000000", accent_hex, 0.21 + c * 0.1),
    );
    d.insert(
        "border".into(),
        Value::String(format_rgba(ink, 0.1 + c * 0.04)),
    );
    d.insert(
        "borderFocus".into(),
        Value::String(format_opaque_rgb(focus_base)),
    );
    d.insert(
        "borderHeavy".into(),
        Value::String(format_rgba(ink, 0.16 + c * 0.06)),
    );
    d.insert(
        "borderLight".into(),
        Value::String(format_rgba(ink, 0.06 + c * 0.02)),
    );
    d.insert(
        "buttonPrimaryBackground".into(),
        Value::String(ink_hex.to_string()),
    );
    d.insert(
        "buttonPrimaryBackgroundActive".into(),
        Value::String(format_rgba(ink, 0.07 + c * 0.05)),
    );
    d.insert(
        "buttonPrimaryBackgroundHover".into(),
        Value::String(format_rgba(ink, 0.04 + c * 0.03)),
    );
    d.insert(
        "buttonPrimaryBackgroundInactive".into(),
        Value::String(format_rgba(ink, 0.02 + c * 0.02)),
    );
    d.insert(
        "buttonSecondaryBackground".into(),
        Value::String(format_rgba(ink, 0.04 + c * 0.02)),
    );
    d.insert(
        "buttonSecondaryBackgroundActive".into(),
        Value::String(format_rgba(ink, 0.09 + c * 0.05)),
    );
    d.insert(
        "buttonSecondaryBackgroundHover".into(),
        Value::String(format_rgba(ink, 0.06 + c * 0.03)),
    );
    d.insert(
        "buttonSecondaryBackgroundInactive".into(),
        Value::String(format_rgba(ink, 0.02 + c * 0.03)),
    );
    d.insert(
        "buttonTertiaryBackground".into(),
        Value::String(format_rgba(ink, 0.02 + c * 0.015)),
    );
    d.insert(
        "buttonTertiaryBackgroundActive".into(),
        Value::String(format_rgba(ink, 0.07 + c * 0.05)),
    );
    d.insert(
        "buttonTertiaryBackgroundHover".into(),
        Value::String(format_rgba(ink, 0.05 + c * 0.03)),
    );
    d.insert(
        "controlBackground".into(),
        Value::String(format_rgba(control_base, 0.96)),
    );
    d.insert(
        "controlBackgroundOpaque".into(),
        Value::String(format_opaque_rgb(control_base)),
    );
    d.insert(
        "elevatedPrimary".into(),
        Value::String(format_rgba(elevated_primary_base, 0.96)),
    );
    d.insert(
        "elevatedPrimaryOpaque".into(),
        Value::String(format_opaque_rgb(elevated_primary_base)),
    );
    d.insert(
        "elevatedSecondary".into(),
        Value::String(format_rgba(ink, 0.02 + c * 0.02)),
    );
    d.insert(
        "elevatedSecondaryOpaque".into(),
        mh(surface_hex, ink_hex, 0.04 + c * 0.05),
    );
    d.insert(
        "iconAccent".into(),
        Value::String(format_opaque_rgb(focus_base)),
    );
    d.insert(
        "iconPrimary".into(),
        Value::String(format_rgba(ink, 0.82 + c * 0.14)),
    );
    d.insert(
        "iconSecondary".into(),
        Value::String(format_rgba(ink, 0.65 + c * 0.1)),
    );
    d.insert(
        "iconTertiary".into(),
        Value::String(format_rgba(ink, 0.45 + c * 0.1)),
    );
    d.insert(
        "simpleScrim".into(),
        Value::String(format_rgba(ink, 0.08 + c * 0.04)),
    );
    d.insert(
        "textAccent".into(),
        Value::String(format_opaque_rgb(focus_base)),
    );
    d.insert(
        "textButtonPrimary".into(),
        member(&theme.theme, "surface").clone(),
    );
    d.insert(
        "textButtonSecondary".into(),
        Value::String(ink_hex.to_string()),
    );
    d.insert(
        "textButtonTertiary".into(),
        Value::String(format_rgba(ink, 0.45 + c * 0.1)),
    );
    d.insert("textForeground".into(), Value::String(ink_hex.to_string()));
    d.insert(
        "textForegroundSecondary".into(),
        Value::String(format_rgba(ink, 0.65 + c * 0.1)),
    );
    d.insert(
        "textForegroundTertiary".into(),
        Value::String(format_rgba(ink, 0.42 + c * 0.13)),
    );
    d
}

fn theme_surface(theme: &ComputedTheme) -> Rgb {
    parse_hex_color(member(&theme.theme, "surface")).unwrap_or(Rgb {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    })
}

fn js_string_of(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".to_string(),
        _ => crate::js::number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN)),
    }
}

fn build_codex_variables(
    theme: &ComputedTheme,
    derived: &Map<String, Value>,
    panel: &str,
    variant: &str,
) -> Map<String, Value> {
    let accent = member(&theme.theme, "accent")
        .as_str()
        .unwrap_or("")
        .to_string();
    let surface = member(&theme.theme, "surface")
        .as_str()
        .unwrap_or("")
        .to_string();
    let ink_s = member(&theme.theme, "ink")
        .as_str()
        .unwrap_or("")
        .to_string();
    let sem = member(&theme.theme, "semanticColors");
    let diff_added = member(sem, "diffAdded").as_str().unwrap_or("").to_string();
    let diff_removed = member(sem, "diffRemoved")
        .as_str()
        .unwrap_or("")
        .to_string();
    let skill = member(sem, "skill").as_str().unwrap_or("").to_string();
    let warning = warning_color(variant).to_string();
    let ansi_alpha = if variant == "light" { 0.15 } else { 0.23 };
    let ansi_green = mix_hex(&diff_added, "#000000", 0.18).unwrap_or_default();
    let get = |k: &str| -> Value { derived.get(k).cloned().unwrap_or(Value::Null) };
    let mut v = Map::new();

    v.insert(
        "--codex-base-accent".to_string(),
        Value::String(accent.clone()),
    );
    v.insert(
        "--codex-base-contrast".to_string(),
        Value::String(js_string_of(member(&theme.theme, "contrast"))),
    );
    v.insert("--codex-base-ink".to_string(), Value::String(ink_s.clone()));
    v.insert(
        "--codex-base-surface".to_string(),
        Value::String(surface.clone()),
    );
    v.insert(
        "--color-accent-blue".to_string(),
        Value::String(accent.clone()),
    );
    v.insert(
        "--color-accent-green".to_string(),
        Value::String(diff_added.clone()),
    );
    v.insert(
        "--color-accent-red".to_string(),
        Value::String(diff_removed.clone()),
    );
    v.insert(
        "--color-accent-purple".to_string(),
        Value::String(skill.clone()),
    );
    v.insert(
        "--color-accent-yellow".to_string(),
        Value::String(warning.clone()),
    );
    for (k, src) in [
        ("--color-background-accent", "accentBackground"),
        ("--color-background-accent-active", "accentBackgroundActive"),
        ("--color-background-accent-hover", "accentBackgroundHover"),
        (
            "--color-background-button-primary",
            "buttonPrimaryBackground",
        ),
        (
            "--color-background-button-primary-active",
            "buttonPrimaryBackgroundActive",
        ),
        (
            "--color-background-button-primary-hover",
            "buttonPrimaryBackgroundHover",
        ),
        (
            "--color-background-button-primary-inactive",
            "buttonPrimaryBackgroundInactive",
        ),
        (
            "--color-background-button-secondary",
            "buttonSecondaryBackground",
        ),
        (
            "--color-background-button-secondary-active",
            "buttonSecondaryBackgroundActive",
        ),
        (
            "--color-background-button-secondary-hover",
            "buttonSecondaryBackgroundHover",
        ),
        (
            "--color-background-button-secondary-inactive",
            "buttonSecondaryBackgroundInactive",
        ),
        (
            "--color-background-button-tertiary",
            "buttonTertiaryBackground",
        ),
        (
            "--color-background-button-tertiary-active",
            "buttonTertiaryBackgroundActive",
        ),
        (
            "--color-background-button-tertiary-hover",
            "buttonTertiaryBackgroundHover",
        ),
        ("--color-background-control", "controlBackground"),
        (
            "--color-background-control-opaque",
            "controlBackgroundOpaque",
        ),
        ("--color-background-elevated-primary", "elevatedPrimary"),
        (
            "--color-background-elevated-primary-opaque",
            "elevatedPrimaryOpaque",
        ),
        ("--color-background-elevated-secondary", "elevatedSecondary"),
        (
            "--color-background-elevated-secondary-opaque",
            "elevatedSecondaryOpaque",
        ),
        (
            "--color-background-user-message",
            "buttonSecondaryBackground",
        ),
        ("--color-border", "border"),
        ("--color-border-focus", "borderFocus"),
        ("--color-border-heavy", "borderHeavy"),
        ("--color-border-light", "borderLight"),
        ("--color-icon-accent", "iconAccent"),
        ("--color-icon-primary", "iconPrimary"),
        ("--color-icon-secondary", "iconSecondary"),
        ("--color-icon-tertiary", "iconTertiary"),
        ("--color-simple-scrim", "simpleScrim"),
        ("--color-text-accent", "textAccent"),
        ("--color-text-button-primary", "textButtonPrimary"),
        ("--color-text-button-secondary", "textButtonSecondary"),
        ("--color-text-button-tertiary", "textButtonTertiary"),
        ("--color-text-foreground", "textForeground"),
        (
            "--color-text-foreground-secondary",
            "textForegroundSecondary",
        ),
        ("--color-text-foreground-tertiary", "textForegroundTertiary"),
        ("--vscode-terminal-ansiBlack", "textForegroundTertiary"),
        (
            "--vscode-terminal-ansiBrightBlack",
            "textForegroundSecondary",
        ),
        ("--vscode-terminal-ansiBrightWhite", "textForeground"),
        ("--vscode-terminal-ansiWhite", "textForeground"),
        ("--vscode-terminal-border", "border"),
        ("--vscode-terminal-foreground", "textForeground"),
    ] {
        v.insert(k.to_string(), get(src));
    }
    v.insert(
        "--color-background-editor-opaque".to_string(),
        Value::String(format_opaque_rgb(theme.editor_background)),
    );
    v.insert(
        "--color-background-panel".to_string(),
        Value::String(panel.to_string()),
    );
    v.insert(
        "--color-background-surface".to_string(),
        Value::String(surface.clone()),
    );
    v.insert(
        "--color-background-surface-under".to_string(),
        Value::String(theme.surface_under.clone()),
    );
    v.insert(
        "--color-decoration-added".to_string(),
        Value::String(diff_added.clone()),
    );
    v.insert(
        "--color-decoration-deleted".to_string(),
        Value::String(diff_removed.clone()),
    );
    let added_rgb = parse_hex_color(&Value::String(diff_added.clone())).unwrap_or(Rgb {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    });
    let removed_rgb = parse_hex_color(&Value::String(diff_removed.clone())).unwrap_or(Rgb {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    });
    v.insert(
        "--color-editor-added".to_string(),
        Value::String(format_rgba(added_rgb, ansi_alpha)),
    );
    v.insert(
        "--color-editor-deleted".to_string(),
        Value::String(format_rgba(removed_rgb, ansi_alpha)),
    );
    v.insert(
        "--vscode-terminal-ansiBlue".to_string(),
        Value::String(accent.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiBrightBlue".to_string(),
        Value::String(accent.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiBrightCyan".to_string(),
        Value::String(accent.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiBrightGreen".to_string(),
        Value::String(ansi_green.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiBrightMagenta".to_string(),
        Value::String(skill.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiBrightRed".to_string(),
        Value::String(diff_removed.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiCyan".to_string(),
        Value::String(accent.clone()),
    );
    v.insert(
        "--vscode-terminal-ansiGreen".to_string(),
        Value::String(ansi_green),
    );
    v.insert(
        "--vscode-terminal-ansiMagenta".to_string(),
        Value::String(skill),
    );
    v.insert(
        "--vscode-terminal-ansiRed".to_string(),
        Value::String(diff_removed),
    );
    v.insert(
        "--vscode-terminal-ansiYellow".to_string(),
        Value::String(warning),
    );
    v.insert(
        "--vscode-terminal-background".to_string(),
        Value::String(surface),
    );
    v
}

fn build_aliases(vars: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let req = |k: &str| -> Result<String, String> {
        vars.get(k)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("Missing required theme variable: {k}"))
    };
    let mut a = Map::new();
    let pairs: [(&str, &str); 54] = [
        (
            "--color-token-badge-background",
            "--color-background-accent",
        ),
        ("--color-token-badge-foreground", "--color-text-foreground"),
        ("--color-token-border", "--color-border"),
        ("--color-token-border-default", "--color-border"),
        ("--color-token-border-heavy", "--color-border-heavy"),
        ("--color-token-border-light", "--color-border-light"),
        (
            "--color-token-button-background",
            "--color-background-button-primary",
        ),
        ("--color-token-button-border", "--color-border"),
        (
            "--color-token-button-foreground",
            "--color-text-button-primary",
        ),
        (
            "--color-token-button-secondary-hover-background",
            "--color-background-button-secondary-hover",
        ),
        (
            "--color-token-checkbox-active-background",
            "--color-background-accent-hover",
        ),
        (
            "--color-token-checkbox-active-foreground",
            "--color-text-foreground",
        ),
        (
            "--color-token-description-foreground",
            "--color-text-foreground-secondary",
        ),
        (
            "--color-token-disabled-foreground",
            "--color-text-foreground-tertiary",
        ),
        (
            "--color-token-dropdown-background",
            "--color-background-control-opaque",
        ),
        ("--color-token-focus-border", "--color-border-focus"),
        ("--color-token-foreground", "--color-text-foreground"),
        (
            "--color-token-input-background",
            "--color-background-control",
        ),
        ("--color-token-input-border", "--color-border"),
        ("--color-token-input-foreground", "--color-text-foreground"),
        (
            "--color-token-input-placeholder-foreground",
            "--color-text-foreground-tertiary",
        ),
        ("--color-token-link", "--color-text-accent"),
        (
            "--color-token-list-active-selection-background",
            "--color-background-button-secondary",
        ),
        (
            "--color-token-list-active-selection-foreground",
            "--color-text-foreground",
        ),
        (
            "--color-token-list-active-selection-icon-foreground",
            "--color-icon-primary",
        ),
        (
            "--color-token-list-hover-background",
            "--color-background-button-secondary-hover",
        ),
        (
            "--color-token-main-surface-primary",
            "--color-background-surface",
        ),
        (
            "--color-token-menu-background",
            "--color-background-elevated-primary",
        ),
        ("--color-token-menu-border", "--color-border"),
        (
            "--color-token-progress-bar-background",
            "--color-background-accent",
        ),
        (
            "--color-token-radio-active-foreground",
            "--color-icon-accent",
        ),
        (
            "--color-token-scrollbar-slider-active-background",
            "--color-border-heavy",
        ),
        (
            "--color-token-scrollbar-slider-background",
            "--color-border-light",
        ),
        (
            "--color-token-scrollbar-slider-hover-background",
            "--color-border",
        ),
        (
            "--color-token-side-bar-background",
            "--color-background-surface",
        ),
        (
            "--color-token-terminal-ansi-black",
            "--vscode-terminal-ansiBlack",
        ),
        (
            "--color-token-terminal-ansi-blue",
            "--vscode-terminal-ansiBlue",
        ),
        (
            "--color-token-terminal-ansi-bright-black",
            "--vscode-terminal-ansiBrightBlack",
        ),
        (
            "--color-token-terminal-ansi-bright-blue",
            "--vscode-terminal-ansiBrightBlue",
        ),
        (
            "--color-token-terminal-ansi-bright-cyan",
            "--vscode-terminal-ansiBrightCyan",
        ),
        (
            "--color-token-terminal-ansi-bright-green",
            "--vscode-terminal-ansiBrightGreen",
        ),
        (
            "--color-token-terminal-ansi-bright-magenta",
            "--vscode-terminal-ansiBrightMagenta",
        ),
        (
            "--color-token-terminal-ansi-bright-red",
            "--vscode-terminal-ansiBrightRed",
        ),
        (
            "--color-token-terminal-ansi-bright-white",
            "--vscode-terminal-ansiBrightWhite",
        ),
        (
            "--color-token-terminal-ansi-bright-yellow",
            "--vscode-terminal-ansiBrightYellow",
        ),
        (
            "--color-token-terminal-ansi-cyan",
            "--vscode-terminal-ansiCyan",
        ),
        (
            "--color-token-terminal-ansi-green",
            "--vscode-terminal-ansiGreen",
        ),
        (
            "--color-token-terminal-ansi-magenta",
            "--vscode-terminal-ansiMagenta",
        ),
        (
            "--color-token-terminal-ansi-red",
            "--vscode-terminal-ansiRed",
        ),
        (
            "--color-token-terminal-ansi-white",
            "--vscode-terminal-ansiWhite",
        ),
        (
            "--color-token-terminal-ansi-yellow",
            "--vscode-terminal-ansiYellow",
        ),
        (
            "--color-token-terminal-background",
            "--vscode-terminal-background",
        ),
        ("--color-token-terminal-border", "--vscode-terminal-border"),
        (
            "--color-token-terminal-foreground",
            "--vscode-terminal-foreground",
        ),
    ];
    for (k, src) in pairs {
        a.insert(k.to_string(), Value::String(req(src)?));
    }
    a.insert(
        "--color-token-text-code-block-background".to_string(),
        Value::String(req("--color-background-elevated-secondary-opaque")?),
    );
    a.insert(
        "--color-token-text-link-active-foreground".to_string(),
        Value::String(req("--color-text-accent")?),
    );
    a.insert(
        "--color-token-text-link-foreground".to_string(),
        Value::String(req("--color-text-accent")?),
    );
    a.insert(
        "--color-token-text-primary".to_string(),
        Value::String(req("--color-text-foreground")?),
    );
    a.insert(
        "--color-token-text-secondary".to_string(),
        Value::String(req("--color-text-foreground-secondary")?),
    );
    a.insert(
        "--color-token-text-tertiary".to_string(),
        Value::String(req("--color-text-foreground-tertiary")?),
    );
    a.insert(
        "--color-token-toolbar-hover-background".to_string(),
        Value::String(req("--color-background-button-tertiary-hover")?),
    );
    a.insert(
        "--color-token-editor-background".to_string(),
        Value::String(req("--color-background-editor-opaque")?),
    );
    a.insert(
        "--color-token-editor-foreground".to_string(),
        Value::String(req("--color-text-foreground")?),
    );
    Ok(a)
}

fn build_resolved_theme_tokens_inner(pack: &Value, variant: &str) -> Result<Value, String> {
    let theme = member(pack, "theme");
    let computed = build_computed_theme(theme, variant)?;
    let derived = if variant == "light" {
        build_light_derived(&computed)
    } else {
        build_dark_derived(&computed)
    };
    let panel = build_panel_variant(&computed, variant)?;
    let codex = build_codex_variables(&computed, &derived, &panel, variant);
    let aliases = build_aliases(&codex)?;
    Ok(serde_json::json!({
        "aliases": aliases,
        "codexVariables": codex,
        "computed": {
            "contrast": computed.contrast,
            "editorBackground": format_opaque_rgb(computed.editor_background),
            "panel": panel,
            "surfaceUnder": computed.surface_under,
        },
        "derived": derived,
    }))
}

fn build_panel_variant(theme: &ComputedTheme, variant: &str) -> Result<String, String> {
    let surface = member(&theme.theme, "surface")
        .as_str()
        .unwrap_or("")
        .to_string();
    let anchor = if variant == "light" {
        "#ffffff".to_string()
    } else {
        format_hex(theme.ink)
    };
    let (base, step) = if variant == "light" {
        (0.18, 0.008)
    } else {
        (0.03, 0.03)
    };
    mix_hex(&surface, &anchor, base + theme.contrast * step)
}

pub fn build_resolved_theme_tokens(pack: Value, variant: String) -> Result<Value, String> {
    build_resolved_theme_tokens_inner(&pack, &variant)
}

fn normalize_font_css_list(value: &str) -> String {
    let mut families: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut depth = 0i32;
    for ch in value.chars() {
        if let Some(q) = quote {
            current.push(ch);
            if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => {
                quote = Some(ch);
                current.push(ch);
            }
            '(' => {
                depth += 1;
                current.push(ch);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            ',' if depth == 0 => {
                families.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }
    families.push(current.trim().to_string());
    families
        .into_iter()
        .filter(|f| !f.is_empty())
        .map(|f| {
            let lower = f.to_lowercase();
            if f.starts_with('"')
                || f.starts_with('\'')
                || f.contains('(')
                || matches!(
                    lower.as_str(),
                    "inherit" | "initial" | "revert" | "revert-layer" | "unset"
                )
                || matches!(
                    lower.as_str(),
                    "cursive"
                        | "emoji"
                        | "fangsong"
                        | "fantasy"
                        | "math"
                        | "monospace"
                        | "sans-serif"
                        | "serif"
                        | "system-ui"
                        | "ui-monospace"
                        | "ui-rounded"
                        | "ui-sans-serif"
                        | "ui-serif"
                )
            {
                f
            } else if f.chars().any(|c| c.is_whitespace()) {
                format!("\"{}\"", f.replace('\\', "\\\\").replace('"', "\\\""))
            } else {
                f
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn normalize_font_family_css(value: &Value) -> Option<String> {
    let s = value.as_str()?;
    let trimmed = js_trim(s);
    if trimmed.is_empty() {
        return None;
    }
    Some(normalize_font_css_list(trimmed))
}

fn normalize_mono_font_css(value: &Value) -> Option<String> {
    let normalized = normalize_font_family_css(value)?;
    let lower = normalized.to_lowercase();
    if matches!(
        lower.as_str(),
        "inherit" | "initial" | "revert" | "revert-layer" | "unset"
    ) {
        return Some(normalized);
    }
    let has_generic = normalize_font_css_list(&normalized).split(", ").any(|f| {
        let unquoted = f
            .trim()
            .trim_start_matches(['"', '\''])
            .trim_end_matches(['"', '\'']);
        matches!(
            unquoted.to_lowercase().as_str(),
            "cursive"
                | "emoji"
                | "fangsong"
                | "fantasy"
                | "math"
                | "monospace"
                | "sans-serif"
                | "serif"
                | "system-ui"
                | "ui-monospace"
                | "ui-rounded"
                | "ui-sans-serif"
                | "ui-serif"
        )
    });
    Some(if has_generic {
        normalized
    } else {
        format!("{normalized}, {MONO_STACK}")
    })
}

pub fn build_theme_css_variables(
    pack: Value,
    variant: String,
    options: Option<Value>,
) -> Result<Value, String> {
    let resolved = build_resolved_theme_tokens_inner(&pack, &variant)?;
    let codex = member(&resolved, "codexVariables");
    let theme = member(&pack, "theme");
    let window_translucency = options
        .as_ref()
        .map(|o| member(o, "windowTranslucency"))
        .unwrap_or(&Value::Null);
    let opaque_windows = member(theme, "opaqueWindows");
    let material = if !window_translucency.is_null() && !js_truthy(opaque_windows) {
        "translucent"
    } else {
        "opaque"
    };
    let warning = warning_color(&variant);
    let translucency = options
        .as_ref()
        .map(|o| member(o, "translucency"))
        .filter(|t| !t.is_null() && !crate::js::is_undefined(t))
        .cloned()
        .unwrap_or_else(default_translucency);
    let translucent_opacity = member(&translucency, "opacity").as_f64().unwrap_or(90.0);
    let sidebar_only = member(&translucency, "sidebarOnly") == &Value::Bool(true);
    let scope = if material == "opaque" {
        "none"
    } else if sidebar_only {
        "sidebar"
    } else {
        "window"
    };
    let whole_window = scope == "window";
    let read = |k: &str| -> Result<String, String> {
        codex
            .get(k)
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("Missing required theme variable: {k}"))
    };
    let sidebar_surface = read("--color-background-surface")?;
    let glass_surface = if variant == "dark" {
        format!(
            "color-mix(in srgb, color-mix(in srgb, {sidebar_surface} 80%, black) {}%, transparent)",
            crate::js::number_to_string(translucent_opacity)
        )
    } else {
        format!(
            "color-mix(in srgb, {sidebar_surface} {}%, transparent)",
            crate::js::number_to_string(translucent_opacity)
        )
    };
    let composer_surface = if variant == "dark" {
        read("--color-background-control-opaque")?
    } else {
        "color-mix(in oklab, var(--color-background-control) 90%, transparent)".to_string()
    };
    let (floor, ratio) = if variant == "dark" {
        (10.0, 0.5)
    } else {
        (4.0, 0.3)
    };
    let raised_opacity = js_round(floor + translucent_opacity * ratio);
    let raised_surface = format!(
        "color-mix(in srgb, var(--popover) {}%, transparent)",
        crate::js::number_to_string(raised_opacity)
    );
    let overlay_surface = if whole_window {
        raised_surface.clone()
    } else {
        "color-mix(in srgb, var(--popover) 55%, transparent)".to_string()
    };
    let panel = member(member(&resolved, "computed"), "panel")
        .as_str()
        .unwrap_or_default()
        .to_string();
    let panel_rgb = parse_hex_color(&Value::String(panel.clone()))?;
    let anchor = if variant == "dark" {
        Rgb {
            red: 255.0,
            green: 255.0,
            blue: 255.0,
        }
    } else {
        parse_hex_color(member(theme, "ink"))?
    };
    let focus_contrast = normalize_contrast_strength(
        crate::js::as_number(member(theme, "contrast")).unwrap_or(f64::NAN),
        &variant,
    );
    let mix_amount = if variant == "dark" {
        0.12 + focus_contrast * 0.06
    } else {
        0.1 + focus_contrast * 0.05
    };
    let composer_focus_border = format_hex(mix_rgb(panel_rgb, anchor, mix_amount));
    let chat_code_surface = read("--color-background-user-message")?;
    let rail_ratio = if variant == "dark" {
        64.0 / 72.0
    } else {
        82.0 / 38.0
    };
    let rail_opacity = if whole_window {
        "0%".to_string()
    } else if material == "translucent" {
        format!(
            "{}%",
            crate::js::number_to_string(js_round(translucent_opacity * rail_ratio).min(100.0))
        )
    } else {
        "100%".to_string()
    };
    let mut vars = spread_obj(codex);
    for (k, v) in spread_obj(member(&resolved, "aliases")) {
        vars.insert(k, v);
    }
    let app_vars: Vec<(&str, String)> = vec![
        ("--accent", read("--color-background-accent")?),
        ("--accent-foreground", read("--color-text-foreground")?),
        (
            "--app-shell-background",
            if material == "translucent" {
                "transparent".to_string()
            } else {
                read("--color-background-surface-under")?
            },
        ),
        (
            "--app-window-background",
            if whole_window {
                glass_surface.clone()
            } else if material == "translucent" {
                "transparent".to_string()
            } else {
                read("--color-background-surface-under")?
            },
        ),
        (
            "--app-content-surface",
            if whole_window {
                "transparent".to_string()
            } else {
                read("--color-background-surface")?
            },
        ),
        ("--app-rail-shell-opacity", rail_opacity),
        ("--app-composer-focus-border", composer_focus_border),
        (
            "--app-glass-raised-surface",
            if whole_window {
                raised_surface
            } else {
                String::new()
            },
        ),
        (
            "--app-composer-picker-backdrop-filter",
            if material == "translucent" {
                "blur(32px)".to_string()
            } else {
                "none".to_string()
            },
        ),
        ("--app-composer-picker-surface", overlay_surface.clone()),
        ("--app-overlay-surface", overlay_surface),
        (
            "--app-overlay-backing",
            if scope == "sidebar" {
                glass_surface.clone()
            } else {
                String::new()
            },
        ),
        ("--app-chat-code-surface", chat_code_surface.clone()),
        ("--app-user-message-background", chat_code_surface),
        ("--app-settings-backdrop-filter", "none".to_string()),
        (
            "--app-sidebar-surface",
            if scope == "sidebar" {
                glass_surface.clone()
            } else if whole_window {
                "transparent".to_string()
            } else {
                sidebar_surface.clone()
            },
        ),
        (
            "--app-sidebar-chip-surface",
            if scope == "none" {
                sidebar_surface.clone()
            } else {
                glass_surface
            },
        ),
        (
            "--app-settings-surface",
            if whole_window {
                "transparent".to_string()
            } else {
                sidebar_surface
            },
        ),
        ("--background", read("--color-background-surface-under")?),
        ("--border", read("--color-border")?),
        ("--card", read("--color-background-panel")?),
        ("--card-foreground", read("--color-text-foreground")?),
        ("--composer-surface", composer_surface),
        (
            "--destructive",
            js_string_of(member(member(theme, "semanticColors"), "diffRemoved")),
        ),
        (
            "--destructive-foreground",
            js_string_of(member(theme, "surface")),
        ),
        ("--foreground", read("--color-text-foreground")?),
        ("--info", js_string_of(member(theme, "accent"))),
        ("--info-foreground", read("--color-text-accent")?),
        ("--input", read("--color-background-control-opaque")?),
        ("--muted", read("--color-background-elevated-secondary")?),
        (
            "--muted-foreground",
            read("--color-text-foreground-secondary")?,
        ),
        (
            "--popover",
            read("--color-background-elevated-primary-opaque")?,
        ),
        ("--popover-foreground", read("--color-text-foreground")?),
        ("--primary", read("--color-background-button-primary")?),
        ("--primary-foreground", read("--color-text-button-primary")?),
        ("--ring", read("--color-border-focus")?),
        ("--secondary", read("--color-background-button-secondary")?),
        (
            "--secondary-foreground",
            read("--color-text-button-secondary")?,
        ),
        ("--sidebar", read("--color-background-surface")?),
        (
            "--sidebar-accent",
            read("--color-background-button-secondary-hover")?,
        ),
        (
            "--sidebar-accent-active",
            read("--color-background-button-secondary-hover")?,
        ),
        (
            "--sidebar-selected",
            read("--color-background-user-message")?,
        ),
        (
            "--sidebar-accent-foreground",
            read("--color-text-foreground")?,
        ),
        ("--sidebar-border", read("--color-border")?),
        ("--sidebar-foreground", read("--color-text-foreground")?),
        (
            "--success",
            js_string_of(member(member(theme, "semanticColors"), "diffAdded")),
        ),
        (
            "--success-foreground",
            js_string_of(member(theme, "surface")),
        ),
        (
            "--theme-font-code-family",
            normalize_mono_font_css(member(member(theme, "fonts"), "code")).unwrap_or_default(),
        ),
        (
            "--theme-font-ui-family",
            if options
                .as_ref()
                .is_some_and(|o| member(o, "systemUiFont") == &Value::Bool(true))
            {
                String::new()
            } else {
                normalize_font_family_css(member(member(theme, "fonts"), "ui")).unwrap_or_default()
            },
        ),
        ("--warning", warning.to_string()),
        (
            "--warning-foreground",
            js_string_of(member(theme, "surface")),
        ),
    ];
    for (k, v) in app_vars {
        vars.insert(k.to_string(), Value::String(v));
    }
    Ok(serde_json::json!({
        "material": material,
        "translucencyScope": scope,
        "variables": vars,
    }))
}
