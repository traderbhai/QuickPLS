"""Receipt-bound packaged Windows and responsive acceptance for PLSc v2."""

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
from plsc_v2_factory_common import (
    MANIFEST_PATH,
    ROOT,
    manifest,
    repository_path,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/plsc_v2_packaged_acceptance.py"
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
REQUIRED_XLSX_SHEETS = {
    "PLSc correction reliability",
    "PLSc construct correlations",
    "Run provenance",
}
GATE_SOURCES = {
    SOURCE,
    "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
    "validation/diagnostic_bundle_source_manifest.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/plsc_v2.manifest.json",
    "validation/packaged_windows_acceptance_v2.py",
    "validation/plsc_v2_factory_audit.py",
    "validation/plsc_v2_factory_common.py",
    "validation/run_v247_cumulative_native_acceptance.ps1",
    "validation/test_plsc_v2_packaged_adapter.py",
    "validation/test_extended_pls_cumulative_receipt.py",
    "validation/test_v247_cumulative_native_acceptance_supervisor.py",
    "validation/v247_native_desktop_visual_acceptance.mjs",
    "validation/v247_tauri_native_acceptance.mjs",
}


def _parse_utc(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00")).astimezone(timezone.utc)


def _artifact_descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": repository_path(path),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "mtime_ns": path.stat().st_mtime_ns,
    }


def cli_source_paths() -> list[str]:
    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates/qpls-cli/src").rglob("*.rs")
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
        release_cli = _artifact_descriptor(RELEASE_CLI)
        cli_rows = [_artifact_descriptor(ROOT / relative) for relative in cli_source_paths()]
        cli_newer = [row["path"] for row in cli_rows if row["mtime_ns"] > release_cli["mtime_ns"]]
        return {
            "passed": not cli_newer,
            "desktop_receipt_exact": True,
            "desktop": _artifact_descriptor(DESKTOP),
            "release_cli": release_cli,
            "release_cli_newer_build_sources": cli_newer,
            "release_cli_build_sources": cli_rows,
            "gate_sources_excluded_from_binary_freshness": [
                _artifact_descriptor(ROOT / relative) for relative in sorted(GATE_SOURCES)
            ],
            "build_receipt": _artifact_descriptor(BUILD_RECEIPT),
            "build_receipt_schema_version": receipt.get("schema_version"),
        }
    except (FileNotFoundError, OSError, ValueError, SourceManifestFailure) as error:
        return {"passed": False, "desktop_receipt_exact": False, "error": str(error)}


def verify_prior_factory_stages() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    required = {
        "engine_only": {"method_spec", "independent_reference", "simulation_report", "boundary_report"},
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    rows: list[dict[str, Any]] = []
    for stage, expected_roles in required.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            rows.append({"stage": stage, "path": artifact["path"], "roles": artifact["roles"], "passed": passed, "errors": errors})
        if observed != expected_roles:
            rows.append({"stage": stage, "passed": False, "errors": [f"expected roles {sorted(expected_roles)}, found {sorted(observed)}"]})
    return {"passed": all(row["passed"] for row in rows), "artifacts": rows}


def verify_cumulative_receipt(started: datetime) -> dict[str, Any]:
    try:
        receipt = strict_load_json(CUMULATIVE_RECEIPT)
        report = _artifact_descriptor(RAW_REPORT)
        exports = receipt.get("exports", [])
        plsc_exports = [row for row in exports if row.get("role") == "plsc"]
        export = plsc_exports[0] if len(plsc_exports) == 1 else {}
        export_path = ROOT / export.get("path", "")
        completed = _parse_utc(receipt["completed_at_utc"])
        supervisor_started = _parse_utc(receipt["supervisor_started_at_utc"])
        checks = {
            "receipt_identity": receipt.get("kind") == "quickpls_v247_cumulative_native_acceptance_receipt" and receipt_binds_packaged_acceptance_contract(receipt),
            "receipt_passed": receipt.get("passed") is True,
            "fresh_for_invocation": supervisor_started >= started - timedelta(seconds=2) and completed >= started,
            "report_path_exact": receipt.get("report") == repository_path(RAW_REPORT),
            "report_hash_and_size_exact": receipt.get("report_sha256") == report["sha256"] and receipt.get("report_size") == report["size"],
            "exact_check_contract": receipt_binds_packaged_acceptance_contract(receipt),
            "clean_report": receipt.get("failures") == 0 and receipt.get("console_errors") == 0,
            "final_scope_exact": receipt.get("final_scope") == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"],
            "graceful_cleanup_verified": receipt.get("graceful_process_cleanup_verified") is True,
            "one_plsc_export": len(plsc_exports) == 1,
            "plsc_export_exact": export_path.is_file() and export.get("size") == export_path.stat().st_size and export.get("sha256") == sha256_file(export_path),
        }
        return {"passed": all(checks.values()), "checks": checks, "receipt": receipt, "plsc_export": export}
    except (KeyError, OSError, ValueError) as error:
        return {"passed": False, "checks": {}, "error": str(error)}


def verify_native_report(started: datetime, cumulative: dict[str, Any]) -> dict[str, Any]:
    report = strict_load_json(RAW_REPORT)
    checks = report.get("checks", {})
    invalid = checks.get("plscInvalidSetup", {})
    dialog = checks.get("plscDialog", {})
    progress = checks.get("plscProgress", {})
    result = checks.get("plscResult", {})
    exported = checks.get("plscExport", {})
    native_xlsx = exported.get("nativeXlsx", {})
    completion = native_xlsx.get("helper", {}).get("completion", {})
    workbook = completion.get("workbook", {})
    reopened = checks.get("plscSaveReopen", {})
    runtime = checks.get("runtime", {})
    focused = report.get("focusedRun", {})
    run_id = result.get("runId")
    invalid_before = invalid.get("archiveBefore", {})
    invalid_after = invalid.get("archiveAfter", {})
    export_receipt = cumulative.get("plsc_export", {})
    native_checks = {
        "fresh_after_invocation_start": _parse_utc(report["generatedAt"]) >= started,
        "raw_report_passed": report.get("passed") is True,
        "current_cumulative_chain": focused.get("scope") == "regression_bootstrap" and _parse_utc(focused["completedAt"]) >= started,
        "exact_cumulative_check_contract": validate_required_report_checks(PACKAGED_ACCEPTANCE_CONTRACT, checks)["passed"],
        "tauri_runtime": runtime.get("tauriRuntime") is True,
        "invalid_scope_blocked": invalid.get("attempted") is True and invalid.get("startEnabled") is False and invalid.get("underspecifiedReflectiveBlocker") is True and bool(invalid.get("blockers")),
        "invalid_scope_created_no_state": invalid.get("runStateUnchanged") is True and invalid.get("resultCreated") is False and all(invalid_before.get(key) == 0 and invalid_after.get(key) == 0 for key in ("recipeCount", "resultCount", "runCount")),
        "valid_setup_selected": "Consistent PLS" in dialog.get("selectedMethod", "") and dialog.get("startEnabled") is True and not dialog.get("blockers"),
        "completed_run_identity": bool(run_id) and progress.get("completedRunProof", {}).get("runId") == run_id and progress.get("completedRunProof", {}).get("matched") is True and "Consistent PLS" in result.get("runLabel", ""),
        "method_tables_nonempty": result.get("reliabilityRows", 0) > 0 and result.get("correlationRows", 0) > 0 and result.get("recordedSeedLabel") == 0,
        "export_bound_to_exact_run": exported.get("selectedRunId") == run_id and exported.get("expectedRunId") == run_id and exported.get("xlsxEnabled") is True,
        "xlsx_created_and_read_back": native_xlsx.get("attempted") is True and completion.get("passed") is True and native_xlsx.get("file", {}).get("isFile") is True and native_xlsx.get("file", {}).get("size", 0) > 0,
        "xlsx_method_identity": REQUIRED_XLSX_SHEETS <= set(native_xlsx.get("workbookSheets", [])) and native_xlsx.get("methodSheetsPresentExactlyOnce") is True and "rho_A" in workbook.get("requiredSharedStrings", []),
        "xlsx_receipt_exact": native_xlsx.get("targetPath") == str((ROOT / export_receipt.get("path", "")).resolve()) and workbook.get("sha256") == export_receipt.get("sha256") and workbook.get("size") == export_receipt.get("size"),
        "same_run_reopened": reopened.get("sameRunRestored") is True and reopened.get("expectedRunId") == run_id and reopened.get("selectedRunId") == run_id,
        "reopened_tables_exact": reopened.get("reliabilityRows", 0) > 0 and reopened.get("correlationRows", 0) > 0 and reopened.get("immutableLabelsRestored") is True,
        "clean_failures": not report.get("failures"),
        "clean_console": not report.get("consoleErrors"),
    }
    return {"passed": all(native_checks.values()), "checks": native_checks, "run_id": run_id, "invalid_setup": invalid, "export": exported, "save_reopen": reopened}


def verify_visual_report(started: datetime) -> dict[str, Any]:
    report = strict_load_json(VISUAL_REPORT)
    rows = report.get("checks", {}).get("plsc", [])
    by_viewport = {row.get("viewport"): row for row in rows}
    reported_viewports = {row.get("id"): {"width": row.get("width"), "height": row.get("height")} for row in report.get("viewports", [])}
    screenshots = report.get("screenshots", [])
    viewport_checks: dict[str, Any] = {}
    artifacts: list[str] = []
    for viewport in sorted(REQUIRED_VIEWPORTS):
        row = by_viewport.get(viewport, {})
        linkage = row.get("linkage", {})
        truth = row.get("truthAndOverflow", {})
        close = row.get("closeFocus", {})
        matches = [item for item in screenshots if item.get("viewport") == viewport and item.get("state") == "plsc-dialog"]
        screenshot_ok = False
        screenshot_path = matches[0].get("path") if len(matches) == 1 else None
        if isinstance(screenshot_path, str) and "\\" not in screenshot_path and not Path(screenshot_path).is_absolute() and ".." not in Path(screenshot_path).parts:
            absolute = ROOT / screenshot_path
            screenshot_ok = absolute.is_file() and matches[0].get("size") == absolute.stat().st_size and matches[0].get("sha256") == sha256_file(absolute)
            if screenshot_ok:
                artifacts.append(screenshot_path)
        passed = (
            row.get("dialogOpened") is True
            and row.get("pointerSelected") is True
            and linkage.get("expectedKind") == "plsc"
            and linkage.get("expectedLabel") == "Consistent PLS"
            and linkage.get("linkage") is True
            and row.get("scopeLabel") == "Supported setup"
            and row.get("scopeDetail") == "Reflective constructs with at least two indicators each; path or factor weighting; raw observations with listwise deletion"
            and row.get("pcaWeightingOptionCount") == 1
            and row.get("pcaWeightingDisabled") is True
            and row.get("startCommandCount") == 1
            and truth.get("noFabricatedRunState") is True
            and truth.get("noHorizontalOverflow") is True
            and close.get("dialogClosed") is True
            and close.get("focusRestored") is True
            and screenshot_ok
        )
        viewport_checks[viewport] = {"passed": passed, "row": row, "screenshot": {"path": screenshot_path, "passed": screenshot_ok}}
    checks = {
        "fresh_after_invocation_start": _parse_utc(report["generatedAt"]) >= started,
        "raw_report_passed": report.get("passed") is True,
        "production_bundle_not_mislabeled_as_tauri": report.get("harness", {}).get("actualTauriWindow") is False,
        "exact_required_viewports": reported_viewports == REQUIRED_VIEWPORTS,
        "one_check_per_viewport": len(rows) == 3 and set(by_viewport) == set(REQUIRED_VIEWPORTS),
        "all_method_viewports_pass": all(row["passed"] for row in viewport_checks.values()),
        "screenshot_integrity_passes": report.get("coverage", {}).get("screenshotIntegrity", {}).get("passed") is True,
        "clean_failures": not report.get("failures"),
        "clean_console": not report.get("consoleErrors"),
    }
    return {"passed": all(checks.values()), "checks": checks, "viewports": viewport_checks, "artifact_paths": sorted(artifacts)}


def finalize_release_promotion() -> dict[str, Any]:
    completed, execution = run_command(["python", "validation/plsc_v2_factory_audit.py"], timeout=600)
    final = validate_manifest(MANIFEST_PATH, ROOT)
    return {
        "passed": completed.returncode == 0 and final.get("passed") is True and final.get("derived_state") == "release_qualified",
        "audit_execution": execution,
        "manifest_validation": final,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument("--not-before-utc", help="Required with --skip-run; rejects stale cumulative and visual reports.")
    args = parser.parse_args()
    if args.skip_run and not args.not_before_utc:
        parser.error("--skip-run requires --not-before-utc")

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
    if not args.skip_run:
        for command, timeout in [
            (["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "validation/run_v247_cumulative_native_acceptance.ps1"], 7200),
            (["npm.cmd", "run", "qpls:v247:native-desktop-visual"], 3600),
        ]:
            completed, execution = run_command(command, timeout=timeout)
            executions.append(execution)
            if completed.returncode != 0:
                print(json.dumps(execution, indent=2))
                return 1

    cumulative = verify_cumulative_receipt(started)
    native = verify_native_report(started, cumulative)
    visual = verify_visual_report(started)
    freshness_after = source_freshness()
    stable = freshness_before == freshness_after
    detail = {
        "passed": prior["passed"] and cumulative["passed"] and native["passed"] and visual["passed"] and freshness_after["passed"] and stable,
        "prior_factory_stages": prior,
        "cumulative_receipt": cumulative,
        "native": native,
        "responsive_viewports": visual,
        "runner_cleanup_verified": cumulative.get("checks", {}).get("graceful_cleanup_verified") is True,
        "source_freshness": {"passed": freshness_before["passed"] and freshness_after["passed"] and stable, "source_stable_during_gate": stable, "before": freshness_before, "after": freshness_after},
    }
    export_path = cumulative.get("plsc_export", {}).get("path")
    report = write_identity_report(
        "packaged_acceptance",
        passed=detail["passed"],
        checks=detail,
        execution={"commands": executions, "reused_successful_cumulative_chain": args.skip_run},
        extras=[
            *sorted(GATE_SOURCES),
            repository_path(BUILD_RECEIPT),
            repository_path(CUMULATIVE_RECEIPT),
            repository_path(DESKTOP),
            repository_path(RELEASE_CLI),
            repository_path(RAW_REPORT),
            repository_path(VISUAL_REPORT),
            *visual["artifact_paths"],
            *([export_path] if isinstance(export_path, str) else []),
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
    print("PLSc v2 derived state: release_qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
