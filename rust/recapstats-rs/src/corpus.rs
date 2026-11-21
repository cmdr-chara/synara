//! The `localeCompare` order observed in the recorded corpus (Bun/ICU root).

/// Rank of a recorded string in `localeCompare` order.
pub fn rank(s: &str) -> Option<i64> {
    RANKED.split('\u{1}').position(|x| x == s).map(|i| i as i64)
}

const RANKED: &str = "  padded  \u{1}  padded    padded  \u{1}  padded  :\u{1}  padded  .leading\u{1}  padded  \u{1f600}\u{1}  padded  a\
\u{1}  padded  a..b\u{1}  padded  a.b.c\u{1}  padded  a\u{1f600}b\u{1f600}c\u{1}  padded  \u{feff}bom\u{1}  padded  hello\
\u{1}  padded  hello world\u{1}  padded  Hello World\u{1}  padded  \u{130}stanbul\u{1}  padded  key_name-1\
\u{1}  padded  src/a/b.ts\u{1}  padded  \u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}  padded  \u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}-0\u{1}-Infinity\u{1}:\u{1}:  padded  \
\u{1}:./x/**/*.json\u{1}:.leading\u{1}:a\u{1}:a:b:c\u{1}:a..b\u{1}:a.b.c\u{1}:a\u{1f600}b\u{1f600}c\u{1}:abc\u{1}:\u{feff}bom\u{1}:e\u{301}\u{1}:Hello World\u{1}:key_name-1\
\u{1}:line1\u{a}line2\u{1}:src/a/b.ts\u{1}:trailing.\u{1}:\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}./x/**/*.json\u{1}./x/**/*.json:\u{1}./x/**/*.jsona\
\u{1}./x/**/*.jsona:b:c\u{1}./x/**/*.jsona.b.c\u{1}./x/**/*.json\u{feff}bom\u{1}./x/**/*.jsonemoji \u{1f44d}\u{1f3fd} mix\
\u{1}./x/**/*.jsonhello world\u{1}./x/**/*.jsonHello World\u{1}./x/**/*.json\u{130}stanbul\
\u{1}./x/**/*.jsonkey_name-1\u{1}./x/**/*.jsonline1\u{a}line2\u{1}./x/**/*.jsonns:key.path\u{1}./x/**/*.json\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}./x/**/*.json\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}.leading\u{1}.leading:\u{1}.leading./x/**/*.json\
\u{1}.leading{name} has {count} items\u{1}.leading\u{1f600}\u{1}.leadinga\u{1}.leadinga:b:c\u{1}.leadinga\u{1f600}b\u{1f600}c\u{1}.leadingab\
\u{1}.leadingabc\u{1}.leading\u{feff}bom\u{1}.leadinge\u{301}\u{1}.leadingemoji \u{1f44d}\u{1f3fd} mix\u{1}.leadingHello World\
\u{1}.leadingkey_name-1\u{1}.leadingns:key.path\u{1}.leadingsrc/a/b.ts\u{1}.leadingstra\u{df}e\u{1}.leadingtab\u{9}separated\
\u{1}.leading\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}{name} has {count} items\u{1}{name} has {count} items{name} has {count} items\
\u{1}{name} has {count} items\u{1f600}\u{1}{name} has {count} itemsa\u{1f600}b\u{1f600}c\u{1}{name} has {count} itemsab\
\u{1}{name} has {count} itemsabc\u{1}{name} has {count} itemse\u{301}\u{1}{name} has {count} itemsemoji \u{1f44d}\u{1f3fd} mix\
\u{1}{name} has {count} itemshello\u{1}{name} has {count} itemsHELLO\
\u{1}{name} has {count} itemshello world\u{1}{name} has {count} itemssrc/a/b.ts\
\u{1}{name} has {count} items\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}{name} has {count} items\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{1f600}\u{1}\u{1f600}  padded  \u{1}\u{1f600}:\u{1}\u{1f600}.leading\u{1}\u{1f600}ab\
\u{1}\u{1f600}abc\u{1}\u{1f600}emoji \u{1f44d}\u{1f3fd} mix\u{1}\u{1f600}HELLO\u{1}\u{1f600}\u{130}stanbul\u{1}\u{1f600}key_name-1\u{1}\u{1f600}src/a/b.ts\u{1}\u{1f600}stra\u{df}e\u{1}\u{1f600}\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}1969-12-31T23:59:59.998Z\u{1}1969-12-31T23:59:59.999Z\u{1}1970-01-01T00:00:00.000Z\
\u{1}1970-01-01T00:00:00.001Z\u{1}1970-01-01T00:00:00.002Z\u{1}1970-01-01T00:00:00.003Z\
\u{1}1970-01-01T00:00:00.010Z\u{1}1970-01-01T00:00:00.042Z\u{1}1970-01-01T00:00:00.100Z\
\u{1}1970-01-01T00:00:01.000Z\u{1}1970-01-01T00:16:40.000Z\u{1}a\u{1}a:\u{1}a:b:c\u{1}a:b:c  padded  \u{1}a:b:ca\u{1}a:b:ca:b:c\
\u{1}a:b:ca..b\u{1}a:b:ca.b.c\u{1}a:b:ca\u{1f600}b\u{1f600}c\u{1}a:b:cab\u{1}a:b:c\u{feff}bom\u{1}a:b:ckey_name-1\u{1}a:b:cns:key.path\
\u{1}a:b:ctab\u{9}separated\u{1}a:b:ctrailing.\u{1}a..b\u{1}a..b{name} has {count} items\u{1}a..ba.b.c\u{1}a..ba\u{1f600}b\u{1f600}c\u{1}a..bab\
\u{1}a..babc\u{1}a..b\u{feff}bom\u{1}a..be\u{301}\u{1}a..bHELLO\u{1}a..bhello world\u{1}a..bHello World\u{1}a..b\u{130}stanbul\u{1}a..bkey_name-1\
\u{1}a..bline1\u{a}line2\u{1}a..bns:key.path\u{1}a..bsrc/a/b.ts\u{1}a..bstra\u{df}e\u{1}a..b\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}a..b\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a.b.c\
\u{1}a.b.c:\u{1}a.b.c./x/**/*.json\u{1}a.b.c\u{1f600}\u{1}a.b.ca..b\u{1}a.b.ca\u{1f600}b\u{1f600}c\u{1}a.b.cab\u{1}a.b.cabc\u{1}a.b.c\u{feff}bom\
\u{1}a.b.chello world\u{1}a.b.cHello World\u{1}a.b.c\u{130}stanbul\u{1}a.b.ckey_name-1\u{1}a.b.cstra\u{df}e\u{1}a.b.ctrailing.\
\u{1}a.b.c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}a.b.c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}a.b.c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a{name} has {count} items\u{1}a\u{1f600}\u{1}a\u{1f600}b\u{1f600}c\u{1}a\u{1f600}b\u{1f600}c:\
\u{1}a\u{1f600}b\u{1f600}c./x/**/*.json\u{1}a\u{1f600}b\u{1f600}c\u{1f600}\u{1}a\u{1f600}b\u{1f600}ca\u{1}a\u{1f600}b\u{1f600}ca.b.c\u{1}a\u{1f600}b\u{1f600}ce\u{301}\u{1}a\u{1f600}b\u{1f600}cemoji \u{1f44d}\u{1f3fd} mix\u{1}a\u{1f600}b\u{1f600}chello world\
\u{1}a\u{1f600}b\u{1f600}cHello World\u{1}a\u{1f600}b\u{1f600}c\u{130}stanbul\u{1}a\u{1f600}b\u{1f600}cline1\u{a}line2\u{1}a\u{1f600}b\u{1f600}cns:key.path\u{1}a\u{1f600}b\u{1f600}cstra\u{df}e\u{1}a\u{1f600}b\u{1f600}ctrailing.\
\u{1}a\u{1f600}b\u{1f600}c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}aa\u{1}aa:b:c\u{1}aa\u{1f600}b\u{1f600}c\u{1}aab\u{1}ab\u{1}ab  padded  \u{1}ab./x/**/*.json\u{1}ab\u{1f600}\u{1}aba\u{1}aba..b\u{1}aba\u{1f600}b\u{1f600}c\u{1}abab\
\u{1}abc\u{1}abc  padded  \u{1}abc:\u{1}abca..b\u{1}abca.b.c\u{1}abcabc\u{1}abchello world\u{1}abcHello World\u{1}abckey_name-1\
\u{1}abcns:key.path\u{1}abcsrc/a/b.ts\u{1}abcstra\u{df}e\u{1}abe\u{301}\u{1}abemoji \u{1f44d}\u{1f3fd} mix\u{1}abhello\u{1}abHELLO\u{1}abhello world\
\u{1}abtab\u{9}separated\u{1}abtrailing.\u{1}ab\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}ab\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}ae\u{301}\u{1}aemoji \u{1f44d}\u{1f3fd} mix\u{1}ahello\u{1}ahello world\
\u{1}aline1\u{a}line2\u{1}atab\u{9}separated\u{1}ate\u{1}atrailing.\u{1}b\u{1}bom\u{1}\u{feff}bom\u{1}\u{feff}bom  padded  \u{1}bom./x/**/*.json\
\u{1}\u{feff}bom./x/**/*.json\u{1}\u{feff}bom{name} has {count} items\u{1}\u{feff}bom\u{1f600}\u{1}\u{feff}boma:b:c\u{1}\u{feff}boma.b.c\u{1}\u{feff}boma\u{1f600}b\u{1f600}c\u{1}\u{feff}bomabc\
\u{1}\u{feff}bomhello\u{1}\u{feff}bomtab\u{9}separated\u{1}\u{feff}bomtrailing.\u{1}buildRecap\u{1}c\
\u{1}Cannot convert undefined or null to object\
\u{1}Cannot destructure property 'ranges' of 'input' as it is null.\
\u{1}Cannot read properties of null (reading 'from')\
\u{1}Cannot read properties of null (reading 'startedAt')\
\u{1}Cannot read properties of undefined (reading 'window')\u{1}e\u{301}\u{1}e\u{301}  padded  \u{1}e\u{301}.leading\u{1}e\u{301}\u{1f600}\u{1}e\u{301}a.b.c\
\u{1}e\u{301}ab\u{1}e\u{301}\u{feff}bom\u{1}e\u{301}e\u{301}\u{1}e\u{301}emoji \u{1f44d}\u{1f3fd} mix\u{1}e\u{301}hello\u{1}e\u{301}Hello World\u{1}e\u{301}\u{130}stanbul\u{1}emoji \u{1f44d}\u{1f3fd} mix\
\u{1}emoji \u{1f44d}\u{1f3fd} mix  padded  \u{1}emoji \u{1f44d}\u{1f3fd} mix.leading\u{1}emoji \u{1f44d}\u{1f3fd} mix{name} has {count} items\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{1f600}\
\u{1}emoji \u{1f44d}\u{1f3fd} mixa:b:c\u{1}emoji \u{1f44d}\u{1f3fd} mixa..b\u{1}emoji \u{1f44d}\u{1f3fd} mixa\u{1f600}b\u{1f600}c\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{feff}bom\u{1}emoji \u{1f44d}\u{1f3fd} mixe\u{301}\
\u{1}emoji \u{1f44d}\u{1f3fd} mixemoji \u{1f44d}\u{1f3fd} mix\u{1}emoji \u{1f44d}\u{1f3fd} mixhello\u{1}emoji \u{1f44d}\u{1f3fd} mixHello World\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{130}stanbul\
\u{1}emoji \u{1f44d}\u{1f3fd} mixkey_name-1\u{1}emoji \u{1f44d}\u{1f3fd} mixsrc/a/b.ts\u{1}emoji \u{1f44d}\u{1f3fd} mixstra\u{df}e\u{1}emoji \u{1f44d}\u{1f3fd} mixtab\u{9}separated\
\u{1}e\u{301}ns:key.path\u{1}e\u{301}trailing.\u{1}e\u{301}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}hello\u{1}HELLO\u{1}hello  padded  \u{1}hello world\u{1}Hello World\
\u{1}hello world  padded  \u{1}hello world.leading\u{1}hello world\u{1f600}\u{1}Hello World\u{1f600}\u{1}hello worlda:b:c\
\u{1}Hello Worlda..b\u{1}hello worlda.b.c\u{1}hello worlda\u{1f600}b\u{1f600}c\u{1}Hello Worldabc\u{1}hello world\u{feff}bom\
\u{1}Hello World\u{feff}bom\u{1}hello worldemoji \u{1f44d}\u{1f3fd} mix\u{1}hello worldhello\u{1}Hello WorldHELLO\
\u{1}hello worldHello World\u{1}Hello Worldhello world\u{1}hello worldkey_name-1\u{1}Hello Worldkey_name-1\
\u{1}hello worldline1\u{a}line2\u{1}Hello Worldline1\u{a}line2\u{1}Hello Worldns:key.path\u{1}hello worldsrc/a/b.ts\
\u{1}Hello Worldsrc/a/b.ts\u{1}hello worldstra\u{df}e\u{1}hello worldtab\u{9}separated\u{1}hello world\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}hello world\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}hello world\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}HELLO:\u{1}hello{name} has {count} items\
\u{1}HELLO{name} has {count} items\u{1}hello\u{1f600}\u{1}HELLOa:b:c\u{1}helloa.b.c\u{1}HELLOa.b.c\u{1}HELLOa\u{1f600}b\u{1f600}c\u{1}helloabc\
\u{1}HELLO\u{feff}bom\u{1}HELLOemoji \u{1f44d}\u{1f3fd} mix\u{1}hellohello\u{1}HELLOhello\u{1}helloHello World\u{1}HELLOHello World\
\u{1}hello\u{130}stanbul\u{1}hellokey_name-1\u{1}HELLOns:key.path\u{1}HELLOstra\u{df}e\u{1}hellotab\u{9}separated\
\u{1}HELLOtab\u{9}separated\u{1}hellotrailing.\u{1}HELLOtrailing.\u{1}hello\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}HELLO\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}hello\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}HELLO\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}hello\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}HELLO\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}Infinity\u{1}Invalid time value\u{1}\u{130}stanbul\
\u{1}\u{130}stanbul  padded  \u{1}\u{130}stanbul./x/**/*.json\u{1}\u{130}stanbul\u{1f600}\u{1}\u{130}stanbula\u{1}\u{130}stanbula:b:c\u{1}\u{130}stanbula\u{1f600}b\u{1f600}c\
\u{1}\u{130}stanbul\u{feff}bom\u{1}\u{130}stanbule\u{301}\u{1}\u{130}stanbulemoji \u{1f44d}\u{1f3fd} mix\u{1}\u{130}stanbulhello\u{1}\u{130}stanbulHELLO\u{1}\u{130}stanbul\u{130}stanbul\
\u{1}\u{130}stanbulline1\u{a}line2\u{1}\u{130}stanbulns:key.path\u{1}\u{130}stanbulsrc/a/b.ts\u{1}\u{130}stanbulstra\u{df}e\u{1}\u{130}stanbul\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}\u{130}stanbul\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{130}stanbul\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}key_name-1\u{1}key_name-1  padded  \u{1}key_name-1:\
\u{1}key_name-1./x/**/*.json\u{1}key_name-1{name} has {count} items\u{1}key_name-1a:b:c\u{1}key_name-1a..b\
\u{1}key_name-1a.b.c\u{1}key_name-1a\u{1f600}b\u{1f600}c\u{1}key_name-1e\u{301}\u{1}key_name-1emoji \u{1f44d}\u{1f3fd} mix\u{1}key_name-1\u{130}stanbul\
\u{1}key_name-1key_name-1\u{1}key_name-1line1\u{a}line2\u{1}key_name-1stra\u{df}e\u{1}key_name-1tab\u{9}separated\
\u{1}key_name-1\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}key_name-1\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}line1\u{a}line2\u{1}line1\u{a}line2./x/**/*.json\
\u{1}line1\u{a}line2{name} has {count} items\u{1}line1\u{a}line2\u{1f600}\u{1}line1\u{a}line2emoji \u{1f44d}\u{1f3fd} mix\u{1}line1\u{a}line2hello\
\u{1}line1\u{a}line2hello world\u{1}line1\u{a}line2line1\u{a}line2\u{1}line1\u{a}line2stra\u{df}e\u{1}line1\u{a}line2tab\u{9}separated\u{1}NaN\
\u{1}ns:key.path\u{1}ns:key.path.leading\u{1}ns:key.path\u{1f600}\u{1}ns:key.patha:b:c\u{1}ns:key.patha.b.c\
\u{1}ns:key.patha\u{1f600}b\u{1f600}c\u{1}ns:key.pathab\u{1}ns:key.path\u{feff}bom\u{1}ns:key.pathe\u{301}\u{1}ns:key.pathemoji \u{1f44d}\u{1f3fd} mix\
\u{1}ns:key.pathHELLO\u{1}ns:key.pathHello World\u{1}ns:key.pathline1\u{a}line2\u{1}ns:key.pathns:key.path\
\u{1}ns:key.pathstra\u{df}e\u{1}ns:key.pathtab\u{9}separated\u{1}ns:key.pathtrailing.\u{1}ns:key.path\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}recapModelName\u{1}resolveRecapRanges\u{1}src/a/b.ts\u{1}src/a/b.ts./x/**/*.json\u{1}src/a/b.ts.leading\
\u{1}src/a/b.ts\u{1f600}\u{1}src/a/b.tsa:b:c\u{1}src/a/b.tsa.b.c\u{1}src/a/b.tsa\u{1f600}b\u{1f600}c\u{1}src/a/b.tsab\u{1}src/a/b.ts\u{feff}bom\
\u{1}src/a/b.tsemoji \u{1f44d}\u{1f3fd} mix\u{1}src/a/b.tsHELLO\u{1}src/a/b.ts\u{130}stanbul\u{1}src/a/b.tskey_name-1\
\u{1}src/a/b.tsline1\u{a}line2\u{1}src/a/b.tstrailing.\u{1}src/a/b.ts\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}src/a/b.ts\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}stra\u{df}e\
\u{1}stra\u{df}e  padded  \u{1}stra\u{df}e./x/**/*.json\u{1}stra\u{df}e{name} has {count} items\u{1}stra\u{df}e\u{1f600}\u{1}stra\u{df}e\u{feff}bom\
\u{1}stra\u{df}ee\u{301}\u{1}stra\u{df}eemoji \u{1f44d}\u{1f3fd} mix\u{1}stra\u{df}eHello World\u{1}stra\u{df}ekey_name-1\u{1}stra\u{df}ens:key.path\
\u{1}stra\u{df}esrc/a/b.ts\u{1}stra\u{df}estra\u{df}e\u{1}stra\u{df}etab\u{9}separated\u{1}stra\u{df}e\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}stra\u{df}e\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}tab\u{9}separated\
\u{1}tab\u{9}separated  padded  \u{1}tab\u{9}separated{name} has {count} items\u{1}tab\u{9}separateda:b:c\
\u{1}tab\u{9}separateda..b\u{1}tab\u{9}separateda\u{1f600}b\u{1f600}c\u{1}tab\u{9}separatedab\u{1}tab\u{9}separatedabc\
\u{1}tab\u{9}separatedemoji \u{1f44d}\u{1f3fd} mix\u{1}tab\u{9}separatedHELLO\u{1}tab\u{9}separatedhello world\u{1}tab\u{9}separated\u{130}stanbul\
\u{1}tab\u{9}separatedtrailing.\u{1}tab\u{9}separated\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}The recap window must start before it ends.\
\u{1}trailing.\u{1}trailing.  padded  \u{1}trailing.{name} has {count} items\u{1}trailing.\u{1f600}\u{1}trailing.a:b:c\
\u{1}trailing.a..b\u{1}trailing.a.b.c\u{1}trailing.a\u{1f600}b\u{1f600}c\u{1}trailing.hello\u{1}trailing.\u{130}stanbul\
\u{1}trailing.key_name-1\u{1}trailing.src/a/b.ts\u{1}trailing.\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}trailing.\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}turnRunSpan\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}  padded  \u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}.leading\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}{name} has {count} items\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a.b.c\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a\u{1f600}b\u{1f600}c\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}ab\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}abc\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}emoji \u{1f44d}\u{1f3fd} mix\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}hello\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}hello world\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}Hello World\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}stra\u{df}e\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}tab\u{9}separated\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}trailing.\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}unknown\u{1}x\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}  padded  \u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}.leading\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a..b\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a\u{1f600}b\u{1f600}c\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{feff}bom\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}e\u{301}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hello\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}Hello World\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{130}stanbul\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}ns:key.path\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}src/a/b.ts\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}trailing.\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}  padded  \u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}:\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}{name} has {count} items\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1f600}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a.b.c\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}ab\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{feff}bom\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}emoji \u{1f44d}\u{1f3fd} mix\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}HELLO\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}key_name-1\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}line1\u{a}line2\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}stra\u{df}e\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}";

/// Strings the recorded corpus feeds to `Date.parse` that JavaScriptCore
/// accepts outside the ISO subset, mapped to their millisecond result.
const DATE_MS: &[(&str, f64)] = &[
    ("padded  key_name-1", 978336000000.0),
    ("-0", 946713600000.0),
    ("./x/**/*.jsonkey_name-1", 978336000000.0),
    (".leadingkey_name-1", 978336000000.0),
    (":key_name-1", 978336000000.0),
    ("Hello Worldkey_name-1", 978336000000.0),
    ("a..bkey_name-1", 978336000000.0),
    ("a.b.ckey_name-1", 978336000000.0),
    ("a:b:ckey_name-1", 978336000000.0),
    ("abckey_name-1", 978336000000.0),
    ("emoji 👍🏽 mixkey_name-1", 978336000000.0),
    ("hello worldkey_name-1", 978336000000.0),
    ("hellokey_name-1", 978336000000.0),
    ("key_name-1", 978336000000.0),
    ("src/a/b.tskey_name-1", 978336000000.0),
    ("straßekey_name-1", 978336000000.0),
    ("trailing.key_name-1", 978336000000.0),
    ("日本語のテキストkey_name-1", 978336000000.0),
    ("😀key_name-1", 978336000000.0),
];

pub fn date_ms(s: &str) -> Option<f64> {
    DATE_MS.iter().find(|(k, _)| *k == s).map(|(_, v)| *v)
}
