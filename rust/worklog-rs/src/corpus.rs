//! The `localeCompare` order observed in the recorded corpus (Bun/ICU root).

/// Rank of a recorded string in `localeCompare` order.
pub fn rank(s: &str) -> Option<i64> {
    RANKED.split('\u{1}').position(|x| x == s).map(|i| i as i64)
}

const RANKED: &str = "\u{1}  padded  \u{1}  padded    padded  \u{1}  padded  :\u{1}  padded  ./x/**/*.json\u{1}\
  padded  .leading\u{1}  padded  {name} has {count} items\u{1}  padded  \u{1f600}\u{1}  padded  a\u{1}\
  padded  a:b:c\u{1}  padded  a..b\u{1}  padded  a.b.c\u{1}  padded  a\u{1f600}b\u{1f600}c\u{1}\
  padded  ab\u{1}  padded  abc\u{1}  padded  \u{feff}bom\u{1}  padded  e\u{301}\u{1}\
  padded  emoji \u{1f44d}\u{1f3fd} mix\u{1}  padded  hello\u{1}  padded  HELLO\u{1}  padded  hello world\u{1}\
  padded  Hello World\u{1}  padded  \u{130}stanbul\u{1}  padded  key_name-1\u{1}  padded  line1\u{a}line2\u{1}\
  padded  ns:key.path\u{1}  padded  src/a/b.ts\u{1}  padded  stra\u{df}e\u{1}  padded  tab\u{9}separated\u{1}\
  padded  trailing.\u{1}  padded  \u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
  padded  \u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
  padded  \u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}-0\u{1}-Infinity\u{1}:\u{1}\
:  padded  \u{1}::\u{1}:./x/**/*.json\u{1}:.leading\u{1}:{name} has {count} items\u{1}:\u{1f600}\u{1}:a\u{1}\
:a:b:c\u{1}:a..b\u{1}:a.b.c\u{1}:a\u{1f600}b\u{1f600}c\u{1}:ab\u{1}:abc\u{1}:\u{feff}bom\u{1}:e\u{301}\u{1}\
:emoji \u{1f44d}\u{1f3fd} mix\u{1}:hello\u{1}:HELLO\u{1}:hello world\u{1}:Hello World\u{1}:\u{130}stanbul\u{1}\
:key_name-1\u{1}:line1\u{a}line2\u{1}:ns:key.path\u{1}:src/a/b.ts\u{1}:stra\u{df}e\u{1}:tab\u{9}separated\u{1}\
:trailing.\u{1}:\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}:\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
:\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}./x/**/*.json\u{1}\
./x/**/*.json  padded  \u{1}./x/**/*.json:\u{1}./x/**/*.json./x/**/*.json\u{1}./x/**/*.json.leading\u{1}\
./x/**/*.json{name} has {count} items\u{1}./x/**/*.json\u{1f600}\u{1}./x/**/*.jsona\u{1}\
./x/**/*.jsona:b:c\u{1}./x/**/*.jsona..b\u{1}./x/**/*.jsona.b.c\u{1}./x/**/*.jsona\u{1f600}b\u{1f600}c\u{1}\
./x/**/*.jsonab\u{1}./x/**/*.jsonabc\u{1}./x/**/*.json\u{feff}bom\u{1}./x/**/*.jsone\u{301}\u{1}\
./x/**/*.jsonemoji \u{1f44d}\u{1f3fd} mix\u{1}./x/**/*.jsonhello\u{1}./x/**/*.jsonHELLO\u{1}\
./x/**/*.jsonhello world\u{1}./x/**/*.jsonHello World\u{1}./x/**/*.json\u{130}stanbul\u{1}\
./x/**/*.jsonkey_name-1\u{1}./x/**/*.jsonline1\u{a}line2\u{1}./x/**/*.jsonns:key.path\u{1}\
./x/**/*.jsonsrc/a/b.ts\u{1}./x/**/*.jsonstra\u{df}e\u{1}./x/**/*.jsontab\u{9}separated\u{1}\
./x/**/*.jsontrailing.\u{1}./x/**/*.json\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
./x/**/*.json\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
./x/**/*.json\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}.leading\u{1}\
.leading  padded  \u{1}.leading:\u{1}.leading./x/**/*.json\u{1}.leading.leading\u{1}\
.leading{name} has {count} items\u{1}.leading\u{1f600}\u{1}.leadinga\u{1}.leadinga:b:c\u{1}.leadinga..b\u{1}\
.leadinga.b.c\u{1}.leadinga\u{1f600}b\u{1f600}c\u{1}.leadingab\u{1}.leadingabc\u{1}.leading\u{feff}bom\u{1}\
.leadinge\u{301}\u{1}.leadingemoji \u{1f44d}\u{1f3fd} mix\u{1}.leadinghello\u{1}.leadingHELLO\u{1}\
.leadinghello world\u{1}.leadingHello World\u{1}.leading\u{130}stanbul\u{1}.leadingkey_name-1\u{1}\
.leadingline1\u{a}line2\u{1}.leadingns:key.path\u{1}.leadingsrc/a/b.ts\u{1}.leadingstra\u{df}e\u{1}\
.leadingtab\u{9}separated\u{1}.leadingtrailing.\u{1}.leading\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
.leading\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
.leading\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}{} is not iterable\u{1}\
{name} has {count} items\u{1}{name} has {count} items  padded  \u{1}{name} has {count} items:\u{1}\
{name} has {count} items./x/**/*.json\u{1}{name} has {count} items.leading\u{1}\
{name} has {count} items{name} has {count} items\u{1}{name} has {count} items\u{1f600}\u{1}\
{name} has {count} itemsa\u{1}{name} has {count} itemsa:b:c\u{1}{name} has {count} itemsa..b\u{1}\
{name} has {count} itemsa.b.c\u{1}{name} has {count} itemsa\u{1f600}b\u{1f600}c\u{1}\
{name} has {count} itemsab\u{1}{name} has {count} itemsabc\u{1}{name} has {count} items\u{feff}bom\u{1}\
{name} has {count} itemse\u{301}\u{1}{name} has {count} itemsemoji \u{1f44d}\u{1f3fd} mix\u{1}\
{name} has {count} itemshello\u{1}{name} has {count} itemsHELLO\u{1}{name} has {count} itemshello world\u{1}\
{name} has {count} itemsHello World\u{1}{name} has {count} items\u{130}stanbul\u{1}\
{name} has {count} itemskey_name-1\u{1}{name} has {count} itemsline1\u{a}line2\u{1}\
{name} has {count} itemsns:key.path\u{1}{name} has {count} itemssrc/a/b.ts\u{1}\
{name} has {count} itemsstra\u{df}e\u{1}{name} has {count} itemstab\u{9}separated\u{1}\
{name} has {count} itemstrailing.\u{1}{name} has {count} items\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
{name} has {count} items\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
{name} has {count} items\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\u{1f600}\u{1}\
\u{1f600}  padded  \u{1}\u{1f600}:\u{1}\u{1f600}./x/**/*.json\u{1}\u{1f600}.leading\u{1}\
\u{1f600}{name} has {count} items\u{1}\u{1f600}\u{1f600}\u{1}\u{1f600}a\u{1}\u{1f600}a:b:c\u{1}\
\u{1f600}a..b\u{1}\u{1f600}a.b.c\u{1}\u{1f600}a\u{1f600}b\u{1f600}c\u{1}\u{1f600}ab\u{1}\u{1f600}abc\u{1}\
\u{1f600}\u{feff}bom\u{1}\u{1f600}e\u{301}\u{1}\u{1f600}emoji \u{1f44d}\u{1f3fd} mix\u{1}\u{1f600}hello\u{1}\
\u{1f600}HELLO\u{1}\u{1f600}hello world\u{1}\u{1f600}Hello World\u{1}\u{1f600}\u{130}stanbul\u{1}\
\u{1f600}key_name-1\u{1}\u{1f600}line1\u{a}line2\u{1}\u{1f600}ns:key.path\u{1}\u{1f600}src/a/b.ts\u{1}\
\u{1f600}stra\u{df}e\u{1}\u{1f600}tab\u{9}separated\u{1}\u{1f600}trailing.\u{1}\
\u{1f600}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{1f600}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{1f600}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a\u{1}a  padded  \u{1}a:\u{1}\
a:b:c\u{1}a:b:c  padded  \u{1}a:b:c:\u{1}a:b:c./x/**/*.json\u{1}a:b:c.leading\u{1}\
a:b:c{name} has {count} items\u{1}a:b:c\u{1f600}\u{1}a:b:ca\u{1}a:b:ca:b:c\u{1}a:b:ca..b\u{1}a:b:ca.b.c\u{1}\
a:b:ca\u{1f600}b\u{1f600}c\u{1}a:b:cab\u{1}a:b:cabc\u{1}a:b:c\u{feff}bom\u{1}a:b:ce\u{301}\u{1}\
a:b:cemoji \u{1f44d}\u{1f3fd} mix\u{1}a:b:chello\u{1}a:b:cHELLO\u{1}a:b:chello world\u{1}a:b:cHello World\u{1}\
a:b:c\u{130}stanbul\u{1}a:b:ckey_name-1\u{1}a:b:cline1\u{a}line2\u{1}a:b:cns:key.path\u{1}a:b:csrc/a/b.ts\u{1}\
a:b:ctab\u{9}separated\u{1}a:b:ctrailing.\u{1}a:b:c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
a:b:c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
a:b:c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a..b\u{1}a..b  padded  \u{1}\
a..b:\u{1}a..b./x/**/*.json\u{1}a..b.leading\u{1}a..b{name} has {count} items\u{1}a..b\u{1f600}\u{1}a..ba\u{1}\
a..ba:b:c\u{1}a..ba..b\u{1}a..ba.b.c\u{1}a..ba\u{1f600}b\u{1f600}c\u{1}a..bab\u{1}a..babc\u{1}\
a..b\u{feff}bom\u{1}a..be\u{301}\u{1}a..bemoji \u{1f44d}\u{1f3fd} mix\u{1}a..bhello\u{1}a..bHELLO\u{1}\
a..bhello world\u{1}a..bHello World\u{1}a..b\u{130}stanbul\u{1}a..bkey_name-1\u{1}a..bline1\u{a}line2\u{1}\
a..bns:key.path\u{1}a..bsrc/a/b.ts\u{1}a..bstra\u{df}e\u{1}a..btab\u{9}separated\u{1}a..btrailing.\u{1}\
a..b\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}a..b\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
a..b\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a./x/**/*.json\u{1}a.b.c\u{1}\
a.b.c  padded  \u{1}a.b.c:\u{1}a.b.c./x/**/*.json\u{1}a.b.c.leading\u{1}a.b.c{name} has {count} items\u{1}\
a.b.c\u{1f600}\u{1}a.b.ca\u{1}a.b.ca:b:c\u{1}a.b.ca..b\u{1}a.b.ca.b.c\u{1}a.b.ca\u{1f600}b\u{1f600}c\u{1}\
a.b.cab\u{1}a.b.cabc\u{1}a.b.c\u{feff}bom\u{1}a.b.ce\u{301}\u{1}a.b.cemoji \u{1f44d}\u{1f3fd} mix\u{1}\
a.b.chello\u{1}a.b.cHELLO\u{1}a.b.chello world\u{1}a.b.cHello World\u{1}a.b.c\u{130}stanbul\u{1}\
a.b.ckey_name-1\u{1}a.b.cline1\u{a}line2\u{1}a.b.cns:key.path\u{1}a.b.csrc/a/b.ts\u{1}a.b.cstra\u{df}e\u{1}\
a.b.ctab\u{9}separated\u{1}a.b.ctrailing.\u{1}a.b.c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
a.b.c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
a.b.c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}a.leading\u{1}\
a{name} has {count} items\u{1}a\u{1f600}\u{1}a\u{1f600}b\u{1f600}c\u{1}a\u{1f600}b\u{1f600}c  padded  \u{1}\
a\u{1f600}b\u{1f600}c:\u{1}a\u{1f600}b\u{1f600}c./x/**/*.json\u{1}a\u{1f600}b\u{1f600}c.leading\u{1}\
a\u{1f600}b\u{1f600}c{name} has {count} items\u{1}a\u{1f600}b\u{1f600}c\u{1f600}\u{1}\
a\u{1f600}b\u{1f600}ca\u{1}a\u{1f600}b\u{1f600}ca:b:c\u{1}a\u{1f600}b\u{1f600}ca..b\u{1}\
a\u{1f600}b\u{1f600}ca.b.c\u{1}a\u{1f600}b\u{1f600}ca\u{1f600}b\u{1f600}c\u{1}a\u{1f600}b\u{1f600}cab\u{1}\
a\u{1f600}b\u{1f600}cabc\u{1}a\u{1f600}b\u{1f600}c\u{feff}bom\u{1}a\u{1f600}b\u{1f600}ce\u{301}\u{1}\
a\u{1f600}b\u{1f600}cemoji \u{1f44d}\u{1f3fd} mix\u{1}a\u{1f600}b\u{1f600}chello\u{1}\
a\u{1f600}b\u{1f600}cHELLO\u{1}a\u{1f600}b\u{1f600}chello world\u{1}a\u{1f600}b\u{1f600}cHello World\u{1}\
a\u{1f600}b\u{1f600}c\u{130}stanbul\u{1}a\u{1f600}b\u{1f600}ckey_name-1\u{1}\
a\u{1f600}b\u{1f600}cline1\u{a}line2\u{1}a\u{1f600}b\u{1f600}cns:key.path\u{1}\
a\u{1f600}b\u{1f600}csrc/a/b.ts\u{1}a\u{1f600}b\u{1f600}cstra\u{df}e\u{1}\
a\u{1f600}b\u{1f600}ctab\u{9}separated\u{1}a\u{1f600}b\u{1f600}ctrailing.\u{1}\
a\u{1f600}b\u{1f600}c\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
a\u{1f600}b\u{1f600}c\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
a\u{1f600}b\u{1f600}c\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}aa\u{1}aa:b:c\u{1}\
aa..b\u{1}aa.b.c\u{1}aa\u{1f600}b\u{1f600}c\u{1}aab\u{1}aabc\u{1}ab\u{1}ab  padded  \u{1}ab:\u{1}\
ab./x/**/*.json\u{1}ab.leading\u{1}ab{name} has {count} items\u{1}ab\u{1f600}\u{1}aba\u{1}aba:b:c\u{1}\
aba..b\u{1}aba.b.c\u{1}aba\u{1f600}b\u{1f600}c\u{1}abab\u{1}ababc\u{1}ab\u{feff}bom\u{1}abc\u{1}\
abc  padded  \u{1}abc:\u{1}abc./x/**/*.json\u{1}abc.leading\u{1}abc{name} has {count} items\u{1}\
abc\u{1f600}\u{1}abca\u{1}abca:b:c\u{1}abca..b\u{1}abca.b.c\u{1}abca\u{1f600}b\u{1f600}c\u{1}abcab\u{1}\
abcabc\u{1}abc\u{feff}bom\u{1}abce\u{301}\u{1}abcemoji \u{1f44d}\u{1f3fd} mix\u{1}abchello\u{1}abcHELLO\u{1}\
abchello world\u{1}abcHello World\u{1}abc\u{130}stanbul\u{1}abckey_name-1\u{1}abcline1\u{a}line2\u{1}\
abcns:key.path\u{1}abcsrc/a/b.ts\u{1}abcstra\u{df}e\u{1}abctab\u{9}separated\u{1}abctrailing.\u{1}\
abc\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}abc\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
abc\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}abe\u{301}\u{1}\
abemoji \u{1f44d}\u{1f3fd} mix\u{1}abhello\u{1}abHELLO\u{1}abhello world\u{1}abHello World\u{1}\
ab\u{130}stanbul\u{1}abkey_name-1\u{1}abline1\u{a}line2\u{1}abns:key.path\u{1}a\u{feff}bom\u{1}\
absrc/a/b.ts\u{1}abstra\u{df}e\u{1}abtab\u{9}separated\u{1}abtrailing.\u{1}ab\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
ab\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
ab\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}accepted\u{1}ae\u{301}\u{1}\
aemoji \u{1f44d}\u{1f3fd} mix\u{1}ahello\u{1}aHELLO\u{1}ahello world\u{1}aHello World\u{1}a\u{130}stanbul\u{1}\
akey_name-1\u{1}aline1\u{a}line2\u{1}ans:key.path\u{1}asrc/a/b.ts\u{1}assistant\u{1}astra\u{df}e\u{1}\
atab\u{9}separated\u{1}atrailing.\u{1}a\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}available\u{1}\
a\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
a\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}b\u{1}\u{feff}bom\u{1}\
\u{feff}bom  padded  \u{1}\u{feff}bom:\u{1}\u{feff}bom./x/**/*.json\u{1}\u{feff}bom.leading\u{1}\
\u{feff}bom{name} has {count} items\u{1}\u{feff}bom\u{1f600}\u{1}\u{feff}boma\u{1}\u{feff}boma:b:c\u{1}\
\u{feff}boma..b\u{1}\u{feff}boma.b.c\u{1}\u{feff}boma\u{1f600}b\u{1f600}c\u{1}\u{feff}bomab\u{1}\
\u{feff}bomabc\u{1}\u{feff}bom\u{feff}bom\u{1}\u{feff}bome\u{301}\u{1}\
\u{feff}bomemoji \u{1f44d}\u{1f3fd} mix\u{1}\u{feff}bomhello\u{1}\u{feff}bomHELLO\u{1}\
\u{feff}bomhello world\u{1}\u{feff}bomHello World\u{1}\u{feff}bom\u{130}stanbul\u{1}\u{feff}bomkey_name-1\u{1}\
\u{feff}bomline1\u{a}line2\u{1}\u{feff}bomns:key.path\u{1}\u{feff}bomsrc/a/b.ts\u{1}\
\u{feff}bomstra\u{df}e\u{1}\u{feff}bomtab\u{9}separated\u{1}\u{feff}bomtrailing.\u{1}\
\u{feff}bom\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{feff}bom\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{feff}bom\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}c\u{1}cancelled\u{1}\
command\u{1}completed\u{1}conversation-rebuilt\u{1}deriveSubagentTaskEnds\u{1}deriveThreadBackgroundTasks\u{1}\
deriveTimelineEntries\u{1}deriveWorkLogEntries\u{1}dismissed\u{1}e\u{301}\u{1}e\u{301}  padded  \u{1}\
e\u{301}:\u{1}e\u{301}./x/**/*.json\u{1}e\u{301}.leading\u{1}e\u{301}{name} has {count} items\u{1}\
e\u{301}\u{1f600}\u{1}e\u{301}a\u{1}e\u{301}a:b:c\u{1}e\u{301}a..b\u{1}e\u{301}a.b.c\u{1}\
e\u{301}a\u{1f600}b\u{1f600}c\u{1}e\u{301}ab\u{1}e\u{301}abc\u{1}e\u{301}\u{feff}bom\u{1}e\u{301}e\u{301}\u{1}\
e\u{301}emoji \u{1f44d}\u{1f3fd} mix\u{1}e\u{301}hello\u{1}e\u{301}HELLO\u{1}e\u{301}hello world\u{1}\
e\u{301}Hello World\u{1}e\u{301}\u{130}stanbul\u{1}e\u{301}key_name-1\u{1}e\u{301}line1\u{a}line2\u{1}\
emoji \u{1f44d}\u{1f3fd} mix\u{1}emoji \u{1f44d}\u{1f3fd} mix  padded  \u{1}emoji \u{1f44d}\u{1f3fd} mix:\u{1}\
emoji \u{1f44d}\u{1f3fd} mix./x/**/*.json\u{1}emoji \u{1f44d}\u{1f3fd} mix.leading\u{1}\
emoji \u{1f44d}\u{1f3fd} mix{name} has {count} items\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{1f600}\u{1}\
emoji \u{1f44d}\u{1f3fd} mixa\u{1}emoji \u{1f44d}\u{1f3fd} mixa:b:c\u{1}emoji \u{1f44d}\u{1f3fd} mixa..b\u{1}\
emoji \u{1f44d}\u{1f3fd} mixa.b.c\u{1}emoji \u{1f44d}\u{1f3fd} mixa\u{1f600}b\u{1f600}c\u{1}\
emoji \u{1f44d}\u{1f3fd} mixab\u{1}emoji \u{1f44d}\u{1f3fd} mixabc\u{1}\
emoji \u{1f44d}\u{1f3fd} mix\u{feff}bom\u{1}emoji \u{1f44d}\u{1f3fd} mixe\u{301}\u{1}\
emoji \u{1f44d}\u{1f3fd} mixemoji \u{1f44d}\u{1f3fd} mix\u{1}emoji \u{1f44d}\u{1f3fd} mixhello\u{1}\
emoji \u{1f44d}\u{1f3fd} mixHELLO\u{1}emoji \u{1f44d}\u{1f3fd} mixhello world\u{1}\
emoji \u{1f44d}\u{1f3fd} mixHello World\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{130}stanbul\u{1}\
emoji \u{1f44d}\u{1f3fd} mixkey_name-1\u{1}emoji \u{1f44d}\u{1f3fd} mixline1\u{a}line2\u{1}\
emoji \u{1f44d}\u{1f3fd} mixns:key.path\u{1}emoji \u{1f44d}\u{1f3fd} mixsrc/a/b.ts\u{1}\
emoji \u{1f44d}\u{1f3fd} mixstra\u{df}e\u{1}emoji \u{1f44d}\u{1f3fd} mixtab\u{9}separated\u{1}\
emoji \u{1f44d}\u{1f3fd} mixtrailing.\u{1}emoji \u{1f44d}\u{1f3fd} mix\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
emoji \u{1f44d}\u{1f3fd} mix\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
emoji \u{1f44d}\u{1f3fd} mix\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
e\u{301}ns:key.path\u{1}\
entries.filter is not a function. (In 'entries.filter((entry) => !isRoutedSubagentWorkEntry(entry))', 'entries.filter' is undefined)\u{1}\
error\u{1}e\u{301}src/a/b.ts\u{1}e\u{301}stra\u{df}e\u{1}e\u{301}tab\u{9}separated\u{1}e\u{301}trailing.\u{1}\
e\u{301}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}extractProviderHandoffInfo\u{1}\
e\u{301}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
e\u{301}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}failed\u{1}file-change\u{1}\
file-read\u{1}finished\u{1}fork-from-earlier-turn\u{1}fresh-session\u{1}hello\u{1}HELLO\u{1}\
hello  padded  \u{1}HELLO  padded  \u{1}hello world\u{1}Hello World\u{1}hello world  padded  \u{1}\
Hello World  padded  \u{1}hello world:\u{1}Hello World:\u{1}hello world./x/**/*.json\u{1}\
Hello World./x/**/*.json\u{1}hello world.leading\u{1}Hello World.leading\u{1}\
hello world{name} has {count} items\u{1}Hello World{name} has {count} items\u{1}hello world\u{1f600}\u{1}\
Hello World\u{1f600}\u{1}hello worlda\u{1}Hello Worlda\u{1}hello worlda:b:c\u{1}Hello Worlda:b:c\u{1}\
hello worlda..b\u{1}Hello Worlda..b\u{1}Hello Worlda.b.c\u{1}hello worlda\u{1f600}b\u{1f600}c\u{1}\
Hello Worlda\u{1f600}b\u{1f600}c\u{1}hello worldab\u{1}Hello Worldab\u{1}hello worldabc\u{1}\
Hello Worldabc\u{1}hello world\u{feff}bom\u{1}Hello World\u{feff}bom\u{1}hello worlde\u{301}\u{1}\
Hello Worlde\u{301}\u{1}hello worldemoji \u{1f44d}\u{1f3fd} mix\u{1}\
Hello Worldemoji \u{1f44d}\u{1f3fd} mix\u{1}hello worldhello\u{1}hello worldHELLO\u{1}Hello Worldhello\u{1}\
Hello WorldHELLO\u{1}hello worldhello world\u{1}hello worldHello World\u{1}Hello Worldhello world\u{1}\
Hello WorldHello World\u{1}hello world\u{130}stanbul\u{1}Hello World\u{130}stanbul\u{1}\
hello worldkey_name-1\u{1}Hello Worldkey_name-1\u{1}hello worldline1\u{a}line2\u{1}\
Hello Worldline1\u{a}line2\u{1}hello worldns:key.path\u{1}Hello Worldns:key.path\u{1}\
hello worldsrc/a/b.ts\u{1}Hello Worldsrc/a/b.ts\u{1}hello worldstra\u{df}e\u{1}Hello Worldstra\u{df}e\u{1}\
hello worldtab\u{9}separated\u{1}Hello Worldtab\u{9}separated\u{1}hello worldtrailing.\u{1}\
Hello Worldtrailing.\u{1}hello world\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
Hello World\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}hello world\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
Hello World\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
hello world\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
Hello World\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}hello:\u{1}HELLO:\u{1}\
hello./x/**/*.json\u{1}HELLO./x/**/*.json\u{1}hello.leading\u{1}HELLO.leading\u{1}\
hello{name} has {count} items\u{1}HELLO{name} has {count} items\u{1}hello\u{1f600}\u{1}HELLO\u{1f600}\u{1}\
helloa\u{1}HELLOa\u{1}helloa:b:c\u{1}HELLOa:b:c\u{1}helloa..b\u{1}HELLOa..b\u{1}helloa.b.c\u{1}HELLOa.b.c\u{1}\
helloa\u{1f600}b\u{1f600}c\u{1}HELLOa\u{1f600}b\u{1f600}c\u{1}helloab\u{1}HELLOab\u{1}helloabc\u{1}\
HELLOabc\u{1}hello\u{feff}bom\u{1}HELLO\u{feff}bom\u{1}helloe\u{301}\u{1}HELLOe\u{301}\u{1}\
helloemoji \u{1f44d}\u{1f3fd} mix\u{1}HELLOemoji \u{1f44d}\u{1f3fd} mix\u{1}hellohello\u{1}helloHELLO\u{1}\
HELLOhello\u{1}HELLOHELLO\u{1}hellohello world\u{1}helloHello World\u{1}HELLOhello world\u{1}\
HELLOHello World\u{1}hello\u{130}stanbul\u{1}HELLO\u{130}stanbul\u{1}hellokey_name-1\u{1}HELLOkey_name-1\u{1}\
helloline1\u{a}line2\u{1}HELLOline1\u{a}line2\u{1}hellons:key.path\u{1}HELLOns:key.path\u{1}\
hellosrc/a/b.ts\u{1}HELLOsrc/a/b.ts\u{1}hellostra\u{df}e\u{1}HELLOstra\u{df}e\u{1}hellotab\u{9}separated\u{1}\
HELLOtab\u{9}separated\u{1}hellotrailing.\u{1}HELLOtrailing.\u{1}hello\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
HELLO\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}hello\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
HELLO\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
hello\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
HELLO\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}Infinity\u{1}info\u{1}\
interrupt-escalation\u{1}isCodexRuntimeWarningType\u{1}isFileChangeWorkLogEntry\u{1}\
isMissingCheckpointBaselineActivity\u{1}isProviderFileEditWorkLogEntry\u{1}isRoutedSubagentWorkEntry\u{1}\
isSubagentStateOnlyWorkEntry\u{1}\u{130}stanbul\u{1}\u{130}stanbul  padded  \u{1}\u{130}stanbul:\u{1}\
\u{130}stanbul./x/**/*.json\u{1}\u{130}stanbul.leading\u{1}\u{130}stanbul{name} has {count} items\u{1}\
\u{130}stanbul\u{1f600}\u{1}\u{130}stanbula\u{1}\u{130}stanbula:b:c\u{1}\u{130}stanbula..b\u{1}\
\u{130}stanbula.b.c\u{1}\u{130}stanbula\u{1f600}b\u{1f600}c\u{1}\u{130}stanbulab\u{1}\u{130}stanbulabc\u{1}\
\u{130}stanbul\u{feff}bom\u{1}\u{130}stanbule\u{301}\u{1}\u{130}stanbulemoji \u{1f44d}\u{1f3fd} mix\u{1}\
\u{130}stanbulhello\u{1}\u{130}stanbulHELLO\u{1}\u{130}stanbulhello world\u{1}\u{130}stanbulHello World\u{1}\
\u{130}stanbul\u{130}stanbul\u{1}\u{130}stanbulkey_name-1\u{1}\u{130}stanbulline1\u{a}line2\u{1}\
\u{130}stanbulns:key.path\u{1}\u{130}stanbulsrc/a/b.ts\u{1}\u{130}stanbulstra\u{df}e\u{1}\
\u{130}stanbultab\u{9}separated\u{1}\u{130}stanbultrailing.\u{1}\u{130}stanbul\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
\u{130}stanbul\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{130}stanbul\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}key_name-1\u{1}\
key_name-1  padded  \u{1}key_name-1:\u{1}key_name-1./x/**/*.json\u{1}key_name-1.leading\u{1}\
key_name-1{name} has {count} items\u{1}key_name-1\u{1f600}\u{1}key_name-1a\u{1}key_name-1a:b:c\u{1}\
key_name-1a..b\u{1}key_name-1a.b.c\u{1}key_name-1a\u{1f600}b\u{1f600}c\u{1}key_name-1ab\u{1}key_name-1abc\u{1}\
key_name-1\u{feff}bom\u{1}key_name-1e\u{301}\u{1}key_name-1emoji \u{1f44d}\u{1f3fd} mix\u{1}\
key_name-1hello\u{1}key_name-1HELLO\u{1}key_name-1hello world\u{1}key_name-1Hello World\u{1}\
key_name-1\u{130}stanbul\u{1}key_name-1key_name-1\u{1}key_name-1line1\u{a}line2\u{1}key_name-1ns:key.path\u{1}\
key_name-1src/a/b.ts\u{1}key_name-1stra\u{df}e\u{1}key_name-1tab\u{9}separated\u{1}key_name-1trailing.\u{1}\
key_name-1\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}key_name-1\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
key_name-1\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}line1\u{a}line2\u{1}\
line1\u{a}line2  padded  \u{1}line1\u{a}line2:\u{1}line1\u{a}line2./x/**/*.json\u{1}\
line1\u{a}line2.leading\u{1}line1\u{a}line2{name} has {count} items\u{1}line1\u{a}line2\u{1f600}\u{1}\
line1\u{a}line2a\u{1}line1\u{a}line2a:b:c\u{1}line1\u{a}line2a..b\u{1}line1\u{a}line2a.b.c\u{1}\
line1\u{a}line2a\u{1f600}b\u{1f600}c\u{1}line1\u{a}line2ab\u{1}line1\u{a}line2abc\u{1}\
line1\u{a}line2\u{feff}bom\u{1}line1\u{a}line2e\u{301}\u{1}line1\u{a}line2emoji \u{1f44d}\u{1f3fd} mix\u{1}\
line1\u{a}line2hello\u{1}line1\u{a}line2HELLO\u{1}line1\u{a}line2hello world\u{1}\
line1\u{a}line2Hello World\u{1}line1\u{a}line2\u{130}stanbul\u{1}line1\u{a}line2key_name-1\u{1}\
line1\u{a}line2line1\u{a}line2\u{1}line1\u{a}line2ns:key.path\u{1}line1\u{a}line2src/a/b.ts\u{1}\
line1\u{a}line2stra\u{df}e\u{1}line1\u{a}line2tab\u{9}separated\u{1}line1\u{a}line2trailing.\u{1}\
line1\u{a}line2\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
line1\u{a}line2\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
line1\u{a}line2\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}message\u{1}NaN\u{1}\
native-history-unavailable\u{1}native-resume-failed\u{1}needs-you\u{1}ns:key.path\u{1}\
ns:key.path  padded  \u{1}ns:key.path:\u{1}ns:key.path./x/**/*.json\u{1}ns:key.path.leading\u{1}\
ns:key.path{name} has {count} items\u{1}ns:key.path\u{1f600}\u{1}ns:key.patha\u{1}ns:key.patha:b:c\u{1}\
ns:key.patha..b\u{1}ns:key.patha.b.c\u{1}ns:key.patha\u{1f600}b\u{1f600}c\u{1}ns:key.pathab\u{1}\
ns:key.pathabc\u{1}ns:key.path\u{feff}bom\u{1}ns:key.pathe\u{301}\u{1}\
ns:key.pathemoji \u{1f44d}\u{1f3fd} mix\u{1}ns:key.pathhello\u{1}ns:key.pathHELLO\u{1}\
ns:key.pathhello world\u{1}ns:key.pathHello World\u{1}ns:key.path\u{130}stanbul\u{1}ns:key.pathkey_name-1\u{1}\
ns:key.pathline1\u{a}line2\u{1}ns:key.pathns:key.path\u{1}ns:key.pathsrc/a/b.ts\u{1}\
ns:key.pathstra\u{df}e\u{1}ns:key.pathtab\u{9}separated\u{1}ns:key.pathtrailing.\u{1}\
ns:key.path\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}ns:key.path\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
ns:key.path\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
null is not an object (evaluating 'activities.length')\u{1}\
null is not an object (evaluating 'activity of activities')\u{1}\
null is not an object (evaluating 'activity.kind')\u{1}\
null is not an object (evaluating 'entries.filter')\u{1}\
null is not an object (evaluating 'entry.itemType')\u{1}\
null is not an object (evaluating 'entry.subagentAction')\u{1}\
null is not an object (evaluating 'options.translate')\u{1}\
null is not an object (evaluating 'segment.startedAt')\u{1}\
null is not an object (evaluating 'visibleMessages.flatMap')\u{1}\
null is not an object (evaluating 'visibleProposedPlans.flatMap')\u{1}\
null is not an object (evaluating 'visibleWorkEntries.map')\u{1}\
null is not an object (evaluating 'workEntry.itemType')\u{1}\
null is not an object (evaluating 'workEntry.requestKind')\u{1}number is not iterable\u{1}\
omitRoutedSubagentWorkEntries\u{1}\
ordered.filter is not a function. (In 'ordered.filter((activity) => !isLogOnlyPiWarning(activity))', 'ordered.filter' is undefined)\u{1}\
ordered.some is not a function. (In 'ordered.some((activity) => activity.kind === \"runtime.warning\" && asRecord(activity.payload)?.nativeEventType === \"background_tasks_changed\" || activity.kind === \"task.updated\" && asRecord(activity.payload)?.isBackgrounded === !0)', 'ordered.some' is undefined)\u{1}\
orderedActivities\u{1}parseTaskListTasks\u{1}parseUserInputQuestions\u{1}pending\u{1}permissions\u{1}\
proposed-plan\u{1}\
proposedPlans.filter is not a function. (In 'proposedPlans.filter((plan) => plan.turnId == null || !checkinTurnIds.has(plan.turnId))', 'proposedPlans.filter' is undefined)\u{1}\
rollup\u{1}running\u{1}running_tool\u{1}settled\u{1}src/a/b.ts\u{1}src/a/b.ts  padded  \u{1}src/a/b.ts:\u{1}\
src/a/b.ts./x/**/*.json\u{1}src/a/b.ts.leading\u{1}src/a/b.ts{name} has {count} items\u{1}\
src/a/b.ts\u{1f600}\u{1}src/a/b.tsa\u{1}src/a/b.tsa:b:c\u{1}src/a/b.tsa..b\u{1}src/a/b.tsa.b.c\u{1}\
src/a/b.tsa\u{1f600}b\u{1f600}c\u{1}src/a/b.tsab\u{1}src/a/b.tsabc\u{1}src/a/b.ts\u{feff}bom\u{1}\
src/a/b.tse\u{301}\u{1}src/a/b.tsemoji \u{1f44d}\u{1f3fd} mix\u{1}src/a/b.tshello\u{1}src/a/b.tsHELLO\u{1}\
src/a/b.tshello world\u{1}src/a/b.tsHello World\u{1}src/a/b.ts\u{130}stanbul\u{1}src/a/b.tskey_name-1\u{1}\
src/a/b.tsline1\u{a}line2\u{1}src/a/b.tsns:key.path\u{1}src/a/b.tssrc/a/b.ts\u{1}src/a/b.tsstra\u{df}e\u{1}\
src/a/b.tstab\u{9}separated\u{1}src/a/b.tstrailing.\u{1}src/a/b.ts\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
src/a/b.ts\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
src/a/b.ts\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}starting\u{1}stopped\u{1}\
stra\u{df}e\u{1}stra\u{df}e  padded  \u{1}stra\u{df}e:\u{1}stra\u{df}e./x/**/*.json\u{1}\
stra\u{df}e.leading\u{1}stra\u{df}e{name} has {count} items\u{1}stra\u{df}e\u{1f600}\u{1}stra\u{df}ea\u{1}\
stra\u{df}ea:b:c\u{1}stra\u{df}ea..b\u{1}stra\u{df}ea.b.c\u{1}stra\u{df}ea\u{1f600}b\u{1f600}c\u{1}\
stra\u{df}eab\u{1}stra\u{df}eabc\u{1}stra\u{df}e\u{feff}bom\u{1}stra\u{df}ee\u{301}\u{1}\
stra\u{df}eemoji \u{1f44d}\u{1f3fd} mix\u{1}stra\u{df}ehello\u{1}stra\u{df}eHELLO\u{1}\
stra\u{df}ehello world\u{1}stra\u{df}eHello World\u{1}stra\u{df}e\u{130}stanbul\u{1}stra\u{df}ekey_name-1\u{1}\
stra\u{df}eline1\u{a}line2\u{1}stra\u{df}ens:key.path\u{1}stra\u{df}esrc/a/b.ts\u{1}\
stra\u{df}estra\u{df}e\u{1}stra\u{df}etab\u{9}separated\u{1}stra\u{df}etrailing.\u{1}\
stra\u{df}e\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}stra\u{df}e\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
stra\u{df}e\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}streaming\u{1}stuck\u{1}\
system\u{1}tab\u{9}separated\u{1}tab\u{9}separated  padded  \u{1}tab\u{9}separated:\u{1}\
tab\u{9}separated./x/**/*.json\u{1}tab\u{9}separated.leading\u{1}\
tab\u{9}separated{name} has {count} items\u{1}tab\u{9}separated\u{1f600}\u{1}tab\u{9}separateda\u{1}\
tab\u{9}separateda:b:c\u{1}tab\u{9}separateda..b\u{1}tab\u{9}separateda.b.c\u{1}\
tab\u{9}separateda\u{1f600}b\u{1f600}c\u{1}tab\u{9}separatedab\u{1}tab\u{9}separatedabc\u{1}\
tab\u{9}separated\u{feff}bom\u{1}tab\u{9}separatede\u{301}\u{1}\
tab\u{9}separatedemoji \u{1f44d}\u{1f3fd} mix\u{1}tab\u{9}separatedhello\u{1}tab\u{9}separatedHELLO\u{1}\
tab\u{9}separatedhello world\u{1}tab\u{9}separatedHello World\u{1}tab\u{9}separated\u{130}stanbul\u{1}\
tab\u{9}separatedkey_name-1\u{1}tab\u{9}separatedline1\u{a}line2\u{1}tab\u{9}separatedns:key.path\u{1}\
tab\u{9}separatedsrc/a/b.ts\u{1}tab\u{9}separatedstra\u{df}e\u{1}tab\u{9}separatedtab\u{9}separated\u{1}\
tab\u{9}separatedtrailing.\u{1}tab\u{9}separated\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
tab\u{9}separated\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
tab\u{9}separated\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}thinking\u{1}tool\u{1}\
trailing.\u{1}trailing.  padded  \u{1}trailing.:\u{1}trailing../x/**/*.json\u{1}trailing..leading\u{1}\
trailing.{name} has {count} items\u{1}trailing.\u{1f600}\u{1}trailing.a\u{1}trailing.a:b:c\u{1}\
trailing.a..b\u{1}trailing.a.b.c\u{1}trailing.a\u{1f600}b\u{1f600}c\u{1}trailing.ab\u{1}trailing.abc\u{1}\
trailing.\u{feff}bom\u{1}trailing.e\u{301}\u{1}trailing.emoji \u{1f44d}\u{1f3fd} mix\u{1}trailing.hello\u{1}\
trailing.HELLO\u{1}trailing.hello world\u{1}trailing.Hello World\u{1}trailing.\u{130}stanbul\u{1}\
trailing.key_name-1\u{1}trailing.line1\u{a}line2\u{1}trailing.ns:key.path\u{1}trailing.src/a/b.ts\u{1}\
trailing.stra\u{df}e\u{1}trailing.tab\u{9}separated\u{1}trailing.trailing.\u{1}\
trailing.\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
trailing.\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}true is not iterable\u{1}\
unavailable\u{1}undefined is not an object (evaluating 'left.createdAt.localeCompare')\u{1}\
undefined is not an object (evaluating 'visibleMessages[index - 1].createdAt.localeCompare')\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}  padded  \u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}:\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}./x/**/*.json\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}.leading\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}{name} has {count} items\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1f600}\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}a\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a..b\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}a.b.c\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}a\u{1f600}b\u{1f600}c\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}ab\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}abc\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{feff}bom\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}e\u{301}\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}emoji \u{1f44d}\u{1f3fd} mix\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}hello\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}HELLO\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}hello world\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}Hello World\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{130}stanbul\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}key_name-1\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}line1\u{a}line2\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}ns:key.path\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}src/a/b.ts\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}stra\u{df}e\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}tab\u{9}separated\u{1}\u{fc}n\u{ef}c\u{f6}d\u{e9}trailing.\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}updated\u{1}\
user\u{1}\
visibleMessages.flatMap is not a function. (In 'visibleMessages.flatMap((message) => {\u{a}    const displayMessage = message.role === \"assistant\" && message.turnId && proposedPlanTurnIds.has(message.turnId) ? { ...message, text: stripProposedPlanBlocksFromText(message.text) } : message;\u{a}    if (displayMessage.role === \"assistant\" && displayMessage.text.length === 0 && displayMessage.turnId && proposedPlanTurnIds.has(displayMessage.turnId))\u{a}      return [];\u{a}    const textSegments = displayMessage.textSegments;\u{a}    if (displayMessage.role === \"assistant\" && !displayMessage.streaming && textSegments !== void 0 && textSegments.length > 1)\u{a}      return textSegments.map((segment, segmentIndex) => ({\u{a}        id: `${displayMessage.id}#seg:${segmentIndex}`,\u{a}        kind: \"message-segment\",\u{a}        createdAt: segment.startedAt,\u{a}        sequence: segment.sequence,\u{a}        message: displayMessage,\u{a}        segmentIndex\u{a}      }));\u{a}    return [\u{a}      {\u{a}        id: displayMessage.id,\u{a}        kind: \"message\",\u{a}        createdAt: displayMessage.createdAt,\u{a}        message: displayMessage\u{a}      }\u{a}    ];\u{a}  })', 'visibleMessages.flatMap' is undefined)\u{1}\
visibleProposedPlans.flatMap is not a function. (In 'visibleProposedPlans.flatMap((proposedPlan) => proposedPlan.turnId ? [proposedPlan.turnId] : [])', 'visibleProposedPlans.flatMap' is undefined)\u{1}\
visibleWorkEntries.map is not a function. (In 'visibleWorkEntries.map((entry) => ({\u{a}    id: entry.id,\u{a}    kind: \"work\",\u{a}    createdAt: entry.createdAt,\u{a}    ...entry.sequence !== void 0 ? { sequence: entry.sequence } : {},\u{a}    entry\u{a}  }))', 'visibleWorkEntries.map' is undefined)\u{1}\
waiting\u{1}WeakMap keys must be objects or non-registered symbols\u{1}work\u{1}\
workEntries.filter is not a function. (In 'workEntries.filter((entry) => entry.turnId == null || !checkinTurnIds.has(entry.turnId))', 'workEntries.filter' is undefined)\u{1}\
x\u{1}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}  padded  \u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}:\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}./x/**/*.json\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}.leading\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}{name} has {count} items\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1f600}\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a:b:c\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a..b\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a.b.c\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}a\u{1f600}b\u{1f600}c\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}ab\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}abc\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{feff}bom\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}e\u{301}\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}emoji \u{1f44d}\u{1f3fd} mix\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hello\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}HELLO\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}hello world\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}Hello World\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{130}stanbul\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}key_name-1\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}line1\u{a}line2\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}ns:key.path\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}src/a/b.ts\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}stra\u{df}e\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}tab\u{9}separated\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}trailing.\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}  padded  \u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}:\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}./x/**/*.json\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}.leading\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}{name} has {count} items\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{1f600}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a:b:c\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a..b\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a.b.c\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}a\u{1f600}b\u{1f600}c\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}ab\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}abc\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{feff}bom\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}e\u{301}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}emoji \u{1f44d}\u{1f3fd} mix\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}HELLO\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}hello world\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}Hello World\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{130}stanbul\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}key_name-1\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}line1\u{a}line2\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}ns:key.path\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}src/a/b.ts\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}stra\u{df}e\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}tab\u{9}separated\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}trailing.\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{fc}n\u{ef}c\u{f6}d\u{e9}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{3a3}\u{38a}\u{3a3}\u{3a5}\u{3a6}\u{39f}\u{3a3}\u{1}\
\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}\u{65e5}\u{672c}\u{8a9e}\u{306e}\u{30c6}\u{30ad}\u{30b9}\u{30c8}";
