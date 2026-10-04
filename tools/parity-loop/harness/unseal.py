#!/usr/bin/env python3
"""Map a blind verdict back to upstream/port and track round history for a piece.

  unseal.py <piece> <N>

Writes rounds/<piece>/r<N>/verdict-unsealed.md and appends to rounds/<piece>/history.json.
Prints the decision: PASS (port picked or indistinguishable), CONTINUE, or RESTART (worse than best,
or the same gap twice) with the best round to restart from.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
import lib  # noqa: E402


def side_map(blind: Path) -> dict:
    for kid in sorted(blind.glob("*/key-id.txt")):
        key = json.loads((lib.KEYS / f"{kid.read_text().strip()}.json").read_text())
        return {"A": key["A"], "B": key["B"]}
    raise SystemExit("no blind captures found")


def main() -> None:
    piece, n = sys.argv[1], int(sys.argv[2])
    rd = lib.ROOT / "rounds" / piece / f"r{n}"
    v = json.loads((rd / "verdict.json").read_text())
    m = side_map(rd / "blind")
    name = lambda x: {"same": "same"}.get(x, m.get(x, x))  # noqa: E731
    lines = [f"# Unsealed verdict: {piece} round {n}", "",
             f"Mapping for this round: A = {m['A']}, B = {m['B']}.", "",
             f"Overall better: {name(v['overall_better'])}. Distance: {v['distance']}/10.", "",
             f"Biggest gap (A/B terms, use the mapping): {v['biggest_gap']}", ""]
    for s in v["states"]:
        lines += [f"## {s['state']}: better = {name(s['better'])}", "", f"Biggest gap: {s['biggest_gap']}", ""]
        lines += [f"- {d}" for d in s.get("differences", [])] + [""]
    (rd / "verdict-unsealed.md").write_text("\n".join(lines))
    hist_p = lib.ROOT / "rounds" / piece / "history.json"
    hist = json.loads(hist_p.read_text()) if hist_p.exists() else []
    hist = [h for h in hist if h["round"] != n]
    port_won = name(v["overall_better"]) in ("port", "same")
    hist.append({"round": n, "distance": v["distance"], "winner": name(v["overall_better"]),
                 "gap": v["biggest_gap"], "port_won": port_won})
    hist.sort(key=lambda h: h["round"])
    hist_p.write_text(json.dumps(hist, indent=2))
    best = min(hist, key=lambda h: (h["distance"], -h["round"]))
    prev = [h for h in hist if h["round"] < n]
    decision = "CONTINUE"
    if port_won:
        decision = "PASS"
    elif prev and v["distance"] > min(h["distance"] for h in prev):
        decision = f"RESTART from r{best['round']} (got worse)"
    print(json.dumps({"piece": piece, "round": n, "mapping": m, "decision": decision,
                      "distance": v["distance"], "best_round": best["round"]}, indent=2))
    print("check gap repetition manually against history:")
    for h in hist:
        print(f"  r{h['round']} d={h['distance']} winner={h['winner']}: {h['gap'][:160]}")


if __name__ == "__main__":
    main()
