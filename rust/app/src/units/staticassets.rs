//! Glue from the front door to `staticassets-rs`: read the request, call `staticassets_rs::port`,
//! answer what the original answers (status, headers that matter, body).

use std::collections::HashMap;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::Response;

/// `route` is the route as wired (`GET /users/{id}`, or `*`), `params` its path parameters.
pub async fn handle(route: &str, params: HashMap<String, String>, req: Request) -> Response {
    let _ = (route, params, req);

    // The front door configures no static directory and no dev URL, so the
    // original answers every static request `503 text/plain` (app-cases
    // `index-html`, `example-001-get-root`). The ported helpers in
    // `staticassets_rs::port` — `is_sidecar_request_path`,
    // `negotiate_static_encoding_preference`, `static_etag`,
    // `if_none_match_satisfies`, `static_cache_control` — plug in here once
    // a static directory is configured and files are actually served.
    Response::builder()
        .status(StatusCode::SERVICE_UNAVAILABLE)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(
            "No static directory configured and no dev URL set.",
        ))
        .expect("static 503 response")
}
