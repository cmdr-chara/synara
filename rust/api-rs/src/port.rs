//! The Rust port of `apps/analytics/src/api.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const FILTER_KEYS: &[&str] = &["days", "version", "platform", "flavor", "kind", "q"];

/// Port of `pickFilters` (`apps/analytics/src/api.ts:25`).
pub fn pick_filters(
    f: serde_json::Value,
    fields: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // `{ ...EMPTY_FILTERS, days: f.days }` — `days` stays first, `f.days` wins.
    let get = |f: &Value, key: &str| -> Value { f.get(key).cloned().unwrap_or_else(js::undefined) };
    let mut out = serde_json::Map::new();
    for key in FILTER_KEYS {
        out.insert(
            (*key).to_string(),
            match *key {
                "days" => get(&f, "days"),
                _ => Value::String(String::new()),
            },
        );
    }
    // `days: f.days` reads `days` on `f` before the loop over `fields`.
    if f.is_null() {
        return Err("Cannot read properties of null (reading 'days')".to_string());
    }
    out.insert("days".to_string(), get(&f, "days"));
    // `for key of fields`: arrays spread directly, strings iterate code points.
    let owned: Vec<Value>;
    let fields: &[Value] = match &fields {
        Value::Array(a) => a,
        Value::String(s) => {
            owned = s.chars().map(|c| Value::String(c.to_string())).collect();
            &owned
        }
        _ => return Err("fields is not iterable".to_string()),
    };
    for field in fields {
        let key = jscore::js_to_string(field);
        if key != "days" {
            out.insert(key.clone(), get(&f, &key));
        }
    }
    Ok(Value::Object(out))
}

fn query(pairs: &[(&str, Value)]) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (key, value) in pairs {
        parts.push(format!(
            "{key}={}",
            jscore::form_url_encode(&jscore::js_to_string(value))
        ));
    }
    format!("?{}", parts.join("&"))
}

/// Port of `filtersQuery` (`apps/analytics/src/api.ts:33`).
pub fn filters_query(f: serde_json::Value) -> Result<String, String> {
    if f.is_null() {
        return Err("Cannot read properties of null (reading 'days')".to_string());
    }
    let mut pairs = vec![("days", f.get("days").cloned().unwrap_or_else(js::undefined))];
    for key in ["version", "platform", "flavor", "kind", "q"] {
        if let Some(v) = f.get(key).filter(|v| jscore::js_truthy(v)) {
            pairs.push((key, v.clone()));
        }
    }
    Ok(query(&pairs))
}

/// Port of `loginErrorMessage` (`apps/analytics/src/api.ts:44`).
pub fn login_error_message(res: serde_json::Value) -> Result<String, String> {
    if res.is_null() {
        return Ok("Couldn't reach the server. Try again.".to_string());
    }
    if js::is_undefined(&res) {
        return Err("Cannot read properties of undefined (reading 'status')".to_string());
    }
    if res
        .get("status")
        .is_some_and(|s| js::as_number(s) == Some(429.0))
    {
        return Ok("Too many attempts. Wait a minute and try again.".to_string());
    }
    Ok("Wrong password.".to_string())
}

/// Port of `productFiltersQuery` (`apps/analytics/src/api.ts:318`).
pub fn product_filters_query(f: serde_json::Value) -> Result<String, String> {
    if f.is_null() {
        return Err("Cannot read properties of null (reading 'from')".to_string());
    }
    let mut pairs = vec![
        ("from", f.get("from").cloned().unwrap_or_else(js::undefined)),
        ("to", f.get("to").cloned().unwrap_or_else(js::undefined)),
    ];
    for key in ["channel", "surface"] {
        if let Some(v) = f.get(key).filter(|v| jscore::js_truthy(v)) {
            pairs.push((key, v.clone()));
        }
    }
    Ok(query(&pairs))
}
