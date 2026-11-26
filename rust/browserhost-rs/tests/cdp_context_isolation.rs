use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Opt-in live smoke test: separate automation threads get separate incognito
/// browser contexts even though they share the same Chrome process.
/// Run with `cargo test --test cdp_context_isolation -- --ignored`.
#[tokio::test]
#[ignore]
async fn cookies_do_not_leak_between_thread_contexts() {
    if !browserhost_rs::cdp::cdp_available() {
        return;
    }
    let home = std::env::temp_dir().join(format!("synara-cdp-context-test-{}", std::process::id()));
    std::env::set_var("SYNARA_HOME", &home);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                let _ = socket.read(&mut buf).await;
                let body = "<html><body>isolation</body></html>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });

    let url = format!("http://127.0.0.1:{port}/");
    let first = "context-isolation-first";
    let second = "context-isolation-second";
    browserhost_rs::cdp::call_cdp_tool(
        "browser_open",
        &json!({"url": url, "backend": "obscura"}),
        Some(first),
    )
    .await
    .expect("first navigation");
    browserhost_rs::cdp::call_cdp_tool(
        "browser_evaluate",
        &json!({"expression": "document.cookie='synara-isolation=first; path=/'; document.cookie"}),
        Some(first),
    )
    .await
    .expect("set first cookie");
    browserhost_rs::cdp::call_cdp_tool(
        "browser_open",
        &json!({"url": format!("http://127.0.0.1:{port}/"), "backend": "obscura"}),
        Some(second),
    )
    .await
    .expect("second navigation");
    let second_cookies = browserhost_rs::cdp::call_cdp_tool(
        "browser_evaluate",
        &json!({"expression": "document.cookie"}),
        Some(second),
    )
    .await
    .expect("read second cookies");
    assert_eq!(
        second_cookies.get("value").and_then(serde_json::Value::as_str),
        Some("")
    );

    for thread in [first, second] {
        let tabs = browserhost_rs::cdp::call_cdp_tool("browser_tabs", &json!({}), Some(thread))
            .await
            .expect("thread tabs");
        assert_eq!(
            tabs.get("tabs").and_then(serde_json::Value::as_array).map(Vec::len),
            Some(1)
        );
        let _ = browserhost_rs::cdp::call_cdp_tool("browser_close", &json!({}), Some(thread)).await;
    }
    browserhost_rs::cdp::shutdown_cdp().await;
    let _ = std::fs::remove_dir_all(home);
}
