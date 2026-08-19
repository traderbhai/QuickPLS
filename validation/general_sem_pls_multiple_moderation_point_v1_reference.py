#!/usr/bin/env python3
"""Independent contract micro-reference for the General SEM moderation point cell.

This standard-library-only reference never imports or invokes production Rust or
TypeScript. It freezes the joint-equation, product-scale, scientific-gamma, and
fixed-probe arithmetic for deterministic same-focal and different-focal cases.
It is deliberately not a PLS score oracle or qualification-scale simulation.
"""

from __future__ import annotations

import json
import math
from typing import Iterable, Sequence


TOLERANCE = 1e-10


def _mean(values: Sequence[float]) -> float:
    return math.fsum(values) / len(values)


def _sample_sd(values: Sequence[float]) -> float:
    center = _mean(values)
    return math.sqrt(math.fsum((value - center) ** 2 for value in values) / (len(values) - 1))


def _standardize(values: Sequence[float]) -> list[float]:
    center = _mean(values)
    scale = _sample_sd(values)
    if not math.isfinite(scale) or scale <= 0:
        raise ValueError("cannot standardize a constant or nonfinite column")
    return [(value - center) / scale for value in values]


def _product_column(left: Sequence[float], right: Sequence[float]) -> tuple[list[float], float, float]:
    product = [a * b for a, b in zip(left, right, strict=True)]
    center = _mean(product)
    scale = _sample_sd(product)
    if not math.isfinite(scale) or scale <= 0:
        raise ValueError("cannot standardize a constant or nonfinite product column")
    return [(value - center) / scale for value in product], center, scale


def _solve(matrix: list[list[float]], vector: list[float]) -> list[float]:
    size = len(vector)
    augmented = [row[:] + [value] for row, value in zip(matrix, vector, strict=True)]
    for column in range(size):
        pivot = max(range(column, size), key=lambda row: abs(augmented[row][column]))
        if abs(augmented[pivot][column]) <= 1e-12:
            raise ValueError("singular joint stage-two equation")
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        divisor = augmented[column][column]
        augmented[column] = [value / divisor for value in augmented[column]]
        for row in range(size):
            if row == column:
                continue
            factor = augmented[row][column]
            augmented[row] = [
                current - factor * basis
                for current, basis in zip(augmented[row], augmented[column], strict=True)
            ]
    return [augmented[row][-1] for row in range(size)]


def _fit(columns: Sequence[Sequence[float]], outcome: Sequence[float]) -> list[float]:
    width = len(columns)
    gram = [
        [math.fsum(a * b for a, b in zip(columns[i], columns[j], strict=True)) for j in range(width)]
        for i in range(width)
    ]
    rhs = [math.fsum(x * y for x, y in zip(column, outcome, strict=True)) for column in columns]
    return _solve(gram, rhs)


def _linear_combination(columns: Sequence[Sequence[float]], coefficients: Sequence[float]) -> list[float]:
    return [
        math.fsum(coefficient * column[row] for coefficient, column in zip(coefficients, columns, strict=True))
        for row in range(len(columns[0]))
    ]


def _max_error(actual: Iterable[float], expected: Iterable[float]) -> float:
    return max(abs(a - b) for a, b in zip(actual, expected, strict=True))


def run_reference() -> dict[str, object]:
    rows = list(range(-12, 13))
    x = _standardize([value + 0.031 * value * value for value in rows])
    w = _standardize([math.sin(value * 0.43) + 0.07 * value for value in rows])
    z = _standardize([0.62 * math.sin(value * 0.43) + math.cos(value * 0.29) + 0.03 * value for value in rows])
    a = _standardize([math.cos(value * 0.37) - 0.04 * value + 0.006 * value * value for value in rows])

    xw, xw_mean, xw_sd = _product_column(x, w)
    xz, xz_mean, xz_sd = _product_column(x, z)
    az, az_mean, az_sd = _product_column(a, z)

    same_focal_columns = [x, w, z, xw, xz]
    same_focal_truth = [0.31, 0.18, -0.12, 0.22, -0.17]
    same_focal_outcome = _linear_combination(same_focal_columns, same_focal_truth)
    same_focal_fit = _fit(same_focal_columns, same_focal_outcome)

    # An isolated one-interaction regression is intentionally not authoritative
    # when a correlated second interaction belongs to the same joint equation.
    isolated_first_fit = _fit([x, w, z, xw], same_focal_outcome)

    different_focal_columns = [x, w, a, z, xw, az]
    different_focal_truth = [0.27, 0.14, -0.23, 0.16, 0.19, -0.21]
    different_focal_outcome = _linear_combination(
        different_focal_columns,
        different_focal_truth,
    )
    different_focal_fit = _fit(different_focal_columns, different_focal_outcome)

    gamma_xw = same_focal_fit[3] / xw_sd
    expected_gamma_xw = same_focal_truth[3] / xw_sd
    slopes = [same_focal_fit[0] + gamma_xw * probe for probe in (-1.0, 0.0, 1.0)]
    expected_slopes = [same_focal_truth[0] + expected_gamma_xw * probe for probe in (-1.0, 0.0, 1.0)]

    reordered = list(reversed(range(len(rows))))
    reordered_fit = _fit(
        [[column[index] for index in reordered] for column in same_focal_columns],
        [same_focal_outcome[index] for index in reordered],
    )
    singular_rejected = False
    try:
        _fit([x, x], same_focal_outcome)
    except ValueError:
        singular_rejected = True
    constant_product_rejected = False
    try:
        _product_column([1.0] * len(rows), [1.0] * len(rows))
    except ValueError:
        constant_product_rejected = True

    checks = {
        "same_focal_joint_coefficients": _max_error(same_focal_fit, same_focal_truth) <= TOLERANCE,
        "different_focal_joint_coefficients": _max_error(
            different_focal_fit,
            different_focal_truth,
        ) <= TOLERANCE,
        "product_sample_standardization": all(
            abs(_mean(column)) <= TOLERANCE and abs(_sample_sd(column) - 1.0) <= TOLERANCE
            for column in (xw, xz, az)
        ),
        "scientific_gamma_rescaling": abs(gamma_xw - expected_gamma_xw) <= TOLERANCE,
        "fixed_probe_simple_slopes": _max_error(slopes, expected_slopes) <= TOLERANCE,
        "row_order_metamorphic": _max_error(reordered_fit, same_focal_fit) <= TOLERANCE,
        "joint_fit_not_isolated_fit": abs(isolated_first_fit[3] - same_focal_fit[3]) > 1e-4,
        "singular_equation_rejected": singular_rejected,
        "constant_product_rejected": constant_product_rejected,
    }
    return {
        "schema_version": 1,
        "reference_kind": "independent_contract_micro_reference",
        "feature_id": "qpls3.pls.general_sem_multiple_two_way_moderation_point",
        "method_version": "general_sem_pls_multiple_two_way_moderation_point_v1",
        "passed": all(checks.values()),
        "qualification_ready": False,
        "checks": checks,
        "metrics": {
            "same_focal_max_absolute_coefficient_error": _max_error(same_focal_fit, same_focal_truth),
            "different_focal_max_absolute_coefficient_error": _max_error(
                different_focal_fit,
                different_focal_truth,
            ),
            "row_order_max_absolute_error": _max_error(reordered_fit, same_focal_fit),
            "xw_product_mean": xw_mean,
            "xw_product_sample_sd": xw_sd,
            "xz_product_mean": xz_mean,
            "xz_product_sample_sd": xz_sd,
            "az_product_mean": az_mean,
            "az_product_sample_sd": az_sd,
        },
        "limitations": [
            "The reference never imports or invokes production QuickPLS code.",
            "Inputs are deterministic standardized score columns, not indicator-level PLS score recovery.",
            "The scenarios verify joint OLS, product scaling, scientific gamma, fixed probes, row order, constant-product rejection, and singular failure only.",
            "This is not qualification-scale simulation, interval coverage, SmartPLS numerical parity, or release evidence.",
        ],
    }


def main() -> int:
    report = run_reference()
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
