# Dependency, advisory and diagnostic retention checkpoint

Roadmap owner: N5.

## Dependency and license inventory

`scripts/dependency_inventory.py` runs `cargo metadata --locked` and emits a
bounded JSON inventory containing package name, version, source kind and declared
license metadata. It intentionally omits source URLs and local filesystem paths.
External packages with neither a license expression nor a declared license file
are reported as gaps rather than assigned guessed terms.

Workspace-package licensing remains an owner decision under P5. This inventory is
engineering evidence, not legal approval.

## Vulnerability checking

`.github/workflows/security.yml` installs the explicitly pinned
`cargo-audit 0.22.2` client and checks the locked dependency graph against the
current RustSec advisory database. The audit JSON and dependency inventory are
retained as short-lived CI artifacts for seven days. The workflow has read-only
repository permissions and does not upload prompts, workspace files, credentials,
host labels or application diagnostics.

An advisory result is candidate-specific and time-sensitive. A previously green
RustSec run is not evidence that a later advisory database contains no finding.

## Diagnostic retention and privacy

Synara does not create persistent application log files in this native rewrite.
Normal tracing is written to the process stderr stream. Therefore there is no
silent application-owned log directory requiring file rotation today. If
persistent file logging is introduced later, it must add an explicit size/age
rotation policy before being accepted.

ACP inspector state is deliberately different from generic logging: the adapter
retains at most 512 redacted metadata entries in memory, raw protocol payloads and
stderr content are not retained, clear is explicit, and JSON export is capped at
1 MiB. Export is user-directed and local. No automatic telemetry or diagnostic
upload exists.

The bounded inspector export is the current diagnostic bundle for the implemented
ACP host. It contains connection/process/capability metadata and redacted trace
shape only. Transcript/file-content export is a separate trust boundary because
that material may contain sensitive user data.
