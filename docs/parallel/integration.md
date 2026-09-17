# Parallel-session integration contract

This is preparation only. No integration branch is created or merged by the
EKOP session. The user must authorize actual integration after session handoffs.
The same independent rewrite history must be preserved throughout.

## Inputs

Common base: `1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121`.
Expected parentless root: `43b1fb89bf19dadc388d18008f9ceb21b8215716`.
Destination after acceptance: `astra/gpui-clean-rewrite`.

| Session | Branch | Primary implementation owner |
| --- | --- | --- |
| BCD | astra/session-bcd | agent, ACP, conversation and permission components |
| FGH | astra/session-fgh | workspace/persistence, files/editor/diff and Git |
| AJM | astra/session-ajm | process/runtime, terminal and SSH |
| EKOP | astra/session-ekop | registry, isolated browser policy, verification tooling and GUI specification |

Emanuele's work may be published in a different authorized repository. Do not
assume a missing branch means the work does not exist. Resolve its repository and
exact commit from the handoff, verify the expected base/root, then fetch that
specific source for the integration review. Do not create an empty substitute.

## Shared ownership risk

The original prompts permit localized changes to shared core, workspace, shell
and panels. This is an integration risk, not exclusive ownership. Separate
branches prevent immediate overwrites but do not prove compatibility.

The parent integrator owns final reconciliation of Cargo manifests/lockfiles,
SQLite migrations, central domain contracts, shared shell/input/panel wiring,
README, root ROADMAP, CI triggers and release metadata. Review opposite ends of an
API even when no textual conflict occurs. Do not resolve conflicts by taking an
entire file from one session or by silently removing another session's behavior.

EKOP avoids these shared surfaces. Its registry GUI uses existing workspace and
shell APIs. FGH changes to registration/storage or BCD changes to operation state
must be reconciled when EKOP is integrated.

## Read-only preflight

After explicit integration authorization, fetch the approved branches into a
clean integration workspace. Freeze every input to its exact final SHA before
reviewing changes. Then run:

```sh
python3 scripts/ekop/integration_audit.py --self-test
python3 scripts/ekop/integration_audit.py --repo . \
  --session bcd=origin/astra/session-bcd \
  --session fgh=origin/astra/session-fgh \
  --session ajm=origin/astra/session-ajm \
  --session ekop=origin/astra/session-ekop
```

The audit itself does not fetch, merge, reset, update refs or resolve conflicts.
It rejects foreign roots, missing common ancestry and unsafe ref spelling. It
reports exact SHAs, shared files, cross-owner changes, unassigned paths, overlap
and a dirty worktree. Renames contribute both old and new paths to overlap checks.

Exit 0 means ready for human/integrator review, not merge safety. Exit 1 means
review is required. Exit 2 means invalid inputs or an unproven ancestry/command.
A clean textual diff cannot prove schema, API, runtime or UX compatibility.
Run the script only against the intended checked-out repository. Its JSON contains
source paths and is local engineering evidence, not an automatic diagnostic payload.

## Integration sequence

1. Read every handoff, inspect actual diffs and correlate tests to final SHAs.
   Reject unowned changes or arrange explicit reconciliation before proceeding.
2. Create one temporary integration branch from the current destination HEAD.
   Reconfirm the destination still has the expected independent history.
3. Integrate shared domain/schema contracts before consumers. The provisional
   sequence is BCD, FGH, AJM, then EKOP, but change that order if the actual API or
   migration dependencies require it. Do not treat a suggested order as a proof.
4. After each accepted unit, reconcile affected consumers and rerun focused tests.
   Maintain a single schema/lockfile owner and regenerate from reviewed inputs.
5. Review full app state, connection ownership, missing/dirty documents, consent,
   selected model/config, terminal teardown and local/remote path interpretation.
6. Reconcile roadmap evidence. Only mark a task complete when its full acceptance
   condition has evidence. Preserve partial platform/vendor/browser gaps.
7. Run the complete applicable suite against the combined candidate. Only after
   acceptance, update the destination by a nonforce fast-forward. If the destination
   moved, stop, inspect its changes and revalidate instead of forcing it backward.

These steps require integration authority. They do not authorize importing any
legacy/main history, creating PRs/releases or changing the default branch.

## CI and source identity

Session workflows may initially be restricted to their own branch. The integrator
must explicitly adapt verification to the temporary integration branch without
silently running a branch-specific source publisher on the delivery branch.
In particular, the EKOP formatter is restricted to `astra/session-ekop` and its
owned files. Its downstream checks use the actual formatted revision output,
which can differ from the workflow trigger SHA. The source-checkpoint artifact
contains that candidate SHA, its Git bundle, Cargo.lock and digests.

Prefer read-only verification on the final integration branch. Do not enable four
independent formatter/publisher workflows that race over shared refs or lockfiles.
Do not count a skipped vendor/SSH test as real-agent or remote-workspace proof.

## Final acceptance

Required evidence includes full formatting, compilation, Clippy, workspace tests,
workspace/publisher/roadmap audits, native GUI smoke, focused registry/browser
policy tests, integration-tool self-tests and the relevant SSH/vendor tests.
macOS/Windows compile checks do not establish native interaction acceptance.
Retain the final candidate, exact commands/results, residual gaps and a clean
ownership report. Verify protected refs and the absence of unintended PR/release
side effects before updating the delivery branch.

## Read-only audit hardening

The audit disables Git's configured file-monitor hook for each invocation, disables
external diff/text-conversion helpers for path comparisons, and ignores replacement
objects while checking ancestry. These are invocation-local overrides, not edits
to repository or user configuration. A regression fixture first proves that a
configured benign file-monitor hook runs under ordinary `git status`, then proves
that the audit does not run it, still detects uncommitted work, and leaves HEAD
unchanged. This does not turn an arbitrary repository, Git installation or working
environment into a sandbox or prove semantic compatibility of the branches.

The reviewed EKOP support workflow and GUI acceptance document are now explicit
owned paths. Unrelated workflows and verification documents continue to require
review rather than inheriting a blanket ownership exemption.
