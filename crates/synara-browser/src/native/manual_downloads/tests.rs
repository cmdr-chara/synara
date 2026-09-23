use super::*;
use crate::{
    StoragePartition,
    session::{Command, NativePort},
};
use std::{
    io::{Read, Write},
    net::TcpListener,
    time::Instant,
};
use wry::{WebViewBuilderExtUnix, WebViewExtUnix};

#[test]
fn download_publication_never_overwrites_or_follows_a_destination_symlink() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("saved");
    let (target, staging) = prepare_destination(&destination).unwrap();
    let source = staging.path().join("payload");
    fs::write(&source, b"download bytes").unwrap();
    publish(&source, &target).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"download bytes");
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(prepare_destination(&destination).is_err());
    assert!(publish(&source, &destination).is_err());
    let link = root.path().join("link");
    symlink(&destination, &link).unwrap();
    assert!(publish(&source, &link).is_err());
    assert!(prepare_destination(&link).is_err());
    assert!(prepare_destination(Path::new("relative")).is_err());
    assert!(publish(&link, &root.path().join("copy")).is_err());
    let file = fs::OpenOptions::new().write(true).open(&source).unwrap();
    file.set_len(MAX_BYTES + 1).unwrap();
    assert!(publish(&source, &root.path().join("too-large")).is_err());
}

#[test]
#[ignore = "requires real WebKitGTK and a private X11 display"]
fn real_manual_download_has_owned_destination_and_cancellation() {
    gtk::init().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming().take(16) {
            let mut stream = stream.unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut bytes = [0; 4096];
            let len = stream.read(&mut bytes).unwrap_or(0);
            let failed = String::from_utf8_lossy(&bytes[..len]).contains("/error ");
            let status = if failed { "404 Not Found" } else { "200 OK" };
            let body = "owned download fixture";
            let _ = write!(
                stream,
                "HTTP/1.1 {status}\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=fixture.txt\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    let root = tempfile::tempdir().unwrap();
    let (mut port, _receiver, shared) = bridge::channel();
    shared.ready.store(true, Ordering::Release);
    assert!(!port.capabilities().downloads);
    let gate = Rc::new(Gate::default());
    let mut context = wry::WebContext::new(Some(root.path().join("browser")));
    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    let container = gtk::Fixed::new();
    window.add(&container);
    window.set_default_size(800, 600);
    window.show_all();
    let mut views = Vec::new();
    let mut owners = Vec::new();
    for id in 1..=2 {
        let tab = HostTabId(id);
        port.send(Command::Open {
            tab,
            partition: StoragePartition::Manual,
        })
        .unwrap();
        let route = gate.clone();
        let view = wry::WebViewBuilder::new_with_web_context(&mut context)
            .with_download_started_handler(move |url, path| route.destination(&url, path))
            .build_gtk(&container)
            .unwrap();
        let web = view.webview();
        owners.push(Owner::new(&web, shared.clone(), tab, 0, gate.clone()));
        web.load_html(
            "<!doctype html><title>Download fixture</title><h1>Manual download</h1>",
            None,
        );
        views.push(view);
    }
    let pump = || {
        for _ in 0..32 {
            if !gtk::events_pending() {
                break;
            }
            gtk::main_iteration_do(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let until = Instant::now() + Duration::from_secs(30);
    while views.iter().any(|view| {
        view.webview().is_loading()
            || view.webview().title().as_deref() != Some("Download fixture")
            || !view.webview().is_mapped()
    }) {
        pump();
        assert!(Instant::now() < until, "fixture did not load");
    }
    let settle = || {
        let until = Instant::now() + Duration::from_secs(15);
        while gate.active.borrow().is_some() {
            pump();
            assert!(Instant::now() < until, "download did not settle");
        }
    };
    // Both views share a Wry context. Older Wry callbacks must consult the same
    // gate rather than reject or capture the second view's approved destination.
    for (index, owner) in owners.iter().enumerate() {
        let destination = root.path().join(format!("saved-{index}"));
        gate.begin(owner.ui.clone(), &format!("{base}/file"), &destination)
            .unwrap();
        settle();
        assert_eq!(fs::read(&destination).unwrap(), b"owned download fixture");
    }
    let failed = root.path().join("failed");
    gate.begin(owners[0].ui.clone(), &format!("{base}/error"), &failed)
        .unwrap();
    settle();
    assert!(!failed.exists());
    let cancelled = root.path().join("cancelled");
    gate.begin(owners[0].ui.clone(), &format!("{base}/file"), &cancelled)
        .unwrap();
    let transfer = gate.active.borrow().clone().unwrap();
    gate.cancel(&transfer, "Cancelled by test");
    drop(transfer);
    settle();
    assert!(!cancelled.exists());
    let stale = root.path().join("stale");
    gate.begin(owners[1].ui.clone(), &format!("{base}/file"), &stale)
        .unwrap();
    port.send(Command::Stop { tab: HostTabId(2) }).unwrap();
    settle();
    assert!(!stale.exists());
    let mut destination = root.path().join("unauthorized");
    assert!(!gate.destination(&format!("{base}/file"), &mut destination));
    assert!(!destination.exists());
    drop(owners);
    drop(views);
    window.close();
    assert!(fs::read_dir(root.path()).unwrap().all(|item| {
        !item
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".synara-download-")
    }));
    println!(
        "MANUAL_DOWNLOAD_ACCEPTANCE: real bytes, shared-context ownership, HTTP errors, cancel, stale epoch, no overwrite and private cleanup passed"
    );
}
