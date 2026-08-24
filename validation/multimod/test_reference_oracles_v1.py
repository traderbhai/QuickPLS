"""Unit checks for the frozen MultiMod independent-reference identities.

These tests validate the oracle and immutable expected values only.  They do
not invoke QuickPLS and therefore cannot, on their own, promote a capability.
"""

from __future__ import annotations

import json
import unittest
from pathlib import Path
from typing import Any

from reference_oracles_v1 import (
    adjust_probabilities,
    assess_observed_treatment_positivity,
    compile_path_polynomial,
    conditional_path_derivative,
    conditional_path_effect,
    conditional_probe_contrast,
    exhaustive_pairwise_slope_permutation,
    fimix_gaussian_identities,
    interventional_g_computation,
)


FIXTURE_PATH = Path(__file__).with_name("fixtures") / "reference_oracles_v1.json"
FIXTURE = json.loads(FIXTURE_PATH.read_text(encoding="utf-8"))
TOLERANCE = float(FIXTURE["provenance"]["numeric_tolerance"])


class FrozenReferenceTestCase(unittest.TestCase):
    def assert_close(self, actual: float, expected: float, message: str = "") -> None:
        self.assertAlmostEqual(actual, expected, delta=TOLERANCE, msg=message)

    def assert_vector_close(
        self, actual: list[float], expected: list[float], message: str = ""
    ) -> None:
        self.assertEqual(len(actual), len(expected), msg=message)
        for index, (actual_value, expected_value) in enumerate(zip(actual, expected, strict=True)):
            self.assert_close(actual_value, expected_value, f"{message} at index {index}")


class MgaPermutationOracleTests(FrozenReferenceTestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.case = FIXTURE["mga"]["pairwise_slope_permutation"]

    def calculate(self, reverse: bool = False) -> dict[str, Any]:
        group_a = self.case["group_b_indices"] if reverse else self.case["group_a_indices"]
        group_b = self.case["group_a_indices"] if reverse else self.case["group_b_indices"]
        return exhaustive_pairwise_slope_permutation(
            self.case["x"], self.case["y"], group_a, group_b
        )

    def test_signed_pairwise_permutation_matches_frozen_exhaustive_counts(self) -> None:
        actual = self.calculate()
        for identity, expected in self.case["expected"].items():
            if isinstance(expected, float):
                self.assert_close(actual[identity], expected, identity)
            else:
                self.assertEqual(actual[identity], expected, identity)
        self.assertEqual(
            actual["usable_partitions"],
            len(actual["permuted_differences"]),
        )

    def test_label_reversal_negates_signed_difference_and_swaps_directional_tails(self) -> None:
        original = self.calculate()
        reversed_result = self.calculate(reverse=True)
        expected = self.case["expected_after_label_reversal"]
        for identity, expected_value in expected.items():
            self.assert_close(reversed_result[identity], expected_value, identity)
        self.assert_close(
            reversed_result["difference_a_minus_b"],
            -original["difference_a_minus_b"],
        )
        self.assert_close(reversed_result["p_value_two_sided"], original["p_value_two_sided"])
        self.assert_close(reversed_result["p_value_greater"], original["p_value_less"])
        self.assert_close(reversed_result["p_value_less"], original["p_value_greater"])


class MgaMultiplicityOracleTests(FrozenReferenceTestCase):
    def test_all_frozen_adjustments_match_hand_values(self) -> None:
        case = FIXTURE["mga"]["multiplicity"]
        for method, expected in case["expected"].items():
            actual = adjust_probabilities(case["hypotheses"], method)
            self.assertEqual(set(actual), set(expected), method)
            for identity, expected_value in expected.items():
                self.assert_close(actual[identity], expected_value, f"{method}.{identity}")


class ConditionalPolynomialOracleTests(FrozenReferenceTestCase):
    @staticmethod
    def expected_polynomial(
        moderators: list[str], terms: list[dict[str, Any]]
    ) -> dict[tuple[int, ...], float]:
        return {
            tuple(term["powers"].get(moderator, 0) for moderator in moderators): float(
                term["coefficient"]
            )
            for term in terms
        }

    def test_explicit_path_products_match_frozen_polynomials(self) -> None:
        for case in FIXTURE["conditional_process"]["cases"]:
            with self.subTest(case_id=case["case_id"]):
                actual = compile_path_polynomial(case["moderators"], case["edges"])
                expected = self.expected_polynomial(
                    case["moderators"], case["expected_polynomial"]
                )
                self.assertEqual(set(actual), set(expected))
                for exponent, expected_value in expected.items():
                    self.assert_close(actual[exponent], expected_value, str(exponent))

    def test_conditional_effects_derivatives_and_left_minus_right_contrasts(self) -> None:
        for case in FIXTURE["conditional_process"]["cases"]:
            moderators = case["moderators"]
            polynomial = compile_path_polynomial(moderators, case["edges"])
            probes = {probe["id"]: probe for probe in case["probes"]}
            for probe in case["probes"]:
                with self.subTest(case_id=case["case_id"], probe=probe["id"]):
                    effect = conditional_path_effect(polynomial, moderators, probe["values"])
                    self.assert_close(effect, probe["expected_effect"])
            for derivative in case["derivatives"]:
                with self.subTest(
                    case_id=case["case_id"],
                    probe=derivative["probe_id"],
                    orders=derivative["orders"],
                ):
                    estimate = conditional_path_derivative(
                        polynomial,
                        moderators,
                        probes[derivative["probe_id"]]["values"],
                        derivative["orders"],
                    )
                    self.assert_close(estimate, derivative["expected"])
            for contrast in case["contrasts"]:
                with self.subTest(case_id=case["case_id"], contrast=contrast):
                    estimate = conditional_probe_contrast(
                        polynomial,
                        moderators,
                        probes[contrast["left_probe_id"]]["values"],
                        probes[contrast["right_probe_id"]]["values"],
                    )
                    self.assert_close(estimate, contrast["expected_left_minus_right"])


class InterventionalGComputationOracleTests(FrozenReferenceTestCase):
    def test_binary_and_continuous_observed_linear_targets(self) -> None:
        cases = FIXTURE["interventional_mediation"]["known_targets"]
        self.assertEqual(
            {case["contrast"]["kind"] for case in cases},
            {"binary", "continuous_contrast"},
        )
        for case in cases:
            with self.subTest(case_id=case["case_id"]):
                actual = interventional_g_computation(case)
                for identity, expected in case["expected"].items():
                    self.assert_close(actual[identity], expected, identity)
                self.assert_close(
                    actual["total_interventional_contrast"],
                    actual["interventional_direct_effect"]
                    + actual["joint_interventional_indirect_effect"],
                )


class PositivityOracleTests(FrozenReferenceTestCase):
    def test_observed_binary_and_continuous_support_receipts_and_failures(self) -> None:
        cases = FIXTURE["interventional_mediation"]["positivity_cases"]
        self.assertTrue(any(not case["expected"]["eligible"] for case in cases))
        for case in cases:
            with self.subTest(case_id=case["case_id"]):
                actual = assess_observed_treatment_positivity(
                    case["treatment_values"], case["contrast"], case["policy"]
                )
                expected = case["expected"]
                self.assertEqual(actual["eligible"], expected["eligible"])
                self.assertEqual(actual["blocker_codes"], expected["blocker_codes"])
                self.assertEqual(actual["x0_support_count"], expected["x0_support_count"])
                self.assertEqual(actual["x1_support_count"], expected["x1_support_count"])
                self.assertIn("does not prove causal positivity", actual["wording"])


class FimixIdentityOracleTests(FrozenReferenceTestCase):
    def test_likelihood_posteriors_full_parameter_criteria_and_entropy(self) -> None:
        case = FIXTURE["fimix"]
        expected = case["expected"]
        actual = fimix_gaussian_identities(case)

        for actual_row, expected_row in zip(
            actual["log_joint"], expected["log_joint"], strict=True
        ):
            self.assert_vector_close(actual_row, expected_row, "log_joint")
        self.assert_vector_close(
            actual["log_normalizers"], expected["log_normalizers"], "log_normalizers"
        )
        for actual_row, expected_row in zip(
            actual["posteriors"], expected["posteriors"], strict=True
        ):
            self.assert_vector_close(actual_row, expected_row, "posteriors")
            self.assert_close(sum(actual_row), 1.0, "posterior row sum")

        self.assert_close(actual["log_likelihood"], expected["log_likelihood"])
        self.assertEqual(actual["parameter_count"], expected["parameter_count"])
        for identity, expected_value in expected["criteria"].items():
            self.assert_close(actual["criteria"][identity], expected_value, identity)
        for identity, expected_value in expected["entropy"].items():
            self.assert_close(actual["entropy"][identity], expected_value, identity)


if __name__ == "__main__":
    unittest.main(verbosity=2)
