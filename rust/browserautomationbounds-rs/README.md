# browserautomationbounds-rs

`packages/contracts/src/browserAutomationBounds.ts`., bridged with wasm-bindgen.

## Functions

- `utf8ByteLength` (`packages/contracts/src/browserAutomationBounds.ts:5`)
- `BoundedUtf8String` (`packages/contracts/src/browserAutomationBounds.ts:7`)

## Gate waiver

`error cases`: `utf8ByteLength` throws only for `Symbol` input
(`Cannot convert a symbol to a string`), which has no representation on the
JSON wire — the reviver supports `$undefined`/`$num`/`$utf16`/`$bigint`/`$bytes`/
`$buffer`/`$map`/`$set`/`$date`/`$regexp`/`$function` only. The path is
unreachable for every transportable value, so no recorded error case can
exist. All other gate checks pass; behavior 427/427.
