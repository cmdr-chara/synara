//! The Rust port of `packages/shared/src/platformEnvironment.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const DEFAULT_SYSTEM_ROOT: &str = "C:\\Windows";

fn env_prop<'a>(env: &'a Value, key: &str) -> Result<&'a Value, String> {
    if env.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if crate::js::is_undefined(env) {
        return Ok(&Value::Null);
    }
    Ok(env.get(key).unwrap_or(&Value::Null))
}

fn system_root(env: &Value) -> Result<String, String> {
    let root = env_prop(env, "SystemRoot")?;
    if !root.is_null()
        && !crate::js::is_undefined(root)
        && let Some(s) = root.as_str()
    {
        return Ok(s.to_string());
    }
    let alt = env_prop(env, "SYSTEMROOT")?;
    if let Some(s) = alt.as_str() {
        return Ok(s.to_string());
    }
    Ok(DEFAULT_SYSTEM_ROOT.to_string())
}

/// Port of `resolveWindowsSystemRoot` (`packages/shared/src/platformEnvironment.ts`).
pub fn resolve_windows_system_root(env: serde_json::Value) -> Result<String, String> {
    system_root(&env)
}

/// Port of `resolveWindowsSystemExecutable` (`packages/shared/src/platformEnvironment.ts`).
pub fn resolve_windows_system_executable(
    relative_segments: Vec<String>,
    env: serde_json::Value,
) -> Result<String, String> {
    let mut out = system_root(&env)?;
    for seg in &relative_segments {
        for part in seg.split('/') {
            if !part.is_empty() && part != "." {
                out.push('\\');
                out.push_str(part);
            }
        }
    }
    Ok(out)
}

/// Port of `resolveWindowsComSpec` (`packages/shared/src/platformEnvironment.ts`).
pub fn resolve_windows_com_spec(env: serde_json::Value) -> Result<String, String> {
    let com_spec = env_prop(&env, "ComSpec")?;
    if !crate::js::is_undefined(com_spec)
        && let Some(s) = com_spec.as_str()
    {
        return Ok(s.to_string());
    }
    Ok(format!("{}\\System32\\cmd.exe", system_root(&env)?))
}

/// Port of `resolveWindowsWslExecutable` (`packages/shared/src/platformEnvironment.ts`).
pub fn resolve_windows_wsl_executable(env: serde_json::Value) -> Result<String, String> {
    Ok(format!("{}\\System32\\wsl.exe", system_root(&env)?))
}

/// Port of `resolveWindowsPowerShellExecutable` (`packages/shared/src/platformEnvironment.ts`).
pub fn resolve_windows_power_shell_executable(env: serde_json::Value) -> Result<String, String> {
    Ok(format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        system_root(&env)?
    ))
}
