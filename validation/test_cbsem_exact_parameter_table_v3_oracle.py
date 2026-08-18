from __future__ import annotations

import sys
import unittest
from pathlib import Path

import numpy as np

VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from cbsem_exact_parameter_table_v3_oracle import (  # noqa: E402
    STABLE_PARAMETER_IDS,
    OracleContractError,
    disturbance_covariance_sem,
    shared_open_bound_value,
    two_factor_cfa,
)


class ExactParameterTableOracleTests(unittest.TestCase):
    def test_undeclared_latent_covariance_is_exact_zero(self) -> None:
        absent = two_factor_cfa()
        declared = two_factor_cfa(latent_covariance=0.3)
        self.assertEqual(float(absent.psi[0, 1]), 0.0)
        self.assertEqual(float(absent.sigma[0, 3]), 0.0)
        self.assertAlmostEqual(float(declared.sigma[0, 3]), 0.3, places=14)

    def test_declared_residual_covariance_changes_only_theta_placement(self) -> None:
        absent = two_factor_cfa(latent_covariance=0.3)
        declared = two_factor_cfa(
            latent_covariance=0.3, residual_covariance=0.2
        )
        delta = declared.sigma - absent.sigma
        expected = np.zeros((6, 6))
        expected[0, 1] = expected[1, 0] = 0.2
        np.testing.assert_allclose(delta, expected, atol=1e-14, rtol=0.0)

    def test_declared_disturbance_covariance_changes_endogenous_covariance(self) -> None:
        absent = disturbance_covariance_sem()
        declared = disturbance_covariance_sem(disturbance_covariance=0.1)
        self.assertEqual(float(absent.psi[1, 2]), 0.0)
        self.assertAlmostEqual(
            float(declared.sigma[1, 2] - absent.sigma[1, 2]), 0.1, places=14
        )

    def test_non_positive_definite_residual_covariance_is_typed(self) -> None:
        with self.assertRaisesRegex(OracleContractError, "theta must be positive") as raised:
            two_factor_cfa(residual_covariance=0.75)
        self.assertEqual(raised.exception.code, "theta_not_positive_definite")

    def test_equality_start_and_open_bounds_are_fail_closed(self) -> None:
        self.assertEqual(
            shared_open_bound_value(starts=(0.7, 0.7), lower=0.2, upper=1.5),
            0.7,
        )
        for starts, lower, upper, code in [
            ((0.7, 0.8), 0.2, 1.5, "equality_start_conflict"),
            ((0.2, 0.2), 0.2, 1.5, "parameter_start_outside_bounds"),
            ((0.7, 0.7), 1.5, 0.2, "equality_bounds_empty"),
        ]:
            with self.assertRaises(OracleContractError) as raised:
                shared_open_bound_value(starts=starts, lower=lower, upper=upper)
            self.assertEqual(raised.exception.code, code)

    def test_stable_parameter_mapping_is_exact_and_order_invariant(self) -> None:
        canonical = dict(sorted(STABLE_PARAMETER_IDS.items()))
        reversed_input = dict(reversed(list(STABLE_PARAMETER_IDS.items())))
        self.assertEqual(canonical, dict(sorted(reversed_input.items())))
        self.assertEqual(
            canonical["construct:f~~construct:g"], "covariance:f:g"
        )


if __name__ == "__main__":
    unittest.main()
