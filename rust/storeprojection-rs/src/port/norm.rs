//! Port of `apps/web/src/storeNormalization.ts` plus its inlined helpers
//! (`storePersistence`, `threadWorkspace`, `git`, `asyncUserInput`, `pendingInteractions`,
//! `threadSummary`, `backgroundTasks`, `session-logic`, `workLog`, `messageTextSegments`,
//! `model`, `wsHttpUrl`, `threadVisitedPersistence`).
#![allow(dead_code)]

use crate::port::jv::*;
use std::cell::RefCell;
use std::collections::HashSet;

type R<T> = Result<T, String>;

// ---------- storePersistence (module state persists across corpus cases) ----------

#[derive(Default)]
struct Persisted {
    expanded: HashSet<String>,
    order: Vec<(String, f64)>,
    names: Vec<(String, Vec<u16>)>,
    appearances: Vec<(String, JV)>,
    legacy_defined: bool,
}
thread_local! {
    static PERSISTED: RefCell<Persisted> = RefCell::new(Persisted::default());
}
fn persisted<Rv>(f: impl FnOnce(&mut Persisted) -> Rv) -> Rv {
    PERSISTED.with(|p| f(&mut p.borrow_mut()))
}
fn order_get(k: &str) -> Option<f64> {
    persisted(|p| p.order.iter().find(|(x, _)| x == k).map(|(_, i)| *i))
}
fn order_set(k: String, i: f64) {
    persisted(|p| {
        if let Some(e) = p.order.iter_mut().find(|(x, _)| *x == k) {
            e.1 = i;
        } else {
            p.order.push((k, i));
        }
    });
}
fn order_count() -> usize {
    persisted(|p| p.order.len())
}
fn expanded_count() -> usize {
    persisted(|p| p.expanded.len())
}
fn is_legacy() -> bool {
    persisted(|p| p.legacy_defined)
}
fn name_for(k: &str) -> Option<Vec<u16>> {
    persisted(|p| p.names.iter().find(|(x, _)| x == k).map(|(_, v)| v.clone()))
}
fn name_set(k: &str, v: Vec<u16>) {
    persisted(|p| {
        if let Some(e) = p.names.iter_mut().find(|(x, _)| *x == k) {
            e.1 = v;
        } else {
            p.names.push((k.to_string(), v));
        }
    });
}
fn name_del(k: &str) {
    persisted(|p| p.names.retain(|(x, _)| x != k));
}
fn appearance_for(k: &str) -> JV {
    persisted(|p| {
        p.appearances
            .iter()
            .find(|(x, _)| x == k)
            .map(|(_, v)| v.clone())
            .unwrap_or(JV::U)
    })
}
fn appearance_set(k: &str, v: JV) {
    persisted(|p| {
        if let Some(e) = p.appearances.iter_mut().find(|(x, _)| *x == k) {
            e.1 = v;
        } else {
            p.appearances.push((k.to_string(), v));
        }
    });
}
fn appearance_del(k: &str) {
    persisted(|p| p.appearances.retain(|(x, _)| x != k));
}

pub fn forget_project_state(cwd: &JV) -> R<()> {
    let k = project_cwd_key(cwd)?;
    persisted(|p| {
        p.expanded.remove(&k);
        p.order.retain(|(x, _)| *x != k);
        p.names.retain(|(x, _)| *x != k);
        p.appearances.retain(|(x, _)| *x != k);
    });
    Ok(())
}
pub fn reset_remembered() {
    persisted(|p| *p = Persisted::default());
}
/// `resetStaleRememberedProjectState` over normalized cwd keys.
pub fn reset_stale_remembered(incoming: &HashSet<String>) {
    persisted(|p| {
        if incoming.is_empty() {
            *p = Persisted::default();
            return;
        }
        let mut remembered: HashSet<String> = HashSet::new();
        for k in p
            .order
            .iter()
            .map(|(k, _)| k)
            .chain(p.names.iter().map(|(k, _)| k))
            .chain(p.appearances.iter().map(|(k, _)| k))
        {
            remembered.insert(k.clone());
        }
        for k in p.expanded.iter() {
            remembered.insert(k.clone());
        }
        if p.legacy_defined && remembered.is_empty() {
            return;
        }
        let mut has_incoming = false;
        for k in remembered.iter() {
            if incoming.contains(k) {
                has_incoming = true;
            } else {
                p.expanded.remove(k);
                p.order.retain(|(x, _)| x != k);
                p.names.retain(|(x, _)| x != k);
                p.appearances.retain(|(x, _)| x != k);
            }
        }
        if !has_incoming {
            *p = Persisted::default();
            return;
        }
        let mut ordered: Vec<(String, f64)> = p.order.clone();
        crate::js::sort(&mut ordered[..], |a, b| a.1 - b.1);
        for (index, (k, _)) in ordered.iter().enumerate() {
            order_set(k.clone(), index as f64);
        }
    });
}
/// `rememberProjectState(projects)` — `projects.entries()` errors on non-arrays.
pub fn remember_project_state(projects: &JV) -> R<()> {
    let items = arr_recv(projects, "projects", "entries", "projects.entries()")?;
    for (index, project) in items.iter().enumerate() {
        let cwd_key = project_cwd_key(&g(project, "cwd"))?;
        if truthy(&g(project, "expanded")) {
            persisted(|p| p.expanded.insert(cwd_key.clone()));
        } else {
            persisted(|p| p.expanded.remove(&cwd_key));
        }
        order_set(cwd_key.clone(), index as f64);
        let local_name = g(project, "localName");
        let trimmed = if is_nullish(&local_name) {
            Vec::new()
        } else {
            str_trim(str_recv(
                &local_name,
                "project.localName?",
                "trim",
                "project.localName?.trim()",
            )?)
        };
        if !trimmed.is_empty() {
            name_set(&cwd_key, trimmed);
        } else {
            name_del(&cwd_key);
        }
        let appearance = g(project, "appearance");
        if truthy(&appearance) {
            appearance_set(&cwd_key, appearance);
        } else {
            appearance_del(&cwd_key);
        }
    }
    if order_count() > 0 {
        persisted(|p| p.legacy_defined = false);
    }
    Ok(())
}

// ---------- threadWorkspace ----------

fn is_likely_windows_root(value: &[u16]) -> bool {
    re_windows_drive(value) || str_starts(value, "\\\\") || str_starts(value, "//")
}
/// `normalizeWorkspaceRootForComparison` (no platform: darwin/win never apply).
pub fn normalize_workspace_root(value: &JV, expr: &str) -> R<Vec<u16>> {
    let u = str_recv(value, expr, "trim", &format!("{expr}.trim()"))?;
    let trimmed = str_trim(u);
    if trimmed.is_empty() {
        return Ok(trimmed);
    }
    let mut fwd: Vec<u16> = trimmed
        .iter()
        .map(|c| if *c == 0x5c { 0x2f } else { *c })
        .collect();
    let is_win_drive_root = fwd.len() >= 3
        && (fwd[0] | 0x20) >= 0x61
        && (fwd[0] | 0x20) <= 0x7a
        && fwd[1] == 0x3a
        && fwd[2..].iter().all(|c| *c == 0x2f);
    let has_unc = str_starts(&fwd, "//");
    let prefix_len = if has_unc {
        2
    } else if str_starts(&fwd, "/") {
        1
    } else {
        0
    };
    // body = fwd[prefix..].replace(/\/+/g,"/")
    let mut body: Vec<u16> = Vec::new();
    let mut last_slash = false;
    for c in &fwd[prefix_len..] {
        if *c == 0x2f {
            if !last_slash {
                body.push(0x2f);
            }
            last_slash = true;
        } else {
            body.push(*c);
            last_slash = false;
        }
    }
    while body.last() == Some(&0x2f) {
        body.pop();
    }
    let mut normalized: Vec<u16> = fwd[..prefix_len].to_vec();
    normalized.extend(&body);
    let final_value = if is_win_drive_root {
        fwd.truncate(2);
        fwd.push(0x2f);
        fwd
    } else if !normalized.is_empty() {
        normalized
    } else {
        fwd[..prefix_len].to_vec()
    };
    if is_likely_windows_root(&trimmed) {
        Ok(str_lower(&final_value))
    } else {
        Ok(final_value)
    }
}
/// `projectCwdKey` — the recorded error names the inner expression `value.trim`.
pub fn project_cwd_key(cwd: &JV) -> R<String> {
    let u = normalize_workspace_root(cwd, "value")?;
    Ok(units_pkey(&u))
}

// ---------- model ----------

fn aliases_for(provider: &JV) -> Option<&'static [(&'static str, &'static str)]> {
    const CODEX: &[(&str, &str)] = &[
        ("sol", "gpt-6-sol"),
        ("luna", "gpt-6-luna"),
        ("astra", "gpt-6-astra"),
        ("6", "gpt-6-astra"),
        ("gpt-6", "gpt-6-astra"),
        ("5.5", "gpt-5.5"),
        ("5.4", "gpt-5.4"),
        ("5.3", "gpt-5.3-codex"),
        ("gpt-5.3", "gpt-5.3-codex"),
        ("5.3-spark", "gpt-5.3-codex-spark"),
        ("gpt-5.3-spark", "gpt-5.3-codex-spark"),
    ];
    const CLAUDE: &[(&str, &str)] = &[
        ("fable", "claude-fable-5-1"),
        ("fable-5.1", "claude-fable-5-1"),
        ("claude-fable-5.1", "claude-fable-5-1"),
        ("claude-fable-5-1", "claude-fable-5-1"),
        ("fable-5", "claude-fable-5"),
        ("claude-fable-5", "claude-fable-5"),
        ("opus", "claude-opus-5-5"),
        ("opus-5.5", "claude-opus-5-5"),
        ("claude-opus-5.5", "claude-opus-5-5"),
        ("claude-opus-5-5", "claude-opus-5-5"),
        ("opus-5", "claude-opus-5"),
        ("claude-opus-5", "claude-opus-5"),
        ("opus-4.8", "claude-opus-4-8"),
        ("claude-opus-4.8", "claude-opus-4-8"),
        ("claude-opus-4-8-20260528", "claude-opus-4-8"),
        ("opus-4.7", "claude-opus-4-7"),
        ("claude-opus-4.7", "claude-opus-4-7"),
        ("claude-opus-4-7-20260416", "claude-opus-4-7"),
        ("opus-4.6", "claude-opus-4-6"),
        ("claude-opus-4.6", "claude-opus-4-6"),
        ("claude-opus-4-6-20251117", "claude-opus-4-6"),
        ("opus-4.5", "claude-opus-4-5"),
        ("claude-opus-4.5", "claude-opus-4-5"),
        ("claude-opus-4-5-20250120", "claude-opus-4-5"),
        ("sonnet", "claude-sonnet-5"),
        ("sonnet-5", "claude-sonnet-5"),
        ("claude-sonnet-5", "claude-sonnet-5"),
        ("sonnet-4.6", "claude-sonnet-4-6"),
        ("claude-sonnet-4.6", "claude-sonnet-4-6"),
        ("claude-sonnet-4-6-20251117", "claude-sonnet-4-6"),
        ("haiku", "claude-haiku-4-5"),
        ("haiku-4.5", "claude-haiku-4-5"),
        ("claude-haiku-4.5", "claude-haiku-4-5"),
        ("claude-haiku-4-5-20251001", "claude-haiku-4-5"),
    ];
    const CURSOR: &[(&str, &str)] = &[
        ("auto", "auto"),
        ("default", "auto"),
        ("composer", "composer-2.5"),
        ("composer-2.5", "composer-2.5"),
        ("composer-2", "composer-2.5"),
        ("opus", "claude-opus-5"),
        ("opus-5", "claude-opus-5"),
        ("opus-4.8", "claude-opus-4-8"),
        ("opus-4.7", "claude-opus-4-7"),
        ("opus-4.6", "claude-opus-4-6"),
        ("opus-4.6-thinking", "claude-opus-4-6"),
        ("sonnet", "claude-sonnet-5"),
        ("sonnet-5", "claude-sonnet-5"),
        ("sonnet-4.6", "claude-sonnet-4-6"),
        ("fable", "claude-fable-5"),
        ("fable-5", "claude-fable-5"),
        ("sol", "gpt-5.6-sol"),
        ("5.6", "gpt-5.6-sol"),
        ("gpt-5.3", "gpt-5.3-codex"),
        ("codex-5.3", "gpt-5.3-codex"),
        ("grok", "grok-4.6"),
        ("grok-4.5", "grok-4.5"),
        ("grok-4.6", "grok-4.6"),
        ("cursor-grok-4.5", "grok-4.5"),
        ("cursor-grok-4.6", "grok-4.6"),
        ("gemini", "gemini-3.1-pro"),
        ("gemini-3", "gemini-3.1-pro"),
        ("gemini-3-pro", "gemini-3.1-pro"),
        ("gemini-3.1-pro-preview", "gemini-3.1-pro"),
        ("glm", "glm-5.2"),
        ("kimi", "kimi-k2.7-code"),
    ];
    const DROID: &[(&str, &str)] = &[
        ("droid", "claude-opus-4-8"),
        ("factory", "claude-opus-4-8"),
        ("opus", "claude-opus-4-8"),
        ("opus-4.8", "claude-opus-4-8"),
        ("opus-fast", "claude-opus-4-8-fast"),
        ("opus-4.8-fast", "claude-opus-4-8-fast"),
        ("opus-4.7", "claude-opus-4-7"),
        ("opus-4.7-fast", "claude-opus-4-7-fast"),
        ("opus-4.6", "claude-opus-4-6"),
        ("sonnet", "claude-sonnet-5"),
        ("sonnet-5", "claude-sonnet-5"),
        ("sonnet-4.6", "claude-sonnet-4-6"),
        ("sonnet-4.5", "claude-sonnet-4-5-20250929"),
        ("fable", "claude-fable-5"),
        ("haiku", "claude-haiku-4-5-20251001"),
        ("5.5", "gpt-5.5"),
        ("5.5-fast", "gpt-5.5-fast"),
        ("5.5-pro", "gpt-5.5-pro"),
        ("5.4", "gpt-5.4"),
        ("5.4-fast", "gpt-5.4-fast"),
        ("5.4-mini", "gpt-5.4-mini"),
        ("5.3", "gpt-5.3-codex"),
        ("5.3-fast", "gpt-5.3-codex-fast"),
        ("gpt-5.3", "gpt-5.3-codex"),
        ("gemini-3-pro", "gemini-3.1-pro-preview"),
        ("gemini-3.1-pro", "gemini-3.1-pro-preview"),
        ("gemini-3.5-flash", "gemini-3.5-flash"),
        ("gemini-3-flash", "gemini-3-flash-preview"),
        ("glm", "glm-5.2"),
        ("glm-5.2", "glm-5.2"),
        ("glm-5.1", "glm-5.1"),
        ("nemotron", "nemotron-3-ultra"),
        ("kimi", "kimi-k2.7-code"),
        ("kimi-code", "kimi-k2.7-code"),
        ("deepseek", "deepseek-v4-pro"),
        ("minimax", "minimax-m3"),
    ];
    const GROK: &[(&str, &str)] = &[
        ("grok", "grok-build-0.1"),
        ("build", "grok-build-0.1"),
        ("grok-build-0.1", "grok-build-0.1"),
        ("grok-build", "grok-build"),
        ("4.3", "grok-build"),
        ("grok-4", "grok-build"),
        ("grok-4.3", "grok-build"),
        ("grok-latest", "grok-build"),
        ("grok-code-fast", "grok-build-0.1"),
        ("grok-code-fast-1", "grok-build-0.1"),
        ("grok-code-fast-1-0825", "grok-build-0.1"),
        ("code-fast", "grok-build-0.1"),
        ("4.5", "grok-4.5"),
        ("grok-4.5", "grok-4.5"),
        ("4.6", "grok-4.6"),
        ("grok-4.6", "grok-4.6"),
    ];
    const DEVIN: &[(&str, &str)] = &[
        ("adaptive", "adaptive"),
        ("auto", "adaptive"),
        ("fast", "swe-1-6"),
        ("swe-1.6-fast", "swe-1-6"),
        ("swe-1.6", "swe-1-6"),
        ("swe-1.7", "swe-1-7"),
        ("swe", "swe-1-6"),
        ("swe-1-6", "swe-1-6"),
        ("swe-1-7", "swe-1-7"),
        ("opus", "claude-opus-4-8"),
        ("sonnet", "claude-sonnet-5"),
        ("fable", "claude-fable-5"),
    ];
    const EMPTY: &[(&str, &str)] = &[];
    match units(provider).map(|u| u.to_vec()) {
        None => Some(EMPTY),
        Some(u) => {
            if ueq(&u, "codex") {
                Some(CODEX)
            } else if ueq(&u, "claudeAgent") {
                Some(CLAUDE)
            } else if ueq(&u, "cursor") {
                Some(CURSOR)
            } else if ueq(&u, "antigravity") {
                Some(EMPTY)
            } else if ueq(&u, "droid") {
                Some(DROID)
            } else if ueq(&u, "grok") {
                Some(GROK)
            } else if ueq(&u, "opencode") || ueq(&u, "pi") {
                Some(EMPTY)
            } else if ueq(&u, "devin") {
                Some(DEVIN)
            } else if ueq(&u, "omp") {
                Some(EMPTY)
            } else {
                None
            }
        }
    }
}
/// `normalizeModelSlug(model, provider = "codex")`; `provider` here is the already-read
/// `value.provider` (the default applies when it is `undefined`). Unknown provider maps to
/// `MODEL_SLUG_ALIASES_BY_PROVIDER[provider]` === undefined -> `hasOwnProperty` TypeError.
pub fn normalize_model_slug(model: &JV, provider: &JV) -> R<JV> {
    let Some(u) = units(model).map(|x| x.to_vec()) else {
        return Ok(JV::N);
    };
    let trimmed = str_trim(&u);
    if trimmed.is_empty() {
        return Ok(JV::N);
    }
    let is_claude = eq(provider, &s("claudeAgent"));
    let provider = if matches!(provider, JV::U) {
        s("codex")
    } else {
        provider.clone()
    };
    let is_devin = eq(&provider, &s("devin"));
    let scoped = if is_claude {
        match re_bracket_suffix(&trimmed) {
            Some(_) => {
                let open = trimmed.iter().rposition(|c| *c == 0x5b).unwrap_or(0);
                trimmed[..open].to_vec()
            }
            None => trimmed.clone(),
        }
    } else if is_devin {
        let lower = str_lower(&trimmed);
        if trimmed == lower && str_ends(&trimmed, "-medium") {
            trimmed[..trimmed.len() - 7].to_vec()
        } else {
            trimmed.clone()
        }
    } else {
        trimmed.clone()
    };
    let Some(aliases) = aliases_for(&provider) else {
        return Err("Cannot convert undefined or null to object".into());
    };
    let key = String::from_utf16_lossy(&str_lower(&scoped));
    let normalized: Vec<u16> = aliases
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.encode_utf16().collect())
        .unwrap_or(scoped);
    let suffix_1m = is_claude
        && re_bracket_suffix(&trimmed)
            .is_some_and(|inner| str_lower(&inner) == "1m".encode_utf16().collect::<Vec<u16>>());
    if suffix_1m {
        let mut out = normalized;
        out.extend("[1m]".encode_utf16());
        return Ok(su(out));
    }
    Ok(su(normalized))
}
/// `normalizeModelSelection(value, previous)` — `value.model` errors on nullish `value`.
pub fn normalize_model_selection(value: &JV, previous: &JV) -> R<JV> {
    let model = gv(value, "model", "value.model")?;
    let provider = gv(value, "provider", "value.provider")?;
    let normalized = normalize_model_slug(&model, &provider)?;
    let normalized = if matches!(normalized, JV::N) {
        model.clone()
    } else {
        normalized
    };
    let next = if eq(&normalized, &model) {
        value.clone()
    } else {
        let mut m = spread_map(value);
        mset(&mut m, "model", normalized);
        o(m)
    };
    Ok(
        if !matches!(previous, JV::U | JV::N) && deep_eq(previous, &next) {
            previous.clone()
        } else {
            next
        },
    )
}

// ---------- git ----------

fn is_temporary_worktree_branch(branch: &JV) -> bool {
    let Some(u) = units(branch) else { return false };
    let lowered = str_lower(&str_trim(u));
    let Some(ns) = re_temp_branch(&lowered) else {
        return false;
    };
    ueq(&ns, "synara") || matches!(fnv1a(&ns), 0x8559131d2c062cf0 | 0xd53b4ab95395e345)
}
/// `resolveThreadBranchRegressionGuard`.
pub fn branch_regression_guard(current: &JV, next: &JV) -> JV {
    if !matches!(current, JV::N)
        && !matches!(next, JV::N)
        && !is_temporary_worktree_branch(current)
        && is_temporary_worktree_branch(next)
    {
        return current.clone();
    }
    next.clone()
}

// ---------- asyncUserInput ----------

pub fn merge_async_user_input(previous: &JV, incoming: &JV) -> JV {
    if !truthy(previous) {
        return incoming.clone();
    }
    if !truthy(incoming) {
        return previous.clone();
    }
    let pseq = g(previous, "responseSequence");
    let iseq = g(incoming, "responseSequence");
    let ps = if is_nullish(&pseq) {
        0.0
    } else {
        to_num(&pseq)
    };
    let is_ = if is_nullish(&iseq) {
        0.0
    } else {
        to_num(&iseq)
    };
    if ps > is_ {
        return previous.clone();
    }
    if is_ > ps {
        return incoming.clone();
    }
    if truthy(&g(previous, "response")) {
        previous.clone()
    } else {
        incoming.clone()
    }
}

// ---------- pendingInteractions / threadSummary ----------

/// `pendingRequestInstanceKey(requestId, generation)` — `${id} ${generation ?? "legacy"}`.
pub fn pending_request_instance_key(request_id: &[u16], generation: Option<&[u16]>) -> Vec<u16> {
    let mut out = request_id.to_vec();
    out.extend(" ".encode_utf16());
    match generation {
        Some(g) if !g.is_empty() => out.extend(g.iter().copied()),
        _ => out.extend("legacy".encode_utf16()),
    }
    out
}
pub fn is_stale_pending_request_failure_detail(detail: &JV) -> bool {
    let Some(u) = units(detail) else { return false };
    if u.is_empty() {
        return false;
    }
    let n = str_lower(u);
    [
        "stale pending approval request",
        "stale pending user-input request",
        "unknown pending approval request",
        "unknown pending permission request",
        "unknown pending user-input request",
        "stale pending user input request",
        "unknown pending user input request",
    ]
    .iter()
    .any(|p| str_contains(&n, p))
}
fn lifecycle_generation(payload: &JV) -> Option<Vec<u16>> {
    let generation = g(payload, "lifecycleGeneration");
    match units(&generation) {
        Some(u) if !u.is_empty() => Some(u.to_vec()),
        _ => None,
    }
}
/// `createStalePendingInteractionMatcher(activities)` -> predicate on
/// {interactionKind, requestId, createdAt, lifecycleGeneration?}.
pub fn stale_matcher(activities: &[JV]) -> impl Fn(&str, &[u16], &JV, Option<&[u16]>) -> bool + '_ {
    let mut stale_at: JsMap = JsMap::default();
    for activity in activities {
        let kind = g(activity, "kind");
        let kind_name = if streq(&kind, "provider.approval.respond.failed") {
            "approval"
        } else if streq(&kind, "provider.user-input.respond.failed") {
            "userInput"
        } else {
            continue;
        };
        let payload = g(activity, "payload");
        if !is_obj(&payload) {
            continue;
        }
        let request_id = g(&payload, "requestId");
        let detail = g(&payload, "detail");
        if !is_str(&request_id) || !is_stale_pending_request_failure_detail(&detail) {
            continue;
        }
        let generation = lifecycle_generation(&payload);
        let mut key = kind_name.encode_utf16().collect::<Vec<u16>>();
        key.push(0u16);
        key.extend(pending_request_instance_key(
            units(&request_id).unwrap_or(&[]),
            generation.as_deref(),
        ));
        let key = su(key);
        let previous = stale_at.get(&key);
        let created = g(activity, "createdAt");
        if matches!(previous, JV::U) || js_gt(&created, &previous) {
            stale_at.set(key, created);
        }
    }
    move |interaction_kind: &str,
          request_id: &[u16],
          created_at: &JV,
          generation: Option<&[u16]>| {
        let kind_units: Vec<u16> = interaction_kind.encode_utf16().collect();
        if let Some(gen_) = generation {
            let mut key = kind_units.clone();
            key.push(0u16);
            key.extend(pending_request_instance_key(request_id, Some(gen_)));
            if stale_at.has(&su(key)) {
                return true;
            }
        }
        let mut key = kind_units;
        key.push(0u16);
        key.extend(pending_request_instance_key(request_id, None));
        let legacy = stale_at.get(&su(key));
        !matches!(legacy, JV::U) && js_ge(&legacy, created_at)
    }
}

/// `orderedActivities` from `workLog.ts` (sequence/lifecycle/compaction/id ranking).
pub fn wl_cmp(l: &JV, r: &JV) -> R<f64> {
    let ls = g(l, "sequence");
    let rs = g(r, "sequence");
    let ldef = !matches!(ls, JV::U);
    let rdef = !matches!(rs, JV::U);
    if ldef && rdef {
        if !eq(&ls, &rs) {
            return Ok(to_num(&ls) - to_num(&rs));
        }
    } else if ldef {
        return Ok(1.0);
    } else if rdef {
        return Ok(-1.0);
    }
    let created = locale_compare(
        &g(l, "createdAt"),
        &g(r, "createdAt"),
        "left.createdAt",
        "left.createdAt.localeCompare(right.createdAt)",
    )?;
    if created != 0.0 {
        return Ok(created);
    }
    let lrank = lifecycle_rank(&g(l, "kind"));
    let rrank = lifecycle_rank(&g(r, "kind"));
    if lrank != rrank {
        return Ok(lrank - rrank);
    }
    if streq(&g(l, "kind"), "context-compaction") && streq(&g(r, "kind"), "context-compaction") {
        let lsum = compaction_rank(&g(l, "summary"));
        let rsum = compaction_rank(&g(r, "summary"));
        if lsum != rsum {
            return Ok(lsum - rsum);
        }
    }
    locale_compare(
        &g(l, "id"),
        &g(r, "id"),
        "left.id",
        "left.id.localeCompare(right.id)",
    )
}
fn lifecycle_rank(kind: &JV) -> f64 {
    match units(kind) {
        Some(u) => {
            if str_ends(u, ".started") || ueq(u, "tool.started") {
                0.0
            } else if str_ends(u, ".progress") || str_ends(u, ".updated") {
                1.0
            } else if str_ends(u, ".completed") || str_ends(u, ".resolved") {
                2.0
            } else {
                1.0
            }
        }
        None => 1.0,
    }
}
fn compaction_rank(summary: &JV) -> f64 {
    match units(summary) {
        Some(u) if ueq(u, "Compacting context") || ueq(u, "Compacting conversation...") => 0.0,
        _ => 1.0,
    }
}
/// `orderedActivities` from `workLog.ts`: stable pre-check, then toSorted.
pub fn wl_ordered(activities: &JV) -> R<Vec<JV>> {
    let items = arr_recv(
        activities,
        "activities",
        "toSorted",
        "activities.toSorted(compareActivitiesByOrder)",
    )?;
    let mut sorted = true;
    for i in 1..items.len() {
        if wl_cmp(&items[i - 1], &items[i])? > 0.0 {
            sorted = false;
            break;
        }
    }
    if sorted {
        return Ok(items.to_vec());
    }
    let mut out = items.to_vec();
    let mut err = None;
    js_sort(&mut out, |a, b| match wl_cmp(a, b) {
        Ok(v) => v,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            f64::NAN
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

/// `orderedActivities` from `threadSummary.ts` (sequence ?? MAX_SAFE, createdAt, id).
pub fn ts_cmp(l: &JV, r: &JV) -> R<f64> {
    let ldef = !is_nullish(&g(l, "sequence"));
    let rdef = !is_nullish(&g(r, "sequence"));
    let ls = if ldef {
        to_num(&g(l, "sequence"))
    } else {
        9007199254740991.0
    };
    let rs = if rdef {
        to_num(&g(r, "sequence"))
    } else {
        9007199254740991.0
    };
    let d = ls - rs;
    if d != 0.0 && !d.is_nan() {
        return Ok(d);
    }
    let created = locale_compare(
        &g(l, "createdAt"),
        &g(r, "createdAt"),
        "left.createdAt",
        "left.createdAt.localeCompare(right.createdAt)",
    )?;
    if created != 0.0 {
        return Ok(created);
    }
    locale_compare(
        &g(l, "id"),
        &g(r, "id"),
        "left.id",
        "left.id.localeCompare(right.id)",
    )
}
pub fn ts_ordered(activities: &[JV]) -> R<Vec<JV>> {
    let mut sorted = true;
    for i in 1..activities.len() {
        if ts_cmp(&activities[i - 1], &activities[i])? > 0.0 {
            sorted = false;
            break;
        }
    }
    if sorted {
        return Ok(activities.to_vec());
    }
    let mut out = activities.to_vec();
    let mut err = None;
    js_sort(&mut out, |a, b| match ts_cmp(a, b) {
        Ok(v) => v,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            f64::NAN
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}

// ---------- backgroundTasks / session-logic ----------

fn as_record(v: &JV) -> Option<&Obj> {
    match v {
        JV::O(_, m) => Some(m),
        _ => None,
    }
}
/// `orderedActivities` (workLog) folded into outstanding background task ids.
fn fold_outstanding_background_task_ids(activities: &[JV]) -> R<Vec<Vec<u16>>> {
    let mut outstanding: Vec<Vec<u16>> = Vec::new();
    let ordered = wl_ordered(&a(activities.to_vec()))?;
    for activity in &ordered {
        let payload = g(activity, "payload");
        if streq(&g(activity, "kind"), "runtime.warning") {
            if !streq(&g(&payload, "nativeEventType"), "background_tasks_changed") {
                continue;
            }
            let data = g(&payload, "data");
            let Some(tasks) = (match &g(&data, "tasks") {
                JV::A(_, t) => Some(t.clone()),
                _ => None,
            }) else {
                continue;
            };
            for task in tasks.iter() {
                let task_id = g(task, "task_id");
                if let Some(u) = units(&task_id) {
                    let u = u.to_vec();
                    if !outstanding.contains(&u) {
                        outstanding.push(u);
                    }
                }
            }
            continue;
        }
        let task_id = g(&payload, "taskId");
        let Some(tu) = units(&task_id).map(|u| u.to_vec()) else {
            continue;
        };
        let kind = g(activity, "kind");
        if streq(&kind, "task.completed") {
            outstanding.retain(|x| *x != tu);
        } else if streq(&kind, "task.updated") {
            let status = g(&payload, "status");
            if streq(&status, "completed")
                || streq(&status, "failed")
                || streq(&status, "killed")
                || streq(&status, "paused")
                || eq(&g(&payload, "isBackgrounded"), &JV::B(false))
            {
                outstanding.retain(|x| *x != tu);
            } else if eq(&g(&payload, "isBackgrounded"), &JV::B(true)) && !outstanding.contains(&tu)
            {
                outstanding.push(tu);
            }
        }
    }
    Ok(outstanding)
}
fn can_session_finish_background(status: &JV) -> bool {
    !matches!(status, JV::U | JV::N) && !streq(status, "stopped") && !streq(status, "error")
}
/// `countOutstandingBackgroundWork`.
pub fn count_outstanding_background_work(activities: &JV, session: &JV) -> R<f64> {
    let status = g(session, "orchestrationStatus");
    if !can_session_finish_background(&status) {
        return Ok(0.0);
    }
    let items = match activities {
        JV::A(_, t) => t.as_ref().clone(),
        JV::U | JV::N => Vec::new(),
        _ => Vec::new(),
    };
    Ok(fold_outstanding_background_task_ids(&items)?.len() as f64)
}
/// `foldActiveTasks` — active task map {taskType,isBackgrounded,toolUseId,subagentType,workflowTaskId,turnId}.
fn fold_active_find<'a>(active: &'a mut [(Vec<u16>, Obj)], id: &[u16]) -> Option<&'a mut Obj> {
    active.iter_mut().find(|(x, _)| *x == id).map(|(_, t)| t)
}
fn fold_active_tasks(activities: &[JV], latest_turn_id: &JV) -> R<Vec<(Vec<u16>, Obj)>> {
    let ordered = wl_ordered(&a(activities.to_vec()))?;
    let mut active: Vec<(Vec<u16>, Obj)> = Vec::new();

    for activity in &ordered {
        let atid = g(activity, "turnId");
        if !matches!(latest_turn_id, JV::U)
            && truthy(latest_turn_id)
            && truthy(&atid)
            && !eq(&atid, latest_turn_id)
            && !streq(&g(activity, "kind"), "task.completed")
            && !streq(&g(activity, "kind"), "task.updated")
        {
            continue;
        }
        let kind = g(activity, "kind");
        let is_task_kind = streq(&kind, "task.started")
            || streq(&kind, "task.progress")
            || streq(&kind, "task.updated")
            || streq(&kind, "task.completed");
        if !is_task_kind {
            continue;
        }
        let payload = g(activity, "payload");
        let task_id = g(&payload, "taskId");
        let Some(tid) = units(&task_id).map(|u| u.to_vec()) else {
            continue;
        };
        if streq(&kind, "task.completed") {
            active.retain(|(x, _)| *x != tid);
            continue;
        }
        if streq(&kind, "task.updated") {
            let status = g(&payload, "status");
            if streq(&status, "completed")
                || streq(&status, "failed")
                || streq(&status, "killed")
                || streq(&status, "paused")
            {
                active.retain(|(x, _)| *x != tid);
                continue;
            }
            let previous_exists = active.iter().any(|(x, _)| *x == tid);
            let previous_bg = fold_active_find(&mut active, &tid)
                .map(|t| t.get("isBackgrounded").cloned().unwrap_or(JV::B(false)))
                .unwrap_or(JV::B(false));
            let ib = g(&payload, "isBackgrounded");
            let is_backgrounded = match &ib {
                JV::B(x) => JV::B(*x),
                _ => previous_bg,
            };
            let in_turn = !truthy(latest_turn_id) || !truthy(&atid) || eq(&atid, latest_turn_id);
            if previous_exists || (truthy(&is_backgrounded) && in_turn) {
                let prev = fold_active_find(&mut active, &tid)
                    .cloned()
                    .unwrap_or_default();
                let mut t = prev.clone();
                mset(&mut t, "isBackgrounded", is_backgrounded);
                let tool = g(&payload, "toolUseId");
                let keep = t.get("toolUseId").cloned().unwrap_or(JV::U);
                mset(
                    &mut t,
                    "toolUseId",
                    if !matches!(keep, JV::U) {
                        keep
                    } else if is_str(&tool) && !units(&tool).unwrap_or(&[]).is_empty() {
                        tool
                    } else {
                        JV::U
                    },
                );
                let wf = g(&payload, "workflowTaskId");
                let keep = t.get("workflowTaskId").cloned().unwrap_or(JV::U);
                mset(
                    &mut t,
                    "workflowTaskId",
                    if !matches!(keep, JV::U) {
                        keep
                    } else if is_str(&wf) && !units(&wf).unwrap_or(&[]).is_empty() {
                        wf
                    } else {
                        JV::U
                    },
                );
                let keep_t = t.get("turnId").cloned().unwrap_or(JV::U);
                mset(
                    &mut t,
                    "turnId",
                    if !matches!(keep_t, JV::U) {
                        keep_t
                    } else if matches!(atid, JV::U) {
                        JV::N
                    } else {
                        atid.clone()
                    },
                );
                *fold_active_find(&mut active, &tid).unwrap_or(&mut Obj::new()) = t;
            }
            continue;
        }
        // task.started / task.progress
        let prev = fold_active_find(&mut active, &tid).cloned();
        let mut t = prev.clone().unwrap_or_default();
        let tt = g(&payload, "taskType");
        mset(
            &mut t,
            "taskType",
            if is_str(&tt) && !units(&tt).unwrap_or(&[]).is_empty() {
                tt
            } else {
                prev.as_ref()
                    .and_then(|p| p.get("taskType").cloned())
                    .unwrap_or(JV::U)
            },
        );
        mset(
            &mut t,
            "isBackgrounded",
            prev.as_ref()
                .and_then(|p| p.get("isBackgrounded").cloned())
                .unwrap_or(JV::B(false)),
        );
        let tool = g(&payload, "toolUseId");
        mset(
            &mut t,
            "toolUseId",
            prev.as_ref()
                .and_then(|p| p.get("toolUseId").cloned())
                .filter(|x| !matches!(x, JV::U))
                .or(
                    if is_str(&tool) && !units(&tool).unwrap_or(&[]).is_empty() {
                        Some(tool.clone())
                    } else {
                        None
                    },
                )
                .unwrap_or(JV::U),
        );
        let st = g(&payload, "subagentType");
        mset(
            &mut t,
            "subagentType",
            prev.as_ref()
                .and_then(|p| p.get("subagentType").cloned())
                .filter(|x| !matches!(x, JV::U))
                .or(if is_str(&st) && !units(&st).unwrap_or(&[]).is_empty() {
                    Some(st.clone())
                } else {
                    None
                })
                .unwrap_or(JV::U),
        );
        let wf = g(&payload, "workflowTaskId");
        mset(
            &mut t,
            "workflowTaskId",
            prev.as_ref()
                .and_then(|p| p.get("workflowTaskId").cloned())
                .filter(|x| !matches!(x, JV::U))
                .or(if is_str(&wf) && !units(&wf).unwrap_or(&[]).is_empty() {
                    Some(wf.clone())
                } else {
                    None
                })
                .unwrap_or(JV::U),
        );
        let keep_t = prev
            .as_ref()
            .and_then(|p| p.get("turnId").cloned())
            .unwrap_or(JV::U);
        mset(
            &mut t,
            "turnId",
            if !matches!(keep_t, JV::U) {
                keep_t
            } else if matches!(atid, JV::U) {
                JV::N
            } else {
                atid.clone()
            },
        );
        if let Some(slot) = active.iter_mut().find(|(x, _)| *x == tid).map(|(_, v)| v) {
            *slot = t;
        } else {
            active.push((tid, t));
        }
    }
    Ok(active)
}
/// `deriveActiveBackgroundTasksState !== null` check.
fn has_active_background_tasks(activities: &[JV], latest_turn_id: &JV) -> R<bool> {
    let tasks = fold_active_tasks(activities, latest_turn_id)?;
    Ok(tasks
        .iter()
        .any(|(_, t)| !streq_val(t.get("taskType"), "plan")))
}
fn streq_val(v: Option<&JV>, x: &str) -> bool {
    v.is_some_and(|v| streq(v, x))
}
/// `derivePendingBackgroundWork` -> count or None (JS null).
pub fn derive_pending_background_work(
    activities: &[JV],
    latest_turn: &JV,
    session: &JV,
) -> R<Option<f64>> {
    let latest_turn_id = g(latest_turn, "turnId");
    if !truthy(&latest_turn_id) {
        return Ok(None);
    }
    let status = g(session, "orchestrationStatus");
    if matches!(status, JV::U) || streq(&status, "stopped") || streq(&status, "error") {
        return Ok(None);
    }
    let tasks = fold_active_tasks(activities, &latest_turn_id)?;
    let count = tasks
        .iter()
        .filter(|(_, t)| truthy(t.get("isBackgrounded").unwrap_or(&JV::B(false))))
        .count();
    Ok(if count > 0 { Some(count as f64) } else { None })
}
/// `hasLiveTurnTailWork`.
pub fn has_live_turn_tail_work(
    latest_turn: &JV,
    messages: &JV,
    activities: &[JV],
    session: &JV,
) -> R<bool> {
    let latest_turn_id = g(latest_turn, "turnId");
    if !truthy(&latest_turn_id) {
        return Ok(false);
    }
    let msgs = match messages {
        JV::A(_, m) => m.as_ref().clone(),
        _ => Vec::new(),
    };
    let streaming = msgs.iter().any(|m| {
        streq(&g(m, "role"), "assistant")
            && eq(&g(m, "turnId"), &latest_turn_id)
            && truthy(&g(m, "streaming"))
    });
    if streaming {
        return Ok(is_nullish(&g(latest_turn, "completedAt")));
    }
    if !streq(&g(session, "orchestrationStatus"), "running") {
        return Ok(false);
    }
    has_active_background_tasks(activities, &latest_turn_id)
}
/// `isSessionOrLatestTurnRunning`.
pub fn is_session_or_latest_turn_running(status: &JV, turn_state: &JV) -> bool {
    streq(status, "running") || streq(turn_state, "running")
}

// ---------- threadSummary derivations ----------

fn max_iso(left: &JV, right: &JV) -> JV {
    if matches!(left, JV::N) {
        return right.clone();
    }
    if js_gt(left, right) {
        left.clone()
    } else {
        right.clone()
    }
}
fn resolve_human_message_at(message: &JV) -> JV {
    if !streq(&g(message, "role"), "user") {
        return JV::N;
    }
    let origin = g(message, "dispatchOrigin");
    if !is_nullish(&origin) && !streq(&origin, "user") {
        return JV::N;
    }
    let updated = g(message, "updatedAt");
    max_iso(
        &g(message, "createdAt"),
        &if matches!(updated, JV::U) {
            g(message, "createdAt")
        } else {
            updated
        },
    )
}
fn approval_request_kind(request_type: &JV) -> Option<&'static str> {
    match units(request_type) {
        Some(u) if ueq(u, "command_execution_approval") || ueq(u, "exec_command_approval") => {
            Some("command")
        }
        Some(u) if ueq(u, "file_read_approval") => Some("file-read"),
        Some(u) if ueq(u, "file_change_approval") || ueq(u, "apply_patch_approval") => {
            Some("file-change")
        }
        Some(u) if ueq(u, "permissions_approval") => Some("permissions"),
        Some(u) if ueq(u, "tool_approval") || ueq(u, "dynamic_tool_call") => Some("tool"),
        _ => None,
    }
}
fn has_structured_user_input_questions(payload: &JV) -> bool {
    let questions = match &g(payload, "questions") {
        JV::A(_, q) => q.clone(),
        _ => return false,
    };
    questions.iter().any(|entry| {
        let Some(q) = as_record(entry) else {
            return false;
        };
        let options = match q.get("options") {
            Some(JV::A(_, o)) => o.clone(),
            _ => return false,
        };
        q.get("id").is_some_and(is_str)
            && q.get("header").is_some_and(is_str)
            && q.get("question").is_some_and(is_str)
            && options.iter().any(|opt| {
                as_record(opt).is_some_and(|o| {
                    o.get("label").is_some_and(is_str) && o.get("description").is_some_and(is_str)
                })
            })
    })
}
fn delete_open_request(
    open: &mut Vec<(Vec<u16>, Vec<u16>)>,
    request_id: &[u16],
    generation: Option<&[u16]>,
) {
    match generation {
        Some(gen_) => {
            let key = pending_request_instance_key(request_id, Some(gen_));
            open.retain(|(k, _)| *k != key);
        }
        None => open.retain(|(_, rid)| *rid != request_id),
    }
}
fn replace_open_request(
    open: &mut Vec<(Vec<u16>, Vec<u16>)>,
    request_id: &[u16],
    generation: Option<&[u16]>,
) {
    delete_open_request(open, request_id, None);
    let key = pending_request_instance_key(request_id, generation);
    if !open.iter().any(|(k, _)| *k == key) {
        open.push((key, request_id.to_vec()));
    }
}
/// `derivePendingThreadRequestIds` (pendingInteractions always absent in this call graph).
pub fn derive_pending_thread_request_ids(activities: &[JV]) -> R<(Vec<JV>, Vec<JV>)> {
    let mut open_approvals: Vec<(Vec<u16>, Vec<u16>)> = Vec::new();
    let mut open_inputs: Vec<(Vec<u16>, Vec<u16>)> = Vec::new();
    for activity in &ts_ordered(activities)? {
        let payload = g(activity, "payload");
        let request_id = g(&payload, "requestId");
        let request_id = units(&request_id).map(|u| u.to_vec());
        let detail = g(&payload, "detail");
        let generation = lifecycle_generation(&payload);
        let kind = g(activity, "kind");
        if streq(&kind, "approval.requested") {
            if let Some(rid) = &request_id {
                let rk = g(&payload, "requestKind");
                let valid = ["command", "file-read", "file-change", "permissions", "tool"]
                    .iter()
                    .any(|k| streq(&rk, k));
                if valid || approval_request_kind(&g(&payload, "requestType")).is_some() {
                    replace_open_request(&mut open_approvals, rid, generation.as_deref());
                }
            }
            continue;
        }
        if streq(&kind, "approval.resolved") {
            if let Some(rid) = &request_id {
                delete_open_request(&mut open_approvals, rid, generation.as_deref());
            }
            continue;
        }
        if streq(&kind, "provider.approval.respond.failed")
            && request_id.is_some()
            && is_stale_pending_request_failure_detail(&detail)
        {
            delete_open_request(
                &mut open_approvals,
                request_id.as_deref().unwrap_or(&[]),
                generation.as_deref(),
            );
            continue;
        }
        if streq(&kind, "user-input.requested") {
            if let Some(rid) = &request_id
                && has_structured_user_input_questions(&payload)
            {
                replace_open_request(&mut open_inputs, rid, generation.as_deref());
            }
            continue;
        }
        if streq(&kind, "user-input.resolved") {
            if let Some(rid) = &request_id {
                delete_open_request(&mut open_inputs, rid, generation.as_deref());
            }
            continue;
        }
        if streq(&kind, "provider.user-input.respond.failed")
            && request_id.is_some()
            && is_stale_pending_request_failure_detail(&detail)
        {
            delete_open_request(
                &mut open_inputs,
                request_id.as_deref().unwrap_or(&[]),
                generation.as_deref(),
            );
        }
    }
    Ok((
        open_approvals.iter().map(|(_, r)| su(r.clone())).collect(),
        open_inputs.iter().map(|(_, r)| su(r.clone())).collect(),
    ))
}
/// `resolveLatestProposedPlan` -> JV (null or the plan object).
fn resolve_latest_proposed_plan(proposed_plans: &JV, latest_turn: &JV) -> R<JV> {
    let plans = match proposed_plans {
        JV::A(_, p) => p.as_ref().clone(),
        _ => Vec::new(),
    };
    let sort_plans = |plans: &[JV]| -> R<Vec<JV>> {
        let mut out = plans.to_vec();
        let mut err = None;
        js_sort(&mut out, |l, r| {
            let c = (|| -> R<f64> {
                let c = locale_compare(
                    &g(l, "updatedAt"),
                    &g(r, "updatedAt"),
                    "left.updatedAt",
                    "left.updatedAt.localeCompare(right.updatedAt)",
                )?;
                if c != 0.0 {
                    return Ok(c);
                }
                locale_compare(
                    &g(l, "id"),
                    &g(r, "id"),
                    "left.id",
                    "left.id.localeCompare(right.id)",
                )
            })();
            match c {
                Ok(v) => v,
                Err(e) => {
                    err = Some(e);
                    f64::NAN
                }
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(out),
        }
    };
    let turn_id = g(latest_turn, "turnId");
    if truthy(&turn_id) {
        let matching: Vec<JV> = plans
            .iter()
            .filter(|p| eq(&g(p, "turnId"), &turn_id))
            .cloned()
            .collect();
        let sorted = sort_plans(&matching)?;
        if let Some(last) = sorted.last() {
            return Ok(last.clone());
        }
    }
    let sorted = sort_plans(&plans)?;
    Ok(sorted.last().cloned().unwrap_or(JV::N))
}
/// `deriveThreadSummaryMetadata` -> object with the five fields.
pub fn derive_thread_summary_metadata(
    messages: &JV,
    activities: &[JV],
    proposed_plans: &JV,
    latest_turn: &JV,
) -> R<JV> {
    let mut latest_user_at = JV::N;
    let mut latest_human_at = JV::N;
    let msgs = match messages {
        JV::A(_, m) => m.as_ref().clone(),
        _ => Vec::new(),
    };
    for message in &msgs {
        if streq(&g(message, "role"), "user") {
            latest_user_at = max_iso(&latest_user_at, &g(message, "createdAt"));
            let human = resolve_human_message_at(message);
            if !matches!(human, JV::N) {
                latest_human_at = max_iso(&latest_human_at, &human);
            }
        }
    }
    let (approvals, inputs) = derive_pending_thread_request_ids(activities)?;
    let latest_plan = resolve_latest_proposed_plan(proposed_plans, latest_turn)?;
    let actionable =
        !matches!(latest_plan, JV::N) && matches!(g(&latest_plan, "implementedAt"), JV::N);
    Ok(op(&[
        ("latestUserMessageAt", latest_user_at),
        ("latestHumanMessageAt", latest_human_at),
        ("hasPendingApprovals", b(!approvals.is_empty())),
        ("hasPendingUserInput", b(!inputs.is_empty())),
        ("hasActionableProposedPlan", b(actionable)),
    ]))
}

// ---------- storeNormalization ----------

pub const MAX_THREAD_MESSAGES: usize = 2000;
pub const MAX_THREAD_ACTIVITIES: usize = 2000;
const PENDING_REQUEST_KINDS: &[&str] = &["approval.requested", "user-input.requested"];
const PENDING_ACTIVITY_KINDS: &[&str] = &[
    "approval.requested",
    "user-input.requested",
    "approval.resolved",
    "user-input.resolved",
    "provider.approval.respond.failed",
    "provider.user-input.respond.failed",
];

/// `x ?? y` (nullish).
pub fn or(x: &JV, y: JV) -> JV {
    if is_nullish(x) { y } else { x.clone() }
}
/// `x ?? y` where undefined is the only nullish trigger (=== undefined semantics differ: null
/// triggers ?? too; this is plain ??).
pub fn orf(x: &JV, y: impl FnOnce() -> JV) -> JV {
    if is_nullish(x) { y() } else { x.clone() }
}
/// `a === b` on possibly-absent properties with a `??` default first.
pub fn eq_or(v: &JV, d: &JV) -> bool {
    eq(&or(v, d.clone()), d)
}
pub fn gv_or_null(v: &JV, k: &str) -> JV {
    or(&g(v, k), JV::N)
}
/// `v?.k` when v is undefined -> JV::U (already what `g` does).
pub fn arrays_shallow_eq(left: &JV, right: &[JV]) -> bool {
    let JV::A(_, l) = left else { return false };
    if l.len() != right.len() {
        return false;
    }
    l.iter().zip(right).all(|(a, b)| eq(a, b))
}
pub fn records_shallow_eq(left: &JV, right: &Obj) -> bool {
    let JV::O(_, l) = left else { return false };
    if l.len() != right.len() {
        return false;
    }
    l.iter()
        .all(|(k, v)| right.get(k).is_some_and(|r| eq(v, r)))
}
/// `deepEqualJson`.
pub fn deep_equal_json(a: &JV, b: &JV) -> bool {
    if same_value(a, b) {
        return true;
    }
    if is_nullish(a) || is_nullish(b) {
        return false;
    }
    match (a, b) {
        (JV::A(_, x), JV::A(_, y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| deep_equal_json(p, q))
        }
        (JV::O(_, x), JV::O(_, y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| deep_equal_json(v, w)))
        }
        (JV::A(..), _) | (_, JV::A(..)) => false,
        _ => false,
    }
}
fn source_proposed_plans_eq(l: &JV, r: &JV) -> bool {
    if eq(l, r) {
        return true;
    }
    if matches!(l, JV::U) || matches!(r, JV::U) {
        return false;
    }
    eq(&g(l, "threadId"), &g(r, "threadId")) && eq(&g(l, "planId"), &g(r, "planId"))
}
fn latest_turns_eq(l: &JV, r: &JV) -> bool {
    if eq(l, r) {
        return true;
    }
    if is_nullish(l) || is_nullish(r) {
        return false;
    }
    eq(&g(l, "turnId"), &g(r, "turnId"))
        && eq(&g(l, "state"), &g(r, "state"))
        && eq(&g(l, "requestedAt"), &g(r, "requestedAt"))
        && eq(&g(l, "startedAt"), &g(r, "startedAt"))
        && eq(&g(l, "completedAt"), &g(r, "completedAt"))
        && eq(&g(l, "assistantMessageId"), &g(r, "assistantMessageId"))
        && source_proposed_plans_eq(&g(l, "sourceProposedPlan"), &g(r, "sourceProposedPlan"))
}
pub fn thread_sessions_eq(l: &JV, r: &JV) -> bool {
    if eq(l, r) {
        return true;
    }
    if is_nullish(l) || is_nullish(r) {
        return false;
    }
    eq(&g(l, "provider"), &g(r, "provider"))
        && eq(&g(l, "status"), &g(r, "status"))
        && eq(&g(l, "orchestrationStatus"), &g(r, "orchestrationStatus"))
        && eq(&g(l, "runtimeMode"), &g(r, "runtimeMode"))
        && eq(&g(l, "activeTurnId"), &g(r, "activeTurnId"))
        && eq(&g(l, "createdAt"), &g(r, "createdAt"))
        && eq(&g(l, "updatedAt"), &g(r, "updatedAt"))
        && eq(&g(l, "lastError"), &g(r, "lastError"))
}
fn obj_g(v: &Obj, k: &str) -> JV {
    v.get(k).cloned().unwrap_or(JV::U)
}
pub fn create_branch_flow_merge(m: &[(&str, JV)]) -> JV {
    let get = |k: &str| {
        m.iter()
            .find(|(n, _)| *n == k)
            .map(|(_, v)| v.clone())
            .unwrap_or(JV::U)
    };
    let context_changed = !eq(&get("currentBranch"), &get("nextBranch"))
        || !eq(&get("currentWorktreePath"), &get("nextWorktreePath"))
        || !eq_or(
            &get("currentAssociatedWorktreePath"),
            &gv_or_null_map(&get("nextAssociatedWorktreePath")),
        )
        || !eq_or(
            &get("currentAssociatedWorktreeBranch"),
            &gv_or_null_map(&get("nextAssociatedWorktreeBranch")),
        )
        || !eq_or(
            &get("currentAssociatedWorktreeRef"),
            &gv_or_null_map(&get("nextAssociatedWorktreeRef")),
        );
    if context_changed {
        return or(&get("nextCreateBranchFlowCompleted"), b(false));
    }
    let next = get("nextCreateBranchFlowCompleted");
    if matches!(next, JV::U) {
        return or(&get("currentCreateBranchFlowCompleted"), b(false));
    }
    if truthy(&or(&get("currentCreateBranchFlowCompleted"), b(false))) && !truthy(&next) {
        return b(true);
    }
    next
}
fn gv_or_null_map(v: &JV) -> JV {
    or(v, JV::N)
}
/// `(l.f ?? d) === (r.f ?? d)` across two objects.
pub fn field_eq_or(l: &JV, r: &JV, k: &str, d: &JV) -> bool {
    eq(&or(&g(l, k), d.clone()), &or(&g(r, k), d.clone()))
}
/// `threadShellsEqual` on JV objects (shell shape incl. `..` output fields).
pub fn thread_shells_eq(l: &JV, r: &JV) -> bool {
    if matches!(l, JV::U) {
        return false;
    }
    for k in [
        "id",
        "codexThreadId",
        "projectId",
        "title",
        "modelSelection",
        "runtimeMode",
        "interactionMode",
        "error",
        "createdAt",
        "updatedAt",
        "envMode",
        "branch",
        "worktreePath",
        "claudeCacheReviewSequence",
        "usageLimitRecoverySequence",
        "snoozeSequence",
        "latestUserMessageAt",
        "latestHumanMessageAt",
        "hasPendingApprovals",
        "hasPendingUserInput",
        "hasActionableProposedPlan",
        "pendingBackgroundTaskCount",
        "latestReplyPreview",
        "pendingInteractions",
        "lastVisitedAt",
    ] {
        if !eq(&g(l, k), &g(r, k)) {
            return false;
        }
    }
    for k in [
        "archivedAt",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
        "forkSourceThreadId",
        "sidechatSourceThreadId",
        "sidechatLastActivityAt",
        "sidechatExpiredAt",
        "goalStartedAt",
        "goalPausedAt",
    ] {
        if !field_eq_or(l, r, k, &JV::N) {
            return false;
        }
    }
    for k in ["isPinned", "createBranchFlowCompleted", "isProjectImport"] {
        if !field_eq_or(l, r, k, &b(false)) {
            return false;
        }
    }
    if !field_eq_or(l, r, "notes", &s("")) || !field_eq_or(l, r, "goal", &s("")) {
        return false;
    }
    if !eq(
        &gv_or_null(&g(l, "sidechatContext"), "url"),
        &gv_or_null(&g(r, "sidechatContext"), "url"),
    ) {
        return false;
    }
    for k in [
        "lastKnownPr",
        "pullRequests",
        "claudeCacheReview",
        "usageLimitRecovery",
        "pinnedMessages",
        "goalAchievements",
    ] {
        let d = if k == "pullRequests" {
            a(vec![])
        } else {
            JV::N
        };
        if !deep_equal_json(&or(&g(l, k), d.clone()), &or(&g(r, k), d)) {
            return false;
        }
    }
    field_eq_or(l, r, "handoff", &JV::N)
}
pub fn thread_turn_states_eq(l: &JV, r: &JV) -> bool {
    !matches!(l, JV::U)
        && latest_turns_eq(&g(l, "latestTurn"), &g(r, "latestTurn"))
        && source_proposed_plans_eq(
            &g(l, "pendingSourceProposedPlan"),
            &g(r, "pendingSourceProposedPlan"),
        )
        && eq(
            &g(l, "pendingTurnStartMessageId"),
            &g(r, "pendingTurnStartMessageId"),
        )
}
fn text_segment_arrays_eq(l: &JV, r: &JV) -> bool {
    if eq(l, r) {
        return true;
    }
    let (JV::A(_, la), JV::A(_, ra)) = (l, r) else {
        return false;
    };
    if la.len() != ra.len() {
        return false;
    }
    la.iter().zip(ra.iter()).all(|(x, y)| {
        eq(&g(x, "sequence"), &g(y, "sequence"))
            && eq(&g(x, "startedAt"), &g(y, "startedAt"))
            && eq(&g(x, "endedAt"), &g(y, "endedAt"))
            && eq(&g(x, "text"), &g(y, "text"))
    })
}
fn provider_ref_arrays_eq(l: &[JV], r: &[JV]) -> bool {
    if l.len() != r.len() {
        return false;
    }
    l.iter()
        .zip(r)
        .all(|(x, y)| eq(&g(x, "name"), &g(y, "name")) && eq(&g(x, "path"), &g(y, "path")))
}
pub fn activities_equal(l: &JV, r: &JV) -> bool {
    eq(&g(l, "kind"), &g(r, "kind"))
        && eq(&g(l, "tone"), &g(r, "tone"))
        && eq(&g(l, "summary"), &g(r, "summary"))
        && deep_equal_json(&g(l, "payload"), &g(r, "payload"))
        && eq(&g(l, "turnId"), &g(r, "turnId"))
        && eq(&g(l, "sequence"), &g(r, "sequence"))
        && eq(&g(l, "sequenceSource"), &g(r, "sequenceSource"))
        && eq(&g(l, "createdAt"), &g(r, "createdAt"))
}
fn activity_payload_score(activity: &JV) -> f64 {
    let payload_jv = g(activity, "payload");
    let payload = as_record(&payload_jv);
    let data = payload.and_then(|p| p.get("data")).and_then(as_record);
    let item = data.and_then(|d| d.get("item")).and_then(as_record);
    let command_actions = item
        .and_then(|i| i.get("commandActions"))
        .or(data.and_then(|d| d.get("commandActions")))
        .or(payload.and_then(|p| p.get("commandActions")))
        .cloned()
        .unwrap_or(JV::U);
    let mut score = 0.0;
    if payload.is_some_and(|p| !matches!(p.get("itemType"), None | Some(JV::U))) {
        score += 4.0;
    }
    if payload.is_some_and(|p| !matches!(p.get("title"), None | Some(JV::U))) {
        score += 1.0;
    }
    if payload.is_some_and(|p| !matches!(p.get("detail"), None | Some(JV::U))) {
        score += 2.0;
    }
    if data.is_some() {
        score += 2.0;
    }
    if item.is_some() {
        score += 4.0;
    }
    let command = item
        .and_then(|i| i.get("command"))
        .or(data.and_then(|d| d.get("command")))
        .or(payload.and_then(|p| p.get("command")))
        .cloned()
        .unwrap_or(JV::U);
    if normalize_activity_command_value(&command).is_some() {
        score += 8.0;
    }
    if let JV::A(_, c) = &command_actions
        && !c.is_empty()
    {
        score += 8.0;
    }
    score
}
fn normalize_activity_command_value(value: &JV) -> Option<Vec<u16>> {
    match value {
        JV::S(u) => {
            let t = str_trim(u);
            if t.is_empty() { None } else { Some(t) }
        }
        JV::A(_, items) => {
            let parts: Vec<Vec<u16>> = items
                .iter()
                .filter_map(|e| units(e).map(str_trim))
                .filter(|e| !e.is_empty())
                .collect();
            if parts.is_empty() {
                return None;
            }
            let mut out = parts[0].clone();
            for p in &parts[1..] {
                out.push(0x20);
                out.extend(p);
            }
            Some(out)
        }
        _ => None,
    }
}
pub fn prefer_richer_activity(previous: &JV, incoming: &JV) -> JV {
    if activities_equal(previous, incoming) {
        return previous.clone();
    }
    let ps = activity_payload_score(previous);
    let is_ = activity_payload_score(incoming);
    if is_ < ps {
        previous.clone()
    } else {
        incoming.clone()
    }
}
pub fn dedupe_activities_by_id(activities: &[JV]) -> Vec<JV> {
    let mut index_by_id: Vec<(JV, usize)> = Vec::new();
    let mut result: Vec<JV> = Vec::new();
    for activity in activities {
        let id = g(activity, "id");
        if let Some(&(_, i)) = index_by_id.iter().find(|(x, _)| eq(x, &id)) {
            result[i] = prefer_richer_activity(&result[i], activity);
        } else {
            index_by_id.push((id, result.len()));
            result.push(activity.clone());
        }
    }
    result
}
fn dedupe_activities_node(activities: &[JV]) -> JV {
    let result = dedupe_activities_by_id(activities);
    if activities.len() == result.len() && activities.iter().zip(&result).all(|(a, b)| eq(a, b)) {
        a(activities.to_vec())
    } else {
        a(result)
    }
}
/// `dedupeActivitiesByIdAfterAppend` — returns the deduped list (as Vec).
pub fn dedupe_activities_after_append(
    activities: &[JV],
    previous_activities: Option<&[JV]>,
    previous_by_id: Option<&Obj>,
) -> Vec<JV> {
    match (previous_activities, previous_by_id) {
        (Some(prev), Some(prev_by_id)) if !prev.is_empty() && prev.len() <= activities.len() => {
            for (index, prev_item) in prev.iter().enumerate() {
                if !eq(&activities[index], prev_item) {
                    return dedupe_activities_by_id(activities);
                }
            }
            let mut appended: Vec<JV> = Vec::new();
            for activity in &activities[prev.len()..] {
                let id = g(activity, "id");
                let id_key = pkey(&id);
                if prev_by_id.get(&id_key).is_some_and(|v| !matches!(v, JV::U))
                    || appended.iter().any(|x| eq(x, &id))
                {
                    return dedupe_activities_by_id(activities);
                }
                appended.push(id);
            }
            activities.to_vec()
        }
        _ => dedupe_activities_by_id(activities),
    }
}
fn resolve_turn_aligned_drop_count(activities: &[JV], minimum_drop_count: usize) -> usize {
    let boundary = activities
        .get(minimum_drop_count - 1)
        .map(|x| or(&g(x, "turnId"), JV::N))
        .unwrap_or(JV::N);
    let mut drop_count = minimum_drop_count;
    while drop_count < activities.len()
        && eq(&or(&g(&activities[drop_count], "turnId"), JV::N), &boundary)
    {
        drop_count += 1;
    }
    if drop_count >= activities.len() {
        minimum_drop_count
    } else {
        drop_count
    }
}
fn retain_task_activity_ids(activities: &[JV], retained: &mut Vec<JV>) -> R<()> {
    let mut latest: Vec<(String, JV)> = Vec::new();
    let latest_set = |latest: &mut Vec<(String, JV)>, key: String, activity: &JV| {
        if let Some(e) = latest.iter_mut().find(|(k, _)| *k == key) {
            e.1 = activity.clone();
        } else {
            latest.push((key, activity.clone()));
        }
    };
    let ordered = wl_ordered(&a(activities.to_vec()))?;
    for activity in &ordered {
        let kind = g(activity, "kind");
        if streq(&kind, "turn.tasks.updated") {
            latest_set(&mut latest, "task-list".into(), activity);
            continue;
        }
        let payload = g(activity, "payload");
        let prec = as_record(&payload);
        if streq(&kind, "runtime.warning")
            && streq_val(
                prec.and_then(|p| p.get("nativeEventType")),
                "background_tasks_changed",
            )
        {
            latest_set(&mut latest, "background-notice".into(), activity);
            continue;
        }
        let task_id = prec
            .and_then(|p| p.get("taskId"))
            .and_then(units)
            .map(|u| u.to_vec());
        let Some(task_id) = task_id else { continue };
        if streq(&kind, "task.started") {
            retained.push(g(activity, "id"));
            continue;
        }
        if !streq(&kind, "task.updated")
            && !streq(&kind, "task.progress")
            && !streq(&kind, "task.completed")
        {
            continue;
        }
        if !streq(&kind, "task.progress") {
            latest_set(
                &mut latest,
                json_stringify(&a(vec![su(task_id.clone()), s("lifecycle")])),
                activity,
            );
        }
        if let Some(p) = prec {
            for field in p.keys() {
                if field != "taskId" {
                    latest_set(
                        &mut latest,
                        json_stringify(&a(vec![su(task_id.clone()), s("field"), s(field)])),
                        activity,
                    );
                }
            }
        }
    }
    for (_, activity) in &latest {
        retained.push(g(activity, "id"));
    }
    Ok(())
}
fn pending_interaction_activity_identity(activity: &JV) -> Option<PendingReq> {
    let kind = g(activity, "kind");
    let ku = units(&kind)?;
    if !PENDING_ACTIVITY_KINDS.iter().any(|k| ueq(ku, k)) {
        return None;
    }
    let payload_jv = g(activity, "payload");
    let payload = as_record(&payload_jv);
    let request_id = payload
        .and_then(|p| p.get("requestId"))
        .and_then(units)
        .map(|u| u.to_vec())?;
    if str_trim(&request_id).is_empty() {
        return None;
    }
    let interaction_kind = if str_starts(ku, "approval.") || str_starts(ku, "provider.approval.") {
        "approval"
    } else {
        "userInput"
    };
    let generation = payload
        .and_then(|p| p.get("lifecycleGeneration"))
        .and_then(units)
        .map(|u| u.to_vec())
        .filter(|u| !u.is_empty());
    let mut key = interaction_kind.encode_utf16().collect::<Vec<u16>>();
    key.push(0x3a);
    key.extend(pending_request_instance_key(
        &request_id,
        generation.as_deref(),
    ));
    Some((
        interaction_kind.to_string(),
        request_id,
        generation,
        g(activity, "createdAt"),
        key,
    ))
}
/// Pending-interaction request identity: kind, requestId, generation, createdAt, key.
type PendingReq = (String, Vec<u16>, Option<Vec<u16>>, JV, Vec<u16>);
/// `pendingInteractionsFromActivities` -> open request identities.
fn pending_interactions_from_activities(activities: &[JV]) -> Vec<PendingReq> {
    let mut open: Vec<(String, PendingReq)> = Vec::new();
    for activity in activities {
        let kind = g(activity, "kind");
        let is_req = PENDING_REQUEST_KINDS.iter().any(|k| streq(&kind, k));
        if !is_req && !streq(&kind, "approval.resolved") && !streq(&kind, "user-input.resolved") {
            continue;
        }
        let Some(identity) = pending_interaction_activity_identity(activity) else {
            continue;
        };
        let key = format!("{}:{}", identity.0, String::from_utf16_lossy(&identity.1));
        if is_req {
            if let Some(e) = open.iter_mut().find(|(k, _)| *k == key) {
                e.1 = identity;
            } else {
                open.push((key, identity));
            }
            continue;
        }
        let resolved = open
            .iter()
            .position(|(k, _)| *k == key)
            .map(|i| open[i].1.2.clone());
        match (&identity.2, resolved) {
            (None, _) => {
                open.retain(|(k, _)| *k != key);
            }
            (Some(gen_), Some(open_gen)) if open_gen.as_ref() == Some(gen_) => {
                open.retain(|(k, _)| *k != key);
            }
            (Some(_), None) => {
                open.retain(|(k, _)| *k != key);
            }
            _ => {}
        }
    }
    if open.is_empty() {
        return Vec::new();
    }
    let stale = stale_matcher(activities);
    open.into_iter()
        .map(|(_, v)| v)
        .filter(|req| !stale(req.0.as_str(), req.1.as_slice(), &req.3, req.2.as_deref()))
        .collect()
}
/// `capThreadActivities`.
pub fn cap_thread_activities(activities: &[JV]) -> R<Vec<JV>> {
    if activities.len() <= MAX_THREAD_ACTIVITIES {
        return Ok(activities.to_vec());
    }
    let drop_count =
        resolve_turn_aligned_drop_count(activities, activities.len() - MAX_THREAD_ACTIVITIES);
    let tail = &activities[drop_count..];
    let mut retained: Vec<JV> = tail.iter().map(|x| g(x, "id")).collect();
    retain_task_activity_ids(activities, &mut retained)?;
    let mut retained_requests: Vec<(String, PendingReq)> = Vec::new();
    for r in pending_interactions_from_activities(activities) {
        let key = String::from_utf16_lossy(&r.4);
        if let Some(e) = retained_requests.iter_mut().find(|(k, _)| *k == key) {
            e.1 = r;
        } else {
            retained_requests.push((key, r));
        }
    }
    for activity in tail {
        let kind = g(activity, "kind");
        if !PENDING_REQUEST_KINDS.iter().any(|k| streq(&kind, k)) {
            continue;
        }
        if let Some(identity) = pending_interaction_activity_identity(activity) {
            let key = String::from_utf16_lossy(&identity.4);
            if let Some(e) = retained_requests.iter_mut().find(|(k, _)| *k == key) {
                e.1 = identity;
            } else {
                retained_requests.push((key, identity));
            }
        }
    }
    if retained_requests.is_empty() {
        return Ok(activities
            .iter()
            .filter(|x| retained.iter().any(|id| eq(id, &g(x, "id"))))
            .cloned()
            .collect());
    }
    // oldest retained request createdAt per `${kind}:${requestId}`
    let mut oldest_at: Vec<(String, JV)> = Vec::new();
    for (_, r) in &retained_requests {
        let key = format!("{}:{}", r.0, String::from_utf16_lossy(&r.1));
        match oldest_at.iter_mut().find(|(k, _)| *k == key) {
            Some((_, prev)) => {
                if js_lt(&r.3, prev) {
                    *prev = r.3.clone();
                }
            }
            None => oldest_at.push((key, r.3.clone())),
        }
    }
    let mut latest_evidence: Vec<(String, JV)> = Vec::new();
    for activity in activities {
        let Some(identity) = pending_interaction_activity_identity(activity) else {
            continue;
        };
        let kind = g(activity, "kind");
        let is_request = PENDING_REQUEST_KINDS.iter().any(|k| streq(&kind, k));
        let base_key = format!("{}:{}", identity.0, String::from_utf16_lossy(&identity.1));
        let oldest = oldest_at
            .iter()
            .find(|(k, _)| *k == base_key)
            .map(|(_, v)| v.clone());
        let id_key = String::from_utf16_lossy(&identity.4);
        let keep = if identity.2.is_some() {
            retained_requests.iter().any(|(k, _)| *k == id_key)
        } else if is_request {
            retained_requests
                .iter()
                .find(|(k, _)| *k == id_key)
                .is_some_and(|(_, r)| eq(&r.3, &identity.3))
        } else {
            oldest.as_ref().is_some_and(|o| js_le(o, &identity.3))
        };
        if !keep {
            continue;
        }
        if is_request {
            retained.push(g(activity, "id"));
            continue;
        }
        let detail = as_record(&g(activity, "payload"))
            .and_then(|p| p.get("detail"))
            .cloned();
        let stale = (streq(&kind, "provider.approval.respond.failed")
            || streq(&kind, "provider.user-input.respond.failed"))
            && is_stale_pending_request_failure_detail(&detail.unwrap_or(JV::U));
        let key = format!(
            "{}:{}{}",
            String::from_utf16_lossy(&identity.4),
            String::from_utf16_lossy(units(&kind).unwrap_or(&[])),
            if stale { ":stale" } else { "" }
        );
        match latest_evidence.iter_mut().find(|(k, _)| *k == key) {
            Some((_, prev)) => {
                if js_ge(&g(activity, "createdAt"), &g(prev, "createdAt")) {
                    *prev = activity.clone();
                }
            }
            None => latest_evidence.push((key, activity.clone())),
        }
    }
    for (_, activity) in &latest_evidence {
        retained.push(g(activity, "id"));
    }
    Ok(activities
        .iter()
        .filter(|x| retained.iter().any(|id| eq(id, &g(x, "id"))))
        .cloned()
        .collect())
}
/// `isNonFatalThreadErrorMessage`.
fn is_non_fatal_thread_error(message: &JV) -> bool {
    let Some(u) = units(message) else {
        return false;
    };
    if u.is_empty() {
        return false;
    }
    let n = str_lower(&str_trim(u));
    str_contains(&n, "write_stdin failed: stdin is closed for this session")
}
pub fn normalize_thread_error(message: &JV) -> JV {
    if truthy(message) && !is_non_fatal_thread_error(message) {
        message.clone()
    } else {
        JV::N
    }
}
fn to_legacy_session_status(status: &JV) -> JV {
    if streq(status, "starting") {
        s("connecting")
    } else if streq(status, "running") {
        s("running")
    } else if streq(status, "error") {
        s("error")
    } else if streq(status, "ready") || streq(status, "interrupted") {
        s("ready")
    } else {
        s("closed")
    }
}
pub fn to_legacy_provider(name: &JV) -> JV {
    for k in [
        "codex",
        "claudeAgent",
        "cursor",
        "antigravity",
        "grok",
        "droid",
        "opencode",
        "pi",
        "devin",
        "omp",
    ] {
        if streq(name, k) {
            return s(k);
        }
    }
    if streq(name, "kilo") {
        return s("opencode");
    }
    s("codex")
}
/// `normalizeThreadSession` -> JV (null or session object).
pub fn normalize_thread_session(incoming: &JV, previous: &JV) -> JV {
    if !truthy(incoming) {
        return JV::N;
    }
    let last_error = g(incoming, "lastError");
    let next_last_error = if truthy(&last_error) && !is_non_fatal_thread_error(&last_error) {
        Some(last_error)
    } else {
        None
    };
    let mut m = Obj::new();
    mset(
        &mut m,
        "provider",
        to_legacy_provider(&g(incoming, "providerName")),
    );
    let piid = g(incoming, "providerInstanceId");
    if !matches!(piid, JV::U) {
        mset(&mut m, "providerInstanceId", piid);
    }
    mset(
        &mut m,
        "status",
        to_legacy_session_status(&g(incoming, "status")),
    );
    mset(&mut m, "orchestrationStatus", g(incoming, "status"));
    mset(&mut m, "runtimeMode", g(incoming, "runtimeMode"));
    let atid = g(incoming, "activeTurnId");
    if !matches!(atid, JV::U | JV::N) {
        mset(&mut m, "activeTurnId", atid);
    }
    mset(&mut m, "createdAt", g(incoming, "updatedAt"));
    mset(&mut m, "updatedAt", g(incoming, "updatedAt"));
    if let Some(le) = next_last_error {
        mset(&mut m, "lastError", le);
    }
    let sess = o(m);
    if !is_nullish(previous) && thread_sessions_eq(previous, &sess) {
        return previous.clone();
    }
    sess
}
fn normalize_latest_turn(incoming: &JV, previous: &JV) -> JV {
    if !truthy(incoming) {
        return JV::N;
    }
    let incoming_spp = g(incoming, "sourceProposedPlan");
    let next_spp = if truthy(&incoming_spp) {
        let prev_spp = g(previous, "sourceProposedPlan");
        if !matches!(prev_spp, JV::U)
            && truthy(&prev_spp)
            && eq(&g(&prev_spp, "threadId"), &g(&incoming_spp, "threadId"))
            && eq(&g(&prev_spp, "planId"), &g(&incoming_spp, "planId"))
        {
            prev_spp
        } else {
            incoming_spp.clone()
        }
    } else {
        JV::U
    };
    if !is_nullish(previous)
        && eq(&g(previous, "turnId"), &g(incoming, "turnId"))
        && eq(&g(previous, "state"), &g(incoming, "state"))
        && eq(&g(previous, "requestedAt"), &g(incoming, "requestedAt"))
        && eq(&g(previous, "startedAt"), &g(incoming, "startedAt"))
        && eq(&g(previous, "completedAt"), &g(incoming, "completedAt"))
        && eq(
            &g(previous, "assistantMessageId"),
            &g(incoming, "assistantMessageId"),
        )
        && eq(&g(previous, "sourceProposedPlan"), &next_spp)
    {
        return previous.clone();
    }
    let mut m = Obj::new();
    mset(&mut m, "turnId", g(incoming, "turnId"));
    mset(&mut m, "state", g(incoming, "state"));
    mset(&mut m, "requestedAt", g(incoming, "requestedAt"));
    mset(&mut m, "startedAt", g(incoming, "startedAt"));
    mset(&mut m, "completedAt", g(incoming, "completedAt"));
    mset(
        &mut m,
        "assistantMessageId",
        g(incoming, "assistantMessageId"),
    );
    if truthy(&next_spp) {
        mset(&mut m, "sourceProposedPlan", next_spp);
    }
    o(m)
}
/// `resolveInitialLastVisitedAt` — persisted visited state is empty under Bun
/// (`window` undefined), so this reduces to `latestServerTime(thread)`.
fn resolve_initial_last_visited_at(thread: &JV, _restoring: bool) -> JV {
    let completed = or(&g(&g(thread, "latestTurn"), "completedAt"), JV::U);
    let completed = if matches!(completed, JV::N) {
        JV::U
    } else {
        completed
    };
    let updated = g(thread, "updatedAt");
    let completed_ms = date_parse(units(&completed).unwrap_or(&[]));
    let completed_ms = if completed_ms.is_nan() {
        0.0
    } else {
        completed_ms
    };
    let updated_ms = date_parse(units(&updated).unwrap_or(&[]));
    let updated_ms = if updated_ms.is_nan() { 0.0 } else { updated_ms };
    if completed_ms > updated_ms {
        completed
    } else {
        updated
    }
}

/// `normalizeProject`.
pub fn normalize_project(incoming: &JV, previous: &JV) -> R<JV> {
    let workspace_root_key =
        project_cwd_key(&gv(incoming, "workspaceRoot", "incoming.workspaceRoot")?)?;
    let basename = basename_of_path(
        units(&gv(incoming, "workspaceRoot", "incoming.workspaceRoot")?).unwrap_or(&[]),
    );
    let folder_name = if basename.is_empty() {
        gv(incoming, "title", "incoming.title")?
    } else {
        su(basename)
    };
    let prev_local_name = g(previous, "localName");
    let local_name = if !matches!(prev_local_name, JV::U) {
        prev_local_name
    } else {
        match name_for(&workspace_root_key) {
            Some(n) => su(n),
            None => JV::N,
        }
    };
    let prev_appearance = g(previous, "appearance");
    let appearance = if !matches!(prev_appearance, JV::U) {
        prev_appearance
    } else {
        or(&appearance_for(&workspace_root_key), JV::N)
    };
    let incoming_dms = gv(
        incoming,
        "defaultModelSelection",
        "incoming.defaultModelSelection",
    )?;
    let default_model_selection = if matches!(incoming_dms, JV::N) {
        JV::N
    } else {
        normalize_model_selection(&incoming_dms, &g(previous, "defaultModelSelection"))?
    };
    // normalizeProjectScripts
    let scripts_jv = gv(incoming, "scripts", "incoming.scripts")?;
    let incoming_scripts = arr_recv(
        &scripts_jv,
        "incoming.scripts",
        "map",
        "incoming.scripts.map((script, index) => ...)",
    )?;
    let prev_scripts = g(previous, "scripts");
    let prev_scripts_items: &[JV] = match &prev_scripts {
        JV::A(_, x) => x.as_ref(),
        _ => &[],
    };
    let next_scripts: Vec<JV> = incoming_scripts
        .iter()
        .enumerate()
        .map(|(i, script)| {
            let existing = prev_scripts_items.get(i).cloned().unwrap_or(JV::U);
            if !is_nullish(&existing) && deep_equal_json(&existing, script) {
                existing
            } else {
                script.clone()
            }
        })
        .collect();
    let scripts = if arrays_shallow_eq(&prev_scripts, &next_scripts) {
        prev_scripts.clone()
    } else {
        a(next_scripts)
    };
    let incoming_folders = or(
        &gv(incoming, "additionalFolders", "incoming.additionalFolders")?,
        a(vec![]),
    );
    let prev_folders = g(previous, "additionalFolders");
    let additional_folders =
        if !matches!(prev_folders, JV::U) && deep_equal_json(&prev_folders, &incoming_folders) {
            prev_folders
        } else {
            incoming_folders
        };
    let persisted_index = order_get(&workspace_root_key);
    let has_legacy = order_count() == 0 && (expanded_count() > 0 || is_legacy());
    let prev_cwd_key = if !matches!(previous, JV::U) {
        Some(project_cwd_key(&g(previous, "cwd"))?)
    } else {
        None
    };
    let prev_expanded = if prev_cwd_key.as_deref() == Some(workspace_root_key.as_str()) {
        g(previous, "expanded")
    } else {
        JV::U
    };
    let expanded = if !matches!(prev_expanded, JV::U) {
        prev_expanded
    } else if persisted_index.is_some() || has_legacy {
        b(persisted(|p| p.expanded.contains(&workspace_root_key)))
    } else {
        b(true)
    };
    let is_prev = !matches!(previous, JV::U);
    if is_prev
        && eq(&g(previous, "id"), &gv(incoming, "id", "incoming.id")?)
        && eq(
            &g(previous, "kind"),
            &gv(incoming, "kind", "incoming.kind")?,
        )
        && eq(
            &g(previous, "name"),
            &or(&local_name, gv(incoming, "title", "incoming.title")?),
        )
        && eq(
            &g(previous, "remoteName"),
            &gv(incoming, "title", "incoming.title")?,
        )
        && eq(&g(previous, "folderName"), &folder_name)
        && eq(&g(previous, "localName"), &local_name)
        && eq_or(&g(previous, "appearance"), &appearance)
        && eq(
            &g(previous, "cwd"),
            &gv(incoming, "workspaceRoot", "incoming.workspaceRoot")?,
        )
        && eq(
            &g(previous, "defaultModelSelection"),
            &default_model_selection,
        )
        && eq(&g(previous, "expanded"), &expanded)
        && eq_or(
            &g(previous, "isPinned"),
            &or(&gv(incoming, "isPinned", "incoming.isPinned")?, b(false)),
        )
        && eq_or(&g(previous, "spaceId"), &gv_or_null(incoming, "spaceId"))
        && eq(
            &g(previous, "createdAt"),
            &gv(incoming, "createdAt", "incoming.createdAt")?,
        )
        && eq(
            &g(previous, "updatedAt"),
            &gv(incoming, "updatedAt", "incoming.updatedAt")?,
        )
        && eq(&g(previous, "scripts"), &scripts)
        && eq(&g(previous, "additionalFolders"), &additional_folders)
    {
        return Ok(previous.clone());
    }
    Ok(op(&[
        ("id", gv(incoming, "id", "incoming.id")?),
        (
            "kind",
            or(&gv(incoming, "kind", "incoming.kind")?, s("project")),
        ),
        (
            "name",
            or(&local_name, gv(incoming, "title", "incoming.title")?),
        ),
        ("remoteName", gv(incoming, "title", "incoming.title")?),
        ("folderName", folder_name),
        ("localName", local_name),
        ("appearance", appearance),
        (
            "cwd",
            gv(incoming, "workspaceRoot", "incoming.workspaceRoot")?,
        ),
        ("defaultModelSelection", default_model_selection),
        ("expanded", expanded),
        (
            "isPinned",
            or(&gv(incoming, "isPinned", "incoming.isPinned")?, b(false)),
        ),
        (
            "spaceId",
            or(&gv(incoming, "spaceId", "incoming.spaceId")?, JV::N),
        ),
        (
            "createdAt",
            gv(incoming, "createdAt", "incoming.createdAt")?,
        ),
        (
            "updatedAt",
            gv(incoming, "updatedAt", "incoming.updatedAt")?,
        ),
        ("scripts", scripts),
        ("additionalFolders", additional_folders),
    ]))
}
/// `normalizeSpace`.
pub fn normalize_space(incoming: &JV, previous: &JV) -> R<JV> {
    if !matches!(previous, JV::U)
        && eq(&g(previous, "id"), &gv(incoming, "id", "incoming.id")?)
        && eq(
            &g(previous, "name"),
            &gv(incoming, "name", "incoming.name")?,
        )
        && eq(
            &g(previous, "icon"),
            &gv(incoming, "icon", "incoming.icon")?,
        )
        && eq(
            &g(previous, "sortOrder"),
            &gv(incoming, "sortOrder", "incoming.sortOrder")?,
        )
        && eq(
            &g(previous, "createdAt"),
            &gv(incoming, "createdAt", "incoming.createdAt")?,
        )
        && eq(
            &g(previous, "updatedAt"),
            &gv(incoming, "updatedAt", "incoming.updatedAt")?,
        )
    {
        return Ok(previous.clone());
    }
    Ok(op(&[
        ("id", gv(incoming, "id", "incoming.id")?),
        ("name", gv(incoming, "name", "incoming.name")?),
        ("icon", gv(incoming, "icon", "incoming.icon")?),
        (
            "sortOrder",
            gv(incoming, "sortOrder", "incoming.sortOrder")?,
        ),
        (
            "createdAt",
            gv(incoming, "createdAt", "incoming.createdAt")?,
        ),
        (
            "updatedAt",
            gv(incoming, "updatedAt", "incoming.updatedAt")?,
        ),
    ]))
}
/// space sort: `left.sortOrder - right.sortOrder || left.id.localeCompare(right.id)`
/// (a `NaN` difference is falsy and falls through to `localeCompare`).
fn space_cmp(l: &JV, r: &JV) -> R<f64> {
    let d = to_num(&g(l, "sortOrder")) - to_num(&g(r, "sortOrder"));
    if d != 0.0 && !d.is_nan() {
        return Ok(d);
    }
    locale_compare(
        &g(l, "id"),
        &g(r, "id"),
        "left.id",
        "left.id.localeCompare(right.id)",
    )
}
fn to_sorted(items: &[JV], cmp: impl Fn(&JV, &JV) -> R<f64>) -> R<Vec<JV>> {
    let mut out = items.to_vec();
    let mut err = None;
    js_sort(&mut out, |x, y| match cmp(x, y) {
        Ok(v) => v,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            f64::NAN
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(out),
    }
}
pub fn map_spaces(incoming: &[JV], previous: &[JV]) -> R<Vec<JV>> {
    let next: Vec<JV> = incoming
        .iter()
        .map(|space| {
            normalize_space(
                space,
                &previous
                    .iter()
                    .find(|p| eq(&g(p, "id"), &g(space, "id")))
                    .cloned()
                    .unwrap_or(JV::U),
            )
        })
        .collect::<R<Vec<JV>>>()?;
    to_sorted(&next, space_cmp)
}
/// `mapProjects`.
pub fn map_projects(incoming: &[JV], previous: &[JV]) -> R<Vec<JV>> {
    let prev_by_id: Vec<(JV, JV)> = previous.iter().map(|p| (g(p, "id"), p.clone())).collect();
    let mut prev_by_cwd: Vec<(String, JV)> = Vec::new();
    for p in previous {
        prev_by_cwd.push((project_cwd_key(&g(p, "cwd"))?, p.clone()));
    }
    let prev_order_id: Vec<(JV, usize)> = previous
        .iter()
        .enumerate()
        .map(|(i, p)| (g(p, "id"), i))
        .collect();
    let prev_order_cwd: Vec<(String, usize)> = previous
        .iter()
        .enumerate()
        .map(|(i, p)| Ok((project_cwd_key(&g(p, "cwd"))?, i)))
        .collect::<R<_>>()?;
    let use_persisted = previous.is_empty();
    let mut mapped: Vec<(JV, usize, f64)> = Vec::new();
    for (incoming_index, project) in incoming.iter().enumerate() {
        let prev_same_id = prev_by_id
            .iter()
            .find(|(id, _)| eq(id, &g(project, "id")))
            .map(|(_, p)| p.clone());
        let existing = match prev_same_id {
            Some(p) => p,
            None => {
                let key = project_cwd_key(&g(project, "workspaceRoot"))?;
                prev_by_cwd
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, p)| p.clone())
                    .unwrap_or(JV::U)
            }
        };
        let normalized = normalize_project(project, &existing)?;
        let prev_index = prev_order_id
            .iter()
            .find(|(id, _)| eq(id, &g(&normalized, "id")))
            .map(|(_, i)| *i)
            .map(|i| i as f64);
        let prev_index = match prev_index {
            Some(i) => Some(i),
            None => {
                let key = project_cwd_key(&g(&normalized, "cwd"))?;
                prev_order_cwd
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, i)| *i as f64)
            }
        };
        let persisted_index = if use_persisted {
            order_get(&project_cwd_key(&g(&normalized, "cwd"))?)
        } else {
            None
        };
        let base = if use_persisted {
            order_count() as f64
        } else {
            previous.len() as f64
        };
        let order_index = prev_index
            .or(persisted_index)
            .unwrap_or(base + incoming_index as f64);
        mapped.push((normalized, incoming_index, order_index));
    }
    sort_t(&mut mapped, |x, y| {
        let by_order = x.2 - y.2;
        if by_order != 0.0 && !by_order.is_nan() {
            return by_order;
        }
        x.1 as f64 - y.1 as f64
    });
    Ok(mapped.into_iter().map(|(p, _, _)| p).collect())
}
/// Generic stable sort with JS comparator semantics (`NaN` -> equal).
pub fn sort_t<T>(items: &mut [T], mut cmp: impl FnMut(&T, &T) -> f64) {
    items.sort_by(|a, b| {
        let v = cmp(a, b);
        if v.is_nan() {
            std::cmp::Ordering::Equal
        } else if v < 0.0 {
            std::cmp::Ordering::Less
        } else if v > 0.0 {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    });
}
/// `normalizeChatAttachments`.
fn normalize_chat_attachments(incoming: &JV, previous: &JV) -> R<JV> {
    let items = match incoming {
        JV::A(_, t) if !t.is_empty() => t.as_ref(),
        _ => return Ok(JV::U),
    };
    let prev_items: &[JV] = match previous {
        JV::A(_, t) => t.as_ref(),
        _ => &[],
    };
    let mut next: Vec<JV> = Vec::new();
    for attachment in items {
        let ty = g(attachment, "type");
        let next_attachment = if streq(&ty, "assistant-selection") {
            op(&[
                ("type", s("assistant-selection")),
                ("id", g(attachment, "id")),
                ("assistantMessageId", g(attachment, "assistantMessageId")),
                ("text", g(attachment, "text")),
            ])
        } else if streq(&ty, "file") {
            op(&[
                ("type", s("file")),
                ("id", g(attachment, "id")),
                ("name", g(attachment, "name")),
                ("mimeType", g(attachment, "mimeType")),
                ("sizeBytes", g(attachment, "sizeBytes")),
            ])
        } else {
            let mut m = Obj::new();
            mset(&mut m, "type", s("image"));
            let sp = g(attachment, "sourcePath");
            if !matches!(sp, JV::U) {
                mset(&mut m, "sourcePath", sp);
            }
            mset(&mut m, "id", g(attachment, "id"));
            mset(&mut m, "name", g(attachment, "name"));
            mset(&mut m, "mimeType", g(attachment, "mimeType"));
            mset(&mut m, "sizeBytes", g(attachment, "sizeBytes"));
            let aid = g(attachment, "id");
            let mut path = "/attachments/".encode_utf16().collect::<Vec<u16>>();
            path.extend(encode_uri_component(units(&aid).unwrap_or(&[])));
            mset(&mut m, "previewUrl", su(path));
            o(m)
        };
        let existing = prev_items
            .iter()
            .find(|p| eq(&g(p, "id"), &g(attachment, "id")))
            .cloned()
            .unwrap_or(JV::U);
        let keep = {
            let ety = g(&existing, "type");
            let nty = g(&next_attachment, "type");
            (streq(&ety, "assistant-selection")
                && streq(&nty, "assistant-selection")
                && eq(
                    &g(&existing, "assistantMessageId"),
                    &g(&next_attachment, "assistantMessageId"),
                )
                && eq(&g(&existing, "text"), &g(&next_attachment, "text")))
                || (streq(&ety, "image")
                    && streq(&nty, "image")
                    && eq(&g(&existing, "name"), &g(&next_attachment, "name"))
                    && eq(&g(&existing, "mimeType"), &g(&next_attachment, "mimeType"))
                    && eq(
                        &g(&existing, "sizeBytes"),
                        &g(&next_attachment, "sizeBytes"),
                    )
                    && eq(
                        &g(&existing, "sourcePath"),
                        &g(&next_attachment, "sourcePath"),
                    )
                    && eq(
                        &g(&existing, "previewUrl"),
                        &g(&next_attachment, "previewUrl"),
                    ))
                || (streq(&ety, "file")
                    && streq(&nty, "file")
                    && eq(&g(&existing, "name"), &g(&next_attachment, "name"))
                    && eq(&g(&existing, "mimeType"), &g(&next_attachment, "mimeType"))
                    && eq(
                        &g(&existing, "sizeBytes"),
                        &g(&next_attachment, "sizeBytes"),
                    ))
        };
        next.push(if keep { existing } else { next_attachment });
    }
    if arrays_shallow_eq(previous, &next) {
        Ok(previous.clone())
    } else {
        Ok(a(next))
    }
}
/// `normalizeChatMessage`.
pub fn normalize_chat_message(incoming: &JV, previous: &JV) -> R<JV> {
    let attachments =
        normalize_chat_attachments(&g(incoming, "attachments"), &g(previous, "attachments"))?;
    let incoming_skills = g(incoming, "skills");
    let skills = match &incoming_skills {
        JV::A(_, t) if !t.is_empty() => incoming_skills.clone(),
        _ => or(&g(previous, "skills"), a(vec![])),
    };
    let incoming_mentions = g(incoming, "mentions");
    let mentions = match &incoming_mentions {
        JV::A(_, t) if !t.is_empty() => incoming_mentions.clone(),
        _ => or(&g(previous, "mentions"), a(vec![])),
    };
    let prev_skills: Vec<JV> = match &g(previous, "skills") {
        JV::A(_, t) => t.as_ref().clone(),
        _ => vec![],
    };
    let prev_mentions: Vec<JV> = match &g(previous, "mentions") {
        JV::A(_, t) => t.as_ref().clone(),
        _ => vec![],
    };
    let completed_at = if truthy(&g(incoming, "streaming")) {
        JV::U
    } else {
        g(incoming, "updatedAt")
    };
    let async_user_input = merge_async_user_input(
        &g(previous, "asyncUserInput"),
        &g(incoming, "asyncUserInput"),
    );
    if !matches!(previous, JV::U)
        && eq(&g(previous, "role"), &g(incoming, "role"))
        && eq(&g(previous, "text"), &g(incoming, "text"))
        && eq(&g(previous, "asyncUserInput"), &async_user_input)
        && eq(&g(previous, "dispatchMode"), &g(incoming, "dispatchMode"))
        && eq(
            &g(previous, "dispatchOrigin"),
            &g(incoming, "dispatchOrigin"),
        )
        && eq(&g(previous, "startsNewTurn"), &g(incoming, "startsNewTurn"))
        && eq(&g(previous, "turnId"), &g(incoming, "turnId"))
        && eq(&g(previous, "createdAt"), &g(incoming, "createdAt"))
        && eq(&g(previous, "updatedAt"), &g(incoming, "updatedAt"))
        && eq(&g(previous, "streaming"), &g(incoming, "streaming"))
        && eq(&g(previous, "source"), &g(incoming, "source"))
        && eq(&g(previous, "completedAt"), &completed_at)
        && eq(&g(previous, "attachments"), &attachments)
        && text_segment_arrays_eq(&g(previous, "textSegments"), &g(incoming, "textSegments"))
        && provider_ref_arrays_eq(&prev_skills, units_arr(&skills).as_slice())
        && provider_ref_arrays_eq(&prev_mentions, units_arr(&mentions).as_slice())
    {
        return Ok(previous.clone());
    }
    let mut m = Obj::new();
    mset(&mut m, "id", g(incoming, "id"));
    mset(&mut m, "role", g(incoming, "role"));
    mset(&mut m, "text", g(incoming, "text"));
    if truthy(&async_user_input) {
        mset(&mut m, "asyncUserInput", async_user_input);
    }
    let ts = g(incoming, "textSegments");
    if let JV::A(_, t) = &ts
        && !t.is_empty()
    {
        mset(&mut m, "textSegments", a(t.as_ref().clone()));
    }
    let dm = g(incoming, "dispatchMode");
    if truthy(&dm) {
        mset(&mut m, "dispatchMode", dm);
    }
    let dorigin = g(incoming, "dispatchOrigin");
    if truthy(&dorigin) {
        mset(&mut m, "dispatchOrigin", dorigin);
    }
    let snt = g(incoming, "startsNewTurn");
    if !matches!(snt, JV::U) {
        mset(&mut m, "startsNewTurn", snt);
    }
    mset(&mut m, "turnId", g(incoming, "turnId"));
    mset(&mut m, "createdAt", g(incoming, "createdAt"));
    mset(&mut m, "updatedAt", g(incoming, "updatedAt"));
    mset(&mut m, "streaming", g(incoming, "streaming"));
    mset(&mut m, "source", g(incoming, "source"));
    if truthy(&completed_at) {
        mset(&mut m, "completedAt", completed_at);
    }
    if !matches!(attachments, JV::U) {
        mset(&mut m, "attachments", attachments);
    }
    if let JV::A(_, t) = &skills
        && !t.is_empty()
    {
        mset(&mut m, "skills", a(t.as_ref().clone()));
    }
    if let JV::A(_, t) = &mentions
        && !t.is_empty()
    {
        mset(&mut m, "mentions", a(t.as_ref().clone()));
    }
    Ok(o(m))
}
pub fn units_arr(v: &JV) -> Vec<JV> {
    match v {
        JV::A(_, t) => t.as_ref().clone(),
        _ => vec![],
    }
}
/// `normalizeChatMessages` — `incoming.slice(-2000).map(...)`.
pub fn normalize_chat_messages(incoming: &[JV], previous: &JV) -> R<JV> {
    let prev_by_id: Vec<(JV, JV)> = units_arr(previous)
        .iter()
        .map(|m| (g(m, "id"), m.clone()))
        .collect();
    let start = incoming.len().saturating_sub(MAX_THREAD_MESSAGES);
    let mut next: Vec<JV> = Vec::new();
    for message in &incoming[start..] {
        let existing = prev_by_id
            .iter()
            .find(|(id, _)| eq(id, &g(message, "id")))
            .map(|(_, m)| m.clone())
            .unwrap_or(JV::U);
        next.push(normalize_chat_message(message, &existing)?);
    }
    if arrays_shallow_eq(previous, &next) {
        Ok(previous.clone())
    } else {
        Ok(a(next))
    }
}
/// `normalizeProposedPlans`.
pub fn normalize_proposed_plans(incoming: &[JV], previous: &JV) -> JV {
    let prev_by_id: Vec<(JV, JV)> = units_arr(previous)
        .iter()
        .map(|p| (g(p, "id"), p.clone()))
        .collect();
    let mut next: Vec<JV> = Vec::new();
    for plan in incoming {
        let existing = prev_by_id
            .iter()
            .find(|(id, _)| eq(id, &g(plan, "id")))
            .map(|(_, p)| p.clone())
            .unwrap_or(JV::U);
        if !matches!(existing, JV::U)
            && eq(&g(&existing, "turnId"), &g(plan, "turnId"))
            && eq(&g(&existing, "planMarkdown"), &g(plan, "planMarkdown"))
            && eq(&g(&existing, "implementedAt"), &g(plan, "implementedAt"))
            && eq(
                &g(&existing, "implementationThreadId"),
                &g(plan, "implementationThreadId"),
            )
            && eq(&g(&existing, "createdAt"), &g(plan, "createdAt"))
            && eq(&g(&existing, "updatedAt"), &g(plan, "updatedAt"))
        {
            next.push(existing);
            continue;
        }
        next.push(op(&[
            ("id", g(plan, "id")),
            ("turnId", g(plan, "turnId")),
            ("planMarkdown", g(plan, "planMarkdown")),
            ("implementedAt", g(plan, "implementedAt")),
            ("implementationThreadId", g(plan, "implementationThreadId")),
            ("createdAt", g(plan, "createdAt")),
            ("updatedAt", g(plan, "updatedAt")),
        ]));
    }
    if arrays_shallow_eq(previous, &next) {
        previous.clone()
    } else {
        a(next)
    }
}
/// `normalizeTurnDiffFiles`.
fn merge_turn_diff_files(files: &[JV]) -> Vec<JV> {
    let mut by_path: Vec<(JV, JV)> = Vec::new();
    for file in files {
        let path = g(file, "path");
        if let Some((_, existing)) = by_path.iter_mut().find(|(p, _)| eq(p, &path)) {
            let adds = js_add(
                &or(&g(existing, "additions"), n(0.0)),
                &or(&g(file, "additions"), n(0.0)),
            );
            let dels = js_add(
                &or(&g(existing, "deletions"), n(0.0)),
                &or(&g(file, "deletions"), n(0.0)),
            );
            *existing = op(&[
                ("path", path.clone()),
                ("kind", g(existing, "kind")),
                ("additions", adds),
                ("deletions", dels),
            ]);
        } else {
            by_path.push((path, file.clone()));
        }
    }
    by_path.into_iter().map(|(_, f)| f).collect()
}
fn normalize_turn_diff_files(incoming: &[JV], previous: &JV) -> JV {
    let merged = merge_turn_diff_files(incoming);
    let prev_items = units_arr(previous);
    let next: Vec<JV> = merged
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let existing = prev_items.get(i).cloned().unwrap_or(JV::U);
            if !matches!(existing, JV::U)
                && eq(&g(&existing, "path"), &g(file, "path"))
                && eq(&g(&existing, "kind"), &g(file, "kind"))
                && eq(&g(&existing, "additions"), &g(file, "additions"))
                && eq(&g(&existing, "deletions"), &g(file, "deletions"))
            {
                existing
            } else {
                file.clone()
            }
        })
        .collect();
    if arrays_shallow_eq(previous, &next) {
        previous.clone()
    } else {
        a(next)
    }
}
/// `normalizeTurnDiffSummaries` from `incoming.checkpoints`.
pub fn normalize_turn_diff_summaries(incoming: &[JV], previous: &JV) -> JV {
    let prev_by_turn: Vec<(JV, JV)> = units_arr(previous)
        .iter()
        .map(|x| (g(x, "turnId"), x.clone()))
        .collect();
    let mut next: Vec<JV> = Vec::new();
    for checkpoint in incoming {
        let existing = prev_by_turn
            .iter()
            .find(|(t, _)| eq(t, &g(checkpoint, "turnId")))
            .map(|(_, x)| x.clone())
            .unwrap_or(JV::U);
        let files =
            normalize_turn_diff_files(&units_arr(&g(checkpoint, "files")), &g(&existing, "files"));
        let started = or(&g(checkpoint, "startedAt"), g(&existing, "startedAt"));
        if !matches!(existing, JV::U)
            && eq(&g(&existing, "startedAt"), &started)
            && eq(&g(&existing, "completedAt"), &g(checkpoint, "completedAt"))
            && eq(&g(&existing, "status"), &g(checkpoint, "status"))
            && eq(
                &g(&existing, "assistantMessageId"),
                &or(&g(checkpoint, "assistantMessageId"), JV::U),
            )
            && eq(
                &g(&existing, "checkpointTurnCount"),
                &g(checkpoint, "checkpointTurnCount"),
            )
            && eq(
                &g(&existing, "checkpointRef"),
                &g(checkpoint, "checkpointRef"),
            )
            && eq(&g(&existing, "files"), &files)
        {
            next.push(existing);
            continue;
        }
        let mut m = Obj::new();
        mset(&mut m, "turnId", g(checkpoint, "turnId"));
        if truthy(&started) {
            mset(&mut m, "startedAt", started);
        }
        mset(&mut m, "completedAt", g(checkpoint, "completedAt"));
        mset(&mut m, "status", g(checkpoint, "status"));
        let amid = g(checkpoint, "assistantMessageId");
        if !matches!(amid, JV::N) {
            mset(&mut m, "assistantMessageId", amid);
        }
        mset(
            &mut m,
            "checkpointTurnCount",
            g(checkpoint, "checkpointTurnCount"),
        );
        mset(&mut m, "checkpointRef", g(checkpoint, "checkpointRef"));
        mset(&mut m, "files", files);
        next.push(o(m));
    }
    if arrays_shallow_eq(previous, &next) {
        previous.clone()
    } else {
        a(next)
    }
}
/// `normalizeActivities`.
pub fn normalize_activities(incoming: &[JV], previous: &JV) -> R<JV> {
    let prev_items = units_arr(previous);
    let prev_deduped = if matches!(previous, JV::A(..)) {
        Some(dedupe_activities_by_id(&prev_items))
    } else {
        None
    };
    let incoming_deduped = dedupe_activities_by_id(incoming);
    let prev_by_id: Vec<(JV, JV)> = prev_deduped
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|x| (g(x, "id"), x.clone()))
        .collect();
    let mut next: Vec<JV> = Vec::new();
    for activity in &incoming_deduped {
        let existing = prev_by_id
            .iter()
            .find(|(id, _)| eq(id, &g(activity, "id")))
            .map(|(_, x)| x.clone());
        if let Some(existing) = existing {
            let preferred = prefer_richer_activity(&existing, activity);
            if eq(&preferred, &existing) || activities_equal(&existing, &preferred) {
                next.push(existing);
            } else {
                next.push(preferred);
            }
        } else {
            next.push(activity.clone());
        }
    }
    let capped = cap_thread_activities(&next)?;
    if prev_items.len() == capped.len() && prev_items.iter().zip(&capped).all(|(x, y)| eq(x, y)) {
        Ok(previous.clone())
    } else {
        Ok(a(capped))
    }
}

// ---------- hot-path merge (mergeReadModelThreadDetailWithLiveHotPath) ----------

fn should_retain_live_assistant(previous_thread: &JV, message: &JV) -> bool {
    if truthy(&g(message, "streaming")) {
        return true;
    }
    let latest_turn = g(previous_thread, "latestTurn");
    if !truthy(&latest_turn) {
        return false;
    }
    if eq(&g(&latest_turn, "assistantMessageId"), &g(message, "id")) {
        return true;
    }
    streq(
        &g(&g(previous_thread, "session"), "orchestrationStatus"),
        "running",
    ) && !matches!(g(message, "turnId"), JV::U)
        && eq(&g(&latest_turn, "turnId"), &g(message, "turnId"))
}
const LOCAL_USER_MESSAGE_RETENTION_MS: f64 = 10_000.0;
fn should_retain_live_user(previous_thread: &JV, message: &JV) -> bool {
    let latest_turn = g(previous_thread, "latestTurn");
    if truthy(&latest_turn)
        && !is_nullish(&g(message, "turnId"))
        && eq(&g(message, "turnId"), &g(&latest_turn, "turnId"))
    {
        return is_session_or_latest_turn_running(
            &g(&g(previous_thread, "session"), "orchestrationStatus"),
            &g(&latest_turn, "state"),
        );
    }
    let created_ms = date_parse(units(&g(message, "createdAt")).unwrap_or(&[]));
    created_ms.is_finite() && date_now() - created_ms <= LOCAL_USER_MESSAGE_RETENTION_MS
}
fn should_retain_live_message(previous_thread: &JV, message: &JV) -> bool {
    if streq(&g(message, "role"), "assistant") {
        should_retain_live_assistant(previous_thread, message)
    } else if streq(&g(message, "role"), "user") {
        should_retain_live_user(previous_thread, message)
    } else {
        false
    }
}
fn read_model_attachments_from_chat(attachments: &JV) -> JV {
    let items = units_arr(attachments);
    let mut out: Vec<JV> = Vec::new();
    for at in &items {
        let ty = g(at, "type");
        if streq(&ty, "assistant-selection") {
            out.push(op(&[
                ("id", g(at, "id")),
                ("type", s("assistant-selection")),
                ("assistantMessageId", g(at, "assistantMessageId")),
                ("text", g(at, "text")),
            ]));
        } else if streq(&ty, "file") {
            out.push(op(&[
                ("id", g(at, "id")),
                ("name", g(at, "name")),
                ("type", s("file")),
                ("mimeType", g(at, "mimeType")),
                ("sizeBytes", g(at, "sizeBytes")),
            ]));
        } else {
            let mut m = Obj::new();
            mset(&mut m, "id", g(at, "id"));
            mset(&mut m, "name", g(at, "name"));
            mset(&mut m, "type", s("image"));
            let sp = g(at, "sourcePath");
            if !matches!(sp, JV::U) {
                mset(&mut m, "sourcePath", sp);
            }
            mset(&mut m, "mimeType", g(at, "mimeType"));
            mset(&mut m, "sizeBytes", g(at, "sizeBytes"));
            out.push(o(m));
        }
    }
    a(out)
}
fn read_model_message_from_chat(message: &JV) -> JV {
    let mut m = Obj::new();
    mset(&mut m, "id", g(message, "id"));
    mset(&mut m, "role", g(message, "role"));
    mset(&mut m, "text", g(message, "text"));
    let aui = g(message, "asyncUserInput");
    if truthy(&aui) {
        mset(&mut m, "asyncUserInput", aui);
    }
    let dm = g(message, "dispatchMode");
    if truthy(&dm) {
        mset(&mut m, "dispatchMode", dm);
    }
    let dorigin = g(message, "dispatchOrigin");
    if truthy(&dorigin) {
        mset(&mut m, "dispatchOrigin", dorigin);
    }
    let snt = g(message, "startsNewTurn");
    if !matches!(snt, JV::U) {
        mset(&mut m, "startsNewTurn", snt);
    }
    mset(&mut m, "turnId", gv_or_null(message, "turnId"));
    mset(&mut m, "streaming", g(message, "streaming"));
    mset(&mut m, "source", or(&g(message, "source"), s("native")));
    mset(&mut m, "createdAt", g(message, "createdAt"));
    mset(
        &mut m,
        "updatedAt",
        or(
            &g(message, "updatedAt"),
            or(&g(message, "completedAt"), g(message, "createdAt")),
        ),
    );
    mset(
        &mut m,
        "attachments",
        read_model_attachments_from_chat(&g(message, "attachments")),
    );
    let skills = g(message, "skills");
    if let JV::A(_, t) = &skills
        && !t.is_empty()
    {
        mset(&mut m, "skills", skills);
    }
    let mentions = g(message, "mentions");
    if let JV::A(_, t) = &mentions
        && !t.is_empty()
    {
        mset(&mut m, "mentions", mentions);
    }
    o(m)
}
fn text_segments_cover(segments: &JV, text: &JV) -> bool {
    let JV::A(_, segs) = segments else {
        return false;
    };
    let mut collated: Vec<u16> = Vec::new();
    for seg in segs.iter() {
        if let Some(u) = units(&g(seg, "text")) {
            collated.extend(u.iter().copied());
        }
    }
    units(text).is_some_and(|t| t == collated.as_slice())
}
fn merge_read_model_messages_hot_path(
    incoming_messages: &[JV],
    previous_thread: &JV,
    authoritative_turn_id: &JV,
) -> R<JV> {
    let prev_messages = units_arr(&g(previous_thread, "messages"));
    if matches!(previous_thread, JV::U) || prev_messages.is_empty() {
        return Ok(a(incoming_messages.to_vec()));
    }
    let prev_by_id: Vec<(JV, JV)> = prev_messages
        .iter()
        .map(|m| (g(m, "id"), m.clone()))
        .collect();
    let mut merged: Vec<(JV, JV)> = Vec::new(); // (id, message) insertion-ordered map
    let mut changed = false;
    let merged_set = |merged: &mut Vec<(JV, JV)>, id: JV, msg: JV| {
        if let Some(e) = merged.iter_mut().find(|(k, _)| eq(k, &id)) {
            e.1 = msg;
        } else {
            merged.push((id, msg));
        }
    };
    for incoming_message in incoming_messages {
        let previous_message = prev_by_id
            .iter()
            .find(|(id, _)| eq(id, &g(incoming_message, "id")))
            .map(|(_, m)| m.clone());
        match previous_message {
            None => {
                merged_set(
                    &mut merged,
                    g(incoming_message, "id"),
                    incoming_message.clone(),
                );
                continue;
            }
            Some(prev) if !eq(&g(&prev, "role"), &g(incoming_message, "role")) => {
                merged_set(
                    &mut merged,
                    g(incoming_message, "id"),
                    incoming_message.clone(),
                );
                continue;
            }
            Some(prev) => {
                let incoming_completed_at = if truthy(&g(incoming_message, "streaming")) {
                    JV::U
                } else {
                    g(incoming_message, "updatedAt")
                };
                let prev_text_len = units(&g(&prev, "text")).map(|u| u.len()).unwrap_or(0);
                let inc_text_len = units(&g(incoming_message, "text"))
                    .map(|u| u.len())
                    .unwrap_or(0);
                let local_ahead = truthy(&g(&prev, "streaming")) && prev_text_len > inc_text_len;
                let prev_completed = g(&prev, "completedAt");
                let should_prefer = (matches!(authoritative_turn_id, JV::N)
                    || !eq(&g(incoming_message, "turnId"), authoritative_turn_id))
                    && (local_ahead
                        || (!truthy(&g(&prev, "streaming"))
                            && truthy(&g(incoming_message, "streaming")))
                        || (!matches!(prev_completed, JV::U)
                            && (matches!(incoming_completed_at, JV::U)
                                || js_gt(&prev_completed, &incoming_completed_at))));
                if !should_prefer {
                    let mut m = spread_map(incoming_message);
                    let im = g(incoming_message, "mentions");
                    let pm = g(&prev, "mentions");
                    if (matches!(&im, JV::U) || matches!(&im, JV::A(_, t) if t.is_empty()))
                        && let JV::A(_, t) = &pm
                        && !t.is_empty()
                    {
                        mset(&mut m, "mentions", pm);
                    }
                    let isk = g(incoming_message, "skills");
                    let psk = g(&prev, "skills");
                    if (matches!(&isk, JV::U) || matches!(&isk, JV::A(_, t) if t.is_empty()))
                        && let JV::A(_, t) = &psk
                        && !t.is_empty()
                    {
                        mset(&mut m, "skills", psk);
                    }
                    merged_set(&mut merged, g(incoming_message, "id"), o(m));
                    continue;
                }
                changed = true;
                let mut m = spread_map(incoming_message);
                m.remove("textSegments");
                let retained_segments = [
                    &g(incoming_message, "textSegments"),
                    &g(&prev, "textSegments"),
                ]
                .into_iter()
                .find(|seg| {
                    if let JV::A(_, t) = seg {
                        !t.is_empty() && text_segments_cover(seg, &g(&prev, "text"))
                    } else {
                        false
                    }
                })
                .cloned();
                if let Some(segs) = retained_segments {
                    mset(&mut m, "textSegments", segs);
                }
                mset(&mut m, "text", g(&prev, "text"));
                mset(
                    &mut m,
                    "dispatchMode",
                    or(
                        &g(&prev, "dispatchMode"),
                        g(incoming_message, "dispatchMode"),
                    ),
                );
                mset(
                    &mut m,
                    "dispatchOrigin",
                    or(
                        &g(incoming_message, "dispatchOrigin"),
                        g(&prev, "dispatchOrigin"),
                    ),
                );
                mset(
                    &mut m,
                    "startsNewTurn",
                    or(
                        &g(incoming_message, "startsNewTurn"),
                        g(&prev, "startsNewTurn"),
                    ),
                );
                mset(
                    &mut m,
                    "asyncUserInput",
                    merge_async_user_input(
                        &g(&prev, "asyncUserInput"),
                        &g(incoming_message, "asyncUserInput"),
                    ),
                );
                mset(
                    &mut m,
                    "turnId",
                    or(&g(&prev, "turnId"), gv_or_null(incoming_message, "turnId")),
                );
                mset(
                    &mut m,
                    "source",
                    or(
                        &g(&prev, "source"),
                        or(&g(incoming_message, "source"), s("native")),
                    ),
                );
                mset(&mut m, "streaming", g(&prev, "streaming"));
                mset(
                    &mut m,
                    "updatedAt",
                    or(
                        &g(&prev, "updatedAt"),
                        or(&g(&prev, "completedAt"), g(incoming_message, "updatedAt")),
                    ),
                );
                mset(
                    &mut m,
                    "attachments",
                    read_model_attachments_from_chat(&g(&prev, "attachments")),
                );
                let psk = g(&prev, "skills");
                if let JV::A(_, t) = &psk {
                    if !t.is_empty() {
                        mset(&mut m, "skills", psk);
                    }
                } else {
                    m.remove("skills");
                }
                let pm = g(&prev, "mentions");
                if let JV::A(_, t) = &pm {
                    if !t.is_empty() {
                        mset(&mut m, "mentions", pm);
                    }
                } else {
                    m.remove("mentions");
                }
                merged_set(&mut merged, g(incoming_message, "id"), o(m));
            }
        }
    }
    for previous_message in &prev_messages {
        if merged
            .iter()
            .any(|(id, _)| eq(id, &g(previous_message, "id")))
        {
            continue;
        }
        if !should_retain_live_message(previous_thread, previous_message) {
            continue;
        }
        changed = true;
        merged_set(
            &mut merged,
            g(previous_message, "id"),
            read_model_message_from_chat(previous_message),
        );
    }
    if !changed {
        return Ok(a(incoming_messages.to_vec()));
    }
    let mut out: Vec<JV> = merged.into_iter().map(|(_, m)| m).collect();
    let mut err = None;
    js_sort(&mut out, |l, r| {
        match locale_compare(
            &g(l, "createdAt"),
            &g(r, "createdAt"),
            "left.createdAt",
            "left.createdAt.localeCompare(right.createdAt)",
        ) {
            Ok(v) => v,
            Err(e) => {
                err = Some(e);
                f64::NAN
            }
        }
    });
    match err {
        Some(e) => Err(e),
        None => Ok(a(out)),
    }
}
fn has_live_assistant_intro(previous_thread: &JV) -> bool {
    if matches!(previous_thread, JV::U) {
        return false;
    }
    let latest_turn = g(previous_thread, "latestTurn");
    if !truthy(&latest_turn) || !streq(&g(&latest_turn, "state"), "running") {
        return false;
    }
    if !streq(
        &g(&g(previous_thread, "session"), "orchestrationStatus"),
        "running",
    ) {
        return false;
    }
    units_arr(&g(previous_thread, "messages")).iter().any(|m| {
        streq(&g(m, "role"), "assistant")
            && eq(&g(m, "turnId"), &g(&latest_turn, "turnId"))
            && (truthy(&g(m, "streaming"))
                || eq(&g(m, "id"), &g(&latest_turn, "assistantMessageId")))
    })
}
fn should_preserve_running_turn(previous_thread: &JV, incoming: &JV) -> bool {
    if !has_live_assistant_intro(previous_thread) {
        return false;
    }
    let previous_turn_id = g(&g(previous_thread, "latestTurn"), "turnId");
    if !truthy(&previous_turn_id) {
        return false;
    }
    let incoming_lt = g(incoming, "latestTurn");
    if !eq(&g(&incoming_lt, "turnId"), &previous_turn_id) {
        return true;
    }
    !truthy(&g(&incoming_lt, "completedAt"))
}
fn read_model_session_from_thread_session(
    previous_session: &JV,
    previous_thread: &JV,
    incoming_session: &JV,
) -> JV {
    let mut m = Obj::new();
    let thread_id = if !matches!(g(previous_thread, "id"), JV::U) {
        g(previous_thread, "id")
    } else if !matches!(g(incoming_session, "threadId"), JV::U) {
        g(incoming_session, "threadId")
    } else {
        s("unknown")
    };
    mset(&mut m, "threadId", thread_id);
    mset(&mut m, "status", g(previous_session, "orchestrationStatus"));
    mset(&mut m, "providerName", g(previous_session, "provider"));
    let piid = g(previous_session, "providerInstanceId");
    if !matches!(piid, JV::U) {
        mset(&mut m, "providerInstanceId", piid);
    }
    mset(
        &mut m,
        "runtimeMode",
        or(
            &g(previous_session, "runtimeMode"),
            or(
                &g(previous_thread, "runtimeMode"),
                or(&g(incoming_session, "runtimeMode"), s("full-access")),
            ),
        ),
    );
    mset(
        &mut m,
        "activeTurnId",
        gv_or_null(previous_session, "activeTurnId"),
    );
    mset(
        &mut m,
        "lastError",
        gv_or_null(previous_session, "lastError"),
    );
    mset(&mut m, "updatedAt", g(previous_session, "updatedAt"));
    o(m)
}
fn merge_read_model_session_hot_path(
    incoming_session: &JV,
    previous_thread: &JV,
    preserve: bool,
    incoming_latest_turn: &JV,
) -> JV {
    let previous_session = g(previous_thread, "session");
    if is_nullish(&previous_session) || !preserve {
        return incoming_session.clone();
    }
    if !truthy(incoming_session) {
        return if streq(&g(&previous_session, "orchestrationStatus"), "running") {
            read_model_session_from_thread_session(
                &previous_session,
                previous_thread,
                incoming_session,
            )
        } else {
            incoming_session.clone()
        };
    }
    if js_gt(
        &g(&previous_session, "updatedAt"),
        &g(incoming_session, "updatedAt"),
    ) {
        let next = read_model_session_from_thread_session(
            &previous_session,
            previous_thread,
            incoming_session,
        );
        let mut m = spread_map(&next);
        mset(&mut m, "providerName", g(incoming_session, "providerName"));
        let ipiid = g(incoming_session, "providerInstanceId");
        if !matches!(ipiid, JV::U) {
            mset(&mut m, "providerInstanceId", ipiid);
        } else {
            let ppiid = g(&previous_session, "providerInstanceId");
            if !matches!(ppiid, JV::U) {
                mset(&mut m, "providerInstanceId", ppiid);
            } else {
                m.remove("providerInstanceId");
            }
        }
        mset(&mut m, "runtimeMode", g(incoming_session, "runtimeMode"));
        mset(
            &mut m,
            "activeTurnId",
            or(
                &g(&previous_session, "activeTurnId"),
                g(incoming_session, "activeTurnId"),
            ),
        );
        mset(
            &mut m,
            "lastError",
            or(
                &g(&previous_session, "lastError"),
                g(incoming_session, "lastError"),
            ),
        );
        return o(m);
    }
    let superseded = js_gt(
        &g(incoming_session, "updatedAt"),
        &g(&previous_session, "updatedAt"),
    ) && !is_nullish(incoming_latest_turn)
        && !is_nullish(&g(incoming_latest_turn, "completedAt"))
        && !eq(
            &g(incoming_latest_turn, "turnId"),
            &g(&g(previous_thread, "latestTurn"), "turnId"),
        );
    if streq(&g(&previous_session, "orchestrationStatus"), "running")
        && !streq(&g(incoming_session, "status"), "running")
        && !streq(&g(incoming_session, "status"), "error")
        && !matches!(g(&previous_session, "activeTurnId"), JV::U)
        && !superseded
    {
        let mut m = spread_map(incoming_session);
        mset(&mut m, "status", s("running"));
        mset(&mut m, "activeTurnId", g(&previous_session, "activeTurnId"));
        mset(
            &mut m,
            "lastError",
            or(
                &g(&previous_session, "lastError"),
                g(incoming_session, "lastError"),
            ),
        );
        mset(
            &mut m,
            "updatedAt",
            if js_ge(
                &g(&previous_session, "updatedAt"),
                &g(incoming_session, "updatedAt"),
            ) {
                g(&previous_session, "updatedAt")
            } else {
                g(incoming_session, "updatedAt")
            },
        );
        return o(m);
    }
    incoming_session.clone()
}
fn merge_read_model_latest_turn_hot_path(
    incoming_lt: &JV,
    previous_thread: &JV,
    preserve: bool,
) -> JV {
    let previous_lt = g(previous_thread, "latestTurn");
    if !truthy(&previous_lt) {
        return incoming_lt.clone();
    }
    if preserve {
        if matches!(incoming_lt, JV::N) || eq(&g(incoming_lt, "turnId"), &g(&previous_lt, "turnId"))
        {
            let base = if matches!(incoming_lt, JV::N) {
                previous_lt.clone()
            } else {
                incoming_lt.clone()
            };
            let mut m = spread_map(&base);
            mset(&mut m, "turnId", g(&previous_lt, "turnId"));
            mset(&mut m, "state", s("running"));
            mset(
                &mut m,
                "requestedAt",
                or(
                    &g(incoming_lt, "requestedAt"),
                    g(&previous_lt, "requestedAt"),
                ),
            );
            mset(
                &mut m,
                "startedAt",
                or(&g(incoming_lt, "startedAt"), g(&previous_lt, "startedAt")),
            );
            mset(&mut m, "completedAt", JV::N);
            mset(
                &mut m,
                "assistantMessageId",
                or(
                    &g(&previous_lt, "assistantMessageId"),
                    gv_or_null(incoming_lt, "assistantMessageId"),
                ),
            );
            let spp = or(
                &g(incoming_lt, "sourceProposedPlan"),
                g(&previous_lt, "sourceProposedPlan"),
            );
            m.remove("sourceProposedPlan");
            if truthy(&spp) {
                mset(&mut m, "sourceProposedPlan", spp);
            }
            return o(m);
        }
        return incoming_lt.clone();
    }
    if matches!(incoming_lt, JV::N) || !eq(&g(incoming_lt, "turnId"), &g(&previous_lt, "turnId")) {
        return incoming_lt.clone();
    }
    if matches!(g(&previous_lt, "assistantMessageId"), JV::U)
        || eq(
            &g(incoming_lt, "assistantMessageId"),
            &g(&previous_lt, "assistantMessageId"),
        )
    {
        return incoming_lt.clone();
    }
    let mut m = spread_map(incoming_lt);
    mset(
        &mut m,
        "assistantMessageId",
        g(&previous_lt, "assistantMessageId"),
    );
    o(m)
}
fn clear_settled_turn_streaming(messages: &[JV], settled_turn_id: &JV) -> JV {
    let mut changed = false;
    let next: Vec<JV> = messages
        .iter()
        .map(|m| {
            if !truthy(&g(m, "streaming")) || !eq(&g(m, "turnId"), settled_turn_id) {
                m.clone()
            } else {
                changed = true;
                let mut mm = spread_map(m);
                mset(&mut mm, "streaming", b(false));
                o(mm)
            }
        })
        .collect();
    if changed {
        a(next)
    } else {
        a(messages.to_vec())
    }
}
/// Returns (usageLimitRecovery, usageLimitRecoverySequence).
fn normalize_usage_limit_recovery(
    incoming: &JV,
    previous: &JV,
    snapshot_sequence: &JV,
) -> (JV, JV) {
    let stale = !matches!(previous, JV::U)
        && (if matches!(snapshot_sequence, JV::U) {
            js_lt(
                &g(incoming, "updatedAt"),
                &or(&g(previous, "updatedAt"), g(previous, "createdAt")),
            )
        } else {
            js_lt(
                snapshot_sequence,
                &or(&g(previous, "usageLimitRecoverySequence"), n(0.0)),
            )
        });
    let candidate = if stale {
        g(previous, "usageLimitRecovery")
    } else {
        g(incoming, "usageLimitRecovery")
    };
    let ulr = if truthy(&g(previous, "usageLimitRecovery"))
        && deep_equal_json(&g(previous, "usageLimitRecovery"), &or(&candidate, JV::N))
    {
        g(previous, "usageLimitRecovery")
    } else {
        or(&candidate, JV::N)
    };
    let seq = if matches!(snapshot_sequence, JV::U) {
        g(previous, "usageLimitRecoverySequence")
    } else {
        n(js_max(
            to_num(snapshot_sequence),
            to_num(&or(&g(previous, "usageLimitRecoverySequence"), n(0.0))),
        ))
    };
    (ulr, seq)
}
/// `mergeReadModelThreadDetailWithLiveHotPath`.
pub fn merge_read_model_thread_detail(
    incoming: &JV,
    previous_thread: &JV,
    snapshot_sequence: &JV,
) -> R<JV> {
    if matches!(previous_thread, JV::U) {
        return Ok(incoming.clone());
    }
    let prev_lt = g(previous_thread, "latestTurn");
    let incoming_lt = g(incoming, "latestTurn");
    let settled_local_turn_id: JV = if streq(&g(&prev_lt, "state"), "running")
        && !matches!(incoming_lt, JV::N)
        && eq(&g(&incoming_lt, "turnId"), &g(&prev_lt, "turnId"))
        && !streq(&g(&incoming_lt, "state"), "running")
        && !matches!(g(&incoming_lt, "completedAt"), JV::N)
    {
        g(&incoming_lt, "turnId")
    } else {
        JV::N
    };
    let preserve = matches!(settled_local_turn_id, JV::N)
        && should_preserve_running_turn(previous_thread, incoming);
    let merged_messages = merge_read_model_messages_hot_path(
        &units_arr(&g(incoming, "messages")),
        previous_thread,
        &settled_local_turn_id,
    )?;
    let messages = if matches!(settled_local_turn_id, JV::N) {
        merged_messages
    } else {
        clear_settled_turn_streaming(&units_arr(&merged_messages), &settled_local_turn_id)
    };
    let session = merge_read_model_session_hot_path(
        &g(incoming, "session"),
        previous_thread,
        preserve,
        &incoming_lt,
    );
    let latest_turn =
        merge_read_model_latest_turn_hot_path(&incoming_lt, previous_thread, preserve);
    let stale_cc = !matches!(g(previous_thread, "claudeCacheReview"), JV::U)
        && (if matches!(snapshot_sequence, JV::U) {
            js_lt(
                &g(incoming, "updatedAt"),
                &or(
                    &g(previous_thread, "updatedAt"),
                    g(previous_thread, "createdAt"),
                ),
            )
        } else {
            js_lt(
                snapshot_sequence,
                &or(&g(previous_thread, "claudeCacheReviewSequence"), n(0.0)),
            )
        });
    let claude_cache_review = if stale_cc {
        g(previous_thread, "claudeCacheReview")
    } else {
        g(incoming, "claudeCacheReview")
    };
    let stale_ulr = if matches!(snapshot_sequence, JV::U) {
        js_lt(
            &g(incoming, "updatedAt"),
            &or(
                &g(previous_thread, "updatedAt"),
                g(previous_thread, "createdAt"),
            ),
        )
    } else {
        js_lt(
            snapshot_sequence,
            &or(&g(previous_thread, "usageLimitRecoverySequence"), n(0.0)),
        )
    };
    let usage_limit_recovery = if stale_ulr {
        g(previous_thread, "usageLimitRecovery")
    } else {
        g(incoming, "usageLimitRecovery")
    };
    let preserve_snooze = if matches!(snapshot_sequence, JV::U) {
        js_lt(
            &g(incoming, "updatedAt"),
            &or(
                &g(previous_thread, "updatedAt"),
                g(previous_thread, "createdAt"),
            ),
        )
    } else {
        js_lt(
            snapshot_sequence,
            &or(&g(previous_thread, "snoozeSequence"), n(0.0)),
        )
    };
    let snoozed_until = if preserve_snooze {
        g(previous_thread, "snoozedUntil")
    } else {
        g(incoming, "snoozedUntil")
    };
    let snooze_reminder_at = if preserve_snooze {
        g(previous_thread, "snoozeReminderAt")
    } else {
        g(incoming, "snoozeReminderAt")
    };
    if eq(&messages, &g(incoming, "messages"))
        && eq(&session, &g(incoming, "session"))
        && eq(&latest_turn, &g(incoming, "latestTurn"))
        && eq(&claude_cache_review, &g(incoming, "claudeCacheReview"))
        && eq(&usage_limit_recovery, &g(incoming, "usageLimitRecovery"))
        && eq(&snoozed_until, &g(incoming, "snoozedUntil"))
        && eq(&snooze_reminder_at, &g(incoming, "snoozeReminderAt"))
    {
        return Ok(incoming.clone());
    }
    let mut m = spread_map(incoming);
    mset(&mut m, "messages", messages);
    mset(&mut m, "session", session);
    mset(&mut m, "latestTurn", latest_turn);
    if !matches!(claude_cache_review, JV::U) {
        mset(&mut m, "claudeCacheReview", claude_cache_review);
    }
    if !matches!(usage_limit_recovery, JV::U) {
        mset(&mut m, "usageLimitRecovery", usage_limit_recovery);
    }
    if !matches!(snoozed_until, JV::U) {
        mset(&mut m, "snoozedUntil", snoozed_until);
    }
    if !matches!(snooze_reminder_at, JV::U) {
        mset(&mut m, "snoozeReminderAt", snooze_reminder_at);
    }
    Ok(o(m))
}

/// `normalizeThreadFromReadModel` -> full Thread JV.
pub fn normalize_thread_from_read_model(
    incoming: &JV,
    previous: &JV,
    snapshot_sequence: &JV,
    restoring_session: bool,
) -> R<JV> {
    let model_selection = normalize_model_selection(
        &gv(incoming, "modelSelection", "incoming.modelSelection")?,
        &g(previous, "modelSelection"),
    )?;
    let session = normalize_thread_session(
        &gv(incoming, "session", "incoming.session")?,
        &g(previous, "session"),
    );
    let messages = normalize_chat_messages(
        &units_arr(&gv(incoming, "messages", "incoming.messages")?),
        &g(previous, "messages"),
    )?;
    let proposed_plans = normalize_proposed_plans(
        &units_arr(&gv(incoming, "proposedPlans", "incoming.proposedPlans")?),
        &g(previous, "proposedPlans"),
    );
    let latest_turn = normalize_latest_turn(
        &gv(incoming, "latestTurn", "incoming.latestTurn")?,
        &g(previous, "latestTurn"),
    );
    let prev_handoff = g(previous, "handoff");
    let incoming_handoff = gv(incoming, "handoff", "incoming.handoff")?;
    let handoff = if truthy(&prev_handoff)
        && truthy(&incoming_handoff)
        && deep_equal_json(&prev_handoff, &incoming_handoff)
    {
        prev_handoff
    } else {
        or(&incoming_handoff, JV::N)
    };
    let prev_exists = !matches!(previous, JV::U);
    let preserve_snooze = prev_exists
        && (if matches!(snapshot_sequence, JV::U) {
            js_lt(
                &gv(incoming, "updatedAt", "incoming.updatedAt")?,
                &or(&g(previous, "updatedAt"), g(previous, "createdAt")),
            )
        } else {
            js_lt(
                snapshot_sequence,
                &or(&g(previous, "snoozeSequence"), n(0.0)),
            )
        });
    let snoozed_until = or(
        &(if preserve_snooze {
            g(previous, "snoozedUntil")
        } else {
            gv(incoming, "snoozedUntil", "incoming.snoozedUntil")?
        }),
        JV::N,
    );
    let snooze_reminder_at = or(
        &(if preserve_snooze {
            g(previous, "snoozeReminderAt")
        } else {
            gv(incoming, "snoozeReminderAt", "incoming.snoozeReminderAt")?
        }),
        JV::N,
    );
    let snooze_sequence = if matches!(snapshot_sequence, JV::U) {
        g(previous, "snoozeSequence")
    } else {
        n(js_max(
            to_num(snapshot_sequence),
            to_num(&or(&g(previous, "snoozeSequence"), n(0.0))),
        ))
    };
    let (usage_limit_recovery, usage_limit_recovery_sequence) =
        normalize_usage_limit_recovery(incoming, previous, snapshot_sequence);
    let incoming_ccr = if !matches!(snapshot_sequence, JV::U)
        && js_lt(
            snapshot_sequence,
            &or(&g(previous, "claudeCacheReviewSequence"), n(0.0)),
        ) {
        g(previous, "claudeCacheReview")
    } else {
        gv(incoming, "claudeCacheReview", "incoming.claudeCacheReview")?
    };
    let claude_cache_review_sequence = if matches!(snapshot_sequence, JV::U) {
        g(previous, "claudeCacheReviewSequence")
    } else {
        n(js_max(
            to_num(snapshot_sequence),
            to_num(&or(&g(previous, "claudeCacheReviewSequence"), n(0.0))),
        ))
    };
    let claude_cache_review = if truthy(&g(previous, "claudeCacheReview"))
        && deep_equal_json(&g(previous, "claudeCacheReview"), &or(&incoming_ccr, JV::N))
    {
        g(previous, "claudeCacheReview")
    } else {
        or(&incoming_ccr, JV::N)
    };
    let prev_lkp = g(previous, "lastKnownPr");
    let inc_lkp = gv(incoming, "lastKnownPr", "incoming.lastKnownPr")?;
    let last_known_pr =
        if truthy(&prev_lkp) && truthy(&inc_lkp) && deep_equal_json(&prev_lkp, &inc_lkp) {
            prev_lkp
        } else {
            or(&inc_lkp, JV::N)
        };
    let prev_prs = g(previous, "pullRequests");
    let pull_requests = if truthy(&prev_prs)
        && deep_equal_json(
            &prev_prs,
            &or(
                &gv(incoming, "pullRequests", "incoming.pullRequests")?,
                a(vec![]),
            ),
        ) {
        prev_prs
    } else {
        or(
            &gv(incoming, "pullRequests", "incoming.pullRequests")?,
            a(vec![]),
        )
    };
    let prev_pinned = g(previous, "pinnedMessages");
    let pinned_messages = if truthy(&prev_pinned)
        && deep_equal_json(
            &prev_pinned,
            &or(
                &gv(incoming, "pinnedMessages", "incoming.pinnedMessages")?,
                JV::N,
            ),
        ) {
        prev_pinned
    } else {
        gv(incoming, "pinnedMessages", "incoming.pinnedMessages")?
    };
    let notes = gv(incoming, "notes", "incoming.notes")?;
    let goal = gv(incoming, "goal", "incoming.goal")?;
    let goal_started_at = gv(incoming, "goalStartedAt", "incoming.goalStartedAt")?;
    let goal_paused_at = gv(incoming, "goalPausedAt", "incoming.goalPausedAt")?;
    let prev_ga = g(previous, "goalAchievements");
    let goal_achievements = if truthy(&prev_ga)
        && deep_equal_json(
            &prev_ga,
            &or(
                &gv(incoming, "goalAchievements", "incoming.goalAchievements")?,
                JV::N,
            ),
        ) {
        prev_ga
    } else {
        gv(incoming, "goalAchievements", "incoming.goalAchievements")?
    };
    let turn_diff_summaries = normalize_turn_diff_summaries(
        &units_arr(&gv(incoming, "checkpoints", "incoming.checkpoints")?),
        &g(previous, "turnDiffSummaries"),
    );
    let activities = normalize_activities(
        &units_arr(&gv(incoming, "activities", "incoming.activities")?),
        &g(previous, "activities"),
    )?;
    let incoming_pi = if has(
        &gv(
            incoming,
            "pendingInteractions",
            "incoming.pendingInteractions",
        )?,
        "pendingInteractions",
    ) {
        or(
            &gv(
                incoming,
                "pendingInteractions",
                "incoming.pendingInteractions",
            )?,
            a(vec![]),
        )
    } else {
        g(previous, "pendingInteractions")
    };
    let prev_pi = g(previous, "pendingInteractions");
    let pending_interactions =
        if truthy(&prev_pi) && deep_equal_json(&prev_pi, &or(&incoming_pi, a(vec![]))) {
            prev_pi
        } else if matches!(incoming_pi, JV::U) {
            JV::U
        } else {
            a(units_arr(&incoming_pi))
        };
    let error = normalize_thread_error(&g(
        &gv(incoming, "session", "incoming.session")?,
        "lastError",
    ));
    let prev_visited = g(previous, "lastVisitedAt");
    let last_visited_at = if !matches!(prev_visited, JV::U) {
        prev_visited
    } else {
        resolve_initial_last_visited_at(incoming, restoring_session)
    };
    let resolved_latest_human = gv(
        incoming,
        "latestHumanMessageAt",
        "incoming.latestHumanMessageAt",
    )?;
    let resolved_latest_user = if has(
        &gv(
            incoming,
            "latestUserMessageAt",
            "incoming.latestUserMessageAt",
        )?,
        "latestUserMessageAt",
    ) && !matches!(
        gv(
            incoming,
            "latestUserMessageAt",
            "incoming.latestUserMessageAt"
        )?,
        JV::U
    ) {
        or(
            &gv(
                incoming,
                "latestUserMessageAt",
                "incoming.latestUserMessageAt",
            )?,
            JV::N,
        )
    } else {
        JV::U
    };
    let rha = gv(
        incoming,
        "hasPendingApprovals",
        "incoming.hasPendingApprovals",
    )?;
    let resolved_approvals = if let JV::B(_) = rha { rha } else { JV::U };
    let rhui = gv(
        incoming,
        "hasPendingUserInput",
        "incoming.hasPendingUserInput",
    )?;
    let resolved_user_input = if let JV::B(_) = rhui { rhui } else { JV::U };
    let rhap = gv(
        incoming,
        "hasActionableProposedPlan",
        "incoming.hasActionableProposedPlan",
    )?;
    let resolved_plan = if let JV::B(_) = rhap { rhap } else { JV::U };
    let next_worktree_path = gv(incoming, "worktreePath", "incoming.worktreePath")?;
    let next_working_directory = or(
        &gv(incoming, "workingDirectory", "incoming.workingDirectory")?,
        JV::N,
    );
    let next_awp = or(
        &gv(
            incoming,
            "associatedWorktreePath",
            "incoming.associatedWorktreePath",
        )?,
        JV::N,
    );
    let next_awb = or(
        &gv(
            incoming,
            "associatedWorktreeBranch",
            "incoming.associatedWorktreeBranch",
        )?,
        JV::N,
    );
    let next_awr = or(
        &gv(
            incoming,
            "associatedWorktreeRef",
            "incoming.associatedWorktreeRef",
        )?,
        JV::N,
    );
    let resolved_branch = branch_regression_guard(
        &gv_or_null(previous, "branch"),
        &gv(incoming, "branch", "incoming.branch")?,
    );
    let resolved_cbfc = create_branch_flow_merge(&[
        ("currentBranch", gv_or_null(previous, "branch")),
        ("nextBranch", resolved_branch.clone()),
        ("currentWorktreePath", gv_or_null(previous, "worktreePath")),
        ("nextWorktreePath", next_worktree_path.clone()),
        (
            "currentAssociatedWorktreePath",
            g(previous, "associatedWorktreePath"),
        ),
        ("nextAssociatedWorktreePath", next_awp.clone()),
        (
            "currentAssociatedWorktreeBranch",
            g(previous, "associatedWorktreeBranch"),
        ),
        ("nextAssociatedWorktreeBranch", next_awb.clone()),
        (
            "currentAssociatedWorktreeRef",
            g(previous, "associatedWorktreeRef"),
        ),
        ("nextAssociatedWorktreeRef", next_awr.clone()),
        (
            "currentCreateBranchFlowCompleted",
            g(previous, "createBranchFlowCompleted"),
        ),
        (
            "nextCreateBranchFlowCompleted",
            gv(
                incoming,
                "createBranchFlowCompleted",
                "incoming.createBranchFlowCompleted",
            )?,
        ),
    ]);
    let pending_spp = if !is_nullish(&g(&latest_turn, "sourceProposedPlan")) {
        g(&latest_turn, "sourceProposedPlan")
    } else if streq(
        &g(&gv(incoming, "session", "incoming.session")?, "status"),
        "running",
    ) {
        g(previous, "pendingSourceProposedPlan")
    } else {
        JV::U
    };
    let pending_turn_start = if streq(&g(&claude_cache_review, "status"), "compacting") {
        JV::N
    } else {
        g(previous, "pendingTurnStartMessageId")
    };
    if prev_exists
        && eq(
            &g(previous, "projectId"),
            &gv(incoming, "projectId", "incoming.projectId")?,
        )
        && eq(
            &g(previous, "title"),
            &gv(incoming, "title", "incoming.title")?,
        )
        && eq(&g(previous, "modelSelection"), &model_selection)
        && eq(
            &g(previous, "runtimeMode"),
            &gv(incoming, "runtimeMode", "incoming.runtimeMode")?,
        )
        && eq(
            &g(previous, "interactionMode"),
            &gv(incoming, "interactionMode", "incoming.interactionMode")?,
        )
        && eq(&g(previous, "session"), &session)
        && eq(&g(previous, "messages"), &messages)
        && eq(&g(previous, "proposedPlans"), &proposed_plans)
        && eq(&g(previous, "error"), &error)
        && eq(
            &g(previous, "createdAt"),
            &gv(incoming, "createdAt", "incoming.createdAt")?,
        )
        && field_eq_or(previous, incoming, "archivedAt", &JV::N)
        && field_eq_or(previous, incoming, "settledAt", &JV::N)
        && eq_or(&g(previous, "snoozedUntil"), &snoozed_until)
        && eq_or(&g(previous, "snoozeReminderAt"), &snooze_reminder_at)
        && eq(&g(previous, "snoozeSequence"), &snooze_sequence)
        && eq(
            &g(previous, "updatedAt"),
            &gv(incoming, "updatedAt", "incoming.updatedAt")?,
        )
        && eq_or(
            &g(previous, "isPinned"),
            &or(&gv(incoming, "isPinned", "incoming.isPinned")?, b(false)),
        )
        && eq(&g(previous, "latestTurn"), &latest_turn)
        && eq(&g(previous, "pendingSourceProposedPlan"), &pending_spp)
        && eq(
            &g(previous, "pendingTurnStartMessageId"),
            &pending_turn_start,
        )
        && eq(&g(previous, "lastVisitedAt"), &last_visited_at)
        && field_eq_or(previous, incoming, "parentThreadId", &JV::N)
        && field_eq_or(previous, incoming, "creationSource", &JV::N)
        && field_eq_or(previous, incoming, "sourceThreadId", &JV::N)
        && field_eq_or(previous, incoming, "subagentAgentId", &JV::N)
        && field_eq_or(previous, incoming, "subagentNickname", &JV::N)
        && field_eq_or(previous, incoming, "subagentRole", &JV::N)
        && eq(
            &g(previous, "envMode"),
            &or(&gv(incoming, "envMode", "incoming.envMode")?, s("local")),
        )
        && eq(&g(previous, "branch"), &resolved_branch)
        && eq(&g(previous, "worktreePath"), &next_worktree_path)
        && eq_or(&g(previous, "workingDirectory"), &next_working_directory)
        && eq_or(&g(previous, "associatedWorktreePath"), &next_awp)
        && eq_or(&g(previous, "associatedWorktreeBranch"), &next_awb)
        && eq_or(&g(previous, "associatedWorktreeRef"), &next_awr)
        && eq_or(&g(previous, "createBranchFlowCompleted"), &resolved_cbfc)
        && eq(&g(previous, "latestUserMessageAt"), &resolved_latest_user)
        && eq(&g(previous, "latestHumanMessageAt"), &resolved_latest_human)
        && eq(&g(previous, "hasPendingApprovals"), &resolved_approvals)
        && eq(&g(previous, "hasPendingUserInput"), &resolved_user_input)
        && eq(&g(previous, "hasActionableProposedPlan"), &resolved_plan)
        && field_eq_or(previous, incoming, "forkSourceThreadId", &JV::N)
        && field_eq_or(previous, incoming, "sidechatSourceThreadId", &JV::N)
        && eq(
            &gv_or_null(&g(previous, "sidechatContext"), "url"),
            &gv_or_null(
                &gv(incoming, "sidechatContext", "incoming.sidechatContext")?,
                "url",
            ),
        )
        && field_eq_or(previous, incoming, "sidechatLastActivityAt", &JV::N)
        && field_eq_or(previous, incoming, "sidechatExpiredAt", &JV::N)
        && deep_equal_json(&gv_or_null(previous, "lastKnownPr"), &last_known_pr)
        && eq_or(&g(previous, "handoff"), &handoff)
        && eq_or(
            &g(previous, "isProjectImport"),
            &or(
                &gv(incoming, "isProjectImport", "incoming.isProjectImport")?,
                b(false),
            ),
        )
        && eq_or(&g(previous, "claudeCacheReview"), &claude_cache_review)
        && eq(
            &g(previous, "claudeCacheReviewSequence"),
            &claude_cache_review_sequence,
        )
        && eq_or(&g(previous, "usageLimitRecovery"), &usage_limit_recovery)
        && eq(
            &g(previous, "usageLimitRecoverySequence"),
            &usage_limit_recovery_sequence,
        )
        && eq(&g(previous, "pinnedMessages"), &pinned_messages)
        && eq(&g(previous, "notes"), &notes)
        && eq(&g(previous, "goal"), &goal)
        && eq_or(
            &g(previous, "goalStartedAt"),
            &or(
                &gv(incoming, "goalStartedAt", "incoming.goalStartedAt")?,
                JV::N,
            ),
        )
        && eq_or(
            &g(previous, "goalPausedAt"),
            &or(
                &gv(incoming, "goalPausedAt", "incoming.goalPausedAt")?,
                JV::N,
            ),
        )
        && eq(&g(previous, "goalAchievements"), &goal_achievements)
        && eq(&g(previous, "turnDiffSummaries"), &turn_diff_summaries)
        && eq(&g(previous, "activities"), &activities)
        && eq(&g(previous, "pendingInteractions"), &pending_interactions)
    {
        return Ok(previous.clone());
    }
    let mut m = Obj::new();
    mset(&mut m, "id", gv(incoming, "id", "incoming.id")?);
    mset(&mut m, "codexThreadId", JV::N);
    mset(
        &mut m,
        "projectId",
        gv(incoming, "projectId", "incoming.projectId")?,
    );
    mset(&mut m, "title", gv(incoming, "title", "incoming.title")?);
    mset(&mut m, "modelSelection", model_selection);
    mset(
        &mut m,
        "runtimeMode",
        gv(incoming, "runtimeMode", "incoming.runtimeMode")?,
    );
    mset(
        &mut m,
        "interactionMode",
        gv(incoming, "interactionMode", "incoming.interactionMode")?,
    );
    mset(&mut m, "session", session);
    mset(&mut m, "messages", messages);
    mset(&mut m, "proposedPlans", proposed_plans);
    mset(&mut m, "error", error);
    mset(
        &mut m,
        "createdAt",
        gv(incoming, "createdAt", "incoming.createdAt")?,
    );
    mset(
        &mut m,
        "archivedAt",
        or(&gv(incoming, "archivedAt", "incoming.archivedAt")?, JV::N),
    );
    mset(
        &mut m,
        "settledAt",
        or(&gv(incoming, "settledAt", "incoming.settledAt")?, JV::N),
    );
    mset(&mut m, "snoozedUntil", snoozed_until);
    mset(&mut m, "snoozeReminderAt", snooze_reminder_at);
    if !matches!(snooze_sequence, JV::U) {
        mset(&mut m, "snoozeSequence", snooze_sequence);
    }
    mset(
        &mut m,
        "updatedAt",
        gv(incoming, "updatedAt", "incoming.updatedAt")?,
    );
    mset(
        &mut m,
        "isPinned",
        or(&gv(incoming, "isPinned", "incoming.isPinned")?, b(false)),
    );
    mset(&mut m, "latestTurn", latest_turn);
    if truthy(&pending_spp) {
        mset(&mut m, "pendingSourceProposedPlan", pending_spp);
    }
    if !matches!(pending_turn_start, JV::U) {
        mset(&mut m, "pendingTurnStartMessageId", pending_turn_start);
    }
    mset(&mut m, "lastVisitedAt", last_visited_at);
    mset(
        &mut m,
        "parentThreadId",
        or(
            &gv(incoming, "parentThreadId", "incoming.parentThreadId")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "creationSource",
        or(
            &gv(incoming, "creationSource", "incoming.creationSource")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sourceThreadId",
        or(
            &gv(incoming, "sourceThreadId", "incoming.sourceThreadId")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "subagentAgentId",
        or(
            &gv(incoming, "subagentAgentId", "incoming.subagentAgentId")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "subagentNickname",
        or(
            &gv(incoming, "subagentNickname", "incoming.subagentNickname")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "subagentRole",
        or(
            &gv(incoming, "subagentRole", "incoming.subagentRole")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "envMode",
        or(&gv(incoming, "envMode", "incoming.envMode")?, s("local")),
    );
    mset(&mut m, "branch", resolved_branch);
    mset(&mut m, "worktreePath", next_worktree_path);
    mset(&mut m, "workingDirectory", next_working_directory);
    mset(&mut m, "associatedWorktreePath", next_awp);
    mset(&mut m, "associatedWorktreeBranch", next_awb);
    mset(&mut m, "associatedWorktreeRef", next_awr);
    mset(&mut m, "createBranchFlowCompleted", resolved_cbfc);
    mset(
        &mut m,
        "forkSourceThreadId",
        or(
            &gv(
                incoming,
                "forkSourceThreadId",
                "incoming.forkSourceThreadId",
            )?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatSourceThreadId",
        or(
            &gv(
                incoming,
                "sidechatSourceThreadId",
                "incoming.sidechatSourceThreadId",
            )?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatContext",
        or(
            &gv(incoming, "sidechatContext", "incoming.sidechatContext")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatLastActivityAt",
        or(
            &gv(
                incoming,
                "sidechatLastActivityAt",
                "incoming.sidechatLastActivityAt",
            )?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatExpiredAt",
        or(
            &gv(incoming, "sidechatExpiredAt", "incoming.sidechatExpiredAt")?,
            JV::N,
        ),
    );
    mset(&mut m, "lastKnownPr", last_known_pr);
    mset(&mut m, "pullRequests", pull_requests);
    mset(&mut m, "handoff", handoff);
    if truthy(&gv(
        incoming,
        "isProjectImport",
        "incoming.isProjectImport",
    )?) {
        mset(&mut m, "isProjectImport", b(true));
    }
    mset(&mut m, "usageLimitRecovery", usage_limit_recovery);
    if !matches!(usage_limit_recovery_sequence, JV::U) {
        mset(
            &mut m,
            "usageLimitRecoverySequence",
            usage_limit_recovery_sequence,
        );
    }
    mset(&mut m, "claudeCacheReview", claude_cache_review);
    if !matches!(claude_cache_review_sequence, JV::U) {
        mset(
            &mut m,
            "claudeCacheReviewSequence",
            claude_cache_review_sequence,
        );
    }
    if !matches!(pinned_messages, JV::U) {
        mset(&mut m, "pinnedMessages", pinned_messages);
    }
    if !matches!(notes, JV::U) {
        mset(&mut m, "notes", notes);
    }
    if !matches!(goal, JV::U) {
        mset(&mut m, "goal", goal);
    }
    if !matches!(goal_started_at, JV::U) {
        mset(&mut m, "goalStartedAt", goal_started_at);
    }
    if !matches!(goal_paused_at, JV::U) {
        mset(&mut m, "goalPausedAt", goal_paused_at);
    }
    if !matches!(goal_achievements, JV::U) {
        mset(&mut m, "goalAchievements", goal_achievements);
    }
    if !matches!(resolved_latest_human, JV::U) {
        mset(&mut m, "latestHumanMessageAt", resolved_latest_human);
    }
    if !matches!(resolved_latest_user, JV::U) {
        mset(&mut m, "latestUserMessageAt", resolved_latest_user);
    }
    if !matches!(resolved_approvals, JV::U) {
        mset(&mut m, "hasPendingApprovals", resolved_approvals);
    }
    if !matches!(resolved_user_input, JV::U) {
        mset(&mut m, "hasPendingUserInput", resolved_user_input);
    }
    if !matches!(resolved_plan, JV::U) {
        mset(&mut m, "hasActionableProposedPlan", resolved_plan);
    }
    let pbtc = g(previous, "pendingBackgroundTaskCount");
    if !matches!(pbtc, JV::U) {
        mset(&mut m, "pendingBackgroundTaskCount", pbtc);
    }
    let lrp = g(previous, "latestReplyPreview");
    if !matches!(lrp, JV::U) {
        mset(&mut m, "latestReplyPreview", lrp);
    }
    mset(&mut m, "turnDiffSummaries", turn_diff_summaries);
    mset(&mut m, "activities", activities);
    if !matches!(pending_interactions, JV::U) {
        mset(&mut m, "pendingInteractions", pending_interactions);
    }
    Ok(o(m))
}
/// `normalizeThreadShellSnapshot` -> (shell, session, turnState).
pub fn normalize_thread_shell_snapshot(
    incoming: &JV,
    previous: &JV,
    snapshot_sequence: &JV,
    restoring_session: bool,
) -> R<(JV, JV, JV)> {
    let model_selection = normalize_model_selection(
        &gv(incoming, "modelSelection", "incoming.modelSelection")?,
        &g(previous, "modelSelection"),
    )?;
    let session = normalize_thread_session(
        &gv(incoming, "session", "incoming.session")?,
        &g(previous, "session"),
    );
    let latest_turn = normalize_latest_turn(
        &gv(incoming, "latestTurn", "incoming.latestTurn")?,
        &g(previous, "latestTurn"),
    );
    let prev_handoff = g(previous, "handoff");
    let incoming_handoff = gv(incoming, "handoff", "incoming.handoff")?;
    let handoff = if truthy(&prev_handoff)
        && truthy(&incoming_handoff)
        && deep_equal_json(&prev_handoff, &incoming_handoff)
    {
        prev_handoff
    } else {
        or(&incoming_handoff, JV::N)
    };
    let prev_exists = !matches!(previous, JV::U);
    let preserve_snooze = prev_exists
        && (if matches!(snapshot_sequence, JV::U) {
            js_lt(
                &gv(incoming, "updatedAt", "incoming.updatedAt")?,
                &or(&g(previous, "updatedAt"), g(previous, "createdAt")),
            )
        } else {
            js_lt(
                snapshot_sequence,
                &or(&g(previous, "snoozeSequence"), n(0.0)),
            )
        });
    let snoozed_until = or(
        &(if preserve_snooze {
            g(previous, "snoozedUntil")
        } else {
            gv(incoming, "snoozedUntil", "incoming.snoozedUntil")?
        }),
        JV::N,
    );
    let snooze_reminder_at = or(
        &(if preserve_snooze {
            g(previous, "snoozeReminderAt")
        } else {
            gv(incoming, "snoozeReminderAt", "incoming.snoozeReminderAt")?
        }),
        JV::N,
    );
    let snooze_sequence = if matches!(snapshot_sequence, JV::U) {
        g(previous, "snoozeSequence")
    } else {
        n(js_max(
            to_num(snapshot_sequence),
            to_num(&or(&g(previous, "snoozeSequence"), n(0.0))),
        ))
    };
    let (usage_limit_recovery, usage_limit_recovery_sequence) =
        normalize_usage_limit_recovery(incoming, previous, snapshot_sequence);
    let incoming_ccr = if !matches!(snapshot_sequence, JV::U)
        && js_lt(
            snapshot_sequence,
            &or(&g(previous, "claudeCacheReviewSequence"), n(0.0)),
        ) {
        g(previous, "claudeCacheReview")
    } else {
        gv(incoming, "claudeCacheReview", "incoming.claudeCacheReview")?
    };
    let claude_cache_review_sequence = if matches!(snapshot_sequence, JV::U) {
        g(previous, "claudeCacheReviewSequence")
    } else {
        n(js_max(
            to_num(snapshot_sequence),
            to_num(&or(&g(previous, "claudeCacheReviewSequence"), n(0.0))),
        ))
    };
    let claude_cache_review = if truthy(&g(previous, "claudeCacheReview"))
        && deep_equal_json(&g(previous, "claudeCacheReview"), &or(&incoming_ccr, JV::N))
    {
        g(previous, "claudeCacheReview")
    } else {
        or(&incoming_ccr, JV::N)
    };
    let prev_lkp = g(previous, "lastKnownPr");
    let inc_lkp = gv(incoming, "lastKnownPr", "incoming.lastKnownPr")?;
    let last_known_pr =
        if truthy(&prev_lkp) && truthy(&inc_lkp) && deep_equal_json(&prev_lkp, &inc_lkp) {
            prev_lkp
        } else {
            or(&inc_lkp, JV::N)
        };
    let prev_prs = g(previous, "pullRequests");
    let pull_requests = if truthy(&prev_prs)
        && deep_equal_json(
            &prev_prs,
            &or(
                &gv(incoming, "pullRequests", "incoming.pullRequests")?,
                a(vec![]),
            ),
        ) {
        prev_prs
    } else {
        or(
            &gv(incoming, "pullRequests", "incoming.pullRequests")?,
            a(vec![]),
        )
    };
    let error = normalize_thread_error(&g(
        &gv(incoming, "session", "incoming.session")?,
        "lastError",
    ));
    let prev_visited = g(previous, "lastVisitedAt");
    let last_visited_at = if !matches!(prev_visited, JV::U) {
        prev_visited
    } else {
        resolve_initial_last_visited_at(incoming, restoring_session)
    };
    let next_worktree_path = gv(incoming, "worktreePath", "incoming.worktreePath")?;
    let next_working_directory = or(
        &gv(incoming, "workingDirectory", "incoming.workingDirectory")?,
        JV::N,
    );
    let next_awp = or(
        &gv(
            incoming,
            "associatedWorktreePath",
            "incoming.associatedWorktreePath",
        )?,
        JV::N,
    );
    let next_awb = or(
        &gv(
            incoming,
            "associatedWorktreeBranch",
            "incoming.associatedWorktreeBranch",
        )?,
        JV::N,
    );
    let next_awr = or(
        &gv(
            incoming,
            "associatedWorktreeRef",
            "incoming.associatedWorktreeRef",
        )?,
        JV::N,
    );
    let goal = if !matches!(gv(incoming, "goal", "incoming.goal")?, JV::U) {
        gv(incoming, "goal", "incoming.goal")?
    } else {
        g(previous, "goal")
    };
    let goal_started_at = if !matches!(
        gv(incoming, "goalStartedAt", "incoming.goalStartedAt")?,
        JV::U
    ) {
        gv(incoming, "goalStartedAt", "incoming.goalStartedAt")?
    } else {
        g(previous, "goalStartedAt")
    };
    let goal_paused_at = if !matches!(
        gv(incoming, "goalPausedAt", "incoming.goalPausedAt")?,
        JV::U
    ) {
        gv(incoming, "goalPausedAt", "incoming.goalPausedAt")?
    } else {
        g(previous, "goalPausedAt")
    };
    let resolved_branch = branch_regression_guard(
        &gv_or_null(previous, "branch"),
        &gv(incoming, "branch", "incoming.branch")?,
    );
    let resolved_cbfc = create_branch_flow_merge(&[
        ("currentBranch", gv_or_null(previous, "branch")),
        ("nextBranch", resolved_branch.clone()),
        ("currentWorktreePath", gv_or_null(previous, "worktreePath")),
        ("nextWorktreePath", next_worktree_path.clone()),
        (
            "currentAssociatedWorktreePath",
            g(previous, "associatedWorktreePath"),
        ),
        ("nextAssociatedWorktreePath", next_awp.clone()),
        (
            "currentAssociatedWorktreeBranch",
            g(previous, "associatedWorktreeBranch"),
        ),
        ("nextAssociatedWorktreeBranch", next_awb.clone()),
        (
            "currentAssociatedWorktreeRef",
            g(previous, "associatedWorktreeRef"),
        ),
        ("nextAssociatedWorktreeRef", next_awr.clone()),
        (
            "currentCreateBranchFlowCompleted",
            g(previous, "createBranchFlowCompleted"),
        ),
        (
            "nextCreateBranchFlowCompleted",
            gv(
                incoming,
                "createBranchFlowCompleted",
                "incoming.createBranchFlowCompleted",
            )?,
        ),
    ]);
    let mut m = Obj::new();
    mset(&mut m, "id", gv(incoming, "id", "incoming.id")?);
    mset(
        &mut m,
        "codexThreadId",
        gv_or_null(previous, "codexThreadId"),
    );
    mset(
        &mut m,
        "projectId",
        gv(incoming, "projectId", "incoming.projectId")?,
    );
    mset(&mut m, "title", gv(incoming, "title", "incoming.title")?);
    mset(&mut m, "modelSelection", model_selection);
    mset(
        &mut m,
        "runtimeMode",
        gv(incoming, "runtimeMode", "incoming.runtimeMode")?,
    );
    mset(
        &mut m,
        "interactionMode",
        gv(incoming, "interactionMode", "incoming.interactionMode")?,
    );
    mset(&mut m, "error", error);
    mset(
        &mut m,
        "createdAt",
        gv(incoming, "createdAt", "incoming.createdAt")?,
    );
    mset(
        &mut m,
        "archivedAt",
        or(&gv(incoming, "archivedAt", "incoming.archivedAt")?, JV::N),
    );
    mset(
        &mut m,
        "settledAt",
        or(&gv(incoming, "settledAt", "incoming.settledAt")?, JV::N),
    );
    mset(&mut m, "snoozedUntil", snoozed_until);
    mset(&mut m, "snoozeReminderAt", snooze_reminder_at);
    if !matches!(snooze_sequence, JV::U) {
        mset(&mut m, "snoozeSequence", snooze_sequence);
    }
    mset(
        &mut m,
        "updatedAt",
        gv(incoming, "updatedAt", "incoming.updatedAt")?,
    );
    mset(
        &mut m,
        "isPinned",
        or(&gv(incoming, "isPinned", "incoming.isPinned")?, b(false)),
    );
    mset(
        &mut m,
        "envMode",
        or(&gv(incoming, "envMode", "incoming.envMode")?, s("local")),
    );
    mset(&mut m, "branch", resolved_branch);
    mset(&mut m, "worktreePath", next_worktree_path);
    mset(&mut m, "workingDirectory", next_working_directory);
    mset(&mut m, "associatedWorktreePath", next_awp);
    mset(&mut m, "associatedWorktreeBranch", next_awb);
    mset(&mut m, "associatedWorktreeRef", next_awr);
    mset(&mut m, "createBranchFlowCompleted", resolved_cbfc);
    mset(
        &mut m,
        "parentThreadId",
        or(
            &gv(incoming, "parentThreadId", "incoming.parentThreadId")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "creationSource",
        or(
            &gv(incoming, "creationSource", "incoming.creationSource")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sourceThreadId",
        or(
            &gv(incoming, "sourceThreadId", "incoming.sourceThreadId")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "subagentAgentId",
        or(
            &gv(incoming, "subagentAgentId", "incoming.subagentAgentId")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "subagentNickname",
        or(
            &gv(incoming, "subagentNickname", "incoming.subagentNickname")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "subagentRole",
        or(
            &gv(incoming, "subagentRole", "incoming.subagentRole")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "forkSourceThreadId",
        or(
            &gv(
                incoming,
                "forkSourceThreadId",
                "incoming.forkSourceThreadId",
            )?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatSourceThreadId",
        or(
            &gv(
                incoming,
                "sidechatSourceThreadId",
                "incoming.sidechatSourceThreadId",
            )?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatContext",
        or(
            &gv(incoming, "sidechatContext", "incoming.sidechatContext")?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatLastActivityAt",
        or(
            &gv(
                incoming,
                "sidechatLastActivityAt",
                "incoming.sidechatLastActivityAt",
            )?,
            JV::N,
        ),
    );
    mset(
        &mut m,
        "sidechatExpiredAt",
        or(
            &gv(incoming, "sidechatExpiredAt", "incoming.sidechatExpiredAt")?,
            JV::N,
        ),
    );
    mset(&mut m, "lastKnownPr", last_known_pr);
    mset(&mut m, "pullRequests", pull_requests);
    mset(&mut m, "handoff", handoff);
    if truthy(&gv(
        incoming,
        "isProjectImport",
        "incoming.isProjectImport",
    )?) {
        mset(&mut m, "isProjectImport", b(true));
    }
    mset(&mut m, "usageLimitRecovery", usage_limit_recovery);
    if !matches!(usage_limit_recovery_sequence, JV::U) {
        mset(
            &mut m,
            "usageLimitRecoverySequence",
            usage_limit_recovery_sequence,
        );
    }
    let shell_ccr = claude_cache_review.clone();
    mset(&mut m, "claudeCacheReview", claude_cache_review);
    if !matches!(claude_cache_review_sequence, JV::U) {
        mset(
            &mut m,
            "claudeCacheReviewSequence",
            claude_cache_review_sequence,
        );
    }
    let pp = g(previous, "pinnedMessages");
    if !matches!(pp, JV::U) {
        mset(&mut m, "pinnedMessages", pp);
    }
    let pn = g(previous, "notes");
    if !matches!(pn, JV::U) {
        mset(&mut m, "notes", pn);
    }
    let pga = g(previous, "goalAchievements");
    if !matches!(pga, JV::U) {
        mset(&mut m, "goalAchievements", pga);
    }
    if !matches!(goal, JV::U) {
        mset(&mut m, "goal", goal);
    }
    if !matches!(goal_started_at, JV::U) {
        mset(&mut m, "goalStartedAt", goal_started_at);
    }
    if !matches!(goal_paused_at, JV::U) {
        mset(&mut m, "goalPausedAt", goal_paused_at);
    }
    let lhm = gv(
        incoming,
        "latestHumanMessageAt",
        "incoming.latestHumanMessageAt",
    )?;
    if !matches!(lhm, JV::U) {
        mset(&mut m, "latestHumanMessageAt", lhm);
    }
    if !matches!(
        gv(
            incoming,
            "latestUserMessageAt",
            "incoming.latestUserMessageAt"
        )?,
        JV::U
    ) {
        mset(
            &mut m,
            "latestUserMessageAt",
            or(
                &gv(
                    incoming,
                    "latestUserMessageAt",
                    "incoming.latestUserMessageAt",
                )?,
                JV::N,
            ),
        );
    }
    let hpa = gv(
        incoming,
        "hasPendingApprovals",
        "incoming.hasPendingApprovals",
    )?;
    if !matches!(hpa, JV::U) {
        mset(&mut m, "hasPendingApprovals", hpa);
    }
    let hui = gv(
        incoming,
        "hasPendingUserInput",
        "incoming.hasPendingUserInput",
    )?;
    if !matches!(hui, JV::U) {
        mset(&mut m, "hasPendingUserInput", hui);
    }
    let hap = gv(
        incoming,
        "hasActionableProposedPlan",
        "incoming.hasActionableProposedPlan",
    )?;
    if !matches!(hap, JV::U) {
        mset(&mut m, "hasActionableProposedPlan", hap);
    }
    let pbtc = gv(
        incoming,
        "pendingBackgroundTaskCount",
        "incoming.pendingBackgroundTaskCount",
    )?;
    if !matches!(pbtc, JV::U) {
        mset(&mut m, "pendingBackgroundTaskCount", pbtc);
    }
    let lrp = gv(
        incoming,
        "latestReplyPreview",
        "incoming.latestReplyPreview",
    )?;
    if !matches!(lrp, JV::U) {
        mset(&mut m, "latestReplyPreview", lrp);
    }
    let ppi = g(previous, "pendingInteractions");
    if !matches!(ppi, JV::U) {
        mset(&mut m, "pendingInteractions", ppi);
    }
    if !matches!(last_visited_at, JV::U) {
        mset(&mut m, "lastVisitedAt", last_visited_at);
    }
    let shell = o(m);
    let mut ts = Obj::new();
    mset(&mut ts, "latestTurn", latest_turn.clone());
    let lt_spp = g(&latest_turn, "sourceProposedPlan");
    if truthy(&lt_spp) {
        mset(&mut ts, "pendingSourceProposedPlan", lt_spp);
    }
    if streq(&g(&shell_ccr, "status"), "compacting") {
        mset(&mut ts, "pendingTurnStartMessageId", JV::N);
    } else {
        let pts = g(previous, "pendingTurnStartMessageId");
        if !matches!(pts, JV::U) {
            mset(&mut ts, "pendingTurnStartMessageId", pts);
        }
    }
    Ok((shell, session, o(ts)))
}
