#!/usr/bin/env python3
"""Cross-runtime tests for the independent R/cSEM Rank 0 oracle."""

from __future__ import annotations

import dataclasses
import math
import sys
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from general_sem_rank0_csem_oracle import (  # noqa: E402
    CsemOracleUnavailable,
    build_request,
    find_rscript,
    run_csem_oracle,
)
from general_sem_rank0_independent_pls_oracle import (  # noqa: E402
    bootstrap_mediation,
    bootstrap_moderation,
    fit_pls_pm,
    fit_simultaneous_moderation,
    mediation_effects,
)
from general_sem_rank0_qualification import (  # noqa: E402
    make_mediation_scenario,
    make_moderation_scenario,
)


def require_rscript(test: unittest.TestCase) -> None:
    try:
        find_rscript()
    except CsemOracleUnavailable as error:
        test.skipTest(str(error))


class GeneralSemRank0CsemOracleTests(unittest.TestCase):
    def test_request_freezes_complete_indexed_samples_without_product_code(self) -> None:
        scenario = make_mediation_scenario(
            "parallel_mediation",
            missingness="listwise_mcar_five_percent",
            rows=100,
            seed=401,
        )
        first = build_request(
            scenario, "mediation_bootstrap", requested=5, seed=402
        )
        replay = build_request(
            scenario, "mediation_bootstrap", requested=5, seed=402
        )
        self.assertEqual(first, replay)
        self.assertEqual(len(first["bootstrap"]["replicate_indices"]), 5)
        self.assertTrue(
            all(
                len(indices) == 95
                for indices in first["bootstrap"]["replicate_indices"]
            )
        )

    def test_csem_point_matches_independent_python_for_mediation_and_moderation(self) -> None:
        require_rscript(self)
        mediation = make_mediation_scenario(
            "mixed_mediation",
            measurement_model="mixed_mode_a_b",
            distribution="skewed_heavy_tailed",
            missingness="listwise_mcar_five_percent",
            rows=130,
            seed=403,
        )
        python_fit = fit_pls_pm(mediation.rows, mediation.model)
        python_effects = mediation_effects(
            python_fit, mediation.model, "x", "y"
        )
        r_effects = run_csem_oracle(
            mediation, "mediation_point"
        )["point"]["values"]["values"]
        self.assertEqual(set(r_effects), set(python_effects))
        for identity, expected in python_effects.items():
            self.assertTrue(
                math.isclose(r_effects[identity], expected, abs_tol=1e-6, rel_tol=1e-5),
                identity,
            )

        moderation = make_moderation_scenario(
            "different_focal_simultaneous",
            measurement_model="mixed_mode_a_b",
            distribution="skewed_heavy_tailed",
            missingness="listwise_mcar_five_percent",
            rows=140,
            seed=404,
        )
        python_point = fit_simultaneous_moderation(
            fit_pls_pm(moderation.rows, moderation.model),
            moderation.model,
            moderation.interactions,
        )
        r_point = run_csem_oracle(
            moderation, "moderation_point"
        )["point"]["values"]
        self.assertEqual(
            set(r_point["scientific_gammas"]), set(python_point.scientific_gammas)
        )
        for identity, expected in python_point.scientific_gammas.items():
            self.assertTrue(
                math.isclose(
                    r_point["scientific_gammas"][identity],
                    expected,
                    abs_tol=1e-6,
                    rel_tol=1e-5,
                ),
                identity,
            )
            self.assertTrue(
                math.isclose(
                    r_point["product_scales"][identity],
                    python_point.product_scales[identity],
                    abs_tol=1e-6,
                    rel_tol=1e-5,
                ),
                identity,
            )

    def test_csem_full_refit_bootstrap_matches_frozen_python_summaries(self) -> None:
        require_rscript(self)
        mediation = make_mediation_scenario(
            "serial_mediation",
            measurement_model="mixed_mode_a_b",
            rows=100,
            seed=405,
        )
        python_mediation = bootstrap_mediation(
            mediation.rows,
            mediation.model,
            "x",
            "y",
            requested=7,
            seed=406,
        )
        r_mediation = run_csem_oracle(
            mediation,
            "mediation_bootstrap",
            requested=7,
            seed=406,
        )["bootstrap"]
        self.assertEqual(r_mediation["usable"], python_mediation.usable)
        for identity, expected in python_mediation.summaries.items():
            actual = r_mediation["summaries"][identity]
            for field in (
                "mean",
                "bias",
                "standard_error",
                "lower",
                "upper",
                "plus_one_two_sided_probability",
            ):
                self.assertTrue(
                    math.isclose(
                        actual[field],
                        getattr(expected, field),
                        abs_tol=1e-6,
                        rel_tol=1e-5,
                    ),
                    f"{identity}.{field}",
                )

        moderation = make_moderation_scenario(
            "same_focal_simultaneous", rows=110, seed=407
        )
        python_moderation = bootstrap_moderation(
            moderation.rows,
            moderation.model,
            moderation.interactions,
            requested=7,
            seed=408,
        )
        r_moderation = run_csem_oracle(
            moderation,
            "moderation_bootstrap",
            requested=7,
            seed=408,
        )["bootstrap"]
        self.assertEqual(r_moderation["usable"], python_moderation.usable)
        for identity, expected in python_moderation.summaries.items():
            actual = r_moderation["summaries"][identity]
            self.assertTrue(
                math.isclose(
                    actual["mean"], expected.mean, abs_tol=1e-6, rel_tol=1e-5
                ),
                identity,
            )
            self.assertEqual(actual["exceedances"], expected.exceedances)

    def test_csem_mapped_results_are_declaration_indicator_and_row_order_invariant(self) -> None:
        require_rscript(self)
        original = make_moderation_scenario(
            "different_focal_simultaneous",
            measurement_model="mixed_mode_a_b",
            rows=120,
            seed=409,
        )
        reordered_model = dataclasses.replace(
            original.model,
            blocks=tuple(
                dataclasses.replace(block, indicator_ids=tuple(reversed(block.indicator_ids)))
                for block in reversed(original.model.blocks)
            ),
            paths=tuple(reversed(original.model.paths)),
        )
        reordered = dataclasses.replace(
            original,
            rows=tuple(reversed(original.rows)),
            model=reordered_model,
            interactions=tuple(reversed(original.interactions)),
        )
        first = run_csem_oracle(original, "moderation_point")["point"]["values"]
        second = run_csem_oracle(reordered, "moderation_point")["point"]["values"]
        self.assertEqual(set(first["scientific_gammas"]), set(second["scientific_gammas"]))
        for identity, expected in first["scientific_gammas"].items():
            self.assertTrue(
                math.isclose(
                    second["scientific_gammas"][identity],
                    expected,
                    abs_tol=1e-6,
                    rel_tol=1e-5,
                ),
                identity,
            )


if __name__ == "__main__":
    unittest.main()
