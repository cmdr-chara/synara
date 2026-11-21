//! The Rust port of `packages/shared/src/account.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::Value;

/// `ACCOUNT_URL_ENV_NAME`.
const ACCOUNT_URL_ENV_NAME: &str = "SYNARA_ACCOUNT_URL";

/// `DEFAULT_ACCOUNT_URL`.
const DEFAULT_ACCOUNT_URL: &str = "https://api.synara.vrbty.dev";

fn is_undefined(v: &Value) -> bool {
    v.get("$undefined").is_some()
}

/// `String.prototype.trim` — Rust whitespace plus the BOM JS also strips.
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

/// JS `obj.key` where `obj` may be any value: member access on primitives and
/// arrays yields `undefined`; only objects look the key up.
fn prop<'a>(obj: &'a Value, key: &str) -> &'a Value {
    match obj {
        Value::Object(m) => m.get(key).unwrap_or(&Value::Null),
        _ => &Value::Null,
    }
}

/// `v?.trim()` for an already-read property: null/undefined short-circuits to
/// `undefined`, a string trims, anything else is a TypeError.
fn opt_trim<'a>(v: &'a Value, expr: &str) -> Result<Option<&'a str>, String> {
    match v {
        Value::Null => Ok(None),
        _ if is_undefined(v) => Ok(None),
        Value::String(s) => Ok(Some(js_trim(s))),
        _ => Err(format!("{expr} is not a function")),
    }
}

/// `resolveConfiguredAccountUrl` on a non-nullish input.
fn configured(input: &Value) -> Result<Option<String>, String> {
    // `input.flag?.trim()` — `flag` read is `undefined` on non-objects.
    if let Some(flag) = opt_trim(prop(input, "flag"), "input.flag?.trim")?
        && !flag.is_empty()
    {
        return Ok(Some(flag.to_string()));
    }
    // `(input.env ?? process.env)[ACCOUNT_URL_ENV_NAME]?.trim()`
    let env_prop = prop(input, "env");
    let real_env;
    let env: &Value = if env_prop.is_null() || is_undefined(env_prop) {
        real_env = match std::env::var(ACCOUNT_URL_ENV_NAME) {
            Ok(v) => serde_json::json!({ACCOUNT_URL_ENV_NAME: v}),
            Err(_) => serde_json::json!({}),
        };
        &real_env
    } else {
        env_prop
    };
    if let Some(from_env) = opt_trim(
        prop(env, ACCOUNT_URL_ENV_NAME),
        "(input.env ?? process.env)[ACCOUNT_URL_ENV_NAME]?.trim",
    )? && !from_env.is_empty()
    {
        return Ok(Some(from_env.to_string()));
    }
    Ok(None)
}

/// Port of `resolveConfiguredAccountUrl` (`packages/shared/src/account.ts:108`).
pub fn resolve_configured_account_url(input: serde_json::Value) -> Result<Option<String>, String> {
    match &input {
        Value::Null => Err("Cannot read properties of null (reading 'flag')".to_string()),
        _ if is_undefined(&input) => {
            Err("Cannot read properties of undefined (reading 'flag')".to_string())
        }
        _ => configured(&input),
    }
}

/// Port of `resolveAccountUrl` (`packages/shared/src/account.ts:116`).
///
/// `input = {}`: only `undefined` (or an absent argument) selects the default;
/// `null` is used as the input and fails on `.flag` like any other value.
pub fn resolve_account_url(input: serde_json::Value) -> Result<String, String> {
    if is_undefined(&input) {
        return Ok(DEFAULT_ACCOUNT_URL.to_string());
    }
    let resolved = resolve_configured_account_url(input)?;
    Ok(resolved.unwrap_or_else(|| DEFAULT_ACCOUNT_URL.to_string()))
}
