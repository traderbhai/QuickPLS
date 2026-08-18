#!/usr/bin/env python3
from __future__ import annotations

import sys
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from micom_mga_v3_reference import (  # noqa: E402
    deterministic_stream_reference,
    exhaustive_partition_reference,
    swap_checks,
)


class MicomMgaV3ReferenceTests(unittest.TestCase):
    def test_exhaustive_partition_and_seed_stream_references_pass(self) -> None:
        exhaustive = exhaustive_partition_reference()
        streams = deterministic_stream_reference()
        self.assertTrue(exhaustive["passed"])
        self.assertEqual(exhaustive["partitions_checked"], 912)
        self.assertTrue(streams["passed"])
        self.assertGreater(streams["synthetic_failures"], 0)

    def test_swap_contract_requires_exact_signed_probability_and_retry_mapping(self) -> None:
        forward = {
            "paths": {("x", "y"): {"original_difference": 0.2, "empirical_p_value_two_sided": 0.04}},
            "measurements": {("loading", "x", "x1"): {"original_difference": -0.1, "empirical_p_value_two_sided": 0.3}},
            "micom": {"x": {"mean_difference": 0.4, "variance_difference": -0.2, "compositional_p_value": 0.5, "mean_p_value": 0.2, "variance_p_value": 0.7, "partial_invariance": True, "equal_means": True, "equal_variances": False, "full_invariance": False}},
            "usable_permutations": 5000,
            "attempted_permutations": 5013,
            "failed_permutations": 13,
        }
        reverse = {
            **forward,
            "paths": {("x", "y"): {"original_difference": -0.2, "empirical_p_value_two_sided": 0.04}},
            "measurements": {("loading", "x", "x1"): {"original_difference": 0.1, "empirical_p_value_two_sided": 0.3}},
            "micom": {"x": {**forward["micom"]["x"], "mean_difference": -0.4, "variance_difference": 0.2}},
        }
        self.assertTrue(swap_checks(forward, reverse)["passed"])
        reverse["failed_permutations"] = 14
        self.assertFalse(swap_checks(forward, reverse)["passed"])


if __name__ == "__main__":
    unittest.main()
