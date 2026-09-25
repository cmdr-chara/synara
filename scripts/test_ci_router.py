#!/usr/bin/env python3
"""Regression tests for Jev-driven Synara CI routing."""
import io
import json
import os
import sys
import tempfile
import unittest
import urllib.error
from unittest.mock import Mock

sys.path.insert(0, os.path.dirname(__file__))

import ci_router


def response(
    lane=ci_router.NO_FOCUSED_UI,
    lane_confidence=0.95,
    risk=ci_router.LOW_RISK,
    risk_confidence=0.90,
    backend=ci_router.BACKEND_SKIP,
    backend_confidence=0.95,
    native=ci_router.NATIVE_SKIP,
    native_confidence=0.95,
    ssh=ci_router.SSH_SKIP,
    ssh_confidence=0.95,
):
    return {
        "model": "jev-1.13.0",
        "results": [{
            "dimensions": {
                "verification_lane": {
                    "label": lane,
                    "confidence": lane_confidence,
                    "scores": {},
                },
                "regression_risk": {
                    "label": risk,
                    "confidence": risk_confidence,
                    "scores": {},
                },
                "backend_acceptance": {
                    "label": backend,
                    "confidence": backend_confidence,
                    "scores": {},
                },
                "organization_context_batch": {
                    "label": native,
                    "confidence": native_confidence,
                    "scores": {},
                },
                "ssh_transport": {
                    "label": ssh,
                    "confidence": ssh_confidence,
                    "scores": {},
                },
            }
        }],
    }


class RouterTests(unittest.TestCase):
    def test_docs_skip_all_expensive_lanes_without_classifier(self):
        classifier = Mock(side_effect=AssertionError("classifier should not run"))
        route = ci_router.select_route(
            ["ROADMAP.md", "docs/ui/parity-9488759.md"],
            context="unused",
            api_key=None,
            classifier=classifier,
        )
        self.assertEqual(route.backend_mode, "skip")
        self.assertFalse(route.native_batch)
        self.assertFalse(route.ssh)
        self.assertEqual(route.scope, "docs")
        classifier.assert_not_called()

    def test_known_presentation_change_can_skip_backend_matrix(self):
        classifier = Mock(return_value=response(
            lane=ci_router.NO_FOCUSED_UI,
            backend=ci_router.BACKEND_SKIP,
        ))
        route = ci_router.select_route(
            ["crates/synara-app/src/ui/menu.rs"],
            context="ctx",
            api_key="secret",
            classifier=classifier,
        )
        self.assertTrue(route.presentation)
        self.assertEqual(route.scope, "presentation")
        self.assertEqual(route.backend_mode, "skip")
        self.assertFalse(route.native_batch)
        self.assertFalse(route.ssh)
        classifier.assert_called_once_with("ctx", "secret")

    def test_unknown_backend_change_can_choose_linux_only(self):
        route = ci_router.semantic_route(
            ["crates/synara-agent/src/new_logic.rs"],
            response(backend=ci_router.BACKEND_LINUX),
        )
        self.assertEqual(route.backend_mode, "linux")
        self.assertFalse(route.presentation)

    def test_manifest_change_forces_full_cross_platform_backend(self):
        route = ci_router.semantic_route(
            ["Cargo.lock"],
            response(backend=ci_router.BACKEND_SKIP),
        )
        self.assertEqual(route.backend_mode, "full")

    def test_organization_anchor_forces_specialized_native_batch(self):
        route = ci_router.semantic_route(
            ["crates/synara-workspace/src/storage/task_context.rs"],
            response(native=ci_router.NATIVE_SKIP),
        )
        self.assertTrue(route.native_batch)

    def test_ssh_fixture_change_forces_ssh_lane(self):
        route = ci_router.semantic_route(
            ["scripts/ssh_smoke.py"],
            response(ssh=ci_router.SSH_SKIP),
        )
        self.assertTrue(route.ssh)

    def test_remote_native_ssh_smoke_change_forces_ssh_lane(self):
        route = ci_router.semantic_route(
            ["scripts/remote_native_smoke.py"],
            response(ssh=ci_router.SSH_SKIP),
        )
        self.assertTrue(route.ssh)

    def test_low_confidence_backend_decision_fails_closed(self):
        route = ci_router.semantic_route(
            ["crates/synara-agent/src/new_logic.rs"],
            response(
                backend=ci_router.BACKEND_SKIP,
                backend_confidence=0.40,
                ssh=ci_router.SSH_SKIP,
                ssh_confidence=0.40,
            ),
        )
        self.assertEqual(route.backend_mode, "full")

    def test_classifier_failure_uses_conservative_path_fallbacks(self):
        def broken(_context, _key):
            raise urllib.error.URLError("offline")

        ui = ci_router.select_route(
            ["crates/synara-app/src/ui/menu.rs"],
            context="ctx",
            api_key=None,
            classifier=broken,
        )
        backend = ci_router.select_route(
            ["crates/synara-runtime/src/process.rs"],
            context="ctx",
            api_key=None,
            classifier=broken,
        )
        self.assertTrue(ui.presentation)
        self.assertEqual(ui.backend_mode, "skip")
        self.assertFalse(ui.ssh)
        self.assertEqual(backend.backend_mode, "full")
        self.assertTrue(backend.ssh)

    def test_high_risk_code_forces_full_backend(self):
        route = ci_router.semantic_route(
            ["crates/synara-agent/src/new_logic.rs"],
            response(
                risk=ci_router.HIGH_RISK,
                risk_confidence=0.96,
                backend=ci_router.BACKEND_SKIP,
            ),
        )
        self.assertEqual(route.backend_mode, "full")

    def test_request_uses_bearer_key_and_all_routing_dimensions(self):
        captured = {}

        class FakeResponse:
            def __enter__(self):
                return io.BytesIO(json.dumps(response()).encode())

            def __exit__(self, *_args):
                return False

        def fake_urlopen(request, timeout):
            captured["headers"] = dict(request.header_items())
            captured["body"] = json.loads(request.data)
            captured["timeout"] = timeout
            return FakeResponse()

        result = ci_router.call_classifier(
            "context",
            "classifier_agent_test",
            urlopen=fake_urlopen,
        )
        self.assertEqual(result["model"], "jev-1.13.0")
        self.assertEqual(
            captured["headers"]["Authorization"],
            "Bearer classifier_agent_test",
        )
        self.assertEqual(captured["body"]["model"], "jev")
        self.assertEqual(captured["body"]["tier"], "fast")
        for dimension in [
            "verification_lane",
            "regression_risk",
            "backend_acceptance",
            "organization_context_batch",
            "ssh_transport",
        ]:
            self.assertIn(dimension, captured["body"]["dimensions"])
        self.assertNotIn("classifier_agent_test", json.dumps(captured["body"]))

    def test_github_outputs_expose_decisions_without_reason_or_secret(self):
        route = ci_router.Route(
            presentation=True,
            chat_tools=False,
            environment=False,
            backend_mode="linux",
            native_batch=False,
            ssh=True,
            scope="presentation",
            reason="secret-ish reason",
            semantic_used=True,
            lane_label=ci_router.PRESENTATION,
            lane_confidence=0.9,
            risk_label=ci_router.SUBSYSTEM_RISK,
            risk_confidence=0.8,
            backend_label=ci_router.BACKEND_LINUX,
            backend_confidence=0.91,
            native_label=ci_router.NATIVE_SKIP,
            native_confidence=0.92,
            ssh_label=ci_router.SSH_RUN,
            ssh_confidence=0.93,
            model="jev",
        )
        with tempfile.NamedTemporaryFile(mode="r+", encoding="utf-8") as output:
            ci_router.write_github_outputs(route, output.name)
            output.seek(0)
            text = output.read()
        self.assertIn("backend_mode=linux", text)
        self.assertIn("native_batch=false", text)
        self.assertIn("ssh=true", text)
        self.assertIn("backend_confidence=0.9100", text)
        self.assertNotIn("secret-ish", text)


if __name__ == "__main__":
    unittest.main()
