#!/usr/bin/env python3
"""Regression tests for conservative semantic native CI routing."""
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


def response(lane, lane_confidence=0.95, risk=ci_router.LOW_RISK, risk_confidence=0.90):
    return {
        "model": "jev-1.13.0",
        "results": [{
            "dimensions": {
                "verification_lane": {"label": lane, "confidence": lane_confidence, "scores": {}},
                "regression_risk": {"label": risk, "confidence": risk_confidence, "scores": {}},
            }
        }],
    }


class RouterTests(unittest.TestCase):
    def test_existing_deterministic_scopes_do_not_call_classifier(self):
        cases = [
            (["ROADMAP.md"], (False, False, False, "docs")),
            (["crates/synara-app/src/ui/menu.rs"], (True, False, False, "presentation")),
            (["crates/synara-app/src/shell/environment.rs"], (False, False, True, "environment")),
            (["crates/synara-app/src/shell/chat_tools.rs"], (False, True, False, "chat-tools")),
        ]
        for paths, expected in cases:
            with self.subTest(paths=paths):
                classifier = Mock(side_effect=AssertionError("classifier should not run"))
                route = ci_router.select_route(paths, context="unused", api_key=None, classifier=classifier)
                self.assertEqual(
                    (route.presentation, route.chat_tools, route.environment, route.scope),
                    expected,
                )
                classifier.assert_not_called()

    def test_semantic_route_can_focus_new_ui_surface(self):
        paths = ["crates/synara-app/src/ui/new_model_picker.rs"]
        classifier = Mock(return_value=response(ci_router.PRESENTATION))
        route = ci_router.select_route(paths, context="ctx", api_key="secret", classifier=classifier)
        self.assertTrue(route.presentation)
        self.assertEqual(route.scope, "presentation")
        self.assertTrue(route.semantic_used)
        classifier.assert_called_once_with("ctx", "secret")

    def test_semantic_chat_and_environment_routes_remain_distinct(self):
        for lane, expected in [
            (ci_router.CHAT_TOOLS, (False, True, False, "chat-tools")),
            (ci_router.ENVIRONMENT, (False, False, True, "environment")),
        ]:
            with self.subTest(lane=lane):
                route = ci_router.semantic_route(["crates/synara-app/src/new.rs"], response(lane))
                self.assertEqual(
                    (route.presentation, route.chat_tools, route.environment, route.scope),
                    expected,
                )

    def test_high_risk_native_change_adds_broad_presentation(self):
        route = ci_router.semantic_route(
            ["crates/synara-app/src/shell/chat_tools_v2.rs"],
            response(ci_router.CHAT_TOOLS, risk=ci_router.HIGH_RISK),
        )
        self.assertTrue(route.presentation)
        self.assertTrue(route.chat_tools)
        self.assertEqual(route.scope, "presentation+chat-tools")

    def test_low_confidence_native_change_fails_closed_to_presentation(self):
        route = ci_router.semantic_route(
            ["crates/synara-app/src/ui/new_surface.rs"],
            response(ci_router.NO_FOCUSED_UI, lane_confidence=0.60),
        )
        self.assertTrue(route.presentation)
        self.assertEqual(route.scope, "presentation")

    def test_low_confidence_backend_change_does_not_invent_ui_work(self):
        route = ci_router.semantic_route(
            ["crates/synara-runtime/src/new_contract.rs"],
            response(ci_router.PRESENTATION, lane_confidence=0.60),
        )
        self.assertFalse(route.presentation)
        self.assertEqual(route.scope, "baseline-only")

    def test_classifier_failure_falls_back_without_failing_ci(self):
        def broken(_context, _key):
            raise urllib.error.URLError("offline")

        native = ci_router.select_route(
            ["crates/synara-app/src/new_surface.rs"],
            context="ctx",
            api_key=None,
            classifier=broken,
        )
        backend = ci_router.select_route(
            ["crates/synara-runtime/src/new_contract.rs"],
            context="ctx",
            api_key=None,
            classifier=broken,
        )
        self.assertTrue(native.presentation)
        self.assertEqual(backend.scope, "baseline-only")

    def test_request_uses_jev_bearer_key_and_dimensions(self):
        captured = {}

        class FakeResponse:
            def __enter__(self):
                return io.BytesIO(json.dumps(response(ci_router.PRESENTATION)).encode())

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
        self.assertIn("verification_lane", captured["body"]["dimensions"])
        self.assertNotIn("classifier_agent_test", json.dumps(captured["body"]))

    def test_github_outputs_do_not_include_reason_or_secret_material(self):
        route = ci_router.Route(
            True,
            False,
            False,
            "presentation",
            "secret-ish reason",
            True,
            0.9,
            0.8,
            "jev",
        )
        with tempfile.NamedTemporaryFile(mode="r+", encoding="utf-8") as output:
            ci_router.write_github_outputs(route, output.name)
            output.seek(0)
            text = output.read()
        self.assertIn("presentation=true", text)
        self.assertIn("scope=presentation", text)
        self.assertNotIn("secret-ish", text)


if __name__ == "__main__":
    unittest.main()
