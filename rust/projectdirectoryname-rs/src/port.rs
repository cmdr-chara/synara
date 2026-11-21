//! The Rust port of `packages/shared/src/projectDirectoryName.ts`.

#![forbid(unsafe_code)]

const MAX_BYTES: usize = 255;
const INVALID_CHARS: &[char] = &['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
const RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

fn js_trim(s: &str) -> &str {
    const JS_WS: &[char] = &[
        '\t', '\n', '\u{b}', '\u{c}', '\r', ' ', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2001}',
        '\u{2002}', '\u{2003}', '\u{2004}', '\u{2005}', '\u{2006}', '\u{2007}', '\u{2008}',
        '\u{2009}', '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}',
        '\u{feff}',
    ];
    s.trim_matches(|c| JS_WS.contains(&c))
}

/// Port of `normalizeProjectDirectoryName` (`packages/shared/src/projectDirectoryName.ts`).
pub fn normalize_project_directory_name(value: String) -> Result<Option<String>, String> {
    let name = js_trim(&value);
    if name.is_empty()
        || name == "."
        || name == ".."
        || name.ends_with('.')
        || name.len() > MAX_BYTES
        || name.chars().any(|c| INVALID_CHARS.contains(&c) || c < ' ')
        || {
            let stem = name.split('.').next().unwrap_or_default().to_lowercase();
            RESERVED.contains(&stem.as_str())
        }
    {
        return Ok(None);
    }
    Ok(Some(name.to_string()))
}
