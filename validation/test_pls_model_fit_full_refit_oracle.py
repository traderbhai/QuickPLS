#!/usr/bin/env python3
"""Focused checks for the independent PLS model-fit full-refit oracle."""

from __future__ import annotations

import ast
import hashlib
import json
import sys
import unittest
from pathlib import Path

import jsonschema
import numpy as np


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
sys.path.insert(0, str(VALIDATION_DIR))

from pls_model_fit_full_refit_oracle import (  # noqa: E402
    ConstructSpec,
    ModelFitOracleError,
    ModelSpec,
    _synthetic_fixture,
    build_index_plan,
    compare_frozen_product_point_fit,
    fit_pls_model,
    model_from_recipe_document,
    null_transform,
    read_csv_matrix,
    run_adapted_bollen_stine,
    sample_index_digest,
    type7_quantile,
)


class PlsModelFitFullRefitOracleTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.synthetic, cls.synthetic_model = _synthetic_fixture()
        cls.draws = 12
        cls.seed = 2026081511
        cls.exact = run_adapted_bollen_stine(
            cls.synthetic,
            cls.synthetic_model,
            requested_replicates=cls.draws,
            seed=cls.seed,
            tolerance=1e-9,
        )

    def test_source_imports_no_product_module_or_binding(self) -> None:
        source_path = VALIDATION_DIR / "pls_model_fit_full_refit_oracle.py"
        tree = ast.parse(source_path.read_text(encoding="utf-8"))
        imported_roots = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                imported_roots.update(alias.name.split(".")[0] for alias in node.names)
            elif isinstance(node, ast.ImportFrom) and node.module:
                imported_roots.add(node.module.split(".")[0])
        self.assertFalse(
            imported_roots
            & {
                "qpls",
                "qpls_assessment",
                "qpls_core",
                "qpls_estimation",
                "qpls_resampling",
                "qpls_runner",
            }
        )

    def test_full_point_refit_matches_frozen_product_fixture(self) -> None:
        recipe_path = VALIDATION_DIR / "fixtures" / "simple_reflective.recipe.json"
        data_path = VALIDATION_DIR / "fixtures" / "simple_reflective.csv"
        product_path = VALIDATION_DIR / "results" / "pls_quickpls_path_mode_a.json"
        recipe = json.loads(recipe_path.read_text(encoding="utf-8"))
        model = model_from_recipe_document(recipe)
        observations = read_csv_matrix(data_path, model.indicator_order)
        point = fit_pls_model(
            observations,
            model,
            tolerance=float(recipe["settings"]["tolerance"]),
            max_iterations=int(recipe["settings"]["max_iterations"]),
        )
        product = json.loads(product_path.read_text(encoding="utf-8"))
        comparison = compare_frozen_product_point_fit(point, model, product)

        self.assertTrue(comparison["passed"], comparison)
        self.assertLessEqual(comparison["maximum_absolute_difference"], 1e-10)
        self.assertEqual(
            comparison["role"],
            "behavioral_comparator_only_not_numerical_oracle",
        )
        self.assertEqual(point.iterations, 4)

    def test_null_transforms_and_full_refits_are_separate_by_variant(self) -> None:
        saturated = self.exact.saturated
        estimated = self.exact.estimated
        np.testing.assert_allclose(
            saturated.recovered_transformed_correlation,
            saturated.target_correlation,
            rtol=0,
            atol=1e-9,
        )
        np.testing.assert_allclose(
            estimated.recovered_transformed_correlation,
            estimated.target_correlation,
            rtol=0,
            atol=1e-9,
        )
        self.assertFalse(
            np.allclose(
                saturated.target_correlation,
                estimated.target_correlation,
                rtol=0,
                atol=1e-12,
            )
        )
        self.assertNotEqual(
            [entry.sample_index_digest for entry in saturated.ledger],
            [entry.sample_index_digest for entry in estimated.ledger],
        )
        self.assertFalse(
            np.allclose(
                [entry.d_uls for entry in saturated.ledger],
                [entry.d_uls for entry in estimated.ledger],
                rtol=0,
                atol=1e-15,
            )
        )

    def test_type7_hi95_hi99_and_decisions_are_recomputed_from_fixed_ledgers(self) -> None:
        for variant in (self.exact.saturated, self.exact.estimated):
            self.assertEqual(variant.status, "available")
            for summary in variant.criteria:
                values = [
                    getattr(entry, summary.criterion)
                    for entry in variant.ledger
                    if getattr(entry, summary.criterion) is not None
                ]
                self.assertEqual(summary.requested_replicates, self.draws)
                self.assertEqual(summary.usable_replicates, len(values))
                self.assertAlmostEqual(summary.upper_95 or 0, type7_quantile(values, 0.95))
                self.assertAlmostEqual(summary.upper_99 or 0, type7_quantile(values, 0.99))
                self.assertEqual(
                    summary.not_rejected_95,
                    summary.original <= (summary.upper_95 or 0),
                )
                self.assertEqual(
                    summary.not_rejected_99,
                    summary.original <= (summary.upper_99 or 0),
                )
                self.assertAlmostEqual(
                    summary.upper_95 or 0,
                    float(np.quantile(values, 0.95, method="linear")),
                )
                self.assertAlmostEqual(
                    summary.upper_99 or 0,
                    float(np.quantile(values, 0.99, method="linear")),
                )

    def test_seed_index_identity_is_repeatable_and_domain_separated(self) -> None:
        rerun = run_adapted_bollen_stine(
            self.synthetic,
            self.synthetic_model,
            requested_replicates=self.draws,
            seed=self.seed,
            tolerance=1e-9,
        )
        for variant in ("saturated", "estimated"):
            baseline_entries = getattr(self.exact, variant).ledger
            rerun_entries = getattr(rerun, variant).ledger
            self.assertEqual(baseline_entries, rerun_entries)
            plan = build_index_plan(
                self.synthetic.shape[0], self.draws, self.seed, variant
            )
            self.assertEqual(
                [entry.sample_index_digest for entry in baseline_entries],
                [sample_index_digest(indices) for indices in plan],
            )

    def test_fixed_failure_ledger_keeps_requested_cells_and_exact_usable_counts(self) -> None:
        plans = {
            variant: list(build_index_plan(self.synthetic.shape[0], 10, 2026081512, variant))
            for variant in ("saturated", "estimated")
        }
        for variant in plans:
            plans[variant][0] = np.zeros(self.synthetic.shape[0], dtype=np.uint64)
        result = run_adapted_bollen_stine(
            self.synthetic,
            self.synthetic_model,
            requested_replicates=10,
            seed=2026081512,
            tolerance=1e-9,
            index_plans=plans,
        )

        for variant in (result.saturated, result.estimated):
            self.assertEqual(len(variant.ledger), 10)
            self.assertEqual([entry.replicate_index for entry in variant.ledger], list(range(10)))
            self.assertEqual(sum(entry.status == "failed" for entry in variant.ledger), 1)
            failed = variant.ledger[0]
            self.assertEqual(failed.status, "failed")
            self.assertEqual(failed.failure_code, "model_fit_oracle.constant_indicator")
            self.assertIsNone(failed.srmr)
            for summary in variant.criteria:
                self.assertEqual(summary.requested_replicates, 10)
                self.assertEqual(summary.minimum_usable_replicates, 9)
                self.assertEqual(summary.usable_replicates, 9)
                self.assertEqual(summary.failed_replicates, 1)
                self.assertEqual(summary.status, "available")

    def test_row_and_column_permutations_preserve_point_and_mapped_plan_results(self) -> None:
        baseline = fit_pls_model(
            self.synthetic, self.synthetic_model, tolerance=1e-9
        )
        row_permutation = np.random.default_rng(2026081513).permutation(
            self.synthetic.shape[0]
        )
        row_point = fit_pls_model(
            self.synthetic[row_permutation], self.synthetic_model, tolerance=1e-9
        )
        for variant in ("saturated", "estimated"):
            left = getattr(baseline, variant)
            right = getattr(row_point, variant)
            for field in (
                "srmr",
                "d_uls",
                "d_g",
                "chi_square",
                "degrees_of_freedom",
                "nfi",
            ):
                self.assertAlmostEqual(getattr(left, field), getattr(right, field), places=10)

        draws = 8
        plans = {
            variant: build_index_plan(self.synthetic.shape[0], draws, 2026081514, variant)
            for variant in ("saturated", "estimated")
        }
        exact = run_adapted_bollen_stine(
            self.synthetic,
            self.synthetic_model,
            requested_replicates=draws,
            seed=2026081514,
            tolerance=1e-9,
            index_plans=plans,
        )
        inverse = np.empty_like(row_permutation)
        inverse[row_permutation] = np.arange(row_permutation.size)
        mapped = {
            variant: tuple(inverse[np.asarray(indices, dtype=np.intp)] for indices in plan)
            for variant, plan in plans.items()
        }
        row_exact = run_adapted_bollen_stine(
            self.synthetic[row_permutation],
            self.synthetic_model,
            requested_replicates=draws,
            seed=2026081514,
            tolerance=1e-9,
            index_plans=mapped,
        )
        for variant in ("saturated", "estimated"):
            for baseline_summary, permuted_summary in zip(
                getattr(exact, variant).criteria,
                getattr(row_exact, variant).criteria,
            ):
                self.assertAlmostEqual(
                    baseline_summary.upper_95 or 0,
                    permuted_summary.upper_95 or 0,
                    places=10,
                )
                self.assertAlmostEqual(
                    baseline_summary.upper_99 or 0,
                    permuted_summary.upper_99 or 0,
                    places=10,
                )
                self.assertEqual(
                    baseline_summary.not_rejected_95,
                    permuted_summary.not_rejected_95,
                )

        column_order = np.array([1, 0, 3, 2, 5, 4])
        column_model = ModelSpec(
            constructs=tuple(
                ConstructSpec(
                    construct.construct_id,
                    tuple(reversed(construct.indicators)),
                    construct.mode,
                )
                for construct in self.synthetic_model.constructs
            ),
            paths=self.synthetic_model.paths,
        )
        column_point = fit_pls_model(
            self.synthetic[:, column_order], column_model, tolerance=1e-9
        )
        for variant in ("saturated", "estimated"):
            left = getattr(baseline, variant)
            right = getattr(column_point, variant)
            for field in ("srmr", "d_uls", "d_g", "chi_square", "nfi"):
                self.assertAlmostEqual(getattr(left, field), getattr(right, field), places=10)
        column_exact = run_adapted_bollen_stine(
            self.synthetic[:, column_order],
            column_model,
            requested_replicates=draws,
            seed=2026081514,
            tolerance=1e-9,
            index_plans=plans,
        )
        for variant in ("saturated", "estimated"):
            for baseline_summary, permuted_summary in zip(
                getattr(exact, variant).criteria,
                getattr(column_exact, variant).criteria,
            ):
                self.assertAlmostEqual(
                    baseline_summary.upper_95 or 0,
                    permuted_summary.upper_95 or 0,
                    places=10,
                )
                self.assertAlmostEqual(
                    baseline_summary.upper_99 or 0,
                    permuted_summary.upper_99 or 0,
                    places=10,
                )
                self.assertEqual(
                    baseline_summary.not_rejected_99,
                    permuted_summary.not_rejected_99,
                )

    def test_non_positive_definite_and_singular_inputs_fail_with_typed_codes(self) -> None:
        with self.assertRaises(ModelFitOracleError) as non_pd:
            null_transform(
                self.synthetic[:, :2],
                [[1.0, 1.0], [1.0, 1.0]],
            )
        self.assertEqual(
            non_pd.exception.code,
            "model_fit_oracle.target_correlation_not_positive_definite",
        )

        singular = np.column_stack(
            [
                self.synthetic[:, 0],
                self.synthetic[:, 0],
                self.synthetic[:, 2],
                self.synthetic[:, 3],
            ]
        )
        model = ModelSpec(
            constructs=(
                ConstructSpec("x", ("x1", "x2")),
                ConstructSpec("m", ("m1", "m2")),
            ),
            paths=(("x", "m"),),
        )
        with self.assertRaises(ModelFitOracleError) as singular_error:
            fit_pls_model(singular, model)
        self.assertEqual(
            singular_error.exception.code,
            "model_fit_oracle.observed_correlation_not_positive_definite",
        )

    def test_generated_work_report_is_schema_valid_and_fail_closed_for_promotion(self) -> None:
        report_path = (
            VALIDATION_DIR
            / "results"
            / "method_factory"
            / "pls_model_fit_v2"
            / "work"
            / "independent_full_refit_oracle.json"
        )
        schema_path = VALIDATION_DIR / "pls_model_fit_full_refit_oracle.schema.json"
        report = json.loads(report_path.read_text(encoding="utf-8"))
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
        jsonschema.Draft202012Validator(schema).validate(report)

        self.assertTrue(report["passed"])
        self.assertFalse(report["qualification_ready"])
        self.assertFalse(report["promotion_requested"])
        source_path = REPOSITORY_ROOT / report["reference"]["source"]
        self.assertEqual(
            report["reference"]["source_sha256"],
            hashlib.sha256(source_path.read_bytes()).hexdigest(),
        )
        for path_key, digest_key in (
            ("recipe", "recipe_sha256"),
            ("data", "data_sha256"),
            ("frozen_product", "frozen_product_sha256"),
        ):
            artifact_path = REPOSITORY_ROOT / report["fixtures"][path_key]
            self.assertEqual(
                report["fixtures"][digest_key],
                hashlib.sha256(artifact_path.read_bytes()).hexdigest(),
            )
        blockers = set(report["blockers"])
        self.assertIn(
            "calibration.preregistered_type_i_error_power_coverage_and_failure_rate_not_run",
            blockers,
        )
        self.assertIn(
            "packaged_windows.installed_portable_accessibility_scaling_cancellation_not_run",
            blockers,
        )
        self.assertIn(
            "performance.maximum_axis_compound_stress_soak_and_leak_not_run",
            blockers,
        )


if __name__ == "__main__":
    unittest.main()
