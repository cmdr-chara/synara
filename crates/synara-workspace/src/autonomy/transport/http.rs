use super::*;
use std::io::{Read, Write};
fn equal(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (a, b)| diff | (a ^ b)) == 0
}
fn headers(head: &str, host: &str, token: &str) -> Result<usize, u16> {
    let mut lines = head.split("\r\n");
    let request = lines.next().ok_or(400u16)?;
    let mut map = BTreeMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        if line.starts_with([' ', '\t']) {
            return Err(400);
        }
        let (key, value) = line.split_once(':').ok_or(400u16)?;
        if key.is_empty()
            || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
            || value.chars().any(|c| c.is_control() && c != '\t')
        {
            return Err(400);
        }
        if map.insert(key.to_ascii_lowercase(), value.trim()).is_some() {
            return Err(400);
        }
    }
    if map.contains_key("origin") || map.get("host") != Some(&host) {
        return Err(403);
    }
    if !equal(
        map.get("authorization").unwrap_or(&"").as_bytes(),
        format!("Bearer {token}").as_bytes(),
    ) {
        return Err(401);
    }
    if matches!(request, "GET /mcp HTTP/1.1" | "DELETE /mcp HTTP/1.1") {
        return Err(405);
    }
    if request != "POST /mcp HTTP/1.1" {
        return Err(400);
    }
    if ["transfer-encoding", "content-encoding", "upgrade", "expect"]
        .iter()
        .any(|k| map.contains_key(*k))
    {
        return Err(400);
    }
    if !matches!(
        map.get("mcp-protocol-version"),
        None | Some(&"2025-03-26") | Some(&"2025-06-18") | Some(&"2025-11-25")
    ) {
        return Err(400);
    }
    if map
        .get("content-type")
        .is_none_or(|v| v.split(';').next().map(str::trim) != Some("application/json"))
    {
        return Err(415);
    }
    let accept = map.get("accept").ok_or(406u16)?;
    let accepts: Vec<_> = accept
        .split(',')
        .map(|v| v.trim().split(';').next().unwrap_or("").trim())
        .collect();
    if !accepts.contains(&"application/json") || !accepts.contains(&"text/event-stream") {
        return Err(406);
    }
    let length = map.get("content-length").ok_or(411u16)?;
    if length.is_empty() || !length.bytes().all(|c| c.is_ascii_digit()) {
        return Err(400);
    }
    let length = length.parse::<usize>().map_err(|_| 413u16)?;
    if length == 0 || length > 256 * 1024 {
        return Err(413);
    }
    Ok(length)
}
pub(super) fn read(
    socket: &mut TcpStream,
    host: &str,
    token: &str,
    deadline: Instant,
) -> Result<Vec<u8>, u16> {
    let mut bytes = Vec::new();
    let end = loop {
        if let Some(index) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break index + 4;
        }
        if bytes.len() >= 8192 {
            return Err(431);
        }
        socket
            .set_read_timeout(Some(
                deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(408u16)?,
            ))
            .map_err(|_| 400u16)?;
        let mut buffer = [0; 1024];
        let n = socket.read(&mut buffer).map_err(|_| 408u16)?;
        if n == 0 {
            return Err(400);
        }
        bytes.extend_from_slice(&buffer[..n]);
    };
    if end > 8192 {
        return Err(431);
    }
    let count = headers(
        std::str::from_utf8(&bytes[..end]).map_err(|_| 400u16)?,
        host,
        token,
    )?;
    let mut body = bytes[end..].to_vec();
    if body.len() > count {
        return Err(400);
    }
    let mut read = body.len();
    body.resize(count, 0);
    while read < count {
        socket
            .set_read_timeout(Some(
                deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(408u16)?,
            ))
            .map_err(|_| 400u16)?;
        let n = socket.read(&mut body[read..]).map_err(|_| 408u16)?;
        if n == 0 {
            return Err(400);
        }
        read += n;
    }
    Ok(body)
}
pub(super) fn respond(
    socket: &mut TcpStream,
    status: u16,
    body: Option<Value>,
    deadline: Instant,
) -> Result<(), ()> {
    let body = body.map(|v| v.to_string()).unwrap_or_default();
    if body.len() > 4 * 1024 * 1024 {
        return Err(());
    }
    let header = format!(
        "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    for mut bytes in [header.as_bytes(), body.as_bytes()] {
        while !bytes.is_empty() {
            socket
                .set_write_timeout(Some(
                    deadline.checked_duration_since(Instant::now()).ok_or(())?,
                ))
                .map_err(|_| ())?;
            let n = socket.write(bytes).map_err(|_| ())?;
            if n == 0 {
                return Err(());
            }
            bytes = &bytes[n..];
        }
    }
    let _ = socket.shutdown(std::net::Shutdown::Both);
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn head(extra: &str) -> String {
        format!(
            "POST /mcp HTTP/1.1\r\nHost: 127.0.0.1:123\r\nAuthorization: Bearer secret\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nContent-Length: 2\r\n{extra}\r\n"
        )
    }
    #[test]
    fn authority_duplicates_origin_and_smuggling_fail_closed() {
        assert_eq!(headers(&head(""), "127.0.0.1:123", "secret"), Ok(2));
        for extra in [
            "Host: evil\r\n",
            "Origin: null\r\n",
            "Transfer-Encoding: chunked\r\n",
            "Content-Length: 2\r\n",
            "MCP-Protocol-Version: unknown\r\n",
            " folded: nope\r\n",
        ] {
            assert!(headers(&head(extra), "127.0.0.1:123", "secret").is_err());
        }
        assert_eq!(headers(&head(""), "127.0.0.1:123", "wrong"), Err(401));
        assert_eq!(
            headers(
                &head("").replace("Content-Length: 2", "Content-Length: 999999"),
                "127.0.0.1:123",
                "secret"
            ),
            Err(413)
        );
        assert_eq!(
            headers(&head("").replace("POST", "GET"), "127.0.0.1:123", "secret"),
            Err(405)
        );
        assert_eq!(
            headers(
                &head("Origin: https://hostile.test\r\n").replace("POST", "GET"),
                "127.0.0.1:123",
                "secret"
            ),
            Err(403)
        );
    }
}
