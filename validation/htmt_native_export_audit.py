#!/usr/bin/env python3
"""Focused native-result and export source audit for HTMT/HTMT+."""

from __future__ import annotations

import argparse
import json
import shutil
from typing import Any

from htmt_plus_v1_factory_common import ROOT, run_command, write_identity_report


SOURCE = "validation/htmt_native_export_audit.py"
NATIVE_TEST = "src/native/nativeHtmt.test.ts"
NATIVE_RESULTS = "src/native/nativeResults.ts"
NATIVE_EXPORT = "src/native/nativeExportTables.ts"
CLI_SOURCE = "crates/qpls-cli/src/main.rs"


def source_contract() -> dict[str, Any]:
    results = (ROOT / NATIVE_RESULTS).read_text(encoding="utf-8")
    export = (ROOT / NATIVE_EXPORT).read_text(encoding="utf-8")
    cli = (ROOT / CLI_SOURCE).read_text(encoding="utf-8")
    test = (ROOT / NATIVE_TEST).read_text(encoding="utf-8")
    checks = {
        "dedicated_native_test_present": "HTMT native qualification contract" in test,
        "point_tables_present": '"htmt_plus"' in results
        and '"htmt_original"' in results,
        "bootstrap_tables_present": (
            '"htmt_plus_bootstrap"' in results
            and '"htmt_original_bootstrap"' in results
        ),
        "bc_not_bca_label_present": "Bias-corrected percentile (Type 7); not BCa"
        in export,
        "one_tailed_label_present": "One-tailed upper, alpha .05" in export,
        "explicit_decision_present": "upper bound strictly below 0.90" in export,
        "digest_column_present": "Usable-index digest" in results,
        "duplicate_pair_index_rejected": "pairUnavailable.has" in results,
        "cli_point_export_present": "push_htmt_point_rows" in cli,
        "cli_inference_export_present": "push_htmt_inference_rows" in cli,
        "cli_decision_validation_present": (
            "upper_bound_below_critical_value" in cli
            and "HTMT_BOOTSTRAP_CRITICAL_VALUE" in cli
        ),
        "cli_digest_validation_present": (
            "usable_replicate_indices_sha256" in cli
            and "HTMT_BOOTSTRAP_REPLICATE_INDEX_DIGEST_METHOD" in cli
        ),
        "labs_warning_present": "Experimental HTMT inference" in results,
    }
    return {"passed": all(checks.values()), "checks": checks}


def run_frontend_checks() -> dict[str, Any]:
    npx = shutil.which("npx") or shutil.which("npx.cmd")
    if npx is None:
        return {
            "passed": False,
            "npx_available": False,
            "vitest": {"passed": False, "execution": None},
            "typescript": {"passed": False, "execution": None},
        }
    vitest, vitest_execution = run_command(
        [
            npx,
            "vitest",
            "run",
            NATIVE_TEST,
            "--reporter=dot",
        ],
        timeout=180,
    )
    typescript, typescript_execution = run_command(
        [npx, "tsc", "-b", "--pretty", "false"], timeout=180
    )
    return {
        "passed": vitest.returncode == 0 and typescript.returncode == 0,
        "npx_available": True,
        "vitest": {
            "passed": vitest.returncode == 0,
            "execution": vitest_execution,
        },
        "typescript": {
            "passed": typescript.returncode == 0,
            "execution": typescript_execution,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-execution", action="store_true")
    args = parser.parse_args()
    source = source_contract()
    frontend = (
        {"passed": True, "skipped": True}
        if args.skip_execution
        else run_frontend_checks()
    )
    passed = source["passed"] and frontend["passed"]
    common_extras = [
        SOURCE,
        NATIVE_TEST,
        NATIVE_RESULTS,
        NATIVE_EXPORT,
        CLI_SOURCE,
        "src/types.ts",
    ]
    frontend_blockers = [
        "real_packaged_windows_gui_execution_not_supplied",
        "keyboard_pointer_accessibility_and_scaling_matrix_not_executed",
        "installed_and_portable_offline_runs_not_executed",
    ]
    export_blockers = [
        "same_run_gui_cli_export_readback_not_executed",
        "csv_xlsx_html_semantic_readback_not_executed_by_this_source_audit",
        "svg_pdf_png_export_and_semantic_readback_not_implemented",
    ]
    frontend_report = write_identity_report(
        "frontend_report",
        stage="packaged_windows",
        passed=passed,
        checks={"source_contract": source, "focused_execution": frontend},
        blockers=frontend_blockers,
        extras=common_extras,
    )
    export_report = write_identity_report(
        "export_report",
        stage="persistence_export",
        passed=passed,
        checks={"source_contract": source, "focused_execution": frontend},
        blockers=export_blockers,
        extras=common_extras,
    )
    print(
        json.dumps(
            {
                "passed": passed,
                "qualification_evidence": False,
                "frontend_blockers": frontend_blockers,
                "export_blockers": export_blockers,
                "reports": [
                    frontend_report.relative_to(ROOT).as_posix(),
                    export_report.relative_to(ROOT).as_posix(),
                ],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
