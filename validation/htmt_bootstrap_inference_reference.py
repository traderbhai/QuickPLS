#!/usr/bin/env python3
"""Transparent HTMT bias-corrected bootstrap interval oracle.

This validation-only implementation has no QuickPLS imports.  It implements
the bias-corrected percentile (BC, not BCa) interval used for the documented
one-tailed HTMT workflow: a 90% two-sided interval has the same 95th-percentile
upper endpoint as a one-tailed alpha=.05 test.
"""

from __future__ import annotations

import json
import hashlib
import math
from statistics import NormalDist


NORMAL = NormalDist()
CRITICAL_VALUE = 0.90
DECISION_RULE = "bias_corrected_upper_bound_strictly_below_critical_value_v1"
INDEX_DIGEST_METHOD = "sha256_u32_le_v1"


def replicate_index_digest(indices: list[int]) -> str:
    digest = hashlib.sha256()
    for index in indices:
        digest.update(index.to_bytes(4, byteorder="little", signed=False))
    return digest.hexdigest()


def type7_quantile(values: list[float], probability: float) -> float:
    if not values:
        raise ValueError("at least one value is required")
    if not 0.0 <= probability <= 1.0:
        raise ValueError("probability must be in [0, 1]")
    ordered = sorted(values)
    position = (len(ordered) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (position - lower) * (ordered[upper] - ordered[lower])


def bias_corrected_percentile(
    values: list[float], original: float, confidence_level: float = 0.90
) -> dict[str, float | int]:
    if len(values) < 2 or not all(math.isfinite(value) for value in values):
        raise ValueError("at least two finite bootstrap values are required")
    if not math.isfinite(original) or not 0.0 < confidence_level < 1.0:
        raise ValueError("invalid original value or confidence level")
    below = sum(value < original for value in values)
    tied = sum(value == original for value in values)
    count = len(values)
    probability = (below + 0.5 * tied) / count
    probability = min(max(probability, 0.5 / count), 1.0 - 0.5 / count)
    bias_correction = NORMAL.inv_cdf(probability)
    tail = (1.0 - confidence_level) / 2.0
    lower_probability = NORMAL.cdf(2.0 * bias_correction + NORMAL.inv_cdf(tail))
    upper_probability = NORMAL.cdf(
        2.0 * bias_correction + NORMAL.inv_cdf(1.0 - tail)
    )
    mean = sum(values) / count
    standard_error = math.sqrt(
        sum((value - mean) ** 2 for value in values) / (count - 1)
    )
    upper = type7_quantile(values, upper_probability)
    return {
        "original": original,
        "bootstrap_mean": mean,
        "bias": mean - original,
        "standard_error": standard_error,
        "bias_correction": bias_correction,
        "lower_probability": lower_probability,
        "upper_probability": upper_probability,
        "lower": type7_quantile(values, lower_probability),
        "upper": upper,
        "upper_bound_below_critical_value": upper < CRITICAL_VALUE,
        "below_original": below,
        "tied_original": tied,
    }


def report() -> dict:
    scenarios = [
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
    return {
        "schema_version": 1,
        "method": "bias_corrected_percentile_type7_v1",
        "test_type": "one_tailed_upper",
        "significance_level": 0.05,
        "equivalent_two_sided_confidence_level": 0.90,
        "critical_value": CRITICAL_VALUE,
        "decision_rule": DECISION_RULE,
        "replicate_index_digest_method": INDEX_DIGEST_METHOD,
        "scenarios": [
            {
                **scenario,
                "usable_replicate_indices": list(range(len(scenario["values"]))),
                "usable_replicate_indices_sha256": replicate_index_digest(
                    list(range(len(scenario["values"])))
                ),
                "expected": bias_corrected_percentile(
                    scenario["values"], scenario["original"]
                ),
            }
            for scenario in scenarios
        ],
    }


if __name__ == "__main__":
    print(json.dumps(report(), indent=2, sort_keys=True))
