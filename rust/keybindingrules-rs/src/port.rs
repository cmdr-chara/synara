//! The Rust port of `packages/shared/src/keybindingRules.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

use crate::js::is_undefined;

fn prop_read_err(v: &Value, key: &str) -> String {
    if v.is_null() {
        format!("Cannot read properties of null (reading '{key}')")
    } else {
        format!("Cannot read properties of undefined (reading '{key}')")
    }
}

/// `v[key]` with JS semantics: throws on null/undefined receivers, yields
/// `undefined` on primitives, and `undefined` for missing keys.
fn field(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(prop_read_err(v, key)),
        v if is_undefined(v) => Err(prop_read_err(v, key)),
        Value::Object(_) => Ok(v.get(key).cloned().unwrap_or_else(undef)),
        _ => Ok(undef()),
    }
}

fn undef() -> Value {
    crate::js::undefined()
}

/// `String(value)` coercion for the shapes this port meets.
fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        v if is_undefined(v) => "undefined".to_string(),
        Value::Array(a) => a.iter().map(js_string).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
        Value::String(s) => !s.is_empty(),
        v if is_undefined(v) => false,
        _ => true,
    }
}

/// JavaScript `String#trim` whitespace (includes \u{FEFF}).
fn is_js_trim_char(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_trim_char)
}

/// `normalizeKeyToken`: modifier aliases plus `esc`/`space` key names.
fn normalize_key_token(token: &str) -> &str {
    match token {
        "control" => "ctrl",
        "cmd" => "meta",
        "option" => "alt",
        "esc" => "escape",
        "space" => " ",
        other => other,
    }
}

fn is_modifier(token: &str) -> bool {
    matches!(token, "ctrl" | "meta" | "shift" | "alt" | "mod")
}

fn shortcut_value(key: &str, meta: bool, ctrl: bool, shift: bool, alt: bool, r#mod: bool) -> Value {
    json!({
        "key": key,
        "metaKey": meta,
        "ctrlKey": ctrl,
        "shiftKey": shift,
        "altKey": alt,
        "modKey": r#mod,
    })
}

/// Port of `parseKeybindingShortcut` (`packages/shared/src/keybindingRules.ts:31`).
pub fn parse_keybinding_shortcut(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let text = match &value {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'toLowerCase')".to_string());
        }
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string());
        }
        Value::String(s) => s.clone(),
        _ => return Err("value.toLowerCase is not a function".to_string()),
    };
    let s = js_trim(&text.to_lowercase()).to_string();
    let (key_raw, mods): (String, Vec<String>) = if s.ends_with('+') {
        // A trailing '+' is the key itself; the text before it carries the modifiers.
        let mods = s[..s.len() - 1]
            .split('+')
            .map(js_trim)
            .filter(|t| !t.is_empty())
            .map(|t| t.to_string())
            .collect();
        ("+".to_string(), mods)
    } else {
        let mut parts: Vec<String> = s.split('+').map(|t| js_trim(t).to_string()).collect();
        let mut key = parts.pop().unwrap_or_default();
        if key.is_empty() {
            key = "+".to_string();
        }
        (key, parts)
    };
    let key = normalize_key_token(&key_raw).to_string();
    if is_modifier(&key) {
        return Ok(None);
    }
    let (mut meta, mut ctrl, mut shift, mut alt, mut r#mod) = (false, false, false, false, false);
    for token in &mods {
        match normalize_key_token(token) {
            "meta" => meta = true,
            "ctrl" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            "mod" => r#mod = true,
            _ => return Ok(None),
        }
    }
    Ok(Some(shortcut_value(&key, meta, ctrl, shift, alt, r#mod)))
}

// --- when-expression tokenizer/parser ---

#[derive(Clone, Debug)]
enum Tok {
    Bang,
    LParen,
    RParen,
    And,
    Or,
    Ident(String),
}

fn tokenize_when(expr: &str) -> Option<Vec<Tok>> {
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if is_js_trim_char(c) {
            i += 1;
            continue;
        }
        match c {
            '!' => {
                out.push(Tok::Bang);
                i += 1;
            }
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            '&' => {
                if chars.get(i + 1) == Some(&'&') {
                    out.push(Tok::And);
                    i += 2;
                } else {
                    return None;
                }
            }
            '|' => {
                if chars.get(i + 1) == Some(&'|') {
                    out.push(Tok::Or);
                    i += 2;
                } else {
                    return None;
                }
            }
            _ if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                i += 1;
                while i < chars.len() {
                    let d = chars[i];
                    if d.is_ascii_alphanumeric() || d == '_' || d == '.' || d == '-' {
                        i += 1;
                    } else {
                        break;
                    }
                }
                out.push(Tok::Ident(chars[start..i].iter().collect()));
            }
            _ => return None,
        }
    }
    Some(out)
}

enum WhenAst {
    Identifier(String),
    Not(Box<WhenAst>),
    And(Box<WhenAst>, Box<WhenAst>),
    Or(Box<WhenAst>, Box<WhenAst>),
}

impl WhenAst {
    fn to_json(&self) -> Value {
        match self {
            WhenAst::Identifier(name) => json!({"type": "identifier", "name": name}),
            WhenAst::Not(n) => json!({"type": "not", "node": n.to_json()}),
            WhenAst::And(l, r) => {
                json!({"type": "and", "left": l.to_json(), "right": r.to_json()})
            }
            WhenAst::Or(l, r) => json!({"type": "or", "left": l.to_json(), "right": r.to_json()}),
        }
    }
}

struct WhenParser<'a> {
    toks: &'a [Tok],
    pos: usize,
}

impl<'a> WhenParser<'a> {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos)
    }

    fn parse_or(&mut self) -> Option<WhenAst> {
        let mut node = self.parse_and()?;
        while matches!(self.peek(), Some(Tok::Or)) {
            self.pos += 1;
            let right = self.parse_and()?;
            node = WhenAst::Or(Box::new(node), Box::new(right));
        }
        Some(node)
    }

    fn parse_and(&mut self) -> Option<WhenAst> {
        let mut node = self.parse_unary()?;
        while matches!(self.peek(), Some(Tok::And)) {
            self.pos += 1;
            let right = self.parse_unary()?;
            node = WhenAst::And(Box::new(node), Box::new(right));
        }
        Some(node)
    }

    fn parse_unary(&mut self) -> Option<WhenAst> {
        if matches!(self.peek(), Some(Tok::Bang)) {
            self.pos += 1;
            return Some(WhenAst::Not(Box::new(self.parse_unary()?)));
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Option<WhenAst> {
        match self.peek()? {
            Tok::LParen => {
                self.pos += 1;
                let node = self.parse_or()?;
                if matches!(self.peek(), Some(Tok::RParen)) {
                    self.pos += 1;
                    Some(node)
                } else {
                    None
                }
            }
            Tok::Ident(name) => {
                let node = WhenAst::Identifier(name.clone());
                self.pos += 1;
                Some(node)
            }
            _ => None,
        }
    }
}

/// Port of `parseKeybindingWhenExpression` (`packages/shared/src/keybindingRules.ts:145`).
pub fn parse_keybinding_when_expression(
    expression: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let text = match &expression {
        Value::Null => return Err("Cannot read properties of null (reading 'length')".to_string()),
        v if is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'length')".to_string());
        }
        Value::String(s) => s.clone(),
        // `expression.length` is undefined for non-strings: the tokenizer loop
        // runs zero iterations, leaving nothing to parse.
        _ => return Ok(None),
    };
    let toks = match tokenize_when(&text) {
        Some(t) => t,
        None => return Ok(None),
    };
    let mut parser = WhenParser {
        toks: &toks,
        pos: 0,
    };
    match parser.parse_or() {
        Some(ast) if parser.pos == toks.len() => Ok(Some(ast.to_json())),
        _ => Ok(None),
    }
}

/// Port of `compileKeybindingRule` (`packages/shared/src/keybindingRules.ts:231`).
pub fn compile_keybinding_rule(
    rule: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let key = field(&rule, "key")?.clone();
    let shortcut = match parse_keybinding_shortcut(key)? {
        Some(s) => s,
        None => return Ok(None),
    };
    let when = field(&rule, "when")?.clone();
    let mut out = Map::new();
    out.insert("command".to_string(), field(&rule, "command")?.clone());
    out.insert("shortcut".to_string(), shortcut);
    if !is_undefined(&when) {
        match parse_keybinding_when_expression(when)? {
            Some(ast) => {
                out.insert("whenAst".to_string(), ast);
            }
            None => return Ok(None),
        }
    }
    Ok(Some(Value::Object(out)))
}

/// Port of `encodeKeybindingShortcut` (`packages/shared/src/keybindingRules.ts:241`).
pub fn encode_keybinding_shortcut(shortcut: serde_json::Value) -> Result<Option<String>, String> {
    // Modifier flags are read before the key: `shortcut.modKey` throws on
    // null/undefined receivers.
    let read_flag = |name: &str| -> Result<bool, String> {
        match &shortcut {
            Value::Null => Err(prop_read_err(&shortcut, name)),
            v if is_undefined(v) => Err(prop_read_err(&shortcut, name)),
            Value::Object(_) => Ok(truthy(shortcut.get(name).unwrap_or(&Value::Null))),
            _ => Ok(false),
        }
    };
    let mut mods: Vec<&str> = Vec::new();
    if read_flag("modKey")? {
        mods.push("mod");
    }
    if read_flag("metaKey")? {
        mods.push("meta");
    }
    if read_flag("ctrlKey")? {
        mods.push("ctrl");
    }
    if read_flag("altKey")? {
        mods.push("alt");
    }
    if read_flag("shiftKey")? {
        mods.push("shift");
    }
    let key = match &shortcut {
        Value::Object(_) => shortcut.get("key").cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    };
    if !truthy(&key) {
        return Ok(None);
    }
    let key_str = match &key {
        Value::String(s) => s.clone(),
        _ => return Err("shortcut.key.includes is not a function".to_string()),
    };
    mods.push(if key_str == " " { "space" } else { &key_str });
    Ok(Some(mods.join("+")))
}

fn when_type_prec(node: &Value) -> Option<u8> {
    let t = node.get("type")?.as_str()?;
    Some(match t {
        "or" => 0,
        "and" => 1,
        "not" => 2,
        "identifier" => 3,
        _ => return None,
    })
}

/// `encodeKeybindingWhen` inner: returns the raw JS value (string, number,
/// boolean, null, or the undefined marker) or throws like `node.type` does.
fn encode_when_value(node: &Value) -> Result<Value, String> {
    let ty = match node {
        Value::Null => return Err(prop_read_err(node, "type")),
        v if is_undefined(v) => return Err(prop_read_err(node, "type")),
        Value::Object(_) => node.get("type").cloned().unwrap_or(Value::Null),
        _ => return Ok(undef()),
    };
    let ty = match ty.as_str() {
        Some(t) => t,
        None => return Ok(undef()),
    };
    match ty {
        "identifier" => Ok(node.get("name").cloned().unwrap_or_else(undef)),
        "not" => {
            let child = field(node, "node")?;
            let inner = encode_when_value(&child)?;
            let mut text = js_string(&inner);
            if when_type_prec(&child).is_some_and(|p| p < 2) {
                text = format!("({text})");
            }
            Ok(Value::String(format!("!{text}")))
        }
        "and" | "or" => {
            let prec = if ty == "and" { 1 } else { 0 };
            let sep = if ty == "and" { " && " } else { " || " };
            let mut sides = Vec::new();
            for (idx, side) in ["left", "right"].iter().enumerate() {
                let child = field(node, side)?;
                let mut text = match encode_when_value(&child)? {
                    v if is_undefined(&v) || v.is_null() => String::new(),
                    v => js_string(&v),
                };
                let cp = when_type_prec(&child);
                if cp.is_some_and(|p| p < prec || (p == prec && idx == 1)) {
                    text = format!("({text})");
                }
                sides.push(text);
            }
            Ok(Value::String(sides.join(sep)))
        }
        _ => Ok(undef()),
    }
}

/// Port of `encodeKeybindingWhen` (`packages/shared/src/keybindingRules.ts:266`).
pub fn encode_keybinding_when(node: serde_json::Value) -> Result<serde_json::Value, String> {
    encode_when_value(&node)
}

/// Port of `encodeKeybindingRule` (`packages/shared/src/keybindingRules.ts:286`).
pub fn encode_keybinding_rule(
    rule: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let shortcut = field(&rule, "shortcut")?;
    let _ = field(&shortcut, "key")?;
    let key = match encode_keybinding_shortcut(shortcut)? {
        Some(k) => k,
        None => return Ok(None),
    };
    let mut out = Map::new();
    out.insert("command".to_string(), field(&rule, "command")?.clone());
    out.insert("key".to_string(), Value::String(key));
    let when_ast = field(&rule, "whenAst")?;
    if truthy(&when_ast) {
        out.insert("when".to_string(), encode_when_value(&when_ast)?);
    }
    Ok(Some(Value::Object(out)))
}

/// Port of `resolvedKeybindingRuleIdentity` (`packages/shared/src/keybindingRules.ts:302`).
pub fn resolved_keybinding_rule_identity(rule: serde_json::Value) -> Result<String, String> {
    let shortcut = field(&rule, "shortcut")?;
    let _ = field(&shortcut, "key")?;
    let encoded = encode_keybinding_shortcut(shortcut)?.unwrap_or_default();
    let command = js_string(&field(&rule, "command")?);
    let when_ast = field(&rule, "whenAst")?;
    let when = if truthy(&when_ast) {
        js_string(&encode_when_value(&when_ast)?)
    } else {
        String::new()
    };
    Ok(format!("{command}\u{0}{encoded}\u{0}{when}"))
}

/// Port of `keybindingRuleIdentity` (`packages/shared/src/keybindingRules.ts:312`).
pub fn keybinding_rule_identity(rule: serde_json::Value) -> Result<Option<String>, String> {
    match compile_keybinding_rule(rule)? {
        Some(compiled) => Ok(Some(resolved_keybinding_rule_identity(compiled)?)),
        None => Ok(None),
    }
}

/// `value === 'unassigned'`, after a `value.toLowerCase()` call for error parity.
fn key_is_unassigned(owner: &Value) -> Result<bool, String> {
    let key = field(owner, "key")?;
    if key.is_null() {
        return Ok(false);
    }
    match &key {
        Value::String(s) => {
            let _ = s.to_lowercase();
            Ok(s == "unassigned")
        }
        v if is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'toLowerCase')".to_string())
        }
        _ => Err("value.toLowerCase is not a function".to_string()),
    }
}

/// Port of `isUnassignedKeybindingShortcut` (`packages/shared/src/keybindingRules.ts:322`).
pub fn is_unassigned_keybinding_shortcut(owner: Value) -> Result<bool, String> {
    if owner.is_null() {
        return Err("Cannot read properties of null (reading 'key')".to_string());
    }
    if is_undefined(&owner) {
        return Err("Cannot read properties of undefined (reading 'key')".to_string());
    }
    let key = match &owner {
        Value::Object(m) => m.get("key"),
        _ => None,
    };
    Ok(matches!(key, Some(Value::String(s)) if s == "unassigned"))
}

/// Port of `isUnassignedKeybindingRule` (`packages/shared/src/keybindingRules.ts:327`).
pub fn is_unassigned_keybinding_rule(rule: serde_json::Value) -> Result<bool, String> {
    key_is_unassigned(&rule)
}
