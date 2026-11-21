//! JS values that JSON cannot express, and the JSON transport of the bindings.

#![forbid(unsafe_code)]

/// Reads a structured argument sent by `index.cjs`.
pub fn from_json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T, String> {
    let mut de = serde_json::Deserializer::from_str(text);
    de.disable_recursion_limit();
    let value =
        <T as serde::Deserialize>::deserialize(&mut de).map_err(|e| format!("TypeError: {e}"))?;
    de.end().map_err(|e| format!("TypeError: {e}"))?;
    Ok(value)
}

/// Writes a structured result for `index.cjs`.
pub fn to_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|e| e.to_string())
}

/// JS `undefined` inside a structured value: `{ a: undefined }` is not `{ a: null }`.
pub fn undefined() -> serde_json::Value {
    serde_json::json!({ "$undefined": true })
}

/// Whether a structured value received from JS was `undefined`.
pub fn is_undefined(value: &serde_json::Value) -> bool {
    value.as_object().is_some_and(|o| {
        o.len() == 1 && o.get("$undefined") == Some(&serde_json::Value::Bool(true))
    })
}

/// A JS string from UTF-16 code units: a plain string when well-formed, else a marker that
/// `index.cjs` turns into the exact JS string, lone surrogates included.
pub fn utf16_string(units: &[u16]) -> serde_json::Value {
    match String::from_utf16(units) {
        Ok(s) => serde_json::Value::String(s),
        Err(_) => serde_json::json!({ "$utf16": units }),
    }
}

/// The UTF-16 code units of a string received from JS, plain or marked.
pub fn string_units(value: &serde_json::Value) -> Option<Vec<u16>> {
    match value {
        serde_json::Value::String(s) => Some(s.encode_utf16().collect()),
        serde_json::Value::Object(o) if o.len() == 1 => o
            .get("$utf16")?
            .as_array()?
            .iter()
            .map(|u| u.as_u64().and_then(|u| u16::try_from(u).ok()))
            .collect(),
        _ => None,
    }
}

/// A JS number inside a structured value. JSON has no NaN, Infinity or -0: those cross as
/// `{"$num": "NaN"}` and friends, which `index.cjs` turns back into numbers.
pub fn number(n: f64) -> serde_json::Value {
    if n.is_nan() {
        serde_json::json!({ "$num": "NaN" })
    } else if n.is_infinite() {
        serde_json::json!({ "$num": if n > 0.0 { "Infinity" } else { "-Infinity" } })
    } else if n == 0.0 && n.is_sign_negative() {
        serde_json::json!({ "$num": "-0" })
    } else if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
        serde_json::Value::from(n as i64)
    } else {
        serde_json::Value::from(n)
    }
}

/// JS `Array.prototype.sort(compare)`, as V8 runs it.
///
/// With a consistent comparator any stable sort agrees with JS. With one that returns NaN or
/// contradicts itself, the order depends on the algorithm: under 64 elements this reproduces
/// V8's TimSort exactly (one run, then binary insertion); longer arrays use Rust's stable sort.
pub fn sort<T>(items: &mut [T], mut compare: impl FnMut(&T, &T) -> f64) {
    let n = items.len();
    if n < 2 {
        return;
    }
    if n >= 64 {
        items.sort_by(|a, b| {
            compare(a, b)
                .partial_cmp(&0.0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return;
    }
    // CountAndMakeRun: the leading ascending run, or strictly descending run (reversed).
    let mut run = 2;
    let descending = compare(&items[1], &items[0]) < 0.0;
    while run < n {
        let order = compare(&items[run], &items[run - 1]);
        if (descending && order >= 0.0) || (!descending && order < 0.0) {
            break;
        }
        run += 1;
    }
    if descending {
        items[..run].reverse();
    }
    // BinaryInsertionSort of the rest.
    for start in run..n {
        let (mut left, mut right) = (0, start);
        while left < right {
            let mid = left + ((right - left) >> 1);
            if compare(&items[start], &items[mid]) < 0.0 {
                right = mid;
            } else {
                left = mid + 1;
            }
        }
        items[left..=start].rotate_right(1);
    }
}

/// A number received from JS inside a structured value, plain or marked.
pub fn as_number(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::Object(o) if o.len() == 1 => match o.get("$num")?.as_str()? {
            "NaN" => Some(f64::NAN),
            "Infinity" => Some(f64::INFINITY),
            "-Infinity" => Some(f64::NEG_INFINITY),
            "-0" => Some(-0.0),
            _ => None,
        },
        _ => None,
    }
}
/// JS `Number.prototype.toString()` (what `${n}` and `String(n)` print): `1e+21`, `5e-324`,
/// `0` for `-0`, `NaN`, `Infinity`, and the shortest digits that read back as the same number
/// (the closest of them on a tie, as V8 picks). The digits come from serde_json's printer, several
/// times faster than `format!`.
#[allow(dead_code)]
pub fn number_to_string(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    if x.fract() == 0.0 && x.abs() < 9007199254740992.0 {
        return (x as i64).to_string();
    }
    let json = serde_json::to_string(&x).unwrap_or_default();
    let (sign, json) = match json.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", json.as_str()),
    };
    let (mantissa, exponent) = match json.find(['e', 'E']) {
        Some(i) => (&json[..i], json[i + 1..].parse::<i64>().unwrap_or(0)),
        None => (json, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let all = format!("{int}{frac}");
    let lead = all.len() - all.trim_start_matches('0').len();
    let digits = all[lead..].trim_end_matches('0');
    let k = digits.len() as i64;
    let n = int.len() as i64 - lead as i64 + exponent;
    let mut out = String::with_capacity(digits.len() + 8);
    out.push_str(sign);
    if k <= n && n <= 21 {
        out.push_str(digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(digits);
    } else {
        let e = n - 1;
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if e < 0 { '-' } else { '+' });
        out.push_str(&e.abs().to_string());
    }
    out
}

/// JS `Math.round`: the nearest integer, halves toward positive infinity (`-2.5` gives `-2`,
/// where Rust's `round` gives `-3`), and `-0` for negatives that round to zero.
#[allow(dead_code)]
pub fn math_round(x: f64) -> f64 {
    if !x.is_finite() || x == x.trunc() {
        return x;
    }
    let floor = x.floor();
    let r = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    if r == 0.0 && x < 0.0 { -0.0 } else { r }
}

/// JS truthiness, wire markers included.
pub fn truthy(v: Option<&serde_json::Value>) -> bool {
    use serde_json::Value;
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(v) => {
            if let Some(n) = as_number(v) {
                n != 0.0 && !n.is_nan()
            } else if let Some(u) = string_units(v) {
                !u.is_empty()
            } else {
                !is_undefined(v)
            }
        }
    }
}

/// `v === "lit"` — strict string equality, wire string markers included.
pub fn eq_str(v: Option<&serde_json::Value>, s: &str) -> bool {
    v.and_then(string_units)
        .is_some_and(|u| u == s.encode_utf16().collect::<Vec<u16>>())
}

/// `a === b` on primitives (wire markers included); objects never equal.
pub fn js_eq(a: Option<&serde_json::Value>, b: Option<&serde_json::Value>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(x), Some(y)) => {
            (x.is_null() && y.is_null())
                || (is_undefined(x) && is_undefined(y))
                || (string_units(x).is_some() && string_units(x) == string_units(y))
                || (as_number(x).is_some() && as_number(x) == as_number(y))
                || matches!(
                    (x, y),
                    (serde_json::Value::Bool(u), serde_json::Value::Bool(v)) if u == v
                )
        }
        _ => false,
    }
}

/// A canonical array-index key: `k` equals `ToString(ToUint32(k))` and is not 2**32-1.
pub fn array_index_key(k: &str) -> Option<u32> {
    if k.is_empty() || k == "4294967295" {
        return None;
    }
    let u: u32 = k.parse().ok()?;
    if u.to_string() == k { Some(u) } else { None }
}

/// `Object.entries(v)` — objects in own-key order (integer-index keys first, numerically
/// ascending, then the rest in insertion order), arrays by index, strings per UTF-16 code
/// unit; numbers, booleans and nullish values have no entries.
pub fn entries(v: &serde_json::Value) -> Vec<(String, serde_json::Value)> {
    use serde_json::Value;
    match v {
        Value::Object(o) => {
            let mut ints: Vec<(u32, String, serde_json::Value)> = Vec::new();
            let mut rest: Vec<(String, serde_json::Value)> = Vec::new();
            for (k, x) in o {
                if let Some(u) = array_index_key(k) {
                    ints.push((u, k.clone(), x.clone()));
                } else {
                    rest.push((k.clone(), x.clone()));
                }
            }
            ints.sort_by_key(|(u, _, _)| *u);
            ints.into_iter()
                .map(|(_, k, x)| (k, x))
                .chain(rest)
                .collect()
        }
        Value::Array(a) => a
            .iter()
            .enumerate()
            .map(|(i, x)| (i.to_string(), x.clone()))
            .collect(),
        Value::String(s) => s
            .encode_utf16()
            .enumerate()
            .map(|(i, u)| (i.to_string(), utf16_string(&[u])))
            .collect(),
        _ => Vec::new(),
    }
}

/// `ToPropertyKey(v)` for the JSON value range: string to itself, numbers via
/// `Number.prototype.toString`, booleans/null literal, arrays joined on `,`,
/// objects as "[object Object]".
pub fn to_property_key(v: &serde_json::Value) -> String {
    use serde_json::Value;
    if is_undefined(v) {
        return "undefined".into();
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::Bool(b) => b.to_string(),
        Value::Null => "null".into(),
        Value::Array(a) => a
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                Value::String(s) => s.clone(),
                other => to_property_key(other),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// `a.localeCompare(b)` for `en` collation semantics: primary compares the
/// case-folded strings, ties put lowercase before uppercase per code point.
pub fn locale_compare(a: &str, b: &str) -> f64 {
    let al: Vec<char> = a.chars().flat_map(|c| c.to_lowercase()).collect();
    let bl: Vec<char> = b.chars().flat_map(|c| c.to_lowercase()).collect();
    match al.cmp(&bl) {
        std::cmp::Ordering::Less => return -1.0,
        std::cmp::Ordering::Greater => return 1.0,
        std::cmp::Ordering::Equal => {}
    }
    // Tertiary: lowercase sorts before uppercase at the first differing position.
    for (x, y) in a.chars().zip(b.chars()) {
        if x == y {
            continue;
        }
        return if x.is_lowercase() && !y.is_lowercase() {
            -1.0
        } else if !x.is_lowercase() && y.is_lowercase() {
            1.0
        } else {
            -1.0
        };
    }
    0.0
}

/// `receiver[key]` — null/undefined receivers throw TypeError; objects read own keys,
/// arrays and strings read canonical numeric indices plus `length`; anything else is
/// `undefined`.
pub fn prop<'v>(
    v: &'v serde_json::Value,
    k: &str,
) -> Result<Option<&'v serde_json::Value>, String> {
    if v.is_null() || is_undefined(v) {
        let which = if v.is_null() { "null" } else { "undefined" };
        return Err(format!("Cannot read properties of {which} (reading '{k}')"));
    }
    Ok(match v {
        serde_json::Value::Object(o) => o.get(k),
        serde_json::Value::Array(a) => {
            if k == "length" {
                return Ok(None); // handled by callers needing the length value
            }
            array_index_key(k).and_then(|i| a.get(i as usize))
        }
        _ => None,
    }
    .filter(|x| !is_undefined(x)))
}

/// `receiver[key]` for cases where arrays/strings may answer `length` and indices —
/// returns the looked-up JSON value rather than a reference.
pub fn get_key(v: &serde_json::Value, k: &str) -> Result<serde_json::Value, String> {
    use serde_json::Value;
    if v.is_null() || is_undefined(v) {
        let which = if v.is_null() { "null" } else { "undefined" };
        return Err(format!("Cannot read properties of {which} (reading '{k}')"));
    }
    Ok(match v {
        Value::Object(o) => o.get(k).cloned().unwrap_or_else(undefined),
        Value::Array(a) => {
            if k == "length" {
                number(a.len() as f64)
            } else {
                array_index_key(k)
                    .and_then(|i| a.get(i as usize).cloned())
                    .unwrap_or_else(undefined)
            }
        }
        Value::String(s) => {
            if k == "length" {
                number(s.encode_utf16().count() as f64)
            } else {
                array_index_key(k)
                    .and_then(|i| s.encode_utf16().nth(i as usize))
                    .map(|u| utf16_string(&[u]))
                    .unwrap_or_else(undefined)
            }
        }
        _ => undefined(),
    })
}

#[cfg(test)]
mod js_number_tests {
    use super::*;

    #[test]
    fn prints_numbers_as_js_does() {
        for (x, js) in [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-7.5, "-7.5"),
            (0.1, "0.1"),
            (0.30000000000000004, "0.30000000000000004"),
            (1e21, "1e+21"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e-7, "1e-7"),
            (0.000001, "0.000001"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (2.5e-7, "2.5e-7"),
            (9007199254740993.0, "9007199254740992"),
            (-123456789.0, "-123456789"),
            (1e300, "1e+300"),
            (153596401214599.62, "153596401214599.62"),
            (-25.728225708007812, "-25.728225708007812"),
            (f64::NAN, "NaN"),
            (f64::NEG_INFINITY, "-Infinity"),
        ] {
            assert_eq!(number_to_string(x), js, "{x:?}");
        }
    }

    #[test]
    fn rounds_as_math_round() {
        for (x, js) in [
            (2.5, 3.0),
            (-2.5, -2.0),
            (-2.6, -3.0),
            (0.49999999999999994, 0.0),
            (1.5, 2.0),
            (-0.4, -0.0),
            (4503599627370495.5, 4503599627370496.0),
            (-7.0, -7.0),
        ] {
            let r = math_round(x);
            assert_eq!(r.to_bits(), f64::to_bits(js), "{x}: {r}");
        }
    }
}
