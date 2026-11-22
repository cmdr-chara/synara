// orchestration.listProjectImports — native port of
// apps/server/src/orchestration/projectImportRoute.ts (list path),
// provider/claudeProjectImport.ts and provider/codexProjectImport.ts.
// Discovery is disk-only: codex reads its state sqlite + desktop state json;
// claude reads ~/.claude/projects/*/<uuid>.jsonl transcripts.

use rusqlite::Connection;
use serde_json::{json, Value};
use sha2::Digest;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::todo::WsErr;

fn err(msg: impl Into<String>) -> WsErr {
    WsErr(msg.into(), None)
}

pub fn handles(tag: &str) -> bool {
    tag == "orchestration.listProjectImports"
        || (tag == "orchestration.importProject"
            && std::env::var("SYNARA_NO_UPSTREAM").is_ok())
}

pub fn handle(tag: &str, payload: &Value) -> Result<Value, WsErr> {
    match tag {
        "orchestration.listProjectImports" => list_project_imports(payload),
        "orchestration.importProject" => import_project(payload),
        _ => Err(err("unhandled project import tag")),
    }
}

// ── Path identity helpers (projectImportPaths.ts + threadWorkspace.ts) ──

fn is_abs_dir(v: &str) -> bool {
    !v.trim().is_empty() && !v.contains('\0') && Path::new(v.trim()).is_absolute()
}

fn canonical_import_path(value: &str) -> String {
    let absolute = normalize_windows(value);
    let absolute = if Path::new(&absolute).is_absolute() {
        absolute
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(&absolute)
            .to_string_lossy()
            .into_owned()
    };
    match std::fs::canonicalize(&absolute) {
        Ok(p) => normalize_windows(&p.to_string_lossy()),
        Err(_) => {
            let parent = Path::new(&absolute)
                .parent()
                .map(|p| p.to_string_lossy().into_owned());
            match parent {
                Some(p) if p != absolute => {
                    let base = Path::new(&absolute)
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    format!("{}/{}", canonical_import_path(&p).trim_end_matches('/'), base)
                }
                _ => absolute,
            }
        }
    }
}

fn normalize_windows(v: &str) -> String {
    // no-op on non-Windows; keeps call sites parallel to upstream.
    v.to_string()
}

fn path_identity(v: &str) -> String {
    // normalizeWorkspaceRootForComparison with platform "darwin".
    let trimmed = v.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd = trimmed.replace('\\', "/");
    let is_drive_root = fwd.len() >= 3
        && fwd.as_bytes()[1] == b':'
        && fwd.as_bytes()[0].is_ascii_alphabetic()
        && fwd[2..].chars().all(|c| c == '/');
    let prefix = if fwd.starts_with("//") {
        "//"
    } else if fwd.starts_with('/') {
        "/"
    } else {
        ""
    };
    let mut body = fwd[prefix.len()..].to_string();
    while body.contains("//") {
        body = body.replace("//", "/");
    }
    let mut normalized = format!(
        "{}{}",
        prefix,
        body.trim_end_matches('/')
    );
    if is_drive_root {
        normalized = format!("{}/", &fwd[..2]);
    } else if normalized.is_empty() {
        normalized = prefix.to_string();
    }
    // macOS /private alias.
    if let Some(rest) = normalized
        .strip_prefix("/private/var/")
        .or_else(|| normalized.strip_prefix("/private/tmp/"))
    {
        normalized = format!("/{rest}");
        normalized = match normalized.strip_prefix("//") {
            Some(r) if !normalized.starts_with("/private") => format!("/{}", r),
            _ => normalized,
        };
    }
    normalized
}

fn root_within(candidate: &str, ancestor: &str) -> bool {
    let c = path_identity(candidate);
    let a = path_identity(ancestor);
    if c.is_empty() || a.is_empty() {
        return false;
    }
    if c == a {
        return true;
    }
    let prefix = if a.ends_with('/') {
        a
    } else {
        format!("{a}/")
    };
    c.starts_with(&prefix)
}

fn most_specific_root<'a>(roots: impl Iterator<Item = &'a str>, cwd: &str) -> Option<&'a str> {
    roots
        .filter(|r| root_within(cwd, r))
        .max_by_key(|r| path_identity(r).len())
}

fn project_key(parts: &[&str]) -> String {
    let arr = Value::Array(parts.iter().map(|p| json!(p)).collect());
    let bytes = serde_json::to_vec(&arr).unwrap_or_default();
    let mut h = sha2::Sha256::new();
    h.update(&bytes);
    format!("{:x}", h.finalize())
}

fn dir_exists(p: &str) -> bool {
    Path::new(p).is_dir()
}

fn find_git_workspace(cwd: &str) -> Option<(String, Option<String>)> {
    let mut current = cwd.to_string();
    loop {
        let git_path = Path::new(&current).join(".git");
        if let Ok(md) = std::fs::metadata(&git_path) {
            if md.is_dir() {
                return Some((current.clone(), None));
            }
            if md.is_file() {
                let pointer = if md.len() <= 8192 {
                    std::fs::read_to_string(&git_path).unwrap_or_default()
                } else {
                    String::new()
                };
                if let Some(root) = parse_worktree_root(&pointer, &current) {
                    return Some((canonical_import_path(&root), Some(current.clone())));
                }
                return Some((current.clone(), None));
            }
        }
        let parent = match Path::new(&current).parent() {
            Some(p) if p.as_os_str() != Path::new(&current).as_os_str() => {
                p.to_string_lossy().into_owned()
            }
            _ => return None,
        };
        current = parent;
    }
}

fn parse_worktree_root(contents: &str, worktree_path: &str) -> Option<String> {
    let first = contents.lines().next()?.trim().to_string();
    if !first.to_lowercase().starts_with("gitdir:") {
        return None;
    }
    let value = first["gitdir:".len()..].trim();
    if value.is_empty() {
        return None;
    }
    let resolved = if Path::new(value).is_absolute() {
        value.to_string()
    } else {
        Path::new(worktree_path)
            .join(value)
            .to_string_lossy()
            .into_owned()
    };
    let normalized = resolved.replace('\\', "/");
    let marker = "/.git/worktrees/";
    normalized
        .rfind(marker)
        .map(|i| normalized[..i].to_string())
}

fn basename(p: &str) -> String {
    Path::new(p.trim_end_matches('/'))
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string())
}

fn expand_home(p: &str, home: &str) -> String {
    if let Some(rest) = p.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else if p == "~" {
        home.to_string()
    } else {
        p.to_string()
    }
}

fn home_dir() -> String {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/".to_string())
}

fn iso_from_f64_ms(ms: f64) -> Option<String> {
    if !ms.is_finite() {
        return None;
    }
    Some(crate::todo::iso_from_ms(ms as i64))
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(|s| s.to_string()).filter(|s| !s.is_empty())
}

// ── Sources (projectImportSources.ts) ────────────────────────────────

struct Source {
    provider: &'static str,
    instance_id: String,
    is_default: bool,
    account_label: Option<String>,
    codex_home: Option<String>,
    claude_config_dir: Option<String>,
    environment: HashMap<String, String>,
}

fn sources(providers: &[&str]) -> Vec<Source> {
    let settings = crate::server_settings::current();
    let instances = settings.get("providerInstances").cloned().unwrap_or(json!({}));
    let mut out: Vec<Source> = Vec::new();
    for provider in providers {
        let default_id = provider.to_string();
        // instanceId -> (config, enabled, display_name)
        let mut merged: BTreeMap<String, (Value, bool, Option<String>)> = BTreeMap::new();
        let prov_cfg = settings
            .get("providers")
            .and_then(|p| p.get(*provider))
            .cloned()
            .unwrap_or(json!({}));
        let default_enabled = prov_cfg
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        merged.insert(default_id.clone(), (prov_cfg, default_enabled, None));
        if let Some(map) = instances.as_object() {
            for (id, inst) in map {
                let driver = inst
                    .get("driver")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id.as_str());
                if driver != *provider {
                    continue;
                }
                if id == &default_id {
                    // Merges over the built-in default config.
                    let mut cfg = merged[&default_id].0.clone();
                    if let Some(ex) = inst.get("config") {
                        if cfg.is_object() && ex.is_object() {
                            for (k, v) in ex.as_object().unwrap() {
                                cfg[k] = v.clone();
                            }
                        }
                    }
                    let en = inst.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)
                        && cfg
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                    merged.insert(default_id.clone(), (cfg, en, None));
                    continue;
                }
                let cfg = inst.get("config").cloned().unwrap_or(json!({}));
                let en = inst.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true)
                    && cfg
                        .get("enabled")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                merged.insert(id.clone(), (cfg, en, inst.get("displayName").and_then(|v| text(v))));
            }
        }
        for (id, (cfg, enabled, display)) in merged {
            let is_default = id == default_id;
            if !enabled && !is_default {
                continue;
            }
            let environment: HashMap<String, String> = cfg
                .get("environment")
                .and_then(|e| e.as_object())
                .map(|m| {
                    m.iter()
                        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                        .collect()
                })
                .unwrap_or_default();
            let label = if is_default { None } else { display.or(Some(id.clone())) };
            if *provider == "codex" {
                let home = cfg
                    .get("homePath")
                    .and_then(|v| text(v))
                    .or_else(|| cfg.get("profileDir").and_then(|v| text(v)))
                    .map(|h| expand_home(&h, &home_dir()));
                out.push(Source {
                    provider: "codex",
                    instance_id: id,
                    is_default,
                    account_label: label,
                    codex_home: home,
                    claude_config_dir: None,
                    environment,
                });
            } else {
                let mut config_dir = cfg
                    .get("configDir")
                    .and_then(|v| text(v))
                    .or_else(|| cfg.get("profileDir").and_then(|v| text(v)))
                    .map(|d| expand_home(&d, &home_dir()));
                if config_dir.is_none() {
                    if let Some(env_dir) = environment.get("CLAUDE_CONFIG_DIR") {
                        config_dir = Some(expand_home(env_dir, &home_dir()));
                    }
                }
                if config_dir.is_none() {
                    if let Some(home) = cfg
                        .get("homePath")
                        .and_then(|v| text(v))
                        .or_else(|| environment.get("HOME").cloned())
                    {
                        let h = expand_home(&home, &home_dir());
                        config_dir = Some(format!("{}/.claude", h.trim_end_matches('/')));
                    }
                }
                if config_dir.is_none() && !is_default {
                    // claudeIsolatedHomePath(stateDir, instanceId):
                    // <state>/provider-homes/claude/instance-<hex(id)>/.claude
                    let scope = format!("instance-{}", hex_encode(id.as_bytes()));
                    config_dir = Some(format!(
                        "{}/provider-homes/claude/{}/.claude",
                        crate::todo::state_dir().trim_end_matches('/'),
                        scope
                    ));
                }
                out.push(Source {
                    provider: "claudeAgent",
                    instance_id: id,
                    is_default,
                    account_label: label,
                    codex_home: None,
                    claude_config_dir: config_dir,
                    environment,
                });
            }
        }
    }
    out
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ── Native catalog types ─────────────────────────────────────────────

struct NativeProject {
    id: String,
    title: String,
    roots: Vec<String>,
}

struct NativeSession {
    id: String,
    title: String,
    cwd: String,
    project_id: Option<String>,
    created_at: String,
    updated_at: String,
    archived: bool,
}

struct Catalog {
    source_home: String,
    projects: Vec<NativeProject>,
    sessions: Vec<NativeSession>,
}

// ── Codex discovery (codexProjectImport.ts) ──────────────────────────

fn discover_codex(src: &Source) -> Result<Catalog, String> {
    let base = src
        .codex_home
        .clone()
        .or_else(|| src.environment.get("CODEX_HOME").cloned())
        .map(|h| expand_home(&h, &home_dir()))
        .unwrap_or_else(|| format!("{}/.codex", home_dir()));
    let home = canonical_import_path(&base);
    // Provenance identity: the state file may be an overlay symlink.
    let global_state_path = format!("{}/.codex-global-state.json", home.trim_end_matches('/'));
    let source_home = match std::fs::canonicalize(&global_state_path) {
        Ok(p) => p
            .parent()
            .map(|d| d.to_string_lossy().into_owned())
            .unwrap_or_else(|| home.clone()),
        Err(_) => home.clone(),
    };
    let state: Value = std::fs::read_to_string(&global_state_path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(json!({}));
    let sqlite_home = src
        .environment
        .get("CODEX_SQLITE_HOME")
        .map(|h| expand_home(h, &home_dir()))
        .map(|h| canonical_import_path(&h))
        .unwrap_or_else(|| canonical_import_path(&source_home));
    let db_path = std::fs::read_dir(&sqlite_home)
        .ok()
        .into_iter()
        .flat_map(|it| it.flatten())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let v = name
                .strip_prefix("state_")?
                .strip_suffix(".sqlite")?
                .parse::<u64>()
                .ok()?;
            Some((v, e.path()))
        })
        .max_by_key(|(v, _)| *v)
        .map(|(_, p)| p);
    let conn = db_path.and_then(|p| Connection::open_with_flags(p, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok());
    let tables: HashSet<String> = conn
        .as_ref()
        .map(|c| {
            let mut s = c
                .prepare("SELECT name FROM sqlite_master WHERE type='table'")
                .unwrap();
            s.query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .flatten()
                .collect()
        })
        .unwrap_or_default();

    let mut projects: Vec<NativeProject> = Vec::new();
    if let Some(c) = &conn {
        if tables.contains("projects") && tables.contains("project_roots") {
            require_cols(c, "projects", &["id", "name"])?;
            require_cols(c, "project_roots", &["project_id", "path", "position"])?;
            let roots: Vec<(String, String)> = c
                .prepare("SELECT project_id, path FROM project_roots ORDER BY position")
                .map_err(|e| e.to_string())?
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
                .map_err(|e| e.to_string())?
                .flatten()
                .collect();
            let mut stmt = c
                .prepare("SELECT id, name FROM projects")
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| Ok((r.get::<_, String>(0), r.get::<_, String>(1))))
                .map_err(|e| e.to_string())?;
            for row in rows.flatten() {
                let (Ok(id), name) = row else { continue };
                projects.push(NativeProject {
                    title: name.ok().unwrap_or_else(|| id.clone()),
                    roots: roots
                        .iter()
                        .filter(|(pid, _)| pid == &id)
                        .map(|(_, p)| p.clone())
                        .collect(),
                    id,
                });
            }
        }
    }
    // mergeLegacyProjects
    let mut by_id: BTreeMap<String, NativeProject> =
        projects.iter().map(|p| (p.id.clone(), NativeProject {
            id: p.id.clone(),
            title: p.title.clone(),
            roots: p.roots.clone(),
        })).collect();
    let mut legacy_ids: HashMap<String, String> = HashMap::new();
    let mappings = state
        .get("app-server-project-id-by-legacy-project-id-by-host")
        .cloned()
        .unwrap_or(json!({}));
    let host_mappings: Vec<Value> = [&source_home, &home, &base, &sqlite_home]
        .iter()
        .map(|h| {
            canonical_import_path(h);
            mappings
                .get(&format!("local:{h}"))
                .cloned()
                .unwrap_or(json!({}))
        })
        .collect();
    if let Some(local) = state.get("local-projects").and_then(|v| v.as_object()) {
        for (key, value) in local {
            let legacy_id = value.get("id").and_then(|v| text(v)).unwrap_or_else(|| key.clone());
            let mapped = host_mappings
                .iter()
                .filter_map(|m| m.get(&legacy_id).and_then(|v| text(v)))
                .find(|id| by_id.contains_key(id));
            let id = mapped.unwrap_or_else(|| legacy_id.clone());
            legacy_ids.insert(legacy_id, id.clone());
            if by_id.contains_key(&id) {
                continue;
            }
            let roots: Vec<String> = value
                .get("rootPaths")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|r| text(r)).collect())
                .unwrap_or_default();
            let title = value
                .get("name")
                .and_then(|v| text(v))
                .unwrap_or_else(|| basename(roots.first().map(|s| s.as_str()).unwrap_or(&id)));
            by_id.insert(id.clone(), NativeProject { id, title, roots });
        }
    }
    let projects: Vec<NativeProject> = by_id.into_values().collect();

    let mut sessions: Vec<NativeSession> = Vec::new();
    if let Some(c) = &conn {
        if tables.contains("threads") {
            require_cols(c, "threads", &["id", "cwd", "source", "created_at", "updated_at"])?;
            let cols: HashSet<String> = c
                .prepare("PRAGMA table_info(threads)")
                .map_err(|e| e.to_string())?
                .query_map([], |r| r.get::<_, String>(1))
                .map_err(|e| e.to_string())?
                .flatten()
                .collect();
            let wanted = [
                "id", "cwd", "source", "created_at", "updated_at", "created_at_ms",
                "updated_at_ms", "name", "title", "project_id", "archived", "thread_source",
                "parent_thread_id",
            ];
            let selected: Vec<&str> = wanted.iter().filter(|c2| cols.contains(**c2)).copied().collect();
            let children: HashSet<String> = if tables.contains("thread_spawn_edges") {
                c.prepare("SELECT child_thread_id FROM thread_spawn_edges")
                    .map_err(|e| e.to_string())?
                    .query_map([], |r| r.get::<_, String>(0))
                    .map_err(|e| e.to_string())?
                    .flatten()
                    .collect()
            } else {
                HashSet::new()
            };
            let project_ids: HashSet<String> = projects.iter().map(|p| p.id.clone()).collect();
            let assignments = state
                .get("thread-project-assignments")
                .cloned()
                .unwrap_or(json!({}));
            let projectless: HashSet<String> = state
                .get("projectless-thread-ids")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| text(x)).collect())
                .unwrap_or_default();
            let sql = format!("SELECT {} FROM threads", selected.join(", "));
            let mut stmt = c.prepare(&sql).map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(selected
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            r.get::<_, rusqlite::types::Value>(i)
                                .map(sql_to_json)
                                .unwrap_or(Value::Null)
                        })
                        .collect::<Vec<_>>())
                })
                .map_err(|e| e.to_string())?;
            for row in rows.flatten() {
                let get = |name: &str| -> Value {
                    selected
                        .iter()
                        .position(|c2| *c2 == name)
                        .and_then(|i| row.get(i).cloned())
                        .unwrap_or(Value::Null)
                };
                let id = match text(&get("id")) { Some(v) => v, None => continue };
                let cwd = match text(&get("cwd")) { Some(v) => v, None => continue };
                if children.contains(&id) || text(&get("parent_thread_id")).is_some() {
                    continue;
                }
                let source_s = text(&get("source")).unwrap_or_default();
                let thread_source = text(&get("thread_source")).unwrap_or_default();
                let lower = |s: &str| s.to_lowercase();
                if lower(&source_s).starts_with("subagent")
                    || lower(&source_s).starts_with("sub_agent")
                    || lower(&thread_source).starts_with("subagent")
                    || lower(&thread_source).starts_with("sub_agent")
                    || lower(&thread_source).starts_with("guardian")
                {
                    continue;
                }
                if source_s.starts_with('{') {
                    if serde_json::from_str::<Value>(&source_s)
                        .ok()
                        .and_then(|v| v.get("subagent").cloned())
                        .is_some()
                    {
                        continue;
                    }
                }
                let legacy_id = if !projectless.contains(&id)
                    && assignments
                        .get(&id)
                        .and_then(|a| a.get("projectKind"))
                        .and_then(|v| v.as_str())
                        == Some("local")
                {
                    assignments.get(&id).and_then(|a| a.get("projectId")).and_then(|v| text(v))
                } else {
                    None
                };
                let candidate = text(&get("project_id"))
                    .or_else(|| legacy_id.and_then(|l| legacy_ids.get(&l).cloned()));
                let ms = get("created_at_ms").as_f64();
                let sec = get("created_at").as_f64();
                let created_at = ms
                    .or_else(|| sec.map(|s| s * 1000.0))
                    .and_then(iso_from_f64_ms)
                    .ok_or("Codex thread metadata has an unsupported timestamp.")?;
                let updated_at = get("updated_at_ms")
                    .as_f64()
                    .or_else(|| get("updated_at").as_f64().map(|s| s * 1000.0))
                    .and_then(iso_from_f64_ms)
                    .ok_or("Codex thread metadata has an unsupported timestamp.")?;
                sessions.push(NativeSession {
                    title: text(&get("name")).or_else(|| text(&get("title"))).unwrap_or_else(|| id.clone()),
                    cwd,
                    project_id: candidate.filter(|c2| project_ids.contains(c2)),
                    created_at,
                    updated_at,
                    archived: get("archived").as_i64() == Some(1)
                        || get("archived").as_bool() == Some(true),
                    id,
                });
            }
        }
    }
    Ok(Catalog {
        source_home,
        projects,
        sessions,
    })
}

fn require_cols(conn: &Connection, table: &str, required: &[&str]) -> Result<(), String> {
    let cols: HashSet<String> = conn
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(|e| e.to_string())?
        .query_map([], |r| r.get::<_, String>(1))
        .map_err(|e| e.to_string())?
        .flatten()
        .collect();
    if required.iter().any(|c| !cols.contains(*c)) {
        return Err(format!("Unsupported Codex {table} metadata schema."));
    }
    Ok(())
}

// ── Claude discovery (claudeProjectImport.ts, SDK listSessions reimpl) ──

fn discover_claude(src: &Source) -> Result<Catalog, String> {
    let config_dir = src
        .claude_config_dir
        .clone()
        .or_else(|| {
            src.environment
                .get("CLAUDE_CONFIG_DIR")
                .map(|d| expand_home(d, &home_dir()))
        })
        .or_else(|| std::env::var("CLAUDE_CONFIG_DIR").ok())
        .unwrap_or_else(|| format!("{}/.claude", home_dir()));
    let source_home = canonical_import_path(&config_dir);
    let projects_dir = PathBuf::from(&source_home).join("projects");
    // session id -> file path (newest mtime wins on collision)
    let mut files: HashMap<String, PathBuf> = HashMap::new();
    let uuid_re = |name: &str| -> Option<String> {
        let stem = name.strip_suffix(".jsonl")?;
        let b = stem.as_bytes();
        let ok = b.len() == 36
            && b.iter().enumerate().all(|(i, c)| {
                if [8, 13, 18, 23].contains(&i) {
                    *c == b'-'
                } else {
                    c.is_ascii_hexdigit()
                }
            });
        ok.then(|| stem.to_string())
    };
    if let Ok(projects) = std::fs::read_dir(&projects_dir) {
        for project in projects.flatten() {
            if !project.path().is_dir() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(project.path()) {
                for file in entries.flatten() {
                    let name = file.file_name().to_string_lossy().into_owned();
                    let Some(id) = uuid_re(&name) else { continue };
                    if !file.path().is_file() {
                        continue;
                    }
                    if let Some(prev) = files.get(&id) {
                        let pm = std::fs::metadata(prev).and_then(|m| m.modified()).ok();
                        let cm = file.metadata().and_then(|m| m.modified()).ok();
                        if pm >= cm {
                            continue;
                        }
                    }
                    files.insert(id, file.path());
                }
            }
        }
    }
    if files.is_empty() {
        return Ok(Catalog {
            source_home,
            projects: Vec::new(),
            sessions: Vec::new(),
        });
    }
    let mut sessions: Vec<NativeSession> = Vec::new();
    let mut project_ids: BTreeSet<String> = BTreeSet::new();
    for (id, file) in &files {
        // Metadata: first non-sidechain cwd + timestamp; summary title.
        let (mut cwd, mut created_at, mut summary) = (None, None, None);
        let mut read = 0usize;
        if let Ok(content) = std::fs::read(&file) {
            read = content.len();
            let cap = content.len().min(32 * 1024 * 1024);
            for line in content[..cap].split(|b| *b == b'\n') {
                if line.len() > 8 * 1024 * 1024 {
                    continue;
                }
                let Ok(entry) = serde_json::from_slice::<Value>(line) else { continue };
                if entry.get("type").and_then(|v| v.as_str()) == Some("summary") {
                    if summary.is_none() {
                        summary = entry.get("summary").and_then(|v| text(v));
                    }
                    continue;
                }
                if entry.get("isSidechain").and_then(|v| v.as_bool()) == Some(true) {
                    continue;
                }
                if cwd.is_none() {
                    cwd = entry
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .filter(|c| Path::new(c).is_absolute())
                        .map(|c| c.to_string());
                }
                if created_at.is_none() {
                    created_at = entry
                        .get("timestamp")
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| v.as_f64().and_then(iso_from_f64_ms)));
                }
                if cwd.is_some() && created_at.is_some() && summary.is_some() {
                    break;
                }
            }
        }
        let Some(cwd) = cwd else { continue };
        let mtime = file
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| crate::todo::iso_from_ms(d.as_millis() as i64));
        let updated_at = mtime.or(created_at.clone());
        let Some(updated_at) = updated_at else { continue };
        let created_at = created_at.unwrap_or_else(|| updated_at.clone());
        project_ids.insert(cwd.clone());
        let _ = read;
        sessions.push(NativeSession {
            id: id.clone(),
            title: summary.unwrap_or_else(|| basename(&cwd)),
            cwd: cwd.clone(),
            project_id: Some(cwd),
            created_at,
            updated_at,
            archived: false,
        });
    }
    let projects: Vec<NativeProject> = project_ids
        .into_iter()
        .map(|id| NativeProject {
            title: basename(&id),
            roots: vec![id.clone()],
            id,
        })
        .collect();
    Ok(Catalog {
        source_home,
        projects,
        sessions,
    })
}

// ── Merge (projectImportCatalog.ts + route assembly) ─────────────────

fn list_project_imports(payload: &Value) -> Result<Value, WsErr> {
    let mut providers: Vec<&'static str> = Vec::new();
    for p in payload
        .get("providers")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        match p.as_str() {
            Some("codex") if !providers.contains(&"codex") => providers.push("codex"),
            Some("claudeAgent") if !providers.contains(&"claudeAgent") => {
                providers.push("claudeAgent")
            }
            _ => {}
        }
    }
    let import_sources = sources(&providers);
    // (instance_id, account_label, error, catalog)
    let mut results: Vec<(String, Option<String>, Option<String>, Option<Catalog>)> =
        Vec::new();
    for src in &import_sources {
        let outcome = if src.provider == "codex" {
            discover_codex(src)
        } else {
            discover_claude(src)
        };
        match outcome {
            Ok(catalog) => results.push((
                src.instance_id.clone(),
                src.account_label.clone(),
                None,
                Some(catalog),
            )),
            Err(e) => results.push((
                src.instance_id.clone(),
                src.account_label.clone(),
                Some(e),
                None,
            )),
        }
    }

    // Read model pieces from a state copy (upstream holds an exclusive lock).
    let (existing, live_threads, live_projects, origins, bindings) = read_model_rows();

    let mut existing_by_root: HashMap<String, (String, String, String)> = HashMap::new();
    for (id, title, root, kind, deleted) in &existing {
        if deleted.is_some() || (kind.is_some() && kind.as_deref() != Some("project")) {
            continue;
        }
        if !is_abs_dir(root) {
            continue;
        }
        let canon = canonical_import_path(root);
        existing_by_root
            .entry(path_identity(&canon))
            .or_insert_with(|| (id.clone(), title.clone(), canon));
    }
    let mut declared: HashMap<String, (String, String)> = HashMap::new();
    for (src, (_, _, _, catalog)) in import_sources.iter().zip(results.iter()) {
        let Some(catalog) = catalog else { continue };
        if src.provider != "codex" || !is_abs_dir(&catalog.source_home) {
            continue;
        }
        for project in &catalog.projects {
            for value in &project.roots {
                if !is_abs_dir(value) {
                    continue;
                }
                let root = canonical_import_path(value);
                declared
                    .entry(path_identity(&root))
                    .or_insert_with(|| (root, project.title.clone()));
            }
        }
    }
    let inference_roots: Vec<String> = existing_by_root
        .values()
        .map(|(_, _, r)| r.clone())
        .chain(declared.values().map(|(r, _)| r.clone()))
        .collect();

    let mut out: HashMap<String, OutProject> = HashMap::new();
    let mut imported_keys: HashSet<String> = HashSet::new();

    let derived_workspace = |cwd: &str| -> String {
        let git = find_git_workspace(cwd);
        let existing = most_specific_root(inference_roots.iter().map(|s| s.as_str()), cwd)
            .map(|s| s.to_string());
        let Some((git_root, worktree)) = git else {
            return existing.unwrap_or_else(|| cwd.to_string());
        };
        if let Some(ex) = &existing {
            if root_within(ex, worktree.as_ref().unwrap_or(&git_root)) {
                return ex.clone();
            }
        }
        if let Some(wt) = &worktree {
            if let Some(rel) = cwd.strip_prefix(&format!("{}/", wt.trim_end_matches('/'))) {
                let original = format!("{}/{}", git_root.trim_end_matches('/'), rel);
                if let Some(oe) =
                    most_specific_root(inference_roots.iter().map(|s| s.as_str()), &original)
                {
                    if root_within(oe, &git_root) {
                        return oe.to_string();
                    }
                }
            }
        }
        canonical_import_path(&git_root)
    };


    for (src, (_, _, _, catalog)) in import_sources.iter().zip(results.iter()) {
        let Some(catalog) = catalog else { continue };
        if !is_abs_dir(&catalog.source_home) {
            continue;
        }
        let source_home = canonical_import_path(&catalog.source_home);
        // sourceProject id -> (title, resolved roots)
        let mut source_projects: HashMap<String, (String, Vec<String>)> = HashMap::new();
        for sp in &catalog.projects {
            let mut roots: BTreeMap<String, String> = BTreeMap::new();
            for sr in &sp.roots {
                if !is_abs_dir(sr) {
                    continue;
                }
                let physical = canonical_import_path(sr);
                let root = if src.provider == "claudeAgent" {
                    derived_workspace(&physical)
                } else {
                    physical
                };
                let title = if src.provider == "claudeAgent" {
                    let b = basename(&root);
                    if b.is_empty() {
                        sp.title.clone()
                    } else {
                        b
                    }
                } else {
                    sp.title.clone()
                };
                ensure_project(&mut out, &existing_by_root, &declared, path_identity(&root), &root, &title, src.provider);
                roots.insert(path_identity(&root), root);
            }
            source_projects.insert(
                sp.id.clone(),
                (sp.title.clone(), roots.into_values().collect()),
            );
        }
        for session in &catalog.sessions {
            if session.id.trim().is_empty() || !is_abs_dir(&session.cwd) {
                continue;
            }
            let key = project_key(&[src.provider, &source_home, &session.id]);
            if imported_keys.contains(&key) {
                continue;
            }
            let cwd = canonical_import_path(&session.cwd);
            let source_project = if src.provider == "codex" {
                session
                    .project_id
                    .as_ref()
                    .and_then(|p| source_projects.get(p))
            } else {
                None
            };
            let mut root: Option<String> = None;
            let mut assigned = false;
            if let Some((_, sproots)) = source_project {
                if sproots.len() == 1 {
                    root = Some(sproots[0].clone());
                } else if !sproots.is_empty() {
                    root = most_specific_root(sproots.iter().map(|s| s.as_str()), &cwd)
                        .map(|s| s.to_string());
                    if root.is_none() {
                        if let Some((git_root, Some(wt))) = find_git_workspace(&cwd) {
                            if let Some(rel) =
                                cwd.strip_prefix(&format!("{}/", wt.trim_end_matches('/')))
                            {
                                let original =
                                    format!("{}/{}", git_root.trim_end_matches('/'), rel);
                                root = most_specific_root(
                                    sproots.iter().map(|s| s.as_str()),
                                    &original,
                                )
                                .map(|s| s.to_string());
                            }
                        }
                    }
                }
            }
            if root.is_some() {
                assigned = true;
            }
            let root = match root {
                Some(r) => r,
                None => derived_workspace(&cwd),
            };
            let title = if assigned {
                source_project.map(|(t, _)| t.clone()).unwrap()
            } else {
                let b = basename(&root);
                if b.is_empty() {
                    root.clone()
                } else {
                    b
                }
            };
            let ident = path_identity(&root);
            ensure_project(&mut out, &existing_by_root, &declared, ident.clone(), &root, &title, src.provider);
            imported_keys.insert(key.clone());
            let mut thread = json!({
                "key": key,
                "provider": src.provider,
                "title": session.title,
                "cwd": cwd,
                "createdAt": session.created_at,
                "updatedAt": session.updated_at,
                "archived": session.archived,
            });
            if !src.is_default {
                thread["providerInstanceId"] = json!(src.instance_id);
                if let Some(label) = &src.account_label {
                    thread["accountLabel"] = json!(label);
                }
            }
            // filled below once origins/known are computed
            thread["_externalId"] = json!(session.id);
            if let Some(p) = out.get_mut(&ident) {
                p.threads.push(thread);
            }
        }
    }

    // alreadyImported: origins alive in the read model, else a live native binding.
    let origin_by_key: HashMap<String, String> = origins
        .iter()
        .filter_map(|o| {
            let key = o.get("sourceKey")?.as_str()?.to_string();
            let status = o.get("status")?.as_str()?.to_string();
            let thread_id = o.get("threadId")?.as_str()?;
            let project_id = o.get("projectId")?.as_str()?;
            let alive = live_threads.get(thread_id) == Some(&Some(project_id.to_string()));
            let pending_ok = status == "pending"
                && !live_threads.contains_key(thread_id)
                && live_projects.contains(project_id);
            (alive || pending_ok).then_some((key, status))
        })
        .collect();
    let known: HashSet<String> = bindings
        .iter()
        .filter_map(|(thread_id, project_id, provider, cursor)| {
            if !live_projects.contains(project_id.as_str()) {
                return None;
            }
            if live_threads.get(thread_id.as_str()) != Some(&Some(project_id.clone())) {
                return None;
            }
            let parsed: Value = serde_json::from_str(cursor.as_deref().unwrap_or("null")).ok()?;
            let external = if provider == "codex" {
                parsed.get("threadId").and_then(|v| text(v))
            } else if provider == "claudeAgent" {
                parsed.get("resume").and_then(|v| text(v))
            } else {
                return None;
            };
            external.map(|e| format!("{provider}:{e}"))
        })
        .collect();

    write_import_catalog(&out);
    let mut projects_out: Vec<Value> = Vec::new();
    let mut sorted: Vec<(String, OutProject)> = out.into_iter().collect();
    sorted.sort_by(|a, b| a.1.title.cmp(&b.1.title).then(a.0.cmp(&b.0)));
    for (ident, mut project) in sorted {
        project.providers.sort();
        project.threads.sort_by(|a, b| {
            b["updatedAt"]
                .as_str()
                .unwrap_or("")
                .cmp(a["updatedAt"].as_str().unwrap_or(""))
                .then(
                    a["key"].as_str().unwrap_or("").cmp(b["key"].as_str().unwrap_or("")),
                )
        });
        let key = project_key(&["project", &ident]);
        let threads: Vec<Value> = project
            .threads
            .iter_mut()
            .map(|t| {
                let tkey = t["key"].as_str().unwrap_or("").to_string();
                let provider = t["provider"].as_str().unwrap_or("").to_string();
                let external = t["_externalId"].as_str().unwrap_or("").to_string();
                let already = origin_by_key.get(&tkey).map(|s| s.as_str()) == Some("completed")
                    || (!origin_by_key.contains_key(&tkey)
                        && known.contains(&format!("{provider}:{external}")));
                t.as_object_mut().unwrap().remove("_externalId");
                t["alreadyImported"] = json!(already);
                t.clone()
            })
            .collect();
        projects_out.push(json!({
            "key": key,
            "title": project.title,
            "workspaceRoot": project.workspace_root,
            "directoryExists": project.directory_exists,
            "existingProjectId": project.existing_project_id,
            "providers": project.providers,
            "threads": threads,
        }));
    }
    let sources_out: Vec<Value> = import_sources
        .iter()
        .zip(results.iter())
        .map(|(src, (_, _, error, _))| {
            let mut s = json!({
                "provider": src.provider,
                "error": error,
            });
            if !src.is_default {
                s["providerInstanceId"] = json!(src.instance_id);
            }
            if let Some(label) = &src.account_label {
                s["accountLabel"] = json!(label);
            }
            s
        })
        .collect();
    Ok(json!({"projects": projects_out, "sources": sources_out}))
}

// ── orchestration.importProject (project-linked path) ──

/// Upstream keeps the preview catalog in-process (CATALOG_TTL_MS = 30 min);
/// ours persists the same shape so stateless handlers can resume it.
const CATALOG_TTL_MS: i64 = 30 * 60 * 1000;

fn catalog_path() -> PathBuf {
    PathBuf::from(crate::todo::state_dir()).join("import-catalog.json")
}

fn write_import_catalog(projects: &HashMap<String, OutProject>) {
    let now = now_ms();
    let mut projs = serde_json::Map::new();
    let mut sessions = serde_json::Map::new();
    for (ident, p) in projects {
        let pkey = project_key(&["project", ident]);
        projs.insert(
            pkey.clone(),
            json!({"title": p.title, "workspaceRoot": p.workspace_root,
                   "expiresAt": now + CATALOG_TTL_MS}),
        );
        for t in &p.threads {
            if let Some(tkey) = t.get("key").and_then(Value::as_str) {
                sessions.insert(
                    tkey.to_string(),
                    json!({"projectKey": pkey,
                           "provider": t.get("provider").and_then(Value::as_str).unwrap_or(""),
                           "externalId": t.get("_externalId").and_then(Value::as_str).unwrap_or(""),
                           "expiresAt": now + CATALOG_TTL_MS}),
                );
            }
        }
    }
    let doc = json!({"projects": projs, "sessions": sessions});
    if let Ok(text) = serde_json::to_string(&doc) {
        let _ = std::fs::write(catalog_path(), text);
    }
}

fn read_import_catalog() -> Option<Value> {
    let text = std::fs::read_to_string(catalog_path()).ok()?;
    serde_json::from_str(&text).ok()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `importProject` without a thread selection links (or reuses) a Synara
/// project for the catalog entry. Thread copies go through
/// `providerService.importExternalThread`, which has no provider runtime
/// here — matching upstream's own missing-capability error.
fn import_project(payload: &Value) -> Result<Value, WsErr> {
    let project_key_in = payload
        .get("projectKey")
        .and_then(Value::as_str)
        .ok_or_else(|| err("missing projectKey"))?;
    let catalog = read_import_catalog().unwrap_or(json!({}));
    let now = now_ms();
    let cached = catalog
        .pointer(&format!("/projects/{project_key_in}"))
        .cloned();
    let expired = cached
        .as_ref()
        .and_then(|c| c.get("expiresAt").and_then(Value::as_i64))
        .map(|e| e < now)
        .unwrap_or(true);
    if cached.is_none() || expired {
        return Err(err("Refresh the project list before importing."));
    }
    let cached = cached.unwrap();
    let thread_key = payload.get("threadKey").and_then(Value::as_str);
    let mut source_selected = false;
    if let Some(tk) = thread_key {
        let sel = catalog.pointer(&format!("/sessions/{tk}")).cloned();
        let ok = sel
            .as_ref()
            .map(|s| {
                s.get("projectKey").and_then(Value::as_str) == Some(project_key_in)
                    && s.get("expiresAt").and_then(Value::as_i64).unwrap_or(0) >= now
            })
            .unwrap_or(false);
        if !ok {
            return Err(err(
                "This conversation is no longer in the project preview. Refresh the list.",
            ));
        }
        source_selected = true;
    }

    let raw_root = payload
        .get("workspaceRoot")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| {
            cached
                .get("workspaceRoot")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        });
    let expanded = if let Some(rest) = raw_root.strip_prefix("~/") {
        format!("{}/{}", std::env::var("HOME").unwrap_or_default(), rest)
    } else {
        raw_root
    };
    if !Path::new(&expanded).is_absolute() {
        return Err(err("Choose an absolute project folder path."));
    }
    let workspace_root = canonical_import_path(&expanded);
    if payload.get("workspaceRoot").is_some() && !dir_exists(&workspace_root) {
        return Err(err("The selected project folder does not exist."));
    }

    // Existing live project whose canonical root matches keeps the link.
    let find_existing = |conn: &Connection| -> Option<String> {
        let mut stmt = conn
            .prepare(
                "SELECT project_id, workspace_root FROM projection_projects \
                 WHERE deleted_at IS NULL AND kind = 'project'",
            )
            .ok()?;
        let mut rows = stmt.query([]).ok()?;
        while let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0).ok()?;
            let root: String = row.get(1).unwrap_or_default();
            if path_identity(&canonical_import_path(&root)) == path_identity(&workspace_root) {
                return Some(id);
            }
        }
        None
    };
    let conn = crate::orchestration_engine::open_state_rw()?;
    let project_id = match find_existing(&conn) {
        Some(id) => id,
        None => {
            let new_id = crate::orchestration_engine::uuid_v4();
            let mut command = serde_json::Map::new();
            command.insert("type".into(), json!("project.create"));
            command.insert(
                "commandId".into(),
                json!(crate::orchestration_engine::uuid_v4()),
            );
            command.insert("projectId".into(), json!(new_id.clone()));
            command.insert(
                "title".into(),
                cached.get("title").cloned().unwrap_or(json!("Imported project")),
            );
            command.insert("workspaceRoot".into(), json!(workspace_root.clone()));
            command.insert("kind".into(), json!("project"));
            command.insert("defaultModelSelection".into(), Value::Null);
            command.insert("preserveExistingProject".into(), json!(true));
            if let Some(space) = payload.get("spaceId").and_then(Value::as_str) {
                command.insert("spaceId".into(), json!(space));
            }
            command.insert(
                "createdAt".into(),
                json!(crate::todo::iso_from_ms(now)),
            );
            match crate::orchestration_engine::project_create(&command) {
                Ok(_) => {
                    // Read back the projection row; fall back to an existing
                    // match when the decider preserved an older project.
                    let mut stmt = conn
                        .prepare(
                            "SELECT project_id FROM projection_projects \
                             WHERE project_id = ?1 AND deleted_at IS NULL",
                        )
                        .map_err(|e| err(e.to_string()))?;
                    let found: Option<String> = stmt
                        .query_row(rusqlite::params![new_id], |r| r.get(0))
                        .ok();
                    match found.or_else(|| find_existing(&conn)) {
                        Some(id) => id,
                        None => {
                            return Err(err("The imported project could not be loaded."));
                        }
                    }
                }
                Err(e) => match find_existing(&conn) {
                    Some(id) => id,
                    None => return Err(e),
                },
            }
        }
    };

    if !source_selected {
        return Ok(json!({
            "projectId": project_id,
            "threadId": Value::Null,
            "status": "project-linked",
        }));
    }
    // The project link is already committed, matching upstream's ordering —
    // the thread copy then fails at the provider-capability check.
    Err(err("Native conversation copying is unavailable."))
}

// (existing projects, live thread->project map, live project ids, origins, native bindings)
#[allow(clippy::type_complexity)]
fn read_model_rows() -> (
    Vec<(String, String, String, Option<String>, Option<String>)>,
    HashMap<String, Option<String>>,
    HashSet<String>,
    Vec<Value>,
    Vec<(String, String, String, Option<String>)>,
) {
    let Some(copy) = crate::orchestration_snapshot::open_state_copy() else {
        return (
            Vec::new(),
            HashMap::new(),
            HashSet::new(),
            Vec::new(),
            Vec::new(),
        );
    };
    let conn = &copy.conn;
    let mut existing = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT project_id, title, workspace_root, kind, deleted_at FROM projection_projects",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        }) {
            existing = rows.flatten().collect();
        }
    }
    let mut live_threads: HashMap<String, Option<String>> = HashMap::new();
    let mut live_projects: HashSet<String> = HashSet::new();
    for (id, _, _, _, deleted) in &existing {
        if deleted.is_none() {
            live_projects.insert(id.clone());
        }
    }
    if let Ok(mut stmt) =
        conn.prepare("SELECT thread_id, project_id, deleted_at FROM projection_threads")
    {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
            ))
        }) {
            for (tid, pid, deleted) in rows.flatten() {
                if deleted.is_none() {
                    live_threads.insert(tid, Some(pid));
                }
            }
        }
    }
    let mut origins = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT source_key, provider, source_home, external_id, project_id, thread_id, status \
         FROM project_import_origins",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok(json!({
                "sourceKey": r.get::<_, String>(0)?,
                "provider": r.get::<_, String>(1)?,
                "sourceHome": r.get::<_, String>(2)?,
                "externalId": r.get::<_, String>(3)?,
                "projectId": r.get::<_, String>(4)?,
                "threadId": r.get::<_, String>(5)?,
                "status": r.get::<_, String>(6)?,
            }))
        }) {
            origins = rows.flatten().collect();
        }
    }
    let mut bindings = Vec::new();
    if let Ok(mut stmt) = conn.prepare(
        "SELECT r.thread_id, t.project_id, r.provider_name, r.resume_cursor_json \
         FROM provider_session_runtime r JOIN projection_threads t ON t.thread_id = r.thread_id",
    ) {
        if let Ok(rows) = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, Option<String>>(3)?,
            ))
        }) {
            bindings = rows.flatten().collect();
        }
    }
    (existing, live_threads, live_projects, origins, bindings)
}

fn sql_to_json(v: rusqlite::types::Value) -> Value {
    match v {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => json!(i),
        rusqlite::types::Value::Real(f) => json!(f),
        rusqlite::types::Value::Text(t) => json!(t),
        rusqlite::types::Value::Blob(b) => {
            json!(String::from_utf8_lossy(&b).into_owned())
        }
    }
}


#[derive(Default)]
struct OutProject {
    title: String,
    workspace_root: String,
    directory_exists: bool,
    existing_project_id: Option<String>,
    providers: Vec<&'static str>,
    threads: Vec<Value>,
}

#[allow(clippy::too_many_arguments)]
fn ensure_project(
    out: &mut HashMap<String, OutProject>,
    existing_by_root: &HashMap<String, (String, String, String)>,
    declared: &HashMap<String, (String, String)>,
    ident: String,
    root: &str,
    title: &str,
    provider: &'static str,
) {
    let entry = out.entry(ident.clone()).or_insert_with(|| {
        let existing = existing_by_root.get(&ident);
        OutProject {
            title: existing
                .map(|(_, t, _)| t.clone())
                .or_else(|| declared.get(&ident).map(|(_, t)| t.clone()))
                .unwrap_or_else(|| title.to_string()),
            workspace_root: root.to_string(),
            directory_exists: dir_exists(root),
            existing_project_id: existing.map(|(id, _, _)| id.clone()),
            providers: Vec::new(),
            threads: Vec::new(),
        }
    });
    if !entry.providers.contains(&provider) {
        entry.providers.push(provider);
    }
}
