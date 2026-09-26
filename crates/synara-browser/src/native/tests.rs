use super::*;
use crate::session::{RequestState, Session};
use wry::WebViewBuilderExtUnix;

#[test]
fn native_origin_policy_fences_scheme_host_and_port() {
    let document = CommittedDocument::parse("https://example.test:8443/page").unwrap();
    for url in [
        "http://example.test:8443/",
        "https://example.test/",
        "https://exampleXtest:8443/",
        "https://other.test:8443/",
    ] {
        assert!(!approved_origin_allows(
            Some(&document.origin),
            &CommittedDocument::parse(url).unwrap()
        ));
    }
    assert!(approved_origin_allows(
        Some(&document.origin),
        &CommittedDocument::parse("https://example.test:8443/next").unwrap()
    ));
}

#[test]
fn cancellation_reaches_queued_actions_without_a_ui_pump() {
    let (mut port, receiver, shared) = bridge::channel();
    let tab = HostTabId(1);
    let id = HostRequestId(1);
    port.send(Command::Open {
        tab,
        partition: StoragePartition::AgentTask(1),
    })
    .unwrap();
    let _ = receiver.recv().unwrap();
    port.send(Command::Operation {
        request: id,
        command: NativeCommand {
            tab,
            partition: StoragePartition::AgentTask(1),
            operation: BrowserOperation::ReadDocument,
        },
        max_output_bytes: 1024,
    })
    .unwrap();
    port.send(Command::Cancel { request: id }).unwrap();
    let delivery = receiver.recv().unwrap();
    assert!(delivery.cancelled.load(Ordering::Acquire));
    port.send(Command::Stop { tab }).unwrap();
    assert_ne!(shared.epoch(tab), Some(delivery.epoch));
}

/// Real WebKitGTK, not a mock. Run on a private Xvfb display, one test thread.
#[test]
#[ignore = "requires WebKitGTK 4.1 and an isolated X11 display"]
fn real_webkit_navigation_consent_input_redirect_and_isolation() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::Mutex,
    };
    gtk::init().unwrap();
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let cross = TcpListener::bind("127.0.0.1:0").unwrap();
    cross.set_nonblocking(true).unwrap();
    let forbidden = format!("http://{}/forbidden", cross.local_addr().unwrap());
    let log = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(100) {
            let mut stream = stream.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut data = [0; 8192];
            let len = stream.read(&mut data).unwrap_or(0);
            let request = String::from_utf8_lossy(&data[..len]).into_owned();
            log.lock().unwrap().push(request.clone());
            let body = "<!doctype html><title>Native browser fixture</title><style>body{min-height:2400px}</style><h1>REAL WEBKIT PAGE</h1><input aria-label='Name'><button onclick=\"document.querySelector('h1').textContent='Clicked '+document.querySelector('input').value\">Apply</button><script>window.__synaraRefs='page-forgery';</script>";
            let response = if request.starts_with("GET /redirect ") {
                format!(
                    "HTTP/1.1 302 Found\r\nLocation: {forbidden}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                )
            } else {
                format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nSet-Cookie: fixture=manual; Path=/\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
            };
            let _ = stream.write_all(response.as_bytes());
        }
    });
    let root = tempfile::tempdir().unwrap();
    let (mut host, port) = NativeHost::new(root.path().to_path_buf());
    host.initialized = true;
    host.shared.ready.store(true, Ordering::Release);
    let mut session = Session::new(port);
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_default_size(800, 600);
    let container = gtk::Fixed::new();
    window.add(&container);
    window.show_all();
    let clock = Instant::now();
    let now = || clock.elapsed().as_millis() as u64;
    let pump = |host: &mut NativeHost, session: &mut Session| {
        host.reap();
        while let Ok(delivery) = host.commands.try_recv() {
            host.dispatch(delivery, |b| b.build_gtk(&container));
        }
        for _ in 0..32 {
            if !gtk::events_pending() {
                break;
            }
            gtk::main_iteration_do(false);
        }
        for event in host.drain_events() {
            let _ = session.event_at(event, now());
        }
        session.tick(now());
    };
    let wait = |host: &mut NativeHost, session: &mut Session, id: HostRequestId| {
        println!("Awaiting native request {id:?}");
        let end = Instant::now() + Duration::from_secs(20);
        loop {
            pump(host, session);
            match session.result(7, id).unwrap().state {
                RequestState::Complete(output) => break output,
                RequestState::Failed(error) => panic!("native operation failed: {error}"),
                _ => assert!(Instant::now() < end, "native operation timed out"),
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    };
    let manual = session.open(BrowserProfile::Manual).unwrap();
    session
        .user_navigate(
            manual,
            &format!("{base}/manual"),
            NavigationKind::Push,
            now(),
        )
        .unwrap();
    for _ in 0..1000 {
        pump(&mut host, &mut session);
        if session
            .tabs()
            .iter()
            .any(|t| t.id == manual && t.state == "ready")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        session
            .tabs()
            .iter()
            .any(|t| t.id == manual && t.state == "ready")
    );
    // Establish that the manual cookie was accepted before testing isolation.
    session
        .user_navigate(
            manual,
            &format!("{base}/manual-again"),
            NavigationKind::Push,
            now(),
        )
        .unwrap();
    for _ in 0..1000 {
        pump(&mut host, &mut session);
        if session
            .tabs()
            .iter()
            .any(|t| t.id == manual && t.state == "ready")
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.starts_with("GET /manual-again ")
                && r.to_lowercase().contains("cookie: fixture=manual"))
    );
    let tab = session.open(BrowserProfile::AgentTask { task: 7 }).unwrap();
    host.viewport(Some(tab), Some(ViewportRect::logical(0., 0., 800., 600.)));
    let nav = session
        .request(
            7,
            tab,
            BrowserOperation::Navigate {
                url: format!("{base}/agent"),
            },
            now(),
        )
        .unwrap();
    pump(&mut host, &mut session);
    assert!(
        !requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.starts_with("GET /agent "))
    );
    session.decide(nav, true, now()).unwrap();
    wait(&mut host, &mut session, nav);
    assert!(host.visible_bounds().is_some());
    let blank = session.open(BrowserProfile::Manual).unwrap();
    let bounds = ViewportRect::logical(0., 0., 800., 600.);
    host.viewport(Some(blank), Some(bounds));
    assert!(
        host.visible_bounds().is_none(),
        "blank selection has no native surface"
    );
    assert!(!host.views[&tab].webview.webview().is_visible());
    host.viewport(Some(tab), Some(bounds));
    assert!(host.visible_bounds().is_some());
    assert!(host.views[&tab].webview.webview().is_visible());
    host.viewport(Some(tab), None);
    assert!(
        host.visible_bounds().is_none(),
        "overlays hide existing content"
    );
    host.viewport(Some(tab), Some(bounds));
    session.close(blank).unwrap();
    let request = requests
        .lock()
        .unwrap()
        .iter()
        .find(|r| r.starts_with("GET /agent "))
        .unwrap()
        .clone();
    assert!(
        !request.to_lowercase().contains("cookie:"),
        "agent inherited manual cookies"
    );
    let read = session
        .request(7, tab, BrowserOperation::ReadDocument, now())
        .unwrap();
    session.decide(read, true, now()).unwrap();
    let Output::Document { text, elements } = wait(&mut host, &mut session, read) else {
        panic!("expected document");
    };
    assert!(text.contains("REAL WEBKIT PAGE"));
    let input = elements
        .iter()
        .find(|e| e.name == "Name")
        .unwrap()
        .id
        .clone();
    let button = elements
        .iter()
        .find(|e| e.name == "Apply")
        .unwrap()
        .id
        .clone();
    // Main-world code must not replace the inventory held in the private world.
    let tampered = Rc::new(Cell::new(false));
    let complete = tampered.clone();
    host.views[&tab].webview.webview().evaluate_javascript(
        "globalThis.__synaraRefs = 'page-forgery-after-read';",
        None,
        None,
        None::<&gio::Cancellable>,
        move |r| {
            assert!(r.is_ok());
            complete.set(true);
        },
    );
    let end = Instant::now() + Duration::from_secs(5);
    while !tampered.get() {
        pump(&mut host, &mut session);
        assert!(Instant::now() < end);
        std::thread::sleep(Duration::from_millis(10));
    }
    let fill = session
        .request(
            7,
            tab,
            BrowserOperation::Fill {
                element: input,
                text: "Synara".into(),
            },
            now(),
        )
        .unwrap();
    session.decide(fill, true, now()).unwrap();
    wait(&mut host, &mut session, fill);
    let click = session
        .request(7, tab, BrowserOperation::Click { element: button }, now())
        .unwrap();
    session.decide(click, true, now()).unwrap();
    wait(&mut host, &mut session, click);
    let read = session
        .request(7, tab, BrowserOperation::ReadDocument, now())
        .unwrap();
    session.decide(read, true, now()).unwrap();
    let Output::Document { text, .. } = wait(&mut host, &mut session, read) else {
        panic!()
    };
    assert!(
        text.contains("Clicked Synara"),
        "real DOM action was not applied: {text}"
    );
    // The existing one-shot approval path owns scrolling too. Reading is
    // required again before an old element can be used after a viewport change.
    let read = session
        .request(7, tab, BrowserOperation::ReadDocument, now())
        .unwrap();
    session.decide(read, true, now()).unwrap();
    let Output::Document { elements, .. } = wait(&mut host, &mut session, read) else {
        panic!()
    };
    let old_button = elements
        .iter()
        .find(|e| e.name == "Apply")
        .unwrap()
        .id
        .clone();
    assert!(
        session
            .request(
                7,
                tab,
                BrowserOperation::Input {
                    event: InputEvent::Scroll { x: 0, y: 4097 },
                },
                now()
            )
            .is_err()
    );
    let scroll = session
        .request(
            7,
            tab,
            BrowserOperation::Input {
                event: InputEvent::Scroll { x: 0, y: 300 },
            },
            now(),
        )
        .unwrap();
    pump(&mut host, &mut session);
    assert!(matches!(
        session.result(7, scroll).unwrap().state,
        RequestState::AwaitingConsent
    ));
    session.decide(scroll, true, now()).unwrap();
    assert!(matches!(
        wait(&mut host, &mut session, scroll),
        Output::Done
    ));
    assert!(
        session
            .request(
                7,
                tab,
                BrowserOperation::Click {
                    element: old_button
                },
                now()
            )
            .is_err()
    );
    let position = Rc::new(Cell::new(None));
    let observed = position.clone();
    host.views[&tab].webview.webview().evaluate_javascript(
        "window.scrollY",
        None,
        None,
        None::<&gio::Cancellable>,
        move |result| {
            observed.set(Some(result.unwrap().to_double()));
        },
    );
    let end = Instant::now() + Duration::from_secs(5);
    while position.get().is_none() {
        pump(&mut host, &mut session);
        assert!(Instant::now() < end, "scroll observation timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        position.get().unwrap() >= 299.,
        "real WebKit viewport did not scroll"
    );
    let redirect = session
        .request(
            7,
            tab,
            BrowserOperation::Navigate {
                url: format!("{base}/redirect"),
            },
            now(),
        )
        .unwrap();
    session.decide(redirect, true, now()).unwrap();
    let end = Instant::now() + Duration::from_secs(8);
    while Instant::now() < end {
        pump(&mut host, &mut session);
        assert!(
            cross.accept().is_err(),
            "cross-origin redirect reached the target server"
        );
        if matches!(
            session.result(7, redirect).unwrap().state,
            RequestState::Failed(_)
        ) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        requests
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.starts_with("GET /redirect "))
    );
    assert!(
        matches!(
            session.result(7, redirect).unwrap().state,
            RequestState::Failed(_)
        ),
        "redirect did not fail closed"
    );
    session.shutdown_task(7);
    pump(&mut host, &mut session);
    assert!(host.views.keys().all(|id| *id != tab));
    assert!(host.profiles.keys().all(|key| key != "task-7"));
    println!(
        "REAL_WEBKIT_ACCEPTANCE: manual load, isolated cookies, consent, document, fill, click, approved scroll, stale references, redirect fence, teardown passed"
    );
}

#[test]
fn cancelled_queued_requests_release_capacity() {
    let root = tempfile::tempdir().unwrap();
    let (mut host, mut port) = NativeHost::new(root.path().to_path_buf());
    let tab = HostTabId(1);
    port.send(Command::Open {
        tab,
        partition: StoragePartition::AgentTask(1),
    })
    .unwrap();
    host.dispatch(host.commands.recv().unwrap(), |_| {
        panic!("Open must not create a view")
    });
    for i in 1..300 {
        let request = HostRequestId(i);
        port.send(Command::Operation {
            request,
            command: NativeCommand {
                tab,
                partition: StoragePartition::AgentTask(1),
                operation: BrowserOperation::ReadDocument,
            },
            max_output_bytes: 1024,
        })
        .unwrap();
        port.send(Command::Cancel { request }).unwrap();
        host.dispatch(host.commands.recv().unwrap(), |_| {
            panic!("Cancelled operation must not create a view")
        });
    }
}
#[test]
fn failed_open_does_not_leak_tab_capacity() {
    let (mut port, receiver, shared) = bridge::channel();
    let first = HostTabId(1);
    port.send(Command::Open {
        tab: first,
        partition: StoragePartition::Manual,
    })
    .unwrap();
    for _ in 1..128 {
        port.send(Command::Stop { tab: first }).unwrap();
    }
    let refused = HostTabId(2);
    assert_eq!(
        port.send(Command::Open {
            tab: refused,
            partition: StoragePartition::Manual
        }),
        Err(BrowserError::Limit)
    );
    assert_eq!(shared.epoch(refused), None);
    drop(receiver);
}
#[test]
fn browser_storage_rejects_files_and_symlinks() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let root = tempfile::tempdir().unwrap();
    let real = root.path().join("real");
    private_directory(&real).unwrap();
    assert_eq!(
        std::fs::metadata(&real).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let link = root.path().join("manual");
    symlink(&real, &link).unwrap();
    assert!(private_directory(&link).is_err());
    let file = root.path().join("file");
    std::fs::write(&file, "untouched").unwrap();
    assert!(private_directory(&file).is_err());
    assert_eq!(std::fs::read_to_string(file).unwrap(), "untouched");
}

#[test]
fn content_allocation_does_not_repeat_the_outer_pane_offset() {
    let bounds = content_bounds(ViewportRect::logical(268., 186., 1140., 659.));
    assert_eq!(
        bounds.position.to_logical::<i32>(1.),
        wry::dpi::LogicalPosition::new(0, 0)
    );
    assert_eq!(
        bounds.size.to_logical::<i32>(1.),
        wry::dpi::LogicalSize::new(1140, 659)
    );
}

#[test]
fn a_selected_tab_row_without_a_view_never_maps_the_native_surface() {
    let root = tempfile::tempdir().unwrap();
    let (mut host, mut port) = NativeHost::new(root.path().to_path_buf());
    host.shared.ready.store(true, Ordering::Release);
    let tab = HostTabId(1);
    port.send(Command::Open {
        tab,
        partition: StoragePartition::Manual,
    })
    .unwrap();
    host.dispatch(host.commands.recv().unwrap(), |_| {
        panic!("Open must not create a view")
    });
    host.viewport(Some(tab), Some(ViewportRect::logical(0., 0., 800., 600.)));
    assert!(host.tabs.contains_key(&tab));
    assert!(host.visible_bounds().is_none());
    port.send(Command::Close { tab }).unwrap();
    host.reap();
    assert!(!host.tabs.contains_key(&tab));
    assert!(host.visible_bounds().is_none());
}


/// Real WebKitGTK authentication flow with cookie-backed session state and a
/// denied-by-default popup that the trusted host reopens in the same partition.
#[test]
#[ignore = "requires WebKitGTK 4.1 and an isolated X11 display"]
fn real_webkit_authentication_login_session_and_popup() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::Mutex,
    };

    gtk::init().unwrap();
    let requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let log = requests.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming().take(32) {
            let mut stream = stream.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut data = [0; 8192];
            let len = stream.read(&mut data).unwrap_or(0);
            let request = String::from_utf8_lossy(&data[..len]).into_owned();
            log.lock().unwrap().push(request.clone());
            let request_line = request.lines().next().unwrap_or_default();
            let (headers, body) = if request_line.starts_with("GET /login ") {
                (
                    "Set-Cookie: synara_auth=accepted; Path=/; HttpOnly\r\n",
                    "<!doctype html><title>Login</title><h1>Authenticated</h1><script>setTimeout(()=>window.open('/oauth?code=secret','oauth'),250)</script>",
                )
            } else if request_line.starts_with("GET /oauth?code=secret ") {
                (
                    "Set-Cookie: synara_oauth=complete; Path=/; HttpOnly\r\n",
                    "<!doctype html><title>OAuth</title><h1>OAuth complete</h1>",
                )
            } else {
                (
                    "",
                    "<!doctype html><title>Session</title><h1>Session check</h1>",
                )
            };
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
        }
    });

    let root = tempfile::tempdir().unwrap();
    let (mut host, port) = NativeHost::new(root.path().to_path_buf());
    host.initialized = true;
    host.shared.ready.store(true, Ordering::Release);
    let mut session = Session::new(port);
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_default_size(800, 600);
    let container = gtk::Fixed::new();
    window.add(&container);
    window.show_all();
    let clock = Instant::now();
    let now = || clock.elapsed().as_millis() as u64;
    let pump = |host: &mut NativeHost, session: &mut Session| {
        host.reap();
        while let Ok(delivery) = host.commands.try_recv() {
            host.dispatch(delivery, |builder| builder.build_gtk(&container));
        }
        for _ in 0..32 {
            if !gtk::events_pending() {
                break;
            }
            gtk::main_iteration_do(false);
        }
        for event in host.drain_events() {
            let _ = session.event_at(event, now());
        }
        session.tick(now());
    };
    let wait_ready = |host: &mut NativeHost, session: &mut Session, tab: HostTabId| {
        let end = Instant::now() + Duration::from_secs(20);
        loop {
            pump(host, session);
            if session
                .tabs()
                .iter()
                .any(|view| view.id == tab && view.state == "ready")
            {
                break;
            }
            assert!(Instant::now() < end, "authentication page timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
    };

    let profile = BrowserProfile::Authentication { flow: 77 };
    let source = session.open(profile).unwrap();
    session
        .user_navigate(
            source,
            &format!("{base}/login"),
            NavigationKind::Push,
            now(),
        )
        .unwrap();
    wait_ready(&mut host, &mut session, source);

    let popup_deadline = Instant::now() + Duration::from_secs(20);
    loop {
        pump(&mut host, &mut session);
        if session.popup_preview(source).unwrap().is_some() {
            break;
        }
        assert!(
            Instant::now() < popup_deadline,
            "authentication popup was not captured"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        session.popup_preview(source).unwrap().as_deref(),
        Some(format!("{base}/oauth").as_str())
    );

    let (popup, popup_url) = session.open_popup(source, now()).unwrap();
    assert_eq!(popup_url, format!("{base}/oauth?code=secret"));
    assert_eq!(
        session.tabs().iter().find(|view| view.id == popup).unwrap().profile,
        profile
    );
    wait_ready(&mut host, &mut session, popup);

    session
        .user_navigate(
            source,
            &format!("{base}/session"),
            NavigationKind::Push,
            now(),
        )
        .unwrap();
    wait_ready(&mut host, &mut session, source);

    let manual = session.open(BrowserProfile::Manual).unwrap();
    session
        .user_navigate(
            manual,
            &format!("{base}/session"),
            NavigationKind::Push,
            now(),
        )
        .unwrap();
    wait_ready(&mut host, &mut session, manual);

    let requests = requests.lock().unwrap();
    let oauth = requests
        .iter()
        .find(|request| request.starts_with("GET /oauth?code=secret "))
        .expect("OAuth popup request");
    assert!(oauth.contains("synara_auth=accepted"));
    let sessions: Vec<_> = requests
        .iter()
        .filter(|request| request.starts_with("GET /session "))
        .collect();
    assert!(sessions.len() >= 2);
    assert!(sessions[0].contains("synara_auth=accepted"));
    assert!(sessions[0].contains("synara_oauth=complete"));
    assert!(!sessions.last().unwrap().contains("synara_auth=accepted"));
    assert!(!sessions.last().unwrap().contains("synara_oauth=complete"));

    println!(
        "AUTH_WEBKIT_ACCEPTANCE: login cookie, explicit popup, shared auth session, manual isolation"
    );
}
