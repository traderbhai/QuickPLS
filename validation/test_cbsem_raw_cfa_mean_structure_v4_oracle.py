#!/usr/bin/env python3
"""Focused tests for the independent CB-SEM mean-structure V4 oracle."""

from __future__ import annotations

import copy
import json
import math
import sys
import unittest
from pathlib import Path

import numpy as np


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import cbsem_raw_cfa_mean_structure_v4_oracle as oracle  # noqa: E402


def numerical_likelihood_hessian(
    estimate: oracle.OracleEstimate,
) -> np.ndarray:
    """Central-difference the full -2 log-likelihood/2 at the exact fit."""

    parameters = np.asarray(
        [estimate.parameters[name] for name in oracle.FREE_PARAMETER_NAMES]
    )
    steps = 2e-4 * np.maximum(np.abs(parameters), 1.0)

    def likelihood(values: np.ndarray) -> float:
        sigma, means = oracle.moments_from_free_parameters(values)
        return (
            0.5
            * estimate.sample_size
            * oracle.joint_ml_discrepancy(
                estimate.sample_covariance_ml,
                estimate.sample_means,
                sigma,
                means,
            )
        )

    size = parameters.size
    result = np.empty((size, size), dtype=np.float64)
    base = likelihood(parameters)
    for row in range(size):
        row_step = np.zeros(size)
        row_step[row] = steps[row]
        result[row, row] = (
            likelihood(parameters + row_step)
            - 2.0 * base
            + likelihood(parameters - row_step)
        ) / steps[row] ** 2
        for column in range(row):
            column_step = np.zeros(size)
            column_step[column] = steps[column]
            value = (
                likelihood(parameters + row_step + column_step)
                - likelihood(parameters + row_step - column_step)
                - likelihood(parameters - row_step + column_step)
                + likelihood(parameters - row_step - column_step)
            ) / (4.0 * steps[row] * steps[column])
            result[row, column] = result[column, row] = value
    return result


class CbsemRawCfaMeanStructureV4OracleTests(unittest.TestCase):
    def test_preregistered_product_input_uses_ml_n_moments_and_joint_fit(self) -> None:
        estimate = oracle.estimate_marker_cfa_mean_structure(
            oracle.deterministic_product_rows()
        )
        self.assertEqual(estimate.sample_size, 40)
        np.testing.assert_allclose(
            estimate.sample_means,
            oracle.EXPECTED_FIXTURE_MEANS,
            atol=2e-14,
            rtol=0.0,
        )
        np.testing.assert_allclose(
            estimate.sample_covariance_ml,
            oracle.EXPECTED_FIXTURE_COVARIANCE_ML,
            atol=2e-12,
            rtol=0.0,
        )
        sample_n_minus_one = np.cov(
            oracle.deterministic_product_rows(), rowvar=False, ddof=1
        )
        self.assertGreater(
            float(np.max(np.abs(sample_n_minus_one - estimate.sample_covariance_ml))),
            1.0,
        )
        np.testing.assert_allclose(
            estimate.implied_means,
            estimate.sample_means,
            atol=2e-12,
            rtol=0.0,
        )
        np.testing.assert_allclose(
            estimate.implied_covariance,
            estimate.sample_covariance_ml,
            atol=2e-10,
            rtol=0.0,
        )
        self.assertLessEqual(abs(estimate.objective), 2e-12)

        perturbed = np.asarray(
            [estimate.parameters[name] for name in oracle.FREE_PARAMETER_NAMES]
        )
        perturbed[[0, 6, 8]] += np.asarray((0.08, -0.3, 0.2))
        sigma, means = oracle.moments_from_free_parameters(perturbed)
        self.assertGreater(
            oracle.joint_ml_discrepancy(
                estimate.sample_covariance_ml,
                estimate.sample_means,
                sigma,
                means,
            ),
            1e-4,
        )

    def test_exact_moment_population_recovers_covariance_and_location_parameters(
        self,
    ) -> None:
        expected = {
            "construct:f=~x1": 1.0,
            "construct:f=~x2": 0.7,
            "construct:f=~x3": 1.2,
            "construct:f~~construct:f": 1.4,
            "x1~~x1": 0.5,
            "x2~~x2": 0.8,
            "x3~~x3": 0.6,
            "x1~1": 0.0,
            "x2~1": 2.0,
            "x3~1": -1.0,
            "construct:f~1": 0.75,
        }
        rows = oracle.exact_moment_rows(
            sample_size=96,
            loadings=(1.0, 0.7, 1.2),
            latent_variance=1.4,
            residual_variances=(0.5, 0.8, 0.6),
            intercepts=(0.0, 2.0, -1.0),
            latent_mean=0.75,
        )
        estimate = oracle.estimate_marker_cfa_mean_structure(rows)
        self.assertLessEqual(
            max(
                abs(estimate.parameters[name] - value)
                for name, value in expected.items()
            ),
            3e-15,
        )
        self.assertTrue(
            all(
                value is None or (np.isfinite(value) and value > 0.0)
                for value in estimate.standard_errors.values()
            )
        )

    def test_expected_information_matches_numerical_likelihood_curvature(self) -> None:
        estimate = oracle.estimate_marker_cfa_mean_structure(
            oracle.deterministic_product_rows()
        )
        information = estimate.expected_information
        np.testing.assert_allclose(information, information.T, atol=0.0, rtol=0.0)
        self.assertGreater(float(np.min(np.linalg.eigvalsh(information))), 0.0)
        numerical = numerical_likelihood_hessian(estimate)
        scaled_error = np.max(
            np.abs(numerical - information) / np.maximum(np.abs(information), 1.0)
        )
        self.assertLessEqual(float(scaled_error), 3e-5)

    def test_row_and_variable_binding_reorder_are_metamorphic(self) -> None:
        rows = oracle.deterministic_product_rows()
        baseline = oracle.estimate_marker_cfa_mean_structure(rows)
        row_reordered = oracle.estimate_marker_cfa_mean_structure(rows[::-1])
        permutation = (2, 0, 1)
        column_reordered = oracle.estimate_marker_cfa_mean_structure(
            rows[:, permutation],
            input_columns=tuple(oracle.VARIABLES[index] for index in permutation),
        )
        for candidate in (row_reordered, column_reordered):
            self.assertEqual(candidate.variables, baseline.variables)
            self.assertLessEqual(
                max(
                    abs(candidate.parameters[name] - baseline.parameters[name])
                    for name in oracle.PARAMETER_NAMES
                ),
                4e-13,
            )
            np.testing.assert_allclose(
                candidate.sample_covariance_ml,
                baseline.sample_covariance_ml,
                atol=4e-13,
                rtol=0.0,
            )

    def test_positive_affine_location_transformation_has_exact_parameter_mapping(
        self,
    ) -> None:
        rows = oracle.deterministic_product_rows()
        baseline = oracle.estimate_marker_cfa_mean_structure(rows)
        scale = np.asarray((1.7, 0.8, 2.1))
        shift = np.asarray((2.2, -3.1, 0.75))
        transformed = oracle.estimate_marker_cfa_mean_structure(rows * scale + shift)
        loading_2 = scale[1] / scale[0] * baseline.parameters["construct:f=~x2"]
        loading_3 = scale[2] / scale[0] * baseline.parameters["construct:f=~x3"]
        expected = {
            "construct:f=~x1": 1.0,
            "construct:f=~x2": loading_2,
            "construct:f=~x3": loading_3,
            "construct:f~~construct:f": scale[0] ** 2
            * baseline.parameters["construct:f~~construct:f"],
            "x1~~x1": scale[0] ** 2 * baseline.parameters["x1~~x1"],
            "x2~~x2": scale[1] ** 2 * baseline.parameters["x2~~x2"],
            "x3~~x3": scale[2] ** 2 * baseline.parameters["x3~~x3"],
            "x1~1": 0.0,
            "x2~1": scale[1] * baseline.parameters["x2~1"]
            + shift[1]
            - loading_2 * shift[0],
            "x3~1": scale[2] * baseline.parameters["x3~1"]
            + shift[2]
            - loading_3 * shift[0],
            "construct:f~1": scale[0] * baseline.parameters["construct:f~1"] + shift[0],
        }
        self.assertLessEqual(
            max(
                abs(transformed.parameters[name] - value)
                for name, value in expected.items()
            ),
            2e-12,
        )
        np.testing.assert_allclose(
            transformed.implied_means,
            baseline.implied_means * scale + shift,
            atol=2e-12,
            rtol=0.0,
        )

    def test_free_anchor_matrix_mean_non_pd_and_broader_shapes_fail_typed(self) -> None:
        expected = {
            "free_marker_intercept": "latent_mean_marker_intercept_must_be_fixed",
            "nonzero_marker_intercept": "latent_mean_marker_intercept_must_be_fixed",
            "matrix_means": "mean_structure_raw_input_required",
            "structural_intercept": "mean_structure_cfa_required",
            "multiple_groups": "mean_structure_single_group_required",
            "ordinal": "mean_structure_continuous_required",
            "weights": "mean_structure_weights_unsupported",
            "non_positive_definite": "sample_covariance_not_positive_definite",
            "underidentified_two_indicator": "local_underidentification",
        }
        self.assertEqual(
            {name: oracle._typed_failure(name) for name in expected},  # noqa: SLF001
            expected,
        )

    def test_identity_quartet_and_stable_parameter_map_are_exact(self) -> None:
        self.assertEqual(oracle.ESTIMATOR_ID, "cbsem_ml_exact_parameter_table_v4")
        self.assertEqual(oracle.MOMENT_ADAPTER_ID, "cbsem_ml_compiled_moment_input_v4")
        self.assertEqual(oracle.MOMENT_RESULT_SCHEMA_VERSION, 3)
        self.assertEqual(
            oracle.RUNNER_ADAPTER_ID,
            "compiled_recipe_v4_cbsem_plan_v2_execution_v3",
        )
        self.assertEqual(set(oracle.STABLE_PARAMETER_IDS), set(oracle.PARAMETER_NAMES))
        self.assertEqual(len(set(oracle.STABLE_PARAMETER_IDS.values())), 11)
        self.assertEqual(oracle.STABLE_PARAMETER_IDS["x1~1"], "parameter:intercept:x1")
        self.assertEqual(
            oracle.STABLE_PARAMETER_IDS["construct:f~1"],
            "parameter:factor_mean:f",
        )

    def test_product_fixture_convergence_fixed_and_se_contract_is_fail_closed(
        self,
    ) -> None:
        fixture = oracle.load_product_fixture()
        baseline = oracle.compare_product_fixture(fixture)
        self.assertTrue(baseline["passed"])
        self.assertTrue(baseline["converged_passed"])
        self.assertTrue(baseline["gradient_within_product_convergence_contract"])

        for name in oracle.PARAMETER_NAMES:
            mutated = copy.deepcopy(fixture)
            row = next(row for row in mutated["parameters"] if row["name"] == name)
            row["fixed"] = not row["fixed"]
            comparison = oracle.compare_product_fixture(mutated)
            self.assertFalse(comparison["passed"])
            check = next(
                row for row in comparison["parameter_checks"] if row["name"] == name
            )
            self.assertFalse(check["fixed_passed"])

        fixed_se = copy.deepcopy(fixture)
        next(row for row in fixed_se["parameters"] if row["name"] == "construct:f=~x1")[
            "standard_error"
        ] = 0.1
        self.assertFalse(oracle.compare_product_fixture(fixed_se)["passed"])

        for invalid_se in (None, 0.0, -0.1):
            free_se = copy.deepcopy(fixture)
            next(
                row for row in free_se["parameters"] if row["name"] == "construct:f=~x2"
            )["standard_error"] = invalid_se
            self.assertFalse(oracle.compare_product_fixture(free_se)["passed"])

        not_converged = copy.deepcopy(fixture)
        not_converged["converged"] = False
        comparison = oracle.compare_product_fixture(not_converged)
        self.assertFalse(comparison["passed"])
        self.assertFalse(comparison["converged_passed"])

        at_gradient_bound = copy.deepcopy(fixture)
        at_gradient_bound["gradient_norm"] = oracle.PRODUCT_GRADIENT_NORM_MAX
        self.assertTrue(oracle.compare_product_fixture(at_gradient_bound)["passed"])

        above_gradient_bound = copy.deepcopy(fixture)
        above_gradient_bound["gradient_norm"] = math.nextafter(
            oracle.PRODUCT_GRADIENT_NORM_MAX, math.inf
        )
        comparison = oracle.compare_product_fixture(above_gradient_bound)
        self.assertFalse(comparison["passed"])
        self.assertFalse(comparison["gradient_within_product_convergence_contract"])

        for invalid_gradient in (-1.0, math.nan, math.inf):
            mutated = copy.deepcopy(fixture)
            mutated["gradient_norm"] = invalid_gradient
            if math.isfinite(invalid_gradient):
                self.assertFalse(oracle.compare_product_fixture(mutated)["passed"])
            else:
                with self.assertRaises(oracle.OracleContractError) as raised:
                    oracle.compare_product_fixture(mutated)
                self.assertEqual(
                    raised.exception.code,
                    "frozen_product_fixture_diagnostics_invalid",
                )

    def test_product_fixture_duplicate_identity_and_unknown_fields_fail_typed(
        self,
    ) -> None:
        fixture = oracle.load_product_fixture()

        reordered = copy.deepcopy(fixture)
        reordered["parameters"].reverse()
        reordered["implied_means"].reverse()
        self.assertTrue(oracle.compare_product_fixture(reordered)["passed"])

        duplicate_parameter = copy.deepcopy(fixture)
        duplicate_parameter["parameters"][-1] = copy.deepcopy(
            duplicate_parameter["parameters"][0]
        )
        with self.assertRaises(oracle.OracleContractError) as raised:
            oracle.compare_product_fixture(duplicate_parameter)
        self.assertEqual(
            raised.exception.code,
            "frozen_product_fixture_parameter_identity_mismatch",
        )

        duplicate_implied_mean = copy.deepcopy(fixture)
        duplicate_implied_mean["implied_means"][-1] = copy.deepcopy(
            duplicate_implied_mean["implied_means"][0]
        )
        with self.assertRaises(oracle.OracleContractError) as raised:
            oracle.compare_product_fixture(duplicate_implied_mean)
        self.assertEqual(
            raised.exception.code,
            "frozen_product_fixture_implied_means_invalid",
        )

        unknown_field = copy.deepcopy(fixture)
        unknown_field["internal_only"] = True
        with self.assertRaises(oracle.OracleContractError) as raised:
            oracle.compare_product_fixture(unknown_field)
        self.assertEqual(
            raised.exception.code,
            "frozen_product_fixture_schema_invalid",
        )

        missing_convergence = copy.deepcopy(fixture)
        del missing_convergence["converged"]
        with self.assertRaises(oracle.OracleContractError) as raised:
            oracle.compare_product_fixture(missing_convergence)
        self.assertEqual(
            raised.exception.code,
            "frozen_product_fixture_schema_invalid",
        )

        for identity_key in oracle.PRODUCT_IDENTITY_KEYS:
            identity_tamper = copy.deepcopy(fixture)
            identity_tamper["identity"][identity_key] = "tampered"
            with self.assertRaises(oracle.OracleContractError) as raised:
                oracle.compare_product_fixture(identity_tamper)
            self.assertEqual(
                raised.exception.code,
                "frozen_product_fixture_identity_mismatch",
            )

        for input_key, value in (
            ("sample_size", oracle.SAMPLE_SIZE + 1),
            ("variable_order", ["x2", "x1", "x3"]),
            ("raw_sha256", "0" * 64),
        ):
            input_tamper = copy.deepcopy(fixture)
            input_tamper["input"][input_key] = value
            with self.assertRaises(oracle.OracleContractError) as raised:
                oracle.compare_product_fixture(input_tamper)
            self.assertEqual(
                raised.exception.code,
                "frozen_product_fixture_input_mismatch",
            )

    def test_missing_product_fixture_is_fail_closed_and_never_a_receipt(self) -> None:
        if oracle.PRODUCT_FIXTURE_PATH.is_file():
            comparison = oracle.compare_product_fixture(oracle.load_product_fixture())
            self.assertTrue(comparison["passed"])
        else:
            with self.assertRaises(oracle.OracleContractError) as raised:
                oracle.load_product_fixture()
            self.assertEqual(raised.exception.code, "frozen_product_fixture_missing")

        checked_in = json.loads(oracle.REPORT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(checked_in, oracle.build_work_report())
        self.assertTrue(checked_in["independent_checks_passed"])
        self.assertFalse(checked_in["qualification_role_satisfied"])
        self.assertFalse(checked_in["receipt_eligible"])
        self.assertFalse(checked_in["promotion_requested"])
        self.assertFalse(checked_in["coverage_or_evidence_state_changed"])
        self.assertTrue(checked_in["blockers"])
        self.assertFalse(checked_in["reference"]["product_code_imported"])
        self.assertFalse(checked_in["reference"]["product_executable_invoked"])
        self.assertFalse(checked_in["reference"]["installation_attempted"])


if __name__ == "__main__":
    unittest.main()
