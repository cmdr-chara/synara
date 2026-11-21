//! The collation and Date.parse results observed in the recorded corpus (V8).

/// Rank of a recorded string in `localeCompare` order.
pub fn rank(s: &str) -> Option<i64> {
    RANKED.split('\u{1}').position(|x| x == s).map(|i| i as i64)
}

const RANKED: &str = "\u{1} \u{1}  padded  \u{1}  padded  ./x/**/*.json\u{1}  padded  \u{1f600}\u{1}  padded  a:b:c\
\u{1}  padded  a..b\u{1}  padded  ab\u{1}  padded  e\u{301}\u{1}  padded  emoji \u{1f44d}\
\u{1f3fd} mix\u{1}  padded  key_name-1\u{1}  padded  line1\u{a}line2\u{1}  padded  stra\
\u{df}e\u{1}-0\u{1}-Infinity\u{1}:\u{1}:  padded  \u{1}::\u{1}:./x/**/*.json\u{1}:{name} has {count} items\
\u{1}:a..b\u{1}:a.b.c\u{1}:abc\u{1}:\u{feff}bom\u{1}:e\u{301}\u{1}:HELLO\u{1}:Hello World\
\u{1}:src/a/b.ts\u{1}:stra\u{df}e\u{1}:trailing.\u{1}:\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}:\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}:\u{65e5}\u{672c}\u{8a9e}\u{306e}\
\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}./x/**/*.json\u{1}./x/**/*.json  padded  \u{1}./x/**/*.json:\
\u{1}./x/**/*.json./x/**/*.json\u{1}./x/**/*.json.leading\u{1}./x/**/*.jsona..b\u{1}./x/**/*.jsona.b.c\
\u{1}./x/**/*.jsonab\u{1}./x/**/*.jsonabc\u{1}./x/**/*.jsone\u{301}\u{1}./x/**/*.jsonemoji \
\u{1f44d}\u{1f3fd} mix\u{1}./x/**/*.jsonhello\u{1}./x/**/*.jsonHELLO\u{1}./x/**/*.jsonHello World\
\u{1}./x/**/*.jsonkey_name-1\u{1}./x/**/*.jsonline1\u{a}line2\u{1}./x/**/*.jsonsrc/a/b.ts\
\u{1}./x/**/*.jsonstra\u{df}e\u{1}./x/**/*.jsontab\u{9}separated\u{1}./x/**/*.json\u{65e5}\
\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}.leading\u{1}.leading./x/**/*.json\
\u{1}.leading.leading\u{1}.leading{name} has {count} items\u{1}.leadingab\u{1}.leadinghello world\
\u{1}.leadingHello World\u{1}.leading\u{130}stanbul\u{1}.leadingline1\u{a}line2\u{1}.leadingns:key.path\
\u{1}.leadingstra\u{df}e\u{1}.leadingtrailing.\u{1}.leading\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}.leading\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}{name} has {count} items\
\u{1}{name} has {count} items./x/**/*.json\u{1}{name} has {count} items.leading\u{1}{name} has {count} itemsa\
\u{1}{name} has {count} itemsa..b\u{1}{name} has {count} itemsa.b.c\u{1}{name} has {count} itemsa\
\u{1f600}b\u{1f600}c\u{1}{name} has {count} itemse\u{301}\u{1}{name} has {count} itemsHELLO\
\u{1}{name} has {count} itemshello world\u{1}{name} has {count} itemsHello World\u{1}{name} has {count} itemsline1\
\u{a}line2\u{1}{name} has {count} itemsns:key.path\u{1}{name} has {count} items\u{fc}n\
\u{ef}c\u{f6}d\u{e9}\u{1}{name} has {count} items\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\
\u{3a3}\u{1}{name} has {count} items\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\
\u{30c8}\u{1}\u{1f44d}\u{1f3fd} thumbs\u{1}\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} family\
\u{1}\u{1f600}\u{1}\u{1f600}  padded  \u{1}\u{1f600}./x/**/*.json\u{1}\u{1f600}.leading\
\u{1}\u{1f600}\u{1f600}\u{1}\u{1f600}a:b:c\u{1}\u{1f600}\u{feff}bom\u{1}\u{1f600}HELLO\
\u{1}\u{1f600}hello world\u{1}\u{1f600}line1\u{a}line2\u{1}\u{1f600}stra\u{df}e\u{1}\u{1f600}tab\
\u{9}separated\u{1}\u{1f600}trailing.\u{1}\u{1f600}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\
\u{39f}\u{3a3}\u{1}$num\u{1}$set\u{1}\u{1d7d8}\u{1d7d9} \u{b2}\u{b3}\u{1}\u{661}\u{662}\
\u{663} \u{664}\u{665}\u{1}a\u{1}a\u{a0}b\u{1}a\u{a0}b\u{2009}c\u{3000}d\u{1}a:b:c\u{1}a:b:ca..b\
\u{1}a:b:ce\u{301}\u{1}a:b:cemoji \u{1f44d}\u{1f3fd} mix\u{1}a:b:chello\u{1}a:b:chello world\
\u{1}a:b:c\u{130}stanbul\u{1}a:b:csrc/a/b.ts\u{1}a:b:ctab\u{9}separated\u{1}a:b:c\u{fc}n\
\u{ef}c\u{f6}d\u{e9}\u{1}a:b:c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\
\u{30c8}\u{1}a..b\u{1}a..b  padded  \u{1}a..b:\u{1}a..b./x/**/*.json\u{1}a..b{name} has {count} items\
\u{1}a..b\u{1f600}\u{1}a..ba\u{1}a..bab\u{1}a..babc\u{1}a..b\u{feff}bom\u{1}a..bemoji \
\u{1f44d}\u{1f3fd} mix\u{1}a..bHELLO\u{1}a..bline1\u{a}line2\u{1}a..bsrc/a/b.ts\u{1}a..bstra\
\u{df}e\u{1}a..b\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}a.b.c\u{1}a.b.c.leading\
\u{1}a.b.ca:b:c\u{1}a.b.ca..b\u{1}a.b.ca.b.c\u{1}a.b.ca\u{1f600}b\u{1f600}c\u{1}a.b.cabc\
\u{1}a.b.c\u{feff}bom\u{1}a.b.ce\u{301}\u{1}a.b.cHello World\u{1}a.b.c\u{130}stanbul\u{1}a.b.cns:key.path\
\u{1}a.b.c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}a.b.c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\
\u{3a3}\u{1}a\u{1f600}\u{1}a\u{1f600}b\u{1}a\u{1f600}b\u{1f600}c\u{1}a\u{1f600}b\u{1f600}c  padded  \
\u{1}a\u{1f600}b\u{1f600}c:\u{1}a\u{1f600}b\u{1f600}c{name} has {count} items\u{1}a\u{1f600}b\
\u{1f600}ca\u{1}a\u{1f600}b\u{1f600}ca:b:c\u{1}a\u{1f600}b\u{1f600}ca.b.c\u{1}a\u{1f600}b\
\u{1f600}ca\u{1f600}b\u{1f600}c\u{1}a\u{1f600}b\u{1f600}cabc\u{1}a\u{1f600}b\u{1f600}ce\
\u{301}\u{1}a\u{1f600}b\u{1f600}cemoji \u{1f44d}\u{1f3fd} mix\u{1}a\u{1f600}b\u{1f600}c\
\u{130}stanbul\u{1}a\u{1f600}b\u{1f600}cns:key.path\u{1}a\u{b2}b x\u{bd} y\u{2460}\u{1}aa\
\u{1}aa..b\u{1}aa.b.c\u{1}aa\u{1f600}b\u{1f600}c\u{1}aab\u{1}aabc\u{1}ab\u{1}ab:\u{1}ab\
\u{1f600}\u{1}aba:b:c\u{1}aba..b\u{1}abab\u{1}ababc\u{1}abc\u{1}abc  padded  \u{1}abc\u{1f600}\
\u{1}abca\u{1}abca.b.c\u{1}abca\u{1f600}b\u{1f600}c\u{1}abcab\u{1}abc\u{feff}bom\u{1}abce\
\u{301}\u{1}abchello\u{1}abchello world\u{1}abcHello World\u{1}abc\u{130}stanbul\u{1}abcns:key.path\
\u{1}abcstra\u{df}e\u{1}abctrailing.\u{1}abc\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\
\u{3a3}\u{1}abemoji \u{1f44d}\u{1f3fd} mix\u{1}abhello\u{1}ab\u{130}stanbul\u{1}abkey_name-1\
\u{1}abstra\u{df}e\u{1}abtab\u{9}separated\u{1}abtrailing.\u{1}ab\u{fc}n\u{ef}c\u{f6}d\
\u{e9}\u{1}ae\u{301}\u{1}aemoji \u{1f44d}\u{1f3fd} mix\u{1}aHello World\u{1}a\u{2160}b \
\u{216b}\u{1}akey_name-1\u{1}args\u{1}astra\u{df}e\u{1}atab\u{9}separated\u{1}a\u{fc}n\
\u{ef}c\u{f6}d\u{e9}\u{1}a\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}a\u{10400}b \
\u{1d400}bc\u{1}b\u{1}\u{feff}bom\u{1}\u{feff}bom:\u{1}\u{feff}bom./x/**/*.json\u{1}\u{feff}bom\
\u{1f600}\u{1}\u{feff}bome\u{301}\u{1}\u{feff}bomhello\u{1}\u{feff}bomHello World\u{1}\
\u{feff}bomkey_name-1\u{1}\u{feff}bomline1\u{a}line2\u{1}\u{feff}bomns:key.path\u{1}\u{feff}bomtab\
\u{9}separated\u{1}\u{feff}bomtrailing.\u{1}\u{feff}bom\u{65e5}\u{672c}\u{8a9e}\u{306e}\
\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}buildTranscriptRows\u{1}buildUsageAnalytics\u{1}c\
\u{1}cacheTokens\u{1}cacheWriteTokens\u{1}cafe\u{301} caf\u{e9}\u{1}Cannot read properties of null (reading 'from')\
\u{1}Cannot read properties of null (reading 'sessionFiles')\u{1}Cannot read properties of null (reading 'some')\
\u{1}Cannot read properties of null (reading 'splitTokens')\u{1}Cannot read properties of null (reading 'window')\
\u{1}Cannot read properties of undefined (reading 'from')\u{1}costRows is not iterable\
\u{1}costs\u{1}costUsd\u{1}\u{1c5}ungla \u{1c4}\u{1}E\u{1}e\u{301}\u{1}e\u{301}:\u{1}e\
\u{301}./x/**/*.json\u{1}e\u{301}{name} has {count} items\u{1}e\u{301}\u{1f600}\u{1}\u{e9}a..\
\u{1}e\u{301}a.b.c\u{1}e\u{301}ab\u{1}e\u{301}abc\u{1}e\u{301}\u{feff}bom\u{1}e\u{301}e\
\u{301}\u{1}e\u{301}hello\u{1}e\u{301}hello world\u{1}e\u{301}Hello World\u{1}e\u{301}\
\u{130}stanbul\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{1}emoji \u{1f44d}\u{1f3fd} mix  padded  \
\u{1}emoji \u{1f44d}\u{1f3fd} mix./x/**/*.json\u{1}emoji \u{1f44d}\u{1f3fd} mixa:b:c\u{1}emoji \
\u{1f44d}\u{1f3fd} mixa..b\u{1}emoji \u{1f44d}\u{1f3fd} mixa.b.c\u{1}emoji \u{1f44d}\u{1f3fd} mixe\
\u{301}\u{1}emoji \u{1f44d}\u{1f3fd} mixhello\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{130}stanbul\
\u{1}emoji \u{1f44d}\u{1f3fd} mixline1\u{a}line2\u{1}emoji \u{1f44d}\u{1f3fd} mixsrc/a/b.ts\
\u{1}emoji \u{1f44d}\u{1f3fd} mixtrailing.\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{3a3}\u{38a}\
\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}error\u{1}estimateCost\u{1}e\u{301}tab\u{9}separated\
\u{1}e\u{301}trailing.\u{1}e\u{301}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}e\u{301}\u{3a3}\u{38a}\
\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}e\u{301}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\
\u{30ad}\u{30b9}\u{30c8}\u{1}fetchedAt\u{1}file is not iterable\u{1}files\u{1}fn\u{1}from\
\u{1}hello\u{1}HELLO\u{1}HELLO  padded  \u{1}hello world\u{1}Hello World\u{1}h\u{e9}llo w\
\u{f6}rld\u{1}Hello World:\u{1}hello world./x/**/*.json\u{1}Hello World./x/**/*.json\u{1}hello world.leading\
\u{1}hello world{name} has {count} items\u{1}Hello World{name} has {count} items\u{1}Hello Worlda\
\u{1}hello worlda.b.c\u{1}hello worlda\u{1f600}b\u{1f600}c\u{1}Hello Worlda\u{1f600}b\u{1f600}c\
\u{1}Hello Worldab\u{1}hello worlde\u{301}\u{1}Hello Worlde\u{301}\u{1}hello worldemoji \
\u{1f44d}\u{1f3fd} mix\u{1}hello worldHELLO\u{1}hello worldhello world\u{1}hello worldHello World\
\u{1}Hello Worldhello world\u{1}hello world\u{130}stanbul\u{1}hello worldkey_name-1\u{1}Hello Worldkey_name-1\
\u{1}Hello Worldline1\u{a}line2\u{1}hello worldns:key.path\u{1}Hello Worldns:key.path\u{1}hello worldsrc/a/b.ts\
\u{1}Hello Worldsrc/a/b.ts\u{1}hello worldtab\u{9}separated\u{1}hello worldtrailing.\u{1}Hello Worldtrailing.\
\u{1}hello world\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}hello world\u{3a3}\u{38a}\u{3a3}\u{3a5}\
\u{3a6}\u{39f}\u{3a3}\u{1}hello world\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\
\u{30c8}\u{1}Hello World\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\
\u{1}hello.leading\u{1}HELLO.leading\u{1}hello{name} has {count} items\u{1}HELLO{name} has {count} items\
\u{1}hello\u{1f600}\u{1}HELLO\u{1f600}\u{1}HELLOa\u{1}helloa.b.c\u{1}HELLOa.b.c\u{1}HELLOab\
\u{1}helloabc\u{1}HELLO\u{feff}bom\u{1}helloemoji \u{1f44d}\u{1f3fd} mix\u{1}hellohello\
\u{1}helloHello World\u{1}HELLOHello World\u{1}hello\u{130}stanbul\u{1}HELLO\u{130}stanbul\
\u{1}helloline1\u{a}line2\u{1}hellons:key.path\u{1}hellosrc/a/b.ts\u{1}HELLOsrc/a/b.ts\
\u{1}hellotrailing.\u{1}HELLO\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}hello\u{3a3}\u{38a}\u{3a3}\
\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}HELLO\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}hello\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}hour\u{1}I\
\u{131} \u{130}i\u{1}Infinity\u{1}\u{130}stanbul\u{1}\u{130}stanbul:\u{1}\u{130}stanbula\
\u{1}\u{130}stanbula:b:c\u{1}\u{130}stanbula..b\u{1}\u{130}stanbula.b.c\u{1}\u{130}stanbulabc\
\u{1}\u{130}stanbule\u{301}\u{1}\u{130}stanbulemoji \u{1f44d}\u{1f3fd} mix\u{1}\u{130}stanbulns:key.path\
\u{1}\u{130}stanbul\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{130}stanbul\u{3a3}\u{38a}\u{3a3}\
\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{131}l\u{131}k \u{131}rmak\u{1}K \u{212a} k\u{1}key_name-1\
\u{1}key_name-1./x/**/*.json\u{1}key_name-1a\u{1}key_name-1abc\u{1}key_name-1\u{feff}bom\
\u{1}key_name-1e\u{301}\u{1}key_name-1hello\u{1}key_name-1\u{130}stanbul\u{1}key_name-1ns:key.path\
\u{1}key_name-1src/a/b.ts\u{1}key_name-1tab\u{9}separated\u{1}key_name-1trailing.\u{1}key_name-1\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}key_name-1\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}line1\u{a}line2\u{1}line1\u{a}line2.leading\u{1}line1\u{a}line2a:b:c\u{1}line1\u{a}line2a..b\
\u{1}line1\u{a}line2a\u{1f600}b\u{1f600}c\u{1}line1\u{a}line2ab\u{1}line1\u{a}line2emoji \
\u{1f44d}\u{1f3fd} mix\u{1}line1\u{a}line2hello\u{1}line1\u{a}line2HELLO\u{1}line1\u{a}line2line1\
\u{a}line2\u{1}line1\u{a}line2ns:key.path\u{1}line1\u{a}line2trailing.\u{1}line1\u{a}line2\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}model\u{1}NaN\u{1}needsPrices\u{1}ns:key.path\
\u{1}ns:key.path  padded  \u{1}ns:key.path:\u{1}ns:key.path./x/**/*.json\u{1}ns:key.path{name} has {count} items\
\u{1}ns:key.path\u{1f600}\u{1}ns:key.patha..b\u{1}ns:key.pathab\u{1}ns:key.pathemoji \u{1f44d}\
\u{1f3fd} mix\u{1}ns:key.pathkey_name-1\u{1}ns:key.pathline1\u{a}line2\u{1}ns:key.pathns:key.path\
\u{1}ns:key.pathsrc/a/b.ts\u{1}ok\u{1}one\u{d}\u{a}two\u{2028}three\u{85}four\u{1}origin\
\u{1}outputTokens\u{1}path\u{1}prices\u{1}project\u{1}projectId\u{1}projectRoots\u{1}projects\
\u{1}provider\u{1}providers\u{1}reporting.has is not a function\u{1}reportingProviders\
\u{1}resolveUsageAnalyticsWindow\u{1}\u{17f} s S\u{1}scan\u{1}snake_case foo-bar o'neil\
\u{1}splitCacheTokens\u{1}splitTokens\u{1}src/a/b.ts\u{1}src/a/b.ts./x/**/*.json\u{1}src/a/b.ts.leading\
\u{1}src/a/b.ts{name} has {count} items\u{1}src/a/b.ts\u{1f600}\u{1}src/a/b.tsa..b\u{1}src/a/b.tsa.b.c\
\u{1}src/a/b.tsa\u{1f600}b\u{1f600}c\u{1}src/a/b.tsemoji \u{1f44d}\u{1f3fd} mix\u{1}src/a/b.tshello world\
\u{1}src/a/b.tsHello World\u{1}src/a/b.tskey_name-1\u{1}src/a/b.tsline1\u{a}line2\u{1}src/a/b.tssrc/a/b.ts\
\u{1}src/a/b.tstrailing.\u{1}status\u{1}stra\u{df}e\u{1}stra\u{df}e:\u{1}stra\u{df}e./x/**/*.json\
\u{1}stra\u{df}e.leading\u{1}stra\u{df}e\u{1f600}\u{1}stra\u{df}ea\u{1}stra\u{df}ea:b:c\
\u{1}stra\u{df}ea..b\u{1}stra\u{df}ea.b.c\u{1}stra\u{df}eabc\u{1}stra\u{df}e\u{feff}bom\
\u{1}stra\u{df}ee\u{301}\u{1}stra\u{df}eHello World\u{1}stra\u{df}e\u{130}stanbul\u{1}stra\
\u{df}ekey_name-1\u{1}stra\u{df}esrc/a/b.ts\u{1}stra\u{df}estra\u{df}e\u{1}stra\u{df}etab\
\u{9}separated\u{1}stra\u{df}etrailing.\u{1}stra\u{df}e\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\
\u{39f}\u{3a3}\u{1}stra\u{df}e\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\
\u{30c8}\u{1}tab\u{9}separated\u{1}tab\u{9}separated  padded  \u{1}tab\u{9}separated.leading\
\u{1}tab\u{9}separated{name} has {count} items\u{1}tab\u{9}separated\u{1f600}\u{1}tab\u{9}separateda:b:c\
\u{1}tab\u{9}separateda.b.c\u{1}tab\u{9}separateda\u{1f600}b\u{1f600}c\u{1}tab\u{9}separatede\
\u{301}\u{1}tab\u{9}separatedemoji \u{1f44d}\u{1f3fd} mix\u{1}tab\u{9}separatedhello\u{1}tab\
\u{9}separatedHELLO\u{1}tab\u{9}separatedhello world\u{1}tab\u{9}separatedHello World\u{1}tab\
\u{9}separated\u{130}stanbul\u{1}tab\u{9}separatedline1\u{a}line2\u{1}tab\u{9}separatedstra\
\u{df}e\u{1}tab\u{9}separated\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}table\
\u{1}The analytics window must start before it ends.\u{1}title\u{1}to\u{1}tokenRows.some is not a function\
\u{1}tokens\u{1}trailing.\u{1}trailing.  padded  \u{1}trailing.:\u{1}trailing../x/**/*.json\
\u{1}trailing..leading\u{1}trailing.{name} has {count} items\u{1}trailing.a.b.c\u{1}trailing.e\
\u{301}\u{1}trailing.hello\u{1}trailing.Hello World\u{1}trailing.src/a/b.ts\u{1}trailing.\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}trailing.\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}turns\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}.leading\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}{name} has {count} items\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1f600}\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a:b:c\u{1}\u{fc}n\u{ef}c\
\u{f6}d\u{e9}a..b\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}abc\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}e\u{301}\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}HELLO\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}hello world\u{1}\u{fc}n\
\u{ef}c\u{f6}d\u{e9}line1\u{a}line2\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}src/a/b.ts\u{1}\u{fc}n\
\u{ef}c\u{f6}d\u{e9}stra\u{df}e\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}tab\u{9}separated\u{1}\u{fc}n\
\u{ef}c\u{f6}d\u{e9}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{3a3}\
\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{65e5}\u{672c}\
\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}unknown\u{1}unpricedTokens\u{1}unsplitTokens\
\u{1}window\u{1}x\u{1}x\u{200d}y \u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{1}\u{3a3}\
\u{391}\u{3a3} \u{3c3}\u{3b1}\u{3c2} \u{3c2}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\
\u{3a3}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}:\u{1}\u{3a3}\u{38a}\u{3a3}\
\u{3a5}\u{3a6}\u{39f}\u{3a3}./x/**/*.json\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\
\u{3a3}.leading\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}{name} has {count} items\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1f600}\u{1}\u{3a3}\u{38a}\u{3a3}\
\u{3a5}\u{3a6}\u{39f}\u{3a3}a\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}abc\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}e\u{301}\u{1}\u{3a3}\u{38a}\u{3a3}\
\u{3a5}\u{3a6}\u{39f}\u{3a3}emoji \u{1f44d}\u{1f3fd} mix\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\
\u{3a6}\u{39f}\u{3a3}HELLO\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{130}stanbul\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}tab\u{9}separated\u{1}\u{3a3}\u{38a}\
\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}trailing.\u{1}\u{915}\u{93f} \u{663}\u{664}\u{1}\u{65e5}\
\u{672c}\u{8a9e}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}:\u{1}\u{65e5}\u{672c}\
\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1f600}\u{1}\u{65e5}\u{672c}\u{8a9e}\
\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a..b\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\
\u{30ad}\u{30b9}\u{30c8}a\u{1f600}b\u{1f600}c\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\
\u{30ad}\u{30b9}\u{30c8}ab\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\
\u{30c8}e\u{301}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}emoji \
\u{1f44d}\u{1f3fd} mix\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello world\u{1}\u{65e5}\
\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}Hello World\u{1}\u{65e5}\u{672c}\
\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}ns:key.path\u{1}\u{65e5}\u{672c}\u{8a9e}\
\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}src/a/b.ts\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\
\u{30c6}\u{30ad}\u{30b9}\u{30c8}trailing.\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\
\u{30ad}\u{30b9}\u{30c8}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}";

const DATE_MS: &[(&str, i64)] = &[
    ("-0", 946713600000),
    ("./x/**/*.jsonkey_name-1", 978336000000),
    ("Hello Worldkey_name-1", 978336000000),
    ("abkey_name-1", 978336000000),
    ("akey_name-1", 978336000000),
    ("bomkey_name-1", 978336000000),
    ("hello worldkey_name-1", 978336000000),
    ("key_name-1", 978336000000),
    ("ns:key.pathkey_name-1", 978336000000),
    ("padded  key_name-1", 978336000000),
    ("src/a/b.tskey_name-1", 978336000000),
    ("stra\u{df}ekey_name-1", 978336000000),
];

/// `Date.parse` results for recorded strings the ISO subset parser cannot read.
pub fn date_ms(s: &str) -> Option<f64> {
    DATE_MS
        .iter()
        .find(|(k, _)| *k == s)
        .map(|(_, v)| *v as f64)
}
