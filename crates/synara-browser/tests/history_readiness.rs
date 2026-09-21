//! A network request is not a native document commit. Exercise the real Session
//! API while deliberately holding callbacks, without a browser, timer or sleep.
use std::sync::{Arc, Mutex};
use synara_browser::session::{Capabilities, Command, Event, NativePort, Session};
use synara_browser::{BrowserError, BrowserProfile, NavigationKind, Result};

struct Port(Arc<Mutex<Vec<Command>>>);
impl NativePort for Port {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            navigation: true,
            ..Capabilities::default()
        }
    }
    fn send(&mut self, command: Command) -> Result<()> {
        self.0.lock().unwrap().push(command);
        Ok(())
    }
}
fn last(commands: &Arc<Mutex<Vec<Command>>>) -> Command {
    commands.lock().unwrap().last().unwrap().clone()
}
fn committed(command: Command) -> Event {
    let Command::Navigate {
        tab,
        navigation,
        document,
        ..
    } = command
    else {
        panic!("Expected an actual navigation dispatch");
    };
    Event::Committed {
        tab,
        navigation,
        url: document.canonical_url,
        title: "Fixture".into(),
    }
}

#[test]
fn forward_is_valid_only_after_the_pending_back_commits() {
    let commands = Arc::new(Mutex::new(Vec::new()));
    let mut session = Session::new(Box::new(Port(commands.clone())));
    let tab = session.open(BrowserProfile::Manual).unwrap();
    for path in ["first", "second"] {
        session
            .user_navigate(
                tab,
                &format!("https://example.test/{path}"),
                NavigationKind::Push,
                0,
            )
            .unwrap();
        session.event(committed(last(&commands))).unwrap();
    }
    session
        .user_navigate(tab, "", NavigationKind::Back, 1)
        .unwrap();
    let back = last(&commands);
    let count = commands.lock().unwrap().len();
    let pending = session.tabs().remove(0);
    assert_eq!(pending.state, "loading");
    assert_eq!(pending.url.as_deref(), Some("https://example.test/second"));
    assert!(!pending.forward);
    // This is the old native UI's failing sequence: the request has been sent,
    // but the engine's completed-document callback has not arrived yet.
    assert_eq!(
        session.user_navigate(tab, "", NavigationKind::Forward, 2),
        Err(BrowserError::Invalid)
    );
    assert_eq!(commands.lock().unwrap().len(), count);
    session.event_at(committed(back), 3).unwrap();
    let ready = session.tabs().remove(0);
    assert_eq!(ready.state, "ready");
    assert!(!ready.back);
    assert!(ready.forward);
    session
        .user_navigate(tab, "", NavigationKind::Forward, 4)
        .unwrap();
    session.event_at(committed(last(&commands)), 5).unwrap();
    assert_eq!(
        session.tabs()[0].url.as_deref(),
        Some("https://example.test/second")
    );
}

#[test]
fn a_new_explicit_address_supersedes_pending_history_without_replaying_a_callback() {
    let commands = Arc::new(Mutex::new(Vec::new()));
    let mut session = Session::new(Box::new(Port(commands.clone())));
    let tab = session.open(BrowserProfile::Manual).unwrap();
    for path in ["first", "second"] {
        session
            .user_navigate(
                tab,
                &format!("https://example.test/{path}"),
                NavigationKind::Push,
                0,
            )
            .unwrap();
        session.event(committed(last(&commands))).unwrap();
    }
    session
        .user_navigate(tab, "", NavigationKind::Back, 1)
        .unwrap();
    let stale = committed(last(&commands));
    session
        .user_navigate(tab, "https://example.test/third", NavigationKind::Push, 2)
        .unwrap();
    assert_eq!(
        session.event_at(stale, 3),
        Err(BrowserError::MissingNavigation)
    );
    assert_eq!(session.tabs()[0].state, "loading");
    session.event_at(committed(last(&commands)), 4).unwrap();
    assert_eq!(
        session.tabs()[0].url.as_deref(),
        Some("https://example.test/third")
    );
    assert!(!session.tabs()[0].forward);
}
