//! The Rust port of `apps/server/src/jpegHeader.ts`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

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

fn is_standalone_marker(marker: u8) -> bool {
    marker == 0x01 || (0xd0..=0xd9).contains(&marker)
}

fn is_start_of_frame(marker: u8) -> bool {
    (0xc0..=0xc3).contains(&marker)
        || (0xc5..=0xc7).contains(&marker)
        || (0xc9..=0xcb).contains(&marker)
        || (0xcd..=0xcf).contains(&marker)
}

/// Port of `jpegDimensions` (`apps/server/src/jpegHeader.ts:34`).
pub fn jpeg_dimensions(bytes: Value) -> Result<Option<Value>, String> {
    if bytes.is_null() {
        return Err("Cannot read properties of null (reading 'byteLength')".to_string());
    }
    let Some(bytes) = bytes_of(&bytes) else {
        return Ok(None);
    };
    if bytes.len() < 4 || bytes[0] != 0xff || bytes[1] != 0xd8 {
        return Ok(None);
    }
    let get_u16 = |i: usize| u16::from_be_bytes([bytes[i], bytes[i + 1]]);
    let mut offset = 2usize;
    while offset + 4 <= bytes.len() {
        // Fill bytes before a marker are legal and always 0xff.
        if bytes[offset] != 0xff {
            return Ok(None);
        }
        let mut marker = bytes[offset + 1];
        while marker == 0xff && offset + 2 < bytes.len() {
            offset += 1;
            marker = bytes[offset + 1];
        }
        if is_standalone_marker(marker) {
            offset += 2;
            continue;
        }
        if offset + 4 > bytes.len() {
            return Ok(None);
        }
        let length = get_u16(offset + 2);
        if length < 2 {
            return Ok(None);
        }
        if is_start_of_frame(marker) {
            // Segment payload: precision byte, then height, then width.
            if offset + 9 > bytes.len() {
                return Ok(None);
            }
            let height = get_u16(offset + 5);
            let width = get_u16(offset + 7);
            return Ok(
                (width > 0 && height > 0).then(|| json!({ "width": width, "height": height }))
            );
        }
        // SOS begins entropy-coded data: no more headers to walk.
        if marker == 0xda {
            return Ok(None);
        }
        offset += 2 + length as usize;
    }
    Ok(None)
}
