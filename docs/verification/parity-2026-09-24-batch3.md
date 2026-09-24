# September 24: setup, isolated forks and model controls

Base: `6b84417` (published editor/Studio batch). Upstream remains
`eaa61eded31b6755d4f30ba8eabc5d905cf817cb`, checked live on September 24.
Upstream comparison: getting-started onboarding/provider documentation,
`apps/web` model-cycle tests and composer keyboard documentation. Existing
native ACP, task creation, Git process/consent and input owners are reused.

## Delivered slices

1. Empty-install setup can prepare an independent saved, unsent chat for a
   configured local agent. Preparation does not start it. Explicit Connect uses
   the normal Controller, then only advertised authentication methods are shown.
   Connection-scoped questions reuse the native interaction UI and the existing
   task terminal can be opened. Replies are task/selection fenced. Setup chats
   persist and never replay authentication or prompts on restart. Connection
   status is reported protocol state, not inferred billing/account health.
2. An assistant-message fork can review a generated local worktree path, branch
   and exact source commit, then explicitly approve repository checkout execution.
   The checkout uses the owned, bounded Git helper with hooks, credentials,
   signing and network helpers disabled. Source dirty/index state is untouched
   and excluded. Revalidation refuses changed HEAD/owner, active source, occupied
   or symlink destination/parent and duplicate confirmation. The unsent task/draft
   insert is atomic and its persisted cwd participates in existing removal guards.
   Checkout and SQLite are not one transaction: failure preserves the exact
   reported worktree/branch for manual recovery rather than deleting files.
3. Alt+[ / Alt+] cycle live-advertised ACP models only in the main composer.
   Menus, IME/character-preferred input, held keys and other editors retain their
   input. Qualified `/synara/model next` / `previous` use the same control owner,
   refuse extra arguments and never send a provider prompt. Direct-model tasks
   remain on their separately reviewed switching path.

## Verification contract

Focused service tests exercise denied checkout, pinned content with dirty source
preservation, duplicate refusal, stale HEAD and cancellation, task/draft reopening
and assignment. Parser tests protect the native command namespace. The native
`native_setup_worktree_controls_smoke.py` exercises empty setup, inert task
preparation, fixture authentication, model shortcuts/commands, reviewed new
worktree creation and restart. Integrated checks cover only workspace/app,
strict Clippy, formatting and the roadmap. No dependency or lockfile changes.

Results are recorded below only after actual validation. A successful fixture
login is not live provider acceptance. No SSH managed creation, automatic
worktree deletion/reconciliation, provider-native session transfer, PDF renderer,
Simulator touch input or non-Linux acceptance is claimed. All 21 gates stay OPEN.

## Observed validation

129 app tests, 308 workspace unit tests, three settings integration tests and strict Clippy passed in run 35990594053. One process fixture and six isolated SSH tests were intentionally ignored.
Rust source and manifests were verified byte-identical by Git-index manifest SHA-256 `72df2db87a1213b162743a128cafaad819f3d728ca8106c297c54de80e15cfdd` before reusing those successful checks.
The native journey, formatting and roadmap checks passed before publication in https://github.com/cmdr-chara/synara/actions/runs/35991478509
Run attempt: 1. Publication base: `df182f05dd882a6daa00f31c40c33b3e91f2d64a`.
The initial native run reached restart with the correct selected task and persisted worktree. Its test incorrectly compared a path with a trailing slash against a normalized string. The corrected test compares Path values and separately asserts task identity; no Rust change or weakened workflow requirement was needed.
Linux/X11 native evidence covers fresh setup preparation, explicit fixture Connect and advertised sign-in, composer-only model cycling, preserved drafts, reviewed committed-HEAD worktree forks and inert restart. Worktree unit evidence covers denied consent, stale HEAD, cancellation, duplicate confirmation, retained recovery paths and symlink refusal. No live provider, non-Linux, SSH creation or automatic-cleanup acceptance is claimed.
