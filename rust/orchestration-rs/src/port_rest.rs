// ---------------------------------------------------------------- exportThreadArchive.ts
fn slugify_title(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut last_dash = false;
    for c in title.to_lowercase().trim().chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    let slug: String = slug.chars().take(48).collect();
    if slug.is_empty() {
        "thread".into()
    } else {
        slug
    }
}

pub fn thread_archive_file_name(input: Value) -> Result<String, String> {
    let iso = str_of(&prop(&input, "isoTimestamp"));
    let date: String = iso.chars().take(10).filter(|c| *c != '-').collect();
    Ok(format!(
        "synara-thread-{}-{}.zip",
        slugify_title(&str_of(&prop(&input, "title"))),
        date
    ))
}

// ---------------------------------------------------------------- handoff.ts
const RECENT_MESSAGE_COUNT: usize = 6;
const EARLIER_MESSAGE_CHAR_LIMIT: usize = 320;
const RECENT_MESSAGE_CHAR_LIMIT: usize = 2400;
const BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET: f64 = 32_000.0;

fn imported_messages(thread: &Value, source: &str) -> Vec<Value> {
    arr(&prop(thread, "messages"))
        .into_iter()
        .filter(|m| {
            str_of(&prop(m, "source")) == source
                && matches!(str_of(&prop(m, "role")).as_str(), "user" | "assistant")
                && prop(m, "streaming") == Value::Bool(false)
        })
        .collect()
}

pub fn list_imported_handoff_messages(thread: Value) -> Result<Vec<Value>, String> {
    Ok(imported_messages(&thread, "handoff-import"))
}
pub fn list_imported_fork_messages(thread: Value) -> Result<Vec<Value>, String> {
    Ok(imported_messages(&thread, "fork-import"))
}
pub fn has_native_handoff_messages(thread: Value) -> Result<bool, String> {
    Ok(arr(&prop(&thread, "messages")).iter().any(|m| {
        matches!(str_of(&prop(m, "role")).as_str(), "user" | "assistant")
            && matches!(
                str_of(&prop(m, "source")).as_str(),
                "native" | "async-user-input"
            )
            && prop(m, "streaming") == Value::Bool(false)
    }))
}
fn msg_index_before(thread: &Value, current: &Option<String>) -> i64 {
    match current {
        None => arr(&prop(thread, "messages")).len() as i64,
        Some(id) => arr(&prop(thread, "messages"))
            .iter()
            .position(|m| str_of(&prop(m, "id")) == *id)
            .map(|i| i as i64)
            .unwrap_or(-1),
    }
}
pub fn has_native_assistant_messages_before(
    thread: Value,
    current_message_id: Option<String>,
) -> Result<bool, String> {
    let messages = arr(&prop(&thread, "messages"));
    let idx = msg_index_before(&thread, &current_message_id);
    if idx <= 0 {
        return Ok(false);
    }
    Ok(messages[..idx as usize].iter().any(|m| {
        str_of(&prop(m, "role")) == "assistant"
            && str_of(&prop(m, "source")) == "native"
            && prop(m, "streaming") == Value::Bool(false)
    }))
}
pub fn list_prior_transcript_messages(
    thread: Value,
    current_message_id: Option<String>,
) -> Result<Vec<Value>, String> {
    let messages = arr(&prop(&thread, "messages"));
    let idx = msg_index_before(&thread, &current_message_id);
    if idx <= 0 {
        return Ok(vec![]);
    }
    Ok(messages[..idx as usize]
        .iter()
        .filter(|m| {
            matches!(str_of(&prop(m, "role")).as_str(), "user" | "assistant")
                && prop(m, "streaming") == Value::Bool(false)
                && !str_of(&prop(m, "text")).trim().is_empty()
        })
        .cloned()
        .collect())
}

fn normalize_message_text(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut newline_run = 0usize;
    let mut pending_ws = String::new();
    for c in v.trim().chars() {
        if c == '\n' {
            pending_ws.clear();
            newline_run += 1;
            if newline_run >= 3 && out.ends_with("\n\n") {
                continue;
            }
            out.push('\n');
        } else if c.is_whitespace() {
            if newline_run > 0 {
                newline_run = 0;
            }
            pending_ws.push(c);
        } else {
            if newline_run > 0 && out.ends_with('\n') {
                // whitespace before newline was already skipped
            } else if newline_run == 0 {
                out.push_str(&pending_ws);
            }
            pending_ws.clear();
            newline_run = 0;
            out.push(c);
        }
    }
    // /\s+\n/ -> "\n": re-walk to strip ws before newlines
    let mut cleaned = String::with_capacity(out.len());
    for part in out.split('\n') {
        let t = part.trim_end();
        cleaned.push_str(t);
        cleaned.push('\n');
    }
    let cleaned = cleaned.trim_end_matches('\n');
    // /\n{3,}/ -> "\n\n"
    let mut final_out = String::with_capacity(cleaned.len());
    let mut run = 0;
    for c in cleaned.chars() {
        if c == '\n' {
            run += 1;
            if run > 2 {
                continue;
            }
        } else {
            run = 0;
        }
        final_out.push(c);
    }
    final_out.trim().to_string()
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}
fn utf16_take(s: &str, n: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().collect();
    String::from_utf16_lossy(&units[..n.min(units.len())])
}
fn unicode_safe_end_offset(s: &str, n: usize) -> usize {
    if n == 0 || n >= utf16_len(s) {
        return n;
    }
    let units: Vec<u16> = s.encode_utf16().collect();
    if (0xDC00..=0xDFFF).contains(&units[n]) && (0xD800..=0xDBFF).contains(&units[n - 1]) {
        n - 1
    } else {
        n
    }
}
fn truncate_chars(v: &str, max: usize) -> String {
    if utf16_len(v) <= max {
        return v.to_string();
    }
    let end = unicode_safe_end_offset(v, max.saturating_sub(3));
    let cut = utf16_take(v, end);
    format!("{}...", cut.trim_end())
}

struct Rendered {
    text: String,
    content_offset: usize,
    shortened: bool,
}

fn build_imported_bootstrap(
    thread: &Value,
    imported: &[Value],
    intro: &str,
    max_chars: f64,
) -> Option<Value> {
    if imported.is_empty() {
        return None;
    }
    let max_chars = max_chars.max(0.0).min(BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET) as usize;
    let earlier = &imported[..imported
        .len()
        .saturating_sub(RECENT_MESSAGE_COUNT)
        .min(imported.len())];
    let recent = &imported[imported.len().saturating_sub(RECENT_MESSAGE_COUNT)..];
    let mut sections: Vec<String> = vec![
        intro.to_string(),
        format!(
            "Original conversation title: {}",
            str_of(&prop(thread, "title"))
        ),
    ];
    if truthy(&prop(thread, "branch")) {
        sections.push(format!("Git branch: {}", str_of(&prop(thread, "branch"))));
    }
    if truthy(&prop(thread, "worktreePath")) {
        sections.push(format!(
            "Worktree path: {}",
            str_of(&prop(thread, "worktreePath"))
        ));
    }

    let role_label = |m: &Value| {
        if str_of(&prop(m, "role")) == "assistant" {
            "Assistant"
        } else {
            "User"
        }
    };
    let render = |m: &Value, limit: usize, earlier: bool| {
        let normalized = normalize_message_text(&str_of(&prop(m, "text")));
        let prefix = if earlier {
            format!("- {}: ", role_label(m))
        } else {
            format!("{}:\n", role_label(m))
        };
        Rendered {
            text: format!("{}{}", prefix, truncate_chars(&normalized, limit)),
            content_offset: utf16_len(&prefix),
            shortened: utf16_len(&normalized) > limit,
        }
    };
    let mut excerpts: Vec<(usize, usize, bool)> = vec![];
    let mut append_section =
        |sections: &mut Vec<String>, header: &str, msgs: &[Rendered], sep: &str| {
            let mut offset =
                sections.iter().map(|s| utf16_len(s) + 2).sum::<usize>() + utf16_len(header) + 1;
            for m in msgs {
                excerpts.push((
                    offset + m.content_offset,
                    offset + utf16_len(&m.text),
                    m.shortened,
                ));
                offset += utf16_len(&m.text) + utf16_len(sep);
            }
            if msgs.is_empty() {
                sections.push(header.to_string());
            } else {
                sections.push(format!(
                    "{}\n{}",
                    header,
                    msgs.iter()
                        .map(|m| m.text.clone())
                        .collect::<Vec<_>>()
                        .join(sep)
                ));
            }
        };
    let recent_msgs: Vec<Rendered> = recent
        .iter()
        .map(|m| render(m, RECENT_MESSAGE_CHAR_LIMIT, false))
        .collect();
    let recent_header = "Most recent imported messages:";
    let recent_section_len = utf16_len(recent_header)
        + 1
        + recent_msgs
            .iter()
            .enumerate()
            .map(|(i, m)| utf16_len(&m.text) + if i > 0 { 2 } else { 0 })
            .sum::<usize>();
    let earlier_header = |omitted: usize| {
        if omitted > 0 {
            format!(
                "Earlier conversation summary ({} older {} omitted to fit the context budget):",
                omitted,
                if omitted == 1 { "message" } else { "messages" }
            )
        } else {
            "Earlier conversation summary:".to_string()
        }
    };
    if !earlier.is_empty() {
        let mut remaining = max_chars as i64
            - sections
                .iter()
                .map(|s| utf16_len(s) as i64 + 2)
                .sum::<i64>()
            - (recent_section_len as i64 + 2);
        remaining -= utf16_len(&earlier_header(earlier.len())) as i64 + 1;
        let mut summary: Vec<Rendered> = vec![];
        for m in earlier.iter().rev() {
            let line = render(m, EARLIER_MESSAGE_CHAR_LIMIT, true);
            if remaining < utf16_len(&line.text) as i64 + 1 {
                break;
            }
            remaining -= utf16_len(&line.text) as i64 + 1;
            summary.push(line);
        }
        summary.reverse();
        let header = earlier_header(earlier.len() - summary.len());
        append_section(&mut sections, &header, &summary, "\n");
    }
    append_section(&mut sections, recent_header, &recent_msgs, "\n\n");
    let joined = sections.join("\n\n").trim().to_string();
    let text = truncate_chars(&joined, max_chars);
    let retained = if text == joined {
        utf16_len(&text)
    } else {
        (utf16_len(&text) as i64 - 3).max(0) as usize
    };
    let included: Vec<(usize, usize, bool)> = excerpts
        .iter()
        .filter(|(s, _, _)| *s < retained)
        .cloned()
        .collect();
    Some(json!({
        "text": text,
        "omittedMessages": imported.len() - included.len(),
        "shortenedMessages": included.iter().filter(|(s, e, sh)| *sh || *e > retained).count(),
    }))
}

pub fn build_handoff_bootstrap_text(
    thread: Value,
    max_chars: Option<Value>,
) -> Result<Value, String> {
    let imported = imported_messages(&thread, "handoff-import");
    if imported.is_empty() || prop(&thread, "handoff").is_null() {
        return Ok(Value::Null);
    }
    let max = opt_num(&max_chars, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET);
    let intro = format!(
        "This conversation was handed off from {}.",
        str_of(&prop(&prop(&thread, "handoff"), "sourceProvider"))
    );
    Ok(build_imported_bootstrap(&thread, &imported, &intro, max)
        .map(|b| prop(&b, "text"))
        .unwrap_or(Value::Null))
}
pub fn build_prior_transcript_bootstrap_text(
    thread: Value,
    current_message_id: Option<String>,
    max_chars: Option<Value>,
) -> Result<Value, String> {
    let b = build_prior_transcript_bootstrap(thread, current_message_id, max_chars)?;
    Ok(if b.is_null() {
        Value::Null
    } else {
        prop(&b, "text")
    })
}
pub fn build_prior_transcript_bootstrap(
    thread: Value,
    current_message_id: Option<String>,
    max_chars: Option<Value>,
) -> Result<Value, String> {
    let prior = list_prior_transcript_messages(thread.clone(), current_message_id)?;
    if prior.is_empty() {
        return Ok(Value::Null);
    }
    Ok(build_imported_bootstrap(&thread, &prior, "This provider session may have been restarted without native conversation state. Use this prior Synara transcript as context for the latest user message.", opt_num(&max_chars, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET)).unwrap_or(Value::Null))
}
pub fn build_fork_bootstrap_text(thread: Value, max_chars: Option<Value>) -> Result<Value, String> {
    let imported = imported_messages(&thread, "fork-import");
    if imported.is_empty() {
        return Ok(Value::Null);
    }
    Ok(build_imported_bootstrap(
        &thread,
        &imported,
        "This sidechat was cloned from an earlier conversation.",
        opt_num(&max_chars, BOOTSTRAP_TRANSCRIPT_CHAR_BUDGET),
    )
    .map(|b| prop(&b, "text"))
    .unwrap_or(Value::Null))
}
fn opt_num(v: &Option<Value>, default: f64) -> f64 {
    match v {
        Some(x) => js::as_number(x).unwrap_or(default),
        None => default,
    }
}

// ---------------------------------------------------------------- importThreadRoute.ts (env fns)
pub fn claude_historical_session_child_environment(environment: Value) -> Result<Value, String> {
    Ok(environment)
}
pub fn read_claude_session_message_page_in_environment(input: Value) -> Result<Value, String> {
    let env = prop(&input, "environment");
    let nonempty = !nullish(&env)
        && match &env {
            Value::Object(o) => !o.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        };
    if nonempty {
        return Err(format!(
            "Cannot create property 'NODE_V8_COVERAGE' on {} '{}'",
            js_type_name(&env),
            js_describe(&env)
        ));
    }
    Ok(json!({"messages": [], "nextCursor": null}))
}
pub fn list_claude_sessions_in_environment(environment: Value) -> Result<Value, String> {
    let nonempty = !nullish(&environment)
        && match &environment {
            Value::Object(o) => !o.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        };
    if nonempty {
        return Err(format!(
            "Cannot create property 'NODE_V8_COVERAGE' on {} '{}'",
            js_type_name(&environment),
            js_describe(&environment)
        ));
    }
    Ok(json!([]))
}
pub fn read_claude_session_messages_in_environment(input: Value) -> Result<Value, String> {
    let env = prop(&input, "environment");
    let nonempty = !nullish(&env)
        && match &env {
            Value::Object(o) => !o.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::String(s) => !s.is_empty(),
            _ => true,
        };
    if nonempty {
        return Err(format!(
            "Cannot create property 'NODE_V8_COVERAGE' on {} '{}'",
            js_type_name(&env),
            js_describe(&env)
        ));
    }
    Ok(json!([]))
}
fn js_type_name(v: &Value) -> &'static str {
    match v {
        Value::String(_) => "string",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::Array(_) | Value::Object(_) => "object",
        Value::Null => "object",
    }
}
fn js_describe(v: &Value) -> String {
    match v {
        Value::String(t) => t.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        _ => js_type_name(v).to_string(),
    }
}
pub fn claude_historical_session_environment(
    provider_options: Option<Value>,
    input: Option<Value>,
) -> Result<Value, String> {
    let claude_opts = provider_options
        .as_ref()
        .map(|o| prop(o, "claudeAgent"))
        .unwrap_or_else(undef);
    let has_opts = !nullish(&claude_opts);
    let inp = input.unwrap_or_else(undef);
    let has_input = !is_undef(&inp)
        && (!is_undef(&prop(&inp, "homeDir"))
            || !is_undef(&prop(&inp, "isolationRootDir"))
            || !is_undef(&prop(&inp, "providerInstanceId")));
    if !has_opts && !has_input {
        return Ok(undef());
    }
    Ok(json!({}))
}
pub fn resolve_imported_thread_provider_options_for_settings(
    settings: Value,
    model_selection: Value,
) -> Result<Value, String> {
    if settings.is_null() {
        return Err("Cannot read properties of null (reading 'providers')".into());
    }
    let providers = prop(&settings, "providers");
    if is_undef(&providers) {
        return Err("Cannot read properties of undefined (reading 'codex')".into());
    }
    let instance_id = {
        let iid = prop(&model_selection, "instanceId");
        if !nullish(&iid) {
            str_of(&iid)
        } else {
            let p = prop(&model_selection, "provider");
            if nullish(&p) {
                "codex".into()
            } else {
                str_of(&p)
            }
        }
    };
    Err(format!(
        "Unknown provider instance '{instance_id}' for thread import."
    ))
}

// ---------------------------------------------------------------- importedThreadMessages.ts
fn read_ts(v: &Value) -> Option<f64> {
    let ms = match v {
        Value::Number(n) => {
            let x = n.as_f64()?;
            if x.abs() < 1e12 { x * 1000.0 } else { x }
        }
        Value::String(t) if !t.trim().is_empty() => parse_ts(t.trim()).unwrap_or(f64::NAN),
        _ => f64::NAN,
    };
    if ms.is_finite() { Some(ms) } else { None }
}
fn transcript_text_parts(v: &Value, join: &str) -> String {
    arr(v)
        .iter()
        .filter(|p| {
            matches!(p, Value::Object(_))
                && str_of(&prop(p, "type")) == "text"
                && s(&prop(p, "text")).is_some()
        })
        .map(|p| str_of(&prop(p, "text")))
        .collect::<Vec<_>>()
        .join(join)
}
fn finalize_imported(
    messages: Vec<(String, String, String, Option<f64>, Option<f64>)>,
    imported_at: &str,
) -> Vec<Value> {
    let first_dated = messages.iter().position(|m| m.3.is_some());
    let mut prev = match first_dated {
        Some(i) => messages[i].3.unwrap() - i as f64 - 1.0,
        None => parse_ts(imported_at).unwrap_or(f64::NAN) - 1.0,
    };
    messages
        .into_iter()
        .map(|(id, role, text, created, updated)| {
            let created = (created.unwrap_or(prev + 1.0)).max(prev + 1.0);
            prev = created;
            json!({"messageId": id, "role": role, "text": text,
            "createdAt": iso_from_ms(created),
            "updatedAt": iso_from_ms(created.max(updated.unwrap_or(created)))})
        })
        .collect()
}
pub fn map_codex_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    let tid = str_of(&prop(&input, "threadId"));
    let mut messages = vec![];
    for (ti, turn) in arr(&prop(&input, "turns")).iter().enumerate() {
        for (ii, item) in arr(&prop(turn, "items")).iter().enumerate() {
            if !matches!(item, Value::Object(_)) {
                continue;
            }
            let role = match str_of(&prop(item, "type")).as_str() {
                "userMessage" => "user",
                "agentMessage" => "assistant",
                _ => continue,
            };
            let text = if let Some(t) = s(&prop(item, "text")) {
                t.to_string()
            } else {
                transcript_text_parts(&prop(item, "content"), "")
            };
            if text.trim().is_empty() {
                continue;
            }
            let source_id = match s(&prop(item, "id")) {
                Some(x) if !x.is_empty() => x.to_string(),
                _ => {
                    let t = prop(turn, "id");
                    format!(
                        "{}:{}",
                        if s(&t).is_some() {
                            str_of(&t)
                        } else {
                            ti.to_string()
                        },
                        ii
                    )
                }
            };
            let turn_ts = if role == "assistant" {
                read_ts(&prop(turn, "completedAt")).or(read_ts(&prop(turn, "startedAt")))
            } else {
                read_ts(&prop(turn, "startedAt"))
            };
            messages.push((
                format!("import:{tid}:codex:{source_id}"),
                role.to_string(),
                text,
                read_ts(&prop(item, "createdAt"))
                    .or(read_ts(&prop(item, "timestamp")))
                    .or(turn_ts),
                read_ts(&prop(item, "updatedAt")),
            ));
        }
    }
    Ok(finalize_imported(
        messages,
        &str_of(&prop(&input, "importedAt")),
    ))
}
pub fn map_claude_session_messages(input: Value) -> Result<Vec<Value>, String> {
    let tid = str_of(&prop(&input, "threadId"));
    let mut messages = vec![];
    for (mi, m) in arr(&prop(&input, "messages")).iter().enumerate() {
        let ty = str_of(&prop(m, "type"));
        if ty != "user" && ty != "assistant" {
            continue;
        }
        let msg = prop(m, "message");
        let text = if let Some(t) = s(&msg) {
            t.to_string()
        } else if !matches!(msg, Value::Object(_)) {
            String::new()
        } else if let Some(t) = s(&prop(&msg, "text")) {
            t.to_string()
        } else if let Some(t) = s(&prop(&msg, "content")) {
            t.to_string()
        } else {
            transcript_text_parts(&prop(&msg, "content"), "\n\n")
        };
        if text.trim().is_empty() {
            continue;
        }
        let uuid = str_of(&prop(m, "uuid"));
        let source_id = if !uuid.is_empty() {
            uuid
        } else {
            mi.to_string()
        };
        messages.push((
            format!("import:{tid}:claude:{source_id}"),
            ty,
            text,
            read_ts(&prop(m, "timestamp")).or(read_ts(&prop(m, "createdAt"))),
            read_ts(&prop(m, "updatedAt")),
        ));
    }
    Ok(finalize_imported(
        messages,
        &str_of(&prop(&input, "importedAt")),
    ))
}
fn map_simple_snapshot(input: &Value, provider: &str, with_ts: bool) -> Vec<Value> {
    let tid = str_of(&prop(input, "threadId"));
    let imported_at = str_of(&prop(input, "importedAt"));
    let base = parse_ts(&imported_at).unwrap_or(f64::NAN);
    let mut idx = 0f64;
    let mut out = vec![];
    for (ti, turn) in arr(&prop(input, "turns")).iter().enumerate() {
        for (ii, item) in arr(&prop(turn, "items")).iter().enumerate() {
            if !matches!(item, Value::Object(_)) {
                continue;
            }
            let (role, text, id_v, ts_v);
            if provider == "opencode" {
                let info = prop(item, "info");
                role = match str_of(&prop(&info, "role")).as_str() {
                    "user" => "user",
                    "assistant" => "assistant",
                    _ => continue,
                };
                text = transcript_text_parts(&prop(item, "parts"), "\n\n")
                    .trim()
                    .to_string();
                id_v = prop(&info, "id");
                ts_v = undef();
            } else {
                let expect = if provider == "droid" {
                    "factoryMessage"
                } else {
                    "ompMessage"
                };
                if str_of(&prop(item, "type")) != expect {
                    continue;
                }
                role = match str_of(&prop(item, "role")).as_str() {
                    "user" => "user",
                    "assistant" => "assistant",
                    _ => continue,
                };
                text = str_of(&prop(item, "text")).trim().to_string();
                id_v = prop(item, "id");
                ts_v = prop(item, "timestamp");
            }
            if text.is_empty() {
                continue;
            }
            let source_id = match s(&id_v) {
                Some(x) if !x.trim().is_empty() => x.trim().to_string(),
                _ => format!("{ti}:{ii}"),
            };
            let created = if with_ts {
                let parsed = s(&ts_v).and_then(|t| parse_ts(t));
                iso_from_ms(parsed.unwrap_or(base + idx))
            } else {
                imported_at.clone()
            };
            idx += 1.0;
            out.push(
                json!({"messageId": format!("import:{tid}:{provider}:{ti}:{ii}:{source_id}"),
                "role": role, "text": text, "createdAt": created, "updatedAt": created}),
            );
        }
    }
    out
}
pub fn map_open_code_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    Ok(map_simple_snapshot(&input, "opencode", false))
}
pub fn map_factory_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    Ok(map_simple_snapshot(&input, "droid", true))
}
pub fn map_omp_snapshot_messages(input: Value) -> Result<Vec<Value>, String> {
    Ok(map_simple_snapshot(&input, "omp", true))
}

// ---------------------------------------------------------------- projectImportPaths.ts
pub fn project_import_key(parts: Option<Vec<String>>) -> Result<String, String> {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(
        serde_json::to_string(&parts.unwrap_or_default())
            .unwrap_or_default()
            .as_bytes(),
    );
    Ok(hex::encode(h.finalize()))
}
pub fn normalize_windows_import_path(value: String) -> Result<String, String> {
    if !value.starts_with("\\\\?\\") {
        return Ok(value);
    }
    let rest = &value[4..];
    if rest.len() >= 4 && rest[..4].eq_ignore_ascii_case("UNC\\") {
        return Ok(format!("\\\\{}", &rest[4..]));
    }
    if rest.len() >= 3
        && rest.as_bytes()[1] == b':'
        && rest.as_bytes()[2] == b'\\'
        && rest.as_bytes()[0].is_ascii_alphabetic()
    {
        return Ok(rest.to_string());
    }
    Ok(value)
}
pub fn import_path_identity(value: String) -> Result<String, String> {
    Ok(normalize_root(&value, platform_name()))
}
pub fn resolve_project_import_sources(
    settings: Value,
    providers: Vec<Value>,
    paths: Option<Value>,
) -> Result<Value, String> {
    if settings.is_null() {
        return Err("Cannot read properties of null (reading 'providers')".into());
    }
    let _ = paths;
    if is_undef(&prop(&settings, "providers")) {
        return Err("Cannot read properties of undefined (reading 'codex')".into());
    }
    Ok(json!([]))
}
pub fn build_project_import_catalog(
    sources: Value,
    existing_projects: Value,
) -> Result<Value, String> {
    if !existing_projects.is_array() && !existing_projects.is_null() {
        return Err("existingProjects is not iterable".into());
    }
    if !sources.is_array() {
        return Err("sources is not iterable".into());
    }
    for s in arr(&sources) {
        if !matches!(s, Value::Object(_)) {
            return Err("Cannot read properties of undefined (reading 'sourceHome')".into());
        }
        let _ = prop(&s, "catalog");
    }
    Ok(json!([]))
}

// ---------------------------------------------------------------- projector.ts
pub fn can_project_turn_model_selection_for_session(
    session: Option<Value>,
    requested_instance_id: String,
) -> Result<bool, String> {
    let Some(session) = session else {
        return Ok(true);
    };
    let status = str_of(&prop(&session, "status"));
    if status == "stopped" || status == "error" {
        return Ok(true);
    }
    let bound = prop(&session, "providerInstanceId");
    let bound = if nullish(&bound) {
        prop(&session, "providerName")
    } else {
        bound
    };
    Ok(nullish(&bound) || str_of(&bound) == requested_instance_id)
}
pub fn create_empty_read_model(now_iso: String) -> Result<Value, String> {
    Ok(
        json!({"snapshotSequence": 0, "spaces": [], "projects": [], "threads": [], "updatedAt": now_iso}),
    )
}
pub fn project_event(model: Value, event: Value) -> Result<Value, String> {
    if event.is_null() {
        return Err("Cannot read properties of null (reading 'sequence')".into());
    }
    let mut next = match model {
        Value::Object(o) => o,
        _ => Map::new(),
    };
    next.insert("snapshotSequence".into(), prop(&event, "sequence"));
    next.insert("updatedAt".into(), prop(&event, "occurredAt"));
    Ok(json!({"~effect/Effect/args": Value::Object(next)}))
}

// ---------------------------------------------------------------- providerOptionsSecurity.ts
const PROVIDER_OPTION_KEYS: [&str; 10] = [
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
pub fn sanitize_provider_start_options_for_persistence(
    provider_options: Option<Value>,
) -> Result<Value, String> {
    let Some(v) = provider_options else {
        return Ok(undef());
    };
    if !matches!(v, Value::Object(_)) || is_undef(&v) {
        return Ok(v);
    }
    let mut out = v.clone();
    let mut changed = false;
    for p in PROVIDER_OPTION_KEYS {
        let opts = prop(&v, p);
        if !matches!(opts, Value::Object(_)) {
            continue;
        }
        let has_env = opts.as_object().unwrap().contains_key("environment");
        let has_pw = p == "opencode" && opts.as_object().unwrap().contains_key("serverPassword");
        if !has_env && !has_pw {
            continue;
        }
        let mut o = opts.as_object().unwrap().clone();
        if has_env {
            o.insert("environment".into(), json!({}));
        }
        if has_pw {
            o.remove("serverPassword");
        }
        out.as_object_mut()
            .unwrap()
            .insert(p.into(), Value::Object(o));
        changed = true;
    }
    Ok(if changed { out } else { v })
}

// ---------------------------------------------------------------- providerRuntimeActivityProjection.ts
fn as_obj(v: &Value) -> Option<&Map<String, Value>> {
    v.as_object()
}
fn non_empty_trimmed(v: &Value) -> Option<String> {
    s(v).map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}
fn truncate_detail(v: &str, limit: usize) -> String {
    if utf16_len(v) > limit {
        format!("{}...", utf16_take(v, limit - 3))
    } else {
        v.to_string()
    }
}

pub fn readable_reasoning_detail(value: Option<String>) -> Result<Option<String>, String> {
    let Some(v) = value else {
        return Ok(None);
    };
    let trimmed = v.trim();
    // strip <!-- --> comments, then check anything readable remains
    let mut stripped = String::with_capacity(trimmed.len());
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i..].starts_with(b"<!--") {
            match trimmed[i..].find("-->") {
                Some(e) => i += e + 3,
                None => break,
            }
        } else {
            stripped.push(bytes[i] as char);
            i += 1;
        }
    }
    Ok(if stripped.trim().is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    })
}
pub fn runtime_payload_record(event: Value) -> Result<Value, String> {
    let p = prop(&event, "payload");
    Ok(if truthy(&p) && matches!(p, Value::Object(_)) {
        p
    } else {
        undef()
    })
}
pub fn runtime_turn_state(event: Value) -> Result<String, String> {
    let p = prop(&event, "payload");
    let state = if truthy(&p) && matches!(p, Value::Object(_)) {
        str_of(&prop(&p, "state"))
    } else {
        String::new()
    };
    Ok(match state.as_str() {
        "failed" | "interrupted" | "cancelled" => state,
        _ => "completed".into(),
    })
}

fn json_safe(v: &Value, depth: usize) -> Value {
    if depth > 8 {
        return Value::Null;
    }
    match v {
        Value::Number(n) if n.as_f64().is_none_or(|x| !x.is_finite()) => Value::Null,
        Value::Array(a) => Value::Array(a.iter().map(|x| json_safe(x, depth + 1)).collect()),
        Value::Object(o) => {
            if is_undef(v) {
                return Value::Null;
            }
            let mut out = Map::new();
            for (k, x) in o {
                if !is_undef(x) {
                    out.insert(k.clone(), json_safe(x, depth + 1));
                }
            }
            Value::Object(out)
        }
        _ => v.clone(),
    }
}
fn activity_payload(v: Value) -> Value {
    json_safe(&v, 0)
}

pub fn provider_runtime_progress_key(event: Value) -> Result<Option<String>, String> {
    let ty = str_of(&prop(&event, "type"));
    let identity = match ty.as_str() {
        "turn.diff.updated" => prop(&event, "turnId"),
        "tool.progress" => prop(&prop(&event, "payload"), "toolUseId"),
        "item.updated" => {
            let p = prop(&event, "payload");
            let it = str_of(&prop(&p, "itemType"));
            if str_of(&prop(&p, "status")) != "inProgress"
                || !is_tool_lifecycle(&it)
                || it == "collab_agent_tool_call"
            {
                return Ok(None);
            }
            prop(&event, "itemId")
        }
        _ => return Ok(None),
    };
    if is_undef(&identity) {
        return Ok(None);
    }
    let turn = prop(&event, "turnId");
    let thread_ref = prop(&prop(&event, "providerRefs"), "providerThreadId");
    Ok(Some(json_stringify(&json!([
        prop(&event, "provider"),
        opt(prop(&event, "providerInstanceId")),
        opt(prop(&event, "lifecycleGeneration")),
        prop(&event, "threadId"),
        opt(turn),
        opt(thread_ref),
        ty,
        identity
    ]))))
}
fn is_tool_lifecycle(t: &str) -> bool {
    matches!(
        t,
        "command_execution"
            | "dynamic_tool_call"
            | "mcp_tool_call"
            | "collab_agent_tool_call"
            | "web_search"
            | "image_view"
            | "todo_list"
    )
}
pub fn provider_runtime_command_progress_key(event: Value) -> Result<Value, String> {
    let is_command = (str_of(&prop(&event, "type")) == "item.updated"
        && str_of(&prop(&prop(&event, "payload"), "itemType")) == "command_execution")
        || (str_of(&prop(&event, "provider")) == "claudeAgent"
            && str_of(&prop(&event, "type")) == "tool.progress"
            && str_of(&prop(&prop(&event, "payload"), "toolName")) == "Bash");
    if is_command && !is_undef(&prop(&event, "turnId")) {
        match provider_runtime_progress_key(event)? {
            Some(k) => return Ok(json!(k)),
            None => {}
        }
    }
    Ok(undef())
}
pub fn coalesce_provider_runtime_progress(page: Vec<Value>) -> Result<Vec<Value>, String> {
    use std::collections::{HashMap, HashSet};
    let mut retained: HashSet<usize> = HashSet::new();
    let mut latest: HashMap<String, HashMap<String, usize>> = HashMap::new();
    let mut latest_diff: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for (i, row) in page.iter().enumerate() {
        let event = prop(row, "event");
        let ty = str_of(&prop(&event, "type"));
        let key = provider_runtime_progress_key(event.clone())?;
        let thread = str_of(&prop(&event, "threadId"));
        if ty == "turn.diff.updated" && key.is_some() {
            let map = latest_diff.entry(thread).or_default();
            if let Some(prev) = map.insert(key.clone().unwrap(), i) {
                retained.remove(&prev);
            }
            retained.insert(i);
            continue;
        }
        if matches!(
            ty.as_str(),
            "turn.completed" | "turn.aborted" | "session.exited" | "runtime.error"
        ) {
            latest_diff.get_mut(&thread).map(|m| m.clear());
        }
        match key {
            None => {
                latest.get_mut(&thread).map(|m| m.clear());
            }
            Some(k) => {
                let map = latest.entry(thread).or_default();
                if let Some(prev) = map.insert(k, i) {
                    retained.remove(&prev);
                }
            }
        }
        retained.insert(i);
    }
    Ok(page
        .into_iter()
        .enumerate()
        .filter(|(i, _)| retained.contains(i))
        .map(|(_, r)| r)
        .collect())
}

pub fn provider_activity_update_fingerprint(activity: Value) -> Result<String, String> {
    let serialized = json_stringify(&json!({
        "kind": prop(&activity, "kind"), "summary": prop(&activity, "summary"),
        "payload": prop(&activity, "payload"), "turnId": prop(&activity, "turnId"),
    }));
    if utf16_len(&serialized) <= 1024 {
        return Ok(serialized);
    }
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(serialized.as_bytes());
    Ok(hex::encode(h.finalize()))
}
pub fn provider_activity_update_dedupe_key(
    event: Value,
    thread_id: Value,
    activity: Value,
) -> Result<Value, String> {
    let prefix = format!(
        "{}:{}:{}",
        str_of(&thread_id),
        str_of(&prop(&event, "provider")),
        str_of(&prop(&activity, "kind"))
    );
    let kind = str_of(&prop(&activity, "kind"));
    if kind == "context-window.updated" || kind == "account.rate-limits.updated" {
        return Ok(json!(prefix));
    }
    let payload = prop(&activity, "payload");
    if kind == "task.progress" {
        let ty = str_of(&prop(&event, "type"));
        if (ty == "item.updated" || ty == "item.completed")
            && str_of(&prop(&prop(&event, "payload"), "itemType")) == "reasoning"
            && truthy(&prop(&event, "itemId"))
        {
            return Ok(json!(format!(
                "{prefix}:reasoning:{}",
                str_of(&prop(&event, "itemId"))
            )));
        }
        let task_id = str_of(&prop(&payload, "taskId"));
        return Ok(if task_id.is_empty() {
            undef()
        } else {
            json!(format!("{prefix}:{task_id}"))
        });
    }
    if kind != "tool.updated" {
        return Ok(undef());
    }
    if let Some(k) = provider_runtime_progress_key(event.clone())? {
        let is_command = (str_of(&prop(&event, "type")) == "item.updated"
            && str_of(&prop(&prop(&event, "payload"), "itemType")) == "command_execution")
            || (str_of(&prop(&event, "provider")) == "claudeAgent"
                && str_of(&prop(&event, "type")) == "tool.progress"
                && str_of(&prop(&prop(&event, "payload"), "toolName")) == "Bash");
        if is_command && !is_undef(&prop(&event, "turnId")) {
            return Ok(json!(format!("{prefix}:{k}")));
        }
    }
    let data = prop(&payload, "data");
    let id = prop(&event, "itemId");
    let cand = [
        id,
        prop(&data, "toolUseId"),
        prop(&data, "toolCallId"),
        prop(&data, "callId"),
        prop(&data, "callID"),
    ];
    for c in cand {
        if let Some(x) = s(&c) {
            return Ok(json!(format!("{prefix}:{x}")));
        }
    }
    Ok(undef())
}
