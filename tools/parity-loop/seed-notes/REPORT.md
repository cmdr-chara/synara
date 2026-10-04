# Seed report — shared fixture for upstream + port (work in progress)

## What was built

- `~/parity/harness/fixture.json` — single fixture: repo file trees + commit lists for `atlas-web` (3 commits) and `orbit-api` (2 commits), 2 projects, 4 threads, message turns, relative timestamp offsets, `open_thread`.
- `~/parity/harness/seed.py` — `seed.py <inst>`: stops the pair, wipes only `~/parity/inst/<inst>/{up-home,up-userdata,port-data}`, recreates git repos under `~/parity/fixtures/repos/`, seeds both apps, leaves both stopped. `--launch` additionally launches both via `launch.py` and frames them.

## Storage locations and keys

Upstream (`~/parity/inst/<inst>/up-home`):

- DB: `userdata/state.sqlite`, table `orchestration_events` (append-only event journal; projections replay from `projection_state` cursors at boot). Inserted `project.created`, `thread.created`, `thread.message-sent` rows with `metadata_json={"persistedEventSchemaVersion":1}`, `actor_kind="client"`.
- Server settings: `userdata/settings.json` → `settings.onboardingCompletedAt` (ISO string) suppresses the onboarding tour.
- Renderer localStorage (Chromium profile under `up-userdata`), set via the running app's own UI at seed time:
  - `synara:theme` = `"dark"` via Cmd+K palette "Switch to dark theme".
  - `synara:appsnap-welcome:v1` = `{"acknowledged":true}` via the sheet's dismiss button.
  - `synara:project-import-announcement:v1` = `[<worktreesDir>]` via the announcement's dismiss.
  - `synara:sidebar-ui:v1` → `lastThreadRoute.threadId` via clicking the target thread row.
  - `synara:beta-welcome:v1` not needed — dev build is not flavor `beta`.
- Sidebar layout: upstream fresh-install default (`classic`); not modified.

Port (`~/parity/inst/<inst>/port-data/native-workspace.sqlite3`, `PRAGMA user_version=3`):

- `workspaces`, `projects` — one workspace+project per repo root; chat scratch workspace+project under `port-data/chats/<uuid>/`.
- `tasks` — `data` JSON with `scope` (`project`/`chat`), `state:"completed"`, `agent_id:"opencode"`.
- `events` — `data` = bare `ThreadEvent` JSON (`prompt_started`/`text_delta`/`prompt_finished`), sequence contiguous from 1, `id` = UUID.
- `event_heads` — `{sequence: last, bytes: sum(len(data))}`; `thread_activity` — `{sequence: last, data: {"title","state":"completed","active":false,"permissions":[],"inputs":[],"history_title":null}}`.
- `preferences` — `settings` = AppSettings JSON (`version:1`, `appearance.theme:"dark"`, `onboarding:{started:true,completed:true}`); `selection` = `{project,task}` of `open_thread`.

## Checklist

- [ ] `seed.py seedtest --launch` exits 0 twice in a row — pending
- [ ] `blind.py seedtest ~/parity/work/seed/shot`: both sidebars show `atlas-web`, `orbit-api`, `Brainstorm launch names`, dark theme, no dialogs — pending
- [ ] `blind.py seedtest ~/parity/work/seed/thread`: "Fix login redirect loop" open in both, 4 messages identical — pending
- [ ] no decode errors in `up.log`, no panic in `port.log` — pending
- [ ] `find ~/.synara ~/.synara-canary -newer ~/parity/.start-marker` empty — pending
- [ ] pair stopped at end — pending

## Known differences

- pending
