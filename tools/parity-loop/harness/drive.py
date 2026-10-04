"""Tiny cua-driver wrapper used by scenario files.

    from drive import App
    up = App.from_state(inst, "up"); port = App.from_state(inst, "port")
    up.click("Settings"); port.click("Settings", role="AXButton")
    up.key("escape"); up.hotkey("cmd", "k"); up.type("hello"); up.click_xy(400, 300)
    up.wait_for("Appearance")
"""
from __future__ import annotations

import base64
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import lib  # noqa: E402


class NotFound(RuntimeError):
    pass


class App:
    def __init__(self, pid: int, window_id: int, side: str):
        self.pid, self.window_id, self.side = pid, window_id, side
        self._els: list[dict] = []
        # GPUI ignores background-posted mouse/key events; front the window briefly for those.
        self.foreground = side == "port"

    def _mode(self, args: dict) -> dict:
        if self.foreground:
            args["delivery_mode"] = "foreground"
            args.setdefault("window_id", self.window_id)
        return args

    @classmethod
    def from_state(cls, inst: str, side: str) -> "App":
        st = lib.load_state(inst)
        w = lib.refresh_window(st, side)
        lib.save_state(inst, st)
        return cls(st[side]["pid"], w["window_id"], side)

    def snap(self, min_elements: int = 6) -> list[dict]:
        for _ in range(4):
            d = lib.cua("get_window_state", {"pid": self.pid, "window_id": self.window_id})
            self._els = d.get("elements", [])
            self._last = d
            if len(self._els) >= min_elements:
                break
            time.sleep(0.8)
        return self._els

    def screenshot(self, path: Path) -> Path:
        """Window-local PNG in the same pixel space click_xy uses."""
        self.snap()
        path.write_bytes(base64.b64decode(self._last["screenshot_png_b64"]))
        return path

    def find(self, label: str, role: str | None = None, nth: int = 0, exact: bool = False,
             refresh: bool = True) -> dict:
        els = self.snap() if refresh or not self._els else self._els
        def ok(e: dict) -> bool:
            text = (e.get("label") or "") + " " + (e.get("value") or "") if not exact else (e.get("label") or "")
            hit = (text.strip() == label) if exact else (label.lower() in text.lower())
            return hit and (role is None or e.get("role") == role)
        hits = [e for e in els if ok(e)]
        if len(hits) <= nth:
            raise NotFound(f"[{self.side}] no element {label!r} role={role} (have {len(els)})")
        return hits[nth]

    def exists(self, label: str, role: str | None = None) -> bool:
        try:
            self.find(label, role)
            return True
        except NotFound:
            return False

    def wait_for(self, label: str, role: str | None = None, timeout: float = 20) -> dict:
        end = time.time() + timeout
        while True:
            try:
                return self.find(label, role)
            except NotFound:
                if time.time() > end:
                    raise
                time.sleep(0.7)

    def click(self, label: str, role: str | None = None, nth: int = 0, exact: bool = False,
              settle: float = 0.8, action: str | None = None, pixel: bool = False) -> None:
        """AX press by default. pixel=True clicks the element's frame center instead
        (needed for GPUI elements without an AX press action; steals focus)."""
        e = self.find(label, role, nth, exact)
        if pixel and e.get("frame"):
            f, wb = e["frame"], self._last["window_bounds"]
            scale = self._last["screenshot_width"] / wb["width"]
            self.click_xy((f["x"] - wb["x"] + f["w"] / 2) * scale,
                          (f["y"] - wb["y"] + f["h"] / 2) * scale, settle=settle)
            return
        args = {"pid": self.pid, "element_token": e["element_token"]}
        if action:
            args["action"] = action
        lib.cua("click", args)
        time.sleep(settle)

    def click_xy(self, x: float, y: float, settle: float = 0.8, button: str = "left",
                 count: int = 1) -> None:
        """x, y in window-local screenshot pixels of the latest snap()/screenshot()."""
        if not getattr(self, "_last", None):
            self.snap()
        args = {"pid": self.pid, "window_id": self.window_id, "x": x, "y": y,
                "button": button, "count": count, "capture_id": self._last["capture_id"]}
        r = lib.cua("click", self._mode(args))
        if r.get("code"):
            raise RuntimeError(f"[{self.side}] click_xy failed: {r}")
        time.sleep(settle)

    def key(self, key: str, *mods: str, settle: float = 0.6) -> None:
        lib.cua("press_key", self._mode({"pid": self.pid, "window_id": self.window_id, "key": key,
                                         "modifiers": list(mods)}))
        time.sleep(settle)

    def hotkey(self, *keys: str, settle: float = 0.6) -> None:
        lib.cua("hotkey", self._mode({"pid": self.pid, "window_id": self.window_id, "keys": list(keys)}))
        time.sleep(settle)

    def type(self, text: str, settle: float = 0.6) -> None:
        lib.cua("type_text", self._mode({"pid": self.pid, "window_id": self.window_id, "text": text}))
        time.sleep(settle)

    def scroll(self, direction: str, amount: int = 5, x: float | None = None,
               y: float | None = None) -> None:
        args = {"pid": self.pid, "window_id": self.window_id, "direction": direction, "amount": amount}
        if x is not None:
            args.update(x=x, y=y)
        lib.cua("scroll", self._mode(args))
        time.sleep(0.6)

    def labels(self) -> list[str]:
        return [f"{e.get('role')}:{e.get('label')}" for e in self.snap() if e.get("label")]


def dump(app: App) -> None:
    print(json.dumps(app.labels(), indent=1))
