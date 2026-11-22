//! The Rust port of `apps/web/src/projectScripts.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn nullish(v: &Value) -> bool {
    v.is_null() || crate::js::is_undefined(v)
}

/// JS truthiness over the structured-value encoding.
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        _ => {
            if crate::js::is_undefined(v) {
                return false;
            }
            if let Some(x) = crate::js::as_number(v) {
                return x != 0.0 && !x.is_nan();
            }
            if let Some(units) = crate::js::string_units(v) {
                return !units.is_empty();
            }
            true
        }
    }
}

/// `v.name` — throws on null/undefined receivers like JS property access.
fn member<'a>(v: &'a Value, name: &str) -> Result<&'a Value, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{name}')"));
    }
    if crate::js::is_undefined(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{name}')"
        ));
    }
    Ok(plain_member(v, name))
}

/// Member read on a non-nullish receiver: objects give their own property,
/// anything else is undefined.
fn plain_member<'a>(v: &'a Value, name: &str) -> &'a Value {
    static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    match v {
        Value::Object(m) => m
            .get(name)
            .unwrap_or_else(|| UNDEF.get_or_init(crate::js::undefined)),
        _ => UNDEF.get_or_init(crate::js::undefined),
    }
}

/// `v?.name` — optional chaining: nullish receivers yield undefined, never throw.
fn member_opt<'a>(v: &'a Value, name: &str) -> &'a Value {
    static UNDEF: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    if nullish(v) {
        return UNDEF.get_or_init(crate::js::undefined);
    }
    plain_member(v, name)
}

/// `Array.from(v)` — iterables yield their items (strings their code points),
/// other array-likes read `.length` then numeric slots; null/undefined throw.
fn seq_of(v: &Value) -> Result<Vec<Value>, String> {
    if v.is_null() {
        return Err(
            "object null is not iterable (cannot read property Symbol(Symbol.iterator))"
                .to_string(),
        );
    }
    if crate::js::is_undefined(v) {
        return Err(
            "undefined is not iterable (cannot read property Symbol(Symbol.iterator))".to_string(),
        );
    }
    match v {
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        Value::Array(a) => Ok(a.clone()),
        _ => {
            let len = to_length(plain_member(v, "length"));
            Ok((0..len).map(|i| slot_of(v, i)).collect())
        }
    }
}

/// `ToLength`: NaN/negative become 0, capped at 2^53 - 1.
fn to_length(v: &Value) -> usize {
    let x = crate::js::as_number(v).unwrap_or(f64::NAN);
    if x.is_nan() || x <= 0.0 {
        return 0;
    }
    x.floor().min(9007199254740991.0) as usize
}

/// `v[i]` on an array-like receiver.
fn slot_of(v: &Value, i: usize) -> Value {
    match v {
        Value::Object(m) => m
            .get(i.to_string().as_str())
            .cloned()
            .unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    }
}

/// `^[a-z0-9][a-z0-9-]*$` — ASCII lowercase letters, digits, then dashes too.
fn id_chars_ok(s: &str) -> bool {
    let low = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit();
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if low(c) => {}
        _ => return false,
    }
    chars.all(|c| low(c) || c == '-')
}

/// A slug of `name`: lowercase, each run of non-`[a-z0-9]` becomes one `-`,
/// edges stripped, capped at 24 UTF-16 units; empty becomes `script`.
fn slug_of(name: &str) -> String {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if gap && !out.is_empty() {
                out.push('-');
            }
            gap = false;
            out.push(c);
        } else {
            gap = true;
        }
    }
    let units: Vec<u16> = out.encode_utf16().take(24).collect();
    let trimmed = String::from_utf16_lossy(&units);
    if trimmed.is_empty() {
        "script".to_string()
    } else {
        trimmed
    }
}

/// The error a schema-style id validator produces for a bad pattern.
fn id_pattern_error(id: &str) -> String {
    let shown = serde_json::to_string(&Value::String(id.to_string())).unwrap_or_default();
    format!("Expected a string matching the RegExp ^[a-z0-9][a-z0-9-]*$, got {shown}\n  at [1]")
}

fn check_id(id: &str) -> Result<(), String> {
    if id.is_empty() {
        let shown = serde_json::to_string(&Value::String(id.to_string())).unwrap_or_default();
        return Err(format!(
            "Expected a value with a length of at least 1, got {shown}\n  at [1]"
        ));
    }
    if !id_chars_ok(id) {
        return Err(id_pattern_error(id));
    }
    Ok(())
}

/// Port of `commandForProjectScript` (`apps/web/src/projectScripts.ts:25`).
pub fn command_for_project_script(script_id: String) -> Result<serde_json::Value, String> {
    check_id(&script_id)?;
    Ok(Value::String(format!("script.{script_id}.run")))
}

/// Port of `projectScriptIdFromCommand` (`apps/web/src/projectScripts.ts:28`).
pub fn project_script_id_from_command(command: String) -> Result<Option<String>, String> {
    match command
        .strip_prefix("script.")
        .and_then(|rest| rest.strip_suffix(".run"))
    {
        Some(inner) if check_id(inner).is_ok() => Ok(Some(inner.to_string())),
        _ => Ok(None),
    }
}

/// Port of `nextProjectScriptId` (`apps/web/src/projectScripts.ts:37`).
pub fn next_project_script_id(
    name: String,
    existing_ids: serde_json::Value,
) -> Result<String, String> {
    let taken = seq_of(&existing_ids)?;
    let base = slug_of(&name);
    let has = |candidate: &str| {
        taken
            .iter()
            .any(|entry| matches!(entry, Value::String(s) if s == candidate))
    };
    if !has(&base) {
        return Ok(base);
    }
    let mut n = 2u64;
    loop {
        let candidate = format!("{base}-{n}");
        if !has(&candidate) {
            return Ok(candidate);
        }
        n += 1;
    }
}

/// `Object`-ordered env keys: canonical array-index keys sort numerically
/// first, the rest keep insertion order.
fn js_key_order(entries: &mut [(String, Value)]) {
    fn index_key(k: &str) -> Option<u64> {
        if k.is_empty() || !k.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if k.len() > 1 && k.starts_with('0') {
            return None;
        }
        k.parse::<u64>().ok().filter(|n| *n < 4294967295)
    }
    entries.sort_by(|a, b| match (index_key(&a.0), index_key(&b.0)) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
}

/// `{...extraEnv}` — own enumerable properties: objects their entries, arrays
/// and strings index keys, primitives and nullish nothing.
fn spread_entries(v: &Value, out: &mut Vec<(String, Value)>) {
    if nullish(v) {
        return;
    }
    match v {
        Value::Object(m) => out.extend(m.iter().map(|(k, x)| (k.clone(), x.clone()))),
        Value::Array(a) => out.extend(
            a.iter()
                .enumerate()
                .map(|(i, x)| (i.to_string(), x.clone())),
        ),
        Value::String(s) => out.extend(
            s.encode_utf16()
                .enumerate()
                .map(|(i, u)| (i.to_string(), crate::js::utf16_string(&[u]))),
        ),
        _ => {}
    }
}

/// Port of `projectScriptRuntimeEnv` (`apps/web/src/projectScripts.ts:81`).
pub fn project_script_runtime_env(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let project = member(&input, "project")?;
    let cwd = member(project, "cwd")?.clone();
    let mut entries: Vec<(String, Value)> = vec![("SYNARA_PROJECT_ROOT".to_string(), cwd)];
    let worktree = member_opt(&input, "worktreePath");
    if !nullish(worktree) {
        entries.push(("SYNARA_WORKTREE_PATH".to_string(), worktree.clone()));
    }
    spread_entries(member_opt(&input, "extraEnv"), &mut entries);
    js_key_order(&mut entries);
    let mut out = Map::new();
    for (k, v) in entries {
        out.insert(k, v);
    }
    Ok(Value::Object(out))
}

/// `scripts.find(...)` — the finder errors on non-array receivers like a
/// missing array method does.
fn find_in(
    scripts: &Value,
    pred: impl Fn(&Value) -> Result<bool, String>,
) -> Result<Option<Value>, String> {
    match scripts {
        Value::Null => Err("Cannot read properties of null (reading 'find')".to_string()),
        _ if crate::js::is_undefined(scripts) => {
            Err("Cannot read properties of undefined (reading 'find')".to_string())
        }
        Value::Array(items) => {
            for s in items {
                if pred(s)? {
                    return Ok(Some(s.clone()));
                }
            }
            Ok(None)
        }
        _ => Err("scripts.find is not a function".to_string()),
    }
}

/// Port of `primaryProjectScript` (`apps/web/src/projectScripts.ts:96`).
pub fn primary_project_script(
    scripts: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    find_in(&scripts, |s| Ok(truthy(s)))
}

/// Port of `setupProjectScript` (`apps/web/src/projectScripts.ts:101`).
pub fn setup_project_script(
    scripts: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    find_in(&scripts, |s| {
        Ok(member(s, "name")? == &Value::String("setup".to_string()))
    })
}
