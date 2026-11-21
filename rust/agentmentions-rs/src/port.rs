//! The Rust port of `packages/shared/src/agentMentions.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn claude_subagent(alias: &str, agent_name: &str) -> Value {
    let (display_name, color, description, prompt, tools, model): (
        &str,
        &str,
        &str,
        &str,
        &[&str],
        &str,
    ) = match agent_name {
        "review" => (
            "Code Review",
            "amber",
            "Bug and risk reviewer. Use for code review, regression hunting, and edge-case analysis.",
            "You are a senior code reviewer. Focus on behavioral regressions, correctness bugs, edge cases, and missing tests. Return findings first, then open questions, then a brief summary.",
            &["Read", "Grep", "Glob"],
            "sonnet",
        ),
        "explore" => (
            "Explore",
            "cyan",
            "Read-only codebase explorer. Use for file discovery, code search, and gathering context before implementation.",
            "You are a focused codebase exploration specialist. Search broadly, gather the most relevant findings, and return a concise summary with the key files, evidence, and risks. Do not make code changes.",
            &["Read", "Grep", "Glob"],
            "haiku",
        ),
        "plan" => (
            "Planner",
            "fuchsia",
            "Planning specialist. Use for breaking work into steps, evaluating approaches, and preparing execution plans.",
            "You are a planning specialist. Clarify goals, evaluate tradeoffs, identify edge cases, and return a concrete ordered plan with the main risks called out explicitly.",
            &["Read", "Grep", "Glob", "TodoWrite"],
            "sonnet",
        ),
        _ => (
            "Implementer",
            "violet",
            "Implementation teammate. Use for scoped code changes, debugging, and hands-on execution tasks.",
            "You are an implementation-focused coding teammate. Make targeted changes, validate assumptions with the available tools, and return a short implementation summary plus any remaining risks.",
            &["Read", "Grep", "Glob", "Bash", "Edit", "Write", "MultiEdit"],
            "sonnet",
        ),
    };
    json!({
        "alias": alias,
        "provider": "claudeAgent",
        "kind": "claude-subagent",
        "agentName": agent_name,
        "displayName": display_name,
        "color": color,
        "description": description,
        "prompt": prompt,
        "tools": tools,
        "model": model,
    })
}

fn codex_model(alias: &str, canonical: &str) -> Value {
    let (model, display_name, color): (&str, &str, &str) = match canonical {
        "spark" => ("gpt-5.3-codex-spark", "GPT-5.3 Codex Spark", "cyan"),
        "mini" => ("gpt-5.4-mini", "GPT-5.4 Mini", "fuchsia"),
        _ => ("gpt-5.3-codex", "GPT-5.3 Codex", "teal"),
    };
    json!({
        "alias": alias,
        "provider": "codex",
        "kind": "model",
        "model": model,
        "displayName": display_name,
        "color": color,
    })
}

/// `(canonical_agent_name, is_claude)` for an alias inside one provider's registry.
/// `None` is the falsy-provider path, which resolves against the union registry.
fn lookup_alias(provider: Option<&str>, normalized: &str) -> Option<Option<(&'static str, bool)>> {
    let claude = |a: &str| match a {
        "review" | "reviewer" => Some(("review", true)),
        "explore" => Some(("explore", true)),
        "plan" | "planner" => Some(("plan", true)),
        "build" | "implement" => Some(("build", true)),
        _ => None,
    };
    let codex = |a: &str| match a {
        "spark" => Some(("spark", false)),
        "mini" => Some(("mini", false)),
        "codex" => Some(("codex", false)),
        _ => None,
    };
    match provider {
        None => Some(claude(normalized).or_else(|| codex(normalized))),
        Some("claudeAgent") => Some(claude(normalized)),
        Some("codex") => Some(codex(normalized)),
        Some("opencode" | "cursor" | "droid" | "pi") => Some(None),
        Some(_) => None,
    }
}

/// JS truthiness: null/undefined/false/0/NaN/"" are falsy; objects and arrays are truthy.
fn js_truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// Coercion of `provider` to an object key (JS `String(v)` for the truthy path).
fn provider_key(v: &Value) -> Option<String> {
    if !js_truthy(v) {
        return None;
    }
    Some(match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::Bool(b) => b.to_string(),
        Value::Array(items) => items
            .iter()
            .map(|i| match i {
                Value::Null => String::new(),
                Value::Bool(b) => b.to_string(),
                Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
                Value::String(s) => s.clone(),
                _ => String::from("[object Object]"),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) | Value::Null => String::from("[object Object]"),
    })
}

/// JS `\s`: ASCII whitespace plus the Unicode separators RegExp `\s` covers.
fn is_js_space(u: u16) -> bool {
    matches!(
        u,
        0x0009..=0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F
            | 0x205F | 0x3000 | 0xFEFF
    )
}

fn is_alias_char(u: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&u)
        || (u16::from(b'A')..=u16::from(b'Z')).contains(&u)
        || (u16::from(b'a')..=u16::from(b'z')).contains(&u)
        || u == u16::from(b'_')
}

fn js_trim(units: &[u16]) -> &[u16] {
    let mut start = 0;
    let mut end = units.len();
    while start < end && is_js_space(units[start]) {
        start += 1;
    }
    while end > start && is_js_space(units[end - 1]) {
        end -= 1;
    }
    &units[start..end]
}

/// Port of `parseAgentMentionInvocations` (`packages/shared/src/agentMentions.ts`).
pub fn parse_agent_mention_invocations(
    text: String,
    provider: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    let provider_key = provider_key(&provider);
    let units = crate::utf16::units(&text);
    let mut invocations = Vec::new();
    let mut i = 0;
    while i < units.len() {
        if units[i] != u16::from(b'@') || (i > 0 && !is_js_space(units[i - 1])) {
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < units.len() && is_alias_char(units[j]) {
            j += 1;
        }
        let alias = crate::utf16::slice(&units, i + 1, j);
        if alias.is_empty() || j >= units.len() || units[j] != u16::from(b'(') {
            i += 1;
            continue;
        }
        let mut depth = 1usize;
        let mut k = j + 1;
        while k < units.len() && depth > 0 {
            if units[k] == u16::from(b'(') {
                depth += 1;
            } else if units[k] == u16::from(b')') {
                depth -= 1;
            }
            k += 1;
        }
        if depth != 0 {
            i += 1;
            continue;
        }
        let normalized = alias.to_lowercase();
        let Some(hit) = lookup_alias(provider_key.as_deref(), &normalized) else {
            return Err(format!(
                "Cannot read properties of undefined (reading '{normalized}')"
            ));
        };
        let task_units = js_trim(&units[j + 1..k - 1]);
        if task_units.is_empty() {
            i += 1;
            continue;
        }
        i += 1;
        let Some((canonical, is_claude)) = hit else {
            continue;
        };
        let definition = if is_claude {
            claude_subagent(&alias, canonical)
        } else {
            codex_model(&alias, canonical)
        };
        invocations.push(json!({
            "alias": alias,
            "task": crate::utf16::slice(task_units, 0, task_units.len()),
            "raw": crate::utf16::slice(&units, i - 1, k),
            "start": i - 1,
            "end": k,
            "definition": definition,
        }));
    }
    Ok(invocations)
}

/// Port of `buildClaudeSubagentPrompt` (`packages/shared/src/agentMentions.ts`).
pub fn build_claude_subagent_prompt(text: String) -> Result<serde_json::Value, String> {
    let invocations = parse_agent_mention_invocations(text.clone(), json!("claudeAgent"))?;
    if invocations.is_empty() {
        return Ok(json!({ "prompt": text, "invocations": [] }));
    }
    let mut prompt = String::from(
        "The user included inline subagent directives in the form @alias(task).\n\
         Execute each directive explicitly via the Agent tool using the named subagent below.\n\
         After the delegated work completes, continue with the overall request and synthesize the results.\n\
         Do not echo the literal @alias(task) syntax back to the user unless it is directly relevant.\n\n\
         Inline directives:\n",
    );
    for (index, invocation) in invocations.iter().enumerate() {
        let agent_name = invocation["definition"]["agentName"]
            .as_str()
            .unwrap_or_default();
        let task = invocation["task"].as_str().unwrap_or_default();
        if index > 0 {
            prompt.push_str("\n\n");
        }
        prompt.push_str(&format!(
            "{}. Use the \"{}\" agent for this task:\n{}",
            index + 1,
            agent_name,
            task
        ));
    }
    prompt.push_str("\n\nOriginal user prompt:\n");
    prompt.push_str(&text);
    Ok(json!({ "prompt": prompt, "invocations": invocations }))
}
