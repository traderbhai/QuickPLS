#!/usr/bin/env python3
"""Independent NumPy work oracle for PLS score execution v2.

This validation-only module imports no QuickPLS product code and executes no
QuickPLS binary.  It freezes the bounded scientific contract for standard and
individual estimated-score initialization plus unit/custom fixed scoring.
Passing these checks is work evidence only: it does not qualify a current
product build, attach a receipt, or authorize a Registry/manifest promotion.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
from dataclasses import asdict, dataclass, is_dataclass, replace
from pathlib import Path
from typing import Any, Literal, Sequence

import numpy as np


ORACLE_VERSION = "independent_numpy_pls_score_execution_v2_work_oracle_v1"
METHOD_VERSION = "pls_score_execution_v2"
CONTRACT_VERSION = "pls_score_execution_v2"
INITIALIZATION_CONTRACT_VERSION = "pls_initial_outer_weights_v2"
FIXED_MAX_ITERATIONS = 3_000
FIXED_STOP_CRITERION = 1.0e-7
NUMERIC_EPSILON = 1.0e-14

BlockScoring = Literal["estimated", "unit", "custom"]
EstimatedMode = Literal["mode_a", "mode_b"]
Normalization = Literal["none", "sum_to_one", "unit_variance"]
WeightingScheme = Literal["path", "factor", "pca"]
Preprocessing = Literal["standardized", "mean_centered", "unstandardized"]
InitializationKind = Literal["standard", "individual"]


class ScoreExecutionOracleFailure(ValueError):
    """Typed, deterministic rejection from the independent work oracle."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code


@dataclass(frozen=True)
class OraclePath:
    source: str
    target: str


@dataclass(frozen=True)
class OracleScoreBlock:
    construct_id: str
    indicator_ids: tuple[str, ...]
    scoring: BlockScoring = "estimated"
    mode: EstimatedMode = "mode_a"
    normalization: Normalization = "unit_variance"
    custom_weights: tuple[tuple[str, float], ...] = ()


@dataclass(frozen=True)
class OracleInitialWeight:
    construct_id: str
    indicator_id: str
    value: float


@dataclass(frozen=True)
class OracleInitialization:
    kind: InitializationKind = "standard"
    weights: tuple[OracleInitialWeight, ...] = ()
    contract_version: str = INITIALIZATION_CONTRACT_VERSION


@dataclass(frozen=True)
class OracleExecutionSettings:
    weighting_scheme: WeightingScheme = "path"
    preprocessing: Preprocessing = "standardized"
    max_iterations: int = FIXED_MAX_ITERATIONS
    stop_criterion: float = FIXED_STOP_CRITERION
    bootstrap_samples: int = 0
    studentized_inner_samples: int = 0
    permutation_samples: int = 0
    case_weights_requested: bool = False
    higher_order_requested: bool = False
    interaction_requested: bool = False


@dataclass(frozen=True)
class _ResolvedBlock:
    construct_id: str
    indicator_ids: tuple[str, ...]
    scoring: BlockScoring
    mode: EstimatedMode
    normalization: Normalization
    requested_weights: tuple[float, ...]
    effective_initial_weights: tuple[float, ...]
    initialization_kind: InitializationKind | None


def _canonical_sha256(value: Any) -> str:
    def encode(value_to_encode: Any) -> Any:
        if is_dataclass(value_to_encode):
            return asdict(value_to_encode)
        if isinstance(value_to_encode, np.ndarray):
            return value_to_encode.tolist()
        if isinstance(value_to_encode, np.floating):
            return float(value_to_encode)
        raise TypeError(
            f"unsupported canonical value: {type(value_to_encode).__name__}"
        )

    payload = json.dumps(
        value,
        sort_keys=True,
        separators=(",", ":"),
        ensure_ascii=False,
        allow_nan=False,
        default=encode,
    ).encode("utf-8")
    return hashlib.sha256(payload).hexdigest()


def _sample_sd(values: np.ndarray) -> float:
    return float(np.std(values, ddof=1))


def _standardize_vector(values: np.ndarray, *, subject: str) -> np.ndarray:
    centered = values - float(np.mean(values))
    deviation = _sample_sd(centered)
    if not math.isfinite(deviation) or deviation <= NUMERIC_EPSILON:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_zero_variance_score",
            f"{subject} produces a zero or non-finite sample variance",
        )
    return centered / deviation


def _covariance(left: np.ndarray, right: np.ndarray) -> float:
    if len(left) != len(right) or len(left) < 3:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_invalid_sample",
            "covariance inputs must have the same length and at least three rows",
        )
    left_centered = left - float(np.mean(left))
    right_centered = right - float(np.mean(right))
    return float(np.dot(left_centered, right_centered) / (len(left) - 1))


def _correlation(left: np.ndarray, right: np.ndarray) -> float:
    standardized_left = _standardize_vector(left, subject="correlation left")
    standardized_right = _standardize_vector(right, subject="correlation right")
    return float(np.dot(standardized_left, standardized_right) / (len(left) - 1))


def _validate_settings(settings: OracleExecutionSettings) -> None:
    if settings.weighting_scheme not in {"path", "factor"}:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_weighting_unsupported",
            "only path and factor inner weighting are in the v2 execution contract",
        )
    if settings.preprocessing != "standardized":
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_result_location_unsupported",
            "v2 is bounded to standardized unit-variance result location",
        )
    if settings.max_iterations != FIXED_MAX_ITERATIONS:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_max_iterations_drift",
            "v2 requires exactly 3000 maximum outer iterations",
        )
    if settings.stop_criterion.hex() != FIXED_STOP_CRITERION.hex():
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_stop_criterion_drift",
            "v2 requires the exact 1e-7 outer-weight stop criterion",
        )
    if (
        min(
            settings.bootstrap_samples,
            settings.studentized_inner_samples,
            settings.permutation_samples,
        )
        < 0
    ):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_resampling_invalid",
            "resampling counts cannot be negative",
        )
    if any(
        count != 0
        for count in (
            settings.bootstrap_samples,
            settings.studentized_inner_samples,
            settings.permutation_samples,
        )
    ):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_resampling_unsupported",
            "v2 is a point-estimate-only contract",
        )
    if settings.case_weights_requested:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_case_weights_unsupported",
            "case-weighted score execution is outside v2",
        )
    if settings.higher_order_requested:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_higher_order_unsupported",
            "higher-order score execution is outside v2",
        )
    if settings.interaction_requested:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_interaction_unsupported",
            "interaction score execution is outside v2",
        )


def _validate_and_prepare_data(
    rows: Sequence[Sequence[float | None]],
    row_ids: Sequence[str],
    variables: Sequence[str],
    blocks: Sequence[OracleScoreBlock],
) -> tuple[np.ndarray, tuple[str, ...], dict[str, int]]:
    if len(rows) != len(row_ids):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_row_id_count_mismatch",
            "every input row must have one stable row identifier",
        )
    if len(set(row_ids)) != len(row_ids) or any(not value for value in row_ids):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_row_id_invalid",
            "row identifiers must be non-empty and unique",
        )
    if len(set(variables)) != len(variables) or any(not value for value in variables):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_variable_id_invalid",
            "variable identifiers must be non-empty and unique",
        )
    if not blocks:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_empty_model",
            "at least one score block is required",
        )
    variable_index = {name: index for index, name in enumerate(variables)}
    used_indicators = {
        indicator for block in blocks for indicator in block.indicator_ids
    }
    if used_indicators != set(variables):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_indicator_coverage",
            "the validation oracle requires exact indicator-to-variable coverage",
        )

    complete_rows: list[list[float]] = []
    complete_ids: list[str] = []
    for row_index, (row_id, row) in enumerate(zip(row_ids, rows)):
        if len(row) != len(variables):
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_row_width_mismatch",
                f"row {row_index} has the wrong width",
            )
        if any(value is None for value in row):
            continue
        converted = [float(value) for value in row]
        if any(not math.isfinite(value) for value in converted):
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_non_finite_input",
                f"row {row_index} contains a non-finite value",
            )
        complete_rows.append(converted)
        complete_ids.append(row_id)
    if len(complete_rows) < 3:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_insufficient_rows",
            "at least three complete rows are required",
        )

    raw = np.asarray(complete_rows, dtype=float)
    means = raw.mean(axis=0)
    deviations = raw.std(axis=0, ddof=1)
    if np.any(~np.isfinite(deviations)) or np.any(deviations <= NUMERIC_EPSILON):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_constant_indicator",
            "each indicator must have finite non-zero sample variance",
        )
    standardized = (raw - means) / deviations
    return standardized, tuple(complete_ids), variable_index


def _validate_model(
    blocks: Sequence[OracleScoreBlock], paths: Sequence[OraclePath]
) -> tuple[tuple[OracleScoreBlock, ...], tuple[OraclePath, ...]]:
    construct_ids = [block.construct_id for block in blocks]
    if len(set(construct_ids)) != len(construct_ids) or any(
        not value for value in construct_ids
    ):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_construct_id_invalid",
            "construct identifiers must be non-empty and unique",
        )
    assigned: set[str] = set()
    canonical_blocks: list[OracleScoreBlock] = []
    for block in blocks:
        if not block.indicator_ids:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_empty_block", block.construct_id
            )
        if len(set(block.indicator_ids)) != len(block.indicator_ids):
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_duplicate_block_indicator", block.construct_id
            )
        if assigned.intersection(block.indicator_ids):
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_duplicate_indicator_assignment",
                block.construct_id,
            )
        assigned.update(block.indicator_ids)
        if block.scoring == "estimated":
            if block.mode not in {"mode_a", "mode_b"}:
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_estimated_mode_unsupported", block.mode
                )
            if block.custom_weights:
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_estimated_custom_weights",
                    "estimated blocks cannot declare fixed custom weights",
                )
        else:
            if block.normalization != "unit_variance":
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_fixed_normalization_unsupported",
                    f"{block.construct_id} requests {block.normalization}",
                )
            if block.scoring == "unit" and block.custom_weights:
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_unit_custom_weights",
                    "unit scoring cannot declare custom weights",
                )
            if block.scoring == "custom":
                custom_ids = [row[0] for row in block.custom_weights]
                if (
                    len(set(custom_ids)) != len(custom_ids)
                    or set(custom_ids) != set(block.indicator_ids)
                    or any(
                        not math.isfinite(float(row[1])) for row in block.custom_weights
                    )
                ):
                    raise ScoreExecutionOracleFailure(
                        "pls_score_execution_v2_custom_weight_coverage",
                        f"custom weights must exactly and finitely cover {block.construct_id}",
                    )
            if block.scoring not in {"unit", "custom"}:
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_block_scoring_unsupported", block.scoring
                )
        canonical_blocks.append(
            replace(block, indicator_ids=tuple(sorted(block.indicator_ids)))
        )
    canonical_blocks.sort(key=lambda row: row.construct_id)

    construct_set = set(construct_ids)
    seen_paths: set[tuple[str, str]] = set()
    successors = {construct_id: [] for construct_id in construct_ids}
    canonical_paths: list[OraclePath] = []
    for path in paths:
        key = (path.source, path.target)
        if path.source not in construct_set or path.target not in construct_set:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_unknown_path_construct",
                f"{path.source}->{path.target}",
            )
        if path.source == path.target or key in seen_paths:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_path_invalid", f"{path.source}->{path.target}"
            )
        seen_paths.add(key)
        successors[path.source].append(path.target)
        canonical_paths.append(path)

    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(construct_id: str) -> None:
        if construct_id in visiting:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_structural_feedback",
                "the structural model must be acyclic",
            )
        if construct_id in visited:
            return
        visiting.add(construct_id)
        for target in successors[construct_id]:
            visit(target)
        visiting.remove(construct_id)
        visited.add(construct_id)

    for construct_id in sorted(construct_set):
        visit(construct_id)
    canonical_paths.sort(key=lambda row: (row.target, row.source))
    return tuple(canonical_blocks), tuple(canonical_paths)


def _normalize_weights(
    block: np.ndarray,
    requested: Sequence[float],
    *,
    preserve_orientation: bool,
    subject: str,
) -> np.ndarray:
    weights = np.asarray(requested, dtype=float).copy()
    if weights.shape != (block.shape[1],) or np.any(~np.isfinite(weights)):
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_requested_weight_invalid",
            f"{subject} requires one finite weight per indicator",
        )
    score = block @ weights
    if not preserve_orientation:
        reference = block @ np.ones(block.shape[1])
        association = _covariance(score, reference)
        if association < -1.0e-15 or (
            abs(association) <= 1.0e-15 and float(weights.sum()) < 0.0
        ):
            weights = -weights
            score = -score
    deviation = _sample_sd(score)
    if not math.isfinite(deviation) or deviation <= NUMERIC_EPSILON:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_zero_variance_requested_score",
            f"{subject} produces a zero or non-finite score variance",
        )
    return weights / deviation


def _resolve_blocks(
    transformed: np.ndarray,
    variable_index: dict[str, int],
    blocks: Sequence[OracleScoreBlock],
    initialization: OracleInitialization,
) -> tuple[tuple[_ResolvedBlock, ...], tuple[np.ndarray, ...], tuple[np.ndarray, ...]]:
    if initialization.contract_version != INITIALIZATION_CONTRACT_VERSION:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_initialization_version_mismatch",
            "configured initialization must use pls_initial_outer_weights_v2",
        )
    if initialization.kind == "standard" and initialization.weights:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_standard_initialization_payload",
            "standard initialization cannot carry individual rows",
        )
    if initialization.kind not in {"standard", "individual"}:
        raise ScoreExecutionOracleFailure(
            "pls_score_execution_v2_initialization_kind_unsupported",
            initialization.kind,
        )

    expected_initial_ids = [
        (block.construct_id, indicator)
        for block in blocks
        if block.scoring == "estimated"
        for indicator in block.indicator_ids
    ]
    initial_map: dict[tuple[str, str], float] = {}
    if initialization.kind == "individual":
        previous: tuple[str, str] | None = None
        for row in initialization.weights:
            key = (row.construct_id, row.indicator_id)
            if previous is not None and previous >= key:
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_individual_order_invalid",
                    "individual rows must be strictly sorted by stable construct/indicator ID",
                )
            previous = key
            if key in initial_map or not math.isfinite(float(row.value)):
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_individual_weight_invalid",
                    "individual rows must be unique and finite",
                )
            initial_map[key] = float(row.value)
        if set(initial_map) != set(expected_initial_ids):
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_individual_coverage",
                "individual initialization must exactly cover every estimated indicator",
            )

    resolved_rows: list[_ResolvedBlock] = []
    block_matrices: list[np.ndarray] = []
    effective_weights: list[np.ndarray] = []
    for block in blocks:
        matrix = transformed[
            :, [variable_index[indicator] for indicator in block.indicator_ids]
        ]
        if block.scoring == "estimated":
            if initialization.kind == "standard":
                requested = tuple(1.0 for _ in block.indicator_ids)
                preserve_orientation = False
            else:
                requested = tuple(
                    initial_map[(block.construct_id, indicator)]
                    for indicator in block.indicator_ids
                )
                if not any(value != 0.0 for value in requested):
                    raise ScoreExecutionOracleFailure(
                        "pls_score_execution_v2_individual_zero_block",
                        f"{block.construct_id} has an all-zero individual start",
                    )
                preserve_orientation = True
            initialization_kind: InitializationKind | None = initialization.kind
        elif block.scoring == "unit":
            requested = tuple(1.0 for _ in block.indicator_ids)
            preserve_orientation = True
            initialization_kind = None
        else:
            custom = {key: float(value) for key, value in block.custom_weights}
            requested = tuple(custom[indicator] for indicator in block.indicator_ids)
            preserve_orientation = True
            initialization_kind = None
        effective = _normalize_weights(
            matrix,
            requested,
            preserve_orientation=preserve_orientation,
            subject=f"{block.construct_id} initial/fixed score",
        )
        resolved_rows.append(
            _ResolvedBlock(
                construct_id=block.construct_id,
                indicator_ids=block.indicator_ids,
                scoring=block.scoring,
                mode=block.mode,
                normalization=block.normalization,
                requested_weights=requested,
                effective_initial_weights=tuple(float(value) for value in effective),
                initialization_kind=initialization_kind,
            )
        )
        block_matrices.append(matrix)
        effective_weights.append(effective)
    return tuple(resolved_rows), tuple(block_matrices), tuple(effective_weights)


def _block_scores(
    block_matrices: Sequence[np.ndarray], weights: Sequence[np.ndarray]
) -> tuple[np.ndarray, ...]:
    return tuple(
        _standardize_vector(block @ weight, subject="construct score")
        for block, weight in zip(block_matrices, weights)
    )


def _inner_proxies(
    scores: Sequence[np.ndarray],
    blocks: Sequence[_ResolvedBlock],
    paths: Sequence[OraclePath],
    weighting_scheme: WeightingScheme,
) -> tuple[np.ndarray, ...]:
    construct_index = {block.construct_id: index for index, block in enumerate(blocks)}
    incoming: list[list[int]] = [[] for _ in blocks]
    outgoing: list[list[int]] = [[] for _ in blocks]
    for path in paths:
        source = construct_index[path.source]
        target = construct_index[path.target]
        incoming[target].append(source)
        outgoing[source].append(target)

    proxies: list[np.ndarray] = []
    for construct, score in enumerate(scores):
        if not incoming[construct] and not outgoing[construct]:
            if len(scores) == 1:
                proxies.append(score.copy())
                continue
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_isolated_construct",
                blocks[construct].construct_id,
            )
        proxy = np.zeros_like(score)
        if weighting_scheme == "path" and incoming[construct]:
            design = np.column_stack([scores[index] for index in incoming[construct]])
            coefficients, _residuals, rank, _singular = np.linalg.lstsq(
                design, score, rcond=None
            )
            if rank < design.shape[1]:
                raise ScoreExecutionOracleFailure(
                    "pls_score_execution_v2_structural_rank_deficient",
                    blocks[construct].construct_id,
                )
            proxy += design @ coefficients
        else:
            for source in incoming[construct]:
                proxy += _correlation(score, scores[source]) * scores[source]
        for target in outgoing[construct]:
            proxy += _correlation(score, scores[target]) * scores[target]
        proxies.append(
            _standardize_vector(
                proxy, subject=f"inner proxy {blocks[construct].construct_id}"
            )
        )
    return tuple(proxies)


def _updated_estimated_weight(
    block: _ResolvedBlock, matrix: np.ndarray, proxy: np.ndarray
) -> np.ndarray:
    centered = matrix - matrix.mean(axis=0)
    if block.mode == "mode_a":
        candidate = centered.T @ proxy / (len(proxy) - 1)
    else:
        candidate, _residuals, rank, _singular = np.linalg.lstsq(
            centered, proxy, rcond=None
        )
        if rank < centered.shape[1]:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_mode_b_rank_deficient", block.construct_id
            )
    return _normalize_weights(
        matrix,
        candidate,
        preserve_orientation=False,
        subject=f"{block.construct_id} estimated update",
    )


def estimate_score_execution_v2(
    rows: Sequence[Sequence[float | None]],
    row_ids: Sequence[str],
    variables: Sequence[str],
    blocks: Sequence[OracleScoreBlock],
    paths: Sequence[OraclePath],
    *,
    initialization: OracleInitialization = OracleInitialization(),
    settings: OracleExecutionSettings = OracleExecutionSettings(),
) -> dict[str, Any]:
    """Estimate the bounded PLS score-execution v2 oracle contract."""

    _validate_settings(settings)
    canonical_blocks, canonical_paths = _validate_model(blocks, paths)
    transformed, complete_ids, variable_index = _validate_and_prepare_data(
        rows, row_ids, variables, canonical_blocks
    )
    resolved, block_matrices, initial_weights = _resolve_blocks(
        transformed, variable_index, canonical_blocks, initialization
    )
    weights = tuple(weight.copy() for weight in initial_weights)
    estimated_indices = [
        index for index, block in enumerate(resolved) if block.scoring == "estimated"
    ]
    trace: list[dict[str, Any]] = []

    if not estimated_indices:
        performed_iterations = 0
    else:
        for iteration in range(1, settings.max_iterations + 1):
            scores = _block_scores(block_matrices, weights)
            proxies = _inner_proxies(
                scores, resolved, canonical_paths, settings.weighting_scheme
            )
            updated = list(weights)
            for index in estimated_indices:
                updated[index] = _updated_estimated_weight(
                    resolved[index], block_matrices[index], proxies[index]
                )
            change = max(
                float(np.max(np.abs(weights[index] - updated[index])))
                for index in estimated_indices
            )
            weights = tuple(updated)
            trace.append(
                {
                    "iteration": iteration,
                    "max_abs_estimated_weight_change": change,
                    "estimated_weight_sha256": _canonical_sha256(
                        {
                            resolved[index].construct_id: weights[index].tolist()
                            for index in estimated_indices
                        }
                    ),
                }
            )
            if change <= settings.stop_criterion:
                performed_iterations = iteration
                break
        else:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_non_convergence",
                "estimated blocks did not converge in 3000 iterations",
            )

    scores = _block_scores(block_matrices, weights)
    construct_index = {
        block.construct_id: index for index, block in enumerate(resolved)
    }
    predecessors: list[list[int]] = [[] for _ in resolved]
    for path in canonical_paths:
        predecessors[construct_index[path.target]].append(construct_index[path.source])
    path_rows: list[dict[str, Any]] = []
    r_squared: dict[str, float] = {}
    for target, sources in enumerate(predecessors):
        if not sources:
            continue
        design = np.column_stack([scores[source] for source in sources])
        coefficients, _residuals, rank, _singular = np.linalg.lstsq(
            design, scores[target], rcond=None
        )
        if rank < design.shape[1]:
            raise ScoreExecutionOracleFailure(
                "pls_score_execution_v2_structural_rank_deficient",
                resolved[target].construct_id,
            )
        fitted = design @ coefficients
        residual = scores[target] - fitted
        denominator = float(np.dot(scores[target], scores[target]))
        r_squared[resolved[target].construct_id] = float(
            1.0 - np.dot(residual, residual) / denominator
        )
        for source, coefficient in zip(sources, coefficients):
            path_rows.append(
                {
                    "source": resolved[source].construct_id,
                    "target": resolved[target].construct_id,
                    "coefficient": float(coefficient),
                }
            )
    path_rows.sort(key=lambda row: (row["target"], row["source"]))

    block_rows: list[dict[str, Any]] = []
    for block, final in zip(resolved, weights):
        block_rows.append(
            {
                "construct_id": block.construct_id,
                "indicator_ids": list(block.indicator_ids),
                "scoring": block.scoring,
                "mode": block.mode if block.scoring == "estimated" else None,
                "normalization": (
                    block.normalization if block.scoring != "estimated" else None
                ),
                "initialization_kind": block.initialization_kind,
                "requested_weights": {
                    indicator: float(value)
                    for indicator, value in zip(
                        block.indicator_ids, block.requested_weights
                    )
                },
                "effective_initial_weights": {
                    indicator: float(value)
                    for indicator, value in zip(
                        block.indicator_ids, block.effective_initial_weights
                    )
                },
                "final_weights": {
                    indicator: float(value)
                    for indicator, value in zip(block.indicator_ids, final)
                },
            }
        )

    estimated_count = len(estimated_indices)
    return {
        "oracle_version": ORACLE_VERSION,
        "method_version": METHOD_VERSION,
        "contract_version": CONTRACT_VERSION,
        "converged": True,
        "used_observations": len(complete_ids),
        "omitted_observations": len(rows) - len(complete_ids),
        "row_ids": list(complete_ids),
        "construct_order": [block.construct_id for block in resolved],
        "blocks": block_rows,
        "paths": path_rows,
        "r_squared": r_squared,
        "construct_scores": {
            block.construct_id: {
                row_id: float(value) for row_id, value in zip(complete_ids, score)
            }
            for block, score in zip(resolved, scores)
        },
        "iteration_accounting": {
            "maximum_iterations": settings.max_iterations,
            "stop_criterion": settings.stop_criterion,
            "estimated_block_count": estimated_count,
            "fixed_block_count": len(resolved) - estimated_count,
            "performed_iterations": performed_iterations,
            "estimated_block_updates": performed_iterations * estimated_count,
        },
        "initial_state_sha256": _canonical_sha256(
            {
                block.construct_id: list(block.effective_initial_weights)
                for block in resolved
            }
        ),
        "iteration_trace_sha256": _canonical_sha256(trace),
        "iteration_trace": trace,
    }


def deterministic_fixture(seed: int = 20_260_815) -> dict[str, Any]:
    """Create a deterministic three-construct raw-data fixture."""

    rng = np.random.default_rng(seed)
    observations = 240
    x = rng.normal(size=observations)
    m = 0.58 * x + rng.normal(scale=0.76, size=observations)
    y = 0.20 * x + 0.61 * m + rng.normal(scale=0.68, size=observations)
    variables: list[str] = []
    columns: list[np.ndarray] = []
    blocks: list[OracleScoreBlock] = []
    for construct_id, latent in zip(("x", "m", "y"), (x, m, y)):
        latent_standardized = _standardize_vector(latent, subject=construct_id)
        indicator_ids: list[str] = []
        for index, loading in enumerate((0.91, 0.83, 0.74), start=1):
            indicator_id = f"{construct_id}{index}"
            variable = loading * latent_standardized + math.sqrt(
                1.0 - loading**2
            ) * rng.normal(size=observations)
            variables.append(indicator_id)
            columns.append(variable)
            indicator_ids.append(indicator_id)
        blocks.append(
            OracleScoreBlock(construct_id, tuple(indicator_ids), "estimated", "mode_a")
        )
    return {
        "rows": np.column_stack(columns).tolist(),
        "row_ids": [f"case:{index:04d}" for index in range(observations)],
        "variables": variables,
        "blocks": blocks,
        "paths": [
            OraclePath("x", "m"),
            OraclePath("x", "y"),
            OraclePath("m", "y"),
        ],
    }


def _individual_initialization(
    fixture: dict[str, Any], values: dict[tuple[str, str], float]
) -> OracleInitialization:
    expected = sorted(
        (block.construct_id, indicator)
        for block in fixture["blocks"]
        if block.scoring == "estimated"
        for indicator in block.indicator_ids
    )
    return OracleInitialization(
        "individual",
        tuple(
            OracleInitialWeight(construct, indicator, values[(construct, indicator)])
            for construct, indicator in expected
        ),
    )


def _mixed_fixture() -> dict[str, Any]:
    fixture = deterministic_fixture()
    fixture["blocks"] = [
        replace(fixture["blocks"][0], scoring="unit"),
        fixture["blocks"][1],
        replace(
            fixture["blocks"][2],
            scoring="custom",
            custom_weights=(("y1", -0.25), ("y2", 0.75), ("y3", 0.40)),
        ),
    ]
    return fixture


def _fixed_only_fixture() -> dict[str, Any]:
    fixture = deterministic_fixture()
    fixture["blocks"] = [
        replace(fixture["blocks"][0], scoring="unit"),
        replace(
            fixture["blocks"][2],
            scoring="custom",
            custom_weights=(("y1", -0.25), ("y2", 0.75), ("y3", 0.40)),
        ),
    ]
    kept = {
        indicator for block in fixture["blocks"] for indicator in block.indicator_ids
    }
    positions = [
        index for index, variable in enumerate(fixture["variables"]) if variable in kept
    ]
    fixture["rows"] = [[row[index] for index in positions] for row in fixture["rows"]]
    fixture["variables"] = [fixture["variables"][index] for index in positions]
    fixture["paths"] = [OraclePath("x", "y")]
    return fixture


def _pathological_initialization_fixture() -> tuple[
    dict[str, Any], OracleInitialization
]:
    rng = np.random.default_rng(3_170_011)
    x = rng.normal(size=80)
    y = 0.65 * x + rng.normal(scale=0.65, size=80)
    rows = np.column_stack((x, x, y + 0.2 * rng.normal(size=80), y)).tolist()
    fixture = {
        "rows": rows,
        "row_ids": [f"pathology:{index:03d}" for index in range(len(rows))],
        "variables": ["x1", "x2", "y1", "y2"],
        "blocks": [
            OracleScoreBlock("x", ("x1", "x2")),
            OracleScoreBlock("y", ("y1", "y2")),
        ],
        "paths": [OraclePath("x", "y")],
    }
    initialization = OracleInitialization(
        "individual",
        (
            OracleInitialWeight("x", "x1", 1.0),
            OracleInitialWeight("x", "x2", -1.0),
            OracleInitialWeight("y", "y1", 1.0),
            OracleInitialWeight("y", "y2", 1.0),
        ),
    )
    return fixture, initialization


def _solution_map(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "paths": {
            f"{row['source']}->{row['target']}": row["coefficient"]
            for row in result["paths"]
        },
        "scores": result["construct_scores"],
        "weights": {
            f"{block['construct_id']}::{indicator}": value
            for block in result["blocks"]
            for indicator, value in block["final_weights"].items()
        },
        "r_squared": result["r_squared"],
    }


def _maximum_numeric_difference(left: Any, right: Any) -> float:
    if isinstance(left, dict) and isinstance(right, dict):
        if set(left) != set(right):
            return math.inf
        return max(
            (_maximum_numeric_difference(left[key], right[key]) for key in left),
            default=0.0,
        )
    if isinstance(left, list) and isinstance(right, list):
        if len(left) != len(right):
            return math.inf
        return max(
            (_maximum_numeric_difference(a, b) for a, b in zip(left, right)),
            default=0.0,
        )
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return abs(float(left) - float(right))
    return 0.0 if left == right else math.inf


def _rename_solution(
    result: dict[str, Any],
    construct_reverse: dict[str, str],
    indicator_reverse: dict[str, str],
) -> dict[str, Any]:
    solution = _solution_map(result)
    return {
        "paths": {
            f"{construct_reverse[key.split('->')[0]]}->{construct_reverse[key.split('->')[1]]}": value
            for key, value in solution["paths"].items()
        },
        "scores": {
            construct_reverse[construct]: values
            for construct, values in solution["scores"].items()
        },
        "weights": {
            f"{construct_reverse[key.split('::')[0]]}::{indicator_reverse[key.split('::')[1]]}": value
            for key, value in solution["weights"].items()
        },
        "r_squared": {
            construct_reverse[construct]: value
            for construct, value in solution["r_squared"].items()
        },
    }


def _assert_failure(code: str, operation: Any) -> bool:
    try:
        operation()
    except ScoreExecutionOracleFailure as error:
        return error.code == code
    return False


def run_work_checks() -> dict[str, Any]:
    fixture = deterministic_fixture()
    standard = estimate_score_execution_v2(**fixture)
    standard_repeat = estimate_score_execution_v2(**fixture)
    individual_values = {
        (block.construct_id, indicator): value
        for block, values in zip(
            fixture["blocks"],
            ((-1.0, -0.35, 0.20), (0.15, 1.0, -0.30), (0.30, -0.10, 1.0)),
        )
        for indicator, value in zip(block.indicator_ids, values)
    }
    individual_initialization = _individual_initialization(fixture, individual_values)
    individual = estimate_score_execution_v2(
        **fixture, initialization=individual_initialization
    )
    individual_repeat = estimate_score_execution_v2(
        **fixture, initialization=individual_initialization
    )

    pathological_fixture, pathological_start = _pathological_initialization_fixture()
    pathological_standard = estimate_score_execution_v2(**pathological_fixture)
    pathological_individual_rejected = _assert_failure(
        "pls_score_execution_v2_zero_variance_requested_score",
        lambda: estimate_score_execution_v2(
            **pathological_fixture, initialization=pathological_start
        ),
    )

    mixed_fixture = _mixed_fixture()
    mixed = estimate_score_execution_v2(**mixed_fixture)
    fixed_blocks_unchanged = all(
        block["effective_initial_weights"] == block["final_weights"]
        for block in mixed["blocks"]
        if block["scoring"] != "estimated"
    )

    fixed_fixture = _fixed_only_fixture()
    fixed_only = estimate_score_execution_v2(**fixed_fixture)
    raw = np.asarray(fixed_fixture["rows"], dtype=float)
    standardized = (raw - raw.mean(axis=0)) / raw.std(axis=0, ddof=1)
    positions = {name: index for index, name in enumerate(fixed_fixture["variables"])}
    expected_fixed_scores: dict[str, dict[str, float]] = {}
    for block in fixed_fixture["blocks"]:
        block_matrix = standardized[
            :, [positions[indicator] for indicator in sorted(block.indicator_ids)]
        ]
        if block.scoring == "unit":
            requested = np.ones(block_matrix.shape[1])
        else:
            custom = dict(block.custom_weights)
            requested = np.asarray(
                [custom[indicator] for indicator in sorted(block.indicator_ids)]
            )
        score = _standardize_vector(
            block_matrix @ requested, subject="fixed-only hand score"
        )
        expected_fixed_scores[block.construct_id] = {
            row_id: float(value)
            for row_id, value in zip(fixed_fixture["row_ids"], score)
        }
    fixed_hand_difference = _maximum_numeric_difference(
        expected_fixed_scores, fixed_only["construct_scores"]
    )

    baseline_map = _solution_map(mixed)
    reordered = {
        **mixed_fixture,
        "blocks": [
            replace(block, indicator_ids=tuple(reversed(block.indicator_ids)))
            for block in reversed(mixed_fixture["blocks"])
        ],
        "paths": list(reversed(mixed_fixture["paths"])),
    }
    order_result = estimate_score_execution_v2(**reordered)

    variable_order = list(reversed(range(len(mixed_fixture["variables"]))))
    variable_result = estimate_score_execution_v2(
        [[row[index] for index in variable_order] for row in mixed_fixture["rows"]],
        mixed_fixture["row_ids"],
        [mixed_fixture["variables"][index] for index in variable_order],
        mixed_fixture["blocks"],
        mixed_fixture["paths"],
    )

    row_order = list(reversed(range(len(mixed_fixture["rows"]))))
    row_result = estimate_score_execution_v2(
        [mixed_fixture["rows"][index] for index in row_order],
        [mixed_fixture["row_ids"][index] for index in row_order],
        mixed_fixture["variables"],
        mixed_fixture["blocks"],
        mixed_fixture["paths"],
    )

    scales = np.linspace(0.55, 2.65, len(mixed_fixture["variables"]))
    offsets = np.linspace(-7.0, 11.0, len(mixed_fixture["variables"]))
    affine_result = estimate_score_execution_v2(
        [
            [
                float(value * scales[index] + offsets[index])
                for index, value in enumerate(row)
            ]
            for row in mixed_fixture["rows"]
        ],
        mixed_fixture["row_ids"],
        mixed_fixture["variables"],
        mixed_fixture["blocks"],
        mixed_fixture["paths"],
    )

    construct_names = {"x": "zeta", "m": "alpha", "y": "kappa"}
    indicator_names = {
        indicator: f"v{index:02d}"
        for index, indicator in enumerate(reversed(mixed_fixture["variables"]), start=1)
    }
    renamed_blocks = [
        replace(
            block,
            construct_id=construct_names[block.construct_id],
            indicator_ids=tuple(
                indicator_names[value] for value in block.indicator_ids
            ),
            custom_weights=tuple(
                (indicator_names[indicator], value)
                for indicator, value in block.custom_weights
            ),
        )
        for block in mixed_fixture["blocks"]
    ]
    renamed_paths = [
        OraclePath(construct_names[path.source], construct_names[path.target])
        for path in mixed_fixture["paths"]
    ]
    renamed_result = estimate_score_execution_v2(
        mixed_fixture["rows"],
        mixed_fixture["row_ids"],
        [indicator_names[value] for value in mixed_fixture["variables"]],
        renamed_blocks,
        renamed_paths,
    )
    renamed_map = _rename_solution(
        renamed_result,
        {value: key for key, value in construct_names.items()},
        {value: key for key, value in indicator_names.items()},
    )

    metamorphic_differences = {
        "stable_id_renaming": _maximum_numeric_difference(baseline_map, renamed_map),
        "model_declaration_order": _maximum_numeric_difference(
            baseline_map, _solution_map(order_result)
        ),
        "variable_declaration_order": _maximum_numeric_difference(
            baseline_map, _solution_map(variable_result)
        ),
        "stable_row_order": _maximum_numeric_difference(
            baseline_map, _solution_map(row_result)
        ),
        "positive_affine_rescaling": _maximum_numeric_difference(
            baseline_map, _solution_map(affine_result)
        ),
    }
    metamorphic_tolerance = 2.0e-10

    base_fixed_block = OracleScoreBlock(
        "fixed", ("f1", "f2"), scoring="unit", normalization="unit_variance"
    )
    boundary_rows = [[1.0, 3.0], [2.0, 4.5], [4.0, 7.0], [8.0, 9.0]]
    boundary_ids = ["b1", "b2", "b3", "b4"]
    boundary_variables = ["f1", "f2"]
    typed_boundaries = {
        "normalization_none": _assert_failure(
            "pls_score_execution_v2_fixed_normalization_unsupported",
            lambda: estimate_score_execution_v2(
                boundary_rows,
                boundary_ids,
                boundary_variables,
                [replace(base_fixed_block, normalization="none")],
                [],
            ),
        ),
        "normalization_sum_to_one": _assert_failure(
            "pls_score_execution_v2_fixed_normalization_unsupported",
            lambda: estimate_score_execution_v2(
                boundary_rows,
                boundary_ids,
                boundary_variables,
                [replace(base_fixed_block, normalization="sum_to_one")],
                [],
            ),
        ),
        "bootstrap": _assert_failure(
            "pls_score_execution_v2_resampling_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(bootstrap_samples=1),
            ),
        ),
        "studentized": _assert_failure(
            "pls_score_execution_v2_resampling_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(studentized_inner_samples=1),
            ),
        ),
        "permutation": _assert_failure(
            "pls_score_execution_v2_resampling_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(permutation_samples=1),
            ),
        ),
        "mean_centered_location": _assert_failure(
            "pls_score_execution_v2_result_location_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(preprocessing="mean_centered"),
            ),
        ),
        "unstandardized_location": _assert_failure(
            "pls_score_execution_v2_result_location_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(preprocessing="unstandardized"),
            ),
        ),
        "case_weights": _assert_failure(
            "pls_score_execution_v2_case_weights_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(case_weights_requested=True),
            ),
        ),
        "higher_order": _assert_failure(
            "pls_score_execution_v2_higher_order_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(higher_order_requested=True),
            ),
        ),
        "interaction": _assert_failure(
            "pls_score_execution_v2_interaction_unsupported",
            lambda: estimate_score_execution_v2(
                **fixed_fixture,
                settings=OracleExecutionSettings(interaction_requested=True),
            ),
        ),
    }

    final_initialization_difference = _maximum_numeric_difference(
        _solution_map(standard), _solution_map(individual)
    )
    ordinary_initialization_solution_tolerance = 2.0e-8
    checks = {
        "standard_deterministic": standard == standard_repeat,
        "individual_deterministic": individual == individual_repeat,
        "standard_and_individual_converge": standard["converged"]
        and individual["converged"],
        "initialization_states_differ": standard["initial_state_sha256"]
        != individual["initial_state_sha256"],
        "initialization_trajectories_differ": standard["iteration_trace_sha256"]
        != individual["iteration_trace_sha256"],
        "ordinary_initializations_reach_same_stationary_solution": (
            final_initialization_difference
            <= ordinary_initialization_solution_tolerance
        ),
        "pathological_start_differs_fail_closed": pathological_standard["converged"]
        and pathological_individual_rejected,
        "mixed_updates_estimated_only": mixed["iteration_accounting"][
            "estimated_block_count"
        ]
        == 1
        and mixed["iteration_accounting"]["fixed_block_count"] == 2
        and mixed["iteration_accounting"]["estimated_block_updates"]
        == mixed["iteration_accounting"]["performed_iterations"],
        "mixed_fixed_weights_never_update": fixed_blocks_unchanged,
        "fixed_only_zero_iterations": fixed_only["iteration_accounting"]
        == {
            "maximum_iterations": FIXED_MAX_ITERATIONS,
            "stop_criterion": FIXED_STOP_CRITERION,
            "estimated_block_count": 0,
            "fixed_block_count": 2,
            "performed_iterations": 0,
            "estimated_block_updates": 0,
        },
        "fixed_only_scores_match_hand_linear_combinations": fixed_hand_difference
        <= 2.0e-14,
        "metamorphics_within_tolerance": all(
            difference <= metamorphic_tolerance
            for difference in metamorphic_differences.values()
        ),
        "typed_boundaries_exact": all(typed_boundaries.values()),
    }
    return {
        "schema_version": 1,
        "report_kind": "pls_score_execution_v2_independent_oracle_work_v1",
        "oracle_version": ORACLE_VERSION,
        "method_version": METHOD_VERSION,
        "contract_version": CONTRACT_VERSION,
        "passed": all(checks.values()),
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_requested": False,
        "product_comparison_performed": False,
        "receipt_attached": False,
        "registry_or_manifest_mutation_requested": False,
        "checks": checks,
        "initialization_cases": {
            "standard_iterations": standard["iteration_accounting"][
                "performed_iterations"
            ],
            "individual_iterations": individual["iteration_accounting"][
                "performed_iterations"
            ],
            "standard_initial_state_sha256": standard["initial_state_sha256"],
            "individual_initial_state_sha256": individual["initial_state_sha256"],
            "standard_trace_sha256": standard["iteration_trace_sha256"],
            "individual_trace_sha256": individual["iteration_trace_sha256"],
            "converged_solution_max_abs_difference": final_initialization_difference,
            "converged_solution_tolerance": ordinary_initialization_solution_tolerance,
            "pathological_standard_converged": pathological_standard["converged"],
            "pathological_individual_failure_code": (
                "pls_score_execution_v2_zero_variance_requested_score"
            ),
        },
        "fixed_and_mixed_cases": {
            "mixed_iteration_accounting": mixed["iteration_accounting"],
            "mixed_fixed_weights_unchanged": fixed_blocks_unchanged,
            "fixed_only_iteration_accounting": fixed_only["iteration_accounting"],
            "fixed_only_hand_score_max_abs_difference": fixed_hand_difference,
        },
        "metamorphic_tolerance": metamorphic_tolerance,
        "metamorphic_max_abs_differences": metamorphic_differences,
        "typed_boundaries": typed_boundaries,
        "qualification_spec_v2_alignment": {
            "candidate_oracle_kind": "independent_implementation",
            "runtime_policy": "development_validation_only",
            "covered_work_estimands": [
                "resolved_initial_outer_weights",
                "fixed_unit_variance_score_weights",
                "construct_scores",
                "path_coefficients",
                "iteration_accounting",
            ],
            "scenario_axes_exercised": [
                "model_topology",
                "measurement_model",
                "data_distribution",
                "input_type",
                "workload",
            ],
            "admissible_receipt": False,
            "reason_not_admissible": (
                "no frozen current QuickPLS build was executed against these cases"
            ),
        },
        "remaining_blockers": [
            "A frozen current QuickPLS product build has not been compared with this oracle.",
            "Source-set, scenario-set, build-fingerprint, command, and immutable receipt identity are absent.",
            "Qualification-sized generative, persistence/export, packaged Windows, performance, accessibility, and independent-review evidence remains required.",
            "Broader result-location, resampling, case-weight, higher-order, and interaction execution remains explicitly outside v2.",
        ],
    }


def write_work_report(path: Path) -> dict[str, Any]:
    report = run_work_checks()
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--write-work-report",
        type=Path,
        help="write a non-promotional deterministic work report",
    )
    args = parser.parse_args()
    report = (
        write_work_report(args.write_work_report)
        if args.write_work_report is not None
        else run_work_checks()
    )
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
