# devicekey-rs

Device identity: the client half of the host-auth handshake. A device proves who it is with a keypair that never leaves the platform keystore.

napi-rs bridge for `packages/shared/src/deviceKey.ts`.

## Functions

- `hostAudience` (`packages/shared/src/deviceKey.ts:43`)
- `generateDeviceKey` (`packages/shared/src/deviceKey.ts:79`)
- `exportPublicJwk` (`packages/shared/src/deviceKey.ts:98`)
- `deviceThumbprint` (`packages/shared/src/deviceKey.ts:115`)
- `signDeviceRegistration` (`packages/shared/src/deviceKey.ts:183`)
- `dpopHttpUri` (`packages/shared/src/deviceKey.ts:210`)
- `credentialHash` (`packages/shared/src/deviceKey.ts:216`)
- `signDpopProof` (`packages/shared/src/deviceKey.ts:245`)
- `signMintRequest` (`packages/shared/src/deviceKey.ts:282`)