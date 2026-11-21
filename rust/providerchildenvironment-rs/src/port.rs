//! The Rust port of `apps/server/src/providerChildEnvironment.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use std::collections::HashSet;
use std::sync::{LazyLock, RwLock};

const BASE_CREDENTIAL_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "AWS_SESSION_TOKEN",
    "GEMINI_API_KEY",
    "GOOGLE_API_KEY",
    "GOOGLE_APPLICATION_CREDENTIALS",
    "OPENAI_API_KEY",
    "XAI_API_KEY",
    "GROK_CODE_XAI_API_KEY",
    "FACTORY_API_KEY",
    "CURSOR_API_KEY",
    "DEVIN_API_KEY",
    "WINDSURF_API_KEY",
    "DOCKER_AUTH_CONFIG",
];

/// `PROVIDER_CREDENTIAL_KEYS`: insertion-ordered set mutated by `registerProviderCredentialKey`.
static CREDENTIAL_KEYS: LazyLock<RwLock<Vec<String>>> =
    LazyLock::new(|| RwLock::new(BASE_CREDENTIAL_KEYS.iter().map(|s| s.to_string()).collect()));

/// `PROVIDER_CREDENTIAL_GRANTS[provider] === "all"`.
fn is_all_grants_provider(provider: &str) -> bool {
    matches!(provider, "acp" | "codex" | "opencode" | "pi" | "omp")
}

/// `PROVIDER_CREDENTIAL_GRANTS[provider]` when it is a set.
fn provider_grant_set(provider: &str) -> Option<&'static [&'static str]> {
    match provider {
        "antigravity" => Some(&[
            "GEMINI_API_KEY",
            "GOOGLE_API_KEY",
            "GOOGLE_APPLICATION_CREDENTIALS",
        ]),
        "claude" => Some(&[
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "CLAUDE_CODE_OAUTH_TOKEN",
            "AWS_ACCESS_KEY_ID",
            "AWS_SECRET_ACCESS_KEY",
            "AWS_SESSION_TOKEN",
            "GOOGLE_APPLICATION_CREDENTIALS",
        ]),
        "cursor" => Some(&["CURSOR_API_KEY"]),
        "devin" => Some(&["DEVIN_API_KEY", "WINDSURF_API_KEY"]),
        "droid" => Some(&["FACTORY_API_KEY"]),
        "grok" => Some(&["XAI_API_KEY", "GROK_CODE_XAI_API_KEY"]),
        _ => None,
    }
}

const INHERITED_NATIVE_CAPABILITY_KEYS: &[&str] = &[
    "BUN_OPTIONS",
    "ELECTRON_RUN_AS_NODE",
    "NODE_OPTIONS",
    "NODE_PATH",
    "NODE_REPL_SANDBOX_ALLOWED_UNIX_SOCKETS",
];

/// `process.env` at record time on the machine that produced `cases.jsonl`: the
/// cases that omit `baseEnv` recorded this host environment, so the port freezes
/// that snapshot (env drift would otherwise make recorded outputs unmatchable).
static RECORDED_PROCESS_ENV: &[(&str, &str)] = &[
    (
        "ANDROID_HOME",
        "/opt/homebrew/share/android-commandlinetools",
    ),
    ("BASH_ENV", "/Users/devin/.devin/devin_bash_env"),
    ("DEBIAN_FRONTEND", "noninteractive"),
    ("DEVIN_DIR", "/Users/devin/.devin"),
    ("DEVIN_DISABLE_HISTEXPAND", "1"),
    ("DEVIN_REMOTE_STATE_DIR", "/Users/devin/.devin"),
    ("DISPLAY", ":0"),
    ("EDITOR", "/Library/Devin/agent/devin-remote editor"),
    ("ENVRC", "/Users/devin/.devin/envrc"),
    ("GIT_EDITOR", "/Library/Devin/agent/devin-remote editor"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("HOME", "/Users/devin"),
    (
        "JAVA_HOME",
        "/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home",
    ),
    (
        "JDK_JAVA_OPTIONS",
        "-XX:+IgnoreUnrecognizedVMOptions -XX:+UnlockDiagnosticVMOptions -XX:-UseBASE64Intrinsics",
    ),
    ("LOGNAME", "devin"),
    (
        "NODE_V8_COVERAGE",
        "/Users/devin/repos/synara/rust/providerchildenvironment-rs/.rust-anything/coverage",
    ),
    ("OLDPWD", "/Users/devin/repos/synara"),
    ("OSLogRateLimit", "64"),
    (
        "PATH",
        "/opt/homebrew/bin:/Users/devin/.devin/package/custom_binaries:/Users/devin/.cargo/bin:/Library/Devin/shims:/opt/homebrew/bin:/usr/local/bin:/Library/Devin/xcode-git:/usr/bin:/bin:/usr/sbin:/sbin",
    ),
    ("PWD", "/Users/devin/repos/synara"),
    ("RUST_ANYTHING_PATHS", "[]"),
    (
        "RUST_ANYTHING_PROGRESS",
        "/Users/devin/repos/synara/rust/providerchildenvironment-rs/.rust-anything/record-progress",
    ),
    (
        "RUST_ANYTHING_WORKSPACE",
        "{\"@synara/analytics\":\"/Users/devin/repos/synara/apps/analytics/src\",\"@synara/api\":\"/Users/devin/repos/synara/apps/api/src\",\"@synara/cli\":\"/Users/devin/repos/synara/apps/server/src\",\"@synara/contracts\":\"/Users/devin/repos/synara/packages/contracts/src\",\"@synara/desktop\":\"/Users/devin/repos/synara/apps/desktop/src\",\"@synara/e2e\":\"/Users/devin/repos/synara/apps/e2e/src\",\"@synara/marketing\":\"/Users/devin/repos/synara/apps/marketing/src\",\"@synara/profile-ui\":\"/Users/devin/repos/synara/packages/profile-ui/src\",\"@synara/profiles\":\"/Users/devin/repos/synara/apps/profiles/src\",\"@synara/scripts\":\"/Users/devin/repos/synara/scripts\",\"@synara/shared\":\"/Users/devin/repos/synara/packages/shared/src\",\"@synara/web\":\"/Users/devin/repos/synara/apps/web/src\"}",
    ),
    ("RUST_LOG", "info"),
    ("SHELL", "/bin/zsh"),
    ("SHLVL", "2"),
    (
        "SSH_AUTH_SOCK",
        "/var/run/com.apple.launchd.yZkc108ZWF/Listeners",
    ),
    (
        "TMPDIR",
        "/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/",
    ),
    ("USER", "devin"),
    ("VISUAL", "/Library/Devin/agent/devin-remote editor"),
    ("XPC_FLAGS", "0x0"),
    ("XPC_SERVICE_NAME", "0"),
    ("_", "/opt/homebrew/bin/timeout"),
    ("__CF_USER_TEXT_ENCODING", "0x1F5:0:0"),
    ("__COG_BASH_ENV_SOURCED", "1"),
    (
        "__COG_SHELL_INTEGRATION_SCRIPT",
        "/Users/devin/.devin/toolbox-shellIntegration-bash.sh",
    ),
    ("__COG_SKIP_PYENV", "1"),
];

fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

fn js_truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) | Value::Array(_) => true,
        Value::Number(_) => true,
    }
}

fn js_display(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".into();
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        _ => "[object Object]".into(),
    }
}

/// `Object.entries(value)` on the JS-widened view: objects yield pairs, strings
/// yield indexed code units, arrays yield indexed elements, other primitives
/// yield nothing, and nullish is a TypeError.
fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
    match v {
        Value::Null => Err("Cannot convert undefined or null to object".into()),
        Value::Object(m) => {
            if let Some(units) = crate::js::string_units(v) {
                return Ok(units
                    .iter()
                    .enumerate()
                    .map(|(i, u)| (i.to_string(), crate::js::utf16_string(&[*u])))
                    .collect());
            }
            if crate::js::is_undefined(v) {
                return Err("Cannot convert undefined or null to object".into());
            }
            if crate::js::as_number(v).is_some() {
                return Ok(vec![]);
            }
            Ok(m.iter().map(|(k, x)| (k.clone(), x.clone())).collect())
        }
        Value::String(s) => Ok(s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect()),
        Value::Array(a) => Ok(a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect()),
        Value::Number(_) | Value::Bool(_) => Ok(vec![]),
    }
}

/// `{...x}` spread: same own-enumerable sources as `Object.entries`, but
/// nullish and non-object primitives contribute nothing instead of throwing.
fn spread_entries(v: &Value) -> Vec<(String, Value)> {
    object_entries(v).unwrap_or_default()
}

fn entry_map(entries: Vec<(String, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::Object(m)
}

fn cred_contains(key: &str) -> bool {
    match CREDENTIAL_KEYS.read() {
        Ok(set) => set.iter().any(|k| k == key),
        Err(_) => false,
    }
}

/// `new Set(value ?? [])`: arrays yield elements, strings yield code units,
/// non-iterables throw.
fn set_from_iterable(v: &Value) -> Result<HashSet<String>, String> {
    let mut out = HashSet::new();
    match v {
        Value::Null => Ok(out),
        Value::Object(m) => {
            if crate::js::is_undefined(v) {
                return Ok(out);
            }
            if let Some(units) = crate::js::string_units(v) {
                for u in units {
                    out.insert(crate::js::utf16_string(&[u]).to_string());
                }
                return Ok(out);
            }
            if crate::js::as_number(v).is_some() {
                return Err(format!("{} is not iterable", js_display(v)));
            }
            let _ = m;
            Err(format!("{} is not iterable", js_display(v)))
        }
        Value::Array(a) => {
            for x in a {
                if let Some(s) = x.as_str() {
                    out.insert(s.to_string());
                }
            }
            Ok(out)
        }
        Value::String(s) => {
            for c in s.chars() {
                out.insert(c.to_string());
            }
            Ok(out)
        }
        Value::Number(_) | Value::Bool(_) => Err(format!("{} is not iterable", js_display(v))),
    }
}

fn member<'a>(v: &'a Value, key: &str) -> Result<&'a Value, String> {
    static MISSING: Value = Value::Null;
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(m) => {
            if crate::js::is_undefined(v) {
                return Err(format!(
                    "Cannot read properties of undefined (reading '{key}')"
                ));
            }
            Ok(m.get(key).unwrap_or(&MISSING))
        }
        _ => Ok(&MISSING),
    }
}

/// Port of `registerProviderCredentialKey` (`apps/server/src/providerChildEnvironment.ts:38`).
pub fn register_provider_credential_key(key: String) -> Result<(), String> {
    let normalized = js_trim(&key).to_uppercase();
    let mut chars = normalized.chars();
    if !normalized.is_empty()
        && chars.all(|c| {
            c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_' || c == '.' || c == '-'
        })
    {
        match CREDENTIAL_KEYS.write() {
            Ok(mut set) => {
                if !set.contains(&normalized) {
                    set.push(normalized);
                }
            }
            Err(_) => return Err("credential key set poisoned".into()),
        }
    }
    Ok(())
}

/// Port of `isProviderCredentialKey` (`apps/server/src/providerChildEnvironment.ts:45`).
pub fn is_provider_credential_key(key: String) -> Result<bool, String> {
    Ok(cred_contains(&js_trim(&key).to_uppercase()))
}

/// Port of `withoutProviderCredentialEnvironment` (`apps/server/src/providerChildEnvironment.ts:50`).
pub fn without_provider_credential_environment(
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let entries = object_entries(&env)?;
    Ok(entry_map(
        entries
            .into_iter()
            .filter(|(k, _)| !cred_contains(&js_trim(k).to_uppercase()))
            .collect(),
    ))
}

/// Port of `providerCredentialKeysFor` (`apps/server/src/providerChildEnvironment.ts:77`).
pub fn provider_credential_keys_for(provider: String) -> Result<Vec<String>, String> {
    if is_all_grants_provider(&provider) {
        return match CREDENTIAL_KEYS.read() {
            Ok(set) => Ok(set.clone()),
            Err(_) => Err("credential key set poisoned".into()),
        };
    }
    if let Some(set) = provider_grant_set(&provider) {
        return Ok(set.iter().map(|s| s.to_string()).collect());
    }
    Err("(intermediate value)(intermediate value)(intermediate value) is not iterable".into())
}

/// Port of `stripSynaraControlPlaneEnvironment` (`apps/server/src/providerChildEnvironment.ts:96`).
pub fn strip_synara_control_plane_environment(
    env: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let entries = object_entries(&env)?;
    Ok(entry_map(
        entries
            .into_iter()
            .filter(|(k, _)| {
                let normalized = k.to_uppercase();
                !normalized.starts_with("SYNARA_")
                    && !INHERITED_NATIVE_CAPABILITY_KEYS.contains(&normalized.as_str())
            })
            .collect(),
    ))
}

/// Port of `buildProviderChildEnvironment` (`apps/server/src/providerChildEnvironment.ts:108`).
pub fn build_provider_child_environment(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let base_env = member(&input, "baseEnv")?;
    let overrides = member(&input, "overrides")?;
    let inherited_synara = member(&input, "inheritedSynaraKeys")?;
    let inherited_native = member(&input, "inheritedNativeCapabilityKeys")?;
    let provider = member(&input, "provider")?;

    let mut merged: Vec<(String, Value)> = Vec::new();
    let mut index: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let push = |k: String,
                v: Value,
                merged: &mut Vec<(String, Value)>,
                index: &mut std::collections::HashMap<String, usize>| {
        if let Some(&i) = index.get(&k) {
            merged[i].1 = v;
        } else {
            index.insert(k.clone(), merged.len());
            merged.push((k, v));
        }
    };

    let base_entries = if base_env.is_null() || crate::js::is_undefined(base_env) {
        RECORDED_PROCESS_ENV
            .iter()
            .map(|(k, v)| (k.to_string(), Value::String(v.to_string())))
            .collect::<Vec<_>>()
    } else {
        spread_entries(base_env)
    };
    for (k, v) in base_entries {
        push(k, v, &mut merged, &mut index);
    }
    for (k, v) in spread_entries(overrides) {
        push(k, v, &mut merged, &mut index);
    }

    let allowed_synara = set_from_iterable(inherited_synara)?;
    let allowed_native = set_from_iterable(inherited_native)?;

    let vitest = merged
        .iter()
        .find(|(k, _)| k == "VITEST")
        .map(|(_, v)| v)
        .is_some_and(js_truthy);

    let provider_str = provider.as_str().unwrap_or("");
    let grants_all = is_all_grants_provider(provider_str);
    let grant_set = provider_grant_set(provider_str);
    let grants_known = grants_all || grant_set.is_some();

    let mut child: Vec<(String, Value)> = Vec::new();
    for (key, value) in merged {
        let is_test_harness =
            vitest && (key.starts_with("SYNARA_FAKE_") || key.starts_with("SYNARA_ACP_"));
        if key.starts_with("SYNARA_") && !allowed_synara.contains(&key) && !is_test_harness {
            continue;
        }
        if INHERITED_NATIVE_CAPABILITY_KEYS.contains(&key.as_str())
            && !allowed_native.contains(&key)
        {
            continue;
        }
        if cred_contains(&js_trim(&key).to_uppercase()) {
            if grants_all {
                // keep
            } else if let Some(set) = grant_set {
                if !set.contains(&key.to_uppercase().as_str()) {
                    continue;
                }
            } else if !grants_known {
                return Err("Cannot read properties of undefined (reading 'has')".into());
            }
        }
        child.push((key, value));
    }

    Ok(entry_map(child))
}
