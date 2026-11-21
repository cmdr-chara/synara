//! The Rust port of `apps/server/src/hostConnectivitySignals.ts`.

#![forbid(unsafe_code)]

thread_local! {
    static REMOTE_ACCESS_GRANTED: std::cell::RefCell<Option<Box<dyn Fn()>>> =
        const { std::cell::RefCell::new(None) };
    static HOST_AUTHORIZATION_NUDGE: std::cell::RefCell<Option<Box<dyn Fn()>>> =
        const { std::cell::RefCell::new(None) };
}

/// Port of `noteRemoteAccessGranted` (`apps/server/src/hostConnectivitySignals.ts:19`).
/// Never fails: a billing read must not fail because connectivity is unavailable.
pub fn note_remote_access_granted() -> Result<(), String> {
    REMOTE_ACCESS_GRANTED.with(|slot| {
        if let Some(wake) = slot.borrow().as_ref() {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(wake));
        }
    });
    Ok(())
}

/// Port of `nudgeHostAuthorization` (`apps/server/src/hostConnectivitySignals.ts:39`).
/// Never fails: revocation already succeeded at the account service.
pub fn nudge_host_authorization() -> Result<(), String> {
    HOST_AUTHORIZATION_NUDGE.with(|slot| {
        if let Some(nudge) = slot.borrow().as_ref() {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(nudge));
        }
    });
    Ok(())
}
