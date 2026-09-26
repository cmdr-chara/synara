#!/usr/bin/env python3
"""Hermetic first-run onboarding foundation for the live A04 journey."""
import argparse
import json
import sqlite3
from pathlib import Path

from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_controls_smoke import option
from native_integrations_smoke import click
from native_model_draft_smoke import close, preference


def project_registered(s):
    database = s.data / "native-workspace.sqlite3"
    if not database.exists():
        return False
    with sqlite3.connect(database.as_uri() + "?mode=ro", uri=True) as db:
        rows = [row[0] for row in db.execute("SELECT data FROM workspaces")]
    return any(str(s.project.resolve()) in row for row in rows)


def run(s):
    profiles = json.loads(s.profiles.read_text(encoding="utf-8"))[:1]
    profiles[0].update(id="auth", name="Fixture sign-in", args=["--integration-fixture", "auth"])
    s.profiles.write_text(json.dumps(profiles), encoding="utf-8")

    s.launch(preserve_selection=True)
    assert task_count(s) == 0 and not s.events()

    click(s, "onboarding-next")
    click(s, "onboarding-next")
    click(s, "onboarding-agent-prepare", slot=0)
    wait_until(lambda: task_count(s) == 1 and selection(s), "saved setup chat")
    task = selection(s)
    assert not s.events(), "preparing setup must not send a prompt"

    click(s, "onboarding-agent-connect", slot=0)
    wait_until(
        lambda: s.control_bounds("onboarding-auth-method", slot=0),
        "fixture advertised authentication",
    )
    click(s, "onboarding-auth-method", slot=0)
    wait_until(lambda: option(s, "model") == "auth", "fixture authentication completion")
    assert not any(event.get("type") == "prompt_started" for event in s.events()), "authentication must not submit a prompt"

    click(s, "onboarding-next")  # Appearance
    click(s, "onboarding-next")  # Project
    click(s, "onboarding-project-path")
    s.desktop.text(str(s.project))
    wait_until(
        lambda: s.control_bounds("onboarding-project-add-existing"),
        "existing project action",
    )
    click(s, "onboarding-project-add-existing")
    wait_until(lambda: project_registered(s), "persisted first project")
    s.checks.append("project-added-through-first-run-ui")

    click(s, "onboarding-next")  # Ready
    click(s, "onboarding-finish")
    wait_until(
        lambda: (preference(s, "settings") or {})
        .get("onboarding", {})
        .get("completed") is True,
        "persisted onboarding completion",
    )
    assert not any(event.get("type") == "prompt_started" for event in s.events()), "finishing onboarding must not submit a prompt"
    s.checks.append("finish-setup-persists-without-agent-autostart")

    s.desktop.key("1", ("Control_L",))
    before = s.prompt("hello")
    s.finished(before)
    assert s.has_text("Hello from auth", before)
    s.checks.append("completed-onboarding-provider-task-can-send-explicit-turn")

    events = s.events()
    close(s)
    s.launch(preserve_selection=True)
    assert selection(s) == task
    assert s.events() == events
    assert (preference(s, "settings") or {}).get("onboarding", {}).get("completed") is True
    s.checks.append("restart-restores-completed-onboarding-without-replay")
    close(s)


def main():
    parser = argparse.ArgumentParser()
    for name in ("binary", "fixture", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    s = Scenario(parser.parse_args())
    result = {"status": "failed", "checks": s.checks, "scope": "hermetic A04 foundation only"}
    try:
        run(s)
        result["status"] = "passed"
        print("A04_FOUNDATION_ACCEPTANCE: full first-run project/finish/restart path passed")
    except BaseException as error:
        result["error"] = str(error)
        if s.process and s.process.poll() is None:
            s.desktop.screenshot("failure", window_only=True)
        raise
    finally:
        s.close()
        (s.output / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
