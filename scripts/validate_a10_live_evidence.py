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
    "accessibility": {
        "keyboard_navigation",
        "names_roles_states",
        "focus_after_dialog",
        "screen_reader",
    },
    "save_picker": {"save_success", "cancel_no_write", "no_overwrite"},
}
ARTIFACT_KINDS = {
    "light-screenshot",
    "dark-screenshot",
    "accessibility-log",
    "save-picker-log",
}


def digest(path: Path) -> str:
    value = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            value.update(chunk)
    return value.hexdigest()


def bounded_text(value: object, name: str, limit: int = 256) -> str:
    if not isinstance(value, str) or not value.strip() or len(value) > limit:
        raise ValueError(f"{name} must be non-empty bounded text")
    if any(ord(ch) < 32 and ch != "\t" for ch in value):
        raise ValueError(f"{name} contains control characters")
    return value.strip()


def validate(doc: dict, root: Path, revision: str, expected_platform: str) -> dict:
    if doc.get("format") != "synara-a10-live-v1":
        raise ValueError("unexpected evidence format")
    if doc.get("revision") != revision:
        raise ValueError("evidence revision does not match reviewed candidate")
    platform = doc.get("platform")
    if platform not in PLATFORMS or platform != expected_platform:
        raise ValueError("evidence platform does not match the selected runner")

    environment = doc.get("environment")
    if not isinstance(environment, dict):
        raise ValueError("missing environment metadata")
    os_version = bounded_text(environment.get("os_version"), "environment.os_version")
    screen_reader = bounded_text(environment.get("screen_reader"), "environment.screen_reader")
    accessibility_api = bounded_text(
        environment.get("accessibility_api"), "environment.accessibility_api"
    )
    scale_factor = environment.get("scale_factor")
    if not isinstance(scale_factor, (int, float)) or not 0.5 <= scale_factor <= 4.0:
        raise ValueError("environment.scale_factor must be between 0.5 and 4.0")

    for section, keys in REQUIRED.items():
        values = doc.get(section)
        if not isinstance(values, dict):
            raise ValueError(f"missing {section} evidence")
        for key in keys:
            if values.get(key) is not True:
                raise ValueError(f"{section}.{key} was not accepted")

    artifacts = doc.get("artifacts")
    if not isinstance(artifacts, list) or len(artifacts) < len(ARTIFACT_KINDS):
        raise ValueError("reviewed visual/accessibility/save-picker artifacts are required")
    observed_kinds = set()
    summary_artifacts = []
    root = root.resolve()
    for item in artifacts:
        if not isinstance(item, dict):
            raise ValueError("artifact entries must be objects")
        kind = item.get("kind")
        if kind not in ARTIFACT_KINDS or kind in observed_kinds:
            raise ValueError("artifact kind is missing, unsupported or duplicated")
        observed_kinds.add(kind)
        rel = item.get("file")
        expected = item.get("sha256")
        if (
            not isinstance(rel, str)
            or not rel
            or Path(rel).is_absolute()
            or ".." in Path(rel).parts
        ):
            raise ValueError("artifact path must stay inside the evidence directory")
        path = (root / rel).resolve()
        if root not in path.parents:
            raise ValueError("artifact escaped the evidence directory")
        if not path.is_file():
            raise ValueError(f"artifact is missing: {rel}")
        actual = digest(path)
        if expected != actual:
            raise ValueError(f"artifact digest mismatch: {rel}")
        summary_artifacts.append({"kind": kind, "sha256": actual})
    if observed_kinds != ARTIFACT_KINDS:
        raise ValueError("required artifact kinds are incomplete")

    return {
        "format": "synara-a10-summary-v1",
        "revision": revision,
        "platform": platform,
        "environment": {
            "os_version": os_version,
            "screen_reader": screen_reader,
            "accessibility_api": accessibility_api,
            "scale_factor": scale_factor,
        },
        "checks": {section: sorted(keys) for section, keys in REQUIRED.items()},
        "artifacts": sorted(summary_artifacts, key=lambda item: item["kind"]),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--platform", choices=sorted(PLATFORMS), required=True)
    parser.add_argument("--summary-out", type=Path, required=True)
    args = parser.parse_args()
    if len(args.revision) != 40 or any(
        ch not in "0123456789abcdef" for ch in args.revision
    ):
        raise SystemExit("revision must be a full lowercase SHA")
    raw = args.evidence.read_bytes()
    if len(raw) > 256 * 1024:
        raise SystemExit("evidence file is too large")
    document = json.loads(raw)
    summary = validate(document, args.evidence.parent, args.revision, args.platform)
    args.summary_out.parent.mkdir(parents=True, exist_ok=True)
    with args.summary_out.open("xb") as output:
        output.write((json.dumps(summary, indent=2, sort_keys=True) + "\n").encode())
    print(f"A10_LIVE_EVIDENCE_ACCEPTED: {args.platform} {args.revision}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
