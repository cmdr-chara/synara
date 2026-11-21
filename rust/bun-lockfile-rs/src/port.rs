//! The Rust port of `scripts/bun-lockfile.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

// JS `\s` for the comma lookahead (the Unicode space set JS regexes use,
// which differs from Rust regex `\s` on U+0085 and U+FEFF).
const JS_SPACE: &str = "\t\n\u{b}\u{c}\r \u{a0}\u{1680}\u{2000}\u{2001}\u{2002}\u{2003}\u{2004}\u{2005}\u{2006}\u{2007}\u{2008}\u{2009}\u{200a}\u{2028}\u{2029}\u{202f}\u{205f}\u{3000}\u{feff}";

// `contents.replace(/"(?:[^"\\]|\\.)*"|,(?=\s*[}\]])/g, t => t === "," ? "" : t)`
fn strip_trailing_commas(input: &str) -> String {
    let b: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == '"' {
            let mut j = i + 1;
            while j < b.len() && b[j] != '"' {
                // `\\.` escapes any char except a line terminator.
                if b[j] == '\\' {
                    match b.get(j + 1) {
                        Some(&c) if !matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}') => j += 2,
                        _ => break,
                    }
                } else {
                    j += 1;
                }
            }
            if j < b.len() {
                out.extend(&b[i..=j]);
                i = j + 1;
                continue;
            }
        }
        if b[i] == ',' {
            let mut j = i + 1;
            while j < b.len() && JS_SPACE.contains(b[j]) {
                j += 1;
            }
            if j < b.len() && matches!(b[j], '}' | ']') {
                i += 1;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    out
}

/// Port of `normalizeBunLockfile` (`scripts/bun-lockfile.ts:3`).
pub fn normalize_bun_lockfile(contents: String) -> Result<String, String> {
    let Ok(mut lockfile) = serde_json::from_str::<Value>(&strip_trailing_commas(&contents)) else {
        return Ok(contents);
    };
    // `lockfileVersion !== 3 || !lockfile.workspaces`: workspaces falsy means
    // null/undefined/false/0/"" (objects, arrays and non-empty strings pass).
    let is_v3 = matches!(lockfile.get("lockfileVersion"), Some(v) if v.as_f64() == Some(3.0));
    let falsy_ws = match lockfile.get("workspaces") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => true,
        Some(Value::Number(n)) => n.as_f64() == Some(0.0),
        Some(Value::String(s)) => s.is_empty(),
        _ => false,
    };
    if !is_v3 || falsy_ws {
        return Ok(contents);
    }
    // Object.values: object values in key order or array elements; primitives
    // yield nothing. A null member throws on `.version` -> catch -> verbatim.
    let members: Vec<&mut Value> = match lockfile.get_mut("workspaces") {
        Some(Value::Object(m)) => m.values_mut().collect(),
        Some(Value::Array(a)) => a.iter_mut().collect(),
        _ => Vec::new(),
    };
    for member in members {
        if member.is_null() {
            return Ok(contents);
        }
        if let Value::Object(m) = member
            && m.get("version").is_some_and(Value::is_string)
        {
            m.remove("version");
        }
    }
    Ok(jscore::json_stringify(&lockfile))
}
