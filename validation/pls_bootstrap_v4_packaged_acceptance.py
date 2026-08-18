"""Receipt-bound packaged Windows acceptance for PLS Bootstrap v4.

The shared cumulative runner owns GUI execution. This adapter independently
verifies its Bootstrap-specific evidence, binds the exact saved run and XLSX,
and promotes only after the final method audit derives release qualification.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

from diagnostic_bundle_source_manifest import SourceManifestFailure, validate_build_receipt
from method_promotion_manifest import _verify_artifact, validate_manifest
from packaged_windows_acceptance_v2 import (
    CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
    CONTRACT_FILE_SHA256,
    EXPECTED_CHECK_COUNT,
    receipt_binds_packaged_acceptance_contract,
    validate_required_report_checks,
)
from pls_bootstrap_v4_factory_common import (
    MANIFEST_PATH,
    REPORT_ROOT,
    ROOT,
    manifest,
    repository_path,
    run_command,
    sha256_file,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_bootstrap_v4_packaged_acceptance.py"
RAW_REPORT = ROOT / "validation" / "results" / "v247_tauri_native_acceptance.json"
CUMULATIVE_RECEIPT = (
    ROOT / "validation" / "results" / "v247_cumulative_native_acceptance_receipt.json"
)
FACTORY_RAW_REPORT = REPORT_ROOT / "pls_bootstrap_v4_packaged_raw.json"
FACTORY_ARCHIVE = REPORT_ROOT / "pls_bootstrap_v4_packaged.qpls"
FACTORY_XLSX = REPORT_ROOT / "pls_bootstrap_v4_packaged.xlsx"
BUILD_RECEIPT = ROOT / "validation" / "results" / "diagnostic_bundle_build_receipt.json"
DESKTOP = ROOT / "target" / "release" / "quickpls-desktop.exe"
RELEASE_CLI = ROOT / "target" / "release" / "qpls.exe"
EXPECTED_CUMULATIVE_CHECKS = EXPECTED_CHECK_COUNT
REQUIRED_VIEWPORTS = {"1024x700", "1280x720", "1440x900"}
GATE_SOURCES = {
    SOURCE,
    "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
    "validation/diagnostic_bundle_source_manifest.py",
    "validation/method_promotion_manifest.py",
    "validation/methods/method_promotion_manifest.schema.json",
    "validation/methods/pls_bootstrap_v4.manifest.json",
    "validation/packaged_windows_acceptance_v2.py",
    "validation/pls_bootstrap_v4_factory_audit.py",
    "validation/pls_bootstrap_v4_factory_common.py",
    "validation/run_v247_cumulative_native_acceptance.ps1",
    "validation/test_pls_bootstrap_v4_packaged_adapter.py",
    "validation/test_v247_cumulative_native_acceptance_supervisor.py",
    "validation/v247_tauri_native_acceptance.mjs",
}


def cli_source_paths() -> list[str]:
    paths = {"Cargo.lock", "Cargo.toml", "crates/qpls-cli/Cargo.toml"}
    paths.update(
        repository_path(path)
        for path in (ROOT / "crates" / "qpls-cli" / "src").rglob("*.rs")
        if path.is_file()
    )
    for crate in (
        "qpls-assessment", "qpls-core", "qpls-data", "qpls-estimation",
        "qpls-project", "qpls-resampling", "qpls-runner",
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


def _descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": path.resolve().relative_to(ROOT.resolve()).as_posix(),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
        "mtime_ns": path.stat().st_mtime_ns,
    }


def source_freshness() -> dict[str, Any]:
    try:
        if not BUILD_RECEIPT.is_file() or not DESKTOP.is_file() or not RELEASE_CLI.is_file():
            raise FileNotFoundError("frozen build receipt, release desktop, or release CLI is missing")
        receipt = strict_load_json(BUILD_RECEIPT)
        validate_build_receipt(receipt, ROOT)
        desktop = _descriptor(DESKTOP)
        release_cli = _descriptor(RELEASE_CLI)
        cli_rows = [_descriptor(ROOT / relative) for relative in cli_source_paths()]
        cli_newer = [row["path"] for row in cli_rows if row["mtime_ns"] > release_cli["mtime_ns"]]
        gate_rows = [_descriptor(ROOT / relative) for relative in sorted(GATE_SOURCES)]
        return {
            "passed": not cli_newer,
            "desktop_receipt_exact": True,
            "desktop": desktop,
            "release_cli": release_cli,
            "release_cli_newer_build_sources": cli_newer,
            "release_cli_build_sources": cli_rows,
            "gate_sources_excluded_from_binary_freshness": gate_rows,
            "build_receipt": _descriptor(BUILD_RECEIPT),
            "build_receipt_schema_version": receipt.get("schema_version"),
        }
    except (FileNotFoundError, OSError, ValueError, SourceManifestFailure) as error:
        return {"passed": False, "desktop_receipt_exact": False, "error": str(error)}


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
        "engine_only": {"method_spec", "independent_reference", "simulation_report", "boundary_report"},
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    rows: list[dict[str, Any]] = []
    for stage, roles in expected_roles.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
            rows.append({"stage": stage, "path": artifact["path"], "roles": artifact["roles"], "passed": passed, "errors": errors})
        if observed != roles:
            rows.append({"stage": stage, "passed": False, "errors": [f"expected roles {sorted(roles)}, found {sorted(observed)}"]})
    return {"passed": all(row["passed"] for row in rows), "artifacts": rows}


def _parse_utc(value: str) -> datetime:
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("UTC timestamp must contain an offset")
    return parsed.astimezone(timezone.utc)


def verify_reusable_cumulative_receipt(not_before: datetime) -> dict[str, Any]:
    """Bind skip-run reuse to the exact cumulative report and Bootstrap XLSX."""

    try:
        if not CUMULATIVE_RECEIPT.is_file() or not RAW_REPORT.is_file():
            raise FileNotFoundError("cumulative receipt or cumulative report is missing")
        receipt = strict_load_json(CUMULATIVE_RECEIPT)
        report = strict_load_json(RAW_REPORT)
        supervisor_started = _parse_utc(receipt["supervisor_started_at_utc"])
        completed = _parse_utc(receipt["completed_at_utc"])
        report_descriptor = _descriptor(RAW_REPORT)
        exports = receipt.get("exports")
        if not isinstance(exports, list):
            raise ValueError("cumulative receipt exports must be a list")
        export_rows_valid = all(
            isinstance(row, dict) and isinstance(row.get("role"), str)
            for row in exports
        )
        export_roles = [row.get("role") for row in exports if isinstance(row, dict)]
        bootstrap_rows = [
            row
            for row in exports
            if isinstance(row, dict) and row.get("role") == "bootstrap"
        ]
        if len(bootstrap_rows) != 1:
            raise ValueError(
                "cumulative receipt must contain exactly one Bootstrap export"
            )
        bootstrap = bootstrap_rows[0]
        bootstrap_relative = bootstrap.get("path")
        if (
            not isinstance(bootstrap_relative, str)
            or Path(bootstrap_relative).is_absolute()
            or "\\" in bootstrap_relative
            or ".." in Path(bootstrap_relative).parts
        ):
            raise ValueError("Bootstrap export path is unsafe")
        bootstrap_path = (ROOT / bootstrap_relative).resolve()
        if ROOT.resolve() not in bootstrap_path.parents or not bootstrap_path.is_file():
            raise ValueError("Bootstrap export is missing or escapes the repository")
        bootstrap_descriptor = _descriptor(bootstrap_path)
        native_xlsx = (
            report.get("checks", {})
            .get("mediationExport", {})
            .get("bootstrap", {})
            .get("nativeXlsx", {})
        )
        workbook = (
            native_xlsx
            .get("helper", {})
            .get("completion", {})
            .get("workbook", {})
        )
        workbook_path_value = workbook.get("path")
        target_path_value = native_xlsx.get("targetPath")
        if not isinstance(workbook_path_value, str) or not isinstance(
            target_path_value, str
        ):
            raise ValueError("Bootstrap workbook or target path is missing from the report")
        workbook_path = Path(workbook_path_value)
        if not workbook_path.is_absolute():
            workbook_path = ROOT / workbook_path
        workbook_path = workbook_path.resolve()
        target_path = Path(target_path_value)
        if not target_path.is_absolute():
            target_path = ROOT / target_path
        target_path = target_path.resolve()
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
            "export_roles_unique": export_rows_valid
            and len(export_roles) == len(set(export_roles)),
            "bootstrap_export_exact_bytes": bootstrap.get("path")
            == bootstrap_descriptor["path"]
            and bootstrap.get("size") == bootstrap_descriptor["size"]
            and bootstrap.get("sha256") == bootstrap_descriptor["sha256"],
            "bootstrap_export_bound_to_report": workbook_path == bootstrap_path
            and target_path == bootstrap_path
            and workbook.get("size") == bootstrap_descriptor["size"]
            and workbook.get("sha256") == bootstrap_descriptor["sha256"],
        }
        receipt_descriptor = _descriptor(CUMULATIVE_RECEIPT)
        return {
            "passed": all(checks.values()),
            "checks": checks,
            "not_before_utc": not_before.isoformat().replace("+00:00", "Z"),
            "supervisor_started_at_utc": receipt["supervisor_started_at_utc"],
            "completed_at_utc": receipt["completed_at_utc"],
            "receipt": receipt_descriptor,
            "report": report_descriptor,
            "bootstrap_export": bootstrap_descriptor,
            "artifact_paths": [
                receipt_descriptor["path"],
                bootstrap_descriptor["path"],
            ],
        }
    except (KeyError, TypeError, FileNotFoundError, OSError, ValueError) as error:
        return {"passed": False, "error": f"{type(error).__name__}: {error}"}


def evaluate_native_report(
    report: dict[str, Any], started: datetime, *, cumulative_wrapper_passed: bool
) -> dict[str, Any]:
    checks = report.get("checks", {})
    invalid = checks.get("bootstrapInvalidSetup", {})
    dialog = checks.get("mediationBootstrapDialog", {})
    result = checks.get("mediationBootstrapResult", {})
    export = checks.get("mediationExport", {}).get("bootstrap", {})
    native_xlsx = export.get("nativeXlsx", {})
    workbook = native_xlsx.get("helper", {}).get("completion", {}).get("workbook", {})
    reopen = checks.get("mediationSaveReopen", {})
    retry = checks.get("bootstrapCancellationRetry", {})
    responsive = checks.get("bootstrapResponsiveViewports", {})
    offline = checks.get("bootstrapFunctionalOffline", {})
    runtime = checks.get("runtime", {})
    focused = report.get("focusedRun", {})
    run_id = result.get("runId")
    setup_viewports = {row.get("id") for row in responsive.get("setup", [])}
    result_viewports = {row.get("id") for row in responsive.get("results", [])}
    semantic = {
        "fresh_cumulative_chain": (
            cumulative_wrapper_passed
            and report.get("passed") is True
            and _parse_utc(report["generatedAt"]) >= started
            and focused.get("scope") == "regression_bootstrap"
            and _parse_utc(focused["completedAt"]) >= started
        ),
        "clean_report": not report.get("failures") and not report.get("consoleErrors"),
        "packaged_tauri_runtime": runtime.get("tauriRuntime") is True,
        "invalid_setup_blocked_without_run": (
            invalid.get("attempted") is True and invalid.get("startEnabled") is False
            and bool(invalid.get("blockers")) and invalid.get("runStateUnchanged") is True
            and invalid.get("resultCreated") is False
            and invalid.get("archiveBefore", {}).get("recipeCount") == 0
            and invalid.get("archiveBefore", {}).get("resultCount") == 0
            and invalid.get("archiveBefore", {}).get("runCount") == 0
        ),
        "exact_valid_setup": (
            "PLS-SEM Bootstrapping" in dialog.get("selectedMethod", "")
            and dialog.get("bootstrapSamples") == "100"
            and dialog.get("studentizedInnerSamples") == "0"
            and dialog.get("startEnabled") is True and not dialog.get("blockers")
        ),
        "completed_bootstrap_result": (
            bool(run_id) and "PLS-SEM Bootstrapping" in result.get("runLabel", "")
            and result.get("navigation", {}).get("bootstrapTreeItems") == 1
            and result.get("navigation", {}).get("rowCounts", {}).get(
                "Aggregate mediation effects bootstrap inference"
            ) == 6
        ),
        "selected_run_native_xlsx": (
            export.get("selectedRunId") == run_id and native_xlsx.get("attempted") is True
            and native_xlsx.get("selectedRunId") == run_id
            and native_xlsx.get("file", {}).get("isFile") is True
            and native_xlsx.get("file", {}).get("size", 0) > 0
            and native_xlsx.get("helper", {}).get("completion", {}).get("passed") is True
            and "Bootstrapping" in workbook.get("sheetNames", [])
            and "Run provenance" in workbook.get("sheetNames", [])
        ),
        "same_run_reopened": (
            reopen.get("hasBootstrap") is True
            and reopen.get("selectedRunId") == run_id
            and reopen.get("expectedBootstrapRunId") == run_id
        ),
        "cancel_retry_identity": (
            retry.get("passed") is True
            and retry.get("cancelledPartialRunVisible") == 0
            and retry.get("completedRetryRunId") == run_id
            and retry.get("exportedRunId") == run_id
            and retry.get("reopenedRunId") == run_id
        ),
        "exact_three_responsive_viewports": (
            responsive.get("passed") is True
            and setup_viewports == REQUIRED_VIEWPORTS and result_viewports == REQUIRED_VIEWPORTS
            and len(responsive.get("setup", [])) == 3 and len(responsive.get("results", [])) == 3
            and all(row.get("passed") is True for row in responsive.get("setup", []))
            and all(row.get("passed") is True for row in responsive.get("results", []))
        ),
        "functional_offline_without_zero_egress_overclaim": (
            offline.get("passed") is True
            and offline.get("analyticalWorkflowRequiresInternet") is False
            and offline.get("strictZeroProcessEgressClaimed") is False
            and offline.get("externalRequestCount") == 0
            and offline.get("observedRequestCount", 0) > 0
        ),
    }
    return {"passed": all(semantic.values()), "checks": semantic, "run_id": run_id, "raw": {
        "invalid_setup": invalid, "setup": dialog, "result": result, "export": export,
        "save_reopen": reopen, "cancel_retry": retry, "responsive": responsive, "offline": offline,
    }}


def inspect_bootstrap_archive(path: Path, run_id: str, setup: dict[str, Any]) -> dict[str, Any]:
    with zipfile.ZipFile(path) as archive:
        names = [entry.filename for entry in archive.infolist()]
        if len(names) != len(set(names)) or any(
            name.startswith(("/", "\\")) or "\\" in name or ".." in Path(name).parts
            for name in names
        ):
            raise ValueError("Bootstrap archive has duplicate or unsafe entries")
        entries = {name: archive.read(name) for name in names}
    project = json.loads(entries["project.json"].decode("utf-8"))
    archive_manifest = json.loads(entries["manifest.json"].decode("utf-8"))
    checksums = archive_manifest.get("checksums", {})
    checksum_exact = set(checksums) == set(entries) - {"manifest.json"} and all(
        hashlib.sha256(entries[name]).hexdigest() == expected for name, expected in checksums.items()
    )
    results = [row for row in project.get("results", []) if row.get("id") == run_id]
    if len(results) != 1:
        raise ValueError("Bootstrap run does not map to exactly one archived result")
    result = results[0]
    recipes = [row for row in project.get("recipes", []) if row.get("id") == result.get("provenance", {}).get("recipe_id")]
    runs = [row for row in project.get("layouts", {}).get("workspace", {}).get("runs", []) if row.get("id") == run_id]
    if len(recipes) != 1 or len(runs) != 1:
        raise ValueError("Bootstrap result does not map to exactly one recipe and run")
    recipe = recipes[0]
    bootstrap = result.get("payload", {}).get("bootstrap", {})
    settings = recipe.get("settings", {})
    provenance_versions = str(
        result.get("provenance", {}).get("method_version", "")
    ).split("+")
    identity = {
        "archive_checksums_exact": checksum_exact,
        "run_completed": runs[0].get("status") == "completed",
        "run_label": runs[0].get("method") == "PLS-SEM Bootstrapping",
        "recipe_kind": recipe.get("method_config") == {"kind": "pls_bootstrap"},
        "payload_kind": result.get("payload", {}).get("kind") in {"pls_pm_v2", "pls_pm_v3"},
        "method_version": bootstrap.get("method_version") == "indexed_resampling_v4",
        "provenance_version": provenance_versions.count("indexed_resampling_v4") == 1,
        "replicates": bootstrap.get("plan", {}).get("replicates") == int(setup["bootstrapSamples"]),
        "master_seed": bootstrap.get("plan", {}).get("master_seed") == int(setup["seed"]),
        "usable_failure_accounting": bootstrap.get("usable_replicates", 0) + len(bootstrap.get("failed_replicates", [])) == int(setup["bootstrapSamples"]),
        "recipe_settings": (
            settings.get("bootstrap_samples") == int(setup["bootstrapSamples"])
            and settings.get("studentized_inner_samples") == int(setup["studentizedInnerSamples"])
            and settings.get("seed") == int(setup["seed"])
            and settings.get("workers") == int(setup["workers"])
            and settings.get("confidence_level") == float(setup["confidenceLevel"]) / 100
        ),
    }
    return {"passed": all(identity.values()), "checks": identity, "schema_version": archive_manifest.get("schema_version")}


def screenshot_artifacts(report: dict[str, Any]) -> list[str]:
    paths = report.get("screenshots", [])
    required = {
        f"43-tauri-native-mediation-bootstrap-dialog-{viewport}.png" for viewport in REQUIRED_VIEWPORTS
    } | {
        f"46-tauri-native-mediation-bootstrap-results-reopened-{viewport}.png" for viewport in REQUIRED_VIEWPORTS
    }
    matches: list[str] = []
    for name in required:
        candidates = [Path(value) for value in paths if Path(value).name == name]
        if len(candidates) != 1 or not candidates[0].is_file() or candidates[0].stat().st_size <= 0:
            raise ValueError(f"expected one non-empty Bootstrap screenshot {name}")
        matches.append(repository_path(candidates[0]))
    return sorted(matches)


def finalize_release_promotion() -> dict[str, Any]:
    completed, execution = run_command(["python", "validation/pls_bootstrap_v4_factory_audit.py"], timeout=600)
    if completed.returncode != 0:
        return {"passed": False, "phase": "method_audit", "execution": execution}
    final = validate_manifest(MANIFEST_PATH, ROOT)
    return {"passed": final.get("passed") is True and final.get("derived_state") == "release_qualified", "phase": "final_manifest", "execution": execution, "manifest_validation": final}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument("--not-before-utc")
    args = parser.parse_args()
    if args.skip_run and not args.not_before_utc:
        parser.error("--skip-run requires --not-before-utc")
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    prior = verify_prior_factory_stages()
    freshness_before = source_freshness()
    if not prior["passed"] or not freshness_before["passed"]:
        print(json.dumps({"prior": prior, "source_freshness": freshness_before}, indent=2))
        return 1
    started = _parse_utc(args.not_before_utc) if args.not_before_utc else datetime.now(timezone.utc)
    executions: list[dict[str, Any]] = []
    wrapper_passed = False
    cumulative_reuse: dict[str, Any] | None = None
    if args.skip_run:
        cumulative_reuse = verify_reusable_cumulative_receipt(started)
        if not cumulative_reuse["passed"]:
            print(json.dumps({"phase": "cumulative_receipt_reuse", **cumulative_reuse}, indent=2))
            return 1
        wrapper_passed = True
    if not args.skip_run:
        completed, execution = run_command([
            "powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File",
            "validation/run_v247_cumulative_native_acceptance.ps1",
        ], timeout=7200)
        executions.append(execution)
        if completed.returncode != 0:
            print(json.dumps(execution, indent=2))
            return 1
        wrapper_passed = True
    if not RAW_REPORT.is_file():
        raise FileNotFoundError(RAW_REPORT)
    raw = strict_load_json(RAW_REPORT)
    native = evaluate_native_report(raw, started, cumulative_wrapper_passed=wrapper_passed)
    if not native["passed"]:
        print(json.dumps(native, indent=2))
        return 1
    source_archive = Path(raw["checks"]["mediationRecentProject"]["projectPath"])
    source_xlsx = Path(native["raw"]["export"]["nativeXlsx"]["targetPath"])
    if not source_archive.is_file() or not source_xlsx.is_file():
        raise FileNotFoundError("Bootstrap archive or native XLSX is missing")
    shutil.copy2(source_archive, FACTORY_ARCHIVE)
    shutil.copy2(source_xlsx, FACTORY_XLSX)
    shutil.copy2(RAW_REPORT, FACTORY_RAW_REPORT)
    archive = inspect_bootstrap_archive(FACTORY_ARCHIVE, native["run_id"], native["raw"]["setup"])
    screenshots = screenshot_artifacts(raw)
    freshness_after = source_freshness()
    stable = freshness_before == freshness_after
    detail = {
        "passed": native["passed"] and archive["passed"] and freshness_after["passed"] and stable,
        "prior_factory_stages": prior,
        "source_freshness": {"passed": freshness_before["passed"] and freshness_after["passed"] and stable, "source_stable_during_gate": stable, "before": freshness_before, "after": freshness_after},
        "tested_binaries": {"desktop": freshness_after.get("desktop"), "release_cli": freshness_after.get("release_cli")},
        "cumulative_execution": (
            {"passed": True, "mode": "receipt_reuse", "receipt": cumulative_reuse}
            if cumulative_reuse is not None
            else {"passed": wrapper_passed, "mode": "direct_run"}
        ),
        "native": native,
        "archive": {**archive, "path": repository_path(FACTORY_ARCHIVE), "sha256": sha256_file(FACTORY_ARCHIVE)},
        "xlsx": {"path": repository_path(FACTORY_XLSX), "sha256": sha256_file(FACTORY_XLSX), "size": FACTORY_XLSX.stat().st_size},
        "responsive_screenshots": screenshots,
        "runner_cleanup_verified": wrapper_passed,
    }
    report_path = write_identity_report(
        "packaged_acceptance", passed=detail["passed"], checks=detail,
        extras=[*sorted(GATE_SOURCES), repository_path(BUILD_RECEIPT), repository_path(DESKTOP),
                repository_path(RELEASE_CLI), repository_path(FACTORY_RAW_REPORT),
                repository_path(FACTORY_ARCHIVE), repository_path(FACTORY_XLSX), *screenshots,
                *(cumulative_reuse.get("artifact_paths", []) if cumulative_reuse is not None else [])],
        execution={"commands": executions},
    )
    print(f"wrote {report_path} | passed={detail['passed']}")
    if not detail["passed"]:
        return 1
    promotion = finalize_release_promotion()
    if not promotion["passed"]:
        print(json.dumps(promotion, indent=2))
        return 1
    print("PLS Bootstrap v4 derived state: release_qualified")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
