//! The Rust port of `apps/web/src/storeNormalization.ts`.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};
use std::borrow::Cow;
use std::cmp::Ordering;
use std::sync::OnceLock;

use crate::corpus;
use crate::js;

// ---------------------------------------------------------------------------
// JS value model
// ---------------------------------------------------------------------------

static UNDEF_CELL: OnceLock<Value> = OnceLock::new();

fn undef() -> &'static Value {
    UNDEF_CELL.get_or_init(|| json!({ "$undefined": true }))
}

fn undefined() -> Value {
    js::undefined()
}

/// The JS category of a transported value. Objects with a single known marker
/// key (`{"$num": …}` …) classify as the decoded primitive or exotic builtin.
#[derive(Clone, Copy, PartialEq, Eq)]
enum K {
    Und,
    Null,
    Bool,
    Num,
    Str,
    Big,
    Arr,
    Map_,
    Set_,
    Date,
    Bytes,
    Buffer,
    Obj,
}

fn kind(v: &Value) -> K {
    match v {
        Value::Null => K::Null,
        Value::Bool(_) => K::Bool,
        Value::Number(_) => K::Num,
        Value::String(_) => K::Str,
        Value::Array(_) => K::Arr,
        Value::Object(o) => {
            if o.len() == 1 {
                if o.contains_key("$undefined") {
                    return K::Und;
                }
                if o.contains_key("$num") {
                    return K::Num;
                }
                if o.contains_key("$utf16") {
                    return K::Str;
                }
                if o.contains_key("$bigint") {
                    return K::Big;
                }
                if o.contains_key("$map") {
                    return K::Map_;
                }
                if o.contains_key("$set") {
                    return K::Set_;
                }
                if o.contains_key("$date") {
                    return K::Date;
                }
                if o.contains_key("$bytes") {
                    return K::Bytes;
                }
                if o.contains_key("$buffer") {
                    return K::Buffer;
                }
            }
            K::Obj
        }
    }
}

fn is_undef(v: &Value) -> bool {
    kind(v) == K::Und
}

fn is_nullish(v: &Value) -> bool {
    matches!(kind(v), K::Und | K::Null)
}

fn is_object(v: &Value) -> bool {
    matches!(
        kind(v),
        K::Obj | K::Arr | K::Map_ | K::Set_ | K::Date | K::Bytes | K::Buffer
    )
}

fn sole<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    if let Value::Object(o) = v
        && o.len() == 1
    {
        return o.get(key);
    }
    None
}

fn inner(v: &Value, key: &str) -> Value {
    sole(v, key).cloned().unwrap_or(Value::Null)
}

fn num(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::Object(o) if o.len() == 1 => o.get("$num").map(to_number),
        _ => None,
    }
}

/// UTF-16 code units of a JS string value, `{"$utf16": …}` included.
fn units(v: &Value) -> Vec<u16> {
    match v {
        Value::String(s) => s.encode_utf16().collect(),
        Value::Object(o) if o.len() == 1 => match o.get("$utf16") {
            Some(Value::Array(a)) => a.iter().map(|x| to_u16(to_number(x))).collect(),
            Some(Value::String(s)) => vec![0u16; s.encode_utf16().count()],
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

fn to_u16(x: f64) -> u16 {
    if !x.is_finite() {
        return 0;
    }
    ((x as i64) & 0xFFFF) as u16
}

fn lossy(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

fn str_of(v: &Value) -> String {
    lossy(&units(v))
}

fn utf16_val(u: &[u16]) -> Value {
    js::utf16_string(u)
}

fn str_val(s: &str) -> Value {
    Value::String(s.to_string())
}

fn num_val(x: f64) -> Value {
    js::number(x)
}

// ---------------------------------------------------------------------------
// JS primitive coercions
// ---------------------------------------------------------------------------

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '\u{b}'
            | '\u{c}'
            | '\u{a0}'
            | '\u{feff}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}' | '\u{202f}' | '\u{205f}' | '\u{3000}'
    )
}

fn js_trim_str(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

fn trim_units(u: &[u16]) -> Vec<u16> {
    lossy(u).trim_matches(is_js_ws).encode_utf16().collect()
}

fn lower_units(u: &[u16]) -> Vec<u16> {
    lossy(u).to_lowercase().encode_utf16().collect()
}

fn u16_cmp(a: &[u16], b: &[u16]) -> Ordering {
    a.iter().cmp(b.iter())
}

fn u16s(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn u16_starts(u: &[u16], p: &[u16]) -> bool {
    u.starts_with(p)
}

fn u16_ends(u: &[u16], p: &[u16]) -> bool {
    u.ends_with(p)
}

fn u16_contains(u: &[u16], p: &[u16]) -> bool {
    if p.is_empty() {
        return true;
    }
    u.windows(p.len()).any(|w| w == p)
}

fn u16_slice(u: &[u16], start: i64, end: i64) -> Vec<u16> {
    let n = u.len() as i64;
    let s = if start < 0 {
        (n + start).max(0)
    } else {
        start.min(n)
    };
    let e = if end < 0 {
        (n + end).max(0)
    } else {
        end.min(n)
    };
    if e <= s {
        Vec::new()
    } else {
        u[s as usize..e as usize].to_vec()
    }
}

fn str_to_number(s: &str) -> f64 {
    let t = js_trim_str(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let body = t.strip_prefix('+').unwrap_or(t);
    let (neg, body) = match body.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, body),
    };
    let radix = body
        .strip_prefix("0x")
        .or_else(|| body.strip_prefix("0X"))
        .map(|h| (h, 16))
        .or_else(|| {
            body.strip_prefix("0o")
                .or_else(|| body.strip_prefix("0O"))
                .map(|h| (h, 8))
        })
        .or_else(|| {
            body.strip_prefix("0b")
                .or_else(|| body.strip_prefix("0B"))
                .map(|h| (h, 2))
        });
    if let Some((digits, base)) = radix {
        if digits.is_empty() {
            return f64::NAN;
        }
        let mut acc = 0f64;
        for c in digits.chars() {
            let Some(d) = c.to_digit(base) else {
                return f64::NAN;
            };
            acc = acc * base as f64 + d as f64;
        }
        return if neg { -acc } else { acc };
    }
    if t.chars()
        .any(|c| c.is_ascii_alphabetic() && !matches!(c, 'e' | 'E'))
    {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn to_number(v: &Value) -> f64 {
    match kind(v) {
        K::Und => f64::NAN,
        K::Null => 0.0,
        K::Bool => {
            if v == &Value::Bool(true) {
                1.0
            } else {
                0.0
            }
        }
        K::Num => num(v).unwrap_or(f64::NAN),
        K::Str => str_to_number(&lossy(&units(v))),
        K::Big => to_number(&inner(v, "$bigint")),
        K::Arr => str_to_number(&join_string(v)),
        K::Date => {
            let inner = inner(v, "$date");
            corpus::date_parse(&to_string_js(&inner))
        }
        K::Bytes | K::Buffer => str_to_number(&join_string(v)),
        _ => f64::NAN,
    }
}

fn join_string(v: &Value) -> String {
    match v {
        Value::Array(a) => a
            .iter()
            .map(|e| {
                if is_nullish(e) {
                    String::new()
                } else {
                    to_string_js(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(o) if o.len() == 1 => {
            if let Some(inner) = o.get("$bytes").or_else(|| o.get("$buffer"))
                && let Value::Array(a) = inner
            {
                return a
                    .iter()
                    .map(|e| js::number_to_string(to_number(e)))
                    .collect::<Vec<_>>()
                    .join(",");
            }
            "[object Object]".to_string()
        }
        _ => String::new(),
    }
}

fn to_string_js(v: &Value) -> String {
    match kind(v) {
        K::Und => "undefined".into(),
        K::Null => "null".into(),
        K::Bool => {
            if v == &Value::Bool(true) {
                "true".into()
            } else {
                "false".into()
            }
        }
        K::Num => js::number_to_string(num(v).unwrap_or(f64::NAN)),
        K::Str => str_of(v),
        K::Big => to_string_js(&inner(v, "$bigint")),
        K::Arr | K::Bytes | K::Buffer => join_string(v),
        K::Map_ => "[object Map]".into(),
        K::Set_ => "[object Set]".into(),
        K::Date => "[object Date]".into(),
        K::Obj => "[object Object]".into(),
    }
}

fn to_property_key(v: &Value) -> String {
    to_string_js(v)
}

fn truthy(v: &Value) -> bool {
    match kind(v) {
        K::Und | K::Null => false,
        K::Bool => v == &Value::Bool(true),
        K::Num => num(v).is_some_and(|x| x != 0.0 && !x.is_nan()),
        K::Str => !units(v).is_empty(),
        K::Big => to_number(&inner(v, "$bigint")) != 0.0,
        _ => true,
    }
}

fn typeof_js(v: &Value) -> &'static str {
    match kind(v) {
        K::Und => "undefined",
        K::Null => "object",
        K::Bool => "boolean",
        K::Num => "number",
        K::Str => "string",
        K::Big => "bigint",
        _ => "object",
    }
}

// ---------------------------------------------------------------------------
// JS equality and relational semantics
// ---------------------------------------------------------------------------

fn prim_eq(a: &Value, b: &Value) -> bool {
    match (kind(a), kind(b)) {
        (K::Und, K::Und) | (K::Null, K::Null) => true,
        (K::Bool, K::Bool) => a == b,
        (K::Num, K::Num) => match (num(a), num(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
        (K::Str, K::Str) => units(a) == units(b),
        (K::Big, K::Big) => {
            to_string_js(&inner(a, "$bigint")) == to_string_js(&inner(b, "$bigint"))
        }
        _ => false,
    }
}

fn strict_eq(a: &Value, b: &Value) -> bool {
    let (ka, kb) = (kind(a), kind(b));
    let (oa, ob) = (is_object(a), is_object(b));
    if oa && ob {
        return std::ptr::eq(a, b);
    }
    if oa != ob {
        return false;
    }
    let _ = (ka, kb);
    prim_eq(a, b)
}

fn svz_eq(a: &Value, b: &Value) -> bool {
    if is_object(a) || is_object(b) {
        return std::ptr::eq(a, b);
    }
    if let (Some(x), Some(y)) = (num(a), num(b))
        && kind(a) == K::Num
        && kind(b) == K::Num
        && x.is_nan()
        && y.is_nan()
    {
        return true;
    }
    prim_eq(a, b)
}

fn object_is(a: &Value, b: &Value) -> bool {
    if is_object(a) || is_object(b) {
        return std::ptr::eq(a, b);
    }
    if kind(a) != kind(b) {
        return false;
    }
    if kind(a) == K::Num {
        let (Some(x), Some(y)) = (num(a), num(b)) else {
            return false;
        };
        if x.is_nan() && y.is_nan() {
            return true;
        }
        if x == 0.0 && y == 0.0 {
            return x.is_sign_negative() == y.is_sign_negative();
        }
        return x == y;
    }
    prim_eq(a, b)
}

fn to_primitive(v: &Value) -> Value {
    match kind(v) {
        K::Arr | K::Obj | K::Map_ | K::Set_ | K::Bytes | K::Buffer | K::Date => {
            str_val(&to_string_js(v))
        }
        _ => v.clone(),
    }
}

fn js_add(a: &Value, b: &Value) -> Value {
    let (pa, pb) = (to_primitive(a), to_primitive(b));
    if kind(&pa) == K::Str || kind(&pb) == K::Str {
        return utf16_val(&[&units(&pa)[..], &units(&pb)[..]].concat());
    }
    num_val(to_number(&pa) + to_number(&pb))
}

fn js_cmp(a: &Value, b: &Value) -> Option<Ordering> {
    let (pa, pb) = (to_primitive(a), to_primitive(b));
    if kind(&pa) == K::Str && kind(&pb) == K::Str {
        return Some(u16_cmp(&units(&pa), &units(&pb)));
    }
    let (x, y) = (to_number(&pa), to_number(&pb));
    if x.is_nan() || y.is_nan() {
        return None;
    }
    x.partial_cmp(&y)
}

fn js_lt(a: &Value, b: &Value) -> bool {
    js_cmp(a, b) == Some(Ordering::Less)
}

fn js_gt(a: &Value, b: &Value) -> bool {
    js_cmp(a, b) == Some(Ordering::Greater)
}

fn js_ge(a: &Value, b: &Value) -> bool {
    matches!(js_cmp(a, b), Some(Ordering::Greater | Ordering::Equal))
}

fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

// ---------------------------------------------------------------------------
// Property access
// ---------------------------------------------------------------------------

fn not_object(v: &Value, expr: &str) -> String {
    if v.is_null() {
        format!("null is not an object (evaluating '{expr}')")
    } else {
        format!("undefined is not an object (evaluating '{expr}')")
    }
}

fn prop<'a>(v: &'a Value, key: &str, expr: &str) -> Result<Cow<'a, Value>, String> {
    if is_nullish(v) {
        return Err(not_object(v, expr));
    }
    Ok(prop_raw(v, key))
}

fn prop_opt<'a>(v: &'a Value, key: &str) -> Cow<'a, Value> {
    if is_nullish(v) {
        Cow::Borrowed(undef())
    } else {
        prop_raw(v, key)
    }
}

fn array_index_key(key: &str) -> Option<usize> {
    if key.is_empty() || key.len() > 10 {
        return None;
    }
    let i: u64 = key.parse().ok()?;
    if i < u32::MAX as u64 && i.to_string() == key {
        Some(i as usize)
    } else {
        None
    }
}

fn prop_raw<'a>(v: &'a Value, key: &str) -> Cow<'a, Value> {
    match v {
        Value::Object(o) => {
            if o.len() == 1 {
                if let Some(inner) = o.get("$map").or_else(|| o.get("$set")) {
                    if key == "size"
                        && let Value::Array(a) = inner
                    {
                        return Cow::Owned(num_val(a.len() as f64));
                    }
                    return Cow::Borrowed(undef());
                }
                if let Some(inner) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                    if let Value::Array(a) = inner {
                        match key {
                            "length" | "byteLength" => {
                                return Cow::Owned(num_val(a.len() as f64));
                            }
                            "byteOffset" => return Cow::Owned(num_val(0.0)),
                            _ => {
                                if let Some(i) = array_index_key(key)
                                    && i < a.len()
                                {
                                    return Cow::Borrowed(&a[i]);
                                }
                            }
                        }
                    }
                    return Cow::Borrowed(undef());
                }
                if let Some(Value::Array(_)) = o.get("$utf16") {
                    return match key {
                        "length" => Cow::Owned(num_val(units(v).len() as f64)),
                        _ => match array_index_key(key) {
                            Some(i) => match u16_slice(&units(v), i as i64, i as i64 + 1) {
                                u if !u.is_empty() => Cow::Owned(utf16_val(&u)),
                                _ => Cow::Borrowed(undef()),
                            },
                            _ => Cow::Borrowed(undef()),
                        },
                    };
                }
            }
            Cow::Borrowed(o.get(key).unwrap_or(undef()))
        }
        Value::Array(a) => match key {
            "length" => Cow::Owned(num_val(a.len() as f64)),
            _ => match array_index_key(key) {
                Some(i) if i < a.len() => Cow::Borrowed(&a[i]),
                _ => Cow::Borrowed(undef()),
            },
        },
        _ => match kind(v) {
            K::Str => match key {
                "length" => Cow::Owned(num_val(units(v).len() as f64)),
                _ => match array_index_key(key) {
                    Some(i) => match u16_slice(&units(v), i as i64, i as i64 + 1) {
                        u if !u.is_empty() => Cow::Owned(utf16_val(&u)),
                        _ => Cow::Borrowed(undef()),
                    },
                    _ => Cow::Borrowed(undef()),
                },
            },
            _ => Cow::Borrowed(undef()),
        },
    }
}

fn idx<'a>(v: &'a Value, i: f64, expr: &str) -> Result<Cow<'a, Value>, String> {
    if is_nullish(v) {
        return Err(not_object(v, expr));
    }
    Ok(idx_opt(v, i))
}

fn idx_opt<'a>(v: &'a Value, i: f64) -> Cow<'a, Value> {
    if is_nullish(v) {
        return Cow::Borrowed(undef());
    }
    let key = if i == i.trunc() && i >= 0.0 {
        format!("{}", i as i64)
    } else {
        js::number_to_string(i)
    };
    prop_raw(v, &key)
}

fn obj_key_get<'a>(v: &'a Value, key: &str, expr: &str) -> Result<Cow<'a, Value>, String> {
    prop(v, key, expr)
}

const OBJ_PROTO: &[&str] = &[
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
    "constructor",
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
];

const ARRAY_METHODS: &[&str] = &[
    "at",
    "concat",
    "copyWithin",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "flat",
    "flatMap",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "lastIndexOf",
    "map",
    "pop",
    "push",
    "reduce",
    "reduceRight",
    "reverse",
    "shift",
    "slice",
    "some",
    "sort",
    "splice",
    "toReversed",
    "toSorted",
    "toSpliced",
    "unshift",
    "values",
    "with",
];

const STRING_METHODS: &[&str] = &[
    "anchor",
    "at",
    "big",
    "blink",
    "bold",
    "charAt",
    "charCodeAt",
    "codePointAt",
    "concat",
    "endsWith",
    "fixed",
    "fontcolor",
    "fontsize",
    "includes",
    "indexOf",
    "isWellFormed",
    "italics",
    "lastIndexOf",
    "link",
    "localeCompare",
    "match",
    "matchAll",
    "normalize",
    "padEnd",
    "padStart",
    "repeat",
    "replace",
    "replaceAll",
    "search",
    "slice",
    "small",
    "split",
    "startsWith",
    "strike",
    "sub",
    "substr",
    "substring",
    "sup",
    "toLocaleLowerCase",
    "toLocaleUpperCase",
    "toLowerCase",
    "toUpperCase",
    "toWellFormed",
    "trim",
    "trimEnd",
    "trimStart",
];

const NUMBER_METHODS: &[&str] = &["toExponential", "toFixed", "toLocaleString", "toPrecision"];

const BOOL_METHODS: &[&str] = &[];
const BIGINT_METHODS: &[&str] = &[];

const MAP_METHODS: &[&str] = &[
    "clear", "delete", "entries", "forEach", "get", "has", "keys", "set", "values",
];

const SET_METHODS: &[&str] = &[
    "add", "clear", "delete", "entries", "forEach", "has", "keys", "values",
];

const TYPED_ARRAY_METHODS: &[&str] = &[
    "at",
    "copyWithin",
    "entries",
    "every",
    "fill",
    "filter",
    "find",
    "findIndex",
    "findLast",
    "findLastIndex",
    "forEach",
    "includes",
    "indexOf",
    "join",
    "keys",
    "lastIndexOf",
    "map",
    "reduce",
    "reduceRight",
    "reverse",
    "set",
    "slice",
    "some",
    "sort",
    "subarray",
    "toLocaleString",
    "toReversed",
    "toSorted",
    "toSpliced",
    "values",
    "with",
];

const DATE_METHODS: &[&str] = &[
    "getDate",
    "getDay",
    "getFullYear",
    "getHours",
    "getMilliseconds",
    "getMinutes",
    "getMonth",
    "getSeconds",
    "getTime",
    "getTimezoneOffset",
    "getUTCDate",
    "getUTCDay",
    "getUTCFullYear",
    "getUTCHours",
    "getUTCMilliseconds",
    "getUTCMinutes",
    "getUTCMonth",
    "getUTCSeconds",
    "getYear",
    "setDate",
    "setFullYear",
    "setHours",
    "setMilliseconds",
    "setMinutes",
    "setMonth",
    "setSeconds",
    "setTime",
    "setUTCDate",
    "setUTCFullYear",
    "setUTCHours",
    "setUTCMilliseconds",
    "setUTCMinutes",
    "setUTCMonth",
    "setUTCSeconds",
    "setYear",
    "toDateString",
    "toGMTString",
    "toISOString",
    "toJSON",
    "toLocaleDateString",
    "toLocaleString",
    "toLocaleTimeString",
    "toTimeString",
    "toUTCString",
];

fn method_known(v: &Value, name: &str) -> bool {
    OBJ_PROTO.contains(&name)
        || match kind(v) {
            K::Arr => ARRAY_METHODS.contains(&name),
            K::Str => STRING_METHODS.contains(&name),
            K::Num => NUMBER_METHODS.contains(&name),
            K::Bool => BOOL_METHODS.contains(&name),
            K::Big => BIGINT_METHODS.contains(&name),
            K::Map_ => MAP_METHODS.contains(&name),
            K::Set_ => SET_METHODS.contains(&name),
            K::Bytes | K::Buffer => TYPED_ARRAY_METHODS.contains(&name),
            K::Date => DATE_METHODS.contains(&name),
            _ => false,
        }
}

fn callable_at(v: &Value, name: &str) -> bool {
    if let Value::Object(o) = v
        && (o.len() != 1 || kind(v) == K::Obj)
        && o.contains_key(name)
    {
        return false;
    }
    method_known(v, name)
}

fn describe(v: &Value) -> String {
    match kind(v) {
        K::Und => "undefined".into(),
        K::Null => "null".into(),
        K::Bool => {
            if v == &Value::Bool(true) {
                "true".into()
            } else {
                "false".into()
            }
        }
        K::Num => js::number_to_string(num(v).unwrap_or(f64::NAN)),
        K::Str => format!("\"{}\"", str_of(v)),
        K::Big => to_string_js(&inner(v, "$bigint")),
        K::Arr => "an instance of Array".into(),
        K::Obj => "an instance of Object".into(),
        K::Map_ => "an instance of Map".into(),
        K::Set_ => "an instance of Set".into(),
        K::Date => "an instance of Date".into(),
        K::Bytes => "an instance of Uint8Array".into(),
        K::Buffer => "an instance of Buffer".into(),
    }
}

fn not_a_function(member_expr: &str, call_expr: &str, callee: &Value) -> String {
    if is_undef(callee) {
        format!("{member_expr} is not a function. (In '{call_expr}', '{member_expr}' is undefined)")
    } else {
        format!(
            "{member_expr} is not a function. (In '{call_expr}', '{member_expr}' is {})",
            describe(callee)
        )
    }
}

// ---------------------------------------------------------------------------
// Iteration, spread, object introspection
// ---------------------------------------------------------------------------

fn iter<'a>(v: &'a Value, bind: &str, expr: &str) -> Result<Vec<Cow<'a, Value>>, String> {
    match v {
        Value::Null => Err(format!(
            "null is not an object (evaluating '{bind} of {expr}')"
        )),
        Value::Array(a) => Ok(a.iter().map(Cow::Borrowed).collect()),
        Value::Object(o) if o.len() == 1 => {
            if o.contains_key("$undefined") {
                return Err(format!(
                    "undefined is not an object (evaluating '{bind} of {expr}')"
                ));
            }
            if let Some(Value::Array(a)) = o.get("$map") {
                return Ok(a.iter().map(Cow::Borrowed).collect());
            }
            if let Some(Value::Array(a)) = o.get("$set") {
                return Ok(a.iter().map(Cow::Borrowed).collect());
            }
            if let Some(inner) = o.get("$bytes").or_else(|| o.get("$buffer"))
                && let Value::Array(a) = inner
            {
                return Ok(a.iter().map(Cow::Borrowed).collect());
            }
            if o.contains_key("$utf16") {
                let u = units(v);
                let mut out = Vec::with_capacity(u.len());
                let mut i = 0;
                while i < u.len() {
                    let n = if (0xD800..0xDC00).contains(&u[i])
                        && i + 1 < u.len()
                        && (0xDC00..0xE000).contains(&u[i + 1])
                    {
                        2
                    } else {
                        1
                    };
                    out.push(Cow::Owned(utf16_val(&u[i..i + n])));
                    i += n;
                }
                return Ok(out);
            }
            Err("{} is not iterable".to_string())
        }
        Value::Object(_) => Err("{} is not iterable".to_string()),
        Value::String(_) => {
            let u = units(v);
            let mut out = Vec::with_capacity(u.len());
            let mut i = 0;
            while i < u.len() {
                let n = if (0xD800..0xDC00).contains(&u[i])
                    && i + 1 < u.len()
                    && (0xDC00..0xE000).contains(&u[i + 1])
                {
                    2
                } else {
                    1
                };
                out.push(Cow::Owned(utf16_val(&u[i..i + n])));
                i += n;
            }
            Ok(out)
        }
        _ => match kind(v) {
            K::Und => Err(format!(
                "undefined is not an object (evaluating '{bind} of {expr}')"
            )),
            K::Num => Err("number is not iterable".to_string()),
            K::Bool => Err(format!(
                "{} is not iterable",
                if v == &Value::Bool(true) {
                    "true"
                } else {
                    "false"
                }
            )),
            _ => Err("value is not iterable".to_string()),
        },
    }
}

fn spread_iter<'a>(v: &'a Value) -> Result<Vec<Cow<'a, Value>>, String> {
    match kind(v) {
        K::Und | K::Null => {
            Err("Spread syntax requires ...iterable not be null or undefined".to_string())
        }
        K::Arr | K::Str | K::Map_ | K::Set_ | K::Bytes | K::Buffer => iter(v, "", ""),
        _ => {
            Err("Spread syntax requires ...iterable[Symbol.iterator] to be a function".to_string())
        }
    }
}

fn spread_obj(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Object(o) if o.len() != 1 || kind(v) == K::Obj => {
            o.iter().map(|(k, x)| (k.clone(), x.clone())).collect()
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        _ => match kind(v) {
            K::Str => {
                let u = units(v);
                let mut out = Vec::with_capacity(u.len());
                let mut i = 0;
                while i < u.len() {
                    let n = if (0xD800..0xDC00).contains(&u[i])
                        && i + 1 < u.len()
                        && (0xDC00..0xE000).contains(&u[i + 1])
                    {
                        2
                    } else {
                        1
                    };
                    out.push((out.len().to_string(), utf16_val(&u[i..i + n])));
                    i += n;
                }
                out
            }
            K::Bytes | K::Buffer => match sole(
                v,
                if kind(v) == K::Bytes {
                    "$bytes"
                } else {
                    "$buffer"
                },
            ) {
                Some(Value::Array(a)) => a
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (i.to_string(), x.clone()))
                    .collect(),
                _ => Vec::new(),
            },
            _ => Vec::new(),
        },
    }
}

fn object_keys(v: &Value) -> Vec<String> {
    match kind(v) {
        K::Obj => match v {
            Value::Object(o) => o.keys().cloned().collect(),
            _ => Vec::new(),
        },
        K::Arr => (0..v.as_array().map_or(0, |a| a.len()))
            .map(|i| i.to_string())
            .collect(),
        K::Str => (0..units(v).len()).map(|i| i.to_string()).collect(),
        K::Bytes | K::Buffer => (0..byte_len(v)).map(|i| i.to_string()).collect(),
        _ => Vec::new(),
    }
}

fn byte_len(v: &Value) -> usize {
    match sole(
        v,
        if kind(v) == K::Bytes {
            "$bytes"
        } else {
            "$buffer"
        },
    ) {
        Some(Value::Array(a)) => a.len(),
        _ => 0,
    }
}

fn has_in(v: &Value, key: &str) -> bool {
    match kind(v) {
        K::Obj => match v {
            Value::Object(o) => o.contains_key(key) || OBJ_PROTO.contains(&key),
            _ => false,
        },
        K::Arr => {
            array_index_key(key).is_some_and(|i| i < v.as_array().map_or(0, |a| a.len()))
                || key == "length"
                || ARRAY_METHODS.contains(&key)
                || OBJ_PROTO.contains(&key)
        }
        K::Str => {
            array_index_key(key).is_some_and(|i| i < units(v).len())
                || key == "length"
                || STRING_METHODS.contains(&key)
                || OBJ_PROTO.contains(&key)
        }
        K::Num => {
            NUMBER_METHODS.contains(&key)
                || OBJ_PROTO.contains(&key)
                || key == "toString"
                || key == "valueOf"
        }
        K::Bool => key == "toString" || key == "valueOf" || OBJ_PROTO.contains(&key),
        K::Big => key == "toString" || key == "valueOf" || OBJ_PROTO.contains(&key),
        K::Map_ => MAP_METHODS.contains(&key) || key == "size" || OBJ_PROTO.contains(&key),
        K::Set_ => SET_METHODS.contains(&key) || key == "size" || OBJ_PROTO.contains(&key),
        K::Bytes | K::Buffer => {
            array_index_key(key).is_some_and(|i| i < byte_len(v))
                || matches!(key, "length" | "byteLength" | "byteOffset" | "buffer")
                || TYPED_ARRAY_METHODS.contains(&key)
                || OBJ_PROTO.contains(&key)
        }
        K::Date => DATE_METHODS.contains(&key) || OBJ_PROTO.contains(&key),
        _ => false,
    }
}

fn object_has_own(v: &Value, key: &str, expr: &str) -> Result<bool, String> {
    if is_nullish(v) {
        return Err(not_object(v, expr));
    }
    Ok(match kind(v) {
        K::Obj => v.as_object().is_some_and(|o| o.contains_key(key)),
        K::Arr => array_index_key(key).is_some_and(|i| i < v.as_array().map_or(0, |a| a.len())),
        K::Str => array_index_key(key).is_some_and(|i| i < units(v).len()),
        K::Bytes | K::Buffer => array_index_key(key).is_some_and(|i| i < byte_len(v)),
        _ => false,
    })
}

// ---------------------------------------------------------------------------
// JS sorting (mirrors js::sort but propagating errors)
// ---------------------------------------------------------------------------

fn js_sort_res<T>(
    items: &mut [T],
    compare: &mut impl FnMut(&T, &T) -> Result<f64, String>,
) -> Result<(), String> {
    let n = items.len();
    if n < 2 {
        return Ok(());
    }
    if n >= 64 {
        let mut err = None;
        items.sort_by(|a, b| {
            if err.is_some() {
                return Ordering::Equal;
            }
            match compare(a, b) {
                Ok(x) => x.partial_cmp(&0.0).unwrap_or(Ordering::Equal),
                Err(e) => {
                    err = Some(e);
                    Ordering::Equal
                }
            }
        });
        if let Some(e) = err {
            return Err(e);
        }
        return Ok(());
    }
    let mut run = 2;
    let descending = compare(&items[1], &items[0])? < 0.0;
    while run < n {
        let order = compare(&items[run], &items[run - 1])?;
        if (descending && order >= 0.0) || (!descending && order < 0.0) {
            break;
        }
        run += 1;
    }
    if descending {
        items[..run].reverse();
    }
    for start in run..n {
        let (mut left, mut right) = (0, start);
        while left < right {
            let mid = left + ((right - left) >> 1);
            if compare(&items[start], &items[mid])? < 0.0 {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        items[left..=start].rotate_right(1);
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// JS arrays/collections
// ---------------------------------------------------------------------------

struct JsMap<'a> {
    entries: Vec<(Value, Cow<'a, Value>)>,
}

impl<'a> JsMap<'a> {
    fn new() -> Self {
        JsMap {
            entries: Vec::new(),
        }
    }
    fn get<'s>(&'s self, k: &Value) -> Option<&'s Value> {
        self.entries
            .iter()
            .find(|(ek, _)| svz_eq(ek, k))
            .map(|(_, v)| v.as_ref())
    }
    fn set(&mut self, k: Value, v: Cow<'a, Value>) {
        if let Some(slot) = self.entries.iter_mut().find(|(ek, _)| svz_eq(ek, &k)) {
            slot.1 = v;
        } else {
            self.entries.push((k, v));
        }
    }
    fn has(&self, k: &Value) -> bool {
        self.entries.iter().any(|(ek, _)| svz_eq(ek, k))
    }
    fn delete(&mut self, k: &Value) -> bool {
        if let Some(i) = self.entries.iter().position(|(ek, _)| svz_eq(ek, k)) {
            self.entries.remove(i);
            true
        } else {
            false
        }
    }
    fn size(&self) -> usize {
        self.entries.len()
    }
    fn values(&self) -> impl Iterator<Item = &Cow<'a, Value>> {
        self.entries.iter().map(|(_, v)| v)
    }
}

struct JsSet {
    items: Vec<Value>,
}

impl JsSet {
    fn new() -> Self {
        JsSet { items: Vec::new() }
    }
    fn has(&self, v: &Value) -> bool {
        self.items.iter().any(|x| svz_eq(x, v))
    }
    fn add(&mut self, v: Value) {
        if !self.has(&v) {
            self.items.push(v);
        }
    }
    fn delete(&mut self, v: &Value) -> bool {
        if let Some(i) = self.items.iter().position(|x| svz_eq(x, v)) {
            self.items.remove(i);
            true
        } else {
            false
        }
    }
}

fn map_of_pairs<'a>(pairs: Vec<Cow<'a, Value>>) -> JsMap<'a> {
    let mut m = JsMap::new();
    for pair in pairs {
        let k = idx_opt(pair.as_ref(), 0.0).into_owned();
        let val = idx_opt(pair.as_ref(), 1.0).into_owned();
        m.set(k, Cow::Owned(val));
    }
    m
}

fn map_call<'a, F>(
    v: &'a Value,
    mexpr: &str,
    cexpr: &str,
    f: &mut F,
) -> Result<Vec<Cow<'a, Value>>, String>
where
    F: FnMut(Cow<'a, Value>, usize) -> Result<Cow<'a, Value>, String>,
{
    let callee = prop(v, "map", mexpr)?;
    if !(is_undef(callee.as_ref()) && callable_at(v, "map")) {
        return Err(not_a_function(mexpr, cexpr, callee.as_ref()));
    }
    match v {
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, e)| f(Cow::Borrowed(e), i))
            .collect(),
        Value::Object(o) if o.len() == 1 => {
            if let Some(Value::Array(a)) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                return a
                    .iter()
                    .enumerate()
                    .map(|(i, e)| f(Cow::Borrowed(e), i))
                    .collect();
            }
            Ok(Vec::new())
        }
        _ => Ok(Vec::new()),
    }
}

fn opt_map_call<'a, F>(
    v: &'a Value,
    mexpr: &str,
    cexpr: &str,
    f: &mut F,
) -> Result<Option<Vec<Cow<'a, Value>>>, String>
where
    F: FnMut(Cow<'a, Value>, usize) -> Result<Cow<'a, Value>, String>,
{
    if is_nullish(v) {
        return Ok(None);
    }
    map_call(v, mexpr, cexpr, f).map(Some)
}

fn some_call<'a, F>(v: &'a Value, mexpr: &str, cexpr: &str, f: &mut F) -> Result<bool, String>
where
    F: FnMut(&'a Value, usize) -> Result<bool, String>,
{
    let callee = prop(v, "some", mexpr)?;
    if !(is_undef(callee.as_ref()) && callable_at(v, "some")) {
        return Err(not_a_function(mexpr, cexpr, callee.as_ref()));
    }
    match v {
        Value::Array(a) => {
            for (i, e) in a.iter().enumerate() {
                if f(e, i)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Value::Object(o) if o.len() == 1 => {
            if let Some(Value::Array(a)) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                for (i, e) in a.iter().enumerate() {
                    if f(e, i)? {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn filter_call<'a, F>(
    v: &'a Value,
    mexpr: &str,
    cexpr: &str,
    f: &mut F,
) -> Result<Vec<Cow<'a, Value>>, String>
where
    F: FnMut(&'a Value, usize) -> Result<bool, String>,
{
    let callee = prop(v, "filter", mexpr)?;
    if !(is_undef(callee.as_ref()) && callable_at(v, "filter")) {
        return Err(not_a_function(mexpr, cexpr, callee.as_ref()));
    }
    let mut out = Vec::new();
    match v {
        Value::Array(a) => {
            for (i, e) in a.iter().enumerate() {
                if f(e, i)? {
                    out.push(Cow::Borrowed(e));
                }
            }
        }
        Value::Object(o) if o.len() == 1 => {
            if let Some(Value::Array(a)) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                for (i, e) in a.iter().enumerate() {
                    if f(e, i)? {
                        out.push(Cow::Borrowed(e));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(out)
}

fn find_call<'a, F>(v: &'a [Cow<'a, Value>], f: &mut F) -> Result<Option<Cow<'a, Value>>, String>
where
    F: FnMut(&Value) -> Result<bool, String>,
{
    for e in v {
        if f(e.as_ref())? {
            return Ok(Some(clone_cow(e)));
        }
    }
    Ok(None)
}

fn slice_call<'a>(
    v: &'a Value,
    mexpr: &str,
    cexpr: &str,
    args: &[f64],
) -> Result<Cow<'a, Value>, String> {
    let callee = prop(v, "slice", mexpr)?;
    if !(is_undef(callee.as_ref()) && callable_at(v, "slice")) {
        return Err(not_a_function(mexpr, cexpr, callee.as_ref()));
    }
    let to_i = |x: f64| -> f64 { if x.is_nan() { 0.0 } else { x.trunc() } };
    match v {
        Value::Array(a) => {
            let n = a.len() as f64;
            let start = if args.is_empty() {
                0.0
            } else {
                let s = to_i(args[0]);
                if s < 0.0 { (n + s).max(0.0) } else { s.min(n) }
            };
            let end = if args.len() < 2 {
                n
            } else {
                let e = to_i(args[1]);
                if e < 0.0 { (n + e).max(0.0) } else { e.min(n) }
            };
            let mut out = Vec::new();
            let mut i = start as usize;
            while i < end as usize && i < a.len() {
                out.push(a[i].clone());
                i += 1;
            }
            Ok(Cow::Owned(Value::Array(out)))
        }
        Value::Object(o) if o.len() == 1 => {
            if let Some(Value::Array(a)) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                let n = a.len() as f64;
                let start = if args.is_empty() {
                    0.0
                } else {
                    let s = to_i(args[0]);
                    if s < 0.0 { (n + s).max(0.0) } else { s.min(n) }
                };
                let end = if args.len() < 2 {
                    n
                } else {
                    let e = to_i(args[1]);
                    if e < 0.0 { (n + e).max(0.0) } else { e.min(n) }
                };
                let mut out = Vec::new();
                let mut i = start as usize;
                while i < end as usize && i < a.len() {
                    out.push(a[i].clone());
                    i += 1;
                }
                return Ok(Cow::Owned(json!({ "$bytes": out })));
            }
            Ok(Cow::Owned(Value::Array(Vec::new())))
        }
        _ => Ok(Cow::Owned(Value::Array(Vec::new()))),
    }
}

fn clone_cow<'a>(c: &Cow<'a, Value>) -> Cow<'a, Value> {
    match c {
        Cow::Borrowed(r) => Cow::Borrowed(r),
        Cow::Owned(v) => Cow::Owned(v.clone()),
    }
}

// ---------------------------------------------------------------------------
// JS string calls used by the port
// ---------------------------------------------------------------------------

fn str_method<F>(v: &Value, name: &str, mexpr: &str, cexpr: &str, f: F) -> Result<Vec<u16>, String>
where
    F: Fn(&[u16]) -> Vec<u16>,
{
    let callee = prop(v, name, mexpr)?;
    if !(is_undef(callee.as_ref()) && callable_at(v, name)) {
        return Err(not_a_function(mexpr, cexpr, callee.as_ref()));
    }
    Ok(f(&units(v)))
}

fn str_trim(v: &Value, mexpr: &str, cexpr: &str) -> Result<Vec<u16>, String> {
    str_method(v, "trim", mexpr, cexpr, trim_units)
}

fn locale_call(a: &Value, mexpr: &str, cexpr: &str, b: &Value) -> Result<f64, String> {
    let callee = prop(a, "localeCompare", mexpr)?;
    if !(is_undef(callee.as_ref()) && callable_at(a, "localeCompare")) {
        return Err(not_a_function(mexpr, cexpr, callee.as_ref()));
    }
    let arg = to_string_js(b);
    let bu: Vec<u16> = arg.encode_utf16().collect();
    Ok(locale_cmp(&units(a), &bu))
}

fn locale_cmp(a: &[u16], b: &[u16]) -> f64 {
    if a == b {
        return 0.0;
    }
    let (ra, rb) = (corpus::rank(&lossy(a)), corpus::rank(&lossy(b)));
    if ra != rb {
        return if ra < rb { -1.0 } else { 1.0 };
    }
    match u16_cmp(a, b) {
        Ordering::Less => -1.0,
        Ordering::Greater => 1.0,
        Ordering::Equal => 0.0,
    }
}

fn js_length<'a>(v: &'a Value, expr: &str) -> Result<Cow<'a, Value>, String> {
    prop(v, "length", expr)
}

fn len_num(v: &Value, expr: &str) -> Result<f64, String> {
    Ok(to_number(js_length(v, expr)?.as_ref()))
}

// ---------------------------------------------------------------------------
// JS Map-key stringify for retainTaskActivityIds
// ---------------------------------------------------------------------------

fn js_json(v: &Value) -> String {
    let mut s = String::new();
    js_json_into(v, &mut s);
    s
}

fn js_json_into(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(_) => {
            let x = to_number(v);
            if x.is_nan() || x.is_infinite() {
                out.push_str("null");
            } else {
                out.push_str(&js::number_to_string(x));
            }
        }
        Value::String(_) => js_json_str(&str_of(v), out),
        Value::Array(a) => {
            out.push('[');
            for (i, e) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if is_undef(e) || is_function_like(e) {
                    out.push_str("null");
                } else {
                    js_json_into(e, out);
                }
            }
            out.push(']');
        }
        Value::Object(o) => {
            if o.len() == 1 && kind(v) != K::Obj {
                match kind(v) {
                    K::Und => out.push_str("null"),
                    K::Num => js_json_into(&num_val(to_number(v)), out),
                    K::Str => js_json_str(&str_of(v), out),
                    K::Big => js_json_str(&to_string_js(&inner(v, "$bigint")), out),
                    _ => out.push_str("{}"),
                }
                return;
            }
            out.push('{');
            let mut first = true;
            for (k, x) in o {
                if is_undef(x) || is_function_like(x) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                js_json_str(k, out);
                out.push(':');
                js_json_into(x, out);
            }
            out.push('}');
        }
    }
}

fn is_function_like(_v: &Value) -> bool {
    false
}

fn js_json_str(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---------------------------------------------------------------------------
// Deep equality / shallow equality / records
// ---------------------------------------------------------------------------

fn deep_eq(a: &Value, b: &Value) -> bool {
    if object_is(a, b) {
        return true;
    }
    if is_nullish(a) || is_nullish(b) || typeof_js(a) != typeof_js(b) {
        return false;
    }
    if kind(a) == K::Arr || kind(b) == K::Arr {
        if kind(a) != K::Arr || kind(b) != K::Arr {
            return false;
        }
        let (la, lb) = match (a, b) {
            (Value::Array(x), Value::Array(y)) => (x, y),
            _ => return false,
        };
        if la.len() != lb.len() {
            return false;
        }
        for i in 0..la.len() {
            if !deep_eq(&la[i], &lb[i]) {
                return false;
            }
        }
        return true;
    }
    if typeof_js(a) != "object" || typeof_js(b) != "object" {
        return false;
    }
    let (lk, rk) = (object_keys(a), object_keys(b));
    if lk.len() != rk.len() {
        return false;
    }
    for key in &lk {
        if !has_in(b, key) {
            return false;
        }
        if !deep_eq(prop_raw(a, key).as_ref(), prop_raw(b, key).as_ref()) {
            return false;
        }
    }
    true
}

fn arrays_shallow_eq<'a>(left: &'a Value, right: &[Cow<'a, Value>]) -> Result<bool, String> {
    if !truthy(left) {
        return Ok(false);
    }
    let llen = prop(left, "length", "left.length")?;
    if !strict_eq(llen.as_ref(), &num_val(right.len() as f64)) {
        return Ok(false);
    }
    let n = match kind(left) {
        K::Arr => left.as_array().map_or(0, |a| a.len()),
        K::Str => units(left).len(),
        K::Bytes | K::Buffer => byte_len(left),
        _ => right.len().min(to_number(llen.as_ref()) as usize),
    };
    for i in 0..n {
        let l = idx_opt(left, i as f64);
        let r = right.get(i).map(|c| c.as_ref()).unwrap_or(undef());
        if !strict_eq(l.as_ref(), r) {
            return Ok(false);
        }
    }
    // `left[i]` beyond the materialized right side still compares.
    for i in n..(to_number(llen.as_ref()).max(0.0) as usize) {
        let l = idx_opt(left, i as f64);
        if !strict_eq(l.as_ref(), undef()) {
            return Ok(false);
        }
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// Shared helper ports
// ---------------------------------------------------------------------------

fn basename_of_path(value: &Value) -> Result<Option<Value>, String> {
    let parts = str_method(
        value,
        "split",
        "value.split",
        "value.split(/[/\\\\]/)",
        |u| {
            let mut out: Vec<Vec<u16>> = Vec::new();
            let mut cur = Vec::new();
            for &c in u {
                if c == '/' as u16 || c == '\\' as u16 {
                    out.push(std::mem::take(&mut cur));
                } else {
                    cur.push(c);
                }
            }
            out.push(cur);
            out.into_iter().flatten().collect()
        },
    )?;
    let _ = parts;
    let u = units(value);
    let mut last: Option<Vec<u16>> = None;
    let mut cur = Vec::new();
    for &c in &u {
        if c == '/' as u16 || c == '\\' as u16 {
            if !cur.is_empty() {
                last = Some(std::mem::take(&mut cur));
            } else {
                cur.clear();
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        last = Some(cur);
    }
    Ok(last.map(|x| utf16_val(&x)))
}

fn source_proposed_plans_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if strict_eq(left, right) {
        return Ok(true);
    }
    if is_undef(left) || is_undef(right) {
        return Ok(false);
    }
    Ok(strict_eq(
        prop(left, "threadId", "left.threadId")?.as_ref(),
        prop(right, "threadId", "right.threadId")?.as_ref(),
    ) && strict_eq(
        prop(left, "planId", "left.planId")?.as_ref(),
        prop(right, "planId", "right.planId")?.as_ref(),
    ))
}

fn latest_turns_equal(left: &Value, right: &Value) -> Result<bool, String> {
    if strict_eq(left, right) {
        return Ok(true);
    }
    if is_nullish(left) || is_nullish(right) {
        return Ok(false);
    }
    for (lk, rk) in [
        ("left.turnId", "right.turnId"),
        ("left.state", "right.state"),
        ("left.requestedAt", "right.requestedAt"),
        ("left.startedAt", "right.startedAt"),
        ("left.completedAt", "right.completedAt"),
        ("left.assistantMessageId", "right.assistantMessageId"),
    ] {
        let key = lk.split('.').nth(1).unwrap_or(lk);
        let rkey = rk.split('.').nth(1).unwrap_or(rk);
        if !strict_eq(
            prop(left, key, lk)?.as_ref(),
            prop(right, rkey, rk)?.as_ref(),
        ) {
            return Ok(false);
        }
    }
    source_proposed_plans_equal(
        prop(left, "sourceProposedPlan", "left.sourceProposedPlan")?.as_ref(),
        prop(right, "sourceProposedPlan", "right.sourceProposedPlan")?.as_ref(),
    )
}

// ---------------------------------------------------------------------------
// Model slug normalization (packages/shared/src/model.ts)
// ---------------------------------------------------------------------------

fn alias_table(provider: &str) -> Option<&'static [(&'static str, &'static str)]> {
    Some(match provider {
        "codex" => &[
            ("sol", "gpt-6-sol"),
            ("luna", "gpt-6-luna"),
            ("astra", "gpt-6-astra"),
            ("6", "gpt-6-astra"),
            ("gpt-6", "gpt-6-astra"),
            ("5.5", "gpt-5.5"),
            ("5.4", "gpt-5.4"),
            ("5.3", "gpt-5.3-codex"),
            ("gpt-5.3", "gpt-5.3-codex"),
            ("5.3-spark", "gpt-5.3-codex-spark"),
            ("gpt-5.3-spark", "gpt-5.3-codex-spark"),
        ],
        "claudeAgent" => &[
            ("fable", "claude-fable-5-1"),
            ("fable-5.1", "claude-fable-5-1"),
            ("claude-fable-5.1", "claude-fable-5-1"),
            ("claude-fable-5-1", "claude-fable-5-1"),
            ("fable-5", "claude-fable-5"),
            ("claude-fable-5", "claude-fable-5"),
            ("opus", "claude-opus-5-5"),
            ("opus-5.5", "claude-opus-5-5"),
            ("claude-opus-5.5", "claude-opus-5-5"),
            ("claude-opus-5-5", "claude-opus-5-5"),
            ("opus-5", "claude-opus-5"),
            ("claude-opus-5", "claude-opus-5"),
            ("opus-4.8", "claude-opus-4-8"),
            ("claude-opus-4.8", "claude-opus-4-8"),
            ("claude-opus-4-8-20260528", "claude-opus-4-8"),
            ("opus-4.7", "claude-opus-4-7"),
            ("claude-opus-4.7", "claude-opus-4-7"),
            ("claude-opus-4-7-20260416", "claude-opus-4-7"),
            ("opus-4.6", "claude-opus-4-6"),
            ("claude-opus-4.6", "claude-opus-4-6"),
            ("claude-opus-4-6-20251117", "claude-opus-4-6"),
            ("opus-4.5", "claude-opus-4-5"),
            ("claude-opus-4.5", "claude-opus-4-5"),
            ("claude-opus-4-5-20250120", "claude-opus-4-5"),
            ("sonnet", "claude-sonnet-5"),
            ("sonnet-5", "claude-sonnet-5"),
            ("claude-sonnet-5", "claude-sonnet-5"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("claude-sonnet-4.6", "claude-sonnet-4-6"),
            ("claude-sonnet-4-6-20251117", "claude-sonnet-4-6"),
            ("haiku", "claude-haiku-4-5"),
            ("haiku-4.5", "claude-haiku-4-5"),
            ("claude-haiku-4.5", "claude-haiku-4-5"),
            ("claude-haiku-4-5-20251001", "claude-haiku-4-5"),
        ],
        "cursor" => &[
            ("auto", "auto"),
            ("default", "auto"),
            ("composer", "composer-2.5"),
            ("composer-2.5", "composer-2.5"),
            ("composer-2", "composer-2.5"),
            ("opus", "claude-opus-5"),
            ("opus-5", "claude-opus-5"),
            ("opus-4.8", "claude-opus-4-8"),
            ("opus-4.7", "claude-opus-4-7"),
            ("opus-4.6", "claude-opus-4-6"),
            ("opus-4.6-thinking", "claude-opus-4-6"),
            ("sonnet", "claude-sonnet-5"),
            ("sonnet-5", "claude-sonnet-5"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("fable", "claude-fable-5"),
            ("fable-5", "claude-fable-5"),
            ("sol", "gpt-5.6-sol"),
            ("5.6", "gpt-5.6-sol"),
            ("gpt-5.3", "gpt-5.3-codex"),
            ("codex-5.3", "gpt-5.3-codex"),
            ("grok", "grok-4.6"),
            ("grok-4.5", "grok-4.5"),
            ("grok-4.6", "grok-4.6"),
            ("cursor-grok-4.5", "grok-4.5"),
            ("cursor-grok-4.6", "grok-4.6"),
            ("gemini", "gemini-3.1-pro"),
            ("gemini-3", "gemini-3.1-pro"),
            ("gemini-3-pro", "gemini-3.1-pro"),
            ("gemini-3.1-pro-preview", "gemini-3.1-pro"),
            ("glm", "glm-5.2"),
            ("kimi", "kimi-k2.7-code"),
        ],
        "antigravity" => &[],
        "droid" => &[
            ("droid", "claude-opus-4-8"),
            ("factory", "claude-opus-4-8"),
            ("opus", "claude-opus-4-8"),
            ("opus-4.8", "claude-opus-4-8"),
            ("opus-fast", "claude-opus-4-8-fast"),
            ("opus-4.8-fast", "claude-opus-4-8-fast"),
            ("opus-4.7", "claude-opus-4-7"),
            ("opus-4.7-fast", "claude-opus-4-7-fast"),
            ("opus-4.6", "claude-opus-4-6"),
            ("sonnet", "claude-sonnet-5"),
            ("sonnet-5", "claude-sonnet-5"),
            ("sonnet-4.6", "claude-sonnet-4-6"),
            ("sonnet-4.5", "claude-sonnet-4-5-20250929"),
            ("fable", "claude-fable-5"),
            ("haiku", "claude-haiku-4-5-20251001"),
            ("5.5", "gpt-5.5"),
            ("5.5-fast", "gpt-5.5-fast"),
            ("5.5-pro", "gpt-5.5-pro"),
            ("5.4", "gpt-5.4"),
            ("5.4-fast", "gpt-5.4-fast"),
            ("5.4-mini", "gpt-5.4-mini"),
            ("5.3", "gpt-5.3-codex"),
            ("5.3-fast", "gpt-5.3-codex-fast"),
            ("gpt-5.3", "gpt-5.3-codex"),
            ("gemini-3-pro", "gemini-3.1-pro-preview"),
            ("gemini-3.1-pro", "gemini-3.1-pro-preview"),
            ("gemini-3.5-flash", "gemini-3.5-flash"),
            ("gemini-3-flash", "gemini-3-flash-preview"),
            ("glm", "glm-5.2"),
            ("glm-5.2", "glm-5.2"),
            ("glm-5.1", "glm-5.1"),
            ("nemotron", "nemotron-3-ultra"),
            ("kimi", "kimi-k2.7-code"),
            ("kimi-code", "kimi-k2.7-code"),
            ("deepseek", "deepseek-v4-pro"),
            ("minimax", "minimax-m3"),
        ],
        "grok" => &[
            ("grok", "grok-build-0.1"),
            ("build", "grok-build-0.1"),
            ("grok-build-0.1", "grok-build-0.1"),
            ("grok-build", "grok-build"),
            ("4.3", "grok-build"),
            ("grok-4", "grok-build"),
            ("grok-4.3", "grok-build"),
            ("grok-latest", "grok-build"),
            ("grok-code-fast", "grok-build-0.1"),
            ("grok-code-fast-1", "grok-build-0.1"),
            ("grok-code-fast-1-0825", "grok-build-0.1"),
            ("code-fast", "grok-build-0.1"),
            ("4.5", "grok-4.5"),
            ("grok-4.5", "grok-4.5"),
            ("4.6", "grok-4.6"),
            ("grok-4.6", "grok-4.6"),
        ],
        "opencode" => &[],
        "pi" => &[],
        "devin" => &[
            ("adaptive", "adaptive"),
            ("auto", "adaptive"),
            ("fast", "swe-1-6"),
            ("swe-1.6-fast", "swe-1-6"),
            ("swe-1.6", "swe-1-6"),
            ("swe-1.7", "swe-1-7"),
            ("swe", "swe-1-6"),
            ("swe-1-6", "swe-1-6"),
            ("swe-1-7", "swe-1-7"),
            ("opus", "claude-opus-4-8"),
            ("sonnet", "claude-sonnet-5"),
            ("fable", "claude-fable-5"),
        ],
        "omp" => &[],
        _ => return None,
    })
}

fn claude_context_window_suffix(model: &[u16]) -> Option<Vec<u16>> {
    if !u16_ends(model, &u16s("]")) {
        return None;
    }
    let mut i = model.len();
    while i > 0 {
        i -= 1;
        if model[i] == '[' as u16 {
            return Some(model[i + 1..model.len() - 1].to_vec());
        }
        if model[i] == ']' as u16 && i != model.len() - 1 {
            return None;
        }
    }
    None
}

fn strip_claude_context_window_suffix(model: &[u16]) -> Vec<u16> {
    if !u16_ends(model, &u16s("]")) {
        return model.to_vec();
    }
    let mut i = model.len();
    while i > 0 {
        i -= 1;
        if model[i] == '[' as u16 {
            return model[..i].to_vec();
        }
        if model[i] == ']' as u16 && i != model.len() - 1 {
            return model.to_vec();
        }
    }
    model.to_vec()
}

fn normalize_model_slug(model: &Value, provider: &Value) -> Result<Option<String>, String> {
    if kind(model) != K::Str {
        return Ok(None);
    }
    let trimmed = trim_units(&units(model));
    if trimmed.is_empty() {
        return Ok(None);
    }
    let provider_scoped: Vec<u16> = if strict_eq(provider, &str_val("claudeAgent")) {
        strip_claude_context_window_suffix(&trimmed)
    } else if strict_eq(provider, &str_val("devin"))
        && trimmed == lower_units(&trimmed)
        && u16_ends(&trimmed, &u16s("-medium"))
    {
        u16_slice(&trimmed, 0, trimmed.len() as i64 - 7)
    } else {
        trimmed.clone()
    };
    let alias_key = lossy(&lower_units(&provider_scoped));
    let provider_key = to_property_key(provider);
    let Some(aliases) = alias_table(&provider_key) else {
        return Err(
            "undefined is not an object (evaluating 'Object.prototype.hasOwnProperty.call(aliases, aliasKey)')".to_string(),
        );
    };
    let aliased = aliases
        .iter()
        .find(|(k, _)| *k == alias_key)
        .map(|(_, v)| *v);
    let normalized = aliased
        .map(str::to_string)
        .unwrap_or_else(|| lossy(&provider_scoped));
    if strict_eq(provider, &str_val("claudeAgent"))
        && claude_context_window_suffix(&trimmed)
            .map(|s| lower_units(&s) == u16s("1m"))
            .unwrap_or(false)
    {
        return Ok(Some(format!("{normalized}[1m]")));
    }
    Ok(Some(normalized))
}

// ---------------------------------------------------------------------------
// Workspace root normalization (packages/shared/src/threadWorkspace.ts)
// ---------------------------------------------------------------------------

fn is_likely_windows_workspace_root(value: &[u16]) -> bool {
    if value.len() >= 2
        && (value[0] as u32) < 128
        && (value[0] as u8 as char).is_ascii_alphabetic()
        && value[1] == ':' as u16
        && (value.len() == 2 || value[2] == '/' as u16 || value[2] == '\\' as u16)
    {
        return true;
    }
    u16_starts(value, &[b'\\' as u16, b'\\' as u16]) || u16_starts(value, &u16s("//"))
}

fn normalize_workspace_root(value: &Value) -> Result<String, String> {
    let trimmed = str_trim(value, "value.trim", "value.trim()")?;
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    let fwd: Vec<u16> = trimmed
        .iter()
        .map(|&c| if c == '\\' as u16 { '/' as u16 } else { c })
        .collect();
    let is_windows_drive_root = fwd.len() >= 3
        && (fwd[0] as u32) < 128
        && (fwd[0] as u8 as char).is_ascii_alphabetic()
        && fwd[1] == ':' as u16
        && fwd[2..].iter().all(|&c| c == '/' as u16);
    let has_unc = u16_starts(&fwd, &u16s("//"));
    let prefix: Vec<u16> = if has_unc {
        u16s("//")
    } else if u16_starts(&fwd, &u16s("/")) {
        u16s("/")
    } else {
        Vec::new()
    };
    // Collapse runs of '/' in the body.
    let mut body: Vec<u16> = Vec::new();
    let mut prev_slash = false;
    for &c in &fwd[prefix.len()..] {
        if c == '/' as u16 {
            if !prev_slash {
                body.push(c);
            }
            prev_slash = true;
        } else {
            body.push(c);
            prev_slash = false;
        }
    }
    while body.last() == Some(&('/' as u16)) {
        body.pop();
    }
    let mut normalized = prefix.clone();
    normalized.extend(&body);
    let final_value: Vec<u16> = if is_windows_drive_root {
        let mut out = fwd[..2].to_vec();
        out.push('/' as u16);
        out
    } else if !normalized.is_empty() {
        normalized
    } else {
        prefix
    };
    if is_likely_windows_workspace_root(&trimmed) {
        return Ok(lossy(&lower_units(&final_value)));
    }
    Ok(lossy(&final_value))
}

fn project_cwd_key(cwd: &Value) -> Result<String, String> {
    normalize_workspace_root(cwd)
}

// ---------------------------------------------------------------------------
// git.ts helpers needed by the branch regression guard
// ---------------------------------------------------------------------------

fn is_temporary_worktree_branch(branch: &Value) -> Result<bool, String> {
    let normalized = str_method(branch, "trim", "branch.trim", "branch.trim()", |u| {
        lower_units(&trim_units(u))
    })?;
    // ^([a-z0-9][a-z0-9-]*)\/[0-9a-f]{8}$
    let s = &normalized;
    let Some(slash) = s.iter().position(|&c| c == '/' as u16) else {
        return Ok(false);
    };
    let (ns, suffix) = (&s[..slash], &s[slash + 1..]);
    let hex = |c: u16| -> bool { matches!(c as u8 as char, '0'..='9' | 'a'..='f') };
    let valid_ns = !ns.is_empty()
        && ns.iter().all(|&c| {
            let ch = c as u8 as char;
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'
        })
        && ns[0] != '-' as u16;
    if !(valid_ns && suffix.len() == 8 && suffix.iter().all(|&c| hex(c))) {
        return Ok(false);
    }
    let namespace = lossy(ns);
    Ok(namespace == "synara" || is_pre_cutover_worktree_namespace(&namespace))
}

fn hash_worktree_namespace(value: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for c in value.encode_utf16() {
        hash ^= c as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn is_pre_cutover_worktree_namespace(value: &str) -> bool {
    matches!(
        hash_worktree_namespace(value),
        0x8559131d2c062cf0 | 0xd53b4ab95395e345
    )
}

fn resolve_thread_branch_regression_guard(current: &Value, next: &Value) -> Result<Value, String> {
    if !current.is_null()
        && !next.is_null()
        && !is_temporary_worktree_branch(current)?
        && is_temporary_worktree_branch(next)?
    {
        return Ok(current.clone());
    }
    Ok(next.clone())
}

// ---------------------------------------------------------------------------
// asyncUserInput.ts
// ---------------------------------------------------------------------------

fn merge_async_user_input(previous: &Value, incoming: &Value) -> Result<Value, String> {
    if !truthy(previous) {
        return Ok(incoming.clone());
    }
    if !truthy(incoming) {
        return Ok(previous.clone());
    }
    let prev_seq = to_number_or(
        prop(previous, "responseSequence", "previous.responseSequence")?.as_ref(),
        0.0,
    );
    let inc_seq = to_number_or(
        prop(incoming, "responseSequence", "incoming.responseSequence")?.as_ref(),
        0.0,
    );
    if prev_seq > inc_seq {
        return Ok(previous.clone());
    }
    if inc_seq > prev_seq {
        return Ok(incoming.clone());
    }
    if truthy(prop(previous, "response", "previous.response")?.as_ref()) {
        return Ok(previous.clone());
    }
    Ok(incoming.clone())
}

// ---------------------------------------------------------------------------
// pendingInteractions.ts / threadSummary.ts helpers
// ---------------------------------------------------------------------------

fn pending_request_instance_key(request_id: &Value, generation: Option<&Value>) -> String {
    format!(
        "{}\u{0}{}",
        to_string_js(request_id),
        generation
            .map(to_string_js)
            .unwrap_or_else(|| "legacy".to_string())
    )
}

fn is_stale_pending_request_failure_detail(detail: Option<&Value>) -> Result<bool, String> {
    let Some(d) = detail else {
        return Ok(false);
    };
    if !truthy(d) {
        return Ok(false);
    }
    let normalized = str_method(
        d,
        "toLowerCase",
        "detail.toLowerCase",
        "detail.toLowerCase()",
        lower_units,
    )?;
    for needle in [
        "stale pending approval request",
        "stale pending user-input request",
        "unknown pending approval request",
        "unknown pending permission request",
        "unknown pending user-input request",
        "stale pending user input request",
        "unknown pending user input request",
    ] {
        let needle_u: Vec<u16> = needle.encode_utf16().collect();
        if u16_contains(&normalized, &needle_u) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn approval_request_kind_from_request_type(request_type: &Value) -> Option<&'static str> {
    for (pat, out) in [
        ("command_execution_approval", "command"),
        ("exec_command_approval", "command"),
        ("file_read_approval", "file-read"),
        ("file_change_approval", "file-change"),
        ("apply_patch_approval", "file-change"),
        ("permissions_approval", "permissions"),
        ("tool_approval", "tool"),
        ("dynamic_tool_call", "tool"),
    ] {
        if strict_eq(request_type, &str_val(pat)) {
            return Some(out);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Activity ordering (workLog.ts / threadSummary.ts versions)
// ---------------------------------------------------------------------------

fn weakmap_can_hold(v: &Value) -> bool {
    is_object(v)
}

fn lifecycle_rank(kind_v: &Value) -> Result<f64, String> {
    let ends = |suffix: &str| -> Result<bool, String> {
        let callee = prop(kind_v, "endsWith", "kind.endsWith")?;
        if !(is_undef(callee.as_ref()) && callable_at(kind_v, "endsWith")) {
            return Err(not_a_function(
                "kind.endsWith",
                &format!("kind.endsWith({suffix:?})"),
                callee.as_ref(),
            ));
        }
        Ok(u16_ends(
            &units(kind_v),
            &suffix.encode_utf16().collect::<Vec<u16>>(),
        ))
    };
    if ends(".started")? || strict_eq(kind_v, &str_val("tool.started")) {
        return Ok(0.0);
    }
    if ends(".progress")? || ends(".updated")? {
        return Ok(1.0);
    }
    if ends(".completed")? || ends(".resolved")? {
        return Ok(2.0);
    }
    Ok(1.0)
}

fn compare_activities_summary(l: &Value, r: &Value) -> Result<f64, String> {
    let lseq = prop(l, "sequence", "left.sequence")?;
    let rseq = prop(r, "sequence", "right.sequence")?;
    let d = to_number_or(lseq.as_ref(), 9007199254740991.0)
        - to_number_or(rseq.as_ref(), 9007199254740991.0);
    if d != 0.0 && !d.is_nan() {
        return Ok(d);
    }
    let lca = prop(l, "createdAt", "left.createdAt")?;
    let rca = prop(r, "createdAt", "right.createdAt")?;
    let c = locale_call(
        lca.as_ref(),
        "left.createdAt.localeCompare",
        "left.createdAt.localeCompare(right.createdAt)",
        rca.as_ref(),
    )?;
    if c != 0.0 {
        return Ok(c);
    }
    let lid = prop(l, "id", "left.id")?;
    let rid = prop(r, "id", "right.id")?;
    locale_call(
        lid.as_ref(),
        "left.id.localeCompare",
        "left.id.localeCompare(right.id)",
        rid.as_ref(),
    )
}

fn compare_activities_worklog(l: &Value, r: &Value) -> Result<f64, String> {
    let lseq = prop(l, "sequence", "left.sequence")?;
    let rseq = prop(r, "sequence", "right.sequence")?;
    if !is_undef(lseq.as_ref()) && !is_undef(rseq.as_ref()) {
        if !strict_eq(lseq.as_ref(), rseq.as_ref()) {
            return Ok(to_number(lseq.as_ref()) - to_number(rseq.as_ref()));
        }
    } else if !is_undef(lseq.as_ref()) {
        return Ok(1.0);
    } else if !is_undef(rseq.as_ref()) {
        return Ok(-1.0);
    }
    let lca = prop(l, "createdAt", "left.createdAt")?;
    let rca = prop(r, "createdAt", "right.createdAt")?;
    let c = locale_call(
        lca.as_ref(),
        "left.createdAt.localeCompare",
        "left.createdAt.localeCompare(right.createdAt)",
        rca.as_ref(),
    )?;
    if c != 0.0 {
        return Ok(c);
    }
    let lk = prop(l, "kind", "left.kind")?;
    let rk = prop(r, "kind", "right.kind")?;
    let ld = lifecycle_rank(lk.as_ref())? - lifecycle_rank(rk.as_ref())?;
    if ld != 0.0 {
        return Ok(ld);
    }
    if strict_eq(lk.as_ref(), &str_val("context-compaction"))
        && strict_eq(rk.as_ref(), &str_val("context-compaction"))
    {
        let ls = prop(l, "summary", "left.summary")?;
        let rs = prop(r, "summary", "right.summary")?;
        let rank = |s: &Value| -> f64 {
            if strict_eq(s, &str_val("Compacting context"))
                || strict_eq(s, &str_val("Compacting conversation..."))
            {
                0.0
            } else {
                1.0
            }
        };
        let d = rank(ls.as_ref()) - rank(rs.as_ref());
        if d != 0.0 {
            return Ok(d);
        }
    }
    let lid = prop(l, "id", "left.id")?;
    let rid = prop(r, "id", "right.id")?;
    locale_call(
        lid.as_ref(),
        "left.id.localeCompare",
        "left.id.localeCompare(right.id)",
        rid.as_ref(),
    )
}

#[derive(Clone, Copy)]
enum OrderFlavor {
    Summary,
    Worklog,
}

fn ordered_activities<'a>(
    activities: &'a Value,
    flavor: OrderFlavor,
) -> Result<Vec<Cow<'a, Value>>, String> {
    // WeakMap.get: primitives read as undefined (no throw).
    let n = len_num(activities, "activities.length")?;
    let cmp = |l: &Value, r: &Value| -> Result<f64, String> {
        match flavor {
            OrderFlavor::Summary => compare_activities_summary(l, r),
            OrderFlavor::Worklog => compare_activities_worklog(l, r),
        }
    };
    let mut stable = true;
    let count = if n.is_nan() || n < 0.0 { 0 } else { n as usize };
    for i in 1..count {
        let l = idx_opt(activities, (i - 1) as f64);
        let r = idx_opt(activities, i as f64);
        if cmp(l.as_ref(), r.as_ref())? > 0.0 {
            stable = false;
            break;
        }
    }
    let mut elements: Vec<Cow<'a, Value>> =
        (0..count).map(|i| idx_opt(activities, i as f64)).collect();
    if !stable {
        js_sort_res(
            &mut elements,
            &mut |a: &Cow<'a, Value>, b: &Cow<'a, Value>| cmp(a.as_ref(), b.as_ref()),
        )?;
    }
    if !weakmap_can_hold(activities) {
        return Err("WeakMap keys must be objects or non-registered symbols".to_string());
    }
    Ok(elements)
}

// ---------------------------------------------------------------------------
// pendingInteractions: stale matcher + identity helpers
// ---------------------------------------------------------------------------

fn stale_pending_matcher<'a>(activities: &'a Value) -> Result<JsMap<'a>, String> {
    let mut stale_at = JsMap::new();
    for activity in iter(activities, "activity", "activities")? {
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        let kind_str = if strict_eq(
            kind_v.as_ref(),
            &str_val("provider.approval.respond.failed"),
        ) {
            Some("approval")
        } else if strict_eq(
            kind_v.as_ref(),
            &str_val("provider.user-input.respond.failed"),
        ) {
            Some("userInput")
        } else {
            None
        };
        let payload_v = prop(activity.as_ref(), "payload", "activity.payload")?;
        if kind_str.is_none() || payload_v.is_null() || !is_object(payload_v.as_ref()) {
            continue;
        }
        let request_id = prop(payload_v.as_ref(), "requestId", "payload.requestId")?;
        let detail = prop(payload_v.as_ref(), "detail", "payload.detail")?;
        let detail_arg = if kind(detail.as_ref()) == K::Str {
            Some(detail.as_ref())
        } else {
            None
        };
        if kind(request_id.as_ref()) != K::Str
            || !is_stale_pending_request_failure_detail(detail_arg)?
        {
            continue;
        }
        let lgen = prop(
            payload_v.as_ref(),
            "lifecycleGeneration",
            "payload.lifecycleGeneration",
        )?;
        let generation = if kind(lgen.as_ref()) == K::Str && !units(lgen.as_ref()).is_empty() {
            Some(lgen.as_ref().clone())
        } else {
            None
        };
        let key = format!(
            "{}\u{0}{}",
            kind_str.unwrap_or(""),
            pending_request_instance_key(request_id.as_ref(), generation.as_ref())
        );
        let key_v = str_val(&key);
        let created = prop(activity.as_ref(), "createdAt", "activity.createdAt")?;
        let previous = stale_at.get(&key_v);
        if previous.is_none() || js_gt(created.as_ref(), previous.unwrap_or(&Value::Null)) {
            stale_at.set(key_v, Cow::Owned(created.as_ref().clone()));
        }
    }
    Ok(stale_at)
}

fn stale_matcher_has(stale_at: &JsMap<'_>, interaction: &Value) -> Result<bool, String> {
    let kind_v = prop(
        interaction,
        "interactionKind",
        "interaction.interactionKind",
    )?;
    let rid = prop(interaction, "requestId", "interaction.requestId")?;
    let lgen = prop(
        interaction,
        "lifecycleGeneration",
        "interaction.lifecycleGeneration",
    )?;
    if !is_nullish(lgen.as_ref()) {
        let key = format!(
            "{}\u{0}{}",
            to_string_js(kind_v.as_ref()),
            pending_request_instance_key(rid.as_ref(), Some(lgen.as_ref()))
        );
        if stale_at.has(&str_val(&key)) {
            return Ok(true);
        }
    }
    let legacy_key = format!(
        "{}\u{0}{}",
        to_string_js(kind_v.as_ref()),
        pending_request_instance_key(rid.as_ref(), None)
    );
    let legacy = stale_at.get(&str_val(&legacy_key));
    let created = prop(interaction, "createdAt", "interaction.createdAt")?;
    Ok(legacy.is_some() && js_ge(legacy.unwrap_or(&Value::Null), created.as_ref()))
}

const PENDING_REQUEST_KINDS: &[&str] = &["approval.requested", "user-input.requested"];
const PENDING_ACTIVITY_KINDS: &[&str] = &[
    "approval.requested",
    "user-input.requested",
    "approval.resolved",
    "user-input.resolved",
    "provider.approval.respond.failed",
    "provider.user-input.respond.failed",
];

fn pending_interaction_identity(activity: &Value) -> Result<Option<Value>, String> {
    let kind_v = prop(activity, "kind", "activity.kind")?;
    if !PENDING_ACTIVITY_KINDS
        .iter()
        .any(|k| strict_eq(kind_v.as_ref(), &str_val(k)))
    {
        return Ok(None);
    }
    let payload_v = prop(activity, "payload", "activity.payload")?;
    let payload = as_record(payload_v.as_ref());
    let request_id = match &payload {
        Some(p) => prop_opt(p, "requestId").into_owned(),
        None => Value::Null,
    };
    let rid_trimmed = if kind(&request_id) == K::Str {
        Some(str_trim(&request_id, "requestId.trim", "requestId.trim()")?)
    } else {
        None
    };
    if kind(&request_id) != K::Str || rid_trimmed.as_ref().is_none_or(|t| t.is_empty()) {
        return Ok(None);
    }
    let starts = |pat: &str| -> Result<bool, String> {
        let callee = prop(kind_v.as_ref(), "startsWith", "activity.kind.startsWith")?;
        if !(is_undef(callee.as_ref()) && callable_at(kind_v.as_ref(), "startsWith")) {
            return Err(not_a_function(
                "activity.kind.startsWith",
                &format!("activity.kind.startsWith({pat:?})"),
                callee.as_ref(),
            ));
        }
        Ok(u16_starts(
            &units(kind_v.as_ref()),
            &pat.encode_utf16().collect::<Vec<u16>>(),
        ))
    };
    let interaction_kind = if starts("approval.")? || starts("provider.approval.")? {
        "approval"
    } else {
        "userInput"
    };
    let generation = match &payload {
        Some(p) => prop_opt(p, "lifecycleGeneration").into_owned(),
        None => Cow::Borrowed(undef()).into_owned(),
    };
    let lifecycle_generation = if kind(&generation) == K::Str && !units(&generation).is_empty() {
        generation.clone()
    } else {
        Value::Null
    };
    let created_at = prop(activity, "createdAt", "activity.createdAt")?;
    let gen_arg = if lifecycle_generation.is_null() {
        None
    } else {
        Some(&lifecycle_generation)
    };
    let key = format!(
        "{}:{}",
        interaction_kind,
        pending_request_instance_key(&request_id, gen_arg)
    );
    let mut o = Map::new();
    o.insert("interactionKind".into(), str_val(interaction_kind));
    o.insert("requestId".into(), request_id.clone());
    o.insert("lifecycleGeneration".into(), lifecycle_generation);
    o.insert("createdAt".into(), created_at.as_ref().clone());
    o.insert("key".into(), str_val(&key));
    Ok(Some(Value::Object(o)))
}

fn pending_interactions_from_activities(activities: &Value) -> Result<Vec<Value>, String> {
    let mut open = JsMap::new();
    for activity in iter(activities, "activity", "activities")? {
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        let is_request = PENDING_REQUEST_KINDS
            .iter()
            .any(|k| strict_eq(kind_v.as_ref(), &str_val(k)));
        let is_resolved = strict_eq(kind_v.as_ref(), &str_val("approval.resolved"))
            || strict_eq(kind_v.as_ref(), &str_val("user-input.resolved"));
        if !is_request && !is_resolved {
            continue;
        }
        let Some(identity) = pending_interaction_identity(activity.as_ref())? else {
            continue;
        };
        let ik = prop(
            identity_obj(&identity),
            "interactionKind",
            "identity.interactionKind",
        )?;
        let rid = prop(identity_obj(&identity), "requestId", "identity.requestId")?;
        let key = format!(
            "{}:{}",
            to_string_js(ik.as_ref()),
            to_string_js(rid.as_ref())
        );
        if is_request {
            open.set(str_val(&key), Cow::Owned(identity));
            continue;
        }
        let lg = prop(
            identity_obj(&identity),
            "lifecycleGeneration",
            "identity.lifecycleGeneration",
        )?;
        if lg.is_null() {
            open.delete(&str_val(&key));
        } else {
            let existing = open.get(&str_val(&key));
            let matches_gen = match existing {
                Some(e) => strict_eq(prop_opt(e, "lifecycleGeneration").as_ref(), lg.as_ref()),
                None => false,
            };
            if matches_gen {
                open.delete(&str_val(&key));
            }
        }
    }
    if open.size() == 0 {
        return Ok(Vec::new());
    }
    let stale_at = stale_pending_matcher(activities)?;
    let mut out = Vec::new();
    for request in open.values() {
        if !stale_matcher_has(&stale_at, request.as_ref())? {
            out.push(request.as_ref().clone());
        }
    }
    Ok(out)
}

fn identity_obj(v: &Value) -> &Value {
    v
}

// ---------------------------------------------------------------------------
// Activities (dedupe, cap, normalize)
// ---------------------------------------------------------------------------

fn as_record(value: &Value) -> Option<&Value> {
    if truthy(value) && typeof_js(value) == "object" {
        Some(value)
    } else {
        None
    }
}

fn normalize_activity_command_value(value: &Value) -> Option<Vec<u16>> {
    if kind(value) == K::Str {
        let t = trim_units(&units(value));
        return if t.is_empty() { None } else { Some(t) };
    }
    let Value::Array(a) = value else {
        return None;
    };
    let parts: Vec<Vec<u16>> = a
        .iter()
        .map(|e| {
            if kind(e) == K::Str {
                trim_units(&units(e))
            } else {
                Vec::new()
            }
        })
        .filter(|e| !e.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        if i > 0 {
            out.push(' ' as u16);
        }
        out.extend(p);
    }
    Some(out)
}

fn activity_payload_detail_score(activity: &Value) -> Result<f64, String> {
    let payload_v = prop(activity, "payload", "activity.payload")?;
    let payload = as_record(payload_v.as_ref());
    let data_v = payload.map(|p| prop_opt(p, "data"));
    let data = data_v.as_ref().and_then(|c| as_record(c.as_ref()));
    let item_v = data.map(|d| prop_opt(d, "item"));
    let item = item_v.as_ref().and_then(|c| as_record(c.as_ref()));
    let command_actions = item
        .and_then(|i| {
            let v = prop_opt(i, "commandActions");
            if is_undef(v.as_ref()) {
                None
            } else {
                Some(v.into_owned())
            }
        })
        .or_else(|| {
            data.and_then(|d| {
                let v = prop_opt(d, "commandActions");
                if is_undef(v.as_ref()) {
                    None
                } else {
                    Some(v.into_owned())
                }
            })
        })
        .or_else(|| {
            payload.and_then(|p| {
                let v = prop_opt(p, "commandActions");
                if is_undef(v.as_ref()) {
                    None
                } else {
                    Some(v.into_owned())
                }
            })
        });
    let mut score = 0.0;
    if payload.is_some_and(|p| truthy(prop_opt(p, "itemType").as_ref())) {
        score += 4.0;
    }
    if payload.is_some_and(|p| truthy(prop_opt(p, "title").as_ref())) {
        score += 1.0;
    }
    if payload.is_some_and(|p| truthy(prop_opt(p, "detail").as_ref())) {
        score += 2.0;
    }
    if data.is_some() {
        score += 2.0;
    }
    if item.is_some() {
        score += 4.0;
    }
    let command = item
        .and_then(|i| {
            let v = prop_opt(i, "command");
            if is_undef(v.as_ref()) {
                None
            } else {
                Some(v.into_owned())
            }
        })
        .or_else(|| {
            data.and_then(|d| {
                let v = prop_opt(d, "command");
                if is_undef(v.as_ref()) {
                    None
                } else {
                    Some(v.into_owned())
                }
            })
        })
        .or_else(|| {
            payload.and_then(|p| {
                let v = prop_opt(p, "command");
                if is_undef(v.as_ref()) {
                    None
                } else {
                    Some(v.into_owned())
                }
            })
        });
    if command
        .as_ref()
        .and_then(normalize_activity_command_value)
        .is_some()
    {
        score += 8.0;
    }
    if let Some(ca) = &command_actions
        && let Value::Array(a) = ca
        && !a.is_empty()
    {
        score += 8.0;
    }
    Ok(score)
}

fn activities_equal(left: &Value, right: &Value) -> Result<bool, String> {
    for field in ["kind", "tone", "summary"] {
        if !strict_eq(
            prop(left, field, &format!("left.{field}"))?.as_ref(),
            prop(right, field, &format!("right.{field}"))?.as_ref(),
        ) {
            return Ok(false);
        }
    }
    if !deep_eq(
        prop(left, "payload", "left.payload")?.as_ref(),
        prop(right, "payload", "right.payload")?.as_ref(),
    ) {
        return Ok(false);
    }
    for field in ["turnId", "sequence", "sequenceSource", "createdAt"] {
        if !strict_eq(
            prop(left, field, &format!("left.{field}"))?.as_ref(),
            prop(right, field, &format!("right.{field}"))?.as_ref(),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn prefer_richer(previous: Cow<'_, Value>, incoming: Cow<'_, Value>) -> Result<Value, String> {
    if activities_equal(previous.as_ref(), incoming.as_ref())? {
        return Ok(previous.into_owned());
    }
    let prev_score = activity_payload_detail_score(previous.as_ref())?;
    let inc_score = activity_payload_detail_score(incoming.as_ref())?;
    if inc_score < prev_score {
        Ok(previous.into_owned())
    } else {
        Ok(incoming.into_owned())
    }
}

fn dedupe_activities_vec<'a>(activities: &'a Value) -> Result<Value, String> {
    let mut index_by_id = JsMap::new();
    let mut result: Vec<Cow<'a, Value>> = Vec::new();
    for activity in iter(activities, "activity", "activities")? {
        let id = prop(activity.as_ref(), "id", "activity.id")?;
        match index_by_id.get(id.as_ref()) {
            None => {
                index_by_id.set(
                    id.as_ref().clone(),
                    Cow::Owned(num_val(result.len() as f64)),
                );
                result.push(activity);
            }
            Some(existing_idx) => {
                let i = to_number(existing_idx) as usize;
                result[i] = Cow::Owned(prefer_richer(clone_cow(&result[i]), clone_cow(&activity))?);
            }
        }
    }
    if arrays_shallow_eq(activities, &result)? {
        Ok(activities.clone())
    } else {
        Ok(Value::Array(
            result.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn retain_task_activity_ids(activities: &Value, retained_ids: &mut JsSet) -> Result<(), String> {
    let mut latest = JsMap::new();
    for activity in ordered_activities(activities, OrderFlavor::Worklog)? {
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        if strict_eq(kind_v.as_ref(), &str_val("turn.tasks.updated")) {
            latest.set(str_val("task-list"), clone_cow(&activity));
            continue;
        }
        let payload_v = prop(activity.as_ref(), "payload", "activity.payload")?;
        let payload = as_record(payload_v.as_ref());
        if strict_eq(kind_v.as_ref(), &str_val("runtime.warning"))
            && payload.is_some_and(|p| {
                strict_eq(
                    prop_opt(p, "nativeEventType").as_ref(),
                    &str_val("background_tasks_changed"),
                )
            })
        {
            latest.set(str_val("background-notice"), clone_cow(&activity));
            continue;
        }
        let task_id = payload.map(|p| prop_opt(p, "taskId").into_owned());
        let Some(task_id) = task_id else { continue };
        if kind(&task_id) != K::Str {
            continue;
        }
        if strict_eq(kind_v.as_ref(), &str_val("task.started")) {
            let id = prop(activity.as_ref(), "id", "activity.id")?;
            retained_ids.add(id.as_ref().clone());
            continue;
        }
        if !strict_eq(kind_v.as_ref(), &str_val("task.updated"))
            && !strict_eq(kind_v.as_ref(), &str_val("task.progress"))
            && !strict_eq(kind_v.as_ref(), &str_val("task.completed"))
        {
            continue;
        }
        if !strict_eq(kind_v.as_ref(), &str_val("task.progress")) {
            latest.set(
                str_val(&js_json(&Value::Array(vec![
                    task_id.clone(),
                    str_val("lifecycle"),
                ]))),
                clone_cow(&activity),
            );
        }
        if let Some(p) = payload {
            for field in object_keys(p) {
                if field != "taskId" {
                    latest.set(
                        str_val(&js_json(&Value::Array(vec![
                            task_id.clone(),
                            str_val("field"),
                            str_val(&field),
                        ]))),
                        clone_cow(&activity),
                    );
                }
            }
        }
    }
    for activity in latest.values() {
        let id = prop(activity.as_ref(), "id", "activity.id")?;
        retained_ids.add(id.as_ref().clone());
    }
    Ok(())
}

fn resolve_turn_aligned_drop_count(
    activities: &Value,
    minimum_drop_count: f64,
) -> Result<f64, String> {
    let boundary = idx_opt(activities, minimum_drop_count - 1.0);
    let boundary_turn_id = prop_opt(boundary.as_ref(), "turnId");
    let boundary_null = if is_nullish(boundary_turn_id.as_ref()) {
        Cow::Owned(Value::Null)
    } else {
        boundary_turn_id
    };
    let mut drop_count = minimum_drop_count;
    let len = len_num(activities, "activities.length")?;
    while drop_count < len {
        let el = idx_opt(activities, drop_count);
        let tid = prop_opt(el.as_ref(), "turnId");
        let tid_null = if is_nullish(tid.as_ref()) {
            Cow::Owned(Value::Null)
        } else {
            tid
        };
        if !strict_eq(tid_null.as_ref(), boundary_null.as_ref()) {
            break;
        }
        drop_count += 1.0;
    }
    Ok(if drop_count >= len {
        minimum_drop_count
    } else {
        drop_count
    })
}

fn cap_thread_activities_vec<'a>(activities: &'a Value) -> Result<Vec<Cow<'a, Value>>, String> {
    let len = len_num(activities, "activities.length")?;
    if len <= 2000.0 {
        return iter(activities, "activity", "activities");
    }
    let drop_count = resolve_turn_aligned_drop_count(activities, len - 2000.0)?;
    let tail = slice_call(
        activities,
        "activities.slice",
        "activities.slice(dropCount)",
        &[drop_count],
    )?;
    let tail_items = iter(tail.as_ref(), "activity", "tail")?;
    let mut retained_ids = JsSet::new();
    for activity in &tail_items {
        let id = prop(activity.as_ref(), "id", "activity.id")?;
        retained_ids.add(id.as_ref().clone());
    }
    retain_task_activity_ids(activities, &mut retained_ids)?;
    let identities = pending_interactions_from_activities(activities)?;
    let mut retained_requests = JsMap::new();
    for request in &identities {
        let key = prop(identity_obj(request), "key", "request.key")?;
        retained_requests.set(key.as_ref().clone(), Cow::Owned(request.clone()));
    }
    for activity in &tail_items {
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        if !PENDING_REQUEST_KINDS
            .iter()
            .any(|k| strict_eq(kind_v.as_ref(), &str_val(k)))
        {
            continue;
        }
        if let Some(identity) = pending_interaction_identity(activity.as_ref())? {
            let key = prop(identity_obj(&identity), "key", "identity.key")?;
            retained_requests.set(key.as_ref().clone(), Cow::Owned(identity));
        }
    }
    if retained_requests.size() == 0 {
        return filter_call(
            activities,
            "activities.filter",
            "activities.filter((activity) => retainedIds.has(activity.id))",
            &mut |activity: &Value, _| {
                let id = prop(activity, "id", "activity.id")?;
                Ok(retained_ids.has(id.as_ref()))
            },
        );
    }
    let mut oldest_retained_at = JsMap::new();
    for request in retained_requests.values() {
        let ik = prop(
            request.as_ref(),
            "interactionKind",
            "request.interactionKind",
        )?;
        let rid = prop(request.as_ref(), "requestId", "request.requestId")?;
        let key = format!(
            "{}:{}",
            to_string_js(ik.as_ref()),
            to_string_js(rid.as_ref())
        );
        let created = prop(request.as_ref(), "createdAt", "request.createdAt")?;
        let prev = oldest_retained_at.get(&str_val(&key));
        if prev.is_none() || js_lt(created.as_ref(), prev.unwrap_or(&Value::Null)) {
            oldest_retained_at.set(str_val(&key), Cow::Owned(created.as_ref().clone()));
        }
    }
    let mut latest_evidence = JsMap::new();
    for activity in iter(activities, "activity", "activities")? {
        let Some(identity) = pending_interaction_identity(activity.as_ref())? else {
            continue;
        };
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        let is_request = PENDING_REQUEST_KINDS
            .iter()
            .any(|k| strict_eq(kind_v.as_ref(), &str_val(k)));
        let ik = prop(
            identity_obj(&identity),
            "interactionKind",
            "identity.interactionKind",
        )?;
        let rid = prop(identity_obj(&identity), "requestId", "identity.requestId")?;
        let lg = prop(
            identity_obj(&identity),
            "lifecycleGeneration",
            "identity.lifecycleGeneration",
        )?;
        let id_key = prop(identity_obj(&identity), "key", "identity.key")?;
        let oldest_key = format!(
            "{}:{}",
            to_string_js(ik.as_ref()),
            to_string_js(rid.as_ref())
        );
        let oldest_at = oldest_retained_at.get(&str_val(&oldest_key));
        let keep = if !lg.is_null() {
            retained_requests.has(id_key.as_ref())
        } else if is_request {
            let ic = prop(identity_obj(&identity), "createdAt", "identity.createdAt")?.into_owned();
            retained_requests
                .get(id_key.as_ref())
                .is_some_and(|r| strict_eq(prop_opt(r, "createdAt").as_ref(), &ic))
        } else {
            let created = prop(identity_obj(&identity), "createdAt", "identity.createdAt")?;
            oldest_at.is_some_and(|o| js_le(o, created.as_ref()))
        };
        if !keep {
            continue;
        }
        if is_request {
            let id = prop(activity.as_ref(), "id", "activity.id")?;
            retained_ids.add(id.as_ref().clone());
            continue;
        }
        let detail = as_record(prop(activity.as_ref(), "payload", "activity.payload")?.as_ref())
            .map(|p| prop_opt(p, "detail").into_owned());
        let detail_arg = detail.as_ref().filter(|d| kind(d) == K::Str);
        let stale = (strict_eq(
            kind_v.as_ref(),
            &str_val("provider.approval.respond.failed"),
        ) || strict_eq(
            kind_v.as_ref(),
            &str_val("provider.user-input.respond.failed"),
        )) && is_stale_pending_request_failure_detail(detail_arg)?;
        let key = format!(
            "{}:{}{}",
            to_string_js(id_key.as_ref()),
            to_string_js(kind_v.as_ref()),
            if stale { ":stale" } else { "" }
        );
        let prev = latest_evidence.get(&str_val(&key)).cloned();
        let created = prop(activity.as_ref(), "createdAt", "activity.createdAt")?;
        let keep_new = match &prev {
            None => true,
            Some(p) => js_ge(
                created.as_ref(),
                prop(p, "createdAt", "previous.createdAt")?.as_ref(),
            ),
        };
        if keep_new {
            latest_evidence.set(str_val(&key), clone_cow(&activity));
        }
    }
    for activity in latest_evidence.values() {
        let id = prop(activity.as_ref(), "id", "activity.id")?;
        retained_ids.add(id.as_ref().clone());
    }
    filter_call(
        activities,
        "activities.filter",
        "activities.filter((activity) => retainedIds.has(activity.id))",
        &mut |activity: &Value, _| {
            let id = prop(activity, "id", "activity.id")?;
            Ok(retained_ids.has(id.as_ref()))
        },
    )
}

fn js_le(a: &Value, b: &Value) -> bool {
    matches!(js_cmp(a, b), Some(Ordering::Less | Ordering::Equal))
}

// ---------------------------------------------------------------------------
// Chat messages
// ---------------------------------------------------------------------------

fn encode_uri_component(v: &Value) -> Result<String, String> {
    let u = units(v);
    // Reject lone surrogates like encodeURIComponent does.
    let s = String::from_utf16(&u)
        .map_err(|_| "URIError: String contained an illegal UTF-16 sequence.".to_string())?;
    let mut out = String::new();
    for b in s.as_bytes() {
        let c = *b as char;
        if c.is_ascii_alphanumeric()
            || matches!(c, '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')')
        {
            out.push(c);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    Ok(out)
}

fn to_attachment_preview_url(raw: &Value) -> Result<Value, String> {
    // `window` is undefined in the worker: resolveWsHttpUrl returns the path.
    let u = units(raw);
    if u16_starts(&u, &u16s("/")) {
        return Ok(utf16_val(&u));
    }
    Ok(raw.clone())
}

fn normalize_chat_attachments(incoming: &Value, previous: &Value) -> Result<Value, String> {
    if !truthy(incoming)
        || strict_eq(
            prop(incoming, "length", "incoming.length")?.as_ref(),
            &num_val(0.0),
        )
    {
        return Ok(js::undefined());
    }
    let pairs = opt_map_call(
        previous,
        "previous?.map",
        "previous?.map((attachment) => [attachment.id, attachment])",
        &mut |attachment: Cow<'_, Value>, _| {
            let id = prop(attachment.as_ref(), "id", "attachment.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                attachment.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_id = map_of_pairs(pairs.unwrap_or_default());
    let next_attachments = map_call(
        incoming,
        "incoming.map",
        "incoming.map((attachment) => {\n    const nextAttachment = attachment.type === \"assistant-selection\" ? {\n      type: \"assistant-selection\",\n      id: attachment.id,\n      assistantMessageId: attachment.assistantMessageId,\n      text: attachment.text\n    } : attachment.type === \"file\" ? {\n      type: \"file\",\n      id: attachment.id,\n      name: attachment.name,\n      mimeType: attachment.mimeType,\n      sizeBytes: attachment.sizeBytes\n    } : {\n      type: \"image\",\n      ...(attachment.sourcePath !== undefined ? { sourcePath: attachment.sourcePath } : {}),\n      id: attachment.id,\n      name: attachment.name,\n      mimeType: attachment.mimeType,\n      sizeBytes: attachment.sizeBytes,\n      previewUrl: toAttachmentPreviewUrl(attachmentPreviewRoutePath(attachment.id))\n    };\n    const existing = previousById.get(attachment.id);\n    if (existing && (existing.type === \"assistant-selection\" && nextAttachment.type === \"assistant-selection\" && existing.assistantMessageId === nextAttachment.assistantMessageId && existing.text === nextAttachment.text || existing.type === \"image\" && nextAttachment.type === \"image\" && existing.name === nextAttachment.name && existing.mimeType === nextAttachment.mimeType && existing.sizeBytes === nextAttachment.sizeBytes && existing.sourcePath === nextAttachment.sourcePath && existing.previewUrl === nextAttachment.previewUrl || existing.type === \"file\" && nextAttachment.type === \"file\" && existing.name === nextAttachment.name && existing.mimeType === nextAttachment.mimeType && existing.sizeBytes === nextAttachment.sizeBytes))\n      return existing;\n    return nextAttachment;\n  })",
        &mut |attachment: Cow<'_, Value>, _| -> Result<Cow<'_, Value>, String> {
            let att = attachment.as_ref();
            let ty = prop(att, "type", "attachment.type")?;
            let mut o = Map::new();
            if strict_eq(ty.as_ref(), &str_val("assistant-selection")) {
                o.insert("type".into(), str_val("assistant-selection"));
                o.insert(
                    "id".into(),
                    prop(att, "id", "attachment.id")?.as_ref().clone(),
                );
                o.insert(
                    "assistantMessageId".into(),
                    prop(att, "assistantMessageId", "attachment.assistantMessageId")?
                        .as_ref()
                        .clone(),
                );
                o.insert(
                    "text".into(),
                    prop(att, "text", "attachment.text")?.as_ref().clone(),
                );
            } else if strict_eq(ty.as_ref(), &str_val("file")) {
                o.insert("type".into(), str_val("file"));
                o.insert(
                    "id".into(),
                    prop(att, "id", "attachment.id")?.as_ref().clone(),
                );
                o.insert(
                    "name".into(),
                    prop(att, "name", "attachment.name")?.as_ref().clone(),
                );
                o.insert(
                    "mimeType".into(),
                    prop(att, "mimeType", "attachment.mimeType")?
                        .as_ref()
                        .clone(),
                );
                o.insert(
                    "sizeBytes".into(),
                    prop(att, "sizeBytes", "attachment.sizeBytes")?
                        .as_ref()
                        .clone(),
                );
            } else {
                o.insert("type".into(), str_val("image"));
                let sp = prop(att, "sourcePath", "attachment.sourcePath")?;
                if !is_undef(sp.as_ref()) {
                    o.insert("sourcePath".into(), sp.as_ref().clone());
                }
                o.insert(
                    "id".into(),
                    prop(att, "id", "attachment.id")?.as_ref().clone(),
                );
                o.insert(
                    "name".into(),
                    prop(att, "name", "attachment.name")?.as_ref().clone(),
                );
                o.insert(
                    "mimeType".into(),
                    prop(att, "mimeType", "attachment.mimeType")?
                        .as_ref()
                        .clone(),
                );
                o.insert(
                    "sizeBytes".into(),
                    prop(att, "sizeBytes", "attachment.sizeBytes")?
                        .as_ref()
                        .clone(),
                );
                let id = prop(att, "id", "attachment.id")?;
                let encoded = encode_uri_component(id.as_ref())?;
                let route = format!("/attachments/{encoded}");
                o.insert(
                    "previewUrl".into(),
                    to_attachment_preview_url(&str_val(&route))?,
                );
            }
            let next_attachment = Value::Object(o);
            let id = prop(att, "id", "attachment.id")?;
            let existing = previous_by_id.get(id.as_ref());
            if let Some(existing) = existing {
                let next = &next_attachment;
                let e_ty = prop(existing, "type", "existing.type")?;
                let n_ty = prop(next, "type", "nextAttachment.type")?;
                let matches = (strict_eq(e_ty.as_ref(), &str_val("assistant-selection"))
                    && strict_eq(n_ty.as_ref(), &str_val("assistant-selection"))
                    && strict_eq(
                        prop(
                            existing,
                            "assistantMessageId",
                            "existing.assistantMessageId",
                        )?
                        .as_ref(),
                        prop(
                            next,
                            "assistantMessageId",
                            "nextAttachment.assistantMessageId",
                        )?
                        .as_ref(),
                    )
                    && strict_eq(
                        prop(existing, "text", "existing.text")?.as_ref(),
                        prop(next, "text", "nextAttachment.text")?.as_ref(),
                    ))
                    || (strict_eq(e_ty.as_ref(), &str_val("image"))
                        && strict_eq(n_ty.as_ref(), &str_val("image"))
                        && strict_eq(
                            prop(existing, "name", "existing.name")?.as_ref(),
                            prop(next, "name", "nextAttachment.name")?.as_ref(),
                        )
                        && strict_eq(
                            prop(existing, "mimeType", "existing.mimeType")?.as_ref(),
                            prop(next, "mimeType", "nextAttachment.mimeType")?.as_ref(),
                        )
                        && strict_eq(
                            prop(existing, "sizeBytes", "existing.sizeBytes")?.as_ref(),
                            prop(next, "sizeBytes", "nextAttachment.sizeBytes")?.as_ref(),
                        )
                        && strict_eq(
                            prop(existing, "sourcePath", "existing.sourcePath")?.as_ref(),
                            prop(next, "sourcePath", "nextAttachment.sourcePath")?.as_ref(),
                        )
                        && strict_eq(
                            prop(existing, "previewUrl", "existing.previewUrl")?.as_ref(),
                            prop(next, "previewUrl", "nextAttachment.previewUrl")?.as_ref(),
                        ))
                    || (strict_eq(e_ty.as_ref(), &str_val("file"))
                        && strict_eq(n_ty.as_ref(), &str_val("file"))
                        && strict_eq(
                            prop(existing, "name", "existing.name")?.as_ref(),
                            prop(next, "name", "nextAttachment.name")?.as_ref(),
                        )
                        && strict_eq(
                            prop(existing, "mimeType", "existing.mimeType")?.as_ref(),
                            prop(next, "mimeType", "nextAttachment.mimeType")?.as_ref(),
                        )
                        && strict_eq(
                            prop(existing, "sizeBytes", "existing.sizeBytes")?.as_ref(),
                            prop(next, "sizeBytes", "nextAttachment.sizeBytes")?.as_ref(),
                        ));
                if matches {
                    return Ok(Cow::Owned(existing.clone()));
                }
            }
            Ok(Cow::Owned(next_attachment))
        },
    )?;
    if arrays_shallow_eq(previous, &next_attachments)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            next_attachments
                .iter()
                .map(|c| c.as_ref().clone())
                .collect(),
        ))
    }
}

fn text_segments_eq<'a>(left: &'a Value, right: &'a Value) -> Result<bool, String> {
    if strict_eq(left, right) {
        return Ok(true);
    }
    if !truthy(left) || !truthy(right) {
        return Ok(false);
    }
    let ll = prop(left, "length", "left.length")?;
    let rl = prop(right, "length", "right.length")?;
    if !strict_eq(ll.as_ref(), rl.as_ref()) {
        return Ok(false);
    }
    let n = to_number(ll.as_ref()).max(0.0) as usize;
    for i in 0..n {
        let ls = idx(left, i as f64, "left[index]")?;
        let rs = idx(right, i as f64, "right[index]")?;
        for (f, le, re) in [
            ("sequence", "leftSegment.sequence", "rightSegment.sequence"),
            (
                "startedAt",
                "leftSegment.startedAt",
                "rightSegment.startedAt",
            ),
            ("endedAt", "leftSegment.endedAt", "rightSegment.endedAt"),
            ("text", "leftSegment.text", "rightSegment.text"),
        ] {
            if !strict_eq(
                prop(ls.as_ref(), f, le)?.as_ref(),
                prop(rs.as_ref(), f, re)?.as_ref(),
            ) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn provider_refs_eq<'a>(left: &'a Value, right: &'a Value) -> Result<bool, String> {
    if strict_eq(left, right) {
        return Ok(true);
    }
    if !truthy(left) || !truthy(right) {
        return Ok(false);
    }
    let ll = prop(left, "length", "left.length")?;
    let rl = prop(right, "length", "right.length")?;
    if !strict_eq(ll.as_ref(), rl.as_ref()) {
        return Ok(false);
    }
    let n = to_number(ll.as_ref()).max(0.0) as usize;
    for i in 0..n {
        let l = idx_opt(left, i as f64);
        let r = idx_opt(right, i as f64);
        if !strict_eq(
            prop_opt(l.as_ref(), "name").as_ref(),
            prop_opt(r.as_ref(), "name").as_ref(),
        ) || !strict_eq(
            prop_opt(l.as_ref(), "path").as_ref(),
            prop_opt(r.as_ref(), "path").as_ref(),
        ) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn normalize_chat_message_inner(incoming: &Value, previous: &Value) -> Result<Value, String> {
    let incoming_att = prop(incoming, "attachments", "incoming.attachments")?;
    let previous_att = prop_opt(previous, "attachments");
    let attachments = normalize_chat_attachments(incoming_att.as_ref(), previous_att.as_ref())?;
    let empty_arr = Value::Array(Vec::new());
    let incoming_skills = prop(incoming, "skills", "incoming.skills")?;
    let skills = if truthy(incoming_skills.as_ref())
        && js_gt(
            prop(incoming_skills.as_ref(), "length", "incoming.skills.length")?.as_ref(),
            &num_val(0.0),
        ) {
        clone_cow(&incoming_skills)
    } else {
        let v = prop_opt(previous, "skills");
        if is_nullish(v.as_ref()) {
            Cow::Borrowed(&empty_arr)
        } else {
            v
        }
    };
    let incoming_mentions = prop(incoming, "mentions", "incoming.mentions")?;
    let mentions = if truthy(incoming_mentions.as_ref())
        && js_gt(
            prop(
                incoming_mentions.as_ref(),
                "length",
                "incoming.mentions.length",
            )?
            .as_ref(),
            &num_val(0.0),
        ) {
        clone_cow(&incoming_mentions)
    } else {
        let v = prop_opt(previous, "mentions");
        if is_nullish(v.as_ref()) {
            Cow::Borrowed(&empty_arr)
        } else {
            v
        }
    };
    let previous_skills = {
        let v = prop_opt(previous, "skills");
        if is_nullish(v.as_ref()) {
            Cow::Borrowed(&empty_arr)
        } else {
            v
        }
    };
    let previous_mentions = {
        let v = prop_opt(previous, "mentions");
        if is_nullish(v.as_ref()) {
            Cow::Borrowed(&empty_arr)
        } else {
            v
        }
    };
    let incoming_streaming = prop(incoming, "streaming", "incoming.streaming")?;
    let completed_at = if truthy(incoming_streaming.as_ref()) {
        Cow::Owned(js::undefined())
    } else {
        prop(incoming, "updatedAt", "incoming.updatedAt")?
    };
    let previous_aui = prop_opt(previous, "asyncUserInput");
    let incoming_aui = prop(incoming, "asyncUserInput", "incoming.asyncUserInput")?;
    let async_user_input = merge_async_user_input(previous_aui.as_ref(), incoming_aui.as_ref())?;
    if truthy(previous) {
        let mut all = true;
        for (f, pe, ie) in [
            ("role", "previous.role", "incoming.role"),
            ("text", "previous.text", "incoming.text"),
        ] {
            if !strict_eq(
                prop(previous, f, pe)?.as_ref(),
                prop(incoming, f, ie)?.as_ref(),
            ) {
                all = false;
                break;
            }
        }
        if all {
            all = strict_eq(
                prop(previous, "asyncUserInput", "previous.asyncUserInput")?.as_ref(),
                &async_user_input,
            );
        }
        if all {
            for (f, pe, ie) in [
                (
                    "dispatchMode",
                    "previous.dispatchMode",
                    "incoming.dispatchMode",
                ),
                (
                    "dispatchOrigin",
                    "previous.dispatchOrigin",
                    "incoming.dispatchOrigin",
                ),
                (
                    "startsNewTurn",
                    "previous.startsNewTurn",
                    "incoming.startsNewTurn",
                ),
                ("turnId", "previous.turnId", "incoming.turnId"),
                ("createdAt", "previous.createdAt", "incoming.createdAt"),
                ("updatedAt", "previous.updatedAt", "incoming.updatedAt"),
                ("streaming", "previous.streaming", "incoming.streaming"),
                ("source", "previous.source", "incoming.source"),
            ] {
                if !strict_eq(
                    prop(previous, f, pe)?.as_ref(),
                    prop(incoming, f, ie)?.as_ref(),
                ) {
                    all = false;
                    break;
                }
            }
        }
        if all {
            all = strict_eq(
                prop(previous, "completedAt", "previous.completedAt")?.as_ref(),
                completed_at.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(previous, "attachments", "previous.attachments")?.as_ref(),
                &attachments,
            );
        }
        if all {
            all = text_segments_eq(
                prop(previous, "textSegments", "previous.textSegments")?.as_ref(),
                prop(incoming, "textSegments", "incoming.textSegments")?.as_ref(),
            )?;
        }
        if all {
            all = provider_refs_eq(previous_skills.as_ref(), skills.as_ref())?;
        }
        if all {
            all = provider_refs_eq(previous_mentions.as_ref(), mentions.as_ref())?;
        }
        if all {
            return Ok(previous.clone());
        }
    }
    let mut o = Map::new();
    o.insert(
        "id".into(),
        prop(incoming, "id", "incoming.id")?.as_ref().clone(),
    );
    o.insert(
        "role".into(),
        prop(incoming, "role", "incoming.role")?.as_ref().clone(),
    );
    o.insert(
        "text".into(),
        prop(incoming, "text", "incoming.text")?.as_ref().clone(),
    );
    if truthy(&async_user_input) {
        o.insert("asyncUserInput".into(), async_user_input.clone());
    }
    let ts = prop(incoming, "textSegments", "incoming.textSegments")?;
    if !is_undef(ts.as_ref())
        && js_gt(
            prop(ts.as_ref(), "length", "incoming.textSegments.length")?.as_ref(),
            &num_val(0.0),
        )
    {
        let spread = spread_iter(ts.as_ref())?;
        o.insert(
            "textSegments".into(),
            Value::Array(spread.iter().map(|c| c.as_ref().clone()).collect()),
        );
    }
    for f in ["dispatchMode", "dispatchOrigin"] {
        let v = prop(incoming, f, &format!("incoming.{f}"))?;
        if truthy(v.as_ref()) {
            o.insert(f.into(), v.as_ref().clone());
        }
    }
    let snt = prop(incoming, "startsNewTurn", "incoming.startsNewTurn")?;
    if !is_undef(snt.as_ref()) {
        o.insert("startsNewTurn".into(), snt.as_ref().clone());
    }
    o.insert(
        "turnId".into(),
        prop(incoming, "turnId", "incoming.turnId")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "createdAt".into(),
        prop(incoming, "createdAt", "incoming.createdAt")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "updatedAt".into(),
        prop(incoming, "updatedAt", "incoming.updatedAt")?
            .as_ref()
            .clone(),
    );
    o.insert("streaming".into(), incoming_streaming.as_ref().clone());
    o.insert(
        "source".into(),
        prop(incoming, "source", "incoming.source")?
            .as_ref()
            .clone(),
    );
    if truthy(completed_at.as_ref()) {
        o.insert("completedAt".into(), completed_at.as_ref().clone());
    }
    if truthy(&attachments) {
        o.insert("attachments".into(), attachments.clone());
    }
    if js_gt(
        prop(skills.as_ref(), "length", "skills.length")?.as_ref(),
        &num_val(0.0),
    ) {
        let spread = spread_iter(skills.as_ref())?;
        o.insert(
            "skills".into(),
            Value::Array(spread.iter().map(|c| c.as_ref().clone()).collect()),
        );
    }
    if js_gt(
        prop(mentions.as_ref(), "length", "mentions.length")?.as_ref(),
        &num_val(0.0),
    ) {
        let spread = spread_iter(mentions.as_ref())?;
        o.insert(
            "mentions".into(),
            Value::Array(spread.iter().map(|c| c.as_ref().clone()).collect()),
        );
    }
    Ok(Value::Object(o))
}

fn normalize_chat_messages(incoming: &Value, previous: &Value) -> Result<Value, String> {
    let pairs = opt_map_call(
        previous,
        "previous?.map",
        "previous?.map((message) => [message.id, message])",
        &mut |message: Cow<'_, Value>, _| {
            let id = prop(message.as_ref(), "id", "message.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                message.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_id = map_of_pairs(pairs.unwrap_or_default());
    let sliced = slice_call(
        incoming,
        "incoming.slice",
        "incoming.slice(-MAX_THREAD_MESSAGES)",
        &[-2000.0],
    )?;
    let next_messages = map_call(
        sliced.as_ref(),
        "incoming.slice(-MAX_THREAD_MESSAGES).map",
        "incoming.slice(-MAX_THREAD_MESSAGES).map((message) => normalizeChatMessage(message, previousById.get(message.id)))",
        &mut |message: Cow<'_, Value>, _| {
            let id = prop(message.as_ref(), "id", "message.id")?;
            let prev = previous_by_id.get(id.as_ref()).unwrap_or(undef());
            normalize_chat_message_inner(message.as_ref(), prev).map(Cow::Owned)
        },
    )?;
    if arrays_shallow_eq(previous, &next_messages)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            next_messages.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn read_model_attachments(attachments: &Value) -> Result<Value, String> {
    let pairs = opt_map_call(
        attachments,
        "attachments?.map",
        "attachments?.map((attachment) => attachment.type === \"assistant-selection\" ? {\n      id: attachment.id,\n      type: \"assistant-selection\",\n      assistantMessageId: MessageId.makeUnsafe(attachment.assistantMessageId),\n      text: attachment.text\n    } : attachment.type === \"file\" ? {\n      id: attachment.id,\n      name: attachment.name,\n      type: \"file\",\n      mimeType: attachment.mimeType,\n      sizeBytes: attachment.sizeBytes\n    } : {\n      id: attachment.id,\n      name: attachment.name,\n      type: \"image\",\n      ...(attachment.sourcePath !== undefined ? { sourcePath: attachment.sourcePath } : {}),\n      mimeType: attachment.mimeType,\n      sizeBytes: attachment.sizeBytes\n    })",
        &mut |attachment: Cow<'_, Value>, _| {
            let att = attachment.as_ref();
            let ty = prop(att, "type", "attachment.type")?;
            let mut o = Map::new();
            if strict_eq(ty.as_ref(), &str_val("assistant-selection")) {
                o.insert(
                    "id".into(),
                    prop(att, "id", "attachment.id")?.as_ref().clone(),
                );
                o.insert("type".into(), str_val("assistant-selection"));
                o.insert(
                    "assistantMessageId".into(),
                    prop(att, "assistantMessageId", "attachment.assistantMessageId")?
                        .as_ref()
                        .clone(),
                );
                o.insert(
                    "text".into(),
                    prop(att, "text", "attachment.text")?.as_ref().clone(),
                );
            } else if strict_eq(ty.as_ref(), &str_val("file")) {
                o.insert(
                    "id".into(),
                    prop(att, "id", "attachment.id")?.as_ref().clone(),
                );
                o.insert(
                    "name".into(),
                    prop(att, "name", "attachment.name")?.as_ref().clone(),
                );
                o.insert("type".into(), str_val("file"));
                o.insert(
                    "mimeType".into(),
                    prop(att, "mimeType", "attachment.mimeType")?
                        .as_ref()
                        .clone(),
                );
                o.insert(
                    "sizeBytes".into(),
                    prop(att, "sizeBytes", "attachment.sizeBytes")?
                        .as_ref()
                        .clone(),
                );
            } else {
                o.insert(
                    "id".into(),
                    prop(att, "id", "attachment.id")?.as_ref().clone(),
                );
                o.insert(
                    "name".into(),
                    prop(att, "name", "attachment.name")?.as_ref().clone(),
                );
                o.insert("type".into(), str_val("image"));
                let sp = prop(att, "sourcePath", "attachment.sourcePath")?;
                if !is_undef(sp.as_ref()) {
                    o.insert("sourcePath".into(), sp.as_ref().clone());
                }
                o.insert(
                    "mimeType".into(),
                    prop(att, "mimeType", "attachment.mimeType")?
                        .as_ref()
                        .clone(),
                );
                o.insert(
                    "sizeBytes".into(),
                    prop(att, "sizeBytes", "attachment.sizeBytes")?
                        .as_ref()
                        .clone(),
                );
            }
            Ok(Cow::Owned(Value::Object(o)))
        },
    )?;
    Ok(Value::Array(
        pairs
            .unwrap_or_default()
            .iter()
            .map(|c| c.as_ref().clone())
            .collect(),
    ))
}

fn read_model_message(message: &Value) -> Result<Value, String> {
    let mut o = Map::new();
    o.insert(
        "id".into(),
        prop(message, "id", "message.id")?.as_ref().clone(),
    );
    o.insert(
        "role".into(),
        prop(message, "role", "message.role")?.as_ref().clone(),
    );
    o.insert(
        "text".into(),
        prop(message, "text", "message.text")?.as_ref().clone(),
    );
    let aui = prop(message, "asyncUserInput", "message.asyncUserInput")?;
    if truthy(aui.as_ref()) {
        o.insert("asyncUserInput".into(), aui.as_ref().clone());
    }
    for f in ["dispatchMode", "dispatchOrigin"] {
        let v = prop(message, f, &format!("message.{f}"))?;
        if truthy(v.as_ref()) {
            o.insert(f.into(), v.as_ref().clone());
        }
    }
    let snt = prop(message, "startsNewTurn", "message.startsNewTurn")?;
    if !is_undef(snt.as_ref()) {
        o.insert("startsNewTurn".into(), snt.as_ref().clone());
    }
    let tid = prop(message, "turnId", "message.turnId")?;
    o.insert(
        "turnId".into(),
        if is_nullish(tid.as_ref()) {
            Value::Null
        } else {
            tid.as_ref().clone()
        },
    );
    o.insert(
        "streaming".into(),
        prop(message, "streaming", "message.streaming")?
            .as_ref()
            .clone(),
    );
    let src = prop(message, "source", "message.source")?;
    o.insert(
        "source".into(),
        if is_nullish(src.as_ref()) {
            str_val("native")
        } else {
            src.as_ref().clone()
        },
    );
    o.insert(
        "createdAt".into(),
        prop(message, "createdAt", "message.createdAt")?
            .as_ref()
            .clone(),
    );
    let upd = prop(message, "updatedAt", "message.updatedAt")?;
    let upd2 = if is_nullish(upd.as_ref()) {
        let comp = prop(message, "completedAt", "message.completedAt")?;
        if is_nullish(comp.as_ref()) {
            prop(message, "createdAt", "message.createdAt")?
                .as_ref()
                .clone()
        } else {
            comp.as_ref().clone()
        }
    } else {
        upd.as_ref().clone()
    };
    o.insert("updatedAt".into(), upd2);
    o.insert(
        "attachments".into(),
        read_model_attachments(prop(message, "attachments", "message.attachments")?.as_ref())?,
    );
    let skills = prop(message, "skills", "message.skills")?;
    if truthy(skills.as_ref())
        && js_gt(
            prop(skills.as_ref(), "length", "message.skills.length")?.as_ref(),
            &num_val(0.0),
        )
    {
        o.insert("skills".into(), skills.as_ref().clone());
    }
    let mentions = prop(message, "mentions", "message.mentions")?;
    if truthy(mentions.as_ref())
        && js_gt(
            prop(mentions.as_ref(), "length", "message.mentions.length")?.as_ref(),
            &num_val(0.0),
        )
    {
        o.insert("mentions".into(), mentions.as_ref().clone());
    }
    Ok(Value::Object(o))
}

fn is_session_or_latest_turn_running(status: &Value, turn_state: &Value) -> bool {
    strict_eq(status, &str_val("running")) || strict_eq(turn_state, &str_val("running"))
}

fn should_retain_assistant<'a>(
    previous_thread: &'a Value,
    message: &'a Value,
) -> Result<bool, String> {
    if truthy(prop(message, "streaming", "message.streaming")?.as_ref()) {
        return Ok(true);
    }
    let latest_turn = prop(previous_thread, "latestTurn", "previousThread.latestTurn")?;
    if !truthy(&latest_turn) {
        return Ok(false);
    }
    if strict_eq(
        prop(
            &latest_turn,
            "assistantMessageId",
            "latestTurn.assistantMessageId",
        )?
        .as_ref(),
        prop(message, "id", "message.id")?.as_ref(),
    ) {
        return Ok(true);
    }
    let session = prop(previous_thread, "session", "previousThread.session")?;
    Ok(strict_eq(
        prop_opt(&session, "orchestrationStatus").as_ref(),
        &str_val("running"),
    ) && !is_undef(prop(message, "turnId", "message.turnId")?.as_ref())
        && strict_eq(
            prop(&latest_turn, "turnId", "latestTurn.turnId")?.as_ref(),
            prop(message, "turnId", "message.turnId")?.as_ref(),
        ))
}

fn should_retain_user<'a>(previous_thread: &'a Value, message: &'a Value) -> Result<bool, String> {
    let latest_turn = prop(previous_thread, "latestTurn", "previousThread.latestTurn")?;
    if truthy(&latest_turn)
        && !is_nullish(prop(message, "turnId", "message.turnId")?.as_ref())
        && strict_eq(
            prop(message, "turnId", "message.turnId")?.as_ref(),
            prop(&latest_turn, "turnId", "latestTurn.turnId")?.as_ref(),
        )
    {
        let session = prop(previous_thread, "session", "previousThread.session")?;
        return Ok(is_session_or_latest_turn_running(
            prop_opt(&session, "orchestrationStatus").as_ref(),
            prop(&latest_turn, "state", "latestTurn.state")?.as_ref(),
        ));
    }
    let created = prop(message, "createdAt", "message.createdAt")?;
    let created_ms = date_parse_val(created.as_ref());
    Ok(created_ms.is_finite() && js_now() - created_ms <= 10_000.0)
}

fn js_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

fn date_parse_val(v: &Value) -> f64 {
    if is_nullish(v) {
        return corpus::date_parse("");
    }
    corpus::date_parse(&to_string_js(v))
}

fn should_retain_live<'a>(previous_thread: &'a Value, message: &'a Value) -> Result<bool, String> {
    let role = prop(message, "role", "message.role")?;
    if strict_eq(role.as_ref(), &str_val("assistant")) {
        should_retain_assistant(previous_thread, message)
    } else if strict_eq(role.as_ref(), &str_val("user")) {
        should_retain_user(previous_thread, message)
    } else {
        Ok(false)
    }
}

fn text_segments_cover(segments: &Value, text: &Value) -> Result<bool, String> {
    let mut collated: Vec<u16> = Vec::new();
    for segment in iter(segments, "segment", "segments")? {
        let t = prop(segment.as_ref(), "text", "segment.text")?;
        collated.extend(&units(t.as_ref()));
    }
    Ok(collated == units(text))
}

fn merge_messages<'a>(
    incoming_messages: &'a Value,
    previous_thread: &Value,
    authoritative_turn_id: &Value,
) -> Result<Value, String> {
    if !truthy(previous_thread)
        || strict_eq(
            prop(
                prop(previous_thread, "messages", "previousThread.messages")?.as_ref(),
                "length",
                "previousThread.messages.length",
            )?
            .as_ref(),
            &num_val(0.0),
        )
    {
        return Ok(incoming_messages.clone());
    }
    let prev_messages = prop(previous_thread, "messages", "previousThread.messages")?;
    let pairs = map_call(
        prev_messages.as_ref(),
        "previousThread.messages.map",
        "previousThread.messages.map((message) => [message.id, message])",
        &mut |message: Cow<'_, Value>, _| {
            let id = prop(message.as_ref(), "id", "message.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                message.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_id = map_of_pairs(pairs);
    let mut merged: JsMap = JsMap::new();
    let mut changed = false;
    for incoming_message in iter(incoming_messages, "incomingMessage", "incomingMessages")? {
        let im = incoming_message.as_ref();
        let id = prop(im, "id", "incomingMessage.id")?;
        let previous_message = previous_by_id.get(id.as_ref());
        let role_differs = match previous_message {
            Some(p) => !strict_eq(
                prop(p, "role", "previousMessage.role")?.as_ref(),
                prop(im, "role", "incomingMessage.role")?.as_ref(),
            ),
            None => true,
        };
        if previous_message.is_none() || role_differs {
            merged.set(
                id.as_ref().clone(),
                Cow::Owned(incoming_message.as_ref().clone()),
            );
            continue;
        }
        let pm = previous_message.unwrap_or(undef());
        let incoming_completed_at =
            if truthy(prop(im, "streaming", "incomingMessage.streaming")?.as_ref()) {
                Cow::Owned(js::undefined())
            } else {
                prop(im, "updatedAt", "incomingMessage.updatedAt")?
            };
        let local_ahead = truthy(prop(pm, "streaming", "previousMessage.streaming")?.as_ref())
            && js_gt(
                prop(
                    prop(pm, "text", "previousMessage.text")?.as_ref(),
                    "length",
                    "previousMessage.text.length",
                )?
                .as_ref(),
                prop(
                    prop(im, "text", "incomingMessage.text")?.as_ref(),
                    "length",
                    "incomingMessage.text.length",
                )?
                .as_ref(),
            );
        let prefer_live = (authoritative_turn_id.is_null()
            || !strict_eq(
                prop(im, "turnId", "incomingMessage.turnId")?.as_ref(),
                authoritative_turn_id,
            ))
            && (local_ahead
                || (!truthy(prop(pm, "streaming", "previousMessage.streaming")?.as_ref())
                    && truthy(prop(im, "streaming", "incomingMessage.streaming")?.as_ref()))
                || (!is_undef(prop(pm, "completedAt", "previousMessage.completedAt")?.as_ref())
                    && (is_undef(incoming_completed_at.as_ref())
                        || js_gt(
                            prop(pm, "completedAt", "previousMessage.completedAt")?.as_ref(),
                            incoming_completed_at.as_ref(),
                        ))));
        if !prefer_live {
            let mut o = Map::new();
            for (k, x) in spread_obj(im) {
                o.insert(k, x);
            }
            let im_mentions = prop(im, "mentions", "incomingMessage.mentions")?;
            let mentions_empty = !truthy(im_mentions.as_ref())
                || strict_eq(
                    prop(
                        im_mentions.as_ref(),
                        "length",
                        "incomingMessage.mentions.length",
                    )?
                    .as_ref(),
                    &num_val(0.0),
                );
            if mentions_empty {
                let pm_mentions = prop(pm, "mentions", "previousMessage.mentions")?;
                if truthy(pm_mentions.as_ref())
                    && js_gt(
                        prop(
                            pm_mentions.as_ref(),
                            "length",
                            "previousMessage.mentions.length",
                        )?
                        .as_ref(),
                        &num_val(0.0),
                    )
                {
                    o.insert("mentions".into(), pm_mentions.as_ref().clone());
                }
            }
            let im_skills = prop(im, "skills", "incomingMessage.skills")?;
            let skills_empty = !truthy(im_skills.as_ref())
                || strict_eq(
                    prop(
                        im_skills.as_ref(),
                        "length",
                        "incomingMessage.skills.length",
                    )?
                    .as_ref(),
                    &num_val(0.0),
                );
            if skills_empty {
                let pm_skills = prop(pm, "skills", "previousMessage.skills")?;
                if truthy(pm_skills.as_ref())
                    && js_gt(
                        prop(
                            pm_skills.as_ref(),
                            "length",
                            "previousMessage.skills.length",
                        )?
                        .as_ref(),
                        &num_val(0.0),
                    )
                {
                    o.insert("skills".into(), pm_skills.as_ref().clone());
                }
            }
            merged.set(id.as_ref().clone(), Cow::Owned(Value::Object(o)));
            continue;
        }
        changed = true;
        let candidates: Vec<Cow<'_, Value>> = vec![
            prop(im, "textSegments", "incomingMessage.textSegments")?,
            prop(pm, "textSegments", "previousMessage.textSegments")?,
        ];
        let pm_text = prop(pm, "text", "previousMessage.text")?;
        let retained = find_call(&candidates, &mut |segments: &Value| {
            if is_undef(segments) {
                return Ok(false);
            }
            if !js_gt(
                prop(segments, "length", "segments.length")?.as_ref(),
                &num_val(0.0),
            ) {
                return Ok(false);
            }
            text_segments_cover(segments, pm_text.as_ref())
        })?;
        let mut o = Map::new();
        for (k, x) in spread_obj(im) {
            if k != "textSegments" {
                o.insert(k, x);
            }
        }
        if let Some(segments) = retained
            && !is_undef(segments.as_ref())
        {
            o.insert("textSegments".into(), segments.as_ref().clone());
        }
        o.insert("text".into(), pm_text.as_ref().clone());
        let dm = prop(pm, "dispatchMode", "previousMessage.dispatchMode")?;
        o.insert(
            "dispatchMode".into(),
            if is_nullish(dm.as_ref()) {
                prop(im, "dispatchMode", "incomingMessage.dispatchMode")?
                    .as_ref()
                    .clone()
            } else {
                dm.as_ref().clone()
            },
        );
        let io = prop(im, "dispatchOrigin", "incomingMessage.dispatchOrigin")?;
        o.insert(
            "dispatchOrigin".into(),
            if is_nullish(io.as_ref()) {
                prop(pm, "dispatchOrigin", "previousMessage.dispatchOrigin")?
                    .as_ref()
                    .clone()
            } else {
                io.as_ref().clone()
            },
        );
        let snt = prop(im, "startsNewTurn", "incomingMessage.startsNewTurn")?;
        o.insert(
            "startsNewTurn".into(),
            if is_nullish(snt.as_ref()) {
                prop(pm, "startsNewTurn", "previousMessage.startsNewTurn")?
                    .as_ref()
                    .clone()
            } else {
                snt.as_ref().clone()
            },
        );
        o.insert(
            "asyncUserInput".into(),
            merge_async_user_input(
                prop(pm, "asyncUserInput", "previousMessage.asyncUserInput")?.as_ref(),
                prop(im, "asyncUserInput", "incomingMessage.asyncUserInput")?.as_ref(),
            )?,
        );
        let pt = prop(pm, "turnId", "previousMessage.turnId")?;
        let tid = if is_nullish(pt.as_ref()) {
            let it = prop(im, "turnId", "incomingMessage.turnId")?;
            if is_nullish(it.as_ref()) {
                Value::Null
            } else {
                it.as_ref().clone()
            }
        } else {
            pt.as_ref().clone()
        };
        o.insert("turnId".into(), tid);
        let ps = prop(pm, "source", "previousMessage.source")?;
        let source = if is_nullish(ps.as_ref()) {
            let is = prop(im, "source", "incomingMessage.source")?;
            if is_nullish(is.as_ref()) {
                str_val("native")
            } else {
                is.as_ref().clone()
            }
        } else {
            ps.as_ref().clone()
        };
        o.insert("source".into(), source);
        o.insert(
            "streaming".into(),
            prop(pm, "streaming", "previousMessage.streaming")?
                .as_ref()
                .clone(),
        );
        let pu = prop(pm, "updatedAt", "previousMessage.updatedAt")?;
        let updated = if is_nullish(pu.as_ref()) {
            let pc = prop(pm, "completedAt", "previousMessage.completedAt")?;
            if is_nullish(pc.as_ref()) {
                prop(im, "updatedAt", "incomingMessage.updatedAt")?
                    .as_ref()
                    .clone()
            } else {
                pc.as_ref().clone()
            }
        } else {
            pu.as_ref().clone()
        };
        o.insert("updatedAt".into(), updated);
        o.insert(
            "attachments".into(),
            read_model_attachments(
                prop(pm, "attachments", "previousMessage.attachments")?.as_ref(),
            )?,
        );
        let pskills = prop(pm, "skills", "previousMessage.skills")?;
        if truthy(pskills.as_ref())
            && js_gt(
                prop(pskills.as_ref(), "length", "previousMessage.skills.length")?.as_ref(),
                &num_val(0.0),
            )
        {
            o.insert("skills".into(), pskills.as_ref().clone());
        }
        let pmen = prop(pm, "mentions", "previousMessage.mentions")?;
        if truthy(pmen.as_ref())
            && js_gt(
                prop(pmen.as_ref(), "length", "previousMessage.mentions.length")?.as_ref(),
                &num_val(0.0),
            )
        {
            o.insert("mentions".into(), pmen.as_ref().clone());
        }
        merged.set(id.as_ref().clone(), Cow::Owned(Value::Object(o)));
    }
    for previous_message in iter(
        prev_messages.as_ref(),
        "previousMessage",
        "previousThread.messages",
    )? {
        let pm = previous_message.as_ref();
        let id = prop(pm, "id", "previousMessage.id")?;
        if merged.has(id.as_ref()) {
            continue;
        }
        if !should_retain_live(previous_thread, pm)? {
            continue;
        }
        changed = true;
        merged.set(id.as_ref().clone(), Cow::Owned(read_model_message(pm)?));
    }
    if !changed {
        return Ok(incoming_messages.clone());
    }
    let mut vals: Vec<Cow<'a, Value>> = merged
        .values()
        .map(|c| Cow::Owned(c.as_ref().clone()))
        .collect();
    js_sort_res(&mut vals, &mut |l: &Cow<'a, Value>, r: &Cow<'a, Value>| {
        let lc = prop(l.as_ref(), "createdAt", "left.createdAt")?;
        let rc = prop(r.as_ref(), "createdAt", "right.createdAt")?;
        locale_call(
            lc.as_ref(),
            "left.createdAt.localeCompare",
            "left.createdAt.localeCompare(right.createdAt)",
            rc.as_ref(),
        )
    })?;
    Ok(Value::Array(
        vals.iter().map(|c| c.as_ref().clone()).collect(),
    ))
}

fn has_live_assistant_intro(previous_thread: &Value) -> Result<bool, String> {
    if !truthy(previous_thread) {
        return Ok(false);
    }
    let latest_turn = prop(previous_thread, "latestTurn", "previousThread.latestTurn")?;
    if !truthy(&latest_turn)
        || !strict_eq(
            prop(&latest_turn, "state", "latestTurn.state")?.as_ref(),
            &str_val("running"),
        )
    {
        return Ok(false);
    }
    let session = prop(previous_thread, "session", "previousThread.session")?;
    if !strict_eq(
        prop_opt(&session, "orchestrationStatus").as_ref(),
        &str_val("running"),
    ) {
        return Ok(false);
    }
    let latest_turn_id = prop(&latest_turn, "turnId", "latestTurn.turnId")?;
    let messages = prop(previous_thread, "messages", "previousThread.messages")?;
    some_call(
        messages.as_ref(),
        "previousThread.messages.some",
        "previousThread.messages.some((message) => message.role === \"assistant\" && message.turnId === latestTurn.turnId && (message.streaming || message.id === latestTurn.assistantMessageId))",
        &mut |message: &Value, _| {
            if !strict_eq(
                prop(message, "role", "message.role")?.as_ref(),
                &str_val("assistant"),
            ) {
                return Ok(false);
            }
            if !strict_eq(
                prop(message, "turnId", "message.turnId")?.as_ref(),
                latest_turn_id.as_ref(),
            ) {
                return Ok(false);
            }
            Ok(
                truthy(prop(message, "streaming", "message.streaming")?.as_ref())
                    || strict_eq(
                        prop(message, "id", "message.id")?.as_ref(),
                        prop(
                            &latest_turn,
                            "assistantMessageId",
                            "latestTurn.assistantMessageId",
                        )?
                        .as_ref(),
                    ),
            )
        },
    )
}

fn should_preserve_running_turn<'a>(
    previous_thread: &'a Value,
    incoming: &'a Value,
) -> Result<bool, String> {
    if !has_live_assistant_intro(previous_thread)? {
        return Ok(false);
    }
    let lt = prop_opt(previous_thread, "latestTurn");
    let prev_turn_id = prop_opt(lt.as_ref(), "turnId");
    if !truthy(prev_turn_id.as_ref()) {
        return Ok(false);
    }
    let ilt = prop(incoming, "latestTurn", "incoming.latestTurn")?;
    if !strict_eq(
        prop_opt(ilt.as_ref(), "turnId").as_ref(),
        prev_turn_id.as_ref(),
    ) {
        return Ok(true);
    }
    if truthy(
        prop(
            ilt.as_ref(),
            "completedAt",
            "incoming.latestTurn.completedAt",
        )?
        .as_ref(),
    ) {
        return Ok(false);
    }
    Ok(true)
}

fn read_model_session<'a>(
    previous_session: &'a Value,
    previous_thread: &'a Value,
    incoming_session: &'a Value,
) -> Result<Value, String> {
    let mut o = Map::new();
    let ptid = prop_opt(previous_thread, "id");
    let thread_id = if !is_nullish(ptid.as_ref()) {
        ptid.as_ref().clone()
    } else {
        let ist = prop_opt(incoming_session, "threadId");
        if !is_nullish(ist.as_ref()) {
            ist.as_ref().clone()
        } else {
            str_val("unknown")
        }
    };
    o.insert("threadId".into(), thread_id);
    o.insert(
        "status".into(),
        prop(
            previous_session,
            "orchestrationStatus",
            "previousSession.orchestrationStatus",
        )?
        .as_ref()
        .clone(),
    );
    o.insert(
        "providerName".into(),
        prop(previous_session, "provider", "previousSession.provider")?
            .as_ref()
            .clone(),
    );
    let piid = prop(
        previous_session,
        "providerInstanceId",
        "previousSession.providerInstanceId",
    )?;
    if !is_undef(piid.as_ref()) {
        o.insert("providerInstanceId".into(), piid.as_ref().clone());
    }
    let rm = prop(
        previous_session,
        "runtimeMode",
        "previousSession.runtimeMode",
    )?;
    let runtime_mode = if !is_nullish(rm.as_ref()) {
        rm.as_ref().clone()
    } else {
        let ptr = prop_opt(previous_thread, "runtimeMode");
        if !is_nullish(ptr.as_ref()) {
            ptr.as_ref().clone()
        } else {
            let isr = prop_opt(incoming_session, "runtimeMode");
            if !is_nullish(isr.as_ref()) {
                isr.as_ref().clone()
            } else {
                str_val("full-access")
            }
        }
    };
    o.insert("runtimeMode".into(), runtime_mode);
    let atid = prop(
        previous_session,
        "activeTurnId",
        "previousSession.activeTurnId",
    )?;
    o.insert(
        "activeTurnId".into(),
        if is_nullish(atid.as_ref()) {
            Value::Null
        } else {
            atid.as_ref().clone()
        },
    );
    let le = prop(previous_session, "lastError", "previousSession.lastError")?;
    o.insert(
        "lastError".into(),
        if is_nullish(le.as_ref()) {
            Value::Null
        } else {
            le.as_ref().clone()
        },
    );
    o.insert(
        "updatedAt".into(),
        prop(previous_session, "updatedAt", "previousSession.updatedAt")?
            .as_ref()
            .clone(),
    );
    Ok(Value::Object(o))
}

fn merge_session<'a>(
    incoming_session: &'a Value,
    previous_thread: &Value,
    preserve_running_turn: bool,
    incoming_latest_turn: &'a Value,
) -> Result<Value, String> {
    let previous_session = prop_opt(previous_thread, "session");
    if !truthy(&previous_session) || !preserve_running_turn {
        return Ok(incoming_session.clone());
    }
    let ps = &previous_session;
    if !truthy(incoming_session) {
        if strict_eq(
            prop(
                ps,
                "orchestrationStatus",
                "previousSession.orchestrationStatus",
            )?
            .as_ref(),
            &str_val("running"),
        ) {
            return read_model_session(ps, previous_thread, incoming_session);
        }
        return Ok(incoming_session.clone());
    }
    if js_gt(
        prop(ps, "updatedAt", "previousSession.updatedAt")?.as_ref(),
        prop(incoming_session, "updatedAt", "incomingSession.updatedAt")?.as_ref(),
    ) {
        let next = read_model_session(ps, previous_thread, incoming_session)?;
        let mut o = Map::new();
        for (k, x) in spread_obj(&next) {
            o.insert(k, x);
        }
        o.insert(
            "providerName".into(),
            prop(
                incoming_session,
                "providerName",
                "incomingSession.providerName",
            )?
            .as_ref()
            .clone(),
        );
        let ipiid = prop(
            incoming_session,
            "providerInstanceId",
            "incomingSession.providerInstanceId",
        )?;
        if !is_undef(ipiid.as_ref()) {
            o.insert("providerInstanceId".into(), ipiid.as_ref().clone());
        } else if !is_undef(piid_helper(ps)?.as_ref()) {
            o.insert(
                "providerInstanceId".into(),
                prop(
                    ps,
                    "providerInstanceId",
                    "previousSession.providerInstanceId",
                )?
                .as_ref()
                .clone(),
            );
        }
        o.insert(
            "runtimeMode".into(),
            prop(
                incoming_session,
                "runtimeMode",
                "incomingSession.runtimeMode",
            )?
            .as_ref()
            .clone(),
        );
        let pat = prop(ps, "activeTurnId", "previousSession.activeTurnId")?;
        o.insert(
            "activeTurnId".into(),
            if !is_nullish(pat.as_ref()) {
                pat.as_ref().clone()
            } else {
                prop(
                    incoming_session,
                    "activeTurnId",
                    "incomingSession.activeTurnId",
                )?
                .as_ref()
                .clone()
            },
        );
        let ple = prop(ps, "lastError", "previousSession.lastError")?;
        o.insert(
            "lastError".into(),
            if !is_nullish(ple.as_ref()) {
                ple.as_ref().clone()
            } else {
                prop(incoming_session, "lastError", "incomingSession.lastError")?
                    .as_ref()
                    .clone()
            },
        );
        return Ok(Value::Object(o));
    }
    let superseded = js_gt(
        prop(incoming_session, "updatedAt", "incomingSession.updatedAt")?.as_ref(),
        prop(ps, "updatedAt", "previousSession.updatedAt")?.as_ref(),
    ) && !is_nullish(incoming_latest_turn)
        && !is_nullish(prop_opt(incoming_latest_turn, "completedAt").as_ref())
        && {
            let prev_lt = prop_opt(previous_thread, "latestTurn");
            let prev_tid = prop_opt(prev_lt.as_ref(), "turnId");
            !strict_eq(
                prop(incoming_latest_turn, "turnId", "incomingLatestTurn.turnId")?.as_ref(),
                prev_tid.as_ref(),
            )
        };
    if strict_eq(
        prop(
            ps,
            "orchestrationStatus",
            "previousSession.orchestrationStatus",
        )?
        .as_ref(),
        &str_val("running"),
    ) && !strict_eq(
        prop(incoming_session, "status", "incomingSession.status")?.as_ref(),
        &str_val("running"),
    ) && !strict_eq(
        prop(incoming_session, "status", "incomingSession.status")?.as_ref(),
        &str_val("error"),
    ) && !is_undef(prop(ps, "activeTurnId", "previousSession.activeTurnId")?.as_ref())
        && !superseded
    {
        let mut o = Map::new();
        for (k, x) in spread_obj(incoming_session) {
            o.insert(k, x);
        }
        o.insert("status".into(), str_val("running"));
        o.insert(
            "activeTurnId".into(),
            prop(ps, "activeTurnId", "previousSession.activeTurnId")?
                .as_ref()
                .clone(),
        );
        let ple = prop(ps, "lastError", "previousSession.lastError")?;
        o.insert(
            "lastError".into(),
            if !is_nullish(ple.as_ref()) {
                ple.as_ref().clone()
            } else {
                prop(incoming_session, "lastError", "incomingSession.lastError")?
                    .as_ref()
                    .clone()
            },
        );
        let pu = prop(ps, "updatedAt", "previousSession.updatedAt")?;
        let iu = prop(incoming_session, "updatedAt", "incomingSession.updatedAt")?;
        o.insert(
            "updatedAt".into(),
            if js_ge(pu.as_ref(), iu.as_ref()) {
                pu.as_ref().clone()
            } else {
                iu.as_ref().clone()
            },
        );
        return Ok(Value::Object(o));
    }
    Ok(incoming_session.clone())
}

fn piid_helper<'a>(ps: &'a Value) -> Result<Cow<'a, Value>, String> {
    prop(
        ps,
        "providerInstanceId",
        "previousSession.providerInstanceId",
    )
}

fn merge_latest_turn(
    incoming_latest_turn: &Value,
    previous_thread: &Value,
    preserve_running_turn: bool,
) -> Result<Value, String> {
    let previous_latest_turn = prop_opt(previous_thread, "latestTurn");
    if !truthy(&previous_latest_turn) {
        return Ok(incoming_latest_turn.clone());
    }
    let plt = &previous_latest_turn;
    if preserve_running_turn {
        if incoming_latest_turn.is_null()
            || strict_eq(
                prop(incoming_latest_turn, "turnId", "incomingLatestTurn.turnId")?.as_ref(),
                prop(plt, "turnId", "previousLatestTurn.turnId")?.as_ref(),
            )
        {
            let mut o = Map::new();
            let base = if is_nullish(incoming_latest_turn) {
                plt
            } else {
                incoming_latest_turn
            };
            for (k, x) in spread_obj(base) {
                o.insert(k, x);
            }
            o.insert(
                "turnId".into(),
                prop(plt, "turnId", "previousLatestTurn.turnId")?
                    .as_ref()
                    .clone(),
            );
            o.insert("state".into(), str_val("running"));
            let ira = prop_opt(incoming_latest_turn, "requestedAt");
            o.insert(
                "requestedAt".into(),
                if !is_nullish(ira.as_ref()) {
                    ira.as_ref().clone()
                } else {
                    prop(plt, "requestedAt", "previousLatestTurn.requestedAt")?
                        .as_ref()
                        .clone()
                },
            );
            let isa = prop_opt(incoming_latest_turn, "startedAt");
            o.insert(
                "startedAt".into(),
                if !is_nullish(isa.as_ref()) {
                    isa.as_ref().clone()
                } else {
                    prop(plt, "startedAt", "previousLatestTurn.startedAt")?
                        .as_ref()
                        .clone()
                },
            );
            o.insert("completedAt".into(), Value::Null);
            let pamid = prop(
                plt,
                "assistantMessageId",
                "previousLatestTurn.assistantMessageId",
            )?;
            let amid = if !is_nullish(pamid.as_ref()) {
                pamid.as_ref().clone()
            } else {
                let iam = prop_opt(incoming_latest_turn, "assistantMessageId");
                if !is_nullish(iam.as_ref()) {
                    iam.as_ref().clone()
                } else {
                    Value::Null
                }
            };
            o.insert("assistantMessageId".into(), amid);
            let spp = {
                let i = prop_opt(incoming_latest_turn, "sourceProposedPlan");
                if is_nullish(i.as_ref()) {
                    prop(
                        plt,
                        "sourceProposedPlan",
                        "previousLatestTurn.sourceProposedPlan",
                    )?
                } else {
                    i
                }
            };
            if truthy(spp.as_ref()) {
                o.insert("sourceProposedPlan".into(), spp.as_ref().clone());
            }
            return Ok(Value::Object(o));
        }
        return Ok(incoming_latest_turn.clone());
    }
    if incoming_latest_turn.is_null()
        || !strict_eq(
            prop(incoming_latest_turn, "turnId", "incomingLatestTurn.turnId")?.as_ref(),
            prop(plt, "turnId", "previousLatestTurn.turnId")?.as_ref(),
        )
    {
        return Ok(incoming_latest_turn.clone());
    }
    let pamid = prop(
        plt,
        "assistantMessageId",
        "previousLatestTurn.assistantMessageId",
    )?;
    if is_undef(pamid.as_ref())
        || strict_eq(
            prop(
                incoming_latest_turn,
                "assistantMessageId",
                "incomingLatestTurn.assistantMessageId",
            )?
            .as_ref(),
            pamid.as_ref(),
        )
    {
        return Ok(incoming_latest_turn.clone());
    }
    let mut o = Map::new();
    for (k, x) in spread_obj(incoming_latest_turn) {
        o.insert(k, x);
    }
    o.insert("assistantMessageId".into(), pamid.as_ref().clone());
    Ok(Value::Object(o))
}

fn clear_settled_streaming<'a>(
    messages: &'a Value,
    settled_turn_id: &'a Value,
) -> Result<Cow<'a, Value>, String> {
    let mut changed = false;
    let next = map_call(
        messages,
        "messages.map",
        "messages.map((message) => {\n    if (!message.streaming || message.turnId !== settledTurnId)\n      return message;\n    changed = true;\n    return { ...message, streaming: false };\n  })",
        &mut |message: Cow<'_, Value>, _| {
            let streaming = prop(message.as_ref(), "streaming", "message.streaming")?;
            let tid = prop(message.as_ref(), "turnId", "message.turnId")?;
            if !truthy(streaming.as_ref()) || !strict_eq(tid.as_ref(), settled_turn_id) {
                return Ok(clone_cow(&message));
            }
            changed = true;
            let mut o = Map::new();
            for (k, x) in spread_obj(message.as_ref()) {
                o.insert(k, x);
            }
            o.insert("streaming".into(), Value::Bool(false));
            Ok(Cow::Owned(Value::Object(o)))
        },
    )?;
    if changed {
        Ok(Cow::Owned(Value::Array(
            next.iter().map(|c| c.as_ref().clone()).collect(),
        )))
    } else {
        Ok(Cow::Borrowed(messages))
    }
}

// ---------------------------------------------------------------------------
// Thread-level normalization
// ---------------------------------------------------------------------------

fn normalize_usage_limit_recovery<'a>(
    incoming: &'a Value,
    previous: &'a Value,
    snapshot_sequence: &'a Value,
) -> Result<(Cow<'a, Value>, Cow<'a, Value>), String> {
    let candidate = if !is_undef(previous)
        && (if is_undef(snapshot_sequence) {
            js_lt(
                prop(incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
                {
                    let u = prop(previous, "updatedAt", "previous.updatedAt")?;
                    if is_nullish(u.as_ref()) {
                        prop(previous, "createdAt", "previous.createdAt")?
                    } else {
                        u
                    }
                }
                .as_ref(),
            )
        } else {
            js_lt(
                snapshot_sequence,
                {
                    let s = prop(
                        previous,
                        "usageLimitRecoverySequence",
                        "previous.usageLimitRecoverySequence",
                    )?;
                    if is_nullish(s.as_ref()) {
                        Cow::Owned(num_val(0.0))
                    } else {
                        s
                    }
                }
                .as_ref(),
            )
        }) {
        prop(
            previous,
            "usageLimitRecovery",
            "previous.usageLimitRecovery",
        )?
    } else {
        prop(
            incoming,
            "usageLimitRecovery",
            "incoming.usageLimitRecovery",
        )?
    };
    let prev_ulr = prop_opt(previous, "usageLimitRecovery");
    let usage_limit_recovery = if truthy(prev_ulr.as_ref())
        && deep_eq(
            prev_ulr.as_ref(),
            &if is_nullish(candidate.as_ref()) {
                Value::Null
            } else {
                candidate.as_ref().clone()
            },
        ) {
        prev_ulr
    } else if is_nullish(candidate.as_ref()) {
        Cow::Owned(Value::Null)
    } else {
        candidate
    };
    let seq = if is_undef(snapshot_sequence) {
        prop_opt(previous, "usageLimitRecoverySequence")
    } else {
        let ps = prop_opt(previous, "usageLimitRecoverySequence");
        Cow::Owned(num_val(js_max(
            to_number(snapshot_sequence),
            if is_nullish(ps.as_ref()) {
                0.0
            } else {
                to_number(ps.as_ref())
            },
        )))
    };
    Ok((usage_limit_recovery, seq))
}

fn normalize_model_selection_inner(value: &Value, previous: &Value) -> Result<Value, String> {
    let model = prop(value, "model", "value.model")?;
    let provider = prop(value, "provider", "value.provider")?;
    let normalized = normalize_model_slug(model.as_ref(), provider.as_ref())?;
    let normalized_model = normalized
        .map(|s| str_val(&s))
        .unwrap_or_else(|| model.as_ref().clone());
    let model2 = prop(value, "model", "value.model")?;
    let next = if strict_eq(&normalized_model, model2.as_ref()) {
        Cow::Borrowed(value)
    } else {
        let mut o = Map::new();
        for (k, x) in spread_obj(value) {
            o.insert(k, x);
        }
        o.insert("model".into(), normalized_model);
        Cow::Owned(Value::Object(o))
    };
    if truthy(previous) && deep_eq(previous, next.as_ref()) {
        Ok(previous.clone())
    } else {
        Ok(next.into_owned())
    }
}

fn normalize_project_scripts<'a>(
    incoming: &'a Value,
    previous: &'a Value,
) -> Result<Value, String> {
    let next_scripts = map_call(
        incoming,
        "incoming.map",
        "incoming.map((script, index) => {\n    const existing = previous?.[index];\n    return existing && deepEqualJson(existing, script) ? existing : script;\n  })",
        &mut |script: Cow<'a, Value>, index| {
            let existing = idx_opt(previous, index as f64);
            if truthy(existing.as_ref()) && deep_eq(existing.as_ref(), script.as_ref()) {
                Ok(existing)
            } else {
                Ok(script)
            }
        },
    )?;
    if arrays_shallow_eq(previous, &next_scripts)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            next_scripts.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn normalize_project_inner<'a>(incoming: &'a Value, previous: &'a Value) -> Result<Value, String> {
    let workspace_root = prop(incoming, "workspaceRoot", "incoming.workspaceRoot")?;
    let workspace_root_key = project_cwd_key(workspace_root.as_ref())?;
    let folder_name = match basename_of_path(workspace_root.as_ref())? {
        Some(v) => v,
        None => prop(incoming, "title", "incoming.title")?.as_ref().clone(),
    };
    let pl = prop_opt(previous, "localName");
    let local_name = if !is_nullish(pl.as_ref()) {
        Some(pl.as_ref().clone())
    } else {
        None
    };
    let pa = prop_opt(previous, "appearance");
    let appearance = if !is_nullish(pa.as_ref()) {
        Some(pa.as_ref().clone())
    } else {
        None
    };
    let incoming_dms = prop(
        incoming,
        "defaultModelSelection",
        "incoming.defaultModelSelection",
    )?;
    let prev_dms = prop_opt(previous, "defaultModelSelection");
    let default_model_selection = if incoming_dms.is_null() {
        Value::Null
    } else {
        normalize_model_selection_inner(incoming_dms.as_ref(), prev_dms.as_ref())?
    };
    let scripts = normalize_project_scripts(
        prop(incoming, "scripts", "incoming.scripts")?.as_ref(),
        prop_opt(previous, "scripts").as_ref(),
    )?;
    let inc_folders = prop(incoming, "additionalFolders", "incoming.additionalFolders")?;
    let inc_folders = if is_nullish(inc_folders.as_ref()) {
        Cow::Owned(Value::Array(Vec::new()))
    } else {
        inc_folders
    };
    let prev_folders = prop_opt(previous, "additionalFolders");
    let additional_folders =
        if truthy(prev_folders.as_ref()) && deep_eq(prev_folders.as_ref(), inc_folders.as_ref()) {
            prev_folders
        } else {
            inc_folders
        };
    // Persisted state is empty in the worker: persistedProjectOrderIndex is
    // undefined and hasKnownLegacyExpansion is false.
    let expanded = {
        let prev_exp = if truthy(previous)
            && project_cwd_key(prop(previous, "cwd", "previous.cwd")?.as_ref())?
                == workspace_root_key
        {
            prop(previous, "expanded", "previous.expanded")?
        } else {
            Cow::Owned(js::undefined())
        };
        if is_nullish(prev_exp.as_ref()) {
            Cow::Owned(Value::Bool(true))
        } else {
            prev_exp
        }
    };
    if truthy(previous)
        && strict_eq(
            prop(previous, "id", "previous.id")?.as_ref(),
            prop(incoming, "id", "incoming.id")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "kind", "previous.kind")?.as_ref(),
            prop(incoming, "kind", "incoming.kind")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "name", "previous.name")?.as_ref(),
            &local_name.clone().unwrap_or_else(|| {
                prop(incoming, "title", "incoming.title")
                    .map(|c| c.into_owned())
                    .unwrap_or(Value::Null)
            }),
        )
        && strict_eq(
            prop(previous, "remoteName", "previous.remoteName")?.as_ref(),
            prop(incoming, "title", "incoming.title")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "folderName", "previous.folderName")?.as_ref(),
            &folder_name,
        )
        && strict_eq(
            prop(previous, "localName", "previous.localName")?.as_ref(),
            &local_name.clone().unwrap_or_else(undefined),
        )
        && strict_eq(
            &if is_nullish(prop(previous, "appearance", "previous.appearance")?.as_ref()) {
                Value::Null
            } else {
                prop(previous, "appearance", "previous.appearance")?
                    .as_ref()
                    .clone()
            },
            &appearance.clone().unwrap_or(Value::Null),
        )
        && strict_eq(
            prop(previous, "cwd", "previous.cwd")?.as_ref(),
            workspace_root.as_ref(),
        )
        && strict_eq(
            prop(
                previous,
                "defaultModelSelection",
                "previous.defaultModelSelection",
            )?
            .as_ref(),
            &default_model_selection,
        )
        && strict_eq(
            prop(previous, "expanded", "previous.expanded")?.as_ref(),
            expanded.as_ref(),
        )
        && strict_eq(
            &if is_nullish(prop(previous, "isPinned", "previous.isPinned")?.as_ref()) {
                Value::Bool(false)
            } else {
                prop(previous, "isPinned", "previous.isPinned")?
                    .as_ref()
                    .clone()
            },
            &if is_nullish(prop(incoming, "isPinned", "incoming.isPinned")?.as_ref()) {
                Value::Bool(false)
            } else {
                prop(incoming, "isPinned", "incoming.isPinned")?
                    .as_ref()
                    .clone()
            },
        )
        && strict_eq(
            &if is_nullish(prop(previous, "spaceId", "previous.spaceId")?.as_ref()) {
                Value::Null
            } else {
                prop(previous, "spaceId", "previous.spaceId")?
                    .as_ref()
                    .clone()
            },
            &if is_nullish(prop(incoming, "spaceId", "incoming.spaceId")?.as_ref()) {
                Value::Null
            } else {
                prop(incoming, "spaceId", "incoming.spaceId")?
                    .as_ref()
                    .clone()
            },
        )
        && strict_eq(
            prop(previous, "createdAt", "previous.createdAt")?.as_ref(),
            prop(incoming, "createdAt", "incoming.createdAt")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "updatedAt", "previous.updatedAt")?.as_ref(),
            prop(incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "scripts", "previous.scripts")?.as_ref(),
            &scripts,
        )
        && strict_eq(
            prop(previous, "additionalFolders", "previous.additionalFolders")?.as_ref(),
            additional_folders.as_ref(),
        )
    {
        return Ok(previous.clone());
    }
    let mut o = Map::new();
    o.insert(
        "id".into(),
        prop(incoming, "id", "incoming.id")?.as_ref().clone(),
    );
    let k = prop(incoming, "kind", "incoming.kind")?;
    o.insert(
        "kind".into(),
        if is_nullish(k.as_ref()) {
            str_val("project")
        } else {
            k.as_ref().clone()
        },
    );
    o.insert(
        "name".into(),
        local_name.clone().unwrap_or_else(|| {
            prop(incoming, "title", "incoming.title")
                .map(|c| c.into_owned())
                .unwrap_or(Value::Null)
        }),
    );
    o.insert(
        "remoteName".into(),
        prop(incoming, "title", "incoming.title")?.as_ref().clone(),
    );
    o.insert("folderName".into(), folder_name);
    o.insert("localName".into(), local_name.unwrap_or(Value::Null));
    o.insert("appearance".into(), appearance.unwrap_or(Value::Null));
    o.insert("cwd".into(), workspace_root.as_ref().clone());
    o.insert(
        "defaultModelSelection".into(),
        default_model_selection.clone(),
    );
    o.insert("expanded".into(), expanded.as_ref().clone());
    let ip = prop(incoming, "isPinned", "incoming.isPinned")?;
    o.insert(
        "isPinned".into(),
        if is_nullish(ip.as_ref()) {
            Value::Bool(false)
        } else {
            ip.as_ref().clone()
        },
    );
    let sid = prop(incoming, "spaceId", "incoming.spaceId")?;
    o.insert(
        "spaceId".into(),
        if is_nullish(sid.as_ref()) {
            Value::Null
        } else {
            sid.as_ref().clone()
        },
    );
    o.insert(
        "createdAt".into(),
        prop(incoming, "createdAt", "incoming.createdAt")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "updatedAt".into(),
        prop(incoming, "updatedAt", "incoming.updatedAt")?
            .as_ref()
            .clone(),
    );
    o.insert("scripts".into(), scripts.clone());
    o.insert(
        "additionalFolders".into(),
        additional_folders.as_ref().clone(),
    );
    Ok(Value::Object(o))
}

// ---------------------------------------------------------------------------
// Session / turn / thread normalization
// ---------------------------------------------------------------------------

fn is_nonfatal_thread_error(message: &Value) -> Result<bool, String> {
    if !truthy(message) {
        return Ok(false);
    }
    let trimmed = str_method(message, "trim", "message.trim", "message.trim()", |u| {
        trim_units(u)
    })?;
    let tv = utf16_val(&trimmed);
    let normalized = str_method(
        &tv,
        "toLowerCase",
        "message.trim().toLowerCase",
        "message.trim().toLowerCase()",
        lower_units,
    )?;
    let nv = utf16_val(&normalized);
    let callee = prop(&nv, "includes", "normalized.includes")?;
    if !(is_undef(callee.as_ref()) && callable_at(&nv, "includes")) {
        return Err(not_a_function(
            "normalized.includes",
            "normalized.includes(\"write_stdin failed: stdin is closed for this session\")",
            callee.as_ref(),
        ));
    }
    Ok(u16_contains(
        &normalized,
        &"write_stdin failed: stdin is closed for this session"
            .encode_utf16()
            .collect::<Vec<u16>>(),
    ))
}

fn normalize_thread_error_message_inner(message: &Value) -> Result<Value, String> {
    if truthy(message) && !is_nonfatal_thread_error(message)? {
        Ok(message.clone())
    } else {
        Ok(Value::Null)
    }
}

fn to_legacy_provider_val<'a>(provider_name: &'a Value) -> Cow<'a, Value> {
    for p in [
        "codex",
        "claudeAgent",
        "cursor",
        "antigravity",
        "grok",
        "droid",
        "opencode",
        "pi",
        "devin",
        "omp",
    ] {
        if strict_eq(provider_name, &str_val(p)) {
            return Cow::Borrowed(provider_name);
        }
    }
    if strict_eq(provider_name, &str_val("kilo")) {
        return Cow::Owned(str_val("opencode"));
    }
    Cow::Owned(str_val("codex"))
}

fn to_legacy_session_status<'a>(status: &'a Value) -> Cow<'a, Value> {
    for (pat, out) in [
        ("starting", "connecting"),
        ("running", "running"),
        ("error", "error"),
        ("ready", "ready"),
        ("interrupted", "ready"),
        ("idle", "closed"),
        ("stopped", "closed"),
    ] {
        if strict_eq(status, &str_val(pat)) {
            return Cow::Owned(str_val(out));
        }
    }
    Cow::Owned(js::undefined())
}

fn normalize_thread_session_inner(incoming: &Value, previous: &Value) -> Result<Value, String> {
    if !truthy(incoming) {
        return Ok(Value::Null);
    }
    let last_error_in = prop(incoming, "lastError", "incoming.lastError")?;
    let next_last_error =
        if truthy(last_error_in.as_ref()) && !is_nonfatal_thread_error(last_error_in.as_ref())? {
            clone_cow(&last_error_in)
        } else {
            Cow::Owned(js::undefined())
        };
    let mut o = Map::new();
    o.insert(
        "provider".into(),
        to_legacy_provider_val(prop(incoming, "providerName", "incoming.providerName")?.as_ref())
            .as_ref()
            .clone(),
    );
    let piid = prop(
        incoming,
        "providerInstanceId",
        "incoming.providerInstanceId",
    )?;
    if !is_undef(piid.as_ref()) {
        o.insert("providerInstanceId".into(), piid.as_ref().clone());
    }
    o.insert(
        "status".into(),
        to_legacy_session_status(prop(incoming, "status", "incoming.status")?.as_ref())
            .as_ref()
            .clone(),
    );
    o.insert(
        "orchestrationStatus".into(),
        prop(incoming, "status", "incoming.status")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "runtimeMode".into(),
        prop(incoming, "runtimeMode", "incoming.runtimeMode")?
            .as_ref()
            .clone(),
    );
    let at = prop(incoming, "activeTurnId", "incoming.activeTurnId")?;
    o.insert(
        "activeTurnId".into(),
        if is_nullish(at.as_ref()) {
            undefined()
        } else {
            at.as_ref().clone()
        },
    );
    let upd = prop(incoming, "updatedAt", "incoming.updatedAt")?;
    o.insert("createdAt".into(), upd.as_ref().clone());
    o.insert("updatedAt".into(), upd.as_ref().clone());
    if truthy(next_last_error.as_ref()) {
        o.insert("lastError".into(), next_last_error.as_ref().clone());
    }
    let next_session = Value::Object(o);
    if truthy(previous) {
        let fields: Vec<(&str, &str)> = vec![
            ("provider", "provider"),
            ("providerInstanceId", "providerInstanceId"),
            ("status", "status"),
            ("orchestrationStatus", "orchestrationStatus"),
            ("runtimeMode", "runtimeMode"),
            ("activeTurnId", "activeTurnId"),
            ("createdAt", "createdAt"),
            ("updatedAt", "updatedAt"),
            ("lastError", "lastError"),
        ];
        let mut all = true;
        for (f, _) in &fields {
            let pe = format!("previous.{f}");
            let ne = format!("nextSession.{f}");
            if !strict_eq(
                prop(previous, f, &pe)?.as_ref(),
                prop(&next_session, f, &ne)?.as_ref(),
            ) {
                all = false;
                break;
            }
        }
        if all {
            return Ok(previous.clone());
        }
    }
    Ok(next_session)
}

fn normalize_latest_turn<'a>(
    incoming: &'a Value,
    previous: &'a Value,
) -> Result<Cow<'a, Value>, String> {
    if !truthy(incoming) {
        return Ok(Cow::Owned(Value::Null));
    }
    let next_spp = {
        let isp = prop(
            incoming,
            "sourceProposedPlan",
            "incoming.sourceProposedPlan",
        )?;
        if truthy(isp.as_ref()) {
            let psp = prop_opt(previous, "sourceProposedPlan");
            let reuse = truthy(psp.as_ref())
                && strict_eq(
                    prop(
                        psp.as_ref(),
                        "threadId",
                        "previous.sourceProposedPlan.threadId",
                    )?
                    .as_ref(),
                    prop(
                        isp.as_ref(),
                        "threadId",
                        "incoming.sourceProposedPlan.threadId",
                    )?
                    .as_ref(),
                )
                && strict_eq(
                    prop(psp.as_ref(), "planId", "previous.sourceProposedPlan.planId")?.as_ref(),
                    prop(isp.as_ref(), "planId", "incoming.sourceProposedPlan.planId")?.as_ref(),
                );
            if reuse { psp } else { isp }
        } else {
            Cow::Borrowed(undef())
        }
    };
    if truthy(previous)
        && strict_eq(
            prop(previous, "turnId", "previous.turnId")?.as_ref(),
            prop(incoming, "turnId", "incoming.turnId")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "state", "previous.state")?.as_ref(),
            prop(incoming, "state", "incoming.state")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "requestedAt", "previous.requestedAt")?.as_ref(),
            prop(incoming, "requestedAt", "incoming.requestedAt")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "startedAt", "previous.startedAt")?.as_ref(),
            prop(incoming, "startedAt", "incoming.startedAt")?.as_ref(),
        )
        && strict_eq(
            prop(previous, "completedAt", "previous.completedAt")?.as_ref(),
            prop(incoming, "completedAt", "incoming.completedAt")?.as_ref(),
        )
        && strict_eq(
            prop(
                previous,
                "assistantMessageId",
                "previous.assistantMessageId",
            )?
            .as_ref(),
            prop(
                incoming,
                "assistantMessageId",
                "incoming.assistantMessageId",
            )?
            .as_ref(),
        )
        && strict_eq(
            prop(
                previous,
                "sourceProposedPlan",
                "previous.sourceProposedPlan",
            )?
            .as_ref(),
            next_spp.as_ref(),
        )
    {
        return Ok(Cow::Borrowed(previous));
    }
    let mut o = Map::new();
    o.insert(
        "turnId".into(),
        prop(incoming, "turnId", "incoming.turnId")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "state".into(),
        prop(incoming, "state", "incoming.state")?.as_ref().clone(),
    );
    o.insert(
        "requestedAt".into(),
        prop(incoming, "requestedAt", "incoming.requestedAt")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "startedAt".into(),
        prop(incoming, "startedAt", "incoming.startedAt")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "completedAt".into(),
        prop(incoming, "completedAt", "incoming.completedAt")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "assistantMessageId".into(),
        prop(
            incoming,
            "assistantMessageId",
            "incoming.assistantMessageId",
        )?
        .as_ref()
        .clone(),
    );
    if truthy(next_spp.as_ref()) {
        o.insert("sourceProposedPlan".into(), next_spp.as_ref().clone());
    }
    Ok(Cow::Owned(Value::Object(o)))
}

fn normalize_proposed_plans_inner(incoming: &Value, previous: &Value) -> Result<Value, String> {
    let pairs = opt_map_call(
        previous,
        "previous?.map",
        "previous?.map((plan) => [plan.id, plan])",
        &mut |plan: Cow<'_, Value>, _| {
            let id = prop(plan.as_ref(), "id", "plan.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                plan.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_id = map_of_pairs(pairs.unwrap_or_default());
    let next_plans = map_call(
        incoming,
        "incoming.map",
        "incoming.map((plan) => {\n    const existing = previousById.get(plan.id);\n    if (existing && existing.turnId === plan.turnId && existing.planMarkdown === plan.planMarkdown && existing.implementedAt === plan.implementedAt && existing.implementationThreadId === plan.implementationThreadId && existing.createdAt === plan.createdAt && existing.updatedAt === plan.updatedAt)\n      return existing;\n    return {\n      id: plan.id,\n      turnId: plan.turnId,\n      planMarkdown: plan.planMarkdown,\n      implementedAt: plan.implementedAt,\n      implementationThreadId: plan.implementationThreadId,\n      createdAt: plan.createdAt,\n      updatedAt: plan.updatedAt\n    };\n  })",
        &mut |plan: Cow<'_, Value>, _| -> Result<Cow<'_, Value>, String> {
            let p = plan.as_ref();
            let id = prop(p, "id", "plan.id")?;
            if let Some(existing) = previous_by_id.get(id.as_ref()) {
                let mut same = true;
                for f in [
                    "turnId",
                    "planMarkdown",
                    "implementedAt",
                    "implementationThreadId",
                    "createdAt",
                    "updatedAt",
                ] {
                    if !strict_eq(
                        prop(existing, f, &format!("existing.{f}"))?.as_ref(),
                        prop(p, f, &format!("plan.{f}"))?.as_ref(),
                    ) {
                        same = false;
                        break;
                    }
                }
                if same {
                    return Ok(Cow::Owned(existing.clone()));
                }
            }
            let mut o = Map::new();
            for f in [
                "id",
                "turnId",
                "planMarkdown",
                "implementedAt",
                "implementationThreadId",
                "createdAt",
                "updatedAt",
            ] {
                o.insert(f.into(), prop(p, f, &format!("plan.{f}"))?.as_ref().clone());
            }
            Ok(Cow::Owned(Value::Object(o)))
        },
    )?;
    if arrays_shallow_eq(previous, &next_plans)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            next_plans.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn merge_turn_diff_files<'a>(files: &'a Value) -> Result<Vec<Cow<'a, Value>>, String> {
    let mut files_by_path = JsMap::new();
    for file in iter(files, "file", "files")? {
        let f = file.as_ref();
        let path = prop(f, "path", "file.path")?;
        let existing = files_by_path.get(path.as_ref());
        match truthy(existing.unwrap_or(undef())) {
            false => {
                files_by_path.set(path.as_ref().clone(), clone_cow(&file));
            }
            true => {
                let existing = existing.unwrap_or(undef());
                let additions = js_add(
                    &{
                        let a = prop(existing, "additions", "existing.additions")?;
                        if is_nullish(a.as_ref()) {
                            num_val(0.0)
                        } else {
                            a.as_ref().clone()
                        }
                    },
                    &{
                        let a = prop(f, "additions", "file.additions")?;
                        if is_nullish(a.as_ref()) {
                            num_val(0.0)
                        } else {
                            a.as_ref().clone()
                        }
                    },
                );
                let deletions = js_add(
                    &{
                        let a = prop(existing, "deletions", "existing.deletions")?;
                        if is_nullish(a.as_ref()) {
                            num_val(0.0)
                        } else {
                            a.as_ref().clone()
                        }
                    },
                    &{
                        let a = prop(f, "deletions", "file.deletions")?;
                        if is_nullish(a.as_ref()) {
                            num_val(0.0)
                        } else {
                            a.as_ref().clone()
                        }
                    },
                );
                let mut o = Map::new();
                o.insert("path".into(), path.as_ref().clone());
                o.insert(
                    "kind".into(),
                    prop(existing, "kind", "existing.kind")?.as_ref().clone(),
                );
                o.insert("additions".into(), additions);
                o.insert("deletions".into(), deletions);
                files_by_path.set(path.as_ref().clone(), Cow::Owned(Value::Object(o)));
            }
        }
    }
    Ok(files_by_path.values().map(clone_cow).collect())
}

fn normalize_turn_diff_files_inner<'a>(
    incoming: &'a Value,
    previous: &'a Value,
) -> Result<Value, String> {
    let merged = merge_turn_diff_files(incoming)?;
    let mut next_files: Vec<Cow<'a, Value>> = Vec::with_capacity(merged.len());
    for (index, file) in merged.iter().enumerate() {
        let existing = idx_opt(previous, index as f64);
        let same = truthy(existing.as_ref())
            && strict_eq(
                prop(existing.as_ref(), "path", "existing.path")?.as_ref(),
                prop(file.as_ref(), "path", "file.path")?.as_ref(),
            )
            && strict_eq(
                prop(existing.as_ref(), "kind", "existing.kind")?.as_ref(),
                prop(file.as_ref(), "kind", "file.kind")?.as_ref(),
            )
            && strict_eq(
                prop(existing.as_ref(), "additions", "existing.additions")?.as_ref(),
                prop(file.as_ref(), "additions", "file.additions")?.as_ref(),
            )
            && strict_eq(
                prop(existing.as_ref(), "deletions", "existing.deletions")?.as_ref(),
                prop(file.as_ref(), "deletions", "file.deletions")?.as_ref(),
            );
        if same {
            next_files.push(existing);
        } else {
            next_files.push(clone_cow(file));
        }
    }
    if arrays_shallow_eq(previous, &next_files)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            next_files.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn normalize_turn_diff_summaries<'a>(
    incoming: &'a Value,
    previous: &Value,
) -> Result<Value, String> {
    let pairs = opt_map_call(
        previous,
        "previous?.map",
        "previous?.map((summary) => [summary.turnId, summary])",
        &mut |summary: Cow<'_, Value>, _| {
            let tid = prop(summary.as_ref(), "turnId", "summary.turnId")?;
            Ok(Cow::Owned(Value::Array(vec![
                tid.as_ref().clone(),
                summary.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_turn = map_of_pairs(pairs.unwrap_or_default());
    let mut next_summaries: Vec<Cow<'a, Value>> = Vec::new();
    for checkpoint in iter(incoming, "checkpoint", "incoming")? {
        let cp = checkpoint.as_ref();
        let turn_id = prop(cp, "turnId", "checkpoint.turnId")?;
        let existing = previous_by_turn.get(turn_id.as_ref());
        let existing_files = existing.map(|e| prop_opt(e, "files").into_owned());
        let files = normalize_turn_diff_files_inner(
            prop(cp, "files", "checkpoint.files")?.as_ref(),
            existing_files.as_ref().unwrap_or(undef()),
        )?;
        let same = if let Some(e) = existing {
            let estarted = prop_opt(e, "startedAt");
            let cstarted = prop(cp, "startedAt", "checkpoint.startedAt")?;
            let resolved_started = if is_nullish(cstarted.as_ref()) {
                estarted
            } else {
                cstarted
            };
            let amid_cp = prop(cp, "assistantMessageId", "checkpoint.assistantMessageId")?;
            strict_eq(
                prop(e, "startedAt", "existing.startedAt")?.as_ref(),
                resolved_started.as_ref(),
            ) && strict_eq(
                prop(e, "completedAt", "existing.completedAt")?.as_ref(),
                prop(cp, "completedAt", "checkpoint.completedAt")?.as_ref(),
            ) && strict_eq(
                prop(e, "status", "existing.status")?.as_ref(),
                prop(cp, "status", "checkpoint.status")?.as_ref(),
            ) && strict_eq(
                prop(e, "assistantMessageId", "existing.assistantMessageId")?.as_ref(),
                &if is_nullish(amid_cp.as_ref()) {
                    undefined()
                } else {
                    amid_cp.as_ref().clone()
                },
            ) && strict_eq(
                prop(e, "checkpointTurnCount", "existing.checkpointTurnCount")?.as_ref(),
                prop(cp, "checkpointTurnCount", "checkpoint.checkpointTurnCount")?.as_ref(),
            ) && strict_eq(
                prop(e, "checkpointRef", "existing.checkpointRef")?.as_ref(),
                prop(cp, "checkpointRef", "checkpoint.checkpointRef")?.as_ref(),
            ) && strict_eq(prop(e, "files", "existing.files")?.as_ref(), &files)
        } else {
            false
        };
        if same {
            next_summaries.push(Cow::Owned(existing.unwrap_or(undef()).clone()));
            continue;
        }
        let cstarted = prop(cp, "startedAt", "checkpoint.startedAt")?;
        let started_at = if is_nullish(cstarted.as_ref()) {
            existing.map(|e| prop_opt(e, "startedAt").into_owned())
        } else {
            Some(cstarted.into_owned())
        };
        let mut o = Map::new();
        o.insert("turnId".into(), turn_id.as_ref().clone());
        if let Some(sa) = &started_at
            && truthy(sa)
        {
            o.insert("startedAt".into(), sa.clone());
        }
        o.insert(
            "completedAt".into(),
            prop(cp, "completedAt", "checkpoint.completedAt")?
                .as_ref()
                .clone(),
        );
        o.insert(
            "status".into(),
            prop(cp, "status", "checkpoint.status")?.as_ref().clone(),
        );
        let amid = prop(cp, "assistantMessageId", "checkpoint.assistantMessageId")?;
        o.insert(
            "assistantMessageId".into(),
            if is_nullish(amid.as_ref()) {
                undefined()
            } else {
                amid.as_ref().clone()
            },
        );
        o.insert(
            "checkpointTurnCount".into(),
            prop(cp, "checkpointTurnCount", "checkpoint.checkpointTurnCount")?
                .as_ref()
                .clone(),
        );
        o.insert(
            "checkpointRef".into(),
            prop(cp, "checkpointRef", "checkpoint.checkpointRef")?
                .as_ref()
                .clone(),
        );
        o.insert("files".into(), files.clone());
        next_summaries.push(Cow::Owned(Value::Object(o)));
    }
    if arrays_shallow_eq(previous, &next_summaries)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            next_summaries.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn normalize_activities_inner(incoming: &Value, previous: &Value) -> Result<Value, String> {
    let previous_activities = if truthy(previous) {
        Some(dedupe_activities_vec(previous)?)
    } else {
        None
    };
    let incoming_activities = dedupe_activities_vec(incoming)?;
    let pairs = match &previous_activities {
        Some(pa) => opt_map_call(
            pa,
            "previousActivities?.map",
            "previousActivities?.map((activity) => [activity.id, activity])",
            &mut |activity: Cow<'_, Value>, _| {
                let id = prop(activity.as_ref(), "id", "activity.id")?;
                Ok(Cow::Owned(Value::Array(vec![
                    id.as_ref().clone(),
                    activity.as_ref().clone(),
                ])))
            },
        )?,
        None => None,
    };
    let previous_by_id = map_of_pairs(pairs.unwrap_or_default());
    let next_activities = map_call(
        &incoming_activities,
        "incomingActivities.map",
        "incomingActivities.map((activity) => {\n    const existing = previousById.get(activity.id);\n    if (existing) {\n      const preferred = preferRicherActivity(existing, activity);\n      if (preferred === existing || activitiesEqual(existing, preferred))\n        return existing;\n      return preferred;\n    }\n    return activity;\n  })",
        &mut |activity: Cow<'_, Value>, _| -> Result<Cow<'_, Value>, String> {
            let id = prop(activity.as_ref(), "id", "activity.id")?;
            match previous_by_id.get(id.as_ref()) {
                Some(existing) => {
                    let preferred =
                        prefer_richer(Cow::Owned(existing.clone()), clone_cow(&activity))?;
                    if strict_eq(&preferred, existing) || activities_equal(existing, &preferred)? {
                        Ok(Cow::Owned(existing.clone()))
                    } else {
                        Ok(Cow::Owned(preferred))
                    }
                }
                None => Ok(clone_cow(&activity)),
            }
        },
    )?;
    let capped = cap_activities_cows(&next_activities)?;
    if arrays_shallow_eq(previous, &capped)? {
        Ok(previous.clone())
    } else {
        Ok(Value::Array(
            capped.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

fn cap_activities_cows<'a>(items: &[Cow<'a, Value>]) -> Result<Vec<Cow<'a, Value>>, String> {
    if items.len() <= 2000 {
        return Ok(items.iter().map(clone_cow).collect());
    }
    let arr = Value::Array(items.iter().map(|c| c.as_ref().clone()).collect());
    let capped = cap_thread_activities_vec(&arr)?;
    Ok(capped
        .iter()
        .map(|c| Cow::Owned(c.as_ref().clone()))
        .collect())
}

// ---------------------------------------------------------------------------
// Thread summary derivation (threadSummary.ts + session-logic.ts)
// ---------------------------------------------------------------------------

fn to_payload_record(payload: &Value) -> Option<&Value> {
    as_record(payload)
}

fn lifecycle_generation_from_payload(payload: Option<&Value>) -> Option<Value> {
    let lgen = prop_opt(payload?, "lifecycleGeneration");
    if kind(lgen.as_ref()) == K::Str && !units(lgen.as_ref()).is_empty() {
        Some(lgen.into_owned())
    } else {
        None
    }
}

fn delete_open_request(
    list: &mut Vec<(String, Value)>,
    request_id: &Value,
    generation: Option<&Value>,
) {
    if let Some(g) = generation {
        let key = pending_request_instance_key(request_id, Some(g));
        list.retain(|(k, _)| k != &key);
        return;
    }
    list.retain(|(_, rid)| !strict_eq(rid, request_id));
}

fn replace_open_request(
    list: &mut Vec<(String, Value)>,
    request_id: &Value,
    generation: Option<&Value>,
) {
    delete_open_request(list, request_id, None);
    let key = pending_request_instance_key(request_id, generation);
    if let Some(slot) = list.iter_mut().find(|(k, _)| k == &key) {
        slot.1 = request_id.clone();
    } else {
        list.push((key, request_id.clone()));
    }
}

fn has_structured_user_input_questions(payload: Option<&Value>) -> Result<bool, String> {
    let Some(p) = payload else {
        return Ok(false);
    };
    let questions = prop_opt(p, "questions");
    let Value::Array(qs) = questions.as_ref() else {
        return Ok(false);
    };
    for entry in qs {
        if !truthy(entry) || typeof_js(entry) != "object" {
            continue;
        }
        let options_v = prop(entry, "options", "question.options")?;
        let options = if let Value::Array(o) = options_v.as_ref() {
            Some(o)
        } else {
            None
        };
        let id_ok = kind(prop(entry, "id", "question.id")?.as_ref()) == K::Str;
        let header_ok = kind(prop(entry, "header", "question.header")?.as_ref()) == K::Str;
        let question_ok = kind(prop(entry, "question", "question.question")?.as_ref()) == K::Str;
        if !(id_ok && header_ok && question_ok && options.is_some()) {
            continue;
        }
        for option in options.unwrap_or(&Vec::new()) {
            if !truthy(option) || typeof_js(option) != "object" {
                continue;
            }
            if kind(prop(option, "label", "optionRecord.label")?.as_ref()) == K::Str
                && kind(prop(option, "description", "optionRecord.description")?.as_ref()) == K::Str
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn derive_pending_request_ids(activities: &Value) -> Result<(Vec<Value>, Vec<Value>), String> {
    let mut approvals = Vec::new();
    let mut user_inputs = Vec::new();
    for activity in ordered_activities(activities, OrderFlavor::Summary)? {
        let payload_v = prop(activity.as_ref(), "payload", "activity.payload")?;
        let payload = to_payload_record(payload_v.as_ref());
        let rid = payload.map(|p| prop_opt(p, "requestId").into_owned());
        let request_id = rid.filter(|r| kind(r) == K::Str);
        let detail = payload
            .map(|p| prop_opt(p, "detail").into_owned())
            .filter(|d| kind(d) == K::Str);
        let lifecycle = lifecycle_generation_from_payload(payload);
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        if strict_eq(kind_v.as_ref(), &str_val("approval.requested")) && request_id.is_some() {
            let request_kind = {
                let rk = payload.map(|p| prop_opt(p, "requestKind").into_owned());
                match rk {
                    Some(r)
                        if ["command", "file-read", "file-change", "permissions", "tool"]
                            .iter()
                            .any(|k| strict_eq(&r, &str_val(k))) =>
                    {
                        Some(())
                    }
                    _ => payload
                        .and_then(|p| {
                            let rt = prop_opt(p, "requestType");
                            approval_request_kind_from_request_type(rt.as_ref())
                        })
                        .map(|_| ()),
                }
            };
            if request_kind.is_some() {
                replace_open_request(
                    &mut approvals,
                    request_id.as_ref().unwrap_or(&Value::Null),
                    lifecycle.as_ref(),
                );
            }
            continue;
        }
        if strict_eq(kind_v.as_ref(), &str_val("approval.resolved")) && request_id.is_some() {
            delete_open_request(
                &mut approvals,
                request_id.as_ref().unwrap_or(&Value::Null),
                lifecycle.as_ref(),
            );
            continue;
        }
        if strict_eq(
            kind_v.as_ref(),
            &str_val("provider.approval.respond.failed"),
        ) && request_id.is_some()
            && is_stale_pending_request_failure_detail(detail.as_ref())?
        {
            delete_open_request(
                &mut approvals,
                request_id.as_ref().unwrap_or(&Value::Null),
                lifecycle.as_ref(),
            );
            continue;
        }
        if strict_eq(kind_v.as_ref(), &str_val("user-input.requested")) && request_id.is_some() {
            if has_structured_user_input_questions(payload)? {
                replace_open_request(
                    &mut user_inputs,
                    request_id.as_ref().unwrap_or(&Value::Null),
                    lifecycle.as_ref(),
                );
            }
            continue;
        }
        if strict_eq(kind_v.as_ref(), &str_val("user-input.resolved")) && request_id.is_some() {
            delete_open_request(
                &mut user_inputs,
                request_id.as_ref().unwrap_or(&Value::Null),
                lifecycle.as_ref(),
            );
            continue;
        }
        if strict_eq(
            kind_v.as_ref(),
            &str_val("provider.user-input.respond.failed"),
        ) && request_id.is_some()
            && is_stale_pending_request_failure_detail(detail.as_ref())?
        {
            delete_open_request(
                &mut user_inputs,
                request_id.as_ref().unwrap_or(&Value::Null),
                lifecycle.as_ref(),
            );
        }
    }
    Ok((
        approvals.into_iter().map(|(_, v)| v).collect(),
        user_inputs.into_iter().map(|(_, v)| v).collect(),
    ))
}

fn max_iso<'a>(left: &'a Value, right: &'a Value) -> Cow<'a, Value> {
    if left.is_null() {
        return Cow::Borrowed(right);
    }
    if js_gt(left, right) {
        Cow::Borrowed(left)
    } else {
        Cow::Borrowed(right)
    }
}

fn resolve_latest_proposed_plan<'a>(
    proposed_plans: &'a Value,
    latest_turn: &'a Value,
) -> Result<Cow<'a, Value>, String> {
    let lt_tid = prop_opt(latest_turn, "turnId");
    if truthy(lt_tid.as_ref()) {
        let spread = spread_iter(proposed_plans)?;
        let filtered: Vec<Cow<'a, Value>> = {
            let mut out = Vec::new();
            for plan in spread {
                if strict_eq(
                    prop(plan.as_ref(), "turnId", "plan.turnId")?.as_ref(),
                    lt_tid.as_ref(),
                ) {
                    out.push(plan);
                }
            }
            out
        };
        let mut sorted = filtered;
        js_sort_res(
            &mut sorted,
            &mut |l: &Cow<'a, Value>, r: &Cow<'a, Value>| {
                let lu = prop(l.as_ref(), "updatedAt", "left.updatedAt")?;
                let ru = prop(r.as_ref(), "updatedAt", "right.updatedAt")?;
                let c = locale_call(
                    lu.as_ref(),
                    "left.updatedAt.localeCompare",
                    "left.updatedAt.localeCompare(right.updatedAt)",
                    ru.as_ref(),
                )?;
                if c != 0.0 {
                    return Ok(c);
                }
                let li = prop(l.as_ref(), "id", "left.id")?;
                let ri = prop(r.as_ref(), "id", "right.id")?;
                locale_call(
                    li.as_ref(),
                    "left.id.localeCompare",
                    "left.id.localeCompare(right.id)",
                    ri.as_ref(),
                )
            },
        )?;
        if let Some(plan) = sorted.last() {
            return Ok(clone_cow(plan));
        }
    }
    let spread = spread_iter(proposed_plans)?;
    let mut sorted = spread;
    js_sort_res(
        &mut sorted,
        &mut |l: &Cow<'a, Value>, r: &Cow<'a, Value>| {
            let lu = prop(l.as_ref(), "updatedAt", "left.updatedAt")?;
            let ru = prop(r.as_ref(), "updatedAt", "right.updatedAt")?;
            let c = locale_call(
                lu.as_ref(),
                "left.updatedAt.localeCompare",
                "left.updatedAt.localeCompare(right.updatedAt)",
                ru.as_ref(),
            )?;
            if c != 0.0 {
                return Ok(c);
            }
            let li = prop(l.as_ref(), "id", "left.id")?;
            let ri = prop(r.as_ref(), "id", "right.id")?;
            locale_call(
                li.as_ref(),
                "left.id.localeCompare",
                "left.id.localeCompare(right.id)",
                ri.as_ref(),
            )
        },
    )?;
    Ok(sorted
        .last()
        .map(clone_cow)
        .unwrap_or(Cow::Owned(Value::Null)))
}

fn derive_thread_summary_metadata<'a>(
    messages: &'a Value,
    activities: &'a Value,
    proposed_plans: &'a Value,
    latest_turn: &'a Value,
) -> Result<Value, String> {
    let mut latest_user_at = Value::Null;
    let mut latest_human_at = Value::Null;
    for message in iter(messages, "message", "input.messages")? {
        let m = message.as_ref();
        if strict_eq(prop(m, "role", "message.role")?.as_ref(), &str_val("user")) {
            let created = prop(m, "createdAt", "message.createdAt")?;
            latest_user_at = max_iso(&latest_user_at, created.as_ref()).into_owned();
            // resolveHumanMessageAt
            let dispatch = prop(m, "dispatchOrigin", "message.dispatchOrigin")?;
            let is_human = strict_eq(prop(m, "role", "message.role")?.as_ref(), &str_val("user"))
                && (is_nullish(dispatch.as_ref())
                    || strict_eq(dispatch.as_ref(), &str_val("user")));
            if is_human {
                let upd = prop(m, "updatedAt", "message.updatedAt")?;
                let right = if is_nullish(upd.as_ref()) {
                    created.as_ref().clone()
                } else {
                    upd.as_ref().clone()
                };
                let human_at = max_iso(created.as_ref(), &right).into_owned();
                latest_human_at = max_iso(&latest_human_at, &human_at).into_owned();
            }
        }
    }
    let (approvals, user_inputs) = derive_pending_request_ids(activities)?;
    let latest_plan = resolve_latest_proposed_plan(proposed_plans, latest_turn)?;
    let implemented = prop_opt(latest_plan.as_ref(), "implementedAt");
    let mut o = Map::new();
    o.insert("latestUserMessageAt".into(), latest_user_at);
    o.insert("latestHumanMessageAt".into(), latest_human_at);
    o.insert(
        "hasPendingApprovals".into(),
        Value::Bool(!approvals.is_empty()),
    );
    o.insert(
        "hasPendingUserInput".into(),
        Value::Bool(!user_inputs.is_empty()),
    );
    o.insert(
        "hasActionableProposedPlan".into(),
        Value::Bool(implemented.is_null()),
    );
    Ok(Value::Object(o))
}

fn task_payload_string(payload: Option<&Value>, key: &str) -> Option<Value> {
    let v = prop_opt(payload?, key);
    if kind(v.as_ref()) == K::Str && !units(v.as_ref()).is_empty() {
        Some(v.into_owned())
    } else {
        None
    }
}

fn fold_active_tasks<'a>(
    activities: &'a Value,
    latest_turn_id: &'a Value,
) -> Result<JsMap<'a>, String> {
    let ordered = ordered_activities(activities, OrderFlavor::Worklog)?;
    let mut active_tasks = JsMap::new();
    for activity in &ordered {
        let a = activity.as_ref();
        let turn_id = prop(a, "turnId", "activity.turnId")?;
        let kind_v = prop(a, "kind", "activity.kind")?;
        if truthy(latest_turn_id)
            && truthy(turn_id.as_ref())
            && !strict_eq(turn_id.as_ref(), latest_turn_id)
            && !strict_eq(kind_v.as_ref(), &str_val("task.completed"))
            && !strict_eq(kind_v.as_ref(), &str_val("task.updated"))
        {
            continue;
        }
        if !strict_eq(kind_v.as_ref(), &str_val("task.started"))
            && !strict_eq(kind_v.as_ref(), &str_val("task.progress"))
            && !strict_eq(kind_v.as_ref(), &str_val("task.updated"))
            && !strict_eq(kind_v.as_ref(), &str_val("task.completed"))
        {
            continue;
        }
        let payload_v = prop(a, "payload", "activity.payload")?;
        let payload = as_record(payload_v.as_ref());
        let task_id = payload.and_then(|p| {
            let t = prop_opt(p, "taskId");
            if kind(t.as_ref()) == K::Str {
                Some(t.into_owned())
            } else {
                None
            }
        });
        let Some(task_id) = task_id else {
            continue;
        };
        if strict_eq(kind_v.as_ref(), &str_val("task.completed")) {
            active_tasks.delete(&task_id);
            continue;
        }
        if strict_eq(kind_v.as_ref(), &str_val("task.updated")) {
            let status = payload.and_then(|p| {
                let s = prop_opt(p, "status");
                if kind(s.as_ref()) == K::Str {
                    Some(s.into_owned())
                } else {
                    None
                }
            });
            if status.as_ref().is_some_and(|s| {
                strict_eq(s, &str_val("completed"))
                    || strict_eq(s, &str_val("failed"))
                    || strict_eq(s, &str_val("killed"))
                    || strict_eq(s, &str_val("paused"))
            }) {
                active_tasks.delete(&task_id);
                continue;
            }
            let previous = active_tasks.get(&task_id).cloned();
            let is_backgrounded = {
                let ib = payload.map(|p| prop_opt(p, "isBackgrounded").into_owned());
                match ib {
                    Some(v) if kind(&v) == K::Bool => v,
                    _ => previous
                        .as_ref()
                        .map(|p| prop_opt(p, "isBackgrounded").into_owned())
                        .filter(|v| !is_nullish(v))
                        .unwrap_or(Value::Bool(false)),
                }
            };
            let in_turn = !truthy(latest_turn_id)
                || !truthy(turn_id.as_ref())
                || strict_eq(turn_id.as_ref(), latest_turn_id);
            if previous.is_some() || (truthy(&is_backgrounded) && in_turn) {
                let mut o = Map::new();
                if let Some(p) = &previous {
                    for (k, x) in spread_obj(p) {
                        o.insert(k, x);
                    }
                }
                o.insert("isBackgrounded".into(), is_backgrounded);
                let prev_tool = previous
                    .as_ref()
                    .map(|p| prop_opt(p, "toolUseId").into_owned());
                o.insert(
                    "toolUseId".into(),
                    prev_tool
                        .filter(|v| !is_nullish(v))
                        .or_else(|| task_payload_string(payload, "toolUseId"))
                        .unwrap_or_else(undefined),
                );
                let prev_wf = previous
                    .as_ref()
                    .map(|p| prop_opt(p, "workflowTaskId").into_owned());
                o.insert(
                    "workflowTaskId".into(),
                    prev_wf
                        .filter(|v| !is_nullish(v))
                        .or_else(|| task_payload_string(payload, "workflowTaskId"))
                        .unwrap_or_else(undefined),
                );
                let prev_tid = previous
                    .as_ref()
                    .map(|p| prop_opt(p, "turnId").into_owned());
                o.insert(
                    "turnId".into(),
                    prev_tid
                        .filter(|v| !is_nullish(v))
                        .or_else(|| {
                            if is_nullish(turn_id.as_ref()) {
                                None
                            } else {
                                Some(turn_id.as_ref().clone())
                            }
                        })
                        .unwrap_or(Value::Null),
                );
                active_tasks.set(task_id.clone(), Cow::Owned(Value::Object(o)));
            }
            continue;
        }
        let previous = active_tasks.get(&task_id).cloned();
        let mut o = Map::new();
        if let Some(p) = &previous {
            for (k, x) in spread_obj(p) {
                o.insert(k, x);
            }
        }
        let new_type = task_payload_string(payload, "taskType").or_else(|| {
            previous
                .as_ref()
                .map(|p| prop_opt(p, "taskType").into_owned())
                .filter(|v| !is_nullish(v))
        });
        o.insert("taskType".into(), new_type.unwrap_or_else(undefined));
        o.insert(
            "isBackgrounded".into(),
            previous
                .as_ref()
                .map(|p| prop_opt(p, "isBackgrounded").into_owned())
                .filter(|v| !is_nullish(v))
                .unwrap_or(Value::Bool(false)),
        );
        o.insert(
            "toolUseId".into(),
            previous
                .as_ref()
                .map(|p| prop_opt(p, "toolUseId").into_owned())
                .filter(|v| !is_nullish(v))
                .or_else(|| task_payload_string(payload, "toolUseId"))
                .unwrap_or_else(undefined),
        );
        o.insert(
            "subagentType".into(),
            previous
                .as_ref()
                .map(|p| prop_opt(p, "subagentType").into_owned())
                .filter(|v| !is_nullish(v))
                .or_else(|| task_payload_string(payload, "subagentType"))
                .unwrap_or_else(undefined),
        );
        o.insert(
            "workflowTaskId".into(),
            previous
                .as_ref()
                .map(|p| prop_opt(p, "workflowTaskId").into_owned())
                .filter(|v| !is_nullish(v))
                .or_else(|| task_payload_string(payload, "workflowTaskId"))
                .unwrap_or_else(undefined),
        );
        o.insert(
            "turnId".into(),
            previous
                .as_ref()
                .map(|p| prop_opt(p, "turnId").into_owned())
                .filter(|v| !is_nullish(v))
                .or_else(|| {
                    if is_nullish(turn_id.as_ref()) {
                        None
                    } else {
                        Some(turn_id.as_ref().clone())
                    }
                })
                .unwrap_or(Value::Null),
        );
        active_tasks.set(task_id.clone(), Cow::Owned(Value::Object(o)));
    }
    Ok(active_tasks)
}

fn derive_active_background_tasks<'a>(
    activities: &'a Value,
    latest_turn_id: &'a Value,
) -> Result<Option<Value>, String> {
    let folded = fold_active_tasks(activities, latest_turn_id)?;
    let mut ids = Vec::new();
    for (task_id, task) in &folded.entries {
        if !strict_eq(
            prop_opt(task.as_ref(), "taskType").as_ref(),
            &str_val("plan"),
        ) {
            ids.push(task_id.clone());
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    let mut o = Map::new();
    o.insert("activeCount".into(), num_val(ids.len() as f64));
    o.insert("taskIds".into(), Value::Array(ids));
    Ok(Some(Value::Object(o)))
}

fn derive_pending_background_work<'a>(
    activities: &'a Value,
    latest_turn: &'a Value,
    session: &'a Value,
) -> Result<Option<Value>, String> {
    let latest_turn_id = prop_opt(latest_turn, "turnId");
    if !truthy(latest_turn_id.as_ref()) {
        return Ok(None);
    }
    let session_status = prop_opt(session, "orchestrationStatus");
    if is_undef(session_status.as_ref())
        || strict_eq(session_status.as_ref(), &str_val("stopped"))
        || strict_eq(session_status.as_ref(), &str_val("error"))
    {
        return Ok(None);
    }
    let folded = fold_active_tasks(activities, latest_turn_id.as_ref())?;
    let mut ids = Vec::new();
    for (task_id, task) in &folded.entries {
        if truthy(prop_opt(task.as_ref(), "isBackgrounded").as_ref()) {
            ids.push(task_id.clone());
        }
    }
    if ids.is_empty() {
        return Ok(None);
    }
    let mut o = Map::new();
    o.insert("count".into(), num_val(ids.len() as f64));
    o.insert("taskIds".into(), Value::Array(ids));
    Ok(Some(Value::Object(o)))
}

fn fold_outstanding_background_ids<'a>(
    activities: &[Cow<'a, Value>],
) -> Result<Vec<Value>, String> {
    let mut outstanding = JsSet::new();
    for activity in activities {
        let a = activity.as_ref();
        let payload_v = prop(a, "payload", "activity.payload")?;
        let payload = as_record(payload_v.as_ref());
        let kind_v = prop(a, "kind", "activity.kind")?;
        if strict_eq(kind_v.as_ref(), &str_val("runtime.warning")) {
            if !payload.is_some_and(|p| {
                strict_eq(
                    prop_opt(p, "nativeEventType").as_ref(),
                    &str_val("background_tasks_changed"),
                )
            }) {
                continue;
            }
            let data = payload.and_then(|p| {
                let d = prop_opt(p, "data");
                if as_record(d.as_ref()).is_some() {
                    Some(d.into_owned())
                } else {
                    None
                }
            });
            let tasks = data.map(|d| prop_opt(&d, "tasks").into_owned());
            if !tasks.as_ref().is_some_and(|t| matches!(t, Value::Array(_))) {
                continue;
            }
            if let Some(Value::Array(task_list)) = tasks {
                for task in &task_list {
                    let tid = as_record(task)
                        .map(|t| prop_opt(t, "task_id").into_owned())
                        .filter(|t| kind(t) == K::Str);
                    if let Some(t) = tid {
                        outstanding.add(t);
                    }
                }
            }
            continue;
        }
        let task_id = payload.and_then(|p| {
            let t = prop_opt(p, "taskId");
            if kind(t.as_ref()) == K::Str {
                Some(t.into_owned())
            } else {
                None
            }
        });
        let Some(task_id) = task_id else {
            continue;
        };
        if strict_eq(kind_v.as_ref(), &str_val("task.completed")) {
            outstanding.delete(&task_id);
        } else if strict_eq(kind_v.as_ref(), &str_val("task.updated")) {
            let status = payload.and_then(|p| {
                let s = prop_opt(p, "status");
                if kind(s.as_ref()) == K::Str {
                    Some(s.into_owned())
                } else {
                    None
                }
            });
            let ib = payload.map(|p| prop_opt(p, "isBackgrounded").into_owned());
            if status.as_ref().is_some_and(|s| {
                strict_eq(s, &str_val("completed"))
                    || strict_eq(s, &str_val("failed"))
                    || strict_eq(s, &str_val("killed"))
                    || strict_eq(s, &str_val("paused"))
            }) || strict_eq(ib.as_ref().unwrap_or(&Value::Null), &Value::Bool(false))
            {
                outstanding.delete(&task_id);
            } else if strict_eq(ib.as_ref().unwrap_or(&Value::Null), &Value::Bool(true)) {
                outstanding.add(task_id.clone());
            }
        }
    }
    Ok(outstanding.items)
}

fn count_outstanding_background_work<'a>(
    activities: &'a Value,
    session: &'a Value,
) -> Result<f64, String> {
    let status = prop_opt(session, "orchestrationStatus");
    let can_finish = !status.is_null()
        && !is_undef(status.as_ref())
        && !strict_eq(status.as_ref(), &str_val("stopped"))
        && !strict_eq(status.as_ref(), &str_val("error"));
    if !can_finish {
        return Ok(0.0);
    }
    let ordered = ordered_activities(activities, OrderFlavor::Worklog)?;
    let ids = fold_outstanding_background_ids(&ordered)?;
    Ok(ids.len() as f64)
}

fn has_live_turn_tail_work<'a>(
    latest_turn: &'a Value,
    messages: &'a Value,
    activities: &'a Value,
    session: &'a Value,
) -> Result<bool, String> {
    let latest_turn_id = prop_opt(latest_turn, "turnId");
    if !truthy(latest_turn_id.as_ref()) {
        return Ok(false);
    }
    let has_streaming = some_call(
        messages,
        "input.messages.some",
        "input.messages.some((message) => message.role === \"assistant\" && message.turnId === latestTurnId && message.streaming)",
        &mut |message: &Value, _| {
            Ok(strict_eq(
                prop(message, "role", "message.role")?.as_ref(),
                &str_val("assistant"),
            ) && strict_eq(
                prop(message, "turnId", "message.turnId")?.as_ref(),
                latest_turn_id.as_ref(),
            ) && truthy(prop(message, "streaming", "message.streaming")?.as_ref()))
        },
    )?;
    if has_streaming {
        return Ok(is_nullish(prop_opt(latest_turn, "completedAt").as_ref()));
    }
    if !strict_eq(
        prop_opt(session, "orchestrationStatus").as_ref(),
        &str_val("running"),
    ) {
        return Ok(false);
    }
    Ok(derive_active_background_tasks(activities, latest_turn_id.as_ref())?.is_some())
}

// ---------------------------------------------------------------------------
// lastVisitedAt (threadVisitedPersistence.ts)
// ---------------------------------------------------------------------------

fn latest_server_time(thread: &Value) -> Result<Value, String> {
    let lt = prop(thread, "latestTurn", "thread.latestTurn")?;
    let completed = prop_opt(lt.as_ref(), "completedAt");
    let completed_or_undef = if is_nullish(completed.as_ref()) {
        Cow::Owned(js::undefined())
    } else {
        completed
    };
    let upd = prop(thread, "updatedAt", "thread.updatedAt")?;
    let l = date_parse_val(completed_or_undef.as_ref());
    let l = if l.is_nan() { 0.0 } else { l };
    let r = date_parse_val(upd.as_ref());
    let r = if r.is_nan() { 0.0 } else { r };
    if l > r {
        Ok(completed_or_undef.into_owned())
    } else {
        Ok(upd.into_owned())
    }
}

fn resolve_initial_last_visited_at<'a>(
    thread: &'a Value,
    options: &'a Value,
) -> Result<Value, String> {
    // Persisted visited state is empty in the worker.
    let id = prop(thread, "id", "thread.id")?;
    let _ = id;
    let latest_at = latest_server_time(thread)?;
    let restoring = prop(options, "restoringSession", "options.restoringSession")?;
    if strict_eq(restoring.as_ref(), &Value::Bool(true)) {
        // watermarkAt is always null, so the restoring branch never wins.
    }
    Ok(latest_at)
}

// ---------------------------------------------------------------------------
// Exported ports
// ---------------------------------------------------------------------------

/// Port of `threadSessionsEqual`.
pub fn thread_sessions_equal(left: Value, right: Value) -> Result<Value, String> {
    if strict_eq(&left, &right) {
        return Ok(Value::Bool(true));
    }
    if is_nullish(&left) || is_nullish(&right) {
        return Ok(Value::Bool(false));
    }
    for (f, le, re) in [
        ("provider", "left.provider", "right.provider"),
        ("status", "left.status", "right.status"),
        (
            "orchestrationStatus",
            "left.orchestrationStatus",
            "right.orchestrationStatus",
        ),
        ("runtimeMode", "left.runtimeMode", "right.runtimeMode"),
        ("activeTurnId", "left.activeTurnId", "right.activeTurnId"),
        ("createdAt", "left.createdAt", "right.createdAt"),
        ("updatedAt", "left.updatedAt", "right.updatedAt"),
        ("lastError", "left.lastError", "right.lastError"),
    ] {
        if !strict_eq(prop(&left, f, le)?.as_ref(), prop(&right, f, re)?.as_ref()) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// Port of `resolveCreateBranchFlowCompletedMerge`.
pub fn resolve_create_branch_flow_completed_merge(input: Value) -> Result<Value, String> {
    let cur_branch = prop(&input, "currentBranch", "input.currentBranch")?;
    let next_branch = prop(&input, "nextBranch", "input.nextBranch")?;
    let cur_wt = prop(&input, "currentWorktreePath", "input.currentWorktreePath")?;
    let next_wt = prop(&input, "nextWorktreePath", "input.nextWorktreePath")?;
    let cur_awp = prop(
        &input,
        "currentAssociatedWorktreePath",
        "input.currentAssociatedWorktreePath",
    )?;
    let next_awp = prop(
        &input,
        "nextAssociatedWorktreePath",
        "input.nextAssociatedWorktreePath",
    )?;
    let cur_awb = prop(
        &input,
        "currentAssociatedWorktreeBranch",
        "input.currentAssociatedWorktreeBranch",
    )?;
    let next_awb = prop(
        &input,
        "nextAssociatedWorktreeBranch",
        "input.nextAssociatedWorktreeBranch",
    )?;
    let cur_awr = prop(
        &input,
        "currentAssociatedWorktreeRef",
        "input.currentAssociatedWorktreeRef",
    )?;
    let next_awr = prop(
        &input,
        "nextAssociatedWorktreeRef",
        "input.nextAssociatedWorktreeRef",
    )?;
    let cur_completed = prop(
        &input,
        "currentCreateBranchFlowCompleted",
        "input.currentCreateBranchFlowCompleted",
    )?;
    let next_completed = prop(
        &input,
        "nextCreateBranchFlowCompleted",
        "input.nextCreateBranchFlowCompleted",
    )?;
    let or_null = |c: &Cow<Value>| -> Value {
        if is_nullish(c.as_ref()) {
            Value::Null
        } else {
            c.as_ref().clone()
        }
    };
    let context_changed = !strict_eq(cur_branch.as_ref(), next_branch.as_ref())
        || !strict_eq(cur_wt.as_ref(), next_wt.as_ref())
        || !strict_eq(&or_null(&cur_awp), &or_null(&next_awp))
        || !strict_eq(&or_null(&cur_awb), &or_null(&next_awb))
        || !strict_eq(&or_null(&cur_awr), &or_null(&next_awr));
    if context_changed {
        return Ok(if is_nullish(next_completed.as_ref()) {
            Value::Bool(false)
        } else {
            next_completed.as_ref().clone()
        });
    }
    if is_undef(next_completed.as_ref()) {
        return Ok(if is_nullish(cur_completed.as_ref()) {
            Value::Bool(false)
        } else {
            cur_completed.as_ref().clone()
        });
    }
    if truthy(&if is_nullish(cur_completed.as_ref()) {
        Value::Bool(false)
    } else {
        cur_completed.as_ref().clone()
    }) && !truthy(next_completed.as_ref())
    {
        return Ok(Value::Bool(true));
    }
    Ok(next_completed.as_ref().clone())
}

/// Port of `threadShellsEqual`.
pub fn thread_shells_equal(left: Value, right: Value) -> Result<Value, String> {
    if is_undef(&left) {
        return Ok(Value::Bool(false));
    }
    macro_rules! cmp {
        ($f:literal) => {
            if !strict_eq(
                prop(&left, $f, concat!("left.", $f))?.as_ref(),
                prop(&right, $f, concat!("right.", $f))?.as_ref(),
            ) {
                return Ok(Value::Bool(false));
            }
        };
    }
    cmp!("id");
    cmp!("codexThreadId");
    cmp!("projectId");
    cmp!("title");
    cmp!("modelSelection");
    cmp!("runtimeMode");
    cmp!("interactionMode");
    cmp!("error");
    cmp!("createdAt");
    for f in [
        "archivedAt",
        "settledAt",
        "snoozedUntil",
        "snoozeReminderAt",
    ] {
        let l = prop(&left, f, &format!("left.{f}"))?;
        let r = prop(&right, f, &format!("right.{f}"))?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    cmp!("updatedAt");
    {
        let l = prop(&left, "isPinned", "left.isPinned")?;
        let r = prop(&right, "isPinned", "right.isPinned")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Bool(false)
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Bool(false)
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    cmp!("envMode");
    cmp!("branch");
    cmp!("worktreePath");
    for f in [
        "workingDirectory",
        "associatedWorktreePath",
        "associatedWorktreeBranch",
        "associatedWorktreeRef",
    ] {
        let l = prop(&left, f, &format!("left.{f}"))?;
        let r = prop(&right, f, &format!("right.{f}"))?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(
            &left,
            "createBranchFlowCompleted",
            "left.createBranchFlowCompleted",
        )?;
        let r = prop(
            &right,
            "createBranchFlowCompleted",
            "right.createBranchFlowCompleted",
        )?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Bool(false)
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Bool(false)
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    for f in [
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
        "forkSourceThreadId",
        "sidechatSourceThreadId",
    ] {
        let l = prop(&left, f, &format!("left.{f}"))?;
        let r = prop(&right, f, &format!("right.{f}"))?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let lctx = prop(&left, "sidechatContext", "left.sidechatContext")?;
        let rctx = prop(&right, "sidechatContext", "right.sidechatContext")?;
        let lu = prop_opt(lctx.as_ref(), "url");
        let ru = prop_opt(rctx.as_ref(), "url");
        let ln = if is_nullish(lu.as_ref()) {
            Value::Null
        } else {
            lu.as_ref().clone()
        };
        let rn = if is_nullish(ru.as_ref()) {
            Value::Null
        } else {
            ru.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    for f in ["sidechatLastActivityAt", "sidechatExpiredAt"] {
        let l = prop(&left, f, &format!("left.{f}"))?;
        let r = prop(&right, f, &format!("right.{f}"))?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(&left, "lastKnownPr", "left.lastKnownPr")?;
        let r = prop(&right, "lastKnownPr", "right.lastKnownPr")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !deep_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(&left, "pullRequests", "left.pullRequests")?;
        let r = prop(&right, "pullRequests", "right.pullRequests")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Array(vec![])
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Array(vec![])
        } else {
            r.as_ref().clone()
        };
        if !deep_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(&left, "handoff", "left.handoff")?;
        let r = prop(&right, "handoff", "right.handoff")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(&left, "isProjectImport", "left.isProjectImport")?;
        let r = prop(&right, "isProjectImport", "right.isProjectImport")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Bool(false)
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Bool(false)
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(&left, "claudeCacheReview", "left.claudeCacheReview")?;
        let r = prop(&right, "claudeCacheReview", "right.claudeCacheReview")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !deep_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    cmp!("claudeCacheReviewSequence");
    {
        let l = prop(&left, "usageLimitRecovery", "left.usageLimitRecovery")?;
        let r = prop(&right, "usageLimitRecovery", "right.usageLimitRecovery")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !deep_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    cmp!("usageLimitRecoverySequence");
    cmp!("snoozeSequence");
    {
        let l = prop(&left, "pinnedMessages", "left.pinnedMessages")?;
        let r = prop(&right, "pinnedMessages", "right.pinnedMessages")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !deep_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    for f in ["notes", "goal"] {
        let l = prop(&left, f, &format!("left.{f}"))?;
        let r = prop(&right, f, &format!("right.{f}"))?;
        let ln = if is_nullish(l.as_ref()) {
            str_val("")
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            str_val("")
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    for f in ["goalStartedAt", "goalPausedAt"] {
        let l = prop(&left, f, &format!("left.{f}"))?;
        let r = prop(&right, f, &format!("right.{f}"))?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !strict_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    {
        let l = prop(&left, "goalAchievements", "left.goalAchievements")?;
        let r = prop(&right, "goalAchievements", "right.goalAchievements")?;
        let ln = if is_nullish(l.as_ref()) {
            Value::Null
        } else {
            l.as_ref().clone()
        };
        let rn = if is_nullish(r.as_ref()) {
            Value::Null
        } else {
            r.as_ref().clone()
        };
        if !deep_eq(&ln, &rn) {
            return Ok(Value::Bool(false));
        }
    }
    cmp!("latestUserMessageAt");
    cmp!("latestHumanMessageAt");
    cmp!("hasPendingApprovals");
    cmp!("hasPendingUserInput");
    cmp!("hasActionableProposedPlan");
    cmp!("pendingBackgroundTaskCount");
    cmp!("latestReplyPreview");
    cmp!("pendingInteractions");
    cmp!("lastVisitedAt");
    Ok(Value::Bool(true))
}

/// Port of `threadTurnStatesEqual`.
pub fn thread_turn_states_equal(left: Value, right: Value) -> Result<Value, String> {
    if is_undef(&left) {
        return Ok(Value::Bool(false));
    }
    if !latest_turns_equal(
        prop(&left, "latestTurn", "left.latestTurn")?.as_ref(),
        prop(&right, "latestTurn", "right.latestTurn")?.as_ref(),
    )? {
        return Ok(Value::Bool(false));
    }
    if !source_proposed_plans_equal(
        prop(
            &left,
            "pendingSourceProposedPlan",
            "left.pendingSourceProposedPlan",
        )?
        .as_ref(),
        prop(
            &right,
            "pendingSourceProposedPlan",
            "right.pendingSourceProposedPlan",
        )?
        .as_ref(),
    )? {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(strict_eq(
        prop(
            &left,
            "pendingTurnStartMessageId",
            "left.pendingTurnStartMessageId",
        )?
        .as_ref(),
        prop(
            &right,
            "pendingTurnStartMessageId",
            "right.pendingTurnStartMessageId",
        )?
        .as_ref(),
    )))
}

/// Port of `textSegmentArraysEqual`.
pub fn text_segment_arrays_equal(left: Value, right: Value) -> Result<Value, String> {
    Ok(Value::Bool(text_segments_eq(&left, &right)?))
}

/// Port of `providerReferenceArraysEqual`.
pub fn provider_reference_arrays_equal(left: Value, right: Value) -> Result<Value, String> {
    Ok(Value::Bool(provider_refs_eq(&left, &right)?))
}

/// Port of `deepEqualJson`.
pub fn deep_equal_json(left: Value, right: Value) -> Result<Value, String> {
    Ok(Value::Bool(deep_eq(&left, &right)))
}

/// Port of `normalizeModelSelection`.
pub fn normalize_model_selection(value: Value, previous: Value) -> Result<Value, String> {
    normalize_model_selection_inner(&value, &previous)
}

/// Port of `normalizeProject`.
pub fn normalize_project(incoming: Value, previous: Value) -> Result<Value, String> {
    normalize_project_inner(&incoming, &previous)
}

/// Port of `normalizeSpace`.
pub fn normalize_space(incoming: Value, previous: Value) -> Result<Value, String> {
    if truthy(&previous)
        && strict_eq(
            prop(&previous, "id", "previous.id")?.as_ref(),
            prop(&incoming, "id", "incoming.id")?.as_ref(),
        )
        && strict_eq(
            prop(&previous, "name", "previous.name")?.as_ref(),
            prop(&incoming, "name", "incoming.name")?.as_ref(),
        )
        && strict_eq(
            prop(&previous, "icon", "previous.icon")?.as_ref(),
            prop(&incoming, "icon", "incoming.icon")?.as_ref(),
        )
        && strict_eq(
            prop(&previous, "sortOrder", "previous.sortOrder")?.as_ref(),
            prop(&incoming, "sortOrder", "incoming.sortOrder")?.as_ref(),
        )
        && strict_eq(
            prop(&previous, "createdAt", "previous.createdAt")?.as_ref(),
            prop(&incoming, "createdAt", "incoming.createdAt")?.as_ref(),
        )
        && strict_eq(
            prop(&previous, "updatedAt", "previous.updatedAt")?.as_ref(),
            prop(&incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
        )
    {
        return Ok(previous);
    }
    let mut o = Map::new();
    for f in ["id", "name", "icon", "sortOrder", "createdAt", "updatedAt"] {
        o.insert(
            f.into(),
            prop(&incoming, f, &format!("incoming.{f}"))?
                .as_ref()
                .clone(),
        );
    }
    Ok(Value::Object(o))
}

/// Port of `mapSpaces`.
pub fn map_spaces(incoming: Value, previous: Value) -> Result<Value, String> {
    let pairs = map_call(
        &previous,
        "previous.map",
        "previous.map((space) => [space.id, space])",
        &mut |space: Cow<Value>, _| {
            let id = prop(space.as_ref(), "id", "space.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                space.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_id = map_of_pairs(pairs);
    let mut next = map_call(
        &incoming,
        "incoming.map",
        "incoming.map((space) => normalizeSpace(space, previousById.get(space.id)))",
        &mut |space: Cow<Value>, _| {
            let id = prop(space.as_ref(), "id", "space.id")?;
            let prev = previous_by_id.get(id.as_ref()).unwrap_or(undef());
            Ok(Cow::Owned(normalize_space(
                space.as_ref().clone(),
                prev.clone(),
            )?))
        },
    )?;
    js_sort_res(&mut next, &mut |l: &Cow<Value>, r: &Cow<Value>| {
        let ls = prop(l.as_ref(), "sortOrder", "left.sortOrder")?;
        let rs = prop(r.as_ref(), "sortOrder", "right.sortOrder")?;
        let d = to_number(ls.as_ref()) - to_number(rs.as_ref());
        if d != 0.0 && !d.is_nan() {
            return Ok(d);
        }
        let lid = prop(l.as_ref(), "id", "left.id")?;
        let rid = prop(r.as_ref(), "id", "right.id")?;
        locale_call(
            lid.as_ref(),
            "left.id.localeCompare",
            "left.id.localeCompare(right.id)",
            rid.as_ref(),
        )
    })?;
    if arrays_shallow_eq(&previous, &next)? {
        Ok(previous)
    } else {
        Ok(Value::Array(
            next.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

/// Port of `normalizeChatMessage`.
pub fn normalize_chat_message(incoming: Value, previous: Value) -> Result<Value, String> {
    normalize_chat_message_inner(&incoming, &previous)
}

/// Port of `mergeReadModelThreadDetailWithLiveHotPath`.
pub fn merge_read_model_thread_detail_with_live_hot_path(
    incoming: Value,
    previous_thread: Value,
    snapshot_sequence: Value,
) -> Result<Value, String> {
    if !truthy(&previous_thread) {
        return Ok(incoming);
    }
    let settled_local_turn_id: Cow<Value> = {
        let plt = prop(&previous_thread, "latestTurn", "previousThread.latestTurn")?;
        let plt_state = prop_opt(plt.as_ref(), "state");
        let ilt = prop(&incoming, "latestTurn", "incoming.latestTurn")?;
        if strict_eq(plt_state.as_ref(), &str_val("running"))
            && !ilt.is_null()
            && strict_eq(
                prop(ilt.as_ref(), "turnId", "incoming.latestTurn.turnId")?.as_ref(),
                prop(plt.as_ref(), "turnId", "previousThread.latestTurn.turnId")?.as_ref(),
            )
            && !strict_eq(
                prop(ilt.as_ref(), "state", "incoming.latestTurn.state")?.as_ref(),
                &str_val("running"),
            )
            && !prop(
                ilt.as_ref(),
                "completedAt",
                "incoming.latestTurn.completedAt",
            )?
            .is_null()
        {
            Cow::Owned(prop(ilt.as_ref(), "turnId", "incoming.latestTurn.turnId")?.into_owned())
        } else {
            Cow::Owned(Value::Null)
        }
    };
    let preserve_running_turn = settled_local_turn_id.is_null()
        && should_preserve_running_turn(&previous_thread, &incoming)?;
    let merged_messages = merge_messages(
        prop(&incoming, "messages", "incoming.messages")?.as_ref(),
        &previous_thread,
        &settled_local_turn_id,
    )?;
    let messages = if settled_local_turn_id.is_null() {
        Cow::Owned(merged_messages)
    } else {
        clear_settled_streaming(&merged_messages, &settled_local_turn_id)?
    };
    let incoming_latest_turn = prop(&incoming, "latestTurn", "incoming.latestTurn")?;
    let session = merge_session(
        prop(&incoming, "session", "incoming.session")?.as_ref(),
        &previous_thread,
        preserve_running_turn,
        &incoming_latest_turn,
    )?;
    let latest_turn = merge_latest_turn(
        &incoming_latest_turn,
        &previous_thread,
        preserve_running_turn,
    )?;
    let pt_ccr = prop(
        &previous_thread,
        "claudeCacheReview",
        "previousThread.claudeCacheReview",
    )?;
    let claude_cache_review = if !is_undef(pt_ccr.as_ref())
        && (if is_undef(&snapshot_sequence) {
            js_lt(
                prop(&incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
                {
                    let u = prop(&previous_thread, "updatedAt", "previousThread.updatedAt")?;
                    if is_nullish(u.as_ref()) {
                        prop(&previous_thread, "createdAt", "previousThread.createdAt")?
                    } else {
                        u
                    }
                }
                .as_ref(),
            )
        } else {
            js_lt(&snapshot_sequence, &{
                let s = prop(
                    &previous_thread,
                    "claudeCacheReviewSequence",
                    "previousThread.claudeCacheReviewSequence",
                )?;
                if is_nullish(s.as_ref()) {
                    num_val(0.0)
                } else {
                    s.as_ref().clone()
                }
            })
        }) {
        clone_cow(&pt_ccr)
    } else {
        prop(&incoming, "claudeCacheReview", "incoming.claudeCacheReview")?
    };
    let usage_limit_recovery = if is_undef(&snapshot_sequence) {
        js_lt(
            prop(&incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
            &{
                let u = prop(&previous_thread, "updatedAt", "previousThread.updatedAt")?;
                if is_nullish(u.as_ref()) {
                    prop(&previous_thread, "createdAt", "previousThread.createdAt")?
                        .as_ref()
                        .clone()
                } else {
                    u.as_ref().clone()
                }
            },
        )
    } else {
        js_lt(&snapshot_sequence, &{
            let s = prop(
                &previous_thread,
                "usageLimitRecoverySequence",
                "previousThread.usageLimitRecoverySequence",
            )?;
            if is_nullish(s.as_ref()) {
                num_val(0.0)
            } else {
                s.as_ref().clone()
            }
        })
    };
    let usage_limit_recovery = if usage_limit_recovery {
        prop(
            &previous_thread,
            "usageLimitRecovery",
            "previousThread.usageLimitRecovery",
        )?
    } else {
        prop(
            &incoming,
            "usageLimitRecovery",
            "incoming.usageLimitRecovery",
        )?
    };
    let preserve_snooze = if is_undef(&snapshot_sequence) {
        js_lt(
            prop(&incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
            &{
                let u = prop(&previous_thread, "updatedAt", "previousThread.updatedAt")?;
                if is_nullish(u.as_ref()) {
                    prop(&previous_thread, "createdAt", "previousThread.createdAt")?
                        .as_ref()
                        .clone()
                } else {
                    u.as_ref().clone()
                }
            },
        )
    } else {
        js_lt(&snapshot_sequence, &{
            let s = prop(
                &previous_thread,
                "snoozeSequence",
                "previousThread.snoozeSequence",
            )?;
            if is_nullish(s.as_ref()) {
                num_val(0.0)
            } else {
                s.as_ref().clone()
            }
        })
    };
    let snoozed_until = if preserve_snooze {
        prop(
            &previous_thread,
            "snoozedUntil",
            "previousThread.snoozedUntil",
        )?
    } else {
        prop(&incoming, "snoozedUntil", "incoming.snoozedUntil")?
    };
    let snooze_reminder_at = if preserve_snooze {
        prop(
            &previous_thread,
            "snoozeReminderAt",
            "previousThread.snoozeReminderAt",
        )?
    } else {
        prop(&incoming, "snoozeReminderAt", "incoming.snoozeReminderAt")?
    };
    let inc_messages = prop(&incoming, "messages", "incoming.messages")?;
    let inc_session = prop(&incoming, "session", "incoming.session")?;
    if strict_eq(messages.as_ref(), inc_messages.as_ref())
        && strict_eq(&session, &inc_session)
        && strict_eq(&latest_turn, &incoming_latest_turn)
        && strict_eq(
            claude_cache_review.as_ref(),
            prop(&incoming, "claudeCacheReview", "incoming.claudeCacheReview")?.as_ref(),
        )
        && strict_eq(
            usage_limit_recovery.as_ref(),
            prop(
                &incoming,
                "usageLimitRecovery",
                "incoming.usageLimitRecovery",
            )?
            .as_ref(),
        )
        && strict_eq(
            snoozed_until.as_ref(),
            prop(&incoming, "snoozedUntil", "incoming.snoozedUntil")?.as_ref(),
        )
        && strict_eq(
            snooze_reminder_at.as_ref(),
            prop(&incoming, "snoozeReminderAt", "incoming.snoozeReminderAt")?.as_ref(),
        )
    {
        return Ok(incoming);
    }
    let mut o = Map::new();
    for (k, x) in spread_obj(&incoming) {
        o.insert(k, x);
    }
    o.insert("messages".into(), messages.as_ref().clone());
    o.insert("session".into(), session.clone());
    o.insert("latestTurn".into(), latest_turn.clone());
    if !is_undef(claude_cache_review.as_ref()) {
        o.insert(
            "claudeCacheReview".into(),
            claude_cache_review.as_ref().clone(),
        );
    }
    if !is_undef(usage_limit_recovery.as_ref()) {
        o.insert(
            "usageLimitRecovery".into(),
            usage_limit_recovery.as_ref().clone(),
        );
    }
    if !is_undef(snoozed_until.as_ref()) {
        o.insert("snoozedUntil".into(), snoozed_until.as_ref().clone());
    }
    if !is_undef(snooze_reminder_at.as_ref()) {
        o.insert(
            "snoozeReminderAt".into(),
            snooze_reminder_at.as_ref().clone(),
        );
    }
    Ok(Value::Object(o))
}

/// Port of `normalizeProposedPlans`.
pub fn normalize_proposed_plans(incoming: Value, previous: Value) -> Result<Value, String> {
    normalize_proposed_plans_inner(&incoming, &previous)
}

/// Port of `normalizeTurnDiffFiles`.
pub fn normalize_turn_diff_files(incoming: Value, previous: Value) -> Result<Value, String> {
    normalize_turn_diff_files_inner(&incoming, &previous)
}

/// Port of `normalizeActivities`.
pub fn normalize_activities(incoming: Value, previous: Value) -> Result<Value, String> {
    normalize_activities_inner(&incoming, &previous)
}

/// Port of `withOrchestrationEventSequence`.
pub fn with_orchestration_event_sequence(
    activity: Value,
    sequence: Value,
) -> Result<Value, String> {
    if !is_undef(prop(&activity, "sequence", "activity.sequence")?.as_ref()) {
        return Ok(activity);
    }
    let mut o = Map::new();
    for (k, x) in spread_obj(&activity) {
        o.insert(k, x);
    }
    o.insert("sequence".into(), sequence);
    o.insert("sequenceSource".into(), str_val("orchestration"));
    Ok(Value::Object(o))
}

/// Port of `capThreadActivities`.
pub fn cap_thread_activities(activities: Value) -> Result<Value, String> {
    let len = len_num(&activities, "activities.length")?;
    if len <= 2000.0 {
        return Ok(activities);
    }
    let drop_count = resolve_turn_aligned_drop_count(&activities, len - 2000.0)?;
    let tail = slice_call(
        &activities,
        "activities.slice",
        "activities.slice(dropCount)",
        &[drop_count],
    )?;
    let tail_items = iter(tail.as_ref(), "activity", "tail")?;
    let mut retained_ids = JsSet::new();
    for activity in &tail_items {
        let id = prop(activity.as_ref(), "id", "activity.id")?;
        retained_ids.add(id.as_ref().clone());
    }
    retain_task_activity_ids(&activities, &mut retained_ids)?;
    let identities = pending_interactions_from_activities(&activities)?;
    let mut retained_requests = JsMap::new();
    for request in &identities {
        let key = prop(identity_obj(request), "key", "request.key")?;
        retained_requests.set(key.as_ref().clone(), Cow::Owned(request.clone()));
    }
    for activity in &tail_items {
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        if !PENDING_REQUEST_KINDS
            .iter()
            .any(|k| strict_eq(kind_v.as_ref(), &str_val(k)))
        {
            continue;
        }
        if let Some(identity) = pending_interaction_identity(activity.as_ref())? {
            let key = prop(identity_obj(&identity), "key", "identity.key")?;
            retained_requests.set(key.as_ref().clone(), Cow::Owned(identity));
        }
    }
    if retained_requests.size() == 0 {
        return Ok(Value::Array(
            filter_call(
                &activities,
                "activities.filter",
                "activities.filter((activity) => retainedIds.has(activity.id))",
                &mut |activity: &Value, _| {
                    let id = prop(activity, "id", "activity.id")?;
                    Ok(retained_ids.has(id.as_ref()))
                },
            )?
            .iter()
            .map(|c| c.as_ref().clone())
            .collect(),
        ));
    }
    let mut oldest_retained_at = JsMap::new();
    for request in retained_requests.values() {
        let ik = prop(
            request.as_ref(),
            "interactionKind",
            "request.interactionKind",
        )?;
        let rid = prop(request.as_ref(), "requestId", "request.requestId")?;
        let key = format!(
            "{}:{}",
            to_string_js(ik.as_ref()),
            to_string_js(rid.as_ref())
        );
        let created = prop(request.as_ref(), "createdAt", "request.createdAt")?;
        let prev = oldest_retained_at.get(&str_val(&key));
        if prev.is_none() || js_lt(created.as_ref(), prev.unwrap_or(&Value::Null)) {
            oldest_retained_at.set(str_val(&key), Cow::Owned(created.as_ref().clone()));
        }
    }
    let mut latest_evidence = JsMap::new();
    for activity in iter(&activities, "activity", "activities")? {
        let Some(identity) = pending_interaction_identity(activity.as_ref())? else {
            continue;
        };
        let kind_v = prop(activity.as_ref(), "kind", "activity.kind")?;
        let is_request = PENDING_REQUEST_KINDS
            .iter()
            .any(|k| strict_eq(kind_v.as_ref(), &str_val(k)));
        let ik = prop(
            identity_obj(&identity),
            "interactionKind",
            "identity.interactionKind",
        )?;
        let rid = prop(identity_obj(&identity), "requestId", "identity.requestId")?;
        let lg = prop(
            identity_obj(&identity),
            "lifecycleGeneration",
            "identity.lifecycleGeneration",
        )?;
        let id_key = prop(identity_obj(&identity), "key", "identity.key")?;
        let oldest_key = format!(
            "{}:{}",
            to_string_js(ik.as_ref()),
            to_string_js(rid.as_ref())
        );
        let oldest_at = oldest_retained_at.get(&str_val(&oldest_key)).cloned();
        let keep = if !lg.is_null() {
            retained_requests.has(id_key.as_ref())
        } else if is_request {
            let id_created = prop(identity_obj(&identity), "createdAt", "identity.createdAt")?;
            retained_requests
                .get(id_key.as_ref())
                .is_some_and(|r| strict_eq(prop_opt(r, "createdAt").as_ref(), id_created.as_ref()))
        } else {
            let created = prop(identity_obj(&identity), "createdAt", "identity.createdAt")?;
            oldest_at.is_some_and(|o| js_le(&o, created.as_ref()))
        };
        if !keep {
            continue;
        }
        if is_request {
            let id = prop(activity.as_ref(), "id", "activity.id")?;
            retained_ids.add(id.as_ref().clone());
            continue;
        }
        let detail = as_record(prop(activity.as_ref(), "payload", "activity.payload")?.as_ref())
            .map(|p| prop_opt(p, "detail").into_owned());
        let detail_arg = detail.as_ref().filter(|d| kind(d) == K::Str);
        let stale = (strict_eq(
            kind_v.as_ref(),
            &str_val("provider.approval.respond.failed"),
        ) || strict_eq(
            kind_v.as_ref(),
            &str_val("provider.user-input.respond.failed"),
        )) && is_stale_pending_request_failure_detail(detail_arg)?;
        let key = format!(
            "{}:{}{}",
            to_string_js(id_key.as_ref()),
            to_string_js(kind_v.as_ref()),
            if stale { ":stale" } else { "" }
        );
        let prev = latest_evidence.get(&str_val(&key)).cloned();
        let created = prop(activity.as_ref(), "createdAt", "activity.createdAt")?;
        let keep_new = match &prev {
            None => true,
            Some(p) => js_ge(
                created.as_ref(),
                prop(p, "createdAt", "previous.createdAt")?.as_ref(),
            ),
        };
        if keep_new {
            latest_evidence.set(str_val(&key), clone_cow(&activity));
        }
    }
    for activity in latest_evidence.values() {
        let id = prop(activity.as_ref(), "id", "activity.id")?;
        retained_ids.add(id.as_ref().clone());
    }
    Ok(Value::Array(
        filter_call(
            &activities,
            "activities.filter",
            "activities.filter((activity) => retainedIds.has(activity.id))",
            &mut |activity: &Value, _| {
                let id = prop(activity, "id", "activity.id")?;
                Ok(retained_ids.has(id.as_ref()))
            },
        )?
        .iter()
        .map(|c| c.as_ref().clone())
        .collect(),
    ))
}

/// Port of `dedupeActivitiesByIdAfterAppend`.
pub fn dedupe_activities_by_id_after_append(
    activities: Value,
    previous_activities: Value,
    previous_activity_by_id: Value,
) -> Result<Value, String> {
    let acts_len = len_num(&activities, "activities.length").unwrap_or(f64::NAN);
    let prev_len_val = prop(&previous_activities, "length", "previousActivities.length");
    let _fallback = is_undef(&previous_activities)
        || is_undef(&previous_activity_by_id)
        || match &prev_len_val {
            Ok(l) => strict_eq(l.as_ref(), &num_val(0.0)),
            Err(_) => false,
        };
    if is_undef(&previous_activities) {
        return dedupe_activities_vec(&activities);
    }
    if is_undef(&previous_activity_by_id) {
        return dedupe_activities_vec(&activities);
    }
    let prev_len = to_number(prev_len_val?.as_ref());
    if strict_eq(&num_val(prev_len), &num_val(0.0)) || js_gt(&num_val(prev_len), &num_val(acts_len))
    {
        return dedupe_activities_vec(&activities);
    }
    let mut i = 0.0;
    while js_lt(&num_val(i), &num_val(prev_len)) {
        if !strict_eq(
            idx(&activities, i, "activities[index]")?.as_ref(),
            idx(&previous_activities, i, "previousActivities[index]")?.as_ref(),
        ) {
            return dedupe_activities_vec(&activities);
        }
        i += 1.0;
    }
    let mut appended_ids = JsSet::new();
    let mut i = prev_len;
    while js_lt(&num_val(i), &num_val(acts_len)) {
        let el = idx(&activities, i, "activities[index]")?;
        let id = prop(el.as_ref(), "id", "activities[index].id")?;
        let key = to_property_key(id.as_ref());
        if !is_undef(
            obj_key_get(&previous_activity_by_id, &key, "previousActivityById[id]")?.as_ref(),
        ) || appended_ids.has(id.as_ref())
        {
            return dedupe_activities_vec(&activities);
        }
        appended_ids.add(id.as_ref().clone());
        i += 1.0;
    }
    Ok(activities)
}

/// Port of `dedupeActivitiesById`.
pub fn dedupe_activities_by_id(activities: Value) -> Result<Value, String> {
    dedupe_activities_vec(&activities)
}

/// Port of `asActivityRecord`.
pub fn as_activity_record(value: Value) -> Result<Value, String> {
    Ok(match as_record(&value) {
        Some(v) => v.clone(),
        None => Value::Null,
    })
}

/// Port of `normalizeThreadErrorMessage`.
pub fn normalize_thread_error_message(message: Value) -> Result<Value, String> {
    normalize_thread_error_message_inner(&message)
}

/// Port of `normalizeThreadSession`.
pub fn normalize_thread_session(incoming: Value, previous: Value) -> Result<Value, String> {
    normalize_thread_session_inner(&incoming, &previous)
}

struct ThreadDerived<'a> {
    model_selection: Cow<'a, Value>,
    session: Cow<'a, Value>,
    messages: Cow<'a, Value>,
    proposed_plans: Cow<'a, Value>,
    latest_turn: Cow<'a, Value>,
    handoff: Cow<'a, Value>,
    snoozed_until: Value,
    snooze_reminder_at: Value,
    snooze_sequence: Cow<'a, Value>,
    usage_limit_recovery: Cow<'a, Value>,
    usage_limit_recovery_sequence: Cow<'a, Value>,
    claude_cache_review: Cow<'a, Value>,
    claude_cache_review_sequence: Cow<'a, Value>,
    last_known_pr: Cow<'a, Value>,
    pull_requests: Cow<'a, Value>,
    pinned_messages: Option<Cow<'a, Value>>,
    notes: Cow<'a, Value>,
    goal: Cow<'a, Value>,
    goal_started_at: Cow<'a, Value>,
    goal_paused_at: Cow<'a, Value>,
    goal_achievements: Option<Cow<'a, Value>>,
    turn_diff_summaries: Cow<'a, Value>,
    activities: Cow<'a, Value>,
    pending_interactions: Cow<'a, Value>,
    error: Cow<'a, Value>,
    last_visited_at: Cow<'a, Value>,
    resolved_latest_human: Cow<'a, Value>,
    resolved_latest_user: Cow<'a, Value>,
    resolved_approvals: Cow<'a, Value>,
    resolved_user_input: Cow<'a, Value>,
    resolved_actionable: Cow<'a, Value>,
    next_worktree_path: Cow<'a, Value>,
    next_working_directory: Value,
    next_associated_path: Value,
    next_associated_branch: Value,
    next_associated_ref: Value,
    resolved_branch: Cow<'a, Value>,
    resolved_create_completed: Value,
    pending_source_proposed_plan: Cow<'a, Value>,
    pending_turn_start_message_id: Cow<'a, Value>,
}

#[allow(clippy::too_many_lines)]
fn derive_thread_fields<'a>(
    incoming: &'a Value,
    previous: &'a Value,
    snapshot_sequence: &'a Value,
    options: &'a Value,
    shell_mode: bool,
) -> Result<ThreadDerived<'a>, String> {
    let model_selection = Cow::Owned(normalize_model_selection_inner(
        prop(incoming, "modelSelection", "incoming.modelSelection")?.as_ref(),
        prop_opt(previous, "modelSelection").as_ref(),
    )?);
    let session = Cow::Owned(normalize_thread_session_inner(
        prop(incoming, "session", "incoming.session")?.as_ref(),
        prop_opt(previous, "session").as_ref(),
    )?);
    let messages = Cow::Owned(normalize_chat_messages(
        prop(incoming, "messages", "incoming.messages")?.as_ref(),
        prop_opt(previous, "messages").as_ref(),
    )?);
    let proposed_plans = Cow::Owned(normalize_proposed_plans_inner(
        prop(incoming, "proposedPlans", "incoming.proposedPlans")?.as_ref(),
        prop_opt(previous, "proposedPlans").as_ref(),
    )?);
    let latest_turn: Cow<'a, Value> = Cow::Owned(
        normalize_latest_turn(
            prop(incoming, "latestTurn", "incoming.latestTurn")?.as_ref(),
            prop_opt(previous, "latestTurn").as_ref(),
        )?
        .into_owned(),
    );
    let prev_handoff = prop_opt(previous, "handoff");
    let inc_handoff = prop(incoming, "handoff", "incoming.handoff")?;
    let handoff = if truthy(prev_handoff.as_ref())
        && truthy(inc_handoff.as_ref())
        && deep_eq(
            prop(previous, "handoff", "previous.handoff")?.as_ref(),
            inc_handoff.as_ref(),
        ) {
        prev_handoff
    } else if is_nullish(inc_handoff.as_ref()) {
        Cow::Owned(Value::Null)
    } else {
        inc_handoff
    };
    let preserve_snooze = !is_undef(previous)
        && (if is_undef(snapshot_sequence) {
            js_lt(
                prop(incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
                {
                    let u = prop(previous, "updatedAt", "previous.updatedAt")?;
                    if is_nullish(u.as_ref()) {
                        prop(previous, "createdAt", "previous.createdAt")?
                    } else {
                        u
                    }
                }
                .as_ref(),
            )
        } else {
            js_lt(snapshot_sequence, &{
                let s = prop(previous, "snoozeSequence", "previous.snoozeSequence")?;
                if is_nullish(s.as_ref()) {
                    num_val(0.0)
                } else {
                    s.as_ref().clone()
                }
            })
        });
    let snoozed_until = {
        let v = if preserve_snooze {
            prop_opt(previous, "snoozedUntil")
        } else {
            prop(incoming, "snoozedUntil", "incoming.snoozedUntil")?
        };
        if is_nullish(v.as_ref()) {
            Value::Null
        } else {
            v.as_ref().clone()
        }
    };
    let snooze_reminder_at = {
        let v = if preserve_snooze {
            prop_opt(previous, "snoozeReminderAt")
        } else {
            prop(incoming, "snoozeReminderAt", "incoming.snoozeReminderAt")?
        };
        if is_nullish(v.as_ref()) {
            Value::Null
        } else {
            v.as_ref().clone()
        }
    };
    let snooze_sequence = if is_undef(snapshot_sequence) {
        prop_opt(previous, "snoozeSequence")
    } else {
        let ps = prop_opt(previous, "snoozeSequence");
        Cow::Owned(num_val(js_max(
            to_number(snapshot_sequence),
            if is_nullish(ps.as_ref()) {
                0.0
            } else {
                to_number(ps.as_ref())
            },
        )))
    };
    let (usage_limit_recovery, usage_limit_recovery_sequence) =
        normalize_usage_limit_recovery(incoming, previous, snapshot_sequence)?;
    let incoming_ccr = if !is_undef(snapshot_sequence)
        && js_lt(snapshot_sequence, &{
            let s = prop_opt(previous, "claudeCacheReviewSequence");
            if is_nullish(s.as_ref()) {
                num_val(0.0)
            } else {
                s.as_ref().clone()
            }
        }) {
        prop_opt(previous, "claudeCacheReview")
    } else {
        prop(incoming, "claudeCacheReview", "incoming.claudeCacheReview")?
    };
    let claude_cache_review_sequence = if is_undef(snapshot_sequence) {
        prop_opt(previous, "claudeCacheReviewSequence")
    } else {
        let ps = prop_opt(previous, "claudeCacheReviewSequence");
        Cow::Owned(num_val(js_max(
            to_number(snapshot_sequence),
            if is_nullish(ps.as_ref()) {
                0.0
            } else {
                to_number(ps.as_ref())
            },
        )))
    };
    let prev_ccr = prop_opt(previous, "claudeCacheReview");
    let claude_cache_review = if truthy(prev_ccr.as_ref())
        && deep_eq(
            prev_ccr.as_ref(),
            &if is_nullish(incoming_ccr.as_ref()) {
                Value::Null
            } else {
                incoming_ccr.as_ref().clone()
            },
        ) {
        prev_ccr
    } else if is_nullish(incoming_ccr.as_ref()) {
        Cow::Owned(Value::Null)
    } else {
        incoming_ccr
    };
    let prev_pr = prop_opt(previous, "lastKnownPr");
    let inc_pr = prop(incoming, "lastKnownPr", "incoming.lastKnownPr")?;
    let last_known_pr = if truthy(prev_pr.as_ref())
        && truthy(inc_pr.as_ref())
        && deep_eq(
            prop(previous, "lastKnownPr", "previous.lastKnownPr")?.as_ref(),
            inc_pr.as_ref(),
        ) {
        prev_pr
    } else if is_nullish(inc_pr.as_ref()) {
        Cow::Owned(Value::Null)
    } else {
        inc_pr
    };
    let prev_prs = prop_opt(previous, "pullRequests");
    let inc_prs = prop(incoming, "pullRequests", "incoming.pullRequests")?;
    let inc_prs_nn = if is_nullish(inc_prs.as_ref()) {
        Cow::Owned(Value::Array(vec![]))
    } else {
        inc_prs.clone()
    };
    let pull_requests =
        if truthy(prev_prs.as_ref()) && deep_eq(prev_prs.as_ref(), inc_prs_nn.as_ref()) {
            prev_prs
        } else {
            inc_prs_nn
        };
    let pinned_messages = if shell_mode {
        None
    } else {
        let prev_pm = prop_opt(previous, "pinnedMessages");
        let inc_pm = prop(incoming, "pinnedMessages", "incoming.pinnedMessages")?;
        let inc_pm_nn = if is_nullish(inc_pm.as_ref()) {
            Cow::Owned(Value::Null)
        } else {
            inc_pm.clone()
        };
        Some(
            if truthy(prev_pm.as_ref()) && deep_eq(prev_pm.as_ref(), inc_pm_nn.as_ref()) {
                prev_pm
            } else {
                inc_pm.clone()
            },
        )
    };
    let notes = prop(incoming, "notes", "incoming.notes")?;
    let goal = if shell_mode {
        let g = prop(incoming, "goal", "incoming.goal")?;
        if !is_undef(g.as_ref()) {
            g
        } else {
            prop_opt(previous, "goal")
        }
    } else {
        prop(incoming, "goal", "incoming.goal")?
    };
    let goal_started_at = if shell_mode {
        let g = prop(incoming, "goalStartedAt", "incoming.goalStartedAt")?;
        if !is_undef(g.as_ref()) {
            g
        } else {
            prop_opt(previous, "goalStartedAt")
        }
    } else {
        prop(incoming, "goalStartedAt", "incoming.goalStartedAt")?
    };
    let goal_paused_at = if shell_mode {
        let g = prop(incoming, "goalPausedAt", "incoming.goalPausedAt")?;
        if !is_undef(g.as_ref()) {
            g
        } else {
            prop_opt(previous, "goalPausedAt")
        }
    } else {
        prop(incoming, "goalPausedAt", "incoming.goalPausedAt")?
    };
    let goal_achievements = if shell_mode {
        None
    } else {
        let prev_ga = prop_opt(previous, "goalAchievements");
        let inc_ga = prop(incoming, "goalAchievements", "incoming.goalAchievements")?;
        let inc_ga_nn = if is_nullish(inc_ga.as_ref()) {
            Cow::Owned(Value::Null)
        } else {
            inc_ga.clone()
        };
        Some(
            if truthy(prev_ga.as_ref()) && deep_eq(prev_ga.as_ref(), inc_ga_nn.as_ref()) {
                prev_ga
            } else {
                inc_ga.clone()
            },
        )
    };
    let turn_diff_summaries = Cow::Owned(normalize_turn_diff_summaries(
        prop(incoming, "checkpoints", "incoming.checkpoints")?.as_ref(),
        prop_opt(previous, "turnDiffSummaries").as_ref(),
    )?);
    let activities = Cow::Owned(normalize_activities_inner(
        prop(incoming, "activities", "incoming.activities")?.as_ref(),
        prop_opt(previous, "activities").as_ref(),
    )?);
    let has_pi = object_has_own(
        incoming,
        "pendingInteractions",
        "Object.hasOwn(incoming, \"pendingInteractions\")",
    )?;
    let incoming_pending = if has_pi {
        let v = prop(
            incoming,
            "pendingInteractions",
            "incoming.pendingInteractions",
        )?;
        if is_nullish(v.as_ref()) {
            Cow::Owned(Value::Array(vec![]))
        } else {
            v
        }
    } else {
        prop_opt(previous, "pendingInteractions")
    };
    let prev_pi = prop_opt(previous, "pendingInteractions");
    let pending_interactions = if truthy(prev_pi.as_ref()) {
        let candidate = if is_undef(incoming_pending.as_ref()) {
            Cow::Owned(Value::Array(vec![]))
        } else {
            incoming_pending.clone()
        };
        if deep_eq(prev_pi.as_ref(), candidate.as_ref()) {
            prev_pi
        } else if is_undef(incoming_pending.as_ref()) {
            Cow::Borrowed(undef())
        } else {
            let spread = spread_iter(incoming_pending.as_ref())?;
            Cow::Owned(Value::Array(
                spread.iter().map(|c| c.as_ref().clone()).collect(),
            ))
        }
    } else if is_undef(incoming_pending.as_ref()) {
        Cow::Borrowed(undef())
    } else {
        let spread = spread_iter(incoming_pending.as_ref())?;
        Cow::Owned(Value::Array(
            spread.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    };
    let session_for_error = prop(incoming, "session", "incoming.session")?;
    let error = Cow::Owned(normalize_thread_error_message_inner(
        prop_opt(session_for_error.as_ref(), "lastError").as_ref(),
    )?);
    let last_visited_at = {
        let plv = prop_opt(previous, "lastVisitedAt");
        if !is_nullish(plv.as_ref()) {
            plv
        } else {
            Cow::Owned(resolve_initial_last_visited_at(incoming, options)?)
        }
    };
    let resolved_latest_human = prop(
        incoming,
        "latestHumanMessageAt",
        "incoming.latestHumanMessageAt",
    )?;
    let resolved_latest_user = if object_has_own(
        incoming,
        "latestUserMessageAt",
        "Object.hasOwn(incoming, \"latestUserMessageAt\")",
    )? && !is_undef(
        prop(
            incoming,
            "latestUserMessageAt",
            "incoming.latestUserMessageAt",
        )?
        .as_ref(),
    ) {
        let v = prop(
            incoming,
            "latestUserMessageAt",
            "incoming.latestUserMessageAt",
        )?;
        if is_nullish(v.as_ref()) {
            Cow::Owned(Value::Null)
        } else {
            v
        }
    } else {
        Cow::Borrowed(undef())
    };
    let bool_or_undef = |name: &str| -> Result<Cow<'a, Value>, String> {
        let v = prop(incoming, name, &format!("incoming.{name}"))?;
        Ok(if kind(v.as_ref()) == K::Bool {
            v
        } else {
            Cow::Borrowed(undef())
        })
    };
    let resolved_approvals = bool_or_undef("hasPendingApprovals")?;
    let resolved_user_input = bool_or_undef("hasPendingUserInput")?;
    let resolved_actionable = bool_or_undef("hasActionableProposedPlan")?;
    let next_worktree_path = prop(incoming, "worktreePath", "incoming.worktreePath")?;
    let next_working_directory = {
        let v = prop(incoming, "workingDirectory", "incoming.workingDirectory")?;
        if is_nullish(v.as_ref()) {
            Value::Null
        } else {
            v.as_ref().clone()
        }
    };
    let next_associated_path = {
        let v = prop(
            incoming,
            "associatedWorktreePath",
            "incoming.associatedWorktreePath",
        )?;
        if is_nullish(v.as_ref()) {
            Value::Null
        } else {
            v.as_ref().clone()
        }
    };
    let next_associated_branch = {
        let v = prop(
            incoming,
            "associatedWorktreeBranch",
            "incoming.associatedWorktreeBranch",
        )?;
        if is_nullish(v.as_ref()) {
            Value::Null
        } else {
            v.as_ref().clone()
        }
    };
    let next_associated_ref = {
        let v = prop(
            incoming,
            "associatedWorktreeRef",
            "incoming.associatedWorktreeRef",
        )?;
        if is_nullish(v.as_ref()) {
            Value::Null
        } else {
            v.as_ref().clone()
        }
    };
    let prev_branch = prop_opt(previous, "branch");
    let cur_branch: Cow<'a, Value> = if is_nullish(prev_branch.as_ref()) {
        Cow::Owned(Value::Null)
    } else {
        prev_branch
    };
    let resolved_branch: Cow<'a, Value> = Cow::Owned(resolve_thread_branch_regression_guard(
        cur_branch.as_ref(),
        prop(incoming, "branch", "incoming.branch")?.as_ref(),
    )?);
    let resolved_create_completed = resolve_create_merge_internal(
        cur_branch.as_ref(),
        resolved_branch.as_ref(),
        &{
            let p = prop_opt(previous, "worktreePath");
            if is_nullish(p.as_ref()) {
                Value::Null
            } else {
                p.into_owned()
            }
        },
        next_worktree_path.as_ref(),
        prop_opt(previous, "associatedWorktreePath").as_ref(),
        &next_associated_path,
        prop_opt(previous, "associatedWorktreeBranch").as_ref(),
        &next_associated_branch,
        prop_opt(previous, "associatedWorktreeRef").as_ref(),
        &next_associated_ref,
        prop_opt(previous, "createBranchFlowCompleted").as_ref(),
        prop(
            incoming,
            "createBranchFlowCompleted",
            "incoming.createBranchFlowCompleted",
        )?
        .as_ref(),
    )?;
    let pending_source_proposed_plan: Cow<'a, Value> = {
        let lt_spp = prop_opt(latest_turn.as_ref(), "sourceProposedPlan");
        if !is_nullish(lt_spp.as_ref()) {
            Cow::Owned(lt_spp.into_owned())
        } else {
            let sess = prop(incoming, "session", "incoming.session")?;
            let status = prop_opt(sess.as_ref(), "status");
            if strict_eq(status.as_ref(), &str_val("running")) {
                prop_opt(previous, "pendingSourceProposedPlan")
            } else {
                Cow::Borrowed(undef())
            }
        }
    };
    let pending_turn_start_message_id = {
        let status = prop_opt(claude_cache_review.as_ref(), "status");
        if strict_eq(status.as_ref(), &str_val("compacting")) {
            Cow::Owned(Value::Null)
        } else {
            prop_opt(previous, "pendingTurnStartMessageId")
        }
    };
    Ok(ThreadDerived {
        model_selection,
        session,
        messages,
        proposed_plans,
        latest_turn,
        handoff,
        snoozed_until,
        snooze_reminder_at,
        snooze_sequence,
        usage_limit_recovery,
        usage_limit_recovery_sequence,
        claude_cache_review,
        claude_cache_review_sequence,
        last_known_pr,
        pull_requests,
        pinned_messages,
        notes,
        goal,
        goal_started_at,
        goal_paused_at,
        goal_achievements,
        turn_diff_summaries,
        activities,
        pending_interactions,
        error,
        last_visited_at,
        resolved_latest_human,
        resolved_latest_user,
        resolved_approvals,
        resolved_user_input,
        resolved_actionable,
        next_worktree_path,
        next_working_directory,
        next_associated_path,
        next_associated_branch,
        next_associated_ref,
        resolved_branch,
        resolved_create_completed,
        pending_source_proposed_plan,
        pending_turn_start_message_id,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_create_merge_internal(
    current_branch: &Value,
    next_branch: &Value,
    current_wt: &Value,
    next_wt: &Value,
    current_awp: &Value,
    next_awp: &Value,
    current_awb: &Value,
    next_awb: &Value,
    current_awr: &Value,
    next_awr: &Value,
    current_completed: &Value,
    next_completed: &Value,
) -> Result<Value, String> {
    let or_null = |v: &Value| -> Value {
        if is_nullish(v) {
            Value::Null
        } else {
            v.clone()
        }
    };
    let context_changed = !strict_eq(current_branch, next_branch)
        || !strict_eq(current_wt, next_wt)
        || !strict_eq(&or_null(current_awp), &or_null(next_awp))
        || !strict_eq(&or_null(current_awb), &or_null(next_awb))
        || !strict_eq(&or_null(current_awr), &or_null(next_awr));
    if context_changed {
        return Ok(or_null_false(next_completed));
    }
    if is_undef(next_completed) {
        return Ok(or_null_false(current_completed));
    }
    if truthy(&or_null_false(current_completed)) && !truthy(next_completed) {
        return Ok(Value::Bool(true));
    }
    Ok(next_completed.clone())
}

fn or_null_false(v: &Value) -> Value {
    if is_nullish(v) {
        Value::Bool(false)
    } else {
        v.clone()
    }
}

/// Port of `normalizeThreadFromReadModel`.
pub fn normalize_thread_from_read_model(
    incoming: Value,
    previous: Value,
    snapshot_sequence: Value,
    options: Value,
) -> Result<Value, String> {
    let options_val = if is_undef(&options) {
        Cow::Owned(Value::Object(Map::new()))
    } else {
        Cow::Borrowed(&options)
    };
    let d = derive_thread_fields(
        &incoming,
        &previous,
        &snapshot_sequence,
        options_val.as_ref(),
        false,
    )?;
    if truthy(&previous) {
        let mut all = strict_eq(
            prop(&previous, "projectId", "previous.projectId")?.as_ref(),
            prop(&incoming, "projectId", "incoming.projectId")?.as_ref(),
        );
        if all {
            all = strict_eq(
                prop(&previous, "title", "previous.title")?.as_ref(),
                prop(&incoming, "title", "incoming.title")?.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "modelSelection", "previous.modelSelection")?.as_ref(),
                d.model_selection.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "runtimeMode", "previous.runtimeMode")?.as_ref(),
                prop(&incoming, "runtimeMode", "incoming.runtimeMode")?.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "interactionMode", "previous.interactionMode")?.as_ref(),
                prop(&incoming, "interactionMode", "incoming.interactionMode")?.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "session", "previous.session")?.as_ref(),
                &d.session,
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "messages", "previous.messages")?.as_ref(),
                d.messages.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "proposedPlans", "previous.proposedPlans")?.as_ref(),
                d.proposed_plans.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "error", "previous.error")?.as_ref(),
                d.error.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "createdAt", "previous.createdAt")?.as_ref(),
                prop(&incoming, "createdAt", "incoming.createdAt")?.as_ref(),
            );
        }
        if all {
            all = {
                let a = prop(&previous, "archivedAt", "previous.archivedAt")?;
                let b = prop(&incoming, "archivedAt", "incoming.archivedAt")?;
                strict_eq(&nn(a.as_ref()), &nn(b.as_ref()))
            };
        }
        if all {
            all = {
                let a = prop(&previous, "settledAt", "previous.settledAt")?;
                let b = prop(&incoming, "settledAt", "incoming.settledAt")?;
                strict_eq(&nn(a.as_ref()), &nn(b.as_ref()))
            };
        }
        if all {
            let a = prop(&previous, "snoozedUntil", "previous.snoozedUntil")?;
            all = strict_eq(&nn(a.as_ref()), &d.snoozed_until);
        }
        if all {
            let a = prop(&previous, "snoozeReminderAt", "previous.snoozeReminderAt")?;
            all = strict_eq(&nn(a.as_ref()), &d.snooze_reminder_at);
        }
        if all {
            all = strict_eq(
                prop(&previous, "snoozeSequence", "previous.snoozeSequence")?.as_ref(),
                d.snooze_sequence.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "updatedAt", "previous.updatedAt")?.as_ref(),
                prop(&incoming, "updatedAt", "incoming.updatedAt")?.as_ref(),
            );
        }
        if all {
            let a = prop(&previous, "isPinned", "previous.isPinned")?;
            let b = prop(&incoming, "isPinned", "incoming.isPinned")?;
            all = strict_eq(&nnf(a.as_ref()), &nnf(b.as_ref()));
        }
        if all {
            all = strict_eq(
                prop(&previous, "latestTurn", "previous.latestTurn")?.as_ref(),
                &d.latest_turn,
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "pendingSourceProposedPlan",
                    "previous.pendingSourceProposedPlan",
                )?
                .as_ref(),
                d.pending_source_proposed_plan.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "pendingTurnStartMessageId",
                    "previous.pendingTurnStartMessageId",
                )?
                .as_ref(),
                d.pending_turn_start_message_id.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "lastVisitedAt", "previous.lastVisitedAt")?.as_ref(),
                d.last_visited_at.as_ref(),
            );
        }
        if all {
            for f in [
                "parentThreadId",
                "creationSource",
                "sourceThreadId",
                "subagentAgentId",
                "subagentNickname",
                "subagentRole",
                "forkSourceThreadId",
                "sidechatSourceThreadId",
            ] {
                let a = prop(&previous, f, &format!("previous.{f}"))?;
                let b = prop(&incoming, f, &format!("incoming.{f}"))?;
                if !strict_eq(&nn(a.as_ref()), &nn(b.as_ref())) {
                    all = false;
                    break;
                }
            }
        }
        if all {
            all = strict_eq(prop(&previous, "envMode", "previous.envMode")?.as_ref(), &{
                let e = prop(&incoming, "envMode", "incoming.envMode")?;
                if is_nullish(e.as_ref()) {
                    str_val("local")
                } else {
                    e.as_ref().clone()
                }
            });
        }
        if all {
            all = strict_eq(
                prop(&previous, "branch", "previous.branch")?.as_ref(),
                d.resolved_branch.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "worktreePath", "previous.worktreePath")?.as_ref(),
                d.next_worktree_path.as_ref(),
            );
        }
        if all {
            let a = prop(&previous, "workingDirectory", "previous.workingDirectory")?;
            all = strict_eq(&nn(a.as_ref()), &d.next_working_directory);
        }
        if all {
            let a = prop(
                &previous,
                "associatedWorktreePath",
                "previous.associatedWorktreePath",
            )?;
            all = strict_eq(&nn(a.as_ref()), &d.next_associated_path);
        }
        if all {
            let a = prop(
                &previous,
                "associatedWorktreeBranch",
                "previous.associatedWorktreeBranch",
            )?;
            all = strict_eq(&nn(a.as_ref()), &d.next_associated_branch);
        }
        if all {
            let a = prop(
                &previous,
                "associatedWorktreeRef",
                "previous.associatedWorktreeRef",
            )?;
            all = strict_eq(&nn(a.as_ref()), &d.next_associated_ref);
        }
        if all {
            let a = prop(
                &previous,
                "createBranchFlowCompleted",
                "previous.createBranchFlowCompleted",
            )?;
            all = strict_eq(&nnf(a.as_ref()), &d.resolved_create_completed);
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "latestUserMessageAt",
                    "previous.latestUserMessageAt",
                )?
                .as_ref(),
                d.resolved_latest_user.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "latestHumanMessageAt",
                    "previous.latestHumanMessageAt",
                )?
                .as_ref(),
                d.resolved_latest_human.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "hasPendingApprovals",
                    "previous.hasPendingApprovals",
                )?
                .as_ref(),
                d.resolved_approvals.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "hasPendingUserInput",
                    "previous.hasPendingUserInput",
                )?
                .as_ref(),
                d.resolved_user_input.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "hasActionableProposedPlan",
                    "previous.hasActionableProposedPlan",
                )?
                .as_ref(),
                d.resolved_actionable.as_ref(),
            );
        }
        if all {
            all = {
                let a = prop(&previous, "sidechatContext", "previous.sidechatContext")?;
                let b = prop(&incoming, "sidechatContext", "incoming.sidechatContext")?;
                strict_eq(
                    &nn(prop_opt(a.as_ref(), "url").as_ref()),
                    &nn(prop_opt(b.as_ref(), "url").as_ref()),
                )
            };
        }
        if all {
            for f in ["sidechatLastActivityAt", "sidechatExpiredAt"] {
                let a = prop(&previous, f, &format!("previous.{f}"))?;
                let b = prop(&incoming, f, &format!("incoming.{f}"))?;
                if !strict_eq(&nn(a.as_ref()), &nn(b.as_ref())) {
                    all = false;
                    break;
                }
            }
        }
        if all {
            let a = prop(&previous, "lastKnownPr", "previous.lastKnownPr")?;
            all = deep_eq(&nn(a.as_ref()), d.last_known_pr.as_ref());
        }
        if all {
            let a = prop(&previous, "handoff", "previous.handoff")?;
            all = strict_eq(&nn(a.as_ref()), d.handoff.as_ref());
        }
        if all {
            let a = prop(&previous, "isProjectImport", "previous.isProjectImport")?;
            let b = prop(&incoming, "isProjectImport", "incoming.isProjectImport")?;
            all = strict_eq(&nnf(a.as_ref()), &nnf(b.as_ref()));
        }
        if all {
            let a = prop(&previous, "claudeCacheReview", "previous.claudeCacheReview")?;
            all = strict_eq(&nn(a.as_ref()), d.claude_cache_review.as_ref());
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "claudeCacheReviewSequence",
                    "previous.claudeCacheReviewSequence",
                )?
                .as_ref(),
                d.claude_cache_review_sequence.as_ref(),
            );
        }
        if all {
            let a = prop(
                &previous,
                "usageLimitRecovery",
                "previous.usageLimitRecovery",
            )?;
            all = strict_eq(&nn(a.as_ref()), d.usage_limit_recovery.as_ref());
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "usageLimitRecoverySequence",
                    "previous.usageLimitRecoverySequence",
                )?
                .as_ref(),
                d.usage_limit_recovery_sequence.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "pinnedMessages", "previous.pinnedMessages")?.as_ref(),
                &d.pinned_messages
                    .as_ref()
                    .map(|c| c.as_ref().clone())
                    .unwrap_or_else(undefined),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "notes", "previous.notes")?.as_ref(),
                d.notes.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "goal", "previous.goal")?.as_ref(),
                d.goal.as_ref(),
            );
        }
        if all {
            let a = prop(&previous, "goalStartedAt", "previous.goalStartedAt")?;
            all = strict_eq(&nn(a.as_ref()), &nn(d.goal_started_at.as_ref()));
        }
        if all {
            let a = prop(&previous, "goalPausedAt", "previous.goalPausedAt")?;
            all = strict_eq(&nn(a.as_ref()), &nn(d.goal_paused_at.as_ref()));
        }
        if all {
            all = strict_eq(
                prop(&previous, "goalAchievements", "previous.goalAchievements")?.as_ref(),
                &d.goal_achievements
                    .as_ref()
                    .map(|c| c.as_ref().clone())
                    .unwrap_or_else(undefined),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "turnDiffSummaries", "previous.turnDiffSummaries")?.as_ref(),
                d.turn_diff_summaries.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(&previous, "activities", "previous.activities")?.as_ref(),
                d.activities.as_ref(),
            );
        }
        if all {
            all = strict_eq(
                prop(
                    &previous,
                    "pendingInteractions",
                    "previous.pendingInteractions",
                )?
                .as_ref(),
                d.pending_interactions.as_ref(),
            );
        }
        if all {
            return Ok(previous);
        }
    }
    let mut o = Map::new();
    o.insert(
        "id".into(),
        prop(&incoming, "id", "incoming.id")?.as_ref().clone(),
    );
    o.insert("codexThreadId".into(), Value::Null);
    o.insert(
        "projectId".into(),
        prop(&incoming, "projectId", "incoming.projectId")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "title".into(),
        prop(&incoming, "title", "incoming.title")?.as_ref().clone(),
    );
    o.insert("modelSelection".into(), d.model_selection.as_ref().clone());
    o.insert(
        "runtimeMode".into(),
        prop(&incoming, "runtimeMode", "incoming.runtimeMode")?
            .as_ref()
            .clone(),
    );
    o.insert(
        "interactionMode".into(),
        prop(&incoming, "interactionMode", "incoming.interactionMode")?
            .as_ref()
            .clone(),
    );
    o.insert("session".into(), d.session.as_ref().clone());
    o.insert("messages".into(), d.messages.as_ref().clone());
    o.insert("proposedPlans".into(), d.proposed_plans.as_ref().clone());
    o.insert("error".into(), d.error.as_ref().clone());
    o.insert(
        "createdAt".into(),
        prop(&incoming, "createdAt", "incoming.createdAt")?
            .as_ref()
            .clone(),
    );
    {
        let v = prop(&incoming, "archivedAt", "incoming.archivedAt")?;
        o.insert("archivedAt".into(), nn(v.as_ref()));
    }
    {
        let v = prop(&incoming, "settledAt", "incoming.settledAt")?;
        o.insert("settledAt".into(), nn(v.as_ref()));
    }
    o.insert("snoozedUntil".into(), d.snoozed_until);
    o.insert("snoozeReminderAt".into(), d.snooze_reminder_at);
    if !is_undef(d.snooze_sequence.as_ref()) {
        o.insert("snoozeSequence".into(), d.snooze_sequence.as_ref().clone());
    }
    o.insert(
        "updatedAt".into(),
        prop(&incoming, "updatedAt", "incoming.updatedAt")?
            .as_ref()
            .clone(),
    );
    {
        let v = prop(&incoming, "isPinned", "incoming.isPinned")?;
        o.insert("isPinned".into(), nnf(v.as_ref()));
    }
    o.insert("latestTurn".into(), d.latest_turn.as_ref().clone());
    if truthy(d.pending_source_proposed_plan.as_ref()) {
        o.insert(
            "pendingSourceProposedPlan".into(),
            d.pending_source_proposed_plan.as_ref().clone(),
        );
    }
    if !is_undef(d.pending_turn_start_message_id.as_ref()) {
        o.insert(
            "pendingTurnStartMessageId".into(),
            d.pending_turn_start_message_id.as_ref().clone(),
        );
    }
    o.insert("lastVisitedAt".into(), d.last_visited_at.as_ref().clone());
    for f in [
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
    ] {
        let v = prop(&incoming, f, &format!("incoming.{f}"))?;
        o.insert(f.into(), nn(v.as_ref()));
    }
    {
        let v = prop(&incoming, "envMode", "incoming.envMode")?;
        o.insert(
            "envMode".into(),
            if is_nullish(v.as_ref()) {
                str_val("local")
            } else {
                v.as_ref().clone()
            },
        );
    }
    o.insert("branch".into(), d.resolved_branch.as_ref().clone());
    o.insert("worktreePath".into(), d.next_worktree_path.as_ref().clone());
    o.insert("workingDirectory".into(), d.next_working_directory);
    o.insert("associatedWorktreePath".into(), d.next_associated_path);
    o.insert("associatedWorktreeBranch".into(), d.next_associated_branch);
    o.insert("associatedWorktreeRef".into(), d.next_associated_ref);
    o.insert(
        "createBranchFlowCompleted".into(),
        d.resolved_create_completed,
    );
    for f in ["forkSourceThreadId", "sidechatSourceThreadId"] {
        let v = prop(&incoming, f, &format!("incoming.{f}"))?;
        o.insert(f.into(), nn(v.as_ref()));
    }
    {
        let v = prop(&incoming, "sidechatContext", "incoming.sidechatContext")?;
        o.insert("sidechatContext".into(), nn(v.as_ref()));
        let v = prop(
            &incoming,
            "sidechatLastActivityAt",
            "incoming.sidechatLastActivityAt",
        )?;
        o.insert("sidechatLastActivityAt".into(), nn(v.as_ref()));
        let v = prop(&incoming, "sidechatExpiredAt", "incoming.sidechatExpiredAt")?;
        o.insert("sidechatExpiredAt".into(), nn(v.as_ref()));
    }
    o.insert("lastKnownPr".into(), d.last_known_pr.as_ref().clone());
    o.insert("pullRequests".into(), d.pull_requests.as_ref().clone());
    o.insert("handoff".into(), d.handoff.as_ref().clone());
    {
        let v = prop(&incoming, "isProjectImport", "incoming.isProjectImport")?;
        if truthy(v.as_ref()) {
            o.insert("isProjectImport".into(), Value::Bool(true));
        }
    }
    o.insert(
        "usageLimitRecovery".into(),
        d.usage_limit_recovery.as_ref().clone(),
    );
    if !is_undef(d.usage_limit_recovery_sequence.as_ref()) {
        o.insert(
            "usageLimitRecoverySequence".into(),
            d.usage_limit_recovery_sequence.as_ref().clone(),
        );
    }
    o.insert(
        "claudeCacheReview".into(),
        d.claude_cache_review.as_ref().clone(),
    );
    if !is_undef(d.claude_cache_review_sequence.as_ref()) {
        o.insert(
            "claudeCacheReviewSequence".into(),
            d.claude_cache_review_sequence.as_ref().clone(),
        );
    }
    if let Some(pm) = &d.pinned_messages
        && !is_undef(pm.as_ref())
    {
        o.insert("pinnedMessages".into(), pm.as_ref().clone());
    }
    if !is_undef(d.notes.as_ref()) {
        o.insert("notes".into(), d.notes.as_ref().clone());
    }
    if !is_undef(d.goal.as_ref()) {
        o.insert("goal".into(), d.goal.as_ref().clone());
    }
    if !is_undef(d.goal_started_at.as_ref()) {
        o.insert("goalStartedAt".into(), d.goal_started_at.as_ref().clone());
    }
    if !is_undef(d.goal_paused_at.as_ref()) {
        o.insert("goalPausedAt".into(), d.goal_paused_at.as_ref().clone());
    }
    if let Some(ga) = &d.goal_achievements
        && !is_undef(ga.as_ref())
    {
        o.insert("goalAchievements".into(), ga.as_ref().clone());
    }
    if !is_undef(d.resolved_latest_human.as_ref()) {
        o.insert(
            "latestHumanMessageAt".into(),
            d.resolved_latest_human.as_ref().clone(),
        );
    }
    if !is_undef(d.resolved_latest_user.as_ref()) {
        o.insert(
            "latestUserMessageAt".into(),
            d.resolved_latest_user.as_ref().clone(),
        );
    }
    if !is_undef(d.resolved_approvals.as_ref()) {
        o.insert(
            "hasPendingApprovals".into(),
            d.resolved_approvals.as_ref().clone(),
        );
    }
    if !is_undef(d.resolved_user_input.as_ref()) {
        o.insert(
            "hasPendingUserInput".into(),
            d.resolved_user_input.as_ref().clone(),
        );
    }
    if !is_undef(d.resolved_actionable.as_ref()) {
        o.insert(
            "hasActionableProposedPlan".into(),
            d.resolved_actionable.as_ref().clone(),
        );
    }
    {
        let v = prop_opt(&previous, "pendingBackgroundTaskCount");
        if !is_undef(v.as_ref()) {
            o.insert("pendingBackgroundTaskCount".into(), v.as_ref().clone());
        }
        let v = prop_opt(&previous, "latestReplyPreview");
        if !is_undef(v.as_ref()) {
            o.insert("latestReplyPreview".into(), v.as_ref().clone());
        }
    }
    o.insert(
        "turnDiffSummaries".into(),
        d.turn_diff_summaries.as_ref().clone(),
    );
    o.insert("activities".into(), d.activities.as_ref().clone());
    if !is_undef(d.pending_interactions.as_ref()) {
        o.insert(
            "pendingInteractions".into(),
            d.pending_interactions.as_ref().clone(),
        );
    }
    Ok(Value::Object(o))
}

fn nn(v: &Value) -> Value {
    if is_nullish(v) {
        Value::Null
    } else {
        v.clone()
    }
}

fn nnf(v: &Value) -> Value {
    if is_nullish(v) {
        Value::Bool(false)
    } else {
        v.clone()
    }
}

/// Port of `normalizeThreadShellSnapshot`.
pub fn normalize_thread_shell_snapshot(
    incoming: Value,
    previous: Value,
    snapshot_sequence: Value,
    options: Value,
) -> Result<Value, String> {
    let options_val = if is_undef(&options) {
        Cow::Owned(Value::Object(Map::new()))
    } else {
        Cow::Borrowed(&options)
    };
    let d = derive_thread_fields(
        &incoming,
        &previous,
        &snapshot_sequence,
        options_val.as_ref(),
        true,
    )?;
    let mut shell = Map::new();
    shell.insert(
        "id".into(),
        prop(&incoming, "id", "incoming.id")?.as_ref().clone(),
    );
    {
        let v = prop_opt(&previous, "codexThreadId");
        shell.insert("codexThreadId".into(), nn(v.as_ref()));
    }
    shell.insert(
        "projectId".into(),
        prop(&incoming, "projectId", "incoming.projectId")?
            .as_ref()
            .clone(),
    );
    shell.insert(
        "title".into(),
        prop(&incoming, "title", "incoming.title")?.as_ref().clone(),
    );
    shell.insert("modelSelection".into(), d.model_selection.as_ref().clone());
    shell.insert(
        "runtimeMode".into(),
        prop(&incoming, "runtimeMode", "incoming.runtimeMode")?
            .as_ref()
            .clone(),
    );
    shell.insert(
        "interactionMode".into(),
        prop(&incoming, "interactionMode", "incoming.interactionMode")?
            .as_ref()
            .clone(),
    );
    shell.insert("error".into(), d.error.as_ref().clone());
    shell.insert(
        "createdAt".into(),
        prop(&incoming, "createdAt", "incoming.createdAt")?
            .as_ref()
            .clone(),
    );
    {
        let v = prop(&incoming, "archivedAt", "incoming.archivedAt")?;
        shell.insert("archivedAt".into(), nn(v.as_ref()));
        let v = prop(&incoming, "settledAt", "incoming.settledAt")?;
        shell.insert("settledAt".into(), nn(v.as_ref()));
    }
    shell.insert("snoozedUntil".into(), d.snoozed_until.clone());
    shell.insert("snoozeReminderAt".into(), d.snooze_reminder_at.clone());
    if !is_undef(d.snooze_sequence.as_ref()) {
        shell.insert("snoozeSequence".into(), d.snooze_sequence.as_ref().clone());
    }
    shell.insert(
        "updatedAt".into(),
        prop(&incoming, "updatedAt", "incoming.updatedAt")?
            .as_ref()
            .clone(),
    );
    {
        let v = prop(&incoming, "isPinned", "incoming.isPinned")?;
        shell.insert("isPinned".into(), nnf(v.as_ref()));
        let v = prop(&incoming, "envMode", "incoming.envMode")?;
        shell.insert(
            "envMode".into(),
            if is_nullish(v.as_ref()) {
                str_val("local")
            } else {
                v.as_ref().clone()
            },
        );
    }
    shell.insert("branch".into(), d.resolved_branch.as_ref().clone());
    shell.insert("worktreePath".into(), d.next_worktree_path.as_ref().clone());
    shell.insert("workingDirectory".into(), d.next_working_directory.clone());
    shell.insert(
        "associatedWorktreePath".into(),
        d.next_associated_path.clone(),
    );
    shell.insert(
        "associatedWorktreeBranch".into(),
        d.next_associated_branch.clone(),
    );
    shell.insert(
        "associatedWorktreeRef".into(),
        d.next_associated_ref.clone(),
    );
    shell.insert(
        "createBranchFlowCompleted".into(),
        d.resolved_create_completed.clone(),
    );
    for f in [
        "parentThreadId",
        "creationSource",
        "sourceThreadId",
        "subagentAgentId",
        "subagentNickname",
        "subagentRole",
        "forkSourceThreadId",
        "sidechatSourceThreadId",
    ] {
        let v = prop(&incoming, f, &format!("incoming.{f}"))?;
        shell.insert(f.into(), nn(v.as_ref()));
    }
    {
        let v = prop(&incoming, "sidechatContext", "incoming.sidechatContext")?;
        shell.insert("sidechatContext".into(), nn(v.as_ref()));
        let v = prop(
            &incoming,
            "sidechatLastActivityAt",
            "incoming.sidechatLastActivityAt",
        )?;
        shell.insert("sidechatLastActivityAt".into(), nn(v.as_ref()));
        let v = prop(&incoming, "sidechatExpiredAt", "incoming.sidechatExpiredAt")?;
        shell.insert("sidechatExpiredAt".into(), nn(v.as_ref()));
    }
    shell.insert("lastKnownPr".into(), d.last_known_pr.as_ref().clone());
    shell.insert("pullRequests".into(), d.pull_requests.as_ref().clone());
    shell.insert("handoff".into(), d.handoff.as_ref().clone());
    {
        let v = prop(&incoming, "isProjectImport", "incoming.isProjectImport")?;
        if truthy(v.as_ref()) {
            shell.insert("isProjectImport".into(), Value::Bool(true));
        }
    }
    shell.insert(
        "usageLimitRecovery".into(),
        d.usage_limit_recovery.as_ref().clone(),
    );
    if !is_undef(d.usage_limit_recovery_sequence.as_ref()) {
        shell.insert(
            "usageLimitRecoverySequence".into(),
            d.usage_limit_recovery_sequence.as_ref().clone(),
        );
    }
    shell.insert(
        "claudeCacheReview".into(),
        d.claude_cache_review.as_ref().clone(),
    );
    if !is_undef(d.claude_cache_review_sequence.as_ref()) {
        shell.insert(
            "claudeCacheReviewSequence".into(),
            d.claude_cache_review_sequence.as_ref().clone(),
        );
    }
    {
        let v = prop_opt(&previous, "pinnedMessages");
        if !is_undef(v.as_ref()) {
            shell.insert("pinnedMessages".into(), v.as_ref().clone());
        }
        let v = prop_opt(&previous, "notes");
        if !is_undef(v.as_ref()) {
            shell.insert("notes".into(), v.as_ref().clone());
        }
        let v = prop_opt(&previous, "goalAchievements");
        if !is_undef(v.as_ref()) {
            shell.insert("goalAchievements".into(), v.as_ref().clone());
        }
    }
    if !is_undef(d.goal.as_ref()) {
        shell.insert("goal".into(), d.goal.as_ref().clone());
    }
    if !is_undef(d.goal_started_at.as_ref()) {
        shell.insert("goalStartedAt".into(), d.goal_started_at.as_ref().clone());
    }
    if !is_undef(d.goal_paused_at.as_ref()) {
        shell.insert("goalPausedAt".into(), d.goal_paused_at.as_ref().clone());
    }
    {
        let v = prop(
            &incoming,
            "latestHumanMessageAt",
            "incoming.latestHumanMessageAt",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("latestHumanMessageAt".into(), v.as_ref().clone());
        }
        let v = prop(
            &incoming,
            "latestUserMessageAt",
            "incoming.latestUserMessageAt",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("latestUserMessageAt".into(), nn(v.as_ref()));
        }
        let v = prop(
            &incoming,
            "hasPendingApprovals",
            "incoming.hasPendingApprovals",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("hasPendingApprovals".into(), v.as_ref().clone());
        }
        let v = prop(
            &incoming,
            "hasPendingUserInput",
            "incoming.hasPendingUserInput",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("hasPendingUserInput".into(), v.as_ref().clone());
        }
        let v = prop(
            &incoming,
            "hasActionableProposedPlan",
            "incoming.hasActionableProposedPlan",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("hasActionableProposedPlan".into(), v.as_ref().clone());
        }
        let v = prop(
            &incoming,
            "pendingBackgroundTaskCount",
            "incoming.pendingBackgroundTaskCount",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("pendingBackgroundTaskCount".into(), v.as_ref().clone());
        }
        let v = prop(
            &incoming,
            "latestReplyPreview",
            "incoming.latestReplyPreview",
        )?;
        if !is_undef(v.as_ref()) {
            shell.insert("latestReplyPreview".into(), v.as_ref().clone());
        }
        let v = prop_opt(&previous, "pendingInteractions");
        if !is_undef(v.as_ref()) {
            shell.insert("pendingInteractions".into(), v.as_ref().clone());
        }
    }
    if !is_undef(d.last_visited_at.as_ref()) {
        shell.insert("lastVisitedAt".into(), d.last_visited_at.as_ref().clone());
    }
    let mut turn_state = Map::new();
    turn_state.insert("latestTurn".into(), d.latest_turn.as_ref().clone());
    {
        let spp = prop_opt(&d.latest_turn, "sourceProposedPlan");
        if truthy(spp.as_ref()) {
            let plan = prop(
                &d.latest_turn,
                "sourceProposedPlan",
                "latestTurn.sourceProposedPlan",
            )?;
            turn_state.insert("pendingSourceProposedPlan".into(), plan.as_ref().clone());
        }
    }
    {
        let status = prop_opt(d.claude_cache_review.as_ref(), "status");
        if strict_eq(status.as_ref(), &str_val("compacting")) {
            turn_state.insert("pendingTurnStartMessageId".into(), Value::Null);
        } else {
            let v = prop_opt(&previous, "pendingTurnStartMessageId");
            if !is_undef(v.as_ref()) {
                turn_state.insert("pendingTurnStartMessageId".into(), v.as_ref().clone());
            }
        }
    }
    let mut out = Map::new();
    out.insert("shell".into(), Value::Object(shell));
    out.insert("session".into(), d.session.as_ref().clone());
    out.insert("turnState".into(), Value::Object(turn_state));
    Ok(Value::Object(out))
}

/// Port of `mapProjects`.
pub fn map_projects(incoming: Value, previous: Value) -> Result<Value, String> {
    let pairs = map_call(
        &previous,
        "previous.map",
        "previous.map((project) => [project.id, project])",
        &mut |project: Cow<Value>, _| {
            let id = prop(project.as_ref(), "id", "project.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                project.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_id = map_of_pairs(pairs);
    let pairs = map_call(
        &previous,
        "previous.map",
        "previous.map((project) => [projectCwdKey(project.cwd), project])",
        &mut |project: Cow<Value>, _| {
            let cwd = prop(project.as_ref(), "cwd", "project.cwd")?;
            let key = project_cwd_key(cwd.as_ref())?;
            Ok(Cow::Owned(Value::Array(vec![
                str_val(&key),
                project.as_ref().clone(),
            ])))
        },
    )?;
    let previous_by_cwd = map_of_pairs(pairs);
    let pairs = map_call(
        &previous,
        "previous.map",
        "previous.map((project, index) => [project.id, index])",
        &mut |project: Cow<Value>, index| {
            let id = prop(project.as_ref(), "id", "project.id")?;
            Ok(Cow::Owned(Value::Array(vec![
                id.as_ref().clone(),
                num_val(index as f64),
            ])))
        },
    )?;
    let previous_order_by_id = map_of_pairs(pairs);
    let pairs = map_call(
        &previous,
        "previous.map",
        "previous.map((project, index) => [projectCwdKey(project.cwd), index])",
        &mut |project: Cow<Value>, index| {
            let cwd = prop(project.as_ref(), "cwd", "project.cwd")?;
            let key = project_cwd_key(cwd.as_ref())?;
            Ok(Cow::Owned(Value::Array(vec![
                str_val(&key),
                num_val(index as f64),
            ])))
        },
    )?;
    let previous_order_by_cwd = map_of_pairs(pairs);
    let use_persisted = strict_eq(
        prop(&previous, "length", "previous.length")?.as_ref(),
        &num_val(0.0),
    );
    let mapped = map_call(
        &incoming,
        "incoming.map",
        "incoming.map((project) => {\n    const existing = previousById.get(project.id) ?? previousByCwd.get(projectCwdKey(project.workspaceRoot));\n    return normalizeProject(project, existing);\n  })",
        &mut |project: Cow<Value>, _| {
            let id = prop(project.as_ref(), "id", "project.id")?;
            let existing = previous_by_id.get(id.as_ref()).map(Cow::Borrowed);
            let existing = match existing {
                Some(e) => e,
                None => {
                    let wr = prop(project.as_ref(), "workspaceRoot", "project.workspaceRoot")?;
                    let key = project_cwd_key(wr.as_ref())?;
                    previous_by_cwd
                        .get(&str_val(&key))
                        .map(Cow::Borrowed)
                        .unwrap_or(Cow::Borrowed(undef()))
                }
            };
            Ok(Cow::Owned(normalize_project_inner(
                project.as_ref(),
                existing.as_ref(),
            )?))
        },
    )?;
    let prev_len = prop(&previous, "length", "previous.length")?;
    let mut entries: Vec<Cow<'_, Value>> = Vec::with_capacity(mapped.len());
    for (incoming_index, project) in mapped.iter().enumerate() {
        let id = prop(project.as_ref(), "id", "project.id")?;
        let cwd = prop(project.as_ref(), "cwd", "project.cwd")?;
        let cwd_key = project_cwd_key(cwd.as_ref())?;
        let previous_index = previous_order_by_id
            .get(id.as_ref())
            .cloned()
            .or_else(|| previous_order_by_cwd.get(&str_val(&cwd_key)).cloned());
        let order_index = match previous_index {
            Some(i) => i,
            None => {
                let base = if use_persisted {
                    0.0
                } else {
                    to_number(prev_len.as_ref())
                };
                num_val(base + incoming_index as f64)
            }
        };
        let mut o = Map::new();
        o.insert("project".into(), project.as_ref().clone());
        o.insert("incomingIndex".into(), num_val(incoming_index as f64));
        o.insert("orderIndex".into(), order_index);
        entries.push(Cow::Owned(Value::Object(o)));
    }
    js_sort_res(&mut entries, &mut |a: &Cow<Value>, b: &Cow<Value>| {
        let ao = prop(a.as_ref(), "orderIndex", "a.orderIndex")?;
        let bo = prop(b.as_ref(), "orderIndex", "b.orderIndex")?;
        let by_order = to_number(ao.as_ref()) - to_number(bo.as_ref());
        if by_order != 0.0 {
            return Ok(by_order);
        }
        let ai = prop(a.as_ref(), "incomingIndex", "a.incomingIndex")?;
        let bi = prop(b.as_ref(), "incomingIndex", "b.incomingIndex")?;
        Ok(to_number(ai.as_ref()) - to_number(bi.as_ref()))
    })?;
    let mapped_projects: Vec<Cow<'_, Value>> = entries
        .iter()
        .map(|e| {
            prop(e.as_ref(), "project", "entry.project")
                .map(|c| c.into_owned())
                .map(Cow::Owned)
        })
        .collect::<Result<_, String>>()?;
    if arrays_shallow_eq(&previous, &mapped_projects)? {
        Ok(previous)
    } else {
        Ok(Value::Array(
            mapped_projects.iter().map(|c| c.as_ref().clone()).collect(),
        ))
    }
}

/// Port of `toLegacyProvider`.
pub fn to_legacy_provider(provider_name: Value) -> Result<Value, String> {
    Ok(to_legacy_provider_val(&provider_name).into_owned())
}

/// Port of `resolveThreadSidebarMetadata`.
pub fn resolve_thread_sidebar_metadata(thread: Value) -> Result<Value, String> {
    let needs =
        is_undef(prop(&thread, "latestUserMessageAt", "thread.latestUserMessageAt")?.as_ref())
            || is_undef(
                prop(
                    &thread,
                    "latestHumanMessageAt",
                    "thread.latestHumanMessageAt",
                )?
                .as_ref(),
            )
            || is_undef(
                prop(&thread, "hasPendingApprovals", "thread.hasPendingApprovals")?.as_ref(),
            )
            || is_undef(
                prop(&thread, "hasPendingUserInput", "thread.hasPendingUserInput")?.as_ref(),
            )
            || is_undef(
                prop(
                    &thread,
                    "hasActionableProposedPlan",
                    "thread.hasActionableProposedPlan",
                )?
                .as_ref(),
            );
    let derived = if needs {
        Some(derive_thread_summary_metadata(
            prop(&thread, "messages", "thread.messages")?.as_ref(),
            prop(&thread, "activities", "thread.activities")?.as_ref(),
            prop(&thread, "proposedPlans", "thread.proposedPlans")?.as_ref(),
            prop(&thread, "latestTurn", "thread.latestTurn")?.as_ref(),
        )?)
    } else {
        None
    };
    let t_lu = prop(&thread, "latestUserMessageAt", "thread.latestUserMessageAt")?;
    let latest_user = if !is_nullish(t_lu.as_ref()) {
        t_lu.as_ref().clone()
    } else {
        let d = derived
            .as_ref()
            .map(|v| prop_opt(v, "latestUserMessageAt").into_owned());
        match d {
            Some(v) if !is_nullish(&v) => v,
            _ => Value::Null,
        }
    };
    let t_lh = prop(
        &thread,
        "latestHumanMessageAt",
        "thread.latestHumanMessageAt",
    )?;
    let latest_human = if !is_undef(t_lh.as_ref()) {
        t_lh.as_ref().clone()
    } else {
        let d = derived
            .as_ref()
            .map(|v| prop_opt(v, "latestHumanMessageAt").into_owned());
        match d {
            Some(v) if !is_nullish(&v) => v,
            _ => Value::Null,
        }
    };
    let or_false = |v: Cow<'_, Value>, key: &str| -> Value {
        if !is_nullish(v.as_ref()) {
            return v.into_owned();
        }
        let d = derived.as_ref().map(|x| prop_opt(x, key).into_owned());
        match d {
            Some(x) if !is_nullish(&x) => x,
            _ => Value::Bool(false),
        }
    };
    let has_approvals = or_false(
        prop(&thread, "hasPendingApprovals", "thread.hasPendingApprovals")?,
        "hasPendingApprovals",
    );
    let has_user_input = or_false(
        prop(&thread, "hasPendingUserInput", "thread.hasPendingUserInput")?,
        "hasPendingUserInput",
    );
    let has_actionable = or_false(
        prop(
            &thread,
            "hasActionableProposedPlan",
            "thread.hasActionableProposedPlan",
        )?,
        "hasActionableProposedPlan",
    );
    let has_live_tail = has_live_turn_tail_work(
        prop(&thread, "latestTurn", "thread.latestTurn")?.as_ref(),
        prop(&thread, "messages", "thread.messages")?.as_ref(),
        prop(&thread, "activities", "thread.activities")?.as_ref(),
        prop(&thread, "session", "thread.session")?.as_ref(),
    )?;
    let pending_count = {
        let v = prop(
            &thread,
            "pendingBackgroundTaskCount",
            "thread.pendingBackgroundTaskCount",
        )?;
        if !is_nullish(v.as_ref()) {
            v.as_ref().clone()
        } else {
            let pbw = derive_pending_background_work(
                prop(&thread, "activities", "thread.activities")?.as_ref(),
                prop(&thread, "latestTurn", "thread.latestTurn")?.as_ref(),
                prop(&thread, "session", "thread.session")?.as_ref(),
            )?;
            let count = pbw
                .as_ref()
                .map(|p| prop_opt(p, "count").into_owned())
                .filter(|c| !is_nullish(c))
                .unwrap_or_else(|| num_val(0.0));
            let outstanding = count_outstanding_background_work(
                prop(&thread, "activities", "thread.activities")?.as_ref(),
                prop(&thread, "session", "thread.session")?.as_ref(),
            )?;
            num_val(js_max(to_number(&count), outstanding))
        }
    };
    let mut o = Map::new();
    o.insert("latestUserMessageAt".into(), latest_user);
    o.insert("latestHumanMessageAt".into(), latest_human);
    o.insert("hasPendingApprovals".into(), has_approvals);
    o.insert("hasPendingUserInput".into(), has_user_input);
    o.insert("hasActionableProposedPlan".into(), has_actionable);
    o.insert("hasLiveTailWork".into(), Value::Bool(has_live_tail));
    o.insert("pendingBackgroundWorkCount".into(), pending_count);
    Ok(Value::Object(o))
}

fn to_number_or(v: &Value, default: f64) -> f64 {
    if is_nullish(v) { default } else { to_number(v) }
}
