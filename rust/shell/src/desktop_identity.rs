//! Packaged desktop flavor and flavor-owned data home resolution.
//! Mirrors the stable/beta/remote separation in packages/shared/desktopIdentity:
//! parallel installs must never migrate or update into one another's state.

use std::path::{Path, PathBuf};

fn valid_flavor(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "production" => Some("production"),
        "canary" => Some("canary"),
        "cua" => Some("cua"),
        "beta" => Some("beta"),
        "remote" => Some("remote"),
        _ => None,
    }
}

fn flavor_for_bundle_id(bundle_id: &str) -> Option<&'static str> {
    match bundle_id.trim() {
        "com.emanueledipietro.synara" => Some("production"),
        "com.emanueledipietro.synara.canary" => Some("canary"),
        "com.emanueledipietro.synara.cua" => Some("cua"),
        "com.emanueledipietro.synara.beta" => Some("beta"),
        "com.emanueledipietro.synara.remote" => Some("remote"),
        _ => None,
    }
}

fn current_app_info_plist() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    for ancestor in exe.ancestors() {
        if ancestor.extension().and_then(|value| value.to_str()) == Some("app") {
            return Some(ancestor.join("Contents").join("Info.plist"));
        }
    }
    None
}

fn plist_string_value(text: &str, key: &str) -> Option<String> {
    let needle = format!("<key>{key}</key>");
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim() != needle {
            continue;
        }
        for next in lines.by_ref() {
            let trimmed = next.trim();
            if trimmed.is_empty() {
                continue;
            }
            return trimmed
                .strip_prefix("<string>")
                .and_then(|value| value.strip_suffix("</string>"))
                .map(str::to_string);
        }
    }
    None
}

fn plist_value(path: &Path, key: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("/usr/libexec/PlistBuddy")
            .arg("-c")
            .arg(format!("Print:{key}"))
            .arg(path)
            .output()
        {
            if output.status.success() {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    plist_string_value(&std::fs::read_to_string(path).ok()?, key)
}

fn flavor_for_executable_name(name: &str) -> Option<&'static str> {
    let normalized = name
        .trim_end_matches(".exe")
        .trim_end_matches(".EXE")
        .replace(' ', "-")
        .to_ascii_lowercase();
    match normalized.as_str() {
        "synara-beta" => Some("beta"),
        "synara-remote" => Some("remote"),
        "synara-canary" => Some("canary"),
        "synara-cua" => Some("cua"),
        "synara" => Some("production"),
        _ => None,
    }
}

fn packaged_flavor() -> Option<&'static str> {
    if let Some(plist) = current_app_info_plist() {
        if let Some(value) = plist_value(&plist, "SynaraDesktopFlavor") {
            if let Some(flavor) = valid_flavor(&value) {
                return Some(flavor);
            }
        }
        if let Some(flavor) =
            plist_value(&plist, "CFBundleIdentifier").and_then(|id| flavor_for_bundle_id(&id))
        {
            return Some(flavor);
        }
    }
    // Linux and Windows do not carry the macOS bundle plist. Their packaged
    // flavors use dedicated executable names, which are fixed inside the
    // AppImage/installer even when the outer downloaded file is renamed.
    let exe = std::env::current_exe().ok()?;
    exe.file_name()
        .and_then(|name| name.to_str())
        .and_then(flavor_for_executable_name)
}

/// The runtime flavor. Packaged bundle metadata is authoritative; the
/// environment override exists for source/dev launches and must not be able to
/// turn an installed beta into stable (or the reverse).
pub(crate) fn runtime_flavor() -> &'static str {
    if let Some(flavor) = packaged_flavor() {
        return flavor;
    }
    std::env::var("SYNARA_DESKTOP_FLAVOR")
        .ok()
        .and_then(|value| valid_flavor(&value))
        .unwrap_or("production")
}

pub(crate) fn default_home_directory_name(flavor: &str) -> &'static str {
    match flavor {
        "canary" => ".synara-canary",
        "cua" => ".synara-cua",
        "beta" => ".synara-beta",
        "remote" => ".synara-remote",
        _ => ".synara",
    }
}

pub(crate) fn configured_home_dir() -> PathBuf {
    let flavor = runtime_flavor();
    // Beta/Remote deliberately ignore SYNARA_HOME: a globally exported stable
    // home must not make the parallel app migrate stable's database.
    let override_value = match flavor {
        "beta" => std::env::var_os("SYNARA_BETA_HOME"),
        "remote" => std::env::var_os("SYNARA_REMOTE_HOME"),
        _ => std::env::var_os("SYNARA_HOME"),
    };
    if let Some(path) = override_value.filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    let directory = default_home_directory_name(flavor);
    if let Some(home) = std::env::var_os("HOME") {
        return Path::new(&home).join(directory);
    }
    PathBuf::from(directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_ids_resolve_to_isolated_flavors() {
        assert_eq!(
            flavor_for_bundle_id("com.emanueledipietro.synara"),
            Some("production")
        );
        assert_eq!(
            flavor_for_bundle_id("com.emanueledipietro.synara.beta"),
            Some("beta")
        );
        assert_eq!(
            flavor_for_bundle_id("com.emanueledipietro.synara.remote"),
            Some("remote")
        );
        assert_eq!(
            flavor_for_bundle_id("com.emanueledipietro.synara.canary"),
            Some("canary")
        );
        assert_eq!(
            flavor_for_bundle_id("com.emanueledipietro.synara.cua"),
            Some("cua")
        );
        assert_eq!(flavor_for_bundle_id("com.example.other"), None);
    }

    #[test]
    fn executable_names_resolve_non_macos_flavors() {
        assert_eq!(flavor_for_executable_name("synara"), Some("production"));
        assert_eq!(flavor_for_executable_name("synara-beta"), Some("beta"));
        assert_eq!(flavor_for_executable_name("Synara Beta.exe"), Some("beta"));
        assert_eq!(flavor_for_executable_name("synara-remote"), Some("remote"));
        assert_eq!(flavor_for_executable_name("custom-renamed"), None);
    }

    #[test]
    fn flavor_default_homes_are_isolated() {
        assert_eq!(default_home_directory_name("production"), ".synara");
        assert_eq!(default_home_directory_name("beta"), ".synara-beta");
        assert_eq!(default_home_directory_name("remote"), ".synara-remote");
        assert_eq!(default_home_directory_name("canary"), ".synara-canary");
        assert_eq!(default_home_directory_name("cua"), ".synara-cua");
    }
}
