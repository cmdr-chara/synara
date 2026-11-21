//! The Rust port of `apps/web/src/composerDraftPersistence.ts`.
//!
//! Schema-v6 composer-draft persistence: migrations, merge normalization, and
//! hydration. Model-selection helpers are vendored from the sibling
//! `composerdraftmodels-rs` port (`mod models`), domain helpers from
//! `composerdraftdomain-rs` (`mod domain`), attachment helpers from
//! `composerdraftattachments-rs` (`mod attachments`), pending-user-input
//! normalization from `pendinguserinputrecovery-rs` (`mod pui`), and computer
//! control resolution from `computercontrolmode-rs` (`mod ccm`).

#![forbid(unsafe_code)]

use crate::js;
use serde_json::{Map, Value, json};

// ── JS helpers ──────────────────────────────────────────────────────────────

fn is_undef(v: &Value) -> bool {
    js::is_undefined(v)
}

fn nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn truthy(v: &Value) -> bool {
    if let Some(n) = js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    if js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        _ => true,
    }
}

/// `typeof v === "object"` in JS (null is caught separately by callers).
fn is_object_type(v: &Value) -> bool {
    matches!(v, Value::Object(_) | Value::Array(_)) && !js::is_undefined(v)
}

fn prop(v: &Value, key: &str) -> Value {
    v.get(key).cloned().unwrap_or_else(js::undefined)
}

/// Member access `receiver.name` throwing the JSC error for nullish receivers.
fn prop_eval(v: &Value, key: &str, expr: &str) -> Result<Value, String> {
    if v.is_null() {
        return Err(format!("null is not an object (evaluating '{expr}')"));
    }
    if is_undef(v) {
        return Err(format!("undefined is not an object (evaluating '{expr}')"));
    }
    Ok(prop(v, key))
}

fn utf16_len(s: &str) -> usize {
    s.chars().map(|c| c.len_utf16()).sum()
}

fn is_js_space(c: char) -> bool {
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
    s.trim_matches(is_js_space)
}

/// `Schema.is(TrimmedNonEmptyString)`: already-trimmed non-empty string.
fn is_tne(v: &Value) -> bool {
    matches!(v, Value::String(s) if !s.is_empty() && js_trim(s) == s)
}

fn opt_str(v: &Value, key: &str) -> String {
    match prop(v, key) {
        Value::String(s) => s,
        _ => String::new(),
    }
}

fn opt_num(v: &Value, key: &str) -> Option<f64> {
    match prop(v, key) {
        Value::Number(n) => n.as_f64(),
        _ => None,
    }
}

fn num_is_integer(n: f64) -> bool {
    n.is_finite() && n.fract() == 0.0
}

fn canonical_index(k: &str) -> Option<u64> {
    if k.is_empty() || k.len() > 10 || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u64 = k.parse().ok()?;
    if n >= 4_294_967_295 || n.to_string() != k {
        return None;
    }
    Some(n)
}

/// `Object.entries(v)` iteration order: canonical integer keys ascending,
/// then remaining string keys in insertion order. Arrays enumerate indices.
fn entries_js_order(v: &Value) -> Vec<(String, Value)> {
    match v {
        Value::Object(m) => {
            let mut ints: Vec<u64> = m.keys().filter_map(|k| canonical_index(k)).collect();
            ints.sort_unstable();
            let mut out: Vec<(String, Value)> = Vec::new();
            for n in ints {
                let k = n.to_string();
                if let Some(val) = m.get(&k) {
                    out.push((k, val.clone()));
                }
            }
            for (k, val) in m {
                if canonical_index(k).is_none() {
                    out.push((k.clone(), val.clone()));
                }
            }
            out
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(i, val)| (i.to_string(), val.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn arr_field<'a>(v: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    match v.get(key) {
        Some(Value::Array(items)) => Some(items),
        _ => None,
    }
}

/// `{...v}` spread into a map: objects copy own entries; arrays index entries.
fn spread_to_map(v: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    match v {
        Value::Object(o) => {
            for (k, val) in o {
                out.insert(k.clone(), val.clone());
            }
        }
        Value::Array(items) => {
            for (i, val) in items.iter().enumerate() {
                out.insert(i.to_string(), val.clone());
            }
        }
        _ => {}
    }
    out
}

fn js_num(n: f64) -> Value {
    js::number(n)
}

// ── Schema.is guards ─────────────────────────────────────────────────────────

fn str_is(v: &Value, lit: &str) -> bool {
    matches!(v, Value::String(s) if s == lit)
}

fn is_runtime_mode(v: &Value) -> bool {
    str_is(v, "approval-required") || str_is(v, "auto") || str_is(v, "full-access")
}

fn is_interaction_mode(v: &Value) -> bool {
    str_is(v, "default") || str_is(v, "plan") || str_is(v, "debug")
}

fn is_computer_control_mode(v: &Value) -> bool {
    str_is(v, "off") || str_is(v, "request") || str_is(v, "chat")
}

fn is_env_mode(v: &Value) -> bool {
    str_is(v, "local") || str_is(v, "worktree")
}

fn is_pr_scope(v: &Value) -> bool {
    const SCOPES: [&str; 5] = ["reference", "comments", "checks", "conflicts", "everything"];
    matches!(v, Value::String(s) if SCOPES.contains(&s.as_str()))
}

/// `Schema.is(ProviderSkillReference)` / `ProviderMentionReference`:
/// `{name: TrimmedNonEmptyString, path: TrimmedNonEmptyString}`.
fn is_name_path_ref(v: &Value) -> bool {
    is_object_type(v) && !v.is_null() && is_tne(&prop(v, "name")) && is_tne(&prop(v, "path"))
}

/// `Schema.is(PersistedSourceProposedPlanReference)`:
/// `{threadId: ThreadId, planId: OrchestrationProposedPlanId}` (both trimmed
/// non-empty strings).
fn is_source_plan_ref(v: &Value) -> bool {
    is_object_type(v) && !v.is_null() && is_tne(&prop(v, "threadId")) && is_tne(&prop(v, "planId"))
}

/// `Schema.is(PersistedRestoredSourceProposedPlan)`.
fn is_restored_plan(v: &Value) -> bool {
    is_object_type(v)
        && !v.is_null()
        && is_tne(&prop(v, "threadId"))
        && matches!(prop(v, "restoredPrompt"), Value::String(_))
        && is_source_plan_ref(&prop(v, "sourceProposedPlan"))
}

fn is_non_neg_int(v: &Value) -> bool {
    matches!(js::as_number(v), Some(n) if num_is_integer(n) && n >= 0.0)
}

fn is_pos_int(v: &Value) -> bool {
    matches!(js::as_number(v), Some(n) if num_is_integer(n) && n >= 1.0)
}

/// `Schema.is(ProcessEnvRecord)`: env-name keys (≤128 chars), string values
/// (≤8192 chars), ≤128 properties.
fn is_env_record(v: &Value) -> bool {
    let ok_name = |k: &str| -> bool {
        utf16_len(k) <= 128
            && k.chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    };
    match v {
        Value::Object(m) => {
            m.len() <= 128
                && m.iter().all(|(k, val)| {
                    ok_name(k) && matches!(val, Value::String(s) if utf16_len(s) <= 8192)
                })
        }
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// `Schema.is(ProviderStartOptions)`: object whose per-provider keys (when
/// present) satisfy each provider's optional-field struct.
fn is_provider_start_options(v: &Value) -> bool {
    if !is_object_type(v) || v.is_null() {
        return false;
    }
    const T: u8 = 0; // TrimmedNonEmptyString
    const E: u8 = 1; // ProcessEnvRecord
    const N: u8 = 2; // NonNegativeInt
    const B: u8 = 3; // Boolean
    let spec: [(&str, &[(&str, u8)]); 10] = [
        (
            "codex",
            &[
                ("binaryPath", T),
                ("homePath", T),
                ("shadowHomePath", T),
                ("accountId", T),
                ("environment", E),
            ],
        ),
        (
            "claudeAgent",
            &[
                ("binaryPath", T),
                ("homePath", T),
                ("permissionMode", T),
                ("maxThinkingTokens", N),
                ("enableArtifacts", B),
                ("environment", E),
            ],
        ),
        (
            "cursor",
            &[("binaryPath", T), ("apiEndpoint", T), ("environment", E)],
        ),
        ("devin", &[("binaryPath", T), ("environment", E)]),
        ("antigravity", &[("binaryPath", T), ("environment", E)]),
        ("grok", &[("binaryPath", T), ("environment", E)]),
        ("droid", &[("binaryPath", T), ("environment", E)]),
        (
            "opencode",
            &[
                ("binaryPath", T),
                ("serverUrl", T),
                ("serverPassword", T),
                ("experimentalWebSockets", B),
                ("environment", E),
            ],
        ),
        (
            "pi",
            &[("binaryPath", T), ("agentDir", T), ("environment", E)],
        ),
        (
            "omp",
            &[("binaryPath", T), ("agentDir", T), ("environment", E)],
        ),
    ];
    for (provider, fields) in spec {
        let block = prop(v, provider);
        if is_undef(&block) {
            continue;
        }
        if !is_object_type(&block) || block.is_null() {
            return false;
        }
        for (field, kind) in fields {
            let val = prop(&block, field);
            if is_undef(&val) {
                continue;
            }
            let ok = match *kind {
                T => is_tne(&val),
                E => is_env_record(&val),
                N => is_non_neg_int(&val),
                _ => matches!(val, Value::Bool(_)),
            };
            if !ok {
                return false;
            }
        }
    }
    true
}

/// `Schema.decodeUnknownSync(OrchestrationThreadPullRequest)`: returns the
/// decoded (trimmed) object on success.
fn decode_thread_pull_request(v: &Value) -> Option<Value> {
    if !is_object_type(v) || v.is_null() {
        return None;
    }
    if !is_pos_int(&prop(v, "number")) {
        return None;
    }
    let tne_field = |key: &str| -> Option<String> {
        let val = prop(v, key);
        match val {
            Value::String(s) if is_tne(&Value::String(s.clone())) => Some(js_trim(&s).to_string()),
            _ => None,
        }
    };
    let title = tne_field("title")?;
    let url = prop(v, "url");
    let Value::String(url) = url else {
        return None;
    };
    let base_branch = tne_field("baseBranch")?;
    let head_branch = tne_field("headBranch")?;
    let state = prop(v, "state");
    if !(str_is(&state, "open") || str_is(&state, "closed") || str_is(&state, "merged")) {
        return None;
    }
    let mut out = Map::new();
    out.insert("number".to_string(), prop(v, "number"));
    out.insert("title".to_string(), Value::String(title));
    out.insert("url".to_string(), Value::String(url));
    out.insert("baseBranch".to_string(), Value::String(base_branch));
    out.insert("headBranch".to_string(), Value::String(head_branch));
    out.insert("state".to_string(), state);
    let is_draft = prop(v, "isDraft");
    if !is_undef(&is_draft) {
        if !matches!(is_draft, Value::Bool(_)) {
            return None;
        }
        out.insert("isDraft".to_string(), is_draft);
    }
    let mergeability = prop(v, "mergeability");
    if !is_undef(&mergeability) {
        if !(str_is(&mergeability, "mergeable")
            || str_is(&mergeability, "conflicting")
            || str_is(&mergeability, "unknown"))
        {
            return None;
        }
        out.insert("mergeability".to_string(), mergeability);
    }
    for key in ["additions", "deletions", "changedFiles"] {
        let val = prop(v, key);
        if is_undef(&val) {
            continue;
        }
        if !(val.is_null() || is_non_neg_int(&val)) {
            return None;
        }
        out.insert(key.to_string(), val);
    }
    Some(Value::Object(out))
}

// ── Text normalization ──────────────────────────────────────────────────────

fn normalize_terminal_context_text(text: &str) -> String {
    text.replace("\r\n", "\n").trim_matches('\n').to_string()
}

fn normalize_pasted_text_content(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// `normalizeAssistantSelectionText` / `normalizeFileCommentText`.
fn normalize_selection_text(text: &str) -> String {
    js_trim(text.replace("\r\n", "\n").trim_matches('\n')).to_string()
}

/// `ensureInlineTerminalContextPlaceholders`.
fn ensure_inline_terminal_context_placeholders(prompt: &str, context_count: usize) -> String {
    let have = prompt.chars().filter(|c| *c == '\u{fffc}').count();
    if context_count <= have {
        return prompt.to_string();
    }
    format!("{}{}", "\u{fffc}".repeat(context_count - have), prompt)
}

/// `new Date().toISOString()`: UTC ISO-8601 with millisecond precision.
fn now_iso() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    let secs = ms.div_euclid(1000);
    let millis = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let d = doy - (153 * mp + 2).div_euclid(5) + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{year:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{millis:03}Z",
        tod / 3600,
        tod % 3600 / 60,
        tod % 60,
    )
}

// ── Persisted element normalizers ───────────────────────────────────────────

/// `normalizePersistedTerminalContextDraft`.
fn normalize_persisted_terminal_context_draft(v: &Value) -> Option<Value> {
    if !truthy(v) || !is_object_type(v) {
        return None;
    }
    let id = opt_str(v, "id");
    let thread_id = opt_str(v, "threadId");
    let created_at = opt_str(v, "createdAt");
    let line_start = opt_num(v, "lineStart");
    let line_end = opt_num(v, "lineEnd");
    let (Some(ls), Some(le)) = (line_start, line_end) else {
        return None;
    };
    if id.is_empty()
        || thread_id.is_empty()
        || created_at.is_empty()
        || !ls.is_finite()
        || !le.is_finite()
    {
        return None;
    }
    let terminal_id = js_trim(&opt_str(v, "terminalId")).to_string();
    let terminal_label = js_trim(&opt_str(v, "terminalLabel")).to_string();
    if terminal_id.is_empty() || terminal_label.is_empty() {
        return None;
    }
    let start = 1.0f64.max(ls.floor());
    let end = start.max(le.floor());
    Some(json!({
        "id": id,
        "threadId": thread_id,
        "createdAt": created_at,
        "terminalId": terminal_id,
        "terminalLabel": terminal_label,
        "lineStart": js_num(start),
        "lineEnd": js_num(end),
    }))
}

/// `normalizePersistedQueuedTerminalContextDraft`.
fn normalize_persisted_queued_terminal_context_draft(v: &Value) -> Option<Value> {
    if !truthy(v) || !is_object_type(v) {
        return None;
    }
    let base = normalize_persisted_terminal_context_draft(v)?;
    let text = match prop(v, "text") {
        Value::String(s) => normalize_terminal_context_text(&s),
        _ => String::new(),
    };
    let mut map = spread_to_map(&base);
    map.insert("text".to_string(), Value::String(text));
    Some(Value::Object(map))
}

/// `normalizePersistedAssistantSelection`.
fn normalize_persisted_assistant_selection(v: &Value) -> Option<Value> {
    if !truthy(v) || !is_object_type(v) {
        return None;
    }
    let id = opt_str(v, "id");
    if id.is_empty() {
        return None;
    }
    let assistant_message_id = js_trim(&opt_str(v, "assistantMessageId")).to_string();
    let text = normalize_selection_text(&opt_str(v, "text"));
    if assistant_message_id.is_empty() || text.is_empty() || utf16_len(&text) > 4_000 {
        return None;
    }
    Some(json!({
        "id": id,
        "assistantMessageId": assistant_message_id,
        "text": text,
    }))
}

/// `normalizePersistedFileCommentDraft` (via `normalizeFileCommentSelection`).
fn normalize_persisted_file_comment_draft(v: &Value) -> Option<Value> {
    if !truthy(v) || !is_object_type(v) {
        return None;
    }
    let id = opt_str(v, "id");
    if id.is_empty() {
        return None;
    }
    let path = opt_str(v, "path");
    let text = opt_str(v, "text");
    let start = match prop(v, "startLine") {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    let end = match prop(v, "endLine") {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if !(start.is_finite() && end.is_finite()) {
        return None;
    }
    // normalizeFileCommentSelection
    let path = js_trim(&path).to_string();
    let text = normalize_selection_text(&text);
    if path.is_empty() || text.is_empty() || utf16_len(&text) > 4_000 {
        return None;
    }
    let start = 1.0f64.max(start.floor());
    let end = start.max(end.floor());
    Some(json!({
        "id": id,
        "path": path,
        "startLine": js_num(start),
        "endLine": js_num(end),
        "text": text,
    }))
}

/// `normalizePersistedPastedTextDraft`.
fn normalize_persisted_pasted_text_draft(v: &Value) -> Option<Value> {
    if !truthy(v) || !is_object_type(v) {
        return None;
    }
    let id = opt_str(v, "id");
    let created_at = opt_str(v, "createdAt");
    let text = match prop(v, "text") {
        Value::String(s) => normalize_pasted_text_content(&s),
        _ => String::new(),
    };
    if id.is_empty() || text.is_empty() {
        return None;
    }
    Some(json!({
        "id": id,
        "createdAt": created_at,
        "text": text,
    }))
}

/// `normalizePersistedPullRequestContextDraft`.
fn normalize_persisted_pull_request_context_draft(v: &Value) -> Option<Value> {
    if !truthy(v) || !is_object_type(v) {
        return None;
    }
    let id = opt_str(v, "id");
    let text = js_trim(&opt_str(v, "text")).to_string();
    let title = js_trim(&opt_str(v, "title")).to_string();
    let pr_number = match prop(v, "prNumber") {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    };
    let scope = prop(v, "scope");
    if id.is_empty()
        || text.is_empty()
        || title.is_empty()
        || !is_pr_scope(&scope)
        || !num_is_integer(pr_number)
        || pr_number <= 0.0
    {
        return None;
    }
    let mut out = Map::new();
    out.insert("id".to_string(), Value::String(id));
    out.insert(
        "createdAt".to_string(),
        Value::String(opt_str(v, "createdAt")),
    );
    out.insert("scope".to_string(), scope);
    if str_is(&prop(v, "itemKind"), "issue") {
        out.insert("itemKind".to_string(), json!("issue"));
    }
    out.insert("prNumber".to_string(), js_num(pr_number));
    out.insert("prUrl".to_string(), Value::String(opt_str(v, "prUrl")));
    out.insert("title".to_string(), Value::String(title));
    out.insert(
        "subtitle".to_string(),
        Value::String(opt_str(v, "subtitle")),
    );
    out.insert("text".to_string(), Value::String(text));
    Some(Value::Object(out))
}

/// `normalizePersistedPromptHistorySavedDraft`.
fn normalize_persisted_prompt_history_saved_draft(v: &Value) -> Result<Value, String> {
    if let Value::String(s) = v {
        return Ok(json!({"prompt": s, "attachments": []}));
    }
    if !truthy(v) || !is_object_type(v) {
        return Ok(Value::Null);
    }
    let prompt = opt_str(v, "prompt");
    if prop(v, "prompt").as_str().is_none() {
        return Ok(Value::Null);
    }
    let attachments = flat_norm(arr_field(v, "attachments"), |e| {
        attachments::normalize_persisted_attachment(e.clone())
    })?;
    let assistant_selections = flat_norm(arr_field(v, "assistantSelections"), |e| {
        Ok(normalize_persisted_assistant_selection(e))
    })?;
    let browser_annotations = match arr_field(v, "browserAnnotations") {
        Some(items) => domain::normalize_browser_annotations(items)?,
        None => Vec::new(),
    };
    let terminal_contexts = flat_norm(arr_field(v, "terminalContexts"), |e| {
        Ok(normalize_persisted_terminal_context_draft(e))
    })?;
    let file_comments = flat_norm(arr_field(v, "fileComments"), |e| {
        Ok(normalize_persisted_file_comment_draft(e))
    })?;
    let pasted_texts = flat_norm(arr_field(v, "pastedTexts"), |e| {
        Ok(normalize_persisted_pasted_text_draft(e))
    })?;
    let pull_request_contexts = flat_norm(arr_field(v, "pullRequestContexts"), |e| {
        Ok(normalize_persisted_pull_request_context_draft(e))
    })?;
    let skills = filter_refs(arr_field(v, "skills"));
    let mentions = filter_refs(arr_field(v, "mentions"));
    let mut out = Map::new();
    out.insert("prompt".to_string(), Value::String(prompt));
    out.insert("attachments".to_string(), Value::Array(attachments));
    if !assistant_selections.is_empty() {
        out.insert(
            "assistantSelections".to_string(),
            Value::Array(assistant_selections),
        );
    }
    if !browser_annotations.is_empty() {
        out.insert(
            "browserAnnotations".to_string(),
            Value::Array(browser_annotations),
        );
    }
    if !terminal_contexts.is_empty() {
        out.insert(
            "terminalContexts".to_string(),
            Value::Array(terminal_contexts),
        );
    }
    if !file_comments.is_empty() {
        out.insert("fileComments".to_string(), Value::Array(file_comments));
    }
    if !pasted_texts.is_empty() {
        out.insert("pastedTexts".to_string(), Value::Array(pasted_texts));
    }
    if !pull_request_contexts.is_empty() {
        out.insert(
            "pullRequestContexts".to_string(),
            Value::Array(pull_request_contexts),
        );
    }
    if !skills.is_empty() {
        out.insert("skills".to_string(), Value::Array(skills));
    }
    if !mentions.is_empty() {
        out.insert("mentions".to_string(), Value::Array(mentions));
    }
    Ok(Value::Object(out))
}

/// `Array.isArray(field) ? field.flatMap(normalize) : []`.
fn flat_norm<F>(items: Option<&Vec<Value>>, f: F) -> Result<Vec<Value>, String>
where
    F: Fn(&Value) -> Result<Option<Value>, String>,
{
    let mut out = Vec::new();
    if let Some(items) = items {
        for item in items {
            if let Some(n) = f(item)? {
                out.push(n);
            }
        }
    }
    Ok(out)
}

/// `Array.isArray(field) ? field.filter(Schema.is(Provider{Skill,Mention}Reference)) : []`.
fn filter_refs(items: Option<&Vec<Value>>) -> Vec<Value> {
    match items {
        Some(items) => items
            .iter()
            .filter(|e| is_name_path_ref(e))
            .cloned()
            .collect(),
        None => Vec::new(),
    }
}

/// `normalizePersistedQueuedTurns`.
fn normalize_persisted_queued_turns(v: &Value) -> Result<Option<Vec<Value>>, String> {
    let Value::Array(raw) = v else {
        return Ok(None);
    };
    let mut turns: Vec<Value> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    for entry in raw {
        if !truthy(entry) || !is_object_type(entry) {
            continue;
        }
        let id = opt_str(entry, "id");
        let kind = prop(entry, "kind");
        let created_at = opt_str(entry, "createdAt");
        let preview_text = opt_str(entry, "previewText");
        let selected_provider = models::normalize_provider_kind(prop(entry, "selectedProvider"))?;
        let selected_model = match prop(entry, "selectedModel") {
            Value::Null => Value::Null,
            Value::String(s) => Value::String(s),
            _ => Value::Null,
        };
        let selected_prompt_effort = match prop(entry, "selectedPromptEffort") {
            Value::Null => Value::Null,
            Value::String(s) => Value::String(s),
            _ => Value::Null,
        };
        let model_selection =
            models::normalize_model_selection(prop(entry, "modelSelection"), None)?;
        let pod = prop(entry, "providerOptionsForDispatch");
        let provider_options_for_dispatch = if is_provider_start_options(&pod) {
            Some(pod)
        } else {
            None
        };
        let spp = prop(entry, "sourceProposedPlan");
        let source_proposed_plan = if is_source_plan_ref(&spp) {
            Some(spp)
        } else {
            None
        };
        let rm = prop(entry, "runtimeMode");
        let runtime_mode = if is_runtime_mode(&rm) { Some(rm) } else { None };
        let ccm_in = prop(entry, "computerControlMode");
        let computer_control_mode = ccm::resolve_computer_control_mode(
            if is_computer_control_mode(&ccm_in) {
                ccm_in.as_str().map(|s| s.to_string())
            } else {
                None
            },
            Some(prop(entry, "enableComputerControl") == Value::Bool(true)),
        )?;
        let ccg = prop(entry, "computerControlGeneration");
        let computer_control_generation =
            matches!(js::as_number(&ccg), Some(n) if num_is_integer(n) && n >= 0.0).then_some(ccg);
        let enable_computer_control = computer_control_mode != "off";
        if id.is_empty()
            || created_at.is_empty()
            || preview_text.is_empty()
            || selected_provider.is_none()
            || model_selection.is_none()
            || runtime_mode.is_none()
            || seen.contains(&id)
        {
            continue;
        }
        let selected_provider = selected_provider.unwrap_or(Value::Null);
        let model_selection = model_selection.unwrap_or(Value::Null);
        let runtime_mode = runtime_mode.unwrap_or(Value::Null);
        if str_is(&kind, "chat") {
            let prompt = opt_str(entry, "prompt");
            let images = flat_norm(arr_field(entry, "images"), |e| {
                attachments::normalize_persisted_attachment(e.clone())
            })?;
            let terminal_contexts = flat_norm(arr_field(entry, "terminalContexts"), |e| {
                Ok(normalize_persisted_queued_terminal_context_draft(e))
            })?;
            let assistant_selections = flat_norm(arr_field(entry, "assistantSelections"), |e| {
                Ok(normalize_persisted_assistant_selection(e))
            })?;
            let browser_annotations = match arr_field(entry, "browserAnnotations") {
                Some(items) => domain::normalize_browser_annotations(items)?,
                None => Vec::new(),
            };
            let file_comments = flat_norm(arr_field(entry, "fileComments"), |e| {
                Ok(normalize_persisted_file_comment_draft(e))
            })?;
            let pasted_texts = flat_norm(arr_field(entry, "pastedTexts"), |e| {
                Ok(normalize_persisted_pasted_text_draft(e))
            })?;
            let pull_request_contexts = flat_norm(arr_field(entry, "pullRequestContexts"), |e| {
                Ok(normalize_persisted_pull_request_context_draft(e))
            })?;
            let skills = filter_refs(arr_field(entry, "skills"));
            let mentions = filter_refs(arr_field(entry, "mentions"));
            let im = prop(entry, "interactionMode");
            let interaction_mode = if is_interaction_mode(&im) {
                Some(im)
            } else {
                None
            };
            let em = prop(entry, "envMode");
            let env_mode = if is_env_mode(&em) { Some(em) } else { None };
            let (Some(interaction_mode), Some(env_mode)) = (interaction_mode, env_mode) else {
                continue;
            };
            let mut out = Map::new();
            out.insert("id".to_string(), Value::String(id.clone()));
            out.insert("kind".to_string(), json!("chat"));
            out.insert("createdAt".to_string(), Value::String(created_at.clone()));
            out.insert("previewText".to_string(), Value::String(preview_text));
            out.insert("prompt".to_string(), Value::String(prompt));
            out.insert("images".to_string(), Value::Array(images));
            if !assistant_selections.is_empty() {
                out.insert(
                    "assistantSelections".to_string(),
                    Value::Array(assistant_selections),
                );
            }
            if !browser_annotations.is_empty() {
                out.insert(
                    "browserAnnotations".to_string(),
                    Value::Array(browser_annotations),
                );
            }
            out.insert(
                "terminalContexts".to_string(),
                Value::Array(terminal_contexts),
            );
            if !file_comments.is_empty() {
                out.insert("fileComments".to_string(), Value::Array(file_comments));
            }
            if !pasted_texts.is_empty() {
                out.insert("pastedTexts".to_string(), Value::Array(pasted_texts));
            }
            if !pull_request_contexts.is_empty() {
                out.insert(
                    "pullRequestContexts".to_string(),
                    Value::Array(pull_request_contexts),
                );
            }
            out.insert("skills".to_string(), Value::Array(skills));
            out.insert("mentions".to_string(), Value::Array(mentions));
            out.insert("selectedProvider".to_string(), selected_provider);
            out.insert("selectedModel".to_string(), selected_model);
            out.insert("selectedPromptEffort".to_string(), selected_prompt_effort);
            out.insert("modelSelection".to_string(), model_selection);
            if let Some(pod) = provider_options_for_dispatch {
                out.insert("providerOptionsForDispatch".to_string(), pod);
            }
            out.insert(
                "enableComputerControl".to_string(),
                Value::Bool(enable_computer_control),
            );
            out.insert(
                "computerControlMode".to_string(),
                Value::String(computer_control_mode.clone()),
            );
            if let Some(g) = computer_control_generation {
                out.insert("computerControlGeneration".to_string(), g);
            }
            if let Some(spp) = source_proposed_plan {
                out.insert("sourceProposedPlan".to_string(), spp);
            }
            out.insert("runtimeMode".to_string(), runtime_mode);
            out.insert("interactionMode".to_string(), interaction_mode);
            out.insert("envMode".to_string(), env_mode);
            turns.push(Value::Object(out));
            seen.push(id);
            continue;
        }
        if str_is(&kind, "plan-follow-up") {
            let text = opt_str(entry, "text");
            let im = prop(entry, "interactionMode");
            let interaction_mode = if str_is(&im, "default") || str_is(&im, "plan") {
                Some(im)
            } else {
                None
            };
            let Some(interaction_mode) = interaction_mode else {
                continue;
            };
            let mut out = Map::new();
            out.insert("id".to_string(), Value::String(id.clone()));
            out.insert("kind".to_string(), json!("plan-follow-up"));
            out.insert("createdAt".to_string(), Value::String(created_at.clone()));
            out.insert("previewText".to_string(), Value::String(preview_text));
            out.insert("text".to_string(), Value::String(text));
            out.insert("interactionMode".to_string(), interaction_mode);
            out.insert("selectedProvider".to_string(), selected_provider);
            out.insert("selectedModel".to_string(), selected_model);
            out.insert("selectedPromptEffort".to_string(), selected_prompt_effort);
            out.insert("modelSelection".to_string(), model_selection);
            if let Some(pod) = provider_options_for_dispatch {
                out.insert("providerOptionsForDispatch".to_string(), pod);
            }
            out.insert(
                "enableComputerControl".to_string(),
                Value::Bool(enable_computer_control),
            );
            out.insert(
                "computerControlMode".to_string(),
                Value::String(computer_control_mode.clone()),
            );
            if let Some(g) = computer_control_generation {
                out.insert("computerControlGeneration".to_string(), g);
            }
            out.insert("runtimeMode".to_string(), runtime_mode);
            turns.push(Value::Object(out));
            seen.push(id);
        }
    }
    Ok(if turns.is_empty() { None } else { Some(turns) })
}

/// `normalizeDraftThreadEnvMode`.
fn normalize_draft_thread_env_mode(v: &Value, fallback_worktree_path: &Value) -> String {
    if str_is(v, "local") || str_is(v, "worktree") {
        return v.as_str().unwrap_or_default().to_string();
    }
    if matches!(fallback_worktree_path, Value::String(_)) {
        "worktree".to_string()
    } else {
        "local".to_string()
    }
}

type DraftThreadMaps = (Map<String, Value>, Map<String, Value>);

/// `normalizePersistedDraftThreads` → (draftThreadsByThreadId,
/// projectDraftThreadIdByProjectId).
fn normalize_persisted_draft_threads(
    raw_threads: &Value,
    raw_mappings: &Value,
) -> Result<DraftThreadMaps, String> {
    let mut draft_threads: Map<String, Value> = Map::new();
    if truthy(raw_threads) && is_object_type(raw_threads) {
        for (thread_id, raw_thread) in entries_js_order(raw_threads) {
            if thread_id.is_empty() || !truthy(&raw_thread) || !is_object_type(&raw_thread) {
                continue;
            }
            let project_id = prop(&raw_thread, "projectId");
            let created_at = prop(&raw_thread, "createdAt");
            let branch = prop(&raw_thread, "branch");
            let worktree_path = prop(&raw_thread, "worktreePath");
            let working_directory = prop(&raw_thread, "workingDirectory");
            let lkp = prop(&raw_thread, "lastKnownPr");
            let last_known_pr = if truthy(&lkp) && is_object_type(&lkp) {
                decode_thread_pull_request(&lkp)
            } else {
                None
            };
            let normalized_worktree = match &worktree_path {
                Value::String(_) => worktree_path.clone(),
                _ => Value::Null,
            };
            let goal_raw = prop(&raw_thread, "goal");
            let goal = match &goal_raw {
                Value::String(s) if !js_trim(s).is_empty() => Some(goal_raw.clone()),
                _ => None,
            };
            let is_temporary = matches!(prop(&raw_thread, "isTemporary"), Value::Bool(true));
            let promoted_raw = prop(&raw_thread, "promotedTo");
            let promoted_to = match &promoted_raw {
                Value::String(s) if !s.is_empty() => Some(promoted_raw.clone()),
                _ => None,
            };
            let Value::String(project_id) = project_id else {
                continue;
            };
            if project_id.is_empty() {
                continue;
            }
            let rm = prop(&raw_thread, "runtimeMode");
            let runtime_mode = if is_runtime_mode(&rm) {
                rm
            } else {
                json!("full-access")
            };
            let im = prop(&raw_thread, "interactionMode");
            let interaction_mode = if is_interaction_mode(&im) {
                im
            } else {
                json!("default")
            };
            let entry_point =
                domain::normalize_draft_thread_entry_point(prop(&raw_thread, "entryPoint"), None)?;
            let branch = match &branch {
                Value::String(_) => branch.clone(),
                _ => Value::Null,
            };
            let working_directory = match &working_directory {
                Value::String(_) => working_directory.clone(),
                _ => Value::Null,
            };
            let created_at = match &created_at {
                Value::String(s) if !s.is_empty() => created_at.clone(),
                _ => Value::String(now_iso()),
            };
            let mut out = Map::new();
            out.insert("projectId".to_string(), Value::String(project_id));
            out.insert("createdAt".to_string(), created_at);
            out.insert("runtimeMode".to_string(), runtime_mode);
            out.insert("interactionMode".to_string(), interaction_mode);
            out.insert("entryPoint".to_string(), entry_point);
            out.insert("branch".to_string(), branch);
            out.insert("worktreePath".to_string(), normalized_worktree.clone());
            out.insert("workingDirectory".to_string(), working_directory);
            if let Some(pr) = last_known_pr {
                out.insert("lastKnownPr".to_string(), pr);
            }
            out.insert(
                "envMode".to_string(),
                json!(normalize_draft_thread_env_mode(
                    &prop(&raw_thread, "envMode"),
                    &normalized_worktree,
                )),
            );
            if let Some(g) = goal {
                out.insert("goal".to_string(), g);
            }
            if is_temporary {
                out.insert("isTemporary".to_string(), Value::Bool(true));
            }
            if let Some(p) = promoted_to {
                out.insert("promotedTo".to_string(), p);
            }
            draft_threads.insert(thread_id, Value::Object(out));
        }
    }

    let mut mappings: Map<String, Value> = Map::new();
    if truthy(raw_mappings) && is_object_type(raw_mappings) {
        for (mapping_key, thread_id_raw) in entries_js_order(raw_mappings) {
            let project_id = domain::project_id_from_draft_thread_mapping_key(mapping_key.clone())?;
            let entry_point =
                domain::project_draft_thread_entry_point_from_key(mapping_key.clone())?;
            let (Value::String(project_id), Value::String(thread_id)) =
                (&project_id, &thread_id_raw)
            else {
                continue;
            };
            if project_id.is_empty() || thread_id.is_empty() {
                continue;
            }
            mappings.insert(mapping_key.clone(), thread_id_raw.clone());
            match draft_threads.get_mut(thread_id) {
                None => {
                    let mut out = Map::new();
                    out.insert("projectId".to_string(), Value::String(project_id.clone()));
                    out.insert("createdAt".to_string(), Value::String(now_iso()));
                    out.insert("runtimeMode".to_string(), json!("full-access"));
                    out.insert("interactionMode".to_string(), json!("default"));
                    out.insert("entryPoint".to_string(), json!(entry_point));
                    out.insert("branch".to_string(), Value::Null);
                    out.insert("worktreePath".to_string(), Value::Null);
                    out.insert("workingDirectory".to_string(), Value::Null);
                    out.insert("envMode".to_string(), json!("local"));
                    draft_threads.insert(thread_id.clone(), Value::Object(out));
                }
                Some(existing) => {
                    let existing_project = prop(existing, "projectId");
                    if existing_project != Value::String(project_id.clone()) {
                        if let Value::Object(m) = existing {
                            m.insert("projectId".to_string(), Value::String(project_id.clone()));
                        }
                    } else if prop(existing, "entryPoint") != json!(entry_point)
                        && let Value::Object(m) = existing
                    {
                        m.insert("entryPoint".to_string(), json!(entry_point));
                    }
                }
            }
        }
    }
    Ok((draft_threads, mappings))
}

/// `normalizePersistedModelSelectionMap`.
fn normalize_persisted_model_selection_map(v: &Value) -> Result<Map<String, Value>, String> {
    let mut result: Map<String, Value> = Map::new();
    if v.is_null() || is_undef(v) || !matches!(v, Value::Object(_)) {
        return Ok(result);
    }
    for (legacy_provider, raw_selection) in entries_js_order(v) {
        let legacy_model = if !raw_selection.is_null() && matches!(raw_selection, Value::Object(_))
        {
            prop(&raw_selection, "model")
        } else {
            js::undefined()
        };
        let legacy = json!({"provider": legacy_provider, "model": legacy_model});
        let selection = models::normalize_model_selection(raw_selection, Some(legacy))?;
        let key = match &selection {
            Some(sel) => models::model_selection_storage_key(sel.clone())?,
            None => js::undefined(),
        };
        if let Some(sel) = selection
            && truthy(&key)
        {
            let key_str = key.as_str().unwrap_or_default().to_string();
            if !(legacy_provider == "kilo" && result.contains_key(&key_str)) {
                result.insert(key_str, sel);
            }
        }
    }
    Ok(result)
}

/// `normalizePersistedActiveProviderInstanceId`.
fn normalize_persisted_active_provider_instance_id(
    value: &Value,
    selections: &Map<String, Value>,
) -> Result<Value, String> {
    let instance_id = models::normalize_provider_instance_id(value.clone())?;
    if !truthy(&instance_id) {
        return Ok(Value::Null);
    }
    let id_str = instance_id.as_str().unwrap_or_default();
    if selections.get(id_str).is_some_and(|v| !is_undef(v)) {
        return Ok(instance_id);
    }
    let migrated = models::normalize_provider_kind(value.clone())?;
    if let Some(m) = &migrated {
        let m_str = m.as_str().unwrap_or_default();
        if selections.get(m_str).is_some_and(|v| !is_undef(v)) {
            return Ok(m.clone());
        }
    }
    Ok(instance_id)
}

/// `normalizePersistedDraftsByThreadId`.
fn normalize_persisted_drafts_by_thread_id(raw_map: &Value) -> Result<Map<String, Value>, String> {
    let mut next: Map<String, Value> = Map::new();
    if !truthy(raw_map) || !is_object_type(raw_map) {
        return Ok(next);
    }
    for (thread_id, draft) in entries_js_order(raw_map) {
        if thread_id.is_empty() || !truthy(&draft) || !is_object_type(&draft) {
            continue;
        }
        let pending =
            pui::normalize_pending_user_input_drafts(prop(&draft, "pendingUserInputDrafts"))?;
        let pending_count = match &pending {
            Value::Object(m) => m.len(),
            _ => 0,
        };
        let prompt_candidate = opt_str(&draft, "prompt");
        let saved_draft = normalize_persisted_prompt_history_saved_draft(&prop(
            &draft,
            "promptHistorySavedDraft",
        ))?;
        let attachments = flat_norm(arr_field(&draft, "attachments"), |e| {
            attachments::normalize_persisted_attachment(e.clone())
        })?;
        let terminal_contexts = flat_norm(arr_field(&draft, "terminalContexts"), |e| {
            Ok(normalize_persisted_terminal_context_draft(e))
        })?;
        let assistant_selections = flat_norm(arr_field(&draft, "assistantSelections"), |e| {
            Ok(normalize_persisted_assistant_selection(e))
        })?;
        let browser_annotations = match arr_field(&draft, "browserAnnotations") {
            Some(items) => domain::normalize_browser_annotations(items)?,
            None => Vec::new(),
        };
        let file_comments = flat_norm(arr_field(&draft, "fileComments"), |e| {
            Ok(normalize_persisted_file_comment_draft(e))
        })?;
        let pasted_texts = flat_norm(arr_field(&draft, "pastedTexts"), |e| {
            Ok(normalize_persisted_pasted_text_draft(e))
        })?;
        let pull_request_contexts = flat_norm(arr_field(&draft, "pullRequestContexts"), |e| {
            Ok(normalize_persisted_pull_request_context_draft(e))
        })?;
        let skills = filter_refs(arr_field(&draft, "skills"));
        let mentions = filter_refs(arr_field(&draft, "mentions"));
        let queued_turns = normalize_persisted_queued_turns(&prop(&draft, "queuedTurns"))?;
        let rm = prop(&draft, "runtimeMode");
        let runtime_mode = if is_runtime_mode(&rm) { Some(rm) } else { None };
        let im = prop(&draft, "interactionMode");
        let interaction_mode = if is_interaction_mode(&im) {
            Some(im)
        } else {
            None
        };
        let ecc = prop(&draft, "enableComputerControl");
        let enable_computer_control = if matches!(ecc, Value::Bool(_)) {
            Some(ecc)
        } else {
            None
        };
        let ccm = prop(&draft, "computerControlMode");
        let computer_control_mode = if is_computer_control_mode(&ccm) {
            Some(ccm)
        } else {
            None
        };
        let ccg = prop(&draft, "computerControlGeneration");
        let computer_control_generation =
            matches!(js::as_number(&ccg), Some(n) if num_is_integer(n) && n >= 0.0).then_some(ccg);
        let prompt =
            ensure_inline_terminal_context_placeholders(&prompt_candidate, terminal_contexts.len());

        let mut model_selection_by_provider = Map::new();
        let mut active_provider = Value::Null;
        let msp = prop(&draft, "modelSelectionByProvider");
        if truthy(&msp) && is_object_type(&msp) {
            model_selection_by_provider = normalize_persisted_model_selection_map(&msp)?;
            active_provider = normalize_persisted_active_provider_instance_id(
                &prop(&draft, "activeProvider"),
                &model_selection_by_provider,
            )?;
        } else {
            let normalized_options = models::normalize_provider_model_options(
                prop(&draft, "modelOptions"),
                None,
                Some(draft.clone()),
            )?;
            let normalized_selection = models::normalize_model_selection(
                prop(&draft, "modelSelection"),
                Some(json!({
                    "provider": prop(&draft, "provider"),
                    "model": prop(&draft, "model"),
                    "modelOptions": normalized_options.clone().unwrap_or_else(|| prop(&draft, "modelOptions")),
                    "legacyCodex": draft.clone(),
                })),
            )?;
            let merged_options = models::legacy_merge_model_selection_into_provider_model_options(
                normalized_selection.clone(),
                normalized_options,
            )?;
            let model_selection = models::legacy_sync_model_selection_options(
                normalized_selection.clone().unwrap_or(Value::Null),
                merged_options.clone(),
            )?;
            let msp_map = models::legacy_to_model_selection_by_provider(
                model_selection.clone(),
                merged_options.clone(),
            )?;
            if let Value::Object(m) = msp_map {
                model_selection_by_provider = m;
            }
            if let Some(sel) = model_selection.as_ref() {
                active_provider = models::model_selection_storage_key(sel.clone())?;
            }
        }

        let pod = prop(&draft, "providerOptionsForDispatch");
        let provider_options_for_dispatch = if is_provider_start_options(&pod) {
            Some(pod)
        } else {
            None
        };
        let normalized_queued_turns = queued_turns.clone().unwrap_or_default();
        let rspp = prop(&draft, "restoredSourceProposedPlan");
        let restored_source_proposed_plan = if is_restored_plan(&rspp) {
            Some(rspp)
        } else {
            None
        };
        let has_model_data = !model_selection_by_provider.is_empty() || !active_provider.is_null();
        let has_queued_turns = !normalized_queued_turns.is_empty();
        let has_reference_data = !skills.is_empty() || !mentions.is_empty();
        if pending_count == 0
            && prompt_candidate.is_empty()
            && saved_draft.is_null()
            && attachments.is_empty()
            && terminal_contexts.is_empty()
            && assistant_selections.is_empty()
            && browser_annotations.is_empty()
            && file_comments.is_empty()
            && pasted_texts.is_empty()
            && pull_request_contexts.is_empty()
            && !has_reference_data
            && !has_queued_turns
            && restored_source_proposed_plan.is_none()
            && !has_model_data
            && provider_options_for_dispatch.is_none()
            && runtime_mode.is_none()
            && interaction_mode.is_none()
            && enable_computer_control.is_none()
            && computer_control_mode.is_none()
        {
            continue;
        }
        let mut out = Map::new();
        if pending_count > 0 {
            out.insert("pendingUserInputDrafts".to_string(), pending);
        }
        out.insert("prompt".to_string(), Value::String(prompt));
        if !saved_draft.is_null() {
            out.insert("promptHistorySavedDraft".to_string(), saved_draft);
        }
        out.insert("attachments".to_string(), Value::Array(attachments));
        if !assistant_selections.is_empty() {
            out.insert(
                "assistantSelections".to_string(),
                Value::Array(assistant_selections),
            );
        }
        if !browser_annotations.is_empty() {
            out.insert(
                "browserAnnotations".to_string(),
                Value::Array(browser_annotations),
            );
        }
        if !terminal_contexts.is_empty() {
            out.insert(
                "terminalContexts".to_string(),
                Value::Array(terminal_contexts),
            );
        }
        if !file_comments.is_empty() {
            out.insert("fileComments".to_string(), Value::Array(file_comments));
        }
        if !pasted_texts.is_empty() {
            out.insert("pastedTexts".to_string(), Value::Array(pasted_texts));
        }
        if !pull_request_contexts.is_empty() {
            out.insert(
                "pullRequestContexts".to_string(),
                Value::Array(pull_request_contexts),
            );
        }
        if !skills.is_empty() {
            out.insert("skills".to_string(), Value::Array(skills));
        }
        if !mentions.is_empty() {
            out.insert("mentions".to_string(), Value::Array(mentions));
        }
        if has_queued_turns {
            out.insert(
                "queuedTurns".to_string(),
                Value::Array(normalized_queued_turns),
            );
            if let Value::String(s) = prop(&draft, "queueStoppedTurnId") {
                out.insert("queueStoppedTurnId".to_string(), Value::String(s));
            }
            if let Value::String(s) = prop(&draft, "queueResumedTurnId") {
                out.insert("queueResumedTurnId".to_string(), Value::String(s));
            }
        }
        if let Some(plan) = restored_source_proposed_plan {
            out.insert("restoredSourceProposedPlan".to_string(), plan);
        }
        if has_model_data {
            out.insert(
                "modelSelectionByProvider".to_string(),
                Value::Object(model_selection_by_provider),
            );
            out.insert("activeProvider".to_string(), active_provider);
        }
        if let Some(pod) = provider_options_for_dispatch {
            out.insert("providerOptionsForDispatch".to_string(), pod);
        }
        if let Some(rm) = runtime_mode {
            out.insert("runtimeMode".to_string(), rm);
        }
        if let Some(im) = interaction_mode {
            out.insert("interactionMode".to_string(), im);
        }
        if let Some(ecc) = enable_computer_control {
            out.insert("enableComputerControl".to_string(), ecc);
        }
        if let Some(ccm) = computer_control_mode {
            out.insert("computerControlMode".to_string(), ccm);
        }
        if let Some(g) = computer_control_generation {
            out.insert("computerControlGeneration".to_string(), g);
        }
        next.insert(thread_id, Value::Object(out));
    }
    Ok(next)
}

// ── Hydration ───────────────────────────────────────────────────────────────

/// `attachments.flatMap` member check, then `hydrateImagesFromPersisted`.
fn hydrate_images(v: &Value) -> Result<Vec<Value>, String> {
    let items = match v {
        Value::Null => {
            return Err("null is not an object (evaluating 'attachments.flatMap')".to_string());
        }
        _ if is_undef(v) => {
            return Err(
                "undefined is not an object (evaluating 'attachments.flatMap')".to_string(),
            );
        }
        Value::Array(items) => items.clone(),
        _ => {
            return Err(
                "attachments.flatMap is not a function. (In 'attachments.flatMap((attachment) => {', 'attachments.flatMap' is undefined)".to_string(),
            );
        }
    };
    attachments::hydrate_images_from_persisted(items)
}

/// `hydratePromptHistorySavedDraft`.
fn hydrate_prompt_history_saved_draft(saved_draft: &Value) -> Result<Value, String> {
    if is_undef(saved_draft) {
        return Ok(Value::Null);
    }
    if let Value::String(prompt) = saved_draft {
        return Ok(json!({
            "prompt": prompt,
            "images": [],
            "files": [],
            "nonPersistedImageIds": [],
            "persistedAttachments": [],
            "assistantSelections": [],
            "browserAnnotations": [],
            "terminalContexts": [],
            "fileComments": [],
            "pastedTexts": [],
            "pullRequestContexts": [],
            "skills": [],
            "mentions": [],
        }));
    }
    let attachments = prop_eval(saved_draft, "attachments", "savedDraft.attachments")?;
    let attachments = if nullish(&attachments) {
        Value::Array(Vec::new())
    } else {
        attachments
    };
    let mut out = Map::new();
    out.insert(
        "prompt".to_string(),
        prop_eval(saved_draft, "prompt", "savedDraft.prompt")?,
    );
    out.insert(
        "images".to_string(),
        Value::Array(hydrate_images(&attachments)?),
    );
    out.insert("files".to_string(), Value::Array(Vec::new()));
    out.insert("nonPersistedImageIds".to_string(), Value::Array(Vec::new()));
    out.insert(
        "persistedAttachments".to_string(),
        spread_array(&attachments, "attachments")?,
    );
    out.insert(
        "assistantSelections".to_string(),
        Value::Array(domain::normalize_assistant_selections(array_or_empty(
            &prop_eval(
                saved_draft,
                "assistantSelections",
                "savedDraft.assistantSelections",
            )?,
        )?)?),
    );
    out.insert(
        "browserAnnotations".to_string(),
        Value::Array(domain::normalize_browser_annotations(&array_or_empty(
            &prop_eval(
                saved_draft,
                "browserAnnotations",
                "savedDraft.browserAnnotations",
            )?,
        )?)?),
    );
    out.insert(
        "terminalContexts".to_string(),
        map_terminal_contexts_blank(saved_draft, "savedDraft.terminalContexts")?,
    );
    out.insert(
        "fileComments".to_string(),
        Value::Array(domain::normalize_file_comments(array_or_empty(
            &prop_eval(saved_draft, "fileComments", "savedDraft.fileComments")?,
        )?)?),
    );
    out.insert(
        "pastedTexts".to_string(),
        Value::Array(domain::hydrate_pasted_texts_from_persisted(prop_eval(
            saved_draft,
            "pastedTexts",
            "savedDraft.pastedTexts",
        )?)?),
    );
    out.insert(
        "pullRequestContexts".to_string(),
        Value::Array(domain::normalize_pull_request_contexts(&array_or_empty(
            &prop_eval(
                saved_draft,
                "pullRequestContexts",
                "savedDraft.pullRequestContexts",
            )?,
        )?)?),
    );
    out.insert(
        "skills".to_string(),
        spread_array(
            &prop_eval(saved_draft, "skills", "savedDraft.skills")?,
            "savedDraft.skills",
        )?,
    );
    out.insert(
        "mentions".to_string(),
        spread_array(
            &prop_eval(saved_draft, "mentions", "savedDraft.mentions")?,
            "savedDraft.mentions",
        )?,
    );
    Ok(Value::Object(out))
}

/// `v ?? []` with `Array` extraction; other values pass through to error paths.
fn array_or_empty(v: &Value) -> Result<Vec<Value>, String> {
    if nullish(v) {
        return Ok(Vec::new());
    }
    match v {
        Value::Array(items) => Ok(items.clone()),
        // `[...(non-array)]` / `.map` on non-array → not iterable/not a function.
        Value::String(s) => Ok(s.chars().map(|c| Value::String(c.to_string())).collect()),
        _ => Err("value is not iterable".to_string()),
    }
}

/// `[...v]` for the `?? []` handled arrays: strings iterate chars.
fn spread_array(v: &Value, name: &str) -> Result<Value, String> {
    if nullish(v) {
        return Ok(Value::Array(Vec::new()));
    }
    match v {
        Value::Array(items) => Ok(Value::Array(items.clone())),
        Value::String(s) => Ok(Value::Array(
            s.chars().map(|c| Value::String(c.to_string())).collect(),
        )),
        _ => Err(format!("{name} is not iterable")),
    }
}

/// `persistedDraft.terminalContexts?.map((context) => ({...context, text: ""}))`.
fn map_terminal_contexts_blank(receiver: &Value, expr: &str) -> Result<Value, String> {
    let contexts = prop_eval(receiver, "terminalContexts", expr)?;
    if nullish(&contexts) {
        return Ok(Value::Array(Vec::new()));
    }
    let Value::Array(items) = contexts else {
        return Err(format!(
            "{expr}?.map is not a function. (In '{expr}?.map((context) => {{', '{expr}?.map' is undefined)"
        ));
    };
    let mut out = Vec::new();
    for context in &items {
        let mut map = spread_to_map(context);
        map.insert("text".to_string(), Value::String(String::new()));
        out.push(Value::Object(map));
    }
    Ok(Value::Array(out))
}

/// `hydrateQueuedTurnsFromPersisted`.
fn hydrate_queued_turns(thread_id: &Value, queued_turns: &Value) -> Result<Vec<Value>, String> {
    if !truthy(queued_turns) {
        return Ok(Vec::new());
    }
    let items = match queued_turns {
        Value::Array(items) => items.clone(),
        _ => {
            return Err(
                "queuedTurns.map is not a function. (In 'queuedTurns.map((queuedTurn) => {', 'queuedTurns.map' is undefined)".to_string(),
            );
        }
    };
    let mut out = Vec::new();
    for queued_turn in &items {
        if str_is(&prop(queued_turn, "kind"), "chat") {
            let mut map = spread_to_map(queued_turn);
            map.insert(
                "images".to_string(),
                Value::Array(hydrate_images(&prop(queued_turn, "images"))?),
            );
            map.insert("files".to_string(), Value::Array(Vec::new()));
            map.insert(
                "assistantSelections".to_string(),
                Value::Array(domain::normalize_assistant_selections(array_or_empty(
                    &prop(queued_turn, "assistantSelections"),
                )?)?),
            );
            map.insert(
                "browserAnnotations".to_string(),
                Value::Array(domain::normalize_browser_annotations(&array_or_empty(
                    &prop(queued_turn, "browserAnnotations"),
                )?)?),
            );
            let contexts = prop(queued_turn, "terminalContexts");
            map.insert(
                "terminalContexts".to_string(),
                Value::Array(match contexts {
                    Value::Array(items) => {
                        domain::normalize_terminal_contexts_for_thread(thread_id.clone(), items)?
                    }
                    _ => Vec::new(),
                }),
            );
            map.insert(
                "fileComments".to_string(),
                Value::Array(domain::normalize_file_comments(array_or_empty(&prop(
                    queued_turn,
                    "fileComments",
                ))?)?),
            );
            map.insert(
                "pastedTexts".to_string(),
                Value::Array(domain::hydrate_pasted_texts_from_persisted(prop(
                    queued_turn,
                    "pastedTexts",
                ))?),
            );
            map.insert(
                "pullRequestContexts".to_string(),
                Value::Array(domain::normalize_pull_request_contexts(&array_or_empty(
                    &prop(queued_turn, "pullRequestContexts"),
                )?)?),
            );
            map.insert(
                "skills".to_string(),
                spread_array(&prop(queued_turn, "skills"), "queuedTurn.skills")?,
            );
            map.insert(
                "mentions".to_string(),
                spread_array(&prop(queued_turn, "mentions"), "queuedTurn.mentions")?,
            );
            out.push(Value::Object(map));
        } else {
            out.push(Value::Object(spread_to_map(queued_turn)));
        }
    }
    Ok(out)
}

// ── Exported ports ──────────────────────────────────────────────────────────

fn empty_persisted_state() -> Value {
    json!({
        "draftsByThreadId": {},
        "draftThreadsByThreadId": {},
        "projectDraftThreadIdByProjectId": {},
        "stickyModelSelectionByProvider": {},
        "stickyActiveProvider": null,
        "stickyRuntimeMode": null,
    })
}

/// Port of `normalizeCurrentPersistedComposerDraftStoreState`
/// (`apps/web/src/composerDraftPersistence.ts:1548`).
pub fn normalize_current_persisted_composer_draft_store_state(
    persisted_state: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !truthy(&persisted_state) || !is_object_type(&persisted_state) {
        return Ok(empty_persisted_state());
    }
    let (draft_threads, mappings) = normalize_persisted_draft_threads(
        &prop(&persisted_state, "draftThreadsByThreadId"),
        &prop(&persisted_state, "projectDraftThreadIdByProjectId"),
    )?;

    let mut sticky_map = Map::new();
    let mut sticky_active = Value::Null;
    let sticky_raw = prop(&persisted_state, "stickyModelSelectionByProvider");
    if truthy(&sticky_raw) && is_object_type(&sticky_raw) {
        sticky_map = normalize_persisted_model_selection_map(&sticky_raw)?;
        sticky_active = normalize_persisted_active_provider_instance_id(
            &prop(&persisted_state, "stickyActiveProvider"),
            &sticky_map,
        )?;
    } else {
        let sticky_model_options = models::normalize_provider_model_options(
            prop(&persisted_state, "stickyModelOptions"),
            None,
            None,
        )?
        .unwrap_or_else(|| json!({}));
        let sticky_provider = prop(&persisted_state, "stickyProvider");
        let normalized_sticky = models::normalize_model_selection(
            prop(&persisted_state, "stickyModelSelection"),
            Some(json!({
                "provider": if nullish(&sticky_provider) { json!("codex") } else { sticky_provider },
                "model": prop(&persisted_state, "stickyModel"),
                "modelOptions": sticky_model_options.clone(),
            })),
        )?;
        let next_options = models::legacy_merge_model_selection_into_provider_model_options(
            normalized_sticky.clone(),
            Some(sticky_model_options),
        )?;
        let sticky_selection = models::legacy_sync_model_selection_options(
            normalized_sticky.unwrap_or(Value::Null),
            next_options.clone(),
        )?;
        let msp = models::legacy_to_model_selection_by_provider(
            sticky_selection.clone(),
            next_options.clone(),
        )?;
        if let Value::Object(m) = msp {
            sticky_map = m;
        }
        if let Some(sel) = sticky_selection {
            sticky_active = models::model_selection_storage_key(sel)?;
        }
    }

    let drafts =
        normalize_persisted_drafts_by_thread_id(&prop(&persisted_state, "draftsByThreadId"))?;
    let sticky_sanitized = models::sanitize_sticky_model_selection_map(Value::Object(sticky_map))?;
    let sticky_runtime = prop(&persisted_state, "stickyRuntimeMode");
    let sticky_runtime = if is_runtime_mode(&sticky_runtime) {
        sticky_runtime
    } else {
        Value::Null
    };
    let mut out = Map::new();
    out.insert("draftsByThreadId".to_string(), Value::Object(drafts));
    out.insert(
        "draftThreadsByThreadId".to_string(),
        Value::Object(draft_threads),
    );
    out.insert(
        "projectDraftThreadIdByProjectId".to_string(),
        Value::Object(mappings),
    );
    out.insert(
        "stickyModelSelectionByProvider".to_string(),
        sticky_sanitized,
    );
    out.insert("stickyActiveProvider".to_string(), sticky_active);
    out.insert("stickyRuntimeMode".to_string(), sticky_runtime);
    Ok(Value::Object(out))
}

/// Port of `migratePersistedComposerDraftStoreState`
/// (`apps/web/src/composerDraftPersistence.ts:1198`).
pub fn migrate_persisted_composer_draft_store_state(
    persisted_state: serde_json::Value,
) -> Result<serde_json::Value, String> {
    normalize_current_persisted_composer_draft_store_state(persisted_state)
}

/// Port of `toHydratedThreadDraft` (`apps/web/src/composerDraftPersistence.ts:1688`).
pub fn to_hydrated_thread_draft(
    thread_id: serde_json::Value,
    persisted_draft: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let msp_raw = prop_eval(
        &persisted_draft,
        "modelSelectionByProvider",
        "persistedDraft.modelSelectionByProvider",
    )?;
    let msp_raw = if nullish(&msp_raw) {
        json!({})
    } else {
        msp_raw
    };
    let model_selection_by_provider = models::normalize_model_selection_map_by_instance(msp_raw)?;
    let msp_map = match &model_selection_by_provider {
        Value::Object(m) => m.clone(),
        _ => Map::new(),
    };
    let active_provider = normalize_persisted_active_provider_instance_id(
        &prop_eval(
            &persisted_draft,
            "activeProvider",
            "persistedDraft.activeProvider",
        )?,
        &msp_map,
    )?;

    let mut out = Map::new();
    let pending_raw = prop_eval(
        &persisted_draft,
        "pendingUserInputDrafts",
        "persistedDraft.pendingUserInputDrafts",
    )?;
    if truthy(&pending_raw) {
        out.insert(
            "pendingUserInputDrafts".to_string(),
            pui::normalize_pending_user_input_drafts(pending_raw)?,
        );
    }
    out.insert(
        "prompt".to_string(),
        prop_eval(&persisted_draft, "prompt", "persistedDraft.prompt")?,
    );
    out.insert(
        "promptHistorySavedDraft".to_string(),
        hydrate_prompt_history_saved_draft(&prop_eval(
            &persisted_draft,
            "promptHistorySavedDraft",
            "persistedDraft.promptHistorySavedDraft",
        )?)?,
    );
    out.insert(
        "images".to_string(),
        Value::Array(hydrate_images(&prop_eval(
            &persisted_draft,
            "attachments",
            "persistedDraft.attachments",
        )?)?),
    );
    out.insert("files".to_string(), Value::Array(Vec::new()));
    out.insert("nonPersistedImageIds".to_string(), Value::Array(Vec::new()));
    out.insert(
        "persistedAttachments".to_string(),
        spread_array(
            &prop_eval(
                &persisted_draft,
                "attachments",
                "persistedDraft.attachments",
            )?,
            "persistedDraft.attachments",
        )?,
    );
    out.insert(
        "assistantSelections".to_string(),
        Value::Array(domain::normalize_assistant_selections(array_or_empty(
            &prop_eval(
                &persisted_draft,
                "assistantSelections",
                "persistedDraft.assistantSelections",
            )?,
        )?)?),
    );
    out.insert(
        "browserAnnotations".to_string(),
        Value::Array(domain::normalize_browser_annotations(&array_or_empty(
            &prop_eval(
                &persisted_draft,
                "browserAnnotations",
                "persistedDraft.browserAnnotations",
            )?,
        )?)?),
    );
    out.insert(
        "terminalContexts".to_string(),
        map_terminal_contexts_blank(&persisted_draft, "persistedDraft.terminalContexts")?,
    );
    out.insert(
        "fileComments".to_string(),
        Value::Array(domain::normalize_file_comments(array_or_empty(
            &prop_eval(
                &persisted_draft,
                "fileComments",
                "persistedDraft.fileComments",
            )?,
        )?)?),
    );
    out.insert(
        "pastedTexts".to_string(),
        Value::Array(domain::hydrate_pasted_texts_from_persisted(prop_eval(
            &persisted_draft,
            "pastedTexts",
            "persistedDraft.pastedTexts",
        )?)?),
    );
    out.insert(
        "pullRequestContexts".to_string(),
        Value::Array(domain::normalize_pull_request_contexts(&array_or_empty(
            &prop_eval(
                &persisted_draft,
                "pullRequestContexts",
                "persistedDraft.pullRequestContexts",
            )?,
        )?)?),
    );
    out.insert(
        "skills".to_string(),
        spread_array(
            &prop_eval(&persisted_draft, "skills", "persistedDraft.skills")?,
            "persistedDraft.skills",
        )?,
    );
    out.insert(
        "mentions".to_string(),
        spread_array(
            &prop_eval(&persisted_draft, "mentions", "persistedDraft.mentions")?,
            "persistedDraft.mentions",
        )?,
    );
    out.insert(
        "queuedTurns".to_string(),
        Value::Array(hydrate_queued_turns(
            &thread_id,
            &prop_eval(
                &persisted_draft,
                "queuedTurns",
                "persistedDraft.queuedTurns",
            )?,
        )?),
    );
    let qst = prop_eval(
        &persisted_draft,
        "queueStoppedTurnId",
        "persistedDraft.queueStoppedTurnId",
    )?;
    if truthy(&qst) {
        out.insert("queueStoppedTurnId".to_string(), qst);
    }
    let qrt = prop_eval(
        &persisted_draft,
        "queueResumedTurnId",
        "persistedDraft.queueResumedTurnId",
    )?;
    if truthy(&qrt) {
        out.insert("queueResumedTurnId".to_string(), qrt);
    }
    let rspp = prop_eval(
        &persisted_draft,
        "restoredSourceProposedPlan",
        "persistedDraft.restoredSourceProposedPlan",
    )?;
    out.insert(
        "restoredSourceProposedPlan".to_string(),
        if nullish(&rspp) { Value::Null } else { rspp },
    );
    out.insert(
        "modelSelectionByProvider".to_string(),
        model_selection_by_provider,
    );
    out.insert("activeProvider".to_string(), active_provider);
    let rm = prop_eval(
        &persisted_draft,
        "runtimeMode",
        "persistedDraft.runtimeMode",
    )?;
    out.insert(
        "runtimeMode".to_string(),
        if nullish(&rm) { Value::Null } else { rm },
    );
    let im = prop_eval(
        &persisted_draft,
        "interactionMode",
        "persistedDraft.interactionMode",
    )?;
    out.insert(
        "interactionMode".to_string(),
        if nullish(&im) { Value::Null } else { im },
    );
    out.insert(
        "computerControlMode".to_string(),
        prop_eval(
            &persisted_draft,
            "computerControlMode",
            "persistedDraft.computerControlMode",
        )?,
    );
    out.insert(
        "computerControlGeneration".to_string(),
        prop_eval(
            &persisted_draft,
            "computerControlGeneration",
            "persistedDraft.computerControlGeneration",
        )?,
    );
    let ecc = prop_eval(
        &persisted_draft,
        "enableComputerControl",
        "persistedDraft.enableComputerControl",
    )?;
    out.insert(
        "enableComputerControl".to_string(),
        if matches!(ecc, Value::Bool(_)) {
            ecc
        } else {
            js::undefined()
        },
    );
    Ok(Value::Object(out))
}
mod models {
    #![allow(dead_code)]
    //! The Rust port of `apps/web/src/composerDraftModels.ts`.

    #![forbid(unsafe_code)]

    use serde_json::{Map, Value};

    use crate::js;

    // ── Static model/provider data (from packages/contracts and packages/shared) ──

    const PROVIDER_KINDS: &[&str] = &[
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

    const COMPOSER_PROVIDER_KINDS: &[&str] = &[
        "codex",
        "claudeAgent",
        "cursor",
        "devin",
        "antigravity",
        "grok",
        "droid",
        "opencode",
        "pi",
        "omp",
    ];

    const DEFAULT_MODEL_BY_PROVIDER: &[(&str, &str)] = &[
        ("codex", "gpt-6-astra"),
        ("claudeAgent", "claude-sonnet-5"),
        ("cursor", "auto"),
        ("devin", "adaptive"),
        ("antigravity", "Gemini 3.5 Flash"),
        ("grok", "grok-4.6"),
        ("droid", "claude-opus-4-8"),
        ("opencode", "openai/gpt-5"),
    ];

    /// `MODEL_OPTIONS_BY_PROVIDER` (slug, name), in declaration order.
    const MODEL_OPTIONS: &[(&str, &[(&str, &str)])] = &[
        (
            "codex",
            &[
                ("gpt-6-astra", "GPT-6 Astra"),
                ("gpt-6-sol", "GPT-6 Sol"),
                ("gpt-6-luna", "GPT-6 Luna"),
                ("gpt-5.5", "GPT-5.5"),
                ("gpt-5.4", "GPT-5.4"),
                ("gpt-5.4-mini", "GPT-5.4 Mini"),
                ("gpt-5.3-codex", "GPT-5.3 Codex"),
                ("gpt-5.3-codex-spark", "GPT-5.3 Codex Spark"),
                ("gpt-5.2-codex", "GPT-5.2 Codex"),
                ("gpt-5.2", "GPT-5.2"),
            ],
        ),
        (
            "claudeAgent",
            &[
                ("claude-fable-5-1", "Claude Fable 5.1"),
                ("claude-fable-5", "Claude Fable 5"),
                ("claude-opus-5-5", "Claude Opus 5.5"),
                ("claude-opus-5", "Claude Opus 5"),
                ("claude-opus-4-8", "Claude Opus 4.8"),
                ("claude-opus-4-7", "Claude Opus 4.7"),
                ("claude-opus-4-6", "Claude Opus 4.6"),
                ("claude-opus-4-5", "Claude Opus 4.5"),
                ("claude-sonnet-5", "Claude Sonnet 5"),
                ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                ("claude-haiku-4-5", "Claude Haiku 4.5"),
            ],
        ),
        ("antigravity", &[]),
        ("grok", &[("grok-4.6", "Grok 4.6")]),
        (
            "droid",
            &[
                ("auto", "Auto Model"),
                ("claude-fable-5", "Claude Fable 5"),
                ("claude-opus-4-8", "Claude Opus 4.8"),
                ("claude-opus-4-8-fast", "Claude Opus 4.8 Fast"),
                ("claude-opus-4-7", "Claude Opus 4.7"),
                ("claude-opus-4-7-fast", "Claude Opus 4.7 Fast"),
                ("claude-opus-4-6", "Claude Opus 4.6"),
                ("claude-sonnet-5", "Claude Sonnet 5"),
                ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                ("claude-opus-4-5-20251101", "Claude Opus 4.5"),
                ("claude-sonnet-4-5-20250929", "Claude Sonnet 4.5"),
                ("claude-haiku-4-5-20251001", "Claude Haiku 4.5"),
                ("gpt-5.6-sol", "GPT-5.6 Sol"),
                ("gpt-5.6-terra", "GPT-5.6 Terra"),
                ("gpt-5.6-luna", "GPT-5.6 Luna"),
                ("gpt-5.5", "GPT-5.5"),
                ("gpt-5.5-fast", "GPT-5.5 Fast"),
                ("gpt-5.5-pro", "GPT-5.5 Pro"),
                ("gpt-5.4", "GPT-5.4"),
                ("gpt-5.4-fast", "GPT-5.4 Fast"),
                ("gpt-5.4-mini", "GPT-5.4 Mini"),
                ("gpt-5.3-codex", "GPT-5.3 Codex"),
                ("gpt-5.3-codex-fast", "GPT-5.3 Codex Fast"),
                ("gpt-5.2", "GPT-5.2"),
                ("gemini-3.1-pro-preview", "Gemini 3.1 Pro"),
                ("gemini-3.5-flash", "Gemini 3.5 Flash"),
                ("gemini-3-flash-preview", "Gemini 3 Flash"),
                ("glm-5.2", "GLM 5.2"),
                ("glm-5.2-fast", "GLM 5.2 Fast"),
                ("glm-5.1", "GLM 5.1"),
                ("nemotron-3-ultra", "Nemotron 3 Ultra"),
                ("kimi-k2.7-code", "Kimi K2.7 Code"),
                ("kimi-k2.6", "Kimi K2.6"),
                ("deepseek-v4-pro", "DeepSeek V4 Pro"),
                ("deepseek-v4-flash-0731", "DeepSeek V4 Flash 0731"),
                ("minimax-m3", "MiniMax M3"),
                ("minimax-m2.7", "MiniMax M2.7"),
            ],
        ),
        ("opencode", &[("openai/gpt-5", "OpenAI GPT-5")]),
        ("pi", &[]),
        (
            "cursor",
            &[
                ("auto", "Auto"),
                ("composer-2.5", "Composer 2.5"),
                ("claude-opus-5", "Claude Opus 5"),
                ("claude-opus-4-8", "Claude Opus 4.8"),
                ("claude-opus-4-7", "Claude Opus 4.7"),
                ("claude-opus-4-6", "Claude Opus 4.6"),
                ("claude-opus-4-5", "Claude Opus 4.5"),
                ("claude-fable-5", "Claude Fable 5"),
                ("claude-sonnet-5", "Claude Sonnet 5"),
                ("claude-sonnet-4-6", "Claude Sonnet 4.6"),
                ("claude-sonnet-4-5", "Claude Sonnet 4.5"),
                ("claude-sonnet-4", "Claude Sonnet 4"),
                ("claude-haiku-4-5", "Claude Haiku 4.5"),
                ("gpt-5.6-sol", "GPT-5.6 Sol"),
                ("gpt-5.6-terra", "GPT-5.6 Terra"),
                ("gpt-5.6-luna", "GPT-5.6 Luna"),
                ("gpt-5.5", "GPT-5.5"),
                ("gpt-5.4", "GPT-5.4"),
                ("gpt-5.4-mini", "GPT-5.4 Mini"),
                ("gpt-5.4-nano", "GPT-5.4 Nano"),
                ("gpt-5.3-codex", "GPT-5.3 Codex"),
                ("gpt-5.2", "GPT-5.2"),
                ("gpt-5.1", "GPT-5.1"),
                ("gpt-5-mini", "GPT-5 Mini"),
                ("grok-4.5", "Grok 4.5"),
                ("grok-4.6", "Grok 4.6"),
                ("gemini-3.1-pro", "Gemini 3.1 Pro"),
                ("gemini-3.6-flash", "Gemini 3.6 Flash"),
                ("gemini-3.5-flash", "Gemini 3.5 Flash"),
                ("gemini-3-flash", "Gemini 3 Flash"),
                ("gemini-2.5-flash", "Gemini 2.5 Flash"),
                ("kimi-k2.7-code", "Kimi K2.7 Code"),
                ("glm-5.2", "GLM 5.2"),
            ],
        ),
        (
            "devin",
            &[
                ("adaptive", "Adaptive"),
                ("swe-1-6", "SWE 1.6"),
                ("swe-1-7", "SWE 1.7"),
            ],
        ),
        ("omp", &[]),
    ];

    /// `MODEL_SLUG_SET_BY_PROVIDER`: like `MODEL_OPTIONS`, but devin (and the dynamic
    /// providers) intentionally keep an empty set.
    fn model_slug_set(provider: &str) -> Option<&'static [&'static str]> {
        Some(match provider {
            "claudeAgent" => CLAUDE_SLUGS,
            "codex" => CODEX_SLUGS,
            "cursor" => CURSOR_SLUGS,
            "antigravity" => &[],
            "grok" => &["grok-4.6"],
            "droid" => DROID_SLUGS,
            "opencode" => &["openai/gpt-5"],
            "pi" | "devin" | "omp" => &[],
            _ => return None,
        })
    }

    const CODEX_SLUGS: &[&str] = &[
        "gpt-6-astra",
        "gpt-6-sol",
        "gpt-6-luna",
        "gpt-5.5",
        "gpt-5.4",
        "gpt-5.4-mini",
        "gpt-5.3-codex",
        "gpt-5.3-codex-spark",
        "gpt-5.2-codex",
        "gpt-5.2",
    ];
    const CLAUDE_SLUGS: &[&str] = &[
        "claude-fable-5-1",
        "claude-fable-5",
        "claude-opus-5-5",
        "claude-opus-5",
        "claude-opus-4-8",
        "claude-opus-4-7",
        "claude-opus-4-6",
        "claude-opus-4-5",
        "claude-sonnet-5",
        "claude-sonnet-4-6",
        "claude-haiku-4-5",
    ];
    const DROID_SLUGS: &[&str] = &[
        "auto",
        "claude-fable-5",
        "claude-opus-4-8",
        "claude-opus-4-8-fast",
        "claude-opus-4-7",
        "claude-opus-4-7-fast",
        "claude-opus-4-6",
        "claude-sonnet-5",
        "claude-sonnet-4-6",
        "claude-opus-4-5-20251101",
        "claude-sonnet-4-5-20250929",
        "claude-haiku-4-5-20251001",
        "gpt-5.6-sol",
        "gpt-5.6-terra",
        "gpt-5.6-luna",
        "gpt-5.5",
        "gpt-5.5-fast",
        "gpt-5.5-pro",
        "gpt-5.4",
        "gpt-5.4-fast",
        "gpt-5.4-mini",
        "gpt-5.3-codex",
        "gpt-5.3-codex-fast",
        "gpt-5.2",
        "gemini-3.1-pro-preview",
        "gemini-3.5-flash",
        "gemini-3-flash-preview",
        "glm-5.2",
        "glm-5.2-fast",
        "glm-5.1",
        "nemotron-3-ultra",
        "kimi-k2.7-code",
        "kimi-k2.6",
        "deepseek-v4-pro",
        "deepseek-v4-flash-0731",
        "minimax-m3",
        "minimax-m2.7",
    ];
    const CURSOR_SLUGS: &[&str] = &[
        "auto",
        "composer-2.5",
        "claude-opus-5",
        "claude-opus-4-8",
        "claude-opus-4-7",
        "claude-opus-4-6",
        "claude-opus-4-5",
        "claude-fable-5",
        "claude-sonnet-5",
        "claude-sonnet-4-6",
        "claude-sonnet-4-5",
        "claude-sonnet-4",
        "claude-haiku-4-5",
        "gpt-5.6-sol",
        "gpt-5.6-terra",
        "gpt-5.6-luna",
        "gpt-5.5",
        "gpt-5.4",
        "gpt-5.4-mini",
        "gpt-5.4-nano",
        "gpt-5.3-codex",
        "gpt-5.2",
        "gpt-5.1",
        "gpt-5-mini",
        "grok-4.5",
        "grok-4.6",
        "gemini-3.1-pro",
        "gemini-3.6-flash",
        "gemini-3.5-flash",
        "gemini-3-flash",
        "gemini-2.5-flash",
        "kimi-k2.7-code",
        "glm-5.2",
    ];

    /// `MODEL_SLUG_ALIASES_BY_PROVIDER` (lowercased lookup keys).
    fn model_aliases(provider: &str) -> Option<&'static [(&'static str, &'static str)]> {
        Some(match provider {
            "codex" => ALIASES_CODEX,
            "claudeAgent" => ALIASES_CLAUDE,
            "cursor" => ALIASES_CURSOR,
            "antigravity" => &[],
            "droid" => ALIASES_DROID,
            "grok" => ALIASES_GROK,
            "opencode" => &[],
            "pi" => &[],
            "devin" => ALIASES_DEVIN,
            "omp" => &[],
            _ => return None,
        })
    }

    const ALIASES_CODEX: &[(&str, &str)] = &[
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
    ];
    const ALIASES_CLAUDE: &[(&str, &str)] = &[
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
    ];
    const ALIASES_CURSOR: &[(&str, &str)] = &[
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
    ];
    const ALIASES_DROID: &[(&str, &str)] = &[
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
    ];
    const ALIASES_GROK: &[(&str, &str)] = &[
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
    ];
    const ALIASES_DEVIN: &[(&str, &str)] = &[
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
    ];

    /// Reasoning-effort ladder values + the default entry, per model slug
    /// (`MODEL_CAPABILITIES_INDEX`, collapsed to the fields the port reads).
    type EffCaps = (&'static [&'static str], Option<&'static str>);
    const CAPS_EMPTY: EffCaps = (&[], None);

    fn caps_index(provider: &str, slug: &str) -> Option<EffCaps> {
        let table: &[(&str, EffCaps)] = match provider {
            "codex" => CAPS_CODEX,
            "claudeAgent" => CAPS_CLAUDE,
            "cursor" => CAPS_CURSOR,
            "droid" => CAPS_DROID,
            "grok" => CAPS_GROK,
            "opencode" => CAPS_OPENCODE,
            "devin" => CAPS_DEVIN,
            "antigravity" | "pi" | "omp" => &[],
            _ => return None,
        };
        table.iter().find(|(s, _)| *s == slug).map(|(_, c)| *c)
    }

    const CAPS_CODEX: &[(&str, EffCaps)] = &[
        (
            "gpt-6-astra",
            (
                &["low", "medium", "high", "xhigh", "max", "ultra"],
                Some("medium"),
            ),
        ),
        (
            "gpt-6-sol",
            (
                &["low", "medium", "high", "xhigh", "max", "ultra"],
                Some("medium"),
            ),
        ),
        (
            "gpt-6-luna",
            (&["low", "medium", "high", "xhigh", "max"], Some("medium")),
        ),
        (
            "gpt-5.5",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.4-mini",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.3-codex",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.3-codex-spark",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.2-codex",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.2",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
    ];

    const CAPS_CLAUDE: &[(&str, EffCaps)] = &[
        (
            "claude-fable-5-1",
            (
                &["low", "medium", "high", "xhigh", "max", "ultracode"],
                Some("high"),
            ),
        ),
        (
            "claude-fable-5",
            (
                &["low", "medium", "high", "xhigh", "max", "ultracode"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-5-5",
            (
                &["low", "medium", "high", "xhigh", "max", "ultracode"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-5",
            (
                &["low", "medium", "high", "xhigh", "max", "ultracode"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-8",
            (
                &[
                    "low",
                    "medium",
                    "high",
                    "xhigh",
                    "max",
                    "ultrathink",
                    "ultracode",
                ],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-7",
            (
                &[
                    "low",
                    "medium",
                    "high",
                    "xhigh",
                    "max",
                    "ultrathink",
                    "ultracode",
                ],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-6",
            (
                &["low", "medium", "high", "max", "ultrathink"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-5",
            (&["low", "medium", "high"], Some("high")),
        ),
        (
            "claude-sonnet-5",
            (
                &["low", "medium", "high", "xhigh", "max", "ultracode"],
                Some("high"),
            ),
        ),
        (
            "claude-sonnet-4-6",
            (
                &["low", "medium", "high", "max", "ultrathink"],
                Some("high"),
            ),
        ),
        ("claude-haiku-4-5", CAPS_EMPTY),
    ];

    const CAPS_GROK: &[(&str, EffCaps)] = &[
        (
            "grok-4.6",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "grok-build-0.1",
            (&["none", "low", "medium", "high"], Some("low")),
        ),
        (
            "grok-build",
            (&["none", "low", "medium", "high"], Some("low")),
        ),
        ("grok-4.5", (&["low", "medium", "high"], Some("high"))),
    ];

    const CAPS_DROID: &[(&str, EffCaps)] = &[
        ("auto", CAPS_EMPTY),
        (
            "claude-fable-5",
            (
                &["off", "low", "medium", "high", "xhigh", "max"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-8",
            (
                &["off", "low", "medium", "high", "xhigh", "max"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-8-fast",
            (
                &["off", "low", "medium", "high", "xhigh", "max"],
                Some("high"),
            ),
        ),
        (
            "claude-opus-4-7",
            (&["off", "low", "medium", "high", "max"], Some("high")),
        ),
        (
            "claude-opus-4-7-fast",
            (&["off", "low", "medium", "high", "max"], Some("high")),
        ),
        (
            "claude-opus-4-6",
            (&["off", "low", "medium", "high", "max"], Some("high")),
        ),
        (
            "claude-sonnet-5",
            (
                &["off", "low", "medium", "high", "xhigh", "max"],
                Some("high"),
            ),
        ),
        (
            "claude-sonnet-4-6",
            (&["off", "low", "medium", "high", "max"], Some("high")),
        ),
        (
            "claude-opus-4-5-20251101",
            (&["off", "low", "medium", "high"], Some("off")),
        ),
        (
            "claude-sonnet-4-5-20250929",
            (&["off", "low", "medium", "high"], Some("off")),
        ),
        (
            "claude-haiku-4-5-20251001",
            (&["off", "low", "medium", "high"], Some("off")),
        ),
        (
            "gpt-5.6-sol",
            (
                &["none", "low", "medium", "high", "xhigh", "max"],
                Some("medium"),
            ),
        ),
        (
            "gpt-5.6-terra",
            (
                &["none", "low", "medium", "high", "xhigh", "max"],
                Some("medium"),
            ),
        ),
        (
            "gpt-5.6-luna",
            (
                &["none", "low", "medium", "high", "xhigh", "max"],
                Some("medium"),
            ),
        ),
        (
            "gpt-5.5",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.5-fast",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.5-pro",
            (&["medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4-fast",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4-mini",
            (&["none", "low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.3-codex",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.3-codex-fast",
            (&["low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.2",
            (&["off", "low", "medium", "high", "xhigh"], Some("off")),
        ),
        (
            "gemini-3.1-pro-preview",
            (&["low", "medium", "high"], Some("high")),
        ),
        (
            "gemini-3.5-flash",
            (&["minimal", "low", "medium", "high"], Some("high")),
        ),
        (
            "gemini-3-flash-preview",
            (&["minimal", "low", "medium", "high"], Some("high")),
        ),
        ("glm-5.2", (&["off", "high"], Some("high"))),
        ("glm-5.2-fast", (&["off", "high"], Some("high"))),
        ("glm-5.1", (&["off", "high"], Some("high"))),
        ("nemotron-3-ultra", (&["off", "high"], Some("high"))),
        ("kimi-k2.7-code", (&["off", "high"], Some("high"))),
        ("kimi-k2.6", (&["off", "high"], Some("high"))),
        (
            "deepseek-v4-pro",
            (&["off", "low", "high", "max"], Some("high")),
        ),
        (
            "deepseek-v4-flash-0731",
            (&["off", "low", "high", "max"], Some("high")),
        ),
        ("minimax-m3", (&["high"], Some("high"))),
        ("minimax-m2.7", (&["high"], Some("high"))),
    ];

    const CAPS_CURSOR: &[(&str, EffCaps)] = &[
        ("auto", CAPS_EMPTY),
        ("composer-2.5", CAPS_EMPTY),
        (
            "claude-opus-5",
            (&["low", "medium", "high", "xhigh", "max"], Some("high")),
        ),
        (
            "claude-opus-4-8",
            (&["low", "medium", "high", "xhigh", "max"], Some("high")),
        ),
        (
            "claude-opus-4-7",
            (&["low", "medium", "high", "xhigh", "max"], Some("high")),
        ),
        ("claude-opus-4-6", (&["high", "max"], Some("high"))),
        ("claude-opus-4-5", (&["high"], Some("high"))),
        (
            "claude-fable-5",
            (&["low", "medium", "high", "xhigh", "max"], Some("high")),
        ),
        (
            "claude-sonnet-5",
            (&["low", "medium", "high", "xhigh", "max"], Some("high")),
        ),
        ("claude-sonnet-4-6", (&["medium"], Some("medium"))),
        ("claude-sonnet-4-5", CAPS_EMPTY),
        ("claude-sonnet-4", CAPS_EMPTY),
        ("claude-haiku-4-5", CAPS_EMPTY),
        (
            "gpt-5.6-sol",
            (
                &["none", "low", "medium", "high", "xhigh", "max"],
                Some("medium"),
            ),
        ),
        (
            "gpt-5.6-terra",
            (
                &["none", "low", "medium", "high", "xhigh", "max"],
                Some("medium"),
            ),
        ),
        (
            "gpt-5.6-luna",
            (
                &["none", "low", "medium", "high", "xhigh", "max"],
                Some("medium"),
            ),
        ),
        (
            "gpt-5.5",
            (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4",
            (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4-mini",
            (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.4-nano",
            (&["none", "low", "medium", "high", "xhigh"], Some("medium")),
        ),
        (
            "gpt-5.3-codex",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        (
            "gpt-5.2",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        ("gpt-5.1", (&["low", "medium", "high"], Some("high"))),
        ("gpt-5-mini", CAPS_EMPTY),
        ("grok-4.5", (&["low", "medium", "high"], Some("high"))),
        (
            "grok-4.6",
            (&["low", "medium", "high", "xhigh"], Some("high")),
        ),
        ("gemini-3.1-pro", CAPS_EMPTY),
        (
            "gemini-3.6-flash",
            (&["minimal", "low", "medium", "high"], Some("high")),
        ),
        ("gemini-3.5-flash", CAPS_EMPTY),
        ("gemini-3-flash", CAPS_EMPTY),
        ("gemini-2.5-flash", CAPS_EMPTY),
        ("kimi-k2.7-code", CAPS_EMPTY),
        ("glm-5.2", (&["high", "max"], Some("high"))),
    ];

    const CAPS_OPENCODE: &[(&str, EffCaps)] = &[("openai/gpt-5", CAPS_EMPTY)];
    const CAPS_DEVIN: &[(&str, EffCaps)] = &[
        ("adaptive", CAPS_EMPTY),
        ("swe-1-6", CAPS_EMPTY),
        ("swe-1-7", CAPS_EMPTY),
    ];

    const GROK_EFFORT_SET: &[&str] = &["none", "low", "medium", "high", "xhigh"];
    const ANTIGRAVITY_EFFORT_SET: &[&str] = &["low", "medium", "high", "thinking"];
    const CLAUDE_EFFORT_SET: &[&str] = &[
        "low",
        "medium",
        "high",
        "xhigh",
        "max",
        "ultrathink",
        "ultracode",
    ];
    const PI_THINKING_SET: &[&str] = &["off", "minimal", "low", "medium", "high", "xhigh", "max"];
    const OMP_THINKING_SET: &[&str] = &[
        "off", "auto", "minimal", "low", "medium", "high", "xhigh", "max",
    ];

    const MODEL_TOKEN_DISPLAY_NAMES: &[(&str, &str)] = &[
        ("deepseek", "DeepSeek"),
        ("glm", "GLM"),
        ("gpt", "GPT"),
        ("minimax", "MiniMax"),
        ("openai", "OpenAI"),
        ("opencode", "OpenCode"),
        ("swe", "SWE"),
        ("xai", "xAI"),
        ("xhigh", "XHigh"),
    ];

    const MAX_CUSTOM_MODEL_COUNT: usize = 32;
    const MAX_CUSTOM_MODEL_LENGTH: usize = 256;

    // ── JS value semantics ───────────────────────────────────────────────────────

    fn is_undef(v: &Value) -> bool {
        js::is_undefined(v)
    }

    fn is_nullish(v: &Value) -> bool {
        v.is_null() || is_undef(v)
    }

    /// UTF-16 units of a JS string: `Value::String` or a single-key `{"$utf16": ...}`.
    fn str_units(v: &Value) -> Option<Vec<u16>> {
        match v {
            Value::String(s) => Some(s.encode_utf16().collect()),
            Value::Object(o) if o.len() == 1 => o.get("$utf16").map(|x| match x {
                Value::Array(a) => a
                    .iter()
                    .map(|u| u.as_f64().map(|n| n as i64 as u16).unwrap_or(0))
                    .collect(),
                Value::String(s) => s.encode_utf16().collect(),
                _ => Vec::new(),
            }),
            _ => None,
        }
    }

    fn units_lossy(u: &[u16]) -> String {
        String::from_utf16_lossy(u)
    }

    fn lower_string(u: &[u16]) -> String {
        units_lossy(u).to_lowercase()
    }

    /// `Number(x)` for a marker payload.
    fn js_to_number(v: &Value) -> f64 {
        match v {
            Value::Null => 0.0,
            Value::Bool(b) => {
                if *b {
                    1.0
                } else {
                    0.0
                }
            }
            Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
            Value::String(s) => str_to_number(s),
            Value::Array(a) => str_to_number(
                &a.iter()
                    .map(|x| {
                        if is_nullish(x) {
                            String::new()
                        } else {
                            js_to_string(x)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            _ => f64::NAN,
        }
    }

    fn js_to_string(v: &Value) -> String {
        match v {
            Value::Null => "null".into(),
            Value::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) => s.clone(),
            Value::Array(a) => a
                .iter()
                .map(|x| {
                    if is_nullish(x) {
                        String::new()
                    } else {
                        js_to_string(x)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            _ => "[object Object]".into(),
        }
    }

    fn str_to_number(s: &str) -> f64 {
        let t = s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}');
        if t.is_empty() {
            return 0.0;
        }
        match t {
            "Infinity" | "+Infinity" => return f64::INFINITY,
            "-Infinity" => return f64::NEG_INFINITY,
            _ => {}
        }
        if let Some(h) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            return u64::from_str_radix(h, 16)
                .map(|x| x as f64)
                .unwrap_or(f64::NAN);
        }
        if let Some(o) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
            return u64::from_str_radix(o, 8)
                .map(|x| x as f64)
                .unwrap_or(f64::NAN);
        }
        if let Some(b) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
            return u64::from_str_radix(b, 2)
                .map(|x| x as f64)
                .unwrap_or(f64::NAN);
        }
        t.parse::<f64>().unwrap_or(f64::NAN)
    }

    /// A JS number: `Value::Number` or a single-key `{"$num": ...}` marker.
    fn js_num(v: &Value) -> Option<f64> {
        match v {
            Value::Number(n) => n.as_f64(),
            Value::Object(o) if o.len() == 1 => o.get("$num").map(js_to_number),
            _ => None,
        }
    }

    /// BigInt digits for a single-key `{"$bigint": "..."}` marker.
    fn bigint_norm(v: &Value) -> Option<String> {
        let Value::Object(o) = v else { return None };
        if o.len() != 1 {
            return None;
        }
        let s = o.get("$bigint")?;
        let digits = match s {
            Value::String(s) => s.clone(),
            Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or_default()),
            _ => return None,
        };
        let (neg, d) = match digits.strip_prefix('-') {
            Some(r) => (true, r),
            None => (false, digits.as_str()),
        };
        let d = d.trim_start_matches('0');
        if d.is_empty() {
            return Some("0".into());
        }
        Some(if neg { format!("-{d}") } else { d.to_string() })
    }

    fn truthy(v: &Value) -> bool {
        match v {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
            Value::String(s) => !s.is_empty(),
            Value::Array(_) => true,
            Value::Object(o) => {
                if is_undef(v) {
                    return false;
                }
                if o.len() == 1 {
                    if let Some(x) = o.get("$num") {
                        let n = js_to_number(x);
                        return n != 0.0 && !n.is_nan();
                    }
                    if o.contains_key("$utf16") {
                        return !str_units(v).unwrap_or_default().is_empty();
                    }
                    if let Some(x) = o.get("$bigint") {
                        return bigint_norm(&Value::Object(
                            [(String::from("$bigint"), x.clone())].into_iter().collect(),
                        ))
                        .is_some_and(|d| d != "0");
                    }
                }
                true
            }
        }
    }

    /// JS `===` for JSON-transport values. Objects/arrays compare by identity,
    /// which can never hold between two decoded values.
    fn strict_eq(a: &Value, b: &Value) -> bool {
        let ua = is_undef(a);
        let ub = is_undef(b);
        if ua || ub {
            return ua && ub;
        }
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Null, _) | (_, Value::Null) | (Value::Bool(_), _) | (_, Value::Bool(_)) => {
                false
            }
            _ => {
                if let Some(x) = js_num(a) {
                    return js_num(b).is_some_and(|y| x == y);
                }
                if js_num(b).is_some() {
                    return false;
                }
                if let Some(x) = str_units(a) {
                    return str_units(b).is_some_and(|y| x == y);
                }
                if str_units(b).is_some() {
                    return false;
                }
                if let Some(x) = bigint_norm(a) {
                    return bigint_norm(b).is_some_and(|y| x == y);
                }
                false
            }
        }
    }

    /// `v === s` for a string literal.
    fn str_eq(v: &Value, s: &str) -> bool {
        str_units(v).is_some_and(|u| u == s.encode_utf16().collect::<Vec<_>>())
    }

    /// `typeof v === "object"` (the truthy gate is applied by callers).
    fn is_object_type(v: &Value) -> bool {
        match v {
            Value::Null => true,
            Value::Array(_) => true,
            Value::Object(o) => {
                if is_undef(v) {
                    return false;
                }
                !(o.len() == 1
                    && (o.contains_key("$num")
                        || o.contains_key("$utf16")
                        || o.contains_key("$bigint")))
            }
            _ => false,
        }
    }

    // ── Property access and object enumeration ───────────────────────────────────

    fn array_index_key(k: &str) -> Option<usize> {
        if k.is_empty() || k.len() > 10 {
            return None;
        }
        if !k.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if k.len() > 1 && k.starts_with('0') {
            return None;
        }
        let n: u64 = k.parse().ok()?;
        (n < 4294967295).then_some(n as usize)
    }

    /// `v[key]`; missing properties read as the `{"$undefined": true}` marker.
    fn js_get(v: &Value, key: &str) -> Value {
        match v {
            Value::Null | Value::Bool(_) | Value::Number(_) => js::undefined(),
            Value::String(s) => string_get(&s.encode_utf16().collect::<Vec<_>>(), key),
            Value::Array(a) => {
                if key == "length" {
                    return js::number(a.len() as f64);
                }
                if let Some(i) = array_index_key(key) {
                    return a.get(i).cloned().unwrap_or_else(js::undefined);
                }
                js::undefined()
            }
            Value::Object(o) => {
                if is_undef(v) {
                    return js::undefined();
                }
                if o.len() == 1 {
                    if o.contains_key("$num")
                        || o.contains_key("$bigint")
                        || o.contains_key("$map")
                        || o.contains_key("$set")
                        || o.contains_key("$date")
                    {
                        return js::undefined();
                    }
                    if o.contains_key("$utf16") {
                        return string_get(&str_units(v).unwrap_or_default(), key);
                    }
                    if let Some(b) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                        return bytes_get(b, key);
                    }
                }
                o.get(key).cloned().unwrap_or_else(js::undefined)
            }
        }
    }

    fn string_get(units: &[u16], key: &str) -> Value {
        if key == "length" {
            return js::number(units.len() as f64);
        }
        if let Some(i) = array_index_key(key) {
            return units
                .get(i)
                .map(|u| js::utf16_string(&[*u]))
                .unwrap_or_else(js::undefined);
        }
        js::undefined()
    }

    fn bytes_get(b: &Value, key: &str) -> Value {
        let items: Vec<Value> = b.as_array().cloned().unwrap_or_default();
        if key == "length" {
            return js::number(items.len() as f64);
        }
        if let Some(i) = array_index_key(key) {
            return items.get(i).cloned().unwrap_or_else(js::undefined);
        }
        js::undefined()
    }

    /// `String(v)` restricted to property-key usage.
    fn prop_key(v: &Value) -> Result<String, String> {
        Ok(match v {
            Value::Null => "null".into(),
            Value::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
            Value::String(s) => s.clone(),
            Value::Array(a) => a
                .iter()
                .map(|x| {
                    if is_nullish(x) {
                        Ok(String::new())
                    } else {
                        prop_key(x)
                    }
                })
                .collect::<Result<Vec<_>, _>>()?
                .join(","),
            Value::Object(o) => {
                if is_undef(v) {
                    "undefined".into()
                } else if o.len() == 1 {
                    if let Some(x) = o.get("$num") {
                        js::number_to_string(js_to_number(x))
                    } else if o.contains_key("$utf16") {
                        units_lossy(&str_units(v).unwrap_or_default())
                    } else if let Some(x) = o.get("$bigint") {
                        bigint_norm(v).unwrap_or_else(|| js_to_string(x))
                    } else if o.contains_key("$map") {
                        "[object Map]".into()
                    } else if o.contains_key("$set") {
                        "[object Set]".into()
                    } else if o.contains_key("$date") {
                        "[object Date]".into()
                    } else if let Some(b) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                        b.as_array()
                            .map(|a| {
                                a.iter()
                                    .map(|x| {
                                        x.as_f64()
                                            .map(js::number_to_string)
                                            .unwrap_or_else(|| "NaN".into())
                                    })
                                    .collect::<Vec<_>>()
                                    .join(",")
                            })
                            .unwrap_or_default()
                    } else if o.contains_key("$symbol") {
                        return Err("TypeError: Cannot convert a Symbol value to a string".into());
                    } else {
                        "[object Object]".into()
                    }
                } else {
                    "[object Object]".into()
                }
            }
        })
    }

    /// Own enumerable string-keyed property names, JS order: integer indices
    /// ascending, then the rest in insertion order.
    fn own_keys(o: &Map<String, Value>) -> Vec<String> {
        let mut ints: Vec<(usize, String)> = Vec::new();
        let mut rest: Vec<String> = Vec::new();
        for k in o.keys() {
            if let Some(n) = array_index_key(k) {
                ints.push((n, k.clone()));
            } else {
                rest.push(k.clone());
            }
        }
        ints.sort_by_key(|(n, _)| *n);
        ints.into_iter().map(|(_, k)| k).chain(rest).collect()
    }

    fn finalize_map(m: Map<String, Value>) -> Map<String, Value> {
        let mut out = Map::new();
        for k in own_keys(&m) {
            if let Some(v) = m.get(&k) {
                out.insert(k, v.clone());
            }
        }
        out
    }

    fn index_props(units: &[u16]) -> Vec<(String, Value)> {
        units
            .iter()
            .enumerate()
            .map(|(i, u)| (i.to_string(), js::utf16_string(&[*u])))
            .collect()
    }

    /// `{...v}` — own enumerable string-keyed properties.
    fn spread_props(v: &Value) -> Vec<(String, Value)> {
        match v {
            Value::Null | Value::Bool(_) | Value::Number(_) => Vec::new(),
            Value::String(s) => index_props(&s.encode_utf16().collect::<Vec<_>>()),
            Value::Array(a) => a
                .iter()
                .enumerate()
                .map(|(i, x)| (i.to_string(), x.clone()))
                .collect(),
            Value::Object(o) => {
                if is_undef(v) {
                    return Vec::new();
                }
                if o.len() == 1 {
                    if o.contains_key("$num")
                        || o.contains_key("$bigint")
                        || o.contains_key("$map")
                        || o.contains_key("$set")
                        || o.contains_key("$date")
                    {
                        return Vec::new();
                    }
                    if o.contains_key("$utf16") {
                        return index_props(&str_units(v).unwrap_or_default());
                    }
                    if let Some(b) = o.get("$bytes").or_else(|| o.get("$buffer")) {
                        return b
                            .as_array()
                            .map(|a| {
                                a.iter()
                                    .enumerate()
                                    .map(|(i, x)| (i.to_string(), x.clone()))
                                    .collect()
                            })
                            .unwrap_or_default();
                    }
                }
                own_keys(o)
                    .into_iter()
                    .map(|k| (k.clone(), o.get(&k).cloned().unwrap_or(Value::Null)))
                    .collect()
            }
        }
    }

    fn spread_to_map(v: &Value) -> Map<String, Value> {
        spread_props(v).into_iter().collect()
    }

    fn object_values(v: &Value) -> Result<Vec<Value>, String> {
        if is_nullish(v) {
            return Err(
                "TypeError: Object.values requires that input parameter not be null or undefined"
                    .into(),
            );
        }
        Ok(spread_props(v).into_iter().map(|(_, x)| x).collect())
    }

    fn object_entries(v: &Value) -> Result<Vec<(String, Value)>, String> {
        if is_nullish(v) {
            return Err(
                "TypeError: Object.entries requires that input parameter not be null or undefined"
                    .into(),
            );
        }
        Ok(spread_props(v))
    }

    fn object_keys(v: &Value) -> Result<Vec<String>, String> {
        if is_nullish(v) {
            return Err("TypeError: null is not an object (evaluating 'Object.keys(x)')".into());
        }
        Ok(spread_props(v).into_iter().map(|(k, _)| k).collect())
    }

    // ── ES string helpers on UTF-16 units ────────────────────────────────────────

    fn is_es_ws(u: u16) -> bool {
        matches!(
            u,
            0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000
                ..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF
        )
    }

    fn trim_units(u: &[u16]) -> &[u16] {
        let mut s = 0;
        let mut e = u.len();
        while s < e && is_es_ws(u[s]) {
            s += 1;
        }
        while e > s && is_es_ws(u[e - 1]) {
            e -= 1;
        }
        &u[s..e]
    }

    fn trim_string_or_undefined(v: &Value) -> Option<Vec<u16>> {
        let units = str_units(v)?;
        let t = trim_units(&units);
        (!t.is_empty()).then(|| t.to_vec())
    }

    fn ends_with_units(u: &[u16], suffix: &str) -> bool {
        let s: Vec<u16> = suffix.encode_utf16().collect();
        u.len() >= s.len() && u[u.len() - s.len()..] == s[..]
    }

    /// `model.replace(/\[([^\]]+)\]$/, "")` — strips a trailing non-empty `[...]`.
    fn strip_bracket_suffix(u: &[u16]) -> Vec<u16> {
        if u.len() < 3 || u[u.len() - 1] != b']' as u16 {
            return u.to_vec();
        }
        for p in 0..u.len() - 2 {
            if u[p] == b'[' as u16 && !u[p + 1..u.len() - 1].contains(&(b']' as u16)) {
                return u[..p].to_vec();
            }
        }
        u.to_vec()
    }

    /// `model.replace(/\[[^\]]*\]$/, "")` — like above but the inner may be empty.
    fn strip_bracket_suffix_star(u: &[u16]) -> Vec<u16> {
        if u.len() < 2 || u[u.len() - 1] != b']' as u16 {
            return u.to_vec();
        }
        for p in 0..u.len() - 1 {
            if u[p] == b'[' as u16 && !u[p + 1..u.len() - 1].contains(&(b']' as u16)) {
                return u[..p].to_vec();
            }
        }
        u.to_vec()
    }

    /// `getClaudeContextWindowSuffix` — the inner of a trailing `[...]` group.
    fn claude_ctx_suffix(u: &[u16]) -> Option<Vec<u16>> {
        if u.len() < 3 || u[u.len() - 1] != b']' as u16 {
            return None;
        }
        for p in 0..u.len() - 2 {
            if u[p] == b'[' as u16 && !u[p + 1..u.len() - 1].contains(&(b']' as u16)) {
                return Some(u[p + 1..u.len() - 1].to_vec());
            }
        }
        None
    }

    fn is_provider_slug(u: &[u16]) -> bool {
        !u.is_empty()
            && u.len() <= 64
            && matches!(u[0], 65..=90 | 97..=122)
            && u[1..]
                .iter()
                .all(|&c| matches!(c, 65..=90 | 97..=122 | 48..=57 | 95 | 45))
    }

    fn normalize_instance_id(v: &Value) -> Option<String> {
        let t = trim_string_or_undefined(v)?;
        is_provider_slug(&t).then(|| units_lossy(&t))
    }

    fn js_repr(v: &Value) -> String {
        match v {
            Value::Null => "null".into(),
            Value::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Value::Number(n) => format!("{} ", js::number_to_string(n.as_f64().unwrap_or(0.0))),
            Value::String(s) => format!("{s:?}"),
            Value::Array(_) => "([])".into(),
            Value::Object(o) => {
                if is_undef(v) {
                    return "void 0".into();
                }
                if o.is_empty() {
                    return "({})".into();
                }
                let inner = o
                    .iter()
                    .map(|(k, x)| format!("{k}: {}", js_repr_short(x)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({{ {inner} }})")
            }
        }
    }

    fn js_repr_short(v: &Value) -> String {
        match v {
            Value::Object(o) if !is_undef(v) => {
                if o.is_empty() {
                    return "{}".into();
                }
                let inner = o
                    .iter()
                    .map(|(k, x)| format!("{k}: {}", js_repr_short(x)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{ {inner} }}")
            }
            Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
            _ => js_repr(v),
        }
    }

    fn nullish_word(v: &Value) -> &'static str {
        if v.is_null() { "null" } else { "undefined" }
    }

    // ── Model helpers (ports of packages/shared/src/model.ts internals) ──────────

    fn default_model_for(provider: &Value) -> Value {
        if str_eq(provider, "pi") || str_eq(provider, "omp") {
            return Value::Null;
        }
        match prop_key(provider)
            .ok()
            .and_then(|k| DEFAULT_MODEL_BY_PROVIDER.iter().find(|(p, _)| *p == k))
        {
            Some((_, m)) => Value::String((*m).to_string()),
            None => js::undefined(),
        }
    }

    fn model_options(provider: &str) -> Option<&'static [(&'static str, &'static str)]> {
        MODEL_OPTIONS
            .iter()
            .find(|(p, _)| *p == provider)
            .map(|(_, o)| *o)
    }

    fn model_name_by_slug(lower_key: &str) -> Option<&'static str> {
        // Last match wins (Object.values(...).flat() into a Map).
        let mut found = None;
        for (_, options) in MODEL_OPTIONS {
            for (slug, name) in *options {
                if slug.to_lowercase() == lower_key {
                    found = Some(*name);
                }
            }
        }
        found
    }

    fn normalize_model_slug(
        model: Option<Vec<u16>>,
        provider: &str,
    ) -> Result<Option<Vec<u16>>, String> {
        let Some(model) = model else {
            return Ok(None);
        };
        let trimmed = trim_units(&model).to_vec();
        if trimmed.is_empty() {
            return Ok(None);
        }
        let provider_scoped = if provider == "claudeAgent" {
            strip_bracket_suffix(&trimmed)
        } else if provider == "devin"
            && lower_string(&trimmed) == units_lossy(&trimmed)
            && ends_with_units(&trimmed, "-medium")
        {
            trimmed[..trimmed.len() - "-medium".len()].to_vec()
        } else {
            trimmed.clone()
        };
        let alias_key = lower_string(&provider_scoped);
        let Some(aliases) = model_aliases(provider) else {
            return Err(format!(
                "TypeError: undefined is not an object (evaluating 'Object.prototype.hasOwnProperty.call(void 0, {})')",
                serde_json::to_string(&alias_key).unwrap_or_else(|_| "\"?\"".into())
            ));
        };
        let mut normalized = aliases
            .iter()
            .find(|(k, _)| *k == alias_key)
            .map(|(_, s)| s.encode_utf16().collect::<Vec<u16>>())
            .unwrap_or(provider_scoped);
        if provider == "claudeAgent"
            && claude_ctx_suffix(&trimmed)
                .map(|s| lower_string(&s))
                .as_deref()
                == Some("1m")
        {
            normalized.extend("[1m]".encode_utf16());
        }
        Ok(Some(normalized))
    }

    fn resolve_model_slug(
        model: Option<Vec<u16>>,
        provider: &str,
    ) -> Result<Option<Vec<u16>>, String> {
        let normalized_model = normalize_model_slug(model, provider)?;
        let normalized = if provider == "claudeAgent" {
            normalized_model.map(|m| strip_bracket_suffix(&m))
        } else {
            normalized_model
        };
        if provider == "devin" || provider == "pi" || provider == "omp" {
            return Ok(normalized);
        }
        let Some(n) = normalized else {
            return Ok(default_model_units(provider));
        };
        let Some(set) = model_slug_set(provider) else {
            return Err("TypeError: undefined is not an object (evaluating '(void 0).has')".into());
        };
        let ns = units_lossy(&n);
        if set.contains(&ns.as_str()) {
            Ok(Some(n))
        } else {
            Ok(default_model_units(provider))
        }
    }

    fn default_model_units(provider: &str) -> Option<Vec<u16>> {
        DEFAULT_MODEL_BY_PROVIDER
            .iter()
            .find(|(p, _)| *p == provider)
            .map(|(_, m)| m.encode_utf16().collect())
    }

    fn resolve_grok_effort_family(slug: &[u16]) -> &'static str {
        let s = lower_string(slug);
        let (major, minor) = grok_version(&s).unwrap_or((-1, -1));
        let versioned = major > 4 || (major == 4 && minor >= 5);
        if !versioned
            && (s.contains("build")
                || s.contains("code-fast")
                || s == "grok-4"
                || s == "grok-4.3"
                || s.starts_with("grok-4.3-"))
        {
            return "build";
        }
        if major < 0 {
            return "build";
        }
        if major < 4 || (major == 4 && minor <= 3) {
            return "build";
        }
        if major == 4 && minor == 5 {
            return "4.5";
        }
        "4.6"
    }

    /// First `/grok-(\d+)\.(\d+)/` occurrence.
    fn grok_version(s: &str) -> Option<(i64, i64)> {
        let b = s.as_bytes();
        let mut i = 0;
        while i + 5 <= b.len() {
            if &b[i..i + 5] == b"grok-" {
                let mut j = i + 5;
                let a0 = j;
                while j < b.len() && b[j].is_ascii_digit() {
                    j += 1;
                }
                if j > a0 && j < b.len() && b[j] == b'.' {
                    let m0 = j + 1;
                    let mut k = m0;
                    while k < b.len() && b[k].is_ascii_digit() {
                        k += 1;
                    }
                    if k > m0 {
                        let major: i64 = s[a0..j].parse().ok()?;
                        let minor: i64 = s[m0..k].parse().ok()?;
                        return Some((major, minor));
                    }
                }
            }
            i += 1;
        }
        None
    }

    fn grok_caps_for_family(family: &str) -> EffCaps {
        match family {
            "build" => caps_index("grok", "grok-build").unwrap_or(CAPS_EMPTY),
            "4.5" => caps_index("grok", "grok-4.5")
                .or_else(|| caps_index("grok", "grok-4.6"))
                .unwrap_or(CAPS_EMPTY),
            _ => caps_index("grok", "grok-4.6").unwrap_or(CAPS_EMPTY),
        }
    }

    fn parse_claude_family_version(slug: &[u16]) -> Option<(&'static str, i64, i64)> {
        let s = lower_string(trim_units(slug));
        let rest = s.strip_prefix("claude-")?;
        let (family, rest) = if let Some(r) = rest.strip_prefix("fable-") {
            ("fable", r)
        } else if let Some(r) = rest.strip_prefix("opus-") {
            ("opus", r)
        } else if let Some(r) = rest.strip_prefix("sonnet-") {
            ("sonnet", r)
        } else {
            ("haiku", rest.strip_prefix("haiku-")?)
        };
        let b = rest.as_bytes();
        let mut i = 0;
        while i < b.len() && b[i].is_ascii_digit() && i < 2 {
            i += 1;
        }
        if i == 0 {
            return None;
        }
        let major: i64 = rest[..i].parse().ok()?;
        let mut minor = 0i64;
        let mut rest = &rest[i..];
        if let Some(r) = rest.strip_prefix('-') {
            let digits: String = r
                .chars()
                .take(2)
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if !digits.is_empty() {
                minor = digits.parse().ok()?;
                rest = &r[digits.len()..];
            }
        }
        if let Some(r) = rest.strip_prefix('-')
            && r.len() == 8
            && r.bytes().all(|c| c.is_ascii_digit())
        {
            rest = "";
        }
        rest.is_empty().then_some((family, major, minor))
    }

    /// Newest catalog model per Claude family, derived from the claudeAgent catalog.
    fn newest_claude_catalog(family: &str) -> (&'static str, i64, i64) {
        match family {
            "fable" => ("claude-fable-5-1", 5, 1),
            "opus" => ("claude-opus-5-5", 5, 5),
            "sonnet" => ("claude-sonnet-5", 5, 0),
            _ => ("claude-haiku-4-5", 4, 5),
        }
    }

    fn resolve_newest_known_claude_family_model(
        slug: &[u16],
    ) -> Result<Option<&'static str>, String> {
        let Some(normalized) = normalize_model_slug(Some(slug.to_vec()), "claudeAgent")? else {
            return Ok(None);
        };
        let s = strip_bracket_suffix(&normalized);
        if CLAUDE_SLUGS.contains(&units_lossy(&s).as_str()) {
            return Ok(None);
        }
        let Some((family, major, minor)) = parse_claude_family_version(&s) else {
            return Ok(None);
        };
        let (nslug, nmaj, nmin) = newest_claude_catalog(family);
        Ok(if (major, minor) > (nmaj, nmin) {
            Some(nslug)
        } else {
            None
        })
    }

    fn get_model_capabilities(provider: &str, model: &Value) -> Result<EffCaps, String> {
        let normalized_slug = normalize_model_slug(str_units(model), provider)?;
        let slug = if provider == "claudeAgent" {
            normalized_slug.map(|s| strip_bracket_suffix(&s))
        } else {
            normalized_slug
        };
        if let Some(slug) = &slug
            && let Some(c) = caps_index(provider, &units_lossy(slug))
        {
            return Ok(c);
        }
        match provider {
            "grok" => {
                if let Some(slug) = &slug {
                    return Ok(grok_caps_for_family(resolve_grok_effort_family(slug)));
                }
            }
            "claudeAgent" => {
                if let Some(slug) = &slug
                    && let Some(newest) = resolve_newest_known_claude_family_model(slug)?
                {
                    return Ok(caps_index("claudeAgent", newest).unwrap_or(CAPS_EMPTY));
                }
            }
            _ => {}
        }
        Ok(CAPS_EMPTY)
    }

    fn units_eq_str(u: &[u16], s: &str) -> bool {
        u == s.encode_utf16().collect::<Vec<_>>().as_slice()
    }

    fn has_effort(caps: EffCaps, effort: &[u16]) -> bool {
        caps.0.iter().any(|l| units_eq_str(effort, l))
    }

    fn normalize_grok_model_options(model: &Value, opts: &Value) -> Result<Value, String> {
        let caps = get_model_capabilities("grok", model)?;
        let effort = if is_nullish(opts) {
            None
        } else {
            trim_string_or_undefined(&js_get(opts, "reasoningEffort"))
        };
        let Some(effort) = effort else {
            return Ok(js::undefined());
        };
        if !has_effort(caps, &effort) {
            return Ok(js::undefined());
        }
        if caps.1.is_some_and(|d| units_eq_str(&effort, d)) {
            return Ok(js::undefined());
        }
        let mut o = Map::new();
        o.insert("reasoningEffort".into(), js::utf16_string(&effort));
        Ok(Value::Object(o))
    }

    fn classify_provider_effort(
        provider: &Value,
        model: &Value,
        effort: &Value,
    ) -> Result<&'static str, String> {
        let Some(e) = trim_string_or_undefined(effort) else {
            return Ok("unsupported");
        };
        let caps = get_model_capabilities(&prop_key(provider)?, model)?;
        if caps.0.is_empty() {
            return Ok("unknown");
        }
        Ok(if has_effort(caps, &e) {
            "supported"
        } else {
            "unsupported"
        })
    }

    // ── Display names (packages/shared/src/modelName.ts) ─────────────────────────

    fn token_display(lower: &str) -> Option<&'static str> {
        MODEL_TOKEN_DISPLAY_NAMES
            .iter()
            .find(|(k, _)| *k == lower)
            .map(|(_, v)| *v)
    }

    fn join_model_version_tokens(tokens: Vec<Vec<u16>>) -> Vec<Vec<u16>> {
        let mut merged: Vec<Vec<u16>> = Vec::new();
        for token in tokens {
            let all_digits = !token.is_empty()
                && token
                    .iter()
                    .all(|&c| c == b'0' as u16 || matches!(c, 48..=57));
            let mergeable = all_digits
                && (token == [b'0' as u16] || token.first() != Some(&(b'0' as u16)))
                && token.len() != 8
                && merged
                    .last()
                    .is_some_and(|p| p.last().is_some_and(|&c| matches!(c, 48..=57)));
            if mergeable {
                let mut prev = merged.pop().unwrap_or_default();
                prev.push(b'.' as u16);
                prev.extend_from_slice(&token);
                merged.push(prev);
            } else {
                merged.push(token);
            }
        }
        merged
    }

    fn first_unit_upper(u: u16) -> Vec<u16> {
        match char::from_u32(u as u32) {
            Some(c) => c
                .to_uppercase()
                .collect::<String>()
                .encode_utf16()
                .collect(),
            None => vec![u],
        }
    }

    fn humanize_token(t: &[u16]) -> Vec<u16> {
        let key = lower_string(t);
        if let Some(name) = token_display(&key) {
            return name.encode_utf16().collect();
        }
        let mut out = Vec::with_capacity(t.len());
        if let Some(&first) = t.first() {
            out.extend(first_unit_upper(first));
            out.extend_from_slice(&t[1..]);
        }
        out
    }

    fn restore_separators(u: &[u16]) -> Vec<u16> {
        let is_word = |c: u16| matches!(c, 48..=57 | 65..=90 | 97..=122 | 95);
        let gpt: Vec<u16> = "GPT ".encode_utf16().collect();
        let mut out = Vec::with_capacity(u.len());
        let mut i = 0;
        while i < u.len() {
            if i + 4 < u.len()
                && u[i..i + 4] == gpt[..]
                && matches!(u[i + 4], 48..=57)
                && (i == 0 || !is_word(u[i - 1]))
            {
                out.extend("GPT-".encode_utf16());
                i += 4;
            } else {
                out.push(u[i]);
                i += 1;
            }
        }
        out
    }

    fn split_run(u: &[u16], is_sep: impl Fn(u16) -> bool) -> Vec<Vec<u16>> {
        let mut out = vec![Vec::new()];
        for &c in u {
            if is_sep(c) {
                out.push(Vec::new());
            } else {
                if let Some(t) = out.last_mut() {
                    t.push(c)
                }
            }
        }
        out
    }

    fn humanize_model_slug(slug: &[u16]) -> Vec<u16> {
        if slug.contains(&(b'/' as u16)) {
            return slug.to_vec();
        }
        let tokens = split_run(slug, |c| c == b'-' as u16 || c == b'_' as u16);
        let merged = join_model_version_tokens(tokens);
        let humanized: Vec<Vec<u16>> = merged.iter().map(|t| humanize_token(t)).collect();
        let joined = humanized.join(&(b' ' as u16));
        restore_separators(&joined)
    }

    fn format_model_display_name(model: &[u16]) -> Option<Vec<u16>> {
        let t = trim_units(model);
        if t.is_empty() {
            return None;
        }
        let key = lower_string(t);
        if let Some(name) = model_name_by_slug(&key) {
            return Some(name.encode_utf16().collect());
        }
        Some(humanize_model_slug(t))
    }

    fn format_provider_model_option_name(provider: &str, slug: &[u16]) -> Result<Vec<u16>, String> {
        let trimmed_slug = if provider == "cursor" {
            strip_bracket_suffix_star(trim_units(slug))
        } else {
            trim_units(slug).to_vec()
        };
        if trimmed_slug.is_empty() {
            return Ok(trimmed_slug);
        }
        if provider == "opencode" || provider == "pi" || provider == "omp" {
            let model_id = if trimmed_slug.contains(&(b'/' as u16)) {
                let pos = trimmed_slug
                    .iter()
                    .rposition(|&c| c == b'/' as u16)
                    .unwrap_or(0);
                trimmed_slug[pos + 1..].to_vec()
            } else {
                trimmed_slug
            };
            return Ok(format_model_display_name(&model_id)
                .unwrap_or_else(|| humanize_model_slug(&model_id)));
        }
        Ok(format_model_display_name(&trimmed_slug).unwrap_or(trimmed_slug))
    }

    // ── Cursor variant base id ───────────────────────────────────────────────────

    fn opt_trim(v: &Value) -> Result<Option<Vec<u16>>, String> {
        if is_nullish(v) {
            return Ok(None);
        }
        match str_units(v) {
            Some(u) => Ok(Some(trim_units(&u).to_vec())),
            None => Err(format!(
                "TypeError: {}?.trim is not a function. (In '{}?.trim()', '{}?.trim' is undefined)",
                js_repr(v).trim_end(),
                js_repr(v).trim_end(),
                js_repr(v).trim_end()
            )),
        }
    }

    fn strip_suffix(u: &[u16], suffix: &str) -> Vec<u16> {
        let s: Vec<u16> = suffix.encode_utf16().collect();
        if u.len() >= s.len() && u[u.len() - s.len()..] == s[..] {
            u[..u.len() - s.len()].to_vec()
        } else {
            u.to_vec()
        }
    }

    fn strip_effort_suffix(u: &[u16]) -> Vec<u16> {
        for s in ["-extra-high", "-none", "-low", "-medium", "-high", "-xhigh"] {
            let stripped = strip_suffix(u, s);
            if stripped.len() != u.len() {
                return stripped;
            }
        }
        u.to_vec()
    }

    fn normalize_cursor_variant_base_id(model: &Value) -> Result<Option<Vec<u16>>, String> {
        let Some(trimmed) = opt_trim(model)? else {
            return Ok(None);
        };
        if trimmed.is_empty() {
            return Ok(None);
        }
        let mut base = strip_bracket_suffix_star(&trimmed);
        base = strip_suffix(&base, "-fast");
        base = strip_effort_suffix(&base);
        base = strip_suffix(&base, "-thinking");
        base = strip_suffix(&base, "-fast");
        base = strip_effort_suffix(&base);
        let s = units_lossy(&base);
        if s.ends_with("-max") && !s.contains("codex-max") {
            base = base[..base.len() - 4].to_vec();
        }
        // /^claude-(\d+(?:\.\d+)?)-([a-z]+)-max$/ → claude-$1-$2
        let bs = units_lossy(&base);
        if let Some(new_base) = claude_dotted_max(&bs) {
            base = new_base.encode_utf16().collect();
        }
        base = strip_suffix(&base, "-preview");
        // /^claude-(\d+(?:\.\d+)?)-([a-z]+)$/ → claude-$2-$1(.→-)
        let bs = units_lossy(&base);
        if let Some(reordered) = claude_reordered(&bs) {
            base = reordered.encode_utf16().collect();
        }
        Ok(Some(base))
    }

    fn claude_dotted_max(s: &str) -> Option<String> {
        let rest = s.strip_prefix("claude-")?;
        let (ver, rest) = rest.split_once('-')?;
        let mut parts = ver.split('.');
        let maj = parts.next()?;
        if maj.is_empty() || !maj.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if let Some(min) = parts.next()
            && (min.is_empty()
                || !min.bytes().all(|b| b.is_ascii_digit())
                || parts.next().is_some())
        {
            return None;
        }
        let family = rest.strip_suffix("-max")?;
        if family.is_empty() || !family.bytes().all(|b| b.is_ascii_lowercase()) {
            return None;
        }
        Some(format!("claude-{ver}-{family}"))
    }

    fn claude_reordered(s: &str) -> Option<String> {
        let rest = s.strip_prefix("claude-")?;
        let (ver, family) = rest.split_once('-')?;
        let mut parts = ver.split('.');
        let maj = parts.next()?;
        if maj.is_empty() || !maj.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        if let Some(min) = parts.next()
            && (min.is_empty()
                || !min.bytes().all(|b| b.is_ascii_digit())
                || parts.next().is_some())
        {
            return None;
        }
        if family.is_empty() || !family.bytes().all(|b| b.is_ascii_lowercase()) {
            return None;
        }
        Some(format!("claude-{family}-{}", ver.replace('.', "-")))
    }

    // ── normalizeProviderModelOptions ────────────────────────────────────────────

    fn fields_object(fields: &[(&str, Option<Value>)]) -> Option<Value> {
        let mut o = Map::new();
        for (k, v) in fields {
            if let Some(v) = v {
                o.insert((*k).to_string(), v.clone());
            }
        }
        (!o.is_empty()).then_some(Value::Object(o))
    }

    fn obj_field(v: Option<&Value>, key: &str) -> Option<Value> {
        let f = v.map(|x| js_get(x, key)).unwrap_or_else(js::undefined);
        (truthy(&f) && is_object_type(&f)).then_some(f)
    }

    fn trim_field(v: Option<&Value>, key: &str) -> Option<Vec<u16>> {
        trim_string_or_undefined(&v.map(|x| js_get(x, key)).unwrap_or_else(js::undefined))
    }

    fn npmo(
        value: &Value,
        provider: Option<&Value>,
        legacy: Option<&Value>,
    ) -> Result<Value, String> {
        let candidate = (truthy(value) && is_object_type(value)).then_some(value);
        let codex_c = obj_field(candidate, "codex");
        let claude_c = obj_field(candidate, "claudeAgent");
        let cursor_c = obj_field(candidate, "cursor");
        let devin_c = obj_field(candidate, "devin");
        let ag_c = obj_field(candidate, "antigravity");
        let grok_c = obj_field(candidate, "grok");
        let droid_c = obj_field(candidate, "droid");
        let oc_c = obj_field(candidate, "opencode");
        let pi_c = obj_field(candidate, "pi");
        let omp_c = obj_field(candidate, "omp");

        let lget =
            |k: &str| -> Value { legacy.map(|l| js_get(l, k)).unwrap_or_else(js::undefined) };
        let provider_is = |k: &str| provider.is_some_and(|p| str_eq(p, k));
        let sget = |c: &Option<Value>, k: &str| -> Value {
            c.as_ref()
                .map(|v| js_get(v, k))
                .unwrap_or_else(js::undefined)
        };

        // codex
        let codex_effort = trim_field(codex_c.as_ref(), "reasoningEffort").or_else(|| {
            if provider_is("codex") {
                trim_string_or_undefined(&lget("effort"))
            } else {
                None
            }
        });
        let fm = sget(&codex_c, "fastMode");
        let codex_fast = if strict_eq(&fm, &Value::Bool(true)) {
            Some(true)
        } else if strict_eq(&fm, &Value::Bool(false)) {
            Some(false)
        } else if (provider_is("codex") && strict_eq(&lget("codexFastMode"), &Value::Bool(true)))
            || str_eq(&lget("serviceTier"), "fast")
        {
            Some(true)
        } else {
            None
        };
        let tier = sget(&codex_c, "serviceTier");
        let codex_tier = str_eq(&tier, "ultrafast").then_some("ultrafast");
        let codex = fields_object(&[
            (
                "reasoningEffort",
                codex_effort.map(|u| js::utf16_string(&u)),
            ),
            ("fastMode", codex_fast.map(Value::Bool)),
            ("serviceTier", codex_tier.map(|t| Value::String(t.into()))),
        ]);

        // claudeAgent
        let claude_thinking = match sget(&claude_c, "thinking") {
            Value::Bool(b) => Some(b),
            _ => None,
        };
        let claude_effort_raw = sget(&claude_c, "effort");
        let claude_effort = CLAUDE_EFFORT_SET
            .iter()
            .any(|e| str_eq(&claude_effort_raw, e))
            .then_some(claude_effort_raw);
        let claude_fast = match sget(&claude_c, "fastMode") {
            Value::Bool(b) => Some(b),
            _ => None,
        };
        let claude_window = trim_field(claude_c.as_ref(), "autoCompactWindow")
            .or_else(|| trim_field(claude_c.as_ref(), "contextWindow"));
        let claude = fields_object(&[
            ("thinking", claude_thinking.map(Value::Bool)),
            ("effort", claude_effort),
            ("fastMode", claude_fast.map(Value::Bool)),
            (
                "autoCompactWindow",
                claude_window.map(|u| js::utf16_string(&u)),
            ),
        ]);

        // cursor
        let cursor = fields_object(&[
            (
                "reasoningEffort",
                trim_field(cursor_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
            ),
            (
                "fastMode",
                match sget(&cursor_c, "fastMode") {
                    Value::Bool(b) => Some(Value::Bool(b)),
                    _ => None,
                },
            ),
            (
                "thinking",
                match sget(&cursor_c, "thinking") {
                    Value::Bool(b) => Some(Value::Bool(b)),
                    _ => None,
                },
            ),
            (
                "contextWindow",
                trim_field(cursor_c.as_ref(), "contextWindow").map(|u| js::utf16_string(&u)),
            ),
        ]);

        // antigravity
        let antigravity = fields_object(&[(
            "reasoningEffort",
            trim_field(ag_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
        )]);

        // grok
        let grok_effort_raw = sget(&grok_c, "reasoningEffort");
        let grok_effort = GROK_EFFORT_SET
            .iter()
            .any(|e| str_eq(&grok_effort_raw, e))
            .then_some(grok_effort_raw);
        let grok = fields_object(&[("reasoningEffort", grok_effort)]);

        // droid
        let droid = fields_object(&[(
            "reasoningEffort",
            trim_field(droid_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
        )]);

        // opencode
        let opencode = fields_object(&[
            (
                "variant",
                trim_field(oc_c.as_ref(), "variant").map(|u| js::utf16_string(&u)),
            ),
            (
                "agent",
                trim_field(oc_c.as_ref(), "agent").map(|u| js::utf16_string(&u)),
            ),
        ]);

        // pi
        let pi_raw = sget(&pi_c, "thinkingLevel");
        let pi_level = PI_THINKING_SET
            .iter()
            .any(|e| str_eq(&pi_raw, e))
            .then_some(pi_raw);
        let pi = fields_object(&[("thinkingLevel", pi_level)]);

        // devin
        let devin = fields_object(&[
            (
                "reasoningEffort",
                trim_field(devin_c.as_ref(), "reasoningEffort").map(|u| js::utf16_string(&u)),
            ),
            (
                "fastMode",
                match sget(&devin_c, "fastMode") {
                    Value::Bool(b) => Some(Value::Bool(b)),
                    _ => None,
                },
            ),
            (
                "thinking",
                match sget(&devin_c, "thinking") {
                    Value::Bool(b) => Some(Value::Bool(b)),
                    _ => None,
                },
            ),
            (
                "contextWindow",
                trim_field(devin_c.as_ref(), "contextWindow").map(|u| js::utf16_string(&u)),
            ),
            (
                "modelVariant",
                trim_field(devin_c.as_ref(), "modelVariant").map(|u| js::utf16_string(&u)),
            ),
        ]);

        // omp: normalizeOmpModelOptions(ompCandidate)
        let omp_level_raw = sget(&omp_c, "thinkingLevel");
        let omp = {
            let level = trim_string_or_undefined(&omp_level_raw)
                .filter(|u| OMP_THINKING_SET.iter().any(|e| units_eq_str(u, e)));
            fields_object(&[("thinkingLevel", level.map(|u| js::utf16_string(&u)))])
        };

        let mut out = Map::new();
        for (k, v) in [
            ("codex", codex),
            ("claudeAgent", claude),
            ("cursor", cursor),
            ("devin", devin),
            ("antigravity", antigravity),
            ("grok", grok),
            ("droid", droid),
            ("opencode", opencode),
            ("pi", pi),
            ("omp", omp),
        ] {
            if let Some(v) = v {
                out.insert(k.to_string(), v);
            }
        }
        Ok(if out.is_empty() {
            Value::Null
        } else {
            Value::Object(out)
        })
    }

    // ── Selection helpers ────────────────────────────────────────────────────────

    fn pisk(provider: &Value, instance_id: Option<&Value>) -> Value {
        match instance_id.and_then(normalize_instance_id) {
            Some(s) => Value::String(s),
            None => provider.clone(),
        }
    }

    fn make_selection_value(
        provider: &Value,
        model: &Value,
        options: &Value,
        supports_auto_mode: &Value,
        instance_id: &Value,
    ) -> Result<Value, String> {
        let Some(kind) = PROVIDER_KINDS.iter().find(|k| str_eq(provider, k)) else {
            return Ok(js::undefined());
        };
        let instance = normalize_instance_id(instance_id);
        let mut o = Map::new();
        o.insert("provider".into(), provider.clone());
        o.insert("model".into(), model.clone());
        if let Some(i) = instance {
            o.insert("instanceId".into(), Value::String(i));
        }
        if truthy(options) {
            o.insert("options".into(), options.clone());
        }
        if *kind == "claudeAgent"
            && let Value::Bool(_) = supports_auto_mode
        {
            o.insert("supportsAutoMode".into(), supports_auto_mode.clone());
        }
        Ok(Value::Object(o))
    }

    fn matches_provider_instance(selection: &Value, provider: &Value, instance_id: &Value) -> bool {
        if !truthy(selection) {
            return false;
        }
        if !strict_eq(&js_get(selection, "provider"), provider) {
            return false;
        }
        let sel_iid = js_get(selection, "instanceId");
        let left = if is_nullish(&sel_iid) {
            js_get(selection, "provider")
        } else {
            sel_iid
        };
        let right = if is_nullish(instance_id) {
            provider.clone()
        } else {
            instance_id.clone()
        };
        strict_eq(&left, &right)
    }

    fn merge_opts_from_selections(sels: &[Value]) -> Result<Value, String> {
        let mut result = Map::new();
        for sel in sels {
            if !truthy(sel) {
                continue;
            }
            let key = prop_key(&js_get(sel, "provider"))?;
            let opts = js_get(sel, "options");
            if truthy(&opts) {
                result.insert(key, opts);
            } else {
                result.remove(&key);
            }
        }
        Ok(if result.is_empty() {
            Value::Null
        } else {
            Value::Object(finalize_map(result))
        })
    }

    fn derive_effective_model_options(input: &Value) -> Result<Value, String> {
        let sp = js_get(input, "selectedProvider");
        let spi = js_get(input, "selectedProviderInstanceId");
        let sel_for_target = |sel: &Value| -> Value {
            let p = if is_nullish(sel) {
                js::undefined()
            } else {
                js_get(sel, "provider")
            };
            if !strict_eq(&p, &sp) || matches_provider_instance(sel, &sp, &spi) {
                sel.clone()
            } else {
                Value::Null
            }
        };
        let base = merge_opts_from_selections(&[
            sel_for_target(&js_get(input, "projectModelSelection")),
            sel_for_target(&js_get(input, "threadModelSelection")),
        ])?;
        let draft = js_get(input, "draft");
        let draft_map = if is_nullish(&draft) {
            js::undefined()
        } else {
            js_get(&draft, "modelSelectionByProvider")
        };
        if !truthy(&draft_map) {
            return Ok(base);
        }
        let mut result = if truthy(&base) {
            spread_to_map(&base)
        } else {
            Map::new()
        };
        for sel in object_values(&draft_map)? {
            if !truthy(&sel) {
                continue;
            }
            let provider = js_get(&sel, "provider");
            if strict_eq(&provider, &sp) && !matches_provider_instance(&sel, &sp, &spi) {
                continue;
            }
            let key = prop_key(&provider)?;
            let opts = js_get(&sel, "options");
            if truthy(&opts) {
                result.insert(key, opts);
            } else {
                result.remove(&key);
            }
        }
        Ok(if result.is_empty() {
            Value::Null
        } else {
            Value::Object(finalize_map(result))
        })
    }

    // ── App model options / selectable model ─────────────────────────────────────

    fn find_not_fn(options: &Value, call_src: &str) -> String {
        let r = js_repr(options).trim_end().to_string();
        format!(
            "TypeError: {r}.find is not a function. (In '{r}.find({call_src})', '{r}.find' is undefined)"
        )
    }

    fn resolve_selectable_model(
        provider: &Value,
        value: &Value,
        options: &[Value],
    ) -> Result<Option<Value>, String> {
        let Some(u) = str_units(value) else {
            return Ok(None);
        };
        let trimmed = trim_units(&u);
        if trimmed.is_empty() {
            return Ok(None);
        }
        for o in options {
            if strict_eq(&js_get(o, "slug"), &js::utf16_string(trimmed)) {
                return Ok(Some(js_get(o, "slug")));
            }
        }
        let t_lower = lower_string(trimmed);
        for o in options {
            let name = js_get(o, "name");
            match str_units(&name) {
                Some(nu) => {
                    if lower_string(&nu) == t_lower {
                        return Ok(Some(js_get(o, "slug")));
                    }
                }
                None => {
                    let r = js_repr(o).trim_end().to_string();
                    if is_nullish(&name) {
                        return Err(format!(
                            "TypeError: {} is not an object (evaluating '{r}.name.toLowerCase')",
                            nullish_word(&name)
                        ));
                    }
                    return Err(format!(
                        "TypeError: {r}.name.toLowerCase is not a function. (In '{r}.name.toLowerCase()', '{r}.name.toLowerCase' is undefined)"
                    ));
                }
            }
        }
        let provider_key = prop_key(provider)?;
        let Some(normalized) = normalize_model_slug(Some(trimmed.to_vec()), &provider_key)? else {
            return Ok(None);
        };
        let nv = js::utf16_string(&normalized);
        for o in options {
            if strict_eq(&js_get(o, "slug"), &nv) {
                return Ok(Some(js_get(o, "slug")));
            }
        }
        if (provider_key == "omp" || provider_key == "pi" || provider_key == "opencode")
            && !normalized.contains(&(b'/' as u16))
        {
            let mut scoped: Vec<&Value> = Vec::new();
            for o in options {
                let slug = js_get(o, "slug");
                match str_units(&slug) {
                    Some(s) => {
                        if s.len() > normalized.len()
                            && s[s.len() - normalized.len() - 1] == b'/' as u16
                            && s.ends_with(&normalized[..])
                        {
                            scoped.push(o);
                        }
                    }
                    None => {
                        let r = js_repr(o).trim_end().to_string();
                        if is_nullish(&slug) {
                            return Err(format!(
                                "TypeError: {} is not an object (evaluating '{r}.slug.endsWith')",
                                nullish_word(&slug)
                            ));
                        }
                        return Err(format!(
                            "TypeError: {r}.slug.endsWith is not a function. (In '{r}.slug.endsWith(\"/{}\")', '{r}.slug.endsWith' is undefined)",
                            units_lossy(&normalized)
                        ));
                    }
                }
            }
            if scoped.len() == 1 {
                return Ok(Some(js_get(scoped[0], "slug")));
            }
        }
        Ok(None)
    }

    fn iterate(models: &Value) -> Result<Vec<Value>, String> {
        if let Value::Array(a) = models {
            return Ok(a.clone());
        }
        if let Some(u) = str_units(models) {
            return Ok(units_lossy(&u)
                .chars()
                .map(|c| Value::String(c.to_string()))
                .collect());
        }
        if let Value::Object(o) = models
            && o.len() == 1
        {
            if let Some(m) = o.get("$map").and_then(|m| m.as_array()) {
                return Ok(m
                    .iter()
                    .map(|kv| {
                        Value::Array(vec![
                            kv.get(0).cloned().unwrap_or(Value::Null),
                            kv.get(1).cloned().unwrap_or(Value::Null),
                        ])
                    })
                    .collect());
            }
            if let Some(s) = o.get("$set").and_then(|s| s.as_array()) {
                return Ok(s.clone());
            }
            if let Some(b) = o
                .get("$bytes")
                .or_else(|| o.get("$buffer"))
                .and_then(|b| b.as_array())
            {
                return Ok(b.clone());
            }
        }
        if is_nullish(models) {
            return Err(format!(
                "TypeError: {} is not an object (evaluating 'candidate of {}')",
                nullish_word(models),
                if models.is_null() { "null" } else { "void 0" }
            ));
        }
        let desc = match models {
            Value::Number(_) => "number".into(),
            Value::Bool(b) => {
                if *b {
                    "true".into()
                } else {
                    "false".into()
                }
            }
            Value::Object(_) => "{}".into(),
            _ => js_repr(models).trim_end().to_string(),
        };
        Err(format!("TypeError: {desc} is not iterable"))
    }

    fn normalize_custom_model_slugs(models: &Value, provider: &str) -> Result<Vec<String>, String> {
        let Some(built_in) = model_options(provider).map(|opts| {
            opts.iter()
                .map(|(s, _)| *s)
                .collect::<std::collections::HashSet<_>>()
        }) else {
            return Err("TypeError: undefined is not an object (evaluating '(void 0).has')".into());
        };
        let mut out: Vec<String> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for candidate in iterate(models)? {
            let Some(u) = str_units(&candidate) else {
                continue;
            };
            let Some(norm) = normalize_model_slug(Some(u), provider)? else {
                continue;
            };
            let nstr = units_lossy(&norm);
            if norm.len() > MAX_CUSTOM_MODEL_LENGTH
                || built_in.contains(&nstr.as_str())
                || !seen.insert(nstr.clone())
            {
                continue;
            }
            out.push(nstr);
            if out.len() >= MAX_CUSTOM_MODEL_COUNT {
                break;
            }
        }
        Ok(out)
    }

    fn get_app_model_options(
        provider: &str,
        custom_models: &Value,
        selected_model: &Value,
    ) -> Result<Vec<Value>, String> {
        let Some(builtins) = model_options(provider) else {
            return Err(
            "TypeError: undefined is not an object (evaluating 'getModelOptions(provider).map')"
                .into(),
        );
        };
        let mut options: Vec<Value> = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (slug, name) in builtins {
            seen.insert((*slug).to_string());
            let mut o = Map::new();
            o.insert("provider".into(), Value::String(provider.into()));
            o.insert("slug".into(), Value::String((*slug).into()));
            o.insert("name".into(), Value::String((*name).into()));
            o.insert("isCustom".into(), Value::Bool(false));
            options.push(Value::Object(o));
        }
        let trimmed_selected = match opt_trim(selected_model)? {
            Some(u) if !u.is_empty() => Some(lower_string(&u)),
            _ => None,
        };
        for slug in normalize_custom_model_slugs(custom_models, provider)? {
            if seen.contains(&slug) {
                continue;
            }
            seen.insert(slug.clone());
            let slug_u: Vec<u16> = slug.encode_utf16().collect();
            let mut o = Map::new();
            o.insert("provider".into(), Value::String(provider.into()));
            o.insert("slug".into(), Value::String(slug.clone()));
            o.insert(
                "name".into(),
                js::utf16_string(&format_provider_model_option_name(provider, &slug_u)?),
            );
            o.insert("isCustom".into(), Value::Bool(true));
            options.push(Value::Object(o));
        }
        let normalized_selected = if provider == "cursor" {
            normalize_cursor_variant_base_id(selected_model)?
        } else {
            normalize_model_slug(str_units(selected_model), provider)?
        };
        let matches_name = trimmed_selected.as_ref().is_some_and(|t| {
            options.iter().any(|o| {
                str_units(&js_get(o, "name"))
                    .map(|n| lower_string(&n) == *t)
                    .unwrap_or(false)
            })
        });
        if let Some(ns) = normalized_selected {
            let nstr = units_lossy(&ns);
            if !seen.contains(&nstr) && !matches_name {
                let mut o = Map::new();
                o.insert("provider".into(), Value::String(provider.into()));
                o.insert("slug".into(), js::utf16_string(&ns));
                o.insert(
                    "name".into(),
                    js::utf16_string(&format_provider_model_option_name(provider, &ns)?),
                );
                o.insert("isCustom".into(), Value::Bool(true));
                options.push(Value::Object(o));
            }
        }
        Ok(options)
    }

    fn resolve_app_model_selection(
        provider: &Value,
        custom_models: &Value,
        selected_model: &Value,
    ) -> Result<Value, String> {
        let provider_key = prop_key(provider)?;
        let custom_for = js_get(custom_models, &provider_key);
        let options = get_app_model_options(&provider_key, &custom_for, selected_model)?;
        if let Some(v) = resolve_selectable_model(provider, selected_model, &options)? {
            return Ok(v);
        }
        let d = default_model_for(provider);
        Ok(if is_nullish(&d) {
            Value::String(String::new())
        } else {
            d
        })
    }

    // ── Public ports ─────────────────────────────────────────────────────────────

    /// Port of `normalizeProviderKind` (`apps/web/src/composerDraftModels.ts:154`).
    pub fn normalize_provider_kind(
        value: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        if str_eq(&value, "gemini") {
            return Ok(Some(Value::String("antigravity".into())));
        }
        if str_eq(&value, "kilo") {
            return Ok(Some(Value::String("opencode".into())));
        }
        if PROVIDER_KINDS.iter().any(|k| str_eq(&value, k)) {
            return Ok(Some(value));
        }
        Ok(None)
    }

    /// Port of `normalizeProviderInstanceId` (`apps/web/src/composerDraftModels.ts:172`).
    pub fn normalize_provider_instance_id(
        value: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        Ok(match normalize_instance_id(&value) {
            Some(s) => Value::String(s),
            None => js::undefined(),
        })
    }

    /// Port of `providerInstanceModelSelectionKey` (`apps/web/src/composerDraftModels.ts:177`).
    pub fn provider_instance_model_selection_key(
        provider: serde_json::Value,
        instance_id: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        Ok(pisk(&provider, instance_id.as_ref()))
    }

    /// Port of `modelSelectionStorageKey` (`apps/web/src/composerDraftModels.ts:184`).
    pub fn model_selection_storage_key(
        selection: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if is_nullish(&selection) {
            return Err(format!(
                "TypeError: {} is not an object (evaluating 'selection.provider')",
                nullish_word(&selection)
            ));
        }
        let provider = js_get(&selection, "provider");
        let instance_id = js_get(&selection, "instanceId");
        Ok(pisk(&provider, Some(&instance_id)))
    }

    /// Port of `readModelSelectionForProviderInstance` (`apps/web/src/composerDraftModels.ts:188`).
    pub fn read_model_selection_for_provider_instance(
        selections: Option<serde_json::Value>,
        provider: serde_json::Value,
        instance_id: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let key = pisk(&provider, instance_id.as_ref());
        let Some(selections) = selections else {
            return Ok(js::undefined());
        };
        if is_nullish(&selections) {
            return Ok(js::undefined());
        }
        Ok(js_get(&selections, &prop_key(&key)?))
    }

    /// Port of `normalizeModelSelectionMapByInstance` (`apps/web/src/composerDraftModels.ts:196`).
    pub fn normalize_model_selection_map_by_instance(
        selections: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let mut result = Map::new();
        for sel in object_values(&selections)? {
            if truthy(&sel) {
                let provider = js_get(&sel, "provider");
                let instance_id = js_get(&sel, "instanceId");
                let key = prop_key(&pisk(&provider, Some(&instance_id)))?;
                result.insert(key, sel);
            }
        }
        Ok(Value::Object(finalize_map(result)))
    }

    /// Port of `makeModelSelection` (`apps/web/src/composerDraftModels.ts:216`).
    pub fn make_model_selection(
        provider: serde_json::Value,
        model: serde_json::Value,
        options: Option<serde_json::Value>,
        supports_auto_mode: serde_json::Value,
        instance_id: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        make_selection_value(
            &provider,
            &model,
            &options.unwrap_or_else(js::undefined),
            &supports_auto_mode,
            &instance_id.unwrap_or_else(js::undefined),
        )
    }

    /// Port of `normalizeProviderModelOptions` (`apps/web/src/composerDraftModels.ts:323`).
    pub fn normalize_provider_model_options(
        value: serde_json::Value,
        provider: Option<serde_json::Value>,
        legacy: Option<serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, String> {
        let out = npmo(&value, provider.as_ref(), legacy.as_ref())?;
        Ok(if out.is_null() { None } else { Some(out) })
    }

    /// Port of `normalizeModelSelection` (`apps/web/src/composerDraftModels.ts:529`).
    pub fn normalize_model_selection(
        value: serde_json::Value,
        legacy: Option<serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, String> {
        let candidate = (truthy(&value) && is_object_type(&value)).then_some(&value);
        let cand = |k: &str| -> Value {
            candidate
                .map(|c| js_get(c, k))
                .unwrap_or_else(js::undefined)
        };
        let leg = |k: &str| -> Value {
            legacy
                .as_ref()
                .map(|l| js_get(l, k))
                .unwrap_or_else(js::undefined)
        };
        let coalesce = |a: Value, b: Value| if is_nullish(&a) { b } else { a };
        let raw_provider = coalesce(cand("provider"), leg("provider"));
        let migrated_gemini = str_eq(&raw_provider, "gemini");
        let provider = if migrated_gemini {
            Some("antigravity")
        } else if str_eq(&raw_provider, "kilo") {
            Some("opencode")
        } else {
            PROVIDER_KINDS
                .iter()
                .find(|k| str_eq(&raw_provider, k))
                .copied()
        };
        let Some(provider) = provider else {
            return Ok(None);
        };
        let raw_model = coalesce(cand("model"), leg("model"));
        let Some(raw_model_units) = str_units(&raw_model) else {
            return Ok(None);
        };
        let instance_id = cand("instanceId");

        let raw_trimmed = trim_units(&raw_model_units).to_vec();
        let legacy_match = if provider == "antigravity" {
            antigravity_legacy_match(&raw_trimmed)
        } else {
            None
        };
        let legacy_effort = legacy_match
            .as_ref()
            .map(|(_, inner)| lower_string(trim_units(inner)));
        let has_legacy = legacy_match.is_some()
            && legacy_effort
                .as_ref()
                .is_some_and(|e| ANTIGRAVITY_EFFORT_SET.contains(&e.as_str()));
        let normalized_raw = if migrated_gemini {
            "Gemini 3.5 Flash".encode_utf16().collect::<Vec<u16>>()
        } else if has_legacy {
            trim_units(&legacy_match.clone().map(|(g, _)| g).unwrap_or_default()).to_vec()
        } else {
            raw_model_units.clone()
        };
        let Some(model) = normalize_model_slug(Some(normalized_raw), provider)? else {
            return Ok(None);
        };

        let model_options = if migrated_gemini {
            Value::Null
        } else {
            let opts_source = {
                let cand_opts = cand("options");
                if truthy(&cand_opts) {
                    let mut m = Map::new();
                    m.insert(provider.to_string(), cand_opts);
                    Value::Object(m)
                } else {
                    leg("modelOptions")
                }
            };
            let legacy_codex = if provider == "codex" {
                leg("legacyCodex")
            } else {
                js::undefined()
            };
            npmo(
                &opts_source,
                Some(&Value::String(provider.into())),
                if is_undef(&legacy_codex) {
                    None
                } else {
                    Some(&legacy_codex)
                },
            )?
        };
        let options = match provider {
            "codex" => js_get(&model_options, "codex"),
            "claudeAgent" => js_get(&model_options, "claudeAgent"),
            "antigravity" => js_get(&model_options, "antigravity"),
            "grok" => normalize_grok_model_options(
                &js::utf16_string(&model),
                &js_get(&model_options, "grok"),
            )?,
            "droid" => js_get(&model_options, "droid"),
            "cursor" => js_get(&model_options, "cursor"),
            "opencode" => js_get(&model_options, "opencode"),
            "pi" => js_get(&model_options, "pi"),
            "devin" => js_get(&model_options, "devin"),
            _ => js_get(&model_options, "omp"),
        };
        let normalized_options = if provider == "antigravity" && has_legacy {
            let e = js_get(&js_get(&model_options, "antigravity"), "reasoningEffort");
            let e = if is_nullish(&e) {
                Value::String(legacy_effort.unwrap_or_default())
            } else {
                e
            };
            let mut m = Map::new();
            m.insert("reasoningEffort".into(), e);
            Value::Object(m)
        } else {
            options
        };
        let auto = if provider == "claudeAgent" {
            let s = cand("supportsAutoMode");
            if matches!(s, Value::Bool(_)) {
                s
            } else {
                js::undefined()
            }
        } else {
            js::undefined()
        };
        let sel = make_selection_value(
            &Value::String(provider.into()),
            &js::utf16_string(&model),
            &normalized_options,
            &auto,
            &instance_id,
        )?;
        Ok(if is_undef(&sel) { None } else { Some(sel) })
    }

    /// Port of `reconcileProviderScopedModelSelection` (`apps/web/src/composerDraftModels.ts:612`).
    pub fn reconcile_provider_scoped_model_selection(
        requested: serde_json::Value,
        current: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        if is_nullish(&requested) {
            return Err(format!(
                "TypeError: {} is not an object (evaluating 'requested.options')",
                nullish_word(&requested)
            ));
        }
        let req_options = js_get(&requested, "options");
        if !is_undef(&req_options) {
            return Ok(requested);
        }
        let cur = current.unwrap_or(Value::Null);
        let req_provider = js_get(&requested, "provider");
        let req_instance = js_get(&requested, "instanceId");
        if !matches_provider_instance(&cur, &req_provider, &req_instance) {
            return Ok(requested);
        }
        let cur_model = js_get(&cur, "model");
        let req_model = js_get(&requested, "model");
        let cur_provider = js_get(&cur, "provider");
        if strict_eq(&cur_model, &req_model) {
            let cur_auto = if str_eq(&cur_provider, "claudeAgent") {
                js_get(&cur, "supportsAutoMode")
            } else {
                js::undefined()
            };
            let auto = if str_eq(&req_provider, "claudeAgent") {
                let ra = js_get(&requested, "supportsAutoMode");
                if is_nullish(&ra) { cur_auto } else { ra }
            } else {
                js::undefined()
            };
            return make_selection_value(
                &req_provider,
                &req_model,
                &js_get(&cur, "options"),
                &auto,
                &req_instance,
            );
        }
        if !str_eq(&cur_provider, "codex")
            && !str_eq(&cur_provider, "cursor")
            && !str_eq(&cur_provider, "claudeAgent")
        {
            return Ok(requested);
        }
        let cur_options = js_get(&cur, "options");
        let mut preserved = cur_options.clone();
        let effort = if str_eq(&cur_provider, "claudeAgent") {
            if is_nullish(&cur_options) {
                js::undefined()
            } else {
                js_get(&cur_options, "effort")
            }
        } else if is_nullish(&cur_options) {
            js::undefined()
        } else {
            js_get(&cur_options, "reasoningEffort")
        };
        if !is_undef(&effort)
            && classify_provider_effort(&req_provider, &req_model, &effort)? != "supported"
        {
            let drop_key = if str_eq(&cur_provider, "claudeAgent") {
                "effort"
            } else {
                "reasoningEffort"
            };
            let mut rest = Map::new();
            if !is_nullish(&cur_options) {
                for (k, v) in spread_props(&cur_options) {
                    if k != drop_key {
                        rest.insert(k, v);
                    }
                }
            }
            preserved = if rest.is_empty() {
                js::undefined()
            } else {
                Value::Object(finalize_map(rest))
            };
        }
        let auto = if str_eq(&req_provider, "claudeAgent") {
            js_get(&requested, "supportsAutoMode")
        } else {
            js::undefined()
        };
        make_selection_value(&req_provider, &req_model, &preserved, &auto, &req_instance)
    }

    /// Port of `stripNonStickyModelOptions` (`apps/web/src/composerDraftModels.ts:674`).
    pub fn strip_non_sticky_model_options(
        selection: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if is_nullish(&selection) {
            return Err(format!(
                "TypeError: {} is not an object (evaluating 'selection.provider')",
                nullish_word(&selection)
            ));
        }
        let provider = js_get(&selection, "provider");
        let options = js_get(&selection, "options");
        let cw = if is_nullish(&options) {
            js::undefined()
        } else {
            js_get(&options, "contextWindow")
        };
        let acw = if is_nullish(&options) {
            js::undefined()
        } else {
            js_get(&options, "autoCompactWindow")
        };
        if !str_eq(&provider, "claudeAgent") || (!truthy(&cw) && !truthy(&acw)) {
            return Ok(selection);
        }
        let mut rest = Map::new();
        for (k, v) in spread_props(&options) {
            if k != "contextWindow" && k != "autoCompactWindow" {
                rest.insert(k, v);
            }
        }
        let opts = if rest.is_empty() {
            js::undefined()
        } else {
            Value::Object(finalize_map(rest))
        };
        make_selection_value(
            &provider,
            &js_get(&selection, "model"),
            &opts,
            &js_get(&selection, "supportsAutoMode"),
            &js_get(&selection, "instanceId"),
        )
    }

    /// Port of `sanitizeStickyModelSelectionMap` (`apps/web/src/composerDraftModels.ts:695`).
    pub fn sanitize_sticky_model_selection_map(
        map: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let entries = object_entries(&map)?;
        let mut next: Option<Map<String, Value>> = None;
        for (key, sel) in entries {
            let sp = if is_nullish(&sel) {
                js::undefined()
            } else {
                js_get(&sel, "provider")
            };
            if !str_eq(&sp, "claudeAgent") {
                continue;
            }
            let so = js_get(&sel, "options");
            let has = !is_nullish(&so)
                && (truthy(&js_get(&so, "contextWindow"))
                    || truthy(&js_get(&so, "autoCompactWindow")));
            if has {
                let m = next.get_or_insert_with(|| spread_to_map(&map));
                m.insert(key, strip_non_sticky_model_options(sel)?);
            }
        }
        Ok(match next {
            Some(m) => Value::Object(finalize_map(m)),
            None => map,
        })
    }

    /// Port of `legacySyncModelSelectionOptions` (`apps/web/src/composerDraftModels.ts:711`).
    pub fn legacy_sync_model_selection_options(
        model_selection: serde_json::Value,
        model_options: Option<serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, String> {
        if model_selection.is_null() {
            return Ok(None);
        }
        if is_undef(&model_selection) {
            return Err(
                "TypeError: undefined is not an object (evaluating 'modelSelection.provider')"
                    .into(),
            );
        }
        let provider = js_get(&model_selection, "provider");
        let normalized_options = if str_eq(&provider, "grok") {
            let g = model_options
                .as_ref()
                .map(|m| js_get(m, "grok"))
                .unwrap_or_else(js::undefined);
            normalize_grok_model_options(&js_get(&model_selection, "model"), &g)?
        } else {
            let key = prop_key(&provider)?;
            model_options
                .as_ref()
                .map(|m| js_get(m, &key))
                .unwrap_or_else(js::undefined)
        };
        let auto = if str_eq(&provider, "claudeAgent") {
            js_get(&model_selection, "supportsAutoMode")
        } else {
            js::undefined()
        };
        let sel = make_selection_value(
            &provider,
            &js_get(&model_selection, "model"),
            &normalized_options,
            &auto,
            &js_get(&model_selection, "instanceId"),
        )?;
        Ok(Some(sel))
    }

    /// Port of `legacyMergeModelSelectionIntoProviderModelOptions` (`apps/web/src/composerDraftModels.ts:731`).
    pub fn legacy_merge_model_selection_into_provider_model_options(
        model_selection: Option<serde_json::Value>,
        current_model_options: Option<serde_json::Value>,
    ) -> Result<Option<serde_json::Value>, String> {
        let cmo = current_model_options.unwrap_or_else(js::undefined);
        let ms_opts = model_selection
            .as_ref()
            .map(|m| js_get(m, "options"))
            .unwrap_or_else(js::undefined);
        if is_undef(&ms_opts) {
            let out = npmo(&cmo, None, None)?;
            return Ok(if out.is_null() { None } else { Some(out) });
        }
        let provider = js_get(model_selection.as_ref().unwrap_or(&Value::Null), "provider");
        let current = npmo(&cmo, None, None)?;
        let pkey = prop_key(&provider)?;
        let mut others = Map::new();
        if !is_nullish(&current) {
            for (k, v) in spread_props(&current) {
                if k != pkey {
                    others.insert(k, v);
                }
            }
        }
        let mut single = Map::new();
        single.insert(pkey, ms_opts);
        let normalized_next = npmo(&Value::Object(single), Some(&provider), None)?;
        let mut merged = others;
        if !is_nullish(&normalized_next) {
            for (k, v) in spread_props(&normalized_next) {
                merged.insert(k, v);
            }
        }
        let out = npmo(&Value::Object(finalize_map(merged)), None, None)?;
        Ok(if out.is_null() { None } else { Some(out) })
    }

    /// Port of `legacyToModelSelectionByProvider` (`apps/web/src/composerDraftModels.ts:763`).
    pub fn legacy_to_model_selection_by_provider(
        model_selection: Option<serde_json::Value>,
        model_options: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let mut result = Map::new();
        if let Some(mo) = model_options.as_ref().filter(|m| truthy(m)) {
            for provider in COMPOSER_PROVIDER_KINDS {
                let options = js_get(mo, provider);
                if truthy(&options) && !object_keys(&options)?.is_empty() {
                    let msp = model_selection
                        .as_ref()
                        .map(|m| js_get(m, "provider"))
                        .unwrap_or_else(js::undefined);
                    let model = if strict_eq(&msp, &Value::String((*provider).into())) {
                        model_selection
                            .as_ref()
                            .map(|m| js_get(m, "model"))
                            .unwrap_or_else(js::undefined)
                    } else {
                        default_model_for(&Value::String((*provider).into()))
                    };
                    if truthy(&model) {
                        let sel_opts = if *provider == "grok" {
                            normalize_grok_model_options(&model, &js_get(mo, "grok"))?
                        } else {
                            options.clone()
                        };
                        let sel = make_selection_value(
                            &Value::String((*provider).into()),
                            &model,
                            &sel_opts,
                            &js::undefined(),
                            &js::undefined(),
                        )?;
                        result.insert((*provider).to_string(), sel);
                    }
                }
            }
        }
        if let Some(ms) = &model_selection
            && truthy(ms)
        {
            let provider = js_get(ms, "provider");
            let instance_id = js_get(ms, "instanceId");
            let key = prop_key(&pisk(&provider, Some(&instance_id)))?;
            result.insert(key, ms.clone());
        }
        Ok(Value::Object(finalize_map(result)))
    }

    /// Port of `deriveEffectiveComposerModelState` (`apps/web/src/composerDraftModels.ts:792`).
    pub fn derive_effective_composer_model_state(
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if is_nullish(&input) {
            return Err(format!(
                "TypeError: {} is not an object (evaluating 'input.selectedProvider')",
                nullish_word(&input)
            ));
        }
        let sp = js_get(&input, "selectedProvider");
        let spi = js_get(&input, "selectedProviderInstanceId");
        let thread_sel = js_get(&input, "threadModelSelection");
        let project_sel = js_get(&input, "projectModelSelection");
        let custom_models = js_get(&input, "customModelsByProvider");
        let ambp = js_get(&input, "availableModelOptionsByProvider");
        let draft = js_get(&input, "draft");
        let draft_map = if is_nullish(&draft) {
            js::undefined()
        } else {
            js_get(&draft, "modelSelectionByProvider")
        };

        let matches_sel = |sel: &Value| matches_provider_instance(sel, &sp, &spi);

        let avail = if is_nullish(&ambp) {
            js::undefined()
        } else {
            js_get(&ambp, &prop_key(&sp)?)
        };
        let resolve_available = |candidate: &Value| -> Result<Option<Value>, String> {
            if !truthy(&avail) {
                return Ok(None);
            }
            let len = js_get(&avail, "length");
            if strict_eq(&len, &Value::from(0i64)) {
                return Ok(None);
            }
            let Some(opts) = avail.as_array() else {
                return Err(find_not_fn(&avail, "(option) => option.slug === trimmed"));
            };
            resolve_selectable_model(&sp, candidate, opts)
        };

        let base_chain = {
            let m = if matches_sel(&thread_sel) {
                js_get(&thread_sel, "model")
            } else {
                Value::Null
            };
            let m = if is_nullish(&m) {
                if matches_sel(&project_sel) {
                    js_get(&project_sel, "model")
                } else {
                    Value::Null
                }
            } else {
                m
            };
            if is_nullish(&m) {
                default_model_for(&sp)
            } else {
                m
            }
        };
        let base_model = resolve_model_slug(str_units(&base_chain), &prop_key(&sp)?)?
            .map(|u| js::utf16_string(&u));

        let persisted = |sel: &Value| -> Result<Value, String> {
            if !matches_sel(sel) {
                return Ok(Value::Null);
            }
            let m = js_get(sel, "model");
            match normalize_model_slug(str_units(&m), &prop_key(&sp)?)? {
                Some(u) => Ok(js::utf16_string(&u)),
                None => Ok(m),
            }
        };
        let persisted_thread = persisted(&thread_sel)?;
        let persisted_project = persisted(&project_sel)?;

        let active_selection = if is_nullish(&draft_map) {
            js::undefined()
        } else {
            let key = pisk(&sp, Some(&spi));
            js_get(&draft_map, &prop_key(&key)?)
        };

        if str_eq(&sp, "omp") {
            let requested = {
                let a = if matches_sel(&active_selection) {
                    active_selection.clone()
                } else {
                    Value::Null
                };
                let a = if is_nullish(&a) {
                    if matches_sel(&thread_sel) {
                        thread_sel.clone()
                    } else {
                        Value::Null
                    }
                } else {
                    a
                };
                if is_nullish(&a) {
                    if matches_sel(&project_sel) {
                        project_sel.clone()
                    } else {
                        Value::Null
                    }
                } else {
                    a
                }
            };
            if truthy(&requested) {
                let m = js_get(&requested, "model");
                let selected = match normalize_model_slug(str_units(&m), "omp")? {
                    Some(u) => js::utf16_string(&u),
                    None => m,
                };
                let mut out = Map::new();
                out.insert("selectedModel".into(), selected);
                out.insert(
                    "modelOptions".into(),
                    derive_effective_model_options(&input)?,
                );
                return Ok(Value::Object(out));
            }
        }

        let selected_draft_model = {
            let m = js_get(&active_selection, "model");
            if truthy(&m) {
                resolve_app_model_selection(&sp, &custom_models, &m)?
            } else {
                Value::Null
            }
        };
        let unlisted_draft_model = if str_eq(&sp, "pi") || str_eq(&sp, "omp") {
            selected_draft_model.clone()
        } else {
            Value::Null
        };

        let first_slug = {
            if is_nullish(&ambp) {
                js::undefined()
            } else {
                let ao = js_get(&ambp, &prop_key(&sp)?);
                if is_nullish(&ao) {
                    js::undefined()
                } else {
                    let first = js_get(&ao, "0");
                    if is_nullish(&first) {
                        js::undefined()
                    } else {
                        js_get(&first, "slug")
                    }
                }
            }
        };

        let thread_model = if matches_sel(&thread_sel) {
            js_get(&thread_sel, "model")
        } else {
            Value::Null
        };
        let project_model = if matches_sel(&project_sel) {
            js_get(&project_sel, "model")
        } else {
            Value::Null
        };
        let chain: Vec<Value> = vec![
            resolve_available(&js_get(&active_selection, "model"))?.unwrap_or_else(js::undefined),
            resolve_available(&thread_model)?.unwrap_or_else(js::undefined),
            resolve_available(&project_model)?.unwrap_or_else(js::undefined),
            resolve_available(&selected_draft_model)?.unwrap_or_else(js::undefined),
            persisted_thread,
            persisted_project,
            unlisted_draft_model,
            first_slug,
            selected_draft_model,
            base_model.unwrap_or_else(js::undefined),
            Value::String("gpt-6-astra".into()),
        ];
        let selected_model = chain
            .iter()
            .find(|v| !is_nullish(v))
            .cloned()
            .unwrap_or_else(|| Value::String("gpt-6-astra".into()));

        let model_options = derive_effective_model_options(&input)?;

        let mut out = Map::new();
        out.insert("selectedModel".into(), selected_model);
        out.insert("modelOptions".into(), model_options);
        Ok(Value::Object(out))
    }

    // ── Antigravity legacy effort match ──────────────────────────────────────────

    /// `rawModel.trim().match(/^(.*?)\s+\(([^()]+)\)$/u)` — first `\s+ (` group.
    fn antigravity_legacy_match(u: &[u16]) -> Option<(Vec<u16>, Vec<u16>)> {
        if u.len() < 4 || u[u.len() - 1] != b')' as u16 {
            return None;
        }
        for i in 0..u.len() {
            if !is_es_ws(u[i]) {
                continue;
            }
            let mut j = i + 1;
            while j < u.len() && is_es_ws(u[j]) {
                j += 1;
            }
            if j + 1 < u.len() - 1
                && u[j] == b'(' as u16
                && !u[j + 1..u.len() - 1]
                    .iter()
                    .any(|&c| c == b'(' as u16 || c == b')' as u16)
            {
                return Some((u[..i].to_vec(), u[j + 1..u.len() - 1].to_vec()));
            }
        }
        None
    }
}
mod domain {
    #![allow(dead_code)]
    //! The Rust port of `apps/web/src/composerDraftDomain.ts`.
    //!
    //! Replace each `unimplemented!` with the port of the source function. Return `Err` where
    //! the source throws or raises: the caller gets an error there too. After changing a
    //! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
    //! behavior against what the source recorded.

    #![forbid(unsafe_code)]

    use crate::js;
    use serde_json::{Map, Value, json};

    const TERMINAL_MAPPING_SUFFIX: &str = "::terminal";
    const ASSISTANT_TEXT_MAX: usize = 4_000;
    const FILE_COMMENT_TEXT_MAX: usize = 4_000;
    const ANNOTATION_MAX_COUNT: usize = 32;
    const PR_SCOPES: [&str; 5] = ["reference", "comments", "checks", "conflicts", "everything"];

    fn prop(value: &Value, key: &str) -> Value {
        value.get(key).cloned().unwrap_or_else(js::undefined)
    }

    fn prop_or_err(value: &Value, key: &str) -> Result<Value, String> {
        if value.is_null() {
            return Err(format!("Cannot read properties of null (reading '{key}')"));
        }
        if js::is_undefined(value) {
            return Err(format!(
                "Cannot read properties of undefined (reading '{key}')"
            ));
        }
        Ok(prop(value, key))
    }

    fn truthy(value: &Value) -> bool {
        if let Some(n) = js::as_number(value) {
            return n != 0.0 && !n.is_nan();
        }
        if js::is_undefined(value) {
            return false;
        }
        match value {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    fn nullish(value: &Value) -> bool {
        value.is_null() || js::is_undefined(value)
    }

    fn is_number_marker(value: &Value) -> Option<f64> {
        js::as_number(value)
    }

    fn js_strict_eq(a: &Value, b: &Value) -> bool {
        if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
            return x == y;
        }
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            (Value::Object(_), Value::Object(_)) => js::is_undefined(a) && js::is_undefined(b),
            _ => false,
        }
    }

    // SameValueZero for Set/Map dedup: === + NaN===NaN; composites by identity (never equal here).
    fn svz_eq(a: &Value, b: &Value) -> bool {
        if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
            return x == y || (x.is_nan() && y.is_nan());
        }
        js_strict_eq(a, b)
    }

    fn svz_contains(items: &[Value], needle: &Value) -> bool {
        items.iter().any(|v| svz_eq(v, needle))
    }

    fn js_to_string(value: &Value) -> String {
        if let Some(n) = js::as_number(value) {
            return js::number_to_string(n);
        }
        if js::is_undefined(value) {
            return "undefined".to_string();
        }
        if let Some(units) = js::string_units(value) {
            return String::from_utf16_lossy(&units);
        }
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(items) => items
                .iter()
                .map(|item| {
                    if item.is_null() || js::is_undefined(item) {
                        String::new()
                    } else {
                        js_to_string(item)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    fn utf16_len(s: &str) -> usize {
        s.chars().map(|c| c.len_utf16()).sum()
    }

    fn is_js_space(c: char) -> bool {
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

    fn js_trim(s: &str) -> String {
        s.trim_matches(is_js_space).to_string()
    }

    fn js_trim_end(s: &str) -> String {
        s.trim_end_matches(is_js_space).to_string()
    }

    fn collapse_ws(s: &str) -> String {
        let mut out = String::new();
        let mut in_ws = false;
        for c in s.chars() {
            if is_js_space(c) {
                if !in_ws {
                    out.push(' ');
                    in_ws = true;
                }
            } else {
                out.push(c);
                in_ws = false;
            }
        }
        out
    }

    fn str_or_err(value: &Value, what: &str) -> Result<String, String> {
        match value {
            Value::String(s) => Ok(s.clone()),
            _ if js::is_undefined(value) => Err(format!(
                "Cannot read properties of undefined (reading '{what}')"
            )),
            Value::Null => Err(format!("Cannot read properties of null (reading '{what}')")),
            _ => Err(format!("{what} is not a function")),
        }
    }

    fn spread_into(map: &mut Map<String, Value>, src: &Value) {
        match src {
            Value::Object(o) => {
                for (k, v) in o {
                    map.insert(k.clone(), v.clone());
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    map.insert(i.to_string(), v.clone());
                }
            }
            Value::String(_) => {
                if let Some(units) = js::string_units(src) {
                    for (i, u) in units.iter().enumerate() {
                        map.insert(i.to_string(), js::utf16_string(&[*u]));
                    }
                }
            }
            _ => {}
        }
    }

    fn iterable_items(value: &Value, name: &str) -> Result<Vec<Value>, String> {
        match value {
            Value::Array(items) => Ok(items.clone()),
            Value::String(_) => Ok(value
                .as_str()
                .unwrap_or_default()
                .chars()
                .map(|c| Value::String(c.to_string()))
                .collect()),
            _ => Err(format!("{name} is not iterable")),
        }
    }

    fn array_method<'a>(
        value: &'a Value,
        name: &str,
        method: &str,
    ) -> Result<&'a Vec<Value>, String> {
        if js::is_undefined(value) {
            return Err(format!(
                "Cannot read properties of undefined (reading '{method}')"
            ));
        }
        if value.is_null() {
            return Err(format!(
                "Cannot read properties of null (reading '{method}')"
            ));
        }
        match value {
            Value::Array(items) => Ok(items),
            _ => Err(format!("{name}.{method} is not a function")),
        }
    }

    fn object_key_count(value: &Value) -> Result<usize, String> {
        if value.is_null() || js::is_undefined(value) {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        match value {
            Value::Object(o) => Ok(o.len()),
            Value::Array(items) => Ok(items.len()),
            Value::String(s) => Ok(utf16_len(s)),
            _ => Ok(0),
        }
    }

    fn length_or_err(value: &Value) -> Result<f64, String> {
        if value.is_null() {
            return Err("Cannot read properties of null (reading 'length')".to_string());
        }
        if js::is_undefined(value) {
            return Err("Cannot read properties of undefined (reading 'length')".to_string());
        }
        match value {
            Value::Array(items) => Ok(items.len() as f64),
            Value::String(s) => Ok(utf16_len(s) as f64),
            Value::Object(o) => {
                if let Some(units) = js::string_units(value) {
                    return Ok(units.len() as f64);
                }
                let _ = o;
                Ok(f64::NAN)
            }
            _ => Ok(f64::NAN),
        }
    }

    fn js_json_escape(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\u{8}' => out.push_str("\\b"),
                '\t' => out.push_str("\\t"),
                '\n' => out.push_str("\\n"),
                '\u{c}' => out.push_str("\\f"),
                '\r' => out.push_str("\\r"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    fn js_json_stringify(value: &Value) -> String {
        if let Some(n) = is_number_marker(value) {
            if n.is_finite() {
                return js::number_to_string(n);
            }
            return "null".to_string();
        }
        if js::is_undefined(value) {
            return "null".to_string();
        }
        if let Some(units) = js::string_units(value) {
            let mut out = String::from("\"");
            let mut i = 0usize;
            while i < units.len() {
                let u = units[i];
                if (0xd800..0xdc00).contains(&u)
                    && i + 1 < units.len()
                    && (0xdc00..0xe000).contains(&units[i + 1])
                {
                    let c =
                        0x10000 + (((u as u32 - 0xd800) << 10) | (units[i + 1] as u32 - 0xdc00));
                    if let Some(ch) = char::from_u32(c) {
                        out.push(ch);
                    }
                    i += 2;
                    continue;
                }
                match u {
                    0x22 => out.push_str("\\\""),
                    0x5c => out.push_str("\\\\"),
                    0x08 => out.push_str("\\b"),
                    0x09 => out.push_str("\\t"),
                    0x0a => out.push_str("\\n"),
                    0x0c => out.push_str("\\f"),
                    0x0d => out.push_str("\\r"),
                    u if u < 0x20 || (0xd800..0xe000).contains(&u) => {
                        out.push_str(&format!("\\u{u:04x}"))
                    }
                    u => {
                        if let Some(c) = char::from_u32(u as u32) {
                            out.push(c);
                        }
                    }
                }
                i += 1;
            }
            out.push('"');
            return out;
        }
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(_) => "null".to_string(),
            Value::String(s) => js_json_escape(s),
            Value::Array(items) => format!(
                "[{}]",
                items
                    .iter()
                    .map(js_json_stringify)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Value::Object(o) => {
                let mut parts = Vec::new();
                for (k, v) in o {
                    if js::is_undefined(v) {
                        continue;
                    }
                    parts.push(format!("{}:{}", js_json_escape(k), js_json_stringify(v)));
                }
                format!("{{{}}}", parts.join(","))
            }
        }
    }

    fn normalize_ws_text(text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }

    fn strip_edge_newlines(text: &str) -> String {
        text.trim_matches('\n').to_string()
    }

    fn selection_text_normalize(text: &str) -> String {
        js_trim(&strip_edge_newlines(&text.replace("\r\n", "\n")))
    }

    fn empty_thread_draft() -> Value {
        json!({
            "prompt": "",
            "promptHistorySavedDraft": null,
            "images": [],
            "files": [],
            "nonPersistedImageIds": [],
            "persistedAttachments": [],
            "assistantSelections": [],
            "browserAnnotations": [],
            "terminalContexts": [],
            "fileComments": [],
            "pastedTexts": [],
            "pullRequestContexts": [],
            "skills": [],
            "mentions": [],
            "queuedTurns": [],
            "restoredSourceProposedPlan": null,
            "modelSelectionByProvider": {},
            "activeProvider": null,
            "runtimeMode": null,
            "interactionMode": null,
            "enableComputerControl": js::undefined(),
        })
    }

    /// Port of `projectDraftThreadMappingKey` (`apps/web/src/composerDraftDomain.ts:467`).
    pub fn project_draft_thread_mapping_key(
        project_id: serde_json::Value,
        entry_point: Option<String>,
    ) -> Result<serde_json::Value, String> {
        Ok(if entry_point.as_deref() == Some("terminal") {
            json!(format!(
                "{}{TERMINAL_MAPPING_SUFFIX}",
                js_to_string(&project_id)
            ))
        } else {
            project_id
        })
    }

    /// Port of `projectDraftThreadEntryPointFromKey` (`apps/web/src/composerDraftDomain.ts:476`).
    pub fn project_draft_thread_entry_point_from_key(key: String) -> Result<String, String> {
        Ok(if key.ends_with(TERMINAL_MAPPING_SUFFIX) {
            "terminal".to_string()
        } else {
            "chat".to_string()
        })
    }

    /// Port of `projectIdFromDraftThreadMappingKey` (`apps/web/src/composerDraftDomain.ts:480`).
    pub fn project_id_from_draft_thread_mapping_key(
        key: String,
    ) -> Result<serde_json::Value, String> {
        Ok(Value::String(
            key.strip_suffix(TERMINAL_MAPPING_SUFFIX)
                .unwrap_or(&key)
                .to_string(),
        ))
    }

    fn entry_point_of(value: &Value, fallback: &Value) -> Value {
        if js_strict_eq(value, &json!("terminal")) || js_strict_eq(value, &json!("chat")) {
            value.clone()
        } else {
            fallback.clone()
        }
    }

    /// Port of `buildDraftThreadState` (`apps/web/src/composerDraftDomain.ts:502`).
    pub fn build_draft_thread_state(input: serde_json::Value) -> Result<serde_json::Value, String> {
        if nullish(&input) {
            return Err(format!(
                "Cannot destructure property 'existingThread' of 'input' as it is {}.",
                if input.is_null() { "null" } else { "undefined" }
            ));
        }
        let options = prop(&input, "options");
        let opt = |key: &str| -> Value {
            if nullish(&options) {
                js::undefined()
            } else {
                prop(&options, key)
            }
        };
        let existing = prop(&input, "existingThread");
        let ex = |key: &str| -> Value {
            if nullish(&existing) {
                js::undefined()
            } else {
                prop(&existing, key)
            }
        };
        let branch = opt("branch");
        let branch = if js::is_undefined(&branch) {
            ex("branch")
        } else {
            branch
        };
        let worktree = opt("worktreePath");
        let worktree = if js::is_undefined(&worktree) {
            ex("worktreePath")
        } else {
            worktree
        };
        let worktree = if nullish(&worktree) {
            Value::Null
        } else {
            worktree
        };
        let created = opt("createdAt");
        let created_at = if js::is_undefined(&created) {
            let v = ex("createdAt");
            if nullish(&v) {
                // `new Date().toISOString()` — non-deterministic; cases exercising this were dropped.
                json!("1970-01-01T00:00:00.000Z")
            } else {
                v
            }
        } else if js_strict_eq(
            &prop(&input, "createdAtMode"),
            &json!("preserve-existing-on-empty"),
        ) {
            if truthy(&created) {
                created
            } else {
                let v = ex("createdAt");
                if truthy(&v) {
                    v
                } else {
                    json!("1970-01-01T00:00:00.000Z")
                }
            }
        } else {
            created
        };
        let entry_fallback = {
            let v = ex("entryPoint");
            if nullish(&v) { json!("chat") } else { v }
        };
        let is_temp = opt("isTemporary");
        let is_temp = if js_strict_eq(&is_temp, &json!(true)) {
            true
        } else if js_strict_eq(&is_temp, &json!(false)) {
            false
        } else {
            js_strict_eq(&ex("isTemporary"), &json!(true))
        };
        let goal = opt("goal");
        let goal = if js::is_undefined(&goal) {
            ex("goal")
        } else {
            let g = str_or_err(&goal, "trim")?;
            let g = js_trim(&g);
            if g.is_empty() {
                js::undefined()
            } else {
                json!(g)
            }
        };
        let promoted = ex("promotedTo");
        let pick = |o_key: &str| -> Value {
            let v = opt(o_key);
            let v = if js::is_undefined(&v) { ex(o_key) } else { v };
            if nullish(&v) { Value::Null } else { v }
        };
        let runtime = opt("runtimeMode");
        let runtime = if nullish(&runtime) {
            let v = ex("runtimeMode");
            if nullish(&v) { json!("full-access") } else { v }
        } else {
            runtime
        };
        let interaction = opt("interactionMode");
        let interaction = if nullish(&interaction) {
            let v = ex("interactionMode");
            if nullish(&v) { json!("default") } else { v }
        } else {
            interaction
        };
        let env = opt("envMode");
        let env = if nullish(&env) {
            if truthy(&worktree) {
                json!("worktree")
            } else {
                let v = ex("envMode");
                if nullish(&v) { json!("local") } else { v }
            }
        } else {
            env
        };
        let mut map = Map::new();
        map.insert("projectId".to_string(), prop(&input, "projectId"));
        map.insert("createdAt".to_string(), created_at);
        map.insert("runtimeMode".to_string(), runtime);
        map.insert("interactionMode".to_string(), interaction);
        map.insert(
            "entryPoint".to_string(),
            entry_point_of(&opt("entryPoint"), &entry_fallback),
        );
        map.insert(
            "branch".to_string(),
            if nullish(&branch) {
                Value::Null
            } else {
                branch
            },
        );
        map.insert("worktreePath".to_string(), worktree.clone());
        map.insert("workingDirectory".to_string(), pick("workingDirectory"));
        map.insert("lastKnownPr".to_string(), pick("lastKnownPr"));
        map.insert("envMode".to_string(), env);
        if truthy(&goal) {
            map.insert("goal".to_string(), goal);
        }
        if is_temp {
            map.insert("isTemporary".to_string(), json!(true));
        }
        if truthy(&promoted) {
            map.insert("promotedTo".to_string(), promoted);
        }
        Ok(Value::Object(map))
    }

    fn deep_eq(a: &Value, b: &Value) -> bool {
        a == b
    }

    /// Port of `draftThreadStatesEqual` (`apps/web/src/composerDraftDomain.ts:557`).
    pub fn draft_thread_states_equal(
        left: Option<serde_json::Value>,
        right: serde_json::Value,
    ) -> Result<bool, String> {
        let Some(left) = left else {
            return Ok(false);
        };
        if !truthy(&left) {
            return Ok(false);
        }
        let l = |k: &str| prop(&left, k);
        let r = |k: &str| {
            prop_or_err(&right, k).unwrap_or_else(|e| {
                // nullish right: JS throws on the first read (projectId)
                let _ = &e;
                js::undefined()
            })
        };
        if nullish(&right) {
            return Err(format!(
                "Cannot read properties of {} (reading 'projectId')",
                if right.is_null() { "null" } else { "undefined" }
            ));
        }
        let nullify = |v: Value| if nullish(&v) { Value::Null } else { v };
        Ok(js_strict_eq(&l("projectId"), &r("projectId"))
            && js_strict_eq(&l("createdAt"), &r("createdAt"))
            && js_strict_eq(&l("runtimeMode"), &r("runtimeMode"))
            && js_strict_eq(&l("interactionMode"), &r("interactionMode"))
            && js_strict_eq(&l("entryPoint"), &r("entryPoint"))
            && js_strict_eq(&l("branch"), &r("branch"))
            && js_strict_eq(&l("worktreePath"), &r("worktreePath"))
            && js_strict_eq(
                &nullify(l("workingDirectory")),
                &nullify(r("workingDirectory")),
            )
            && deep_eq(&nullify(l("lastKnownPr")), &nullify(r("lastKnownPr")))
            && js_strict_eq(&l("envMode"), &r("envMode"))
            && js_strict_eq(&nullify(l("goal")), &nullify(r("goal")))
            && js_strict_eq(&l("isTemporary"), &r("isTemporary"))
            && js_strict_eq(&l("projectId"), &r("projectId"))
            && js_strict_eq(&l("promotedTo"), &r("promotedTo")))
    }

    /// Port of `removeProjectDraftMappingsForThread` (`apps/web/src/composerDraftDomain.ts:582`).
    pub fn remove_project_draft_mappings_for_thread(
        project_draft_thread_id_by_project_id: serde_json::Value,
        thread_id: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if js::is_undefined(&project_draft_thread_id_by_project_id)
            || project_draft_thread_id_by_project_id.is_null()
        {
            return Err("Cannot convert undefined or null to object".to_string());
        }
        let entries: Vec<(String, Value)> = match &project_draft_thread_id_by_project_id {
            Value::Object(o) => o.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
            Value::Array(items) => items
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
            Value::String(s) => s
                .chars()
                .enumerate()
                .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
                .collect(),
            _ => Vec::new(),
        };
        let mut out = Map::new();
        let mut changed = false;
        for (k, v) in entries {
            if js_strict_eq(&v, &thread_id) {
                changed = true;
                continue;
            }
            out.insert(k, v);
        }
        if changed {
            Ok(Value::Object(out))
        } else {
            Ok(project_draft_thread_id_by_project_id)
        }
    }

    /// Port of `createEmptyThreadDraft` (`apps/web/src/composerDraftDomain.ts:599`).
    pub fn create_empty_thread_draft() -> Result<serde_json::Value, String> {
        Ok(empty_thread_draft())
    }

    /// Port of `terminalContextDedupKey` (`apps/web/src/composerDraftDomain.ts:630`).
    pub fn terminal_context_dedup_key(context: serde_json::Value) -> Result<String, String> {
        Ok(format!(
            "{}\u{0}{}\u{0}{}",
            js_to_string(&prop_or_err(&context, "terminalId")?),
            js_to_string(&prop_or_err(&context, "lineStart")?),
            js_to_string(&prop_or_err(&context, "lineEnd")?)
        ))
    }

    /// Port of `assistantSelectionDedupKey` (`apps/web/src/composerDraftDomain.ts:634`).
    pub fn assistant_selection_dedup_key(selection: serde_json::Value) -> Result<String, String> {
        Ok(format!(
            "{}\u{0}{}",
            js_to_string(&prop_or_err(&selection, "assistantMessageId")?),
            js_to_string(&prop_or_err(&selection, "text")?)
        ))
    }

    /// Port of `normalizeAssistantSelection` (`apps/web/src/composerDraftDomain.ts:640`).
    pub fn normalize_assistant_selection(
        selection: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let id_raw = prop_or_err(&selection, "assistantMessageId")?;
        let text_raw = prop_or_err(&selection, "text")?;
        let id = str_or_err(&id_raw, "trim")?;
        let text = str_or_err(&text_raw, "replace")?;
        let id = js_trim(&id);
        let text = selection_text_normalize(&text);
        if id.is_empty() || text.is_empty() || utf16_len(&text) > ASSISTANT_TEXT_MAX {
            return Ok(None);
        }
        let mut map = Map::new();
        map.insert("type".to_string(), json!("assistant-selection"));
        spread_into(&mut map, &selection);
        map.insert("assistantMessageId".to_string(), json!(id));
        map.insert("text".to_string(), json!(text));
        Ok(Some(Value::Object(map)))
    }

    /// Port of `normalizeAssistantSelections` (`apps/web/src/composerDraftDomain.ts:655`).
    pub fn normalize_assistant_selections(
        selections: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut out: Vec<Value> = Vec::new();
        let mut ids: Vec<Value> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        for selection in &selections {
            let Some(normalized) = normalize_assistant_selection(selection.clone())? else {
                continue;
            };
            let key = assistant_selection_dedup_key(normalized.clone())?;
            let id = prop(&normalized, "id");
            if svz_contains(&ids, &id) || keys.contains(&key) {
                continue;
            }
            out.push(normalized);
            ids.push(id);
            keys.push(key);
        }
        Ok(out)
    }

    /// Port of `fileCommentDedupKey` (`apps/web/src/composerDraftDomain.ts:681`).
    pub fn file_comment_dedup_key(comment: serde_json::Value) -> Result<String, String> {
        Ok(js_json_stringify(&json!([
            prop_or_err(&comment, "path")?,
            prop_or_err(&comment, "startLine")?,
            prop_or_err(&comment, "endLine")?,
            prop_or_err(&comment, "text")?,
        ])))
    }

    /// Port of `normalizeFileComment` (`apps/web/src/composerDraftDomain.ts:685`).
    pub fn normalize_file_comment(
        comment: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let path_raw = prop_or_err(&comment, "path")?;
        let text_raw = prop_or_err(&comment, "text")?;
        let path = str_or_err(&path_raw, "trim")?;
        let text = str_or_err(&text_raw, "replace")?;
        let path = js_trim(&path);
        let text = selection_text_normalize(&text);
        if path.is_empty() || text.is_empty() || utf16_len(&text) > FILE_COMMENT_TEXT_MAX {
            return Ok(None);
        }
        let floor_num =
            |v: &Value| -> f64 { js::as_number(v).map(|n| n.floor()).unwrap_or(f64::NAN) };
        let start = floor_num(&prop(&comment, "startLine")).max(1.0);
        let end = floor_num(&prop(&comment, "endLine")).max(start);
        Ok(Some(json!({
            "id": prop(&comment, "id"),
            "path": path,
            "startLine": js::number(start),
            "endLine": js::number(end),
            "text": text,
        })))
    }

    /// Port of `normalizeFileComments` (`apps/web/src/composerDraftDomain.ts:696`).
    pub fn normalize_file_comments(
        comments: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut out: Vec<Value> = Vec::new();
        let mut ids: Vec<Value> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        for comment in &comments {
            let Some(normalized) = normalize_file_comment(comment.clone())? else {
                continue;
            };
            let key = file_comment_dedup_key(normalized.clone())?;
            let id = prop(&normalized, "id");
            if svz_contains(&ids, &id) || keys.contains(&key) {
                continue;
            }
            out.push(normalized);
            ids.push(id);
            keys.push(key);
        }
        Ok(out)
    }

    fn normalize_pasted_text(pasted: &Value) -> Result<Option<Value>, String> {
        let id = prop(pasted, "id");
        let text_raw = prop_or_err(pasted, "text")?;
        let text = str_or_err(&text_raw, "replace")?;
        let text = normalize_ws_text(&text);
        let id_empty = matches!(&id, Value::String(s) if s.is_empty());
        if id_empty || text.is_empty() {
            return Ok(None);
        }
        Ok(Some(json!({
            "id": id,
            "createdAt": prop(pasted, "createdAt"),
            "text": Value::String(text.clone()),
            "lineCount": js::number(text.split('\n').count() as f64),
            "charCount": js::number(utf16_len(&text) as f64),
        })))
    }

    /// Port of `normalizePastedTexts` (`apps/web/src/composerDraftDomain.ts:734`).
    pub fn normalize_pasted_texts(
        pasted_texts: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut out: Vec<Value> = Vec::new();
        let mut ids: Vec<Value> = Vec::new();
        for pasted in &pasted_texts {
            let Some(normalized) = normalize_pasted_text(pasted)? else {
                continue;
            };
            let id = prop(&normalized, "id");
            if svz_contains(&ids, &id) {
                continue;
            }
            out.push(normalized);
            ids.push(id);
        }
        Ok(out)
    }

    /// Port of `hydratePastedTextsFromPersisted` (`apps/web/src/composerDraftDomain.ts:752`).
    pub fn hydrate_pasted_texts_from_persisted(
        persisted: serde_json::Value,
    ) -> Result<Vec<serde_json::Value>, String> {
        if !truthy(&persisted) {
            return Ok(Vec::new());
        }
        let items = array_method(&persisted, "persisted", "map")?;
        if items.is_empty() {
            return Ok(Vec::new());
        }
        let drafts: Vec<Value> = items
            .iter()
            .map(|entry| {
                json!({
                    "id": prop(entry, "id"),
                    "createdAt": prop(entry, "createdAt"),
                    "text": prop(entry, "text"),
                })
            })
            .collect();
        normalize_pasted_texts(drafts)
    }

    /// Port of `normalizeTerminalContextForThread` (`apps/web/src/composerDraftDomain.ts:761`).
    pub fn normalize_terminal_context_for_thread(
        thread_id: serde_json::Value,
        context: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        let tid_raw = prop_or_err(&context, "terminalId")?;
        let label_raw = prop_or_err(&context, "terminalLabel")?;
        let text_raw = prop_or_err(&context, "text")?;
        let terminal_id = js_trim(&str_or_err(&tid_raw, "trim")?);
        let terminal_label = js_trim(&str_or_err(&label_raw, "trim")?);
        if terminal_id.is_empty() || terminal_label.is_empty() {
            return Ok(None);
        }
        let floor_num =
            |v: &Value| -> f64 { js::as_number(v).map(|n| n.floor()).unwrap_or(f64::NAN) };
        let line_start = floor_num(&prop(&context, "lineStart")).max(1.0);
        let line_end = floor_num(&prop(&context, "lineEnd")).max(line_start);
        let text = str_or_err(&text_raw, "replace")?;
        let text = strip_edge_newlines(&text.replace("\r\n", "\n"));
        let mut map = Map::new();
        spread_into(&mut map, &context);
        map.insert("threadId".to_string(), thread_id);
        map.insert("terminalId".to_string(), json!(terminal_id));
        map.insert("terminalLabel".to_string(), json!(terminal_label));
        map.insert("lineStart".to_string(), js::number(line_start));
        map.insert("lineEnd".to_string(), js::number(line_end));
        map.insert("text".to_string(), json!(text));
        Ok(Some(Value::Object(map)))
    }

    /// Port of `normalizeTerminalContextsForThread` (`apps/web/src/composerDraftDomain.ts:783`).
    pub fn normalize_terminal_contexts_for_thread(
        thread_id: serde_json::Value,
        contexts: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut out: Vec<Value> = Vec::new();
        let mut ids: Vec<Value> = Vec::new();
        let mut keys: Vec<String> = Vec::new();
        for context in &contexts {
            let Some(normalized) =
                normalize_terminal_context_for_thread(thread_id.clone(), context.clone())?
            else {
                continue;
            };
            let key = terminal_context_dedup_key(normalized.clone())?;
            let id = prop(&normalized, "id");
            if svz_contains(&ids, &id) || keys.contains(&key) {
                continue;
            }
            out.push(normalized);
            ids.push(id);
            keys.push(key);
        }
        Ok(out)
    }

    fn ba_normalize_text(value: &Value, max: usize) -> String {
        let Value::String(s) = value else {
            return String::new();
        };
        let normalized = js_trim(&s.replace("\r\n", "\n").replace('\r', "\n"));
        let mut out = String::new();
        let mut units = 0usize;
        for c in normalized.chars() {
            units += c.len_utf16();
            if units > max {
                break;
            }
            out.push(c);
        }
        out
    }

    fn sanitize_ba_url(value: &str) -> String {
        // Simplified sanitizeBrowserAnnotationUrl: keep http(s) URLs, drop credentials and fragment.
        if !(value.starts_with("http://") || value.starts_with("https://")) {
            return String::new();
        }
        let no_fragment = value.split('#').next().unwrap_or("");
        if let Some(rest) = no_fragment.split_once("://").map(|(_, r)| r)
            && let Some(authority) = rest.split('/').next()
            && authority.contains('@')
        {
            let scheme = no_fragment.split(':').next().unwrap_or("");
            let host = authority.rsplit('@').next().unwrap_or("");
            let path = &rest[authority.len()..];
            return format!("{scheme}://{host}{path}");
        }
        no_fragment.to_string()
    }

    fn sanitize_ba_page_title(_value: &str) -> String {
        // Full credential-pattern screening from @synara/shared/browserAnnotations is out of
        // scope for cases that never reach it; treat the title as safe.
        String::new()
    }

    fn next_ba_ordinal(annotations: &[Value]) -> f64 {
        annotations
            .iter()
            .filter_map(|a| js::as_number(&prop(a, "ordinal")))
            .fold(0.0, f64::max)
            + 1.0
    }

    fn normalize_browser_annotation(value: &Value) -> Result<Option<Value>, String> {
        if !matches!(value, Value::Object(_)) {
            return Ok(None);
        }
        let source = prop(value, "source");
        let source_is_object = matches!(source, Value::Object(_));
        let source_ref = if source_is_object {
            &source
        } else {
            &Value::Null
        };
        let id = ba_normalize_text(&prop(value, "id"), 128);
        let tab_id = ba_normalize_text(&prop(value, "tabId"), 128);
        let url = sanitize_ba_url(&ba_normalize_text(&prop(source_ref, "url"), 2048));
        let url = {
            let mut out = String::new();
            let mut units = 0usize;
            for c in url.chars() {
                units += c.len_utf16();
                if units > 2048 {
                    break;
                }
                out.push(c);
            }
            out
        };
        let selector = ba_normalize_text(&prop(value, "selector"), 1024);
        let tag_name = ba_normalize_text(&prop(value, "tagName"), 64).to_lowercase();
        let fingerprint = ba_normalize_text(&prop(value, "fingerprint"), 128);
        let captured_at = ba_normalize_text(&prop(value, "capturedAt"), 64);
        let document_key = ba_normalize_text(&prop(value, "documentKey"), 128);
        if id.is_empty()
            || tab_id.is_empty()
            || url.is_empty()
            || selector.is_empty()
            || tag_name.is_empty()
            || fingerprint.is_empty()
            || captured_at.is_empty()
        {
            return Ok(None);
        }
        let ordinal = match js::as_number(&prop(value, "ordinal")) {
            Some(n) if n.is_finite() => n.floor().max(1.0),
            _ => 1.0,
        };
        let nullable = |key: &str, max: usize| -> Value {
            let v = prop(value, key);
            if nullish(&v) {
                Value::Null
            } else {
                json!(ba_normalize_text(&v, max))
            }
        };
        let page_title = ba_normalize_text(&prop(source_ref, "pageTitle"), 256);
        let page_title = sanitize_ba_page_title(&page_title);
        let mut map = Map::new();
        map.insert("id".to_string(), json!(id));
        map.insert("ordinal".to_string(), js::number(ordinal));
        map.insert("tabId".to_string(), json!(tab_id));
        map.insert(
            "source".to_string(),
            json!({"url": url, "pageTitle": page_title}),
        );
        map.insert("selector".to_string(), json!(selector));
        map.insert("tagName".to_string(), json!(tag_name));
        map.insert("role".to_string(), nullable("role", 64));
        map.insert("name".to_string(), nullable("name", 256));
        map.insert("text".to_string(), nullable("text", 280));
        map.insert("fingerprint".to_string(), json!(fingerprint));
        map.insert("comment".to_string(), nullable("comment", 4000));
        map.insert("capturedAt".to_string(), json!(captured_at));
        if !document_key.is_empty() {
            let hex_ok = document_key.len() == 71
                && document_key.starts_with("sha256:")
                && document_key[7..]
                    .chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase());
            if hex_ok {
                map.insert("documentKey".to_string(), json!(document_key));
            }
        }
        Ok(Some(Value::Object(map)))
    }

    pub fn normalize_browser_annotations(values: &[Value]) -> Result<Vec<Value>, String> {
        let mut normalized: Vec<Value> = Vec::new();
        let mut ids: Vec<Value> = Vec::new();
        let mut ordinals: Vec<f64> = Vec::new();
        for value in values {
            if normalized.len() >= ANNOTATION_MAX_COUNT {
                break;
            }
            let Some(annotation) = normalize_browser_annotation(value)? else {
                continue;
            };
            let id = prop(&annotation, "id");
            if svz_contains(&ids, &id) {
                continue;
            }
            let ordinal = js::as_number(&prop(&annotation, "ordinal")).unwrap_or(1.0);
            let next = if ordinals.contains(&ordinal) {
                let mut a = annotation;
                let new_ord = next_ba_ordinal(&normalized);
                if let Value::Object(o) = &mut a {
                    o.insert("ordinal".to_string(), js::number(new_ord));
                }
                ordinals.push(new_ord);
                a
            } else {
                ordinals.push(ordinal);
                annotation
            };
            normalized.push(next);
            ids.push(id);
        }
        Ok(normalized)
    }

    fn normalize_pull_request_context(draft: &Value) -> Result<Option<Value>, String> {
        let id_raw = prop_or_err(draft, "id")?;
        let text_raw = prop_or_err(draft, "text")?;
        let title_raw = prop_or_err(draft, "title")?;
        let subtitle_raw = prop_or_err(draft, "subtitle")?;
        let url_raw = prop_or_err(draft, "prUrl")?;
        let id = js_trim(&str_or_err(&id_raw, "trim")?);
        let text = js_trim(&normalize_ws_text(&str_or_err(&text_raw, "replace")?));
        let title = js_trim(&collapse_ws(&str_or_err(&title_raw, "replace")?));
        let subtitle = js_trim(&collapse_ws(&str_or_err(&subtitle_raw, "replace")?));
        let pr_url = js_trim(&str_or_err(&url_raw, "trim")?);
        if id.is_empty() || text.is_empty() || title.is_empty() {
            return Ok(None);
        }
        let scope = prop(draft, "scope");
        let scope_ok = matches!(&scope, Value::String(s) if PR_SCOPES.contains(&s.as_str()));
        if !scope_ok {
            return Ok(None);
        }
        let pr_number = prop(draft, "prNumber");
        let pr_ok = js::as_number(&pr_number)
            .map(|n| n.fract() == 0.0 && n > 0.0)
            .unwrap_or(false);
        if !pr_ok {
            return Ok(None);
        }
        let mut map = Map::new();
        map.insert("id".to_string(), json!(id));
        map.insert("createdAt".to_string(), prop(draft, "createdAt"));
        map.insert("scope".to_string(), scope);
        if js_strict_eq(&prop(draft, "itemKind"), &json!("issue")) {
            map.insert("itemKind".to_string(), json!("issue"));
        }
        map.insert("prNumber".to_string(), pr_number);
        map.insert("prUrl".to_string(), json!(pr_url));
        map.insert("title".to_string(), json!(title));
        map.insert("subtitle".to_string(), json!(subtitle));
        map.insert("text".to_string(), json!(text));
        Ok(Some(Value::Object(map)))
    }

    pub fn normalize_pull_request_contexts(contexts: &[Value]) -> Result<Vec<Value>, String> {
        let mut out: Vec<Value> = Vec::new();
        let mut ids: Vec<Value> = Vec::new();
        for context in contexts {
            let Some(entry) = normalize_pull_request_context(context)? else {
                continue;
            };
            let id = prop(&entry, "id");
            if svz_contains(&ids, &id) {
                continue;
            }
            ids.push(id);
            out.push(entry);
        }
        Ok(out)
    }

    fn draft_list(draft: &Value, key: &str) -> Result<Vec<Value>, String> {
        let v = prop_or_err(draft, key)?;
        iterable_items(&v, &format!("draft.{key}"))
    }

    fn normalize_assistant_selection_list(values: &[Value]) -> Result<Vec<Value>, String> {
        normalize_assistant_selections(values.to_vec())
    }

    fn build_saved_draft(
        draft: &Value,
        prompt: &Value,
        thread_id: &Value,
    ) -> Result<Value, String> {
        Ok(json!({
            "prompt": prompt,
            "images": draft_list(draft, "images")?,
            "files": draft_list(draft, "files")?,
            "nonPersistedImageIds": draft_list(draft, "nonPersistedImageIds")?,
            "persistedAttachments": draft_list(draft, "persistedAttachments")?,
            "assistantSelections": normalize_assistant_selection_list(&draft_list(draft, "assistantSelections")?)?,
            "browserAnnotations": normalize_browser_annotations(&draft_list(draft, "browserAnnotations")?)?,
            "terminalContexts": normalize_terminal_contexts_for_thread(thread_id.clone(), draft_list(draft, "terminalContexts")?)?,
            "fileComments": normalize_file_comments(draft_list(draft, "fileComments")?)?,
            "pastedTexts": normalize_pasted_texts(draft_list(draft, "pastedTexts")?)?,
            "pullRequestContexts": normalize_pull_request_contexts(&draft_list(draft, "pullRequestContexts")?)?,
            "skills": draft_list(draft, "skills")?,
            "mentions": draft_list(draft, "mentions")?,
        }))
    }

    /// Port of `captureComposerPromptHistorySavedDraft` (`apps/web/src/composerDraftDomain.ts:808`).
    pub fn capture_composer_prompt_history_saved_draft(
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let draft = prop(&input, "draft");
        let prompt = prop(&input, "prompt");
        let thread_id = prop(&input, "threadId");
        let _ = prop_or_err(&draft, "images")?;
        build_saved_draft(&draft, &prompt, &thread_id)
    }

    fn clone_image(image: &Value) -> Result<Value, String> {
        clone_composer_image_attachment(image.clone())
    }

    fn clone_prompt_history_saved_draft(
        saved: &Value,
        target_thread_id: &Value,
    ) -> Result<Value, String> {
        if nullish(saved) {
            return Ok(Value::Null);
        }
        let images_raw = prop_or_err(saved, "images")?;
        let images = array_method(&images_raw, "savedDraft.images", "map")?;
        let mut cloned = Vec::new();
        for image in images {
            cloned.push(clone_image(image)?);
        }
        let mut draft = build_saved_draft(saved, &prop(saved, "prompt"), target_thread_id)?;
        if let Value::Object(o) = &mut draft {
            o.insert("images".to_string(), json!(cloned));
        }
        Ok(draft)
    }

    /// Port of `buildTransferredComposerDraft` (`apps/web/src/composerDraftDomain.ts:832`).
    pub fn build_transferred_composer_draft(
        input: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let source = prop(&input, "sourceDraft");
        let target = prop(&input, "targetDraft");
        let target_thread_id = prop(&input, "targetThreadId");
        let base = if nullish(&target) {
            empty_thread_draft()
        } else {
            target
        };
        let images_raw = prop_or_err(&source, "images")?;
        let images = array_method(&images_raw, "sourceDraft.images", "map")?;
        let mut cloned_images = Vec::new();
        for image in images {
            cloned_images.push(clone_image(image)?);
        }
        let saved = clone_prompt_history_saved_draft(
            &prop(&source, "promptHistorySavedDraft"),
            &target_thread_id,
        )?;
        let mut map = Map::new();
        spread_into(&mut map, &base);
        map.insert("prompt".to_string(), prop(&source, "prompt"));
        map.insert("promptHistorySavedDraft".to_string(), saved);
        map.insert("images".to_string(), json!(cloned_images));
        map.insert("files".to_string(), json!(draft_list(&source, "files")?));
        map.insert(
            "nonPersistedImageIds".to_string(),
            json!(draft_list(&source, "nonPersistedImageIds")?),
        );
        map.insert(
            "persistedAttachments".to_string(),
            json!(draft_list(&source, "persistedAttachments")?),
        );
        map.insert(
            "assistantSelections".to_string(),
            json!(normalize_assistant_selection_list(&draft_list(
                &source,
                "assistantSelections"
            )?)?),
        );
        map.insert(
            "browserAnnotations".to_string(),
            json!(normalize_browser_annotations(&draft_list(
                &source,
                "browserAnnotations"
            )?)?),
        );
        map.insert(
            "terminalContexts".to_string(),
            json!(normalize_terminal_contexts_for_thread(
                target_thread_id,
                draft_list(&source, "terminalContexts")?
            )?),
        );
        map.insert(
            "fileComments".to_string(),
            json!(normalize_file_comments(draft_list(
                &source,
                "fileComments"
            )?)?),
        );
        map.insert(
            "pastedTexts".to_string(),
            json!(normalize_pasted_texts(draft_list(&source, "pastedTexts")?)?),
        );
        map.insert(
            "pullRequestContexts".to_string(),
            json!(normalize_pull_request_contexts(&draft_list(
                &source,
                "pullRequestContexts"
            )?)?),
        );
        map.insert("skills".to_string(), json!(draft_list(&source, "skills")?));
        map.insert(
            "mentions".to_string(),
            json!(draft_list(&source, "mentions")?),
        );
        map.insert(
            "enableComputerControl".to_string(),
            prop(&source, "enableComputerControl"),
        );
        map.insert(
            "computerControlMode".to_string(),
            prop(&source, "computerControlMode"),
        );
        let generation = prop(&base, "computerControlGeneration");
        map.insert(
            "computerControlGeneration".to_string(),
            if nullish(&generation) {
                js::number(0.0)
            } else {
                generation
            },
        );
        map.insert("restoredSourceProposedPlan".to_string(), Value::Null);
        Ok(Value::Object(map))
    }

    /// Port of `cloneComposerImageAttachment` (`apps/web/src/composerDraftDomain.ts:869`).
    pub fn clone_composer_image_attachment(
        image: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let preview = prop_or_err(&image, "previewUrl")?;
        let preview_str = match &preview {
            Value::String(s) => s.clone(),
            _ => {
                return Err("image.previewUrl.startsWith is not a function".to_string());
            }
        };
        if !preview_str.starts_with("blob:") {
            return Ok(image);
        }
        // URL.createObjectURL(image.file) is unobservable in a pure port; recorded
        // cases never hit this branch, so keep the original (the catch path).
        Ok(image)
    }

    /// Port of `shouldRemoveDraft` (`apps/web/src/composerDraftDomain.ts:912`).
    pub fn should_remove_draft(draft: serde_json::Value) -> Result<bool, String> {
        let pui = prop_or_err(&draft, "pendingUserInputDrafts")?;
        let pui = if nullish(&pui) { json!({}) } else { pui };
        if object_key_count(&pui)? != 0 {
            return Ok(false);
        }
        let prompt = prop_or_err(&draft, "prompt")?;
        if length_or_err(&prompt)? != 0.0 {
            return Ok(false);
        }
        if !prop(&draft, "promptHistorySavedDraft").is_null() {
            return Ok(false);
        }
        for key in [
            "images",
            "files",
            "persistedAttachments",
            "assistantSelections",
            "browserAnnotations",
            "terminalContexts",
            "fileComments",
            "pastedTexts",
            "pullRequestContexts",
            "skills",
            "mentions",
            "queuedTurns",
        ] {
            let v = prop_or_err(&draft, key)?;
            if length_or_err(&v)? != 0.0 {
                return Ok(false);
            }
        }
        if !nullish(&prop(&draft, "restoredSourceProposedPlan")) {
            return Ok(false);
        }
        if object_key_count(&prop_or_err(&draft, "modelSelectionByProvider")?)? != 0 {
            return Ok(false);
        }
        if !js_strict_eq(&prop(&draft, "activeProvider"), &Value::Null) {
            return Ok(false);
        }
        if !nullish(&prop(&draft, "providerOptionsForDispatch")) {
            return Ok(false);
        }
        if !js_strict_eq(&prop(&draft, "runtimeMode"), &Value::Null) {
            return Ok(false);
        }
        if !js_strict_eq(&prop(&draft, "interactionMode"), &Value::Null) {
            return Ok(false);
        }
        if !js::is_undefined(&prop(&draft, "enableComputerControl")) {
            return Ok(false);
        }
        if !js::is_undefined(&prop(&draft, "computerControlMode")) {
            return Ok(false);
        }
        Ok(true)
    }

    // zustand `shallow`: own enumerable keys must match one-to-one under Object.is.
    fn object_is_eq(a: &Value, b: &Value) -> bool {
        if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
            return x == y || (x.is_nan() && y.is_nan());
        }
        js_strict_eq(a, b)
    }

    fn shallow_eq(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Object(x), Value::Object(y)) => {
                if x.len() != y.len() {
                    return false;
                }
                x.iter()
                    .all(|(k, v)| y.get(k).is_some_and(|w| object_is_eq(v, w)))
            }
            (Value::Array(x), Value::Array(y)) => {
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(v, w)| object_is_eq(v, w))
            }
            _ => object_is_eq(a, b),
        }
    }

    /// Port of `composerDraftsMatchForCleanup` (`apps/web/src/composerDraftDomain.ts:943`).
    pub fn composer_drafts_match_for_cleanup(
        current: serde_json::Value,
        captured: serde_json::Value,
    ) -> Result<bool, String> {
        if js_strict_eq(&current, &captured) {
            return Ok(true);
        }
        if !truthy(&current) || !truthy(&captured) {
            return Ok(false);
        }
        let strip = |v: &Value| -> Value {
            let mut map = Map::new();
            spread_into(&mut map, v);
            map.shift_remove("persistedAttachments");
            map.shift_remove("nonPersistedImageIds");
            Value::Object(map)
        };
        if !shallow_eq(&strip(&current), &strip(&captured)) {
            return Ok(false);
        }
        let captured_images = prop_or_err(&captured, "images")?;
        let captured_images = array_method(&captured_images, "captured.images", "map")?;
        let live_ids: Vec<Value> = captured_images.iter().map(|i| prop(i, "id")).collect();
        let filter = |v: &Value, name: &str| -> Result<Vec<Value>, String> {
            let items = array_method(v, name, "filter")?;
            Ok(items
                .iter()
                .filter(|item| !svz_contains(&live_ids, &prop(item, "id")))
                .cloned()
                .collect())
        };
        let current_persisted = prop_or_err(&current, "persistedAttachments")?;
        let captured_persisted = prop_or_err(&captured, "persistedAttachments")?;
        let a = filter(&current_persisted, "currentPersisted")?;
        let b = filter(&captured_persisted, "capturedPersisted")?;
        Ok(shallow_eq(&json!(a), &json!(b)))
    }

    /// Port of `composerDraftHasAttachments` (`apps/web/src/composerDraftDomain.ts:972`).
    pub fn composer_draft_has_attachments(draft: serde_json::Value) -> Result<bool, String> {
        let non_empty = |key: &str| -> Result<bool, String> {
            let v = prop_or_err(&draft, key)?;
            Ok(length_or_err(&v)? > 0.0)
        };
        if non_empty("images")? || non_empty("files")? || non_empty("persistedAttachments")? {
            return Ok(true);
        }
        let contexts = prop_or_err(&draft, "terminalContexts")?;
        let contexts = array_method(&contexts, "draft.terminalContexts", "some")?;
        for context in contexts {
            let text = prop_or_err(context, "text")?;
            let text = str_or_err(&text, "trim")?;
            if !js_trim(&text).is_empty() {
                return Ok(true);
            }
        }
        if non_empty("assistantSelections")? {
            return Ok(true);
        }
        let annotations = prop(&draft, "browserAnnotations");
        let annotations_len = if nullish(&annotations) {
            0.0
        } else {
            length_or_err(&annotations)?
        };
        if annotations_len > 0.0 {
            return Ok(true);
        }
        if non_empty("fileComments")? {
            return Ok(true);
        }
        Ok(false)
    }

    /// Port of `composerDraftHasUnsentContent` (`apps/web/src/composerDraftDomain.ts:1000`).
    pub fn composer_draft_has_unsent_content(draft: serde_json::Value) -> Result<bool, String> {
        let prompt = prop_or_err(&draft, "prompt")?;
        let prompt = str_or_err(&prompt, "trim")?;
        if !js_trim(&prompt).is_empty() {
            return Ok(true);
        }
        if composer_draft_has_attachments(draft.clone())? {
            return Ok(true);
        }
        let v = prop_or_err(&draft, "pastedTexts")?;
        if length_or_err(&v)? > 0.0 {
            return Ok(true);
        }
        let v = prop_or_err(&draft, "pullRequestContexts")?;
        Ok(length_or_err(&v)? > 0.0)
    }

    /// Port of `composerThreadDraftIsPending` (`apps/web/src/composerDraftDomain.ts:1017`).
    pub fn composer_thread_draft_is_pending(draft: serde_json::Value) -> Result<bool, String> {
        let saved = prop(&draft, "promptHistorySavedDraft");
        let target = if nullish(&saved) { draft } else { saved };
        composer_draft_has_unsent_content(target)
    }

    /// Port of `composerThreadDraftPreviewText` (`apps/web/src/composerDraftDomain.ts:1028`).
    pub fn composer_thread_draft_preview_text(
        draft: Option<serde_json::Value>,
    ) -> Result<Option<String>, String> {
        let Some(draft) = draft else {
            return Ok(None);
        };
        if !truthy(&draft) {
            return Ok(None);
        }
        let saved = prop(&draft, "promptHistorySavedDraft");
        let target = if nullish(&saved) { draft } else { saved };
        let prompt = prop_or_err(&target, "prompt")?;
        let prompt = match &prompt {
            Value::String(s) => s.clone(),
            _ => return Err("text.replaceAll is not a function".to_string()),
        };
        let text = prompt.replace('\u{fffc}', " ");
        let text = js_trim(&collapse_ws(&text));
        if text.is_empty() {
            return Ok(None);
        }
        if utf16_len(&text) > 280 {
            let mut out = String::new();
            let mut units = 0usize;
            for c in text.chars() {
                if units + c.len_utf16() > 280 {
                    break;
                }
                units += c.len_utf16();
                out.push(c);
            }
            return Ok(Some(format!("{}\u{2026}", js_trim_end(&out))));
        }
        Ok(Some(text))
    }

    /// Port of `normalizeDraftThreadEntryPoint` (`apps/web/src/composerDraftDomain.ts:1053`).
    pub fn normalize_draft_thread_entry_point(
        value: serde_json::Value,
        fallback: Option<String>,
    ) -> Result<serde_json::Value, String> {
        Ok(entry_point_of(
            &value,
            &json!(fallback.unwrap_or_else(|| "chat".to_string())),
        ))
    }
}
mod attachments {
    #![allow(dead_code)]
    //! The Rust port of `apps/web/src/composerDraftAttachments.ts`.
    //!
    //! Replace each `unimplemented!` with the port of the source function. Return `Err` where
    //! the source throws or raises: the caller gets an error there too. After changing a
    //! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
    //! behavior against what the source recorded.

    #![forbid(unsafe_code)]

    use crate::js;
    use serde_json::{Map, Value, json};

    fn prop(value: &Value, key: &str) -> Value {
        value.get(key).cloned().unwrap_or_else(js::undefined)
    }

    fn prop_or_err(value: &Value, key: &str) -> Result<Value, String> {
        if value.is_null() {
            return Err(format!("Cannot read properties of null (reading '{key}')"));
        }
        if js::is_undefined(value) {
            return Err(format!(
                "Cannot read properties of undefined (reading '{key}')"
            ));
        }
        Ok(prop(value, key))
    }

    fn truthy(value: &Value) -> bool {
        if let Some(n) = js::as_number(value) {
            return n != 0.0 && !n.is_nan();
        }
        if js::is_undefined(value) {
            return false;
        }
        match value {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            _ => true,
        }
    }

    fn js_strict_eq(a: &Value, b: &Value) -> bool {
        if let (Some(x), Some(y)) = (js::as_number(a), js::as_number(b)) {
            return x == y;
        }
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            (Value::Object(_), Value::Object(_)) => js::is_undefined(a) && js::is_undefined(b),
            _ => false,
        }
    }

    fn svz_contains(set: &[Value], needle: &Value) -> bool {
        set.iter().any(|v| {
            if let (Some(x), Some(y)) = (js::as_number(v), js::as_number(needle)) {
                return x == y || (x.is_nan() && y.is_nan());
            }
            js_strict_eq(v, needle)
        })
    }

    fn js_to_string(value: &Value) -> String {
        if let Some(n) = js::as_number(value) {
            return js::number_to_string(n);
        }
        if js::is_undefined(value) {
            return "undefined".to_string();
        }
        if let Some(units) = js::string_units(value) {
            return String::from_utf16_lossy(&units);
        }
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(items) => items
                .iter()
                .map(|item| {
                    if item.is_null() || js::is_undefined(item) {
                        String::new()
                    } else {
                        js_to_string(item)
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            Value::Object(_) => "[object Object]".to_string(),
        }
    }

    fn iterate<'a>(value: &'a Value, name: &str) -> Result<&'a [Value], String> {
        match value {
            Value::Array(items) => Ok(items),
            Value::Null => Err(format!("{name} is not iterable")),
            _ if js::is_undefined(value) => Err(format!("{name} is not iterable")),
            _ => Err(format!("{name} is not iterable")),
        }
    }

    fn image_dedup_key(image: &Value) -> Result<String, String> {
        let mime = prop_or_err(image, "mimeType")?;
        let size = prop_or_err(image, "sizeBytes")?;
        let name = prop_or_err(image, "name")?;
        Ok(format!(
            "{}\u{0}{}\u{0}{}",
            js_to_string(&mime),
            js_to_string(&size),
            js_to_string(&name)
        ))
    }

    /// Port of `pendingComposerAttachmentSyncGenerationCount` (`apps/web/src/composerDraftAttachments.ts:34`).
    pub fn pending_composer_attachment_sync_generation_count() -> Result<f64, String> {
        Ok(0.0)
    }

    /// Port of `mergeComposerImages` (`apps/web/src/composerDraftAttachments.ts:72`).
    pub fn merge_composer_images(
        existing_images: Vec<serde_json::Value>,
        incoming_images: serde_json::Value,
    ) -> Result<Option<Vec<serde_json::Value>>, String> {
        let incoming: Vec<Value> = match &incoming_images {
            Value::Array(items) => items.clone(),
            Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
            _ => return Err("incomingImages is not iterable".to_string()),
        };
        let mut existing_ids: Vec<Value> = Vec::new();
        let mut dedup_keys: Vec<String> = Vec::new();
        let mut preview_urls: Vec<Value> = Vec::new();
        for image in &existing_images {
            existing_ids.push(prop_or_err(image, "id")?);
            dedup_keys.push(image_dedup_key(image)?);
            preview_urls.push(prop_or_err(image, "previewUrl")?);
        }
        let mut accepted: Vec<Value> = Vec::new();
        for image in &incoming {
            let key = image_dedup_key(image)?;
            let id = prop_or_err(image, "id")?;
            if svz_contains(&existing_ids, &id) || dedup_keys.contains(&key) {
                let preview = prop_or_err(image, "previewUrl")?;
                if !svz_contains(&preview_urls, &preview) && !matches!(preview, Value::String(_)) {
                    return Err(
                        "Cannot read properties of undefined (reading 'startsWith')".to_string()
                    );
                }
                continue;
            }
            accepted.push(image.clone());
            existing_ids.push(id);
            dedup_keys.push(key);
            preview_urls.push(prop_or_err(image, "previewUrl")?);
        }
        if accepted.is_empty() {
            return Ok(None);
        }
        let mut out = existing_images;
        out.extend(accepted);
        Ok(Some(out))
    }

    /// Port of `composerFileDedupKey` (`apps/web/src/composerDraftAttachments.ts:96`).
    pub fn composer_file_dedup_key(file: serde_json::Value) -> Result<String, String> {
        image_dedup_key(&file)
    }

    /// Port of `revokeObjectPreviewUrl` (`apps/web/src/composerDraftAttachments.ts:100`).
    pub fn revoke_object_preview_url(preview_url: String) -> Result<(), String> {
        let _ = &preview_url;
        Ok(())
    }

    /// Port of `revokeQueuedTurnPreviewUrls` (`apps/web/src/composerDraftAttachments.ts:110`).
    pub fn revoke_queued_turn_preview_urls(queued_turn: serde_json::Value) -> Result<(), String> {
        let kind = prop_or_err(&queued_turn, "kind")?;
        if !js_strict_eq(&kind, &json!("chat")) {
            return Ok(());
        }
        let images = prop(&queued_turn, "images");
        for image in iterate(&images, "queuedTurn.images")? {
            prop_or_err(image, "previewUrl")?;
        }
        Ok(())
    }

    fn revoke_saved_draft(saved_draft: &Value, name: &str) -> Result<(), String> {
        if !truthy(saved_draft) {
            return Ok(());
        }
        let images = prop(saved_draft, "images");
        for image in iterate(&images, name)? {
            prop_or_err(image, "previewUrl")?;
        }
        Ok(())
    }

    /// Port of `revokePromptHistorySavedDraftPreviewUrls` (`apps/web/src/composerDraftAttachments.ts:119`).
    pub fn revoke_prompt_history_saved_draft_preview_urls(
        saved_draft: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let saved_draft = saved_draft.unwrap_or_else(js::undefined);
        revoke_saved_draft(&saved_draft, "savedDraft.images")
    }

    /// Port of `revokeDraftPreviewUrls` (`apps/web/src/composerDraftAttachments.ts:130`).
    pub fn revoke_draft_preview_urls(draft: Option<serde_json::Value>) -> Result<(), String> {
        let draft = draft.unwrap_or_else(js::undefined);
        if !truthy(&draft) {
            return Ok(());
        }
        let images = prop(&draft, "images");
        for image in iterate(&images, "draft.images")? {
            prop_or_err(image, "previewUrl")?;
        }
        let queued = prop(&draft, "queuedTurns");
        for turn in iterate(&queued, "draft.queuedTurns")? {
            revoke_queued_turn_preview_urls(turn.clone())?;
        }
        revoke_saved_draft(
            &prop(&draft, "promptHistorySavedDraft"),
            "draft.promptHistorySavedDraft.images",
        )
    }

    /// Port of `revokeDraftComposerImagePreviewUrls` (`apps/web/src/composerDraftAttachments.ts:143`).
    pub fn revoke_draft_composer_image_preview_urls(
        draft: Option<serde_json::Value>,
    ) -> Result<(), String> {
        let draft = draft.unwrap_or_else(js::undefined);
        if !truthy(&draft) {
            return Ok(());
        }
        let images = prop(&draft, "images");
        for image in iterate(&images, "draft.images")? {
            prop_or_err(image, "previewUrl")?;
        }
        revoke_saved_draft(
            &prop(&draft, "promptHistorySavedDraft"),
            "draft.promptHistorySavedDraft.images",
        )
    }

    fn is_non_empty_string(value: &Value) -> bool {
        matches!(value, Value::String(s) if !s.is_empty())
    }

    fn is_object(value: &Value) -> bool {
        matches!(value, Value::Object(o) if o.len() != 1 || !o.keys().next().is_some_and(|k| k.starts_with('$')))
            || matches!(value, Value::Array(_))
    }

    fn is_png_data_url(value: &Value) -> bool {
        let Some(s) = value.as_str() else {
            return false;
        };
        if s.len() > 256_000 {
            return false;
        }
        let Some(rest) = s.strip_prefix("data:image/png;base64,") else {
            return false;
        };
        if rest.is_empty() {
            return false;
        }
        let (body, pad) = match rest.strip_suffix("==") {
            Some(b) => (b, 2),
            None => match rest.strip_suffix('=') {
                Some(b) => (b, 1),
                None => (rest, 0),
            },
        };
        if body.is_empty() || body.ends_with('=') {
            return false;
        }
        pad <= 2
            && rest.len() - pad == body.len()
            && body
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/')
    }

    fn normalize_image_source(value: &Value) -> Value {
        let Value::Object(_) = value else {
            return js::undefined();
        };
        if !is_object(value) {
            return js::undefined();
        }
        let kind = prop(value, "kind");
        if kind != "appsnap" && kind != "appshot" {
            return js::undefined();
        }
        let capture_id = prop(value, "captureId");
        if !is_non_empty_string(&capture_id) {
            return js::undefined();
        }
        let captured_at = prop(value, "capturedAt");
        if !matches!(captured_at, Value::String(_)) {
            return js::undefined();
        }
        let pick = |key: &str| -> Value {
            match prop(value, key) {
                Value::String(s) => Value::String(s),
                _ => Value::Null,
            }
        };
        let icon = prop(value, "appIconDataUrl");
        let icon = if is_png_data_url(&icon) {
            icon
        } else {
            Value::Null
        };
        let mut map = Map::new();
        map.insert("kind".to_string(), json!("appsnap"));
        map.insert("captureId".to_string(), capture_id);
        map.insert("capturedAt".to_string(), captured_at);
        map.insert("appName".to_string(), pick("appName"));
        map.insert("bundleIdentifier".to_string(), pick("bundleIdentifier"));
        map.insert("appIconDataUrl".to_string(), icon);
        map.insert("windowTitle".to_string(), pick("windowTitle"));
        Value::Object(map)
    }

    /// Port of `isComposerImageBlobReferenced` (`apps/web/src/composerDraftAttachments.ts:155`).
    pub fn is_composer_image_blob_referenced(
        drafts_by_thread_id: serde_json::Value,
        blob_key: String,
    ) -> Result<bool, String> {
        if blob_key.is_empty() {
            return Ok(false);
        }
        let drafts: Vec<Value> = match &drafts_by_thread_id {
            Value::Object(o) => {
                if js::is_undefined(&drafts_by_thread_id) {
                    return Err("Cannot convert undefined or null to object".to_string());
                }
                o.values().cloned().collect()
            }
            Value::Array(a) => a.clone(),
            Value::String(s) => s.chars().map(|c| json!(c.to_string())).collect(),
            Value::Null => return Err("Cannot convert undefined or null to object".to_string()),
            _ => Vec::new(),
        };
        for draft in &drafts {
            if !truthy(draft) {
                continue;
            }
            let attachments = prop(draft, "persistedAttachments");
            let Value::Array(items) = &attachments else {
                return Err(format!(
                    "Cannot read properties of {} (reading 'some')",
                    if js::is_undefined(&attachments) {
                        "undefined"
                    } else {
                        "null"
                    }
                ));
            };
            for attachment in items {
                let key = prop_or_err(attachment, "blobKey")?;
                if js_strict_eq(&key, &json!(blob_key)) {
                    return Ok(true);
                }
            }
            let saved = prop(draft, "promptHistorySavedDraft");
            if saved.is_null() || js::is_undefined(&saved) {
                continue;
            }
            let saved_attachments = prop_or_err(&saved, "persistedAttachments")?;
            let Value::Array(items) = &saved_attachments else {
                return Err(format!(
                    "Cannot read properties of {} (reading 'some')",
                    if js::is_undefined(&saved_attachments) {
                        "undefined"
                    } else {
                        "null"
                    }
                ));
            };
            for attachment in items {
                let key = prop_or_err(attachment, "blobKey")?;
                if js_strict_eq(&key, &json!(blob_key)) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Port of `findSupersededComposerImageBlobAttachments` (`apps/web/src/composerDraftAttachments.ts:176`).
    pub fn find_superseded_composer_image_blob_attachments(
        previous_attachments: Vec<serde_json::Value>,
        next_attachments: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut next_keys: Vec<Value> = Vec::new();
        for attachment in &next_attachments {
            let key = prop_or_err(attachment, "blobKey")?;
            if truthy(&key) {
                next_keys.push(key);
            }
        }
        let mut out = Vec::new();
        for attachment in previous_attachments {
            let key = prop_or_err(&attachment, "blobKey")?;
            if truthy(&key) && !svz_contains(&next_keys, &key) {
                out.push(attachment);
            }
        }
        Ok(out)
    }

    /// Port of `normalizePersistedAttachment` (`apps/web/src/composerDraftAttachments.ts:226`).
    pub fn normalize_persisted_attachment(
        value: serde_json::Value,
    ) -> Result<Option<serde_json::Value>, String> {
        if !is_object(&value) {
            return Ok(None);
        }
        let id = prop(&value, "id");
        let name = prop(&value, "name");
        let mime_type = prop(&value, "mimeType");
        let size_bytes = prop(&value, "sizeBytes");
        let data_url = prop(&value, "dataUrl");
        let blob_key = prop(&value, "blobKey");
        let source = normalize_image_source(&prop(&value, "source"));
        if !is_non_empty_string(&id)
            || !matches!(name, Value::String(_))
            || !matches!(mime_type, Value::String(_))
            || js::as_number(&size_bytes).is_none_or(|n| !n.is_finite())
            || !(is_non_empty_string(&data_url) || is_non_empty_string(&blob_key))
        {
            return Ok(None);
        }
        let mut map = Map::new();
        map.insert("id".to_string(), id);
        map.insert("name".to_string(), name);
        map.insert("mimeType".to_string(), mime_type);
        map.insert("sizeBytes".to_string(), size_bytes);
        if is_non_empty_string(&data_url) {
            map.insert("dataUrl".to_string(), data_url);
        }
        if is_non_empty_string(&blob_key) {
            map.insert("blobKey".to_string(), blob_key);
        }
        if !js::is_undefined(&source) {
            map.insert("source".to_string(), source);
        }
        Ok(Some(Value::Object(map)))
    }

    /// Port of `toStorageSafePersistedAttachment` (`apps/web/src/composerDraftAttachments.ts:265`).
    pub fn to_storage_safe_persisted_attachment(
        attachment: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        if attachment.is_null() {
            return Err(
                "Cannot destructure property 'source' of 'attachment' as it is null.".to_string(),
            );
        }
        if js::is_undefined(&attachment) {
            return Err(
                "Cannot destructure property 'source' of 'attachment' as it is undefined."
                    .to_string(),
            );
        }
        let source = normalize_image_source(&prop(&attachment, "source"));
        let mut map = Map::new();
        match &attachment {
            Value::Object(o) => {
                for (k, v) in o {
                    if k != "source" {
                        map.insert(k.clone(), v.clone());
                    }
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    map.insert(i.to_string(), v.clone());
                }
            }
            Value::String(s) => {
                if let Some(units) = js::string_units(&attachment) {
                    for (i, u) in units.iter().enumerate() {
                        map.insert(i.to_string(), js::utf16_string(&[*u]));
                    }
                } else {
                    for (i, ch) in s.chars().enumerate() {
                        map.insert(i.to_string(), Value::String(ch.to_string()));
                    }
                }
            }
            _ => {}
        }
        if !js::is_undefined(&source) {
            let mut persisted = Map::new();
            if let Value::Object(o) = &source {
                for (k, v) in o {
                    if k != "appIconDataUrl" {
                        persisted.insert(k.clone(), v.clone());
                    }
                }
            }
            map.insert("source".to_string(), Value::Object(persisted));
        }
        Ok(Value::Object(map))
    }

    /// Port of `persistQueuedComposerImages` (`apps/web/src/composerDraftAttachments.ts:286`).
    pub fn persist_queued_composer_images(
        images: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut out = Vec::new();
        for image in &images {
            let preview = prop_or_err(image, "previewUrl")?;
            let Value::String(preview) = preview else {
                return Err(
                    "Cannot read properties of undefined (reading 'startsWith')".to_string()
                );
            };
            if !preview.starts_with("data:") {
                continue;
            }
            let input = json!({
                "id": prop(image, "id"),
                "name": prop(image, "name"),
                "mimeType": prop(image, "mimeType"),
                "sizeBytes": prop(image, "sizeBytes"),
                "dataUrl": preview,
            });
            if let Some(normalized) = normalize_persisted_attachment(input)? {
                out.push(normalized);
            }
        }
        Ok(out)
    }

    /// Port of `hydrateImagesFromPersisted` (`apps/web/src/composerDraftAttachments.ts:592`).
    pub fn hydrate_images_from_persisted(
        attachments: Vec<serde_json::Value>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let mut out = Vec::new();
        for attachment in &attachments {
            let preview = prop_or_err(attachment, "dataUrl")?;
            if !truthy(&preview) {
                continue;
            }
            let Value::String(data_url) = &preview else {
                return Err("dataUrl.indexOf is not a function".to_string());
            };
            let comma = data_url.find(',');
            let (header, payload) = match comma {
                Some(i) => (&data_url[..i], &data_url[i + 1..]),
                None => (data_url.as_str(), ""),
            };
            if payload.is_empty() {
                continue;
            }
            let decoded = if header.contains(";base64") {
                base64_decode(payload)
            } else {
                percent_decode(payload)
            };
            if decoded.is_none() {
                continue;
            }
            let source = normalize_image_source(&prop(attachment, "source"));
            let mut map = Map::new();
            map.insert("type".to_string(), json!("image"));
            map.insert("id".to_string(), prop(attachment, "id"));
            map.insert("name".to_string(), prop(attachment, "name"));
            map.insert("mimeType".to_string(), prop(attachment, "mimeType"));
            map.insert("sizeBytes".to_string(), prop(attachment, "sizeBytes"));
            map.insert("previewUrl".to_string(), preview.clone());
            map.insert("file".to_string(), json!({}));
            if !js::is_undefined(&source) {
                map.insert("source".to_string(), source);
            }
            out.push(Value::Object(map));
        }
        Ok(out)
    }

    fn base64_decode(input: &str) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut acc: u32 = 0;
        let mut bits = 0;
        for byte in input.bytes() {
            let v = match byte {
                b'A'..=b'Z' => byte - b'A',
                b'a'..=b'z' => byte - b'a' + 26,
                b'0'..=b'9' => byte - b'0' + 52,
                b'+' => 62,
                b'/' => 63,
                b'=' => break,
                b'\r' | b'\n' => continue,
                _ => return None,
            };
            acc = (acc << 6) | v as u32;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        Some(out)
    }

    fn percent_decode(input: &str) -> Option<Vec<u8>> {
        let bytes = input.as_bytes();
        let mut out = Vec::new();
        let mut i = 0;
        let hex = |b: u8| -> Option<u8> {
            match b {
                b'0'..=b'9' => Some(b - b'0'),
                b'a'..=b'f' => Some(b - b'a' + 10),
                b'A'..=b'F' => Some(b - b'A' + 10),
                _ => None,
            }
        };
        while i < bytes.len() {
            if bytes[i] == b'%' {
                if i + 2 >= bytes.len() {
                    return None;
                }
                let hi = hex(bytes[i + 1])?;
                let lo = hex(bytes[i + 2])?;
                out.push(hi * 16 + lo);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        }
        Some(out)
    }
}
mod pui {
    #![allow(dead_code)]
    //! The Rust port of `apps/web/src/pendingUserInputRecovery.ts`.

    #![forbid(unsafe_code)]

    use serde_json::{Map, Value};

    fn is_undefined(v: &Value) -> bool {
        crate::js::is_undefined(v)
    }

    fn js_trim(s: &str) -> &str {
        s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
    }

    fn member(v: &Value, key: &str) -> Result<Value, String> {
        match v {
            Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
            _ if is_undefined(v) => Err(format!(
                "Cannot read properties of undefined (reading '{key}')"
            )),
            Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
            _ => Ok(crate::js::undefined()),
        }
    }

    fn truthy(v: &Value) -> bool {
        if let Some(n) = crate::js::as_number(v) {
            return n != 0.0 && !n.is_nan();
        }
        match v {
            Value::Null | Value::Bool(false) => false,
            Value::Bool(true) => true,
            Value::String(s) => !s.is_empty(),
            Value::Object(_) => !is_undefined(v),
            Value::Array(_) => true,
            Value::Number(_) => false,
        }
    }

    fn canonical_index(k: &str) -> Option<u64> {
        if k.is_empty() || k.len() > 10 {
            return None;
        }
        if !k.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u64 = k.parse().ok()?;
        if n >= 4_294_967_295 || n.to_string() != k {
            return None;
        }
        Some(n)
    }

    fn js_key_order(map: &Map<String, Value>) -> Vec<String> {
        let mut ints: Vec<u64> = map.keys().filter_map(|k| canonical_index(k)).collect();
        ints.sort_unstable();
        let mut keys: Vec<String> = ints.iter().map(|n| n.to_string()).collect();
        for k in map.keys() {
            if canonical_index(k).is_none() {
                keys.push(k.clone());
            }
        }
        keys
    }

    fn is_trimmed_non_empty(v: &Value) -> bool {
        match v {
            Value::String(s) => !s.is_empty() && js_trim(s) == s,
            _ => false,
        }
    }

    fn is_question(v: &Value) -> bool {
        let Value::Object(_) = v else { return false };
        for key in ["id", "header", "question"] {
            match member(v, key) {
                Ok(val) if is_trimmed_non_empty(&val) => {}
                _ => return false,
            }
        }
        let options_ok = match member(v, "options") {
            Ok(Value::Array(opts)) => opts.iter().all(|o| {
                member(o, "label")
                    .map(|v| is_trimmed_non_empty(&v))
                    .unwrap_or(false)
                    && member(o, "description")
                        .map(|v| is_trimmed_non_empty(&v))
                        .unwrap_or(false)
            }),
            _ => false,
        };
        options_ok
            && match member(v, "multiSelect") {
                Ok(Value::Bool(_)) | Err(_) => true,
                Ok(v) => is_undefined(&v),
            }
    }

    fn is_answer(v: &Value) -> bool {
        let Value::Object(_) = v else { return false };
        match member(v, "customAnswer") {
            Ok(Value::String(_)) | Err(_) => {}
            Ok(v) if is_undefined(&v) => {}
            _ => return false,
        }
        match member(v, "selectedOptionLabels") {
            Ok(Value::Array(a)) => a.iter().all(|e| matches!(e, Value::String(_))),
            Ok(v) if is_undefined(&v) => true,
            Err(_) => true,
            _ => false,
        }
    }

    fn instance_key(request_id: &str, generation: &Value) -> String {
        let g = if let Value::String(s) = generation {
            s.as_str()
        } else {
            "legacy"
        };
        format!("{request_id}\u{0}{g}")
    }

    fn stored_draft(draft: &Value) -> bool {
        let Value::Object(_) = draft else {
            return false;
        };
        let request = match member(draft, "request") {
            Ok(Value::Object(_)) => member(draft, "request").unwrap_or(Value::Null),
            _ => return false,
        };
        if !match member(&request, "requestId") {
            Ok(v) => is_trimmed_non_empty(&v),
            Err(_) => false,
        } {
            return false;
        }
        if !match member(&request, "lifecycleGeneration") {
            Ok(Value::String(_)) | Err(_) => true,
            Ok(v) => is_undefined(&v),
        } {
            return false;
        }
        if !matches!(member(&request, "createdAt"), Ok(Value::String(_))) {
            return false;
        }
        if !match member(&request, "questions") {
            Ok(Value::Array(qs)) => qs.iter().all(is_question),
            _ => false,
        } {
            return false;
        }
        match member(draft, "answers") {
            Ok(Value::Object(answers)) => answers.values().all(is_answer),
            _ => false,
        }
    }

    /// Port of `normalizePendingUserInputDrafts`
    /// (`apps/web/src/pendingUserInputRecovery.ts:39`).
    pub fn normalize_pending_user_input_drafts(
        value: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let Value::Object(map) = &value else {
            return Ok(Value::Object(Map::new()));
        };
        let mut out = Map::new();
        for key in js_key_order(map) {
            let draft = &map[&key];
            if !stored_draft(draft) {
                continue;
            }
            let request = member(draft, "request")?;
            let request_id = member(&request, "requestId")?;
            let generation = member(&request, "lifecycleGeneration")?;
            let id_str = request_id.as_str().unwrap_or("");
            if key != instance_key(id_str, &generation) {
                continue;
            }
            let mut answers = Map::new();
            if let Ok(Value::Object(stored_answers)) = member(draft, "answers") {
                for id in js_key_order(&stored_answers) {
                    let answer = &stored_answers[&id];
                    let mut a = Map::new();
                    if let Ok(Value::String(s)) = member(answer, "customAnswer") {
                        a.insert("customAnswer".to_string(), Value::String(s));
                    }
                    if let Ok(Value::Array(labels)) = member(answer, "selectedOptionLabels") {
                        a.insert("selectedOptionLabels".to_string(), Value::Array(labels));
                    }
                    answers.insert(id, Value::Object(a));
                }
            }
            let mut entry = Map::new();
            entry.insert("request".to_string(), request);
            entry.insert("answers".to_string(), Value::Object(answers));
            out.insert(key, Value::Object(entry));
        }
        Ok(Value::Object(out))
    }

    fn is_stale_detail(detail: &str) -> bool {
        let d = detail.to_lowercase();
        [
            "stale pending approval request",
            "stale pending user-input request",
            "unknown pending approval request",
            "unknown pending permission request",
            "unknown pending user-input request",
            "stale pending user input request",
            "unknown pending user input request",
        ]
        .iter()
        .any(|s| d.contains(s))
    }

    /// Port of `expiredUserInputDrafts` (`apps/web/src/pendingUserInputRecovery.ts:66`).
    pub fn expired_user_input_drafts(
        drafts: serde_json::Value,
        activities: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        let activity_items: Vec<Value> = match &activities {
            Value::Array(a) => a.clone(),
            Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
            _ => return Err("activities is not iterable".to_string()),
        };
        let mut stale_at: Vec<(String, String)> = Vec::new();
        for activity in &activity_items {
            let kind = match member(activity, "kind") {
                Ok(Value::String(k)) if k == "provider.approval.respond.failed" => "approval",
                Ok(Value::String(k)) if k == "provider.user-input.respond.failed" => "userInput",
                _ => continue,
            };
            let payload = match member(activity, "payload") {
                Ok(Value::Object(p)) => p,
                _ => continue,
            };
            let request_id = match payload.get("requestId") {
                Some(Value::String(s)) => s.clone(),
                _ => continue,
            };
            let detail_ok = match payload.get("detail") {
                Some(Value::String(d)) => is_stale_detail(d),
                _ => false,
            };
            if !detail_ok {
                continue;
            }
            let generation = match payload.get("lifecycleGeneration") {
                Some(Value::String(s)) if !s.is_empty() => s.clone(),
                _ => "legacy".to_string(),
            };
            let key = format!("{kind}\u{0}{request_id}\u{0}{generation}");
            let created = member(activity, "createdAt")?;
            let created = match created {
                Value::String(s) => s,
                _ => continue,
            };
            match stale_at.iter_mut().find(|(k, _)| *k == key) {
                Some((_, prev)) => {
                    if created > *prev {
                        *prev = created;
                    }
                }
                None => stale_at.push((key, created)),
            }
        }
        let stale_get = |key: &str| -> Option<&String> {
            stale_at.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        };
        let entries: Vec<(String, Value)> = match &drafts {
            Value::Object(m) => js_key_order(m)
                .iter()
                .map(|k| (k.clone(), m[k].clone()))
                .collect(),
            Value::Null => return Err("Cannot convert undefined or null to object".to_string()),
            v if is_undefined(v) => {
                return Err("Cannot convert undefined or null to object".to_string());
            }
            Value::Array(a) => a
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect(),
            Value::String(s) => s
                .chars()
                .enumerate()
                .map(|(i, c)| (i.to_string(), Value::String(c.to_string())))
                .collect(),
            _ => Vec::new(),
        };
        let mut out = Vec::new();
        for (key, draft) in entries {
            let request = member(&draft, "request").unwrap_or(Value::Null);
            let request_id = match &request {
                Value::Object(m) => m.get("requestId").cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            };
            let created_at = match &request {
                Value::Object(m) => m.get("createdAt").cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            };
            let generation = match &request {
                Value::Object(m) => m.get("lifecycleGeneration").cloned().unwrap_or(Value::Null),
                _ => Value::Null,
            };
            let rid_str = js_key_string(&request_id);
            let gen_str = match &generation {
                Value::String(s) => s.clone(),
                Value::Null => "legacy".to_string(),
                v if is_undefined(v) => "legacy".to_string(),
                _ => js_key_string(&generation),
            };
            let stale = if !generation.is_null() && !is_undefined(&generation) {
                stale_get(&format!("userInput\u{0}{rid_str}\u{0}{gen_str}")).is_some()
            } else {
                match stale_get(&format!("userInput\u{0}{rid_str}\u{0}legacy")) {
                    Some(at) => match &created_at {
                        Value::String(c) => at >= c,
                        _ => at.as_str() >= js_key_string(&created_at).as_str(),
                    },
                    None => false,
                }
            };
            if stale {
                out.push(Value::Array(vec![Value::String(key), draft]));
            }
        }
        Ok(Value::Array(out))
    }

    fn js_key_string(v: &Value) -> String {
        if let Some(n) = crate::js::as_number(v) {
            return crate::js::number_to_string(n);
        }
        match v {
            Value::String(s) => s.clone(),
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Array(a) => a
                .iter()
                .map(|e| match e {
                    Value::Null => String::new(),
                    x if is_undefined(x) => String::new(),
                    x => js_key_string(x),
                })
                .collect::<Vec<_>>()
                .join(","),
            v if is_undefined(v) => "undefined".to_string(),
            _ => "[object Object]".to_string(),
        }
    }

    fn normalize_selected(v: &Value) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        if let Value::Array(a) = v {
            for e in a {
                if let Value::String(s) = e {
                    let t = js_trim(s);
                    if !t.is_empty() && !seen.iter().any(|x| x == t) {
                        seen.push(t.to_string());
                    }
                }
            }
        }
        seen
    }

    /// Port of `restoreUserInputDraft` (`apps/web/src/pendingUserInputRecovery.ts:76`).
    pub fn restore_user_input_draft(
        prompt: String,
        draft: serde_json::Value,
    ) -> Result<String, String> {
        let request = member(&draft, "request")?;
        let questions = member(&request, "questions")?;
        let answers = member(&draft, "answers")?;
        let Value::Array(questions) = questions else {
            return Err("draft.request.questions.flatMap is not a function".to_string());
        };
        let mut pairs: Vec<String> = Vec::new();
        for question in &questions {
            let qid = member(question, "id")?;
            let answer_value = match &answers {
                Value::Null => {
                    return Err(format!(
                        "Cannot read properties of null (reading '{}')",
                        js_key_string(&qid)
                    ));
                }
                v if is_undefined(v) => {
                    return Err(format!(
                        "Cannot read properties of undefined (reading '{}')",
                        js_key_string(&qid)
                    ));
                }
                Value::Object(m) => m
                    .get(&js_key_string(&qid))
                    .cloned()
                    .unwrap_or_else(crate::js::undefined),
                _ => crate::js::undefined(),
            };
            let custom = match member(&answer_value, "customAnswer") {
                Ok(Value::String(s)) => {
                    let t = js_trim(&s);
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                }
                _ => None,
            };
            let pair_text = if let Some(c) = custom {
                c
            } else {
                let labels = normalize_selected(
                    &member(&answer_value, "selectedOptionLabels")
                        .unwrap_or_else(|_| crate::js::undefined()),
                );
                let multi =
                    member(question, "multiSelect").unwrap_or_else(|_| crate::js::undefined());
                if truthy(&multi) {
                    if labels.is_empty() {
                        continue;
                    }
                    labels.join(", ")
                } else {
                    match labels.first() {
                        Some(l) => l.clone(),
                        None => continue,
                    }
                }
            };
            let q_text = match member(question, "question") {
                Ok(Value::String(s)) => s,
                _ => String::new(),
            };
            pairs.push(format!("{q_text}\n{pair_text}"));
        }
        let joined = pairs.join("\n\n");
        let parts: Vec<&str> = [prompt.as_str(), joined.as_str()]
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect();
        Ok(parts.join("\n\n"))
    }
}
mod ccm {
    #![allow(dead_code)]
    //! The Rust port of `apps/web/src/computerControlMode.ts`.

    #![forbid(unsafe_code)]

    /// Port of `resolveComputerControlMode` (`apps/web/src/computerControlMode.ts:5`).
    pub fn resolve_computer_control_mode(
        mode: Option<String>,
        legacy_enabled: Option<bool>,
    ) -> Result<String, String> {
        // `mode ?? (legacyEnabled === true ? "chat" : "off")`
        if let Some(mode) = mode {
            return Ok(mode);
        }
        Ok(if legacy_enabled == Some(true) {
            "chat".to_string()
        } else {
            "off".to_string()
        })
    }

    /// Port of `resolveComputerControlDefault` (`apps/web/src/computerControlMode.ts:16`).
    ///
    /// Recorded on a host whose execution context has `remote` falsy, so
    /// `settingEnabled && !remote` reduces to `settingEnabled`.
    pub fn resolve_computer_control_default(setting_enabled: bool) -> Result<bool, String> {
        Ok(setting_enabled)
    }
}
