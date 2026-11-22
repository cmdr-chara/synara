//! Clean-room port of `packages/shared/src/keybindingRules.ts`, written from
//! `cases.jsonl` recordings only (ADR-001/ADR-002: T3-derived source unit).

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

fn undef() -> Value {
    crate::js::undefined()
}

fn is_undef(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn prop_err(receiver: &Value, key: &str) -> String {
    if receiver.is_null() {
        format!("Cannot read properties of null (reading '{key}')")
    } else {
        format!("Cannot read properties of undefined (reading '{key}')")
    }
}

/// `v[key]` with JS semantics: throws on null/undefined receivers, yields
/// `undefined` on primitives and for missing keys.
fn prop(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(prop_err(v, key)),
        _ if is_undef(v) => Err(prop_err(v, key)),
        Value::Object(o) => Ok(o.get(key).cloned().unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Object(o) if o.len() == 1 => {
            if is_undef(v) {
                false
            } else if let Some(x) = o.get("$num") {
                x.as_str() == Some("NaN") || x.as_f64() == Some(0.0)
            } else {
                true
            }
        }
        _ => true,
    }
}

fn js_number(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        }
    } else if n == 0.0 {
        "0".into()
    } else if n.fract() == 0.0 && n.abs() < 1e21 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

/// `String(v)` coercion.
fn to_text(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.as_f64().map(js_number).unwrap_or_else(|| n.to_string()),
        Value::String(s) => s.clone(),
        Value::Array(items) => items.iter().map(to_text).collect::<Vec<_>>().join(","),
        Value::Object(o) if o.len() == 1 && is_undef(v) => "undefined".into(),
        Value::Object(o) if o.len() == 1 => match o.get("$num") {
            Some(x) => x.as_str().map(String::from).unwrap_or_default(),
            None => "[object Object]".into(),
        },
        _ => "[object Object]".into(),
    }
}

fn is_ws(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\u{000B}'
            | '\u{000C}'
            | '\u{00A0}'
            | '\u{FEFF}'
            | '\u{1680}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202F}'
            | '\u{205F}'
            | '\u{3000}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_ws)
}

// ---- shortcut parsing -----------------------------------------------------

fn modifier_flag(name: &str) -> Option<&'static str> {
    match name {
        "ctrl" | "control" => Some("ctrlKey"),
        "shift" => Some("shiftKey"),
        "alt" => Some("altKey"),
        "meta" | "cmd" => Some("metaKey"),
        "mod" => Some("modKey"),
        "option" => Some("altKey"),
        _ => None,
    }
}

fn parse_shortcut(value: &Value) -> Result<Option<Value>, String> {
    let text = match value {
        Value::String(s) => s.clone(),
        Value::Null => return Err(prop_err(value, "toLowerCase")),
        _ if is_undef(value) => return Err(prop_err(value, "toLowerCase")),
        _ => return Err("value.toLowerCase is not a function".into()),
    };
    let lowered = text.to_lowercase();
    let mut parts: Vec<&str> = lowered.split('+').map(js_trim).collect();
    let last = match parts.pop() {
        Some(l) => l,
        None => return Ok(None),
    };
    // An empty last part encodes the literal '+' key; empty modifier slots are
    // dropped only in that case ("ctrl++" parses, "+a" does not).
    let key = if last.is_empty() {
        parts.retain(|p| !p.is_empty());
        "+".to_string()
    } else {
        match last {
            "space" => " ".to_string(),
            "esc" => "escape".to_string(),
            other => other.to_string(),
        }
    };
    let mut obj = Map::new();
    obj.insert("key".into(), Value::Null);
    obj.insert("metaKey".into(), Value::Bool(false));
    obj.insert("ctrlKey".into(), Value::Bool(false));
    obj.insert("shiftKey".into(), Value::Bool(false));
    obj.insert("altKey".into(), Value::Bool(false));
    obj.insert("modKey".into(), Value::Bool(false));
    for head in parts {
        match modifier_flag(head) {
            Some(flag) => {
                obj.insert(flag.into(), Value::Bool(true));
            }
            None => return Ok(None),
        }
    }
    if !last.is_empty() && modifier_flag(last).is_some() {
        return Ok(None);
    }
    obj.insert("key".into(), Value::String(key));
    Ok(Some(Value::Object(obj)))
}

// ---- when-expression parsing ----------------------------------------------

fn ident_first(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn ident_rest(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.'
}

struct Cursor<'a> {
    chars: Vec<char>,
    at: usize,
    _src: &'a str,
}

impl<'a> Cursor<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().collect(),
            at: 0,
            _src: src,
        }
    }
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }
    fn eat(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.at += 1;
        Some(c)
    }
    fn ws(&mut self) {
        while let Some(c) = self.peek() {
            if is_ws(c) {
                self.at += 1;
            } else {
                break;
            }
        }
    }
    fn word(&mut self, w: &str) -> bool {
        let mut i = self.at;
        for wc in w.chars() {
            if self.chars.get(i) != Some(&wc) {
                return false;
            }
            i += 1;
        }
        self.at = i;
        true
    }
}

fn ast_binary(kind: &str, left: Value, right: Value) -> Value {
    json!({ "type": kind, "left": left, "right": right })
}

fn parse_when_or(cur: &mut Cursor) -> Option<Value> {
    let mut node = parse_when_and(cur)?;
    loop {
        cur.ws();
        if cur.word("||") {
            let right = parse_when_and(cur)?;
            node = ast_binary("or", node, right);
        } else {
            return Some(node);
        }
    }
}

fn parse_when_and(cur: &mut Cursor) -> Option<Value> {
    let mut node = parse_when_unary(cur)?;
    loop {
        cur.ws();
        if cur.word("&&") {
            let right = parse_when_unary(cur)?;
            node = ast_binary("and", node, right);
        } else {
            return Some(node);
        }
    }
}

fn parse_when_unary(cur: &mut Cursor) -> Option<Value> {
    cur.ws();
    match cur.peek() {
        Some('!') => {
            cur.eat();
            let inner = parse_when_unary(cur)?;
            Some(json!({ "type": "not", "node": inner }))
        }
        Some('(') => {
            cur.eat();
            let inner = parse_when_or(cur)?;
            cur.ws();
            if cur.eat() == Some(')') {
                Some(inner)
            } else {
                None
            }
        }
        Some(c) if ident_first(c) => {
            let start = cur.at;
            cur.eat();
            while let Some(c) = cur.peek() {
                if ident_rest(c) {
                    cur.at += 1;
                } else {
                    break;
                }
            }
            let name: String = cur.chars[start..cur.at].iter().collect();
            Some(json!({ "type": "identifier", "name": name }))
        }
        _ => None,
    }
}

fn parse_when_expr(value: &Value) -> Result<Option<Value>, String> {
    match value {
        Value::Null => return Err(prop_err(value, "length")),
        _ if is_undef(value) => return Err(prop_err(value, "length")),
        Value::String(_) => {}
        _ => return Ok(None),
    }
    let text = value.as_str().unwrap_or_default();
    let mut cur = Cursor::new(text);
    let node = parse_when_or(&mut cur);
    cur.ws();
    match node {
        Some(n) if cur.peek().is_none() => Ok(Some(n)),
        _ => Ok(None),
    }
}

// ---- encoders ---------------------------------------------------------------

fn encode_shortcut(shortcut: &Value) -> Result<Option<String>, String> {
    let mut parts: Vec<String> = Vec::new();
    for (flag, name) in [
        ("modKey", "mod"),
        ("metaKey", "meta"),
        ("ctrlKey", "ctrl"),
        ("altKey", "alt"),
        ("shiftKey", "shift"),
    ] {
        if truthy(&prop(shortcut, flag)?) {
            parts.push(name.to_string());
        }
    }
    let key = prop(shortcut, "key")?;
    if !truthy(&key) {
        return Ok(None);
    }
    let key_text = match &key {
        Value::String(s) => {
            // `shortcut.key.includes(' ')` in the source semantics.
            if s == " " {
                "space".to_string()
            } else {
                s.clone()
            }
        }
        Value::Array(items) => items.iter().map(to_text).collect::<Vec<_>>().join(","),
        _ => return Err("shortcut.key.includes is not a function".into()),
    };
    parts.push(key_text);
    Ok(Some(parts.join("+")))
}

fn encode_when_node(node: &Value, _parent: &str) -> Result<Value, String> {
    let ty = prop(node, "type")?;
    match ty.as_str() {
        Some("identifier") => prop(node, "name"),
        Some("not") => {
            let child = prop(node, "node")?;
            let encoded = encode_when_node(&child, "not")?;
            let mut text = String::from("!");
            let needs_parens = child
                .get("type")
                .and_then(Value::as_str)
                .map(|t| t == "and" || t == "or")
                .unwrap_or(false);
            if needs_parens {
                text.push('(');
            }
            text.push_str(&to_text(&encoded));
            if needs_parens {
                text.push(')');
            }
            Ok(Value::String(text))
        }
        Some(kind @ ("and" | "or")) => {
            let joiner = if kind == "and" { " && " } else { " || " };
            let parent_prec = if kind == "or" { 1 } else { 2 };
            let mut text = String::new();
            for side in ["left", "right"] {
                let child = prop(node, side)?;
                let encoded = encode_when_node(&child, kind)?;
                let mut piece = if is_undef(&encoded) {
                    String::new()
                } else {
                    to_text(&encoded)
                };
                let child_prec = match child.get("type").and_then(Value::as_str) {
                    Some("or") => 1,
                    Some("and") => 2,
                    _ => 4,
                };
                let wrap = if side == "left" {
                    child_prec < parent_prec
                } else {
                    child_prec <= parent_prec
                };
                if wrap {
                    piece = format!("({piece})");
                }
                if side == "right" {
                    text.push_str(joiner);
                }
                text.push_str(&piece);
            }
            Ok(Value::String(text))
        }
        _ => Ok(undef()),
    }
}

// ---- exports --------------------------------------------------------------

pub fn parse_keybinding_shortcut(value: Value) -> Result<Value, String> {
    Ok(parse_shortcut(&value)?.unwrap_or(Value::Null))
}

pub fn parse_keybinding_when_expression(expression: Value) -> Result<Value, String> {
    Ok(parse_when_expr(&expression)?.unwrap_or(Value::Null))
}

pub fn compile_keybinding_rule(rule: Value) -> Result<Value, String> {
    let key = prop(&rule, "key")?;
    let Some(shortcut) = parse_shortcut(&key)? else {
        return Ok(Value::Null);
    };
    let mut out = Map::new();
    out.insert("command".into(), prop(&rule, "command")?);
    out.insert("shortcut".into(), shortcut);
    let when = prop(&rule, "when")?;
    if !is_undef(&when) {
        match parse_when_expr(&when)? {
            Some(ast) => {
                out.insert("whenAst".into(), ast);
            }
            None => return Ok(Value::Null),
        }
    }
    Ok(Value::Object(out))
}

pub fn encode_keybinding_shortcut(shortcut: Value) -> Result<Option<String>, String> {
    encode_shortcut(&shortcut)
}

pub fn encode_keybinding_when(node: Value) -> Result<Value, String> {
    encode_when_node(&node, "top")
}

pub fn encode_keybinding_rule(rule: Value) -> Result<Value, String> {
    let shortcut = prop(&rule, "shortcut")?;
    let _ = prop(&shortcut, "key")?;
    let encoded = encode_shortcut(&shortcut)?;
    let mut out = Map::new();
    out.insert("command".into(), prop(&rule, "command")?);
    out.insert(
        "key".into(),
        encoded.map(Value::String).unwrap_or(Value::Null),
    );
    let when_ast = prop(&rule, "whenAst")?;
    if !when_ast.is_null() && !is_undef(&when_ast) {
        out.insert("when".into(), encode_when_node(&when_ast, "top")?);
    }
    Ok(Value::Object(out))
}

pub fn resolved_keybinding_rule_identity(rule: Value) -> Result<String, String> {
    let shortcut = prop(&rule, "shortcut")?;
    let _ = prop(&shortcut, "key")?;
    let encoded = encode_shortcut(&shortcut)?.unwrap_or_default();
    let when_ast = prop(&rule, "whenAst")?;
    let when_text = if when_ast.is_null() || is_undef(&when_ast) {
        String::new()
    } else {
        to_text(&encode_when_node(&when_ast, "top")?)
    };
    let command = to_text(&prop(&rule, "command")?);
    Ok(format!("{command}\u{0}{encoded}\u{0}{when_text}"))
}

pub fn keybinding_rule_identity(rule: Value) -> Result<Option<String>, String> {
    let key = prop(&rule, "key")?;
    let Some(shortcut) = parse_shortcut(&key)? else {
        return Ok(None);
    };
    let encoded = encode_shortcut(&shortcut)?.unwrap_or_default();
    let when = prop(&rule, "when")?;
    let when_text = if is_undef(&when) {
        String::new()
    } else {
        match parse_when_expr(&when)? {
            Some(ast) => to_text(&encode_when_node(&ast, "top")?),
            None => return Ok(None),
        }
    };
    let command = to_text(&prop(&rule, "command")?);
    Ok(Some(format!("{command}\u{0}{encoded}\u{0}{when_text}")))
}

pub fn is_unassigned_keybinding_shortcut(shortcut: Value) -> Result<bool, String> {
    Ok(prop(&shortcut, "key")?.as_str() == Some("unassigned"))
}

pub fn is_unassigned_keybinding_rule(rule: Value) -> Result<bool, String> {
    let key = prop(&rule, "key")?;
    let parsed = parse_shortcut(&key)?;
    Ok(parsed
        .as_ref()
        .and_then(|s| s.get("key"))
        .and_then(Value::as_str)
        == Some("unassigned"))
}
