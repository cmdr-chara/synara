# Shell priorities

## HIGH — Temple backdrop fidelity vs upstream web app

Progress 2026-11-26 evening: dot rendering is now faithful (device-scale
rasterized circles with the per-cell radial mask, nearest-neighbor crisp,
area-averaged scene downscale via SAT); scene geometry uses content-pane
dimensions; transparent titlebar aligns the vertical origin (roofline
matches within 1px; temple/sky-mid coverage match within ~1.5%). Composer
has the opaque shell backing.

Remaining known divergences to verify and close:

- Hills/bottom band: native still denser than the reference (bottom ~56%
  vs ~29% coverage) and the dense areas read blobbier — investigate the
  unsharp-mask radius units (cells vs px) and the thin/quiet interplay in
  the bottom band.
- Landing layout: upstream composer includes the "Work in a project"
  project-picker header row and the composer column metrics differ; measure
  upstream composer x/y/width and match.
- Dark mode never visually verified against upstream; also the full-window
  dimming (opacity 0.6, brightness 0.75, saturate 0.6) is not ported.
- Shell chrome strips: upstream paints the shell atmosphere at opacity 0.35
  (0.8 dark) clipped to the chrome strips even in default mode; the native
  only renders the landing instance. Ripples and the gradient style are
  not ported.
- Top strip height is 47px vs upstream 44px (verify exact upstream strip
  height in Electron rather than the browser reference).

Method that works: side-by-side screenshots of web app and native shell at
the same window size, then pixel-sample ink/paper/density in matched
regions. For the web reference use CDP Emulation.setDeviceMetricsOverride
— the browser window size is NOT the viewport. Do not eyeball.

## Overnight pixel-perfect program (deadline 13:00, user asleep)

User mandate 2026-11-27 00:34: "fai pixel perfect TUTTE le sezioni frontend" —
systematic anaglyph pass on every shell surface vs the web reference.
Child session 4d0502480de04b9d99c9568a648d279d owns ALL backend completion in parallel.

DONE tonight:
- Two-canvas backdrop structure (commit 5f24209efc): atmosphere full-window at
  35%/80% under pane-sized landing texture. Temple mass now aligns in overlay;
  mid-hills within 1.2%, bottom-right exact.

OPEN, in priority order:
1. Sky density: web upper sky reads denser than shell in anaglyph. Suspects:
   (a) sky_alpha ramp coordinate units (logical vs device px in raster loop),
   (b) atmosphere/landing cell-grid phase interplay, (c) dot alpha blending.
   Measure with alpha-aware detection (pale dots fail naive ink thresholds).
2. Transcript surface pass — needs same thread in both clients. Web test
   session URLs are like /4dbd7265-....; drive shell via computer tool clicks,
   capture, anaglyph, fix worst fringes. Areas: message bubbles, markdown,
   code blocks, activities rows, composer-in-thread (permission/branch/model row).

PROGRESS LOG (overnight):
- Settings/General: 65% -> 8.8% pixel diff after opaque-surface fix
  (5510376856). REMAINING: section vertical rhythm — web section content sits
  ~15px lower than shell; header top padding + section gap need exact matching.
  Sidebar nav aligns well. Next: measure exact y positions of General title /
  first card in both captures and adjust settings pane padding.
- Web settings IS a route after all: /settings works (earlier redirect was a
  fluke). Navigate web by URL where possible — much cheaper than JS clicks.
- USER CLARIFICATION 00:41: GPUI must be IDENTICAL to Electron — not
  'close enough'. Micro-offsets count. Every surface iterates to convergence.
3. Sidebar/inbox/kanban/tasks/codereview/automations/hubs/settings — same method,
   one surface at a time. Settings is static content: cheapest high-value target.
4. Composer fine metrics: input zone padding/height, chip row spacing vs upstream.
5. Dark mode backdrop — never verified end to end.

Method (tools/pixel-perfect/): capture.sh + anaglyph.py — red=web reference,
cyan=shell; grey means matched. Both at 1600x1088. NEVER eyeball — measure.
Web capture requires CDP 57012 tab on the target surface; viewport override
must be re-applied after any navigation; wait ~7s for backdrop rebuild.

SETTINGS EXACT SPEC (measured in live web via CDP, 1600x1088):
- Content column: `mx-auto w-full px-6 py-8 max-w-2xl` — centered in pane,
  max width 672px, padding x 24 top 32. At 1600 window: x=417 y=44 w=672.
- Header row: `mb-8 flex items-start justify-between` — title y=76,
  32px bottom margin before first section.
- First card top edge web y=197 vs shell y=188 — shell is 9px high.
- Scroll container is the full pane (x 308, w 889).

NOTE on diff metric: 8.65% at thresh 18 with aligned geometry — cross-rasterizer
text AA inflates it. Next refinement: measure diff EXCLUDING text pixels, or
diff on blurred images (geometry-only). reddiff/anaglyph on downsampled pairs.

NEXT ACTION (post-compaction pickup): transcript surface pass. Blocker hit:
test threads vanished from shell sidebar (project shows 0 chats) though web
tabs still show sessions /4dbd7265 /4c8deb08 /57a0ee49 /a903e821. State db
location uncertain (state.sqlite unreadable/empty via sqlite3 cli — likely
locked or wrong path; backend pid running as ./target/debug/index). FASTEST
unblock: create a fresh test thread in the web app UI (composer input burns
no quota if echo provider configured — verify first), or send a message via
the backend RPC directly. Alternative: point both clients at the same session
by navigating web tab to a thread then finding it in shell sidebar under its
project. ALSO: /settings IS a valid web route — use URL navigation for all
web surfaces where routes exist (cheap).

CONFIRMED LIVE: upstream .app-shell-atmosphere opacity 0.35 light, always
rendered behind everything (even thread pages). Web / redirects to the last
active thread — to capture the empty-chat landing, first close/archive test
threads or use a fresh home; sky profile measurement needs the real landing.
SKY PROFILE data point (earlier valid capture): web sky density peaks y150-200
then dips — non-monotonic scene-driven; shell ramp monotonic too slow early,
overshoots y400+. Hypothesis to test next cycle: shell vertical scene mapping
or the unsharp-mask vertical ramp (sharpenFrom/To 0.24/0.36 of scene height)
differs — verify the sharpen smoothstep math uses scene-height fractions
correctly in device vs logical units.

INBOX pass (web_inbox/shell_inbox/anaglyph_inbox in /tmp/pxcmp):
- Structural: shell content sits ~15-20px too HIGH vs web (same rhythm class
  as settings had). Extract exact paddings from upstream inbox component and
  match: top padding of the pane, section gaps, row heights.
- Web rows in Ready-for-review are TALLER (title + description lines?) — check
  upstream InboxView row structure vs shell's single-line rows.
- Shell shows "Mark all read"/"Try again" actions web doesn't show in the same
  spots — verify against upstream conditions.
- Data note: web showed a task entry (5h) the shell didn't — sync/timing,
  not layout.
- geodiff r3: mean 2.73, >10: 3.7% — geometry close; rhythm is the gap.

SYSTEMATIC PATTERN for remaining surfaces (kanban/tasks/codereview/automations/
hubs/transcript/composer): measure upstream metrics via CDP getBoundingClientRect
+ computed style on the live web app (cheap, exact), then apply to the shell
port — do NOT eyeball screenshots except to verify.

INBOX EXACT SPEC (live CDP, 1600x1088):
- Column: mx-auto max-w-[1760px], padding 40px (p-10 @2xl), gap 28px (gap-7)
- h1 'Inbox': text-2xl->3xl = 30px medium-500, first baseline content y=84
  (44 pane top + 40 padding)
- Header: flex flex-wrap items-end justify-between gap-3 (12px)

INBOX LAYOUT — structural divergence (bigger than rhythm):
Upstream InboxView.tsx is a container-query adaptive layout:
- LEFT column: Ready-for-review card THEN Today's-tasks card stacked
- RIGHT column: recap/digest cards ('The recap didn't load' + Try again)
- Shell currently: top row [tasks | nothing-ran] + full-width review list — WRONG
- Tiles: rounded-2xl bg-muted sections; grid gaps gap-4/@lg:gap-7 (16/28px)
- Also missing in shell: 'Mark all read' top-right action
- Component spec: apps/web/src/components/inbox/InboxView.tsx — port the grid
  structure faithfully (task column + recap column), not just spacing.
Shell padding fix (p-10/gap-7) already in, uncommitted at time of writing.

## Progress log (cont.)
- `fa313681d0` in-thread composer = upstream tray: white pill (rounded 9999) inside opaque grey squircle 24.2 spanning chat col; Full-access/model chips ON the grey, not floating. Verified vs web crop.
- Transcript comparison session: both clients on "Sessione piccola" (web thread id ddf3cb33-8a94-4f9a-b64b-43bef61fd9e4, reachable by clicking inbox row). Captures: /tmp/pxcmp/{web,shell}_thread.png + anaglyph_thread.png.
- NEXT: transcript content column too wide in shell — upstream constrains assistant content to 46rem (728px measured firstP x=384 w=728 at 1600w); shell spans nearly full pane. Also user-attachment layout (image+pdf chips) diverges; "Worked for 11s" row missing in web capture position. Scroll-sync problem: captures differ in scroll — compare structurally not pixelwise until scroll synced.

## HANDOFF STATE (nov 27 ~01:15, pre-compaction dump)
Deadline 13:00. Mandate: pixel-perfect ALL frontend sections, identical to Electron. Child session does backend.

### Verified facts this hour
- `/tasks` is NOT a web route (redirects to thread). Routes are nested under chat shell; only `/settings` `/inbox` confirmed as top-level. Non-thread surfaces (tasks/kanban/automations/pull-requests/hubs) are reached via RAIL ICONS in both clients, not URLs.
- Transcript horizontal metrics ALREADY MATCH: text ink x=338..1099 identical both clients (measured). The composer tray now matches too (`fa313681d0`).
- Anaglyph vertical mismatches in transcript were SCROLL desync, not layout. For valid comparison must scroll-sync both (web via JS scrollTop; shell autoscrolls to bottom on open).
- Shell sidebar border at x=307 (1600w). Web rail shows dither texture noise x<40.
- web nav: clicking inbox rows via JS works (row.click() then location.pathname changes). Thread "Sessione piccola" id = ddf3cb33-8a94-4f9a-b64b-43bef61fd9e4.
- Shell rail icon y-coords (1024x768 computer space): home 62, inbox 92; more icons below (tasks ~122?, kanban, automations...). Verify visually.
- Web rail navigation: evaluate JS click on rail buttons. Rail aria labels unknown — inspect DOM `document.querySelectorAll('a[href],button')` in the rail container to find targets.

### Remaining surface passes (do in order)
1. transcript fine detail — scroll-sync first, then compare: attachment layout (image+pdf chips diverge), user bubbles, activity rows, timestamps
2. kanban — navigate via rail both sides
3. tasks list — same
4. pull-requests (code review)
5. automations
6. hubs
7. settings sub-pages beyond first (20+ sections in shell sidebar)
8. dark mode — NEVER verified yet, do a full pass
9. inbox two-column restructure (left review+tasks stacked, right recaps; spec in earlier section of this file)
10. sky density backdrop refinement (low priority per user)

### Method reminders
- harness: /tmp/pxcmp/*.py, captures via CDP port 57012 + screencapture -R0,30,1600,1088
- ALWAYS `git pull --rebase origin feat/rust-port` before push (child pushes constantly); commit as Commander Chara with --no-verify
- shell relaunch: pkill -f synara-shell; SYNARA_URL=http://127.0.0.1:8580 SYNARA_HTTP=http://127.0.0.1:8580 SYNARA_HOME=/tmp/synara-bisect ./target/debug/synara-shell
- backend: cd rust/app && PORT=8580 SYNARA_HOME=/tmp/synara-bisect ./target/debug/index (check if still running first)

## CHROME STRUCTURE FINDING (highest priority structural gap — affects ALL surfaces)
Web upstream (web_tasks.png): the main content is a ROUNDED-CORNER CARD inset in the window, NOT edge-to-edge:
- atmosphere texture shows around the card: behind the left rail (narrow strip x<40) AND in a thin top chrome strip AND around card margins
- card has rounded top-left/top-right corners (visible radius ~12px), starts x~40 y~30ish, inset from window edges
- thin top chrome strip (~28px) spanning full width with sidebar-toggle icon at far left
- segmented "List | Kanban" toggle at TOP RIGHT of the tasks surface (in the strip or card header area)
- heading "Tasks" bold ~x400/y68 inside card; input row "Add a task, or press Tab..." below it
Shell currently: content edge-to-edge rectangular, no card inset, no visible chrome strip, no List/Kanban toggle in tasks view.
=> This is the Kartik #2323 window-chrome model fully realized: persistent atmosphere behind EVERYTHING + opaque rounded content card floating inside. Port this structure: it changes every surface comparison at once. Implement in native.rs root layout: window = atmosphere layer full + rounded content card inset (margins ~8-12px?) + chrome strip row on top.
Measurements needed from upstream: exact card inset margins, corner radius, chrome strip height, whether bottom also inset. Get via CDP getBoundingClientRect on the card container when implementing.

## Surface pass status (01:30)
- tasks: structural gap = chrome card above; content itself minimal (empty state matches: same strings)
- transcript: horizontal metrics match; composer tray fixed; scroll-sync needed for detail pass
- landing: converged earlier
- settings: aligned y174
- inbox: padding fixed; two-column restructure pending
- kanban/pull-requests/automations/hubs: NOT YET compared — after chrome fix, since chrome dominates all

## Chrome exact measurements (CDP, tasks surface, 1600w — right side truncated by panel/measurement quirk but key dims valid)
- Top chrome strip: `h-[44px]` full width, contains sidebar-toggle; consistent with shell's y44 pane start
- Rail width: 51px total
- Content card corner radius: `0 0 13px 13px` — bottom corners rounded 13px (atmosphere peeks at bottom edges/margins)
- Card container chain: `MAIN.relative.flex-1.bg-transparent` → inner `flex flex-col` with the 13px bottom rounding, `overflow-hidden`
- Tasks column: `mx-auto max-w-[36rem]` (576px) `flex-col gap-7` — header h1 `text-2xl font-semibold tracking-tight` (24px), subtitle "Add anything you need to do" muted, then input row
- Tasks input placeholder: "Add a task, or press Tab to hand it to an agent" with dashed-circle checkbox icon at left
- List/Kanban segmented control sits in the TOP STRIP at right (visible at y~15)

## Chrome structure DONE (01:45)
- `caf27fdd1c`+`767ea30bb3`+`40ff76b15e`: atmosphere frame mounted at root always (35%/80%), transparent strip+rail, content opaque, card inset 3px right/bottom + rounded bottom corners 13px, List/Kanban toggle in strip. Tasks view now matches upstream structure. macOS notification was photobombing captures — dismissed; keep it dismissed.
- Landing density VERIFIED correct: upstream landing is dense orange everywhere (web_landing.png) — shell matches geometrically (anaglyph black overlap on temple).

## Kanban gaps (documented, not fixed)
- Upstream strip in kanban: [☑ Kanban | 4 tasks] + 4-way segmented [List|Kanban|Attention|Classic] at LEFT, [+ New task] at RIGHT. Shell: 2-way toggle at right only. Attention/Classic views DO NOT EXIST in shell — feature-level gap beyond pixels.
- Card status divergence: shell shows "Draft", upstream shows blue "Done" badge for the same threads — status derivation differs. Times also differ (32m vs 5h — freshness or derivation).
- Upstream kanban cards: provider glyph under title, time+badge row. Roughly matching already.

## Remaining queue
1. pull-requests surface compare
2. automations compare
3. hubs compare
4. inbox two-column restructure (spec earlier in file)
5. dark mode pass — ALL surfaces
6. transcript detail pass (scroll-sync, attachments layout)
7. landing composer upstream controls (tmp/Select branch/Temporary/Worktree row) — shell has simplified "Work in a project" row; heading should be "What should we do in {project}?" when project selected

## Dark mode backdrop delta (2026-11-27 ~02:10, verified vs upstream dark)
- Shell dark texture visibly brighter/denser/more saturated than upstream dark landing.
- Density constants (DARK_LEVELS) already match upstream exactly — gap is in ink rendering:
  upstream blends per-dot `accentRgb + (highlight-accent)*lit` (luminance varies with local
  density); shell paints uniform accent ink. Upstream dark also has no sky mask (correct in
  shell) but the canvas composite differs.
- Rail strip in dark shows bright white-ish noise in shell vs dim subdued texture upstream —
  check atmosphere frame opacity path in dark (upstream `.dark .app-shell-atmosphere` = 0.8,
  full = 0.6 + brightness(0.75) saturate(0.6) — the filter part is NOT ported).

## CORRECTION 2026-11-27 ~02:30 — spaces/hubs NOT a divergence
Upstream clicking Spaces rail icon navigates INTO the first space (URL /{spaceId}) and shows
the LANDING composer in space context, sidebar label = space name ("Void"). Shell already does
exactly this — the earlier "structural divergence" note was wrong. Hubs surface = STUDIO icon
(upstream label key `common:hubs`), correctly wired to view_studio.
Remaining real gap here: upstream landing heading is project-scoped "What should we do in
{project}?" when a project is selected; shell always "What should we work on?".
