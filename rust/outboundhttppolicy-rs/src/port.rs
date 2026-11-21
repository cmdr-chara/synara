use std::net::{Ipv4Addr, Ipv6Addr};

use serde_json::{Value, json};

use crate::js::{is_undefined, number_to_string, string_units};

const BLOCKED_V4: &[(u32, u8)] = &[
    (0x00000000, 8),  // 0.0.0.0/8
    (0x0a000000, 8),  // 10.0.0.0/8
    (0x64400000, 10), // 100.64.0.0/10
    (0x7f000000, 8),  // 127.0.0.0/8
    (0xa9fe0000, 16), // 169.254.0.0/16
    (0xac100000, 12), // 172.16.0.0/12
    (0xc0000000, 24), // 192.0.0.0/24
    (0xc0000200, 24), // 192.0.2.0/24
    (0xc0a80000, 16), // 192.168.0.0/16
    (0xc6120000, 15), // 198.18.0.0/15
    (0xc6336400, 24), // 198.51.100.0/24
    (0xcb007100, 24), // 203.0.113.0/24
    (0xe0000000, 4),  // 224.0.0.0/4
    (0xf0000000, 4),  // 240.0.0.0/4
];

const BLOCKED_V6: &[(u128, u8)] = &[
    (0x0, 128),                               // ::/128
    (0x1, 128),                               // ::1/128
    (0x0064ff9b000000000000000000000000, 96), // 64:ff9b::/96
    (0x01000000000000000000000000000000, 64), // 100::/64
    (0x20010db8000000000000000000000000, 32), // 2001:db8::/32
    (0x20010010000000000000000000000000, 28), // 2001:10::/28
    (0xfc000000000000000000000000000000, 7),  // fc00::/7
    (0xfe800000000000000000000000000000, 10), // fe80::/10
    (0xff000000000000000000000000000000, 8),  // ff00::/8
];

fn in_v4_list(ip: u32, list: &[(u32, u8)]) -> bool {
    list.iter().any(|&(net, p)| {
        let mask: u32 = if p == 0 { 0 } else { u32::MAX << (32 - p) };
        ip & mask == net & mask
    })
}

fn in_v6_list(ip: u128, list: &[(u128, u8)]) -> bool {
    list.iter().any(|&(net, p)| {
        let mask: u128 = if p == 0 { 0 } else { u128::MAX << (128 - p) };
        ip & mask == net & mask
    })
}

fn js_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        v if is_undefined(v) => "undefined".to_string(),
        v if crate::js::as_number(v).is_some() => {
            number_to_string(crate::js::as_number(v).unwrap_or(f64::NAN))
        }
        v @ Value::Object(_) if string_units(v).is_some() => string_units(v)
            .and_then(|u| String::from_utf16(&u).ok())
            .unwrap_or_default(),
        Value::Array(items) => items
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                v if is_undefined(v) => String::new(),
                v => js_string(v),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_string(),
    }
}

fn to_key(v: &Value) -> Result<String, String> {
    if let Value::Object(map) = v
        && map.contains_key("toString")
    {
        return Err("Cannot convert object to primitive value".to_string());
    }
    Ok(js_string(v))
}

fn node_is_ip(s: &str) -> u8 {
    if s.parse::<Ipv4Addr>().is_ok() {
        4
    } else if s.parse::<Ipv6Addr>().is_ok() {
        6
    } else {
        0
    }
}

fn instance_err(v: &Value) -> String {
    let what = match v {
        Value::Array(_) => "Array",
        Value::Object(_) => "Object",
        _ => "Object",
    };
    format!("The \"address\" argument must be of type string. Received an instance of {what}")
}

fn family_of(address: &Value) -> Result<u8, String> {
    match address {
        Value::String(s) => Ok(node_is_ip(s)),
        _ => {
            let family = node_is_ip(&to_key(address)?);
            if family != 0 {
                Err(instance_err(address))
            } else {
                Ok(0)
            }
        }
    }
}

fn is_public(address: &Value) -> Result<bool, String> {
    match family_of(address)? {
        4 => {
            let ip: Ipv4Addr = js_string(address).parse().map_err(|_| String::new())?;
            Ok(!in_v4_list(u32::from(ip), BLOCKED_V4))
        }
        6 => {
            let ip: Ipv6Addr = js_string(address).parse().map_err(|_| String::new())?;
            if ip.to_ipv4_mapped().is_some() {
                return Ok(false);
            }
            Ok(!in_v6_list(u128::from(ip), BLOCKED_V6))
        }
        _ => Ok(false),
    }
}

/// Port of `isPublicIpAddress` (`packages/shared/src/outboundHttpPolicy.ts`).
pub fn is_public_ip_address(address: Value) -> Result<bool, String> {
    is_public(&address)
}

/// Port of `assertPublicIpAddress` (`packages/shared/src/outboundHttpPolicy.ts`).
pub fn assert_public_ip_address(address: Value) -> Result<Value, String> {
    if !is_public(&address)? {
        return Err(
            "Outbound destination resolved to a private, local, reserved, or invalid address."
                .to_string(),
        );
    }
    Ok(crate::js::undefined())
}

fn truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        v if is_undefined(v) => false,
        v if crate::js::as_number(v).is_some() => {
            crate::js::as_number(v).is_some_and(|x| x != 0.0 && !x.is_nan())
        }
        _ => true,
    }
}

fn prop_read_err(v: &Value, key: &str) -> String {
    let name = if v.is_null() { "null" } else { "undefined" };
    format!("Cannot read properties of {name} (reading '{key}')")
}

/// Port of `assertExactLoopbackIpAddress` (`packages/shared/src/outboundHttpPolicy.ts`).
pub fn assert_exact_loopback_ip_address(
    address: Value,
    allow_ipv4_subnet: Value,
) -> Result<Value, String> {
    let is_exact = matches!(&address, Value::String(s) if s == "127.0.0.1");
    if !is_exact {
        let lower = match &address {
            Value::String(s) => s.to_lowercase(),
            v if v.is_null() || is_undefined(v) => return Err(prop_read_err(v, "toLowerCase")),
            _ => return Err("address.toLowerCase is not a function".to_string()),
        };
        if lower != "::1" {
            let subnet_ok = truthy(&allow_ipv4_subnet)
                && node_is_ip(&to_key(&address)?) == 4
                && match &address {
                    Value::String(s) => s.starts_with("127."),
                    v if v.is_null() || is_undefined(v) => {
                        return Err(prop_read_err(v, "startsWith"));
                    }
                    _ => return Err("address.startsWith is not a function".to_string()),
                };
            if !subnet_ok {
                return Err(
                    "Outbound loopback destination resolved to a non-loopback address.".to_string(),
                );
            }
        }
    }
    Ok(crate::js::undefined())
}

/// Port of `isBenchmarkIpAddress` (`packages/shared/src/outboundHttpPolicy.ts`).
pub fn is_benchmark_ip_address(address: Value) -> Result<bool, String> {
    if family_of(&address)? != 4 {
        return Ok(false);
    }
    let ip: Ipv4Addr = js_string(&address).parse().map_err(|_| String::new())?;
    Ok(in_v4_list(u32::from(ip), &[(0xc6120000, 15)]))
}

fn field(v: &Value, key: &str) -> Result<Value, String> {
    match v {
        Value::Null => Err(prop_read_err(v, key)),
        v if is_undefined(v) => Err(prop_read_err(v, key)),
        Value::Object(_) => Ok(v.get(key).cloned().unwrap_or_else(crate::js::undefined)),
        _ => Ok(crate::js::undefined()),
    }
}

fn js_to_number(v: &Value) -> Result<f64, String> {
    match v {
        v if crate::js::as_number(v).is_some() => Ok(crate::js::as_number(v).unwrap_or(f64::NAN)),
        Value::String(s) => Ok(js_parse_number(s)),
        Value::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        Value::Null => Ok(0.0),
        v if is_undefined(v) => Ok(f64::NAN),
        Value::Number(n) => Ok(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(items) => Ok(if items.is_empty() {
            0.0
        } else {
            js_parse_number(&js_string(v))
        }),
        Value::Object(map) => {
            if map.contains_key("toString") {
                Err("Cannot convert object to primitive value".to_string())
            } else {
                Ok(f64::NAN)
            }
        }
    }
}

fn js_parse_number(s: &str) -> f64 {
    let t = s.trim_matches(|c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    });
    if t.is_empty() {
        return 0.0;
    }
    let lower = t.to_ascii_lowercase();
    let (sign, body) = match t.as_bytes().first() {
        Some(b'-') => (-1.0, &lower[1..]),
        Some(b'+') => (1.0, &lower[1..]),
        _ => (1.0, lower.as_str()),
    };
    if body == "infinity" {
        return sign * f64::INFINITY;
    }
    if let Some(hex) = body.strip_prefix("0x") {
        return sign
            * u64::from_str_radix(hex, 16)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
    }
    if let Some(oct) = body.strip_prefix("0o") {
        return sign
            * u64::from_str_radix(oct, 8)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
    }
    if let Some(bin) = body.strip_prefix("0b") {
        return sign
            * u64::from_str_radix(bin, 2)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
    }
    sign * t.parse::<f64>().unwrap_or(f64::NAN)
}

/// Port of `assertJsonWithinLimits` (`packages/shared/src/outboundHttpPolicy.ts`).
pub fn assert_json_within_limits(value: Value, limits: Value) -> Result<Value, String> {
    let mut stack: Vec<(&Value, i64)> = vec![(&value, 0)];
    let mut nodes: f64 = 0.0;
    while let Some((current, depth)) = stack.pop() {
        nodes += 1.0;
        let max_nodes = field(&limits, "maxNodes")?;
        if nodes > js_to_number(&max_nodes)? {
            return Err(format!(
                "Outbound JSON exceeded the {}-node limit.",
                js_string(&max_nodes)
            ));
        }
        let max_depth = field(&limits, "maxDepth")?;
        if (depth as f64) > js_to_number(&max_depth)? {
            return Err(format!(
                "Outbound JSON exceeded the depth limit of {}.",
                js_string(&max_depth)
            ));
        }
        match current {
            Value::Object(map) => {
                for child in map.values() {
                    stack.push((child, depth + 1));
                }
            }
            Value::Array(items) => {
                for child in items {
                    stack.push((child, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(crate::js::undefined())
}

fn is_tchar(c: u16) -> bool {
    matches!(
        c,
        33 | 35..=39 | 42 | 43 | 45 | 46 | 48..=57 | 65..=90 | 94 | 95 | 96 | 97..=122 | 124 | 126
    )
}

fn byte_string_check(v: &Value) -> Result<String, String> {
    let s = js_string(v);
    for (i, u) in s.encode_utf16().enumerate() {
        if u > 255 {
            return Err(format!(
                "Cannot convert argument to a ByteString because the character at index {i} has a value of {u} which is greater than 255."
            ));
        }
    }
    Ok(s)
}

fn append_header(name: &Value, value: &Value) -> Result<(), String> {
    let name_str = byte_string_check(name)?;
    let name_units: Vec<u16> = name_str.encode_utf16().collect();
    if name_units.is_empty() || !name_units.iter().all(|&u| is_tchar(u)) {
        return Err(format!(
            "Headers.append: \"{name_str}\" is an invalid header name."
        ));
    }
    let value_str = byte_string_check(value)?;
    if value_str.contains(['\0', '\r', '\n']) {
        return Err(format!(
            "Headers.append: \"{value_str}\" is an invalid header value."
        ));
    }
    Ok(())
}

const HEADERS_INIT_ERR: &str = "Headers constructor: Argument 1 could not be converted to one of: sequence<sequence<ByteString>>, record<ByteString, ByteString>.";

/// Port of `stripOutboundSensitiveHeaders` (`packages/shared/src/outboundHttpPolicy.ts`).
pub fn strip_outbound_sensitive_headers(headers: Value) -> Result<Value, String> {
    match &headers {
        v if is_undefined(v) => Ok(json!({})),
        Value::Array(items) => {
            for (i, el) in items.iter().enumerate() {
                match el {
                    Value::Array(pair) => {
                        if pair.len() != 2 {
                            return Err(format!(
                                "Headers constructor: expected name/value pair to be length 2, found {}.",
                                pair.len()
                            ));
                        }
                        append_header(&pair[0], &pair[1])?;
                    }
                    _ => {
                        let desc = match el {
                            Value::String(s) => format!("\"{s}\""),
                            v => js_string(v),
                        };
                        return Err(format!(
                            "Headers constructor: init[{i}] ({desc}) is not iterable."
                        ));
                    }
                }
            }
            Ok(json!({}))
        }
        Value::Object(map) => {
            for (k, v) in map {
                append_header(&Value::String(k.clone()), v)?;
            }
            Ok(json!({}))
        }
        _ => Err(HEADERS_INIT_ERR.to_string()),
    }
}
