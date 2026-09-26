#!/usr/bin/env python3
"""Manual fresh-install onboarding acceptance against a real pre-authenticated ACP provider.

The runner keeps its existing HOME so the provider can use its own previously
established account state. Synara itself always receives a brand-new --data-dir.
No credential is copied into the evidence directory.
"""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import time

from native_smoke import Scenario, wait_until
from native_navigation_smoke import selection, task_count
from native_model_draft_smoke import close, preference
from native_integrations_smoke import fill


def provider_profile():
    raw = os.environ.get("SYNARA_LIVE_PROVIDER_PROFILE", "")
    if not raw or len(raw) > 16 * 1024:
        raise RuntimeError("SYNARA_LIVE_PROVIDER_PROFILE must contain one bounded provider profile")
    value = json.loads(raw)
    if not isinstance(value, dict):
        raise RuntimeError("provider profile must be an object")
    allowed = {"id", "name", "command", "args", "inherit_env"}
    if set(value) - allowed:
        raise RuntimeError("provider profile contains unsupported fields; credentials must stay in provider-owned state")
    for key in ("id", "name", "command"):
        if not isinstance(value.get(key), str) or not value[key].strip():
            raise RuntimeError(f"provider profile requires {key}")
    args = value.get("args", [])
    if not isinstance(args, list) or not all(isinstance(item, str) for item in args):
        raise RuntimeError("provider profile args must be strings")
    inherit = value.get("inherit_env", [])
    if not isinstance(inherit, list) or not all(isinstance(item, str) for item in inherit):
        raise RuntimeError("inherit_env must contain environment variable names")
    value["args"] = args
    value["inherit_env"] = inherit
    return value


def assistant_text(events, after):
    return "".join(
        event.get("text", "")
        for event in events[after:]
        if event.get("type") == "text_delta" and event.get("role") == "assistant"
    ).strip()


def main():
    parser = argparse.ArgumentParser()
    for name in ("binary", "fixture", "output"):
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()

    profile = provider_profile()
    scenario = Scenario(args)
    # Fresh Synara state, real provider-owned login state.
    scenario.home = Path.home().resolve()
    scenario.profiles.write_text(json.dumps([profile]), encoding="utf-8")
    result = {
        "status": "failed",
        "provider": {"id": profile["id"], "name": profile["name"]},
        "checks": scenario.checks,
        "credentials_copied": False,
    }
    try:
        scenario.launch(preserve_selection=True)
        assert task_count(scenario) == 0 and not scenario.events()
        scenario.click_control("onboarding-next")
        scenario.click_control("onboarding-next")
        scenario.click_control("onboarding-agent-prepare", slot=0)
        wait_until(lambda: task_count(scenario) == 1 and selection(scenario), "real-provider setup chat", 30)
        task = selection(scenario)
        assert not scenario.events(), "preparing onboarding must not send a provider prompt"

        scenario.click_control("onboarding-agent-connect", slot=0)
        # The provider is expected to be pre-authenticated on this dedicated
        # runner. Connection may continue while the user completes the remaining
        # first-run pages; onboarding itself must never submit a prompt.
        time.sleep(3)

        scenario.click_control("onboarding-next")  # Appearance
        scenario.click_control("onboarding-next")  # Project
        fill(scenario, "onboarding-project-path", str(scenario.project))
        wait_until(
            lambda: scenario.control_bounds("onboarding-project-add-existing"),
            "existing project onboarding action",
        )
        scenario.click_control("onboarding-project-add-existing")

        database = scenario.data / "native-workspace.sqlite3"
        def project_registered():
            if not database.exists():
                return False
            with sqlite3.connect(database.as_uri() + "?mode=ro", uri=True) as db:
                rows = [row[0] for row in db.execute("SELECT data FROM workspaces")]
            return any(str(scenario.project.resolve()) in row for row in rows)

        wait_until(project_registered, "persisted onboarding project", 30)
        scenario.checks.append("fresh-install-project-is-added-through-onboarding")

        scenario.click_control("onboarding-next")  # Ready
        scenario.click_control("onboarding-finish")
        wait_until(
            lambda: (preference(scenario, "settings") or {})
                .get("onboarding", {})
                .get("completed") is True,
            "persisted onboarding completion",
            20,
        )
        assert not any(event.get("type") == "prompt_started" for event in scenario.events()), "finishing onboarding must not send a provider prompt"
        scenario.checks.append("fresh-install-onboarding-completes-without-agent-autostart")

        # Prove the previously connected real account can perform one explicit
        # user turn after the full first-run flow.
        scenario.desktop.key("1", ("Control_L",))
        before = scenario.prompt("Reply briefly with SYNARA_ACCEPTED to confirm this real provider session.")
        wait_until(
            lambda: any(event.get("type") == "prompt_finished" for event in scenario.events()[before:]),
            "real provider prompt completion",
            120,
        )
        reply = assistant_text(scenario.events(), before)
        assert reply, "real provider produced no assistant response"
        scenario.checks.append("fresh-install-setup-connects-and-completes-real-provider-turn")

        events = scenario.events()
        close(scenario)
        scenario.launch(preserve_selection=True)
        assert selection(scenario) == task, "setup task was not restored after restart"
        assert scenario.events() == events, "restart replayed or mutated provider history"
        settings = preference(scenario, "settings") or {}
        assert settings.get("onboarding", {}).get("completed") is True
        scenario.checks.append("fresh-install-provider-project-and-history-replay-without-autostart")
        result["status"] = "passed"
        result["assistant_response_observed"] = True
        print("ONBOARDING_LIVE_ACCEPTANCE: fresh Synara data + real provider account + project + completed onboarding + durable replay")
    except BaseException as error:
        result["error"] = str(error)
        if scenario.process and scenario.process.poll() is None:
            scenario.desktop.screenshot("failure", window_only=True)
        raise
    finally:
        scenario.close()
        (scenario.output / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
