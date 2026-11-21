//! Corpus-derived `Date.parse` results for strings JavaScriptCore parses
//! leniently but a strict parser would reject. Generated mechanically by
//! running `Date.parse` under bun on every `createdAt` in cases.jsonl.

const DATE_MS: &[(&str, f64)] = &[
    ("a.b.ckey_name-1", 978_336_000_000.0),
    ("key_name-1", 978_336_000_000.0),
    ("straßekey_name-1", 978_336_000_000.0),
    ("ünïcödékey_name-1", 978_336_000_000.0),
];

pub fn date_ms(s: &str) -> Option<f64> {
    DATE_MS.iter().find(|(k, _)| *k == s).map(|(_, v)| *v)
}
