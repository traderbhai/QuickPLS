#!/usr/bin/env python3
"""Fail-closed checks for the post-hoc technical sample-size V2 contract."""

from __future__ import annotations

import sys
import unittest
from copy import deepcopy
from pathlib import Path


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
SPEC_PATH = (
    VALIDATION_DIR
    / "qualification_v2"
    / "pls_posthoc_technical_minimum_sample_size_v2.qualification.json"
)
REGISTRY_PATH = VALIDATION_DIR / "capabilities" / "capability_registry_v2.json"
sys.path.insert(0, str(VALIDATION_DIR))

from qualification_spec_v2 import strict_load_json, validate_spec_document  # noqa: E402


class PlsPosthocTechnicalSampleSizeQualificationV2Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = strict_load_json(SPEC_PATH)
        cls.registry = strict_load_json(REGISTRY_PATH)

    def test_contract_is_valid_and_linked_but_remains_deliberately_unqualified(self) -> None:
        report = validate_spec_document(
            self.document,
            repository_root=REPOSITORY_ROOT,
            verify_receipts=True,
            registry_document=self.registry,
            require_registry=True,
        )

        self.assertTrue(report["passed"], report)
        self.assertTrue(report["schema_valid"], report)
        self.assertTrue(report["semantic_valid"], report)
        self.assertTrue(report["registry_verified"], report)
        self.assertTrue(report["receipts_verified"], report)
        self.assertFalse(report["qualification_ready"], report)
        self.assertEqual(self.document["migration"]["status"], "compatibility_only")
        self.assertEqual(self.document["evidence_contract"]["receipts"], [])
        self.assertGreater(len(self.document["migration"]["unresolved_items"]), 0)

    def test_formula_direction_and_driver_significance_are_separate_frozen_boundaries(self) -> None:
        preprocessing = {
            row["id"]: row for row in self.document["scientific_contract"]["preprocessing"]
        }
        selection = preprocessing["significant_driver_selection"]["parameters"]
        formula = preprocessing["inverse_square_root_calculation"]["parameters"]

        self.assertEqual(formula["formula_test"], "directional")
        self.assertEqual(formula["alpha"], 0.05)
        self.assertEqual(formula["power"], 0.80)
        self.assertEqual(formula["constant"], 2.486)
        self.assertEqual(selection["alternative"], "two_sided")
        self.assertEqual(selection["alpha"], 0.05)

    def test_executed_oracle_and_compact_scale_boundary_are_frozen(self) -> None:
        oracles = {
            row["id"]: row for row in self.document["scientific_contract"]["oracles"]
        }
        r_oracle = oracles["required_independent_r_reference"]
        self.assertEqual(
            r_oracle["implementation"]["version"],
            "inverse_square_root_posthoc_v2_reference_v1",
        )
        self.assertTrue(
            (REPOSITORY_ROOT / r_oracle["locator"]).is_file(), r_oracle
        )

        profiles = {
            row["id"]: row
            for row in self.document["scenario_contract"]["complexity_profiles"]
        }
        self.assertEqual(profiles["micro_exact"]["workload"]["candidate_models"], 24)
        self.assertEqual(profiles["applied"]["workload"]["resamples"], 99)
        for profile_id in ("large", "maximum_axis", "compound_stress"):
            self.assertEqual(profiles[profile_id]["applicability"], "not_applicable")
            self.assertTrue(profiles[profile_id]["not_applicable_reason"])

        required_roles = set(self.document["evidence_contract"]["required_roles"])
        self.assertNotIn("generative_recovery", required_roles)
        self.assertNotIn("performance_scale", required_roles)

    def test_erasing_open_evidence_work_cannot_promote_the_cell(self) -> None:
        forged = deepcopy(self.document)
        forged["migration"]["status"] = "completed"
        forged["migration"]["unresolved_items"] = []

        report = validate_spec_document(
            forged,
            repository_root=REPOSITORY_ROOT,
            verify_receipts=True,
            registry_document=self.registry,
            require_registry=True,
        )

        self.assertFalse(report["passed"], report)
        self.assertFalse(report["qualification_ready"], report)
        self.assertTrue(
            any("receipt roles must exactly match" in error for error in report["errors"]),
            report,
        )


if __name__ == "__main__":
    unittest.main()
