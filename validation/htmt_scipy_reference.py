#!/usr/bin/env python3
"""Independent NumPy/SciPy HTMT and bias-corrected interval reference.

This validation-only oracle shares no QuickPLS product code and does not import
the standard-library reference implementation.  Its purpose is to detect a
common-mode error in correlation, quantile, or normal-CDF calculations.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
from typing import Any

import numpy as np
import scipy
from scipy.stats import norm


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "validation" / "fixtures" / "corporate_reputation.csv"
DEFAULT_OUTPUT = ROOT / "validation" / "results" / "htmt_scipy_reference.json"
CONSTRUCTS = {
    "comp": ["COMP1", "COMP2", "COMP3"],
    "like": ["LIKE1", "LIKE2"],
    "satisfaction": ["CUSA1", "CUSA2"],
    "loyalty": ["CUSL1", "CUSL2"],
}
BOOTSTRAP_SCENARIOS = [
    {
        "id": "asymmetric_htmt_plus",
        "original": 0.75,
        "values": [0.62, 0.68, 0.70, 0.73, 0.76, 0.78, 0.81, 0.84, 0.88, 0.91],
    },
    {
        "id": "midrank_ties",
        "original": 0.50,
        "values": [0.50, 0.50, 0.50, 0.60, 0.70, 0.80],
    },
    {
        "id": "signed_original_htmt",
        "original": -0.20,
        "values": [-0.45, -0.35, -0.28, -0.22, -0.18, -0.12, -0.08, 0.02],
    },
]


def _mean(values: np.ndarray) -> float:
    value = float(np.mean(values, dtype=np.float64))
    if not np.isfinite(value):
        raise ValueError("reference mean is non-finite")
    return value


def _htmt_cell(
    correlations: np.ndarray,
    names: list[str],
    left: list[str],
    right: list[str],
    *,
    absolute: bool,
) -> dict[str, Any]:
    index = {name: position for position, name in enumerate(names)}
    cross = np.asarray(
        [correlations[index[a], index[b]] for a in left for b in right],
        dtype=np.float64,
    )
    within_left = np.asarray(
        [
            correlations[index[left[i]], index[left[j]]]
            for i in range(len(left))
            for j in range(i + 1, len(left))
        ],
        dtype=np.float64,
    )
    within_right = np.asarray(
        [
            correlations[index[right[i]], index[right[j]]]
            for i in range(len(right))
            for j in range(i + 1, len(right))
        ],
        dtype=np.float64,
    )
    if absolute:
        cross = np.abs(cross)
        within_left = np.abs(within_left)
        within_right = np.abs(within_right)
        unavailable_reason = "htmt.zero_monotrait_denominator"
    else:
        unavailable_reason = "htmt.original_nonpositive_monotrait_mean"
    left_mean = _mean(within_left)
    right_mean = _mean(within_right)
    tolerance = 64.0 * np.finfo(np.float64).eps
    if left_mean <= tolerance or right_mean <= tolerance:
        return {
            "status": "unavailable",
            "value": None,
            "reason": unavailable_reason,
        }
    return {
        "status": "available",
        "value": _mean(cross) / float(np.sqrt(left_mean * right_mean)),
        "reason": None,
    }


def point_reference() -> dict[str, Any]:
    table = np.genfromtxt(
        SOURCE,
        delimiter=",",
        names=True,
        dtype=np.float64,
        encoding="utf-8-sig",
    )
    names = list(table.dtype.names or ())
    matrix = np.column_stack([table[name] for name in names])
    if matrix.ndim != 2 or matrix.shape[0] < 3 or not np.isfinite(matrix).all():
        raise ValueError("corporate-reputation reference must be a finite raw matrix")
    correlations = np.corrcoef(matrix, rowvar=False, dtype=np.float64)
    constructs = list(CONSTRUCTS)

    def artifact(absolute: bool) -> list[list[dict[str, Any]]]:
        return [
            [
                (
                    {"status": "available", "value": 1.0, "reason": None}
                    if left == right
                    else _htmt_cell(
                        correlations,
                        names,
                        CONSTRUCTS[left],
                        CONSTRUCTS[right],
                        absolute=absolute,
                    )
                )
                for right in constructs
            ]
            for left in constructs
        ]

    return {
        "constructs": constructs,
        "htmt_plus": artifact(True),
        "htmt_original": artifact(False),
        "rows": int(matrix.shape[0]),
    }


def _index_digest(indices: list[int]) -> str:
    digest = hashlib.sha256()
    for index in indices:
        digest.update(int(index).to_bytes(4, byteorder="little", signed=False))
    return digest.hexdigest()


def bias_corrected_reference(
    values: list[float], original: float, confidence_level: float = 0.90
) -> dict[str, Any]:
    vector = np.asarray(values, dtype=np.float64)
    if vector.ndim != 1 or vector.size < 2 or not np.isfinite(vector).all():
        raise ValueError("at least two finite bootstrap values are required")
    below = int(np.count_nonzero(vector < original))
    tied = int(np.count_nonzero(vector == original))
    count = int(vector.size)
    probability = (below + 0.5 * tied) / count
    probability = min(max(probability, 0.5 / count), 1.0 - 0.5 / count)
    z0 = float(norm.ppf(probability))
    tail = (1.0 - confidence_level) / 2.0
    lower_probability = float(norm.cdf(2.0 * z0 + norm.ppf(tail)))
    upper_probability = float(norm.cdf(2.0 * z0 + norm.ppf(1.0 - tail)))
    lower = float(np.quantile(vector, lower_probability, method="linear"))
    upper = float(np.quantile(vector, upper_probability, method="linear"))
    return {
        "original": original,
        "bootstrap_mean": float(np.mean(vector, dtype=np.float64)),
        "bias": float(np.mean(vector, dtype=np.float64) - original),
        "standard_error": float(np.std(vector, ddof=1, dtype=np.float64)),
        "bias_correction": z0,
        "lower_probability": lower_probability,
        "upper_probability": upper_probability,
        "lower": lower,
        "upper": upper,
        "upper_bound_below_critical_value": upper < 0.90,
        "below_original": below,
        "tied_original": tied,
    }


def report() -> dict[str, Any]:
    return {
        "schema_version": 1,
        "kind": "htmt_numpy_scipy_independent_reference_v1",
        "implementations": {
            "numpy": np.__version__,
            "scipy": scipy.__version__,
        },
        "source": "validation/fixtures/corporate_reputation.csv",
        "point": point_reference(),
        "bootstrap": {
            "interval_method": "bias_corrected_percentile_type7_v1",
            "test_type": "one_tailed_upper",
            "significance_level": 0.05,
            "equivalent_two_sided_confidence_level": 0.90,
            "critical_value": 0.90,
            "scenarios": [
                {
                    **scenario,
                    "usable_replicate_indices_sha256": _index_digest(
                        list(range(len(scenario["values"])))
                    ),
                    "expected": bias_corrected_reference(
                        scenario["values"], scenario["original"]
                    ),
                }
                for scenario in BOOTSTRAP_SCENARIOS
            ],
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    document = report()
    encoded = json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n"
    if args.output:
        output = args.output if args.output.is_absolute() else ROOT / args.output
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(encoded, encoding="utf-8")
    else:
        print(encoded, end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
