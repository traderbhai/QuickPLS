#!/usr/bin/env python3
"""Compact independent QuickPLS 2.53 mediation/moderation reference.

The reference is intentionally standard-library-only and never imports the
QuickPLS implementation.  It freezes the observed-score arithmetic needed to
check the new bounded three-way point/bootstrap cells and the distinct
single-path mediation bootstrap identity without repeating the historical
qualification matrices.

By default an available ``Rscript`` is used as a second, base-R QR point-fit
oracle.  Use ``--require-r`` at the promotion gate or ``--skip-r`` on a machine
without R.  The case-bootstrap stream is an explicit independent SHA-256
indexed stream; it is a replay oracle, not an implementation of Rust's RNG.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import re
import shutil
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterable, Mapping, Sequence


TOLERANCE = 1.0e-10
PRODUCT_TOLERANCE = 5.0e-9
RESAMPLES = 79
CONFIDENCE_LEVEL = 0.95
SEED = 20_260_821
REFERENCE_STREAM = "qpls_v253_independent_sha256_indexed_case_stream_v1"
PRODUCTION_STREAM = "indexed_case_resampling_v1"

SINGLE_MEDIATION = {
    "capability_id": "smartpls.mediation",
    "cell_id": "qpls3.pls.general_sem_single_mediation_bootstrap",
    "capability_version": "general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
    "method_version": "general_sem_pls_single_mediation_full_model_case_bootstrap_v1",
    "operation_version": "general_sem_pls_single_mediation_case_bootstrap_v1",
}
THREE_WAY_POINT = {
    "capability_id": "smartpls.moderation",
    "cell_id": "qpls3.pls.general_sem_three_way_moderation_point",
    "capability_version": "general_sem_pls_three_way_moderation_point_v1",
    "method_version": "qpls.general-sem-pls.three-way.point.v1",
}
THREE_WAY_BOOTSTRAP = {
    "capability_id": "smartpls.moderation",
    "cell_id": "qpls3.pls.general_sem_three_way_moderation_bootstrap",
    "capability_version": "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1",
    "method_version": "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1",
    "operation_version": "general_sem_pls_three_way_moderation_case_bootstrap_v1",
}


def _cell_identity(row: Mapping[str, str]) -> str:
    return (
        f"capability_registry_v2:{row['capability_id']}:"
        f"{row['cell_id']}:{row['capability_version']}"
    )


def _mean(values: Sequence[float]) -> float:
    if not values:
        raise ValueError("mean requires observations")
    return math.fsum(values) / len(values)


def _sample_sd(values: Sequence[float]) -> float:
    if len(values) < 2:
        raise ValueError("sample standard deviation requires two observations")
    center = _mean(values)
    value = math.sqrt(
        math.fsum((item - center) ** 2 for item in values) / (len(values) - 1)
    )
    if not math.isfinite(value) or value <= 1.0e-14:
        raise ValueError("column is constant or nonfinite")
    return value


def _standardize(values: Sequence[float]) -> list[float]:
    if any(not math.isfinite(item) for item in values):
        raise ValueError("column is nonfinite")
    center = _mean(values)
    scale = _sample_sd(values)
    return [(item - center) / scale for item in values]


def _standardized_product(*columns: Sequence[float]) -> tuple[list[float], float]:
    raw = [math.prod(items) for items in zip(*columns, strict=True)]
    scale = _sample_sd(raw)
    return _standardize(raw), scale


def _solve(matrix: Sequence[Sequence[float]], vector: Sequence[float]) -> list[float]:
    size = len(vector)
    if size == 0 or len(matrix) != size or any(len(row) != size for row in matrix):
        raise ValueError("invalid linear-system dimensions")
    augmented = [list(row) + [value] for row, value in zip(matrix, vector, strict=True)]
    for column in range(size):
        pivot = max(range(column, size), key=lambda row: abs(augmented[row][column]))
        if abs(augmented[pivot][column]) <= 1.0e-12:
            raise ValueError("joint equation is singular")
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        divisor = augmented[column][column]
        augmented[column] = [value / divisor for value in augmented[column]]
        for row in range(size):
            if row == column:
                continue
            factor = augmented[row][column]
            augmented[row] = [
                value - factor * basis
                for value, basis in zip(augmented[row], augmented[column], strict=True)
            ]
    solution = [augmented[row][-1] for row in range(size)]
    if any(not math.isfinite(value) for value in solution):
        raise ValueError("joint equation produced a nonfinite coefficient")
    return solution


def _fit(columns: Sequence[Sequence[float]], outcome: Sequence[float]) -> list[float]:
    if not columns or any(len(column) != len(outcome) for column in columns):
        raise ValueError("invalid regression dimensions")
    width = len(columns)
    gram = [
        [
            math.fsum(a * b for a, b in zip(columns[row], columns[column], strict=True))
            for column in range(width)
        ]
        for row in range(width)
    ]
    rhs = [
        math.fsum(value * target for value, target in zip(column, outcome, strict=True))
        for column in columns
    ]
    return _solve(gram, rhs)


def _max_error(left: Iterable[float], right: Iterable[float]) -> float:
    pairs = list(zip(left, right, strict=True))
    return max((abs(a - b) for a, b in pairs), default=0.0)


def _type7(values: Sequence[float], probability: float) -> float:
    if not values or not 0.0 <= probability <= 1.0:
        raise ValueError("invalid Type-7 quantile input")
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    fraction = position - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def _positions(operation: str, row_count: int, replicate: int) -> list[int]:
    positions: list[int] = []
    for draw in range(row_count):
        token = (
            f"{REFERENCE_STREAM}|{operation}|{SEED}|{replicate}|{draw}|{row_count}"
        ).encode("utf-8")
        positions.append(int.from_bytes(hashlib.sha256(token).digest()[:8], "big") % row_count)
    return positions


@dataclass(frozen=True)
class ThreeWayFixture:
    scenario_id: str
    raw: Mapping[str, tuple[float, ...]]
    first_probe_kind: str
    second_probe_kind: str


@dataclass(frozen=True)
class ThreeWayFit:
    coefficients: tuple[float, ...]
    pairwise_gammas: tuple[float, float, float]
    delta: float
    first_probes: tuple[tuple[float, float], ...]
    second_probes: tuple[tuple[float, float], ...]
    conditional_interactions: tuple[float, ...]
    simple_slopes: tuple[float, ...]
    design: tuple[tuple[float, ...], ...]
    outcome: tuple[float, ...]


def _probe_values(raw: Sequence[float], standardized: Sequence[float], kind: str) -> tuple[tuple[float, float], ...]:
    if kind == "continuous_standardized":
        return ((-1.0, -1.0), (0.0, 0.0), (1.0, 1.0))
    if kind != "binary_zero_one" or set(raw) != {0.0, 1.0}:
        raise ValueError("binary moderator must contain exact 0/1 categories")
    pairs = sorted({(raw_value, score) for raw_value, score in zip(raw, standardized, strict=True)})
    if len(pairs) != 2:
        raise ValueError("binary moderator lost a category")
    return tuple(pairs)


def _fit_three_way(fixture: ThreeWayFixture, positions: Sequence[int] | None = None) -> ThreeWayFit:
    if positions is None:
        positions = range(len(fixture.raw["x"]))
    sampled = {
        name: [values[index] for index in positions]
        for name, values in fixture.raw.items()
    }
    x, w, z, y = (_standardize(sampled[name]) for name in ("x", "w", "z", "y"))
    xw, xw_sd = _standardized_product(x, w)
    xz, xz_sd = _standardized_product(x, z)
    wz, wz_sd = _standardized_product(w, z)
    xwz, xwz_sd = _standardized_product(x, w, z)
    design = (x, w, z, xw, xz, wz, xwz)
    coefficients = _fit(design, y)
    gammas = (
        coefficients[3] / xw_sd,
        coefficients[4] / xz_sd,
        coefficients[5] / wz_sd,
    )
    delta = coefficients[6] / xwz_sd
    first_probes = _probe_values(sampled["w"], w, fixture.first_probe_kind)
    second_probes = _probe_values(sampled["z"], z, fixture.second_probe_kind)
    conditional = tuple(gammas[0] + delta * standardized_z for _, standardized_z in second_probes)
    slopes = tuple(
        coefficients[0]
        + gammas[0] * standardized_w
        + gammas[1] * standardized_z
        + delta * standardized_w * standardized_z
        for _, standardized_w in first_probes
        for _, standardized_z in second_probes
    )
    return ThreeWayFit(
        coefficients=tuple(coefficients),
        pairwise_gammas=gammas,
        delta=delta,
        first_probes=first_probes,
        second_probes=second_probes,
        conditional_interactions=conditional,
        simple_slopes=slopes,
        design=tuple(tuple(column) for column in design),
        outcome=tuple(y),
    )


def _fixtures() -> tuple[ThreeWayFixture, ThreeWayFixture]:
    index = [position - 29.5 for position in range(60)]
    x = [math.sin(0.173 * row) + 0.011 * row + 0.0008 * row * row for row in index]
    w = [math.cos(0.217 * row) - 0.006 * row + 0.19 * math.sin(0.071 * row) for row in index]
    z_continuous = [math.sin(0.307 * row) + 0.23 * math.cos(0.127 * row) + 0.004 * row for row in index]
    z_binary = [float((position * 7 + position // 5) % 2) for position in range(60)]
    noise = [0.045 * math.sin(0.811 * row) + 0.027 * math.cos(0.439 * row) for row in index]

    def outcome(z: Sequence[float]) -> tuple[float, ...]:
        xs, ws, zs = map(_standardize, (x, w, z))
        return tuple(
            0.31 * a + 0.17 * b - 0.13 * c
            + 0.24 * a * b - 0.18 * a * c + 0.12 * b * c
            + 0.21 * a * b * c + error
            for a, b, c, error in zip(xs, ws, zs, noise, strict=True)
        )

    return (
        ThreeWayFixture(
            "continuous_continuous",
            {"x": tuple(x), "w": tuple(w), "z": tuple(z_continuous), "y": outcome(z_continuous)},
            "continuous_standardized",
            "continuous_standardized",
        ),
        ThreeWayFixture(
            "continuous_binary",
            {"x": tuple(x), "w": tuple(w), "z": tuple(z_binary), "y": outcome(z_binary)},
            "continuous_standardized",
            "binary_zero_one",
        ),
    )


def _target_values(fit: ThreeWayFit) -> tuple[float, ...]:
    return (fit.delta, *fit.conditional_interactions, *fit.simple_slopes)


def _three_way_bootstrap(fixture: ThreeWayFixture, workers: int) -> dict[str, object]:
    by_index: dict[int, tuple[float, ...]] = {}
    failures: dict[int, str] = {}
    operation = THREE_WAY_BOOTSTRAP["operation_version"]
    for worker in range(workers):
        for replicate in range(worker, RESAMPLES, workers):
            positions = _positions(operation, len(fixture.raw["x"]), replicate)
            try:
                by_index[replicate] = _target_values(_fit_three_way(fixture, positions))
            except ValueError as error:
                failures[replicate] = str(error)
    usable = sorted(by_index)
    target_count = len(next(iter(by_index.values()))) if by_index else 0
    alpha = (1.0 - CONFIDENCE_LEVEL) / 2.0
    intervals = tuple(
        (
            _type7([by_index[index][target] for index in usable], alpha),
            _type7([by_index[index][target] for index in usable], 1.0 - alpha),
        )
        for target in range(target_count)
    )
    digest_payload = {
        "usable": usable,
        "failures": failures,
        "values": [[index, list(by_index[index])] for index in usable],
    }
    digest = hashlib.sha256(
        json.dumps(digest_payload, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
    ).hexdigest()
    return {
        "workers": workers,
        "usable_indices": usable,
        "failed_replicates": failures,
        "target_intervals": intervals,
        "replay_sha256": digest,
    }


def _mediation_fixture() -> Mapping[str, tuple[float, ...]]:
    index = [position - 34.5 for position in range(70)]
    x = [math.sin(0.149 * row) + 0.012 * row + 0.0005 * row * row for row in index]
    xs = _standardize(x)
    disturbance_m = _standardize([math.cos(0.337 * row) + 0.09 * math.sin(0.71 * row) for row in index])
    m = [0.63 * a + 0.31 * error for a, error in zip(xs, disturbance_m, strict=True)]
    ms = _standardize(m)
    disturbance_y = _standardize([math.sin(0.419 * row) - 0.11 * math.cos(0.83 * row) for row in index])
    y = [0.22 * a + 0.57 * b + 0.27 * error for a, b, error in zip(xs, ms, disturbance_y, strict=True)]
    return {"x": tuple(x), "m": tuple(m), "y": tuple(y)}


def _fit_mediation(raw: Mapping[str, Sequence[float]], positions: Sequence[int] | None = None) -> tuple[float, float, float, float]:
    if positions is None:
        positions = range(len(raw["x"]))
    x, m, y = (
        _standardize([raw[name][index] for index in positions])
        for name in ("x", "m", "y")
    )
    a = _fit([x], m)[0]
    direct, b = _fit([x, m], y)
    indirect = a * b
    return a, b, indirect, direct + indirect


def _mediation_bootstrap(raw: Mapping[str, Sequence[float]], workers: int) -> dict[str, object]:
    operation = SINGLE_MEDIATION["operation_version"]
    by_index: dict[int, float] = {}
    failures: dict[int, str] = {}
    for worker in range(workers):
        for replicate in range(worker, RESAMPLES, workers):
            try:
                positions = _positions(operation, len(raw["x"]), replicate)
                by_index[replicate] = _fit_mediation(raw, positions)[2]
            except ValueError as error:
                failures[replicate] = str(error)
    usable = sorted(by_index)
    alpha = (1.0 - CONFIDENCE_LEVEL) / 2.0
    values = [by_index[index] for index in usable]
    payload = {"usable": usable, "failures": failures, "values": [[index, by_index[index]] for index in usable]}
    return {
        "workers": workers,
        "usable_indices": usable,
        "failed_replicates": failures,
        "interval": (_type7(values, alpha), _type7(values, 1.0 - alpha)),
        "replay_sha256": hashlib.sha256(
            json.dumps(payload, sort_keys=True, separators=(",", ":"), allow_nan=False).encode("utf-8")
        ).hexdigest(),
    }


def _numeric_vector(value: Any) -> list[float] | None:
    if not isinstance(value, (list, tuple)):
        return None
    result: list[float] = []
    for item in value:
        if isinstance(item, bool) or not isinstance(item, (int, float)):
            return None
        number = float(item)
        if not math.isfinite(number):
            return None
        result.append(number)
    return result


def _vector_comparison(actual: Any, expected: Sequence[float], tolerance: float = PRODUCT_TOLERANCE) -> dict[str, object]:
    observed = _numeric_vector(actual)
    if observed is None:
        return {"passed": False, "reason": "value is not a finite numeric array"}
    if len(observed) != len(expected):
        return {
            "passed": False,
            "reason": "array length differs",
            "expected_count": len(expected),
            "observed_count": len(observed),
        }
    error = _max_error(observed, expected)
    return {
        "passed": error <= tolerance,
        "count": len(expected),
        "maximum_absolute_error": error,
        "tolerance": tolerance,
    }


def _scalar_comparison(actual: Any, expected: float, tolerance: float = PRODUCT_TOLERANCE) -> dict[str, object]:
    if isinstance(actual, bool) or not isinstance(actual, (int, float)) or not math.isfinite(float(actual)):
        return {"passed": False, "reason": "value is not finite"}
    error = abs(float(actual) - expected)
    return {
        "passed": error <= tolerance,
        "maximum_absolute_error": error,
        "tolerance": tolerance,
    }


def _probe_pairs(value: Any) -> list[tuple[float, float]] | None:
    if not isinstance(value, list):
        return None
    pairs: list[tuple[float, float]] = []
    for row in value:
        if isinstance(row, Mapping):
            raw = row.get("reported_value")
            standardized = row.get("standardized_value")
        elif isinstance(row, (list, tuple)) and len(row) == 2:
            raw, standardized = row
        else:
            return None
        if (
            isinstance(raw, bool)
            or isinstance(standardized, bool)
            or not isinstance(raw, (int, float))
            or not isinstance(standardized, (int, float))
            or not math.isfinite(float(raw))
            or not math.isfinite(float(standardized))
        ):
            return None
        pairs.append((float(raw), float(standardized)))
    return pairs


def _probe_comparison(actual: Any, expected: Sequence[tuple[float, float]]) -> dict[str, object]:
    observed = _probe_pairs(actual)
    if observed is None:
        return {"passed": False, "reason": "probe inventory is malformed"}
    return _vector_comparison(
        [value for pair in observed for value in pair],
        [value for pair in expected for value in pair],
    )


def _failed_indices(value: Any) -> list[int] | None:
    if not isinstance(value, list):
        return None
    indices: list[int] = []
    for row in value:
        index = row.get("replicate_index") if isinstance(row, Mapping) else row
        if isinstance(index, bool) or not isinstance(index, int) or index < 0:
            return None
        indices.append(index)
    return indices


def _replicate_positions(value: Any, resamples: int, case_count: int) -> dict[int, list[int]] | None:
    if isinstance(value, Mapping):
        rows: Any = [
            {"replicate_index": key, "sample_indices": positions}
            for key, positions in value.items()
        ]
    else:
        rows = value
    if not isinstance(rows, list):
        return None
    result: dict[int, list[int]] = {}
    for row in rows:
        if not isinstance(row, Mapping):
            return None
        raw_index = row.get("replicate_index")
        try:
            index = int(raw_index)
        except (TypeError, ValueError):
            return None
        positions = row.get("sample_indices")
        if (
            isinstance(raw_index, bool)
            or index < 0
            or index >= resamples
            or index in result
            or not isinstance(positions, list)
            or len(positions) != case_count
            or any(
                isinstance(position, bool)
                or not isinstance(position, int)
                or position < 0
                or position >= case_count
                for position in positions
            )
        ):
            return None
        result[index] = list(positions)
    return result


def _target_rows(section: Mapping[str, Any]) -> list[Mapping[str, Any]] | None:
    value = section.get("targets", section.get("target_intervals"))
    if isinstance(value, Mapping):
        result: list[Mapping[str, Any]] = []
        for target_id, row in value.items():
            if not isinstance(row, Mapping):
                return None
            result.append({"target_id": str(target_id), **row})
        return result
    if not isinstance(value, list) or any(not isinstance(row, Mapping) for row in value):
        return None
    return list(value)


def _three_way_value_for_target(fit: ThreeWayFit, target_id: str) -> float:
    if target_id.startswith("three_way_delta:"):
        return fit.delta
    conditional = re.search(r":z(\d+)$", target_id)
    if target_id.startswith("three_way_conditional_xw:") and conditional:
        index = int(conditional.group(1))
        return fit.conditional_interactions[index]
    simple = re.search(r":w(\d+):z(\d+)$", target_id)
    if target_id.startswith("three_way_simple_x:") and simple:
        first = int(simple.group(1))
        second = int(simple.group(2))
        return fit.simple_slopes[first * len(fit.second_probes) + second]
    raise ValueError(f"unknown three-way target identity: {target_id}")


def _sample_se(values: Sequence[float]) -> float:
    center = _mean(values)
    return math.sqrt(math.fsum((value - center) ** 2 for value in values) / (len(values) - 1))


def _compare_three_way_bootstrap_product(
    fixture: ThreeWayFixture,
    section: Mapping[str, Any],
) -> dict[str, object]:
    checks: dict[str, bool] = {}
    details: dict[str, object] = {}
    checks["method identity"] = (
        section.get("method_version") == THREE_WAY_BOOTSTRAP["method_version"]
        and section.get("operation_version") == THREE_WAY_BOOTSTRAP["operation_version"]
        and section.get("stream_version") == PRODUCTION_STREAM
    )
    resamples = section.get("resamples")
    seed = section.get("seed")
    checks["resample count"] = resamples == RESAMPLES
    checks["seed"] = str(seed) == str(SEED)
    positions = _replicate_positions(
        section.get("replicate_positions"),
        RESAMPLES,
        len(fixture.raw["x"]),
    )
    checks["complete indexed sampling witness"] = positions is not None and set(positions) == set(range(RESAMPLES))
    failed = _failed_indices(section.get("failed_replicates"))
    usable = section.get("usable_indices")
    usable_valid = (
        isinstance(usable, list)
        and all(isinstance(index, int) and not isinstance(index, bool) for index in usable)
        and usable == sorted(set(usable))
    )
    checks["canonical usable ledger"] = usable_valid
    expected_usable_sha256 = hashlib.sha256(
        json.dumps(usable, separators=(",", ":")).encode("utf-8")
    ).hexdigest() if usable_valid else None
    checks["usable ledger digest"] = (
        expected_usable_sha256 is not None
        and section.get("usable_indices_sha256") == expected_usable_sha256
    )
    checks["minimum usable ledger"] = bool(
        usable_valid and len(usable) >= math.ceil(0.9 * RESAMPLES)
    )
    checks["canonical failure ledger"] = failed is not None and failed == sorted(set(failed))
    checks["complete shared ledger"] = bool(
        usable_valid
        and failed is not None
        and sorted([*usable, *failed]) == list(range(RESAMPLES))
    )
    target_rows = _target_rows(section)
    expected_target_count = 1 + len(_fit_three_way(fixture).second_probes) + len(_fit_three_way(fixture).simple_slopes)
    checks["target inventory"] = (
        target_rows is not None
        and len(target_rows) == expected_target_count
        and len({str(row.get("target_id", "")) for row in target_rows}) == expected_target_count
    )
    if (
        not checks["complete indexed sampling witness"]
        or not usable_valid
        or failed is None
        or target_rows is None
    ):
        return {"passed": False, "checks": checks, "details": details}

    independent: dict[int, ThreeWayFit] = {}
    independent_failures: list[int] = []
    for index in range(RESAMPLES):
        try:
            independent[index] = _fit_three_way(fixture, positions[index])
        except ValueError:
            independent_failures.append(index)
    checks["production and oracle failure ledgers match"] = independent_failures == failed
    checks["production usable rows all refit independently"] = all(index in independent for index in usable)

    point = _fit_three_way(fixture)
    target_details: dict[str, object] = {}
    all_target_checks = True
    if (
        checks["target inventory"]
        and checks["minimum usable ledger"]
        and checks["production usable rows all refit independently"]
    ):
        alpha = (1.0 - CONFIDENCE_LEVEL) / 2.0
        for row in target_rows:
            target_id = str(row.get("target_id", ""))
            try:
                original = _three_way_value_for_target(point, target_id)
                values = [_three_way_value_for_target(independent[index], target_id) for index in usable]
                expected = {
                    "original": original,
                    "bootstrap_mean": _mean(values),
                    "standard_error": _sample_se(values),
                    "lower": _type7(values, alpha),
                    "upper": _type7(values, 1.0 - alpha),
                    "usable_replicates": len(values),
                }
                comparisons = {
                    field: (
                        {"passed": row.get(field) == value}
                        if field == "usable_replicates"
                        else _scalar_comparison(row.get(field), float(value))
                    )
                    for field, value in expected.items()
                    if field in row or field in {"original", "lower", "upper", "usable_replicates"}
                }
                required = {
                    "original",
                    "bootstrap_mean",
                    "standard_error",
                    "lower",
                    "upper",
                    "usable_replicates",
                }
                complete = required.issubset(comparisons)
                target_passed = complete and all(item["passed"] for item in comparisons.values())
                all_target_checks = all_target_checks and target_passed
                target_details[target_id] = {
                    "passed": target_passed,
                    "required_fields_present": complete,
                    "comparisons": comparisons,
                }
            except (IndexError, ValueError) as error:
                all_target_checks = False
                target_details[target_id] = {"passed": False, "reason": str(error)}
    else:
        all_target_checks = False
    checks["exact production bootstrap targets"] = all_target_checks
    details["targets"] = target_details
    details["oracle_failed_indices"] = independent_failures
    details["production_failed_indices"] = failed
    return {"passed": all(checks.values()), "checks": checks, "details": details}


def _compare_mediation_product(
    raw: Mapping[str, Sequence[float]],
    section: Mapping[str, Any],
) -> dict[str, object]:
    checks: dict[str, bool] = {}
    details: dict[str, object] = {}
    checks["method identity"] = (
        section.get("method_version") == SINGLE_MEDIATION["method_version"]
        and section.get("operation_version") == SINGLE_MEDIATION["operation_version"]
        and section.get("stream_version") == PRODUCTION_STREAM
    )
    checks["resample count"] = section.get("resamples") == RESAMPLES
    checks["seed"] = str(section.get("seed")) == str(SEED)
    point = section.get("point")
    expected_point = _fit_mediation(raw)
    point_fields = ("path_a", "path_b", "specific_indirect", "total_effect")
    point_checks = {
        name: _scalar_comparison(point.get(name) if isinstance(point, Mapping) else None, expected)
        for name, expected in zip(point_fields, expected_point, strict=True)
    }
    checks["exact production point effects"] = all(row["passed"] for row in point_checks.values())
    details["point"] = point_checks

    positions = _replicate_positions(section.get("replicate_positions"), RESAMPLES, len(raw["x"]))
    checks["complete indexed sampling witness"] = positions is not None and set(positions) == set(range(RESAMPLES))
    failed = _failed_indices(section.get("failed_replicates"))
    usable = section.get("usable_indices")
    usable_valid = (
        isinstance(usable, list)
        and all(isinstance(index, int) and not isinstance(index, bool) for index in usable)
        and usable == sorted(set(usable))
    )
    checks["canonical usable ledger"] = usable_valid
    expected_usable_sha256 = hashlib.sha256(
        json.dumps(usable, separators=(",", ":")).encode("utf-8")
    ).hexdigest() if usable_valid else None
    checks["usable ledger digest"] = (
        expected_usable_sha256 is not None
        and section.get("usable_indices_sha256") == expected_usable_sha256
    )
    checks["minimum usable ledger"] = bool(
        usable_valid and len(usable) >= math.ceil(0.9 * RESAMPLES)
    )
    checks["canonical failure ledger"] = failed is not None and failed == sorted(set(failed))
    checks["complete shared ledger"] = bool(
        usable_valid and failed is not None and sorted([*usable, *failed]) == list(range(RESAMPLES))
    )
    if not checks["complete indexed sampling witness"] or not usable_valid or failed is None:
        return {"passed": False, "checks": checks, "details": details}

    independent: dict[int, float] = {}
    independent_failures: list[int] = []
    for index in range(RESAMPLES):
        try:
            independent[index] = _fit_mediation(raw, positions[index])[2]
        except ValueError:
            independent_failures.append(index)
    checks["production and oracle failure ledgers match"] = independent_failures == failed
    checks["production usable rows all refit independently"] = all(index in independent for index in usable)

    target: Any = section.get("target", section.get("inference", section.get("interval")))
    if target is None:
        rows = _target_rows(section)
        target = rows[0] if rows and len(rows) == 1 else None
    if (
        isinstance(target, Mapping)
        and checks["minimum usable ledger"]
        and checks["production usable rows all refit independently"]
    ):
        values = [independent[index] for index in usable]
        alpha = (1.0 - CONFIDENCE_LEVEL) / 2.0
        expected = {
            "original": expected_point[2],
            "bootstrap_mean": _mean(values),
            "standard_error": _sample_se(values),
            "lower": _type7(values, alpha),
            "upper": _type7(values, 1.0 - alpha),
            "usable_replicates": len(values),
        }
        comparisons = {
            field: (
                {"passed": target.get(field) == value}
                if field == "usable_replicates"
                else _scalar_comparison(target.get(field), float(value))
            )
            for field, value in expected.items()
            if field in target or field in {"original", "lower", "upper", "usable_replicates"}
        }
        required = {
            "original",
            "bootstrap_mean",
            "standard_error",
            "lower",
            "upper",
            "usable_replicates",
        }
        checks["exact production bootstrap target"] = required.issubset(comparisons) and all(
            row["passed"] for row in comparisons.values()
        )
        details["target"] = comparisons
    else:
        checks["exact production bootstrap target"] = False
    details["oracle_failed_indices"] = independent_failures
    details["production_failed_indices"] = failed
    return {"passed": all(checks.values()), "checks": checks, "details": details}


def _identity_comparison(product: Mapping[str, Any]) -> dict[str, bool]:
    identities = product.get("identities")
    if not isinstance(identities, Mapping):
        return {name: False for name in ("single_mediation_bootstrap", "three_way_point", "three_way_bootstrap")}
    expected = {
        "single_mediation_bootstrap": SINGLE_MEDIATION,
        "three_way_point": THREE_WAY_POINT,
        "three_way_bootstrap": THREE_WAY_BOOTSTRAP,
    }
    return {
        name: isinstance(identities.get(name), Mapping)
        and all(identities[name].get(key) == value for key, value in contract.items())
        for name, contract in expected.items()
    }


def _compare_product(
    product: Mapping[str, Any] | None,
    fixtures: Sequence[ThreeWayFixture],
    mediation: Mapping[str, Sequence[float]],
    required: bool,
    source: str | None,
    source_sha256: str | None,
    load_error: str | None = None,
) -> dict[str, object]:
    if product is None:
        return {
            "status": "failed" if required or load_error else "skipped",
            "passed": not required and load_error is None,
            "required": required,
            "source": source,
            "source_sha256": source_sha256,
            "reason": load_error or "no production JSON supplied",
        }
    identity_checks = _identity_comparison(product)
    point_root = product.get("three_way_point")
    bootstrap_root = product.get("three_way_bootstrap")
    point_reports: dict[str, object] = {}
    bootstrap_reports: dict[str, object] = {}
    point_passed = isinstance(point_root, Mapping)
    bootstrap_passed = isinstance(bootstrap_root, Mapping)
    for fixture in fixtures:
        expected = _fit_three_way(fixture)
        row = point_root.get(fixture.scenario_id) if isinstance(point_root, Mapping) else None
        if not isinstance(row, Mapping):
            point_reports[fixture.scenario_id] = {"passed": False, "reason": "scenario is missing"}
            point_passed = False
        else:
            delta_value = row.get("delta", row.get("three_way_delta"))
            comparisons = {
                "method_version": {
                    "passed": row.get("method_version") == THREE_WAY_POINT["method_version"]
                },
                "coefficients": _vector_comparison(row.get("coefficients"), expected.coefficients),
                "pairwise_gammas": _vector_comparison(row.get("pairwise_gammas"), expected.pairwise_gammas),
                "delta": _scalar_comparison(delta_value, expected.delta),
                "first_moderator_probes": _probe_comparison(row.get("first_moderator_probes"), expected.first_probes),
                "second_moderator_probes": _probe_comparison(row.get("second_moderator_probes"), expected.second_probes),
                "conditional_interactions": _vector_comparison(row.get("conditional_interactions"), expected.conditional_interactions),
                "simple_slopes": _vector_comparison(row.get("simple_slopes"), expected.simple_slopes),
            }
            row_passed = all(comparison["passed"] for comparison in comparisons.values())
            point_reports[fixture.scenario_id] = {"passed": row_passed, "comparisons": comparisons}
            point_passed = point_passed and row_passed

        bootstrap_row = bootstrap_root.get(fixture.scenario_id) if isinstance(bootstrap_root, Mapping) else None
        if not isinstance(bootstrap_row, Mapping):
            bootstrap_reports[fixture.scenario_id] = {"passed": False, "reason": "scenario is missing"}
            bootstrap_passed = False
        else:
            report = _compare_three_way_bootstrap_product(fixture, bootstrap_row)
            bootstrap_reports[fixture.scenario_id] = report
            bootstrap_passed = bootstrap_passed and bool(report["passed"])

    mediation_section = product.get("single_mediation_bootstrap")
    mediation_report = (
        _compare_mediation_product(mediation, mediation_section)
        if isinstance(mediation_section, Mapping)
        else {"passed": False, "reason": "single_mediation_bootstrap is missing"}
    )
    checks = {
        "product report passed": product.get("passed") is True,
        "schema version": product.get("schema_version") == 1,
        "exact identities": all(identity_checks.values()),
        "exact three-way point and probes": point_passed,
        "exact three-way indexed bootstrap": bootstrap_passed,
        "exact single-mediation point and indexed bootstrap": bool(mediation_report["passed"]),
    }
    return {
        "status": "passed" if all(checks.values()) else "failed",
        "passed": all(checks.values()),
        "required": required,
        "source": source,
        "source_sha256": source_sha256,
        "suite_id": product.get("suite_id"),
        "checks": checks,
        "identity_checks": identity_checks,
        "three_way_point": point_reports,
        "three_way_bootstrap": bootstrap_reports,
        "single_mediation_bootstrap": mediation_report,
    }


def _r_point_cross_check(fixtures: Sequence[ThreeWayFixture], rscript: str | None, required: bool) -> dict[str, object]:
    if not rscript:
        return {
            "status": "failed" if required else "skipped",
            "passed": not required,
            "required": required,
            "reason": "Rscript was not found",
        }
    try:
        with tempfile.TemporaryDirectory(prefix="quickpls-v253-reference-") as folder:
            root = Path(folder)
            data_path = root / "point.csv"
            script_path = root / "reference.R"
            with data_path.open("w", encoding="utf-8", newline="") as handle:
                writer = csv.writer(handle)
                writer.writerow(["scenario", "x", "w", "z", "xw", "xz", "wz", "xwz", "y"])
                for fixture in fixtures:
                    fit = _fit_three_way(fixture)
                    for row in range(len(fit.outcome)):
                        writer.writerow([
                            fixture.scenario_id,
                            *(fit.design[column][row] for column in range(7)),
                            fit.outcome[row],
                        ])
            script_path.write_text(
                """args <- commandArgs(trailingOnly=TRUE)
d <- read.csv(args[1], check.names=FALSE)
for (scenario in unique(d$scenario)) {
  s <- d[d$scenario == scenario, ]
  x <- as.matrix(s[, c('x','w','z','xw','xz','wz','xwz')])
  fit <- qr.solve(crossprod(x), crossprod(x, s$y), tol=1e-12)
  cat(scenario, paste(sprintf('%.17g', fit), collapse=','), sep='|')
  cat('\\n')
}
""",
                encoding="utf-8",
            )
            completed = subprocess.run(
                [rscript, "--vanilla", str(script_path), str(data_path)],
                check=False,
                capture_output=True,
                text=True,
                timeout=60,
            )
        if completed.returncode != 0:
            raise RuntimeError(completed.stderr.strip() or f"Rscript exited {completed.returncode}")
        observed: dict[str, list[float]] = {}
        for line in completed.stdout.splitlines():
            if not line.strip():
                continue
            scenario, encoded = line.split("|", 1)
            observed[scenario] = [float(value) for value in encoded.split(",")]
        errors = {
            fixture.scenario_id: _max_error(
                _fit_three_way(fixture).coefficients,
                observed.get(fixture.scenario_id, []),
            )
            for fixture in fixtures
        }
        passed = set(errors) == set(observed) and all(value <= TOLERANCE for value in errors.values())
        return {
            "status": "passed" if passed else "failed",
            "passed": passed,
            "required": required,
            "rscript": rscript,
            "maximum_absolute_coefficient_errors": errors,
        }
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as error:
        return {
            "status": "failed",
            "passed": False,
            "required": required,
            "rscript": rscript,
            "reason": str(error),
        }


def run_reference(
    rscript: str | None,
    require_r: bool,
    product: Mapping[str, Any] | None = None,
    require_product: bool = False,
    product_source: str | None = None,
    product_sha256: str | None = None,
    product_load_error: str | None = None,
) -> dict[str, object]:
    fixtures = _fixtures()
    point_rows: list[dict[str, object]] = []
    point_passed = True
    for fixture in fixtures:
        fit = _fit_three_way(fixture)
        reversed_fit = _fit_three_way(fixture, list(reversed(range(len(fixture.raw["x"])))))
        normal_errors = []
        for column, coefficient in enumerate(fit.coefficients):
            lhs = math.fsum(
                math.fsum(a * b for a, b in zip(fit.design[column], fit.design[other], strict=True))
                * fit.coefficients[other]
                for other in range(len(fit.design))
            )
            rhs = math.fsum(a * b for a, b in zip(fit.design[column], fit.outcome, strict=True))
            normal_errors.append(abs(lhs - rhs))
        checks = {
            "joint_normal_equations": max(normal_errors) <= TOLERANCE,
            "row_reorder_invariant": _max_error(fit.coefficients, reversed_fit.coefficients) <= TOLERANCE,
            "conditional_grid_complete": len(fit.conditional_interactions) == len(fit.second_probes),
            "simple_slope_grid_complete": len(fit.simple_slopes) == len(fit.first_probes) * len(fit.second_probes),
            "reported_probe_grid": tuple(value for value, _ in fit.first_probes)
            == (-1.0, 0.0, 1.0)
            and tuple(value for value, _ in fit.second_probes)
            == ((0.0, 1.0) if fixture.second_probe_kind == "binary_zero_one" else (-1.0, 0.0, 1.0)),
        }
        point_passed = point_passed and all(checks.values())
        point_rows.append({
            "scenario_id": fixture.scenario_id,
            "probe_kinds": [fixture.first_probe_kind, fixture.second_probe_kind],
            "checks": checks,
            "coefficients": fit.coefficients,
            "pairwise_gammas": fit.pairwise_gammas,
            "three_way_delta": fit.delta,
            "first_moderator_probes": fit.first_probes,
            "second_moderator_probes": fit.second_probes,
            "conditional_interactions": fit.conditional_interactions,
            "simple_slopes": fit.simple_slopes,
            "maximum_normal_equation_error": max(normal_errors),
            "row_reorder_maximum_absolute_error": _max_error(fit.coefficients, reversed_fit.coefficients),
        })

    continuous = fixtures[0]
    bootstrap_one = _three_way_bootstrap(continuous, 1)
    bootstrap_four = _three_way_bootstrap(continuous, 4)
    bootstrap_checks = {
        "worker_replay_identical": bootstrap_one["replay_sha256"] == bootstrap_four["replay_sha256"],
        "shared_target_ledger": len(bootstrap_one["target_intervals"]) == 13,
        "minimum_usable_fraction": len(bootstrap_one["usable_indices"]) >= math.ceil(0.9 * RESAMPLES),
        "type7_intervals_finite_ordered": all(
            math.isfinite(lower) and math.isfinite(upper) and lower <= upper
            for lower, upper in bootstrap_one["target_intervals"]
        ),
    }

    mediation = _mediation_fixture()
    mediation_point = _fit_mediation(mediation)
    mediation_one = _mediation_bootstrap(mediation, 1)
    mediation_four = _mediation_bootstrap(mediation, 4)
    old_multiple_cell = "qpls3.pls.general_sem_multiple_mediation_bootstrap"
    mediation_checks = {
        "point_finite": all(math.isfinite(value) for value in mediation_point),
        "worker_replay_identical": mediation_one["replay_sha256"] == mediation_four["replay_sha256"],
        "minimum_usable_fraction": len(mediation_one["usable_indices"]) >= math.ceil(0.9 * RESAMPLES),
        "interval_finite_ordered": all(math.isfinite(value) for value in mediation_one["interval"])
        and mediation_one["interval"][0] <= mediation_one["interval"][1],
        "single_identity_distinct_from_multiple": SINGLE_MEDIATION["cell_id"] != old_multiple_cell
        and SINGLE_MEDIATION["method_version"] != "general_sem_pls_full_model_case_bootstrap_v1",
    }

    r_report = _r_point_cross_check(fixtures, rscript, require_r)
    product_report = _compare_product(
        product,
        fixtures,
        mediation,
        require_product,
        product_source,
        product_sha256,
        product_load_error,
    )
    checks = {
        "three_way_point_matrix": point_passed,
        "three_way_bootstrap_replay": all(bootstrap_checks.values()),
        "single_mediation_bootstrap_replay": all(mediation_checks.values()),
        "r_cross_check": bool(r_report["passed"]),
        "production_product_comparison": bool(product_report["passed"]),
    }
    return {
        "schema_version": 1,
        "suite_id": "quickpls_v253_general_sem_compact_reference_v1",
        "passed": all(checks.values()),
        "qualification_scope": "compact_independent_observed_score_reference",
        "checks": checks,
        "identities": {
            "single_mediation_bootstrap": {**SINGLE_MEDIATION, "identity": _cell_identity(SINGLE_MEDIATION)},
            "three_way_point": {**THREE_WAY_POINT, "identity": _cell_identity(THREE_WAY_POINT)},
            "three_way_bootstrap": {**THREE_WAY_BOOTSTRAP, "identity": _cell_identity(THREE_WAY_BOOTSTRAP)},
        },
        "three_way_point": point_rows,
        "three_way_bootstrap": {
            "resamples": RESAMPLES,
            "seed": SEED,
            "confidence_level": CONFIDENCE_LEVEL,
            "stream_version": REFERENCE_STREAM,
            "checks": bootstrap_checks,
            "one_worker": bootstrap_one,
            "four_workers": bootstrap_four,
        },
        "single_mediation_bootstrap": {
            "point": {"path_a": mediation_point[0], "path_b": mediation_point[1], "specific_indirect": mediation_point[2], "total_effect": mediation_point[3]},
            "resamples": RESAMPLES,
            "seed": SEED,
            "confidence_level": CONFIDENCE_LEVEL,
            "stream_version": REFERENCE_STREAM,
            "checks": mediation_checks,
            "one_worker": mediation_one,
            "four_workers": mediation_four,
        },
        "r_cross_check": r_report,
        "product_comparison": product_report,
        "limitations": [
            "This program never imports or invokes QuickPLS production code.",
            "The fixtures start from observed construct-score proxies and do not qualify indicator-level PLS score recovery.",
            "The explicit SHA-256 resample stream verifies indexed full-refit replay and worker invariance; it is not the Rust ChaCha stream implementation.",
            "When --product-json is supplied, exact production intervals are independently recomputed from the product's bounded indexed-position witness rather than comparing unlike random streams.",
            "Previously qualified two-way moderation and multiple-mediation matrices are intentionally not repeated.",
        ],
    }


def _arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, help="Optional JSON report path")
    parser.add_argument("--product-json", type=Path, help="Bounded qpls-runner product JSON to compare exactly")
    parser.add_argument("--require-product", action="store_true", help="Fail unless the exact production comparison passes")
    parser.add_argument("--rscript", help="Explicit Rscript executable")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--require-r", action="store_true", help="Fail unless the independent base-R point cross-check passes")
    mode.add_argument("--skip-r", action="store_true", help="Do not attempt the optional base-R cross-check")
    return parser.parse_args()


def main() -> int:
    args = _arguments()
    rscript = None if args.skip_r else (args.rscript or shutil.which("Rscript"))
    product: Mapping[str, Any] | None = None
    product_error: str | None = None
    product_sha256: str | None = None
    product_source = str(args.product_json.resolve()) if args.product_json else None
    if args.product_json:
        try:
            encoded_product = args.product_json.read_bytes()
            product_sha256 = hashlib.sha256(encoded_product).hexdigest()
            decoded = json.loads(encoded_product)
            if not isinstance(decoded, Mapping):
                raise ValueError("production JSON must contain an object")
            product = decoded
        except (OSError, UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
            product_error = str(error)
    report = run_reference(
        rscript,
        args.require_r,
        product,
        args.require_product,
        product_source,
        product_sha256,
        product_error,
    )
    encoded = json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(encoded, encoding="utf-8")
    print(encoded, end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
