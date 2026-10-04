#!/usr/bin/env python3
"""First-pass port mapping: items.json -> status.json.

For every upstream item, grep the port (~/Projects/synara-gpui-pr11, read-only)
for:
  * name evidence   - the component/command/key name (or its snake_case form)
                      appearing in port source, comments allowed
  * string evidence - user-visible string literals copied verbatim into port
                      code (comments stripped before matching)

Verdicts (auto-proposed, then hand-audited per README):
  done    - string evidence AND name evidence, or a curated equivalence hit
  partial - exactly one of string/name evidence, or an explicitly weaker
            equivalent (different chord, subset UI)
  missing - no evidence
  na      - hand-listed only; web-only/Electron-plumbing items that cannot
            exist in a native desktop app

Run `python3 map_status.py` after `extract.py`. Safe to re-run: it rewrites the
draft fields but preserves any entry whose `locked` flag is true.
"""

import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
UPSTREAM = os.path.expanduser(os.environ.get("PARITY_UPSTREAM", "~/parity/upstream"))
PORT = os.path.expanduser(os.environ.get("PARITY_PORT", "~/Projects/synara-gpui-pr11"))
ITEMS = os.path.join(HERE, "items.json")
STATUS = os.path.join(HERE, "status.json")
OVERRIDES = os.path.join(HERE, "overrides.json")

CODE_EXT = (".rs",)

GENERIC_WORDS = {
    "cancel", "close", "save", "ok", "loading", "error", "untitled", "submit",
    "delete", "rename", "open", "yes", "no", "done", "copy", "edit", "add",
    "remove", "back", "next", "continue", "search", "settings", "default",
}

# snake_case names that are too common to count as evidence on their own
WEAK_NAMES = {
    "sidebar", "composer", "settings", "terminal", "browser", "editor",
    "message", "thread", "chat", "panel", "header", "footer", "dialog",
    "menu", "list", "row", "item", "view", "icon", "button", "enabled",
    "accounts", "disabled", "preview", "version", "destination", "editor",
    "theme", "usage", "repository", "chats", "hubs", "skills", "providers",
    "shortcuts", "profile", "activity", "keybindings",
}

# server-schema fields too generic to prove a provider setting exists
GENERIC_SERVER_FIELDS = {
    "enabled", "accounts", "customModels", "disabled", "binaryPath",
    "homePath", "label", "id", "serverPasswordConfigured", "providerInstances",
    "onboardingCompletedAt",
}

# settings rows must show up in the UI crate to count
UI_CRATE_PREFIX = "crates/synara-app/src/"


def distinctive(s: str) -> bool:
    return len(s) >= 10 and (" " in s or len(s) >= 16)


def read(path):
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def strip_comments(src: str) -> str:
    """Remove // line comments and /* */ blocks (naive but effective)."""
    src = re.sub(r"/\*.*?\*/", "", src, flags=re.S)
    out = []
    for line in src.split("\n"):
        # keep the part before // unless inside a string-ish url; naive cut
        idx = line.find("//")
        if idx >= 0:
            line = line[:idx]
        out.append(line)
    return "\n".join(out)


def load_port_index():
    files = {}
    crates = os.path.join(PORT, "crates")
    for root, dirs, fns in os.walk(crates):
        dirs.sort()
        if "target" in dirs:
            dirs.remove("target")
        for fn in sorted(fns):
            if not fn.endswith(CODE_EXT):
                continue
            p = os.path.join(root, fn)
            rel = os.path.relpath(p, PORT)
            src = read(p)
            files[rel] = {
                "src": src,
                "clean": strip_comments(src),
                "lines": src.count("\n") + 1,
            }
    return files


def find(files, needle, clean_only=False, word=False):
    """Return (relpath, line) of first match, or None."""
    if not needle:
        return None
    rx = None
    if word:
        rx = re.compile(r"(?<![A-Za-z0-9_])" + re.escape(needle) + r"(?![A-Za-z0-9_])")
    for rel in sorted(files):
        text = files[rel]["clean"] if clean_only else files[rel]["src"]
        if rx:
            m = rx.search(text)
            if m:
                return (rel, text.count("\n", 0, m.start()) + 1)
        elif needle in text:
            return (rel, text[: text.index(needle)].count("\n") + 1)
    return None


def find_all(files, needle, clean_only=False, word=False):
    hits = []
    if not needle:
        return hits
    rx = None
    if word:
        rx = re.compile(r"(?<![A-Za-z0-9_])" + re.escape(needle) + r"(?![A-Za-z0-9_])")
    for rel in sorted(files):
        text = files[rel]["clean"] if clean_only else files[rel]["src"]
        if rx:
            for m in rx.finditer(text):
                hits.append((rel, text.count("\n", 0, m.start()) + 1))
        elif needle in text:
            hits.append((rel, text[: text.index(needle)].count("\n") + 1))
    return hits


def snake(name: str) -> str:
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name).lower()


def kebab(name: str) -> str:
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "-", name).lower()


# ---------------------------------------------------------------------------
# probe extraction from upstream component files

PROBE_PATTERNS = [
    re.compile(r">([A-Z][A-Za-z0-9][^<>{}()\n]{6,80})<"),
    re.compile(
        r'(?:title|aria-label|placeholder|label|description|tooltip|subtitle|'
        r'heading|emptyTitle|emptyText|emptyMessage|buttonLabel|confirmText|'
        r'confirmLabel|headerText|message)=\{?\s*"([^"{}]{6,80})"'
    ),
    re.compile(r'toast\.(?:success|error|info|warning|message)\(\s*"([^"{}]{8,100})"'),
    re.compile(r'"(You [a-z][^"{}]{10,90})"'),
    re.compile(r'"(Are you sure[^"{}]{0,80})"'),
]

FILE_PROBE_CACHE = {}


def file_probes(path, global_freq):
    if path in FILE_PROBE_CACHE:
        return FILE_PROBE_CACHE[path]
    src = read(os.path.join(UPSTREAM, path))
    cands = []
    for rx in PROBE_PATTERNS:
        cands += rx.findall(src)
    seen = set()
    probes = []
    for c in cands:
        c = " ".join(c.split()).strip(" .")
        if not (8 <= len(c) <= 80):
            continue
        if not re.search(r"[A-Za-z]", c):
            continue
        if any(t in c for t in ("${", "://", "=>", "classname", "px", "__")):
            continue
        if c.lower() in GENERIC_WORDS:
            continue
        if global_freq.get(c, 0) > 12:
            continue
        if c not in seen:
            seen.add(c)
            probes.append(c)
    probes.sort(key=lambda s: (" " not in s, len(s)))
    FILE_PROBE_CACHE[path] = probes[:4]
    return probes[:4]


# ---------------------------------------------------------------------------
# route expectations: path -> port module candidates (substrings of relpath)

ROUTE_MODULES = {
    "/": ["shell.rs", "main.rs"],
    "/_chat": ["shell/rail.rs", "shell.rs"],
    "/_chat/": ["shell/overview.rs", "shell/composer.rs"],
    "/_chat/$threadId": ["shell/conversation.rs", "shell/messages.rs"],
    "/_chat/settings": ["shell/settings.rs", "shell/settings/"],
    "/_chat/automations": ["shell/automations.rs"],
    "/_chat/automations/": ["shell/automations.rs"],
    "/_chat/automations/$automationId": ["shell/automations.rs"],
    "/_chat/hubs/": ["shell/hubs.rs", "shell/hubs/"],
    "/_chat/studio/": ["shell/studio.rs"],
    "/_chat/kanban/": ["shell/kanban.rs"],
    "/_chat/kanban/$projectId": ["shell/kanban.rs"],
    "/_chat/pull-requests": ["shell/pull_requests.rs"],
    "/_chat/pull-requests/": ["shell/pull_requests.rs"],
    "/_chat/inbox": ["shell/pull_requests.rs", "shell/notifications", "shell/recap.rs"],
    "/_chat/tasks/": ["shell/kanban.rs", "shell/task_split.rs"],
    "/_chat/groups/": ["shell/hubs.rs"],
    "/_chat/plugins": ["shell/registry.rs", "shell/integrations.rs"],
}


# keybinding equivalents: upstream command -> (port needle, verdict, note)
KB_EQUIV = {
    "editor.file.save": ("editor.save", "done", "port `editor.save` Primary+S matches upstream mod+s"),
    "model.next": ("model.next", "done", "port `model.next` Alt+] matches upstream alt+]"),
    "model.previous": ("model.previous", "done", "port `model.previous` Alt+[ matches upstream alt+["),
}


def kb_default(item):
    return ", ".join(k["key"] for k in item.get("keys", []) if k.get("key")) or "unassigned"


def main() -> int:
    items = json.load(open(ITEMS))
    files = load_port_index()
    old = {}
    if os.path.exists(STATUS):
        try:
            old = {e["id"]: e for e in json.load(open(STATUS))["items"]}
        except Exception:
            old = {}
    overrides = {}
    port_only_extra = []
    if os.path.exists(OVERRIDES):
        odoc = json.load(open(OVERRIDES))
        overrides = odoc.get("items", {})
        port_only_extra = odoc.get("port_only", [])

    # global probe frequency across component files (distinctiveness filter)
    freq = {}
    comp_files = sorted({i["file"] for i in items if i["kind"] == "component"})
    for f in comp_files:
        src = read(os.path.join(UPSTREAM, f))
        local = set()
        for rx in PROBE_PATTERNS:
            for c in rx.findall(src):
                c = " ".join(c.split()).strip(" .")
                if 8 <= len(c) <= 80 and re.search(r"[A-Za-z]", c):
                    local.add(c)
        for c in local:
            freq[c] = freq.get(c, 0) + 1

    out = []
    stats = {"done": 0, "partial": 0, "missing": 0, "na": 0}
    for item in items:
        iid = item["id"]
        prev = old.get(iid)
        if iid in overrides:
            o = overrides[iid]
            entry = {
                "id": iid,
                "port_ref": o.get("port_ref"),
                "status": o["status"],
                "note": o.get("note", ""),
                "evidence": o.get("evidence", []),
                "locked": True,
            }
            out.append(entry)
            stats[entry["status"]] = stats.get(entry["status"], 0) + 1
            continue
        if prev and prev.get("locked"):
            out.append(prev)
            stats[prev["status"]] = stats.get(prev["status"], 0) + 1
            continue
        entry = {"id": iid, "port_ref": None, "status": "missing", "note": "", "evidence": []}
        kind = item["kind"]

        if kind == "component":
            name = item["name"]
            names = [name]
            stem = os.path.splitext(os.path.basename(item["file"]))[0]
            if stem != name:
                names.append(stem)
            for cand in (snake(name), snake(stem)):
                if len(cand) >= 12 and cand not in WEAK_NAMES:
                    names.append(cand)
            name_hit = None
            for n in names:
                name_hit = find(files, n, word=True)
                if name_hit:
                    entry["evidence"].append(f"name `{n}`")
                    break
            probes = file_probes(item["file"], freq)
            str_hit = None
            str_probe = None
            for p in probes:
                if not distinctive(p):
                    continue
                str_hit = find(files, p, clean_only=True)
                if str_hit:
                    str_probe = p
                    entry["evidence"].append(f"string {p!r}")
                    break
            best = None
            for p in probes:
                if not distinctive(p):
                    continue
                for rel, ln in find_all(files, p, clean_only=True):
                    if name_hit and rel == name_hit[0]:
                        best = (rel, ln)
                        break
                if best:
                    break
            if name_hit and str_hit:
                entry["status"] = "done"
                entry["port_ref"] = f"{best[0]}:{best[1]}" if best else f"{str_hit[0]}:{str_hit[1]}"
            elif name_hit or str_hit:
                entry["status"] = "partial"
                hit = str_hit or name_hit
                entry["port_ref"] = f"{hit[0]}:{hit[1]}"
                entry["note"] = "only " + ("string" if str_hit else "name") + " evidence"
            else:
                entry["status"] = "missing"

        elif kind == "route":
            mods = ROUTE_MODULES.get(item["path"], [])
            mod_hit = None
            for m in mods:
                for rel in sorted(files):
                    if m in rel:
                        mod_hit = (rel, 1)
                        break
                if mod_hit:
                    break
            probes = file_probes(item["file"], freq)
            str_hit = None
            for p in probes:
                str_hit = find(files, p, clean_only=True)
                if str_hit:
                    entry["evidence"].append(f"string {p!r}")
                    break
            if mod_hit:
                entry["evidence"].append(f"module {mod_hit[0]}")
            if mod_hit and str_hit:
                entry["status"] = "done"
                entry["port_ref"] = f"{str_hit[0]}:{str_hit[1]}"
            elif mod_hit:
                entry["status"] = "partial"
                entry["port_ref"] = f"{mod_hit[0]}:{mod_hit[1]}"
                entry["note"] = "port module exists; upstream strings not found"
            elif str_hit:
                entry["status"] = "partial"
                entry["port_ref"] = f"{str_hit[0]}:{str_hit[1]}"
            else:
                entry["status"] = "missing"

        elif kind == "setting":
            if item["source"] == "search-index":
                title = item["label"]
                hit = find(files, title, clean_only=True)
                if hit and (len(title.split()) > 1 or hit[0].startswith(UI_CRATE_PREFIX + "shell/settings")):
                    entry["status"] = "done"
                    entry["port_ref"] = f"{hit[0]}:{hit[1]}"
                    entry["evidence"].append(f"label {title!r}")
                else:
                    # loose: all significant words in one UI file
                    words = [w for w in re.findall(r"[A-Za-z]{4,}", title.lower())]
                    hit = None
                    if words:
                        for rel in sorted(files):
                            if not rel.startswith(UI_CRATE_PREFIX):
                                continue
                            t = files[rel]["clean"].lower()
                            if all(w in t for w in words):
                                hit = (rel, 1)
                                break
                    if hit:
                        entry["status"] = "partial"
                        entry["port_ref"] = f"{hit[0]}:{hit[1]}"
                        entry["note"] = "row label not verbatim; words found"
                    else:
                        entry["status"] = "missing"
            else:
                key = item["key"].split(".")[-1]
                needles = [item["key"], snake(key), key]
                generic = key in GENERIC_SERVER_FIELDS and item["source"] == "server-schema"
                hit = None
                used = None
                for n in needles:
                    if generic:
                        for rel in sorted(files):
                            if not (
                                "workspace" in rel
                                or "registry" in rel
                                or "settings" in rel
                            ):
                                continue
                            text = files[rel]["clean"]
                            rx = re.compile(
                                r"(?<![A-Za-z0-9_])" + re.escape(n) + r"(?![A-Za-z0-9_])"
                            )
                            m = rx.search(text)
                            if m:
                                hit = (rel, text.count("\n", 0, m.start()) + 1)
                                break
                    else:
                        hit = find(files, n, clean_only=True, word=True)
                    if hit:
                        used = n
                        break
                if hit:
                    entry["status"] = "done"
                    entry["port_ref"] = f"{hit[0]}:{hit[1]}"
                    entry["evidence"].append(f"key `{used}`")
                else:
                    entry["status"] = "missing"

        elif kind == "keybinding":
            cmd = item.get("command")
            if cmd and cmd in KB_EQUIV:
                needle, verdict, note = KB_EQUIV[cmd]
                hit = find(files, needle, clean_only=True)
                entry["status"] = verdict if hit else "missing"
                entry["note"] = note
                if hit:
                    entry["port_ref"] = f"{hit[0]}:{hit[1]}"
                    entry["evidence"].append(f"equivalent `{needle}`")
            elif cmd:
                hit = find(files, cmd, clean_only=True)
                if hit:
                    entry["status"] = "done"
                    entry["port_ref"] = f"{hit[0]}:{hit[1]}"
                    entry["evidence"].append(f"command `{cmd}`")
                else:
                    entry["status"] = "missing"
                    if kb_default(item) != "unassigned":
                        entry["note"] = f"upstream default {kb_default(item)}"
            else:
                # fixed chord without command id - search by reason keywords later
                entry["status"] = "missing"

        if prev and prev.get("status") != entry["status"]:
            entry["note"] = (entry["note"] + f" [was {prev['status']}]").strip()
        stats[entry["status"]] = stats.get(entry["status"], 0) + 1
        out.append(entry)

    with open(STATUS, "w", encoding="utf-8") as fh:
        json.dump({"items": out, "port_only": port_only_extra}, fh, indent=2, ensure_ascii=False)
        fh.write("\n")
    print(json.dumps(stats, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
