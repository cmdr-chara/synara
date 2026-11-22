//! The Rust port of `apps/server/src/environment/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};

const MACHINE_ID_SALT: &str = "synara-machine-id-v1:";
const READ_TIMEOUT_MS: u64 = 3_000;

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

fn is_js_regex_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

fn skip_js_spaces(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && is_js_regex_space(chars[i]) {
        i += 1;
    }
    i
}

fn is_machine_id_char(c: char) -> bool {
    c.is_ascii_hexdigit() || c == '-'
}

fn read_id36(chars: &[char], i: usize) -> Option<String> {
    if i + 36 > chars.len() {
        return None;
    }
    if chars[i..i + 36].iter().all(|c| is_machine_id_char(*c)) {
        Some(chars[i..i + 36].iter().collect())
    } else {
        None
    }
}

/// Port of `hashMachineId` (`apps/server/src/environment/machineId.ts:14`).
pub fn hash_machine_id(raw: String) -> Result<Option<String>, String> {
    let normalized = js_trim(&raw).to_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }
    let digest = Sha256::digest(format!("{MACHINE_ID_SALT}{normalized}"));
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    Ok(Some(hex.chars().take(32).collect()))
}

/// Port of `parseIoregPlatformUuid` (`apps/server/src/environment/machineId.ts:24`).
pub fn parse_ioreg_platform_uuid(output: String) -> Result<Option<String>, String> {
    let chars: Vec<char> = output.chars().collect();
    const LIT: &str = "\"IOPlatformUUID\"";
    let lit: Vec<char> = LIT.chars().collect();
    if chars.len() < lit.len() {
        return Ok(None);
    }
    for start in 0..=chars.len() - lit.len() {
        if chars[start..start + lit.len()] != lit[..] {
            continue;
        }
        let mut i = skip_js_spaces(&chars, start + lit.len());
        if i >= chars.len() || chars[i] != '=' {
            continue;
        }
        i = skip_js_spaces(&chars, i + 1);
        if i >= chars.len() || chars[i] != '"' {
            continue;
        }
        if let Some(id) = read_id36(&chars, i + 1)
            && chars.get(i + 37) == Some(&'"')
        {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Port of `parseWindowsMachineGuid` (`apps/server/src/environment/machineId.ts:29`).
pub fn parse_windows_machine_guid(output: String) -> Result<Option<String>, String> {
    let chars: Vec<char> = output.chars().collect();
    const LIT: &str = "MachineGuid";
    const KEY: &str = "REG_SZ";
    let lit: Vec<char> = LIT.chars().collect();
    let key: Vec<char> = KEY.chars().collect();
    if chars.len() < lit.len() {
        return Ok(None);
    }
    for start in 0..=chars.len() - lit.len() {
        if chars[start..start + lit.len()] != lit[..] {
            continue;
        }
        let mut i = start + lit.len();
        let s0 = i;
        i = skip_js_spaces(&chars, i);
        if i == s0 {
            continue;
        }
        if i + key.len() > chars.len() || chars[i..i + key.len()] != key[..] {
            continue;
        }
        i += key.len();
        let s1 = i;
        i = skip_js_spaces(&chars, i);
        if i == s1 {
            continue;
        }
        if let Some(id) = read_id36(&chars, i) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

fn run_command(command: &str, args: &[&str]) -> Result<String, String> {
    use std::time::{Duration, Instant};
    let mut child = std::process::Command::new(command)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_millis(READ_TIMEOUT_MS);
    loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) if status.success() => {
                let mut out = String::new();
                use std::io::Read as _;
                child
                    .stdout
                    .take()
                    .ok_or("stdout")?
                    .read_to_string(&mut out)
                    .map_err(|e| e.to_string())?;
                return Ok(out);
            }
            Some(_) => return Err("non-zero exit".to_string()),
            None if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("timeout".to_string());
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn read_raw_machine_id() -> Result<Option<String>, String> {
    if cfg!(target_os = "macos") {
        return parse_ioreg_platform_uuid(run_command(
            "/usr/sbin/ioreg",
            &["-rd1", "-c", "IOPlatformExpertDevice"],
        )?);
    }
    if cfg!(target_os = "linux") {
        for path in ["/etc/machine-id", "/var/lib/dbus/machine-id"] {
            let value = js_trim(&std::fs::read_to_string(path).unwrap_or_default()).to_string();
            if !value.is_empty() {
                return Ok(Some(value));
            }
        }
        return Ok(None);
    }
    if cfg!(target_os = "windows") {
        return parse_windows_machine_guid(run_command(
            "reg",
            &[
                "query",
                "HKLM\\SOFTWARE\\Microsoft\\Cryptography",
                "/v",
                "MachineGuid",
            ],
        )?);
    }
    Ok(None)
}

/// Port of `readMachineId` (`apps/server/src/environment/machineId.ts:64`).
pub async fn read_machine_id() -> Result<Option<String>, String> {
    match read_raw_machine_id() {
        Ok(Some(raw)) => hash_machine_id(raw),
        _ => Ok(None),
    }
}
