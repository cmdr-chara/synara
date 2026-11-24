# hostsecrets-rs

FILE: hostSecrets.ts Purpose: The client-side cryptographic core of Host Secrets sync (ADR 0004, ADR 0015). Per-host configuration — SSH destination, launcher config, host-key verification policy — syncs across the owner's devices through the account service, which stores opaque bytes and can never read them.

napi-rs bridge for `packages/shared/src/hostSecrets.ts`.

## Functions

- `encodeBase64Url` (`packages/shared/src/hostSecrets.ts:157`)
- `decodeBase64Url` (`packages/shared/src/hostSecrets.ts:171`)
- `generateSyncKey` (`packages/shared/src/hostSecrets.ts:225`)
- `sealHostSecret` (`packages/shared/src/hostSecrets.ts:240`)
- `openHostSecret` (`packages/shared/src/hostSecrets.ts:284`)
- `generatePairingKeyPair` (`packages/shared/src/hostSecrets.ts:399`)
- `wrapSyncKey` (`packages/shared/src/hostSecrets.ts:473`)
- `unwrapSyncKey` (`packages/shared/src/hostSecrets.ts:515`)
- `pairingVerificationCode` (`packages/shared/src/hostSecrets.ts:564`)
- `rotateHostSecrets` (`packages/shared/src/hostSecrets.ts:620`)