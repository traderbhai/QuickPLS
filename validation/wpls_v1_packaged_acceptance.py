"""Receipt-bound packaged Windows and responsive acceptance for WPLS v1."""

from __future__ import annotations

import argparse
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from diagnostic_bundle_source_manifest import SourceManifestFailure, validate_build_receipt
from method_promotion_manifest import _verify_artifact, validate_manifest
from packaged_windows_acceptance_v2 import (
    CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
    EXPECTED_CHECK_COUNT,
    receipt_binds_packaged_acceptance_contract,
    validate_required_report_checks,
)
from wpls_v1_factory_common import (
    MANIFEST_PATH, ROOT, manifest, repository_path, run_command, sha256_file,
    strict_load_json, write_identity_report,
)


SOURCE = "validation/wpls_v1_packaged_acceptance.py"
RAW_REPORT = ROOT / "validation/results/v247_tauri_native_acceptance.json"
VISUAL_REPORT = ROOT / "validation/results/v247_native_desktop_visual_acceptance.json"
CUMULATIVE_RECEIPT = ROOT / "validation/results/v247_cumulative_native_acceptance_receipt.json"
BUILD_RECEIPT = ROOT / "validation/results/diagnostic_bundle_build_receipt.json"
DESKTOP = ROOT / "target/release/quickpls-desktop.exe"
RELEASE_CLI = ROOT / "target/release/qpls.exe"
EXPECTED_CUMULATIVE_CHECKS = EXPECTED_CHECK_COUNT
REQUIRED_VIEWPORTS = {
    "1024x700": {"width": 1024, "height": 700},
    "1280x720": {"width": 1280, "height": 720},
    "1440x900": {"width": 1440, "height": 900},
}
REQUIRED_XLSX_SHEETS = {"WPLS case-weight diagnostics", "Run provenance"}
GATE_SOURCES = {
    SOURCE,
    "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
    "validation/diagnostic_bundle_source_manifest.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/wpls_v1.manifest.json",
    "validation/packaged_windows_acceptance_v2.py",
    "validation/wpls_v1_factory_audit.py",
    "validation/wpls_v1_factory_common.py",
    "validation/run_v247_cumulative_native_acceptance.ps1",
    "validation/test_wpls_v1_packaged_adapter.py",
    "validation/test_extended_pls_cumulative_receipt.py",
    "validation/test_v247_cumulative_native_acceptance_supervisor.py",
    "validation/v247_native_desktop_visual_acceptance.mjs",
    "validation/v247_tauri_native_acceptance.mjs",
}


def _parse_utc(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)


def _artifact(path: Path) -> dict[str, Any]:
    return {"path": repository_path(path), "size": path.stat().st_size, "sha256": sha256_file(path), "mtime_ns": path.stat().st_mtime_ns}


def cli_source_paths() -> list[str]:
    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(repository_path(path) for path in (ROOT / "crates/qpls-cli/src").rglob("*.rs") if path.is_file())
    for crate in ("qpls-assessment", "qpls-core", "qpls-data", "qpls-estimation", "qpls-project", "qpls-resampling", "qpls-runner"):
        crate_root = ROOT / "crates" / crate
        paths.add(repository_path(crate_root / "Cargo.toml"))
        paths.update(repository_path(path) for path in (crate_root / "src").rglob("*.rs") if path.is_file())
    development_slices = ROOT / "validation/development_slices.json"
    if development_slices.is_file():
        paths.add(repository_path(development_slices))
    return sorted(paths)


def source_freshness() -> dict[str, Any]:
    try:
        for path in (BUILD_RECEIPT, DESKTOP, RELEASE_CLI):
            if not path.is_file():
                raise FileNotFoundError(path)
        receipt = strict_load_json(BUILD_RECEIPT)
        validate_build_receipt(receipt, ROOT)
        cli = _artifact(RELEASE_CLI)
        rows = [_artifact(ROOT / relative) for relative in cli_source_paths()]
        newer = [row["path"] for row in rows if row["mtime_ns"] > cli["mtime_ns"]]
        return {
            "passed": not newer, "desktop_receipt_exact": True,
            "desktop": _artifact(DESKTOP), "release_cli": cli,
            "release_cli_newer_build_sources": newer, "release_cli_build_sources": rows,
            "gate_sources_excluded_from_binary_freshness": [_artifact(ROOT / relative) for relative in sorted(GATE_SOURCES)],
            "build_receipt": _artifact(BUILD_RECEIPT), "build_receipt_schema_version": receipt.get("schema_version"),
        }
    except (FileNotFoundError, OSError, ValueError, SourceManifestFailure) as error:
        return {"passed": False, "desktop_receipt_exact": False, "error": str(error)}


def verify_prior_factory_stages() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    identity = {"passed": True, "feature_id": feature["id"], "method_version": feature["method_version"], "catalogue_snapshot_date": feature["catalogue_snapshot_date"]}
    required = {
        "engine_only": {"method_spec", "independent_reference", "simulation_report", "boundary_report"},
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    rows: list[dict[str, Any]] = []
    for stage, expected in required.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            rows.append({"stage": stage, "path": artifact["path"], "roles": artifact["roles"], "passed": passed, "errors": errors})
        if observed != expected:
            rows.append({"stage": stage, "passed": False, "errors": [f"expected roles {sorted(expected)}, found {sorted(observed)}"]})
    return {"passed": all(row["passed"] for row in rows), "artifacts": rows}


def verify_cumulative_receipt(started: datetime) -> dict[str, Any]:
    try:
        receipt = strict_load_json(CUMULATIVE_RECEIPT)
        report = _artifact(RAW_REPORT)
        matches = [row for row in receipt.get("exports", []) if row.get("role") == "wpls"]
        export = matches[0] if len(matches) == 1 else {}
        export_path = ROOT / export.get("path", "")
        checks = {
            "receipt_identity": receipt.get("kind") == "quickpls_v247_cumulative_native_acceptance_receipt" and receipt_binds_packaged_acceptance_contract(receipt),
            "receipt_passed": receipt.get("passed") is True,
            "fresh_for_invocation": _parse_utc(receipt["supervisor_started_at_utc"]) >= started - timedelta(seconds=2) and _parse_utc(receipt["completed_at_utc"]) >= started,
            "report_path_exact": receipt.get("report") == repository_path(RAW_REPORT),
            "report_hash_and_size_exact": receipt.get("report_sha256") == report["sha256"] and receipt.get("report_size") == report["size"],
            "exact_check_contract": receipt_binds_packaged_acceptance_contract(receipt),
            "clean_report": receipt.get("failures") == 0 and receipt.get("console_errors") == 0,
            "final_scope_exact": receipt.get("final_scope") == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"],
            "graceful_cleanup_verified": receipt.get("graceful_process_cleanup_verified") is True,
            "one_wpls_export": len(matches) == 1,
            "wpls_export_exact": export_path.is_file() and export.get("size") == export_path.stat().st_size and export.get("sha256") == sha256_file(export_path),
        }
        return {"passed": all(checks.values()), "checks": checks, "receipt": receipt, "wpls_export": export}
    except (KeyError, OSError, ValueError) as error:
        return {"passed": False, "checks": {}, "error": str(error)}


def verify_native_report(started: datetime, cumulative: dict[str, Any]) -> dict[str, Any]:
    report = strict_load_json(RAW_REPORT)
    all_checks = report.get("checks", {})
    invalid = all_checks.get("wplsInvalidSetup", {})
    dialog = all_checks.get("wplsDialog", {})
    progress = all_checks.get("wplsProgress", {})
    result = all_checks.get("wplsResult", {})
    weights = all_checks.get("wpls_weights", {})
    exported = all_checks.get("wplsExport", {})
    native_xlsx = exported.get("nativeXlsx", {})
    completion = native_xlsx.get("helper", {}).get("completion", {})
    workbook = completion.get("workbook", {})
    reopened = all_checks.get("wplsSaveReopen", {})
    focused = report.get("focusedRun", {})
    run_id = result.get("runId")
    before, after = invalid.get("archiveBefore", {}), invalid.get("archiveAfter", {})
    receipt_export = cumulative.get("wpls_export", {})
    checks = {
        "fresh_after_invocation_start": _parse_utc(report["generatedAt"]) >= started,
        "raw_report_passed": report.get("passed") is True,
        "current_cumulative_chain": focused.get("scope") == "regression_bootstrap" and _parse_utc(focused["completedAt"]) >= started,
        "exact_cumulative_check_contract": validate_required_report_checks(PACKAGED_ACCEPTANCE_CONTRACT, all_checks)["passed"],
        "tauri_runtime": all_checks.get("runtime", {}).get("tauriRuntime") is True,
        "invalid_weight_blocked": invalid.get("attempted") is True and invalid.get("caseWeightColumn") == "" and invalid.get("startEnabled") is False and invalid.get("missingWeightBlocker") is True,
        "invalid_weight_created_no_state": invalid.get("runStateUnchanged") is True and invalid.get("resultCreated") is False and all(before.get(key) == 0 and after.get(key) == 0 for key in ("recipeCount", "resultCount", "runCount")),
        "valid_setup_selected": "Weighted PLS" in dialog.get("selectedMethod", "") and dialog.get("caseWeightColumn") == "case_wt" and dialog.get("startEnabled") is True and not dialog.get("blockers") and "Standardized (fixed)" in dialog.get("standardized", ""),
        "completed_run_identity": bool(run_id) and progress.get("completedRunProof", {}).get("runId") == run_id and progress.get("completedRunProof", {}).get("matched") is True and "Weighted PLS" in result.get("runLabel", ""),
        "method_tables_nonempty": result.get("pathRows", 0) > 0 and weights.get("rows", 0) > 0 and weights.get("caseWeightColumnVisible") is True,
        "export_bound_to_exact_run": exported.get("selectedRunId") == run_id and exported.get("expectedRunId") == run_id and exported.get("xlsxEnabled") is True,
        "xlsx_created_and_read_back": native_xlsx.get("attempted") is True and completion.get("passed") is True and native_xlsx.get("file", {}).get("isFile") is True and native_xlsx.get("file", {}).get("size", 0) > 0,
        "xlsx_method_identity": REQUIRED_XLSX_SHEETS <= set(native_xlsx.get("workbookSheets", [])) and native_xlsx.get("methodSheetsPresentExactlyOnce") is True and "case_wt" in workbook.get("requiredSharedStrings", []),
        "xlsx_receipt_exact": native_xlsx.get("targetPath") == str((ROOT / receipt_export.get("path", "")).resolve()) and workbook.get("sha256") == receipt_export.get("sha256") and workbook.get("size") == receipt_export.get("size"),
        "same_run_reopened": reopened.get("sameRunRestored") is True and reopened.get("expectedRunId") == run_id and reopened.get("selectedRunId") == run_id,
        "reopened_tables_exact": reopened.get("pathRows", 0) > 0 and reopened.get("diagnosticRows", 0) > 0 and reopened.get("immutableLabelsRestored") is True and reopened.get("caseWeightColumnRestored") is True,
        "clean_failures": not report.get("failures"), "clean_console": not report.get("consoleErrors"),
    }
    return {"passed": all(checks.values()), "checks": checks, "run_id": run_id, "invalid_setup": invalid, "export": exported, "save_reopen": reopened}


def verify_visual_report(started: datetime) -> dict[str, Any]:
    report = strict_load_json(VISUAL_REPORT)
    rows = report.get("checks", {}).get("wpls", [])
    by_viewport = {row.get("viewport"): row for row in rows}
    reported = {row.get("id"): {"width": row.get("width"), "height": row.get("height")} for row in report.get("viewports", [])}
    screenshots = report.get("screenshots", [])
    viewport_checks, artifacts = {}, []
    for viewport in sorted(REQUIRED_VIEWPORTS):
        row = by_viewport.get(viewport, {})
        linkage, truth, close = row.get("linkage", {}), row.get("truthAndOverflow", {}), row.get("closeFocus", {})
        matches = [item for item in screenshots if item.get("viewport") == viewport and item.get("state") == "wpls-dialog"]
        relative = matches[0].get("path") if len(matches) == 1 else None
        screenshot_ok = False
        if isinstance(relative, str) and "\\" not in relative and not Path(relative).is_absolute() and ".." not in Path(relative).parts:
            absolute = ROOT / relative
            screenshot_ok = absolute.is_file() and matches[0].get("size") == absolute.stat().st_size and matches[0].get("sha256") == sha256_file(absolute)
            if screenshot_ok: artifacts.append(relative)
        passed = (
            row.get("dialogOpened") is True and row.get("pointerSelected") is True
            and linkage.get("expectedKind") == "wpls" and linkage.get("expectedLabel") == "Weighted PLS" and linkage.get("linkage") is True
            and row.get("resultData") == "Standardized (fixed)" and row.get("caseWeightCount") == 1
            and row.get("caseWeightPlaceholder") == "Select a numeric variable"
            and row.get("missingWeightBlocker") is True and row.get("startCommandCount") == 1 and row.get("startCommandDisabled") is True
            and truth.get("noFabricatedRunState") is True and truth.get("noHorizontalOverflow") is True
            and close.get("dialogClosed") is True and close.get("focusRestored") is True and screenshot_ok
        )
        viewport_checks[viewport] = {"passed": passed, "row": row, "screenshot": {"path": relative, "passed": screenshot_ok}}
    checks = {
        "fresh_after_invocation_start": _parse_utc(report["generatedAt"]) >= started,
        "raw_report_passed": report.get("passed") is True,
        "production_bundle_not_mislabeled_as_tauri": report.get("harness", {}).get("actualTauriWindow") is False,
        "exact_required_viewports": reported == REQUIRED_VIEWPORTS,
        "one_check_per_viewport": len(rows) == 3 and set(by_viewport) == set(REQUIRED_VIEWPORTS),
        "all_method_viewports_pass": all(row["passed"] for row in viewport_checks.values()),
        "screenshot_integrity_passes": report.get("coverage", {}).get("screenshotIntegrity", {}).get("passed") is True,
        "clean_failures": not report.get("failures"), "clean_console": not report.get("consoleErrors"),
    }
    return {"passed": all(checks.values()), "checks": checks, "viewports": viewport_checks, "artifact_paths": sorted(artifacts)}


def finalize_release_promotion() -> dict[str, Any]:
    completed, execution = run_command(["python", "validation/wpls_v1_factory_audit.py"], timeout=600)
    final = validate_manifest(MANIFEST_PATH, ROOT)
    return {"passed": completed.returncode == 0 and final.get("passed") is True and final.get("derived_state") == "release_qualified", "audit_execution": execution, "manifest_validation": final}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument("--not-before-utc", help="Required with --skip-run; rejects stale cumulative and visual reports.")
    args = parser.parse_args()
    if args.skip_run and not args.not_before_utc: parser.error("--skip-run requires --not-before-utc")
    before = source_freshness()
    prior = verify_prior_factory_stages()
    if not before["passed"] or not prior["passed"]:
        print(json.dumps({"source_freshness": before, "prior_factory_stages": prior}, indent=2)); return 1
    started = _parse_utc(args.not_before_utc) if args.not_before_utc else datetime.now(timezone.utc)
    executions = []
    if not args.skip_run:
        for command, timeout in [
            (["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "validation/run_v247_cumulative_native_acceptance.ps1"], 7200),
            (["npm.cmd", "run", "qpls:v247:native-desktop-visual"], 3600),
        ]:
            completed, execution = run_command(command, timeout=timeout); executions.append(execution)
            if completed.returncode != 0: print(json.dumps(execution, indent=2)); return 1
    cumulative = verify_cumulative_receipt(started)
    native, visual, after = verify_native_report(started, cumulative), verify_visual_report(started), source_freshness()
    stable = before == after
    detail = {
        "passed": prior["passed"] and cumulative["passed"] and native["passed"] and visual["passed"] and after["passed"] and stable,
        "prior_factory_stages": prior, "cumulative_receipt": cumulative, "native": native, "responsive_viewports": visual,
        "runner_cleanup_verified": cumulative.get("checks", {}).get("graceful_cleanup_verified") is True,
        "source_freshness": {"passed": before["passed"] and after["passed"] and stable, "source_stable_during_gate": stable, "before": before, "after": after},
    }
    export_path = cumulative.get("wpls_export", {}).get("path")
    report = write_identity_report(
        "packaged_acceptance", passed=detail["passed"], checks=detail,
        execution={"commands": executions, "reused_successful_cumulative_chain": args.skip_run},
        extras=[*sorted(GATE_SOURCES), repository_path(BUILD_RECEIPT), repository_path(CUMULATIVE_RECEIPT), repository_path(DESKTOP), repository_path(RELEASE_CLI), repository_path(RAW_REPORT), repository_path(VISUAL_REPORT), *visual["artifact_paths"], *([export_path] if isinstance(export_path, str) else []), "Cargo.lock", "src-tauri/Cargo.toml", "src-tauri/src/lib.rs", "src/native/nativeAnalysisRecipe.ts", "src/native/nativeResults.ts", "src/native/nativeExportTables.ts"],
    )
    print(f"wrote {report} | passed={detail['passed']}")
    if not detail["passed"]: print(json.dumps(detail, indent=2, sort_keys=True)); return 1
    promotion = finalize_release_promotion()
    if not promotion["passed"]: print(json.dumps(promotion, indent=2, sort_keys=True)); return 1
    print("WPLS v1 derived state: release_qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
