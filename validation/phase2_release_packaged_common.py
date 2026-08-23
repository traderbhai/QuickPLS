#!/usr/bin/env python3
"""Receipt-bound release adapters for focused method promotions.

This module never launches QuickPLS.  It converts one already-completed,
source-current cumulative Tauri run, or a separately supervised focused GSCA/NCA
run, into method-scoped release identities only
when the manifest-defined cumulative report, the exact same-run XLSX/archive/reopen proof,
invalid-setup state invariants, required cancellation/retry behavior, cleanup
receipt, and the method-bound three-viewport *actual packaged window* matrix
all verify fail closed. Browser-preview or viewport-emulation evidence cannot
satisfy the packaged viewport contract.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
import sys
import zipfile
from dataclasses import dataclass
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Iterable

try:
    from validation.diagnostic_bundle_source_manifest import (
        SourceManifestFailure,
        validate_build_receipt,
    )
    from validation.method_promotion_manifest import (
        _verify_artifact,
        validate_manifest,
    )
    from validation.packaged_windows_acceptance_v2 import (
        CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
        DEFAULT_CONTRACT_PATH as PACKAGED_ACCEPTANCE_CONTRACT_PATH,
        EXPECTED_CHECK_COUNT,
        EXPECTED_CHECK_IDS,
        packaged_acceptance_contract_descriptor,
        validate_required_report_checks,
    )
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.diagnostic_bundle_source_manifest import (  # type: ignore[no-redef]
        SourceManifestFailure,
        validate_build_receipt,
    )
    from validation.method_promotion_manifest import (  # type: ignore[no-redef]
        _verify_artifact,
        validate_manifest,
    )
    from validation.packaged_windows_acceptance_v2 import (  # type: ignore[no-redef]
        CONTRACT as PACKAGED_ACCEPTANCE_CONTRACT,
        DEFAULT_CONTRACT_PATH as PACKAGED_ACCEPTANCE_CONTRACT_PATH,
        EXPECTED_CHECK_COUNT,
        EXPECTED_CHECK_IDS,
        packaged_acceptance_contract_descriptor,
        validate_required_report_checks,
    )


ROOT = Path(__file__).resolve().parents[1]
BUILD_RECEIPT = ROOT / "validation/results/diagnostic_bundle_build_receipt.json"
CUMULATIVE_RECEIPT = ROOT / "validation/results/v247_cumulative_native_acceptance_receipt.json"
GSCA_SCOPED_RECEIPT = ROOT / "validation/results/v247_gsca_scoped_native_acceptance_receipt_v2.json"
NCA_SCOPED_RECEIPT = ROOT / "validation/results/v247_nca_scoped_native_acceptance_receipt_v2.json"
NATIVE_HARNESS = ROOT / "validation/v247_tauri_native_acceptance.mjs"
SCREEN_ROOT = ROOT / "validation/results/screens/v247-native-desktop-acceptance"
EXPECTED_VIEWPORTS = ("1024x700", "1280x720", "1440x900")
GSCA_SCOPED_CHECK_IDS = (
    "gscaDialog", "gscaExport", "gscaFixture", "gscaFixtureProvisioning",
    "gscaFunctionalOffline", "gscaInitialModelCreation", "gscaInvalidSetup",
    "gscaModel", "gscaPackagedViewports", "gscaProgress", "gscaResult",
    "gscaSaveReopen", "recentProjectsRestored", "runtime", "runtimePreflight",
)
NCA_SCOPED_CHECK_IDS = (
    "ncaCalculationDialog", "ncaCancellationRetry", "ncaExport",
    "ncaFixtureProvisioning", "ncaFunctionalOffline", "ncaInvalidSetup",
    "ncaPackagedViewports", "ncaReferenceFixture", "ncaResult", "ncaRunning",
    "ncaSaveReopen", "recentProjectsRestored", "runtime", "runtimePreflight",
)


class AdapterError(ValueError):
    """Raised when release evidence does not satisfy the frozen contract."""


@dataclass(frozen=True)
class MethodContract:
    slug: str
    feature_id: str
    method_version: str
    catalogue_date: str
    selected_method: str
    archive_method: str
    receipt_role: str
    dialog_key: str
    result_key: str
    progress_key: str
    export_key: str
    reopen_key: str
    fixture_key: str
    project_field: str
    invalid_key: str
    cancellation_key: str | None
    visual_key: str
    visual_state: str
    capture_prefix: str
    captures: tuple[tuple[str, str], ...]
    exact_result_values: tuple[tuple[str, Any], ...]
    minimum_result_values: tuple[tuple[str, int], ...]
    progress_kind: str
    adapter_script: str
    audit_script: str


METHODS: dict[str, MethodContract] = {
    "gsca_als_v2": MethodContract(
        slug="gsca_als_v2",
        feature_id="qpls3.gsca.als",
        method_version="gsca_als_v2",
        catalogue_date="2026-08-12",
        selected_method="GSCA",
        archive_method="gsca",
        receipt_role="gsca",
        dialog_key="gscaDialog",
        result_key="gscaResult",
        progress_key="gscaProgress",
        export_key="gscaExport",
        reopen_key="gscaSaveReopen",
        fixture_key="gscaFixture",
        project_field="projectPath",
        invalid_key="gscaInvalidSetup",
        cancellation_key=None,
        visual_key="gsca",
        visual_state="gsca-dialog",
        capture_prefix="gsca",
        captures=(("140", "fixture-data"), ("140a", "invalid-setup"), ("141", "model"), ("142", "dialog"), ("144", "results"), ("145", "export"), ("146", "reopened")),
        exact_result_values=(("/noPlaceholder", True), ("/noGenericPlsOrInference", True)),
        minimum_result_values=(("/fit/rows", 1), ("/paths/rows", 1), ("/weights/rows", 1)),
        progress_kind="captured",
        adapter_script="validation/gsca_als_v2_packaged_acceptance.py",
        audit_script="validation/gsca_als_v2_release_audit.py",
    ),
    "higher_order_v1": MethodContract(
        slug="higher_order_v1",
        feature_id="qpls3.pls.higher_order_two_stage",
        method_version="pls_pm_v1",
        catalogue_date="2026-08-12",
        selected_method="PLS-SEM Algorithm",
        archive_method="pls_pm",
        receipt_role="hoc",
        dialog_key="hocCalculation",
        result_key="hocResult",
        progress_key="hocProgress",
        export_key="hocExport",
        reopen_key="hocSaveReopen",
        fixture_key="hocFixtureProvisioning",
        project_field="project",
        invalid_key="hocInvalidSetup",
        cancellation_key=None,
        visual_key="hoc",
        visual_state="hoc-dialog",
        capture_prefix="hoc",
        captures=(("100", "fixture-data"), ("101a", "invalid-setup"), ("101", "dialog"), ("102", "model"), ("103", "running"), ("104", "results"), ("105", "export"), ("106", "reopened")),
        exact_result_values=(("/noTechnicalIds", True), ("/noPlaceholder", True)),
        minimum_result_values=(("/component/rows", 2), ("/structural/rows", 1), ("/scope/rows", 1)),
        progress_kind="captured",
        adapter_script="validation/higher_order_v1_packaged_acceptance.py",
        audit_script="validation/higher_order_method_promotion_audit.py",
    ),
    "cca_residuals_v1": MethodContract(
        slug="cca_residuals_v1",
        feature_id="qpls3.assessment.cca_residuals",
        method_version="cca_composite_residual_v1",
        catalogue_date="2026-08-12",
        selected_method="CCA composite residual diagnostics",
        archive_method="cca",
        receipt_role="cca",
        dialog_key="ccaCalculationDialog",
        result_key="ccaResult",
        progress_key="ccaRunning",
        export_key="ccaExport",
        reopen_key="ccaSaveReopen",
        fixture_key="ccaFixtureProvisioning",
        project_field="project",
        invalid_key="ccaInvalidSetup",
        cancellation_key=None,
        visual_key="cca",
        visual_state="cca-dialog",
        capture_prefix="cca",
        captures=(("70", "fixture-data"), ("70a", "invalid-setup"), ("71", "model"), ("72", "dialog"), ("73", "running"), ("74", "results"), ("75", "export"), ("76", "reopened")),
        exact_result_values=(("/archiveMaximumMatchesVisible", True), ("/navigation/finiteAndConsistent", True), ("/navigation/maximumMatchesRows", True), ("/navigation/noInventedInferenceOrClassification", True)),
        minimum_result_values=(("/navigation/summary/rowCount", 1), ("/navigation/residuals/rowCount", 1)),
        progress_kind="active",
        adapter_script="validation/cca_residuals_v1_packaged_acceptance.py",
        audit_script="validation/cca_residuals_v1_release_audit.py",
    ),
    "ipma_v1": MethodContract(
        slug="ipma_v1",
        feature_id="qpls3.assessment.ipma",
        method_version="ipma_v1",
        catalogue_date="2026-08-12",
        selected_method="Importance-Performance Map Analysis",
        archive_method="ipma",
        receipt_role="ipma",
        dialog_key="ipmaCalculationDialog",
        result_key="ipmaResult",
        progress_key="ipmaRunning",
        export_key="ipmaExport",
        reopen_key="ipmaSaveReopen",
        fixture_key="ipmaFixtureProvisioning",
        project_field="project",
        invalid_key="ipmaInvalidSetup",
        cancellation_key=None,
        visual_key="ipma",
        visual_state="ipma-dialog",
        capture_prefix="ipma",
        captures=(("77", "fixture-data"), ("77a", "invalid-setup"), ("78", "model"), ("79", "dialog"), ("80", "running"), ("81", "results"), ("82", "export"), ("83", "reopened")),
        exact_result_values=(("/navigation/constructValuesFinite", True), ("/navigation/indicatorValuesFinite", True), ("/navigation/predecessorOnly", True), ("/navigation/excludesTargetAndUnrelatedConstructRows", True), ("/navigation/excludesTargetAndUnrelatedIndicatorRows", True), ("/navigation/noPlaceholderOrUnsupportedClaims", True)),
        minimum_result_values=(("/navigation/constructs/rowCount", 1), ("/navigation/indicators/rowCount", 1)),
        progress_kind="active",
        adapter_script="validation/ipma_v1_packaged_acceptance.py",
        audit_script="validation/ipma_v1_release_audit.py",
    ),
    "cbsem_ml_v1": MethodContract(
        slug="cbsem_ml_v1",
        feature_id="qpls3.cbsem.ml",
        method_version="cbsem_ml_v1",
        catalogue_date="2026-08-12",
        selected_method="CB-SEM / CFA",
        archive_method="cbsem",
        receipt_role="cbsem",
        dialog_key="cbsemDialog",
        result_key="cbsemResult",
        progress_key="cbsemProgress",
        export_key="cbsemExport",
        reopen_key="cbsemSaveReopen",
        fixture_key="cbsemFixtureProvisioning",
        project_field="project",
        invalid_key="cbsemInvalidSetup",
        cancellation_key=None,
        visual_key="cbsem",
        visual_state="cbsem-dialog",
        capture_prefix="cbsem",
        captures=(("130", "fixture-data"), ("130a", "invalid-setup"), ("131", "model"), ("132", "dialog"), ("134", "results"), ("135", "export"), ("136", "reopened")),
        exact_result_values=(("/noPlaceholder", True), ("/noGenericPlsTables", True)),
        minimum_result_values=(("/fit/rows", 1), ("/standardized/rows", 1), ("/modificationDiagnostics/rows", 1)),
        progress_kind="captured",
        adapter_script="validation/cbsem_ml_v1_packaged_adapter.py",
        audit_script="validation/cbsem_ml_v1_factory_audit.py",
    ),
    "plspredict_cvpat_v2": MethodContract(
        slug="plspredict_cvpat_v2",
        feature_id="qpls3.prediction.plspredict_cvpat",
        method_version="plspredict_indicator_v2",
        catalogue_date="2026-08-12",
        selected_method="PLSpredict / CVPAT",
        archive_method="predict",
        receipt_role="prediction",
        dialog_key="predictionV2Dialog",
        result_key="predictionV2Result",
        progress_key="predictionV2Progress",
        export_key="predictionV2Export",
        reopen_key="predictionV2SaveReopen",
        fixture_key="predictionFixture",
        project_field="projectPath",
        invalid_key="predictionInvalidSetup",
        cancellation_key="predictionCancellationRetry",
        visual_key="prediction",
        visual_state="prediction-dialog",
        capture_prefix="prediction",
        captures=(("90", "fixture-data"), ("90a", "invalid-setup"), ("91", "model"), ("92", "dialog"), ("92a", "cancellation-running"), ("92b", "cancelled"), ("93", "running"), ("94", "indicator-results"), ("95", "cvpat-results"), ("96", "export"), ("97", "reopened")),
        exact_result_values=(("/noPlaceholderOrLegacyClaim", True),),
        minimum_result_values=(("/indicator/rows", 1), ("/cvpat/rows", 1), ("/validationPlan/rows", 1)),
        progress_kind="captured",
        adapter_script="validation/plspredict_cvpat_v2_packaged_acceptance.py",
        audit_script="validation/plspredict_cvpat_v2_release_audit.py",
    ),
    "nca_v2": MethodContract(
        slug="nca_v2",
        feature_id="qpls3.standalone.nca",
        method_version="nca_v2",
        catalogue_date="2026-08-12",
        selected_method="Necessary Condition Analysis",
        archive_method="nca",
        receipt_role="nca",
        dialog_key="ncaCalculationDialog",
        result_key="ncaResult",
        progress_key="ncaRunning",
        export_key="ncaExport",
        reopen_key="ncaSaveReopen",
        fixture_key="ncaFixtureProvisioning",
        project_field="project",
        invalid_key="ncaInvalidSetup",
        cancellation_key="ncaCancellationRetry",
        visual_key="nca",
        visual_state="nca-standalone-dialog",
        capture_prefix="nca",
        captures=(("84", "fixture-data-no-model"), ("84a", "invalid-setup"), ("85", "dialog"), ("85a", "cancelled"), ("86", "running"), ("87", "results"), ("88", "export"), ("89", "reopened")),
        exact_result_values=(("/navigation/pValueLattice", True), ("/navigation/bottlenecksMatch", True), ("/navigation/noModelOrQualityTree", True), ("/navigation/noPlaceholder", True), ("/navigation/noBroaderNcaClaim", True)),
        minimum_result_values=(("/navigation/effects/rowCount", 1), ("/navigation/bottlenecks/rowCount", 1)),
        progress_kind="active",
        adapter_script="validation/nca_v2_packaged_acceptance.py",
        audit_script="validation/nca_v2_release_audit.py",
    ),
}


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AdapterError(message)


def functional_check_passed(row: object) -> bool:
    """Accept the harness's exact scalar success sentinel or a non-red evidence object."""
    return row is True or (
        isinstance(row, dict)
        and ("passed" not in row or row.get("passed") is True)
    )


def strict_json(path: Path) -> dict[str, Any]:
    def object_hook(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        value: dict[str, Any] = {}
        for key, item in pairs:
            require(key not in value, f"duplicate JSON key in {path}: {key}")
            value[key] = item
        return value

    value = json.loads(
        path.read_text(encoding="utf-8-sig"),
        object_pairs_hook=object_hook,
        parse_constant=lambda token: (_ for _ in ()).throw(
            AdapterError(f"non-finite JSON value in {path}: {token}")
        ),
    )
    require(isinstance(value, dict), f"{path} must be a JSON object")
    return value


def repository_path(path: Path) -> str:
    resolved = path.resolve()
    require(resolved.is_relative_to(ROOT.resolve()), f"path escapes repository: {path}")
    return resolved.relative_to(ROOT.resolve()).as_posix()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def descriptor(path: Path) -> dict[str, Any]:
    require(path.is_file() and not path.is_symlink(), f"missing or linked artifact: {path}")
    size = path.stat().st_size
    require(size > 0, f"empty artifact: {path}")
    return {"path": repository_path(path), "size": size, "sha256": sha256_file(path)}


def parse_utc(value: object, label: str) -> datetime:
    require(isinstance(value, str), f"{label} must be an ISO timestamp")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    require(parsed.tzinfo is not None, f"{label} must include an offset")
    return parsed.astimezone(timezone.utc)


def pointer(document: Any, value: str) -> Any:
    current = document
    require(value.startswith("/"), f"invalid pointer: {value}")
    for part in value[1:].split("/"):
        part = part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict):
            current = current[part]
        elif isinstance(current, list) and part.isdigit():
            current = current[int(part)]
        else:
            raise AdapterError(f"missing pointer {value}")
    return current


def nested_strings(value: Any) -> Iterable[str]:
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for child in value.values():
            yield from nested_strings(child)
    elif isinstance(value, list):
        for child in value:
            yield from nested_strings(child)


def window_bounds_equal(left: Any, right: Any, tolerance_pixels: int = 0) -> bool:
    if not isinstance(left, dict) or not isinstance(right, dict):
        return False
    left_state = left.get("windowState", "normal")
    right_state = right.get("windowState", "normal")
    if left_state != right_state:
        return False
    if left_state != "normal":
        return True
    return all(
        isinstance(left.get(key), int)
        and not isinstance(left.get(key), bool)
        and isinstance(right.get(key), int)
        and not isinstance(right.get(key), bool)
        and abs(left[key] - right[key]) <= tolerance_pixels
        for key in ("left", "top", "width", "height")
    )


def manifest_path(contract: MethodContract) -> Path:
    return ROOT / f"validation/methods/{contract.slug}.manifest.json"


def output_dir(contract: MethodContract) -> Path:
    return ROOT / "validation/results/method_factory" / contract.slug


def output_path(contract: MethodContract, role: str) -> Path:
    return output_dir(contract) / f"{role}.identity.json"


def stable_archive_path(contract: MethodContract) -> Path:
    return output_dir(contract) / f"{contract.slug}_packaged.qpls"


def stable_workbook_path(contract: MethodContract) -> Path:
    return output_dir(contract) / f"{contract.slug}_packaged.xlsx"


def load_manifest(contract: MethodContract) -> dict[str, Any]:
    document = strict_json(manifest_path(contract))
    feature = document.get("feature", {})
    require(feature.get("id") == contract.feature_id, f"{contract.slug} feature identity changed")
    require(feature.get("method_version") == contract.method_version, f"{contract.slug} method version changed")
    require(feature.get("catalogue_snapshot_date") == contract.catalogue_date, f"{contract.slug} catalogue date changed")
    return document


def verify_prior_factory_stages(contract: MethodContract) -> dict[str, Any]:
    document = load_manifest(contract)
    expected = {
        "passed": True,
        "feature_id": contract.feature_id,
        "method_version": contract.method_version,
        "catalogue_snapshot_date": contract.catalogue_date,
    }
    required_roles = {
        "engine_only": {"method_spec", "independent_reference", "simulation_report", "boundary_report"},
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    rows: list[dict[str, Any]] = []
    for stage, roles in required_roles.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact.get("roles", []))
            passed, errors = _verify_artifact(artifact, document, ROOT, expected)
            rows.append({"stage": stage, "path": artifact.get("path"), "roles": artifact.get("roles"), "passed": passed, "errors": errors})
        rows.append({"stage": stage, "path": None, "roles": sorted(observed), "passed": observed == roles, "errors": [] if observed == roles else [f"expected {sorted(roles)}, found {sorted(observed)}"]})
    require(all(row["passed"] for row in rows), f"{contract.slug} prior factory stages are not current: {rows}")
    return {"passed": True, "derived_state": "native_qualified", "artifacts": rows}


def cli_source_paths() -> list[Path]:
    paths = [ROOT / "Cargo.lock", ROOT / "Cargo.toml"]
    paths.extend(sorted((ROOT / "crates").glob("*/Cargo.toml")))
    paths.extend(sorted((ROOT / "crates").glob("*/src/**/*.rs")))
    return [path for path in paths if path.is_file()]


def source_freshness() -> tuple[dict[str, Any], list[Path]]:
    receipt = strict_json(BUILD_RECEIPT)
    validate_build_receipt(receipt, ROOT)
    desktop = ROOT / "target/release/quickpls-desktop.exe"
    cli = ROOT / "target/release/qpls.exe"
    desktop_row = descriptor(desktop)
    tested = receipt.get("tested_desktop")
    require(isinstance(tested, dict), "build receipt has no tested desktop")
    require(
        desktop_row == {"path": tested.get("path"), "size": tested.get("size"), "sha256": tested.get("sha256")},
        "release desktop differs from the frozen build receipt",
    )
    cli_row = descriptor(cli)
    rust_sources = cli_source_paths()
    newer = [repository_path(path) for path in rust_sources if path.stat().st_mtime_ns > cli.stat().st_mtime_ns]
    require(not newer, "release CLI predates current Rust inputs: " + ", ".join(newer))
    cli_source_rows = [descriptor(path) for path in rust_sources]
    return {
        "passed": True,
        "desktop_receipt_exact": True,
        "release_cli_newer_sources": newer,
        "build_finished_at_utc": receipt.get("build_finished_at_utc"),
        "desktop": desktop_row,
        "cli": cli_row,
        "cli_source_closure": cli_source_rows,
    }, [BUILD_RECEIPT, desktop, cli, *rust_sources]


def validate_cumulative_receipt(
    contract: MethodContract,
    receipt: dict[str, Any],
    report: dict[str, Any],
    not_before: datetime,
) -> tuple[dict[str, Any], Path, Path]:
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= not_before - timedelta(seconds=2), "cumulative receipt predates the requested evidence floor")
    require(completed >= started, "cumulative receipt completion predates its start")
    require(receipt.get("schema_version") == 2 and receipt.get("kind") == "quickpls_v247_cumulative_native_acceptance_receipt", "cumulative receipt identity is wrong")
    require(receipt.get("passed") is True and receipt.get("failures") == 0 and receipt.get("console_errors") == 0, "cumulative receipt is not clean")
    require(receipt.get("checks") == EXPECTED_CHECK_COUNT and receipt.get("unique_checks") == EXPECTED_CHECK_COUNT, "cumulative receipt count differs from the packaged acceptance manifest")
    require(receipt.get("final_scope") == PACKAGED_ACCEPTANCE_CONTRACT["final_scope"], "cumulative final scope is wrong")
    require(receipt.get("graceful_process_cleanup_verified") is True, "cumulative cleanup is not verified")
    acceptance_contract = receipt.get("acceptance_contract")
    expected_contract_descriptor = packaged_acceptance_contract_descriptor()
    require(isinstance(acceptance_contract, dict), "cumulative receipt has no acceptance-contract descriptor")
    require(
        acceptance_contract.get("path") == "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
        and acceptance_contract.get("contract_id") == PACKAGED_ACCEPTANCE_CONTRACT["contract_id"]
        and acceptance_contract.get("contract_version") == PACKAGED_ACCEPTANCE_CONTRACT["contract_version"]
        and acceptance_contract.get("required_check_count") == EXPECTED_CHECK_COUNT
        and acceptance_contract.get("sha256") == expected_contract_descriptor["sha256"]
        and acceptance_contract.get("bundled_sample_catalog")
        == expected_contract_descriptor["bundled_sample_catalog"],
        "cumulative receipt acceptance-contract descriptor is stale or invalid",
    )
    report_path = ROOT / str(receipt.get("report", ""))
    require(repository_path(report_path) == "validation/results/v247_tauri_native_acceptance.json", "cumulative receipt points to the wrong report")
    actual_report = descriptor(report_path)
    require(actual_report["size"] == receipt.get("report_size") and actual_report["sha256"] == receipt.get("report_sha256"), "cumulative report bytes differ from receipt")
    require(report.get("runtime") == "tauri-webview2-cdp", "functional report did not run through packaged Tauri WebView2 CDP")
    require(report.get("passed") is True and report.get("failures") == [] and report.get("consoleErrors") == [], "functional packaged report did not pass cleanly")
    checks = report.get("checks")
    check_contract = validate_required_report_checks(PACKAGED_ACCEPTANCE_CONTRACT, checks)
    require(check_contract["passed"], f"functional report check IDs differ from the packaged acceptance manifest: {check_contract}")
    require(
        all(functional_check_passed(row) for row in checks.values()),
        "functional report contains a red or malformed check",
    )
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "regression_bootstrap", "functional report final focused scope is wrong")
    require(parse_utc(focused.get("completedAt"), "focusedRun.completedAt") <= completed + timedelta(seconds=2), "functional report completion falls outside the cumulative receipt")
    exports = [row for row in receipt.get("exports", []) if isinstance(row, dict) and row.get("role") == contract.receipt_role]
    require(len(exports) == 1, f"cumulative receipt must contain exactly one {contract.receipt_role} export")
    workbook = ROOT / str(exports[0].get("path", ""))
    workbook_row = descriptor(workbook)
    require(workbook_row == {"path": exports[0].get("path"), "size": exports[0].get("size"), "sha256": exports[0].get("sha256")}, f"receipt-bound {contract.receipt_role} workbook changed")
    return {
        "passed": True,
        "exact_required_checks": True,
        "required_check_count": EXPECTED_CHECK_COUNT,
        "acceptance_contract": expected_contract_descriptor,
        "started_at_utc": started.isoformat(),
        "completed_at_utc": completed.isoformat(),
        "report": actual_report,
        "export": workbook_row,
        "cleanup_verified": True,
    }, report_path, workbook


def validate_gsca_scoped_receipt(
    contract: MethodContract,
    receipt: dict[str, Any],
    report: dict[str, Any],
    not_before: datetime,
) -> tuple[dict[str, Any], Path, Path]:
    """Validate the append-only, method-scoped GSCA supervisor receipt."""

    require(contract.slug == "gsca_als_v2", "the focused GSCA receipt cannot qualify another method")
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= not_before - timedelta(seconds=2), "focused GSCA receipt predates the requested evidence floor")
    require(completed >= started, "focused GSCA receipt completion predates its start")
    require(
        receipt.get("schema_version") == 1
        and receipt.get("kind") == "quickpls_v247_gsca_scoped_native_acceptance_receipt"
        and receipt.get("scope") == "gsca"
        and receipt.get("feature_id") == contract.feature_id
        and receipt.get("method_version") == contract.method_version,
        "focused GSCA receipt identity is wrong",
    )
    require(
        receipt.get("passed") is True
        and receipt.get("failures") == 0
        and receipt.get("console_errors") == 0
        and receipt.get("checks") == len(GSCA_SCOPED_CHECK_IDS)
        and receipt.get("unique_checks") == len(GSCA_SCOPED_CHECK_IDS)
        and tuple(receipt.get("check_ids", ())) == GSCA_SCOPED_CHECK_IDS,
        "focused GSCA receipt is not a clean exact-check receipt",
    )
    require(
        receipt.get("runtime") == "tauri-webview2-cdp"
        and receipt.get("cdp_endpoint") == "http://127.0.0.1:9222"
        and receipt.get("cdp_loopback_only") is True
        and receipt.get("graceful_process_cleanup_verified") is True
        and receipt.get("forced_process_cleanup_used") is False
        and receipt.get("orphan_processes") == 0,
        "focused GSCA process/runtime boundary is not clean",
    )
    report_row = receipt.get("report")
    require(isinstance(report_row, dict), "focused GSCA receipt has no report descriptor")
    report_path = ROOT / str(report_row.get("path", ""))
    require(repository_path(report_path) == "validation/results/v247_tauri_native_acceptance_gsca.json", "focused GSCA receipt points to the wrong report")
    require(descriptor(report_path) == report_row, "focused GSCA report bytes differ from its receipt")
    require(
        report.get("runtime") == "tauri-webview2-cdp"
        and report.get("passed") is True
        and report.get("failures") == []
        and report.get("consoleErrors") == [],
        "focused GSCA report did not pass cleanly",
    )
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "gsca", "focused GSCA report scope is wrong")
    require(parse_utc(focused.get("completedAt"), "focusedRun.completedAt") <= completed + timedelta(seconds=2), "focused GSCA report completion falls outside the supervisor receipt")
    checks = report.get("checks")
    require(isinstance(checks, dict) and tuple(sorted(checks)) == GSCA_SCOPED_CHECK_IDS, "focused GSCA report check IDs differ from the frozen method-scoped contract")
    offline = checks.get("gscaFunctionalOffline")
    require(
        isinstance(offline, dict)
        and offline == receipt.get("functional_offline")
        and offline.get("passed") is True
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("externalRequestCount") == 0
        and offline.get("strictZeroProcessEgressClaimed") is False,
        "focused GSCA functional-offline receipt is missing or overstated",
    )
    desktop = ROOT / "target/release/quickpls-desktop.exe"
    require(receipt.get("executable") == descriptor(desktop), "focused GSCA executable differs from its receipt")
    process_rows = receipt.get("observed_process_tree")
    desktop_rows = [row for row in process_rows if isinstance(row, dict) and row.get("name") == "quickpls-desktop.exe"] if isinstance(process_rows, list) else []
    require(len(desktop_rows) == 1 and Path(str(desktop_rows[0].get("executable_path", ""))).resolve() == desktop.resolve(), "focused GSCA receipt did not authenticate exactly one release desktop process")
    export = receipt.get("export")
    project = receipt.get("project_archive")
    require(isinstance(export, dict) and isinstance(project, dict), "focused GSCA receipt is missing export/archive descriptors")
    workbook_path = ROOT / str(export.get("path", ""))
    project_path = ROOT / str(project.get("path", ""))
    require(descriptor(workbook_path) == export, "focused GSCA XLSX bytes differ from its receipt")
    require(descriptor(project_path) == project, "focused GSCA archive bytes differ from its receipt")
    require(Path(str(checks["gscaExport"]["nativeXlsx"]["targetPath"])).resolve() == workbook_path.resolve(), "focused GSCA report export differs from its receipt")
    require(Path(str(checks["gscaFixture"]["projectPath"])).resolve() == project_path.resolve(), "focused GSCA report archive differs from its receipt")
    screenshot_rows = [descriptor(Path(str(path))) for path in report.get("screenshots", [])]
    require(len(screenshot_rows) == 11 and receipt.get("screenshots") == screenshot_rows, "focused GSCA screenshot family differs from its receipt")
    require(report.get("screenshotArtifacts") == screenshot_rows, "focused GSCA report does not bind the exact screenshot bytes")
    return {
        "passed": True,
        "exact_required_checks": True,
        "required_check_count": len(GSCA_SCOPED_CHECK_IDS),
        "receipt_kind": receipt["kind"],
        "started_at_utc": started.isoformat(),
        "completed_at_utc": completed.isoformat(),
        "report": report_row,
        "export": export,
        "cleanup_verified": True,
        "functional_offline_verified": True,
    }, report_path, workbook_path


def validate_nca_scoped_receipt(
    contract: MethodContract,
    receipt: dict[str, Any],
    report: dict[str, Any],
    not_before: datetime,
) -> tuple[dict[str, Any], Path, Path]:
    """Validate the append-only, method-scoped NCA supervisor receipt."""

    require(contract.slug == "nca_v2", "the focused NCA receipt cannot qualify another method")
    started = parse_utc(receipt.get("supervisor_started_at_utc"), "supervisor_started_at_utc")
    completed = parse_utc(receipt.get("completed_at_utc"), "completed_at_utc")
    require(started >= not_before - timedelta(seconds=2), "focused NCA receipt predates the requested evidence floor")
    require(completed >= started, "focused NCA receipt completion predates its start")
    require(
        receipt.get("schema_version") == 1
        and receipt.get("kind") == "quickpls_v247_nca_scoped_native_acceptance_receipt"
        and receipt.get("scope") == "nca"
        and receipt.get("feature_id") == contract.feature_id
        and receipt.get("method_version") == contract.method_version,
        "focused NCA receipt identity is wrong",
    )
    require(
        receipt.get("passed") is True
        and receipt.get("failures") == 0
        and receipt.get("console_errors") == 0
        and receipt.get("checks") == len(NCA_SCOPED_CHECK_IDS)
        and receipt.get("unique_checks") == len(NCA_SCOPED_CHECK_IDS)
        and tuple(receipt.get("check_ids", ())) == NCA_SCOPED_CHECK_IDS,
        "focused NCA receipt is not a clean exact-check receipt",
    )
    require(
        receipt.get("runtime") == "tauri-webview2-cdp"
        and receipt.get("cdp_endpoint") == "http://127.0.0.1:9222"
        and receipt.get("cdp_loopback_only") is True
        and receipt.get("graceful_process_cleanup_verified") is True
        and receipt.get("forced_process_cleanup_used") is False
        and receipt.get("orphan_processes") == 0,
        "focused NCA process/runtime boundary is not clean",
    )
    report_row = receipt.get("report")
    require(isinstance(report_row, dict), "focused NCA receipt has no report descriptor")
    report_path = ROOT / str(report_row.get("path", ""))
    require(repository_path(report_path) == "validation/results/v247_tauri_native_acceptance_nca.json", "focused NCA receipt points to the wrong report")
    require(descriptor(report_path) == report_row, "focused NCA report bytes differ from its receipt")
    require(
        report.get("runtime") == "tauri-webview2-cdp"
        and report.get("passed") is True
        and report.get("failures") == []
        and report.get("consoleErrors") == [],
        "focused NCA report did not pass cleanly",
    )
    focused = report.get("focusedRun")
    require(isinstance(focused, dict) and focused.get("scope") == "nca", "focused NCA report scope is wrong")
    require(parse_utc(focused.get("completedAt"), "focusedRun.completedAt") <= completed + timedelta(seconds=2), "focused NCA report completion falls outside the supervisor receipt")
    checks = report.get("checks")
    require(isinstance(checks, dict) and tuple(sorted(checks)) == NCA_SCOPED_CHECK_IDS, "focused NCA report check IDs differ from the frozen method-scoped contract")
    offline = checks.get("ncaFunctionalOffline")
    require(
        isinstance(offline, dict)
        and offline == receipt.get("functional_offline")
        and offline.get("passed") is True
        and offline.get("analyticalWorkflowRequiresInternet") is False
        and offline.get("externalRequestCount") == 0
        and offline.get("strictZeroProcessEgressClaimed") is False,
        "focused NCA functional-offline receipt is missing or overstated",
    )
    desktop = ROOT / "target/release/quickpls-desktop.exe"
    require(receipt.get("executable") == descriptor(desktop), "focused NCA executable differs from its receipt")
    process_rows = receipt.get("observed_process_tree")
    desktop_rows = [row for row in process_rows if isinstance(row, dict) and row.get("name") == "quickpls-desktop.exe"] if isinstance(process_rows, list) else []
    require(len(desktop_rows) == 1 and Path(str(desktop_rows[0].get("executable_path", ""))).resolve() == desktop.resolve(), "focused NCA receipt did not authenticate exactly one release desktop process")
    export = receipt.get("export")
    project = receipt.get("project_archive")
    require(isinstance(export, dict) and isinstance(project, dict), "focused NCA receipt is missing export/archive descriptors")
    workbook_path = ROOT / str(export.get("path", ""))
    project_path = ROOT / str(project.get("path", ""))
    require(descriptor(workbook_path) == export, "focused NCA XLSX bytes differ from its receipt")
    require(descriptor(project_path) == project, "focused NCA archive bytes differ from its receipt")
    require(Path(str(checks["ncaExport"]["nativeXlsx"]["targetPath"])).resolve() == workbook_path.resolve(), "focused NCA report export differs from its receipt")
    reported_project = checks["ncaFixtureProvisioning"].get(contract.project_field)
    require(
        isinstance(reported_project, str)
        and Path(reported_project).resolve() == project_path.resolve(),
        "focused NCA report archive differs from its receipt",
    )
    screenshot_rows = [descriptor(Path(str(path))) for path in report.get("screenshots", [])]
    require(len(screenshot_rows) == 11 and receipt.get("screenshots") == screenshot_rows, "focused NCA screenshot family differs from its receipt")
    require(report.get("screenshotArtifacts") == screenshot_rows, "focused NCA report does not bind the exact screenshot bytes")
    return {
        "passed": True,
        "exact_required_checks": True,
        "required_check_count": len(NCA_SCOPED_CHECK_IDS),
        "receipt_kind": receipt["kind"],
        "started_at_utc": started.isoformat(),
        "completed_at_utc": completed.isoformat(),
        "report": report_row,
        "export": export,
        "cleanup_verified": True,
        "functional_offline_verified": True,
    }, report_path, workbook_path


def invalid_setup_contract(contract: MethodContract, checks: dict[str, Any]) -> dict[str, Any]:
    invalid = checks.get(contract.invalid_key)
    require(isinstance(invalid, dict), f"missing {contract.invalid_key}")
    before = invalid.get("archiveBefore")
    after = invalid.get("archiveAfter")
    blocker = invalid.get("missingRolesBlocked") if contract.slug == "nca_v2" else invalid.get("emptyModelBlocker")
    passed = (
        invalid.get("attempted") is True
        and invalid.get("selectedMethod") == contract.selected_method
        and invalid.get("startEnabled") is False
        and blocker is True
        and isinstance(before, dict)
        and after == before
        and invalid.get("archiveStateUnchanged") is True
        and invalid.get("resultCreated") is False
    )
    require(passed, f"{contract.slug} invalid setup did not fail closed without state mutation: {invalid}")
    return {"passed": True, "check": contract.invalid_key, "archive_state_unchanged": True, "result_created": False}


def cancellation_contract(
    contract: MethodContract,
    checks: dict[str, Any],
    run_id: str,
) -> dict[str, Any]:
    if contract.cancellation_key is None:
        return {"passed": True, "required": False}
    value = checks.get(contract.cancellation_key)
    require(isinstance(value, dict), f"missing {contract.cancellation_key}")
    before = value.get("archiveBefore")
    after = value.get("archiveAfter")
    retry = value.get("retrySettings")
    cancelled = value.get("cancelledSettings")
    require(isinstance(before, dict) and isinstance(after, dict), f"{contract.slug} cancellation archive snapshots are missing")
    require(isinstance(retry, dict) and isinstance(cancelled, dict), f"{contract.slug} cancellation settings are missing")
    common = (
        value.get("passed") is True
        and value.get("cancelledMethod") == contract.selected_method
        and value.get("noPartialVisibleResult") is True
        and value.get("noPartialCommittedResult") is True
        and value.get("archiveStateUnchanged") is True
        and after == before
        and value.get("retryEnabled") is True
        and value.get("completedRetryRunId") == run_id
        and retry.get("selectedMethod") == contract.selected_method
        and isinstance(value.get("terminalMessage"), str)
        and bool(value.get("terminalMessage"))
    )
    if contract.slug == "plspredict_cvpat_v2":
        settings_match = retry.get("plan") == cancelled.get("plan") and retry.get("seed") == cancelled.get("seed")
        active = value.get("activeLifecycle")
        lifecycle = (
            value.get("activeLifecycleCaptured") is True
            and isinstance(active, dict)
            and active.get("captured") is True
            and active.get("status") in {"queued", "validating", "running", "cancelling"}
            and active.get("ariaBusy") == "true"
        )
    else:
        keys = ("x", "y", "ceiling", "permutations", "seed")
        settings_match = all(retry.get(key) == cancelled.get(key) for key in keys)
        active = value.get("activeLifecycle")
        lifecycle = isinstance(active, dict) and active.get("status") in {"queued", "validating", "running"} and active.get("cancelVisible") is True
    require(common and settings_match and lifecycle, f"{contract.slug} cancellation/retry contract failed: {value}")
    return {
        "passed": True,
        "required": True,
        "check": contract.cancellation_key,
        "same_settings_retried": True,
        "no_partial_visible_result": True,
        "no_partial_committed_result": True,
        "archive_state_unchanged": True,
        "completed_retry_run_id": run_id,
        "cancelled_settings": cancelled,
    }


def screenshot_contract(contract: MethodContract, report: dict[str, Any]) -> tuple[dict[str, Any], list[Path]]:
    rows = report.get("screenshots")
    require(isinstance(rows, list) and all(isinstance(row, str) for row in rows), "functional screenshot list is malformed")
    artifacts = report.get("screenshotArtifacts")
    require(isinstance(artifacts, list) and all(isinstance(row, dict) for row in artifacts), "functional screenshot descriptor list is malformed")
    paths: list[Path] = []
    for sequence, state in contract.captures:
        name = f"{sequence}-tauri-native-{contract.capture_prefix}-{state}-1536x794.png"
        path = SCREEN_ROOT / name
        require(rows.count(str(path.resolve())) == 1, f"functional report must bind exactly one screenshot: {name}")
        actual = descriptor(path)
        require(artifacts.count(actual) == 1, f"functional report descriptor must bind exact screenshot bytes: {name}")
        paths.append(path)
    return {"passed": True, "count": len(paths), "states": [state for _, state in contract.captures]}, paths


def packaged_workflow_contract(
    contract: MethodContract,
    report: dict[str, Any],
    receipt_workbook: Path,
) -> tuple[dict[str, Any], Path, Path, list[Path]]:
    checks = report.get("checks")
    require(isinstance(checks, dict), "functional report checks are missing")
    dialog = checks.get(contract.dialog_key)
    result = checks.get(contract.result_key)
    progress = checks.get(contract.progress_key)
    export = checks.get(contract.export_key)
    reopen = checks.get(contract.reopen_key)
    fixture = checks.get(contract.fixture_key)
    for label, value in (("dialog", dialog), ("result", result), ("progress", progress), ("export", export), ("reopen", reopen), ("fixture", fixture)):
        require(isinstance(value, dict), f"{contract.slug} {label} evidence is missing")
    require(dialog.get("selectedMethod") == contract.selected_method, f"{contract.slug} selected method changed")
    require(dialog.get("startEnabled") is True and dialog.get("blockers") == [], f"{contract.slug} valid setup is not runnable")
    run_id = result.get("runId")
    require(isinstance(run_id, str) and bool(run_id), f"{contract.slug} completed run has no identity")
    require(any(contract.method_version in value for value in nested_strings(result)), f"{contract.slug} result does not expose its method version")
    for path, expected in contract.exact_result_values:
        require(pointer(result, path) == expected, f"{contract.slug} result contract differs at {path}")
    for path, minimum in contract.minimum_result_values:
        actual = pointer(result, path)
        require(isinstance(actual, int) and not isinstance(actual, bool) and actual >= minimum, f"{contract.slug} result is incomplete at {path}")
    if contract.progress_kind == "captured":
        proof = progress.get("completedRunProof")
        lifecycle_captured = progress.get("captured") is True or progress.get("status") in {"queued", "validating", "running"}
        require(lifecycle_captured and isinstance(proof, dict) and proof.get("matched") is True and proof.get("runId") == run_id, f"{contract.slug} lifecycle did not bind the completed run")
    else:
        require(progress.get("status") in {"queued", "validating", "running"} and progress.get("cancelVisible") is True and int(progress.get("logEntries", 0)) >= 1, f"{contract.slug} active lifecycle evidence is missing")
    invalid = invalid_setup_contract(contract, checks)
    cancellation = cancellation_contract(contract, checks, run_id)
    require(reopen.get("sameRunRestored") is True and reopen.get("expectedRunId") == run_id and reopen.get("selectedRunId") == run_id, f"{contract.slug} did not reopen the exact completed run")
    archive_summary = reopen.get("archive")
    require(isinstance(archive_summary, dict) and any(contract.method_version in value for value in nested_strings(archive_summary)), f"{contract.slug} reopened archive identity is missing")
    native = export.get("nativeXlsx")
    require(isinstance(native, dict) and native.get("attempted") is True and native.get("file", {}).get("isFile") is True, f"{contract.slug} native XLSX was not created")
    target = Path(str(native.get("targetPath", "")))
    require(target.resolve() == receipt_workbook.resolve(), f"{contract.slug} report XLSX is not the receipt-bound export")
    completion = native.get("helper", {}).get("completion", {})
    workbook = completion.get("workbook", {})
    sheets = workbook.get("sheetNames")
    require(completion.get("passed") is True and isinstance(sheets, list) and len(sheets) >= 2 and "Run provenance" in sheets, f"{contract.slug} XLSX read-back is incomplete")
    actual_workbook = descriptor(receipt_workbook)
    require(workbook.get("size") == actual_workbook["size"] and workbook.get("sha256") == actual_workbook["sha256"], f"{contract.slug} XLSX read-back bytes differ")
    project = Path(str(fixture.get(contract.project_field, "")))
    require(project.is_file(), f"{contract.slug} reopened project archive is missing")
    offline_key = {
        "gsca_als_v2": "gscaFunctionalOffline",
        "nca_v2": "ncaFunctionalOffline",
    }.get(contract.slug, "bootstrapFunctionalOffline")
    offline = checks.get(offline_key)
    require(isinstance(offline, dict) and offline.get("passed") is True and offline.get("analyticalWorkflowRequiresInternet") is False and offline.get("externalRequestCount") == 0 and offline.get("strictZeroProcessEgressClaimed") is False, "functional-offline boundary failed or was overstated")
    screenshots, screenshot_paths = screenshot_contract(contract, report)
    packaged_viewports, viewport_screenshot_paths = packaged_viewport_contract(contract, report, run_id)
    return {
        "passed": True,
        "run_id": run_id,
        "selected_method": contract.selected_method,
        "method_version": contract.method_version,
        "invalid_setup": invalid,
        "cancellation_retry": cancellation,
        "same_run_reopened": True,
        "xlsx_sheets": sheets,
        "functional_offline": True,
        "strict_zero_process_egress_claimed": False,
        "screenshots": screenshots,
        "packaged_viewports": packaged_viewports,
    }, project, receipt_workbook, [*screenshot_paths, *viewport_screenshot_paths]


def packaged_viewport_method_slug(contract: MethodContract) -> str:
    if contract.slug == "consistent_bootstrap_v1":
        require(
            contract.feature_id == "qpls3.inference.consistent_bootstrap"
            and contract.method_version == "plsc_bootstrap_v1",
            "consistent-bootstrap viewport alias is bound to an unexpected method identity",
        )
        return contract.method_version
    return contract.slug


def packaged_viewport_contract(
    contract: MethodContract,
    report: dict[str, Any],
    run_id: str,
) -> tuple[dict[str, Any], list[Path]]:
    """Validate real WebView2/Tauri window resizing; emulated viewports fail."""

    key = f"{contract.visual_key}PackagedViewports"
    value = report.get("checks", {}).get(key)
    require(isinstance(value, dict), f"missing actual packaged viewport check: {key}")
    target = value.get("targetIdentity")
    metrics = value.get("deviceMetricsOverride")
    method = value.get("method")
    restored = value.get("restoredFinalWindowState")
    expected_method_slug = packaged_viewport_method_slug(contract)
    require(
        value.get("passed") is True
        and value.get("actualTauriWindow") is True
        and value.get("resizeMechanism") == "Browser.setWindowBounds",
        f"{key} is not actual packaged-window evidence",
    )
    require(
        isinstance(target, dict)
        and isinstance(target.get("targetId"), str)
        and bool(target.get("targetId"))
        and isinstance(target.get("windowId"), int)
        and not isinstance(target.get("windowId"), bool)
        and target.get("lookupCommand") == "Browser.getWindowForTarget"
        and target.get("origin") == "http://tauri.localhost",
        f"{key} did not bind a concrete Tauri WebView2 target/window identity",
    )
    require(
        isinstance(metrics, dict)
        and metrics.get("clearCommand") == "Emulation.clearDeviceMetricsOverride"
        and metrics.get("cleared") is True
        and "playwrightViewportBefore" in metrics
        and metrics.get("playwrightViewportBefore") is None
        and metrics.get("pageSetViewportSizeUsed") is False
        and metrics.get("emulationOnly") is False,
        f"{key} used or retained viewport emulation",
    )
    require(
        isinstance(method, dict)
        and method.get("slug") == expected_method_slug
        and method.get("version") == contract.method_version
        and method.get("expectedRunId") == run_id
        and isinstance(method.get("expectedRunLabel"), str)
        and contract.selected_method in method.get("expectedRunLabel"),
        f"{key} method/run envelope differs",
    )
    rows = value.get("exactViewports")
    require(isinstance(rows, list) and len(rows) == 3, f"{key} must contain exactly three actual-window rows")
    require([row.get("id") for row in rows if isinstance(row, dict)] == list(EXPECTED_VIEWPORTS), f"{key} viewport order or membership differs")
    screenshots = report.get("screenshots")
    require(isinstance(screenshots, list) and all(isinstance(row, str) for row in screenshots), "functional screenshot list is malformed")
    artifacts = report.get("screenshotArtifacts")
    require(isinstance(artifacts, list) and all(isinstance(row, dict) for row in artifacts), "functional screenshot descriptor list is malformed")
    paths: list[Path] = []
    outer_sizes: set[tuple[int, int]] = set()
    for expected, row in zip(EXPECTED_VIEWPORTS, rows, strict=True):
        require(isinstance(row, dict), f"{key} contains a malformed viewport row")
        width, height = (int(part) for part in expected.split("x"))
        requested = row.get("requestedClientViewport")
        dom = row.get("domInnerDimensions")
        before = row.get("outerBoundsBefore")
        after = row.get("outerBoundsAfter")
        require(
            row.get("passed") is True
            and requested == {"width": width, "height": height}
            and dom == {"width": width, "height": height}
            and isinstance(before, dict)
            and isinstance(after, dict)
            and all(isinstance(after.get(field), int) and not isinstance(after.get(field), bool) for field in ("width", "height"))
            and row.get("origin") == "http://tauri.localhost"
            and row.get("tauriRuntime") is True
            and row.get("surface") == "results"
            and row.get("noHorizontalOverflow") is True
            and row.get("methodRunLinkage") is True
            and row.get("methodVersionEvidenceBound") is True
            and row.get("selectedRunId") == run_id
            and row.get("selectedRunLabel") is not None
            and contract.selected_method in row.get("selectedRunLabel")
            and row.get("selectedTableId") == method.get("expectedTableId")
            and isinstance(row.get("resultRows"), int)
            and row.get("resultRows") > 0,
            f"{key} failed its exact DOM, Tauri, overflow, or method/run linkage contract at {expected}",
        )
        outer_sizes.add((after["width"], after["height"]))
        screenshot = Path(str(row.get("screenshot", ""))).resolve()
        require(screenshot.is_relative_to(SCREEN_ROOT.resolve()), f"{key} screenshot escaped the native evidence directory")
        require(screenshot.name.endswith(f"packaged-viewport-{expected}.png"), f"{key} screenshot is not viewport-labelled")
        require(screenshots.count(str(screenshot)) == 1, f"{key} report must bind exactly one screenshot for {expected}")
        actual = descriptor(screenshot)
        require(artifacts.count(actual) == 1, f"{key} report descriptor does not bind exact screenshot bytes for {expected}")
        paths.append(screenshot)
    require(len(outer_sizes) == 3, f"{key} did not resize the actual outer window across all three client sizes")
    require(
        isinstance(restored, dict)
        and restored.get("passed") is True
        and restored.get("tolerancePixels") == 1
        and window_bounds_equal(restored.get("requested"), restored.get("actual"), tolerance_pixels=1),
        f"{key} did not restore the final desktop window state",
    )
    return {
        "passed": True,
        "check": key,
        "actual_tauri_window": True,
        "viewport_emulation": False,
        "resize_mechanism": "Browser.setWindowBounds",
        "target_id": target["targetId"],
        "window_id": target["windowId"],
        "viewports": list(EXPECTED_VIEWPORTS),
        "same_run_id": run_id,
        "restored_final_window_state": True,
        "screenshot_count": 3,
    }, paths


def read_archive(path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    with zipfile.ZipFile(path) as archive:
        names = [entry.filename for entry in archive.infolist()]
        require(len(names) == len(set(names)), "project archive contains duplicate entries")
        require(all(not name.startswith(("/", "\\")) and "\\" not in name and ".." not in Path(name).parts for name in names), "project archive contains an unsafe entry")
        entries = {name: archive.read(name) for name in names}
    require("project.json" in entries and "manifest.json" in entries, "project archive is incomplete")
    project = json.loads(entries["project.json"].decode("utf-8"), object_pairs_hook=lambda pairs: _strict_pairs(pairs, path), parse_constant=lambda token: (_ for _ in ()).throw(AdapterError(f"non-finite archive JSON value: {token}")))
    manifest = json.loads(entries["manifest.json"].decode("utf-8"), object_pairs_hook=lambda pairs: _strict_pairs(pairs, path), parse_constant=lambda token: (_ for _ in ()).throw(AdapterError(f"non-finite archive JSON value: {token}")))
    require(isinstance(project, dict) and isinstance(manifest, dict), "project archive JSON roots must be objects")
    checksums = manifest.get("checksums")
    require(isinstance(checksums, dict) and set(checksums) == set(entries) - {"manifest.json"}, "project archive checksum membership differs")
    require(all(isinstance(expected, str) and hashlib.sha256(entries[name]).hexdigest() == expected for name, expected in checksums.items()), "project archive checksum verification failed")
    return project, manifest


def _strict_pairs(pairs: list[tuple[str, Any]], path: Path) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        require(key not in result, f"duplicate JSON key in archive {path}: {key}")
        result[key] = value
    return result


def archive_contract(
    contract: MethodContract,
    archive_path: Path,
    run_id: str,
    cancellation: dict[str, Any],
) -> dict[str, Any]:
    project, manifest = read_archive(archive_path)
    results = [row for row in project.get("results", []) if isinstance(row, dict) and row.get("id") == run_id]
    require(len(results) == 1, f"{contract.slug} archive does not contain exactly one reported result")
    result = results[0]
    provenance = result.get("provenance")
    payload = result.get("payload")
    require(result.get("status") == "completed" and isinstance(provenance, dict) and isinstance(payload, dict), f"{contract.slug} archived result is not completed")
    require(provenance.get("method") == contract.archive_method and contract.method_version in str(provenance.get("method_version", "")), f"{contract.slug} archived provenance differs")
    require(payload.get("kind") == "pls_pm_v1" and any(contract.method_version in value for value in nested_strings(payload)), f"{contract.slug} archived payload identity differs")
    recipe_id = provenance.get("recipe_id")
    recipes = [row for row in project.get("recipes", []) if isinstance(row, dict) and row.get("id") == recipe_id]
    require(len(recipes) == 1, f"{contract.slug} archived result has no unique recipe")
    recipe = recipes[0]
    config = recipe.get("method_config")
    settings = recipe.get("settings")
    require(isinstance(config, dict) and config.get("kind") == contract.archive_method, f"{contract.slug} archived method config differs")
    require(isinstance(settings, dict) and settings.get("method") == contract.archive_method, f"{contract.slug} archived settings differ")
    if cancellation.get("required") is True:
        cancelled = cancellation["cancelled_settings"]
        if contract.slug == "plspredict_cvpat_v2":
            require(settings.get("seed") == cancelled.get("seed"), "prediction archived seed differs from cancelled/retried seed")
            predict = payload.get("estimation", {}).get("predict", {})
            repeated = predict.get("repeated_kfold", {}) if isinstance(predict, dict) else {}
            require(
                isinstance(repeated, dict)
                and repeated.get("method_version") == "plspredict_repeated_kfold_indicator_v2"
                and repeated.get("folds") == 10
                and repeated.get("repeats") == 10
                and repeated.get("seed") == cancelled.get("seed"),
                "prediction archived run differs from the cancelled/retried 10-fold by 10-repeat plan",
            )
        else:
            require(config.get("condition") == cancelled.get("x") and config.get("outcome") == cancelled.get("y") and config.get("ceiling") == cancelled.get("ceiling") and str(config.get("permutation_samples")) == cancelled.get("permutations") and str(settings.get("seed")) == cancelled.get("seed"), "NCA archived settings differ from cancelled/retried settings")
    return {
        "passed": True,
        "run_id": run_id,
        "result_completed": True,
        "payload_kind": "pls_pm_v1",
        "provenance_method": contract.archive_method,
        "method_version": contract.method_version,
        "recipe_id": recipe_id,
        "archive_engine_version": manifest.get("engine_version"),
        "checksums_verified": True,
        "retry_settings_persisted": True,
    }


def role_sources(contract: MethodContract, role: str, extras: Iterable[Path]) -> list[Path]:
    document = load_manifest(contract)
    governance = document["governance"]
    relative = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        *document["qualification"]["source_requirements"][role],
    }
    paths = [ROOT / item for item in relative]
    paths.extend(extras)
    unique = {repository_path(path): path for path in paths}
    return [unique[key] for key in sorted(unique)]


def write_identity(
    contract: MethodContract,
    role: str,
    checks: dict[str, Any],
    sources: Iterable[Path],
) -> Path:
    rows = [descriptor(path) for path in sources]
    unique = {row["path"]: row for row in rows}
    payload = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "passed": True,
        "feature_id": contract.feature_id,
        "method_version": contract.method_version,
        "catalogue_snapshot_date": contract.catalogue_date,
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "role": role,
        "checks": checks,
        "source_artifacts": [unique[key] for key in sorted(unique)],
    }
    target = output_path(contract, role)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(payload, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8", newline="\r\n")
    return target


def run_adapter(contract: MethodContract, not_before: datetime) -> dict[str, Any]:
    prior = verify_prior_factory_stages(contract)
    fresh, fresh_sources = source_freshness()
    build_finished = parse_utc(fresh["build_finished_at_utc"], "build_finished_at_utc")
    harness_changed = datetime.fromtimestamp(NATIVE_HARNESS.stat().st_mtime, timezone.utc)
    evidence_floor = max(not_before, build_finished, harness_changed)
    receipt_path = {
        "gsca_als_v2": GSCA_SCOPED_RECEIPT,
        "nca_v2": NCA_SCOPED_RECEIPT,
    }.get(contract.slug, CUMULATIVE_RECEIPT)
    receipt = strict_json(receipt_path)
    report_value = receipt.get("report", "")
    report_path = ROOT / str(report_value.get("path", "") if isinstance(report_value, dict) else report_value)
    report = strict_json(report_path)
    if contract.slug == "gsca_als_v2":
        cumulative, bound_report_path, receipt_workbook = validate_gsca_scoped_receipt(contract, receipt, report, evidence_floor)
    elif contract.slug == "nca_v2":
        cumulative, bound_report_path, receipt_workbook = validate_nca_scoped_receipt(contract, receipt, report, evidence_floor)
    else:
        cumulative, bound_report_path, receipt_workbook = validate_cumulative_receipt(contract, receipt, report, evidence_floor)
    packaged, source_project, source_workbook, screenshot_paths = packaged_workflow_contract(contract, report, receipt_workbook)
    target_archive = stable_archive_path(contract)
    target_workbook = stable_workbook_path(contract)
    target_archive.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source_project, target_archive)
    shutil.copy2(source_workbook, target_workbook)
    require(descriptor(target_workbook)["size"] == descriptor(source_workbook)["size"] and descriptor(target_workbook)["sha256"] == descriptor(source_workbook)["sha256"], f"{contract.slug} stable workbook copy differs")
    archive = archive_contract(contract, target_archive, packaged["run_id"], packaged["cancellation_retry"])
    checks = {
        "passed": True,
        "prior_factory": prior["derived_state"],
        "source_freshness": fresh,
        "cumulative": cumulative,
        "packaged": packaged,
        "archive": archive,
        "stable_evidence": {"archive": descriptor(target_archive), "workbook": descriptor(target_workbook)},
    }
    extras = [
        *fresh_sources,
        receipt_path,
        bound_report_path,
        NATIVE_HARNESS,
        target_archive,
        target_workbook,
        *screenshot_paths,
    ]
    packaged_identity = write_identity(contract, "packaged_acceptance", checks, role_sources(contract, "packaged_acceptance", extras))
    audit = subprocess.run(
        [sys.executable, contract.audit_script],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
        timeout=600,
    )
    require(audit.returncode == 0, f"{contract.slug} independent method audit failed: " + (audit.stdout + "\n" + audit.stderr)[-3000:])
    final_fresh, final_sources = source_freshness()
    require(final_fresh == fresh, f"{contract.slug} product freshness changed during release audit")
    require([descriptor(path) for path in final_sources] == [descriptor(path) for path in fresh_sources], f"{contract.slug} source descriptors changed during release audit")
    final = validate_manifest(manifest_path(contract), ROOT)
    require(final.get("passed") is True and final.get("derived_state") == "release_qualified", f"{contract.slug} final manifest did not derive release-qualified: {final.get('errors')}")
    return {
        "passed": True,
        "derived_state": final["derived_state"],
        "declared_state": final["declared_state"],
        "packaged_identity": descriptor(packaged_identity),
        "method_audit_identity": descriptor(output_path(contract, "method_audit")),
    }


def evaluate_audit(contract: MethodContract) -> dict[str, Any]:
    document = load_manifest(contract)
    identity = {"passed": True, "feature_id": contract.feature_id, "method_version": contract.method_version, "catalogue_snapshot_date": contract.catalogue_date}
    rows: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            rows.append({"stage": stage, "path": artifact.get("path"), "roles": artifact.get("roles"), "passed": passed, "errors": errors})
    release = document["qualification"]["evidence"]["release_qualified"]
    release_roles = [role for artifact in release for role in artifact.get("roles", [])]
    release_contract = len(release) == 2 and sorted(release_roles) == ["method_audit", "packaged_acceptance"] and all(len(artifact.get("roles", [])) == 1 for artifact in release)
    packaged_rows = [row for row in release if row.get("roles") == ["packaged_acceptance"]]
    require(len(packaged_rows) == 1, f"{contract.slug} manifest must contain one packaged identity row")
    packaged_passed, packaged_errors = _verify_artifact(packaged_rows[0], document, ROOT, identity)
    packaged = strict_json(ROOT / packaged_rows[0]["path"])
    checks = packaged.get("checks", {})
    semantic = {
        "packaged_identity_passes": packaged.get("passed") is True and checks.get("passed") is True,
        "prior_factory_native": checks.get("prior_factory") == "native_qualified",
        "source_receipt_and_cli_current": checks.get("source_freshness", {}).get("passed") is True and checks.get("source_freshness", {}).get("desktop_receipt_exact") is True and checks.get("source_freshness", {}).get("release_cli_newer_sources") == [],
        "exact_required_check_receipt": checks.get("cumulative", {}).get("passed") is True and checks.get("cumulative", {}).get("exact_required_checks") is True and checks.get("cumulative", {}).get("cleanup_verified") is True,
        "bounded_packaged_workflow": checks.get("packaged", {}).get("passed") is True and checks.get("packaged", {}).get("same_run_reopened") is True,
        "invalid_setup_no_state_mutation": checks.get("packaged", {}).get("invalid_setup", {}).get("passed") is True and checks.get("packaged", {}).get("invalid_setup", {}).get("archive_state_unchanged") is True,
        "required_cancellation_retry": checks.get("packaged", {}).get("cancellation_retry", {}).get("passed") is True and checks.get("packaged", {}).get("cancellation_retry", {}).get("required") is (contract.cancellation_key is not None),
        "same_run_archive": checks.get("archive", {}).get("passed") is True and checks.get("archive", {}).get("run_id") == checks.get("packaged", {}).get("run_id"),
        "actual_packaged_viewport_matrix": checks.get("packaged", {}).get("packaged_viewports", {}).get("passed") is True and checks.get("packaged", {}).get("packaged_viewports", {}).get("actual_tauri_window") is True and checks.get("packaged", {}).get("packaged_viewports", {}).get("viewport_emulation") is False and checks.get("packaged", {}).get("packaged_viewports", {}).get("viewports") == list(EXPECTED_VIEWPORTS),
    }
    return {
        "passed": release_contract and all(row["passed"] for row in rows) and packaged_passed and not packaged_errors and all(semantic.values()),
        "release_evidence_contract_passes": release_contract,
        "verified_prior_artifacts": rows,
        "packaged_artifact": {"path": packaged_rows[0]["path"], "passed": packaged_passed, "errors": packaged_errors},
        **semantic,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }


def run_audit(contract: MethodContract) -> dict[str, Any]:
    checks = evaluate_audit(contract)
    require(checks["passed"], f"{contract.slug} final method audit failed: {checks}")
    packaged = output_path(contract, "packaged_acceptance")
    sources = role_sources(contract, "method_audit", [ROOT / contract.audit_script, ROOT / contract.adapter_script, Path(__file__).resolve(), packaged])
    target = write_identity(contract, "method_audit", {"final_release_audit": checks}, sources)
    return {"passed": True, "method_audit_identity": descriptor(target)}


def main_for(slug: str) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--not-before-utc", required=True)
    args = parser.parse_args()
    try:
        report = run_adapter(METHODS[slug], parse_utc(args.not_before_utc, "--not-before-utc"))
    except (AdapterError, OSError, KeyError, TypeError, ValueError, zipfile.BadZipFile, SourceManifestFailure, subprocess.SubprocessError) as error:
        print(json.dumps({"passed": False, "method": slug, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0


def audit_main_for(slug: str) -> int:
    try:
        report = run_audit(METHODS[slug])
    except (AdapterError, OSError, KeyError, TypeError, ValueError, zipfile.BadZipFile) as error:
        print(json.dumps({"passed": False, "method": slug, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 0
