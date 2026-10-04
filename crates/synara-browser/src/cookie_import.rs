//! Explicit, bounded import of a user-selected Netscape/Mozilla cookie jar.
//!
//! The parsed bytes are normalized before they reach the native browser. Source
//! paths and cookie values are never logged or persisted by this module.

use std::{
    fs::{self, File, OpenOptions},
    io::Read,
    path::{Path, PathBuf},
};

const MAX_SOURCE_BYTES: u64 = 256 * 1024;
const MAX_COOKIES: usize = 256;
const MAX_DOMAIN_BYTES: usize = 253;
const MAX_PATH_BYTES: usize = 1024;
const MAX_NAME_BYTES: usize = 256;
const MAX_VALUE_BYTES: usize = 4096;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CookieImportError {
    #[error("cookie import source is unavailable")]
    Storage,
    #[error("cookie import source is invalid")]
    Invalid,
    #[error("cookie import exceeds protected limits")]
    Limit,
    #[error("cookie import contains no current cookies")]
    Empty,
}

/// Validated Netscape text ready for one temporary Authentication profile.
/// Values stay private and are intentionally not Debug/Serialize exposed.
pub struct ProtectedCookieJar {
    bytes: Vec<u8>,
    count: usize,
}

impl ProtectedCookieJar {
    pub fn count(&self) -> usize {
        self.count
    }

    #[cfg(all(feature = "native-webview", target_os = "linux"))]
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

impl Drop for ProtectedCookieJar {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

pub fn read_netscape_cookie_jar(
    path: &Path,
    now_unix_seconds: u64,
) -> Result<ProtectedCookieJar, CookieImportError> {
    let file = open_read_nofollow(path)?;
    let metadata = file.metadata().map_err(|_| CookieImportError::Storage)?;
    if !metadata.is_file() || metadata.len() > MAX_SOURCE_BYTES {
        return Err(CookieImportError::Limit);
    }
    let mut source = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_SOURCE_BYTES + 1)
        .read_to_end(&mut source)
        .map_err(|_| CookieImportError::Storage)?;
    if source.len() as u64 > MAX_SOURCE_BYTES {
        source.fill(0);
        return Err(CookieImportError::Limit);
    }
    let decoded = std::str::from_utf8(&source).map_err(|_| CookieImportError::Invalid);
    let result = decoded.and_then(|text| parse_netscape(text, now_unix_seconds));
    source.fill(0);
    result
}

fn parse_netscape(
    source: &str,
    now_unix_seconds: u64,
) -> Result<ProtectedCookieJar, CookieImportError> {
    let mut output =
        b"# Netscape HTTP Cookie File\n# Temporary Synara authentication import\n".to_vec();
    let mut count = 0usize;

    for raw in source.lines() {
        if raw.is_empty() || (raw.starts_with('#') && !raw.starts_with("#HttpOnly_")) {
            continue;
        }
        let mut fields = raw.split('\t');
        let raw_domain = fields.next().ok_or(CookieImportError::Invalid)?;
        let include_subdomains = fields.next().ok_or(CookieImportError::Invalid)?;
        let path = fields.next().ok_or(CookieImportError::Invalid)?;
        let secure = fields.next().ok_or(CookieImportError::Invalid)?;
        let expires = fields.next().ok_or(CookieImportError::Invalid)?;
        let name = fields.next().ok_or(CookieImportError::Invalid)?;
        let value = fields.next().ok_or(CookieImportError::Invalid)?;
        if fields.next().is_some() {
            return Err(CookieImportError::Invalid);
        }

        let (http_only, raw_domain) = raw_domain
            .strip_prefix("#HttpOnly_")
            .map_or((false, raw_domain), |domain| (true, domain));
        let subdomains = parse_bool(include_subdomains)?;
        let secure = parse_bool(secure)?;
        let expires = expires
            .parse::<u64>()
            .map_err(|_| CookieImportError::Invalid)?;
        if expires != 0 && expires <= now_unix_seconds {
            continue;
        }

        let host = raw_domain.trim_start_matches('.');
        validate_host(host)?;
        if raw_domain.len() > MAX_DOMAIN_BYTES || path.len() > MAX_PATH_BYTES {
            return Err(CookieImportError::Limit);
        }
        if !path.starts_with('/') || path.chars().any(char::is_control) || path.contains('\t') {
            return Err(CookieImportError::Invalid);
        }
        if name.is_empty()
            || name.len() > MAX_NAME_BYTES
            || !name.bytes().all(valid_cookie_name_byte)
        {
            return Err(CookieImportError::Invalid);
        }
        if value.len() > MAX_VALUE_BYTES
            || value.chars().any(char::is_control)
            || value.contains(['\t', '\r', '\n', '\0'])
        {
            return Err(CookieImportError::Invalid);
        }
        if count >= MAX_COOKIES {
            return Err(CookieImportError::Limit);
        }

        let domain = if subdomains {
            format!(".{host}")
        } else {
            host.to_owned()
        };
        if http_only {
            output.extend_from_slice(b"#HttpOnly_");
        }
        output.extend_from_slice(domain.as_bytes());
        output.push(b'\t');
        output.extend_from_slice(if subdomains { b"TRUE" } else { b"FALSE" });
        output.push(b'\t');
        output.extend_from_slice(path.as_bytes());
        output.push(b'\t');
        output.extend_from_slice(if secure { b"TRUE" } else { b"FALSE" });
        output.push(b'\t');
        output.extend_from_slice(expires.to_string().as_bytes());
        output.push(b'\t');
        output.extend_from_slice(name.as_bytes());
        output.push(b'\t');
        output.extend_from_slice(value.as_bytes());
        output.push(b'\n');
        count += 1;

        if output.len() as u64 > MAX_SOURCE_BYTES {
            return Err(CookieImportError::Limit);
        }
    }

    if count == 0 {
        return Err(CookieImportError::Empty);
    }
    Ok(ProtectedCookieJar {
        bytes: output,
        count,
    })
}

fn parse_bool(value: &str) -> Result<bool, CookieImportError> {
    match value {
        "TRUE" => Ok(true),
        "FALSE" => Ok(false),
        _ => Err(CookieImportError::Invalid),
    }
}

fn validate_host(host: &str) -> Result<(), CookieImportError> {
    if host.is_empty()
        || host.len() > MAX_DOMAIN_BYTES
        || host.chars().any(char::is_control)
        || host.contains(['/', '\\', '@', ':'])
    {
        return Err(CookieImportError::Invalid);
    }
    let parsed =
        url::Url::parse(&format!("https://{host}/")).map_err(|_| CookieImportError::Invalid)?;
    let normalized = parsed.host_str().ok_or(CookieImportError::Invalid)?;
    if normalized != host.to_ascii_lowercase() {
        return Err(CookieImportError::Invalid);
    }
    if !host.eq_ignore_ascii_case("localhost") && !host.contains('.') {
        return Err(CookieImportError::Invalid);
    }
    Ok(())
}

fn valid_cookie_name_byte(byte: u8) -> bool {
    byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={} ".contains(&byte)
}

fn open_read_nofollow(path: &Path) -> Result<File, CookieImportError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| CookieImportError::Storage)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(CookieImportError::Storage);
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x2_0000); // O_NOFOLLOW
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(0x0000_0100); // O_NOFOLLOW
    }
    options.open(path).map_err(|_| CookieImportError::Storage)
}

#[allow(dead_code)]
fn _type_anchor(_: PathBuf) {}
