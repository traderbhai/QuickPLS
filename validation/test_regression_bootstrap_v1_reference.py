#!/usr/bin/env python3
"""Focused unit tests for regression bootstrap v1 reference arithmetic."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

import regression_bootstrap_v1_reference as reference  # noqa: E402


class RegressionBootstrapReferenceTests(unittest.TestCase):
    def test_frozen_supplied_fixture_covers_exact_arithmetic_and_unavailability(self) -> None:
        report = reference.frozen_supplied_check()
        self.assertTrue(report["passed"], report)
        self.assertLessEqual(
            report["maximum_absolute_difference"], reference.EXACT_TOLERANCE
        )
        self.assertEqual(
            report["degenerate_status"]["test"]["reason_code"],
            "degenerate_bootstrap_standard_error",
        )
        self.assertEqual(
            report["incomplete_jackknife_status"]["reason_code"],
            "incomplete_jackknife",
        )

    def test_type7_and_midrank_bca_match_frozen_hand_fixture(self) -> None:
        self.assertEqual(reference.type7_quantile([1.0, 2.0, 4.0, 8.0], 0.25), 1.75)
        interval = reference.bca_interval(
            [1.1, 1.3, 1.7, 1.8, 2.0, 2.1, 2.4, 2.8, 3.0, 3.2],
            2.0,
            [1.85, 1.90, 2.05, 2.10, 2.20, 1.95],
        )
        self.assertIsNotNone(interval)
        assert interval is not None
        self.assertAlmostEqual(interval["bias_correction"], -0.12566134685507402, places=12)
        self.assertAlmostEqual(interval["acceleration"], -0.015853543711576476, places=12)
        self.assertAlmostEqual(interval["lower"], 1.1202082785627896, places=12)
        self.assertAlmostEqual(interval["upper"], 3.112197306363598, places=11)

    def test_witness_partition_requires_exact_ordered_complements(self) -> None:
        bootstrap = {
            "requested_replicates": 3,
            "usable_replicates": 2,
            "jackknife_cases": 3,
            "usable_jackknife_cases": 2,
            "failed_replicates": [
                {"replicate_index": 1, "reason_code": "fit", "message": "failed"}
            ],
            "validation_witness": {
                "method_version": reference.WITNESS_VERSION,
                "terms": reference.TERMS,
                "successful_bootstrap": [
                    {"replicate_index": 0, "coefficients": [1.0, 2.0, 3.0, 4.0]},
                    {"replicate_index": 2, "coefficients": [1.1, 2.1, 3.1, 4.1]},
                ],
                "successful_jackknife": [
                    {"omitted_case": 0, "coefficients": [1.0, 2.0, 3.0, 4.0]},
                    {"omitted_case": 2, "coefficients": [1.1, 2.1, 3.1, 4.1]},
                ],
                "failed_jackknife": [
                    {"omitted_case": 1, "reason_code": "fit", "message": "failed"}
                ],
            },
        }
        self.assertTrue(reference.validate_witness_partition(bootstrap)["passed"])
        duplicate = copy.deepcopy(bootstrap)
        duplicate["validation_witness"]["successful_bootstrap"][1]["replicate_index"] = 0
        self.assertFalse(reference.validate_witness_partition(duplicate)["passed"])

    def test_distribution_comparison_is_evidence_derived_and_fails_large_drift(self) -> None:
        supplied = reference.summarize_supplied(
            reference.FROZEN_SUPPLIED["terms"],
            reference.FROZEN_SUPPLIED["original"],
            reference.FROZEN_SUPPLIED["bootstrap"],
            reference.FROZEN_SUPPLIED["jackknife"],
            expected_jackknife_cases=4,
            logistic=True,
        )
        # Expand the fixture to the production term identity without changing
        # the arithmetic pattern under test.
        rows = []
        for index, term in enumerate(reference.TERMS):
            row = copy.deepcopy(supplied[index % 2])
            row["term"] = term
            rows.append(row)
        same = reference.compare_distributions(
            {"coefficients": copy.deepcopy(rows)},
            {"coefficients": copy.deepcopy(rows)},
            logistic=True,
        )
        self.assertTrue(same["passed"], same)
        drifted = copy.deepcopy(rows)
        drifted[0]["bootstrap_mean"] += 10.0
        failure = reference.compare_distributions(
            {"coefficients": rows},
            {"coefficients": drifted},
            logistic=True,
        )
        self.assertFalse(failure["passed"])
        self.assertFalse(failure["threshold_checks"]["mean_pooled_se_units"])


if __name__ == "__main__":
    unittest.main()
