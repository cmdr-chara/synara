//! The Rust port of `apps/server/src/provider/testing/`.
//!
//! Derived from `cases.jsonl` plus the declared unit interface (clean-room):
//! writes a fake `@earendil-works/pi-*` install tree under `root` and returns
//! the binary path (`npm`/`bun` -> `<root>/bin/pi` symlink, `npm-cmd` ->
//! `pi.cmd`, `bun-exe` -> `pi.exe` + `pi.bunx` metadata).

#![forbid(unsafe_code)]

use std::path::{Component, Path, PathBuf};

/// `path.join` over forward-slash segments: resolve `.`/`..`, drop empties.
fn join_norm(parts: &[&str]) -> PathBuf {
    let mut out = PathBuf::new();
    for p in parts {
        if p.is_empty() {
            continue;
        }
        out.push(p);
    }
    out
}

fn normalized(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in path.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            other => out.push(other),
        }
    }
    out
}

/// `path.relative(from, to)` on already-normalized absolute paths.
fn relative(from: &Path, to: &Path) -> PathBuf {
    let f: Vec<_> = from.components().collect();
    let t: Vec<_> = to.components().collect();
    let mut i = 0;
    while i < f.len() && i < t.len() && f[i] == t[i] {
        i += 1;
    }
    let mut out = PathBuf::new();
    for _ in i..f.len() {
        out.push("..");
    }
    for c in &t[i..] {
        out.push(c);
    }
    out
}

fn cwd_join(p: &Path) -> PathBuf {
    normalized(&std::env::current_dir().unwrap_or_default().join(p))
}

fn node_err(code: &str, op: &str, p: &Path) -> String {
    format!("{code}: {op} '{}'", p.display())
}

fn mkdir(p: &Path) -> Result<(), String> {
    std::fs::create_dir_all(p).map_err(|e| node_err(errno(&e), "mkdir", p))
}

fn write(p: &Path, data: impl AsRef<[u8]>, mode: Option<u32>) -> Result<(), String> {
    std::fs::write(p, data).map_err(|e| node_err(errno(&e), "open", p))?;
    if let Some(m) = mode {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(p, std::fs::Permissions::from_mode(m));
        }
    }
    Ok(())
}

fn errno(e: &std::io::Error) -> &'static str {
    match e.raw_os_error() {
        Some(2) => "ENOENT",
        Some(17) => "EEXIST",
        Some(13) => "EACCES",
        Some(20) => "ENOTDIR",
        _ => "EIO",
    }
}

const PI_AI_PKG: &str = r#"{"name":"@earendil-works/pi-ai","type":"module","exports":{".":{"import":"./index.js"},"./compat":{"import":"./index.js"},"./providers/*":{"import":"./index.js"},"./utils/*":{"import":"./index.js"}}}"#;
const AGENT_PKG: &str = r#"{"name":"@earendil-works/pi-coding-agent","type":"module","exports":{".":{"import":"./index.js"}},"bin":{"pi":"./cli.js"}}"#;
const AGENT_INDEX: &str = "export { version } from \"@earendil-works/pi-ai\";\n";
const CLI_SHEBANG: &str = "#!/usr/bin/env node\n";

/// Port of `writePiInstallFixture` (`apps/server/src/provider/testing/piSdkFixture.ts:13`).
pub fn write_pi_install_fixture(
    root: String,
    version: String,
    layout: Option<String>,
) -> Result<serde_json::Value, String> {
    let layout = layout.as_deref().unwrap_or("npm"); // darwin/linux default
    let root = normalized(&join_norm(&[&root]));
    let binary_dir = if layout == "npm-cmd" {
        root.clone()
    } else {
        root.join("bin")
    };
    let mut modules = root.clone();
    if layout == "bun" || layout == "bun-exe" {
        modules.push("install");
        modules.push("global");
    } else if layout == "npm" {
        modules.push("lib");
    }
    modules.push("node_modules");
    let agent = modules.join("@earendil-works").join("pi-coding-agent");
    let ai = agent
        .join("node_modules")
        .join("@earendil-works")
        .join("pi-ai");
    for (directory, label) in [
        (ai.clone(), version.clone()),
        (
            modules.join("@earendil-works").join("pi-ai"),
            "hoisted-decoy".to_string(),
        ),
    ] {
        mkdir(&directory)?;
        write(&directory.join("package.json"), PI_AI_PKG, None)?;
        write(
            &directory.join("index.js"),
            format!("export const version = {};\n", serde_json::json!(label)),
            None,
        )?;
    }
    write(&agent.join("package.json"), AGENT_PKG, None)?;
    write(&agent.join("index.js"), AGENT_INDEX, None)?;
    let cli = agent.join("cli.js");
    write(&cli, CLI_SHEBANG, Some(0o755))?;
    let binary = binary_dir.join(match layout {
        "npm-cmd" => "pi.cmd",
        "bun-exe" => "pi.exe",
        _ => "pi",
    });
    mkdir(&binary_dir)?;
    if layout == "npm-cmd" {
        let rel = relative(&cwd_join(&binary_dir), &cwd_join(&cli));
        let rel_win = rel.display().to_string().replace('/', "\\");
        write(
            &binary,
            format!("@ECHO OFF\r\n\"node\" \"%dp0%\\{rel_win}\" %*\r\n"),
            Some(0o755),
        )?;
    } else if layout == "bun-exe" {
        let target = relative(&cwd_join(&root), &cwd_join(&cli))
            .display()
            .to_string()
            .replace('/', "\\");
        // Layout: [path utf16le][u16le 34][2 pad]["node " utf16le]
        //         [u32le pathLen][u32le argLen][u16le 0xABF7].
        let path_bytes: Vec<u8> = target.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let arg_bytes: Vec<u8> = "node ".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut metadata = Vec::with_capacity(path_bytes.len() + arg_bytes.len() + 14);
        metadata.extend_from_slice(&path_bytes);
        metadata.extend_from_slice(&34u16.to_le_bytes());
        metadata.extend_from_slice(&[0u8; 2]);
        metadata.extend_from_slice(&arg_bytes);
        metadata.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
        metadata.extend_from_slice(&(arg_bytes.len() as u32).to_le_bytes());
        metadata.extend_from_slice(&(((2739u16) << 4) | 7).to_le_bytes());
        write(&binary, "MZ", Some(0o755))?;
        write(&binary_dir.join("pi.bunx"), metadata, None)?;
    } else {
        let rel = relative(&cwd_join(&binary_dir), &cwd_join(&cli));
        #[cfg(unix)]
        std::os::unix::fs::symlink(&rel, &binary).map_err(|e| {
            let reason = if errno(&e) == "EEXIST" {
                "file already exists"
            } else {
                "no such file or directory"
            };
            format!(
                "{}: {}, symlink '{}' -> '{}'",
                errno(&e),
                reason,
                rel.display(),
                binary.display()
            )
        })?;
    }
    Ok(serde_json::Value::String(binary.display().to_string()))
}
