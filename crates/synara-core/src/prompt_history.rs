//! Upstream `ChatView.logic.ts` prompt-history navigation: composer Up/Down
//! recalls the task's sent prompts and restores the in-progress draft when
//! navigation exits. Starting recall never replaces a non-empty draft.
use crate::{Message, Role};

/// Upstream `PROMPT_HISTORY_MAX_ENTRIES`.
pub const PROMPT_HISTORY_MAX_ENTRIES: usize = 100;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PromptHistoryDirection {
    Older,
    Newer,
}

/// The active history browse: which entry is showing and the draft it saved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptHistoryState {
    pub index: usize,
    pub draft: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PromptHistoryNavigation {
    pub handled: bool,
    pub prompt: String,
    /// Byte offset where the caret lands on the resulting prompt.
    pub expanded_cursor: usize,
    pub state: Option<PromptHistoryState>,
}

/// Upstream `derivePromptHistoryFromMessages`: newest-first sent user prompts.
/// Rust user messages carry no `source` marker, so this cannot exclude
/// imported text the way upstream's `source === "native"` filter does.
pub fn derive_prompt_history(messages: &[Message]) -> Vec<String> {
    messages
        .iter()
        .rev()
        .filter(|message| message.role == Role::User)
        .map(|message| message.text.trim())
        .filter(|prompt| !prompt.is_empty())
        .take(PROMPT_HISTORY_MAX_ENTRIES)
        .map(str::to_owned)
        .collect()
}

fn cursor_on_first_line(prompt: &str, cursor: usize) -> bool {
    match prompt.find('\n') {
        Some(first_line_end) => cursor.min(prompt.len()) <= first_line_end,
        None => true,
    }
}

fn cursor_on_last_line(prompt: &str, cursor: usize) -> bool {
    cursor.min(prompt.len()) >= prompt.rfind('\n').map_or(0, |last| last + 1)
}

/// Upstream `expandedCursorForPromptHistoryItem`: Up lands on the first line's
/// end, Down at the end of the prompt.
fn history_item_cursor(prompt: &str, direction: PromptHistoryDirection) -> usize {
    match direction {
        PromptHistoryDirection::Older => prompt.find('\n').unwrap_or(prompt.len()),
        PromptHistoryDirection::Newer => prompt.len(),
    }
}

/// Upstream `resolvePromptHistoryNavigation`. `history` is newest-first.
pub fn resolve_prompt_history_navigation(
    direction: PromptHistoryDirection,
    history: &[String],
    prompt: &str,
    cursor: usize,
    selection_collapsed: bool,
    state: Option<&PromptHistoryState>,
) -> PromptHistoryNavigation {
    let not_handled = |state: Option<PromptHistoryState>| PromptHistoryNavigation {
        handled: false,
        prompt: prompt.to_owned(),
        expanded_cursor: cursor,
        state,
    };
    if !selection_collapsed || history.is_empty() {
        return not_handled(state.cloned());
    }
    // A browse that lost its place (history changed, or the text no longer
    // shows the active entry) restarts from the newest entry going older, or
    // restores the saved draft going newer — never navigating a bogus index.
    let stale = state.is_some_and(|state| {
        history
            .get(state.index)
            .is_none_or(|active| prompt != active)
    });
    if direction == PromptHistoryDirection::Older {
        if state.is_none() && !prompt.is_empty() {
            return not_handled(None);
        }
        if !cursor_on_first_line(prompt, cursor) {
            return not_handled(state.cloned());
        }
        let next = match state {
            None => PromptHistoryState {
                index: 0,
                draft: prompt.to_owned(),
            },
            Some(state) if stale => PromptHistoryState {
                index: 0,
                draft: state.draft.clone(),
            },
            Some(state) => PromptHistoryState {
                index: (state.index + 1).min(history.len() - 1),
                draft: state.draft.clone(),
            },
        };
        let next_prompt = history
            .get(next.index)
            .cloned()
            .unwrap_or_else(|| prompt.to_owned());
        return PromptHistoryNavigation {
            handled: true,
            expanded_cursor: history_item_cursor(&next_prompt, direction),
            prompt: next_prompt,
            state: Some(next),
        };
    }
    let Some(state) = state else {
        return not_handled(None);
    };
    if !cursor_on_last_line(prompt, cursor) && !cursor_on_first_line(prompt, cursor) {
        return not_handled(Some(state.clone()));
    }
    if stale || state.index == 0 {
        return PromptHistoryNavigation {
            handled: true,
            expanded_cursor: state.draft.len(),
            prompt: state.draft.clone(),
            state: None,
        };
    }
    let next = PromptHistoryState {
        index: state.index - 1,
        draft: state.draft.clone(),
    };
    let next_prompt = history
        .get(next.index)
        .cloned()
        .unwrap_or_else(|| prompt.to_owned());
    PromptHistoryNavigation {
        handled: true,
        expanded_cursor: history_item_cursor(&next_prompt, direction),
        prompt: next_prompt,
        state: Some(next),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Message;

    fn history() -> Vec<String> {
        ["third prompt", "second prompt", "first prompt"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn up_from_empty_composer_recalls_newest_and_down_restores_draft() {
        let history = history();
        let first = resolve_prompt_history_navigation(
            PromptHistoryDirection::Older,
            &history,
            "",
            0,
            true,
            None,
        );
        assert!(first.handled);
        assert_eq!(first.prompt, "third prompt");
        assert_eq!(first.expanded_cursor, "third prompt".len());
        assert_eq!(
            first.state,
            Some(PromptHistoryState {
                index: 0,
                draft: String::new()
            })
        );
        let second = resolve_prompt_history_navigation(
            PromptHistoryDirection::Older,
            &history,
            &first.prompt,
            first.expanded_cursor,
            true,
            first.state.as_ref(),
        );
        assert_eq!(second.prompt, "second prompt");
        assert_eq!(second.state.as_ref().map(|s| s.index), Some(1));
        let back = resolve_prompt_history_navigation(
            PromptHistoryDirection::Newer,
            &history,
            &second.prompt,
            second.expanded_cursor,
            true,
            second.state.as_ref(),
        );
        assert_eq!(back.prompt, "third prompt");
        let exit = resolve_prompt_history_navigation(
            PromptHistoryDirection::Newer,
            &history,
            &back.prompt,
            back.expanded_cursor,
            true,
            back.state.as_ref(),
        );
        assert!(exit.handled);
        assert_eq!(exit.prompt, "");
        assert_eq!(exit.state, None);
    }

    #[test]
    fn up_from_typed_draft_is_not_handled() {
        // Upstream 2e41dd03c: recall must not replace text still being edited.
        for prompt in ["draft in progress", "first\nsecond", " \n"] {
            for cursor in [0, prompt.len() / 2, prompt.len()] {
                let result = resolve_prompt_history_navigation(
                    PromptHistoryDirection::Older,
                    &history(),
                    prompt,
                    cursor,
                    true,
                    None,
                );
                assert_eq!(result, not_handled(prompt, cursor));
            }
        }
    }

    #[test]
    fn up_requires_first_line_cursor_and_down_requires_line_edge() {
        let history = vec!["old".to_string()];
        let prompt = "first\nsecond";
        assert_eq!(
            resolve_prompt_history_navigation(
                PromptHistoryDirection::Older,
                &history,
                prompt,
                8,
                true,
                None
            ),
            not_handled(prompt, 8)
        );
        // In an active browse, Down from a mid-line caret (a middle line of
        // three) keeps normal caret movement instead of navigating.
        let entered = resolve_prompt_history_navigation(
            PromptHistoryDirection::Older,
            &history,
            "",
            0,
            true,
            None,
        );
        let mid_line = resolve_prompt_history_navigation(
            PromptHistoryDirection::Newer,
            &history,
            "a\nb\nc",
            3,
            true,
            entered.state.as_ref(),
        );
        assert!(!mid_line.handled);
        assert_eq!(mid_line.state, entered.state);
    }

    #[test]
    fn stale_state_restarts_at_newest_and_never_drops_draft() {
        let history = history();
        let entered = resolve_prompt_history_navigation(
            PromptHistoryDirection::Older,
            &history,
            "",
            0,
            true,
            None,
        );
        let typed_over = resolve_prompt_history_navigation(
            PromptHistoryDirection::Older,
            &history,
            "edited text",
            0,
            true,
            entered.state.as_ref(),
        );
        assert_eq!(typed_over.prompt, "third prompt");
        assert_eq!(typed_over.state.as_ref().map(|s| s.index), Some(0));
        assert_eq!(
            typed_over.state.as_ref().map(|s| s.draft.as_str()),
            Some("")
        );
    }

    #[test]
    fn selection_or_empty_history_is_not_handled() {
        let history = history();
        assert_eq!(
            resolve_prompt_history_navigation(
                PromptHistoryDirection::Older,
                &history,
                "",
                0,
                false,
                None
            ),
            not_handled("", 0)
        );
        assert_eq!(
            resolve_prompt_history_navigation(
                PromptHistoryDirection::Older,
                &[],
                "",
                0,
                true,
                None
            ),
            not_handled("", 0)
        );
    }

    fn not_handled(prompt: &str, cursor: usize) -> PromptHistoryNavigation {
        PromptHistoryNavigation {
            handled: false,
            prompt: prompt.to_owned(),
            expanded_cursor: cursor,
            state: None,
        }
    }

    #[test]
    fn derive_collects_newest_nonempty_user_prompts_up_to_cap() {
        let mut messages = Vec::new();
        for index in 0..(PROMPT_HISTORY_MAX_ENTRIES + 10) {
            messages.push(Message {
                id: format!("m{index}"),
                role: Role::User,
                text: if index == 5 {
                    "   ".into()
                } else {
                    format!("prompt {index}")
                },
            });
        }
        messages.push(Message {
            id: "assistant".into(),
            role: Role::Assistant,
            text: "reply".into(),
        });
        let history = derive_prompt_history(&messages);
        assert_eq!(history.len(), PROMPT_HISTORY_MAX_ENTRIES);
        assert_eq!(history[0], "prompt 109");
        assert!(!history.iter().any(|prompt| prompt.trim().is_empty()));
    }
}
