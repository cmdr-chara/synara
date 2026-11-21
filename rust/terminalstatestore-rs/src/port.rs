//! The Rust port of `apps/web/src/terminalStateStore.ts`.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

const DEFAULT_TID: &str = "default";

fn default_map() -> Map<String, Value> {
    let Value::Object(m) = default_state() else {
        // INFALLIBLE: the literal below is always an object.
        unreachable!()
    };
    m
}

fn default_state() -> Value {
    json!({
        "entryPoint": "chat",
        "terminalOpen": false,
        "hasSession": false,
        "presentationMode": "workspace",
        "workspaceLayout": "both",
        "workspaceActiveTab": "terminal",
        "terminalIds": ["default"],
        "terminalLabelsById": { "default": "Terminal 1" },
        "terminalTitleOverridesById": {},
        "terminalCliKindsById": {},
        "terminalAttentionStatesById": {},
        "runningTerminalIds": [],
        "activeTerminalId": "default"
    })
}

/// `state.<key>` — null/undefined receivers throw TypeError; primitives box to no property.
fn field<'v>(state: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    match state {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        v if js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(o) => Ok(o.get(key)),
        _ => Ok(None),
    }
}

fn trim_or_err(v: &Value, who: &str) -> Result<String, String> {
    if let Some(u) = js::string_units(v) {
        let s = String::from_utf16_lossy(&u);
        return Ok(s.trim().to_string());
    }
    if let Value::String(s) = v {
        return Ok(s.trim().to_string());
    }
    Err(format!("{who}.trim is not a function"))
}

fn dedup_strings(it: impl Iterator<Item = String>) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    it.filter(|s| seen.insert(s.clone())).collect()
}

fn sanitize_terminal_ids(ids: Option<&Value>) -> Result<Vec<String>, String> {
    match ids {
        Some(Value::Array(a)) => {
            let mut out = Vec::with_capacity(a.len());
            for id in a {
                out.push(trim_or_err(id, "id")?);
            }
            let ids = dedup_strings(out.into_iter().filter(|s| !s.is_empty()));
            Ok(if ids.is_empty() {
                vec![DEFAULT_TID.to_string()]
            } else {
                ids
            })
        }
        Some(Value::Null) => Err("Cannot read properties of null (reading 'map')".into()),
        _ => Err("Cannot read properties of undefined (reading 'map')".into()),
    }
}

fn spread_iterable(v: &Value) -> Result<Vec<Value>, String> {
    match v {
        Value::Null => Ok(Vec::new()),
        x if js::is_undefined(x) => Ok(Vec::new()),
        Value::Array(a) => Ok(a.clone()),
        Value::String(s) => Ok(s.chars().map(|c| json!(c.to_string())).collect()),
        _ => Err("(intermediate value) is not iterable".into()),
    }
}

/// `new Set(items)` dedup for JSON values — SameValueZero on the wire-value range.
fn dedup_values(items: Vec<Value>) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    for item in items {
        if !out.iter().any(|x| js::js_eq(Some(x), Some(&item))) {
            out.push(item);
        }
    }
    out
}

fn normalize_running_ids(running: &Value, terminal_ids: &[String]) -> Result<Vec<String>, String> {
    if matches!(running, Value::Array(a) if a.is_empty()) {
        return Ok(Vec::new());
    }
    let items = match running {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'length')".into());
        }
        v if js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'length')".into());
        }
        v => spread_iterable(v)?,
    };
    let mut out = Vec::new();
    for id in dedup_values(items) {
        let t = trim_or_err(&id, "id")?;
        if !t.is_empty() && terminal_ids.contains(&t) {
            out.push(t);
        }
    }
    Ok(out)
}

/// `Object.fromEntries(entries.toSorted(localeCompare))` — last write wins.
fn sorted_record(
    raw: Option<&Value>,
    terminal_ids: &[String],
    keep_value: impl Fn(&Value) -> bool,
    trim_name: Option<&str>,
) -> Result<Map<String, Value>, String> {
    let raw = match raw {
        None | Some(Value::Null) => return Ok(Map::new()),
        Some(v) if js::is_undefined(v) => return Ok(Map::new()),
        Some(v) => v,
    };
    let mut entries: Vec<(String, Value)> = Vec::new();
    for (id, val) in js::entries(raw) {
        let id = id.trim().to_string();
        if let Some(name) = trim_name {
            let t = trim_or_err(&val, name)?;
            if id.is_empty() || t.is_empty() {
                continue;
            }
            entries.push((id, Value::String(t)));
        } else {
            if id.is_empty() || !keep_value(&val) {
                continue;
            }
            entries.push((id, val));
        }
    }
    entries.retain(|(id, _)| terminal_ids.contains(id));
    js::sort(&mut entries, |a, b| js::locale_compare(&a.0, &b.0));
    let mut out = Map::new();
    for (k, v) in entries {
        out.insert(k, v);
    }
    Ok(out)
}

fn cli_kind_base(kind: Option<&Value>) -> &'static str {
    match kind
        .and_then(js::string_units)
        .and_then(|u| String::from_utf16(&u).ok())
    {
        Some(ref s) if s == "codex" => "Codex",
        Some(ref s) if s == "claude" => "Claude",
        Some(ref s) if s == "antigravity" => "Antigravity",
        _ => "Terminal",
    }
}

fn display_title(id: &str, labels: &Map<String, Value>, overrides: &Map<String, Value>) -> String {
    for m in [overrides, labels] {
        if let Some(t) = m.get(id).and_then(js::string_units).and_then(|u| {
            let t = String::from_utf16_lossy(&u).trim().to_string();
            (!t.is_empty()).then_some(t)
        }) {
            return t;
        }
    }
    String::new()
}

fn unique_title(
    kind: Option<&Value>,
    exclude: &str,
    labels: &Map<String, Value>,
    overrides: &Map<String, Value>,
) -> String {
    let base = cli_kind_base(kind);
    let taken: std::collections::HashSet<String> = labels
        .keys()
        .filter(|id| id.as_str() != exclude)
        .map(|id| display_title(id, labels, overrides))
        .filter(|t| !t.is_empty())
        .collect();
    let mut i = 1u32;
    loop {
        let cand = format!("{base} {i}");
        if !taken.contains(&cand) {
            return cand;
        }
        i += 1;
    }
}

fn value_json(v: Option<&Value>) -> Option<String> {
    match v {
        None => None,
        Some(x) if js::is_undefined(x) => None,
        Some(x) => js::to_json(x).ok(),
    }
}

fn array_eq(left: Option<&Value>, right: &[Value]) -> bool {
    match left {
        Some(Value::Array(a)) => {
            a.len() == right.len()
                && a.iter()
                    .zip(right.iter())
                    .all(|(u, v)| js::js_eq(Some(u), Some(v)))
        }
        _ => right.is_empty(),
    }
}

fn state_equal(left: &Value, right: &Map<String, Value>) -> bool {
    let get = |k: &str| left.as_object().and_then(|o| o.get(k));
    let empty: &[Value] = &[];
    js::js_eq(get("entryPoint"), right.get("entryPoint"))
        && js::js_eq(get("terminalOpen"), right.get("terminalOpen"))
        && js::js_eq(get("hasSession"), right.get("hasSession"))
        && js::js_eq(get("presentationMode"), right.get("presentationMode"))
        && js::js_eq(get("workspaceLayout"), right.get("workspaceLayout"))
        && js::js_eq(get("workspaceActiveTab"), right.get("workspaceActiveTab"))
        && js::js_eq(get("activeTerminalId"), right.get("activeTerminalId"))
        && array_eq(
            get("terminalIds"),
            right["terminalIds"]
                .as_array()
                .map(|v| v.as_slice())
                .unwrap_or(empty),
        )
        && array_eq(
            get("runningTerminalIds"),
            right["runningTerminalIds"]
                .as_array()
                .map(|v| v.as_slice())
                .unwrap_or(empty),
        )
        && array_eq(
            get("retiredTerminalIds"),
            right
                .get("retiredTerminalIds")
                .and_then(Value::as_array)
                .map(|v| v.as_slice())
                .unwrap_or(empty),
        )
        && value_json(get("terminalLabelsById")) == value_json(right.get("terminalLabelsById"))
        && value_json(get("terminalTitleOverridesById"))
            == value_json(right.get("terminalTitleOverridesById"))
        && value_json(get("terminalCliKindsById")) == value_json(right.get("terminalCliKindsById"))
        && value_json(get("terminalAttentionStatesById"))
            == value_json(right.get("terminalAttentionStatesById"))
        && value_json(get("dockTerminalIdsByPaneId"))
            == value_json(right.get("dockTerminalIdsByPaneId"))
}

/// `normalizeThreadTerminalState`. Returns the normalized record plus whether the input
/// should be kept verbatim (`!hasLegacyLayout && state equal normalized`).
fn normalize(state: &Value) -> Result<(Value, bool), String> {
    let terminal_ids = sanitize_terminal_ids(field(state, "terminalIds")?)?;
    let active_raw = field(state, "activeTerminalId")?;
    let active = match active_raw {
        Some(v) => js::string_units(v)
            .and_then(|u| String::from_utf16(&u).ok())
            .filter(|s| terminal_ids.contains(s))
            .unwrap_or_else(|| terminal_ids[0].clone()),
        None => terminal_ids[0].clone(),
    };
    let dock = field(state, "dockTerminalIdsByPaneId")?;
    let next_ids: Vec<String> = if js::truthy(dock) {
        terminal_ids.clone()
    } else {
        vec![active.clone()]
    };
    let mut retired: Vec<Value> = Vec::new();
    if let Some(v) = field(state, "retiredTerminalIds")? {
        retired.extend(spread_iterable(v)?);
    }
    retired.extend(
        terminal_ids
            .iter()
            .filter(|id| !next_ids.contains(id))
            .map(|id| Value::String(id.clone())),
    );
    retired = dedup_values(retired);
    retired.retain(|v| {
        !js::string_units(v)
            .and_then(|u| String::from_utf16(&u).ok())
            .is_some_and(|s| next_ids.contains(&s))
    });

    let labels = sorted_record(
        field(state, "terminalLabelsById")?,
        &next_ids,
        |_| true,
        Some("label"),
    )?;
    let overrides = sorted_record(
        field(state, "terminalTitleOverridesById")?,
        &next_ids,
        |_| true,
        Some("titleOverride"),
    )?;
    let cli_kinds = sorted_record(
        field(state, "terminalCliKindsById")?,
        &next_ids,
        |v| {
            js::eq_str(Some(v), "codex")
                || js::eq_str(Some(v), "claude")
                || js::eq_str(Some(v), "antigravity")
        },
        None,
    )?;
    let attention = sorted_record(
        field(state, "terminalAttentionStatesById")?,
        &next_ids,
        |v| js::eq_str(Some(v), "attention") || js::eq_str(Some(v), "review"),
        None,
    )?;

    let mut ensured_labels = labels.clone();
    for id in &next_ids {
        let has_label = ensured_labels
            .get(id)
            .and_then(js::string_units)
            .is_some_and(|u| !String::from_utf16_lossy(&u).trim().is_empty());
        if !has_label {
            ensured_labels.insert(
                id.clone(),
                Value::String(unique_title(
                    cli_kinds.get(id),
                    id,
                    &ensured_labels,
                    &overrides,
                )),
            );
        }
    }

    let running = normalize_running_ids(
        field(state, "runningTerminalIds")?.unwrap_or(&js::undefined()),
        &next_ids,
    )?;

    let terminal_open = field(state, "terminalOpen")?
        .cloned()
        .unwrap_or_else(js::undefined);
    let has_session_raw = field(state, "hasSession")?;
    let has_session = if js::truthy(Some(&terminal_open)) {
        terminal_open.clone()
    } else {
        match has_session_raw {
            Some(v) if !v.is_null() && !js::is_undefined(v) => v.clone(),
            _ => Value::Bool(active != DEFAULT_TID),
        }
    };

    let mut out = Map::new();
    out.insert(
        "entryPoint".into(),
        if js::eq_str(field(state, "entryPoint")?, "terminal") {
            json!("terminal")
        } else {
            json!("chat")
        },
    );
    out.insert("terminalOpen".into(), terminal_open);
    out.insert("hasSession".into(), has_session);
    out.insert("presentationMode".into(), json!("workspace"));
    out.insert(
        "workspaceLayout".into(),
        if js::eq_str(field(state, "workspaceLayout")?, "terminal-only") {
            json!("terminal-only")
        } else {
            json!("both")
        },
    );
    out.insert(
        "workspaceActiveTab".into(),
        if js::eq_str(field(state, "workspaceActiveTab")?, "chat") {
            json!("chat")
        } else {
            json!("terminal")
        },
    );
    out.insert(
        "terminalIds".into(),
        Value::Array(next_ids.iter().cloned().map(Value::String).collect()),
    );
    out.insert("terminalLabelsById".into(), Value::Object(ensured_labels));
    out.insert(
        "terminalTitleOverridesById".into(),
        Value::Object(overrides),
    );
    out.insert("terminalCliKindsById".into(), Value::Object(cli_kinds));
    out.insert(
        "terminalAttentionStatesById".into(),
        Value::Object(attention),
    );
    out.insert(
        "runningTerminalIds".into(),
        Value::Array(running.into_iter().map(Value::String).collect()),
    );
    out.insert("activeTerminalId".into(), Value::String(active));
    if !retired.is_empty() {
        out.insert("retiredTerminalIds".into(), Value::Array(retired));
    }
    if let Some(d) = dock.filter(|v| js::truthy(Some(v))) {
        out.insert("dockTerminalIdsByPaneId".into(), d.clone());
    }

    let obj = match state {
        Value::Object(o) => o,
        // normalize only reaches here for objects; other receivers already failed.
        _ => return Ok((Value::Object(out), false)),
    };
    let has_legacy = obj.contains_key("terminalGroups")
        || obj.contains_key("activeTerminalGroupId")
        || obj.contains_key("terminalHeight");
    let keep = !has_legacy && state_equal(state, &out);
    Ok((Value::Object(out), keep))
}

/// `stripVolatileTerminalRuntimeState`.
fn strip_volatile(state: &Value) -> Result<Value, String> {
    let (normalized, _) = normalize(state)?;
    // INFALLIBLE: `normalize` always returns a Map-backed object.
    let obj = normalized.as_object().unwrap_or_else(|| unreachable!());
    let quiet = obj["runningTerminalIds"]
        .as_array()
        .is_some_and(|a| a.is_empty())
        && obj["terminalAttentionStatesById"]
            .as_object()
            .is_some_and(|o| o.is_empty());
    if quiet {
        return Ok(normalized);
    }
    let mut next = obj.clone();
    next.insert("terminalAttentionStatesById".into(), json!({}));
    next.insert("runningTerminalIds".into(), json!([]));
    Ok(Value::Object(next))
}

/// Port of `sanitizePersistedTerminalStateByThreadId` (`apps/web/src/terminalStateStore.ts:347`).
pub fn sanitize_persisted_terminal_state_by_thread_id(
    terminal_state_by_thread_id: Option<Value>,
) -> Result<Value, String> {
    let input = terminal_state_by_thread_id.unwrap_or(Value::Null);
    let mut next = Map::new();
    for (thread_id, state) in js::entries(&input) {
        if thread_id.starts_with("workspace:") {
            continue;
        }
        let sanitized = strip_volatile(&state)?;
        let (renormalized, _) = normalize(&sanitized)?;
        if !state_equal(&renormalized, &default_map()) {
            next.insert(thread_id, sanitized);
        }
    }
    Ok(Value::Object(next))
}

/// Port of `selectThreadTerminalState` (`apps/web/src/terminalStateStore.ts:792`).
pub fn select_thread_terminal_state(
    terminal_state_by_thread_id: Value,
    thread_id: Value,
) -> Result<Value, String> {
    // `threadId.length === 0`
    if thread_id.is_null() {
        return Err("Cannot read properties of null (reading 'length')".into());
    }
    if js::is_undefined(&thread_id) {
        return Err("Cannot read properties of undefined (reading 'length')".into());
    }
    let len_zero = match &thread_id {
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        _ => false,
    };
    if len_zero {
        return Ok(default_state());
    }
    // `terminalStateByThreadId[threadId]`
    if terminal_state_by_thread_id.is_null() {
        return Err(match &thread_id {
            Value::Array(_) => "Cannot read properties of null".into(),
            Value::Object(_) => "Cannot read properties of null (reading '#<Object>')".into(),
            other => format!(
                "Cannot read properties of null (reading '{}')",
                js::to_property_key(other)
            ),
        });
    }
    if js::is_undefined(&terminal_state_by_thread_id) {
        return Err(match &thread_id {
            Value::Array(_) => "Cannot read properties of undefined".into(),
            Value::Object(_) => "Cannot read properties of undefined (reading '#<Object>')".into(),
            other => format!(
                "Cannot read properties of undefined (reading '{}')",
                js::to_property_key(other)
            ),
        });
    }
    let key = js::to_property_key(&thread_id);
    let found = js::get_key(&terminal_state_by_thread_id, &key)?;
    if found.is_null() || js::is_undefined(&found) {
        return Ok(default_state());
    }
    Ok(found)
}
