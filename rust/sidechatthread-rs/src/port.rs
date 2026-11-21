//! The Rust port of `packages/shared/src/sidechatThread.ts`.
//!
//! Behavior inferred clean-room from recorded cases and black-box probes:
//! a thread is a sidechat when it has a truthy `sidechatSourceThreadId` or a
//! non-nullish `sidechatContext`; it is standalone when it has the context but
//! no truthy source thread id. GitHub-item contexts match on
//! `kind === "github-item"` plus strict `number`/`repository` equality.

#![forbid(unsafe_code)]

use serde_json::Value;

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !crate::js::is_undefined(v),
    }
}

fn non_nullish(v: Option<&Value>) -> bool {
    v.is_some_and(|v| !matches!(v, Value::Null) && !crate::js::is_undefined(v))
}

fn field<'a>(v: &'a Value, name: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(name))
}

fn check_thread(thread: &Value) -> Result<(), String> {
    match thread {
        Value::Null => {
            Err("Cannot read properties of null (reading 'sidechatSourceThreadId')".to_string())
        }
        v if crate::js::is_undefined(v) => Err(
            "Cannot read properties of undefined (reading 'sidechatSourceThreadId')".to_string(),
        ),
        _ => Ok(()),
    }
}

/// Port of `isSidechatThread` (`packages/shared/src/sidechatThread.ts:5`).
pub fn is_sidechat_thread(thread: Value) -> Result<bool, String> {
    check_thread(&thread)?;
    Ok(field(&thread, "sidechatSourceThreadId").is_some_and(truthy)
        || non_nullish(field(&thread, "sidechatContext")))
}

/// Port of `isStandaloneSidechatThread` (`packages/shared/src/sidechatThread.ts:9`).
pub fn is_standalone_sidechat_thread(thread: Value) -> Result<bool, String> {
    check_thread(&thread)?;
    Ok(non_nullish(field(&thread, "sidechatContext"))
        && !field(&thread, "sidechatSourceThreadId").is_some_and(truthy))
}

/// Port of `sidechatContextMatchesGitHubItem` (`packages/shared/src/sidechatThread.ts:26`).
pub fn sidechat_context_matches_git_hub_item(
    context: Option<Value>,
    item: Value,
) -> Result<bool, String> {
    let Some(Value::Object(ctx)) = &context else {
        return Ok(false);
    };
    if ctx.get("kind") != Some(&Value::String("github-item".to_string())) {
        return Ok(false);
    }
    let (number, repository) = match &item {
        Value::Null => return Err("Cannot read properties of null (reading 'number')".to_string()),
        v if crate::js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'number')".to_string());
        }
        Value::Object(o) => (o.get("number"), o.get("repository")),
        _ => (None, None),
    };
    Ok(ctx.get("number") == number && ctx.get("repository") == repository)
}
