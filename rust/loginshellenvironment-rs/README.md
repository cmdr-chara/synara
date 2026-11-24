# loginshellenvironment-rs

FILE: loginShellEnvironment.ts Purpose: Caches the login-shell environment probe on disk so cold starts stop paying ~1s to source the user's interactive rc on every launch. Exports: canonical probe names plus cached reader factories used by the server and desktop.

napi-rs bridge for `packages/shared/src/loginShellEnvironment.ts`.

## Functions

- `loginShellEnvironmentCachePath` (`packages/shared/src/loginShellEnvironment.ts:51`)