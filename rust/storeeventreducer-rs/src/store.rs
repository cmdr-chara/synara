//! Rust port of the `storeEventReducer.ts` dependency subset.
//!
//! Provenance:
//! - `apps/web/src/storeNormalization.ts` — normalization + equality helpers consumed by the
//!   reducer (`normalizeChatMessage`, `normalizeActivities`, `normalizeThreadSession`,
//!   `normalizeModelSelection`, `normalizeProposedPlans`, `normalizeTurnDiffFiles`,
//!   `normalizeProject`, `normalizeSpace`, `resolveThreadSidebarMetadata`,
//!   `resolveCreateBranchFlowCompletedMerge`, `withOrchestrationEventSequence`,
//!   `createThreadActivityAccumulator`, the `*Equal`/`asActivityRecord` helpers,
//!   `MAX_THREAD_MESSAGES`).
//! - `apps/web/src/storeProjection.ts` — `applyThreadUpdate` machinery (`writeThreadState`,
//!   `commitThreadProjection`, `buildNormalizedSlice`, `buildSidebarThreadSummary`, derived
//!   thread-state signals), space/project upserts and removals, and the deletion-tombstone
//!   transitions (`removeDeletedProjectFromClientState`, `removeDeletedThreadFromClientState`).
//! - `apps/web/src/threadDerivation.ts` — `getThreadFromState`/`getThreadsFromState` via
//!   `collectByIds` (the WeakMap caches are identity-only optimizations: returning the same
//!   structural values is observationally identical).
//! - `apps/web/src/storePersistence.ts` — the remembered-project-UI-state module caches, as
//!   `thread_local!` maps (`getRememberedProjectUiState`/`rememberProjectState`/
//!   `resetStaleRememberedProjectState`/`forgetProjectState`/`projectCwdKey`).
//! - `apps/web/src/threadDetailResumeCursors.ts` — only `clearThreadDetailResumeCursor`, called
//!   from `clearThreadDetailSyncState`.
//!
//! Not ported (out of scope): `syncServerShellSnapshot`, `syncServerReadModel`, `applyShellEvent`,
//! `syncServerThreadDetail*`, `mergeReadModel*`, `normalizeThreadFromReadModel`,
//! `normalizeThreadShellSnapshot`, `mapProjects`, `mapSpaces`, `rebuildThreadShellRecords`,
//! `retainThreadScopedRecord`, `reuseThreadIdRegistry`, eviction/sync-flag writers and the
//! resume-cursor retain/reset paths — all exclusive to the snapshot-sync flows.
//!
//! Conventions: `serde_json::Value` everywhere; `crate::js::undefined()` is JS `undefined`.
//! `===`/`!==` between objects is approximated by structural equality (`same`): two structurally
//! equal values take the branch JS takes for the same reference, which preserves the "reuse the
//! previous object" fast paths without references. Scalars (including `NaN` number markers) go
//! through `strict_eq` so `NaN !== NaN` still holds.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use crate::jsv;

// ---- JS-semantics helpers ---------------------------------------------------------------------

fn undef() -> Value {
    crate::js::undefined()
}

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `{"$num": ...}` marker — a JS number JSON cannot express (NaN, ±Infinity, -0).
fn is_num_marker(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.contains_key("$num"))
}

/// `{"$utf16": [...]}` marker — a JS string with lone surrogates.
fn is_utf16_marker(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.contains_key("$utf16"))
}

fn is_string_like(v: &Value) -> bool {
    matches!(v, Value::String(_)) || is_utf16_marker(v)
}

fn js_typeof(v: &Value) -> &'static str {
    if is_undef(v) {
        "undefined"
    } else {
        match v {
            Value::Null => "object",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) | Value::Object(_) => "object",
        }
    }
}

fn to_num(v: &Value) -> f64 {
    jsv::to_number(v).unwrap_or(f64::NAN)
}

/// `String(v)` covering the transport markers (`$num`, `$utf16`, `$undefined`).
fn js_string(v: &Value) -> String {
    if let Value::Object(o) = v
        && o.len() == 1
    {
        if let Some(n) = o.get("$num").and_then(Value::as_str) {
            return n.to_string();
        }
        if o.contains_key("$utf16")
            && let Some(units) = crate::js::string_units(v)
        {
            return String::from_utf16_lossy(&units);
        }
    }
    jsv::js_to_string(v)
}

/// `a === b` (or `!==` negated). Scalars use `strict_eq` exactly; for objects/arrays JS would
/// compare references, which a structural port approximates with `deepEqualJson` — equal
/// contents take the same-reference branch, which reproduces the "keep previous" fast paths.
fn same(a: &Value, b: &Value) -> bool {
    if jsv::strict_eq(a, b) {
        return true;
    }
    // `NaN === NaN` is false in JS; undefined handled by strict_eq already.
    if is_num_marker(a) || is_num_marker(b) || is_undef(a) || is_undef(b) {
        return false;
    }
    matches!(
        (a, b),
        (Value::Array(_), Value::Array(_)) | (Value::Object(_), Value::Object(_))
    ) && deep_equal_json(a, b)
}

/// `a ?? b`.
fn coalesce<'a>(a: &'a Value, b: &'a Value) -> &'a Value {
    if jsv::is_nullish(a) { b } else { a }
}

/// `a > b` — codepoint compare when both operands are strings, else numeric (NaN → false).
fn js_gt(a: &Value, b: &Value) -> bool {
    if is_string_like(a) && is_string_like(b) {
        return js_string(a) > js_string(b);
    }
    to_num(a) > to_num(b)
}

fn js_ge(a: &Value, b: &Value) -> bool {
    if is_string_like(a) && is_string_like(b) {
        return js_string(a) >= js_string(b);
    }
    to_num(a) >= to_num(b)
}

fn js_lt(a: &Value, b: &Value) -> bool {
    if is_string_like(a) && is_string_like(b) {
        return js_string(a) < js_string(b);
    }
    to_num(a) < to_num(b)
}

fn js_le(a: &Value, b: &Value) -> bool {
    if is_string_like(a) && is_string_like(b) {
        return js_string(a) <= js_string(b);
    }
    to_num(a) <= to_num(b)
}

/// `a + b` — string concatenation when either side stringifies, else numeric addition.
fn js_add(a: &Value, b: &Value) -> Value {
    let str_like = |v: &Value| {
        !is_undef(v)
            && !is_num_marker(v)
            && matches!(v, Value::String(_) | Value::Array(_) | Value::Object(_))
    };
    if str_like(a) || str_like(b) {
        Value::String(format!("{}{}", js_string(a), js_string(b)))
    } else {
        crate::js::number(to_num(a) + to_num(b))
    }
}

/// `.length` as a number: arrays and strings report their length, objects an own `length`
/// property, anything else reads `undefined` (None).
fn js_length(v: &Value) -> Option<f64> {
    if let Value::Array(a) = v {
        return Some(a.len() as f64);
    }
    if let Some(units) = crate::js::string_units(v) {
        return Some(units.len() as f64);
    }
    if is_undef(v) {
        return None;
    }
    v.as_object()
        .and_then(|o| o.get("length"))
        .and_then(jsv::to_number)
}

/// `v.length` — member access: throws on null/undefined receivers.
fn member_length(v: &Value, expr: &str) -> Result<Option<f64>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if is_undef(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(js_length(v))
}

/// `v[key]` — index access: throws on null/undefined receivers, `undefined` otherwise when absent.
fn index_access<'a>(v: &'a Value, key: &str, expr: &str) -> Result<Option<&'a Value>, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if is_undef(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

/// `v?.[key]` — optional index: `undefined` on a nullish receiver, non-object, or missing key.
fn opt_index<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(key))
}

/// `v[i]` where `v` may be an array, string, plain object or primitive.
fn item_at(v: &Value, i: usize) -> Value {
    match v {
        Value::Array(a) => a.get(i).cloned().unwrap_or_else(undef),
        _ => {
            if let Some(units) = crate::js::string_units(v) {
                return units
                    .get(i)
                    .map(|u| crate::js::utf16_string(&[*u]))
                    .unwrap_or_else(undef);
            }
            opt_index(v, &i.to_string()).cloned().unwrap_or_else(undef)
        }
    }
}

/// `v?.[i]` — optional numeric index.
fn opt_elem(v: &Value, i: usize) -> Value {
    if jsv::is_nullish(v) {
        undef()
    } else {
        item_at(v, i)
    }
}

/// Property key for `record[id]` / `{[id]: v}` writes — JS `ToPropertyKey`.
fn record_key(v: &Value) -> String {
    if let Value::String(s) = v {
        return s.clone();
    }
    if let Some(units) = crate::js::string_units(v) {
        return String::from_utf16_lossy(&units);
    }
    js_string(v)
}

/// Key for `Map`/`Set` storage — SameValueZero is approximated by a type-tagged encoding:
/// objects collapse by structure (no reference identity exists here).
fn js_map_key(v: &Value) -> String {
    match v {
        Value::String(s) => {
            let mut k = String::with_capacity(s.len() + 2);
            k.push_str("s:");
            k.push_str(s);
            k
        }
        Value::Number(n) => {
            format!(
                "n:{}",
                crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN))
            )
        }
        Value::Bool(b) => format!("b:{b}"),
        Value::Null => "null".to_string(),
        _ => format!("o:{}", serde_json::to_string(v).unwrap_or_default()),
    }
}

/// `recv.method(...)` where the method only exists on arrays (`map`, `filter`, `find`, `some`,
/// `forEach`, `flatMap`, `includes`, `entries`): throws with the JSCore "not a function" text.
fn array_recv<'a>(
    v: &'a Value,
    access_expr: &str,
    call_expr: &str,
) -> Result<&'a Vec<Value>, String> {
    if v.is_null() {
        return Err(format!(
            "null is not an object (evaluating '{access_expr}')"
        ));
    }
    if is_undef(v) {
        return Err(format!(
            "undefined is not an object (evaluating '{access_expr}')"
        ));
    }
    if let Value::Array(a) = v {
        return Ok(a);
    }
    Err(format!(
        "{access_expr} is not a function. (In '{call_expr}', '{access_expr}' is undefined)"
    ))
}

/// `for..of`/`[...v]` iterable read — arrays iterate, strings iterate by code point, anything
/// else throws "X is not iterable".
fn spread_iterable(v: &Value, expr: &str) -> Result<Vec<Value>, String> {
    if let Value::Array(a) = v {
        return Ok(a.clone());
    }
    if let Some(units) = crate::js::string_units(v) {
        let s = String::from_utf16_lossy(&units);
        return Ok(s.chars().map(|c| Value::String(c.to_string())).collect());
    }
    Err(format!("{expr} is not iterable"))
}

/// `{...v}` — own enumerable string-keyed properties; strings spread by index; markers and
/// other primitives contribute nothing.
fn spread_any(v: &Value) -> Map<String, Value> {
    if is_undef(v) || is_num_marker(v) {
        return Map::new();
    }
    if let Some(units) = crate::js::string_units(v) {
        let mut m = Map::new();
        for (i, u) in units.iter().enumerate() {
            m.insert(i.to_string(), crate::js::utf16_string(&[*u]));
        }
        return m;
    }
    jsv::spread(v)
}

/// `Object.hasOwn`.
fn js_has_own(v: &Value, key: &str) -> bool {
    match v {
        Value::Object(o) => o.contains_key(key),
        Value::String(s) => {
            key == "length"
                || key
                    .parse::<usize>()
                    .ok()
                    .is_some_and(|i| i < crate::utf16::units(s).len())
        }
        _ => false,
    }
}

/// `Object.keys` (own enumerable) — objects give their keys, strings their indices.
#[allow(dead_code)] // reached through `records_shallow_equal`, kept for the reducer port
fn js_own_keys(v: &Value) -> Vec<String> {
    match v {
        Value::Object(o) => o.keys().cloned().collect(),
        _ => crate::js::string_units(v)
            .map(|u| (0..u.len()).map(|i| i.to_string()).collect())
            .unwrap_or_default(),
    }
}

/// `{...obj, key: value}` on a state/record value (spread of non-objects yields `{}`/indices).
fn record_set(record: &Value, key: &str, value: Value) -> Value {
    let mut m = spread_any(record);
    m.insert(key.to_string(), value);
    Value::Object(m)
}

/// `{[key]: _, ...rest}` — the record without `key`.
fn record_delete(record: &Value, key: &str) -> Value {
    let mut m = spread_any(record);
    m.shift_remove(key);
    Value::Object(m)
}

fn set_field(state: &Value, key: &str, value: Value) -> Value {
    record_set(state, key, value)
}

/// Insert when `v !== undefined` (`...(x !== undefined ? {k: x} : {})`).
fn set_if_defined(o: &mut Map<String, Value>, key: &str, v: &Value) {
    jsv::set_if_defined(o, key, v);
}

/// Insert when `v` is truthy (`...(x ? {k: x} : {})`).
fn set_if_truthy(o: &mut Map<String, Value>, key: &str, v: &Value) {
    if jsv::truthy(v) {
        o.insert(key.to_string(), v.clone());
    }
}

/// JS `encodeURIComponent`.
fn encode_uri_component(units: &[u16]) -> Result<String, String> {
    let Ok(s) = String::from_utf16(units) else {
        return Err("URI malformed".to_string());
    };
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let keep = b.is_ascii_alphanumeric()
            || matches!(
                *b,
                b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')'
            );
        if keep {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Ok(out)
}

// ---- remembered project UI state (`storePersistence.ts`) --------------------------------------

thread_local! {
    static PERSISTED_EXPANDED_PROJECT_CWDS: RefCell<HashSet<String>> =
        RefCell::new(HashSet::new());
    static PERSISTED_PROJECT_ORDER_BY_CWD: RefCell<Map<String, Value>> =
        RefCell::new(Map::new());
    static PERSISTED_PROJECT_NAMES_BY_CWD: RefCell<Map<String, Value>> =
        RefCell::new(Map::new());
    static PERSISTED_PROJECT_APPEARANCE_BY_CWD: RefCell<Map<String, Value>> =
        RefCell::new(Map::new());
    static PERSISTED_EXPANDED_PROJECT_CWDS_DEFINED: Cell<bool> = const { Cell::new(false) };
    static THREAD_DETAIL_RESUME_CURSORS: RefCell<HashMap<String, f64>> =
        RefCell::new(HashMap::new());
}

pub(crate) fn project_cwd_key(cwd: &Value) -> Result<String, String> {
    let out = crate::shared::normalize_workspace_root_for_comparison(cwd)?;
    Ok(js_string(&out))
}

/// `RememberedProjectUiState`: getter methods read the module maps like the TS getters do.
pub(crate) struct RememberedProjectUiState;

impl RememberedProjectUiState {
    pub(crate) fn expanded_project_count(&self) -> usize {
        PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow().len())
    }
    pub(crate) fn is_legacy_expansion_payload(&self) -> bool {
        PERSISTED_EXPANDED_PROJECT_CWDS_DEFINED.with(|d| d.get())
    }
    pub(crate) fn is_project_expanded(&self, cwd_key: &str) -> bool {
        PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow().contains(cwd_key))
    }
    pub(crate) fn project_order_count(&self) -> usize {
        PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| m.borrow().len())
    }
    pub(crate) fn project_order_index_for_cwd(&self, cwd_key: &str) -> Option<f64> {
        PERSISTED_PROJECT_ORDER_BY_CWD
            .with(|m| m.borrow().get(cwd_key).and_then(crate::js::as_number))
    }
    pub(crate) fn project_name_for_cwd(&self, cwd_key: &str) -> Option<String> {
        PERSISTED_PROJECT_NAMES_BY_CWD.with(|m| {
            m.borrow()
                .get(cwd_key)
                .and_then(Value::as_str)
                .map(str::to_string)
        })
    }
    pub(crate) fn project_appearance_for_cwd(&self, cwd_key: &str) -> Option<Value> {
        PERSISTED_PROJECT_APPEARANCE_BY_CWD.with(|m| m.borrow().get(cwd_key).cloned())
    }
}

pub(crate) fn get_remembered_project_ui_state() -> RememberedProjectUiState {
    RememberedProjectUiState
}

fn reset_remembered_project_state() {
    PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow_mut().clear());
    PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| m.borrow_mut().clear());
    PERSISTED_PROJECT_NAMES_BY_CWD.with(|m| m.borrow_mut().clear());
    PERSISTED_PROJECT_APPEARANCE_BY_CWD.with(|m| m.borrow_mut().clear());
    PERSISTED_EXPANDED_PROJECT_CWDS_DEFINED.with(|d| d.set(false));
}

pub(crate) fn forget_project_state(cwd: &Value) -> Result<(), String> {
    let cwd_key = project_cwd_key(cwd)?;
    PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow_mut().remove(&cwd_key));
    PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
    PERSISTED_PROJECT_NAMES_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
    PERSISTED_PROJECT_APPEARANCE_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
    Ok(())
}

/// `resetStaleRememberedProjectState(incomingCwdKeys)` — callers pass normalized cwd keys.
#[allow(dead_code)] // part of the spec'd API; the snapshot-sync callers live in the reducer port
pub(crate) fn reset_stale_remembered_project_state(incoming_cwd_keys: &HashSet<String>) {
    if incoming_cwd_keys.is_empty() {
        reset_remembered_project_state();
        return;
    }
    let remembered: HashSet<String> = PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| {
        let mut keys: HashSet<String> = m.borrow().keys().cloned().collect();
        PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| keys.extend(s.borrow().iter().cloned()));
        PERSISTED_PROJECT_NAMES_BY_CWD.with(|n| keys.extend(n.borrow().keys().cloned()));
        PERSISTED_PROJECT_APPEARANCE_BY_CWD.with(|a| keys.extend(a.borrow().keys().cloned()));
        keys
    });
    // An all-collapsed legacy payload has no identities to compare; keep it for the first
    // non-empty snapshot.
    if PERSISTED_EXPANDED_PROJECT_CWDS_DEFINED.with(|d| d.get()) && remembered.is_empty() {
        return;
    }
    let mut has_incoming = false;
    for cwd_key in remembered {
        if incoming_cwd_keys.contains(&cwd_key) {
            has_incoming = true;
        } else {
            PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow_mut().remove(&cwd_key));
            PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
            PERSISTED_PROJECT_NAMES_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
            PERSISTED_PROJECT_APPEARANCE_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
        }
    }
    if !has_incoming {
        reset_remembered_project_state();
        return;
    }
    // Close gaps left by removals so unknown projects sort after retained ones.
    let mut ordered: Vec<(String, f64)> = PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| {
        m.borrow()
            .iter()
            .map(|(k, v)| (k.clone(), crate::js::as_number(v).unwrap_or(f64::NAN)))
            .collect()
    });
    crate::js::sort(&mut ordered, |a, b| a.1 - b.1);
    for (index, (cwd_key, _)) in ordered.iter().enumerate() {
        PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| {
            m.borrow_mut()
                .insert(cwd_key.clone(), crate::js::number(index as f64))
        });
    }
}

#[allow(dead_code)] // part of the spec'd API; the snapshot-sync callers live in the reducer port
pub(crate) fn remember_project_state(projects: &Value) -> Result<(), String> {
    let items = array_recv(projects, "projects.entries", "projects.entries()")?;
    for (index, project) in items.iter().enumerate() {
        let cwd = jsv::member(project, "cwd", "project.cwd")?
            .cloned()
            .unwrap_or_else(undef);
        let cwd_key = project_cwd_key(&cwd)?;
        if jsv::truthy(&jsv::opt(project, "expanded")) {
            PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow_mut().insert(cwd_key.clone()));
        } else {
            PERSISTED_EXPANDED_PROJECT_CWDS.with(|s| s.borrow_mut().remove(&cwd_key));
        }
        PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| {
            m.borrow_mut()
                .insert(cwd_key.clone(), crate::js::number(index as f64))
        });
        // `project.localName?.trim() ?? ""`
        let local_name = jsv::opt(project, "localName");
        let trimmed = if jsv::is_nullish(&local_name) {
            String::new()
        } else if let Some(s) = local_name.as_str() {
            s.trim().to_string()
        } else {
            return Err(
                "project.localName.trim is not a function. (In 'project.localName.trim()', \
                 'project.localName.trim' is undefined)"
                    .to_string(),
            );
        };
        if !trimmed.is_empty() {
            PERSISTED_PROJECT_NAMES_BY_CWD.with(|m| {
                m.borrow_mut()
                    .insert(cwd_key.clone(), Value::String(trimmed))
            });
        } else {
            PERSISTED_PROJECT_NAMES_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
        }
        let appearance = jsv::opt(project, "appearance");
        if jsv::truthy(&appearance) {
            PERSISTED_PROJECT_APPEARANCE_BY_CWD
                .with(|m| m.borrow_mut().insert(cwd_key.clone(), appearance));
        } else {
            PERSISTED_PROJECT_APPEARANCE_BY_CWD.with(|m| m.borrow_mut().shift_remove(&cwd_key));
        }
    }
    if PERSISTED_PROJECT_ORDER_BY_CWD.with(|m| !m.borrow().is_empty()) {
        PERSISTED_EXPANDED_PROJECT_CWDS_DEFINED.with(|d| d.set(false));
    }
    Ok(())
}

/// `clearThreadDetailResumeCursor(threadId)`.
fn clear_thread_detail_resume_cursor(thread_id: &Value) {
    let key = js_map_key(thread_id);
    THREAD_DETAIL_RESUME_CURSORS.with(|m| m.borrow_mut().remove(&key));
}

// ---- equality helpers (`storeNormalization.ts`) ------------------------------------------------

pub(crate) const MAX_THREAD_MESSAGES: usize = 2_000;
const MAX_THREAD_ACTIVITIES: usize = 2_000;
const PENDING_INTERACTION_REQUEST_KINDS: &[&str] = &["approval.requested", "user-input.requested"];
const PENDING_INTERACTION_ACTIVITY_KINDS: &[&str] = &[
    "approval.requested",
    "user-input.requested",
    "approval.resolved",
    "user-input.resolved",
    "provider.approval.respond.failed",
    "provider.user-input.respond.failed",
];

/// `arraysShallowEqual(left, right)` — `left` may be undefined or any falsy value.
pub(crate) fn arrays_shallow_equal(left: &Value, right: &Value) -> bool {
    if !jsv::truthy(left) {
        return false;
    }
    let (ll, rl) = (js_length(left), js_length(right));
    if ll != rl {
        return false;
    }
    let Some(n) = ll else {
        // `.length` read `undefined` on both sides: the element loop never runs.
        return true;
    };
    for i in 0..(n as usize) {
        if !same(&opt_elem(left, i), &opt_elem(right, i)) {
            return false;
        }
    }
    true
}

/// `textSegmentArraysEqual` — structural equality for a message's text-segment list.
/// (A nullish element's `.sequence` read would throw in JS; here it just compares `undefined`,
/// which still differs from any defined segment field and reports inequality.)
pub(crate) fn text_segment_arrays_equal(left: &Value, right: &Value) -> bool {
    if same(left, right) {
        return true;
    }
    if !jsv::truthy(left) || !jsv::truthy(right) {
        return false;
    }
    let (ll, rl) = (js_length(left), js_length(right));
    if ll != rl {
        return false;
    }
    let Some(n) = ll else { return true };
    for i in 0..(n as usize) {
        let l = opt_elem(left, i);
        let r = opt_elem(right, i);
        for key in ["sequence", "startedAt", "endedAt", "text"] {
            if !same(&jsv::opt(&l, key), &jsv::opt(&r, key)) {
                return false;
            }
        }
    }
    true
}

/// `providerReferenceArraysEqual` — compares `{name, path}` entries with `?.` reads.
pub(crate) fn provider_reference_arrays_equal(left: &Value, right: &Value) -> bool {
    if same(left, right) {
        return true;
    }
    if !jsv::truthy(left) || !jsv::truthy(right) {
        return false;
    }
    let (ll, rl) = (js_length(left), js_length(right));
    if ll != rl {
        return false;
    }
    let Some(n) = ll else { return true };
    for i in 0..(n as usize) {
        let l = opt_elem(left, i);
        let r = opt_elem(right, i);
        if !same(&jsv::opt(&l, "name"), &jsv::opt(&r, "name"))
            || !same(&jsv::opt(&l, "path"), &jsv::opt(&r, "path"))
        {
            return false;
        }
    }
    true
}

/// `recordsShallowEqual(left, right)`.
#[allow(dead_code)] // spec'd export; only the (excluded) snapshot-sync paths call it in TS
pub(crate) fn records_shallow_equal(left: &Value, right: &Value) -> bool {
    let left_keys = js_own_keys(left);
    let right_keys = js_own_keys(right);
    if left_keys.len() != right_keys.len() {
        return false;
    }
    for key in &left_keys {
        if !js_has_own(right, key) {
            return false;
        }
        if !same(&opt_index_v(left, key), &opt_index_v(right, key)) {
            return false;
        }
    }
    true
}

fn opt_index_v(v: &Value, key: &str) -> Value {
    opt_index(v, key).cloned().unwrap_or_else(undef)
}

/// `deepEqualJson`.
pub(crate) fn deep_equal_json(left: &Value, right: &Value) -> bool {
    if jsv::strict_eq(left, right) {
        return true;
    }
    if jsv::is_nullish(left) || jsv::is_nullish(right) {
        return false;
    }
    if js_typeof(left) != js_typeof(right) {
        return false;
    }
    if left.is_array() || right.is_array() {
        let (Some(l), Some(r)) = (left.as_array(), right.as_array()) else {
            return false;
        };
        if l.len() != r.len() {
            return false;
        }
        return l.iter().zip(r.iter()).all(|(a, b)| deep_equal_json(a, b));
    }
    if js_typeof(left) != "object" {
        return false;
    }
    let (Some(l), Some(r)) = (left.as_object(), right.as_object()) else {
        return false;
    };
    if l.len() != r.len() {
        return false;
    }
    l.iter()
        .all(|(k, v)| r.get(k).is_some_and(|rv| deep_equal_json(v, rv)))
}

/// `asActivityRecord` — `value && typeof value === "object" ? value : null`.
/// `Option<&Map>` can only hold real objects; callers that need JS's array-including
/// "is a record" truthiness should use `as_recordish`.
pub(crate) fn as_activity_record(v: &Value) -> Option<&Map<String, Value>> {
    if is_undef(v) || is_num_marker(v) || is_utf16_marker(v) {
        return None;
    }
    v.as_object()
}

/// JS `typeof v === "object"` truthiness (arrays included; number/utf16/undefined markers are
/// strings/numbers/undefined, never objects).
fn as_recordish(v: &Value) -> bool {
    jsv::truthy(v)
        && !is_undef(v)
        && !is_num_marker(v)
        && !is_utf16_marker(v)
        && matches!(v, Value::Object(_) | Value::Array(_))
}

/// `withOrchestrationEventSequence`: only an activity *missing* `sequence` gets the
/// orchestration envelope sequence, tagged by `sequenceSource`.
pub(crate) fn with_orchestration_event_sequence(
    activity: &Value,
    sequence: &Value,
) -> Result<Value, String> {
    let seq = jsv::member(activity, "sequence", "activity.sequence")?;
    if seq.is_some_and(|s| !is_undef(s)) {
        return Ok(activity.clone());
    }
    let mut m = spread_any(activity);
    m.insert("sequence".to_string(), sequence.clone());
    m.insert(
        "sequenceSource".to_string(),
        Value::String("orchestration".to_string()),
    );
    Ok(Value::Object(m))
}

/// `attachmentPreviewRoutePath(attachmentId)` = `/attachments/${encodeURIComponent(id)}`.
fn attachment_preview_route_path(attachment_id: &Value) -> Result<String, String> {
    let units = crate::utf16::units(&js_string(attachment_id));
    Ok(format!("/attachments/{}", encode_uri_component(&units)?))
}

/// `toAttachmentPreviewUrl(rawPath)` — the recorded env has no `window` and no remote execution
/// context, so `resolveWsHttpUrl` returns the raw path unchanged.
fn to_attachment_preview_url(raw_path: String) -> String {
    raw_path
}

/// `normalizeChatAttachments(incoming, previous)`.
fn normalize_chat_attachments(incoming: &Value, previous: &Value) -> Result<Value, String> {
    if !jsv::truthy(incoming) || js_length(incoming) == Some(0.0) {
        return Ok(undef());
    }
    let items = array_recv(
        incoming,
        "incoming.map",
        "incoming.map((attachment) => ...)",
    )?;
    let mut previous_by_id: HashMap<String, &Value> = HashMap::new();
    if !jsv::is_nullish(previous) {
        for a in array_recv(
            previous,
            "previous.map",
            "previous.map((attachment) => [attachment.id, attachment])",
        )? {
            previous_by_id.insert(js_map_key(&jsv::opt(a, "id")), a);
        }
    }
    let mut next: Vec<Value> = Vec::with_capacity(items.len());
    for attachment in items {
        let ty = jsv::member(attachment, "type", "attachment.type")?
            .cloned()
            .unwrap_or_else(undef);
        let mut att = Map::new();
        if jsv::strict_eq(&ty, &Value::String("assistant-selection".to_string())) {
            att.insert("type".to_string(), ty.clone());
            att.insert(
                "id".to_string(),
                jsv::member(attachment, "id", "attachment.id")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "assistantMessageId".to_string(),
                jsv::member(
                    attachment,
                    "assistantMessageId",
                    "attachment.assistantMessageId",
                )?
                .cloned()
                .unwrap_or_else(undef),
            );
            att.insert(
                "text".to_string(),
                jsv::member(attachment, "text", "attachment.text")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
        } else if jsv::strict_eq(&ty, &Value::String("file".to_string())) {
            att.insert("type".to_string(), ty.clone());
            att.insert(
                "id".to_string(),
                jsv::member(attachment, "id", "attachment.id")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "name".to_string(),
                jsv::member(attachment, "name", "attachment.name")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "mimeType".to_string(),
                jsv::member(attachment, "mimeType", "attachment.mimeType")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "sizeBytes".to_string(),
                jsv::member(attachment, "sizeBytes", "attachment.sizeBytes")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
        } else {
            att.insert("type".to_string(), Value::String("image".to_string()));
            let source_path = jsv::member(attachment, "sourcePath", "attachment.sourcePath")?
                .cloned()
                .unwrap_or_else(undef);
            set_if_defined(&mut att, "sourcePath", &source_path);
            let id = jsv::member(attachment, "id", "attachment.id")?
                .cloned()
                .unwrap_or_else(undef);
            att.insert("id".to_string(), id.clone());
            att.insert(
                "name".to_string(),
                jsv::member(attachment, "name", "attachment.name")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "mimeType".to_string(),
                jsv::member(attachment, "mimeType", "attachment.mimeType")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "sizeBytes".to_string(),
                jsv::member(attachment, "sizeBytes", "attachment.sizeBytes")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            att.insert(
                "previewUrl".to_string(),
                Value::String(to_attachment_preview_url(attachment_preview_route_path(
                    &id,
                )?)),
            );
        }
        let att_v = Value::Object(att);
        let id_key = js_map_key(&jsv::opt(attachment, "id"));
        if let Some(existing) = previous_by_id.get(&id_key)
            && chat_attachment_matches(existing, &att_v)?
        {
            next.push((*existing).clone());
            continue;
        }
        next.push(att_v);
    }
    let next_v = Value::Array(next);
    Ok(if arrays_shallow_equal(previous, &next_v) {
        previous.clone()
    } else {
        next_v
    })
}

/// The per-type "keep the stored attachment" comparisons inside `normalizeChatAttachments`.
/// `existing.type` is a member read — a nullish stored attachment throws in JS.
fn chat_attachment_matches(existing: &Value, next: &Value) -> Result<bool, String> {
    let e_type = jsv::member(existing, "type", "existing.type")?
        .cloned()
        .unwrap_or_else(undef);
    let e = |k: &str| jsv::opt(existing, k);
    let n = |k: &str| jsv::opt(next, k);
    if jsv::strict_eq(&e_type, &Value::String("assistant-selection".to_string()))
        && jsv::strict_eq(
            &n("type"),
            &Value::String("assistant-selection".to_string()),
        )
        && same(&e("assistantMessageId"), &n("assistantMessageId"))
        && same(&e("text"), &n("text"))
    {
        return Ok(true);
    }
    if jsv::strict_eq(&e_type, &Value::String("image".to_string()))
        && jsv::strict_eq(&n("type"), &Value::String("image".to_string()))
        && same(&e("name"), &n("name"))
        && same(&e("mimeType"), &n("mimeType"))
        && same(&e("sizeBytes"), &n("sizeBytes"))
        && same(&e("sourcePath"), &n("sourcePath"))
        && same(&e("previewUrl"), &n("previewUrl"))
    {
        return Ok(true);
    }
    if jsv::strict_eq(&e_type, &Value::String("file".to_string()))
        && jsv::strict_eq(&n("type"), &Value::String("file".to_string()))
        && same(&e("name"), &n("name"))
        && same(&e("mimeType"), &n("mimeType"))
        && same(&e("sizeBytes"), &n("sizeBytes"))
    {
        return Ok(true);
    }
    Ok(false)
}

/// `normalizeChatMessage(incoming, previous)`.
pub(crate) fn normalize_chat_message(
    incoming: &Value,
    previous: Option<&Value>,
) -> Result<Value, String> {
    let undef_v = undef();
    let prev = previous.unwrap_or(&undef_v);
    let inc = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(incoming, k, &format!("incoming.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let attachments =
        normalize_chat_attachments(&inc("attachments")?, &jsv::opt(prev, "attachments"))?;
    // Partial live updates omit skills/mentions; keep the previous arrays.
    let inc_skills = inc("skills")?;
    let inc_mentions = inc("mentions")?;
    let skills = if jsv::truthy(&inc_skills)
        && member_length(&inc_skills, "incoming.skills.length")?.unwrap_or(0.0) > 0.0
    {
        inc_skills
    } else {
        coalesce(&jsv::opt(prev, "skills"), &Value::Array(vec![])).clone()
    };
    let mentions = if jsv::truthy(&inc_mentions)
        && member_length(&inc_mentions, "incoming.mentions.length")?.unwrap_or(0.0) > 0.0
    {
        inc_mentions
    } else {
        coalesce(&jsv::opt(prev, "mentions"), &Value::Array(vec![])).clone()
    };
    let previous_skills = coalesce(&jsv::opt(prev, "skills"), &Value::Array(vec![])).clone();
    let previous_mentions = coalesce(&jsv::opt(prev, "mentions"), &Value::Array(vec![])).clone();
    let streaming = inc("streaming")?;
    let completed_at = if jsv::truthy(&streaming) {
        undef()
    } else {
        inc("updatedAt")?
    };
    let async_user_input = crate::shared::merge_async_user_input(
        &jsv::opt(prev, "asyncUserInput"),
        &inc("asyncUserInput")?,
    )?;

    if jsv::truthy(prev)
        && same(&jsv::opt(prev, "role"), &inc("role")?)
        && same(&jsv::opt(prev, "text"), &inc("text")?)
        && same(&jsv::opt(prev, "asyncUserInput"), &async_user_input)
        && same(&jsv::opt(prev, "dispatchMode"), &inc("dispatchMode")?)
        && same(&jsv::opt(prev, "dispatchOrigin"), &inc("dispatchOrigin")?)
        && same(&jsv::opt(prev, "startsNewTurn"), &inc("startsNewTurn")?)
        && same(&jsv::opt(prev, "turnId"), &inc("turnId")?)
        && same(&jsv::opt(prev, "createdAt"), &inc("createdAt")?)
        && same(&jsv::opt(prev, "updatedAt"), &inc("updatedAt")?)
        && same(&jsv::opt(prev, "streaming"), &streaming)
        && same(&jsv::opt(prev, "source"), &inc("source")?)
        && same(&jsv::opt(prev, "completedAt"), &completed_at)
        && same(&jsv::opt(prev, "attachments"), &attachments)
        && text_segment_arrays_equal(&jsv::opt(prev, "textSegments"), &inc("textSegments")?)
        && provider_reference_arrays_equal(&previous_skills, &skills)
        && provider_reference_arrays_equal(&previous_mentions, &mentions)
    {
        return Ok(prev.clone());
    }

    let mut m = Map::new();
    m.insert("id".to_string(), inc("id")?);
    m.insert("role".to_string(), inc("role")?);
    m.insert("text".to_string(), inc("text")?);
    set_if_truthy(&mut m, "asyncUserInput", &async_user_input);
    let inc_ts = inc("textSegments")?;
    if !is_undef(&inc_ts)
        && member_length(&inc_ts, "incoming.textSegments.length")?.unwrap_or(0.0) > 0.0
    {
        m.insert(
            "textSegments".to_string(),
            Value::Array(spread_iterable(&inc_ts, "incoming.textSegments")?),
        );
    }
    set_if_truthy(&mut m, "dispatchMode", &inc("dispatchMode")?);
    set_if_truthy(&mut m, "dispatchOrigin", &inc("dispatchOrigin")?);
    let starts_new_turn = inc("startsNewTurn")?;
    set_if_defined(&mut m, "startsNewTurn", &starts_new_turn);
    m.insert("turnId".to_string(), inc("turnId")?);
    m.insert("createdAt".to_string(), inc("createdAt")?);
    m.insert("updatedAt".to_string(), inc("updatedAt")?);
    m.insert("streaming".to_string(), streaming);
    m.insert("source".to_string(), inc("source")?);
    set_if_truthy(&mut m, "completedAt", &completed_at);
    set_if_truthy(&mut m, "attachments", &attachments);
    if member_length(&skills, "skills.length")?.unwrap_or(0.0) > 0.0 {
        m.insert(
            "skills".to_string(),
            Value::Array(spread_iterable(&skills, "skills")?),
        );
    }
    if member_length(&mentions, "mentions.length")?.unwrap_or(0.0) > 0.0 {
        m.insert(
            "mentions".to_string(),
            Value::Array(spread_iterable(&mentions, "mentions")?),
        );
    }
    Ok(Value::Object(m))
}

/// `normalizeModelSelection(value, previous)`.
pub(crate) fn normalize_model_selection(value: &Value, previous: &Value) -> Result<Value, String> {
    let model = jsv::member(value, "model", "value.model")?
        .cloned()
        .unwrap_or_else(undef);
    let provider = jsv::member(value, "provider", "value.provider")?
        .cloned()
        .unwrap_or_else(undef);
    // `normalizeModelSlug(value.model, value.provider) ?? value.model`
    let slug = crate::shared::normalize_model_slug(&model, &provider)?;
    let normalized_model = if jsv::is_nullish(&slug) {
        model.clone()
    } else {
        slug
    };
    let next = if jsv::strict_eq(&normalized_model, &model) {
        value.clone()
    } else {
        let mut m = spread_any(value);
        m.insert("model".to_string(), normalized_model);
        Value::Object(m)
    };
    Ok(
        if jsv::truthy(previous) && deep_equal_json(previous, &next) {
            previous.clone()
        } else {
            next
        },
    )
}

/// `normalizeProjectScripts`.
fn normalize_project_scripts(incoming: &Value, previous: &Value) -> Result<Value, String> {
    let items = array_recv(
        incoming,
        "incoming.map",
        "incoming.map((script, index) => ...)",
    )?;
    let mut next = Vec::with_capacity(items.len());
    for (i, script) in items.iter().enumerate() {
        let existing = opt_elem(previous, i);
        if jsv::truthy(&existing) && deep_equal_json(&existing, script) {
            next.push(existing);
        } else {
            next.push(script.clone());
        }
    }
    let next_v = Value::Array(next);
    Ok(if arrays_shallow_equal(previous, &next_v) {
        previous.clone()
    } else {
        next_v
    })
}

/// `normalizeProposedPlans(incoming, previous)`.
pub(crate) fn normalize_proposed_plans(
    incoming: &Value,
    previous: Option<&Value>,
) -> Result<Value, String> {
    let undef_v = undef();
    let prev = previous.unwrap_or(&undef_v);
    let items = array_recv(incoming, "incoming.map", "incoming.map((plan) => ...)")?;
    let mut previous_by_id: HashMap<String, &Value> = HashMap::new();
    if !jsv::is_nullish(prev) {
        for p in array_recv(
            prev,
            "previous.map",
            "previous.map((plan) => [plan.id, plan])",
        )? {
            let id = jsv::member(p, "id", "plan.id")?
                .cloned()
                .unwrap_or_else(undef);
            previous_by_id.insert(js_map_key(&id), p);
        }
    }
    let mut next = Vec::with_capacity(items.len());
    for plan in items {
        let id = jsv::member(plan, "id", "plan.id")?
            .cloned()
            .unwrap_or_else(undef);
        let existing = previous_by_id.get(&js_map_key(&id));
        let reuse = existing.is_some_and(|e| {
            jsv::truthy(e)
                && same(&jsv::opt(e, "turnId"), &jsv::opt(plan, "turnId"))
                && same(
                    &jsv::opt(e, "planMarkdown"),
                    &jsv::opt(plan, "planMarkdown"),
                )
                && same(
                    &jsv::opt(e, "implementedAt"),
                    &jsv::opt(plan, "implementedAt"),
                )
                && same(
                    &jsv::opt(e, "implementationThreadId"),
                    &jsv::opt(plan, "implementationThreadId"),
                )
                && same(&jsv::opt(e, "createdAt"), &jsv::opt(plan, "createdAt"))
                && same(&jsv::opt(e, "updatedAt"), &jsv::opt(plan, "updatedAt"))
        });
        if let (true, Some(e)) = (reuse, existing) {
            next.push((*e).clone());
            continue;
        }
        let mut m = Map::new();
        m.insert("id".to_string(), id);
        for k in [
            "turnId",
            "planMarkdown",
            "implementedAt",
            "implementationThreadId",
            "createdAt",
            "updatedAt",
        ] {
            m.insert(
                k.to_string(),
                jsv::member(plan, k, &format!("plan.{k}"))?
                    .cloned()
                    .unwrap_or_else(undef),
            );
        }
        next.push(Value::Object(m));
    }
    let next_v = Value::Array(next);
    Ok(if arrays_shallow_equal(prev, &next_v) {
        prev.clone()
    } else {
        next_v
    })
}

/// `mergeTurnDiffFilesByPath` — duplicate paths fold their additions/deletions into the first
/// entry (keeping its `kind`), in first-seen order.
fn merge_turn_diff_files_by_path(files: &Value) -> Result<Value, String> {
    let items = spread_iterable(files, "files")?;
    let mut order: Vec<String> = Vec::new();
    let mut by_path: HashMap<String, Value> = HashMap::new();
    for file in &items {
        let path = jsv::member(file, "path", "file.path")?
            .cloned()
            .unwrap_or_else(undef);
        let key = js_map_key(&path);
        match by_path.get(&key) {
            None => {
                order.push(key.clone());
                by_path.insert(key, file.clone());
            }
            Some(existing) => {
                let mut m = Map::new();
                m.insert("path".to_string(), path);
                m.insert("kind".to_string(), jsv::opt(existing, "kind").clone());
                let zero = crate::js::number(0.0);
                m.insert(
                    "additions".to_string(),
                    js_add(
                        coalesce(&jsv::opt(existing, "additions"), &zero),
                        coalesce(&jsv::opt(file, "additions"), &zero),
                    ),
                );
                m.insert(
                    "deletions".to_string(),
                    js_add(
                        coalesce(&jsv::opt(existing, "deletions"), &zero),
                        coalesce(&jsv::opt(file, "deletions"), &zero),
                    ),
                );
                by_path.insert(key, Value::Object(m));
            }
        }
    }
    Ok(Value::Array(
        order
            .iter()
            .filter_map(|k| by_path.get(k).cloned())
            .collect(),
    ))
}

/// `normalizeTurnDiffFiles(incoming, previous)`.
pub(crate) fn normalize_turn_diff_files(
    incoming: &Value,
    previous: Option<&Value>,
) -> Result<Value, String> {
    let undef_v = undef();
    let prev = previous.unwrap_or(&undef_v);
    let merged = merge_turn_diff_files_by_path(incoming)?;
    let merged_items = merged.as_array();
    let mut next = Vec::with_capacity(merged_items.map_or(0, Vec::len));
    if let Some(merged_items) = merged_items {
        for (i, file) in merged_items.iter().enumerate() {
            let existing = opt_elem(prev, i);
            let reuse = jsv::truthy(&existing)
                && same(&jsv::opt(&existing, "path"), &jsv::opt(file, "path"))
                && same(&jsv::opt(&existing, "kind"), &jsv::opt(file, "kind"))
                && same(
                    &jsv::opt(&existing, "additions"),
                    &jsv::opt(file, "additions"),
                )
                && same(
                    &jsv::opt(&existing, "deletions"),
                    &jsv::opt(file, "deletions"),
                );
            if reuse {
                next.push(existing);
            } else {
                next.push(file.clone());
            }
        }
    }
    let next_v = Value::Array(next);
    Ok(if arrays_shallow_equal(prev, &next_v) {
        prev.clone()
    } else {
        next_v
    })
}

// ---- activities (`storeNormalization.ts`) ------------------------------------------------------

/// `resolveTurnAlignedDropCount`.
fn resolve_turn_aligned_drop_count(activities: &[Value], minimum_drop_count: usize) -> usize {
    let boundary = activities
        .get(minimum_drop_count.saturating_sub(1))
        .map(|a| coalesce(&jsv::opt(a, "turnId"), &Value::Null).clone())
        .unwrap_or(Value::Null);
    let mut drop_count = minimum_drop_count;
    while drop_count < activities.len()
        && same(
            &coalesce(&jsv::opt(&activities[drop_count], "turnId"), &Value::Null).clone(),
            &boundary,
        )
    {
        drop_count += 1;
    }
    if drop_count >= activities.len() {
        minimum_drop_count
    } else {
        drop_count
    }
}

/// `pendingInteractionActivityIdentity` → `{interactionKind, requestId, lifecycleGeneration,
/// createdAt, key}` or null.
fn pending_interaction_activity_identity(
    activity: &Value,
) -> Result<Option<Map<String, Value>>, String> {
    let kind = jsv::member(activity, "kind", "activity.kind")?
        .cloned()
        .unwrap_or_else(undef);
    let kind_str = match kind.as_str() {
        Some(k) if PENDING_INTERACTION_ACTIVITY_KINDS.contains(&k) => k,
        _ => return Ok(None),
    };
    let payload = jsv::member(activity, "payload", "activity.payload")?
        .cloned()
        .unwrap_or_else(undef);
    let payload_rec = as_activity_record(&payload);
    let request_id = payload_rec.and_then(|p| p.get("requestId"));
    let Some(request_id_str) = request_id.and_then(Value::as_str) else {
        return Ok(None);
    };
    if request_id_str.trim().is_empty() {
        return Ok(None);
    }
    let interaction_kind =
        if kind_str.starts_with("approval.") || kind_str.starts_with("provider.approval.") {
            "approval"
        } else {
            "userInput"
        };
    let generation = payload_rec.and_then(|p| p.get("lifecycleGeneration"));
    let lifecycle_generation: Value = match generation.and_then(Value::as_str) {
        Some(g) if !g.is_empty() => Value::String(g.to_string()),
        _ => Value::Null,
    };
    let generation_arg: Option<&Value> = match &lifecycle_generation {
        Value::Null => None,
        g => Some(g),
    };
    // `pending_request_instance_key` is total — it returns `String`, not `Result`.
    let key = format!(
        "{}:{}",
        interaction_kind,
        crate::shared::pending_request_instance_key(
            &Value::String(request_id_str.to_string()),
            generation_arg,
        )
    );
    let mut identity = Map::new();
    identity.insert(
        "interactionKind".to_string(),
        Value::String(interaction_kind.to_string()),
    );
    identity.insert(
        "requestId".to_string(),
        Value::String(request_id_str.to_string()),
    );
    identity.insert("lifecycleGeneration".to_string(), lifecycle_generation);
    identity.insert(
        "createdAt".to_string(),
        jsv::member(activity, "createdAt", "activity.createdAt")?
            .cloned()
            .unwrap_or_else(undef),
    );
    identity.insert("key".to_string(), Value::String(key));
    Ok(Some(identity))
}

/// `pendingInteractionsFromActivities` — the still-open requests after replaying resolved
/// markers, minus requests proven stale by later failure evidence.
fn pending_interactions_from_activities(activities: &[Value]) -> Result<Vec<Value>, String> {
    let mut open_requests: Map<String, Value> = Map::new();
    for activity in activities {
        let kind = jsv::member(activity, "kind", "activity.kind")?
            .cloned()
            .unwrap_or_else(undef);
        let kind_str = kind.as_str().unwrap_or("");
        let tracked = PENDING_INTERACTION_REQUEST_KINDS.contains(&kind_str)
            || kind_str == "approval.resolved"
            || kind_str == "user-input.resolved";
        if !tracked {
            continue;
        }
        let Some(identity) = pending_interaction_activity_identity(activity)? else {
            continue;
        };
        let identity_v = Value::Object(identity);
        let key = format!(
            "{}:{}",
            js_string(&jsv::opt(&identity_v, "interactionKind")),
            js_string(&jsv::opt(&identity_v, "requestId"))
        );
        if kind_str == "approval.requested" || kind_str == "user-input.requested" {
            open_requests.insert(key, identity_v);
            continue;
        }
        // `approval.resolved` / `user-input.resolved`
        let open_generation = open_requests
            .get(&key)
            .map(|r| jsv::opt(r, "lifecycleGeneration"))
            .unwrap_or_else(undef);
        if jsv::opt(&identity_v, "lifecycleGeneration").is_null()
            || same(
                &open_generation,
                &jsv::opt(&identity_v, "lifecycleGeneration"),
            )
        {
            open_requests.shift_remove(&key);
        }
    }
    if open_requests.is_empty() {
        return Ok(Vec::new());
    }
    // `createStalePendingInteractionMatcher(activities)` applied to each open request.
    let activities_v = Value::Array(activities.to_vec());
    let mut out = Vec::with_capacity(open_requests.len());
    for (_, request) in open_requests {
        // Shared signature: `(interaction, activities, lastPendingSeenAt)` — the third
        // argument is currently unused by the matcher.
        if !crate::shared::is_stale_pending_interaction(&request, &activities_v, &Value::Null)? {
            out.push(request);
        }
    }
    Ok(out)
}

/// `retainTaskActivityIds` — keep the latest task-state patches so trimming cannot resurrect
/// completed work or erase ownership/launch identity.
fn retain_task_activity_ids(
    activities: &[Value],
    retained_ids: &mut HashSet<String>,
) -> Result<(), String> {
    let ordered = crate::shared::ordered_activities(&Value::Array(activities.to_vec()))?;
    let items = array_recv(&ordered, "ordered", "orderedActivities(activities)")?;
    let mut latest: Map<String, Value> = Map::new();
    for activity in items {
        let kind = jsv::member(activity, "kind", "activity.kind")?
            .cloned()
            .unwrap_or_else(undef);
        let kind_str = kind.as_str().unwrap_or("");
        if kind_str == "turn.tasks.updated" {
            latest.insert("task-list".to_string(), activity.clone());
            continue;
        }
        let payload = jsv::member(activity, "payload", "activity.payload")?
            .cloned()
            .unwrap_or_else(undef);
        let payload_rec = as_activity_record(&payload);
        let native_event_type = payload_rec
            .and_then(|p| p.get("nativeEventType"))
            .cloned()
            .unwrap_or_else(undef);
        if kind_str == "runtime.warning"
            && jsv::strict_eq(
                &native_event_type,
                &Value::String("background_tasks_changed".to_string()),
            )
        {
            latest.insert("background-notice".to_string(), activity.clone());
            continue;
        }
        let Some(task_id) = payload_rec
            .and_then(|p| p.get("taskId"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if kind_str == "task.started" {
            retained_ids.insert(js_map_key(
                &jsv::member(activity, "id", "activity.id")?
                    .cloned()
                    .unwrap_or_else(undef),
            ));
            continue;
        }
        if kind_str != "task.updated" && kind_str != "task.progress" && kind_str != "task.completed"
        {
            continue;
        }
        if kind_str != "task.progress" {
            latest.insert(
                json_key(&[
                    &Value::String(task_id.to_string()),
                    &Value::String("lifecycle".to_string()),
                ]),
                activity.clone(),
            );
        }
        if let Some(rec) = payload_rec {
            for field in rec.keys() {
                if field != "taskId" {
                    latest.insert(
                        json_key(&[
                            &Value::String(task_id.to_string()),
                            &Value::String("field".to_string()),
                            &Value::String(field.clone()),
                        ]),
                        activity.clone(),
                    );
                }
            }
        }
    }
    for (_, activity) in latest {
        retained_ids.insert(js_map_key(
            &jsv::member(&activity, "id", "activity.id")?
                .cloned()
                .unwrap_or_else(undef),
        ));
    }
    Ok(())
}

/// `JSON.stringify` of a small heterogeneous array — keys for the `latest` map.
fn json_key(values: &[&Value]) -> String {
    let inner: Vec<String> = values
        .iter()
        .map(|v| serde_json::to_string(v).unwrap_or_default())
        .collect();
    format!("[{}]", inner.join(","))
}

/// `capThreadActivities`.
pub(crate) fn cap_thread_activities(activities: &Value) -> Result<Value, String> {
    let len = member_length(activities, "activities.length")?;
    if let Some(n) = len
        && n <= MAX_THREAD_ACTIVITIES as f64
    {
        return Ok(activities.clone());
    }
    let items = match activities {
        Value::Array(a) => a.clone(),
        // `String.prototype.slice` exists, so strings get one step further and fail at `tail.map`.
        Value::String(_) => {
            return Err(
                "tail.map is not a function. (In 'tail.map((activity) => activity.id)', \
                 'tail.map' is undefined)"
                    .to_string(),
            );
        }
        _ => {
            return Err(
                "activities.slice is not a function. (In 'activities.slice(dropCount)', \
                 'activities.slice' is undefined)"
                    .to_string(),
            );
        }
    };
    let drop_count = resolve_turn_aligned_drop_count(&items, items.len() - MAX_THREAD_ACTIVITIES);
    let tail = &items[drop_count..];
    let mut retained_ids: HashSet<String> = HashSet::new();
    for activity in tail {
        retained_ids.insert(js_map_key(
            &jsv::member(activity, "id", "activity.id")?
                .cloned()
                .unwrap_or_else(undef),
        ));
    }
    retain_task_activity_ids(&items, &mut retained_ids)?;
    let mut retained_requests: Map<String, Value> = Map::new();
    for request in pending_interactions_from_activities(&items)? {
        retained_requests.insert(js_string(&jsv::opt(&request, "key")), request);
    }
    // A request in the tail may already be closed by an older array entry; keep its settlement
    // evidence alongside it so trimming cannot reopen the card.
    for activity in tail {
        let kind = jsv::member(activity, "kind", "activity.kind")?
            .cloned()
            .unwrap_or_else(undef);
        let is_request = kind
            .as_str()
            .is_some_and(|k| PENDING_INTERACTION_REQUEST_KINDS.contains(&k));
        if !is_request {
            continue;
        }
        if let Some(identity) = pending_interaction_activity_identity(activity)? {
            let key = js_string(&jsv::opt(&Value::Object(identity.clone()), "key"));
            retained_requests.insert(key, Value::Object(identity));
        }
    }
    if retained_requests.is_empty() {
        let mut out = Vec::new();
        for activity in &items {
            if retained_ids.contains(&js_map_key(
                &jsv::member(activity, "id", "activity.id")?
                    .cloned()
                    .unwrap_or_else(undef),
            )) {
                out.push(activity.clone());
            }
        }
        return Ok(Value::Array(out));
    }
    let mut oldest_retained_request_at: Map<String, Value> = Map::new();
    for (_, request) in &retained_requests {
        let key = format!(
            "{}:{}",
            js_string(&jsv::opt(request, "interactionKind")),
            js_string(&jsv::opt(request, "requestId"))
        );
        let created = jsv::opt(request, "createdAt");
        match oldest_retained_request_at.get(&key) {
            None => {
                oldest_retained_request_at.insert(key, created);
            }
            Some(previous) if js_lt(&created, previous) => {
                oldest_retained_request_at.insert(key, created);
            }
            _ => {}
        }
    }
    let mut latest_evidence: Map<String, Value> = Map::new();
    for activity in &items {
        let Some(identity) = pending_interaction_activity_identity(activity)? else {
            continue;
        };
        let identity_v = Value::Object(identity);
        let kind = jsv::member(activity, "kind", "activity.kind")?
            .cloned()
            .unwrap_or_else(undef);
        let is_request = kind
            .as_str()
            .is_some_and(|k| PENDING_INTERACTION_REQUEST_KINDS.contains(&k));
        let request_key = format!(
            "{}:{}",
            js_string(&jsv::opt(&identity_v, "interactionKind")),
            js_string(&jsv::opt(&identity_v, "requestId"))
        );
        let oldest_request_at = oldest_retained_request_at.get(&request_key);
        let identity_key = js_string(&jsv::opt(&identity_v, "key"));
        let keep = if !jsv::opt(&identity_v, "lifecycleGeneration").is_null() {
            retained_requests.contains_key(&identity_key)
        } else if is_request {
            retained_requests.get(&identity_key).is_some_and(|r| {
                same(
                    &jsv::opt(r, "createdAt"),
                    &jsv::opt(&identity_v, "createdAt"),
                )
            })
        } else {
            oldest_request_at.is_some_and(|o| js_le(o, &jsv::opt(&identity_v, "createdAt")))
        };
        if !keep {
            continue;
        }
        if is_request {
            retained_ids.insert(js_map_key(
                &jsv::member(activity, "id", "activity.id")?
                    .cloned()
                    .unwrap_or_else(undef),
            ));
            continue;
        }
        let payload = jsv::member(activity, "payload", "activity.payload")?
            .cloned()
            .unwrap_or_else(undef);
        let detail = as_activity_record(&payload)
            .and_then(|p| p.get("detail"))
            .and_then(Value::as_str);
        let kind_str = kind.as_str().unwrap_or("");
        let stale = (kind_str == "provider.approval.respond.failed"
            || kind_str == "provider.user-input.respond.failed")
            && crate::shared::is_stale_pending_request_failure_detail(detail);
        let evidence_key = format!(
            "{}:{}{}",
            identity_key,
            kind_str,
            if stale { ":stale" } else { "" }
        );
        let created_at = jsv::opt(&identity_v, "createdAt");
        match latest_evidence.get(&evidence_key) {
            None => {
                latest_evidence.insert(evidence_key, activity.clone());
            }
            Some(previous) if js_ge(&created_at, &jsv::opt(previous, "createdAt")) => {
                latest_evidence.insert(evidence_key, activity.clone());
            }
            _ => {}
        }
    }
    for (_, activity) in &latest_evidence {
        retained_ids.insert(js_map_key(
            &jsv::member(activity, "id", "activity.id")?
                .cloned()
                .unwrap_or_else(undef),
        ));
    }
    let mut out = Vec::new();
    for activity in &items {
        if retained_ids.contains(&js_map_key(
            &jsv::member(activity, "id", "activity.id")?
                .cloned()
                .unwrap_or_else(undef),
        )) {
            out.push(activity.clone());
        }
    }
    Ok(Value::Array(out))
}

/// `activitiesEqual`.
fn activities_equal(left: &Value, right: &Value) -> Result<bool, String> {
    for key in ["kind", "tone", "summary"] {
        if !same(
            &jsv::member(left, key, &format!("left.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
            &jsv::member(right, key, &format!("right.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
        ) {
            return Ok(false);
        }
    }
    if !deep_equal_json(
        &jsv::member(left, "payload", "left.payload")?
            .cloned()
            .unwrap_or_else(undef),
        &jsv::member(right, "payload", "right.payload")?
            .cloned()
            .unwrap_or_else(undef),
    ) {
        return Ok(false);
    }
    for key in ["turnId", "sequence", "sequenceSource", "createdAt"] {
        if !same(
            &jsv::member(left, key, &format!("left.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
            &jsv::member(right, key, &format!("right.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `normalizeActivityCommandValue` — trimmed command string, or the non-empty trimmed parts of a
/// command array joined by a space.
fn normalize_activity_command_value(value: &Value) -> Option<String> {
    if let Some(s) = value.as_str() {
        let t = s.trim();
        return if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        };
    }
    if let Value::Array(a) = value {
        let parts: Vec<String> = a
            .iter()
            .map(|e| e.as_str().map(|s| s.trim().to_string()).unwrap_or_default())
            .filter(|s| !s.is_empty())
            .collect();
        return if parts.is_empty() {
            None
        } else {
            Some(parts.join(" "))
        };
    }
    None
}

/// `activityPayloadDetailScore` — prefers the side that carries richer payload detail.
fn activity_payload_detail_score(activity: &Value) -> Result<f64, String> {
    let payload = jsv::member(activity, "payload", "activity.payload")?
        .cloned()
        .unwrap_or_else(undef);
    let payload_rec = as_activity_record(&payload);
    let data_v = payload_rec
        .and_then(|p| p.get("data"))
        .cloned()
        .unwrap_or_else(undef);
    let data_rec = as_activity_record(&data_v);
    let item_v = data_rec
        .and_then(|d| d.get("item"))
        .cloned()
        .unwrap_or_else(undef);
    let item_rec = as_activity_record(&item_v);
    // `item?.commandActions ?? data?.commandActions ?? payload?.commandActions` — `??` skips
    // nullish values, so each candidate reads `undefined` when the field is absent *or* null.
    let nullish_field = |rec: Option<&Map<String, Value>>, key: &str| -> Value {
        rec.and_then(|r| r.get(key)).cloned().unwrap_or_else(undef)
    };
    let command_actions = coalesce(
        &nullish_field(item_rec, "commandActions"),
        coalesce(
            &nullish_field(data_rec, "commandActions"),
            &nullish_field(payload_rec, "commandActions"),
        ),
    )
    .clone();
    let command = coalesce(
        &nullish_field(item_rec, "command"),
        coalesce(
            &nullish_field(data_rec, "command"),
            &nullish_field(payload_rec, "command"),
        ),
    )
    .clone();
    let mut score = 0.0;
    if payload_rec
        .and_then(|p| p.get("itemType"))
        .is_some_and(jsv::truthy)
    {
        score += 4.0;
    }
    if payload_rec
        .and_then(|p| p.get("title"))
        .is_some_and(jsv::truthy)
    {
        score += 1.0;
    }
    if payload_rec
        .and_then(|p| p.get("detail"))
        .is_some_and(jsv::truthy)
    {
        score += 2.0;
    }
    // `if (data)` / `if (item)` — `asActivityRecord` counts arrays too.
    if as_recordish(&data_v) {
        score += 2.0;
    }
    if as_recordish(&item_v) {
        score += 4.0;
    }
    if normalize_activity_command_value(&command).is_some() {
        score += 8.0;
    }
    if command_actions.as_array().is_some_and(|a| !a.is_empty()) {
        score += 8.0;
    }
    Ok(score)
}

/// `preferRicherActivity`.
fn prefer_richer_activity(previous: &Value, incoming: &Value) -> Result<Value, String> {
    if activities_equal(previous, incoming)? {
        return Ok(previous.clone());
    }
    let previous_score = activity_payload_detail_score(previous)?;
    let incoming_score = activity_payload_detail_score(incoming)?;
    Ok(if incoming_score < previous_score {
        previous.clone()
    } else {
        incoming.clone()
    })
}

/// `dedupeActivitiesById`.
pub(crate) fn dedupe_activities_by_id(activities: &Value) -> Result<Value, String> {
    let items = spread_iterable(activities, "activities")?;
    let mut index_by_id: HashMap<String, usize> = HashMap::new();
    let mut result: Vec<Value> = Vec::with_capacity(items.len());
    for activity in &items {
        let id = jsv::member(activity, "id", "activity.id")?
            .cloned()
            .unwrap_or_else(undef);
        let key = js_map_key(&id);
        match index_by_id.get(&key) {
            None => {
                index_by_id.insert(key, result.len());
                result.push(activity.clone());
            }
            Some(&existing_index) => {
                result[existing_index] = prefer_richer_activity(&result[existing_index], activity)?;
            }
        }
    }
    let result_v = Value::Array(result);
    Ok(if arrays_shallow_equal(activities, &result_v) {
        activities.clone()
    } else {
        result_v
    })
}

/// `dedupeActivitiesByIdAfterAppend` — the streaming-hot-path dedupe: only an appended tail can
/// introduce a duplicate; any other shape falls back to the full pass.
pub(crate) fn dedupe_activities_by_id_after_append(
    activities: &Value,
    previous_activities: &Value,
    previous_activity_by_id: &Value,
) -> Result<Value, String> {
    let prev_len = js_length(previous_activities);
    let acts_len = js_length(activities);
    let needs_fallback = is_undef(previous_activities)
        || is_undef(previous_activity_by_id)
        || prev_len == Some(0.0)
        || match (prev_len, acts_len) {
            (Some(p), Some(a)) => p > a,
            _ => false,
        };
    if needs_fallback {
        return dedupe_activities_by_id(activities);
    }
    if let Some(pl) = prev_len {
        for i in 0..(pl as usize) {
            if !same(&opt_elem(activities, i), &opt_elem(previous_activities, i)) {
                return dedupe_activities_by_id(activities);
            }
        }
        let start = pl as usize;
        if let Some(al) = acts_len {
            let mut appended_ids: HashSet<String> = HashSet::new();
            for i in start..(al as usize) {
                let element = item_at(activities, i);
                let id = jsv::member(&element, "id", "activities[index].id")?
                    .cloned()
                    .unwrap_or_else(undef);
                let known = index_access(
                    previous_activity_by_id,
                    &record_key(&id),
                    "previousActivityById[id]",
                )?
                .is_some_and(|v| !is_undef(v));
                if known || appended_ids.contains(&js_map_key(&id)) {
                    return dedupe_activities_by_id(activities);
                }
                appended_ids.insert(js_map_key(&id));
            }
        }
    }
    Ok(activities.clone())
}

/// `normalizeActivities(incoming, previous)`.
pub(crate) fn normalize_activities(activities: &Value, previous: &Value) -> Result<Value, String> {
    let previous_activities = if jsv::truthy(previous) {
        Some(dedupe_activities_by_id(previous)?)
    } else {
        None
    };
    let incoming_activities = dedupe_activities_by_id(activities)?;
    let incoming_items = incoming_activities.as_array().cloned().unwrap_or_default();
    let mut previous_by_id: HashMap<String, Value> = HashMap::new();
    if let Some(prev) = previous_activities.as_ref().and_then(Value::as_array) {
        for activity in prev {
            let id = jsv::member(activity, "id", "activity.id")?
                .cloned()
                .unwrap_or_else(undef);
            previous_by_id.insert(js_map_key(&id), activity.clone());
        }
    }
    let mut next = Vec::with_capacity(incoming_items.len());
    for activity in &incoming_items {
        let id = jsv::member(activity, "id", "activity.id")?
            .cloned()
            .unwrap_or_else(undef);
        if let Some(existing) = previous_by_id.get(&js_map_key(&id)) {
            let preferred = prefer_richer_activity(existing, activity)?;
            if same(&preferred, existing) || activities_equal(existing, &preferred)? {
                next.push(existing.clone());
            } else {
                next.push(preferred);
            }
            continue;
        }
        next.push(activity.clone());
    }
    let capped = cap_thread_activities(&Value::Array(next))?;
    Ok(if arrays_shallow_equal(previous, &capped) {
        previous.clone()
    } else {
        capped
    })
}

/// `createThreadActivityAccumulator` — the incremental equivalent of repeatedly calling
/// `normalizeActivities([...previous, activity], previous)` while folding a batch of
/// `thread.activity-appended` events.
pub(crate) struct ThreadActivityAccumulator {
    previous: Value,
    working: Vec<Value>,
    owned: bool,
    pending_dedupe_change: bool,
    index_by_id: Option<HashMap<String, usize>>,
}

pub(crate) fn create_thread_activity_accumulator(
    activities: &Value,
) -> Result<ThreadActivityAccumulator, String> {
    let deduped = dedupe_activities_by_id(activities)?;
    // `deduped !== previous`: dedupe only returns a new array when it removed a duplicate, so a
    // structural difference means the first `append()` must report a change.
    let pending_dedupe_change = !same(&deduped, activities);
    let working = deduped.as_array().cloned().unwrap_or_default();
    Ok(ThreadActivityAccumulator {
        previous: activities.clone(),
        working,
        owned: pending_dedupe_change,
        pending_dedupe_change,
        index_by_id: None,
    })
}

impl ThreadActivityAccumulator {
    fn ensure_index_by_id(&mut self) -> Result<(), String> {
        if self.index_by_id.is_none() {
            let mut map = HashMap::with_capacity(self.working.len());
            for (index, activity) in self.working.iter().enumerate() {
                let id = jsv::member(activity, "id", "activity.id")?
                    .cloned()
                    .unwrap_or_else(undef);
                map.insert(js_map_key(&id), index);
            }
            self.index_by_id = Some(map);
        }
        Ok(())
    }

    fn ensure_owned(&mut self) {
        if !self.owned {
            self.working = self.working.clone();
            self.owned = true;
        }
    }

    /// Appends one already-sequenced activity. Returns true when the accumulated list changed.
    pub(crate) fn append(&mut self, activity: &Value) -> Result<bool, String> {
        self.ensure_index_by_id()?;
        let id = jsv::member(activity, "id", "activity.id")?
            .cloned()
            .unwrap_or_else(undef);
        let key = js_map_key(&id);
        let existing_index = self.index_by_id.as_ref().and_then(|m| m.get(&key).copied());
        let mut changed = false;
        match existing_index {
            None => {
                self.ensure_owned();
                self.working.push(activity.clone());
                let index = self.working.len() - 1;
                if let Some(m) = self.index_by_id.as_mut() {
                    m.insert(key, index);
                }
                changed = true;
            }
            Some(i) => {
                let preferred = prefer_richer_activity(&self.working[i], activity)?;
                if !same(&preferred, &self.working[i]) {
                    self.ensure_owned();
                    self.working[i] = preferred;
                    changed = true;
                }
            }
        }
        if self.working.len() > MAX_THREAD_ACTIVITIES {
            let capped = cap_thread_activities(&Value::Array(self.working.clone()))?;
            let capped_len = capped.as_array().map_or(0, Vec::len);
            // `capThreadActivities` only filters, so an unchanged length means unchanged contents.
            if capped_len != self.working.len() {
                self.working = capped.as_array().cloned().unwrap_or_default();
                self.owned = true;
                self.index_by_id = None;
                changed = true;
            }
        }
        if self.pending_dedupe_change {
            self.pending_dedupe_change = false;
            return Ok(true);
        }
        Ok(changed)
    }

    /// Accumulated activities, reference-identical to `previous` when nothing changed.
    pub(crate) fn result(&self) -> Value {
        let working_v = Value::Array(self.working.clone());
        if arrays_shallow_equal(&self.previous, &working_v) {
            self.previous.clone()
        } else {
            working_v
        }
    }
}

// ---- thread error/session (`storeNormalization.ts`) --------------------------------------------

/// `isNonFatalThreadErrorMessage`.
fn is_non_fatal_thread_error_message(message: &Value) -> Result<bool, String> {
    if !jsv::truthy(message) {
        return Ok(false);
    }
    let Some(s) = message.as_str() else {
        return Err(
            "message.trim is not a function. (In 'message.trim()', 'message.trim' is undefined)"
                .to_string(),
        );
    };
    Ok(s.trim()
        .to_lowercase()
        .contains("write_stdin failed: stdin is closed for this session"))
}

/// `normalizeThreadErrorMessage`.
pub(crate) fn normalize_thread_error_message(message: &Value) -> Result<Value, String> {
    if jsv::truthy(message) && !is_non_fatal_thread_error_message(message)? {
        Ok(message.clone())
    } else {
        Ok(Value::Null)
    }
}

/// `toLegacySessionStatus`.
fn to_legacy_session_status(status: &Value) -> Value {
    Value::String(
        match status.as_str() {
            Some("starting") => "connecting",
            Some("running") => "running",
            Some("error") => "error",
            Some("ready") | Some("interrupted") => "ready",
            Some("idle") | Some("stopped") => "closed",
            _ => "closed",
        }
        .to_string(),
    )
}

/// `toLegacyProvider`.
fn to_legacy_provider(provider_name: &Value) -> Value {
    let name = match provider_name.as_str() {
        Some("codex") | Some("claudeAgent") | Some("cursor") | Some("antigravity")
        | Some("grok") | Some("droid") | Some("opencode") | Some("pi") | Some("devin")
        | Some("omp") => provider_name.as_str().unwrap_or("codex").to_string(),
        Some("kilo") => "opencode".to_string(),
        _ => "codex".to_string(),
    };
    Value::String(name)
}

/// `normalizeThreadSession(incoming, previous)`.
pub(crate) fn normalize_thread_session(
    incoming: &Value,
    previous: &Value,
) -> Result<Value, String> {
    if !jsv::truthy(incoming) {
        return Ok(Value::Null);
    }
    let inc = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(incoming, k, &format!("incoming.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let last_error = inc("lastError")?;
    let next_last_error =
        if jsv::truthy(&last_error) && !is_non_fatal_thread_error_message(&last_error)? {
            last_error
        } else {
            undef()
        };
    let mut m = Map::new();
    m.insert(
        "provider".to_string(),
        to_legacy_provider(&inc("providerName")?),
    );
    set_if_defined(&mut m, "providerInstanceId", &inc("providerInstanceId")?);
    m.insert(
        "status".to_string(),
        to_legacy_session_status(&inc("status")?),
    );
    m.insert("orchestrationStatus".to_string(), inc("status")?);
    m.insert("runtimeMode".to_string(), inc("runtimeMode")?);
    // `activeTurnId: incoming.activeTurnId ?? undefined` — the key is always present.
    m.insert("activeTurnId".to_string(), inc("activeTurnId")?);
    let updated_at = inc("updatedAt")?;
    m.insert("createdAt".to_string(), updated_at.clone());
    m.insert("updatedAt".to_string(), updated_at);
    set_if_truthy(&mut m, "lastError", &next_last_error);
    let next_session = Value::Object(m);
    if jsv::truthy(previous) {
        let all_equal = [
            "provider",
            "providerInstanceId",
            "status",
            "orchestrationStatus",
            "runtimeMode",
            "activeTurnId",
            "createdAt",
            "updatedAt",
            "lastError",
        ]
        .iter()
        .all(|k| same(&jsv::opt(previous, k), &jsv::opt(&next_session, k)));
        if all_equal {
            return Ok(previous.clone());
        }
    }
    Ok(next_session)
}

/// `resolveCreateBranchFlowCompletedMerge`.
pub(crate) fn resolve_create_branch_flow_completed_merge(input: &Value) -> Result<Value, String> {
    let get = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(input, k, &format!("input.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let null = Value::Null;
    let context_changed = !same(&get("currentBranch")?, &get("nextBranch")?)
        || !same(&get("currentWorktreePath")?, &get("nextWorktreePath")?)
        || !same(
            coalesce(&get("currentAssociatedWorktreePath")?, &null),
            coalesce(&get("nextAssociatedWorktreePath")?, &null),
        )
        || !same(
            coalesce(&get("currentAssociatedWorktreeBranch")?, &null),
            coalesce(&get("nextAssociatedWorktreeBranch")?, &null),
        )
        || !same(
            coalesce(&get("currentAssociatedWorktreeRef")?, &null),
            coalesce(&get("nextAssociatedWorktreeRef")?, &null),
        );
    if context_changed {
        return Ok(Value::Bool(jsv::truthy(coalesce(
            &get("nextCreateBranchFlowCompleted")?,
            &Value::Bool(false),
        ))));
    }
    let next_completed = get("nextCreateBranchFlowCompleted")?;
    if is_undef(&next_completed) {
        return Ok(Value::Bool(jsv::truthy(coalesce(
            &get("currentCreateBranchFlowCompleted")?,
            &Value::Bool(false),
        ))));
    }
    if jsv::truthy(coalesce(
        &get("currentCreateBranchFlowCompleted")?,
        &Value::Bool(false),
    )) && !jsv::truthy(&next_completed)
    {
        return Ok(Value::Bool(true));
    }
    Ok(Value::Bool(jsv::truthy(&next_completed)))
}

/// `basenameOfPath`.
fn basename_of_path(value: &Value) -> Result<Value, String> {
    let Some(s) = value.as_str() else {
        return Err(
            "value.split is not a function. (In 'value.split(/[/\\\\]/)', 'value.split' is \
             undefined)"
                .to_string(),
        );
    };
    let last = s.split(['/', '\\']).rfind(|seg| !seg.is_empty());
    Ok(match last {
        Some(seg) => Value::String(seg.to_string()),
        None => Value::Null,
    })
}

/// `normalizeProject(incoming, previous)`.
pub(crate) fn normalize_project(input: &Value, previous: &Value) -> Result<Value, String> {
    let remembered = get_remembered_project_ui_state();
    let inc = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(input, k, &format!("incoming.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let workspace_root = inc("workspaceRoot")?;
    let workspace_root_key = project_cwd_key(&workspace_root)?;
    let basename = basename_of_path(&workspace_root)?;
    let title = inc("title")?;
    let folder_name = if basename.is_null() {
        title.clone()
    } else {
        basename
    };
    let remembered_name = remembered
        .project_name_for_cwd(&workspace_root_key)
        .map(Value::String)
        .unwrap_or_else(undef);
    let local_name = coalesce(
        coalesce(&jsv::opt(previous, "localName"), &remembered_name),
        &Value::Null,
    )
    .clone();
    let remembered_appearance = remembered
        .project_appearance_for_cwd(&workspace_root_key)
        .unwrap_or_else(undef);
    let appearance = coalesce(
        coalesce(&jsv::opt(previous, "appearance"), &remembered_appearance),
        &Value::Null,
    )
    .clone();
    let incoming_dms = inc("defaultModelSelection")?;
    let default_model_selection = if incoming_dms.is_null() {
        Value::Null
    } else {
        normalize_model_selection(&incoming_dms, &jsv::opt(previous, "defaultModelSelection"))?
    };
    let scripts = normalize_project_scripts(&inc("scripts")?, &jsv::opt(previous, "scripts"))?;
    let empty: Vec<Value> = Vec::new();
    let empty_arr = Value::Array(empty);
    let incoming_additional_folders = coalesce(&inc("additionalFolders")?, &empty_arr).clone();
    let previous_additional_folders = jsv::opt(previous, "additionalFolders");
    let additional_folders = if jsv::truthy(&previous_additional_folders)
        && deep_equal_json(&previous_additional_folders, &incoming_additional_folders)
    {
        previous_additional_folders
    } else {
        incoming_additional_folders
    };
    let persisted_project_order_index = remembered.project_order_index_for_cwd(&workspace_root_key);
    let has_known_legacy_expansion = remembered.project_order_count() == 0
        && (remembered.expanded_project_count() > 0 || remembered.is_legacy_expansion_payload());
    let previous_expanded = if jsv::truthy(previous)
        && project_cwd_key(
            &jsv::member(previous, "cwd", "previous.cwd")?
                .cloned()
                .unwrap_or_else(undef),
        )? == workspace_root_key
    {
        jsv::opt(previous, "expanded")
    } else {
        undef()
    };
    let expanded = if !is_undef(&previous_expanded) {
        previous_expanded
    } else if persisted_project_order_index.is_some() || has_known_legacy_expansion {
        Value::Bool(remembered.is_project_expanded(&workspace_root_key))
    } else {
        Value::Bool(true)
    };

    if jsv::truthy(previous)
        && same(&jsv::opt(previous, "id"), &inc("id")?)
        && same(&jsv::opt(previous, "kind"), &inc("kind")?)
        && same(&jsv::opt(previous, "name"), coalesce(&local_name, &title))
        && same(&jsv::opt(previous, "remoteName"), &title)
        && same(&jsv::opt(previous, "folderName"), &folder_name)
        && same(&jsv::opt(previous, "localName"), &local_name)
        && same(
            coalesce(&jsv::opt(previous, "appearance"), &Value::Null),
            &appearance,
        )
        && same(&jsv::opt(previous, "cwd"), &workspace_root)
        && same(
            &jsv::opt(previous, "defaultModelSelection"),
            &default_model_selection,
        )
        && same(&jsv::opt(previous, "expanded"), &expanded)
        && same(
            coalesce(&jsv::opt(previous, "isPinned"), &Value::Bool(false)),
            coalesce(&inc("isPinned")?, &Value::Bool(false)),
        )
        && same(
            coalesce(&jsv::opt(previous, "spaceId"), &Value::Null),
            coalesce(&inc("spaceId")?, &Value::Null),
        )
        && same(&jsv::opt(previous, "createdAt"), &inc("createdAt")?)
        && same(&jsv::opt(previous, "updatedAt"), &inc("updatedAt")?)
        && same(&jsv::opt(previous, "scripts"), &scripts)
        && same(
            &jsv::opt(previous, "additionalFolders"),
            &additional_folders,
        )
    {
        return Ok(previous.clone());
    }

    let mut m = Map::new();
    m.insert("id".to_string(), inc("id")?);
    m.insert(
        "kind".to_string(),
        coalesce(&inc("kind")?, &Value::String("project".to_string())).clone(),
    );
    m.insert("name".to_string(), coalesce(&local_name, &title).clone());
    m.insert("remoteName".to_string(), title);
    m.insert("folderName".to_string(), folder_name);
    m.insert("localName".to_string(), local_name);
    m.insert("appearance".to_string(), appearance);
    m.insert("cwd".to_string(), workspace_root);
    m.insert("defaultModelSelection".to_string(), default_model_selection);
    m.insert("expanded".to_string(), expanded);
    m.insert(
        "isPinned".to_string(),
        coalesce(&inc("isPinned")?, &Value::Bool(false)).clone(),
    );
    m.insert(
        "spaceId".to_string(),
        coalesce(&inc("spaceId")?, &Value::Null).clone(),
    );
    m.insert("createdAt".to_string(), inc("createdAt")?);
    m.insert("updatedAt".to_string(), inc("updatedAt")?);
    m.insert("scripts".to_string(), scripts);
    m.insert("additionalFolders".to_string(), additional_folders);
    Ok(Value::Object(m))
}

/// `normalizeSpace(incoming, previous)`.
pub(crate) fn normalize_space(input: &Value, previous: &Value) -> Result<Value, String> {
    let inc = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(input, k, &format!("incoming.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    if jsv::truthy(previous)
        && same(&jsv::opt(previous, "id"), &inc("id")?)
        && same(&jsv::opt(previous, "name"), &inc("name")?)
        && same(&jsv::opt(previous, "icon"), &inc("icon")?)
        && same(&jsv::opt(previous, "sortOrder"), &inc("sortOrder")?)
        && same(&jsv::opt(previous, "createdAt"), &inc("createdAt")?)
        && same(&jsv::opt(previous, "updatedAt"), &inc("updatedAt")?)
    {
        return Ok(previous.clone());
    }
    let mut m = Map::new();
    m.insert("id".to_string(), inc("id")?);
    m.insert("name".to_string(), inc("name")?);
    m.insert("icon".to_string(), inc("icon")?);
    m.insert("sortOrder".to_string(), inc("sortOrder")?);
    m.insert("createdAt".to_string(), inc("createdAt")?);
    m.insert("updatedAt".to_string(), inc("updatedAt")?);
    Ok(Value::Object(m))
}

/// `resolveThreadSidebarMetadata(thread)` — sidebar-only summary fields derived from detail.
pub(crate) fn resolve_thread_sidebar_metadata(thread: &Value) -> Result<Value, String> {
    let t = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(thread, k, &format!("thread.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let latest_user = t("latestUserMessageAt")?;
    let latest_human = t("latestHumanMessageAt")?;
    let pending_approvals = t("hasPendingApprovals")?;
    let pending_user_input = t("hasPendingUserInput")?;
    let actionable_plan = t("hasActionableProposedPlan")?;
    let needs_derived = is_undef(&latest_user)
        || is_undef(&latest_human)
        || is_undef(&pending_approvals)
        || is_undef(&pending_user_input)
        || is_undef(&actionable_plan);
    let messages = t("messages")?;
    let activities = t("activities")?;
    let proposed_plans = t("proposedPlans")?;
    let latest_turn = t("latestTurn")?;
    let session = t("session")?;
    let derived = if needs_derived {
        crate::shared::derive_thread_summary_metadata(&json!({
            "messages": messages,
            "activities": activities,
            "proposedPlans": proposed_plans,
            "latestTurn": latest_turn,
        }))?
    } else {
        Value::Null
    };
    let mut m = Map::new();
    m.insert(
        "latestUserMessageAt".to_string(),
        coalesce(
            &latest_user,
            coalesce(&jsv::opt(&derived, "latestUserMessageAt"), &Value::Null),
        )
        .clone(),
    );
    m.insert(
        "latestHumanMessageAt".to_string(),
        if !is_undef(&latest_human) {
            latest_human
        } else {
            coalesce(&jsv::opt(&derived, "latestHumanMessageAt"), &Value::Null).clone()
        },
    );
    m.insert(
        "hasPendingApprovals".to_string(),
        coalesce(
            &pending_approvals,
            coalesce(
                &jsv::opt(&derived, "hasPendingApprovals"),
                &Value::Bool(false),
            ),
        )
        .clone(),
    );
    m.insert(
        "hasPendingUserInput".to_string(),
        coalesce(
            &pending_user_input,
            coalesce(
                &jsv::opt(&derived, "hasPendingUserInput"),
                &Value::Bool(false),
            ),
        )
        .clone(),
    );
    m.insert(
        "hasActionableProposedPlan".to_string(),
        coalesce(
            &actionable_plan,
            coalesce(
                &jsv::opt(&derived, "hasActionableProposedPlan"),
                &Value::Bool(false),
            ),
        )
        .clone(),
    );
    // `hasLiveTurnTailWork({latestTurn, messages, activities, session})` — shared takes the
    // same single input object as the TS signature.
    let has_live_tail = crate::shared::has_live_turn_tail_work(&json!({
        "latestTurn": latest_turn,
        "messages": messages,
        "activities": activities,
        "session": session,
    }))?;
    m.insert("hasLiveTailWork".to_string(), Value::Bool(has_live_tail));
    let pending_background_task_count = t("pendingBackgroundTaskCount")?;
    let count = if jsv::is_nullish(&pending_background_task_count) {
        let pending = crate::shared::derive_pending_background_work(&json!({
            "activities": activities,
            "latestTurn": latest_turn,
            "session": session,
        }))?;
        let zero = crate::js::number(0.0);
        let pending_count_v = jsv::opt(&pending, "count");
        let pending_count = coalesce(&pending_count_v, &zero);
        let outstanding = crate::shared::count_outstanding_background_work(&json!({
            "activities": activities,
            "session": session,
        }))?;
        crate::js::number(jsv::js_max(to_num(pending_count), to_num(&outstanding)))
    } else {
        pending_background_task_count
    };
    m.insert("pendingBackgroundWorkCount".to_string(), count);
    Ok(Value::Object(m))
}

// ---- thread derivation (`threadDerivation.ts`) --------------------------------------------------

/// `collectByIds` — the WeakMap cache only reuses identity; the computed list is the same.
fn collect_by_ids(ids: &Value, by_id: &Value) -> Result<Value, String> {
    let empty = Value::Array(Vec::new());
    if !jsv::truthy(ids) || js_length(ids) == Some(0.0) || !jsv::truthy(by_id) {
        return Ok(empty);
    }
    let items = array_recv(ids, "ids.forEach", "ids.forEach((id) => ...)")?;
    let mut out = Vec::new();
    for id in items {
        let value = opt_index_v(by_id, &record_key(id));
        if jsv::truthy(&value) {
            out.push(value);
        }
    }
    Ok(Value::Array(out))
}

fn opt_record<'a>(state: &'a Value, field: &str) -> Result<Option<&'a Value>, String> {
    jsv::member(state, field, &format!("state.{field}"))
}

fn record_lookup(record: Option<&Value>, key: &str) -> Value {
    record
        .and_then(|r| opt_index(r, key))
        .cloned()
        .unwrap_or_else(undef)
}

/// `getThreadFromState` — rebuilds the `Thread` for `threadId` from shell/detail slices, or
/// `None` when no shell is stored.
pub(crate) fn get_thread_from_state(
    state: &Value,
    thread_id: &Value,
) -> Result<Option<Value>, String> {
    let tid = record_key(thread_id);
    let shell = record_lookup(opt_record(state, "threadShellById")?, &tid);
    if !jsv::truthy(&shell) {
        return Ok(None);
    }
    let session = {
        let v = record_lookup(opt_record(state, "threadSessionById")?, &tid);
        if jsv::is_nullish(&v) { Value::Null } else { v }
    };
    let turn_state = record_lookup(opt_record(state, "threadTurnStateById")?, &tid);
    let message_ids = record_lookup(opt_record(state, "messageIdsByThreadId")?, &tid);
    let message_map = record_lookup(opt_record(state, "messageByThreadId")?, &tid);
    let activity_ids = record_lookup(opt_record(state, "activityIdsByThreadId")?, &tid);
    let activity_map = record_lookup(opt_record(state, "activityByThreadId")?, &tid);
    let plan_ids = record_lookup(opt_record(state, "proposedPlanIdsByThreadId")?, &tid);
    let plan_map = record_lookup(opt_record(state, "proposedPlanByThreadId")?, &tid);
    let diff_ids = record_lookup(opt_record(state, "turnDiffIdsByThreadId")?, &tid);
    let diff_map = record_lookup(opt_record(state, "turnDiffSummaryByThreadId")?, &tid);
    let messages = collect_by_ids(&message_ids, &message_map)?;
    let activities = collect_by_ids(&activity_ids, &activity_map)?;
    let proposed_plans = collect_by_ids(&plan_ids, &plan_map)?;
    let turn_diff_summaries = collect_by_ids(&diff_ids, &diff_map)?;

    let mut thread = spread_any(&shell);
    thread.insert("session".to_string(), session);
    thread.insert(
        "latestTurn".to_string(),
        coalesce(&jsv::opt(&turn_state, "latestTurn"), &Value::Null).clone(),
    );
    // `pendingSourceProposedPlan` is always present on a derived Thread (possibly `undefined`).
    thread.insert(
        "pendingSourceProposedPlan".to_string(),
        jsv::opt(&turn_state, "pendingSourceProposedPlan"),
    );
    let pts = jsv::opt(&turn_state, "pendingTurnStartMessageId");
    set_if_defined(&mut thread, "pendingTurnStartMessageId", &pts);
    thread.insert("messages".to_string(), messages);
    thread.insert("activities".to_string(), activities);
    thread.insert("proposedPlans".to_string(), proposed_plans);
    thread.insert("turnDiffSummaries".to_string(), turn_diff_summaries);
    Ok(Some(Value::Object(thread)))
}

/// `getThreadsFromState`.
#[allow(dead_code)] // spec'd export; the snapshot-sync callers live in the reducer port
pub(crate) fn get_threads_from_state(state: &Value) -> Result<Vec<Value>, String> {
    let thread_ids = coalesce(
        &jsv::member(state, "threadIds", "state.threadIds")?
            .cloned()
            .unwrap_or_else(undef),
        &Value::Array(Vec::new()),
    )
    .clone();
    let ids = array_recv(
        &thread_ids,
        "threadIds.flatMap",
        "threadIds.flatMap((threadId) => ...)",
    )?;
    let mut out = Vec::new();
    for id in ids {
        if let Some(thread) = get_thread_from_state(state, id)? {
            out.push(thread);
        }
    }
    Ok(out)
}

// ---- shell/turn-state projection (`storeProjection.ts`) ----------------------------------------

/// `toThreadShell`.
fn to_thread_shell(thread: &Value) -> Result<Value, String> {
    let t = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(thread, k, &format!("thread.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let null = Value::Null;
    let mut m = Map::new();
    m.insert("id".to_string(), t("id")?);
    m.insert("codexThreadId".to_string(), t("codexThreadId")?);
    m.insert("projectId".to_string(), t("projectId")?);
    m.insert("title".to_string(), t("title")?);
    m.insert("modelSelection".to_string(), t("modelSelection")?);
    m.insert("runtimeMode".to_string(), t("runtimeMode")?);
    m.insert("interactionMode".to_string(), t("interactionMode")?);
    m.insert("error".to_string(), t("error")?);
    m.insert("createdAt".to_string(), t("createdAt")?);
    m.insert(
        "archivedAt".to_string(),
        coalesce(&t("archivedAt")?, &null).clone(),
    );
    m.insert(
        "settledAt".to_string(),
        coalesce(&t("settledAt")?, &null).clone(),
    );
    m.insert(
        "snoozedUntil".to_string(),
        coalesce(&t("snoozedUntil")?, &null).clone(),
    );
    m.insert(
        "snoozeReminderAt".to_string(),
        coalesce(&t("snoozeReminderAt")?, &null).clone(),
    );
    m.insert("updatedAt".to_string(), t("updatedAt")?);
    m.insert(
        "isPinned".to_string(),
        coalesce(&t("isPinned")?, &Value::Bool(false)).clone(),
    );
    m.insert("envMode".to_string(), t("envMode")?);
    m.insert("branch".to_string(), t("branch")?);
    m.insert("worktreePath".to_string(), t("worktreePath")?);
    m.insert(
        "workingDirectory".to_string(),
        coalesce(&t("workingDirectory")?, &null).clone(),
    );
    m.insert(
        "associatedWorktreePath".to_string(),
        coalesce(&t("associatedWorktreePath")?, &null).clone(),
    );
    m.insert(
        "associatedWorktreeBranch".to_string(),
        coalesce(&t("associatedWorktreeBranch")?, &null).clone(),
    );
    m.insert(
        "associatedWorktreeRef".to_string(),
        coalesce(&t("associatedWorktreeRef")?, &null).clone(),
    );
    m.insert(
        "createBranchFlowCompleted".to_string(),
        coalesce(&t("createBranchFlowCompleted")?, &Value::Bool(false)).clone(),
    );
    m.insert(
        "parentThreadId".to_string(),
        coalesce(&t("parentThreadId")?, &null).clone(),
    );
    m.insert(
        "creationSource".to_string(),
        coalesce(&t("creationSource")?, &null).clone(),
    );
    m.insert(
        "sourceThreadId".to_string(),
        coalesce(&t("sourceThreadId")?, &null).clone(),
    );
    m.insert(
        "subagentAgentId".to_string(),
        coalesce(&t("subagentAgentId")?, &null).clone(),
    );
    m.insert(
        "subagentNickname".to_string(),
        coalesce(&t("subagentNickname")?, &null).clone(),
    );
    m.insert(
        "subagentRole".to_string(),
        coalesce(&t("subagentRole")?, &null).clone(),
    );
    m.insert(
        "forkSourceThreadId".to_string(),
        coalesce(&t("forkSourceThreadId")?, &null).clone(),
    );
    m.insert(
        "sidechatSourceThreadId".to_string(),
        coalesce(&t("sidechatSourceThreadId")?, &null).clone(),
    );
    m.insert(
        "sidechatContext".to_string(),
        coalesce(&t("sidechatContext")?, &null).clone(),
    );
    m.insert(
        "sidechatLastActivityAt".to_string(),
        coalesce(&t("sidechatLastActivityAt")?, &null).clone(),
    );
    m.insert(
        "sidechatExpiredAt".to_string(),
        coalesce(&t("sidechatExpiredAt")?, &null).clone(),
    );
    m.insert(
        "lastKnownPr".to_string(),
        coalesce(&t("lastKnownPr")?, &null).clone(),
    );
    m.insert(
        "pullRequests".to_string(),
        coalesce(&t("pullRequests")?, &Value::Array(Vec::new())).clone(),
    );
    m.insert(
        "handoff".to_string(),
        coalesce(&t("handoff")?, &null).clone(),
    );
    if jsv::truthy(&t("isProjectImport")?) {
        m.insert("isProjectImport".to_string(), Value::Bool(true));
    }
    m.insert(
        "usageLimitRecovery".to_string(),
        coalesce(&t("usageLimitRecovery")?, &null).clone(),
    );
    set_if_defined(
        &mut m,
        "usageLimitRecoverySequence",
        &t("usageLimitRecoverySequence")?,
    );
    m.insert(
        "claudeCacheReview".to_string(),
        coalesce(&t("claudeCacheReview")?, &null).clone(),
    );
    set_if_defined(&mut m, "snoozeSequence", &t("snoozeSequence")?);
    set_if_defined(
        &mut m,
        "claudeCacheReviewSequence",
        &t("claudeCacheReviewSequence")?,
    );
    set_if_defined(&mut m, "pinnedMessages", &t("pinnedMessages")?);
    set_if_defined(&mut m, "notes", &t("notes")?);
    set_if_defined(&mut m, "goal", &t("goal")?);
    set_if_defined(&mut m, "goalStartedAt", &t("goalStartedAt")?);
    set_if_defined(&mut m, "goalPausedAt", &t("goalPausedAt")?);
    set_if_defined(&mut m, "goalAchievements", &t("goalAchievements")?);
    set_if_defined(&mut m, "latestHumanMessageAt", &t("latestHumanMessageAt")?);
    set_if_defined(&mut m, "latestUserMessageAt", &t("latestUserMessageAt")?);
    set_if_defined(&mut m, "hasPendingApprovals", &t("hasPendingApprovals")?);
    set_if_defined(&mut m, "hasPendingUserInput", &t("hasPendingUserInput")?);
    set_if_defined(
        &mut m,
        "hasActionableProposedPlan",
        &t("hasActionableProposedPlan")?,
    );
    set_if_defined(
        &mut m,
        "pendingBackgroundTaskCount",
        &t("pendingBackgroundTaskCount")?,
    );
    set_if_defined(&mut m, "latestReplyPreview", &t("latestReplyPreview")?);
    set_if_defined(&mut m, "pendingInteractions", &t("pendingInteractions")?);
    set_if_defined(&mut m, "lastVisitedAt", &t("lastVisitedAt")?);
    Ok(Value::Object(m))
}

/// `toThreadTurnState`.
fn to_thread_turn_state(thread: &Value) -> Result<Value, String> {
    let mut m = Map::new();
    m.insert(
        "latestTurn".to_string(),
        jsv::member(thread, "latestTurn", "thread.latestTurn")?
            .cloned()
            .unwrap_or_else(undef),
    );
    let psp = jsv::member(
        thread,
        "pendingSourceProposedPlan",
        "thread.pendingSourceProposedPlan",
    )?
    .cloned()
    .unwrap_or_else(undef);
    set_if_truthy(&mut m, "pendingSourceProposedPlan", &psp);
    let pts = jsv::member(
        thread,
        "pendingTurnStartMessageId",
        "thread.pendingTurnStartMessageId",
    )?
    .cloned()
    .unwrap_or_else(undef);
    set_if_defined(&mut m, "pendingTurnStartMessageId", &pts);
    Ok(Value::Object(m))
}

/// `buildNormalizedSlice` — the `{ids, byId}` pair for one thread-scoped detail slice.
/// `id_key` is `"id"` for messages/activities/plans and `"turnId"` for turn-diff summaries.
fn build_normalized_slice(
    items: &Value,
    id_key: &str,
    previous_items: &Value,
    previous_ids: &Value,
    previous_by_id: &Value,
) -> Result<(Value, Value), String> {
    let items_len = member_length(items, "items.length")?;
    let prev_items_len = js_length(previous_items);
    let prev_ids_len = js_length(previous_ids);
    let mut reusable_ids: Option<Value> = None;
    if !is_undef(previous_items)
        && !is_undef(previous_ids)
        && !is_undef(previous_by_id)
        && prev_items_len == items_len
        && prev_ids_len == items_len
    {
        let n = items_len.unwrap_or(0.0) as usize;
        let mut first_changed: Option<usize> = None;
        for index in 0..n {
            if !same(&item_at(previous_items, index), &item_at(items, index)) {
                first_changed = Some(index);
                break;
            }
        }
        match first_changed {
            None => return Ok((previous_ids.clone(), previous_by_id.clone())),
            Some(first) => {
                reusable_ids = Some(previous_ids.clone());
                for index in first..n {
                    let id = jsv::member(&item_at(items, index), id_key, "getId(items[index])")?
                        .cloned()
                        .unwrap_or_else(undef);
                    if !same(&item_at(previous_ids, index), &id) {
                        reusable_ids = None;
                        break;
                    }
                }
            }
        }
    }
    let mut by_id = Map::new();
    if let Some(ids) = reusable_ids {
        for index in 0..(items_len.unwrap_or(0.0) as usize) {
            let item = item_at(items, index);
            let id = jsv::member(&item, id_key, "getId(item)")?
                .cloned()
                .unwrap_or_else(undef);
            by_id.insert(record_key(&id), item);
        }
        return Ok((ids, Value::Object(by_id)));
    }
    let Some(n) = items_len else {
        // `new Array(items.length)` with an undefined length is a RangeError in JS.
        return Err("Array size is not a small positive integer".to_string());
    };
    let mut ids = Vec::with_capacity(n as usize);
    for index in 0..(n as usize) {
        let item = item_at(items, index);
        let id = jsv::member(&item, id_key, "getId(item)")?
            .cloned()
            .unwrap_or_else(undef);
        by_id.insert(record_key(&id), item);
        ids.push(id);
    }
    Ok((Value::Array(ids), Value::Object(by_id)))
}

/// `sourceProposedPlansEqual`.
fn source_proposed_plans_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if same(left, right) {
        return Ok(true);
    }
    if is_undef(left) || is_undef(right) {
        return Ok(false);
    }
    Ok(same(
        &jsv::member(left, "threadId", "left.threadId")?
            .cloned()
            .unwrap_or_else(undef),
        &jsv::member(right, "threadId", "right.threadId")?
            .cloned()
            .unwrap_or_else(undef),
    ) && same(
        &jsv::member(left, "planId", "left.planId")?
            .cloned()
            .unwrap_or_else(undef),
        &jsv::member(right, "planId", "right.planId")?
            .cloned()
            .unwrap_or_else(undef),
    ))
}

/// `latestTurnsEqual`.
fn latest_turns_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if same(left, right) {
        return Ok(true);
    }
    if jsv::is_nullish(left) || jsv::is_nullish(right) {
        return Ok(false);
    }
    for key in [
        "turnId",
        "state",
        "requestedAt",
        "startedAt",
        "completedAt",
        "assistantMessageId",
    ] {
        if !same(
            &jsv::member(left, key, &format!("left.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
            &jsv::member(right, key, &format!("right.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
        ) {
            return Ok(false);
        }
    }
    source_proposed_plans_equal(
        &jsv::member(left, "sourceProposedPlan", "left.sourceProposedPlan")?
            .cloned()
            .unwrap_or_else(undef),
        &jsv::member(right, "sourceProposedPlan", "right.sourceProposedPlan")?
            .cloned()
            .unwrap_or_else(undef),
    )
}

/// `threadSessionsEqual`.
fn thread_sessions_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if same(left, right) {
        return Ok(true);
    }
    if jsv::is_nullish(left) || jsv::is_nullish(right) {
        return Ok(false);
    }
    for key in [
        "provider",
        "status",
        "orchestrationStatus",
        "runtimeMode",
        "activeTurnId",
        "createdAt",
        "updatedAt",
        "lastError",
    ] {
        if !same(
            &jsv::member(left, key, &format!("left.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
            &jsv::member(right, key, &format!("right.{key}"))?
                .cloned()
                .unwrap_or_else(undef),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

/// `threadTurnStatesEqual`.
fn thread_turn_states_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if is_undef(left) {
        return Ok(false);
    }
    Ok(latest_turns_equal(
        &jsv::member(left, "latestTurn", "left.latestTurn")?
            .cloned()
            .unwrap_or_else(undef),
        &jsv::member(right, "latestTurn", "right.latestTurn")?
            .cloned()
            .unwrap_or_else(undef),
    )? && source_proposed_plans_equal(
        &jsv::member(
            left,
            "pendingSourceProposedPlan",
            "left.pendingSourceProposedPlan",
        )?
        .cloned()
        .unwrap_or_else(undef),
        &jsv::member(
            right,
            "pendingSourceProposedPlan",
            "right.pendingSourceProposedPlan",
        )?
        .cloned()
        .unwrap_or_else(undef),
    )? && same(
        &jsv::member(
            left,
            "pendingTurnStartMessageId",
            "left.pendingTurnStartMessageId",
        )?
        .cloned()
        .unwrap_or_else(undef),
        &jsv::member(
            right,
            "pendingTurnStartMessageId",
            "right.pendingTurnStartMessageId",
        )?
        .cloned()
        .unwrap_or_else(undef),
    ))
}

/// `threadShellsEqual(left, right)` — `left` may be `undefined`.
fn thread_shells_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if is_undef(left) {
        return Ok(false);
    }
    let l = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(left, k, &format!("left.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let r = |k: &str| jsv::opt(right, k);
    let null = Value::Null;
    let eq = |la: Result<Value, String>, rb: Value| -> Result<bool, String> { Ok(same(&la?, &rb)) };
    let eq_null = |la: Result<Value, String>, rb: Value| -> Result<bool, String> {
        Ok(same(coalesce(&la?, &null), coalesce(&rb, &null)))
    };
    if !(eq(l("id"), r("id"))?
        && eq(l("codexThreadId"), r("codexThreadId"))?
        && eq(l("projectId"), r("projectId"))?
        && eq(l("title"), r("title"))?
        && eq(l("modelSelection"), r("modelSelection"))?
        && eq(l("runtimeMode"), r("runtimeMode"))?
        && eq(l("interactionMode"), r("interactionMode"))?
        && eq(l("error"), r("error"))?
        && eq(l("createdAt"), r("createdAt"))?
        && eq_null(l("archivedAt"), r("archivedAt"))?
        && eq_null(l("settledAt"), r("settledAt"))?
        && eq_null(l("snoozedUntil"), r("snoozedUntil"))?
        && eq_null(l("snoozeReminderAt"), r("snoozeReminderAt"))?
        && eq(l("updatedAt"), r("updatedAt"))?
        && same(
            coalesce(&l("isPinned")?, &Value::Bool(false)),
            coalesce(&r("isPinned"), &Value::Bool(false)),
        )
        && eq(l("envMode"), r("envMode"))?
        && eq(l("branch"), r("branch"))?
        && eq(l("worktreePath"), r("worktreePath"))?
        && eq_null(l("workingDirectory"), r("workingDirectory"))?
        && eq_null(l("associatedWorktreePath"), r("associatedWorktreePath"))?
        && eq_null(l("associatedWorktreeBranch"), r("associatedWorktreeBranch"))?
        && eq_null(l("associatedWorktreeRef"), r("associatedWorktreeRef"))?
        && same(
            coalesce(&l("createBranchFlowCompleted")?, &Value::Bool(false)),
            coalesce(&r("createBranchFlowCompleted"), &Value::Bool(false)),
        )
        && eq_null(l("parentThreadId"), r("parentThreadId"))?
        && eq_null(l("creationSource"), r("creationSource"))?
        && eq_null(l("sourceThreadId"), r("sourceThreadId"))?
        && eq_null(l("subagentAgentId"), r("subagentAgentId"))?
        && eq_null(l("subagentNickname"), r("subagentNickname"))?
        && eq_null(l("subagentRole"), r("subagentRole"))?
        && eq_null(l("forkSourceThreadId"), r("forkSourceThreadId"))?
        && eq_null(l("sidechatSourceThreadId"), r("sidechatSourceThreadId"))?)
    {
        return Ok(false);
    }
    // The context never changes after creation; its URL identifies the item.
    let left_url = coalesce(&jsv::opt(&jsv::opt(left, "sidechatContext"), "url"), &null).clone();
    let right_url = coalesce(&jsv::opt(&jsv::opt(right, "sidechatContext"), "url"), &null).clone();
    Ok(same(&left_url, &right_url)
        && eq_null(l("sidechatLastActivityAt"), r("sidechatLastActivityAt"))?
        && eq_null(l("sidechatExpiredAt"), r("sidechatExpiredAt"))?
        && deep_equal_json(
            coalesce(&jsv::opt(left, "lastKnownPr"), &null),
            coalesce(&r("lastKnownPr"), &null),
        )
        && deep_equal_json(
            coalesce(&jsv::opt(left, "pullRequests"), &Value::Array(Vec::new())),
            coalesce(&r("pullRequests"), &Value::Array(Vec::new())),
        )
        && eq_null(l("handoff"), r("handoff"))?
        && same(
            coalesce(&l("isProjectImport")?, &Value::Bool(false)),
            coalesce(&r("isProjectImport"), &Value::Bool(false)),
        )
        && deep_equal_json(
            coalesce(&jsv::opt(left, "claudeCacheReview"), &null),
            coalesce(&r("claudeCacheReview"), &null),
        )
        && eq(
            l("claudeCacheReviewSequence"),
            r("claudeCacheReviewSequence"),
        )?
        && deep_equal_json(
            coalesce(&jsv::opt(left, "usageLimitRecovery"), &null),
            coalesce(&r("usageLimitRecovery"), &null),
        )
        && eq(
            l("usageLimitRecoverySequence"),
            r("usageLimitRecoverySequence"),
        )?
        && eq(l("snoozeSequence"), r("snoozeSequence"))?
        && deep_equal_json(
            coalesce(&jsv::opt(left, "pinnedMessages"), &null),
            coalesce(&r("pinnedMessages"), &null),
        )
        && same(
            coalesce(&jsv::opt(left, "notes"), &Value::String(String::new())),
            coalesce(&r("notes"), &Value::String(String::new())),
        )
        && same(
            coalesce(&jsv::opt(left, "goal"), &Value::String(String::new())),
            coalesce(&r("goal"), &Value::String(String::new())),
        )
        && eq_null(l("goalStartedAt"), r("goalStartedAt"))?
        && eq_null(l("goalPausedAt"), r("goalPausedAt"))?
        && deep_equal_json(
            coalesce(&jsv::opt(left, "goalAchievements"), &null),
            coalesce(&r("goalAchievements"), &null),
        )
        && eq(l("latestUserMessageAt"), r("latestUserMessageAt"))?
        && eq(l("latestHumanMessageAt"), r("latestHumanMessageAt"))?
        && eq(l("hasPendingApprovals"), r("hasPendingApprovals"))?
        && eq(l("hasPendingUserInput"), r("hasPendingUserInput"))?
        && eq(
            l("hasActionableProposedPlan"),
            r("hasActionableProposedPlan"),
        )?
        && eq(
            l("pendingBackgroundTaskCount"),
            r("pendingBackgroundTaskCount"),
        )?
        && eq(l("latestReplyPreview"), r("latestReplyPreview"))?
        && eq(l("pendingInteractions"), r("pendingInteractions"))?
        && eq(l("lastVisitedAt"), r("lastVisitedAt"))?)
}

/// `sidebarThreadSummariesEqual`.
fn sidebar_thread_summaries_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if is_undef(left) {
        return Ok(false);
    }
    let l = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(left, k, &format!("left.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let r = |k: &str| jsv::opt(right, k);
    let null = Value::Null;
    for pair in [
        ("id", "id"),
        ("projectId", "projectId"),
        ("title", "title"),
        ("modelSelection", "modelSelection"),
        ("interactionMode", "interactionMode"),
        ("envMode", "envMode"),
        ("branch", "branch"),
        ("worktreePath", "worktreePath"),
        ("session", "session"),
        ("createdAt", "createdAt"),
        ("updatedAt", "updatedAt"),
        ("latestTurn", "latestTurn"),
        ("lastVisitedAt", "lastVisitedAt"),
        ("latestUserMessageAt", "latestUserMessageAt"),
        ("latestHumanMessageAt", "latestHumanMessageAt"),
        ("hasPendingApprovals", "hasPendingApprovals"),
        ("hasPendingUserInput", "hasPendingUserInput"),
        ("hasActionableProposedPlan", "hasActionableProposedPlan"),
        ("hasLiveTailWork", "hasLiveTailWork"),
        ("pendingBackgroundWorkCount", "pendingBackgroundWorkCount"),
        ("latestReplyPreview", "latestReplyPreview"),
    ] {
        if !same(&l(pair.0)?, &r(pair.1)) {
            return Ok(false);
        }
    }
    for key in [
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
        "archivedAt",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
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
        "handoff",
    ] {
        if !same(coalesce(&l(key)?, &null), coalesce(&r(key), &null)) {
            return Ok(false);
        }
    }
    if !same(
        coalesce(&l("isPinned")?, &Value::Bool(false)),
        coalesce(&r("isPinned"), &Value::Bool(false)),
    ) {
        return Ok(false);
    }
    let left_url = coalesce(&jsv::opt(&jsv::opt(left, "sidechatContext"), "url"), &null).clone();
    let right_url = coalesce(&jsv::opt(&jsv::opt(right, "sidechatContext"), "url"), &null).clone();
    Ok(same(&left_url, &right_url)
        && deep_equal_json(
            coalesce(&jsv::opt(left, "lastKnownPr"), &null),
            coalesce(&r("lastKnownPr"), &null),
        )
        && deep_equal_json(
            coalesce(&jsv::opt(left, "pullRequests"), &Value::Array(Vec::new())),
            coalesce(&r("pullRequests"), &Value::Array(Vec::new())),
        ))
}

/// `buildSidebarThreadSummary`.
fn build_sidebar_thread_summary(thread: &Value, previous: &Value) -> Result<Value, String> {
    let metadata = resolve_thread_sidebar_metadata(thread)?;
    let t = |k: &str| -> Result<Value, String> {
        Ok(jsv::member(thread, k, &format!("thread.{k}"))?
            .cloned()
            .unwrap_or_else(undef))
    };
    let null = Value::Null;
    let mut m = Map::new();
    m.insert("id".to_string(), t("id")?);
    m.insert("projectId".to_string(), t("projectId")?);
    m.insert("title".to_string(), t("title")?);
    m.insert("modelSelection".to_string(), t("modelSelection")?);
    m.insert("interactionMode".to_string(), t("interactionMode")?);
    m.insert("envMode".to_string(), t("envMode")?);
    m.insert("branch".to_string(), t("branch")?);
    m.insert("worktreePath".to_string(), t("worktreePath")?);
    m.insert(
        "workingDirectory".to_string(),
        coalesce(&t("workingDirectory")?, &null).clone(),
    );
    m.insert(
        "associatedWorktreePath".to_string(),
        coalesce(&t("associatedWorktreePath")?, &null).clone(),
    );
    m.insert(
        "associatedWorktreeBranch".to_string(),
        coalesce(&t("associatedWorktreeBranch")?, &null).clone(),
    );
    m.insert(
        "associatedWorktreeRef".to_string(),
        coalesce(&t("associatedWorktreeRef")?, &null).clone(),
    );
    m.insert("session".to_string(), t("session")?);
    m.insert("createdAt".to_string(), t("createdAt")?);
    m.insert(
        "archivedAt".to_string(),
        coalesce(&t("archivedAt")?, &null).clone(),
    );
    m.insert(
        "settledAt".to_string(),
        coalesce(&t("settledAt")?, &null).clone(),
    );
    m.insert(
        "snoozedUntil".to_string(),
        coalesce(&t("snoozedUntil")?, &null).clone(),
    );
    m.insert(
        "snoozeReminderAt".to_string(),
        coalesce(&t("snoozeReminderAt")?, &null).clone(),
    );
    m.insert("updatedAt".to_string(), t("updatedAt")?);
    m.insert(
        "isPinned".to_string(),
        coalesce(&t("isPinned")?, &Value::Bool(false)).clone(),
    );
    m.insert("latestTurn".to_string(), t("latestTurn")?);
    m.insert("lastVisitedAt".to_string(), t("lastVisitedAt")?);
    m.insert(
        "parentThreadId".to_string(),
        coalesce(&t("parentThreadId")?, &null).clone(),
    );
    m.insert(
        "creationSource".to_string(),
        coalesce(&t("creationSource")?, &null).clone(),
    );
    m.insert(
        "sourceThreadId".to_string(),
        coalesce(&t("sourceThreadId")?, &null).clone(),
    );
    m.insert(
        "subagentAgentId".to_string(),
        coalesce(&t("subagentAgentId")?, &null).clone(),
    );
    m.insert(
        "subagentNickname".to_string(),
        coalesce(&t("subagentNickname")?, &null).clone(),
    );
    m.insert(
        "subagentRole".to_string(),
        coalesce(&t("subagentRole")?, &null).clone(),
    );
    for (key, field) in [
        ("latestUserMessageAt", "latestUserMessageAt"),
        ("latestHumanMessageAt", "latestHumanMessageAt"),
        ("hasPendingApprovals", "hasPendingApprovals"),
        ("hasPendingUserInput", "hasPendingUserInput"),
        ("hasActionableProposedPlan", "hasActionableProposedPlan"),
        ("hasLiveTailWork", "hasLiveTailWork"),
        ("pendingBackgroundWorkCount", "pendingBackgroundWorkCount"),
    ] {
        m.insert(key.to_string(), jsv::opt(&metadata, field));
    }
    m.insert(
        "forkSourceThreadId".to_string(),
        coalesce(&t("forkSourceThreadId")?, &null).clone(),
    );
    m.insert(
        "sidechatSourceThreadId".to_string(),
        coalesce(&t("sidechatSourceThreadId")?, &null).clone(),
    );
    m.insert(
        "sidechatContext".to_string(),
        coalesce(&t("sidechatContext")?, &null).clone(),
    );
    m.insert(
        "sidechatLastActivityAt".to_string(),
        coalesce(&t("sidechatLastActivityAt")?, &null).clone(),
    );
    m.insert(
        "sidechatExpiredAt".to_string(),
        coalesce(&t("sidechatExpiredAt")?, &null).clone(),
    );
    m.insert(
        "lastKnownPr".to_string(),
        coalesce(&t("lastKnownPr")?, &null).clone(),
    );
    m.insert(
        "pullRequests".to_string(),
        coalesce(&t("pullRequests")?, &Value::Array(Vec::new())).clone(),
    );
    m.insert(
        "handoff".to_string(),
        coalesce(&t("handoff")?, &null).clone(),
    );
    set_if_defined(&mut m, "latestReplyPreview", &t("latestReplyPreview")?);
    let next_summary = Value::Object(m);
    Ok(
        if jsv::truthy(previous) && sidebar_thread_summaries_equal(previous, &next_summary)? {
            previous.clone()
        } else {
            next_summary
        },
    )
}

/// `ensureThreadRegistered`.
fn ensure_thread_registered(state: Value, thread_id: &Value) -> Result<Value, String> {
    let empty = Value::Array(Vec::new());
    let thread_ids = coalesce(
        &jsv::member(&state, "threadIds", "state.threadIds")?
            .cloned()
            .unwrap_or_else(undef),
        &empty,
    )
    .clone();
    if let Value::Array(ids) = &thread_ids {
        if ids.iter().any(|id| same(id, thread_id)) {
            return Ok(state);
        }
        let mut next_ids = ids.clone();
        next_ids.push(thread_id.clone());
        return Ok(set_field(&state, "threadIds", Value::Array(next_ids)));
    }
    // `threadIds.includes` on a non-array receiver.
    Err(
        "threadIds.includes is not a function. (In 'threadIds.includes(threadId)', \
         'threadIds.includes' is undefined)"
            .to_string(),
    )
}

/// `writeThreadState` — projects an updated `Thread` back into the normalized slices.
fn write_thread_state(
    state: Value,
    next_thread: &Value,
    previous_thread: Option<&Value>,
) -> Result<Value, String> {
    let undef_v = undef();
    let prev_thread = previous_thread.unwrap_or(&undef_v);
    let next_shell = to_thread_shell(next_thread)?;
    let next_turn_state = to_thread_turn_state(next_thread)?;
    let tid_key = record_key(&jsv::opt(&next_shell, "id"));
    let shell_map = opt_record(&state, "threadShellById")?;
    let previous_shell = record_lookup(shell_map, &tid_key);
    let turn_state_map = opt_record(&state, "threadTurnStateById")?;
    let previous_turn_state = record_lookup(turn_state_map, &tid_key);
    let mut next_state = ensure_thread_registered(state, &jsv::opt(&next_shell, "id"))?;

    if !thread_shells_equal(&previous_shell, &next_shell)? {
        let map = opt_record(&next_state, "threadShellById")?
            .cloned()
            .unwrap_or_else(undef);
        next_state = set_field(
            &next_state,
            "threadShellById",
            record_set(&map, &tid_key, next_shell.clone()),
        );
    }

    let next_session = jsv::member(next_thread, "session", "thread.session")?
        .cloned()
        .unwrap_or_else(undef);
    let prev_session = coalesce(&jsv::opt(prev_thread, "session"), &Value::Null).clone();
    if !thread_sessions_equal(&prev_session, &next_session)? {
        let map = opt_record(&next_state, "threadSessionById")?
            .cloned()
            .unwrap_or_else(undef);
        next_state = set_field(
            &next_state,
            "threadSessionById",
            record_set(&map, &tid_key, next_session),
        );
    }

    if !thread_turn_states_equal(&previous_turn_state, &next_turn_state)? {
        let map = opt_record(&next_state, "threadTurnStateById")?
            .cloned()
            .unwrap_or_else(undef);
        next_state = set_field(
            &next_state,
            "threadTurnStateById",
            record_set(&map, &tid_key, next_turn_state.clone()),
        );
    }

    // Each detail slice: `{ids, byId}` rebuilt only when the array actually changed.
    let next_messages = jsv::member(next_thread, "messages", "thread.messages")?
        .cloned()
        .unwrap_or_else(undef);
    if !same(&jsv::opt(prev_thread, "messages"), &next_messages) {
        let prev_ids = record_lookup(opt_record(&next_state, "messageIdsByThreadId")?, &tid_key);
        let prev_by = record_lookup(opt_record(&next_state, "messageByThreadId")?, &tid_key);
        let (ids, by_id) = build_normalized_slice(
            &next_messages,
            "id",
            &jsv::opt(prev_thread, "messages"),
            &prev_ids,
            &prev_by,
        )?;
        if !same(&ids, &prev_ids) {
            let map = opt_record(&next_state, "messageIdsByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "messageIdsByThreadId",
                record_set(&map, &tid_key, ids),
            );
        }
        if !same(&by_id, &prev_by) {
            let map = opt_record(&next_state, "messageByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "messageByThreadId",
                record_set(&map, &tid_key, by_id),
            );
        }
    }

    let next_activities = jsv::member(next_thread, "activities", "thread.activities")?
        .cloned()
        .unwrap_or_else(undef);
    if !same(&jsv::opt(prev_thread, "activities"), &next_activities) {
        let prev_ids = record_lookup(opt_record(&next_state, "activityIdsByThreadId")?, &tid_key);
        let prev_by = record_lookup(opt_record(&next_state, "activityByThreadId")?, &tid_key);
        let capped = cap_thread_activities(&dedupe_activities_by_id_after_append(
            &next_activities,
            &jsv::opt(prev_thread, "activities"),
            &prev_by,
        )?)?;
        let (ids, by_id) = build_normalized_slice(
            &capped,
            "id",
            &jsv::opt(prev_thread, "activities"),
            &prev_ids,
            &prev_by,
        )?;
        if !same(&ids, &prev_ids) {
            let map = opt_record(&next_state, "activityIdsByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "activityIdsByThreadId",
                record_set(&map, &tid_key, ids),
            );
        }
        if !same(&by_id, &prev_by) {
            let map = opt_record(&next_state, "activityByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "activityByThreadId",
                record_set(&map, &tid_key, by_id),
            );
        }
    }

    let next_plans = jsv::member(next_thread, "proposedPlans", "thread.proposedPlans")?
        .cloned()
        .unwrap_or_else(undef);
    if !same(&jsv::opt(prev_thread, "proposedPlans"), &next_plans) {
        let prev_ids = record_lookup(
            opt_record(&next_state, "proposedPlanIdsByThreadId")?,
            &tid_key,
        );
        let prev_by = record_lookup(opt_record(&next_state, "proposedPlanByThreadId")?, &tid_key);
        let (ids, by_id) = build_normalized_slice(
            &next_plans,
            "id",
            &jsv::opt(prev_thread, "proposedPlans"),
            &prev_ids,
            &prev_by,
        )?;
        if !same(&ids, &prev_ids) {
            let map = opt_record(&next_state, "proposedPlanIdsByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "proposedPlanIdsByThreadId",
                record_set(&map, &tid_key, ids),
            );
        }
        if !same(&by_id, &prev_by) {
            let map = opt_record(&next_state, "proposedPlanByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "proposedPlanByThreadId",
                record_set(&map, &tid_key, by_id),
            );
        }
    }

    let next_diffs = jsv::member(next_thread, "turnDiffSummaries", "thread.turnDiffSummaries")?
        .cloned()
        .unwrap_or_else(undef);
    if !same(&jsv::opt(prev_thread, "turnDiffSummaries"), &next_diffs) {
        let prev_ids = record_lookup(opt_record(&next_state, "turnDiffIdsByThreadId")?, &tid_key);
        let prev_by = record_lookup(
            opt_record(&next_state, "turnDiffSummaryByThreadId")?,
            &tid_key,
        );
        let (ids, by_id) = build_normalized_slice(
            &next_diffs,
            "turnId",
            &jsv::opt(prev_thread, "turnDiffSummaries"),
            &prev_ids,
            &prev_by,
        )?;
        if !same(&ids, &prev_ids) {
            let map = opt_record(&next_state, "turnDiffIdsByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "turnDiffIdsByThreadId",
                record_set(&map, &tid_key, ids),
            );
        }
        if !same(&by_id, &prev_by) {
            let map = opt_record(&next_state, "turnDiffSummaryByThreadId")?
                .cloned()
                .unwrap_or_else(undef);
            next_state = set_field(
                &next_state,
                "turnDiffSummaryByThreadId",
                record_set(&map, &tid_key, by_id),
            );
        }
    }

    Ok(next_state)
}

/// `clearThreadDetailSyncState` — wipes the detail sync flags and the resume cursor.
fn clear_thread_detail_sync_state(state: Value, thread_id: &Value) -> Result<Value, String> {
    clear_thread_detail_resume_cursor(thread_id);
    let tid = record_key(thread_id);
    let sync_map = coalesce(
        &jsv::member(&state, "threadDetailSyncById", "state.threadDetailSyncById")?
            .cloned()
            .unwrap_or_else(undef),
        &Value::Object(Map::new()),
    )
    .clone();
    let revision_map = coalesce(
        &jsv::member(
            &state,
            "threadDetailRevisionSequenceById",
            "state.threadDetailRevisionSequenceById",
        )?
        .cloned()
        .unwrap_or_else(undef),
        &Value::Object(Map::new()),
    )
    .clone();
    if !js_has_own(&sync_map, &tid) && !js_has_own(&revision_map, &tid) {
        return Ok(state);
    }
    let mut m = spread_any(&state);
    m.insert(
        "threadDetailSyncById".to_string(),
        record_delete(&sync_map, &tid),
    );
    m.insert(
        "threadDetailRevisionSequenceById".to_string(),
        record_delete(&revision_map, &tid),
    );
    Ok(Value::Object(m))
}

/// `removeThreadState` — drops every thread-scoped slice and the sidebar row.
fn remove_thread_state(state: Value, thread_id: &Value) -> Result<Value, String> {
    let tid = record_key(thread_id);
    let fields = [
        "threadShellById",
        "threadSessionById",
        "threadTurnStateById",
        "messageIdsByThreadId",
        "messageByThreadId",
        "activityIdsByThreadId",
        "activityByThreadId",
        "proposedPlanIdsByThreadId",
        "proposedPlanByThreadId",
        "turnDiffIdsByThreadId",
        "turnDiffSummaryByThreadId",
    ];
    let thread_ids = coalesce(
        &jsv::member(&state, "threadIds", "state.threadIds")?
            .cloned()
            .unwrap_or_else(undef),
        &Value::Array(Vec::new()),
    )
    .clone();
    let next_thread_ids = match &thread_ids {
        Value::Array(ids) => Value::Array(
            ids.iter()
                .filter(|id| !same(id, thread_id))
                .cloned()
                .collect(),
        ),
        // `(state.threadIds ?? EMPTY).filter(...)` on a non-array receiver.
        _ => {
            return Err(
                "(state.threadIds ?? EMPTY_THREAD_IDS).filter is not a function. (In \
                 '(state.threadIds ?? EMPTY_THREAD_IDS).filter((id) => id !== threadId)', \
                 '(state.threadIds ?? EMPTY_THREAD_IDS).filter' is undefined)"
                    .to_string(),
            );
        }
    };
    let sidebar = jsv::member(
        &state,
        "sidebarThreadSummaryById",
        "state.sidebarThreadSummaryById",
    )?
    .cloned()
    .unwrap_or_else(undef);
    let next_sidebar = record_delete(&sidebar, &tid);

    if same(&next_thread_ids, &thread_ids) && same(&next_sidebar, &sidebar) {
        return clear_thread_detail_sync_state(state, thread_id);
    }

    let mut m = spread_any(&state);
    m.insert("threadIds".to_string(), next_thread_ids);
    for field in fields {
        let map = jsv::member(&state, field, &format!("state.{field}"))?
            .cloned()
            .unwrap_or_else(undef);
        m.insert(field.to_string(), record_delete(&map, &tid));
    }
    m.insert("sidebarThreadSummaryById".to_string(), next_sidebar);
    clear_thread_detail_sync_state(Value::Object(m), thread_id)
}

/// `nextTombstoneSequence` — the sequence at/after which a deletion is guaranteed visible to
/// server snapshots; re-deleting keeps the highest known sequence.
fn next_tombstone_sequence(
    tombstones: &Value,
    id: &Value,
    state: &Value,
    deleted_at_sequence: &Value,
) -> Result<Value, String> {
    let candidate = if jsv::is_nullish(deleted_at_sequence) {
        let shell_seq = jsv::member(
            state,
            "shellSnapshotSequence",
            "state.shellSnapshotSequence",
        )?
        .cloned()
        .unwrap_or_else(undef);
        js_add(
            coalesce(&shell_seq, &crate::js::number(0.0)),
            &crate::js::number(1.0),
        )
    } else {
        deleted_at_sequence.clone()
    };
    // `tombstones?.[id]` — optional index, so a missing tombstone map reads `undefined`.
    let existing = opt_index(tombstones, &record_key(id))
        .cloned()
        .unwrap_or_else(undef);
    if is_undef(&existing) {
        return Ok(candidate);
    }
    Ok(crate::js::number(jsv::js_max(
        to_num(&existing),
        to_num(&candidate),
    )))
}

/// `removeDeletedThreadFromClientState`.
pub(crate) fn remove_deleted_thread_from_client_state(
    state: Value,
    thread_id: &Value,
    tombstone_sequence: &Value,
) -> Result<Value, String> {
    let tombstones = jsv::member(&state, "deletedThreadIdsById", "state.deletedThreadIdsById")?
        .cloned()
        .unwrap_or_else(undef);
    let sequence = next_tombstone_sequence(&tombstones, thread_id, &state, tombstone_sequence)?;
    let tid = record_key(thread_id);
    let existing = opt_index(&tombstones, &tid).cloned().unwrap_or_else(undef);
    let deleted_thread_ids_by_id = if jsv::strict_eq(&existing, &sequence) {
        tombstones
    } else {
        record_set(&tombstones, &tid, sequence)
    };
    let next_state = remove_thread_state(state, thread_id)?;
    let next_tombstones = jsv::member(
        &next_state,
        "deletedThreadIdsById",
        "nextState.deletedThreadIdsById",
    )?
    .cloned()
    .unwrap_or_else(undef);
    if same(&next_tombstones, &deleted_thread_ids_by_id) {
        return Ok(next_state);
    }
    Ok(set_field(
        &next_state,
        "deletedThreadIdsById",
        deleted_thread_ids_by_id,
    ))
}

/// `removeProjectState` — removes a project and every thread belonging to it.
fn remove_project_state(state: Value, project_id: &Value) -> Result<Value, String> {
    let shell_map = coalesce(
        &jsv::member(&state, "threadShellById", "state.threadShellById")?
            .cloned()
            .unwrap_or_else(undef),
        &Value::Object(Map::new()),
    )
    .clone();
    // `Object.values(...)` — strings iterate by char; other primitives give `[]`.
    let mut thread_ids: Vec<Value> = Vec::new();
    for shell in match &shell_map {
        Value::Object(o) => o.values().cloned().collect::<Vec<_>>(),
        _ => spread_iterable(&shell_map, "Object.values(...)").unwrap_or_default(),
    } {
        let pid = jsv::member(&shell, "projectId", "shell.projectId")?
            .cloned()
            .unwrap_or_else(undef);
        if same(&pid, project_id) {
            let shell_id = jsv::member(&shell, "id", "shell.id")?
                .cloned()
                .unwrap_or_else(undef);
            if !thread_ids.iter().any(|t| same(t, &shell_id)) {
                thread_ids.push(shell_id);
            }
        }
    }
    let projects = jsv::member(&state, "projects", "state.projects")?
        .cloned()
        .unwrap_or_else(undef);
    let project_items = array_recv(
        &projects,
        "state.projects.find",
        "state.projects.find((project) => project.id === projectId)",
    )?;
    let mut removed_cwd: Option<Value> = None;
    for project in project_items {
        if same(
            &jsv::member(project, "id", "project.id")?
                .cloned()
                .unwrap_or_else(undef),
            project_id,
        ) {
            removed_cwd = Some(
                jsv::member(project, "cwd", "removedProject.cwd")?
                    .cloned()
                    .unwrap_or_else(undef),
            );
            break;
        }
    }
    if let Some(cwd) = removed_cwd {
        forget_project_state(&cwd)?;
    }
    let next_projects = match &projects {
        Value::Array(items) => {
            let filtered: Vec<Value> = items
                .iter()
                .filter(|p| !same(&jsv::opt(p, "id"), project_id))
                .cloned()
                .collect();
            if filtered.len() == items.len() {
                projects.clone()
            } else {
                Value::Array(filtered)
            }
        }
        _ => projects.clone(),
    };
    let mut next_state = state.clone();
    for tid in &thread_ids {
        next_state = remove_thread_state(next_state, tid)?;
    }
    if same(&next_projects, &projects) && same(&next_state, &state) {
        return Ok(state);
    }
    let next_state_projects = jsv::member(&next_state, "projects", "nextState.projects")?
        .cloned()
        .unwrap_or_else(undef);
    if same(&next_projects, &next_state_projects) {
        Ok(next_state)
    } else {
        Ok(set_field(&next_state, "projects", next_projects))
    }
}

/// `removeDeletedProjectFromClientState`.
pub(crate) fn remove_deleted_project_from_client_state(
    state: Value,
    project_id: &Value,
    tombstone_sequence: &Value,
) -> Result<Value, String> {
    let tombstones = jsv::member(
        &state,
        "deletedProjectIdsById",
        "state.deletedProjectIdsById",
    )?
    .cloned()
    .unwrap_or_else(undef);
    let sequence = next_tombstone_sequence(&tombstones, project_id, &state, tombstone_sequence)?;
    let pid = record_key(project_id);
    let existing = opt_index(&tombstones, &pid).cloned().unwrap_or_else(undef);
    let deleted_project_ids_by_id = if jsv::strict_eq(&existing, &sequence) {
        tombstones
    } else {
        record_set(&tombstones, &pid, sequence)
    };
    let next_state = remove_project_state(state, project_id)?;
    let next_tombstones = jsv::member(
        &next_state,
        "deletedProjectIdsById",
        "nextState.deletedProjectIdsById",
    )?
    .cloned()
    .unwrap_or_else(undef);
    if same(&next_tombstones, &deleted_project_ids_by_id) {
        return Ok(next_state);
    }
    Ok(set_field(
        &next_state,
        "deletedProjectIdsById",
        deleted_project_ids_by_id,
    ))
}

/// `commitThreadProjection` — refreshes `sidebarThreadSummaryById[threadId]` unless pinned off.
fn commit_thread_projection(
    state: Value,
    thread_id: &Value,
    update_sidebar_summary: &Value,
) -> Result<Value, String> {
    let should_update = if jsv::is_nullish(update_sidebar_summary) {
        true
    } else {
        jsv::truthy(update_sidebar_summary)
    };
    let tid = record_key(thread_id);
    let sidebar_map = jsv::member(
        &state,
        "sidebarThreadSummaryById",
        "state.sidebarThreadSummaryById",
    )?
    .cloned()
    .unwrap_or_else(undef);
    let previous_summary = index_access(
        &sidebar_map,
        &tid,
        "state.sidebarThreadSummaryById[threadId]",
    )?
    .cloned()
    .unwrap_or_else(undef);
    if !should_update && !is_undef(&previous_summary) {
        return Ok(state);
    }
    let Some(next_thread) = get_thread_from_state(&state, thread_id)? else {
        return Ok(state);
    };
    let next_summary = build_sidebar_thread_summary(&next_thread, &previous_summary)?;
    if same(&next_summary, &previous_summary) {
        return Ok(state);
    }
    Ok(set_field(
        &state,
        "sidebarThreadSummaryById",
        record_set(&sidebar_map, &tid, next_summary),
    ))
}

/// `deriveThreadStateSignals`.
fn derive_thread_state_signals(thread: &Value) -> Result<Value, String> {
    let metadata = crate::shared::derive_thread_summary_metadata(&json!({
        "messages": jsv::member(thread, "messages", "thread.messages")?
            .cloned()
            .unwrap_or_else(undef),
        "activities": jsv::member(thread, "activities", "thread.activities")?
            .cloned()
            .unwrap_or_else(undef),
        "proposedPlans": jsv::member(thread, "proposedPlans", "thread.proposedPlans")?
            .cloned()
            .unwrap_or_else(undef),
        "latestTurn": jsv::member(thread, "latestTurn", "thread.latestTurn")?
            .cloned()
            .unwrap_or_else(undef),
    }))?;
    // `thread.pendingInteractions?.filter(...)`
    let pending_interactions = jsv::opt(thread, "pendingInteractions");
    let actionable = if jsv::is_nullish(&pending_interactions) {
        undef()
    } else {
        let items = array_recv(
            &pending_interactions,
            "thread.pendingInteractions?.filter",
            "thread.pendingInteractions?.filter((interaction) => ...)",
        )?;
        let mut filtered = Vec::new();
        for interaction in items {
            let status = jsv::member(interaction, "status", "interaction.status")?
                .cloned()
                .unwrap_or_else(undef);
            if jsv::strict_eq(&status, &Value::String("pending".to_string()))
                || jsv::strict_eq(&status, &Value::String("retryable".to_string()))
            {
                filtered.push(interaction.clone());
            }
        }
        Value::Array(filtered)
    };
    let any_of_kind = |kind: &str| -> Result<Option<bool>, String> {
        if is_undef(&actionable) {
            return Ok(None);
        }
        let items = actionable.as_array();
        let Some(items) = items else {
            return Ok(Some(false));
        };
        for interaction in items {
            let k = jsv::member(
                interaction,
                "interactionKind",
                "interaction.interactionKind",
            )?
            .cloned()
            .unwrap_or_else(undef);
            if jsv::strict_eq(&k, &Value::String(kind.to_string())) {
                return Ok(Some(true));
            }
        }
        Ok(Some(false))
    };
    let mut m = Map::new();
    m.insert(
        "latestUserMessageAt".to_string(),
        jsv::opt(&metadata, "latestUserMessageAt"),
    );
    let latest_human = jsv::opt(thread, "latestHumanMessageAt");
    m.insert(
        "latestHumanMessageAt".to_string(),
        if !is_undef(&latest_human) {
            latest_human
        } else {
            jsv::opt(&metadata, "latestHumanMessageAt")
        },
    );
    m.insert(
        "hasPendingApprovals".to_string(),
        match any_of_kind("approval")? {
            Some(b) => Value::Bool(b),
            None => jsv::opt(&metadata, "hasPendingApprovals"),
        },
    );
    m.insert(
        "hasPendingUserInput".to_string(),
        match any_of_kind("userInput")? {
            Some(b) => Value::Bool(b),
            None => jsv::opt(&metadata, "hasPendingUserInput"),
        },
    );
    m.insert(
        "hasActionableProposedPlan".to_string(),
        jsv::opt(&metadata, "hasActionableProposedPlan"),
    );
    Ok(Value::Object(m))
}

/// `withDerivedThreadStateSignals`.
fn with_derived_thread_state_signals(thread: &Value) -> Result<Value, String> {
    let next_signals = derive_thread_state_signals(thread)?;
    if same(
        &jsv::opt(thread, "latestUserMessageAt"),
        &jsv::opt(&next_signals, "latestUserMessageAt"),
    ) && same(
        &jsv::opt(thread, "latestHumanMessageAt"),
        &jsv::opt(&next_signals, "latestHumanMessageAt"),
    ) && same(
        &jsv::opt(thread, "hasPendingApprovals"),
        &jsv::opt(&next_signals, "hasPendingApprovals"),
    ) && same(
        &jsv::opt(thread, "hasPendingUserInput"),
        &jsv::opt(&next_signals, "hasPendingUserInput"),
    ) && same(
        &jsv::opt(thread, "hasActionableProposedPlan"),
        &jsv::opt(&next_signals, "hasActionableProposedPlan"),
    ) {
        return Ok(thread.clone());
    }
    let mut m = spread_any(thread);
    for (k, v) in jsv::spread(&next_signals) {
        m.insert(k, v);
    }
    Ok(Value::Object(m))
}

/// `applyThreadUpdate(state, threadId, updater, options)` — `recompute_summary_signals` is
/// `options?.recomputeSummarySignals !== false`; `update_sidebar_summary` is the raw
/// `options?.updateSidebarSummary` value (`?? true` inside `commitThreadProjection`).
pub(crate) fn apply_thread_update(
    state: Value,
    thread_id: &Value,
    updater: &mut dyn FnMut(Value) -> Result<Value, String>,
    recompute_summary_signals: bool,
    update_sidebar_summary: &Value,
) -> Result<Value, String> {
    let Some(current_thread) = get_thread_from_state(&state, thread_id)? else {
        return Ok(state);
    };
    let updated_thread = if recompute_summary_signals {
        with_derived_thread_state_signals(&updater(current_thread.clone())?)?
    } else {
        updater(current_thread.clone())?
    };
    if same(&updated_thread, &current_thread) {
        return Ok(state);
    }
    let next_state = write_thread_state(state, &updated_thread, Some(&current_thread))?;
    commit_thread_projection(next_state, thread_id, update_sidebar_summary)
}

// ---- spaces / projects (`storeProjection.ts`) ---------------------------------------------------

/// Sort comparator shared by space lists: `l.sortOrder - r.sortOrder || l.id.localeCompare(r.id)`.
fn space_sort_cmp(left: &Value, right: &Value) -> Result<f64, String> {
    let by_order = to_num(&jsv::opt(left, "sortOrder")) - to_num(&jsv::opt(right, "sortOrder"));
    if by_order != 0.0 && !by_order.is_nan() {
        return Ok(by_order);
    }
    let lid = jsv::member(left, "id", "left.id")?
        .cloned()
        .unwrap_or_else(undef);
    let rid = jsv::member(right, "id", "right.id")?
        .cloned()
        .unwrap_or_else(undef);
    Ok(jsv::locale_compare(&js_string(&lid), &js_string(&rid)) as f64)
}

/// `upsertSpace`.
pub(crate) fn upsert_space(state: Value, space: &Value) -> Result<Value, String> {
    let spaces_v = jsv::member(&state, "spaces", "state.spaces")?
        .cloned()
        .unwrap_or_else(undef);
    let items = array_recv(
        &spaces_v,
        "state.spaces.find",
        "state.spaces.find((space) => space.id === incoming.id)",
    )?;
    let incoming_id = jsv::member(space, "id", "incoming.id")?
        .cloned()
        .unwrap_or_else(undef);
    let mut existing: Option<&Value> = None;
    for s in items {
        let sid = jsv::member(s, "id", "space.id")?
            .cloned()
            .unwrap_or_else(undef);
        if same(&sid, &incoming_id) {
            existing = Some(s);
            break;
        }
    }
    let existing_v = existing.cloned().unwrap_or_else(undef);
    let next_space = normalize_space(space, &existing_v)?;
    if same(&existing_v, &next_space) {
        return Ok(state);
    }
    let mut new_spaces: Vec<Value> = Vec::with_capacity(items.len() + 1);
    if is_undef(&existing_v) || jsv::is_nullish(&existing_v) {
        new_spaces.extend(items.iter().cloned());
        new_spaces.push(next_space);
    } else {
        for s in items {
            let sid = jsv::member(s, "id", "space.id")?
                .cloned()
                .unwrap_or_else(undef);
            if same(&sid, &incoming_id) {
                new_spaces.push(next_space.clone());
            } else {
                new_spaces.push(s.clone());
            }
        }
    }
    let mut sorted = new_spaces;
    let mut err: Option<String> = None;
    crate::js::sort(&mut sorted, |a, b| match space_sort_cmp(a, b) {
        Ok(n) => n,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            0.0
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    Ok(set_field(&state, "spaces", Value::Array(sorted)))
}

/// `removeSpace`.
pub(crate) fn remove_space(
    state: Value,
    space_id: &Value,
    deleted_at: &Value,
) -> Result<Value, String> {
    let spaces_v = jsv::member(&state, "spaces", "state.spaces")?
        .cloned()
        .unwrap_or_else(undef);
    let mut next_spaces = Vec::new();
    for s in array_recv(
        &spaces_v,
        "state.spaces.filter",
        "state.spaces.filter((space) => space.id !== spaceId)",
    )? {
        let sid = jsv::member(s, "id", "space.id")?
            .cloned()
            .unwrap_or_else(undef);
        if !same(&sid, space_id) {
            next_spaces.push(s.clone());
        }
    }
    let projects_v = jsv::member(&state, "projects", "state.projects")?
        .cloned()
        .unwrap_or_else(undef);
    let mut projects_changed = false;
    let mut next_projects = Vec::new();
    for project in array_recv(
        &projects_v,
        "state.projects.map",
        "state.projects.map((project) => ...)",
    )? {
        let project_space = coalesce(
            &jsv::member(project, "spaceId", "project.spaceId")?
                .cloned()
                .unwrap_or_else(undef),
            &Value::Null,
        )
        .clone();
        if !same(&project_space, space_id) {
            next_projects.push(project.clone());
            continue;
        }
        projects_changed = true;
        let mut p = spread_any(project);
        p.insert("spaceId".to_string(), Value::Null);
        if !is_undef(deleted_at) {
            let cur = jsv::member(project, "updatedAt", "project.updatedAt")?
                .cloned()
                .unwrap_or_else(undef);
            let updated = if jsv::truthy(&cur) && js_gt(&cur, deleted_at) {
                cur
            } else {
                deleted_at.clone()
            };
            p.insert("updatedAt".to_string(), updated);
        }
        next_projects.push(Value::Object(p));
    }
    let spaces_len = spaces_v.as_array().map_or(0, Vec::len);
    if next_spaces.len() == spaces_len && !projects_changed {
        return Ok(state);
    }
    let mut m = spread_any(&state);
    m.insert("spaces".to_string(), Value::Array(next_spaces));
    m.insert(
        "projects".to_string(),
        if projects_changed {
            Value::Array(next_projects)
        } else {
            projects_v
        },
    );
    Ok(Value::Object(m))
}

/// `applySpaceOrder`.
pub(crate) fn apply_space_order(
    state: Value,
    ordered_space_ids: &Value,
    updated_at: &Value,
) -> Result<Value, String> {
    let id_items = array_recv(
        ordered_space_ids,
        "orderedSpaceIds.map",
        "orderedSpaceIds.map((spaceId, index) => [spaceId, index])",
    )?;
    let mut order_by_id: HashMap<String, f64> = HashMap::with_capacity(id_items.len());
    for (index, id) in id_items.iter().enumerate() {
        order_by_id.insert(js_map_key(id), index as f64);
    }
    let spaces_v = jsv::member(&state, "spaces", "state.spaces")?
        .cloned()
        .unwrap_or_else(undef);
    let mut new_spaces = Vec::new();
    for space in array_recv(
        &spaces_v,
        "state.spaces.map",
        "state.spaces.map((space) => ...)",
    )? {
        let sid = jsv::member(space, "id", "space.id")?
            .cloned()
            .unwrap_or_else(undef);
        match order_by_id.get(&js_map_key(&sid)) {
            None => new_spaces.push(space.clone()),
            Some(&sort_order) => {
                let current = jsv::member(space, "sortOrder", "space.sortOrder")?
                    .cloned()
                    .unwrap_or_else(undef);
                if same(&crate::js::number(sort_order), &current) {
                    new_spaces.push(space.clone());
                } else {
                    let mut s = spread_any(space);
                    s.insert("sortOrder".to_string(), crate::js::number(sort_order));
                    set_if_defined(&mut s, "updatedAt", updated_at);
                    new_spaces.push(Value::Object(s));
                }
            }
        }
    }
    let mut sorted = new_spaces;
    let mut err: Option<String> = None;
    crate::js::sort(&mut sorted, |a, b| match space_sort_cmp(a, b) {
        Ok(n) => n,
        Err(e) => {
            if err.is_none() {
                err = Some(e);
            }
            0.0
        }
    });
    if let Some(e) = err {
        return Err(e);
    }
    let sorted_v = Value::Array(sorted);
    if arrays_shallow_equal(&sorted_v, &spaces_v) {
        return Ok(state);
    }
    Ok(set_field(&state, "spaces", sorted_v))
}

/// `upsertProject` — `mode` is the `ProjectMatchPolicy`: `"id-only"` or `"id-or-cwd"`.
pub(crate) fn upsert_project(state: Value, project: &Value, mode: &str) -> Result<Value, String> {
    let incoming_id = jsv::member(project, "id", "incoming.id")?
        .cloned()
        .unwrap_or_else(undef);
    let tombstones = jsv::member(
        &state,
        "deletedProjectIdsById",
        "state.deletedProjectIdsById",
    )?
    .cloned()
    .unwrap_or_else(undef);
    // `state.deletedProjectIdsById?.[incoming.id] !== undefined`
    let tombstone = opt_index(&tombstones, &record_key(&incoming_id));
    if tombstone.is_some_and(|t| !is_undef(t)) {
        return Ok(state);
    }
    let projects_v = jsv::member(&state, "projects", "state.projects")?
        .cloned()
        .unwrap_or_else(undef);
    let items = array_recv(
        &projects_v,
        "state.projects.find",
        "state.projects.find((project) => ...)",
    )?;
    let mut existing: Option<&Value> = None;
    for p in items {
        let pid = jsv::member(p, "id", "project.id")?
            .cloned()
            .unwrap_or_else(undef);
        if same(&pid, &incoming_id) {
            existing = Some(p);
            break;
        }
    }
    if existing.is_none() && mode == "id-or-cwd" {
        let incoming_root = jsv::member(project, "workspaceRoot", "incoming.workspaceRoot")?
            .cloned()
            .unwrap_or_else(undef);
        let incoming_key = project_cwd_key(&incoming_root)?;
        for p in items {
            let cwd = jsv::member(p, "cwd", "project.cwd")?
                .cloned()
                .unwrap_or_else(undef);
            if project_cwd_key(&cwd)? == incoming_key {
                existing = Some(p);
                break;
            }
        }
    }
    let existing_v = existing.cloned().unwrap_or_else(undef);
    let next_project = normalize_project(project, &existing_v)?;
    if same(&existing_v, &next_project) {
        return Ok(state);
    }
    if let Some(existing_project) = existing {
        let existing_id = jsv::member(existing_project, "id", "existingProject.id")?
            .cloned()
            .unwrap_or_else(undef);
        let mut mapped = Vec::with_capacity(items.len());
        for p in items {
            if same(&jsv::opt(p, "id"), &existing_id) {
                mapped.push(next_project.clone());
            } else {
                mapped.push(p.clone());
            }
        }
        return Ok(set_field(&state, "projects", Value::Array(mapped)));
    }
    let mut appended = items.clone();
    appended.push(next_project);
    Ok(set_field(&state, "projects", Value::Array(appended)))
}
