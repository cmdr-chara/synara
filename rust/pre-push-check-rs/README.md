# pre-push-check-rs

FILE: pre-push-check.ts Purpose: The pre-push gate. Formats, lints and type-checks only what the push          changes, so it stays fast enough that nobody skips it.

napi-rs bridge for `scripts/pre-push-check.ts`.

## Functions

- `parsePushedRefs` (`scripts/pre-push-check.ts:23`)
- `affectedProjects` (`scripts/pre-push-check.ts:74`)