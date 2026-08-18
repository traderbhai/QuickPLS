#!/usr/bin/env python3
"""Fail-closed checks for the bounded PLS model-fit QualificationSpec V2."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
SPEC = (
    VALIDATION_DIR
    / "qualification_v2"
    / "pls_model_fit_exact_v1.qualification.json"
)
REGISTRY = VALIDATION_DIR / "capabilities" / "capability_registry_v2.json"
sys.path.insert(0, str(VALIDATION_DIR))

from qualification_spec_v2 import strict_load_json, validate_spec_path  # noqa: E402


class PlsModelFitExactQualificationV2Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = strict_load_json(SPEC)

    def test_scaffold_is_semantically_valid_and_registry_linked_without_promotion(self) -> None:
        result = validate_spec_path(
            SPEC,
            repository_root=REPOSITORY_ROOT,
            registry_path=REGISTRY,
            require_registry=True,
        )

        self.assertTrue(result["passed"], result)
        self.assertTrue(result["schema_valid"], result)
        self.assertTrue(result["semantic_valid"], result)
        self.assertTrue(result["registry_verified"], result)
        self.assertFalse(result["receipts_verified"], result)
        self.assertFalse(result["qualification_ready"], result)
        self.assertEqual(
            self.document["migration"]["status"],
            "compatibility_only",
        )
        self.assertEqual(self.document["evidence_contract"]["receipts"], [])

    def test_scaffold_names_the_unclosed_full_pipeline_and_product_gates(self) -> None:
        unresolved = set(self.document["migration"]["unresolved_items"])
        self.assertNotIn(
            "oracle.full_independent_pls_refit_pipeline_missing",
            unresolved,
        )
        self.assertIn(
            "oracle.full_plsc_and_advanced_model_shape_refit_pipeline_missing",
            unresolved,
        )
        self.assertIn(
            "oracle.small_pls_pm_refit_work_report_is_not_immutable_receipt",
            unresolved,
        )
        self.assertIn(
            "simulation.type_i_error_power_coverage_and_failure_rate_not_run",
            unresolved,
        )
        self.assertIn(
            "persistence.real_999_draw_archive_reopen_and_all_format_readback_not_recorded",
            unresolved,
        )
        self.assertIn(
            "packaged_windows.accessibility_scaling_cancellation_matrix_not_run",
            unresolved,
        )
        self.assertIn(
            "review.independent_scientific_review_not_recorded",
            unresolved,
        )

    def test_every_current_validation_locator_exists(self) -> None:
        for oracle in self.document["scientific_contract"]["oracles"]:
            locator = REPOSITORY_ROOT / oracle["locator"]
            self.assertTrue(locator.is_file(), (oracle["id"], locator))

    def test_full_refit_oracle_is_registered_as_early_evidence_not_a_receipt(self) -> None:
        oracles = {
            oracle["id"]: oracle
            for oracle in self.document["scientific_contract"]["oracles"]
        }
        full_refit = oracles["numpy_scipy_pls_pm_full_refit_oracle"]
        self.assertEqual(full_refit["kind"], "independent_implementation")
        self.assertEqual(
            full_refit["locator"],
            "validation/pls_model_fit_full_refit_oracle.py",
        )
        self.assertEqual(
            set(full_refit["covered_estimand_ids"]),
            {
                "point_fit_discrepancy_family",
                "adapted_bollen_stine_exact_fit",
            },
        )
        self.assertEqual(self.document["evidence_contract"]["receipts"], [])


if __name__ == "__main__":
    unittest.main()
