#!/usr/bin/env python3
"""Focused tests for the QuickPLS 3 parity-ledger evaluator."""

from __future__ import annotations

import json
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
sys.path.insert(0, str(VALIDATION_DIR))

from parity_ledger import (  # noqa: E402
    evaluate_release,
    json_pointer,
    load_json,
    validate_ledger,
    validate_ledger_document,
)


LEDGER = VALIDATION_DIR / "quickpls_3_parity_ledger.json"


class ParityLedgerTests(unittest.TestCase):
    def test_current_ledger_is_evidence_backed_and_conservative(self) -> None:
        report = validate_ledger(LEDGER, REPOSITORY_ROOT)

        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["feature_count"], 17)
        self.assertEqual(
            report["declared_states"],
            {"absent": 1, "native_qualified": 1, "release_qualified": 15},
        )
        self.assertEqual(
            report["derived_states"],
            {"absent": 1, "native_qualified": 1, "release_qualified": 15},
        )
        # Keep withheld PROCESS descriptors visible so its release gate stays
        # inspectable and fail-closed while qualification remains native-only.
        self.assertEqual(len(report["release_evidence_descriptors"]), 32)
        for descriptor in report["release_evidence_descriptors"]:
            self.assertEqual(set(descriptor), {"path", "size", "sha256"})
            self.assertTrue(descriptor["path"].startswith("validation/results/"))
            self.assertGreater(descriptor["size"], 0)
            self.assertRegex(descriptor["sha256"], r"^[0-9a-f]{64}$")
        self.assertEqual(
            [feature["id"] for feature in report["features"] if feature["declared_state"] == "release_qualified"],
            [
                "qpls3.pls.algorithm",
                "qpls3.pls.consistent",
                "qpls3.pls.weighted",
                "qpls3.gsca.als",
                "qpls3.assessment.cca_residuals",
                "qpls3.assessment.ipma",
                "qpls3.cbsem.ml",
                "qpls3.inference.bootstrap",
                "qpls3.inference.structural_path_randomization",
                "qpls3.prediction.plspredict_cvpat",
                "qpls3.standalone.nca",
                "qpls3.standalone.pca",
                "qpls3.standalone.ols",
                "qpls3.standalone.logistic",
                "qpls3.standalone.regression_bootstrap",
            ],
        )

    def test_duplicate_feature_id_is_rejected(self) -> None:
        document = load_json(LEDGER)
        document["features"][1]["id"] = document["features"][0]["id"]

        report = validate_ledger_document(document, REPOSITORY_ROOT)

        self.assertFalse(report["passed"])
        self.assertTrue(any("duplicate feature IDs" in error for error in report["errors"]))

    def test_shared_catalog_entry_requires_exact_capability_mapping(self) -> None:
        document = load_json(LEDGER)
        logistic = next(
            feature for feature in document["features"]
            if feature["id"] == "qpls3.standalone.logistic"
        )
        logistic["catalog_kind"] = "pca"

        report = validate_ledger_document(document, REPOSITORY_ROOT)

        self.assertFalse(report["passed"])
        self.assertTrue(
            any("catalog capability mapping differs" in error for error in report["errors"])
        )

    def test_release_claim_without_fresh_reports_is_rejected(self) -> None:
        document = deepcopy(load_json(LEDGER))
        gsca = next(
            feature
            for feature in document["features"]
            if feature["id"] == "qpls3.gsca.als"
        )
        gsca["release_evidence"] = None

        report = validate_ledger_document(document, REPOSITORY_ROOT)

        self.assertFalse(report["passed"])
        self.assertTrue(any("release_qualified" in error for error in report["errors"]))

    def test_release_requires_both_passed_reports_and_exact_identities(self) -> None:
        feature = {
            "id": "qpls3.example.method",
            "method_version": "example_v2",
            "release_evidence": {
                "current_scoped_method_audit": {
                    "path": "validation/results/method.json",
                    "passed_pointer": "/passed",
                    "feature_id_pointer": "/feature_id",
                    "method_version_pointer": "/method_version",
                    "catalogue_snapshot_date_pointer": "/catalogue_snapshot_date"
                },
                "packaged_acceptance": {
                    "path": "validation/results/package.json",
                    "passed_pointer": "/passed",
                    "feature_id_pointer": "/feature_id",
                    "method_version_pointer": "/method_version",
                    "catalogue_snapshot_date_pointer": "/catalogue_snapshot_date"
                }
            }
        }
        evidence = {
            "passed": True,
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
        }

        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            results = root / "validation" / "results"
            results.mkdir(parents=True)
            (results / "method.json").write_text(json.dumps(evidence), encoding="utf-8")
            (results / "package.json").write_text(json.dumps(evidence), encoding="utf-8")

            accepted = evaluate_release(feature, root, "2026-08-12")
            self.assertTrue(accepted["passed"])

            failed_package = dict(evidence, passed=False)
            (results / "package.json").write_text(json.dumps(failed_package), encoding="utf-8")
            rejected_pass = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(rejected_pass["passed"])

            wrong_version = dict(evidence, method_version="example_v1")
            (results / "package.json").write_text(json.dumps(wrong_version), encoding="utf-8")
            rejected_identity = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(rejected_identity["passed"])

    def test_release_reports_reject_duplicate_keys_and_nonfinite_numbers(self) -> None:
        feature = {
            "id": "qpls3.example.method",
            "method_version": "example_v2",
            "release_evidence": {
                role: {
                    "path": f"validation/results/{filename}",
                    "passed_pointer": "/passed",
                    "feature_id_pointer": "/feature_id",
                    "method_version_pointer": "/method_version",
                    "catalogue_snapshot_date_pointer": "/catalogue_snapshot_date",
                }
                for role, filename in (
                    ("current_scoped_method_audit", "method.json"),
                    ("packaged_acceptance", "package.json"),
                )
            },
        }
        valid = {
            "passed": True,
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
        }

        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            results = root / "validation" / "results"
            results.mkdir(parents=True)
            (results / "package.json").write_text(json.dumps(valid), encoding="utf-8")

            duplicate = (
                '{"passed":false,"passed":true,'
                '"feature_id":"qpls3.example.method",'
                '"method_version":"example_v2",'
                '"catalogue_snapshot_date":"2026-08-12"}'
            )
            (results / "method.json").write_text(duplicate, encoding="utf-8")
            self.assertFalse(evaluate_release(feature, root, "2026-08-12")["passed"])

            for token in ("NaN", "1e999", "-1e999"):
                with self.subTest(token=token):
                    (results / "method.json").write_text(
                        json.dumps(valid)[:-1] + f',"metric":{token}}}',
                        encoding="utf-8",
                    )
                    self.assertFalse(
                        evaluate_release(feature, root, "2026-08-12")["passed"]
                    )

    def test_release_rejects_nested_failed_check_and_nonempty_errors(self) -> None:
        feature = {
            "id": "qpls3.example.method",
            "method_version": "example_v2",
            "release_evidence": {
                role: {
                    "path": f"validation/results/{filename}",
                    "passed_pointer": "/passed",
                    "feature_id_pointer": "/feature_id",
                    "method_version_pointer": "/method_version",
                    "catalogue_snapshot_date_pointer": "/catalogue_snapshot_date",
                }
                for role, filename in (
                    ("current_scoped_method_audit", "method.json"),
                    ("packaged_acceptance", "package.json"),
                )
            },
        }
        base = {
            "passed": True,
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
            "checks": {"identity": {"passed": True}},
            "errors": [],
            "failures": 0,
            "console_errors": 0,
        }

        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            results = root / "validation" / "results"
            results.mkdir(parents=True)
            (results / "method.json").write_text(json.dumps(base), encoding="utf-8")
            (results / "package.json").write_text(json.dumps(base), encoding="utf-8")
            self.assertTrue(evaluate_release(feature, root, "2026-08-12")["passed"])

            nested_false = deepcopy(base)
            nested_false["checks"]["identity"]["passed"] = False
            (results / "method.json").write_text(json.dumps(nested_false), encoding="utf-8")
            false_report = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(false_report["passed"])
            self.assertIn(
                "/checks/identity/passed=false",
                false_report["method_audit"]["semantic_integrity"]["failures"],
            )

            (results / "method.json").write_text(json.dumps(base), encoding="utf-8")
            nonempty_errors = deepcopy(base)
            nonempty_errors["errors"] = ["hidden failure"]
            (results / "package.json").write_text(json.dumps(nonempty_errors), encoding="utf-8")
            error_report = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(error_report["passed"])
            self.assertIn(
                "/errors is not empty",
                error_report["packaged_acceptance"]["semantic_integrity"]["failures"],
            )

            nonzero_failure_count = deepcopy(base)
            nonzero_failure_count["failures"] = 1
            (results / "package.json").write_text(
                json.dumps(nonzero_failure_count), encoding="utf-8"
            )
            count_report = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(count_report["passed"])
            self.assertIn(
                "/failures is not empty",
                count_report["packaged_acceptance"]["semantic_integrity"]["failures"],
            )

    def test_release_rejects_failed_freshness_and_inconsistent_reported_descriptor(self) -> None:
        feature = {
            "id": "qpls3.example.method",
            "method_version": "example_v2",
            "release_evidence": {
                role: {
                    "path": f"validation/results/{filename}",
                    "passed_pointer": "/passed",
                    "feature_id_pointer": "/feature_id",
                    "method_version_pointer": "/method_version",
                    "catalogue_snapshot_date_pointer": "/catalogue_snapshot_date",
                }
                for role, filename in (
                    ("current_scoped_method_audit", "method.json"),
                    ("packaged_acceptance", "package.json"),
                )
            },
        }
        base = {
            "passed": True,
            "feature_id": feature["id"],
            "method_version": feature["method_version"],
            "catalogue_snapshot_date": "2026-08-12",
            "source_freshness": [{"path": "source.rs", "report_not_older": True}],
            "artifact": {
                "path": "app.exe",
                "reported_size": 10,
                "actual_size": 10,
                "reported_sha256": "a" * 64,
                "actual_sha256": "a" * 64,
                "passed": True,
            },
        }

        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            results = root / "validation" / "results"
            results.mkdir(parents=True)
            (results / "method.json").write_text(json.dumps(base), encoding="utf-8")
            (results / "package.json").write_text(json.dumps(base), encoding="utf-8")

            stale = deepcopy(base)
            stale["source_freshness"][0]["report_not_older"] = False
            (results / "method.json").write_text(json.dumps(stale), encoding="utf-8")
            stale_report = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(stale_report["passed"])

            (results / "method.json").write_text(json.dumps(base), encoding="utf-8")
            inconsistent = deepcopy(base)
            inconsistent["artifact"]["actual_sha256"] = "b" * 64
            (results / "package.json").write_text(json.dumps(inconsistent), encoding="utf-8")
            descriptor_report = evaluate_release(feature, root, "2026-08-12")
            self.assertFalse(descriptor_report["passed"])
            self.assertTrue(
                any(
                    "reported_sha256 differs" in failure
                    for failure in descriptor_report["packaged_acceptance"]["semantic_integrity"]["failures"]
                )
            )

    def test_json_pointer_handles_arrays_and_escaping(self) -> None:
        document = {"a/b": [{"~value": "ok"}]}
        self.assertEqual(json_pointer(document, "/a~1b/0/~0value"), "ok")


if __name__ == "__main__":
    unittest.main()
