// V8 JSON.parse error-message emulation for inputs serde rejects.

#[rustfmt::skip]
pub fn v8_err(value: &str) -> String {
    let u: Vec<u16> = value.encode_utf16().collect();
    let ws = |u: u16| matches!(u, 0x20 | 0x09 | 0x0A | 0x0D);
    let i = (0..u.len()).find(|&i| !ws(u[i])).unwrap_or(u.len());
    if i >= u.len() {
        return "Unexpected end of JSON input".into();
    }
    // failure position: where the first parse error occurs, in UTF-16 units
    let pos = match u[i] {
        c if c == '{' as u16 => {
            let j = (i + 1..=u.len()).find(|&j| j == u.len() || !ws(u[j])).unwrap();
            return match u.get(j) {
                Some(&c) if c == '"' as u16 || c == '}' as u16 => "Unexpected end of JSON input".into(),
                Some(_) => {
                    let (l, col) = line_col(&u, j);
                    format!("Expected property name or '}}' in JSON at position {j} (line {l} column {col})")
                }
                None => "Unexpected end of JSON input".into(),
            };
        }
        c if c == 't' as u16 || c == 'f' as u16 || c == 'n' as u16 => {
            let lit: &[u8] = if u[i] == 't' as u16 { b"true" } else if u[i] == 'f' as u16 { b"false" } else { b"null" };
            match (0..lit.len()).find(|&k| u.get(i + k).copied() != Some(lit[k] as u16)) {
                Some(k) => i + k,
                None => {
                    let j = (i + lit.len()..=u.len()).find(|&j| j == u.len() || !ws(u[j])).unwrap();
                    return if j >= u.len() {
                        "Unexpected end of JSON input".into()
                    } else {
                        let (l, col) = line_col(&u, j);
                        format!("Unexpected non-whitespace character after JSON at position {j} (line {l} column {col})")
                    };
                }
            }
        }
        _ => i,
    };
    let ch = u[pos];
    let esc = if (0xD800..=0xDFFF).contains(&ch) || ch == 0xFEFF || ch < 0x20 || (0x7F..=0x9F).contains(&ch) {
        format!("\\u{ch:04x}")
    } else {
        char::from_u32(ch as u32).map(|c| c.to_string()).unwrap_or_default()
    };
    let ctx = if u.len() <= 20 {
        String::from_utf16_lossy(&u)
    } else {
        let end = (pos + 10).min(u.len());
        format!("{}...", String::from_utf16_lossy(&u[..end]))
    };
    format!("Unexpected token '{esc}', \"{ctx}\" is not valid JSON")
}

fn line_col(u: &[u16], pos: usize) -> (usize, usize) {
    let (mut l, mut c) = (1usize, 1usize);
    for &x in u.iter().take(pos) {
        if x == '\n' as u16 {
            l += 1;
            c = 1;
        } else {
            c += 1;
        }
    }
    (l, c)
}
