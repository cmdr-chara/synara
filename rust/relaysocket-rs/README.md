# relaysocket-rs

Relay socket.

`data` reaches the port as a JSON value: `RawData`'s `Buffer` crosses as
`{"$buffer": [..]}` (revived by `Buffer.from(v)`), a `Uint8Array` as
`{"$bytes": [..]}` (`Uint8Array.from(v)`), and other JS values carry their
usual markers (`$utf16`, `$num`, `$bigint`, `$date`, `$map`, `$set`,
`$undefined`).

napi-rs bridge for `apps/server/src/relaySocket.ts`.

## Functions

- `normalizeRelayFrame` (`apps/server/src/relaySocket.ts:22`)