//! The Rust port of `apps/server/src/pngHeader.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const PNG_IHDR: [u8; 4] = [0x49, 0x48, 0x44, 0x52];
const PNG_HEADER_BYTES: usize = 24;

/// Bytes only where JS indexed access could yield numbers (`Uint8Array`,
/// `$buffer`, or an array of integers). Anything else fails the signature
/// check in JS, so `None` maps straight to a `null` return.
fn bytes_of(v: &Value) -> Option<Vec<u8>> {
    match v {
        Value::Array(a) => {
            let mut out = Vec::with_capacity(a.len());
            for item in a {
                let n = item.as_f64()?;
                if n.fract() != 0.0 || !(0.0..=255.0).contains(&n) {
                    return None;
                }
                out.push(n as u8);
            }
            Some(out)
        }
        Value::Object(o) => {
            let key = ["$bytes", "$buffer"].iter().find(|k| o.contains_key(**k))?;
            bytes_of(o.get(*key)?)
        }
        _ => None,
    }
}

/// Port of `pngDimensions` (`apps/server/src/pngHeader.ts:28`).
pub fn png_dimensions(bytes: Value) -> Result<Option<Value>, String> {
    if bytes.is_null() {
        return Err("Cannot read properties of null (reading 'byteLength')".to_string());
    }
    let Some(bytes) = bytes_of(&bytes) else {
        return Ok(None);
    };
    if bytes.len() < PNG_HEADER_BYTES || bytes[..8] != PNG_SIGNATURE || bytes[12..16] != PNG_IHDR {
        return Ok(None);
    }
    let width = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let height = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if width == 0 || height == 0 {
        return Ok(None);
    }
    Ok(Some(json!({ "width": width, "height": height })))
}
