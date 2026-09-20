//! Composer key policy leaves single-line controls and file editors independent.
use super::EntryMode;
pub(super) fn submits_enter(
    mode: EntryMode,
    send_on_enter: bool,
    command: bool,
    shift: bool,
    alt: bool,
) -> bool {
    match mode {
        EntryMode::SingleLine => true,
        EntryMode::Composer => !shift && !alt && (command || send_on_enter),
        EntryMode::Editor => command,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_enter_is_newline_but_control_or_command_enter_sends() {
        assert!(!submits_enter(
            EntryMode::Composer,
            false,
            false,
            false,
            false
        ));
        assert!(submits_enter(
            EntryMode::Composer,
            false,
            true,
            false,
            false
        ));
        assert!(submits_enter(
            EntryMode::Composer,
            true,
            false,
            false,
            false
        ));
    }
    #[test]
    fn shift_and_alt_enter_do_not_send_composer_text() {
        for enabled in [false, true] {
            for command in [false, true] {
                assert!(!submits_enter(
                    EntryMode::Composer,
                    enabled,
                    command,
                    true,
                    false
                ));
                assert!(!submits_enter(
                    EntryMode::Composer,
                    enabled,
                    command,
                    false,
                    true
                ));
            }
        }
    }
    #[test]
    fn file_and_single_line_controls_do_not_inherit_chat_policy() {
        assert!(submits_enter(
            EntryMode::SingleLine,
            false,
            false,
            false,
            false
        ));
        assert!(!submits_enter(EntryMode::Editor, true, false, false, false));
        assert!(submits_enter(EntryMode::Editor, false, true, false, false));
    }
}
