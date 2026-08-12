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
        self.assertEqual(report["feature_count"], 16)
        self.assertEqual(
            report["declared_states"],
            {"absent": 1, "engine_only": 1, "native_qualified": 13, "release_qualified": 1},
        )
        self.assertEqual(
            report["derived_states"],
            {"absent": 1, "engine_only": 1, "native_qualified": 13, "release_qualified": 1},
        )
        self.assertEqual(
            [feature["id"] for feature in report["features"] if feature["declared_state"] == "release_qualified"],
            ["qpls3.standalone.logistic"],
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
        document["features"][0]["state"] = "release_qualified"

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

    def test_json_pointer_handles_arrays_and_escaping(self) -> None:
        document = {"a/b": [{"~value": "ok"}]}
        self.assertEqual(json_pointer(document, "/a~1b/0/~0value"), "ok")


if __name__ == "__main__":
    unittest.main()
