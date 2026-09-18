#!/usr/bin/env python3
"""Local process-duration benchmark with strict, opt-in diagnostic preview. Never sends data."""
from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import platform
import re
import signal
import statistics
import subprocess
import sys
import tempfile
import time
import unittest

REPORT_KEYS = {"schema", "scenario", "os", "arch", "candidate", "samples"}
SAMPLE_KEYS = {"wall_ms", "exit_code", "timed_out"}
OS_NAMES = {"Darwin": "macos", "Windows": "windows", "Linux": "linux"}
ARCH_NAMES = {"arm64": "arm64", "aarch64": "arm64", "x86_64": "x64", "amd64": "x64"}


def validate(report: dict) -> None:
    if not isinstance(report, dict) or set(report) != REPORT_KEYS:
        raise ValueError("Unrecognized report fields")
    if type(report["schema"]) is not int or report["schema"] != 1:
        raise ValueError("Unsupported report schema")
    if report["scenario"] != "process-exit" or report["os"] not in {"macos", "windows", "linux"}:
        raise ValueError("Unsupported measurement scenario or platform")
    if report["arch"] not in {"arm64", "x64", "other"}:
        raise ValueError("Unsupported architecture label")
    if not isinstance(report["candidate"], str) or not re.fullmatch(r"[a-f0-9]{40}", report["candidate"]):
        raise ValueError("Candidate must be the measured Synara source commit")
    samples = report["samples"]
    if not isinstance(samples, list) or not 1 <= len(samples) <= 50:
        raise ValueError("Invalid sample count")
    for row in samples:
        if not isinstance(row, dict) or set(row) != SAMPLE_KEYS:
            raise ValueError("Unrecognized sample fields")
        value = row["wall_ms"]
        if type(value) not in (int, float) or not math.isfinite(value) or not 0 <= value <= 3_600_000:
            raise ValueError("Invalid duration")
        if type(row["timed_out"]) is not bool:
            raise ValueError("Invalid timeout flag")
        code = row["exit_code"]
        if code is not None and (type(code) is not int or not -(2**31) <= code < 2**32):
            raise ValueError("Invalid exit classification")
        if row["timed_out"] != (code is None):
            raise ValueError("Inconsistent timeout classification")


def diagnostic_preview(report: dict, *, opt_in: bool = False) -> dict | None:
    """Fresh allowlisted aggregate, not a redacted copy of arbitrary input.

    No URL, path, timestamp, command, environment, exception, raw output, source
    commit, session ID or device ID can be included. This function has no sender.
    """
    if opt_in is not True:
        return None
    validate(report)
    durations = sorted(row["wall_ms"] for row in report["samples"])
    return {
        "schema": 1,
        "kind": "process_duration_summary",
        "os": report["os"],
        "arch": report["arch"],
        "sample_count": len(durations),
        "median_10ms": int(round(statistics.median(durations) / 10)),
        "p95_10ms": int(round(durations[math.ceil(0.95 * len(durations)) - 1] / 10)),
        "timeout_count": sum(row["timed_out"] for row in report["samples"]),
        "failed_count": sum(row["exit_code"] not in (0, None) for row in report["samples"]),
    }


def stop(process: subprocess.Popen) -> None:
    if os.name == "nt":
        system_root = os.environ.get("SystemRoot", r"C:\Windows")
        taskkill = str(Path(system_root) / "System32" / "taskkill.exe")
        try:
            subprocess.run([taskkill, "/PID", str(process.pid), "/T", "/F"],
                           stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, timeout=10, check=False)
        except (OSError, subprocess.TimeoutExpired):
            process.kill()
    else:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired as exc:
        raise ValueError("Benchmark process teardown did not complete") from exc


def sample(command: list[str], timeout: float) -> dict:
    started = time.perf_counter()
    options = {"start_new_session": True} if os.name != "nt" else {"creationflags": subprocess.CREATE_NEW_PROCESS_GROUP}
    try:
        process = subprocess.Popen(command, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                   stderr=subprocess.DEVNULL, **options)
    except OSError as exc:
        raise ValueError("Benchmark command could not start") from exc
    timed_out = False
    try:
        code = process.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        stop(process)
        code = None
    except BaseException:
        stop(process)
        raise
    return {"wall_ms": round((time.perf_counter() - started) * 1000, 3),
            "exit_code": code, "timed_out": timed_out}


def measure(command: list[str], candidate: str, *, runs: int = 5, warmups: int = 1,
            timeout: float = 30) -> dict:
    if not command or type(runs) is not int or not 1 <= runs <= 50:
        raise ValueError("Command and 1-50 measured runs are required")
    if type(warmups) is not int or not 0 <= warmups <= 10:
        raise ValueError("Use 0-10 warmup runs")
    if not math.isfinite(timeout) or not 0.01 <= timeout <= 300:
        raise ValueError("Use a finite timeout between 0.01 and 300 seconds")
    report = {
        "schema": 1, "scenario": "process-exit",
        "os": OS_NAMES.get(platform.system(), "unsupported"),
        "arch": ARCH_NAMES.get(platform.machine().lower(), "other"),
        "candidate": candidate,
        "samples": [{"wall_ms": 0, "exit_code": 0, "timed_out": False}],
    }
    validate(report)
    for _ in range(warmups):
        row = sample(command, timeout)
        if row["timed_out"] or row["exit_code"] != 0:
            raise ValueError("Warmup failed or timed out")
    report["samples"] = [sample(command, timeout) for _ in range(runs)]
    validate(report)
    return report


class Tests(unittest.TestCase):
    def report(self):
        return {"schema": 1, "scenario": "process-exit", "os": "macos", "arch": "arm64",
                "candidate": "a" * 40,
                "samples": [{"wall_ms": 12.5, "exit_code": 0, "timed_out": False}]}

    def test_diagnostics_require_explicit_opt_in(self):
        self.assertIsNone(diagnostic_preview(self.report()))
        self.assertIsNone(diagnostic_preview(self.report(), opt_in="yes"))

    def test_preview_is_bounded_aggregate_without_identifiers(self):
        result = diagnostic_preview(self.report(), opt_in=True)
        self.assertEqual(result["sample_count"], 1)
        self.assertNotIn("candidate", result)
        self.assertNotIn("samples", result)
        self.assertNotIn("a" * 40, json.dumps(result))

    def test_unknown_fields_are_rejected_instead_of_redacted_optimistically(self):
        for key in ["path", "url", "prompt", "token", "email", "stdout", "device_id"]:
            report = self.report()
            report[key] = "personal-canary"
            with self.assertRaises(ValueError):
                diagnostic_preview(report, opt_in=True)

    def test_nonfinite_and_boolean_timings_are_rejected(self):
        for value in [float("nan"), float("inf"), -1, True, "canary"]:
            report = self.report()
            report["samples"][0]["wall_ms"] = value
            with self.assertRaises(ValueError):
                validate(report)

    def test_raw_output_and_command_are_never_retained(self):
        report = measure([sys.executable, "-c", "print('personal-canary')"], "b" * 40,
                         runs=1, warmups=0)
        self.assertEqual(report["samples"][0]["exit_code"], 0)
        self.assertNotIn("personal-canary", json.dumps(report))
        self.assertNotIn(sys.executable, json.dumps(report))

    def test_failure_and_timeout_are_not_successful_measurements(self):
        failed = measure([sys.executable, "-c", "raise SystemExit(7)"], "b" * 40,
                         runs=1, warmups=0)
        self.assertEqual(failed["samples"][0]["exit_code"], 7)
        timed = measure([sys.executable, "-c", "import time; time.sleep(30)"], "b" * 40,
                        runs=1, warmups=0, timeout=0.05)
        self.assertTrue(timed["samples"][0]["timed_out"])
        self.assertIsNone(timed["samples"][0]["exit_code"])

    def test_invalid_candidate_is_rejected_before_execution(self):
        with self.assertRaises(ValueError):
            measure(["must-not-execute"], "personal project name")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--candidate")
    parser.add_argument("--runs", type=int, default=5)
    parser.add_argument("--warmups", type=int, default=1)
    parser.add_argument("--timeout", type=float, default=30)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--diagnostic-preview", action="store_true")
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
        return 0 if result.wasSuccessful() else 1
    command = args.command[1:] if args.command[:1] == ["--"] else args.command
    try:
        report = measure(command, args.candidate, runs=args.runs, warmups=args.warmups, timeout=args.timeout)
        payload = diagnostic_preview(report, opt_in=True) if args.diagnostic_preview else report
        text = json.dumps(payload, indent=2, allow_nan=False) + "\n"
        if args.output:
            # Do not overwrite a baseline or follow an existing output symlink.
            with args.output.open("x", encoding="utf-8") as output:
                output.write(text)
        else:
            print(text, end="")
    except (ValueError, OSError) as error:
        parser.exit(2, "Measurement failed: " + type(error).__name__ + "\n")
    return 1 if any(row["timed_out"] or row["exit_code"] != 0 for row in report["samples"]) else 0


if __name__ == "__main__":
    raise SystemExit(main())
