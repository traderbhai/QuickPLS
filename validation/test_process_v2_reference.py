#!/usr/bin/env python3
"""Pure unit checks for the independent PROCESS v2 reference.

These tests never execute QuickPLS, Cargo, a browser, or R. They freeze the
scientific identities that the runtime qualification generator later compares.
"""

from __future__ import annotations

import math
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import process_v2_reference as reference


class ProcessV2ReferenceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.temporary = tempfile.TemporaryDirectory(prefix="quickpls-process-v2-unit-")
        cls.fixture = Path(cls.temporary.name) / "process-v2.csv"
        reference.write_fixture(cls.fixture)
        cls.columns, cls.total_rows = reference.complete_case_columns(cls.fixture)
        cls.profiles = reference.variable_profiles(cls.columns)
        cls.graph = reference.reference_graph(
            cls.columns,
            raw_probe_profiles=cls.profiles,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary.cleanup()

    def test_fixture_uses_one_global_listwise_sample_and_exact_binary_coding(self) -> None:
        self.assertEqual(self.total_rows, 180)
        self.assertEqual(len(self.columns[reference.OUTCOME]), 175)
        self.assertEqual(set(np.unique(self.columns["B"])), {0.0, 1.0})
        self.assertEqual(self.graph["complete_cases"], 175)
        self.assertEqual(len(self.graph["variable_profiles"]), 9)

    def test_typed_recipe_freezes_graph_without_numbered_template_metadata(self) -> None:
        recipe = reference.recipe_payload("sha256:fixture", workers=4, samples=999)
        config = recipe["method_config"]
        relationship = config["model"]["relationship"]
        self.assertEqual(recipe["schema_version"], 3)
        self.assertEqual(config["kind"], "regression")
        self.assertEqual(config["model"]["type"], "process")
        self.assertEqual(relationship["model"], "graph")
        self.assertEqual(relationship["continuous_product_centering"], "equation_complete_case_mean_v1")
        self.assertEqual(len(relationship["paths"]), 8)
        self.assertEqual(len(relationship["moderators"]), 2)
        self.assertEqual(len(relationship["moderations"]), 3)
        self.assertEqual(config["bootstrap"], {"algorithm": "case_resampling", "intervals": ["percentile", "bca"]})
        self.assertEqual(recipe["metadata"]["status"], "validated_regression_process_v2_plus_bootstrap_v1_bounded_scope")
        self.assertNotIn("process_model", recipe["metadata"])

    def test_equation_and_term_order_matches_canonical_contract(self) -> None:
        self.assertEqual(
            [equation["outcome"] for equation in self.graph["equations"]],
            ["M1", "M2", "M3", "M4", "Y"],
        )
        by_outcome = {equation["outcome"]: equation for equation in self.graph["equations"]}
        self.assertEqual(
            by_outcome["M4"]["term_ids"],
            ["intercept", "path:X->M4", "control:C"],
        )
        self.assertEqual(
            by_outcome["Y"]["term_ids"],
            [
                "intercept", "path:X->Y", "path:M2->Y", "path:M3->Y", "path:M4->Y",
                "moderator:W", "moderator:B", "interaction:M4*B", "interaction:W*B",
                "interaction:X*B", "interaction:X*W", "interaction:X*W*B", "control:C",
            ],
        )
        self.assertTrue(all(equation["residual_degrees_of_freedom"] > 0 for equation in self.graph["equations"]))
        self.assertTrue(all(np.all(np.isfinite(equation["coefficient_covariance"])) for equation in self.graph["equations"]))
        for equation in self.graph["equations"]:
            fit = equation["fit"]
            self.assertEqual(fit["observations"] - fit["parameter_count"], equation["residual_degrees_of_freedom"])
            self.assertAlmostEqual(
                fit["r_squared"],
                1.0 - fit["residual_sum_squares"] / fit["total_sum_squares"],
                places=12,
            )

    def test_optional_conditioning_moderator_matches_wire_omission_contract(self) -> None:
        by_id = {
            moderation["moderation_id"]: moderation
            for moderation in self.graph["moderations"]
        }
        self.assertEqual(
            set(by_id["moderation:X->M3@W"]),
            {"moderation_id", "from", "to", "moderator"},
        )
        self.assertEqual(
            by_id["moderation:X->Y@W|B"]["conditioning_moderator"],
            "B",
        )
        self.assertEqual(
            set(by_id["moderation:M4->Y@B"]),
            {"moderation_id", "from", "to", "moderator"},
        )

    def test_hc3_rejects_near_unit_leverage_without_clamping(self) -> None:
        self.assertTrue(reference.hc3_high_leverage_boundary_check())
        stable = reference.hc3_scaled_residuals(
            np.asarray([2.0]),
            np.asarray([1.0 - 2.0e-12]),
            equation_id="equation:stable",
        )
        self.assertAlmostEqual(stable[0], 1.0e12, delta=5.0e7)
        for leverage in (1.0 - 1.0e-12, 1.0, math.nan, math.inf):
            with self.assertRaises(reference.ProcessReferenceFitError) as caught:
                reference.hc3_scaled_residuals(
                    np.asarray([1.0]),
                    np.asarray([leverage]),
                    equation_id="equation:unstable",
                )
            self.assertEqual(caught.exception.reason_code, "high_leverage_hc3_instability")
            self.assertTrue(str(caught.exception).startswith("high_leverage_hc3_instability|"))

    def test_hc3_and_simple_slope_variances_fail_closed_without_abs_or_clamp(self) -> None:
        self.assertTrue(reference.hc3_covariance_diagonal_boundary_check())
        self.assertTrue(reference.simple_slope_variance_boundary_check())
        for variance in (0.0, -1e-12, math.nan, math.inf):
            with self.assertRaises(reference.ProcessReferenceFitError) as caught:
                reference.validate_hc3_covariance_diagonal(
                    np.asarray([[variance]]), equation_id="equation:invalid"
                )
            self.assertEqual(caught.exception.reason_code, "invalid_hc3_covariance")
            with self.assertRaises(reference.ProcessReferenceFitError) as caught:
                reference.linear_combination(
                    {
                        "coefficients": [{"estimate": 1.0}],
                        "coefficient_covariance": [[variance]],
                    },
                    np.asarray([1.0]),
                    moderation_id="moderation:invalid",
                )
            self.assertEqual(caught.exception.reason_code, "degenerate_simple_slope_variance")

    def test_scale_aware_solver_is_unit_invariant_and_rejects_relative_collinearity(self) -> None:
        contract = reference.scale_aware_solver_boundary_check()
        self.assertTrue(contract["passed"], contract)
        self.assertEqual(
            contract["normalization"],
            "non_intercept_welford_mean_population_rms_v1",
        )
        self.assertEqual(
            contract["rank_rule"],
            "s_min_gt_s_max_times_max_n_p_times_epsilon_times_100",
        )
        self.assertTrue(contract["relative_collinearity_rejected"])

    def test_johnson_neyman_solver_is_affine_stable_and_invalid_covariance_is_tagged(self) -> None:
        roots = reference.johnson_neyman_root_solver_boundary_check()
        invalid = reference.johnson_neyman_invalid_covariance_boundary_check()
        self.assertTrue(roots["passed"], roots)
        self.assertEqual(roots["exact_double_root_count"], 1)
        self.assertEqual(roots["resolvable_near_double_root_count"], 2)
        self.assertEqual(roots["coefficient_tolerance_multiplier"], 64.0)
        self.assertEqual(roots["root_deduplication_tolerance_multiplier"], 128.0)
        self.assertTrue(invalid["passed"], invalid)
        self.assertEqual(invalid["reason_code"], "invalid_hc3_covariance")
        self.assertEqual(invalid["message"], reference.JN_INVALID_COVARIANCE_MESSAGE)

    def test_binary_endogenous_scope_and_reference_condition_are_frozen(self) -> None:
        binary = reference.binary_endogenous_outcome_boundary_check()
        collapsed = reference.collapsed_probe_grid_boundary_check()
        condition = reference.reference_condition_policy_check(self.graph)
        self.assertTrue(binary["passed"], binary)
        self.assertEqual(binary["rejected_outcomes"], ["M1", "Y"])
        self.assertTrue(binary["original_sample_only"])
        self.assertTrue(collapsed["passed"], collapsed)
        self.assertEqual(collapsed["reason_code"], "collapsed_process_probe_grid")
        self.assertEqual(
            collapsed["semantic_assignment"],
            "canonical_grid_index_primary_outer_conditioning_inner",
        )
        self.assertTrue(condition["passed"], condition)
        self.assertEqual(condition["column"], "Reference condition")
        self.assertEqual(condition["value"], reference.REFERENCE_CONDITION)
        self.assertEqual(condition["continuous_coded_value"], 0.0)
        self.assertEqual(condition["binary_raw_value"], 0.0)

    def test_point_reference_is_row_irrelevant_column_and_path_order_invariant(self) -> None:
        result = reference.point_metamorphic_invariance(self.columns)
        self.assertTrue(result["passed"], result)
        self.assertLessEqual(
            result["row_order_maximum_absolute_difference"],
            reference.ARITHMETIC_TOLERANCE,
        )
        self.assertEqual(result["irrelevant_column_maximum_absolute_difference"], 0.0)
        self.assertTrue(result["path_order_canonicalized"])

    def test_worker_comparison_ignores_only_declared_worker_metadata_and_reports_drift(self) -> None:
        serial = {"bootstrap": {"workers": 1, "estimands": [{"estimate": 1.0}]}}
        parallel = {"bootstrap": {"workers": 4, "estimands": [{"estimate": 1.0}]}}
        equal = reference.worker_invariance_comparison(serial, parallel)
        self.assertTrue(equal["passed"], equal)
        self.assertEqual(equal["workers_compared"], [1, 4])
        self.assertEqual(equal["mismatch_count"], 0)
        self.assertEqual(equal["first_mismatches"], [])
        self.assertEqual(
            equal["serial_normalized_sha256"],
            equal["parallel_normalized_sha256"],
        )

        parallel["bootstrap"]["estimands"][0]["estimate"] = 1.0 + 1.0e-12
        drift = reference.worker_invariance_comparison(serial, parallel)
        self.assertFalse(drift["passed"])
        self.assertEqual(drift["mismatch_count"], 1)
        self.assertEqual(
            drift["first_mismatches"][0]["path"],
            "graph_v2.bootstrap.estimands[0].estimate",
        )
        self.assertGreater(drift["maximum_numeric_difference"], 0.0)

    def test_full_graph_covers_every_frozen_effect_family(self) -> None:
        self.assertEqual(len(self.graph["reference_effects"]), 6)
        self.assertEqual(len(self.graph["conditional_indirect_effects"]), 5)
        self.assertEqual(len(self.graph["moderated_mediation_indices"]), 2)
        self.assertEqual(len(self.graph["simple_slopes"]), 11)
        self.assertEqual(len(self.graph["plots"]), 3)
        self.assertEqual(len(self.graph["johnson_neyman"]), 4)
        self.assertEqual(
            [row["status"] for row in self.graph["johnson_neyman"]].count("unavailable"),
            1,
        )
        unavailable = next(row for row in self.graph["johnson_neyman"] if row["status"] == "unavailable")
        self.assertEqual(unavailable["reason_code"], "binary_solved_moderator")
        ids, values = reference.estimand_vector(self.graph)
        self.assertEqual(len(ids), 24)
        self.assertEqual(len(ids), len(set(ids)))
        self.assertIn("slope:moderation:X->M3@W@W=mean", ids)
        self.assertIn("slope:moderation:X->Y@W|B@W=mean,B=binary_0", ids)
        self.assertIn("slope:moderation:M4->Y@B@B=binary_1", ids)
        self.assertTrue(all("." not in effect_id.split("@")[-1] for effect_id in ids if "@" in effect_id))
        self.assertTrue(np.all(np.isfinite(values)))

    def test_resamples_recenter_fits_but_keep_original_raw_probe_grid_and_ids(self) -> None:
        original_ids, _ = reference.estimand_vector(self.graph)
        # Shift the resample composition enough to move W's fitted mean/SD.
        indexes = np.concatenate((np.arange(0, 80), np.arange(0, 80), np.arange(80, 95)))
        resampled = {name: values[indexes] for name, values in self.columns.items()}
        resample_profiles = reference.variable_profiles(resampled)
        self.assertNotAlmostEqual(resample_profiles["W"]["raw_mean"], self.profiles["W"]["raw_mean"])
        fitted = reference.reference_graph(
            resampled,
            raw_probe_profiles=self.profiles,
            include_diagnostics=False,
            enforce_original_outcome_scope=False,
        )
        observed_ids, _ = reference.estimand_vector(fitted)
        self.assertEqual(observed_ids, original_ids)
        w_rows = [
            row for row in fitted["conditional_indirect_effects"]
            if row["path_id"] == "X->M3->Y"
        ]
        expected_raw = reference.probe_values(self.profiles["W"])
        self.assertEqual([row["moderator_values"][0]["raw_value"] for row in w_rows], expected_raw)
        for row in w_rows:
            value = row["moderator_values"][0]
            self.assertAlmostEqual(
                value["coded_value"],
                value["raw_value"] - resample_profiles["W"]["raw_mean"],
                places=12,
            )

        delete_one = {name: np.delete(values, 0) for name, values in self.columns.items()}
        delete_one_profiles = reference.variable_profiles(delete_one)
        delete_one_graph = reference.reference_graph(
            delete_one,
            raw_probe_profiles=self.profiles,
            include_diagnostics=False,
            enforce_original_outcome_scope=False,
        )
        delete_one_ids, _ = reference.estimand_vector(delete_one_graph)
        self.assertEqual(delete_one_ids, original_ids)
        delete_one_w_rows = [
            row for row in delete_one_graph["conditional_indirect_effects"]
            if row["path_id"] == "X->M3->Y"
        ]
        self.assertEqual(
            [row["moderator_values"][0]["raw_value"] for row in delete_one_w_rows],
            expected_raw,
        )
        for row in delete_one_w_rows:
            value = row["moderator_values"][0]
            self.assertAlmostEqual(
                value["coded_value"],
                value["raw_value"] - delete_one_profiles["W"]["raw_mean"],
                places=12,
            )

    def test_type7_and_bca_arithmetic_are_frozen(self) -> None:
        self.assertAlmostEqual(reference.type7([1.0, 2.0, 4.0, 8.0], 0.25), 1.75)
        self.assertAlmostEqual(reference.type7([1.0, 2.0, 4.0, 8.0], 0.50), 3.0)
        result = reference.bca(
            [0.9, 1.0, 1.1, 1.2, 1.3, 1.4],
            1.15,
            [1.02, 1.08, 1.11, 1.19, 1.25, 1.30],
        )
        self.assertEqual(result["status"], "available")
        self.assertTrue(all(math.isfinite(result[name]) for name in ("bias_correction", "acceleration", "lower", "upper")))
        self.assertLess(result["lower"], result["upper"])
        unavailable = reference.bca([1.0, 1.1, 1.2], 1.1, [2.0, 2.0, 2.0])
        self.assertEqual(unavailable, {"status": "unavailable", "reason_code": "zero_jackknife_variance"})


if __name__ == "__main__":
    unittest.main()
