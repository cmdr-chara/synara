mod git_domain;
mod native_ws;
mod runtime_status;
mod server_env;
mod shell_editor;
mod todo;
mod ws_relay;
mod original;
mod units;

use std::net::SocketAddr;

use axum::serve::ListenerExt;
use original::shutdown_signal;

#[tokio::main]
async fn main() {
    original::report_fallbacks();
    runtime_status::start_sampler();
    let original = original::Original::start().await;
    let app = units::router()
        .merge(native_ws::router())
        .merge(ws_relay::router())
        .with_state(original.clone());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("binding the port");
    let listener = listener.tap_io(|tcp| {
        let _ = tcp.set_nodelay(true);
    });
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("serving");
    original.stop();
}
