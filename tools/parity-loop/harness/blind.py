#!/usr/bin/env python3
"""Capture both live windows of an instance pair and write a blind A/B sheet.

  blind.py <inst> <out_dir> [--tag piece/round/state]

Both windows are first set to 1440x900. Output: <out_dir>/{A.png,B.png,side-by-side.png,key-id.txt}.
Which side is upstream is stored only under ~/parity/keys (critics must not read it).
"""
from __future__ import annotations

import argparse
import sys
import tempfile
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import lib  # noqa: E402


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("inst")
    ap.add_argument("out_dir", type=Path)
    ap.add_argument("--tag", default="")
    ap.add_argument("--no-frame", action="store_true")
    a = ap.parse_args()
    state = lib.load_state(a.inst)
    tmp = Path(tempfile.mkdtemp(prefix="parity-raw-"))
    raw = {}
    for side in ("up", "port"):
        w = lib.refresh_window(state, side)
        if not a.no_frame:
            lib.set_frame(state[side]["pid"], w["window_id"])
    time.sleep(1.0)
    for side in ("up", "port"):
        raw[side] = lib.capture(state[side]["window_id"], tmp / f"{side}.png")
    lib.save_state(a.inst, state)
    res = lib.compose_blind(raw["up"], raw["port"], a.out_dir, a.tag or str(a.out_dir))
    for f in raw.values():
        f.unlink()
    print(f"wrote {a.out_dir}/side-by-side.png (A.png, B.png) key={res['key_id']}")


if __name__ == "__main__":
    main()
