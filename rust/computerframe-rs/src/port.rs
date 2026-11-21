use serde_json::{Value, json};

use crate::js;

const FRAME_HEADER_FIXED_BYTES: usize = 17;
const FRAME_FLAG_KEYFRAME: u8 = 0b0000_0001;
const FRAME_FLAG_CODEC_CONFIG: u8 = 0b0000_0010;
const COMPUTER_FRAME_MAGIC: u16 = 21315;
const COMPUTER_FRAME_VERSION: u8 = 1;

fn bytes_arg(v: &Value) -> Result<Vec<u8>, String> {
    match v {
        Value::Null => Err("Cannot read properties of null (reading 'byteLength')".to_string()),
        Value::Object(o) => {
            let data = o
                .get("$bytes")
                .or_else(|| o.get("$buffer"))
                .and_then(|x| x.as_array());
            match data {
                Some(a) => Ok(a.iter().map(|x| x.as_u64().unwrap_or(0) as u8).collect()),
                None => {
                    Err("First argument to DataView constructor must be an ArrayBuffer".to_string())
                }
            }
        }
        _ => Err("First argument to DataView constructor must be an ArrayBuffer".to_string()),
    }
}

/// Port of `decodeComputerFrame` (`packages/shared/src/computerFrame.ts:53`).
pub fn decode_computer_frame(bytes: serde_json::Value) -> Result<serde_json::Value, String> {
    let bytes = bytes_arg(&bytes)?;
    if bytes.len() < FRAME_HEADER_FIXED_BYTES {
        return Ok(json!({"ok": false, "reason": "too-short"}));
    }
    let magic = u16::from_le_bytes([bytes[0], bytes[1]]);
    if magic != COMPUTER_FRAME_MAGIC {
        return Ok(json!({"ok": false, "reason": "bad-magic"}));
    }
    if bytes[2] != COMPUTER_FRAME_VERSION {
        return Ok(json!({"ok": false, "reason": "unsupported-version"}));
    }
    let flags = bytes[3];
    let sid_len = bytes[16] as usize;
    let payload_offset = FRAME_HEADER_FIXED_BYTES + sid_len;
    if sid_len == 0 || bytes.len() < payload_offset {
        return Ok(json!({"ok": false, "reason": "truncated-computer-id"}));
    }
    let sid = match std::str::from_utf8(&bytes[FRAME_HEADER_FIXED_BYTES..payload_offset]) {
        Ok(s) => s.to_string(),
        Err(_) => return Ok(json!({"ok": false, "reason": "invalid-computer-id"})),
    };
    let sequence = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let timestamp_ms = f64::from_le_bytes([
        bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    ]);
    let payload: Vec<u64> = bytes[payload_offset..].iter().map(|b| *b as u64).collect();
    Ok(json!({
        "ok": true,
        "frame": {
            "header": {
                "computerId": sid,
                "sequence": sequence,
                "timestampMs": js::number(timestamp_ms),
                "keyframe": flags & FRAME_FLAG_KEYFRAME != 0,
                "codecConfig": flags & FRAME_FLAG_CODEC_CONFIG != 0,
            },
            "payload": { "$bytes": payload },
        },
    }))
}
