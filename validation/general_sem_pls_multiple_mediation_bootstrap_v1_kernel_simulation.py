#!/usr/bin/env python3
"""Deterministic observed-score reference simulation for mediation bootstrap v1.

This validation-only program is intentionally narrower than a PLS-PM oracle.
It generates an observed-score linear path system containing parallel, serial,
and mixed mediation paths, refits that complete downstream path system with OLS
inside every case bootstrap, and checks path-product recovery, bootstrap means,
Type-7 percentile behavior, deterministic replay, and the 90% usable-replicate
publication boundary. Coefficients are deliberately unstandardized. This smoke
reference never calls or compares production Rust and does not validate PLS
scoring, standardized latent scores, sample-SE or plus-one-probability output,
null calibration, SmartPLS parity, coverage qualification, or release readiness.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import random
from pathlib import Path
from typing import Any


FEATURE_ID = "qpls3.pls.general_sem_multiple_mediation_bootstrap"
METHOD_VERSION = "general_sem_pls_full_model_case_bootstrap_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-19"
GENERATED_AT_UTC = "2026-08-19T04:16:57.0846520Z"
SCRIPT_RELATIVE_PATH = (
    "validation/general_sem_pls_multiple_mediation_bootstrap_v1_kernel_simulation.py"
)
GOVERNANCE_SOURCE_RELATIVE_PATHS = (
    "validation/methods/general_sem_pls_multiple_mediation_bootstrap_v1.manifest.json",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/method_promotion_manifest.py",
    "validation/test_method_promotion_manifest.py",
)

MONTE_CARLO_TRIALS = 96
OBSERVATIONS_PER_TRIAL = 240
BOOTSTRAP_REPLICATES = 199
CONFIDENCE_LEVEL = 0.95
DATA_SEED = 73_104_729
BOOTSTRAP_SEED = 91_700_113

PATHS = {
    "x_m1": 0.5,
    "x_m2": 0.3,
    "m1_m2": 0.4,
    "m1_y": 0.6,
    "m2_y": 0.7,
    "x_y": 0.2,
}
TRUTH = {
    "specific_x_m1_y": PATHS["x_m1"] * PATHS["m1_y"],
    "specific_x_m2_y": PATHS["x_m2"] * PATHS["m2_y"],
    "specific_x_m1_m2_y": PATHS["x_m1"] * PATHS["m1_m2"] * PATHS["m2_y"],
}
TRUTH["total_indirect_x_y"] = math.fsum(TRUTH.values())
TRUTH["total_x_y"] = PATHS["x_y"] + TRUTH["total_indirect_x_y"]

MAX_ABSOLUTE_BIAS = 0.025
MAX_RELATIVE_BIAS = 0.10
MAX_RMSE = 0.15
MAX_ABSOLUTE_MEAN_BOOTSTRAP_BIAS = 0.02
MIN_PERCENTILE_INCLUSION_SMOKE_RATE = 0.85


class SingularPathSystem(ValueError):
    """Raised when a generated or resampled downstream path fit is singular."""


def _standard_normal(rng: random.Random) -> float:
    """Stable Box-Muller draw using only Random.random's frozen MT stream."""

    first = max(rng.random(), 1e-300)
    second = rng.random()
    return math.sqrt(-2.0 * math.log(first)) * math.cos(2.0 * math.pi * second)


def _solve_normal_equations(
    predictors: list[tuple[float, ...]], outcome: list[float]
) -> tuple[float, ...]:
    if len(predictors) != len(outcome) or not predictors:
        raise SingularPathSystem("regression rows are empty or misaligned")
    width = len(predictors[0])
    if width == 0 or any(len(row) != width for row in predictors):
        raise SingularPathSystem("regression design width is invalid")

    augmented = [[0.0 for _ in range(width + 1)] for _ in range(width)]
    for row, target in zip(predictors, outcome, strict=True):
        for left in range(width):
            augmented[left][width] += row[left] * target
            for right in range(width):
                augmented[left][right] += row[left] * row[right]

    for column in range(width):
        pivot = max(range(column, width), key=lambda index: abs(augmented[index][column]))
        if abs(augmented[pivot][column]) <= 1e-12:
            raise SingularPathSystem("downstream path normal equations are singular")
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        pivot_value = augmented[column][column]
        augmented[column] = [value / pivot_value for value in augmented[column]]
        for row_index in range(width):
            if row_index == column:
                continue
            factor = augmented[row_index][column]
            augmented[row_index] = [
                current - factor * pivot_current
                for current, pivot_current in zip(
                    augmented[row_index], augmented[column], strict=True
                )
            ]
    return tuple(row[width] for row in augmented)


def _fit_effects(rows: list[tuple[float, float, float, float]]) -> dict[str, float]:
    x = [row[0] for row in rows]
    m1 = [row[1] for row in rows]
    m2 = [row[2] for row in rows]
    y = [row[3] for row in rows]

    _, x_m1 = _solve_normal_equations([(1.0, value) for value in x], m1)
    _, x_m2, m1_m2 = _solve_normal_equations(
        [(1.0, x_value, m1_value) for x_value, m1_value in zip(x, m1, strict=True)],
        m2,
    )
    _, direct, m1_y, m2_y = _solve_normal_equations(
        [
            (1.0, x_value, m1_value, m2_value)
            for x_value, m1_value, m2_value in zip(x, m1, m2, strict=True)
        ],
        y,
    )

    effects = {
        "specific_x_m1_y": x_m1 * m1_y,
        "specific_x_m2_y": x_m2 * m2_y,
        "specific_x_m1_m2_y": x_m1 * m1_m2 * m2_y,
    }
    effects["total_indirect_x_y"] = math.fsum(effects.values())
    effects["total_x_y"] = direct + effects["total_indirect_x_y"]
    return effects


def _generate_rows(trial_index: int) -> list[tuple[float, float, float, float]]:
    rng = random.Random(DATA_SEED + 104_729 * trial_index)
    rows: list[tuple[float, float, float, float]] = []
    for _ in range(OBSERVATIONS_PER_TRIAL):
        x = _standard_normal(rng)
        m1 = PATHS["x_m1"] * x + 0.8 * _standard_normal(rng)
        m2 = (
            PATHS["x_m2"] * x
            + PATHS["m1_m2"] * m1
            + 0.8 * _standard_normal(rng)
        )
        y = (
            PATHS["x_y"] * x
            + PATHS["m1_y"] * m1
            + PATHS["m2_y"] * m2
            + 0.8 * _standard_normal(rng)
        )
        rows.append((x, m1, m2, y))
    return rows


def _type7(values: list[float], probability: float) -> float:
    ordered = sorted(values)
    location = (len(ordered) - 1) * probability
    lower = math.floor(location)
    upper = math.ceil(location)
    if lower == upper:
        return ordered[lower]
    fraction = location - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def _minimum_usable(requested: int) -> int:
    if not 2 <= requested <= 10_000:
        raise ValueError("requested resamples must be in [2, 10000]")
    return max(2, math.ceil(0.9 * requested))


def _publication_allowed(requested: int, usable: int) -> bool:
    return _minimum_usable(requested) <= usable <= requested


def _trial(trial_index: int) -> dict[str, Any]:
    rows = _generate_rows(trial_index)
    point = _fit_effects(rows)
    rng = random.Random(BOOTSTRAP_SEED + 130_363 * trial_index)
    bootstrap = {effect_id: [] for effect_id in TRUTH}
    failures = 0
    for _ in range(BOOTSTRAP_REPLICATES):
        sampled = [rows[rng.randrange(len(rows))] for _ in range(len(rows))]
        try:
            effects = _fit_effects(sampled)
        except SingularPathSystem:
            failures += 1
            continue
        for effect_id, value in effects.items():
            bootstrap[effect_id].append(value)

    usable = BOOTSTRAP_REPLICATES - failures
    published = _publication_allowed(BOOTSTRAP_REPLICATES, usable)
    intervals: dict[str, tuple[float, float]] = {}
    bootstrap_means: dict[str, float] = {}
    if published:
        alpha = 1.0 - CONFIDENCE_LEVEL
        for effect_id, values in bootstrap.items():
            intervals[effect_id] = (
                _type7(values, alpha / 2.0),
                _type7(values, 1.0 - alpha / 2.0),
            )
            bootstrap_means[effect_id] = math.fsum(values) / len(values)
    return {
        "point": point,
        "intervals": intervals,
        "bootstrap_means": bootstrap_means,
        "failures": failures,
        "usable": usable,
        "published": published,
    }


def _rounded(value: float) -> float:
    return round(value, 12)


def _wilson_interval(successes: int, trials: int) -> tuple[float, float]:
    z = 1.959963984540054
    proportion = successes / trials
    denominator = 1.0 + z * z / trials
    center = (proportion + z * z / (2.0 * trials)) / denominator
    half_width = (
        z
        * math.sqrt(
            proportion * (1.0 - proportion) / trials
            + z * z / (4.0 * trials * trials)
        )
        / denominator
    )
    return center - half_width, center + half_width


def _run_simulation() -> tuple[dict[str, Any], dict[str, bool]]:
    trials = [_trial(index) for index in range(MONTE_CARLO_TRIALS)]
    metrics: dict[str, Any] = {}
    max_reconciliation_error = 0.0
    for trial in trials:
        point = trial["point"]
        max_reconciliation_error = max(
            max_reconciliation_error,
            abs(
                point["total_indirect_x_y"]
                - math.fsum(
                    [
                        point["specific_x_m1_y"],
                        point["specific_x_m2_y"],
                        point["specific_x_m1_m2_y"],
                    ]
                )
            ),
        )

    for effect_id, truth in TRUTH.items():
        estimates = [trial["point"][effect_id] for trial in trials]
        biases = [estimate - truth for estimate in estimates]
        coverage = [
            trial["intervals"][effect_id][0]
            <= truth
            <= trial["intervals"][effect_id][1]
            for trial in trials
            if trial["published"]
        ]
        widths = [
            trial["intervals"][effect_id][1] - trial["intervals"][effect_id][0]
            for trial in trials
            if trial["published"]
        ]
        bootstrap_biases = [
            trial["bootstrap_means"][effect_id] - trial["point"][effect_id]
            for trial in trials
            if trial["published"]
        ]
        bias = math.fsum(biases) / len(biases)
        coverage_hits = sum(coverage)
        wilson_lower, wilson_upper = _wilson_interval(coverage_hits, len(coverage))
        metrics[effect_id] = {
            "truth": _rounded(truth),
            "mean_estimate": _rounded(math.fsum(estimates) / len(estimates)),
            "bias": _rounded(bias),
            "absolute_relative_bias": _rounded(abs(bias) / abs(truth)),
            "rmse": _rounded(
                math.sqrt(math.fsum(value * value for value in biases) / len(biases))
            ),
            "percentile_95_coverage": _rounded(sum(coverage) / len(coverage)),
            "percentile_95_coverage_hits": coverage_hits,
            "percentile_95_coverage_trials": len(coverage),
            "percentile_95_coverage_wilson_lower": _rounded(wilson_lower),
            "percentile_95_coverage_wilson_upper": _rounded(wilson_upper),
            "mean_interval_width": _rounded(math.fsum(widths) / len(widths)),
            "mean_bootstrap_bias": _rounded(
                math.fsum(bootstrap_biases) / len(bootstrap_biases)
            ),
        }

    first_replay = _trial(0)
    second_replay = _trial(0)
    singular_rejected = False
    try:
        _fit_effects([(0.0, 0.0, 0.0, 0.0)] * OBSERVATIONS_PER_TRIAL)
    except SingularPathSystem:
        singular_rejected = True

    checks = {
        "deterministic_replay": first_replay == second_replay,
        "all_trials_publish": all(trial["published"] for trial in trials),
        "all_bootstrap_refits_usable": all(trial["failures"] == 0 for trial in trials),
        "specific_and_total_effects_reconcile": max_reconciliation_error <= 1e-12,
        "absolute_bias_within_bound": all(
            abs(row["bias"]) <= MAX_ABSOLUTE_BIAS for row in metrics.values()
        ),
        "relative_bias_within_bound": all(
            row["absolute_relative_bias"] <= MAX_RELATIVE_BIAS
            for row in metrics.values()
        ),
        "rmse_within_bound": all(row["rmse"] <= MAX_RMSE for row in metrics.values()),
        "percentile_truth_inclusion_smoke_above_floor": all(
            MIN_PERCENTILE_INCLUSION_SMOKE_RATE <= row["percentile_95_coverage"] <= 1.0
            for row in metrics.values()
        ),
        "bootstrap_bias_within_bound": all(
            abs(row["mean_bootstrap_bias"]) <= MAX_ABSOLUTE_MEAN_BOOTSTRAP_BIAS
            for row in metrics.values()
        ),
        "interval_widths_positive": all(
            row["mean_interval_width"] > 0.0 for row in metrics.values()
        ),
        "usable_gate_accepts_exact_boundary": _publication_allowed(20, 18),
        "usable_gate_rejects_below_boundary": not _publication_allowed(20, 17),
        "usable_gate_preserves_minimum_two": _publication_allowed(2, 2)
        and not _publication_allowed(2, 1),
        "singular_path_system_rejected": singular_rejected,
    }
    metrics["run_summary"] = {
        "trials_requested": MONTE_CARLO_TRIALS,
        "trials_published": sum(trial["published"] for trial in trials),
        "bootstrap_replicates_per_trial": BOOTSTRAP_REPLICATES,
        "bootstrap_refit_failures": sum(trial["failures"] for trial in trials),
        "maximum_effect_reconciliation_error": _rounded(max_reconciliation_error),
    }
    return metrics, checks


def _source_descriptors() -> list[dict[str, Any]]:
    repository_root = Path(__file__).resolve().parents[1]
    descriptors = []
    for relative in (SCRIPT_RELATIVE_PATH, *GOVERNANCE_SOURCE_RELATIVE_PATHS):
        payload = (repository_root / relative).read_bytes()
        descriptors.append(
            {
                "path": relative,
                "size": len(payload),
                "sha256": hashlib.sha256(payload).hexdigest(),
            }
        )
    return descriptors


def build_report() -> dict[str, Any]:
    metrics, checks = _run_simulation()
    return {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": "simulation_report",
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "generated_at_utc": GENERATED_AT_UTC,
        "passed": all(checks.values()),
        "qualification_ready": False,
        "evidence_scope": "independent_unstandardized_observed_score_estimand_percentile_smoke_only",
        "design": {
            "system": "mixed parallel and serial unstandardized observed-score linear mediation",
            "estimator": "independent complete unstandardized OLS path-system refit per case bootstrap",
            "monte_carlo_trials": MONTE_CARLO_TRIALS,
            "observations_per_trial": OBSERVATIONS_PER_TRIAL,
            "bootstrap_replicates_per_trial": BOOTSTRAP_REPLICATES,
            "confidence_level": CONFIDENCE_LEVEL,
            "interval": "percentile_type7",
            "data_seed": str(DATA_SEED),
            "bootstrap_seed": str(BOOTSTRAP_SEED),
        },
        "acceptance": {
            "maximum_absolute_bias": MAX_ABSOLUTE_BIAS,
            "maximum_absolute_relative_bias": MAX_RELATIVE_BIAS,
            "maximum_rmse": MAX_RMSE,
            "maximum_absolute_mean_bootstrap_bias": MAX_ABSOLUTE_MEAN_BOOTSTRAP_BIAS,
            "minimum_percentile_95_truth_inclusion_smoke_rate": MIN_PERCENTILE_INCLUSION_SMOKE_RATE,
            "minimum_usable_rule": "max(2, ceil(0.9 * requested))",
        },
        "checks": checks,
        "metrics": metrics,
        "limitations": [
            "This independent smoke reference never calls production Rust or compares a shared Rust-Python bootstrap ledger.",
            "It covers only unstandardized observed-score path products, bootstrap means, Type-7 percentile behavior, and the usable-replicate gate.",
            "PLS path scores are standardized; this report is not a PLS-PM scoring or full-refit oracle and does not validate latent-score recovery.",
            "Sample standard errors, plus-one probabilities, and null-rejection calibration are outside this simulation.",
            "Ninety-six trials provide a deterministic engine smoke gate, not qualification-scale interval-coverage evidence.",
            "Wilson intervals are descriptive Monte Carlo uncertainty only; interval-width, efficiency, and standard-error calibration are excluded.",
            "It is not SmartPLS numerical-parity, native qualification, packaged acceptance, or release evidence.",
            "Qualification-scale PLS simulations across loading, reliability, nonnormality, missingness, and failure-rate axes remain pending.",
        ],
        "source_artifacts": _source_descriptors(),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check-report", type=Path)
    args = parser.parse_args()
    report = build_report()
    if args.check_report is not None:
        checked = json.loads(args.check_report.read_text(encoding="utf-8"))
        if checked != report:
            print(
                json.dumps(
                    {
                        "passed": False,
                        "error": "checked report does not equal deterministic simulation output",
                    },
                    indent=2,
                    sort_keys=True,
                )
            )
            return 1
        print(
            json.dumps(
                {
                    "passed": True,
                    "checked_report": str(args.check_report),
                    "simulation_passed": report["passed"],
                    "source_sha256": report["source_artifacts"][0]["sha256"],
                },
                indent=2,
                sort_keys=True,
            )
        )
        return 0 if report["passed"] else 1
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
