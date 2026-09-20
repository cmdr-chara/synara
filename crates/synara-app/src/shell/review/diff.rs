//! Bounded, non-executing unified diff presentation. Copy uses the original text.
const MAX_LINES: usize = 6000;
const MAX_LINE_CHARS: usize = 2000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Kind {
    Context,
    Added,
    Removed,
    Hunk,
    Meta,
}
#[derive(Debug)]
pub(super) struct Line {
    pub old: Option<u64>,
    pub new: Option<u64>,
    pub text: String,
    pub kind: Kind,
}
#[derive(Default, Debug)]
pub(super) struct Diff {
    pub lines: Vec<Line>,
    pub added: usize,
    pub removed: usize,
    pub limited: bool,
}
fn start(range: &str, prefix: char) -> Option<u64> {
    range.strip_prefix(prefix)?.split(',').next()?.parse().ok()
}
fn hunk(text: &str) -> Option<(u64, u64)> {
    let mut words = text.strip_prefix("@@ ")?.split_whitespace();
    Some((start(words.next()?, '-')?, start(words.next()?, '+')?))
}
impl Diff {
    pub fn parse(raw: &str) -> Self {
        let mut result = Self::default();
        let mut positions = None;
        for text in raw.lines() {
            let mut old = None;
            let mut new = None;
            let kind = if text.starts_with("diff --git ") {
                positions = None;
                Kind::Meta
            } else if let Some(pair) = hunk(text) {
                positions = Some(pair);
                Kind::Hunk
            } else if let Some((left, right)) = positions.as_mut() {
                match text.as_bytes().first() {
                    Some(b'+') => {
                        new = Some(*right);
                        *right = right.saturating_add(1);
                        result.added += 1;
                        Kind::Added
                    }
                    Some(b'-') => {
                        old = Some(*left);
                        *left = left.saturating_add(1);
                        result.removed += 1;
                        Kind::Removed
                    }
                    Some(b' ') => {
                        old = Some(*left);
                        new = Some(*right);
                        *left = left.saturating_add(1);
                        *right = right.saturating_add(1);
                        Kind::Context
                    }
                    _ => Kind::Meta,
                }
            } else {
                Kind::Meta
            };
            if result.lines.len() < MAX_LINES {
                let clipped = text.chars().count() > MAX_LINE_CHARS;
                let mut text: String = text.chars().take(MAX_LINE_CHARS).collect();
                if clipped {
                    text.push_str(" [line shortened]");
                }
                result.limited |= clipped;
                result.lines.push(Line {
                    old,
                    new,
                    text,
                    kind,
                });
            } else {
                result.limited = true;
            }
        }
        result
    }
}

pub(super) fn draft_with_diff(
    existing: &str,
    title: &str,
    raw: &str,
) -> Result<String, &'static str> {
    if raw.is_empty() {
        return Err("There is no diff to add. Untracked and binary files may have no text diff.");
    }
    if raw.len() > 128 * 1024 {
        return Err(
            "This diff exceeds the 128 KiB draft-context limit. Select one file or copy the diff instead.",
        );
    }
    let longest = raw.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.saturating_add(1).max(3));
    let addition = format!("Git review: {title}\n\n{fence}diff\n{raw}\n{fence}");
    if existing
        .len()
        .saturating_add(addition.len())
        .saturating_add(2)
        > 1024 * 1024
    {
        return Err("The chat draft would exceed 1 MiB. Your existing text is unchanged.");
    }
    Ok(if existing.is_empty() {
        addition
    } else {
        format!("{existing}\n\n{addition}")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn headers_are_not_additions_and_hunk_line_numbers_are_exact() {
        let diff = Diff::parse(
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n@@ -10,2 +20,2 @@\n same\n-old\n+new\n\\ No newline at end of file\n",
        );
        assert_eq!((diff.added, diff.removed), (1, 1));
        assert_eq!((diff.lines[4].old, diff.lines[4].new), (Some(10), Some(20)));
        assert_eq!((diff.lines[5].old, diff.lines[5].new), (Some(11), None));
        assert_eq!((diff.lines[6].old, diff.lines[6].new), (None, Some(21)));
    }
    #[test]
    fn source_that_looks_like_a_header_still_counts_inside_a_hunk() {
        let diff = Diff::parse(
            "@@ -0,0 +1,2 @@\n+++ source\n+日本語\ndiff --git a/b b/b\nBinary files a/b and b/b differ\n",
        );
        assert_eq!(diff.added, 2);
        assert_eq!(diff.lines[1].new, Some(1));
        assert_eq!(diff.lines[4].kind, Kind::Meta);
    }
    #[test]
    fn bounded_preview_retains_total_counts_and_clips_unicode_safely() {
        let raw = format!(
            "@@ -0,0 +1,7000 @@\n{}+{}",
            "+line\n".repeat(7000),
            "界".repeat(3000)
        );
        let diff = Diff::parse(&raw);
        assert_eq!(diff.lines.len(), MAX_LINES);
        assert_eq!(diff.added, 7001);
        assert!(diff.limited);
    }
    #[test]
    fn context_append_is_non_destructive_and_fence_safe() {
        let value = draft_with_diff("Keep this", "worktree", "```\n+hello").unwrap();
        assert!(value.starts_with("Keep this\n\nGit review:"));
        assert!(value.contains("````diff\n```\n+hello\n````"));
        assert!(draft_with_diff("", "x", &"x".repeat(128 * 1024 + 1)).is_err());
        assert!(draft_with_diff(&"x".repeat(1024 * 1024), "x", "+x").is_err());
    }
}
