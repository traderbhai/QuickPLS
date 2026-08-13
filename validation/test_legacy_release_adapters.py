from __future__ import annotations

import copy
import json
import os
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent))
import legacy_release_adapter_common as adapters


UTC = timezone.utc


class LegacyReleaseAdapterTests(unittest.TestCase):
    def _documents(self, method: str) -> tuple[dict, dict]:
        config = adapters.METHODS[method]
        generated = "2026-08-13T16:00:00Z"
        completed = "2026-08-13T16:01:00Z"
        raw_checks = {
            raw_name: {"passed": True}
            for raw_name in config["package_checks"].values()
            if raw_name is not None
        }
        packaged = {
            "schema_version": "quickpls.packaged_acceptance.v1",
            "kind": config["kind"],
            "passed": True,
            "generated_at_utc": generated,
            "completed_at_utc": completed,
            "feature_id": config["feature_id"],
            "method_version": config["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
            "runtime": "tauri-webview2-cdp",
            "endpoint": "http://127.0.0.1:9222",
            "source_report": config["raw_report"],
            "checks": {
                name: {
                    "passed": True,
                    **(
                        {"source_check": raw_name}
                        if raw_name is not None and name != raw_name
                        else {}
                    ),
                }
                for name, raw_name in config["package_checks"].items()
            },
            "failures": [],
            "console_errors": [],
        }
        raw = {
            "passed": True,
            "generatedAt": generated,
            "feature_id": config["feature_id"],
            "method_version": config["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
            "focusedRun": {"scope": config["scope"], "completedAt": completed},
            "checks": raw_checks,
            "failures": [],
            "consoleErrors": [],
        }
        return packaged, raw

    def test_all_four_scoped_contracts_accept_exact_current_documents(self) -> None:
        not_before = datetime(2026, 8, 13, 15, 59, tzinfo=UTC)
        build_finished = datetime(2026, 8, 13, 15, 30, tzinfo=UTC)
        for method in adapters.METHODS:
            with self.subTest(method=method):
                packaged, raw = self._documents(method)
                result = adapters.evaluate_scoped_documents(
                    method, packaged, raw, not_before, build_finished
                )
                self.assertTrue(result["passed"], result)
                self.assertTrue(all(result["checks"].values()), result)

    def test_identity_time_scope_and_check_mutations_fail_closed(self) -> None:
        method = "logistic_regression_v2"
        not_before = datetime(2026, 8, 13, 15, 59, tzinfo=UTC)
        build_finished = datetime(2026, 8, 13, 15, 30, tzinfo=UTC)
        mutations = {
            "feature": lambda package, raw: package.__setitem__("feature_id", "qpls3.changed"),
            "method": lambda package, raw: raw.__setitem__("method_version", "regression_logistic_v1"),
            "scope": lambda package, raw: raw["focusedRun"].__setitem__("scope", "ols"),
            "stale": lambda package, raw: (
                package.__setitem__("generated_at_utc", "2026-08-13T15:00:00Z"),
                raw.__setitem__("generatedAt", "2026-08-13T15:00:00Z"),
            ),
            "package_red": lambda package, raw: package.__setitem__("passed", False),
            "raw_check_red": lambda package, raw: raw["checks"]["logisticResult"].__setitem__("passed", False),
            "package_source_drift": lambda package, raw: package["checks"]["results"].__setitem__("source_check", "logisticExport"),
            "extra_check": lambda package, raw: package["checks"].__setitem__("unscoped", {"passed": True}),
            "source_report": lambda package, raw: package.__setitem__("source_report", "validation/results/other.json"),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                packaged, raw = self._documents(method)
                mutate(packaged, raw)
                result = adapters.evaluate_scoped_documents(
                    method, packaged, raw, not_before, build_finished
                )
                self.assertFalse(result["passed"], result)

    def _visual_row(self, method: str) -> dict:
        common = {
            "dialogOpened": True,
            "truthAndOverflow": {
                "noFabricatedRunState": True,
                "noHorizontalOverflow": True,
            },
            "closeFocus": {"dialogClosed": True, "focusRestored": True},
        }
        if method == "structural_path_randomization_v1":
            return {
                **common,
                "pointerSelected": True,
                "linkage": {"expectedKind": "pls_permutation", "linkage": True},
                "mutuallyExclusive": True,
                "distinctFromMgaAndMicom": True,
            }
        if method == "logistic_regression_v2":
            return {
                **common,
                "fixtureApiPresent": True,
                "dataSurface": True,
                "visibleModelNodes": 0,
                "linkage": {"expectedKind": "regression", "linkage": True},
                "regressionType": "logistic",
                "startCommandDisabled": True,
                "noModelBlocker": True,
                "noPhantomResult": True,
            }
        if method == "regression_bootstrap_v1":
            return {
                "dialogOpened": True,
                "fixtureApiPresent": True,
                "dataSurface": True,
                "visibleModelNodes": 0,
                "linkage": {"linkage": True},
                "bootstrap": {"value": "enabled"},
                "ols": {
                    "startCommandDisabled": True,
                    "truthAndOverflow": {
                        "noFabricatedRunState": True,
                        "noHorizontalOverflow": True,
                    },
                    "noPhantomResult": True,
                },
                "logistic": {
                    "startCommandDisabled": True,
                    "truthAndOverflow": {
                        "noFabricatedRunState": True,
                        "noHorizontalOverflow": True,
                    },
                    "noPhantomResult": True,
                },
                "closeFocus": {"dialogClosed": True, "focusRestored": True},
            }
        return {
            **common,
            "fixtureApiPresent": True,
            "dataSurface": True,
            "regressionType": "process",
            "setup": {
                "pathsExact": True,
                "moderatorsExact": True,
                "moderationsExact": True,
                "stableRowIdentity": {
                    "paths": {"passed": True},
                    "moderators": {"passed": True},
                    "moderations": {"passed": True},
                },
            },
            "accessibility": {
                "controlsLabeled": True,
                "groupsNamed": True,
                "keyboardReachable": True,
                "focusRestored": True,
            },
            "dialogBounds": {"withinHorizontalViewport": True},
            "completedResult": {"synthesizedByHarness": False},
        }

    def test_each_responsive_contract_has_a_fail_closed_mutation(self) -> None:
        for method in adapters.METHODS:
            with self.subTest(method=method):
                row = self._visual_row(method)
                self.assertTrue(adapters._visual_row_passes(method, row), row)
                changed = copy.deepcopy(row)
                if method == "regression_bootstrap_v1":
                    changed["logistic"]["noPhantomResult"] = False
                elif method == "process_v2":
                    changed["setup"]["stableRowIdentity"]["paths"]["passed"] = False
                else:
                    changed["truthAndOverflow"]["noHorizontalOverflow"] = False
                self.assertFalse(adapters._visual_row_passes(method, changed), changed)

    def test_artifact_hash_mutation_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            artifact = root / "validation/results/evidence.xlsx"
            artifact.parent.mkdir(parents=True)
            artifact.write_bytes(b"exact bytes")
            reported = {
                "path": "validation/results/evidence.xlsx",
                "size": artifact.stat().st_size,
                "sha256": adapters.hashlib.sha256(artifact.read_bytes()).hexdigest(),
            }
            with patch.object(adapters, "ROOT", root):
                passed, _ = adapters._descriptor_matches(reported)
                self.assertTrue(passed)
                artifact.write_bytes(b"mutated bytes")
                passed, _ = adapters._descriptor_matches(reported)
                self.assertFalse(passed)

    def test_final_audit_never_consumes_its_own_output(self) -> None:
        method = "logistic_regression_v2"
        document = adapters.manifest(method)
        packaged_report = {
            "checks": {
                "prior_factory_stages": {"passed": True},
                "source_freshness": {
                    "passed": True,
                    "source_stable_during_gate": True,
                },
                "native": {"passed": True},
                "responsive_viewports": {"passed": True},
                "runner_cleanup_verified": True,
            }
        }
        observed_roles: list[str] = []

        def verify(artifact, *_args):
            observed_roles.extend(artifact["roles"])
            return True, []

        with (
            patch.object(adapters, "_verify_artifact", side_effect=verify),
            patch.object(adapters, "strict_load_json", return_value=packaged_report),
        ):
            result = adapters.evaluate_audit_inputs(method, document)
        self.assertTrue(result["passed"], result)
        self.assertTrue(result["release_evidence_contract_passes"], result)
        self.assertIn("packaged_acceptance", observed_roles)
        self.assertNotIn("method_audit", observed_roles)

        changed = copy.deepcopy(document)
        changed["qualification"]["evidence"]["release_qualified"].append(
            copy.deepcopy(changed["qualification"]["evidence"]["release_qualified"][1])
        )
        with (
            patch.object(adapters, "_verify_artifact", side_effect=verify),
            patch.object(adapters, "strict_load_json", return_value=packaged_report),
        ):
            mutated = adapters.evaluate_audit_inputs(method, changed)
        self.assertFalse(mutated["passed"], mutated)
        self.assertFalse(mutated["release_evidence_contract_passes"], mutated)

    def test_method_audit_identity_binds_the_exact_packaged_receipt(self) -> None:
        staged = [
            {"path": "validation/results/method_factory/process_v2/native_stage.identity.json"},
            {"path": "validation/results/method_factory/process_v2/packaged_acceptance.identity.json"},
        ]
        written: dict = {}

        def write(*_args, **kwargs):
            written.update(kwargs)
            return Path("method_audit.identity.json")

        with (
            patch.object(
                adapters,
                "evaluate_audit_inputs",
                return_value={"passed": True, "stage_artifacts": staged},
            ),
            patch.object(adapters, "write_identity_report", side_effect=write),
        ):
            self.assertEqual(adapters.audit_main("process_v2"), 0)

        self.assertIn(
            "validation/results/method_factory/process_v2/packaged_acceptance.identity.json",
            written["extras"],
        )
        self.assertNotIn(
            "validation/results/method_factory/process_v2/method_audit.identity.json",
            written["extras"],
        )

    def test_manifests_add_exact_release_roles_without_overclaiming_declared_state(self) -> None:
        for method, config in adapters.METHODS.items():
            with self.subTest(method=method):
                document = json.loads((adapters.ROOT / config["manifest"]).read_text(encoding="utf-8"))
                release = document["qualification"]["evidence"]["release_qualified"]
                expected_roles = (
                    []
                    if method == "process_v2"
                    else [["method_audit"], ["packaged_acceptance"]]
                )
                self.assertEqual([row["roles"] for row in release], expected_roles)
                expected_declared_state = (
                    "native_qualified" if method == "process_v2" else "release_qualified"
                )
                self.assertEqual(
                    document["qualification"]["declared_state"],
                    expected_declared_state,
                )
                self.assertEqual(document["qualification"]["target_state"], "release_qualified")


if __name__ == "__main__":
    unittest.main()
