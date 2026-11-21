//! The Rust port of `scripts/check-brand-identity.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::process::Command;

fn characters(codes: &[u16]) -> String {
    String::from_utf16(codes).unwrap_or_default()
}

fn retired_short_name() -> String {
    characters(&[116, 51])
}
fn retired_first_name() -> String {
    format!(
        "{}{}",
        retired_short_name(),
        characters(&[99, 111, 100, 101])
    )
}
fn retired_company_name() -> String {
    format!(
        "{}{}",
        retired_short_name(),
        characters(&[116, 111, 111, 108, 115])
    )
}
fn retired_second_name() -> String {
    characters(&[100, 112, 99, 111, 100, 101])
}
fn retired_predecessor_name() -> String {
    characters(&[99, 111, 100, 101, 116, 104, 105, 110, 103])
}
fn incorrect_bundle_domain() -> String {
    characters(&[99, 111, 109, 46, 115, 121, 110, 97, 114, 97])
}
fn retired_first_display_name() -> String {
    characters(&[84, 51, 67, 111, 100, 101])
}
fn retired_first_spaced_display_name() -> String {
    format!("{} Code", characters(&[84, 51]))
}
fn retired_company_display_name() -> String {
    format!(
        "{} {}",
        characters(&[84, 51]),
        characters(&[84, 111, 111, 108, 115])
    )
}

fn is_undef(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

fn received_desc(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("type boolean ({b})"),
        Value::Number(n) => format!(
            "type number ({})",
            crate::js::number_to_string(n.as_f64().unwrap_or(0.0))
        ),
        Value::String(s) => format!("type string ('{s}')"),
        Value::Array(_) => "an instance of Array".to_string(),
        Value::Object(_) => "an instance of Object".to_string(),
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn is_js_sep(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}' || matches!(c, '.' | '_' | '/' | '@' | ':' | '-')
}

// `left[\s._/@:-]*right`, case-insensitive.
fn joined_match(hay: &str, left: &str, right: &str) -> bool {
    let lower = hay.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let llen = left.chars().count();
    let rlen = right.chars().count();
    for i in 0..chars.len() {
        if i + llen > chars.len() {
            break;
        }
        let window: String = chars[i..i + llen].iter().collect();
        if window != left {
            continue;
        }
        let mut j = i + llen;
        while j < chars.len() && is_js_sep(chars[j]) {
            j += 1;
        }
        if j + rlen <= chars.len() {
            let tail: String = chars[j..j + rlen].iter().collect();
            if tail == right {
                return true;
            }
        }
    }
    false
}

fn is_t3_boundary(c: char) -> bool {
    c.is_whitespace()
        || c == '\u{feff}'
        || matches!(c, '"' | '\'' | '`' | '.' | '/' | ':' | '@' | '_' | '-')
}

// `(?:^|[\s"'\x60./:@_-])t3(?:$|[\s"'\x60./:@_-])`, case-insensitive.
fn bare_t3_match(hay: &str, needle: &str) -> bool {
    let lower = hay.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let nlen = needle.chars().count();
    let nchars: Vec<char> = needle.chars().collect();
    for i in 0..chars.len() {
        if i + nlen > chars.len() {
            break;
        }
        if chars[i..i + nlen] != nchars[..] {
            continue;
        }
        let before_ok = i == 0 || is_t3_boundary(chars[i - 1]);
        let after_ok = i + nlen == chars.len() || is_t3_boundary(chars[i + nlen]);
        if before_ok && after_ok {
            return true;
        }
    }
    false
}

fn contains_forbidden_identity(value: &str) -> bool {
    let short = retired_short_name();
    let second = retired_second_name();
    let lower = value.to_lowercase();
    joined_match(value, &short, &retired_first_name()[short.len()..])
        || joined_match(value, &short, &retired_company_name()[short.len()..])
        || joined_match(value, &second[..2], &second[2..])
        || lower.contains(&retired_predecessor_name())
        || lower.contains(&format!("@{}", retired_company_name()))
        || bare_t3_match(value, &short)
        || lower.contains(&incorrect_bundle_domain())
}

struct ApprovedLine {
    path: &'static str,
    line: String,
    markdown_section: Option<&'static str>,
}

fn approved_identity_lines() -> Vec<ApprovedLine> {
    let company = retired_company_display_name();
    let first = retired_first_display_name();
    let spaced = retired_first_spaced_display_name();
    let short = retired_short_name();
    let mut lines = vec![
        ApprovedLine {
            path: "docs/implementation/cloudflare-remote/UPSTREAM-COMPATIBILITY.md",
            markdown_section: Some("# Upstream managed-connection compatibility"),
            line: format!(
                "Read-only upstream: [upstream repository](https://github.com/pingdotgg/{}/tree/d15210cd3da79f9a1a495a6309d912d76362a046), commit `d15210cd3da79f9a1a495a6309d912d76362a046`, 2026-09-28. Required references read: `docs/internals/{}-connect.md`, `docs/operations/connect-setup.md`, `docs/user/remote-access.md`, `infra/relay/README.md`. Followed `ManagedEndpointProvider.ts`, `ManagedEndpointRuntime.ts`, and shared `relayClient.ts`. MIT, Copyright 2026 {} Inc. Patterns adapted; no source copied at this checkpoint.",
                retired_first_name(),
                short,
                company
            ),
        },
        ApprovedLine {
            path: "LICENSE",
            markdown_section: None,
            line: format!("Copyright (c) 2026 {} Inc.", company),
        },
        ApprovedLine {
            path: "README.md",
            markdown_section: Some("## Origins"),
            line: format!(
                "Synara began as a clone of [{}](https://github.com/pingdotgg/{}), but it has since become a substantially different product with its own branding, packaging, release system, provider orchestration, desktop app behavior, and product direction.",
                first,
                retired_first_name()
            ),
        },
        ApprovedLine {
            path: "CHANGELOG.md",
            markdown_section: Some("## 0.7.0 - 2026-08-05"),
            line: format!(
                "**A review of the Synara codebase found an analytics configuration that came from the original {} codebase when Synara was created as a clone in March. We did not add it, and we have no access to the PostHog project receiving the events.**",
                spaced
            ),
        },
        ApprovedLine {
            path: "apps/web/src/whatsNew/entries.ts",
            markdown_section: None,
            line: format!(
                "\"A review of the Synara codebase found an analytics configuration that came from the original {} codebase when Synara was created as a clone in March.\",",
                spaced
            ),
        },
        ApprovedLine {
            path: "apps/marketing/src/data/changelog.ts",
            markdown_section: None,
            line: format!(
                "\"A review of the Synara codebase found an analytics configuration that came from the original {} codebase when Synara was created as a clone in March.\",",
                first[..2].to_string() + " Code"
            ),
        },
        ApprovedLine {
            path: "apps/marketing/src/data/testimonials.ts",
            markdown_section: None,
            line: format!(
                "\"I've been using @trySynara for a few hours now. I'm really impressed. I'd already tried {} Chat, Orca, and Terax, but none of them managed to grab my attention quite like Synara did.\",",
                &first[..2]
            ),
        },
        ApprovedLine {
            path: "docs/computer-use-cua/extraction-plan.md",
            markdown_section: Some("## Upstream license and PR-back feasibility"),
            line: format!(
                "also MIT (LICENSE, {} Inc and Emanuele Di Pietro). There is no license",
                company
            ),
        },
        ApprovedLine {
            path: "docs/computer-use-cua/handoffs/synara-cu-v2-mega-handoff-2026-09-16.md",
            markdown_section: None,
            line: format!(
                "- Decide what ships open: likely the driver patch + host protocol + tools + docs (MIT/Apache), either as its own project or upstreamed into trycua/cua; Synara product code can stay whatever it is. Keep `CUA-LICENSE.txt` attribution; Synara repo is MIT ({} Inc + Emanuele Di Pietro).",
                company
            ),
        },
        ApprovedLine {
            path: "docs/computer-use-cua/open-decisions-sheet.md",
            markdown_section: Some("## 3. Which license"),
            line: format!(
                "- Verification: Synara repo is MIT, held by {} Inc and Emanuele Di Pietro",
                company
            ),
        },
        ApprovedLine {
            path: "docs/computer-use-cua/workstream-f-opensource-spec.md",
            markdown_section: Some("## Current state"),
            line: format!(
                "The tree already carries two licenses. The Synara repo is MIT, held by {} Inc and Emanuele Di Pietro (`LICENSE:1`, `LICENSE:3`). The Cua driver redistribution license is MIT, held by Cua AI Inc (`docs/computer-use-cua/CUA-LICENSE.txt:1`, `docs/computer-use-cua/CUA-LICENSE.txt:3`). The redistribution license is referenced from the computer use README (`docs/computer-use-cua/README.md:15`). A full read of `package.json:1` through the end of the file shows no license field, so the root manifest states no license of its own. This is unverified as a problem, but an auditor will flag it.",
                company
            ),
        },
    ];
    for axis in 0..2 {
        lines.push(ApprovedLine {
            path: "apps/desktop/patches/cua-driver/0001-synara-native.patch",
            markdown_section: None,
            line: format!(
                "+                            + {} * self.to.{}",
                short, axis
            ),
        });
    }
    lines
}

fn fixture_paths() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for p in [
        "apps/desktop/src/cuaFixtures/electron.ts",
        "apps/desktop/src/cuaFixtures/live.ts",
        "scripts/computer-use-fixtures/build-electron.mjs",
        "docs/computer-use-cua/evidence/efficiency-implementation-2026-09-08/native/signed-fixture-permission-check.json",
        "docs/computer-use-cua/evidence/fixture-g5-set-value-2026-09-17.report.json",
        "docs/computer-use-cua/evidence/gateway-native-report.json",
        "docs/computer-use-cua/evidence/live-provider-report.json",
        "docs/computer-use-cua/evidence/native-fixture-report.json",
        "docs/computer-use-cua/evidence/native-fixture-run6-report.json",
        "docs/computer-use-cua/evidence/native-revision1-final-report.json",
        "docs/computer-use-cua/evidence/native-revision1-foreground-initial-report.json",
        "docs/computer-use-cua/evidence/native-revision1-renewed-build-permission-refusal.json",
        "docs/computer-use-cua/evidence/native-revision1-report.json",
        "docs/computer-use-cua/evidence/overlay-disabled-report.json",
        "docs/computer-use-cua/evidence/overlay-enabled-report.json",
        "docs/computer-use-cua/evidence/rev15-electron-2026-09-17-report.json",
        "docs/computer-use-cua/evidence/short-cursor-report.json",
    ] {
        out.push((p, "cua-fixture"));
    }
    for p in [
        "docs/computer-use-cua/evidence/rev17-cancellation-2026-09-17-notes.md",
        "docs/computer-use-cua/evidence/rev17-cancellation-2026-09-17-report.json",
        "docs/computer-use-cua/evidence/rev17-gateway-2026-09-17-notes.md",
        "docs/computer-use-cua/evidence/rev17-gateway-2026-09-17-report.json",
        "docs/computer-use-cua/evidence/rev17-live-2026-09-17-live-report.json",
        "docs/computer-use-cua/evidence/rev17-live-2026-09-17-notes.md",
        "docs/computer-use-cua/evidence/rev17-live-2026-09-17-report.json",
        "docs/computer-use-cua/evidence/rev17-native-2026-09-17-notes.md",
        "docs/computer-use-cua/evidence/rev17-native-2026-09-17-report.json",
    ] {
        out.push((p, "cua-fixture-external"));
    }
    for p in [
        "scripts/computer-use-fixtures/build-canary.mjs",
        "scripts/computer-use-fixtures/canary-main.ts",
        "scripts/computer-use-fixtures/live-cert.ts",
        "docs/computer-use-cua/belief-canary-runbook.md",
        "docs/computer-use-cua/handoffs/synara-cu-v2-session-handoff-2026-09-17.md",
        "docs/computer-use-cua/evidence/rev20-realapp-calculator-2026-09-17-report.json",
        "docs/computer-use-cua/evidence/rev20-realapp-notes-2026-09-17-report.json",
    ] {
        out.push((p, "cua-canary"));
    }
    for p in [
        "apps/server/src/computer/CuaComputerBackend.test.ts",
        "apps/server/src/computer/computerSignatureChange.test.ts",
    ] {
        out.push((p, "test"));
    }
    for p in [
        "docs/computer-use-cua/evidence/latency-rev17-probe.ts",
        "docs/computer-use-cua/evidence/latency-rev17-supplement-probe.ts",
    ] {
        out.push((p, "latency-probe"));
    }
    out.push((
        "scripts/computer-use-fixtures/multi-display-cert.ts",
        "cua-display-cert",
    ));
    out
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-'
}

// Removes every `com.synara.<suffix>` bounded by non-identifier chars (case-sensitive).
fn strip_fixture_identity(line: &str, suffix: &str) -> String {
    let needle = format!("{}.{}", incorrect_bundle_domain(), suffix);
    let bytes = line.as_bytes();
    let nb = needle.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + nb.len() <= bytes.len() && &bytes[i..i + nb.len()] == nb {
            let before = i == 0 || !is_ident_char(bytes[i - 1]);
            let after = i + nb.len() == bytes.len() || !is_ident_char(bytes[i + nb.len()]);
            if before && after {
                i += nb.len();
                continue;
            }
        }
        // Copy one UTF-8 char.
        let ch_len = line[i..].chars().next().map_or(1, |c| c.len_utf8());
        out.push_str(&line[i..i + ch_len]);
        i += ch_len;
    }
    out
}

fn violation(path: &Value, line: Value, text: &Value) -> Value {
    let mut o = Map::new();
    o.insert("path".into(), path.clone());
    o.insert("line".into(), line);
    o.insert("text".into(), text.clone());
    Value::Object(o)
}

/// Port of `findBrandIdentityViolations` (`scripts/check-brand-identity.ts:256`).
pub fn find_brand_identity_violations(files: Value) -> Result<Vec<Value>, String> {
    let files_iter: Vec<Value> = match &files {
        Value::Array(a) => a.clone(),
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("files is not iterable".to_string()),
    };
    let approved = approved_identity_lines();
    let fixture_map: HashMap<&'static str, &'static str> = fixture_paths().into_iter().collect();
    let mut violations = Vec::new();
    for file in &files_iter {
        let path = file.get("path").cloned().unwrap_or(Value::Null);
        let path_str = match &path {
            Value::String(s) => s.clone(),
            other => js_string_coerce(other),
        };
        if contains_forbidden_identity(&path_str) {
            violations.push(violation(&path, Value::Null, &path));
        }
        let contents = match file.get("contents") {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Null) => {
                return Err("Cannot read properties of null (reading 'split')".to_string());
            }
            Some(v) if crate::js::is_undefined(v) => {
                return Err("Cannot read properties of undefined (reading 'split')".to_string());
            }
            None => {
                return Err("Cannot read properties of undefined (reading 'split')".to_string());
            }
            _ => return Err("file.contents.split is not a function".to_string()),
        };
        let mut consumed: HashSet<usize> = HashSet::new();
        let fixture_suffix = fixture_map.get(path_str.as_str()).copied();
        let mut markdown_section: Option<String> = None;
        for (index, raw) in contents.split('\n').enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw);
            if line.starts_with('#') {
                let rest = line.trim_start_matches('#');
                let hashes = line.len() - rest.len();
                if (1..=2).contains(&hashes)
                    && rest.chars().next().is_some_and(|c| c.is_whitespace())
                {
                    markdown_section = Some(js_trim(line).to_string());
                }
            }
            let to_check = match fixture_suffix {
                Some(suf) => strip_fixture_identity(line, suf),
                None => line.to_string(),
            };
            if !contains_forbidden_identity(&to_check) {
                continue;
            }
            let trimmed = js_trim(line);
            let mut matched: Option<usize> = None;
            for (ci, a) in approved.iter().enumerate() {
                if consumed.contains(&ci) {
                    continue;
                }
                if a.path != path_str {
                    continue;
                }
                if a.line != trimmed {
                    continue;
                }
                let section_ok = match (&a.markdown_section, &markdown_section) {
                    (None, _) => true,
                    (Some(want), Some(got)) => want == got,
                    (Some(_), None) => false,
                };
                if section_ok {
                    matched = Some(ci);
                    break;
                }
            }
            if let Some(ci) = matched {
                consumed.insert(ci);
                continue;
            }
            violations.push(violation(
                &path,
                json!(index + 1),
                &Value::String(trimmed.to_string()),
            ));
        }
    }
    Ok(violations)
}

fn js_string_coerce(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_string_coerce).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn default_digests() -> Vec<(Value, Value)> {
    vec![
        (
            Value::String("apps/marketing/public/screenshot.jpeg".into()),
            Value::String(
                "0b4be139f13dd08885a1aac26fc1f7c623697db157777d16360e985c93d47bcf".into(),
            ),
        ),
        (
            Value::String("assets/prod/synara-hero.jpeg".into()),
            Value::String(
                "07fbd00bde259b5ed2c69f404c00c1347de2fa46fa4a5e2aa70f016912dc2490".into(),
            ),
        ),
    ]
}

fn hashable_contents(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::String(s) => Ok(s.clone().into_bytes()),
        Value::Object(o) => {
            for key in ["$bytes", "$buffer"] {
                if let Some(Value::Array(b)) = o.get(key) {
                    return Ok(b
                        .iter()
                        .filter_map(|x| x.as_u64().and_then(|n| u8::try_from(n).ok()))
                        .collect());
                }
            }
            Err(format!(
                "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
                received_desc(v)
            ))
        }
        _ => Err(format!(
            "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
            received_desc(v)
        )),
    }
}

/// Port of `findVisualBrandAssetViolations` (`scripts/check-brand-identity.ts:289`).
pub fn find_visual_brand_asset_violations(
    files: Value,
    approved_digests: Value,
) -> Result<Vec<Value>, String> {
    let file_list = match &files {
        Value::Array(a) => a.clone(),
        Value::Null => {
            return Err("Cannot read properties of null (reading 'map')".to_string());
        }
        other if crate::js::is_undefined(other) => {
            return Err("Cannot read properties of undefined (reading 'map')".to_string());
        }
        _ => return Err("files.map is not a function".to_string()),
    };
    let mut files_by_path: HashMap<String, Value> = HashMap::new();
    for f in &file_list {
        let key = match f.get("path") {
            Some(Value::String(s)) => s.clone(),
            _ => js_string_coerce(&Value::Null),
        };
        files_by_path.insert(key, f.clone());
    }
    let pairs: Vec<(Value, Value)> = if crate::js::is_undefined(&approved_digests) {
        default_digests()
    } else {
        let elements: Vec<Value> = match &approved_digests {
            Value::Array(a) => a.clone(),
            Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
            _ => return Err("approvedDigests is not iterable".to_string()),
        };
        let mut pairs = Vec::new();
        for el in elements {
            match el {
                Value::Array(a) => {
                    let path = a.first().cloned().unwrap_or_else(crate::js::undefined);
                    let digest = a.get(1).cloned().unwrap_or_else(crate::js::undefined);
                    pairs.push((path, digest));
                }
                Value::String(s) => {
                    let mut chars = s.chars();
                    let path = chars
                        .next()
                        .map(|c| Value::String(c.to_string()))
                        .unwrap_or_else(crate::js::undefined);
                    let digest = chars
                        .next()
                        .map(|c| Value::String(c.to_string()))
                        .unwrap_or_else(crate::js::undefined);
                    pairs.push((path, digest));
                }
                _ => return Err(".for is not iterable".to_string()),
            }
        }
        pairs
    };
    let mut violations = Vec::new();
    for (path, digest) in &pairs {
        let key = js_string_coerce(path);
        let missing = || {
            violation(
                path,
                Value::Null,
                &Value::String("Required visual brand asset is missing.".into()),
            )
        };
        let Some(file) = files_by_path.get(&key) else {
            violations.push(missing());
            continue;
        };
        let contents = hashable_contents(file.get("contents").unwrap_or(&Value::Null))?;
        let hex = Sha256::digest(&contents)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let expected = js_string_coerce(digest);
        if hex != expected {
            violations.push(violation(
                path,
                Value::Null,
                &Value::String(
                    "Visual brand asset changed; perform a visual identity review before approving it."
                        .into(),
                ),
            ));
        }
    }
    Ok(violations)
}

fn node_path_resolve(cwd: &str, path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("{}/{}", cwd.trim_end_matches('/'), path)
    }
}

/// Port of `readTrackedFiles` (`scripts/check-brand-identity.ts:317`).
pub fn read_tracked_files(cwd: Value) -> Result<Vec<Value>, String> {
    let dir = if is_undef(&cwd) {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    } else {
        match &cwd {
            Value::String(s) => s.clone(),
            other => {
                return Err(format!(
                    "The \"options.cwd\" property must be of type string or an instance of Buffer or URL. Received {}",
                    received_desc(other)
                ));
            }
        }
    };
    let output = Command::new("git")
        .args(["ls-files", "--stage", "-z"])
        .current_dir(&dir)
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("spawnSync git ENOENT".to_string());
        }
        Err(e) => return Err(format!("spawnSync git {}", e)),
    };
    if !output.status.success() {
        return Err(format!(
            "Command failed: git ls-files --stage -z\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut files = Vec::new();
    for entry in stdout.split('\0').filter(|e| !e.is_empty()) {
        if entry.starts_with("160000 ") {
            continue;
        }
        let Some(tab) = entry.find('\t') else {
            continue;
        };
        let path = &entry[tab + 1..];
        let full = node_path_resolve(&dir, path);
        let contents = match std::fs::read(&full) {
            Ok(b) => b,
            Err(_) => {
                return Err(format!("ENOENT: no such file or directory, open '{full}'"));
            }
        };
        let mut o = Map::new();
        o.insert("path".into(), Value::String(path.to_string()));
        o.insert(
            "contents".into(),
            json!({ "$buffer": contents.iter().map(|b| *b as u64).collect::<Vec<u64>>() }),
        );
        files.push(Value::Object(o));
    }
    Ok(files)
}
