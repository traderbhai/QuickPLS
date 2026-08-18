"""Orchestrate fresh identity reports for the bounded CTA-PLS v1 factory."""

from __future__ import annotations

import argparse
import json
import re
import time
from pathlib import Path
from typing import Any

from cta_pls_v1_factory_common import (
    ROOT,
    ensure_cli,
    repository_path,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/cta_pls_v1_factory_evidence.py"
REFERENCE_SOURCE = "validation/cta_pls_reference.py"
REFERENCE_REPORT = ROOT / "validation" / "results" / "cta_pls_reference_report.json"
REFERENCE_RESULT = ROOT / "validation" / "results" / "cta_pls_reference_quickpls.json"


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
    path = ROOT / "docs" / "methods" / "PLS_CTA_PLS_V1.md"
    text = path.read_text(encoding="utf-8")
    required_fragments = [
        "cta_pls_tetrad_v1",
        "sample covariances of the preprocessed indicator columns",
        "each indicator quadruple emits three tetrad pairings",
        "reflective and formative blocks are both reportable",
        "same-run CSV/HTML/XLSX provenance export",
        "bootstrap, permutation, or asymptotic tetrad inference",
        "remain unsupported",
    ]
    fragments = {fragment: fragment in text for fragment in required_fragments}
    bounded_scope = (
        "scoped Standard QuickPLS workflow" in text
        and "descriptive scope" in text
        and "does not classify either measurement shape" in text
        and "Broader inferential CTA-PLS decision rules remain unsupported" in text
    )
    checks = {
        "passed": all(fragments.values()) and bounded_scope,
        "document": "docs/methods/PLS_CTA_PLS_V1.md",
        "required_contract_fragments": fragments,
        "bounded_descriptive_scope": bounded_scope,
    }
    report = write_identity_report(
        "method_spec",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def independent_reference_report() -> bool:
    cli_identity = ensure_cli()
    started_ns = time.time_ns()
    completed, execution = run_command(["python", REFERENCE_SOURCE], timeout=1200)
    report = strict_load_json(REFERENCE_REPORT) if REFERENCE_REPORT.is_file() else {}
    result = strict_load_json(REFERENCE_RESULT) if REFERENCE_RESULT.is_file() else {}
    artifacts = [
        REFERENCE_REPORT,
        REFERENCE_RESULT,
        ROOT / "validation" / "results" / "cta_pls_reference.csv",
        ROOT / "validation" / "results" / "cta_pls_reference.recipe.json",
        ROOT / "validation" / "results" / "cta_pls_invalid.recipe.json",
    ]
    freshness = []
    for path in artifacts:
        fresh = path.is_file() and path.stat().st_size > 0 and path.stat().st_mtime_ns >= started_ns - 2_000_000_000
        freshness.append(
            {
                "path": repository_path(path),
                "passed": fresh,
                "size": path.stat().st_size if path.is_file() else None,
                "sha256": sha256_file(path) if path.is_file() else None,
            }
        )
    estimation = result.get("payload", {}).get("estimation", {})
    cta = estimation.get("cta_pls", {})
    checks = {
        "passed": (
            completed.returncode == 0
            and report.get("passed") is True
            and report.get("kind") == "cta_pls_reference_v1"
            and isinstance(report.get("max_delta"), (int, float))
            and report["max_delta"] <= 1e-10
            and report.get("checks", {}).get("estimate_count") is True
            and report.get("checks", {}).get("guard") is True
            and estimation.get("method_version") == "cta_pls_tetrad_v1"
            and cta.get("method_version") == "cta_pls_tetrad_v1"
            and len(cta.get("estimates", [])) == 3
            and all(row["passed"] for row in freshness)
        ),
        "fresh_generation_only": True,
        "legacy_reports_accepted": False,
        "independence": {
            "engine": "independent Python sample-covariance implementation",
            "quickpls_imported_by_reference_math": False,
            "runtime_policy": "development validation only; never packaged with QuickPLS",
        },
        "cli_build_identity": cli_identity,
        "reference_report": report,
        "exact_current_payload_identity": {
            "estimation_method_version": estimation.get("method_version"),
            "cta_method_version": cta.get("method_version"),
            "pairing_count": len(cta.get("estimates", [])),
        },
        "generated_artifacts": freshness,
        "execution": execution,
    }
    identity = write_identity_report(
        "independent_reference",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            REFERENCE_SOURCE,
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-core/src/validation.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
            *(repository_path(path) for path in artifacts if path.is_file()),
        ],
    )
    print(f"wrote {identity} | passed={checks['passed']}")
    return checks["passed"]


def run_python_gate(script: str) -> bool:
    completed, execution = run_command(["python", script], timeout=1800)
    if completed.returncode != 0:
        print(json.dumps(execution, indent=2))
        return False
    return True


def frontend_report() -> bool:
    command = [
        "npx.cmd",
        "vitest",
        "run",
        "src/native/nativeCtaPls.test.ts",
        "src/native/nativeCtaPlsResults.test.ts",
        "src/native/nativeAnalysisRecipe.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
        "--reporter=verbose",
    ]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=4, minimum_tests=40)
    checks = {
        "passed": completed.returncode == 0 and summary["passed"],
        "vitest_summary": summary,
        "method_scoped_setup_contract": "nativeCtaPls.test.ts" in output,
        "typed_result_and_tamper_contract": "nativeCtaPlsResults.test.ts" in output,
        "typed_recipe_and_scope_contract": "nativeAnalysisRecipe.test.ts" in output,
        "accessible_dialog_contract": "NativeCalculationDialog.test.ts" in output,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "method_scoped_setup_contract",
            "typed_result_and_tamper_contract",
            "typed_recipe_and_scope_contract",
            "accessible_dialog_contract",
        )
    )
    report = write_identity_report(
        "frontend_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/nativeCtaPls.ts",
            "src/native/nativeCtaPls.test.ts",
            "src/native/nativeCtaPls.testFixture.ts",
            "src/native/nativeCtaPlsResults.test.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/NativeCalculationDialog.tsx",
            "src/native/nativeResults.ts",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def export_report() -> bool:
    command = [
        "npx.cmd",
        "vitest",
        "run",
        "src/native/nativeCtaPlsExport.test.ts",
        "src/native/nativeExportTables.test.ts",
        "src/native/NativeExportDialog.test.ts",
        "--reporter=verbose",
    ]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=3, minimum_tests=18)
    checks = {
        "passed": completed.returncode == 0 and summary["passed"],
        "vitest_summary": summary,
        "cta_csv_html_projection": "nativeCtaPlsExport.test.ts" in output,
        "xlsx_table_projection": "nativeExportTables.test.ts" in output,
        "same_run_export_scope": "NativeExportDialog.test.ts" in output,
        "provenance_required": True,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in ("cta_csv_html_projection", "xlsx_table_projection", "same_run_export_scope")
    )
    report = write_identity_report(
        "export_report",
        passed=checks["passed"],
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/nativeCtaPlsExport.test.ts",
            "src/native/nativeCtaPls.testFixture.ts",
            "src/native/nativeExportTables.ts",
            "src/native/NativeExportDialog.tsx",
            "src/domain/resultTables.ts",
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
                run_python_gate("validation/cta_pls_simulation.py"),
                run_python_gate("validation/cta_pls_boundary_gate.py"),
            ]
        )
    if stage == "archive":
        return run_python_gate("validation/cta_pls_persistence_gate.py")
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
    print(f"CTA-PLS v1 factory lightweight stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
