"""Fail-closed boundary and metamorphic checks for CB-SEM ML v1."""

from __future__ import annotations

import math
import random
from typing import Any

from cbsem_ml_v1_factory_common import (
    WORK_ROOT,
    engine_source_paths,
    run_cbsem,
    run_command,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/cbsem_ml_v1_boundary_gate.py"
TOLERANCE = 1e-6
VARIABLES = ["x1", "x2", "x3", "m1", "m2", "m3", "y1", "y2", "y3"]
CONSTRUCTS = [
    ("x", ["x1", "x2", "x3"]),
    ("m", ["m1", "m2", "m3"]),
    ("y", ["y1", "y2", "y3"]),
]
PATHS = [("x", "m"), ("x", "y"), ("m", "y")]


def generated_rows(n: int = 240, seed: int = 20260813) -> list[list[float]]:
    rng = random.Random(seed)
    rows: list[list[float]] = []
    for _ in range(n):
        x = rng.gauss(0.0, 1.0)
        m = 0.55 * x + rng.gauss(0.0, 0.75)
        y = 0.30 * x + 0.50 * m + rng.gauss(0.0, 0.65)
        rows.append(
            [
                x + rng.gauss(0.0, 0.25),
                0.82 * x + rng.gauss(0.0, 0.28),
                0.74 * x + rng.gauss(0.0, 0.30),
                m + rng.gauss(0.0, 0.24),
                0.84 * m + rng.gauss(0.0, 0.27),
                0.72 * m + rng.gauss(0.0, 0.30),
                y + rng.gauss(0.0, 0.24),
                0.82 * y + rng.gauss(0.0, 0.27),
                0.70 * y + rng.gauss(0.0, 0.30),
            ]
        )
    return rows


def _numeric_map(cbsem: dict[str, Any]) -> tuple[dict[str, float], dict[str, Any]]:
    numeric: dict[str, float] = {}
    structural: dict[str, Any] = {
        "method_version": cbsem.get("method_version"),
        "model_type": cbsem.get("model_type"),
        "estimator": cbsem.get("estimator"),
        "input": cbsem.get("input"),
        "mean_structure": cbsem.get("mean_structure"),
        "converged": cbsem.get("converged"),
        "sample_size": cbsem.get("sample_size"),
    }
    for field in ("objective", "gradient_norm"):
        value = cbsem.get(field)
        if isinstance(value, (int, float)) and math.isfinite(float(value)):
            numeric[field] = float(value)
    for row in cbsem.get("parameters", []):
        name = row.get("name")
        structural[f"parameter:{name}:kind"] = row.get("kind")
        structural[f"parameter:{name}:fixed"] = row.get("fixed")
        for field in ("estimate", "standard_error", "z_statistic", "p_value_two_sided"):
            value = row.get(field)
            if isinstance(value, (int, float)) and math.isfinite(float(value)):
                numeric[f"parameter:{name}:{field}"] = float(value)
    for row in cbsem.get("standardized", []):
        name = row.get("name")
        for field in ("std_lv", "std_all"):
            value = row.get(field)
            if isinstance(value, (int, float)) and math.isfinite(float(value)):
                numeric[f"standardized:{name}:{field}"] = float(value)
    fit = cbsem.get("fit", {})
    structural["fit:method_version"] = fit.get("method_version")
    for field, value in fit.items():
        if isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(float(value)):
            numeric[f"fit:{field}"] = float(value)
    for field in (
        "implied_covariance",
        "residual_covariance",
        "residual_correlation",
    ):
        for row in cbsem.get(field, []):
            key = f"{field}:{row.get('row')}:{row.get('column')}"
            value = row.get("value")
            if isinstance(value, (int, float)) and math.isfinite(float(value)):
                numeric[key] = float(value)
    return numeric, structural


def compare_mapped(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    left_numeric, left_structural = _numeric_map(left)
    right_numeric, right_structural = _numeric_map(right)
    keys_equal = set(left_numeric) == set(right_numeric)
    maximum = max(
        (
            abs(left_numeric[key] - right_numeric[key])
            for key in set(left_numeric) & set(right_numeric)
        ),
        default=math.inf,
    )
    return {
        "passed": keys_equal and left_structural == right_structural and maximum <= TOLERANCE,
        "numeric_keys_equal": keys_equal,
        "structural_values_equal": left_structural == right_structural,
        "maximum_absolute_error": maximum,
        "compared_numeric_values": len(set(left_numeric) & set(right_numeric)),
    }


def run_boundaries() -> dict[str, Any]:
    rows = generated_rows()
    baseline_path = WORK_ROOT / "boundary_baseline.csv"
    write_csv(baseline_path, VARIABLES, rows)
    baseline = run_cbsem(
        name="cbsem_ml_v1_boundary_baseline",
        csv_path=baseline_path,
        constructs=CONSTRUCTS,
        paths=PATHS,
    )

    singular_path = WORK_ROOT / "boundary_singular.csv"
    singular_rows = [
        [row[0], row[0], row[0], row[3], row[3], row[3], row[6], row[6], row[6]]
        for row in rows
    ]
    write_csv(singular_path, VARIABLES, singular_rows)
    singular = run_cbsem(
        name="cbsem_ml_v1_boundary_singular",
        csv_path=singular_path,
        constructs=CONSTRUCTS,
        paths=PATHS,
        expect_success=False,
    )
    singular_text = (
        singular["execution"]["stdout_tail"] + singular["execution"]["stderr_tail"]
    ).lower()
    data_pathology = {
        "passed": singular["passed"] and any(
            token in singular_text
            for token in ("positive", "singular", "definite", "covariance", "cb-sem")
        ),
        "no_partial_result": singular["passed"],
        "typed_diagnostic": singular_text[-2000:],
    }

    unsupported = run_cbsem(
        name="cbsem_ml_v1_boundary_robust_ml",
        csv_path=baseline_path,
        constructs=CONSTRUCTS,
        paths=PATHS,
        estimator="robust_ml",
        expect_success=False,
    )
    unsupported_text = (
        unsupported["execution"]["stdout_tail"]
        + unsupported["execution"]["stderr_tail"]
    ).lower()
    unsupported_scope = {
        "passed": unsupported["passed"]
        and "cbsem.ml_required" in unsupported_text
        and "maximum likelihood" in unsupported_text,
        "no_partial_result": unsupported["passed"],
        "typed_diagnostic": unsupported_text[-2000:],
    }

    reversed_rows_path = WORK_ROOT / "boundary_rows_reversed.csv"
    write_csv(reversed_rows_path, VARIABLES, list(reversed(rows)))
    reversed_rows = run_cbsem(
        name="cbsem_ml_v1_boundary_rows_reversed",
        csv_path=reversed_rows_path,
        constructs=CONSTRUCTS,
        paths=PATHS,
    )
    row_reorder = compare_mapped(baseline["cbsem"], reversed_rows["cbsem"])

    reversed_variables = list(reversed(VARIABLES))
    reversed_columns_path = WORK_ROOT / "boundary_columns_reversed.csv"
    write_csv(
        reversed_columns_path,
        reversed_variables,
        [list(reversed(row)) for row in rows],
    )
    reversed_columns = run_cbsem(
        name="cbsem_ml_v1_boundary_columns_reversed",
        csv_path=reversed_columns_path,
        constructs=CONSTRUCTS,
        paths=PATHS,
    )
    column_reorder = compare_mapped(baseline["cbsem"], reversed_columns["cbsem"])
    metamorphic = {
        "passed": row_reorder["passed"] and column_reorder["passed"],
        "row_reorder": row_reorder,
        "observed_column_reorder": column_reorder,
    }

    repeated = run_cbsem(
        name="cbsem_ml_v1_boundary_baseline",
        csv_path=baseline_path,
        constructs=CONSTRUCTS,
        paths=PATHS,
    )
    deterministic = {
        "passed": baseline["cbsem"] == repeated["cbsem"],
        "analytical_payloads_exactly_equal": baseline["cbsem"] == repeated["cbsem"],
        "first_output_sha256": baseline["output_sha256"],
        "second_output_sha256": repeated["output_sha256"],
    }

    cargo, cargo_execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-project",
            "runner_generated_cbsem_and_cfa_commit_save_reopen_and_reject_contract_tampering",
            "--",
        ],
        timeout=1800,
    )
    cargo_text = cargo.stdout + cargo.stderr
    tamper = {
        "passed": cargo.returncode == 0 and "1 passed" in cargo_text and "0 failed" in cargo_text,
        "fresh_method_scoped_archive_test": True,
        "execution": cargo_execution,
    }

    categories = {
        "data_pathology": data_pathology,
        "unsupported_scope": unsupported_scope,
        "metamorphic": metamorphic,
        "determinism": deterministic,
        "tamper": tamper,
    }
    return {
        "passed": baseline["passed"] and all(row["passed"] for row in categories.values()),
        "tolerance": TOLERANCE,
        "baseline_identity": {
            "passed": baseline["identity_passed"],
            "output": baseline["output"],
            "output_sha256": baseline["output_sha256"],
        },
        "categories": categories,
    }


def main() -> int:
    checks = run_boundaries()
    path = write_identity_report(
        "boundary_report",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE, *engine_source_paths()],
    )
    print(f"wrote {path} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
