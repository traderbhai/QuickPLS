#!/usr/bin/env python3
"""Validation-only generative and adversarial work evidence for PLS model fit.

The reports produced here are deliberately not QualificationSpec V2 receipts.
They exercise the independent NumPy/SciPy full-refit oracle with deterministic
generated data, Wilson Monte Carlo intervals, metamorphic transformations, and
typed boundary failures.  Product-minimum resampling, full PLS/PLSc breadth,
and the required Monte Carlo precision remain explicit blockers.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import sys
from dataclasses import asdict
from pathlib import Path
from statistics import NormalDist
from typing import Callable

import numpy as np


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
SPEC_PATH = VALIDATION / "qualification_v2" / "pls_model_fit_exact_v1.qualification.json"
WORK = VALIDATION / "results" / "method_factory" / "pls_model_fit_v2" / "work"
SIMULATION_REPORT = WORK / "generative_calibration.json"
ADVERSARIAL_REPORT = WORK / "adversarial_matrix.json"
sys.path.insert(0, str(VALIDATION))

from pls_model_fit_full_refit_oracle import (  # noqa: E402
    ConstructSpec,
    ModelFitOracleError,
    ModelSpec,
    build_index_plan,
    compare_frozen_product_point_fit,
    fit_pls_model,
    model_from_recipe_document,
    null_transform,
    read_csv_matrix,
    run_adapted_bollen_stine,
)
from pls_model_fit_v2_reference import fit_values  # noqa: E402
from qualification_spec_v2 import canonical_sha256, strict_load_json  # noqa: E402


EVIDENCE_VERSION = "pls_model_fit_v2_qualification_work_evidence_v1"


def _sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def wilson_interval(
    successes: int,
    trials: int,
    confidence: float = 0.95,
) -> dict[str, float | int]:
    if trials < 1 or not 0 <= successes <= trials or not 0 < confidence < 1:
        raise ValueError("invalid Wilson interval inputs")
    z = NormalDist().inv_cdf(0.5 + confidence / 2)
    probability = successes / trials
    denominator = 1 + z * z / trials
    center = (probability + z * z / (2 * trials)) / denominator
    half_width = (
        z
        * math.sqrt(
            probability * (1 - probability) / trials + z * z / (4 * trials * trials)
        )
        / denominator
    )
    return {
        "successes": successes,
        "trials": trials,
        "estimate": probability,
        "lower": max(0.0, center - half_width),
        "upper": min(1.0, center + half_width),
        "half_width": half_width,
        "confidence": confidence,
    }


def _model() -> ModelSpec:
    return ModelSpec(
        constructs=(
            ConstructSpec("x", ("x1", "x2")),
            ConstructSpec("m", ("m1", "m2")),
            ConstructSpec("y", ("y1", "y2")),
        ),
        paths=(("x", "m"), ("m", "y")),
    )


def generated_observations(
    seed: int,
    row_count: int,
    *,
    misspecified: bool,
    distribution: str,
) -> np.ndarray:
    if row_count < 10:
        raise ValueError("generated scenarios require at least ten rows")
    generator = np.random.Generator(np.random.PCG64(seed))

    def noise(size: int) -> np.ndarray:
        if distribution == "gaussian":
            return generator.normal(size=size)
        if distribution == "skewed_heavy_tail":
            raw = generator.standard_t(df=7, size=size)
            skew = generator.lognormal(mean=0.0, sigma=0.35, size=size)
            combined = 0.72 * raw + 0.28 * (skew - np.mean(skew))
            return (combined - np.mean(combined)) / np.std(combined, ddof=1)
        raise ValueError(f"unknown distribution {distribution!r}")

    x = noise(row_count)
    mediator = 0.58 * x + 0.82 * noise(row_count)
    direct = 0.34 * x if misspecified else 0.0
    outcome = direct + 0.52 * mediator + 0.74 * noise(row_count)
    return np.column_stack(
        [
            0.86 * x + 0.42 * noise(row_count),
            0.78 * x + 0.50 * noise(row_count),
            0.84 * mediator + 0.44 * noise(row_count),
            0.76 * mediator + 0.52 * noise(row_count),
            0.87 * outcome + 0.40 * noise(row_count),
            0.75 * outcome + 0.55 * noise(row_count),
        ]
    )


def _metric_reference_error(point: object) -> float:
    errors: list[float] = []
    for variant in ("saturated", "estimated"):
        implied = getattr(point, f"{variant}_implied_correlation")
        expected = fit_values(
            point.observed_correlation,
            implied,
            point.analytical_sample_size,
        )
        actual = getattr(point, variant)
        for field in ("srmr", "d_uls", "d_g", "chi_square", "nfi"):
            expected_value = getattr(expected, field)
            if expected_value is None:
                raise AssertionError(f"generated {variant}.{field} unexpectedly unavailable")
            errors.append(abs(getattr(actual, field) - expected_value))
    return max(errors)


def _path_map(point: object) -> dict[tuple[str, str], float]:
    return {
        (source, target): coefficient
        for source, target, coefficient in point.path_coefficients
    }


def _criterion_rows(exact_results: list[object], variant: str) -> list[dict[str, object]]:
    rows = []
    for criterion in ("srmr", "d_uls", "d_g"):
        summaries = [
            next(
                summary
                for summary in getattr(result, variant).criteria
                if summary.criterion == criterion
            )
            for result in exact_results
        ]
        available = [summary for summary in summaries if summary.status == "available"]
        rejected_95 = sum(summary.not_rejected_95 is False for summary in available)
        rejected_99 = sum(summary.not_rejected_99 is False for summary in available)
        rows.append(
            {
                "variant": variant,
                "criterion": criterion,
                "requested_datasets": len(summaries),
                "available_datasets": len(available),
                "failed_or_unavailable_datasets": len(summaries) - len(available),
                "rejection_0_05": wilson_interval(rejected_95, len(available))
                if available
                else None,
                "rejection_0_01": wilson_interval(rejected_99, len(available))
                if available
                else None,
            }
        )
    return rows


def build_generative_report(
    *,
    point_replicates: int = 96,
    exact_datasets_per_condition: int = 12,
    exact_draws: int = 19,
) -> dict[str, object]:
    if point_replicates < 4 or exact_datasets_per_condition < 2 or exact_draws < 2:
        raise ValueError("work-evidence counts are below the deterministic test minimum")
    spec = strict_load_json(SPEC_PATH)
    model = _model()
    population_paths: dict[str, dict[tuple[str, str], float]] = {}
    for distribution_index, distribution in enumerate(("gaussian", "skewed_heavy_tail")):
        population = generated_observations(
            202608152000 + distribution_index,
            20_000,
            misspecified=False,
            distribution=distribution,
        )
        population_paths[distribution] = _path_map(
            fit_pls_model(population, model, tolerance=1e-10)
        )

    point_rows = []
    maximum_arithmetic_error = 0.0
    path_errors: dict[str, dict[tuple[str, str], list[float]]] = {
        distribution: {path: [] for path in model.paths}
        for distribution in population_paths
    }
    point_failures = []
    for replicate in range(point_replicates):
        distribution = ("gaussian", "skewed_heavy_tail")[replicate % 2]
        seed = 202608153000 + replicate
        try:
            point = fit_pls_model(
                generated_observations(
                    seed,
                    100,
                    misspecified=False,
                    distribution=distribution,
                ),
                model,
                tolerance=1e-9,
            )
            arithmetic_error = _metric_reference_error(point)
            maximum_arithmetic_error = max(maximum_arithmetic_error, arithmetic_error)
            estimated_paths = _path_map(point)
            for path in model.paths:
                path_errors[distribution][path].append(
                    estimated_paths[path] - population_paths[distribution][path]
                )
            point_rows.append(
                {
                    "replicate_index": replicate,
                    "seed": seed,
                    "distribution": distribution,
                    "status": "success",
                    "matrix_criterion_maximum_absolute_error": arithmetic_error,
                }
            )
        except ModelFitOracleError as error:
            point_failures.append(
                {
                    "replicate_index": replicate,
                    "seed": seed,
                    "distribution": distribution,
                    "code": error.code,
                    "message": error.message,
                }
            )
            point_rows.append(
                {
                    "replicate_index": replicate,
                    "seed": seed,
                    "distribution": distribution,
                    "status": "failed",
                    "matrix_criterion_maximum_absolute_error": None,
                }
            )

    recovery_rows = []
    for distribution, by_path in path_errors.items():
        for (source, target), errors in by_path.items():
            vector = np.asarray(errors, dtype=float)
            recovery_rows.append(
                {
                    "distribution": distribution,
                    "source": source,
                    "target": target,
                    "population_proxy": population_paths[distribution][(source, target)],
                    "successful_replicates": len(errors),
                    "mean_error": float(np.mean(vector)) if vector.size else None,
                    "rmse": float(np.sqrt(np.mean(vector**2))) if vector.size else None,
                    "maximum_absolute_error": float(np.max(np.abs(vector)))
                    if vector.size
                    else None,
                }
            )

    exact_by_condition: dict[str, list[object]] = {"correct": [], "misspecified": []}
    exact_failures = []
    for condition_index, condition in enumerate(("correct", "misspecified")):
        for dataset_index in range(exact_datasets_per_condition):
            seed = 202608154000 + condition_index * 10_000 + dataset_index
            try:
                exact_by_condition[condition].append(
                    run_adapted_bollen_stine(
                        generated_observations(
                            seed,
                            100,
                            misspecified=condition == "misspecified",
                            distribution=("gaussian", "skewed_heavy_tail")[
                                dataset_index % 2
                            ],
                        ),
                        model,
                        requested_replicates=exact_draws,
                        seed=seed + 1_000_000,
                        tolerance=1e-8,
                    )
                )
            except ModelFitOracleError as error:
                exact_failures.append(
                    {
                        "condition": condition,
                        "dataset_index": dataset_index,
                        "seed": seed,
                        "code": error.code,
                        "message": error.message,
                    }
                )

    calibration_rows = []
    for condition, results in exact_by_condition.items():
        for variant in ("saturated", "estimated"):
            for row in _criterion_rows(results, variant):
                row["condition"] = condition
                calibration_rows.append(row)
    interval_half_widths = [
        interval["half_width"]
        for row in calibration_rows
        for interval in (row["rejection_0_05"], row["rejection_0_01"])
        if interval is not None
    ]
    maximum_half_width = max(interval_half_widths, default=1.0)
    point_work_passed = (
        not point_failures
        and maximum_arithmetic_error <= 1e-10
        and all(
            row["mean_error"] is not None and abs(row["mean_error"]) <= 0.12
            for row in recovery_rows
        )
    )
    exact_work_passed = (
        not exact_failures
        and all(
            row["available_datasets"] == exact_datasets_per_condition
            for row in calibration_rows
        )
    )
    blockers = [
        "generative.product_minimum_999_draws_not_used",
        "generative.monte_carlo_interval_half_width_exceeds_one_percentage_point",
        "generative.plsc_formative_mixed_higher_order_and_interaction_axes_not_run",
        "generative.worker_count_and_product_engine_execution_not_run",
        "generative.failed_fit_rate_not_calibrated_at_qualification_scale",
    ]
    return {
        "schema_version": 1,
        "kind": "pls_model_fit_v2_generative_calibration_work_v1",
        "evidence_version": EVIDENCE_VERSION,
        "passed_work_checks": point_work_passed and exact_work_passed,
        "qualification_role_satisfied": False,
        "receipt_eligible": False,
        "scenario_set_sha256": canonical_sha256(spec["scenario_contract"]),
        "source": "validation/pls_model_fit_v2_qualification_evidence.py",
        "source_sha256": _sha256(Path(__file__).resolve()),
        "design": {
            "point_replicates": point_replicates,
            "point_rows": 100,
            "population_proxy_rows": 20_000,
            "exact_datasets_per_condition": exact_datasets_per_condition,
            "exact_draws": exact_draws,
            "conditions": ["correct", "misspecified"],
            "distributions": ["gaussian", "skewed_heavy_tail"],
            "confidence": 0.95,
            "qualification_maximum_half_width": 0.01,
        },
        "point_recovery": {
            "maximum_independent_arithmetic_error": maximum_arithmetic_error,
            "failed_replicates": point_failures,
            "path_rows": recovery_rows,
            "ledger": point_rows,
        },
        "exact_fit_calibration": {
            "rows": calibration_rows,
            "failed_datasets": exact_failures,
            "maximum_wilson_half_width": maximum_half_width,
            "precision_gate_passed": maximum_half_width <= 0.01,
            "product_draw_minimum_gate_passed": exact_draws >= 999,
        },
        "blockers": blockers,
        "note": (
            "This is calibrated work evidence with explicit Wilson uncertainty, not a "
            "QualificationSpec receipt. Its small exact-fit design is intended to expose "
            "the remaining precision and breadth gap rather than conceal it."
        ),
    }


def _point_scalar_difference(left: object, right: object) -> float:
    values = []
    for variant in ("saturated", "estimated"):
        left_fit = getattr(left, variant)
        right_fit = getattr(right, variant)
        for field in ("srmr", "d_uls", "d_g", "chi_square", "degrees_of_freedom", "nfi"):
            values.append(abs(getattr(left_fit, field) - getattr(right_fit, field)))
    return max(values)


def build_adversarial_report() -> dict[str, object]:
    spec = strict_load_json(SPEC_PATH)
    model = _model()
    observations = generated_observations(
        202608155000,
        100,
        misspecified=True,
        distribution="skewed_heavy_tail",
    )
    baseline = fit_pls_model(observations, model, tolerance=1e-9)
    cases: list[dict[str, object]] = []

    def expected_error(
        case_id: str,
        operation: Callable[[], object],
        expected_codes: set[str],
    ) -> None:
        try:
            operation()
        except ModelFitOracleError as error:
            cases.append(
                {
                    "case_id": case_id,
                    "expected": "typed_failure",
                    "actual": "typed_failure",
                    "code": error.code,
                    "passed": error.code in expected_codes,
                }
            )
        else:
            cases.append(
                {
                    "case_id": case_id,
                    "expected": "typed_failure",
                    "actual": "unexpected_success",
                    "code": None,
                    "passed": False,
                }
            )

    expected_error(
        "zero_variance_indicator",
        lambda: fit_pls_model(
            np.column_stack([np.ones(observations.shape[0]), observations[:, 1:]]),
            model,
        ),
        {"model_fit_oracle.constant_indicator"},
    )
    expected_error(
        "duplicated_indicator_singular_observed",
        lambda: fit_pls_model(
            np.column_stack(
                [observations[:, 0], observations[:, 0], observations[:, 2:]]
            ),
            model,
        ),
        {"model_fit_oracle.observed_correlation_not_positive_definite"},
    )
    nonfinite = observations.copy()
    nonfinite[0, 0] = np.nan
    expected_error(
        "non_finite_raw_cell",
        lambda: fit_pls_model(nonfinite, model),
        {"model_fit_oracle.non_finite_raw_data"},
    )
    expected_error(
        "n_less_than_p_singular_correlation",
        lambda: fit_pls_model(observations[:5], model),
        {"model_fit_oracle.observed_correlation_not_positive_definite"},
    )
    expected_error(
        "non_positive_definite_target",
        lambda: null_transform(observations[:, :2], [[1.0, 1.0], [1.0, 1.0]]),
        {"model_fit_oracle.target_correlation_not_positive_definite"},
    )
    expected_error(
        "nonrecursive_cycle",
        lambda: fit_pls_model(
            observations,
            ModelSpec(model.constructs, (*model.paths, ("y", "x"))),
        ),
        {"model_fit_oracle.nonrecursive_model"},
    )
    expected_error(
        "duplicate_structural_path",
        lambda: fit_pls_model(
            observations,
            ModelSpec(model.constructs, (*model.paths, model.paths[0])),
        ),
        {"model_fit_oracle.duplicate_path"},
    )
    expected_error(
        "incomplete_explicit_index_plans",
        lambda: run_adapted_bollen_stine(
            observations,
            model,
            requested_replicates=4,
            seed=202608155001,
            index_plans={
                "saturated": build_index_plan(
                    observations.shape[0], 4, 202608155001, "saturated"
                )
            },
        ),
        {"model_fit_oracle.incomplete_index_plans"},
    )
    invalid_plans = {
        variant: list(build_index_plan(observations.shape[0], 4, 202608155002, variant))
        for variant in ("saturated", "estimated")
    }
    invalid_plans["estimated"][2] = np.full(
        observations.shape[0], observations.shape[0], dtype=np.uint64
    )
    expected_error(
        "out_of_range_sample_index",
        lambda: run_adapted_bollen_stine(
            observations,
            model,
            requested_replicates=4,
            seed=202608155002,
            index_plans=invalid_plans,
        ),
        {"model_fit_oracle.invalid_sample_indices"},
    )

    affine = observations * np.array([1e-6, 1e6, 0.1, 10.0, 2.5, 0.4]) + np.array(
        [1e-4, -1e7, 25.0, -40.0, 0.0, 300.0]
    )
    affine_difference = _point_scalar_difference(
        baseline, fit_pls_model(affine, model, tolerance=1e-9)
    )
    cases.append(
        {
            "case_id": "positive_affine_extreme_scale_invariance",
            "expected": "maximum_scalar_difference_le_1e-7",
            "actual": affine_difference,
            "passed": affine_difference <= 1e-7,
        }
    )
    permutation = np.random.default_rng(202608155003).permutation(observations.shape[0])
    row_difference = _point_scalar_difference(
        baseline, fit_pls_model(observations[permutation], model, tolerance=1e-9)
    )
    cases.append(
        {
            "case_id": "row_permutation_invariance",
            "expected": "maximum_scalar_difference_le_1e-10",
            "actual": row_difference,
            "passed": row_difference <= 1e-10,
        }
    )
    near_collinear = observations.copy()
    near_collinear[:, 1] = near_collinear[:, 0] + np.random.default_rng(
        202608155004
    ).normal(scale=1e-5, size=observations.shape[0])
    try:
        near_point = fit_pls_model(near_collinear, model, tolerance=1e-9)
        finite = all(
            math.isfinite(getattr(getattr(near_point, variant), criterion))
            for variant in ("saturated", "estimated")
            for criterion in ("srmr", "d_uls", "d_g", "chi_square", "nfi")
        )
        cases.append(
            {
                "case_id": "near_collinear_finite_or_typed_failure",
                "expected": "finite_result_or_typed_failure",
                "actual": "finite_result" if finite else "non_finite_result",
                "passed": finite,
            }
        )
    except ModelFitOracleError as error:
        cases.append(
            {
                "case_id": "near_collinear_finite_or_typed_failure",
                "expected": "finite_result_or_typed_failure",
                "actual": "typed_failure",
                "code": error.code,
                "passed": True,
            }
        )

    failure_plans = {
        variant: list(build_index_plan(observations.shape[0], 10, 202608155005, variant))
        for variant in ("saturated", "estimated")
    }
    for variant in failure_plans:
        failure_plans[variant][0] = np.zeros(observations.shape[0], dtype=np.uint64)
        failure_plans[variant][1] = np.ones(observations.shape[0], dtype=np.uint64)
    failure_result = run_adapted_bollen_stine(
        observations,
        model,
        requested_replicates=10,
        seed=202608155005,
        tolerance=1e-8,
        index_plans=failure_plans,
    )
    fixed_ledger_passed = all(
        len(getattr(failure_result, variant).ledger) == 10
        and sum(entry.status == "failed" for entry in getattr(failure_result, variant).ledger)
        == 2
        and all(
            summary.status == "unavailable"
            and summary.usable_replicates == 8
            and summary.failed_replicates == 2
            and summary.upper_95 is None
            for summary in getattr(failure_result, variant).criteria
        )
        for variant in ("saturated", "estimated")
    )
    cases.append(
        {
            "case_id": "fixed_failure_ledger_below_ninety_percent",
            "expected": "ten_cells_two_failures_no_decision",
            "actual": {
                variant: [
                    asdict(summary) for summary in getattr(failure_result, variant).criteria
                ]
                for variant in ("saturated", "estimated")
            },
            "passed": fixed_ledger_passed,
        }
    )

    recipe_path = VALIDATION / "fixtures" / "simple_reflective.recipe.json"
    data_path = VALIDATION / "fixtures" / "simple_reflective.csv"
    product_path = VALIDATION / "results" / "pls_quickpls_path_mode_a.json"
    recipe = strict_load_json(recipe_path)
    fixture_model = model_from_recipe_document(recipe)
    fixture_point = fit_pls_model(
        read_csv_matrix(data_path, fixture_model.indicator_order),
        fixture_model,
        tolerance=float(recipe["settings"]["tolerance"]),
        max_iterations=int(recipe["settings"]["max_iterations"]),
    )
    product = strict_load_json(product_path)
    tampered = copy.deepcopy(product)
    tampered["payload"]["assessment"]["model_fit"]["saturated"]["srmr"] += 0.001
    tamper_comparison = compare_frozen_product_point_fit(
        fixture_point, fixture_model, tampered
    )
    cases.append(
        {
            "case_id": "frozen_product_scalar_tamper_detected",
            "expected": "comparison_failed",
            "actual": "comparison_failed"
            if not tamper_comparison["passed"]
            else "unexpected_pass",
            "passed": not tamper_comparison["passed"],
        }
    )
    passed = all(bool(case["passed"]) for case in cases)
    return {
        "schema_version": 1,
        "kind": "pls_model_fit_v2_adversarial_work_v1",
        "evidence_version": EVIDENCE_VERSION,
        "passed_work_checks": passed,
        "qualification_role_satisfied": False,
        "receipt_eligible": False,
        "scenario_set_sha256": canonical_sha256(spec["scenario_contract"]),
        "source": "validation/pls_model_fit_v2_qualification_evidence.py",
        "source_sha256": _sha256(Path(__file__).resolve()),
        "case_count": len(cases),
        "cases": cases,
        "blockers": [
            "adversarial.product_engine_exact_fit_boundary_run_not_captured",
            "adversarial.archive_reopen_tamper_run_not_captured",
            "adversarial.native_fail_closed_run_not_captured",
            "adversarial.plsc_formative_mixed_higher_order_and_interaction_matrix_incomplete",
            "adversarial.worker_count_gui_cli_and_archive_metamorphic_matrix_incomplete",
        ],
        "note": (
            "All independent-oracle work cases pass, but this report cannot satisfy the "
            "adversarial receipt role until matching product, archive, and native executions "
            "are captured as immutable source-bound evidence."
        ),
    }


def _write_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--point-replicates", type=int, default=96)
    parser.add_argument("--exact-datasets", type=int, default=12)
    parser.add_argument("--exact-draws", type=int, default=19)
    args = parser.parse_args()
    simulation = build_generative_report(
        point_replicates=args.point_replicates,
        exact_datasets_per_condition=args.exact_datasets,
        exact_draws=args.exact_draws,
    )
    adversarial = build_adversarial_report()
    if args.write:
        _write_json(SIMULATION_REPORT, simulation)
        _write_json(ADVERSARIAL_REPORT, adversarial)
    summary = {
        "simulation_work_passed": simulation["passed_work_checks"],
        "simulation_receipt_eligible": simulation["receipt_eligible"],
        "adversarial_work_passed": adversarial["passed_work_checks"],
        "adversarial_receipt_eligible": adversarial["receipt_eligible"],
        "qualification_ready": False,
    }
    print(json.dumps(summary, indent=2, sort_keys=True, allow_nan=False))
    return 0 if all(
        [
            simulation["passed_work_checks"],
            adversarial["passed_work_checks"],
            not simulation["receipt_eligible"],
            not adversarial["receipt_eligible"],
        ]
    ) else 1


if __name__ == "__main__":
    raise SystemExit(main())
