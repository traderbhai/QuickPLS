#!/usr/bin/env python3
"""Tests for portable, non-claiming competitor-program validation."""

from __future__ import annotations

import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from validation import method_promotion_manifest as factory
from validation import quickpls_3_competitor_contracts as contracts
from validation.quickpls_3_competitor_program import load_json
from validation.quickpls_3_competitor_program import validate_program_document


class QuickPls3CompetitorContractTests(unittest.TestCase):
    def test_current_contracts_pass_without_materializing_runtime_evidence(self) -> None:
        with patch.object(
            factory,
            "_verify_artifact",
            side_effect=AssertionError("portable validation read runtime evidence"),
        ):
            report = contracts.validate_contracts()

        self.assertTrue(report["passed"], report)
        self.assertFalse(report["claim_authorized"])
        self.assertFalse(report["evidence_verified"])
        self.assertFalse(report["competitor_ready"])
        self.assertEqual(report["method_manifest_count"], 47)
        self.assertEqual(report["method_count"], 45)

    def test_catalogue_mapping_mutation_still_fails_closed(self) -> None:
        document = copy.deepcopy(load_json(contracts.CATALOGUE))
        method = next(row for row in document["methods"] if row["competitor_scope"])
        method["quickpls_capability_ids"] = []

        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "catalogue.json"
            path.write_text(json.dumps(document), encoding="utf-8")
            report = contracts.validate_contracts(catalogue_path=path)

        self.assertFalse(report["passed"])
        self.assertFalse(report["claim_authorized"])
        self.assertFalse(report["evidence_verified"])
        self.assertFalse(report["competitor_ready"])

    def test_non_claiming_factory_report_is_rejected_by_claim_api(self) -> None:
        method_report = contracts.method_promotion_contracts.validate_contracts()
        factory_report = contracts._declared_factory_contract(method_report)
        parity_report = contracts.parity_ledger.validate_ledger(
            contracts.LEDGER,
            contracts.ROOT,
        )
        commercial_report = contracts.quickpls_3_release_readiness.load_and_validate(
            contracts.READINESS,
            repository_root=contracts.ROOT,
        )
        beta_report = contracts.quickpls_external_beta.validate_contract(
            contracts.quickpls_external_beta.strict_json(contracts.BETA)
        )

        report = validate_program_document(
            load_json(contracts.CATALOGUE),
            parity_report,
            contracts.ROOT,
            commercial_readiness_report=commercial_report,
            external_beta_report=beta_report,
            manifest_factory_report=factory_report,
            aggregate_approval_report={"present": False, "passed": False},
        )

        self.assertFalse(report["passed"])
        self.assertFalse(report["competitor_ready"])
        self.assertTrue(
            any("non-claiming method contracts" in error for error in report["errors"]),
            report,
        )


if __name__ == "__main__":
    unittest.main()
