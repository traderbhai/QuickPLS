#!/usr/bin/env python3
"""Transparent validation-only oracle for compiled CB-SEM matrix input v2.

This module is intentionally independent from the QuickPLS implementation. It
does not import product code, execute Rust, or use a product result as an
oracle.  It implements a marker-identified one-factor CFA directly from the ML
covariance discrepancy, its analytic gradient, and a closed-form three-
indicator hand solution.  NumPy and SciPy are validation-only dependencies.

The generated report is work evidence, not a qualification receipt.  In
particular, a passing report cannot promote a capability cell and explicitly
records the missing product comparison and second maintained SEM reference.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Iterable

import numpy as np
import scipy
from scipy.optimize import minimize


ROOT = Path(__file__).resolve().parents[1]
SOURCE_PATH = Path("validation/cbsem_matrix_input_v2_oracle.py")
REPORT_PATH = (
    ROOT
    / "validation"
    / "results"
    / "method_factory"
    / "cbsem_matrix_input_v2"
    / "work"
    / "independent_oracle.json"
)
ORACLE_VERSION = "cbsem_matrix_input_numpy_scipy_oracle_v1"
SCENARIO_VERSION = "cbsem_matrix_input_small_scenarios_v1"
OPTIMIZER_GRADIENT_TOLERANCE = 1e-7
GRADIENT_AUDIT_TOLERANCE = 2e-6
MOMENT_EQUIVALENCE_TOLERANCE = 2e-12
PARAMETER_EQUIVALENCE_TOLERANCE = 2e-7
OBJECTIVE_EQUIVALENCE_TOLERANCE = 2e-10
MINIMUM_SAMPLE_SIZE = 10


class OracleInputError(ValueError):
    """Typed failure matching a frozen matrix-input contract boundary."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class Population:
    identifier: str
    loadings: tuple[float, float, float]
    latent_variance: float
    residual_variances: tuple[float, float, float]

    def ensure_valid(self) -> None:
        if self.loadings[0] != 1.0:
            raise ValueError("the first loading is the fixed marker and must equal one")
        if self.latent_variance <= 0.0 or any(
            value <= 0.0 for value in self.residual_variances
        ):
            raise ValueError("population variances must be positive")


@dataclass(frozen=True)
class Estimate:
    parameters: dict[str, float]
    objective: float
    gradient_infinity_norm: float
    iterations: int
    converged: bool
    optimizer_success: bool
    optimizer_message: str
    implied_covariance: np.ndarray


POPULATIONS = (
    Population("balanced", (1.0, 0.78, 0.64), 1.35, (0.42, 0.58, 0.73)),
    Population("mixed_scale", (1.0, 0.70, 1.15), 2.10, (0.50, 0.90, 2.00)),
)

GENERATIVE_SCENARIOS = tuple(
    {
        "id": f"{population.identifier}_n{sample_size}_seed{seed}",
        "population": population.identifier,
        "sample_size": sample_size,
        "seed": seed,
    }
    for population in POPULATIONS
    for sample_size, seed in ((96, 2026081501), (192, 2026081502), (384, 2026081503))
)

ADVERSARIAL_SCENARIOS = (
    {
        "id": "non_positive_definite",
        "expected_code": "MatrixNotPositiveDefinite",
    },
    {"id": "wrong_shape", "expected_code": "MatrixShape"},
    {"id": "sample_size_mismatch", "expected_code": "SampleSizeMismatch"},
    {
        "id": "sample_denominator_invalid",
        "expected_code": "SampleDenominatorInvalid",
    },
    {
        "id": "correlation_scale_missing",
        "expected_code": "CorrelationScaleMetadataRequired",
    },
)


def _json_sha256(value: Any) -> str:
    payload = json.dumps(
        value, sort_keys=True, separators=(",", ":"), ensure_ascii=False, allow_nan=False
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def scenario_contract() -> dict[str, Any]:
    return {
        "version": SCENARIO_VERSION,
        "generative": list(GENERATIVE_SCENARIOS),
        "adversarial": list(ADVERSARIAL_SCENARIOS),
        "fixed_parameterization": {
            "construct": "construct:f",
            "marker": "x1",
            "observed_order": ["x1", "x2", "x3"],
            "minimum_sample_size": MINIMUM_SAMPLE_SIZE,
            "raw_covariance_denominator": "maximum_likelihood_n",
        },
    }


def stable_parameter_ids() -> dict[str, str]:
    """Exact IDs from the independent hand fixture's SemModelV4 contract."""

    return {
        "construct:f=~x1": "parameter_66_7831",
        "construct:f=~x2": "parameter_66_7832",
        "construct:f=~x3": "parameter_66_7833",
        "construct:f~~construct:f": "variance_66",
        "x1~~x1": "residual_variance_7831",
        "x2~~x2": "residual_variance_7832",
        "x3~~x3": "residual_variance_7833",
    }


def implied_covariance(
    loadings: Iterable[float],
    latent_variance: float,
    residual_variances: Iterable[float],
) -> np.ndarray:
    loading = np.asarray(tuple(loadings), dtype=np.float64)
    residual = np.asarray(tuple(residual_variances), dtype=np.float64)
    if loading.shape != (3,) or residual.shape != (3,):
        raise ValueError("the frozen microcase requires exactly three indicators")
    return latent_variance * np.outer(loading, loading) + np.diag(residual)


def population_covariance(population: Population) -> np.ndarray:
    population.ensure_valid()
    return implied_covariance(
        population.loadings,
        population.latent_variance,
        population.residual_variances,
    )


def generate_raw(population: Population, sample_size: int, seed: int) -> np.ndarray:
    population.ensure_valid()
    if sample_size < MINIMUM_SAMPLE_SIZE:
        raise OracleInputError(
            "InsufficientObservations",
            f"at least {MINIMUM_SAMPLE_SIZE} observations are required",
        )
    rng = np.random.default_rng(seed)
    factor = rng.normal(size=(sample_size, 1)) * math.sqrt(
        population.latent_variance
    )
    errors = rng.normal(size=(sample_size, 3)) * np.sqrt(
        np.asarray(population.residual_variances)
    )
    return factor * np.asarray(population.loadings) + errors


def _require_square(matrix: np.ndarray, expected_dimension: int) -> None:
    if matrix.ndim != 2 or matrix.shape != (expected_dimension, expected_dimension):
        rows = matrix.shape[0] if matrix.ndim >= 1 else 0
        columns = matrix.shape[1] if matrix.ndim >= 2 else 0
        raise OracleInputError(
            "MatrixShape",
            f"expected {expected_dimension} x {expected_dimension}, found {rows} x {columns}",
        )


def _require_positive_definite(matrix: np.ndarray) -> None:
    try:
        np.linalg.cholesky(matrix)
    except np.linalg.LinAlgError as error:
        raise OracleInputError(
            "MatrixNotPositiveDefinite", "matrix must be strictly positive definite"
        ) from error


def canonical_raw_covariance(raw: np.ndarray) -> np.ndarray:
    values = np.asarray(raw, dtype=np.float64)
    if values.ndim != 2 or values.shape[1] != 3:
        raise OracleInputError("RawShape", "raw input requires three indicator columns")
    if values.shape[0] < MINIMUM_SAMPLE_SIZE:
        raise OracleInputError(
            "InsufficientObservations",
            f"at least {MINIMUM_SAMPLE_SIZE} observations are required",
        )
    if not np.isfinite(values).all():
        raise OracleInputError("RawValueNonFinite", "raw input contains a nonfinite value")
    centered = values - values.mean(axis=0)
    covariance = centered.T @ centered / values.shape[0]
    _require_positive_definite(covariance)
    return covariance


def canonical_matrix_covariance(
    matrix: np.ndarray,
    *,
    kind: str,
    declared_sample_size: int,
    dataset_sample_size: int,
    denominator: str,
    expected_dimension: int = 3,
    standard_deviations: np.ndarray | None = None,
) -> np.ndarray:
    if declared_sample_size != dataset_sample_size:
        raise OracleInputError(
            "SampleSizeMismatch",
            "declared sample size differs from matrix dataset metadata",
        )
    if declared_sample_size < MINIMUM_SAMPLE_SIZE:
        raise OracleInputError(
            "InsufficientObservations",
            f"at least {MINIMUM_SAMPLE_SIZE} observations are required",
        )
    if denominator not in {"sample_n_minus_one", "maximum_likelihood_n"}:
        raise OracleInputError(
            "SampleDenominatorInvalid", "matrix denominator is not a supported enum value"
        )
    values = np.asarray(matrix, dtype=np.float64)
    _require_square(values, expected_dimension)
    if not np.isfinite(values).all():
        raise OracleInputError("MatrixCellNonFinite", "matrix contains a nonfinite value")
    if not np.allclose(values, values.T, atol=1e-10, rtol=0.0):
        raise OracleInputError("MatrixNotSymmetric", "matrix must be symmetric")
    covariance = values.copy()
    if kind == "correlation":
        if not np.allclose(np.diag(values), 1.0, atol=1e-10, rtol=0.0):
            raise OracleInputError(
                "CorrelationDiagonalInvalid", "correlation diagonal must equal one"
            )
        if np.max(np.abs(values)) > 1.0 + 1e-12:
            raise OracleInputError(
                "CorrelationOutOfRange", "correlation values must be in [-1, 1]"
            )
        if standard_deviations is None:
            raise OracleInputError(
                "CorrelationScaleMetadataRequired",
                "correlation input requires one scale for every variable",
            )
        scales = np.asarray(standard_deviations, dtype=np.float64)
        if scales.shape != (expected_dimension,) or not np.isfinite(scales).all() or np.any(
            scales <= 0.0
        ):
            raise OracleInputError(
                "CorrelationScaleMetadataRequired",
                "correlation scales must be finite, positive, and complete",
            )
        covariance = values * np.outer(scales, scales)
    elif kind != "covariance":
        raise OracleInputError("InputKindInvalid", f"unsupported matrix kind {kind!r}")
    if denominator == "sample_n_minus_one":
        covariance *= (declared_sample_size - 1) / declared_sample_size
    _require_positive_definite(covariance)
    return covariance


def _decode(raw_parameters: np.ndarray) -> tuple[np.ndarray, float, np.ndarray]:
    loadings = np.asarray((1.0, raw_parameters[0], raw_parameters[1]))
    latent_variance = math.exp(float(raw_parameters[2]))
    residual_variances = np.exp(np.asarray(raw_parameters[3:6]))
    return loadings, latent_variance, residual_variances


def ml_discrepancy(sample_covariance: np.ndarray, raw_parameters: np.ndarray) -> float:
    loadings, latent_variance, residual = _decode(raw_parameters)
    sigma = implied_covariance(loadings, latent_variance, residual)
    sample_sign, sample_logdet = np.linalg.slogdet(sample_covariance)
    sigma_sign, sigma_logdet = np.linalg.slogdet(sigma)
    if sample_sign <= 0.0 or sigma_sign <= 0.0:
        return math.inf
    return float(
        sigma_logdet
        + np.trace(sample_covariance @ np.linalg.inv(sigma))
        - sample_logdet
        - sample_covariance.shape[0]
    )


def ml_gradient(sample_covariance: np.ndarray, raw_parameters: np.ndarray) -> np.ndarray:
    loadings, latent_variance, residual = _decode(raw_parameters)
    sigma = implied_covariance(loadings, latent_variance, residual)
    inverse = np.linalg.inv(sigma)
    discrepancy_gradient = inverse - inverse @ sample_covariance @ inverse
    loading_gradient = 2.0 * latent_variance * (discrepancy_gradient @ loadings)
    return np.asarray(
        (
            loading_gradient[1],
            loading_gradient[2],
            latent_variance * float(loadings @ discrepancy_gradient @ loadings),
            residual[0] * discrepancy_gradient[0, 0],
            residual[1] * discrepancy_gradient[1, 1],
            residual[2] * discrepancy_gradient[2, 2],
        ),
        dtype=np.float64,
    )


def central_difference_gradient(
    function: Callable[[np.ndarray], float], parameters: np.ndarray
) -> np.ndarray:
    gradient = np.empty_like(parameters)
    for index, value in enumerate(parameters):
        step = 1e-6 * max(abs(float(value)), 1.0)
        plus = parameters.copy()
        minus = parameters.copy()
        plus[index] += step
        minus[index] -= step
        gradient[index] = (function(plus) - function(minus)) / (2.0 * step)
    return gradient


def closed_form_three_indicator(sample_covariance: np.ndarray) -> dict[str, float]:
    sample = np.asarray(sample_covariance, dtype=np.float64)
    _require_square(sample, 3)
    _require_positive_definite(sample)
    s12, s13, s23 = sample[0, 1], sample[0, 2], sample[1, 2]
    if abs(s12) < 1e-14 or abs(s13) < 1e-14 or abs(s23) < 1e-14:
        raise OracleInputError(
            "ClosedFormIneligible", "three nonzero cross-covariances are required"
        )
    latent_variance = s12 * s13 / s23
    loadings = np.asarray((1.0, s23 / s13, s23 / s12))
    residual = np.diag(sample) - latent_variance * np.square(loadings)
    if latent_variance <= 0.0 or np.any(residual <= 0.0):
        raise OracleInputError(
            "ClosedFormInadmissible", "closed-form solution has a nonpositive variance"
        )
    values = {
        "construct:f=~x1": 1.0,
        "construct:f=~x2": float(loadings[1]),
        "construct:f=~x3": float(loadings[2]),
        "construct:f~~construct:f": float(latent_variance),
        "x1~~x1": float(residual[0]),
        "x2~~x2": float(residual[1]),
        "x3~~x3": float(residual[2]),
    }
    return values


def _raw_from_parameters(parameters: dict[str, float]) -> np.ndarray:
    return np.asarray(
        (
            parameters["construct:f=~x2"],
            parameters["construct:f=~x3"],
            math.log(parameters["construct:f~~construct:f"]),
            math.log(parameters["x1~~x1"]),
            math.log(parameters["x2~~x2"]),
            math.log(parameters["x3~~x3"]),
        )
    )


def estimate_one_factor(sample_covariance: np.ndarray) -> Estimate:
    sample = np.asarray(sample_covariance, dtype=np.float64)
    _require_square(sample, 3)
    _require_positive_definite(sample)
    hand = closed_form_three_indicator(sample)
    start = _raw_from_parameters(hand)
    start += np.asarray((0.04, -0.03, 0.08, -0.06, 0.05, -0.04))
    result = minimize(
        lambda values: ml_discrepancy(sample, values),
        start,
        method="BFGS",
        jac=lambda values: ml_gradient(sample, values),
        options={"gtol": OPTIMIZER_GRADIENT_TOLERANCE, "maxiter": 2_000},
    )
    loadings, latent_variance, residual = _decode(result.x)
    gradient_norm = float(np.linalg.norm(ml_gradient(sample, result.x), ord=np.inf))
    objective = float(ml_discrepancy(sample, result.x))
    converged = bool(
        np.isfinite(objective)
        and gradient_norm <= OPTIMIZER_GRADIENT_TOLERANCE
        and objective >= -1e-10
    )
    return Estimate(
        parameters={
            "construct:f=~x1": 1.0,
            "construct:f=~x2": float(loadings[1]),
            "construct:f=~x3": float(loadings[2]),
            "construct:f~~construct:f": float(latent_variance),
            "x1~~x1": float(residual[0]),
            "x2~~x2": float(residual[1]),
            "x3~~x3": float(residual[2]),
        },
        objective=objective,
        gradient_infinity_norm=gradient_norm,
        iterations=int(result.nit),
        converged=converged,
        optimizer_success=bool(result.success),
        optimizer_message=str(result.message),
        implied_covariance=implied_covariance(loadings, latent_variance, residual),
    )


def _max_parameter_delta(left: Estimate, right: Estimate) -> float:
    return max(
        abs(left.parameters[name] - right.parameters[name])
        for name in stable_parameter_ids()
    )


def _population_parameters(population: Population) -> dict[str, float]:
    return {
        "construct:f=~x1": 1.0,
        "construct:f=~x2": population.loadings[1],
        "construct:f=~x3": population.loadings[2],
        "construct:f~~construct:f": population.latent_variance,
        "x1~~x1": population.residual_variances[0],
        "x2~~x2": population.residual_variances[1],
        "x3~~x3": population.residual_variances[2],
    }


def _probe_lavaan() -> dict[str, Any]:
    executable = shutil.which("Rscript")
    if executable is None:
        return {
            "available": False,
            "executed": False,
            "reason": "Rscript_not_installed",
            "installation_attempted": False,
        }
    completed = subprocess.run(
        [
            executable,
            "--vanilla",
            "-e",
            "if (!requireNamespace('lavaan', quietly=TRUE)) quit(status=3); cat(as.character(packageVersion('lavaan')))",
        ],
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=30,
    )
    return {
        "available": completed.returncode == 0,
        "executed": True,
        "reason": None if completed.returncode == 0 else "lavaan_not_installed",
        "version": completed.stdout.strip() if completed.returncode == 0 else None,
        "installation_attempted": False,
    }


def _typed_failure(case: str) -> str:
    population = POPULATIONS[0]
    covariance = population_covariance(population)
    try:
        if case == "non_positive_definite":
            invalid = covariance.copy()
            invalid[0, 1] = invalid[1, 0] = math.sqrt(
                invalid[0, 0] * invalid[1, 1]
            ) * 1.01
            canonical_matrix_covariance(
                invalid,
                kind="covariance",
                declared_sample_size=120,
                dataset_sample_size=120,
                denominator="maximum_likelihood_n",
            )
        elif case == "wrong_shape":
            canonical_matrix_covariance(
                covariance[:2, :],
                kind="covariance",
                declared_sample_size=120,
                dataset_sample_size=120,
                denominator="maximum_likelihood_n",
            )
        elif case == "sample_size_mismatch":
            canonical_matrix_covariance(
                covariance,
                kind="covariance",
                declared_sample_size=120,
                dataset_sample_size=119,
                denominator="maximum_likelihood_n",
            )
        elif case == "sample_denominator_invalid":
            canonical_matrix_covariance(
                covariance,
                kind="covariance",
                declared_sample_size=120,
                dataset_sample_size=120,
                denominator="unspecified",
            )
        elif case == "correlation_scale_missing":
            scales = np.sqrt(np.diag(covariance))
            correlation = covariance / np.outer(scales, scales)
            canonical_matrix_covariance(
                correlation,
                kind="correlation",
                declared_sample_size=120,
                dataset_sample_size=120,
                denominator="maximum_likelihood_n",
            )
        else:
            raise AssertionError(f"unknown adversarial case {case}")
    except OracleInputError as error:
        return error.code
    raise AssertionError(f"adversarial case {case} did not fail")


def build_report() -> dict[str, Any]:
    exact_rows = []
    representation_rows = []
    gradient_rows = []
    generative_rows = []

    for population in POPULATIONS:
        covariance = population_covariance(population)
        estimate = estimate_one_factor(covariance)
        expected = _population_parameters(population)
        hand = closed_form_three_indicator(covariance)
        exact_delta = max(
            abs(estimate.parameters[name] - expected[name]) for name in expected
        )
        hand_delta = max(abs(hand[name] - expected[name]) for name in expected)
        start = _raw_from_parameters(hand) + np.asarray(
            (0.07, -0.04, 0.05, -0.03, 0.06, -0.02)
        )
        analytic = ml_gradient(covariance, start)
        numeric = central_difference_gradient(
            lambda values: ml_discrepancy(covariance, values), start
        )
        gradient_delta = float(np.max(np.abs(analytic - numeric)))
        exact_rows.append(
            {
                "population": population.identifier,
                "maximum_optimizer_parameter_delta": exact_delta,
                "maximum_hand_parameter_delta": hand_delta,
                "objective": estimate.objective,
                "gradient_infinity_norm": estimate.gradient_infinity_norm,
                "iterations": estimate.iterations,
                "converged": estimate.converged,
                "optimizer_success": estimate.optimizer_success,
            }
        )
        gradient_rows.append(
            {
                "population": population.identifier,
                "maximum_analytic_numeric_delta": gradient_delta,
                "passed": gradient_delta <= GRADIENT_AUDIT_TOLERANCE,
            }
        )

    populations = {row.identifier: row for row in POPULATIONS}
    for scenario in GENERATIVE_SCENARIOS:
        population = populations[scenario["population"]]
        raw = generate_raw(population, scenario["sample_size"], scenario["seed"])
        raw_covariance = canonical_raw_covariance(raw)
        sample_covariance = np.cov(raw, rowvar=False, ddof=1)
        covariance_input = canonical_matrix_covariance(
            sample_covariance,
            kind="covariance",
            declared_sample_size=scenario["sample_size"],
            dataset_sample_size=scenario["sample_size"],
            denominator="sample_n_minus_one",
        )
        standard_deviations = np.std(raw, axis=0, ddof=1)
        correlation = np.corrcoef(raw, rowvar=False)
        correlation_input = canonical_matrix_covariance(
            correlation,
            kind="correlation",
            declared_sample_size=scenario["sample_size"],
            dataset_sample_size=scenario["sample_size"],
            denominator="sample_n_minus_one",
            standard_deviations=standard_deviations,
        )
        estimates = {
            "raw": estimate_one_factor(raw_covariance),
            "covariance": estimate_one_factor(covariance_input),
            "correlation": estimate_one_factor(correlation_input),
        }
        max_moment_delta = max(
            float(np.max(np.abs(raw_covariance - candidate)))
            for candidate in (covariance_input, correlation_input)
        )
        max_parameter_delta = max(
            _max_parameter_delta(estimates["raw"], candidate)
            for candidate in (estimates["covariance"], estimates["correlation"])
        )
        max_objective_delta = max(
            abs(estimates["raw"].objective - candidate.objective)
            for candidate in (estimates["covariance"], estimates["correlation"])
        )
        representation_rows.append(
            {
                "scenario_id": scenario["id"],
                "maximum_canonical_moment_delta": max_moment_delta,
                "maximum_parameter_delta": max_parameter_delta,
                "maximum_objective_delta": max_objective_delta,
                "all_converged": all(row.converged for row in estimates.values()),
                "passed": (
                    max_moment_delta <= MOMENT_EQUIVALENCE_TOLERANCE
                    and max_parameter_delta <= PARAMETER_EQUIVALENCE_TOLERANCE
                    and max_objective_delta <= OBJECTIVE_EQUIVALENCE_TOLERANCE
                    and all(row.converged for row in estimates.values())
                ),
            }
        )
        expected = _population_parameters(population)
        recovery_delta = max(
            abs(estimates["raw"].parameters[name] - expected[name])
            / max(abs(expected[name]), 0.25)
            for name in expected
            if name != "construct:f=~x1"
        )
        generative_rows.append(
            {
                "scenario_id": scenario["id"],
                "maximum_scaled_parameter_error": recovery_delta,
                "converged": estimates["raw"].converged,
            }
        )

    failure_rows = [
        {
            "scenario_id": row["id"],
            "expected_code": row["expected_code"],
            "observed_code": _typed_failure(row["id"]),
        }
        for row in ADVERSARIAL_SCENARIOS
    ]
    for row in failure_rows:
        row["passed"] = row["observed_code"] == row["expected_code"]

    exact_passed = all(
        row["maximum_optimizer_parameter_delta"] <= 2e-7
        and row["maximum_hand_parameter_delta"] <= 2e-12
        and row["converged"]
        and row["objective"] <= 2e-10
        and row["gradient_infinity_norm"] <= OPTIMIZER_GRADIENT_TOLERANCE
        for row in exact_rows
    )
    stable_ids = stable_parameter_ids()
    stable_id_passed = (
        len(stable_ids) == 7
        and len(set(stable_ids.values())) == 7
        and set(stable_ids)
        == {
            "construct:f=~x1",
            "construct:f=~x2",
            "construct:f=~x3",
            "construct:f~~construct:f",
            "x1~~x1",
            "x2~~x2",
            "x3~~x3",
        }
    )
    small_recovery_passed = (
        all(row["converged"] for row in generative_rows)
        and float(
            np.median([row["maximum_scaled_parameter_error"] for row in generative_rows])
        )
        <= 0.45
    )
    passed_work_checks = all(
        (
            exact_passed,
            all(row["passed"] for row in representation_rows),
            all(row["passed"] for row in gradient_rows),
            all(row["passed"] for row in failure_rows),
            stable_id_passed,
            small_recovery_passed,
        )
    )
    scenarios = scenario_contract()
    source = ROOT / SOURCE_PATH
    second_reference = _probe_lavaan()
    blockers = [
        "no_frozen_current_product_result_is_compared_by_this_validation_only_oracle",
        "no_second_maintained_external_sem_implementation_is_available_locally",
        "small_seeded_recovery_matrix_is_not_a_monte_carlo_qualification_campaign",
        "archive_export_frontend_packaged_windows_performance_and_scientific_review_roles_are_open",
    ]
    return {
        "schema_version": 1,
        "report_kind": "cbsem_matrix_input_v2_oracle_work_report",
        "oracle_version": ORACLE_VERSION,
        "passed_work_checks": passed_work_checks,
        "qualification_role_satisfied": False,
        "receipt_eligible": False,
        "promotion_requested": False,
        "reference": {
            "implementation": "transparent_numpy_scipy_marker_cfa",
            "maintainer": "QuickPLS validation-only independent oracle",
            "numpy_version": np.__version__,
            "scipy_version": scipy.__version__,
            "source": SOURCE_PATH.as_posix(),
            "source_sha256": _file_sha256(source),
            "runtime_policy": "development_validation_only",
            "product_code_imported": False,
            "product_executable_invoked": False,
        },
        "primary_method_reference": {
            "citation": "Bollen, K. A. (1989), Structural Equations with Latent Variables, Wiley",
            "doi": "10.1002/9781118619179",
            "ml_discrepancy": "log|Sigma| + trace(S Sigma^-1) - log|S| - p",
        },
        "scenario_contract": scenarios,
        "scenario_set_sha256": _json_sha256(scenarios),
        "tolerances": {
            "optimizer_gradient_infinity_norm": OPTIMIZER_GRADIENT_TOLERANCE,
            "analytic_numeric_gradient_absolute": GRADIENT_AUDIT_TOLERANCE,
            "canonical_moment_absolute": MOMENT_EQUIVALENCE_TOLERANCE,
            "representation_parameter_absolute": PARAMETER_EQUIVALENCE_TOLERANCE,
            "objective_absolute": OBJECTIVE_EQUIVALENCE_TOLERANCE,
        },
        "stable_parameter_ids": stable_ids,
        "checks": {
            "exact_population_parameter_recovery": {
                "passed": exact_passed,
                "rows": exact_rows,
            },
            "analytic_gradient_against_central_difference": {
                "passed": all(row["passed"] for row in gradient_rows),
                "rows": gradient_rows,
            },
            "raw_covariance_scaled_correlation_equivalence": {
                "passed": all(row["passed"] for row in representation_rows),
                "rows": representation_rows,
            },
            "small_seeded_parameter_recovery": {
                "passed": small_recovery_passed,
                "qualification_sized": False,
                "rows": generative_rows,
            },
            "stable_parameter_id_mapping": {
                "passed": stable_id_passed,
                "expected_cardinality": 7,
            },
            "typed_adversarial_failures": {
                "passed": all(row["passed"] for row in failure_rows),
                "rows": failure_rows,
            },
        },
        "second_external_reference": second_reference,
        "blockers": blockers,
    }


def write_report(path: Path = REPORT_PATH) -> Path:
    report = build_report()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="verify checked-in report bytes")
    parser.add_argument("--output", type=Path, default=REPORT_PATH)
    args = parser.parse_args()
    report = build_report()
    if args.check:
        if not args.output.is_file():
            print(f"missing oracle work report: {args.output}")
            return 1
        current = json.loads(args.output.read_text(encoding="utf-8"))
        if current != report:
            print(f"stale oracle work report: {args.output}")
            return 1
    else:
        write_report(args.output)
    print(
        json.dumps(
            {
                "path": args.output.resolve().relative_to(ROOT.resolve()).as_posix(),
                "passed_work_checks": report["passed_work_checks"],
                "qualification_role_satisfied": report[
                    "qualification_role_satisfied"
                ],
                "receipt_eligible": report["receipt_eligible"],
                "blockers": report["blockers"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if report["passed_work_checks"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
