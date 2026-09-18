# Development packaging and updater architecture

Roadmap owner: P3/P4. This is development groundwork only. No release is
published and no production signing identity, update endpoint or supported OS
minimum is declared.

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

This is not a native installer. Product-owner decisions about supported OS
minimums and platform installer formats remain open.

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

`UpdateHandoff` is a narrow serializable contract for a future platform helper:
staged artifact, current executable, rollback-copy path, expected digest, version
and current data-schema version. It contains no endpoint or trust material and
requires distinct absolute paths.

The portable layer intentionally stops before replacing a running executable.
That final operation differs across Linux, macOS and Windows and requires the
production signing/update policy and real installer/updater interaction evidence.
Rollback preservation is therefore part of the helper handoff contract rather
than an untested rename implementation.

## Tests

Deterministic tests cover signature-first rejection, target/schema
incompatibility, artifact size/digest verification, no-clobber staging, cleanup
after mismatch, distinct handoff paths and reproducible Linux/macOS tar or Windows
ZIP development-package construction.

P3 remains open for actual native installers and approved prerequisites/OS
minimums. P4 remains open for a real production verifier/key, authorized endpoint,
download transport and platform replacement/rollback helper.
