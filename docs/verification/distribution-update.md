# Development packaging and updater architecture

The updater lifecycle is implemented at the product-code level. This repository
still does not declare a production feed URL, publisher identity or supported OS
minimum for development builds. Those are deployment configuration, not missing
runtime behavior.

## Development packages

`scripts/package_dev.py` creates deterministic, no-clobber development archives
for the three architecture labels already used by native compile evidence:

- `x86_64-unknown-linux-gnu` -> `linux-x64`;
- `aarch64-apple-darwin` -> `macos-arm64`;
- `x86_64-pc-windows-msvc` -> `windows-x64`.

The script accepts an already-built application binary and the privacy-safe
dependency inventory. It embeds the binary, `DEPENDENCIES.json` and a
`DEVELOPMENT_BUILD.json` manifest, then writes a separate SHA-256 file. Archive
timestamps/ownership metadata are normalized so identical inputs produce
identical package bytes. Existing outputs are never overwritten.

The manifest states `development-only` and explicitly does not claim an OS
minimum or production release status. It contains no signing key, endpoint,
credential or user path.

These archives are development packages rather than a public release channel.
Production feed/signing policy and supported-OS declarations remain deployment
configuration.

## Authenticated updater boundary

`synara-runtime::UpdateManifest::verify_signed` receives:

1. exact manifest bytes;
2. an opaque external signature;
3. an injected `UpdateSignatureVerifier`;
4. the current platform/architecture and durable-data schema version.

Signature verification happens before the manifest is trusted or parsed. The
portable layer does not select a signature algorithm or key. A verified manifest
then enforces target identity, bounded artifact size, SHA-256 and an explicit
compatible durable-data schema range.

Downloads are staged to a new absolute path using no-clobber creation. Length and
digest are checked while writing, the file is synchronized before success, and
failed/interrupted/mismatched staging removes the incomplete destination.

`UpdateHandoff` is the serializable install transaction passed to the launcher
or updater helper when a platform locks the running executable. It contains the
staged artifact, current executable, rollback-copy path, expected digest, version
and current data-schema version. It contains no endpoint or trust material and
requires distinct absolute paths.

The transaction now implements the replacement lifecycle itself. Immediately
before installation it rechecks the staged artifact's identity, size and digest,
requires the installed executable and rollback destination to satisfy the
no-symlink/no-clobber contract, carries forward the installed file permissions,
moves the current executable to the rollback path, atomically renames the staged
artifact into place and re-verifies the published bytes. If publication or
post-install verification fails, it restores the rollback copy or reports an
unknown write outcome rather than claiming success.

Rollback is also explicit: the installed update is first moved out of the live
path, the retained previous executable is restored, and the superseded update is
discarded only after restoration succeeds. The rename-based contract deliberately
requires one filesystem so a failed cross-volume copy can never leave a partial
executable. On platforms such as Windows that lock the running executable, the
same transaction is executed by the launcher/updater helper after Synara exits.

## Tests

Deterministic tests cover signature-first rejection, target/schema
incompatibility, artifact size/digest verification, no-clobber staging, cleanup
after mismatch, distinct handoff paths and reproducible Linux/macOS tar or Windows
ZIP development-package construction.

M28 is complete at the product-code level. A production release still needs a
configured feed, trusted publisher identity and platform distribution policy, but
those values are intentionally not invented inside development builds.
