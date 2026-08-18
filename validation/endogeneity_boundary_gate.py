"""Fail-closed boundary, metamorphic, and determinism checks for endogeneity."""

from __future__ import annotations

import argparse
import math
from typing import Any

from endogeneity_factory_common import (
    optionally_write_identity_report,
    run_command,
    run_endogeneity,
)
from endogeneity_simulation import generated_rows


SOURCE = "validation/endogeneity_boundary_gate.py"
TOLERANCE = 1e-9
NUMERIC_FIELDS = (
    "path_coefficient",
    "copula_coefficient",
    "standard_error",
    "t_statistic",
    "p_value_two_sided",
    "predictor_skewness",
)


def compare_analysis(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    left_rows = {
        (row["source"], row["target"]): row for row in left["analysis"]["estimates"]
    }
    right_rows = {
        (row["source"], row["target"]): row for row in right["analysis"]["estimates"]
    }
    keys_match = set(left_rows) == set(right_rows)
    maxima = {
        field: max(
            (
                abs(float(left_rows[key][field]) - float(right_rows[key][field]))
                for key in left_rows
                if key in right_rows
            ),
            default=math.inf,
        )
        for field in NUMERIC_FIELDS
    }
    categorical_equal = keys_match and all(
        left_rows[key]["applicable"] == right_rows[key]["applicable"]
        and left_rows[key]["warning"] == right_rows[key]["warning"]
        for key in left_rows
    )
    metadata_equal = (
        left["analysis"]["method_version"] == right["analysis"]["method_version"]
        and left["analysis"]["transform"] == right["analysis"]["transform"]
        and left["analysis"]["warnings"] == right["analysis"]["warnings"]
    )
    return {
        "passed": keys_match
        and categorical_equal
        and metadata_equal
        and all(value <= TOLERANCE for value in maxima.values()),
        "keys_match": keys_match,
        "categorical_equal": categorical_equal,
        "metadata_equal": metadata_equal,
        "max_abs_errors": maxima,
    }


def run_boundaries() -> dict[str, Any]:
    rows = generated_rows(2026081499, 160, 0.4)
    baseline = run_endogeneity(name="boundary_baseline", rows=rows)

    constant_rows = [dict(row, x1=2.0, x2=2.0) for row in rows]
    constant = run_endogeneity(
        name="boundary_constant_predictor",
        rows=constant_rows,
        expect_success=False,
    )
    constant_text = (
        constant["execution"]["stdout_tail"]
        + constant["execution"]["stderr_tail"]
    ).lower()
    data_pathology = {
        "passed": constant["passed"]
        and any(word in constant_text for word in ("constant", "variance", "singular")),
        "no_completed_output": constant["passed"],
        "typed_diagnostic": any(
            word in constant_text for word in ("constant", "variance", "singular")
        ),
        "execution": constant["execution"],
    }

    pca_weighting = run_endogeneity(
        name="boundary_pca_weighting",
        rows=rows,
        weighting_scheme="pca",
        expect_success=False,
    )
    resampling = run_endogeneity(
        name="boundary_resampling",
        rows=rows,
        settings_overrides={"bootstrap_samples": 10},
        expect_success=False,
    )
    pca_text = (
        pca_weighting["execution"]["stdout_tail"]
        + pca_weighting["execution"]["stderr_tail"]
    ).lower()
    resampling_text = (
        resampling["execution"]["stdout_tail"]
        + resampling["execution"]["stderr_tail"]
    ).lower()
    unsupported_scope = {
        "passed": pca_weighting["passed"]
        and resampling["passed"]
        and "path or factor weighting" in pca_text
        and any(word in resampling_text for word in ("bootstrap", "resampling")),
        "pca_weighting_blocked": pca_weighting["passed"]
        and "path or factor weighting" in pca_text,
        "resampling_blocked": resampling["passed"]
        and any(word in resampling_text for word in ("bootstrap", "resampling")),
        "executions": [pca_weighting["execution"], resampling["execution"]],
    }

    row_reordered = run_endogeneity(
        name="boundary_rows_reversed", rows=list(reversed(rows))
    )
    path_reordered = run_endogeneity(
        name="boundary_paths_reversed",
        rows=rows,
        model_overrides={
            "paths": [
                {"source": "z", "target": "y"},
                {"source": "x", "target": "y"},
            ]
        },
    )
    row_comparison = compare_analysis(baseline, row_reordered)
    path_comparison = compare_analysis(baseline, path_reordered)
    metamorphic = {
        "passed": row_comparison["passed"] and path_comparison["passed"],
        "row_reorder": row_comparison,
        "predecessor_declaration_reorder": path_comparison,
    }

    repeated = run_endogeneity(name="boundary_repeat", rows=rows)
    worker_variant = run_endogeneity(
        name="boundary_workers_four",
        rows=rows,
        settings_overrides={"workers": 4},
    )
    repeat_comparison = compare_analysis(baseline, repeated)
    worker_comparison = compare_analysis(baseline, worker_variant)
    determinism = {
        "passed": repeat_comparison["passed"] and worker_comparison["passed"],
        "repeat": repeat_comparison,
        "worker_invariance": worker_comparison,
    }

    cargo, cargo_execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-project",
            "tests::runner_generated_endogeneity_appends_round_trips_and_rejects_contract_tampering",
            "--",
            "--exact",
        ],
        timeout=900,
    )
    cargo_text = cargo.stdout + cargo.stderr
    tamper = {
        "passed": cargo.returncode == 0
        and "1 passed" in cargo_text
        and "0 failed" in cargo_text,
        "archive_test_is_method_scoped": True,
        "execution": cargo_execution,
    }

    categories = {
        "data_pathology": data_pathology,
        "unsupported_scope": unsupported_scope,
        "metamorphic": metamorphic,
        "determinism": determinism,
        "tamper": tamper,
    }
    return {
        "passed": baseline["passed"]
        and all(category["passed"] for category in categories.values()),
        "tolerance": TOLERANCE,
        "baseline_identity": {
            "passed": baseline["identity_passed"],
            "output": baseline["output"],
            "output_sha256": baseline["output_sha256"],
        },
        "categories": categories,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-identity", action="store_true")
    args = parser.parse_args()
    detail = run_boundaries()
    report = optionally_write_identity_report(
        "boundary_report",
        write_identity=args.write_identity,
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            "validation/endogeneity_simulation.py",
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-project/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
    )
    print(
        f"endogeneity boundary passed={detail['passed']} "
        f"identity={report or 'not-written'}"
    )
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
