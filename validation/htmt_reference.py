#!/usr/bin/env python3
"""Independent HTMT/HTMT+ fixture generator for QuickPLS.

This script intentionally does not call the Rust engine. It reads raw CSV data,
computes Pearson correlations with sample covariance, and applies the frozen
HTMT/HTMT+ formulas from docs/methods/PLS_HTMT_V1.md.
"""

from __future__ import annotations

import csv
import json
import math
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "validation" / "fixtures" / "corporate_reputation.csv"
OUTPUT = ROOT / "validation" / "results" / "htmt_reference.json"

CONSTRUCTS = {
    "comp": ["COMP1", "COMP2", "COMP3"],
    "like": ["LIKE1", "LIKE2"],
    "satisfaction": ["CUSA1", "CUSA2"],
    "loyalty": ["CUSL1", "CUSL2"],
}


def read_rows(path: Path) -> list[dict[str, float]]:
    with path.open(newline="", encoding="utf-8") as handle:
        return [
            {key: float(value) for key, value in row.items()}
            for row in csv.DictReader(handle)
        ]


def pearson(left: list[float], right: list[float]) -> float:
    if len(left) != len(right) or len(left) < 2:
        raise ValueError("Pearson correlation requires equal vectors with at least two values")
    left_mean = sum(left) / len(left)
    right_mean = sum(right) / len(right)
    numerator = sum((x - left_mean) * (y - right_mean) for x, y in zip(left, right))
    left_ss = sum((x - left_mean) ** 2 for x in left)
    right_ss = sum((y - right_mean) ** 2 for y in right)
    denominator = math.sqrt(left_ss * right_ss)
    if denominator == 0:
        raise ValueError("constant column")
    return numerator / denominator


def columns(rows: list[dict[str, float]], transform=None) -> dict[str, list[float]]:
    output = {key: [row[key] for row in rows] for key in rows[0]}
    if transform:
        output = transform(output)
    return output


def htmt_cell(
    construct_a: list[str],
    construct_b: list[str],
    data: dict[str, list[float]],
    absolute: bool,
) -> dict[str, float | str | None]:
    if len(construct_a) < 2 or len(construct_b) < 2:
        return {"status": "not_applicable", "value": None, "reason": "htmt.single_indicator_not_applicable"}
    cross = [
        pearson(data[left], data[right])
        for left in construct_a
        for right in construct_b
    ]
    within_a = [
        pearson(data[construct_a[i]], data[construct_a[j]])
        for i in range(len(construct_a))
        for j in range(i + 1, len(construct_a))
    ]
    within_b = [
        pearson(data[construct_b[i]], data[construct_b[j]])
        for i in range(len(construct_b))
        for j in range(i + 1, len(construct_b))
    ]
    if absolute:
        cross_mean = sum(abs(value) for value in cross) / len(cross)
        left_mean = sum(abs(value) for value in within_a) / len(within_a)
        right_mean = sum(abs(value) for value in within_b) / len(within_b)
        reason = "htmt.zero_monotrait_denominator"
    else:
        cross_mean = sum(cross) / len(cross)
        left_mean = sum(within_a) / len(within_a)
        right_mean = sum(within_b) / len(within_b)
        reason = "htmt.original_nonpositive_monotrait_mean"
    if left_mean <= 64 * 2.220446049250313e-16 or right_mean <= 64 * 2.220446049250313e-16:
        return {"status": "unavailable", "value": None, "reason": reason}
    return {
        "status": "available",
        "value": cross_mean / math.sqrt(left_mean * right_mean),
        "reason": None,
    }


def matrix(data: dict[str, list[float]], constructs: dict[str, list[str]], absolute: bool) -> list[list[dict[str, float | str | None]]]:
    keys = list(constructs)
    rows = []
    for left in keys:
        row = []
        for right in keys:
            if left == right:
                row.append({"status": "available", "value": 1.0, "reason": None})
            else:
                row.append(htmt_cell(constructs[left], constructs[right], data, absolute))
        rows.append(row)
    return rows


def transform_positive_affine(data: dict[str, list[float]]) -> dict[str, list[float]]:
    output = {}
    for index, (key, values) in enumerate(data.items(), start=1):
        output[key] = [value * (index + 1.5) + index * 7.0 for value in values]
    return output


def transform_reverse_one_indicator(data: dict[str, list[float]]) -> dict[str, list[float]]:
    output = {key: values[:] for key, values in data.items()}
    output["COMP1"] = [-value for value in output["COMP1"]]
    return output


def summarize(name: str, data: dict[str, list[float]], constructs=CONSTRUCTS) -> dict:
    return {
        "name": name,
        "constructs": list(constructs),
        "htmt_plus": matrix(data, constructs, True),
        "htmt_original": matrix(data, constructs, False),
    }


def max_cell_delta(first: list[list[dict]], second: list[list[dict]]) -> float:
    maximum = 0.0
    for row_a, row_b in zip(first, second):
        for cell_a, cell_b in zip(row_a, row_b):
            if cell_a["value"] is not None and cell_b["value"] is not None:
                maximum = max(maximum, abs(cell_a["value"] - cell_b["value"]))
            elif cell_a["value"] != cell_b["value"]:
                return math.inf
    return maximum


def main() -> None:
    rows = read_rows(SOURCE)
    baseline_data = columns(rows)
    baseline = summarize("baseline", baseline_data)
    positive_affine = summarize("positive_affine", columns(rows, transform_positive_affine))
    reversed_one = summarize("reverse_COMP1", columns(rows, transform_reverse_one_indicator))
    permuted_constructs = {
        "loyalty": CONSTRUCTS["loyalty"],
        "satisfaction": CONSTRUCTS["satisfaction"],
        "like": CONSTRUCTS["like"],
        "comp": CONSTRUCTS["comp"],
    }
    permuted = summarize("permuted_constructs", baseline_data, permuted_constructs)
    report = {
        "schema_version": 1,
        "method_versions": {
            "htmt_plus": "ringle_et_al_htmt_plus_v1",
            "htmt_original": "henseler_et_al_htmt_v1",
        },
        "source": "validation/fixtures/corporate_reputation.csv",
        "correlation": "Pearson sample correlation on complete cases",
        "fixtures": [baseline, positive_affine, reversed_one, permuted],
        "metamorphic_checks": {
            "positive_affine_htmt_plus_max_delta": max_cell_delta(baseline["htmt_plus"], positive_affine["htmt_plus"]),
            "positive_affine_original_max_delta": max_cell_delta(baseline["htmt_original"], positive_affine["htmt_original"]),
            "reverse_one_indicator_plus_max_delta": max_cell_delta(baseline["htmt_plus"], reversed_one["htmt_plus"]),
            "reverse_one_indicator_original_has_unavailable": any(
                cell["status"] == "unavailable"
                for row in reversed_one["htmt_original"]
                for cell in row
            ),
        },
        "note": "Independent formula fixture for current QuickPLS validation. This is not a second SEM-engine reference.",
    }
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(f"wrote {OUTPUT}")


if __name__ == "__main__":
    main()
