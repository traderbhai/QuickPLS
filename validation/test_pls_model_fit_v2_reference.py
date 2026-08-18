#!/usr/bin/env python3

from __future__ import annotations

import math
import sys
import unittest
from pathlib import Path

import numpy as np


VALIDATION_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION_DIR))

from pls_model_fit_v2_reference import fit_values  # noqa: E402


class PlsModelFitV2ReferenceTests(unittest.TestCase):
    def test_hand_microcase_matches_the_frozen_natural_log_equations(self) -> None:
        result = fit_values(
            [[1.0, 0.4], [0.4, 1.0]],
            [[1.0, 0.1], [0.1, 1.0]],
            100,
        )

        self.assertAlmostEqual(result.d_uls, 0.09, places=14)
        self.assertAlmostEqual(result.srmr, math.sqrt(0.09 / 3), places=14)
        self.assertAlmostEqual(result.d_g, 0.11128054577065873, places=12)
        self.assertAlmostEqual(result.chi_square, 10.266002077836372, places=12)
        self.assertAlmostEqual(result.nfi or 0, 0.4052482008903624, places=12)

    def test_consistent_indicator_permutation_preserves_every_criterion(self) -> None:
        observed = np.array(
            [[1.0, 0.25, -0.10], [0.25, 1.0, 0.30], [-0.10, 0.30, 1.0]],
        )
        implied = np.array(
            [[1.0, 0.20, -0.05], [0.20, 1.0, 0.22], [-0.05, 0.22, 1.0]],
        )
        permutation = [2, 0, 1]
        baseline = fit_values(observed, implied, 250)
        reordered = fit_values(
            observed[np.ix_(permutation, permutation)],
            implied[np.ix_(permutation, permutation)],
            250,
        )

        for field in ("srmr", "d_uls", "d_g", "chi_square", "nfi"):
            self.assertAlmostEqual(
                getattr(baseline, field) or 0,
                getattr(reordered, field) or 0,
                places=12,
                msg=field,
            )

    def test_singular_correlation_matrix_fails_instead_of_emitting_a_placeholder(self) -> None:
        with self.assertRaises(np.linalg.LinAlgError):
            fit_values(
                [[1.0, 1.0], [1.0, 1.0]],
                [[1.0, 0.1], [0.1, 1.0]],
                100,
            )


if __name__ == "__main__":
    unittest.main()
