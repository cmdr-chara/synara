//! The Rust port of `apps/server/src/endpointReporter.ts`.

use serde_json::Value;

/// `String.prototype.localeCompare` for the JSON array literals this module
/// sorts: ICU collation orders symbols before digits before letters, with
/// case-insensitive letter comparison and lowercase before uppercase.
fn locale_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn class(c: char) -> u8 {
        if c.is_ascii_digit() {
            1
        } else if c.is_ascii_alphabetic() {
            2
        } else {
            0
        }
    }
    let mut ai = a.chars();
    let mut bi = b.chars();
    loop {
        match (ai.next(), bi.next()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(ca), Some(cb)) => {
                let (cla, clb) = (class(ca), class(cb));
                if cla != clb {
                    return cla.cmp(&clb);
                }
                let (la, lb) = (ca.to_ascii_lowercase(), cb.to_ascii_lowercase());
                if la != lb {
                    return la.cmp(&lb);
                }
                if ca != cb {
                    return ca.cmp(&cb);
                }
            }
        }
    }
}

#[cfg(unix)]
mod nics {
    use serde_json::{Value, json};
    use std::ffi::CStr;
    use std::net::{Ipv4Addr, Ipv6Addr};

    pub fn collect() -> Vec<Value> {
        let mut entries = Vec::new();
        // SAFETY: getifaddrs owns the returned list; it is freed via
        // freeifaddrs before returning. Fields are read only while valid.
        unsafe {
            let mut head: *mut libc::ifaddrs = std::ptr::null_mut();
            if libc::getifaddrs(&mut head) != 0 {
                return entries;
            }
            let mut cur = head;
            while !cur.is_null() {
                let ifa = &*cur;
                let name = if ifa.ifa_name.is_null() {
                    String::new()
                } else {
                    CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned()
                };
                let internal = ifa.ifa_flags & (libc::IFF_LOOPBACK as u32) != 0;
                if !ifa.ifa_addr.is_null() {
                    let family = (*ifa.ifa_addr).sa_family as i32;
                    if family == libc::AF_INET {
                        let sa = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                        let addr = Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr)).to_string();
                        entries.push(json!([name, addr, "IPv4", internal, Value::Null]));
                    } else if family == libc::AF_INET6 {
                        let sa = &*(ifa.ifa_addr as *const libc::sockaddr_in6);
                        let addr = Ipv6Addr::from(sa.sin6_addr.s6_addr).to_string();
                        entries.push(json!([name, addr, "IPv6", internal, sa.sin6_scope_id]));
                    }
                }
                cur = ifa.ifa_next;
            }
            libc::freeifaddrs(head);
        }
        entries
    }
}

#[cfg(not(unix))]
mod nics {
    use serde_json::Value;

    pub fn collect() -> Vec<Value> {
        Vec::new()
    }
}

/// Port of `networkFingerprint` (`apps/server/src/endpointReporter.ts:11`).
pub fn network_fingerprint() -> Result<String, String> {
    let mut entries = nics::collect();
    entries.sort_by(|a, b| {
        locale_cmp(
            &serde_json::to_string(a).unwrap_or_default(),
            &serde_json::to_string(b).unwrap_or_default(),
        )
    });
    Ok(serde_json::to_string(&Value::Array(entries)).unwrap_or_else(|_| "[]".to_string()))
}
