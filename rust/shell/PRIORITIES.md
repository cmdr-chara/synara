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
