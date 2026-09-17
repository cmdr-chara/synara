use crate::{RegistryError, Result};
use std::path::PathBuf;

const MAX_PATH_DEPTH: usize = 64;

/// Apply Windows as well as POSIX rules regardless of the extraction host.
pub(crate) fn relative_path(value: &str) -> Result<PathBuf> {
    if value.is_empty()
        || value.len() > 2048
        || value
            .chars()
            .any(|c| c.is_control() || directional_control(c))
    {
        return Err(RegistryError::Invalid(
            "empty, oversized or control-bearing archive path".into(),
        ));
    }
    let normalized = value.replace('\\', "/");
    let normalized = normalized.strip_prefix("./").unwrap_or(&normalized);
    let normalized = normalized.strip_suffix('/').unwrap_or(normalized);
    let mut result = PathBuf::new();
    for (depth, part) in normalized.split('/').enumerate() {
        if depth >= MAX_PATH_DEPTH {
            return Err(RegistryError::Limit);
        }
        if part.is_empty()
            || matches!(part, "." | "..")
            || part.ends_with(['.', ' '])
            || part.contains([':', '*', '?', '"', '<', '>', '|'])
            || device_name(part)
        {
            return Err(RegistryError::Invalid(
                "unsafe or non-portable archive path".into(),
            ));
        }
        result.push(part);
    }
    Ok(result)
}

fn device_name(part: &str) -> bool {
    let basename = part
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    matches!(
        basename.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CLOCK$" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        basename.strip_prefix(prefix).is_some_and(|n| {
            // Win32 also reserves the ISO-8859-1 superscript digits 1, 2 and 3.
            matches!(
                n,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    })
}

fn directional_control(c: char) -> bool {
    matches!(
        c,
        '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
    )
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

    #[test]
    fn rejects_windows_device_aliases_before_touching_the_filesystem() {
        for name in [
            "COM¹", "com²", "COM³", "LPT¹", "lpt²", "LPT³", "CONIN$", "CONOUT$",
        ] {
            for path in [
                name.to_owned(),
                format!("bin/{name}.exe"),
                format!("{name}/child"),
            ] {
                assert!(relative_path(&path).is_err(), "accepted {path:?}");
            }
        }
        assert!(relative_path("COM1 .exe").is_err());
        for path in ["company/tool", "COM10.txt", "LPT0", "bin/agent-²"] {
            assert!(relative_path(path).is_ok(), "rejected {path:?}");
        }
    }

    #[test]
    fn rejects_visual_path_spoofing_but_preserves_normal_unicode() {
        for c in ['\u{061c}', '\u{200f}', '\u{202e}', '\u{2066}', '\u{2069}'] {
            assert!(relative_path(&format!("bin/agent{c}exe")).is_err());
        }
        for path in ["工具/agent", "café/agent", "مسار/agent"] {
            assert!(relative_path(path).is_ok());
        }
    }

    #[test]
    fn directory_depth_is_bounded_independently_of_byte_length() {
        assert!(relative_path(&vec!["a"; MAX_PATH_DEPTH].join("/")).is_ok());
        assert!(matches!(
            relative_path(&vec!["a"; MAX_PATH_DEPTH + 1].join("/")),
            Err(RegistryError::Limit)
        ));
    }
}
