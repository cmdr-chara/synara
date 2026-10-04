#!/usr/bin/env python3
"""Seed one parity instance pair (upstream + port) from fixture.json.

  python3 seed.py <inst>             stop pair, wipe instance dirs, re-seed, leave stopped
  python3 seed.py <inst> --launch    same, then launch both apps and frame them

Seeds:
  upstream  ~/parity/inst/<inst>/up-home/userdata/state.sqlite  (orchestration_events)
            ~/parity/inst/<inst>/up-home/userdata/settings.json (onboardingCompletedAt)
            renderer localStorage via the app's own UI (theme, welcome sheets, open thread)
  port      ~/parity/inst/<inst>/port-data/native-workspace.sqlite3
  repos     ~/parity/fixtures/repos/{atlas-web,orbit-api}       (real git repos)
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import sqlite3
import subprocess
import sys
import time
import uuid
from datetime import datetime, timedelta, timezone
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import lib  # noqa: E402
import drive  # noqa: E402

ROOT = lib.ROOT
INST = lib.INST
HARNESS = Path(__file__).resolve().parent
FIXTURE = HARNESS / "fixture.json"
REPOS = ROOT / "fixtures" / "repos"
WIPE_DIRS = ("up-home", "up-userdata", "port-data")
GUILOCK = ROOT / ".guilock"
NS = uuid.UUID("3f5a1c2e-9b7d-4e6a-8c0f-2d4b6a8c1e3f")
DEFAULT_PORT_BIN = Path.home() / "Projects" / "synara-gpui-pr11" / "target" / "debug" / "synara-app"

os.environ.setdefault("PARITY_CUA_SESSION", "seed")


def log(msg: str) -> None:
    print(f"[seed] {msg}", flush=True)


def sh(*cmd, env: dict | None = None) -> None:
    subprocess.run([str(c) for c in cmd], check=True, env=env)


def launch(*args) -> None:
    sh(sys.executable, HARNESS / "launch.py", *args)


def valid_inst(name: str) -> Path:
    if not name or name in (".", "..") or "/" in name or "\\" in name:
        raise SystemExit(f"bad instance name: {name!r}")
    return INST / name


def wipe(d: Path) -> None:
    for sub in WIPE_DIRS:
        target = d / sub
        resolved = target.resolve()
        if resolved != d.resolve() and d.resolve() not in resolved.parents:
            raise SystemExit(f"refusing to wipe outside instance dir: {resolved}")
        shutil.rmtree(target, ignore_errors=True)


def build_repos(fx: dict, now: datetime) -> dict[str, Path]:
    REPOS.mkdir(parents=True, exist_ok=True)
    out = {}
    for name, spec in fx["repos"].items():
        path = REPOS / name
        shutil.rmtree(path, ignore_errors=True)
        path.mkdir(parents=True)
        env = dict(
            os.environ,
            GIT_AUTHOR_NAME="Parity Seed",
            GIT_AUTHOR_EMAIL="seed@parity.local",
            GIT_COMMITTER_NAME="Parity Seed",
            GIT_COMMITTER_EMAIL="seed@parity.local",
        )
        sh("git", "init", "-q", "-b", "main", path)
        for i, commit in enumerate(spec["commits"]):
            for rel, text in commit["files"].items():
                f = path / rel
                f.parent.mkdir(parents=True, exist_ok=True)
                f.write_text(text)
            stamp = (now + timedelta(seconds=i * 600)).strftime("%Y-%m-%dT%H:%M:%S+00:00")
            env["GIT_AUTHOR_DATE"] = env["GIT_COMMITTER_DATE"] = stamp
            sh("git", "-C", path, "add", "-A", env=env)
            sh("git", "-C", path, "commit", "-q", "-m", commit["message"], env=env)
        out[name] = path
        log(f"repo {name}: {len(spec['commits'])} commits")
    return out


def iso(ts: datetime) -> str:
    return ts.astimezone(timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.") + f"{ts.microsecond // 1000:03d}Z"


def wait_file(path: Path, timeout: float = 60) -> None:
    end = time.time() + timeout
    while time.time() < end:
        if path.exists():
            return
        time.sleep(0.5)
    raise SystemExit(f"timed out waiting for {path}")


# ── upstream ────────────────────────────────────────────────────────────────

EVENT_SQL = """INSERT INTO orchestration_events
  (event_id, aggregate_kind, stream_id, stream_version, event_type, occurred_at,
   command_id, causation_event_id, correlation_id, actor_kind, payload_json, metadata_json)
  VALUES (?,?,?,?,?,?,?,NULL,?,'client',?,?)"""
META = json.dumps({"persistedEventSchemaVersion": 1})
MODEL_SELECTION = {"provider": "codex", "instanceId": "codex", "model": "gpt-6-luna"}


def up_insert(cur: sqlite3.Cursor, aggregate: str, stream: str, version: int,
              etype: str, at: datetime, payload: dict) -> None:
    cmd = str(uuid.uuid5(NS, f"cmd:{etype}:{stream}:{version}"))
    eid = str(uuid.uuid5(NS, f"evt:{etype}:{stream}:{version}"))
    cur.execute(EVENT_SQL, (eid, aggregate, stream, version, etype, iso(at),
                            cmd, cmd, json.dumps(payload), META))


def check_upstream_schema(db_path: Path) -> None:
    """The server holds the DB lock for its whole lifetime, so this runs only
    after the app has been stopped."""
    db = sqlite3.connect(db_path)
    try:
        tables = {r[0] for r in db.execute(
            "SELECT name FROM sqlite_master WHERE type='table'")}
        required = {"orchestration_events", "projection_projects",
                    "projection_threads", "projection_thread_messages",
                    "effect_sql_migrations"}
        missing = required - tables
        if missing:
            raise SystemExit(f"upstream schema incomplete after boot; missing {missing}")
        migrations = db.execute(
            "SELECT COUNT(*) FROM effect_sql_migrations").fetchone()[0]
        log(f"upstream: schema ready ({migrations} migrations applied)")
    finally:
        db.close()


def seed_upstream(d: Path, fx: dict, now: datetime, repos: dict[str, Path],
                  home_id: str) -> dict[str, str]:
    home = d / "up-home"
    db_path = home / "userdata" / "state.sqlite"
    settings_path = home / "userdata" / "settings.json"

    settings = json.loads(settings_path.read_text())
    settings.setdefault("settings", {})["onboardingCompletedAt"] = iso(now)
    settings_path.write_text(json.dumps(settings, indent=2))

    project_ids = {}
    thread_ids = {}
    db = sqlite3.connect(db_path)
    cur = db.cursor()
    at = now + timedelta(seconds=fx.get("home_created_offset_s", -259200))
    up_insert(cur, "project", home_id, 0, "project.created", at, {
        "projectId": home_id,
        "kind": "chat",
        "title": "Home",
        "workspaceRoot": str(Path.home()),
        "defaultModelSelection": None,
        "scripts": [],
        "isPinned": False,
        "spaceId": None,
        "createdAt": iso(at),
        "updatedAt": iso(at),
    })
    for proj in fx["projects"]:
        pid = str(uuid.uuid5(NS, f"project:{proj['key']}"))
        project_ids[proj["key"]] = pid
        at = now + timedelta(seconds=proj["created_offset_s"])
        up_insert(cur, "project", pid, 0, "project.created", at, {
            "projectId": pid,
            "kind": "project",
            "title": proj["title"],
            "workspaceRoot": str(repos[proj["repo"]]),
            "defaultModelSelection": None,
            "scripts": [],
            "isPinned": False,
            "spaceId": None,
            "createdAt": iso(at),
            "updatedAt": iso(at),
        })
    for thread in fx["threads"]:
        tid = str(uuid.uuid5(NS, f"thread:{thread['key']}"))
        thread_ids[thread["key"]] = tid
        pid = project_ids[thread["project"]] if thread["project"] else home_id
        at = now + timedelta(seconds=thread["created_offset_s"])
        up_insert(cur, "thread", tid, 0, "thread.created", at, {
            "threadId": tid,
            "projectId": pid,
            "title": thread["title"],
            "modelSelection": MODEL_SELECTION,
            "branch": None,
            "worktreePath": None,
            "createdAt": iso(at),
            "updatedAt": iso(at),
        })
        for n, msg in enumerate(thread["messages"], start=1):
            mid = str(uuid.uuid5(NS, f"msg:{thread['key']}:{n}"))
            at = now + timedelta(seconds=msg["offset_s"])
            up_insert(cur, "thread", tid, n, "thread.message-sent", at, {
                "threadId": tid,
                "messageId": mid,
                "role": msg["role"],
                "text": msg["text"],
                "turnId": None,
                "streaming": False,
                "createdAt": iso(at),
                "updatedAt": iso(at),
            })
    db.commit()
    count = db.execute("SELECT COUNT(*) FROM orchestration_events").fetchone()[0]
    db.close()
    log(f"upstream: {count} events, home project {home_id}")
    thread_ids["__home__"] = home_id

    out = ROOT / "work" / "seed"
    out.mkdir(parents=True, exist_ok=True)
    (out / f"seed-upstream-{d.name}.json").write_text(json.dumps(
        {"projects": project_ids, "threads": thread_ids}, indent=2))
    return thread_ids


# ── port ────────────────────────────────────────────────────────────────────

# DDL mirrors crates/synara-workspace/src/storage.rs migrations (user_version 1-3).
PORT_DDL = """
CREATE TABLE workspaces(id TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE projects(id TEXT PRIMARY KEY, workspace_id TEXT NOT NULL REFERENCES workspaces(id), data TEXT NOT NULL);
CREATE TABLE tasks(id TEXT PRIMARY KEY, project_id TEXT NOT NULL REFERENCES projects(id), thread_id TEXT NOT NULL UNIQUE, updated_ms INTEGER NOT NULL, data TEXT NOT NULL);
CREATE TABLE sessions(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), data TEXT NOT NULL);
CREATE TABLE events(thread_id TEXT NOT NULL REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>0), id TEXT NOT NULL UNIQUE, timestamp_ms INTEGER NOT NULL, data TEXT NOT NULL, PRIMARY KEY(thread_id,sequence));
CREATE INDEX task_recency ON tasks(updated_ms DESC);
CREATE TABLE preferences(key TEXT PRIMARY KEY, data TEXT NOT NULL);
CREATE TABLE event_heads(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>=0), bytes INTEGER NOT NULL CHECK(bytes>=0));
CREATE TABLE thread_activity(thread_id TEXT PRIMARY KEY REFERENCES tasks(thread_id), sequence INTEGER NOT NULL CHECK(sequence>=0), data TEXT NOT NULL);
PRAGMA user_version=3;
"""

PORT_SETTINGS = {
    "version": 1,
    "appearance": {"theme": "dark"},
    "onboarding": {"started": True, "completed": True},
}


def ms(ts: datetime) -> int:
    return int(ts.timestamp() * 1000)


def port_workspace(cur: sqlite3.Cursor, ws_id: str, name: str, root: str) -> None:
    cur.execute("INSERT INTO workspaces(id,data) VALUES(?,?)", (ws_id, json.dumps({
        "id": ws_id, "name": name, "location": {"kind": "local", "root": root}})))


def port_project(cur: sqlite3.Cursor, pid: str, ws_id: str, name: str) -> None:
    cur.execute("INSERT INTO projects(id,workspace_id,data) VALUES(?,?,?)",
                (pid, ws_id, json.dumps({
                    "id": pid, "workspace_id": ws_id, "name": name,
                    "relative_directory": ""})))


def seed_port(d: Path, fx: dict, now: datetime, repos: dict[str, Path]) -> dict[str, str]:
    data = d / "port-data"
    chats_root = data / "chats"
    chats_root.mkdir(parents=True, exist_ok=True)
    db = sqlite3.connect(data / "native-workspace.sqlite3")
    cur = db.cursor()
    cur.executescript(PORT_DDL)

    project_ids: dict[str, str] = {}
    workdirs: dict[str, str] = {}
    open_ref: dict[str, str] = {}
    for proj in fx["projects"]:
        root = str(repos[proj["repo"]])
        ws_id = str(uuid.uuid5(NS, f"ws:{proj['key']}"))
        pid = str(uuid.uuid5(NS, f"project:{proj['key']}"))
        project_ids[proj["key"]] = pid
        workdirs[proj["key"]] = root
        port_workspace(cur, ws_id, proj["title"], root)
        port_project(cur, pid, ws_id, proj["title"])

    for thread in fx["threads"]:
        scope = "project" if thread["project"] else "chat"
        if thread["project"]:
            pid = project_ids[thread["project"]]
            workdir = workdirs[thread["project"]]
        else:
            # Mirrors shell.rs create_chat: scratch dir under <data>/chats/<id>,
            # registered as its own workspace+project, hidden from Projects by
            # is_chat_workspace.
            chat_dir = chats_root / str(uuid.uuid5(NS, f"chatdir:{thread['key']}"))
            chat_dir.mkdir(parents=True, exist_ok=True)
            ws_id = str(uuid.uuid5(NS, f"ws:chat:{thread['key']}"))
            pid = str(uuid.uuid5(NS, f"project:chat:{thread['key']}"))
            port_workspace(cur, ws_id, chat_dir.name, str(chat_dir))
            port_project(cur, pid, ws_id, chat_dir.name)
            workdir = str(chat_dir)

        task_id = str(uuid.uuid5(NS, f"task:{thread['key']}"))
        thread_id = str(uuid.uuid5(NS, f"thread:{thread['key']}"))
        last_ms = ms(now + timedelta(seconds=thread["messages"][-1]["offset_s"]))
        task_data = {
            "id": task_id, "project_id": pid, "title": thread["title"],
            "state": "completed", "thread_id": thread_id, "agent_id": "opencode",
            "working_directory": workdir, "updated_at_ms": last_ms, "scope": scope,
        }
        cur.execute("INSERT INTO tasks(id,project_id,thread_id,updated_ms,data) VALUES(?,?,?,?,?)",
                    (task_id, pid, thread_id, last_ms, json.dumps(task_data)))
        if thread["key"] == fx["open_thread"]:
            open_ref = {"project": pid, "task": task_id}

        # One turn per user message: prompt_started, user delta, assistant delta,
        # prompt_finished (mirrors "Seeding conversation events" in
        # testing-synara-app/SKILL.md). Fixture threads alternate user/assistant
        # and end on an assistant message.
        if len(thread["messages"]) % 2:
            raise SystemExit(f"thread {thread['key']!r} has an odd message count")
        seq = 0
        total = 0
        turns = [(thread["messages"][i], thread["messages"][i + 1])
                 for i in range(0, len(thread["messages"]), 2)]
        for t, (user, reply) in enumerate(turns, start=1):
            for suffix, event, offset in (
                ("start", {"type": "prompt_started", "turn": f"t{t}"}, user["offset_s"]),
                ("user", {"type": "text_delta", "message_id": f"m{t}u",
                          "role": "user", "text": user["text"]}, user["offset_s"]),
                ("asst", {"type": "text_delta", "message_id": f"m{t}a",
                          "role": "assistant", "text": reply["text"]}, reply["offset_s"]),
                ("end", {"type": "prompt_finished", "reason": "end_turn"}, reply["offset_s"]),
            ):
                seq += 1
                encoded = json.dumps(event)
                total += len(encoded)
                cur.execute(
                    "INSERT INTO events(thread_id,sequence,id,timestamp_ms,data) VALUES(?,?,?,?,?)",
                    (thread_id, seq,
                     str(uuid.uuid5(NS, f"evt:{thread['key']}:{seq}:{suffix}")),
                     ms(now + timedelta(seconds=offset)), encoded))
        cur.execute("INSERT INTO event_heads(thread_id,sequence,bytes) VALUES(?,?,?)",
                    (thread_id, seq, total))
        cur.execute("INSERT INTO thread_activity(thread_id,sequence,data) VALUES(?,?,?)",
                    (thread_id, seq, json.dumps({
                        "title": thread["title"], "state": "completed", "active": False,
                        "permissions": [], "inputs": [], "history_title": None,
                    })))

    cur.execute("INSERT INTO preferences(key,data) VALUES('settings',?)",
                (json.dumps(PORT_SETTINGS),))
    cur.execute("INSERT INTO preferences(key,data) VALUES('selection',?)",
                (json.dumps(open_ref),))
    db.commit()
    db.close()
    log(f"port: {len(fx['projects'])} projects, {len(fx['threads'])} tasks seeded")
    return project_ids


# ── renderer seeding via the app's own UI (localStorage) ────────────────────

def dismiss_sheets(up: drive.App) -> None:
    """Click through welcome/announcement sheets (they queue one at a time)."""
    for _ in range(8):
        found = False
        for label in ("Not now", "Skip", "Got it", "Maybe later"):
            try:
                up.click(label, role="AXButton", settle=1.0)
                log(f"upstream: dismissed sheet via {label!r}")
                found = True
                break
            except (drive.NotFound, RuntimeError):
                continue
        if not found:
            return
        time.sleep(1.5)


def drive_upstream(inst: str, fx: dict) -> None:
    up = drive.App.from_state(inst, "up")
    # Sidebar is ready once a seeded project name is in the AX tree.
    up.wait_for(fx["projects"][0]["title"], timeout=120)
    time.sleep(3)  # let deferred sheets surface
    dismiss_sheets(up)
    # Dark theme via the Cmd+K search palette's theme command.
    up.hotkey("cmd", "k", settle=1.5)
    up.type("dark", settle=1.0)
    try:
        up.click("Switch to dark theme", settle=1.0)
        log("upstream: theme set to dark via palette")
    except (drive.NotFound, RuntimeError):
        try:
            up.click("Switch to dark theme", pixel=True, settle=1.0)
            log("upstream: theme set to dark via palette (pixel)")
        except (drive.NotFound, RuntimeError):
            up.key("escape")
            up.hotkey("cmd", "a")
            up.key("delete")
            log("upstream: 'Switch to dark theme' palette item not found")
    # Open the fixture's target thread so lastThreadRoute persists it.
    title = next(t["title"] for t in fx["threads"] if t["key"] == fx["open_thread"])
    try:
        up.click(title, settle=1.5)
    except (drive.NotFound, RuntimeError):
        up.hotkey("cmd", "k", settle=1.5)
        up.type(title, settle=1.0)
        up.click(title, settle=1.5)
    log(f"upstream: opened thread {title!r}")


# ── main ────────────────────────────────────────────────────────────────────

def acquire_guilock() -> None:
    while True:
        try:
            GUILOCK.mkdir()
            (GUILOCK / "pid").write_text(str(os.getpid()))
            (GUILOCK / "owner").write_text("seed")
            return
        except FileExistsError:
            pid_file = GUILOCK / "pid"
            pid = pid_file.read_text().strip() if pid_file.exists() else ""
            if pid:
                try:
                    os.kill(int(pid), 0)
                except (ProcessLookupError, ValueError):
                    shutil.rmtree(GUILOCK, ignore_errors=True)
                    continue
            time.sleep(3)


def release_guilock() -> None:
    if GUILOCK.exists() and (GUILOCK / "owner").read_text().strip() == "seed":
        shutil.rmtree(GUILOCK, ignore_errors=True)


def main() -> None:
    ap = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("inst")
    ap.add_argument("--launch", action="store_true")
    ap.add_argument("--bin", type=Path, default=DEFAULT_PORT_BIN)
    a = ap.parse_args()

    d = valid_inst(a.inst)
    fx = json.loads(FIXTURE.read_text())
    now = datetime.now(timezone.utc)

    log(f"stop {a.inst}")
    launch("stop", a.inst, "all")
    wipe(d)
    repos = build_repos(fx, now)

    # Upstream phase 1: boot once so migrations run (window up = server ready),
    # wait briefly for the journal to settle, then stop before writing events.
    log("upstream: first boot (migrations)")
    db_path = d / "up-home" / "userdata" / "state.sqlite"
    settings_path = d / "up-home" / "userdata" / "settings.json"
    try:
        launch("up", a.inst)
        wait_file(db_path, 90)
        wait_file(settings_path, 30)
        time.sleep(5)
    finally:
        launch("stop", a.inst, "up")
    check_upstream_schema(db_path)
    home_id = str(uuid.uuid5(NS, "project:home"))
    seed_upstream(d, fx, now, repos, home_id)

    # Port seeds entirely offline; the app must not be running.
    seed_port(d, fx, now, repos)

    # Upstream phase 2: boot again so the renderer seeds localStorage via its UI.
    log("upstream: second boot (renderer seeding)")
    acquire_guilock()
    try:
        launch("up", a.inst)
        drive_upstream(a.inst, fx)
    except BaseException:
        launch("stop", a.inst, "up")
        raise
    finally:
        release_guilock()
    if not a.launch:
        launch("stop", a.inst, "up")
        log("done; both apps stopped")
        return

    launch("port", a.inst, "--bin", str(a.bin))
    launch("frame", a.inst)
    log("launched and framed")


if __name__ == "__main__":
    main()
