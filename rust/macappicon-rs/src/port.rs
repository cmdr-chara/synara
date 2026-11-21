//! The Rust port of `apps/desktop/src/macAppIcon.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::cell::RefCell;

// NSWorkspace writes custom-icon metadata outside the signed Contents tree.
// Passing nil restores the bundle's appearance-aware icon. Arguments are data,
// never interpolated into this script.
const SET_FILE_ICON_SCRIPT: &str = "\nObjC.import('AppKit');\nfunction run(argv) {\n  // $() is Objective-C nil; JavaScript null would bridge to NSNull.\n  var image = argv[1] === '' ? $() : $.NSImage.alloc.initWithContentsOfFile(argv[1]);\n  if (argv[1] !== '' && (!image || !image.isValid)) {\n    throw new Error('Unable to load the app icon');\n  }\n  if (!$.NSWorkspace.sharedWorkspace.setIconForFileOptions(image, argv[0], 0)) {\n    throw new Error('Unable to update the app icon; the app must be writable');\n  }\n}\n";

#[derive(Clone)]
struct BundleSignature {
    size: f64,
    mtime_ms: f64,
    inode: f64,
}

#[derive(Clone)]
struct LastPersisted {
    bundle_path: String,
    digest: Option<String>,
    signature: BundleSignature,
}

thread_local! {
    static LAST_PERSISTED: RefCell<Option<LastPersisted>> = const { RefCell::new(None) };
}

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.get("$undefined") == Some(&Value::Bool(true)))
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
}

/// `Crypto.createHash("sha256").update(png)` — strings and Buffer-likes only.
fn digest_of(png: &Value) -> Result<Option<String>, String> {
    const MSG: &str = "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView.";
    if png.is_null() {
        return Ok(None);
    }
    match png {
        Value::String(s) => Ok(Some(sha256_hex(s.as_bytes()))),
        Value::Number(n) => Err(format!(
            "{MSG} Received type number ({})",
            crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
        )),
        Value::Bool(b) => Err(format!("{MSG} Received type boolean ({b})")),
        Value::Array(_) => Err(format!("{MSG} Received an instance of Array")),
        Value::Object(_) if is_undefined(png) => Err(format!("{MSG} Received undefined")),
        Value::Object(_) => Err(format!("{MSG} Received an instance of Object")),
        Value::Null => unreachable!(),
    }
}

fn js_path_normalize(p: &str) -> String {
    let absolute = p.starts_with('/');
    let trailing = p.len() > 1 && p.ends_with('/');
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(out.last(), Some(&s) if s != "..") {
                    out.pop();
                } else if !absolute {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    let mut joined = out.join("/");
    if absolute {
        joined.insert(0, '/');
    }
    if joined.is_empty() {
        joined = ".".to_string();
    } else if trailing && !joined.ends_with('/') {
        joined.push('/');
    }
    joined
}

/// `readBundleSignature` — `FS.stat` inside `try`, so any stat failure is null.
fn read_bundle_signature(bundle_path: &str) -> Option<BundleSignature> {
    let md = std::fs::metadata(bundle_path).ok()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Some(BundleSignature {
            size: md.size() as f64,
            mtime_ms: md.mtime() as f64 * 1000.0 + md.mtime_nsec() as f64 / 1e6,
            inode: md.ino() as f64,
        })
    }
    #[cfg(not(unix))]
    {
        let m = md
            .modified()
            .ok()?
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        Some(BundleSignature {
            size: md.len() as f64,
            mtime_ms: m.as_secs() as f64 * 1000.0 + m.subsec_nanos() as f64 / 1e6,
            inode: 0.0,
        })
    }
}

/// `isBundleStable(baseline, current)`: current readable and identical identity.
fn is_bundle_stable(baseline: &BundleSignature, current: &Option<BundleSignature>) -> bool {
    let Some(c) = current else {
        return false;
    };
    c.size == baseline.size && c.mtime_ms == baseline.mtime_ms && c.inode == baseline.inode
}

/// Port of `persistMacAppIcon` (`apps/desktop/src/macAppIcon.ts:48`).
pub async fn persist_mac_app_icon(input: serde_json::Value) -> Result<(), String> {
    let png = member(&input, "png")?;
    let digest = digest_of(&png)?;
    let bundle_path = member(&input, "bundlePath")?;
    let cache_dir = member(&input, "cacheDirectory")?;
    let bundle_path_s = bundle_path.as_str().unwrap_or("");
    let cache_dir_s = cache_dir.as_str().unwrap_or("");

    let signature = read_bundle_signature(bundle_path_s);
    let skip = LAST_PERSISTED.with(|l| {
        let last = l.borrow();
        matches!(
            last.as_ref(),
            Some(p) if p.bundle_path == bundle_path_s
                && p.digest == digest
                && is_bundle_stable(&p.signature, &signature)
        )
    });
    if skip {
        return Ok(());
    }

    let mut image_path = String::new();
    if let Value::String(s) = &png {
        // Path.join drops empty segments before normalizing.
        let file = format!("{}.png", digest.as_deref().unwrap_or(""));
        let parts: Vec<&str> = [cache_dir_s, file.as_str()]
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect();
        image_path = js_path_normalize(&parts.join("/"));
        std::fs::create_dir_all(cache_dir_s).map_err(|e| e.to_string())?;
        std::fs::write(&image_path, s.as_bytes()).map_err(|e| e.to_string())?;
    }

    let out = std::process::Command::new("/usr/bin/osascript")
        .args([
            "-l",
            "JavaScript",
            "-e",
            SET_FILE_ICON_SCRIPT,
            bundle_path_s,
            &image_path,
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(format!(
            "Command failed: /usr/bin/osascript -l JavaScript -e {} {} {}\n{}",
            SET_FILE_ICON_SCRIPT,
            bundle_path_s,
            image_path,
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let persisted = read_bundle_signature(bundle_path_s);
    LAST_PERSISTED.with(|l| {
        *l.borrow_mut() = match (&signature, &persisted) {
            (Some(before), Some(after)) if before.inode == after.inode => Some(LastPersisted {
                bundle_path: bundle_path_s.to_string(),
                digest,
                signature: after.clone(),
            }),
            _ => None,
        };
    });
    Ok(())
}
