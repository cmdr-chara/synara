//! The Rust port of `apps/web/src/splitView.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

/// `v.key` — `None` mirrors a missing member; errors carry the member name.
fn prop<'v>(v: &'v Value, key: &str) -> Result<Option<&'v Value>, String> {
    if v.is_null() {
        return Err(format!("Cannot read properties of null (reading '{key}')"));
    }
    if is_undef(v) {
        return Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        ));
    }
    Ok(v.as_object().and_then(|o| o.get(key)))
}

/// `v.key` resolving a missing member to the `undefined` marker.
fn member(v: &Value, key: &str) -> Result<Value, String> {
    Ok(prop(v, key)?.cloned().unwrap_or_else(crate::js::undefined))
}

/// `a === b`: strict equality; distinct structured objects never equal.
fn strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    if let (Some(x), Some(y)) = (crate::js::string_units(a), crate::js::string_units(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        _ => false,
    }
}

/// `Number(str)` — trims whitespace then parses JS numeric syntax.
fn str_to_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}');
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .or_else(|| t.strip_prefix("+0x"))
        .or_else(|| t.strip_prefix("+0X"))
    {
        return u64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(bin) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(oct) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    match t {
        "Infinity" | "+Infinity" | "Inf" | "+Inf" => f64::INFINITY,
        "-Infinity" | "-Inf" => f64::NEG_INFINITY,
        _ => t.parse().unwrap_or(f64::NAN),
    }
}

/// `ToNumber(v)` for the `*` operator in layout math.
fn to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(*b as u8),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => str_to_number(s),
        Value::Array(items) => match items.len() {
            0 => 0.0,
            1 => to_number(&items[0]),
            _ => f64::NAN,
        },
        Value::Object(_) => {
            if let Some(n) = crate::js::as_number(v) {
                return n;
            }
            if let Some(units) = crate::js::string_units(v) {
                return str_to_number(&String::from_utf16_lossy(&units));
            }
            if is_undef(v) {
                return f64::NAN;
            }
            f64::NAN
        }
    }
}

/// `{...root, first, second}` — copies enumerable keys in order, then assigns
/// `first`/`second` (in place when the key already exists, appended otherwise).
fn with_first_second(root: &Value, first: Value, second: Value) -> Value {
    let mut out = match root.as_object() {
        Some(o) => o.clone(),
        None => Map::new(),
    };
    out.insert("first".into(), first);
    out.insert("second".into(), second);
    Value::Object(out)
}

fn is_leaf_kind(kind: &Value) -> bool {
    matches!(kind.as_str(), Some("leaf"))
}

fn find_pane(root: &Value, pane_id: &Value) -> Result<Option<Value>, String> {
    if strict_eq(&member(root, "id")?, pane_id) {
        return Ok(Some(root.clone()));
    }
    if is_leaf_kind(&member(root, "kind")?) {
        return Ok(None);
    }
    let first = find_pane(&member(root, "first")?, pane_id)?;
    if first.is_some() {
        return Ok(first);
    }
    find_pane(&member(root, "second")?, pane_id)
}

fn find_parent(root: &Value, pane_id: &Value) -> Result<Option<Value>, String> {
    if is_leaf_kind(&member(root, "kind")?) {
        return Ok(None);
    }
    let first = member(root, "first")?;
    let second = member(root, "second")?;
    if strict_eq(&member(&first, "id")?, pane_id) || strict_eq(&member(&second, "id")?, pane_id) {
        return Ok(Some(root.clone()));
    }
    if let Some(found) = find_parent(&first, pane_id)? {
        return Ok(Some(found));
    }
    find_parent(&second, pane_id)
}

fn depth_of(root: &Value, pane_id: &Value) -> Result<Option<f64>, String> {
    if strict_eq(&member(root, "id")?, pane_id) {
        return Ok(Some(0.0));
    }
    if is_leaf_kind(&member(root, "kind")?) {
        return Ok(None);
    }
    if let Some(d) = depth_of(&member(root, "first")?, pane_id)? {
        return Ok(Some(d + 1.0));
    }
    match depth_of(&member(root, "second")?, pane_id)? {
        Some(d) => Ok(Some(d + 1.0)),
        None => Ok(None),
    }
}

fn leaves_of(root: &Value, out: &mut Vec<Value>) -> Result<(), String> {
    if is_leaf_kind(&member(root, "kind")?) {
        out.push(root.clone());
        return Ok(());
    }
    leaves_of(&member(root, "first")?, out)?;
    leaves_of(&member(root, "second")?, out)
}

/// Port of `findPaneById` (`apps/web/src/splitView.logic.ts:18`).
pub fn find_pane_by_id(
    root: serde_json::Value,
    pane_id: String,
) -> Result<Option<serde_json::Value>, String> {
    find_pane(&root, &Value::String(pane_id))
}

/// Port of `findLeafPaneById` (`apps/web/src/splitView.logic.ts:28`).
pub fn find_leaf_pane_by_id(
    root: serde_json::Value,
    pane_id: String,
) -> Result<Option<serde_json::Value>, String> {
    match find_pane(&root, &Value::String(pane_id))? {
        Some(found) if is_leaf_kind(&member(&found, "kind")?) => Ok(Some(found)),
        _ => Ok(None),
    }
}

/// Port of `findSplitNodeById` (`apps/web/src/splitView.logic.ts:33`).
pub fn find_split_node_by_id(
    root: serde_json::Value,
    pane_id: String,
) -> Result<Option<serde_json::Value>, String> {
    match find_pane(&root, &Value::String(pane_id))? {
        Some(found) if member(&found, "kind")?.as_str() == Some("split") => Ok(Some(found)),
        _ => Ok(None),
    }
}

/// Port of `findParentSplitNode` (`apps/web/src/splitView.logic.ts:39`).
pub fn find_parent_split_node(
    root: serde_json::Value,
    pane_id: String,
) -> Result<Option<serde_json::Value>, String> {
    find_parent(&root, &Value::String(pane_id))
}

/// Port of `findPaneDepth` (`apps/web/src/splitView.logic.ts:49`).
pub fn find_pane_depth(root: serde_json::Value, pane_id: String) -> Result<Option<f64>, String> {
    depth_of(&root, &Value::String(pane_id))
}

/// Port of `collectLeaves` (`apps/web/src/splitView.logic.ts:64`).
pub fn collect_leaves(root: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let mut out = Vec::new();
    leaves_of(&root, &mut out)?;
    Ok(out)
}

fn rect(left: f64, top: f64, width: f64, height: f64) -> Value {
    json!({
        "left": crate::js::number(left),
        "top": crate::js::number(top),
        "width": crate::js::number(width),
        "height": crate::js::number(height),
    })
}

/// Port of `layoutSplitPanes` (`apps/web/src/splitView.logic.ts:91`).
pub fn layout_split_panes(root: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut leaves = Vec::new();
    let mut splits = Vec::new();
    fn visit(
        pane: &Value,
        r: (f64, f64, f64, f64),
        leaves: &mut Vec<Value>,
        splits: &mut Vec<Value>,
    ) -> Result<(), String> {
        if is_leaf_kind(&member(pane, "kind")?) {
            leaves.push(json!({ "leaf": pane, "rect": rect(r.0, r.1, r.2, r.3) }));
            return Ok(());
        }
        splits.push(json!({ "node": pane, "rect": rect(r.0, r.1, r.2, r.3) }));
        let (left, top, width, height) = r;
        let ratio = to_number(&member(pane, "ratio")?);
        let first = member(pane, "first")?;
        let second = member(pane, "second")?;
        if member(pane, "direction")?.as_str() == Some("horizontal") {
            let first_width = width * ratio;
            visit(&first, (left, top, first_width, height), leaves, splits)?;
            visit(
                &second,
                (left + first_width, top, width - first_width, height),
                leaves,
                splits,
            )?;
            return Ok(());
        }
        let first_height = height * ratio;
        visit(&first, (left, top, width, first_height), leaves, splits)?;
        visit(
            &second,
            (left, top + first_height, width, height - first_height),
            leaves,
            splits,
        )
    }
    visit(&root, (0.0, 0.0, 1.0, 1.0), &mut leaves, &mut splits)?;
    Ok(json!({ "leaves": leaves, "splits": splits }))
}

/// Port of `replacePaneInTree` (`apps/web/src/splitView.logic.ts:124`).
pub fn replace_pane_in_tree(
    root: serde_json::Value,
    pane_id: String,
    replacement: serde_json::Value,
) -> Result<serde_json::Value, String> {
    fn rep(root: &Value, pane_id: &Value, replacement: &Value) -> Result<(Value, bool), String> {
        if strict_eq(&member(root, "id")?, pane_id) {
            return Ok((replacement.clone(), true));
        }
        if is_leaf_kind(&member(root, "kind")?) {
            return Ok((root.clone(), false));
        }
        let old_first = member(root, "first")?;
        let old_second = member(root, "second")?;
        let (first, cf) = rep(&old_first, pane_id, replacement)?;
        let (second, cs) = rep(&old_second, pane_id, replacement)?;
        if !cf && !cs {
            return Ok((root.clone(), false));
        }
        Ok((with_first_second(root, first, second), true))
    }
    Ok(rep(&root, &Value::String(pane_id), &replacement)?.0)
}

/// `removeLeafBy*` shared recursion: returns (nextRoot, removedLeafIds).
fn remove_leaf(
    root: &Value,
    pred: impl Fn(&Value) -> Result<bool, String> + Copy,
) -> Result<(Value, Vec<Value>), String> {
    if is_leaf_kind(&member(root, "kind")?) {
        if pred(root)? {
            return Ok((Value::Null, vec![member(root, "id")?]));
        }
        return Ok((root.clone(), vec![]));
    }
    let (first_root, mut removed) = remove_leaf(&member(root, "first")?, pred)?;
    let (second_root, removed2) = remove_leaf(&member(root, "second")?, pred)?;
    removed.extend(removed2);
    if removed.is_empty() {
        return Ok((root.clone(), removed));
    }
    let first_alive = !first_root.is_null();
    let second_alive = !second_root.is_null();
    if first_alive && second_alive {
        return Ok((with_first_second(root, first_root, second_root), removed));
    }
    if first_alive {
        return Ok((first_root, removed));
    }
    if second_alive {
        return Ok((second_root, removed));
    }
    Ok((Value::Null, removed))
}

/// Port of `removeLeafByThreadId` (`apps/web/src/splitView.logic.ts:146`).
pub fn remove_leaf_by_thread_id(
    root: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (next_root, removed) = remove_leaf(&root, |leaf: &Value| {
        Ok(strict_eq(&member(leaf, "threadId")?, &thread_id))
    })?;
    Ok(json!({ "nextRoot": next_root, "removedLeafIds": removed }))
}

/// Port of `removeLeafByPaneId` (`apps/web/src/splitView.logic.ts:179`).
pub fn remove_leaf_by_pane_id(
    root: serde_json::Value,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    let pane_id = Value::String(pane_id);
    let (next_root, removed) = remove_leaf(&root, |leaf: &Value| {
        Ok(strict_eq(&member(leaf, "id")?, &pane_id))
    })?;
    Ok(json!({ "nextRoot": next_root, "removedLeafIds": removed }))
}

/// Port of `canSubdivide` (`apps/web/src/splitView.logic.ts:215`).
pub fn can_subdivide(
    parent_direction: Option<String>,
    requested_direction: String,
) -> Result<bool, String> {
    let parent = parent_direction.map(Value::String).unwrap_or(Value::Null);
    if parent.is_null() {
        return Ok(true);
    }
    Ok(!strict_eq(&parent, &Value::String(requested_direction)))
}

/// Port of `canSubdividePane` (`apps/web/src/splitView.logic.ts:225`).
pub fn can_subdivide_pane(
    root: serde_json::Value,
    target_pane_id: String,
    requested_direction: String,
) -> Result<bool, String> {
    let pane_id = Value::String(target_pane_id);
    match find_pane(&root, &pane_id)? {
        Some(found) if is_leaf_kind(&member(&found, "kind")?) => {}
        _ => return Ok(false),
    }
    match depth_of(&root, &pane_id)? {
        Some(d) if d < 2.0 => {}
        _ => return Ok(false),
    }
    let parent_direction = match find_parent(&root, &pane_id)? {
        Some(parent) => member(&parent, "direction")?,
        None => Value::Null,
    };
    if parent_direction.is_null() {
        return Ok(true);
    }
    Ok(!strict_eq(
        &parent_direction,
        &Value::String(requested_direction),
    ))
}

/// Port of `resolveDefaultFocusLeafId` (`apps/web/src/splitView.logic.ts:242`).
pub fn resolve_default_focus_leaf_id(root: serde_json::Value) -> Result<String, String> {
    let mut leaves = Vec::new();
    leaves_of(&root, &mut leaves)?;
    let id = match leaves.first() {
        Some(leaf) => member(leaf, "id")?,
        None => member(&root, "id")?,
    };
    match id {
        Value::String(s) => Ok(s),
        _ => Ok(crate::js::to_json(&id).unwrap_or_default()),
    }
}

/// `typeof value === "string"` for marker-aware strings.
fn is_string(v: &Value) -> bool {
    v.is_string() || crate::js::string_units(v).is_some()
}

/// Port of `isLegacySplitViewLike` (`apps/web/src/splitView.logic.ts:263`).
pub fn is_legacy_split_view_like(value: serde_json::Value) -> Result<bool, String> {
    let falsy = value.is_null()
        || is_undef(&value)
        || matches!(&value, Value::Bool(false))
        || matches!(&value, Value::Number(n) if n.as_f64().unwrap_or(0.0) == 0.0)
        || matches!(&value, Value::String(s) if s.is_empty());
    if falsy {
        return Ok(false);
    }
    let Some(o) = value.as_object() else {
        return Ok(false);
    };
    Ok(is_string(o.get("id").unwrap_or(&Value::Null))
        && is_string(o.get("sourceThreadId").unwrap_or(&Value::Null))
        && o.contains_key("leftThreadId")
        && o.contains_key("rightThreadId")
        && is_string(o.get("focusedPane").unwrap_or(&Value::Null)))
}
