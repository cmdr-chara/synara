use crate::{RegistryError, Result};
use std::path::PathBuf;

/// Apply Windows as well as POSIX rules regardless of the extraction host.
pub(crate) fn relative_path(value: &str) -> Result<PathBuf> {
    if value.is_empty() || value.len() > 2048 || value.chars().any(char::is_control) {
        return Err(RegistryError::Invalid(
            "empty or oversized archive path".into(),
        ));
    }
    let normalized = value.replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let normalized = normalized.strip_suffix('/').unwrap_or(normalized);
    let mut result = PathBuf::new();
    for part in normalized.split('/') {
        let basename = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        let device = matches!(basename.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$")
            || ["COM", "LPT"].iter().any(|prefix| {
                basename.strip_prefix(prefix).is_some_and(|n| {
                    matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                })
            });
        if part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || part.contains([':', '*', '?', '"', '<', '>', '|'])
            || device
        {
            return Err(RegistryError::Invalid(
                "unsafe or non-portable archive path".into(),
            ));
        }
        result.push(part);
    }
    Ok(result)
}

pub(crate) fn digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit())
}

pub(crate) fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_paths_on_both_operating_systems() {
        for path in [
            "../outside",
            "/absolute",
            "C:/agent",
            "C:\\agent",
            "\\\\server\\agent",
            "a/../b",
            "a\\..\\b",
            "a//b",
            "a/./b",
            "NUL",
            "com1.exe",
            "AUX/log",
            "a:ads",
            "a.",
            "a ",
            "",
            "./",
            "a\0b",
        ] {
            assert!(relative_path(path).is_err(), "accepted {path:?}");
        }
        assert_eq!(
            relative_path("./bin\\agent.exe").unwrap(),
            PathBuf::from("bin/agent.exe")
        );
        assert_eq!(
            relative_path("package/v1+build/tool").unwrap(),
            PathBuf::from("package/v1+build/tool")
        );
    }
}
