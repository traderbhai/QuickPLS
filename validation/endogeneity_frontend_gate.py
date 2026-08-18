"""Native/product checks and lightweight factory orchestration for endogeneity."""

from __future__ import annotations

import argparse
import re
from pathlib import Path
from typing import Any

from endogeneity_factory_common import (
    ROOT,
    optionally_write_identity_report,
    run_command,
    strict_load_json,
)


SOURCE = "validation/endogeneity_frontend_gate.py"


def vitest_summary(output: str, *, expected_files: int, minimum_tests: int) -> dict[str, Any]:
    files_match = re.search(r"Test Files\s+(\d+) passed \((\d+)\)", output)
    tests_match = re.search(r"Tests\s+(\d+) passed \((\d+)\)", output)
    files_passed = int(files_match.group(1)) if files_match else None
    files_total = int(files_match.group(2)) if files_match else None
    tests_passed = int(tests_match.group(1)) if tests_match else None
    tests_total = int(tests_match.group(2)) if tests_match else None
    return {
        "passed": files_passed == expected_files
        and files_total == expected_files
        and tests_passed is not None
        and tests_passed == tests_total
        and tests_passed >= minimum_tests,
        "expected_test_files": expected_files,
        "minimum_tests": minimum_tests,
        "observed_test_files_passed": files_passed,
        "observed_test_files_total": files_total,
        "observed_tests_passed": tests_passed,
        "observed_tests_total": tests_total,
    }


def method_spec_report(write_identity: bool) -> bool:
    path = ROOT / "docs" / "methods" / "PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md"
    text = path.read_text(encoding="utf-8")
    required_fragments = [
        "`gaussian_copula_endogeneity_v1`",
        "rankit inverse-normal",
        "absolute sample skewness",
        "diagnostic, not proof of causality",
        "PCA weighting",
        "bootstrap",
        "same-run",
        "packaged desktop acceptance",
    ]
    fragments = {fragment: fragment.lower() in text.lower() for fragment in required_fragments}
    checks = {
        "passed": all(fragments.values()),
        "document": "docs/methods/PLS_GAUSSIAN_COPULA_ENDOGENEITY_V1.md",
        "required_contract_fragments": fragments,
        "does_not_claim_release_qualification": all(
            marker not in text.lower()
            for marker in (
                "status: release-qualified",
                "publication status: release-qualified",
                "is release-qualified",
            )
        ),
    }
    checks["passed"] = checks["passed"] and checks["does_not_claim_release_qualification"]
    report = optionally_write_identity_report(
        "method_spec",
        write_identity=write_identity,
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE],
    )
    print(f"endogeneity method spec passed={checks['passed']} identity={report or 'not-written'}")
    return checks["passed"]


def independent_reference_report(write_identity: bool) -> bool:
    completed, execution = run_command(
        ["python", "validation/endogeneity_reference.py"], timeout=900
    )
    path = ROOT / "validation" / "results" / "endogeneity_reference_report.json"
    document = strict_load_json(path) if path.is_file() else {}
    checks = {
        "passed": completed.returncode == 0
        and document.get("passed") is True
        and document.get("kind") == "gaussian_copula_endogeneity_reference_v1"
        and float(document.get("tolerance", 1.0)) <= 1e-6,
        "independent_reference_report": "validation/results/endogeneity_reference_report.json",
        "reference_passed": document.get("passed") is True,
        "tolerance": document.get("tolerance"),
        "execution": execution,
    }
    report = optionally_write_identity_report(
        "independent_reference",
        write_identity=write_identity,
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/endogeneity_reference.py",
            "validation/higher_order_reference.py",
            "validation/results/endogeneity_reference_report.json",
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
        execution=execution,
    )
    print(f"endogeneity independent reference passed={checks['passed']} identity={report or 'not-written'}")
    return checks["passed"]


def frontend_report(write_identity: bool) -> bool:
    test_files = [
        "src/native/nativeEndogeneity.test.ts",
        "src/domain/endogeneityApplicability.test.ts",
    ]
    command = ["npx.cmd", "vitest", "run", *test_files, "--reporter=verbose"]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=2, minimum_tests=5)
    checks = {
        "passed": completed.returncode == 0 and summary["passed"],
        "vitest_summary": summary,
        "typed_recipe_and_blockers_exercised": "nativeEndogeneity.test.ts" in output,
        "applicability_exercised": "endogeneityApplicability.test.ts" in output,
        "accessible_native_table_exercised": "nativeEndogeneity.test.ts" in output,
        "method_scoped_files": test_files,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "typed_recipe_and_blockers_exercised",
            "applicability_exercised",
            "accessible_native_table_exercised",
        )
    )
    report = optionally_write_identity_report(
        "frontend_report",
        write_identity=write_identity,
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "src/native/nativeEndogeneity.test.ts",
            "src/domain/endogeneityApplicability.test.ts",
            "src/domain/methodApplicability.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/nativeResults.ts",
        ],
        execution=execution,
    )
    print(f"endogeneity frontend passed={checks['passed']} identity={report or 'not-written'}")
    return checks["passed"]


def export_report(write_identity: bool) -> bool:
    test_files = [
        "src/native/nativeEndogeneity.test.ts",
        "src/native/nativeExportTables.test.ts",
        "src/native/NativeExportDialog.test.ts",
    ]
    command = ["npx.cmd", "vitest", "run", *test_files, "--reporter=verbose"]
    completed, execution = run_command(command, timeout=900)
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=3, minimum_tests=12)
    dialog_source = (ROOT / "src" / "native" / "NativeExportDialog.tsx").read_text(
        encoding="utf-8"
    )
    checks = {
        "passed": completed.returncode == 0 and summary["passed"],
        "vitest_summary": summary,
        "endogeneity_csv_and_html_exercised": "nativeEndogeneity.test.ts" in output,
        "xlsx_table_bridge_exercised": "nativeExportTables.test.ts" in output,
        "same_run_export_dialog_exercised": "NativeExportDialog.test.ts" in output,
        "same_tables_sent_to_xlsx_bridge": "exportNativeXlsxTables(tablesWithProvenance" in dialog_source,
        "required_formats": ["csv", "xlsx", "html"],
        "method_scoped_files": test_files,
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "endogeneity_csv_and_html_exercised",
            "xlsx_table_bridge_exercised",
            "same_run_export_dialog_exercised",
            "same_tables_sent_to_xlsx_bridge",
        )
    )
    report = optionally_write_identity_report(
        "export_report",
        write_identity=write_identity,
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "src/native/nativeEndogeneity.test.ts",
            "src/native/nativeExportTables.test.ts",
            "src/native/NativeExportDialog.test.ts",
            "src/domain/resultTables.ts",
            "src/native/nativeExportTables.ts",
            "src/native/NativeExportDialog.tsx",
        ],
        execution=execution,
    )
    print(f"endogeneity export passed={checks['passed']} identity={report or 'not-written'}")
    return checks["passed"]


def run_python_gate(script: str, write_identity: bool) -> bool:
    command = ["python", script]
    if write_identity:
        command.append("--write-identity")
    completed, execution = run_command(command, timeout=1800)
    if completed.returncode != 0:
        print(execution)
        return False
    return True


def run_stage(stage: str, write_identities: bool) -> bool:
    if stage == "engine":
        return all(
            [
                method_spec_report(write_identities),
                independent_reference_report(write_identities),
                run_python_gate("validation/endogeneity_simulation.py", write_identities),
                run_python_gate("validation/endogeneity_boundary_gate.py", write_identities),
            ]
        )
    if stage == "archive":
        return run_python_gate(
            "validation/endogeneity_persistence_gate.py", write_identities
        )
    if stage == "native":
        return frontend_report(write_identities) and export_report(write_identities)
    if stage == "all-light":
        return (
            run_stage("engine", write_identities)
            and run_stage("archive", write_identities)
            and run_stage("native", write_identities)
        )
    raise ValueError(stage)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage", choices=["engine", "archive", "native", "all-light"], default="all-light"
    )
    parser.add_argument(
        "--write-identities",
        action="store_true",
        help="Write promotion identities. Omit for a source-only check.",
    )
    args = parser.parse_args()
    passed = run_stage(args.stage, args.write_identities)
    print(
        f"endogeneity factory stage={args.stage} passed={passed} "
        f"identities={'written' if args.write_identities else 'not-written'}"
    )
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
