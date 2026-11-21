//! The Rust port of `apps/web/src/splitView.logic.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn member(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (crate::js::as_number(a), crate::js::as_number(b)) {
        return x == y;
    }
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => is_undefined(a) && is_undefined(b),
    }
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Null | Value::Bool(false) => false,
        Value::Bool(true) => true,
        Value::String(s) => !s.is_empty(),
        Value::Object(_) => !is_undefined(v),
        Value::Array(_) => true,
        Value::Number(_) => false,
    }
}

fn js_str(v: &Value) -> String {
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if e.is_null() || is_undefined(e) {
                    String::new()
                } else {
                    js_str(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        v if is_undefined(v) => "undefined".to_string(),
        _ => "[object Object]".to_string(),
    }
}

fn kind_of(v: &Value) -> Result<String, String> {
    match member(v, "kind")? {
        Value::String(s) => Ok(s),
        _ => Ok(String::new()),
    }
}

fn num(v: &Value) -> f64 {
    crate::js::as_number(v).unwrap_or(f64::NAN)
}

fn num_value(n: f64) -> Value {
    if n.is_finite() {
        Value::from(n)
    } else if n.is_nan() {
        crate::js::number(f64::NAN)
    } else {
        crate::js::number(n)
    }
}

fn find_pane(root: &Value, pane_id: &Value) -> Result<Option<Value>, String> {
    if strict_eq(&member(root, "id")?, pane_id) {
        return Ok(Some(root.clone()));
    }
    if kind_of(root)? == "leaf" {
        return Ok(None);
    }
    let first = member(root, "first")?;
    if let Some(p) = find_pane(&first, pane_id)? {
        return Ok(Some(p));
    }
    let second = member(root, "second")?;
    find_pane(&second, pane_id)
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
        Some(p) if member(&p, "kind")?.as_str() == Some("leaf") => Ok(Some(p)),
        _ => Ok(None),
    }
}

/// Port of `findSplitNodeById` (`apps/web/src/splitView.logic.ts:33`).
pub fn find_split_node_by_id(
    root: serde_json::Value,
    pane_id: String,
) -> Result<Option<serde_json::Value>, String> {
    match find_pane(&root, &Value::String(pane_id))? {
        Some(p) if member(&p, "kind")?.as_str() == Some("split") => Ok(Some(p)),
        _ => Ok(None),
    }
}

fn find_parent(root: &Value, pane_id: &Value) -> Result<Option<Value>, String> {
    if kind_of(root)? == "leaf" {
        return Ok(None);
    }
    let first = member(root, "first")?;
    let second = member(root, "second")?;
    if strict_eq(&member(&first, "id")?, pane_id) || strict_eq(&member(&second, "id")?, pane_id) {
        return Ok(Some(root.clone()));
    }
    if let Some(p) = find_parent(&first, pane_id)? {
        return Ok(Some(p));
    }
    find_parent(&second, pane_id)
}

/// Port of `findParentSplitNode` (`apps/web/src/splitView.logic.ts:39`).
pub fn find_parent_split_node(
    root: serde_json::Value,
    pane_id: String,
) -> Result<Option<serde_json::Value>, String> {
    find_parent(&root, &Value::String(pane_id))
}

fn find_depth(root: &Value, pane_id: &Value) -> Result<Option<f64>, String> {
    if strict_eq(&member(root, "id")?, pane_id) {
        return Ok(Some(0.0));
    }
    if kind_of(root)? == "leaf" {
        return Ok(None);
    }
    let first = member(root, "first")?;
    if let Some(d) = find_depth(&first, pane_id)? {
        return Ok(Some(d + 1.0));
    }
    let second = member(root, "second")?;
    Ok(find_depth(&second, pane_id)?.map(|d| d + 1.0))
}

/// Port of `findPaneDepth` (`apps/web/src/splitView.logic.ts:49`).
pub fn find_pane_depth(root: serde_json::Value, pane_id: String) -> Result<Option<f64>, String> {
    find_depth(&root, &Value::String(pane_id))
}

fn collect(root: &Value, out: &mut Vec<Value>) -> Result<(), String> {
    if kind_of(root)? == "leaf" {
        out.push(root.clone());
        return Ok(());
    }
    let first = member(root, "first")?;
    collect(&first, out)?;
    let second = member(root, "second")?;
    collect(&second, out)
}

/// Port of `collectLeaves` (`apps/web/src/splitView.logic.ts:64`).
pub fn collect_leaves(root: serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    let mut out = Vec::new();
    collect(&root, &mut out)?;
    Ok(out)
}

fn rect(l: f64, t: f64, w: f64, h: f64) -> Value {
    let mut m = Map::new();
    m.insert("left".to_string(), num_value(l));
    m.insert("top".to_string(), num_value(t));
    m.insert("width".to_string(), num_value(w));
    m.insert("height".to_string(), num_value(h));
    Value::Object(m)
}

fn visit(
    pane: &Value,
    r: (f64, f64, f64, f64),
    leaves: &mut Vec<Value>,
    splits: &mut Vec<Value>,
) -> Result<(), String> {
    if kind_of(pane)? == "leaf" {
        let mut m = Map::new();
        m.insert("leaf".to_string(), pane.clone());
        m.insert("rect".to_string(), rect(r.0, r.1, r.2, r.3));
        leaves.push(Value::Object(m));
        return Ok(());
    }
    let mut m = Map::new();
    m.insert("node".to_string(), pane.clone());
    m.insert("rect".to_string(), rect(r.0, r.1, r.2, r.3));
    splits.push(Value::Object(m));
    let ratio = num(&member(pane, "ratio")?);
    let first = member(pane, "first")?;
    let second = member(pane, "second")?;
    if member(pane, "direction")?.as_str() == Some("horizontal") {
        let first_width = r.2 * ratio;
        visit(&first, (r.0, r.1, first_width, r.3), leaves, splits)?;
        visit(
            &second,
            (r.0 + first_width, r.1, r.2 - first_width, r.3),
            leaves,
            splits,
        )
    } else {
        let first_height = r.3 * ratio;
        visit(&first, (r.0, r.1, r.2, first_height), leaves, splits)?;
        visit(
            &second,
            (r.0, r.1 + first_height, r.2, r.3 - first_height),
            leaves,
            splits,
        )
    }
}

/// Port of `layoutSplitPanes` (`apps/web/src/splitView.logic.ts:91`).
pub fn layout_split_panes(root: serde_json::Value) -> Result<serde_json::Value, String> {
    let mut leaves = Vec::new();
    let mut splits = Vec::new();
    visit(&root, (0.0, 0.0, 1.0, 1.0), &mut leaves, &mut splits)?;
    let mut m = Map::new();
    m.insert("leaves".to_string(), Value::Array(leaves));
    m.insert("splits".to_string(), Value::Array(splits));
    Ok(Value::Object(m))
}

/// Port of `replacePaneInTree` (`apps/web/src/splitView.logic.ts:124`).
pub fn replace_pane_in_tree(
    root: serde_json::Value,
    pane_id: String,
    replacement: serde_json::Value,
) -> Result<serde_json::Value, String> {
    replace_pane(&root, &Value::String(pane_id), &replacement)
}

fn replace_pane(root: &Value, pane_id: &Value, replacement: &Value) -> Result<Value, String> {
    if strict_eq(&member(root, "id")?, pane_id) {
        return Ok(replacement.clone());
    }
    if kind_of(root)? == "leaf" {
        return Ok(root.clone());
    }
    let old_first = member(root, "first")?;
    let old_second = member(root, "second")?;
    let first = replace_pane(&old_first, pane_id, replacement)?;
    let second = replace_pane(&old_second, pane_id, replacement)?;
    if first == old_first && second == old_second {
        return Ok(root.clone());
    }
    let mut m = match root {
        Value::Object(map) => map.clone(),
        _ => Map::new(),
    };
    m.insert("first".to_string(), first);
    m.insert("second".to_string(), second);
    Ok(Value::Object(m))
}

fn remove_by(
    root: &Value,
    pred: &dyn Fn(&Value) -> Result<bool, String>,
) -> Result<(Value, Vec<Value>), String> {
    if kind_of(root)? == "leaf" {
        if pred(root)? {
            return Ok((Value::Null, vec![member(root, "id")?]));
        }
        return Ok((root.clone(), Vec::new()));
    }
    let old_first = member(root, "first")?;
    let old_second = member(root, "second")?;
    let (first, mut ids) = remove_by(&old_first, pred)?;
    let (second, ids2) = remove_by(&old_second, pred)?;
    ids.extend(ids2);
    if ids.is_empty() {
        return Ok((root.clone(), ids));
    }
    let f_ok = truthy(&first);
    let s_ok = truthy(&second);
    if f_ok && s_ok {
        let mut m = match root {
            Value::Object(map) => map.clone(),
            _ => Map::new(),
        };
        m.insert("first".to_string(), first);
        m.insert("second".to_string(), second);
        return Ok((Value::Object(m), ids));
    }
    if f_ok {
        return Ok((first, ids));
    }
    if s_ok {
        return Ok((second, ids));
    }
    Ok((Value::Null, ids))
}

fn remove_result(next_root: Value, ids: Vec<Value>) -> Value {
    let mut m = Map::new();
    m.insert("nextRoot".to_string(), next_root);
    m.insert("removedLeafIds".to_string(), Value::Array(ids));
    Value::Object(m)
}

/// Port of `removeLeafByThreadId` (`apps/web/src/splitView.logic.ts:146`).
pub fn remove_leaf_by_thread_id(
    root: serde_json::Value,
    thread_id: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (next_root, ids) = remove_by(&root, &|leaf: &Value| {
        Ok(strict_eq(&member(leaf, "threadId")?, &thread_id))
    })?;
    Ok(remove_result(next_root, ids))
}

/// Port of `removeLeafByPaneId` (`apps/web/src/splitView.logic.ts:179`).
pub fn remove_leaf_by_pane_id(
    root: serde_json::Value,
    pane_id: String,
) -> Result<serde_json::Value, String> {
    let pid = Value::String(pane_id);
    let (next_root, ids) = remove_by(&root, &|leaf: &Value| {
        Ok(strict_eq(&member(leaf, "id")?, &pid))
    })?;
    Ok(remove_result(next_root, ids))
}

/// Port of `canSubdivide` (`apps/web/src/splitView.logic.ts:215`).
pub fn can_subdivide(
    parent_direction: Option<String>,
    requested_direction: String,
) -> Result<bool, String> {
    match parent_direction {
        None => Ok(true),
        Some(d) => Ok(d != requested_direction),
    }
}

/// Port of `canSubdividePane` (`apps/web/src/splitView.logic.ts:225`).
pub fn can_subdivide_pane(
    root: serde_json::Value,
    target_pane_id: String,
    requested_direction: String,
) -> Result<bool, String> {
    let pid = Value::String(target_pane_id);
    let leaf = match find_pane(&root, &pid)? {
        Some(p) if member(&p, "kind")?.as_str() == Some("leaf") => p,
        _ => return Ok(false),
    };
    let _ = leaf;
    let depth = match find_depth(&root, &pid)? {
        Some(d) => d,
        None => return Ok(false),
    };
    if depth >= 2.0 {
        return Ok(false);
    }
    let parent = find_parent(&root, &pid)?;
    let parent_direction = match &parent {
        Some(p) => member(p, "direction")?,
        None => Value::Null,
    };
    if parent_direction.is_null() || is_undefined(&parent_direction) {
        return Ok(true);
    }
    Ok(js_str(&parent_direction) != requested_direction)
}

/// Port of `resolveDefaultFocusLeafId` (`apps/web/src/splitView.logic.ts:242`).
pub fn resolve_default_focus_leaf_id(root: serde_json::Value) -> Result<String, String> {
    let mut leaves = Vec::new();
    collect(&root, &mut leaves)?;
    if let Some(first) = leaves.first() {
        let id = member(first, "id")?;
        if !id.is_null() && !is_undefined(&id) {
            return Ok(js_str(&id));
        }
    }
    Ok(js_str(&member(&root, "id")?))
}

/// Port of `isLegacySplitViewLike` (`apps/web/src/splitView.logic.ts:263`).
pub fn is_legacy_split_view_like(value: serde_json::Value) -> Result<bool, String> {
    let Value::Object(m) = &value else {
        return Ok(false);
    };
    if is_undefined(&value) {
        return Ok(false);
    }
    Ok(matches!(m.get("id"), Some(Value::String(_)))
        && matches!(m.get("sourceThreadId"), Some(Value::String(_)))
        && m.contains_key("leftThreadId")
        && m.contains_key("rightThreadId")
        && matches!(m.get("focusedPane"), Some(Value::String(_))))
}
