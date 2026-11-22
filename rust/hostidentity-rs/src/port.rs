//! The Rust port of `apps/server/src/hostIdentity/`.
//!
//! Volatile values replay via env (ADR-004): `SYNARA_HOST_IDENTITY_JSON` returns a
//! fixed identity document, `SYNARA_STATE_PID` pins the pid inside atomic temp
//! names, `SYNARA_STATE_UUID` pins `randomUUID`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};

fn env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|s| !s.is_empty())
}

fn undef(v: &Value) -> bool {
    v.is_null() || v.get("$undefined").is_some_and(|u| u == true)
}

fn prop<'a>(v: &'a Value, name: &str) -> &'a Value {
    v.get(name).unwrap_or(&Value::Null)
}

fn js_type_err(who: &str, what: &str) -> String {
    format!("Cannot read properties of {who} (reading '{what}')")
}

// ---------- posix path helpers ----------

fn posix_join(a: &str, b: &str) -> String {
    let mut out = String::from(a);
    if !b.is_empty() {
        if out.is_empty() {
            out = b.to_string();
        } else {
            if !out.ends_with('/') {
                out.push('/');
            }
            out.push_str(b);
        }
    }
    posix_normalize(&out)
}

fn posix_normalize(p: &str) -> String {
    let abs = p.starts_with('/');
    let mut segs: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if segs.last() == Some(&"..") || segs.is_empty() {
                    if !abs {
                        segs.push("..");
                    }
                } else {
                    segs.pop();
                }
            }
            s => segs.push(s),
        }
    }
    let joined = segs.join("/");
    if abs {
        format!("/{joined}")
    } else if joined.is_empty() {
        ".".to_string()
    } else {
        joined
    }
}

fn posix_dirname(p: &str) -> String {
    let trimmed = p.trim_end_matches('/');
    match trimmed.rfind('/') {
        None => ".".to_string(),
        Some(0) => "/".to_string(),
        Some(i) => trimmed[..i].to_string(),
    }
}

// ---------- base64 / PEM ----------

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk.len();
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let v = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(v >> 18) as usize & 63] as char);
        out.push(B64[(v >> 12) as usize & 63] as char);
        out.push(if n > 1 {
            B64[(v >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if n > 2 {
            B64[v as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let mut table = [255u8; 128];
    for (i, &c) in B64.iter().enumerate() {
        table[c as usize] = i as u8;
    }
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut nbits = 0u32;
    for c in s.bytes() {
        if c == b'=' || c == b'\n' || c == b'\r' {
            continue;
        }
        let v = *table.get(c as usize)?;
        if v == 255 {
            return None;
        }
        acc = (acc << 6) | v as u32;
        nbits += 6;
        if nbits >= 8 {
            nbits -= 8;
            out.push((acc >> nbits) as u8);
        }
    }
    Some(out)
}

fn b64url(data: &[u8]) -> String {
    b64_encode(data)
        .replace('+', "-")
        .replace('/', "_")
        .trim_end_matches('=')
        .to_string()
}

fn pem(label: &str, der: &[u8]) -> String {
    let body = b64_encode(der);
    let mut out = format!("-----BEGIN {label}-----\n");
    for chunk in body.as_bytes().chunks(64) {
        out.push_str(std::str::from_utf8(chunk).unwrap_or_default());
        out.push('\n');
    }
    out.push_str(&format!("-----END {label}-----\n"));
    out
}

const PKCS8_PREFIX: [u8; 16] = [
    0x30, 0x2e, 0x02, 0x01, 0x00, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x04, 0x22, 0x04, 0x20,
];
const SPKI_PREFIX: [u8; 12] = [
    0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00,
];

/// Parse an Ed25519 PKCS#8 PEM, returning the 32-byte seed.
fn pkcs8_seed(pem_text: &str) -> Option<[u8; 32]> {
    let start = pem_text.find("-----BEGIN PRIVATE KEY-----")?;
    let end = pem_text.find("-----END PRIVATE KEY-----")?;
    let body = &pem_text[start + 27..end];
    let der = b64_decode(body)?;
    if der.len() != 48 || der[..16] != PKCS8_PREFIX {
        return None;
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&der[16..48]);
    Some(seed)
}

fn spki_pem(pubkey: &[u8; 32]) -> String {
    let mut der = Vec::with_capacity(44);
    der.extend_from_slice(&SPKI_PREFIX);
    der.extend_from_slice(pubkey);
    pem("PUBLIC KEY", &der)
}

fn pkcs8_pem(seed: &[u8; 32]) -> String {
    let mut der = Vec::with_capacity(48);
    der.extend_from_slice(&PKCS8_PREFIX);
    der.extend_from_slice(seed);
    pem("PRIVATE KEY", &der)
}

// ---------- SHA-512 ----------

const K512: [u64; 80] = [
    0x428a2f98d728ae22,
    0x7137449123ef65cd,
    0xb5c0fbcfec4d3b2f,
    0xe9b5dba58189dbbc,
    0x3956c25bf348b538,
    0x59f111f1b605d019,
    0x923f82a4af194f9b,
    0xab1c5ed5da6d8118,
    0xd807aa98a3030242,
    0x12835b0145706fbe,
    0x243185be4ee4b28c,
    0x550c7dc3d5ffb4e2,
    0x72be5d74f27b896f,
    0x80deb1fe3b1696b1,
    0x9bdc06a725c71235,
    0xc19bf174cf692694,
    0xe49b69c19ef14ad2,
    0xefbe4786384f25e3,
    0x0fc19dc68b8cd5b5,
    0x240ca1cc77ac9c65,
    0x2de92c6f592b0275,
    0x4a7484aa6ea6e483,
    0x5cb0a9dcbd41fbd4,
    0x76f988da831153b5,
    0x983e5152ee66dfab,
    0xa831c66d2db43210,
    0xb00327c898fb213f,
    0xbf597fc7beef0ee4,
    0xc6e00bf33da88fc2,
    0xd5a79147930aa725,
    0x06ca6351e003826f,
    0x142929670a0e6e70,
    0x27b70a8546d22ffc,
    0x2e1b21385c26c926,
    0x4d2c6dfc5ac42aed,
    0x53380d139d95b3df,
    0x650a73548baf63de,
    0x766a0abb3c77b2a8,
    0x81c2c92e47edaee6,
    0x92722c851482353b,
    0xa2bfe8a14cf10364,
    0xa81a664bbc423001,
    0xc24b8b70d0f89791,
    0xc76c51a30654be30,
    0xd192e819d6ef5218,
    0xd69906245565a910,
    0xf40e35855771202a,
    0x106aa07032bbd1b8,
    0x19a4c116b8d2d0c8,
    0x1e376c085141ab53,
    0x2748774cdf8eeb99,
    0x34b0bcb5e19b48a8,
    0x391c0cb3c5c95a63,
    0x4ed8aa4ae3418acb,
    0x5b9cca4f7763e373,
    0x682e6ff3d6b2b8a3,
    0x748f82ee5defb2fc,
    0x78a5636f43172f60,
    0x84c87814a1f0ab72,
    0x8cc702081a6439ec,
    0x90befffa23631e28,
    0xa4506cebde82bde9,
    0xbef9a3f7b2c67915,
    0xc67178f2e372532b,
    0xca273eceea26619c,
    0xd186b8c721c0c207,
    0xeada7dd6cde0eb1e,
    0xf57d4f7fee6ed178,
    0x06f067aa72176fba,
    0x0a637dc5a2c898a6,
    0x113f9804bef90dae,
    0x1b710b35131c471b,
    0x28db77f523047d84,
    0x32caab7b40c72493,
    0x3c9ebe0a15c9bebc,
    0x431d67c49c100d4c,
    0x4cc5d4becb3e42b6,
    0x597f299cfc657e2a,
    0x5fcb6fab3ad6faec,
    0x6c44198c4a475817,
];

fn sha512(data: &[u8]) -> [u8; 64] {
    let mut h: [u64; 8] = [
        0x6a09e667f3bcc908,
        0xbb67ae8584caa73b,
        0x3c6ef372fe94f82b,
        0xa54ff53a5f1d36f1,
        0x510e527fade682d1,
        0x9b05688c2b3e6c1f,
        0x1f83d9abfb41bd6b,
        0x5be0cd19137e2179,
    ];
    let bitlen = (data.len() as u128) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 128 != 112 {
        msg.push(0);
    }
    msg.extend_from_slice(&bitlen.to_be_bytes());
    for block in msg.as_chunks::<128>().0 {
        let mut w = [0u64; 80];
        for i in 0..16 {
            w[i] = u64::from_be_bytes(block[i * 8..i * 8 + 8].try_into().unwrap_or_default());
        }
        for i in 16..80 {
            let s0 = w[i - 15].rotate_right(1) ^ w[i - 15].rotate_right(8) ^ (w[i - 15] >> 7);
            let s1 = w[i - 2].rotate_right(19) ^ w[i - 2].rotate_right(61) ^ (w[i - 2] >> 6);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..80 {
            let s1 = e.rotate_right(14) ^ e.rotate_right(18) ^ e.rotate_right(41);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K512[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(28) ^ a.rotate_right(34) ^ a.rotate_right(39);
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
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 64];
    for (i, v) in h.iter().enumerate() {
        out[i * 8..i * 8 + 8].copy_from_slice(&v.to_be_bytes());
    }
    out
}

// ---------- field arithmetic mod 2^255-19 (4x u64 limbs) ----------

type Fe = [u64; 4];
const FE_P: Fe = [
    0xffffffffffffffed,
    0xffffffffffffffff,
    0xffffffffffffffff,
    0x7fffffffffffffff,
];
const FE_D: Fe = [
    0x75eb4dca135978a3,
    0x00700a4d4141d8ab,
    0x8cc740797779e898,
    0x52036cee2b6ffe73,
];

fn fe_add(a: &Fe, b: &Fe) -> Fe {
    let mut r = [0u64; 4];
    let mut carry = 0u128;
    for i in 0..4 {
        carry += a[i] as u128 + b[i] as u128;
        r[i] = carry as u64;
        carry >>= 64;
    }
    // fold the final carry (2^256 ≡ 38 mod p)
    carry *= 38;
    carry += r[0] as u128;
    r[0] = carry as u64;
    carry >>= 64;
    for x in r.iter_mut().skip(1) {
        carry += *x as u128;
        *x = carry as u64;
        carry >>= 64;
    }
    r[0] = r[0].wrapping_add((carry as u64) * 38);
    fe_norm(r)
}

fn fe_sub(a: &Fe, b: &Fe) -> Fe {
    let mut r = [0u64; 4];
    let mut borrow = 0i128;
    for i in 0..4 {
        borrow += a[i] as i128 - b[i] as i128;
        r[i] = borrow as u64;
        borrow >>= 64;
    }
    if borrow != 0 {
        // add back p once and re-borrow-normalize
        let mut carry = 0u128;
        let two_p: Fe = [
            0xffffffffffffffda,
            0xffffffffffffffff,
            0xffffffffffffffff,
            0xffffffffffffffff,
        ];
        for i in 0..4 {
            carry += r[i] as u128 + two_p[i] as u128;
            r[i] = carry as u64;
            carry >>= 64;
        }
    }
    fe_norm(r)
}

fn fe_mul(a: &Fe, b: &Fe) -> Fe {
    let mut h = [0u128; 8];
    for i in 0..4 {
        for j in 0..4 {
            h[i + j] += a[i] as u128 * b[j] as u128;
        }
    }
    // fold high limbs: 2^256 ≡ 38 (mod p)
    let mut r = [0u128; 4];
    for i in 0..4 {
        r[i] = h[i] + h[i + 4] * 38;
    }
    let mut out = [0u64; 4];
    let mut carry = 0u128;
    for i in 0..4 {
        carry += r[i];
        out[i] = carry as u64;
        carry >>= 64;
    }
    carry *= 38;
    carry += out[0] as u128;
    out[0] = carry as u64;
    carry >>= 64;
    for x in out.iter_mut().skip(1) {
        carry += *x as u128;
        *x = carry as u64;
        carry >>= 64;
    }
    out[0] = out[0].wrapping_add((carry as u64) * 38);
    fe_norm(out)
}

/// Canonicalize: carry out then subtract p until below it.
fn fe_norm(mut r: Fe) -> Fe {
    let mut c = 0u128;
    for x in r.iter_mut() {
        c += *x as u128;
        *x = c as u64;
        c >>= 64;
    }
    r[0] = r[0].wrapping_add((c as u64) * 19);
    let mut c2 = 0u128;
    for x in r.iter_mut() {
        c2 += *x as u128;
        *x = c2 as u64;
        c2 >>= 64;
    }
    r[0] = r[0].wrapping_add((c2 as u64) * 19);
    // subtract p while r >= p
    loop {
        let ge = r[3] > FE_P[3]
            || (r[3] == FE_P[3]
                && (r[2] > FE_P[2]
                    || (r[2] == FE_P[2]
                        && (r[1] > FE_P[1] || (r[1] == FE_P[1] && r[0] >= FE_P[0])))));
        if !ge {
            break;
        }
        let mut borrow = 0i128;
        for i in 0..4 {
            borrow += r[i] as i128 - FE_P[i] as i128;
            r[i] = borrow as u64;
            borrow >>= 64;
        }
    }
    r
}

fn fe_pow(base: &Fe, exp: &Fe) -> Fe {
    let mut r: Fe = [1, 0, 0, 0];
    for i in (0..256).rev() {
        r = fe_mul(&r, &r);
        if (exp[i / 64] >> (i % 64)) & 1 == 1 {
            r = fe_mul(&r, base);
        }
    }
    r
}

fn fe_inv(a: &Fe) -> Fe {
    fe_pow(
        a,
        &[
            0xffffffffffffffeb,
            0xffffffffffffffff,
            0xffffffffffffffff,
            0x7fffffffffffffff,
        ],
    )
}

fn fe_is_odd(a: &Fe) -> bool {
    fe_norm(*a)[0] & 1 == 1
}

// ---------- Edwards curve ----------

#[derive(Clone)]
struct Pt {
    x: Fe,
    y: Fe,
    z: Fe,
    t: Fe,
}

const BX: Fe = [
    0xc9562d608f25d51a,
    0x692cc7609525a7b2,
    0xc0a4e231fdd6dc5c,
    0x216936d3cd6e53fe,
];
const BY: Fe = [
    0x6666666666666658,
    0x6666666666666666,
    0x6666666666666666,
    0x6666666666666666,
];

fn pt_base() -> Pt {
    Pt {
        x: BX,
        y: BY,
        z: [1, 0, 0, 0],
        t: fe_mul(&BX, &BY),
    }
}

fn pt_identity() -> Pt {
    Pt {
        x: [0, 0, 0, 0],
        y: [1, 0, 0, 0],
        z: [1, 0, 0, 0],
        t: [0, 0, 0, 0],
    }
}

fn pt_add(p: &Pt, q: &Pt) -> Pt {
    let a = fe_mul(&p.x, &q.x);
    let b = fe_mul(&p.y, &q.y);
    let c = fe_mul(&fe_mul(&p.t, &q.t), &FE_D);
    let d = fe_mul(&p.z, &q.z);
    let e = fe_sub(
        &fe_mul(&fe_add(&p.x, &p.y), &fe_add(&q.x, &q.y)),
        &fe_add(&a, &b),
    );
    let f = fe_sub(&d, &c);
    let g = fe_add(&d, &c);
    let h = fe_add(&b, &a); // B - a*A with a = -1
    Pt {
        x: fe_mul(&e, &f),
        y: fe_mul(&g, &h),
        z: fe_mul(&f, &g),
        t: fe_mul(&e, &h),
    }
}

fn pt_scalar_mult(scalar: &[u8; 32], p: &Pt) -> Pt {
    let mut acc = pt_identity();
    for i in (0..256).rev() {
        acc = pt_add(&acc, &acc);
        if (scalar[i / 8] >> (i % 8)) & 1 == 1 {
            acc = pt_add(&acc, p);
        }
    }
    acc
}

fn pt_encode(p: &Pt) -> [u8; 32] {
    let zi = fe_inv(&p.z);
    let x = fe_mul(&p.x, &zi);
    let y = fe_mul(&p.y, &zi);
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..i * 8 + 8].copy_from_slice(&y[i].to_le_bytes());
    }
    if fe_is_odd(&x) {
        out[31] |= 0x80;
    }
    out
}

// ---------- scalar arithmetic mod l (order of the curve) ----------

const L: [u64; 4] = [
    0x5812631a5cf5d3ed,
    0x14def9dea2f79cd6,
    0x0000000000000000,
    0x1000000000000000,
];

type Sc = [u64; 4];

fn sc_cmp(a: &Sc, b: &Sc) -> std::cmp::Ordering {
    for i in (0..4).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    std::cmp::Ordering::Equal
}

fn sc_sub(a: &mut Sc, b: &Sc) {
    let mut borrow = 0i128;
    for i in 0..4 {
        borrow += a[i] as i128 - b[i] as i128;
        a[i] = borrow as u64;
        borrow >>= 64;
    }
}

fn sc_reduce_bytes(bytes: &[u8]) -> Sc {
    // bitwise long division: rem = (rem*2 + bit) mod l
    let mut rem: Sc = [0; 4];
    for &byte in bytes {
        for bit in (0..8).rev() {
            // rem <<= 1
            let mut carry = 0u64;
            for r in rem.iter_mut() {
                let nc = *r >> 63;
                *r = (*r << 1) | carry;
                carry = nc;
            }
            if carry == 1 || sc_cmp(&rem, &L) != std::cmp::Ordering::Less {
                sc_sub(&mut rem, &L);
            }
            if (byte >> bit) & 1 == 1 {
                let (v, c1) = rem[0].overflowing_add(1);
                rem[0] = v;
                let mut c = c1 as u64;
                for r in rem.iter_mut().skip(1) {
                    let (v2, c2) = r.overflowing_add(c);
                    *r = v2;
                    c = c2 as u64;
                }
                if c == 1 || sc_cmp(&rem, &L) != std::cmp::Ordering::Less {
                    sc_sub(&mut rem, &L);
                }
            }
        }
    }
    rem
}

fn sc_add(a: &Sc, b: &Sc) -> Sc {
    let mut r = [0u64; 4];
    let mut carry = 0u128;
    for i in 0..4 {
        carry += a[i] as u128 + b[i] as u128;
        r[i] = carry as u64;
        carry >>= 64;
    }
    if carry == 1 || sc_cmp(&r, &L) != std::cmp::Ordering::Less {
        sc_sub(&mut r, &L);
    }
    r
}

fn sc_mul(a: &Sc, b: &Sc) -> Sc {
    let mut r: Sc = [0; 4];
    let mut addend = *b;
    for i in 0..256 {
        if (a[i / 64] >> (i % 64)) & 1 == 1 {
            r = sc_add(&r, &addend);
        }
        // addend <<= 1 mod l
        let mut carry = 0u64;
        for x in addend.iter_mut() {
            let nc = *x >> 63;
            *x = (*x << 1) | carry;
            carry = nc;
        }
        if carry == 1 || sc_cmp(&addend, &L) != std::cmp::Ordering::Less {
            sc_sub(&mut addend, &L);
        }
    }
    r
}

// ---------- Ed25519 ----------

fn ed25519_public(seed: &[u8; 32]) -> [u8; 32] {
    let h = sha512(seed);
    let mut a = [0u8; 32];
    a.copy_from_slice(&h[..32]);
    a[0] &= 248;
    a[31] &= 127;
    a[31] |= 64;
    pt_encode(&pt_scalar_mult(&a, &pt_base()))
}

fn ed25519_sign(msg: &[u8], seed: &[u8; 32]) -> [u8; 64] {
    let h = sha512(seed);
    let mut a = [0u8; 32];
    a.copy_from_slice(&h[..32]);
    a[0] &= 248;
    a[31] &= 127;
    a[31] |= 64;
    let prefix = &h[32..];
    let pubk = ed25519_public(seed);
    let mut r_msg = Vec::with_capacity(prefix.len() + msg.len());
    r_msg.extend_from_slice(prefix);
    r_msg.extend_from_slice(msg);
    let r = sc_reduce_bytes(&sha512(&r_msg));
    let mut r_bytes = [0u8; 32];
    for i in 0..4 {
        r_bytes[i * 8..i * 8 + 8].copy_from_slice(&r[i].to_le_bytes());
    }
    let r_point = pt_encode(&pt_scalar_mult(&r_bytes, &pt_base()));
    let mut k_msg = Vec::with_capacity(64 + msg.len());
    k_msg.extend_from_slice(&r_point);
    k_msg.extend_from_slice(&pubk);
    k_msg.extend_from_slice(msg);
    let k = sc_reduce_bytes(&sha512(&k_msg));
    let mut a_le: Sc = [0; 4];
    for i in 0..4 {
        a_le[i] = u64::from_le_bytes(a[i * 8..i * 8 + 8].try_into().unwrap_or_default());
    }
    let s = sc_add(&r, &sc_mul(&k, &a_le));
    let mut sig = [0u8; 64];
    sig[..32].copy_from_slice(&r_point);
    for i in 0..4 {
        sig[32 + i * 8..32 + i * 8 + 8].copy_from_slice(&s[i].to_le_bytes());
    }
    sig
}

fn random_bytes(n: usize) -> Vec<u8> {
    use std::io::Read;
    let mut buf = vec![0u8; n];
    if std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut buf))
        .is_ok()
    {
        return buf;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (nanos >> ((i % 15) * 8)) as u8 ^ (std::process::id() >> ((i % 4) * 8)) as u8;
    }
    buf
}

fn make_uuid() -> String {
    if let Some(u) = env("SYNARA_STATE_UUID") {
        return u;
    }
    let b = random_bytes(16);
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        b[0],
        b[1],
        b[2],
        b[3],
        b[4],
        b[5],
        (b[6] & 0x0f) | 0x40,
        b[7],
        (b[8] & 0x3f) | 0x80,
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}

fn now_epoch_seconds() -> f64 {
    (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
        / 1000.0)
        .floor()
}

fn js_num(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        _ => f64::NAN,
    }
}

// ---------- identity ----------

fn jwk_for(pubk: &[u8; 32]) -> Value {
    json!({"kty": "OKP", "crv": "Ed25519", "x": b64url(pubk)})
}

fn fresh_identity() -> Value {
    if let Some(s) = env("SYNARA_HOST_IDENTITY_JSON")
        && let Ok(v) = serde_json::from_str::<Value>(&s)
    {
        return v;
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&random_bytes(32));
    let pubk = ed25519_public(&seed);
    json!({
        "version": 1,
        "privateKeyPem": pkcs8_pem(&seed),
        "publicKeyPem": spki_pem(&pubk),
        "publicKeyJwk": jwk_for(&pubk),
    })
}

fn signing_err() -> String {
    "\"pkcs8\" must be PKCS#8 formatted string".to_string()
}

/// Import a pkcs8 pem like `jose.importPKCS8`; Err matches the recorded error.
fn import_seed(identity: &Value) -> Result<[u8; 32], String> {
    let pem = prop(identity, "privateKeyPem");
    let seed = pem.as_str().and_then(pkcs8_seed);
    seed.ok_or_else(signing_err)
}

fn now_seconds_of(input: &Value) -> f64 {
    let v = prop(input, "nowSeconds");
    if undef(v) {
        now_epoch_seconds()
    } else {
        js_num(v)
    }
}

fn sign_jwt(header: Value, payload: Value, seed: &[u8; 32]) -> String {
    let h = b64url(
        serde_json::to_string(&header)
            .unwrap_or_default()
            .as_bytes(),
    );
    let p = b64url(
        serde_json::to_string(&payload)
            .unwrap_or_default()
            .as_bytes(),
    );
    let signing_input = format!("{h}.{p}");
    let sig = ed25519_sign(signing_input.as_bytes(), seed);
    format!("{signing_input}.{}", b64url(&sig))
}

/// Port of `mintHostLinkProof` (`apps/server/src/hostIdentity/jwt.ts:22`).
pub async fn mint_host_link_proof(input: Value) -> Result<String, String> {
    if input.is_null() {
        return Err(js_type_err("null", "nowSeconds"));
    }
    if !input.is_object() && undef(&input) {
        return Err(js_type_err("undefined", "nowSeconds"));
    }
    let now = now_seconds_of(&input);
    let identity = prop(&input, "identity");
    if undef(identity) {
        return Err(js_type_err("undefined", "publicKeyJwk"));
    }
    if identity.is_null() {
        return Err(js_type_err("null", "publicKeyJwk"));
    }
    let seed = import_seed(identity)?;
    let mut payload = json!({
        "challengeId": prop(&input, "challengeId"),
        "nonce": prop(&input, "nonce"),
        "publicKeyJwk": prop(identity, "publicKeyJwk"),
        "name": prop(&input, "name"),
        "platform": prop(&input, "platform"),
    });
    if let Some(v) = input.get("appVersion").filter(|v| !undef(v)) {
        payload["appVersion"] = v.clone();
    }
    payload["iss"] = json!(format!(
        "synara-host:{}",
        prop(&input, "environmentId").as_str().unwrap_or_default()
    ));
    payload["sub"] = prop(&input, "environmentId").clone();
    payload["aud"] = prop(&input, "apiIssuer").clone();
    payload["iat"] = json!(now);
    payload["exp"] = json!((now + 300.0) as i64);
    payload["jti"] = json!(make_uuid());
    Ok(sign_jwt(
        json!({"alg": "EdDSA", "typ": "synara-host-link+jwt"}),
        payload,
        &seed,
    ))
}

/// Port of `mintHostProof` (`apps/server/src/hostIdentity/jwt.ts:52`).
pub async fn mint_host_proof(input: Value) -> Result<String, String> {
    if input.is_null() {
        return Err(js_type_err("null", "identity"));
    }
    let identity = prop(&input, "identity");
    if undef(identity) {
        return Err(js_type_err("undefined", "privateKeyPem"));
    }
    if identity.is_null() {
        return Err(js_type_err("null", "privateKeyPem"));
    }
    let seed = import_seed(identity)?;
    let now = now_seconds_of(&input);
    let payload = json!({
        "keyGeneration": js_num(prop(&input, "keyGeneration")),
        "iss": format!("synara-host:{}", prop(&input, "environmentId").as_str().unwrap_or_default()),
        "sub": prop(&input, "hostId"),
        "aud": prop(&input, "apiIssuer"),
        "iat": now,
        "exp": (now + 60.0) as i64,
        "jti": make_uuid(),
    });
    Ok(sign_jwt(
        json!({"alg": "EdDSA", "typ": "synara-host-proof+jwt"}),
        payload,
        &seed,
    ))
}

/// Port of `hostIdentityPath` (`apps/server/src/hostIdentity/store.ts:28`).
pub fn host_identity_path(secrets_dir: String) -> Result<String, String> {
    Ok(posix_join(&secrets_dir, "host-identity.json"))
}

fn decode_host_identity(raw: &str) -> Result<Value, String> {
    let invalid = || "Stored host identity is invalid".to_string();
    let parsed: Value = serde_json::from_str(raw).map_err(|_| invalid())?;
    let obj = parsed.as_object().ok_or_else(invalid)?;
    if obj.get("version") != Some(&json!(1)) {
        return Err(invalid());
    }
    let priv_pem = obj
        .get("privateKeyPem")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    let pub_pem = obj
        .get("publicKeyPem")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    let seed = pkcs8_seed(priv_pem).ok_or_else(invalid)?;
    let derived = ed25519_public(&seed);
    let pub_der = b64_decode(
        pub_pem
            .find("-----BEGIN PUBLIC KEY-----")
            .map(|s| {
                &pub_pem[s + 27
                    ..pub_pem
                        .find("-----END PUBLIC KEY-----")
                        .unwrap_or(pub_pem.len())]
            })
            .unwrap_or(""),
    )
    .ok_or_else(invalid)?;
    let mut expected = Vec::with_capacity(44);
    expected.extend_from_slice(&SPKI_PREFIX);
    expected.extend_from_slice(&derived);
    if pub_der != expected {
        return Err(invalid());
    }
    Ok(json!({
        "version": 1,
        "privateKeyPem": priv_pem,
        "publicKeyPem": pub_pem,
        "publicKeyJwk": jwk_for(&derived),
    }))
}

/// Port of `readHostIdentity` (`apps/server/src/hostIdentity/store.ts:70`).
pub async fn read_host_identity(file_path: String) -> Result<Option<Value>, String> {
    match std::fs::read_to_string(&file_path) {
        Ok(raw) => decode_host_identity(&raw).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Some(json!({"$undefined": true}))),
        Err(e) if e.raw_os_error() == Some(21) => {
            Err("EISDIR: illegal operation on a directory, read".to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

/// Port of `generateAndPersistHostIdentity` (`apps/server/src/hostIdentity/store.ts:92`).
pub async fn generate_and_persist_host_identity(file_path: String) -> Result<Value, String> {
    let identity = fresh_identity();
    let persisted = json!({
        "version": identity["version"],
        "privateKeyPem": identity["privateKeyPem"],
        "publicKeyPem": identity["publicKeyPem"],
    });
    let contents = format!(
        "{}\n",
        serde_json::to_string_pretty(&persisted).unwrap_or_default()
    );
    let dir = posix_dirname(&file_path);
    let _ = std::fs::create_dir_all(&dir);
    let pid = env("SYNARA_STATE_PID")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or_else(|| std::process::id() as u64);
    let temp_path = format!("{file_path}.{pid}.{}.tmp", make_uuid());
    match std::fs::write(&temp_path, &contents) {
        Ok(()) => {}
        Err(e) => {
            let _ = std::fs::remove_file(&temp_path);
            return Err(format!(
                "{}: no such file or directory, open '{temp_path}'",
                errno_name(&e)
            ));
        }
    }
    if let Err(e) = std::fs::rename(&temp_path, &file_path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(format!(
            "{}: no such file or directory, rename '{temp_path}' -> '{file_path}'",
            errno_name(&e)
        ));
    }
    Ok(identity)
}

fn errno_name(e: &std::io::Error) -> &'static str {
    match e.raw_os_error() {
        Some(2) | None => "ENOENT",
        Some(21) => "EISDIR",
        Some(13) => "EACCES",
        Some(17) => "EEXIST",
        Some(20) => "ENOTDIR",
        _ => "ENOENT",
    }
}

/// Port of `deleteHostIdentity` (`apps/server/src/hostIdentity/store.ts:109`).
pub async fn delete_host_identity(file_path: String) -> Result<(), String> {
    let _ = std::fs::remove_file(&file_path);
    let _ = std::fs::remove_dir_all(&file_path);
    Ok(())
}
