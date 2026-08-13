import copy
import hashlib
import json
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

from method_promotion_manifest import strict_load_json, validate_manifest
from pls_algorithm_v1_factory_evidence import _compare_values
from pls_algorithm_v1_factory_common import (
    MANIFEST_PATH,
    ROOT,
    source_descriptors,
)
from pls_algorithm_v1_packaged_acceptance import (
    REQUIRED_VIEWPORTS,
    verify_native_report,
    verify_visual_report,
)


EXPECTED_IDENTITY = {
    "id": "qpls3.pls.algorithm",
    "method_version": "pls_pm_v1",
    "catalogue_snapshot_date": "2026-08-12",
}
EXPECTED_ROLES = {
    "method_spec",
    "independent_reference",
    "simulation_report",
    "boundary_report",
    "persistence_report",
    "frontend_report",
    "export_report",
    "method_audit",
    "packaged_acceptance",
}


class PlsAlgorithmV1FactoryEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = strict_load_json(MANIFEST_PATH)

    def test_manifest_identity_and_source_contract_are_complete(self):
        feature = self.document["feature"]
        self.assertEqual(EXPECTED_IDENTITY, {key: feature[key] for key in EXPECTED_IDENTITY})
        sources = self.document["qualification"]["source_requirements"]
        self.assertEqual(EXPECTED_ROLES, set(sources))
        for role, paths in sources.items():
            with self.subTest(role=role):
                self.assertTrue(paths)
                self.assertEqual(len(paths), len(set(paths)))
                for relative in paths:
                    self.assertTrue((ROOT / relative).is_file(), relative)

    def test_factory_state_is_derived_and_does_not_borrow_legacy_reports(self):
        evidence = self.document["qualification"]["evidence"]
        result = validate_manifest(MANIFEST_PATH, ROOT)
        self.assertTrue(result["passed"], result["errors"])
        self.assertEqual(result["derived_state"], result["declared_state"])
        serialized = str(evidence)
        self.assertNotIn("pls_core_method_promotion_audit.json", serialized)
        self.assertNotIn("pls_csem_comparison.json", serialized)
        self.assertNotIn("pls_plspm_comparison.json", serialized)
        self.assertNotIn("pls_pca_numpy_comparison.json", serialized)
        for artifacts in evidence.values():
            for artifact in artifacts:
                self.assertTrue(
                    artifact["path"].startswith(
                        "validation/results/method_factory/pls_algorithm_v1/"
                    ),
                    artifact["path"],
                )

    def test_source_descriptors_bind_exact_current_bytes(self):
        paths = [
            "validation/pls_algorithm_v1_factory_common.py",
            "validation/pls_core_simulation.py",
            "validation/pls_core_boundary_gate.py",
            "validation/pls_core_persistence_gate.py",
            "src/native/plsAlgorithmFactory.test.ts",
        ]
        descriptors = source_descriptors(paths)
        self.assertEqual(sorted(paths), [row["path"] for row in descriptors])
        for descriptor in descriptors:
            path = ROOT / descriptor["path"]
            self.assertEqual(path.stat().st_size, descriptor["size"])
            self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), descriptor["sha256"])

    def test_any_partial_stage_is_rejected(self):
        changed = copy.deepcopy(self.document)
        changed["qualification"]["evidence"]["engine_only"] = [
            {
                "path": "validation/results/method_factory/pls_algorithm_v1/method_spec.identity.json",
                "roles": ["method_spec"],
                "verification": {
                    "kind": "identity_report",
                    "identity_pointers": {
                        "passed": "/passed",
                        "feature_id": "/feature_id",
                        "method_version": "/method_version",
                        "catalogue_snapshot_date": "/catalogue_snapshot_date",
                    },
                    "source_artifacts_pointer": "/source_artifacts",
                    "generated_at_pointer": "/generated_at_utc",
                },
            }
        ]
        result = validate_manifest_document_for_test(changed)
        self.assertFalse(result["passed"])
        self.assertTrue(any("missing roles" in error for error in result["errors"]))

    def test_independent_comparison_requires_exact_key_membership(self):
        key = ("path", "x", "y", "")
        exact = _compare_values(
            engine="independent",
            variant="exact",
            actual={key: 0.5},
            expected={key: 0.5 + 1e-8},
        )
        self.assertTrue(exact["passed"])

        missing = _compare_values(
            engine="independent",
            variant="missing",
            actual={},
            expected={key: 0.5},
        )
        self.assertFalse(missing["passed"])
        self.assertFalse(missing["exact_key_membership"])
        self.assertEqual([list(key)], missing["missing_from_quickpls"])

        nonfinite = _compare_values(
            engine="independent",
            variant="nonfinite",
            actual={key: float("nan")},
            expected={key: 0.5},
        )
        self.assertFalse(nonfinite["passed"])

    def test_packaged_viewport_verifier_requires_exact_three_viewports(self):
        rows = []
        results = []
        screenshots = []
        with tempfile.TemporaryDirectory() as directory:
            test_root = Path(directory)
            artifact_dir = (
                test_root
                / "validation"
                / "results"
                / "screens"
                / "v247-native-desktop-visual"
            )
            artifact_dir.mkdir(parents=True)
            for viewport in REQUIRED_VIEWPORTS:
                rows.append(
                    {
                        "viewport": viewport,
                        "linkage": {
                            "expectedKind": "pls_algorithm",
                            "selectedLabel": "PLS-SEM Algorithm",
                            "linkage": True,
                        },
                        "truthAndOverflow": {
                            "noFabricatedRunState": True,
                            "noHorizontalOverflow": True,
                        },
                    }
                )
                results.append(
                    {
                        "viewport": viewport,
                        "source": "completedSamplePlsRun",
                        "runId": "fixture-run",
                        "rowCounts": {"Direct effects": 1},
                    }
                )
                for state in (
                    "calculation-dialog",
                    "completed-results",
                    "export-dialog",
                ):
                    relative = (
                        "validation/results/screens/v247-native-desktop-visual/"
                        f"{state}-{viewport}.png"
                    )
                    content = f"{state}:{viewport}".encode()
                    (test_root / relative).write_bytes(content)
                    screenshots.append(
                        {
                            "viewport": viewport,
                            "state": state,
                            "path": relative,
                            "size": len(content),
                            "sha256": hashlib.sha256(content).hexdigest(),
                        }
                    )
            document = {
                "generatedAt": "2026-08-13T12:00:00Z",
                "passed": True,
                "harness": {"actualTauriWindow": False},
                "viewports": [
                    {"id": viewport, **dimensions}
                    for viewport, dimensions in REQUIRED_VIEWPORTS.items()
                ],
                "checks": {"calculationCatalog": rows, "mediation": results},
                "screenshots": screenshots,
                "coverage": {"screenshotIntegrity": {"passed": True}},
                "failures": [],
                "consoleErrors": [],
            }
            report_path = test_root / "visual.json"
            report_path.write_text(json.dumps(document), encoding="utf-8")
            with patch.multiple(
                "pls_algorithm_v1_packaged_acceptance",
                VISUAL_REPORT=report_path,
                ROOT=test_root,
            ):
                result = verify_visual_report(
                    datetime(2026, 8, 13, 11, 59, tzinfo=timezone.utc)
                )
                self.assertTrue(result["passed"], result)

                document["viewports"].pop()
                report_path.write_text(json.dumps(document), encoding="utf-8")
                result = verify_visual_report(
                    datetime(2026, 8, 13, 11, 59, tzinfo=timezone.utc)
                )
                self.assertFalse(result["passed"])
                self.assertFalse(result["checks"]["exact_required_viewports"])

    def test_packaged_native_verifier_rejects_bootstrap_run_as_algorithm_export(self):
        required_titles = [
            "Direct effects",
            "Specific indirect effects",
            "Total indirect effects",
            "Total effects",
        ]
        document = {
            "generatedAt": "2026-08-13T12:00:00Z",
            "passed": True,
            "focusedRun": None,
            "checks": {
                "runtime": {"tauriRuntime": True, "viewport": {"width": 1440, "height": 900}},
                "mediationPlsDialog": {
                    "selectedMethod": "PLS-SEM Algorithm",
                    "startEnabled": True,
                    "blockers": [],
                },
                "plsAlgorithmInvalidSetup": {
                    "attempted": True,
                    "startEnabled": False,
                    "blockers": ["No valid model"],
                    "resultCreated": False,
                },
                "mediationPlsResult": {
                    "runId": "algorithm-run",
                    "runLabel": "PLS-SEM Algorithm run",
                    "navigation": {
                        "requiredTitles": required_titles,
                        "rowCounts": {title: 1 for title in required_titles},
                        "bootstrapTreeItems": 0,
                    },
                },
                "mediationExport": {
                    "selectedRunId": "bootstrap-run",
                    "nativeXlsx": {
                        "helper": {
                            "completion": {
                                "passed": True,
                                "workbook": {
                                    "sheetNames": ["Run provenance"],
                                    "path": "bootstrap.xlsx",
                                    "sha256": "0" * 64,
                                },
                            }
                        }
                    },
                },
                "mediationSaveReopen": {
                    "hasPlsAlgorithm": True,
                    "selectedRunId": "bootstrap-run",
                    "runOptions": ["PLS-SEM Algorithm run", "PLS-SEM Bootstrapping run"],
                },
            },
            "failures": [],
            "consoleErrors": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            report_path = Path(directory) / "native.json"
            report_path.write_text(json.dumps(document), encoding="utf-8")
            with patch(
                "pls_algorithm_v1_packaged_acceptance.RAW_REPORT", report_path
            ):
                result = verify_native_report(
                    datetime(2026, 8, 13, 11, 59, tzinfo=timezone.utc),
                    cumulative_wrapper_passed=True,
                )
        self.assertFalse(result["passed"])
        self.assertFalse(result["checks"]["export_is_same_pls_algorithm_run"])
        self.assertFalse(result["checks"]["reopen_selects_same_pls_algorithm_run"])
        self.assertTrue(result["checks"]["invalid_pls_setup_is_blocked"])


def validate_manifest_document_for_test(document):
    from method_promotion_manifest import validate_manifest_document

    return validate_manifest_document(
        document,
        ROOT,
        manifest_path=MANIFEST_PATH,
        verify_evidence=False,
    )


if __name__ == "__main__":
    unittest.main()
