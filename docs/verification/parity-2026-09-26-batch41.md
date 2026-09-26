# Parity continuation verification - batch 41

Date: 2026-09-26

## Delivered

### M26 Real provider/account telemetry integration

Direct-provider settings now expose an explicit **Refresh live account** action.
The request is made against the exact reviewed provider endpoint using the
endpoint-bound OS credential owner. A successful credentialed metadata request
is presented as live provider access, not inferred from a stored key.

Synara reads only bounded rate/quota headers returned by the provider: request
limit/remaining/reset, token limit/remaining/reset, and Retry-After when present.
OpenAI-compatible and Anthropic header families are handled explicitly, with
standard RateLimit fallbacks; Google/direct-compatible endpoints may report the
same bounded common headers.

Missing telemetry remains visibly unavailable. Billing, credits, subscription
tier and account identity are never inferred. The live result is intentionally
ephemeral so a restart cannot display stale account quota as current.

## Inventory effect

- M26 complete: major remaining 9 -> 8.
- Shipped feature slices 87 -> 88.
- Execution total 11 -> 10.
