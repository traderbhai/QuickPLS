#!/usr/bin/env python3
"""Receipt-bound packaged Windows acceptance for PLSc bootstrap v1.

The release build is shared with the surrounding promotion batch.  This gate
either invokes only the method-scoped supervisor or consumes its append-only
receipt, then independently binds the exact run, archive, XLSX, viewports,
offline observation, cancellation/retry, and clean process shutdown.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import shutil
import subprocess
import sys
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Iterable
from xml.etree import ElementTree

try:
    from validation.method_promotion_manifest import validate_manifest
    from validation.phase2_release_packaged_common import (
        AdapterError,
        MethodContract,
        ROOT,
        descriptor,
        output_path,
        packaged_viewport_contract,
        parse_utc,
        read_archive,
        repository_path,
        require,
        role_sources,
        source_freshness,
        strict_json,
        verify_prior_factory_stages,
        write_identity,
    )
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.method_promotion_manifest import validate_manifest  # type: ignore[no-redef]
    from validation.phase2_release_packaged_common import (  # type: ignore[no-redef]
        AdapterError,
        MethodContract,
        ROOT,
        descriptor,
        output_path,
        packaged_viewport_contract,
        parse_utc,
        read_archive,
        repository_path,
        require,
        role_sources,
        source_freshness,
        strict_json,
        verify_prior_factory_stages,
        write_identity,
    )


CONTRACT = MethodContract(
    slug="consistent_bootstrap_v1",
    feature_id="qpls3.inference.consistent_bootstrap",
    method_version="plsc_bootstrap_v1",
    catalogue_date="2026-08-12",
    selected_method="PLSc Consistent Bootstrapping",
    archive_method="plsc",
    receipt_role="plsc_bootstrap",
    dialog_key="plscBootstrapDialog",
    result_key="plscBootstrapResult",
    progress_key="plscBootstrapProgress",
    export_key="plscBootstrapExport",
    reopen_key="plscBootstrapSaveReopen",
    fixture_key="plscBootstrapFixture",
    project_field="projectPath",
    invalid_key="plscBootstrapInvalidSetup",
    cancellation_key="plscBootstrapCancellation",
    visual_key="plscBootstrap",
    visual_state="plsc-bootstrap-dialog",
    capture_prefix="plsc-bootstrap",
    captures=(),
    exact_result_values=(),
    minimum_result_values=(),
    progress_kind="captured",
    adapter_script="validation/consistent_bootstrap_v1_packaged_acceptance.py",
    audit_script="validation/consistent_bootstrap_v1_factory_audit.py",
)
OUTPUT = ROOT / "validation/results/method_factory/consistent_bootstrap_v1"
RAW_REPORT = ROOT / "validation/results/v247_tauri_native_acceptance_plsc_bootstrap.json"
WRAPPER = ROOT / "validation/run_v247_plsc_bootstrap_native_acceptance.ps1"
HARNESS = ROOT / "validation/v247_tauri_native_acceptance.mjs"
MANIFEST = ROOT / "validation/methods/consistent_bootstrap_v1.manifest.json"
EXPECTED_CHECK_IDS = (
    "plscBootstrapCancellation",
    "plscBootstrapDialog",
    "plscBootstrapExport",
    "plscBootstrapFixture",
    "plscBootstrapFixtureProvisioning",
    "plscBootstrapFunctionalOffline",
    "plscBootstrapInitialModel",
    "plscBootstrapInvalidSetup",
    "plscBootstrapPackagedViewports",
    "plscBootstrapProgress",
    "plscBootstrapResult",
    "plscBootstrapSaveReopen",
    "recentProjectsRestored",
    "runtime",
    "runtimePreflight",
)
REQUIRED_SHEETS = {
    "PLSc bootstrap replicate accoun",
    "PLSc consistent bootstrapping",
    "Bias-corrected and accelerated",
    "Run provenance",
}
PLSC_BOOTSTRAP_SAMPLES = 10_000
PLSC_BOOTSTRAP_SEED = 20_260_818


def stable_path(name: str) -> Path:
    return OUTPUT / name


def copy_exact(source: Path, target: Path) -> dict[str, Any]:
    require(source.is_file() and source.stat().st_size > 0, f"missing source artifact: {source}")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    require(descriptor(source)["size"] == descriptor(target)["size"], f"copy size differs: {target}")
    require(descriptor(source)["sha256"] == descriptor(target)["sha256"], f"copy hash differs: {target}")
    return descriptor(target)


def xlsx_sheet_names(path: Path) -> list[str]:
    with zipfile.ZipFile(path) as workbook:
        names = [row.filename for row in workbook.infolist()]
        require(len(names) == len(set(names)), "PLSc-bootstrap XLSX contains duplicate members")
        require("xl/workbook.xml" in names, "PLSc-bootstrap XLSX has no workbook manifest")
        root = ElementTree.fromstring(workbook.read("xl/workbook.xml"))
    namespace = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
    return [row.attrib["name"] for row in root.findall(f".//{{{namespace}}}sheet")]


def finite_parameter_map(value: object) -> bool:
    return (
        isinstance(value, dict)
        and bool(value)
        and all(isinstance(key, str) and key and isinstance(item, (int, float))
                and not isinstance(item, bool) and math.isfinite(float(item))
                for key, item in value.items())
    )


def exact_archive_contract(path: Path, run_id: str) -> dict[str, Any]:
    project, archive_manifest = read_archive(path)
    results = [row for row in project.get("results", []) if isinstance(row, dict) and row.get("id") == run_id]
    require(len(results) == 1, "PLSc-bootstrap archive must contain exactly the reported result")
    result = results[0]
    provenance = result.get("provenance")
    payload = result.get("payload")
    require(result.get("status") == "completed" and isinstance(provenance, dict) and isinstance(payload, dict), "PLSc-bootstrap archived result is incomplete")
    recipe_id = provenance.get("recipe_id")
    recipes = [row for row in project.get("recipes", []) if isinstance(row, dict) and row.get("id") == recipe_id]
    runs = [row for row in project.get("layouts", {}).get("workspace", {}).get("runs", []) if isinstance(row, dict) and row.get("id") == run_id]
    require(len(recipes) == 1 and len(runs) == 1, "PLSc-bootstrap result must map to one recipe and completed run")
    recipe = recipes[0]
    bootstrap = payload.get("bootstrap")
    require(isinstance(bootstrap, dict), "PLSc-bootstrap payload is missing")
    plan = bootstrap.get("plan")
    ledger = bootstrap.get("replicate_ledger")
    successful = bootstrap.get("successful_replicates")
    failed = bootstrap.get("failed_replicates")
    successful_jackknife = bootstrap.get("successful_jackknife_cases")
    failed_jackknife = bootstrap.get("failed_jackknife_cases")
    for label, value in (
        ("ledger", ledger), ("successful witnesses", successful), ("failed witnesses", failed),
        ("successful delete-one witnesses", successful_jackknife), ("failed delete-one witnesses", failed_jackknife),
    ):
        require(isinstance(value, list), f"PLSc-bootstrap {label} are missing")
    requested = plan.get("replicates") if isinstance(plan, dict) else None
    usable = bootstrap.get("usable_replicates")
    require(requested == PLSC_BOOTSTRAP_SAMPLES and usable == len(successful) and usable + len(failed) == requested, "PLSc-bootstrap requested/usable/failed accounting differs")
    success_by_index = {row.get("replicate_index"): row for row in successful if isinstance(row, dict)}
    failed_by_index = {row.get("replicate_index"): row for row in failed if isinstance(row, dict)}
    require(len(success_by_index) == len(successful) and len(failed_by_index) == len(failed), "PLSc-bootstrap witness indices are duplicated")
    require(set(success_by_index).isdisjoint(failed_by_index) and set(success_by_index) | set(failed_by_index) == set(range(requested)), "PLSc-bootstrap witness partition is not exact")
    require(len(ledger) == requested, "PLSc-bootstrap ledger does not close over the requested plan")
    for index, row in enumerate(ledger):
        require(isinstance(row, dict) and row.get("replicate_index") == index, "PLSc-bootstrap ledger indices are not exact")
        require(isinstance(row.get("sample_indices_sha256"), str) and re.fullmatch(r"[0-9a-f]{64}", row["sample_indices_sha256"]), "PLSc-bootstrap sample digest is invalid")
        if row.get("status") == "success":
            witness = success_by_index.get(index)
            require(isinstance(witness, dict) and finite_parameter_map(witness.get("parameters")), "PLSc-bootstrap successful witness is not replayable")
            require(isinstance(row.get("parameter_values_sha256"), str) and re.fullmatch(r"[0-9a-f]{64}", row["parameter_values_sha256"]), "PLSc-bootstrap successful ledger digest is invalid")
            require(row.get("reason_code") is None and row.get("message") is None, "PLSc-bootstrap successful ledger row carries failure data")
        elif row.get("status") == "failed":
            failure = failed_by_index.get(index)
            require(isinstance(failure, dict) and failure.get("sample_indices_sha256") == row.get("sample_indices_sha256"), "PLSc-bootstrap failed ledger row lacks its witness")
            require(all(isinstance(failure.get(key), str) and failure[key].strip() for key in ("reason_code", "message")), "PLSc-bootstrap failed replicate is untyped")
            require(row.get("reason_code") == failure.get("reason_code") and row.get("message") == failure.get("message") and row.get("parameter_values_sha256") is None, "PLSc-bootstrap failed ledger row differs from its witness")
        else:
            raise AdapterError("PLSc-bootstrap ledger status is invalid")
    jackknife_count = len(successful_jackknife) + len(failed_jackknife)
    jackknife_indices = [row.get("omitted_case") for row in [*successful_jackknife, *failed_jackknife] if isinstance(row, dict)]
    require(jackknife_count > 0 and len(jackknife_indices) == jackknife_count and set(jackknife_indices) == set(range(jackknife_count)), "PLSc-bootstrap delete-one witnesses are not an exact partition")
    require(all(finite_parameter_map(row.get("parameters")) for row in successful_jackknife if isinstance(row, dict)), "PLSc-bootstrap successful delete-one witness is not replayable")
    require(all(all(isinstance(row.get(key), str) and row[key].strip() for key in ("reason_code", "message")) for row in failed_jackknife if isinstance(row, dict)), "PLSc-bootstrap failed delete-one witness is untyped")
    versions = str(provenance.get("method_version", "")).split("+")
    settings = recipe.get("settings")
    contract = {
        "passed": True,
        "run_id": run_id,
        "recipe_id": recipe_id,
        "payload_kind": payload.get("kind"),
        "provenance_method": provenance.get("method"),
        "method_versions": versions,
        "requested": requested,
        "attempted": len(ledger),
        "usable": usable,
        "failed": len(failed),
        "successful_witnesses": len(successful),
        "successful_delete_one_witnesses": len(successful_jackknife),
        "failed_delete_one_witnesses": len(failed_jackknife),
        "archive_engine_version": archive_manifest.get("engine_version"),
        "checksums_verified": True,
    }
    require(
        runs[0].get("status") == "completed"
        and payload.get("kind") == "pls_pm_v2"
        and provenance.get("method") == "plsc"
        and "plsc_v2" in versions and "plsc_bootstrap_v1" in versions
        and bootstrap.get("method_version") == "plsc_bootstrap_v1"
        and bootstrap.get("estimator_method_version") == "plsc_v2"
        and isinstance(plan, dict) and plan.get("master_seed") == PLSC_BOOTSTRAP_SEED
        and plan.get("operation") == "plsc_consistent_bootstrap_v1"
        and bootstrap.get("retry_policy") == "no_retry_no_replacement_fixed_indexed_draws_v1"
        and isinstance(settings, dict) and settings.get("method") == "plsc"
        and settings.get("bootstrap_samples") == PLSC_BOOTSTRAP_SAMPLES
        and settings.get("seed") == PLSC_BOOTSTRAP_SEED
        and recipe.get("method_config") == {"kind": "plsc"}
        and provenance.get("dataset_fingerprint") == recipe.get("dataset_fingerprint")
        and isinstance(bootstrap.get("percentile", {}).get("parameters"), list)
        and bool(bootstrap["percentile"]["parameters"]),
        f"PLSc-bootstrap archive identity or recipe differs: {contract}",
    )
    return contract


def read_network_samples(path: Path) -> dict[str, Any]:
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
    require(bool(rows) and all(isinstance(row, dict) for row in rows), "PLSc-bootstrap network sample family is empty or malformed")
    require(all(row.get("root_present") is True and row.get("observation") == "sampled_exact_process_tree_tcp_v1" for row in rows), "PLSc-bootstrap process-tree network sampling was incomplete")
    require(all(isinstance(row.get("remote_connections"), list) for row in rows), "PLSc-bootstrap process-tree remote-connection rows are malformed")
    remote = [connection for row in rows for connection in row.get("remote_connections", [])]
    require(all(isinstance(connection, dict) for connection in remote), "PLSc-bootstrap process-tree remote-connection descriptors are malformed")
    egress_observed = bool(remote)
    return {
        "passed": True,
        "observation_kind": "sampled_exact_process_tree_tcp_v1",
        "sample_count": len(rows),
        "root_present_every_sample": True,
        "platform_background_egress_observed": egress_observed,
        "commercial_zero_egress_passed": not egress_observed,
        "remote_connections": remote,
    }


def snapshot_screenshots(paths: Iterable[Path], identity: str) -> list[Path]:
    destination = OUTPUT / "packaged_screenshots" / identity
    destination.mkdir(parents=True, exist_ok=True)
    copied: list[Path] = []
    for source in paths:
        row = descriptor(source)
        target = destination / f"{row['sha256'][:16]}-{source.name}"
        if target.exists():
            require(descriptor(target)["sha256"] == row["sha256"] and descriptor(target)["size"] == row["size"], f"immutable screenshot snapshot differs: {target}")
        else:
            shutil.copy2(source, target)
        copied.append(target)
    return copied


def validate_scoped_receipt(receipt_path: Path, evidence_floor: datetime) -> tuple[dict[str, Any], dict[str, Any], Path, Path, Path, list[Path]]:
    receipt = strict_json(receipt_path)
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= evidence_floor - timedelta(seconds=2) and completed >= started, "PLSc-bootstrap receipt predates current gate/build sources")
    require(
        receipt.get("schema_version") == 1
        and receipt.get("kind") == "quickpls_v247_plsc_bootstrap_scoped_native_acceptance_receipt"
        and receipt.get("scope") == "plsc_bootstrap"
        and receipt.get("feature_id") == CONTRACT.feature_id
        and receipt.get("method_version") == CONTRACT.method_version
        and receipt.get("passed") is True
        and receipt.get("failures") == 0 and receipt.get("console_errors") == 0,
        "PLSc-bootstrap receipt identity or pass state differs",
    )
    require(
        tuple(receipt.get("check_ids", ())) == EXPECTED_CHECK_IDS
        and receipt.get("checks") == len(EXPECTED_CHECK_IDS)
        and receipt.get("unique_checks") == len(EXPECTED_CHECK_IDS),
        "PLSc-bootstrap receipt check family is not exact",
    )
    require(
        receipt.get("runtime") == "tauri-webview2-cdp"
        and receipt.get("cdp_endpoint") == "http://127.0.0.1:9222"
        and receipt.get("cdp_loopback_only") is True
        and receipt.get("graceful_process_cleanup_verified") is True
        and receipt.get("forced_process_cleanup_used") is False
        and receipt.get("orphan_processes") == 0,
        "PLSc-bootstrap scoped process/runtime boundary is not clean",
    )
    receipt_rows = {
        key: receipt.get(key)
        for key in ("report", "export", "project_archive", "network_samples")
    }
    require(all(isinstance(row, dict) for row in receipt_rows.values()), "PLSc-bootstrap receipt artifact descriptors are malformed")
    report_path = ROOT / str(receipt_rows["report"].get("path", ""))
    workbook_path = ROOT / str(receipt_rows["export"].get("path", ""))
    project_path = ROOT / str(receipt_rows["project_archive"].get("path", ""))
    network_path = ROOT / str(receipt_rows["network_samples"].get("path", ""))
    for label, path, row in (
        ("report", report_path, receipt_rows["report"]),
        ("XLSX", workbook_path, receipt_rows["export"]),
        ("project", project_path, receipt_rows["project_archive"]),
        ("network samples", network_path, receipt_rows["network_samples"]),
    ):
        require(isinstance(row, dict) and descriptor(path) == row, f"PLSc-bootstrap {label} bytes differ from the receipt")
    report = strict_json(report_path)
    checks = report.get("checks")
    require(
        report.get("schema_version") == "quickpls.packaged_acceptance.v1"
        and report.get("feature_id") == CONTRACT.feature_id
        and report.get("method_version") == CONTRACT.method_version
        and report.get("acceptance_scope") == "plsc_bootstrap"
        and report.get("runtime") == "tauri-webview2-cdp"
        and report.get("passed") is True
        and report.get("failures") == [] and report.get("consoleErrors") == []
        and isinstance(checks, dict) and tuple(sorted(checks)) == EXPECTED_CHECK_IDS,
        "PLSc-bootstrap report identity, cleanliness, or checks differ",
    )
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "plsc_bootstrap" and parse_utc(focused.get("completedAt"), "focusedRun.completedAt") <= completed + timedelta(seconds=2), "PLSc-bootstrap focused report is not bounded by the receipt")
    offline = checks.get("plscBootstrapFunctionalOffline") if isinstance(checks, dict) else None
    require(
        isinstance(offline, dict)
        and offline.get("passed") is True
        and offline.get("externalRequestCount") == 0
        and offline.get("externalRequests") == []
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("strictZeroProcessEgressClaimed") is False
        and receipt.get("functional_offline") == offline,
        "PLSc-bootstrap receipt does not preserve its bounded page/application functional-offline evidence",
    )
    require(receipt.get("executable") == descriptor(ROOT / "target/release/quickpls-desktop.exe"), "PLSc-bootstrap receipt desktop differs from the frozen binary")
    require(receipt.get("cli") == descriptor(ROOT / "target/release/qpls.exe"), "PLSc-bootstrap receipt CLI differs from the frozen binary")
    process_rows = receipt.get("observed_process_tree")
    desktop_rows = [row for row in process_rows if isinstance(row, dict) and row.get("name") == "quickpls-desktop.exe"] if isinstance(process_rows, list) else []
    require(len(desktop_rows) == 1 and Path(str(desktop_rows[0].get("executable_path", ""))).resolve() == (ROOT / "target/release/quickpls-desktop.exe").resolve(), "PLSc-bootstrap receipt did not authenticate exactly one release desktop root")
    network = read_network_samples(network_path)
    require(
        receipt.get("platform_background_egress_observation") == network
        and receipt.get("sampled_process_tree_zero_egress") is network["commercial_zero_egress_passed"]
        and receipt.get("network_sample_count") == network["sample_count"],
        "PLSc-bootstrap platform-background-egress receipt differs from its sample family",
    )
    screenshots = [Path(str(value)).resolve() for value in report.get("screenshots", [])]
    screenshot_rows = [descriptor(path) for path in screenshots]
    require(len(screenshots) >= 4 and len(screenshots) == len(set(screenshots)) and receipt.get("screenshots") == screenshot_rows and report.get("screenshotArtifacts") == screenshot_rows, "PLSc-bootstrap screenshot family is not exact")
    return receipt, report, workbook_path, project_path, network_path, screenshots


def workflow_contract(report: dict[str, Any], workbook: Path, project: Path) -> dict[str, Any]:
    checks = report["checks"]
    invalid = checks["plscBootstrapInvalidSetup"]
    dialog = checks["plscBootstrapDialog"]
    cancellation = checks["plscBootstrapCancellation"]
    progress = checks["plscBootstrapProgress"]
    result = checks["plscBootstrapResult"]
    export = checks["plscBootstrapExport"]
    reopen = checks["plscBootstrapSaveReopen"]
    offline = checks["plscBootstrapFunctionalOffline"]
    run_id = result.get("runId")
    require(isinstance(run_id, str) and run_id, "PLSc-bootstrap result has no immutable run ID")
    require(
        invalid.get("attempted") is True and invalid.get("selectedMethod") == CONTRACT.selected_method
        and invalid.get("startEnabled") is False and invalid.get("emptyModelBlocker") is True
        and invalid.get("archiveStateUnchanged") is True and invalid.get("archiveAfter") == invalid.get("archiveBefore")
        and invalid.get("resultCreated") is False,
        "PLSc-bootstrap invalid setup did not fail closed",
    )
    require(
        dialog.get("selectedMethod") == CONTRACT.selected_method and dialog.get("samples") == str(PLSC_BOOTSTRAP_SAMPLES)
        and dialog.get("seed") == str(PLSC_BOOTSTRAP_SEED) and dialog.get("workers") == "2"
        and dialog.get("startEnabled") is True and dialog.get("blockers") == [] and dialog.get("standardSurface") is True,
        "PLSc-bootstrap valid setup differs from its Standard contract",
    )
    cancelled_settings = cancellation.get("cancelledSettings")
    retry_settings = cancellation.get("retrySettings")
    require(
        cancellation.get("passed") is True and cancellation.get("activeState", {}).get("captured") is True
        and cancellation.get("noPartialVisibleResult") is True and cancellation.get("noPartialCommittedResult") is True
        and cancellation.get("archiveStateUnchanged") is True and cancellation.get("archiveAfter") == cancellation.get("archiveBefore")
        and cancellation.get("completedRetryRunId") == run_id and cancellation.get("retryEnabled") is True
        and isinstance(cancelled_settings, dict) and isinstance(retry_settings, dict)
        and retry_settings.get("samples") == cancelled_settings.get("samples") == str(PLSC_BOOTSTRAP_SAMPLES)
        and retry_settings.get("seed") == cancelled_settings.get("seed") == str(PLSC_BOOTSTRAP_SEED),
        "PLSc-bootstrap cancellation/retry contract differs",
    )
    require(progress.get("captured") is True and progress.get("completedRunProof") == {"matched": True, "runId": run_id}, "PLSc-bootstrap lifecycle is not bound to the completed run")
    requested, attempted, usable, failed = (result.get(key) for key in ("requested", "attempted", "usable", "failed"))
    require(
        requested == attempted == PLSC_BOOTSTRAP_SAMPLES and isinstance(usable, int) and isinstance(failed, int)
        and usable + failed == requested and result.get("successfulWitnesses") == usable
        and result.get("successfulJackknife") + result.get("failedJackknife") == result.get("jackknife")
        and result.get("accountingRows", 0) >= 12 and result.get("percentileRows", 0) > 0
        and result.get("bcaRows", 0) > 0
        and result.get("methodVersion") == CONTRACT.method_version
        and result.get("failureDisclosure", {}).get("rows") == failed
        and result.get("failureDisclosure", {}).get("typed") is True
        and result.get("jackknifeFailureDisclosure", {}).get("rows") == result.get("failedJackknife")
        and result.get("jackknifeFailureDisclosure", {}).get("typed") is True,
        "PLSc-bootstrap result accounting, witnesses, intervals, or failures differ",
    )
    native = export.get("nativeXlsx")
    completion = native.get("helper", {}).get("completion", {}) if isinstance(native, dict) else {}
    workbook_row = completion.get("workbook", {})
    workbook_descriptor = descriptor(workbook)
    sheets = xlsx_sheet_names(workbook)
    require(
        export.get("selectedRunId") == run_id and export.get("expectedRunId") == run_id
        and isinstance(native, dict) and native.get("attempted") is True and native.get("file", {}).get("isFile") is True
        and Path(str(native.get("targetPath", ""))).resolve() == workbook.resolve()
        and completion.get("passed") is True and workbook_row.get("size") == workbook_descriptor["size"]
        and workbook_row.get("sha256") == workbook_descriptor["sha256"] and REQUIRED_SHEETS <= set(sheets),
        "PLSc-bootstrap XLSX is not a verified export of the selected immutable run",
    )
    require(
        reopen.get("sameRunRestored") is True and reopen.get("expectedRunId") == run_id
        and reopen.get("selectedRunId") == run_id and reopen.get("witnessRowsRestored") is True
        and reopen.get("archiveBeforeReopen", {}).get("immutableRunChecksum") == reopen.get("archiveAfterReopen", {}).get("immutableRunChecksum"),
        "PLSc-bootstrap save/reopen did not restore the exact immutable run",
    )
    require(Path(str(checks["plscBootstrapFixture"]["projectPath"])).resolve() == project.resolve(), "PLSc-bootstrap project path differs from its receipt")
    require(
        offline.get("passed") is True and offline.get("externalRequestCount") == 0
        and offline.get("externalRequests") == []
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("strictZeroProcessEgressClaimed") is False,
        "PLSc-bootstrap functional-offline boundary failed or was overstated",
    )
    viewport, viewport_paths = packaged_viewport_contract(CONTRACT, report, run_id)
    return {
        "passed": True,
        "run_id": run_id,
        "invalid_setup": {"passed": True, "archive_state_unchanged": True},
        "cancellation_retry": {"passed": True, "same_plan_retried": True, "completed_retry_run_id": run_id},
        "accounting": {"requested": requested, "attempted": attempted, "usable": usable, "failed": failed},
        "witnesses": {"successful": result["successfulWitnesses"], "successful_delete_one": result["successfulJackknife"], "failed_delete_one": result["failedJackknife"]},
        "selected_run_xlsx": True,
        "same_run_reopened": True,
        "functional_offline": True,
        "packaged_viewports": viewport,
        "viewport_screenshots": [repository_path(path) for path in viewport_paths],
    }


def run_adapter(receipt_path: Path, not_before: datetime) -> dict[str, Any]:
    prior = verify_prior_factory_stages(CONTRACT)
    freshness, freshness_sources = source_freshness()
    build_finished = parse_utc(freshness.get("build_finished_at_utc"), "build_finished_at_utc")
    gate_sources = [WRAPPER, HARNESS, Path(__file__).resolve(), MANIFEST]
    gate_changed = max(datetime.fromtimestamp(path.stat().st_mtime, timezone.utc) for path in gate_sources)
    evidence_floor = max(not_before, build_finished, gate_changed)
    receipt, report, workbook, project, network, screenshots = validate_scoped_receipt(receipt_path, evidence_floor)
    workflow = workflow_contract(report, workbook, project)
    archive = exact_archive_contract(project, workflow["run_id"])

    OUTPUT.mkdir(parents=True, exist_ok=True)
    stable_report = stable_path("consistent_bootstrap_v1_packaged_raw.json")
    stable_receipt = stable_path("consistent_bootstrap_v1_packaged_receipt.json")
    stable_workbook = stable_path("consistent_bootstrap_v1_packaged.xlsx")
    stable_project = stable_path("consistent_bootstrap_v1_packaged.qpls")
    stable_network = stable_path("consistent_bootstrap_v1_network_samples.jsonl")
    stable_rows = {
        "report": copy_exact(ROOT / receipt["report"]["path"], stable_report),
        "receipt": copy_exact(receipt_path, stable_receipt),
        "workbook": copy_exact(workbook, stable_workbook),
        "project": copy_exact(project, stable_project),
        "network": copy_exact(network, stable_network),
    }
    stable_screenshots = snapshot_screenshots(screenshots, receipt["report"]["sha256"])
    checks = {
        "passed": True,
        "prior_factory": prior["derived_state"],
        "source_freshness": freshness,
        "scoped_receipt": {
            "passed": True,
            "exact_required_checks": True,
            "required_check_count": len(EXPECTED_CHECK_IDS),
            "cleanup_verified": True,
            "forced_cleanup_used": False,
            "sampled_process_tree_zero_egress": receipt["sampled_process_tree_zero_egress"],
        },
        "method_functional_offline": receipt["functional_offline"],
        "platform_background_egress_observation": receipt["platform_background_egress_observation"],
        "workflow": workflow,
        "archive": archive,
        "xlsx": {"passed": True, "sheets": xlsx_sheet_names(stable_workbook), "same_run": workflow["run_id"]},
        "stable_evidence": {**stable_rows, "screenshots": [descriptor(path) for path in stable_screenshots]},
    }
    extras = [
        *freshness_sources,
        stable_report, stable_receipt, stable_workbook, stable_project, stable_network,
        *stable_screenshots,
    ]
    identity = write_identity(CONTRACT, "packaged_acceptance", checks, role_sources(CONTRACT, "packaged_acceptance", extras))
    audit = subprocess.run(
        [sys.executable, CONTRACT.audit_script], cwd=ROOT, capture_output=True,
        text=True, check=False, timeout=600,
    )
    require(audit.returncode == 0, "PLSc-bootstrap final method audit failed: " + (audit.stdout + "\n" + audit.stderr)[-3000:])
    final = validate_manifest(MANIFEST, ROOT)
    require(final.get("passed") is True and final.get("derived_state") == "release_qualified", f"PLSc-bootstrap manifest did not derive release-qualified: {final.get('errors')}")
    return {
        "passed": True,
        "derived_state": final["derived_state"],
        "packaged_identity": descriptor(identity),
        "method_audit_identity": descriptor(output_path(CONTRACT, "method_audit")),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument("--not-before-utc")
    parser.add_argument("--receipt")
    args = parser.parse_args()
    if args.skip_run and not args.not_before_utc:
        parser.error("--skip-run requires --not-before-utc")
    started = parse_utc(args.not_before_utc, "--not-before-utc") if args.not_before_utc else datetime.now(timezone.utc)
    if args.receipt:
        receipt_path = Path(args.receipt)
        if not receipt_path.is_absolute():
            receipt_path = ROOT / receipt_path
    else:
        stamp = datetime.now(timezone.utc).strftime("%Y%m%d-%H%M%S%f")
        receipt_path = ROOT / f"validation/results/v247_plsc_bootstrap_scoped_native_acceptance_receipt_{stamp}.json"
    try:
        if not args.skip_run:
            completed = subprocess.run(
                ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", repository_path(WRAPPER), "-ReceiptPath", str(receipt_path)],
                cwd=ROOT, capture_output=True, text=True, check=False, timeout=3_600,
            )
            require(completed.returncode == 0, "PLSc-bootstrap scoped supervisor failed: " + (completed.stdout + "\n" + completed.stderr)[-3000:])
        report = run_adapter(receipt_path.resolve(), started)
    except (AdapterError, OSError, UnicodeError, KeyError, TypeError, ValueError, zipfile.BadZipFile, ElementTree.ParseError, subprocess.SubprocessError) as error:
        print(json.dumps({"passed": False, "method": CONTRACT.slug, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
