#!/usr/bin/env python3
"""Select additional focused native CI lanes with deterministic rules plus Jev.

The semantic classifier can only add focused verification. Existing branch-level
baseline workflows remain authoritative and continue to run independently.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import time
import urllib.error
import urllib.request
from dataclasses import asdict, dataclass
from typing import Callable

from native_ui_scope import scope_for_paths

API_URL = "https://classifier.dev/v1/classify"
LANE_CONFIDENCE = 0.82
RISK_CONFIDENCE = 0.75
MAX_CONTEXT = 12000
MAX_PATHS = 120
MAX_SUBJECTS = 8

PRESENTATION = "native presentation and general GPUI user interface"
CHAT_TOOLS = "chat utilities, transcript search, or conversation tools"
ENVIRONMENT = "Environment workspace and terminal or panel environment behavior"
NO_FOCUSED_UI = "backend or non-UI change with no focused native UI lane"
BROAD_PRESENTATION = "cross-cutting or unclear native change requiring broad presentation verification"

LOW_RISK = "isolated low-risk change"
SUBSYSTEM_RISK = "subsystem-level regression risk"
HIGH_RISK = "cross-cutting or high regression risk"

SHA_RE = re.compile(r"^[0-9a-f]{40}$")


@dataclass(frozen=True)
class Route:
    presentation: bool
    chat_tools: bool
    environment: bool
    scope: str
    reason: str
    semantic_used: bool = False
    lane_confidence: float | None = None
    risk_confidence: float | None = None
    model: str | None = None


def native_path(path: str) -> bool:
    return (
        path.startswith("crates/synara-app/")
        or path.startswith("scripts/native_")
        or path.startswith("scripts/test_native_")
        or path in {
            ".github/workflows/native.yml",
            ".github/workflows/ui-presentation.yml",
            ".github/workflows/ui-chat-tools.yml",
            ".github/workflows/ui-environment.yml",
        }
    )


def deterministic_route(paths: list[str]) -> Route | None:
    scope = scope_for_paths(paths)
    if scope == "docs":
        return Route(False, False, False, "docs", "deterministic documentation-only change")
    if scope == "presentation":
        return Route(True, False, False, "presentation", "existing deterministic presentation scope")
    if scope == "chat-tools":
        return Route(False, True, False, "chat-tools", "existing deterministic chat-tools scope")
    if scope == "environment":
        return Route(False, False, True, "environment", "existing deterministic environment scope")
    return None


def conservative_fallback(paths: list[str], reason: str) -> Route:
    if any(native_path(path) for path in paths):
        return Route(True, False, False, "presentation", reason)
    return Route(False, False, False, "baseline-only", reason)


def _dimension(dimensions: dict, name: str) -> tuple[str | None, float | None]:
    value = dimensions.get(name)
    if not isinstance(value, dict):
        return None, None
    label = value.get("label")
    confidence = value.get("confidence")
    if not isinstance(label, str):
        label = None
    if not isinstance(confidence, (int, float)):
        confidence = None
    return label, float(confidence) if confidence is not None else None


def semantic_route(paths: list[str], response: dict) -> Route:
    results = response.get("results")
    if not isinstance(results, list) or len(results) != 1 or not isinstance(results[0], dict):
        return conservative_fallback(paths, "classifier response missing a single result")

    dimensions = results[0].get("dimensions")
    if not isinstance(dimensions, dict):
        return conservative_fallback(paths, "classifier response missing dimensions")

    lane, lane_confidence = _dimension(dimensions, "verification_lane")
    risk, risk_confidence = _dimension(dimensions, "regression_risk")
    model = response.get("model") if isinstance(response.get("model"), str) else None

    if lane not in {PRESENTATION, CHAT_TOOLS, ENVIRONMENT, NO_FOCUSED_UI, BROAD_PRESENTATION}:
        fallback = conservative_fallback(paths, "classifier returned an unknown verification lane")
        return Route(**{**asdict(fallback), "semantic_used": True, "model": model})

    if lane_confidence is None or lane_confidence < LANE_CONFIDENCE:
        fallback = conservative_fallback(
            paths,
            f"classifier lane confidence below {LANE_CONFIDENCE:.2f}",
        )
        return Route(
            fallback.presentation,
            fallback.chat_tools,
            fallback.environment,
            fallback.scope,
            fallback.reason,
            True,
            lane_confidence,
            risk_confidence,
            model,
        )

    presentation = lane in {PRESENTATION, BROAD_PRESENTATION}
    chat_tools = lane == CHAT_TOOLS
    environment = lane == ENVIRONMENT
    scope = {
        PRESENTATION: "presentation",
        CHAT_TOOLS: "chat-tools",
        ENVIRONMENT: "environment",
        NO_FOCUSED_UI: "baseline-only",
        BROAD_PRESENTATION: "presentation",
    }[lane]

    if (
        risk == HIGH_RISK
        and risk_confidence is not None
        and risk_confidence >= RISK_CONFIDENCE
        and any(native_path(path) for path in paths)
    ):
        presentation = True
        scope = "presentation+" + scope if scope not in {"presentation", "baseline-only"} else "presentation"

    return Route(
        presentation,
        chat_tools,
        environment,
        scope,
        "classifier.dev Jev semantic route",
        True,
        lane_confidence,
        risk_confidence,
        model,
    )


def valid_revision(value: str) -> bool:
    return bool(SHA_RE.fullmatch(value)) and value != "0" * 40


def git_output(*args: str) -> str:
    return subprocess.check_output(
        ["git", *args],
        stderr=subprocess.DEVNULL,
        timeout=30,
    ).decode("utf-8", errors="replace")


def changed_paths(base: str, head: str) -> list[str] | None:
    if not valid_revision(base) or not valid_revision(head):
        return None
    try:
        raw = subprocess.check_output(
            ["git", "diff", "--name-only", "--no-renames", "-z", base, head, "--"],
            stderr=subprocess.DEVNULL,
            timeout=30,
        )
        return [path for path in raw.decode("utf-8", errors="strict").rstrip("\0").split("\0") if path]
    except (OSError, UnicodeError, subprocess.SubprocessError):
        return None


def build_context(paths: list[str], base: str, head: str) -> str:
    lines = [
        "Repository: cmdr-chara/synara",
        "Branch: astra/gpui-clean-rewrite",
        "Purpose: choose an additional focused native verification lane. Existing mandatory CI runs separately.",
        f"Changed files: {len(paths)}",
        "",
        "Paths:",
    ]
    for path in paths[:MAX_PATHS]:
        lines.append(f"- {path}")
    if len(paths) > MAX_PATHS:
        lines.append(f"- ... {len(paths) - MAX_PATHS} more paths omitted")

    if valid_revision(base) and valid_revision(head):
        try:
            subjects = [
                subject.strip()
                for subject in git_output("log", "--format=%s", f"{base}..{head}", "--").splitlines()
                if subject.strip()
            ][:MAX_SUBJECTS]
        except subprocess.SubprocessError:
            subjects = []
        if subjects:
            lines.extend(["", "Commit subjects:"])
            lines.extend(f"- {subject}" for subject in subjects)

        try:
            stat = git_output("diff", "--stat", "--no-renames", base, head, "--").strip()
        except subprocess.SubprocessError:
            stat = ""
        if stat:
            lines.extend(["", "Diff stat:", stat])

    return "\n".join(lines)[:MAX_CONTEXT]


def call_classifier(
    context: str,
    api_key: str | None,
    *,
    urlopen: Callable = urllib.request.urlopen,
    sleep: Callable[[float], None] = time.sleep,
) -> dict:
    body = {
        "model": "jev",
        "tier": "fast",
        "items": [context],
        "dimensions": {
            "verification_lane": {
                "labels": [
                    PRESENTATION,
                    CHAT_TOOLS,
                    ENVIRONMENT,
                    NO_FOCUSED_UI,
                    BROAD_PRESENTATION,
                ],
                "instructions": (
                    "Choose the narrowest safe additional native CI lane for this Rust/GPUI change. "
                    "Do not assume baseline CI can be skipped."
                ),
            },
            "regression_risk": {
                "labels": [LOW_RISK, SUBSYSTEM_RISK, HIGH_RISK],
                "instructions": (
                    "Judge regression blast radius from the changed paths and commit subjects. "
                    "Use cross-cutting only when multiple native areas or shared contracts can be affected."
                ),
            },
        },
    }
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "User-Agent": "synara-ci-router/1.0",
    }
    if api_key:
        headers["Authorization"] = f"Bearer {api_key}"

    request = urllib.request.Request(
        API_URL,
        data=json.dumps(body).encode("utf-8"),
        headers=headers,
        method="POST",
    )

    for attempt in range(3):
        try:
            with urlopen(request, timeout=10) as response:
                payload = json.load(response)
            if not isinstance(payload, dict):
                raise ValueError("classifier response was not an object")
            return payload
        except urllib.error.HTTPError as exc:
            if exc.code not in {429, 502, 503} or attempt == 2:
                raise
            retry_after = exc.headers.get("Retry-After") if exc.headers else None
            try:
                delay = min(max(float(retry_after), 0.0), 5.0) if retry_after else (attempt + 1)
            except ValueError:
                delay = attempt + 1
            sleep(delay)
        except (urllib.error.URLError, TimeoutError):
            if attempt == 2:
                raise
            sleep(attempt + 1)
    raise RuntimeError("classifier retry loop exhausted")


def select_route(
    paths: list[str],
    *,
    context: str,
    api_key: str | None,
    classifier: Callable[[str, str | None], dict] = call_classifier,
) -> Route:
    deterministic = deterministic_route(paths)
    if deterministic is not None:
        return deterministic

    try:
        response = classifier(context, api_key)
    except Exception as exc:
        return conservative_fallback(paths, f"classifier unavailable ({type(exc).__name__})")
    return semantic_route(paths, response)


def write_github_outputs(route: Route, path: str) -> None:
    values = {
        "presentation": route.presentation,
        "chat_tools": route.chat_tools,
        "environment": route.environment,
        "scope": route.scope,
        "semantic_used": route.semantic_used,
        "lane_confidence": "" if route.lane_confidence is None else f"{route.lane_confidence:.4f}",
        "risk_confidence": "" if route.risk_confidence is None else f"{route.risk_confidence:.4f}",
        "model": route.model or "",
    }
    with open(path, "a", encoding="utf-8") as output:
        for key, value in values.items():
            if isinstance(value, bool):
                value = str(value).lower()
            output.write(f"{key}={value}\n")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", default=os.environ.get("BASE_REVISION", ""))
    parser.add_argument("--head", default=os.environ.get("CANDIDATE_REVISION", ""))
    parser.add_argument("--github-output", default=os.environ.get("GITHUB_OUTPUT"))
    args = parser.parse_args()

    paths = changed_paths(args.base, args.head)
    if paths is None:
        route = Route(
            True,
            False,
            False,
            "presentation",
            "comparison history unavailable; fail closed to broad native presentation verification",
        )
        paths = []
    else:
        context = build_context(paths, args.base, args.head)
        route = select_route(
            paths,
            context=context,
            api_key=os.environ.get("CLASSIFIER_API_KEY") or None,
        )

    summary = {
        "route": asdict(route),
        "changed_paths": paths,
    }
    print(json.dumps(summary, indent=2, sort_keys=True))
    if args.github_output:
        write_github_outputs(route, args.github_output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
