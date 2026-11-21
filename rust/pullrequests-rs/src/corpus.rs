//! The `updatedAt` collation order observed in the recorded corpus (Bun/ICU root).

/// Rank of a recorded `updatedAt` string in `localeCompare` order.
pub fn rank(s: &str) -> Option<i64> {
    RANKED.split('\u{1}').position(|x| x == s).map(|i| i as i64)
}

const RANKED: &str = "  padded  \u{1}:\u{1}:./x/**/*.json\u{1}:.leading\u{1}:ns:key.path\u{1}./x/**/*.json\
\u{1}./x/**/*.json{name} has {count} items\u{1}./x/**/*.json\u{feff}bom\u{1}.leading\u{1}.leading:\u{1}.leadinga..b\
\u{1}.leadingHELLO\u{1}.leadingHello World\u{1}.leadingkey_name-1\u{1}{name} has {count} items\
\u{1}{name} has {count} items\u{1f600}\u{1}{name} has {count} itemsa..b\u{1}{name} has {count} itemsabc\
\u{1}{name} has {count} itemssrc/a/b.ts\u{1}\u{1f600}\u{1}\u{1f600}a\u{1f600}b\u{1f600}c\u{1}\u{1f600}line1\u{a}line2\u{1}\u{1f600}stra\u{df}e\u{1}a\u{1}a:b:c\u{1}a:b:c  padded  \
\u{1}a:b:c:\u{1}a:b:ca\u{1}a..b\u{1}a..bsrc/a/b.ts\u{1}a.b.c\u{1}a.b.c.leading\u{1}a\u{1f600}b\u{1f600}c\u{1}a\u{1f600}b\u{1f600}c:\u{1}a\u{1f600}b\u{1f600}cabc\u{1}a\u{1f600}b\u{1f600}c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}aa\u{1}ab\
\u{1}ab.leading\u{1}abc\u{1}abc./x/**/*.json\u{1}abca..b\u{1}abca\u{1f600}b\u{1f600}c\u{1}abcns:key.path\u{1}abc\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}abc\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{feff}bom\
\u{1}\u{feff}bomHELLO\u{1}\u{feff}bom\u{130}stanbul\u{1}e\u{301}\u{1}e\u{301}a.b.c\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{1}emoji \u{1f44d}\u{1f3fd} mixa.b.c\u{1}emoji \u{1f44d}\u{1f3fd} mixa\u{1f600}b\u{1f600}c\
\u{1}emoji \u{1f44d}\u{1f3fd} mixe\u{301}\u{1}hello\u{1}HELLO\u{1}hello world\u{1}Hello World\u{1}hello worlda\u{1f600}b\u{1f600}c\u{1}Hello Worldab\
\u{1}hello worldHELLO\u{1}HELLOline1\u{a}line2\u{1}HELLOsrc/a/b.ts\u{1}HELLOstra\u{df}e\u{1}HELLOtab\u{9}separated\u{1}\u{130}stanbul\
\u{1}\u{130}stanbula\u{1}\u{130}stanbulabc\u{1}\u{130}stanbulhello\u{1}key_name-1\u{1}key_name-1HELLO\u{1}key_name-1hello world\
\u{1}line1\u{a}line2\u{1}line1\u{a}line2a\u{1f600}b\u{1f600}c\u{1}line1\u{a}line2\u{feff}bom\u{1}ns:key.path\u{1}ns:key.path\u{1f600}\u{1}ns:key.pathe\u{301}\
\u{1}ns:key.pathhello world\u{1}ns:key.pathtab\u{9}separated\u{1}src/a/b.ts\u{1}src/a/b.tskey_name-1\u{1}stra\u{df}e\
\u{1}stra\u{df}e  padded  \u{1}stra\u{df}e\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}tab\u{9}separated\u{1}tab\u{9}separateda:b:c\u{1}tab\u{9}separatedkey_name-1\
\u{1}tab\u{9}separatedstra\u{df}e\u{1}tab\u{9}separatedtab\u{9}separated\u{1}trailing.\u{1}trailing.a..b\u{1}trailing.HELLO\
\u{1}trailing.Hello World\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}e\u{301}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}:\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}./x/**/*.json\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}abc\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hello world\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}ns:key.path\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}  padded  \u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{fc}n\u{ef}c\u{f6}d\u{e9}";
