#!/usr/bin/env python3
"""Receipt-bound release adapter for the exact Standard two-stage HOC workflow."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

try:
    from validation.method_promotion_manifest import validate_manifest
    from validation.phase2_release_packaged_common import (
        AdapterError,
        METHODS,
        ROOT,
        descriptor,
        nested_strings,
        output_path,
        packaged_viewport_contract,
        parse_utc,
        read_archive,
        require,
        role_sources,
        screenshot_contract,
        source_freshness,
        stable_archive_path,
        stable_workbook_path,
        strict_json,
        verify_prior_factory_stages,
        write_identity,
    )
except ModuleNotFoundError:
    from method_promotion_manifest import validate_manifest
    from phase2_release_packaged_common import (
        AdapterError,
        METHODS,
        ROOT,
        descriptor,
        nested_strings,
        output_path,
        packaged_viewport_contract,
        parse_utc,
        read_archive,
        require,
        role_sources,
        screenshot_contract,
        source_freshness,
        stable_archive_path,
        stable_workbook_path,
        strict_json,
        verify_prior_factory_stages,
        write_identity,
    )


CONTRACT = METHODS["higher_order_v1"]
RECEIPT = ROOT / "validation/results/v247_hoc_scoped_native_acceptance_receipt_v1.json"
REPORT = ROOT / "validation/results/v247_tauri_native_acceptance_hoc.json"
HOC_CHECK_IDS = (
    "hocCalculation",
    "hocCompletedRunSaved",
    "hocDialog",
    "hocExport",
    "hocFixture",
    "hocFixtureProvisioning",
    "hocFunctionalOffline",
    "hocInitialModel",
    "hocInvalidSetup",
    "hocModel",
    "hocPackagedViewports",
    "hocProgress",
    "hocResult",
    "hocSaveReopen",
    "recentProjectsRestored",
    "runtime",
    "runtimePreflight",
)


def validate_scoped_receipt(
    receipt: dict[str, Any],
    report: dict[str, Any],
    evidence_floor: datetime,
) -> tuple[dict[str, Any], Path, Path]:
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= evidence_floor - timedelta(seconds=2), "focused HOC receipt predates the current source/build floor")
    require(completed >= started, "focused HOC receipt completion predates its start")
    require(
        receipt.get("schema_version") == 1
        and receipt.get("kind") == "quickpls_v247_hoc_scoped_native_acceptance_receipt"
        and receipt.get("scope") == "hoc"
        and receipt.get("feature_id") == CONTRACT.feature_id
        and receipt.get("method_version") == CONTRACT.method_version,
        "focused HOC receipt identity is wrong",
    )
    require(
        receipt.get("passed") is True
        and receipt.get("failures") == 0
        and receipt.get("console_errors") == 0
        and receipt.get("checks") == len(HOC_CHECK_IDS)
        and receipt.get("unique_checks") == len(HOC_CHECK_IDS)
        and tuple(receipt.get("check_ids", ())) == HOC_CHECK_IDS,
        "focused HOC receipt is not a clean exact-check receipt",
    )
    require(
        receipt.get("runtime") == "tauri-webview2-cdp"
        and receipt.get("cdp_endpoint") == "http://127.0.0.1:9222"
        and receipt.get("cdp_loopback_only") is True
        and receipt.get("graceful_process_cleanup_verified") is True
        and receipt.get("forced_process_cleanup_used") is False
        and receipt.get("orphan_processes") == 0,
        "focused HOC process/runtime boundary is not clean",
    )
    report_row = receipt.get("report")
    require(isinstance(report_row, dict) and descriptor(REPORT) == report_row, "focused HOC report bytes differ from its receipt")
    require(
        report.get("runtime") == "tauri-webview2-cdp"
        and report.get("passed") is True
        and report.get("failures") == []
        and report.get("consoleErrors") == [],
        "focused HOC report did not pass cleanly",
    )
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "hoc", "focused HOC report scope is wrong")
    require(parse_utc(focused.get("completedAt"), "focusedRun.completedAt") <= completed + timedelta(seconds=2), "focused HOC report completion falls outside its receipt")
    checks = report.get("checks")
    require(isinstance(checks, dict) and tuple(sorted(checks)) == HOC_CHECK_IDS, "focused HOC report check IDs differ from the frozen scoped contract")
    offline = checks.get("hocFunctionalOffline")
    require(
        isinstance(offline, dict)
        and offline == receipt.get("functional_offline")
        and offline.get("passed") is True
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("externalRequestCount") == 0
        and offline.get("strictZeroProcessEgressClaimed") is False,
        "focused HOC functional-offline receipt is missing or overstated",
    )
    desktop = ROOT / "target/release/quickpls-desktop.exe"
    require(receipt.get("executable") == descriptor(desktop), "focused HOC executable differs from its receipt")
    process_rows = receipt.get("observed_process_tree")
    desktop_rows = [row for row in process_rows if isinstance(row, dict) and row.get("name") == "quickpls-desktop.exe"] if isinstance(process_rows, list) else []
    require(len(desktop_rows) == 1 and Path(str(desktop_rows[0].get("executable_path", ""))).resolve() == desktop.resolve(), "focused HOC receipt did not authenticate exactly one release desktop process")
    export_row = receipt.get("export")
    project_row = receipt.get("project_archive")
    require(isinstance(export_row, dict) and isinstance(project_row, dict), "focused HOC receipt lacks export/archive descriptors")
    workbook = ROOT / str(export_row.get("path", ""))
    project = ROOT / str(project_row.get("path", ""))
    require(descriptor(workbook) == export_row and descriptor(project) == project_row, "focused HOC export/archive bytes differ from the receipt")
    require(Path(str(checks["hocExport"]["nativeXlsx"]["targetPath"])).resolve() == workbook.resolve(), "focused HOC report export differs from its receipt")
    require(Path(str(checks["hocFixtureProvisioning"]["project"])).resolve() == project.resolve(), "focused HOC report archive differs from its receipt")
    screenshots = [descriptor(Path(str(path))) for path in report.get("screenshots", [])]
    require(len(screenshots) == 11 and receipt.get("screenshots") == screenshots, "focused HOC screenshot family differs from its receipt")
    require(report.get("screenshotArtifacts") == screenshots, "focused HOC report does not bind exact screenshot bytes")
    return {
        "passed": True,
        "exact_required_checks": True,
        "required_check_count": len(HOC_CHECK_IDS),
        "cleanup_verified": True,
        "functional_offline_verified": True,
        "report": report_row,
    }, project, workbook


def validate_workflow(
    report: dict[str, Any],
    receipt_workbook: Path,
) -> tuple[dict[str, Any], list[Path]]:
    checks = report["checks"]
    dialog = checks["hocDialog"]
    invalid = checks["hocInvalidSetup"]
    calculation = checks["hocCalculation"]
    result = checks["hocResult"]
    export = checks["hocExport"]
    reopen = checks["hocSaveReopen"]
    require(
        dialog.get("componentCount") == 3
        and dialog.get("capabilitySelected") is True
        and dialog.get("resourcesSelected") is True
        and dialog.get("performanceSelected") is False
        and dialog.get("createEnabled") is True
        and dialog.get("inferenceControls") == 0
        and dialog.get("noBroaderClaim") is True
        and "one HOC-to-outcome relationship" in str(dialog.get("scope"))
        and "no other structural path" in str(dialog.get("scope")),
        "focused HOC authoring selections or scope disclosure differ",
    )
    require(
        invalid.get("attempted") is True
        and invalid.get("selectedMethod") == CONTRACT.selected_method
        and invalid.get("startEnabled") is False
        and invalid.get("missingHocPathBlocked") is True
        and invalid.get("archiveAfter") == invalid.get("archiveBefore")
        and invalid.get("archiveStateUnchanged") is True
        and invalid.get("resultCreated") is False,
        "path-free HOC setup did not fail closed without state mutation",
    )
    require(
        calculation.get("selectedMethod") == CONTRACT.selected_method
        and calculation.get("startEnabled") is True
        and calculation.get("blockers") == []
        and calculation.get("bootstrapControls") == 0
        and calculation.get("permutationControls") == 0,
        "valid HOC calculation setup differs from the point-only contract",
    )
    run_id = result.get("runId")
    require(isinstance(run_id, str) and bool(run_id), "completed HOC result has no immutable run ID")
    component = result.get("component", {})
    structural = result.get("structural", {})
    scope = result.get("scope", {})
    require(
        result.get("initialSelectedTable") == "hoc_component_relationships"
        and component.get("rows") == 2
        and structural.get("rows") == 1
        and scope.get("rows") == 1
        and result.get("noTechnicalIds") is True
        and result.get("noPlaceholder") is True
        and "pls_pm_v1" in str(result.get("runDetails", {}).get("properties", {}).get("Method version", "")),
        "completed HOC result tables or identity differ",
    )
    component_text = " ".join(value for value in nested_strings(component.get("values", [])))
    structural_text = " ".join(value for value in nested_strings(structural.get("values", [])))
    scope_text = " ".join(value for value in nested_strings(scope.get("values", [])))
    require("Capability" in component_text and "Resources" in component_text and "Performance" not in component_text, "HOC result component mapping differs")
    require("Organizational Capability" in structural_text and "Performance" in structural_text, "HOC structural path differs")
    require("Reflective-reflective disjoint two-stage" in scope_text and "Point estimates only" in scope_text, "HOC result scope disclosure differs")
    native = export.get("nativeXlsx", {})
    completion = native.get("helper", {}).get("completion", {})
    workbook = completion.get("workbook", {})
    actual_workbook = descriptor(receipt_workbook)
    required_sheets = {
        "Higher-order component relation",
        "Higher-order structural paths",
        "Higher-order calculation scope",
        "Run provenance",
    }
    require(
        native.get("attempted") is True
        and native.get("file", {}).get("isFile") is True
        and completion.get("passed") is True
        and required_sheets <= set(workbook.get("sheetNames", []))
        and workbook.get("size") == actual_workbook["size"]
        and workbook.get("sha256") == actual_workbook["sha256"],
        "HOC XLSX export/readback is incomplete or unbound",
    )
    require(
        reopen.get("sameRunRestored") is True
        and reopen.get("expectedRunId") == run_id
        and reopen.get("selectedRunId") == run_id
        and reopen.get("componentRows") == 2
        and reopen.get("structuralRows") == 1
        and reopen.get("scopeRows") == 1,
        "HOC save/reopen did not restore the exact selected run",
    )
    archive = reopen.get("archive", {})
    require(
        archive.get("payloadKind") == "pls_pm_v1"
        and archive.get("recipeMethod") == "pls_pm"
        and archive.get("weightingScheme") == "path"
        and archive.get("preprocessing") == "standardized"
        and archive.get("missingData") == "listwise_deletion"
        and archive.get("bootstrapSamples") == 0
        and archive.get("studentizedInnerSamples") == 0
        and archive.get("permutationSamples") == 0
        and archive.get("caseWeightColumn") is None
        and archive.get("higherOrderCount") == 1
        and archive.get("pathCount") == 1
        and archive.get("declaration", {}).get("method") == "two_stage",
        "reopened HOC archive summary is outside the Standard scope",
    )
    screenshots, screenshot_paths = screenshot_contract(CONTRACT, report)
    viewports, viewport_paths = packaged_viewport_contract(CONTRACT, report, run_id)
    progress = checks["hocProgress"]
    require(progress.get("captured") is True or progress.get("completedBeforeCapture") is True, "HOC execution lifecycle has no honest completion evidence")
    return {
        "passed": True,
        "run_id": run_id,
        "same_run_reopened": True,
        "invalid_setup": {"passed": True, "archive_state_unchanged": True, "result_created": False},
        "cancellation_retry": {"passed": True, "required": False},
        "functional_offline": True,
        "screenshots": screenshots,
        "packaged_viewports": viewports,
    }, [*screenshot_paths, *viewport_paths]


def validate_archive(path: Path, run_id: str) -> dict[str, Any]:
    project, manifest = read_archive(path)
    results = [row for row in project.get("results", []) if isinstance(row, dict) and row.get("id") == run_id]
    require(len(results) == 1 and results[0].get("status") == "completed", "HOC archive lacks the exact completed run")
    result = results[0]
    provenance = result.get("provenance", {})
    payload = result.get("payload", {})
    require(provenance.get("method") == "pls_pm" and "pls_pm_v1" in str(provenance.get("method_version", "")), "HOC archive provenance differs")
    require(payload.get("kind") == "pls_pm_v1", "HOC archive payload identity differs")
    recipes = [row for row in project.get("recipes", []) if isinstance(row, dict) and row.get("id") == provenance.get("recipe_id")]
    require(len(recipes) == 1, "HOC archived result has no unique recipe")
    recipe = recipes[0]
    settings = recipe.get("settings", {})
    model = recipe.get("model", {})
    declarations = model.get("higher_order_constructs", [])
    require(
        recipe.get("method_config", {}).get("kind") == "pls_algorithm"
        and settings.get("method") == "pls_pm"
        and settings.get("weighting_scheme") == "path"
        and settings.get("preprocessing") == "standardized"
        and settings.get("missing_data") == "listwise_deletion"
        and settings.get("bootstrap_samples") == 0
        and settings.get("studentized_inner_samples") == 0
        and settings.get("permutation_samples") == 0
        and settings.get("case_weight_column") is None
        and model.get("controls") == []
        and model.get("interactions") == []
        and len(model.get("paths", [])) == 1
        and len(declarations) == 1
        and declarations[0].get("method") == "two_stage"
        and declarations[0].get("stage_one_recipe") is None
        and len(set(declarations[0].get("components", []))) >= 2,
        "HOC archived recipe is outside the exact Standard contract",
    )
    path_row = model["paths"][0]
    require(path_row.get("source") == declarations[0].get("id") and path_row.get("target") not in declarations[0].get("components", []), "HOC archived structural path differs")
    require(provenance.get("dataset_fingerprint") == recipe.get("dataset_fingerprint"), "HOC archive dataset fingerprint differs")
    return {
        "passed": True,
        "run_id": run_id,
        "payload_kind": "pls_pm_v1",
        "method_version": CONTRACT.method_version,
        "archive_engine_version": manifest.get("engine_version"),
        "checksums_verified": True,
    }


def run(not_before: datetime) -> dict[str, Any]:
    prior = verify_prior_factory_stages(CONTRACT)
    fresh, fresh_sources = source_freshness()
    floor = max(
        not_before,
        parse_utc(fresh.get("build_finished_at_utc"), "build_finished_at_utc"),
        datetime.fromtimestamp((ROOT / "validation/v247_tauri_native_acceptance.mjs").stat().st_mtime, timezone.utc),
    )
    receipt = strict_json(RECEIPT)
    report = strict_json(REPORT)
    scoped, source_project, source_workbook = validate_scoped_receipt(receipt, report, floor)
    packaged, screenshot_paths = validate_workflow(report, source_workbook)
    target_archive = stable_archive_path(CONTRACT)
    target_workbook = stable_workbook_path(CONTRACT)
    target_archive.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source_project, target_archive)
    shutil.copy2(source_workbook, target_workbook)
    require(descriptor(target_workbook)["sha256"] == descriptor(source_workbook)["sha256"], "stable HOC workbook copy differs")
    archive = validate_archive(target_archive, packaged["run_id"])
    checks = {
        "passed": True,
        "prior_factory": prior["derived_state"],
        "source_freshness": fresh,
        "cumulative": scoped,
        "packaged": packaged,
        "archive": archive,
        "stable_evidence": {"archive": descriptor(target_archive), "workbook": descriptor(target_workbook)},
    }
    extras = [
        *fresh_sources,
        RECEIPT,
        REPORT,
        ROOT / "validation/v247_tauri_native_acceptance.mjs",
        target_archive,
        target_workbook,
        *screenshot_paths,
    ]
    packaged_identity = write_identity(CONTRACT, "packaged_acceptance", checks, role_sources(CONTRACT, "packaged_acceptance", extras))
    audit = subprocess.run([sys.executable, CONTRACT.audit_script], cwd=ROOT, capture_output=True, text=True, check=False, timeout=600)
    require(audit.returncode == 0, "HOC independent method audit failed: " + (audit.stdout + "\n" + audit.stderr)[-3000:])
    final_fresh, final_sources = source_freshness()
    require(final_fresh == fresh and [descriptor(path) for path in final_sources] == [descriptor(path) for path in fresh_sources], "HOC product sources changed during release audit")
    final = validate_manifest(ROOT / "validation/methods/higher_order_v1.manifest.json", ROOT)
    require(final.get("passed") is True and final.get("derived_state") == "release_qualified", f"HOC final manifest did not derive release-qualified: {final.get('errors')}")
    return {
        "passed": True,
        "derived_state": final["derived_state"],
        "packaged_identity": descriptor(packaged_identity),
        "method_audit_identity": descriptor(output_path(CONTRACT, "method_audit")),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--not-before-utc", required=True)
    args = parser.parse_args()
    try:
        report = run(parse_utc(args.not_before_utc, "--not-before-utc"))
    except (AdapterError, OSError, KeyError, TypeError, ValueError, subprocess.SubprocessError) as error:
        print(json.dumps({"passed": False, "method": CONTRACT.slug, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
