# Parity loop tooling (work in progress)

Tooling for the upstream-vs-port visual parity loop. Written for a macOS host with `cua-driver`;
paths assume it is copied to `~/parity` (see `CONTEXT.md`).

- `harness/`: launch an isolated upstream Electron build and the port side by side (own home,
  userData and data dir, never `~/.synara`), seed both with `fixture.json`, drive them with
  `cua-driver`, and capture blind A/B sheets (`run_scenario.py --blind`, `unseal.py`).
  `seed.py` is unverified: its acceptance run never finished.
- `checklist/`: `extract.py` builds `items.json` (routes, components, settings, keybindings) from
  upstream main `5f2ee77ae`; `map_status.py` wrote the first-pass `status.json`
  (52 done, 286 partial, 807 missing of 1145). `score.py` was not written yet.
- `pieces/*/PIECE.md`: 28 judgeable pieces with scope, upstream sources and required states.
- `briefs/`: builder and blind-critic briefs used per round.

Known issue: port-side clicks use foreground delivery (GPUI ignores background clicks), so a
run brings the port window to the front. Run it on a dedicated display or VM.
