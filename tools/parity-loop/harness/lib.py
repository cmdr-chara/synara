"""Shared helpers for the upstream-vs-port parity harness (macOS, cua-driver)."""
from __future__ import annotations

import hashlib
import json
import os
import random
import subprocess
import time
from pathlib import Path

ROOT = Path.home() / "parity"
INST = ROOT / "inst"
KEYS = ROOT / "keys"  # blind keys; critics must never read this directory
WIDTH, HEIGHT = 1440, 900
ORIGIN = (0, 40)
UPSTREAM_APP = "Synara (Dev)"


NO_SESSION = {"list_windows"}


def cua(tool: str, args: dict) -> dict:
    if tool not in NO_SESSION:
        args = {"session": os.environ.get("PARITY_CUA_SESSION", "parity"), **args}
    out = subprocess.run(
        ["cua-driver", "call", tool, json.dumps(args)],
        capture_output=True, text=True, timeout=120,
    )
    try:
        return json.loads(out.stdout)
    except json.JSONDecodeError:
        raise RuntimeError(f"cua-driver {tool} failed: {out.stdout[:400]} {out.stderr[:400]}")


def windows() -> list[dict]:
    return cua("list_windows", {})["windows"]


def main_window(pid: int) -> dict | None:
    """Largest on-screen layer-0 window owned by pid."""
    cands = [w for w in windows() if w["pid"] == pid and w.get("is_on_screen")]
    cands.sort(key=lambda w: w["bounds"]["width"] * w["bounds"]["height"], reverse=True)
    return cands[0] if cands else None


def wait_window(pid: int, timeout: float = 90) -> dict:
    end = time.time() + timeout
    while time.time() < end:
        w = main_window(pid)
        if w and w["bounds"]["width"] > 400:
            return w
        time.sleep(1)
    raise RuntimeError(f"no window for pid {pid}")


def set_frame(pid: int, window_id: int) -> None:
    cua("set_window_frame", {"pid": pid, "window_id": window_id, "x": ORIGIN[0],
                             "y": ORIGIN[1], "width": WIDTH, "height": HEIGHT})


def capture(window_id: int, path: Path) -> Path:
    path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(["screencapture", "-x", "-o", "-l", str(window_id), str(path)],
                   check=True, timeout=60)
    return path


def load_state(inst: str) -> dict:
    return json.loads((INST / inst / "state.json").read_text())


def save_state(inst: str, state: dict) -> None:
    (INST / inst).mkdir(parents=True, exist_ok=True)
    (INST / inst / "state.json").write_text(json.dumps(state, indent=2))


def refresh_window(state: dict, side: str) -> dict:
    w = wait_window(state[side]["pid"])
    state[side]["window_id"] = w["window_id"]
    return w


def compose_blind(up_png: Path, port_png: Path, out_dir: Path, tag: str,
                  order: list[str] | None = None) -> dict:
    """Write A.png, B.png and side-by-side.png with random order. Key goes to KEYS."""
    from PIL import Image, ImageDraw, ImageFont

    out_dir.mkdir(parents=True, exist_ok=True)
    if order is None:
        order = ["upstream", "port"]
        random.shuffle(order)
    src = {"upstream": up_png, "port": port_png}
    imgs = {}
    for label, side in zip("AB", order):
        im = Image.open(src[side]).convert("RGB")
        im.save(out_dir / f"{label}.png")
        imgs[label] = im
    w = max(i.width for i in imgs.values())
    h = max(i.height for i in imgs.values())
    pad, head = 24, 64
    sheet = Image.new("RGB", (w * 2 + pad * 3, h + head + pad), (128, 128, 128))
    draw = ImageDraw.Draw(sheet)
    try:
        font = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 44)
    except OSError:
        font = ImageFont.load_default()
    for i, label in enumerate("AB"):
        x = pad + i * (w + pad)
        draw.text((x, 8), label, fill=(0, 0, 0), font=font)
        sheet.paste(imgs[label], (x, head))
    sheet.save(out_dir / "side-by-side.png")
    key_id = hashlib.sha256(f"{tag}{time.time()}{os.getpid()}".encode()).hexdigest()[:16]
    KEYS.mkdir(parents=True, exist_ok=True)
    (KEYS / f"{key_id}.json").write_text(json.dumps(
        {"tag": tag, "out_dir": str(out_dir), "A": order[0], "B": order[1]}, indent=2))
    (out_dir / "key-id.txt").write_text(key_id + "\n")
    return {"key_id": key_id, "dir": str(out_dir)}
