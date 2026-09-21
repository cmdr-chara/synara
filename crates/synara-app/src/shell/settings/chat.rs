//! Native, persisted chat behavior controls. Approval authority is unchanged.
use super::*;
impl Shell {
    pub(super) fn chat_settings(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        div().child(heading("Writing messages"))
            .child(card()
                .child(row("Enter to send", "When off, Enter adds a new line. Ctrl+Enter or Command+Enter sends. Shift+Enter always adds a new line.",
                    self.toggle("chat-send-enter", "Enter to send", self.settings.value.chat.send_on_enter, |s| s.chat.send_on_enter = !s.chat.send_on_enter, cx)))
                .child(row("Message timestamps", "Show local timestamps alongside completed assistant responses.",
                    self.toggle("chat-timestamps", "Message timestamps", self.settings.value.chat.show_timestamps, |s| s.chat.show_timestamps = !s.chat.show_timestamps, cx))))
            .child(row("Recent attachments", "Show the explicit recent-file reuse list. Hiding it never removes pending attachments or changes what will be sent.",
                self.toggle("chat-recent-files", "Recent attachments", self.settings.value.chat.show_recent_attachments, |s| s.chat.show_recent_attachments = !s.chat.show_recent_attachments, cx)))
            .child(heading("Conversation"))
            .child(card()
                .child(row("Drafts", "Unsent text is saved separately for each chat and restored without starting an agent.", "Saved automatically"))
                .child(row("Permissions", "The agent asks before actions that need approval. Changing writing preferences does not change approval policy.", "Ask permission"))
                .child(row("Work summaries", "Expand Worked for to read agent commentary and commands.", "")))
            .into_any_element()
    }
}
