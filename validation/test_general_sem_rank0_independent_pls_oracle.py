#!/usr/bin/env python3
"""Focused unit tests for the production-independent General SEM oracle."""

from __future__ import annotations

import dataclasses
import math
import sys
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from general_sem_rank0_independent_pls_oracle import (  # noqa: E402
    BlockSpec,
    PlsModelSpec,
    BootstrapFailure,
    NumericalOracleError,
    align_fit_to_reference,
    bootstrap_mediation,
    bootstrap_moderation,
    fit_pls_pm,
    fit_simultaneous_moderation,
    mean,
    mediation_effects,
    minimum_usable_replicates,
    plus_one_two_sided,
    sample_sd,
    sample_standard_error,
    summarize_bootstrap_distributions,
    type7,
)
from general_sem_rank0_qualification import (  # noqa: E402
    make_mediation_scenario,
    make_moderation_scenario,
)


class GeneralSemRank0IndependentPlsOracleTests(unittest.TestCase):
    def test_frozen_bootstrap_arithmetic_and_usable_gate(self) -> None:
        values = [0.1, 0.2, 0.3, 0.4]
        self.assertAlmostEqual(type7(values, 0.025), 0.1075, places=15)
        self.assertAlmostEqual(type7(values, 0.975), 0.3925, places=15)
        self.assertAlmostEqual(
            sample_standard_error(values), 0.12909944487358055, places=15
        )
        exceedances, probability = plus_one_two_sided(
            0.25, [0.0, 0.2, 0.3, 0.4]
        )
        self.assertEqual(exceedances, 1)
        self.assertAlmostEqual(probability, 0.4, places=15)
        self.assertEqual(minimum_usable_replicates(2), 2)
        self.assertEqual(minimum_usable_replicates(10), 9)
        self.assertEqual(minimum_usable_replicates(11), 10)
        self.assertEqual(minimum_usable_replicates(10_000), 9_000)

    def test_mode_a_and_supported_mode_b_scores_are_oriented_and_standardized(self) -> None:
        for measurement_model in ("all_mode_a", "mixed_mode_a_b"):
            with self.subTest(measurement_model=measurement_model):
                scenario = make_mediation_scenario(
                    "mixed_mediation",
                    measurement_model=measurement_model,
                    rows=180,
                    seed=31,
                )
                fit = fit_pls_pm(scenario.rows, scenario.model)
                self.assertLessEqual(fit.convergence_change, 1.0e-10)
                self.assertTrue(
                    any(block.mode == "B" for block in scenario.model.blocks)
                    if measurement_model == "mixed_mode_a_b"
                    else all(block.mode == "A" for block in scenario.model.blocks)
                )
                for block in scenario.model.blocks:
                    self.assertGreater(fit.weights[block.construct_id][0], 0.0)
                    score = fit.scores[block.construct_id]
                    self.assertAlmostEqual(mean(score), 0.0, places=10)
                    self.assertAlmostEqual(sample_sd(score), 1.0, places=10)
                self.assertEqual(
                    set(fit.path_coefficients),
                    {
                        (path.source_id, path.target_id)
                        for path in scenario.model.paths
                    },
                )

    def test_listwise_missingness_uses_one_model_wide_frame(self) -> None:
        scenario = make_mediation_scenario(
            "parallel_mediation",
            missingness="listwise_mcar_five_percent",
            rows=200,
            seed=32,
        )
        fit = fit_pls_pm(scenario.rows, scenario.model)
        self.assertEqual(len(fit.used_row_indices), 190)
        self.assertTrue(all(index % 20 != 0 for index in fit.used_row_indices))

    def test_sign_alignment_precedes_recomputed_paths(self) -> None:
        scenario = make_mediation_scenario(
            "serial_mediation", rows=160, seed=33
        )
        reference = fit_pls_pm(scenario.rows, scenario.model)
        negated = dataclasses.replace(
            reference,
            weights={
                key: tuple(-value for value in values)
                for key, values in reference.weights.items()
            },
            loadings={
                key: tuple(-value for value in values)
                for key, values in reference.loadings.items()
            },
            scores={
                key: tuple(-value for value in values)
                for key, values in reference.scores.items()
            },
        )
        aligned, corrections = align_fit_to_reference(
            negated,
            reference,
            tuple(range(len(reference.used_row_indices))),
            scenario.model,
        )
        self.assertEqual(corrections, len(scenario.model.blocks))
        self.assertEqual(aligned.scores, reference.scores)
        for key, expected in reference.path_coefficients.items():
            self.assertAlmostEqual(aligned.path_coefficients[key], expected, places=12)

    def test_parallel_serial_and_mixed_mediation_path_inventory(self) -> None:
        expected = {
            "parallel_mediation": 2,
            "serial_mediation": 1,
            "mixed_mediation": 3,
        }
        for index, (topology, path_count) in enumerate(expected.items()):
            with self.subTest(topology=topology):
                scenario = make_mediation_scenario(
                    topology, rows=180, seed=40 + index
                )
                fit = fit_pls_pm(scenario.rows, scenario.model)
                effects = mediation_effects(fit, scenario.model, "x", "y")
                specific = {
                    key: value
                    for key, value in effects.items()
                    if key.startswith("specific:")
                }
                self.assertEqual(len(specific), path_count)
                self.assertAlmostEqual(
                    effects["total_indirect:x->y"],
                    math.fsum(specific.values()),
                    places=14,
                )
                self.assertAlmostEqual(
                    effects["total:x->y"],
                    effects["direct:x->y"] + effects["total_indirect:x->y"],
                    places=14,
                )

    def test_same_and_different_focal_moderation_use_complete_joint_equations(self) -> None:
        for index, topology in enumerate(
            ("same_focal_simultaneous", "different_focal_simultaneous")
        ):
            with self.subTest(topology=topology):
                scenario = make_moderation_scenario(
                    topology,
                    measurement_model="mixed_mode_a_b" if index else "all_mode_a",
                    rows=220,
                    seed=50 + index,
                )
                fit = fit_pls_pm(scenario.rows, scenario.model)
                moderation = fit_simultaneous_moderation(
                    fit, scenario.model, scenario.interactions
                )
                expected_ids = {
                    interaction.interaction_id
                    for interaction in scenario.interactions
                }
                self.assertEqual(set(moderation.scientific_gammas), expected_ids)
                self.assertEqual(
                    set(moderation.standardized_product_coefficients), expected_ids
                )
                self.assertTrue(
                    all(value > 0 for value in moderation.product_scales.values())
                )
                for identifier, sign in (scenario.expected_gamma_signs or {}).items():
                    self.assertEqual(
                        1 if moderation.scientific_gammas[identifier] > 0 else -1,
                        sign,
                    )
                    focal = next(
                        row.focal_id
                        for row in scenario.interactions
                        if row.interaction_id == identifier
                    )
                    outcome = next(
                        row.outcome_id
                        for row in scenario.interactions
                        if row.interaction_id == identifier
                    )
                    self.assertAlmostEqual(
                        moderation.fixed_probe_slopes[identifier][1],
                        moderation.direct_coefficients[(focal, outcome)],
                        places=14,
                    )

    def test_full_model_bootstraps_are_index_schedule_invariant(self) -> None:
        mediation = make_mediation_scenario(
            "mixed_mediation", rows=140, seed=60
        )
        serial = bootstrap_mediation(
            mediation.rows,
            mediation.model,
            "x",
            "y",
            requested=11,
            seed=70,
        )
        rescheduled = bootstrap_mediation(
            mediation.rows,
            mediation.model,
            "x",
            "y",
            requested=11,
            seed=70,
            evaluation_order=tuple(reversed(range(11))),
        )
        self.assertEqual(serial, rescheduled)
        self.assertTrue(serial.published)
        self.assertGreaterEqual(serial.usable, serial.minimum_usable)
        moderation_scenario = make_moderation_scenario(
            "different_focal_simultaneous", rows=150, seed=61
        )
        moderation = bootstrap_moderation(
            moderation_scenario.rows,
            moderation_scenario.model,
            moderation_scenario.interactions,
            requested=11,
            seed=71,
        )
        self.assertTrue(moderation.published)
        self.assertEqual(
            set(moderation.summaries),
            {
                interaction.interaction_id
                for interaction in moderation_scenario.interactions
            },
        )
        self.assertTrue(
            all(
                math.isfinite(summary.standard_error)
                and summary.lower <= summary.upper
                for summary in moderation.summaries.values()
            )
        )

    def test_declaration_indicator_and_row_reorder_is_mapped_invariant(self) -> None:
        scenario = make_moderation_scenario(
            "different_focal_simultaneous",
            measurement_model="mixed_mode_a_b",
            distribution="skewed_heavy_tailed",
            missingness="listwise_mcar_five_percent",
            rows=140,
            seed=8_110,
        )
        reordered_model = PlsModelSpec(
            tuple(
                BlockSpec(
                    block.construct_id,
                    tuple(reversed(block.indicator_ids)),
                    block.mode,
                )
                for block in reversed(scenario.model.blocks)
            ),
            tuple(reversed(scenario.model.paths)),
        )
        original = fit_pls_pm(scenario.rows, scenario.model)
        reordered = fit_pls_pm(tuple(reversed(scenario.rows)), reordered_model)
        self.assertEqual(set(original.path_coefficients), set(reordered.path_coefficients))
        for identity, expected in original.path_coefficients.items():
            self.assertTrue(
                math.isclose(
                    reordered.path_coefficients[identity],
                    expected,
                    abs_tol=1e-10,
                    rel_tol=1e-10,
                ),
                identity,
            )
        first = fit_simultaneous_moderation(
            original, scenario.model, scenario.interactions
        )
        second = fit_simultaneous_moderation(
            reordered, reordered_model, tuple(reversed(scenario.interactions))
        )
        for identity, expected in first.scientific_gammas.items():
            self.assertTrue(
                math.isclose(
                    second.scientific_gammas[identity],
                    expected,
                    abs_tol=1e-10,
                    rel_tol=1e-10,
                ),
                identity,
            )

    def test_declaration_and_indicator_reorder_preserves_bootstrap_exactly(self) -> None:
        scenario = make_mediation_scenario(
            "mixed_mediation",
            measurement_model="mixed_mode_a_b",
            rows=100,
            seed=8_111,
        )
        reordered_model = PlsModelSpec(
            tuple(
                BlockSpec(
                    block.construct_id,
                    tuple(reversed(block.indicator_ids)),
                    block.mode,
                )
                for block in reversed(scenario.model.blocks)
            ),
            tuple(reversed(scenario.model.paths)),
        )
        original = bootstrap_mediation(
            scenario.rows,
            scenario.model,
            "x",
            "y",
            requested=7,
            seed=8_112,
        )
        reordered = bootstrap_mediation(
            scenario.rows,
            reordered_model,
            "x",
            "y",
            requested=7,
            seed=8_112,
        )
        self.assertEqual(original, reordered)

    def test_usable_gate_suppresses_all_summaries_below_ninety_percent(self) -> None:
        accepted = summarize_bootstrap_distributions(
            requested=20,
            originals={"theta": 0.2},
            distributions={"theta": [0.1 + index * 0.01 for index in range(18)]},
            usable_indices=range(18),
            failures=(BootstrapFailure(18, "controlled"), BootstrapFailure(19, "controlled")),
        )
        self.assertTrue(accepted.published)
        rejected = summarize_bootstrap_distributions(
            requested=20,
            originals={"theta": 0.2},
            distributions={"theta": [0.1 + index * 0.01 for index in range(17)]},
            usable_indices=range(17),
            failures=tuple(
                BootstrapFailure(index, "controlled") for index in range(17, 20)
            ),
        )
        self.assertFalse(rejected.published)
        self.assertEqual(rejected.summaries, {})

    def test_constant_indicator_is_rejected(self) -> None:
        scenario = make_mediation_scenario(
            "parallel_mediation", rows=100, seed=80
        )
        rows = [dict(row) for row in scenario.rows]
        indicator = scenario.model.blocks[0].indicator_ids[0]
        for row in rows:
            row[indicator] = 1.0
        with self.assertRaises(NumericalOracleError):
            fit_pls_pm(rows, scenario.model)


if __name__ == "__main__":
    unittest.main()
