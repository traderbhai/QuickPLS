#!/usr/bin/env python3
"""Fail-closed promotion audit for the current bounded logistic v2 workflow.

This audit does not run Cargo, build the desktop application, or manufacture
packaged evidence. It consumes a dedicated numerical reference report and a
method-specific packaged-Tauri acceptance report. Missing evidence, a legacy
v1 method token, a generic cumulative desktop report, or any identity mismatch
keeps promotion false.
"""

from __future__ import annotations

import json
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
    load_json,
    sha256_file,
)


FEATURE_ID = "qpls3.standalone.logistic"
METHOD_VERSION = "regression_logistic_v2"
LEGACY_METHOD_VERSION = "regression_logistic_v1"
CATALOGUE_SNAPSHOT_DATE = "2026-08-12"
TARGET = "quickpls3_standalone_logistic_v2_promotion"
REFERENCE_TARGET = "binary_logistic_regression_v2_reference"
REFERENCE_REPORT_NAME = "logistic_v2_reference_report.json"
PACKAGED_REPORT_NAME = "logistic_v2_packaged_acceptance.json"
PACKAGED_SOURCE_REPORT = "validation/results/v247_tauri_native_acceptance_logistic.json"
OUTPUT_NAME = "logistic_method_promotion_audit.json"
PACKAGED_KIND = "quickpls3_scoped_tauri_logistic_v2_acceptance"
PACKAGED_SCHEMA_VERSION = "quickpls.packaged_acceptance.v1"
PACKAGED_PLATFORM_TARGET = "windows_10_11_x64_packaged_tauri"
PACKAGED_RUNTIME = "tauri-webview2-cdp"
PACKAGED_GENERATOR = "validation/v247_tauri_native_acceptance.mjs"

REFERENCE_CHECK_NAMES = frozenset({
    "quickpls_v2_exact_contract",
    "independent_python_full_arithmetic",
    "external_r_glm_full_arithmetic",
    "seed_invariant_deterministic_result",
    "non_binary_outcome_rejected",
    "single_class_outcome_rejected",
    "rank_deficiency_rejected",
    "complete_separation_rejected",
    "nondefault_worker_count_rejected",
})
PACKAGED_CHECK_NAMES = frozenset({
    "workflow",
    "results",
    "export",
    "save_reopen",
    "failure_lifecycle",
    "legacy_v1",
})

REFERENCE_SOURCE_PATHS = [
    "validation/logistic_v2_reference.py",
    "crates/qpls-core/src/contract.rs",
    "crates/qpls-core/src/validation.rs",
    "crates/qpls-estimation/src/pls.rs",
    "crates/qpls-project/src/lib.rs",
    "crates/qpls-runner/src/lib.rs",
]
PACKAGED_SOURCE_PATHS = [
    PACKAGED_GENERATOR,
    "validation/logistic_v2_packaged_acceptance.schema.json",
    "validation/run_v247_logistic_native_acceptance.ps1",
]


def _load_optional(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        return load_json(path)
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
    reported_size = evidence["reported_size"]
    reported_sha256 = evidence["reported_sha256"]
    if (
        not isinstance(relative, str)
        or not relative.strip()
        or not isinstance(reported_size, int)
        or isinstance(reported_size, bool)
        or reported_size <= 0
        or not isinstance(reported_sha256, str)
        or re.fullmatch(r"[0-9a-f]{64}", reported_sha256) is None
    ):
        return evidence

    repository = root.resolve()
    artifact_path = (repository / relative).resolve()
    try:
        artifact_path.relative_to(repository)
    except ValueError:
        return evidence
    evidence["inside_repository"] = True
    if not artifact_path.is_file():
        return evidence
    evidence["present"] = True
    evidence["actual_size"] = artifact_path.stat().st_size
    evidence["actual_sha256"] = sha256_file(artifact_path)
    evidence["passed"] = (
        evidence["actual_size"] == reported_size
        and evidence["actual_sha256"] == reported_sha256
    )
    return evidence


def _packaged_attestation(root: Path, path: Path) -> dict[str, Any]:
    document = _load_optional(path)
    artifacts = document.get("artifacts") if isinstance(document.get("artifacts"), dict) else {}
    xlsx = _artifact_attestation(root, artifacts.get("xlsx"))
    project_archive = _artifact_attestation(root, artifacts.get("project_archive"))
    raw_screenshots = artifacts.get("screenshots")
    screenshots = (
        [_artifact_attestation(root, item) for item in raw_screenshots]
        if isinstance(raw_screenshots, list)
        else []
    )
    check_rows = document.get("checks") if isinstance(document.get("checks"), dict) else {}
    export = check_rows.get("export") if isinstance(check_rows.get("export"), dict) else {}
    save_reopen = check_rows.get("save_reopen") if isinstance(check_rows.get("save_reopen"), dict) else {}

    def timestamp(value: Any) -> datetime | None:
        if not isinstance(value, str) or not value.strip():
            return None
        try:
            parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        except ValueError:
            return None
        return parsed if parsed.tzinfo is not None else None

    generated = timestamp(document.get("generated_at_utc"))
    completed = timestamp(document.get("completed_at_utc"))
    checks = {
        "ordered_utc_timestamps": generated is not None and completed is not None and completed >= generated,
        "xlsx_artifact_matches_disk": xlsx["passed"],
        "project_archive_matches_disk": project_archive["passed"],
        "screenshot_artifacts_match_disk": len(screenshots) >= 6 and all(item["passed"] for item in screenshots),
        "artifact_paths_are_unique": len({
            item["path"] for item in [xlsx, project_archive, *screenshots] if item["path"] is not None
        }) == 2 + len(screenshots),
        "export_digest_is_bound": export.get("artifact_sha256") == xlsx["reported_sha256"],
        "archive_digest_is_bound": save_reopen.get("archive_sha256") == project_archive["reported_sha256"],
    }
    return {
        "checks": checks,
        "artifacts": {
            "xlsx": xlsx,
            "project_archive": project_archive,
            "screenshots": screenshots,
        },
        "passed": all(checks.values()),
    }


def _reference_attestation(path: Path) -> dict[str, Any]:
    document = _load_optional(path)
    tolerance = document.get("tolerance")
    python_delta = document.get("maximum_absolute_difference_python")
    r_delta = document.get("maximum_absolute_difference_r")

    def within_tolerance(value: Any) -> bool:
        return (
            isinstance(value, (int, float))
            and not isinstance(value, bool)
            and isinstance(tolerance, (int, float))
            and not isinstance(tolerance, bool)
            and math.isfinite(float(value))
            and math.isfinite(float(tolerance))
            and 0.0 <= float(value) <= float(tolerance)
        )

    r_reference = document.get("r_reference")
    checks = {
        "python_delta_within_declared_tolerance": within_tolerance(python_delta),
        "r_delta_within_declared_tolerance": within_tolerance(r_delta),
        "external_r_reference_identified": (
            isinstance(r_reference, dict)
            and r_reference.get("available") is True
            and r_reference.get("passed") is True
            and isinstance(r_reference.get("version"), str)
            and bool(r_reference.get("version", "").strip())
        ),
    }
    return {"checks": checks, "passed": all(checks.values())}


def _reference_spec() -> dict[str, Any]:
    return {
        "name": REFERENCE_REPORT_NAME,
        "required_values": {
            "schema_version": 1,
            "target": REFERENCE_TARGET,
            "feature_id": FEATURE_ID,
            "method_version": METHOD_VERSION,
            "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
            "tolerance": 2e-6,
            "scope.outcome_coding": "exact numeric 0 and 1",
            "scope.missing_data": "listwise_deletion",
            "scope.confidence": "two_sided_95_percent_fixed",
            "scope.classification_threshold": 0.5,
            "scope.workers": 1,
            "artifacts.fixture": "validation/results/logistic_v2_reference.csv",
            "artifacts.recipe": "validation/results/logistic_v2_reference.recipe.json",
            "artifacts.quickpls_result": "validation/results/logistic_v2_reference_quickpls.json",
        },
        "required_true": [f"checks.{name}" for name in sorted(REFERENCE_CHECK_NAMES)],
        "required_nonempty": ["r_reference.version"],
        "source_paths": REFERENCE_SOURCE_PATHS,
        "companions": [
            {
                "path": "validation/results/logistic_v2_reference.recipe.json",
                "required_values": {
                    "schema_version": 3,
                    "settings.method": "regression",
                    "settings.preprocessing": "unstandardized",
                    "settings.workers": 1,
                    "settings.confidence_level": 0.95,
                    "method_config.kind": "regression",
                    "method_config.model.type": "logistic",
                    "model.constructs": [],
                    "model.paths": [],
                },
                "required_nonempty": ["dataset_fingerprint"],
            },
            {
                "path": "validation/results/logistic_v2_reference_quickpls.json",
                "required_values": {
                    "status": "completed",
                    "provenance.method": "regression",
                    "provenance.method_version": METHOD_VERSION,
                    "provenance.settings.workers": 1,
                    "payload.estimation.method_version": METHOD_VERSION,
                    "payload.estimation.regression.method_version": METHOD_VERSION,
                    "payload.estimation.regression.regression_type": "logistic",
                    "payload.estimation.regression.logistic.outcome_profile.readiness": "ready",
                    "payload.estimation.regression.logistic.convergence.converged": True,
                    "payload.assessment.method_version": "assessment_not_applicable_v1",
                },
            },
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
        },
        "required_true": [f"checks.{name}.passed" for name in sorted(PACKAGED_CHECK_NAMES)],
        "required_nonempty": [
            "generated_at_utc",
            "completed_at_utc",
        ],
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
                    "checks.logisticWorkflow.passed": True,
                    "checks.logisticWorkflow.feature_id": FEATURE_ID,
                    "checks.logisticWorkflow.method_version": METHOD_VERSION,
                    "checks.logisticWorkflow.catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
                    "checks.logisticFailureLifecycle.passed": True,
                    "checks.logisticLegacyV1.passed": True,
                },
                "required_nonempty": ["generatedAt", "focusedRun.completedAt", "screenshots"],
            },
        ],
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
    reference_names = _exact_check_names(results / REFERENCE_REPORT_NAME, REFERENCE_CHECK_NAMES)
    packaged_names = _exact_check_names(results / PACKAGED_REPORT_NAME, PACKAGED_CHECK_NAMES)
    reference_attestation = _reference_attestation(results / REFERENCE_REPORT_NAME)
    packaged_attestation = _packaged_attestation(root, results / PACKAGED_REPORT_NAME)
    current_doc = evaluate_document(root, {
        "path": "docs/methods/REGRESSION_LOGISTIC_V2.md",
        "required_phrases": [
            METHOD_VERSION,
            "exact numeric values `0` and `1`",
            "deterministic Newton IRLS",
            "single-worker",
            "not a validated predictive performance estimate",
            "Historical `regression_logistic_v1` results remain readable",
        ],
    })
    legacy_doc = evaluate_document(root, {
        "path": "docs/methods/REGRESSION_LOGISTIC_V1.md",
        "required_phrases": [
            LEGACY_METHOD_VERSION,
            "historical archive-readable contract",
            "New execution and append are disabled",
            METHOD_VERSION,
        ],
    })
    checks = [
        {
            "name": "dedicated_current_v2_reference",
            "passed": reference["passed"] and reference_names["passed"] and reference_attestation["passed"],
            "detail": "The dedicated report must reproduce the exact v2 numerical and guard contract and bind its schema-v3 recipe plus QuickPLS result.",
        },
        {
            "name": "genuine_method_specific_packaged_acceptance",
            "passed": packaged["passed"] and packaged_names["passed"] and packaged_attestation["passed"],
            "detail": "A dedicated packaged-Tauri report must prove the logistic workflow, truthful results, export, save/reopen, failure lifecycle, and legacy-v1 handling with bound artifact digests.",
        },
        {
            "name": "current_and_legacy_contracts_are_separate",
            "passed": current_doc["passed"] and legacy_doc["passed"],
            "detail": "The current v2 contract and archive-only v1 contract must remain explicitly separate.",
        },
    ]
    passed = all(check["passed"] for check in checks)
    return {
        "schema_version": 1,
        "integrity_contract": "exact_current_reference_and_packaged_identity_v1",
        "generated_at_utc": observed_at.isoformat().replace("+00:00", "Z"),
        "target": TARGET,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": CATALOGUE_SNAPSHOT_DATE,
        "legacy_method_version": LEGACY_METHOD_VERSION,
        "legacy_status": "archive_only",
        "passed": passed,
        "reference_report": reference,
        "packaged_acceptance": packaged,
        "exact_check_sets": {
            "reference": reference_names,
            "packaged_acceptance": packaged_names,
        },
        "reference_attestation": reference_attestation,
        "packaged_attestation": packaged_attestation,
        "docs": [current_doc, legacy_doc],
        "checks": checks,
        "note": (
            "Promotion remains false until the dedicated current-v2 reference and a genuine "
            "method-specific packaged Windows/Tauri acceptance report both pass with the exact "
            "feature, method-version, and catalogue-snapshot identities. Historical v1 evidence "
            "is archive-only and cannot satisfy this audit."
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
