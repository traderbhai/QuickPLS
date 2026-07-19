#!/usr/bin/env python3
"""Compare QuickPLS studentized interval math to supplied independent references."""

from __future__ import annotations

import csv
import json
import math
import subprocess
import sys
from pathlib import Path
from typing import Iterable

from r_runtime import find_rscript_optional

ORIGINAL = 10.0
OUTER_STANDARD_ERROR = 2.0
CONFIDENCE = 0.80
THETA_STAR = [8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 7.0, 10.5, 9.5, 11.5]
INNER_STANDARD_ERRORS = [1.0, 1.2, 0.9, 1.1, 1.3, 1.0, 0.8, 1.4, 1.05, 0.95]
TOLERANCE = 1.0e-12


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def type7_quantile(values: Iterable[float], probability: float) -> float:
    sorted_values = sorted(values)
    if not sorted_values:
        raise ValueError("cannot compute a quantile for an empty sample")
    if probability <= 0.0:
        return sorted_values[0]
    if probability >= 1.0:
        return sorted_values[-1]
    position = probability * (len(sorted_values) - 1)
    lower_index = math.floor(position)
    upper_index = math.ceil(position)
    if lower_index == upper_index:
        return sorted_values[lower_index]
    fraction = position - lower_index
    return sorted_values[lower_index] * (1.0 - fraction) + sorted_values[upper_index] * fraction


def independent_python_reference() -> dict[str, float]:
    pivots = [
        (theta - ORIGINAL) / inner_se
        for theta, inner_se in zip(THETA_STAR, INNER_STANDARD_ERRORS)
    ]
    alpha = 1.0 - CONFIDENCE
    lower_pivot = type7_quantile(pivots, alpha / 2.0)
    upper_pivot = type7_quantile(pivots, 1.0 - alpha / 2.0)
    return {
        "confidence": CONFIDENCE,
        "original": ORIGINAL,
        "outer_standard_error": OUTER_STANDARD_ERROR,
        "lower_pivot": lower_pivot,
        "upper_pivot": upper_pivot,
        "lower": ORIGINAL - upper_pivot * OUTER_STANDARD_ERROR,
        "upper": ORIGINAL - lower_pivot * OUTER_STANDARD_ERROR,
    }


def find_rscript() -> str | None:
    found = find_rscript_optional()
    return found[0] if found is not None else None


def run_r_reference(root: Path, output: Path) -> dict[str, dict[str, float | None]] | None:
    rscript = find_rscript()
    if rscript is None:
        return None
    script = root / "validation" / "studentized_supplied_reference.R"
    completed = subprocess.run(
        [rscript, "--vanilla", str(script), str(output)],
        cwd=root,
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"R studentized reference failed\nstdout:\n{completed.stdout}\n\nstderr:\n{completed.stderr}"
        )
    with output.open("r", encoding="utf-8-sig", newline="") as handle:
        rows = list(csv.DictReader(handle))
    parsed: dict[str, dict[str, float | None]] = {}
    for row in rows:
        method = row["method"]
        parsed[method] = {
            key: None if row[key] in {"", "NA"} else float(row[key])
            for key in [
                "confidence",
                "original",
                "outer_standard_error",
                "lower_pivot",
                "upper_pivot",
                "lower",
                "upper",
            ]
        }
    return parsed


def max_abs_difference(left: dict[str, float], right: dict[str, float | None], keys: list[str]) -> float:
    return max(abs(left[key] - float(right[key])) for key in keys if right[key] is not None)


def main() -> int:
    root = repo_root()
    results_dir = root / "validation" / "results"
    results_dir.mkdir(parents=True, exist_ok=True)
    python_reference = independent_python_reference()
    r_csv = results_dir / "studentized_supplied_r_reference.csv"
    r_reference = run_r_reference(root, r_csv)
    r_type7 = r_reference.get("r_type7") if r_reference else None
    boot_ci = r_reference.get("r_boot_ci_stud") if r_reference else None
    r_type7_diff = (
        max_abs_difference(
            python_reference,
            r_type7,
            [
                "confidence",
                "original",
                "outer_standard_error",
                "lower_pivot",
                "upper_pivot",
                "lower",
                "upper",
            ],
        )
        if r_type7
        else None
    )
    boot_ci_difference = (
        {
            "lower_difference_from_type7": float(boot_ci["lower"]) - python_reference["lower"],
            "upper_difference_from_type7": float(boot_ci["upper"]) - python_reference["upper"],
        }
        if boot_ci
        else None
    )
    passed = r_type7_diff is not None and r_type7_diff <= TOLERANCE
    report = {
        "schema_version": 1,
        "kind": "studentized_supplied_reference_v1",
        "passed": passed,
        "tolerance": TOLERANCE,
        "fixture": {
            "original": ORIGINAL,
            "outer_standard_error": OUTER_STANDARD_ERROR,
            "confidence": CONFIDENCE,
            "theta_star": THETA_STAR,
            "inner_standard_errors": INNER_STANDARD_ERRORS,
            "pivot_definition": "(theta_star - original) / inner_standard_error",
            "interval_definition": "[original - q_hi * outer_se, original - q_lo * outer_se]",
            "quantile_type": "Hyndman-Fan Type 7",
        },
        "independent_python": python_reference,
        "r_type7": r_type7,
        "r_type7_max_abs_difference": r_type7_diff,
        "r_boot_ci_stud": boot_ci,
        "r_boot_ci_difference_note": "R boot::boot.ci(type='stud') is recorded as a development comparison, but its endpoint interpolation is not equivalent to QuickPLS Type-7 pivot quantiles for this finite supplied-replicate fixture.",
        "r_boot_ci_difference_from_type7": boot_ci_difference,
        "r_reference_csv": str(r_csv) if r_reference else None,
    }
    output = results_dir / "studentized_supplied_reference.json"
    with output.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2)
        handle.write("\n")
    print(f"wrote {output} | passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    sys.exit(main())
