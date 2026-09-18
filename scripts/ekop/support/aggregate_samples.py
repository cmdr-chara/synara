#!/usr/bin/env python3
"""Validate and aggregate local benchmark samples. No network or automatic telemetry.

Only enumerated metadata and bounded numeric samples enter the output. Paths,
commands, host names, logs and environment values are never copied into reports.
"""
from __future__ import annotations
import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics

CASES = {"browser-policy", "registry-parse", "startup", "composer", "transcript", "terminal", "file-diff"}
MAX_BYTES = 64 * 1024


def object_without_duplicates(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate report field")
        result[key] = value
    return result


def summarize(raw: dict, candidate: str, system: str | None = None, machine: str | None = None) -> dict:
    if not isinstance(raw, dict) or set(raw) != {"case", "build", "synthetic", "operations_per_sample", "samples_ms"}:
        raise ValueError("unrecognized report schema")
    if not isinstance(candidate, str) or not re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", candidate):
        raise ValueError("invalid candidate identity")
    if not isinstance(raw["case"], str) or raw["case"] not in CASES:
        raise ValueError("unrecognized benchmark case")
    if not isinstance(raw["build"], str) or raw["build"] not in {"debug", "release"}:
        raise ValueError("unrecognized build type")
    if type(raw["synthetic"]) is not bool:
        raise ValueError("missing synthetic workload classification")
    if type(raw["operations_per_sample"]) is not int or not 1 <= raw["operations_per_sample"] <= 10**9:
        raise ValueError("invalid operation count")
    samples = raw["samples_ms"]
    if not isinstance(samples, list) or not 3 <= len(samples) <= 1000:
        raise ValueError("expected 3 to 1000 samples")
    if any(type(x) not in (int, float) or not 0 <= x <= 3_600_000 or not math.isfinite(x) for x in samples):
        raise ValueError("invalid timing sample")
    ordered = sorted(samples)
    os_name = {"Darwin": "macos", "Windows": "windows", "Linux": "linux"}.get(system or platform.system(), "other")
    arch = {"arm64": "arm64", "aarch64": "arm64", "AMD64": "x64", "x86_64": "x64"}.get(machine or platform.machine(), "other")
    return {
        "format": 1, "candidate": candidate, "platform": os_name, "architecture": arch,
        "case": raw["case"], "build": raw["build"], "synthetic": raw["synthetic"],
        "operations_per_sample": raw["operations_per_sample"], "sample_count": len(samples),
        "unit": "milliseconds", "minimum": min(samples), "median": statistics.median(samples),
        "p95_nearest_rank": ordered[math.ceil(len(samples) * .95) - 1], "maximum": max(samples),
        "samples": list(samples),
    }


def load(path: Path) -> dict:
    with path.open("rb") as stream:
        data = stream.read(MAX_BYTES + 1)
    if len(data) > MAX_BYTES:
        raise ValueError("report input exceeds size limit")
    return json.loads(data, object_pairs_hook=object_without_duplicates)


def write_new(path: Path, report: dict) -> None:
    # O_EXCL also refuses an existing symlink. Never overwrite previous measurements.
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8") as stream:
        json.dump(report, stream, indent=2, allow_nan=False)
        stream.write("\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--candidate", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        report = summarize(load(args.input), args.candidate)
        write_new(args.output, report)
    except (ValueError, TypeError, OverflowError, RecursionError, OSError):
        print("Measurement rejected or output unavailable. No report was sent.")
        return 2
    print("Local measurement report created. Nothing was sent over the network.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
