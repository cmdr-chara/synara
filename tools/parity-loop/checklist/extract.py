#!/usr/bin/env python3
"""Extract the upstream parity checklist into items.json.

Deterministic: same input tree -> byte-identical output. Run twice, shasum both.

Sources (all under ~/parity/upstream, current main):
  routes      apps/web/src/routes/*.tsx defining createFileRoute/createRootRoute
              (skip "-*" helper files and tests)
  components  every .tsx under apps/web/src/components except *.test.tsx,
              *.browser.tsx, *.stories.tsx — one item per exported component,
              one item per file minimum
  settings    SETTINGS_SEARCH_ENTRIES rows in apps/web/src/settingsSearchIndex.ts
              + every field of AppSettingsSchema in apps/web/src/appSettings.ts
              + every field of ServerSettings in packages/contracts/src/settings.ts
              (provider structs flattened one level)
  keybindings STATIC_KEYBINDING_COMMANDS in packages/contracts/src/keybindings.ts
              with default keys from apps/server/src/keybindings.ts
              (DEFAULT_KEYBINDINGS) and apps/web/src/keybindings.ts
              (DEFAULT_SHORTCUT_FALLBACKS) + FIXED_SHORTCUTS chords in
              apps/web/src/fixedShortcuts.ts + the script.*.run pattern from
              KEYBINDINGS.md
"""

import json
import os
import re
import sys

UPSTREAM = os.path.expanduser(os.environ.get("PARITY_UPSTREAM", "~/parity/upstream"))
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "items.json")

WEB = os.path.join(UPSTREAM, "apps/web/src")
ROUTES_DIR = os.path.join(WEB, "routes")
COMPONENTS_DIR = os.path.join(WEB, "components")
CONTRACTS = os.path.join(UPSTREAM, "packages/contracts/src")
SERVER = os.path.join(UPSTREAM, "apps/server/src")

AREAS = [
    "chrome",
    "rail-sidebar",
    "home-composer",
    "thread",
    "settings",
    "kanban",
    "pull-requests",
    "automations",
    "studio-hubs",
    "inbox-tasks-groups-plugins",
    "onboarding",
    "right-dock",
    "dialogs-menus-palette",
    "other",
]


def rel(path: str) -> str:
    return os.path.relpath(path, UPSTREAM)


def read(path: str) -> str:
    with open(path, encoding="utf-8") as fh:
        return fh.read()


def line_of(src: str, index: int) -> int:
    return src.count("\n", 0, index) + 1


def block_from(src: str, start: int, open_ch: str = "{", close_ch: str = "}") -> tuple[str, int]:
    """Return (block text including braces, end index) for the first balanced
    open_ch..close_ch starting at or after `start`. Brace counting skips
    string literals and comments well enough for schema blocks."""
    i = src.index(open_ch, start)
    depth = 0
    j = i
    in_str = None
    in_line_comment = False
    in_block_comment = False
    while j < len(src):
        ch = src[j]
        nxt = src[j + 1] if j + 1 < len(src) else ""
        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
        elif in_block_comment:
            if ch == "*" and nxt == "/":
                in_block_comment = False
                j += 1
        elif in_str:
            if ch == "\\":
                j += 1
            elif ch == in_str:
                in_str = None
        else:
            if ch == "/" and nxt == "/":
                in_line_comment = True
            elif ch == "/" and nxt == "*":
                in_block_comment = True
                j += 1
            elif ch in ('"', "'", "`"):
                in_str = ch
            elif ch == open_ch:
                depth += 1
            elif ch == close_ch:
                depth -= 1
                if depth == 0:
                    return src[i : j + 1], j + 1
        j += 1
    raise ValueError(f"unbalanced block at offset {i}")


def top_level_fields(block: str) -> list[tuple[str, int, str]]:
    """Split a `{ key: value, ... }` object literal into (key, offset, value)
    at brace depth 1. Values run to the next top-level comma."""
    assert block.startswith("{") and block.endswith("}")
    body = block[1:-1]
    fields = []
    depth = 0
    i = 0
    n = len(body)
    in_str = None
    key = None
    key_off = 0
    val_start = 0
    while i < n:
        ch = body[i]
        nxt = body[i + 1] if i + 1 < n else ""
        if in_str:
            if ch == "\\":
                i += 1
            elif ch == in_str:
                in_str = None
        elif ch in ('"', "'", "`"):
            in_str = ch
        elif ch == "/" and nxt == "/":
            while i < n and body[i] != "\n":
                i += 1
        elif ch in "{[(":
            depth += 1
        elif ch in "}])":
            depth -= 1
        elif depth == 0:
            if ch == "," or i == n - 1:
                if key is not None:
                    val = body[val_start : i + (0 if ch == "," else 1)].rstrip().rstrip(",")
                    fields.append((key, key_off, val.strip()))
                    key = None
            elif key is None:
                m = re.match(r"\s*((?:\.\.\.)?[A-Za-z_$][A-Za-z0-9_$]*)\s*(:)?", body[i:])
                if m and (m.group(2) or m.group(1).startswith("...")):
                    key = m.group(1)
                    key_off = i + m.start(1)
                    val_start = i + m.end()
                    i += m.end() - 1
        i += 1
    if key is not None:
        fields.append((key, key_off, body[val_start:].strip()))
    return fields


def default_expr(field_text: str):
    """Extract the literal default from withDefaults/withDecodingDefault calls."""
    m = re.search(r"(?:withDefaults|withDecodingDefault)\(\s*(?:\(\)\s*=>\s*)?", field_text)
    if not m:
        return None
    i = m.end()
    # balanced read until the call's closing paren
    depth = 1
    j = i
    in_str = None
    while j < len(field_text):
        ch = field_text[j]
        if in_str:
            if ch == "\\":
                j += 1
            elif ch == in_str:
                in_str = None
        elif ch in ('"', "'", "`"):
            in_str = ch
        elif ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                break
        j += 1
    return field_text[i:j].strip()


def literal(expr):
    """Turn a simple JS literal expression into a python value; else return the
    raw source text (truncated)."""
    if expr is None:
        return None
    e = expr.strip()
    if e in ("true", "false"):
        return e == "true"
    if e == "null":
        return None
    if re.fullmatch(r"-?\d+(\.\d+)?", e):
        return float(e) if "." in e else int(e)
    if re.fullmatch(r'"([^"\\]|\\.)*"', e):
        return json.loads(e)
    if e in ("[]", "{}"):
        return json.loads(e)
    return e[:160]


def resolve_const(expr, consts):
    if isinstance(expr, str) and expr in consts:
        return consts[expr]
    return expr


def collect_consts(src: str) -> dict:
    consts = {}
    for m in re.finditer(r"(?:export\s+)?const\s+([A-Z][A-Z0-9_]{2,})\s*=\s*([^\n;]+)", src):
        consts[m.group(1)] = literal(m.group(2).strip())
    return consts


def snake(name: str) -> str:
    return re.sub(r"(?<=[a-z0-9])(?=[A-Z])", "_", name).lower()


# --------------------------------------------------------------------------
# areas


def route_area(path: str) -> str:
    p = path.lower()
    if "settings" in p:
        return "settings"
    if "kanban" in p:
        return "kanban"
    if "pull-requests" in p:
        return "pull-requests"
    if "automations" in p:
        return "automations"
    if "hubs" in p or "studio" in p:
        return "studio-hubs"
    if any(k in p for k in ("inbox", "tasks", "groups", "plugins")):
        return "inbox-tasks-groups-plugins"
    if "$threadid" in p:
        return "thread"
    if p.rstrip("/").endswith("_chat"):
        return "home-composer"
    return "chrome"


NAME_AREA_RULES = [
    # (substring of lowercased component name or path, area) — first match wins
    ("welcome", "onboarding"),
    ("onboarding", "onboarding"),
    ("tour", "onboarding"),
    ("announcement", "onboarding"),
    ("splash", "onboarding"),
    ("palette", "dialogs-menus-palette"),
    ("dialog", "dialogs-menus-palette"),
    ("sheet", "dialogs-menus-palette"),
    ("menu", "dialogs-menus-palette"),
    ("popover", "dialogs-menus-palette"),
    ("tooltip", "dialogs-menus-palette"),
    ("toast", "dialogs-menus-palette"),
    ("contextmenu", "dialogs-menus-palette"),
    ("settings", "settings"),
    ("terminal", "right-dock"),
    ("browser", "right-dock"),
    ("diff", "right-dock"),
    ("editor", "right-dock"),
    ("device", "right-dock"),
    ("computer", "right-dock"),
    ("pdf", "right-dock"),
    ("filepreview", "right-dock"),
    ("composer", "home-composer"),
    ("inlinechip", "home-composer"),
    ("linkchip", "home-composer"),
    ("kanban", "kanban"),
    ("pullrequest", "pull-requests"),
    ("automation", "automations"),
    ("hub", "studio-hubs"),
    ("studio", "studio-hubs"),
    ("inbox", "inbox-tasks-groups-plugins"),
    ("task", "inbox-tasks-groups-plugins"),
    ("group", "inbox-tasks-groups-plugins"),
    ("plugin", "inbox-tasks-groups-plugins"),
    ("rail", "rail-sidebar"),
    ("sidebar", "rail-sidebar"),
    ("usage", "rail-sidebar"),
    ("project", "rail-sidebar"),
    ("topstrip", "chrome"),
    ("windowcontrols", "chrome"),
    ("navigation", "chrome"),
    ("routesurface", "chrome"),
    ("routeinset", "chrome"),
    ("recentview", "chrome"),
    ("theme", "chrome"),
    ("space", "chrome"),
    ("tab", "chrome"),
    ("update", "chrome"),
    ("snooze", "inbox-tasks-groups-plugins"),
    ("branchtoolbar", "thread"),
    ("gitaction", "thread"),
    ("message", "thread"),
    ("chat", "thread"),
    ("thread", "thread"),
    ("markdown", "thread"),
    ("transcript", "thread"),
    ("plan", "thread"),
    ("approval", "thread"),
    ("worklog", "thread"),
    ("followup", "thread"),
    ("sidechat", "thread"),
    ("checkpoint", "thread"),
    ("review", "pull-requests"),
]

DIR_AREA = {
    "settings": "settings",
    "kanban": "kanban",
    "pullrequest": "pull-requests",
    "automation": "automations",
    "tasks": "inbox-tasks-groups-plugins",
    "githubinbox": "inbox-tasks-groups-plugins",
    "inbox": "inbox-tasks-groups-plugins",
    "terminal": "right-dock",
    "browser": "right-dock",
    "computer": "right-dock",
    "device": "right-dock",
    "codeeditor": "right-dock",
    "pdf": "right-dock",
    "chat": "thread",
    "chat-drop-overlay": "thread",
    "group": "inbox-tasks-groups-plugins",
    "composer-nodes": "home-composer",
    "profile": "settings",
    "ui": "other",
}


def component_area(rel_path: str, name: str) -> str:
    r = rel_path.lower()
    parts = r.split("/")
    # deepest matching directory wins, else fall back to name keywords
    for d in reversed(parts[:-1]):
        if d in DIR_AREA:
            if d == "chat" and "composer" in name.lower():
                return "home-composer"
            return DIR_AREA[d]
    base = parts[-1]
    lowered = name.lower() + "|" + base
    for key, area in NAME_AREA_RULES:
        if key == "hub":
            if re.search(r"(?<!git)hub", lowered):
                return area
            continue
        if key == "studio":
            if re.search(r"(?<!visual)studio", lowered):
                return area
            continue
        if key in lowered:
            return area
    return "other"


def setting_area() -> str:
    return "settings"


def keybinding_area(command: str) -> str:
    c = command.lower()
    if c.startswith(("terminal.", "browser.", "device.", "diff.", "editor.")):
        return "right-dock"
    if c.startswith(("sidebar.", "thread.jump", "chat.visible", "threadtab")):
        return "rail-sidebar"
    if c.startswith(("space.", "view.")):
        return "chrome"
    if c.startswith(("composer.", "chat.find", "sidechat.", "chat.split")):
        return "thread"
    if c.startswith(("chat.new", "model", "traitspicker", "modelpicker")):
        return "home-composer"
    if c.startswith("settings."):
        return "settings"
    if c.startswith("git."):
        return "pull-requests"
    if c.startswith("thread.copyid"):
        return "thread"
    if c.startswith("script."):
        return "other"
    if c.startswith(("search.", "navigation.")):
        return "dialogs-menus-palette"
    return "other"


# --------------------------------------------------------------------------
# extraction


def extract_routes() -> list[dict]:
    items = []
    for fn in sorted(os.listdir(ROUTES_DIR)):
        if fn.startswith("-") or ".test." in fn or not fn.endswith(".tsx"):
            continue
        path = os.path.join(ROUTES_DIR, fn)
        src = read(path)
        m = re.search(r'createFileRoute\(\s*"([^"]+)"', src)
        if m:
            route_path = m.group(1)
            ln = line_of(src, m.start())
        elif re.search(r"createRootRoute", src):
            route_path = "/"
            ln = line_of(src, re.search(r"createRootRoute", src).start())
        else:
            continue
        items.append(
            {
                "id": f"route:{route_path}",
                "kind": "route",
                "upstream_ref": f"{rel(path)}:{ln}",
                "area": route_area(route_path),
                "path": route_path,
                "file": rel(path),
            }
        )
    return items


COMPONENT_EXPORT_RES = [
    re.compile(r"export\s+(?:default\s+)?function\s+([A-Z][A-Za-z0-9_]*)"),
    re.compile(r"export\s+class\s+([A-Z][A-Za-z0-9_]*)\s+extends"),
    re.compile(
        r"export\s+const\s+([A-Z][A-Za-z0-9_]*)\s*(?::[^=\n]+)?=\s*"
        r"(?:React\.|memo\b|forwardRef\b|lazy\b|function\b|async\b|\(|[A-Za-z_$][\w$]*\s*=>)"
    ),
]
NON_COMPONENT_SUFFIX = (
    "Props",
    "State",
    "Context",
    "Config",
    "Options",
    "Handle",
    "Schema",
    "Error",
    "Result",
    "Args",
)


def exported_components(src: str, stem: str) -> list[tuple[str, int]]:
    names: list[tuple[str, int]] = []
    seen = set()
    for rx in COMPONENT_EXPORT_RES:
        for m in rx.finditer(src):
            name = m.group(1)
            if name in seen or name.endswith(NON_COMPONENT_SUFFIX):
                continue
            seen.add(name)
            names.append((name, line_of(src, m.start())))
    # export { A, B } blocks (not "export type {" and not re-exports "from")
    for m in re.finditer(r"export\s*\{([^}]*)\}(?!\s*from)", src):
        if re.match(r"export\s*\{\s*type\b", m.group(0)):
            continue
        for part in m.group(1).split(","):
            name = part.strip().split(" as ")[-1].strip()
            if (
                re.fullmatch(r"[A-Z][A-Za-z0-9]*", name or "")
                and "_" not in name
                and not name.endswith(NON_COMPONENT_SUFFIX)
                and name not in seen
            ):
                # confirm it's a component-ish value defined in this file
                if re.search(
                    rf"(?:function|const|class)\s+{re.escape(name)}\b", src
                ):
                    seen.add(name)
                    names.append((name, line_of(src, m.start())))
    if not names:
        # export default <Name>; or unnamed default
        m = re.search(r"export\s+default\s+([A-Z][A-Za-z0-9_]*)\b", src)
        if m:
            names.append((m.group(1), line_of(src, m.start())))
        else:
            names.append((stem, 1))
    return names


def extract_components() -> list[dict]:
    items = []
    used_ids = set()
    for root, dirs, fns in os.walk(COMPONENTS_DIR):
        dirs.sort()
        for fn in sorted(fns):
            if (
                not fn.endswith(".tsx")
                or ".test." in fn
                or ".browser." in fn
                or ".stories." in fn
            ):
                continue
            path = os.path.join(root, fn)
            src = read(path)
            stem = fn[:-4]
            rel_p = rel(path)
            dir_rel = os.path.relpath(root, COMPONENTS_DIR)
            dir_rel = "" if dir_rel == "." else dir_rel
            for name, ln in exported_components(src, stem):
                item_id = f"component:{dir_rel + '/' if dir_rel else ''}{name}"
                if item_id in used_ids:
                    item_id = f"{item_id}#{ln}"
                used_ids.add(item_id)
                items.append(
                    {
                        "id": item_id,
                        "kind": "component",
                        "upstream_ref": f"{rel_p}:{ln}",
                        "area": component_area(rel_p, name),
                        "name": name,
                        "file": rel_p,
                    }
                )
    return items


def extract_setting_rows() -> list[dict]:
    src = read(os.path.join(WEB, "settingsSearchIndex.ts"))
    items = []
    for m in re.finditer(
        r'id:\s*"([^"]+)",\s*section:\s*"([^"]+)",\s*title:\s*"([^"]+)"', src
    ):
        entry_id, section, title = m.groups()
        items.append(
            {
                "id": f"setting:row:{entry_id}",
                "kind": "setting",
                "upstream_ref": f"apps/web/src/settingsSearchIndex.ts:{line_of(src, m.start())}",
                "area": "settings",
                "key": entry_id,
                "section": section,
                "label": title,
                "default": None,
                "source": "search-index",
            }
        )
    return items


def extract_app_schema_settings() -> list[dict]:
    path = os.path.join(WEB, "appSettings.ts")
    src = read(path)
    consts = collect_consts(src)
    # resolve shared defaults defined elsewhere
    consts.update(
        {
            "DEFAULT_CHAT_FONT_SIZE_PX": 13,
            "DEFAULT_TERMINAL_FONT_SIZE_PX": 12,
            "DEFAULT_TERMINAL_FONT_FAMILY": "",
            "DEFAULT_UI_DENSITY": literal(
                re.search(
                    r"UI_DENSITY_MODES\s*=\s*\[[^\]]*\]",
                    read(os.path.join(WEB, "lib/appDensity.ts")),
                ).group(0)
            )
            if os.path.exists(os.path.join(WEB, "lib/appDensity.ts"))
            else "DEFAULT_UI_DENSITY",
        }
    )
    start = src.index("export const AppSettingsSchema = Schema.Struct(")
    block, _ = block_from(src, start)
    items = []
    for key, off, val in top_level_fields(block):
        d = resolve_const(literal(default_expr(val)), consts)
        ln = line_of(src, src.index(key, src.index("AppSettingsSchema")) + off)
        items.append(
            {
                "id": f"setting:app:{key}",
                "kind": "setting",
                "upstream_ref": f"apps/web/src/appSettings.ts:{ln}",
                "area": "settings",
                "key": key,
                "section": "app",
                "label": re.sub(r"(?<=[a-z0-9])(?=[A-Z])", " ", key),
                "default": d,
                "source": "app-schema",
            }
        )
    return items


def extract_server_schema_settings() -> list[dict]:
    path = os.path.join(CONTRACTS, "settings.ts")
    src = read(path)

    def struct_fields(decl_name: str):
        marker = f"export const {decl_name} = Schema.Struct("
        i = src.index(marker)
        block, _ = block_from(src, i)
        return top_level_fields(block), i

    provider_structs = {}
    for m in re.finditer(r"export const (\w+ServerProviderSettings) = Schema\.Struct\(", src):
        fields, off = struct_fields(m.group(1))
        provider_structs[m.group(1)] = (fields, off)

    # provider base defaults
    base_defaults = {}
    m = re.search(r"const ProviderSettingsBase = (\{)", src)
    if m:
        block, _ = block_from(src, m.start(1) - 1)
        for key, _, val in top_level_fields(block):
            base_defaults[key] = default_expr(val)

    items = []

    def emit(key_path, val, base_off, label=None):
        if "...ProviderSettingsBase" in val or val.strip().startswith("..."):
            pass
        d = default_expr(val)
        ln = line_of(src, base_off)
        items.append(
            {
                "id": f"setting:server:{key_path}",
                "kind": "setting",
                "upstream_ref": f"packages/contracts/src/settings.ts:{ln}",
                "area": "settings",
                "key": key_path,
                "section": "server",
                "label": label or key_path.split(".")[-1],
                "default": literal(d),
                "source": "server-schema",
            }
        )

    fields, struct_off = struct_fields("ServerSettings")
    for key, off, val in fields:
        if key == "providers":
            pblock, _ = block_from(val, 0)
            for pname, poff, pval in top_level_fields(pblock):
                m = re.match(r"(\w+ServerProviderSettings)", pval.strip())
                if not m:
                    continue
                pfields, pstruct_off = provider_structs[m.group(1)]
                # expand ProviderSettingsBase spread into real keys; explicit
                # struct fields override base fields of the same name
                explicit = {fk for fk, _, _ in pfields if not fk.startswith("...")}
                expanded = []
                for fk, foff, fval in pfields:
                    if fk == "...ProviderSettingsBase":
                        for bk, bval in base_defaults.items():
                            if bk not in explicit:
                                expanded.append((bk, foff, f"__base__:{bval or ''}"))
                    else:
                        expanded.append((fk, foff, fval))
                for fk, foff, fval in expanded:
                    if fval.startswith("__base__:"):
                        default = literal(fval[len("__base__:") :] or None)
                    else:
                        default = literal(default_expr(fval))
                    items.append(
                        {
                            "id": f"setting:server:providers.{pname}.{fk}",
                            "kind": "setting",
                            "upstream_ref": f"packages/contracts/src/settings.ts:{line_of(src, pstruct_off + foff)}",
                            "area": "settings",
                            "key": f"providers.{pname}.{fk}",
                            "section": "providers",
                            "label": f"{pname} {fk}",
                            "default": default,
                            "source": "server-schema",
                        }
                    )
        elif key == "skills":
            sfields, soff = struct_fields("SkillsServerSettings")
            for fk, foff, fval in sfields:
                items.append(
                    {
                        "id": f"setting:server:skills.{fk}",
                        "kind": "setting",
                        "upstream_ref": f"packages/contracts/src/settings.ts:{line_of(src, soff + foff)}",
                        "area": "settings",
                        "key": f"skills.{fk}",
                        "section": "skills",
                        "label": f"skills {fk}",
                        "default": literal(default_expr(fval)),
                        "source": "server-schema",
                    }
                )
        else:
            emit(key, val, struct_off + off)
    return items


def extract_keybindings() -> list[dict]:
    items = []
    kpath = os.path.join(CONTRACTS, "keybindings.ts")
    ksrc = read(kpath)
    m = re.search(r"STATIC_KEYBINDING_COMMANDS\s*=\s*\[", ksrc)
    block, _ = block_from(ksrc, m.start(), "[", "]")
    commands = re.findall(r'"([^"]+)"', block)

    # defaults from the server table
    server_defaults: dict[str, list[dict]] = {c: [] for c in commands}
    spath = os.path.join(SERVER, "keybindings.ts")
    ssrc = read(spath)
    for bm in re.finditer(
        r"\{[^{}]*?key:\s*\"([^\"]+)\"[^{}]*?command:\s*\"([^\"]+)\"[^{}]*?\}", ssrc, re.S
    ):
        inner = bm.group(0)
        when = re.search(r'when:\s*"([^"]+)"', inner)
        entry = {"key": bm.group(1), "source": "server-default"}
        if when:
            entry["when"] = when.group(1)
        server_defaults.setdefault(bm.group(2), []).append(entry)

    # web fallback table
    wsrc = read(os.path.join(WEB, "keybindings.ts"))
    fb_start = wsrc.index("DEFAULT_SHORTCUT_FALLBACKS")
    fblock, _ = block_from(wsrc, wsrc.index("[", fb_start), "[", "]")

    def parse_shortcut_call(text):
        sm = re.search(r'commandShortcut\("([^"]+)"\s*(?:,\s*\{([^}]*)\})?\)', text)
        if not sm:
            return None
        key = sm.group(1)
        flags = {"modKey": True}
        if sm.group(2):
            for fm in re.finditer(r"(\w+):\s*(true|false)", sm.group(2)):
                flags[fm.group(1)] = fm.group(2) == "true"
        parts = []
        if flags.get("modKey"):
            parts.append("mod")
        if flags.get("metaKey"):
            parts.append("meta")
        if flags.get("ctrlKey"):
            parts.append("ctrl")
        if flags.get("shiftKey"):
            parts.append("shift")
        if flags.get("altKey"):
            parts.append("alt")
        parts.append(key)
        return "+".join(parts)

    web_defaults: dict[str, list[dict]] = {}
    for em in re.finditer(
        r'command:\s*"([^"]+)",\s*shortcut:\s*(commandShortcut\([^)]*\)(?:\s*))?(?:,\s*whenAst:\s*(.*?))?\n\s*\}',
        fblock,
        re.S,
    ):
        command, sc_call, when = em.group(1), em.group(2), em.group(3)
        key = parse_shortcut_call(sc_call or "") if sc_call else None
        entry = {"key": key, "source": "web-fallback"}
        if when:
            entry["when"] = " ".join(when.split())[:120]
        web_defaults.setdefault(command, []).append(entry)
    # ...SPACE_JUMP_KEYBINDING_COMMANDS.map(...) spread -> mod+alt+N
    if "SPACE_JUMP_KEYBINDING_COMMANDS.map" in fblock:
        for i in range(1, 10):
            web_defaults.setdefault(f"space.jump.{i}", []).append(
                {"key": f"mod+alt+{i}", "source": "web-fallback", "when": "whenModChordAllowed"}
            )

    for c in commands:
        keys = server_defaults.get(c, []) + [
            e
            for e in web_defaults.get(c, [])
            if e["key"] and all(e["key"] != d["key"] for d in server_defaults.get(c, []))
        ]
        items.append(
            {
                "id": f"keybinding:{c}",
                "kind": "keybinding",
                "upstream_ref": f"packages/contracts/src/keybindings.ts:{line_of(ksrc, ksrc.index(chr(34) + c + chr(34), m.start()))}",
                "area": keybinding_area(c),
                "command": c,
                "keys": keys,
            }
        )
    # script.*.run pattern (KEYBINDINGS.md + SCRIPT_RUN_COMMAND_PATTERN)
    items.append(
        {
            "id": "keybinding:script.*.run",
            "kind": "keybinding",
            "upstream_ref": f"packages/contracts/src/keybindings.ts:{line_of(ksrc, ksrc.index('SCRIPT_RUN_COMMAND_PATTERN'))}",
            "area": "other",
            "command": "script.<id>.run",
            "keys": [],
        }
    )

    # fixed chords
    fsrc = read(os.path.join(WEB, "fixedShortcuts.ts"))
    fs_start = fsrc.index("FIXED_SHORTCUTS")
    fsblock, _ = block_from(fsrc, fsrc.index("= [", fs_start) + 1, "[", "]")
    # walk balanced top-level { ... } entries inside the array
    body = fsblock[1:-1]
    i = 0
    while i < len(body):
        if body[i] == "{":
            entry_start = i
            depth = 0
            j = i
            in_str = None
            while j < len(body):
                ch = body[j]
                if in_str:
                    if ch == "\\":
                        j += 1
                    elif ch == in_str:
                        in_str = None
                elif ch in ('"', "'", "`"):
                    in_str = ch
                elif ch == "{":
                    depth += 1
                elif ch == "}":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            inner = body[i + 1 : j]
            i = j + 1
        else:
            i += 1
            continue
        if "chord(" not in inner:
            continue
        idm = re.search(r'id:\s*"([^"]+)"', inner)
        chord_m = re.search(r'chord\("([^"]+)"\s*(?:,\s*(\w+|\{[^}]*\}))?\)', inner)
        reason = re.search(r'reason:\s*"([^"]+)"', inner)
        platform = re.search(r'platform:\s*"([^"]+)"', inner)
        key = chord_m.group(1) if chord_m else "?"
        flags_arg = chord_m.group(2) if chord_m else None
        flags = {"modKey": True}
        if flags_arg and flags_arg.startswith("{"):
            for fm in re.finditer(r"(\w+):\s*(true|false)", flags_arg):
                flags[fm.group(1)] = fm.group(2) == "true"
        elif flags_arg == "alt":
            flags = {"modKey": False, "altKey": True}
        parts = []
        if flags.get("modKey"):
            parts.append("mod")
        if flags.get("metaKey"):
            parts.append("meta")
        if flags.get("ctrlKey"):
            parts.append("ctrl")
        if flags.get("shiftKey"):
            parts.append("shift")
        if flags.get("altKey"):
            parts.append("alt")
        parts.append(key)
        key_str = "+".join(parts)
        base = idm.group(1) if idm else key_str
        suffix = f":{platform.group(1)}" if platform else ""
        items.append(
            {
                "id": f"keybinding:fixed:{base}{suffix}",
                "kind": "keybinding",
                "upstream_ref": f"apps/web/src/fixedShortcuts.ts:{line_of(fsrc, fs_start + 2 + entry_start)}",
                "area": keybinding_area(idm.group(1) if idm else ""),
                "command": idm.group(1) if idm else None,
                "keys": [
                    {
                        "key": key_str,
                        "source": "fixed",
                        "reason": reason.group(1) if reason else "",
                        **({"platform": platform.group(1)} if platform else {}),
                    }
                ],
            }
        )
    return items


def main() -> int:
    items = (
        extract_routes()
        + extract_components()
        + extract_setting_rows()
        + extract_app_schema_settings()
        + extract_server_schema_settings()
        + extract_keybindings()
    )
    items.sort(key=lambda i: i["id"])
    ids = [i["id"] for i in items]
    dupes = {i for i in ids if ids.count(i) > 1}
    if dupes:
        print(f"FATAL duplicate ids: {sorted(dupes)[:10]}", file=sys.stderr)
        return 1
    with open(OUT, "w", encoding="utf-8") as fh:
        json.dump(items, fh, indent=2, ensure_ascii=False)
        fh.write("\n")
    counts = {}
    for i in items:
        counts[i["kind"]] = counts.get(i["kind"], 0) + 1
    print(json.dumps({"total": len(items), **counts}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
