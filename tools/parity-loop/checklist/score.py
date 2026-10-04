#!/usr/bin/env python3
"""Score the parity checklist: status.json -> pass/fail.

Reads checklist/items.json (upstream items) and checklist/status.json
(per-item status + port_only list). Prints a per-area table and the overall
score, then exits:

  0  iff every item is "done" (or hand-marked "na") AND port_only is empty
  1  otherwise

Statuses: done | partial | missing | na (not applicable — hand-listed only).

Usage:
  python3 score.py [--json] [--status PATH]
"""
from __future__ import annotations

import argparse
import collections
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
STATUS = os.path.join(HERE, "status.json")
ITEMS = os.path.join(HERE, "items.json")

COUNTED = ("done", "partial", "missing")  # 'na' items leave the denominator


def load() -> tuple[list[dict], dict]:
    items = json.loads(open(ITEMS).read())
    status = json.loads(open(STATUS).read())
    return items, status


def area_of(item: dict) -> str:
    for key in ("area",):
        if item.get(key):
            return item[key]
    # status.json items carry no area; fall back to the id prefix kind.
    return item.get("id", "?").split(":", 1)[0]


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--json", action="store_true")
    ap.add_argument("--status", default=STATUS)
    args = ap.parse_args()

    items, status = load() if args.status == STATUS else (
        json.loads(open(ITEMS).read()), json.loads(open(args.status).read()))
    by_id = {i["id"]: i for i in items}
    rows = status.get("items", [])
    port_only = status.get("port_only", [])

    areas: dict[str, collections.Counter] = collections.defaultdict(collections.Counter)
    counted = 0
    done = 0
    unknown = []
    for row in rows:
        area = area_of(by_id.get(row["id"], row))
        st = row.get("status", "missing")
        areas[area][st] += 1
        if st == "done":
            done += 1
            counted += 1
        elif st == "na":
            pass
        else:
            counted += 1
            if st not in COUNTED:
                unknown.append((row["id"], st))

    score = (done / counted * 100) if counted else 0.0
    ok = (done == counted) and not port_only and not unknown

    report = {
        "score_pct": round(score, 2),
        "done": done,
        "counted": counted,
        "port_only": len(port_only),
        "unknown_status": len(unknown),
        "pass": ok,
        "areas": {a: dict(c) for a, c in sorted(areas.items())},
    }
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        w = max((len(a) for a in areas), default=4)
        for a in sorted(areas):
            c = areas[a]
            total = sum(c.values())
            print(f"{a:<{w}}  done {c.get('done', 0):>4}/{total:<4} "
                  f"partial {c.get('partial', 0):<4} missing {c.get('missing', 0):<4} "
                  f"na {c.get('na', 0):<3}")
        print("-" * (w + 44))
        print(f"{'TOTAL':<{w}}  done {done:>4}/{counted:<4} score {score:.1f}%  "
              f"port-only {len(port_only)}")
        if unknown:
            print("unknown statuses:", unknown[:10])
        print("PASS" if ok else "FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
