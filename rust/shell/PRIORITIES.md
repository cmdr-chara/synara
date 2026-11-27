# Shell priorities

## RESOLVED 2026-11-27 night — Temple backdrop fidelity

The night sweep closed the structural divergences (see commit log on
feat/rust-port): landing canvas confined to the content-pane geometry
(x308/y44, matching upstream inset-0), blur compensated to 0.6 for the
browser's high-quality smoothing, paper wash weakened toward the oklab
color-mix strength, click ripples ported with live re-dithering, and the
dark ink corrected to full accent strength on the landing canvas
(upstream dims only the full-window atmosphere layer in dark mode —
measured lit-dot rgb (239,107,60) vs upstream (237,105,57), temple
density 0.179 vs 0.198 in dark default mode). Measured end state vs the Electron-served reference
at 1600x1088: temple coverage 24.1% vs 24.9%, sky 1.4% vs 1.3%, scene
position exact (dx=0 dy=0 by cross-correlation), sky paper tint r-g 3.0
vs 4.7.

## HIGH — remaining backdrop divergences

- Per-dot pattern detail differs (IoU ~0.54 in transition bands) though
  density/position/scale all match: JPEG decode and browser upscale
  kernel differences. Perceptually invisible at normal scale; closing it
  needs exact replication of the browser's high-quality drawImage kernel.
- The gradient backdrop style is not ported (temple only).
- Hills bottom band may still read slightly denser; verify after the
  blur compensation at other window sizes.

## MEDIUM — empty-state vertical rhythm

Automations/code-review empty states drift 3-5px per element vs the
reference: gpui line-box metrics differ from Chromium's. Any fix is a
fragile per-element compensation; revisit only with a font-metrics-driven
approach.

## LOW — unverified surfaces

- Settings: web reference unreachable in the signed-out test home (no
  settings entry point); shell implementation verified structurally
  complete section-by-section instead.
- Thread transcript content fidelity blocked on the backend projection
  bug (web stuck on "Loading conversation"; reported).
