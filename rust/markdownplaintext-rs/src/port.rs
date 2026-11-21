//! The Rust port of `packages/shared/src/markdownPlainText.ts`.

#![forbid(unsafe_code)]

use regex::{Captures, Regex};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;
use unicode_segmentation::UnicodeSegmentation;

const WS: &str = "[\\t-\\r \\u{a0}\\u{1680}\\u{2000}-\\u{200a}\\u{2028}\\u{2029}\\u{202f}\\u{205f}\\u{3000}\\u{feff}]";
const PROTECTED_START: char = '\u{e000}';
const PROTECTED_END: char = '\u{e001}';
const GRAPHEME_CUT_WINDOW: f64 = 64.0;

macro_rules! regex {
    ($e:expr) => {{
        static R: LazyLock<Regex> = LazyLock::new(|| Regex::new($e).expect("regex compiles"));
        &*R
    }};
}

fn ws(pattern: &str) -> String {
    pattern.replace("WS", WS)
}

fn js_ws(c: char) -> bool {
    matches!(
        c,
        '\t'..='\r'
            | ' '
            | '\u{a0}'
            | '\u{1680}'
            | '\u{2000}'..='\u{200a}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202f}'
            | '\u{205f}'
            | '\u{3000}'
            | '\u{feff}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(js_ws)
}

fn emit_js_string(units: &[u16]) -> Value {
    match String::from_utf16(units) {
        Ok(s) => Value::String(s),
        Err(_) => json!({ "$utf16": units }),
    }
}

fn opening_fence() -> &'static Regex {
    regex!(
        r"(^|\n)((?:[ \t]{0,3}(?:>[ \t]?|(?:[-+*]|[0-9]{1,9}[.)])[ \t]+))*[ \t]{0,3})(`{3,}|~{3,})([^\r\n]*)\r?\n"
    )
}

fn escape_re() -> &'static Regex {
    regex!(r##"\\([!"#$%&'()*+,\-./:;<=>?@\[\\\]^_`{|}~])"##)
}

fn restore_re() -> &'static Regex {
    regex!("\u{e000}([0-9]+)\u{e001}")
}

fn reference_def_re() -> &'static Regex {
    regex!(&ws(
        "(?m)^[ \t]{0,3}\\[([^\\]\n]+)\\]:[ \t]*(?:<[^<>\r\n]*>|[^WS<>]+)(?:[ \t]+(?:\"[^\"\r\n]*\"|'[^'\r\n]*'|\\([^)\r\n]*\\)))?[ \t]*$"
    ))
}

fn column_width(value: &str) -> usize {
    let mut columns = 0usize;
    for character in value.chars() {
        columns = if character == '\t' {
            columns + (4 - (columns % 4))
        } else {
            columns + 1
        };
    }
    columns
}

fn find_container_boundary(
    text: &str,
    content_start: usize,
    quote_re: &Regex,
    quote_depth: usize,
    list_continuation_indent: usize,
) -> Option<usize> {
    if quote_depth == 0 && list_continuation_indent == 0 {
        return None;
    }
    let mut line_start = content_start;
    while line_start < text.len() {
        let next_newline = text[line_start..].find('\n').map(|i| line_start + i);
        let line_end = next_newline.unwrap_or(text.len());
        let line = text[line_start..line_end]
            .strip_suffix('\r')
            .unwrap_or(&text[line_start..line_end]);
        let quote_match = quote_re.find(line);
        if quote_depth > 0 && quote_match.is_none() {
            return Some(line_start);
        }
        let content = &line[quote_match.map(|m| m.end()).unwrap_or(0)..];
        if list_continuation_indent > 0 && !js_trim(content).is_empty() {
            let indentation: String = content
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect();
            if column_width(&indentation) < list_continuation_indent {
                return Some(line_start);
            }
        }
        let next_newline = match next_newline {
            Some(n) => n,
            None => break,
        };
        line_start = next_newline + 1;
    }
    None
}

fn protect_markdown_code(text: &str) -> (String, Vec<String>) {
    let mut segments: Vec<String> = Vec::new();
    let protect = |segments: &mut Vec<String>, content: &str| -> String {
        let token = format!("{PROTECTED_START}{}{PROTECTED_END}", segments.len());
        segments.push(content.to_string());
        token
    };

    // Fenced blocks
    let mut result = String::new();
    let mut cursor = 0usize;
    let mut pos = 0usize;
    while let Some(opening) = opening_fence().captures_at(text, pos) {
        let whole = opening.get(0).map_or(0..0, |m| m.range());
        let line_prefix = opening.get(1).map_or("", |m| m.as_str());
        let container_prefix = opening.get(2).map_or("", |m| m.as_str());
        let fence = opening.get(3).map_or("", |m| m.as_str());
        let info = opening.get(4).map_or("", |m| m.as_str());
        let fence_character = fence.chars().next().unwrap_or('`');
        let open_end = whole.end;
        pos = open_end;
        if fence_character == '`' && info.contains('`') {
            continue;
        }

        let quote_depth = container_prefix.matches('>').count();
        let quote_prefix = format!("(?:[ \\t]{{0,3}}>[ \\t]?){{{quote_depth}}}");
        let quote_re = Regex::new(&format!("^{quote_prefix}")).expect("regex compiles");
        let strip_re = Regex::new(&format!("(^|\\n){quote_prefix}")).expect("regex compiles");
        let prefix_without_quotes = regex!(r"[ \t]{0,3}>[ \t]?")
            .replace_all(container_prefix, "")
            .into_owned();
        let list_continuation_indent =
            if regex!(r"(?:[-+*]|[0-9]{1,9}[.)])[ \t]+").is_match(&prefix_without_quotes) {
                column_width(&prefix_without_quotes)
            } else {
                0
            };

        // Closing fence: `(^|\n)quotePrefix([ \t]*)fc{len,}[ \t]*\r?(?=\n|$)`
        let closing_re = Regex::new(&format!(
            "(^|\\n){}([ \\t]*){}{{{},}}[ \\t]*\\r?",
            quote_prefix,
            fence_character,
            fence.len()
        ))
        .expect("regex compiles");
        let mut closing_match: Option<(usize, usize, usize)> = None; // (index, end, indent columns)
        let mut scan = open_end;
        while let Some(cand) = closing_re.captures_at(text, scan) {
            let cm = cand.get(0).map_or(0..0, |m| m.range());
            let indent = cand.get(2).map_or("", |m| m.as_str());
            // Lookahead (?=\n|$): next char after the match must be \n or EOS,
            // with `\r?` allowed to give its char back.
            let end = cm.end;
            let ok_end = match text.as_bytes().get(end) {
                None | Some(b'\n') => Some(end),
                _ => {
                    if end > cm.start && text.as_bytes().get(end - 1) == Some(&b'\r') {
                        match text.as_bytes().get(end - 1) {
                            None | Some(b'\n') => Some(end - 1),
                            _ => None,
                        }
                    } else {
                        None
                    }
                }
            };
            if let Some(end) = ok_end {
                let columns = column_width(indent);
                if columns >= list_continuation_indent && columns <= list_continuation_indent + 3 {
                    closing_match = Some((cm.start, end, columns));
                    break;
                }
            }
            scan = cm.start + 1;
            if scan > text.len() {
                break;
            }
        }

        let container_boundary = find_container_boundary(
            text,
            open_end,
            &quote_re,
            quote_depth,
            list_continuation_indent,
        );
        let boundary_precedes = container_boundary.is_some()
            && (closing_match.is_none()
                || container_boundary.unwrap_or(0) < closing_match.map_or(0, |c| c.0));
        let effective_closing = if boundary_precedes {
            None
        } else {
            closing_match
        };
        let content_end = if boundary_precedes {
            container_boundary.unwrap_or(text.len())
        } else {
            effective_closing.map_or(text.len(), |c| c.0)
        };
        let content = strip_re
            .replace_all(&text[open_end..content_end], "$1")
            .into_owned();

        result += &text[cursor..whole.start];
        result += line_prefix;
        result += &protect(&mut segments, &content);

        let Some((_, closing_end, _)) = effective_closing else {
            cursor = content_end;
            if container_boundary.is_none() {
                break;
            }
            pos = cursor;
            continue;
        };
        cursor = closing_end;
        pos = cursor;
    }
    result += &text[cursor..];

    // Escapes
    let text = result;
    let mut result = String::new();
    let mut cursor = 0usize;
    for cap in escape_re().captures_iter(&text) {
        let m = cap.get(0).map_or(0..0, |m| m.range());
        result += &text[cursor..m.start];
        result += &protect(&mut segments, &cap[1]);
        cursor = m.end;
    }
    result += &text[cursor..];

    // Inline code
    let text = result;
    let bytes = text.as_bytes();
    let mut runs: Vec<(usize, usize)> = Vec::new(); // (start, end)
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let start = index;
        while index < bytes.len() && bytes[index] == b'`' {
            index += 1;
        }
        runs.push((start, index));
    }
    let mut next_matching: Vec<Option<usize>> = vec![None; runs.len()];
    let mut next_by_length: HashMap<usize, usize> = HashMap::new();
    for run_index in (0..runs.len()).rev() {
        let (start, end) = runs[run_index];
        next_matching[run_index] = next_by_length.get(&(end - start)).copied();
        next_by_length.insert(end - start, run_index);
    }
    let mut result = String::new();
    let mut cursor = 0usize;
    let mut run_index = 0usize;
    while run_index < runs.len() {
        let opening_run = runs[run_index];
        match next_matching[run_index] {
            None => run_index += 1,
            Some(closing_run_index) => {
                let closing_run = runs[closing_run_index];
                result += &text[cursor..opening_run.0];
                result += &protect(&mut segments, &text[opening_run.1..closing_run.0]);
                cursor = closing_run.1;
                run_index = closing_run_index + 1;
            }
        }
    }
    result += &text[cursor..];

    (result, segments)
}

fn build_matching_delimiters(chars: &[char], open: char, close: char) -> HashMap<usize, usize> {
    let mut opening_indexes: Vec<usize> = Vec::new();
    let mut matches = HashMap::new();
    let mut i = 0usize;
    while i < chars.len() {
        let character = chars[i];
        if character == '\\' {
            i += 2;
            continue;
        }
        if character == open {
            opening_indexes.push(i);
        } else if character == close
            && let Some(opening_index) = opening_indexes.pop()
        {
            matches.insert(opening_index, i);
        }
        i += 1;
    }
    matches
}

fn normalize_reference_label(label: &str) -> String {
    let mut out = String::new();
    let mut pending = false;
    for c in js_trim(label).chars() {
        if js_ws(c) {
            pending = true;
        } else {
            if pending {
                out.push(' ');
                pending = false;
            }
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn collect_reference_labels(text: &str) -> HashSet<String> {
    let mut labels = HashSet::new();
    for cap in reference_def_re().captures_iter(text) {
        if let Some(label) = cap.get(1) {
            labels.insert(normalize_reference_label(label.as_str()));
        }
    }
    labels
}

fn strip_markdown_links(text: &str, reference_labels: &HashSet<String>) -> String {
    let chars: Vec<char> = text.chars().collect();
    let label_matches = build_matching_delimiters(&chars, '[', ']');
    let destination_matches = build_matching_delimiters(&chars, '(', ')');
    let mut result = String::new();
    let mut index = 0usize;

    while index < chars.len() {
        let is_image = chars[index] == '!' && chars.get(index + 1) == Some(&'[');
        let label_open = if is_image { index + 1 } else { index };
        if chars.get(label_open) != Some(&'[') {
            result.push(chars[index]);
            index += 1;
            continue;
        }
        let Some(&label_close_ci) = label_matches.get(&label_open) else {
            result.push(chars[index]);
            index += 1;
            continue;
        };
        let destination_open_ci = label_close_ci + 1;
        if chars.get(destination_open_ci) == Some(&'(') {
            let Some(&destination_close_ci) = destination_matches.get(&destination_open_ci) else {
                result.push(chars[index]);
                index += 1;
                continue;
            };
            if is_image {
                result.push(' ');
            } else {
                for c in &chars[label_open + 1..label_close_ci] {
                    result.push(*c);
                }
            }
            index = destination_close_ci + 1;
            continue;
        }

        let label: String = chars[label_open + 1..label_close_ci].iter().collect();
        if chars.get(destination_open_ci) == Some(&'[') {
            let Some(&reference_close_ci) = label_matches.get(&destination_open_ci) else {
                result.push(chars[index]);
                index += 1;
                continue;
            };
            let explicit: String = chars[destination_open_ci + 1..reference_close_ci]
                .iter()
                .collect();
            let reference = if explicit.is_empty() {
                label.clone()
            } else {
                explicit
            };
            if !reference_labels.contains(&normalize_reference_label(&reference)) {
                result.push(chars[index]);
                index += 1;
                continue;
            }
            if is_image {
                result.push(' ');
            } else {
                result += &label;
            }
            index = reference_close_ci + 1;
            continue;
        }

        if !reference_labels.contains(&normalize_reference_label(&label)) {
            result.push(chars[index]);
            index += 1;
            continue;
        }
        if is_image {
            result.push(' ');
        } else {
            result += &label;
        }
        index = label_close_ci + 1;
    }
    result
}

/// `text.replace(re, repl)` where the replacement may carry a boundary predicate
/// emulating JS `^`-edge/lookahead conditions the `regex` crate cannot express.
fn replace_all(
    text: &str,
    re: &Regex,
    next_disallow: Option<&Regex>,
    repl: &dyn Fn(&Captures) -> String,
) -> String {
    let mut out = String::new();
    let mut cursor = 0usize;
    let mut pos = 0usize;
    while let Some(cap) = re.captures_at(text, pos) {
        let m = cap.get(0).map_or(0..0, |m| m.range());
        let next_ok = next_disallow.is_none_or(|disallowed| {
            text[m.end..]
                .chars()
                .next()
                .is_none_or(|c| !disallowed.is_match(&c.to_string()))
        });
        if next_ok {
            out += &text[cursor..m.start];
            out += &repl(&cap);
            cursor = m.end;
            pos = m.end.max(m.start + 1);
        } else {
            pos = text.ceil_char_boundary(m.start + 1).max(m.start + 1);
        }
        if pos > text.len() {
            break;
        }
    }
    out + &text[cursor..]
}

fn word_boundary_disallow() -> &'static Regex {
    regex!(r"[\p{L}\p{N}\p{M}_]")
}

/// Port of `markdownToPlainText` (`packages/shared/src/markdownPlainText.ts:358`).
pub fn markdown_to_plain_text(markdown: String) -> Result<Value, String> {
    let (protected_text, segments) = protect_markdown_code(&markdown);
    let reference_labels = collect_reference_labels(&protected_text);
    let without_reference_definitions = reference_def_re()
        .replace_all(&protected_text, "")
        .into_owned();
    let mut cleaned = strip_markdown_links(&without_reference_definitions, &reference_labels);
    cleaned = regex!(r"`+").replace_all(&cleaned, "").into_owned();
    cleaned = regex!("(?m)^[ \\t]{0,3}(?:#{1,6}[ \\t]+|>[ \\t]?)")
        .replace_all(&cleaned, "")
        .into_owned();
    cleaned = regex!("(?m)^[ \\t]{0,3}(?:[-*+]|[0-9]{1,9}[.)])[ \\t]+(?:\\[[ xX]\\][ \\t]+)?")
        .replace_all(&cleaned, " \u{b7} ")
        .into_owned();
    let strong = ws(r"\*\*([^WS*\n](?:[^*\n]*[^WS*\n])?)\*\*");
    cleaned = Regex::new(&strong).map_or(cleaned.clone(), |re| {
        re.replace_all(&cleaned, "$1").into_owned()
    });
    let underscore2 = ws(r"(^|[^\p{L}\p{N}\p{M}_])__([^WS_\n](?:[^_\n]*[^WS_\n])?)__");
    cleaned = Regex::new(&underscore2).map_or(cleaned.clone(), |re| {
        replace_all(
            &cleaned,
            &re,
            Some(word_boundary_disallow()),
            &|c: &Captures| format!("{}{}", &c[1], &c[2]),
        )
    });
    let strike = ws(r"~~([^WS~\n](?:[^~\n]*[^WS~\n])?)~~");
    cleaned = Regex::new(&strike).map_or(cleaned.clone(), |re| {
        re.replace_all(&cleaned, "$1").into_owned()
    });
    let star = ws(r"(^|[^\p{L}\p{N}\p{M}_])\*([^WS*\n](?:[^*\n]*[^WS*\n])?)\*");
    cleaned = Regex::new(&star).map_or(cleaned.clone(), |re| {
        replace_all(
            &cleaned,
            &re,
            Some(word_boundary_disallow()),
            &|c: &Captures| format!("{}{}", &c[1], &c[2]),
        )
    });
    let underscore = ws(r"(^|[^\p{L}\p{N}\p{M}_])_([^WS_\n](?:[^_\n]*[^WS_\n])?)_");
    cleaned = Regex::new(&underscore).map_or(cleaned.clone(), |re| {
        replace_all(
            &cleaned,
            &re,
            Some(word_boundary_disallow()),
            &|c: &Captures| format!("{}{}", &c[1], &c[2]),
        )
    });

    let restored = restore_re()
        .replace_all(&cleaned, |cap: &Captures| {
            cap[1]
                .parse::<usize>()
                .ok()
                .and_then(|i| segments.get(i))
                .cloned()
                .unwrap_or_default()
        })
        .into_owned();
    let trimmed = js_trim(&restored);
    let mut out = String::new();
    let mut pending = false;
    for c in trimmed.chars() {
        if js_ws(c) {
            pending = true;
        } else {
            if pending {
                out.push(' ');
                pending = false;
            }
            out.push(c);
        }
    }
    Ok(emit_js_string(&crate::utf16::units(&out)))
}

/// JS `Math.max(0, x)`.
fn js_max_zero(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        if x.is_nan() { f64::NAN } else { 0.0 }
    } else {
        x
    }
}

/// JS `slice(start, end)` over UTF-16 units (negative indexes add length; NaN/frac via
/// ToIntegerOrInfinity).
fn js_slice_units(units: &[u16], start: f64, end: f64) -> Vec<u16> {
    let len = units.len() as f64;
    let to_index = |v: f64, is_end: bool| -> usize {
        if v.is_nan() {
            return 0;
        }
        let v = v.trunc();
        let i = if v < 0.0 {
            (len + v).max(0.0)
        } else {
            v.min(len)
        };
        let _ = is_end;
        i as usize
    };
    let s = to_index(start, false);
    let e = to_index(end, true);
    if s >= e {
        Vec::new()
    } else {
        units[s..e].to_vec()
    }
}

/// Port of `truncatePlainText` (`packages/shared/src/markdownPlainText.ts:393`).
pub fn truncate_plain_text(text: String, max_length: f64) -> Result<Value, String> {
    let units: Vec<u16> = text.encode_utf16().collect();
    if (units.len() as f64) <= max_length {
        return Ok(Value::String(text));
    }
    let budget = max_length - 1.0;
    let window_start = js_max_zero(budget - GRAPHEME_CUT_WINDOW);
    let window_units = js_slice_units(&units, window_start, budget + 2.0);
    let window = String::from_utf16_lossy(&window_units);
    let mut end = window_start;
    for (byte_index, _) in window.grapheme_indices(true) {
        let boundary = window_start + crate::utf16::utf16_offset(&window, byte_index) as f64;
        if boundary > budget {
            break;
        }
        end = boundary;
    }
    let mut cut = js_slice_units(&units, 0.0, end);
    // trimEnd
    while let Some(&last) = cut.last() {
        let is_ws = match char::from_u32(last as u32) {
            Some(c) => js_ws(c),
            None => false,
        };
        if !is_ws {
            break;
        }
        cut.pop();
    }
    cut.extend("\u{2026}".encode_utf16());
    Ok(emit_js_string(&cut))
}
