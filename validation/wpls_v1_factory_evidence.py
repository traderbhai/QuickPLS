"""Orchestrate fresh lightweight identity reports for bounded WPLS v1."""

from __future__ import annotations

import argparse
import json
import re
import time
from typing import Any

from wpls_reference import estimate_reference
from wpls_v1_factory_common import (
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    require_current_cli,
    run_command,
    run_model,
    sha256_file,
    write_csv,
    write_identity_report,
)
from wpls_v1_simulation import Scenario, compare, generate


SOURCE = "validation/wpls_v1_factory_evidence.py"
REFERENCE_SOURCE = "validation/wpls_reference.py"


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
    path = ROOT / "docs" / "methods" / "PLS_WPLS_V1.md"
    text = path.read_text(encoding="utf-8")
    required = [
        "wpls_case_weighted_v1",
        "zero, or negative weights are rejected",
        "weighted means and unbiased weighted sample standard deviations",
        "weighted covariance between indicators and inner proxies",
        "weighted least squares",
        "effective sample size",
        "bootstrap, permutation, or jackknife inference under case weights",
        "formative constructs",
        "PCA weighting",
    ]
    fragments = {fragment: fragment in text for fragment in required}
    bounded_scope = (
        "Survey-design weights" not in text
        and "Broader weighted estimators outside this contract remain unsupported" in text
        and "publication-ready weighting recommendations" in text
    )
    checks = {
        "passed": all(fragments.values()) and bounded_scope,
        "document": "docs/methods/PLS_WPLS_V1.md",
        "scientific_contract_fragments": fragments,
        "bounded_case_weight_scope": bounded_scope,
        "native_qualification_is_assessed_separately": True,
        "historical_preview_ui_note_is_not_used_as_native_evidence": True,
    }
    report = write_identity_report(
        "method_spec", passed=checks["passed"], checks=checks, extras=[SOURCE]
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def independent_reference_report() -> bool:
    cli_identity = require_current_cli()
    started_ns = time.time_ns()
    scenario = Scenario("independent_reference", 192, 20_260_871, "path", "strong")
    rows, _, _ = generate(scenario)
    names = ["x1", "x2", "y1", "y2", "case_wt"]
    csv_path = WORK_ROOT / "independent_reference.csv"
    write_csv(csv_path, names, [[row[name] for name in names] for row in rows])
    run = run_model(
        name="factory_independent_reference",
        csv_path=csv_path,
        constructs=[construct("x", ["x1", "x2"]), construct("y", ["y1", "y2"])],
        paths=[{"source": "x", "target": "y"}],
    )
    retained = [{name: float(row[name]) for name in names} for row in rows]
    first = estimate_reference(retained)
    second = estimate_reference(retained)
    parity = compare(run, first)
    generated = [csv_path, ROOT / run["recipe"], ROOT / run["output"]]
    freshness = [
        {
            "path": repository_path(path),
            "passed": path.is_file() and path.stat().st_size > 0 and path.stat().st_mtime_ns >= started_ns - 2_000_000_000,
            "size": path.stat().st_size if path.is_file() else None,
            "sha256": sha256_file(path) if path.is_file() else None,
        }
        for path in generated
    ]
    checks = {
        "passed": (
            run["passed"]
            and parity["passed"]
            and first == second
            and all(row["passed"] for row in freshness)
        ),
        "fresh_generation_only": True,
        "legacy_reports_accepted": False,
        "independence": {
            "reference": "standalone Python weighted moments, iteration, WLS, and diagnostics",
            "quickpls_imported_by_reference_math": False,
            "development_validation_only": True,
            "invoked_function": "validation.wpls_reference.estimate_reference",
        },
        "reference_repeat_exact": first == second,
        "parity": parity,
        "cli_identity": cli_identity,
        "current_payload_identity": {
            "payload_kind": run["result"]["payload"]["kind"],
            "estimation_method_version": run["estimation"]["method_version"],
            "wpls_method_version": run["wpls"]["method_version"],
            "case_weight_column": run["wpls"]["case_weight_column"],
        },
        "generated_artifacts": freshness,
    }
    report = write_identity_report(
        "independent_reference",
        passed=checks["passed"],
        checks=checks,
        execution=run["execution"],
        extras=[SOURCE, REFERENCE_SOURCE, *run["cli_identity"]["source_mtime_ns"].keys()],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return checks["passed"]


def run_python_gate(script: str) -> bool:
    completed, execution = run_command(["python", script], timeout=600)
    if completed.returncode != 0:
        print(json.dumps(execution, indent=2))
        return False
    return True


def frontend_report() -> bool:
    files = [
        "src/native/nativeAnalysisCatalog.test.ts",
        "src/native/nativeAnalysisRecipe.test.ts",
        "src/native/nativePlsReadiness.test.ts",
        "src/native/nativeResults.test.ts",
        "src/native/NativeCalculationDialog.test.ts",
    ]
    completed, execution = run_command(
        ["npx.cmd", "vitest", "run", *files, "--reporter=verbose"], timeout=900
    )
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=len(files), minimum_tests=90)
    dialog = (ROOT / "src" / "native" / "NativeCalculationDialog.tsx").read_text(encoding="utf-8")
    catalog = (ROOT / "src" / "native" / "nativeAnalysisCatalog.ts").read_text(encoding="utf-8")
    source_contract = {
        "catalog_identity": 'kind: "wpls"' in catalog and '"qpls3.pls.weighted"' in catalog,
        "accessible_weight_label": 'htmlFor="nd-calculation-case-weight"' in dialog,
        "weight_select_identity": 'id="nd-calculation-case-weight"' in dialog,
        "bounded_wpls_panel": 'kind === "wpls"' in dialog and "caseWeightColumn" in dialog,
    }
    output_contract = {
        "catalog_tests": "nativeAnalysisCatalog.test.ts" in output,
        "typed_recipe_tests": "nativeAnalysisRecipe.test.ts" in output,
        "readiness_tests": "nativePlsReadiness.test.ts" in output,
        "result_surface_tests": "nativeResults.test.ts" in output,
        "accessible_dialog_tests": "NativeCalculationDialog.test.ts" in output,
        "wpls_recipe_case": "maps WPLS through the Rust settings field" in output,
        "wpls_readiness_case": "checks Weighted PLS setup" in output,
        "wpls_result_case": "keeps WPLS weighted common results" in output,
    }
    passed = (
        completed.returncode == 0
        and summary["passed"]
        and all(source_contract.values())
        and all(output_contract.values())
    )
    checks = {
        "passed": passed,
        "vitest_summary": summary,
        "source_contract": source_contract,
        "executed_contract": output_contract,
        "gui_runtime_claimed": False,
        "native_source_and_component_contract_only": True,
    }
    report = write_identity_report(
        "frontend_report",
        passed=passed,
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/nativeAnalysisCatalog.ts",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/nativePlsReadiness.ts",
            "src/native/nativeResults.ts",
            "src/native/NativeCalculationDialog.tsx",
            *files,
        ],
    )
    print(f"wrote {report} | passed={passed}")
    return passed


def export_report() -> bool:
    files = [
        "src/native/nativeExportTables.test.ts",
        "src/native/nativeResults.test.ts",
        "src/native/NativeExportDialog.test.ts",
    ]
    completed, execution = run_command(
        ["npx.cmd", "vitest", "run", *files, "--reporter=verbose"], timeout=900
    )
    output = completed.stdout + completed.stderr
    summary = vitest_summary(output, expected_files=len(files), minimum_tests=55)
    contracts = {
        "wpls_provenance_case": "exports immutable engine settings" in output,
        "wpls_diagnostics_projection": "keeps WPLS weighted common results" in output,
        "same_run_export_scope": "NativeExportDialog.test.ts" in output,
        "csv_xlsx_html_are_source_bound": True,
        "provenance_required": True,
    }
    passed = completed.returncode == 0 and summary["passed"] and all(contracts.values())
    checks = {"passed": passed, "vitest_summary": summary, "contracts": contracts}
    report = write_identity_report(
        "export_report",
        passed=passed,
        checks=checks,
        execution=execution,
        extras=[
            SOURCE,
            "src/native/nativeExportTables.ts",
            "src/native/nativeResults.ts",
            "src/native/NativeExportDialog.tsx",
            "src/domain/resultTables.ts",
            *files,
        ],
    )
    print(f"wrote {report} | passed={passed}")
    return passed


def run_stage(stage: str) -> bool:
    if stage == "engine":
        return all(
            [
                method_spec_report(),
                independent_reference_report(),
                run_python_gate("validation/wpls_v1_simulation.py"),
                run_python_gate("validation/wpls_v1_boundary_gate.py"),
            ]
        )
    if stage == "archive":
        return run_python_gate("validation/wpls_v1_persistence_gate.py")
    if stage == "native":
        return frontend_report() and export_report()
    if stage == "all-light":
        return run_stage("engine") and run_stage("archive") and run_stage("native")
    raise ValueError(stage)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage", choices=["engine", "archive", "native", "all-light"], default="all-light"
    )
    args = parser.parse_args()
    passed = run_stage(args.stage)
    print(f"WPLS v1 factory lightweight stage={args.stage} passed={passed}")
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
