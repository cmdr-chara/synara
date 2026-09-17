#!/usr/bin/env python3
"""Read-only regression guard for the isolated A/J/M session's ownership."""
from __future__ import annotations

import argparse
import difflib
from pathlib import Path
import re
import subprocess
import sys
import unittest

COMMON_BASE = "1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121"
OWNED_HEADINGS = frozenset({
    "## A. Recover and complete the native terminal",
    "## J. SSH and remote development",
    "## M. Application runtime and service boundaries",
})
PROTECTED_PREFIXES = (
    "crates/synara-agent/",
    "crates/synara-acp/",
    "crates/synara-core/",
    "crates/synara-registry/",
    "crates/synara-app/src/shell/conversation",
)
PROTECTED_FILES = frozenset({
    "docs/agent-compatibility.md",
    ".github/workflows/vendor-probe.yml",
    "crates/synara-app/src/shell/registry.rs",
})


def unowned_roadmap(text: str) -> str:
    """Retain all bytes outside the three uniquely named owned sections."""
    blocks = re.split(r"(?m)(?=^## )", text)
    headings = [block.partition("\n")[0] for block in blocks]
    for heading in OWNED_HEADINGS:
        if headings.count(heading) != 1:
            raise ValueError(f"expected exactly one roadmap section: {heading}")
    return "".join(
        block for block, heading in zip(blocks, headings)
        if heading not in OWNED_HEADINGS
    )


def protected_changes(paths: list[str]) -> list[str]:
    return sorted(
        path for path in paths
        if path in PROTECTED_FILES or path.startswith(PROTECTED_PREFIXES)
    )


def git(root: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), *args], text=True, encoding="utf-8"
    )


def verify(root: Path, base: str) -> None:
    if re.fullmatch(r"[0-9a-f]{40}", base) is None:
        raise ValueError("base must be a full lowercase commit SHA")
    if git(root, "merge-base", base, "HEAD").strip() != base:
        raise ValueError("the common base is not an ancestor of this candidate")
    paths = git(root, "diff", "--name-only", "-z", base, "HEAD").split("\0")
    violations = protected_changes([path for path in paths if path])
    if violations:
        raise ValueError("changes outside A/J/M ownership: " + ", ".join(violations))
    before = unowned_roadmap(git(root, "show", f"{base}:ROADMAP.md"))
    after = unowned_roadmap(git(root, "show", "HEAD:ROADMAP.md"))
    if before != after:
        diff = "".join(difflib.unified_diff(
            before.splitlines(keepends=True), after.splitlines(keepends=True),
            fromfile="common-base/unowned-roadmap", tofile="candidate/unowned-roadmap",
        ))
        raise ValueError("roadmap edits outside A/J/M ownership:\n" + diff)
    print("AJM scope verified: protected source and other roadmap sections unchanged")


class OwnershipTests(unittest.TestCase):
    def setUp(self) -> None:
        self.text = (
            "# Roadmap\n\nIntro\n\n"
            "## A. Recover and complete the native terminal\n\nA pending\n\n"
            "## B. Agent lifecycle\n\nB pending\n\n"
            "## J. SSH and remote development\n\nJ pending\n\n"
            "## M. Application runtime and service boundaries\n\nM pending\n\n"
            "## Q. Integration\n\nQ pending\n"
        )

    def test_owned_sections_can_change(self) -> None:
        changed = self.text.replace("A pending", "A evidence").replace(
            "J pending", "J evidence"
        ).replace("M pending", "M evidence")
        self.assertEqual(unowned_roadmap(self.text), unowned_roadmap(changed))

    def test_other_sections_cannot_change(self) -> None:
        for old in ("B pending", "Q pending", "Intro"):
            with self.subTest(old=old):
                self.assertNotEqual(
                    unowned_roadmap(self.text),
                    unowned_roadmap(self.text.replace(old, "changed")),
                )

    def test_duplicate_owned_heading_fails_closed(self) -> None:
        with self.assertRaises(ValueError):
            unowned_roadmap(self.text + "## J. SSH and remote development\n")

    def test_missing_owned_heading_fails_closed(self) -> None:
        with self.assertRaises(ValueError):
            unowned_roadmap(self.text.replace("## J. SSH and remote development", "## J. Renamed"))

    def test_third_level_heading_is_owned_content(self) -> None:
        changed = self.text.replace("A pending", "A pending\n### Evidence\nDetails")
        self.assertEqual(unowned_roadmap(self.text), unowned_roadmap(changed))

    def test_protected_files_and_directories(self) -> None:
        paths = [
            "crates/synara-acp/src/lib.rs", "crates/synara-agent/src/api.rs",
            "crates/synara-app/src/shell/conversation.rs",
            "docs/agent-compatibility.md", ".github/workflows/vendor-probe.yml",
        ]
        self.assertEqual(protected_changes(paths), sorted(paths))

    def test_runtime_and_narrow_shared_paths_are_not_rejected(self) -> None:
        self.assertEqual(protected_changes([
            "crates/synara-runtime/src/terminal.rs",
            "crates/synara-workspace/src/remote.rs",
            "crates/synara-app/src/shell.rs",
            "crates/synara-app/src/shell/panels.rs",
            ".github/workflows/ssh.yml", "ROADMAP.md", "Cargo.lock",
        ]), [])


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--base", default=COMMON_BASE)
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()
    if args.self_test:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(OwnershipTests)
        return 0 if unittest.TextTestRunner(verbosity=2).run(suite).wasSuccessful() else 1
    try:
        verify(Path(__file__).resolve().parents[3], args.base)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"AJM scope check failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
