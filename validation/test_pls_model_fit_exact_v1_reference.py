#!/usr/bin/env python3

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

import numpy as np


VALIDATION_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION_DIR))

from pls_model_fit_exact_v1_reference import (  # noqa: E402
    null_transform,
    summarize_fixed_ledger,
    type7_quantile,
)


class PlsModelFitExactV1ReferenceTests(unittest.TestCase):
    def test_symmetric_null_transform_reproduces_target_correlation(self) -> None:
        observations = np.array(
            [
                [-1.2, -0.8, 0.3],
                [-0.7, 0.1, -0.5],
                [-0.1, 0.6, 1.1],
                [0.4, -0.2, 0.7],
                [0.8, 1.3, -0.9],
                [1.4, -1.0, -0.7],
            ]
        )
        target = np.array(
            [[1.0, 0.35, -0.20], [0.35, 1.0, 0.25], [-0.20, 0.25, 1.0]]
        )
        transformed = null_transform(observations, target)
        np.testing.assert_allclose(
            np.corrcoef(transformed, rowvar=False), target, rtol=0, atol=1e-12
        )

    def test_type7_and_fixed_ledger_summary_match_frozen_decisions(self) -> None:
        values = [index / 10 for index in range(9)] + [math.nan]
        summary = summarize_fixed_ledger(0.4, values, 10)

        self.assertEqual(summary.status, "available")
        self.assertEqual(summary.usable_replicates, 9)
        self.assertAlmostEqual(summary.upper_95 or 0, 0.76, places=14)
        self.assertAlmostEqual(summary.upper_99 or 0, 0.792, places=14)
        self.assertEqual(summary.exceed_or_equal_count, 5)
        self.assertAlmostEqual(summary.empirical_upper_tail_probability or 0, 5 / 9)
        self.assertTrue(summary.not_rejected_95)
        self.assertAlmostEqual(type7_quantile(list(range(9)), 0.95), 7.6)

    def test_below_ninety_percent_is_unavailable_without_a_decision(self) -> None:
        summary = summarize_fixed_ledger(0.1, [0.1] * 8 + [math.nan] * 2, 10)
        self.assertEqual(summary.status, "unavailable")
        self.assertIsNone(summary.upper_95)
        self.assertIsNone(summary.not_rejected_95)
        self.assertEqual(
            summary.unavailable_reason_code,
            "model_fit_exact.insufficient_usable_replicates",
        )

    def test_singular_observed_or_target_matrix_fails_without_repair(self) -> None:
        with self.assertRaises(ValueError):
            null_transform(
                [[-1.0, -2.0], [0.0, 0.0], [1.0, 2.0]],
                [[1.0, 0.2], [0.2, 1.0]],
            )
        with self.assertRaises(ValueError):
            null_transform(
                [[-1.0, 0.0], [0.0, 1.0], [1.0, -1.0]],
                [[1.0, 1.0], [1.0, 1.0]],
            )


if __name__ == "__main__":
    unittest.main()
