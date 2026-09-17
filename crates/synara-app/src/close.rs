/// A close request never treats an outstanding or failed write as a successful save.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CloseState {
    #[default]
    Open,
    Review,
    WaitingForSave,
}
impl CloseState {
    pub fn request(&mut self, dirty: bool, saving: bool) -> bool {
        if saving {
            *self = Self::WaitingForSave;
            false
        } else if dirty {
            *self = Self::Review;
            false
        } else {
            true
        }
    }
    pub fn saved(&mut self, clean: bool) -> bool {
        if *self != Self::WaitingForSave {
            return false;
        }
        if clean {
            true
        } else {
            *self = Self::Review;
            false
        }
    }
    pub fn cancel(&mut self) {
        *self = Self::Open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clean_window_closes_without_confirmation() {
        assert!(CloseState::default().request(false, false));
    }
    #[test]
    fn dirty_close_is_reviewed_and_can_be_cancelled() {
        let mut close = CloseState::default();
        assert!(!close.request(true, false));
        assert_eq!(close, CloseState::Review);
        close.cancel();
        assert_eq!(close, CloseState::Open);
        assert!(!close.saved(true));
    }
    #[test]
    fn outstanding_save_must_settle_before_close() {
        let mut close = CloseState::default();
        assert!(!close.request(false, true));
        assert_eq!(close, CloseState::WaitingForSave);
        assert!(close.saved(true));
    }
    #[test]
    fn failed_or_stale_save_returns_to_review() {
        let mut close = CloseState::WaitingForSave;
        assert!(!close.saved(false));
        assert_eq!(close, CloseState::Review);
    }
    #[test]
    fn cancelling_close_does_not_cancel_a_save_or_quit_after_it() {
        let mut close = CloseState::WaitingForSave;
        close.cancel();
        assert!(!close.saved(true));
        assert_eq!(close, CloseState::Open);
    }
}
