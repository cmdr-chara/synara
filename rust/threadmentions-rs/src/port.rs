//! The Rust port of `packages/shared/src/threadMentions.ts` (clean-room port).

#![forbid(unsafe_code)]

const PREFIX: &str = "thread://";

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

pub fn is_thread_mention_path(path: String) -> Result<bool, String> {
    Ok(path.starts_with(PREFIX))
}

pub fn thread_mention_path_for_thread_id(thread_id: String) -> Result<String, String> {
    Ok(format!("{PREFIX}{thread_id}"))
}

pub fn thread_id_from_thread_mention_path(path: String) -> Result<Option<String>, String> {
    let Some(rest) = path.strip_prefix(PREFIX) else {
        return Ok(None);
    };
    let t = js_trim(rest);
    Ok(if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    })
}
