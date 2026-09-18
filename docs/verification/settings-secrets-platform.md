# Settings, secrets and platform backend checkpoint

Roadmap owner: I. Product UI remains owned by the concurrent frontend work.

## Versioned settings

`synara-workspace::AppSettings` is a versioned, non-secret settings contract.
The current schema records theme, reduced-motion, UI/code font preferences and
custom keybindings. Bounds reject control characters, non-finite or unreasonable
font sizes, duplicate commands/shortcuts and oversized collections.

Settings use the existing SQLite preference store on the blocking workspace
worker. Loading a malformed record, a record from an unsupported schema version,
or invalid data for the current version returns validated defaults plus a typed
`SettingsRecovery` reason. The stored bytes are not overwritten merely by
reading/recovering them. An explicit validated save is required to replace the
record. This preserves forensic/recovery data instead of silently resetting it.

## Secret-store boundary

`synara-runtime::SecretStore` is the only new backend contract for Synara-owned
secret values. `SecretReference` is serializable, but `SecretValue` is not:
it has redacted Debug output, explicit byte exposure and zeroes its owned bytes
on drop. `UnavailableSecretStore` and its locked state fail closed. There is no
SQLite, settings-file or environment-file fallback.

This checkpoint does not claim native Keychain, Credential Manager or Linux
Secret Service acceptance. Those adapters and their real target-environment
tests remain required before I3 can close.

## Platform-service boundary

`synara-runtime::PlatformServices` defines bounded native file/save/folder
dialog and notification requests. Inputs validate titles, file extensions and
notification payload size. The unsupported adapter fails explicitly instead of
shelling out or silently substituting a different host.

This establishes the backend boundary for I4 only. Native GPUI menu/dialog/
notification implementations, accessibility presentation and per-platform
interaction evidence remain outside this backend checkpoint.

## Verification

Focused acceptance is provided by unit tests in `settings.rs`, `secrets.rs`
and `platform.rs`, plus the ordinary workspace/backend/static verification
lanes. No secret value is used as a repository fixture except synthetic
`secret-canary` bytes inside an isolated unit test.
