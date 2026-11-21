//! The Rust port of `apps/desktop/src/hostUsageRelay.ts`.
//!
//! Local fixed-day spool receipts: a day-scoped hash crosses the wire; the product UUID and
//! consent generation stay local.

#![forbid(unsafe_code)]

#[path = "../../hostusage-rs/src/port.rs"]
pub mod hostusage_port;

use crate::utf16;

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const POLICY_FILE: &str = "host-usage-policy.json";
const DIRECTORY: &str = "host-usage";
const RECEIPTS_FILE: &str = "host-usage-receipts.json";
const DAY_MS: f64 = 86_400_000.0;
const RECEIPT_DAYS: f64 = 14.0;
const MAX_READ_BYTES: u64 = 128 * 1024;

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(f64::NAN)
}

/// `String()` coercion for template literals.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_str).collect::<Vec<_>>().join(","),
        _ => "[object Object]".into(),
    }
}

fn uuid_v4() -> String {
    let mut b = [0u8; 16];
    getrandom_fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        b[6],
        b[7],
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

#[cfg(unix)]
fn getrandom_fill(buf: &mut [u8]) {
    use std::io::Read;
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        let _ = f.read_exact(buf);
    }
}

#[cfg(not(unix))]
fn getrandom_fill(buf: &mut [u8]) {
    let mut x = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        .wrapping_add(std::process::id() as u64);
    for byte in buf.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *byte = x as u8;
    }
}

fn is_policy_id(s: &str) -> bool {
    s.len() == 36
        && s.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c) || c == '-')
}

fn is_spool_name(name: &str) -> bool {
    let Some(rest) = name.strip_suffix(".json") else {
        return false;
    };
    if rest.len() != 46 {
        return false;
    }
    let (id, date) = rest.split_at(36);
    is_policy_id(id)
        && date.len() == 10
        && date.as_bytes()[0] == b'-'
        && date.as_bytes()[5] == b'-'
        && date.as_bytes()[8] == b'-'
        && date[1..5].chars().all(|c| c.is_ascii_digit())
        && date[6..8].chars().all(|c| c.is_ascii_digit())
        && date[9..10].chars().all(|c| c.is_ascii_digit())
}

fn day_prefix(s: &str) -> String {
    utf16::slice(&utf16::units(s), 0, 10)
}

/// Port of class `HostUsageRelay` (`apps/desktop/src/hostUsageRelay.ts:24`).
pub struct HostUsageRelay {
    root: PathBuf,
    channel: String,
    policy: Option<Map<String, Value>>,
}

impl HostUsageRelay {
    /// Port of the `HostUsageRelay` constructor (`apps/desktop/src/hostUsageRelay.ts:24`).
    pub fn new(home_dir: String, channel: String) -> Result<Self, String> {
        Ok(Self {
            root: Path::new(&home_dir).join("product-analytics"),
            channel,
            policy: None,
        })
    }

    fn read(&self, name: &str) -> Option<Map<String, Value>> {
        let path = self.root.join(name);
        if fs::metadata(&path).ok()?.len() > MAX_READ_BYTES {
            return None;
        }
        let text = fs::read_to_string(&path).ok()?;
        match serde_json::from_str::<Value>(&text).ok()? {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }

    fn write(&self, name: &str, value: &Value) -> Result<(), String> {
        fs::create_dir_all(&self.root).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700));
        }
        let target = self.root.join(name);
        let staging = self
            .root
            .join(format!("{}.tmp-{}", name, std::process::id()));
        let result = (|| -> Result<(), String> {
            let data = serde_json::to_string(value).map_err(|e| e.to_string())?;
            #[cfg(unix)]
            {
                use std::io::Write;
                use std::os::unix::fs::OpenOptionsExt;
                fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .mode(0o600)
                    .open(&staging)
                    .and_then(|mut f| f.write_all(data.as_bytes()))
                    .map_err(|e| e.to_string())?;
            }
            #[cfg(not(unix))]
            fs::write(&staging, data).map_err(|e| e.to_string())?;
            fs::rename(&staging, &target).map_err(|e| e.to_string())?;
            Ok(())
        })();
        let _ = fs::remove_file(&staging);
        result
    }

    /// Restarts preserve the cutoff; a new opt-in begins a new consent generation.
    pub fn enable(&mut self) -> Result<(), String> {
        let saved = self.read(POLICY_FILE);
        let keep = saved.as_ref().is_some_and(|s| {
            s.get("enabled") == Some(&Value::Bool(true))
                && s.get("policyId")
                    .and_then(Value::as_str)
                    .is_some_and(is_policy_id)
                && s.get("since")
                    .and_then(Value::as_str)
                    .and_then(hostusage_port::parse_iso_ms)
                    .is_some_and(|t| t <= now_ms())
        });
        self.policy = Some(if keep {
            saved.unwrap_or_default()
        } else {
            let mut p = Map::new();
            p.insert("enabled".into(), Value::Bool(true));
            p.insert("policyId".into(), Value::String(uuid_v4()));
            p.insert(
                "since".into(),
                Value::String(hostusage_port::ms_to_iso(now_ms()).unwrap_or_default()),
            );
            p
        });
        let policy = Value::Object(self.policy.clone().unwrap_or_default());
        self.write(POLICY_FILE, &policy)
    }

    /// Consent/identity reset invalidates both frozen snapshots and local delivery receipts.
    pub fn clear(&mut self) -> Result<(), String> {
        self.policy = None;
        for name in [POLICY_FILE, RECEIPTS_FILE, DIRECTORY] {
            let path = self.root.join(name);
            if name == DIRECTORY {
                if path.exists() {
                    fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
                }
            } else {
                let _ = fs::remove_file(&path);
            }
        }
        Ok(())
    }

    /// Reconstruct a pending window with a day-scoped hash, never an account or installation key.
    pub fn peek(&mut self, install_id: String) -> Result<Option<serde_json::Value>, String> {
        let Some(policy) = self.policy.clone() else {
            return Ok(None);
        };
        let policy_id = policy
            .get("policyId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let receipts = self.read(RECEIPTS_FILE);
        let delivered: HashSet<Value> = match receipts.as_ref() {
            Some(r)
                if r.get("policyId") == Some(&Value::String(policy_id.clone()))
                    && r.get("days").is_some_and(Value::is_array) =>
            {
                r.get("days")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect()
            }
            _ => HashSet::new(),
        };
        let dir = self.root.join(DIRECTORY);
        let mut files: Vec<String> = match fs::read_dir(&dir) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect(),
            Err(_) => return Ok(None),
        };
        files.sort();
        for name in files {
            if !is_spool_name(&name) {
                continue;
            }
            let rel = format!("{}/{}", DIRECTORY, name);
            let Some(raw) = self.read(&rel) else {
                continue;
            };
            if raw.get("policyId") != Some(&Value::String(policy_id.clone())) {
                continue;
            }
            let Some(window_start) = raw.get("windowStart").and_then(Value::as_str) else {
                continue;
            };
            if delivered.contains(&Value::String(day_prefix(window_start))) {
                continue;
            }
            let mut hasher = Sha256::new();
            hasher.update(format!(
                "{}\n{}\n{}",
                install_id,
                window_start,
                js_str(raw.get("windowEnd").unwrap_or(&Value::Null))
            ));
            let window_id = hasher
                .finalize()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            let mut merged = raw.clone();
            merged.insert("windowId".into(), Value::String(window_id));
            merged.insert("channel".into(), Value::String(self.channel.clone()));
            let Some(window) =
                hostusage_port::normalize_host_usage_window(Value::Object(merged), None)?
            else {
                continue;
            };
            let covered = window
                .get("coveredFrom")
                .and_then(Value::as_str)
                .and_then(hostusage_port::parse_iso_ms)
                .unwrap_or(f64::NAN);
            let expected = hostusage_port::parse_iso_ms(window_start)
                .unwrap_or(f64::NAN)
                .max(
                    hostusage_port::parse_iso_ms(
                        policy.get("since").and_then(Value::as_str).unwrap_or(""),
                    )
                    .unwrap_or(f64::NAN),
                );
            if covered == expected {
                return Ok(Some(window));
            }
        }
        Ok(None)
    }

    /// Persist the receipt before removing the spool, so retry/restart cannot change its window.
    pub fn acknowledge(&mut self, window: serde_json::Value) -> Result<(), String> {
        let Some(policy) = self.policy.clone() else {
            return Ok(());
        };
        let policy_id = policy
            .get("policyId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let Some(window_start) = window.get("windowStart").and_then(Value::as_str) else {
            return Err("Cannot read properties of undefined (reading 'slice')".to_string());
        };
        let day = day_prefix(window_start);
        let raw = self.read(RECEIPTS_FILE);
        let days: Vec<String> = match raw.as_ref() {
            Some(r)
                if r.get("policyId") == Some(&Value::String(policy_id.clone()))
                    && r.get("days").is_some_and(Value::is_array) =>
            {
                r.get("days")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        };
        let cutoff = now_ms() - RECEIPT_DAYS * DAY_MS;
        let mut seen = HashSet::new();
        let mut kept: Vec<String> = days
            .into_iter()
            .chain(std::iter::once(day.clone()))
            .filter(|d| seen.insert(d.clone()))
            .filter(|d| {
                hostusage_port::parse_iso_ms(&format!("{}T00:00:00.000Z", d))
                    .map(|t| t + DAY_MS >= cutoff)
                    .unwrap_or(false)
            })
            .collect();
        kept.sort();
        self.write(
            RECEIPTS_FILE,
            &json!({ "policyId": policy_id, "days": kept }),
        )?;
        let spool = self
            .root
            .join(DIRECTORY)
            .join(format!("{}-{}.json", policy_id, day));
        let _ = fs::remove_file(spool);
        Ok(())
    }
}
