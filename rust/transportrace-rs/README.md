# transportrace-rs

Transport selection: probe every candidate path to a host at once, take the most preferred one that answers. ADR 0007 fixes the order — loopback > LAN > Tailscale > SSH > relay — and explicitly rejects latency-based selection: the fastest answer is not the best path.

napi-rs bridge for `packages/shared/src/transportRace.ts`.

## Functions

- `transportPreferenceRank` (`packages/shared/src/transportRace.ts:49`)
- `sortByTransportPreference` (`packages/shared/src/transportRace.ts:58`)