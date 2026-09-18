# Custom agent profile backend checkpoint

Roadmap owner: E6 and I3.

Custom profiles remain protocol-independent launch metadata. They can now be
created, edited, imported, exported and deleted through `WorkspaceService`
without introducing provider-specific backends.

## Secret references

Profile JSON may contain `secret_env`, a map from validated environment-variable
names to `SecretReference { service, account }`. It never contains the secret
value. `AgentProfile::launch_spec_with_secret_store` resolves those references
only when the controller is about to launch the profile, through the injected
`SecretStore` boundary.

The default controller uses a fail-closed unavailable store. Applications that
support Synara-owned credentials must inject an OS-backed implementation with
`Controller::with_secret_store`. A profile requiring a secret therefore fails
explicitly until an approved credential adapter is available; there is no
plaintext SQLite/settings fallback.

Managed registry profiles cannot add `secret_env` overrides. Their reviewed
launch contract remains immutable. Agent/vendor-owned subscription login remains
owned by the agent protocol rather than copied into Synara profile storage.

## CRUD and import/export safety

`WorkspaceService` provides explicit custom-profile upsert, delete, export and
import operations.

- Removing a profile that is assigned to any durable task is refused.
- Editing a profile assigned to a Running or Waiting task is refused.
- Editing an inactive assigned profile invalidates its stored remote session
  reference before the replacement is persisted, so a changed executable/env
  contract cannot silently resume an old agent session.
- Managed registry IDs cannot be overwritten by custom imports or edits.
- Custom export omits managed registry receipts and remains bounded to 1 MiB.
- Import uses the same strict profile parser and rejects managed receipts.
- Environment-variable and secret-reference names cannot overlap.

Tests use only a synthetic `secret-canary` value in an in-memory credential
fixture. Serialized profiles expose the reference name but not that value.

## Remaining acceptance boundary

This closes the portable backend shape for custom-profile CRUD/import/export and
secret references. Product settings/profile UI remains owned by the concurrent
frontend work. Real macOS Keychain, Windows Credential Manager and Linux secret
store adapters and interaction evidence remain required before I3 is complete.
