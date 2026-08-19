#!/usr/bin/env python3
"""Validation-only independent PLS-PM oracle for General SEM Rank 0.

This module uses only the Python standard library and never imports QuickPLS
production Rust or TypeScript.  It implements the documented path-weighted
PLS-PM iteration for Mode A and supported Mode B blocks, deterministic score
orientation, structural paths and effects, simultaneous two-way moderation,
and indexed full-model case bootstraps.

The implementation is an independently maintained qualification oracle, not a
runtime dependency and not evidence by itself.  Qualification-scale evidence
must be minted later from frozen scenarios after all compared sources are
stable.
"""

from __future__ import annotations

import hashlib
import math
import random
from dataclasses import dataclass
from typing import Iterable, Literal, Mapping, Sequence


Mode = Literal["A", "B"]
DEFAULT_TOLERANCE = 1e-10
DEFAULT_MAX_ITERATIONS = 3_000
NUMERICAL_RANK_TOLERANCE = 1e-12
BOOTSTRAP_STREAM_VERSION = "general_sem_rank0_sha256_indexed_case_stream_v1"
FIXED_PROBES = (-1.0, 0.0, 1.0)


class OracleError(ValueError):
    """Base class for deterministic scientific-oracle failures."""


class ModelContractError(OracleError):
    """The declared model is outside the bounded recursive PLS contract."""


class NumericalOracleError(OracleError):
    """A fit is numerically undefined under the frozen contract."""


@dataclass(frozen=True)
class BlockSpec:
    construct_id: str
    indicator_ids: tuple[str, ...]
    mode: Mode = "A"


@dataclass(frozen=True)
class PathSpec:
    source_id: str
    target_id: str


@dataclass(frozen=True)
class PlsModelSpec:
    blocks: tuple[BlockSpec, ...]
    paths: tuple[PathSpec, ...]


@dataclass(frozen=True)
class PlsFit:
    used_row_indices: tuple[int, ...]
    weights: Mapping[str, tuple[float, ...]]
    loadings: Mapping[str, tuple[float, ...]]
    scores: Mapping[str, tuple[float, ...]]
    path_coefficients: Mapping[tuple[str, str], float]
    r_squared: Mapping[str, float]
    iterations: int
    convergence_change: float


@dataclass(frozen=True)
class InteractionSpec:
    interaction_id: str
    focal_id: str
    moderator_id: str
    outcome_id: str


@dataclass(frozen=True)
class ModerationFit:
    direct_coefficients: Mapping[tuple[str, str], float]
    standardized_product_coefficients: Mapping[str, float]
    scientific_gammas: Mapping[str, float]
    product_means: Mapping[str, float]
    product_scales: Mapping[str, float]
    fixed_probe_slopes: Mapping[str, tuple[float, float, float]]


@dataclass(frozen=True)
class BootstrapSummary:
    original: float
    mean: float
    bias: float
    standard_error: float
    lower: float
    upper: float
    exceedances: int
    plus_one_two_sided_probability: float


@dataclass(frozen=True)
class BootstrapFailure:
    replicate_index: int
    reason: str


@dataclass(frozen=True)
class BootstrapResult:
    requested: int
    usable: int
    minimum_usable: int
    published: bool
    summaries: Mapping[str, BootstrapSummary]
    failures: tuple[BootstrapFailure, ...]
    usable_indices: tuple[int, ...]
    sign_corrections: int


def _identifier_key(value: str) -> bytes:
    """Match the production plan's stable UTF-8 identifier ordering."""

    return value.encode("utf-8")


def canonicalize_model(model: PlsModelSpec) -> PlsModelSpec:
    """Remove declaration order as a scientific or orientation authority."""

    blocks = tuple(
        BlockSpec(
            block.construct_id,
            tuple(sorted(block.indicator_ids, key=_identifier_key)),
            block.mode,
        )
        for block in sorted(
            model.blocks, key=lambda block: _identifier_key(block.construct_id)
        )
    )
    paths = tuple(
        sorted(
            model.paths,
            key=lambda path: (
                _identifier_key(path.source_id),
                _identifier_key(path.target_id),
            ),
        )
    )
    return PlsModelSpec(blocks, paths)


def canonicalize_interactions(
    interactions: Sequence[InteractionSpec],
) -> tuple[InteractionSpec, ...]:
    return tuple(
        sorted(
            interactions,
            key=lambda interaction: (
                _identifier_key(interaction.interaction_id),
                _identifier_key(interaction.focal_id),
                _identifier_key(interaction.moderator_id),
                _identifier_key(interaction.outcome_id),
            ),
        )
    )


def mean(values: Sequence[float]) -> float:
    if not values:
        raise NumericalOracleError("mean requires at least one value")
    return math.fsum(values) / len(values)


def sample_sd(values: Sequence[float]) -> float:
    if len(values) < 2:
        raise NumericalOracleError("sample standard deviation requires two values")
    center = mean(values)
    variance = math.fsum((value - center) ** 2 for value in values) / (
        len(values) - 1
    )
    if not math.isfinite(variance) or variance <= NUMERICAL_RANK_TOLERANCE**2:
        raise NumericalOracleError("sample variance is zero or nonfinite")
    return math.sqrt(variance)


def standardize(values: Sequence[float]) -> list[float]:
    if any(not math.isfinite(value) for value in values):
        raise NumericalOracleError("cannot standardize nonfinite values")
    center = mean(values)
    scale = sample_sd(values)
    return [(value - center) / scale for value in values]


def covariance(left: Sequence[float], right: Sequence[float]) -> float:
    if len(left) != len(right) or len(left) < 2:
        raise NumericalOracleError("covariance vectors must have equal length >= 2")
    left_center = mean(left)
    right_center = mean(right)
    return math.fsum(
        (a - left_center) * (b - right_center)
        for a, b in zip(left, right, strict=True)
    ) / (len(left) - 1)


def correlation(left: Sequence[float], right: Sequence[float]) -> float:
    return covariance(left, right) / (sample_sd(left) * sample_sd(right))


def type7(values: Iterable[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if not ordered:
        raise NumericalOracleError("Type-7 quantile requires at least one value")
    if not 0.0 <= probability <= 1.0:
        raise NumericalOracleError("quantile probability must be in [0, 1]")
    location = (len(ordered) - 1) * probability
    lower = math.floor(location)
    upper = math.ceil(location)
    if lower == upper:
        return ordered[lower]
    fraction = location - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def sample_standard_error(values: Iterable[float]) -> float:
    return sample_sd([float(value) for value in values])


def plus_one_two_sided(
    original: float, values: Iterable[float]
) -> tuple[int, float]:
    samples = [float(value) for value in values]
    if not samples:
        raise NumericalOracleError("plus-one probability requires replicates")
    exceedances = sum(
        abs(value - original) >= abs(original) for value in samples
    )
    return exceedances, (exceedances + 1) / (len(samples) + 1)


def minimum_usable_replicates(requested: int) -> int:
    if not 2 <= requested <= 10_000:
        raise ModelContractError("requested resamples must be in [2, 10000]")
    return max(2, math.ceil(0.9 * requested))


def _linear_combination(
    columns: Sequence[Sequence[float]], coefficients: Sequence[float]
) -> list[float]:
    if not columns or len(columns) != len(coefficients):
        raise NumericalOracleError("invalid linear-combination dimensions")
    width = len(columns[0])
    if any(len(column) != width for column in columns):
        raise NumericalOracleError("linear-combination columns differ in length")
    return [
        math.fsum(
            coefficient * column[row]
            for coefficient, column in zip(coefficients, columns, strict=True)
        )
        for row in range(width)
    ]


def _solve(matrix: Sequence[Sequence[float]], vector: Sequence[float]) -> list[float]:
    size = len(vector)
    if size == 0 or len(matrix) != size or any(len(row) != size for row in matrix):
        raise NumericalOracleError("invalid linear-system dimensions")
    augmented = [
        [float(value) for value in row] + [float(target)]
        for row, target in zip(matrix, vector, strict=True)
    ]
    largest = max(abs(value) for row in matrix for value in row)
    threshold = max(NUMERICAL_RANK_TOLERANCE, largest * NUMERICAL_RANK_TOLERANCE)
    for column in range(size):
        pivot = max(range(column, size), key=lambda row: abs(augmented[row][column]))
        if abs(augmented[pivot][column]) <= threshold:
            raise NumericalOracleError("rank-deficient least-squares equation")
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        divisor = augmented[column][column]
        augmented[column] = [value / divisor for value in augmented[column]]
        for row in range(size):
            if row == column:
                continue
            factor = augmented[row][column]
            augmented[row] = [
                value - factor * basis
                for value, basis in zip(
                    augmented[row], augmented[column], strict=True
                )
            ]
    solution = [augmented[row][-1] for row in range(size)]
    if any(not math.isfinite(value) for value in solution):
        raise NumericalOracleError("least-squares solution is nonfinite")
    return solution


def _ols(
    predictors: Sequence[Sequence[float]], outcome: Sequence[float]
) -> list[float]:
    if not predictors:
        raise NumericalOracleError("least squares requires predictors")
    rows = len(outcome)
    if rows <= len(predictors) or any(len(column) != rows for column in predictors):
        raise NumericalOracleError("least-squares dimensions are underidentified")
    gram = [
        [
            math.fsum(
                left * right
                for left, right in zip(
                    predictors[i], predictors[j], strict=True
                )
            )
            for j in range(len(predictors))
        ]
        for i in range(len(predictors))
    ]
    rhs = [
        math.fsum(x * y for x, y in zip(column, outcome, strict=True))
        for column in predictors
    ]
    return _solve(gram, rhs)


def _validate_model(model: PlsModelSpec) -> None:
    if not model.blocks:
        raise ModelContractError("PLS model requires at least one block")
    block_ids = [block.construct_id for block in model.blocks]
    if len(block_ids) != len(set(block_ids)):
        raise ModelContractError("construct identifiers must be unique")
    indicators: list[str] = []
    for block in model.blocks:
        if block.mode not in {"A", "B"} or not block.indicator_ids:
            raise ModelContractError("every block needs indicators and Mode A or B")
        indicators.extend(block.indicator_ids)
    if len(indicators) != len(set(indicators)):
        raise ModelContractError("indicators must belong to exactly one block")
    ids = set(block_ids)
    path_keys: set[tuple[str, str]] = set()
    outgoing: dict[str, list[str]] = {identifier: [] for identifier in ids}
    for path in model.paths:
        key = (path.source_id, path.target_id)
        if path.source_id not in ids or path.target_id not in ids:
            raise ModelContractError("structural path references an unknown construct")
        if path.source_id == path.target_id or key in path_keys:
            raise ModelContractError("structural paths must be unique and non-reflexive")
        path_keys.add(key)
        outgoing[path.source_id].append(path.target_id)
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(identifier: str) -> None:
        if identifier in visiting:
            raise ModelContractError("structural graph must be acyclic")
        if identifier in visited:
            return
        visiting.add(identifier)
        for target in outgoing[identifier]:
            visit(target)
        visiting.remove(identifier)
        visited.add(identifier)

    for identifier in block_ids:
        visit(identifier)


def _prepare_columns(
    rows: Sequence[Mapping[str, float | int | None]], model: PlsModelSpec
) -> tuple[tuple[int, ...], dict[str, list[float]]]:
    indicator_ids = [
        indicator for block in model.blocks for indicator in block.indicator_ids
    ]
    retained: list[tuple[int, list[float]]] = []
    for index, row in enumerate(rows):
        values: list[float] = []
        missing = False
        for indicator in indicator_ids:
            value = row.get(indicator)
            if value is None:
                missing = True
                break
            if isinstance(value, bool) or not isinstance(value, (int, float)):
                raise ModelContractError(f"indicator {indicator!r} is not numeric")
            numeric = float(value)
            if not math.isfinite(numeric):
                raise ModelContractError(f"indicator {indicator!r} is nonfinite")
            values.append(numeric)
        if not missing:
            retained.append((index, values))
    if len(retained) < 3:
        raise NumericalOracleError("fewer than three complete observations remain")
    columns = {
        indicator: standardize([values[position] for _, values in retained])
        for position, indicator in enumerate(indicator_ids)
    }
    return tuple(index for index, _ in retained), columns


def _normalize_weights(
    block: BlockSpec,
    columns: Mapping[str, Sequence[float]],
    weights: Sequence[float],
) -> list[float]:
    if len(weights) != len(block.indicator_ids):
        raise NumericalOracleError("block-weight dimensions differ")
    score = _linear_combination(
        [columns[indicator] for indicator in block.indicator_ids], weights
    )
    scale = sample_sd(score)
    normalized = [value / scale for value in weights]
    anchor = next(
        (value for value in normalized if abs(value) > NUMERICAL_RANK_TOLERANCE),
        None,
    )
    if anchor is None:
        raise NumericalOracleError(f"all weights are zero for {block.construct_id}")
    if anchor < 0:
        normalized = [-value for value in normalized]
    return normalized


def _block_scores(
    model: PlsModelSpec,
    columns: Mapping[str, Sequence[float]],
    weights: Mapping[str, Sequence[float]],
) -> dict[str, list[float]]:
    return {
        block.construct_id: _linear_combination(
            [columns[indicator] for indicator in block.indicator_ids],
            weights[block.construct_id],
        )
        for block in model.blocks
    }


def _inner_proxies(
    model: PlsModelSpec, scores: Mapping[str, Sequence[float]]
) -> dict[str, list[float]]:
    incoming: dict[str, list[str]] = {
        block.construct_id: [] for block in model.blocks
    }
    outgoing: dict[str, list[str]] = {
        block.construct_id: [] for block in model.blocks
    }
    for path in model.paths:
        incoming[path.target_id].append(path.source_id)
        outgoing[path.source_id].append(path.target_id)
    proxies: dict[str, list[float]] = {}
    for block in model.blocks:
        identifier = block.construct_id
        if not incoming[identifier] and not outgoing[identifier]:
            if len(model.blocks) == 1:
                proxies[identifier] = list(scores[identifier])
                continue
            raise ModelContractError(f"isolated construct {identifier!r}")
        proxy = [0.0] * len(scores[identifier])
        if incoming[identifier]:
            coefficients = _ols(
                [scores[source] for source in incoming[identifier]],
                scores[identifier],
            )
            for source, coefficient in zip(
                incoming[identifier], coefficients, strict=True
            ):
                proxy = [
                    current + coefficient * value
                    for current, value in zip(proxy, scores[source], strict=True)
                ]
        for target in outgoing[identifier]:
            coefficient = correlation(scores[identifier], scores[target])
            proxy = [
                current + coefficient * value
                for current, value in zip(proxy, scores[target], strict=True)
            ]
        proxies[identifier] = standardize(proxy)
    return proxies


def _structural_solution(
    model: PlsModelSpec, scores: Mapping[str, Sequence[float]]
) -> tuple[dict[tuple[str, str], float], dict[str, float]]:
    paths: dict[tuple[str, str], float] = {}
    r_squared: dict[str, float] = {}
    for target in (block.construct_id for block in model.blocks):
        predecessors = [
            path.source_id for path in model.paths if path.target_id == target
        ]
        if not predecessors:
            continue
        coefficients = _ols(
            [scores[source] for source in predecessors], scores[target]
        )
        fitted = _linear_combination(
            [scores[source] for source in predecessors], coefficients
        )
        residual = math.fsum(
            (actual - predicted) ** 2
            for actual, predicted in zip(scores[target], fitted, strict=True)
        )
        total = math.fsum(value * value for value in scores[target])
        if total <= NUMERICAL_RANK_TOLERANCE:
            raise NumericalOracleError(f"zero structural variance for {target}")
        r_squared[target] = 1.0 - residual / total
        for source, coefficient in zip(predecessors, coefficients, strict=True):
            paths[(source, target)] = coefficient
    return paths, r_squared


def fit_pls_pm(
    rows: Sequence[Mapping[str, float | int | None]],
    model: PlsModelSpec,
    *,
    tolerance: float = DEFAULT_TOLERANCE,
    maximum_iterations: int = DEFAULT_MAX_ITERATIONS,
) -> PlsFit:
    """Fit the bounded path-weighted PLS-PM contract independently."""

    model = canonicalize_model(model)
    _validate_model(model)
    if not 0 < tolerance < 1 or maximum_iterations < 1:
        raise ModelContractError("invalid convergence controls")
    used_rows, columns = _prepare_columns(rows, model)
    weights: dict[str, list[float]] = {
        block.construct_id: _normalize_weights(
            block, columns, [1.0] * len(block.indicator_ids)
        )
        for block in model.blocks
    }
    convergence_change = math.inf
    iterations = 0
    for iteration in range(1, maximum_iterations + 1):
        scores = _block_scores(model, columns, weights)
        inner = _inner_proxies(model, scores)
        updated: dict[str, list[float]] = {}
        for block in model.blocks:
            if block.mode == "A":
                candidate = [
                    covariance(columns[indicator], inner[block.construct_id])
                    for indicator in block.indicator_ids
                ]
            else:
                candidate = _ols(
                    [columns[indicator] for indicator in block.indicator_ids],
                    inner[block.construct_id],
                )
            updated[block.construct_id] = _normalize_weights(
                block, columns, candidate
            )
        convergence_change = max(
            abs(old - new)
            for block in model.blocks
            for old, new in zip(
                weights[block.construct_id],
                updated[block.construct_id],
                strict=True,
            )
        )
        weights = updated
        iterations = iteration
        if convergence_change <= tolerance:
            break
    else:
        raise NumericalOracleError(
            f"PLS-PM did not converge in {maximum_iterations} iterations"
        )
    scores = _block_scores(model, columns, weights)
    path_coefficients, r_squared = _structural_solution(model, scores)
    loadings = {
        block.construct_id: tuple(
            correlation(columns[indicator], scores[block.construct_id])
            for indicator in block.indicator_ids
        )
        for block in model.blocks
    }
    return PlsFit(
        used_row_indices=used_rows,
        weights={identifier: tuple(values) for identifier, values in weights.items()},
        loadings=loadings,
        scores={identifier: tuple(values) for identifier, values in scores.items()},
        path_coefficients=path_coefficients,
        r_squared=r_squared,
        iterations=iterations,
        convergence_change=convergence_change,
    )


def align_fit_to_reference(
    fit: PlsFit,
    reference: PlsFit,
    sampled_indices: Sequence[int],
    model: PlsModelSpec,
) -> tuple[PlsFit, int]:
    """Align every refit score vector before effects or products are rebuilt."""

    model = canonicalize_model(model)
    if len(sampled_indices) != len(next(iter(fit.scores.values()))):
        raise NumericalOracleError("alignment index count differs from refit rows")
    aligned_scores: dict[str, tuple[float, ...]] = {}
    aligned_weights: dict[str, tuple[float, ...]] = {}
    aligned_loadings: dict[str, tuple[float, ...]] = {}
    corrections = 0
    for block in model.blocks:
        identifier = block.construct_id
        reference_vector = [reference.scores[identifier][index] for index in sampled_indices]
        score = fit.scores[identifier]
        association = covariance(reference_vector, score)
        if abs(association) <= NUMERICAL_RANK_TOLERANCE:
            raise NumericalOracleError(
                f"orientation is indeterminate for {identifier}"
            )
        sign = -1.0 if association < 0 else 1.0
        corrections += int(sign < 0)
        aligned_scores[identifier] = tuple(sign * value for value in score)
        aligned_weights[identifier] = tuple(
            sign * value for value in fit.weights[identifier]
        )
        aligned_loadings[identifier] = tuple(
            sign * value for value in fit.loadings[identifier]
        )
    paths, r_squared = _structural_solution(model, aligned_scores)
    return (
        PlsFit(
            used_row_indices=fit.used_row_indices,
            weights=aligned_weights,
            loadings=aligned_loadings,
            scores=aligned_scores,
            path_coefficients=paths,
            r_squared=r_squared,
            iterations=fit.iterations,
            convergence_change=fit.convergence_change,
        ),
        corrections,
    )


def directed_paths(
    model: PlsModelSpec, source_id: str, target_id: str
) -> tuple[tuple[str, ...], ...]:
    model = canonicalize_model(model)
    ids = {block.construct_id for block in model.blocks}
    if source_id not in ids or target_id not in ids:
        raise ModelContractError("effect endpoint is not a construct")
    outgoing: dict[str, list[str]] = {identifier: [] for identifier in ids}
    for path in model.paths:
        outgoing[path.source_id].append(path.target_id)
    result: list[tuple[str, ...]] = []

    def walk(current: str, route: tuple[str, ...]) -> None:
        if current == target_id:
            result.append(route)
            return
        for successor in outgoing[current]:
            if successor not in route:
                walk(successor, (*route, successor))

    walk(source_id, (source_id,))
    return tuple(result)


def mediation_effects(
    fit: PlsFit, model: PlsModelSpec, source_id: str, target_id: str
) -> dict[str, float]:
    """Return specific paths plus direct, total-indirect, and total effects."""

    routes = directed_paths(model, source_id, target_id)
    specific: dict[str, float] = {}
    for route in routes:
        if len(route) < 3:
            continue
        product = math.prod(
            fit.path_coefficients[(left, right)]
            for left, right in zip(route[:-1], route[1:], strict=True)
        )
        specific["specific:" + "->".join(route)] = product
    direct = fit.path_coefficients.get((source_id, target_id), 0.0)
    total_indirect = math.fsum(specific.values())
    return {
        **specific,
        f"total_indirect:{source_id}->{target_id}": total_indirect,
        f"direct:{source_id}->{target_id}": direct,
        f"total:{source_id}->{target_id}": direct + total_indirect,
    }


def fit_simultaneous_moderation(
    fit: PlsFit,
    model: PlsModelSpec,
    interactions: Sequence[InteractionSpec],
) -> ModerationFit:
    """Fit complete joint stage-two equations and scientific gamma targets."""

    model = canonicalize_model(model)
    interactions = canonicalize_interactions(interactions)
    if not interactions:
        raise ModelContractError("moderation requires at least one interaction")
    ids = {block.construct_id for block in model.blocks}
    interaction_ids: set[str] = set()
    for interaction in interactions:
        if interaction.interaction_id in interaction_ids:
            raise ModelContractError("interaction identifiers must be unique")
        interaction_ids.add(interaction.interaction_id)
        if {
            interaction.focal_id,
            interaction.moderator_id,
            interaction.outcome_id,
        } - ids:
            raise ModelContractError("interaction references an unknown construct")
        required = {
            (interaction.focal_id, interaction.outcome_id),
            (interaction.moderator_id, interaction.outcome_id),
        }
        if not required <= {
            (path.source_id, path.target_id) for path in model.paths
        }:
            raise ModelContractError("strong hierarchy requires both direct paths")
    direct: dict[tuple[str, str], float] = {}
    product_coefficients: dict[str, float] = {}
    gammas: dict[str, float] = {}
    product_means: dict[str, float] = {}
    product_scales: dict[str, float] = {}
    slopes: dict[str, tuple[float, float, float]] = {}
    outcomes = list(dict.fromkeys(row.outcome_id for row in interactions))
    for outcome in outcomes:
        predecessors = [
            path.source_id for path in model.paths if path.target_id == outcome
        ]
        outcome_interactions = [
            interaction
            for interaction in interactions
            if interaction.outcome_id == outcome
        ]
        product_columns: dict[str, list[float]] = {}
        for interaction in outcome_interactions:
            raw = [
                left * right
                for left, right in zip(
                    fit.scores[interaction.focal_id],
                    fit.scores[interaction.moderator_id],
                    strict=True,
                )
            ]
            product_means[interaction.interaction_id] = mean(raw)
            product_scales[interaction.interaction_id] = sample_sd(raw)
            product_columns[interaction.interaction_id] = standardize(raw)
        columns = [fit.scores[source] for source in predecessors] + [
            product_columns[interaction.interaction_id]
            for interaction in outcome_interactions
        ]
        coefficients = _ols(columns, fit.scores[outcome])
        for source, coefficient in zip(
            predecessors, coefficients[: len(predecessors)], strict=True
        ):
            direct[(source, outcome)] = coefficient
        for interaction, coefficient in zip(
            outcome_interactions,
            coefficients[len(predecessors) :],
            strict=True,
        ):
            interaction_id = interaction.interaction_id
            product_coefficients[interaction_id] = coefficient
            gamma = coefficient / product_scales[interaction_id]
            gammas[interaction_id] = gamma
            focal = direct[(interaction.focal_id, outcome)]
            slopes[interaction_id] = tuple(
                focal + gamma * probe for probe in FIXED_PROBES
            )
    return ModerationFit(
        direct_coefficients=direct,
        standardized_product_coefficients=product_coefficients,
        scientific_gammas=gammas,
        product_means=product_means,
        product_scales=product_scales,
        fixed_probe_slopes=slopes,
    )


def _replicate_indices(rows: int, seed: int, replicate_index: int) -> tuple[int, ...]:
    material = (
        f"{BOOTSTRAP_STREAM_VERSION}|{seed}|{replicate_index}".encode("utf-8")
    )
    indexed_seed = int.from_bytes(hashlib.sha256(material).digest()[:16], "big")
    generator = random.Random(indexed_seed)
    return tuple(generator.randrange(rows) for _ in range(rows))


def _evaluation_order(
    requested: int, evaluation_order: Sequence[int] | None
) -> tuple[int, ...]:
    order = tuple(range(requested)) if evaluation_order is None else tuple(evaluation_order)
    if sorted(order) != list(range(requested)):
        raise ModelContractError("evaluation order must contain every replicate exactly once")
    return order


def _validated_index_plan(
    *,
    requested: int,
    rows: int,
    seed: int,
    index_plan: Sequence[Sequence[int]] | None,
) -> tuple[tuple[int, ...], ...]:
    """Resolve an exact external case plan or the oracle's independent stream."""

    if index_plan is None:
        return tuple(_replicate_indices(rows, seed, index) for index in range(requested))
    resolved = tuple(tuple(indices) for indices in index_plan)
    if len(resolved) != requested:
        raise ModelContractError("bootstrap index plan count differs from requested")
    if any(
        len(indices) != rows
        or any(type(index) is not int or index < 0 or index >= rows for index in indices)
        for indices in resolved
    ):
        raise ModelContractError("bootstrap index plan leaves the complete-case frame")
    return resolved


def _summary(original: float, values: Sequence[float], confidence: float) -> BootstrapSummary:
    center = mean(values)
    alpha = 1.0 - confidence
    exceedances, probability = plus_one_two_sided(original, values)
    return BootstrapSummary(
        original=original,
        mean=center,
        bias=center - original,
        standard_error=sample_standard_error(values),
        lower=type7(values, alpha / 2.0),
        upper=type7(values, 1.0 - alpha / 2.0),
        exceedances=exceedances,
        plus_one_two_sided_probability=probability,
    )


def summarize_bootstrap_distributions(
    *,
    requested: int,
    originals: Mapping[str, float],
    distributions: Mapping[str, Sequence[float]],
    usable_indices: Sequence[int],
    failures: Sequence[BootstrapFailure],
    sign_corrections: int = 0,
    confidence: float = 0.95,
) -> BootstrapResult:
    minimum = minimum_usable_replicates(requested)
    usable = len(usable_indices)
    if usable + len(failures) != requested:
        raise NumericalOracleError("usable and failed replicate counts are incoherent")
    if set(distributions) != set(originals):
        raise NumericalOracleError("bootstrap target identities drifted")
    if any(len(values) != usable for values in distributions.values()):
        raise NumericalOracleError("bootstrap target distributions differ in length")
    published = usable >= minimum
    summaries = (
        {
            identifier: _summary(original, distributions[identifier], confidence)
            for identifier, original in originals.items()
        }
        if published
        else {}
    )
    return BootstrapResult(
        requested=requested,
        usable=usable,
        minimum_usable=minimum,
        published=published,
        summaries=summaries,
        failures=tuple(sorted(failures, key=lambda row: row.replicate_index)),
        usable_indices=tuple(sorted(usable_indices)),
        sign_corrections=sign_corrections,
    )


def bootstrap_mediation(
    rows: Sequence[Mapping[str, float | int | None]],
    model: PlsModelSpec,
    source_id: str,
    target_id: str,
    *,
    requested: int,
    seed: int,
    confidence: float = 0.95,
    evaluation_order: Sequence[int] | None = None,
    index_plan: Sequence[Sequence[int]] | None = None,
) -> BootstrapResult:
    """Run indexed full-PLS refits and recompute every mediation effect."""

    model = canonicalize_model(model)
    minimum_usable_replicates(requested)
    original_fit = fit_pls_pm(rows, model)
    originals = mediation_effects(original_fit, model, source_id, target_id)
    complete_frame = [rows[index] for index in original_fit.used_row_indices]
    resolved_index_plan = _validated_index_plan(
        requested=requested,
        rows=len(complete_frame),
        seed=seed,
        index_plan=index_plan,
    )
    replicate_values: dict[int, Mapping[str, float]] = {}
    usable_indices: list[int] = []
    failures: list[BootstrapFailure] = []
    sign_corrections = 0
    for replicate_index in _evaluation_order(requested, evaluation_order):
        indices = resolved_index_plan[replicate_index]
        sampled = [complete_frame[index] for index in indices]
        try:
            refit = fit_pls_pm(sampled, model)
            aligned, corrected = align_fit_to_reference(
                refit, original_fit, indices, model
            )
            effects = mediation_effects(aligned, model, source_id, target_id)
            if set(effects) != set(originals):
                raise NumericalOracleError("mediation effect identities drifted")
            replicate_values[replicate_index] = effects
            usable_indices.append(replicate_index)
            sign_corrections += corrected
        except OracleError as error:
            failures.append(BootstrapFailure(replicate_index, str(error)))
    ordered_usable = sorted(usable_indices)
    distributions = {
        key: [replicate_values[index][key] for index in ordered_usable]
        for key in originals
    }
    return summarize_bootstrap_distributions(
        requested=requested,
        originals=originals,
        distributions=distributions,
        usable_indices=usable_indices,
        failures=failures,
        sign_corrections=sign_corrections,
        confidence=confidence,
    )


def bootstrap_moderation(
    rows: Sequence[Mapping[str, float | int | None]],
    model: PlsModelSpec,
    interactions: Sequence[InteractionSpec],
    *,
    requested: int,
    seed: int,
    confidence: float = 0.95,
    evaluation_order: Sequence[int] | None = None,
    index_plan: Sequence[Sequence[int]] | None = None,
) -> BootstrapResult:
    """Run indexed full-PLS refits and rebuild products before gamma inference."""

    model = canonicalize_model(model)
    interactions = canonicalize_interactions(interactions)
    minimum_usable_replicates(requested)
    original_fit = fit_pls_pm(rows, model)
    original_moderation = fit_simultaneous_moderation(
        original_fit, model, interactions
    )
    originals = dict(original_moderation.scientific_gammas)
    complete_frame = [rows[index] for index in original_fit.used_row_indices]
    resolved_index_plan = _validated_index_plan(
        requested=requested,
        rows=len(complete_frame),
        seed=seed,
        index_plan=index_plan,
    )
    replicate_values: dict[int, Mapping[str, float]] = {}
    usable_indices: list[int] = []
    failures: list[BootstrapFailure] = []
    sign_corrections = 0
    for replicate_index in _evaluation_order(requested, evaluation_order):
        indices = resolved_index_plan[replicate_index]
        sampled = [complete_frame[index] for index in indices]
        try:
            refit = fit_pls_pm(sampled, model)
            aligned, corrected = align_fit_to_reference(
                refit, original_fit, indices, model
            )
            moderation = fit_simultaneous_moderation(aligned, model, interactions)
            if set(moderation.scientific_gammas) != set(originals):
                raise NumericalOracleError("moderation target identities drifted")
            replicate_values[replicate_index] = moderation.scientific_gammas
            usable_indices.append(replicate_index)
            sign_corrections += corrected
        except OracleError as error:
            failures.append(BootstrapFailure(replicate_index, str(error)))
    ordered_usable = sorted(usable_indices)
    distributions = {
        key: [replicate_values[index][key] for index in ordered_usable]
        for key in originals
    }
    return summarize_bootstrap_distributions(
        requested=requested,
        originals=originals,
        distributions=distributions,
        usable_indices=usable_indices,
        failures=failures,
        sign_corrections=sign_corrections,
        confidence=confidence,
    )


__all__ = [
    "BOOTSTRAP_STREAM_VERSION",
    "BlockSpec",
    "BootstrapFailure",
    "BootstrapResult",
    "BootstrapSummary",
    "InteractionSpec",
    "ModelContractError",
    "ModerationFit",
    "NumericalOracleError",
    "OracleError",
    "PathSpec",
    "PlsFit",
    "PlsModelSpec",
    "align_fit_to_reference",
    "bootstrap_mediation",
    "bootstrap_moderation",
    "canonicalize_interactions",
    "canonicalize_model",
    "correlation",
    "directed_paths",
    "fit_pls_pm",
    "fit_simultaneous_moderation",
    "mean",
    "mediation_effects",
    "minimum_usable_replicates",
    "plus_one_two_sided",
    "sample_sd",
    "sample_standard_error",
    "standardize",
    "summarize_bootstrap_distributions",
    "type7",
]
