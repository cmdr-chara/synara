# text-rs

FILE: text.ts Purpose: Small, dependency-free text helpers shared across server and web so repeated string semantics (count pluralization, etc.) live in one place. Layer: Shared runtime utility Exports: normalizeLineEndings, pluralize, nonEmptyTrimmed, splitsSurrogatePair, stripTerminalControlSequences, unicodeSafeEndOffset

napi-rs bridge for `packages/shared/src/text.ts`.

## Functions

- `normalizeLineEndings` (`packages/shared/src/text.ts:9`)
- `splitsSurrogatePair` (`packages/shared/src/text.ts:16`)
- `unicodeSafeEndOffset` (`packages/shared/src/text.ts:30`)
- `nonEmptyTrimmed` (`packages/shared/src/text.ts:44`)
- `stripTerminalControlSequences` (`packages/shared/src/text.ts:52`)
- `pluralize` (`packages/shared/src/text.ts:61`)