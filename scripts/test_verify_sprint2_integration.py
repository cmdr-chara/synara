"""Regression coverage for task-preserving, status-aware roadmap comparison."""
import unittest
from verify_sprint2_integration import historical_ledger

BASE = """# Roadmap
## A. Recover original intent

Status: Partial

- [ ] A1 Preserve the original task.
  Its continuation is also protected.

Historical evidence: original proof.

## B. Runtime

Status: Open

- [x] B1 Keep completion evidence.

## Verification contract
Current execution queue may change.
"""


class HistoricalLedgerTests(unittest.TestCase):
    def test_current_lane_status_can_advance(self):
        changed = BASE.replace("Status: Partial", "Status: Present").replace("Status: Open", "Status: Partial")
        self.assertEqual(historical_ledger(BASE), historical_ledger(changed))

    def test_task_body_and_continuation_remain_protected(self):
        for old, new in [("original task", "different task"), ("continuation", "replacement")]:
            with self.subTest(old=old):
                self.assertNotEqual(historical_ledger(BASE), historical_ledger(BASE.replace(old, new)))

    def test_checkbox_and_history_remain_protected(self):
        for old, new in [("[x] B1", "[ ] B1"), ("original proof", "new claim")]:
            with self.subTest(old=old):
                self.assertNotEqual(historical_ledger(BASE), historical_ledger(BASE.replace(old, new)))

    def test_status_word_in_task_evidence_is_not_excluded(self):
        with_evidence = BASE.replace("Historical evidence:", "Status: historical observation\nHistorical evidence:")
        changed = with_evidence.replace("historical observation", "changed observation")
        self.assertNotEqual(historical_ledger(with_evidence), historical_ledger(changed))

    def test_current_queue_is_outside_the_historical_ledger(self):
        self.assertEqual(historical_ledger(BASE), historical_ledger(BASE.replace("Current execution queue", "New execution queue")))

    def test_missing_boundaries_fail_closed(self):
        for text in [BASE.replace("## A. Recover", "## A. Other"), BASE.replace("## Verification contract", "## Other")]:
            with self.subTest(text=text):
                with self.assertRaises(ValueError):
                    historical_ledger(text)


if __name__ == "__main__":
    unittest.main()
