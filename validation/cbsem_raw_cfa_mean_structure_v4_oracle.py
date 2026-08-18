#!/usr/bin/env python3
"""Independent oracle for the bounded raw-CFA mean-structure V4 slice.

This validation-only module imports no QuickPLS product code and never invokes
a product executable.  It freezes the supported statistical contract for a
continuous, single-group, three-indicator CFA with marker-loading
identification, an observed-intercept marker anchor, and one estimated latent
mean.  Broader SEM mean structures are intentionally rejected.

For x = nu + Lambda eta + epsilon, the joint normal-theory ML discrepancy is

    F = log|Sigma| + tr(S Sigma^-1) - log|S| - p
        + (xbar - mu)' Sigma^-1 (xbar - mu).

The expected information used here is

    I_ij = n/2 tr(Sigma^-1 dSigma_i Sigma^-1 dSigma_j)
           + n dmu_i' Sigma^-1 dmu_j.

The formula follows the normal-theory SEM likelihood and information-matrix
derivation in Bollen (1989), Structural Equations with Latent Variables.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Sequence

import numpy as np
import scipy
from scipy import linalg


ROOT = Path(__file__).resolve().parents[1]
SOURCE_PATH = Path("validation/cbsem_raw_cfa_mean_structure_v4_oracle.py")
PRODUCT_FIXTURE_PATH = ROOT / (
    "validation/fixtures/cbsem_raw_cfa_mean_structure_v4_product_fixture.json"
)
REPORT_PATH = ROOT / (
    "validation/results/cbsem_raw_cfa_mean_structure_v4_oracle_work_report.json"
)

ORACLE_VERSION = "cbsem_raw_cfa_mean_structure_v4_oracle_v1"
ESTIMATOR_ID = "cbsem_ml_exact_parameter_table_v4"
MOMENT_ADAPTER_ID = "cbsem_ml_compiled_moment_input_v4"
MOMENT_RESULT_SCHEMA_VERSION = 3
RUNNER_ADAPTER_ID = "compiled_recipe_v4_cbsem_plan_v2_execution_v3"
VARIABLES = ("x1", "x2", "x3")
MARKER = "x1"
SAMPLE_SIZE = 40
LOCATION_SHIFT = np.asarray((3.0, 4.4, 0.5), dtype=np.float64)

PARAMETER_NAMES = (
    "construct:f=~x1",
    "construct:f=~x2",
    "construct:f=~x3",
    "construct:f~~construct:f",
    "x1~~x1",
    "x2~~x2",
    "x3~~x3",
    "x1~1",
    "x2~1",
    "x3~1",
    "construct:f~1",
)
FIXED_PARAMETER_NAMES = frozenset(("construct:f=~x1", "x1~1"))
FREE_PARAMETER_NAMES = (
    "construct:f=~x2",
    "construct:f=~x3",
    "construct:f~~construct:f",
    "x1~~x1",
    "x2~~x2",
    "x3~~x3",
    "x2~1",
    "x3~1",
    "construct:f~1",
)
STABLE_PARAMETER_IDS = {
    "construct:f=~x1": "parameter_66_7831",
    "construct:f=~x2": "parameter_66_7832",
    "construct:f=~x3": "parameter_66_7833",
    "construct:f~~construct:f": "variance_66",
    "x1~~x1": "residual_variance_7831",
    "x2~~x2": "residual_variance_7832",
    "x3~~x3": "residual_variance_7833",
    "x1~1": "parameter:intercept:x1",
    "x2~1": "parameter:intercept:x2",
    "x3~1": "parameter:intercept:x3",
    "construct:f~1": "parameter:factor_mean:f",
}

EXPECTED_FIXTURE_MEANS = np.asarray((3.015, 4.325, 0.45), dtype=np.float64)
EXPECTED_FIXTURE_COVARIANCE_ML = np.asarray(
    (
        (135.471275, 107.417125, 64.96625),
        (107.417125, 88.911875, 53.66325),
        (64.96625, 53.66325, 34.368),
    ),
    dtype=np.float64,
)

ESTIMATE_ABSOLUTE_TOLERANCE = 2e-5
ESTIMATE_RELATIVE_TOLERANCE = 2e-5
STANDARD_ERROR_ABSOLUTE_TOLERANCE = 5e-5
STANDARD_ERROR_RELATIVE_TOLERANCE = 5e-4
OBJECTIVE_ABSOLUTE_TOLERANCE = 2e-7
MEAN_REPRODUCTION_TOLERANCE = 2e-6
# The product optimizer may accept objective stagnation only while the
# gradient norm is below this declared bound. This is a convergence contract,
# not a tolerance fitted to the frozen fixture.
PRODUCT_GRADIENT_NORM_MAX = 1e-5

PRODUCT_FIXTURE_KEYS = frozenset(
    (
        "schema_version",
        "fixture_kind",
        "identity",
        "input",
        "parameters",
        "implied_means",
        "converged",
        "objective",
        "gradient_norm",
    )
)
PRODUCT_IDENTITY_KEYS = frozenset(
    (
        "estimator",
        "moment_adapter",
        "moment_result_schema_version",
        "runner_adapter",
    )
)
PRODUCT_INPUT_KEYS = frozenset(
    (
        "sample_size",
        "variable_order",
        "raw_sha256",
        "observed_means",
        "covariance_ml",
    )
)
PRODUCT_PARAMETER_KEYS = frozenset(
    ("name", "stable_id", "estimate", "standard_error", "fixed")
)
PRODUCT_IMPLIED_MEAN_KEYS = frozenset(("variable", "value"))


class OracleContractError(ValueError):
    """A typed, fail-closed oracle contract violation."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code


@dataclass(frozen=True)
class OracleEstimate:
    sample_size: int
    variables: tuple[str, ...]
    sample_means: np.ndarray
    sample_covariance_ml: np.ndarray
    implied_means: np.ndarray
    implied_covariance: np.ndarray
    parameters: dict[str, float]
    standard_errors: dict[str, float | None]
    expected_information: np.ndarray
    objective: float


def _json_sha256(value: Any) -> str:
    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _file_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def deterministic_product_rows() -> np.ndarray:
    """Recreate the preregistered 40-row product microcase independently."""

    rows = []
    for index in range(SAMPLE_SIZE):
        t = float(index) - 19.5
        a = float((index * 7) % 11) - 5.0
        b = float((index * 5) % 13) - 6.0
        rows.append(
            (
                t + 0.30 * a + LOCATION_SHIFT[0],
                0.80 * t + 0.50 * b + LOCATION_SHIFT[1],
                0.50 * t - 0.40 * a + 0.20 * b + LOCATION_SHIFT[2],
            )
        )
    return np.asarray(rows, dtype=np.float64)


def product_rows_sha256() -> str:
    return _json_sha256(
        {
            "variables": VARIABLES,
            "rows": deterministic_product_rows().tolist(),
        }
    )


def _validate_raw_contract(
    raw: np.ndarray,
    input_columns: Sequence[str],
    *,
    input_kind: str,
    model_kind: str,
    group_count: int,
    weight_column: str | None,
    ordinal: bool,
    marker_intercept_fixed: bool,
    marker_intercept_value: float,
) -> np.ndarray:
    if input_kind != "raw":
        raise OracleContractError(
            "mean_structure_raw_input_required",
            "This bounded mean structure requires raw observations.",
        )
    if model_kind != "cfa":
        raise OracleContractError(
            "mean_structure_cfa_required",
            "Structural regressions and structural intercepts are outside this slice.",
        )
    if group_count != 1:
        raise OracleContractError(
            "mean_structure_single_group_required",
            "Only one group is supported by this bounded oracle.",
        )
    if weight_column is not None:
        raise OracleContractError(
            "mean_structure_weights_unsupported",
            "Case, sampling, and frequency weights are outside this slice.",
        )
    if ordinal:
        raise OracleContractError(
            "mean_structure_continuous_required",
            "Ordinal thresholds are outside this continuous-ML slice.",
        )
    if not marker_intercept_fixed or not math.isclose(
        marker_intercept_value, 0.0, abs_tol=0.0
    ):
        raise OracleContractError(
            "latent_mean_marker_intercept_must_be_fixed",
            "The marker indicator intercept must be fixed at exactly zero.",
        )
    if len(input_columns) != len(set(input_columns)):
        raise OracleContractError(
            "duplicate_input_column", "Input-column identities must be unique."
        )
    if len(input_columns) < 3:
        raise OracleContractError(
            "local_underidentification",
            "A marker-identified one-factor model needs at least three indicators in this slice.",
        )
    if set(input_columns) != set(VARIABLES):
        raise OracleContractError(
            "raw_cfa_indicator_set_unsupported",
            "This micro-oracle requires exactly x1, x2, and x3.",
        )
    value = np.asarray(raw, dtype=np.float64)
    if value.ndim != 2 or value.shape[1] != 3 or value.shape[0] < 4:
        raise OracleContractError(
            "raw_shape_invalid",
            "Raw input must contain at least four rows and three columns.",
        )
    if not np.isfinite(value).all():
        raise OracleContractError(
            "raw_non_finite", "Raw observations must be complete and finite."
        )
    binding = {name: index for index, name in enumerate(input_columns)}
    canonical = value[:, [binding[name] for name in VARIABLES]]
    covariance = np.cov(canonical, rowvar=False, ddof=0)
    try:
        linalg.cholesky(covariance, lower=True, check_finite=True)
    except linalg.LinAlgError as error:
        raise OracleContractError(
            "sample_covariance_not_positive_definite",
            "The listwise ML covariance must be positive definite.",
        ) from error
    return canonical


def _implied_covariance(
    loading_2: float,
    loading_3: float,
    latent_variance: float,
    residual_variances: Iterable[float],
) -> np.ndarray:
    loadings = np.asarray((1.0, loading_2, loading_3), dtype=np.float64)
    residual = np.asarray(tuple(residual_variances), dtype=np.float64)
    if latent_variance <= 0.0 or np.any(residual <= 0.0):
        raise OracleContractError(
            "inadmissible_variance",
            "All latent and residual variances must be positive.",
        )
    return latent_variance * np.outer(loadings, loadings) + np.diag(residual)


def joint_ml_discrepancy(
    sample_covariance: np.ndarray,
    sample_means: np.ndarray,
    implied_covariance: np.ndarray,
    implied_means: np.ndarray,
) -> float:
    sample = np.asarray(sample_covariance, dtype=np.float64)
    sigma = np.asarray(implied_covariance, dtype=np.float64)
    observed = np.asarray(sample_means, dtype=np.float64)
    implied = np.asarray(implied_means, dtype=np.float64)
    if sample.shape != sigma.shape or sample.shape != (observed.size, observed.size):
        raise OracleContractError(
            "moment_shape_invalid", "Sample and implied moment dimensions must match."
        )
    sample_sign, sample_logdet = np.linalg.slogdet(sample)
    sigma_sign, sigma_logdet = np.linalg.slogdet(sigma)
    if sample_sign <= 0.0 or sigma_sign <= 0.0:
        raise OracleContractError(
            "moment_not_positive_definite",
            "Both covariance matrices must be positive definite.",
        )
    inverse = linalg.inv(sigma, check_finite=True)
    residual = observed - implied
    covariance_term = (
        sigma_logdet
        + float(np.trace(sample @ inverse))
        - sample_logdet
        - float(sample.shape[0])
    )
    mean_term = float(residual @ inverse @ residual)
    return float(covariance_term + mean_term)


def _natural_parameter_vector(parameters: dict[str, float]) -> np.ndarray:
    return np.asarray(
        [parameters[name] for name in FREE_PARAMETER_NAMES], dtype=np.float64
    )


def moments_from_free_parameters(
    parameters: Sequence[float],
) -> tuple[np.ndarray, np.ndarray]:
    values = np.asarray(parameters, dtype=np.float64)
    if values.shape != (9,) or not np.isfinite(values).all():
        raise OracleContractError(
            "free_parameter_shape_invalid",
            "Exactly nine finite free parameters are required.",
        )
    loading_2, loading_3, latent_variance = values[:3]
    residual = values[3:6]
    intercept_2, intercept_3, latent_mean = values[6:9]
    sigma = _implied_covariance(loading_2, loading_3, latent_variance, residual)
    mu = np.asarray(
        (
            latent_mean,
            intercept_2 + loading_2 * latent_mean,
            intercept_3 + loading_3 * latent_mean,
        ),
        dtype=np.float64,
    )
    return sigma, mu


def expected_information(parameters: Sequence[float], sample_size: int) -> np.ndarray:
    """Return expected information for the nine natural free parameters."""

    values = np.asarray(parameters, dtype=np.float64)
    sigma, _ = moments_from_free_parameters(values)
    inverse = linalg.inv(sigma, check_finite=True)
    loading_2, loading_3, latent_variance = values[:3]
    latent_mean = values[8]
    loadings = np.asarray((1.0, loading_2, loading_3), dtype=np.float64)

    covariance_derivatives = []
    for indicator in (1, 2):
        basis = np.zeros(3)
        basis[indicator] = 1.0
        covariance_derivatives.append(
            latent_variance * (np.outer(basis, loadings) + np.outer(loadings, basis))
        )
    covariance_derivatives.append(np.outer(loadings, loadings))
    for indicator in range(3):
        derivative = np.zeros((3, 3))
        derivative[indicator, indicator] = 1.0
        covariance_derivatives.append(derivative)
    covariance_derivatives.extend(np.zeros((3, 3)) for _ in range(3))

    mean_derivatives = [
        np.asarray((0.0, latent_mean, 0.0)),
        np.asarray((0.0, 0.0, latent_mean)),
        np.zeros(3),
        np.zeros(3),
        np.zeros(3),
        np.zeros(3),
        np.asarray((0.0, 1.0, 0.0)),
        np.asarray((0.0, 0.0, 1.0)),
        loadings,
    ]

    information = np.empty((9, 9), dtype=np.float64)
    for row in range(9):
        for column in range(row, 9):
            covariance_information = 0.5 * np.trace(
                inverse
                @ covariance_derivatives[row]
                @ inverse
                @ covariance_derivatives[column]
            )
            mean_information = (
                mean_derivatives[row] @ inverse @ mean_derivatives[column]
            )
            value = float(sample_size) * float(
                covariance_information + mean_information
            )
            information[row, column] = value
            information[column, row] = value
    return information


def estimate_marker_cfa_mean_structure(
    raw: np.ndarray,
    *,
    input_columns: Sequence[str] = VARIABLES,
    input_kind: str = "raw",
    model_kind: str = "cfa",
    group_count: int = 1,
    weight_column: str | None = None,
    ordinal: bool = False,
    marker_intercept_fixed: bool = True,
    marker_intercept_value: float = 0.0,
) -> OracleEstimate:
    canonical = _validate_raw_contract(
        raw,
        input_columns,
        input_kind=input_kind,
        model_kind=model_kind,
        group_count=group_count,
        weight_column=weight_column,
        ordinal=ordinal,
        marker_intercept_fixed=marker_intercept_fixed,
        marker_intercept_value=marker_intercept_value,
    )
    means = np.mean(canonical, axis=0)
    covariance = np.cov(canonical, rowvar=False, ddof=0)
    s12, s13, s23 = covariance[0, 1], covariance[0, 2], covariance[1, 2]
    if min(abs(float(s12)), abs(float(s13)), abs(float(s23))) <= 1e-12:
        raise OracleContractError(
            "local_underidentification",
            "Three nonzero cross-covariances are required for this marker CFA.",
        )

    latent_variance = float(s12 * s13 / s23)
    loading_2 = float(s23 / s13)
    loading_3 = float(s23 / s12)
    loadings = np.asarray((1.0, loading_2, loading_3), dtype=np.float64)
    residual = np.diag(covariance) - latent_variance * np.square(loadings)
    sigma = _implied_covariance(loading_2, loading_3, latent_variance, residual)

    latent_mean = float(means[0])
    intercept_2 = float(means[1] - loading_2 * latent_mean)
    intercept_3 = float(means[2] - loading_3 * latent_mean)
    parameters = {
        "construct:f=~x1": 1.0,
        "construct:f=~x2": loading_2,
        "construct:f=~x3": loading_3,
        "construct:f~~construct:f": latent_variance,
        "x1~~x1": float(residual[0]),
        "x2~~x2": float(residual[1]),
        "x3~~x3": float(residual[2]),
        "x1~1": 0.0,
        "x2~1": intercept_2,
        "x3~1": intercept_3,
        "construct:f~1": latent_mean,
    }
    free = _natural_parameter_vector(parameters)
    information = expected_information(free, canonical.shape[0])
    try:
        parameter_covariance = linalg.inv(information, check_finite=True)
    except linalg.LinAlgError as error:
        raise OracleContractError(
            "expected_information_singular",
            "The expected information matrix is singular.",
        ) from error
    diagonal = np.diag(parameter_covariance)
    if np.any(diagonal <= 0.0) or not np.isfinite(diagonal).all():
        raise OracleContractError(
            "expected_information_singular",
            "Expected-information variances must be finite and positive.",
        )
    free_standard_errors = np.sqrt(diagonal)
    standard_errors: dict[str, float | None] = {
        "construct:f=~x1": None,
        "x1~1": None,
    }
    standard_errors.update(
        {
            name: float(free_standard_errors[index])
            for index, name in enumerate(FREE_PARAMETER_NAMES)
        }
    )
    implied_means = np.asarray(
        (
            latent_mean,
            intercept_2 + loading_2 * latent_mean,
            intercept_3 + loading_3 * latent_mean,
        )
    )
    objective = joint_ml_discrepancy(covariance, means, sigma, implied_means)
    return OracleEstimate(
        sample_size=canonical.shape[0],
        variables=VARIABLES,
        sample_means=means,
        sample_covariance_ml=covariance,
        implied_means=implied_means,
        implied_covariance=sigma,
        parameters=parameters,
        standard_errors=standard_errors,
        expected_information=information,
        objective=objective,
    )


def exact_moment_rows(
    *,
    sample_size: int,
    loadings: Sequence[float],
    latent_variance: float,
    residual_variances: Sequence[float],
    intercepts: Sequence[float],
    latent_mean: float,
) -> np.ndarray:
    """Build deterministic raw rows with exact requested ML moments."""

    if sample_size < 8:
        raise OracleContractError(
            "exact_fixture_sample_too_small", "At least eight rows are required."
        )
    loading = np.asarray(loadings, dtype=np.float64)
    residual = np.asarray(residual_variances, dtype=np.float64)
    intercept = np.asarray(intercepts, dtype=np.float64)
    if loading.shape != (3,) or not math.isclose(loading[0], 1.0):
        raise OracleContractError(
            "marker_loading_invalid", "The first loading must be fixed at one."
        )
    covariance = latent_variance * np.outer(loading, loading) + np.diag(residual)
    mean = intercept + loading * latent_mean

    index = np.arange(sample_size, dtype=np.float64)
    candidates = np.column_stack(
        (
            np.sin((index + 1.0) * 0.71),
            np.cos((index + 1.0) * 1.13),
            np.sin((index + 1.0) * 1.73) + np.cos((index + 1.0) * 0.37),
        )
    )
    candidates -= np.mean(candidates, axis=0)
    orthogonal, _ = linalg.qr(candidates, mode="economic")
    standardized = orthogonal * math.sqrt(float(sample_size))
    root = linalg.cholesky(covariance, lower=True)
    return mean + standardized @ root.T


def _close(left: float, right: float, absolute: float, relative: float) -> bool:
    return math.isclose(left, right, abs_tol=absolute, rel_tol=relative)


def _has_exact_keys(value: Any, expected: frozenset[str]) -> bool:
    return isinstance(value, dict) and set(value) == expected


def _is_finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def _validate_product_fixture_payload(fixture: Any) -> dict[str, Any]:
    if not _has_exact_keys(fixture, PRODUCT_FIXTURE_KEYS):
        raise OracleContractError(
            "frozen_product_fixture_schema_invalid",
            "The product fixture must contain exactly the frozen top-level fields.",
        )
    if fixture.get("schema_version") != 1 or fixture.get("fixture_kind") != (
        "quickpls_cbsem_raw_cfa_mean_structure_v4_product_result"
    ):
        raise OracleContractError(
            "frozen_product_fixture_schema_invalid",
            "The frozen product fixture schema or kind is invalid.",
        )

    expected_identity = {
        "estimator": ESTIMATOR_ID,
        "moment_adapter": MOMENT_ADAPTER_ID,
        "moment_result_schema_version": MOMENT_RESULT_SCHEMA_VERSION,
        "runner_adapter": RUNNER_ADAPTER_ID,
    }
    identity = fixture.get("identity")
    if (
        not _has_exact_keys(identity, PRODUCT_IDENTITY_KEYS)
        or identity != expected_identity
    ):
        raise OracleContractError(
            "frozen_product_fixture_identity_mismatch",
            "The product fixture does not bind the preregistered V4 identity quartet.",
        )

    input_row = fixture.get("input")
    if not _has_exact_keys(input_row, PRODUCT_INPUT_KEYS):
        raise OracleContractError(
            "frozen_product_fixture_input_mismatch",
            "The product fixture input contract is incomplete or contains unknown fields.",
        )
    variable_order = input_row.get("variable_order")
    if (
        input_row.get("sample_size") != SAMPLE_SIZE
        or not isinstance(variable_order, list)
        or tuple(variable_order) != VARIABLES
        or input_row.get("raw_sha256") != product_rows_sha256()
    ):
        raise OracleContractError(
            "frozen_product_fixture_input_mismatch",
            "The product fixture is not derived from the preregistered raw microcase.",
        )
    observed_means = input_row.get("observed_means")
    covariance_ml = input_row.get("covariance_ml")
    if (
        not isinstance(observed_means, list)
        or len(observed_means) != len(VARIABLES)
        or not all(_is_finite_number(value) for value in observed_means)
        or not isinstance(covariance_ml, list)
        or len(covariance_ml) != len(VARIABLES)
        or not all(
            isinstance(row, list)
            and len(row) == len(VARIABLES)
            and all(_is_finite_number(value) for value in row)
            for row in covariance_ml
        )
    ):
        raise OracleContractError(
            "frozen_product_fixture_input_mismatch",
            "Observed means and the ML covariance must be finite and have exact shape.",
        )

    rows = fixture.get("parameters")
    if (
        not isinstance(rows, list)
        or len(rows) != len(PARAMETER_NAMES)
        or not all(_has_exact_keys(row, PRODUCT_PARAMETER_KEYS) for row in rows)
    ):
        raise OracleContractError(
            "frozen_product_fixture_parameters_invalid",
            "The product fixture must contain exact parameter rows.",
        )
    names = [row.get("name") for row in rows]
    if (
        not all(isinstance(name, str) for name in names)
        or len(set(names)) != len(names)
        or set(names) != set(PARAMETER_NAMES)
    ):
        raise OracleContractError(
            "frozen_product_fixture_parameter_identity_mismatch",
            "Product parameter names must be unique and complete.",
        )
    identities = {row["name"]: row.get("stable_id") for row in rows}
    if identities != STABLE_PARAMETER_IDS:
        raise OracleContractError(
            "frozen_product_fixture_parameter_identity_mismatch",
            "Parameter names and stable IDs do not match the V4 contract.",
        )
    if not all(
        _is_finite_number(row.get("estimate"))
        and isinstance(row.get("fixed"), bool)
        and (
            row.get("standard_error") is None
            or _is_finite_number(row.get("standard_error"))
        )
        for row in rows
    ):
        raise OracleContractError(
            "frozen_product_fixture_parameters_invalid",
            "Parameter estimates, fixed flags, or standard errors are malformed.",
        )

    implied_rows = fixture.get("implied_means")
    if (
        not isinstance(implied_rows, list)
        or len(implied_rows) != len(VARIABLES)
        or not all(
            _has_exact_keys(row, PRODUCT_IMPLIED_MEAN_KEYS) for row in implied_rows
        )
    ):
        raise OracleContractError(
            "frozen_product_fixture_implied_means_invalid",
            "The product fixture must contain exact implied-mean rows.",
        )
    implied_variables = [row.get("variable") for row in implied_rows]
    if (
        not all(isinstance(variable, str) for variable in implied_variables)
        or len(set(implied_variables)) != len(implied_variables)
        or set(implied_variables) != set(VARIABLES)
        or not all(_is_finite_number(row.get("value")) for row in implied_rows)
    ):
        raise OracleContractError(
            "frozen_product_fixture_implied_means_invalid",
            "Implied-mean variables must be unique, complete, and finite.",
        )

    if (
        not isinstance(fixture.get("converged"), bool)
        or not _is_finite_number(fixture.get("objective"))
        or not _is_finite_number(fixture.get("gradient_norm"))
    ):
        raise OracleContractError(
            "frozen_product_fixture_diagnostics_invalid",
            "Convergence, objective, and gradient diagnostics are malformed.",
        )
    return fixture


def load_product_fixture(path: Path = PRODUCT_FIXTURE_PATH) -> dict[str, Any]:
    """Load the post-green product result; never invent a missing fixture."""

    if not path.is_file():
        raise OracleContractError(
            "frozen_product_fixture_missing",
            f"No product-generated fixture exists at {path.relative_to(ROOT)}.",
        )
    try:
        fixture = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise OracleContractError(
            "frozen_product_fixture_schema_invalid",
            "The product fixture is not valid UTF-8 JSON.",
        ) from error
    return _validate_product_fixture_payload(fixture)


def compare_product_fixture(fixture: dict[str, Any]) -> dict[str, Any]:
    fixture = _validate_product_fixture_payload(fixture)
    oracle = estimate_marker_cfa_mean_structure(deterministic_product_rows())
    input_row = fixture["input"]
    input_checks = {
        "means": bool(
            np.allclose(
                np.asarray(input_row.get("observed_means"), dtype=np.float64),
                oracle.sample_means,
                atol=1e-12,
                rtol=0.0,
            )
        ),
        "covariance_ml": bool(
            np.allclose(
                np.asarray(input_row.get("covariance_ml"), dtype=np.float64),
                oracle.sample_covariance_ml,
                atol=2e-11,
                rtol=0.0,
            )
        ),
    }
    parameter_rows = {row["name"]: row for row in fixture["parameters"]}
    parameter_checks = []
    for name in PARAMETER_NAMES:
        row = parameter_rows[name]
        expected_fixed = name in FIXED_PARAMETER_NAMES
        fixed_ok = row["fixed"] is expected_fixed
        estimate_ok = _close(
            float(row["estimate"]),
            oracle.parameters[name],
            ESTIMATE_ABSOLUTE_TOLERANCE,
            ESTIMATE_RELATIVE_TOLERANCE,
        )
        expected_se = oracle.standard_errors[name]
        actual_se = row.get("standard_error")
        if expected_se is None:
            se_ok = actual_se is None
        else:
            se_ok = (
                actual_se is not None
                and float(actual_se) > 0.0
                and _close(
                    float(actual_se),
                    expected_se,
                    STANDARD_ERROR_ABSOLUTE_TOLERANCE,
                    STANDARD_ERROR_RELATIVE_TOLERANCE,
                )
            )
        parameter_checks.append(
            {
                "name": name,
                "stable_id": STABLE_PARAMETER_IDS[name],
                "fixed_passed": fixed_ok,
                "estimate_passed": estimate_ok,
                "standard_error_passed": se_ok,
            }
        )
    implied_rows = fixture.get("implied_means", [])
    implied = {row.get("variable"): row.get("value") for row in implied_rows}
    implied_ok = set(implied) == set(VARIABLES) and all(
        _close(
            float(implied[name]),
            float(oracle.implied_means[index]),
            MEAN_REPRODUCTION_TOLERANCE,
            0.0,
        )
        for index, name in enumerate(VARIABLES)
    )
    objective_ok = (
        math.isfinite(float(fixture.get("objective", math.nan)))
        and abs(float(fixture["objective"]) - oracle.objective)
        <= OBJECTIVE_ABSOLUTE_TOLERANCE
    )
    converged_ok = fixture["converged"] is True
    gradient = float(fixture["gradient_norm"])
    gradient_ok = 0.0 <= gradient <= PRODUCT_GRADIENT_NORM_MAX
    passed = (
        all(input_checks.values())
        and all(
            row["fixed_passed"]
            and row["estimate_passed"]
            and row["standard_error_passed"]
            for row in parameter_checks
        )
        and implied_ok
        and objective_ok
        and converged_ok
        and gradient_ok
    )
    return {
        "passed": passed,
        "input_checks": input_checks,
        "parameter_checks": parameter_checks,
        "implied_means_passed": implied_ok,
        "objective_passed": objective_ok,
        "converged_passed": converged_ok,
        "gradient_within_product_convergence_contract": gradient_ok,
    }


def _typed_failure(case: str) -> str:
    rows = deterministic_product_rows()
    kwargs: dict[str, Any] = {}
    if case == "free_marker_intercept":
        kwargs["marker_intercept_fixed"] = False
    elif case == "nonzero_marker_intercept":
        kwargs["marker_intercept_value"] = 1.0
    elif case == "matrix_means":
        kwargs["input_kind"] = "covariance"
    elif case == "structural_intercept":
        kwargs["model_kind"] = "sem"
    elif case == "multiple_groups":
        kwargs["group_count"] = 2
    elif case == "ordinal":
        kwargs["ordinal"] = True
    elif case == "weights":
        kwargs["weight_column"] = "w"
    elif case == "non_positive_definite":
        rows = rows.copy()
        rows[:, 2] = rows[:, 1]
    elif case == "underidentified_two_indicator":
        rows = rows[:, :2]
        kwargs["input_columns"] = ("x1", "x2")
    else:
        raise AssertionError(f"unknown failure case {case}")
    try:
        estimate_marker_cfa_mean_structure(rows, **kwargs)
    except OracleContractError as error:
        return error.code
    raise AssertionError(f"adversarial scenario {case} did not fail")


def build_work_report() -> dict[str, Any]:
    estimate = estimate_marker_cfa_mean_structure(deterministic_product_rows())
    fixture_check: dict[str, Any]
    fixture_blocker: str | None = None
    try:
        fixture_check = compare_product_fixture(load_product_fixture())
        if not fixture_check["passed"]:
            fixture_blocker = "frozen_product_fixture_oracle_comparison_failed"
    except OracleContractError as error:
        fixture_check = {"passed": False, "failure_code": error.code}
        fixture_blocker = error.code

    failure_expectations = {
        "free_marker_intercept": "latent_mean_marker_intercept_must_be_fixed",
        "nonzero_marker_intercept": "latent_mean_marker_intercept_must_be_fixed",
        "matrix_means": "mean_structure_raw_input_required",
        "structural_intercept": "mean_structure_cfa_required",
        "multiple_groups": "mean_structure_single_group_required",
        "ordinal": "mean_structure_continuous_required",
        "weights": "mean_structure_weights_unsupported",
        "non_positive_definite": "sample_covariance_not_positive_definite",
        "underidentified_two_indicator": "local_underidentification",
    }
    failures = [
        {
            "scenario": name,
            "expected": expected,
            "observed": _typed_failure(name),
        }
        for name, expected in failure_expectations.items()
    ]
    for row in failures:
        row["passed"] = row["expected"] == row["observed"]

    source = ROOT / SOURCE_PATH
    blockers = [
        blocker
        for blocker in (
            fixture_blocker,
            "no_second_independently_maintained_sem_implementation_compared",
            "no_qualification_sized_monte_carlo_coverage_campaign",
            "archive_export_packaged_windows_performance_and_scientific_review_open",
        )
        if blocker is not None
    ]
    independent_checks_passed = bool(
        np.allclose(estimate.sample_means, EXPECTED_FIXTURE_MEANS, atol=2e-14, rtol=0.0)
        and np.allclose(
            estimate.sample_covariance_ml,
            EXPECTED_FIXTURE_COVARIANCE_ML,
            atol=2e-12,
            rtol=0.0,
        )
        and np.max(np.abs(estimate.sample_means - estimate.implied_means)) <= 2e-12
        and np.max(np.abs(estimate.sample_covariance_ml - estimate.implied_covariance))
        <= 2e-10
        and abs(estimate.objective) <= 2e-12
        and np.min(np.linalg.eigvalsh(estimate.expected_information)) > 0.0
        and all(row["passed"] for row in failures)
    )
    return {
        "schema_version": 1,
        "report_kind": "cbsem_raw_cfa_mean_structure_v4_oracle_work_report",
        "oracle_version": ORACLE_VERSION,
        "independent_checks_passed": independent_checks_passed,
        "product_fixture_comparison": fixture_check,
        "qualification_role_satisfied": False,
        "receipt_eligible": False,
        "promotion_requested": False,
        "coverage_or_evidence_state_changed": False,
        "surface": "internal_labs_only",
        "identity_contract": {
            "estimator": ESTIMATOR_ID,
            "moment_adapter": MOMENT_ADAPTER_ID,
            "moment_result_schema_version": MOMENT_RESULT_SCHEMA_VERSION,
            "runner_adapter": RUNNER_ADAPTER_ID,
        },
        "reference": {
            "implementation": "transparent_numpy_scipy_marker_cfa_mean_structure",
            "source": SOURCE_PATH.as_posix(),
            "source_sha256": _file_sha256(source),
            "numpy_version": np.__version__,
            "scipy_version": scipy.__version__,
            "product_code_imported": False,
            "product_executable_invoked": False,
            "installation_attempted": False,
        },
        "primary_method_reference": {
            "citation": "Bollen, K. A. (1989), Structural Equations with Latent Variables, Wiley",
            "doi": "10.1002/9781118619179",
            "joint_ml_discrepancy": "F_cov + (xbar-mu)' Sigma^-1 (xbar-mu)",
            "expected_information": "n/2 tr(Sigma^-1 dSigma_i Sigma^-1 dSigma_j) + n dmu_i' Sigma^-1 dmu_j",
        },
        "input_contract": {
            "sample_size": SAMPLE_SIZE,
            "variables": list(VARIABLES),
            "raw_sha256": product_rows_sha256(),
            "means": estimate.sample_means.tolist(),
            "covariance_ml": estimate.sample_covariance_ml.tolist(),
            "covariance_denominator": "maximum_likelihood_n",
        },
        "stable_parameter_ids": STABLE_PARAMETER_IDS,
        "typed_failures": failures,
        "explicit_exclusions": [
            "general_sem_structural_intercepts",
            "groups",
            "ordinal_thresholds",
            "weights",
            "matrix_means",
            "effects_coding",
            "fixed_variance_latent_mean_identification",
            "bootstrap_or_other_resampling",
        ],
        "blockers": blockers,
    }


def write_work_report(path: Path = REPORT_PATH) -> Path:
    report = build_work_report()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--output", type=Path, default=REPORT_PATH)
    args = parser.parse_args()
    report = build_work_report()
    if args.check:
        if not args.output.is_file():
            print(f"missing work report: {args.output}")
            return 1
        if json.loads(args.output.read_text(encoding="utf-8")) != report:
            print(f"stale work report: {args.output}")
            return 1
    else:
        write_work_report(args.output)
    print(
        json.dumps(
            {
                "path": args.output.resolve().relative_to(ROOT.resolve()).as_posix(),
                "independent_checks_passed": report["independent_checks_passed"],
                "product_fixture_comparison": report["product_fixture_comparison"],
                "qualification_role_satisfied": False,
                "receipt_eligible": False,
                "blockers": report["blockers"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if report["independent_checks_passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
