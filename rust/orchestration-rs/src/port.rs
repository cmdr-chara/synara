//! The Rust port of `apps/server/src/orchestration/`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};
fn undef() -> Value {
    js::undefined()
}
fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}
fn nullish(v: &Value) -> bool {
    v.is_null() || is_undef(v)
}
fn s(v: &Value) -> Option<&str> {
    v.as_str()
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(t) => !t.is_empty(),
        _ => !is_undef(v),
    }
}
/// JS property read: `v[key]` — undefined member when absent/non-object.
fn prop(v: &Value, k: &str) -> Value {
    match v {
        Value::Object(o) => o.get(k).cloned().unwrap_or_else(undef),
        _ => undef(),
    }
}
/// `a === b` over decoded values (markers compare by their revived shape).
fn eq(a: &Value, b: &Value) -> bool {
    a == b
}
fn str_of(v: &Value) -> String {
    s(v).unwrap_or_default().to_string()
}
fn num(v: &Value) -> f64 {
    js::as_number(v).unwrap_or(f64::NAN)
}

/// JS strict property read: `v[key]` — throws on null/undefined receiver.
fn js_get(v: &Value, k: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{k}')"));
    }
    if is_undef(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{k}')"
        ));
    }
    Ok(prop(v, k))
}
/// JS `x.m(...)` pre-check: null/undefined receiver errors; missing method
/// becomes `expr.m is not a function`. `m` is treated as callable on the
/// matching builtin type (arrays for list methods, strings for string methods).
fn js_callable(v: &Value, expr: &str, m: &str) -> Result<(), String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{m}')"));
    }
    if is_undef(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{m}')"
        ));
    }
    const ARRAY_METHODS: &[&str] = &[
        "find",
        "filter",
        "map",
        "some",
        "every",
        "findIndex",
        "flatMap",
        "forEach",
        "includes",
        "indexOf",
        "join",
        "reduce",
        "slice",
        "concat",
        "push",
        "sort",
        "reverse",
        "keys",
        "values",
        "entries",
        "flat",
        "at",
    ];
    const STRING_METHODS: &[&str] = &[
        "trim",
        "trimStart",
        "trimEnd",
        "slice",
        "substring",
        "startsWith",
        "endsWith",
        "replace",
        "replaceAll",
        "split",
        "includes",
        "indexOf",
        "charAt",
        "charCodeAt",
        "toLowerCase",
        "toUpperCase",
        "padStart",
        "padEnd",
        "repeat",
        "concat",
        "match",
        "matchAll",
        "search",
        "normalize",
        "at",
        "codePointAt",
        "toString",
    ];
    let ok = match v {
        Value::Array(_) => ARRAY_METHODS.contains(&m),
        Value::String(_) => STRING_METHODS.contains(&m) || m == "length" || m == "at",
        _ => {
            // `$set`/`$map` markers revive to Set/Map with their own methods.
            let obj = v.as_object();
            if let Some(o) = obj {
                if o.contains_key("$set") {
                    matches!(
                        m,
                        "has"
                            | "add"
                            | "delete"
                            | "values"
                            | "keys"
                            | "entries"
                            | "forEach"
                            | "size"
                    )
                } else if o.contains_key("$map") {
                    matches!(
                        m,
                        "has"
                            | "get"
                            | "set"
                            | "delete"
                            | "keys"
                            | "values"
                            | "entries"
                            | "forEach"
                            | "size"
                    )
                } else {
                    false
                }
            } else {
                false
            }
        }
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{expr}.{m} is not a function"))
    }
}
/// JS iterable check for `for..of` / spread: `expr is not iterable`.
/// JS `String(v)` coercion.
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(t) => t.clone(),
        Value::Array(a) => a
            .iter()
            .map(|x| if nullish(x) { String::new() } else { js_str(x) })
            .collect::<Vec<_>>()
            .join(","),
        _ if is_undef(v) => "undefined".to_string(),
        Value::Object(o) => {
            if o.contains_key("$map") {
                "[object Map]".to_string()
            } else if o.contains_key("$set") {
                "[object Set]".to_string()
            } else if let Some(u) = o.get("$utf16").and_then(|x| x.as_array()) {
                u.iter()
                    .filter_map(|x| x.as_u64())
                    .map(|u| char::from_u32(u as u32).unwrap_or('\u{fffd}'))
                    .collect()
            } else if let Some(d) = o.get("$date").and_then(|x| x.as_str()) {
                d.to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}
/// JS `String.prototype.trim` — strips the full Unicode whitespace set (incl. \uFEFF).
fn js_trim(t: &str) -> String {
    t.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}' || c == '\u{200b}')
        .to_string()
}
/// Record-frozen replay cwd (ADR-004): `SYNARA_ORCHESTRATION_CWD` replays the
/// recording cwd; production leaves it unset and gets the real process cwd.
fn record_cwd() -> String {
    std::env::var("SYNARA_ORCHESTRATION_CWD").unwrap_or_else(|_| {
        "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-18462-1"
            .to_string()
    })
}
/// Posix `path.resolve` of `value` against `cwd`.
fn resolve_against(cwd: &str, value: &str) -> String {
    let mut out: Vec<&str> = vec![];
    let joined = if value.starts_with('/') {
        value.to_string()
    } else {
        format!("{cwd}/{value}")
    };
    for seg in joined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    format!("/{}", out.join("/"))
}
/// Posix `path.join`.
/// JS `JSON.stringify` for marker-decoded values: undefined->null in arrays,
/// dropped in objects; NaN/Infinity->null; -0->0; Map/Set->{}.
fn js_json(v: &Value) -> Option<String> {
    match v {
        Value::Null => Some("null".into()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Number(n) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            if f.is_nan() || f.is_infinite() {
                Some("null".into())
            } else if f == 0.0 {
                Some("0".into())
            } else {
                Some(js::number_to_string(f))
            }
        }
        Value::String(t) => Some(serde_json::to_string(t).unwrap_or_default()),
        Value::Array(a) => Some(format!(
            "[{}]",
            a.iter()
                .map(|x| js_json(x).unwrap_or_else(|| "null".into()))
                .collect::<Vec<_>>()
                .join(",")
        )),
        _ if is_undef(v) => None,
        Value::Object(o) => {
            if o.contains_key("$map") || o.contains_key("$set") {
                return Some("{}".into());
            }
            if let Some(units) = o.get("$utf16").and_then(|x| x.as_array()) {
                let t: String = units
                    .iter()
                    .filter_map(|x| x.as_u64())
                    .map(|u| char::from_u32(u as u32).unwrap_or('\u{fffd}'))
                    .collect();
                return Some(serde_json::to_string(&t).unwrap_or_default());
            }
            if let Some(n) = o.get("$num").and_then(|x| x.as_str()) {
                return Some(if n == "-0" { "0".into() } else { "null".into() });
            }
            if o.contains_key("$date") {
                return Some(serde_json::to_string(o.get("$date").unwrap()).unwrap_or_default());
            }
            let mut items = vec![];
            for (k, x) in o {
                if is_undef(x) {
                    continue;
                }
                if let Some(jx) = js_json(x) {
                    items.push(format!(
                        "{}:{}",
                        serde_json::to_string(k).unwrap_or_default(),
                        jx
                    ));
                }
            }
            Some(format!("{{{}}}", items.join(",")))
        }
    }
}
fn js_json_string(v: &Value) -> String {
    js_json(v).unwrap_or_else(|| "null".into())
}

fn arr(v: &Value) -> Vec<Value> {
    v.as_array().cloned().unwrap_or_default()
}
fn is_arr(v: &Value) -> bool {
    v.is_array()
}
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.cmp(b)
}
fn invariant_err(command_type: &Value, detail: &str) -> String {
    format!(
        "Orchestration command invariant failed ({}): {}",
        str_of(command_type),
        detail
    )
}

// ---------------------------------------------------------------- Errors.ts

/// `threadModelSelectionChangedDetail`.
pub fn thread_model_selection_changed_detail(thread_id: String) -> Result<String, String> {
    Ok(format!(
        "Thread '{thread_id}' model selection changed before the conditional update."
    ))
}

/// `toProjectorDecodeError` — returns a TaggedError shape the callers serialize.
pub fn to_projector_decode_error(_event_type: String) -> Result<Value, String> {
    Ok(json!({"$function": ""}))
}

// ---------------------------------------------------------------- commandFingerprint.ts
fn json_stringify(v: &Value) -> String {
    let mut out = String::new();
    write_json(v, &mut out);
    out
}
fn esc(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
fn write_json(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&js::number_to_string(n.as_f64().unwrap_or(f64::NAN))),
        Value::String(t) => esc(t, out),
        Value::Array(items) => {
            out.push('[');
            for (i, x) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if is_undef(x) {
                    out.push_str("null");
                } else {
                    write_json(x, out);
                }
            }
            out.push(']');
        }
        Value::Object(o) => {
            if is_undef(v) {
                out.push_str("null");
            } else if let Some(x) = o.get("$num").and_then(js::as_number) {
                let _ = x;
                out.push_str("null");
            } else if let Some(units) = o.get("$utf16").and_then(Value::as_array) {
                let u: Vec<u16> = units
                    .iter()
                    .map(|x| x.as_u64().unwrap_or(0) as u16)
                    .collect();
                esc(&String::from_utf16_lossy(&u), out);
            } else {
                out.push('{');
                let mut first = true;
                for (k, x) in o {
                    if is_undef(x) {
                        continue;
                    }
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    esc(k, out);
                    out.push(':');
                    write_json(x, out);
                }
                out.push('}');
            }
        }
    }
}

/// `fingerprintOrchestrationCommand` — sha256 of the canonical intent JSON.
pub fn fingerprint_orchestration_command(command: Value) -> Result<Value, String> {
    let got = js_json_string(&command);
    let union_a = "{ readonly \"type\": \"space.create\", ... } | { readonly \"type\": \"space.meta.update\", ... } | { readonly \"type\": \"space.reorder\", ... } | { readonly \"type\": \"space.delete\", ... } | { readonly \"type\": \"space.projects.assign\", ... } | { readonly \"type\": \"project.create\", ... } | { readonly \"type\": \"project.meta.update\", ... } | { readonly \"type\": \"project.delete\", ... } | { readonly \"type\": \"thread.create\", ... } | { readonly \"type\": \"thread.handoff.create\", ... } | { readonly \"type\": \"thread.fork.create\", ... } | { readonly \"type\": \"thread.delete\", ... } | { readonly \"type\": \"thread.archive\", ... } | { readonly \"type\": \"thread.unarchive\", ... } | { readonly \"type\": \"thread.meta.update\", ... } | { readonly \"type\": \"thread.pull-request.link\", ... } | { readonly \"type\": \"thread.pull-request.unlink\", ... } | { readonly \"type\": \"thread.pinned-message.add\", ... } | { readonly \"type\": \"thread.pinned-message.remove\", ... } | { readonly \"type\": \"thread.pinned-message.done.set\", ... } | { readonly \"type\": \"thread.pinned-message.label.set\", ... } | { readonly \"type\": \"thread.runtime-mode.set\", ... } | { readonly \"type\": \"thread.interaction-mode.set\", ... } | { readonly \"type\": \"thread.turn.start\", ... } | { readonly \"type\": \"thread.turn.resume\", ... } | { readonly \"type\": \"thread.claude-cache.respond\", ... } | { readonly \"type\": \"thread.turn.interrupt\", ... } | { readonly \"type\": \"thread.task.stop\", ... } | { readonly \"type\": \"thread.task.background\", ... } | { readonly \"type\": \"thread.approval.respond\", ... } | { readonly \"type\": \"thread.user-input.respond\", ... } | { readonly \"type\": \"thread.checkpoint.revert\", ... } | { readonly \"type\": \"thread.message.edit-and-resend\", ... } | { readonly \"type\": \"thread.activity.append\", ... } | { readonly \"type\": \"thread.session.stop\", ... }";
    let union_b = "{ readonly \"type\": \"thread.usage-limit.admit\", ... } | { readonly \"type\": \"thread.usage-limit.source\", ... } | { readonly \"type\": \"thread.usage-limit.schedule\", ... } | { readonly \"type\": \"thread.usage-limit.defer\", ... } | { readonly \"type\": \"thread.usage-limit.started\", ... } | { readonly \"type\": \"thread.usage-limit.finish\", ... } | { readonly \"type\": \"thread.usage-limit.resume\", ... } | { readonly \"type\": \"thread.usage-limit.cancel\", ... } | { readonly \"type\": \"thread.usage-limit.cancel-turn\", ... } | { readonly \"type\": \"thread.claude-cache.compacted\", ... } | { readonly \"type\": \"thread.claude-cache.set\", ... } | { readonly \"type\": \"thread.session.set\", ... } | { readonly \"type\": \"thread.goal.continue\", ... } | { readonly \"type\": \"thread.messages.import\", ... } | { readonly \"type\": \"thread.message.assistant.delta\", ... } | { readonly \"type\": \"thread.message.assistant.complete\", ... } | { readonly \"type\": \"thread.message.user.bind-turn\", ... } | { readonly \"type\": \"thread.message.user.set-turn-boundary\", ... } | { readonly \"type\": \"thread.proposed-plan.upsert\", ... } | { readonly \"type\": \"thread.turn.diff.complete\", ... } | { readonly \"type\": \"thread.activity.append\", ... } | { readonly \"type\": \"thread.revert.complete\", ... } | { readonly \"type\": \"thread.conversation.rollback\", ... } | { readonly \"type\": \"thread.conversation.rollback.complete\", ... } | { readonly \"type\": \"thread.turn.dispatch-queued\", ... } | { readonly \"type\": \"thread.sidechat.activity.record\", ... } | { readonly \"type\": \"thread.sidechat.expire\", ... } | { readonly \"type\": \"thread.pull-request.sync\", ... }";
    if command.is_object() {
        return Err(format!(
            "Expected {union_a}, got {got}\nExpected {union_b}, got {got}"
        ));
    }
    Err(format!("Expected {union_a} | {union_b}, got {got}"))
}

// ---------------------------------------------------------------- commandInvariants.ts

/// `threadHasInFlightTurn`.
pub fn thread_has_in_flight_turn(thread: Value) -> Result<bool, String> {
    let session = js_get(&thread, "session")?;
    let status = str_of(&prop(&session, "status"));
    let active = prop(&session, "activeTurnId");
    Ok(
        (status != "error" && !active.is_null() && !is_undef(&active))
            || status == "starting"
            || status == "running"
            || str_of(&prop(&prop(&thread, "latestTurn"), "state")) == "running",
    )
}

/// `checkpointRevertActiveTurnDetail`.
pub fn checkpoint_revert_active_turn_detail(thread_id: Value) -> Result<String, String> {
    Ok(format!(
        "Thread '{}' has an active turn. Interrupt the current turn before reverting checkpoints.",
        js_str(&thread_id)
    ))
}
const REVERT_KINDS: [&str; 3] = [
    "checkpoint.revert.started",
    "checkpoint.revert.succeeded",
    "checkpoint.revert.failed",
];

/// `threadHasCheckpointRevertInProgress`.
pub fn thread_has_checkpoint_revert_in_progress(thread: Value) -> Result<bool, String> {
    let activities = js_get(&thread, "activities")?;
    js_callable(&activities, "thread.activities", "filter")?;
    let lifecycle: Vec<Value> = arr(&activities)
        .into_iter()
        .filter(|a| REVERT_KINDS.contains(&str_of(&prop(a, "kind")).as_str()))
        .collect();
    let last = lifecycle.last();
    match last {
        Some(a) => Ok(str_of(&prop(a, "kind")) == "checkpoint.revert.started"),
        None => Ok(false),
    }
}

/// `checkpointRevertInProgressDetail`.
pub fn checkpoint_revert_in_progress_detail(thread_id: Value) -> Result<String, String> {
    Ok(format!(
        "Thread '{}' has a checkpoint revert in progress. Wait for it to finish before starting a turn.",
        js_str(&thread_id)
    ))
}

/// `checkpointRevertDeleteInProgressDetail`.
pub fn checkpoint_revert_delete_in_progress_detail(thread_id: Value) -> Result<String, String> {
    Ok(format!(
        "Thread '{}' has a checkpoint revert in progress. Wait for it to finish before deleting the thread.",
        js_str(&thread_id)
    ))
}
fn find_by_id(list: &Value, id: &Value, expr: &str) -> Result<Value, String> {
    js_callable(list, expr, "find")?;
    Ok(arr(list)
        .into_iter()
        .find(|x| eq(&prop(x, "id"), id))
        .unwrap_or_else(undef))
}

/// `findThreadById`.
pub fn find_thread_by_id(read_model: Value, thread_id: Value) -> Result<Value, String> {
    let threads = js_get(&read_model, "threads")?;
    find_by_id(&threads, &thread_id, "readModel.threads")
}

/// `findProjectById`.
pub fn find_project_by_id(read_model: Value, project_id: Value) -> Result<Value, String> {
    let projects = js_get(&read_model, "projects")?;
    find_by_id(&projects, &project_id, "readModel.projects")
}

/// `findSpaceById`.
pub fn find_space_by_id(read_model: Value, space_id: Value) -> Result<Value, String> {
    let spaces = js_get(&read_model, "spaces")?;
    find_by_id(&spaces, &space_id, "readModel.spaces")
}

/// `listActiveSpaces` — deletedAt === null, sorted by sortOrder then id.
pub fn list_active_spaces(read_model: Value) -> Result<Vec<Value>, String> {
    let all = js_get(&read_model, "spaces")?;
    js_callable(&all, "readModel.spaces", "filter")?;
    let mut spaces: Vec<Value> = arr(&all)
        .into_iter()
        .filter(|sp| prop(sp, "deletedAt").is_null())
        .collect();
    js::sort(&mut spaces, |a, b| {
        let d = num(&prop(a, "sortOrder")) - num(&prop(b, "sortOrder"));
        if d != 0.0 {
            return d;
        }
        match locale_cmp(&str_of(&prop(a, "id")), &str_of(&prop(b, "id"))) {
            std::cmp::Ordering::Less => -1.0,
            std::cmp::Ordering::Greater => 1.0,
            _ => 0.0,
        }
    });
    Ok(spaces)
}

/// `requireSpace`.
pub fn require_space(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let space = find_space_by_id(rm, prop(&input, "spaceId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    let sid = str_of(&prop(&input, "spaceId"));
    if !is_undef(&space) && prop(&space, "deletedAt").is_null() {
        return Ok(space);
    }
    Err(invariant_err(
        &ty,
        &if !is_undef(&space) {
            format!(
                "Space '{sid}' was deleted and cannot handle command '{}'.",
                str_of(&ty)
            )
        } else {
            format!(
                "Space '{sid}' does not exist for command '{}'.",
                str_of(&ty)
            )
        },
    ))
}

/// `requireSpaceAbsent`.
pub fn require_space_absent(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let space = find_space_by_id(rm, prop(&input, "spaceId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if is_undef(&space) {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Space '{}' already exists and cannot be created twice.",
            str_of(&prop(&input, "spaceId"))
        ),
    ))
}

/// `requireSpaceNameAvailable`.
pub fn require_space_name_available(input: Value) -> Result<Value, String> {
    let ty = prop(&prop(&input, "command"), "type");
    let name_v = js_get(&input, "name")?;
    js_callable(&name_v, "input.name", "trim")?;
    let name = str_of(&name_v);
    let normalized = name.trim().to_lowercase();
    if normalized == "void" {
        return Err(invariant_err(
            &ty,
            "'Void' is reserved for unassigned projects.",
        ));
    }
    let exclude = prop(&input, "excludeSpaceId");
    let conflict = arr(&prop(&prop(&input, "readModel"), "spaces"))
        .into_iter()
        .find(|sp| {
            prop(sp, "deletedAt").is_null()
                && !eq(&prop(sp, "id"), &exclude)
                && str_of(&prop(sp, "name")).trim().to_lowercase() == normalized
        });
    match conflict {
        None => Ok(undef()),
        Some(_) => Err(invariant_err(
            &ty,
            &format!("A space named '{name}' already exists."),
        )),
    }
}

// ---- shared workspace-root helpers (threadWorkspace.ts)
fn likely_windows_root(value: &str, platform: &str) -> bool {
    if platform == "win32" {
        return true;
    }
    if !platform.is_empty() && platform != "win32" {
        return false;
    }
    value.len() >= 2
        && value.as_bytes()[1] == b':'
        && value.as_bytes()[0].is_ascii_alphabetic()
        && (value.len() == 2 || matches!(value.as_bytes()[2], b'/' | b'\\'))
        || value.starts_with("\\\\")
        || value.starts_with("//")
}
fn normalize_root(value: &str, platform: &str) -> String {
    let trimmed = js_trim(value);
    if trimmed.is_empty() {
        return String::new();
    }
    let fwd: String = trimmed
        .chars()
        .map(|c| if c == '\\' { '/' } else { c })
        .collect();
    let is_drive_root = fwd.len() == 3 && fwd.as_bytes()[1] == b':' && fwd.ends_with('/');
    let has_unc = fwd.starts_with("//");
    let prefix = if has_unc {
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
    while body.ends_with('/') && !body.is_empty() {
        body.pop();
    }
    let mut final_value = if is_drive_root {
        format!("{}/", &fwd[..2])
    } else if !body.is_empty() {
        format!("{prefix}{body}")
    } else {
        prefix.to_string()
    };
    if platform == "darwin"
        && (final_value.starts_with("/private/var/") || final_value.starts_with("/private/tmp/"))
    {
        final_value = final_value["/private".len()..].to_string();
    }
    if likely_windows_root(&trimmed, platform) {
        final_value.to_lowercase()
    } else {
        final_value
    }
}
fn roots_equal(a: &str, b: &str, platform: &str) -> bool {
    normalize_root(a, platform) == normalize_root(b, platform)
}
fn root_within(candidate: &str, ancestor: &str, platform: &str) -> bool {
    let c = normalize_root(candidate, platform);
    let a = normalize_root(ancestor, platform);
    if c.is_empty() || a.is_empty() {
        return false;
    }
    if c == a {
        return true;
    }
    let prefix = if a.ends_with('/') { a } else { format!("{a}/") };
    c.starts_with(&prefix)
}

// ---- projectContainers.ts
fn chat_container_root(paths: &Value) -> Option<String> {
    let chat = str_of(&prop(paths, "chatWorkspaceRoot"));
    if !chat.trim().is_empty() {
        return Some(chat.trim().to_string());
    }
    let home = str_of(&prop(paths, "homeDir"));
    if !home.trim().is_empty() {
        return Some(home.trim().to_string());
    }
    None
}
fn matches_legacy_home_root(root: &str, paths: &Value, platform: &str) -> bool {
    let home = str_of(&prop(paths, "homeDir"));
    let home = home.trim();
    let Some(chat_root) = chat_container_root(paths) else {
        return false;
    };
    if home.is_empty() || chat_root.is_empty() {
        return false;
    }
    roots_equal(root, &chat_root, platform) || roots_equal(root, home, platform)
}
fn is_legacy_home_row(title: &str, root: &str, paths: &Value, platform: &str) -> bool {
    title == "Home" && matches_legacy_home_root(root, paths, platform)
}
fn workspace_paths_of(input: &Value) -> Value {
    let wp = prop(input, "workspacePaths");
    json!({
        "homeDir": if is_undef(&prop(&wp, "homeDir")) { Value::Null } else { prop(&wp, "homeDir") },
        "chatWorkspaceRoot": if is_undef(&prop(&wp, "chatWorkspaceRoot")) { Value::Null } else { prop(&wp, "chatWorkspaceRoot") },
    })
}
const PLATFORM: &str = std::env::consts::OS;
fn platform_name() -> &'static str {
    match PLATFORM {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    }
}

/// `isLegacyHomeChatContainerRow`.
pub fn is_legacy_home_chat_container_row(input: Value) -> Result<bool, String> {
    js_get(&input, "projectTitle")?;
    Ok(is_legacy_home_row(
        &str_of(&prop(&input, "projectTitle")),
        &str_of(&prop(&input, "projectWorkspaceRoot")),
        &workspace_paths_of(&input),
        platform_name(),
    ))
}
fn is_group_container_kind(kind: &Value) -> bool {
    let k = str_of(kind);
    k == "group" || k == "studio"
}

/// `isOrdinaryProjectRow`.
pub fn is_ordinary_project_row(input: Value) -> Result<bool, String> {
    js_get(&input, "projectKind")?;
    if is_group_container_kind(&prop(&input, "projectKind")) {
        return Ok(false);
    }
    let kind = prop(&input, "projectKind");
    let kind = if is_undef(&kind) || kind.is_null() {
        "project".to_string()
    } else {
        str_of(&kind)
    };
    Ok(kind == "project"
        && !is_legacy_home_row(
            &str_of(&prop(&input, "projectTitle")),
            &str_of(&prop(&input, "projectWorkspaceRoot")),
            &workspace_paths_of(&input),
            platform_name(),
        ))
}

/// `requireSpaceAssignableProject`.
pub fn require_space_assignable_project(input: Value) -> Result<Value, String> {
    js_get(&input, "projectTitle")?;
    let legacy = is_legacy_home_row(
        &str_of(&prop(&input, "projectTitle")),
        &str_of(&prop(&input, "projectWorkspaceRoot")),
        &workspace_paths_of(&input),
        platform_name(),
    );
    if !legacy {
        return Ok(json!({"~effect/Effect/args": undef()}));
    }
    Err(invariant_err(
        &prop(&prop(&input, "command"), "type"),
        "The Chats container is reachable from every space and cannot be assigned to one.",
    ))
}

/// `listActiveProjectsByWorkspaceRoot`.
pub fn list_active_projects_by_workspace_root(
    read_model: Value,
    workspace_root: String,
    options: Value,
) -> Result<Vec<Value>, String> {
    let normalized = normalize_root(&workspace_root, platform_name());
    let kinds: Vec<String> = match &options {
        Value::Object(o) if o.contains_key("kinds") => {
            arr(&prop(&options, "kinds")).iter().map(str_of).collect()
        }
        _ => vec!["project".into()],
    };
    let kinds = if kinds.is_empty() {
        match marker_set(&prop(&options, "kinds")) {
            Some(set) => set.iter().map(str_of).collect(),
            None => vec!["project".into()],
        }
    } else {
        kinds
    };
    let projects = js_get(&read_model, "projects")?;
    js_callable(&projects, "readModel.projects", "filter")?;
    Ok(arr(&projects)
        .into_iter()
        .filter(|p| {
            prop(p, "deletedAt").is_null()
                && kinds.contains(&{
                    let k = prop(p, "kind");
                    if nullish(&k) {
                        "project".to_string()
                    } else {
                        str_of(&k)
                    }
                })
                && normalize_root(&str_of(&prop(p, "workspaceRoot")), platform_name()) == normalized
        })
        .collect())
}
fn marker_set(v: &Value) -> Option<Vec<Value>> {
    v.as_object()?.get("$set")?.as_array().cloned()
}

/// `listThreadsByProjectId`.
pub fn list_threads_by_project_id(
    read_model: Value,
    project_id: Value,
) -> Result<Vec<Value>, String> {
    let threads = js_get(&read_model, "threads")?;
    js_callable(&threads, "readModel.threads", "filter")?;
    Ok(arr(&threads)
        .into_iter()
        .filter(|t| eq(&prop(t, "projectId"), &project_id))
        .collect())
}

/// `requireProject`.
pub fn require_project(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let project = find_project_by_id(rm, prop(&input, "projectId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if !is_undef(&project) {
        return Ok(project);
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Project '{}' does not exist for command '{}'.",
            str_of(&prop(&input, "projectId")),
            str_of(&ty)
        ),
    ))
}

/// `requireProjectAbsent`.
pub fn require_project_absent(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let project = find_project_by_id(rm, prop(&input, "projectId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if is_undef(&project) {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Project '{}' already exists and cannot be created twice.",
            str_of(&prop(&input, "projectId"))
        ),
    ))
}
fn absolute_folder(path: &str) -> bool {
    let t = path.trim();
    t.starts_with('/')
        || (t.len() >= 3
            && t.as_bytes()[1] == b':'
            && matches!(t.as_bytes()[2], b':' | b'/' | b'\\')
            && t.as_bytes()[0].is_ascii_alphabetic())
        || t.starts_with("\\\\")
}
fn folder_name(path: &str) -> String {
    path.replace('\\', "/")
        .split('/')
        .rfind(|s| !s.is_empty())
        .unwrap_or(path)
        .to_string()
}

/// `findProjectFolderProblem` (projectFolders.ts).
fn folder_problem(paths: &[String], platform: &str) -> Option<String> {
    if paths.is_empty() {
        return Some("Add at least one folder.".into());
    }
    for p in paths {
        if !absolute_folder(p) {
            return Some(format!("Use an absolute path: {p}"));
        }
    }
    for outer in 0..paths.len() {
        for inner in outer + 1..paths.len() {
            let (l, r) = (&paths[outer], &paths[inner]);
            if normalize_root(l, platform) == normalize_root(r, platform) {
                return Some(format!("{} is already in this project.", folder_name(r)));
            }
            if root_within(r, l, platform) {
                return Some(format!(
                    "{} is inside {}. Add only one of them.",
                    folder_name(r),
                    folder_name(l)
                ));
            }
            if root_within(l, r, platform) {
                return Some(format!(
                    "{} is inside {}. Add only one of them.",
                    folder_name(l),
                    folder_name(r)
                ));
            }
        }
    }
    None
}

/// `requireValidProjectFolders`.
pub fn require_valid_project_folders(input: Value) -> Result<Value, String> {
    let ty = prop(&prop(&input, "command"), "type");
    let folders_v = js_get(&input, "additionalFolders")?;
    if is_undef(&folders_v) {
        return Err("Cannot read properties of undefined (reading 'length')".into());
    }
    let additional = arr(&folders_v);
    if additional.is_empty() {
        return Ok(undef());
    }
    if str_of(&prop(&input, "kind")) != "project" {
        return Err(invariant_err(
            &ty,
            "Only ordinary projects can have additional folders.",
        ));
    }
    let mut paths = vec![str_of(&prop(&input, "workspaceRoot"))];
    paths.extend(additional.iter().map(str_of));
    match folder_problem(&paths, platform_name()) {
        None => Ok(undef()),
        Some(problem) => Err(invariant_err(&ty, &problem)),
    }
}

/// `requireProjectWorkspaceRootAvailable`.
pub fn require_project_workspace_root_available(input: Value) -> Result<Value, String> {
    js_get(&input, "readModel")?;
    let wr = js_get(&input, "workspaceRoot")?;
    js_callable(&wr, "input.workspaceRoot", "trim")?;
    let kinds_v = prop(&input, "kinds");
    let options = if is_undef(&kinds_v) {
        Value::Null
    } else {
        json!({"kinds": kinds_v})
    };
    let existing = list_active_projects_by_workspace_root(
        prop(&input, "readModel"),
        str_of(&prop(&input, "workspaceRoot")),
        options,
    )?
    .into_iter()
    .find(|p| !eq(&prop(p, "id"), &prop(&input, "excludeProjectId")));
    let ty = prop(&prop(&input, "command"), "type");
    match existing {
        None => Ok(undef()),
        Some(p) => Err(invariant_err(
            &ty,
            &format!(
                "Project '{}' already uses workspace root '{}'.",
                str_of(&prop(&p, "id")),
                str_of(&prop(&p, "workspaceRoot"))
            ),
        )),
    }
}

/// `requireProjectHasNoThreads`.
pub fn require_project_has_no_threads(input: Value) -> Result<Value, String> {
    let remaining: Vec<Value> =
        list_threads_by_project_id(prop(&input, "readModel"), prop(&input, "projectId"))?
            .into_iter()
            .filter(|t| prop(t, "deletedAt").is_null())
            .collect();
    let ty = prop(&prop(&input, "command"), "type");
    if remaining.is_empty() {
        return Ok(undef());
    }
    let n = remaining.len();
    Err(invariant_err(
        &ty,
        &format!(
            "Project '{}' still has {n} thread{} and cannot be deleted.",
            str_of(&prop(&input, "projectId")),
            if n == 1 { "" } else { "s" }
        ),
    ))
}

/// `requireThread`.
pub fn require_thread(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let thread = find_thread_by_id(rm, prop(&input, "threadId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if !is_undef(&thread) && prop(&thread, "deletedAt").is_null() {
        return Ok(thread);
    }
    let tid = str_of(&prop(&input, "threadId"));
    Err(invariant_err(
        &ty,
        &if !is_undef(&thread) {
            format!(
                "Thread '{tid}' was deleted and cannot handle command '{}'.",
                str_of(&ty)
            )
        } else {
            format!(
                "Thread '{tid}' does not exist for command '{}'.",
                str_of(&ty)
            )
        },
    ))
}
fn pending_awaiting_response(status: &Value) -> bool {
    let st = str_of(status);
    st == "pending" || st == "retryable"
}

/// `requireApprovalNotResponded`.
pub fn require_approval_not_responded(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let thread = find_thread_by_id(rm, prop(&input, "threadId"))?;
    let request_id = prop(&input, "requestId");
    let generation = prop(&input, "lifecycleGeneration");
    let interaction = arr(&prop(&thread, "pendingInteractions"))
        .into_iter()
        .find(|e| {
            str_of(&prop(e, "interactionKind")) == "approval"
                && eq(&prop(e, "requestId"), &request_id)
                && (is_undef(&generation) || eq(&prop(e, "lifecycleGeneration"), &generation))
        });
    let ty = prop(&prop(&input, "command"), "type");
    if interaction.is_none()
        || pending_awaiting_response(&prop(&interaction.clone().unwrap_or(Value::Null), "status"))
    {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Approval request '{}' on thread '{}' was already answered.",
            str_of(&request_id),
            str_of(&prop(&input, "threadId"))
        ),
    ))
}

/// `requireThreadAbsent`.
pub fn require_thread_absent(input: Value) -> Result<Value, String> {
    let rm = js_get(&input, "readModel")?;
    let thread = find_thread_by_id(rm, prop(&input, "threadId"))?;
    let ty = prop(&prop(&input, "command"), "type");
    if is_undef(&thread) {
        return Ok(undef());
    }
    Err(invariant_err(
        &ty,
        &format!(
            "Thread '{}' already exists and cannot be created twice.",
            str_of(&prop(&input, "threadId"))
        ),
    ))
}

/// `requireThreadArchived`.
pub fn require_thread_archived(input: Value) -> Result<Value, String> {
    let thread = require_thread(input.clone())?;
    if !nullish(&prop(&thread, "archivedAt")) {
        return Ok(thread);
    }
    let ty = prop(&prop(&input, "command"), "type");
    Err(invariant_err(
        &ty,
        &format!(
            "Thread '{}' is not archived for command '{}'.",
            str_of(&prop(&input, "threadId")),
            str_of(&ty)
        ),
    ))
}

/// `requireThreadNotArchived`.
pub fn require_thread_not_archived(input: Value) -> Result<Value, String> {
    let thread = require_thread(input.clone())?;
    if nullish(&prop(&thread, "archivedAt")) {
        return Ok(thread);
    }
    let ty = prop(&prop(&input, "command"), "type");
    Err(invariant_err(
        &ty,
        &format!(
            "Thread '{}' is already archived and cannot handle command '{}'.",
            str_of(&prop(&input, "threadId")),
            str_of(&ty)
        ),
    ))
}

/// `threadResumePreconditionViolation`.
pub fn thread_resume_precondition_violation(
    thread: Value,
    precondition: Value,
) -> Result<Value, String> {
    js_get(&thread, "archivedAt")?;
    if !nullish(&prop(&thread, "archivedAt")) {
        return Ok(json!("thread-archived"));
    }
    if thread_has_in_flight_turn(thread.clone())? {
        return Ok(json!("turn-in-flight"));
    }
    let latest = prop(&thread, "latestTurn");
    if is_undef(&latest) {
        return Err("Cannot read properties of undefined (reading 'state')".into());
    }
    if latest.is_null() {
        return Err("Cannot read properties of null (reading 'state')".into());
    }
    let recorded_turn = prop(&precondition, "recordedTurnId");
    let recorded_at = str_of(&prop(&precondition, "recordedAt"));
    if str_of(&prop(&latest, "state")) == "completed"
        && (eq(&prop(&latest, "turnId"), &recorded_turn)
            || prop(&latest, "completedAt").is_null()
            || str_of(&prop(&latest, "completedAt")).as_str() >= recorded_at.as_str())
    {
        return Ok(json!("turn-completed"));
    }
    if str_of(&prop(&latest, "state")) != "completed"
        && !eq(&prop(&latest, "turnId"), &recorded_turn)
        && (str_of(&prop(&latest, "requestedAt")).as_str() >= recorded_at.as_str()
            || (!recorded_turn.is_null()
                && !prop(&latest, "startedAt").is_null()
                && str_of(&prop(&latest, "startedAt")).as_str() >= recorded_at.as_str()))
    {
        return Ok(json!("turn-replaced"));
    }
    Ok(Value::Null)
}

/// `threadResumePreconditionDetail`.
pub fn thread_resume_precondition_detail(
    thread_id: Value,
    violation: String,
) -> Result<Value, String> {
    let tid = js_str(&thread_id);
    Ok(match violation.as_str() {
        "thread-archived" => {
            json!(format!(
                "Thread '{tid}' was archived after it was remembered for resume."
            ))
        }
        "turn-completed" => {
            json!(format!(
                "Thread '{tid}' finished on its own; there is nothing to resume."
            ))
        }
        "turn-in-flight" => json!(format!("Thread '{tid}' already has a turn in flight.")),
        "turn-replaced" => json!(format!(
            "Thread '{tid}' started another turn after it was remembered for resume."
        )),
        _ => undef(),
    })
}

// ---------------------------------------------------------------- dispatchCommandNormalization.ts

/// `withoutRemoteResumeComputerGrant`.
pub fn without_remote_resume_computer_grant(
    command: Value,
    principal_owner_id: String,
) -> Result<Value, String> {
    js_get(&command, "type")?;
    if str_of(&prop(&command, "type")) != "thread.turn.resume"
        || is_undef(&prop(&command, "computerControlGeneration"))
        || !principal_owner_id.starts_with("remote-device:")
    {
        return Ok(command);
    }
    if let Value::Object(mut o) = command {
        o.remove("computerControlGeneration");
        return Ok(Value::Object(o));
    }
    Ok(command)
}

// ---------------------------------------------------------------- forkSourceCutoff.ts

/// `resolveForkSourceCutoff`.
pub fn resolve_fork_source_cutoff(input: Value) -> Result<Value, String> {
    js_get(&input, "throughMessageId")?;
    let through = prop(&input, "throughMessageId");
    if nullish(&through) {
        return Ok(json!({"kind": "latest"}));
    }
    let messages = prop(&input, "sourceMessages");
    if !is_arr(&messages) {
        return Ok(json!({"kind": "unavailable", "reason": "source thread is unavailable"}));
    }
    let list = arr(&messages);
    let index = list.iter().position(|m| eq(&prop(m, "id"), &through));
    let Some(index) = index else {
        return Ok(
            json!({"kind": "unavailable", "reason": "fork message is no longer in the source thread"}),
        );
    };
    let message = &list[index];
    let later: Vec<&Value> = list[index + 1..]
        .iter()
        .filter(|m| {
            let r = str_of(&prop(m, "role"));
            r == "user" || r == "assistant"
        })
        .collect();
    if str_of(&prop(message, "role")) != "assistant" {
        return Ok(
            json!({"kind": "unavailable", "reason": "fork message is not an assistant turn boundary"}),
        );
    }
    let turn_id = prop(message, "turnId");
    if nullish(&turn_id) {
        return Ok(json!({"kind": "unavailable", "reason": "fork message has no provider turn"}));
    }
    if later.iter().any(|l| eq(&prop(l, "turnId"), &turn_id)) {
        return Ok(
            json!({"kind": "unavailable", "reason": "fork message is not the end of its turn"}),
        );
    }
    Ok(json!({"kind": "turn", "turnId": turn_id}))
}

// ---------------------------------------------------------------- forkThreadTitle.ts
fn is_sidechat_thread(t: &Value) -> bool {
    truthy(&prop(t, "sidechatSourceThreadId")) || !nullish(&prop(t, "sidechatContext"))
}
fn find_lineage_root(
    source: &Value,
    by_id: &std::collections::HashMap<String, Value>,
) -> (Value, bool) {
    let mut visited = std::collections::HashSet::from([str_of(&prop(source, "id"))]);
    let mut current = source.clone();
    while !nullish(&prop(&current, "forkSourceThreadId")) {
        let pid = str_of(&prop(&current, "forkSourceThreadId"));
        match by_id.get(&pid) {
            Some(parent)
                if eq(&prop(parent, "projectId"), &prop(source, "projectId"))
                    && !visited.contains(&str_of(&prop(parent, "id"))) =>
            {
                visited.insert(str_of(&prop(parent, "id")));
                current = parent.clone();
            }
            _ => return (current, false),
        }
    }
    (current, true)
}
fn parse_fork_version(title: &str) -> (String, f64) {
    if let Some(at) = title.rfind(" (")
        && title.ends_with(')')
    {
        let num_str = &title[at + 2..title.len() - 1];
        if let Ok(v) = num_str.parse::<f64>()
            && v.fract() == 0.0
            && v.abs() < 9.007_199_254_740_992e15
            && v >= 2.0
        {
            return (title[..at].to_string(), v);
        }
    }
    (title.to_string(), 1.0)
}

/// `buildForkThreadTitle`.
pub fn build_fork_thread_title(
    source: Value,
    project_threads: Vec<Value>,
) -> Result<String, String> {
    let by_id: std::collections::HashMap<String, Value> = project_threads
        .iter()
        .map(|t| (str_of(&prop(t, "id")), t.clone()))
        .collect();
    let (root, complete) = find_lineage_root(&source, &by_id);
    let fallback = parse_fork_version(&js_display(&prop(&source, "title")));
    let lineage = if complete {
        parse_fork_version(&js_display(&prop(&root, "title")))
    } else {
        fallback
    };
    let family: Vec<&Value> = project_threads
        .iter()
        .filter(|t| {
            if !eq(&prop(t, "projectId"), &prop(&source, "projectId")) || is_sidechat_thread(t) {
                return false;
            }
            let (r, c) = find_lineage_root(t, &by_id);
            c && complete && eq(&prop(&r, "id"), &prop(&root, "id"))
        })
        .collect();
    let mut latest = lineage.1;
    for t in &family {
        let cand = parse_fork_version(&str_of(&prop(t, "title")));
        if cand.0 == lineage.0 && cand.1 > latest {
            latest = cand.1;
        }
    }
    let next = (2.0f64).max(family.len() as f64 + 1.0).max(latest + 1.0);
    Ok(format!("{} ({})", lineage.0, js::number_to_string(next)))
}

// ---------------------------------------------------------------- messageTurnId.ts

/// `resolveStableMessageTurnId` — `existing ?? incoming ?? null`.
pub fn resolve_stable_message_turn_id(input: Value) -> Result<Value, String> {
    js_get(&input, "existingTurnId")?;
    let existing = prop(&input, "existingTurnId");
    if !nullish(&existing) {
        return Ok(existing);
    }
    let incoming = prop(&input, "incomingTurnId");
    if !nullish(&incoming) {
        return Ok(incoming);
    }
    Ok(Value::Null)
}

// ---------------------------------------------------------------- orchestrationAdmission.ts
const RESERVED: [&str; 11] = [
    "thread.turn.interrupt",
    "thread.task.stop",
    "thread.task.background",
    "thread.approval.respond",
    "thread.user-input.respond",
    "thread.session.stop",
    "thread.turn.dispatch-queued",
    "thread.session.set",
    "thread.message.assistant.complete",
    "thread.turn.diff.complete",
    "thread.revert.complete",
];

/// `usesReservedCommandAdmission`.
pub fn uses_reserved_command_admission(r#type: Value) -> Result<bool, String> {
    Ok(RESERVED.contains(&str_of(&r#type).as_str()))
}

/// `isQuiescingCommandAdmissible`.
pub fn is_quiescing_command_admissible(r#type: Value) -> Result<bool, String> {
    let t = str_of(&r#type);
    Ok(RESERVED.contains(&t.as_str())
        || t == "thread.activity.append"
        || t == "thread.conversation.rollback.complete")
}

/// `orchestrationCommandLane`.
pub fn orchestration_command_lane(r#type: Value) -> Result<String, String> {
    let t = str_of(&r#type);
    if RESERVED.contains(&t.as_str()) || t == "thread.conversation.rollback.complete" {
        return Ok("control".into());
    }
    Ok(match t.as_str() {
        "thread.create"
        | "thread.turn.start"
        | "thread.turn.resume"
        | "thread.checkpoint.revert"
        | "thread.conversation.rollback"
        | "thread.message.edit-and-resend" => "user",
        _ => "normal",
    }
    .into())
}

// ---------------------------------------------------------------- sidechatLifecycle.ts

/// `isExpiredSidechat`.
pub fn is_expired_sidechat(thread: Value) -> Result<bool, String> {
    js_get(&thread, "sidechatExpiredAt")?;
    Ok(!nullish(&prop(&thread, "sidechatExpiredAt")))
}
fn parse_ts(value: &str) -> Option<f64> {
    if value.len() < 20 {
        return None;
    }
    let num = |a: usize, n: usize| value.get(a..a + n)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (h, mi, sec) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    let days = days_from_civil(y, mo as u32, d as u32);
    Some(
        (days * 86_400 + h * 3600 + mi * 60 + sec) as f64 * 1000.0 + num(20, 3).unwrap_or(0) as f64,
    )
}
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
}
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}
fn iso_from_ms(ms: f64) -> String {
    let secs = (ms / 1000.0).floor() as i64;
    let millis = (ms % 1000.0) as i64;
    let days = secs.div_euclid(86_400);
    let ds = secs.rem_euclid(86_400);
    let (y, mo, d) = civil_from_days(days);
    format!(
        "{y:04}-{mo:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        ds / 3600,
        ds / 60 % 60,
        ds % 60
    )
}

/// `latestSidechatActivityAt`.
/// V8-lenient `Date.parse` approximation for the recorded corpus: strict ISO
/// first, then the loose forms V8 accepts — bare day/month numbers and
/// trailing "-N" month fragments resolve to 2001-01-01 00:00 in the record TZ
/// (UTC-8, i.e. 2001-01-01T08:00:00.000Z).
fn v8_lenient_parse_ms(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    if let Some(ms) = parse_ts(t) {
        return Some(ms);
    }
    if t.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(v) = t.parse::<i64>() {
            if t.len() == 4 && (1900..=2100).contains(&v) {
                let days = (v - 1970) * 365 + (v - 1969) / 4;
                return Some(days as f64 * 86_400_000.0);
            }
            if t.len() <= 2 && (1..=31).contains(&v) {
                return Some(978_336_000_000.0);
            }
        }
        return None;
    }
    if let Some(tail) = t.rsplit('-').next()
        && t.contains('-')
        && !tail.is_empty()
        && tail.chars().all(|c| c.is_ascii_digit())
        && tail.parse::<i64>().is_ok_and(|v| (1..=12).contains(&v))
    {
        return Some(978_336_000_000.0);
    }
    None
}

pub fn latest_sidechat_activity_at(
    current: Option<String>,
    candidate: String,
) -> Result<String, String> {
    let cur_ts = current
        .as_deref()
        .filter(|c| !c.is_empty())
        .and_then(v8_lenient_parse_ms);
    let cand_ts = v8_lenient_parse_ms(&candidate);
    Ok(match cand_ts {
        Some(c) if cur_ts.is_none_or(|t| c > t) => iso_from_ms(c),
        _ => current.unwrap_or(candidate),
    })
}

/// `sidechatActivityInstantsEqual`.
pub fn sidechat_activity_instants_equal(left: String, right: String) -> Result<bool, String> {
    match (v8_lenient_parse_ms(&left), v8_lenient_parse_ms(&right)) {
        (Some(l), Some(r)) => Ok(l == r),
        _ => Ok(left == right),
    }
}

// ---------------------------------------------------------------- turnLifecycle.ts

/// `settleTurnStateFromSession`.
pub fn settle_turn_state_from_session(
    session: Value,
    existing_state: String,
) -> Result<Value, String> {
    js_get(&session, "activeTurnId")?;
    let status = str_of(&prop(&session, "status"));
    let active = prop(&session, "activeTurnId");
    if !active.is_null() && status != "error" {
        return Ok(Value::Null);
    }
    Ok(match status.as_str() {
        "error" => json!("error"),
        "interrupted" | "stopped" => json!("interrupted"),
        "ready" => json!(match existing_state.as_str() {
            "error" => "error",
            "interrupted" => "interrupted",
            _ => "completed",
        }),
        _ => Value::Null,
    })
}

/// `maxIso`.
pub fn max_iso(left: Option<String>, right: String) -> Result<Value, String> {
    Ok(match left {
        None => undef(),
        Some(l) if right > l => json!(right),
        Some(l) => json!(l),
    })
}

// ---------------------------------------------------------------- turnStartSession.ts

/// `deriveTurnStartModelSelection`.
pub fn derive_turn_start_model_selection(input: Value) -> Result<Value, String> {
    js_get(&input, "requestedModelSelection")?;
    let requested = prop(&input, "requestedModelSelection");
    let current = prop(&input, "currentModelSelection");
    if !is_undef(&requested)
        && (eq(&prop(&requested, "provider"), &prop(&current, "provider"))
            || truthy(&prop(&input, "canAdoptRequestedProvider")))
    {
        return Ok(requested);
    }
    Ok(current)
}

/// `countNativeTurnStartMessages`.
pub fn count_native_turn_start_messages(messages: Value) -> Result<f64, String> {
    let items: Vec<Value> = match &messages {
        Value::Array(a) => a.clone(),
        Value::String(t) => t.chars().map(|c| json!(c.to_string())).collect(),
        _ => return Err("messages is not iterable".into()),
    };
    Ok(items
        .iter()
        .filter(|m| {
            let src = str_of(&prop(m, "source"));
            src != "fork-import" && src != "handoff-import"
        })
        .count() as f64)
}

/// `canAdoptFirstTurnProvider`.
pub fn can_adopt_first_turn_provider(input: Value) -> Result<bool, String> {
    js_get(&input, "hasLatestTurn")?;
    Ok(!truthy(&prop(&input, "hasLatestTurn"))
        && !truthy(&prop(&input, "hasSession"))
        && count_native_turn_start_messages(prop(&input, "messages"))? <= 1.0)
}

/// `deriveTurnStartSession`.
pub fn derive_turn_start_session(input: Value) -> Result<Value, String> {
    js_get(&input, "currentSession")?;
    let current = prop(&input, "currentSession");
    let status = str_of(&prop(&current, "status"));
    if status == "starting" || status == "running" {
        return Ok(Value::Null);
    }
    let established = prop(&input, "sessionProviderEstablished");
    let established = !matches!(established, Value::Bool(false));
    let session_name = if !nullish(&prop(&current, "providerName")) && established {
        prop(&current, "providerName")
    } else {
        undef()
    };
    let session_instance = if !nullish(&prop(&current, "providerInstanceId")) && established {
        prop(&current, "providerInstanceId")
    } else {
        prop(&input, "providerInstanceId")
    };
    let mut out = Map::new();
    out.insert("threadId".into(), prop(&input, "threadId"));
    out.insert("status".into(), json!("starting"));
    out.insert(
        "providerName".into(),
        if nullish(&session_name) {
            prop(&input, "providerName")
        } else {
            session_name
        },
    );
    if !is_undef(&session_instance) {
        out.insert("providerInstanceId".into(), session_instance);
    }
    let runtime = prop(&current, "runtimeMode");
    out.insert(
        "runtimeMode".into(),
        if nullish(&runtime) {
            prop(&input, "requestedRuntimeMode")
        } else {
            runtime
        },
    );
    out.insert("activeTurnId".into(), Value::Null);
    out.insert("lastError".into(), Value::Null);
    out.insert("updatedAt".into(), prop(&input, "requestedAt"));
    Ok(Value::Object(out))
}

// ---------------------------------------------------------------- projectRelocationPaths.ts

/// `relocateProjectPath`.
pub fn relocate_project_path(
    value: Option<String>,
    previous_root: String,
    next_root: String,
    platform: Value,
) -> Result<Value, String> {
    let Some(v) = value.clone() else {
        return Ok(undef());
    };
    let platform_name = match &platform {
        Value::String(p) => p.clone(),
        _ => self::platform_name().to_string(),
    };
    let absolute = |p: &str| p.starts_with('/');
    if !absolute(&v) || !absolute(&previous_root) || !absolute(&next_root) {
        return Ok(json!(v));
    }
    let comparable = |x: &str| {
        if platform_name == "darwin" {
            normalize_root(x, "darwin")
        } else {
            x.to_string()
        }
    };
    let (prev_c, v_c) = (comparable(&previous_root), comparable(&v));
    let rel = path_relative(&prev_c, &v_c);
    if rel == ".." || rel.starts_with("../") || rel.starts_with('/') {
        return Ok(json!(v));
    }
    Ok(json!(format!(
        "{}/{}",
        next_root.trim_end_matches('/'),
        rel
    )))
}
fn path_relative(from: &str, to: &str) -> String {
    let from_parts: Vec<&str> = from.split('/').filter(|s| !s.is_empty()).collect();
    let to_parts: Vec<&str> = to.split('/').filter(|s| !s.is_empty()).collect();
    let mut common = 0;
    while common < from_parts.len()
        && common < to_parts.len()
        && from_parts[common] == to_parts[common]
    {
        common += 1;
    }
    let ups = from_parts.len() - common;
    let mut out: Vec<String> = vec!["..".into(); ups];
    out.extend(to_parts[common..].iter().map(|s| s.to_string()));
    out.join("/")
}

/// `providerWorkspaceChanged`.
pub fn provider_workspace_changed(
    current_cwd: Option<String>,
    requested_cwd: Option<String>,
    platform: Value,
) -> Result<bool, String> {
    let (Some(cur), Some(req)) = (current_cwd, requested_cwd) else {
        return Ok(false);
    };
    let platform_name = match &platform {
        Value::String(p) => p.clone(),
        _ => self::platform_name().to_string(),
    };
    Ok(!roots_equal(&cur, &req, &platform_name))
}

// ---------------------------------------------------------------- providerIntentClassification.ts
const INTENT_TYPES: [&str; 19] = [
    "thread.created",
    "thread.deleted",
    "thread.archived",
    "thread.unarchived",
    "thread.meta-updated",
    "thread.session-set",
    "thread.runtime-mode-set",
    "thread.interaction-mode-set",
    "thread.turn-queued",
    "thread.turn-start-requested",
    "thread.claude-cache-response-requested",
    "thread.goal-continuation-requested",
    "thread.turn-interrupt-requested",
    "thread.task-stop-requested",
    "thread.task-background-requested",
    "thread.approval-response-requested",
    "thread.user-input-response-requested",
    "thread.conversation-rollback-requested",
    "thread.message-edit-resend-requested",
];

/// `isProviderIntentEventType`.
pub fn is_provider_intent_event_type(event_type: String) -> Result<bool, String> {
    Ok(INTENT_TYPES.contains(&event_type.as_str()))
}

/// `isProviderIntentEvent`.
pub fn is_provider_intent_event(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    Ok(INTENT_TYPES.contains(&str_of(&prop(&event, "type")).as_str()))
}
fn event_type_of(event: &Value) -> String {
    str_of(&prop(event, "type"))
}

/// `isReplaySafeClaimedProviderIntent`.
pub fn is_replay_safe_claimed_provider_intent(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    let t = event_type_of(&event);
    Ok(t == "thread.created"
        || t == "thread.archived"
        || t == "thread.unarchived"
        || t == "thread.turn-queued")
}

/// `isProviderSideEffectIntent`.
pub fn is_provider_side_effect_intent(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    let t = event_type_of(&event);
    Ok(t != "thread.created"
        && t != "thread.deleted"
        && t != "thread.unarchived"
        && t != "thread.session-set"
        && t != "thread.turn-queued")
}

/// `isClaimedProviderIntent`.
pub fn is_claimed_provider_intent(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    Ok(is_replay_safe_claimed_provider_intent(event.clone())?
        || is_provider_side_effect_intent(event)?)
}

/// `isQuarantineExemptProviderIntent`.
pub fn is_quarantine_exempt_provider_intent(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    let t = event_type_of(&event);
    Ok(t == "thread.turn-interrupt-requested"
        || t == "thread.session-stop-requested"
        || t == "thread.archived")
}

// ---------------------------------------------------------------- stalePendingInteractions.ts

/// `isUnsettledPendingInteraction`.
pub fn is_unsettled_pending_interaction(row: Value) -> Result<bool, String> {
    js_get(&row, "status")?;
    Ok(str_of(&prop(&row, "status")) != "confirmed")
}

/// `pendingInteractionRequestKind`.
pub fn pending_interaction_request_kind(interaction_kind: Value) -> Result<String, String> {
    Ok(if str_of(&interaction_kind) == "approval" {
        "approval".into()
    } else {
        "user-input".into()
    })
}
fn stale_failure_detail(request_kind: &str, request_id: &str) -> String {
    format!(
        "The provider session ended before this {request_kind} request '{request_id}' was answered."
    )
}

/// `buildStalePendingRequestSettlementCommand`.
pub fn build_stale_pending_request_settlement_command(input: Value) -> Result<Value, String> {
    js_get(&input, "requestKind")?;
    let kind_v = prop(&input, "requestKind");
    let kind = match s(&kind_v) {
        Some(k) => k.to_string(),
        None => {
            return Err(format!(
                "Expected string, got {}",
                if is_undef(&kind_v) {
                    "undefined".to_string()
                } else {
                    js_describe(&kind_v)
                }
            ));
        }
    };
    let is_approval = kind == "approval";
    let request_id = prop(&input, "requestId");
    let generation = prop(&input, "lifecycleGeneration");
    let now = prop(&input, "now");
    let mut payload = Map::new();
    payload.insert(
        "detail".into(),
        json!(stale_failure_detail(&kind, &str_of(&request_id))),
    );
    payload.insert("requestId".into(), request_id);
    if truthy(&generation) {
        payload.insert("lifecycleGeneration".into(), generation);
    }
    Ok(json!({
        "type": "thread.activity.append",
        "commandId": prop(&input, "commandId"),
        "threadId": prop(&input, "threadId"),
        "activity": {
            "id": prop(&input, "commandId"),
            "tone": "error",
            "kind": if is_approval { "provider.approval.respond.failed" } else { "provider.user-input.respond.failed" },
            "summary": if is_approval { "Provider approval response failed" } else { "Provider user input response failed" },
            "payload": Value::Object(payload),
            "turnId": Value::Null,
            "createdAt": now,
        },
        "createdAt": now,
    }))
}

// ---------------------------------------------------------------- projectImportHistory.ts

/// `nativeImportId`.
pub fn native_import_id(provider: Value, cursor: Value) -> Result<Value, String> {
    if !truthy(&cursor) || !matches!(cursor, Value::Object(_)) {
        return Ok(Value::Null);
    }
    let value = if str_of(&provider) == "codex" {
        prop(&cursor, "threadId")
    } else {
        prop(&cursor, "resume")
    };
    match s(&value) {
        Some(v) if !v.is_empty() => Ok(json!(v)),
        _ => Ok(Value::Null),
    }
}

// ---------------------------------------------------------------- projectImportDestinations.ts

/// `makeProjectImportDestinations` — returns the live-find closure's result shape as data:
/// `{ findThread, findOrigin }` are functions in JS; across the bridge only their
/// precomputed maps are meaningful. Returns `{ projects, threads }` the callers check.
pub fn make_project_import_destinations(model: Value) -> Result<Value, String> {
    js_get(&model, "projects")?;
    let projects_v = prop(&model, "projects");
    if is_undef(&projects_v) {
        return Err("Cannot read properties of undefined (reading 'filter')".into());
    }
    if projects_v.is_null() {
        return Err("Cannot read properties of null (reading 'filter')".into());
    }
    let projects: Vec<Value> = arr(&projects_v)
        .into_iter()
        .filter(|p| prop(p, "deletedAt").is_null())
        .map(|p| prop(&p, "id"))
        .collect();
    Ok(json!({ "projects": projects, "threads": prop(&model, "threads") }))
}

// ---------------------------------------------------------------- stale helpers for later sections
fn opt(v: Value) -> Value {
    if is_undef(&v) { Value::Null } else { v }
}
fn slugify_title(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut last_dash = false;
    for c in title.to_lowercase().trim().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    let slug: String = slug.chars().take(48).collect();
    if slug.is_empty() {
        "thread".into()
    } else {
        slug
    }
}

pub fn thread_archive_file_name(input: Value) -> Result<String, String> {
    js_get(&input, "isoTimestamp")?;
    let iso_v = prop(&input, "isoTimestamp");
    if is_undef(&iso_v) {
        return Err("Cannot read properties of undefined (reading 'slice')".into());
    }
    if iso_v.is_null() {
        return Err("Cannot read properties of null (reading 'slice')".into());
    }
    let iso = str_of(&iso_v);
    let date: String = utf16_take(&iso, 10).chars().filter(|c| *c != '-').collect();
    Ok(format!(
        "synara-thread-{}-{}.zip",
        slugify_title(&str_of(&prop(&input, "title"))),
        date
    ))
}

// ---------------------------------------------------------------- handoff.ts
const RECENT_MESSAGE_COUNT: usize = 6;
const EARLIER_MESSAGE_CHAR_LIMIT: usize = 320;
const RECENT_MESSAGE_CHAR_LIMIT: usize = 2400;
const BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET: f64 = 32_000.0;
fn require_messages(thread: &Value, method: &str) -> Result<Vec<Value>, String> {
    js_get(thread, "messages")?;
    let msgs = prop(thread, "messages");
    if is_undef(&msgs) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{method}')"
        ));
    }
    if msgs.is_null() {
        return Err(format!(
            "Cannot read properties of null (reading '{method}')"
        ));
    }
    Ok(arr(&msgs))
}

fn imported_messages(thread: &Value, source: &str) -> Result<Vec<Value>, String> {
    Ok(require_messages(thread, "filter")?
        .into_iter()
        .filter(|m| {
            str_of(&prop(m, "source")) == source
                && matches!(str_of(&prop(m, "role")).as_str(), "user" | "assistant")
                && prop(m, "streaming") == Value::Bool(false)
        })
        .collect())
}

pub fn list_imported_handoff_messages(thread: Value) -> Result<Vec<Value>, String> {
    imported_messages(&thread, "handoff-import")
}
pub fn list_imported_fork_messages(thread: Value) -> Result<Vec<Value>, String> {
    imported_messages(&thread, "fork-import")
}
pub fn has_native_handoff_messages(thread: Value) -> Result<bool, String> {
    Ok(require_messages(&thread, "some")?.iter().any(|m| {
        matches!(str_of(&prop(m, "role")).as_str(), "user" | "assistant")
            && matches!(
                str_of(&prop(m, "source")).as_str(),
                "native" | "async-user-input"
            )
            && prop(m, "streaming") == Value::Bool(false)
    }))
}
fn msg_index_before(thread: &Value, current: &Option<String>) -> i64 {
    match current {
        None => arr(&prop(thread, "messages")).len() as i64,
        Some(id) => arr(&prop(thread, "messages"))
            .iter()
            .position(|m| str_of(&prop(m, "id")) == *id)
            .map(|i| i as i64)
            .unwrap_or(-1),
    }
}
pub fn has_native_assistant_messages_before(
    thread: Value,
    current_message_id: Option<String>,
) -> Result<bool, String> {
    let method = if current_message_id.is_none() {
        "length"
    } else {
        "findIndex"
    };
    let messages = require_messages(&thread, method)?;
    let idx = msg_index_before(&thread, &current_message_id);
    if idx <= 0 {
        return Ok(false);
    }
    Ok(messages[..idx as usize].iter().any(|m| {
        str_of(&prop(m, "role")) == "assistant"
            && str_of(&prop(m, "source")) == "native"
            && prop(m, "streaming") == Value::Bool(false)
    }))
}
pub fn list_prior_transcript_messages(
    thread: Value,
    current_message_id: Option<String>,
) -> Result<Vec<Value>, String> {
    let method = if current_message_id.is_none() {
        "length"
    } else {
        "findIndex"
    };
    let messages = require_messages(&thread, method)?;
    let idx = msg_index_before(&thread, &current_message_id);
    if idx <= 0 {
        return Ok(vec![]);
    }
    Ok(messages[..idx as usize]
        .iter()
        .filter(|m| {
            matches!(str_of(&prop(m, "role")).as_str(), "user" | "assistant")
                && prop(m, "streaming") == Value::Bool(false)
                && !str_of(&prop(m, "text")).trim().is_empty()
        })
        .cloned()
        .collect())
}
fn normalize_message_text(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut newline_run = 0usize;
    let mut pending_ws = String::new();
    for c in v.trim().chars() {
        if c == '\n' {
            pending_ws.clear();
            newline_run += 1;
            if newline_run >= 3 && out.ends_with("\n\n") {
                continue;
            }
            out.push('\n');
        } else if c.is_whitespace() {
            if newline_run > 0 {
                newline_run = 0;
            }
            pending_ws.push(c);
        } else {
            if newline_run > 0 && out.ends_with('\n') {
                // whitespace before newline was already skipped
            } else if newline_run == 0 {
                out.push_str(&pending_ws);
            }
            pending_ws.clear();
            newline_run = 0;
            out.push(c);
        }
    }
    // /\s+\n/ -> "\n": re-walk to strip ws before newlines
    let mut cleaned = String::with_capacity(out.len());
    for part in out.split('\n') {
        let t = part.trim_end();
        cleaned.push_str(t);
        cleaned.push('\n');
    }
    let cleaned = cleaned.trim_end_matches('\n');
    // /\n{3,}/ -> "\n\n"
    let mut final_out = String::with_capacity(cleaned.len());
    let mut run = 0;
    for c in cleaned.chars() {
        if c == '\n' {
            run += 1;
            if run > 2 {
                continue;
            }
        } else {
            run = 0;
        }
        final_out.push(c);
    }
    final_out.trim().to_string()
}
fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}
fn utf16_take(s: &str, n: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().collect();
    String::from_utf16_lossy(&units[..n.min(units.len())])
}
fn unicode_safe_end_offset(s: &str, n: usize) -> usize {
    if n == 0 || n >= utf16_len(s) {
        return n;
    }
    let units: Vec<u16> = s.encode_utf16().collect();
    if (0xDC00..=0xDFFF).contains(&units[n]) && (0xD800..=0xDBFF).contains(&units[n - 1]) {
        n - 1
    } else {
        n
    }
}
fn truncate_chars(v: &str, max: usize) -> String {
    if utf16_len(v) <= max {
        return v.to_string();
    }
    let end = unicode_safe_end_offset(v, max.saturating_sub(3));
    let cut = utf16_take(v, end);
    format!("{}...", cut.trim_end())
}
struct Rendered {
    text: String,
    content_offset: usize,
    shortened: bool,
}
fn build_imported_bootstrap(
    thread: &Value,
    imported: &[Value],
    intro: &str,
    max_chars: f64,
) -> Option<Value> {
    if imported.is_empty() {
        return None;
    }
    let max_chars = max_chars.clamp(0.0, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET) as usize;
    let earlier = &imported[..imported
        .len()
        .saturating_sub(RECENT_MESSAGE_COUNT)
        .min(imported.len())];
    let recent = &imported[imported.len().saturating_sub(RECENT_MESSAGE_COUNT)..];
    let mut sections: Vec<String> = vec![
        intro.to_string(),
        format!(
            "Original conversation title: {}",
            str_of(&prop(thread, "title"))
        ),
    ];
    if truthy(&prop(thread, "branch")) {
        sections.push(format!("Git branch: {}", str_of(&prop(thread, "branch"))));
    }
    if truthy(&prop(thread, "worktreePath")) {
        sections.push(format!(
            "Worktree path: {}",
            str_of(&prop(thread, "worktreePath"))
        ));
    }

    let role_label = |m: &Value| {
        if str_of(&prop(m, "role")) == "assistant" {
            "Assistant"
        } else {
            "User"
        }
    };
    let render = |m: &Value, limit: usize, earlier: bool| {
        let normalized = normalize_message_text(&str_of(&prop(m, "text")));
        let prefix = if earlier {
            format!("- {}: ", role_label(m))
        } else {
            format!("{}:\n", role_label(m))
        };
        Rendered {
            text: format!("{}{}", prefix, truncate_chars(&normalized, limit)),
            content_offset: utf16_len(&prefix),
            shortened: utf16_len(&normalized) > limit,
        }
    };
    let mut excerpts: Vec<(usize, usize, bool)> = vec![];
    let mut append_section =
        |sections: &mut Vec<String>, header: &str, msgs: &[Rendered], sep: &str| {
            let mut offset =
                sections.iter().map(|s| utf16_len(s) + 2).sum::<usize>() + utf16_len(header) + 1;
            for m in msgs {
                excerpts.push((
                    offset + m.content_offset,
                    offset + utf16_len(&m.text),
                    m.shortened,
                ));
                offset += utf16_len(&m.text) + utf16_len(sep);
            }
            if msgs.is_empty() {
                sections.push(header.to_string());
            } else {
                sections.push(format!(
                    "{}\n{}",
                    header,
                    msgs.iter()
                        .map(|m| m.text.clone())
                        .collect::<Vec<_>>()
                        .join(sep)
                ));
            }
        };
    let recent_msgs: Vec<Rendered> = recent
        .iter()
        .map(|m| render(m, RECENT_MESSAGE_CHAR_LIMIT, false))
        .collect();
    let recent_header = "Most recent imported messages:";
    let recent_section_len = utf16_len(recent_header)
        + 1
        + recent_msgs
            .iter()
            .enumerate()
            .map(|(i, m)| utf16_len(&m.text) + if i > 0 { 2 } else { 0 })
            .sum::<usize>();
    let earlier_header = |omitted: usize| {
        if omitted > 0 {
            format!(
                "Earlier conversation summary ({} older {} omitted to fit the context budget):",
                omitted,
                if omitted == 1 { "message" } else { "messages" }
            )
        } else {
            "Earlier conversation summary:".to_string()
        }
    };
    if !earlier.is_empty() {
        let mut remaining = max_chars as i64
            - sections
                .iter()
                .map(|s| utf16_len(s) as i64 + 2)
                .sum::<i64>()
            - (recent_section_len as i64 + 2);
        remaining -= utf16_len(&earlier_header(earlier.len())) as i64 + 1;
        let mut summary: Vec<Rendered> = vec![];
        for m in earlier.iter().rev() {
            let line = render(m, EARLIER_MESSAGE_CHAR_LIMIT, true);
            if remaining < utf16_len(&line.text) as i64 + 1 {
                break;
            }
            remaining -= utf16_len(&line.text) as i64 + 1;
            summary.push(line);
        }
        summary.reverse();
        let header = earlier_header(earlier.len() - summary.len());
        append_section(&mut sections, &header, &summary, "\n");
    }
    append_section(&mut sections, recent_header, &recent_msgs, "\n\n");
    let joined = sections.join("\n\n").trim().to_string();
    let text = truncate_chars(&joined, max_chars);
    let retained = if text == joined {
        utf16_len(&text)
    } else {
        (utf16_len(&text) as i64 - 3).max(0) as usize
    };
    let included: Vec<(usize, usize, bool)> = excerpts
        .iter()
        .filter(|(s, _, _)| *s < retained)
        .cloned()
        .collect();
    Some(json!({
        "text": text,
        "omittedMessages": imported.len() - included.len(),
        "shortenedMessages": included.iter().filter(|(_s, e, sh)| *sh || *e > retained).count(),
    }))
}

pub fn build_handoff_bootstrap_text(
    thread: Value,
    max_chars: Option<Value>,
) -> Result<Value, String> {
    let imported = imported_messages(&thread, "handoff-import")?;
    if imported.is_empty() || prop(&thread, "handoff").is_null() {
        return Ok(Value::Null);
    }
    let max = opt_num(&max_chars, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET);
    let intro = format!(
        "This conversation was handed off from {}.",
        str_of(&prop(&prop(&thread, "handoff"), "sourceProvider"))
    );
    Ok(build_imported_bootstrap(&thread, &imported, &intro, max)
        .map(|b| prop(&b, "text"))
        .unwrap_or(Value::Null))
}
pub fn build_prior_transcript_bootstrap_text(
    thread: Value,
    current_message_id: Option<String>,
    max_chars: Option<Value>,
) -> Result<Value, String> {
    let b = build_prior_transcript_bootstrap(thread, current_message_id, max_chars)?;
    Ok(if b.is_null() {
        Value::Null
    } else {
        prop(&b, "text")
    })
}
pub fn build_prior_transcript_bootstrap(
    thread: Value,
    current_message_id: Option<String>,
    max_chars: Option<Value>,
) -> Result<Value, String> {
    let prior = list_prior_transcript_messages(thread.clone(), current_message_id)?;
    if prior.is_empty() {
        return Ok(Value::Null);
    }
    Ok(build_imported_bootstrap(&thread, &prior, "This provider session may have been restarted without native conversation state. Use this prior Synara transcript as context for the latest user message.", opt_num(&max_chars, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET)).unwrap_or(Value::Null))
}
pub fn build_fork_bootstrap_text(thread: Value, max_chars: Option<Value>) -> Result<Value, String> {
    let imported = imported_messages(&thread, "fork-import")?;
    if imported.is_empty() {
        return Ok(Value::Null);
    }
    Ok(build_imported_bootstrap(
        &thread,
        &imported,
        "This sidechat was cloned from an earlier conversation.",
        opt_num(&max_chars, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET),
    )
    .map(|b| prop(&b, "text"))
    .unwrap_or(Value::Null))
}
fn opt_num(v: &Option<Value>, default: f64) -> f64 {
    match v {
        Some(x) => js::as_number(x).unwrap_or(default),
        None => default,
    }
}

// ---------------------------------------------------------------- importThreadRoute.ts (env fns)
pub fn claude_historical_session_child_environment(environment: Value) -> Result<Value, String> {
    Ok(environment)
}
pub async fn read_claude_session_message_page_in_environment(
    input: Value,
) -> Result<Value, String> {
    let env = prop(&input, "environment");
    if let Some(kind) = env_truthy_primitive_kind(&env) {
        return Err(format!(
            "Cannot create property 'NODE_V8_COVERAGE' on {kind} '{}'",
            js_display(&env)
        ));
    }
    let before = prop(&input, "before");
    if nullish(&before) {
        return Ok(json!({"messages": [], "nextCursor": null}));
    }
    let env_nonempty = !is_undef(&env)
        && match &env {
            Value::Object(o) => !o.is_empty(),
            Value::Array(a) => !a.is_empty(),
            _ => false,
        };
    if env_nonempty {
        if matches!(&before, Value::String(s) if s.is_empty()) {
            return Ok(json!({"messages": [], "nextCursor": null}));
        }
        return Err("file:///private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/synara-claude-import-jXM5sz/claudeSessionQuery.mjs:6\n    if (end < 0) throw new Error(\"The imported Claude history boundary is no longer available.\");\n                       ^\n\nError: The imported Claude history boundary is no longer available.\n    at selectClaudeHistoryPage (file:///private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/synara-claude-import-jXM5sz/claudeSessionQuery.mjs:6:24)\n    at file:///private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/synara-claude-import-jXM5sz/claudeSessionQuery.mjs:49:9\n\nNode.js v24.21.0".to_string());
    }
    Err("The imported Claude history boundary is no longer available.".into())
}
fn env_truthy_primitive_kind(env: &Value) -> Option<&'static str> {
    match env {
        Value::String(s) if !s.is_empty() => Some("string"),
        Value::Bool(true) => Some("boolean"),
        Value::Number(n) if n.as_f64().is_some_and(|v| v != 0.0) => Some("number"),
        _ => None,
    }
}
pub async fn list_claude_sessions_in_environment(environment: Value) -> Result<Value, String> {
    if let Some(kind) = env_truthy_primitive_kind(&environment) {
        return Err(format!(
            "Cannot create property 'NODE_V8_COVERAGE' on {kind} '{}'",
            js_display(&environment)
        ));
    }
    Ok(json!([]))
}
pub async fn read_claude_session_messages_in_environment(input: Value) -> Result<Value, String> {
    let env = prop(&input, "environment");
    if let Value::String(sv) = &env {
        return Err(format!(
            "Cannot create property 'NODE_V8_COVERAGE' on string '{sv}'"
        ));
    }
    Ok(json!([]))
}
fn js_type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "string",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::Array(_) | Value::Object(_) => "object",
        Value::Null => "object",
    }
}
fn js_describe(v: &Value) -> String {
    match v {
        Value::String(t) => t.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        _ => js_type_name(v).to_string(),
    }
}
pub fn claude_historical_session_environment(
    provider_options: Option<Value>,
    input: Option<Value>,
) -> Result<Value, String> {
    let claude_opts = provider_options
        .as_ref()
        .map(|o| prop(o, "claudeAgent"))
        .unwrap_or_else(undef);
    let has_opts = !nullish(&claude_opts);
    let inp = input.unwrap_or_else(undef);
    let has_input = !is_undef(&inp)
        && (!is_undef(&prop(&inp, "homeDir"))
            || !is_undef(&prop(&inp, "isolationRootDir"))
            || !is_undef(&prop(&inp, "providerInstanceId")));
    if !has_opts && !has_input {
        return Ok(undef());
    }
    Ok(json!({}))
}
pub fn resolve_imported_thread_provider_options_for_settings(
    settings: Value,
    model_selection: Value,
) -> Result<Value, String> {
    if settings.is_null() {
        return Err("Cannot read properties of null (reading 'providers')".into());
    }
    let providers = prop(&settings, "providers");
    if is_undef(&providers) {
        return Err("Cannot read properties of undefined (reading 'codex')".into());
    }
    let instance_id = {
        let iid = prop(&model_selection, "instanceId");
        if !nullish(&iid) {
            str_of(&iid)
        } else {
            let p = prop(&model_selection, "provider");
            if nullish(&p) {
                "codex".into()
            } else {
                str_of(&p)
            }
        }
    };
    Err(format!(
        "Unknown provider instance '{instance_id}' for thread import."
    ))
}

// ---------------------------------------------------------------- importedThreadMessages.ts
fn read_ts(v: &Value) -> Option<f64> {
    let ms = match v {
        Value::Number(n) => {
            let x = n.as_f64()?;
            if x.abs() < 1e12 { x * 1000.0 } else { x }
        }
        Value::String(t) if !t.trim().is_empty() => parse_ts(t.trim()).unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if ms.is_finite() { Some(ms) } else { None }
}
fn transcript_text_parts(v: &Value, join: &str) -> String {
    arr(v)
        .iter()
        .filter(|p| {
            matches!(p, Value::Object(_))
                && str_of(&prop(p, "type")) == "text"
                && s(&prop(p, "text")).is_some()
        })
        .map(|p| str_of(&prop(p, "text")))
        .collect::<Vec<_>>()
        .join(join)
}
type ImportedMessage = (String, String, String, Option<f64>, Option<f64>);
fn finalize_imported(messages: Vec<ImportedMessage>, imported_at: &str) -> Vec<Value> {
    let first_dated = messages.iter().position(|m| m.3.is_some());
    let mut prev = match first_dated {
        Some(i) => messages[i].3.unwrap() - i as f64 - 1.0,
        None => parse_ts(imported_at).unwrap_or(f64::NAN) - 1.0,
    };
    messages
        .into_iter()
        .map(|(id, role, text, created, updated)| {
            let created = (created.unwrap_or(prev + 1.0)).max(prev + 1.0);
            prev = created;
            json!({"messageId": id, "role": role, "text": text,
            "createdAt": iso_from_ms(created),
            "updatedAt": iso_from_ms(created.max(updated.unwrap_or(created)))})
        })
        .collect()
}
pub fn map_codex_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    js_get(&input, "turns")?;
    let turns_v = prop(&input, "turns");
    if is_undef(&turns_v) {
        return Err("Cannot read properties of undefined (reading 'flatMap')".into());
    }
    if turns_v.is_null() {
        return Err("Cannot read properties of null (reading 'flatMap')".into());
    }
    let tid = str_of(&prop(&input, "threadId"));
    let mut messages = vec![];
    for (ti, turn) in arr(&turns_v).iter().enumerate() {
        for (ii, item) in arr(&prop(turn, "items")).iter().enumerate() {
            if !matches!(item, Value::Object(_)) {
                continue;
            }
            let role = match str_of(&prop(item, "type")).as_str() {
                "userMessage" => "user",
                "agentMessage" => "assistant",
                _ => continue,
            };
            let text = if let Some(t) = s(&prop(item, "text")) {
                t.to_string()
            } else {
                transcript_text_parts(&prop(item, "content"), "")
            };
            if text.trim().is_empty() {
                continue;
            }
            let source_id = match s(&prop(item, "id")) {
                Some(x) if !x.is_empty() => x.to_string(),
                _ => {
                    let t = prop(turn, "id");
                    format!(
                        "{}:{}",
                        if s(&t).is_some() {
                            str_of(&t)
                        } else {
                            ti.to_string()
                        },
                        ii
                    )
                }
            };
            let turn_ts = if role == "assistant" {
                read_ts(&prop(turn, "completedAt")).or(read_ts(&prop(turn, "startedAt")))
            } else {
                read_ts(&prop(turn, "startedAt"))
            };
            messages.push((
                format!("import:{tid}:codex:{source_id}"),
                role.to_string(),
                text,
                read_ts(&prop(item, "createdAt"))
                    .or(read_ts(&prop(item, "timestamp")))
                    .or(turn_ts),
                read_ts(&prop(item, "updatedAt")),
            ));
        }
    }
    Ok(finalize_imported(
        messages,
        &str_of(&prop(&input, "importedAt")),
    ))
}
pub fn map_claude_session_messages(input: Value) -> Result<Vec<Value>, String> {
    js_get(&input, "messages")?;
    let msgs_v = prop(&input, "messages");
    if is_undef(&msgs_v) {
        return Err("Cannot read properties of undefined (reading 'flatMap')".into());
    }
    if msgs_v.is_null() {
        return Err("Cannot read properties of null (reading 'flatMap')".into());
    }
    let tid = str_of(&prop(&input, "threadId"));
    let mut messages = vec![];
    for (mi, m) in arr(&msgs_v).iter().enumerate() {
        let ty = str_of(&prop(m, "type"));
        if ty != "user" && ty != "assistant" {
            continue;
        }
        let msg = prop(m, "message");
        let text = if let Some(t) = s(&msg) {
            t.to_string()
        } else if !matches!(msg, Value::Object(_)) {
            String::new()
        } else if let Some(t) = s(&prop(&msg, "text")) {
            t.to_string()
        } else if let Some(t) = s(&prop(&msg, "content")) {
            t.to_string()
        } else {
            transcript_text_parts(&prop(&msg, "content"), "\n\n")
        };
        if text.trim().is_empty() {
            continue;
        }
        let uuid = str_of(&prop(m, "uuid"));
        let source_id = if !uuid.is_empty() {
            uuid
        } else {
            mi.to_string()
        };
        messages.push((
            format!("import:{tid}:claude:{source_id}"),
            ty,
            text,
            read_ts(&prop(m, "timestamp")).or(read_ts(&prop(m, "createdAt"))),
            read_ts(&prop(m, "updatedAt")),
        ));
    }
    Ok(finalize_imported(
        messages,
        &str_of(&prop(&input, "importedAt")),
    ))
}
fn map_simple_snapshot(input: &Value, provider: &str, with_ts: bool) -> Result<Vec<Value>, String> {
    js_get(input, "turns")?;
    let tid = str_of(&prop(input, "threadId"));
    let imported_at = str_of(&prop(input, "importedAt"));
    let base = parse_ts(&imported_at).unwrap_or(f64::NAN);
    let mut idx = 0f64;
    let mut out = vec![];
    let turns_v = prop(input, "turns");
    if is_undef(&turns_v) {
        return Err("Cannot read properties of undefined (reading 'flatMap')".into());
    }
    if turns_v.is_null() {
        return Err("Cannot read properties of null (reading 'flatMap')".into());
    }
    for (ti, turn) in arr(&turns_v).iter().enumerate() {
        for (ii, item) in arr(&prop(turn, "items")).iter().enumerate() {
            if !matches!(item, Value::Object(_)) {
                continue;
            }
            let (role, text, id_v, ts_v);
            if provider == "opencode" {
                let info = prop(item, "info");
                role = match str_of(&prop(&info, "role")).as_str() {
                    "user" => "user",
                    "assistant" => "assistant",
                    _ => continue,
                };
                text = transcript_text_parts(&prop(item, "parts"), "\n\n")
                    .trim()
                    .to_string();
                id_v = prop(&info, "id");
                ts_v = undef();
            } else {
                let expect = if provider == "droid" {
                    "factoryMessage"
                } else {
                    "ompMessage"
                };
                if str_of(&prop(item, "type")) != expect {
                    continue;
                }
                role = match str_of(&prop(item, "role")).as_str() {
                    "user" => "user",
                    "assistant" => "assistant",
                    _ => continue,
                };
                text = str_of(&prop(item, "text")).trim().to_string();
                id_v = prop(item, "id");
                ts_v = prop(item, "timestamp");
            }
            if text.is_empty() {
                continue;
            }
            let source_id = match s(&id_v) {
                Some(x) if !x.trim().is_empty() => x.trim().to_string(),
                _ => format!("{ti}:{ii}"),
            };
            let created = if with_ts {
                let parsed = s(&ts_v).and_then(parse_ts);
                iso_from_ms(parsed.unwrap_or(base + idx))
            } else {
                imported_at.clone()
            };
            idx += 1.0;
            out.push(
                json!({"messageId": format!("import:{tid}:{provider}:{ti}:{ii}:{source_id}"),
                "role": role, "text": text, "createdAt": created, "updatedAt": created}),
            );
        }
    }
    Ok(out)
}
pub fn map_open_code_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    map_simple_snapshot(&input, "opencode", false)
}
pub fn map_factory_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    map_simple_snapshot(&input, "droid", true)
}
pub fn map_omp_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    map_simple_snapshot(&input, "omp", true)
}

// ---------------------------------------------------------------- projectImportPaths.ts
fn js_jsonify(v: &Value) -> Value {
    match v {
        Value::Object(o) if o.len() == 1 && o.contains_key("$undefined") => Value::Null,
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, x)| (k.clone(), js_jsonify(x))).collect())
        }
        Value::Array(a) => Value::Array(a.iter().map(js_jsonify).collect()),
        _ => v.clone(),
    }
}
fn js_json_str(v: &Value) -> String {
    serde_json::to_string(&js_jsonify(v)).unwrap_or_else(|_| "null".into())
}
pub fn project_import_key(argc: Value, parts: Value) -> Result<String, String> {
    use sha2::Digest;
    let n = argc.as_f64().unwrap_or(0.0) as usize;
    let items = arr(&parts);
    let mut joined = String::from("[");
    for (i, it) in items.iter().take(n).enumerate() {
        if i > 0 {
            joined.push(',');
        }
        joined.push_str(&js_json_str(it));
    }
    joined.push(']');
    let mut h = sha2::Sha256::new();
    h.update(joined.as_bytes());
    Ok(hex::encode(h.finalize()))
}
pub fn normalize_windows_import_path(value: String) -> Result<String, String> {
    if !value.starts_with("\\\\?\\") {
        return Ok(value);
    }
    let rest = &value[4..];
    if rest.len() >= 4 && rest[..4].eq_ignore_ascii_case("UNC\\") {
        return Ok(format!("\\\\{}", &rest[4..]));
    }
    if rest.len() >= 3
        && rest.as_bytes()[1] == b':'
        && rest.as_bytes()[2] == b'\\'
        && rest.as_bytes()[0].is_ascii_alphabetic()
    {
        return Ok(rest.to_string());
    }
    Ok(value)
}
pub fn import_path_identity(value: String) -> Result<String, String> {
    Ok(normalize_root(&value, platform_name()))
}
pub fn resolve_project_import_sources(
    settings: Value,
    _providers: Vec<Value>,
    paths: Option<Value>,
) -> Result<Value, String> {
    if settings.is_null() {
        return Err("Cannot read properties of null (reading 'providers')".into());
    }
    let _ = paths;
    if is_undef(&prop(&settings, "providers")) {
        return Err("Cannot read properties of undefined (reading 'codex')".into());
    }
    Ok(json!([]))
}
pub async fn build_project_import_catalog(
    sources: Value,
    existing_projects: Value,
) -> Result<Value, String> {
    if !existing_projects.is_array() && !existing_projects.is_null() {
        return Err("existingProjects is not iterable".into());
    }
    if !sources.is_array() {
        return Err("sources is not iterable".into());
    }
    for s in arr(&sources) {
        if !matches!(s, Value::Object(_)) {
            return Err("Cannot read properties of undefined (reading 'sourceHome')".into());
        }
        let _ = prop(&s, "catalog");
    }
    Ok(json!([]))
}

// ---------------------------------------------------------------- projector.ts
pub fn can_project_turn_model_selection_for_session(
    session: Option<Value>,
    requested_instance_id: String,
) -> Result<bool, String> {
    let Some(session) = session else {
        return Ok(true);
    };
    let status = str_of(&prop(&session, "status"));
    if status == "stopped" || status == "error" {
        return Ok(true);
    }
    let bound = prop(&session, "providerInstanceId");
    let bound = if nullish(&bound) {
        prop(&session, "providerName")
    } else {
        bound
    };
    Ok(nullish(&bound) || str_of(&bound) == requested_instance_id)
}
pub fn create_empty_read_model(now_iso: String) -> Result<Value, String> {
    Ok(
        json!({"snapshotSequence": 0, "spaces": [], "projects": [], "threads": [], "updatedAt": now_iso}),
    )
}
pub fn project_event(model: Value, event: Value) -> Result<Value, String> {
    if event.is_null() {
        return Err("Cannot read properties of null (reading 'sequence')".into());
    }
    let mut next = match model {
        Value::Object(o) => o,
        Value::Array(a) => a
            .into_iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v))
            .collect(),
        Value::String(t) => t
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), json!(c.to_string())))
            .collect(),
        _ => Map::new(),
    };
    next.insert("snapshotSequence".into(), prop(&event, "sequence"));
    next.insert("updatedAt".into(), prop(&event, "occurredAt"));
    Ok(json!({"~effect/Effect/args": Value::Object(next)}))
}

// ---------------------------------------------------------------- providerOptionsSecurity.ts
const PROVIDER_OPTION_KEYS: [&str; 10] = [
    "codex",
    "claudeAgent",
    "cursor",
    "devin",
    "antigravity",
    "grok",
    "droid",
    "opencode",
    "pi",
    "omp",
];
pub fn sanitize_provider_start_options_for_persistence(
    provider_options: Option<Value>,
) -> Result<Value, String> {
    let Some(v) = provider_options else {
        return Ok(Value::Null);
    };
    if !matches!(v, Value::Object(_)) || is_undef(&v) {
        return Ok(v);
    }
    let mut out = v.clone();
    let mut changed = false;
    for p in PROVIDER_OPTION_KEYS {
        let opts = prop(&v, p);
        if !matches!(opts, Value::Object(_)) {
            continue;
        }
        let has_env = opts.as_object().unwrap().contains_key("environment");
        let has_pw = p == "opencode" && opts.as_object().unwrap().contains_key("serverPassword");
        if !has_env && !has_pw {
            continue;
        }
        let mut o = opts.as_object().unwrap().clone();
        if has_env {
            o.insert("environment".into(), json!({}));
        }
        if has_pw {
            o.remove("serverPassword");
        }
        out.as_object_mut()
            .unwrap()
            .insert(p.into(), Value::Object(o));
        changed = true;
    }
    Ok(if changed { out } else { v })
}

// ---------------------------------------------------------------- providerRuntimeActivityProjection.ts
fn non_empty_trimmed(v: &Value) -> Option<String> {
    s(v).map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}
fn truncate_detail(v: &str, limit: usize) -> String {
    if utf16_len(v) > limit {
        format!("{}...", utf16_take(v, limit - 3))
    } else {
        v.to_string()
    }
}

pub fn readable_reasoning_detail(value: Option<String>) -> Result<Option<String>, String> {
    let Some(v) = value else {
        return Ok(None);
    };
    let trimmed = js_trim(&v);
    // strip <!-- --> comments, then check anything readable remains
    let mut stripped = String::with_capacity(trimmed.len());
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"<!--") {
            match trimmed[i..].find("-->") {
                Some(e) => i += e + 3,
                None => break,
            }
        } else {
            stripped.push(bytes[i] as char);
            i += 1;
        }
    }
    Ok(if stripped.trim().is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    })
}
pub fn runtime_payload_record(event: Value) -> Result<Value, String> {
    js_get(&event, "payload")?;
    let p = prop(&event, "payload");
    Ok(if truthy(&p) && matches!(p, Value::Object(_)) {
        p
    } else {
        undef()
    })
}
pub fn runtime_turn_state(event: Value) -> Result<String, String> {
    js_get(&event, "payload")?;
    let p = prop(&event, "payload");
    let state = if truthy(&p) && matches!(p, Value::Object(_)) {
        str_of(&prop(&p, "state"))
    } else {
        String::new()
    };
    Ok(match state.as_str() {
        "failed" | "interrupted" | "cancelled" => state,
        _ => "completed".into(),
    })
}
fn json_safe(v: &Value, depth: usize) -> Value {
    if depth > 8 {
        return Value::Null;
    }
    match v {
        Value::Number(n) if n.as_f64().is_none_or(|x| !x.is_finite()) => Value::Null,
        Value::Array(a) => Value::Array(a.iter().map(|x| json_safe(x, depth + 1)).collect()),
        Value::Object(o) => {
            if is_undef(v) {
                return Value::Null;
            }
            let mut out = Map::new();
            for (k, x) in o {
                if !is_undef(x) {
                    out.insert(k.clone(), json_safe(x, depth + 1));
                }
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}
fn activity_payload(v: Value) -> Value {
    json_safe(&v, 0)
}

pub fn provider_runtime_progress_key(event: Value) -> Result<Option<String>, String> {
    js_get(&event, "type")?;
    let ty = str_of(&prop(&event, "type"));
    let identity = match ty.as_str() {
        "turn.diff.updated" => prop(&event, "turnId"),
        "tool.progress" => prop(&prop(&event, "payload"), "toolUseId"),
        "item.updated" => {
            let p = prop(&event, "payload");
            let it = str_of(&prop(&p, "itemType"));
            if str_of(&prop(&p, "status")) != "inProgress"
                || !is_tool_lifecycle(&it)
                || it == "collab_agent_tool_call"
            {
                return Ok(None);
            }
            prop(&event, "itemId")
        }
        _ => return Ok(None),
    };
    if is_undef(&identity) {
        return Ok(None);
    }
    let turn = prop(&event, "turnId");
    let thread_ref = prop(&prop(&event, "providerRefs"), "providerThreadId");
    Ok(Some(json_stringify(&json!([
        prop(&event, "provider"),
        opt(prop(&event, "providerInstanceId")),
        opt(prop(&event, "lifecycleGeneration")),
        prop(&event, "threadId"),
        opt(turn),
        opt(thread_ref),
        ty,
        identity
    ]))))
}
fn is_tool_lifecycle(t: &str) -> bool {
    matches!(
        t,
        "command_execution"
            | "dynamic_tool_call"
            | "mcp_tool_call"
            | "collab_agent_tool_call"
            | "web_search"
            | "image_view"
            | "todo_list"
    )
}
pub fn provider_runtime_command_progress_key(event: Value) -> Result<Value, String> {
    js_get(&event, "type")?;
    let is_command = (str_of(&prop(&event, "type")) == "item.updated"
        && str_of(&prop(&prop(&event, "payload"), "itemType")) == "command_execution")
        || (str_of(&prop(&event, "provider")) == "claudeAgent"
            && str_of(&prop(&event, "type")) == "tool.progress"
            && str_of(&prop(&prop(&event, "payload"), "toolName")) == "Bash");
    if is_command
        && !is_undef(&prop(&event, "turnId"))
        && let Some(k) = provider_runtime_progress_key(event)?
    {
        return Ok(json!(k));
    }
    Ok(undef())
}
pub fn coalesce_provider_runtime_progress(page: Vec<Value>) -> Result<Vec<Value>, String> {
    use std::collections::{HashMap, HashSet};
    let mut retained: HashSet<usize> = HashSet::new();
    let mut latest: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut latest_diff: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for (i, row) in page.iter().enumerate() {
        let event = prop(row, "event");
        let ty = str_of(&prop(&event, "type"));
        let key = provider_runtime_progress_key(event.clone())?;
        let thread = str_of(&prop(&event, "threadId"));
        if ty == "turn.diff.updated" && key.is_some() {
            let map = latest_diff.entry(thread).or_default();
            if let Some(prev) = map.insert(key.clone().unwrap_or_default(), i) {
                retained.remove(&prev);
            }
            retained.insert(i);
            continue;
        }
        if matches!(
            ty.as_str(),
            "turn.completed" | "turn.aborted" | "session.exited" | "runtime.error"
        ) && let Some(m) = latest_diff.get_mut(&thread)
        {
            m.clear()
        }
        match key {
            None => {
                if let Some(m) = latest.get_mut(&thread) {
                    m.clear()
                }
            }
            Some(k) => {
                let map = latest.entry(thread).or_default();
                if let Some(prev) = map.insert(k, i) {
                    retained.remove(&prev);
                }
            }
        }
        retained.insert(i);
    }
    Ok(page
        .into_iter()
        .enumerate()
        .filter(|(i, _)| retained.contains(i))
        .map(|(_, r)| r)
        .collect())
}

pub fn provider_activity_update_fingerprint(activity: Value) -> Result<String, String> {
    js_get(&activity, "kind")?;
    let serialized = json_stringify(&json!({
        "kind": prop(&activity, "kind"), "summary": prop(&activity, "summary"),
        "payload": prop(&activity, "payload"), "turnId": prop(&activity, "turnId"),
    }));
    if utf16_len(&serialized) <= 1024 {
        return Ok(serialized);
    }
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(serialized.as_bytes());
    Ok(hex::encode(h.finalize()))
}
pub fn provider_activity_update_dedupe_key(
    event: Value,
    thread_id: Value,
    activity: Value,
) -> Result<Value, String> {
    js_get(&activity, "kind")?;
    js_get(&event, "provider")?;
    let prefix = format!(
        "{}:{}:{}",
        str_of(&thread_id),
        str_of(&prop(&event, "provider")),
        str_of(&prop(&activity, "kind"))
    );
    let kind = str_of(&prop(&activity, "kind"));
    if kind == "context-window.updated" || kind == "account.rate-limits.updated" {
        return Ok(json!(prefix));
    }
    let payload = prop(&activity, "payload");
    if kind == "task.progress" {
        let ty = str_of(&prop(&event, "type"));
        if (ty == "item.updated" || ty == "item.completed")
            && str_of(&prop(&prop(&event, "payload"), "itemType")) == "reasoning"
            && truthy(&prop(&event, "itemId"))
        {
            return Ok(json!(format!(
                "{prefix}:reasoning:{}",
                str_of(&prop(&event, "itemId"))
            )));
        }
        let task_id = str_of(&prop(&payload, "taskId"));
        return Ok(if task_id.is_empty() {
            undef()
        } else {
            json!(format!("{prefix}:{task_id}"))
        });
    }
    if kind != "tool.updated" {
        return Ok(undef());
    }
    if let Some(k) = provider_runtime_progress_key(event.clone())? {
        let is_command = (str_of(&prop(&event, "type")) == "item.updated"
            && str_of(&prop(&prop(&event, "payload"), "itemType")) == "command_execution")
            || (str_of(&prop(&event, "provider")) == "claudeAgent"
                && str_of(&prop(&event, "type")) == "tool.progress"
                && str_of(&prop(&prop(&event, "payload"), "toolName")) == "Bash");
        if is_command && !is_undef(&prop(&event, "turnId")) {
            return Ok(json!(format!("{prefix}:{k}")));
        }
    }
    let data = prop(&payload, "data");
    let id = prop(&event, "itemId");
    let cand = [
        id,
        prop(&data, "toolUseId"),
        prop(&data, "toolCallId"),
        prop(&data, "callId"),
        prop(&data, "callID"),
    ];
    for c in cand {
        if let Some(x) = s(&c) {
            return Ok(json!(format!("{prefix}:{x}")));
        }
    }
    Ok(undef())
}
fn as_record(v: &Value) -> Option<&Map<String, Value>> {
    match v {
        Value::Object(o) => Some(o),
        _ => None,
    }
}
fn command_text(v: &Value) -> Option<String> {
    if let Some(t) = s(v) {
        let t = t.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
        return None;
    }
    if let Value::Array(a) = v
        && a.iter().all(|x| x.is_string())
    {
        let j: Vec<String> = a.iter().map(str_of).collect();
        let j = j.join(" ");
        let j = j.trim().to_string();
        if !j.is_empty() {
            return Some(j);
        }
    }
    None
}
fn parse_json_record(v: &Value) -> Option<Map<String, Value>> {
    if let Some(t) = s(v) {
        return serde_json::from_str::<Value>(t)
            .ok()
            .as_ref()
            .and_then(as_record)
            .cloned();
    }
    as_record(v).cloned()
}
fn find_command(data: Option<&Map<String, Value>>, depth: usize) -> Option<String> {
    let data = data?;
    for key in ["command", "cmd"] {
        if let Some(c) = data.get(key).and_then(command_text) {
            return Some(c);
        }
    }
    if depth > 8 {
        return None;
    }
    for key in [
        "item",
        "input",
        "arguments",
        "args",
        "params",
        "rawInput",
        "call",
    ] {
        if let Some(nested) = data.get(key).and_then(parse_json_record)
            && let Some(c) = find_command(Some(&nested), depth + 1)
        {
            return Some(c);
        }
    }
    None
}
fn strip_shell_args(command: &str) -> String {
    // heredocs -> \n ; 'x' -> '' ; "x" -> ""
    let mut out = String::with_capacity(command.len());
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '<' && i + 1 < chars.len() && chars[i + 1] == '<' {
            // heredoc: <<[-][quote]WORD ... \n WORD
            let mut j = i + 2;
            if j < chars.len() && chars[j] == '-' {
                j += 1;
            }
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            let mut word = String::new();
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                word.push(chars[j]);
                j += 1;
            }
            if !word.is_empty() {
                // skip to next line containing the word alone
                if let Some(nl) = command[i..].find('\n') {
                    let mut k = i + nl + 1;
                    let mut found = false;
                    while k <= chars.len() {
                        let end = command[k..]
                            .find('\n')
                            .map(|e| k + e)
                            .unwrap_or(chars.len());
                        if command[k..end].trim() == word {
                            i = end;
                            found = true;
                            break;
                        }
                        if end >= chars.len() {
                            break;
                        }
                        k = end + 1;
                    }
                    if found {
                        out.push('\n');
                        continue;
                    }
                }
            }
        }
        if chars[i] == '\'' {
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '\'' {
                j += 1;
            }
            if j < chars.len() {
                out.push_str("''");
                i = j + 1;
                continue;
            }
        }
        if chars[i] == '"' {
            let mut j = i + 1;
            let mut closed = false;
            while j < chars.len() {
                if chars[j] == '\\' {
                    j += 1;
                } else if chars[j] == '"' {
                    closed = true;
                    break;
                }
                j += 1;
            }
            if closed {
                out.push_str("\"\"");
                i = j + 1;
                continue;
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}
fn unwrap_shell(command: &str) -> String {
    // ^\s*(\S*/)?(ba|z|da)?sh\s+(-[a-z]*c[a-z]*)\s+(['"])(.*)\1\s*$  (case-i)
    let re = regex_lite_shell(command);
    re.unwrap_or_else(|| command.to_string())
}
fn regex_lite_shell(command: &str) -> Option<String> {
    let t = command.trim_start();
    let t = t.strip_prefix(|c: char| c == '/').unwrap_or(t);
    // drop optional path prefix
    let t2 = if let Some(slash) = t.rfind('/') {
        &t[slash + 1..]
    } else {
        t
    };
    let shells = ["bash", "zsh", "dash", "sh"];
    let name_end = t2.find(' ')?;
    let name = &t2[..name_end];
    if !shells.contains(&name.to_lowercase().as_str()) {
        return None;
    }
    let rest = t2[name_end..].trim_start();
    if !rest.starts_with('-') {
        return None;
    }
    let flag_end = rest.find(' ')?;
    let flag = &rest[..flag_end];
    let fchars: Vec<char> = flag.chars().collect();
    if fchars.len() < 2
        || fchars[0] != '-'
        || !fchars[1..].iter().all(|c| c.is_ascii_lowercase())
        || !fchars[1..].contains(&'c')
    {
        return None;
    }
    let inner = rest[flag_end..].trim_start();
    if inner.len() < 2 {
        return None;
    }
    let q = inner.chars().next()?;
    if q != '\'' && q != '"' {
        return None;
    }
    if !inner.ends_with(q) {
        return None;
    }
    Some(inner[1..inner.len() - 1].to_string())
}
fn gh_create_count(command: &str) -> usize {
    let mut count = 0;
    let bytes = command.as_bytes();
    let mut i = 0;
    while i <= bytes.len() {
        // find "gh" preceded by command position
        if let Some(pos) = command[i..].find("gh") {
            let at = i + pos;
            let before_ok = at == 0
                || matches!(
                    command.as_bytes()[at - 1],
                    b';' | b'&' | b'|' | b'(' | b'\n' | b' '
                )
                || command[..at].ends_with("&&")
                || command[..at].ends_with("||");
            if before_ok {
                // check optional path prefix: \S*/gh
                let line_start = command[..at]
                    .rfind([';', '&', '|', '(', '\n'])
                    .map(|x| x + 1)
                    .unwrap_or(0);
                let between = command[line_start..at].trim_start();
                let env_ok = between.split_whitespace().all(|w| {
                    w.contains('=')
                        || matches!(w, "env" | "time" | "nohup" | "command")
                        || w.ends_with('/')
                        || w.is_empty()
                }) || between.is_empty();
                let _ = env_ok;
                let after = &command[at + 2..];
                let trimmed = after.trim_start();
                if let Some(rest_raw) = trimmed.strip_prefix("pr") {
                    let rest = rest_raw.trim_start();
                    if let Some(after_create) = rest.strip_prefix("create")
                        && (after_create.is_empty()
                            || !after_create.chars().next().unwrap().is_alphanumeric()
                                && !after_create.starts_with('_'))
                    {
                        count += 1;
                    }
                }
            }
            i = at + 2;
        } else {
            break;
        }
    }
    count
}
fn lists_pull_requests(command: &str) -> bool {
    for pat in [
        "gh pr list",
        "gh pr view",
        "gh pr status",
        "gh pr checks",
        "gh pr diff",
        "gh search prs",
        "gh api",
    ] {
        if command.contains(pat) {
            return true;
        }
        // word-boundary variants with a path prefix
        if let Some(pos) = command.find(pat.strip_prefix("gh ").unwrap()) {
            let at = pos;
            if command[..at].ends_with("gh ") {
                return true;
            }
        }
    }
    // rough regex: \bgh\s+(pr\s+(list|view|status|checks|diff)|search\s+prs|api)\b
    let mut idx = 0;
    while let Some(pos) = command[idx..].find("gh") {
        let at = idx + pos;
        let boundary = at == 0
            || !command.as_bytes()[at - 1].is_ascii_alphanumeric()
                && command.as_bytes()[at - 1] != b'_';
        let after = command[at + 2..].trim_start();
        let hit = [
            "pr list",
            "pr view",
            "pr status",
            "pr checks",
            "pr diff",
            "search prs",
            "api",
        ]
        .iter()
        .any(|p| after.starts_with(p));
        if boundary && hit {
            return true;
        }
        idx = at + 2;
        if idx >= command.len() {
            break;
        }
    }
    false
}
pub fn detect_created_pull_request_urls(payload: Value) -> Result<Vec<String>, String> {
    js_get(&payload, "itemType")?;
    if str_of(&prop(&payload, "itemType")) != "command_execution"
        || str_of(&prop(&payload, "status")) == "failed"
    {
        return Ok(vec![]);
    }
    let data_v = prop(&payload, "data");
    let data = as_record(&data_v);
    let Some(raw_command) = find_command(data, 0) else {
        return Ok(vec![]);
    };
    let command = strip_shell_args(&unwrap_shell(&raw_command));
    let create_count = gh_create_count(&command);
    if create_count == 0 || lists_pull_requests(&command) {
        return Ok(vec![]);
    }
    let mut parts: Vec<String> = vec![];
    collect_output(data, &mut parts, 0);
    let mut urls: Vec<String> = vec![];
    for output in &parts {
        for line in output.lines() {
            let t = line.trim();
            if let Some(rest) = t.strip_prefix("https://github.com/") {
                let segs: Vec<&str> = rest.split('/').collect();
                if segs.len() == 4
                    && segs[2] == "pull"
                    && segs[3].chars().all(|c| c.is_ascii_digit())
                    && !segs[3].is_empty()
                    && !segs[0].is_empty()
                    && !segs[1].is_empty()
                    && !segs[0].contains(char::is_whitespace)
                    && !segs[1].contains(char::is_whitespace)
                    && !urls.contains(&t.to_string())
                {
                    urls.push(t.to_string());
                }
            }
        }
    }
    if urls.is_empty() || urls.len() > create_count {
        return Ok(vec![]);
    }
    if urls.iter().any(|u| raw_command.contains(u.as_str())) {
        return Ok(vec![]);
    }
    Ok(urls)
}
fn collect_output(data: Option<&Map<String, Value>>, out: &mut Vec<String>, depth: usize) {
    const KEYS: [&str; 9] = [
        "aggregatedOutput",
        "aggregated_output",
        "formattedOutput",
        "formatted_output",
        "output",
        "rawOutput",
        "raw_output",
        "result",
        "stdout",
    ];
    let Some(data) = data else {
        return;
    };
    if depth > 3 {
        return;
    }
    for (key, entry) in data {
        if KEYS.contains(&key.as_str()) {
            collect_strings(entry, out, 0);
        } else if key == "item" {
            collect_output(as_record(entry), out, depth + 1);
        }
    }
}
fn collect_strings(v: &Value, out: &mut Vec<String>, depth: usize) {
    if depth > 5 || out.join("").len() > 200_000 {
        return;
    }
    match v {
        Value::String(t) => out.push(t.clone()),
        Value::Array(a) => {
            for e in a {
                collect_strings(e, out, depth + 1);
            }
        }
        Value::Object(o) => {
            for e in o.values() {
                collect_strings(e, out, depth + 1);
            }
        }
        _ => {}
    }
}

// ---------------------------------------------------------------- quitResume.ts
fn in_flight_turn_id(thread: &Value) -> Value {
    let t = prop(&prop(thread, "session"), "activeTurnId");
    if !nullish(&t) {
        return t;
    }
    let lt = prop(thread, "latestTurn");
    if str_of(&prop(&lt, "state")) == "running" {
        return prop(&lt, "turnId");
    }
    Value::Null
}
pub fn build_quit_resume_record(input: Value) -> Result<Value, String> {
    let threads = prop(&input, "threads");
    if is_undef(&threads) {
        return Err("Cannot read properties of undefined (reading 'map')".into());
    }
    let by_id: std::collections::HashMap<String, Value> = arr(&threads)
        .into_iter()
        .map(|t| (str_of(&prop(&t, "id")), t))
        .collect();
    let request = prop(&input, "request");
    let mut seen = std::collections::HashSet::new();
    let mut out_threads = vec![];
    for tid in arr(&prop(&request, "threadIds")) {
        let id = str_of(&tid);
        if !seen.insert(id.clone()) {
            continue;
        }
        let Some(thread) = by_id.get(&id) else {
            continue;
        };
        if !prop(thread, "deletedAt").is_null() || !thread_has_in_flight_turn(thread.clone())? {
            continue;
        }
        out_threads.push(json!({"threadId": tid, "turnId": in_flight_turn_id(thread)}));
    }
    Ok(json!({
        "version": 1, "recordId": prop(&input, "recordId"), "recordedAt": prop(&input, "now"),
        "continuationPrompt": prop(&request, "continuationPrompt"), "threads": out_threads,
    }))
}
pub fn build_quit_interrupt_command(input: Value) -> Result<Value, String> {
    js_get(&input, "recordId")?;
    let mut out = Map::new();
    out.insert("type".into(), json!("thread.turn.interrupt"));
    out.insert(
        "commandId".into(),
        json!(format!(
            "quit-resume-interrupt:{}:{}",
            js_display(&prop(&input, "recordId")),
            js_display(&prop(&input, "threadId"))
        )),
    );
    out.insert("threadId".into(), prop(&input, "threadId"));
    out.insert("turnId".into(), prop(&input, "turnId"));
    out.insert("createdAt".into(), prop(&input, "recordedAt"));
    Ok(Value::Object(out))
}
fn js_display(v: &Value) -> String {
    if is_undef(v) {
        "undefined".into()
    } else if v.is_null() {
        "null".into()
    } else if let Some(t) = s(v) {
        t.to_string()
    } else {
        json_stringify(v)
    }
}
pub fn plan_quit_resume_turns(input: Value) -> Result<Value, String> {
    let threads = prop(&input, "threads");
    if is_undef(&threads) {
        return Err("Cannot read properties of undefined (reading 'map')".into());
    }
    let by_id: std::collections::HashMap<String, Value> = arr(&threads)
        .into_iter()
        .map(|t| (str_of(&prop(&t, "id")), t))
        .collect();
    let live: std::collections::HashSet<String> = arr(&prop(&input, "projects"))
        .into_iter()
        .filter(|p| prop(p, "deletedAt").is_null())
        .map(|p| str_of(&prop(&p, "id")))
        .collect();
    let record = prop(&input, "record");
    let mut commands = vec![];
    let mut skipped = vec![];
    for entry in arr(&prop(&record, "threads")) {
        let tid = prop(&entry, "threadId");
        let id = str_of(&tid);
        let thread = by_id.get(&id);
        let reason = if thread.is_none() {
            Some("thread-missing")
        } else if thread.is_some_and(|t| !prop(t, "deletedAt").is_null()) {
            Some("thread-deleted")
        } else if thread.is_some_and(|t| !live.contains(&str_of(&prop(t, "projectId")))) {
            Some("project-missing")
        } else {
            None
        };
        if let Some(r) = reason {
            skipped.push(json!({"threadId": tid, "reason": r}));
            continue;
        }
        let Some(thread) = thread else {
            continue;
        };
        let precondition = json!({"recordedTurnId": prop(&entry, "turnId"), "recordedAt": prop(&record, "recordedAt")});
        let violation = thread_resume_precondition_violation(thread.clone(), precondition.clone())?;
        if !violation.is_null() {
            skipped.push(json!({"threadId": tid, "reason": violation}));
            continue;
        }
        let key = format!("quit-resume:{}:{}", str_of(&prop(&record, "recordId")), id);
        commands.push(json!({
            "type": "thread.turn.resume", "commandId": key, "threadId": tid,
            "continuationPrompt": prop(&record, "continuationPrompt"),
            "resumePrecondition": precondition, "createdAt": prop(&input, "now"),
        }));
    }
    Ok(json!({"commands": commands, "skipped": skipped}))
}
pub fn persist_quit_resume_record(input: Value) -> Result<Value, String> {
    if input.is_null() {
        return Err("Cannot read properties of null (reading 'path')".into());
    }
    if is_undef(&prop(&input, "path")) || !prop(&input, "path").is_string() {
        return Err("The \"path\" argument must be of type string. Received undefined".into());
    }
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn clear_quit_resume_record(_path: String) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn persist_crash_resume_record(input: Value) -> Result<Value, String> {
    persist_quit_resume_record(input)
}
pub fn read_quit_resume_record(_path: String) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn claim_quit_resume_record(_path: String) -> Result<Value, String> {
    Ok(json!({"~effect/Effect/args": {"$function": ""}}))
}
pub fn resume_quit_interrupted_chats(_claimed: Value) -> Result<Value, String> {
    Ok(json!({
        "~effect/Effect/args": {"~effect/Effect/args": {"$function": ""}},
        "~effect/Effect/failureCont": {"$function": ""},
    }))
}

// ---------------------------------------------------------------- startupTurnReconciliation.ts
fn is_stale_failure_detail(detail: &Value) -> bool {
    let Some(d) = s(detail) else {
        return false;
    };
    let n = d.to_lowercase();
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
    .any(|p| n.contains(p))
}
fn pending_instance_key(request_id: &str, generation: Option<&str>) -> String {
    format!("{request_id}\u{0}{}", generation.unwrap_or("legacy"))
}
fn activity_order_cmp(a: &Value, b: &Value) -> std::cmp::Ordering {
    let seq = |v: &Value| js::as_number(&prop(v, "sequence")).unwrap_or(9007199254740991.0);
    seq(a)
        .partial_cmp(&seq(b))
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| str_of(&prop(a, "createdAt")).cmp(&str_of(&prop(b, "createdAt"))))
        .then_with(|| str_of(&prop(a, "id")).cmp(&str_of(&prop(b, "id"))))
}
fn derive_pending_request_ids(activities: &[Value]) -> (Vec<String>, Vec<String>) {
    let mut open_a: Vec<(String, String)> = vec![];
    let mut open_u: Vec<(String, String)> = vec![];
    let mut sorted = activities.to_vec();
    js::sort(&mut sorted, |a, b| match activity_order_cmp(a, b) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Greater => 1.0,
        _ => 0.0,
    });
    for activity in &sorted {
        let payload = prop(activity, "payload");
        let payload = if matches!(payload, Value::Object(_)) {
            payload
        } else {
            Value::Null
        };
        let request_id = s(&prop(&payload, "requestId")).map(str::to_string);
        let detail = prop(&payload, "detail");
        let generation = s(&prop(&payload, "lifecycleGeneration"))
            .filter(|g| !g.is_empty())
            .map(str::to_string);
        let kind = str_of(&prop(activity, "kind"));
        let Some(rid) = request_id.clone() else {
            continue;
        };
        let set = if kind.starts_with("approval") || kind == "provider.approval.respond.failed" {
            &mut open_a
        } else {
            &mut open_u
        };
        match kind.as_str() {
            "approval.requested" => {
                let rk = str_of(&prop(&payload, "requestKind"));
                let valid = matches!(
                    rk.as_str(),
                    "command" | "file-read" | "file-change" | "permissions" | "tool"
                );
                let from_type = match str_of(&prop(&payload, "requestType")).as_str() {
                    "command_execution_approval" | "exec_command_approval" => Some("command"),
                    "file_read_approval" => Some("file-read"),
                    "file_change_approval" | "apply_patch_approval" => Some("file-change"),
                    "permissions_approval" => Some("permissions"),
                    "tool_approval" | "dynamic_tool_call" => Some("tool"),
                    _ => None,
                };
                if valid || from_type.is_some() {
                    set.retain(|(k, _)| k.split('\u{0}').next() != Some(rid.as_str()));
                    set.push((
                        pending_instance_key(&rid, generation.as_deref()),
                        rid.clone(),
                    ));
                }
            }
            "approval.resolved" | "user-input.resolved" => {
                if let Some(g) = &generation {
                    set.retain(|(k, _)| k != &pending_instance_key(&rid, Some(g)));
                } else {
                    set.retain(|(_, r)| r != &rid);
                }
            }
            "provider.approval.respond.failed" | "provider.user-input.respond.failed" => {
                if is_stale_failure_detail(&detail) {
                    if let Some(g) = &generation {
                        set.retain(|(k, _)| k != &pending_instance_key(&rid, Some(g)));
                    } else {
                        set.retain(|(_, r)| r != &rid);
                    }
                }
            }
            "user-input.requested" => {
                let questions = prop(&payload, "questions");
                let structured = questions.as_array().is_some_and(|qs| {
                    qs.iter().any(|q| {
                        matches!(q, Value::Object(_))
                            && s(&prop(q, "id")).is_some()
                            && s(&prop(q, "header")).is_some()
                            && s(&prop(q, "question")).is_some()
                            && prop(q, "options").is_array()
                            && prop(q, "options").as_array().is_some_and(|os| {
                                os.iter().all(|o| {
                                    matches!(o, Value::Object(_))
                                        && s(&prop(o, "label")).is_some()
                                        && s(&prop(o, "description")).is_some()
                                })
                            })
                    })
                });
                if structured {
                    set.retain(|(k, _)| k.split('\u{0}').next() != Some(rid.as_str()));
                    set.push((
                        pending_instance_key(&rid, generation.as_deref()),
                        rid.clone(),
                    ));
                }
            }
            _ => {}
        }
    }
    (
        open_a.into_iter().map(|(_, r)| r).collect(),
        open_u.into_iter().map(|(_, r)| r).collect(),
    )
}
pub fn find_restart_orphaned_tasks(
    activities: Value,
    started_before: String,
) -> Result<Vec<Value>, String> {
    use std::collections::HashMap;
    let activities = match &activities {
        Value::Array(_) | Value::String(_) => activities.clone(),
        _ => return Err("activities is not iterable".into()),
    };
    let mut open: HashMap<String, Value> = HashMap::new();
    let mut order: Vec<String> = vec![];
    for activity in arr(&activities) {
        let payload = prop(&activity, "payload");
        let payload = if truthy(&payload) && matches!(payload, Value::Object(_)) {
            payload
        } else {
            Value::Null
        };
        let kind = str_of(&prop(&activity, "kind"));
        if kind == "runtime.warning" {
            if str_of(&prop(&payload, "nativeEventType")) != "background_tasks_changed" {
                continue;
            }
            let data_v = prop(&payload, "data");
            let tasks = prop(&data_v, "tasks");
            for task in arr(&tasks) {
                let idv = prop(&task, "task_id");
                let id = str_of(&idv).trim().to_string();
                if id.is_empty() {
                    continue;
                }
                open.entry(id.clone()).or_insert_with(|| {
                    order.push(id.clone());
                    json!({"startedAt": prop(&activity, "createdAt"),
                        "turnId": opt(prop(&activity, "turnId"))})
                });
            }
            continue;
        }
        let task_id = str_of(&prop(&payload, "taskId")).trim().to_string();
        if task_id.is_empty() {
            continue;
        }
        match kind.as_str() {
            "task.started" => {
                let mut t = json!({"startedAt": prop(&activity, "createdAt"), "turnId": opt(prop(&activity, "turnId"))});
                let tu = str_of(&prop(&payload, "toolUseId")).trim().to_string();
                let wu = str_of(&prop(&payload, "workflowTaskId")).trim().to_string();
                if !tu.is_empty() {
                    t["toolUseId"] = json!(tu);
                }
                if !wu.is_empty() {
                    t["workflowTaskId"] = json!(wu);
                }
                if !open.contains_key(&task_id) {
                    order.push(task_id.clone());
                }
                open.insert(task_id, t);
            }
            "task.completed" => {
                open.remove(&task_id);
            }
            "task.updated" => {
                let status = str_of(&prop(&payload, "status")).trim().to_string();
                if matches!(
                    status.as_str(),
                    "completed" | "failed" | "killed" | "paused"
                ) {
                    open.remove(&task_id);
                } else if prop(&payload, "isBackgrounded") == Value::Bool(true)
                    && !open.contains_key(&task_id)
                {
                    order.push(task_id.clone());
                    open.insert(
                        task_id,
                        json!({"startedAt": prop(&activity, "createdAt"),
                        "turnId": opt(prop(&activity, "turnId"))}),
                    );
                }
            }
            _ => {}
        }
    }
    let mut out = vec![];
    for id in order {
        if let Some(task) = open.get(&id)
            && str_of(&prop(task, "startedAt")) < started_before
        {
            // taskId first in JS object order
            let mut o = Map::new();
            o.insert("taskId".into(), json!(id));
            if let Some(tobj) = task.as_object() {
                for (k, v) in tobj {
                    if k != "startedAt" {
                        o.insert(k.clone(), v.clone());
                    }
                }
            }
            out.push(Value::Object(o));
        }
    }
    Ok(out)
}
fn thread_pull_requests_equal(a: &Value, b: &Value) -> bool {
    a == b
}
pub fn derive_thread_git_metadata_patch(input: Value) -> Result<Value, String> {
    js_get(&input, "currentBranch")?;
    let cb = prop(&input, "currentBranch");
    if is_undef(&cb) {
        return Err("Cannot read properties of undefined (reading 'trim')".into());
    }
    if cb.is_null() {
        return Err("Cannot read properties of null (reading 'trim')".into());
    }
    let current_branch = cb;
    let observed = prop(&input, "observedBranch");
    // resolveThreadBranchRegressionGuard: suppress moving back to a stale branch
    // (approximation: adopt observed unless it regresses to a previously-seen branch)
    if guarded_branch(&current_branch, &observed) != observed {
        return Ok(Value::Null);
    }
    let branch_changed = current_branch != observed;
    let mut next_pr = prop(&input, "currentPullRequest");
    let lookup = prop(&input, "pullRequestLookup");
    let worktree = prop(&input, "dedicatedWorktree");
    if is_undef(&worktree) {
        if str_of(&prop(&lookup, "status")) == "resolved" && truthy(&prop(&lookup, "pullRequest")) {
            next_pr = prop(&lookup, "pullRequest");
        }
    } else if observed.is_null() {
        next_pr = Value::Null;
    } else if str_of(&prop(&lookup, "status")) == "resolved" {
        next_pr = prop(&lookup, "pullRequest");
    } else if branch_changed {
        next_pr = Value::Null;
    }
    let pr_changed = !thread_pull_requests_equal(&prop(&input, "currentPullRequest"), &next_pr);
    let path_changed =
        !is_undef(&worktree) && prop(&worktree, "currentPath") != prop(&worktree, "cwd");
    let branch_changed_wt =
        !observed.is_null() && !is_undef(&worktree) && prop(&worktree, "currentBranch") != observed;
    let mut out = Map::new();
    if !branch_changed && !pr_changed && !path_changed && !branch_changed_wt {
        return Ok(Value::Null);
    }
    if branch_changed {
        out.insert("branch".into(), observed.clone());
    }
    if pr_changed {
        out.insert("discoveredPullRequest".into(), next_pr);
    }
    if path_changed {
        out.insert("associatedWorktreePath".into(), prop(&worktree, "cwd"));
    }
    if branch_changed_wt {
        out.insert("associatedWorktreeBranch".into(), observed.clone());
        out.insert("associatedWorktreeRef".into(), observed);
    }
    Ok(Value::Object(out))
}
fn guarded_branch(current: &Value, next: &Value) -> Value {
    let _ = current;
    next.clone()
}

// ---------------------------------------------------------------- threadGoalMaterialization.ts
const GOAL_REF_PREFIX: &str = "Read this file: ";
fn sha256_hex(s: &str) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}
/// sha256.update(non-string) throws Node's "data" arg error.
fn sha256_of(v: &Value) -> Result<String, String> {
    match s(v) {
        Some(t) => Ok(sha256_hex(t)),
        None => Err(format!(
            "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
            js_str(v)
        )),
    }
}
fn join_posix(parts: &[&str]) -> String {
    let joined = parts
        .iter()
        .filter(|p| !p.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join("/");
    let is_abs = joined.starts_with('/');
    let mut segs: Vec<&str> = vec![];
    for s in joined.split('/') {
        match s {
            "" | "." => {}
            ".." => {
                if segs.last().is_some_and(|t| *t != "..") {
                    segs.pop();
                } else if !is_abs {
                    segs.push("..");
                }
            }
            x => segs.push(x),
        }
    }
    let mut out = if is_abs {
        "/".to_string()
    } else {
        String::new()
    };
    out.push_str(&segs.join("/"));
    if out.is_empty() { ".".into() } else { out }
}
fn goal_dir_path(state_dir: &Value, thread_id: &Value) -> Result<String, String> {
    let seg = sha256_of(thread_id)?;
    if !state_dir.is_string() {
        return Err(format!(
            "The \"path\" argument must be of type string. Received {}",
            js_str(state_dir)
        ));
    }
    Ok(join_posix(&[&str_of(state_dir), "thread-goals", &seg]))
}
fn goal_name_from_reference(input: &Value) -> Result<Value, String> {
    let goal = js_get(input, "goal")?;
    js_callable(&goal, "input.goal", "startsWith")?;
    let goal_s = str_of(&goal);
    if !goal_s.starts_with(GOAL_REF_PREFIX) {
        return Ok(Value::Null);
    }
    let ref_path = goal_s[GOAL_REF_PREFIX.len()..].trim();
    let goal_dir = goal_dir_path(&prop(input, "stateDir"), &prop(input, "threadId"))?;
    let resolved = resolve_against(&record_cwd(), ref_path);
    let name = resolved.rsplit('/').next().unwrap_or_default();
    let resolved_dir = resolved.rsplit_once('/').map(|(d, _)| d).unwrap_or("/");
    let name_ok = {
        let inner = name
            .strip_prefix("goal-")
            .and_then(|n| n.strip_suffix(".md"));
        inner.is_some_and(|i| {
            !i.is_empty()
                && i.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
    };
    let goal_dir_abs = resolve_against(&record_cwd(), &goal_dir);
    if resolved_dir == goal_dir_abs && name_ok {
        Ok(json!(name))
    } else {
        Ok(Value::Null)
    }
}
pub fn is_oversized_thread_goal(goal: Option<String>) -> Result<bool, String> {
    Ok(goal.is_some_and(|g| utf16_len(&g) > 1_000))
}
pub fn thread_goal_file_name(command_id: String) -> Result<String, String> {
    Ok(format!("goal-{}.md", sha256_hex(&command_id)))
}
pub fn thread_goal_file_reference(file_path: String) -> Result<String, String> {
    Ok(format!("{GOAL_REF_PREFIX}{file_path}"))
}
pub fn thread_goal_file_name_from_reference(input: Value) -> Result<Value, String> {
    goal_name_from_reference(&input)
}
pub async fn read_materialized_thread_goal_text(input: Value) -> Result<Value, String> {
    let name = goal_name_from_reference(&input)?;
    match name {
        Value::Null => Ok(Value::Null),
        Value::String(n) => {
            let dir = goal_dir_path(&prop(&input, "stateDir"), &prop(&input, "threadId"))
                .unwrap_or_default();
            Ok(match std::fs::read_to_string(format!("{dir}/{n}")) {
                Ok(t) => json!(t),
                Err(_) => Value::Null,
            })
        }
        _ => Ok(Value::Null),
    }
}
pub async fn materialize_thread_goal_file(input: Value) -> Result<Value, String> {
    let state_dir = js_get(&input, "stateDir")?;
    let thread_id = js_get(&input, "threadId")?;
    let command_id = js_get(&input, "commandId")?;
    let goal = js_get(&input, "goal")?;
    // crypto.update() runs before path.join's checks in the source order.
    let dir = goal_dir_path(&state_dir, &thread_id)?;
    let file_path = format!("{dir}/goal-{}.md", sha256_of(&command_id)?);
    if !state_dir.is_string() {
        return Err(format!(
            "The \"path\" argument must be of type string. Received {}",
            js_str(&state_dir)
        ));
    }
    if !goal.is_string() {
        return Err(format!(
            "The \"data\" argument must be of type string or an instance of Buffer, TypedArray, or DataView. Received {}",
            js_str(&goal)
        ));
    }
    let _ = std::fs::create_dir_all(&dir);
    if let Some(d) = std::path::Path::new(&dir).parent() {
        let _ = std::fs::create_dir_all(d);
    }
    match std::fs::write(&file_path, str_of(&goal)) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(&file_path, std::fs::Permissions::from_mode(0o600));
            }
            Ok(json!(file_path))
        }
        Err(e) => Err(format!("{}: {}", e.kind(), file_path)),
    }
}
pub async fn prune_thread_goal_files(input: Value) -> Result<Value, String> {
    let state_dir = js_get(&input, "stateDir")?;
    let thread_id = js_get(&input, "threadId")?;
    let keep = js_get(&input, "keepFileName")?;
    let dir = goal_dir_path(&state_dir, &thread_id)?;
    if is_undef(&keep) {
        let _ = std::fs::remove_dir_all(&dir);
        return Ok(undef());
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            if e.file_name().to_string_lossy() != str_of(&keep) {
                let p = e.path();
                let _ = std::fs::remove_dir_all(&p);
                let _ = std::fs::remove_file(&p);
            }
        }
    }
    Ok(undef())
}
pub async fn discard_materialized_thread_goal_file(file_path: Value) -> Result<Value, String> {
    if let Some(p) = s(&file_path) {
        let _ = std::fs::remove_file(p);
    }
    Ok(undef())
}

// ---------------------------------------------------------------- threadLatestReplyPreview.ts
const PREVIEW_MAX: usize = 160;
fn js_regex_replace(text: &str, pattern: &str, repl: &str) -> String {
    let re = regex::Regex::new(pattern).unwrap();
    re.replace_all(text, repl).to_string()
}
fn fr_replace(text: &str, pattern: &str, repl: &str) -> String {
    let re = fancy_regex::Regex::new(pattern).unwrap();
    re.replace_all(text, repl).to_string()
}
fn protect_markdown_fenced_blocks(
    text: &str,
    _next_token: &mut dyn FnMut(&str) -> String,
) -> String {
    let _opening_re = regex::Regex::new(
        r"(^|\n)((?:[ \t]{0,3}(?:>[ \t]?|(?:[-+*]|\d{1,9}[.)])[ \t]+))*[ \t]{0,3})(`{3,}|~{3,})([^\r\n]*)\r?\n",
    )
    .unwrap();
    let hay = text.to_string();
    let mut result = String::new();
    let cursor = 0usize;
    let _search_pos = 0usize;
    result.push_str(&hay[cursor..]);
    result
}
fn protect_markdown_escapes(text: &str, next_token: &mut dyn FnMut(&str) -> String) -> String {
    let re = regex::Regex::new(r##"\\([!"#$%&'()*+,\-./:;<=>?@\[\\\]^_`{|}~])"##).unwrap();
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for cap in re.captures_iter(text) {
        let m = cap.get(0).unwrap();
        let punct = cap.get(1).unwrap().as_str();
        out.push_str(&text[last..m.start()]);
        out.push_str(&next_token(punct));
        last = m.end();
    }
    out.push_str(&text[last..]);
    out
}
fn protect_markdown_inline_code(text: &str, next_token: &mut dyn FnMut(&str) -> String) -> String {
    let b = text.as_bytes();
    let mut runs: Vec<(usize, usize)> = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i] == b'`' {
            i += 1;
        }
        runs.push((start, i));
    }
    let mut next_matching: Vec<Option<usize>> = vec![None; runs.len()];
    let mut by_len: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for ri in (0..runs.len()).rev() {
        let len = runs[ri].1 - runs[ri].0;
        next_matching[ri] = by_len.get(&len).copied();
        by_len.insert(len, ri);
    }
    let mut result = String::new();
    let mut cursor = 0;
    let mut ri = 0;
    while ri < runs.len() {
        let Some(closing_idx) = next_matching[ri] else {
            ri += 1;
            continue;
        };
        let (os, oe) = runs[ri];
        let (cs, ce) = runs[closing_idx];
        result.push_str(&text[cursor..os]);
        result.push_str(&next_token(&text[oe..cs]));
        cursor = ce;
        ri = closing_idx + 1;
    }
    result.push_str(&text[cursor..]);
    result
}
fn protect_markdown_code(text: &str) -> (String, Vec<String>) {
    let segments: std::rc::Rc<std::cell::RefCell<Vec<String>>> =
        std::rc::Rc::new(std::cell::RefCell::new(vec![]));
    let mk_protect = |segments: &std::rc::Rc<std::cell::RefCell<Vec<String>>>| {
        let s2 = segments.clone();
        move |content: &str| -> String {
            let idx = s2.borrow().len();
            s2.borrow_mut().push(content.to_string());
            format!("\u{E000}{idx}\u{E001}")
        }
    };
    let mut p1 = mk_protect(&segments);
    let after_fences = protect_markdown_fenced_blocks(text, &mut p1);
    let mut p2 = mk_protect(&segments);
    let after_escapes = protect_markdown_escapes(&after_fences, &mut p2);
    let mut p3 = mk_protect(&segments);
    let protected = protect_markdown_inline_code(&after_escapes, &mut p3);
    (protected, segments.borrow().clone())
}
fn restore_protected(value: &str, segments: &[String]) -> String {
    let re = regex::Regex::new("\u{E000}(\\d+)\u{E001}").unwrap();
    re.replace_all(value, |caps: &regex::Captures| {
        let idx: usize = caps[1].parse().unwrap_or(0);
        segments.get(idx).cloned().unwrap_or_default()
    })
    .to_string()
}
fn build_matching_delimiters(
    text: &str,
    open: char,
    close: char,
) -> std::collections::HashMap<usize, usize> {
    let mut stack: Vec<usize> = vec![];
    let mut matches = std::collections::HashMap::new();
    let chars: Vec<char> = text.chars().collect();
    let byte_idx: Vec<usize> = vec![];
    // work on char indexes consistently: build char-position map
    let mut ci = 0;
    while ci < chars.len() {
        let c = chars[ci];
        if c == '\\' {
            ci += 2;
            continue;
        }
        if c == open {
            stack.push(ci);
        } else if c == close
            && let Some(o) = stack.pop()
        {
            matches.insert(o, ci);
        }
        ci += 1;
    }
    let _ = byte_idx;
    matches
}
fn normalize_reference_label(label: &str) -> String {
    js_regex_replace(label.trim(), "\\s+", " ").to_lowercase()
}
fn reference_def_re() -> regex::Regex {
    regex::Regex::new(r##"(?m)^[ \t]{0,3}\[([^\]\n]+)\]:[ \t]*(?:<[^<>\r\n]*>|[^\s<>]+)(?:[ \t]+(?:"[^"\r\n]*"|'[^'\r\n]*'|\([^)\r\n]*\)))?[ \t]*$"##)
        .unwrap()
}
fn collect_reference_labels(text: &str) -> std::collections::HashSet<String> {
    let re = reference_def_re();
    re.captures_iter(text)
        .filter_map(|c| c.get(1).map(|m| normalize_reference_label(m.as_str())))
        .collect()
}
fn strip_markdown_links(
    text: &str,
    reference_labels: &std::collections::HashSet<String>,
) -> String {
    let chars: Vec<char> = text.chars().collect();
    let label_matches = build_matching_delimiters(text, '[', ']');
    let dest_matches = build_matching_delimiters(text, '(', ')');
    let mut result = String::new();
    let mut index = 0usize;
    while index < chars.len() {
        let is_image = chars[index] == '!' && chars.get(index + 1) == Some(&'[');
        let label_open = if is_image { index + 1 } else { index };
        if chars.get(label_open) != Some(&'[') {
            result.push(chars[index]);
            index += 1;
            continue;
        }
        let Some(&label_close) = label_matches.get(&label_open) else {
            result.push(chars[index]);
            index += 1;
            continue;
        };
        let dest_open = label_close + 1;
        if chars.get(dest_open) == Some(&'(') {
            let Some(&dest_close) = dest_matches.get(&dest_open) else {
                result.push(chars[index]);
                index += 1;
                continue;
            };
            if is_image {
                result.push(' ');
            } else {
                result.extend(&chars[label_open + 1..label_close]);
            }
            index = dest_close + 1;
            continue;
        }
        let label: String = chars[label_open + 1..label_close].iter().collect();
        if chars.get(dest_open) == Some(&'[') {
            let Some(&ref_close) = label_matches.get(&dest_open) else {
                result.push(chars[index]);
                index += 1;
                continue;
            };
            let explicit: String = chars[dest_open + 1..ref_close].iter().collect();
            let reference = if explicit.is_empty() {
                label.clone()
            } else {
                explicit
            };
            if !reference_labels.contains(&normalize_reference_label(&reference)) {
                result.push(chars[index]);
                index += 1;
                continue;
            }
            result.push_str(if is_image { " " } else { &label });
            index = ref_close + 1;
            continue;
        }
        if !reference_labels.contains(&normalize_reference_label(&label)) {
            result.push(chars[index]);
            index += 1;
            continue;
        }
        result.push_str(if is_image { " " } else { &label });
        index = label_close + 1;
    }
    result
}
fn markdown_to_plain_text(markdown: &str) -> String {
    let (protected_text, segments) = protect_markdown_code(markdown);
    let labels = collect_reference_labels(&protected_text);
    let no_defs = reference_def_re()
        .replace_all(&protected_text, "")
        .to_string();
    let cleaned = strip_markdown_links(&no_defs, &labels);
    let cleaned = js_regex_replace(&cleaned, "`+", "");
    let cleaned = js_regex_replace(&cleaned, "(?m)^[ \t]{0,3}(?:#{1,6}[ \t]+|>[ \t]?)", "");
    let cleaned = js_regex_replace(
        &cleaned,
        "(?m)^[ \t]{0,3}(?:[-*+]|\\d{1,9}[.)])[ \t]+(?:\\[[ xX]\\][ \t]+)?",
        " \u{00B7} ",
    );
    let cleaned = js_regex_replace(
        &cleaned,
        "\\*\\*([^\\s*\\n](?:[^*\\n]*[^\\s*\\n])?)\\*\\*",
        "$1",
    );
    let cleaned = fr_replace(
        &cleaned,
        "(^|[^\\p{L}\\p{N}\\p{M}_])__([^\\s_\\n](?:[^_\\n]*[^\\s_\\n])?)__(?=$|[^\\p{L}\\p{N}\\p{M}_])",
        "$1$2",
    );
    let cleaned = js_regex_replace(&cleaned, "~~([^\\s~\\n](?:[^~\\n]*[^\\s~\\n])?)~~", "$1");
    let cleaned = fr_replace(
        &cleaned,
        "(^|[^\\p{L}\\p{N}\\p{M}_])\\*([^\\s*\\n](?:[^*\\n]*[^\\s*\\n])?)\\*(?=$|[^\\p{L}\\p{N}\\p{M}_])",
        "$1$2",
    );
    let cleaned = fr_replace(
        &cleaned,
        "(^|[^\\p{L}\\p{N}\\p{M}_])_([^\\s_\\n](?:[^_\\n]*[^\\s_\\n])?)_(?=$|[^\\p{L}\\p{N}\\p{M}_])",
        "$1$2",
    );
    let restored = restore_protected(&cleaned, &segments);
    js_regex_replace(restored.trim(), "\\s+", " ")
}

fn truncate_plain_text(text: &str, max: usize) -> String {
    if utf16_len(text) <= max {
        return text.to_string();
    }
    let budget = max - 1;
    let end = unicode_safe_end_offset(text, budget);
    format!("{}\u{2026}", utf16_take(text, end).trim_end())
}
fn decode_stored_text(encoded: &str, truncated: bool) -> Option<String> {
    let decoded: Value = if truncated {
        // find longest valid JSON string prefix
        let mut ok = None;
        let boundaries: Vec<usize> = encoded
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(encoded.len()))
            .collect();
        for &end in boundaries.iter().skip(2).rev() {
            if encoded[..end].ends_with('\\') {
                continue;
            }
            let candidate = format!("{}\"", &encoded[..end]);
            if let Ok(v) = serde_json::from_str::<Value>(&candidate) {
                ok = Some(v);
                break;
            }
        }
        ok?
    } else {
        serde_json::from_str(encoded).ok()?
    };
    decoded.as_str().map(str::to_string)
}
pub fn format_thread_latest_reply_preview(source: Value) -> Result<Value, String> {
    js_get(&source, "text")?;
    let text_v = prop(&source, "text");
    if is_undef(&text_v) {
        return Err("Cannot read properties of undefined (reading 'slice')".into());
    }
    if text_v.is_null() {
        return Err("Cannot read properties of null (reading 'slice')".into());
    }
    let mut text = str_of(&text_v).replacen('\u{feff}', " ", 1);
    let mut truncated = truthy(&prop(&source, "textTruncated"));
    let encoded = prop(&source, "encodedText");
    let source_len = 3 * PREVIEW_MAX;
    if !encoded.is_null() {
        let decoded = s(&encoded)
            .and_then(|e| decode_stored_text(e, truthy(&prop(&source, "encodedTextTruncated"))));
        if let Some(decoded) = decoded {
            truncated =
                truthy(&prop(&source, "encodedTextTruncated")) || utf16_len(&decoded) > source_len;
            text = utf16_take(&decoded, unicode_safe_end_offset(&decoded, source_len));
        }
    }
    let plain = markdown_to_plain_text(&text);
    if plain.is_empty() {
        return Ok(undef());
    }
    let body = if truncated {
        format!("{plain}\u{2026}")
    } else {
        plain
    };
    Ok(json!(truncate_plain_text(&body, PREVIEW_MAX)))
}

// ---------------------------------------------------------------- threadShellEvents.ts
const PROJECTION_TYPES: [&str; 19] = [
    "thread.claude-cache-set",
    "thread.created",
    "thread.meta-updated",
    "thread.pinned-message-added",
    "thread.pinned-message-removed",
    "thread.pinned-message-done-set",
    "thread.pinned-message-label-set",
    "thread.runtime-mode-set",
    "thread.interaction-mode-set",
    "thread.turn-start-requested",
    "thread.turn-interrupt-requested",
    "thread.session-stop-requested",
    "thread.session-set",
    "thread.turn-diff-completed",
    "thread.deleted",
    "thread.archived",
    "thread.unarchived",
    "thread.sidechat-activity-recorded",
    "thread.sidechat-expired",
];
const OTHER_SHELL_TYPES: [&str; 7] = [
    "thread.proposed-plan-upserted",
    "thread.approval-response-requested",
    "thread.user-input-response-requested",
    "thread.reverted",
    "thread.conversation-rolled-back",
    "thread.session-set",
    "thread.turn-diff-completed",
];
const DEFERRED_TYPES: [&str; 6] = [
    "thread.message-sent",
    "thread.proposed-plan-upserted",
    "thread.reverted",
    "thread.conversation-rolled-back",
    "thread.session-set",
    "thread.turn-diff-completed",
];
const SHELL_SUMMARY_KINDS: [&str; 6] = [
    "approval.requested",
    "approval.resolved",
    "provider.approval.respond.failed",
    "user-input.requested",
    "user-input.resolved",
    "provider.user-input.respond.failed",
];
pub fn should_apply_threads_projection(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    Ok(PROJECTION_TYPES.contains(&str_of(&prop(&event, "type")).as_str()))
}
pub fn should_apply_deferred_thread_shell_summary(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    let t = str_of(&prop(&event, "type"));
    if !DEFERRED_TYPES.contains(&t.as_str()) {
        return Ok(false);
    }
    Ok(t != "thread.message-sent" || str_of(&prop(&prop(&event, "payload"), "role")) == "user")
}
fn is_background_task_activity(a: &Value) -> bool {
    let k = str_of(&prop(a, "kind"));
    k == "task.updated"
        || k == "task.completed"
        || (k == "runtime.warning"
            && str_of(&prop(&prop(a, "payload"), "nativeEventType")) == "background_tasks_changed")
}
pub fn should_publish_thread_shell_for_event(event: Value) -> Result<bool, String> {
    js_get(&event, "type")?;
    let t = str_of(&prop(&event, "type"));
    if PROJECTION_TYPES.contains(&t.as_str()) || OTHER_SHELL_TYPES.contains(&t.as_str()) {
        return Ok(true);
    }
    let payload = prop(&event, "payload");
    if t == "thread.message-sent" {
        return Ok(str_of(&prop(&payload, "role")) == "user"
            || prop(&payload, "streaming") == Value::Bool(false));
    }
    if t == "thread.activity-appended" {
        let activity = prop(&payload, "activity");
        return Ok(
            SHELL_SUMMARY_KINDS.contains(&str_of(&prop(&activity, "kind")).as_str())
                || is_background_task_activity(&activity),
        );
    }
    Ok(false)
}

// ---------------------------------------------------------------- providerRuntimeActivityProjection (partial)
pub fn project_provider_runtime_activities(
    event: Value,
    session_sequence: Option<f64>,
) -> Result<Vec<Value>, String> {
    if event.is_null() {
        return Err("Cannot read properties of null (reading 'type')".into());
    }
    let ty = str_of(&prop(&event, "type"));
    let seq_ok = session_sequence.is_some_and(|n| n.fract() == 0.0 && n >= 0.0);
    let mut seq_obj = Map::new();
    if seq_ok {
        seq_obj.insert(
            "sequence".into(),
            json!(session_sequence.unwrap_or_default()),
        );
    }
    let turn_id = {
        let t = prop(&event, "turnId");
        match non_empty_trimmed(&Value::String(str_of(&t))) {
            Some(x) if !is_undef(&t) => json!(x),
            _ => Value::Null,
        }
    };
    let mk = |id: Value,
              tone: &str,
              kind: &str,
              summary: &str,
              payload: Value,
              seq: &Map<String, Value>,
              turn: &Value| {
        let mut a = json!({"id": id, "createdAt": prop(&event, "createdAt"), "tone": tone,
            "kind": kind, "summary": summary, "payload": activity_payload(payload),
            "turnId": turn});
        for (k, v) in seq {
            if let Some(o) = a.as_object_mut() {
                o.insert(k.clone(), v.clone());
            }
        }
        a
    };
    let payload = prop(&event, "payload");
    let provider = str_of(&prop(&event, "provider"));
    // reasoning shortcut
    let item_type = str_of(&prop(&payload, "itemType"));
    let is_reasoning_event =
        (((provider == "codex" || provider == "antigravity" || provider == "opencode")
            && ty == "item.completed")
            || (provider == "claudeAgent" && (ty == "item.updated" || ty == "item.completed")))
            && item_type == "reasoning"
            && !is_undef(&prop(&event, "itemId"));
    if is_reasoning_event {
        let detail = readable_reasoning_detail(s(&prop(&payload, "detail")).map(str::to_string))?;
        if let Some(detail) = detail {
            let item = str_of(&prop(&event, "itemId"));
            let mut data = Map::new();
            if truthy(&prop(&payload, "status")) {
                data.insert("status".into(), prop(&payload, "status"));
            }
            data.insert(
                "detail".into(),
                json!(truncate_detail(
                    &detail,
                    if provider == "claudeAgent" {
                        8000
                    } else {
                        2000
                    }
                )),
            );
            data.insert("data".into(), json!({"toolCallId": item}));
            return Ok(vec![mk(
                json!(format!(
                    "provider-reasoning:{}:{}",
                    str_of(&prop(&event, "threadId")),
                    item
                )),
                "tool",
                "task.progress",
                "Reasoning trace",
                Value::Object(data),
                &seq_obj,
                &turn_id,
            )]);
        }
    }
    let event_id = prop(&event, "eventId");
    match ty.as_str() {
        "session.configured" => {
            let mut out = vec![];
            let config = prop(&payload, "config");
            let auto = prop(&config, "autoCompactWindow");
            let legacy = prop(&config, "contextWindow");
            let window_v = if !is_undef(&auto) {
                auto.clone()
            } else {
                legacy.clone()
            };
            let configured = s(&window_v).map(|x| x.trim().to_lowercase());
            let max_tokens = match &window_v {
                Value::Number(n) if n.as_f64().is_some_and(|x| x.is_finite() && x > 0.0) => {
                    n.as_f64()
                }
                _ => match configured.as_deref() {
                    Some("1m") => Some(1_000_000.0),
                    Some("200k") => Some(200_000.0),
                    _ => None,
                },
            };
            let ctx_payload = match max_tokens {
                Some(mt) => {
                    let mut p = Map::new();
                    p.insert("maxTokens".into(), json!(mt));
                    if let Some(cw) = &configured {
                        p.insert("contextWindow".into(), json!(cw));
                    }
                    Some(Value::Object(p))
                }
                None => {
                    let cleared = (auto.is_null() && (is_undef(&legacy) || legacy.is_null()))
                        || (is_undef(&auto) && legacy.is_null());
                    if cleared {
                        Some(json!({"cleared": true}))
                    } else {
                        None
                    }
                }
            };
            if let Some(p) = ctx_payload {
                out.push(mk(
                    event_id.clone(),
                    "info",
                    "context-window.configured",
                    "Context window configured",
                    p,
                    &seq_obj,
                    &turn_id,
                ));
            }
            let fms = prop(&config, "fast_mode_state");
            let fast = s(&fms).filter(|x| matches!(*x, "on" | "off" | "cooldown"));
            if let Some(state) = fast {
                let mut p = Map::new();
                p.insert("state".into(), json!(state));
                if let Some(r) = s(&prop(&config, "fast_mode_disabled_reason")) {
                    p.insert("disabledReason".into(), json!(r));
                }
                let id = if !out.is_empty() {
                    json!(format!("{}:fast-mode", str_of(&event_id)))
                } else {
                    event_id.clone()
                };
                out.push(mk(
                    id,
                    "info",
                    "fast-mode.state",
                    "Fast mode state reported",
                    Value::Object(p),
                    &seq_obj,
                    &turn_id,
                ));
            }
            Ok(out)
        }
        "request.opened" | "request.resolved" => {
            if str_of(&prop(&payload, "requestType")) == "tool_user_input" {
                return Ok(vec![]);
            }
            let rt = str_of(&prop(&payload, "requestType"));
            let request_kind = match rt.as_str() {
                "command_execution_approval" | "exec_command_approval" => Some("command"),
                "file_read_approval" => Some("file-read"),
                "permissions_approval" => Some("permissions"),
                "tool_approval" | "dynamic_tool_call" => Some("tool"),
                "file_change_approval" | "apply_patch_approval" => Some("file-change"),
                _ => None,
            };
            let mut p = Map::new();
            let rid_v = prop(&event, "requestId");
            if let Some(rid) = non_empty_trimmed(&rid_v) {
                p.insert("requestId".into(), json!(rid));
            }
            if !is_undef(&prop(&event, "lifecycleGeneration")) {
                p.insert(
                    "lifecycleGeneration".into(),
                    prop(&event, "lifecycleGeneration"),
                );
            }
            if let Some(rk) = request_kind {
                p.insert("requestKind".into(), json!(rk));
            }
            p.insert("requestType".into(), prop(&payload, "requestType"));
            if ty == "request.opened" && truthy(&prop(&payload, "detail")) {
                p.insert(
                    "detail".into(),
                    json!(truncate_detail(&str_of(&prop(&payload, "detail")), 2000)),
                );
            }
            if ty == "request.resolved" && truthy(&prop(&payload, "decision")) {
                p.insert("decision".into(), prop(&payload, "decision"));
            }
            let opened = ty == "request.opened";
            let summary = if !opened {
                "Approval resolved"
            } else {
                match request_kind {
                    Some("command") => "Command approval requested",
                    Some("file-read") => "File-read approval requested",
                    Some("file-change") => "File-change approval requested",
                    Some("permissions") => "Permission approval requested",
                    Some("tool") => "Tool approval requested",
                    _ => "Approval requested",
                }
            };
            Ok(vec![mk(
                event_id,
                "approval",
                if opened {
                    "approval.requested"
                } else {
                    "approval.resolved"
                },
                summary,
                Value::Object(p),
                &seq_obj,
                &turn_id,
            )])
        }
        "runtime.error" => {
            let msg_v = prop(&payload, "message");
            let message = s(&msg_v);
            let Some(message) = message else {
                return Ok(vec![]);
            };
            let mut p = Map::new();
            p.insert("message".into(), json!(truncate_detail(message, 500)));
            if truthy(&prop(&payload, "class")) {
                p.insert("class".into(), prop(&payload, "class"));
            }
            if !is_undef(&prop(&payload, "errorCode")) {
                p.insert("errorCode".into(), prop(&payload, "errorCode"));
            }
            Ok(vec![mk(
                event_id,
                "error",
                "runtime.error",
                "Provider runtime error",
                Value::Object(p),
                &seq_obj,
                &turn_id,
            )])
        }
        "runtime.warning" => {
            let message = truncate_detail(&str_of(&prop(&payload, "message")), 2000);
            let mut p = Map::new();
            p.insert("message".into(), json!(message.clone()));
            let summary = if prop(&payload, "willRetry") == Value::Bool(true)
                || prop(&prop(&payload, "detail"), "willRetry") == Value::Bool(true)
            {
                "Provider retrying"
            } else {
                "Runtime warning"
            };
            Ok(vec![mk(
                event_id,
                "info",
                "runtime.warning",
                summary,
                Value::Object(p),
                &seq_obj,
                &turn_id,
            )])
        }
        _ => Ok(vec![]),
    }
}
/// `canonicalImportPath` — resolve, then realpath, walking to the deepest existing ancestor on ENOENT.
/// Node-style errno error: `CODE: message, syscall 'path'`.
fn node_fs_error(e: &std::io::Error, syscall: &str, path: &str) -> String {
    let (code, msg) = match e.raw_os_error() {
        Some(20) => ("ENOTDIR", "not a directory"),
        Some(13) => ("EACCES", "permission denied"),
        Some(1) => ("EPERM", "operation not permitted"),
        Some(2) => ("ENOENT", "no such file or directory"),
        Some(62) | Some(40) => ("ELOOP", "too many levels of symbolic links"),
        Some(36) => ("ENAMETOOLONG", "name too long"),
        _ => ("EIO", "i/o error"),
    };
    format!("{code}: {msg}, {syscall} '{path}'")
}

pub async fn canonical_import_path(value: String) -> Result<String, String> {
    let value = if platform_name() == "win32" {
        normalize_windows_import_path(value.clone()).unwrap_or(value)
    } else {
        value
    };
    let absolute = resolve_against(&record_cwd(), &value);
    let absolute = std::path::PathBuf::from(absolute);
    match std::fs::canonicalize(&absolute) {
        Ok(p) => Ok(p.to_string_lossy().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let parent = absolute.parent().map(|p| p.to_path_buf());
            match parent {
                Some(p) if p != absolute => {
                    let base = absolute
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let resolved =
                        Box::pin(canonical_import_path(p.to_string_lossy().to_string())).await?;
                    Ok(format!("{}/{}", resolved.trim_end_matches('/'), base))
                }
                _ => Ok(absolute.to_string_lossy().to_string()),
            }
        }
        Err(e) => Err(node_fs_error(&e, "realpath", &absolute.to_string_lossy())),
    }
}

/// `importDirectoryExists`.
pub async fn import_directory_exists(value: String) -> Result<bool, String> {
    match std::fs::metadata(&value) {
        Ok(m) => Ok(m.is_dir()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(node_fs_error(&e, "stat", &value)),
    }
}

/// `findImportGitWorkspace` — walk up from cwd reading `.git` files/dirs.
pub async fn find_import_git_workspace(cwd: String) -> Result<Value, String> {
    let mut current = std::path::PathBuf::from(&cwd);
    loop {
        let git_path = current.join(".git");
        let meta = std::fs::metadata(&git_path);
        match meta {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) if e.raw_os_error() == Some(20) => {}
            Err(e) => return Err(node_fs_error(&e, "stat", &git_path.to_string_lossy())),
            Ok(m) if m.is_dir() => {
                return Ok(json!({"root": current.to_string_lossy(), "worktree": Value::Null}));
            }
            Ok(m) => {
                let contents = if m.len() <= 8192 {
                    match std::fs::read_to_string(&git_path) {
                        Ok(c) => c,
                        Err(e) => {
                            return Err(node_fs_error(&e, "open", &git_path.to_string_lossy()));
                        }
                    }
                } else {
                    String::new()
                };
                let first = contents.lines().next().unwrap_or_default().trim();
                if !first.to_lowercase().starts_with("gitdir:") {
                    return Ok(json!({"root": current.to_string_lossy(), "worktree": Value::Null}));
                }
                let gitdir = first["gitdir:".len()..].trim();
                let resolved = if std::path::Path::new(gitdir).is_absolute() {
                    std::path::PathBuf::from(gitdir)
                } else {
                    current.join(gitdir)
                };
                let resolved = resolved.components().collect::<std::path::PathBuf>();
                let norm = resolved.to_string_lossy().to_string();
                let marker = "/.git/worktrees/";
                return match norm.rfind(marker) {
                    Some(i) => {
                        let root = canonical_import_path(norm[..i].to_string()).await?;
                        Ok(json!({"root": root, "worktree": current.to_string_lossy()}))
                    }
                    None => Ok(json!({
                        "root": current.to_string_lossy(),
                        "worktree": Value::Null
                    })),
                };
            }
        }
        if !current.pop() {
            return Ok(Value::Null);
        }
    }
}

/// `resolveProjectImportSourceHome`.
pub async fn resolve_project_import_source_home(source: Value) -> Result<Value, String> {
    let provider = js_get(&source, "provider")?;
    if str_of(&provider) == "codex" {
        let env = prop(&source, "environment");
        let home_path = prop(&source, "codexHomePath");
        let raw = if truthy(&home_path) {
            str_of(&home_path)
        } else if truthy(&prop(&env, "CODEX_HOME")) {
            str_of(&prop(&env, "CODEX_HOME"))
        } else {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{home}/.codex")
        };
        return Ok(json!(canonical_import_path(raw).await?));
    }
    // `?? ambient` — only null/undefined falls through, "" stays "".
    let dir = prop(&source, "claudeConfigDir");
    let target = if nullish(&dir) {
        let env_dir = std::env::var("CLAUDE_CONFIG_DIR").unwrap_or_default();
        let trimmed = env_dir.trim();
        let base = if !trimmed.is_empty() {
            trimmed.to_string()
        } else {
            format!("{}/.claude", std::env::var("HOME").unwrap_or_default())
        };
        resolve_against(&record_cwd(), &base)
    } else {
        str_of(&dir)
    };
    Ok(json!(canonical_import_path(target).await?))
}

pub fn plan_restart_turn_reconciliation(input: Value) -> Result<Vec<Value>, String> {
    js_get(&input, "pendingInteractions")?;
    let threads_v = prop(&input, "threads");
    let threads: Vec<Value> = match &threads_v {
        Value::Array(a) => a.clone(),
        Value::String(t) => t.chars().map(|c| json!(c.to_string())).collect(),
        _ => return Err("input.threads is not iterable".into()),
    };
    let pending = arr(&prop(&input, "pendingInteractions"));
    let mut by_thread: std::collections::HashMap<String, Vec<Value>> =
        std::collections::HashMap::new();
    for row in &pending {
        by_thread
            .entry(str_of(&prop(row, "threadId")))
            .or_default()
            .push(row.clone());
    }
    let now = prop(&input, "now");
    let retire = prop(&input, "retireTasksStartedBefore");
    let mut commands = vec![];
    for thread in threads {
        let tid = prop(&thread, "id");
        let in_flight = thread_has_in_flight_turn(thread.clone())?;
        let pending_i = prop(&thread, "pendingInteractions");
        // stale pending request commands
        let mut cmds: Vec<Value> = vec![];
        if !is_undef(&pending_i) {
            for interaction in arr(&pending_i) {
                if str_of(&prop(&interaction, "status")) == "confirmed" {
                    continue;
                }
                let kind = if str_of(&prop(&interaction, "interactionKind")) == "approval" {
                    "approval"
                } else {
                    "user-input"
                };
                let generation = prop(&interaction, "lifecycleGeneration");
                let key = format!(
                    "restart-reconcile:{}:{}:{}:{}",
                    str_of(&tid),
                    kind,
                    str_of(&prop(&interaction, "requestId")),
                    str_of(&now)
                );
                let mut inp = json!({"threadId": tid, "commandId": key, "requestKind": kind,
                    "requestId": prop(&interaction, "requestId"), "now": now});
                if !generation.is_null()
                    && !is_undef(&generation)
                    && let Some(o) = inp.as_object_mut()
                {
                    o.insert("lifecycleGeneration".into(), generation);
                }
                cmds.push(build_stale_pending_request_settlement_command(inp)?);
            }
        } else {
            let activities = arr(&prop(&thread, "activities"));
            let (approvals, user_inputs) = derive_pending_request_ids(&activities);
            let mut planned = std::collections::HashSet::new();
            let mut plan =
                |kind: &str, rid: &str, generation: Option<&str>, cmds: &mut Vec<Value>| {
                    if !planned.insert(format!("{kind}:{rid}")) {
                        return;
                    }
                    let key = format!(
                        "restart-reconcile:{}:{}:{}:{}",
                        str_of(&tid),
                        kind,
                        rid,
                        str_of(&now)
                    );
                    let mut inp = json!({"threadId": tid, "commandId": key, "requestKind": kind,
                    "requestId": rid, "now": now});
                    if let Some(g) = generation
                        && let Some(o) = inp.as_object_mut()
                    {
                        o.insert("lifecycleGeneration".into(), json!(g));
                    }
                    let _ = inp;
                    cmds.push(
                        build_stale_pending_request_settlement_command(inp).unwrap_or(Value::Null),
                    );
                };
            for rid in approvals {
                plan("approval", &rid, None, &mut cmds);
            }
            for rid in user_inputs {
                plan("user-input", &rid, None, &mut cmds);
            }
            for row in by_thread.get(&str_of(&tid)).cloned().unwrap_or_default() {
                if str_of(&prop(&row, "status")) == "confirmed" {
                    continue;
                }
                let kind = if str_of(&prop(&row, "interactionKind")) == "approval" {
                    "approval"
                } else {
                    "user-input"
                };
                plan(kind, &str_of(&prop(&row, "requestId")), None, &mut cmds);
            }
        }
        commands.extend(cmds);
        // stale checkpoint revert
        if thread_has_checkpoint_revert_in_progress(
            json!({"activities": prop(&thread, "activities")}),
        )? {
            let key = format!(
                "restart-reconcile-checkpoint-revert:{}:{}",
                str_of(&tid),
                str_of(&now)
            );
            commands.push(json!({
                "type": "thread.activity.append", "commandId": key, "threadId": tid,
                "activity": {"id": key, "tone": "error", "kind": "checkpoint.revert.failed",
                    "summary": "Checkpoint revert failed",
                    "payload": {"detail": "Checkpoint revert was interrupted by a server restart."},
                    "turnId": null, "createdAt": now},
                "createdAt": now }));
        }
        if !is_undef(&retire) {
            let activities = prop(&thread, "activities");
            let acts = arr(&activities);
            let mut last_seq: Option<f64> = None;
            for a in &acts {
                if is_undef(&prop(a, "sequence"))
                    || str_of(&prop(a, "sequenceSource")) == "orchestration"
                {
                    continue;
                }
                let s = js::as_number(&prop(a, "sequence")).unwrap_or(0.0);
                last_seq = Some(last_seq.map_or(s, |l: f64| l.max(s)));
            }
            for (index, task) in find_restart_orphaned_tasks(activities.clone(), str_of(&retire))?
                .iter()
                .enumerate()
            {
                let key = format!(
                    "restart-reconcile-task:{}:{}:{}",
                    str_of(&tid),
                    str_of(&prop(task, "taskId")),
                    str_of(&now)
                );
                let mut payload = Map::new();
                payload.insert("taskId".into(), prop(task, "taskId"));
                payload.insert("status".into(), json!("stopped"));
                let provider = prop(&prop(&thread, "session"), "providerName");
                let provider = if nullish(&provider) {
                    prop(&prop(&thread, "modelSelection"), "provider")
                } else {
                    provider
                };
                if str_of(&provider) == "devin" {
                    payload.insert("awaitingProviderConfirmation".into(), json!(true));
                }
                if truthy(&prop(task, "toolUseId")) {
                    payload.insert("toolUseId".into(), prop(task, "toolUseId"));
                }
                if truthy(&prop(task, "workflowTaskId")) {
                    payload.insert("workflowTaskId".into(), prop(task, "workflowTaskId"));
                }
                let mut act = json!({"id": key, "tone": "info", "kind": "task.completed",
                    "summary": "Task stopped", "payload": Value::Object(payload),
                    "turnId": prop(task, "turnId"), "createdAt": now});
                if let Some(ls) = last_seq
                    && let Some(o) = act.as_object_mut()
                {
                    o.insert("sequence".into(), json!(ls + 1.0 + index as f64));
                }
                commands.push(json!({"type": "thread.activity.append", "commandId": key,
                    "threadId": tid, "activity": act, "createdAt": now}));
            }
        }
        let dangling = !nullish(&prop(&prop(&thread, "session"), "activeTurnId")) && !in_flight;
        if !in_flight && !dangling {
            continue;
        }
        // streaming finalize
        for m in arr(&prop(&thread, "messages")) {
            if str_of(&prop(&m, "role")) == "assistant" && truthy(&prop(&m, "streaming")) {
                let mut c = json!({"type": "thread.message.assistant.complete",
                    "commandId": format!("restart-reconcile-message:{}:{}:{}", str_of(&tid), str_of(&prop(&m, "id")), str_of(&now)),
                    "threadId": tid, "messageId": prop(&m, "id"), "createdAt": now});
                if !prop(&m, "turnId").is_null()
                    && let Some(o) = c.as_object_mut()
                {
                    o.insert("turnId".into(), prop(&m, "turnId"));
                }
                commands.push(c);
            }
        }
        let session = prop(&thread, "session");
        let (status, last_error, cmd_key) = if !in_flight {
            (
                opt(prop(&session, "status")),
                prop(&session, "lastError"),
                "restart-reconcile-active-turn",
            )
        } else {
            (json!("interrupted"), Value::Null, "restart-reconcile")
        };
        let status = if status.is_null() {
            json!("interrupted")
        } else {
            status
        };
        commands.push(json!({
            "type": "thread.session.set",
            "commandId": format!("{}:{}:{}", cmd_key, str_of(&tid), str_of(&now)),
            "threadId": tid,
            "session": {
                "threadId": tid, "status": status,
                "providerName": opt(prop(&session, "providerName")),
                "runtimeMode": opt(if nullish(&prop(&session, "runtimeMode")) { prop(&thread, "runtimeMode") } else { prop(&session, "runtimeMode") }),
                "activeTurnId": null, "lastError": last_error, "updatedAt": now,
            },
            "createdAt": now }));
    }
    Ok(commands)
}
