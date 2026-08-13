"""Orchestrate lightweight, fresh identity reports for PCA v1 factory stages."""

from __future__ import annotations

import argparse
import math
import numpy as np
import re
from pathlib import Path
from typing import Any

from pca_v1_factory_common import (
    ROOT,
    WORK_ROOT,
    run_pca,
    strict_load_json,
    run_command,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/pca_v1_factory_evidence.py"


def vitest_summary(output: str, *, expected_files: int, minimum_tests: int) -> dict[str, Any]:
    files_match = re.search(r"Test Files\s+(\d+) passed \((\d+)\)", output)
    tests_match = re.search(r"Tests\s+(\d+) passed \((\d+)\)", output)
    files_passed = int(files_match.group(1)) if files_match else None
    files_total = int(files_match.group(2)) if files_match else None
    tests_passed = int(tests_match.group(1)) if tests_match else None
    tests_total = int(tests_match.group(2)) if tests_match else None
    return {
        "passed": (
            files_passed == expected_files
            and files_total == expected_files
            and tests_passed is not None
            and tests_passed == tests_total
            and tests_passed >= minimum_tests
        ),
        "expected_test_files": expected_files,
        "minimum_tests": minimum_tests,
        "observed_test_files_passed": files_passed,
        "observed_test_files_total": files_total,
        "observed_tests_passed": tests_passed,
        "observed_tests_total": tests_total,
    }


def method_spec_report() -> bool:
    path = ROOT / "docs" / "methods" / "PCA_V1.md"
    text = path.read_text(encoding="utf-8")
    required_fragments = [
        "`pca_v1` is a standalone principal component analysis workflow",
        "2 to 50 distinct selected numeric variables",
        "correlation-matrix eigensystem",
        "Signs are oriented deterministically",
        "kaiser|fixed|variance_threshold",
        "Pairwise deletion",
        "Rotation methods",
        "same-run reopen",
    ]
    fragments = {fragment: fragment in text for fragment in required_fragments}
    checks = {
        "passed": all(fragments.values()),
        "document": "docs/methods/PCA_V1.md",
        "required_contract_fragments": fragments,
        "bounded_scope_not_broadened": "SmartPLS feature parity beyond this independently implemented bounded PCA workflow" in text,
    }
    checks["passed"] = checks["passed"] and checks["bounded_scope_not_broadened"]
    report = write_identity_report(
        "method_spec",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def independent_reference_report() -> bool:
    from pca_simulation import compare, independent_reference

    sample_size = 8
    correlation = 0.5
    first = np.arange(sample_size, dtype=float) - (sample_size - 1) / 2
    first /= first.std(ddof=1)
    raw_second = np.asarray([1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 0.5, -0.5])
    raw_second -= raw_second.mean()
    raw_second -= first * (np.dot(raw_second, first) / np.dot(first, first))
    orthogonal = raw_second / raw_second.std(ddof=1)
    second = correlation * first + math.sqrt(1.0 - correlation**2) * orthogonal
    rows = [[float(first[index]), float(second[index])] for index in range(sample_size)]
    variables = ["x", "y"]
    csv_path = WORK_ROOT / "independent_hand_reference.csv"
    write_csv(csv_path, variables, rows)
    reference = independent_reference(rows)
    result = run_pca(
        name="factory_independent_hand_reference",
        csv_path=csv_path,
        variables=variables,
        rule="fixed",
        components=2,
    )
    comparison = compare(result, variables, reference, 2)
    observed_eigenvalues = [row["eigenvalue"] for row in result["pca"]["components"]]
    expected_eigenvalues = [1.5, 0.5]
    hand_max_error = max(
        abs(observed - expected)
        for observed, expected in zip(observed_eigenvalues, expected_eigenvalues)
    )
    hand_passed = comparison["passed"] and hand_max_error <= 1e-6

    numpy, numpy_execution = run_command(
        ["python", "validation/pls_pca_reference.py"],
        timeout=300,
    )
    numpy_path = ROOT / "validation" / "results" / "pls_pca_numpy_reference.json"
    numpy_report = strict_load_json(numpy_path) if numpy_path.is_file() else {}
    numeric_values = [
        row.get("value") for row in numpy_report.get("paths", [])
    ] + [
        value
        for row in numpy_report.get("outer", [])
        for value in (row.get("loading"), row.get("weight"))
    ]
    numpy_passed = (
        numpy.returncode == 0
        and numpy_report.get("engine") == "numpy-eigh"
        and numpy_report.get("variant") == "PCA"
        and len(numeric_values) == 9
        and all(isinstance(value, (int, float)) and math.isfinite(value) for value in numeric_values)
    )
    checks = {
        "passed": hand_passed and numpy_passed,
        "standalone_hand_and_numpy_comparison": {
            "passed": hand_passed,
            "expected_eigenvalues": expected_eigenvalues,
            "observed_eigenvalues": observed_eigenvalues,
            "hand_max_abs_error": hand_max_error,
            "full_payload_comparison": comparison,
            "output": result["output"],
            "output_sha256": result["output_sha256"],
        },
        "independent_pca_eigensystem_sanity": {
            "passed": numpy_passed,
            "report": "validation/results/pls_pca_numpy_reference.json",
            "finite_numeric_values": len(numeric_values),
            "execution": numpy_execution,
        },
    }
    report = write_identity_report(
        "independent_reference",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/pca_simulation.py",
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
            "validation/results/pls_pca_numpy_reference.json",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def run_python_gate(script: str) -> bool:
    completed, execution = run_command(["python", script], timeout=1800)
    if completed.returncode != 0:
        print(execution)
        return False
    return True


def frontend_report() -> bool:
    command = [
        "npx.cmd",
        "vitest",
        "run",
        "src/native/nativePca.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
        "src/native/nativeResults.test.ts",
        "src/native/nativeAnalysisRecipe.test.ts",
        "--reporter=verbose",
    ]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=4, minimum_tests=108)
    checks = {
        "passed": completed.returncode == 0 and summary["passed"],
        "vitest_summary": summary,
        "method_scoped_files": command[3:-1],
        "setup_readiness_blockers_exercised": "nativePca.test.ts" in output,
        "accessible_setup_render_exercised": "NativeCalculationDialog.test.ts" in output,
        "typed_result_projection_and_tamper_exercised": "nativeResults.test.ts" in output,
        "typed_recipe_identity_exercised": "nativeAnalysisRecipe.test.ts" in output,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "setup_readiness_blockers_exercised",
            "accessible_setup_render_exercised",
            "typed_result_projection_and_tamper_exercised",
            "typed_recipe_identity_exercised",
        )
    )
    report = write_identity_report(
        "frontend_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/nativeResults.test.ts",
            "src/native/nativeAnalysisRecipe.test.ts",
            "src/native/nativePca.ts",
            "src/native/nativeResults.ts",
            "src/native/nativeAnalysisRecipe.ts",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def export_report() -> bool:
    command = [
        "npx.cmd",
        "vitest",
        "run",
        "src/native/nativeExportTables.test.ts",
        "src/native/NativeExportDialog.test.ts",
        "--reporter=verbose",
    ]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=2, minimum_tests=18)
    checks = {
        "passed": completed.returncode == 0 and summary["passed"],
        "vitest_summary": summary,
        "table_projection_exercised": "nativeExportTables.test.ts" in output,
        "same_run_export_scope_exercised": "NativeExportDialog.test.ts" in output,
        "required_test_sources": command[3:-1],
    }
    checks["passed"] = checks["passed"] and checks["table_projection_exercised"] and checks["same_run_export_scope_exercised"]
    report = write_identity_report(
        "export_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/NativeExportDialog.test.ts",
            "src/native/nativeExportTables.ts",
            "src/native/NativeExportDialog.tsx",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def run_stage(stage: str) -> bool:
    if stage == "engine":
        return all(
            [
                method_spec_report(),
                independent_reference_report(),
                run_python_gate("validation/pca_simulation.py"),
                run_python_gate("validation/pca_boundary_gate.py"),
            ]
        )
    if stage == "archive":
        return run_python_gate("validation/pca_persistence_gate.py")
    if stage == "native":
        return frontend_report() and export_report()
    if stage == "all-light":
        return run_stage("engine") and run_stage("archive") and run_stage("native")
    raise ValueError(stage)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage",
        choices=["engine", "archive", "native", "all-light"],
        default="all-light",
    )
    args = parser.parse_args()
    passed = run_stage(args.stage)
    print(f"PCA v1 factory lightweight stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
