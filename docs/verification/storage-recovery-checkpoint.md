# Storage recovery checkpoint

Published implementation: `689effe365cb73ae0a2bd0138f575b34549446d1`.
Verified source tree: `31fd8d4315686ad1ace4188835e4aa13c831435f`.

This checkpoint adds bounded online SQLite backup and no-clobber restore into a
new database, independent source preservation, cancellation, schema and catalog
validation, and asynchronous workspace service entry points. It also fixes the
system-directory-alias regression observed on macOS while retaining rejection
of symlinked database leaves and capability-rooted filesystem callbacks.

Local Linux x86_64 verification used Rust 1.98.1 and the locked dependencies.
Formatting, backend workspace check, strict all-target/all-feature Clippy,
backend all-feature tests, workspace audit, publisher regression tests and
roadmap checks passed. Native application and other-platform acceptance are
not inferred from these local backend results.

The backend acceptance workflow on this checkpoint runs native Linux x86_64,
macOS arm64 and Windows x86_64 checks and tests against the published source.
F4 remains open until that evidence has been inspected. The original database
is never replaced, and neither backup nor restore starts an agent. See
[the recovery contract](../database-recovery.md) for limits and retention.
