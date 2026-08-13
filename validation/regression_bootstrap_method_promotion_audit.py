#!/usr/bin/env python3
"""Fail-closed promotion audit for regression bootstrap v1.

The audit consumes current, method-specific numerical and packaged evidence.
It does not run Cargo, build the desktop application, invoke the CLI, or infer
success from file presence. Missing/stale evidence, a generic native report,
an identity mismatch, altered arithmetic, or an artifact digest mismatch keeps
promotion false.
"""

from __future__ import annotations

import json
import hashlib
import math
import re
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from promotion_audit_integrity import (
    RESULTS,
    ROOT,
    evaluate_document,
    evaluate_report,
    json_value,
    load_json,
    sha256_file,
)


FEATURE_ID = "qpls3.standalone.regression_bootstrap"
METHOD_VERSION = "regression_bootstrap_v1"
WITNESS_VERSION = "regression_bootstrap_validation_witness_v1"
REGRESSION_BOOTSTRAP_DEFAULT_TABLE = "regression_bootstrap_summary"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
TARGET = "quickpls3_regression_bootstrap_v1_promotion"
REFERENCE_TARGET = "regression_bootstrap_v1_reference"
REFERENCE_REPORT_NAME = "regression_bootstrap_v1_reference_report.json"
PACKAGED_REPORT_NAME = "regression_bootstrap_v1_packaged_acceptance.json"
VISUAL_REPORT_NAME = "v247_native_desktop_visual_acceptance.json"
PROCESS_CLEANUP_REPORT_NAME = "v247_regression_bootstrap_process_cleanup.json"
FAILURE_BOUNDARY_REPORT_NAME = "regression_bootstrap_failure_boundary_test_report.json"
FAILURE_BOUNDARY_TARGET = "regression_bootstrap_v1_failure_boundary_rust_tests"
FAILURE_BOUNDARY_GENERATOR = "validation/regression_bootstrap_failure_boundary_gate.py"
PACKAGED_SOURCE_REPORT = "validation/results/v247_tauri_native_acceptance_regression_bootstrap.json"
OUTPUT_NAME = "regression_bootstrap_method_promotion_audit.json"
PACKAGED_KIND = "quickpls3_scoped_tauri_regression_bootstrap_v1_acceptance"
PACKAGED_SCHEMA_VERSION = "quickpls.packaged_acceptance.v1"
PACKAGED_PLATFORM_TARGET = "windows_10_11_x64_packaged_tauri"
PACKAGED_RUNTIME = "tauri-webview2-cdp"
PACKAGED_GENERATOR = "validation/v247_tauri_native_acceptance.mjs"
VISUAL_VIEWPORTS = frozenset({"1024x700", "1280x720", "1440x900"})
FAILURE_BOUNDARY_TEST_NAMES = frozenset(
    {
        "regression_bootstrap_failure_boundary_listwise_complete_cases_are_the_only_sampling_frame",
        "regression_bootstrap_failure_boundary_captures_zero_based_single_class_replicates",
        "regression_bootstrap_failure_boundary_rejects_below_ninety_percent_usable",
        "regression_bootstrap_failure_boundary_real_delete_one_failure_disables_all_bca",
    }
)
ARCHIVE_BOUNDARY_TEST_NAMES = frozenset(
    {
        "regression_bootstrap_json_roundtrip_tolerance_is_narrow",
        "regression_bootstrap_append_save_reopen_and_tamper_contract_are_atomic",
    }
)
FAILURE_BOUNDARY_PRODUCT_SOURCE_PATHS = [
    "crates/qpls-resampling/src/lib.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-core/src/contract.rs",
]
ARCHIVE_BOUNDARY_PRODUCT_SOURCE_PATHS = [
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-core/src/contract.rs",
]
EXACT_TOLERANCE = 5e-11
POINT_TOLERANCE = 2e-6

REFERENCE_CHECK_NAMES = frozenset(
    {
        "frozen_supplied_type7_bca_normal_ratio_or",
        "quickpls_ols_exact_contract",
        "quickpls_logistic_exact_contract",
        "exact_witness_arithmetic_ols",
        "exact_witness_arithmetic_logistic",
        "witness_index_partitions_ols",
        "witness_index_partitions_logistic",
        "independent_python_ols_resampling",
        "independent_python_logistic_resampling",
        "external_r_ols_resampling",
        "external_r_logistic_resampling",
        "deterministic_worker_invariant_ols",
        "deterministic_worker_invariant_logistic",
    }
)
PACKAGED_CHECK_NAMES = frozenset(
    {
        "workflow",
        "results",
        "ols_export",
        "logistic_export",
        "save_reopen",
        "cancellation",
        "witness_boundary",
    }
)

REFERENCE_SOURCE_PATHS = [
    "validation/regression_bootstrap_v1_reference.py",
    "validation/regression_bootstrap_v1_reference.R",
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-resampling/src/lib.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-runner/src/lib.rs",
]
REFERENCE_PRODUCT_SOURCE_PATHS = [
    path for path in REFERENCE_SOURCE_PATHS if path.startswith("crates/")
]
PACKAGED_VALIDATION_SOURCE_PATHS = [
    PACKAGED_GENERATOR,
    "validation/regression_bootstrap_v1_packaged_acceptance.schema.json",
    "validation/run_v247_regression_bootstrap_native_acceptance.ps1",
]
PACKAGED_PRODUCT_SOURCE_PATHS = [
    "src/native/NativeCalculationDialog.tsx",
    "src/native/nativeAnalysisCatalog.ts",
    "src/native/nativeAnalysisRecipe.ts",
    "src/native/nativeCalculationRequest.ts",
    "src/native/nativeRegressionBootstrapWitness.ts",
    "src/native/nativeOls.ts",
    "src/native/nativeLogistic.ts",
    "src/native/nativeResults.ts",
    "src/native/nativeExportTables.ts",
    "src/native/NativeDesktopController.tsx",
    "src/services/projectService.ts",
    "src-tauri/src/lib.rs",
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-resampling/src/lib.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-runner/src/lib.rs",
]
PACKAGED_SOURCE_PATHS = [*PACKAGED_VALIDATION_SOURCE_PATHS, *PACKAGED_PRODUCT_SOURCE_PATHS]
VISUAL_SOURCE_PATHS = [
    "validation/v247_native_desktop_visual_acceptance.mjs",
    "src/native/NativeCalculationDialog.tsx",
    "src/native/nativeAnalysisCatalog.ts",
    "src/native/nativeAnalysisRecipe.ts",
    "src/native/nativeOls.ts",
    "src/native/nativeLogistic.ts",
    "src/native/nativeResults.ts",
    "src/native/nativeDesktop.css",
]

OLS_SCOPE_NOTE = (
    "Raw numeric ordinary least squares with an intercept, listwise deletion, HC3 robust standard errors, "
    "and fixed two-sided 95% confidence intervals. Optional regression case-resampling reports "
    "percentile-primary and conditional BCa inference. Categorical encoding, weights, clusters, generic PLS "
    "resampling, logistic regression, and PROCESS models are not included."
)
LOGISTIC_SCOPE_NOTE = (
    "Binary logistic regression with an intercept, raw numeric predictors, listwise deletion, deterministic "
    "maximum-likelihood estimation, Wald inference, odds ratios, fitted probabilities, and fixed two-sided "
    "95% confidence intervals. Optional regression case-resampling reports percentile-primary and conditional "
    "BCa coefficient and odds-ratio inference. The outcome must be coded exactly 0/1. Multinomial, ordinal, "
    "weighted, clustered, penalized, generic PLS resampling, and Firth-corrected models are not included."
)
BOOTSTRAP_SCOPE_NOTE = (
    "10,000 resamples are recommended for final results; 1,000 can be used for exploratory runs. Percentile "
    "intervals are primary. BCa is reported when delete-one refits support it, otherwise an explicit reason is "
    "shown. Fixed two-sided 95% inference; studentized intervals, one-tailed tests, and custom alpha are excluded. "
    "Runtime grows with resamples. Indexed seeded streams make results deterministic and worker-invariant."
)


def _load_optional(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        # Windows PowerShell 5 writes UTF-8 JSON with a BOM by default. The
        # cleanup report is still valid JSON and remains hash/freshness bound;
        # accept only that encoding variant rather than treating it as absent.
        value = json.loads(path.read_text(encoding="utf-8-sig"))
        return value if isinstance(value, dict) else {}
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError):
        return {}


def _exact_check_names(path: Path, expected: frozenset[str]) -> dict[str, Any]:
    document = _load_optional(path)
    checks = document.get("checks")
    observed = frozenset(checks) if isinstance(checks, dict) else frozenset()
    return {
        "expected": sorted(expected),
        "observed": sorted(observed),
        "passed": observed == expected,
    }


def _finite_number(value: Any) -> bool:
    return (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and math.isfinite(float(value))
    )


def _reference_attestation(root: Path, path: Path) -> dict[str, Any]:
    document = _load_optional(path)
    exact = document.get("exact_arithmetic")
    python = document.get("independent_python")
    external_r = document.get("external_r")
    declared_thresholds = document.get("comparison_thresholds")
    expected_thresholds = {
        "mean_pooled_se_units": 0.35,
        "standard_error_relative": 0.30,
        "percentile_endpoint_pooled_se_units": 0.75,
        "bca_endpoint_pooled_se_units": 0.90,
        "odds_ratio_log_endpoint_pooled_se_units": 0.75,
    }
    tested_cli = _binary_attestation(
        root,
        json_value(document, "artifacts.tested_cli", None),
        expected_path="target/release/qpls.exe",
        source_paths=REFERENCE_PRODUCT_SOURCE_PATHS,
    )

    def exact_delta_ok(value: Any) -> bool:
        return _finite_number(value) and 0.0 <= float(value) <= EXACT_TOLERANCE

    def comparison_ok(container: Any, model: str) -> bool:
        if not isinstance(container, dict):
            return False
        comparison = json_value(container, f"distribution_comparisons.{model}", None)
        if not isinstance(comparison, dict) or comparison.get("passed") is not True:
            return False
        thresholds = comparison.get("thresholds")
        observed = comparison.get("observed_maxima")
        checks = comparison.get("threshold_checks")
        expected = dict(expected_thresholds)
        if model == "ols":
            expected.pop("odds_ratio_log_endpoint_pooled_se_units")
        return (
            thresholds == expected
            and isinstance(observed, dict)
            and isinstance(checks, dict)
            and set(observed) == set(expected)
            and set(checks) == set(expected)
            and all(checks.get(name) is True for name in expected)
            and all(
                _finite_number(observed.get(name))
                and 0.0 <= float(observed[name]) <= threshold
                for name, threshold in expected.items()
            )
        )

    point_deltas = (
        python.get("point_maximum_absolute_difference")
        if isinstance(python, dict)
        else None
    )
    checks = {
        "exact_tolerance_is_frozen": (
            isinstance(exact, dict)
            and exact.get("tolerance") == EXACT_TOLERANCE
            and exact_delta_ok(exact.get("maximum_absolute_difference"))
            and exact_delta_ok(json_value(exact, "frozen_supplied.maximum_absolute_difference", None))
            and exact_delta_ok(json_value(exact, "ols.maximum_absolute_difference", None))
            and exact_delta_ok(json_value(exact, "logistic.maximum_absolute_difference", None))
        ),
        "independent_python_point_estimates_within_tolerance": (
            isinstance(python, dict)
            and python.get("point_tolerance") == POINT_TOLERANCE
            and isinstance(point_deltas, dict)
            and set(point_deltas) == {"ols", "logistic"}
            and all(
                _finite_number(value) and 0.0 <= float(value) <= POINT_TOLERANCE
                for value in point_deltas.values()
            )
        ),
        "declared_distribution_thresholds_are_frozen": declared_thresholds
        == expected_thresholds,
        "independent_python_ols_distribution_within_thresholds": comparison_ok(
            python, "ols"
        ),
        "independent_python_logistic_distribution_within_thresholds": comparison_ok(
            python, "logistic"
        ),
        "external_r_identified": (
            isinstance(external_r, dict)
            and external_r.get("available") is True
            and external_r.get("passed") is True
            and isinstance(external_r.get("version"), str)
            and bool(external_r["version"].strip())
        ),
        "external_r_ols_distribution_within_thresholds": comparison_ok(
            external_r, "ols"
        ),
        "external_r_logistic_distribution_within_thresholds": comparison_ok(
            external_r, "logistic"
        ),
        "tested_release_cli_is_current_and_bound": tested_cli["passed"],
    }
    return {"checks": checks, "tested_cli": tested_cli, "passed": all(checks.values())}


def _artifact_attestation(root: Path, descriptor: Any) -> dict[str, Any]:
    evidence: dict[str, Any] = {
        "path": descriptor.get("path") if isinstance(descriptor, dict) else None,
        "reported_size": descriptor.get("size") if isinstance(descriptor, dict) else None,
        "reported_sha256": descriptor.get("sha256") if isinstance(descriptor, dict) else None,
        "present": False,
        "inside_repository": False,
        "actual_size": None,
        "actual_sha256": None,
        "passed": False,
    }
    relative = evidence["path"]
    size = evidence["reported_size"]
    digest = evidence["reported_sha256"]
    if (
        not isinstance(relative, str)
        or not relative.strip()
        or not isinstance(size, int)
        or isinstance(size, bool)
        or size <= 0
        or not isinstance(digest, str)
        or re.fullmatch(r"[0-9a-f]{64}", digest) is None
    ):
        return evidence
    repository = root.resolve()
    artifact = (repository / relative).resolve()
    try:
        artifact.relative_to(repository)
    except ValueError:
        return evidence
    evidence["inside_repository"] = True
    if not artifact.is_file():
        return evidence
    evidence["present"] = True
    evidence["actual_size"] = artifact.stat().st_size
    evidence["actual_sha256"] = sha256_file(artifact)
    evidence["passed"] = (
        evidence["actual_size"] == size and evidence["actual_sha256"] == digest
    )
    return evidence


def _binary_attestation(
    root: Path,
    descriptor: Any,
    *,
    expected_path: str,
    source_paths: list[str],
) -> dict[str, Any]:
    artifact = _artifact_attestation(root, descriptor)
    binary = root / expected_path
    source_freshness = []
    for relative in source_paths:
        source = root / relative
        source_freshness.append(
            {
                "path": relative,
                "present": source.is_file(),
                "binary_not_older": (
                    binary.is_file()
                    and source.is_file()
                    and binary.stat().st_mtime_ns >= source.stat().st_mtime_ns
                ),
            }
        )
    checks = {
        "exact_path": artifact["path"] == expected_path,
        "digest_and_size_match": artifact["passed"],
        "not_older_than_sources": bool(source_freshness)
        and all(item["present"] and item["binary_not_older"] for item in source_freshness),
    }
    return {
        "artifact": artifact,
        "source_freshness": source_freshness,
        "checks": checks,
        "passed": all(checks.values()),
    }


def _directory_manifest(root: Path, relative: str) -> dict[str, Any]:
    directory = root / relative
    # Path ordering on Windows is case-insensitive, whereas the Node evidence
    # generator sorts normalized POSIX path strings. Bind both sides to the
    # same case-sensitive canonical order before hashing the manifest.
    files = (
        sorted(
            (path for path in directory.rglob("*") if path.is_file()),
            key=lambda path: path.relative_to(directory).as_posix(),
        )
        if directory.is_dir()
        else []
    )
    manifest = []
    total_size = 0
    digest = hashlib.sha256()
    for file in files:
        contents_digest = sha256_file(file)
        size = file.stat().st_size
        child = file.relative_to(directory).as_posix()
        manifest.append({"path": child, "size": size, "sha256": contents_digest})
        total_size += size
        digest.update(f"{child}\0{size}\0{contents_digest}\n".encode("utf-8"))
    return {
        "path": relative,
        "size": total_size,
        "file_count": len(manifest),
        "sha256": digest.hexdigest() if manifest else None,
        "manifest": manifest,
    }


def _dist_bundle_attestation(root: Path, descriptor: Any) -> dict[str, Any]:
    actual = _directory_manifest(root, "dist")
    expected = descriptor if isinstance(descriptor, dict) else {}
    checks = {
        "exact_path": expected.get("path") == "dist",
        "nonempty_bundle": actual["file_count"] > 0,
        "size_matches": expected.get("size") == actual["size"],
        "file_count_matches": expected.get("file_count") == actual["file_count"],
        "digest_matches": expected.get("sha256") == actual["sha256"],
        "manifest_matches": expected.get("manifest") == actual["manifest"],
    }
    return {"reported": expected, "actual": actual, "checks": checks, "passed": all(checks.values())}


def _timestamp(value: Any) -> datetime | None:
    if not isinstance(value, str) or not value.strip():
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        return None
    return parsed if parsed.tzinfo is not None else None


def _packaged_attestation(root: Path, path: Path) -> dict[str, Any]:
    document = _load_optional(path)
    raw_artifacts = document.get("artifacts")
    artifacts = raw_artifacts if isinstance(raw_artifacts, dict) else {}
    ols_xlsx = _artifact_attestation(root, artifacts.get("ols_xlsx"))
    logistic_xlsx = _artifact_attestation(root, artifacts.get("logistic_xlsx"))
    project_archive = _artifact_attestation(root, artifacts.get("project_archive"))
    screenshot_descriptors = artifacts.get("screenshots")
    screenshots = (
        [_artifact_attestation(root, item) for item in screenshot_descriptors]
        if isinstance(screenshot_descriptors, list)
        else []
    )
    check_rows = document.get("checks") if isinstance(document.get("checks"), dict) else {}
    results_check = check_rows.get("results") if isinstance(check_rows.get("results"), dict) else {}
    ols_export = check_rows.get("ols_export") if isinstance(check_rows.get("ols_export"), dict) else {}
    logistic_export = check_rows.get("logistic_export") if isinstance(check_rows.get("logistic_export"), dict) else {}
    save_reopen = check_rows.get("save_reopen") if isinstance(check_rows.get("save_reopen"), dict) else {}
    generated = _timestamp(document.get("generated_at_utc"))
    completed = _timestamp(document.get("completed_at_utc"))
    all_artifacts = [ols_xlsx, logistic_xlsx, project_archive, *screenshots]
    paths = [item["path"] for item in all_artifacts if item["path"] is not None]
    checks = {
        "ordered_utc_timestamps": generated is not None
        and completed is not None
        and completed >= generated,
        "ols_xlsx_matches_disk": ols_xlsx["passed"],
        "logistic_xlsx_matches_disk": logistic_xlsx["passed"],
        "project_archive_matches_disk": project_archive["passed"],
        "screenshot_artifacts_match_disk": len(screenshots) >= 6
        and all(item["passed"] for item in screenshots),
        "artifact_paths_are_unique": len(paths) == len(set(paths)) == 3 + len(screenshots),
        "ols_export_digest_is_bound": ols_export.get("artifact_sha256")
        == ols_xlsx["reported_sha256"],
        "logistic_export_digest_is_bound": logistic_export.get("artifact_sha256")
        == logistic_xlsx["reported_sha256"],
        "archive_digest_is_bound": save_reopen.get("archive_sha256")
        == project_archive["reported_sha256"],
        "bootstrap_initial_result_defaults_are_summary": (
            results_check.get("ols_initial_selected_table") == REGRESSION_BOOTSTRAP_DEFAULT_TABLE
            and results_check.get("logistic_initial_selected_table") == REGRESSION_BOOTSTRAP_DEFAULT_TABLE
        ),
        "bootstrap_reopen_defaults_are_summary": (
            save_reopen.get("ols_initial_selected_table") == REGRESSION_BOOTSTRAP_DEFAULT_TABLE
            and save_reopen.get("logistic_initial_selected_table") == REGRESSION_BOOTSTRAP_DEFAULT_TABLE
        ),
        "ols_witness_scan_fail_closed": (
            isinstance(ols_export.get("witness_scan"), dict)
            and ols_export["witness_scan"].get("passed") is True
            and ols_export["witness_scan"].get("extraction_errors") == []
            and bool(ols_export["witness_scan"].get("worksheet_members"))
            and not ols_export["witness_scan"].get("forbidden_matches")
        ),
        "logistic_witness_scan_fail_closed": (
            isinstance(logistic_export.get("witness_scan"), dict)
            and logistic_export["witness_scan"].get("passed") is True
            and logistic_export["witness_scan"].get("extraction_errors") == []
            and bool(logistic_export["witness_scan"].get("worksheet_members"))
            and not logistic_export["witness_scan"].get("forbidden_matches")
        ),
    }
    tested_product = document.get("tested_product") if isinstance(document.get("tested_product"), dict) else {}
    desktop = _binary_attestation(
        root,
        tested_product.get("quickpls_desktop_exe"),
        expected_path="target/release/quickpls-desktop.exe",
        source_paths=[
            *PACKAGED_PRODUCT_SOURCE_PATHS,
            *(path for path in _visual_source_paths(root) if path.startswith("dist/")),
        ],
    )
    dist_bundle = _dist_bundle_attestation(root, tested_product.get("dist_bundle"))
    cleanup_path = path.parent / PROCESS_CLEANUP_REPORT_NAME
    cleanup = _load_optional(cleanup_path)
    cleanup_fresh = cleanup_path.is_file() and path.is_file() and cleanup_path.stat().st_mtime_ns >= path.stat().st_mtime_ns
    cleanup_check = {
        "path": str(cleanup_path.relative_to(root)),
        "fresh_after_packaged_report": cleanup_fresh,
        "launched_pid": cleanup.get("launched_pid"),
        "parent_exit_confirmed": cleanup.get("parent_exit_confirmed"),
        "lingering_descendant_pids": cleanup.get("lingering_descendant_pids"),
        "passed": (
            cleanup.get("passed") is True
            and isinstance(cleanup.get("launched_pid"), int)
            and not isinstance(cleanup.get("launched_pid"), bool)
            and cleanup["launched_pid"] > 0
            and cleanup.get("parent_exit_confirmed") is True
            and cleanup.get("lingering_descendant_pids") == []
            and cleanup_fresh
        ),
    }
    checks["tested_desktop_binary_is_current_and_bound"] = desktop["passed"]
    checks["tested_dist_bundle_is_exactly_bound"] = dist_bundle["passed"]
    checks["exact_pid_cleanup_confirmed"] = cleanup_check["passed"]
    return {
        "checks": checks,
        "artifacts": {
            "ols_xlsx": ols_xlsx,
            "logistic_xlsx": logistic_xlsx,
            "project_archive": project_archive,
            "screenshots": screenshots,
        },
        "tested_product": {
            "quickpls_desktop_exe": desktop,
            "dist_bundle": dist_bundle,
        },
        "process_cleanup": cleanup_check,
        "passed": all(checks.values()),
    }


def _visual_source_paths(root: Path) -> list[str]:
    dist = root / "dist"
    dist_files = sorted(path.relative_to(root).as_posix() for path in dist.rglob("*") if path.is_file()) if dist.is_dir() else []
    return [*VISUAL_SOURCE_PATHS, *dist_files]


def _visual_attestation(root: Path, path: Path) -> dict[str, Any]:
    document = _load_optional(path)
    rows = json_value(document, "checks.regressionBootstrap", [])
    rows = rows if isinstance(rows, list) else []
    console_errors = document.get("consoleErrors")
    failures = document.get("failures")
    expected_types = [
        {"value": "ols", "label": "Ordinary least squares"},
        {"value": "logistic", "label": "Binary logistic (outcome coded 0/1)"},
    ]
    expected_bootstrap = [
        {"value": "off", "label": "Off"},
        {"value": "enabled", "label": "Case-resampling bootstrap"},
    ]

    def row_passes(row: Any) -> bool:
        if not isinstance(row, dict):
            return False
        accessibility = row.get("accessibility")
        bootstrap = row.get("bootstrap")
        ols = row.get("ols")
        logistic = row.get("logistic")
        close_focus = row.get("closeFocus")
        completed = row.get("completedResult")
        if not all(isinstance(value, dict) for value in (accessibility, bootstrap, ols, logistic, close_focus, completed)):
            return False
        expected_accessibility = {
            "labeledRegressionType": 1,
            "labeledOutcome": 1,
            "labeledBootstrapToggle": 1,
            "labeledSamples": 1,
            "labeledWorkers": 1,
            "labeledSeed": 1,
            "predictorGroup": 1,
            "controlGroup": 1,
            "distinctControlIds": 6,
        }
        ols_truth = ols.get("truthAndOverflow", {})
        logistic_truth = logistic.get("truthAndOverflow", {})
        ols_bounds = ols.get("dialogBounds", {})
        logistic_bounds = logistic.get("dialogBounds", {})
        return (
            row.get("fixture") == {"variables": 5, "models": 0}
            and row.get("dataSurface") is True
            and row.get("visibleModelNodes") == 0
            and row.get("analyzeCommandCount") == 1
            and row.get("dialogOpened") is True
            and row.get("catalogCount") == 14
            and row.get("selectedMethod") == "Regression"
            and json_value(row, "linkage.linkage", False) is True
            and row.get("category") == "Standalone analysis"
            and row.get("regressionTypeOptions") == expected_types
            and row.get("outcome") == "outcome"
            and json_value(row, "roles.selectedPredictors", None) == ["predictor"]
            and json_value(row, "roles.selectedControls", None) == ["control"]
            and bootstrap.get("value") == "enabled"
            and bootstrap.get("options") == expected_bootstrap
            and bootstrap.get("samples") == {"count": 1, "value": "10000", "min": "99", "max": "10000", "step": "1"}
            and json_value(bootstrap, "workers.value", None) == "1"
            and json_value(bootstrap, "seed.value", None) == "20260718"
            and bootstrap.get("scope") == BOOTSTRAP_SCOPE_NOTE
            and bootstrap.get("toggleFocused") is True
            and accessibility == expected_accessibility
            and ols.get("type") == "ols"
            and ols.get("validatedScope") == OLS_SCOPE_NOTE
            and json_value(ols, "blockers.runtime", []) and len(json_value(ols, "blockers.runtime", [])) == 1
            and json_value(ols, "blockers.unexpected", None) == []
            and json_value(ols, "blockers.model", None) == []
            and ols.get("startCommandCount") == 1
            and ols.get("startCommandDisabled") is True
            and logistic.get("type") == "logistic"
            and logistic.get("validatedScope") == LOGISTIC_SCOPE_NOTE
            and logistic.get("bootstrapScope") == BOOTSTRAP_SCOPE_NOTE
            and logistic.get("bootstrapValue") == "enabled"
            and logistic.get("samples") == "10000"
            and logistic.get("workers") == "1"
            and logistic.get("seed") == "20260718"
            and logistic.get("typeFocused") is True
            and json_value(logistic, "profile.role", None) == "status"
            and json_value(logistic, "profile.ariaLive", None) == "polite"
            and json_value(logistic, "profile.ariaBusy", None) == "false"
            and len(json_value(logistic, "blockers.runtime", [])) == 1
            and len(json_value(logistic, "blockers.allowedFixtureProfile", [])) == 3
            and json_value(logistic, "blockers.unexpected", None) == []
            and json_value(logistic, "blockers.model", None) == []
            and logistic.get("startCommandCount") == 1
            and logistic.get("startCommandDisabled") is True
            and ols_truth.get("noFabricatedRunState") is True
            and ols_truth.get("noHorizontalOverflow") is True
            and logistic_truth.get("noFabricatedRunState") is True
            and logistic_truth.get("noHorizontalOverflow") is True
            and ols_bounds.get("withinHorizontalViewport") is True
            and ols_bounds.get("pageHorizontalOverflow") is False
            and logistic_bounds.get("withinHorizontalViewport") is True
            and logistic_bounds.get("pageHorizontalOverflow") is False
            and ols.get("noPhantomResult") is True
            and logistic.get("noPhantomResult") is True
            and close_focus.get("dialogClosed") is True
            and close_focus.get("focusRestored") is True
            and completed.get("synthesizedByHarness") is False
        )

    visual_screens = [
        item for item in document.get("screenshots", [])
        if isinstance(item, dict)
        and item.get("state") in {"regression-bootstrap-ols-dialog", "regression-bootstrap-logistic-dialog"}
    ] if isinstance(document.get("screenshots"), list) else []
    expected_screen_keys = {
        (viewport, state)
        for viewport in VISUAL_VIEWPORTS
        for state in ("regression-bootstrap-ols-dialog", "regression-bootstrap-logistic-dialog")
    }
    observed_screen_keys = {(item.get("viewport"), item.get("state")) for item in visual_screens}
    screen_files = []
    for item in visual_screens:
        reported_path = item.get("path")
        file = Path(reported_path) if isinstance(reported_path, str) else Path()
        if reported_path and not file.is_absolute():
            file = root / file
        screen_files.append(bool(reported_path) and file.is_file() and file.stat().st_size > 0)
    checks = {
        "overall_passed": document.get("passed") is True,
        "zero_failures": failures == [],
        "zero_console_errors": console_errors == [],
        "exact_three_viewport_rows": len(rows) == 3
        and {row.get("viewport") for row in rows if isinstance(row, dict)} == VISUAL_VIEWPORTS,
        "all_method_contracts_pass": len(rows) == 3 and all(row_passes(row) for row in rows),
        "exact_screenshot_matrix_and_files": len(visual_screens) == 6
        and observed_screen_keys == expected_screen_keys
        and all(screen_files),
        "dist_sources_present": any(source.startswith("dist/") for source in _visual_source_paths(root)),
    }
    return {"checks": checks, "rows": rows, "screenshots": visual_screens, "passed": all(checks.values())}


def _failure_boundary_source_attestation(root: Path) -> dict[str, Any]:
    source_path = root / "crates" / "qpls-resampling" / "src" / "lib.rs"
    try:
        source = source_path.read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        source = ""
    observed = sorted(name for name in FAILURE_BOUNDARY_TEST_NAMES if name in source)
    checks = {
        "all_named_real_engine_boundaries_present": frozenset(observed) == FAILURE_BOUNDARY_TEST_NAMES,
        "failed_replicate_reason_is_frozen": "single_class_resample" in source,
        "zero_based_failed_index_fixture_is_frozen": "vec![7]" in source or "[7]" in source,
    }
    return {"expected": sorted(FAILURE_BOUNDARY_TEST_NAMES), "observed": observed, "checks": checks, "passed": all(checks.values())}


def _failure_boundary_spec() -> dict[str, Any]:
    return {
        "name": FAILURE_BOUNDARY_REPORT_NAME,
        "max_age_days": 2,
        "source_paths": [
            FAILURE_BOUNDARY_GENERATOR,
            *FAILURE_BOUNDARY_PRODUCT_SOURCE_PATHS,
            *ARCHIVE_BOUNDARY_PRODUCT_SOURCE_PATHS,
        ],
        "required_values": {
            "schema_version": 1,
            "target": FAILURE_BOUNDARY_TARGET,
            "feature_id": FEATURE_ID,
            "method_version": METHOD_VERSION,
            "passed": True,
            "build_commands.qpls_resampling": [
                "cargo", "test", "--release", "-p", "qpls-resampling", "--lib", "--no-run", "--message-format=json",
            ],
            "build_commands.qpls_project": [
                "cargo", "test", "--release", "-p", "qpls-project", "--lib", "--no-run", "--message-format=json",
            ],
        },
        "required_true": [
            *[f"checks.{name}" for name in sorted(FAILURE_BOUNDARY_TEST_NAMES)],
            *[f"archive_checks.{name}" for name in sorted(ARCHIVE_BOUNDARY_TEST_NAMES)],
        ],
        "required_nonempty": ["generated_at_utc", "test_executables.qpls_resampling.path", "test_executables.qpls_project.path", "executions"],
    }


def _failure_boundary_execution_attestation(root: Path, path: Path) -> dict[str, Any]:
    document = _load_optional(path)
    names = document.get("checks")
    observed_names = frozenset(names) if isinstance(names, dict) else frozenset()
    archive_names = document.get("archive_checks")
    observed_archive_names = frozenset(archive_names) if isinstance(archive_names, dict) else frozenset()
    descriptors = document.get("test_executables") if isinstance(document.get("test_executables"), dict) else {}
    executable_attestations = {}
    for target_name in ("qpls_resampling", "qpls_project"):
        executable = _artifact_attestation(root, descriptors.get(target_name))
        relative = executable.get("path")
        path_pattern_ok = isinstance(relative, str) and re.fullmatch(
            rf"target/release/deps/{target_name}-[0-9A-Za-z_-]+\.exe", relative
        ) is not None
        executable_path = root / relative if isinstance(relative, str) else root
        sources = (
            FAILURE_BOUNDARY_PRODUCT_SOURCE_PATHS
            if target_name == "qpls_resampling"
            else ARCHIVE_BOUNDARY_PRODUCT_SOURCE_PATHS
        )
        freshness = [
            {
                "path": source,
                "present": (root / source).is_file(),
                "binary_not_older": (
                    executable_path.is_file()
                    and (root / source).is_file()
                    and executable_path.stat().st_mtime_ns >= (root / source).stat().st_mtime_ns
                ),
            }
            for source in sources
        ]
        executable_attestations[target_name] = {
            "artifact": executable,
            "path_pattern_ok": path_pattern_ok,
            "source_freshness": freshness,
            "passed": executable["passed"] and path_pattern_ok
            and bool(freshness)
            and all(item["present"] and item["binary_not_older"] for item in freshness),
        }
    executions = document.get("executions")
    execution_contract = (
        isinstance(executions, dict)
        and frozenset(executions) == FAILURE_BOUNDARY_TEST_NAMES | ARCHIVE_BOUNDARY_TEST_NAMES
        and all(
            isinstance(executions.get(name), dict)
            and executions[name].get("passed") is True
            and executions[name].get("exit_code") == 0
            and isinstance(executions[name].get("full_name"), str)
            and executions[name]["full_name"].endswith(name)
            and executions[name].get("target") == (
                "qpls_resampling" if name in FAILURE_BOUNDARY_TEST_NAMES else "qpls_project"
            )
            for name in FAILURE_BOUNDARY_TEST_NAMES | ARCHIVE_BOUNDARY_TEST_NAMES
        )
    )
    checks = {
        "exact_test_name_set": observed_names == FAILURE_BOUNDARY_TEST_NAMES,
        "exact_archive_test_name_set": observed_archive_names == ARCHIVE_BOUNDARY_TEST_NAMES,
        "all_execution_rows_pass": execution_contract,
        "test_executables_are_current_and_bound": all(
            item["passed"] for item in executable_attestations.values()
        ),
    }
    return {
        "checks": checks,
        "test_executables": executable_attestations,
        "observed_test_names": sorted(observed_names),
        "observed_archive_test_names": sorted(observed_archive_names),
        "passed": all(checks.values()),
    }


def _reference_spec() -> dict[str, Any]:
    return {
        "name": REFERENCE_REPORT_NAME,
        "required_values": {
            "schema_version": 1,
            "target": REFERENCE_TARGET,
            "feature_id": FEATURE_ID,
            "method_version": METHOD_VERSION,
            "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
            "scope.base_methods": ["regression_ols_v1", "regression_logistic_v2"],
            "scope.algorithm": "case_resampling_with_replacement",
            "scope.intervals": ["percentile_type7_primary", "bca_midrank_conditional"],
            "scope.test_reference": "standard_normal_bootstrap_ratio_v1",
            "scope.alternative": "two_sided",
            "scope.confidence_level": 0.95,
            "scope.quickpls_replicates": 1000,
            "scope.workers_compared": [1, 4],
            "scope.missing_data": "listwise_deletion",
            "scope.term_limit_including_intercept": 51,
            "scope.process_bootstrap": False,
            "scope.studentized_or_custom_alpha": False,
            "artifacts.fixture": "validation/results/v08_extended_methods_fixture.csv",
            "artifacts.r_script": "validation/regression_bootstrap_v1_reference.R",
            "artifacts.ols.recipe": "validation/results/regression_bootstrap_v1_ols.recipe.json",
            "artifacts.ols.quickpls_result": "validation/results/regression_bootstrap_v1_ols_quickpls.json",
            "artifacts.logistic.recipe": "validation/results/regression_bootstrap_v1_logistic.recipe.json",
            "artifacts.logistic.quickpls_result": "validation/results/regression_bootstrap_v1_logistic_quickpls.json",
        },
        "required_true": [f"checks.{name}" for name in sorted(REFERENCE_CHECK_NAMES)],
        "required_nonempty": ["external_r.version"],
        "source_paths": REFERENCE_SOURCE_PATHS,
        "companions": [
            _recipe_companion("ols"),
            _recipe_companion("logistic"),
            _result_companion("ols"),
            _result_companion("logistic"),
        ],
    }


def _recipe_companion(model: str) -> dict[str, Any]:
    outcome = "y" if model == "ols" else "bin_y"
    model_config = (
        {"type": "ols", "robust_se": "hc3"}
        if model == "ols"
        else {"type": "logistic"}
    )
    return {
        "path": f"validation/results/regression_bootstrap_v1_{model}.recipe.json",
        "required_values": {
            "schema_version": 3,
            "settings.method": "regression",
            "settings.preprocessing": "unstandardized",
            "settings.missing_data": "listwise_deletion",
            "settings.bootstrap_samples": 1000,
            "settings.workers": 1,
            "settings.confidence_level": 0.95,
            "method_config.kind": "regression",
            "method_config.outcome": outcome,
            "method_config.predictors": ["x", "z"],
            "method_config.controls": ["w"],
            "method_config.model": model_config,
            "method_config.bootstrap.algorithm": "case_resampling",
            "method_config.bootstrap.intervals": ["percentile", "bca"],
            "metadata.status": "validated_regression_bootstrap_v1_bounded_scope",
            "model.constructs": [],
            "model.paths": [],
        },
        "required_nonempty": ["dataset_fingerprint"],
    }


def _result_companion(model: str) -> dict[str, Any]:
    base = "regression_ols_v1" if model == "ols" else "regression_logistic_v2"
    return {
        "path": f"validation/results/regression_bootstrap_v1_{model}_quickpls.json",
        "required_values": {
            "status": "completed",
            "provenance.method": "regression",
            "provenance.method_version": f"{base}+{METHOD_VERSION}",
            "provenance.settings.bootstrap_samples": 1000,
            "provenance.settings.workers": 1,
            "payload.estimation.method_version": base,
            "payload.estimation.regression.method_version": base,
            "payload.estimation.regression.regression_type": model,
            "payload.estimation.regression.bootstrap.method_version": METHOD_VERSION,
            "payload.estimation.regression.bootstrap.algorithm": "indexed_case_resampling_v1",
            "payload.estimation.regression.bootstrap.stream_token": "quickpls_indexed_resampling_v1",
            "payload.estimation.regression.bootstrap.validation_witness.method_version": WITNESS_VERSION,
            "payload.assessment.method_version": "assessment_not_applicable_v1",
        },
        "required_nonempty": [
            "payload.estimation.regression.bootstrap.coefficients",
            "payload.estimation.regression.bootstrap.validation_witness.successful_bootstrap",
            "payload.estimation.regression.bootstrap.validation_witness.successful_jackknife",
        ],
    }


def _packaged_spec() -> dict[str, Any]:
    return {
        "name": PACKAGED_REPORT_NAME,
        "required_values": {
            "schema_version": PACKAGED_SCHEMA_VERSION,
            "kind": PACKAGED_KIND,
            "target": PACKAGED_PLATFORM_TARGET,
            "feature_id": FEATURE_ID,
            "method_version": METHOD_VERSION,
            "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
            "runtime": PACKAGED_RUNTIME,
            "generator": PACKAGED_GENERATOR,
            "source_report": PACKAGED_SOURCE_REPORT,
            "checks.results.ols_initial_selected_table": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
            "checks.results.logistic_initial_selected_table": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
            "checks.save_reopen.ols_initial_selected_table": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
            "checks.save_reopen.logistic_initial_selected_table": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
        },
        "required_true": [f"checks.{name}.passed" for name in sorted(PACKAGED_CHECK_NAMES)],
        "required_nonempty": ["generated_at_utc", "completed_at_utc"],
        "source_paths": PACKAGED_SOURCE_PATHS,
        "companions": [
            {
                "path": PACKAGED_SOURCE_REPORT,
                "require_explicit_pass": True,
                "pass_paths": ["passed"],
                "required_values": {
                    "schema_version": PACKAGED_SCHEMA_VERSION,
                    "feature_id": FEATURE_ID,
                    "method_version": METHOD_VERSION,
                    "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
                    "runtime": PACKAGED_RUNTIME,
                    "checks.regressionBootstrapWorkflow.passed": True,
                    "checks.regressionBootstrapWorkflow.feature_id": FEATURE_ID,
                    "checks.regressionBootstrapWorkflow.method_version": METHOD_VERSION,
                    "checks.regressionBootstrapWorkflow.catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
                    "checks.regressionBootstrapResults.passed": True,
                    "checks.regressionBootstrapResults.olsInitialSelectedTable": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                    "checks.regressionBootstrapResults.logisticInitialSelectedTable": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                    "checks.regressionBootstrapOlsExport.passed": True,
                    "checks.regressionBootstrapLogisticExport.passed": True,
                    "checks.regressionBootstrapSaveReopen.passed": True,
                    "checks.regressionBootstrapSaveReopen.initialSelectedTables.ols": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                    "checks.regressionBootstrapSaveReopen.initialSelectedTables.logistic": REGRESSION_BOOTSTRAP_DEFAULT_TABLE,
                    "checks.regressionBootstrapCancellation.passed": True,
                    "checks.regressionBootstrapWitnessBoundary.passed": True,
                },
                "required_nonempty": ["generatedAt", "focusedRun.completedAt", "screenshots"],
            }
        ],
    }


def _visual_spec(root: Path) -> dict[str, Any]:
    return {
        "name": VISUAL_REPORT_NAME,
        "max_age_days": 2,
        "source_paths": _visual_source_paths(root),
        "required_values": {
            "passed": True,
            "consoleErrors": [],
            "failures": [],
        },
    }


def build_audit(
    *,
    root: Path = ROOT,
    results: Path = RESULTS,
    now: datetime | None = None,
) -> dict[str, Any]:
    observed_at = now or datetime.now(timezone.utc)
    reference = evaluate_report(root, results, _reference_spec(), now=observed_at)
    packaged = evaluate_report(root, results, _packaged_spec(), now=observed_at)
    visual = evaluate_report(root, results, _visual_spec(root), now=observed_at)
    failure_boundary_report = evaluate_report(
        root, results, _failure_boundary_spec(), now=observed_at
    )
    reference_names = _exact_check_names(
        results / REFERENCE_REPORT_NAME, REFERENCE_CHECK_NAMES
    )
    packaged_names = _exact_check_names(
        results / PACKAGED_REPORT_NAME, PACKAGED_CHECK_NAMES
    )
    reference_attestation = _reference_attestation(root, results / REFERENCE_REPORT_NAME)
    packaged_attestation = _packaged_attestation(
        root, results / PACKAGED_REPORT_NAME
    )
    visual_attestation = _visual_attestation(root, results / VISUAL_REPORT_NAME)
    failure_boundary_execution = _failure_boundary_execution_attestation(
        root, results / FAILURE_BOUNDARY_REPORT_NAME
    )
    failure_boundary_source_identity = _failure_boundary_source_attestation(root)
    method_doc = evaluate_document(
        root,
        {
            "path": "docs/methods/REGRESSION_BOOTSTRAP_V1.md",
            "required_phrases": [
                METHOD_VERSION,
                "indexed_case_resampling_v1",
                "percentile_primary_bca_conditional_v1",
                "standard_normal_bootstrap_ratio_v1",
                WITNESS_VERSION,
                "not rendered or exported",
                "at most 50 predictors and controls plus the intercept",
                "Studentized intervals, custom alpha/tails",
                "PROCESS bootstrapping are excluded",
            ],
        },
    )
    compatibility_doc = evaluate_document(
        root,
        {
            "path": "docs/METHOD_COMPATIBILITY.md",
            "required_phrases": [
                "Regression | OLS and binary-logistic bootstrapping",
                METHOD_VERSION,
                "Release-qualified for the bounded v1 scope",
                "genuine packaged OLS and logistic 10,000-resample execution",
                "PROCESS, weights, multinomial, and ordinal inference remain excluded",
            ],
            "forbidden_phrases": [
                "native, independent-reference, packaged, and promotion evidence still required before release qualification",
            ],
        },
    )
    checks = [
        {
            "name": "dedicated_exact_and_independent_reference",
            "passed": reference["passed"]
            and reference_names["passed"]
            and reference_attestation["passed"],
            "detail": (
                "The dedicated report must bind exact witness arithmetic and independently "
                "seeded Python/R OLS and logistic sampling-distribution comparisons."
            ),
        },
        {
            "name": "genuine_method_specific_packaged_acceptance",
            "passed": packaged["passed"]
            and packaged_names["passed"]
            and packaged_attestation["passed"],
            "detail": (
                "The dedicated packaged report must prove both workflows, truthful results, "
                "two genuine XLSX exports, cancellation, archive-only witness handling, and "
                "same-run save/reopen with bound digests."
            ),
        },
        {
            "name": "browser_visual_acceptance_gate_7",
            "passed": visual["passed"] and visual_attestation["passed"],
            "detail": (
                "A fresh production-bundle visual report must pass the exact three-viewport "
                "Regression bootstrap OLS/logistic setup, accessibility, focus, and overflow contract."
            ),
        },
        {
            "name": "real_engine_failure_boundaries_executed",
            "passed": failure_boundary_report["passed"]
            and failure_boundary_execution["passed"],
            "detail": "A fresh exact-name four-test report must bind passing real-engine boundaries to its current release-profile Rust test executable.",
        },
        {
            "name": "frozen_scope_documented",
            "passed": method_doc["passed"] and compatibility_doc["passed"],
            "detail": "The current method contract, exclusions, archive boundary, term limit, and public release-qualified status must remain explicit and synchronized.",
        },
    ]
    passed = all(check["passed"] for check in checks)
    return {
        "schema_version": 1,
        "integrity_contract": "exact_current_reference_packaged_identity_and_artifact_binding_v1",
        "generated_at_utc": observed_at.isoformat().replace("+00:00", "Z"),
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "passed": passed,
        "reference_report": reference,
        "packaged_acceptance": packaged,
        "browser_visual_acceptance": visual,
        "failure_boundary_report": failure_boundary_report,
        "exact_check_sets": {
            "reference": reference_names,
            "packaged_acceptance": packaged_names,
        },
        "reference_attestation": reference_attestation,
        "packaged_attestation": packaged_attestation,
        "visual_attestation": visual_attestation,
        "failure_boundary_execution_attestation": failure_boundary_execution,
        "failure_boundary_source_identity": failure_boundary_source_identity,
        "docs": [method_doc, compatibility_doc],
        "checks": checks,
        "note": (
            "Promotion remains false until current numerical and genuine packaged Windows/Tauri "
            "evidence pass with the exact feature, method-version, and catalogue-snapshot "
            "identities. Generic, point-only, PLS-bootstrap, or logistic-v2 evidence cannot "
            "satisfy this regression-bootstrap audit."
        ),
    }


def main() -> int:
    RESULTS.mkdir(parents=True, exist_ok=True)
    report = build_audit()
    output = RESULTS / OUTPUT_NAME
    output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {output} | passed={report['passed']}")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
