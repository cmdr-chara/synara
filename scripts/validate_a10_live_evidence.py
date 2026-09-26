#!/usr/bin/env python3
"""Validate reviewed A10 live-platform evidence without inferring accessibility."""
from __future__ import annotations
import argparse
import hashlib
import json
from pathlib import Path

PLATFORMS = {"macos", "windows", "linux"}
REQUIRED = {
    "visual": {"light_theme", "dark_theme", "scaled_layout"},
    "accessibility": {"keyboard_navigation", "names_roles_states", "focus_after_dialog", "screen_reader"},
    "save_picker": {"save_success", "cancel_no_write", "no_overwrite"},
}

def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            h.update(chunk)
    return h.hexdigest()

def validate(doc: dict, root: Path, revision: str) -> None:
    if doc.get("format") != "synara-a10-live-v1":
        raise ValueError("unexpected evidence format")
    if doc.get("revision") != revision:
        raise ValueError("evidence revision does not match reviewed candidate")
    platform = doc.get("platform")
    if platform not in PLATFORMS:
        raise ValueError("unsupported platform")
    for section, keys in REQUIRED.items():
        values = doc.get(section)
        if not isinstance(values, dict):
            raise ValueError(f"missing {section} evidence")
        for key in keys:
            if values.get(key) is not True:
                raise ValueError(f"{section}.{key} was not accepted")
    artifacts = doc.get("artifacts")
    if not isinstance(artifacts, list) or not artifacts:
        raise ValueError("at least one reviewed artifact is required")
    for item in artifacts:
        if not isinstance(item, dict):
            raise ValueError("artifact entries must be objects")
        rel = item.get("file")
        expected = item.get("sha256")
        if not isinstance(rel, str) or not rel or Path(rel).is_absolute() or ".." in Path(rel).parts:
            raise ValueError("artifact path must stay inside the evidence directory")
        path = (root / rel).resolve()
        if root.resolve() not in path.parents:
            raise ValueError("artifact escaped the evidence directory")
        if not path.is_file() or digest(path) != expected:
            raise ValueError(f"artifact digest mismatch: {rel}")

def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()
    if len(args.revision) != 40 or any(ch not in "0123456789abcdef" for ch in args.revision):
        raise SystemExit("revision must be a full lowercase SHA")
    raw = args.evidence.read_bytes()
    if len(raw) > 256 * 1024:
        raise SystemExit("evidence file is too large")
    doc = json.loads(raw)
    validate(doc, args.evidence.parent, args.revision)
    print(f"A10_LIVE_EVIDENCE_ACCEPTED: {doc['platform']} {args.revision}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
