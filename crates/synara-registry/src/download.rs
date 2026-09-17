use crate::{RegistryError, Result, model::https_url};
use std::{
    io::{Read, Write},
    net::IpAddr,
    time::{Duration, Instant},
};
use ureq::{
    config::Config,
    unversioned::{
        resolver::{DefaultResolver, ResolvedSocketAddrs, Resolver},
        transport::{DefaultConnector, NextTimeout},
    },
};

/// Downloads bytes only. Registry contents are never executed by this boundary.
pub trait Downloader: Send + Sync {
    fn download(&self, url: &str, destination: &mut dyn Write, limit: u64) -> Result<()>;
}
#[derive(Clone)]
pub struct HttpsDownloader {
    agent: ureq::Agent,
}
impl Default for HttpsDownloader {
    fn default() -> Self {
        let config = ureq::Agent::config_builder()
            .https_only(true)
            .proxy(None)
            .max_redirects(0)
            .timeout_global(Some(Duration::from_secs(60)))
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_resolve(Some(Duration::from_secs(10)))
            .max_response_header_size(32 * 1024)
            .build();
        Self {
            agent: ureq::Agent::with_parts(config, DefaultConnector::default(), PublicResolver),
        }
    }
}
impl Downloader for HttpsDownloader {
    fn download(&self, uri: &str, destination: &mut dyn Write, limit: u64) -> Result<()> {
        let mut url = https_url(uri)?;
        let start = Instant::now();
        for _ in 0..6 {
            if start.elapsed() > Duration::from_secs(120) {
                return Err(RegistryError::Network("redirect chain timed out".into()));
            }
            // Resolve and validate every redirect through the same transport. Environment
            // proxies are intentionally disabled so they cannot bypass the address policy.
            let mut response = self.agent.get(url.as_str()).call().map_err(|_| {
                RegistryError::Network("connection, TLS, timeout or HTTP status error".into())
            })?;
            if response.status().is_redirection() {
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|h| h.to_str().ok())
                    .ok_or_else(|| {
                        RegistryError::Network("redirect has no valid Location".into())
                    })?;
                url = https_url(
                    url.join(location)
                        .map_err(|_| RegistryError::Network("invalid redirect".into()))?
                        .as_str(),
                )?;
                continue;
            }
            if response.status().as_u16() != 200 {
                return Err(RegistryError::Network(
                    "expected a complete HTTP 200 response".into(),
                ));
            }
            if response
                .headers()
                .get("content-length")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.parse::<u64>().ok())
                .is_some_and(|n| n > limit)
            {
                return Err(RegistryError::Limit);
            }
            copy_limited(response.body_mut().as_reader(), destination, limit)?;
            return Ok(());
        }
        Err(RegistryError::Network("too many redirects".into()))
    }
}
#[derive(Debug)]
struct PublicResolver;
impl Resolver for PublicResolver {
    fn resolve(
        &self,
        uri: &ureq::http::Uri,
        config: &Config,
        timeout: NextTimeout,
    ) -> std::result::Result<ResolvedSocketAddrs, ureq::Error> {
        if https_url(&uri.to_string()).is_err() {
            return Err(ureq::Error::HostNotFound);
        }
        let addresses = DefaultResolver::default().resolve(uri, config, timeout)?;
        if addresses
            .iter()
            .any(|a| !public_address(a.ip()) || a.port() != 443)
        {
            return Err(ureq::Error::HostNotFound);
        }
        Ok(addresses)
    }
}
fn public_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_documentation()
                && a != 0
                && a < 224
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && (b == 0 && c == 0 || b == 88 && c == 99))
                && !(a == 198 && (b == 18 || b == 19))
        }
        IpAddr::V6(ip) => {
            let s = ip.segments();
            // Only global unicast. Exclude mapped/translation/tunnel, benchmark and
            // documentation spaces instead of letting them reach private IPv4 hosts.
            (s[0] & 0xe000) == 0x2000
                && s[0] != 0x2002
                && s[0] != 0x3fff
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
        }
    }
}
pub(crate) fn copy_limited(
    mut source: impl Read,
    destination: &mut dyn Write,
    limit: u64,
) -> Result<u64> {
    let mut copied = 0u64;
    let mut buffer = [0; 32 * 1024];
    loop {
        let length = source.read(&mut buffer)?;
        if length == 0 {
            return Ok(copied);
        }
        copied = copied
            .checked_add(length as u64)
            .ok_or(RegistryError::Limit)?;
        if copied > limit {
            return Err(RegistryError::Limit);
        }
        destination.write_all(&buffer[..length])?;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn private_and_special_addresses_cannot_be_download_targets() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "172.16.0.2",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "198.18.1.1",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "192.0.2.1",
            "::1",
            "::ffff:127.0.0.1",
            "fd00::1",
            "fe80::1",
            "2002:a00:1::",
            "2001:db8::1",
            "64:ff9b::a00:1",
        ] {
            assert!(!public_address(ip.parse().unwrap()), "{ip}");
        }
        for ip in [
            "1.1.1.1",
            "8.8.8.8",
            "2606:4700:4700::1111",
            "2001:4860:4860::8888",
        ] {
            assert!(public_address(ip.parse().unwrap()));
        }
    }
    #[test]
    fn unsafe_redirect_urls_are_rejected_before_network_io() {
        for url in [
            "http://example.com/a",
            "https://user:pass@example.com/a",
            "https://example.com:8443/a",
            "file:///tmp/a",
            "https://example.com/a#fragment",
        ] {
            assert!(https_url(url).is_err());
        }
    }
    #[test]
    fn byte_limits_cover_streams_without_a_content_length() {
        let mut out = Vec::new();
        assert!(copy_limited(&b"abcd"[..], &mut out, 3).is_err());
        assert!(out.is_empty());
        assert_eq!(copy_limited(&b"abc"[..], &mut out, 3).unwrap(), 3);
    }
}
