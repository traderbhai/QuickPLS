"""Fail-closed boundary and metamorphic checks for standalone PCA v1."""

from __future__ import annotations

import copy
from typing import Any

import numpy as np

from pca_simulation import scenario_rows
from pca_v1_factory_common import WORK_ROOT, run_command, run_pca, write_csv, write_identity_report


SOURCE = "validation/pca_boundary_gate.py"
PROJECT_SOURCE = "crates/qpls-project/src/lib.rs"
TOLERANCE = 1e-6


def mapped_payload_errors(
    left: dict[str, Any],
    right: dict[str, Any],
    *,
    row_map: list[int] | None = None,
) -> dict[str, Any]:
    left_pca = left["pca"]
    right_pca = right["pca"]
    component_error = max(
        abs(a["eigenvalue"] - b["eigenvalue"])
        for a, b in zip(left_pca["components"], right_pca["components"])
    )
    left_loadings = {
        (row["variable"], row["component"]): (row["loading"], row["weight"])
        for row in left_pca["loadings"]
    }
    right_loadings = {
        (row["variable"], row["component"]): (row["loading"], row["weight"])
        for row in right_pca["loadings"]
    }
    loading_error = max(
        abs(left_loadings[key][index] - right_loadings[key][index])
        for key in left_loadings
        for index in (0, 1)
    )
    left_scores = {
        (row["observation"], row["component"]): row["score"]
        for row in left_pca["scores"]
    }
    right_scores = {
        (row["observation"], row["component"]): row["score"]
        for row in right_pca["scores"]
    }
    if row_map is None:
        row_map = list(range(left_pca["observations"]))
    component_ids = [row["component"] for row in left_pca["components"]]
    score_error = max(
        abs(left_scores[(source_row, component)] - right_scores[(target_row, component)])
        for target_row, source_row in enumerate(row_map)
        for component in component_ids
    )
    return {
        "passed": max(component_error, loading_error, score_error) <= TOLERANCE,
        "component_max_abs_error": component_error,
        "loading_or_weight_max_abs_error": loading_error,
        "score_max_abs_error": score_error,
    }


def run_boundaries() -> dict[str, Any]:
    variables = ["a", "b", "c", "d"]
    rows = scenario_rows(
        41,
        n=72,
        p=4,
        non_gaussian=True,
        missing=False,
        near_tie=False,
    )
    baseline_path = WORK_ROOT / "boundary_baseline.csv"
    write_csv(baseline_path, variables, rows)
    baseline = run_pca(
        name="factory_boundary_baseline",
        csv_path=baseline_path,
        variables=variables,
        rule="fixed",
        components=3,
    )

    constant_path = WORK_ROOT / "boundary_constant.csv"
    constant_rows = [row[:3] + [5.0] for row in rows]
    write_csv(constant_path, variables, constant_rows)
    constant = run_pca(
        name="factory_boundary_constant",
        csv_path=constant_path,
        variables=variables,
        rule="fixed",
        components=2,
        expect_success=False,
    )
    constant_text = (
        constant["execution"]["stdout_tail"] + constant["execution"]["stderr_tail"]
    ).lower()
    data_pathology = {
        "passed": constant["passed"] and "constant" in constant_text,
        "no_partial_result": constant["passed"],
        "diagnostic_mentions_constant": "constant" in constant_text,
        "execution": constant["execution"],
    }

    unsupported = run_pca(
        name="factory_boundary_unsupported_components",
        csv_path=baseline_path,
        variables=variables,
        rule="fixed",
        components=5,
        expect_success=False,
    )
    unsupported_text = (
        unsupported["execution"]["stdout_tail"] + unsupported["execution"]["stderr_tail"]
    ).lower()
    unsupported_scope = {
        "passed": unsupported["passed"]
        and ("component" in unsupported_text or "retention" in unsupported_text),
        "no_partial_result": unsupported["passed"],
        "typed_diagnostic": "component" in unsupported_text or "retention" in unsupported_text,
        "execution": unsupported["execution"],
    }

    reverse_path = WORK_ROOT / "boundary_rows_reversed.csv"
    write_csv(reverse_path, variables, list(reversed(rows)))
    reversed_rows = run_pca(
        name="factory_boundary_rows_reversed",
        csv_path=reverse_path,
        variables=variables,
        rule="fixed",
        components=3,
    )
    row_reorder = mapped_payload_errors(
        baseline,
        reversed_rows,
        row_map=list(reversed(range(len(rows)))),
    )

    reversed_variables = list(reversed(variables))
    variable_path = WORK_ROOT / "boundary_variables_reversed.csv"
    variable_rows = [list(reversed(row)) for row in rows]
    write_csv(variable_path, reversed_variables, variable_rows)
    variable_reorder_result = run_pca(
        name="factory_boundary_variables_reversed",
        csv_path=variable_path,
        variables=reversed_variables,
        rule="fixed",
        components=3,
    )
    variable_reorder = mapped_payload_errors(baseline, variable_reorder_result)
    metamorphic = {
        "passed": row_reorder["passed"] and variable_reorder["passed"],
        "row_reorder": row_reorder,
        "variable_reorder": variable_reorder,
    }

    repeated = run_pca(
        name="factory_boundary_repeat",
        csv_path=baseline_path,
        variables=variables,
        rule="fixed",
        components=3,
    )
    deterministic = {
        "passed": baseline["pca"] == repeated["pca"],
        "payloads_exactly_equal": baseline["pca"] == repeated["pca"],
        "first_output_sha256": baseline["output_sha256"],
        "second_output_sha256": repeated["output_sha256"],
    }

    cargo, cargo_execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-project",
            "tests::runner_generated_pca_v1_commits_saves_reopens_and_rejects_contract_tampering",
            "--",
        ],
        timeout=900,
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
    detail = run_boundaries()
    path = write_identity_report(
        "boundary_report",
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            PROJECT_SOURCE,
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
    )
    print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
