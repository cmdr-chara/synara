//! Explicit direct-chat context projection. No file reads, hidden reasoning,
//! sessions, approvals or tool authority cross this boundary.
use super::*;
use synara_model::Content;

pub(super) struct PreparedPrompt {
    pub message: Message,
    pub display: String,
    pub context_text: String,
    pub images: Vec<TranscriptImage>,
}

pub(super) fn prepare(prompt: Prompt) -> WorkspaceResult<PreparedPrompt> {
    let mut content = Vec::new();
    let mut display = Vec::new();
    let mut context_text = String::new();
    let mut images = Vec::new();
    for part in prompt.parts {
        match part {
            PromptPart::Text(text) => {
                display.push(text.clone());
                content.push(Content::Text { text });
            }
            PromptPart::Context {
                uri,
                text,
                mime_type,
            } => {
                if mime_type != "text/plain"
                    || uri.len() > 2048
                    || uri.chars().any(char::is_control)
                    || text.len() > crate::MAX_ATTACHMENT_BATCH_BYTES
                {
                    return Err(model_error(synara_model::ModelError::Unsupported(
                        "only bounded, explicitly attached plain-text context is supported",
                    )));
                }
                display.push(format!("[Context: {uri}]"));
                let visible = format!("\n\n[Attached text: {uri}]\n{text}");
                context_text.push_str(&visible);
                content.push(Content::Text { text: visible });
            }
            PromptPart::MediaImage(image) => {
                if !image.bounded() {
                    return Err(model_error(synara_model::ModelError::Invalid(
                        "inline image",
                    )));
                }
                display.push("[Image]".into());
                content.push(Content::Image {
                    media_type: image.mime_type.clone(),
                    base64: image.base64.clone(),
                });
                images.push(image);
            }
            PromptPart::Image { base64, mime_type } => {
                let image = TranscriptImage {
                    source: ImageSource::Uploaded,
                    mime_type,
                    base64,
                };
                if !image.bounded() {
                    return Err(model_error(synara_model::ModelError::Invalid(
                        "inline image",
                    )));
                }
                display.push("[Image]".into());
                content.push(Content::Image {
                    media_type: image.mime_type.clone(),
                    base64: image.base64.clone(),
                });
                images.push(image);
            }
            PromptPart::Audio { .. } => {
                return Err(model_error(synara_model::ModelError::Unsupported(
                    "direct audio input is not implemented",
                )));
            }
        }
    }
    Ok(PreparedPrompt {
        message: Message {
            role: MessageRole::User,
            content,
            tool_calls: vec![],
            tool_call_id: None,
        },
        display: display.join("\n"),
        context_text,
        images,
    })
}

/// Limit prior user turns only when the user explicitly reviewed that policy.
/// The current prompt is added by the caller and the durable transcript is never
/// truncated. Select references before cloning any retained image payloads.
pub(super) fn history(
    thread: &Thread,
    selection: &ModelSelection,
) -> WorkspaceResult<Vec<Message>> {
    selection.validate_context().map_err(model_error)?;
    let candidates: Vec<_> = thread
        .messages
        .iter()
        .filter(|m| matches!(m.role, Role::User | Role::Assistant))
        .collect();
    let start = match selection.history_turns {
        None => 0,
        Some(0) => candidates.len(),
        Some(limit) => {
            let starts: Vec<_> = candidates
                .iter()
                .enumerate()
                .filter_map(|(index, m)| (m.role == Role::User).then_some(index))
                .collect();
            starts
                .get(starts.len().saturating_sub(usize::from(limit)))
                .copied()
                .unwrap_or(candidates.len())
        }
    };
    let mut messages = Vec::new();
    for source in &candidates[start..] {
        let role = if source.role == Role::User {
            MessageRole::User
        } else {
            MessageRole::Assistant
        };
        let mut content = Vec::new();
        if !source.text.is_empty() {
            content.push(Content::Text {
                text: source.text.clone(),
            });
        }
        for item in thread
            .images
            .iter()
            .filter(|i| i.message_id == source.id && i.role == source.role)
        {
            if source.role != Role::User {
                return Err(WorkspaceError::Invalid("This history includes an assistant-returned image, which direct chat cannot replay. Review a smaller history window before sending.".into()));
            }
            if !item.image.bounded() {
                return Err(model_error(synara_model::ModelError::Invalid(
                    "stored inline image",
                )));
            }
            content.push(Content::Image {
                media_type: item.image.mime_type.clone(),
                base64: item.image.base64.clone(),
            });
        }
        if !content.is_empty() {
            messages.push(Message {
                role,
                content,
                tool_calls: vec![],
                tool_call_id: None,
            });
        }
    }
    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn selection(history_turns: Option<u16>) -> ModelSelection {
        ModelSelection {
            provider_id: "fixture".into(),
            model_id: "fixture".into(),
            max_output_tokens: 32,
            reasoning_effort: None,
            output: Default::default(),
            history_turns,
        }
    }
    fn image() -> TranscriptImage {
        TranscriptImage {
            source: ImageSource::AppSnap,
            mime_type: "image/png".into(),
            base64: "AAAA".into(),
        }
    }
    fn append(thread: &mut Thread, event: ThreadEvent) {
        thread
            .apply(&EventEnvelope {
                id: EventId::new(),
                thread_id: thread.id,
                sequence: thread.last_sequence + 1,
                timestamp_ms: 1,
                event,
            })
            .unwrap();
    }
    fn text(thread: &mut Thread, id: &str, role: Role, value: &str) {
        append(
            thread,
            ThreadEvent::TextDelta {
                message_id: Some(id.into()),
                role,
                text: value.into(),
            },
        );
    }
    #[test]
    fn plain_text_projection_is_unchanged() {
        let prepared = prepare(Prompt::text("hello\nworld")).unwrap();
        assert_eq!(prepared.display, "hello\nworld");
        assert!(prepared.context_text.is_empty() && prepared.images.is_empty());
        assert!(
            matches!(&prepared.message.content[0], Content::Text { text } if text == "hello\nworld")
        );
    }
    #[test]
    fn attached_text_has_matching_local_echo_and_visible_replay_content() {
        let prepared = prepare(Prompt {
            parts: vec![
                PromptPart::Text("review".into()),
                PromptPart::Text("Attached file: notes.txt".into()),
                PromptPart::Context {
                    uri: "synara-attachment://owned".into(),
                    text: "visible sentinel".into(),
                    mime_type: "text/plain".into(),
                },
            ],
        })
        .unwrap();
        assert_eq!(
            prepared.display,
            "review\nAttached file: notes.txt\n[Context: synara-attachment://owned]"
        );
        assert!(prepared.context_text.contains("visible sentinel"));
        assert!(
            matches!(prepared.message.content.last().unwrap(), Content::Text { text } if text.contains("visible sentinel"))
        );
    }
    #[test]
    fn image_bytes_and_capture_provenance_survive_projection() {
        let prepared = prepare(Prompt {
            parts: vec![
                PromptPart::Text("look".into()),
                PromptPart::MediaImage(image()),
            ],
        })
        .unwrap();
        assert_eq!(prepared.display, "look\n[Image]");
        assert_eq!(prepared.images, vec![image()]);
        assert!(
            matches!(&prepared.message.content[1], Content::Image { media_type, base64 } if media_type == "image/png" && base64 == "AAAA")
        );
    }
    #[test]
    fn unsupported_parts_are_errors_not_silent_omissions() {
        assert!(
            prepare(Prompt {
                parts: vec![PromptPart::Audio {
                    base64: "AAAA".into(),
                    mime_type: "audio/wav".into()
                }]
            })
            .is_err()
        );
        assert!(
            prepare(Prompt {
                parts: vec![PromptPart::Context {
                    uri: "owned".into(),
                    text: "data".into(),
                    mime_type: "application/pdf".into()
                }]
            })
            .is_err()
        );
        let mut invalid = image();
        invalid.mime_type = "image/svg+xml".into();
        assert!(
            prepare(Prompt {
                parts: vec![PromptPart::MediaImage(invalid)]
            })
            .is_err()
        );
    }
    #[test]
    fn history_keeps_matching_user_images_and_excludes_hidden_reasoning() {
        let mut thread = Thread::new(ThreadId::new());
        text(&mut thread, "user", Role::User, "look");
        append(
            &mut thread,
            ThreadEvent::ImageMessage {
                message_id: Some("user".into()),
                role: Role::User,
                image: image(),
            },
        );
        text(&mut thread, "hidden", Role::Reasoning, "never transmit");
        text(&mut thread, "answer", Role::Assistant, "answer");
        let messages = history(&thread, &selection(None)).unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content.len(), 2);
        assert!(
            !serde_json::to_string(&messages)
                .unwrap()
                .contains("never transmit")
        );
        assert_eq!(thread.images[0].image.source, ImageSource::AppSnap);
    }
    #[test]
    fn explicit_window_keeps_whole_turns_without_mutating_history() {
        let mut thread = Thread::new(ThreadId::new());
        for i in 0..3 {
            text(
                &mut thread,
                &format!("u{i}"),
                Role::User,
                &format!("question {i}"),
            );
            text(
                &mut thread,
                &format!("a{i}"),
                Role::Assistant,
                &format!("answer {i}"),
            );
        }
        let before = thread.last_sequence;
        let messages = history(&thread, &selection(Some(2))).unwrap();
        assert_eq!(messages.len(), 4);
        assert!(matches!(&messages[0].content[0], Content::Text { text } if text == "question 1"));
        assert!(history(&thread, &selection(Some(0))).unwrap().is_empty());
        assert_eq!(thread.messages.len(), 6);
        assert_eq!(thread.last_sequence, before);
        assert!(history(&thread, &selection(Some(257))).is_err());
    }
    #[test]
    fn reviewed_window_drops_orphan_assistant_prefix() {
        let mut thread = Thread::new(ThreadId::new());
        text(&mut thread, "orphan", Role::Assistant, "old answer");
        text(&mut thread, "user", Role::User, "new question");
        assert_eq!(history(&thread, &selection(Some(10))).unwrap().len(), 1);
        assert_eq!(history(&thread, &selection(None)).unwrap().len(), 2);
    }
    #[test]
    fn assistant_images_are_not_relabelled_or_silently_dropped() {
        let mut thread = Thread::new(ThreadId::new());
        append(
            &mut thread,
            ThreadEvent::ImageMessage {
                message_id: Some("assistant-image".into()),
                role: Role::Assistant,
                image: image(),
            },
        );
        assert!(history(&thread, &selection(None)).is_err());
        assert!(history(&thread, &selection(Some(0))).unwrap().is_empty());
    }
}
