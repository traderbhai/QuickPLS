"""PLS Algorithm v1 packaged gate.

This gate is intentionally separate from the lightweight factory lane. It
combines a genuine packaged-Tauri workflow with the production-bundle visual
harness that already covers the exact three required viewports. Each source is
verified independently and the gate remains red unless export and reopen are
bound to the same PLS Algorithm run (not a later bootstrap run).
"""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from diagnostic_bundle_source_manifest import (
    SourceManifestFailure,
    validate_build_receipt,
)
from method_promotion_manifest import _verify_artifact, validate_manifest
from packaged_windows_acceptance_v2 import (
    CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
    CONTRACT_FILE_SHA256,
    EXPECTED_CHECK_COUNT,
    receipt_binds_packaged_acceptance_contract,
    validate_required_report_checks,
)
from pls_algorithm_v1_factory_common import (
    MANIFEST_PATH,
    ROOT,
    manifest,
    repository_path,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_algorithm_v1_packaged_acceptance.py"
RAW_REPORT = ROOT / "validation" / "results" / "v247_tauri_native_acceptance.json"
VISUAL_REPORT = ROOT / "validation" / "results" / "v247_native_desktop_visual_acceptance.json"
CUMULATIVE_RECEIPT = (
    ROOT / "validation" / "results" / "v247_cumulative_native_acceptance_receipt.json"
)
BUILD_RECEIPT = ROOT / "validation" / "results" / "diagnostic_bundle_build_receipt.json"
DESKTOP = ROOT / "target" / "release" / "quickpls-desktop.exe"
RELEASE_CLI = ROOT / "target" / "release" / "qpls.exe"
EXPECTED_CUMULATIVE_CHECKS = EXPECTED_CHECK_COUNT
GATE_SOURCES = {
    SOURCE,
    "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
    "validation/diagnostic_bundle_source_manifest.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/pls_algorithm_v1.manifest.json",
    "validation/packaged_windows_acceptance_v2.py",
    "validation/pls_algorithm_v1_factory_audit.py",
    "validation/pls_algorithm_v1_factory_common.py",
    "validation/run_v247_cumulative_native_acceptance.ps1",
    "validation/test_pls_algorithm_v1_packaged_adapter.py",
    "validation/test_pls_algorithm_v1_packaged_contract.py",
    "validation/v247_native_desktop_visual_acceptance.mjs",
    "validation/v247_tauri_native_acceptance.mjs",
}
REQUIRED_VIEWPORTS = {
    "1024x700": {"width": 1024, "height": 700},
    "1280x720": {"width": 1280, "height": 720},
    "1440x900": {"width": 1440, "height": 900},
}


def cli_source_paths() -> list[str]:
    """Return the local Rust source closure that can affect release qpls.exe."""

    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates" / "qpls-cli" / "src").rglob("*.rs")
        if path.is_file()
    )
    for crate in (
        "qpls-assessment",
        "qpls-core",
        "qpls-data",
        "qpls-estimation",
        "qpls-project",
        "qpls-resampling",
        "qpls-runner",
    ):
        crate_root = ROOT / "crates" / crate
        paths.add(repository_path(crate_root / "Cargo.toml"))
        paths.update(
            repository_path(path)
            for path in (crate_root / "src").rglob("*.rs")
            if path.is_file()
        )
    development_slices = ROOT / "validation" / "development_slices.json"
    if development_slices.is_file():
        paths.add(repository_path(development_slices))
    return sorted(paths)


def _artifact_descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": path.resolve().relative_to(ROOT.resolve()).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "mtime_ns": path.stat().st_mtime_ns,
    }


def source_freshness() -> dict[str, Any]:
    """Bind the desktop exactly to its receipt and the CLI to current sources.

    Method-gate files are recorded, but do not make a frozen product binary
    stale. Product-source drift is rejected by ``validate_build_receipt``.
    """

    try:
        if not BUILD_RECEIPT.is_file() or not DESKTOP.is_file() or not RELEASE_CLI.is_file():
            raise FileNotFoundError(
                "frozen build receipt, release desktop, or release CLI is missing"
            )
        receipt = strict_load_json(BUILD_RECEIPT)
        validate_build_receipt(receipt, ROOT)
        desktop = _artifact_descriptor(DESKTOP)
        release_cli = _artifact_descriptor(RELEASE_CLI)
        cli_rows = [_artifact_descriptor(ROOT / relative) for relative in cli_source_paths()]
        cli_newer = [
            row["path"]
            for row in cli_rows
            if row["mtime_ns"] > release_cli["mtime_ns"]
        ]
        gate_rows = [
            _artifact_descriptor(ROOT / relative) for relative in sorted(GATE_SOURCES)
        ]
        return {
            "passed": not cli_newer,
            "desktop_receipt_exact": True,
            "desktop": desktop,
            "release_cli": release_cli,
            "release_cli_newer_build_sources": cli_newer,
            "release_cli_build_sources": cli_rows,
            "gate_sources_excluded_from_binary_freshness": gate_rows,
            "build_receipt": _artifact_descriptor(BUILD_RECEIPT),
            "build_receipt_schema_version": receipt.get("schema_version"),
        }
    except (FileNotFoundError, OSError, ValueError, SourceManifestFailure) as error:
        return {
            "passed": False,
            "desktop_receipt_exact": False,
            "error": str(error),
        }


def verify_prior_factory_stages() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    expected_roles = {
        "engine_only": {
            "method_spec",
            "independent_reference",
            "simulation_report",
            "boundary_report",
        },
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    checks: list[dict[str, Any]] = []
    for stage, required_roles in expected_roles.items():
        observed_roles: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed_roles.update(artifact["roles"])
            passed, errors = _verify_artifact(
                artifact,
                document,
                ROOT,
                expected_identity,
            )
            checks.append(
                {
                    "stage": stage,
                    "path": artifact["path"],
                    "roles": artifact["roles"],
                    "passed": passed,
                    "errors": errors,
                }
            )
        if observed_roles != required_roles:
            checks.append(
                {
                    "stage": stage,
                    "roles": sorted(observed_roles),
                    "passed": False,
                    "errors": [
                        f"expected roles {sorted(required_roles)}, "
                        f"found {sorted(observed_roles)}"
                    ],
                }
            )
    return {"passed": all(row["passed"] for row in checks), "artifacts": checks}


def finalize_release_promotion() -> dict[str, Any]:
    """Write the final audit, then derive the full manifest state once."""

    audit, audit_execution = run_command(
        ["python", "validation/pls_algorithm_v1_factory_audit.py"],
        timeout=600,
    )
    if audit.returncode != 0:
        return {
            "passed": False,
            "phase": "method_audit",
            "audit_execution": audit_execution,
        }
    final_manifest = validate_manifest(MANIFEST_PATH, ROOT)
    return {
        "passed": final_manifest.get("passed") is True
        and final_manifest.get("derived_state") == "release_qualified",
        "phase": "final_manifest",
        "audit_execution": audit_execution,
        "manifest_validation": final_manifest,
    }


def _parse_utc(value: str) -> datetime:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("UTC timestamp must contain an offset")
    return parsed.astimezone(timezone.utc)


def verify_reusable_cumulative_receipt(not_before: datetime) -> dict[str, Any]:
    """Verify one exact cumulative run before allowing ``--skip-run`` reuse."""

    try:
        if not CUMULATIVE_RECEIPT.is_file() or not RAW_REPORT.is_file():
            raise FileNotFoundError("cumulative receipt or cumulative report is missing")
        receipt = strict_load_json(CUMULATIVE_RECEIPT)
        report = strict_load_json(RAW_REPORT)
        supervisor_started = _parse_utc(receipt["supervisor_started_at_utc"])
        completed = _parse_utc(receipt["completed_at_utc"])
        report_descriptor = _artifact_descriptor(RAW_REPORT)
        exports = receipt.get("exports")
        if not isinstance(exports, list):
            raise ValueError("cumulative receipt exports must be a list")
        export_roles = [row.get("role") for row in exports if isinstance(row, dict)]
        generic_rows = [
            row
            for row in exports
            if isinstance(row, dict) and row.get("role") == "generic"
        ]
        if len(generic_rows) != 1:
            raise ValueError("cumulative receipt must contain exactly one generic PLS export")
        generic = generic_rows[0]
        generic_relative = generic.get("path")
        if (
            not isinstance(generic_relative, str)
            or Path(generic_relative).is_absolute()
            or "\\" in generic_relative
            or ".." in Path(generic_relative).parts
        ):
            raise ValueError("generic PLS export path is unsafe")
        generic_path = (ROOT / generic_relative).resolve()
        if ROOT.resolve() not in generic_path.parents or not generic_path.is_file():
            raise ValueError("generic PLS export is missing or escapes the repository")
        generic_descriptor = _artifact_descriptor(generic_path)
        workbook = (
            report.get("checks", {})
            .get("mediationExport", {})
            .get("nativeXlsx", {})
            .get("helper", {})
            .get("completion", {})
            .get("workbook", {})
        )
        workbook_path_value = workbook.get("path")
        if not isinstance(workbook_path_value, str):
            raise ValueError("PLS Algorithm workbook path is missing from the report")
        workbook_path = Path(workbook_path_value)
        if not workbook_path.is_absolute():
            workbook_path = ROOT / workbook_path
        workbook_path = workbook_path.resolve()
        checks = {
            "schema_and_kind": receipt_binds_packaged_acceptance_contract(receipt)
            and receipt.get("kind")
            == "quickpls_v247_cumulative_native_acceptance_receipt",
            "receipt_passed_cleanly": receipt.get("passed") is True
            and receipt.get("failures") == 0
            and receipt.get("console_errors") == 0,
            "start_within_two_second_reuse_tolerance": supervisor_started
            >= not_before - timedelta(seconds=2),
            "completed_after_reuse_boundary": completed >= not_before
            and completed >= supervisor_started,
            "exact_report_path": receipt.get("report")
            == "validation/results/v247_tauri_native_acceptance.json",
            "exact_report_bytes": receipt.get("report_sha256")
            == report_descriptor["sha256"]
            and receipt.get("report_size") == report_descriptor["size"],
            "exact_required_checks": receipt_binds_packaged_acceptance_contract(receipt)
            and validate_required_report_checks(PACKAGED_ACCEPTANCE_CONTRACT, report.get("checks"))["passed"],
            "final_scope_regression_bootstrap": receipt.get("final_scope")
            == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"]
            and report.get("focusedRun", {}).get("scope") == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"],
            "graceful_cleanup_verified": receipt.get(
                "graceful_process_cleanup_verified"
            )
            is True,
            "export_roles_unique": len(export_roles) == len(set(export_roles)),
            "generic_pls_export_exact_bytes": generic.get("path")
            == generic_descriptor["path"]
            and generic.get("size") == generic_descriptor["size"]
            and generic.get("sha256") == generic_descriptor["sha256"],
            "generic_pls_export_bound_to_report": workbook_path == generic_path
            and workbook.get("size") == generic_descriptor["size"]
            and workbook.get("sha256") == generic_descriptor["sha256"],
        }
        receipt_descriptor = _artifact_descriptor(CUMULATIVE_RECEIPT)
        return {
            "passed": all(checks.values()),
            "checks": checks,
            "not_before_utc": not_before.isoformat().replace("+00:00", "Z"),
            "supervisor_started_at_utc": receipt["supervisor_started_at_utc"],
            "completed_at_utc": receipt["completed_at_utc"],
            "receipt": receipt_descriptor,
            "report": report_descriptor,
            "generic_pls_export": generic_descriptor,
            "artifact_paths": [
                receipt_descriptor["path"],
                generic_descriptor["path"],
            ],
        }
    except (KeyError, TypeError, FileNotFoundError, OSError, ValueError) as error:
        return {"passed": False, "error": f"{type(error).__name__}: {error}"}


def verify_native_report(
    started: datetime,
    *,
    cumulative_wrapper_passed: bool,
) -> dict[str, Any]:
    report = strict_load_json(RAW_REPORT)
    generated = _parse_utc(report["generatedAt"])
    checks = report.get("checks", {})
    pls_result = checks.get("mediationPlsResult", {})
    pls_dialog = checks.get("mediationPlsDialog", {})
    invalid_setup = checks.get("plsAlgorithmInvalidSetup", {})
    save_reopen = checks.get("mediationSaveReopen", {})
    export = checks.get("mediationExport", {})
    bootstrap_result = checks.get("mediationBootstrapResult", {})
    bootstrap_export = export.get("bootstrap", {})
    runtime = checks.get("runtime", {})
    native_xlsx = export.get("nativeXlsx", {})
    workbook = native_xlsx.get("helper", {}).get("completion", {}).get("workbook", {})
    run_label = pls_result.get("runLabel", "")
    result_navigation = pls_result.get("navigation", {})
    required_titles = {
        "Direct effects",
        "Specific indirect effects",
        "Total indirect effects",
        "Total effects",
    }
    observed_titles = set(result_navigation.get("requiredTitles", []))
    row_counts = result_navigation.get("rowCounts", {})
    algorithm_run_id = pls_result.get("runId")
    export_run_id = export.get("selectedRunId")
    reopened_run_id = save_reopen.get("selectedPlsRunId")
    invalid_archive_before = invalid_setup.get("archiveBefore", {})
    invalid_archive_after = invalid_setup.get("archiveAfter", {})
    focused = report.get("focusedRun")
    current_chain = focused is None or (
        cumulative_wrapper_passed
        and isinstance(focused, dict)
        and focused.get("scope") == "regression_bootstrap"
        and _parse_utc(focused["completedAt"]) >= started
    )
    report_checks = {
        "fresh_after_invocation_start": generated >= started,
        "raw_report_passed": report.get("passed") is True,
        "current_full_or_verified_cumulative_chain": current_chain,
        "tauri_runtime": runtime.get("tauriRuntime") is True,
        "pls_algorithm_setup_selected": (
            "PLS-SEM Algorithm" in pls_dialog.get("selectedMethod", "")
            and pls_dialog.get("startEnabled") is True
            and not pls_dialog.get("blockers")
        ),
        "pls_algorithm_completed": "PLS-SEM Algorithm" in run_label,
        "point_estimate_tables_present": required_titles <= observed_titles,
        "point_estimate_tables_nonempty": all(row_counts.get(title, 0) > 0 for title in required_titles),
        "no_inference_tree_on_algorithm_run": result_navigation.get("bootstrapTreeItems") == 0,
        "xlsx_export_passed": native_xlsx.get("helper", {}).get("completion", {}).get("passed") is True,
        "xlsx_provenance_sheet_present": "Run provenance" in workbook.get("sheetNames", []),
        "save_reopen_retains_pls_algorithm": save_reopen.get("hasPlsAlgorithm") is True,
        "export_is_same_pls_algorithm_run": bool(algorithm_run_id) and export_run_id == algorithm_run_id,
        "reopen_selects_same_pls_algorithm_run": bool(algorithm_run_id) and reopened_run_id == algorithm_run_id,
        "invalid_pls_setup_is_blocked": (
            invalid_setup.get("attempted") is True
            and invalid_setup.get("startEnabled") is False
            and bool(invalid_setup.get("blockers"))
            and invalid_setup.get("resultCreated") is False
        ),
        "invalid_pls_archive_has_no_run": (
            invalid_archive_before.get("recipeCount") == 0
            and invalid_archive_before.get("resultCount") == 0
            and invalid_archive_before.get("runCount") == 0
            and invalid_archive_after.get("recipeCount") == 0
            and invalid_archive_after.get("resultCount") == 0
            and invalid_archive_after.get("runCount") == 0
            and invalid_setup.get("runStateUnchanged") is True
            and invalid_setup.get("resultCreated") is False
        ),
        "bootstrap_evidence_preserved": (
            save_reopen.get("hasBootstrap") is True
            and bool(bootstrap_result.get("runId"))
            and bootstrap_export.get("selectedRunId") == bootstrap_result.get("runId")
            and bootstrap_export.get("xlsxEnabled") is True
            and save_reopen.get("selectedRunId") == bootstrap_result.get("runId")
        ),
        "clean_failures": not report.get("failures"),
        "clean_console": not report.get("consoleErrors"),
    }
    return {
        "passed": all(report_checks.values()),
        "raw_report": "validation/results/v247_tauri_native_acceptance.json",
        "raw_report_sha256": sha256_file(RAW_REPORT),
        "generated_at_utc": report["generatedAt"],
        "focused_run": focused,
        "viewport_observed": runtime.get("viewport"),
        "checks": report_checks,
        "pls_result": pls_result,
        "invalid_setup": invalid_setup,
        "save_reopen": {
            "has_pls_algorithm": save_reopen.get("hasPlsAlgorithm"),
            "selected_pls_run_id": save_reopen.get("selectedPlsRunId"),
            "selected_bootstrap_run_id": save_reopen.get("selectedRunId"),
            "run_options": save_reopen.get("runOptions"),
        },
        "xlsx": {
            "path": workbook.get("path"),
            "sha256": workbook.get("sha256"),
            "sheet_names": workbook.get("sheetNames"),
            "selected_run_id": export_run_id,
        },
        "scope_note": (
            "The cumulative native harness blocks an invalid empty-model PLS setup without "
            "creating a recipe, run, or result; exports the exact completed PLS Algorithm run; "
            "and saves, reopens, and reselects that same run. The following Bootstrap run and "
            "its export/readback evidence remain independently verified in the same workflow."
        ),
    }


def verify_visual_report(started: datetime) -> dict[str, Any]:
    report = strict_load_json(VISUAL_REPORT)
    generated = _parse_utc(report["generatedAt"])
    reported_viewports = {
        row.get("id"): {"width": row.get("width"), "height": row.get("height")}
        for row in report.get("viewports", [])
    }
    catalogue_rows = report.get("checks", {}).get("calculationCatalog", [])
    catalogue_by_viewport = {row.get("viewport"): row for row in catalogue_rows}
    result_rows = report.get("checks", {}).get("mediation", [])
    result_by_viewport = {row.get("viewport"): row for row in result_rows}
    screenshots = report.get("screenshots", [])
    required_states = {"calculation-dialog", "completed-results", "export-dialog"}
    responsive_checks: dict[str, Any] = {}
    artifact_paths: set[str] = set()
    for viewport in sorted(REQUIRED_VIEWPORTS):
        catalogue = catalogue_by_viewport.get(viewport, {})
        linkage = catalogue.get("linkage", {})
        truth = catalogue.get("truthAndOverflow", {})
        result = result_by_viewport.get(viewport, {})
        viewport_screenshots = [
            row for row in screenshots if row.get("viewport") == viewport
        ]
        captured_states = {row.get("state") for row in viewport_screenshots}
        screenshot_integrity: dict[str, Any] = {}
        for state in sorted(required_states):
            matches = [row for row in viewport_screenshots if row.get("state") == state]
            valid = False
            reason = None
            relative = matches[0].get("path") if len(matches) == 1 else None
            if len(matches) != 1:
                reason = f"expected_one_descriptor_found_{len(matches)}"
            elif not isinstance(relative, str) or not relative.startswith(
                "validation/results/screens/v247-native-desktop-visual/"
            ):
                reason = "unsafe_or_unexpected_path"
            elif "\\" in relative or Path(relative).is_absolute() or ".." in Path(relative).parts:
                reason = "unsafe_or_unexpected_path"
            else:
                absolute = (ROOT / relative).resolve()
                if ROOT.resolve() not in absolute.parents or not absolute.is_file():
                    reason = "missing_or_escaped_artifact"
                else:
                    stat = absolute.stat()
                    valid = (
                        stat.st_size > 0
                        and matches[0].get("size") == stat.st_size
                        and matches[0].get("sha256") == sha256_file(absolute)
                    )
                    if valid:
                        artifact_paths.add(relative)
                    else:
                        reason = "size_or_sha256_mismatch"
            screenshot_integrity[state] = {
                "passed": valid,
                "path": relative,
                "reason": reason,
            }
        responsive_checks[viewport] = {
            "passed": (
                linkage.get("expectedKind") == "pls_algorithm"
                and linkage.get("selectedLabel") == "PLS-SEM Algorithm"
                and linkage.get("linkage") is True
                and truth.get("noFabricatedRunState") is True
                and truth.get("noHorizontalOverflow") is True
                and required_states <= captured_states
                and all(row["passed"] for row in screenshot_integrity.values())
                and result.get("source") == "completedSamplePlsRun"
                and all(count > 0 for count in result.get("rowCounts", {}).values())
            ),
            "linkage": linkage,
            "truth_and_overflow": truth,
            "captured_states": sorted(captured_states & required_states),
            "screenshot_integrity": screenshot_integrity,
            "result_source": result.get("source"),
            "result_run_id": result.get("runId"),
        }
    checks = {
        "fresh_after_invocation_start": generated >= started,
        "raw_report_passed": report.get("passed") is True,
        "production_bundle_not_mislabeled_as_tauri": report.get("harness", {}).get("actualTauriWindow") is False,
        "exact_required_viewports": reported_viewports == REQUIRED_VIEWPORTS,
        "one_catalogue_check_per_viewport": set(catalogue_by_viewport) == set(REQUIRED_VIEWPORTS)
        and len(catalogue_rows) == len(REQUIRED_VIEWPORTS),
        "one_result_check_per_viewport": set(result_by_viewport) == set(REQUIRED_VIEWPORTS)
        and len(result_rows) == len(REQUIRED_VIEWPORTS),
        "all_responsive_checks_pass": all(row["passed"] for row in responsive_checks.values()),
        "screenshot_integrity_passes": report.get("coverage", {}).get("screenshotIntegrity", {}).get("passed") is True,
        "clean_failures": not report.get("failures"),
        "clean_console": not report.get("consoleErrors"),
    }
    return {
        "passed": all(checks.values()),
        "raw_report": "validation/results/v247_native_desktop_visual_acceptance.json",
        "raw_report_sha256": sha256_file(VISUAL_REPORT),
        "generated_at_utc": report["generatedAt"],
        "checks": checks,
        "viewports": responsive_checks,
        "artifact_paths": sorted(artifact_paths),
        "scope_note": (
            "This is production-bundle responsive evidence only. The packaged-Tauri report "
            "separately proves Windows runtime, execution, persistence, export, and cleanup."
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument(
        "--not-before-utc",
        help="Required with --skip-run; rejects reports older than this ISO-8601 UTC time.",
    )
    args = parser.parse_args()
    if args.skip_run and not args.not_before_utc:
        parser.error("--skip-run requires --not-before-utc so stale reports cannot be promoted")
    freshness_before = source_freshness()
    if not freshness_before["passed"]:
        print(json.dumps({"phase": "source_freshness_before", **freshness_before}, indent=2))
        return 1
    prior = verify_prior_factory_stages()
    if not prior["passed"]:
        print(json.dumps({"phase": "prior_factory_stages", **prior}, indent=2))
        return 1
    started = _parse_utc(args.not_before_utc) if args.not_before_utc else datetime.now(timezone.utc)
    executions: list[dict[str, Any]] = []
    cumulative_wrapper_passed = False
    cumulative_reuse: dict[str, Any] | None = None
    if args.skip_run:
        cumulative_reuse = verify_reusable_cumulative_receipt(started)
        if not cumulative_reuse["passed"]:
            print(json.dumps({"phase": "cumulative_receipt_reuse", **cumulative_reuse}, indent=2))
            return 1
        cumulative_wrapper_passed = True
    if not args.skip_run:
        completed, execution = run_command(
            [
                "powershell",
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                "validation/run_v247_cumulative_native_acceptance.ps1",
            ],
            timeout=7200,
        )
        executions.append(execution)
        if completed.returncode != 0:
            print(json.dumps(execution, indent=2))
            return 1
        cumulative_wrapper_passed = True
        visual_completed, visual_execution = run_command(
            ["npm.cmd", "run", "qpls:v247:native-desktop-visual"],
            timeout=3600,
        )
        executions.append(visual_execution)
        if visual_completed.returncode != 0:
            print(json.dumps(visual_execution, indent=2))
            return 1
    for path in (RAW_REPORT, VISUAL_REPORT):
        if not path.is_file():
            raise FileNotFoundError(path)
    native = verify_native_report(
        started,
        cumulative_wrapper_passed=cumulative_wrapper_passed,
    )
    visual = verify_visual_report(started)
    freshness_after = source_freshness()
    source_stable_during_gate = freshness_before == freshness_after
    detail = {
        "passed": (
            native["passed"]
            and visual["passed"]
            and cumulative_wrapper_passed
            and prior["passed"]
            and freshness_after["passed"]
            and source_stable_during_gate
        ),
        "prior_factory_stages": prior,
        "source_freshness": {
            "passed": freshness_before["passed"]
            and freshness_after["passed"]
            and source_stable_during_gate,
            "source_stable_during_gate": source_stable_during_gate,
            "before": freshness_before,
            "after": freshness_after,
        },
        "tested_binaries": {
            "desktop": freshness_after.get("desktop"),
            "release_cli": freshness_after.get("release_cli"),
        },
        "cumulative_execution": (
            {"passed": True, "mode": "receipt_reuse", "receipt": cumulative_reuse}
            if cumulative_reuse is not None
            else {"passed": cumulative_wrapper_passed, "mode": "direct_run"}
        ),
        "native": native,
        "responsive_viewports": visual,
        "runner_cleanup_verified": cumulative_wrapper_passed,
    }
    report = write_identity_report(
        "packaged_acceptance",
        passed=detail["passed"],
        checks=detail,
        execution={"commands": executions},
        extras=[
            *sorted(GATE_SOURCES),
            repository_path(BUILD_RECEIPT),
            repository_path(DESKTOP),
            repository_path(RELEASE_CLI),
            *cli_source_paths(),
            *(
                cumulative_reuse.get("artifact_paths", [])
                if cumulative_reuse is not None
                else []
            ),
            "validation/results/v247_tauri_native_acceptance.json",
            "validation/results/v247_native_desktop_visual_acceptance.json",
            *visual["artifact_paths"],
            "Cargo.lock",
            "src-tauri/Cargo.toml",
            "src-tauri/src/lib.rs",
            "src/native/nativeAnalysisRecipe.ts",
            "src/native/nativeResults.ts",
            "src/native/nativeExportTables.ts",
        ],
    )
    print(f"wrote {report} | passed={detail['passed']}")
    if not detail["passed"]:
        print(json.dumps(detail, indent=2, sort_keys=True))
        return 1
    promotion = finalize_release_promotion()
    if not promotion["passed"]:
        print(json.dumps(promotion, indent=2, sort_keys=True))
        return 1
    print("PLS Algorithm v1 derived state: release_qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
