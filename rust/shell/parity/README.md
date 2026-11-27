# Parity kit

Small tools for comparing the native shell with the Electron/web reference, screen by
screen, at the same size and on the same data. They are dev-only and nothing here ships.

| File | What it does |
|---|---|
| `seed-demo.ts` | Adds a made-up "Test Project" with two chat threads (one with a list, inline code and a code block, one with a very long title) to a reference home. |
| `screens.json` | The screens to compare: web route and native `SYNARA_PARITY_OPEN` value for each. |
| `capture-web.ts` | Captures each web route at an exact viewport (default 1562x1075, dark). It dismisses one-off overlays, and with `--styles` it also dumps `getComputedStyle` values. |
| `compare.ts` | For each screen with `web.png` and `native.png`, it writes `side.png` and `diff.png`, and adds a row to `report.md` with the share of differing pixels. |

## 1. Same data on both sides

Both apps read one home. Seed it once, with the server for that home stopped:

```sh
bun rust/shell/parity/seed-demo.ts <home>/dev/state.sqlite /tmp/parity-workspace
```

The seeder only appends orchestration events, and the server projects them on its next
start. The threads get fixed ids `11111111-aaaa-4aaa-8aaa-00000000000{1,2}`, which is what
`screens.json` uses. Settings pages need no seed: `/settings?section=<id>` opens them on
the web side.

## 2. Capture the web reference

Start the reference on that home, for example `bun run dev --home-dir <home>` on `main`.
Then run:

```sh
bun rust/shell/parity/capture-web.ts out/ --only chat-thread,settings-general \
  --styles '[data-chat-composer-surface], [data-chat-composer-surface] button'
```

It uses the Playwright that `apps/web` already depends on. If Chromium is missing, run
`bun run --cwd apps/web test:browser:install`, or set `PARITY_CHROMIUM` to an existing
Chromium executable.

## 3. Capture the native shell

Start the shell on the same home. `SYNARA_PARITY_OPEN` opens straight onto one screen,
about 2.5s after launch:

```sh
SYNARA_PARITY_OPEN=thread:11111111-aaaa-4aaa-8aaa-000000000001 synara-shell
SYNARA_PARITY_OPEN=settings:appearance synara-shell
SYNARA_PARITY_OPEN=rail:automations synara-shell   # inbox, tasks, kanban, pull-requests, automations, hubs, spaces
```

Size the window to the web viewport, then capture only that window and save it as
`out/<screen>/native.png`:
- macOS: `screencapture -o -l <window-id> out/<screen>/native.png`.
- Linux/Hyprland: `grim -g "<x>,<y> <w>x<h>" out/<screen>/native.png`, using the window
  geometry from `hyprctl clients -j`.

## 4. Compare

```sh
bun rust/shell/parity/compare.ts out/ --tolerance 8
```

Open `out/<screen>/side.png` and `diff.png`; `out/report.md` lists the scores.

## Things that skew a comparison

- **Viewport width:** in-app browser panes can lay the web UI out narrower than the
  window. The transcript uses `scrollbar-gutter: stable`, so the chat column moves.
  `capture-web.ts` uses a real headless viewport of exactly the given size.
- **Overlays:** desktop notification toasts land on screen-region captures. Capture the
  window itself, not the screen.
- **Theme:** the native window can paint a frame or two in the light theme before the
  saved dark theme applies. Wait a few seconds after launch before capturing.
- **Fonts:** "system-ui" maps to different fonts per OS. Compare the positions of glyph
  boxes, not anti-aliasing.
- **Fractional display scaling** (e.g. 1.25) makes the native capture smaller in
  logical pixels. Scale it back to the web size before comparing; `compare.ts` scales
  and notes it.
