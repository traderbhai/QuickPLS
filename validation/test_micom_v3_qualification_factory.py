#!/usr/bin/env python3
"""Fail-closed checks for the MICOM v3 QualificationSpec work factory."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import micom_v3_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_document,
    validate_spec_path,
)


class MicomV3QualificationFactoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.spec = strict_load_json(factory.SPEC_PATH)
        cls.audit = strict_load_json(factory.AUDIT_PATH)
        cls.work = strict_load_json(factory.ORACLE_REPORT_PATH)

    def test_checked_in_spec_is_semantically_valid_but_not_qualified(self) -> None:
        result = validate_spec_path(
            factory.SPEC_PATH,
            repository_root=ROOT,
            registry_path=factory.REGISTRY_PATH,
            require_registry=True,
        )
        self.assertTrue(result["passed"], result["errors"])
        self.assertTrue(result["registry_verified"], result["errors"])
        self.assertFalse(result["qualification_ready"])
        self.assertEqual(self.spec["migration"]["status"], "compatibility_only")
        self.assertEqual(self.spec["evidence_contract"]["receipts"], [])

    def test_identity_links_existing_absent_micom_cell_without_promoting_it(self) -> None:
        identity = self.spec["identity"]
        self.assertEqual(identity["qualification_id"], factory.QUALIFICATION_ID)
        self.assertEqual(identity["method_version"], factory.METHOD_VERSION)
        self.assertEqual(identity["capability_cell"]["capability_id"], "smartpls.micom")
        self.assertEqual(
            identity["capability_cell"]["cell_id"],
            "qpls3.groups.micom_permutation_mga",
        )
        registry = strict_load_json(factory.REGISTRY_PATH)
        capability = next(
            row
            for row in registry["capabilities"]
            if row["capability_id"] == factory.CAPABILITY_ID
        )
        cell = next(
            row for row in capability["option_cells"] if row["cell_id"] == factory.CELL_ID
        )
        self.assertEqual((capability["coverage_state"], capability["evidence_state"]), ("absent", "absent"))
        self.assertEqual((cell["coverage_state"], cell["evidence_state"], cell["surface"]), ("absent", "absent", "labs"))

    def test_contract_separates_micom_from_mga_and_consistent_permutation(self) -> None:
        output_ids = {
            output
            for estimand in self.spec["scientific_contract"]["estimands"]
            for output in estimand["output_ids"]
        }
        self.assertIn("compositional_correlation", output_ids)
        self.assertIn("permutation_ledger", output_ids)
        self.assertFalse(any("path" in output for output in output_ids))
        self.assertFalse(any("plsc" in output for output in output_ids))
        unresolved = " ".join(self.spec["migration"]["unresolved_items"]).lower()
        self.assertIn("combine micom with structural-path permutation mga", unresolved)
        self.assertIn("exactly-once no-retry", unresolved)

    def test_step1_and_no_retry_semantics_are_frozen(self) -> None:
        preprocessing = {
            row["id"]: row for row in self.spec["scientific_contract"]["preprocessing"]
        }
        step1 = preprocessing["require_qualitative_configural_review"]
        self.assertFalse(step1["parameters"]["computed"])
        self.assertTrue(step1["parameters"]["attestation_required"])
        plan = preprocessing["generate_size_preserving_indexed_partitions"]
        self.assertEqual(plan["parameters"]["retry_policy"], "none")
        self.assertFalse(plan["parameters"]["replacement"])
        self.assertIn(
            "obtained mean differences",
            preprocessing["calculate_mean_and_variance_equality"]["operation"],
        )

    def test_work_report_has_exact_ledger_metamorphics_and_typed_failures(self) -> None:
        self.assertTrue(self.work["passed"])
        self.assertTrue(self.work["work_evidence_only"])
        self.assertFalse(self.work["qualification_ready"])
        self.assertFalse(self.work["promotion_requested"])
        self.assertTrue(all(self.work["checks"].values()), self.work["checks"])
        accounting = self.work["accounting"]
        self.assertEqual(
            accounting["requested_permutations"], accounting["attempted_permutations"]
        )
        self.assertEqual(accounting["retry_policy"], "none")
        self.assertEqual(
            set(self.work["boundary_codes"]),
            {
                "missing_configural_review",
                "empty_group",
                "small_group",
                "extreme_imbalance",
                "degenerate_indicator",
            },
        )

    def test_factory_audit_is_source_bound_and_non_promotional(self) -> None:
        self.assertTrue(self.audit["passed"], self.audit["checks"])
        self.assertTrue(all(self.audit["checks"].values()))
        current_sources = factory.source_descriptors()
        self.assertEqual(self.audit["source_artifacts"], current_sources)
        self.assertEqual(
            self.audit["source_set_sha256"], canonical_sha256(current_sources)
        )
        self.assertEqual(
            self.audit["scenario_set_sha256"],
            canonical_sha256(self.spec["scenario_contract"]),
        )
        self.assertEqual(self.audit["candidate_receipt_descriptors"], [])
        self.assertEqual(self.audit["attached_receipt_count"], 0)
        self.assertFalse(self.audit["qualification_ready"])
        self.assertFalse(self.audit["promotion_allowed"])
        self.assertFalse(self.audit["scientific_review_satisfied"])
        self.assertEqual(
            {
                row["role"]
                for row in self.audit["role_matrix"]
                if row["status"] == "work_evidence_only"
            },
            {"method_contract", "oracle_independence", "adversarial_boundaries"},
        )

    def test_attaching_only_work_report_cannot_make_spec_ready(self) -> None:
        candidate = copy.deepcopy(self.spec)
        self.assertEqual(candidate["evidence_contract"]["receipts"], [])
        result = validate_spec_document(
            candidate,
            repository_root=ROOT,
            registry_document=strict_load_json(factory.REGISTRY_PATH),
        )
        self.assertTrue(result["passed"], result["errors"])
        self.assertFalse(result["qualification_ready"])

    def test_checked_in_factory_verifier_passes_fail_closed(self) -> None:
        result = factory.verify_checked_in_factory()
        self.assertTrue(result["passed"], result)
        self.assertEqual(
            set(result["work_evidence_roles"]),
            {"method_contract", "oracle_independence", "adversarial_boundaries"},
        )
        self.assertFalse(result["qualification_ready"])
        self.assertFalse(result["promotion_allowed"])


if __name__ == "__main__":
    unittest.main()
