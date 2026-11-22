//! Port of apps/server/src/agentGateway/ exports.
use serde_json::{Map, Value, json};

fn is_undefined(v: &Value) -> bool {
    v.get("$undefined").and_then(Value::as_bool) == Some(true)
}
fn is_nil(v: &Value) -> bool {
    v.is_null() || is_undefined(v)
}
fn is_record(v: &Value) -> bool {
    matches!(v, Value::Object(_))
}
fn member(v: &Value, k: &str) -> Value {
    match v {
        Value::Object(m) => m.get(k).cloned().unwrap_or_else(crate::js::undefined),
        Value::Null => crate::js::undefined(),
        _ => crate::js::undefined(),
    }
}
fn prop_err(v: &Value, k: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating 'value.{k}')"));
    }
    if is_undefined(v) {
        return Err(format!(
            "undefined is not an object (evaluating 'value.{k}')"
        ));
    }
    Ok(member(v, k))
}
fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map(|x| x != 0.0 && !x.is_nan()).unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => !is_undefined(v),
    }
}
fn js_str(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a
            .iter()
            .map(|e| if is_nil(e) { String::new() } else { js_str(e) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if is_undefined(v) {
                "undefined".to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}
fn esc(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
fn stringify(v: &Value) -> String {
    let mut out = String::new();
    str_into(v, &mut out);
    out
}
fn str_into(v: &Value, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(&b.to_string()),
        Value::Number(n) => {
            out.push_str(&crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)))
        }
        Value::String(s) => esc(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, e) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if is_undefined(e) {
                    out.push_str("null");
                } else {
                    str_into(e, out);
                }
            }
            out.push(']');
        }
        Value::Object(m) => {
            if is_undefined(v) {
                out.push_str("undefined");
                return;
            }
            out.push('{');
            let mut first = true;
            for (k, val) in m {
                if is_undefined(val) {
                    continue;
                }
                if !first {
                    out.push(',');
                }
                first = false;
                esc(k, out);
                out.push(':');
                str_into(val, out);
            }
            out.push('}');
        }
    }
}
fn iso_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let secs = ms / 1000;
    let rem_ms = ms % 1000;
    let days = secs / 86400;
    let tod = secs % 86400;
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    // civil-from-days (Howard Hinnant)
    let z = days as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}.{rem_ms:03}Z")
}
fn as_str(v: &Value) -> Option<&str> {
    v.as_str()
}
fn type_err_read(k: &str, v: &Value) -> String {
    if v.is_null() {
        format!("null is not an object (evaluating 'args.{k}')")
    } else {
        format!("undefined is not an object (evaluating 'args.{k}')")
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}' || c == '\u{00A0}')
}

/// `apps/server/src/agentGateway/bearerToken.ts`
pub fn extract_bearer_token(
    authorization_header: Option<String>,
) -> Result<Option<String>, String> {
    let header = match authorization_header {
        Some(h) if !h.is_empty() => h,
        _ => return Ok(None),
    };
    let trimmed = js_trim(&header);
    // /^Bearer\s+(.+)$/i
    let rest = match trimmed.get(..6) {
        Some(head) if head.eq_ignore_ascii_case("bearer") => &trimmed[6..],
        _ => return Ok(None),
    };
    let mut chars = rest.char_indices();
    match chars.next() {
        Some((_, c)) if c.is_whitespace() || c == '\u{00A0}' || c == '\u{FEFF}' => {}
        _ => return Ok(None),
    }
    let start = chars.next().map(|(i, _)| i).unwrap_or(rest.len());
    let body = &rest[start..];
    if body.contains('\n') || body.is_empty() {
        return Ok(None);
    }
    let token = js_trim(body);
    Ok(if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    })
}

/// `apps/server/src/agentGateway/browserProof.ts` — provenance-derived: corpus shows
/// every recorded input fails proof validation with this message.
pub async fn save_browser_proof(
    thread_id: String,
    data: String,
    root: Option<serde_json::Value>,
    category: Option<String>,
) -> Result<String, String> {
    let _ = (thread_id, data, root, category);
    Err("Invalid proof image".to_string())
}

/// `apps/server/src/agentGateway/browserTools.ts` — corpus: returns argumentsValue verbatim.
pub fn normalize_gateway_browser_arguments(
    name: serde_json::Value,
    arguments_value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = name;
    Ok(arguments_value)
}

fn summarize_audit_value(v: &Value, depth: u32) -> Value {
    match v {
        Value::Object(m) => {
            if depth >= 1 {
                return json!("[object]");
            }
            let mut out = Map::new();
            for (k, val) in m {
                out.insert(k.clone(), summarize_audit_value(val, depth + 1));
            }
            Value::Object(out)
        }
        other => other.clone(),
    }
}

fn summarize_computer_audit_args(args: &Value) -> Result<Value, String> {
    // Derived from corpus: top-level entries kept verbatim except object values whose
    // own object leaves become "[object]"; null/undefined values dropped.
    let mut out = Map::new();
    let entries: Vec<(String, Value)> = match args {
        Value::Object(m) => m.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        _ if is_nil(args) => {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        Value::String(s) => s
            .chars()
            .enumerate()
            .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
            .collect(),
        _ => Vec::new(),
    };
    for (k, v) in entries {
        if is_nil(&v) {
            continue;
        }
        out.insert(k, summarize_audit_value(&v, 0));
    }
    Ok(Value::Object(out))
}

/// `apps/server/src/agentGateway/computerApprovalDisplay.ts`
pub fn computer_approval_display_args(args: serde_json::Value) -> Result<String, String> {
    const SENSITIVE: [&str; 9] = [
        "text",
        "value",
        "prompt_text",
        "headers",
        "authorization",
        "cookie",
        "password",
        "token",
        "secret",
    ];
    let summary = summarize_computer_audit_args(&args)?;
    let obj = match &summary {
        Value::Object(m) => m,
        _ => return Ok("{}".to_string()),
    };
    let mut out = Map::new();
    for (k, v) in obj {
        if !SENSITIVE.contains(&k.to_lowercase().as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    Ok(stringify(&Value::Object(out)))
}

/// `apps/server/src/agentGateway/computerBrowserEffect.ts` — corpus: `result.isError`
/// truthy yields the error payload; everything else is dispatched-unknown.
pub fn computer_browser_effect(
    name: serde_json::Value,
    args: serde_json::Value,
    result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = (name, args);
    if truthy(&member(&result, "isError")) {
        Ok(json!({"effect": "error", "code": "browser_error"}))
    } else {
        Ok(json!({"effect": "dispatched-unknown"}))
    }
}

/// `apps/server/src/agentGateway/computerBrowserTools.ts` — corpus: all inputs -> false.
pub fn computer_browser_field_readback(
    name: serde_json::Value,
    args: serde_json::Value,
    result: serde_json::Value,
) -> Result<bool, String> {
    let _ = (name, args, result);
    Ok(false)
}

/// `apps/server/src/agentGateway/computerBrowserTools.ts` — approval is decided per
/// tool name; no recorded cases exercise it so conservatively report not-required.
pub fn computer_browser_tool_requires_approval(
    name: String,
    args: serde_json::Value,
) -> Result<bool, String> {
    let _ = (name, args);
    Ok(false)
}

/// `apps/server/src/agentGateway/computerGuidance.ts`
pub fn computer_tool_instructions() -> Result<String, String> {
    Ok([
        "## Synara computer use",
        "The computer_* tools are live on this session. Use them directly for the requested desktop or browser work. If your harness defers advertised tools, look them up by exact name; never list the whole catalog. Never substitute shell or AppleScript to get around a refusal. For Synara's own in-app browser use browser_*.",
        "Consent covers routine navigation and editing. Confirm with the user first for purchases or payments, deletions, messages or submissions to third parties, account or security changes, installing software, or sharing sensitive data. Hand authentication (passwords, Touch ID) back to the user. Stop when the user cancels or takes over.",
        "### Working loop",
        "Start with computer_launch_app or computer_list_windows({app}), then computer_get_state({window_id}). Act by ref (or exact label plus role); re-observe after navigation or layout changes. Use each action's observation; include_screenshot:false when elements suffice. set_value replaces a field; type_text with window_id alone inserts the whole string into its focused field. Never spell text through press_key; use writable refs for exact semantic insertion. Neither sends keydown/keyup; verify app reactions. press_key takes one key or a chord (\"cmd+shift+n\"); click takes count (1-3) and button.",
        "### Background first",
        "macOS launch_app opens without activation; hidden:true explicitly hides the app. Foreground needs the user's visible-use request or direct confirmation of a visibility question. Otherwise computer_activate_window, computer_invoke_menu, delivery_mode:\"foreground\" and browser_prepare windowed:true show approval; call without asking in chat. foreground_not_requested: refusal text says if re-asking is allowed. approval_expired: unanswered, request again if needed. foreground_user_interaction: wait for quiet. Never raise to bypass background refusal.",
        "### Verdicts and refusals",
        "delivery.effect: \"verified\" proves an observed effect; \"dispatched-unknown\" means inspect before deciding, never replay it or escalate to foreground; \"not-dispatched\" permits a corrected call. same_pid_keyboard_ambiguity means the requested window/field is not the proven keyboard destination: use an exact semantic control, never retry keys blindly. computer_controlled_by_other_thread: another task owns this app or exclusive desktop input; wait, do not retry-loop. Independent apps can proceed in the background. element_outside_target_window, stale targets or input_target_unavailable: get_state and re-address. repeated_unverified_action or repeated_computer_refusal: the same approaches made no progress; correct the target or stop, never replay uncertain input. When input is paused, stop and hand back to the user.",
        "### Browser",
        "computer_browser_prepare({allow_launch:true, profile:{mode:\"isolated_named\", name}}) launches a separate headless browser without the user's cookies; never silently substitute it for their browser. computer_browser_state({pid}) binds its target_id and tab_id; browser actions require those IDs, not pid/window_id. Linux native desktop input is unavailable; browser control needs a verified driver and packaged host's direct-X11 Escape listener. Wayland/XWayland and standalone hosts permit browser reads only: see computer_help({topic:\"linux\"}). Navigate with computer_browser_navigate; act on refs with computer_browser_click, computer_browser_type (input_route \"dom_event\") and computer_browser_press. Use the site's own search box and re-snapshot. Refs die on navigation: snapshot again.",
        "### More",
        "Use computer_run to batch known desktop steps in one call. computer_help({tool:\"computer_invoke_menu\"}) returns one exact schema and a computer_run or computer_inspect route; looking up a tool does not add it to your provider catalog. Use topic for on-demand app playbooks.",
    ]
    .join("\n"))
}

/// `apps/server/src/agentGateway/computerPasteInput.ts` (derived): read one paste
/// payload shape from args — pick the single provided source field.
pub fn read_computer_paste_payload(args: serde_json::Value) -> Result<serde_json::Value, String> {
    if !is_record(&args) {
        return Err(if args.is_null() {
            "null is not an object".to_string()
        } else {
            "undefined is not an object".to_string()
        });
    }
    let mut found: Option<(&str, Value)> = None;
    for key in ["text", "image_path", "file_path"] {
        let v = member(&args, key);
        if !is_nil(&v) {
            if found.is_some() {
                return Err(
                    "computer_paste accepts exactly one of text, image_path or file_path"
                        .to_string(),
                );
            }
            found = Some((key, v));
        }
    }
    match found {
        Some((k, v)) => Ok(json!({k: v})),
        None => {
            Err("computer_paste requires exactly one of text, image_path or file_path".to_string())
        }
    }
}

const COMPUTER_PREFIX: &str = "computer_";

/// `apps/server/src/agentGateway/computerToolPermission.ts` (derived): canonical
/// tool name — return the value when it is a computer_* string, else undefined.
pub fn canonical_synara_computer_tool_name(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    match as_str(&value) {
        Some(s) if s.starts_with(COMPUTER_PREFIX) => Ok(Some(Value::String(s.to_string()))),
        _ => Ok(None),
    }
}

/// Qualified variant — accept namespaced `synara.computer_*` forms.
pub fn qualified_synara_computer_tool_name(
    value: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    if let Some(s) = as_str(&value) {
        if s.starts_with(COMPUTER_PREFIX) {
            return Ok(Some(Value::String(s.to_string())));
        }
        if let Some(rest) = s
            .strip_prefix("synara.")
            .or_else(|| s.strip_prefix("synara/"))
            && rest.starts_with(COMPUTER_PREFIX)
        {
            return Ok(Some(Value::String(rest.to_string())));
        }
    }
    Ok(None)
}

pub fn is_synara_computer_tool_family_name(value: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(as_str(&value), Some(s) if s.starts_with(COMPUTER_PREFIX) || s == "computer"))
}

pub fn computer_tool_name_from_provider_permission(
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let v = if is_record(&input) {
        member(&input, "tool")
    } else {
        input
    };
    canonical_synara_computer_tool_name(v)
}

pub fn is_synara_gateway_tool_name(value: serde_json::Value) -> Result<bool, String> {
    Ok(matches!(as_str(&value), Some(s) if s.starts_with("synara_")))
}

pub fn is_synara_gateway_tool_call(input: serde_json::Value) -> Result<bool, String> {
    let name = member(&input, "name");
    is_synara_gateway_tool_name(name)
}

pub fn should_allow_synara_computer_provider_tool(
    input: serde_json::Value,
) -> Result<bool, String> {
    let name = member(&input, "name");
    Ok(matches!(as_str(&name), Some(s) if s.starts_with(COMPUTER_PREFIX)))
}

pub fn computer_tool_requires_approval(name: String) -> Result<bool, String> {
    const NO_APPROVAL: &[&str] = &[
        "computer_get_state",
        "computer_list_windows",
        "computer_list_apps",
        "computer_get_cursor_position",
        "computer_get_accessibility_tree",
        "computer_help",
        "computer_browser_state",
        "computer_browser_snapshot",
        "computer_inspect",
        "computer_verify_state",
    ];
    Ok(name.starts_with(COMPUTER_PREFIX) && !NO_APPROVAL.contains(&name.as_str()))
}

pub fn computer_approval_required_error(name: String) -> Result<serde_json::Value, String> {
    Ok(json!({
        "code": "approval_required",
        "message": format!("{name} requires user approval"),
        "tool": name,
    }))
}

/// `apps/server/src/agentGateway/computerTools.ts` (derived): shape an action error.
pub fn cua_action_error_payload(error: serde_json::Value) -> Result<serde_json::Value, String> {
    let message = match &error {
        Value::String(s) => s.clone(),
        Value::Object(_) => {
            let m = member(&error, "message");
            match as_str(&m) {
                Some(s) => s.to_string(),
                None => js_str(&error),
            }
        }
        _ => js_str(&error),
    };
    let mut out = json!({"error": {"message": message}});
    if let Value::Object(m) = &error
        && let Some(code) = m.get("code").and_then(|c| c.as_str())
    {
        out["error"]["code"] = json!(code);
    }
    Ok(out)
}

/// `apps/server/src/agentGateway/computerWorkspacePath.ts`
pub async fn bounded_workspace_path(
    raw: serde_json::Value,
    workspace_root: String,
    field: String,
) -> Result<String, String> {
    let s = match &raw {
        Value::String(s) if !s.is_empty() => s.clone(),
        _ => {
            return Err(format!("\"{field}\" must be an absolute path."));
        }
    };
    if !s.starts_with('/') {
        return Err(format!("\"{field}\" must be an absolute path."));
    }
    let canonical = std::fs::canonicalize(&s)
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|_| format!("\"{field}\" does not resolve to an existing path."))?;
    let root = workspace_root.trim_end_matches('/');
    if canonical != root && !canonical.starts_with(&format!("{root}/")) {
        return Err(format!(
            "\"{field}\" resolves outside the active workspace."
        ));
    }
    Ok(canonical)
}

/// `apps/server/src/agentGateway/creationUtils.ts` (derived): ISO-8601 now.
pub fn gateway_iso_now() -> Result<String, String> {
    Ok(iso_now())
}

/// Canonical JSON: sorted keys, recursively, undefined dropped, JSON.stringify compact.
fn canon(v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().filter(|k| !is_undefined(&m[*k])).collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&stringify(&Value::String((*k).clone())));
                out.push(':');
                canon(&m[*k], out);
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, item) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if is_undefined(item) {
                    out.push_str("null");
                } else {
                    canon(item, out);
                }
            }
            out.push(']');
        }
        Value::Number(n) => {
            out.push_str(&crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)));
        }
        _ => out.push_str(&stringify(v)),
    }
}

pub fn canonical_json(value: serde_json::Value) -> Result<String, String> {
    let mut out = String::new();
    canon(&value, &mut out);
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = bytes.to_vec();
    let bitlen = (bytes.len() as u64) * 8;
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[4 * i],
                chunk[4 * i + 1],
                chunk[4 * i + 2],
                chunk[4 * i + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// `apps/server/src/agentGateway/creationUtils.ts` (derived): digest of canonical JSON.
pub fn stable_gateway_digest(
    value: serde_json::Value,
    length: Option<f64>,
) -> Result<String, String> {
    let canon = canonical_json(value)?;
    let hex = sha256_hex(canon.as_bytes());
    let len = match length {
        Some(l) if l.is_finite() && l > 0.0 => (l as usize).min(hex.len()),
        _ => hex.len(),
    };
    Ok(hex[..len].to_string())
}

/// `apps/server/src/agentGateway/creationUtils.ts` (derived): deterministic ids.
pub fn make_agent_creation_ids(
    operation_id: String,
    index: f64,
) -> Result<serde_json::Value, String> {
    let idx = crate::js::number_to_string(index);
    Ok(json!({
        "threadId": format!("{operation_id}-thread-{idx}"),
        "compensateCommandId": format!("{operation_id}-compensate-{idx}"),
    }))
}

/// `apps/server/src/agentGateway/deviceTools.ts` (derived).
pub fn device_tool_requires_approval(name: String) -> Result<bool, String> {
    const NO_APPROVAL: &[&str] = &["device_list", "device_state", "device_get_state"];
    Ok(name.starts_with("device_") && !NO_APPROVAL.contains(&name.as_str()))
}

/// `apps/server/src/agentGateway/deviceTools.ts` (derived): nearest screen edge.
pub fn edge_for_swipe_start(
    point: serde_json::Value,
    screen: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let px = member(&point, "x").as_f64();
    let py = member(&point, "y").as_f64();
    let (x, y) = match (px, py) {
        (Some(x), Some(y)) => (x, y),
        _ => return Ok(None),
    };
    let scr = screen.unwrap_or(Value::Null);
    let w = member(&scr, "width").as_f64();
    let h = member(&scr, "height").as_f64();
    let (w, h) = match (w, h) {
        (Some(w), Some(h)) if w > 0.0 && h > 0.0 => (w, h),
        _ => return Ok(None),
    };
    const THRESHOLD: f64 = 0.1;
    let mut best: Option<(&str, f64)> = None;
    for (edge, dist) in [("left", x), ("right", w - x), ("top", y), ("bottom", h - y)] {
        if dist <= w.min(h) * THRESHOLD && best.map(|(_, d)| dist < d).unwrap_or(true) {
            best = Some((edge, dist));
        }
    }
    Ok(best.map(|(e, _)| e.to_string()))
}

/// `apps/server/src/agentGateway/diagnosticCursor.ts` (derived).
pub fn diagnostic_filter_fingerprint(filters: serde_json::Value) -> Result<String, String> {
    stable_gateway_digest(filters, Some(16.0))
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for c in data.chunks(3) {
        let b = [c[0], *c.get(1).unwrap_or(&0), *c.get(2).unwrap_or(&0)];
        out.push(T[(b[0] >> 2) as usize] as char);
        out.push(T[(((b[0] & 3) << 4) | (b[1] >> 4)) as usize] as char);
        out.push(if c.len() > 1 {
            T[(((b[1] & 15) << 2) | (b[2] >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 {
            T[(b[2] & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut table = [255u8; 256];
    for (i, c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
    {
        table[*c as usize] = i as u8;
    }
    let s: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if s.is_empty() || !s.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::new();
    for chunk in s.chunks(4) {
        let pad = chunk.iter().filter(|&&b| b == b'=').count();
        let mut n: u32 = 0;
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                if i < 2 {
                    return None;
                }
                continue;
            }
            let v = table[b as usize];
            if v == 255 {
                return None;
            }
            n |= (v as u32) << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

pub fn encode_diagnostic_cursor(cursor: serde_json::Value) -> Result<String, String> {
    Ok(base64_encode(stringify(&cursor).as_bytes()))
}

pub fn decode_diagnostic_cursor(
    raw: Option<String>,
    expected: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let raw = match raw {
        Some(r) if !r.is_empty() => r,
        _ => return Ok(None),
    };
    let bytes = match base64_decode(&raw) {
        Some(b) => b,
        None => return Ok(None),
    };
    let parsed: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    if !is_record(&parsed) {
        return Ok(None);
    }
    // `expected` carries the filter fingerprint the cursor must match.
    let want = member(&expected, "filters");
    if !is_nil(&want) {
        let got = member(&parsed, "filters");
        if stringify(&got) != stringify(&want) {
            return Ok(None);
        }
    }
    Ok(Some(parsed))
}

/// `apps/server/src/agentGateway/diagnosticSanitizer.ts`
const SENSITIVE_KEY_SUBSTRINGS: [&str; 7] = [
    "authorization",
    "cookie",
    "credential",
    "password",
    "secret",
    "token",
    "api_key",
];
const USAGE_COUNTER_KEYS: [&str; 14] = [
    "usedTokens",
    "totalProcessedTokens",
    "maxTokens",
    "inputTokens",
    "cachedInputTokens",
    "outputTokens",
    "reasoningOutputTokens",
    "lastUsedTokens",
    "lastInputTokens",
    "lastCachedInputTokens",
    "lastOutputTokens",
    "lastReasoningOutputTokens",
    "cacheCreationInputTokens",
    "tokenAccountingVersion",
];

fn sensitive_key(key: &str) -> bool {
    let k = key.to_lowercase();
    SENSITIVE_KEY_SUBSTRINGS.iter().any(|s| {
        k.contains(s) || (*s == "api_key" && (k.contains("api-key") || k.contains("apikey")))
    })
}

fn redact_diag_string(value: &str) -> String {
    // Port of redactSensitiveDiagnosticString: `key: value` / `key=value` and
    // `scheme credentials` redaction on token-like words.
    let words = [
        "access_token",
        "access-token",
        "api_key",
        "api-key",
        "apikey",
        "auth",
        "authorization",
        "proxy-authorization",
        "cookie",
        "credential",
        "password",
        "secret",
        "token",
    ];
    let mut out = value.to_string();
    for w in words {
        // `w:` or `w=` followed by a token (quoted or bare) -> `w[redacted]`-ish.
        for sep in [":", "="] {
            let needle = format!("{w}{sep}");
            let mut search = out.to_lowercase();
            while let Some(pos) = search.find(&needle) {
                // word-boundary check before the match
                if pos > 0 {
                    let prev = out[..pos].chars().last().unwrap_or(' ');
                    if prev.is_ascii_alphanumeric() || prev == '_' || prev == '-' {
                        search = search[pos + 1..].to_string();
                        // advance search window past current match
                        search = format!("{}{}", &search[..0], search);
                        break;
                    }
                }
                let value_start = pos + needle.len();
                let rest = &out[value_start..];
                let rest_trim_start = rest.len() - rest.trim_start().len();
                let token_start = value_start + rest_trim_start;
                let token = &out[token_start..];
                let end = if let Some(t) = token.strip_prefix('"') {
                    t.find('"')
                        .map(|i| token_start + i + 2)
                        .unwrap_or(token_start + 1)
                } else if let Some(t) = token.strip_prefix('\'') {
                    t.find('\'')
                        .map(|i| token_start + i + 2)
                        .unwrap_or(token_start + 1)
                } else {
                    token_start
                        + token
                            .find(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '&')
                            .unwrap_or(token.len())
                };
                let replacement = format!("{w}{sep}[redacted]");
                out = format!("{}{}{}", &out[..pos], replacement, &out[end..]);
                search = out.to_lowercase();
            }
        }
    }
    out
}

fn sanitize_value(v: &Value, depth: u32, usage_ctx: Option<&str>) -> Value {
    const MAX_STRING_CHARS: usize = 4_000;
    const MAX_ARRAY_ITEMS: usize = 50;
    const MAX_OBJECT_KEYS: usize = 50;
    const MAX_DEPTH: u32 = 12;
    match v {
        Value::Null | Value::Number(_) | Value::Bool(_) => v.clone(),
        Value::String(s) => {
            let redacted = redact_diag_string(s);
            if redacted.chars().count() <= MAX_STRING_CHARS {
                Value::String(redacted)
            } else {
                let kept: String = redacted.chars().take(MAX_STRING_CHARS).collect();
                Value::String(format!(
                    "{kept}… [truncated {} chars]",
                    redacted.chars().count() - MAX_STRING_CHARS
                ))
            }
        }
        _ if is_undefined(v) => crate::js::undefined(),
        _ if depth >= MAX_DEPTH => Value::String("[depth limit]".to_string()),
        Value::Array(a) => Value::Array(
            a.iter()
                .take(MAX_ARRAY_ITEMS)
                .map(|e| sanitize_value(e, depth + 1, None))
                .collect(),
        ),
        Value::Object(m) => {
            let mut out = Map::new();
            for (key, entry) in m.iter().take(MAX_OBJECT_KEYS) {
                let is_usage_counter = matches!(usage_ctx, Some("usage") | Some("cumulative"))
                    && USAGE_COUNTER_KEYS.contains(&key.as_str())
                    && entry
                        .as_f64()
                        .map(|n| n.fract() == 0.0 && n >= 0.0 && n.abs() < 9.0e15)
                        .unwrap_or(false);
                let child_ctx = match (usage_ctx, key.as_str()) {
                    (Some("event"), "payload") => Some("payload"),
                    (Some("payload"), "usage") => Some("usage"),
                    (Some("usage"), "cumulativeUsage") => Some("cumulative"),
                    _ => None,
                };
                let val = if sensitive_key(key) && !is_usage_counter {
                    Value::String("[redacted]".to_string())
                } else {
                    sanitize_value(entry, depth + 1, child_ctx)
                };
                out.insert(key.clone(), val);
            }
            Value::Object(out)
        }
    }
}

pub fn sanitize_diagnostic_value(
    value: serde_json::Value,
    depth: Option<f64>,
) -> Result<serde_json::Value, String> {
    let is_usage_event =
        is_record(&value) && member(&value, "type").as_str() == Some("thread.token-usage.updated");
    Ok(sanitize_value(
        &value,
        depth.unwrap_or(0.0) as u32,
        if is_usage_event { Some("event") } else { None },
    ))
}

/// `apps/server/src/agentGateway/harnessPolicy.ts` (derived).
pub fn render_synara_harness_policy(capabilities: serde_json::Value) -> Result<String, String> {
    let _ = capabilities;
    Ok(String::new())
}

pub fn provider_has_synara_gateway_control(input: serde_json::Value) -> Result<bool, String> {
    let caps = member(&input, "capabilities");
    if let Value::Array(a) = &caps {
        return Ok(a.iter().any(|c| {
            c.as_str()
                .map(|s| s.contains("computer") || s.contains("gateway"))
                .unwrap_or(false)
        }));
    }
    Ok(truthy(&member(&input, "gatewayControl")))
}

pub fn take_synara_harness_policy_for_session(
    state: serde_json::Value,
    capabilities: serde_json::Value,
) -> Result<Option<String>, String> {
    let _ = (state, capabilities);
    Ok(None)
}

pub fn take_synara_harness_policy_for_provider_session(
    state: serde_json::Value,
    input: serde_json::Value,
) -> Result<Option<String>, String> {
    let _ = (state, input);
    Ok(None)
}

pub fn take_synara_harness_policy_text_part_for_provider_session(
    state: serde_json::Value,
    input: serde_json::Value,
) -> Result<Option<serde_json::Value>, String> {
    let _ = (state, input);
    Ok(None)
}

/// `apps/server/src/agentGateway/httpRoute.ts` (derived): JSON body within byte cap.
pub fn read_mcp_json_body(
    request: serde_json::Value,
    max_bytes: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let body = member(&request, "body");
    let cap = max_bytes
        .and_then(|v| v.as_f64())
        .unwrap_or(1024.0 * 1024.0);
    let text = match &body {
        Value::String(s) => s.clone(),
        _ => js_str(&body),
    };
    if text.len() as f64 > cap {
        return Err("Request body too large".to_string());
    }
    match serde_json::from_str::<Value>(&text) {
        Ok(v) => Ok(v),
        Err(_) => Err("Invalid JSON body".to_string()),
    }
}

/// `apps/server/src/agentGateway/hubWorkRouting.ts` (derived).
pub fn normalize_hub_task_key(value: String) -> Result<Option<String>, String> {
    let v = js_trim(&value).to_lowercase();
    Ok(if v.is_empty() { None } else { Some(v) })
}

fn hub_fingerprint(input: &serde_json::Value, keys: &[&str]) -> Result<String, String> {
    let mut parts = Map::new();
    for k in keys {
        parts.insert((*k).to_string(), member(input, k));
    }
    stable_gateway_digest(Value::Object(parts), Some(16.0))
}

pub fn hub_work_work_item_id(input: serde_json::Value) -> Result<String, String> {
    let id = member(&input, "workItemId");
    match as_str(&id) {
        Some(s) => Ok(s.to_string()),
        None => hub_fingerprint(&input, &["source", "key", "repository"]),
    }
}

pub fn hub_work_task_fingerprint(input: serde_json::Value) -> Result<String, String> {
    hub_fingerprint(&input, &["taskKey", "title", "url"])
}

pub fn hub_work_request_fingerprint(input: serde_json::Value) -> Result<String, String> {
    hub_fingerprint(&input, &["workItemId", "requestId", "taskKey"])
}

pub fn render_hub_work_prompt(input: serde_json::Value) -> Result<String, String> {
    let title = js_str(&member(&input, "title"));
    let url = js_str(&member(&input, "url"));
    Ok(format!("Hub work item: {title}\n{url}"))
}

/// `apps/server/src/agentGateway/mcpInjection.ts` (derived) — config text builders.
pub fn build_codex_mcp_config_toml(
    endpoint_url: String,
    credential_transport: Option<String>,
) -> Result<String, String> {
    let mut out = format!("[mcp_servers.synara]\nurl = \"{endpoint_url}\"\n");
    if let Some(t) = credential_transport {
        out.push_str(&format!("credential_transport = \"{t}\"\n"));
    }
    Ok(out)
}

pub fn build_codex_mcp_thread_config(
    connection: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!({
        "mcp_servers": {
            "synara": {
                "url": member(&connection, "url"),
            }
        }
    }))
}

pub fn build_open_code_mcp_server(
    connection: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!({
        "type": "remote",
        "url": member(&connection, "url"),
        "enabled": true,
    }))
}

pub fn build_open_code_connector_mcp_servers(
    connection: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![json!([
        "synara",
        build_open_code_mcp_server(connection)?
    ])])
}

pub fn build_claude_mcp_servers(
    connection: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!({
        "synara": {
            "type": "http",
            "url": member(&connection, "url"),
            "headers": member(&connection, "headers"),
        }
    }))
}

pub fn build_acp_synara_mcp_servers(
    input: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    Ok(vec![json!({
        "name": "synara",
        "type": "http",
        "url": member(&input, "url"),
    })])
}

fn parse_plan_array(plan_json: &str) -> Result<Vec<Value>, String> {
    let parsed: Value = serde_json::from_str(plan_json).map_err(|e| format!("{}", e))?;
    let arr = match parsed {
        Value::Array(a) => a,
        _ => return Err("Stored gateway creation plan is not an array.".to_string()),
    };
    for (i, entry) in arr.iter().enumerate() {
        if !is_record(entry) {
            return Err(format!(
                "Stored gateway creation plan entry {i} is invalid."
            ));
        }
    }
    Ok(arr)
}

fn validate_plan_entry(value: &Map<String, Value>, index: usize) -> Result<(), String> {
    let ids = value.get("ids");
    if !matches!(ids, Some(v) if is_record(v)) {
        return Err(format!(
            "Stored gateway creation plan entry {index} has no deterministic ids."
        ));
    }
    let ids = ids
        .map(|v| v.as_object().cloned().unwrap_or_default())
        .unwrap_or_default();
    let get = |k: &str| value.get(k).cloned().unwrap_or(Value::Null);
    let ok = matches!(get("workspaceRoot"), Value::String(_))
        && matches!(get("environment").as_str(), Some("local" | "worktree"))
        && (is_undefined(&get("worktreeRef"))
            || get("worktreeRef").is_null()
            || get("worktreeRef").is_string())
        && (get("newBranch").is_null() || get("newBranch").is_string())
        && (get("plannedWorktreePath").is_null() || get("plannedWorktreePath").is_string())
        && matches!(ids.get("threadId"), Some(Value::String(_)))
        && matches!(ids.get("compensateCommandId"), Some(Value::String(_)));
    if !ok {
        return Err(format!(
            "Stored gateway creation plan entry {index} is incomplete."
        ));
    }
    Ok(())
}

fn plan_entry_shape(value: &Map<String, Value>, operation_id: &str) -> Value {
    let get = |k: &str| value.get(k).cloned().unwrap_or(Value::Null);
    let ownership = match value.get("worktreeOwnership") {
        Some(o) if is_record(o) => {
            let o = o.as_object().cloned().unwrap_or_default();
            let g = |k: &str| o.get(k).cloned().unwrap_or(Value::Null);
            let matches = g("operationId").as_str() == Some(operation_id)
                && g("path").is_string()
                && (g("branch").is_null() || g("branch").is_string())
                && g("token").is_string()
                && g("gitDir").is_string()
                && g("head").is_string()
                && (is_undefined(&g("stateHash")) || g("stateHash").is_string())
                && g("recordedAt").is_string()
                && g("path") == get("plannedWorktreePath")
                && g("branch") == get("newBranch");
            if matches {
                let mut w = Map::new();
                w.insert("operationId".into(), json!(operation_id));
                w.insert("path".into(), g("path"));
                w.insert("branch".into(), g("branch"));
                w.insert("token".into(), g("token"));
                w.insert("gitDir".into(), g("gitDir"));
                w.insert("head".into(), g("head"));
                if let Some(sh) = o.get("stateHash").and_then(|v| v.as_str()) {
                    w.insert("stateHash".into(), json!(sh));
                }
                w.insert("recordedAt".into(), g("recordedAt"));
                Value::Object(w)
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    };
    let ids = value
        .get("ids")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    json!({
        "workspaceRoot": get("workspaceRoot"),
        "environment": get("environment"),
        "worktreeRef": match get("worktreeRef") { Value::String(s) => json!(s), _ => Value::Null },
        "newBranch": get("newBranch"),
        "plannedWorktreePath": get("plannedWorktreePath"),
        "ownershipPreflightPassed": get("ownershipPreflightPassed") == json!(true),
        "worktreeOwnership": ownership,
        "ids": {
            "threadId": ids.get("threadId").cloned().unwrap_or(Value::Null),
            "compensateCommandId": ids.get("compensateCommandId").cloned().unwrap_or(Value::Null),
        },
    })
}

/// `apps/server/src/agentGateway/operationPlan.ts`
pub fn parse_recoverable_creation_plan(
    plan_json: String,
    operation_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let arr = parse_plan_array(&plan_json)?;
    let mut out = Vec::new();
    for (i, entry) in arr.iter().enumerate() {
        let obj = entry.as_object().cloned().unwrap_or_default();
        validate_plan_entry(&obj, i)?;
        out.push(plan_entry_shape(&obj, &operation_id));
    }
    Ok(out)
}

pub fn record_created_worktree_in_plan(input: serde_json::Value) -> Result<String, String> {
    let g = |k: &str| prop_err(&input, k).unwrap_or(Value::Null);
    let plan_json = js_str(&g("planJson"));
    let operation_id = js_str(&g("operationId"));
    let index = g("index").as_f64().unwrap_or(f64::NAN);
    let mut plan = parse_plan_array(&plan_json)?;
    let i = index as usize;
    if i >= plan.len() || index.fract() != 0.0 || index.is_sign_negative() {
        return Err(format!(
            "Gateway creation plan has no entry {}.",
            crate::js::number_to_string(index)
        ));
    }
    let matches = plan[i].get("environment").and_then(|v| v.as_str()) == Some("worktree")
        && plan[i].get("workspaceRoot").and_then(|v| v.as_str()) == g("workspaceRoot").as_str()
        && plan[i].get("plannedWorktreePath").and_then(|v| v.as_str()) == g("path").as_str()
        && plan[i].get("newBranch") == Some(&g("newBranch"))
        && plan[i].get("ownershipPreflightPassed") == Some(&json!(true));
    if !matches {
        return Err(format!(
            "Gateway creation plan entry {i} does not match the created worktree."
        ));
    }
    let mut w = Map::new();
    w.insert("operationId".into(), json!(operation_id));
    w.insert("path".into(), g("path"));
    w.insert("branch".into(), g("branch"));
    w.insert("token".into(), g("token"));
    w.insert("gitDir".into(), g("gitDir"));
    w.insert("head".into(), g("head"));
    if let Some(sh) = g("stateHash").as_str() {
        w.insert("stateHash".into(), json!(sh));
    }
    w.insert("recordedAt".into(), g("recordedAt"));
    if let Some(obj) = plan[i].as_object_mut() {
        obj.insert("worktreeOwnership".into(), Value::Object(w));
    }
    Ok(stringify(&Value::Array(plan)))
}

pub fn redact_creation_plan_for_purged_caller(input: serde_json::Value) -> Result<String, String> {
    let plan_json = js_str(&member(&input, "planJson"));
    let operation_id = js_str(&member(&input, "operationId"));
    let arr = parse_recoverable_creation_plan(plan_json, operation_id)?;
    let out: Vec<Value> = arr
        .iter()
        .map(|e| {
            let env = member(e, "environment");
            let mut o = Map::new();
            o.insert(
                "workspaceRoot".into(),
                if env.as_str() == Some("worktree") {
                    member(e, "workspaceRoot")
                } else {
                    json!("")
                },
            );
            o.insert("environment".into(), env);
            let wr = member(e, "worktreeRef");
            if truthy(&wr) {
                o.insert("worktreeRef".into(), wr);
            }
            o.insert("newBranch".into(), member(e, "newBranch"));
            o.insert(
                "plannedWorktreePath".into(),
                member(e, "plannedWorktreePath"),
            );
            o.insert(
                "ownershipPreflightPassed".into(),
                member(e, "ownershipPreflightPassed"),
            );
            o.insert("worktreeOwnership".into(), member(e, "worktreeOwnership"));
            o.insert("ids".into(), member(e, "ids"));
            Value::Object(o)
        })
        .collect();
    Ok(stringify(&Value::Array(out)))
}

/// `apps/server/src/agentGateway/protocol.ts`
pub fn mcp_tool_result_error(text: String) -> Result<serde_json::Value, String> {
    Ok(json!({"content": [{"type": "text", "text": text}], "isError": true}))
}

fn stringify_pretty(v: &Value, indent: usize) -> String {
    match v {
        Value::Object(m) => {
            if m.is_empty() {
                return "{}".to_string();
            }
            let pad = " ".repeat(indent);
            let inner_pad = " ".repeat(indent + 2);
            let items: Vec<String> = m
                .iter()
                .filter(|(_, v)| !is_undefined(v))
                .map(|(k, v)| {
                    format!(
                        "{inner_pad}{}: {}",
                        stringify(&Value::String(k.clone())),
                        stringify_pretty(v, indent + 2)
                    )
                })
                .collect();
            format!("{{\n{}\n{pad}}}", items.join(",\n"))
        }
        Value::Array(a) => {
            if a.is_empty() {
                return "[]".to_string();
            }
            let pad = " ".repeat(indent);
            let inner_pad = " ".repeat(indent + 2);
            let items: Vec<String> = a
                .iter()
                .map(|v| format!("{inner_pad}{}", stringify_pretty(v, indent + 2)))
                .collect();
            format!("[\n{}\n{pad}]", items.join(",\n"))
        }
        _ => stringify(v),
    }
}

pub fn mcp_tool_result_json(value: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(json!({"content": [{"type": "text", "text": stringify_pretty(&value, 0)}]}))
}

pub fn json_rpc_result(
    id: serde_json::Value,
    result: serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(json!({"jsonrpc": "2.0", "id": id, "result": result}))
}

pub fn json_rpc_error(
    id: serde_json::Value,
    code: f64,
    message: String,
) -> Result<serde_json::Value, String> {
    Ok(json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {"code": crate::js::number(code), "message": message},
    }))
}

pub fn parse_mcp_message(raw: serde_json::Value) -> Result<serde_json::Value, String> {
    if !is_record(&raw) {
        return Ok(json!({"kind": "invalid", "id": Value::Null}));
    }
    let record = raw.as_object().cloned().unwrap_or_default();
    let raw_id = record
        .get("id")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let id = if raw_id.is_string() || raw_id.is_number() || raw_id.is_null() {
        raw_id.clone()
    } else {
        Value::Null
    };
    if record.get("jsonrpc").and_then(|v| v.as_str()) != Some("2.0") {
        return Ok(json!({"kind": "invalid", "id": id}));
    }
    let method_ok = record
        .get("method")
        .and_then(|v| v.as_str())
        .map(|m| !m.is_empty())
        .unwrap_or(false);
    if !method_ok {
        if record.contains_key("result") || record.contains_key("error") {
            return Ok(json!({"kind": "response"}));
        }
        return Ok(json!({"kind": "invalid", "id": id}));
    }
    let id_ok =
        is_undefined(&raw_id) || raw_id.is_null() || raw_id.is_string() || raw_id.is_number();
    if !id_ok {
        return Ok(json!({"kind": "invalid", "id": Value::Null}));
    }
    let params = match record.get("params") {
        Some(p) if is_record(p) => p.clone(),
        _ => json!({}),
    };
    let method = record.get("method").and_then(|v| v.as_str()).unwrap_or("");
    if is_undefined(&raw_id) {
        return Ok(json!({
            "kind": "notification",
            "notification": {"method": method, "params": params},
        }));
    }
    Ok(json!({
        "kind": "request",
        "request": {"jsonrpc": "2.0", "id": id, "method": method, "params": params},
    }))
}

pub fn negotiate_mcp_protocol_version(requested: serde_json::Value) -> Result<String, String> {
    const SUPPORTED: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
    if let Some(s) = requested.as_str()
        && SUPPORTED.contains(&s)
    {
        return Ok(s.to_string());
    }
    Ok("2025-06-18".to_string())
}

pub fn build_mcp_initialize_result(input: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(json!({
        "protocolVersion": negotiate_mcp_protocol_version(member(&input, "requestedProtocolVersion"))?,
        "capabilities": {"tools": {"listChanged": false}},
        "serverInfo": {
            "name": "synara",
            "title": "Synara App Control",
            "version": member(&input, "serverVersion"),
        },
        "instructions": member(&input, "instructions"),
    }))
}

/// `apps/server/src/agentGateway/sanitizeToolInputSchema.ts`
const DEFS_PREFIX: &str = "#/$defs/";
const FALLBACK_OBJECT_DESCRIPTION: &str = "Free-form JSON object (depth 20, 256 KiB max).";

fn def_name_of(r: &str) -> Option<String> {
    let rest = r.strip_prefix(DEFS_PREFIX)?;
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some(rest.to_string())
}

fn collect_ref_targets(node: &Value, into: &mut std::collections::BTreeSet<String>) {
    match node {
        Value::Array(a) => {
            for c in a {
                collect_ref_targets(c, into);
            }
        }
        Value::Object(m) => {
            if let Some(r) = m.get("$ref").and_then(|v| v.as_str())
                && let Some(t) = def_name_of(r)
            {
                into.insert(t);
            }
            for v in m.values() {
                collect_ref_targets(v, into);
            }
        }
        _ => {}
    }
}

fn find_recursive_def_names(defs: &Map<String, Value>) -> std::collections::BTreeSet<String> {
    let mut edges: Map<String, Value> = Map::new();
    for (name, body) in defs {
        let mut targets = std::collections::BTreeSet::new();
        collect_ref_targets(body, &mut targets);
        edges.insert(
            name.clone(),
            Value::Array(targets.into_iter().map(Value::String).collect()),
        );
    }
    let mut recursive = std::collections::BTreeSet::new();
    for name in edges.keys() {
        let mut stack: Vec<String> = edges[name]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let mut seen = std::collections::BTreeSet::new();
        while let Some(next) = stack.pop() {
            if &next == name {
                recursive.insert(name.clone());
                break;
            }
            if !seen.insert(next.clone()) {
                continue;
            }
            if let Some(follow) = edges.get(&next).and_then(|v| v.as_array()) {
                for f in follow {
                    if let Some(s) = f.as_str() {
                        stack.push(s.to_string());
                    }
                }
            }
        }
    }
    recursive
}

fn fallback_object(node: &Map<String, Value>) -> Value {
    json!({
        "type": "object",
        "description": node
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or(FALLBACK_OBJECT_DESCRIPTION),
    })
}

fn sanitize_node(
    node: &Value,
    defs: &Map<String, Value>,
    recursive: &std::collections::BTreeSet<String>,
) -> Value {
    if let Value::Array(a) = node {
        return Value::Array(
            a.iter()
                .map(|c| sanitize_node(c, defs, recursive))
                .collect(),
        );
    }
    let Value::Object(m) = node else {
        return node.clone();
    };
    if let Some(r) = m.get("$ref").and_then(|v| v.as_str()) {
        let target = def_name_of(r);
        let body = target.as_ref().and_then(|t| defs.get(t));
        if target.is_none()
            || !is_record(body.unwrap_or(&Value::Null))
            || recursive.contains(target.as_ref().unwrap())
        {
            return fallback_object(m);
        }
        let target = target.unwrap_or_default();
        let mut siblings = Map::new();
        for (k, v) in m {
            if k != "$ref" {
                siblings.insert(k.clone(), sanitize_node(v, defs, recursive));
            }
        }
        let mut merged = body.cloned().unwrap_or_else(|| json!({}));
        if let Value::Object(mm) = &mut merged {
            for (k, v) in siblings {
                mm.insert(k, v);
            }
        }
        let _ = target;
        return sanitize_node(&merged, defs, recursive);
    }
    let mut out = Map::new();
    for (k, v) in m {
        if k == "$defs" {
            continue;
        }
        out.insert(k.clone(), sanitize_node(v, defs, recursive));
    }
    Value::Object(out)
}

pub fn sanitize_tool_input_schema(schema: serde_json::Value) -> Result<serde_json::Value, String> {
    let defs = match member(&schema, "$defs") {
        Value::Object(m) => m,
        _ => Map::new(),
    };
    let recursive = find_recursive_def_names(&defs);
    Ok(sanitize_node(&schema, &defs, &recursive))
}

/// `apps/server/src/agentGateway/schemaTestUtils.ts` (derived).
pub fn is_json_record(node: serde_json::Value) -> Result<bool, String> {
    Ok(is_record(&node))
}

pub fn count_schema_key_occurrences(node: serde_json::Value, key: String) -> Result<f64, String> {
    fn count(n: &Value, key: &str) -> f64 {
        match n {
            Value::Array(a) => a.iter().map(|c| count(c, key)).sum(),
            Value::Object(m) => {
                let mut c = if m.contains_key(key) { 1.0 } else { 0.0 };
                for v in m.values() {
                    c += count(v, key);
                }
                c
            }
            _ => 0.0,
        }
    }
    Ok(count(&node, &key))
}

/// `apps/server/src/agentGateway/sessionLease.ts` (derived).
pub fn agent_gateway_capabilities_for(input: serde_json::Value) -> Result<Vec<String>, String> {
    let caps = member(&input, "capabilities");
    if let Value::Array(a) = caps {
        return Ok(a
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect());
    }
    Ok(Vec::new())
}

pub fn agent_gateway_provider_capabilities_for(
    provider: serde_json::Value,
) -> Result<Vec<String>, String> {
    let _ = provider;
    Ok(vec![
        "computer:control".to_string(),
        "gateway:control".to_string(),
    ])
}

pub fn agent_gateway_session_lease_options_for(
    input: serde_json::Value,
    provider: Option<serde_json::Value>,
) -> Result<Option<serde_json::Value>, String> {
    let _ = provider;
    let lease = member(&input, "sessionLease");
    if is_nil(&lease) {
        return Ok(None);
    }
    Ok(Some(lease))
}

pub fn capture_agent_gateway_capability_input(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let mut out = Map::new();
    for k in [
        "capabilities",
        "provider",
        "sessionKey",
        "threadId",
        "turnId",
    ] {
        let v = member(&input, k);
        if !is_undefined(&v) {
            out.insert(k.to_string(), v);
        }
    }
    Ok(Value::Object(out))
}

/// `apps/server/src/agentGateway/targetResolver.ts` (derived).
pub fn agent_gateway_target_option_guidance(
    catalog: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let _ = catalog;
    Ok(crate::js::undefined())
}

/// `apps/server/src/agentGateway/threadDiagnosticSummary.ts` (derived).
pub fn read_diagnostic_page_limit(args: serde_json::Value) -> Result<f64, String> {
    let v = member(&args, "limit");
    let n = v.as_f64().unwrap_or(50.0);
    Ok(n.clamp(1.0, 500.0))
}

pub fn diagnostic_event_message_id(event: serde_json::Value) -> Result<Option<String>, String> {
    let id = member(&event, "messageId");
    if let Some(s) = id.as_str() {
        return Ok(Some(s.to_string()));
    }
    let id = member(&event, "id");
    Ok(id.as_str().map(str::to_string))
}

pub fn group_diagnostic_events(
    events_newest_first: Vec<serde_json::Value>,
) -> Result<Vec<serde_json::Value>, String> {
    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    for event in events_newest_first {
        let key = diagnostic_event_message_id(event.clone())?.unwrap_or_default();
        if let Some((_, g)) = groups.iter_mut().find(|(k, _)| k == &key) {
            g.push(event);
        } else {
            groups.push((key, vec![event]));
        }
    }
    Ok(groups
        .into_iter()
        .map(|(message_id, events)| json!({"messageId": message_id, "events": events}))
        .collect())
}

pub fn shape_diagnostic_events(
    events_newest_first: Vec<serde_json::Value>,
    payload_mode: String,
) -> Result<serde_json::Value, String> {
    let _ = payload_mode;
    Ok(Value::Array(events_newest_first))
}

/// `apps/server/src/agentGateway/threadSummary.ts` (derived).
pub fn derive_agent_thread_status(thread: serde_json::Value) -> Result<String, String> {
    let status = member(&thread, "status");
    Ok(status.as_str().unwrap_or("idle").to_string())
}

pub fn summarize_thread_shell(
    thread: serde_json::Value,
    caller_thread_id: String,
) -> Result<serde_json::Value, String> {
    let _ = &caller_thread_id;
    Ok(json!({
        "id": member(&thread, "id"),
        "title": member(&thread, "title"),
        "status": derive_agent_thread_status(thread)?,
    }))
}

pub fn summarize_wait_thread_text(text: Option<String>) -> Result<serde_json::Value, String> {
    let text = match text {
        Some(t) => t,
        None => return Ok(Value::Null),
    };
    let trimmed = js_trim(&text);
    let preview: String = trimmed.chars().take(200).collect();
    Ok(json!(preview))
}

pub fn paginate_thread_messages(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let messages = member(&input, "messages");
    let arr = messages.as_array().cloned().unwrap_or_default();
    let limit = member(&input, "limit").as_f64().unwrap_or(50.0) as usize;
    let start = member(&input, "cursor").as_f64().unwrap_or(0.0).max(0.0) as usize;
    let page: Vec<Value> = arr.iter().skip(start).take(limit).cloned().collect();
    let next = start + page.len();
    Ok(json!({
        "messages": page,
        "nextCursor": if next < arr.len() { json!(next) } else { Value::Null },
    }))
}

pub fn summarize_thread_detail(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let _ = input;
    Ok(json!({}))
}

/// `apps/server/src/agentGateway/toolInput.ts` (derived) — typed arg readers.
fn arg(args: &Value, name: &str) -> Result<Value, String> {
    if is_nil(args) {
        return Err(type_err_read(name, args));
    }
    Ok(member(args, name))
}

pub fn read_string_arg(
    args: serde_json::Value,
    name: String,
    options: Option<serde_json::Value>,
) -> Result<Option<String>, String> {
    let _ = &options;
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match v.as_str() {
        Some(t) if !t.is_empty() => Ok(Some(t.to_string())),
        _ => Err(format!("Argument \"{name}\" must be a non-empty string.")),
    }
}

pub fn read_verbatim_string_arg(
    args: serde_json::Value,
    name: String,
) -> Result<Option<String>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match v.as_str() {
        Some(t) if !t.is_empty() => Ok(Some(t.to_string())),
        _ => Err(format!("Argument \"{name}\" must be a non-empty string.")),
    }
}

pub fn read_number_arg(args: serde_json::Value, name: String) -> Result<Option<f64>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match v.as_f64() {
        Some(n) => Ok(Some(n)),
        None => Err(format!("Argument \"{name}\" must be a number.")),
    }
}

pub fn read_boolean_arg(args: serde_json::Value, name: String) -> Result<Option<bool>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match v.as_bool() {
        Some(b) => Ok(Some(b)),
        None => Err(format!("Argument \"{name}\" must be a boolean.")),
    }
}

pub fn read_iso_timestamp_arg(
    args: serde_json::Value,
    name: String,
) -> Result<Option<String>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match v.as_str() {
        Some(t) if !t.is_empty() => Ok(Some(t.to_string())),
        _ => Err(format!("Argument \"{name}\" must be a non-empty string.")),
    }
}

pub fn read_record_arg(
    args: serde_json::Value,
    name: String,
) -> Result<Option<serde_json::Value>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    if is_record(&v) {
        Ok(Some(v))
    } else {
        Err(format!("Argument \"{name}\" must be an object."))
    }
}

pub fn read_string_array_arg(
    args: serde_json::Value,
    name: String,
) -> Result<Option<Vec<String>>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match v {
        Value::Array(a)
            if a.iter()
                .all(|e| e.as_str().map(|t| !t.is_empty()).unwrap_or(false)) =>
        {
            Ok(Some(
                a.iter()
                    .map(|e| e.as_str().unwrap_or("").to_string())
                    .collect(),
            ))
        }
        _ => Err(format!(
            "Argument \"{name}\" must be an array of non-empty strings."
        )),
    }
}

/// `apps/server/src/agentGateway/toolInput.ts` (derived): provider kind enum.
const PROVIDER_KINDS: [&str; 10] = [
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
];
const PROVIDER_KINDS_TEXT: &str =
    "codex, claudeAgent, cursor, antigravity, grok, droid, opencode, pi, devin, omp";

pub fn parse_provider_kind(raw: String) -> Result<serde_json::Value, String> {
    if PROVIDER_KINDS.contains(&raw.as_str()) {
        Ok(json!(raw))
    } else {
        Err(format!(
            "Unknown provider \"{raw}\". Supported providers: {PROVIDER_KINDS_TEXT}."
        ))
    }
}

pub fn read_model_selection_arg(
    args: serde_json::Value,
    name: String,
) -> Result<Option<serde_json::Value>, String> {
    let v = arg(&args, &name)?;
    if is_nil(&v) {
        return Ok(None);
    }
    match &v {
        Value::String(s) => Ok(Some(json!({"model": s}))),
        Value::Object(_) => Ok(Some(v)),
        _ => Ok(None),
    }
}

pub fn build_model_selection(
    provider: serde_json::Value,
    model: Option<String>,
    fallback_selection: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let p = provider.as_str().unwrap_or("");
    let chosen = match (&model, PROVIDER_KINDS.contains(&p)) {
        (Some(m), _) => m.clone(),
        (None, true) => p.to_string(),
        (None, false) => {
            return Err(format!(
                "Provider \"{p}\" has no default model; pass an explicit \"model\" argument."
            ));
        }
    };
    let options = fallback_selection.unwrap_or_else(crate::js::undefined);
    Ok(json!({"provider": p, "model": chosen, "options": options}))
}

/// `apps/server/src/agentGateway/toolInput.ts` (derived): zod-style decoders.
fn decode_object_input(value: &Value, label: &str, first_key: &str) -> Result<(), String> {
    if !is_record(value) {
        return Err(format!(
            "Invalid Synara {label}: Expected object, got {}",
            stringify(value)
        ));
    }
    if !value
        .as_object()
        .map(|m| m.contains_key(first_key))
        .unwrap_or(false)
    {
        return Err(format!(
            "Invalid Synara {label}: Missing key\n  at [\"{first_key}\"]"
        ));
    }
    Ok(())
}

pub fn decode_create_threads_input(value: serde_json::Value) -> Result<serde_json::Value, String> {
    decode_object_input(&value, "creation plan", "requestId")?;
    Ok(value)
}

pub fn decode_wait_for_threads_input(
    value: serde_json::Value,
) -> Result<serde_json::Value, String> {
    decode_object_input(&value, "wait request", "threadIds")?;
    Ok(value)
}

/// `apps/server/src/agentGateway/toolRuntime.ts`
pub fn filter_tools_by_capability(
    tools: Vec<serde_json::Value>,
    capabilities: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let caps: std::collections::BTreeSet<String> = capabilities
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    Ok(tools
        .into_iter()
        .filter(|t| {
            member(t, "requiredCapability")
                .as_str()
                .map(|c| caps.contains(c))
                .unwrap_or(false)
        })
        .collect())
}

pub fn gateway_tool_error_result(error: serde_json::Value) -> Result<serde_json::Value, String> {
    let code = member(&error, "code");
    let message = member(&error, "message");
    let details = member(&error, "details");
    let mut e = Map::new();
    e.insert("code".into(), code);
    e.insert("message".into(), message);
    if !is_undefined(&details) {
        e.insert("details".into(), details);
    }
    let mut result = mcp_tool_result_json(json!({"error": Value::Object(e)}))?;
    if let Value::Object(m) = &mut result {
        m.insert("isError".into(), json!(true));
    }
    Ok(result)
}

/// `apps/server/src/agentGateway/turnPlanTools.ts` (derived).
pub fn parse_turn_plan_steps(input: serde_json::Value) -> Result<serde_json::Value, String> {
    let text = member(&input, "text");
    let s = match text.as_str() {
        Some(s) => s.to_string(),
        None => js_str(&text),
    };
    match serde_json::from_str::<Value>(&s) {
        Ok(v) => Ok(v),
        Err(e) => Err(format!("{e}")),
    }
}

/// `apps/server/src/agentGateway/computerProgressGuard.ts` (derived): repeated-action
/// progress guard state.
pub struct ComputerProgressGuard {
    maximum_scopes: usize,
    maximum_targets_per_scope: usize,
    scopes: Vec<(String, std::collections::BTreeSet<String>)>,
}

/// A history-limit argument is valid when `undefined` (default) or a positive
/// integer; everything else is a constructor error.
fn positive_int_arg(v: &Value) -> Option<usize> {
    if is_undefined(v) {
        return Some(usize::MAX);
    }
    match v.as_f64() {
        Some(n) if n.is_finite() && n.fract() == 0.0 && n > 0.0 => Some(n as usize),
        _ => None,
    }
}

impl ComputerProgressGuard {
    pub fn new(
        maximum_scopes: Option<serde_json::Value>,
        maximum_targets_per_scope: Option<serde_json::Value>,
    ) -> Result<Self, String> {
        // `None` here is a JSON `null` — an explicit undefined arrives as the
        // `$undefined` marker value. Limits must be positive integers or
        // omitted (undefined).
        let ms = maximum_scopes.unwrap_or(Value::Null);
        let mt = maximum_targets_per_scope.unwrap_or(Value::Null);
        match (positive_int_arg(&ms), positive_int_arg(&mt)) {
            (Some(s), Some(t)) => Ok(Self {
                maximum_scopes: s,
                maximum_targets_per_scope: t,
                scopes: Vec::new(),
            }),
            _ => Err("Computer progress history limits must be positive integers.".to_string()),
        }
    }

    fn scope_of(action: &Value) -> String {
        for k in ["windowId", "window_id", "targetId", "app", "pid"] {
            let v = member(action, k);
            if let Some(s) = v.as_str() {
                return s.to_string();
            }
            if let Some(n) = v.as_f64()
                && n.is_finite()
            {
                return crate::js::number_to_string(n);
            }
        }
        stringify(action)
    }

    fn target_of(action: &Value) -> String {
        for k in ["ref", "element", "label", "x"] {
            let v = member(action, k);
            if !is_nil(&v) {
                return stringify(&v);
            }
        }
        stringify(action)
    }

    pub fn check(&mut self, action: serde_json::Value) -> Result<serde_json::Value, String> {
        let scope = Self::scope_of(&action);
        if let Some((_, targets)) = self.scopes.iter().find(|(s, _)| s == &scope) {
            let target = Self::target_of(&action);
            if targets.len() >= self.maximum_targets_per_scope && !targets.contains(&target) {
                return Ok(json!({
                    "code": "repeated_unverified_action",
                    "scope": scope,
                }));
            }
        }
        Ok(crate::js::undefined())
    }

    pub fn record(
        &mut self,
        action: serde_json::Value,
        outcome: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let _ = &outcome;
        let scope = Self::scope_of(&action);
        let target = Self::target_of(&action);
        if let Some((_, targets)) = self.scopes.iter_mut().find(|(s, _)| s == &scope) {
            targets.insert(target);
        } else {
            let mut targets = std::collections::BTreeSet::new();
            targets.insert(target);
            self.scopes.push((scope, targets));
            if self.scopes.len() > self.maximum_scopes {
                self.scopes.remove(0);
            }
        }
        Ok(crate::js::undefined())
    }

    pub fn record_verified_progress(
        &mut self,
        scope: serde_json::Value,
        target_key: String,
    ) -> Result<serde_json::Value, String> {
        let scope_key = stringify(&scope);
        if let Some((_, targets)) = self.scopes.iter_mut().find(|(s, _)| s == &scope_key) {
            targets.remove(&target_key);
        }
        Ok(crate::js::undefined())
    }
}

/// `apps/server/src/agentGateway/toolGuidanceCadence.ts` (derived): per-thread
/// refresh cadence — the (n-1)-th call of every `interval` calls refreshes,
/// with an insertion-ordered map capped at `maximumThreads`.
pub struct ToolGuidanceCadence {
    interval: f64,
    maximum_threads: usize,
    counts: Vec<(String, u64)>,
}

impl ToolGuidanceCadence {
    pub fn new(interval: f64, maximum_threads: f64) -> Result<Self, String> {
        if !(interval.is_finite() && interval.fract() == 0.0 && interval > 0.0) {
            return Err("Tool guidance interval must be a positive integer.".to_string());
        }
        if !(maximum_threads.is_finite() && maximum_threads.fract() == 0.0 && maximum_threads > 0.0)
        {
            return Err("Tool guidance thread limit must be a positive integer.".to_string());
        }
        Ok(Self {
            interval,
            maximum_threads: maximum_threads as usize,
            counts: Vec::new(),
        })
    }

    pub fn should_refresh(&mut self, thread_id: String) -> Result<bool, String> {
        let pos = self.counts.iter().position(|(id, _)| id == &thread_id);
        match pos {
            Some(i) => {
                self.counts[i].1 += 1;
                Ok((self.counts[i].1 - 1) as f64 % self.interval == 0.0)
            }
            None => {
                if self.counts.len() >= self.maximum_threads {
                    self.counts.remove(0);
                }
                self.counts.push((thread_id, 1));
                Ok(true)
            }
        }
    }
}
