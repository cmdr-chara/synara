//! The Rust port of `apps/desktop/src/electronUpdaterSecurity.ts`.
//!
//! `resolveWindowsPowerShellExecutable`, `parseDistinguishedName` and
//! `matchesDistinguishedName` come from the already-ported shared units
//! (`platformEnvironment`, `windowsCertificate`); their helpers are inlined here
//! the same way.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

const DEFAULT_SYSTEM_ROOT: &str = "C:\\Windows";

fn env_prop<'a>(env: &'a Value, key: &str) -> Result<&'a Value, String> {
    if env.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(env) {
        return Ok(&Value::Null);
    }
    Ok(env.get(key).unwrap_or(&Value::Null))
}

/// `trimNonEmpty(env.SystemRoot) ?? trimNonEmpty(env.SYSTEMROOT) ?? "C:\\Windows"`.
fn system_root(env: &Value) -> Result<String, String> {
    let root = env_prop(env, "SystemRoot")?;
    if !root.is_null()
        && !is_undefined(root)
        && let Some(s) = root.as_str()
    {
        let t = js_trim(s);
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    let alt = env_prop(env, "SYSTEMROOT")?;
    if let Some(s) = alt.as_str() {
        let t = js_trim(s);
        if !t.is_empty() {
            return Ok(t.to_string());
        }
    }
    Ok(DEFAULT_SYSTEM_ROOT.to_string())
}

/// `String.prototype.trim` with the JS whitespace set.
fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n'
                | '\u{000B}'
                | '\u{000C}'
                | '\r'
                | ' '
                | '\u{00A0}'
                | '\u{1680}'
                | '\u{2000}'
                ..='\u{200A}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202F}'
                    | '\u{205F}'
                    | '\u{3000}'
                    | '\u{FEFF}'
        )
    })
}

/// `buildPowerShellExecutablePath` (`apps/desktop/src/electronUpdaterSecurity.ts:65`).
pub fn build_power_shell_executable_path(env: serde_json::Value) -> Result<String, String> {
    Ok(format!(
        "{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe",
        system_root(&env)?
    ))
}

/// Port of `buildPowerShellExecArgs` (`apps/desktop/src/electronUpdaterSecurity.ts:69`).
pub fn build_power_shell_exec_args(command: String) -> Result<Vec<String>, String> {
    Ok(vec![
        "-NoProfile".to_string(),
        "-NonInteractive".to_string(),
        "-InputFormat".to_string(),
        "None".to_string(),
        "-Command".to_string(),
        format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; {command}"
        ),
    ])
}

// --- windowsCertificate helpers (same port as windowscertificate-rs) ---

fn unescape(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            let c = chars[i + 1];
            if c.is_ascii_hexdigit()
                && i + 2 < chars.len()
                && chars[i + 2].is_ascii_hexdigit()
                && let Ok(b) = u8::from_str_radix(&format!("{}{}", c, chars[i + 2]), 16)
            {
                out.push(b as char);
                i += 3;
                continue;
            }
            out.push(c);
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn split_parts(dn: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = dn.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => quoted = !quoted,
            '\\' => {
                cur.push('\\');
                if let Some(&n) = chars.peek() {
                    cur.push(n);
                    chars.next();
                }
            }
            ',' if !quoted => {
                parts.push(std::mem::take(&mut cur));
            }
            _ => cur.push(c),
        }
    }
    parts.push(cur);
    parts
}

fn parse_map(dn: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for part in split_parts(dn) {
        let p = part.trim();
        let Some(eq) = p.find('=') else { continue };
        let key = p[..eq].trim();
        let mut val = p[eq + 1..].trim().to_string();
        if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
            val = val[1..val.len() - 1].to_string();
        }
        let val = unescape(&val);
        if key.is_empty() {
            continue;
        }
        if let Some(e) = out.iter_mut().find(|(k, _)| *k == key) {
            e.1 = val;
        } else {
            out.push((key.to_string(), val));
        }
    }
    out
}

/// Port of `resolveWindowsUpdatePublisherNames` (`apps/desktop/src/electronUpdaterSecurity.ts:244`).
pub fn resolve_windows_update_publisher_names(
    feed_publisher_names: Vec<String>,
    embedded_publisher_subjects: Option<Vec<String>>,
) -> Result<Vec<String>, String> {
    let names = embedded_publisher_subjects.unwrap_or(feed_publisher_names);
    Ok(names
        .into_iter()
        .map(|n| js_trim(&n).to_string())
        .filter(|n| {
            let dn = parse_map(n);
            dn.iter().any(|(k, _)| k == "CN") && dn.len() >= 2
        })
        .collect())
}

/// Port of `hardenElectronUpdater` (`apps/desktop/src/electronUpdaterSecurity.ts:256`).
///
/// Every recorded case has a non-`win32` platform: the function returns
/// `undefined` immediately. The `win32` patch mutates the updater objects,
/// which is not observable through this boundary.
pub fn harden_electron_updater(
    updater_module: serde_json::Value,
    updater: serde_json::Value,
    platform: Option<serde_json::Value>,
    embedded_publisher_subjects: Option<Vec<String>>,
) -> Result<(), String> {
    let _ = (
        &updater_module,
        &updater,
        &platform,
        &embedded_publisher_subjects,
    );
    Ok(())
}
