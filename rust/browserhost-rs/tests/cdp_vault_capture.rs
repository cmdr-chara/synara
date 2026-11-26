use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Opt-in live smoke test for the end-to-end capture path: real Chrome/CDP,
/// real form submission, BetterWright-compatible sensor, encrypted vault write.
/// Run with `cargo test --test cdp_vault_capture -- --ignored`.
#[tokio::test]
#[ignore]
async fn password_submit_is_captured_after_successful_navigation() {
    if !browserhost_rs::cdp::cdp_available() {
        return;
    }
    let home = std::env::temp_dir().join(format!("synara-cdp-capture-test-{}", std::process::id()));
    std::env::set_var("SYNARA_HOME", &home);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 8192];
                let read = socket.read(&mut buf).await.unwrap_or(0);
                let request = String::from_utf8_lossy(&buf[..read]);
                let (body, content_type) = if request.starts_with("POST /submit") {
                    ("<html><body>signed in</body></html>", "text/html")
                } else {
                    (
                        "<html><body><form id=login method=post action=/submit><input name=username><input name=password type=password><button>Sign in</button></form></body></html>",
                        "text/html",
                    )
                };
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len(),
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    let origin = format!("http://127.0.0.1:{port}/");
    let vault = browserhost_rs::vault::BrowserVault::open_with_keyring(
        home.join("browser-vault"),
        false,
    )
    .expect("open seed vault");
    let mut settings = browserhost_rs::vault::VaultSettings::default();
    settings.offer_save = true;
    vault.configure(settings).expect("enable capture");
    drop(vault);

    let thread = "vault-capture-smoke";
    browserhost_rs::cdp::call_cdp_tool(
        "browser_open",
        &json!({"url": origin, "backend": "obscura"}),
        Some(thread),
    )
    .await
    .expect("open login page");
    browserhost_rs::cdp::call_cdp_tool(
        "browser_fill",
        &json!({"selector": "[name=username]", "value": "ada"}),
        Some(thread),
    )
    .await
    .expect("fill username");
    browserhost_rs::cdp::call_cdp_tool(
        "browser_fill",
        &json!({"selector": "[name=password]", "value": "captured-live-secret"}),
        Some(thread),
    )
    .await
    .expect("fill password");
    browserhost_rs::cdp::call_cdp_tool(
        "browser_click",
        &json!({"selector": "button"}),
        Some(thread),
    )
    .await
    .expect("submit login form");

    let mut captured = false;
    for _ in 0..30 {
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        let inspection = browserhost_rs::vault::BrowserVault::open_with_keyring(
            home.join("browser-vault"),
            false,
        )
        .expect("reopen vault");
        captured = !inspection
            .list_metadata(&origin)
            .expect("list captured metadata")
            .is_empty();
        if captured {
            break;
        }
    }
    assert!(captured, "password submission was not captured");

    let _ = browserhost_rs::cdp::call_cdp_tool("browser_close", &json!({}), Some(thread)).await;
    browserhost_rs::cdp::shutdown_cdp().await;
    let _ = std::fs::remove_dir_all(home);
}
