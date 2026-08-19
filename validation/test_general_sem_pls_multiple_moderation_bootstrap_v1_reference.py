from __future__ import annotations

import ast
import importlib.util
import math
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
REFERENCE_PATH = (
    ROOT / "validation/general_sem_pls_multiple_moderation_bootstrap_v1_reference.py"
)
SPEC = importlib.util.spec_from_file_location("moderation_bootstrap_reference", REFERENCE_PATH)
assert SPEC is not None and SPEC.loader is not None
REFERENCE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = REFERENCE
SPEC.loader.exec_module(REFERENCE)


class GeneralSemPlsMultipleModerationBootstrapReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.report = REFERENCE.run_reference()

    def test_reference_is_passing_observed_score_evidence_without_promotion(self):
        self.assertTrue(self.report["passed"])
        self.assertEqual(
            self.report["reference_scope"],
            "independent_observed_score_gamma_only_smoke_v1",
        )
        self.assertFalse(self.report["qualification_ready"])
        self.assertFalse(self.report["promotion_allowed"])
        self.assertTrue(
            any(
                "outside this reference" in item or "does not establish" in item
                for item in self.report["limitations"]
            )
        )

    def test_same_and_different_focal_equations_refit_complete_joint_contract(self):
        scenarios = {row["layout"]: row for row in self.report["scenarios"]}
        self.assertEqual(set(scenarios), {"same_focal", "different_focal"})
        for scenario in scenarios.values():
            self.assertEqual(scenario["joint_interaction_count"], 2)
            self.assertEqual(scenario["resamples_usable"], 199)
            self.assertTrue(scenario["full_joint_point_contract_validated_per_replicate"])
            self.assertTrue(scenario["ordinary_coefficients_and_fixed_probe_slopes_point_only"])

    def test_only_scientific_rescaled_gamma_has_inference_rows(self):
        rows = self.report["gamma_inference"]
        self.assertEqual(len(rows), 4)
        self.assertEqual(len({row["target"]["target_id"] for row in rows}), 4)
        self.assertTrue(
            all(
                row["target"]["kind"] == "interaction_scientific_rescaled_gamma"
                for row in rows
            )
        )
        forbidden = {"ordinary_inference", "standardized_product_beta_inference", "slope_inference", "plot_inference"}
        self.assertTrue(forbidden.isdisjoint(self.report))

    def test_type7_b_minus_one_plus_one_and_exact_gate_microcases(self):
        self.assertAlmostEqual(
            REFERENCE.type7_quantile([1.0, 2.0, 4.0, 8.0], 0.25), 1.75
        )
        summary = REFERENCE.summarize_gamma(
            REFERENCE.Target("t", "i", "r", "X", "W", "Y"),
            2.0,
            [1.0, 2.0, 4.0],
        )
        self.assertAlmostEqual(summary["standard_error"], math.sqrt(7.0 / 3.0))
        self.assertEqual(summary["two_sided_exceedances"], 1)
        self.assertEqual(summary["p_value_two_sided"], 0.5)
        self.assertEqual(REFERENCE.minimum_usable_replicates(20), 18)
        self.assertEqual(REFERENCE.minimum_usable_replicates(2), 2)

    def test_sign_alignment_product_rescaling_replay_and_failures_are_checked(self):
        checks = self.report["checks"]
        for key in (
            "score_vector_sign_alignment_precedes_products",
            "product_scale_recomputed_per_replicate",
            "indexed_replay_and_evaluation_order_invariant",
            "constant_product_rejected",
            "singular_joint_equation_rejected",
            "exact_ninety_percent_gate_accepts_18_of_20",
            "exact_ninety_percent_gate_rejects_17_of_20",
        ):
            self.assertTrue(checks[key], key)
        self.assertEqual(self.report["metrics"]["maximum_replay_difference"], 0.0)
        self.assertGreater(self.report["metrics"]["minimum_product_scale_range"], 0.0)

    def test_reference_imports_only_python_standard_library(self):
        tree = ast.parse(REFERENCE_PATH.read_text(encoding="utf-8"))
        imported_roots: set[str] = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                imported_roots.update(alias.name.split(".")[0] for alias in node.names)
            elif isinstance(node, ast.ImportFrom) and node.module:
                imported_roots.add(node.module.split(".")[0])
        self.assertTrue(imported_roots <= sys.stdlib_module_names | {"__future__"})


if __name__ == "__main__":
    unittest.main()
