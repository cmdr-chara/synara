//! The Rust port of `scripts/check-server-sync-fs.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value};
use std::collections::HashSet;

// ---- a small token scanner over TypeScript source ----------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Str(String),
    Punct(char),
    Other,
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_' || c == '$' || (c as u32) > 127
}
fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$' || (c as u32) > 127
}

fn tokenize(src: &str) -> Vec<Tok> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i = (i + 2).min(chars.len());
            continue;
        }
        if c == '"' || c == '\'' || c == '`' {
            let quote = c;
            i += 1;
            let mut s = String::new();
            while i < chars.len() && chars[i] != quote {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    s.push(chars[i]);
                    s.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                s.push(chars[i]);
                i += 1;
            }
            i += 1;
            toks.push(Tok::Str(s));
            continue;
        }
        if is_ident_start(c) {
            let mut s = String::new();
            while i < chars.len() && is_ident_char(chars[i]) {
                s.push(chars[i]);
                i += 1;
            }
            toks.push(Tok::Ident(s));
            continue;
        }
        toks.push(Tok::Punct(c));
        i += 1;
    }
    toks
}

fn tok_is(t: &Tok, word: &str) -> bool {
    matches!(t, Tok::Ident(w) if w == word)
}

fn is_fs_specifier(s: &str) -> bool {
    s == "fs" || s == "node:fs"
}

fn count_sync_fs_references_impl(source: &str) -> Vec<(String, f64)> {
    let toks = tokenize(source);
    let mut namespaces: HashSet<String> = HashSet::new();
    let mut named: Vec<(String, String)> = Vec::new(); // local -> imported
    let mut counts: Vec<(String, f64)> = Vec::new();
    let count = |counts: &mut Vec<(String, f64)>, name: &str| {
        if let Some(e) = counts.iter_mut().find(|(n, _)| n == name) {
            e.1 += 1.0;
        } else {
            counts.push((name.to_string(), 1.0));
        }
    };
    // Pass 1: `import ... from "fs" | "node:fs"` statements.
    let mut i = 0;
    let mut import_spans: Vec<(usize, usize)> = Vec::new();
    while i < toks.len() {
        if !tok_is(&toks[i], "import") {
            i += 1;
            continue;
        }
        // `import(` is a dynamic import, not a declaration.
        if toks.get(i + 1) == Some(&Tok::Punct('(')) {
            i += 1;
            continue;
        }
        let start = i;
        let mut j = i + 1;
        // `import type` is type-only and ignored.
        if tok_is(toks.get(j).unwrap_or(&Tok::Other), "type")
            && matches!(
                toks.get(j + 1),
                Some(Tok::Ident(_)) | Some(Tok::Punct('*')) | Some(Tok::Punct('{'))
            )
        {
            // type-only: skip the whole statement for counting purposes
            while j < toks.len() && toks[j] != Tok::Punct(';') {
                j += 1;
            }
            i = j + 1;
            continue;
        }
        let mut ns: Vec<String> = Vec::new();
        let mut nm: Vec<(String, String)> = Vec::new();
        if let Some(Tok::Ident(local)) = toks.get(j) {
            ns.push(local.clone());
            j += 1;
        }
        if toks.get(j) == Some(&Tok::Punct(',')) {
            j += 1;
        }
        if toks.get(j) == Some(&Tok::Punct('*')) {
            j += 1;
            if tok_is(toks.get(j).unwrap_or(&Tok::Other), "as") {
                j += 1;
            }
            if let Some(Tok::Ident(local)) = toks.get(j) {
                ns.push(local.clone());
                j += 1;
            }
        } else if toks.get(j) == Some(&Tok::Punct('{')) {
            j += 1;
            while j < toks.len() && toks[j] != Tok::Punct('}') {
                let type_only = tok_is(&toks[j], "type");
                if type_only {
                    j += 1;
                }
                let Some(Tok::Ident(first)) = toks.get(j).cloned() else {
                    j += 1;
                    continue;
                };
                j += 1;
                let imported = first;
                if tok_is(toks.get(j).unwrap_or(&Tok::Other), "as") {
                    j += 1;
                    if let Some(Tok::Ident(local)) = toks.get(j) {
                        if !type_only && imported.ends_with("Sync") {
                            nm.push((local.clone(), imported.clone()));
                        }
                        let _ = local;
                        j += 1;
                    }
                } else if !type_only && imported.ends_with("Sync") {
                    nm.push((imported.clone(), imported.clone()));
                }
                if toks.get(j) == Some(&Tok::Punct(',')) {
                    j += 1;
                }
            }
            if toks.get(j) == Some(&Tok::Punct('}')) {
                j += 1;
            }
        }
        if tok_is(toks.get(j).unwrap_or(&Tok::Other), "from") {
            j += 1;
        }
        if let Some(Tok::Str(spec)) = toks.get(j) {
            if is_fs_specifier(spec) {
                namespaces.extend(ns);
                named.extend(nm);
            }
            j += 1;
        }
        // Skip the rest of the import statement.
        while j < toks.len() && toks[j] != Tok::Punct(';') {
            j += 1;
        }
        import_spans.push((start, j + 1));
        i = j + 1;
    }
    // Pass 2: namespace aliases `x = ns` to a fixed point.
    loop {
        let mut added = false;
        for w in 0..toks.len().saturating_sub(1) {
            if toks[w] == Tok::Punct('=')
                && toks.get(w + 2) != Some(&Tok::Punct('='))
                && toks.get(w + 1) != Some(&Tok::Punct('='))
                && let (Some(Tok::Ident(lhs)), Some(Tok::Ident(rhs))) =
                    (toks.get(w.wrapping_sub(1)), toks.get(w + 1))
                && namespaces.contains(rhs)
                && !namespaces.contains(lhs)
            {
                namespaces.insert(lhs.clone());
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    // Pass 3: scan all tokens outside import statements.
    let in_import = |i: usize| import_spans.iter().any(|(a, b)| i >= *a && i < *b);
    let mut i = 0;
    while i < toks.len() {
        if in_import(i) {
            i += 1;
            continue;
        }
        // `import x = require("fs")`
        if tok_is(&toks[i], "import")
            && matches!(toks.get(i + 1), Some(Tok::Ident(_)))
            && toks.get(i + 2) == Some(&Tok::Punct('='))
            && tok_is(toks.get(i + 3).unwrap_or(&Tok::Other), "require")
            && toks.get(i + 4) == Some(&Tok::Punct('('))
            && let Some(Tok::Str(spec)) = toks.get(i + 5)
            && is_fs_specifier(spec)
        {
            count(&mut counts, "non-static fs import");
        }
        // `require("fs")` / `import("fs")`
        if (tok_is(&toks[i], "require") || tok_is(&toks[i], "import"))
            && toks.get(i + 1) == Some(&Tok::Punct('('))
            && let Some(Tok::Str(spec)) = toks.get(i + 2)
            && is_fs_specifier(spec)
        {
            count(&mut counts, "non-static fs import");
        }
        // namespace passed as a call argument
        if matches!(&toks[i], Tok::Ident(_)) && toks.get(i + 1) == Some(&Tok::Punct('(')) {
            let mut j = i + 2;
            while j < toks.len() && toks[j] != Tok::Punct(')') {
                if let Tok::Ident(arg) = &toks[j]
                    && namespaces.contains(arg)
                {
                    count(&mut counts, "fs namespace escape");
                }
                j += 1;
            }
        }
        // `export ... from "fs"`
        if tok_is(&toks[i], "export") {
            let mut j = i + 1;
            while j < toks.len() && toks[j] != Tok::Punct(';') && !tok_is(&toks[j], "from") {
                j += 1;
            }
            if let Some(Tok::Str(spec)) = toks.get(j + 1)
                && is_fs_specifier(spec)
            {
                count(&mut counts, "fs re-export");
            }
        }
        // named import usage
        if let Tok::Ident(name) = &toks[i] {
            for (local, imported) in &named {
                if local == name {
                    count(&mut counts, imported);
                }
            }
        }
        // `ns.prop` / `ns[...]`
        if let Tok::Ident(base) = &toks[i]
            && namespaces.contains(base)
        {
            if toks.get(i + 1) == Some(&Tok::Punct('.')) {
                if let Some(Tok::Ident(prop)) = toks.get(i + 2)
                    && prop.ends_with("Sync")
                {
                    count(&mut counts, prop);
                }
            } else if toks.get(i + 1) == Some(&Tok::Punct('[')) {
                match toks.get(i + 2) {
                    Some(Tok::Str(key)) => {
                        if key.ends_with("Sync") {
                            count(&mut counts, key);
                        }
                    }
                    _ => count(&mut counts, "dynamic fs access"),
                }
            }
        }
        // `const { a, b: c } = ns`
        if toks[i] == Tok::Punct('{') {
            let mut j = i + 1;
            let mut keys: Vec<String> = Vec::new();
            while j < toks.len() && toks[j] != Tok::Punct('}') {
                if let Tok::Ident(k) = &toks[j] {
                    keys.push(k.clone());
                }
                j += 1;
            }
            if toks.get(j) == Some(&Tok::Punct('}'))
                && toks.get(j + 1) == Some(&Tok::Punct('='))
                && let Some(Tok::Ident(rhs)) = toks.get(j + 2)
                && namespaces.contains(rhs)
            {
                for key in keys {
                    if key.ends_with("Sync") {
                        count(&mut counts, &key);
                    }
                }
            }
        }
        i += 1;
    }
    counts
}

/// Port of `countSyncFsReferences` (`scripts/check-server-sync-fs.ts:17`).
pub fn count_sync_fs_references(source: String) -> Result<Value, String> {
    let mut obj = Map::new();
    for (name, n) in count_sync_fs_references_impl(&source) {
        obj.insert(name, crate::js::number(n));
    }
    Ok(Value::Object(obj))
}

fn js_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => f64::NAN,
    }
}

/// Port of `syncFsViolations` (`scripts/check-server-sync-fs.ts:137`).
pub fn sync_fs_violations(
    file: String,
    source: String,
    budget: Value,
) -> Result<Vec<String>, String> {
    let exception = match &budget {
        Value::Object(o) if crate::js::is_undefined(&budget) => {
            let _ = o;
            return Err(format!(
                "Cannot read properties of undefined (reading '{file}')"
            ));
        }
        Value::Object(o) => o.get(&file).cloned(),
        Value::Null => {
            return Err(format!("Cannot read properties of null (reading '{file}')"));
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .find(|(i, _)| i.to_string() == file)
            .map(|(_, v)| v.clone()),
        _ => None,
    };
    let actual = count_sync_fs_references_impl(&source);
    let reason_ok = match &exception {
        Some(exc) => match exc.get("reason") {
            Some(Value::String(r)) => !r
                .trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
                .is_empty(),
            Some(Value::Null) => {
                return Err("Cannot read properties of null (reading 'trim')".to_string());
            }
            Some(v) if crate::js::is_undefined(v) => {
                return Err("Cannot read properties of undefined (reading 'trim')".to_string());
            }
            Some(_) => {
                return Err("exception.reason.trim is not a function".to_string());
            }
            None => {
                return Err("Cannot read properties of undefined (reading 'trim')".to_string());
            }
        },
        None => false,
    };
    let references = exception
        .as_ref()
        .and_then(|e| e.get("references"))
        .and_then(|r| r.as_object());
    let mut names: Vec<String> = actual.iter().map(|(n, _)| n.clone()).collect();
    if let Some(refs) = references {
        for k in refs.keys() {
            if !names.contains(k) {
                names.push(k.clone());
            }
        }
    }
    let mut out = Vec::new();
    for name in names {
        let count = actual
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, c)| *c)
            .unwrap_or(0.0);
        let expected = if reason_ok {
            references
                .and_then(|r| r.get(&name))
                .map(js_num)
                .unwrap_or(0.0)
        } else {
            0.0
        };
        if count == expected {
            continue;
        }
        out.push(if count < expected {
            format!(
                "{file}: stale {name} budget ({} expected, {} present). Reduce the budget when removing sync references.",
                crate::js::number_to_string(expected),
                crate::js::number_to_string(count)
            )
        } else {
            format!(
                "{file}: {name} has {} references; allowed {}. Use async fs/Effect FileSystem or justify a narrowly scoped exception.",
                crate::js::number_to_string(count),
                crate::js::number_to_string(expected)
            )
        });
    }
    Ok(out)
}

fn is_ts_source(file: &str) -> bool {
    // `/\.tsx?$/` && !`/\.(test|spec)(-d)?\.tsx?$/`
    let ts = file.ends_with(".ts") || file.ends_with(".tsx");
    if !ts {
        return false;
    }
    let stem = file
        .strip_suffix(".tsx")
        .or_else(|| file.strip_suffix(".ts"))
        .unwrap_or(file);
    !(stem.ends_with(".test")
        || stem.ends_with(".spec")
        || stem.ends_with(".test-d")
        || stem.ends_with(".spec-d"))
}

/// Port of `serverSources` (`scripts/check-server-sync-fs.ts:154`).
pub fn server_sources(root: String) -> Result<Vec<String>, String> {
    let entries = match std::fs::read_dir(&root) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "ENOENT: no such file or directory, scandir '{root}'"
            ));
        }
        Err(e) => return Err(format!("{e}")),
    };
    let mut out = Vec::new();
    for entry in entries {
        let Ok(entry) = entry else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        if ["fixtures", "testing", "__tests__"].contains(&name.as_str()) {
            continue;
        }
        let file = if root.ends_with('/') || root.is_empty() {
            format!("{root}{name}")
        } else {
            format!("{root}/{name}")
        };
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        if is_dir {
            out.extend(server_sources(file)?);
            continue;
        }
        if is_ts_source(&file) {
            out.push(file);
        }
    }
    Ok(out)
}
