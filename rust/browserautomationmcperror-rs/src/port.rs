//! The Rust port of `packages/shared/src/browserAutomationMcpError.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const CODES: &[&str] = &[
    "BrowserUnauthorized",
    "BrowserAuthorizationRequired",
    "BrowserAuthorizationDenied",
    "BrowserConduitAttachRejected",
    "BrowserConduitDisconnected",
    "BrowserTransportDisconnected",
    "BrowserProviderProcessMismatch",
    "BrowserTransportIsolationUnavailable",
    "BrowserHostUnavailable",
    "BrowserRuntimeDisconnected",
    "BrowserRuntimeRestarted",
    "BrowserDesktopMismatch",
    "BrowserHomeAlreadyInUse",
    "BrowserGuestAttachRejected",
    "BrowserTabNotFound",
    "BrowserTabScopeViolation",
    "BrowserCapacityExceeded",
    "BrowserQueueFull",
    "BrowserTimeout",
    "BrowserCancelled",
    "BrowserInterruptedByHuman",
    "BrowserDebuggerConflict",
    "BrowserReconciliationRequired",
    "BrowserStaleReference",
    "BrowserWebMcpDiscoveryStale",
    "BrowserTargetNotFound",
    "BrowserTargetAmbiguous",
    "BrowserTargetNotVisible",
    "BrowserTargetNotEnabled",
    "BrowserTargetObscured",
    "BrowserTargetNotEditable",
    "BrowserInvalidLocator",
    "BrowserInputUnsupported",
    "BrowserInvalidArguments",
    "BrowserInvalidTimeout",
    "BrowserCredentialTargetRequired",
    "BrowserCredentialUseUnavailable",
    "BrowserNavigationBlocked",
    "BrowserNetworkBlocked",
    "BrowserNavigationFailed",
    "BrowserPopupBlocked",
    "BrowserPopupOpenerUnsupported",
    "BrowserDownloadApprovalRequired",
    "BrowserEvaluationFailed",
    "BrowserScriptApiUnavailable",
    "BrowserEvaluationResultTooLarge",
    "BrowserSnapshotTooLarge",
    "BrowserScreenshotTooLarge",
    "BrowserUploadPathOutsideWorkspace",
    "BrowserUploadWorkspaceUnavailable",
    "BrowserUploadFileUnsupported",
    "BrowserMalformedResponse",
    "BrowserSpoofedResponse",
    "BrowserRequestConflict",
    "BrowserAmbiguousResult",
    "BrowserMcpNameConflict",
    "BrowserProviderIsolationUnavailable",
];

const PHASES: &[&str] = &[
    "auth",
    "queue",
    "routing",
    "runtime",
    "navigation",
    "snapshot",
    "target",
    "input",
    "evaluate",
    "cleanup",
];

const CAUSE_NAMES: &[&str] = &[
    "Error",
    "TypeError",
    "RangeError",
    "ReferenceError",
    "SyntaxError",
    "EvalError",
    "URIError",
    "TimeoutError",
];

const MESSAGES: &[(&str, &str)] = &[
    (
        "BrowserUnauthorized",
        "Browser access is not authorized for this request.",
    ),
    (
        "BrowserAuthorizationRequired",
        "Browser authorization is required before this request can run.",
    ),
    (
        "BrowserAuthorizationDenied",
        "Browser authorization was denied.",
    ),
    (
        "BrowserConduitAttachRejected",
        "The browser conduit could not be attached to this provider session.",
    ),
    (
        "BrowserConduitDisconnected",
        "The browser conduit disconnected before a result was delivered.",
    ),
    (
        "BrowserTransportDisconnected",
        "The browser transport disconnected before a result was delivered.",
    ),
    (
        "BrowserProviderProcessMismatch",
        "The provider process does not match the authorized browser session.",
    ),
    (
        "BrowserTransportIsolationUnavailable",
        "The required isolated browser transport is unavailable.",
    ),
    (
        "BrowserHostUnavailable",
        "No browser host is available for this workspace.",
    ),
    (
        "BrowserRuntimeDisconnected",
        "The browser runtime disconnected before a result was delivered.",
    ),
    (
        "BrowserRuntimeRestarted",
        "The browser runtime restarted before a result was delivered.",
    ),
    (
        "BrowserDesktopMismatch",
        "The browser desktop does not match this operation.",
    ),
    (
        "BrowserHomeAlreadyInUse",
        "The browser home is already in use by another desktop instance.",
    ),
    (
        "BrowserGuestAttachRejected",
        "The browser tab was created, but its isolated runtime could not be attached.",
    ),
    (
        "BrowserTabNotFound",
        "The requested browser tab was not found.",
    ),
    (
        "BrowserTabScopeViolation",
        "The requested browser tab is outside this thread's scope.",
    ),
    (
        "BrowserCapacityExceeded",
        "Browser capacity is currently exhausted.",
    ),
    ("BrowserQueueFull", "The browser operation queue is full."),
    (
        "BrowserTimeout",
        "The browser operation timed out before a confirmed result was available.",
    ),
    (
        "BrowserCancelled",
        "The browser operation was cancelled before a confirmed result was available.",
    ),
    (
        "BrowserInterruptedByHuman",
        "The browser operation was interrupted by human control.",
    ),
    (
        "BrowserDebuggerConflict",
        "The browser tab is currently controlled by another debugger.",
    ),
    (
        "BrowserReconciliationRequired",
        "The accepted browser routing inventory changed before the operation was admitted. Refresh browser tabs and retry.",
    ),
    ("BrowserStaleReference", "The snapshot reference is stale."),
    (
        "BrowserWebMcpDiscoveryStale",
        "The WebMCP discovery is stale. Discover the page's tools again before calling one.",
    ),
    (
        "BrowserTargetNotFound",
        "No browser element matched the locator.",
    ),
    (
        "BrowserTargetAmbiguous",
        "The locator matched more than one browser element. Use a unique locator.",
    ),
    (
        "BrowserTargetNotVisible",
        "The matched browser element is not visible.",
    ),
    (
        "BrowserTargetNotEnabled",
        "The matched browser element is disabled.",
    ),
    (
        "BrowserTargetObscured",
        "The matched browser element is covered by another page element.",
    ),
    (
        "BrowserTargetNotEditable",
        "The matched browser element is not editable.",
    ),
    ("BrowserInvalidLocator", "The browser locator is invalid."),
    (
        "BrowserInputUnsupported",
        "The requested browser input is unsupported. Use a supported browser action.",
    ),
    (
        "BrowserInvalidArguments",
        "The tool arguments do not match the published input schema. No browser action ran. Correct the argument names, types and bounds before retrying; this does not mean form filling is unsupported.",
    ),
    (
        "BrowserInvalidTimeout",
        "timeoutMs must be an integer from 100 to 30000 milliseconds. No browser action ran. Use timeoutMs: 30000 or omit it, and split longer workflows into smaller calls.",
    ),
    (
        "BrowserCredentialTargetRequired",
        "Saved login form detection could not select a target. Agent password filling and generation are unavailable. Ask the user to sign in manually or import a browser session through Saved logins. Never ask for passwords in chat.",
    ),
    (
        "BrowserCredentialUseUnavailable",
        "Saved logins are metadata-only for agents. Password filling, generation and changes are unavailable. Ask the user to sign in manually or import a browser session through Saved logins. Never ask for passwords in chat.",
    ),
    (
        "BrowserNavigationBlocked",
        "Browser navigation was rejected: browser tools accept only http/https URLs (localhost is allowed) or a resolvable annotationId. The user can open local HTML files from the integrated browser's address bar.",
    ),
    (
        "BrowserNetworkBlocked",
        "The browser network request was blocked by policy.",
    ),
    (
        "BrowserNavigationFailed",
        "Browser navigation failed before a confirmed result was available.",
    ),
    (
        "BrowserPopupBlocked",
        "The browser popup was blocked by policy.",
    ),
    (
        "BrowserPopupOpenerUnsupported",
        "The popup opener relationship is unsupported.",
    ),
    (
        "BrowserDownloadApprovalRequired",
        "The browser download requires explicit approval.",
    ),
    (
        "BrowserEvaluationFailed",
        "Browser evaluation failed before a confirmed result was available. Inspect the current page and isolate the failing operation in a focused call; do not repeat unchanged calls. This error does not establish that input or sign-in buttons are blocked.",
    ),
    (
        "BrowserScriptApiUnavailable",
        "The script used an unavailable browser API/global. Use page.getByRole(...), page.getByLabel(...), page.url(), and page.evaluate(() => ...) for document/window access. Use global snapshot(), not page.snapshot(). Wait with locator.waitFor, page.waitForURL, page.waitForFunction or page.waitForTimeout(ms); setTimeout and bare waitForTimeout are unavailable. Earlier actions may have completed; inspect their result before correcting the script. This is not a password-access error.",
    ),
    (
        "BrowserEvaluationResultTooLarge",
        "The browser evaluation result exceeds the safe response limit.",
    ),
    (
        "BrowserSnapshotTooLarge",
        "The browser snapshot exceeds the safe response limit.",
    ),
    (
        "BrowserScreenshotTooLarge",
        "The browser screenshot exceeds the safe response limit.",
    ),
    (
        "BrowserUploadPathOutsideWorkspace",
        "The requested upload file resolves outside the active workspace.",
    ),
    (
        "BrowserUploadWorkspaceUnavailable",
        "No canonical workspace is available for browser file upload.",
    ),
    (
        "BrowserUploadFileUnsupported",
        "The requested upload path is not a supported regular workspace file.",
    ),
    (
        "BrowserMalformedResponse",
        "Browser automation failed due to an internal error.",
    ),
    (
        "BrowserSpoofedResponse",
        "The browser runtime returned a response with invalid correlation.",
    ),
    (
        "BrowserRequestConflict",
        "The idempotency key is already associated with a different browser request.",
    ),
    (
        "BrowserAmbiguousResult",
        "A prior browser action may have completed, but its result is no longer available. Observe with browser_tabs or browser_snapshot before deciding on a new intention and idempotency key.",
    ),
    (
        "BrowserMcpNameConflict",
        "A browser tool name conflicts with another provider tool.",
    ),
    (
        "BrowserProviderIsolationUnavailable",
        "The provider cannot satisfy browser isolation requirements.",
    ),
];

const FIXED: &[(&str, bool, &str, bool)] = &[
    ("BrowserReconciliationRequired", true, "routing", false),
    ("BrowserWebMcpDiscoveryStale", true, "input", false),
    ("BrowserTargetAmbiguous", false, "target", false),
    ("BrowserTargetNotEnabled", false, "target", false),
    ("BrowserTargetObscured", true, "target", false),
    ("BrowserInputUnsupported", false, "input", false),
    ("BrowserInvalidArguments", false, "input", false),
    ("BrowserInvalidTimeout", false, "input", false),
    ("BrowserScreenshotTooLarge", false, "snapshot", false),
    ("BrowserUploadPathOutsideWorkspace", false, "input", false),
    ("BrowserUploadWorkspaceUnavailable", false, "input", false),
    ("BrowserUploadFileUnsupported", false, "input", false),
    ("BrowserAmbiguousResult", false, "runtime", true),
];
fn is_fn_marker(v: &Value) -> bool {
    matches!(v, Value::Object(m) if m.len() == 1 && m.contains_key("$function"))
}

fn repr(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\"")),
        v if crate::js::is_undefined(v) => "undefined".into(),
        v if crate::js::as_number(v).is_some() => {
            crate::js::number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN))
        }
        v if is_fn_marker(v) => "undefined".into(),
        Value::Array(items) => {
            let inner: Vec<String> = items
                .iter()
                .map(|e| {
                    if crate::js::is_undefined(e) || is_fn_marker(e) {
                        "null".into()
                    } else {
                        repr(e)
                    }
                })
                .collect();
            format!("[{}]", inner.join(","))
        }
        Value::Object(map) => {
            let inner: Vec<String> = map
                .iter()
                .filter(|(_, val)| !crate::js::is_undefined(val) && !is_fn_marker(val))
                .map(|(k, val)| {
                    let vs = match val {
                        Value::Object(o) if o.len() == 1 && o.contains_key("$num") => {
                            match crate::js::as_number(val) {
                                Some(n) if n.is_finite() => {
                                    crate::js::number_to_string(if n == 0.0 { 0.0 } else { n })
                                }
                                Some(_) | None => "null".into(),
                            }
                        }
                        _ => repr(val),
                    };
                    format!("{}:{}", serde_json::to_string(k).unwrap_or_default(), vs)
                })
                .collect();
            format!("{{{}}}", inner.join(","))
        }
    }
}

fn at(path: &[&str]) -> String {
    format!(
        "\n  at {}",
        path.iter()
            .map(|p| format!("[\"{p}\"]"))
            .collect::<String>()
    )
}

fn literals_err(lits: &[&str], v: &Value, path: &[&str], optional: bool) -> String {
    let mut e = format!(
        "Expected {}, got {}",
        lits.iter()
            .map(|l| serde_json::to_string(l).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(" | "),
        repr(v)
    );
    if optional {
        let pos = e.rfind(", got ").unwrap_or(e.len());
        e.insert_str(pos, " | undefined");
    }
    e + &at(path)
}

fn canonical_stringify(v: &Value) -> Result<String, String> {
    let mut out = String::new();
    write_canonical(v, &mut out)?;
    Ok(out)
}

fn write_canonical(v: &Value, out: &mut String) -> Result<(), String> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => {
            let x = n.as_f64().unwrap_or(f64::NAN);
            if !x.is_finite() {
                return Err("Canonical JSON numbers must be finite".to_string());
            }
            out.push_str(&crate::js::number_to_string(x));
        }
        Value::String(s) => out.push_str(&serde_json::to_string(s).unwrap_or_default()),
        Value::Array(items) => {
            out.push('[');
            for (i, e) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(e, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            if crate::js::is_undefined(v) || is_fn_marker(v) || crate::js::as_number(v).is_some() {
                return Err("Value is not canonical JSON".to_string());
            }
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            let mut first = true;
            for k in keys {
                let val = &map[k];
                if crate::js::is_undefined(val) || is_fn_marker(val) {
                    return Err("Value is not canonical JSON".to_string());
                }
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_canonical(val, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn utf8_len(s: &str) -> usize {
    s.len()
}

fn is_uuid(s: &str) -> bool {
    let hex = |c: u8| c.is_ascii_hexdigit();
    let b = s.as_bytes();
    b.len() == 36
        && b[8] == b'-'
        && b[13] == b'-'
        && b[18] == b'-'
        && b[23] == b'-'
        && b[..8].iter().all(|&c| hex(c))
        && b[9..13].iter().all(|&c| hex(c))
        && b[14..18].iter().all(|&c| hex(c))
        && b[19..23].iter().all(|&c| hex(c))
        && b[24..36].iter().all(|&c| hex(c))
        && (matches!(b[19], b'8' | b'9' | b'a' | b'b' | b'A' | b'B')
            || b.iter().all(|&c| c == b'0' || c == b'-'))
}

fn is_api_name(s: &str) -> bool {
    for seg in s.split('.') {
        let mut chars = seg.chars();
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$' => {}
            _ => return false,
        }
        if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$') {
            return false;
        }
    }
    !s.is_empty()
}

fn get_field<'a>(map: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a Value> {
    map.get(key)
}

fn msg(code: &str) -> Option<&'static str> {
    MESSAGES.iter().find(|(c, _)| *c == code).map(|(_, m)| *m)
}

fn fixed_inv(code: &str) -> Option<(bool, &'static str, bool)> {
    FIXED
        .iter()
        .find(|(c, ..)| *c == code)
        .map(|(_, r, p, e)| (*r, *p, *e))
}

/// Port of `encodeBrowserMcpToolError` (`packages/shared/src/browserAutomationMcpError.ts:13`).
pub fn encode_browser_mcp_tool_error(error: Value) -> Result<Value, String> {
    let err = match &error {
        v if crate::js::is_undefined(v) || crate::js::as_number(v).is_some() => {
            return Err(format!("Expected object, got {}", repr(v)));
        }
        Value::Object(_) => &error,
        v => return Err(format!("Expected object, got {}", repr(v))),
    };
    let map = err.as_object().cloned().unwrap_or_default();

    // code
    let code_v =
        get_field(&map, "code").ok_or_else(|| "Missing key".to_string() + &at(&["code"]))?;
    let code = match code_v {
        Value::String(s) if CODES.contains(&s.as_str()) => s.clone(),
        v => return Err(literals_err(CODES, v, &["code"], false)),
    };
    // message
    let message_v =
        get_field(&map, "message").ok_or_else(|| "Missing key".to_string() + &at(&["message"]))?;
    let message = match message_v {
        Value::String(s) => {
            if utf8_len(s) < 1 || utf8_len(s) > 512 {
                return Err(
                    format!("Expected <filter>, got {}", repr(message_v)) + &at(&["message"])
                );
            }
            s.clone()
        }
        v => return Err(format!("Expected string, got {}", repr(v)) + &at(&["message"])),
    };
    // retryable
    let retryable_v = get_field(&map, "retryable")
        .ok_or_else(|| "Missing key".to_string() + &at(&["retryable"]))?;
    let retryable = match retryable_v {
        Value::Bool(b) => *b,
        v => return Err(format!("Expected boolean, got {}", repr(v)) + &at(&["retryable"])),
    };
    // phase
    let phase_v =
        get_field(&map, "phase").ok_or_else(|| "Missing key".to_string() + &at(&["phase"]))?;
    let phase = match phase_v {
        Value::String(s) if PHASES.contains(&s.as_str()) => s.clone(),
        v => return Err(literals_err(PHASES, v, &["phase"], false)),
    };
    // operationId (optional)
    let operation_id = match get_field(&map, "operationId") {
        None => None,
        Some(v) if crate::js::is_undefined(v) => Some(crate::js::undefined()),
        Some(v @ Value::String(s)) => {
            if !is_uuid(s) {
                return Err(format!("Expected a UUID, got {}", repr(v)) + &at(&["operationId"]));
            }
            Some(v.clone())
        }
        Some(v) => {
            return Err(
                format!("Expected string | undefined, got {}", repr(v)) + &at(&["operationId"])
            );
        }
    };
    // tabId (optional)
    let tab_id = match get_field(&map, "tabId") {
        None => None,
        Some(v) if crate::js::is_undefined(v) => Some(crate::js::undefined()),
        Some(v @ Value::String(s)) => {
            if !is_uuid(s) {
                return Err(format!("Expected a UUID, got {}", repr(v)) + &at(&["tabId"]));
            }
            Some(v.clone())
        }
        Some(v) => {
            return Err(format!("Expected string | undefined, got {}", repr(v)) + &at(&["tabId"]));
        }
    };
    // effectMayHaveCommitted
    let commit_v = get_field(&map, "effectMayHaveCommitted")
        .ok_or_else(|| "Missing key".to_string() + &at(&["effectMayHaveCommitted"]))?;
    let commit = match commit_v {
        Value::Bool(b) => *b,
        v => {
            return Err(
                format!("Expected boolean, got {}", repr(v)) + &at(&["effectMayHaveCommitted"])
            );
        }
    };
    // cause (optional)
    let cause = match get_field(&map, "cause") {
        None => None,
        Some(v) if crate::js::is_undefined(v) => Some(crate::js::undefined()),
        Some(Value::Object(cm)) => {
            // name (optional literals)
            let name = match cm.get("name") {
                None => None,
                Some(v) if crate::js::is_undefined(v) => Some(crate::js::undefined()),
                Some(v @ Value::String(s)) => {
                    if !CAUSE_NAMES.contains(&s.as_str()) {
                        return Err(literals_err(CAUSE_NAMES, v, &["cause", "name"], false));
                    }
                    Some(v.clone())
                }
                Some(v) => {
                    return Err(literals_err(CAUSE_NAMES, v, &["cause", "name"], true));
                }
            };
            // api (optional bounded + pattern)
            let api = match cm.get("api") {
                None => None,
                Some(v) if crate::js::is_undefined(v) => Some(crate::js::undefined()),
                Some(v @ Value::String(s)) => {
                    let bl = utf8_len(s);
                    if !(1..=64).contains(&bl) {
                        return Err(
                            format!("Expected <filter>, got {}", repr(v)) + &at(&["cause", "api"])
                        );
                    }
                    if !is_api_name(s) {
                        return Err(format!(
                            "Expected a string matching the RegExp ^[A-Za-z_$][\\w$]*(?:\\.[A-Za-z_$][\\w$]*)*$, got {}",
                            repr(v)
                        ) + &at(&["cause", "api"]));
                    }
                    Some(v.clone())
                }
                Some(v) => {
                    return Err(format!("Expected string | undefined, got {}", repr(v))
                        + &at(&["cause", "api"]));
                }
            };
            // message (required bounded)
            let cmsg = match cm.get("message") {
                None => {
                    return Err("Missing key".to_string() + &at(&["cause", "message"]));
                }
                Some(v @ Value::String(s)) => {
                    let bl = utf8_len(s);
                    if !(1..=320).contains(&bl) {
                        return Err(format!("Expected <filter>, got {}", repr(v))
                            + &at(&["cause", "message"]));
                    }
                    v.clone()
                }
                Some(v) => {
                    return Err(
                        format!("Expected string, got {}", repr(v)) + &at(&["cause", "message"])
                    );
                }
            };
            let mut decoded = serde_json::Map::new();
            if let Some(n) = name {
                decoded.insert("name".into(), n);
            }
            if let Some(a) = api {
                decoded.insert("api".into(), a);
            }
            decoded.insert("message".into(), cmsg);
            Some(Value::Object(decoded))
        }
        Some(v) => {
            return Err(format!("Expected object | undefined, got {}", repr(v)) + &at(&["cause"]));
        }
    };

    // decoded object in schema field order
    let mut decoded = serde_json::Map::new();
    decoded.insert("code".into(), Value::String(code.clone()));
    decoded.insert("message".into(), Value::String(message.clone()));
    decoded.insert("retryable".into(), Value::Bool(retryable));
    decoded.insert("phase".into(), Value::String(phase.clone()));
    if let Some(v) = &operation_id {
        decoded.insert("operationId".into(), v.clone());
    }
    if let Some(v) = &tab_id {
        decoded.insert("tabId".into(), v.clone());
    }
    decoded.insert("effectMayHaveCommitted".into(), Value::Bool(commit));
    if let Some(v) = &cause {
        decoded.insert("cause".into(), v.clone());
    }
    let decoded = Value::Object(decoded);

    // whole-object filter
    let filter_ok = message == msg(&code).unwrap_or_default()
        && (cause.is_none()
            || cause.as_ref().is_some_and(crate::js::is_undefined)
            || matches!(
                code.as_str(),
                "BrowserEvaluationFailed" | "BrowserScriptApiUnavailable"
            ))
        && match fixed_inv(&code) {
            None => true,
            Some((r, p, e)) => retryable == r && phase == p && commit == e,
        };
    // present-but-undefined optionals pass the filter (cause === undefined check treats them
    // as present — but recorded shows canonicalize fails anyway; the filter still runs first).
    if !filter_ok {
        return Err(format!("Expected <filter>, got {}", repr(&decoded)));
    }

    // canonicalize — present-undefined optionals die here
    let envelope = serde_json::json!({
        "type": "synara_browser_error",
        "version": 1,
        "error": decoded,
    });
    let text = canonical_stringify(&envelope)?;
    if utf8_len(&text) > 8 * 1024 {
        return Err("Browser MCP error envelope exceeds 8 KiB".to_string());
    }
    Ok(serde_json::json!({
        "isError": true,
        "content": [{"type": "text", "text": text}],
    }))
}
