"""Independently adapt frozen v247 CB-SEM desktop evidence into the method factory.

The adapter is intentionally reuse-only.  It never upgrades stale evidence: the
coordinated desktop build receipt must match every current product source byte,
and the cumulative supervisor receipt must bind the exact acceptance report,
archive, workbook, cleanup result, responsive checks, and offline request log.
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from diagnostic_bundle_source_manifest import (
    SourceManifestFailure,
    discover_product_source,
    validate_build_receipt,
)
from method_promotion_manifest import _verify_artifact, validate_manifest
from cbsem_ml_v1_factory_common import (
    EXPECTED_PROVENANCE_VERSION,
    REPORT_ROOT,
    ROOT,
    engine_source_paths,
    manifest,
    repository_path,
    sha256_file,
    strict_load_json,
    write_identity_report,
)
from phase2_release_packaged_common import (
    EXPECTED_CHECK_COUNT,
    METHODS,
    PACKAGED_ACCEPTANCE_CONTRACT,
    PACKAGED_ACCEPTANCE_CONTRACT_PATH,
    functional_check_passed,
    packaged_acceptance_contract_descriptor,
    packaged_viewport_contract,
    read_archive as read_strict_archive,
    validate_required_report_checks,
)


SOURCE = "validation/cbsem_ml_v1_packaged_adapter.py"
BUILD_RECEIPT = ROOT / "validation" / "results" / "diagnostic_bundle_build_receipt.json"
CUMULATIVE_RECEIPT = ROOT / "validation" / "results" / "v247_cumulative_native_acceptance_receipt.json"
DESKTOP = ROOT / "target" / "release" / "quickpls-desktop.exe"
RELEASE_CLI = ROOT / "target" / "release" / "qpls.exe"
FACTORY_RAW_REPORT = REPORT_ROOT / "cbsem_ml_v1_packaged_raw.json"
FACTORY_ARCHIVE = REPORT_ROOT / "cbsem_ml_v1_packaged.qpls"
FACTORY_XLSX = REPORT_ROOT / "cbsem_ml_v1_packaged.xlsx"
PREFLIGHT_REPORT = REPORT_ROOT / "packaged_preflight.json"
CBSEM_CAPTURE_STATES = (
    ("130", "fixture-data"),
    ("130a", "invalid-setup"),
    ("131", "model"),
    ("132", "dialog"),
    ("134", "results"),
    ("135", "export"),
    ("136", "reopened"),
)


def _utc(value: Any) -> datetime:
    if not isinstance(value, str):
        raise ValueError("required timestamp is missing")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("required timestamp is not timezone-aware")
    return parsed.astimezone(timezone.utc)


def _receipt_descriptor_matches(path: Path, descriptor: dict[str, Any]) -> bool:
    return (
        path.is_file()
        and descriptor.get("size") == path.stat().st_size
        and descriptor.get("sha256") == sha256_file(path)
    )


def source_freshness() -> dict[str, Any]:
    try:
        if not BUILD_RECEIPT.is_file() or not DESKTOP.is_file() or not RELEASE_CLI.is_file():
            raise FileNotFoundError(
                "frozen build receipt, release desktop, or release CLI is missing"
            )
        receipt = strict_load_json(BUILD_RECEIPT)
        validate_build_receipt(receipt, ROOT)
        tested_desktop = receipt.get("tested_desktop", {})
        if not _receipt_descriptor_matches(DESKTOP, tested_desktop):
            raise ValueError("release desktop differs from the frozen build receipt")
        cli_sources = [ROOT / relative for relative in engine_source_paths()]
        newer = [repository_path(path) for path in cli_sources if path.stat().st_mtime_ns > RELEASE_CLI.stat().st_mtime_ns]
        if newer:
            raise ValueError("release CLI predates current Rust inputs: " + ", ".join(newer))
        return {
            "passed": True,
            "build_receipt_current": True,
            "desktop_receipt_exact": True,
            "release_cli_newer_sources": newer,
            "build_receipt": repository_path(BUILD_RECEIPT),
            "desktop_sha256": sha256_file(DESKTOP),
            "release_cli_sha256": sha256_file(RELEASE_CLI),
            "release_cli_source_closure": [
                {
                    "path": repository_path(path),
                    "size": path.stat().st_size,
                    "sha256": sha256_file(path),
                }
                for path in cli_sources
            ],
        }
    except (FileNotFoundError, OSError, ValueError, SourceManifestFailure) as error:
        return {
            "passed": False,
            "build_receipt_current": False,
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
    rows: list[dict[str, Any]] = []
    for stage, roles in expected_roles.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(
                artifact, document, ROOT, expected_identity
            )
            rows.append(
                {
                    "stage": stage,
                    "path": artifact["path"],
                    "roles": artifact["roles"],
                    "passed": passed,
                    "errors": errors,
                }
            )
        if observed != roles:
            rows.append(
                {
                    "stage": stage,
                    "passed": False,
                    "errors": [f"expected roles {sorted(roles)}, found {sorted(observed)}"],
                }
            )
    return {"passed": all(row["passed"] for row in rows), "artifacts": rows}


def read_archive(path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    return read_strict_archive(path)


def archive_contract(project: dict[str, Any], report: dict[str, Any]) -> dict[str, Any]:
    checks = report.get("checks", {})
    result_view = checks.get("cbsemResult", {})
    reopen = checks.get("cbsemSaveReopen", {})
    run_id = result_view.get("runId")
    results = [row for row in project.get("results", []) if row.get("id") == run_id]
    if len(results) != 1:
        return {"passed": False, "error": "reported CB-SEM run is not unique in archive"}
    result = results[0]
    estimation = result.get("payload", {}).get("estimation", {})
    cbsem = estimation.get("cbsem", {})
    fit = cbsem.get("fit", {})
    recipe_id = result.get("provenance", {}).get("recipe_id")
    recipes = [row for row in project.get("recipes", []) if row.get("id") == recipe_id]
    recipe = recipes[0] if len(recipes) == 1 else {}
    config = recipe.get("method_config", {})
    contract = {
        "same_run_reopened": reopen.get("sameRunRestored") is True
        and reopen.get("selectedRunId") == run_id,
        "result_completed": result.get("status") == "completed",
        "payload_kind": result.get("payload", {}).get("kind") == "pls_pm_v1",
        "provenance_method": result.get("provenance", {}).get("method") == "cbsem",
        "provenance_version": result.get("provenance", {}).get("method_version")
        == EXPECTED_PROVENANCE_VERSION,
        "estimation_version": estimation.get("method_version") == "cbsem_ml_v1",
        "cbsem_identity": cbsem.get("method_version") == "cbsem_ml_v1"
        and cbsem.get("model_type") == "sem"
        and cbsem.get("estimator") == "ml"
        and cbsem.get("input") == "raw"
        and cbsem.get("mean_structure") is False,
        "converged_finite_payload": cbsem.get("converged") is True
        and isinstance(cbsem.get("objective"), (int, float))
        and isinstance(cbsem.get("gradient_norm"), (int, float)),
        "sample_and_parameter_counts": cbsem.get("sample_size") == 240
        and len(cbsem.get("parameters", [])) == 23
        and len(cbsem.get("standardized", [])) == 23,
        "matrix_counts": all(
            len(cbsem.get(field, [])) == 81
            for field in (
                "implied_covariance",
                "residual_covariance",
                "residual_correlation",
            )
        ),
        "fit_and_diagnostics": fit.get("method_version") == "cbsem_fit_v1"
        and len(cbsem.get("modification_indices", [])) == 50
        and all(
            row.get("method_version") == "cbsem_modification_indices_v1"
            for row in cbsem.get("modification_indices", [])
        ),
        "unsupported_payloads_absent": cbsem.get("bootstrap") is None
        and cbsem.get("multigroup") is None,
        "typed_recipe": config == {
            "kind": "cbsem",
            "model_type": "sem",
            "estimator": "ml",
            "input": "raw",
            "mean_structure": False,
            "bootstrap_samples": 0,
        },
    }
    return {"passed": all(contract.values()), "checks": contract, "run_id": run_id}


def workbook_contract(path: Path, expected: dict[str, Any]) -> dict[str, Any]:
    required_sheets = {
        "Model fit",
        "Standardized parameters",
        "Unstandardized parameters",
        "Residual correlations",
        "Residual covariances",
        "Model-implied covariances",
        "Residual-based modification dia",
        "Calculation scope",
        "Run provenance",
    }
    from xml.etree import ElementTree

    main_ns = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
    with zipfile.ZipFile(path) as workbook:
        workbook_xml = workbook.read("xl/workbook.xml")
        shared = workbook.read("xl/sharedStrings.xml").decode("utf-8")
    workbook_root = ElementTree.fromstring(workbook_xml)
    sheet_names = {
        sheet.attrib["name"]
        for sheet in workbook_root.findall(f".//{{{main_ns}}}sheet")
    }
    required_strings = {
        "Maximum likelihood",
        "cbsem_ml_v1",
        "cbsem_fit_v1",
        "Run provenance",
    }
    checks = {
        "receipt_size": expected.get("size") == path.stat().st_size,
        "receipt_sha256": expected.get("sha256") == sha256_file(path),
        "exact_sheet_set": sheet_names == required_sheets,
        "required_provenance_strings": all(value in shared for value in required_strings),
    }
    return {"passed": all(checks.values()), "checks": checks, "sheets": sorted(sheet_names)}


def visual_contract(report: dict[str, Any], *, run_id: str) -> dict[str, Any]:
    """Compatibility name; only actual packaged-window evidence is accepted."""

    value, _ = packaged_viewport_contract(METHODS["cbsem_ml_v1"], report, run_id)
    return value


def functional_screenshot_paths(report: dict[str, Any]) -> list[Path]:
    rows = report.get("screenshots", [])
    if not isinstance(rows, list) or not all(isinstance(row, str) for row in rows):
        raise ValueError("functional CB-SEM screenshot list is malformed")
    root = ROOT / "validation/results/screens/v247-native-desktop-acceptance"
    paths = [
        root / f"{sequence}-tauri-native-cbsem-{state}-1536x794.png"
        for sequence, state in CBSEM_CAPTURE_STATES
    ]
    if not all(rows.count(str(path.resolve())) == 1 for path in paths):
        raise ValueError("functional CB-SEM screenshot set is incomplete or duplicated")
    if not all(path.is_file() and path.stat().st_size > 0 for path in paths):
        raise FileNotFoundError("functional CB-SEM screenshot file is missing or empty")
    artifacts = report.get("screenshotArtifacts", [])
    if not isinstance(artifacts, list) or not all(
        artifacts.count({"path": repository_path(path), "size": path.stat().st_size, "sha256": sha256_file(path)}) == 1
        for path in paths
    ):
        raise ValueError("functional CB-SEM screenshot bytes are not bound by the report")
    return paths


def visual_screenshot_paths(report: dict[str, Any], *, run_id: str) -> list[Path]:
    _, paths = packaged_viewport_contract(METHODS["cbsem_ml_v1"], report, run_id)
    return paths


def packaged_workflow_contract(
    report: dict[str, Any], receipt: dict[str, Any]
) -> dict[str, Any]:
    checks = report.get("checks", {})
    result = checks.get("cbsemResult", {})
    export = checks.get("cbsemExport", {})
    reopen = checks.get("cbsemSaveReopen", {})
    invalid = checks.get("cbsemInvalidSetup", {})
    offline = checks.get("bootstrapFunctionalOffline", {})
    visual_check = visual_contract(report, run_id=result.get("runId"))
    functional_screenshots = functional_screenshot_paths(report)
    steps = {
        "setup": checks.get("cbsemFixture", {}).get("cases") == 240
        and checks.get("cbsemModel", {}).get("constructs") == 3
        and checks.get("cbsemModel", {}).get("structuralPaths") == 2,
        "invalid_setup_blocked": invalid.get("attempted") is True
        and invalid.get("startEnabled") is False
        and invalid.get("resultCreated") is False
        and invalid.get("archiveStateUnchanged") is True,
        "execute": checks.get("cbsemProgress", {}).get("captured") is True,
        "inspect_results": result.get("fit", {}).get("rows") == 13
        and result.get("standardized", {}).get("rows") == 23
        and result.get("noPlaceholder") is True
        and result.get("noGenericPlsTables") is True,
        "export": export.get("nativeXlsx", {}).get("attempted") is True
        and export.get("nativeXlsx", {}).get("file", {}).get("isFile") is True,
        "save": reopen.get("archive", {}).get("resultStatus") == "completed",
        "close": receipt.get("graceful_process_cleanup_verified") is True,
        "reopen_same_run": reopen.get("sameRunRestored") is True,
        "cleanup": receipt.get("graceful_process_cleanup_verified") is True,
    }
    ancillary = {
        "report_passed": report.get("passed") is True,
        "no_console_errors": report.get("consoleErrors") == [],
        "no_failures": report.get("failures") == [],
        "exact_required_check_contract": receipt.get("checks") == EXPECTED_CHECK_COUNT
        and receipt.get("unique_checks") == EXPECTED_CHECK_COUNT
        and validate_required_report_checks(PACKAGED_ACCEPTANCE_CONTRACT, checks)["passed"]
        and all(functional_check_passed(row) for row in checks.values()),
        "offline_functional": offline.get("passed") is True
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("externalRequestCount") == 0,
        "actual_packaged_viewport_contract": visual_check["passed"]
        and visual_check["actual_tauri_window"] is True
        and visual_check["viewport_emulation"] is False,
        "functional_screenshots_exact": len(functional_screenshots) == len(CBSEM_CAPTURE_STATES),
    }
    return {
        "passed": all(steps.values()) and all(ancillary.values()),
        "workflow_steps": steps,
        "ancillary": ancillary,
        "visual": visual_check,
    }


def locate_and_verify_receipt(not_before: datetime) -> tuple[dict[str, Any], Path, dict[str, Any]]:
    receipt = strict_load_json(CUMULATIVE_RECEIPT)
    build_receipt = strict_load_json(BUILD_RECEIPT)
    expected_contract_descriptor = packaged_acceptance_contract_descriptor()
    report_path = ROOT / receipt["report"]
    report_descriptor = {
        "size": receipt.get("report_size"),
        "sha256": receipt.get("report_sha256"),
    }
    if not (
        receipt.get("schema_version") == 2
        and receipt.get("kind") == "quickpls_v247_cumulative_native_acceptance_receipt"
        and receipt.get("passed") is True
        and receipt.get("checks") == EXPECTED_CHECK_COUNT
        and receipt.get("unique_checks") == EXPECTED_CHECK_COUNT
        and receipt.get("failures") == 0
        and receipt.get("console_errors") == 0
        and receipt.get("final_scope") == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"]
        and isinstance(receipt.get("acceptance_contract"), dict)
        and receipt["acceptance_contract"].get("path")
        == "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
        and receipt["acceptance_contract"].get("contract_id")
        == PACKAGED_ACCEPTANCE_CONTRACT["contract_id"]
        and receipt["acceptance_contract"].get("contract_version")
        == PACKAGED_ACCEPTANCE_CONTRACT["contract_version"]
        and receipt["acceptance_contract"].get("required_check_count")
        == EXPECTED_CHECK_COUNT
        and receipt["acceptance_contract"].get("sha256")
        == sha256_file(PACKAGED_ACCEPTANCE_CONTRACT_PATH)
        and receipt["acceptance_contract"].get("bundled_sample_catalog")
        == expected_contract_descriptor["bundled_sample_catalog"]
        and receipt.get("graceful_process_cleanup_verified") is True
        and _utc(receipt.get("supervisor_started_at_utc"))
        >= _utc(build_receipt.get("build_finished_at_utc"))
        and _utc(receipt.get("supervisor_started_at_utc"))
        >= not_before - timedelta(seconds=2)
        and _utc(receipt.get("completed_at_utc"))
        >= _utc(receipt.get("supervisor_started_at_utc"))
        and repository_path(report_path)
        == "validation/results/v247_tauri_native_acceptance.json"
        and _receipt_descriptor_matches(report_path, report_descriptor)
    ):
        raise ValueError("cumulative v247 receipt does not bind a passing current report")
    exports = [row for row in receipt.get("exports", []) if row.get("role") == "cbsem"]
    if len(exports) != 1:
        raise ValueError("cumulative v247 receipt must contain exactly one CB-SEM workbook")
    return receipt, report_path, exports[0]


def release_blocker_report(error: Exception) -> dict[str, Any]:
    blockers = [f"fresh receipt/build binding: {type(error).__name__}: {error}"]
    diagnostics: dict[str, Any] = {}
    try:
        receipt = strict_load_json(CUMULATIVE_RECEIPT)
        candidate = ROOT / receipt["report"]
        report = strict_load_json(candidate)
        workflow = packaged_workflow_contract(report, receipt)
        diagnostics["prior_workflow_contract"] = workflow
        if not workflow["workflow_steps"]["invalid_setup_blocked"]:
            blockers.append(
                "packaged invalid setup: no fail-closed CB-SEM invalid-setup/no-state-mutation proof"
            )
        if not workflow["ancillary"]["actual_packaged_viewport_contract"]:
            blockers.append(
                "actual packaged window matrix: exact 1024x700, 1280x720, and 1440x900 CB-SEM proof is absent"
            )
    except (KeyError, OSError, ValueError, TypeError) as diagnostic_error:
        diagnostics["prior_workflow_error"] = (
            f"{type(diagnostic_error).__name__}: {diagnostic_error}"
        )
    return {
        "passed": False,
        "release_qualified": False,
        "highest_honest_state": "native_qualified",
        "release_blockers": blockers,
        "diagnostics": diagnostics,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--reuse-existing", action="store_true", default=True)
    parser.add_argument("--not-before-utc", required=True)
    args = parser.parse_args()
    not_before = _utc(args.not_before_utc)
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)

    freshness = source_freshness()
    prior = verify_prior_factory_stages()
    preflight = {"passed": freshness["passed"] and prior["passed"], "freshness": freshness, "prior": prior}
    if not preflight["passed"]:
        PREFLIGHT_REPORT.write_text(
            json.dumps(preflight, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )
        print(f"packaged CB-SEM preflight failed; wrote {PREFLIGHT_REPORT}")
        return 1

    try:
        build_receipt = strict_load_json(BUILD_RECEIPT)
        build_finished = _utc(build_receipt.get("build_finished_at_utc"))
        harness_changed = datetime.fromtimestamp(
            (ROOT / "validation/v247_tauri_native_acceptance.mjs").stat().st_mtime,
            timezone.utc,
        )
        receipt, report_path, workbook_descriptor = locate_and_verify_receipt(
            max(not_before, build_finished, harness_changed)
        )
        report = strict_load_json(report_path)
        if _utc(report.get("generatedAt")) < build_finished:
            raise ValueError("focused/cumulative CB-SEM report predates the frozen build")
        if (
            report.get("runtime") != "tauri-webview2-cdp"
            or report.get("focusedRun", {}).get("scope") != "regression_bootstrap"
        ):
            raise ValueError("cumulative CB-SEM report is not the final packaged Tauri scope")
        archive_path = Path(report["checks"]["cbsemFixtureProvisioning"]["project"])
        workbook_path = ROOT / workbook_descriptor["path"]
        if not archive_path.is_file() or not workbook_path.is_file():
            raise FileNotFoundError("receipt-bound CB-SEM archive or workbook is missing")
        if not _receipt_descriptor_matches(workbook_path, workbook_descriptor):
            raise ValueError("receipt-bound CB-SEM workbook bytes changed")
    except (KeyError, OSError, ValueError, TypeError) as error:
        blocker_report = release_blocker_report(error)
        blocker_report["preflight"] = preflight
        PREFLIGHT_REPORT.write_text(
            json.dumps(blocker_report, indent=2, sort_keys=True, allow_nan=False) + "\n",
            encoding="utf-8",
        )
        print(f"packaged CB-SEM release remains blocked; wrote {PREFLIGHT_REPORT}")
        return 1

    shutil.copy2(report_path, FACTORY_RAW_REPORT)
    shutil.copy2(archive_path, FACTORY_ARCHIVE)
    shutil.copy2(workbook_path, FACTORY_XLSX)
    project, archive_manifest = read_archive(FACTORY_ARCHIVE)
    archive = archive_contract(project, report)
    workbook = workbook_contract(FACTORY_XLSX, workbook_descriptor)
    workflow = packaged_workflow_contract(report, receipt)
    functional_screenshots = functional_screenshot_paths(report)
    visual_screenshots = visual_screenshot_paths(report, run_id=report["checks"]["cbsemResult"]["runId"])
    checks = {
        "passed": freshness["passed"]
        and prior["passed"]
        and archive["passed"]
        and workbook["passed"]
        and workflow["passed"],
        "source_freshness": freshness,
        "prior_factory_stages": prior,
        "archive": archive,
        "archive_manifest_engine_version": archive_manifest.get("engine_version"),
        "workbook": workbook,
        "workflow": workflow,
        "focused_scope": {"scope": "cbsem", "method_version": "cbsem_ml_v1"},
    }
    product_sources = discover_product_source(ROOT)["paths"]
    identity_path = write_identity_report(
        "packaged_acceptance",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/diagnostic_bundle_source_manifest.py",
            "validation/run_v247_cbsem_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            repository_path(BUILD_RECEIPT),
            repository_path(CUMULATIVE_RECEIPT),
            repository_path(FACTORY_RAW_REPORT),
            repository_path(FACTORY_ARCHIVE),
            repository_path(FACTORY_XLSX),
            repository_path(DESKTOP),
            repository_path(RELEASE_CLI),
            *[repository_path(path) for path in functional_screenshots],
            *[repository_path(path) for path in visual_screenshots],
            *product_sources,
        ],
    )
    print(f"wrote {identity_path} | passed={checks['passed']}")
    if not checks["passed"]:
        return 1
    audit = subprocess.run(
        [sys.executable, "validation/cbsem_ml_v1_factory_audit.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=600,
    )
    if audit.returncode != 0:
        print((audit.stdout + "\n" + audit.stderr)[-3000:])
        return 1
    final_freshness = source_freshness()
    if final_freshness != freshness:
        print("CB-SEM product/build/CLI source closure changed during the final audit")
        return 1
    final = validate_manifest(ROOT / "validation/methods/cbsem_ml_v1.manifest.json", ROOT)
    if final.get("passed") is not True or final.get("derived_state") != "release_qualified":
        print(json.dumps(final, indent=2, sort_keys=True))
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
