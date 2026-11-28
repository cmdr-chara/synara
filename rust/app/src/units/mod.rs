//! Static-asset unit routes (percent-encoded paths need explicit registration).
//! During the port these were generated shadow-testing wiring; the upstream
//! fallback is gone, every route serves natively.

use std::collections::HashMap;

use axum::Router;
use axum::extract::{Request, State};
use axum::response::Response;
use axum::routing::get;

use crate::state::AppState;

mod staticassets;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(route_0))
        .route("/api/", get(route_1))
        .route("/api/%C4%B0stanbul", get(route_2))
        .route("/api/%C4%B0stanbul/clients", get(route_3))
        .route("/api/%C4%B0stanbul/pairing-links", get(route_4))
        .route("/api/%C4%B0stanbul/session", get(route_5))
        .route("/api/%C4%B1l%C4%B1k%20%C4%B1rmak", get(route_6))
        .route("/api/%C4%B1l%C4%B1k%20%C4%B1rmak/clients", get(route_7))
        .route(
            "/api/%C4%B1l%C4%B1k%20%C4%B1rmak/pairing-links",
            get(route_8),
        )
        .route("/api/%C4%B1l%C4%B1k%20%C4%B1rmak/session", get(route_9))
        .route("/api/%E6%97%A5%E6%9C%AC%E8%AA%9E", get(route_10))
        .route("/api/%E6%97%A5%E6%9C%AC%E8%AA%9E/clients", get(route_11))
        .route(
            "/api/%E6%97%A5%E6%9C%AC%E8%AA%9E/pairing-links",
            get(route_12),
        )
        .route("/api/%E6%97%A5%E6%9C%AC%E8%AA%9E/session", get(route_13))
        .route("/api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs", get(route_14))
        .route(
            "/api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs/clients",
            get(route_15),
        )
        .route(
            "/api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs/pairing-links",
            get(route_16),
        )
        .route(
            "/api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs/session",
            get(route_17),
        )
        .route(
            "/api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family",
            get(route_18),
        )
        .route(
            "/api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family/clients",
            get(route_19),
        )
        .route(
            "/api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family/pairing-links",
            get(route_20),
        )
        .route(
            "/api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family/session",
            get(route_21),
        )
        .route("/api//clients", get(route_22))
        .route("/api//pairing-links", get(route_23))
        .route("/api//session", get(route_24))
        .route("/api/a%C2%A0b", get(route_25))
        .route("/api/a%C2%A0b/clients", get(route_26))
        .route("/api/a%C2%A0b/pairing-links", get(route_27))
        .route("/api/a%C2%A0b/session", get(route_28))
        .route("/api/does-not-exist", get(route_29))
        .route("/api/e%CC%81", get(route_30))
        .route("/api/e%CC%81/clients", get(route_31))
        .route("/api/e%CC%81/pairing-links", get(route_32))
        .route("/api/e%CC%81/session", get(route_33))
        .route("/api/h%C3%A9llo%20w%C3%B6rld", get(route_34))
        .route("/api/h%C3%A9llo%20w%C3%B6rld/clients", get(route_35))
        .route("/api/h%C3%A9llo%20w%C3%B6rld/pairing-links", get(route_36))
        .route("/api/h%C3%A9llo%20w%C3%B6rld/session", get(route_37))
        .route("/api/stra%C3%9Fe", get(route_38))
        .route("/api/stra%C3%9Fe/clients", get(route_39))
        .route("/api/stra%C3%9Fe/pairing-links", get(route_40))
        .route("/api/stra%C3%9Fe/session", get(route_41))
        .route("/index.html", get(route_42))
        .route("/rust-anything-missing", get(route_43))
}

async fn route_0(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /", HashMap::new(), req).await
}

async fn route_1(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/", HashMap::new(), req).await
}

async fn route_2(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/%C4%B0stanbul", HashMap::new(), req).await
}

async fn route_3(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/%C4%B0stanbul/clients", HashMap::new(), req).await
}

async fn route_4(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/%C4%B0stanbul/pairing-links", HashMap::new(), req).await
}

async fn route_5(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/%C4%B0stanbul/session", HashMap::new(), req).await
}

async fn route_6(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/%C4%B1l%C4%B1k%20%C4%B1rmak", HashMap::new(), req).await
}

async fn route_7(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%C4%B1l%C4%B1k%20%C4%B1rmak/clients",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_8(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%C4%B1l%C4%B1k%20%C4%B1rmak/pairing-links",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_9(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%C4%B1l%C4%B1k%20%C4%B1rmak/session",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_10(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/%E6%97%A5%E6%9C%AC%E8%AA%9E", HashMap::new(), req).await
}

async fn route_11(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%E6%97%A5%E6%9C%AC%E8%AA%9E/clients",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_12(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%E6%97%A5%E6%9C%AC%E8%AA%9E/pairing-links",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_13(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%E6%97%A5%E6%9C%AC%E8%AA%9E/session",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_14(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_15(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs/clients",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_16(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs/pairing-links",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_17(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%8D%F0%9F%8F%BD%20thumbs/session",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_18(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_19(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family/clients",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_20(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family/pairing-links",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_21(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/%F0%9F%91%A8%E2%80%8D%F0%9F%91%A9%E2%80%8D%F0%9F%91%A7%20family/session",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_22(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api//clients", HashMap::new(), req).await
}

async fn route_23(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api//pairing-links", HashMap::new(), req).await
}

async fn route_24(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api//session", HashMap::new(), req).await
}

async fn route_25(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/a%C2%A0b", HashMap::new(), req).await
}

async fn route_26(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/a%C2%A0b/clients", HashMap::new(), req).await
}

async fn route_27(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/a%C2%A0b/pairing-links", HashMap::new(), req).await
}

async fn route_28(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/a%C2%A0b/session", HashMap::new(), req).await
}

async fn route_29(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/does-not-exist", HashMap::new(), req).await
}

async fn route_30(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/e%CC%81", HashMap::new(), req).await
}

async fn route_31(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/e%CC%81/clients", HashMap::new(), req).await
}

async fn route_32(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/e%CC%81/pairing-links", HashMap::new(), req).await
}

async fn route_33(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/e%CC%81/session", HashMap::new(), req).await
}

async fn route_34(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/h%C3%A9llo%20w%C3%B6rld", HashMap::new(), req).await
}

async fn route_35(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/h%C3%A9llo%20w%C3%B6rld/clients",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_36(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/h%C3%A9llo%20w%C3%B6rld/pairing-links",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_37(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle(
        "GET /api/h%C3%A9llo%20w%C3%B6rld/session",
        HashMap::new(),
        req,
    )
    .await
}

async fn route_38(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/stra%C3%9Fe", HashMap::new(), req).await
}

async fn route_39(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/stra%C3%9Fe/clients", HashMap::new(), req).await
}

async fn route_40(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/stra%C3%9Fe/pairing-links", HashMap::new(), req).await
}

async fn route_41(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /api/stra%C3%9Fe/session", HashMap::new(), req).await
}

async fn route_42(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /index.html", HashMap::new(), req).await
}

async fn route_43(State(_state): State<AppState>, req: Request) -> Response {
    staticassets::handle("GET /rust-anything-missing", HashMap::new(), req).await
}
