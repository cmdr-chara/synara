use super::*;
use std::io::{Read, Write};
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b).fold(0u8, |diff, (a,b)| diff | (a ^ b)) == 0
}
fn headers(head: &str, host: &str, token: &str) -> std::result::Result<usize, u16> {
    let mut lines = head.split("\r\n");
    let request = lines.next().ok_or(400u16)?;
    if matches!(request, "GET /mcp HTTP/1.1" | "DELETE /mcp HTTP/1.1") { return Err(405); }
    if request != "POST /mcp HTTP/1.1" { return Err(400); }
    let mut map = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        if line.starts_with([' ', '\t']) { return Err(400); }
        let (key, value) = line.split_once(':').ok_or(400u16)?;
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') { return Err(400); }
        if map.insert(key.to_ascii_lowercase(), value.trim()).is_some() { return Err(400); }
    }
    if map.contains_key("origin") || map.get("host") != Some(&host) { return Err(403); }
    let expected = format!("Bearer {token}");
    if !constant_time_eq(map.get("authorization").unwrap_or(&"").as_bytes(), expected.as_bytes()) { return Err(401); }
    if ["transfer-encoding","content-encoding","upgrade","expect"].iter().any(|k| map.contains_key(*k)) { return Err(400); }
    if !matches!(map.get("mcp-protocol-version"), None | Some(&"2025-03-26") | Some(&"2025-06-18")) { return Err(400); }
    if map.get("content-type").is_none_or(|v| v.split(';').next() != Some("application/json")) { return Err(415); }
    let accept = map.get("accept").ok_or(406u16)?;
    if !accept.contains("application/json") || !accept.contains("text/event-stream") { return Err(406); }
    let value = map.get("content-length").ok_or(411u16)?;
    if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) { return Err(400); }
    let n = value.parse::<usize>().map_err(|_| 413u16)?;
    if n > 16 * 1024 { return Err(413); }
    Ok(n)
}
pub(super) fn read(socket: &mut TcpStream, host: &str, token: &str, deadline: Instant) -> std::result::Result<Vec<u8>, u16> {
    let mut bytes = Vec::new();
    let end = loop {
        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") { break at + 4; }
        if bytes.len() >= 8192 { return Err(431); }
        let remaining = deadline.checked_duration_since(Instant::now()).ok_or(408u16)?;
        socket.set_read_timeout(Some(remaining)).map_err(|_| 400u16)?;
        let mut buf = [0u8; 1024]; let n = socket.read(&mut buf).map_err(|_| 408u16)?;
        if n == 0 { return Err(400); } bytes.extend_from_slice(&buf[..n]);
    };
    if end > 8192 { return Err(431); }
    let head = std::str::from_utf8(&bytes[..end]).map_err(|_| 400u16)?;
    let n = headers(head, host, token)?;
    let mut body = bytes[end..].to_vec();
    if body.len() > n { return Err(400); }
    let read = body.len(); body.resize(n, 0);
    let mut read = read;
    while read < n {
        socket.set_read_timeout(Some(deadline.checked_duration_since(Instant::now()).ok_or(408u16)?)).map_err(|_| 400u16)?;
        let got = socket.read(&mut body[read..]).map_err(|_| 408u16)?;
        if got == 0 { return Err(400); } read += got;
    }
    Ok(body)
}
pub(super) fn respond(socket: &mut TcpStream, status: u16, body: Option<Value>, deadline: Instant) -> std::result::Result<(), ()> {
    let body = body.map(|v| v.to_string()).unwrap_or_default();
    if body.len() > 256 * 1024 { return Err(()); }
    let head = format!("HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\n\r\n", body.len());
    let bytes = [head.as_bytes(), body.as_bytes()].concat(); let mut written = 0;
    while written < bytes.len() {
        socket.set_write_timeout(Some(deadline.checked_duration_since(Instant::now()).ok_or(())?)).map_err(|_| ())?;
        let n = socket.write(&bytes[written..]).map_err(|_| ())?;
        if n == 0 { return Err(()); } written += n;
    }
    socket.shutdown(std::net::Shutdown::Both).map_err(|_| ())
}
#[cfg(test)] mod tests {
    use super::*;
    fn head(extra: &str) -> String { format!("POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:123\r\nAuthorization: Bearer token\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: 2\r\n{extra}\r\n") }
    #[test] fn strict_http_authority_and_bounds() {
        assert_eq!(headers(&head(""), "127.0.0.1:123", "token"), Ok(2));
        for extra in ["Origin: null\r\n", "Origin: https://evil.test\r\n", "Transfer-Encoding: chunked\r\n", "Host: evil.test\r\n", "Content-Length: 2\r\n", "MCP-Protocol-Version: unsupported\r\n"] {
            assert!(headers(&head(extra), "127.0.0.1:123", "token").is_err());
        }
        assert!(headers(&head(""), "evil.test", "token").is_err());
        assert!(headers(&head(""), "127.0.0.1:123", "wrong").is_err());
        assert_eq!(headers(&head("").replace("Content-Length: 2", "Content-Length: 17000"), "127.0.0.1:123", "token"), Err(413));
    }
    #[test] fn constant_time_comparison_requires_exact_token() { assert!(constant_time_eq(b"abc", b"abc")); assert!(!constant_time_eq(b"abc", b"abd")); assert!(!constant_time_eq(b"ab", b"abc")); }
}
