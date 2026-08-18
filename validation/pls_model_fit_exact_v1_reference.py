#!/usr/bin/env python3
"""Transparent validation-only oracle for the frozen exact-fit v1 primitives.

This module does not import QuickPLS code.  It independently implements the
symmetric adapted Bollen--Stine null transformation, Type-7 upper quantiles,
fixed-ledger usable accounting, and exact-fit decisions.  Full estimator-refit
and simulation qualification remain separate required evidence.
"""

from __future__ import annotations

import argparse
import json
import math
from dataclasses import asdict, dataclass
from pathlib import Path

import numpy as np


MINIMUM_USABLE_FRACTION = 0.90


@dataclass(frozen=True)
class ExactFitSummary:
    status: str
    original: float
    requested_replicates: int
    minimum_usable_replicates: int
    usable_replicates: int
    failed_replicates: int
    replicate_min: float | None
    replicate_max: float | None
    upper_95: float | None
    upper_99: float | None
    not_rejected_95: bool | None
    not_rejected_99: bool | None
    exceed_or_equal_count: int
    empirical_upper_tail_probability: float | None
    unavailable_reason_code: str | None


def _symmetric_power(matrix: np.ndarray, exponent: float) -> np.ndarray:
    matrix = np.asarray(matrix, dtype=float)
    if (
        matrix.ndim != 2
        or matrix.shape[0] == 0
        or matrix.shape[0] != matrix.shape[1]
        or not np.isfinite(matrix).all()
        or not np.allclose(matrix, matrix.T, rtol=0, atol=1e-12)
    ):
        raise ValueError("matrix must be finite, symmetric, and square")
    eigenvalues, eigenvectors = np.linalg.eigh(matrix)
    tolerance = max(1.0, float(np.max(np.abs(eigenvalues)))) * matrix.shape[0] * np.finfo(float).eps * 128
    if np.any(eigenvalues <= tolerance):
        raise ValueError("matrix must be numerically positive definite")
    return (eigenvectors * np.power(eigenvalues, exponent)) @ eigenvectors.T


def null_transform(observations: object, target_correlation: object) -> np.ndarray:
    values = np.asarray(observations, dtype=float)
    target = np.asarray(target_correlation, dtype=float)
    if values.ndim != 2 or values.shape[0] < 2 or values.shape[1] == 0:
        raise ValueError("observations must contain at least two rows and one column")
    if not np.isfinite(values).all():
        raise ValueError("observations must be finite")
    centered = values - np.mean(values, axis=0)
    standard_deviations = np.std(centered, axis=0, ddof=1)
    if np.any(~np.isfinite(standard_deviations)) or np.any(standard_deviations <= np.finfo(float).eps):
        raise ValueError("observations contain a constant or non-finite column")
    standardized = centered / standard_deviations
    observed = np.corrcoef(standardized, rowvar=False)
    if observed.ndim == 0:
        observed = np.array([[1.0]])
    if target.shape != observed.shape or not np.allclose(np.diag(target), 1.0, rtol=0, atol=1e-12):
        raise ValueError("target must be a same-dimension correlation matrix")
    transform = _symmetric_power(observed, -0.5) @ _symmetric_power(target, 0.5)
    transformed = standardized @ transform
    recovered = np.corrcoef(transformed, rowvar=False)
    if recovered.ndim == 0:
        recovered = np.array([[1.0]])
    if not np.allclose(recovered, target, rtol=0, atol=1e-9):
        raise ValueError("null transformation failed its target-correlation identity")
    return transformed


def type7_quantile(values: object, probability: float) -> float:
    sorted_values = np.sort(np.asarray(values, dtype=float))
    if sorted_values.ndim != 1 or sorted_values.size == 0 or not np.isfinite(sorted_values).all():
        raise ValueError("quantile values must be a nonempty finite vector")
    if not 0 <= probability <= 1:
        raise ValueError("probability must be between zero and one")
    position = (sorted_values.size - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return float(sorted_values[lower])
    return float(
        sorted_values[lower]
        + (position - lower) * (sorted_values[upper] - sorted_values[lower])
    )


def summarize_fixed_ledger(
    original: float,
    replicate_values: object,
    requested_replicates: int,
) -> ExactFitSummary:
    if not math.isfinite(original) or original < 0:
        raise ValueError("original discrepancy must be finite and nonnegative")
    values = np.asarray(replicate_values, dtype=float)
    if values.ndim != 1 or values.size != requested_replicates:
        raise ValueError("the fixed ledger must contain one cell per requested replicate")
    usable = values[np.isfinite(values)]
    if np.any(usable < 0):
        raise ValueError("usable replicate discrepancies must be nonnegative")
    minimum = max(2, math.ceil(requested_replicates * MINIMUM_USABLE_FRACTION))
    exceed_or_equal = int(np.sum(usable >= original))
    if usable.size < minimum:
        return ExactFitSummary(
            status="unavailable",
            original=original,
            requested_replicates=requested_replicates,
            minimum_usable_replicates=minimum,
            usable_replicates=int(usable.size),
            failed_replicates=requested_replicates - int(usable.size),
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
    upper_95 = type7_quantile(usable, 0.95)
    upper_99 = type7_quantile(usable, 0.99)
    return ExactFitSummary(
        status="available",
        original=original,
        requested_replicates=requested_replicates,
        minimum_usable_replicates=minimum,
        usable_replicates=int(usable.size),
        failed_replicates=requested_replicates - int(usable.size),
        replicate_min=float(np.min(usable)),
        replicate_max=float(np.max(usable)),
        upper_95=upper_95,
        upper_99=upper_99,
        not_rejected_95=original <= upper_95,
        not_rejected_99=original <= upper_99,
        exceed_or_equal_count=exceed_or_equal,
        empirical_upper_tail_probability=exceed_or_equal / int(usable.size),
        unavailable_reason_code=None,
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path)
    args = parser.parse_args()
    if args.input:
        payload = json.loads(args.input.read_text(encoding="utf-8"))
        transformed = null_transform(payload["observations"], payload["target_correlation"])
        summary = summarize_fixed_ledger(
            float(payload["original"]),
            payload["replicate_values"],
            int(payload["requested_replicates"]),
        )
    else:
        transformed = null_transform(
            [[-1.2, -0.8], [-0.7, 0.1], [-0.1, 0.6], [0.4, -0.2], [0.8, 1.3], [1.4, -1.0]],
            [[1.0, 0.35], [0.35, 1.0]],
        )
        summary = summarize_fixed_ledger(0.4, [index / 10 for index in range(10)], 10)
    print(
        json.dumps(
            {
                "transformed_correlation": np.corrcoef(transformed, rowvar=False).tolist(),
                "summary": asdict(summary),
            },
            indent=2,
            sort_keys=True,
            allow_nan=False,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
