//! The Rust port of `packages/shared/src/logging.ts`.
//!
//! Replace each `unimplemented!` with the port of the source function. Return `Err` where
//! the source throws or raises: the caller gets an error there too. After changing a
//! signature, run `rust-anything refresh` to regenerate the bindings. `rust-anything diff` checks
//! behavior against what the source recorded.

#![forbid(unsafe_code)]

use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;

fn is_undefined(v: &Value) -> bool {
    matches!(v, Value::Object(o) if o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true)))
}

fn js_to_f64(v: &Value) -> f64 {
    match v {
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        Value::String(s) => js_string_to_f64(s),
        Value::Object(o) if o.len() == 1 && o.get("$num").is_some() => match o["$num"].as_str() {
            Some("NaN") => f64::NAN,
            Some("Infinity") => f64::INFINITY,
            Some("-Infinity") => f64::NEG_INFINITY,
            Some("-0") => -0.0,
            _ => f64::NAN,
        },
        Value::Array(a) if a.len() == 1 => js_to_f64(&a[0]),
        _ => f64::NAN,
    }
}

fn js_string_to_f64(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `value < 1` under JS semantics.
fn js_lt_one(v: &Value) -> bool {
    if is_undefined(v) {
        return false;
    }
    js_to_f64(v) < 1.0
}

/// `${value}` interpolation for error messages.
fn js_to_display(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => fmt_f64(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(js_to_display).collect::<Vec<_>>().join(","),
        Value::Object(o) if o.len() == 1 && o.get("$num").is_some() => match o["$num"].as_str() {
            Some("NaN") => "NaN".into(),
            Some("Infinity") => "Infinity".into(),
            Some("-Infinity") => "-Infinity".into(),
            Some("-0") => "0".into(),
            _ => "NaN".into(),
        },
        Value::Object(o) if is_undefined(&Value::Object(o.clone())) => "undefined".into(),
        Value::Object(_) => "[object Object]".into(),
    }
}

/// JS Number→string formatting (no trailing `.0`, -0 → "0").
fn fmt_f64(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n == f64::INFINITY {
        return "Infinity".into();
    }
    if n == f64::NEG_INFINITY {
        return "-Infinity".into();
    }
    if n == 0.0 {
        return "0".into();
    }
    if n.fract() == 0.0 && n.abs() < 1e21 {
        return format!("{}", n as i64);
    }
    format!("{}", n)
}

/// POSIX `path.dirname` approximation.
fn js_dirname(p: &str) -> String {
    let t = p.trim_end_matches('/');
    if t.is_empty() {
        return "/".into();
    }
    match t.rfind('/') {
        None => ".".into(),
        Some(0) => "/".into(),
        Some(i) => t[..i].to_string(),
    }
}

/// POSIX `path.basename` approximation.
fn js_basename(p: &str) -> String {
    let t = p.trim_end_matches('/');
    if t.is_empty() {
        return String::new();
    }
    match t.rfind('/') {
        None => t.to_string(),
        Some(i) => t[i + 1..].to_string(),
    }
}

/// JS type-of name for fs error messages.
fn js_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) | Value::Object(_) => {
            if is_undefined(v) {
                "undefined"
            } else {
                "object"
            }
        }
    }
}

/// Port of class `RotatingFileSink` (`packages/shared/src/logging.ts:18`): holds the instance state.
pub struct RotatingFileSink {
    file_path: String,
    max_bytes: f64,
    max_files: f64,
    throw_on_error: bool,
    mode: Option<f64>,
    keep_file_open: bool,
    fd: Option<File>,
    current_size: f64,
}

impl RotatingFileSink {
    /// Port of `RotatingFileSink` constructor (`packages/shared/src/logging.ts:18`).
    pub fn new(options: serde_json::Value) -> Result<Self, String> {
        if options.is_null() {
            return Err("null is not an object (evaluating 'options.maxBytes')".into());
        }
        if is_undefined(&options) {
            return Err("undefined is not an object (evaluating 'options.maxBytes')".into());
        }
        let max_bytes_v = options.get("maxBytes").cloned().unwrap_or(Value::Null);
        let max_files_v = options.get("maxFiles").cloned().unwrap_or(Value::Null);
        if js_lt_one(&max_bytes_v) {
            return Err(format!(
                "maxBytes must be >= 1 (received {})",
                js_to_display(&max_bytes_v)
            ));
        }
        if js_lt_one(&max_files_v) {
            return Err(format!(
                "maxFiles must be >= 1 (received {})",
                js_to_display(&max_files_v)
            ));
        }
        let file_path_v = options.get("filePath").cloned().unwrap_or(Value::Null);
        let file_path = match &file_path_v {
            Value::String(s) => s.clone(),
            _ => {
                return Err(format!(
                    "The \"path\" property must be of type string, got {}",
                    if matches!(file_path_v, Value::Null) {
                        "null".to_string()
                    } else {
                        js_type_name(&file_path_v).to_string()
                    }
                ));
            }
        };
        let throw_on_error = options
            .get("throwOnError")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mode = options.get("mode").and_then(Value::as_f64);
        let keep_file_open = options
            .get("keepFileOpen")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max_bytes = js_to_f64(&max_bytes_v);
        let max_files = js_to_f64(&max_files_v);

        // fs.mkdirSync(path.dirname(this.filePath), { recursive: true })
        if let Err(e) = fs::create_dir_all(js_dirname(&file_path)) {
            return Err(format!("{}", e));
        }
        let mut sink = Self {
            file_path,
            max_bytes,
            max_files,
            throw_on_error,
            mode,
            keep_file_open,
            fd: None,
            current_size: 0.0,
        };
        sink.prune_overflow_backups()?;
        sink.current_size = sink.read_current_size();
        Ok(sink)
    }

    /// Port of `RotatingFileSink.write` (`packages/shared/src/logging.ts:48`).
    pub fn write(&mut self, chunk: serde_json::Value) -> Result<(), String> {
        // `typeof chunk === "string" ? Buffer.from(chunk) : chunk` then `buffer.length`.
        let (data, length): (Option<Vec<u8>>, Option<f64>) = match &chunk {
            Value::String(s) => {
                let b = s.clone().into_bytes();
                let l = b.len() as f64;
                (Some(b), Some(l))
            }
            Value::Null => {
                return Err("null is not an object (evaluating 'buffer.length')".into());
            }
            Value::Array(a) => (None, Some(a.len() as f64)),
            v if is_undefined(v) => {
                return Err("undefined is not an object (evaluating 'buffer.length')".into());
            }
            _ => (None, None),
        };
        if length == Some(0.0) {
            return Ok(());
        }

        let write_result: Result<(), ()> = (|| {
            if self.current_size > 0.0
                && self.current_size + length.unwrap_or(f64::NAN) > self.max_bytes
            {
                self.rotate()?;
            }
            match &data {
                Some(buffer) => {
                    if self.keep_file_open {
                        self.write_to_open_file(buffer)?;
                    } else {
                        let mut f = self.open_append()?;
                        f.write_all(buffer).map_err(|_| ())?;
                    }
                    self.current_size += length.unwrap_or(0.0);
                }
                None => {
                    // Non-string/non-Buffer chunk. keepFileOpen opens the fd first, then
                    // `while (offset < buffer.length)` skips when length is undefined and
                    // writeSync throws when it is an array with items. Otherwise
                    // appendFileSync(path, <invalid>) throws ERR_INVALID_ARG_TYPE.
                    if self.keep_file_open {
                        if self.fd.is_none() {
                            self.fd = Some(self.open_append()?);
                        }
                        if length.is_some() {
                            return Err(());
                        }
                    } else {
                        return Err(());
                    }
                    self.current_size = f64::NAN;
                }
            }
            if self.current_size > self.max_bytes {
                self.rotate()?;
            }
            Ok(())
        })();

        match write_result {
            Ok(()) => Ok(()),
            Err(_) => {
                self.close_file();
                self.current_size = self.read_current_size();
                if self.throw_on_error {
                    Err(format!("Failed to write log chunk to {}", self.file_path))
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Port of `RotatingFileSink.close` (`packages/shared/src/logging.ts:79`).
    pub fn close(&mut self) -> Result<(), String> {
        self.close_file();
        Ok(())
    }

    fn open_append(&self) -> Result<File, ()> {
        let mut o = OpenOptions::new();
        o.append(true).create(true);
        if let Some(m) = self.mode {
            o.mode(m as u32);
        }
        o.open(&self.file_path).map_err(|_| ())
    }

    fn write_to_open_file(&mut self, buffer: &[u8]) -> Result<(), ()> {
        if self.fd.is_none() {
            self.fd = Some(self.open_append()?);
        }
        let f = self.fd.as_mut().ok_or(())?;
        let mut offset = 0;
        while offset < buffer.len() {
            let written = f.write(&buffer[offset..]).map_err(|_| ())?;
            if written == 0 {
                return Err(());
            }
            offset += written;
        }
        Ok(())
    }

    fn close_file(&mut self) {
        self.fd = None;
    }

    fn rotate(&mut self) -> Result<(), ()> {
        let result: Result<(), ()> = (|| {
            self.close_file();
            let oldest = self.with_suffix(self.max_files);
            if std::path::Path::new(&oldest).exists() {
                fs::remove_file(&oldest).map_err(|_| ())?;
            }
            // `index = maxFiles - 1` can be in the billions; indices above the highest
            // existing backup are all existsSync misses, so start there instead.
            let top = self.max_existing_suffix().min(self.max_files - 1.0);
            let mut index = top;
            while index >= 1.0 {
                let source = self.with_suffix(index);
                let target = self.with_suffix(index + 1.0);
                if std::path::Path::new(&source).exists() {
                    fs::rename(&source, &target).map_err(|_| ())?;
                }
                index -= 1.0;
            }
            if std::path::Path::new(&self.file_path).exists() {
                fs::rename(&self.file_path, self.with_suffix(1.0)).map_err(|_| ())?;
            }
            self.current_size = 0.0;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(()),
            Err(()) => {
                self.current_size = self.read_current_size();
                if self.throw_on_error { Err(()) } else { Ok(()) }
            }
        }
    }

    fn prune_overflow_backups(&mut self) -> Result<(), String> {
        let result = self.prune_overflow_backups_inner();
        match result {
            Ok(()) => Ok(()),
            Err(_e) => {
                if self.throw_on_error {
                    Err(format!(
                        "Failed to prune log backups for {}",
                        self.file_path
                    ))
                } else {
                    Ok(())
                }
            }
        }
    }

    fn prune_overflow_backups_inner(&self) -> Result<(), String> {
        let dir = js_dirname(&self.file_path);
        let base_name = js_basename(&self.file_path);
        let prefix = format!("{}.", base_name);
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with(&prefix) {
                    continue;
                }
                let suffix = js_string_to_f64(&name[prefix.len()..]);
                if suffix.fract() != 0.0 || suffix.is_nan() || suffix.is_infinite() {
                    continue;
                }
                if suffix <= self.max_files {
                    continue;
                }
                let _ = fs::remove_file(entry.path());
            }
        }
        Ok(())
    }

    /// Highest existing numeric backup suffix (`filePath.N`), 0 when none.
    fn max_existing_suffix(&self) -> f64 {
        let dir = js_dirname(&self.file_path);
        let prefix = format!("{}.", js_basename(&self.file_path));
        let mut top = 0.0;
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if let Some(rest) = name.strip_prefix(&prefix) {
                    let n = js_string_to_f64(rest);
                    if n.fract() == 0.0 && !n.is_nan() && !n.is_infinite() && n > top {
                        top = n;
                    }
                }
            }
        }
        top
    }

    fn read_current_size(&self) -> f64 {
        fs::metadata(&self.file_path)
            .map(|m| m.len() as f64)
            .unwrap_or(0.0)
    }

    fn with_suffix(&self, index: f64) -> String {
        format!("{}.{}", self.file_path, fmt_f64(index))
    }
}
