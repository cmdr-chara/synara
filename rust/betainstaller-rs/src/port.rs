//! The Rust port of `apps/desktop/src/betaInstaller.ts`.

#![forbid(unsafe_code)]

use base64::Engine;
use serde_json::{Map, Value};
use sha2::{Digest, Sha512};

const BETA_MAC_MANIFEST_NAME: &str = "beta-mac.yml";
const BETA_MAC_APP_NAME: &str = "Synara Beta.app";
const SYNARA_BETA_BUNDLE_ID: &str = "com.emanueledipietro.synara.beta";

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

fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| {
                if x.is_null() || is_undefined(x) {
                    String::new()
                } else {
                    js_str(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if let Some(n) = crate::js::as_number(v) {
                return crate::js::number_to_string(n);
            }
            "[object Object]".into()
        }
    }
}

fn trim_js(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    })
}

fn unquote_yaml_scalar(value: &str) -> String {
    let t = trim_js(value);
    if t.len() >= 2 && t.starts_with('\'') && t.ends_with('\'') {
        return t[1..t.len() - 1].replace("''", "'");
    }
    if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') {
        return t[1..t.len() - 1].to_string();
    }
    t.to_string()
}

/// `^([A-Za-z0-9_-]+):\s*(.*)$`
fn yaml_pair(line: &str) -> Option<(String, String)> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-')
    {
        i += 1;
    }
    if i == 0 || bytes.get(i) != Some(&b':') {
        return None;
    }
    let key = &line[..i];
    let rest = &line[i + 1..];
    // `\s*` consumes all leading whitespace; `.*` keeps the rest verbatim.
    let rest = rest.trim_start_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    });
    Some((key.to_string(), rest.to_string()))
}

struct FileEntry {
    url: Option<String>,
    sha512: Option<String>,
    size: Option<f64>,
}

fn assign_file_field(file: &mut FileEntry, key: &str, raw: &str) {
    let value = unquote_yaml_scalar(raw);
    match key {
        "url" => file.url = Some(value),
        "sha512" => file.sha512 = Some(value),
        "size" => {
            let size = js_parse_number(&value);
            if size.is_finite() {
                file.size = Some(size);
            }
        }
        _ => {}
    }
}

/// Port of `parseBetaMacManifest` (`apps/desktop/src/betaInstaller.ts:75`).
pub fn parse_beta_mac_manifest(text: String) -> Result<serde_json::Value, String> {
    let mut version: Option<String> = None;
    let mut files: Vec<FileEntry> = Vec::new();
    let mut current: Option<usize> = None;
    let mut in_files = false;
    for raw_line in text.split('\n') {
        let trimmed = trim_js(raw_line);
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !raw_line.starts_with(' ') && !raw_line.starts_with('\t') && !raw_line.starts_with('-') {
            in_files = false;
            current = None;
            let Some((key, val)) = yaml_pair(trimmed) else {
                continue;
            };
            if key == "files" {
                in_files = true;
                continue;
            }
            if key == "version" {
                version = Some(unquote_yaml_scalar(&val));
            }
            continue;
        }
        if !in_files {
            continue;
        }
        if let Some(pair_src) = trimmed.strip_prefix("- ") {
            files.push(FileEntry {
                url: None,
                sha512: None,
                size: None,
            });
            let i = files.len() - 1;
            current = Some(i);
            if let Some((k, v)) = yaml_pair(pair_src) {
                assign_file_field(&mut files[i], &k, &v);
            }
            continue;
        }
        if let (Some((k, v)), Some(i)) = (yaml_pair(trimmed), current) {
            assign_file_field(&mut files[i], &k, &v);
        }
    }
    let valid: Vec<&FileEntry> = files
        .iter()
        .filter(|f| f.url.is_some() && f.sha512.is_some())
        .collect();
    if version.as_deref().is_none_or(|v| v.is_empty()) || valid.is_empty() {
        return Err(format!(
            "Could not parse {BETA_MAC_MANIFEST_NAME}: missing version or files."
        ));
    }
    let mut out_files = Vec::new();
    for f in valid {
        let mut m = Map::new();
        if let Some(u) = &f.url {
            m.insert("url".into(), Value::String(u.clone()));
        }
        if let Some(s) = &f.sha512 {
            m.insert("sha512".into(), Value::String(s.clone()));
        }
        if let Some(n) = f.size {
            m.insert("size".into(), crate::js::number(n));
        }
        out_files.push(Value::Object(m));
    }
    let mut out = Map::new();
    out.insert("version".into(), Value::String(version.unwrap_or_default()));
    out.insert("files".into(), Value::Array(out_files));
    Ok(Value::Object(out))
}

fn ends_with(hay: &str, needle: &str) -> bool {
    hay.ends_with(needle)
}

/// `/-(arm64|x64)[-.]/` — `"-arm64"` or `"-x64"` followed by `-` or `.`.
fn has_arch_suffix(url: &str) -> bool {
    for marker in ["-arm64", "-x64"] {
        let mut start = 0;
        while let Some(pos) = url[start..].find(marker) {
            let end = start + pos + marker.len();
            match url.as_bytes().get(end) {
                Some(b'-') | Some(b'.') => return true,
                _ => start = end,
            }
        }
    }
    false
}

/// `file.url.endsWith(x)` — TypeError text for non-strings.
fn url_string(file: &Value) -> Result<String, String> {
    let url = member(file, "url")?;
    match url.as_str() {
        Some(s) => Ok(s.to_string()),
        None => {
            if is_undefined(&url) {
                Err("Cannot read properties of undefined (reading 'endsWith')".to_string())
            } else {
                Err("file.url.endsWith is not a function".to_string())
            }
        }
    }
}

/// Port of `selectBetaDownloadFile` (`apps/desktop/src/betaInstaller.ts:128`).
pub fn select_beta_download_file(
    files: serde_json::Value,
    arch: String,
) -> Result<serde_json::Value, String> {
    if files.is_null() {
        return Err("Cannot read properties of null (reading 'filter')".to_string());
    }
    if is_undefined(&files) {
        return Err("Cannot read properties of undefined (reading 'filter')".to_string());
    }
    let Value::Array(items) = &files else {
        return Err("files.filter is not a function".to_string());
    };
    let mut zips: Vec<&Value> = Vec::new();
    for file in items {
        let url = url_string(file)?;
        if ends_with(&url, ".zip") {
            zips.push(file);
        }
    }
    let unmarked = if zips.len() == 1 { Some(zips[0]) } else { None };
    let needle_mid = format!("-{arch}-");
    let needle_end = format!("-{arch}.zip");
    let mut selected = zips.iter().copied().find(|file| {
        let url = file.get("url").and_then(Value::as_str).unwrap_or_default();
        url.contains(&needle_mid) || url.ends_with(&needle_end)
    });
    if selected.is_none()
        && let Some(u) = unmarked
    {
        let url = u.get("url").and_then(Value::as_str).unwrap_or_default();
        if !has_arch_suffix(url) {
            selected = Some(u);
        }
    }
    let Some(file) = selected else {
        return Err(format!(
            "No beta download for {arch} in {BETA_MAC_MANIFEST_NAME}."
        ));
    };
    Ok(file.clone())
}

/// `new URL(url)` validation, limited to what `feedGet` needs.
/// Returns (protocol-with-colon, hostname) or the Node "Invalid URL" error.
fn parse_feed_url(url: &Value) -> Result<(String, String), String> {
    let s = js_str(url);
    let bytes = s.as_bytes();
    let mut i = 0;
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return Err("Invalid URL".to_string());
    }
    while i < bytes.len()
        && (bytes[i].is_ascii_alphanumeric()
            || bytes[i] == b'+'
            || bytes[i] == b'-'
            || bytes[i] == b'.')
    {
        i += 1;
    }
    if bytes.get(i) != Some(&b':') {
        return Err("Invalid URL".to_string());
    }
    let protocol = format!("{}:", s[..i].to_ascii_lowercase());
    let rest = &s[i + 1..];
    let special = matches!(
        protocol.as_str(),
        "http:" | "https:" | "ws:" | "wss:" | "ftp:" | "file:"
    );
    let authority = if let Some(a) = rest.strip_prefix("//") {
        a
    } else if special {
        rest // special schemes tolerate a missing `//`
    } else {
        return Ok((protocol, String::new()));
    };
    let authority = authority.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority.rsplit('@').next().unwrap_or("");
    let hostname = if let Some(inner) = hostport.strip_prefix('[') {
        inner.split(']').next().unwrap_or("").to_string()
    } else {
        hostport.split(':').next().unwrap_or("").to_string()
    };
    if special && protocol != "file:" && hostname.is_empty() {
        return Err("Invalid URL".to_string());
    }
    Ok((protocol, hostname))
}

/// `Number(x)` on a scalar string: trimmed, `""` → 0, `Infinity`, radix
/// prefixes, otherwise a float or NaN.
fn js_parse_number(s: &str) -> f64 {
    let t = trim_js(s);
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    let lower = t.to_ascii_lowercase();
    for (prefix, radix) in [("0x", 16), ("0o", 8), ("0b", 2)] {
        if let Some(digits) = lower.strip_prefix(prefix) {
            return i64::from_str_radix(digits, radix)
                .map(|n| n as f64)
                .unwrap_or(f64::NAN);
        }
        for sign in ["+0x", "-0x", "+0o", "-0o", "+0b", "-0b"] {
            if lower.starts_with(sign) {
                let digits = &lower[3..];
                let n = i64::from_str_radix(digits, radix)
                    .map(|n| n as f64)
                    .unwrap_or(f64::NAN);
                return if sign.starts_with('-') { -n } else { n };
            }
        }
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn curl_text(url: &str, download_to: Option<&str>) -> Result<String, String> {
    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "-sS",
        "-L",
        "--max-redirs",
        "5",
        "--max-time",
        "30",
        "-H",
        "user-agent: synara-desktop",
        "-w",
        "\n%{http_code}",
    ]);
    if let Some(dest) = download_to {
        cmd.arg("-o").arg(dest);
    }
    let out = cmd.arg(url).output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    let (body, code) = text
        .rsplit_once('\n')
        .map(|(b, c)| (b.to_string(), c.trim().to_string()))
        .unwrap_or((text, "0".to_string()));
    if !out.status.success() || code != "200" {
        return Err(format!("GET {url} failed with {code}."));
    }
    Ok(body)
}

/// Port of `httpsFetchText` (`apps/desktop/src/betaInstaller.ts:181`).
pub fn https_fetch_text(url: serde_json::Value) -> Result<serde_json::Value, String> {
    let (protocol, hostname) = parse_feed_url(&url)?;
    let loopback = matches!(
        hostname.as_str(),
        "127.0.0.1" | "localhost" | "::1" | "[::1]"
    );
    if protocol == "http:" && !loopback {
        return Err(format!(
            "Refusing plain-HTTP beta feed for non-loopback host {hostname}."
        ));
    }
    if protocol != "http:" && protocol != "https:" {
        return Err(format!("Unsupported beta feed protocol {protocol}."));
    }
    Ok(Value::String(curl_text(&js_str(&url), None)?))
}

/// Port of `sha512FileBase64` (`apps/desktop/src/betaInstaller.ts:255`).
pub fn sha512_file_base64(path: String) -> Result<String, String> {
    let bytes = std::fs::read(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!("ENOENT: no such file or directory, open '{path}'")
        } else {
            e.to_string()
        }
    })?;
    let mut h = Sha512::new();
    h.update(&bytes);
    Ok(base64::engine::general_purpose::STANDARD.encode(h.finalize()))
}

/// Port of `httpsDownloadFile` (`apps/desktop/src/betaInstaller.ts:266`).
pub fn https_download_file(
    url: serde_json::Value,
    destination_path: serde_json::Value,
    on_progress: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = &on_progress;
    let (protocol, hostname) = parse_feed_url(&url)?;
    let loopback = matches!(
        hostname.as_str(),
        "127.0.0.1" | "localhost" | "::1" | "[::1]"
    );
    if protocol == "http:" && !loopback {
        return Err(format!(
            "Refusing plain-HTTP beta feed for non-loopback host {hostname}."
        ));
    }
    if protocol != "http:" && protocol != "https:" {
        return Err(format!("Unsupported beta feed protocol {protocol}."));
    }
    curl_text(&js_str(&url), Some(&js_str(&destination_path)))?;
    Ok(crate::js::undefined())
}

/// `Path.join` for this module's constant-shape joins (all non-empty strings).
fn path_join(parts: &[&str]) -> String {
    let segs: Vec<&str> = parts.iter().copied().filter(|p| !p.is_empty()).collect();
    let mut out: Vec<&str> = Vec::new();
    let joined_src = segs.join("/");
    let absolute = joined_src.starts_with('/');
    for seg in joined_src.split('/') {
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
        ".".into()
    } else {
        joined
    }
}

/// Port of `verifyBetaAppBundle` (`apps/desktop/src/betaInstaller.ts:379`).
pub fn verify_beta_app_bundle(app_path: String) -> Result<(), String> {
    let plist_path = path_join(&[&app_path, "Contents", "Info.plist"]);
    let plist = std::fs::read_to_string(&plist_path)
        .map_err(|_| format!("The download did not contain {BETA_MAC_APP_NAME}."))?;
    // /<key>CFBundleIdentifier<\/key>\s*<string>([^<]+)<\/string>/
    let needle = "<key>CFBundleIdentifier</key>";
    let bundle_id = plist.find(needle).and_then(|i| {
        let rest = &plist[i + needle.len()..];
        let rest = rest.trim_start_matches(|c: char| c.is_whitespace());
        let rest = rest.strip_prefix("<string>")?;
        let end = rest.find("</string>")?;
        Some(&rest[..end])
    });
    if bundle_id != Some(SYNARA_BETA_BUNDLE_ID) {
        return Err(format!(
            "Downloaded app is not Synara Beta (bundle id {}).",
            bundle_id.unwrap_or("missing")
        ));
    }
    Ok(())
}
