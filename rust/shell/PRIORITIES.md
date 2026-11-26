# Shell priorities

## HIGH — Temple backdrop fidelity vs upstream web app

The native dithered temple (`src/backdrop.rs`) still does not match the web
client's `EmptyChatBackdrop` closely enough (user review 2026-11-26: "è
totalmente diverso"). Accent color (#c74614 light / #f2612d dark), binary
1px-per-cell rendering and Bayer thresholds are now correct, and dense-area
ink measures identical (#d77a56) — but the overall composition still reads
differently. Remaining known divergences to verify and close:

- Scene framing/scale within the pane (cover math, bottom anchoring) — the
  temple sits and sizes differently than in the browser at the same window size.
- Grain feel: browsers upscale the tiny canvas with bilinear smoothing at
  device-pixel ratio; check what gpui's image scaling actually does at 2x
  retina (filter quality, premultiplied alpha interpolation) and match it.
- Sky-mask, fade, thinning, ground-dim and quiet-zone interplay: port the
  exact upstream layering order (canvas mask composited over dots, paper
  wash under dots, gradient haze) instead of approximating per-term.
- Dark mode never visually verified against upstream; also the full-window
  dimming (opacity 0.6, brightness 0.75, saturate 0.6) is not ported.
- Ripples (click feedback) and the animated gradient style are not ported.

Method that works: side-by-side screenshots of web app and native shell at
the same window size, then pixel-sample ink/paper/density in matched regions
(see session notes). Do not eyeball.
