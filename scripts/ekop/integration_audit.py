#!/usr/bin/env python3
"""Read-only ancestry and changed-path audit for independently authored session branches."""
from __future__ import annotations

import argparse
import itertools
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import unittest

BASE = "1cd24dd6f5ac9571c1ea2b7329bcb5fc1a4ad121"
ROOT = "43b1fb89bf19dadc388d18008f9ceb21b8215716"
ROLES = {"bcd", "ajm", "fgh", "ekop"}
SHARED_FILES = {
    "Cargo.toml", "Cargo.lock", "ROADMAP.md", "README.md", "rust-toolchain.toml",
    "crates/synara-app/src/shell.rs", "crates/synara-app/src/main.rs",
    "crates/synara-app/src/input.rs", "crates/synara-app/src/close.rs",
    "crates/synara-app/src/shell/panels.rs", ".github/workflows/native.yml",
}
OWNED_PREFIXES = {
    "bcd": ("crates/synara-agent/", "crates/synara-acp/"),
    "ajm": ("crates/synara-runtime/",),
    "fgh": ("crates/synara-workspace/",),
    "ekop": ("crates/synara-registry/", "foundations/browser/", "scripts/ekop/"),
}
OWNED_FILES = {
    "bcd": {"crates/synara-app/src/shell/conversation.rs", "docs/agent-compatibility.md"},
    "ajm": {".github/workflows/ssh.yml", "scripts/ssh_smoke.py"},
    "fgh": set(),
    "ekop": {
        "crates/synara-app/src/shell/registry.rs", ".github/workflows/ekop.yml",
        "docs/parallel/session-ekop.md", "docs/parallel/integration.md",
        "docs/architecture/zen-synaric.md", "docs/architecture/browser-host.md",
        "docs/roadmap/zen-synaric.md", "docs/verification/ekop.md",
    },
}


class AuditError(ValueError):
    pass


def git(repo: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess:
    env = dict(os.environ, GIT_OPTIONAL_LOCKS="0", GIT_TERMINAL_PROMPT="0")
    try:
        return subprocess.run(
            ["git", "-C", str(repo), *args], check=check, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, timeout=30, env=env,
        )
    except (OSError, subprocess.SubprocessError) as exc:
        raise AuditError("Git audit command failed. No merge or ref update was attempted.") from exc


def resolve(repo: Path, ref: str) -> str:
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/-]{0,1023}", ref) or ".." in ref:
        raise AuditError("Invalid ref spelling")
    value = git(repo, "rev-parse", "--verify", "--end-of-options", ref + "^{commit}")
    sha = value.stdout.decode("ascii").strip()
    if not re.fullmatch(r"[a-f0-9]{40}", sha):
        raise AuditError("Expected an SHA-1 repository commit")
    return sha


def roots(repo: Path, sha: str) -> list[str]:
    return git(repo, "rev-list", "--max-parents=0", sha).stdout.decode("ascii").splitlines()


def owner(path: str) -> str:
    if path in SHARED_FILES or path.startswith("crates/synara-core/"):
        return "integration"
    for role in sorted(ROLES):
        if path in OWNED_FILES[role] or path.startswith(OWNED_PREFIXES[role]):
            return role
    return "review"


def changed_paths(repo: Path, base: str, head: str) -> list[str]:
    # No rename heuristic: both sides of a rename participate in overlap detection.
    data = git(repo, "diff", "--name-only", "--no-renames", "-z", base, head, "--").stdout
    return sorted(part.decode("utf-8", errors="surrogateescape") for part in data.split(b"\0") if part)


def audit(repo: Path, base_ref: str, expected_root: str, branches: dict[str, str]) -> dict:
    if not branches or not set(branches).issubset(ROLES):
        raise AuditError("Specify at least one distinct known session role")
    base = resolve(repo, base_ref)
    if roots(repo, base) != [expected_root]:
        raise AuditError("Common base does not have the expected independent root")
    sessions = {}
    # Freeze refs to exact SHAs before comparing any branch content.
    heads = {role: resolve(repo, ref) for role, ref in branches.items()}
    for role, head in sorted(heads.items()):
        result = git(repo, "merge-base", "--is-ancestor", base, head, check=False)
        if result.returncode != 0 or roots(repo, head) != [expected_root]:
            raise AuditError("Session does not descend exclusively from the approved common history: " + role)
        paths = changed_paths(repo, base, head)
        sessions[role] = {
            "ref": branches[role], "sha": head, "changed_paths": paths,
            "shared_edits": [p for p in paths if owner(p) == "integration"],
            "ownership_conflicts": [p for p in paths if owner(p) in ROLES and owner(p) != role],
            "unassigned_paths": [p for p in paths if owner(p) == "review"],
        }
    overlaps = []
    for left, right in itertools.combinations(sorted(sessions), 2):
        common = sorted(set(sessions[left]["changed_paths"]) & set(sessions[right]["changed_paths"]))
        if common:
            overlaps.append({"sessions": [left, right], "paths": common})
    dirty = bool(git(repo, "status", "--porcelain=v1", "-z").stdout)
    review = dirty or bool(overlaps) or any(
        row["shared_edits"] or row["ownership_conflicts"] or row["unassigned_paths"]
        for row in sessions.values()
    )
    return {
        "schema": 1, "base": base, "expected_root": expected_root,
        "sessions": sessions, "overlaps": overlaps, "worktree_dirty": dirty,
        "status": "review_required" if review else "ready_for_integration_review",
        "semantic_compatibility": "not_proven",
        "mutated_refs": False,
    }


class Tests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="synara-integration-test-")
        self.repo = Path(self.directory.name)
        self.env = dict(os.environ, GIT_CONFIG_NOSYSTEM="1", GIT_CONFIG_GLOBAL=os.devnull)
        self.run_git("init", "--template=", "-b", "base")
        self.commit("seed.txt", "seed")
        self.base = resolve(self.repo, "HEAD")

    def tearDown(self):
        self.directory.cleanup()

    def run_git(self, *args):
        return subprocess.run(
            ["git", "-C", str(self.repo), "-c", "user.name=Fixture", "-c",
             "user.email=fixture@example.invalid", "-c", "commit.gpgsign=false",
             "-c", "core.hooksPath=" + os.devnull, *args],
            check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            timeout=30, env=self.env,
        )

    def commit(self, path, content):
        target = self.repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content, encoding="utf-8")
        self.run_git("add", "--", path)
        self.run_git("commit", "-m", "fixture change")

    def branch(self, name, path):
        self.run_git("switch", "-c", name, self.base)
        self.commit(path, name)

    def test_disjoint_changes_are_only_ready_for_review_not_certified(self):
        self.branch("bcd", "crates/synara-acp/test.txt")
        self.branch("fgh", "crates/synara-workspace/test.txt")
        result = audit(self.repo, self.base, self.base, {"bcd": "bcd", "fgh": "fgh"})
        self.assertEqual(result["status"], "ready_for_integration_review")
        self.assertEqual(result["semantic_compatibility"], "not_proven")

    def test_shared_file_changes_and_overlap_require_review(self):
        self.branch("bcd", "ROADMAP.md")
        self.branch("fgh", "ROADMAP.md")
        result = audit(self.repo, self.base, self.base, {"bcd": "bcd", "fgh": "fgh"})
        self.assertEqual(result["overlaps"][0]["paths"], ["ROADMAP.md"])
        self.assertEqual(result["status"], "review_required")

    def test_foreign_owner_is_detected_without_textual_overlap(self):
        self.branch("bcd", "crates/synara-runtime/host.rs")
        result = audit(self.repo, self.base, self.base, {"bcd": "bcd"})
        self.assertTrue(result["sessions"]["bcd"]["ownership_conflicts"])

    def test_unknown_paths_are_not_silently_declared_owned(self):
        self.branch("ekop", "new-surface.rs")
        result = audit(self.repo, self.base, self.base, {"ekop": "ekop"})
        self.assertEqual(result["sessions"]["ekop"]["unassigned_paths"], ["new-surface.rs"])

    def test_ref_option_injection_is_rejected(self):
        for ref in ["--help", "HEAD;echo", "HEAD\n", "base..HEAD"]:
            with self.assertRaises(AuditError):
                resolve(self.repo, ref)

    def test_dirty_worktree_is_preserved(self):
        self.branch("ekop", "crates/synara-registry/test.txt")
        before = resolve(self.repo, "HEAD")
        dirty = self.repo / "uncommitted.txt"
        dirty.write_text("keep me", encoding="utf-8")
        result = audit(self.repo, self.base, self.base, {"ekop": "ekop"})
        self.assertTrue(result["worktree_dirty"])
        self.assertEqual(resolve(self.repo, "HEAD"), before)
        self.assertEqual(dirty.read_text(encoding="utf-8"), "keep me")

    def test_unrelated_root_is_rejected(self):
        self.run_git("checkout", "--orphan", "foreign")
        self.run_git("rm", "-rf", ".")
        self.commit("foreign.txt", "foreign")
        with self.assertRaises(AuditError):
            audit(self.repo, self.base, self.base, {"ekop": "foreign"})


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument("--repo", type=Path, default=Path.cwd())
    parser.add_argument("--base", default=BASE)
    parser.add_argument("--root", default=ROOT)
    parser.add_argument("--session", action="append", default=[], metavar="ROLE=REF")
    args = parser.parse_args()
    if args.self_test:
        result = unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Tests))
        return 0 if result.wasSuccessful() else 1
    branches = {}
    for value in args.session:
        role, separator, ref = value.partition("=")
        if not separator or role not in ROLES or role in branches:
            parser.error("Each --session must be a distinct known ROLE=REF")
        branches[role] = ref
    try:
        result = audit(args.repo, args.base, args.root, branches)
    except AuditError as error:
        parser.exit(2, str(error) + "\n")
    print(json.dumps(result, indent=2, ensure_ascii=True))
    return 1 if result["status"] == "review_required" else 0


if __name__ == "__main__":
    raise SystemExit(main())
