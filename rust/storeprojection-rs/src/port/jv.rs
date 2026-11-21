//! JS values with node identity: `===` on objects/arrays compares the node a value shares with
//! its source tree, so deep-equal-but-different nodes differ.
#![allow(dead_code)]

use serde_json::Value;
use std::cell::Cell;
use std::rc::Rc;

/// Insertion-ordered string-keyed map (JS object own-property order).
#[derive(Clone, Default)]
pub struct Obj(pub Vec<(String, JV)>);
impl Obj {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn get(&self, k: &str) -> Option<&JV> {
        self.0.iter().find(|(x, _)| x == k).map(|(_, v)| v)
    }
    pub fn get_mut(&mut self, k: &str) -> Option<&mut JV> {
        self.0.iter_mut().find(|(x, _)| x == k).map(|(_, v)| v)
    }
    pub fn insert(&mut self, k: String, v: JV) {
        if let Some(e) = self.0.iter_mut().find(|(x, _)| *x == k) {
            e.1 = v;
        } else {
            self.0.push((k, v));
        }
    }
    pub fn remove(&mut self, k: &str) {
        self.0.retain(|(x, _)| x != k);
    }
    pub fn contains_key(&self, k: &str) -> bool {
        self.0.iter().any(|(x, _)| x == k)
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.0.iter().map(|(k, _)| k)
    }
    pub fn iter(&self) -> core::slice::Iter<'_, (String, JV)> {
        self.0.iter()
    }
}
impl FromIterator<(String, JV)> for Obj {
    fn from_iter<I: IntoIterator<Item = (String, JV)>>(it: I) -> Self {
        let mut m = Obj::new();
        for (k, v) in it {
            m.insert(k, v);
        }
        m
    }
}
impl IntoIterator for Obj {
    type Item = (String, JV);
    type IntoIter = std::vec::IntoIter<(String, JV)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
impl<'a> IntoIterator for &'a Obj {
    type Item = &'a (String, JV);
    type IntoIter = core::slice::Iter<'a, (String, JV)>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

thread_local! {
    static NEXT_ID: Cell<u64> = const { Cell::new(1) };
}
fn fresh_id() -> u64 {
    NEXT_ID.with(|c| {
        let v = c.get();
        c.set(v + 1);
        v
    })
}

#[derive(Clone)]
pub enum JV {
    U,
    N,
    B(bool),
    F(f64),
    S(Rc<Vec<u16>>),
    A(u64, Rc<Vec<JV>>),
    O(u64, Rc<Obj>),
}

pub fn u() -> JV {
    JV::U
}
pub fn null() -> JV {
    JV::N
}
pub fn b(x: bool) -> JV {
    JV::B(x)
}
pub fn n(x: f64) -> JV {
    JV::F(x)
}
pub fn s(x: &str) -> JV {
    JV::S(Rc::new(x.encode_utf16().collect()))
}
pub fn su(units: Vec<u16>) -> JV {
    JV::S(Rc::new(units))
}
pub fn a(items: Vec<JV>) -> JV {
    JV::A(fresh_id(), Rc::new(items))
}
pub fn o(map: Obj) -> JV {
    JV::O(fresh_id(), Rc::new(map))
}
pub fn op(pairs: &[(&str, JV)]) -> JV {
    let mut m = Obj::new();
    for (k, v) in pairs {
        m.insert((*k).to_string(), v.clone());
    }
    o(m)
}

pub fn node_id(v: &JV) -> Option<u64> {
    match v {
        JV::A(i, _) | JV::O(i, _) => Some(*i),
        _ => None,
    }
}

pub fn decode(v: &Value) -> JV {
    match v {
        Value::Null => JV::N,
        Value::Bool(x) => JV::B(*x),
        Value::Number(x) => JV::F(x.as_f64().unwrap_or(f64::NAN)),
        Value::String(x) => s(x),
        Value::Array(items) => JV::A(fresh_id(), Rc::new(items.iter().map(decode).collect())),
        Value::Object(m) => {
            if m.len() == 1 {
                if m.get("$undefined") == Some(&Value::Bool(true)) {
                    return JV::U;
                }
                if let Some(Value::String(t)) = m.get("$num") {
                    return JV::F(match t.as_str() {
                        "NaN" => f64::NAN,
                        "Infinity" => f64::INFINITY,
                        "-Infinity" => f64::NEG_INFINITY,
                        "-0" => -0.0,
                        _ => t.parse::<f64>().unwrap_or(f64::NAN),
                    });
                }
                if let Some(Value::Array(units)) = m.get("$utf16") {
                    let u: Vec<u16> = units
                        .iter()
                        .filter_map(|x| x.as_u64().and_then(|x| u16::try_from(x).ok()))
                        .collect();
                    if u.len() == units.len() {
                        return su(u);
                    }
                }
            }
            JV::O(
                fresh_id(),
                Rc::new(m.iter().map(|(k, v)| (k.clone(), decode(v))).collect()),
            )
        }
    }
}

pub fn encode(v: &JV) -> Value {
    match v {
        JV::U => crate::js::undefined(),
        JV::N => Value::Null,
        JV::B(x) => Value::Bool(*x),
        JV::F(x) => crate::js::number(*x),
        JV::S(units) => crate::js::utf16_string(units),
        JV::A(_, items) => Value::Array(items.iter().map(encode).collect()),
        JV::O(_, m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), encode(v))).collect()),
    }
}

// ---------- predicates / conversions ----------

pub fn is_nullish(v: &JV) -> bool {
    matches!(v, JV::U | JV::N)
}
pub fn is_obj(v: &JV) -> bool {
    matches!(v, JV::O(..))
}
pub fn is_arr(v: &JV) -> bool {
    matches!(v, JV::A(..))
}
pub fn is_str(v: &JV) -> bool {
    matches!(v, JV::S(..))
}
pub fn is_num(v: &JV) -> bool {
    matches!(v, JV::F(..))
}
pub fn truthy(v: &JV) -> bool {
    match v {
        JV::U | JV::N => false,
        JV::B(x) => *x,
        JV::F(x) => *x != 0.0 && !x.is_nan(),
        JV::S(u) => !u.is_empty(),
        JV::A(..) | JV::O(..) => true,
    }
}
pub fn units(v: &JV) -> Option<&[u16]> {
    match v {
        JV::S(u) => Some(u),
        _ => None,
    }
}
pub fn ueq(u: &[u16], x: &str) -> bool {
    u.iter().copied().eq(x.encode_utf16())
}
pub fn streq(v: &JV, x: &str) -> bool {
    units(v).is_some_and(|u| ueq(u, x))
}
pub fn str_owned(v: &JV) -> Option<Vec<u16>> {
    units(v).map(|u| u.to_vec())
}

/// JS `===` (SameValue): numbers by IEEE (`NaN` differs, `-0 === 0`), strings by units,
/// objects/arrays by node identity.
pub fn eq(a: &JV, b: &JV) -> bool {
    match (a, b) {
        (JV::U, JV::U) | (JV::N, JV::N) => true,
        (JV::B(x), JV::B(y)) => x == y,
        (JV::F(x), JV::F(y)) => x == y,
        (JV::S(x), JV::S(y)) => x == y,
        (JV::A(i, _), JV::A(j, _)) | (JV::O(i, _), JV::O(j, _)) => i == j,
        _ => false,
    }
}
/// `Object.is`: like `===` but `NaN` equals `NaN` and `-0` differs from `0`.
pub fn same_value(a: &JV, b: &JV) -> bool {
    match (a, b) {
        (JV::F(x), JV::F(y)) => {
            if x.is_nan() && y.is_nan() {
                true
            } else if *x == 0.0 && *y == 0.0 {
                x.is_sign_negative() == y.is_sign_negative()
            } else {
                x == y
            }
        }
        _ => eq(a, b),
    }
}

const WS: &[u16] = &[
    0x9, 0xa, 0xb, 0xc, 0xd, 0x20, 0xa0, 0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005,
    0x2006, 0x2007, 0x2008, 0x2009, 0x200a, 0x2028, 0x2029, 0x202f, 0x205f, 0x3000, 0xfeff,
];
fn is_ws(c: u16) -> bool {
    WS.contains(&c)
}
pub fn str_trim(u: &[u16]) -> Vec<u16> {
    let mut lo = 0;
    let mut hi = u.len();
    while lo < hi && is_ws(u[lo]) {
        lo += 1;
    }
    while hi > lo && is_ws(u[hi - 1]) {
        hi -= 1;
    }
    u[lo..hi].to_vec()
}
pub fn str_lower(u: &[u16]) -> Vec<u16> {
    let s = String::from_utf16_lossy(u);
    s.to_lowercase().encode_utf16().collect()
}
pub fn str_slice(u: &[u16], start: i64, end: Option<i64>) -> Vec<u16> {
    let len = u.len() as i64;
    let clamp = |x: i64| -> usize {
        if x < 0 {
            (len + x).max(0) as usize
        } else {
            (x.min(len)) as usize
        }
    };
    let a = clamp(start);
    let b = end.map_or(u.len(), clamp);
    if b <= a { Vec::new() } else { u[a..b].to_vec() }
}
pub fn str_starts(u: &[u16], p: &str) -> bool {
    u.len() >= p.len() && ueq(&u[..p.len()], p)
}
pub fn str_ends(u: &[u16], p: &str) -> bool {
    u.len() >= p.len() && ueq(&u[u.len() - p.len()..], p)
}
pub fn str_contains(u: &[u16], p: &str) -> bool {
    let p: Vec<u16> = p.encode_utf16().collect();
    p.is_empty() || u.windows(p.len()).any(|w| w == p.as_slice())
}
pub fn str_index_of(u: &[u16], p: &str) -> i64 {
    let p: Vec<u16> = p.encode_utf16().collect();
    if p.is_empty() {
        return 0;
    }
    match u.windows(p.len()).position(|w| w == p.as_slice()) {
        Some(i) => i as i64,
        None => -1,
    }
}

/// `Number(x)` on a JS string value (already JS-trimmed by callers where needed).
pub fn str_to_num(u: &[u16]) -> f64 {
    let t = str_trim(u);
    if t.is_empty() {
        return 0.0;
    }
    if ueq(&t, "Infinity") || ueq(&t, "+Infinity") {
        return f64::INFINITY;
    }
    if ueq(&t, "-Infinity") {
        return f64::NEG_INFINITY;
    }
    let s = String::from_utf16_lossy(&t);
    let lower = s.to_lowercase();
    for (p, base) in [("0x", 16u32), ("0o", 8), ("0b", 2)] {
        if let Some(rest) = lower.strip_prefix(p) {
            if rest.is_empty() {
                return f64::NAN;
            }
            return match u64::from_str_radix(rest, base) {
                Ok(v) => v as f64,
                Err(_) => {
                    // JS allows bigints-free digits only up to u64 range; longer digit runs parse
                    // through BigInt-free semantics = parse as radix accumulated f64.
                    let mut v = 0f64;
                    let mut ok = true;
                    for c in rest.chars() {
                        match c.to_digit(base) {
                            Some(d) => v = v * base as f64 + d as f64,
                            None => {
                                ok = false;
                                break;
                            }
                        }
                    }
                    if ok { v } else { f64::NAN }
                }
            };
        }
    }
    // JS decimal grammar: [+-]? (digits[.digits?] | .digits) ([eE][+-]?digits)?
    let b = s.as_bytes();
    let mut i = 0;
    let mut mant_digits = false;
    let mut frac_digits = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        i += 1;
    }
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        mant_digits = true;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            frac_digits = true;
        }
    }
    if !mant_digits && !frac_digits {
        return f64::NAN;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return f64::NAN;
        }
    }
    if i != b.len() {
        return f64::NAN;
    }
    s.parse::<f64>().unwrap_or(f64::NAN)
}

pub fn to_num(v: &JV) -> f64 {
    match v {
        JV::U => f64::NAN,
        JV::N => 0.0,
        JV::B(x) => {
            if *x {
                1.0
            } else {
                0.0
            }
        }
        JV::F(x) => *x,
        JV::S(u) => str_to_num(u),
        JV::A(_, items) => {
            // ToPrimitive -> join(",") -> Number
            let mut parts = Vec::new();
            for it in items.iter() {
                parts.extend(to_prim_str(it));
                parts.push(u16::from(b','));
            }
            parts.pop();
            str_to_num(&parts)
        }
        JV::O(..) => f64::NAN,
    }
}
fn to_prim_str(v: &JV) -> Vec<u16> {
    match v {
        JV::U | JV::N => Vec::new(), // inside Array join -> ""
        JV::B(x) => if *x { "true" } else { "false" }.encode_utf16().collect(),
        JV::F(x) => crate::js::number_to_string(*x).encode_utf16().collect(),
        JV::S(u) => u.as_ref().clone(),
        JV::A(_, items) => {
            let mut parts = Vec::new();
            for it in items.iter() {
                parts.extend(to_prim_str(it));
                parts.push(u16::from(b','));
            }
            parts.pop();
            parts
        }
        JV::O(..) => "[object Object]".encode_utf16().collect(),
    }
}
/// `ToString`: undefined->'undefined', null->'null' (unlike inside array join).
pub fn to_str(v: &JV) -> Vec<u16> {
    match v {
        JV::U => "undefined".encode_utf16().collect(),
        JV::N => "null".encode_utf16().collect(),
        _ => to_prim_str(v),
    }
}
/// `ToPropertyKey` as a Rust map key (keys in corpus are well-formed).
pub fn pkey(v: &JV) -> String {
    match v {
        JV::S(u) => String::from_utf16_lossy(u),
        _ => String::from_utf16_lossy(&to_str(v)),
    }
}
pub fn units_pkey(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

/// `a + b`.
pub fn js_add(a: &JV, b: &JV) -> JV {
    if is_str(a) || is_str(b) {
        let mut u = to_str(a);
        u.extend(to_str(b));
        return su(u);
    }
    match (a, b) {
        (JV::A(..) | JV::O(..), _) | (_, JV::A(..) | JV::O(..)) => {
            let mut u = to_str(a);
            u.extend(to_str(b));
            su(u)
        }
        _ => JV::F(to_num(a) + to_num(b)),
    }
}
/// Relational `<`: strings compare by UTF-16 units, otherwise numeric; NaN -> false.
pub fn js_lt(a: &JV, b: &JV) -> bool {
    if let (Some(x), Some(y)) = (units(a), units(b)) {
        return x < y;
    }
    let (x, y) = (to_num(a), to_num(b));
    !x.is_nan() && !y.is_nan() && x < y
}
pub fn js_le(a: &JV, b: &JV) -> bool {
    if let (Some(x), Some(y)) = (units(a), units(b)) {
        return x <= y;
    }
    let (x, y) = (to_num(a), to_num(b));
    !x.is_nan() && !y.is_nan() && x <= y
}
pub fn js_gt(a: &JV, b: &JV) -> bool {
    js_lt(b, a)
}
pub fn js_ge(a: &JV, b: &JV) -> bool {
    js_le(b, a)
}
pub fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `deepEqualJson` from the source: `Object.is` shortcut, arrays elementwise,
/// objects by `key in` presence + recursive equality.
pub fn deep_eq(a: &JV, b: &JV) -> bool {
    if same_value(a, b) {
        return true;
    }
    match (a, b) {
        (JV::A(_, x), JV::A(_, y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| deep_eq(a, b))
        }
        (JV::O(_, x), JV::O(_, y)) => {
            x.len() == y.len()
                && x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| deep_eq(v, w)))
        }
        _ => false,
    }
}

// ---------- property access with exact errors ----------

pub fn not_obj_err(v: &JV, expr: &str) -> String {
    match v {
        JV::N => format!("null is not an object (evaluating '{expr}')"),
        _ => format!("undefined is not an object (evaluating '{expr}')"),
    }
}
/// `x.k` where the receiver is known non-nullish, or throws the property-access error.
pub fn gv(v: &JV, k: &str, expr: &str) -> Result<JV, String> {
    if is_nullish(v) {
        return Err(not_obj_err(v, expr));
    }
    Ok(g(v, k))
}
/// Property read that never throws: object lookup, else `undefined`.
pub fn g(v: &JV, k: &str) -> JV {
    match v {
        JV::O(_, m) => m.get(k).cloned().unwrap_or(JV::U),
        _ => JV::U,
    }
}
/// `record[k]` with a computed key.
pub fn gk(v: &JV, key: &JV) -> JV {
    match v {
        JV::O(_, m) => m.get(&pkey(key)).cloned().unwrap_or(JV::U),
        JV::A(_, items) => match units(key) {
            Some(u) => {
                let ks = String::from_utf16_lossy(u);
                ks.parse::<usize>()
                    .ok()
                    .filter(|i| *i < items.len() && ks == i.to_string())
                    .map(|i| items[i].clone())
                    .unwrap_or(JV::U)
            }
            None => JV::U,
        },
        JV::S(u) => match units(key) {
            Some(k) => {
                let ks = String::from_utf16_lossy(k);
                ks.parse::<usize>()
                    .ok()
                    .filter(|i| *i < u.len() && ks == i.to_string())
                    .map(|i| su(vec![u[i]]))
                    .unwrap_or(JV::U)
            }
            None => JV::U,
        },
        _ => JV::U,
    }
}
/// `key in x` / `Object.hasOwn`-style presence (own properties only).
pub fn has(v: &JV, k: &str) -> bool {
    match v {
        JV::O(_, m) => m.contains_key(k),
        JV::A(_, items) => k
            .parse::<usize>()
            .ok()
            .is_some_and(|i| i < items.len() && k == i.to_string()),
        JV::S(u) => k
            .parse::<usize>()
            .ok()
            .is_some_and(|i| i < u.len() && k == i.to_string()),
        _ => false,
    }
}
pub fn has_key(v: &JV, key: &JV) -> bool {
    has(v, &pkey(key))
}

pub fn mcall_not_fn(expr: &str, method: &str, call: &str) -> String {
    format!("{expr}.{method} is not a function. (In '{call}', '{expr}.{method}' is undefined)")
}
/// `x.m(...)` receiver check: nullish -> `not an object` on `x.m`; non-array -> `not a function`.
pub fn arr_recv<'a>(v: &'a JV, expr: &str, m: &str, call: &str) -> Result<&'a [JV], String> {
    if is_nullish(v) {
        return Err(not_obj_err(v, &format!("{expr}.{m}")));
    }
    match v {
        JV::A(_, items) => Ok(items),
        _ => Err(mcall_not_fn(expr, m, call)),
    }
}
/// `x.trim()`/string-method receiver: returns the string's units or the exact error.
pub fn str_recv<'a>(v: &'a JV, expr: &str, m: &str, call: &str) -> Result<&'a [u16], String> {
    if is_nullish(v) {
        return Err(not_obj_err(v, &format!("{expr}.{m}")));
    }
    match units(v) {
        Some(u) => Ok(u),
        None => Err(mcall_not_fn(expr, m, call)),
    }
}
/// `x.localeCompare(y)` receiver: string or exact error.
pub fn lc_recv<'a>(v: &'a JV, expr: &str, call: &str) -> Result<&'a [u16], String> {
    str_recv(v, expr, "localeCompare", call)
}
/// localeCompare on corpus-ranked strings; unknown strings fall back after all known ones,
/// ordered by UTF-16 (equals JS only when the corpus covered the operand).
pub fn locale_cmp(a: &[u16], b: &[u16]) -> std::cmp::Ordering {
    use std::cmp::Ordering::{Greater, Less};
    match (crate::corpus::rank(a), crate::corpus::rank(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        (None, None) => a.cmp(b),
        (Some(_), None) => Less,
        (None, Some(_)) => Greater,
    }
}

pub fn locale_compare(recv: &JV, arg: &JV, expr: &str, call: &str) -> Result<f64, String> {
    let a = lc_recv(recv, expr, call)?;
    let b = to_str(arg);
    Ok(match locale_cmp(a, &b) {
        std::cmp::Ordering::Less => -1.0,
        std::cmp::Ordering::Equal => 0.0,
        std::cmp::Ordering::Greater => 1.0,
    })
}

// ---------- object/array utilities ----------

/// `{...x}`: own enumerable keys into a fresh map (arrays/strings by index, others empty).
pub fn spread_map(v: &JV) -> Obj {
    let mut m = Obj::new();
    match v {
        JV::O(_, x) => {
            for (k, v) in x.iter() {
                m.insert(k.clone(), v.clone());
            }
        }
        JV::A(_, items) => {
            for (i, v) in items.iter().enumerate() {
                m.insert(i.to_string(), v.clone());
            }
        }
        JV::S(u) => {
            for (i, c) in u.iter().enumerate() {
                m.insert(i.to_string(), su(vec![*c]));
            }
        }
        _ => {}
    }
    m
}
pub fn spread(v: &JV) -> JV {
    o(spread_map(v))
}
/// Set `obj.k = v` preserving JS key order (existing keys keep position).
pub fn mset(m: &mut Obj, k: &str, v: JV) {
    m.insert(k.to_string(), v);
}
/// Iterate `[...x]`: arrays only spread elementwise in these sources.
pub fn spread_arr(v: &JV) -> Result<Vec<JV>, String> {
    match v {
        JV::A(_, items) => Ok(items.as_ref().clone()),
        _ => Err("Spread syntax requires ...iterable[Symbol.iterator] to be a function".into()),
    }
}
/// `Object.entries(x)` — arrays and strings expose index entries; other primitives yield none.
pub fn obj_entries(v: &JV, expr: &str) -> Result<Vec<(String, JV)>, String> {
    match v {
        JV::U | JV::N => Err(format!(
            "Object.entries requires that input parameter not be null or undefined (evaluating '{expr}')"
        )),
        JV::O(_, m) => Ok(m.iter().map(|(k, v)| (k.clone(), v.clone())).collect()),
        JV::A(_, items) => Ok(items
            .iter()
            .enumerate()
            .map(|(i, v)| (i.to_string(), v.clone()))
            .collect()),
        JV::S(u) => Ok(u
            .iter()
            .enumerate()
            .map(|(i, c)| (i.to_string(), su(vec![*c])))
            .collect()),
        _ => Ok(Vec::new()),
    }
}
/// `x.entries()` (Array.prototype.entries).
pub fn arr_entries(v: &JV, expr: &str, call: &str) -> Result<Vec<JV>, String> {
    Ok(arr_recv(v, expr, "entries", call)?.to_vec())
}
/// `x.filter(f)` receiver validation only.
pub fn arr_of<'a>(v: &'a JV, expr: &str, m: &str, call: &str) -> Result<&'a [JV], String> {
    arr_recv(v, expr, m, call)
}

// ---------- Map / Set with SameValueZero keys ----------

pub fn map_eq(a: &JV, b: &JV) -> bool {
    match (a, b) {
        (JV::F(x), JV::F(y)) if x.is_nan() && y.is_nan() => true,
        _ => eq(a, b),
    }
}
#[derive(Default, Clone)]
pub struct JsMap(pub Vec<(JV, JV)>);
impl JsMap {
    pub fn has(&self, k: &JV) -> bool {
        self.0.iter().any(|(x, _)| map_eq(x, k))
    }
    pub fn get(&self, k: &JV) -> JV {
        self.0
            .iter()
            .find(|(x, _)| map_eq(x, k))
            .map(|(_, v)| v.clone())
            .unwrap_or(JV::U)
    }
    pub fn getr(&self, k: &JV) -> Option<&JV> {
        self.0.iter().find(|(x, _)| map_eq(x, k)).map(|(_, v)| v)
    }
    pub fn set(&mut self, k: JV, v: JV) {
        if let Some(e) = self.0.iter_mut().find(|(x, _)| map_eq(x, &k)) {
            e.1 = v;
        } else {
            self.0.push((k, v));
        }
    }
    pub fn delete(&mut self, k: &JV) {
        self.0.retain(|(x, _)| !map_eq(x, k));
    }
}
/// Keyed by strings — the only key kind the sources index by name.
#[derive(Default, Clone)]
pub struct SMap<V: Clone>(pub Vec<(String, V)>);
impl<V: Clone> SMap<V> {
    pub fn has(&self, k: &str) -> bool {
        self.0.iter().any(|(x, _)| x == k)
    }
    pub fn get(&self, k: &str) -> Option<&V> {
        self.0.iter().find(|(x, _)| x == k).map(|(_, v)| v)
    }
    pub fn set(&mut self, k: String, v: V) {
        if let Some(e) = self.0.iter_mut().find(|(x, _)| *x == k) {
            e.1 = v;
        } else {
            self.0.push((k, v));
        }
    }
    pub fn delete(&mut self, k: &str) {
        self.0.retain(|(x, _)| x != k);
    }
}

pub fn js_sort(items: &mut [JV], mut cmp: impl FnMut(&JV, &JV) -> f64) {
    crate::js::sort(items, |a, b| cmp(a, b));
}

// ---------- misc runtime ----------

/// `JSON.stringify` (single-line) for the small values the sources stringify.
pub fn json_stringify(v: &JV) -> String {
    let mut out = String::new();
    stringify_into(v, &mut out, false);
    out
}
fn stringify_into(v: &JV, out: &mut String, in_arr: bool) {
    match v {
        JV::U => out.push_str(if in_arr { "null" } else { "undefined" }),
        JV::N => out.push_str("null"),
        JV::B(x) => out.push_str(if *x { "true" } else { "false" }),
        JV::F(x) => {
            if x.is_finite() {
                out.push_str(&crate::js::number_to_string(*x));
            } else {
                out.push_str("null");
            }
        }
        JV::S(u) => {
            out.push('"');
            for c in u.iter() {
                match *c {
                    0x22 => out.push_str("\\\""),
                    0x5c => out.push_str("\\\\"),
                    0x08 => out.push_str("\\b"),
                    0x09 => out.push_str("\\t"),
                    0x0a => out.push_str("\\n"),
                    0x0c => out.push_str("\\f"),
                    0x0d => out.push_str("\\r"),
                    c if c < 0x20 => out.push_str(&format!("\\u{c:04x}")),
                    c => {
                        if let Some(ch) = char::from_u32(c as u32) {
                            out.push(ch);
                        } else {
                            out.push('\u{fffd}');
                        }
                    }
                }
            }
            out.push('"');
        }
        JV::A(_, items) => {
            out.push('[');
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                match it {
                    JV::U => out.push_str("null"),
                    _ => stringify_into(it, out, true),
                }
            }
            out.push(']');
        }
        JV::O(_, m) => {
            out.push('{');
            let mut first = true;
            for (k, v) in m.iter() {
                if matches!(v, JV::U) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push('"');
                for c in k.encode_utf16() {
                    match c {
                        0x22 => out.push_str("\\\""),
                        0x5c => out.push_str("\\\\"),
                        c if c < 0x20 => out.push_str(&format!("\\u{c:04x}")),
                        c => out.push(char::from_u32(c as u32).unwrap_or('\u{fffd}')),
                    }
                }
                out.push_str("\":");
                stringify_into(v, out, false);
            }
            out.push('}');
        }
    }
}

/// `Date.now()` in milliseconds.
pub fn date_now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// `Date.parse` for ISO-ish strings; anything else -> NaN.
pub fn date_parse(u: &[u16]) -> f64 {
    let s: Vec<u16> = str_trim(u);
    if s.is_empty() {
        return f64::NAN;
    }
    let st = String::from_utf16_lossy(&s);
    iso_parse(&st).unwrap_or(f64::NAN)
}
fn iso_parse(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0usize;
    let take = |b: &[u8], i: &mut usize, n: usize| -> Option<i64> {
        if *i + n > b.len() || !b[*i..*i + n].iter().all(|c| c.is_ascii_digit()) {
            return None;
        }
        let v = std::str::from_utf8(&b[*i..*i + n]).ok()?.parse().ok()?;
        *i += n;
        Some(v)
    };
    let year = if b.len() >= 2 && (b[0] == b'+' || b[0] == b'-') {
        i += 1;
        let y = take(b, &mut i, 6)?;
        if b[0] == b'-' { -y } else { y }
    } else {
        take(b, &mut i, 4)?
    };
    if i >= b.len() || b[i] != b'-' {
        return None;
    }
    i += 1;
    let month = take(b, &mut i, 2)?;
    if i >= b.len() || b[i] != b'-' {
        return None;
    }
    i += 1;
    let day = take(b, &mut i, 2)?;
    let mut hour = 0i64;
    let mut min = 0i64;
    let mut sec = 0i64;
    let mut ms = 0i64;
    let mut offset_min: Option<i64> = None;
    if i < b.len() && (b[i] == b'T' || b[i] == b' ') {
        i += 1;
        hour = take(b, &mut i, 2)?;
        if i >= b.len() || b[i] != b':' {
            return None;
        }
        i += 1;
        min = take(b, &mut i, 2)?;
        if i < b.len() && b[i] == b':' {
            i += 1;
            sec = take(b, &mut i, 2)?;
            if i < b.len() && b[i] == b'.' {
                i += 1;
                let mut frac = String::new();
                while i < b.len() && b[i].is_ascii_digit() {
                    frac.push(b[i] as char);
                    i += 1;
                }
                if frac.is_empty() {
                    return None;
                }
                frac.truncate(3);
                while frac.len() < 3 {
                    frac.push('0');
                }
                ms = frac.parse().ok()?;
            }
        }
        if i < b.len() && b[i] == b'Z' {
            offset_min = Some(0);
            i += 1;
        } else if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            let sign = if b[i] == b'-' { -1 } else { 1 };
            i += 1;
            let oh = take(b, &mut i, 2)?;
            let om = if i < b.len() && b[i] == b':' {
                i += 1;
                take(b, &mut i, 2)?
            } else {
                take(b, &mut i, 2).unwrap_or(0)
            };
            offset_min = Some(sign * (oh * 60 + om));
        }
    }
    if i != b.len() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 24 || min > 59 || sec > 59 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let local_ms = ((days * 24 + hour) * 60 + min) * 60_000 + sec * 1000 + ms;
    Some(match offset_min {
        Some(off) => (local_ms - off * 60_000) as f64,
        None => local_ms as f64, // treat naive as UTC; corpus timestamps carry Z
    })
}
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// `encodeURIComponent`: unreserved A-Z a-z 0-9 - _ . ! ~ * ' ( ) stay; else %XX of UTF-8.
pub fn encode_uri_component(u: &[u16]) -> Vec<u16> {
    const UNRES: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_.!~*'()";
    let mut out = Vec::new();
    let mut i = 0;
    while i < u.len() {
        let ch = u[i];
        if ch < 0x80 && UNRES.contains(ch as u8 as char) {
            out.push(ch);
            i += 1;
            continue;
        }
        let cp: u32 = if (0xd800..0xdc00).contains(&ch)
            && i + 1 < u.len()
            && (0xdc00..0xe000).contains(&u[i + 1])
        {
            i += 1;
            0x10000 + (((ch as u32) - 0xd800) << 10) + ((u[i] as u32) - 0xdc00)
        } else {
            ch as u32
        };
        let mut buf = [0u8; 4];
        let s = char::from_u32(cp)
            .map(|x| x.encode_utf8(&mut buf).to_string())
            .unwrap_or_else(|| "\u{fffd}".to_string());
        for byte in s.as_bytes() {
            out.extend(format!("%{byte:02X}").encode_utf16());
        }
        i += 1;
    }
    out
}

// ---------- the few regexes the sources use ----------

/// `\[([^\]]+)\]$` -> capture group, else None.
pub fn re_bracket_suffix(u: &[u16]) -> Option<Vec<u16>> {
    if u.last() != Some(&0x5d) {
        return None;
    }
    let open = u[..u.len() - 1].iter().rposition(|c| *c == 0x5b)?;
    Some(u[open + 1..u.len() - 1].to_vec())
}
/// `^[a-z0-9][a-z0-9-]*\/[0-9a-f]{8}$` -> capture group.
pub fn re_temp_branch(u: &[u16]) -> Option<Vec<u16>> {
    let slash = u.iter().rposition(|c| *c == 0x2f)?;
    let (head, tail) = (u[..slash].to_vec(), u[slash + 1..].to_vec());
    let head_ok = !head.is_empty()
        && head
            .iter()
            .all(|c| (0x30..=0x39).contains(c) || (0x61..=0x7a).contains(c) || *c == 0x2d)
        && head[0] != 0x2d;
    let tail_ok = tail.len() == 8
        && tail
            .iter()
            .all(|c| (0x30..=0x39).contains(c) || (0x61..=0x66).contains(c));
    if head_ok && tail_ok { Some(head) } else { None }
}
/// `^[a-z]:([\\/]|$)` case-insensitive.
pub fn re_windows_drive(u: &[u16]) -> bool {
    u.len() >= 2
        && (u[0] | 0x20) >= 0x61
        && (u[0] | 0x20) <= 0x7a
        && u[1] == 0x3a
        && (u.len() == 2 || u[2] == 0x2f || u[2] == 0x5c)
}
/// `^[A-Za-z0-9_-]{1,256}$`.
pub fn re_model_id(u: &[u16]) -> bool {
    !u.is_empty()
        && u.len() <= 256
        && u.iter().all(|c| {
            (0x30..=0x39).contains(c)
                || (0x41..=0x5a).contains(c)
                || (0x61..=0x7a).contains(c)
                || *c == 0x5f
                || *c == 0x2d
        })
}
/// fnv1a-64 over UTF-16 code units (the sources hash charCodeAt).
pub fn fnv1a(u: &[u16]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for c in u {
        h ^= *c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}
/// last non-empty `/`-or-`\`-separated segment.
pub fn basename_of_path(u: &[u16]) -> Vec<u16> {
    let mut end = u.len();
    while end > 0 && (u[end - 1] == 0x2f || u[end - 1] == 0x5c) {
        end -= 1;
    }
    let mut start = end;
    while start > 0 && u[start - 1] != 0x2f && u[start - 1] != 0x5c {
        start -= 1;
    }
    u[start..end].to_vec()
}
