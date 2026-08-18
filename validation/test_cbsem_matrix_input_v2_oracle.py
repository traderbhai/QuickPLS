#!/usr/bin/env python3
"""Focused mutation-oriented tests for CB-SEM matrix-input oracle work."""

from __future__ import annotations

import json
import math
import sys
import unittest
from pathlib import Path

import numpy as np


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import cbsem_matrix_input_v2_oracle as oracle  # noqa: E402


class CbsemMatrixInputV2OracleTests(unittest.TestCase):
    def test_checked_in_work_report_is_current_but_never_a_receipt(self) -> None:
        checked_in = json.loads(oracle.REPORT_PATH.read_text(encoding="utf-8"))
        self.assertEqual(checked_in, oracle.build_report())
        self.assertTrue(checked_in["passed_work_checks"])
        self.assertFalse(checked_in["qualification_role_satisfied"])
        self.assertFalse(checked_in["receipt_eligible"])
        self.assertFalse(checked_in["promotion_requested"])
        self.assertTrue(checked_in["blockers"])
        self.assertFalse(checked_in["reference"]["product_code_imported"])
        self.assertFalse(checked_in["reference"]["product_executable_invoked"])

    def test_analytic_gradient_is_independently_audited(self) -> None:
        covariance = oracle.population_covariance(oracle.POPULATIONS[1])
        hand = oracle.closed_form_three_indicator(covariance)
        parameters = oracle._raw_from_parameters(hand)  # noqa: SLF001
        parameters += np.asarray((0.11, -0.07, 0.08, -0.03, 0.05, -0.04))
        analytic = oracle.ml_gradient(covariance, parameters)
        numeric = oracle.central_difference_gradient(
            lambda values: oracle.ml_discrepancy(covariance, values), parameters
        )
        self.assertLessEqual(
            float(np.max(np.abs(analytic - numeric))),
            oracle.GRADIENT_AUDIT_TOLERANCE,
        )
        mutated = analytic.copy()
        mutated[0] += oracle.GRADIENT_AUDIT_TOLERANCE * 2.0
        self.assertGreater(
            float(np.max(np.abs(mutated - numeric))),
            oracle.GRADIENT_AUDIT_TOLERANCE,
        )

    def test_raw_sample_covariance_and_scaled_correlation_share_ml_moment(self) -> None:
        sample_size = 192
        raw = oracle.generate_raw(oracle.POPULATIONS[0], sample_size, 2026081511)
        expected = oracle.canonical_raw_covariance(raw)
        sample_covariance = np.cov(raw, rowvar=False, ddof=1)
        covariance = oracle.canonical_matrix_covariance(
            sample_covariance,
            kind="covariance",
            declared_sample_size=sample_size,
            dataset_sample_size=sample_size,
            denominator="sample_n_minus_one",
        )
        scales = np.std(raw, axis=0, ddof=1)
        correlation = oracle.canonical_matrix_covariance(
            np.corrcoef(raw, rowvar=False),
            kind="correlation",
            declared_sample_size=sample_size,
            dataset_sample_size=sample_size,
            denominator="sample_n_minus_one",
            standard_deviations=scales,
        )
        np.testing.assert_allclose(expected, covariance, atol=2e-12, rtol=0.0)
        np.testing.assert_allclose(expected, correlation, atol=2e-12, rtol=0.0)
        wrong_denominator = oracle.canonical_matrix_covariance(
            sample_covariance,
            kind="covariance",
            declared_sample_size=sample_size,
            dataset_sample_size=sample_size,
            denominator="maximum_likelihood_n",
        )
        self.assertGreater(float(np.max(np.abs(expected - wrong_denominator))), 1e-4)

    def test_population_parameters_and_stable_ids_are_exact(self) -> None:
        population = oracle.POPULATIONS[0]
        covariance = oracle.population_covariance(population)
        estimated = oracle.estimate_one_factor(covariance)
        expected = oracle._population_parameters(population)  # noqa: SLF001
        self.assertTrue(estimated.converged)
        self.assertLessEqual(
            max(abs(estimated.parameters[key] - expected[key]) for key in expected),
            2e-7,
        )
        self.assertEqual(
            oracle.stable_parameter_ids(),
            {
                "construct:f=~x1": "parameter_66_7831",
                "construct:f=~x2": "parameter_66_7832",
                "construct:f=~x3": "parameter_66_7833",
                "construct:f~~construct:f": "variance_66",
                "x1~~x1": "residual_variance_7831",
                "x2~~x2": "residual_variance_7832",
                "x3~~x3": "residual_variance_7833",
            },
        )

    def test_adversarial_matrix_failures_are_typed_and_fail_closed(self) -> None:
        expected = {row["id"]: row["expected_code"] for row in oracle.ADVERSARIAL_SCENARIOS}
        self.assertEqual(
            {name: oracle._typed_failure(name) for name in expected},  # noqa: SLF001
            expected,
        )
        covariance = oracle.population_covariance(oracle.POPULATIONS[0])
        invalid = covariance.copy()
        invalid[0, 1] = invalid[1, 0] = math.sqrt(
            invalid[0, 0] * invalid[1, 1]
        ) * 1.01
        with self.assertRaisesRegex(oracle.OracleInputError, "positive definite") as caught:
            oracle.canonical_matrix_covariance(
                invalid,
                kind="covariance",
                declared_sample_size=120,
                dataset_sample_size=120,
                denominator="maximum_likelihood_n",
            )
        self.assertEqual(caught.exception.code, "MatrixNotPositiveDefinite")

    def test_second_reference_probe_never_installs_or_claims_missing_runtime(self) -> None:
        result = oracle._probe_lavaan()  # noqa: SLF001
        self.assertFalse(result["installation_attempted"])
        if not result["available"]:
            self.assertIn(result["reason"], {"Rscript_not_installed", "lavaan_not_installed"})


if __name__ == "__main__":
    unittest.main()
