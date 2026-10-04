---
name: testing-synara-app
description: How to launch and drive the Synara Rust/GPUI macOS app for end-to-end UI verification (launch flags, isolated data dirs, AX-tree interaction quirks, settings navigation).
---

# Testing the Synara GPUI macOS app

## Launch

- Binary: `cargo build --locked -p synara-app` → `target/debug/synara-app`. Toolchain: `~/.rustup/toolchains/1.98.1-aarch64-apple-darwin/bin/cargo`.
- Flags: `--data-dir DIR` (SQLite + all state; use a fresh dir for deterministic state), `--workspace DIR` (pre-registers a project — requires the dir to be a git repo), `--agents FILE.json` (custom agent profiles array).
- Launch detached: `nohup ./target/debug/synara-app --data-dir /tmp/x/data --workspace /tmp/x/repo > /tmp/x/app.log 2>&1 &`. Check `app.log` for panics (`grep -i panic`).
- A workspace owner lock is held on `native-workspace.sqlite3` — kill any prior `synara-app` instance before relaunching the same data dir. `pgrep -fl synara-app` shows the real PID (skip the `bash` wrapper line).
- Fresh data dir + no projects → app opens directly into the onboarding flow (Settings panel). With `--workspace` → opens the Conversation panel with the project selected and a "New task" created.
- Maximize the window for recording via AppleScript (GPUI uses client decorations; wmctrl does not exist on macOS):
  `osascript -e 'tell application "System Events" to tell process "synara-app" to set position of window 1 to {0, 25}' -e 'tell application "System Events" to tell process "synara-app" to set size of window 1 to {1600, 1175}'`

## Driving the UI

- The `computer` tool's `target=macos` AX tree works: buttons, radiobuttons, menuitems, and headings are queryable and pressable via `query`/`act`/`press`. After app restart, re-query by new PID (`app: "<pid>"`) — name-based queries can hit the stale dead pid.
- Text inputs (composer, settings "Search settings…" box, provider search) are NOT in the AX tree — `role=text field`/`text area` return nothing. Click them by screenshot coordinates, then `type`.
- Scrollable content scrolls via `mouse_move` over the area + `scroll` action.
- Clicking a docs/external link (`cx.open_url`) opens the system browser (Safari) which steals focus and covers the app — return focus with `osascript -e 'tell application "System Events" to set frontmost of process "synara-app" to true'`.
- Transient macOS notification banners may overlay the window's top-right; they don't affect the app.
- When no agent CLI is installed, sending a prompt fails with a red "I/O operation failed: No such file or directory (os error 2)" banner (agent spawn) — this is env-expected, and the composer KEEPS the draft text after a failed send; clear with ctrl+a + BackSpace before typing the next prompt.

## Navigation cheatsheet

- Settings: "Settings" button (gear) at bottom-left of the sidebar nav → settings panel with groups Personal / Integrations / Coding / System / Archived.
- Non-primary sections (Direct models, Device / capture, Privacy & security, Plugins & integrations, Subagents & workflows, Project import) are NOT listed in the sidebar by default — type in the settings search box to reveal them (e.g. "direct"), or use the composer command `/synara/settings <section>` (e.g. `direct-models`, `worktrees`).
- Worktrees: Settings → "Managed worktrees" (Coding). Shows the "Delete worktree on archive" card + repository panel for the selected project.
- Direct models: search "direct" → "Direct models" (Integrations). "Load models.dev catalog" does a live models.dev fetch (works on this box); afterwards the header reads "Catalog: N providers, M compatible metadata candidates — snapshot saved … ago". The snapshot persists as preference `direct-model-catalog-v1` (JSON `{source, stored_at_ms}`) and hydrates on section open without network — verify across an app restart.
- Onboarding: Settings → "Getting started" (Personal) → "Continue" × 2 → step 3 "Agents" lists every default profile (OpenCode, Gemini CLI, Oh My Pi) with per-provider sign-in guides. Guides render even when the CLI isn't installed ("Command not found" — `command_found` only gates the copy-command line and Connect button).
- Agent providers: Settings → "Agent providers" (Coding) lists the provider-order rows; the composer "Model and agent" picker (button in the composer toolbar) has a "Model sources" radiobutton tab per profile (Starred / OpenCode / Gemini CLI / Oh My Pi).

## Native slash commands (namespaced)

- All built-in commands require the `/synara/` prefix — `/synara/debug`, `/synara/default`, `/synara/plan`, `/synara/model next|previous`, `/synara/settings <section>`. A bare `/debug` is NOT intercepted (deliberate: never shadows provider commands) — it sends as a normal prompt, so absence of a command menu while typing it is the tell.
- Typing `/` alone in the composer opens the native command menu; it also appears once text starts with `/synara/`.
- Task switching for persistence checks: the sidebar "Chats" list only shows chat-kind threads (project tasks like the `--workspace` "New task" are NOT listed). Use the "Search all threads" toolbar button (top of window) — its menu lists every task as a menuitem with `[selected=true]` on the active one; press to switch.
- Debug interaction mode: `/synara/debug` sets a per-task mode → a "Debug" badge chip renders on the workflow strip under the header; clicking the chip clears it. The composer "+" ("Add" button, AX id `composer-extras`) Extras menu has a "Debug mode" row whose `selected=true` state shows a checkmark. `/synara/default` clears it.

## Verifying persistence

- SQLite: `sqlite3 <data-dir>/native-workspace.sqlite3 "SELECT key FROM preferences"` — `settings` holds general.* prefs (e.g. `json_extract(data,'$.general.delete_worktree_on_archive')`), `direct-model-catalog-v1` holds the models.dev snapshot.
- Per-task interaction mode persists as `task-interaction-mode:{task-uuid}` = `"debug"`/`"default"`; `selection` holds `{project, task}` UUIDs; `tasks` table holds task rows (title/state/agent_id extractable via `json_extract`); `task-context:{task-uuid}` holds seeded hub-instructions notes JSON (`{"version":2,"revision":N,"notes":"...","checklist":[],"folder_references":[]}`); `automation-ledger-v1` holds the automations ledger (`{definitions:[...], runs:[...]}`).

## Seeding conversation events (for message-action checks without an agent CLI)

- The `events` table is replayed into the transcript on thread open. To fabricate a completed assistant exchange, the app MUST be stopped first, then insert rows (sequences starting at 1, contiguous) AND maintain the two consistency tables or startup recovery aborts with "stored event sequence or identity is inconsistent":
  - `events(thread_id, sequence, id, timestamp_ms, data)` — `id` MUST be a valid UUID (use `uuidgen | tr A-F a-f`, not arbitrary strings). `data` is ThreadEvent JSON: `{"type":"prompt_started","turn":"t1"}`, `{"type":"text_delta","message_id":"m1","role":"user|assistant","text":"..."}`, `{"type":"prompt_finished","reason":"end_turn"}`.
  - `event_heads(thread_id, sequence=<max seq>, bytes=<sum(length(data))>)`.
  - `thread_activity(thread_id, sequence=<max seq>, data)` — data = `{"title":<task title>,"state":<replayed state>,"active":false,"permissions":[],"inputs":[],"history_title":null}`; after `prompt_finished` the state is `"completed"`, and the `tasks` row's `data` `$.state` must match it (non-archived), and `$.title` must match.
- With no agent CLI, a completed message needs this seed; the message action row then renders icon buttons (Copy/side-chat/Pin/Reply) queryable via AX `text=message`.

## Gotchas hit while testing

- `EntryMode::Editor` TextEntry fields collapsed to thin empty rectangles before commit `0d27ae880` (`h_full()` resolved against auto-height parents; fixed via `min_h(visible_height())`). On older builds verify content functionally: the notes dialog's "Copy" button puts `## My notes\n\n<text>` on the clipboard (`pbpaste`), and saved automation `instructions` land in the `automation-ledger-v1` JSON. On fixed builds the fields paint text and accept live edits normally.
- AX `press` on a button inside a scrollable panel silently no-ops when the button is scrolled out of view (e.g. automations editor "Save paused"). Scroll the panel until the button is visible, then real-click it by coordinates.
- Studio mode entry: brand button "Synara ⌄" (id `synara-menu`) at ~(30,80) → menuitem "Hubs". Once in studio mode the same spot is the "← Synara" back button — clicking it exits to Synara mode.
- Thread rows show a hover-revealed "Pin thread" button (AX `role=button text=Pin thread`, press via AX — coordinate clicks land on the row and just select). Pinning moves the row into the flat "Pinned" section above Projects and out of its project group/Chats.
- "Attach window" (extras menu) opens the AppSnap capture card in the composer; on this box it renders a platform-unsupported notice — surface opening is the verifiable part.
