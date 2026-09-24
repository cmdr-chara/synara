//! Small, allocation-bounded lexical highlighting for the native editor.
//!
//! This intentionally recognizes common token classes instead of trying to
//! parse complete language grammars. It runs only for known source extensions
//! and buffers up to `MAX_HIGHLIGHT_BYTES`; larger files keep the existing
//! plain-text editor path.

use std::ops::Range;

pub(super) const MAX_HIGHLIGHT_BYTES: usize = 256 * 1024;
const MAX_HIGHLIGHT_SPANS: usize = 32 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Language {
    Rust,
    JavaScript,
    Python,
    Json,
    Toml,
    Yaml,
    Shell,
    Markdown,
    Html,
    Css,
    Sql,
    CLike,
}

impl Language {
    pub(super) fn from_path(path: &std::path::Path) -> Option<Self> {
        let extension = path.extension().and_then(|value| value.to_str());
        if let Some(extension) = extension {
            return Some(match extension.to_ascii_lowercase().as_str() {
                "rs" => Self::Rust,
                "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => Self::JavaScript,
                "py" | "pyi" => Self::Python,
                "json" | "jsonc" => Self::Json,
                "toml" => Self::Toml,
                "yaml" | "yml" => Self::Yaml,
                "sh" | "bash" | "zsh" | "fish" => Self::Shell,
                "md" | "markdown" | "mdx" => Self::Markdown,
                "html" | "htm" | "xml" | "svg" => Self::Html,
                "css" | "scss" | "less" => Self::Css,
                "sql" => Self::Sql,
                "c" | "h" | "cc" | "hh" | "cpp" | "hpp" | "cxx" | "hxx" | "java" | "kt" | "kts"
                | "go" | "swift" | "cs" => Self::CLike,
                _ => return None,
            });
        }

        match path.file_name().and_then(|value| value.to_str()) {
            Some("Dockerfile") | Some("Makefile") | Some("Justfile") => Some(Self::Shell),
            _ => None,
        }
    }

    fn line_comments(self) -> &'static [&'static str] {
        match self {
            Self::Rust | Self::JavaScript | Self::CLike => &["//"],
            Self::Python | Self::Toml | Self::Yaml | Self::Shell => &["#"],
            Self::Json => &["//"],
            Self::Sql => &["--"],
            Self::Css => &[],
            Self::Html => &[],
            Self::Markdown => &[],
        }
    }

    fn supports_block_comments(self) -> bool {
        matches!(
            self,
            Self::Rust | Self::JavaScript | Self::Css | Self::Sql | Self::CLike
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TokenKind {
    Keyword,
    Type,
    String,
    Comment,
    Number,
    Property,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct HighlightSpan {
    pub(super) range: Range<usize>,
    pub(super) kind: TokenKind,
}

pub(super) fn color(kind: TokenKind, palette: crate::ui::Palette) -> u32 {
    if palette.muted == palette.text && kind != TokenKind::Keyword {
        return palette.text;
    }
    let dark = palette.canvas < 0x808080;
    match kind {
        TokenKind::Keyword => palette.focus,
        TokenKind::Comment => palette.muted,
        TokenKind::String => {
            if dark {
                0x91c88a
            } else {
                0x27643d
            }
        }
        TokenKind::Number => {
            if dark {
                0xe0a36c
            } else {
                0x875000
            }
        }
        TokenKind::Type => {
            if dark {
                0x83b5d1
            } else {
                0x17627c
            }
        }
        TokenKind::Property => {
            if dark {
                0xd7b879
            } else {
                0x795500
            }
        }
    }
}

pub(super) fn highlight(text: &str, language: Language) -> Vec<HighlightSpan> {
    if text.len() > MAX_HIGHLIGHT_BYTES {
        return Vec::new();
    }
    if language == Language::Markdown {
        return highlight_markdown(text);
    }

    let bytes = text.as_bytes();
    let mut spans = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if language == Language::Html && starts_with(bytes, index, b"<!--") {
            let end = find_bytes(bytes, index + 4, b"-->").map_or(bytes.len(), |end| end + 3);
            push_span(&mut spans, index..end, TokenKind::Comment);
            index = end;
            continue;
        }

        if language == Language::Html && bytes[index] == b'<' {
            if let Some((tag_spans, end)) = scan_html_tag(text, index) {
                for (range, kind) in tag_spans {
                    push_span(&mut spans, range, kind);
                }
                index = end;
                continue;
            }
        }

        if language.supports_block_comments() && starts_with(bytes, index, b"/*") {
            let end = scan_block_comment(bytes, index, language == Language::Rust);
            push_span(&mut spans, index..end, TokenKind::Comment);
            index = end;
            continue;
        }

        if language
            .line_comments()
            .iter()
            .any(|prefix| starts_with(bytes, index, prefix.as_bytes()))
        {
            let end = bytes[index..]
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(bytes.len(), |offset| index + offset);
            push_span(&mut spans, index..end, TokenKind::Comment);
            index = end;
            continue;
        }

        if language == Language::Rust
            && let Some(end) = scan_rust_raw_string(bytes, index)
        {
            push_span(&mut spans, index..end, TokenKind::String);
            index = end;
            continue;
        }

        let byte = bytes[index];
        if is_quote(language, byte) {
            if language == Language::Rust && byte == b'\'' {
                if let Some(end) = scan_rust_char(text, index) {
                    push_span(&mut spans, index..end, TokenKind::String);
                    index = end;
                    continue;
                }
            } else {
                let end = scan_quoted(text, index, byte, language == Language::Python);
                let kind = if (language == Language::Json && followed_by(text, end, b':'))
                    || (language == Language::Yaml && followed_by(text, end, b':'))
                    || (language == Language::Toml && followed_by(text, end, b'='))
                {
                    TokenKind::Property
                } else {
                    TokenKind::String
                };
                push_span(&mut spans, index..end, kind);
                index = end;
                continue;
            }
        }

        if byte.is_ascii_digit()
            || (byte == b'.'
                && bytes
                    .get(index + 1)
                    .is_some_and(|next| next.is_ascii_digit()))
        {
            let end = scan_number(bytes, index);
            push_span(&mut spans, index..end, TokenKind::Number);
            index = end;
            continue;
        }

        let ch = text[index..].chars().next().expect("valid UTF-8 boundary");
        if is_identifier_start(ch) {
            let start = index;
            index += ch.len_utf8();
            while index < text.len() {
                let ch = text[index..].chars().next().expect("valid UTF-8 boundary");
                if !is_identifier_continue(ch) {
                    break;
                }
                index += ch.len_utf8();
            }
            let identifier = &text[start..index];
            if is_keyword(language, identifier) {
                push_span(&mut spans, start..index, TokenKind::Keyword);
            } else if language == Language::Css && followed_by_colon(text, index) {
                push_span(&mut spans, start..index, TokenKind::Property);
            } else if (language == Language::Yaml && followed_by(text, index, b':'))
                || (language == Language::Toml && followed_by(text, index, b'='))
            {
                push_span(&mut spans, start..index, TokenKind::Property);
            } else if is_type(language, identifier) {
                push_span(&mut spans, start..index, TokenKind::Type);
            }
            continue;
        }

        index += ch.len_utf8();
    }
    spans
}

fn highlight_markdown(text: &str) -> Vec<HighlightSpan> {
    let mut spans = Vec::new();
    let mut offset = 0;
    let mut in_fence = false;
    for line in text.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = content.trim_start();
        let leading = content.len() - trimmed.len();
        let fence_marker = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if fence_marker {
            push_span(
                &mut spans,
                offset + leading..offset + content.len(),
                TokenKind::Keyword,
            );
            in_fence = !in_fence;
        } else if !in_fence && markdown_heading_len(trimmed).is_some() {
            push_span(
                &mut spans,
                offset + leading..offset + content.len(),
                TokenKind::Type,
            );
        } else if !in_fence {
            highlight_markdown_inline(content, offset, &mut spans);
        }
        offset += line.len();
    }
    spans
}

fn markdown_heading_len(line: &str) -> Option<usize> {
    let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6).contains(&hashes).then(|| hashes).filter(|hashes| {
        line.as_bytes()
            .get(*hashes)
            .is_some_and(|byte| byte.is_ascii_whitespace())
    })
}

fn highlight_markdown_inline(line: &str, offset: usize, spans: &mut Vec<HighlightSpan>) {
    let bytes = line.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'`' {
            let ticks = bytes[index..]
                .iter()
                .take_while(|byte| **byte == b'`')
                .count();
            let marker = &bytes[index..index + ticks];
            if let Some(end) = find_bytes(bytes, index + ticks, marker) {
                let end = end + ticks;
                push_span(spans, offset + index..offset + end, TokenKind::String);
                index = end;
                continue;
            }
        }
        index += line[index..].chars().next().unwrap().len_utf8();
    }
}

fn scan_html_tag(text: &str, start: usize) -> Option<(Vec<(Range<usize>, TokenKind)>, usize)> {
    let bytes = text.as_bytes();
    let mut index = start + 1;
    if bytes.get(index) == Some(&b'/') {
        index += 1;
    }
    if !bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphabetic() || matches!(*byte, b'!' | b'?'))
    {
        return None;
    }
    let mut result = Vec::new();
    let tag_start = index;
    while bytes.get(index).is_some_and(|byte| {
        byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b':' | b'.')
    }) {
        index += 1;
    }
    if tag_start < index {
        result.push((tag_start..index, TokenKind::Type));
    }

    while index < bytes.len() {
        let byte = bytes[index];
        if byte == b'>' {
            return Some((result, index + 1));
        }
        if matches!(byte, b'\'' | b'"') {
            let start_value = index;
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = next_char_boundary(text, index + 1);
                    continue;
                }
                if bytes[index] == byte {
                    result.push((start_value..index + 1, TokenKind::String));
                    index += 1;
                    break;
                }
                index += 1;
            }
            if bytes.get(index.saturating_sub(1)) != Some(&byte) {
                result.push((start_value..bytes.len(), TokenKind::String));
                return Some((result, bytes.len()));
            }
            continue;
        }
        if byte.is_ascii_alphabetic() || matches!(byte, b'_' | b':') {
            let name_start = index;
            index += 1;
            while bytes.get(index).is_some_and(|byte| {
                byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'-' | b':' | b'.')
            }) {
                index += 1;
            }
            result.push((name_start..index, TokenKind::Property));
            continue;
        }
        index += 1;
    }
    Some((result, bytes.len()))
}

fn is_quote(language: Language, byte: u8) -> bool {
    match language {
        Language::Json => byte == b'"',
        Language::Rust | Language::Python | Language::Shell => matches!(byte, b'\'' | b'"'),
        Language::JavaScript => matches!(byte, b'\'' | b'"' | b'`'),
        Language::Toml | Language::Yaml | Language::Sql | Language::CLike | Language::Css => {
            matches!(byte, b'\'' | b'"')
        }
        Language::Html | Language::Markdown => false,
    }
}

fn scan_quoted(text: &str, start: usize, quote: u8, allow_triple: bool) -> usize {
    let bytes = text.as_bytes();
    let triple = allow_triple
        && bytes
            .get(start..start + 3)
            .is_some_and(|prefix| prefix == &[quote, quote, quote]);
    let delimiter_len = if triple { 3 } else { 1 };
    let mut index = start + delimiter_len;
    while index < bytes.len() {
        if bytes[index] == b'\\' {
            index = next_char_boundary(text, index + 1);
            continue;
        }
        if triple {
            if bytes
                .get(index..index + 3)
                .is_some_and(|candidate| candidate == &[quote, quote, quote])
            {
                return index + 3;
            }
        } else if bytes[index] == quote {
            return index + 1;
        }
        index += text[index..].chars().next().unwrap().len_utf8();
    }
    bytes.len()
}

fn scan_rust_char(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = start + 1;
    if bytes.get(index) == Some(&b'\\') {
        index += 1;
        if bytes.get(index) == Some(&b'u') && bytes.get(index + 1) == Some(&b'{') {
            index += 2;
            while bytes.get(index).is_some_and(|byte| *byte != b'}') {
                index += 1;
            }
            index += usize::from(bytes.get(index) == Some(&b'}'));
        } else if index < bytes.len() {
            index = next_char_boundary(text, index);
        }
    } else if index < bytes.len() {
        index = next_char_boundary(text, index);
    }
    (bytes.get(index) == Some(&b'\'')).then_some(index + 1)
}

fn scan_rust_raw_string(bytes: &[u8], start: usize) -> Option<usize> {
    let mut index = start;
    if bytes.get(index) == Some(&b'b') {
        index += 1;
    }
    if bytes.get(index) != Some(&b'r') {
        return None;
    }
    index += 1;
    let hashes_start = index;
    while bytes.get(index) == Some(&b'#') {
        index += 1;
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let hash_count = index - hashes_start;
    index += 1;
    while index < bytes.len() {
        if bytes[index] == b'"'
            && bytes
                .get(index + 1..index + 1 + hash_count)
                .is_some_and(|hashes| hashes.iter().all(|byte| *byte == b'#'))
        {
            return Some(index + 1 + hash_count);
        }
        index += 1;
    }
    Some(bytes.len())
}

fn scan_block_comment(bytes: &[u8], start: usize, nested: bool) -> usize {
    let mut index = start + 2;
    let mut depth = 1usize;
    while index < bytes.len() {
        if nested && bytes.get(index..index + 2) == Some(&b"/*"[..]) {
            depth += 1;
            index += 2;
        } else if bytes.get(index..index + 2) == Some(&b"*/"[..]) {
            depth -= 1;
            index += 2;
            if depth == 0 {
                return index;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn scan_number(bytes: &[u8], start: usize) -> usize {
    let mut index = start;
    while bytes
        .get(index)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.'))
    {
        index += 1;
    }
    index
}

fn followed_by(text: &str, mut index: usize, marker: u8) -> bool {
    while text
        .as_bytes()
        .get(index)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        index += 1;
    }
    text.as_bytes().get(index) == Some(&marker)
}

fn followed_by_colon(text: &str, index: usize) -> bool {
    followed_by(text, index, b':')
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_alphabetic() || !ch.is_ascii()
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_alphanumeric() || !ch.is_ascii()
}

fn is_type(language: Language, word: &str) -> bool {
    let builtin = match language {
        Language::Rust => matches!(
            word,
            "bool"
                | "char"
                | "str"
                | "String"
                | "Self"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "i128"
                | "isize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "usize"
                | "f32"
                | "f64"
        ),
        Language::JavaScript | Language::Json => matches!(
            word,
            "Array"
                | "Boolean"
                | "Date"
                | "Error"
                | "Map"
                | "Number"
                | "Object"
                | "Promise"
                | "RegExp"
                | "Set"
                | "String"
                | "Symbol"
                | "WeakMap"
                | "WeakSet"
        ),
        _ => false,
    };
    builtin || word.chars().next().is_some_and(char::is_uppercase)
}

fn is_keyword(language: Language, word: &str) -> bool {
    if language == Language::Sql {
        return matches!(
            word.to_ascii_lowercase().as_str(),
            "all"
                | "alter"
                | "and"
                | "as"
                | "asc"
                | "begin"
                | "between"
                | "by"
                | "case"
                | "commit"
                | "create"
                | "cross"
                | "delete"
                | "desc"
                | "distinct"
                | "drop"
                | "else"
                | "end"
                | "except"
                | "exists"
                | "false"
                | "from"
                | "full"
                | "group"
                | "having"
                | "in"
                | "inner"
                | "insert"
                | "into"
                | "is"
                | "join"
                | "left"
                | "like"
                | "limit"
                | "not"
                | "null"
                | "offset"
                | "on"
                | "or"
                | "order"
                | "outer"
                | "right"
                | "rollback"
                | "select"
                | "set"
                | "table"
                | "then"
                | "true"
                | "union"
                | "update"
                | "values"
                | "when"
                | "where"
        );
    }
    match language {
        Language::Rust => matches!(
            word,
            "as" | "async"
                | "await"
                | "break"
                | "const"
                | "continue"
                | "crate"
                | "dyn"
                | "else"
                | "enum"
                | "extern"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "in"
                | "let"
                | "loop"
                | "match"
                | "mod"
                | "move"
                | "mut"
                | "pub"
                | "ref"
                | "return"
                | "self"
                | "static"
                | "struct"
                | "super"
                | "trait"
                | "true"
                | "type"
                | "union"
                | "unsafe"
                | "use"
                | "where"
                | "while"
                | "Some"
                | "None"
                | "Ok"
                | "Err"
        ),
        Language::JavaScript => matches!(
            word,
            "as" | "async"
                | "await"
                | "break"
                | "case"
                | "catch"
                | "class"
                | "const"
                | "continue"
                | "debugger"
                | "default"
                | "delete"
                | "do"
                | "else"
                | "export"
                | "extends"
                | "false"
                | "finally"
                | "for"
                | "from"
                | "function"
                | "get"
                | "if"
                | "implements"
                | "import"
                | "in"
                | "instanceof"
                | "interface"
                | "let"
                | "new"
                | "null"
                | "of"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "set"
                | "static"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "true"
                | "try"
                | "typeof"
                | "undefined"
                | "var"
                | "void"
                | "while"
                | "with"
                | "yield"
        ),
        Language::Python => matches!(
            word,
            "and"
                | "as"
                | "assert"
                | "async"
                | "await"
                | "break"
                | "class"
                | "continue"
                | "def"
                | "del"
                | "elif"
                | "else"
                | "except"
                | "False"
                | "finally"
                | "for"
                | "from"
                | "global"
                | "if"
                | "import"
                | "in"
                | "is"
                | "lambda"
                | "None"
                | "nonlocal"
                | "not"
                | "or"
                | "pass"
                | "raise"
                | "return"
                | "True"
                | "try"
                | "while"
                | "with"
                | "yield"
        ),
        Language::Json => matches!(word, "true" | "false" | "null"),
        Language::Toml => matches!(word, "true" | "false"),
        Language::Yaml => matches!(
            word,
            "true" | "false" | "null" | "yes" | "no" | "on" | "off"
        ),
        Language::Shell => matches!(
            word,
            "case"
                | "do"
                | "done"
                | "elif"
                | "else"
                | "esac"
                | "fi"
                | "for"
                | "function"
                | "if"
                | "in"
                | "select"
                | "then"
                | "time"
                | "until"
                | "while"
        ),
        Language::Html => false,
        Language::Css => matches!(
            word,
            "important"
                | "and"
                | "not"
                | "only"
                | "screen"
                | "media"
                | "supports"
                | "keyframes"
                | "from"
                | "to"
        ),
        Language::Sql => false,
        Language::CLike => matches!(
            word,
            "abstract"
                | "auto"
                | "bool"
                | "boolean"
                | "break"
                | "byte"
                | "case"
                | "catch"
                | "char"
                | "class"
                | "const"
                | "continue"
                | "default"
                | "delete"
                | "do"
                | "double"
                | "else"
                | "enum"
                | "extends"
                | "extern"
                | "false"
                | "final"
                | "finally"
                | "float"
                | "for"
                | "foreach"
                | "func"
                | "function"
                | "goto"
                | "if"
                | "implements"
                | "import"
                | "include"
                | "inline"
                | "int"
                | "interface"
                | "let"
                | "long"
                | "namespace"
                | "new"
                | "null"
                | "package"
                | "private"
                | "protected"
                | "public"
                | "return"
                | "short"
                | "signed"
                | "sizeof"
                | "static"
                | "string"
                | "struct"
                | "super"
                | "switch"
                | "this"
                | "throw"
                | "throws"
                | "true"
                | "try"
                | "typedef"
                | "typename"
                | "union"
                | "unsigned"
                | "using"
                | "var"
                | "virtual"
                | "void"
                | "volatile"
                | "while"
        ),
        Language::Markdown => false,
    }
}

fn push_span(spans: &mut Vec<HighlightSpan>, range: Range<usize>, kind: TokenKind) {
    if range.is_empty() {
        return;
    }
    if let Some(previous) = spans.last_mut()
        && previous.kind == kind
        && previous.range.end == range.start
    {
        previous.range.end = range.end;
    } else if spans.len() < MAX_HIGHLIGHT_SPANS {
        spans.push(HighlightSpan { range, kind });
    }
}

fn starts_with(bytes: &[u8], index: usize, prefix: &[u8]) -> bool {
    bytes
        .get(index..index.saturating_add(prefix.len()))
        .is_some_and(|candidate| candidate == prefix)
}

fn find_bytes(bytes: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(start);
    }
    bytes
        .get(start..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| start + offset)
}

fn next_char_boundary(text: &str, index: usize) -> usize {
    text.get(index..)
        .and_then(|rest| rest.chars().next())
        .map_or(text.len(), |ch| index + ch.len_utf8())
}

#[cfg(test)]
mod tests {
    use super::{HighlightSpan, Language, TokenKind, highlight};
    use std::path::Path;

    fn spans_of(text: &str, language: Language) -> Vec<(&str, TokenKind)> {
        highlight(text, language)
            .iter()
            .map(|span: &HighlightSpan| (&text[span.range.clone()], span.kind))
            .collect()
    }

    #[test]
    fn recognizes_common_file_extensions_and_leaves_unknown_files_plain() {
        assert_eq!(
            Language::from_path(Path::new("src/main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            Language::from_path(Path::new("app.TSX")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("README.md")),
            Some(Language::Markdown)
        );
        assert_eq!(Language::from_path(Path::new(".env")), None);
        assert_eq!(Language::from_path(Path::new("Cargo.lock")), None);
    }

    #[test]
    fn highlights_rust_tokens_and_keeps_lifetimes_out_of_string_spans() {
        let text = "fn main<'a>() { let value: &'a str = r#\"hello\"#; // note\n}";
        let spans = spans_of(text, Language::Rust);
        assert!(spans.contains(&("fn", TokenKind::Keyword)));
        assert!(spans.contains(&("str", TokenKind::Type)));
        assert!(spans.contains(&("r#\"hello\"#", TokenKind::String)));
        assert!(spans.contains(&("// note", TokenKind::Comment)));
        assert!(
            !spans
                .iter()
                .any(|(value, kind)| *value == "'a" && *kind == TokenKind::String)
        );
    }

    #[test]
    fn highlights_utf8_source_without_splitting_codepoints() {
        let text = "const café = 'naïve'; // 日本語";
        let spans = highlight(text, Language::JavaScript);
        assert!(spans.iter().all(|span| {
            text.is_char_boundary(span.range.start) && text.is_char_boundary(span.range.end)
        }));
        assert!(spans.iter().any(|span| {
            &text[span.range.clone()] == "'naïve'" && span.kind == TokenKind::String
        }));
    }

    #[test]
    fn supports_config_data_and_markdown() {
        assert!(
            spans_of("enabled = true # flag", Language::Toml)
                .contains(&("true", TokenKind::Keyword))
        );
        assert!(spans_of("{\"count\": 42}", Language::Json).contains(&("42", TokenKind::Number)));
        assert!(
            spans_of("{\"count\": 42}", Language::Json)
                .contains(&("\"count\"", TokenKind::Property))
        );
        assert!(
            spans_of("SELECT * FROM items", Language::Sql)
                .contains(&("SELECT", TokenKind::Keyword))
        );
        assert!(
            spans_of("<div class=\"main\">", Language::Html)
                .contains(&("class", TokenKind::Property))
        );
        assert!(
            spans_of("# heading\n`code`", Language::Markdown)
                .contains(&("# heading", TokenKind::Type))
        );
    }

    #[test]
    fn skips_highlighting_for_oversized_buffers() {
        let text = "x".repeat(super::MAX_HIGHLIGHT_BYTES + 1);
        assert!(highlight(&text, Language::Rust).is_empty());
    }
}
