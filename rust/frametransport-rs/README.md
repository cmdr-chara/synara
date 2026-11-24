# frametransport-rs

Frame transport.

`FrameTransport` and `makeFrameSink` stay in TypeScript: they are class /
callback-shaped APIs that do not cross the bridge.

napi-rs bridge for `packages/shared/src/frameTransport.ts`.

## Functions

- `encodeFrameEnvelope` (`packages/shared/src/frameTransport.ts:43`)
- `decodeFrameEnvelope` (`packages/shared/src/frameTransport.ts:79`)
- `peekFrameHeader` (`packages/shared/src/frameTransport.ts:124`)
- `classifyByFrameFlags` (`packages/shared/src/frameTransport.ts:153`)
- `decodeFrameResyncRequest` (`packages/shared/src/frameTransport.ts:423`)