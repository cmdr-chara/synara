//! Clean-room implementation of the proposed-plan helpers
//! (`apps/web/src/proposedPlan.ts`), derived exclusively from the
//! recorded input/output pairs in `cases.jsonl`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const IMPLEMENT_PROMPT_PREFIX: &str = "PLEASE IMPLEMENT THIS PLAN:\n";
const PREVIEW_UNAVAILABLE: &str = "Plan preview unavailable.";
const DEFAULT_PREVIEW_LINES: f64 = 8.0;

/// The exact `String.prototype.trim` character set: White_Space plus
/// U+FEFF, minus NEL (U+0085) which JS does not treat as whitespace.
fn js_ws(c: char) -> bool {
    (c.is_whitespace() && c != '\u{85}') || c == '\u{FEFF}'
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js_ws)
}

fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(js_ws)
}

/// A line is a markdown heading when, after leading whitespace, it has
/// one or more `#` followed by whitespace then content.
fn heading_text(line: &str) -> Option<&str> {
    let t = line.trim_start_matches(js_ws);
    let after = t.strip_prefix('#')?;
    let after = after.trim_start_matches('#');
    let after = after.strip_prefix(|c: char| js_ws(c))?;
    Some(after)
}

fn first_heading_text(md: &str) -> Option<String> {
    for line in md.split('\n') {
        if let Some(rest) = heading_text(line) {
            return Some(js_trim(rest).to_string());
        }
    }
    None
}

/// `proposedPlanTitle`: the text of the first markdown heading line,
/// or null when the plan has none.
pub fn proposed_plan_title(plan_markdown: String) -> Result<Option<String>, String> {
    match first_heading_text(&plan_markdown) {
        Some(t) if !t.is_empty() => Ok(Some(t)),
        _ => Ok(None),
    }
}

/// `stripDisplayedPlanMarkdown`: normalizes CRLF, drops a leading
/// heading line plus a following `## Summary` heading (each with their
/// trailing blank lines), drops leading blank lines, trims the end.
pub fn strip_displayed_plan_markdown(plan_markdown: String) -> Result<String, String> {
    let normalized = plan_markdown.replace("\r\n", "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();

    let drop_block = |lines: &mut Vec<&str>| {
        lines.remove(0);
        while lines
            .first()
            .is_some_and(|l| l.trim_matches(js_ws).is_empty())
        {
            lines.remove(0);
        }
    };

    if !lines.is_empty() && heading_text(lines[0]).is_some() {
        drop_block(&mut lines);
        if lines
            .first()
            .is_some_and(|l| l.trim_start_matches(js_ws).starts_with("## Summary"))
        {
            drop_block(&mut lines);
        }
    }
    while lines
        .first()
        .is_some_and(|l| l.trim_matches(js_ws).is_empty())
    {
        lines.remove(0);
    }
    Ok(js_trim_end(&lines.join("\n")).to_string())
}

/// JS `ToNumber` on a wire value (as used by `options.maxLines`):
/// strings parse, booleans coerce 0/1, arrays join-then-parse,
/// `{$num}` markers decode, everything else is NaN.
fn to_number(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Bool(b) => i32::from(*b) as f64,
        Value::String(s) => {
            let t = js_trim(s);
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(items) => {
            let joined = items
                .iter()
                .map(|x| match x {
                    Value::String(s) => s.clone(),
                    _ if x.is_null() || crate::js::is_undefined(x) => String::new(),
                    Value::Bool(b) => b.to_string(),
                    other => {
                        crate::js::number_to_string(crate::js::as_number(other).unwrap_or(f64::NAN))
                    }
                })
                .collect::<Vec<_>>()
                .join(",");
            if js_trim(&joined).is_empty() {
                0.0
            } else {
                js_trim(&joined).parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        _ => f64::NAN,
    }
}

/// `buildCollapsedProposedPlanPreviewMarkdown`: the stripped plan cut
/// to `ceil(maxLines)` lines (default 8) followed by `...` when
/// truncated; `Plan preview unavailable.` when nothing remains.
pub fn build_collapsed_proposed_plan_preview_markdown(
    plan_markdown: String,
    options: Option<serde_json::Value>,
) -> Result<String, String> {
    let stripped = strip_displayed_plan_markdown(plan_markdown)?;
    let raw = match options.as_ref() {
        Some(Value::Object(m)) => match m.get("maxLines") {
            None | Some(Value::Null) => DEFAULT_PREVIEW_LINES,
            Some(v) if crate::js::is_undefined(v) => DEFAULT_PREVIEW_LINES,
            Some(v) => to_number(v),
        },
        _ => DEFAULT_PREVIEW_LINES,
    };
    let n = raw.ceil();
    let lines: Vec<&str> = stripped.split('\n').collect();
    let out = if (lines.len() as f64) > n {
        let keep = if n > 0.0 && n.is_finite() {
            lines[..(n as usize).min(lines.len())].join("\n")
        } else {
            String::new()
        };
        if keep.is_empty() {
            String::new()
        } else {
            format!("{keep}\n\n...")
        }
    } else {
        stripped
    };
    Ok(if out.is_empty() {
        PREVIEW_UNAVAILABLE.to_string()
    } else {
        out
    })
}

/// `buildPlanImplementationPrompt`: the fixed prompt prefix plus the
/// trimmed plan markdown.
pub fn build_plan_implementation_prompt(plan_markdown: String) -> Result<String, String> {
    Ok(format!(
        "{IMPLEMENT_PROMPT_PREFIX}{}",
        js_trim(&plan_markdown)
    ))
}

/// `input[key]` errors the way `input.key` does on a null or
/// undefined receiver; any other non-object yields undefined.
fn prop(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        _ if crate::js::is_undefined(v) => Err(format!(
            "Cannot read properties of undefined (reading '{key}')"
        )),
        Value::Object(m) => Ok(m.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

/// `.trim()` on a member value, with the recorded TypeError messages.
fn trimmed_member(input: &Value, key: &str, expr: &str) -> Result<String, String> {
    let v = prop(input, key)?;
    if crate::js::is_undefined(&v) {
        return Err("Cannot read properties of undefined (reading 'trim')".to_string());
    }
    match &v {
        Value::Null => Err("Cannot read properties of null (reading 'trim')".to_string()),
        Value::String(s) => Ok(js_trim(s).to_string()),
        _ => Err(format!("{expr} is not a function")),
    }
}

/// `resolvePlanFollowUpSubmission`: a non-empty draft wins with mode
/// `plan`; otherwise the implementation prompt with mode `default`.
pub fn resolve_plan_follow_up_submission(
    input: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let draft = trimmed_member(&input, "draftText", "input.draftText.trim")?;
    if !draft.is_empty() {
        return Ok(json!({"text": draft, "interactionMode": "plan"}));
    }
    let plan = trimmed_member(&input, "planMarkdown", "planMarkdown.trim")?;
    Ok(json!({
        "text": format!("{IMPLEMENT_PROMPT_PREFIX}{plan}"),
        "interactionMode": "default",
    }))
}

/// `buildPlanImplementationThreadTitle`: `Implement <title>` or the
/// `Implement plan` fallback.
pub fn build_plan_implementation_thread_title(plan_markdown: String) -> Result<String, String> {
    Ok(match proposed_plan_title(plan_markdown)? {
        Some(t) => format!("Implement {t}"),
        None => "Implement plan".to_string(),
    })
}

/// `buildProposedPlanMarkdownFilename`: the title lowercased, inline
/// punctuation removed, non-alphanumeric runs collapsed to `-`;
/// `plan.md` when no usable slug remains.
pub fn build_proposed_plan_markdown_filename(plan_markdown: String) -> Result<String, String> {
    let Some(title) = proposed_plan_title(plan_markdown)? else {
        return Ok("plan.md".to_string());
    };
    let mut slug = String::with_capacity(title.len());
    let mut pending_sep = false;
    for ch in title.to_lowercase().chars() {
        if matches!(ch, '.' | ',' | '\'' | '"') {
            continue;
        }
        if ch.is_ascii_alphanumeric() {
            if pending_sep && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch);
            pending_sep = false;
        } else {
            pending_sep = true;
        }
    }
    Ok(if slug.is_empty() {
        "plan.md".to_string()
    } else {
        format!("{slug}.md")
    })
}

/// `normalizePlanMarkdownForExport`: the plan trimmed at the end,
/// always newline-terminated.
pub fn normalize_plan_markdown_for_export(plan_markdown: String) -> Result<String, String> {
    Ok(format!("{}\n", js_trim_end(&plan_markdown)))
}

/// `stripProposedPlanBlocksFromText`: removes every
/// `<proposed_plan>…</proposed_plan>` block (case-insensitive,
/// non-greedy), then trims.
pub fn strip_proposed_plan_blocks_from_text(text: String) -> Result<String, String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text.as_str();
    loop {
        let Some(open) = find_ci(rest, "<proposed_plan>") else {
            out.push_str(rest);
            break;
        };
        let close_from = open + "<proposed_plan>".len();
        match find_ci(&rest[close_from..], "</proposed_plan>") {
            Some(close) => {
                out.push_str(&rest[..open]);
                rest = &rest[close_from + close + "</proposed_plan>".len()..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    Ok(js_trim(&out).to_string())
}

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}
