#!/usr/bin/env python3
"""Validator for the future packaged-Windows HTMT acceptance matrix.

This source audit never launches or signs a package.  It validates the frozen
matrix now and can later verify a captured installed/portable execution report.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from htmt_plus_v1_factory_common import (
    QUALIFICATION_SPEC_PATH,
    ROOT,
    require_exact_case_ids,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/htmt_packaged_acceptance.py"


def required_case_ids() -> list[str]:
    spec = strict_load_json(QUALIFICATION_SPEC_PATH)
    windows = spec["operational_contract"]["windows"]
    matrix = [
        f"{package}_{viewport}_{scale}_success"
        for package in windows["package_kinds"]
        for viewport in windows["viewports"]
        for scale in windows["display_scale_percent"]
    ]
    return [
        *matrix,
        "invalid_setup_blocked",
        "same_run_save_close_reopen",
        "same_run_export_readback",
        "offline_no_network_dependency",
        "keyboard_only_workflow",
        "real_pointer_diagram_workflow",
        "accessible_tables_and_dialogs",
        "cancel_within_one_second",
        "cancel_no_partial_visible_result",
        "cancel_no_partial_committed_result",
        "cancel_archive_unchanged",
        "retry_same_settings",
        "clean_process_exit",
        "no_orphan_process_or_listener",
    ]


def source_contract() -> dict[str, Any]:
    spec = strict_load_json(QUALIFICATION_SPEC_PATH)
    windows = spec["operational_contract"]["windows"]
    cancellation = spec["operational_contract"]["cancellation"]
    checks = {
        "installed_and_portable": set(windows["package_kinds"])
        == {"installed", "portable"},
        "three_required_viewports": set(windows["viewports"])
        >= {"1024x700", "1280x720", "1440x900"},
        "four_required_scale_factors": set(windows["display_scale_percent"])
        >= {100, 125, 150, 200},
        "offline_required": windows["offline_required"] is True,
        "keyboard_required": windows["keyboard_only_required"] is True,
        "accessible_tables_required": windows["accessible_tables_required"] is True,
        "real_pointer_required": windows["real_pointer_required"] is True,
        "one_second_cancellation": cancellation["maximum_latency_seconds"] <= 1.0,
        "no_partial_results": (
            cancellation["no_partial_visible_result"] is True
            and cancellation["no_partial_committed_result"] is True
            and cancellation["archive_unchanged"] is True
        ),
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "required_case_count": len(required_case_ids()),
        "required_case_ids": required_case_ids(),
    }


def validate_report(path: Path) -> dict[str, Any]:
    document = strict_load_json(path)
    cases = require_exact_case_ids(document, required_case_ids())
    identity = (
        document.get("qualification_id") == "qpls3.assessment.htmt.qualification_v2"
        and document.get("method_version") == "ringle_et_al_htmt_plus_v1"
        and document.get("actual_tauri_windows") is True
        and document.get("network_requests") == 0
        and document.get("orphan_processes") == 0
        and document.get("orphan_listeners") == 0
        and document.get("partial_results") == 0
    )
    return {
        "passed": identity and cases["passed"],
        "identity": identity,
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--qualification-report", type=Path)
    parser.add_argument("--admit", action="store_true")
    args = parser.parse_args()
    source = source_contract()
    checks: dict[str, Any] = {"source_contract": source}
    blockers = [
        "installed_package_matrix_not_executed",
        "portable_package_matrix_not_executed",
        "real_windows_accessibility_and_pointer_runs_not_executed",
        "offline_cancellation_cleanup_and_reopen_runs_not_executed",
    ]
    qualification_evidence = False
    if args.qualification_report:
        qualification = validate_report(args.qualification_report)
        checks["qualification_execution"] = qualification
        if qualification["passed"]:
            blockers = []
            qualification_evidence = args.admit
        else:
            blockers.append("qualification_report_failed_contract")
    passed = source["passed"] and (
        not args.qualification_report or checks["qualification_execution"]["passed"]
    )
    report = write_identity_report(
        "packaged_acceptance",
        stage="packaged_windows",
        passed=passed,
        checks=checks,
        blockers=blockers,
        extras=[SOURCE],
        qualification_evidence=qualification_evidence,
    )
    print(
        json.dumps(
            {
                "passed": passed,
                "qualification_evidence": qualification_evidence,
                "blockers": blockers,
                "report": report.relative_to(ROOT).as_posix(),
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if passed and (not args.admit or qualification_evidence) else 1


if __name__ == "__main__":
    raise SystemExit(main())
