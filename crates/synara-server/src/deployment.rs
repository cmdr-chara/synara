//! Explicit HTTPS origin policy for a TLS proxy on the same host.
use super::{Request, ServerConfig, is_loopback_name, parse_authority, parse_origin};
use anyhow::{Result, ensure};

#[derive(Clone)]
pub(super) struct PublicOrigin {
    origin: String,
    authority: String,
}

impl PublicOrigin {
    fn parse(value: &str) -> Result<Self> {
        let authority = value.strip_prefix("https://").unwrap_or_default();
        ensure!(
            !authority.is_empty() && authority.len() <= 260,
            "public origin must be an HTTPS origin without a path"
        );
        let authority = canonical_authority(authority)
            .ok_or_else(|| anyhow::anyhow!("invalid public HTTPS origin"))?;
        Ok(Self {
            origin: format!("https://{authority}"),
            authority,
        })
    }
}

// Deliberately accepts only ASCII DNS names or bracketed IPv6, with an optional
// explicit port. Credentials, path/query/fragment syntax and ambiguous ports
// cannot become part of the trusted proxy authority.
fn canonical_authority(value: &str) -> Option<String> {
    if value.is_empty() || !value.is_ascii() || value.bytes().any(|b| b.is_ascii_whitespace()) {
        return None;
    }
    let (host, port) = if let Some(rest) = value.strip_prefix('[') {
        let (ip, suffix) = rest.split_once(']')?;
        let ip = ip.parse::<std::net::Ipv6Addr>().ok()?;
        let port = if suffix.is_empty() {
            443
        } else {
            suffix.strip_prefix(':')?.parse::<u16>().ok()?
        };
        (format!("[{ip}]"), port)
    } else {
        let (host, port) = match value.split_once(':') {
            Some((host, port)) => (host, port.parse::<u16>().ok()?),
            None => (value, 443),
        };
        if host.len() > 253
            || host.split('.').any(|label| {
                label.is_empty()
                    || label.len() > 63
                    || !label
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    || !label
                        .as_bytes()
                        .last()
                        .is_some_and(u8::is_ascii_alphanumeric)
                    || !label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return None;
        }
        (host.to_ascii_lowercase(), port)
    };
    (port != 0).then(|| {
        if port == 443 {
            host
        } else {
            format!("{host}:{port}")
        }
    })
}

impl ServerConfig {
    /// Keep the backend bound to loopback and admit exactly this external HTTPS
    /// origin through an operator-managed local TLS reverse proxy.
    pub fn with_public_origin(mut self, origin: &str) -> Result<Self> {
        self.public_origin = Some(PublicOrigin::parse(origin)?);
        Ok(self)
    }
}

pub(super) fn check_authority(
    request: &Request,
    public: Option<&PublicOrigin>,
    local_port: u16,
) -> std::result::Result<(), &'static str> {
    if let Some(public) = public {
        if request
            .header("host")
            .and_then(canonical_authority)
            .as_deref()
            != Some(public.authority.as_str())
        {
            return Err("invalid_host");
        }
        if let Some(origin) = request.header("origin") {
            let origin = PublicOrigin::parse(origin).map_err(|_| "invalid_origin")?;
            if origin.origin != public.origin {
                return Err("invalid_origin");
            }
        }
        // X-Forwarded-* headers are never trusted for authority decisions.
        return Ok(());
    }
    let host = request
        .header("host")
        .and_then(parse_authority)
        .ok_or("invalid_host")?;
    if host.port != local_port || !is_loopback_name(&host.host) {
        return Err("invalid_host");
    }
    if let Some(origin) = request.header("origin") {
        let origin = parse_origin(origin).ok_or("invalid_origin")?;
        if origin.port != host.port || origin.host != host.host {
            return Err("invalid_origin");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_requires_the_exact_https_host_and_origin() {
        let public = PublicOrigin::parse("https://synara.example:443").unwrap();
        for (host, origin, allowed) in [
            ("synara.example", "https://synara.example", true),
            ("SYNARA.EXAMPLE:443", "https://synara.example", true),
            ("attacker.example", "https://synara.example", false),
            ("synara.example", "http://synara.example", false),
            ("synara.example", "https://attacker.example", false),
            ("synara.example", "https://synara.example:444", false),
        ] {
            let request = super::super::parse_request(format!(
                "POST /api/catalog HTTP/1.1\r\nHost: {host}\r\nOrigin: {origin}\r\nX-Forwarded-Host: synara.example\r\n\r\n"
            ).as_bytes()).unwrap();
            assert_eq!(
                check_authority(&request, Some(&public), 17341).is_ok(),
                allowed
            );
        }
    }

    #[test]
    fn public_origin_rejects_url_ambiguity_and_plaintext() {
        for value in [
            "",
            "http://synara.example",
            "https://user@synara.example",
            "https://synara.example/",
            "https://synara.example?x=1",
            "https://synara.example#x",
            "https://synara.example:0",
            "https://synara..example",
            "https://synara.example\\evil",
        ] {
            assert!(PublicOrigin::parse(value).is_err(), "{value}");
        }
        assert_eq!(
            PublicOrigin::parse("https://[::1]:8443").unwrap().origin,
            "https://[::1]:8443"
        );
    }
}
