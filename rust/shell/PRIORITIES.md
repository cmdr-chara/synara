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
