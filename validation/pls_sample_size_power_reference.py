#!/usr/bin/env python3
"""Independent prospective Monte Carlo reference for the bounded QuickPLS v1 design.

This module intentionally does not import or invoke the QuickPLS executable.  It
implements the frozen two-construct reflective Gaussian generator, Mode-A PLS
path estimate, case-bootstrap normal-reference test, Wilson interval, failure
denominator, and conservative grid decision using NumPy and the Python standard
library.  Its random generator is independently implemented; validation compares
statistical behavior and equations, not byte-identical pseudo-random samples.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path
from statistics import NormalDist
from typing import Any

import numpy as np


METHOD_VERSION = "pls_sample_size_power_v1"
CAPABILITY_ID = "qpls3.pls.sample_size_power"
STREAM_DOMAIN = "quickpls/pls_sample_size_power_v1/independent_python"
FAILURE_POLICY = "failed_replicates_count_as_non_rejections_v1"
INTERVAL_METHOD = "wilson_score_two_sided_v1"
INFERENCE_METHOD = "pls_pm_case_bootstrap_normal_reference_two_sided_v1"
MAX_ESTIMATED_PLS_FITS = 250_000
MAX_ESTIMATED_PLS_CASE_FITS = 100_000_000


class ReferenceInputError(ValueError):
    """Raised when a recipe falls outside the frozen independent-reference scope."""


@dataclass(frozen=True)
class ReferenceOutcome:
    sample_size: int
    replicate_index: int
    stream_identity: str
    successful: bool
    converged: bool
    target_estimate: float | None
    p_value_two_sided: float | None
    rejected: bool
    failure_code: str | None
    failure_message: str | None

    def as_dict(self) -> dict[str, Any]:
        return {
            "sample_size": self.sample_size,
            "replicate_index": self.replicate_index,
            "stream_identity": self.stream_identity,
            "attempted": True,
            "successful": self.successful,
            "converged": self.converged,
            "target_estimate": self.target_estimate,
            "p_value_two_sided": self.p_value_two_sided,
            "rejected": self.rejected,
            "failure_code": self.failure_code,
            "failure_message": self.failure_message,
        }


def _require(condition: bool, field: str, message: str) -> None:
    if not condition:
        raise ReferenceInputError(f"{field}: {message}")


def _finite_number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value))


def validate_recipe(recipe: dict[str, Any], *, enforce_production_counts: bool = True) -> None:
    expected_top = {
        "schema_version",
        "capability_id",
        "method_version",
        "scenario_identity",
        "design",
        "estimator",
        "inference",
        "sample_size_grid",
        "alpha",
        "target_power",
        "confidence_level",
        "monte_carlo_replicates",
        "bootstrap_replicates",
        "master_seed",
        "workers",
    }
    _require(set(recipe) == expected_top, "recipe", "unknown or missing fields")
    _require(recipe["schema_version"] == 1, "schema_version", "must equal 1")
    _require(recipe["capability_id"] == CAPABILITY_ID, "capability_id", "identity mismatch")
    _require(recipe["method_version"] == METHOD_VERSION, "method_version", "identity mismatch")
    identity = recipe["scenario_identity"]
    _require(
        isinstance(identity, str)
        and 1 <= len(identity) <= 80
        and all(character.isascii() and (character.isalnum() or character in "_.-") for character in identity),
        "scenario_identity",
        "must be a stable ASCII identifier",
    )

    design = recipe["design"]
    expected_design = {
        "predictor_construct",
        "outcome_construct",
        "predictor_indicator_loadings",
        "outcome_indicator_loadings",
        "population_path",
        "exogenous_distribution",
        "structural_disturbance_distribution",
        "indicator_error_distribution",
        "missing_data",
    }
    _require(isinstance(design, dict) and set(design) == expected_design, "design", "unknown or missing fields")
    for field in ("predictor_construct", "outcome_construct"):
        value = design[field]
        _require(
            isinstance(value, str)
            and 1 <= len(value) <= 80
            and all(character.isascii() and (character.isalnum() or character in "_.-") for character in value),
            f"design.{field}",
            "must be a stable ASCII identifier",
        )
    _require(
        design["predictor_construct"] != design["outcome_construct"],
        "design.outcome_construct",
        "must differ from predictor_construct",
    )
    for field in ("predictor_indicator_loadings", "outcome_indicator_loadings"):
        values = design[field]
        _require(isinstance(values, list) and 3 <= len(values) <= 10, f"design.{field}", "requires 3-10 loadings")
        _require(
            all(_finite_number(value) and 0.50 <= float(value) <= 0.95 for value in values),
            f"design.{field}",
            "loadings must be finite and between 0.50 and 0.95",
        )
    _require(
        _finite_number(design["population_path"]) and abs(float(design["population_path"])) <= 0.80,
        "design.population_path",
        "must be finite and between -0.80 and 0.80",
    )
    for field in (
        "exogenous_distribution",
        "structural_disturbance_distribution",
        "indicator_error_distribution",
    ):
        _require(design[field] == "standard_normal", f"design.{field}", "v1 supports standard_normal only")
    _require(design["missing_data"] == "none", "design.missing_data", "v1 supports none only")

    estimator = recipe["estimator"]
    _require(
        isinstance(estimator, dict)
        and set(estimator) == {"weighting_scheme", "preprocessing", "tolerance", "max_iterations"},
        "estimator",
        "unknown or missing fields",
    )
    _require(estimator["weighting_scheme"] == "path", "estimator.weighting_scheme", "v1 supports path only")
    _require(estimator["preprocessing"] == "standardized", "estimator.preprocessing", "v1 supports standardized only")
    _require(
        _finite_number(estimator["tolerance"]) and 1e-10 <= float(estimator["tolerance"]) <= 1e-3,
        "estimator.tolerance",
        "must be between 1e-10 and 1e-3",
    )
    _require(
        isinstance(estimator["max_iterations"], int) and 100 <= estimator["max_iterations"] <= 10_000,
        "estimator.max_iterations",
        "must be between 100 and 10000",
    )
    _require(
        recipe["inference"] == "case_bootstrap_normal_reference_two_sided",
        "inference",
        "v1 supports the frozen case-bootstrap test only",
    )
    grid = recipe["sample_size_grid"]
    _require(isinstance(grid, list) and 2 <= len(grid) <= 16, "sample_size_grid", "requires 2-16 values")
    _require(
        all(isinstance(value, int) and not isinstance(value, bool) and 30 <= value <= 5_000 for value in grid),
        "sample_size_grid",
        "values must be integers between 30 and 5000",
    )
    _require(all(right > left for left, right in zip(grid, grid[1:])), "sample_size_grid", "must be strictly increasing")
    _require(_finite_number(recipe["alpha"]) and 0.001 <= float(recipe["alpha"]) <= 0.10, "alpha", "out of range")
    _require(
        _finite_number(recipe["target_power"]) and 0.50 <= float(recipe["target_power"]) <= 0.99,
        "target_power",
        "out of range",
    )
    _require(
        _finite_number(recipe["confidence_level"]) and 0.80 <= float(recipe["confidence_level"]) <= 0.999,
        "confidence_level",
        "out of range",
    )
    minimum_monte_carlo = 100 if enforce_production_counts else 10
    minimum_bootstrap = 99 if enforce_production_counts else 9
    _require(
        isinstance(recipe["monte_carlo_replicates"], int)
        and minimum_monte_carlo <= recipe["monte_carlo_replicates"] <= 10_000,
        "monte_carlo_replicates",
        f"must be between {minimum_monte_carlo} and 10000",
    )
    _require(
        isinstance(recipe["bootstrap_replicates"], int)
        and minimum_bootstrap <= recipe["bootstrap_replicates"] <= 1_999
        and recipe["bootstrap_replicates"] % 2 == 1,
        "bootstrap_replicates",
        f"must be odd and between {minimum_bootstrap} and 1999",
    )
    _require(isinstance(recipe["master_seed"], int) and 0 <= recipe["master_seed"] < 2**64, "master_seed", "must be u64")
    _require(isinstance(recipe["workers"], int) and 1 <= recipe["workers"] <= 64, "workers", "must be 1-64")
    best_lower, _ = wilson_interval(
        recipe["monte_carlo_replicates"],
        recipe["monte_carlo_replicates"],
        recipe["confidence_level"],
    )
    _require(
        best_lower + np.finfo(float).eps >= recipe["target_power"],
        "monte_carlo_replicates",
        "Wilson lower bound cannot reach target_power even with all successes",
    )
    fits_per_dataset = 1 + int(recipe["bootstrap_replicates"])
    planned_datasets = len(grid) * int(recipe["monte_carlo_replicates"])
    estimated_pls_fits = planned_datasets * fits_per_dataset
    estimated_pls_case_fits = (
        sum(grid) * int(recipe["monte_carlo_replicates"]) * fits_per_dataset
    )
    _require(
        estimated_pls_fits <= MAX_ESTIMATED_PLS_FITS,
        "sample_size_grid",
        "estimated PLS workload exceeds the 250000-fit desktop execution limit",
    )
    _require(
        estimated_pls_case_fits <= MAX_ESTIMATED_PLS_CASE_FITS,
        "sample_size_grid",
        "estimated case-fit workload exceeds the 100000000-row desktop execution limit",
    )


def wilson_interval(successes: int, trials: int, confidence_level: float) -> tuple[float, float]:
    if trials <= 0 or successes < 0 or successes > trials or not (0.0 < confidence_level < 1.0):
        raise ValueError("invalid Wilson interval inputs")
    proportion = successes / trials
    z = NormalDist().inv_cdf(1.0 - (1.0 - confidence_level) / 2.0)
    z_squared = z * z
    denominator = 1.0 + z_squared / trials
    center = (proportion + z_squared / (2.0 * trials)) / denominator
    half_width = z * math.sqrt(
        proportion * (1.0 - proportion) / trials + z_squared / (4.0 * trials * trials)
    ) / denominator
    return max(0.0, center - half_width), min(1.0, center + half_width)


def _stream_digest(recipe: dict[str, Any], sample_size: int, replicate_index: int, subdomain: str) -> bytes:
    digest = hashlib.sha256()
    for value in (
        STREAM_DOMAIN,
        recipe["method_version"],
        recipe["scenario_identity"],
    ):
        digest.update(value.encode("utf-8"))
        digest.update(b"\0")
    digest.update(int(recipe["master_seed"]).to_bytes(8, "little"))
    digest.update(int(sample_size).to_bytes(4, "little"))
    digest.update(int(replicate_index).to_bytes(4, "little"))
    digest.update(subdomain.encode("ascii"))
    return digest.digest()


def stream_identity(recipe: dict[str, Any], sample_size: int, replicate_index: int) -> str:
    return _stream_digest(recipe, sample_size, replicate_index, "identity").hex()


def generate_dataset(recipe: dict[str, Any], sample_size: int, replicate_index: int) -> tuple[np.ndarray, np.ndarray]:
    seed_bytes = _stream_digest(recipe, sample_size, replicate_index, "generated_data")
    seed = int.from_bytes(seed_bytes[:16], "little")
    rng = np.random.default_rng(seed)
    design = recipe["design"]
    beta = float(design["population_path"])
    predictor = rng.standard_normal(sample_size)
    outcome = beta * predictor + math.sqrt(1.0 - beta * beta) * rng.standard_normal(sample_size)
    predictor_loadings = np.asarray(design["predictor_indicator_loadings"], dtype=float)
    outcome_loadings = np.asarray(design["outcome_indicator_loadings"], dtype=float)
    predictor_block = (
        predictor[:, None] * predictor_loadings[None, :]
        + rng.standard_normal((sample_size, predictor_loadings.size))
        * np.sqrt(1.0 - predictor_loadings * predictor_loadings)[None, :]
    )
    outcome_block = (
        outcome[:, None] * outcome_loadings[None, :]
        + rng.standard_normal((sample_size, outcome_loadings.size))
        * np.sqrt(1.0 - outcome_loadings * outcome_loadings)[None, :]
    )
    return predictor_block, outcome_block


def _standardize(matrix: np.ndarray) -> np.ndarray:
    means = np.mean(matrix, axis=0)
    standard_deviations = np.std(matrix, axis=0, ddof=1)
    if np.any(~np.isfinite(standard_deviations)) or np.any(standard_deviations <= np.finfo(float).eps):
        raise FloatingPointError("indicator has zero or non-finite variance")
    return (matrix - means) / standard_deviations


def _standardize_score(score: np.ndarray) -> np.ndarray:
    centered = score - np.mean(score)
    scale = np.std(centered, ddof=1)
    if not math.isfinite(float(scale)) or scale <= np.finfo(float).eps:
        raise FloatingPointError("construct score has zero or non-finite variance")
    return centered / scale


def estimate_pls_path(
    predictor_block: np.ndarray,
    outcome_block: np.ndarray,
    *,
    tolerance: float,
    max_iterations: int,
) -> tuple[float, bool]:
    """Independent two-block Mode-A path-weighting PLS estimate."""

    predictor = _standardize(np.asarray(predictor_block, dtype=float))
    outcome = _standardize(np.asarray(outcome_block, dtype=float))
    predictor_weights = np.ones(predictor.shape[1], dtype=float)
    outcome_weights = np.ones(outcome.shape[1], dtype=float)
    predictor_weights /= np.linalg.norm(predictor_weights)
    outcome_weights /= np.linalg.norm(outcome_weights)
    converged = False
    for _ in range(max_iterations):
        predictor_score = _standardize_score(predictor @ predictor_weights)
        outcome_score = _standardize_score(outcome @ outcome_weights)
        direction = 1.0 if float(np.dot(predictor_score, outcome_score)) >= 0.0 else -1.0
        predictor_inner = direction * outcome_score
        outcome_inner = direction * predictor_score
        next_predictor = predictor.T @ predictor_inner
        next_outcome = outcome.T @ outcome_inner
        predictor_norm = float(np.linalg.norm(next_predictor))
        outcome_norm = float(np.linalg.norm(next_outcome))
        if predictor_norm <= np.finfo(float).eps or outcome_norm <= np.finfo(float).eps:
            raise FloatingPointError("Mode-A update is numerically singular")
        next_predictor /= predictor_norm
        next_outcome /= outcome_norm
        if next_predictor[0] < 0.0:
            next_predictor *= -1.0
        if next_outcome[0] < 0.0:
            next_outcome *= -1.0
        change = max(
            float(np.max(np.abs(next_predictor - predictor_weights))),
            float(np.max(np.abs(next_outcome - outcome_weights))),
        )
        predictor_weights, outcome_weights = next_predictor, next_outcome
        if change <= tolerance:
            converged = True
            break
    predictor_score = _standardize_score(predictor @ predictor_weights)
    outcome_score = _standardize_score(outcome @ outcome_weights)
    denominator = float(np.dot(predictor_score, predictor_score))
    if denominator <= np.finfo(float).eps:
        raise FloatingPointError("target structural regression is singular")
    coefficient = float(np.dot(predictor_score, outcome_score) / denominator)
    if not math.isfinite(coefficient):
        raise FloatingPointError("target path coefficient is non-finite")
    return coefficient, converged


def bootstrap_target_test(
    recipe: dict[str, Any],
    predictor_block: np.ndarray,
    outcome_block: np.ndarray,
    sample_size: int,
    replicate_index: int,
) -> tuple[float, float, int]:
    estimator = recipe["estimator"]
    original, converged = estimate_pls_path(
        predictor_block,
        outcome_block,
        tolerance=float(estimator["tolerance"]),
        max_iterations=int(estimator["max_iterations"]),
    )
    if not converged:
        raise FloatingPointError("PLS point estimate did not converge")
    seed = int.from_bytes(
        _stream_digest(recipe, sample_size, replicate_index, "bootstrap_inference")[:16],
        "little",
    )
    rng = np.random.default_rng(seed)
    estimates: list[float] = []
    for _ in range(int(recipe["bootstrap_replicates"])):
        indices = rng.integers(0, sample_size, size=sample_size)
        try:
            estimate, converged = estimate_pls_path(
                predictor_block[indices],
                outcome_block[indices],
                tolerance=float(estimator["tolerance"]),
                max_iterations=int(estimator["max_iterations"]),
            )
        except (FloatingPointError, np.linalg.LinAlgError):
            continue
        if converged and math.isfinite(estimate):
            estimates.append(estimate)
    required = max(2, math.ceil(int(recipe["bootstrap_replicates"]) * 0.90))
    if len(estimates) < required:
        raise FloatingPointError(f"only {len(estimates)} usable bootstrap replicates; {required} required")
    standard_error = float(np.std(np.asarray(estimates), ddof=1))
    if not math.isfinite(standard_error) or standard_error <= np.finfo(float).eps:
        raise FloatingPointError("bootstrap standard error is unavailable")
    statistic = original / standard_error
    p_value = math.erfc(abs(statistic) / math.sqrt(2.0))
    return original, min(1.0, max(0.0, p_value)), len(estimates)


def execute_replicate(recipe: dict[str, Any], sample_size: int, replicate_index: int) -> ReferenceOutcome:
    identity = stream_identity(recipe, sample_size, replicate_index)
    try:
        predictor_block, outcome_block = generate_dataset(recipe, sample_size, replicate_index)
        estimate, p_value, _ = bootstrap_target_test(
            recipe,
            predictor_block,
            outcome_block,
            sample_size,
            replicate_index,
        )
    except (FloatingPointError, np.linalg.LinAlgError, ValueError) as error:
        return ReferenceOutcome(
            sample_size=sample_size,
            replicate_index=replicate_index,
            stream_identity=identity,
            successful=False,
            converged=False,
            target_estimate=None,
            p_value_two_sided=None,
            rejected=False,
            failure_code="reference_inference_failed",
            failure_message=str(error),
        )
    return ReferenceOutcome(
        sample_size=sample_size,
        replicate_index=replicate_index,
        stream_identity=identity,
        successful=True,
        converged=True,
        target_estimate=estimate,
        p_value_two_sided=p_value,
        rejected=p_value <= float(recipe["alpha"]),
        failure_code=None,
        failure_message=None,
    )


def run_reference(recipe: dict[str, Any], *, enforce_production_counts: bool = True) -> dict[str, Any]:
    validate_recipe(recipe, enforce_production_counts=enforce_production_counts)
    outcomes = [
        execute_replicate(recipe, sample_size, replicate_index)
        for sample_size in recipe["sample_size_grid"]
        for replicate_index in range(recipe["monte_carlo_replicates"])
    ]
    rows = []
    for sample_size in recipe["sample_size_grid"]:
        selected = [outcome for outcome in outcomes if outcome.sample_size == sample_size]
        successful = sum(outcome.successful for outcome in selected)
        rejections = sum(outcome.rejected for outcome in selected)
        requested = int(recipe["monte_carlo_replicates"])
        lower, upper = wilson_interval(rejections, requested, float(recipe["confidence_level"]))
        rows.append(
            {
                "sample_size": sample_size,
                "requested_replicates": requested,
                "attempted_replicates": len(selected),
                "successful_replicates": successful,
                "failed_replicates": requested - successful,
                "rejections": rejections,
                "achieved_power": rejections / requested,
                "confidence_lower": lower,
                "confidence_upper": upper,
                "qualifies": lower >= float(recipe["target_power"]),
            }
        )
    qualifying = next((row["sample_size"] for row in rows if row["qualifies"]), None)
    monotonicity_violations = sum(
        right["achieved_power"] + 1e-12 < left["achieved_power"]
        for left, right in zip(rows, rows[1:])
    )
    encoded_outcomes = [outcome.as_dict() for outcome in outcomes]
    outcome_digest = hashlib.sha256(
        json.dumps(encoded_outcomes, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
    ).hexdigest()
    return {
        "report_kind": "pls_sample_size_power_independent_python_reference_v1",
        "passed": all(row["attempted_replicates"] == row["requested_replicates"] for row in rows),
        "feature_id": CAPABILITY_ID,
        "method_version": METHOD_VERSION,
        "stream_domain": STREAM_DOMAIN,
        "failure_policy": FAILURE_POLICY,
        "interval_method": INTERVAL_METHOD,
        "inference_method": INFERENCE_METHOD,
        "rows": rows,
        "decision": (
            {"status": "reached", "sample_size": qualifying}
            if qualifying is not None
            else {"status": "not_reached"}
        ),
        "monotonicity_violations": monotonicity_violations,
        "outcomes": encoded_outcomes,
        "outcome_digest": outcome_digest,
    }


def fixture_recipe(*, effect: float = 0.30, replicates: int = 100, bootstrap: int = 99) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "capability_id": CAPABILITY_ID,
        "method_version": METHOD_VERSION,
        "scenario_identity": f"python_reference_effect_{str(effect).replace('.', '_')}",
        "design": {
            "predictor_construct": "x",
            "outcome_construct": "y",
            "predictor_indicator_loadings": [0.80, 0.80, 0.80],
            "outcome_indicator_loadings": [0.80, 0.80, 0.80],
            "population_path": effect,
            "exogenous_distribution": "standard_normal",
            "structural_disturbance_distribution": "standard_normal",
            "indicator_error_distribution": "standard_normal",
            "missing_data": "none",
        },
        "estimator": {
            "weighting_scheme": "path",
            "preprocessing": "standardized",
            "tolerance": 1e-7,
            "max_iterations": 3_000,
        },
        "inference": "case_bootstrap_normal_reference_two_sided",
        "sample_size_grid": [60, 120],
        "alpha": 0.05,
        "target_power": 0.80,
        "confidence_level": 0.95,
        "monte_carlo_replicates": replicates,
        "bootstrap_replicates": bootstrap,
        "master_seed": 20_260_813,
        "workers": 1,
    }


def self_test() -> dict[str, Any]:
    lower, upper = wilson_interval(80, 100, 0.95)
    assert abs(lower - 0.7111708343) < 1e-9
    assert abs(upper - 0.8666330667) < 1e-9
    recipe = fixture_recipe(replicates=20, bootstrap=9)
    validate_recipe(recipe, enforce_production_counts=False)
    first = generate_dataset(recipe, 60, 0)
    repeat = generate_dataset(recipe, 60, 0)
    different = generate_dataset(recipe, 60, 1)
    assert np.array_equal(first[0], repeat[0]) and np.array_equal(first[1], repeat[1])
    assert not np.array_equal(first[0], different[0])
    predictor, outcome = generate_dataset(recipe, 120, 2)
    estimate, converged = estimate_pls_path(predictor, outcome, tolerance=1e-7, max_iterations=3_000)
    assert converged and math.isfinite(estimate)
    report = run_reference(recipe, enforce_production_counts=False)
    assert report["passed"] and len(report["outcomes"]) == 40
    invalid = fixture_recipe()
    invalid["design"]["missing_data"] = "mar"
    try:
        validate_recipe(invalid)
    except ReferenceInputError:
        pass
    else:
        raise AssertionError("unsupported missing-data design was accepted")
    return {
        "report_kind": "pls_sample_size_power_independent_python_self_test_v1",
        "passed": True,
        "feature_id": CAPABILITY_ID,
        "method_version": METHOD_VERSION,
        "checks": 8,
        "quick_reference_rows": report["rows"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--recipe", type=Path)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--self-test", action="store_true")
    parser.add_argument(
        "--allow-small-test-counts",
        action="store_true",
        help="Validation-only shortcut; never qualifies a production power run.",
    )
    args = parser.parse_args()
    if args.self_test:
        report = self_test()
    else:
        if args.recipe is None:
            parser.error("--recipe is required unless --self-test is used")
        recipe = json.loads(args.recipe.read_text(encoding="utf-8"))
        report = run_reference(recipe, enforce_production_counts=not args.allow_small_test_counts)
    encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
