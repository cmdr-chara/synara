//! The Rust port of `packages/shared/src/providerDeliveryBlock.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

const PREFIX: &str = "Thread is blocked by an earlier provider failure:";

fn is_js_whitespace(c: char) -> bool {
    c.is_whitespace() || c == '\u{FEFF}'
}

/// Port of `formatProviderDeliveryBlockDetail` (`packages/shared/src/providerDeliveryBlock.ts:15`).
pub fn format_provider_delivery_block_detail(blocker_detail: String) -> Result<String, String> {
    Ok(format!("{PREFIX} {blocker_detail}"))
}

/// Port of `isProviderDeliveryBlockDetail` (`packages/shared/src/providerDeliveryBlock.ts:23`).
pub fn is_provider_delivery_block_detail(detail: Option<String>) -> Result<bool, String> {
    Ok(match detail {
        Some(d) => d.trim_start_matches(is_js_whitespace).starts_with(PREFIX),
        None => false,
    })
}
