//! Protocol-independent validation shared by transports and native presentation.
use crate::{AgentError, AgentResult};
use std::{collections::HashSet, net::Ipv6Addr};
use synara_core::{InputFieldKind, SelectChoice, UserInputRequest};

fn invalid(message: &str) -> AgentError {
    AgentError::Invalid(message.into())
}

/// Validate navigation without fetching, resolving DNS or opening a browser.
/// Userinfo, escaped authorities and ambiguous backslashes are deliberately rejected.
/// Loopback HTTP is supported for explicit local authentication actions.
pub fn validate_web_url(value: &str) -> AgentResult<()> {
    if value.len() > 8192 {
        return Err(AgentError::Limit);
    }
    let rest = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .ok_or_else(|| invalid("Only HTTP and HTTPS navigation is supported"))?;
    if value.chars().any(|c| c.is_control() || c.is_whitespace())
        || value.contains('\\')
        || !valid_percent_encoding(value)
    {
        return Err(invalid("Invalid website address"));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains(['@', '%']) {
        return Err(invalid(
            "Website authority must not contain credentials or escapes",
        ));
    }
    let port = if let Some(ipv6) = authority.strip_prefix('[') {
        let (ip, suffix) = ipv6
            .split_once(']')
            .ok_or_else(|| invalid("Invalid IPv6 website address"))?;
        ip.parse::<Ipv6Addr>()
            .map_err(|_| invalid("Invalid IPv6 website address"))?;
        if suffix.is_empty() {
            None
        } else {
            Some(
                suffix
                    .strip_prefix(':')
                    .ok_or_else(|| invalid("Invalid website port"))?,
            )
        }
    } else {
        let (host, port) = authority
            .split_once(':')
            .map_or((authority, None), |(h, p)| (h, Some(p)));
        if !valid_host(host) {
            return Err(invalid("Invalid website host"));
        }
        port
    };
    if let Some(port) = port
        && (port.is_empty()
            || !port.bytes().all(|c| c.is_ascii_digit())
            || port.parse::<u16>().is_err())
    {
        return Err(invalid("Invalid website port"));
    }
    Ok(())
}

fn valid_host(host: &str) -> bool {
    let host = host.strip_suffix('.').unwrap_or(host);
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .as_bytes()
                    .last()
                    .is_some_and(u8::is_ascii_alphanumeric)
                && label
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
        })
}

fn valid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if bytes.get(i + 1).is_none_or(|b| !b.is_ascii_hexdigit())
                || bytes.get(i + 2).is_none_or(|b| !b.is_ascii_hexdigit())
            {
                return false;
            }
            i += 2;
        }
        i += 1;
    }
    true
}

/// Reject impossible or unbounded presentation before it reaches the UI queue.
/// These are application limits, not inferred provider capabilities.
pub fn validate_input_request(request: &UserInputRequest) -> AgentResult<()> {
    if request.id.is_empty() || request.id.len() > 128 || request.id.chars().any(char::is_control) {
        return Err(invalid("Invalid question identifier"));
    }
    if request.message.len() > 64 * 1024 || request.fields.len() > 32 {
        return Err(AgentError::Limit);
    }
    if let Some(url) = &request.url {
        validate_web_url(url)?;
        if !request.fields.is_empty() {
            return Err(invalid(
                "A website interaction cannot also collect form values",
            ));
        }
        return Ok(());
    }
    let mut seen = HashSet::new();
    let mut bytes = request.message.len();
    for field in &request.fields {
        if field.id.is_empty()
            || field.id.len() > 128
            || field.id.chars().any(char::is_control)
            || !seen.insert(&field.id)
        {
            return Err(invalid("Invalid or repeated question field"));
        }
        if field.label.len() > 4096 {
            return Err(AgentError::Limit);
        }
        bytes += field.id.len() + field.label.len();
        match &field.kind {
            InputFieldKind::Text {
                min_length,
                max_length,
                format,
            } => {
                check_bounds(*min_length, *max_length)?;
                if min_length.is_some_and(|n| n > 64 * 1024) {
                    return Err(AgentError::Limit);
                }
                if format
                    .as_deref()
                    .is_some_and(|f| !matches!(f, "email" | "uri" | "date" | "date-time"))
                {
                    return Err(AgentError::Unsupported("question text format".into()));
                }
            }
            InputFieldKind::Number {
                minimum,
                maximum,
                integer,
            } => {
                if minimum.is_some_and(|n| !n.is_finite())
                    || maximum.is_some_and(|n| !n.is_finite())
                {
                    return Err(invalid("Non-finite question bound"));
                }
                check_bounds(*minimum, *maximum)?;
                if *integer
                    && minimum
                        .zip(*maximum)
                        .is_some_and(|(a, b)| a.ceil() > b.floor())
                {
                    return Err(invalid("Question range contains no integer"));
                }
            }
            InputFieldKind::Choice { options } => bytes += validate_choices(options)?,
            InputFieldKind::MultiChoice {
                options,
                minimum,
                maximum,
            } => {
                bytes += validate_choices(options)?;
                check_bounds(*minimum, *maximum)?;
                if minimum.is_some_and(|n| n > options.len()) {
                    return Err(invalid("Question requires more choices than offered"));
                }
            }
            InputFieldKind::Boolean => {}
        }
        if bytes > 256 * 1024 {
            return Err(AgentError::Limit);
        }
    }
    Ok(())
}
fn check_bounds<T: PartialOrd + Copy>(minimum: Option<T>, maximum: Option<T>) -> AgentResult<()> {
    if minimum.zip(maximum).is_some_and(|(a, b)| a > b) {
        return Err(invalid("Question minimum exceeds maximum"));
    }
    Ok(())
}
fn validate_choices(options: &[SelectChoice]) -> AgentResult<usize> {
    if options.is_empty() || options.len() > 512 {
        return Err(AgentError::Limit);
    }
    let mut seen = HashSet::new();
    let mut bytes = 0;
    for option in options {
        if !seen.insert(&option.value) {
            return Err(invalid("Repeated question choice"));
        }
        if option.value.len() > 4096 || option.label.len() > 4096 {
            return Err(AgentError::Limit);
        }
        bytes += option.value.len() + option.label.len();
    }
    Ok(bytes)
}

pub(crate) fn valid_text_format(text: &str, format: Option<&str>) -> bool {
    match format {
        None => true,
        Some("date") => valid_date(text),
        Some("date-time") => valid_date_time(text),
        Some("email") => text.rsplit_once('@').is_some_and(|(local, host)| {
            !local.is_empty()
                && local.len() <= 64
                && !local.starts_with('.')
                && !local.ends_with('.')
                && !local.contains("..")
                && local
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&c))
                && valid_host(host)
        }),
        Some("uri") => text.split_once(':').is_some_and(|(scheme, rest)| {
            !rest.is_empty()
                && scheme
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphabetic)
                && scheme
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"+.-".contains(&c))
                && !text.chars().any(|c| c.is_whitespace() || c.is_control())
                && !text.contains('\\')
                && valid_percent_encoding(text)
                && (!matches!(scheme, "http" | "https") || validate_web_url(text).is_ok())
        }),
        _ => false,
    }
}
fn valid_date(text: &str) -> bool {
    if text.len() != 10 || !text.is_ascii() || &text[4..5] != "-" || &text[7..8] != "-" {
        return false;
    }
    if !text
        .bytes()
        .enumerate()
        .all(|(i, c)| matches!(i, 4 | 7) || c.is_ascii_digit())
    {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        text[..4].parse::<u32>(),
        text[5..7].parse::<u32>(),
        text[8..].parse::<u32>(),
    ) else {
        return false;
    };
    let days = match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        _ => return false,
    };
    day > 0 && day <= days
}
fn valid_date_time(text: &str) -> bool {
    if !text.is_ascii()
        || text.len() < 20
        || !valid_date(&text[..10])
        || !matches!(&text[10..11], "T" | "t")
    {
        return false;
    }
    let time = &text[11..];
    if &time[2..3] != ":" || &time[5..6] != ":" {
        return false;
    }
    if !time[..8]
        .bytes()
        .enumerate()
        .all(|(i, c)| matches!(i, 2 | 5) || c.is_ascii_digit())
    {
        return false;
    }
    let (Ok(h), Ok(m), Ok(s)) = (
        time[..2].parse::<u8>(),
        time[3..5].parse::<u8>(),
        time[6..8].parse::<u8>(),
    ) else {
        return false;
    };
    if h > 23 || m > 59 || s > 59 {
        return false;
    }
    let suffix = &time[8..];
    let zone = if let Some(fraction) = suffix.strip_prefix('.') {
        let n = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if n == 0 {
            return false;
        }
        &fraction[n..]
    } else {
        suffix
    };
    if matches!(zone, "Z" | "z") {
        return true;
    }
    if zone.len() != 6 || !matches!(&zone[..1], "+" | "-") || &zone[3..4] != ":" {
        return false;
    }
    if !zone[1..]
        .bytes()
        .enumerate()
        .all(|(i, c)| i == 2 || c.is_ascii_digit())
    {
        return false;
    }
    matches!((zone[1..3].parse::<u8>(), zone[4..6].parse::<u8>()), (Ok(h), Ok(m)) if h <= 23 && m <= 59)
}

#[cfg(test)]
#[path = "input_validation_tests.rs"]
mod tests;
