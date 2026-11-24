# lengthprefixedrecords-rs

Splitting `u32 little-endian length` + payload records out of a byte stream. A native helper that pushes binary payloads at the server — the iOS device helper over a unix socket today — frames them this way, because a pipe or socket delivers arbitrary chunks and the envelope inside is not self-delimiting.

napi-rs bridge for `packages/shared/src/lengthPrefixedRecords.ts`.

## Functions

- `encodeLengthPrefixedRecord` (`packages/shared/src/lengthPrefixedRecords.ts:62`)