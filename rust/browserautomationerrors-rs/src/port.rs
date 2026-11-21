//! The Rust port of `packages/shared/src/browserAutomationErrors.ts`.
//!
//! Clean-room port: behavior inferred from recorded cases and the public contract shapes.
//! Data tables are kept as one `|`-delimited constant each and split at use, so the
//! port stays close to the source's size instead of one Rust line per entry.

#![forbid(unsafe_code)]

use serde_json::{Map, Value, json};

const CODES: &str = "BrowserUnauthorized|BrowserAuthorizationRequired|BrowserAuthorizationDenied|BrowserConduitAttachRejected|BrowserConduitDisconnected|BrowserTransportDisconnected|BrowserProviderProcessMismatch|BrowserTransportIsolationUnavailable|BrowserHostUnavailable|BrowserRuntimeDisconnected|BrowserRuntimeRestarted|BrowserDesktopMismatch|BrowserHomeAlreadyInUse|BrowserGuestAttachRejected|BrowserTabNotFound|BrowserTabScopeViolation|BrowserCapacityExceeded|BrowserQueueFull|BrowserTimeout|BrowserCancelled|BrowserInterruptedByHuman|BrowserDebuggerConflict|BrowserReconciliationRequired|BrowserStaleReference|BrowserWebMcpDiscoveryStale|BrowserTargetNotFound|BrowserTargetAmbiguous|BrowserTargetNotVisible|BrowserTargetNotEnabled|BrowserTargetObscured|BrowserTargetNotEditable|BrowserInvalidLocator|BrowserInputUnsupported|BrowserInvalidArguments|BrowserInvalidTimeout|BrowserCredentialTargetRequired|BrowserCredentialUseUnavailable|BrowserNavigationBlocked|BrowserNetworkBlocked|BrowserNavigationFailed|BrowserPopupBlocked|BrowserPopupOpenerUnsupported|BrowserDownloadApprovalRequired|BrowserEvaluationFailed|BrowserScriptApiUnavailable|BrowserEvaluationResultTooLarge|BrowserSnapshotTooLarge|BrowserScreenshotTooLarge|BrowserUploadPathOutsideWorkspace|BrowserUploadWorkspaceUnavailable|BrowserUploadFileUnsupported|BrowserMalformedResponse|BrowserSpoofedResponse|BrowserRequestConflict|BrowserAmbiguousResult|BrowserMcpNameConflict|BrowserProviderIsolationUnavailable";

const MESSAGES: &str = "Browser access is not authorized for this request.\u{1f}Browser authorization is required before this request can run.\u{1f}Browser authorization was denied.\u{1f}The browser conduit could not be attached to this provider session.\u{1f}The browser conduit disconnected before a result was delivered.\u{1f}The browser transport disconnected before a result was delivered.\u{1f}The provider process does not match the authorized browser session.\u{1f}The required isolated browser transport is unavailable.\u{1f}No browser host is available for this workspace.\u{1f}The browser runtime disconnected before a result was delivered.\u{1f}The browser runtime restarted before a result was delivered.\u{1f}The browser desktop does not match this operation.\u{1f}The browser home is already in use by another desktop instance.\u{1f}The browser tab was created, but its isolated runtime could not be attached.\u{1f}The requested browser tab was not found.\u{1f}The requested browser tab is outside this thread's scope.\u{1f}Browser capacity is currently exhausted.\u{1f}The browser operation queue is full.\u{1f}The browser operation timed out before a confirmed result was available.\u{1f}The browser operation was cancelled before a confirmed result was available.\u{1f}The browser operation was interrupted by human control.\u{1f}The browser tab is currently controlled by another debugger.\u{1f}The accepted browser routing inventory changed before the operation was admitted. Refresh browser tabs and retry.\u{1f}The snapshot reference is stale.\u{1f}The WebMCP discovery is stale. Discover the page's tools again before calling one.\u{1f}No browser element matched the locator.\u{1f}The locator matched more than one browser element. Use a unique locator.\u{1f}The matched browser element is not visible.\u{1f}The matched browser element is disabled.\u{1f}The matched browser element is covered by another page element.\u{1f}The matched browser element is not editable.\u{1f}The browser locator is invalid.\u{1f}The requested browser input is unsupported. Use a supported browser action.\u{1f}The tool arguments do not match the published input schema. No browser action ran. Correct the argument names, types and bounds before retrying; this does not mean form filling is unsupported.\u{1f}timeoutMs must be an integer from 100 to 30000 milliseconds. No browser action ran. Use timeoutMs: 30000 or omit it, and split longer workflows into smaller calls.\u{1f}Saved login form detection could not select a target. Agent password filling and generation are unavailable. Ask the user to sign in manually or import a browser session through Saved logins. Never ask for passwords in chat.\u{1f}Saved logins are metadata-only for agents. Password filling, generation and changes are unavailable. Ask the user to sign in manually or import a browser session through Saved logins. Never ask for passwords in chat.\u{1f}Browser navigation was rejected: browser tools accept only http/https URLs (localhost is allowed) or a resolvable annotationId. The user can open local HTML files from the integrated browser's address bar.\u{1f}The browser network request was blocked by policy.\u{1f}Browser navigation failed before a confirmed result was available.\u{1f}The browser popup was blocked by policy.\u{1f}The popup opener relationship is unsupported.\u{1f}The browser download requires explicit approval.\u{1f}Browser evaluation failed before a confirmed result was available. Inspect the current page and isolate the failing operation in a focused call; do not repeat unchanged calls. This error does not establish that input or sign-in buttons are blocked.\u{1f}The script used an unavailable browser API/global. Use page.getByRole(...), page.getByLabel(...), page.url(), and page.evaluate(() => ...) for document/window access. Use global snapshot(), not page.snapshot(). Wait with locator.waitFor, page.waitForURL, page.waitForFunction or page.waitForTimeout(ms); setTimeout and bare waitForTimeout are unavailable. Earlier actions may have completed; inspect their result before correcting the script. This is not a password-access error.\u{1f}The browser evaluation result exceeds the safe response limit.\u{1f}The browser snapshot exceeds the safe response limit.\u{1f}The browser screenshot exceeds the safe response limit.\u{1f}The requested upload file resolves outside the active workspace.\u{1f}No canonical workspace is available for browser file upload.\u{1f}The requested upload path is not a supported regular workspace file.\u{1f}Browser automation failed due to an internal error.\u{1f}The browser runtime returned a response with invalid correlation.\u{1f}The idempotency key is already associated with a different browser request.\u{1f}A prior browser action may have completed, but its result is no longer available. Observe with browser_tabs or browser_snapshot before deciding on a new intention and idempotency key.\u{1f}A browser tool name conflicts with another provider tool.\u{1f}The provider cannot satisfy browser isolation requirements.";

const PHASES: &str = "auth|queue|routing|runtime|navigation|snapshot|target|input|evaluate|cleanup";
const CAUSE_NAMES: &str =
    "Error|TypeError|RangeError|ReferenceError|SyntaxError|EvalError|URIError|TimeoutError";
/// Fixed invariants as `code=retryable.phase.effectMayHaveCommitted` entries.
const FIXED: &str = "BrowserReconciliationRequired=T.routing.F|BrowserWebMcpDiscoveryStale=T.input.F|BrowserTargetAmbiguous=F.target.F|BrowserTargetNotEnabled=F.target.F|BrowserTargetObscured=T.target.F|BrowserInputUnsupported=F.input.F|BrowserInvalidArguments=F.input.F|BrowserInvalidTimeout=F.input.F|BrowserScreenshotTooLarge=F.snapshot.F|BrowserUploadPathOutsideWorkspace=F.input.F|BrowserUploadWorkspaceUnavailable=F.input.F|BrowserUploadFileUnsupported=F.input.F|BrowserAmbiguousResult=F.runtime.T";

fn message(code: &str) -> &'static str {
    CODES
        .split('|')
        .zip(MESSAGES.split('\u{1f}'))
        .find(|(c, _)| *c == code)
        .map(|(_, m)| m)
        .unwrap_or("Browser automation failed due to an internal error.")
}
fn fixed_invariant(code: &str) -> Option<(bool, &'static str, bool)> {
    let e = FIXED
        .split('|')
        .find(|e| e.starts_with(&format!("{code}=")))?;
    let mut p = e.split('=').nth(1)?.split('.');
    Some((p.next()? == "T", p.next()?, p.next()? == "T"))
}

/// How a decoded value prints after `got` in a decode error: JS inspect style,
/// with `undefined` kept as a bare literal even inside objects.
fn fmt(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = crate::js::as_number(v).filter(|_| !v.is_number()) {
        return crate::js::number_to_string(n);
    }
    match v {
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or_default()),
        Value::Array(a) => format!("[{}]", a.iter().map(fmt).collect::<Vec<_>>().join(",")),
        Value::Object(o) => format!(
            "{{{}}}",
            o.iter()
                .map(|(k, v)| format!(
                    "{}:{}",
                    serde_json::to_string(k).unwrap_or_default(),
                    fmt(v)
                ))
                .collect::<Vec<_>>()
                .join(",")
        ),
        _ => serde_json::to_string(v).unwrap_or_default(),
    }
}

fn err_at(expect: &str, got: &Value, path: &str) -> String {
    format!("Expected {expect}, got {}\n  at {path}", fmt(got))
}
fn union(literals: &str) -> String {
    literals
        .split('|')
        .map(|l| format!("\"{l}\""))
        .collect::<Vec<_>>()
        .join(" | ")
}
fn expect_bool(input: &Map<String, Value>, key: &str) -> Result<bool, String> {
    let v = input.get(key).cloned().unwrap_or_else(crate::js::undefined);
    v.as_bool()
        .ok_or_else(|| err_at("boolean", &v, &format!("[\"{key}\"]")))
}

fn is_uuid(v: &str) -> bool {
    v.split('-').map(str::len).eq([8, 4, 4, 4, 12])
        && v.bytes().all(|b| b == b'-' || b.is_ascii_hexdigit())
}
fn is_api(v: &str) -> bool {
    v.split('.').all(|s| {
        s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || "_$".contains(c))
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || "_$".contains(c))
    })
}

/// An optional `string | undefined` field that must be a UUID when present.
fn check_id(input: &Map<String, Value>, key: &str) -> Result<(), String> {
    let Some(v) = input.get(key).filter(|v| !crate::js::is_undefined(v)) else {
        return Ok(());
    };
    if v.as_str().is_some_and(is_uuid) {
        return Ok(());
    }
    let e = if v.is_string() {
        "a UUID"
    } else {
        "string | undefined"
    };
    Err(err_at(e, v, &format!("[\"{key}\"]")))
}

/// Decodes `cause`; the decoded value keeps only the schema keys that were
/// present in the input, `undefined` markers included.
fn decode_cause(cause: &Value) -> Result<Value, String> {
    let Some(obj) = cause.as_object() else {
        return Err(err_at("object | undefined", cause, "[\"cause\"]"));
    };
    let mut decoded = Map::new();
    for key in ["name", "api"] {
        let Some(v) = obj.get(key) else {
            continue;
        };
        decoded.insert(key.into(), v.clone());
        if crate::js::is_undefined(v) {
            continue;
        }
        let expect: String = match (key, v.as_str()) {
            ("name", s) if !s.is_some_and(|s| CAUSE_NAMES.split('|').any(|l| l == s)) => {
                format!("{} | undefined", union(CAUSE_NAMES))
            }
            ("api", None) => "string | undefined".into(),
            ("api", Some(s)) if s.is_empty() || s.len() > 64 => "<filter>".into(),
            ("api", Some(s)) if !is_api(s) => {
                "a string matching the RegExp ^[A-Za-z_$][\\w$]*(?:\\.[A-Za-z_$][\\w$]*)*$".into()
            }
            _ => continue,
        };
        return Err(err_at(&expect, v, &format!("[\"cause\"][\"{key}\"]")));
    }
    let m = obj
        .get("message")
        .ok_or("Missing key\n  at [\"cause\"][\"message\"]")?;
    match m.as_str().filter(|s| !s.is_empty() && s.len() <= 320) {
        Some(_) => decoded.insert("message".into(), m.clone()),
        _ if m.is_string() => return Err(err_at("<filter>", m, "[\"cause\"][\"message\"]")),
        _ => return Err(err_at("string", m, "[\"cause\"][\"message\"]")),
    };
    Ok(Value::Object(decoded))
}

fn make_error(input: &Value) -> Result<Value, String> {
    let code = match input {
        Value::Null => return Err("Cannot read properties of null (reading 'code')".into()),
        v if crate::js::is_undefined(v) => {
            return Err("Cannot read properties of undefined (reading 'code')".into());
        }
        Value::Object(m) => m.get("code").cloned().unwrap_or_else(crate::js::undefined),
        _ => crate::js::undefined(),
    };
    let code_str = code
        .as_str()
        .filter(|s| CODES.split('|').any(|c| c == *s))
        .ok_or_else(|| err_at(&union(CODES), &code, "[\"code\"]"))?;
    let obj = input.as_object().cloned().unwrap_or_default();
    let fixed = fixed_invariant(code_str);
    let (retryable, phase, emc): (bool, String, bool) = match fixed {
        Some((r, p, e)) => (r, p.to_string(), e),
        None => {
            let retryable = expect_bool(&obj, "retryable")?;
            let phase_v = obj
                .get("phase")
                .cloned()
                .unwrap_or_else(crate::js::undefined);
            let Some(phase) = phase_v
                .as_str()
                .filter(|s| PHASES.split('|').any(|p| p == *s))
            else {
                return Err(err_at(&union(PHASES), &phase_v, "[\"phase\"]"));
            };
            (
                retryable,
                phase.to_string(),
                expect_bool(&obj, "effectMayHaveCommitted")?,
            )
        }
    };
    check_id(&obj, "operationId")?;
    check_id(&obj, "tabId")?;
    let cause = match obj.get("cause") {
        _ if fixed.is_some() => None,
        Some(v) if !crate::js::is_undefined(v) => Some(decode_cause(v)?),
        _ => None,
    };
    let mut out = Map::new();
    out.insert("code".into(), Value::String(code_str.into()));
    out.insert("message".into(), Value::String(message(code_str).into()));
    out.insert("retryable".into(), Value::Bool(retryable));
    out.insert("phase".into(), Value::String(phase));
    out.extend(["operationId", "tabId"].into_iter().filter_map(|k| {
        obj.get(k)
            .filter(|v| !crate::js::is_undefined(v))
            .map(|v| (k.into(), v.clone()))
    }));
    out.insert("effectMayHaveCommitted".into(), Value::Bool(emc));
    if let Some(c) = cause {
        out.insert("cause".into(), c);
    }
    let error = Value::Object(out);
    if error.get("cause").is_some()
        && !matches!(
            code_str,
            "BrowserEvaluationFailed" | "BrowserScriptApiUnavailable"
        )
    {
        return Err(format!("Expected <filter>, got {}", fmt(&error)));
    }
    Ok(error)
}

/// Port of `browserInputErrorCode` (`packages/shared/src/browserAutomationErrors.ts:12`).
pub fn browser_input_error_code(arguments_value: Value) -> Result<Value, String> {
    let code = match arguments_value
        .get("timeoutMs")
        .filter(|v| !crate::js::is_undefined(v))
    {
        None => "BrowserInvalidArguments",
        Some(v) => match crate::js::as_number(v) {
            Some(n) if n.fract() == 0.0 && (100.0..=30000.0).contains(&n) => {
                "BrowserInvalidArguments"
            }
            _ => "BrowserInvalidTimeout",
        },
    };
    Ok(json!(code))
}

/// Port of `makeBrowserAutomationError` (`packages/shared/src/browserAutomationErrors.ts:34`).
pub fn make_browser_automation_error(input: Value) -> Result<Value, String> {
    make_error(&input)
}

/// Port of `makeBrowserMcpToolErrorEnvelope` (`packages/shared/src/browserAutomationErrors.ts:57`).
pub fn make_browser_mcp_tool_error_envelope(input: Value) -> Result<Value, String> {
    Ok(json!({"type": "synara_browser_error", "version": 1, "error": make_error(&input)?}))
}
