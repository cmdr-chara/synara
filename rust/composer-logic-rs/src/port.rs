//! The Rust port of `apps/web/src/composer-logic.ts`.
//!
//! Inlined helpers (crate stays self-contained):
//! - `splitPromptIntoComposerSegments` and its token matchers from
//!   `apps/web/src/composer-editor-mentions.ts` (mentions, skills, slash chips,
//!   agent mentions, link chips, terminal-context placeholder).
//! - `createComposerMentionTokenRegex`, `extractComposerMentionPath`,
//!   `decodeComposerMentionQuotedPath`, `composerMentionQuotedPathHasClosingQuote`
//!   from `apps/web/src/lib/composerMentions.ts`.
//! - `LINK_TOKEN_SOURCE`, `trimTrailingLinkPunctuation`,
//!   `normalizeComposerLinkUrl` from `apps/web/src/lib/linkChips.ts`.
//! - `isBuiltInComposerSlashCommand`/`BUILT_IN_COMPOSER_SLASH_COMMANDS` from
//!   `apps/web/src/composerSlashCommands.ts` + `packages/shared`/`contracts`.
//! - `resolveAgentAlias` (alias → color only) from
//!   `packages/contracts/src/agentMentions.ts`.
//! - `INLINE_TERMINAL_CONTEXT_PLACEHOLDER` (`\u{FFFC}`) from
//!   `apps/web/src/lib/terminalContext.ts`.
//!
//! All string offsets are JS UTF-16 code-unit offsets; `crate::utf16` supplies
//! the unit machinery. Regular expressions are hand-rolled scanners operating on
//! `&[u16]` with the same leftmost-match/backtracking outcomes as the JS
//! patterns they replace.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const FFFC: u16 = 0xFFFC;

// ---------- JS character classes ----------

/// JS `\s` (25 code units).
fn is_js_space(u: u16) -> bool {
    matches!(
        u,
        0x0009 | 0x000A | 0x000B | 0x000C | 0x000D | 0x0020 | 0x00A0 | 0x1680 | 0x2000
            ..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF
    )
}

/// JS line terminators, which `.` (without /s) cannot match.
fn is_line_terminator(u: u16) -> bool {
    matches!(u, 0x000A | 0x000D | 0x2028 | 0x2029)
}

/// `isWhitespace` in composer-logic.ts: space, \n, \t, \r, or the placeholder.
fn is_composer_ws(u: u16) -> bool {
    matches!(u, 0x0020 | 0x000A | 0x0009 | 0x000D | FFFC)
}

fn is_ascii_alpha(u: u16) -> bool {
    matches!(u, 0x41..=0x5a | 0x61..=0x7a)
}

fn is_ascii_digit_u16(u: u16) -> bool {
    (0x30..=0x39).contains(&u)
}

fn is_ascii_alnum(u: u16) -> bool {
    matches!(u, 0x30..=0x39 | 0x41..=0x5a | 0x61..=0x7a)
}

fn u16_units(s: &str) -> Vec<u16> {
    crate::utf16::units(s)
}

fn units_str(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

// ---------- JS number/string coercion ----------

/// `ToIntegerOrInfinity(x)` where x is already a JS number.
fn to_integer(n: f64) -> f64 {
    if n.is_nan() {
        0.0
    } else if n.is_infinite() {
        n
    } else {
        n.trunc()
    }
}

/// JS `Math.min` — propagates NaN, prefers -0 over +0.
fn js_min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// JS `Math.max` — propagates NaN, prefers +0 over -0.
fn js_max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.max(b)
    }
}

/// `Number(str)` — JS parse after trim: "" -> 0, "Infinity" -> inf, decimal.
fn js_number_str(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| {
        let mut buf = [0u16; 2];
        c.encode_utf16(&mut buf).iter().all(|&u| is_js_space(u))
    });
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(v)` for an arbitrary JSON value.
fn js_number_value(v: &Value) -> f64 {
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    if crate::js::is_undefined(v) {
        return f64::NAN;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => *b as i64 as f64,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_number_str(s),
        Value::Array(items) => {
            // ToPrimitive via ToString: elements joined with ',' (null/undefined -> "").
            let joined = items
                .iter()
                .map(|item| match item {
                    Value::Null => String::new(),
                    v if crate::js::is_undefined(v) => String::new(),
                    Value::String(s) => s.clone(),
                    Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
                    Value::Bool(b) => b.to_string(),
                    _ => "[object Object]".to_string(),
                })
                .collect::<Vec<_>>()
                .join(",");
            js_number_str(&joined)
        }
        _ => f64::NAN,
    }
}

// ---------- JS slice / clamp ----------

/// `text.slice(start)` — start is ToIntegerOrInfinity'd and clamped; end = len.
fn js_slice_from(units: &[u16], start: f64) -> &[u16] {
    let len = units.len() as f64;
    let s = to_integer(start);
    let from = if s < 0.0 {
        js_max(len + s, 0.0)
    } else {
        js_min(s, len)
    };
    &units[from as usize..]
}

/// `text.slice(start, end)` — `end === undefined` means end is omitted -> len.
fn js_slice_to(units: &[u16], start: f64, end: &Value) -> Vec<u16> {
    let end_n = if crate::js::is_undefined(end) {
        f64::INFINITY
    } else {
        js_number_value(end)
    };
    let len = units.len() as f64;
    let s = to_integer(start);
    let e = to_integer(end_n);
    let from = if s < 0.0 {
        js_max(len + s, 0.0)
    } else {
        js_min(s, len)
    };
    let to = if e < 0.0 {
        js_max(len + e, 0.0)
    } else {
        js_min(e, len)
    };
    if from >= to {
        Vec::new()
    } else {
        units[from as usize..to as usize].to_vec()
    }
}

/// `clampCursor`: non-finite input -> text length; else floor, clamped.
fn clamp_cursor(len: usize, input: f64) -> f64 {
    if !input.is_finite() {
        return len as f64;
    }
    js_max(0.0, js_min(len as f64, input.floor()))
}

/// `String.prototype.toLowerCase` applied code point by code point (lone
/// surrogates pass through; only used for comparisons, never re-encoded).
fn js_to_lowercase(units: &[u16]) -> String {
    let mut out = String::new();
    for c in char::decode_utf16(units.iter().copied()) {
        let c = c.unwrap_or(char::REPLACEMENT_CHARACTER);
        out.extend(c.to_lowercase());
    }
    out
}

/// JS `String.prototype.trim` on units.
fn js_trim_units(units: &[u16]) -> &[u16] {
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

// ---------- inlined data ----------

/// `BUILT_IN_COMPOSER_SLASH_COMMANDS` (`packages/contracts/src/composerSlashCommands.ts`).
const BUILT_IN_COMMANDS: &[&str] = &[
    "clear",
    "compact",
    "model",
    "plan",
    "debug",
    "default",
    "review",
    "fork",
    "side",
    "status",
    "subagents",
    "computer-use",
    "fast",
    "export",
    "goal",
    "rename",
    "feedback",
    "automation",
];

/// `COMPOSER_SLASH_COMMAND_CHIP_NAMES` (`composer-editor-mentions.ts:70`).
const CHIP_NAMES: &[&str] = &["automation", "goal", "computer-use"];

/// `resolveAgentAlias(alias)` with no provider: codex table first, then
/// claudeAgent (`agentMentions.ts`). Only `color` is read downstream.
fn resolve_agent_alias_color(alias_units: &[u16]) -> Option<&'static str> {
    let normalized = js_to_lowercase(alias_units);
    let color = match normalized.as_str() {
        // codex
        "5.5" => "violet",
        "5.4" => "violet",
        "mini" => "fuchsia",
        "5.4-mini" => "fuchsia",
        "codex" => "teal",
        "5.3-codex" => "teal",
        "spark" => "cyan",
        "5.3-spark" => "cyan",
        "5.2" => "amber",
        "5.2-codex" => "orange",
        // claudeAgent
        "explore" => "cyan",
        "review" => "amber",
        "reviewer" => "amber",
        "build" => "violet",
        "implement" => "violet",
        "plan" => "fuchsia",
        "planner" => "fuchsia",
        _ => return None,
    };
    Some(color)
}

/// `isBuiltInComposerSlashCommand` on an already-lowercased ASCII name
/// (normalizeComposerSlashCommandName cannot change it further).
fn is_built_in_command(name_lower: &str) -> bool {
    BUILT_IN_COMMANDS.contains(&name_lower)
}

fn is_chip_command(name_lower: &str) -> bool {
    is_built_in_command(name_lower) && CHIP_NAMES.contains(&name_lower)
}

// ---------- link helpers (`linkChips.ts`) ----------

/// `TRAILING_PUNCTUATION_REGEX` = `/[.,;:!?'"]+$/`.
fn trim_trailing_link_punctuation_units(units: &[u16]) -> &[u16] {
    let mut end = units.len();
    while end > 0
        && matches!(
            units[end - 1],
            0x2e | 0x2c | 0x3b | 0x3a | 0x21 | 0x3f | 0x27 | 0x22
        )
    {
        end -= 1;
    }
    &units[..end]
}

const PUBLIC_TLDS: &[&str] = &["ai", "app", "co", "com", "dev", "io", "net", "org"];
const FILE_EXT_TLDS: &[&str] = &[
    "c", "cc", "conf", "cpp", "css", "go", "h", "hpp", "html", "java", "js", "json", "jsx", "kt",
    "lock", "md", "mjs", "py", "rb", "rs", "sql", "swift", "toml", "ts", "tsx", "txt", "xml",
    "yaml", "yml",
];

/// `parseUrlForLinkChip` restricted to `https://<host>[/<path>]` inputs produced
/// by the bare-domain branch: extracts the lowercase hostname (without port) and
/// applies the WHATWG port bound. Returns None when `new URL` would throw.
fn url_hostname(url_units: &[u16]) -> Option<Vec<u16>> {
    // Called with `https://` already prepended.
    let scheme_len = "https://".len();
    let rest = url_units.get(scheme_len..)?;
    // Host ends at '/', '?', '#'. Port ends host at ':'.
    let mut host_end = rest.len();
    for (i, &u) in rest.iter().enumerate() {
        if matches!(u, 0x2f | 0x3f | 0x23) {
            host_end = i;
            break;
        }
    }
    let host_port = &rest[..host_end];
    let host = match host_port.iter().position(|&u| u == 0x3a) {
        Some(ci) => {
            // Port must be 0..=65535 digits, or `new URL` throws.
            let port = &host_port[ci + 1..];
            if port.is_empty() || port.len() > 5 || !port.iter().all(|u| is_ascii_digit_u16(*u)) {
                return None;
            }
            let mut p: u64 = 0;
            for &d in port {
                p = p * 10 + u64::from(d - 0x30);
            }
            if p > 65535 {
                return None;
            }
            &host_port[..ci]
        }
        None => host_port,
    };
    if host.is_empty() {
        return None;
    }
    Some(js_to_lowercase(host).encode_utf16().collect())
}

/// `isLikelyBareDomainLink` (`linkChips.ts:77`) on the trimmed URL (no scheme).
fn is_likely_bare_domain_link(url: &[u16]) -> bool {
    // `new URL("https://" + url)`
    let mut full = u16_units("https://");
    full.extend_from_slice(url);
    let Some(host) = url_hostname(&full) else {
        return false;
    };
    // `parsed.hostname.toLowerCase()` — host already lowercased.
    if !host.contains(&0x2e) {
        return false;
    }
    let labels: Vec<&[u16]> = host.split(|&u| u == 0x2e).collect();
    let tld = labels.last().copied().unwrap_or(&[]);
    if !(2..=63).contains(&tld.len()) || !tld.iter().all(|&u| (0x61..=0x7a).contains(&u)) {
        return false;
    }
    for label in &labels {
        if label.is_empty() || label[0] == 0x2d || label[label.len() - 1] == 0x2d {
            return false;
        }
    }
    let has_path_or_query = url.iter().any(|&u| matches!(u, 0x2f | 0x3f | 0x23));
    let tld_str = units_str(tld);
    if !has_path_or_query && FILE_EXT_TLDS.contains(&tld_str.as_str()) {
        return false;
    }
    if !has_path_or_query
        && !host.starts_with(&u16_units("www.")[..])
        && !PUBLIC_TLDS.contains(&tld_str.as_str())
    {
        return false;
    }
    true
}

/// `HTTP_URL_PATTERN` = `/^https?:\/\//i`.
fn starts_with_http_scheme(units: &[u16]) -> bool {
    if units.len() < 8 {
        return false;
    }
    let mut n = 0;
    for (i, expected) in b"http".iter().enumerate() {
        if (units[i] | 0x20) != u16::from(*expected) {
            return false;
        }
        n = i + 1;
    }
    let mut idx = n;
    if units[idx] | 0x20 == 0x73 {
        idx += 1;
    }
    idx + 2 < units.len() && units[idx] == 0x3a && units[idx + 1] == 0x2f && units[idx + 2] == 0x2f
}

/// `normalizeComposerLinkUrl` (`linkChips.ts:101`). `rawUrl` is the regex match
/// already trimmed by `trimTrailingLinkPunctuation`? No — TS calls
/// `normalizeComposerLinkUrl(trimTrailingLinkPunctuation(match[0]))`, and the
/// function itself trims + punctuation-trims again. Input here is match[0].
fn normalize_composer_link_url(raw: &[u16]) -> Option<Vec<u16>> {
    let url = trim_trailing_link_punctuation_units(js_trim_units(raw));
    if url.is_empty() {
        return None;
    }
    if starts_with_http_scheme(url) {
        return Some(url.to_vec());
    }
    if is_likely_bare_domain_link(url) {
        let mut out = u16_units("https://");
        out.extend_from_slice(url);
        return Some(out);
    }
    None
}

// ---------- token matchers on &[u16] ----------

/// One inline token match from `collectInlineTokenMatches`.
#[derive(Clone)]
enum TokenMatch {
    Mention {
        path_units: Vec<u16>,
        start: usize,
        end: usize,
    },
    Skill {
        name_units: Vec<u16>,
        #[allow(dead_code)]
        prefix_slash: bool,
        start: usize,
        end: usize,
    },
    SlashCommand {
        command_lower_units: Vec<u16>,
        start: usize,
        end: usize,
    },
    AgentMention {
        alias_units: Vec<u16>,
        start: usize,
        end: usize,
    },
    Link {
        url_units: Vec<u16>,
        start: usize,
        end: usize,
    },
}

impl TokenMatch {
    fn start(&self) -> usize {
        match self {
            TokenMatch::Mention { start, .. }
            | TokenMatch::Skill { start, .. }
            | TokenMatch::SlashCommand { start, .. }
            | TokenMatch::AgentMention { start, .. }
            | TokenMatch::Link { start, .. } => *start,
        }
    }
    fn end(&self) -> usize {
        match self {
            TokenMatch::Mention { end, .. }
            | TokenMatch::Skill { end, .. }
            | TokenMatch::SlashCommand { end, .. }
            | TokenMatch::AgentMention { end, .. }
            | TokenMatch::Link { end, .. } => *end,
        }
    }
}

/// `(^|\s)` at position i: matches with prefix_len 0 at i==0 or 1 when units[i]
/// is JS whitespace.
fn match_prefix(u: &[u16], i: usize) -> Option<usize> {
    if i == 0 {
        Some(0)
    } else if i < u.len() && is_js_space(u[i]) {
        Some(1)
    } else {
        None
    }
}

/// `SKILL_TOKEN_REGEX` = `/(^|\s)([$/])([a-zA-Z][a-zA-Z0-9_:-]*)(?=\s)/g`
/// at position i -> (prefix_len, token_start, name_start, name_end).
fn match_skill_at(u: &[u16], i: usize) -> Option<(usize, usize, usize, usize)> {
    let prefix = match_prefix(u, i)?;
    let tok = i + prefix;
    if tok >= u.len() || !matches!(u[tok], 0x24 | 0x2f) {
        return None;
    }
    let name_start = tok + 1;
    if name_start >= u.len() || !is_ascii_alpha(u[name_start]) {
        return None;
    }
    let mut name_end = name_start + 1;
    while name_end < u.len()
        && (is_ascii_alnum(u[name_end]) || matches!(u[name_end], 0x5f | 0x3a | 0x2d))
    {
        name_end += 1;
    }
    if name_end < u.len() && is_js_space(u[name_end]) {
        Some((prefix, tok, name_start, name_end))
    } else {
        None
    }
}

/// `AGENT_MENTION_TOKEN_REGEX` = `/(^|\s)@([a-zA-Z0-9._-]+)(?=\()/g` at i.
fn match_agent_at(u: &[u16], i: usize) -> Option<(usize, usize, usize, usize)> {
    let prefix = match_prefix(u, i)?;
    let at = i + prefix;
    if at >= u.len() || u[at] != 0x40 {
        return None;
    }
    let alias_start = at + 1;
    let mut alias_end = alias_start;
    while alias_end < u.len()
        && (is_ascii_alnum(u[alias_end]) || matches!(u[alias_end], 0x2e | 0x5f | 0x2d))
    {
        alias_end += 1;
    }
    if alias_end == alias_start {
        return None;
    }
    if alias_end < u.len() && u[alias_end] == 0x28 {
        Some((prefix, at, alias_start, alias_end))
    } else {
        None
    }
}

/// The mention token body `@"((?:\\.|[^"\\])*)"|[^\s@]+` starting at `at`,
/// ending with `(?=\s)` lookahead. Returns (end, path_units) where path is the
/// decoded group2 or raw group3.
fn match_mention_body_at(u: &[u16], at: usize) -> Option<(usize, Vec<u16>)> {
    debug_assert!(u[at] == 0x40);
    let p = at + 1;
    // Quoted alternative first.
    if p < u.len() && u[p] == 0x22 {
        let mut q = p + 1;
        let mut ok = false;
        while q < u.len() {
            if u[q] == 0x22 {
                // closing quote at q; token ends at q+1 with lookahead \s
                let end = q + 1;
                if end < u.len() && is_js_space(u[end]) {
                    let decoded = decode_quoted_path(&u[p + 1..q]);
                    return Some((end, decoded));
                }
                ok = false;
                break;
            }
            if u[q] == 0x5c {
                if q + 1 < u.len() && !is_line_terminator(u[q + 1]) {
                    q += 2;
                    continue;
                }
                break;
            }
            q += 1;
        }
        let _ = ok;
        // fall through to unquoted alternative
    }
    // Unquoted `[^\s@]+`.
    let mut e = p;
    while e < u.len() && !is_js_space(u[e]) && u[e] != 0x40 {
        e += 1;
    }
    if e == p {
        return None;
    }
    if e < u.len() && is_js_space(u[e]) {
        Some((e, u[p..e].to_vec()))
    } else {
        None
    }
}

/// Mention regex `(^|\s)@(?:...)` at position i.
fn match_mention_at(u: &[u16], i: usize) -> Option<(usize, usize, usize, Vec<u16>)> {
    let prefix = match_prefix(u, i)?;
    let at = i + prefix;
    if at >= u.len() || u[at] != 0x40 {
        return None;
    }
    let (end, path) = match_mention_body_at(u, at)?;
    Some((prefix, at, end, path))
}

/// `decodeComposerMentionQuotedPath` = `path.replace(/\\(["\\])/g, "$1")`.
fn decode_quoted_path(units: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(units.len());
    let mut i = 0;
    while i < units.len() {
        if units[i] == 0x5c && i + 1 < units.len() && matches!(units[i + 1], 0x22 | 0x5c) {
            out.push(units[i + 1]);
            i += 2;
        } else {
            out.push(units[i]);
            i += 1;
        }
    }
    out
}

/// `composerMentionQuotedPathHasClosingQuote` (`composerMentions.ts:38`).
fn quoted_path_has_closing_quote(units: &[u16]) -> bool {
    let mut preceding = 0usize;
    for &u in units {
        if u == 0x5c {
            preceding += 1;
            continue;
        }
        if u == 0x22 && preceding.is_multiple_of(2) {
            return true;
        }
        preceding = 0;
    }
    false
}

/// Character class `[^\s<>()\[\]]` for link bodies.
fn is_link_body_char(u: u16) -> bool {
    !is_js_space(u) && !matches!(u, 0x3c | 0x3e | 0x28 | 0x29 | 0x5b | 0x5d)
}

/// `LINK_TOKEN_REGEX` first alternative: `https?:\/\/[^\s<>()\[\]]+(?=\s)`
/// (lowercase only — the token regex has no /i flag).
fn match_http_link_at(u: &[u16], i: usize) -> Option<usize> {
    if !u[i..].starts_with(&u16_units("http")[..]) {
        return None;
    }
    let mut p = i + 4;
    if p < u.len() && u[p] == 0x73 {
        p += 1;
    }
    if p + 2 >= u.len() || u[p] != 0x3a || u[p + 1] != 0x2f || u[p + 2] != 0x2f {
        return None;
    }
    let body_start = p + 3;
    let mut e = body_start;
    while e < u.len() && is_link_body_char(u[e]) {
        e += 1;
    }
    if e == body_start {
        return None;
    }
    // `(?=\s)`: the run stops at a non-body char; only a JS-space satisfies the
    // lookahead (backtracking can never expose a space inside the run).
    if e < u.len() && is_js_space(u[e]) {
        Some(e)
    } else {
        None
    }
}

/// Bare-domain alternative of LINK_TOKEN_SOURCE at i:
/// `(?<![A-Za-z0-9@._/-])(?:[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?\.)+[A-Za-z]{2,63}(?::\d{2,5})?(?:[/?#][^\s<>()\[\]]*)?(?=\s)`
fn match_bare_domain_at(u: &[u16], i: usize) -> Option<usize> {
    if i >= u.len() {
        return None;
    }
    // lookbehind
    if i > 0 && (is_ascii_alnum(u[i - 1]) || matches!(u[i - 1], 0x40 | 0x2e | 0x5f | 0x2f | 0x2d)) {
        return None;
    }
    // one or more `label.`
    let mut pos = i;
    let mut label_count = 0usize;
    loop {
        // label = [A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])? followed by '.'
        if pos >= u.len() || !is_ascii_alnum(u[pos]) {
            break;
        }
        // Find a prefix of the [A-Za-z0-9-]+ run that ends alnum and is followed
        // by '.', trying longest first (greedy `{0,61}`), max total length 63.
        let mut label_end = None;
        let max_l = 63usize;
        let mut l = 1usize;
        while l <= max_l && pos + l <= u.len() {
            let c = u[pos + l - 1];
            if !(is_ascii_alnum(c) || c == 0x2d) {
                break;
            }
            if is_ascii_alnum(c) && pos + l < u.len() && u[pos + l] == 0x2e {
                label_end = Some(pos + l); // longest so far
            }
            l += 1;
        }
        let Some(le) = label_end else {
            break;
        };
        label_count += 1;
        pos = le + 1;
    }
    if label_count == 0 {
        return None;
    }
    // TLD [A-Za-z]{2,63} — greedy; shrinking cannot satisfy the lookahead (the
    // characters released are letters, never spaces) so only the maximal run can
    // lead to a successful end.
    let mut tld_end = pos;
    while tld_end < u.len() && is_ascii_alpha(u[tld_end]) && tld_end - pos < 63 {
        tld_end += 1;
    }
    if tld_end - pos < 2 {
        return None;
    }
    let e = tld_end;
    // optional port `:\d{2,5}`
    if e < u.len() && u[e] == 0x3a {
        let mut d = e + 1;
        while d < u.len() && is_ascii_digit_u16(u[d]) && d - (e + 1) < 5 {
            d += 1;
        }
        // try longest first, then shorter port lengths, then no port — mirroring
        // JS backtracking.
        let mut port_ends: Vec<usize> = (2..=d - (e + 1).min(d)).map(|k| e + 1 + k).collect();
        port_ends.retain(|&pe| pe <= d && pe - (e + 1) >= 2);
        port_ends.sort_unstable_by(|a, b| b.cmp(a));
        port_ends.push(e); // no port
        for pe in port_ends {
            if let Some(end) = match_path_and_lookahead(u, pe) {
                return Some(end);
            }
        }
        return None;
    }
    match_path_and_lookahead(u, e)
}

/// After the (optional) port: optional path `[/?#][^\s<>()\[\]]*` then `(?=\s)`.
/// Mirrors JS backtracking order: longest path first, then shorter, then none.
fn match_path_and_lookahead(u: &[u16], e: usize) -> Option<usize> {
    if e < u.len() && matches!(u[e], 0x2f | 0x3f | 0x23) {
        // greedy run of link-body chars after the opener
        let mut run_end = e + 1;
        while run_end < u.len() && is_link_body_char(u[run_end]) {
            run_end += 1;
        }
        // Backtracking: the run shrinks from run_end toward e+1 looking for a
        // \s lookahead; interior chars are non-space so only run_end can win.
        if run_end < u.len() && is_js_space(u[run_end]) {
            return Some(run_end);
        }
        // Shrinking only reveals body chars or the opener — never \s — so the
        // only remaining option is no path at all.
    }
    if e < u.len() && is_js_space(u[e]) {
        return Some(e);
    }
    None
}

/// `(^|\s)` does NOT apply to links — LINK_TOKEN_SOURCE has no boundary prefix;
/// matches may start anywhere (bare domain has its own lookbehind).
fn match_link_at(u: &[u16], i: usize) -> Option<usize> {
    if i >= u.len() {
        return None;
    }
    match_http_link_at(u, i).or_else(|| match_bare_domain_at(u, i))
}

// ---------- `collectInlineTokenMatches` (`composer-editor-mentions.ts:183`) ----------

fn in_ranges(ranges: &[(usize, usize)], pos: usize) -> bool {
    ranges.iter().any(|&(s, e)| pos >= s && pos < e)
}

fn collect_inline_token_matches(u: &[u16]) -> Vec<TokenMatch> {
    let mut matches: Vec<TokenMatch> = Vec::new();
    let mut reserved_ranges: Vec<(usize, usize)> = Vec::new();

    // Links first: reserve ranges so mentions/skills can't match inside a URL.
    let mut pos = 0;
    while pos < u.len() {
        if let Some(end) = match_link_at(u, pos) {
            let raw = &u[pos..end];
            let trimmed = trim_trailing_link_punctuation_units(raw);
            if let Some(url) = normalize_composer_link_url(raw) {
                let end = pos + trimmed.len();
                let start = pos;
                reserved_ranges.push((start, end));
                matches.push(TokenMatch::Link {
                    url_units: url,
                    start,
                    end,
                });
                pos = end;
                continue;
            }
        }
        pos += 1;
    }

    // Agent mentions.
    let mut agent_ranges: Vec<(usize, usize)> = Vec::new();
    pos = 0;
    while pos < u.len() {
        if let Some((prefix, at, alias_start, alias_end)) = match_agent_at(u, pos) {
            let _ = prefix;
            let start = at;
            let end = at + 1 + (alias_end - alias_start);
            if !in_ranges(&reserved_ranges, start) {
                let alias = u[alias_start..alias_end].to_vec();
                if resolve_agent_alias_color(&alias).is_some() {
                    agent_ranges.push((start, end));
                    matches.push(TokenMatch::AgentMention {
                        alias_units: alias,
                        start,
                        end,
                    });
                }
            }
            pos = alias_end;
            continue;
        }
        pos += 1;
    }
    let is_inside_agent = |pos: usize| agent_ranges.iter().any(|&(s, e)| pos >= s && pos < e);

    // Mentions.
    pos = 0;
    while pos < u.len() {
        if let Some((_prefix, at, end, path)) = match_mention_at(u, pos) {
            let start = at;
            if !is_inside_agent(start) && !in_ranges(&reserved_ranges, start) && !path.is_empty() {
                matches.push(TokenMatch::Mention {
                    path_units: path,
                    start,
                    end,
                });
            }
            let next = end;
            pos = if next > pos { next } else { pos + 1 };
            continue;
        }
        pos += 1;
    }

    // Skills.
    pos = 0;
    while pos < u.len() {
        if let Some((_prefix, tok, name_start, name_end)) = match_skill_at(u, pos) {
            let start = tok;
            let end = name_end;
            if !is_inside_agent(start) && !in_ranges(&reserved_ranges, start) {
                let name = &u[name_start..name_end];
                let prefix_slash = u[tok] == 0x2f;
                let normalized = js_to_lowercase(name);
                if prefix_slash && is_built_in_command(&normalized) {
                    if is_chip_command(&normalized) {
                        matches.push(TokenMatch::SlashCommand {
                            command_lower_units: normalized.encode_utf16().collect(),
                            start,
                            end,
                        });
                    }
                    // other built-ins stay plain text
                } else {
                    matches.push(TokenMatch::Skill {
                        name_units: name.to_vec(),
                        prefix_slash,
                        start,
                        end,
                    });
                }
            }
            pos = end;
            continue;
        }
        pos += 1;
    }

    // `matches.sort((a, b) => a.start - b.start)` — stable.
    matches.sort_by_key(|m| m.start() as i64);
    matches
}

// ---------- segments ----------

#[derive(Clone)]
enum Seg {
    Text { len: usize },
    Mention { token_len: usize },
    Skill { expanded_len: usize },
    SlashCommand { expanded_len: usize },
    AgentMention { expanded_len: usize },
    Link { expanded_len: usize },
    TerminalContext,
}

impl Seg {
    fn collapsed_len(&self) -> usize {
        match self {
            Seg::Text { len } => *len,
            _ => 1,
        }
    }
    fn expanded_len(&self) -> usize {
        match self {
            Seg::Text { len } => *len,
            Seg::Mention { token_len } => *token_len,
            Seg::Skill { expanded_len }
            | Seg::SlashCommand { expanded_len }
            | Seg::AgentMention { expanded_len }
            | Seg::Link { expanded_len } => *expanded_len,
            Seg::TerminalContext => 1,
        }
    }
    fn is_inline_token(&self) -> bool {
        !matches!(self, Seg::Text { .. })
    }
}

/// `pushTextSegment`: merges with a trailing text segment.
fn push_text_segment(segments: &mut Vec<Seg>, text_len: usize) {
    if text_len == 0 {
        return;
    }
    if let Some(Seg::Text { len }) = segments.last_mut() {
        *len += text_len;
        return;
    }
    segments.push(Seg::Text { len: text_len });
}

/// `splitTextIntoPromptSegments` with includeTrailingTokenAtEnd: false,
/// includeSlashCommandChips: true, mentionReferences: [].
fn split_text_into_prompt_segments(u: &[u16]) -> Vec<Seg> {
    let mut segments: Vec<Seg> = Vec::new();
    if u.is_empty() {
        return segments;
    }
    let matches = collect_inline_token_matches(u);
    let mut cursor = 0usize;
    for m in &matches {
        let start = m.start();
        if start < cursor {
            continue;
        }
        if start > cursor {
            push_text_segment(&mut segments, start - cursor);
        }
        match m {
            TokenMatch::Link { url_units, .. } => segments.push(Seg::Link {
                expanded_len: url_units.len(),
            }),
            TokenMatch::AgentMention { alias_units, .. } => segments.push(Seg::AgentMention {
                expanded_len: alias_units.len() + 1,
            }),
            TokenMatch::Mention {
                start,
                end,
                path_units,
            } => {
                let token_len = end - start;
                // `segment.tokenLength ?? path.length + 1` — tokenLength is
                // always set by the split; keep the fallback anyway.
                let expanded = if token_len > 0 {
                    token_len
                } else {
                    path_units.len() + 1
                };
                segments.push(Seg::Mention {
                    token_len: expanded,
                })
            }
            TokenMatch::SlashCommand {
                command_lower_units,
                ..
            } => segments.push(Seg::SlashCommand {
                expanded_len: command_lower_units.len() + 1,
            }),
            TokenMatch::Skill { name_units, .. } => segments.push(Seg::Skill {
                expanded_len: name_units.len() + 1,
            }),
        }
        cursor = m.end();
    }
    if cursor < u.len() {
        push_text_segment(&mut segments, u.len() - cursor);
    }
    segments
}

/// `splitPromptIntoComposerSegments(prompt)` — terminalContexts and
/// mentionReferences default to [].
fn split_prompt_into_composer_segments(text: &str) -> Vec<Seg> {
    let u = u16_units(text);
    if u.is_empty() {
        return Vec::new();
    }
    let mut segments: Vec<Seg> = Vec::new();
    let mut text_cursor = 0usize;
    for index in 0..u.len() {
        if u[index] != FFFC {
            continue;
        }
        if index > text_cursor {
            segments.extend(split_text_into_prompt_segments(&u[text_cursor..index]));
        }
        segments.push(Seg::TerminalContext);
        text_cursor = index + 1;
    }
    if text_cursor < u.len() {
        segments.extend(split_text_into_prompt_segments(&u[text_cursor..]));
    }
    segments
}

/// `collapsedSegmentLength`.
fn collapsed_segment_length(seg: &Seg) -> usize {
    seg.collapsed_len()
}

/// `clampCollapsedComposerCursorForSegments`.
fn clamp_collapsed_for_segments(segments: &[Seg], cursor_input: f64) -> f64 {
    let collapsed_length: usize = segments.iter().map(collapsed_segment_length).sum();
    if !cursor_input.is_finite() {
        return collapsed_length as f64;
    }
    js_max(0.0, js_min(collapsed_length as f64, cursor_input.floor()))
}

// ---------- exports ----------

/// Port of `stripComposerTriggerText` (`apps/web/src/composer-logic.ts:21`).
pub fn strip_composer_trigger_text(
    text: String,
    trigger: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let units = u16_units(&text);
    // `if (!trigger) return text` — JS falsy check.
    let trigger = match trigger {
        None => return Ok(crate::js::utf16_string(&units)),
        Some(t) => t,
    };
    let falsy = match &trigger {
        _ if crate::js::is_undefined(&trigger) => true,
        Value::Null => true,
        Value::Bool(b) => !*b,
        Value::Number(n) => n.as_f64().is_none_or(|x| x == 0.0 || x.is_nan()),
        Value::String(s) => s.is_empty(),
        _ => false,
    };
    if falsy {
        return Ok(crate::js::utf16_string(&units));
    }
    // `trigger.rangeStart` / `trigger.rangeEnd` — only objects carry fields.
    let (range_start, range_end) = match &trigger {
        Value::Object(o) => (
            o.get("rangeStart")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
            o.get("rangeEnd")
                .cloned()
                .unwrap_or_else(crate::js::undefined),
        ),
        _ => (crate::js::undefined(), crate::js::undefined()),
    };
    // `text.slice(0, trigger.rangeStart)` — explicit `undefined` end = whole.
    let head = js_slice_to(&units, 0.0, &range_start);
    // `text.slice(trigger.rangeEnd)` — undefined start = 0.
    let tail = js_slice_from(&units, js_number_value(&range_end));
    let mut out = head.clone();
    out.extend_from_slice(tail);
    Ok(crate::js::utf16_string(&out))
}

/// Port of `expandCollapsedComposerCursor` (`apps/web/src/composer-logic.ts:81`).
pub fn expand_collapsed_composer_cursor(text: String, cursor_input: f64) -> Result<f64, String> {
    let len = u16_units(&text).len();
    let collapsed_cursor = clamp_cursor(len, cursor_input);
    let segments = split_prompt_into_composer_segments(&text);
    if segments.is_empty() {
        return Ok(collapsed_cursor);
    }
    let mut remaining = collapsed_cursor;
    let mut expanded_cursor = 0.0;
    for segment in &segments {
        if let Seg::Text { len } = segment {
            let segment_length = *len as f64;
            if remaining <= segment_length {
                return Ok(expanded_cursor + remaining);
            }
            remaining -= segment_length;
            expanded_cursor += segment_length;
            continue;
        }
        if matches!(segment, Seg::TerminalContext) {
            if remaining <= 1.0 {
                return Ok(expanded_cursor + remaining);
            }
            remaining -= 1.0;
            expanded_cursor += 1.0;
            continue;
        }
        let expanded_length = segment.expanded_len() as f64;
        if remaining <= 1.0 {
            return Ok(expanded_cursor
                + if remaining == 0.0 {
                    0.0
                } else {
                    expanded_length
                });
        }
        remaining -= 1.0;
        expanded_cursor += expanded_length;
    }
    Ok(expanded_cursor)
}

/// Port of `clampCollapsedComposerCursor` (`apps/web/src/composer-logic.ts:180`).
pub fn clamp_collapsed_composer_cursor(text: String, cursor_input: f64) -> Result<f64, String> {
    let segments = split_prompt_into_composer_segments(&text);
    Ok(clamp_collapsed_for_segments(&segments, cursor_input))
}

/// Port of `collapseExpandedComposerCursor` (`apps/web/src/composer-logic.ts:187`).
pub fn collapse_expanded_composer_cursor(text: String, cursor_input: f64) -> Result<f64, String> {
    let len = u16_units(&text).len();
    let expanded_cursor = clamp_cursor(len, cursor_input);
    let segments = split_prompt_into_composer_segments(&text);
    if segments.is_empty() {
        return Ok(expanded_cursor);
    }
    let mut remaining = expanded_cursor;
    let mut collapsed_cursor = 0.0;
    for segment in &segments {
        if let Seg::Text { len } = segment {
            let segment_length = *len as f64;
            if remaining <= segment_length {
                return Ok(collapsed_cursor + remaining);
            }
            remaining -= segment_length;
            collapsed_cursor += segment_length;
            continue;
        }
        if matches!(segment, Seg::TerminalContext) {
            if remaining <= 1.0 {
                return Ok(collapsed_cursor + remaining);
            }
            remaining -= 1.0;
            collapsed_cursor += 1.0;
            continue;
        }
        let expanded_length = segment.expanded_len() as f64;
        if remaining == 0.0 {
            return Ok(collapsed_cursor);
        }
        if remaining <= expanded_length {
            return Ok(collapsed_cursor + 1.0);
        }
        remaining -= expanded_length;
        collapsed_cursor += 1.0;
    }
    Ok(collapsed_cursor)
}

/// Port of `isCollapsedCursorAdjacentToInlineToken` (`composer-logic.ts:280`).
pub fn is_collapsed_cursor_adjacent_to_inline_token(
    text: String,
    cursor_input: f64,
    direction: String,
) -> Result<bool, String> {
    let segments = split_prompt_into_composer_segments(&text);
    if !segments.iter().any(Seg::is_inline_token) {
        return Ok(false);
    }
    let cursor = clamp_collapsed_for_segments(&segments, cursor_input);
    let mut collapsed_offset = 0.0f64;
    for segment in &segments {
        if segment.is_inline_token() {
            if direction == "left" && cursor == collapsed_offset + 1.0 {
                return Ok(true);
            }
            if direction == "right" && cursor == collapsed_offset {
                return Ok(true);
            }
        }
        collapsed_offset += collapsed_segment_length(segment) as f64;
    }
    Ok(false)
}

/// `tokenStartForCursor` (`composer-logic.ts:55`).
fn token_start_for_cursor(u: &[u16], cursor: usize) -> usize {
    let mut index = cursor as i64 - 1;
    while index >= 0 && !is_composer_ws(u[index as usize]) {
        index -= 1;
    }
    (index + 1) as usize
}

/// `slashTokenStartForCursor` (`composer-logic.ts:68`).
fn slash_token_start_for_cursor(u: &[u16], line_start: usize, cursor: usize) -> i64 {
    let mut slash_start = -1i64;
    for index in line_start..cursor {
        if u[index] != 0x2f {
            continue;
        }
        if index == line_start || is_composer_ws(u[index - 1]) {
            slash_start = index as i64;
        }
    }
    slash_start
}

/// `lastIndexOf(needle, from)` on units — greatest start index <= `from`.
fn last_index_of_from(u: &[u16], needle: &[u16], from: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(from.min(u.len()));
    }
    if needle.len() > u.len() {
        return None;
    }
    let limit = (from + 1).min(u.len().saturating_sub(needle.len()) + 1);
    let mut found = None;
    for i in 0..limit {
        if i + needle.len() <= u.len() && &u[i..i + needle.len()] == needle {
            found = Some(i);
        }
    }
    found
}

/// Port of `detectComposerTrigger` (`apps/web/src/composer-logic.ts:308`).
pub fn detect_composer_trigger(
    text: String,
    cursor_input: f64,
) -> Result<Option<serde_json::Value>, String> {
    let u = u16_units(&text);
    let len = u.len();
    let cursor = clamp_cursor(len, cursor_input) as usize;
    // `text.lastIndexOf("\n", Math.max(0, cursor - 1)) + 1`
    let from = cursor.saturating_sub(1);
    let line_start = last_index_of_from(&u, &u16_units("\n"), from).map_or(0, |i| i + 1);
    let line_prefix = &u[line_start.min(cursor)..cursor];

    let slash_start = slash_token_start_for_cursor(&u, line_start, cursor);
    if slash_start != -1 {
        let slash_start = slash_start as usize;
        let region = &u[slash_start..cursor];
        // `/^\/(\S*)$/`
        if !region.is_empty() && region[0] == 0x2f && region[1..].iter().all(|&c| !is_js_space(c)) {
            let command_query = &region[1..];
            if command_query.contains(&0x2f) {
                return Ok(None);
            }
            if js_to_lowercase(command_query) == "model" {
                return Ok(Some(json!({
                    "kind": "slash-model",
                    "query": "",
                    "rangeStart": slash_start,
                    "rangeEnd": cursor,
                })));
            }
            return Ok(Some(json!({
                "kind": "slash-command",
                "query": units_str(command_query),
                "rangeStart": slash_start,
                "rangeEnd": cursor,
            })));
        }
        // `/^\/model(?:\s+(.*))?$/`
        if region.len() >= 5 && region[..5] == u16_units("/model")[..] {
            if region.len() == 5 {
                return Ok(Some(json!({
                    "kind": "slash-model",
                    "query": "",
                    "rangeStart": slash_start,
                    "rangeEnd": cursor,
                })));
            }
            // `\s+` then `.*` (no line terminators) to end.
            let mut j = 5;
            while j < region.len() && is_js_space(region[j]) {
                j += 1;
            }
            if j > 5 && region[j..].iter().all(|&c| !is_line_terminator(c)) {
                let query = units_str(js_trim_units(&region[j..]));
                return Ok(Some(json!({
                    "kind": "slash-model",
                    "query": query,
                    "rangeStart": slash_start,
                    "rangeEnd": cursor,
                })));
            }
        }
    }

    let token_start = token_start_for_cursor(&u, cursor);
    let token = &u[token_start..cursor];
    if token.first() == Some(&0x24) {
        return Ok(Some(json!({
            "kind": "skill",
            "query": units_str(&token[1..]),
            "rangeStart": token_start,
            "rangeEnd": cursor,
        })));
    }

    // Unclosed `@"..."` mention spans whitespace.
    let at_quote = u16_units("@\"");
    if let Some(qi) = last_index_of_from(line_prefix, &at_quote, line_prefix.len()) {
        let after_open = &line_prefix[qi + 2..];
        if !quoted_path_has_closing_quote(after_open) {
            return Ok(Some(json!({
                "kind": "mention",
                "query": units_str(&decode_quoted_path(after_open)),
                "rangeStart": line_start + qi,
                "rangeEnd": cursor,
            })));
        }
    }

    if token.first() != Some(&0x40) {
        return Ok(None);
    }
    // Anchor to the last `@` in the whitespace-bounded token.
    let last_at = last_index_of_from(token, &u16_units("@"), token.len() - 1).unwrap_or(0);
    let mention_start = token_start + last_at;
    let mention_token = &token[last_at..];
    // `/^@[^()\s@]*$/`
    let ok = mention_token[0] == 0x40
        && mention_token[1..]
            .iter()
            .all(|&c| !(is_js_space(c) || matches!(c, 0x28 | 0x29 | 0x40)));
    if !ok {
        return Ok(None);
    }
    Ok(Some(json!({
        "kind": "mention",
        "query": units_str(&mention_token[1..]),
        "rangeStart": mention_start,
        "rangeEnd": cursor,
    })))
}

/// Port of `parseStandaloneComposerSlashCommand` (`composer-logic.ts:406`).
pub fn parse_standalone_composer_slash_command(
    text: String,
) -> Result<Option<serde_json::Value>, String> {
    let u = u16_units(&text);
    let trimmed = js_trim_units(&u);
    // `/^\/([a-z-]+)\s*$/i` — `i` also folds ſ/K into the class, but neither can
    // produce a built-in name, so ASCII case-insensitivity is equivalent here.
    if trimmed.len() < 2 || trimmed[0] != 0x2f {
        return Ok(None);
    }
    let mut e = 1;
    while e < trimmed.len()
        && (is_ascii_alpha(trimmed[e]) || (0x41..=0x5a).contains(&trimmed[e]) || trimmed[e] == 0x2d)
    {
        e += 1;
    }
    // `[a-z-]` with /i = ASCII letters plus '-'. (is_ascii_alpha already covers
    // both cases; the uppercase check is redundant but harmless.)
    let name_end = e;
    if name_end == 1 {
        return Ok(None);
    }
    if !trimmed[name_end..].iter().all(|&c| is_js_space(c)) {
        return Ok(None);
    }
    let command = js_to_lowercase(&trimmed[1..name_end]);
    if !is_built_in_command(&command) || command == "model" {
        return Ok(None);
    }
    Ok(Some(Value::String(command)))
}

/// Port of `replaceTextRange` (`apps/web/src/composer-logic.ts:420`).
pub fn replace_text_range(
    text: String,
    range_start: f64,
    range_end: f64,
    replacement: String,
) -> Result<serde_json::Value, String> {
    let u = u16_units(&text);
    let len = u.len() as f64;
    let safe_start = js_max(0.0, js_min(len, range_start));
    let safe_end = js_max(safe_start, js_min(len, range_end));
    let head = js_slice_from(&u, 0.0_f64.min(safe_start) * 0.0 + safe_start.min(len));
    // Use the clamped integer positions for slicing (slice re-clamps anyway).
    let s_idx = safe_start as usize;
    let e_idx = safe_end as usize;
    let _ = head;
    let mut next = String::new();
    next.push_str(&units_str(&u[..s_idx.min(u.len())]));
    next.push_str(&replacement);
    next.push_str(&units_str(&u[e_idx.min(u.len())..]));
    let repl_len = u16_units(&replacement).len() as f64;
    Ok(json!({
        "text": next,
        "cursor": crate::js::number(safe_start + repl_len),
    }))
}
