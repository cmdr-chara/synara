# accountssocallback-rs

accountSsoCallback - the loopback half of the desktop PKCE SSO flow. The authorization-code grant needs somewhere for the browser to deliver the code, and on a desktop that is a one-shot HTTP listener on 127.0.0.1: bound to an ephemeral port, alive for exactly one sign-in attempt, and shut down on completion, timeout, or cancellation.

napi-rs bridge for `apps/server/src/accountSsoCallback.ts`.

## Functions

- `createPkcePair` (`apps/server/src/accountSsoCallback.ts:32`)
- `createSsoState` (`apps/server/src/accountSsoCallback.ts:39`)