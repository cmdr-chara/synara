# computerkeynames-rs

The named-key vocabulary every computer-use surface has to agree on. A key press crosses three layers written in three languages: the web pane decides which DOM key names it may swallow instead of letting the browser see them, the Linux evdev synthesizer maps names to kernel key codes, and the macOS Swift helper maps the same names to virtual key codes.

napi-rs bridge for `packages/shared/src/computerKeyNames.ts`.

## Functions

- `isComputerNamedKey` (`packages/shared/src/computerKeyNames.ts:65`)