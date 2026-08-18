#!/usr/bin/env python3
"""Transparent NumPy work oracle for the bounded PLS-PM v1 kernel.

This module is intentionally independent of QuickPLS product code.  It freezes
the equations used for source-level microcases, deterministic metamorphics, and
typed-boundary work.  Passing this oracle does not compare a current QuickPLS
build, qualify the product, or authorize a registry/manifest state change.
"""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import asdict, dataclass, is_dataclass
from pathlib import Path
from typing import Any, Sequence

import numpy as np


ORACLE_VERSION = "pls_pm_numpy_work_oracle_v1"
DEFAULT_TOLERANCE = 1.0e-7
DEFAULT_MAX_ITERATIONS = 3_000
NUMERIC_EPSILON = 1.0e-14


class PlsOracleFailure(ValueError):
    """Typed, deterministic rejection from the transparent work oracle."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code


@dataclass(frozen=True)
class OracleConstruct:
    construct_id: str
    indicators: tuple[str, ...]
    mode: str = "mode_a"


@dataclass(frozen=True)
class OraclePath:
    source: str
    target: str


def _canonical_sha256(value: Any) -> str:
    def encode(value_to_encode: Any) -> Any:
        if is_dataclass(value_to_encode):
            return asdict(value_to_encode)
        raise TypeError(f"unsupported canonical value: {type(value_to_encode).__name__}")

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


def _standardize_vector(values: np.ndarray) -> np.ndarray:
    centered = values - float(np.mean(values))
    scale = _sample_sd(centered)
    if not math.isfinite(scale) or scale <= NUMERIC_EPSILON:
        raise PlsOracleFailure(
            "pls_zero_variance_proxy",
            "a construct proxy has zero or non-finite sample variance",
        )
    return centered / scale


def _correlation(left: np.ndarray, right: np.ndarray) -> float:
    if len(left) != len(right) or len(left) < 3:
        raise PlsOracleFailure(
            "pls_invalid_correlation_sample",
            "correlation inputs must have the same length and at least three rows",
        )
    return float(
        np.dot(_standardize_vector(left), _standardize_vector(right))
        / (len(left) - 1)
    )


def _preprocess(matrix: np.ndarray, policy: str) -> np.ndarray:
    centered = matrix - matrix.mean(axis=0)
    if policy == "mean_centered":
        return centered
    if policy == "unstandardized":
        return matrix.copy()
    if policy != "standardized":
        raise PlsOracleFailure(
            "pls_preprocessing_unsupported", f"unsupported preprocessing: {policy}"
        )
    scale = matrix.std(axis=0, ddof=1)
    if np.any(~np.isfinite(scale)) or np.any(scale <= NUMERIC_EPSILON):
        raise PlsOracleFailure(
            "pls_constant_indicator",
            "every used indicator must have finite non-zero sample variance",
        )
    return centered / scale


def _oriented_unit_score_weight(block: np.ndarray, weight: np.ndarray) -> np.ndarray:
    if np.any(~np.isfinite(weight)):
        raise PlsOracleFailure("pls_non_finite_weight", "outer weight is non-finite")
    score_scale = _sample_sd(block @ weight)
    if not math.isfinite(score_scale) or score_scale <= NUMERIC_EPSILON:
        raise PlsOracleFailure(
            "pls_zero_variance_proxy",
            "outer weights produce a zero-variance construct proxy",
        )
    oriented = weight / score_scale
    for value in oriented:
        if abs(float(value)) > NUMERIC_EPSILON:
            return -oriented if value < 0 else oriented
    raise PlsOracleFailure("pls_zero_weight_vector", "outer weight vector is zero")


def _pca_weight(block: np.ndarray) -> np.ndarray:
    centered = block - block.mean(axis=0)
    covariance = centered.T @ centered / (len(block) - 1)
    eigenvalues, eigenvectors = np.linalg.eigh(covariance)
    weight = eigenvectors[:, int(np.argmax(eigenvalues))]
    score = block @ weight
    unit_reference = block @ np.ones(block.shape[1])
    association = float(
        np.dot(score - score.mean(), unit_reference - unit_reference.mean())
        / (len(block) - 1)
    )
    if association < -1.0e-15 or (
        abs(association) <= 1.0e-15 and float(weight.sum()) < 0
    ):
        weight = -weight
    return _oriented_unit_score_weight(block, weight)


def _validate_contract(
    rows: Sequence[Sequence[float | None]],
    variables: Sequence[str],
    constructs: Sequence[OracleConstruct],
    paths: Sequence[OraclePath],
    *,
    weighting_scheme: str,
    tolerance: float,
    max_iterations: int,
) -> tuple[np.ndarray, list[int], dict[str, int]]:
    if weighting_scheme not in {"path", "factor", "pca"}:
        raise PlsOracleFailure(
            "pls_weighting_scheme_unsupported", weighting_scheme
        )
    if not math.isfinite(tolerance) or tolerance <= 0:
        raise PlsOracleFailure("pls_tolerance_invalid", "tolerance must be positive")
    if max_iterations < 1:
        raise PlsOracleFailure(
            "pls_max_iterations_invalid", "max_iterations must be at least one"
        )
    if len(set(variables)) != len(variables):
        raise PlsOracleFailure("pls_duplicate_variable", "variable identifiers must be unique")
    if not constructs:
        raise PlsOracleFailure("pls_empty_model", "at least one construct is required")
    construct_ids = [row.construct_id for row in constructs]
    if len(set(construct_ids)) != len(construct_ids):
        raise PlsOracleFailure(
            "pls_duplicate_construct", "construct identifiers must be unique"
        )
    assigned: set[str] = set()
    variable_set = set(variables)
    for construct in constructs:
        if construct.mode not in {"mode_a", "mode_b"}:
            raise PlsOracleFailure(
                "pls_measurement_mode_unsupported", construct.mode
            )
        if not construct.indicators:
            raise PlsOracleFailure(
                "pls_empty_measurement_block", construct.construct_id
            )
        for indicator in construct.indicators:
            if indicator not in variable_set:
                raise PlsOracleFailure("pls_unknown_indicator", indicator)
            if indicator in assigned:
                raise PlsOracleFailure("pls_duplicate_assignment", indicator)
            assigned.add(indicator)

    index = {identifier: position for position, identifier in enumerate(construct_ids)}
    seen_paths: set[tuple[str, str]] = set()
    successors: list[list[int]] = [[] for _ in constructs]
    for path in paths:
        if path.source not in index or path.target not in index:
            raise PlsOracleFailure(
                "pls_unknown_path_construct", f"{path.source}->{path.target}"
            )
        if path.source == path.target:
            raise PlsOracleFailure("pls_cycle", f"self-path {path.source}->{path.target}")
        key = (path.source, path.target)
        if key in seen_paths:
            raise PlsOracleFailure("pls_duplicate_path", f"{path.source}->{path.target}")
        seen_paths.add(key)
        successors[index[path.source]].append(index[path.target])

    visiting: set[int] = set()
    visited: set[int] = set()

    def visit(node: int) -> None:
        if node in visiting:
            raise PlsOracleFailure("pls_cycle", "structural graph must be recursive")
        if node in visited:
            return
        visiting.add(node)
        for target in successors[node]:
            visit(target)
        visiting.remove(node)
        visited.add(node)

    for construct_index in range(len(constructs)):
        visit(construct_index)

    complete_indices: list[int] = []
    complete_rows: list[list[float]] = []
    for row_index, row in enumerate(rows):
        if len(row) != len(variables):
            raise PlsOracleFailure(
                "pls_row_width_mismatch", f"row {row_index} has the wrong width"
            )
        if any(value is None for value in row):
            continue
        converted = [float(value) for value in row]
        if any(not math.isfinite(value) for value in converted):
            raise PlsOracleFailure(
                "pls_non_finite_input", f"row {row_index} contains a non-finite value"
            )
        complete_indices.append(row_index)
        complete_rows.append(converted)
    if len(complete_rows) < 3:
        raise PlsOracleFailure(
            "pls_insufficient_rows", "at least three complete rows are required"
        )
    matrix = np.asarray(complete_rows, dtype=float)
    return matrix, complete_indices, index


def estimate_pls(
    rows: Sequence[Sequence[float | None]],
    variables: Sequence[str],
    constructs: Sequence[OracleConstruct],
    paths: Sequence[OraclePath],
    *,
    weighting_scheme: str = "path",
    preprocessing: str = "standardized",
    tolerance: float = DEFAULT_TOLERANCE,
    max_iterations: int = DEFAULT_MAX_ITERATIONS,
) -> dict[str, Any]:
    """Estimate the bounded recursive composite PLS contract."""

    matrix, complete_indices, construct_index = _validate_contract(
        rows,
        variables,
        constructs,
        paths,
        weighting_scheme=weighting_scheme,
        tolerance=tolerance,
        max_iterations=max_iterations,
    )
    transformed = _preprocess(matrix, preprocessing)
    variable_index = {name: index for index, name in enumerate(variables)}
    blocks = [
        transformed[:, [variable_index[name] for name in construct.indicators]]
        for construct in constructs
    ]
    predecessors: list[list[int]] = [[] for _ in constructs]
    successors: list[list[int]] = [[] for _ in constructs]
    for path in paths:
        source = construct_index[path.source]
        target = construct_index[path.target]
        predecessors[target].append(source)
        successors[source].append(target)

    if weighting_scheme == "pca":
        weights = [_pca_weight(block) for block in blocks]
        iterations = 1
        final_change = 0.0
    else:
        weights = [
            _oriented_unit_score_weight(block, np.ones(block.shape[1]))
            for block in blocks
        ]
        final_change = math.inf
        for iterations in range(1, max_iterations + 1):
            scores = np.column_stack(
                [
                    _standardize_vector(block @ weight)
                    for block, weight in zip(blocks, weights)
                ]
            )
            inner = np.zeros_like(scores)
            for target in range(len(constructs)):
                adjacent = predecessors[target] + successors[target]
                if not adjacent:
                    inner[:, target] = scores[:, target]
                elif weighting_scheme == "factor":
                    for neighbor in adjacent:
                        inner[:, target] += (
                            _correlation(scores[:, target], scores[:, neighbor])
                            * scores[:, neighbor]
                        )
                else:
                    if predecessors[target]:
                        design = scores[:, predecessors[target]]
                        coefficients, _residuals, rank, _singular = np.linalg.lstsq(
                            design, scores[:, target], rcond=None
                        )
                        if rank < design.shape[1]:
                            raise PlsOracleFailure(
                                "pls_rank_deficient_structural",
                                constructs[target].construct_id,
                            )
                        inner[:, target] += design @ coefficients
                    for neighbor in successors[target]:
                        inner[:, target] += (
                            _correlation(scores[:, target], scores[:, neighbor])
                            * scores[:, neighbor]
                        )
                inner[:, target] = _standardize_vector(inner[:, target])

            updated: list[np.ndarray] = []
            for construct, block, proxy in zip(constructs, blocks, inner.T):
                centered = block - block.mean(axis=0)
                if construct.mode == "mode_a":
                    weight = centered.T @ proxy / (len(block) - 1)
                else:
                    weight, _residuals, rank, _singular = np.linalg.lstsq(
                        centered, proxy, rcond=None
                    )
                    if rank < centered.shape[1]:
                        raise PlsOracleFailure(
                            "pls_rank_deficient_mode_b", construct.construct_id
                        )
                updated.append(_oriented_unit_score_weight(block, weight))
            final_change = max(
                float(np.max(np.abs(before - after)))
                for before, after in zip(weights, updated)
            )
            weights = updated
            if final_change <= tolerance:
                break
        else:
            raise PlsOracleFailure(
                "pls_non_convergence",
                f"outer weights did not converge in {max_iterations} iterations",
            )

    scores = np.column_stack(
        [
            _standardize_vector(block @ weight)
            for block, weight in zip(blocks, weights)
        ]
    )
    path_rows: list[dict[str, Any]] = []
    r_squared: dict[str, float] = {}
    direct = np.zeros((len(constructs), len(constructs)))
    for target, sources in enumerate(predecessors):
        if not sources:
            continue
        design = scores[:, sources]
        coefficients, _residuals, rank, _singular = np.linalg.lstsq(
            design, scores[:, target], rcond=None
        )
        if rank < design.shape[1]:
            raise PlsOracleFailure(
                "pls_rank_deficient_structural", constructs[target].construct_id
            )
        fitted = design @ coefficients
        residual = scores[:, target] - fitted
        denominator = float(np.dot(scores[:, target], scores[:, target]))
        r_squared[constructs[target].construct_id] = float(
            1.0 - np.dot(residual, residual) / denominator
        )
        for source, coefficient in zip(sources, coefficients):
            direct[target, source] = coefficient
            path_rows.append(
                {
                    "source": constructs[source].construct_id,
                    "target": constructs[target].construct_id,
                    "coefficient": float(coefficient),
                }
            )

    total = np.zeros_like(direct)
    power = direct.copy()
    for _ in range(1, len(constructs)):
        total += power
        power = power @ direct

    outer: list[dict[str, Any]] = []
    for construct, block, weight, score in zip(constructs, blocks, weights, scores.T):
        for position, indicator in enumerate(construct.indicators):
            outer.append(
                {
                    "construct": construct.construct_id,
                    "indicator": indicator,
                    "loading": _correlation(block[:, position], score),
                    "weight": float(weight[position]),
                }
            )
    return {
        "oracle_version": ORACLE_VERSION,
        "converged": True,
        "iterations": iterations,
        "final_max_abs_weight_change": final_change,
        "used_observations": len(complete_indices),
        "omitted_observations": len(rows) - len(complete_indices),
        "complete_indices": complete_indices,
        "construct_order": [row.construct_id for row in constructs],
        "paths": path_rows,
        "outer_estimates": outer,
        "r_squared": r_squared,
        "construct_scores": {
            construct.construct_id: [float(value) for value in scores[:, index]]
            for index, construct in enumerate(constructs)
        },
        "direct_effect_matrix": direct.tolist(),
        "total_effect_matrix": total.tolist(),
    }


def _fixture(seed: int = 20_260_815) -> dict[str, Any]:
    rng = np.random.default_rng(seed)
    observations = 320
    x = rng.normal(size=observations)
    m = 0.62 * x + rng.normal(scale=0.72, size=observations)
    y = 0.24 * x + 0.57 * m + rng.normal(scale=0.65, size=observations)
    latent = (x, m, y)
    variables: list[str] = []
    columns: list[np.ndarray] = []
    constructs: list[OracleConstruct] = []
    for construct_index, (identifier, score) in enumerate(zip(("x", "m", "y"), latent)):
        indicators: list[str] = []
        for indicator_index, loading in enumerate((0.90, 0.82, 0.76)):
            name = f"{identifier}{indicator_index + 1}"
            noise = rng.normal(size=observations)
            column = loading * _standardize_vector(score) + math.sqrt(1 - loading**2) * noise
            variables.append(name)
            indicators.append(name)
            columns.append(column)
        constructs.append(OracleConstruct(identifier, tuple(indicators), "mode_a"))
    return {
        "rows": np.column_stack(columns).tolist(),
        "variables": variables,
        "constructs": constructs,
        "paths": [OraclePath("x", "m"), OraclePath("x", "y"), OraclePath("m", "y")],
    }


def _solution_map(result: dict[str, Any]) -> dict[str, Any]:
    return {
        "paths": {
            f"{row['source']}->{row['target']}": row["coefficient"]
            for row in result["paths"]
        },
        "outer": {
            f"{row['construct']}::{row['indicator']}": {
                "loading": row["loading"],
                "weight": row["weight"],
            }
            for row in result["outer_estimates"]
        },
        "r_squared": result["r_squared"],
        "scores": result["construct_scores"],
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


def _assert_failure(code: str, operation: Any) -> bool:
    try:
        operation()
    except PlsOracleFailure as error:
        return error.code == code
    return False


def run_work_checks() -> dict[str, Any]:
    fixture = _fixture()
    baseline = estimate_pls(**fixture)
    baseline_map = _solution_map(baseline)
    repeat = estimate_pls(**fixture)

    row_order = list(reversed(range(len(fixture["rows"]))))
    reordered_rows = [fixture["rows"][index] for index in row_order]
    row_result = estimate_pls(
        reordered_rows,
        fixture["variables"],
        fixture["constructs"],
        fixture["paths"],
    )
    row_map = _solution_map(row_result)
    for construct_id, values in row_map["scores"].items():
        restored = [0.0] * len(values)
        for new_index, original_index in enumerate(row_order):
            restored[original_index] = values[new_index]
        row_map["scores"][construct_id] = restored

    variable_order = list(reversed(range(len(fixture["variables"]))))
    variable_result = estimate_pls(
        [[row[index] for index in variable_order] for row in fixture["rows"]],
        [fixture["variables"][index] for index in variable_order],
        fixture["constructs"],
        fixture["paths"],
    )

    construct_result = estimate_pls(
        fixture["rows"],
        fixture["variables"],
        list(reversed(fixture["constructs"])),
        list(reversed(fixture["paths"])),
    )

    scales = np.linspace(0.7, 2.3, len(fixture["variables"]))
    offsets = np.linspace(-3.0, 4.0, len(fixture["variables"]))
    affine_rows = [
        [float(value * scales[index] + offsets[index]) for index, value in enumerate(row)]
        for row in fixture["rows"]
    ]
    affine_result = estimate_pls(
        affine_rows,
        fixture["variables"],
        fixture["constructs"],
        fixture["paths"],
    )

    irrelevant_rng = np.random.default_rng(991_731)
    irrelevant_result = estimate_pls(
        [
            [*row, float(extra)]
            for row, extra in zip(fixture["rows"], irrelevant_rng.normal(size=len(fixture["rows"])))
        ],
        [*fixture["variables"], "irrelevant"],
        fixture["constructs"],
        fixture["paths"],
    )

    hand_rows = [[1.0, 2.0], [2.0, 2.5], [3.0, 4.5], [5.0, 7.0], [8.0, 9.0]]
    hand = estimate_pls(
        hand_rows,
        ["x1", "y1"],
        [OracleConstruct("x", ("x1",)), OracleConstruct("y", ("y1",))],
        [OraclePath("x", "y")],
    )
    hand_expected = _correlation(
        np.asarray([row[0] for row in hand_rows]),
        np.asarray([row[1] for row in hand_rows]),
    )
    hand_observed = hand["paths"][0]["coefficient"]

    rank_deficient_rows = [
        [1.0, 1.0, 1.0],
        [2.0, 2.0, 2.0],
        [3.0, 3.0, 2.0],
        [4.0, 4.0, 5.0],
    ]
    typed_boundaries = {
        "empty_model": _assert_failure(
            "pls_empty_model", lambda: estimate_pls(hand_rows, ["x1", "y1"], [], [])
        ),
        "insufficient_rows": _assert_failure(
            "pls_insufficient_rows",
            lambda: estimate_pls(
                hand_rows[:2],
                ["x1", "y1"],
                [OracleConstruct("x", ("x1",)), OracleConstruct("y", ("y1",))],
                [OraclePath("x", "y")],
            ),
        ),
        "constant_indicator": _assert_failure(
            "pls_constant_indicator",
            lambda: estimate_pls(
                [[1.0, 2.0], [1.0, 3.0], [1.0, 5.0]],
                ["x1", "y1"],
                [OracleConstruct("x", ("x1",)), OracleConstruct("y", ("y1",))],
                [OraclePath("x", "y")],
            ),
        ),
        "cycle": _assert_failure(
            "pls_cycle",
            lambda: estimate_pls(
                hand_rows,
                ["x1", "y1"],
                [OracleConstruct("x", ("x1",)), OracleConstruct("y", ("y1",))],
                [OraclePath("x", "y"), OraclePath("y", "x")],
            ),
        ),
        "mode_b_rank": _assert_failure(
            "pls_rank_deficient_mode_b",
            lambda: estimate_pls(
                rank_deficient_rows,
                ["x1", "x2", "y1"],
                [
                    OracleConstruct("x", ("x1", "x2"), "mode_b"),
                    OracleConstruct("y", ("y1",)),
                ],
                [OraclePath("x", "y")],
            ),
        ),
        "nonconvergence": _assert_failure(
            "pls_non_convergence", lambda: estimate_pls(**fixture, max_iterations=1)
        ),
    }

    comparisons = {
        "same_input_repeat": _maximum_numeric_difference(
            baseline_map, _solution_map(repeat)
        ),
        "row_reorder": _maximum_numeric_difference(baseline_map, row_map),
        "variable_reorder": _maximum_numeric_difference(
            baseline_map, _solution_map(variable_result)
        ),
        "construct_and_path_declaration_reorder": _maximum_numeric_difference(
            baseline_map, _solution_map(construct_result)
        ),
        "positive_affine_rescaling": _maximum_numeric_difference(
            baseline_map, _solution_map(affine_result)
        ),
        "irrelevant_variable_addition": _maximum_numeric_difference(
            baseline_map, _solution_map(irrelevant_result)
        ),
    }
    tolerance = 2.0e-10
    checks = {
        "hand_single_item_path_equals_correlation": abs(hand_observed - hand_expected)
        <= 1.0e-12,
        "default_stop_criterion_is_1e_7": DEFAULT_TOLERANCE == 1.0e-7,
        "default_iteration_limit_is_3000": DEFAULT_MAX_ITERATIONS == 3_000,
        "deterministic_orientation_anchor_positive": all(
            next(
                row["weight"]
                for row in baseline["outer_estimates"]
                if row["construct"] == construct.construct_id
            )
            > 0
            for construct in fixture["constructs"]
        ),
        "metamorphics_within_tolerance": all(
            difference <= tolerance for difference in comparisons.values()
        ),
        "typed_boundaries_exact": all(typed_boundaries.values()),
        "same_seed_fixture_repeat_exact": _canonical_sha256(_fixture())
        == _canonical_sha256(_fixture()),
        "different_seed_changes_fixture": _canonical_sha256(_fixture())
        != _canonical_sha256(_fixture(seed=20_260_816)),
    }
    return {
        "schema_version": 1,
        "report_kind": "pls_algorithm_v1_current_source_oracle_work_v1",
        "oracle_version": ORACLE_VERSION,
        "passed": all(checks.values()),
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_requested": False,
        "checks": checks,
        "metamorphic_tolerance": tolerance,
        "metamorphic_max_abs_differences": comparisons,
        "typed_boundaries": typed_boundaries,
        "hand_microcase": {
            "expected_correlation": hand_expected,
            "observed_path": hand_observed,
            "absolute_difference": abs(hand_observed - hand_expected),
        },
        "remaining_blockers": [
            "No current QuickPLS product build was executed or compared.",
            "The transparent NumPy implementation is work evidence, not an independently maintained product oracle receipt.",
            "Qualification-sized simulation, persistence, export, packaged Windows, performance, and scientific-review evidence is absent.",
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
    report = run_work_checks()
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
