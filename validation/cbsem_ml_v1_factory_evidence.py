"""Orchestrate fresh lightweight identity reports for CB-SEM ML v1."""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import Any

from cbsem_ml_v1_factory_common import ROOT, run_command, write_identity_report


SOURCE = "validation/cbsem_ml_v1_factory_evidence.py"


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
    path = ROOT / "docs" / "methods" / "CBSEM_ML_V1.md"
    text = path.read_text(encoding="utf-8")
    required_fragments = [
        "`cbsem_ml_v1` is release-qualified",
        "raw-data single-group reflective ML SEM scope",
        "F_ml = log|Sigma(theta)|",
        "first-loading marker identification",
        "recursive latent regression",
        "Robust corrections",
        "WLSMV/polychoric estimators",
        "same-run reopen",
        "independent QuickPLS method claim",
    ]
    fragments = {fragment: fragment in text for fragment in required_fragments}
    checks = {
        "passed": all(fragments.values()),
        "document": "docs/methods/CBSEM_ML_V1.md",
        "required_contract_fragments": fragments,
        "bounded_scope_not_broadened": (
            "remain experimental or unsupported outside the v1.2.4 scope" in text
            and "does not assert numerical or workflow equivalence with every CB-SEM feature in SmartPLS" in text
        ),
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


def run_python_gate(script: str, timeout: int = 2400) -> bool:
    completed, execution = run_command(["python", script], timeout=timeout)
    if completed.returncode != 0:
        print(execution)
        return False
    return True


def frontend_report() -> bool:
    tests = [
        "src/native/nativeAnalysisCatalog.test.ts",
        "src/native/nativeAnalysisRecipe.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
        "src/native/nativeResults.test.ts",
        "src/native/NativeResultsSurface.test.tsx",
    ]
    command = ["npx.cmd", "vitest", "run", *tests, "--reporter=verbose"]
    completed, execution = run_command(command, timeout=1200)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=len(tests), minimum_tests=100)
    surfaced = {Path(path).name: Path(path).name in output for path in tests}
    checks = {
        "passed": completed.returncode == 0 and summary["passed"] and all(surfaced.values()),
        "vitest_summary": summary,
        "method_scoped_files": tests,
        "test_files_surfaced": surfaced,
        "catalog_and_bounded_setup_exercised": surfaced["nativeAnalysisCatalog.test.ts"]
        and surfaced["nativeAnalysisRecipe.test.ts"],
        "accessible_dialog_exercised": surfaced["NativeCalculationDialog.test.ts"],
        "typed_result_and_surface_exercised": surfaced["nativeResults.test.ts"]
        and surfaced["NativeResultsSurface.test.tsx"],
    }
    report = write_identity_report(
        "frontend_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "package.json",
            "package-lock.json",
            *tests,
            "src/native/nativeAnalysisCatalog.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/NativeCalculationDialog.tsx",
            "src/native/nativeResults.ts",
            "src/native/NativeResultsSurface.tsx",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def export_report() -> bool:
    tests = [
        "src/native/nativeExportTables.test.ts",
        "src/native/NativeExportDialog.test.ts",
    ]
    command = ["npx.cmd", "vitest", "run", *tests, "--reporter=verbose"]
    completed, execution = run_command(command, timeout=1200)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=len(tests), minimum_tests=15)
    surfaced = {Path(path).name: Path(path).name in output for path in tests}
    checks = {
        "passed": completed.returncode == 0 and summary["passed"] and all(surfaced.values()),
        "vitest_summary": summary,
        "table_projection_exercised": surfaced["nativeExportTables.test.ts"],
        "same_run_export_scope_exercised": surfaced["NativeExportDialog.test.ts"],
        "required_test_sources": tests,
    }
    report = write_identity_report(
        "export_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "package.json",
            "package-lock.json",
            *tests,
            "src/native/nativeExportTables.ts",
            "src/native/NativeExportDialog.tsx",
            "src/native/nativeResults.ts",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def run_stage(stage: str) -> bool:
    if stage == "engine":
        if not method_spec_report():
            return False
        for script in (
            "validation/cbsem_ml_v1_reference.py",
            "validation/cbsem_ml_v1_simulation.py",
            "validation/cbsem_ml_v1_boundary_gate.py",
        ):
            if not run_python_gate(script):
                return False
        return True
    if stage == "archive":
        return run_python_gate("validation/cbsem_ml_v1_persistence_gate.py")
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
    print(f"CB-SEM ML v1 factory lightweight stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
