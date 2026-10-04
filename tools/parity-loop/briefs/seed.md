# Brief: shared seed data for upstream and port

First read `~/parity/CONTEXT.md` in full and follow its hard rules.

## Goal

Write `~/parity/harness/seed.py <inst>` that loads ONE fixture (`~/parity/harness/fixture.json`) into
both apps of an instance pair, so that after launch both apps show the same data:

- 2 projects backed by real git repos under `~/parity/fixtures/repos/` (create them with the script:
  `atlas-web` with a small React-ish tree and 3 commits, `orbit-api` with a small Go-ish tree and 2 commits).
- In `atlas-web`: thread "Fix login redirect loop" (2 user turns, 2 assistant replies; one reply has a
  heading, a bullet list, inline code and a fenced code block), thread "Add dark mode toggle"
  (1 user + 1 assistant). In `orbit-api`: thread "Investigate flaky payments test" (1 user + 1 assistant).
  One standalone chat (no project) "Brainstorm launch names" (1 user + 1 assistant).
- Fixed timestamps relative to seed time (e.g. now-3h, now-1d, now-2d) so relative labels match.
- Dark theme, onboarding / welcome / announcement dialogs marked complete or dismissed in BOTH apps,
  so both open straight to the main shell. Same sidebar layout in both (whatever upstream's default
  is for a fresh install; check upstream source for the default — do not change upstream's defaults
  except theme = dark and first-run dialogs dismissed).
- Same selected / open item on launch if both apps support it (otherwise leave both at home).

## How (research first)

- Upstream (`~/parity/upstream`): server is event-sourced. Events live in `orchestration_events`
  (see `apps/server/src/persistence/Migrations/001_OrchestrationEvents.ts`) and projections replay
  from the event log on startup (`apps/server/src/orchestration/Layers/ProjectionPipeline.ts`).
  Event payload schemas are in `packages/contracts/src/orchestration.ts`. Find the DB path under
  `SYNARA_HOME`. Recommended: launch upstream once (`launch.py up <inst>`) so migrations run, stop it,
  insert events with exact schema-valid payloads, relaunch, confirm. Client-side settings (theme,
  dismissed dialogs) may live in Electron localStorage / server settings files; find out from source
  (`apps/web/src/appSettings.ts`, `apps/web/src/onboarding`, `apps/server/src/serverSettings*`).
  If a value lives only in Chromium localStorage, set it through the running app via its own UI or
  via a documented server settings file, never by hand-editing LevelDB unless there is no other way.
- Port (`~/Projects/synara-gpui-pr11`, read only): SQLite `native-workspace.sqlite3` in the data dir.
  Read `.agents/skills/testing-synara-app/SKILL.md` section "Seeding conversation events" and the
  storage code in `crates/synara-app/src/storage*` / `crates/synara-core`. Projects need real git repos.
  The app must be stopped while you write.
- `seed.py` must be idempotent: `seed.py <inst>` stops the instance pair, wipes ONLY
  `~/parity/inst/<inst>/{up-home,up-userdata,port-data}`, re-seeds, and leaves both apps stopped.
  `seed.py <inst> --launch` also launches both (via launch.py) and frames them.

## Acceptance checklist (all must pass)

1. `python3 ~/parity/harness/seed.py seedtest --launch` exits 0 twice in a row.
2. `python3 ~/parity/harness/blind.py seedtest ~/parity/work/seed/shot` then open `A.png` and `B.png`
   with your image viewer tool: both show the two project names and the chat in the sidebar, dark theme,
   no onboarding/welcome dialog.
3. In both apps, opening "Fix login redirect loop" shows all 4 messages with the same text (capture both
   with `blind.py` into `~/parity/work/seed/thread` and look at them).
4. No upstream log errors about event decoding (`~/parity/inst/seedtest/up.log`), no panic in `port.log`.
5. Nothing written outside `~/parity` (check `find ~/.synara ~/.synara-canary -newer ~/parity/.start-marker` is empty).
6. Stop the pair at the end: `python3 ~/parity/harness/launch.py stop seedtest`.

## Output

Write `~/parity/work/seed/REPORT.md` first (create it at the start, update it as you go): what you built,
exact storage locations and keys for both apps, the checklist with PASS/FAIL and the real command output
you saw, and anything that differs between the two apps' seeded state. Max 600 words.
Owned files: `~/parity/harness/seed.py`, `~/parity/harness/fixture.json`, `~/parity/fixtures/`, `~/parity/work/seed/`.
Do not edit any other file. Do not edit port or upstream source.
