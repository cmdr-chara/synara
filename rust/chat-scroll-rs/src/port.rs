//! The Rust port of `apps/web/src/chat-scroll.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

use crate::js;

const AUTO_SCROLL_BOTTOM_THRESHOLD_PX: f64 = 64.0;

fn distance_from_bottom(position: &Value) -> Result<f64, String> {
    if position.is_null() {
        return Err(
            "Cannot destructure property 'scrollTop' of 'position' as it is null.".to_string(),
        );
    }
    if js::is_undefined(position) {
        return Err(
            "Cannot destructure property 'scrollTop' of 'position' as it is undefined.".to_string(),
        );
    }
    let scroll_top = position.get("scrollTop").and_then(js::as_number);
    let client_height = position.get("clientHeight").and_then(js::as_number);
    let scroll_height = position.get("scrollHeight").and_then(js::as_number);
    match (scroll_top, client_height, scroll_height) {
        (Some(t), Some(c), Some(h)) if t.is_finite() && c.is_finite() && h.is_finite() => {
            Ok((h - c - t).max(0.0))
        }
        _ => Ok(0.0),
    }
}

/// Port of `getScrollContainerDistanceFromBottom` (`apps/web/src/chat-scroll.ts:9`).
pub fn get_scroll_container_distance_from_bottom(
    position: serde_json::Value,
) -> Result<f64, String> {
    distance_from_bottom(&position)
}

/// Port of `isScrollContainerNearBottom` (`apps/web/src/chat-scroll.ts:18`).
pub fn is_scroll_container_near_bottom(
    position: serde_json::Value,
    threshold_px: Option<serde_json::Value>,
) -> Result<bool, String> {
    let raw = threshold_px.as_ref().and_then(js::as_number);
    let threshold = match raw {
        Some(t) if t.is_finite() => t.max(0.0),
        _ => AUTO_SCROLL_BOTTOM_THRESHOLD_PX,
    };
    Ok(distance_from_bottom(&position)? <= threshold)
}
