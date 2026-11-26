# Pixel-perfect comparison harness

Overlay-diff method: capture the Electron/web reference and the native shell
at the same window size, overlay them — identical pixels read grey, differences
fringe red (reference) or cyan (shell).

- `capture.sh` — positions the shell window at (0,30) 1600x1088, captures it,
  then drives the web app via CDP (port 57012) to the same viewport and captures it
- `anaglyph.py ref test out` — red/cyan overlay diff
- `reddiff.py ref test out [thresh]` — red-highlight diff with per-band percentages

Prereqs: shell running against the test backend, Chrome with
`--remote-debugging-port=57012` on the web app, python3 + numpy + websockets + PIL.

Captured pairs land in /tmp/pxcmp/. Compare surface by surface; fix the largest
fringe first. Do not eyeball — measure.
