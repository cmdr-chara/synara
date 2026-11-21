//! The `localeCompare` order observed in the recorded corpus (Bun/ICU root).

/// Rank of a recorded string in `localeCompare` order.
pub fn rank(s: &str) -> Option<i64> {
    RANKED.split('\u{1}').position(|x| x == s).map(|i| i as i64)
}

const RANKED: &str = "\u{1}\u{301}\u{1} \u{1}  padded  \u{1}  padded  :\u{1}  padded  ./x/**/*.json\
\u{1}  padded  {name} has {count} items\u{1}  padded  \u{1f600}\u{1}  padded  a\u{1}  padded  a..b\
\u{1}  padded  a.b.c\u{1}  padded  a\u{1f600}b\u{1f600}c\u{1}  padded  ab\u{1}  padded  abc\
\u{1}  padded  \u{feff}bom\u{1}  padded  emoji \u{1f44d}\u{1f3fd} mix\u{1}  padded  HELLO\
\u{1}  padded  hello world\u{1}  padded  Hello World\u{1}  padded  \u{130}stanbul\
\u{1}  padded  line1\u{a}line2\u{1}  padded  ns:key.path\u{1}  padded  src/a/b.ts\
\u{1}  padded  tab\u{9}separated\u{1}  padded  \u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1} has {count} items.\u{1}_\u{1}-0\u{1}-1\u{1}-Infinity\u{1}:\u{1}:  padded  \u{1}:./x/**/*.json\
\u{1}:{name} has {count} items\u{1}:\u{1f600}\u{1}:a:b:c\u{1}:a\u{1f600}b\u{1f600}c\u{1}:abc\u{1}:c\u{1}:hello\
\u{1}:Hello World\u{1}:ke\u{1}:key_name-1\u{1}:line1\u{a}line2\u{1}:ns:key.path\u{1}:stra\u{df}e\
\u{1}:tab\u{9}separated\u{1}:\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}:\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}:\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}.\u{1}./\u{1}./x/**/*.json\
\u{1}./x/**/*.json  padded  \u{1}./x/**/*.json./x/**/*.json\u{1}./x/**/*.json.leading\
\u{1}./x/**/*.json{name} has {count} items\u{1}./x/**/*.json\u{1f600}\u{1}./x/**/*.jsona:b:c\
\u{1}./x/**/*.jsona..b\u{1}./x/**/*.jsona\u{1f600}b\u{1f600}c\u{1}./x/**/*.jsone\u{301}\
\u{1}./x/**/*.jsonHello World\u{1}./x/**/*.json\u{130}stanbul\u{1}./x/**/*.jsonkey_name-1\
\u{1}./x/**/*.jsonline1\u{a}line2\u{1}./x/**/*.jsonsrc/a/b.ts\u{1}./x/**/*.jsonstra\u{df}e\
\u{1}./x/**/*.jsontab\u{9}separated\u{1}./x/**/*.jsontrailing.\u{1}./x/**/*.json\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}.b.\u{1}.leading\u{1}.leading  padded  \u{1}.leading:\u{1}.leading.leading\
\u{1}.leading{name} has {count} items\u{1}.leadinga.b.c\u{1}.leadinga\u{1f600}b\u{1f600}c\u{1}.leadinge\u{301}\
\u{1}.leadinghello world\u{1}.leadingHello World\u{1}.leading\u{130}stanbul\u{1}.leadingstra\u{df}e\
\u{1}.leadingtrailing.\u{1}.leading\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}{\u{1}{name} has {count} items\
\u{1}{name} has {count} items  padded  \u{1}{name} has {count} items./x/**/*.json\
\u{1}{name} has {count} items.leading\u{1}{name} has {count} items{name} has {count} items\
\u{1}{name} has {count} items\u{1f600}\u{1}{name} has {count} itemsa\u{1}{name} has {count} itemsa:b:c\
\u{1}{name} has {count} itemsa..b\u{1}{name} has {count} itemsa\u{1f600}b\u{1f600}c\
\u{1}{name} has {count} itemsab\u{1}{name} has {count} items\u{feff}bom\
\u{1}{name} has {count} itemsemoji \u{1f44d}\u{1f3fd} mix\u{1}{name} has {count} itemshello\
\u{1}{name} has {count} items\u{130}stanbul\u{1}{name} has {count} itemsline1\u{a}line2\
\u{1}{name} has {count} itemsstra\u{df}e\u{1}{name} has {count} itemstab\u{9}separated\
\u{1}{name} has {count} itemstrailing.\u{1}{name} has {count} items\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}**/*\
\u{1}/B.TSKEY_NAME-\u{1}\u{1f44d}\u{1f3fd} thumbs\u{1}\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} family\
\u{1}\u{1f600}\u{1}\u{1f600}  padded  \u{1}\u{1f600}:\u{1}\u{1f600}./x/**/*.json\
\u{1}\u{1f600}{name} has {count} items\u{1}\u{1f600}\u{1f600}\u{1}\u{1f600}a\u{1}\u{1f600}a.b.c\
\u{1}\u{1f600}a\u{1f600}b\u{1f600}c\u{1}\u{1f600}B\u{1f600}C\u{1}\u{1f600}e\u{301}\u{1}\u{1f600}hello\
\u{1}\u{1f600}HELLO\u{1}\u{1f600}\u{130}stanbul\u{1}\u{1f600}key_name-1\u{1}\u{1f600}src/a/b.ts\
\u{1}\u{1f600}trailing.\u{1}\u{1f600}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}\u{1f600}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}0s\u{1}1\u{a}\
\u{1}104249991d 8h\u{1}11574074074074d 1h\u{1}16m 40s\u{1}1ms\u{1}1s\u{1}2.0806633505350875e+300d 7h\
\u{1}24d 20h\u{1}a\u{1}A\u{1}a  padded  \u{1}a\u{a0}b\u{1}a:b:c\u{1}a:b:c:\u{1}a:b:c.leading\
\u{1}a:b:c{name} has {count} items\u{1}a:b:c\u{1f600}\u{1}a:b:ca\u{1}a:b:ca:b:c\u{1}a:b:ca..b\
\u{1}a:b:ca\u{1f600}b\u{1f600}c\u{1}a:b:c\u{feff}bom\u{1}a:b:cemoji \u{1f44d}\u{1f3fd} mix\u{1}a:b:chello\
\u{1}a:b:chello world\u{1}a:b:csrc/a/b.ts\u{1}a:b:ctab\u{9}separated\u{1}a:b:c\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}a:b:c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}a:b:c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a..b\u{1}a..b  padded  \
\u{1}a..b:\u{1}a..b./x/**/*.json\u{1}a..ba:b:c\u{1}a..ba.b.c\u{1}a..ba\u{1f600}b\u{1f600}c\u{1}a..bab\
\u{1}a..babc\u{1}a..be\u{301}\u{1}a..bemoji \u{1f44d}\u{1f3fd} mix\u{1}a..bhello world\u{1}a..bHello World\
\u{1}a..bkey_name-1\u{1}a..bns:key.path\u{1}a..bstra\u{df}e\u{1}a..btrailing.\
\u{1}a..b\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}a..b\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}a..b\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a./x/**/*.json\u{1}a.b.c\
\u{1}a.b.c:\u{1}a.b.c{name} has {count} items\u{1}a.b.c\u{1f600}\u{1}a.b.ca..b\u{1}a.b.ca.b.c\
\u{1}a.b.ca\u{1f600}b\u{1f600}c\u{1}a.b.cab\u{1}a.b.ckey_name-1\u{1}a.b.cns:key.path\u{1}a.b.csrc/a/b.ts\
\u{1}a.b.ctab\u{9}separated\u{1}a.b.ctrailing.\u{1}a.b.c\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}a.b.c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a.leading\
\u{1}a{name} has {count} items\u{1}a\u{1f600}b\u{1}a\u{1f600}b\u{1f600}c\
\u{1}a\u{1f600}b\u{1f600}c./x/**/*.json\u{1}a\u{1f600}b\u{1f600}c.leading\
\u{1}a\u{1f600}b\u{1f600}c{name} has {count} items\u{1}a\u{1f600}b\u{1f600}c\u{1f600}\
\u{1}a\u{1f600}b\u{1f600}ca:b:c\u{1}a\u{1f600}b\u{1f600}ca..b\u{1}a\u{1f600}b\u{1f600}cabc\
\u{1}a\u{1f600}b\u{1f600}ce\u{301}\u{1}a\u{1f600}b\u{1f600}chello\u{1}a\u{1f600}b\u{1f600}cHELLO\
\u{1}a\u{1f600}b\u{1f600}c\u{130}stanbul\u{1}a\u{1f600}b\u{1f600}ckey_name-1\
\u{1}a\u{1f600}b\u{1f600}cline1\u{a}line2\u{1}a\u{1f600}b\u{1f600}c\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}a\u{1f600}b\u{1f600}c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}a\u{1f600}b\u{1f600}c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\
\u{1}a\u{b2}b x\u{bd} y\u{2460}\u{1}aa..b\u{1}aab\u{1}ab\u{1}ab./x/**/*.json\u{1}ab.leading\
\u{1}ab{name} has {count} items\u{1}aba..b\u{1}aba.b.c\u{1}ab\u{feff}bom\u{1}abc\u{1}abc.leading\
\u{1}abc{name} has {count} items\u{1}abc\u{1f600}\u{1}abca:b:c\u{1}abca..b\u{1}abcab\u{1}abc\u{feff}bom\
\u{1}abce\u{301}\u{1}abcemoji \u{1f44d}\u{1f3fd} mix\u{1}abcHELLO\u{1}abchello world\u{1}abcHello World\
\u{1}abc\u{130}stanbul\u{1}abcline1\u{a}line2\u{1}abcns:key.path\u{1}abcsrc/a/b.ts\u{1}abctrailing.\
\u{1}abc\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}abc\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}abe\u{301}\
\u{1}abhello\u{1}abHELLO\u{1}abhello world\u{1}abHello World\u{1}abkey_name-1\u{1}absrc/a/b.ts\u{1}abtrailing.\
\u{1}ab\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}ab\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}adin\
\u{1}aemoji \u{1f44d}\u{1f3fd} mix\u{1}aHELLO\u{1}aHello World\u{1}a\u{2160}b \u{216b}\u{1}a\u{130}stanbul\
\u{1}akey_name-1\u{1}anb\u{1}ans:key.path\u{1}asrc/a/b.ts\u{1}atrailing.\u{1}a\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}a\u{10400}b \u{1d400}bc\u{1}a\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}b\u{1}B\
\u{1}b.c\u{1}bc\u{1}\u{feff}bo\u{1}BO\u{1}\u{feff}bom\u{1}\u{feff}bom./x/**/*.json\u{1}\u{feff}bom.leading\
\u{1}\u{feff}bom\u{1f600}\u{1}\u{feff}boma\u{1}\u{feff}boma:b:c\u{1}\u{feff}boma.b.c\u{1}\u{feff}bomab\
\u{1}\u{feff}bomabc\u{1}\u{feff}bomhello\u{1}\u{feff}bomHELLO\u{1}\u{feff}bom\u{130}stanbul\
\u{1}\u{feff}bomkey_name-1\u{1}\u{feff}bomstra\u{df}e\u{1}\u{feff}bomtab\u{9}separated\
\u{1}\u{feff}bom\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}c\u{1}closed\
\u{1}connecting\u{1}d\u{1}dd\u{1}ded  \u{1}d\u{e9}e\u{1}disconnected\u{1}e\u{301}\u{1}e\u{301}  padded  \
\u{1}e\u{301}:\u{1}e\u{301}./x/**/*.json\u{1}e\u{301}{name} has {count} items\u{1}e\u{301}\u{1f600}\
\u{1}e\u{301}a:b:c\u{1}e\u{301}a..b\u{1}e\u{301}a.b.c\u{1}e\u{301}a\u{1f600}b\u{1f600}c\u{1}e\u{301}ab\
\u{1}e\u{301}abc\u{1}e\u{301}\u{feff}bom\u{1}ed\u{1}e\u{301}emoji \u{1f44d}\u{1f3fd} mix\u{1}E\u{301}HEL\
\u{1}e\u{301}hello\u{1}e\u{301}hello world\u{1}e\u{301}key_name-1\u{1}ELLO\u{1}ELLOkey_name-\
\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{1}emoji \u{1f44d}\u{1f3fd} mix  padded  \u{1}emoji \u{1f44d}\u{1f3fd} mix:\
\u{1}emoji \u{1f44d}\u{1f3fd} mix.leading\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{1f600}\
\u{1}emoji \u{1f44d}\u{1f3fd} mixa\u{1}emoji \u{1f44d}\u{1f3fd} mixa:b:c\u{1}emoji \u{1f44d}\u{1f3fd} mixa.b.c\
\u{1}emoji \u{1f44d}\u{1f3fd} mixabc\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{feff}bom\
\u{1}emoji \u{1f44d}\u{1f3fd} mixe\u{301}\u{1}emoji \u{1f44d}\u{1f3fd} mixemoji \u{1f44d}\u{1f3fd} mix\
\u{1}emoji \u{1f44d}\u{1f3fd} mixhello\u{1}emoji \u{1f44d}\u{1f3fd} mixHELLO\
\u{1}emoji \u{1f44d}\u{1f3fd} mixhello world\u{1}emoji \u{1f44d}\u{1f3fd} mixHello World\
\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{130}stanbul\u{1}emoji \u{1f44d}\u{1f3fd} mixtrailing.\
\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}error\
\u{1}e\u{301}src/a/b.ts\u{1}e\u{301}tab\u{9}separated\u{1}e\u{301}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}ey.pat\
\u{1}e\u{301}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}hello\u{1}HELLO\u{1}hello  padded  \
\u{1}HELLO  padded  \u{1}hello world\u{1}Hello World\u{1}h\u{e9}llo w\u{f6}rld\u{1}hello world  padded  \
\u{1}hello world:\u{1}Hello World:\u{1}hello world{name} has {count} items\
\u{1}Hello World{name} has {count} items\u{1}Hello Worlda\u{1}Hello Worlda:b:c\u{1}hello worlda..b\
\u{1}Hello Worlda..b\u{1}hello worlda.b.c\u{1}hello worlda\u{1f600}b\u{1f600}c\
\u{1}Hello Worlda\u{1f600}b\u{1f600}c\u{1}hello worldab\u{1}Hello Worldab\u{1}Hello Worldabc\
\u{1}hello world\u{feff}bom\u{1}hello worlde\u{301}\u{1}Hello Worlde\u{301}\
\u{1}hello worldemoji \u{1f44d}\u{1f3fd} mix\u{1}Hello Worldemoji \u{1f44d}\u{1f3fd} mix\u{1}hello worldHELLO\
\u{1}hello worldhello world\u{1}hello worldHello World\u{1}hello worldkey_name-1\u{1}Hello Worldkey_name-1\
\u{1}hello worldline1\u{a}line2\u{1}Hello Worldline1\u{a}line2\u{1}hello worldns:key.path\
\u{1}hello worldsrc/a/b.ts\u{1}Hello Worldsrc/a/b.ts\u{1}Hello Worldstra\u{df}e\
\u{1}hello worldtab\u{9}separated\u{1}Hello Worldtab\u{9}separated\u{1}Hello World\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}hello world\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}Hello World\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}hello world\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}HELLO:\u{1}hello.leading\
\u{1}HELLO.leading\u{1}hello{name} has {count} items\u{1}HELLO{name} has {count} items\u{1}HELLO\u{1f600}\
\u{1}helloa\u{1}HELLOa..b\u{1}helloa.b.c\u{1}helloab\u{1}helloabc\u{1}hello\u{feff}bom\u{1}HELLO\u{feff}bom\
\u{1}hellohello\u{1}helloHELLO\u{1}HELLOhello\u{1}HELLOHELLO\u{1}helloHello World\u{1}HELLOHello World\
\u{1}hello\u{130}stanbul\u{1}HELLOkey_name-1\u{1}helloline1\u{a}line2\u{1}hellons:key.path\
\u{1}HELLOns:key.path\u{1}HELLOsrc/a/b.ts\u{1}hellostra\u{df}e\u{1}hellotab\u{9}separated\
\u{1}HELLO\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}hello\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}HELLO\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}hello\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}in\u{1}ine\u{1}Infinity\
\u{1}Infinityd\u{1}\u{130}stanbul\u{1}\u{130}stanbul  padded  \u{1}\u{130}stanbul:\
\u{1}\u{130}stanbul./x/**/*.json\u{1}\u{130}stanbul\u{1f600}\u{1}\u{130}stanbula\u{1}\u{130}stanbula:b:c\
\u{1}\u{130}stanbula\u{1f600}b\u{1f600}c\u{1}\u{130}stanbulab\u{1}\u{130}stanbul\u{feff}bom\
\u{1}\u{130}stanbule\u{301}\u{1}\u{130}stanbulHELLO\u{1}\u{130}stanbulhello world\
\u{1}\u{130}stanbulHello World\u{1}\u{130}stanbulkey_name-1\u{1}\u{130}stanbulstra\u{df}e\
\u{1}\u{130}stanbul\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}\u{130}stanbul\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}ITEM\
\u{1}\u{131}l\u{131}k \u{131}rmak\u{1}key_name-1\u{1}key_name-1  padded  \u{1}key_name-1./x/**/*.json\
\u{1}key_name-1{name} has {count} items\u{1}key_name-1\u{1f600}\u{1}key_name-1a\u{1}key_name-1a:b:c\
\u{1}key_name-1a..b\u{1}key_name-1a\u{1f600}b\u{1f600}c\u{1}key_name-1ab\u{1}key_name-1e\u{301}\
\u{1}key_name-1emoji \u{1f44d}\u{1f3fd} mix\u{1}key_name-1\u{130}stanbul\u{1}key_name-1key_name-1\
\u{1}key_name-1src/a/b.ts\u{1}key_name-1stra\u{df}e\u{1}key_name-1tab\u{9}separated\u{1}key_name-1trailing.\
\u{1}l\u{1}LINE1\u{a}\u{1}line1\u{a}line2\u{1}line1\u{a}line2:\u{1}line1\u{a}line2./x/**/*.json\
\u{1}line1\u{a}line2.leading\u{1}line1\u{a}line2{name} has {count} items\u{1}line1\u{a}line2a\
\u{1}line1\u{a}line2a:b:c\u{1}line1\u{a}line2a..b\u{1}line1\u{a}line2abc\u{1}line1\u{a}line2hello\
\u{1}line1\u{a}line2hello world\u{1}line1\u{a}line2Hello World\u{1}line1\u{a}line2\u{130}stanbul\
\u{1}line1\u{a}line2ns:key.path\u{1}line1\u{a}line2src/a/b.ts\u{1}line1\u{a}line2stra\u{df}e\
\u{1}line1\u{a}line2tab\u{9}separated\u{1}line1\u{a}line2trailing.\
\u{1}line1\u{a}line2\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}llo W\u{1}LO\u{1}NaN\
\u{1}NaNm\u{1}ns:key.path\u{1}ns:key.path  padded  \u{1}ns:key.path:\u{1}ns:key.path{name} has {count} items\
\u{1}ns:key.path\u{1f600}\u{1}ns:key.patha\u{1}ns:key.patha:b:c\u{1}ns:key.patha..b\u{1}ns:key.patha.b.c\
\u{1}ns:key.pathab\u{1}ns:key.pathabc\u{1}ns:key.path\u{feff}bom\u{1}ns:key.pathe\u{301}\
\u{1}ns:key.pathemoji \u{1f44d}\u{1f3fd} mix\u{1}ns:key.pathhello\u{1}ns:key.pathHELLO\
\u{1}ns:key.pathhello world\u{1}ns:key.pathkey_name-1\u{1}ns:key.pathns:key.path\u{1}ns:key.pathsrc/a/b.ts\
\u{1}ns:key.pathtrailing.\u{1}o\u{1}O\u{1}\u{f6}d\u{e9}\u{1}om\u{1}ORL\u{1}PA\u{1}PADD\u{1}rc\u{1}ready\
\u{1}running\u{1}src/a/b.ts\u{1}src/a/b.ts  padded  \u{1}src/a/b.ts:\u{1}src/a/b.ts./x/**/*.json\
\u{1}src/a/b.ts.leading\u{1}src/a/b.ts{name} has {count} items\u{1}src/a/b.tsa\u{1}src/a/b.tsa.b.c\
\u{1}src/a/b.tsa\u{1f600}b\u{1f600}c\u{1}src/a/b.tsab\u{1}src/a/b.tsabc\u{1}src/a/b.ts\u{feff}bom\
\u{1}src/a/b.tse\u{301}\u{1}src/a/b.tsemoji \u{1f44d}\u{1f3fd} mix\u{1}src/a/b.tshello\u{1}src/a/b.tsHELLO\
\u{1}src/a/b.tshello world\u{1}src/a/b.tsHello World\u{1}src/a/b.ts\u{130}stanbul\u{1}src/a/b.tskey_name-1\
\u{1}src/a/b.tsline1\u{a}line2\u{1}src/a/b.tsns:key.path\u{1}src/a/b.tssrc/a/b.ts\u{1}src/a/b.tsstra\u{df}e\
\u{1}src/a/b.tstab\u{9}separated\u{1}src/a/b.tstrailing.\u{1}src/a/b.ts\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}src/a/b.ts\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}src/a/b.ts\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{df}\u{1}stra\u{df}e\
\u{1}stra\u{df}e  padded  \u{1}stra\u{df}e:\u{1}stra\u{df}ea:b:c\u{1}stra\u{df}ea\u{1f600}b\u{1f600}c\
\u{1}stra\u{df}eab\u{1}stra\u{df}ee\u{301}\u{1}stra\u{df}ehello\u{1}stra\u{df}eHELLO\
\u{1}stra\u{df}ehello world\u{1}stra\u{df}eHello World\u{1}stra\u{df}e\u{130}stanbul\
\u{1}stra\u{df}eline1\u{a}line2\u{1}stra\u{df}ens:key.path\u{1}stra\u{df}esrc/a/b.ts\
\u{1}stra\u{df}estra\u{df}e\u{1}stra\u{df}etab\u{9}separated\u{1}stra\u{df}etrailing.\
\u{1}stra\u{df}e\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}tab\u{9}separated\
\u{1}tab\u{9}separated:\u{1}tab\u{9}separated.leading\u{1}tab\u{9}separated{name} has {count} items\
\u{1}tab\u{9}separateda:b:c\u{1}tab\u{9}separateda..b\u{1}tab\u{9}separateda.b.c\
\u{1}tab\u{9}separateda\u{1f600}b\u{1f600}c\u{1}tab\u{9}separatedab\u{1}tab\u{9}separatedabc\
\u{1}tab\u{9}separated\u{feff}bom\u{1}tab\u{9}separatedHello World\u{1}tab\u{9}separatedsrc/a/b.ts\
\u{1}tab\u{9}separatedstra\u{df}e\u{1}tab\u{9}separated\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}tab\u{9}separated\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}trailing.\
\u{1}trailing.:\u{1}trailing..leading\u{1}trailing.{name} has {count} items\u{1}trailing.a\u{1}trailing.a.b.c\
\u{1}trailing.hello\u{1}trailing.HELLO\u{1}trailing.hello world\u{1}trailing.key_name-1\
\u{1}trailing.line1\u{a}line2\u{1}trailing.src/a/b.ts\
\u{1}trailing.\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}  padded  \u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}:\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}./x/**/*.json\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}{name} has {count} items\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1f600}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a.b.c\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a\u{1f600}b\u{1f600}c\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}ab\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{feff}bom\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}e\u{301}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}emoji \u{1f44d}\u{1f3fd} mix\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}key_name-1\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}line1\u{a}line2\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}ns:key.path\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}tab\u{9}separated\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}trailing.\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\
\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}wo\
\u{1}Wor\u{1}x\u{1}X\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}:\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}./x/**/*.json\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}.leading\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1f600}\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a:b:c\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a.b.c\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a\u{1f600}b\u{1f600}c\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}ab\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}abc\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}e\u{301}\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}emoji \u{1f44d}\u{1f3fd} mix\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hello\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}HELLO\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hello world\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}key_name-1\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}src/a/b.ts\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}stra\u{df}e\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}tab\u{9}separated\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}trailing.\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{fc}n\u{ef}c\u{f6}d\u{e9}\
\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\
\u{1}\u{3a3}\u{3a5}\u{3a6}\u{1}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hell\
\u{1}\u{915}\u{93f} \u{663}\u{664}\u{1}\u{30b9}\u{1}\u{306e}\u{30c6}\u{30ad}\u{30b9}\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}:\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}./x/**/*.json\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}{name} has {count} items\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1f600}\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a:b:c\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a..b\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a.b.c\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a\u{1f600}b\u{1f600}c\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}abc\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}e\u{301}\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}emoji \u{1f44d}\u{1f3fd} mix\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}HELLO\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello world\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}key_name-1\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}stra\u{df}e\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}trailing.\
\u{1}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}";
