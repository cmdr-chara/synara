//! `provider.listSkillsCatalog`, `provider.listSkills` and
//! `provider.getComposerCapabilities` — filesystem skill discovery over the
//! same roots the server scans (`~/.synara/skills`-equivalent plus every
//! provider-native skills folder), without needing provider CLIs.

use serde_json::{Value, json};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::todo::WsErr;

pub(crate) fn handles(tag: &str) -> bool {
    matches!(
        tag,
        "provider.listSkillsCatalog"
            | "provider.listSkills"
            | "provider.getComposerCapabilities"
            | "provider.listPlugins"
            | "provider.readPlugin"
    )
}

pub(crate) fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "provider.listSkillsCatalog" => list_skills_catalog(payload),
        "provider.listSkills" => list_skills(payload),
        "provider.getComposerCapabilities" => composer_capabilities(payload),
        "provider.listPlugins" => list_plugins(payload),
        "provider.readPlugin" => read_plugin(payload),
        _ => Err(err(format!("unhandled native tag {tag}"))),
    }
}

fn err(message: impl Into<String>) -> WsErr {
    WsErr(message.into(), None)
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn base_dir() -> PathBuf {
    // state_dir() is `<base>/userdata`.
    PathBuf::from(crate::todo::state_dir())
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(crate::todo::state_dir()))
}

fn synara_skills_dir() -> PathBuf {
    base_dir().join("skills")
}

fn canon(path: &Path) -> Option<PathBuf> {
    path.canonicalize().ok()
}

// ── Frontmatter parsing ──────────────────────────────────────────────

fn strip_quotes(value: &str) -> String {
    let t = value.trim();
    if (t.starts_with('"') && t.ends_with('"') && t.len() >= 2)
        || (t.starts_with('\'') && t.ends_with('\'') && t.len() >= 2)
    {
        t[1..t.len() - 1].trim().to_string()
    } else {
        t.to_string()
    }
}

fn parse_scalar(value: &str) -> Value {
    let unquoted = strip_quotes(value);
    match unquoted.to_lowercase().as_str() {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(unquoted),
    }
}

/// Minimal YAML-subset frontmatter reader: `key: value` lines, `key: >`/`>-`
/// folded blocks, and one nested map level (for `metadata.short-description`).
/// Falls back to the lenient line reader on anything else — matching the
/// server's parse-then-fallback contract.
fn parse_frontmatter(markdown: &str) -> Vec<(String, Value)> {
    let normalized = markdown.replace("\r\n", "\n");
    if !normalized.starts_with("---\n") {
        return Vec::new();
    }
    let rest = &normalized[4..];
    let Some(end) = rest.find("\n---") else {
        return Vec::new();
    };
    let block = &rest[..end];

    let mut out: Vec<(String, Value)> = Vec::new();
    let mut metadata_short: Option<String> = None;
    let mut lines = block.lines().peekable();
    let mut in_metadata = false;
    while let Some(line) = lines.next() {
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        if !indented {
            in_metadata = false;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let key = line[..colon].trim().to_string();
        let raw_value = line[colon + 1..].trim();
        if key.is_empty() {
            continue;
        }
        if !indented && raw_value.is_empty() && key == "metadata" {
            in_metadata = true;
            continue;
        }
        if in_metadata && indented {
            if key == "short-description" && !raw_value.is_empty() {
                metadata_short = Some(strip_quotes(raw_value));
            }
            continue;
        }
        if matches!(raw_value, ">" | ">-" | "|" | "|-" | ">+" | "|+") {
            // Folded/literal block scalar: consume more-indented lines.
            let folded = matches!(raw_value, ">" | ">-" | ">+");
            let mut parts: Vec<String> = Vec::new();
            while let Some(next) = lines.peek() {
                if next.trim().is_empty() {
                    lines.next();
                    continue;
                }
                if next.starts_with(' ') || next.starts_with('\t') {
                    parts.push(next.trim().to_string());
                    lines.next();
                } else {
                    break;
                }
            }
            let value = if folded {
                parts.join(" ")
            } else {
                parts.join("\n")
            };
            out.push((key, Value::String(value)));
            continue;
        }
        if raw_value.is_empty() {
            continue;
        }
        out.push((key, parse_scalar(raw_value)));
    }
    if let Some(short) = metadata_short {
        if !out.iter().any(|(k, _)| k == "short-description") {
            out.push(("short-description".to_string(), Value::String(short)));
        }
    }
    out
}

fn read_str(fields: &[(String, Value)], keys: &[&str]) -> Option<String> {
    for key in keys {
        for (k, v) in fields {
            if k == key {
                if let Value::String(s) = v {
                    let t = s.trim();
                    if !t.is_empty() {
                        return Some(t.to_string());
                    }
                }
            }
        }
    }
    None
}

fn read_bool(fields: &[(String, Value)], keys: &[&str]) -> Option<bool> {
    for key in keys {
        for (k, v) in fields {
            if k == key {
                if let Value::Bool(b) = v {
                    return Some(*b);
                }
            }
        }
    }
    None
}

// ── Filesystem walking ───────────────────────────────────────────────

fn ancestors(cwd: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut current = PathBuf::from(cwd);
    loop {
        out.push(current.clone());
        if !current.pop() {
            return out;
        }
    }
}

fn is_walkable_dir(parent: &Path, entry: &fs::DirEntry, follow_symlinks: bool) -> bool {
    let ft = match entry.file_type() {
        Ok(ft) => ft,
        Err(_) => return false,
    };
    if ft.is_dir() {
        return true;
    }
    if !follow_symlinks || !ft.is_symlink() {
        return false;
    }
    parent
        .join(entry.file_name())
        .metadata()
        .map(|m| m.is_dir())
        .unwrap_or(false)
}

fn is_readable_markdown(parent: &Path, entry: &fs::DirEntry) -> bool {
    let name = entry.file_name().to_string_lossy().to_lowercase();
    if !name.ends_with(".md") || name == "skill.md" {
        return false;
    }
    let ft = match entry.file_type() {
        Ok(ft) => ft,
        Err(_) => return false,
    };
    if ft.is_file() {
        return true;
    }
    if !ft.is_symlink() {
        return false;
    }
    parent
        .join(entry.file_name())
        .metadata()
        .map(|m| m.is_file())
        .unwrap_or(false)
}

fn collect_skill_paths(dir: &Path, depth: usize, include_md: bool, follow_symlinks: bool) -> Vec<PathBuf> {
    let skill_path = dir.join("SKILL.md");
    let is_file = if follow_symlinks {
        skill_path.is_file()
    } else {
        skill_path.symlink_metadata().map(|m| m.is_file()).unwrap_or(false)
    };
    if is_file {
        return vec![skill_path];
    }
    if depth >= 2 {
        return Vec::new();
    }
    let entries: Vec<fs::DirEntry> = match fs::read_dir(dir) {
        Ok(rd) => rd.flatten().collect(),
        Err(_) => return Vec::new(),
    };
    let mut direct_md: Vec<PathBuf> = Vec::new();
    if depth == 0 && include_md {
        let mut names: Vec<_> = entries
            .iter()
            .filter(|e| is_readable_markdown(dir, e))
            .map(|e| e.file_name())
            .collect();
        names.sort();
        direct_md = names.into_iter().map(|n| dir.join(n)).collect();
    }
    let mut subdir_names: Vec<_> = entries
        .iter()
        .filter(|e| is_walkable_dir(dir, e, follow_symlinks))
        .map(|e| e.file_name())
        .collect();
    subdir_names.sort();
    let mut out = direct_md;
    for name in subdir_names {
        out.extend(collect_skill_paths(&dir.join(&name), depth + 1, false, follow_symlinks));
    }
    out
}

fn read_skill_descriptor(path: &Path, scope: &str, namespace: Option<&str>) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    let fields = parse_frontmatter(&raw);
    let fallback = if path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase() == "skill.md")
        .unwrap_or(false)
    {
        path.parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
    } else {
        path.file_stem().map(|n| n.to_string_lossy().into_owned())
    }?;
    let unqualified = read_str(&fields, &["name"]).unwrap_or(fallback);
    let name = match namespace {
        Some(ns) if !unqualified.contains(':') => format!("{ns}:{unqualified}"),
        _ => unqualified,
    };
    if name.trim().is_empty() {
        return None;
    }
    let description = read_str(&fields, &["description"]);
    let display_name = read_str(&fields, &["display-name", "displayName", "title"]);
    let short_description = read_str(&fields, &["short-description", "shortDescription", "summary"]);
    let disabled = read_bool(&fields, &["disable-model-invocation", "disableModelInvocation"])
        .unwrap_or(false);

    let mut descriptor = json!({
        "name": name,
        "path": path.to_string_lossy(),
        "enabled": !disabled,
        "scope": scope,
    });
    if let Some(d) = description {
        descriptor["description"] = json!(d);
    }
    if display_name.is_some() || short_description.is_some() {
        let mut interface = serde_json::Map::new();
        if let Some(d) = display_name {
            interface.insert("displayName".into(), json!(d));
        }
        if let Some(s) = short_description {
            interface.insert("shortDescription".into(), json!(s));
        }
        descriptor["interface"] = Value::Object(interface);
    }
    Some(descriptor)
}

struct SkillRoot {
    path: PathBuf,
    scope: &'static str,
    include_md: bool,
    follow_symlinks: bool,
    namespace: Option<String>,
}

// ── Origin tables ────────────────────────────────────────────────────

const HOME_ORIGIN_ORDER: &[&str] = &[
    "synara", "codex", "claude", "cursor", "grok", "factory", "opencode", "pi", "devin", "omp",
    "agents",
];

fn home_roots_for_origin(
    origin: &str,
    home: &Path,
    claude_config_dir: &Path,
    synara_base: &Path,
    agent_dir: Option<&str>,
) -> Vec<PathBuf> {
    match origin {
        "synara" => vec![synara_base.join("skills")],
        "codex" => vec![home.join(".codex/skills")],
        "claude" => vec![claude_config_dir.join("skills")],
        "cursor" => vec![
            home.join(".cursor/skills-cursor"),
            home.join(".cursor/skills"),
        ],
        "grok" => vec![home.join(".grok/skills")],
        "factory" => vec![home.join(".factory/skills")],
        "opencode" => vec![home.join(".config/opencode/skills")],
        "pi" => vec![agent_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".pi/agent"))
            .join("skills")],
        "devin" => vec![
            home.join(".config/devin/skills"),
            home.join(".config/cognition/skills"),
            home.join(".codeium/windsurf/skills"),
            home.join(".codeium/windsurf-next/skills"),
            home.join(".codeium/windsurf-insiders/skills"),
            home.join(".devin/skills"),
        ],
        "omp" => vec![agent_dir
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".omp/agent"))
            .join("skills")],
        "agents" => vec![home.join(".agents/skills")],
        _ => Vec::new(),
    }
}

fn project_root_names(origin: &str) -> &'static [&'static str] {
    match origin {
        "synara" => &[".synara"],
        "codex" => &[".codex"],
        "claude" => &[".claude"],
        "cursor" => &[".cursor"],
        "grok" => &[".grok"],
        "factory" => &[".factory"],
        "opencode" => &[".opencode"],
        "pi" => &[".pi"],
        "devin" => &[".devin", ".cognition", ".windsurf"],
        "omp" => &[".omp"],
        "agents" => &[".agents"],
        _ => &[],
    }
}

fn preferred_origins(provider: &str) -> &'static [&'static str] {
    match provider {
        "codex" => &["codex", "agents"],
        "claudeAgent" => &["claude"],
        "cursor" => &["cursor", "agents", "claude", "codex"],
        "antigravity" => &["agents"],
        "grok" => &["grok", "claude", "agents"],
        "droid" => &["factory", "agents", "claude", "codex"],
        "opencode" => &["opencode", "claude", "agents"],
        "pi" => &["pi", "agents"],
        "devin" => &["devin", "claude", "agents"],
        "omp" => &["omp", "agents"],
        _ => &[],
    }
}

fn ordered_origins(provider: Option<&str>, include_synara: bool, include_remaining: bool) -> Vec<&'static str> {
    let mut ordered: Vec<&'static str> = provider
        .map(preferred_origins)
        .unwrap_or(&[])
        .to_vec();
    if include_synara && !ordered.contains(&"synara") {
        ordered.push("synara");
    }
    if !include_remaining {
        return ordered;
    }
    for origin in HOME_ORIGIN_ORDER {
        if !include_synara && *origin == "synara" {
            continue;
        }
        if !ordered.contains(origin) {
            ordered.push(origin);
        }
    }
    ordered
}

struct CatalogInput {
    cwd: Option<String>,
    provider: Option<String>,
    agent_dir: Option<String>,
    claude_config_dir: PathBuf,
}

fn roots_for(input: &CatalogInput, ordered: &[&'static str], synara_base: &Path) -> Vec<SkillRoot> {
    let home = home_dir();
    let mut home_roots: Vec<SkillRoot> = Vec::new();
    for origin in ordered {
        for path in home_roots_for_origin(
            origin,
            &home,
            &input.claude_config_dir,
            synara_base,
            input.agent_dir.as_deref(),
        ) {
            home_roots.push(SkillRoot {
                path,
                scope: *origin,
                include_md: *origin == "pi" || *origin == "omp",
                follow_symlinks: true,
                namespace: None,
            });
        }
    }
    let home_paths: HashSet<PathBuf> = home_roots
        .iter()
        .filter_map(|r| canon(&r.path).or_else(|| Some(r.path.clone())))
        .collect();

    let mut project_roots: Vec<SkillRoot> = Vec::new();
    if let Some(cwd) = input.cwd.as_ref().filter(|c| !c.trim().is_empty()) {
        for ancestor in ancestors(cwd.trim()) {
            let mut seen_names = HashSet::new();
            for origin in ordered {
                for root_name in project_root_names(origin) {
                    if !seen_names.insert(root_name) {
                        continue;
                    }
                    let root_path = ancestor.join(root_name).join("skills");
                    let resolved = canon(&root_path).unwrap_or_else(|| root_path.clone());
                    if home_paths.contains(&resolved) {
                        continue;
                    }
                    project_roots.push(SkillRoot {
                        path: root_path,
                        scope: "project",
                        include_md: *origin == "pi" || *origin == "omp",
                        follow_symlinks: true,
                        namespace: None,
                    });
                }
            }
        }
    }
    project_roots.extend(home_roots);
    project_roots
}

// ── Claude plugin skill roots ────────────────────────────────────────

fn scope_precedence(scope: &str) -> i32 {
    match scope {
        "managed" => 0,
        "local" => 1,
        "project" => 2,
        "user" => 3,
        _ => 4,
    }
}

fn path_within(parent: &Path, child: &Path) -> bool {
    child == parent || child.starts_with(parent)
}

fn claude_plugin_roots(input: &CatalogInput) -> Vec<SkillRoot> {
    let plugins_dir = input.claude_config_dir.join("plugins");
    let manifest_raw = match fs::read_to_string(plugins_dir.join("installed_plugins.json")) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let cache_root = match canon(&plugins_dir.join("cache")) {
        Some(p) => p,
        None => return Vec::new(),
    };
    let manifest: Value = match serde_json::from_str(&manifest_raw) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(plugins) = manifest.get("plugins").and_then(|p| p.as_object()) else {
        return Vec::new();
    };

    let mut entries: Vec<(String, Vec<(i32, String, String)>)> = Vec::new();
    for (plugin_id, installs) in plugins {
        let id = plugin_id.trim().to_string();
        if id.is_empty() || !id.contains('@') {
            continue;
        }
        let mut parsed: Vec<(i32, String, String)> = Vec::new();
        if let Some(arr) = installs.as_array() {
            for inst in arr {
                let install_path = inst.get("installPath").and_then(|v| v.as_str());
                let scope = inst.get("scope").and_then(|v| v.as_str());
                let project_path = inst.get("projectPath").and_then(|v| v.as_str());
                let (Some(install_path), Some(scope)) = (install_path, scope) else {
                    continue;
                };
                if install_path.trim().is_empty() || !Path::new(install_path).is_absolute() {
                    continue;
                }
                if !matches!(scope, "user" | "project" | "local" | "managed") {
                    continue;
                }
                parsed.push((
                    scope_precedence(scope),
                    scope.to_string(),
                    format!("{}\u{0}{}", project_path.unwrap_or(""), install_path),
                ));
            }
        }
        parsed.sort();
        entries.push((id, parsed));
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let cwd = input.cwd.as_ref().map(|c| c.trim().to_string());
    let mut roots = Vec::new();
    let mut seen = HashSet::new();
    let mut selected_ids = HashSet::new();
    for (plugin_id, installs) in entries {
        for (_prec, scope, key) in &installs {
            if selected_ids.contains(&plugin_id) {
                break;
            }
            let mut parts = key.split('\u{0}');
            let project_path = parts.next().unwrap_or("");
            let install_path = parts.next().unwrap_or("");
            let applies = if scope == "user" || scope == "managed" {
                true
            } else {
                match (&cwd, project_path.is_empty()) {
                    (Some(cwd), false) => {
                        match (canon(Path::new(project_path)), canon(Path::new(cwd))) {
                            (Some(pp), Some(c)) => path_within(&pp, &c),
                            _ => false,
                        }
                    }
                    _ => false,
                }
            };
            if !applies {
                continue;
            }
            selected_ids.insert(plugin_id.clone());
            let Some(canonical_install) = canon(Path::new(install_path)) else {
                break;
            };
            if !path_within(&cache_root, &canonical_install) {
                break;
            }
            let Some(skills_path) = canon(&canonical_install.join("skills")) else {
                break;
            };
            if !path_within(&canonical_install, &skills_path) || !skills_path.is_dir() {
                break;
            }
            let namespace = plugin_id
                .split('@')
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            if seen.insert(format!("{plugin_id}\u{0}{}", canonical_install.display())) {
                roots.push(SkillRoot {
                    path: skills_path,
                    scope: "claude",
                    include_md: false,
                    follow_symlinks: false,
                    namespace,
                });
            }
            break;
        }
    }
    roots
}

// ── Scanning ─────────────────────────────────────────────────────────

fn descriptors_from_roots(roots: &[SkillRoot]) -> Vec<Value> {
    let mut out = Vec::new();
    for root in roots {
        for path in collect_skill_paths(&root.path, 0, root.include_md, root.follow_symlinks) {
            if let Some(d) = read_skill_descriptor(&path, root.scope, root.namespace.as_deref()) {
                out.push(d);
            }
        }
    }
    out
}

fn skill_name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

fn dedupe(skills: Vec<Value>) -> Vec<Value> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for skill in skills {
        let key = skill
            .get("name")
            .and_then(|n| n.as_str())
            .map(skill_name_key)
            .unwrap_or_default();
        if seen.insert(key) {
            out.push(skill);
        }
    }
    out
}

fn disabled_names() -> HashSet<String> {
    crate::server_settings::current()
        .pointer("/skills/disabled")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(skill_name_key))
                .collect()
        })
        .unwrap_or_default()
}

// ── Handlers ─────────────────────────────────────────────────────────

fn catalog_input(payload: &Value) -> CatalogInput {
    let home = home_dir();
    CatalogInput {
        cwd: payload
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        provider: payload
            .get("provider")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        agent_dir: payload
            .get("agentDir")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        claude_config_dir: home.join(".claude"),
    }
}

fn list_skills_catalog(payload: &Value) -> Result<Value, WsErr> {
    let input = catalog_input(payload);
    let synara_dir = synara_skills_dir();
    let _ = fs::create_dir_all(&synara_dir);
    let ordered = ordered_origins(None, true, true);
    let mut roots = roots_for(&input, &ordered, &base_dir());
    roots.extend(claude_plugin_roots(&input));
    // includeDuplicateOrigins: every origin copy is returned.
    let skills = descriptors_from_roots(&roots);
    Ok(json!({
        "skills": skills,
        "synaraSkillsDir": synara_dir.to_string_lossy(),
    }))
}

fn list_skills(payload: &Value) -> Result<Value, WsErr> {
    let input = catalog_input(payload);
    let _ = fs::create_dir_all(synara_skills_dir());
    let ordered = ordered_origins(input.provider.as_deref(), true, true);
    let mut roots = roots_for(&input, &ordered, &base_dir());
    roots.extend(claude_plugin_roots(&input));
    let merged = dedupe(descriptors_from_roots(&roots));
    let disabled = disabled_names();
    let skills: Vec<Value> = merged
        .into_iter()
        .filter(|s| {
            s.get("name")
                .and_then(|n| n.as_str())
                .map(|n| !disabled.contains(&skill_name_key(n)))
                .unwrap_or(true)
        })
        .collect();
    Ok(json!({
        "skills": skills,
        "source": "synara.catalog",
        "cached": false,
    }))
}

// ── Composer capabilities ────────────────────────────────────────────

fn provider_enabled(payload: &Value) -> Result<bool, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("provider is required"))?;
    let settings = crate::server_settings::current();
    if let Some(instance_id) = payload.get("instanceId").and_then(|v| v.as_str()) {
        let inst = settings
            .pointer(&format!("/providerInstances/{instance_id}"))
            .cloned()
            .or_else(|| {
                settings
                    .get("providerInstances")
                    .and_then(|m| m.get(instance_id))
                    .cloned()
            });
        let Some(inst) = inst else {
            return Err(err(format!("Unknown provider instance '{instance_id}'.")));
        };
        let driver = inst.get("driver").and_then(|v| v.as_str()).unwrap_or(provider);
        if driver != provider {
            return Err(err(format!(
                "Requested provider '{provider}' does not match provider instance '{instance_id}' driver '{driver}'."
            )));
        }
        let enabled = inst.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)
            && inst
                .get("config")
                .and_then(|c| c.get("enabled"))
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
        return Ok(enabled);
    }
    let provider_cfg = settings
        .get("providers")
        .and_then(|p| p.get(provider));
    let enabled = provider_cfg
        .and_then(|c| c.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    // An explicit providerInstances entry keyed by the default instance id
    // (the provider name) merges over the built-in config for that driver.
    if let Some(inst) = settings
        .get("providerInstances")
        .and_then(|m| m.get(provider))
    {
        let driver = inst.get("driver").and_then(|v| v.as_str()).unwrap_or(provider);
        if driver == provider {
            let e = inst.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)
                && inst
                    .get("config")
                    .and_then(|c| c.get("enabled"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);
            return Ok(e);
        }
    }
    Ok(enabled)
}

fn composer_capabilities(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("provider is required"))?
        .to_string();
    let enabled = provider_enabled(payload)?;
    // (nativeSlash, pluginMentions, pluginDiscovery, runtimeModelList,
    //  threadCompaction, threadImport) per adapter; skill mentions/discovery are
    // always true for enabled providers because the Synara catalog backs them.
    let caps = match provider.as_str() {
        "codex" => (false, true, true, true, true, true),
        "claudeAgent" => (true, false, false, true, false, true),
        "cursor" => (false, false, false, true, false, true),
        "antigravity" => (false, false, false, true, false, false),
        "grok" => (false, false, false, true, true, false),
        "droid" => (true, true, true, true, false, true),
        "opencode" => (true, false, false, true, true, true),
        "pi" => (true, false, false, true, true, false),
        "devin" => (true, false, false, true, true, false),
        "omp" => (true, false, false, true, false, true),
        _ => (false, false, false, false, false, false),
    };
    Ok(json!({
        "provider": provider,
        "supportsSkillMentions": enabled,
        "supportsSkillDiscovery": enabled,
        "supportsNativeSlashCommandDiscovery": enabled && caps.0,
        "supportsPluginMentions": enabled && caps.1,
        "supportsPluginDiscovery": enabled && caps.2,
        "supportsRuntimeModelList": enabled && caps.3,
        "supportsThreadCompaction": enabled && caps.4,
        "supportsThreadImport": enabled && caps.5,
    }))
}

// ── Factory (droid) plugin marketplaces ──────────────────────────────
// Mirrors FactoryPluginDiscovery.ts: `~/.factory/plugins/known_marketplaces.json`
// registers install locations; each marketplace has
// `.factory-plugin/marketplace.json` listing plugins, each plugin dir has
// `.factory-plugin/plugin.json`. Enabled state comes from layered settings
// files' `enabledPlugins` maps.

fn read_json_record(path: &Path) -> Option<serde_json::Map<String, Value>> {
    let v: Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    v.as_object().cloned()
}

fn str_field(v: Option<&Value>) -> Option<String> {
    v.and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn factory_enabled_plugins(factory_dir: &Path, cwd: Option<&str>) -> std::collections::HashMap<String, bool> {
    let mut enabled = std::collections::HashMap::new();
    let mut sources: Vec<Option<Value>> = Vec::new();
    if let Some(flags) = read_json_record(&factory_dir.join("cache/feature-flags.json")) {
        let cli_defaults = flags
            .get("configs")
            .and_then(|c| c.get("cli_default_settings"))
            .and_then(|d| d.get("enabledPlugins"))
            .cloned();
        sources.push(cli_defaults);
    }
    sources.push(
        read_json_record(&factory_dir.join("settings.json")).and_then(|r| r.get("enabledPlugins").cloned()),
    );
    sources.push(
        read_json_record(&factory_dir.join("settings.local.json")).and_then(|r| r.get("enabledPlugins").cloned()),
    );
    if let Some(cwd) = cwd {
        let base = Path::new(cwd).join(".factory");
        sources.push(
            read_json_record(&base.join("settings.json")).and_then(|r| r.get("enabledPlugins").cloned()),
        );
        sources.push(
            read_json_record(&base.join("settings.local.json")).and_then(|r| r.get("enabledPlugins").cloned()),
        );
    }
    for source in sources.into_iter().flatten() {
        if let Some(map) = source.as_object() {
            for (id, value) in map {
                if let Some(b) = value.as_bool() {
                    enabled.insert(id.clone(), b);
                }
            }
        }
    }
    enabled
}

fn marketplace_registrations(factory_dir: &Path) -> Vec<(String, PathBuf)> {
    let Some(known) = read_json_record(&factory_dir.join("plugins/known_marketplaces.json")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (name, raw) in &known {
        if let Some(path) = str_field(raw.get("installLocation")) {
            out.push((name.clone(), factory_dir.join(path)));
        }
    }
    out
}

fn resolve_plugin_path(marketplace: &Path, source: &str) -> Option<PathBuf> {
    let resolved = marketplace.join(source);
    let normalized = resolved.canonicalize().unwrap_or(resolved);
    if path_within(marketplace, &normalized) {
        Some(normalized)
    } else {
        None
    }
}

fn plugin_descriptor(
    marketplace_name: &str,
    marketplace_path: &Path,
    entry: &Value,
    enabled: &std::collections::HashMap<String, bool>,
) -> Option<Value> {
    let name = str_field(entry.get("name"))?;
    let source = str_field(entry.get("source"))?;
    let path = resolve_plugin_path(marketplace_path, &source)?;
    let manifest = read_json_record(&path.join(".factory-plugin/plugin.json"));
    let id = format!("{name}@{marketplace_name}");
    let installed = enabled.contains_key(&id);
    let description = manifest
        .as_ref()
        .and_then(|m| str_field(m.get("description")))
        .or_else(|| str_field(entry.get("description")));
    let developer_name = manifest
        .as_ref()
        .and_then(|m| m.get("author"))
        .and_then(|a| str_field(a.get("name")));
    let category = str_field(entry.get("category"));
    let display = manifest
        .as_ref()
        .and_then(|m| str_field(m.get("name")))
        .unwrap_or_else(|| name.clone());
    let mut interface = json!({"displayName": display});
    if let Some(d) = description.clone() {
        interface["shortDescription"] = json!(d);
    }
    if let Some(d) = developer_name {
        interface["developerName"] = json!(d);
    }
    if let Some(c) = category {
        interface["category"] = json!(c);
    }
    Some(json!({
        "id": id,
        "name": name,
        "source": {"type": "local", "path": path.to_string_lossy()},
        "installed": installed,
        "enabled": enabled.get(&format!("{name}@{marketplace_name}")).copied().unwrap_or(false),
        "installPolicy": if installed { "INSTALLED_BY_DEFAULT" } else { "AVAILABLE" },
        "authPolicy": "ON_USE",
        "interface": interface,
    }))
}

fn factory_dir() -> PathBuf {
    home_dir().join(".factory")
}

fn list_factory_plugins(cwd: Option<&str>) -> Value {
    let factory = factory_dir();
    let registrations = marketplace_registrations(&factory);
    let enabled = factory_enabled_plugins(&factory, cwd);
    let mut errors = Vec::new();
    let mut marketplaces = Vec::new();
    for (name, path) in registrations {
        let manifest = read_json_record(&path.join(".factory-plugin/marketplace.json"));
        let Some(manifest) = manifest else {
            errors.push(json!({
                "marketplacePath": path.to_string_lossy(),
                "message": "Factory marketplace manifest is missing or invalid.",
            }));
            continue;
        };
        let plugins: Vec<Value> = manifest
            .get("plugins")
            .and_then(|p| p.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|e| plugin_descriptor(&name, &path, e, &enabled))
                    .collect()
            })
            .unwrap_or_default();
        let display = str_field(manifest.get("name")).unwrap_or_else(|| name.clone());
        marketplaces.push(json!({
            "name": name,
            "path": path.to_string_lossy(),
            "interface": {"displayName": display},
            "plugins": plugins,
        }));
    }
    json!({
        "marketplaces": marketplaces,
        "marketplaceLoadErrors": errors,
        "remoteSyncError": Value::Null,
        "featuredPluginIds": [],
        "source": "factory-local-marketplaces",
        "cached": false,
    })
}

fn read_factory_plugin(marketplace_path: &str, plugin_name: &str, cwd: Option<&str>) -> Option<Value> {
    let factory = factory_dir();
    let resolved = canon(Path::new(marketplace_path)).unwrap_or_else(|| PathBuf::from(marketplace_path));
    let (mp_name, mp_path) = marketplace_registrations(&factory)
        .into_iter()
        .find(|(_, p)| canon(p).unwrap_or_else(|| p.clone()) == resolved)?;
    let manifest = read_json_record(&mp_path.join(".factory-plugin/marketplace.json"))?;
    let entry = manifest
        .get("plugins")?
        .as_array()?
        .iter()
        .find(|p| str_field(p.get("name")).as_deref() == Some(plugin_name))?
        .clone();
    let source = str_field(entry.get("source"))?;
    let path = resolve_plugin_path(&mp_path, &source)?;
    let enabled = factory_enabled_plugins(&factory, cwd);
    let summary = plugin_descriptor(&mp_name, &mp_path, &entry, &enabled)?;
    let plugin_manifest = read_json_record(&path.join(".factory-plugin/plugin.json"));
    let description = plugin_manifest
        .as_ref()
        .and_then(|m| str_field(m.get("description")))
        .or_else(|| str_field(entry.get("description")));
    let root = SkillRoot {
        path: path.join("skills"),
        scope: "project",
        include_md: false,
        follow_symlinks: true,
        namespace: None,
    };
    // Scope label is provider-specific ("factory-plugin:<name>"); patch after scan.
    let skills: Vec<Value> = descriptors_from_roots(&[root])
        .into_iter()
        .map(|mut s| {
            s["scope"] = json!(format!("factory-plugin:{plugin_name}"));
            s
        })
        .collect();
    let mut plugin = json!({
        "marketplaceName": mp_name,
        "marketplacePath": mp_path.to_string_lossy(),
        "summary": summary,
        "skills": skills,
        "apps": [],
        "mcpServers": [],
    });
    if let Some(d) = description {
        plugin["description"] = json!(d);
    }
    Some(json!({
        "plugin": plugin,
        "source": "factory-local-marketplace",
        "cached": false,
    }))
}

fn list_plugins(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("provider is required"))?;
    let empty = |source: &str| {
        json!({
            "marketplaces": [],
            "marketplaceLoadErrors": [],
            "remoteSyncError": Value::Null,
            "featuredPluginIds": [],
            "source": source,
            "cached": false,
        })
    };
    if !provider_enabled(payload)? {
        return Ok(empty("disabled"));
    }
    if provider != "droid" {
        return Ok(empty("unsupported"));
    }
    Ok(list_factory_plugins(
        payload.get("cwd").and_then(|v| v.as_str()),
    ))
}

fn read_plugin(payload: &Value) -> Result<Value, WsErr> {
    let provider = payload
        .get("provider")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("provider is required"))?;
    if !provider_enabled(payload)? {
        return Err(err(format!(
            "Provider instance '{}' is disabled in Synara settings.",
            payload
                .get("instanceId")
                .and_then(|v| v.as_str())
                .unwrap_or(provider)
        )));
    }
    if provider != "droid" {
        return Err(err(format!(
            "Plugin discovery is unavailable for provider '{provider}'."
        )));
    }
    let marketplace_path = payload
        .get("marketplacePath")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("marketplacePath is required"))?;
    let plugin_name = payload
        .get("pluginName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| err("pluginName is required"))?;
    read_factory_plugin(
        marketplace_path,
        plugin_name,
        payload.get("cwd").and_then(|v| v.as_str()),
    )
    .ok_or_else(|| err("Failed to read the Factory plugin."))
}
