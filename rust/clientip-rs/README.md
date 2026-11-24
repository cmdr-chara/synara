# clientip-rs

FILE: clientIp.ts Purpose: Best-effort caller identity for rate limiting. Layer: API support Depends on: @hono/node-server conninfo (socket address behind no proxy).

napi-rs bridge for `apps/api/src/clientIp.ts`.

## Functions

- `clientIp` (`apps/api/src/clientIp.ts:61`)
- `resolveTrustedProxyHops` (`apps/api/src/clientIp.ts:85`)
- `sanitizeForwardableIp` (`apps/api/src/clientIp.ts:101`)
- `sanitizeForwardableUserAgent` (`apps/api/src/clientIp.ts:111`)