//! Glue from the front door to `staticassets-rs`: read the request, call `staticassets_rs::port`,
//! answer what the original answers (status, headers that matter, body).

use std::collections::HashMap;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::Response;

/// `route` is the route as wired (`GET /users/{id}`, or `*`), `params` its path parameters.
pub async fn handle(route: &str, params: HashMap<String, String>, req: Request) -> Response {
    let _ = (route, params);

    // A static directory can now be configured (SYNARA_WEB_DIST or the
    // apps/web/dist walk): serve real files through the same pipeline the
    // SPA fallback uses. When nothing is configured the original 503 stays
    // verbatim (app-cases `index-html`, `example-001-get-root`).
    if crate::web_static::configured() {
        return crate::web_static::serve(req.uri().clone(), req.headers().clone()).await;
    }
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(
            "No static directory configured and no dev URL set.",
        ))
        .expect("static 503 response")
}
