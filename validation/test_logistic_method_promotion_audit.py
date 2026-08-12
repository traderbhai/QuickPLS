#!/usr/bin/env python3
"""Focused fail-closed tests for the current logistic v2 promotion audit."""

from __future__ import annotations

import json
import hashlib
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import logistic_method_promotion_audit as audit  # noqa: E402


NOW = datetime(2026, 8, 12, 12, 0, tzinfo=timezone.utc)


def write_json(path: Path, value: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value), encoding="utf-8")


def write_artifact(path: Path, contents: bytes) -> dict[str, object]:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(contents)
    return {
        "path": path.as_posix(),
        "size": len(contents),
        "sha256": hashlib.sha256(contents).hexdigest(),
    }


class LogisticPromotionAuditTests(unittest.TestCase):
    def prepare_complete_evidence(self, root: Path) -> Path:
        results = root / "validation" / "results"
        results.mkdir(parents=True, exist_ok=True)
        for relative in [*audit.REFERENCE_SOURCE_PATHS, *audit.PACKAGED_SOURCE_PATHS]:
            source = root / relative
            source.parent.mkdir(parents=True, exist_ok=True)
            source.write_text("evidence generator", encoding="utf-8")

        current_doc = root / "docs" / "methods" / "REGRESSION_LOGISTIC_V2.md"
        current_doc.parent.mkdir(parents=True, exist_ok=True)
        current_doc.write_text(
            " ".join([
                audit.METHOD_VERSION,
                "exact numeric values `0` and `1`",
                "deterministic Newton IRLS",
                "single-worker",
                "not a validated predictive performance estimate",
                "Historical `regression_logistic_v1` results remain readable",
            ]),
            encoding="utf-8",
        )
        (current_doc.parent / "REGRESSION_LOGISTIC_V1.md").write_text(
            " ".join([
                audit.LEGACY_METHOD_VERSION,
                "historical archive-readable contract",
                "New execution and append are disabled",
                audit.METHOD_VERSION,
            ]),
            encoding="utf-8",
        )

        write_json(results / "logistic_v2_reference.recipe.json", {
            "schema_version": 3,
            "dataset_fingerprint": "v2:test",
            "settings": {
                "method": "regression",
                "preprocessing": "unstandardized",
                "workers": 1,
                "confidence_level": 0.95,
            },
            "method_config": {"kind": "regression", "model": {"type": "logistic"}},
            "model": {"constructs": [], "paths": []},
        })
        write_json(results / "logistic_v2_reference_quickpls.json", {
            "status": "completed",
            "provenance": {
                "method": "regression",
                "method_version": audit.METHOD_VERSION,
                "settings": {"workers": 1},
            },
            "payload": {
                "estimation": {
                    "method_version": audit.METHOD_VERSION,
                    "regression": {
                        "method_version": audit.METHOD_VERSION,
                        "regression_type": "logistic",
                        "logistic": {
                            "outcome_profile": {"readiness": "ready"},
                            "convergence": {"converged": True},
                        },
                    },
                },
                "assessment": {"method_version": "assessment_not_applicable_v1"},
            },
        })
        write_json(results / audit.REFERENCE_REPORT_NAME, {
            "schema_version": 1,
            "target": audit.REFERENCE_TARGET,
            "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
            "tolerance": 2e-6,
            "passed": True,
            "checks": {name: True for name in audit.REFERENCE_CHECK_NAMES},
            "scope": {
                "outcome_coding": "exact numeric 0 and 1",
                "missing_data": "listwise_deletion",
                "confidence": "two_sided_95_percent_fixed",
                "classification_threshold": 0.5,
                "workers": 1,
            },
            "maximum_absolute_difference_python": 1e-7,
            "maximum_absolute_difference_r": 2e-7,
            "r_reference": {"available": True, "passed": True, "version": "R test"},
            "artifacts": {
                "fixture": "validation/results/logistic_v2_reference.csv",
                "recipe": "validation/results/logistic_v2_reference.recipe.json",
                "quickpls_result": "validation/results/logistic_v2_reference_quickpls.json",
            },
        })
        write_json(root / audit.PACKAGED_SOURCE_REPORT, {
            "schema_version": audit.PACKAGED_SCHEMA_VERSION,
            "passed": True,
            "generatedAt": "2026-08-12T11:58:00Z",
            "runtime": audit.PACKAGED_RUNTIME,
            "focusedRun": {"completedAt": "2026-08-12T11:59:00Z"},
            "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
            "screenshots": [f"screenshot-{index}.png" for index in range(6)],
            "checks": {
                "logisticWorkflow": {
                    "passed": True,
                    "feature_id": audit.FEATURE_ID,
                    "method_version": audit.METHOD_VERSION,
                    "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
                },
                "logisticFailureLifecycle": {"passed": True},
                "logisticLegacyV1": {"passed": True},
            },
        })
        xlsx = write_artifact(
            results / "logistic-v2.xlsx",
            b"genuine-xlsx-fixture",
        )
        project_archive = write_artifact(
            results / "logistic-v2.qpls",
            b"genuine-project-fixture",
        )
        screenshots = [
            write_artifact(
                results / "screens" / f"{150 + index}-tauri-native-logistic-fixture.png",
                f"screenshot-{index}".encode("utf-8"),
            )
            for index in range(6)
        ]
        for descriptor in [xlsx, project_archive, *screenshots]:
            descriptor["path"] = str(
                Path(str(descriptor["path"])).relative_to(root)
            ).replace("\\", "/")
        write_json(results / audit.PACKAGED_REPORT_NAME, {
            "schema_version": audit.PACKAGED_SCHEMA_VERSION,
            "kind": audit.PACKAGED_KIND,
            "target": audit.PACKAGED_PLATFORM_TARGET,
            "feature_id": audit.FEATURE_ID,
            "method_version": audit.METHOD_VERSION,
            "catalogue_snapshot_date": audit.CATALOGUE_SNAPSHOT_DATE,
            "runtime": audit.PACKAGED_RUNTIME,
            "generator": audit.PACKAGED_GENERATOR,
            "source_report": audit.PACKAGED_SOURCE_REPORT,
            "generated_at_utc": "2026-08-12T11:58:00Z",
            "completed_at_utc": "2026-08-12T11:59:00Z",
            "passed": True,
            "artifacts": {
                "xlsx": xlsx,
                "project_archive": project_archive,
                "screenshots": screenshots,
            },
            "checks": {
                **{name: {"passed": True} for name in audit.PACKAGED_CHECK_NAMES},
                "export": {"passed": True, "artifact_sha256": xlsx["sha256"]},
                "save_reopen": {"passed": True, "archive_sha256": project_archive["sha256"]},
            },
        })
        return results

    def test_missing_evidence_keeps_exact_current_identity_failed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = root / "validation" / "results"
            results.mkdir(parents=True)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertEqual(report["feature_id"], audit.FEATURE_ID)
            self.assertEqual(report["method_version"], audit.METHOD_VERSION)
            self.assertEqual(report["catalogue_snapshot_date"], audit.CATALOGUE_SNAPSHOT_DATE)

    def test_exact_dedicated_reference_and_packaged_evidence_can_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertTrue(report["passed"], report)

    def test_legacy_reference_or_wrong_packaged_identity_fails_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            reference_path = results / audit.REFERENCE_REPORT_NAME
            reference = json.loads(reference_path.read_text(encoding="utf-8"))
            reference["method_version"] = audit.LEGACY_METHOD_VERSION
            write_json(reference_path, reference)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

            self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT_NAME
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            packaged["feature_id"] = "qpls3.standalone.ols"
            write_json(packaged_path, packaged)
            self.assertFalse(audit.build_audit(root=root, results=results, now=NOW)["passed"])

    def test_generic_or_incomplete_packaged_checks_are_not_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            results = self.prepare_complete_evidence(root)
            packaged_path = results / audit.PACKAGED_REPORT_NAME
            packaged = json.loads(packaged_path.read_text(encoding="utf-8"))
            packaged["kind"] = "cumulative_native_acceptance"
            packaged["checks"].pop("failure_lifecycle")
            write_json(packaged_path, packaged)
            report = audit.build_audit(root=root, results=results, now=NOW)
            self.assertFalse(report["passed"])
            self.assertFalse(report["exact_check_sets"]["packaged_acceptance"]["passed"])


if __name__ == "__main__":
    unittest.main()
