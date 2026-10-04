#!/usr/bin/env python3
"""Start/stop an isolated upstream + port instance pair.

  launch.py up   <inst>                 start upstream Electron (own SYNARA_HOME + userData)
  launch.py port <inst> [--bin PATH]    start the GPUI port (own --data-dir)
  launch.py stop <inst> [up|port|all]
  launch.py frame <inst>                set both windows to 1440x900 at (0,40)

Data lives under ~/parity/inst/<inst>/{up-home,up-userdata,port-data}. Never ~/.synara.
"""
from __future__ import annotations

import argparse
import os
import signal
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import lib  # noqa: E402

UPSTREAM_DESKTOP = lib.ROOT / "upstream" / "apps" / "desktop"
DEFAULT_PORT_BIN = Path.home() / "Projects" / "synara-gpui-pr11" / "target" / "debug" / "synara-app"


def upstream_pids() -> set[int]:
    out = subprocess.run(["pgrep", "-f", "MacOS/Electron .*dist-electron/main.js"],
                         capture_output=True, text=True).stdout
    return {int(p) for p in out.split()}


def start_up(inst: str) -> None:
    d = lib.INST / inst
    home, ud = d / "up-home", d / "up-userdata"
    home.mkdir(parents=True, exist_ok=True)
    ud.mkdir(parents=True, exist_ok=True)
    for bad in (Path.home() / ".synara", Path.home() / ".synara-canary"):
        assert not str(home).startswith(str(bad)), "refusing to use production data"
    env = dict(os.environ, SYNARA_HOME=str(home), SYNARA_DESKTOP_SMOKE_USER_DATA=str(ud),
               SDKROOT="/Library/Developer/CommandLineTools/SDKs/MacOSX26.5.sdk")
    before = upstream_pids()
    log = open(d / "up.log", "w")
    subprocess.Popen(["bun", "run", "start"], cwd=UPSTREAM_DESKTOP, env=env, stdout=log,
                     stderr=subprocess.STDOUT, start_new_session=True)
    end = time.time() + 90
    pid = None
    while time.time() < end and pid is None:
        new = upstream_pids() - before
        pid = max(new) if new else None
        time.sleep(1)
    if pid is None:
        sys.exit("upstream did not start; see " + str(d / "up.log"))
    w = lib.wait_window(pid)
    state = _state(inst)
    state["up"] = {"pid": pid, "window_id": w["window_id"], "home": str(home), "userdata": str(ud)}
    lib.save_state(inst, state)
    print(f"upstream pid={pid} window={w['window_id']}")


def start_port(inst: str, binary: Path) -> None:
    d = lib.INST / inst
    data = d / "port-data"
    data.mkdir(parents=True, exist_ok=True)
    log = open(d / "port.log", "w")
    p = subprocess.Popen([str(binary), "--data-dir", str(data)], stdout=log,
                         stderr=subprocess.STDOUT, start_new_session=True)
    w = lib.wait_window(p.pid)
    state = _state(inst)
    state["port"] = {"pid": p.pid, "window_id": w["window_id"], "data": str(data), "bin": str(binary)}
    lib.save_state(inst, state)
    print(f"port pid={p.pid} window={w['window_id']}")


def _state(inst: str) -> dict:
    try:
        return lib.load_state(inst)
    except FileNotFoundError:
        return {}


def stop(inst: str, which: str) -> None:
    state = _state(inst)
    for side in (["up", "port"] if which == "all" else [which]):
        info = state.pop(side, None)
        if not info:
            continue
        try:
            os.kill(info["pid"], signal.SIGTERM)
        except ProcessLookupError:
            pass
        for _ in range(20):
            try:
                os.kill(info["pid"], 0)
                time.sleep(0.5)
            except ProcessLookupError:
                break
        else:
            os.kill(info["pid"], signal.SIGKILL)
        print(f"stopped {side} pid={info['pid']}")
    lib.save_state(inst, state)


def frame(inst: str) -> None:
    state = _state(inst)
    for side in ("up", "port"):
        if side in state:
            w = lib.refresh_window(state, side)
            lib.set_frame(state[side]["pid"], w["window_id"])
    lib.save_state(inst, state)


if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("cmd", choices=["up", "port", "stop", "frame"])
    ap.add_argument("inst")
    ap.add_argument("which", nargs="?", default="all")
    ap.add_argument("--bin", type=Path, default=DEFAULT_PORT_BIN)
    a = ap.parse_args()
    {"up": lambda: start_up(a.inst), "port": lambda: start_port(a.inst, a.bin),
     "stop": lambda: stop(a.inst, a.which), "frame": lambda: frame(a.inst)}[a.cmd]()
