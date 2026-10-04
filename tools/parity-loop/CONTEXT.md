# Synara GPUI parity program: shared context for every worker

Read this whole file before you act.

## Goal

The Rust + GPUI port (`cmdr-chara/synara`, PR #11, branch `devin/1790450414-upstream-parity-batch43`)
must match upstream Synara (`Emanuele-web04/synara`, current main) 1:1: every screen, flow, setting,
menu, shortcut, empty state, animation and string. Anything the port has that upstream does not
have, or that is odd or useless, gets deleted.

## Paths

| What | Where |
|---|---|
| Upstream source (current main, read-only, built) | `~/parity/upstream` (React web app in `apps/web/src`, contracts in `packages/contracts/src`) |
| Port integration worktree (PR #11 branch) | `~/Projects/synara-gpui-pr11` (do not edit unless your brief says so) |
| Your own port worktree (builders) | named in your brief, under `~/parity/wt/<piece>` |
| Harness | `~/parity/harness` (`launch.py`, `blind.py`, `lib.py`, `seed.py` once it exists) |
| Instance data | `~/parity/inst/<inst>/` (upstream home, upstream userData, port data dir) |
| Round output | `~/parity/rounds/<piece>/r<N>/` |
| Blind keys | `~/parity/keys/` — CRITICS MUST NEVER READ THIS DIRECTORY |

## Hard rules

- Never read, write or delete `~/.synara`, `~/.synara-canary`, `~/.cache/synara-canary`, or anything
  under `~/Library/Application Support/synara*`. Each app instance uses its own folders under `~/parity/inst`.
- Never `git push`, never force anything, never touch other worktrees or branches than the one in your brief.
- Never kill processes you did not start (other than via `launch.py stop <your inst>`).
- Disk is tight (about 15 GB free). Do not create extra Rust target dirs. Builders use the shared
  `CARGO_TARGET_DIR=~/parity/target`. Do not copy the 200 MB binary more than your brief says.
- Toolchain: run cargo as `RUSTUP_TOOLCHAIN=1.98.1-aarch64-apple-darwin cargo ...`
  (it is symlinked to stable 1.99 on this Mac; CI uses 1.98.1 on Linux).
- Native SDK quirk: upstream native addons need `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk`
  (already handled by `launch.py`).
- No emojis. No placeholder or stub code. No fake data paths in product code.

## Running the two apps

```sh
cd ~/parity/harness
python3 launch.py up   <inst>                     # upstream Electron, isolated home + userData
python3 launch.py port <inst> --bin <path/to/synara-app>   # port, isolated --data-dir
python3 launch.py frame <inst>                    # both windows to 1440x900 at (0,40)
python3 blind.py <inst> <out_dir> --tag <tag>     # capture both windows -> A.png, B.png, side-by-side.png (random order)
python3 launch.py stop <inst> [up|port|all]
```

`~/parity/inst/<inst>/state.json` holds pids and window ids. Drive the apps with `cua-driver`
(`cua-driver call get_window_state '{"pid":P,"window_id":W}'`, `click` with `element_token`,
`press_key`, `hotkey`, `type_text`, `scroll`). Prefer AX actions (no focus steal). Electron's AX tree is
rich. The GPUI port's text inputs are not in its AX tree; click them by coordinates from a fresh
`get_window_state` screenshot. See `~/Projects/synara-gpui-pr11/.agents/skills/testing-synara-app/SKILL.md`.

Window capture: `screencapture -x -o -l <window_id> out.png` (works while the window is covered).

Scenario helper `~/parity/harness/drive.py` (`App.from_state(inst, side)`): `click(label)` does an AX
press (works for Electron). Many GPUI elements have no AX press action: use `click(label, pixel=True)`
(clicks the element's frame center, briefly fronting the port window). `click_xy` coordinates are in the
pixel space of the latest `snap()` / `screenshot()` PNG (it is downscaled; not 2x). Port-side mouse/keys
are sent with foreground delivery automatically. Set `PARITY_CUA_SESSION=<your-piece>` in your env so
cua-driver captures are scoped to you. Upstream may show a "Welcome to Synara" onboarding modal a few
seconds after a fresh launch until the seed marks onboarding done; wait for it and dismiss it if present.

GUI lock: only one agent drives app windows with coordinate clicks or keys at a time. Wrap those
sessions in `~/parity/harness/guilock.sh <your-name> <command...>` or hold the lock with
`mkdir ~/parity/.guilock` (remove it with `rmdir` when done; wait and retry if it exists).

## Port build

```sh
cd <your worktree>
CARGO_TARGET_DIR=~/parity/target RUSTUP_TOOLCHAIN=1.98.1-aarch64-apple-darwin cargo build --locked -p synara-app
```

The shared target dir means `~/parity/target/debug/synara-app` is overwritten by whoever builds last.
Right after your build, copy it to `~/parity/bin/<piece>-r<N>` and launch that copy.

## Port verification commands (all must stay green)

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```
