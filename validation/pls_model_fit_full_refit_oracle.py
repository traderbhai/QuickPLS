#!/usr/bin/env python3
"""Independent NumPy/SciPy full-refit oracle for PLS model-fit v2.

This validation-only module does not import or execute QuickPLS product code.
It implements a transparent recursive PLS-PM estimator, the observed and two
model-implied indicator correlation matrices, the frozen model-fit criteria,
and a small adapted Bollen--Stine indexed case-resampling workflow.

The current oracle scope is raw, finite, complete-case recursive PLS-PM with
Mode A or Mode B blocks.  It intentionally does not claim PLSc parity,
production draw counts, packaged execution, performance qualification, or a
second independent full-pipeline reference.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import platform
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Mapping, Sequence

import numpy as np
import scipy
from scipy import linalg


ORACLE_VERSION = "pls_model_fit_full_refit_oracle_v1"
INDEX_PLAN_VERSION = "numpy_pcg64_seedsequence_indexed_cases_v1"
SAMPLE_DIGEST_VERSION = "sha256_little_endian_u64_indices_v1"
MINIMUM_USABLE_FRACTION = 0.90
VARIANTS = ("saturated", "estimated")
CRITERIA = ("srmr", "d_uls", "d_g")


class ModelFitOracleError(ValueError):
    """Typed, fail-closed oracle error."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


@dataclass(frozen=True)
class ConstructSpec:
    construct_id: str
    indicators: tuple[str, ...]
    mode: str = "reflective"


@dataclass(frozen=True)
class ModelSpec:
    constructs: tuple[ConstructSpec, ...]
    paths: tuple[tuple[str, str], ...]

    @property
    def indicator_order(self) -> tuple[str, ...]:
        return tuple(
            indicator
            for construct in self.constructs
            for indicator in construct.indicators
        )


@dataclass(frozen=True)
class FitStatistics:
    srmr: float
    d_uls: float
    d_g: float
    chi_square: float
    degrees_of_freedom: float
    nfi: float


@dataclass(frozen=True)
class PlsPointFit:
    indicator_order: tuple[str, ...]
    analytical_sample_size: int
    iterations: int
    outer_weights: tuple[tuple[float, ...], ...]
    outer_loadings: tuple[tuple[float, ...], ...]
    path_coefficients: tuple[tuple[str, str, float], ...]
    observed_correlation: np.ndarray
    saturated_implied_correlation: np.ndarray
    estimated_implied_correlation: np.ndarray
    saturated: FitStatistics
    estimated: FitStatistics
    null_model_chi_square: float


@dataclass(frozen=True)
class ReplicateLedgerEntry:
    variant: str
    replicate_index: int
    index_plan_version: str
    sample_index_digest: str
    status: str
    srmr: float | None
    d_uls: float | None
    d_g: float | None
    failure_code: str | None
    failure_message: str | None


@dataclass(frozen=True)
class CriterionSummary:
    criterion: str
    status: str
    original: float
    requested_replicates: int
    minimum_usable_replicates: int
    usable_replicates: int
    failed_replicates: int
    usable_index_digest: str
    replicate_min: float | None
    replicate_max: float | None
    upper_95: float | None
    upper_99: float | None
    not_rejected_95: bool | None
    not_rejected_99: bool | None
    exceed_or_equal_count: int
    empirical_upper_tail_probability: float | None
    unavailable_reason_code: str | None


@dataclass(frozen=True)
class VariantExactFit:
    variant: str
    status: str
    target_correlation: np.ndarray
    recovered_transformed_correlation: np.ndarray
    ledger: tuple[ReplicateLedgerEntry, ...]
    criteria: tuple[CriterionSummary, ...]


@dataclass(frozen=True)
class ExactFitResult:
    oracle_version: str
    index_plan_version: str
    sample_digest_version: str
    seed: int
    requested_replicates: int
    point_fit: PlsPointFit
    saturated: VariantExactFit
    estimated: VariantExactFit


def _fail(code: str, message: str) -> ModelFitOracleError:
    return ModelFitOracleError(code, message)


def _as_raw_matrix(value: object, expected_columns: int) -> np.ndarray:
    matrix = np.asarray(value, dtype=float)
    if matrix.ndim != 2 or matrix.shape[0] < 3 or matrix.shape[1] != expected_columns:
        raise _fail(
            "model_fit_oracle.invalid_raw_shape",
            f"raw data must have at least three rows and exactly {expected_columns} columns",
        )
    if not np.isfinite(matrix).all():
        raise _fail(
            "model_fit_oracle.non_finite_raw_data",
            "raw data must contain only finite complete cases",
        )
    return matrix


def _validate_model(model: ModelSpec) -> tuple[dict[str, int], tuple[int, ...]]:
    if not model.constructs:
        raise _fail("model_fit_oracle.empty_model", "at least one construct is required")
    construct_ids = [construct.construct_id for construct in model.constructs]
    if any(not value.strip() for value in construct_ids) or len(set(construct_ids)) != len(
        construct_ids
    ):
        raise _fail(
            "model_fit_oracle.invalid_construct_identity",
            "construct identifiers must be nonempty and unique",
        )
    indicators = list(model.indicator_order)
    if any(not value.strip() for value in indicators) or len(set(indicators)) != len(indicators):
        raise _fail(
            "model_fit_oracle.invalid_indicator_identity",
            "indicator identifiers must be nonempty and unique",
        )
    if len(indicators) < 2 or any(not construct.indicators for construct in model.constructs):
        raise _fail(
            "model_fit_oracle.insufficient_indicators",
            "at least two indicators overall and one indicator per construct are required",
        )
    for construct in model.constructs:
        if construct.mode not in {"reflective", "formative"}:
            raise _fail(
                "model_fit_oracle.unsupported_measurement_mode",
                f"unsupported mode {construct.mode!r} for {construct.construct_id}",
            )
    index = {construct_id: position for position, construct_id in enumerate(construct_ids)}
    seen_paths: set[tuple[str, str]] = set()
    indegree = [0] * len(construct_ids)
    successors: list[list[int]] = [[] for _ in construct_ids]
    for source, target in model.paths:
        if source not in index or target not in index:
            raise _fail(
                "model_fit_oracle.unknown_path_endpoint",
                f"path {source!r} -> {target!r} references an unknown construct",
            )
        if source == target:
            raise _fail("model_fit_oracle.self_path", f"self path {source!r} is invalid")
        if (source, target) in seen_paths:
            raise _fail(
                "model_fit_oracle.duplicate_path",
                f"path {source!r} -> {target!r} is duplicated",
            )
        seen_paths.add((source, target))
        source_index = index[source]
        target_index = index[target]
        indegree[target_index] += 1
        successors[source_index].append(target_index)
    queue = [position for position, degree in enumerate(indegree) if degree == 0]
    order: list[int] = []
    while queue:
        current = queue.pop(0)
        order.append(current)
        for successor in successors[current]:
            indegree[successor] -= 1
            if indegree[successor] == 0:
                queue.append(successor)
    if len(order) != len(construct_ids):
        raise _fail(
            "model_fit_oracle.nonrecursive_model",
            "the bounded independent oracle requires an acyclic structural model",
        )
    return index, tuple(order)


def _standardize(matrix: np.ndarray) -> np.ndarray:
    centered = matrix - np.mean(matrix, axis=0)
    deviations = np.std(centered, axis=0, ddof=1)
    if np.any(~np.isfinite(deviations)) or np.any(deviations <= np.finfo(float).eps):
        raise _fail(
            "model_fit_oracle.constant_indicator",
            "at least one indicator has zero or non-finite sample variance",
        )
    return centered / deviations


def _correlation(matrix: np.ndarray) -> np.ndarray:
    centered = matrix - np.mean(matrix, axis=0)
    deviations = np.std(centered, axis=0, ddof=1)
    if np.any(deviations <= np.finfo(float).eps) or np.any(~np.isfinite(deviations)):
        raise _fail(
            "model_fit_oracle.constant_indicator",
            "correlation is undefined for a constant or non-finite column",
        )
    standardized = centered / deviations
    result = standardized.T @ standardized / (standardized.shape[0] - 1)
    result = (result + result.T) / 2
    np.fill_diagonal(result, 1.0)
    return result


def _positive_definite_correlation(value: object, label: str) -> np.ndarray:
    matrix = np.asarray(value, dtype=float)
    if matrix.ndim != 2 or matrix.shape[0] == 0 or matrix.shape[0] != matrix.shape[1]:
        raise _fail(
            f"model_fit_oracle.{label}_not_square",
            f"{label} must be a nonempty square matrix",
        )
    if not np.isfinite(matrix).all():
        raise _fail(
            f"model_fit_oracle.{label}_non_finite",
            f"{label} must contain only finite values",
        )
    if not np.allclose(matrix, matrix.T, rtol=0, atol=1e-12):
        raise _fail(
            f"model_fit_oracle.{label}_not_symmetric",
            f"{label} must be symmetric",
        )
    if not np.allclose(np.diag(matrix), 1.0, rtol=0, atol=1e-12) or np.any(
        np.abs(matrix) > 1 + 1e-12
    ):
        raise _fail(
            f"model_fit_oracle.{label}_not_correlation",
            f"{label} must be a unit-diagonal correlation matrix",
        )
    eigenvalues = linalg.eigvalsh(matrix, check_finite=True)
    tolerance = (
        max(1.0, float(np.max(np.abs(eigenvalues))))
        * matrix.shape[0]
        * np.finfo(float).eps
        * 128
    )
    if np.any(eigenvalues <= tolerance):
        raise _fail(
            f"model_fit_oracle.{label}_not_positive_definite",
            f"{label} must be numerically positive definite without repair",
        )
    return matrix


def _symmetric_power(matrix: np.ndarray, exponent: float, label: str) -> np.ndarray:
    checked = _positive_definite_correlation(matrix, label)
    eigenvalues, eigenvectors = linalg.eigh(checked, check_finite=True)
    powered = (eigenvectors * np.power(eigenvalues, exponent)) @ eigenvectors.T
    return (powered + powered.T) / 2


def null_transform(observations: object, target_correlation: object) -> np.ndarray:
    """Transform complete cases so their sample correlation equals ``target``."""

    values = np.asarray(observations, dtype=float)
    if values.ndim != 2 or values.shape[0] < 3 or values.shape[1] < 2:
        raise _fail(
            "model_fit_oracle.invalid_null_transform_input",
            "null transformation needs at least three rows and two columns",
        )
    if not np.isfinite(values).all():
        raise _fail(
            "model_fit_oracle.non_finite_null_transform_input",
            "null transformation accepts only finite complete cases",
        )
    standardized = _standardize(values)
    observed = _positive_definite_correlation(
        _correlation(standardized), "observed_correlation"
    )
    target = _positive_definite_correlation(target_correlation, "target_correlation")
    if target.shape != observed.shape:
        raise _fail(
            "model_fit_oracle.null_transform_dimension_mismatch",
            "target and observed correlation dimensions differ",
        )
    transform = _symmetric_power(
        observed, -0.5, "observed_correlation"
    ) @ _symmetric_power(target, 0.5, "target_correlation")
    transformed = standardized @ transform
    recovered = _correlation(transformed)
    if not np.allclose(recovered, target, rtol=0, atol=1e-9):
        raise _fail(
            "model_fit_oracle.null_transform_identity_failed",
            "transformed sample correlation does not reproduce the target",
        )
    return transformed


def _standardize_vector(values: np.ndarray, subject: str) -> np.ndarray:
    centered = values - np.mean(values)
    deviation = float(np.std(centered, ddof=1))
    if not math.isfinite(deviation) or deviation <= np.finfo(float).eps:
        raise _fail(
            "model_fit_oracle.zero_variance_score",
            f"{subject} has zero or non-finite variance",
        )
    return centered / deviation


def _orient_weights(block: np.ndarray, weights: np.ndarray) -> np.ndarray:
    candidate = np.asarray(weights, dtype=float).copy()
    score = block @ candidate
    reference = np.sum(block, axis=1)
    association = float(np.cov(score, reference, ddof=1)[0, 1])
    if association < -1e-15 or (abs(association) <= 1e-15 and np.sum(candidate) < 0):
        candidate *= -1
    return candidate


def _normalize_weights(block: np.ndarray, weights: np.ndarray, subject: str) -> np.ndarray:
    oriented = _orient_weights(block, weights)
    score_deviation = float(np.std(block @ oriented, ddof=1))
    if not math.isfinite(score_deviation) or score_deviation <= np.finfo(float).eps:
        raise _fail(
            "model_fit_oracle.zero_variance_outer_score",
            f"outer weights for {subject} produce zero variance",
        )
    return _orient_weights(block, oriented / score_deviation)


def _ols(predictors: np.ndarray, outcome: np.ndarray, subject: str) -> np.ndarray:
    matrix = np.asarray(predictors, dtype=float)
    if matrix.ndim != 2 or matrix.shape[0] != outcome.shape[0]:
        raise _fail(
            "model_fit_oracle.invalid_regression_shape",
            f"invalid predictor matrix for {subject}",
        )
    if matrix.shape[1] == 0:
        return np.empty(0, dtype=float)
    centered = matrix - np.mean(matrix, axis=0)
    centered_outcome = outcome - np.mean(outcome)
    singular_values = linalg.svdvals(centered, check_finite=True)
    tolerance = (
        singular_values[0]
        * max(centered.shape)
        * np.finfo(float).eps
        * 100
        if singular_values.size
        else 0.0
    )
    if int(np.sum(singular_values > tolerance)) < matrix.shape[1]:
        raise _fail(
            "model_fit_oracle.rank_deficient_regression",
            f"regression for {subject} is rank deficient",
        )
    coefficients, _, _, _ = linalg.lstsq(
        centered,
        centered_outcome,
        cond=tolerance,
        lapack_driver="gelsy",
        check_finite=True,
    )
    if not np.isfinite(coefficients).all():
        raise _fail(
            "model_fit_oracle.non_finite_regression",
            f"regression for {subject} produced non-finite coefficients",
        )
    return np.asarray(coefficients, dtype=float)


def _degrees_of_freedom(model: ModelSpec, saturated: bool) -> float:
    indicator_count = len(model.indicator_order)
    construct_count = len(model.constructs)
    indicator_moments = indicator_count * (indicator_count - 1) // 2
    if saturated:
        structural_parameters = construct_count * (construct_count - 1) // 2
    else:
        endogenous = {target for _, target in model.paths}
        exogenous_count = sum(
            construct.construct_id not in endogenous for construct in model.constructs
        )
        structural_parameters = exogenous_count * (exogenous_count - 1) // 2 + len(
            model.paths
        )
    measurement_parameters = 0
    for construct in model.constructs:
        count = len(construct.indicators)
        if construct.mode == "reflective":
            measurement_parameters += count
        else:
            measurement_parameters += max(0, count - 1) + count * max(0, count - 1) // 2
    free_parameters = structural_parameters + measurement_parameters
    if free_parameters > indicator_moments:
        raise _fail(
            "model_fit_oracle.nonpositive_degrees_of_freedom",
            "the standardized correlation model has negative degrees of freedom",
        )
    return float(indicator_moments - free_parameters)


def _ml_discrepancy(observed: np.ndarray, implied: np.ndarray) -> float:
    observed_checked = _positive_definite_correlation(observed, "observed_correlation")
    implied_checked = _positive_definite_correlation(implied, "implied_correlation")
    sign_observed, logdet_observed = np.linalg.slogdet(observed_checked)
    sign_implied, logdet_implied = np.linalg.slogdet(implied_checked)
    if sign_observed <= 0 or sign_implied <= 0:
        raise _fail(
            "model_fit_oracle.invalid_ml_determinant",
            "ML discrepancy requires positive determinants",
        )
    value = float(
        np.trace(linalg.solve(implied_checked, observed_checked, assume_a="pos"))
        - logdet_observed
        + logdet_implied
        - observed.shape[0]
    )
    if not math.isfinite(value) or value < -1e-10:
        raise _fail(
            "model_fit_oracle.invalid_ml_discrepancy",
            "ML discrepancy is negative beyond tolerance or non-finite",
        )
    return max(0.0, value)


def _geodesic_discrepancy(observed: np.ndarray, implied: np.ndarray) -> float:
    observed_checked = _positive_definite_correlation(observed, "observed_correlation")
    implied_checked = _positive_definite_correlation(implied, "implied_correlation")
    eigenvalues = linalg.eigh(
        implied_checked,
        observed_checked,
        eigvals_only=True,
        check_finite=True,
    )
    if np.any(~np.isfinite(eigenvalues)) or np.any(eigenvalues <= 0):
        raise _fail(
            "model_fit_oracle.invalid_geodesic_eigenvalue",
            "geodesic discrepancy requires finite positive generalized eigenvalues",
        )
    return float(0.5 * np.sum(np.log(eigenvalues) ** 2))


def _fit_statistics(
    observed: np.ndarray,
    implied: np.ndarray,
    sample_size: int,
    degrees_of_freedom: float,
    null_model_chi_square: float,
) -> FitStatistics:
    residual = observed - implied
    d_uls = float(np.sum(np.tril(residual) ** 2))
    srmr = math.sqrt(d_uls / (observed.shape[0] * (observed.shape[0] + 1) / 2))
    chi_square = (sample_size - 1) * _ml_discrepancy(observed, implied)
    if null_model_chi_square <= np.finfo(float).eps:
        raise _fail(
            "model_fit_oracle.null_chi_square_zero",
            "NFI is unavailable because the null-model chi-square is zero",
        )
    return FitStatistics(
        srmr=srmr,
        d_uls=d_uls,
        d_g=_geodesic_discrepancy(observed, implied),
        chi_square=chi_square,
        degrees_of_freedom=degrees_of_freedom,
        nfi=1 - chi_square / null_model_chi_square,
    )


def _construct_blocks(model: ModelSpec) -> tuple[slice, ...]:
    blocks: list[slice] = []
    start = 0
    for construct in model.constructs:
        stop = start + len(construct.indicators)
        blocks.append(slice(start, stop))
        start = stop
    return tuple(blocks)


def _score_matrix(
    standardized: np.ndarray,
    blocks: tuple[slice, ...],
    weights: Sequence[np.ndarray],
    model: ModelSpec,
) -> np.ndarray:
    columns = []
    for construct, block, coefficients in zip(model.constructs, blocks, weights):
        columns.append(
            _standardize_vector(
                standardized[:, block] @ coefficients,
                f"construct score {construct.construct_id}",
            )
        )
    return np.column_stack(columns)


def _inner_proxies(
    scores: np.ndarray,
    model: ModelSpec,
    construct_index: Mapping[str, int],
) -> np.ndarray:
    incoming: list[list[int]] = [[] for _ in model.constructs]
    outgoing: list[list[int]] = [[] for _ in model.constructs]
    for source, target in model.paths:
        source_index = construct_index[source]
        target_index = construct_index[target]
        incoming[target_index].append(source_index)
        outgoing[source_index].append(target_index)
    proxies: list[np.ndarray] = []
    for construct_position, construct in enumerate(model.constructs):
        if not incoming[construct_position] and not outgoing[construct_position]:
            if len(model.constructs) == 1:
                proxies.append(scores[:, construct_position].copy())
                continue
            raise _fail(
                "model_fit_oracle.isolated_construct",
                f"construct {construct.construct_id!r} is isolated",
            )
        proxy = np.zeros(scores.shape[0], dtype=float)
        if incoming[construct_position]:
            predictors = scores[:, incoming[construct_position]]
            coefficients = _ols(predictors, scores[:, construct_position], construct.construct_id)
            proxy += predictors @ coefficients
        for target in outgoing[construct_position]:
            association = float(
                np.corrcoef(scores[:, construct_position], scores[:, target])[0, 1]
            )
            proxy += scores[:, target] * association
        proxies.append(
            _standardize_vector(proxy, f"inner proxy {construct.construct_id}")
        )
    return np.column_stack(proxies)


def _estimated_construct_correlation(
    model: ModelSpec,
    construct_index: Mapping[str, int],
    topological_order: Sequence[int],
    empirical: np.ndarray,
    path_coefficients: Mapping[tuple[str, str], float],
) -> np.ndarray:
    count = len(model.constructs)
    implied = np.eye(count, dtype=float)
    endogenous = {target for _, target in model.paths}
    exogenous = [
        position
        for position, construct in enumerate(model.constructs)
        if construct.construct_id not in endogenous
    ]
    for offset, left in enumerate(exogenous):
        for right in exogenous[offset + 1 :]:
            implied[left, right] = implied[right, left] = empirical[left, right]
    processed: list[int] = []
    for target in topological_order:
        target_id = model.constructs[target].construct_id
        predecessors = [source for source, candidate in model.paths if candidate == target_id]
        if predecessors:
            for other in processed:
                value = sum(
                    path_coefficients[(source, target_id)]
                    * implied[construct_index[source], other]
                    for source in predecessors
                )
                implied[target, other] = implied[other, target] = value
        processed.append(target)
    return implied


def _implied_indicator_correlation(
    model: ModelSpec,
    observed: np.ndarray,
    construct_correlation: np.ndarray,
    loadings: Sequence[np.ndarray],
) -> np.ndarray:
    indicator_count = len(model.indicator_order)
    implied = np.eye(indicator_count, dtype=float)
    indicator_meta: list[tuple[int, int, ConstructSpec]] = []
    for construct_position, construct in enumerate(model.constructs):
        for within_position, _ in enumerate(construct.indicators):
            indicator_meta.append((construct_position, within_position, construct))
    for left in range(indicator_count):
        left_construct, left_within, left_spec = indicator_meta[left]
        for right in range(left + 1, indicator_count):
            right_construct, right_within, right_spec = indicator_meta[right]
            if left_construct == right_construct and left_spec.mode == "formative":
                value = observed[left, right]
            else:
                value = (
                    loadings[left_construct][left_within]
                    * construct_correlation[left_construct, right_construct]
                    * loadings[right_construct][right_within]
                )
            implied[left, right] = implied[right, left] = value
    return implied


def fit_pls_model(
    observations: object,
    model: ModelSpec,
    *,
    tolerance: float = 1e-7,
    max_iterations: int = 3000,
) -> PlsPointFit:
    """Fully fit the bounded independent recursive PLS-PM model."""

    if not math.isfinite(tolerance) or tolerance <= 0 or max_iterations < 1:
        raise _fail(
            "model_fit_oracle.invalid_iteration_settings",
            "tolerance must be positive and max_iterations must be at least one",
        )
    construct_index, topological_order = _validate_model(model)
    raw = _as_raw_matrix(observations, len(model.indicator_order))
    standardized = _standardize(raw)
    blocks = _construct_blocks(model)
    weights = [
        _normalize_weights(
            standardized[:, block],
            np.ones(len(construct.indicators), dtype=float),
            construct.construct_id,
        )
        for construct, block in zip(model.constructs, blocks)
    ]
    used_iterations: int | None = None
    for iteration in range(1, max_iterations + 1):
        scores = _score_matrix(standardized, blocks, weights, model)
        proxies = _inner_proxies(scores, model, construct_index)
        updated: list[np.ndarray] = []
        for position, (construct, block) in enumerate(zip(model.constructs, blocks)):
            block_matrix = standardized[:, block]
            if construct.mode == "reflective":
                candidate = block_matrix.T @ proxies[:, position] / (raw.shape[0] - 1)
            else:
                candidate = _ols(block_matrix, proxies[:, position], construct.construct_id)
            updated.append(
                _normalize_weights(block_matrix, candidate, construct.construct_id)
            )
        change = max(
            float(np.max(np.abs(previous - current)))
            for previous, current in zip(weights, updated)
        )
        weights = updated
        if change <= tolerance:
            used_iterations = iteration
            break
    if used_iterations is None:
        raise _fail(
            "model_fit_oracle.nonconvergence",
            f"PLS weights did not converge within {max_iterations} iterations",
        )

    scores = _score_matrix(standardized, blocks, weights, model)
    loadings = tuple(
        standardized[:, block].T @ scores[:, position] / (raw.shape[0] - 1)
        for position, block in enumerate(blocks)
    )
    path_coefficients: dict[tuple[str, str], float] = {}
    path_rows: list[tuple[str, str, float]] = []
    for target_position, target_construct in enumerate(model.constructs):
        predecessors = [
            source for source, target in model.paths if target == target_construct.construct_id
        ]
        if not predecessors:
            continue
        predecessor_positions = [construct_index[source] for source in predecessors]
        coefficients = _ols(
            scores[:, predecessor_positions],
            scores[:, target_position],
            target_construct.construct_id,
        )
        for source, coefficient in zip(predecessors, coefficients):
            value = float(coefficient)
            path_coefficients[(source, target_construct.construct_id)] = value
            path_rows.append((source, target_construct.construct_id, value))

    observed = _positive_definite_correlation(
        _correlation(standardized), "observed_correlation"
    )
    empirical_construct = _positive_definite_correlation(
        _correlation(scores), "construct_correlation"
    )
    estimated_construct = _estimated_construct_correlation(
        model,
        construct_index,
        topological_order,
        empirical_construct,
        path_coefficients,
    )
    saturated_implied = _positive_definite_correlation(
        _implied_indicator_correlation(
            model, observed, empirical_construct, loadings
        ),
        "saturated_implied_correlation",
    )
    estimated_implied = _positive_definite_correlation(
        _implied_indicator_correlation(
            model, observed, estimated_construct, loadings
        ),
        "estimated_implied_correlation",
    )
    null_model_chi_square = (raw.shape[0] - 1) * _ml_discrepancy(
        observed, np.eye(observed.shape[0])
    )
    saturated = _fit_statistics(
        observed,
        saturated_implied,
        raw.shape[0],
        _degrees_of_freedom(model, True),
        null_model_chi_square,
    )
    estimated = _fit_statistics(
        observed,
        estimated_implied,
        raw.shape[0],
        _degrees_of_freedom(model, False),
        null_model_chi_square,
    )
    return PlsPointFit(
        indicator_order=model.indicator_order,
        analytical_sample_size=raw.shape[0],
        iterations=used_iterations,
        outer_weights=tuple(tuple(float(value) for value in row) for row in weights),
        outer_loadings=tuple(tuple(float(value) for value in row) for row in loadings),
        path_coefficients=tuple(path_rows),
        observed_correlation=observed,
        saturated_implied_correlation=saturated_implied,
        estimated_implied_correlation=estimated_implied,
        saturated=saturated,
        estimated=estimated,
        null_model_chi_square=null_model_chi_square,
    )


def indexed_case_indices(
    row_count: int,
    master_seed: int,
    variant: str,
    replicate_index: int,
) -> np.ndarray:
    """Return one domain-separated deterministic case draw."""

    if row_count < 1 or not 0 <= master_seed < 2**64 or replicate_index < 0:
        raise _fail(
            "model_fit_oracle.invalid_index_identity",
            "row_count, 64-bit seed, and replicate index must be valid",
        )
    if variant not in VARIANTS:
        raise _fail("model_fit_oracle.invalid_variant", f"unknown variant {variant!r}")
    identity = (
        f"{ORACLE_VERSION}|{INDEX_PLAN_VERSION}|{master_seed}|{variant}|{replicate_index}"
    ).encode("utf-8")
    entropy = np.frombuffer(hashlib.sha256(identity).digest(), dtype="<u4").tolist()
    generator = np.random.Generator(np.random.PCG64(np.random.SeedSequence(entropy)))
    return generator.integers(0, row_count, size=row_count, dtype=np.uint64)


def build_index_plan(
    row_count: int,
    requested_replicates: int,
    master_seed: int,
    variant: str,
) -> tuple[np.ndarray, ...]:
    if requested_replicates < 2:
        raise _fail(
            "model_fit_oracle.insufficient_reference_replicates",
            "the validation oracle needs at least two requested replicates",
        )
    return tuple(
        indexed_case_indices(row_count, master_seed, variant, replicate_index)
        for replicate_index in range(requested_replicates)
    )


def sample_index_digest(indices: object) -> str:
    values = np.asarray(indices, dtype=np.uint64)
    if values.ndim != 1:
        raise _fail(
            "model_fit_oracle.invalid_sample_indices",
            "sample indices must be one dimensional",
        )
    header = len(values).to_bytes(8, byteorder="little", signed=False)
    payload = values.astype("<u8", copy=False).tobytes(order="C")
    return hashlib.sha256(header + payload).hexdigest()


def _usable_index_digest(indices: Sequence[int]) -> str:
    values = np.asarray(indices, dtype="<u8")
    header = len(values).to_bytes(8, byteorder="little", signed=False)
    return hashlib.sha256(header + values.tobytes(order="C")).hexdigest()


def type7_quantile(values: object, probability: float) -> float:
    vector = np.sort(np.asarray(values, dtype=float))
    if vector.ndim != 1 or vector.size == 0 or not np.isfinite(vector).all():
        raise _fail(
            "model_fit_oracle.invalid_quantile_values",
            "Type-7 quantiles require a nonempty finite vector",
        )
    if not 0 <= probability <= 1:
        raise _fail(
            "model_fit_oracle.invalid_quantile_probability",
            "quantile probability must be between zero and one",
        )
    position = (vector.size - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return float(vector[lower])
    return float(vector[lower] + (position - lower) * (vector[upper] - vector[lower]))


def _criterion_summary(
    criterion: str,
    original: float,
    ledger: Sequence[ReplicateLedgerEntry],
) -> CriterionSummary:
    values: list[float] = []
    usable_indices: list[int] = []
    for entry in ledger:
        value = getattr(entry, criterion)
        if value is not None and math.isfinite(value):
            values.append(value)
            usable_indices.append(entry.replicate_index)
    requested = len(ledger)
    minimum = max(2, math.ceil(MINIMUM_USABLE_FRACTION * requested))
    exceed_or_equal = sum(value >= original for value in values)
    digest = _usable_index_digest(usable_indices)
    if len(values) < minimum:
        return CriterionSummary(
            criterion=criterion,
            status="unavailable",
            original=original,
            requested_replicates=requested,
            minimum_usable_replicates=minimum,
            usable_replicates=len(values),
            failed_replicates=requested - len(values),
            usable_index_digest=digest,
            replicate_min=None,
            replicate_max=None,
            upper_95=None,
            upper_99=None,
            not_rejected_95=None,
            not_rejected_99=None,
            exceed_or_equal_count=exceed_or_equal,
            empirical_upper_tail_probability=None,
            unavailable_reason_code="model_fit_exact.insufficient_usable_replicates",
        )
    upper_95 = type7_quantile(values, 0.95)
    upper_99 = type7_quantile(values, 0.99)
    return CriterionSummary(
        criterion=criterion,
        status="available",
        original=original,
        requested_replicates=requested,
        minimum_usable_replicates=minimum,
        usable_replicates=len(values),
        failed_replicates=requested - len(values),
        usable_index_digest=digest,
        replicate_min=min(values),
        replicate_max=max(values),
        upper_95=upper_95,
        upper_99=upper_99,
        not_rejected_95=original <= upper_95,
        not_rejected_99=original <= upper_99,
        exceed_or_equal_count=exceed_or_equal,
        empirical_upper_tail_probability=exceed_or_equal / len(values),
        unavailable_reason_code=None,
    )


def _variant_status(criteria: Sequence[CriterionSummary]) -> str:
    available = sum(summary.status == "available" for summary in criteria)
    if available == len(criteria):
        return "available"
    if available:
        return "partial"
    return "unavailable"


def _validate_explicit_plan(
    plan: Sequence[object],
    requested_replicates: int,
    row_count: int,
    variant: str,
) -> tuple[np.ndarray, ...]:
    if len(plan) != requested_replicates:
        raise _fail(
            "model_fit_oracle.index_plan_length_mismatch",
            f"{variant} plan must contain one draw per requested replicate",
        )
    checked = []
    for replicate_index, value in enumerate(plan):
        indices = np.asarray(value, dtype=np.uint64)
        if indices.ndim != 1 or len(indices) != row_count or np.any(indices >= row_count):
            raise _fail(
                "model_fit_oracle.invalid_sample_indices",
                f"invalid {variant} indices at replicate {replicate_index}",
            )
        checked.append(indices)
    return tuple(checked)


def run_adapted_bollen_stine(
    observations: object,
    model: ModelSpec,
    *,
    requested_replicates: int,
    seed: int,
    tolerance: float = 1e-7,
    max_iterations: int = 3000,
    index_plans: Mapping[str, Sequence[object]] | None = None,
) -> ExactFitResult:
    """Run two independent null transforms and full-refit fixed ledgers."""

    if requested_replicates < 2:
        raise _fail(
            "model_fit_oracle.insufficient_reference_replicates",
            "at least two requested reference replicates are required",
        )
    raw = _as_raw_matrix(observations, len(model.indicator_order))
    point_fit = fit_pls_model(
        raw,
        model,
        tolerance=tolerance,
        max_iterations=max_iterations,
    )
    variant_results: dict[str, VariantExactFit] = {}
    for variant in VARIANTS:
        target = (
            point_fit.saturated_implied_correlation
            if variant == "saturated"
            else point_fit.estimated_implied_correlation
        )
        transformed = null_transform(raw, target)
        recovered = _correlation(transformed)
        if index_plans is None:
            plan = build_index_plan(raw.shape[0], requested_replicates, seed, variant)
        else:
            if set(index_plans) != set(VARIANTS):
                raise _fail(
                    "model_fit_oracle.incomplete_index_plans",
                    "explicit index plans must contain saturated and estimated variants",
                )
            plan = _validate_explicit_plan(
                index_plans[variant], requested_replicates, raw.shape[0], variant
            )
        ledger: list[ReplicateLedgerEntry] = []
        for replicate_index, indices in enumerate(plan):
            digest = sample_index_digest(indices)
            try:
                replicate = fit_pls_model(
                    transformed[np.asarray(indices, dtype=np.intp), :],
                    model,
                    tolerance=tolerance,
                    max_iterations=max_iterations,
                )
                measures = replicate.saturated if variant == "saturated" else replicate.estimated
                ledger.append(
                    ReplicateLedgerEntry(
                        variant=variant,
                        replicate_index=replicate_index,
                        index_plan_version=INDEX_PLAN_VERSION,
                        sample_index_digest=digest,
                        status="success",
                        srmr=measures.srmr,
                        d_uls=measures.d_uls,
                        d_g=measures.d_g,
                        failure_code=None,
                        failure_message=None,
                    )
                )
            except ModelFitOracleError as error:
                ledger.append(
                    ReplicateLedgerEntry(
                        variant=variant,
                        replicate_index=replicate_index,
                        index_plan_version=INDEX_PLAN_VERSION,
                        sample_index_digest=digest,
                        status="failed",
                        srmr=None,
                        d_uls=None,
                        d_g=None,
                        failure_code=error.code,
                        failure_message=error.message,
                    )
                )
        original_measures = (
            point_fit.saturated if variant == "saturated" else point_fit.estimated
        )
        summaries = tuple(
            _criterion_summary(criterion, getattr(original_measures, criterion), ledger)
            for criterion in CRITERIA
        )
        variant_results[variant] = VariantExactFit(
            variant=variant,
            status=_variant_status(summaries),
            target_correlation=target,
            recovered_transformed_correlation=recovered,
            ledger=tuple(ledger),
            criteria=summaries,
        )
    return ExactFitResult(
        oracle_version=ORACLE_VERSION,
        index_plan_version=INDEX_PLAN_VERSION,
        sample_digest_version=SAMPLE_DIGEST_VERSION,
        seed=seed,
        requested_replicates=requested_replicates,
        point_fit=point_fit,
        saturated=variant_results["saturated"],
        estimated=variant_results["estimated"],
    )


def model_from_recipe_document(document: Mapping[str, object]) -> ModelSpec:
    model_value = document.get("model")
    if not isinstance(model_value, Mapping):
        raise _fail("model_fit_oracle.invalid_recipe", "recipe model must be an object")
    constructs_value = model_value.get("constructs")
    paths_value = model_value.get("paths", [])
    if not isinstance(constructs_value, list) or not isinstance(paths_value, list):
        raise _fail(
            "model_fit_oracle.invalid_recipe",
            "recipe constructs and paths must be arrays",
        )
    constructs = []
    for value in constructs_value:
        if not isinstance(value, Mapping) or not isinstance(value.get("indicators"), list):
            raise _fail(
                "model_fit_oracle.invalid_recipe_construct",
                "every recipe construct must contain an indicator array",
            )
        constructs.append(
            ConstructSpec(
                construct_id=str(value.get("id", "")),
                indicators=tuple(str(indicator) for indicator in value["indicators"]),
                mode=str(value.get("mode", "reflective")),
            )
        )
    paths = []
    for value in paths_value:
        if not isinstance(value, Mapping):
            raise _fail(
                "model_fit_oracle.invalid_recipe_path",
                "every recipe path must be an object",
            )
        paths.append((str(value.get("source", "")), str(value.get("target", ""))))
    return ModelSpec(constructs=tuple(constructs), paths=tuple(paths))


def read_csv_matrix(path: Path, indicator_order: Sequence[str]) -> np.ndarray:
    with path.open("r", encoding="utf-8", newline="") as handle:
        rows = list(csv.DictReader(handle))
    try:
        return np.asarray(
            [[float(row[indicator]) for indicator in indicator_order] for row in rows],
            dtype=float,
        )
    except (KeyError, TypeError, ValueError) as error:
        raise _fail(
            "model_fit_oracle.invalid_csv_fixture",
            f"cannot read the requested finite numeric columns from {path}",
        ) from error


def _product_value(value: object) -> float:
    if not isinstance(value, Mapping) or value.get("status") != "available":
        raise _fail(
            "model_fit_oracle.product_fixture_value_unavailable",
            "the frozen product comparator must expose an available value",
        )
    return float(value["value"])


def compare_frozen_product_point_fit(
    reference: PlsPointFit,
    model: ModelSpec,
    product_document: Mapping[str, object],
    *,
    tolerance: float = 1e-10,
) -> dict[str, object]:
    """Compare to a frozen product artifact without treating it as the oracle."""

    try:
        payload = product_document["payload"]
        assessment = payload["assessment"]
        fit = assessment["model_fit"]
        estimation = payload["estimation"]
    except (KeyError, TypeError) as error:
        raise _fail(
            "model_fit_oracle.invalid_product_fixture",
            "frozen product fixture does not contain a PLS model-fit payload",
        ) from error
    if fit["method_version"] != "pls_model_fit_v2":
        raise _fail(
            "model_fit_oracle.product_fixture_identity_mismatch",
            "frozen product fixture is not pls_model_fit_v2",
        )
    comparisons: list[dict[str, object]] = []

    def add(label: str, actual: float, expected: float) -> None:
        difference = actual - expected
        comparisons.append(
            {
                "quantity": label,
                "independent_oracle": actual,
                "frozen_product": expected,
                "difference": difference,
                "absolute_difference": abs(difference),
                "passed": abs(difference) <= tolerance,
            }
        )

    if tuple(fit["indicator_order"]) != reference.indicator_order:
        raise _fail(
            "model_fit_oracle.product_indicator_order_mismatch",
            "frozen product and independent oracle indicator orders differ",
        )
    for matrix_name in (
        "observed_correlation",
        "saturated_implied_correlation",
        "estimated_implied_correlation",
    ):
        product_matrix = np.asarray(fit[matrix_name], dtype=float)
        reference_matrix = getattr(reference, matrix_name)
        add(
            f"{matrix_name}.maximum_absolute_cell_difference",
            float(np.max(np.abs(reference_matrix - product_matrix))),
            0.0,
        )
    for variant in VARIANTS:
        product_variant = fit[variant]
        reference_variant = getattr(reference, variant)
        add(f"{variant}.srmr", reference_variant.srmr, float(product_variant["srmr"]))
        add(
            f"{variant}.d_uls", reference_variant.d_uls, float(product_variant["d_uls"])
        )
        add(f"{variant}.d_g", reference_variant.d_g, _product_value(product_variant["d_g"]))
        add(
            f"{variant}.chi_square",
            reference_variant.chi_square,
            _product_value(product_variant["chi_square"]),
        )
        add(
            f"{variant}.degrees_of_freedom",
            reference_variant.degrees_of_freedom,
            _product_value(product_variant["degrees_of_freedom"]),
        )
        add(f"{variant}.nfi", reference_variant.nfi, _product_value(product_variant["nfi"]))
    product_loadings = {
        (str(row["construct"]), str(row["indicator"])): float(row["loading"])
        for row in estimation["outer_estimates"]
    }
    if model.indicator_order != reference.indicator_order:
        raise _fail(
            "model_fit_oracle.reference_model_identity_mismatch",
            "the comparison model and independent point result have different indicators",
        )
    for construct, loadings in zip(model.constructs, reference.outer_loadings):
        for indicator, loading in zip(construct.indicators, loadings):
            add(
                f"loading.{construct.construct_id}.{indicator}",
                loading,
                product_loadings[(construct.construct_id, indicator)],
            )
    product_paths = {
        (str(row["source"]), str(row["target"])): float(row["coefficient"])
        for row in estimation["paths"]
    }
    for source, target, coefficient in reference.path_coefficients:
        add(f"path.{source}.{target}", coefficient, product_paths[(source, target)])
    maximum = max(row["absolute_difference"] for row in comparisons)
    return {
        "passed": all(bool(row["passed"]) for row in comparisons),
        "tolerance": tolerance,
        "maximum_absolute_difference": maximum,
        "comparisons": comparisons,
        "role": "behavioral_comparator_only_not_numerical_oracle",
    }


def _point_fit_to_dict(point: PlsPointFit) -> dict[str, object]:
    return {
        "indicator_order": list(point.indicator_order),
        "analytical_sample_size": point.analytical_sample_size,
        "iterations": point.iterations,
        "outer_weights": [list(row) for row in point.outer_weights],
        "outer_loadings": [list(row) for row in point.outer_loadings],
        "path_coefficients": [
            {"source": source, "target": target, "coefficient": value}
            for source, target, value in point.path_coefficients
        ],
        "observed_correlation": point.observed_correlation.tolist(),
        "saturated_implied_correlation": point.saturated_implied_correlation.tolist(),
        "estimated_implied_correlation": point.estimated_implied_correlation.tolist(),
        "saturated": asdict(point.saturated),
        "estimated": asdict(point.estimated),
        "null_model_chi_square": point.null_model_chi_square,
    }


def _variant_to_dict(variant: VariantExactFit, *, include_ledger: bool) -> dict[str, object]:
    value: dict[str, object] = {
        "variant": variant.variant,
        "status": variant.status,
        "target_correlation": variant.target_correlation.tolist(),
        "recovered_transformed_correlation": variant.recovered_transformed_correlation.tolist(),
        "maximum_target_recovery_error": float(
            np.max(
                np.abs(
                    variant.target_correlation - variant.recovered_transformed_correlation
                )
            )
        ),
        "criteria": [asdict(summary) for summary in variant.criteria],
        "failed_replicates": sum(entry.status == "failed" for entry in variant.ledger),
        "sample_index_digests": [entry.sample_index_digest for entry in variant.ledger],
    }
    if include_ledger:
        value["ledger"] = [asdict(entry) for entry in variant.ledger]
    return value


def _synthetic_fixture() -> tuple[np.ndarray, ModelSpec]:
    generator = np.random.Generator(np.random.PCG64(2026081407))
    row_count = 120
    x = generator.normal(size=row_count)
    mediator = 0.58 * x + generator.normal(scale=0.82, size=row_count)
    outcome = 0.35 * x + 0.48 * mediator + generator.normal(scale=0.72, size=row_count)
    observations = np.column_stack(
        [
            0.86 * x + generator.normal(scale=0.42, size=row_count),
            0.78 * x + generator.normal(scale=0.50, size=row_count),
            0.84 * mediator + generator.normal(scale=0.44, size=row_count),
            0.76 * mediator + generator.normal(scale=0.52, size=row_count),
            0.87 * outcome + generator.normal(scale=0.40, size=row_count),
            0.75 * outcome + generator.normal(scale=0.55, size=row_count),
        ]
    )
    model = ModelSpec(
        constructs=(
            ConstructSpec("x", ("x1", "x2")),
            ConstructSpec("m", ("m1", "m2")),
            ConstructSpec("y", ("y1", "y2")),
        ),
        paths=(("x", "m"), ("m", "y")),
    )
    return observations, model


def _max_point_scalar_difference(left: PlsPointFit, right: PlsPointFit) -> float:
    differences = []
    for variant in VARIANTS:
        left_values = getattr(left, variant)
        right_values = getattr(right, variant)
        for field in ("srmr", "d_uls", "d_g", "chi_square", "degrees_of_freedom", "nfi"):
            differences.append(abs(getattr(left_values, field) - getattr(right_values, field)))
    differences.append(abs(left.null_model_chi_square - right.null_model_chi_square))
    return max(differences)


def _summary_vector(result: ExactFitResult) -> np.ndarray:
    values = []
    for variant in (result.saturated, result.estimated):
        for summary in variant.criteria:
            values.extend(
                [
                    summary.replicate_min,
                    summary.replicate_max,
                    summary.upper_95,
                    summary.upper_99,
                    summary.empirical_upper_tail_probability,
                ]
            )
    if any(value is None for value in values):
        raise _fail(
            "model_fit_oracle.summary_unavailable",
            "metamorphic witness requires available exact-fit summaries",
        )
    return np.asarray(values, dtype=float)


def _source_sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_small_qualification_report(repository_root: Path, draws: int = 24) -> dict[str, object]:
    """Build a deterministic, explicitly non-promotional work report."""

    recipe_path = repository_root / "validation" / "fixtures" / "simple_reflective.recipe.json"
    data_path = repository_root / "validation" / "fixtures" / "simple_reflective.csv"
    product_path = repository_root / "validation" / "results" / "pls_quickpls_path_mode_a.json"
    recipe_document = json.loads(recipe_path.read_text(encoding="utf-8"))
    model = model_from_recipe_document(recipe_document)
    observations = read_csv_matrix(data_path, model.indicator_order)
    point = fit_pls_model(
        observations,
        model,
        tolerance=float(recipe_document["settings"]["tolerance"]),
        max_iterations=int(recipe_document["settings"]["max_iterations"]),
    )
    product_document = json.loads(product_path.read_text(encoding="utf-8"))
    product_comparison = compare_frozen_product_point_fit(point, model, product_document)

    synthetic, synthetic_model = _synthetic_fixture()
    exact = run_adapted_bollen_stine(
        synthetic,
        synthetic_model,
        requested_replicates=draws,
        seed=2026081408,
        tolerance=1e-9,
    )
    row_permutation = np.random.default_rng(2026081409).permutation(synthetic.shape[0])
    row_point = fit_pls_model(synthetic[row_permutation], synthetic_model, tolerance=1e-9)
    row_point_difference = _max_point_scalar_difference(exact.point_fit, row_point)

    plans = {
        variant: build_index_plan(synthetic.shape[0], draws, 2026081410, variant)
        for variant in VARIANTS
    }
    baseline_plan_result = run_adapted_bollen_stine(
        synthetic,
        synthetic_model,
        requested_replicates=draws,
        seed=2026081410,
        tolerance=1e-9,
        index_plans=plans,
    )
    inverse_row_permutation = np.empty_like(row_permutation)
    inverse_row_permutation[row_permutation] = np.arange(row_permutation.size)
    mapped_plans = {
        variant: tuple(inverse_row_permutation[np.asarray(indices, dtype=np.intp)] for indices in plan)
        for variant, plan in plans.items()
    }
    row_exact = run_adapted_bollen_stine(
        synthetic[row_permutation],
        synthetic_model,
        requested_replicates=draws,
        seed=2026081410,
        tolerance=1e-9,
        index_plans=mapped_plans,
    )
    row_exact_difference = float(
        np.max(np.abs(_summary_vector(baseline_plan_result) - _summary_vector(row_exact)))
    )

    column_order = np.array([1, 0, 3, 2, 5, 4])
    column_model = ModelSpec(
        constructs=tuple(
            ConstructSpec(
                construct.construct_id,
                tuple(reversed(construct.indicators)),
                construct.mode,
            )
            for construct in synthetic_model.constructs
        ),
        paths=synthetic_model.paths,
    )
    column_point = fit_pls_model(synthetic[:, column_order], column_model, tolerance=1e-9)
    column_point_difference = _max_point_scalar_difference(exact.point_fit, column_point)
    column_exact = run_adapted_bollen_stine(
        synthetic[:, column_order],
        column_model,
        requested_replicates=draws,
        seed=2026081410,
        tolerance=1e-9,
        index_plans=plans,
    )
    column_exact_difference = float(
        np.max(
            np.abs(
                _summary_vector(baseline_plan_result) - _summary_vector(column_exact)
            )
        )
    )

    forced_failure_plans = {
        variant: list(build_index_plan(synthetic.shape[0], 10, 2026081411, variant))
        for variant in VARIANTS
    }
    for variant in VARIANTS:
        forced_failure_plans[variant][0] = np.zeros(synthetic.shape[0], dtype=np.uint64)
    failure_witness = run_adapted_bollen_stine(
        synthetic,
        synthetic_model,
        requested_replicates=10,
        seed=2026081411,
        tolerance=1e-9,
        index_plans=forced_failure_plans,
    )
    typed_failures = {}
    for label, action in {
        "non_positive_definite_target": lambda: null_transform(
            synthetic[:, :2], [[1.0, 1.0], [1.0, 1.0]]
        ),
        "singular_observed_matrix": lambda: fit_pls_model(
            np.column_stack([synthetic[:, 0], synthetic[:, 0], synthetic[:, 2], synthetic[:, 3]]),
            ModelSpec(
                constructs=(
                    ConstructSpec("x", ("x1", "x2")),
                    ConstructSpec("m", ("m1", "m2")),
                ),
                paths=(("x", "m"),),
            ),
        ),
    }.items():
        try:
            action()
        except ModelFitOracleError as error:
            typed_failures[label] = {"code": error.code, "message": error.message}
        else:
            raise _fail(
                "model_fit_oracle.expected_failure_missing",
                f"{label} did not fail as required",
            )

    separate_distributions = (
        [entry.sample_index_digest for entry in exact.saturated.ledger]
        != [entry.sample_index_digest for entry in exact.estimated.ledger]
        and not np.allclose(
            [entry.d_uls for entry in exact.saturated.ledger],
            [entry.d_uls for entry in exact.estimated.ledger],
            rtol=0,
            atol=1e-15,
        )
    )
    deterministic_rerun = run_adapted_bollen_stine(
        synthetic,
        synthetic_model,
        requested_replicates=draws,
        seed=2026081408,
        tolerance=1e-9,
    )
    deterministic_identity = (
        [asdict(entry) for entry in exact.saturated.ledger]
        == [asdict(entry) for entry in deterministic_rerun.saturated.ledger]
        and [asdict(entry) for entry in exact.estimated.ledger]
        == [asdict(entry) for entry in deterministic_rerun.estimated.ledger]
    )
    failure_counts = {
        variant: sum(entry.status == "failed" for entry in getattr(failure_witness, variant).ledger)
        for variant in VARIANTS
    }
    passed = (
        bool(product_comparison["passed"])
        and exact.saturated.status == "available"
        and exact.estimated.status == "available"
        and separate_distributions
        and deterministic_identity
        and row_point_difference <= 1e-10
        and row_exact_difference <= 1e-10
        and column_point_difference <= 1e-10
        and column_exact_difference <= 1e-10
        and failure_counts == {"saturated": 1, "estimated": 1}
        and all(
            summary.usable_replicates == 9
            for variant in (failure_witness.saturated, failure_witness.estimated)
            for summary in variant.criteria
        )
    )
    this_path = Path(__file__).resolve()
    return {
        "schema_version": 1,
        "kind": "pls_model_fit_full_refit_oracle_work_report_v1",
        "passed": passed,
        "qualification_ready": False,
        "promotion_requested": False,
        "reference": {
            "oracle_version": ORACLE_VERSION,
            "implementation": "transparent_numpy_scipy_recursive_pls_pm_full_refit",
            "runtime_policy": "development_validation_only",
            "python": platform.python_version(),
            "numpy": np.__version__,
            "scipy": scipy.__version__,
            "source": this_path.relative_to(repository_root).as_posix(),
            "source_sha256": _source_sha256(this_path),
            "independence_statement": (
                "The oracle imports no QuickPLS crate, binary, Python binding, or product "
                "function. Frozen product JSON is read only after independent estimation "
                "and is used as a behavioral comparator, never as a numerical oracle."
            ),
            "bounded_scope": [
                "raw finite complete-case observations",
                "recursive PLS-PM",
                "Mode A and Mode B blocks",
                "path weighting",
                "small deterministic qualification work draws",
            ],
        },
        "fixtures": {
            "recipe": recipe_path.relative_to(repository_root).as_posix(),
            "recipe_sha256": _source_sha256(recipe_path),
            "data": data_path.relative_to(repository_root).as_posix(),
            "data_sha256": _source_sha256(data_path),
            "frozen_product": product_path.relative_to(repository_root).as_posix(),
            "frozen_product_sha256": _source_sha256(product_path),
        },
        "frozen_product_comparison": product_comparison,
        "independent_point_fit": _point_fit_to_dict(point),
        "deterministic_exact_fit": {
            "requested_replicates": draws,
            "seed": exact.seed,
            "index_plan_version": exact.index_plan_version,
            "sample_digest_version": exact.sample_digest_version,
            "separate_variant_distributions": separate_distributions,
            "identical_rerun_ledger": deterministic_identity,
            "saturated": _variant_to_dict(exact.saturated, include_ledger=True),
            "estimated": _variant_to_dict(exact.estimated, include_ledger=True),
        },
        "metamorphic_witness": {
            "row_point_maximum_scalar_difference": row_point_difference,
            "row_mapped_plan_exact_summary_maximum_difference": row_exact_difference,
            "column_point_maximum_scalar_difference": column_point_difference,
            "column_exact_summary_maximum_difference": column_exact_difference,
            "tolerance": 1e-10,
        },
        "fixed_failure_ledger_witness": {
            "requested_replicates": 10,
            "expected_failures_per_variant": 1,
            "actual_failures": failure_counts,
            "saturated": _variant_to_dict(failure_witness.saturated, include_ledger=True),
            "estimated": _variant_to_dict(failure_witness.estimated, include_ledger=True),
        },
        "typed_failure_witness": typed_failures,
        "blockers": [
            "oracle.plsc_full_refit_not_implemented",
            "oracle.higher_order_interaction_and_broad_mixed_model_shapes_not_covered",
            "oracle.second_independent_full_pipeline_or_documented_exception_missing",
            "calibration.preregistered_type_i_error_power_coverage_and_failure_rate_not_run",
            "persistence.real_999_5000_10000_draw_archive_and_cross_format_readback_not_run",
            "packaged_windows.installed_portable_accessibility_scaling_cancellation_not_run",
            "performance.maximum_axis_compound_stress_soak_and_leak_not_run",
            "review.independent_scientific_review_not_recorded",
        ],
        "note": (
            "This deterministic work report demonstrates an independent full-refit "
            "PLS-PM path for small fixtures. It is not an immutable QualificationSpec "
            "receipt and cannot change coverage, evidence, surface, or promotion state."
        ),
    }


def main() -> int:
    repository_root = Path(__file__).resolve().parents[1]
    default_output = (
        repository_root
        / "validation"
        / "results"
        / "method_factory"
        / "pls_model_fit_v2"
        / "work"
        / "independent_full_refit_oracle.json"
    )
    parser = argparse.ArgumentParser()
    parser.add_argument("--draws", type=int, default=24)
    parser.add_argument("--output", type=Path, default=default_output)
    args = parser.parse_args()
    report = build_small_qualification_report(repository_root, draws=args.draws)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "output": str(args.output),
                "passed": report["passed"],
                "qualification_ready": report["qualification_ready"],
            },
            sort_keys=True,
        )
    )
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
