//! The Rust port of `apps/server/src/processArgumentRedaction.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

fn is_undefined(v: &Value) -> bool {
    crate::js::is_undefined(v)
}

fn asc(c: u16) -> Option<u8> {
    u8::try_from(c).ok()
}

fn is_al(c: u16) -> bool {
    matches!(asc(c), Some(x) if x.is_ascii_lowercase())
}
fn is_au(c: u16) -> bool {
    matches!(asc(c), Some(x) if x.is_ascii_uppercase())
}
fn is_dig(c: u16) -> bool {
    matches!(asc(c), Some(x) if x.is_ascii_digit())
}
fn is_name_start(c: u16) -> bool {
    is_al(c) || is_au(c) || c == b'_' as u16
}
fn is_name_char(c: u16) -> bool {
    is_name_start(c) || is_dig(c)
}
fn is_punct(c: u16, set: &[u8]) -> bool {
    matches!(asc(c), Some(x) if set.contains(&x))
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

/// JS `\s` on a single UTF-16 code unit (incl. U+FEFF, excl. U+0085).
fn js_ws(c: u16) -> bool {
    matches!(
        c,
        0x09..=0x0d | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f
            | 0x3000 | 0xfeff
    )
}

fn u16_slice(u: &[u16], a: usize, b: usize) -> Vec<u16> {
    u[a.min(u.len())..b.min(u.len())].to_vec()
}

fn u16_to_string(u: &[u16]) -> String {
    String::from_utf16_lossy(u)
}

fn starts_with_u16(u: &[u16], pat: &str, at: usize) -> bool {
    let p: Vec<u16> = pat.encode_utf16().collect();
    at + p.len() <= u.len() && u[at..at + p.len()] == p[..]
}

// --- ENV_ASSIGNMENT_START_PATTERN matchAll ---
// /(^|[\s;&|()'"])((?:--?(?:e|env|environment|set-?env)=)?)([A-Za-z_][A-Za-z0-9_]*)(?:\+)?=/g

struct EnvMatch {
    boundary: u16, // 0 when the `^` alternative matched
    name: String,
    value_start: usize,
}

fn is_env_boundary(c: u16) -> bool {
    js_ws(c) || is_punct(c, b";&|()'\"")
}

fn env_matches(args: &[u16]) -> Vec<EnvMatch> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < args.len() {
        let (boundary, j0) = if i == 0 {
            (0u16, 0usize)
        } else if is_env_boundary(args[i]) {
            (args[i], i + 1)
        } else {
            i += 1;
            continue;
        };
        // optional flag prefix: --?(e|env|environment|set-?env)=
        let mut j = j0;
        if j < args.len() && args[j] == b'-' as u16 {
            let mut k = j + 1;
            if k < args.len() && args[k] == b'-' as u16 {
                k += 1;
            }
            let flag_names: [&str; 5] = ["e", "env", "environment", "setenv", "set-env"];
            let mut matched_flag = false;
            for name in flag_names {
                let nu: Vec<u16> = name.encode_utf16().collect();
                if k + nu.len() < args.len()
                    && args[k..k + nu.len()] == nu[..]
                    && args[k + nu.len()] == b'=' as u16
                {
                    j = k + nu.len() + 1;
                    matched_flag = true;
                    break;
                }
            }
            if !matched_flag {
                j = j0;
            }
        }
        // name: [A-Za-z_][A-Za-z0-9_]*
        if j >= args.len() || !is_name_start(args[j]) {
            i += 1;
            continue;
        }
        let name_start = j;
        j += 1;
        while j < args.len() && is_name_char(args[j]) {
            j += 1;
        }
        let name = u16_to_string(&args[name_start..j]);
        // (?:\+)?=
        if j < args.len() && args[j] == b'+' as u16 {
            j += 1;
        }
        if j >= args.len() || args[j] != b'=' as u16 {
            i += 1;
            continue;
        }
        j += 1;
        out.push(EnvMatch {
            boundary,
            name,
            value_start: j,
        });
        i = j;
    }
    out
}

const SENSITIVE_EXACT: &[&str] = &[
    "APIKEY",
    "API_KEY",
    "ACCESS_TOKEN",
    "AUTH",
    "AUTH_TOKEN",
    "AUTHORIZATION",
    "CREDENTIAL",
    "CREDENTIALS",
    "KEY",
    "MYSQL_PWD",
    "PASS",
    "PASSWORD",
    "PASSPHRASE",
    "PGPASSWORD",
    "REDISCLI_AUTH",
    "SECRET",
    "TOKEN",
];
const SENSITIVE_SUFFIX: &[&str] = &[
    "APIKEY",
    "API_KEY",
    "ACCESS_TOKEN",
    "AUTH",
    "AUTH_TOKEN",
    "AUTHORIZATION",
    "CREDENTIAL",
    "CREDENTIALS",
    "KEY",
    "PASS",
    "PASSWORD",
    "PASSPHRASE",
    "SECRET",
    "TOKEN",
];

fn is_sensitive_environment_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    let normalized = name.to_uppercase();
    if SENSITIVE_EXACT.contains(&normalized.as_str()) {
        return true;
    }
    SENSITIVE_SUFFIX
        .iter()
        .any(|s| normalized.ends_with(&format!("_{s}")))
}

/// /^["']?[a-z][a-z0-9+.-]*:\/\/[^/?#\s]+@/i tested on a UTF-16 slice.
fn url_credentials_at_start(u: &[u16], start: usize) -> bool {
    let mut i = start;
    if i < u.len() && (u[i] == b'"' as u16 || u[i] == b'\'' as u16) {
        i += 1;
    }
    if i >= u.len() || !(is_al(u[i]) || is_au(u[i])) {
        return false;
    }
    i += 1;
    while i < u.len() && (is_al(u[i]) || is_au(u[i]) || is_dig(u[i]) || is_punct(u[i], b"+.-")) {
        i += 1;
    }
    if !starts_with_u16(u, "://", i) {
        return false;
    }
    i += 3;
    let cred_start = i;
    while i < u.len() && !is_punct(u[i], b"/?#") && !js_ws(u[i]) {
        i += 1;
    }
    // `[^/?#\s]+@`: the '@' is inside the run, after >=1 char.
    (cred_start + 1..i).any(|p| u[p] == b'@' as u16)
}

fn redact_sensitive_environment_remainder(args: &[u16]) -> Vec<u16> {
    for m in env_matches(args) {
        let name_sensitive = is_sensitive_environment_name(&m.name);
        if !name_sensitive && !url_credentials_at_start(args, m.value_start) {
            continue;
        }
        let mut out = u16_slice(args, 0, m.value_start);
        out.extend("[redacted]".encode_utf16());
        return out;
    }
    args.to_vec()
}

// --- boundedShellAssignmentValueEnd ---

const MAX_SHELL_ASSIGNMENT_SCAN_CHARS: usize = 16_384;
const MAX_SHELL_EXPANSION_DEPTH: usize = 128;

#[derive(Clone, Copy, PartialEq)]
enum ExpKind {
    Arithmetic,
    Command,
    Group,
    LegacyArithmetic,
    Parameter,
}

#[derive(Clone)]
struct CaseState {
    phase_patterns: bool,
    subject_seen: bool,
    expect_pattern: bool,
}

struct Expansion {
    close: &'static str,
    kind: ExpKind,
    quote: u16, // 0 = none, else the quote char
    command_position: bool,
    cases: Vec<CaseState>,
    active_command: Option<usize>,
    active_non_group: Option<usize>,
}

struct ExpAtCursor {
    close: &'static str,
    kind: ExpKind,
    length: usize,
}

fn expansion_at_cursor(args: &[u16], index: usize) -> Option<ExpAtCursor> {
    if starts_with_u16(args, "$((", index) {
        return Some(ExpAtCursor {
            close: "))",
            kind: ExpKind::Arithmetic,
            length: 3,
        });
    }
    if starts_with_u16(args, "$(", index) {
        return Some(ExpAtCursor {
            close: ")",
            kind: ExpKind::Command,
            length: 2,
        });
    }
    if starts_with_u16(args, "${", index) {
        return Some(ExpAtCursor {
            close: "}",
            kind: ExpKind::Parameter,
            length: 2,
        });
    }
    if starts_with_u16(args, "$[", index) {
        return Some(ExpAtCursor {
            close: "]",
            kind: ExpKind::LegacyArithmetic,
            length: 2,
        });
    }
    None
}

const COMMAND_LIST_PREFIX_WORDS: &[&str] =
    &["do", "elif", "else", "if", "then", "time", "until", "while"];

fn bounded_shell_assignment_value_end(args: &[u16], value_start: usize, boundary: u16) -> usize {
    let outer_boundary_quote = if boundary == b'\'' as u16 || boundary == b'"' as u16 {
        boundary
    } else {
        0
    };
    let mut word_quote: u16 = outer_boundary_quote;
    let mut expansions: Vec<Expansion> = Vec::new();
    let mut index = value_start;

    macro_rules! active_command_idx {
        () => {
            expansions.last().and_then(|e| e.active_command)
        };
    }
    macro_rules! mark_subject_and_not_command {
        () => {
            if let Some(ci) = active_command_idx!() {
                if let Some(c) = expansions[ci].cases.last_mut() {
                    if !c.phase_patterns {
                        c.subject_seen = true;
                    }
                }
                expansions[ci].command_position = false;
            }
        };
    }
    macro_rules! push_expansion {
        ($close:expr, $kind:expr) => {{
            let (ac, ang) = match expansions.last() {
                Some(p) => (p.active_command, p.active_non_group),
                None => (None, None),
            };
            let own = expansions.len();
            let mut e = Expansion {
                close: $close,
                kind: $kind,
                quote: 0,
                command_position: $kind == ExpKind::Command,
                cases: Vec::new(),
                active_command: ac,
                active_non_group: ang,
            };
            if e.kind == ExpKind::Command {
                e.active_command = Some(own);
            }
            if e.kind != ExpKind::Group {
                e.active_non_group = Some(own);
            }
            expansions.push(e);
        }};
    }

    while index < args.len() {
        if index - value_start >= MAX_SHELL_ASSIGNMENT_SCAN_CHARS
            || expansions.len() > MAX_SHELL_EXPANSION_DEPTH
        {
            return args.len();
        }
        let character = args[index];

        if character == b'\\' as u16 {
            if args.get(index + 1).copied() != Some(0x0a) {
                mark_subject_and_not_command!();
            }
            index += 2;
            continue;
        }

        if let Some(top_idx) = expansions.len().checked_sub(1) {
            // inside an expansion
            if expansions[top_idx].quote != 0 {
                if character == expansions[top_idx].quote {
                    expansions[top_idx].quote = 0;
                    index += 1;
                    continue;
                }
                let nested = if expansions[top_idx].quote != b'\'' as u16 {
                    expansion_at_cursor(args, index)
                } else {
                    None
                };
                if let Some(n) = nested {
                    if n.kind == ExpKind::Command {
                        return args.len();
                    }
                    push_expansion!(n.close, n.kind);
                    index += n.length;
                    continue;
                }
                index += 1;
                continue;
            }
            if character == b'\'' as u16 || character == b'"' as u16 || character == b'`' as u16 {
                mark_subject_and_not_command!();
                expansions[top_idx].quote = character;
                index += 1;
                continue;
            }
            if active_command_idx!().is_some() && starts_with_u16(args, "<<", index) {
                return args.len();
            }
            let active_case_idx = if expansions[top_idx].kind == ExpKind::Command {
                expansions[top_idx].cases.len().checked_sub(1)
            } else {
                None
            };
            let in_patterns_no_expect = active_case_idx.is_some_and(|ci| {
                let c = &expansions[top_idx].cases[ci];
                c.phase_patterns && !c.expect_pattern
            });
            if in_patterns_no_expect
                && (starts_with_u16(args, ";;&", index)
                    || starts_with_u16(args, ";;", index)
                    || starts_with_u16(args, ";&", index))
            {
                let ci = active_case_idx.unwrap_or(0);
                expansions[top_idx].cases[ci].expect_pattern = true;
                expansions[top_idx].command_position = true;
                index += if starts_with_u16(args, ";;&", index) {
                    3
                } else {
                    2
                };
                continue;
            }
            if starts_with_u16(args, expansions[top_idx].close, index) {
                let expect =
                    active_case_idx.is_some_and(|ci| expansions[top_idx].cases[ci].expect_pattern);
                if expansions[top_idx].kind == ExpKind::Command && expect {
                    let ci = active_case_idx.unwrap_or(0);
                    expansions[top_idx].cases[ci].expect_pattern = false;
                    expansions[top_idx].command_position = true;
                    index += 1;
                    continue;
                }
                let close_len = expansions[top_idx].close.len();
                expansions.pop();
                index += close_len;
                continue;
            }
            if let Some(n) = expansion_at_cursor(args, index) {
                if n.kind == ExpKind::Command {
                    return args.len();
                }
                mark_subject_and_not_command!();
                push_expansion!(n.close, n.kind);
                index += n.length;
                continue;
            }
            let prev_boundary_ok = index == value_start
                || args
                    .get(index - 1)
                    .is_some_and(|&p| js_ws(p) || is_punct(p, b";&|()"));
            if active_command_idx!().is_some() && character == b'#' as u16 && prev_boundary_ok {
                while index < args.len() && args[index] != 0x0a {
                    index += 1;
                }
                if let Some(ci) = active_command_idx!() {
                    expansions[ci].command_position = true;
                }
                continue;
            }
            let shell_expansion = expansions
                .last()
                .and_then(|e| e.active_non_group)
                .map(|i| expansions[i].kind);
            let groups_bracket = character == b'[' as u16
                && matches!(
                    shell_expansion,
                    Some(ExpKind::Arithmetic) | Some(ExpKind::LegacyArithmetic)
                );
            if character == b'(' as u16 || character == b'{' as u16 || groups_bracket {
                let (close, kind) = if character == b'(' as u16 {
                    (")", ExpKind::Group)
                } else if character == b'{' as u16 {
                    ("}", ExpKind::Group)
                } else {
                    ("]", ExpKind::Group)
                };
                push_expansion!(close, kind);
                index += 1;
                continue;
            }
            // subject-case tracking
            if let Some(ci) = active_command_idx!() {
                let is_subject_char = !(js_ws(character)
                    || is_al(character)
                    || is_au(character)
                    || character == b'_' as u16);
                let cidx = expansions[ci].cases.len().checked_sub(1);
                if let Some(cc) = cidx
                    && !expansions[ci].cases[cc].phase_patterns
                    && is_subject_char
                {
                    expansions[ci].cases[cc].subject_seen = true;
                    expansions[ci].command_position = false;
                }
            }
            if is_name_start(character) {
                let mut word_end = index + 1;
                while word_end < args.len() && is_name_char(args[word_end]) {
                    word_end += 1;
                }
                if let Some(ci) = active_command_idx!() {
                    let word = u16_to_string(&args[index..word_end]);
                    let cidx = expansions[ci].cases.len().checked_sub(1);
                    let mut handled = false;
                    if let Some(cc) = cidx {
                        if !expansions[ci].cases[cc].phase_patterns {
                            if word == "in" && expansions[ci].cases[cc].subject_seen {
                                expansions[ci].cases[cc].phase_patterns = true;
                                expansions[ci].cases[cc].expect_pattern = true;
                                expansions[ci].command_position = true;
                            } else {
                                expansions[ci].cases[cc].subject_seen = true;
                                expansions[ci].command_position = false;
                            }
                            handled = true;
                        } else if expansions[ci].cases[cc].expect_pattern
                            && expansions[ci].command_position
                            && word == "esac"
                        {
                            expansions[ci].cases.pop();
                            expansions[ci].command_position = false;
                            handled = true;
                        }
                    }
                    if !handled {
                        let in_pending_pattern = cidx.is_some_and(|cc| {
                            expansions[ci].cases[cc].phase_patterns
                                && expansions[ci].cases[cc].expect_pattern
                        });
                        if expansions[ci].command_position && word == "case" && !in_pending_pattern
                        {
                            expansions[ci].cases.push(CaseState {
                                phase_patterns: false,
                                subject_seen: false,
                                expect_pattern: false,
                            });
                            expansions[ci].command_position = false;
                        } else {
                            expansions[ci].command_position = expansions[ci].command_position
                                && COMMAND_LIST_PREFIX_WORDS.contains(&word.as_str());
                        }
                    }
                }
                index = word_end;
                continue;
            }
            if (character == b';' as u16
                || character == b'&' as u16
                || character == b'|' as u16
                || character == 0x0a)
                && let Some(ci) = active_command_idx!()
            {
                expansions[ci].command_position = true;
            }
            index += 1;
            continue;
        }

        // no open expansion
        if word_quote != 0 {
            if character == word_quote {
                if word_quote == outer_boundary_quote {
                    let next = args.get(index + 1).copied();
                    let boundary_follows = next.is_none_or(|n| js_ws(n) || is_punct(n, b";&|)"));
                    return if boundary_follows { index } else { args.len() };
                }
                word_quote = 0;
                index += 1;
                continue;
            }
            let nested = if word_quote != b'\'' as u16 {
                expansion_at_cursor(args, index)
            } else {
                None
            };
            if let Some(n) = nested {
                if n.kind == ExpKind::Command {
                    return args.len();
                }
                push_expansion!(n.close, n.kind);
                index += n.length;
                continue;
            }
            index += 1;
            continue;
        }

        if character == b'\'' as u16 || character == b'"' as u16 || character == b'`' as u16 {
            word_quote = character;
            index += 1;
            continue;
        }
        if let Some(n) = expansion_at_cursor(args, index) {
            if n.kind == ExpKind::Command {
                return args.len();
            }
            push_expansion!(n.close, n.kind);
            index += n.length;
            continue;
        }
        if starts_with_u16(args, "<(", index) || starts_with_u16(args, ">(", index) {
            return args.len();
        }
        if character == b'(' as u16 {
            return args.len();
        }
        if js_ws(character) || is_punct(character, b";&|()<>") {
            break;
        }
        index += 1;
    }
    index
}

fn redact_bounded_sensitive_environment_values(args: &[u16]) -> Vec<u16> {
    let mut result: Vec<u16> = Vec::new();
    let mut cursor = 0usize;
    for m in env_matches(args) {
        if !is_sensitive_environment_name(&m.name) {
            continue;
        }
        let value_start = m.value_start;
        if value_start < cursor {
            continue;
        }
        let value_end = bounded_shell_assignment_value_end(args, value_start, m.boundary);
        result.extend_from_slice(&args[cursor..value_start]);
        result.extend("[redacted]".encode_utf16());
        cursor = value_end;
    }
    result.extend_from_slice(&args[cursor.min(args.len())..]);
    result
}

/// JS `\s` on a char (includes U+FEFF, excludes U+0085 which Rust counts).
fn js_ws_char(c: char) -> bool {
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

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn word_boundary_at(c: &[char], p: usize) -> bool {
    let before = p > 0 && is_word_char(c[p - 1]);
    let after = p < c.len() && is_word_char(c[p]);
    before != after
}

fn ci_eq(a: &[char], s: &str) -> bool {
    a.len() == s.len()
        && a.iter()
            .zip(s.chars())
            .all(|(x, y)| x.eq_ignore_ascii_case(&y))
}

fn push_chars(out: &mut String, c: &[char]) {
    out.extend(c.iter());
}

// --- replace patterns ---

/// Length of the flag-name alternative matched at `c` (JS alternation order).
fn flag_name_len(c: &[char]) -> Option<usize> {
    let n = c.len();
    // api[-_]?key — sep optional, tries with sep first (greedy), then without.
    if n >= 3 && ci_eq(&c[..3], "api") {
        if n >= 7 && matches!(c[3], '-' | '_') && ci_eq(&c[4..7], "key") {
            return Some(7);
        }
        if n >= 6 && ci_eq(&c[3..6], "key") {
            return Some(6);
        }
    }
    for name in [
        "auth",
        "authorization",
        "key",
        "password",
        "secret",
        "token",
    ] {
        if n >= name.len() && ci_eq(&c[..name.len()], name) {
            return Some(name.len());
        }
    }
    None
}

/// Scan one or more value tokens: quoted runs or `[^\s'"`]+`.
/// Returns (end, matched_any).
fn scan_flag_value(c: &[char], start: usize) -> (usize, bool) {
    let n = c.len();
    let mut i = start;
    let mut any = false;
    loop {
        if i >= n {
            break;
        }
        match c[i] {
            q @ ('"' | '\'' | '`') => {
                let mut j = i + 1;
                loop {
                    if j >= n {
                        return (j, true);
                    }
                    if c[j] == '\\' {
                        j += 2;
                        continue;
                    }
                    if c[j] == q {
                        j += 1;
                        break;
                    }
                    j += 1;
                }
                i = j;
                any = true;
            }
            ch if js_ws_char(ch) => break,
            _ => {
                let mut j = i;
                while j < n && !js_ws_char(c[j]) && !matches!(c[j], '"' | '\'' | '`') {
                    j += 1;
                }
                i = j;
                any = true;
            }
        }
    }
    (i, any)
}

fn flag_value_replace(text: &str) -> String {
    // /--?(?:api[-_]?key|auth|authorization|key|password|secret|token)(?:=|\s+)
    //   (?:"(?:\\.|[^"\\])*(?:"|$)|'(?:\\.|[^'\\])*(?:'|$)|`(?:\\.|[^`\\])*(?:`|$)|[^\s'"`]+)+/gi
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    'scan: while i < n {
        if c[i] == '-' {
            let mut j = i + 1;
            if j < n && c[j] == '-' {
                j += 1;
            }
            if let Some(nl) = flag_name_len(&c[j..]) {
                let k = j + nl;
                let prefix_end = if k < n && c[k] == '=' {
                    k + 1
                } else if k < n && js_ws_char(c[k]) {
                    let mut k2 = k;
                    while k2 < n && js_ws_char(c[k2]) {
                        k2 += 1;
                    }
                    k2
                } else {
                    i += 1;
                    continue 'scan;
                };
                let (vend, ok) = scan_flag_value(&c, prefix_end);
                if ok {
                    push_chars(&mut out, &c[cursor..prefix_end]);
                    out.push_str("[redacted]");
                    cursor = vend;
                    i = vend;
                    continue 'scan;
                }
            }
        }
        i += 1;
    }
    push_chars(&mut out, &c[cursor.min(n)..]);
    out
}

fn url_credentials_replace(text: &str) -> String {
    // /\b([a-z][a-z0-9+.-]*:\/\/)[^/?#\s]+@/giu → "$1[redacted]@"
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i < n {
        if word_boundary_at(&c, i) && c[i].is_ascii_alphabetic() {
            let mut j = i + 1;
            while j < n && (c[j].is_ascii_alphanumeric() || matches!(c[j], '+' | '.' | '-')) {
                j += 1;
            }
            if j + 3 <= n && c[j] == ':' && c[j + 1] == '/' && c[j + 2] == '/' {
                let cred = j + 3;
                let mut k = cred;
                while k < n && !matches!(c[k], '/' | '?' | '#') && !js_ws_char(c[k]) {
                    k += 1;
                }
                // `+@` greedy: the match ends at the LAST '@' inside the run (after >=1 char).
                let mut m = k;
                let mut last_at = None;
                while m > cred {
                    m -= 1;
                    if c[m] == '@' {
                        last_at = Some(m);
                        break;
                    }
                }
                if let Some(at) = last_at {
                    push_chars(&mut out, &c[cursor..cred]);
                    out.push_str("[redacted]@");
                    cursor = at + 1;
                    i = at + 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    push_chars(&mut out, &c[cursor.min(n)..]);
    out
}

fn bearer_replace(text: &str) -> String {
    // /(Bearer\s+)[A-Za-z0-9._~+/=-]+/gi → "$1[redacted]"
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i + 6 <= n {
        if ci_eq(&c[i..i + 6], "bearer") {
            let mut j = i + 6;
            while j < n && js_ws_char(c[j]) {
                j += 1;
            }
            if j > i + 6 {
                let mut k = j;
                while k < n
                    && (c[k].is_ascii_alphanumeric()
                        || matches!(c[k], '.' | '_' | '~' | '+' | '/' | '=' | '-'))
                {
                    k += 1;
                }
                if k > j {
                    push_chars(&mut out, &c[cursor..j]);
                    out.push_str("[redacted]");
                    cursor = k;
                    i = k;
                    continue;
                }
            }
        }
        i += 1;
    }
    push_chars(&mut out, &c[cursor.min(n)..]);
    out
}

/// Greedy `[A-Za-z0-9_-]{min,}\b` from `body_start`: backtracks to the first end
/// position where the trailing `\b` holds.
fn token_match_end(c: &[char], body_start: usize, min: usize) -> Option<usize> {
    let mut run = body_start;
    while run < c.len() && (c[run].is_ascii_alphanumeric() || matches!(c[run], '_' | '-')) {
        run += 1;
    }
    let mut e = run;
    while e >= body_start + min {
        if word_boundary_at(c, e) {
            return Some(e);
        }
        e -= 1;
    }
    None
}

fn token_replace(text: &str) -> String {
    // /\bsk-[A-Za-z0-9_-]{8,}\b/g and /\bsyn_(?:pair|mcp)_v1_[A-Za-z0-9_-]+\b/g → "[redacted]"
    let c: Vec<char> = text.chars().collect();
    let n = c.len();
    let mut out = String::with_capacity(text.len());
    let mut cursor = 0usize;
    let mut i = 0usize;
    while i < n {
        if c[i] == 's' && word_boundary_at(&c, i) {
            let end = if i + 3 <= n && c[i + 1] == 'k' && c[i + 2] == '-' {
                token_match_end(&c, i + 3, 8)
            } else if starts_with_chars(&c, i, "syn_pair_v1_") {
                token_match_end(&c, i + 12, 1)
            } else if starts_with_chars(&c, i, "syn_mcp_v1_") {
                token_match_end(&c, i + 11, 1)
            } else {
                None
            };
            if let Some(e) = end {
                push_chars(&mut out, &c[cursor..i]);
                out.push_str("[redacted]");
                cursor = e;
                i = e;
                continue;
            }
        }
        i += 1;
    }
    push_chars(&mut out, &c[cursor.min(n)..]);
    out
}

fn starts_with_chars(c: &[char], at: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    at + p.len() <= c.len() && c[at..at + p.len()] == p[..]
}

/// Port of `redactSensitiveProcessArgs` (`apps/server/src/processArgumentRedaction.ts:349`).
pub fn redact_sensitive_process_args(
    args: String,
    options: serde_json::Value,
) -> Result<String, String> {
    // `options = {}` default param: undefined → {}; null → member access throws.
    let opt = if is_undefined(&options) {
        Value::Object(serde_json::Map::new())
    } else {
        options
    };
    let truncate = truthy(&member(&opt, "truncateSensitiveEnvironmentRemainder")?);
    let u: Vec<u16> = args.encode_utf16().collect();
    let environment_redacted = if truncate {
        redact_sensitive_environment_remainder(&u)
    } else {
        redact_bounded_sensitive_environment_values(&u)
    };
    let env_str = u16_to_string(&environment_redacted);
    let s = flag_value_replace(&env_str);
    let s = url_credentials_replace(&s);
    let s = bearer_replace(&s);
    let s = token_replace(&s);
    Ok(s)
}
