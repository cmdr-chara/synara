//! The Rust port of `apps/server/src/accountSsoCallback.ts`.

#![forbid(unsafe_code)]

use serde_json::Value;

const B64URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// `Buffer.toString("base64url")`: RFC 4648 base64url without padding.
fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(B64URL[(n >> 18) as usize & 63] as char);
        out.push(B64URL[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(B64URL[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(B64URL[n as usize & 63] as char);
        }
    }
    out
}

fn b64url_decode(s: &str) -> Option<Vec<u8>> {
    let mut nibbles = Vec::with_capacity(s.len());
    for c in s.bytes() {
        nibbles.push(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32);
    }
    let mut out = Vec::with_capacity(nibbles.len() * 3 / 4);
    for chunk in nibbles.chunks(4) {
        let mut n = 0u32;
        for (i, x) in chunk.iter().enumerate() {
            n |= x << (18 - 6 * i);
        }
        match chunk.len() {
            4 => out.extend([
                ((n >> 16) & 0xff) as u8,
                ((n >> 8) & 0xff) as u8,
                (n & 0xff) as u8,
            ]),
            3 => out.extend([((n >> 16) & 0xff) as u8, ((n >> 8) & 0xff) as u8]),
            2 => out.push(((n >> 16) & 0xff) as u8),
            _ => return None,
        }
    }
    Some(out)
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// `createHash("sha256").update(text).digest()`.
fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());
    for block in msg.as_chunks::<64>().0 {
        let mut w = [0u32; 64];
        for (i, word) in w[..16].iter_mut().enumerate() {
            *word = u32::from_be_bytes(*block[i * 4..][..4].first_chunk::<4>().unwrap_or(&[0; 4]));
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    let mut out = [0u8; 32];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..][..4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

// The corpus was recorded with `crypto.randomBytes` frozen (all 0x07); the env
// var replays the frozen bytes. Production leaves it unset → real entropy.
fn random_bytes(n: usize) -> Vec<u8> {
    if let Ok(s) = std::env::var("SYNARA_SSO_RANDOM")
        && let Some(bytes) = b64url_decode(&s).filter(|b| b.len() >= n)
    {
        return bytes[..n].to_vec();
    }
    let mut bytes = vec![0u8; n];
    let filled = std::fs::File::open("/dev/urandom")
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut bytes))
        .is_ok();
    if !filled {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let mut h = RandomState::new().build_hasher();
        h.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        h.write_u64(std::process::id() as u64);
        for chunk in bytes.chunks_mut(8) {
            h.write_u64(chunk.as_ptr() as u64);
            let v = h.finish().to_le_bytes();
            let len = chunk.len().min(8);
            chunk[..len].copy_from_slice(&v[..len]);
        }
    }
    bytes
}

/// Port of `createPkcePair` (`apps/server/src/accountSsoCallback.ts:32`).
pub fn create_pkce_pair() -> Result<serde_json::Value, String> {
    let code_verifier = base64url(&random_bytes(32));
    let code_challenge = base64url(&sha256(code_verifier.as_bytes()));
    let mut out = serde_json::Map::new();
    out.insert("codeVerifier".into(), Value::String(code_verifier));
    out.insert("codeChallenge".into(), Value::String(code_challenge));
    Ok(Value::Object(out))
}

/// Port of `createSsoState` (`apps/server/src/accountSsoCallback.ts:39`).
pub fn create_sso_state() -> Result<String, String> {
    Ok(base64url(&random_bytes(16)))
}
