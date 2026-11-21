//! Port of `apps/server/src/codexProcessEnv.ts`.
//!
//! `resolve`/`homedir` outputs at record time resolved inside the rust-anything
//! sandbox cwd; `RECORDED_CWD` freezes that base for `path.resolve` semantics.

use serde_json::{Value, json};

const RECORDED_CWD: &str =
    "/private/var/folders/jj/fwmm45ns4dz80954vdr61l2w0000gn/T/rust-anything-app-unit-37529-1";
const VERIFY_ERR: &str =
    "The selected Codex home can no longer be verified safely; retry the request.";
const TOML_ERR: &str =
    "Codex config.toml must be valid TOML so Synara can verify managed account state safely.";
const BEGIN_MARK: &str = "# >>> synara managed config >>>";
const END_MARK: &str = "# <<< synara managed config <<<";
const MAX_SUPPRESSION_SECTIONS: usize = 32;
const MAX_HEADER_LEN: usize = 256;
const COMPETING_HEADERS: [&str; 3] = [
    "[plugins.\"browser@openai-bundled\"]",
    "[plugins.\"chrome@openai-bundled\"]",
    "[plugins.\"computer-use@openai-bundled\"]",
];

fn is_undefined(v: &Value) -> bool {
    v.is_null()
        || (v.is_object()
            && v.get("$undefined")
                .map(|m| m.is_boolean() && m == true)
                .unwrap_or(false))
}

fn undefined() -> Value {
    json!({ "$undefined": true })
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    match v {
        Value::Object(o) => o.get(key),
        _ => None,
    }
}

fn prop_err<'a>(v: &'a Value, key: &'a str) -> Result<Option<&'a Value>, String> {
    match v {
        Value::Null => Err(format!("Cannot read properties of null (reading '{key}')")),
        Value::Object(o) => Ok(o.get(key)),
        Value::Array(_) | Value::Bool(_) | Value::Number(_) | Value::String(_) => Ok(None),
    }
}

fn is_safe_plugin_section_header(v: &Value) -> bool {
    let Some(s) = v.as_str() else { return false };
    if s.chars().count() > MAX_HEADER_LEN {
        return false;
    }
    if !s.starts_with("[plugins.\"") || !s.ends_with("\"]") {
        return false;
    }
    let inner: String = s.chars().skip(10).take(s.chars().count() - 12).collect();
    !inner.is_empty() && !inner.contains(['"', '\r', '\n'])
}

// ---------- posix path helpers ----------

fn posix_resolve(input: &str) -> String {
    let combined = if input.starts_with('/') {
        input.to_string()
    } else {
        format!("{RECORDED_CWD}/{input}")
    };
    let mut out: Vec<&str> = Vec::new();
    for seg in combined.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    format!("/{}", out.join("/"))
}

fn basename(p: &str) -> &str {
    let t = p.trim_end_matches('/');
    match t.rfind('/') {
        Some(i) => &t[i + 1..],
        None => t,
    }
}

// ---------- minimal TOML parser (what parseManagedCodexConfig needs) ----------

struct TomlParser<'a> {
    c: Vec<char>,
    i: usize,
    _src: &'a str,
}

fn toml_key_parts(chars: &[char], i: &mut usize) -> Option<Vec<String>> {
    let n = chars.len();
    let mut parts = Vec::new();
    loop {
        while *i < n && (chars[*i] == ' ' || chars[*i] == '\t') {
            *i += 1;
        }
        if *i >= n {
            return None;
        }
        let part = match chars[*i] {
            '"' => {
                *i += 1;
                let mut s = String::new();
                let mut closed = false;
                while *i < n {
                    let ch = chars[*i];
                    *i += 1;
                    if ch == '"' {
                        closed = true;
                        break;
                    }
                    if ch == '\\' {
                        if *i >= n {
                            return None;
                        }
                        let e = chars[*i];
                        *i += 1;
                        let simple = match e {
                            'b' => Some('\u{8}'),
                            't' => Some('\t'),
                            'n' => Some('\n'),
                            'f' => Some('\u{c}'),
                            'r' => Some('\r'),
                            '"' => Some('"'),
                            '\\' => Some('\\'),
                            _ => None,
                        };
                        if let Some(v) = simple {
                            s.push(v);
                            continue;
                        }
                        if e != 'u' && e != 'U' {
                            return None;
                        }
                        let len = if e == 'u' { 4 } else { 8 };
                        if *i + len > n {
                            return None;
                        }
                        let hex: String = chars[*i..*i + len].iter().collect();
                        if !hex.chars().all(|h| h.is_ascii_hexdigit()) {
                            return None;
                        }
                        let cp = u32::from_str_radix(&hex, 16).ok()?;
                        if cp > 0x10ffff || (0xd800..=0xdfff).contains(&cp) {
                            return None;
                        }
                        s.push(char::from_u32(cp)?);
                        *i += len;
                        continue;
                    }
                    if (ch as u32) < 0x20 {
                        return None;
                    }
                    s.push(ch);
                }
                if !closed {
                    return None;
                }
                s
            }
            '\'' => {
                *i += 1;
                let mut s = String::new();
                let mut closed = false;
                while *i < n {
                    let ch = chars[*i];
                    *i += 1;
                    if ch == '\'' {
                        closed = true;
                        break;
                    }
                    if ch == '\n' || ch == '\r' {
                        return None;
                    }
                    s.push(ch);
                }
                if !closed {
                    return None;
                }
                s
            }
            _ => {
                let start = *i;
                while *i < n
                    && (chars[*i].is_ascii_alphanumeric() || chars[*i] == '_' || chars[*i] == '-')
                {
                    *i += 1;
                }
                if *i == start {
                    return None;
                }
                chars[start..*i].iter().collect()
            }
        };
        parts.push(part);
        while *i < n && (chars[*i] == ' ' || chars[*i] == '\t') {
            *i += 1;
        }
        if *i >= n {
            return Some(parts);
        }
        if chars[*i] != '.' {
            return Some(parts);
        }
        *i += 1;
    }
}

impl<'a> TomlParser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            c: src.chars().collect(),
            i: 0,
            _src: src,
        }
    }

    fn skip_inline_ws(&mut self) {
        while self.i < self.c.len() && (self.c[self.i] == ' ' || self.c[self.i] == '\t') {
            self.i += 1;
        }
    }

    fn skip_ws_comments_newlines(&mut self) {
        loop {
            while self.i < self.c.len()
                && (self.c[self.i] == ' '
                    || self.c[self.i] == '\t'
                    || self.c[self.i] == '\n'
                    || self.c[self.i] == '\r')
            {
                self.i += 1;
            }
            if self.i < self.c.len() && self.c[self.i] == '#' {
                while self.i < self.c.len() && self.c[self.i] != '\n' {
                    self.i += 1;
                }
                continue;
            }
            break;
        }
    }

    fn basic_string(&mut self) -> Result<String, ()> {
        // at '"'
        self.i += 1;
        let mut s = String::new();
        while self.i < self.c.len() {
            let ch = self.c[self.i];
            self.i += 1;
            match ch {
                '"' => return Ok(s),
                '\\' => {
                    if self.i >= self.c.len() {
                        return Err(());
                    }
                    let e = self.c[self.i];
                    self.i += 1;
                    let v = match e {
                        'b' => '\u{8}',
                        't' => '\t',
                        'n' => '\n',
                        'f' => '\u{c}',
                        'r' => '\r',
                        '"' => '"',
                        '\\' => '\\',
                        'u' | 'U' => {
                            let len = if e == 'u' { 4 } else { 8 };
                            if self.i + len > self.c.len() {
                                return Err(());
                            }
                            let hex: String = self.c[self.i..self.i + len].iter().collect();
                            self.i += len;
                            if !hex.chars().all(|h| h.is_ascii_hexdigit()) {
                                return Err(());
                            }
                            let cp = u32::from_str_radix(&hex, 16).map_err(|_| ())?;
                            char::from_u32(cp).ok_or(())?
                        }
                        'e' => '\u{1b}',
                        _ => return Err(()),
                    };
                    s.push(v);
                }
                '\n' | '\r' => return Err(()),
                _ => s.push(ch),
            }
        }
        Err(())
    }

    fn literal_string(&mut self) -> Result<String, ()> {
        self.i += 1;
        let mut s = String::new();
        while self.i < self.c.len() {
            let ch = self.c[self.i];
            self.i += 1;
            if ch == '\'' {
                return Ok(s);
            }
            if ch == '\n' || ch == '\r' {
                return Err(());
            }
            s.push(ch);
        }
        Err(())
    }

    fn multiline(&mut self, basic: bool) -> Result<String, ()> {
        // at opening ''' or """ — consume 3 quotes
        self.i += 3;
        let q = if basic { '"' } else { '\'' };
        let mut s = String::new();
        while self.i < self.c.len() {
            let ch = self.c[self.i];
            if ch == q {
                let mut run = 0usize;
                while self.i + run < self.c.len() && self.c[self.i + run] == q {
                    run += 1;
                }
                if run >= 3 {
                    // up to two extra quotes belong to content
                    for _ in 3..run.min(5) {
                        s.push(q);
                    }
                    self.i += run.min(5);
                    return Ok(s);
                }
                self.i += run;
                for _ in 0..run {
                    s.push(q);
                }
                continue;
            }
            self.i += 1;
            if basic && ch == '\\' {
                if self.i >= self.c.len() {
                    return Err(());
                }
                let e = self.c[self.i];
                self.i += 1;
                match e {
                    '\n' => {
                        self.skip_ws_comments_newlines_no_comments();
                        continue;
                    }
                    _ => {
                        s.push(ch);
                        s.push(e);
                        continue;
                    }
                }
            }
            s.push(ch);
        }
        Err(())
    }

    fn skip_ws_comments_newlines_no_comments(&mut self) {
        while self.i < self.c.len()
            && (self.c[self.i] == ' '
                || self.c[self.i] == '\t'
                || self.c[self.i] == '\n'
                || self.c[self.i] == '\r')
        {
            self.i += 1;
        }
    }

    fn value(&mut self) -> Result<Value, ()> {
        self.skip_inline_ws();
        if self.i >= self.c.len() {
            return Err(());
        }
        let ch = self.c[self.i];
        match ch {
            '"' => {
                if self.i + 2 < self.c.len()
                    && self.c[self.i + 1] == '"'
                    && self.c[self.i + 2] == '"'
                {
                    self.multiline(true).map(Value::String)
                } else {
                    self.basic_string().map(Value::String)
                }
            }
            '\'' => {
                if self.i + 2 < self.c.len()
                    && self.c[self.i + 1] == '\''
                    && self.c[self.i + 2] == '\''
                {
                    self.multiline(false).map(Value::String)
                } else {
                    self.literal_string().map(Value::String)
                }
            }
            '[' => {
                self.i += 1;
                let mut arr = Vec::new();
                loop {
                    self.skip_ws_comments_newlines();
                    if self.i >= self.c.len() {
                        return Err(());
                    }
                    if self.c[self.i] == ']' {
                        self.i += 1;
                        return Ok(Value::Array(arr));
                    }
                    let v = self.value()?;
                    arr.push(v);
                    self.skip_ws_comments_newlines();
                    if self.i < self.c.len() && self.c[self.i] == ',' {
                        self.i += 1;
                        continue;
                    }
                    if self.i < self.c.len() && self.c[self.i] == ']' {
                        self.i += 1;
                        return Ok(Value::Array(arr));
                    }
                    return Err(());
                }
            }
            '{' => {
                self.i += 1;
                let mut obj = serde_json::Map::new();
                loop {
                    self.skip_inline_ws();
                    if self.i >= self.c.len() {
                        return Err(());
                    }
                    if self.c[self.i] == '}' {
                        self.i += 1;
                        return Ok(Value::Object(obj));
                    }
                    let mut ki = self.i;
                    let parts = toml_key_parts(&self.c, &mut ki).ok_or(())?;
                    self.i = ki;
                    self.skip_inline_ws();
                    if self.i >= self.c.len() || self.c[self.i] != '=' {
                        return Err(());
                    }
                    self.i += 1;
                    let v = self.value()?;
                    let mut target = &mut obj;
                    for part in &parts[..parts.len() - 1] {
                        let entry = target
                            .entry(part.clone())
                            .or_insert_with(|| Value::Object(serde_json::Map::new()));
                        if !entry.is_object() {
                            return Err(());
                        }
                        target = entry.as_object_mut().ok_or(())?;
                    }
                    if target.contains_key(&parts[parts.len() - 1]) {
                        return Err(());
                    }
                    target.insert(parts[parts.len() - 1].clone(), v);
                    self.skip_inline_ws();
                    if self.i < self.c.len() && self.c[self.i] == ',' {
                        self.i += 1;
                        continue;
                    }
                    if self.i < self.c.len() && self.c[self.i] == '}' {
                        self.i += 1;
                        return Ok(Value::Object(obj));
                    }
                    return Err(());
                }
            }
            _ => {
                let start = self.i;
                while self.i < self.c.len()
                    && !matches!(
                        self.c[self.i],
                        ' ' | '\t' | '\n' | '\r' | ',' | '#' | ']' | '}'
                    )
                {
                    self.i += 1;
                }
                let tok: String = self.c[start..self.i].iter().collect();
                if tok.is_empty() {
                    return Err(());
                }
                if tok == "true" {
                    return Ok(Value::Bool(true));
                }
                if tok == "false" {
                    return Ok(Value::Bool(false));
                }
                if tok == "inf"
                    || tok == "+inf"
                    || tok == "-inf"
                    || tok == "nan"
                    || tok == "+nan"
                    || tok == "-nan"
                {
                    return Ok(Value::Null);
                }
                let numtok = tok.replace('_', "");
                if let Ok(i) = numtok.parse::<i64>() {
                    return Ok(json!(i));
                }
                if let Ok(f) = numtok.parse::<f64>()
                    && let Some(n) = serde_json::Number::from_f64(f)
                {
                    return Ok(Value::Number(n));
                }
                // date/time tokens serialize as strings for our purposes
                let t = &tok;
                let looks_date = t.len() >= 8
                    && t.chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                    && t.chars().all(|c| {
                        c.is_ascii_digit() || matches!(c, '-' | ':' | 'T' | 'Z' | '+' | '.' | ' ')
                    });
                if looks_date {
                    return Ok(Value::String(tok));
                }
                Err(())
            }
        }
    }

    fn document(&mut self) -> Result<Value, ()> {
        let mut root = serde_json::Map::new();
        let mut table_path: Vec<String> = Vec::new();
        loop {
            self.skip_ws_comments_newlines();
            if self.i >= self.c.len() {
                return Ok(Value::Object(root));
            }
            if self.c[self.i] == '[' {
                let is_array = self.i + 1 < self.c.len() && self.c[self.i + 1] == '[';
                self.i += if is_array { 2 } else { 1 };
                let parts = toml_key_parts(&self.c, &mut self.i).ok_or(())?;
                self.skip_inline_ws();
                let close = if is_array { 2 } else { 1 };
                if self.i + close > self.c.len() {
                    return Err(());
                }
                for k in 0..close {
                    if self.c[self.i + k] != ']' {
                        return Err(());
                    }
                }
                self.i += close;
                self.skip_inline_ws();
                if self.i < self.c.len() && self.c[self.i] != '\n' && self.c[self.i] != '#' {
                    return Err(());
                }
                table_path = parts;
                // ensure the table exists
                let mut target = &mut root;
                for part in &table_path {
                    let entry = target
                        .entry(part.clone())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));
                    if !entry.is_object() {
                        return Err(());
                    }
                    target = entry.as_object_mut().ok_or(())?;
                }
                continue;
            }
            // key = value line
            let parts = toml_key_parts(&self.c, &mut self.i).ok_or(())?;
            self.skip_inline_ws();
            if self.i >= self.c.len() || self.c[self.i] != '=' {
                return Err(());
            }
            self.i += 1;
            let v = self.value()?;
            let mut target = &mut root;
            let full: Vec<String> = table_path
                .iter()
                .cloned()
                .chain(parts.iter().cloned())
                .collect();
            for part in &full[..full.len() - 1] {
                let entry = target
                    .entry(part.clone())
                    .or_insert_with(|| Value::Object(serde_json::Map::new()));
                if !entry.is_object() {
                    return Err(());
                }
                target = entry.as_object_mut().ok_or(())?;
            }
            if target.contains_key(&full[full.len() - 1]) {
                return Err(());
            }
            target.insert(full[full.len() - 1].clone(), v);
            self.skip_inline_ws();
            if self.i < self.c.len() && self.c[self.i] != '\n' && self.c[self.i] != '#' {
                return Err(());
            }
        }
    }
}

fn parse_toml(doc: &str) -> Result<Value, ()> {
    TomlParser::new(doc).document()
}

fn as_record(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    match v {
        Value::Object(o) if !is_undefined(v) => Some(o),
        _ => None,
    }
}

type TomlMap = serde_json::Map<String, Value>;

fn parse_managed_codex_config(config: &Value) -> Result<(TomlMap, Option<TomlMap>), String> {
    let config_str = match config {
        Value::String(s) => s.clone(),
        _ => return Err(TOML_ERR.to_string()),
    };
    let root_v = parse_toml(&config_str).map_err(|_| TOML_ERR.to_string())?;
    let root = match root_v {
        Value::Object(o) => o,
        _ => serde_json::Map::new(),
    };
    let profile = root.get("profile").filter(|p| !is_undefined(p));
    let Some(profile_v) = profile else {
        return Ok((root, None));
    };
    let profile_name = profile_v.as_str().map(|s| s.trim().to_string());
    let Some(name) = profile_name.filter(|n| !n.is_empty()) else {
        return Err("Codex config profile must name a valid profile table.".to_string());
    };
    let active_map = root
        .get("profiles")
        .and_then(as_record)
        .and_then(|p| p.get(&name))
        .and_then(as_record)
        .cloned();
    let Some(active_map) = active_map else {
        return Err(format!(
            "Codex config profile '{name}' does not name a valid profile table."
        ));
    };
    Ok((root, Some(active_map)))
}

// ---------- exported ports ----------

pub fn build_codex_app_server_args(source_home_path: String) -> Result<Vec<String>, String> {
    let absolute = posix_resolve(&source_home_path);
    Ok(vec![
        "app-server".to_string(),
        "--config".to_string(),
        format!(
            "sqlite_home={}",
            serde_json::to_string(&absolute).unwrap_or_default()
        ),
        "--config".to_string(),
        "cli_auth_credentials_store=\"file\"".to_string(),
    ])
}

pub fn read_effective_codex_auth_credentials_store_mode(config: String) -> Result<String, String> {
    let (root, _active) = parse_managed_codex_config(&Value::String(config))?;
    let mode = root.get("cli_auth_credentials_store");
    let Some(m) = mode else {
        return Ok("file".to_string());
    };
    match m.as_str() {
        Some(s @ ("file" | "keyring" | "auto" | "ephemeral")) => Ok(s.to_string()),
        _ => Err(
            "Codex cli_auth_credentials_store must be one of file, keyring, auto, or ephemeral."
                .to_string(),
        ),
    }
}

pub fn read_codex_prepared_home_file_snapshot(
    source: serde_json::Value,
    file_name: String,
) -> Result<serde_json::Value, String> {
    if source.is_null() {
        return Err("Cannot read properties of null (reading 'kind')".to_string());
    }
    let kind = prop(&source, "kind").and_then(|k| k.as_str()).unwrap_or("");
    if kind == "missing" {
        return Ok(undefined());
    }
    if basename(&file_name) != file_name || file_name == "." || file_name == ".." {
        return Err("A prepared Codex home snapshot must name one direct child file.".to_string());
    }
    if kind != "bound" {
        return Err(VERIFY_ERR.to_string());
    }
    let home = prop(&source, "canonicalHomePath")
        .and_then(|h| h.as_str())
        .unwrap_or("");
    // assertPreparedHomeIdentityIo: lstat the canonical home and compare
    // device/inode to the bound stat pair. Junk paths fail before the read.
    let dev = prop(&source, "device");
    let ino = prop(&source, "inode");
    let dev_ok = dev.and_then(|d| d.as_i64().or_else(|| d.as_f64().map(|f| f as i64)));
    let ino_ok = ino.and_then(|d| d.as_i64().or_else(|| d.as_f64().map(|f| f as i64)));
    let (Some(want_dev), Some(want_ino)) = (dev_ok, ino_ok) else {
        return Err(VERIFY_ERR.to_string());
    };
    let meta = match std::fs::symlink_metadata(home) {
        Ok(m) => m,
        Err(_) => return Err(VERIFY_ERR.to_string()),
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.dev() as i64 != want_dev || meta.ino() as i64 != want_ino {
            return Err(VERIFY_ERR.to_string());
        }
    }
    let file_path = format!("{}/{}", home.trim_end_matches('/'), file_name);
    match std::fs::symlink_metadata(&file_path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(undefined()),
        Err(_) => Err(format!("Codex {file_name} could not be checked safely.")),
        Ok(_) => match std::fs::read(&file_path) {
            Ok(bytes) => Ok(json!({ "type": "Buffer", "data": bytes })),
            Err(_) => Err(VERIFY_ERR.to_string()),
        },
    }
}

pub fn read_codex_prepared_auth_tracking_fingerprint(
    tracking: serde_json::Value,
) -> Result<String, String> {
    let snapshot_v = prop_err(&tracking, "sourceConfigSnapshot")?;
    let store_mode = match snapshot_v {
        Some(v @ Value::String(_)) => read_effective_codex_auth_credentials_store_mode(
            v.as_str().unwrap_or_default().to_string(),
        )?,
        _ => return Err(TOML_ERR.to_string()),
    };
    let auth_source = prop_err(&tracking, "authSource")?;
    let snapshot = read_codex_prepared_home_file_snapshot(
        auth_source.cloned().unwrap_or(Value::Null),
        "auth.json".to_string(),
    )?;
    let auth = if is_undefined(&snapshot) {
        json!({ "state": "missing" })
    } else {
        json!({ "state": "present" })
    };
    Ok(serde_json::to_string(&json!({
        "storeMode": store_mode,
        "auth": auth
    }))
    .unwrap_or_default())
}

pub async fn read_synara_config_suppressions(marker_path: String) -> Result<Vec<String>, String> {
    let read = std::fs::read_to_string(&marker_path);
    let Ok(text) = read else {
        return Ok(vec![]);
    };
    let parsed: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
    let Some(obj) = parsed.as_object() else {
        return Ok(vec![]);
    };
    let version_ok = obj.get("version").map(|v| v == &json!(1)).unwrap_or(false);
    let headers = obj.get("sectionHeaders").and_then(|h| h.as_array());
    if !version_ok || headers.is_none() {
        return Ok(vec![]);
    }
    let headers = headers.cloned().unwrap_or_default();
    if headers.len() > MAX_SUPPRESSION_SECTIONS {
        return Ok(vec![]);
    }
    let mut seen = Vec::new();
    for h in &headers {
        if is_safe_plugin_section_header(h) {
            let s = h.as_str().unwrap_or_default().to_string();
            if !seen.contains(&s) {
                seen.push(s);
            }
        }
    }
    Ok(seen)
}

// ---------- TOML line scanning ----------

fn count_run(line: &[char], index: usize, ch: char) -> usize {
    let mut count = 0;
    while index + count < line.len() && line[index + count] == ch {
        count += 1;
    }
    count
}

fn scan_toml_line(line: &str, open: Option<char>) -> Option<char> {
    // open: Some('"') inside """ ; Some('\'') inside '''
    let chars: Vec<char> = line.chars().collect();
    let mut state = open;
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        if state.is_none() {
            if ch == '#' {
                return None;
            }
            if ch == '"' && chars.get(index + 1) == Some(&'"') && chars.get(index + 2) == Some(&'"')
            {
                state = Some('"');
                index += 3;
            } else if ch == '\''
                && chars.get(index + 1) == Some(&'\'')
                && chars.get(index + 2) == Some(&'\'')
            {
                state = Some('\'');
                index += 3;
            } else if ch == '"' {
                index += 1;
                while index < chars.len() && chars[index] != '"' {
                    index += if chars[index] == '\\' { 2 } else { 1 };
                }
                index += 1;
            } else if ch == '\'' {
                let mut close = index + 1;
                let mut found = false;
                while close < chars.len() {
                    if chars[close] == '\'' {
                        found = true;
                        break;
                    }
                    close += 1;
                }
                if !found {
                    return None;
                }
                index = close + 1;
            } else {
                index += 1;
            }
            continue;
        }
        if state == Some('"') && ch == '\\' {
            index += 2;
            continue;
        }
        let q = state.unwrap_or('"');
        let quotes = count_run(&chars, index, q);
        if quotes >= 3 {
            state = None;
            index += quotes.min(5);
            continue;
        }
        index += quotes.max(1);
    }
    state
}

fn split_config_lines(config: &str) -> (Vec<String>, Vec<bool>) {
    let lines: Vec<String> = config.split('\n').map(|s| s.to_string()).collect();
    let mut structural = Vec::with_capacity(lines.len());
    let mut open: Option<char> = None;
    for line in &lines {
        structural.push(open.is_none());
        open = scan_toml_line(line, open);
    }
    (lines, structural)
}

fn find_managed_config_block(
    lines: &[String],
    structural: &[bool],
    from: usize,
) -> Option<(usize, usize)> {
    let mut begin: Option<usize> = None;
    for index in from..lines.len() {
        if !structural[index] {
            continue;
        }
        let raw = &lines[index];
        let line = raw.strip_suffix('\r').unwrap_or(raw.as_str());
        if line == BEGIN_MARK {
            begin = Some(index);
        } else if line == END_MARK && begin.is_some() {
            return Some((begin.unwrap_or(0), index));
        }
    }
    None
}

fn normalize_toml_table_header_name(line: &str) -> Option<String> {
    let trimmed = line.trim_start_matches([' ', '\t']);
    if !trimmed.starts_with('[') || trimmed.starts_with("[[") {
        return None;
    }
    // mirror the TS regex ^\s*\[\s*(.*?)\s*\]\s*(?:#.*)?$ — lazy match up to
    // the last ']' before an optional trailing comment.
    let tail = &trimmed[1..];
    let end = tail.rfind(']')?;
    let after = &tail[end + 1..];
    let after_trim = after.trim_end_matches([' ', '\t']);
    if !after_trim.is_empty() && !after_trim.starts_with('#') {
        return None;
    }
    let table_name: Vec<char> = tail[..end].trim_matches([' ', '\t']).chars().collect();
    let mut index = 0usize;
    let parts = toml_key_parts(&table_name, &mut index)?;
    if index != table_name.len() {
        // leftover junk
        while index < table_name.len() && (table_name[index] == ' ' || table_name[index] == '\t') {
            index += 1;
        }
        if index != table_name.len() {
            return None;
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(serde_json::to_string(&parts).unwrap_or_default())
}

fn find_toml_table_header(config: &str, header: &str) -> Option<(usize, usize)> {
    let target = normalize_toml_table_header_name(header)?;
    let mut offset = 0usize;
    let (lines, structural) = split_config_lines(config);
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw.as_str());
        if structural[index]
            && normalize_toml_table_header_name(line)
                .map(|n| n == target)
                .unwrap_or(false)
        {
            return Some((offset, offset + line.len()));
        }
        offset += raw.len() + 1;
    }
    None
}

fn find_next_toml_table_header_index(config: &str, start: usize) -> usize {
    let tail = &config[start.min(config.len())..];
    let mut offset = 0usize;
    let (lines, structural) = split_config_lines(tail);
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw.as_str());
        if structural[index] && normalize_toml_table_header_name(line).is_some() {
            return start + offset;
        }
        offset += raw.len() + 1;
    }
    config.len()
}

fn find_toml_key_assignment_value_index(line: &str, key: &str) -> Option<usize> {
    // TS regex matches a single key token (quoted or bare), never dotted.
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() && (chars[i] == ' ' || chars[i] == '\t') {
        i += 1;
    }
    let start = i;
    let mut closed = false;
    match *chars.get(i)? {
        '"' => {
            i += 1;
            while i < chars.len() {
                match chars[i] {
                    '"' => {
                        i += 1;
                        closed = true;
                        break;
                    }
                    '\\' => i += 2,
                    '\r' | '\n' => return None,
                    _ => i += 1,
                }
            }
            if !closed {
                return None;
            }
        }
        '\'' => {
            i += 1;
            while i < chars.len() {
                match chars[i] {
                    '\'' => {
                        i += 1;
                        closed = true;
                        break;
                    }
                    '\r' | '\n' => return None,
                    _ => i += 1,
                }
            }
            if !closed {
                return None;
            }
        }
        c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '-')
            {
                i += 1;
            }
        }
        _ => return None,
    }
    let mut j = i;
    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
        j += 1;
    }
    if j >= chars.len() || chars[j] != '=' {
        return None;
    }
    j += 1;
    while j < chars.len() && (chars[j] == ' ' || chars[j] == '\t') {
        j += 1;
    }
    let raw_key: String = chars[start..i].iter().collect();
    let normalized = normalize_toml_table_header_name(&format!("[{raw_key}]"))?;
    if normalized != serde_json::to_string(&vec![key]).unwrap_or_default() {
        return None;
    }
    let byte_idx: usize = chars[..j].iter().map(|c| c.len_utf8()).sum();
    Some(byte_idx)
}

pub fn disable_codex_config_sections(
    config: String,
    section_headers: Vec<String>,
    append_missing: Option<bool>,
) -> Result<String, String> {
    let mut targets: Vec<(String, String)> = Vec::new(); // (tableName, header)
    for header in &section_headers {
        if !is_safe_plugin_section_header(&Value::String(header.clone())) {
            continue;
        }
        if let Some(name) = normalize_toml_table_header_name(header)
            && !targets.iter().any(|(n, _)| n == &name)
        {
            targets.push((name, header.clone()));
        }
    }
    let normalized = config.replace("\r\n", "\n");
    let lines: Vec<String> = normalized.split('\n').map(|s| s.to_string()).collect();
    let (_, structural) = split_config_lines(&config);
    let mut output: Vec<String> = Vec::new();
    let mut in_target = false;
    let mut seen: Vec<String> = Vec::new();
    let mut has_enabled = false;

    macro_rules! close_section {
        () => {
            if in_target && !has_enabled {
                output.push("enabled = false".to_string());
            }
        };
    }

    for (index, line) in lines.iter().enumerate() {
        if !structural[index] {
            output.push(line.clone());
            continue;
        }
        if let Some(name) = normalize_toml_table_header_name(line) {
            close_section!();
            in_target = targets.iter().any(|(n, _)| n == &name);
            if in_target && !seen.contains(&name) {
                seen.push(name);
            }
            has_enabled = false;
            output.push(line.clone());
            continue;
        }
        let enabled_idx = if in_target {
            find_toml_key_assignment_value_index(line, "enabled")
        } else {
            None
        };
        if let Some(idx) = enabled_idx {
            let rest = &line[idx..];
            let trimmed = rest.trim_end_matches([' ', '\t']);
            let core = match trimmed.find('#') {
                Some(p) => trimmed[..p].trim_end_matches([' ', '\t']),
                None => trimmed,
            };
            if core != "true" && core != "false" {
                return Err(
                    "Codex plugin enabled must be a boolean before Synara can disable it."
                        .to_string(),
                );
            }
            output.push("enabled = false".to_string());
            has_enabled = true;
            continue;
        }
        output.push(line.clone());
    }
    close_section!();

    if append_missing.unwrap_or(false) {
        for (name, header) in &targets {
            if seen.contains(name) {
                continue;
            }
            if let Some(last) = output.last()
                && !last.trim().is_empty()
            {
                output.push(String::new());
            }
            output.push(header.clone());
            output.push("enabled = false".to_string());
        }
    }
    Ok(output.join("\n"))
}

pub fn disable_competing_codex_browser_plugins_in_config(config: String) -> Result<String, String> {
    let (lines, structural) = split_config_lines(&config);
    let mut headers: Vec<String> = COMPETING_HEADERS.iter().map(|s| s.to_string()).collect();
    let mut extra: Vec<String> = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        if !structural[index] {
            continue;
        }
        let t = line.trim();
        if t.starts_with("[plugins.\"") && t.ends_with("\"]") {
            let inner = &t[10..t.len() - 2];
            if inner.len() >= 7 && inner.ends_with("-browser@local") {
                let prefix = &inner[..inner.len() - "-browser@local".len()];
                let ok = prefix.len() == 6
                    && prefix
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
                        .unwrap_or(false)
                    && prefix
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
                if ok && !extra.contains(&t.to_string()) {
                    extra.push(t.to_string());
                }
            }
        }
    }
    headers.extend(extra);
    disable_codex_config_sections(config, headers, Some(true))
}

pub async fn link_or_copy_codex_overlay_entry(
    input: serde_json::Value,
    linker: serde_json::Value,
) -> Result<(), String> {
    if linker.is_null() {
        return Err("Cannot read properties of null (reading 'symlink')".to_string());
    }
    if !is_undefined(&linker) {
        return Err("linker.symlink is not a function".to_string());
    }
    let src = prop(&input, "sourcePath")
        .and_then(|p| p.as_str())
        .unwrap_or("");
    let dst = prop(&input, "targetPath")
        .and_then(|p| p.as_str())
        .unwrap_or("");
    let entry = prop(&input, "entryName")
        .and_then(|p| p.as_str())
        .unwrap_or("");
    let itype = prop(&input, "type").and_then(|p| p.as_str()).unwrap_or("");
    #[cfg(unix)]
    let link_res = std::os::unix::fs::symlink(src, dst);
    #[cfg(not(unix))]
    let link_res: std::io::Result<()> = std::fs::hard_link(src, dst);
    match link_res {
        Ok(()) => Ok(()),
        Err(e) => {
            if itype == "file" && entry == "auth.json" {
                return std::fs::copy(src, dst).map(|_| ()).map_err(|ce| {
                    format!("{}, copyfile '{}' -> '{}'", node_fs_prefix(&ce), src, dst)
                });
            }
            Err(node_symlink_error(&e, src, dst))
        }
    }
}

fn node_fs_prefix(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "ENOENT: no such file or directory".to_string(),
        std::io::ErrorKind::AlreadyExists => "EEXIST: file already exists".to_string(),
        std::io::ErrorKind::PermissionDenied => "EACCES: permission denied".to_string(),
        _ => format!(
            "{}:",
            e.raw_os_error().map(|c| c.to_string()).unwrap_or_default()
        ),
    }
}

fn node_symlink_error(e: &std::io::Error, src: &str, dst: &str) -> String {
    format!("{}, symlink '{}' -> '{}'", node_fs_prefix(e), src, dst)
}

pub fn prioritize_codex_overlay_entries(entries: Vec<String>) -> Result<Vec<String>, String> {
    let mut shared = Vec::new();
    let mut other = Vec::new();
    for e in entries {
        if e == "auth.json" {
            shared.push(e);
        } else {
            other.push(e);
        }
    }
    shared.extend(other);
    Ok(shared)
}

pub fn extract_managed_codex_config_section(config: String) -> Result<serde_json::Value, String> {
    let (lines, structural) = split_config_lines(&config);
    let Some((begin, end)) = find_managed_config_block(&lines, &structural, 0) else {
        return Ok(undefined());
    };
    let content = lines[begin + 1..end].join("\n").trim().to_string();
    if content.is_empty() {
        Ok(undefined())
    } else {
        Ok(Value::String(content))
    }
}

pub fn config_has_toml_table_header(config: String, header: String) -> Result<bool, String> {
    Ok(find_toml_table_header(&config, &header).is_some())
}

pub fn merge_shell_env_policy_exclude(
    config: String,
    env_var_name: String,
) -> Result<String, String> {
    let additions: Vec<String> = if env_var_name.is_empty() {
        vec![]
    } else {
        vec![env_var_name]
    };
    if additions.is_empty() {
        return Ok(config);
    }
    let Some((_, table_start)) = find_toml_table_header(&config, "[shell_environment_policy]")
    else {
        return Ok(config);
    };
    let table_end = find_next_toml_table_header_index(&config, table_start);
    let table_body = &config[table_start..table_end];
    let (body_lines, body_structural) = split_config_lines(table_body);
    let mut open_bracket: Option<usize> = None;
    let mut offset = 0usize;
    for (index, line) in body_lines.iter().enumerate() {
        let vidx = if body_structural[index] {
            find_toml_key_assignment_value_index(line, "exclude")
        } else {
            None
        };
        if let Some(vidx) = vidx
            && line[vidx..].starts_with('[')
        {
            open_bracket = Some(offset + vidx);
            break;
        }
        offset += line.len() + 1;
    }
    if let Some(bracket) = open_bracket {
        let parsed = parse_toml(table_body);
        let existing = parsed
            .ok()
            .and_then(|p| p.get("exclude").cloned())
            .and_then(|v| v.as_array().cloned());
        let Some(existing) = existing else {
            return Ok(config);
        };
        let missing: Vec<&String> = additions
            .iter()
            .filter(|a| !existing.iter().any(|e| e.as_str() == Some(a.as_str())))
            .collect();
        if missing.is_empty() {
            return Ok(config);
        }
        let insert_at = table_start + bracket + 1;
        let separator = if existing.is_empty() { "" } else { ", " };
        let injected = missing
            .iter()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", ");
        return Ok(format!(
            "{}{}{}{}",
            &config[..insert_at],
            injected,
            separator,
            &config[insert_at..]
        ));
    }
    Ok(format!(
        "{}\nexclude = [{}]{}",
        &config[..table_start],
        additions
            .iter()
            .map(|v| serde_json::to_string(v).unwrap_or_default())
            .collect::<Vec<_>>()
            .join(", "),
        &config[table_start..]
    ))
}
