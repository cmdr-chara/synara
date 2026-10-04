#!/usr/bin/env python3
"""Seed an instance pair, drive both apps through a piece's states, capture each state.

  run_scenario.py <piece> --bin <port-binary> --out <dir> [--blind] [--inst NAME] [--keep] [--only STATE]

Scenario file: ~/parity/pieces/<piece>/scenario.py defining
    STATES = [("state-name", up_fn, port_fn), ...]
where up_fn(up: drive.App) / port_fn(port: drive.App) bring each app from the previous state (or from
fresh seeded launch for the first state, or when the state's dict has reset=True) to the named state.
Optional: RESET_BEFORE = {"state-name", ...} relaunches both freshly seeded before that state.

--blind: per state write <out>/<state>/{A.png,B.png,side-by-side.png} with random sides (critic mode).
default: per state write <out>/<state>/{upstream.png,port.png} (builder mode).
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import subprocess
import sys
import tempfile
import time
import traceback
from pathlib import Path

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
import drive  # noqa: E402
import lib  # noqa: E402


def sh(*args: str) -> None:
    subprocess.run([sys.executable, *args], check=True)


def fresh(inst: str, binary: Path) -> tuple[drive.App, drive.App]:
    sh(str(HERE / "launch.py"), "stop", inst)
    if (HERE / "seed.py").exists():
        subprocess.run([sys.executable, str(HERE / "seed.py"), inst], check=True,
                       env={**__import__("os").environ, "PARITY_PORT_BIN": str(binary)})
    sh(str(HERE / "launch.py"), "up", inst)
    sh(str(HERE / "launch.py"), "port", inst, "--bin", str(binary))
    sh(str(HERE / "launch.py"), "frame", inst)
    time.sleep(6)  # let both finish first paint / splash
    return drive.App.from_state(inst, "up"), drive.App.from_state(inst, "port")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("piece")
    ap.add_argument("--bin", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--inst", default=None)
    ap.add_argument("--blind", action="store_true")
    ap.add_argument("--keep", action="store_true", help="leave both apps running at the end")
    ap.add_argument("--only", default=None)
    a = ap.parse_args()
    inst = a.inst or f"{a.piece}-{'crit' if a.blind else 'build'}"
    path = lib.ROOT / "pieces" / a.piece / "scenario.py"
    spec = importlib.util.spec_from_file_location("scenario", path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    resets = set(getattr(mod, "RESET_BEFORE", set()))
    a.out.mkdir(parents=True, exist_ok=True)
    results = []
    order = ["upstream", "port"]
    __import__("random").shuffle(order)  # one blind order for the whole round
    up, port = fresh(inst, a.bin)
    first = True
    for name, up_fn, port_fn in mod.STATES:
        if a.only and name != a.only:
            continue
        if name in resets and not first:
            up, port = fresh(inst, a.bin)
        first = False
        errs = []
        for app, fn in ((up, up_fn), (port, port_fn)):
            try:
                app.window_id = lib.main_window(app.pid)["window_id"]
                fn(app)
            except Exception as exc:  # noqa: BLE001 — record and keep capturing
                errs.append(f"{app.side}: {exc!r}")
                traceback.print_exc()
        time.sleep(1.2)
        d = a.out / name
        if a.blind:
            tmp = Path(tempfile.mkdtemp(prefix="parity-raw-"))
            lib.capture(lib.main_window(up.pid)["window_id"], tmp / "up.png")
            lib.capture(lib.main_window(port.pid)["window_id"], tmp / "port.png")
            lib.compose_blind(tmp / "up.png", tmp / "port.png", d, f"{a.piece}/{name}", order)
        else:
            lib.capture(lib.main_window(up.pid)["window_id"], d / "upstream.png")
            lib.capture(lib.main_window(port.pid)["window_id"], d / "port.png")
        # in blind mode, errors are reported without saying which side failed
        results.append({"state": name, "driving_errors": len(errs) if a.blind else errs})
        print(f"captured {name}: {d}")
    (a.out / "states.json").write_text(json.dumps(results, indent=2))
    if not a.keep:
        sh(str(HERE / "launch.py"), "stop", inst)


if __name__ == "__main__":
    main()
