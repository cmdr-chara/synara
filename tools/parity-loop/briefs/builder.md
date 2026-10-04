# Builder brief: piece {PIECE}, round {N}

Read in full before acting: `~/parity/CONTEXT.md`, `~/parity/pieces/{PIECE}/PIECE.md`.
Then read the previous verdict if it exists: `~/parity/rounds/{PIECE}/r{PREV}/verdict-unsealed.md`
(it names the single biggest remaining gap; fix that first, then anything else you see).
Also read `~/parity/pieces/{PIECE}/NOTES.md` if it exists (orchestrator notes; they override this brief).

## Your worktree

`~/parity/wt/{PIECE}` on branch `parity/{PIECE}`. Edit only files in the piece's scope. Commit there.
Never push. Never touch `~/Projects/synara-gpui-pr11` or other worktrees.

## Loop (time box: about 60 minutes of work, then stop and report)

1. Study the upstream source named in PIECE.md. Take exact strings, sizes (Tailwind classes ->
   px at the default 13px/16px base, check `index.css`), colors (CSS variables), icons, spacing, order,
   hover / active / disabled states, tooltips, AX labels. Upstream is the spec. Do not invent.
2. Implement in the port with the port's existing UI building blocks. Delete port-only things in scope
   (PIECE.md says what). Smallest complete diff. No stubs, no TODOs, no dead code.
3. Build (see CONTEXT.md), then `cp ~/parity/target/debug/synara-app ~/parity/bin/{PIECE}-r{N}`.
4. Scenario: `~/parity/pieces/{PIECE}/scenario.py` must drive both apps into every required state in
   PIECE.md. Create or fix it (format: see `~/parity/harness/run_scenario.py` docstring and
   `~/parity/harness/drive.py`). Make steps robust: check `exists()` before optional clicks.
   Run it (GUI lock!):
   `~/parity/harness/guilock.sh {PIECE}-builder python3 ~/parity/harness/run_scenario.py {PIECE} --bin ~/parity/bin/{PIECE}-r{N} --out ~/parity/rounds/{PIECE}/r{N}/builder-shots`
   Open each `upstream.png` and `port.png` with your image viewer and compare them yourself, region by
   region. Fix what differs. Repeat build + scenario until you cannot find a difference in scope or the
   time box ends.
5. Verify: `cargo fmt --check`, `cargo clippy --locked -p synara-app --all-targets -- -D warnings`,
   and `cargo test --locked -p synara-app` plus tests of any other crate you touched. All green.
   Use `CARGO_TARGET_DIR=~/parity/target RUSTUP_TOOLCHAIN=1.98.1-aarch64-apple-darwin`.
6. Make sure `~/parity/bin/{PIECE}-r{N}` is the binary of your final commit (rebuild + copy if needed).
7. `git add -A && git commit` in your worktree with a clear message. One or a few focused commits.

## Report

Write `~/parity/rounds/{PIECE}/r{N}/builder.md` (create it at the start, update as you go, max 500 words):
commit SHA(s), what changed (file:line), what you deleted, verification commands and their real result,
the scenario states you captured, remaining differences you saw but did not fix. If a step failed, say so.
