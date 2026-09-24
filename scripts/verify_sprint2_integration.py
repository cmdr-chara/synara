#!/usr/bin/env python3
"""Read-only source/ledger checks for native conversation workflow integration."""
import argparse
import importlib.util
import json
import re
from pathlib import Path
import subprocess
import tempfile
import sys

sys.dont_write_bytecode = True

BASELINE = "fe515833c565d666204f24e5257d85025dede0e9"

def audit_module(root):
    spec = importlib.util.spec_from_file_location("workspace_audit", root / "scripts/audit_workspace.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

def historical_ledger(text):
    start = text.index("## A. Recover")
    span = text[start:text.index("## Verification contract", start)]
    # Current lane summaries may advance. Original task bodies, checkbox states,
    # headings and historical evidence must not change with those summaries.
    retained = []
    awaiting_status = False
    for line in span.splitlines(keepends=True):
        if re.fullmatch(r"## [A-Q]\. .+\n?", line):
            awaiting_status = True
        elif awaiting_status and line.startswith("Status: "):
            awaiting_status = False
            continue
        elif line.strip():
            awaiting_status = False
        retained.append(line)
    return "".join(retained)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    args.output.mkdir(parents=True, exist_ok=True)
    head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
    state = subprocess.check_output(["git", "status", "--porcelain"], cwd=root, text=True).strip()
    assert not state, f"Verification requires a clean committed candidate: {state}"
    subprocess.run(["git", "merge-base", "--is-ancestor", BASELINE, head], cwd=root, check=True)
    subprocess.run(["git", "diff", "--check", BASELINE, head], cwd=root, check=True)
    with tempfile.TemporaryDirectory(prefix="synara-ledger-baseline-") as temp:
        baseline = Path(temp) / "source"
        subprocess.run(["git", "worktree", "add", "--detach", str(baseline), BASELINE], cwd=root, check=True)
        try:
            old = audit_module(baseline).audit(baseline)
            current = audit_module(root).audit(root)
            assert old["errors"] == ["crates/synara-browser/src/native/actions.js: non-Rust core source"], old
            # Browser and web-workspace JavaScript belong to assets, outside
            # Rust crates. New workspace crates may be added while the source
            # boundary and root-history invariants continue to hold.
            assert current["errors"] == [], {"baseline": old, "current": current}
            assert current["check"] == old["check"] == "workspace-structure"
            assert current["root"] == old["root"]
            assert current["crates"] >= old["crates"]
            assert current["rust_compilation_performed"] is False
            # The short execution roadmap moved the original A-Q ledger to
            # docs/history; compare the archived text against the baseline.
            current_ledger = root / "ROADMAP.md"
            if "## A. Recover" not in current_ledger.read_text():
                current_ledger = root / "docs/history/roadmap-before-simplification-2026-09-24.md"
            assert historical_ledger((baseline / "ROADMAP.md").read_text()) == historical_ledger(current_ledger.read_text())
        finally:
            # Only remove the clean temporary worktree created above.
            subprocess.run(["git", "worktree", "remove", str(baseline)], cwd=root, check=True)
    assert not (root / ".synara-sprint-2.json").exists(), "Temporary candidate transport remains"
    assert not list(root.glob(".feature-closure-part-*")), "Competing publisher parts remain"
    assert not (root / "scripts/feature_closure_transfer.py").exists(), "Competing publisher remains"
    assert not (root / ".github/workflows/feature-closure-sprint.yml").exists(), "Write-enabled temporary workflow remains"
    workflow = (root / ".github/workflows/sprint-2.yml").read_text()
    assert "contents: read" in workflow and "contents: write" not in workflow
    assert "persist-credentials: false" in workflow and "git push" not in workflow
    report = {
        "candidate": head, "baseline": BASELINE, "status": "passed",
        "historical_A_Q_ledger": "task bodies, checkboxes and historical evidence byte-identical; current lane statuses excluded",
        "structural_baseline": old, "structural_candidate": current,
        "structural_regressions": [],
        "temporary_publisher": "retired; replacement CI is read-only",
        "note": "Historical Browser script-placement error is fixed; the current candidate must have no structural errors."
    }
    (args.output / "source-checks.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))

if __name__ == "__main__":
    main()
