//! The Rust port of `apps/server/src/claudeTokenStats.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_falsy(v: &Value) -> bool {
    match v {
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64() == Some(0.0),
        Value::String(s) => s.is_empty(),
        Value::Object(_) => {
            crate::js::is_undefined(v) || is_num_marker(v, |x| x == 0.0 || x.is_nan())
        }
        Value::Array(_) => false,
    }
}

fn num_marker(v: &Value) -> Option<f64> {
    let Value::Object(m) = v else { return None };
    let s = m.get("$num")?.as_str()?;
    Some(match s {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        "-0" => -0.0,
        _ => s.parse().unwrap_or(f64::NAN),
    })
}

fn is_num_marker(v: &Value, f: impl Fn(f64) -> bool) -> bool {
    num_marker(v).is_some_and(f)
}

// `sql.literal("")` — literal can never be a function on the wire.
fn literal_call(sql: &Value) -> Result<Value, String> {
    match sql {
        Value::Null => Err("Cannot read properties of null (reading 'literal')".to_string()),
        v if crate::js::is_undefined(v) => {
            Err("Cannot read properties of undefined (reading 'literal')".to_string())
        }
        _ => Err("sql.literal is not a function".to_string()),
    }
}

// `sql` invoked as a tagged template / call — never a function on the wire.
fn sql_call() -> Result<Value, String> {
    Err("sql is not a function".to_string())
}

// `String(v)` for the 'in'-operator error operand.
fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        v if num_marker(v).is_some() => crate::js::number_to_string(num_marker(v).unwrap()),
        Value::Array(_) => "[object Object]".to_string(),
        _ => "[object Object]".to_string(),
    }
}

fn is_object_like(v: &Value) -> bool {
    // `key in v` requires an object; JSON objects and arrays qualify, the
    // `$undefined` marker is a plain object too — but `in` on the marker
    // object is legal, so treat it as an object.
    matches!(v, Value::Object(_) | Value::Array(_))
}

/// Port of `tokenStatsThreadFilter` (`apps/server/src/claudeTokenStats.ts:16`).
pub fn token_stats_thread_filter(
    sql: Value,
    _column: Value,
    scope: Option<Value>,
) -> Result<Value, String> {
    let scope = scope.unwrap_or_else(crate::js::undefined);
    if is_falsy(&scope) {
        return literal_call(&sql);
    }
    if !is_object_like(&scope) {
        return Err(format!(
            "Cannot use 'in' operator to search for 'threadId' in {}",
            js_string(&scope)
        ));
    }
    // Both branches evaluate `sql` as a tagged template.
    sql_call()
}

/// Port of `claudeTokenActivityCtes` (`apps/server/src/claudeTokenStats.ts:79`).
pub fn claude_token_activity_ctes(sql: Value, _scope: Option<Value>) -> Result<Value, String> {
    // The template's first substitution is
    // `tokenStatsLineageFilter(sql, sql.literal("a.thread_id"), scope)` and its
    // `sql.literal(...)` argument evaluates before anything else, so every
    // reachable failure is the `literal` call.
    literal_call(&sql)
}
