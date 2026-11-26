//! Browser network policy mirroring BetterWright's default `NetworkPolicy`.
//! Every navigation and every CDP-intercepted page request passes through this
//! check; cloud metadata endpoints stay blocked even though local development
//! addresses are allowed.

use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Clone, Copy, Debug)]
pub struct NetworkPolicy {
    pub allow_loopback: bool,
    pub allow_private_network: bool,
}

impl Default for NetworkPolicy {
    fn default() -> Self {
        Self {
            allow_loopback: true,
            allow_private_network: true,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AddressClass {
    Loopback,
    Private,
    Metadata,
    Public,
}

const METADATA_HOSTNAMES: [&str; 2] = ["metadata.google.internal", "metadata.goog"];
const METADATA_IPV4: [Ipv4Addr; 3] = [
    Ipv4Addr::new(169, 254, 169, 254),
    Ipv4Addr::new(169, 254, 170, 2),
    Ipv4Addr::new(100, 100, 100, 200),
];

fn classify_ipv4(ip: Ipv4Addr) -> AddressClass {
    if METADATA_IPV4.contains(&ip) {
        return AddressClass::Metadata;
    }
    let octets = ip.octets();
    if ip.is_loopback() {
        return AddressClass::Loopback;
    }
    let private = octets[0] == 10
        || (octets[0] == 172 && (16..=31).contains(&octets[1]))
        || (octets[0] == 192 && octets[1] == 168)
        || (octets[0] == 169 && octets[1] == 254)
        || (octets[0] == 100 && (64..=127).contains(&octets[1]))
        || octets[0] == 0
        || (octets[0] == 192 && octets[1] == 0 && octets[2] == 2)
        || (octets[0] == 198 && (18..=19).contains(&octets[1]))
        || (octets[0] == 198 && octets[1] == 51 && octets[2] == 100)
        || (octets[0] == 203 && octets[1] == 0 && octets[2] == 113)
        || octets[0] >= 224;
    if private {
        AddressClass::Private
    } else {
        AddressClass::Public
    }
}

fn classify_ipv6(ip: Ipv6Addr) -> AddressClass {
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return classify_ipv4(mapped);
    }
    let segments = ip.segments();
    if ip == Ipv6Addr::new(0xfd00, 0xec2, 0, 0, 0, 0, 0, 0x254)
        || (segments[0] == 0xfd00 && segments[1] == 0x00ec)
    {
        return AddressClass::Metadata;
    }
    if ip.is_loopback() {
        return AddressClass::Loopback;
    }
    let first = segments[0];
    let private = ip.is_unspecified()
        || (first & 0xffc0) == 0xfe80
        || (first & 0xffc0) == 0xfec0
        || (first & 0xfe00) == 0xfc00
        || (first & 0xff00) == 0xff00;
    if private {
        AddressClass::Private
    } else {
        AddressClass::Public
    }
}

fn classify_host(host: &str) -> Option<AddressClass> {
    let bare = host.trim_matches(['[', ']']);
    if let Ok(ip) = bare.parse::<Ipv4Addr>() {
        return Some(classify_ipv4(ip));
    }
    if let Ok(ip) = bare.parse::<Ipv6Addr>() {
        return Some(classify_ipv6(ip));
    }
    None
}

impl NetworkPolicy {
    /// Top-level navigation is narrower than subresource fetch policy: data
    /// and blob documents are legal inside pages, but browser_open/navigate
    /// only take real web URLs (matching upstream `validateWebUrl`).
    pub fn check_navigation(&self, raw_url: &str) -> Result<(), String> {
        self.check(raw_url)?;
        if raw_url.eq_ignore_ascii_case("about:blank") {
            return Ok(());
        }
        let parsed = url::Url::parse(raw_url).map_err(|_| "invalid URL".to_string())?;
        if matches!(parsed.scheme(), "http" | "https") {
            Ok(())
        } else {
            Err("BrowserNavigationBlocked".into())
        }
    }

    pub fn check(&self, raw_url: &str) -> Result<(), String> {
        let raw_url = raw_url.trim();
        if raw_url.is_empty() {
            return Err("empty URL".into());
        }
        if raw_url.eq_ignore_ascii_case("about:blank")
            || raw_url.starts_with("data:")
            || raw_url.starts_with("blob:")
        {
            return Ok(());
        }
        let parsed = url::Url::parse(raw_url).map_err(|_| "invalid URL".to_string())?;
        let scheme = parsed.scheme().to_ascii_lowercase();
        if !matches!(scheme.as_str(), "http" | "https" | "ws" | "wss") {
            return Err(format!("unsupported browser URL scheme: {scheme}"));
        }
        let host = parsed
            .host_str()
            .ok_or_else(|| "URL has no hostname".to_string())?
            .trim_matches(['[', ']'])
            .to_ascii_lowercase();
        if METADATA_HOSTNAMES.contains(&host.as_str()) {
            return Err("cloud metadata endpoint".into());
        }
        match classify_host(&host) {
            Some(AddressClass::Metadata) => Err("cloud metadata endpoint".into()),
            Some(AddressClass::Loopback) if !self.allow_loopback && !self.allow_private_network => {
                Err("loopback address (set allowLoopback for local dev)".into())
            }
            Some(AddressClass::Private) if !self.allow_private_network => {
                Err("private or internal address".into())
            }
            Some(_) => Ok(()),
            None => {
                if host == "localhost" || host.ends_with(".localhost") {
                    if self.allow_loopback || self.allow_private_network {
                        Ok(())
                    } else {
                        Err("loopback address (set allowLoopback for local dev)".into())
                    }
                } else if (!host.contains('.')
                    || host.ends_with(".local")
                    || host.ends_with(".internal")
                    || host.ends_with(".lan"))
                    && !self.allow_private_network
                {
                    Err("private or internal hostname".into())
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> NetworkPolicy {
        NetworkPolicy::default()
    }

    #[test]
    fn allows_web_and_local_development_urls() {
        for url in [
            "https://example.com/path",
            "http://127.0.0.1:3000",
            "http://localhost:5173",
            "http://192.168.1.20",
            "about:blank",
            "data:text/plain,hello",
        ] {
            assert!(policy().check(url).is_ok(), "{url}");
        }
    }

    #[test]
    fn blocks_metadata_and_unsupported_schemes() {
        for url in [
            "http://169.254.169.254/latest/meta-data",
            "http://metadata.google.internal/",
            "http://[fd00:ec2::254]/",
            "file:///etc/passwd",
            "chrome://settings",
            "not a url",
        ] {
            assert!(policy().check(url).is_err(), "{url}");
        }
    }

    #[test]
    fn navigation_is_narrower_than_subresource_policy() {
        assert!(policy().check("data:text/plain,hello").is_ok());
        assert!(policy().check_navigation("data:text/plain,hello").is_err());
        assert!(policy().check_navigation("about:blank").is_ok());
        assert!(policy().check_navigation("https://example.com").is_ok());
    }

    #[test]
    fn hardened_policy_blocks_private_and_loopback_but_not_metadata_override() {
        let hardened = NetworkPolicy {
            allow_loopback: false,
            allow_private_network: false,
        };
        assert!(hardened.check("http://127.0.0.1").is_err());
        assert!(hardened.check("http://10.0.0.4").is_err());
        assert!(hardened.check("http://router.lan").is_err());
        assert!(hardened.check("https://example.com").is_ok());
    }
}
