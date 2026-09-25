#!/usr/bin/env python3
"""Route Synara GPUI CI with deterministic safeguards plus Jev decisions.

Cheap/static checks remain independent. This router controls expensive backend,
native organization/context, SSH, and focused native UI verification.
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

from native_ui_scope import documentation, scope_for_paths

API_URL = "https://classifier.dev/v1/classify"
LANE_CONFIDENCE = 0.82
RISK_CONFIDENCE = 0.75
EXPENSIVE_CONFIDENCE = 0.82
MAX_CONTEXT = 12000
MAX_PATHS = 120
MAX_SUBJECTS = 8

PRESENTATION = "native presentation and general GPUI user interface"
CHAT_TOOLS = "chat utilities, transcript search, or conversation tools"
ENVIRONMENT = "Environment workspace and terminal or panel environment behavior"
NO_FOCUSED_UI = "no focused native UI lane"
BROAD_PRESENTATION = "cross-cutting or unclear native change requiring broad presentation verification"

LOW_RISK = "isolated low-risk change"
SUBSYSTEM_RISK = "subsystem-level regression risk"
HIGH_RISK = "cross-cutting or high regression risk"

BACKEND_SKIP = "skip backend acceptance"
BACKEND_LINUX = "run Linux backend acceptance only"
BACKEND_FULL = "run full Linux macOS Windows backend acceptance"

NATIVE_SKIP = "skip organization and saved-context native batch"
NATIVE_RUN = "run organization and saved-context native batch"

SSH_SKIP = "skip SSH transport verification"
SSH_RUN = "run SSH transport verification"

SHA_RE = re.compile(r"^[0-9a-f]{40}$")

HARD_FULL_BACKEND = frozenset({
    "Cargo.toml",
    "Cargo.lock",
    "rust-toolchain.toml",
    ".github/workflows/backend.yml",
})
NATIVE_BATCH_ANCHORS = frozenset({
    "crates/synara-app/src/shell/organization.rs",
    "crates/synara-app/src/shell/saved_context.rs",
    "crates/synara-workspace/src/storage/organization.rs",
    "crates/synara-workspace/src/storage/task_context.rs",
    "scripts/assemble_saved_context.py",
    "scripts/native_organization_context_smoke.py",
    ".github/workflows/native.yml",
})
SSH_ANCHORS = frozenset({
    "crates/synara-app/src/shell/terminal.rs",
    "crates/synara-app/src/shell/terminals.rs",
    "crates/synara-runtime/src/remote_terminal.rs",
    "crates/synara-runtime/src/terminal.rs",
    "crates/synara-runtime/src/terminal/posix.rs",
    "scripts/ci_router.py",
    "scripts/ssh_smoke.py",
    "scripts/remote_native_smoke.py",
    ".github/workflows/ssh.yml",
})


@dataclass(frozen=True)
class Route:
    presentation: bool
    chat_tools: bool
    environment: bool
    backend_mode: str
    native_batch: bool
    ssh: bool
    scope: str
    reason: str
    semantic_used: bool = False
    lane_label: str | None = None
    lane_confidence: float | None = None
    risk_label: str | None = None
    risk_confidence: float | None = None
    backend_label: str | None = None
    backend_confidence: float | None = None
    native_label: str | None = None
    native_confidence: float | None = None
    ssh_label: str | None = None
    ssh_confidence: float | None = None
    model: str | None = None


def docs_only(paths: list[str]) -> bool:
    return bool(paths) and all(
        documentation(path)
        or path.startswith("docs/")
        or path.endswith(".md")
        for path in paths
    )


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


def backend_relevant(path: str) -> bool:
    return (
        path.startswith("crates/")
        or path.startswith("scripts/")
        or path in HARD_FULL_BACKEND
        or path.startswith(".github/workflows/backend")
    )


def remote_sensitive(path: str) -> bool:
    return (
        path in SSH_ANCHORS
        or "ssh" in path.lower()
        or path.startswith("crates/synara-runtime/")
        or path.startswith("crates/synara-acp/")
        or path.startswith("crates/synara-workspace/")
    )


def native_batch_anchor(path: str) -> bool:
    return (
        path in NATIVE_BATCH_ANCHORS
        or path.startswith("crates/synara-app/src/shell/organization/")
    )


def hard_backend_mode(paths: list[str]) -> str | None:
    if any(path in HARD_FULL_BACKEND for path in paths):
        return "full"
    return None


def fallback_backend_mode(paths: list[str], focused_scope: str) -> str:
    if docs_only(paths):
        return "skip"
    if hard_backend_mode(paths):
        return "full"
    if focused_scope in {"presentation", "chat-tools", "environment"}:
        return "skip"
    if any(backend_relevant(path) for path in paths):
        return "full"
    return "skip"


def fallback_native_batch(paths: list[str]) -> bool:
    return any(native_batch_anchor(path) for path in paths)


def fallback_ssh(paths: list[str], focused_scope: str) -> bool:
    if any(path in SSH_ANCHORS for path in paths):
        return True
    if focused_scope in {"presentation", "chat-tools", "environment", "docs"}:
        return False
    return any(remote_sensitive(path) for path in paths)


def focused_defaults(paths: list[str]) -> tuple[bool, bool, bool, str]:
    scope = scope_for_paths(paths)
    if scope == "docs":
        return False, False, False, "docs"
    if scope == "presentation":
        return True, False, False, "presentation"
    if scope == "chat-tools":
        return False, True, False, "chat-tools"
    if scope == "environment":
        return False, False, True, "environment"
    return False, False, False, "full"


def conservative_route(paths: list[str], reason: str) -> Route:
    presentation, chat_tools, environment, scope = focused_defaults(paths)
    if scope == "full" and any(native_path(path) for path in paths):
        presentation = True
        scope = "presentation"
    return Route(
        presentation=presentation,
        chat_tools=chat_tools,
        environment=environment,
        backend_mode=fallback_backend_mode(paths, scope),
        native_batch=fallback_native_batch(paths),
        ssh=fallback_ssh(paths, scope),
        scope=scope,
        reason=reason,
    )


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
        return conservative_route(paths, "classifier response missing a single result")

    dimensions = results[0].get("dimensions")
    if not isinstance(dimensions, dict):
        return conservative_route(paths, "classifier response missing dimensions")

    lane, lane_confidence = _dimension(dimensions, "verification_lane")
    risk, risk_confidence = _dimension(dimensions, "regression_risk")
    backend, backend_confidence = _dimension(dimensions, "backend_acceptance")
    native, native_confidence = _dimension(dimensions, "organization_context_batch")
    ssh, ssh_confidence = _dimension(dimensions, "ssh_transport")
    model = response.get("model") if isinstance(response.get("model"), str) else None

    presentation, chat_tools, environment, focused_scope = focused_defaults(paths)

    if focused_scope == "full":
        valid_lanes = {PRESENTATION, CHAT_TOOLS, ENVIRONMENT, NO_FOCUSED_UI, BROAD_PRESENTATION}
        if lane in valid_lanes and lane_confidence is not None and lane_confidence >= LANE_CONFIDENCE:
            presentation = lane in {PRESENTATION, BROAD_PRESENTATION}
            chat_tools = lane == CHAT_TOOLS
            environment = lane == ENVIRONMENT
            focused_scope = {
                PRESENTATION: "presentation",
                CHAT_TOOLS: "chat-tools",
                ENVIRONMENT: "environment",
                NO_FOCUSED_UI: "baseline-only",
                BROAD_PRESENTATION: "presentation",
            }[lane]
        elif any(native_path(path) for path in paths):
            presentation = True
            focused_scope = "presentation"
        else:
            focused_scope = "baseline-only"

    backend_mode = fallback_backend_mode(paths, focused_scope)
    if backend in {BACKEND_SKIP, BACKEND_LINUX, BACKEND_FULL} and backend_confidence is not None and backend_confidence >= EXPENSIVE_CONFIDENCE:
        backend_mode = {
            BACKEND_SKIP: "skip",
            BACKEND_LINUX: "linux",
            BACKEND_FULL: "full",
        }[backend]
    hard_backend = hard_backend_mode(paths)
    if hard_backend is not None:
        backend_mode = hard_backend

    native_batch = fallback_native_batch(paths)
    if native in {NATIVE_SKIP, NATIVE_RUN} and native_confidence is not None and native_confidence >= EXPENSIVE_CONFIDENCE:
        native_batch = native == NATIVE_RUN
    if any(native_batch_anchor(path) for path in paths):
        native_batch = True

    ssh_run = fallback_ssh(paths, focused_scope)
    if ssh in {SSH_SKIP, SSH_RUN} and ssh_confidence is not None and ssh_confidence >= EXPENSIVE_CONFIDENCE:
        ssh_run = ssh == SSH_RUN
    if any(path in SSH_ANCHORS for path in paths):
        ssh_run = True

    if (
        risk == HIGH_RISK
        and risk_confidence is not None
        and risk_confidence >= RISK_CONFIDENCE
    ):
        if any(backend_relevant(path) for path in paths):
            backend_mode = "full"
        if any(native_path(path) for path in paths):
            presentation = True
            if focused_scope in {"baseline-only", "full"}:
                focused_scope = "presentation"

    return Route(
        presentation=presentation,
        chat_tools=chat_tools,
        environment=environment,
        backend_mode=backend_mode,
        native_batch=native_batch,
        ssh=ssh_run,
        scope=focused_scope,
        reason="classifier.dev Jev route with deterministic safety overrides",
        semantic_used=True,
        lane_label=lane,
        lane_confidence=lane_confidence,
        risk_label=risk,
        risk_confidence=risk_confidence,
        backend_label=backend,
        backend_confidence=backend_confidence,
        native_label=native,
        native_confidence=native_confidence,
        ssh_label=ssh,
        ssh_confidence=ssh_confidence,
        model=model,
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
        "Branch: main",
        (
            "Purpose: route CI. Cheap roadmap/format/security/dependency checks run separately. "
            "Expensive lanes are backend acceptance (Linux or full cross-platform), the specialized "
            "organization/saved-context native batch, SSH transport verification, and focused native UI journeys."
        ),
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
                    "Choose the narrowest focused native UI verification lane needed by the changed files. "
                    "Choose no focused native UI lane for backend-only, CI-only, or documentation-only changes."
                ),
            },
            "regression_risk": {
                "labels": [LOW_RISK, SUBSYSTEM_RISK, HIGH_RISK],
                "instructions": (
                    "Judge regression blast radius. Cross-cutting/high means shared contracts, dependencies, "
                    "multiple crates/subsystems, platform-sensitive behavior, or broad runtime impact."
                ),
            },
            "backend_acceptance": {
                "labels": [BACKEND_SKIP, BACKEND_LINUX, BACKEND_FULL],
                "instructions": (
                    "Choose whether expensive backend acceptance is needed. Skip for documentation, CI-router-only, "
                    "or narrowly covered UI changes. Linux-only is appropriate for backend logic without credible "
                    "platform-specific impact. Full cross-platform is for manifests/toolchains, shared portable "
                    "contracts, platform-sensitive changes, or broad/high-risk backend changes."
                ),
            },
            "organization_context_batch": {
                "labels": [NATIVE_SKIP, NATIVE_RUN],
                "instructions": (
                    "Run only when the change can affect organization/Spaces, saved context, task-context persistence, "
                    "IME shortcut guard behavior, or the specialized organization-context native journey."
                ),
            },
            "ssh_transport": {
                "labels": [SSH_SKIP, SSH_RUN],
                "instructions": (
                    "Run only when the change can affect SSH transport, remote filesystem/PTY/Git hosting, remote ACP "
                    "connectivity, SSH fixture behavior, host process transport, authentication, or related shared contracts."
                ),
            },
        },
    }
    headers = {
        "Content-Type": "application/json",
        "Accept": "application/json",
        "User-Agent": "synara-ci-router/2.0",
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
    if docs_only(paths):
        return Route(
            False, False, False, "skip", False, False, "docs",
            "documentation-only change",
        )
    try:
        response = classifier(context, api_key)
    except Exception as exc:
        return conservative_route(paths, f"classifier unavailable ({type(exc).__name__})")
    return semantic_route(paths, response)


def write_github_outputs(route: Route, path: str) -> None:
    values = {
        "presentation": route.presentation,
        "chat_tools": route.chat_tools,
        "environment": route.environment,
        "backend_mode": route.backend_mode,
        "native_batch": route.native_batch,
        "ssh": route.ssh,
        "scope": route.scope,
        "semantic_used": route.semantic_used,
        "lane_label": route.lane_label or "",
        "lane_confidence": "" if route.lane_confidence is None else f"{route.lane_confidence:.4f}",
        "risk_label": route.risk_label or "",
        "risk_confidence": "" if route.risk_confidence is None else f"{route.risk_confidence:.4f}",
        "backend_label": route.backend_label or "",
        "backend_confidence": "" if route.backend_confidence is None else f"{route.backend_confidence:.4f}",
        "native_label": route.native_label or "",
        "native_confidence": "" if route.native_confidence is None else f"{route.native_confidence:.4f}",
        "ssh_label": route.ssh_label or "",
        "ssh_confidence": "" if route.ssh_confidence is None else f"{route.ssh_confidence:.4f}",
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
            "full",
            True,
            True,
            "presentation",
            "comparison history unavailable; fail closed to broad expensive verification",
        )
        paths = []
    else:
        route = select_route(
            paths,
            context=build_context(paths, args.base, args.head),
            api_key=os.environ.get("CLASSIFIER_API_KEY") or None,
        )

    print(json.dumps({"route": asdict(route), "changed_paths": paths}, indent=2, sort_keys=True))
    if args.github_output:
        write_github_outputs(route, args.github_output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
