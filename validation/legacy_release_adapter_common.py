"""Strict release adapters for the four legacy method-factory migrations.

The historical promotion reports are not release evidence.  This module only
accepts current, scoped packaged reports, an exact frozen-build receipt, fresh
runtime/visual harness sources, exact on-disk artifacts, and (where the old
wrapper did not emit a complete cleanup attestation) the cumulative supervisor
receipt.  It writes the same identity-report format consumed by the promotion
manifest validator and keeps the final audit non-circular.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any, Iterable, Sequence

from diagnostic_bundle_source_manifest import (
    SourceManifestFailure,
    validate_build_receipt,
)
from method_promotion_manifest import _verify_artifact, validate_manifest


ROOT = Path(__file__).resolve().parents[1]
SOURCE = "validation/legacy_release_adapter_common.py"
FOCUSED_TEST = "validation/test_legacy_release_adapters.py"
BUILD_RECEIPT = "validation/results/diagnostic_bundle_build_receipt.json"
DESKTOP = "target/release/quickpls-desktop.exe"
RELEASE_CLI = "target/release/qpls.exe"
VISUAL_REPORT = "validation/results/v247_native_desktop_visual_acceptance.json"
CUMULATIVE_REPORT = "validation/results/v247_tauri_native_acceptance.json"
CUMULATIVE_RECEIPT = "validation/results/v247_cumulative_native_acceptance_receipt.json"
EXPECTED_VIEWPORTS = {
    "1024x700": {"width": 1024, "height": 700},
    "1280x720": {"width": 1280, "height": 720},
    "1440x900": {"width": 1440, "height": 900},
}


METHODS: dict[str, dict[str, Any]] = {
    "structural_path_randomization_v1": {
        "feature_id": "qpls3.inference.structural_path_randomization",
        "method_version": "freedman_lane_permutation_v1",
        "manifest": "validation/methods/structural_path_randomization_v1.manifest.json",
        "adapter": "validation/structural_path_randomization_v1_packaged_acceptance.py",
        "audit": "validation/structural_path_randomization_v1_factory_audit.py",
        "runtime_script": "validation/run_v247_structural_path_randomization_native_acceptance.ps1",
        "packaged_report": "validation/results/structural_path_randomization_v1_packaged_acceptance.json",
        "raw_report": "validation/results/v247_tauri_native_acceptance_structural_path_randomization.json",
        "scope": "structural_path_randomization",
        "kind": "quickpls3_scoped_tauri_structural_path_randomization_v1_acceptance",
        "visual_key": "structuralPathRandomization",
        "visual_states": ("structural-path-randomization-dialog",),
        "package_checks": {
            "runtimePreflight": "runtimePreflight",
            "structuralPathRandomizationFixtureProvisioning": "structuralPathRandomizationFixtureProvisioning",
            "structuralPathRandomizationSetup": "structuralPathRandomizationSetup",
            "structuralPathRandomizationCancellation": "structuralPathRandomizationCancellation",
            "structuralPathRandomizationResults": "structuralPathRandomizationResults",
            "structuralPathRandomizationExport": "structuralPathRandomizationExport",
            "structuralPathRandomizationArchive": "structuralPathRandomizationArchive",
            "structuralPathRandomizationSaveReopen": "structuralPathRandomizationSaveReopen",
            "resources": "resources",
            "cleanup": "cleanup",
        },
        "required_artifacts": {
            "xlsx": 1,
            "project_archive": 1,
            "resource_samples": 1,
            "resource_report": 1,
            "cleanup_report": 1,
            "cancellation_archive_before": 1,
            "cancellation_archive_after": 1,
            "screenshots": 6,
        },
        "cleanup_mode": "packaged_check",
        "requires_cumulative": False,
        "cumulative_exports": {},
        "runtime_sources": (
            "validation/run_v247_structural_path_randomization_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            "validation/windows_native_save_export.py",
            "validation/monitor_quickpls_process_tree.ps1",
        ),
    },
    "logistic_regression_v2": {
        "feature_id": "qpls3.standalone.logistic",
        "method_version": "regression_logistic_v2",
        "manifest": "validation/methods/logistic_regression_v2.manifest.json",
        "adapter": "validation/logistic_regression_v2_packaged_acceptance.py",
        "audit": "validation/logistic_regression_v2_factory_audit.py",
        "runtime_script": "validation/run_v247_logistic_native_acceptance.ps1",
        "packaged_report": "validation/results/logistic_v2_packaged_acceptance.json",
        "raw_report": "validation/results/v247_tauri_native_acceptance_logistic.json",
        "scope": "logistic",
        "kind": "quickpls3_scoped_tauri_logistic_v2_acceptance",
        "visual_key": "logistic",
        "visual_states": ("logistic-standalone-dialog",),
        "package_checks": {
            "workflow": "logisticWorkflow",
            "results": "logisticResult",
            "export": "logisticExport",
            "save_reopen": "logisticSaveReopen",
            "failure_lifecycle": "logisticFailureLifecycle",
            "legacy_v1": "logisticLegacyV1",
        },
        "required_artifacts": {"xlsx": 1, "project_archive": 1, "screenshots": 6},
        "cleanup_mode": "cumulative_receipt",
        "requires_cumulative": True,
        "cumulative_exports": {"logistic": "xlsx"},
        "runtime_sources": (
            "validation/run_v247_cumulative_native_acceptance.ps1",
            "validation/run_v247_logistic_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            "validation/windows_native_save_export.py",
        ),
    },
    "regression_bootstrap_v1": {
        "feature_id": "qpls3.standalone.regression_bootstrap",
        "method_version": "regression_bootstrap_v1",
        "manifest": "validation/methods/regression_bootstrap_v1.manifest.json",
        "adapter": "validation/regression_bootstrap_v1_packaged_acceptance.py",
        "audit": "validation/regression_bootstrap_v1_factory_audit.py",
        "runtime_script": "validation/run_v247_regression_bootstrap_native_acceptance.ps1",
        "packaged_report": "validation/results/regression_bootstrap_v1_packaged_acceptance.json",
        "raw_report": "validation/results/v247_tauri_native_acceptance_regression_bootstrap.json",
        "scope": "regression_bootstrap",
        "kind": "quickpls3_scoped_tauri_regression_bootstrap_v1_acceptance",
        "visual_key": "regressionBootstrap",
        "visual_states": (
            "regression-bootstrap-ols-dialog",
            "regression-bootstrap-logistic-dialog",
        ),
        "package_checks": {
            "workflow": "regressionBootstrapWorkflow",
            "results": "regressionBootstrapResults",
            "ols_export": "regressionBootstrapOlsExport",
            "logistic_export": "regressionBootstrapLogisticExport",
            "save_reopen": "regressionBootstrapSaveReopen",
            "cancellation": "regressionBootstrapCancellation",
            "witness_boundary": "regressionBootstrapWitnessBoundary",
        },
        "required_artifacts": {
            "ols_xlsx": 1,
            "logistic_xlsx": 1,
            "project_archive": 1,
            "screenshots": 9,
        },
        "cleanup_mode": "cumulative_receipt",
        "requires_cumulative": True,
        "cumulative_exports": {
            "regression_bootstrap_ols": "ols_xlsx",
            "regression_bootstrap_logistic": "logistic_xlsx",
        },
        "runtime_sources": (
            "validation/run_v247_cumulative_native_acceptance.ps1",
            "validation/run_v247_regression_bootstrap_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            "validation/windows_native_save_export.py",
        ),
    },
    "process_v2": {
        "feature_id": "qpls3.standalone.process",
        "method_version": "regression_process_v2",
        "manifest": "validation/methods/process_v2.manifest.json",
        "adapter": "validation/process_v2_packaged_acceptance.py",
        "audit": "validation/process_v2_factory_audit.py",
        "runtime_script": "validation/run_v247_process_v2_native_acceptance.ps1",
        "packaged_report": "validation/results/process_v2_packaged_acceptance.json",
        "raw_report": "validation/results/v247_tauri_native_acceptance_process_v2.json",
        "scope": "process_v2",
        "kind": "quickpls3_scoped_tauri_process_v2_acceptance",
        "visual_key": "processV2",
        "visual_states": ("process-v2-dialog",),
        "package_checks": {
            "runtime_preflight": "runtimePreflight",
            "workflow": "processV2Workflow",
            "setup": "processV2Setup",
            "results": "processV2Results",
            "export": "processV2Export",
            "save_reopen": "processV2SaveReopen",
            "cancellation": "processV2Cancellation",
            "cancelled_retry_setup": "processV2CancelledRetrySetup",
            "witness_boundary": "processV2WitnessBoundary",
            "resource_reset": "processV2ResourceResetClone",
            "resources": None,
        },
        "required_artifacts": {
            "xlsx": 1,
            "project_archive": 1,
            "resource_report": 1,
            "resource_samples": 1,
            "resource_phases": 1,
            "resource_phase_snapshots": 5,
            "screenshots": 8,
        },
        "cleanup_mode": "dedicated_report",
        "cleanup_report": "validation/results/v247_process_v2_process_cleanup.json",
        "requires_cumulative": False,
        "cumulative_exports": {},
        "runtime_sources": (
            "validation/run_v247_process_v2_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            "validation/windows_native_save_export.py",
            "validation/monitor_quickpls_process_tree.ps1",
            "validation/process_v2_reference.py",
        ),
    },
}


class DuplicateKeyError(ValueError):
    pass


def _strict_pairs(rows: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in rows:
        if key in result:
            raise DuplicateKeyError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def strict_load_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8-sig") as handle:
        value = json.load(
            handle,
            object_pairs_hook=_strict_pairs,
            parse_constant=lambda token: (_ for _ in ()).throw(
                ValueError(f"non-finite JSON value: {token}")
            ),
        )
    if not isinstance(value, dict):
        raise ValueError(f"JSON root must be an object: {path}")
    return value


def _parse_utc(value: Any) -> datetime:
    if not isinstance(value, str):
        raise ValueError("UTC timestamp must be a string")
    parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    if parsed.tzinfo is None:
        raise ValueError("UTC timestamp must contain an offset")
    return parsed.astimezone(timezone.utc)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def repository_path(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def _safe_path(relative: Any) -> Path:
    if (
        not isinstance(relative, str)
        or not relative
        or "\\" in relative
        or Path(relative).is_absolute()
        or ".." in Path(relative).parts
    ):
        raise ValueError(f"unsafe repository path: {relative!r}")
    resolved = (ROOT / relative).resolve()
    resolved.relative_to(ROOT.resolve())
    return resolved


def descriptor(path: Path, *, include_mtime: bool = False) -> dict[str, Any]:
    row: dict[str, Any] = {
        "path": repository_path(path),
        "size": path.stat().st_size,
        "sha256": sha256_file(path),
    }
    if include_mtime:
        row["mtime_ns"] = path.stat().st_mtime_ns
    return row


def _descriptor_matches(reported: Any) -> tuple[bool, dict[str, Any]]:
    try:
        if not isinstance(reported, dict):
            raise ValueError("artifact descriptor must be an object")
        if not {"path", "size", "sha256"} <= set(reported):
            raise ValueError("artifact descriptor must contain path, size, and sha256")
        path = _safe_path(reported["path"])
        if not path.is_file():
            raise FileNotFoundError(reported["path"])
        actual = descriptor(path)
        return (
            all(reported.get(key) == actual[key] for key in ("path", "size", "sha256")),
            actual,
        )
    except (OSError, ValueError, TypeError) as error:
        return False, {"error": f"{type(error).__name__}: {error}"}


def _collect_artifact_descriptors(value: Any) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if isinstance(value, dict):
        if {"path", "size", "sha256"} <= set(value):
            rows.append(value)
        else:
            for child in value.values():
                rows.extend(_collect_artifact_descriptors(child))
    elif isinstance(value, list):
        for child in value:
            rows.extend(_collect_artifact_descriptors(child))
    return rows


def manifest(method: str) -> dict[str, Any]:
    config = METHODS[method]
    document = strict_load_json(_safe_path(config["manifest"]))
    feature = document["feature"]
    expected = {
        "id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": "2026-08-12",
    }
    for key, value in expected.items():
        if feature.get(key) != value:
            raise ValueError(
                f"{method} manifest identity mismatch for {key}: "
                f"expected {value!r}, found {feature.get(key)!r}"
            )
    return document


def _identity(method: str) -> dict[str, Any]:
    feature = manifest(method)["feature"]
    return {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }


def _role_sources(method: str, role: str, extras: Iterable[str]) -> list[str]:
    document = manifest(method)
    governance = document["governance"]
    required = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        SOURCE,
        FOCUSED_TEST,
        *document["qualification"]["source_requirements"][role],
        *extras,
    }
    return sorted(required)


def write_identity_report(
    method: str,
    role: str,
    *,
    passed: bool,
    checks: dict[str, Any],
    extras: Iterable[str] = (),
    execution: dict[str, Any] | None = None,
) -> Path:
    config = METHODS[method]
    feature = manifest(method)["feature"]
    output = ROOT / "validation" / "results" / "method_factory" / method
    output.mkdir(parents=True, exist_ok=True)
    source_artifacts = [descriptor(_safe_path(path)) for path in _role_sources(method, role, extras)]
    report: dict[str, Any] = {
        "schema_version": 1,
        "report_kind": "quickpls_method_factory_identity_report",
        "role": role,
        "passed": bool(passed),
        "feature_id": config["feature_id"],
        "method_version": config["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
        "generated_at_utc": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_artifacts": source_artifacts,
        "checks": checks,
    }
    if execution is not None:
        report["execution"] = execution
    path = output / f"{role}.identity.json"
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return path


def run_command(command: Sequence[str], *, timeout: int) -> tuple[subprocess.CompletedProcess[str], dict[str, Any]]:
    started = time.monotonic()
    completed = subprocess.run(
        list(command),
        cwd=ROOT,
        capture_output=True,
        text=True,
        timeout=timeout,
        env={**os.environ, "CARGO_BUILD_JOBS": "1"},
    )
    return completed, {
        "command": list(command),
        "returncode": completed.returncode,
        "duration_seconds": round(time.monotonic() - started, 3),
        "stdout_tail": completed.stdout[-4000:],
        "stderr_tail": completed.stderr[-4000:],
    }


def cli_source_paths() -> list[str]:
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


def source_freshness(method: str) -> dict[str, Any]:
    """Bind the current desktop receipt and current release CLI source closure."""

    config = METHODS[method]
    try:
        receipt_path = _safe_path(BUILD_RECEIPT)
        desktop_path = _safe_path(DESKTOP)
        cli_path = _safe_path(RELEASE_CLI)
        for path in (receipt_path, desktop_path, cli_path):
            if not path.is_file():
                raise FileNotFoundError(repository_path(path))
        receipt = strict_load_json(receipt_path)
        validate_build_receipt(receipt, ROOT)
        desktop = descriptor(desktop_path, include_mtime=True)
        cli = descriptor(cli_path, include_mtime=True)
        cli_sources = [descriptor(_safe_path(path), include_mtime=True) for path in cli_source_paths()]
        newer = [row["path"] for row in cli_sources if row["mtime_ns"] > cli["mtime_ns"]]
        gate_paths = {
            SOURCE,
            FOCUSED_TEST,
            config["manifest"],
            config["adapter"],
            config["audit"],
            "validation/diagnostic_bundle_source_manifest.py",
            "validation/method_promotion_manifest.py",
            "validation/methods/method_promotion_manifest.schema.json",
            "validation/v247_native_desktop_visual_acceptance.mjs",
            *config["runtime_sources"],
        }
        gates = [descriptor(_safe_path(path), include_mtime=True) for path in sorted(gate_paths)]
        return {
            "passed": not newer,
            "desktop_receipt_exact": True,
            "desktop": desktop,
            "release_cli": cli,
            "release_cli_newer_build_sources": newer,
            "release_cli_build_sources": cli_sources,
            "gate_sources_excluded_from_binary_freshness": gates,
            "build_receipt": descriptor(receipt_path),
            "build_finished_at_utc": receipt.get("build_finished_at_utc"),
        }
    except (OSError, ValueError, KeyError, SourceManifestFailure) as error:
        return {
            "passed": False,
            "desktop_receipt_exact": False,
            "error": f"{type(error).__name__}: {error}",
        }


def verify_prior_factory_stages(method: str) -> dict[str, Any]:
    document = manifest(method)
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
            passed, errors = _verify_artifact(artifact, document, ROOT, _identity(method))
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


def evaluate_scoped_documents(
    method: str,
    packaged: dict[str, Any],
    raw: dict[str, Any],
    not_before: datetime,
    build_finished: datetime,
) -> dict[str, Any]:
    """Pure fail-closed identity/timing/check linkage used by runtime and tests."""

    config = METHODS[method]
    generated = _parse_utc(packaged.get("generated_at_utc"))
    completed = _parse_utc(packaged.get("completed_at_utc"))
    raw_generated = _parse_utc(raw.get("generatedAt"))
    raw_completed = _parse_utc(raw.get("focusedRun", {}).get("completedAt"))
    package_checks = packaged.get("checks")
    raw_checks = raw.get("checks")
    if not isinstance(package_checks, dict) or not isinstance(raw_checks, dict):
        raise ValueError("packaged and raw checks must be objects")
    linked: dict[str, Any] = {}
    for package_name, raw_name in config["package_checks"].items():
        package_row = package_checks.get(package_name)
        raw_row = raw_checks.get(raw_name) if raw_name is not None else None
        package_source_bound = (
            raw_name is None
            or package_name == raw_name
            or (
                isinstance(package_row, dict)
                and package_row.get("source_check") == raw_name
            )
        )
        raw_row_bound = raw_name is None or (
            isinstance(raw_row, dict)
            and bool(raw_row)
            and (
                "passed" not in raw_row
                or raw_row.get("passed") is True
            )
        )
        linked[package_name] = {
            "passed": isinstance(package_row, dict)
            and package_row.get("passed") is True
            and package_source_bound
            and raw_row_bound,
            "raw_check": raw_name,
            "package_source_bound": package_source_bound,
            "raw_row_bound": raw_row_bound,
        }
    checks = {
        "packaged_schema": packaged.get("schema_version") == "quickpls.packaged_acceptance.v1",
        "packaged_kind": packaged.get("kind") == config["kind"],
        "packaged_passed": packaged.get("passed") is True,
        "exact_feature_id": packaged.get("feature_id") == config["feature_id"]
        and raw.get("feature_id") == config["feature_id"],
        "exact_method_version": packaged.get("method_version") == config["method_version"]
        and raw.get("method_version") == config["method_version"],
        "exact_catalogue_snapshot": packaged.get("catalogue_snapshot_date") == "2026-08-12"
        and raw.get("catalogue_snapshot_date") == "2026-08-12",
        "exact_scoped_report_path": packaged.get("source_report") == config["raw_report"],
        "exact_scope": raw.get("focusedRun", {}).get("scope") == config["scope"],
        "timestamps_match_exactly": generated == raw_generated and completed == raw_completed,
        "timestamps_ordered": completed >= generated,
        "fresh_after_requested_boundary": generated >= not_before - timedelta(seconds=2),
        "generated_after_current_build": generated >= build_finished,
        "raw_report_passed": raw.get("passed") is True,
        "clean_packaged_failures": packaged.get("failures") == [],
        "clean_packaged_console": packaged.get("console_errors") == [],
        "clean_raw_failures": raw.get("failures") == [],
        "clean_raw_console": raw.get("consoleErrors") == [],
        "exact_package_check_set": set(package_checks) == set(config["package_checks"]),
        "all_scoped_checks_linked_and_passing": all(row["passed"] for row in linked.values()),
        "packaged_tauri_runtime": packaged.get("runtime") == "tauri-webview2-cdp",
        "loopback_acceptance_endpoint": packaged.get("endpoint") == "http://127.0.0.1:9222",
    }
    return {
        "passed": all(checks.values()),
        "checks": checks,
        "linked_checks": linked,
        "generated_at_utc": generated.isoformat().replace("+00:00", "Z"),
        "completed_at_utc": completed.isoformat().replace("+00:00", "Z"),
    }


def _visual_row_passes(method: str, row: dict[str, Any]) -> bool:
    truth = row.get("truthAndOverflow", {})
    close = row.get("closeFocus", {})
    common = (
        row.get("dialogOpened") is True
        and truth.get("noFabricatedRunState") is True
        and truth.get("noHorizontalOverflow") is True
    )
    if method == "structural_path_randomization_v1":
        return (
            common
            and row.get("pointerSelected") is True
            and row.get("linkage", {}).get("expectedKind") == "pls_permutation"
            and row.get("linkage", {}).get("linkage") is True
            and row.get("mutuallyExclusive") is True
            and row.get("distinctFromMgaAndMicom") is True
            and close.get("dialogClosed") is True
            and close.get("focusRestored") is True
        )
    if method == "logistic_regression_v2":
        return (
            common
            and row.get("fixtureApiPresent") is True
            and row.get("dataSurface") is True
            and row.get("visibleModelNodes") == 0
            and row.get("linkage", {}).get("expectedKind") == "regression"
            and row.get("linkage", {}).get("linkage") is True
            and row.get("regressionType") == "logistic"
            and row.get("startCommandDisabled") is True
            and row.get("noModelBlocker") is True
            and row.get("noPhantomResult") is True
            and close.get("dialogClosed") is True
            and close.get("focusRestored") is True
        )
    if method == "regression_bootstrap_v1":
        ols = row.get("ols", {})
        logistic = row.get("logistic", {})
        return (
            row.get("dialogOpened") is True
            and row.get("fixtureApiPresent") is True
            and row.get("dataSurface") is True
            and row.get("visibleModelNodes") == 0
            and row.get("linkage", {}).get("linkage") is True
            and row.get("bootstrap", {}).get("value") == "enabled"
            and ols.get("startCommandDisabled") is True
            and logistic.get("startCommandDisabled") is True
            and ols.get("truthAndOverflow", {}).get("noFabricatedRunState") is True
            and logistic.get("truthAndOverflow", {}).get("noFabricatedRunState") is True
            and ols.get("truthAndOverflow", {}).get("noHorizontalOverflow") is True
            and logistic.get("truthAndOverflow", {}).get("noHorizontalOverflow") is True
            and ols.get("noPhantomResult") is True
            and logistic.get("noPhantomResult") is True
            and close.get("dialogClosed") is True
            and close.get("focusRestored") is True
        )
    if method == "process_v2":
        setup = row.get("setup", {})
        accessibility = row.get("accessibility", {})
        return (
            common
            and row.get("fixtureApiPresent") is True
            and row.get("dataSurface") is True
            and row.get("regressionType") == "process"
            and setup.get("pathsExact") is True
            and setup.get("moderatorsExact") is True
            and setup.get("moderationsExact") is True
            and all(
                setup.get("stableRowIdentity", {}).get(key, {}).get("passed") is True
                for key in ("paths", "moderators", "moderations")
            )
            and accessibility.get("controlsLabeled") is True
            and accessibility.get("groupsNamed") is True
            and accessibility.get("keyboardReachable") is True
            and accessibility.get("focusRestored") is True
            and row.get("dialogBounds", {}).get("withinHorizontalViewport") is True
            and row.get("completedResult", {}).get("synthesizedByHarness") is False
        )
    raise KeyError(method)


def verify_visual_report(method: str, not_before: datetime) -> dict[str, Any]:
    config = METHODS[method]
    try:
        path = _safe_path(VISUAL_REPORT)
        report = strict_load_json(path)
        generated = _parse_utc(report.get("generatedAt"))
        rows = report.get("checks", {}).get(config["visual_key"])
        if not isinstance(rows, list):
            raise ValueError("method visual rows are missing")
        by_viewport = {row.get("viewport"): row for row in rows if isinstance(row, dict)}
        reported_viewports = {
            row.get("id"): {"width": row.get("width"), "height": row.get("height")}
            for row in report.get("viewports", [])
            if isinstance(row, dict)
        }
        screenshots = report.get("screenshots")
        if not isinstance(screenshots, list):
            raise ValueError("visual screenshots must be a list")
        screenshot_rows: list[dict[str, Any]] = []
        screenshot_paths: list[str] = []
        for viewport in EXPECTED_VIEWPORTS:
            for state in config["visual_states"]:
                matches = [
                    row
                    for row in screenshots
                    if isinstance(row, dict)
                    and row.get("viewport") == viewport
                    and row.get("state") == state
                ]
                match = matches[0] if len(matches) == 1 else None
                exact, actual = _descriptor_matches(match) if match else (False, {})
                screenshot_rows.append(
                    {"viewport": viewport, "state": state, "passed": exact, "actual": actual}
                )
                if exact:
                    screenshot_paths.append(actual["path"])
        runtime_source = _safe_path("validation/v247_native_desktop_visual_acceptance.mjs")
        source_time = datetime.fromtimestamp(runtime_source.stat().st_mtime, timezone.utc)
        checks = {
            "fresh_after_requested_boundary": generated >= not_before - timedelta(seconds=2),
            "visual_harness_not_newer_than_report": source_time <= generated + timedelta(seconds=2),
            "raw_report_passed": report.get("passed") is True,
            "production_bundle_not_mislabeled_as_tauri": report.get("harness", {}).get("actualTauriWindow") is False,
            "exact_required_viewports": reported_viewports == EXPECTED_VIEWPORTS,
            "one_method_row_per_viewport": len(rows) == 3 and set(by_viewport) == set(EXPECTED_VIEWPORTS),
            "all_method_rows_pass": all(_visual_row_passes(method, by_viewport.get(viewport, {})) for viewport in EXPECTED_VIEWPORTS),
            "all_required_screenshots_exact": all(row["passed"] for row in screenshot_rows),
            "screenshot_integrity_passes": report.get("coverage", {}).get("screenshotIntegrity", {}).get("passed") is True,
            "clean_failures": report.get("failures") == [],
            "clean_console": report.get("consoleErrors") == [],
        }
        return {
            "passed": all(checks.values()),
            "checks": checks,
            "generated_at_utc": report.get("generatedAt"),
            "report": descriptor(path),
            "screenshots": screenshot_rows,
            "artifact_paths": sorted(screenshot_paths),
        }
    except (OSError, ValueError, KeyError, TypeError) as error:
        return {"passed": False, "error": f"{type(error).__name__}: {error}"}


def verify_cumulative_receipt(method: str, not_before: datetime, packaged: dict[str, Any]) -> dict[str, Any]:
    config = METHODS[method]
    if not config["requires_cumulative"]:
        return {"passed": True, "mode": "not_required", "artifact_paths": []}
    try:
        receipt_path = _safe_path(CUMULATIVE_RECEIPT)
        report_path = _safe_path(CUMULATIVE_REPORT)
        receipt = strict_load_json(receipt_path)
        report = strict_load_json(report_path)
        started = _parse_utc(receipt.get("supervisor_started_at_utc"))
        completed = _parse_utc(receipt.get("completed_at_utc"))
        report_actual = descriptor(report_path)
        exports = receipt.get("exports")
        if not isinstance(exports, list):
            raise ValueError("cumulative exports must be a list")
        roles = [row.get("role") for row in exports if isinstance(row, dict)]
        exact_exports: dict[str, Any] = {}
        artifact_paths = [repository_path(receipt_path)]
        for role, package_key in config["cumulative_exports"].items():
            candidates = [row for row in exports if isinstance(row, dict) and row.get("role") == role]
            reported = candidates[0] if len(candidates) == 1 else None
            current, actual = _descriptor_matches(reported) if reported else (False, {})
            packaged_artifact = packaged.get("artifacts", {}).get(package_key)
            bound = current and isinstance(packaged_artifact, dict) and all(
                packaged_artifact.get(key) == actual.get(key) for key in ("path", "size", "sha256")
            )
            exact_exports[role] = {
                "passed": bound,
                "receipt": reported,
                "packaged_artifact": packaged_artifact,
            }
            if current:
                artifact_paths.append(actual["path"])
        package_generated = _parse_utc(packaged.get("generated_at_utc"))
        package_completed = _parse_utc(packaged.get("completed_at_utc"))
        supervisor_source = _safe_path("validation/run_v247_cumulative_native_acceptance.ps1")
        supervisor_mtime = datetime.fromtimestamp(supervisor_source.stat().st_mtime, timezone.utc)
        checks = {
            "schema_and_kind": receipt.get("schema_version") == 1
            and receipt.get("kind") == "quickpls_v247_cumulative_native_acceptance_receipt",
            "receipt_passed_cleanly": receipt.get("passed") is True
            and receipt.get("failures") == 0
            and receipt.get("console_errors") == 0,
            "fresh_after_requested_boundary": started >= not_before - timedelta(seconds=2),
            "timestamps_ordered": completed >= started,
            "packaged_stage_inside_supervisor_window": package_generated >= started - timedelta(seconds=2)
            and package_completed <= completed + timedelta(seconds=2),
            "supervisor_source_not_newer_than_receipt": supervisor_mtime <= started + timedelta(seconds=2),
            "exact_report_path": receipt.get("report") == CUMULATIVE_REPORT,
            "exact_report_bytes": receipt.get("report_sha256") == report_actual["sha256"]
            and receipt.get("report_size") == report_actual["size"],
            "exact_177_checks": receipt.get("checks") == 177
            and receipt.get("unique_checks") == 177
            and len(report.get("checks", {})) == 177,
            "final_scope_regression_bootstrap": receipt.get("final_scope") == "regression_bootstrap"
            and report.get("focusedRun", {}).get("scope") == "regression_bootstrap",
            "graceful_cleanup_verified": receipt.get("graceful_process_cleanup_verified") is True,
            "export_roles_unique": len(roles) == len(set(roles)),
            "method_exports_exact_and_bound": all(row["passed"] for row in exact_exports.values()),
        }
        if method == "regression_bootstrap_v1":
            scoped = descriptor(_safe_path(config["raw_report"]))
            checks["final_scoped_report_byte_identical"] = (
                scoped["size"] == report_actual["size"] and scoped["sha256"] == report_actual["sha256"]
            )
        return {
            "passed": all(checks.values()),
            "checks": checks,
            "exports": exact_exports,
            "receipt": descriptor(receipt_path),
            "report": report_actual,
            "artifact_paths": sorted(set(artifact_paths)),
        }
    except (OSError, ValueError, KeyError, TypeError) as error:
        return {"passed": False, "error": f"{type(error).__name__}: {error}"}


def verify_packaged_evidence(method: str, not_before: datetime, freshness: dict[str, Any]) -> dict[str, Any]:
    config = METHODS[method]
    try:
        packaged_path = _safe_path(config["packaged_report"])
        raw_path = _safe_path(config["raw_report"])
        packaged = strict_load_json(packaged_path)
        raw = strict_load_json(raw_path)
        build_finished = _parse_utc(freshness.get("build_finished_at_utc"))
        scoped = evaluate_scoped_documents(method, packaged, raw, not_before, build_finished)

        runtime_generated = _parse_utc(packaged.get("generated_at_utc"))
        runtime_sources: list[dict[str, Any]] = []
        for relative in config["runtime_sources"]:
            row = descriptor(_safe_path(relative), include_mtime=True)
            row["not_newer_than_report"] = datetime.fromtimestamp(
                row["mtime_ns"] / 1_000_000_000, timezone.utc
            ) <= runtime_generated + timedelta(seconds=2)
            runtime_sources.append(row)

        reported_artifacts = packaged.get("artifacts")
        if not isinstance(reported_artifacts, dict):
            raise ValueError("packaged artifacts must be an object")
        required_rows: dict[str, Any] = {}
        for key, minimum in config["required_artifacts"].items():
            value = reported_artifacts.get(key)
            count = len(value) if isinstance(value, list) else (1 if isinstance(value, dict) else 0)
            required_rows[key] = {"minimum": minimum, "observed": count, "passed": count >= minimum}

        artifact_rows: list[dict[str, Any]] = []
        artifact_paths: list[str] = []
        for reported in _collect_artifact_descriptors(reported_artifacts):
            passed, actual = _descriptor_matches(reported)
            artifact_rows.append({"passed": passed, "reported": reported, "actual": actual})
            if passed:
                artifact_paths.append(actual["path"])

        current_desktop = freshness.get("desktop", {})
        current_cli = freshness.get("release_cli", {})
        tested = packaged.get("tested_product")
        tested_desktop = tested.get("quickpls_desktop_exe") if isinstance(tested, dict) else None
        tested_cli = tested.get("qpls_cli_exe") if isinstance(tested, dict) else None
        explicit_desktop_binding = isinstance(tested_desktop, dict) and all(
            tested_desktop.get(key) == current_desktop.get(key) for key in ("path", "size", "sha256")
        )
        cli_binding = tested_cli is None or (
            isinstance(tested_cli, dict)
            and all(tested_cli.get(key) == current_cli.get(key) for key in ("path", "size", "sha256"))
        )

        cumulative = verify_cumulative_receipt(method, not_before, packaged)
        receipt_desktop_binding = (
            tested_desktop is None
            and config["requires_cumulative"]
            and cumulative.get("passed") is True
            and freshness.get("desktop_receipt_exact") is True
        )
        desktop_binding = explicit_desktop_binding or receipt_desktop_binding
        cleanup: dict[str, Any]
        if config["cleanup_mode"] == "packaged_check":
            cleanup = {
                "passed": packaged.get("checks", {}).get("cleanup", {}).get("passed") is True,
                "mode": "packaged_check",
            }
        elif config["cleanup_mode"] == "dedicated_report":
            cleanup_path = _safe_path(config["cleanup_report"])
            cleanup_report = strict_load_json(cleanup_path)
            cleanup_generated = _parse_utc(cleanup_report.get("generated_at_utc"))
            cleanup = {
                "passed": cleanup_report.get("passed") is True
                and cleanup_generated >= _parse_utc(packaged.get("completed_at_utc"))
                and cleanup_generated >= not_before - timedelta(seconds=2),
                "mode": "dedicated_report",
                "report": descriptor(cleanup_path),
            }
            artifact_paths.append(repository_path(cleanup_path))
        else:
            cleanup = {
                "passed": cumulative.get("passed") is True
                and cumulative.get("checks", {}).get("graceful_cleanup_verified") is True,
                "mode": "cumulative_receipt",
            }

        checks = {
            "scoped_report": scoped,
            "runtime_sources_frozen_before_report": {
                "passed": all(row["not_newer_than_report"] for row in runtime_sources),
                "sources": runtime_sources,
            },
            "required_artifact_roles": {
                "passed": all(row["passed"] for row in required_rows.values()),
                "roles": required_rows,
            },
            "all_reported_artifacts_exact": {
                "passed": bool(artifact_rows) and all(row["passed"] for row in artifact_rows),
                "artifacts": artifact_rows,
            },
            "tested_product": {
                "passed": desktop_binding and cli_binding,
                "desktop_exact": desktop_binding,
                "explicit_desktop_exact": explicit_desktop_binding,
                "desktop_bound_by_current_cumulative_receipt": receipt_desktop_binding,
                "cli_exact_or_not_reported": cli_binding,
                "reported": tested,
                "current_desktop": current_desktop,
                "current_cli": current_cli,
            },
            "cumulative_receipt": cumulative,
            "cleanup": cleanup,
        }
        passed = (
            scoped["passed"]
            and checks["runtime_sources_frozen_before_report"]["passed"]
            and checks["required_artifact_roles"]["passed"]
            and checks["all_reported_artifacts_exact"]["passed"]
            and checks["tested_product"]["passed"]
            and cumulative["passed"]
            and cleanup["passed"]
        )
        extras = {
            config["packaged_report"],
            config["raw_report"],
            *artifact_paths,
            *cumulative.get("artifact_paths", []),
        }
        return {"passed": passed, "checks": checks, "artifact_paths": sorted(extras)}
    except (OSError, ValueError, KeyError, TypeError) as error:
        return {"passed": False, "error": f"{type(error).__name__}: {error}"}


def evaluate_audit_inputs(method: str, document: dict[str, Any] | None = None) -> dict[str, Any]:
    """Verify every prior stage plus packaged acceptance, never the audit itself."""

    document = document or manifest(method)
    feature = document["feature"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    rows: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
            rows.append(
                {
                    "stage": stage,
                    "roles": artifact["roles"],
                    "path": artifact["path"],
                    "passed": passed,
                    "errors": errors,
                }
            )
    release_artifacts = document["qualification"]["evidence"]["release_qualified"]
    release_roles = [
        role for artifact in release_artifacts for role in artifact.get("roles", [])
    ]
    release_contract = (
        len(release_artifacts) == 2
        and sorted(release_roles) == ["method_audit", "packaged_acceptance"]
        and all(len(artifact.get("roles", [])) == 1 for artifact in release_artifacts)
    )
    packaged = [
        artifact
        for artifact in release_artifacts
        if artifact.get("roles") == ["packaged_acceptance"]
    ]
    if len(packaged) != 1:
        rows.append(
            {
                "stage": "release_qualified",
                "roles": ["packaged_acceptance"],
                "passed": False,
                "errors": ["release evidence must contain exactly one packaged_acceptance artifact"],
            }
        )
        return {
            "passed": False,
            "release_evidence_contract_passes": release_contract,
            "stage_artifacts": rows,
            "semantic_checks": {},
            "audit_output_not_consumed": True,
        }
    packaged_artifact = packaged[0]
    packaged_passed, packaged_errors = _verify_artifact(
        packaged_artifact, document, ROOT, expected_identity
    )
    rows.append(
        {
            "stage": "release_qualified",
            "roles": packaged_artifact["roles"],
            "path": packaged_artifact["path"],
            "passed": packaged_passed,
            "errors": packaged_errors,
        }
    )
    report = strict_load_json(_safe_path(packaged_artifact["path"]))
    checks = report.get("checks", {})
    required = {
        "prior_factory_stages",
        "source_freshness",
        "native",
        "responsive_viewports",
        "runner_cleanup_verified",
    }
    semantic = {
        "exact_packaged_check_set": set(checks) == required,
        "prior_factory_stages_passed": checks.get("prior_factory_stages", {}).get("passed") is True,
        "source_freshness_passed": checks.get("source_freshness", {}).get("passed") is True
        and checks.get("source_freshness", {}).get("source_stable_during_gate") is True,
        "native_packaged_evidence_passed": checks.get("native", {}).get("passed") is True,
        "responsive_viewports_passed": checks.get("responsive_viewports", {}).get("passed") is True,
        "runner_cleanup_verified": checks.get("runner_cleanup_verified") is True,
    }
    audit_output_not_consumed = all(
        "method_audit" not in row.get("roles", []) for row in rows
    )
    return {
        "passed": release_contract
        and all(row["passed"] for row in rows)
        and all(semantic.values())
        and audit_output_not_consumed,
        "release_evidence_contract_passes": release_contract,
        "stage_artifacts": rows,
        "semantic_checks": semantic,
        "audit_output_not_consumed": audit_output_not_consumed,
    }


def audit_main(method: str) -> int:
    result = evaluate_audit_inputs(method)
    prerequisite_paths = [
        row["path"] for row in result.get("stage_artifacts", []) if row.get("path")
    ]
    report = write_identity_report(
        method,
        "method_audit",
        passed=result["passed"],
        checks=result,
        extras=[METHODS[method]["audit"], *prerequisite_paths],
    )
    print(f"wrote {report} | passed={result['passed']}")
    return 0 if result["passed"] else 1


def _run_release_sources(method: str) -> tuple[bool, list[dict[str, Any]]]:
    config = METHODS[method]
    executions: list[dict[str, Any]] = []
    runtime_command = (
        [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            "validation/run_v247_cumulative_native_acceptance.ps1",
        ]
        if config["requires_cumulative"]
        else [
            "powershell",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            config["runtime_script"],
        ]
    )
    for command, timeout in (
        (runtime_command, 7200),
        (["npm.cmd", "run", "qpls:v247:native-desktop-visual"], 3600),
    ):
        completed, execution = run_command(command, timeout=timeout)
        executions.append(execution)
        if completed.returncode != 0:
            return False, executions
    return True, executions


def packaged_main(method: str, argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-run", action="store_true")
    parser.add_argument(
        "--not-before-utc",
        help="Required with --skip-run; rejects stale scoped, visual, and cumulative evidence.",
    )
    args = parser.parse_args(argv)
    if args.skip_run and not args.not_before_utc:
        parser.error("--skip-run requires --not-before-utc")
    started = _parse_utc(args.not_before_utc) if args.not_before_utc else datetime.now(timezone.utc)

    prior = verify_prior_factory_stages(method)
    before = source_freshness(method)
    if not prior["passed"] or not before["passed"]:
        print(json.dumps({"prior_factory_stages": prior, "source_freshness": before}, indent=2))
        return 1

    executions: list[dict[str, Any]] = []
    if not args.skip_run:
        ran, executions = _run_release_sources(method)
        if not ran:
            print(json.dumps({"phase": "runtime", "executions": executions}, indent=2))
            return 1

    native = verify_packaged_evidence(method, started, before)
    visual = verify_visual_report(method, started)
    after = source_freshness(method)
    stable = before == after
    detail = {
        "prior_factory_stages": prior,
        "source_freshness": {
            "passed": before["passed"] and after["passed"] and stable,
            "source_stable_during_gate": stable,
            "before": before,
            "after": after,
        },
        "native": native,
        "responsive_viewports": visual,
        "runner_cleanup_verified": native.get("checks", {}).get("cleanup", {}).get("passed") is True,
    }
    detail["passed"] = (
        prior["passed"]
        and detail["source_freshness"]["passed"]
        and native["passed"]
        and visual["passed"]
        and detail["runner_cleanup_verified"]
    )
    if not detail["passed"]:
        print(json.dumps(detail, indent=2))
        return 1

    extras = {
        SOURCE,
        FOCUSED_TEST,
        METHODS[method]["adapter"],
        METHODS[method]["audit"],
        BUILD_RECEIPT,
        DESKTOP,
        RELEASE_CLI,
        VISUAL_REPORT,
        *METHODS[method]["runtime_sources"],
        *native["artifact_paths"],
        *visual["artifact_paths"],
    }
    report = write_identity_report(
        method,
        "packaged_acceptance",
        passed=True,
        checks={key: value for key, value in detail.items() if key != "passed"},
        extras=sorted(extras),
        execution={"commands": executions},
    )
    print(f"wrote {report} | passed=True")

    completed, execution = run_command(
        [sys.executable, METHODS[method]["audit"]], timeout=600
    )
    if completed.returncode != 0:
        print(json.dumps({"phase": "method_audit", "execution": execution}, indent=2))
        return 1
    final = validate_manifest(_safe_path(METHODS[method]["manifest"]), ROOT)
    if final.get("passed") is not True or final.get("derived_state") != "release_qualified":
        print(json.dumps({"phase": "final_manifest", "manifest_validation": final}, indent=2))
        return 1
    print(f"{method} derived state: release_qualified")
    return 0
