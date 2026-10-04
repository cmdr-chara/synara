use super::*;
use std::sync::{Arc, Mutex};
struct Port(Arc<Mutex<Vec<Command>>>);
impl NativePort for Port {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            navigation: true,
            document: true,
            input: true,
            capture: true,
            downloads: true,
            uploads: true,
        }
    }
    fn send(&mut self, c: Command) -> Result<()> {
        self.0.lock().unwrap().push(c);
        Ok(())
    }
}
fn fixture() -> (Session, Arc<Mutex<Vec<Command>>>) {
    let commands = Arc::new(Mutex::new(Vec::new()));
    (Session::new(Box::new(Port(commands.clone()))), commands)
}
fn navigate(s: &mut Session, tab: HostTabId, task: u128, url: &str) -> HostRequestId {
    let id = s
        .request(task, tab, BrowserOperation::Navigate { url: url.into() }, 0)
        .unwrap();
    s.decide(id, true, 1).unwrap();
    let nav = s.tabs[&tab].navigation.as_ref().unwrap().0;
    s.event(Event::Committed {
        tab,
        navigation: nav,
        url: url.into(),
        title: "Test".into(),
    })
    .unwrap();
    id
}
fn loaded(s: &mut Session) -> HostTabId {
    let tab = s.open(BrowserProfile::AgentTask { task: 7 }).unwrap();
    navigate(s, tab, 7, "https://example.test/");
    tab
}
#[test]
fn manual_network_diagnostics_are_redacted_bounded_and_not_agent_visible() {
    let (mut s, _) = fixture();
    let manual = s.open(BrowserProfile::Manual).unwrap();
    s.user_navigate(manual, "https://example.test/", NavigationKind::Push, 0)
        .unwrap();
    let navigation = s.tabs[&manual].navigation.as_ref().unwrap().0;
    s.event(Event::NetworkDiagnostic {
        tab: manual,
        navigation,
        url: "https://user:secret@example.test/path?token=secret#secret".into(),
        status: Some(200),
        error: None,
    })
    .unwrap();
    let entries = s.manual_diagnostics(manual).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].url, "https://example.test/path");
    assert_eq!(entries[0].status, Some(200));
    let agent = s.open(BrowserProfile::AgentTask { task: 1 }).unwrap();
    assert!(s.manual_diagnostics(agent).is_err());
    s.clear_manual_diagnostics(manual).unwrap();
    assert!(s.manual_diagnostics(manual).unwrap().is_empty());
}
#[test]
fn runtime_diagnostics_are_bounded_manual_only_and_navigation_scoped() {
    let (mut s, _) = fixture();
    let manual = s.open(BrowserProfile::Manual).unwrap();
    s.user_navigate(manual, "https://example.test/", NavigationKind::Push, 0)
        .unwrap();
    let first_navigation = s.tabs[&manual].navigation.as_ref().unwrap().0;
    for diagnostic in [
        RuntimeDiagnostic {
            kind: RuntimeDiagnosticKind::UncaughtException,
            line: Some(10_000_001),
            column: None,
        },
        RuntimeDiagnostic {
            kind: RuntimeDiagnosticKind::UncaughtException,
            line: None,
            column: Some(10_000_001),
        },
    ] {
        s.event(Event::RuntimeDiagnostic {
            tab: manual,
            navigation: first_navigation,
            diagnostic,
        })
        .unwrap();
    }
    assert!(s.manual_runtime_diagnostics(manual).unwrap().is_empty());
    assert!(
        serde_json::from_str::<RuntimeDiagnostic>(
            r#"{"kind":"uncaught_exception","line":1,"column":2,"text":"secret"}"#
        )
        .is_err()
    );

    for line in 0..205 {
        s.event(Event::RuntimeDiagnostic {
            tab: manual,
            navigation: first_navigation,
            diagnostic: RuntimeDiagnostic {
                kind: RuntimeDiagnosticKind::UncaughtException,
                line: Some(line),
                column: Some(4),
            },
        })
        .unwrap();
    }
    let entries = s.manual_runtime_diagnostics(manual).unwrap();
    assert_eq!(entries.len(), 200);
    assert_eq!(entries.first().unwrap().line, Some(5));
    assert_eq!(entries.last().unwrap().line, Some(204));

    let agent = s.open(BrowserProfile::AgentTask { task: 9 }).unwrap();
    assert!(s.manual_runtime_diagnostics(agent).is_err());
    assert!(s.clear_manual_runtime_diagnostics(agent).is_err());

    s.clear_manual_runtime_diagnostics(manual).unwrap();
    assert!(s.manual_runtime_diagnostics(manual).unwrap().is_empty());

    s.user_navigate(manual, "https://example.test/next", NavigationKind::Push, 1)
        .unwrap();
    let next_navigation = s.tabs[&manual].navigation.as_ref().unwrap().0;
    s.event(Event::RuntimeDiagnostic {
        tab: manual,
        navigation: first_navigation,
        diagnostic: RuntimeDiagnostic {
            kind: RuntimeDiagnosticKind::UnhandledRejection,
            line: None,
            column: None,
        },
    })
    .unwrap();
    assert!(s.manual_runtime_diagnostics(manual).unwrap().is_empty());
    s.event(Event::RuntimeDiagnostic {
        tab: manual,
        navigation: next_navigation,
        diagnostic: RuntimeDiagnostic {
            kind: RuntimeDiagnosticKind::UnhandledRejection,
            line: None,
            column: None,
        },
    })
    .unwrap();
    assert_eq!(s.manual_runtime_diagnostics(manual).unwrap().len(), 1);
}
#[test]
fn popup_requires_explicit_manual_action_and_stays_in_owned_tab() {
    let (mut s, _) = fixture();
    let source = s.open(BrowserProfile::Manual).unwrap();
    s.user_navigate(source, "https://example.test/", NavigationKind::Push, 0)
        .unwrap();
    let navigation = s.tabs[&source].navigation.as_ref().unwrap().0;
    s.event(Event::Committed {
        tab: source,
        navigation,
        url: "https://example.test/".into(),
        title: "Source".into(),
    })
    .unwrap();
    s.event(Event::PopupRequested {
        tab: source,
        navigation,
        url: "https://example.test/login?nonce=secret".into(),
    })
    .unwrap();
    assert_eq!(s.tabs().len(), 1);
    assert_eq!(
        s.manual_popup_preview(source).unwrap().as_deref(),
        Some("https://example.test/login")
    );
    let (target, target_url) = s.open_manual_popup(source, 1).unwrap();
    assert_eq!(s.tabs().len(), 2);
    assert_eq!(target_url, "https://example.test/login?nonce=secret");
    assert_eq!(s.tabs[&target].view.profile, BrowserProfile::Manual);
    assert!(s.manual_popup_preview(source).unwrap().is_none());
    let agent = s.open(BrowserProfile::AgentTask { task: 8 }).unwrap();
    assert!(s.manual_popup_preview(agent).is_err());
}
#[test]
fn canonical_url_rejects_scheme_credentials_controls_and_origin_forgery() {
    for url in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/html,x",
        "https://u:p@example.test/",
        " https://example.test/",
        "https://example.test/\n",
        "https://example.test\\evil/",
    ] {
        assert!(CommittedDocument::parse(url).is_err(), "{url:?}");
    }
    let mut doc = CommittedDocument::parse("https://example.test/").unwrap();
    doc.origin.host = "other.test".into();
    assert!(doc.validate().is_err());
    assert!(CommittedDocument::parse("http://[::1]:3000/").is_ok());
}
#[test]
fn blank_agent_tab_requires_consent_before_navigation() {
    let (mut s, commands) = fixture();
    let tab = s.open(BrowserProfile::AgentTask { task: 7 }).unwrap();
    assert!(
        s.request(7, tab, BrowserOperation::ReadDocument, 0)
            .is_err()
    );
    let r = s
        .request(
            7,
            tab,
            BrowserOperation::Navigate {
                url: "https://example.test/".into(),
            },
            0,
        )
        .unwrap();
    assert_eq!(commands.lock().unwrap().len(), 1);
    s.decide(r, true, 1).unwrap();
    assert!(s.decide(r, true, 2).is_err());
    assert!(matches!(
        &commands.lock().unwrap()[1],
        Command::Navigate {
            partition: StoragePartition::AgentTask(7),
            allowed_origin: Some(_),
            ..
        }
    ));
}
#[test]
fn manual_and_other_task_tabs_are_not_exposed_to_agent() {
    let (mut s, _) = fixture();
    let manual = s.open(BrowserProfile::Manual).unwrap();
    let agent = loaded(&mut s);
    assert!(
        s.request(7, manual, BrowserOperation::ReadDocument, 0)
            .is_err()
    );
    assert!(
        s.request(8, agent, BrowserOperation::ReadDocument, 0)
            .is_err()
    );
    assert_eq!(s.task_tabs(7), vec![agent]);
    assert!(s.task_tabs(8).is_empty());
    assert!(
        s.user_navigate(agent, "https://example.test/", NavigationKind::Push, 0)
            .is_err()
    );
}
#[test]
fn denied_expired_and_replayed_requests_do_not_run() {
    let (mut s, _) = fixture();
    let tab = loaded(&mut s);
    let r = s
        .request(7, tab, BrowserOperation::ReadDocument, 0)
        .unwrap();
    s.decide(r, false, 1).unwrap();
    assert!(s.decide(r, true, 2).is_err());
    let r = s
        .request(7, tab, BrowserOperation::ReadDocument, 0)
        .unwrap();
    s.tick(60_000);
    assert!(s.decide(r, true, 60_000).is_err());
    assert!(s.result(8, r).is_err());
}
#[test]
fn navigation_invalidates_pending_input_and_observation() {
    let (mut s, _) = fixture();
    let tab = loaded(&mut s);
    let old = s
        .request(7, tab, BrowserOperation::ReadDocument, 0)
        .unwrap();
    navigate(&mut s, tab, 7, "https://example.test/next");
    assert!(s.decide(old, true, 2).is_err());
}
#[test]
fn cross_origin_redirect_and_late_callbacks_fail_closed() {
    let (mut s, _) = fixture();
    let tab = loaded(&mut s);
    let r = s
        .request(
            7,
            tab,
            BrowserOperation::Navigate {
                url: "https://example.test/next".into(),
            },
            0,
        )
        .unwrap();
    s.decide(r, true, 1).unwrap();
    let nav = s.tabs[&tab].navigation.as_ref().unwrap().0;
    assert!(
        s.event(Event::Committed {
            tab,
            navigation: nav,
            url: "https://other.test/".into(),
            title: "x".into()
        })
        .is_err()
    );
    assert_eq!(s.tabs[&tab].view.state, "failed");
    assert!(
        s.event(Event::Committed {
            tab,
            navigation: nav,
            url: "https://example.test/next".into(),
            title: "x".into()
        })
        .is_err()
    );
}
#[test]
fn document_elements_are_bounded_and_invalidated() {
    let (mut s, _) = fixture();
    let tab = loaded(&mut s);
    assert!(
        s.request(
            7,
            tab,
            BrowserOperation::Click {
                element: "unknown".into()
            },
            0
        )
        .is_err()
    );
    let r = s
        .request(7, tab, BrowserOperation::ReadDocument, 0)
        .unwrap();
    s.decide(r, true, 1).unwrap();
    s.event(Event::Output {
        request: r,
        output: Output::Document {
            text: "untrusted".into(),
            elements: vec![Element {
                id: "e1".into(),
                role: "button".into(),
                name: "Submit".into(),
            }],
        },
    })
    .unwrap();
    let click = s
        .request(
            7,
            tab,
            BrowserOperation::Click {
                element: "e1".into(),
            },
            2,
        )
        .unwrap();
    navigate(&mut s, tab, 7, "https://example.test/new");
    assert!(s.decide(click, true, 3).is_err());
    assert!(
        s.request(
            7,
            tab,
            BrowserOperation::Click {
                element: "e1".into()
            },
            4
        )
        .is_err()
    );
}
#[test]
fn output_is_typed_and_bounded() {
    let (mut s, _) = fixture();
    let tab = loaded(&mut s);
    let r = s
        .request(7, tab, BrowserOperation::ReadDocument, 0)
        .unwrap();
    s.decide(r, true, 1).unwrap();
    assert!(
        s.event(Event::Output {
            request: r,
            output: Output::Document {
                text: "x".repeat(65 * 1024),
                elements: vec![]
            }
        })
        .is_err()
    );
    assert!(
        Output::Capture {
            token: "../../secret".into(),
            bytes: 1
        }
        .validate(&BrowserOperation::Screenshot { full_page: false })
        .is_err()
    );
    assert!(
        Output::Download {
            token: "download".into(),
            bytes: 33 * 1024 * 1024
        }
        .validate(&BrowserOperation::Download {
            download_id: "d1".into()
        })
        .is_err()
    );
}
#[test]
fn stop_crash_close_and_shutdown_revoke() {
    for mode in 0..4 {
        let (mut s, _) = fixture();
        let tab = loaded(&mut s);
        let r = s
            .request(7, tab, BrowserOperation::ReadDocument, 0)
            .unwrap();
        s.decide(r, true, 1).unwrap();
        match mode {
            0 => s.stop(tab).unwrap(),
            1 => s.event(Event::Crashed { tab }).unwrap(),
            2 => s.close(tab).unwrap(),
            _ => s.shutdown_task(7),
        }
        assert!(
            s.event(Event::Output {
                request: r,
                output: Output::Document {
                    text: "late".into(),
                    elements: vec![]
                }
            })
            .is_err()
        );
    }
}
#[test]
fn history_roundtrip_uses_committed_native_urls() {
    let (mut s, _) = fixture();
    let tab = s.open(BrowserProfile::Manual).unwrap();
    for url in ["http://localhost:3000/a", "http://localhost:3000/b"] {
        s.user_navigate(tab, url, NavigationKind::Push, 0).unwrap();
        let nav = s.tabs[&tab].navigation.as_ref().unwrap().0;
        s.event(Event::Committed {
            tab,
            navigation: nav,
            url: url.into(),
            title: "Local app".into(),
        })
        .unwrap();
    }
    assert!(s.tabs[&tab].view.back);
    s.user_navigate(tab, "", NavigationKind::Back, 0).unwrap();
    let nav = s.tabs[&tab].navigation.as_ref().unwrap().0;
    s.event(Event::Committed {
        tab,
        navigation: nav,
        url: "http://localhost:3000/a".into(),
        title: "Local app".into(),
    })
    .unwrap();
    assert!(s.tabs[&tab].view.forward);
}
#[test]
fn missing_native_host_is_explicit_not_simulated() {
    let mut s = Session::default();
    let tab = s.open(BrowserProfile::Manual).unwrap();
    assert_eq!(s.tabs[&tab].view.state, "unavailable");
    assert_eq!(
        s.user_navigate(tab, "https://example.test/", NavigationKind::Push, 0),
        Err(BrowserError::Unavailable)
    );
    assert!(s.tabs[&tab].view.url.is_none());
}

#[test]
fn stale_native_navigation_title_and_crash_cannot_replace_current_document() {
    let (mut s, _) = fixture();
    let tab = s.open(BrowserProfile::Manual).unwrap();
    s.user_navigate(tab, "https://example.test/", NavigationKind::Push, 0)
        .unwrap();
    let first = s.tabs[&tab].navigation.as_ref().unwrap().0;
    s.event_at(
        Event::Committed {
            tab,
            navigation: first,
            url: "https://example.test/".into(),
            title: "first".into(),
        },
        1,
    )
    .unwrap();
    s.user_navigate(tab, "https://example.test/next", NavigationKind::Push, 2)
        .unwrap();
    let second = s.tabs[&tab].navigation.as_ref().unwrap().0;
    for event in [
        Event::ManualNavigation {
            tab,
            navigation: first,
            url: "https://stale.test/".into(),
        },
        Event::Title {
            tab,
            navigation: first,
            title: "stale".into(),
        },
        Event::DocumentCrashed {
            tab,
            navigation: first,
        },
    ] {
        assert!(s.event_at(event, 3).is_err());
        assert_eq!(s.tabs[&tab].navigation.as_ref().unwrap().0, second);
        assert_eq!(s.tabs[&tab].view.state, "loading");
    }
    s.event_at(
        Event::DocumentCrashed {
            tab,
            navigation: second,
        },
        4,
    )
    .unwrap();
    assert_eq!(s.tabs[&tab].view.state, "crashed");
}
#[test]
fn native_manual_navigation_uses_current_identity_and_never_agent_authority() {
    let (mut s, _) = fixture();
    let tab = s.open(BrowserProfile::Manual).unwrap();
    s.user_navigate(tab, "https://example.test/", NavigationKind::Push, 1)
        .unwrap();
    let navigation = s.tabs[&tab].navigation.as_ref().unwrap().0;
    s.event(Event::Committed {
        tab,
        navigation,
        url: "https://example.test/".into(),
        title: "page".into(),
    })
    .unwrap();
    s.event_at(
        Event::ManualNavigation {
            tab,
            navigation,
            url: "https://example.test/next".into(),
        },
        9000,
    )
    .unwrap();
    assert_eq!(s.tabs[&tab].navigation.as_ref().unwrap().1, 39000);
    let tab = loaded(&mut s);
    let navigation = s.tabs[&tab].committed_navigation.unwrap();
    assert_eq!(
        s.event_at(
            Event::ManualNavigation {
                tab,
                navigation,
                url: "https://example.test/next".into()
            },
            9000
        ),
        Err(BrowserError::WrongContext)
    );
}

#[test]
fn authentication_popup_retains_flow_partition_and_agent_isolation() {
    let (mut s, _) = fixture();
    let profile = BrowserProfile::Authentication { flow: 44 };
    let source = s.open(profile).unwrap();
    s.user_navigate(
        source,
        "https://login.example.test/start",
        NavigationKind::Push,
        0,
    )
    .unwrap();
    let navigation = s.tabs[&source].navigation.as_ref().unwrap().0;
    s.event(Event::Committed {
        tab: source,
        navigation,
        url: "https://login.example.test/start".into(),
        title: "Login".into(),
    })
    .unwrap();
    s.event(Event::PopupRequested {
        tab: source,
        navigation,
        url: "https://login.example.test/oauth?code=secret#callback".into(),
    })
    .unwrap();

    assert_eq!(
        s.popup_preview(source).unwrap().as_deref(),
        Some("https://login.example.test/oauth")
    );
    assert!(s.manual_popup_preview(source).is_err());

    let (popup, url) = s.open_popup(source, 1).unwrap();
    assert_eq!(url, "https://login.example.test/oauth?code=secret#callback");
    assert_eq!(s.tabs[&popup].view.profile, profile);
    assert!(s.popup_preview(source).unwrap().is_none());

    let agent = s.open(BrowserProfile::AgentTask { task: 9 }).unwrap();
    assert!(s.popup_preview(agent).is_err());
    assert!(s.open_popup(agent, 2).is_err());
}
