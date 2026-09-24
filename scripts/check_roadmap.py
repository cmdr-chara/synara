#!/usr/bin/env python3
"""Validate the concise execution roadmap and its count-down invariants."""
from __future__ import annotations

from collections import Counter
from pathlib import Path
import re
import sys
import unittest

ITEM = re.compile(r"^- \[([^\]])\] ([MSA])(\d{2}) (.+)$", re.MULTILINE)
SUMMARY = {
    "shipped": re.compile(r"^- Shipped feature slices: \*\*(\d+)\*\*$", re.MULTILINE),
    "major": re.compile(r"^- Major remaining: \*\*(\d+)\*\*$", re.MULTILINE),
    "smaller": re.compile(r"^- Smaller remaining: \*\*(\d+)\*\*$", re.MULTILINE),
    "acceptance": re.compile(
        r"^- Acceptance/integration remaining: \*\*(\d+)\*\*$", re.MULTILINE
    ),
    "total": re.compile(r"^- Total remaining: \*\*(\d+)\*\*$", re.MULTILINE),
}
EXPECTED_IDS = {"M": 30, "S": 18, "A": 10}
BASELINE_SHIPPED = 40
REQUIRED_SECTIONS = (
    "## Current status",
    "## Major features remaining",
    "## Smaller features remaining",
    "## Acceptance/integration remaining",
    "## Next execution queue",
    "## Shipped",
    "## How to update this roadmap",
)


def _summary_value(name: str, roadmap: str, errors: list[str]) -> int | None:
    match = SUMMARY[name].search(roadmap)
    if not match:
        errors.append(f"Missing summary count: {name}")
        return None
    return int(match.group(1))


def inspect(roadmap: str, readme: str) -> tuple[list[str], dict[str, int]]:
    errors: list[str] = []
    items = list(ITEM.finditer(roadmap))
    identifiers = [f"{item.group(2)}{item.group(3)}" for item in items]

    for identifier, count in Counter(identifiers).items():
        if count != 1:
            errors.append(f"Duplicate roadmap item: {identifier}")

    for item in items:
        if item.group(1) not in (" ", "x"):
            errors.append(f"Invalid checkbox state: {item.group(0)}")

    for prefix, expected_count in EXPECTED_IDS.items():
        numbers = [
            int(item.group(3))
            for item in items
            if item.group(2) == prefix
        ]
        if numbers != list(range(1, expected_count + 1)):
            errors.append(
                f"{prefix} items must appear exactly once and consecutively "
                f"from 01 to {expected_count:02d}"
            )

    remaining = {
        prefix: sum(
            item.group(2) == prefix and item.group(1) == " "
            for item in items
        )
        for prefix in EXPECTED_IDS
    }
    checked = sum(item.group(1) == "x" for item in items)

    shipped = _summary_value("shipped", roadmap, errors)
    major = _summary_value("major", roadmap, errors)
    smaller = _summary_value("smaller", roadmap, errors)
    acceptance = _summary_value("acceptance", roadmap, errors)
    total = _summary_value("total", roadmap, errors)

    if major is not None and major != remaining["M"]:
        errors.append("Major remaining count does not match M checkboxes")
    if smaller is not None and smaller != remaining["S"]:
        errors.append("Smaller remaining count does not match S checkboxes")
    if acceptance is not None and acceptance != remaining["A"]:
        errors.append("Acceptance remaining count does not match A checkboxes")

    expected_remaining = sum(remaining.values())
    if total is not None and total != expected_remaining:
        errors.append("Total remaining does not match unchecked roadmap items")
    if shipped is not None and shipped != BASELINE_SHIPPED + checked:
        errors.append("Shipped count must equal baseline 40 plus checked roadmap items")

    for section in REQUIRED_SECTIONS:
        if section not in roadmap:
            errors.append(f"Missing roadmap section: {section}")

    if "docs/history/roadmap-before-simplification-2026-09-24.md" not in roadmap:
        errors.append("Roadmap must link the archived detailed ledger")
    if not re.search(r"\[[^\]]+\]\((?:\./)?ROADMAP\.md\)", readme):
        errors.append("README must link to ROADMAP.md")

    return errors, {
        "items": len(items),
        "checked": checked,
        "remaining": expected_remaining,
        "major_remaining": remaining["M"],
        "smaller_remaining": remaining["S"],
        "acceptance_remaining": remaining["A"],
    }


class RoadmapTests(unittest.TestCase):
    def setUp(self) -> None:
        self.readme = "[Delivery roadmap](ROADMAP.md)"
        sections = "\n".join(REQUIRED_SECTIONS)
        items = []
        for prefix, count in EXPECTED_IDS.items():
            items.extend(
                f"- [ ] {prefix}{number:02d} Example"
                for number in range(1, count + 1)
            )
        self.document = "\n".join(
            [
                sections,
                "- Shipped feature slices: **40**",
                "- Major remaining: **30**",
                "- Smaller remaining: **18**",
                "- Acceptance/integration remaining: **10**",
                "- Total remaining: **58**",
                "[archive](docs/history/roadmap-before-simplification-2026-09-24.md)",
                *items,
            ]
        )

    def test_valid_structure(self) -> None:
        errors, counts = inspect(self.document, self.readme)
        self.assertEqual(errors, [])
        self.assertEqual(counts["remaining"], 58)

    def test_completed_item_updates_counts(self) -> None:
        document = (
            self.document.replace("- [ ] M01", "- [x] M01", 1)
            .replace("Shipped feature slices: **40**", "Shipped feature slices: **41**")
            .replace("Major remaining: **30**", "Major remaining: **29**")
            .replace("Total remaining: **58**", "Total remaining: **57**")
        )
        self.assertEqual(inspect(document, self.readme)[0], [])

    def test_stale_summary_is_rejected(self) -> None:
        document = self.document.replace("- [ ] M01", "- [x] M01", 1)
        self.assertTrue(inspect(document, self.readme)[0])

    def test_duplicate_id_is_rejected(self) -> None:
        document = self.document.replace("M02 Example", "M01 Example", 1)
        self.assertTrue(inspect(document, self.readme)[0])

    def test_invalid_checkbox_is_rejected(self) -> None:
        document = self.document.replace("- [ ] M01", "- [?] M01", 1)
        self.assertTrue(inspect(document, self.readme)[0])

    def test_broken_readme_link_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document, "[Roadmap](missing.md)")[0])


def main() -> None:
    if sys.argv[1:] == ["--self-test"]:
        unittest.main(argv=[sys.argv[0]])
        return
    if sys.argv[1:]:
        raise SystemExit("Usage: check_roadmap.py [--self-test]")
    root = Path(__file__).resolve().parents[1]
    errors, counts = inspect(
        (root / "ROADMAP.md").read_text(encoding="utf-8"),
        (root / "README.md").read_text(encoding="utf-8"),
    )
    if errors:
        raise SystemExit("\n".join(errors))
    print(f"Roadmap structure PASS: {counts}")
    print("Remaining counts come from the explicit execution inventory, not broad gates.")


if __name__ == "__main__":
    main()
