"""Generate source-bound method-factory evidence for PLS bootstrap v4."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

from pls_bootstrap_v4_factory_common import (
    REPORT_ROOT,
    ROOT,
    run_command,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_bootstrap_v4_factory_evidence.py"
REFERENCE_REPORTS = {
    "csem_multi_variant": "validation/results/pls_bootstrap_external_reference.json",
    "csem_corporate": "validation/results/pls_bootstrap_corporate_csem_reference.json",
    "python_plspm": "validation/results/pls_bootstrap_plspm_external_reference.json",
    "r_studentized": "validation/results/studentized_supplied_reference.json",
}


def method_spec_report() -> bool:
    required: dict[str, tuple[str, ...]] = {
        "docs/methods/RESAMPLING_ENGINE_V4.md": (
            "Indexed Resampling Engine Specification v4",
            "fixed complete-case sampling",
            "canonical identities",
            "sign alignment",
            "Type 7 quantiles",
        ),
        "docs/methods/STUDENTIZED_BOOTSTRAP_V1.md": (
            "Nested Studentized Bootstrap Specification v1",
            "theta*_b",
            "nested_studentized_v1",
            "insufficient_pivots",
            "zero_outer_standard_error",
        ),
        "docs/methods/JACKKNIFE_ENGINE_V1.md": (
            "Indexed Jackknife Engine Specification v1",
            "delete-one jackknife",
            "worker count",
            "BCa",
        ),
    }
    fragments: dict[str, dict[str, bool]] = {}
    for relative, needles in required.items():
        text = (ROOT / relative).read_text(encoding="utf-8")
        fragments[relative] = {needle: needle in text for needle in needles}
    bounded = {
        "normal_reference_is_not_null_resampling": "does not define a null-resampling test"
        in (ROOT / "docs/methods/STUDENTIZED_BOOTSTRAP_V1.md").read_text(encoding="utf-8"),
        "raw_nested_estimates_not_persisted": "must not enter `.qpls`"
        in (ROOT / "docs/methods/STUDENTIZED_BOOTSTRAP_V1.md").read_text(encoding="utf-8"),
        "publication_not_claimed_by_spec_alone": "No validation or accuracy claim follows"
        in (ROOT / "docs/methods/STUDENTIZED_BOOTSTRAP_V1.md").read_text(encoding="utf-8"),
    }
    passed = all(all(row.values()) for row in fragments.values()) and all(bounded.values())
    checks = {
        "passed": passed,
        "contract_fragments": fragments,
        "bounded_nonrelease_scope": bounded,
        "specification_alone_treated_as_non_evidence": True,
    }
    report = write_identity_report(
        "method_spec", passed=passed, checks=checks, extras=[SOURCE]
    )
    print(f"wrote {report} | passed={passed}")
    return passed


def _reference_summary(relative: str) -> dict[str, Any]:
    document = strict_load_json(ROOT / relative)
    summary: dict[str, Any] = {
        "path": relative,
        "kind": document.get("kind"),
        "passed": document.get("passed") is True,
    }
    for key in (
        "tolerance",
        "summary_tolerance",
        "max_replicate_abs_diff",
        "max_summary_abs_diff",
        "r_type7_max_abs_difference",
    ):
        if key in document:
            summary[key] = document[key]
    accepted = document.get("accepted_replicates", [])
    summary["accepted_replicate_count"] = len(accepted) if isinstance(accepted, list) else 0
    if relative.endswith("studentized_supplied_reference.json"):
        summary["passed"] = (
            summary["passed"]
            and document.get("kind") == "studentized_supplied_reference_v1"
            and float(document.get("r_type7_max_abs_difference", float("inf")))
            <= float(document.get("tolerance", 0.0))
        )
    else:
        tolerance = float(document.get("tolerance", 0.0))
        summary_tolerance = float(document.get("summary_tolerance", 0.0))
        summary["passed"] = (
            summary["passed"]
            and summary["accepted_replicate_count"] >= 8
            and float(document.get("max_replicate_abs_diff", float("inf"))) <= tolerance
            and float(document.get("max_summary_abs_diff", float("inf")))
            <= summary_tolerance
        )
    return summary


def _rust_reference_test(test: str) -> dict[str, Any]:
    completed, execution = run_command(
        [
            "cargo",
            "test",
            "-p",
            "qpls-resampling",
            test,
            "--",
            "--exact",
        ],
        timeout=1200,
    )
    output = completed.stdout + completed.stderr
    return {
        "passed": completed.returncode == 0 and "1 passed" in output and "0 failed" in output,
        "test": test,
        "execution": execution,
    }


def independent_reference_report() -> bool:
    references = {name: _reference_summary(path) for name, path in REFERENCE_REPORTS.items()}
    arithmetic = {
        test: _rust_reference_test(test)
        for test in (
            "tests::type7_percentile_interpolates_at_requested_probability",
            "tests::bca_matches_hand_calculated_midrank_fixture",
            "tests::studentized_interval_matches_reversed_pivot_quantiles",
        )
    }
    independent_groups = {
        "r_csem",
        "python_plspm",
        "r_supplied_type7_and_boot",
        "rust_hand_fixtures",
    }
    passed = (
        all(row["passed"] for row in references.values())
        and all(row["passed"] for row in arithmetic.values())
        and len(independent_groups) == 4
    )
    checks = {
        "passed": passed,
        "references": references,
        "current_source_hand_arithmetic": arithmetic,
        "independence_groups": sorted(independent_groups),
        "legacy_aggregate_claim_inherited": False,
        "interpretation": (
            "Historical matched-resample numerical observations are revalidated under their "
            "own tolerances, while current source arithmetic is executed afresh."
        ),
    }
    report = write_identity_report(
        "independent_reference",
        passed=passed,
        checks=checks,
        extras=[
            SOURCE,
            "validation/pls_bootstrap_plspm_external_reference.py",
            "validation/studentized_supplied_compare.py",
            "validation/studentized_supplied_reference.R",
            "crates/qpls-resampling/src/lib.rs",
            *REFERENCE_REPORTS.values(),
        ],
    )
    print(f"wrote {report} | passed={passed}")
    return passed


def _vitest(role: str, files: list[str]) -> bool:
    completed, execution = run_command(
        ["npm.cmd", "exec", "--", "vitest", "run", *files], timeout=900
    )
    output = completed.stdout + completed.stderr
    passed = completed.returncode == 0 and "failed" not in output.lower()
    checks = {
        "passed": passed,
        "test_files": files,
        "gui_runtime_claimed": False,
        "static_native_contract_only": True,
        "build_performed": False,
        "execution": execution,
    }
    report = write_identity_report(
        role,
        passed=passed,
        checks=checks,
        extras=[SOURCE, "package.json", "package-lock.json"],
    )
    print(f"wrote {report} | passed={passed}")
    return passed


def frontend_report() -> bool:
    return _vitest(
        "frontend_report",
        [
            "src/native/nativeAnalysisCatalog.test.ts",
            "src/native/nativeAnalysisRecipe.test.ts",
            "src/native/nativeResults.test.ts",
            "src/native/NativeCalculationDialog.test.ts",
        ],
    )


def export_report() -> bool:
    return _vitest(
        "export_report",
        ["src/native/nativeExportTables.test.ts", "src/native/nativeResults.test.ts"],
    )


def _run_python(relative: str) -> bool:
    completed, execution = run_command([sys.executable, relative], timeout=1800)
    if completed.returncode != 0:
        print(json.dumps(execution, indent=2, sort_keys=True))
    return completed.returncode == 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage",
        choices=("engine", "archive", "native", "all"),
        default="all",
    )
    args = parser.parse_args(argv)
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    passed = method_spec_report()
    passed = independent_reference_report() and passed
    passed = _run_python("validation/pls_bootstrap_v4_simulation.py") and passed
    passed = _run_python("validation/pls_bootstrap_release_boundary_gate.py") and passed
    if args.stage in {"archive", "native", "all"}:
        passed = _run_python("validation/pls_bootstrap_release_persistence_gate.py") and passed
    if args.stage in {"native", "all"}:
        passed = frontend_report() and passed
        passed = export_report() and passed
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
