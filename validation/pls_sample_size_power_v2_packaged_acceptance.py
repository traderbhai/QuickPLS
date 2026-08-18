#!/usr/bin/env python3
"""Receipt-bound packaged acceptance for prospective PLS power v2.

This adapter never builds QuickPLS. It either invokes the focused Windows
supervisor or consumes its append-only receipt, then independently binds the
exact v2 run, tail accounting, XLSX, archive, viewports, offline observation,
cancellation/retry, and clean process shutdown.
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
from typing import Any
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
    slug="pls_sample_size_power_v2",
    feature_id="qpls3.pls.sample_size_power",
    method_version="pls_sample_size_power_v2",
    catalogue_date="2026-08-12",
    selected_method="PLS-SEM Sample Size and Power",
    archive_method="pls_sample_size_power",
    receipt_role="pls_sample_size_power",
    dialog_key="plsSampleSizePowerDialog",
    result_key="plsSampleSizePowerResult",
    progress_key="plsSampleSizePowerProgress",
    export_key="plsSampleSizePowerExport",
    reopen_key="plsSampleSizePowerSaveReopen",
    fixture_key="plsSampleSizePowerFixture",
    project_field="projectPath",
    invalid_key="plsSampleSizePowerInvalidSetup",
    cancellation_key="plsSampleSizePowerCancellation",
    visual_key="plsSampleSizePower",
    visual_state="pls-sample-size-power-dialog",
    capture_prefix="pls-sample-size-power",
    captures=(),
    exact_result_values=(),
    minimum_result_values=(),
    progress_kind="captured",
    adapter_script="validation/pls_sample_size_power_v2_packaged_acceptance.py",
    audit_script="validation/pls_sample_size_power_v2_factory_audit.py",
)
OUTPUT = ROOT / "validation/results/method_factory/pls_sample_size_power_v2"
RAW_REPORT = ROOT / "validation/results/v247_tauri_native_acceptance_pls_sample_size_power.json"
WRAPPER = ROOT / "validation/run_v247_pls_sample_size_power_native_acceptance.ps1"
HARNESS = ROOT / "validation/v247_tauri_native_acceptance.mjs"
MANIFEST = ROOT / "validation/methods/pls_sample_size_power_v2.manifest.json"
EXPECTED_CHECK_IDS = (
    "plsSampleSizePowerCancellation",
    "plsSampleSizePowerDialog",
    "plsSampleSizePowerExport",
    "plsSampleSizePowerFixture",
    "plsSampleSizePowerFixtureProvisioning",
    "plsSampleSizePowerFunctionalOffline",
    "plsSampleSizePowerInitialModel",
    "plsSampleSizePowerInvalidSetup",
    "plsSampleSizePowerModel",
    "plsSampleSizePowerPackagedViewports",
    "plsSampleSizePowerProgress",
    "plsSampleSizePowerResult",
    "plsSampleSizePowerSaveReopen",
    "recentProjectsRestored",
    "runtime",
    "runtimePreflight",
)
GRID = (30, 40)
MONTE_CARLO_REPLICATES = 100
BOOTSTRAP_REPLICATES = 99
MASTER_SEED = 20_260_818
PACKAGE_VERSION = json.loads((ROOT / "package.json").read_text(encoding="utf-8"))["version"]
REQUIRED_SHEETS = {
    "Power by sample size",
    "Bootstrap tail accounting",
    "Simulation failures",
    "Design assumptions",
    "Run provenance",
}


def copy_exact(source: Path, target: Path) -> dict[str, Any]:
    require(source.is_file() and source.stat().st_size > 0, f"missing source artifact: {source}")
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target)
    source_row = descriptor(source)
    target_row = descriptor(target)
    require(source_row["size"] == target_row["size"] and source_row["sha256"] == target_row["sha256"], f"copied bytes differ: {target}")
    return descriptor(target)


def xlsx_sheet_names(path: Path) -> list[str]:
    with zipfile.ZipFile(path) as workbook:
        names = [row.filename for row in workbook.infolist()]
        require(len(names) == len(set(names)), "prospective-power XLSX contains duplicate members")
        require("xl/workbook.xml" in names, "prospective-power XLSX lacks its workbook manifest")
        root = ElementTree.fromstring(workbook.read("xl/workbook.xml"))
    namespace = "http://schemas.openxmlformats.org/spreadsheetml/2006/main"
    return [row.attrib["name"] for row in root.findall(f".//{{{namespace}}}sheet")]


def read_network_samples(path: Path) -> dict[str, Any]:
    rows = [json.loads(line) for line in path.read_text(encoding="utf-8-sig").splitlines() if line.strip()]
    require(bool(rows) and all(isinstance(row, dict) for row in rows), "prospective-power network sample family is empty or malformed")
    require(all(row.get("root_present") is True and row.get("observation") == "sampled_exact_process_tree_tcp_v1" for row in rows), "prospective-power process-tree samples are incomplete")
    require(all(isinstance(row.get("remote_connections"), list) for row in rows), "prospective-power process-tree remote-connection rows are malformed")
    remote = [connection for row in rows for connection in row.get("remote_connections", [])]
    require(all(isinstance(connection, dict) for connection in remote), "prospective-power process-tree remote-connection descriptors are malformed")
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


def exact_archive_contract(path: Path, run_id: str) -> dict[str, Any]:
    project, archive_manifest = read_archive(path)
    results = [row for row in project.get("results", []) if isinstance(row, dict) and row.get("id") == run_id]
    require(len(results) == 1, "prospective-power archive must contain exactly the reported result")
    result = results[0]
    provenance = result.get("provenance")
    payload = result.get("payload")
    require(result.get("status") == "completed" and isinstance(provenance, dict) and isinstance(payload, dict), "prospective-power archived result is incomplete")
    recipes = [row for row in project.get("recipes", []) if isinstance(row, dict) and row.get("id") == provenance.get("recipe_id")]
    runs = [row for row in project.get("layouts", {}).get("workspace", {}).get("runs", []) if isinstance(row, dict) and row.get("id") == run_id]
    require(len(recipes) == 1 and len(runs) == 1 and runs[0].get("status") == "completed", "prospective-power result must map to one completed run and recipe")
    recipe = recipes[0]
    config = recipe.get("method_config")
    model = recipe.get("model")
    analysis = payload.get("analysis")
    require(isinstance(config, dict) and isinstance(model, dict) and isinstance(analysis, dict), "prospective-power recipe, model, or analysis is missing")
    rows = analysis.get("rows")
    outcomes = analysis.get("outcomes")
    require(isinstance(rows, list) and isinstance(outcomes, list), "prospective-power rows or outcome ledger are missing")
    require(len(rows) == len(GRID) and len(outcomes) == len(GRID) * MONTE_CARLO_REPLICATES, "prospective-power planned grid/ledger count differs")
    outer_usable = 0
    outer_failed = 0
    inner_requested = 0
    inner_usable = 0
    inner_failed = 0
    inner_exceedances = 0
    for grid_index, sample_size in enumerate(GRID):
        group = outcomes[grid_index * MONTE_CARLO_REPLICATES:(grid_index + 1) * MONTE_CARLO_REPLICATES]
        require(all(isinstance(outcome, dict) for outcome in group), "prospective-power outcome ledger contains malformed rows")
        for replicate_index, outcome in enumerate(group):
            require(
                outcome.get("sample_size") == sample_size
                and outcome.get("replicate_index") == replicate_index
                and outcome.get("attempted") is True
                and isinstance(outcome.get("stream_identity"), str)
                and re.fullmatch(r"[0-9a-f]{64}", outcome["stream_identity"]) is not None,
                "prospective-power indexed outcome identity differs",
            )
            if outcome.get("successful") is True:
                requested = outcome.get("bootstrap_requested_replicates")
                usable = outcome.get("bootstrap_usable_replicates")
                failed = outcome.get("bootstrap_failed_replicates")
                exceedances = outcome.get("bootstrap_two_sided_exceedances")
                require(
                    outcome.get("converged") is True
                    and isinstance(outcome.get("target_estimate"), (int, float))
                    and math.isfinite(float(outcome["target_estimate"]))
                    and requested == BOOTSTRAP_REPLICATES
                    and all(isinstance(value, int) and not isinstance(value, bool) for value in (usable, failed, exceedances))
                    and usable + failed == requested and usable >= math.ceil(requested * 0.9)
                    and 0 <= exceedances <= usable
                    and outcome.get("p_value_two_sided") == (exceedances + 1) / (usable + 1)
                    and outcome.get("rejected") == (outcome["p_value_two_sided"] <= 0.05)
                    and outcome.get("failure_code") is None and outcome.get("failure_message") is None,
                    "prospective-power successful outcome tail accounting differs",
                )
                outer_usable += 1
                inner_requested += requested
                inner_usable += usable
                inner_failed += failed
                inner_exceedances += exceedances
            else:
                require(
                    outcome.get("successful") is False and outcome.get("converged") is False
                    and outcome.get("target_estimate") is None and outcome.get("p_value_two_sided") is None and outcome.get("rejected") is False
                    and all(outcome.get(key) is None for key in (
                        "bootstrap_requested_replicates", "bootstrap_usable_replicates",
                        "bootstrap_failed_replicates", "bootstrap_two_sided_exceedances",
                    ))
                    and isinstance(outcome.get("failure_code"), str) and bool(outcome["failure_code"].strip())
                    and isinstance(outcome.get("failure_message"), str) and bool(outcome["failure_message"].strip()),
                    "prospective-power failed outer replicate is not typed",
                )
                outer_failed += 1
        stored = rows[grid_index]
        successful = sum(outcome.get("successful") is True for outcome in group)
        rejections = sum(outcome.get("rejected") is True for outcome in group)
        require(
            isinstance(stored, dict) and stored.get("sample_size") == sample_size
            and stored.get("requested_replicates") == MONTE_CARLO_REPLICATES
            and stored.get("attempted_replicates") == MONTE_CARLO_REPLICATES
            and stored.get("successful_replicates") == successful
            and stored.get("failed_replicates") == MONTE_CARLO_REPLICATES - successful
            and stored.get("rejections") == rejections
            and abs(float(stored.get("achieved_power")) - rejections / MONTE_CARLO_REPLICATES) <= 1e-12
            and stored.get("qualifies") == (float(stored.get("confidence_lower")) >= 0.8),
            "prospective-power grid row does not reproduce from its ledger",
        )
    first_qualified = next((row["sample_size"] for row in rows if row.get("qualifies") is True), None)
    decision = analysis.get("decision")
    require(isinstance(decision, dict) and ((first_qualified is None and decision == {"status": "not_reached"}) or (first_qualified is not None and decision == {"status": "reached", "sample_size": first_qualified})), "prospective-power conservative grid decision differs")
    constructs = model.get("constructs")
    paths = model.get("paths")
    predictor_id = config.get("predictor_construct")
    outcome_id = config.get("outcome_construct")
    by_id = {row.get("id"): row for row in constructs if isinstance(row, dict)} if isinstance(constructs, list) else {}
    require(
        isinstance(constructs, list) and len(constructs) == 2 and len(by_id) == 2
        and predictor_id in by_id and outcome_id in by_id and predictor_id != outcome_id
        and all(row.get("mode") == "reflective" and isinstance(row.get("indicators"), list) and len(row["indicators"]) == 3 for row in by_id.values())
        and paths == [{"source": predictor_id, "target": outcome_id}]
        and model.get("controls") == [] and model.get("higher_order_constructs") == [] and model.get("interactions") == [],
        "prospective-power archived model exceeds the exact two-construct reflective scope",
    )
    require(
        payload.get("kind") == "pls_sample_size_power_v2"
        and archive_manifest.get("engine_version") == PACKAGE_VERSION
        and provenance.get("method") == "pls_sample_size_power"
        and provenance.get("method_version") == CONTRACT.method_version
        and recipe.get("schema_version") == 3
        and recipe.get("settings", {}).get("method") == "pls_sample_size_power"
        and recipe.get("settings", {}).get("weighting_scheme") == "path"
        and recipe.get("settings", {}).get("preprocessing") == "standardized"
        and recipe.get("settings", {}).get("tolerance") == 1e-7
        and recipe.get("settings", {}).get("max_iterations") == 3_000
        and recipe.get("settings", {}).get("seed") == MASTER_SEED
        and recipe.get("settings", {}).get("workers") == 2
        and config.get("kind") == "pls_sample_size_power"
        and config.get("scenario_identity") == "packaged_two_construct_path_v2"
        and config.get("predictor_indicator_loadings") == [0.8, 0.8, 0.8]
        and config.get("outcome_indicator_loadings") == [0.8, 0.8, 0.8]
        and config.get("population_path") == 0.3
        and config.get("exogenous_distribution") == "standard_normal"
        and config.get("structural_disturbance_distribution") == "standard_normal"
        and config.get("indicator_error_distribution") == "standard_normal"
        and config.get("missing_data") == "none"
        and config.get("inference") == "case_bootstrap_null_centered_two_sided_plus_one"
        and config.get("sample_size_grid") == list(GRID)
        and config.get("alpha") == 0.05 and config.get("target_power") == 0.8
        and config.get("interval_confidence_level") == 0.95
        and config.get("monte_carlo_replicates") == MONTE_CARLO_REPLICATES
        and config.get("bootstrap_replicates") == BOOTSTRAP_REPLICATES
        and analysis.get("schema_version") == 2
        and analysis.get("capability_id") == CONTRACT.feature_id
        and analysis.get("method_version") == CONTRACT.method_version
        and analysis.get("stream_domain") == "quickpls/pls_sample_size_power_v2/monte_carlo"
        and analysis.get("failure_policy") == "failed_replicates_count_as_non_rejections_v1"
        and analysis.get("interval_method") == "wilson_score_two_sided_v1"
        and analysis.get("inference_method") == "pls_pm_case_bootstrap_null_centered_two_sided_plus_one_v2"
        and analysis.get("pls_method_version") == "pls_pm_v1"
        and analysis.get("resampling_method_version") == "indexed_resampling_v4"
        and analysis.get("workload") == {
            "grid_points": 2,
            "planned_datasets": 200,
            "estimated_pls_fits": 20_000,
            "estimated_pls_case_fits": 700_000,
        }
        and isinstance(analysis.get("warnings"), list) and bool(analysis["warnings"])
        and isinstance(analysis.get("exclusions"), list) and bool(analysis["exclusions"])
        and provenance.get("dataset_fingerprint") == recipe.get("dataset_fingerprint"),
        "prospective-power archive identity or immutable recipe differs",
    )
    return {
        "passed": True,
        "run_id": run_id,
        "recipe_id": recipe.get("id"),
        "payload_kind": payload.get("kind"),
        "method_version": analysis.get("method_version"),
        "grid": list(GRID),
        "outer_requested": len(outcomes),
        "outer_attempted": len(outcomes),
        "outer_usable": outer_usable,
        "outer_failed": outer_failed,
        "inner_requested": inner_requested,
        "inner_usable": inner_usable,
        "inner_failed": inner_failed,
        "inner_exceedances": inner_exceedances,
        "archive_engine_version": archive_manifest.get("engine_version"),
        "checksums_verified": True,
    }


def validate_scoped_receipt(receipt_path: Path, evidence_floor: datetime) -> tuple[dict[str, Any], dict[str, Any], Path, Path, Path, list[Path]]:
    receipt = strict_json(receipt_path)
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= evidence_floor - timedelta(seconds=2) and completed >= started, "prospective-power receipt predates current sources/build")
    require(
        receipt.get("schema_version") == 1
        and receipt.get("kind") == "quickpls_v247_pls_sample_size_power_v2_scoped_native_acceptance_receipt"
        and receipt.get("scope") == "pls_sample_size_power"
        and receipt.get("feature_id") == CONTRACT.feature_id
        and receipt.get("method_version") == CONTRACT.method_version
        and receipt.get("passed") is True
        and receipt.get("failures") == 0 and receipt.get("console_errors") == 0,
        "prospective-power receipt identity or pass state differs",
    )
    require(tuple(receipt.get("check_ids", ())) == EXPECTED_CHECK_IDS and receipt.get("checks") == len(EXPECTED_CHECK_IDS) and receipt.get("unique_checks") == len(EXPECTED_CHECK_IDS), "prospective-power receipt check family is not exact")
    require(
        receipt.get("runtime") == "tauri-webview2-cdp" and receipt.get("cdp_endpoint") == "http://127.0.0.1:9222"
        and receipt.get("cdp_loopback_only") is True and receipt.get("graceful_process_cleanup_verified") is True
        and receipt.get("forced_process_cleanup_used") is False and receipt.get("orphan_processes") == 0,
        "prospective-power process/runtime boundary is not clean",
    )
    artifacts = {key: receipt.get(key) for key in ("report", "export", "project_archive", "network_samples")}
    require(all(isinstance(row, dict) for row in artifacts.values()), "prospective-power receipt artifact descriptors are malformed")
    report_path = ROOT / artifacts["report"]["path"]
    workbook_path = ROOT / artifacts["export"]["path"]
    project_path = ROOT / artifacts["project_archive"]["path"]
    network_path = ROOT / artifacts["network_samples"]["path"]
    for label, path, row in (("report", report_path, artifacts["report"]), ("XLSX", workbook_path, artifacts["export"]), ("project", project_path, artifacts["project_archive"]), ("network", network_path, artifacts["network_samples"])):
        require(descriptor(path) == row, f"prospective-power {label} bytes differ from receipt")
    report = strict_json(report_path)
    checks = report.get("checks")
    require(
        report.get("schema_version") == "quickpls.packaged_acceptance.v1"
        and report.get("feature_id") == CONTRACT.feature_id and report.get("method_version") == CONTRACT.method_version
        and report.get("acceptance_scope") == "pls_sample_size_power" and report.get("runtime") == "tauri-webview2-cdp"
        and report.get("passed") is True and report.get("failures") == [] and report.get("consoleErrors") == []
        and isinstance(checks, dict) and tuple(sorted(checks)) == EXPECTED_CHECK_IDS,
        "prospective-power report identity, cleanliness, or checks differ",
    )
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "pls_sample_size_power" and parse_utc(focused.get("completedAt"), "focusedRun.completedAt") <= completed + timedelta(seconds=2), "prospective-power focused report is not bounded by receipt")
    offline = checks.get("plsSampleSizePowerFunctionalOffline") if isinstance(checks, dict) else None
    require(
        isinstance(offline, dict)
        and offline.get("passed") is True
        and offline.get("externalRequestCount") == 0
        and offline.get("externalRequests") == []
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("strictZeroProcessEgressClaimed") is False
        and receipt.get("functional_offline") == offline,
        "prospective-power receipt does not preserve its bounded page/application functional-offline evidence",
    )
    require(receipt.get("executable") == descriptor(ROOT / "target/release/quickpls-desktop.exe"), "prospective-power desktop differs from frozen binary")
    require(receipt.get("cli") == descriptor(ROOT / "target/release/qpls.exe"), "prospective-power CLI differs from frozen binary")
    process_rows = receipt.get("observed_process_tree")
    desktop_rows = [row for row in process_rows if isinstance(row, dict) and row.get("name") == "quickpls-desktop.exe"] if isinstance(process_rows, list) else []
    require(
        len(desktop_rows) == 1
        and Path(str(desktop_rows[0].get("executable_path", ""))).resolve() == (ROOT / "target/release/quickpls-desktop.exe").resolve(),
        "prospective-power receipt did not authenticate exactly one release desktop root",
    )
    network = read_network_samples(network_path)
    require(
        receipt.get("platform_background_egress_observation") == network
        and receipt.get("sampled_process_tree_zero_egress") is network["commercial_zero_egress_passed"]
        and receipt.get("network_sample_count") == network["sample_count"],
        "prospective-power platform-background-egress receipt differs from samples",
    )
    screenshots = [Path(str(value)).resolve() for value in report.get("screenshots", [])]
    screenshot_rows = [descriptor(path) for path in screenshots]
    require(len(screenshots) >= 5 and len(screenshots) == len(set(screenshots)) and receipt.get("screenshots") == screenshot_rows and report.get("screenshotArtifacts") == screenshot_rows, "prospective-power screenshot family is not exact")
    return receipt, report, workbook_path, project_path, network_path, screenshots


def workflow_contract(report: dict[str, Any], workbook: Path, project: Path) -> dict[str, Any]:
    checks = report["checks"]
    invalid = checks["plsSampleSizePowerInvalidSetup"]
    dialog = checks["plsSampleSizePowerDialog"]
    cancellation = checks["plsSampleSizePowerCancellation"]
    progress = checks["plsSampleSizePowerProgress"]
    result = checks["plsSampleSizePowerResult"]
    export = checks["plsSampleSizePowerExport"]
    reopen = checks["plsSampleSizePowerSaveReopen"]
    offline = checks["plsSampleSizePowerFunctionalOffline"]
    run_id = result.get("runId")
    require(isinstance(run_id, str) and run_id, "prospective-power result lacks an immutable run ID")
    require(invalid.get("attempted") is True and invalid.get("selectedMethod") == CONTRACT.selected_method and invalid.get("startEnabled") is False and invalid.get("modelShapeBlocker") is True and invalid.get("archiveStateUnchanged") is True and invalid.get("archiveAfter") == invalid.get("archiveBefore") and invalid.get("resultCreated") is False, "prospective-power invalid setup did not fail closed")
    require(
        dialog.get("selectedMethod") == CONTRACT.selected_method and dialog.get("grid") == "30,40"
        and dialog.get("monteCarloReplicates") == "100" and dialog.get("bootstrapReplicates") == "99"
        and dialog.get("seed") == str(MASTER_SEED) and dialog.get("workers") == "2"
        and dialog.get("startEnabled") is True and dialog.get("blockers") == [] and dialog.get("standardSurface") is True,
        "prospective-power valid setup differs",
    )
    cancelled = cancellation.get("cancelledSettings", {})
    retry = cancellation.get("retrySettings", {})
    require(
        cancellation.get("passed") is True and cancellation.get("activeState", {}).get("captured") is True
        and cancellation.get("noPartialVisibleResult") is True and cancellation.get("noPartialCommittedResult") is True
        and cancellation.get("archiveStateUnchanged") is True and cancellation.get("archiveAfter") == cancellation.get("archiveBefore")
        and cancellation.get("completedRetryRunId") == run_id and cancellation.get("retryEnabled") is True
        and retry.get("grid") == cancelled.get("grid") == "30,40"
        and retry.get("monteCarloReplicates") == cancelled.get("monteCarloReplicates") == "100"
        and retry.get("bootstrapReplicates") == cancelled.get("bootstrapReplicates") == "99"
        and retry.get("seed") == cancelled.get("seed") == str(MASTER_SEED),
        "prospective-power cancellation/retry differs",
    )
    require(progress.get("captured") is True and progress.get("completedRunProof") == {"matched": True, "runId": run_id}, "prospective-power lifecycle is not bound to its run")
    require(
        result.get("methodVersion") == CONTRACT.method_version and result.get("initialSelectedTable") == "pls_power_by_sample_size"
        and result.get("powerRows") == 2 and result.get("tailRows") == 2
        and result.get("outerAccountingCloses") is True and result.get("tailAccountingCloses") is True
        and result.get("typedFailures") is True and result.get("provenanceMethodVersionPresent") is True,
        "prospective-power grid/tail/failure/provenance result differs",
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
        "prospective-power XLSX is not the selected immutable run export",
    )
    require(
        reopen.get("sameRunRestored") is True and reopen.get("expectedRunId") == run_id and reopen.get("selectedRunId") == run_id
        and reopen.get("reopenedPowerRows") == 2 and reopen.get("reopenedTailRows") == 2
        and reopen.get("archiveBeforeReopen", {}).get("immutableRunChecksum") == reopen.get("archiveAfterReopen", {}).get("immutableRunChecksum"),
        "prospective-power save/reopen did not restore the exact run",
    )
    require(Path(str(checks["plsSampleSizePowerFixture"]["projectPath"])).resolve() == project.resolve(), "prospective-power project path differs from receipt")
    require(offline.get("passed") is True and offline.get("externalRequestCount") == 0 and offline.get("externalRequests") == [] and offline.get("analyticalWorkflowRequiresInternet") is False and offline.get("strictZeroProcessEgressClaimed") is False, "prospective-power functional-offline boundary differs")
    viewports, viewport_paths = packaged_viewport_contract(CONTRACT, report, run_id)
    return {
        "passed": True,
        "run_id": run_id,
        "invalid_setup": {"passed": True, "archive_state_unchanged": True},
        "cancellation_retry": {"passed": True, "same_plan_retried": True, "completed_retry_run_id": run_id},
        "tail_accounting": {"outer_rows": 2, "tail_rows": 2, "outer_closed": True, "inner_closed": True},
        "selected_run_xlsx": True,
        "same_run_reopened": True,
        "functional_offline": True,
        "packaged_viewports": viewports,
        "viewport_screenshots": [repository_path(path) for path in viewport_paths],
    }


def run_adapter(receipt_path: Path, not_before: datetime) -> dict[str, Any]:
    prior = verify_prior_factory_stages(CONTRACT)
    freshness, freshness_sources = source_freshness()
    build_finished = parse_utc(freshness.get("build_finished_at_utc"), "build_finished_at_utc")
    gate_changed = max(datetime.fromtimestamp(path.stat().st_mtime, timezone.utc) for path in (WRAPPER, HARNESS, Path(__file__).resolve(), MANIFEST))
    evidence_floor = max(not_before, build_finished, gate_changed)
    receipt, report, workbook, project, network, screenshots = validate_scoped_receipt(receipt_path, evidence_floor)
    workflow = workflow_contract(report, workbook, project)
    archive = exact_archive_contract(project, workflow["run_id"])
    OUTPUT.mkdir(parents=True, exist_ok=True)
    stable = {
        "report": copy_exact(ROOT / receipt["report"]["path"], OUTPUT / "pls_sample_size_power_v2_packaged_raw.json"),
        "receipt": copy_exact(receipt_path, OUTPUT / "pls_sample_size_power_v2_packaged_receipt.json"),
        "workbook": copy_exact(workbook, OUTPUT / "pls_sample_size_power_v2_packaged.xlsx"),
        "project": copy_exact(project, OUTPUT / "pls_sample_size_power_v2_packaged.qpls"),
        "network": copy_exact(network, OUTPUT / "pls_sample_size_power_v2_network_samples.jsonl"),
    }
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
        "xlsx": {"passed": True, "sheets": xlsx_sheet_names(workbook), "same_run": workflow["run_id"]},
        "stable_evidence": stable,
    }
    extras = [*freshness_sources, *(OUTPUT / name for name in (
        "pls_sample_size_power_v2_packaged_raw.json", "pls_sample_size_power_v2_packaged_receipt.json",
        "pls_sample_size_power_v2_packaged.xlsx", "pls_sample_size_power_v2_packaged.qpls",
        "pls_sample_size_power_v2_network_samples.jsonl",
    )), *screenshots]
    identity = write_identity(CONTRACT, "packaged_acceptance", checks, role_sources(CONTRACT, "packaged_acceptance", extras))
    audit = subprocess.run([sys.executable, CONTRACT.audit_script], cwd=ROOT, capture_output=True, text=True, check=False, timeout=600)
    require(audit.returncode == 0, "prospective-power final audit failed: " + (audit.stdout + "\n" + audit.stderr)[-3000:])
    final = validate_manifest(MANIFEST, ROOT)
    require(final.get("passed") is True and final.get("derived_state") == "release_qualified", f"prospective-power manifest did not derive release-qualified: {final.get('errors')}")
    return {"passed": True, "derived_state": final["derived_state"], "packaged_identity": descriptor(identity), "method_audit_identity": descriptor(output_path(CONTRACT, "method_audit"))}


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
        receipt_path = ROOT / f"validation/results/v247_pls_sample_size_power_v2_scoped_receipt_{stamp}.json"
    try:
        if not args.skip_run:
            completed = subprocess.run(
                ["powershell", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", repository_path(WRAPPER), "-ReceiptPath", str(receipt_path)],
                cwd=ROOT, capture_output=True, text=True, check=False, timeout=3_600,
            )
            require(completed.returncode == 0, "prospective-power supervisor failed: " + (completed.stdout + "\n" + completed.stderr)[-3000:])
        report = run_adapter(receipt_path.resolve(), started)
    except (AdapterError, OSError, UnicodeError, KeyError, TypeError, ValueError, zipfile.BadZipFile, ElementTree.ParseError, subprocess.SubprocessError) as error:
        print(json.dumps({"passed": False, "method": CONTRACT.slug, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
