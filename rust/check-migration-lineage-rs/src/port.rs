//! The Rust port of `scripts/check-migration-lineage.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

fn is_js_ws(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

fn js_string_coerce(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_string_coerce).collect::<Vec<_>>().join(","),
        Value::Object(o) if crate::js::is_undefined(v) => {
            let _ = o;
            "undefined".to_string()
        }
        Value::Object(o) => {
            if let Some(Value::String(n)) = o.get("$num") {
                // String(-0) === "0" in JS.
                if n == "-0" {
                    return "0".to_string();
                }
                return n.clone();
            }
            "[object Object]".to_string()
        }
    }
}

fn js_num(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_parse_number(s),
        Value::Object(o) => {
            if let Some(Value::String(n)) = o.get("$num") {
                match n.as_str() {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    "-0" => -0.0,
                    _ => f64::NAN,
                }
            } else {
                f64::NAN
            }
        }
        _ => f64::NAN,
    }
}

fn js_parse_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    if t.is_empty() {
        return 0.0;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|h| h as f64)
            .unwrap_or(f64::NAN);
    }
    match t {
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => t.parse::<f64>().unwrap_or(f64::NAN),
    }
}

fn js_lte(a: &Value, b: &Value) -> bool {
    let (x, y) = (js_num(a), js_num(b));
    if x.is_nan() || y.is_nan() {
        return false;
    }
    x <= y
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.as_f64().unwrap_or(f64::NAN) == y.as_f64().unwrap_or(f64::NAN)
        }
        _ => a == b,
    }
}

fn iterable(v: &Value) -> Option<Vec<Value>> {
    match v {
        Value::Array(a) => Some(a.clone()),
        Value::String(s) => Some(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ => None,
    }
}

const MIGRATIONS_FILE: &str = "apps/server/src/persistence/Migrations.ts";

fn find_export_array_block<'a>(source: &'a str, name: &str) -> Option<&'a str> {
    // `export const <name>\s*=\s*\[` → the text after the opening bracket.
    let start = source.find(name)?;
    let rest = &source[start + name.len()..];
    let eq = rest.find('=')?;
    let mut i = skip_ws(rest, eq + 1);
    if !rest[i..].starts_with('[') {
        return None;
    }
    i += 1;
    Some(&rest[i..])
}

fn skip_ws(s: &str, mut i: usize) -> usize {
    while i < s.len() {
        let c = s[i..].chars().next().unwrap();
        if !is_js_ws(c) {
            break;
        }
        i += c.len_utf8();
    }
    i
}

fn parse_migration_entries(block: &str) -> Vec<Value> {
    // `\[\s*(\d+)\s*,\s*"([^"]+)"`
    let mut entries = Vec::new();
    let bytes = block.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        let mut j = skip_ws(block, i + 1);
        let num_start = j;
        while j < bytes.len() && bytes[j].is_ascii_digit() {
            j += 1;
        }
        if j == num_start {
            i += 1;
            continue;
        }
        let id_str = &block[num_start..j];
        j = skip_ws(block, j);
        if j >= bytes.len() || bytes[j] != b',' {
            i += 1;
            continue;
        }
        j = skip_ws(block, j + 1);
        if j >= bytes.len() || bytes[j] != b'"' {
            i += 1;
            continue;
        }
        let name_start = j + 1;
        let mut k = name_start;
        while k < bytes.len() && bytes[k] != b'"' {
            k += 1;
        }
        if k >= bytes.len() {
            i += 1;
            continue;
        }
        let name = &block[name_start..k];
        let mut o = Map::new();
        o.insert(
            "id".into(),
            crate::js::number(id_str.parse::<f64>().unwrap_or(f64::NAN)),
        );
        o.insert("name".into(), Value::String(name.to_string()));
        entries.push(Value::Object(o));
        i = k + 1;
    }
    entries
}

/// Port of `parseMigrationLineage` (`scripts/check-migration-lineage.ts:67`).
pub fn parse_migration_lineage(source: Value) -> Result<Vec<Value>, String> {
    let src = js_string_coerce(&source);
    let err = || format!("Could not locate the migrationEntries array in {MIGRATIONS_FILE}.");
    let Some(marker_pos) = src.find("export const migrationEntries") else {
        return Err(err());
    };
    let rest = &src[marker_pos..];
    let Some(body) = find_export_array_block(rest, "export const migrationEntries") else {
        return Err(err());
    };
    // The block is lazily terminated by the first `]` followed by `\s*as const;`.
    let mut end = None;
    let bytes = body.as_bytes();
    for (i, b) in bytes.iter().enumerate() {
        if *b != b']' {
            continue;
        }
        let j = skip_ws(body, i + 1);
        if body[j..].starts_with("as const;") {
            end = Some(i);
            break;
        }
    }
    let Some(end) = end else {
        return Err(err());
    };
    let entries = parse_migration_entries(&body[..end]);
    if entries.is_empty() {
        return Err(format!(
            "Parsed zero migrations from {MIGRATIONS_FILE}; the guard cannot verify it."
        ));
    }
    Ok(entries)
}

/// Port of `parseMigrationLineageAllowances` (`scripts/check-migration-lineage.ts:84`).
pub fn parse_migration_lineage_allowances(source: Value) -> Result<Vec<Value>, String> {
    let src = js_string_coerce(&source);
    let Some(pos) = src.find("export const MIGRATION_LINEAGE_ALIASES") else {
        return Ok(Vec::new());
    };
    let rest = &src[pos + "export const MIGRATION_LINEAGE_ALIASES".len()..];
    // `[^=]*=\s*\[`
    let Some(eq) = rest.find('=') else {
        return Ok(Vec::new());
    };
    let mut i = skip_ws(rest, eq + 1);
    if !rest[i..].starts_with('[') {
        return Ok(Vec::new());
    }
    i += 1;
    let body = &rest[i..];
    // lazy `([\s\S]*?)\n\];` — first `\n];`
    let Some(end) = body.find("\n];") else {
        return Ok(Vec::new());
    };
    let block = &body[..end];
    // `historicalId:\s*(\d+),\s*(?:\/\/[^\n]*\n\s*)*historicalName:\s*"([^"]+)"`
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(rel) = block[cursor..].find("historicalId:") {
        let mut j = cursor + rel + "historicalId:".len();
        j = skip_ws(block, j);
        let num_start = j;
        while j < block.len() && block.as_bytes()[j].is_ascii_digit() {
            j += 1;
        }
        if j == num_start {
            cursor = num_start + 1;
            continue;
        }
        let id_str = &block[num_start..j];
        j = skip_ws(block, j);
        if block.as_bytes().get(j) != Some(&b',') {
            cursor = num_start + 1;
            continue;
        }
        j = skip_ws(block, j + 1);
        // optional comment lines
        loop {
            if block[j..].starts_with("//") {
                let Some(nl) = block[j..].find('\n') else {
                    break;
                };
                j = skip_ws(block, j + nl + 1);
            } else {
                break;
            }
        }
        if !block[j..].starts_with("historicalName:") {
            cursor = num_start + 1;
            continue;
        }
        j = skip_ws(block, j + "historicalName:".len());
        if block.as_bytes().get(j) != Some(&b'"') {
            cursor = num_start + 1;
            continue;
        }
        let name_start = j + 1;
        let mut k = name_start;
        while k < block.len() && block.as_bytes()[k] != b'"' {
            k += 1;
        }
        if k >= block.len() {
            cursor = num_start + 1;
            continue;
        }
        let mut o = Map::new();
        o.insert(
            "id".into(),
            crate::js::number(id_str.parse::<f64>().unwrap_or(f64::NAN)),
        );
        o.insert(
            "name".into(),
            Value::String(block[name_start..k].to_string()),
        );
        out.push(Value::Object(o));
        cursor = k + 1;
    }
    Ok(out)
}

fn prop(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or_else(crate::js::undefined)
}

/// Port of `findLineageStructureViolations` (`scripts/check-migration-lineage.ts:101`).
pub fn find_lineage_structure_violations(entries: Value) -> Result<Vec<Value>, String> {
    let Some(list) = iterable(&entries) else {
        return Err("entries is not iterable".to_string());
    };
    let mut problems: Vec<String> = Vec::new();
    let mut seen: HashMap<String, (Value, Value)> = HashMap::new();
    let mut previous_id = crate::js::number(0.0);
    for entry in &list {
        let id = prop(entry, "id");
        let name = prop(entry, "name");
        let key = js_string_coerce(&id);
        if let Some((_, dup_name)) = seen.get(&key)
            && !crate::js::is_undefined(dup_name)
        {
            problems.push(format!(
                "Migration {} is declared twice (\"{}\" and \"{}\").",
                js_string_coerce(&id),
                js_string_coerce(dup_name),
                js_string_coerce(&name)
            ));
        }
        seen.insert(key, (id.clone(), name.clone()));
        if js_lte(&id, &previous_id) {
            problems.push(format!(
                "Migration {} (\"{}\") is out of ascending order.",
                js_string_coerce(&id),
                js_string_coerce(&name)
            ));
        }
        previous_id = id;
    }
    Ok(problems.into_iter().map(Value::String).collect())
}

/// Port of `findReleasedLineageViolations` (`scripts/check-migration-lineage.ts:121`).
pub fn find_released_lineage_violations(
    released: Value,
    current: Value,
    allowances: Value,
) -> Result<Vec<Value>, String> {
    let current_list = match &current {
        Value::Array(a) => a.clone(),
        Value::Null => {
            return Err("Cannot read properties of null (reading 'map')".to_string());
        }
        other if crate::js::is_undefined(other) => {
            return Err("Cannot read properties of undefined (reading 'map')".to_string());
        }
        _ => return Err("current.map is not a function".to_string()),
    };
    let allowances_list = if crate::js::is_undefined(&allowances) {
        Vec::new()
    } else {
        match &allowances {
            Value::Array(a) => a.clone(),
            Value::Null => {
                return Err("Cannot read properties of null (reading 'map')".to_string());
            }
            _ => return Err("allowances.map is not a function".to_string()),
        }
    };
    let Some(released_list) = iterable(&released) else {
        return Err("released is not iterable".to_string());
    };
    let mut current_names: HashMap<String, Value> = HashMap::new();
    for entry in &current_list {
        let id = prop(entry, "id");
        current_names.insert(js_string_coerce(&id), prop(entry, "name"));
    }
    let allowed: HashSet<String> = allowances_list
        .iter()
        .map(|a| {
            format!(
                "{}:{}",
                js_string_coerce(&prop(a, "id")),
                js_string_coerce(&prop(a, "name"))
            )
        })
        .collect();
    let mut violations = Vec::new();
    for entry in &released_list {
        let id = prop(entry, "id");
        let name = prop(entry, "name");
        let current_name = current_names.get(&js_string_coerce(&id));
        // `get` yields undefined when absent, and `undefined === undefined` is true.
        let current_name_v = current_name.cloned().unwrap_or_else(crate::js::undefined);
        if js_strict_eq(&current_name_v, &name) {
            continue;
        }
        if allowed.contains(&format!(
            "{}:{}",
            js_string_coerce(&id),
            js_string_coerce(&name)
        )) {
            continue;
        }
        let mut o = Map::new();
        o.insert("id".into(), id);
        o.insert("releasedName".into(), name);
        o.insert(
            "currentName".into(),
            if crate::js::is_undefined(&current_name_v) {
                Value::Null
            } else {
                current_name_v
            },
        );
        violations.push(Value::Object(o));
    }
    Ok(violations)
}

/// Port of `resolveMigrationLineageFamily` (`scripts/check-migration-lineage.ts:166`).
pub fn resolve_migration_lineage_family(
    server_version: Value,
    desktop_version: Value,
) -> Result<Value, String> {
    let server_is_string = matches!(&server_version, Value::String(_));
    if !server_is_string || !js_strict_eq(&server_version, &desktop_version) {
        return Err(
            "Migration lineage requires matching server and desktop package versions.".to_string(),
        );
    }
    let version = server_version.as_str().unwrap_or_default();
    // `^\d+\.\d+\.\d+(?:-(beta|remote)\.\d+)?$`
    let (base, suffix) = match version.split_once('-') {
        Some((b, s)) => (b, Some(s)),
        None => (version, None),
    };
    let three_numbers = |s: &str| -> bool {
        let mut parts = s.split('.');
        for _ in 0..3 {
            match parts.next() {
                Some(p) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => {}
                _ => return false,
            }
        }
        parts.next().is_none()
    };
    let suffix_ok = match suffix {
        None => true,
        Some(s) => {
            if let Some((word, digits)) = s.split_once('.') {
                (word == "beta" || word == "remote")
                    && !digits.is_empty()
                    && digits.bytes().all(|b| b.is_ascii_digit())
            } else {
                false
            }
        }
    };
    if !three_numbers(base) || !suffix_ok {
        return Err(format!(
            "Cannot determine the migration lineage family for version \"{version}\"."
        ));
    }
    let family = match suffix.and_then(|s| s.split('.').next()) {
        Some("remote") => "remote",
        _ => "shared",
    };
    Ok(Value::String(family.to_string()))
}

/// Port of `groupViolationsByPair` (`scripts/check-migration-lineage.ts:207`).
pub fn group_violations_by_pair(per_tag: Value) -> Result<Vec<Value>, String> {
    let Some(list) = iterable(&per_tag) else {
        return Err("perTag is not iterable".to_string());
    };
    let mut order: Vec<String> = Vec::new();
    let mut grouped: HashMap<String, (Value, Vec<Value>)> = HashMap::new();
    for item in &list {
        let tag = prop(item, "tag");
        let violations = prop(item, "violations");
        let Some(vlist) = iterable(&violations) else {
            return Err("violations is not iterable".to_string());
        };
        for violation in &vlist {
            let key = format!(
                "{}:{}",
                js_string_coerce(&prop(violation, "id")),
                js_string_coerce(&prop(violation, "releasedName"))
            );
            if let Some((_, tags)) = grouped.get_mut(&key) {
                tags.push(tag.clone());
                continue;
            }
            grouped.insert(key.clone(), (violation.clone(), vec![tag.clone()]));
            order.push(key);
        }
    }
    let mut grouped_list: Vec<(Value, Vec<Value>)> = order
        .iter()
        .filter_map(|k| grouped.get(k).cloned())
        .collect();
    crate::js::sort(&mut grouped_list, |a, b| {
        // V8 coerces a NaN comparator result to 0.
        let d = js_num(&prop(&a.0, "id")) - js_num(&prop(&b.0, "id"));
        if d.is_nan() { 0.0 } else { d }
    });
    Ok(grouped_list
        .into_iter()
        .map(|(violation, tags)| {
            let mut o = Map::new();
            o.insert("violation".into(), violation);
            o.insert("tags".into(), Value::Array(tags));
            Value::Object(o)
        })
        .collect())
}
