//! The Rust port of `packages/shared/src/serverSettings.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

/// A plain JSON object — arrays and primitives are not.
fn plain_object(v: &Value) -> bool {
    matches!(v, Value::Object(_)) && !crate::js::is_undefined(v)
}

/// `v.name` — throws on null/undefined receivers like JS property access.
fn member<'a>(v: &'a Value, name: &str) -> Result<&'a Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(name).unwrap_or(&NULL),
        _ => &NULL,
    })
}

static NULL: Value = Value::Null;

/// `v?.name` — optional chaining: nullish receivers yield undefined, which
/// keeps later property reads failing the way JS fails them.
fn member_opt<'a>(v: &'a Value, name: &str) -> &'a Value {
    static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    let undef = || UNDEF.get_or_init(crate::js::undefined);
    if nullish(v) {
        return undef();
    }
    match v {
        Value::Object(m) => m.get(name).unwrap_or_else(undef),
        _ => undef(),
    }
}

/// Port of `applyServerSettingsPatch` (`packages/shared/src/serverSettings.ts:30`).
///
/// `patch.textGenerationModelSelection` is evaluated before anything else —
/// null/undefined patches fail on that read. Non-object patches (arrays,
/// primitives) pass through unchanged; object patches merge over an object
/// `current`.
pub fn apply_server_settings_patch(current: Value, patch: Value) -> Result<Value, String> {
    member(&patch, "textGenerationModelSelection")?;
    if !plain_object(&patch) {
        return Ok(patch);
    }
    let mut out = Map::new();
    if let Value::Object(m) = &current {
        for (k, v) in m {
            out.insert(k.clone(), v.clone());
        }
    }
    if let Value::Object(m) = patch {
        for (k, v) in m {
            out.insert(k, v);
        }
    }
    Ok(Value::Object(out))
}

/// Port of `providerStartOptionsFromServerSettings` (`packages/shared/src/serverSettings.ts:108`).
///
/// `const { providers } = settings` — the destructure rejects null/undefined
/// `settings`; `providers.codex` is then read unconditionally.
pub fn provider_start_options_from_server_settings(settings: Value) -> Result<Value, String> {
    if nullish(&settings) {
        let kind = if settings.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!(
            "Cannot destructure property 'providers' of 'settings' as it is {kind}."
        ));
    }
    let providers = member_opt(&settings, "providers");
    let codex = member(providers, "codex")?;
    let mut out = Map::new();
    out.insert("codex".to_string(), codex.clone());
    Ok(Value::Object(out))
}
