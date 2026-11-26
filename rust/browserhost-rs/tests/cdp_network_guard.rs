use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Opt-in live smoke test: launches the real local Chrome through CDP and
/// proves Fetch-intercepted pages still load when the guard allows them.
/// Run with `cargo test --test cdp_network_guard -- --ignored`.
#[tokio::test]
#[ignore]
async fn allowed_loopback_page_loads_through_fetch_guard() {
    if !browserhost_rs::cdp::cdp_available() {
        return;
    }
    let home = std::env::temp_dir().join(format!("synara-cdp-guard-test-{}", std::process::id()));
    std::env::set_var("SYNARA_HOME", &home);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = socket.read(&mut buf).await;
                let body = "guard-smoke-ok";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    let thread = "network-guard-smoke";
    let opened = browserhost_rs::cdp::call_cdp_tool(
        "browser_open",
        &json!({"url": format!("http://127.0.0.1:{port}/"), "backend": "obscura"}),
        Some(thread),
    )
    .await
    .expect("loopback navigation should pass the guard");
    assert_eq!(
        opened.get("backend").and_then(serde_json::Value::as_str),
        Some("obscura")
    );

    let evaluated = browserhost_rs::cdp::call_cdp_tool(
        "browser_evaluate",
        &json!({"expression": "document.body.innerText"}),
        Some(thread),
    )
    .await
    .expect("evaluation after guarded navigation");
    assert_eq!(
        evaluated.get("value").and_then(serde_json::Value::as_str),
        Some("guard-smoke-ok")
    );

    let blocked = browserhost_rs::cdp::call_cdp_tool(
        "browser_navigate",
        &json!({"url": "http://169.254.169.254/latest/meta-data"}),
        Some(thread),
    )
    .await;
    assert!(blocked
        .expect_err("cloud metadata navigation must be blocked")
        .contains("BrowserNavigationBlocked"));

    let _ = browserhost_rs::cdp::call_cdp_tool("browser_close", &json!({}), Some(thread)).await;
    browserhost_rs::cdp::shutdown_cdp().await;
    let _ = std::fs::remove_dir_all(home);
}
