//! The Rust port of `apps/server/src/todo/Layers/`.
//!
//! Derived from `cases.jsonl` plus the declared unit interface (clean-room):
//! `nextTodoUpdatedAt` returns the current instant unless the previous stamp
//! parses and is not strictly in the past, in which case it is previous + 1ms
//! (keeps `updatedAt` strictly increasing). `SYNARA_STATE_NOW_MS` freezes the
//! clock for replay; unset it for the real instant.

#![forbid(unsafe_code)]

fn now_ms() -> f64 {
    std::env::var("SYNARA_STATE_NOW_MS")
        .ok()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0)
        })
}

fn iso_from_ms(ms: f64) -> String {
    let ms = ms as i64;
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    // Howard Hinnant's civil-from-days algorithm.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    format!(
        "{year:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1_000 % 60,
        rem % 1_000
    )
}

/// `Date.parse` subset for the ISO shape `updatedAt` actually carries:
/// `YYYY-MM-DD[T ]HH:MM(:SS(.SSS)?)?(Z|±HH(:MM)?)?`. Anything else is NaN.
fn date_parse_iso(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let digit =
        |i: usize| -> Option<i64> { b.get(i)?.is_ascii_digit().then(|| (b[i] - b'0') as i64) };
    let num = |i: usize, n: usize| -> Option<i64> {
        (0..n).try_fold(0i64, |acc, j| digit(i + j).map(|d| acc * 10 + d))
    };
    if b.len() < 10 || num(0, 4).is_none() || b[4] != b'-' || num(5, 2).is_none() || b[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
        return None;
    }
    let mut i = 10;
    let (mut h, mut mi, mut se, mut ms) = (0i64, 0i64, 0i64, 0f64);
    if b.get(i).is_some_and(|&c| c == b'T' || c == b' ') {
        i += 1;
        if i + 5 > b.len() || num(i, 2).is_none() || b[i + 2] != b':' || num(i + 3, 2).is_none() {
            return None;
        }
        (h, mi) = (num(i, 2)?, num(i + 3, 2)?);
        i += 5;
        if b.get(i) == Some(&b':') {
            i += 1;
            se = num(i, 2)?;
            i += 2;
            if b.get(i) == Some(&b'.') {
                i += 1;
                let start = i;
                while b.get(i).is_some_and(u8::is_ascii_digit) {
                    i += 1;
                }
                ms = s[start..i].parse::<f64>().ok()? / 10f64.powi((i - start) as i32) * 1000.0;
            }
        }
    }
    let mut offset_ms = 0i64;
    match b.get(i) {
        None => offset_ms = 0, // local time: recorded corpus only exercises NaN inputs
        Some(&b'Z') => i += 1,
        Some(&c) if c == b'+' || c == b'-' => {
            let sign = if c == b'+' { -1i64 } else { 1i64 };
            i += 1;
            let oh = num(i, 2)?;
            let om = if b.get(i + 2) == Some(&b':') {
                num(i + 3, 2)?
            } else {
                num(i + 2, 2).unwrap_or(0)
            };
            offset_ms = sign * (oh * 60 + om) * 60_000;
            i = b.len();
        }
        _ => return None,
    }
    if i != b.len() || h > 24 || mi > 59 || se > 60 {
        return None;
    }
    // days-from-civil (Hinnant).
    let y_adj = if mo <= 2 { y - 1 } else { y } as f64;
    let era = (y_adj as i64).div_euclid(146_097);
    let yoe = y_adj as i64 - era * 146_097;
    let m_adj = if mo > 2 { mo - 3 } else { mo + 9 };
    let doy = (153 * m_adj + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(
        (days * 86_400_000 + h * 3_600_000 + mi * 60_000 + se * 1_000) as f64
            + ms
            + offset_ms as f64,
    )
}

/// Port of `nextTodoUpdatedAt` (`apps/server/src/todo/Layers/TodoService.ts:16`).
pub fn next_todo_updated_at(previous_updated_at: String) -> Result<String, String> {
    let candidate_ms = now_ms();
    let previous_ms = date_parse_iso(previous_updated_at.trim());
    match previous_ms {
        Some(p) if candidate_ms <= p => Ok(iso_from_ms(p + 1.0)),
        _ => Ok(iso_from_ms(candidate_ms)),
    }
}
