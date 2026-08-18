#!/usr/bin/env python3
"""Focused checks for model-fit generative and adversarial work evidence."""

from __future__ import annotations

import ast
import hashlib
import sys
import unittest
from pathlib import Path

import jsonschema


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

from pls_model_fit_v2_qualification_evidence import (  # noqa: E402
    ADVERSARIAL_REPORT,
    SIMULATION_REPORT,
    build_adversarial_report,
    build_generative_report,
    wilson_interval,
)
from qualification_spec_v2 import canonical_sha256, strict_load_json  # noqa: E402


SCHEMA = VALIDATION / "pls_model_fit_v2_qualification_evidence.schema.json"
SPEC = VALIDATION / "qualification_v2" / "pls_model_fit_exact_v1.qualification.json"
SOURCE = VALIDATION / "pls_model_fit_v2_qualification_evidence.py"


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class PlsModelFitQualificationWorkEvidenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = strict_load_json(SCHEMA)
        cls.spec = strict_load_json(SPEC)
        cls.simulation = strict_load_json(SIMULATION_REPORT)
        cls.adversarial = strict_load_json(ADVERSARIAL_REPORT)

    def test_wilson_interval_is_bounded_and_matches_known_case(self) -> None:
        interval = wilson_interval(5, 10)
        self.assertAlmostEqual(interval["estimate"], 0.5)
        self.assertAlmostEqual(interval["lower"], 0.236593090512564)
        self.assertAlmostEqual(interval["upper"], 0.7634069094874361)
        self.assertAlmostEqual(interval["half_width"], 0.2634069094874361)
        with self.assertRaises(ValueError):
            wilson_interval(1, 0)
        with self.assertRaises(ValueError):
            wilson_interval(11, 10)

    def test_checked_in_reports_validate_and_bind_current_source_and_scenarios(self) -> None:
        scenario_sha = canonical_sha256(self.spec["scenario_contract"])
        for report in (self.simulation, self.adversarial):
            jsonschema.Draft202012Validator(self.schema).validate(report)
            self.assertEqual(report["source_sha256"], sha256(SOURCE))
            self.assertEqual(report["scenario_set_sha256"], scenario_sha)
            self.assertTrue(report["passed_work_checks"])
            self.assertFalse(report["qualification_role_satisfied"])
            self.assertFalse(report["receipt_eligible"])
            self.assertTrue(report["blockers"])

    def test_default_generative_report_exposes_precision_and_breadth_gaps(self) -> None:
        design = self.simulation["design"]
        exact = self.simulation["exact_fit_calibration"]
        self.assertEqual(design["point_replicates"], 96)
        self.assertEqual(design["exact_datasets_per_condition"], 12)
        self.assertEqual(design["exact_draws"], 19)
        self.assertFalse(exact["precision_gate_passed"])
        self.assertFalse(exact["product_draw_minimum_gate_passed"])
        self.assertGreater(exact["maximum_wilson_half_width"], 0.01)
        self.assertIn(
            "generative.plsc_formative_mixed_higher_order_and_interaction_axes_not_run",
            self.simulation["blockers"],
        )

    def test_small_generated_work_is_deterministic_but_never_a_receipt(self) -> None:
        first = build_generative_report(
            point_replicates=8,
            exact_datasets_per_condition=2,
            exact_draws=5,
        )
        second = build_generative_report(
            point_replicates=8,
            exact_datasets_per_condition=2,
            exact_draws=5,
        )
        self.assertEqual(first, second)
        self.assertTrue(first["passed_work_checks"])
        self.assertFalse(first["receipt_eligible"])
        self.assertFalse(first["qualification_role_satisfied"])

    def test_adversarial_matrix_has_exact_required_cases_and_all_pass(self) -> None:
        required = {
            "zero_variance_indicator",
            "duplicated_indicator_singular_observed",
            "non_finite_raw_cell",
            "n_less_than_p_singular_correlation",
            "non_positive_definite_target",
            "nonrecursive_cycle",
            "duplicate_structural_path",
            "incomplete_explicit_index_plans",
            "out_of_range_sample_index",
            "positive_affine_extreme_scale_invariance",
            "row_permutation_invariance",
            "near_collinear_finite_or_typed_failure",
            "fixed_failure_ledger_below_ninety_percent",
            "frozen_product_scalar_tamper_detected",
        }
        observed = {row["case_id"] for row in self.adversarial["cases"]}
        self.assertEqual(observed, required)
        self.assertEqual(self.adversarial["case_count"], len(required))
        self.assertTrue(all(row["passed"] for row in self.adversarial["cases"]))

    def test_evidence_source_imports_no_product_module(self) -> None:
        tree = ast.parse(SOURCE.read_text(encoding="utf-8"))
        roots = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                roots.update(alias.name.split(".")[0] for alias in node.names)
            elif isinstance(node, ast.ImportFrom) and node.module:
                roots.add(node.module.split(".")[0])
        self.assertFalse(
            roots
            & {
                "qpls",
                "qpls_assessment",
                "qpls_core",
                "qpls_estimation",
                "qpls_project",
                "qpls_resampling",
                "qpls_runner",
            }
        )

    def test_adversarial_builder_is_deterministic_and_non_promotional(self) -> None:
        first = build_adversarial_report()
        second = build_adversarial_report()
        self.assertEqual(first, second)
        self.assertTrue(first["passed_work_checks"])
        self.assertFalse(first["receipt_eligible"])
        self.assertIn(
            "adversarial.product_engine_exact_fit_boundary_run_not_captured",
            first["blockers"],
        )


if __name__ == "__main__":
    unittest.main()
