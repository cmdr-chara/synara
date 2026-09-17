#!/usr/bin/env python3
"""Check roadmap structure, not feature completion or release readiness."""
from __future__ import annotations

from collections import Counter
from pathlib import Path
import re
import sys
import unittest

LANES = tuple('ABCDEFGHIJKLMNOPQ')
HEADING = re.compile(r'^## ([A-Z])\. ([^\n]+)$', re.MULTILINE)
TASK = re.compile(r'^- \[([^\]])\] ([A-Z])([0-9]+) (.+)$', re.MULTILINE)


def inspect(roadmap: str, readme: str) -> tuple[list[str], dict[str, int]]:
    errors = []
    headings = list(HEADING.finditer(roadmap))
    letters = [match.group(1) for match in headings]
    if tuple(letters) != LANES:
        errors.append('Expected each delivery lane A-Q exactly once and in order')
    all_tasks = list(TASK.finditer(roadmap))
    identifiers = [f'{task.group(2)}{task.group(3)}' for task in all_tasks]
    for identifier, count in Counter(identifiers).items():
        if count != 1:
            errors.append(f'Duplicate task ID: {identifier}')
    for task in all_tasks:
        if task.group(1) not in (' ', 'x'):
            errors.append(f'Invalid checkbox state: {task.group(0)}')
    accounted = 0
    for index, heading in enumerate(headings):
        end = headings[index + 1].start() if index + 1 < len(headings) else len(roadmap)
        section = roadmap[heading.end():end]
        lane = heading.group(1)
        tasks = list(TASK.finditer(section))
        accounted += len(tasks)
        if 'Status:' not in section:
            errors.append(f'Lane {lane} needs an explicit status')
        if not tasks:
            errors.append(f'Lane {lane} has no actionable tasks')
        if any(task.group(2) != lane for task in tasks):
            errors.append(f'Lane {lane} contains another lane\'s task IDs')
        numbers = [int(task.group(3)) for task in tasks]
        if numbers != list(range(1, len(tasks) + 1)):
            errors.append(f'Lane {lane} task IDs must be consecutive from 1')
    if accounted != len(all_tasks):
        errors.append('Tasks outside their delivery lane are not allowed')
    milestones = re.findall(r'^\| (M[1-5]):', roadmap, re.MULTILINE)
    if milestones != ['M1', 'M2', 'M3', 'M4', 'M5']:
        errors.append('Expected ordered milestones M1-M5')
    if not re.search(r'\[[^\]]+\]\((?:\./)?ROADMAP\.md\)', readme):
        errors.append('README must link to ROADMAP.md')
    for section in ['## Verification contract', '### Coverage map', '## Immediate execution queue']:
        if section not in roadmap:
            errors.append(f'Missing roadmap section: {section}')
    return errors, {
        'lanes': len(headings),
        'tasks': len(all_tasks),
        'checked': sum(task.group(1) == 'x' for task in all_tasks),
    }


class RoadmapTests(unittest.TestCase):
    def setUp(self) -> None:
        milestones = '\n'.join(f'| M{i}: example | outcome |' for i in range(1, 6))
        lanes = '\n'.join(
            f'## {lane}. Example\nStatus: Open\n- [ ] {lane}1 Implement\n- [ ] {lane}2 Verify\n'
            for lane in LANES
        )
        self.document = milestones + '\n' + lanes + '\n'.join([
            '## Verification contract', '### Coverage map', '## Immediate execution queue',
        ])
        self.readme = '[Delivery roadmap](ROADMAP.md)'

    def test_valid_structure_does_not_claim_completion(self) -> None:
        errors, counts = inspect(self.document, self.readme)
        self.assertEqual(errors, [])
        self.assertEqual(counts, {'lanes': 17, 'tasks': 34, 'checked': 0})

    def test_missing_lane_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('## A.', '### A.'), self.readme)[0])

    def test_duplicate_task_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('A2 Verify', 'A1 Verify'), self.readme)[0])

    def test_task_in_wrong_lane_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('A2 Verify', 'Z2 Verify'), self.readme)[0])

    def test_gap_in_task_numbers_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('A2 Verify', 'A3 Verify'), self.readme)[0])

    def test_invalid_checkbox_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('[ ] A1', '[?] A1'), self.readme)[0])

    def test_broken_readme_link_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document, '[Roadmap](missing.md)')[0])

    def test_missing_status_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('Status: Open', '', 1), self.readme)[0])

    def test_missing_milestone_is_rejected(self) -> None:
        self.assertTrue(inspect(self.document.replace('| M3:', '| Future:'), self.readme)[0])


def main() -> None:
    if sys.argv[1:] == ['--self-test']:
        unittest.main(argv=[sys.argv[0]])
        return
    if sys.argv[1:]:
        raise SystemExit('Usage: check_roadmap.py [--self-test]')
    root = Path(__file__).resolve().parents[1]
    errors, counts = inspect(
        (root / 'ROADMAP.md').read_text(encoding='utf-8'),
        (root / 'README.md').read_text(encoding='utf-8'),
    )
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'Roadmap structure PASS: {counts}')
    print('Checkbox counts are not evidence of product completion or release readiness.')


if __name__ == '__main__':
    main()
