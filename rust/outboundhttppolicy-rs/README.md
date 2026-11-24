# outboundhttppolicy-rs

FILE: outboundHttpPolicy.ts Purpose: Defines runtime-neutral outbound URL, address, redirect, and JSON safety policy. Layer: Shared security policy used by server and desktop transports

napi-rs bridge for `packages/shared/src/outboundHttpPolicy.ts`.

## Functions

- `isPublicIpAddress` (`packages/shared/src/outboundHttpPolicy.ts:66`)
- `assertPublicIpAddress` (`packages/shared/src/outboundHttpPolicy.ts:78`)
- `assertExactLoopbackIpAddress` (`packages/shared/src/outboundHttpPolicy.ts:87`)
- `isBenchmarkIpAddress` (`packages/shared/src/outboundHttpPolicy.ts:106`)
- `assertJsonWithinLimits` (`packages/shared/src/outboundHttpPolicy.ts:170`)
- `stripOutboundSensitiveHeaders` (`packages/shared/src/outboundHttpPolicy.ts:212`)