#!/usr/bin/env python3
"""Focused scientific-contract tests for the independent MICOM v3 oracle."""

from __future__ import annotations

import copy
import itertools
import math
import sys
import unittest
from pathlib import Path
from typing import Mapping, Sequence

import numpy as np


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from micom_v3_oracle import (  # noqa: E402
    ConfiguralReview,
    IndependentPlsEstimator,
    MicomConfig,
    MicomFailureCode,
    MicomOracleError,
    load_fixed_fixture,
    run_micom,
)


REVIEW = ConfiguralReview(True, True, True, True, True, "unit-test reviewer")


class _MomentEstimator:
    """Fast test double with group-estimated, orientation-ambiguous weights."""

    def __init__(self, order: Sequence[str] = ("c", "d"), fail_modulus: int | None = None) -> None:
        self._order = tuple(order)
        self._fail_modulus = fail_modulus
        self._fit_calls = 0

    @property
    def construct_ids(self) -> Sequence[str]:
        return self._order

    def fit(self, raw: np.ndarray) -> dict[str, np.ndarray]:
        self._fit_calls += 1
        if (
            self._fail_modulus is not None
            and self._fit_calls > 3
            and self._fit_calls % self._fail_modulus == 0
        ):
            raise RuntimeError("deliberate indexed-fit failure")
        result = {}
        for construct, columns in (("c", (0, 1)), ("d", (2, 3))):
            covariance = np.cov(raw[:, columns], rowvar=False, ddof=1)
            _, vectors = np.linalg.eigh(covariance)
            weights = vectors[:, -1]
            # Preserve an arbitrary estimator orientation so the MICOM layer has
            # to align each group to the pooled reference.
            if float(np.mean(raw[:, columns[0]])) < 0.0:
                weights = -weights
            result[construct] = weights
        return result

    def scores(
        self, raw_pooled: np.ndarray, fit: Mapping[str, np.ndarray]
    ) -> Mapping[str, np.ndarray]:
        columns = {"c": (0, 1), "d": (2, 3)}
        return {
            construct: raw_pooled[:, columns[construct]] @ fit[construct]
            for construct in self._order
        }


class _FixedWeightEstimator:
    @property
    def construct_ids(self) -> Sequence[str]:
        return ("c",)

    def fit(self, raw: np.ndarray) -> np.ndarray:
        del raw
        return np.asarray([0.4, 0.8])

    def scores(
        self, raw_pooled: np.ndarray, fit: np.ndarray
    ) -> Mapping[str, np.ndarray]:
        return {"c": raw_pooled @ fit}


def _synthetic(rows_per_group: int = 24) -> tuple[np.ndarray, np.ndarray, tuple[str, ...]]:
    rng = np.random.default_rng(731)
    latent = rng.normal(size=(rows_per_group * 2, 2))
    raw = np.column_stack(
        (
            latent[:, 0] + rng.normal(scale=0.2, size=len(latent)),
            0.8 * latent[:, 0] + rng.normal(scale=0.25, size=len(latent)),
            latent[:, 1] + rng.normal(scale=0.2, size=len(latent)),
            0.7 * latent[:, 1] + rng.normal(scale=0.3, size=len(latent)),
        )
    )
    labels = np.asarray(["A"] * rows_per_group + ["B"] * rows_per_group, dtype=object)
    ids = tuple(f"case-{index:03d}" for index in range(len(raw)))
    return raw, labels, ids


class MicomV3OracleTests(unittest.TestCase):
    def test_closed_form_fixed_weight_and_exhaustive_partition_hand_case(self) -> None:
        first = np.linspace(-2.0, 2.0, 20)
        second = np.linspace(3.0, -1.0, 20) + np.sin(np.arange(20)) * 0.1
        raw = np.column_stack((first, second))
        labels = np.asarray(["A"] * 10 + ["B"] * 10, dtype=object)
        case_ids = tuple(f"hand-{index:02d}" for index in range(20))
        result = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=_FixedWeightEstimator(),
            config=MicomConfig(permutations=19),
        )
        score = raw @ np.asarray([0.4, 0.8])
        score = (score - np.mean(score)) / np.std(score, ddof=1)
        expected_mean = float(np.mean(score[:10]) - np.mean(score[10:]))
        expected_log_variance = math.log(float(np.var(score[:10], ddof=1))) - math.log(
            float(np.var(score[10:], ddof=1))
        )
        row = result["constructs"]["c"]
        self.assertAlmostEqual(row["step2"]["compositional_correlation"], 1.0, places=12)
        self.assertAlmostEqual(row["step3"]["mean_difference"], expected_mean, places=12)
        self.assertAlmostEqual(
            row["step3"]["log_variance_ratio"], expected_log_variance, places=12
        )

        values = np.asarray([-3.0, -1.0, 0.5, 2.0, 4.0, 7.0])
        for selected in itertools.combinations(range(len(values)), 3):
            mask = np.zeros(len(values), dtype=bool)
            mask[list(selected)] = True
            mean_difference = float(np.mean(values[mask]) - np.mean(values[~mask]))
            reversed_mean = float(np.mean(values[~mask]) - np.mean(values[mask]))
            log_ratio = math.log(float(np.var(values[mask], ddof=1))) - math.log(
                float(np.var(values[~mask], ddof=1))
            )
            reversed_log_ratio = math.log(float(np.var(values[~mask], ddof=1))) - math.log(
                float(np.var(values[mask], ddof=1))
            )
            self.assertEqual(mean_difference, -reversed_mean)
            self.assertEqual(log_ratio, -reversed_log_ratio)

    def test_step1_is_required_review_and_never_computed(self) -> None:
        raw, labels, case_ids = _synthetic()
        with self.assertRaises(MicomOracleError) as missing:
            run_micom(
                raw,
                labels,
                case_ids,
                group_a="A",
                group_b="B",
                configural_review=None,
                estimator=_MomentEstimator(),
                config=MicomConfig(permutations=19),
            )
        self.assertEqual(
            missing.exception.code, MicomFailureCode.CONFIGURAL_REVIEW_REQUIRED
        )
        incomplete = ConfiguralReview(True, False, True, True, True, "reviewer")
        with self.assertRaises(MicomOracleError) as unconfirmed:
            run_micom(
                raw,
                labels,
                case_ids,
                group_a="A",
                group_b="B",
                configural_review=incomplete,
                estimator=_MomentEstimator(),
                config=MicomConfig(permutations=19),
            )
        self.assertEqual(
            unconfirmed.exception.code,
            MicomFailureCode.CONFIGURAL_INVARIANCE_NOT_CONFIRMED,
        )

        result = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=_MomentEstimator(),
            config=MicomConfig(permutations=19),
        )
        self.assertFalse(result["step1"]["computed"])
        self.assertEqual(result["step1"]["status"], "confirmed_by_researcher_review")

    def test_plan_is_size_preserving_exactly_once_and_micom_only(self) -> None:
        raw, labels, case_ids = _synthetic(21)
        result = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=_MomentEstimator(),
            config=MicomConfig(permutations=23),
        )
        accounting = result["accounting"]
        self.assertEqual(accounting["requested_permutations"], 23)
        self.assertEqual(accounting["attempted_permutations"], 23)
        self.assertEqual(accounting["retry_policy"], "none")
        self.assertEqual([row["replicate"] for row in result["ledger"]], list(range(23)))
        self.assertTrue(
            all(row["group_a_rows"] == 21 and row["group_b_rows"] == 21 for row in result["ledger"])
        )
        self.assertEqual(
            result["scope"],
            {
                "method": "micom",
                "structural_path_permutation_mga": False,
                "consistent_permutation_plsc": False,
            },
        )

    def test_group_swap_row_reorder_and_seed_contracts(self) -> None:
        raw, labels, case_ids = _synthetic(24)
        config = MicomConfig(permutations=29, seed=91)
        estimator = _MomentEstimator()
        forward = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=estimator,
            config=config,
        )
        reverse = run_micom(
            raw,
            labels,
            case_ids,
            group_a="B",
            group_b="A",
            configural_review=REVIEW,
            estimator=estimator,
            config=config,
        )
        order = np.arange(len(raw))[::-1]
        reordered = run_micom(
            raw[order],
            labels[order],
            tuple(case_ids[index] for index in order),
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=_MomentEstimator(("d", "c")),
            config=config,
        )
        repeated = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=estimator,
            config=config,
        )
        changed_seed = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=estimator,
            config=MicomConfig(permutations=29, seed=92),
        )
        for construct, left in forward["constructs"].items():
            right = reverse["constructs"][construct]
            self.assertAlmostEqual(
                left["step3"]["mean_difference"],
                -right["step3"]["mean_difference"],
                places=12,
            )
            self.assertAlmostEqual(
                left["step3"]["log_variance_ratio"],
                -right["step3"]["log_variance_ratio"],
                places=12,
            )
            self.assertEqual(
                left["step2"]["lower_tail_p_value"],
                right["step2"]["lower_tail_p_value"],
            )
            self.assertEqual(
                left["partial_measurement_invariance"],
                right["partial_measurement_invariance"],
            )
        self.assertEqual(forward["constructs"], reordered["constructs"])
        self.assertEqual(forward["ledger"], reordered["ledger"])
        self.assertEqual(forward, repeated)
        self.assertNotEqual(
            [row["partition_sha256"] for row in forward["ledger"]],
            [row["partition_sha256"] for row in changed_seed["ledger"]],
        )

    def test_step3_uses_corrected_observed_difference_interval_rule(self) -> None:
        raw, labels, case_ids = _synthetic()
        result = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=_MomentEstimator(),
            config=MicomConfig(permutations=31),
        )
        for row in result["constructs"].values():
            step3 = row["step3"]
            self.assertEqual(
                step3["official_interval_decision"],
                "the obtained difference is compared with the permutation interval",
            )
            lower, upper = step3["mean_difference_interval"]
            self.assertEqual(
                step3["equal_means"], lower <= step3["mean_difference"] <= upper
            )
            self.assertEqual(
                row["full_measurement_invariance"],
                row["partial_measurement_invariance"]
                and step3["equal_means"]
                and step3["equal_variances"],
            )
            self.assertEqual(
                row["step3_interpretable"], row["partial_measurement_invariance"]
            )

    def test_failed_permutation_fits_are_ledged_and_never_replaced(self) -> None:
        raw, labels, case_ids = _synthetic(30)
        result = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=_MomentEstimator(fail_modulus=3),
            config=MicomConfig(
                permutations=41,
                minimum_usable_permutations=1,
            ),
        )
        accounting = result["accounting"]
        self.assertEqual(accounting["attempted_permutations"], 41)
        self.assertEqual(len(result["ledger"]), 41)
        self.assertGreater(accounting["step2_failed_permutations"], 0)
        self.assertEqual(
            accounting["step2_usable_permutations"]
            + accounting["step2_failed_permutations"],
            41,
        )
        self.assertTrue(
            all(
                row["step2_failure_code"] is not None
                for row in result["ledger"]
                if row["step2_status"] == "failed"
            )
        )

    def test_small_empty_imbalanced_and_degenerate_inputs_fail_typed(self) -> None:
        raw, labels, case_ids = _synthetic(20)

        def failure_code(
            candidate_raw: np.ndarray,
            candidate_labels: np.ndarray,
            candidate_ids: Sequence[str],
            config: MicomConfig = MicomConfig(permutations=19),
        ) -> MicomFailureCode:
            with self.assertRaises(MicomOracleError) as raised:
                run_micom(
                    candidate_raw,
                    candidate_labels,
                    candidate_ids,
                    group_a="A",
                    group_b="B",
                    configural_review=REVIEW,
                    estimator=_MomentEstimator(),
                    config=config,
                )
            return raised.exception.code

        self.assertEqual(
            failure_code(raw, np.asarray(["A"] * len(raw), dtype=object), case_ids),
            MicomFailureCode.EMPTY_GROUP,
        )
        small = np.asarray(["A"] * 9 + ["B"] * (len(raw) - 9), dtype=object)
        self.assertEqual(
            failure_code(raw, small, case_ids), MicomFailureCode.GROUP_TOO_SMALL
        )
        rng = np.random.default_rng(902)
        imbalanced = rng.normal(size=(111, 4))
        imbalanced_labels = np.asarray(["A"] * 10 + ["B"] * 101, dtype=object)
        self.assertEqual(
            failure_code(
                imbalanced,
                imbalanced_labels,
                tuple(f"imbalanced-{index}" for index in range(111)),
            ),
            MicomFailureCode.EXTREME_GROUP_IMBALANCE,
        )
        degenerate = copy.deepcopy(raw)
        degenerate[:, 0] = 7.0
        self.assertEqual(
            failure_code(degenerate, labels, case_ids),
            MicomFailureCode.DEGENERATE_INDICATOR,
        )

    def test_fixed_fixture_pls_adapter_preserves_construct_identities(self) -> None:
        raw, labels, case_ids = load_fixed_fixture()
        result = run_micom(
            raw,
            labels,
            case_ids,
            group_a="A",
            group_b="B",
            configural_review=REVIEW,
            estimator=IndependentPlsEstimator(("y", "r", "z", "x")),
            config=MicomConfig(permutations=19),
        )
        self.assertEqual(list(result["constructs"]), ["r", "x", "y", "z"])
        self.assertTrue(
            all(
                row["step2"]["usable_permutations"] == 19
                for row in result["constructs"].values()
            )
        )


if __name__ == "__main__":
    unittest.main()
