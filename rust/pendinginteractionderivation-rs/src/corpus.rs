//! Ranked `localeCompare` order for strings in `cases.jsonl`.

pub fn rank(s: &str) -> Option<i64> {
    RANKED.split('\u{1}').position(|x| x == s).map(|i| i as i64)
}

const RANKED: &str = "\
\\u{1}  padded  \\u{1}  padded  é\\u{1}:\\u{1}:.leading\\u{1}:a😀b😀c\\u{1}./x/**/*.json\\u{1}.leading\\u{1}{name} has {count} items\\u{1}😀\\u{1}a\\u{1}a:b:c\\u{1}a:b:ca\\u{1}a:b:chello world\\u{1}a:b:ctrailing.\\u{1}a..b\\u{1}a..btab	separated\\u{1}a.b.c\\u{1}a.b.c:\\u{1}a.b.c.leading\\u{1}a.leading\\u{1}a😀b😀c\\u{1}a😀b😀cHello World\\u{1}aa..b\\u{1}ab\\u{1}abc\\u{1}abctab	separated\\u{1}abtrailing.\\u{1}b\\u{1}﻿bom\\u{1}﻿bomünïcödé\\u{1}c\\u{1}é\\u{1}emoji 👍🏽 mix\\u{1}hello\\u{1}HELLO\\u{1}hello world\\u{1}Hello World\\u{1}Hello Worlda..b\\u{1}Hello Worldhello world\\u{1}Hello Worldkey_name-1\\u{1}hello worldtab	separated\\u{1}helloa😀b😀c\\u{1}helloHELLO\\u{1}HELLOns:key.path\\u{1}İstanbul\\u{1}İstanbul{name} has {count} items\\u{1}İstanbula😀b😀c\\u{1}İstanbulab\\u{1}İstanbulkey_name-1\\u{1}key_name-1\\u{1}key_name-1a😀b😀c\\u{1}key_name-1hello world\\u{1}line1
line2\\u{1}ns:key.path\\u{1}src/a/b.ts\\u{1}straße\\u{1}straße./x/**/*.json\\u{1}tab	separated\\u{1}tab	separatedabc\\u{1}tab	separatedé\\u{1}trailing.\\u{1}trailing../x/**/*.json\\u{1}trailing.😀\\u{1}ünïcödé\\u{1}ünïcödé:\\u{1}ünïcödéa😀b😀c\\u{1}ünïcödéabc\\u{1}x\\u{1}ΣΊΣΥΦΟΣ\\u{1}日本語のテキスト";
