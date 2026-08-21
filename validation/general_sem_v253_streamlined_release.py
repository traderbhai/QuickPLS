#!/usr/bin/env python3
"""Promote the three bounded QuickPLS 2.53 cells after the one-shot gate passes.

This is deliberately a narrow release authority. It does not run scientific
workloads, tests, builds, packaged journeys, or version publication. It accepts
only the compact 2.53 product/reference report and either a completed
consolidated Labs diagnostic or that diagnostic plus one hash-bound rerun of
exactly its failed steps. It then promotes the three exact cells together under
the user-approved streamlined integration profile.

All mutations are prepared in memory first. The final multi-file commit is
rollback-protected so an exception or failed post-write validation cannot leave
only part of the requested cell set promoted.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import tempfile
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Callable, Mapping


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"

import sys

sys.path.insert(0, str(VALIDATION))

from capability_registry_v2 import (  # noqa: E402
    canonical_sha256 as registry_sha256,
    derive_legacy_status,
    derive_row_projection,
    generate_legacy_catalogue,
    validate_registry_document,
)
from complexity_performance_v2 import (  # noqa: E402
    DEFAULT_MEASUREMENT_SCHEMA_PATH,
    DEFAULT_SCHEMA_PATH as COMPLEXITY_SCHEMA_PATH,
    active_registry_entries,
    canonical_sha256 as complexity_sha256,
    load_json as load_complexity_json,
    validate_contract_documents,
)
from method_promotion_manifest import validate_manifest  # noqa: E402


PROFILE_ID = "quickpls_v253_streamlined_integration_v1"
REGISTRY_PATH = VALIDATION / "capabilities/capability_registry_v2.json"
CATALOGUE_PATH = VALIDATION / "quickpls_3_competitor_catalogue.json"
COMPLEXITY_PATH = (
    VALIDATION / "capabilities/complexity_performance_profiles_v2.manifest.json"
)
STANDARD_ROOT = VALIDATION / "results/general_sem_v253_standard_v1"
PROMOTION_SUMMARY_PATH = STANDARD_ROOT / "promotion-report.json"
DEFAULT_SUMMARY_PATH = STANDARD_ROOT / "finalization-report.json"

CONSOLIDATED_SUITE_ID = "quickpls_v253_consolidated_diagnostics_v1"
REFERENCE_SUITE_ID = "quickpls_v253_general_sem_compact_reference_v1"
PRODUCT_SUITE_ID = "quickpls_v253_general_sem_product_reference_v1"
REMEDIATION_SUITE_ID = "quickpls_v253_failed_step_remediation_v1"
POSTPROMOTION_SUITE_ID = "quickpls_v253_postpromotion_archive_verification_v1"

REQUIRED_CONSOLIDATED_STEPS = frozenset(
    {
        "production_reference",
        "reference",
        "registry",
        "manifests",
        "rustfmt",
        "diff_check",
        "rust_check",
        "rust_core_three_way",
        "rust_core_single_mediation",
        "rust_estimation_three_way",
        "rust_resampling_three_way",
        "rust_resampling_single_mediation",
        "rust_project_moderation_revision",
        "rust_project_three_way_archive",
        "rust_project_single_mediation_archive",
        "frontend_workflows",
        "frontend_typecheck",
        "frontend_build",
        "release_audit",
    }
)
PREPROMOTION_DEFERRED_STEP_IDS = frozenset(
    {
        "rust_project_three_way_archive",
        "rust_project_single_mediation_archive",
    }
)

STAGE_ROLES = {
    "engine_only": [
        "method_spec",
        "independent_reference",
        "simulation_report",
        "boundary_report",
    ],
    "archive_qualified": ["persistence_report"],
    "native_qualified": ["frontend_report", "export_report"],
    "release_qualified": ["method_audit", "packaged_acceptance"],
}


@dataclass(frozen=True)
class CellDefinition:
    key: str
    owner: str
    cell_id: str
    capability_version: str
    analytical_method_version: str
    operation_version: str | None
    method_manifest: str
    cell_manifest: str
    standard_scope: str

    @property
    def report_path(self) -> Path:
        stem = Path(self.method_manifest).name.removesuffix(".manifest.json")
        return (
            VALIDATION
            / "results/method_factory"
            / stem
            / "v253_streamlined_standard_v1/release_evidence.identity.json"
        )


CELLS = (
    CellDefinition(
        key="single_mediation_bootstrap",
        owner="smartpls.mediation",
        cell_id="qpls3.pls.general_sem_single_mediation_bootstrap",
        capability_version=(
            "general_sem_pls_single_mediation_full_model_case_bootstrap_v1"
        ),
        analytical_method_version=(
            "general_sem_pls_single_mediation_full_model_case_bootstrap_v1"
        ),
        operation_version="general_sem_pls_single_mediation_case_bootstrap_v1",
        method_manifest=(
            "validation/methods/"
            "general_sem_pls_single_mediation_bootstrap_v1.manifest.json"
        ),
        cell_manifest=(
            "validation/capabilities/"
            "general_sem_pls_single_mediation_full_model_case_bootstrap_v1."
            "cell.manifest.json"
        ),
        standard_scope=(
            "Exactly one discovered substantive indirect path in a recursive, "
            "single-group composite PLS model with indexed full-model case "
            "bootstrap and two-sided percentile Type-7 inference."
        ),
    ),
    CellDefinition(
        key="three_way_point",
        owner="smartpls.moderation",
        cell_id="qpls3.pls.general_sem_three_way_moderation_point",
        capability_version="general_sem_pls_three_way_moderation_point_v1",
        analytical_method_version="qpls.general-sem-pls.three-way.point.v1",
        operation_version=None,
        method_manifest=(
            "validation/methods/"
            "general_sem_pls_three_way_moderation_point_v1.manifest.json"
        ),
        cell_manifest=(
            "validation/capabilities/"
            "general_sem_pls_three_way_moderation_point_v1.cell.manifest.json"
        ),
        standard_scope=(
            "One true three-way interaction in a recursive, single-group "
            "composite PLS model using two-stage construction, strong hierarchy, "
            "and fixed continuous or exact 0/1 binary probes."
        ),
    ),
    CellDefinition(
        key="three_way_bootstrap",
        owner="smartpls.moderation",
        cell_id="qpls3.pls.general_sem_three_way_moderation_bootstrap",
        capability_version=(
            "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1"
        ),
        analytical_method_version=(
            "qpls.general-sem-pls.three-way.full-model-case-bootstrap.v1"
        ),
        operation_version=(
            "general_sem_pls_three_way_moderation_case_bootstrap_v1"
        ),
        method_manifest=(
            "validation/methods/"
            "general_sem_pls_three_way_moderation_bootstrap_v1.manifest.json"
        ),
        cell_manifest=(
            "validation/capabilities/"
            "general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1."
            "cell.manifest.json"
        ),
        standard_scope=(
            "One true three-way interaction in a recursive, single-group "
            "composite PLS model with complete stage-1/stage-2 case-bootstrap "
            "refits, one shared ledger, and percentile Type-7 inference."
        ),
    ),
)


def _read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _json_bytes(value: Any) -> bytes:
    return (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")


def _relative(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT.resolve()).as_posix()
    except ValueError as error:
        raise RuntimeError(f"Evidence path is outside the repository: {path}") from error


def _sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _descriptor(path: Path, overrides: Mapping[Path, bytes] | None = None) -> dict[str, Any]:
    resolved = path.resolve()
    data = overrides.get(resolved) if overrides is not None else None
    if data is None:
        data = resolved.read_bytes()
    return {
        "path": _relative(resolved),
        "size": len(data),
        "sha256": _sha256_bytes(data),
    }


def _require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def _validate_consolidated(
    path: Path, *, require_passed: bool
) -> tuple[dict[str, Any], dict[str, Any]]:
    _relative(path)
    report = _read_json(path)
    _require(isinstance(report, dict), "Consolidated report root must be an object")
    _require(report.get("schema_version") == 1, "Consolidated schema must equal 1")
    _require(
        report.get("suite_id") == CONSOLIDATED_SUITE_ID,
        "Unexpected consolidated suite identity",
    )
    _require(report.get("version") == "2.53.0", "Consolidated version must be 2.53.0")
    policy = report.get("policy")
    _require(isinstance(policy, dict), "Consolidated policy is missing")
    _require(
        policy.get("expected_registry_surface") == "labs",
        "Promotion requires the pre-promotion Labs diagnostic",
    )
    _require(
        policy.get("repeated_historical_scientific_matrices") is False,
        "Consolidated report does not declare the streamlined matrix policy",
    )
    summary = report.get("summary")
    _require(isinstance(summary, dict), "Consolidated summary is missing")
    _require(summary.get("skipped") == 0, "Consolidated report contains skipped steps")
    steps = report.get("steps")
    _require(isinstance(steps, list) and steps, "Consolidated steps are missing")
    step_by_id = {
        row.get("id"): row for row in steps if isinstance(row, dict) and row.get("id")
    }
    _require(
        len(step_by_id) == len(steps),
        "Consolidated report contains duplicate or malformed step identities",
    )
    missing = sorted(REQUIRED_CONSOLIDATED_STEPS - set(step_by_id))
    _require(not missing, "Consolidated report omits steps: " + ", ".join(missing))
    failed = sorted(
        str(step_id)
        for step_id, row in step_by_id.items()
        if row.get("status") != "passed" or row.get("exit_code") != 0
    )
    reported_failed = sorted(str(value) for value in summary.get("failed_step_ids", []))
    _require(
        failed == reported_failed and summary.get("failed") == len(failed),
        "Consolidated failed-step summary differs from its step records",
    )
    _require(
        summary.get("total") == len(steps)
        and summary.get("passed") == len(steps) - len(failed),
        "Consolidated step counts are inconsistent",
    )
    if require_passed:
        _require(report.get("passed") is True, "Consolidated diagnostic did not pass")
        _require(not failed, "Consolidated steps are not green: " + ", ".join(failed))
    else:
        _require(
            report.get("passed") is False and bool(failed),
            "A remediation union requires an original report with finite failures",
        )
    return report, step_by_id


def _validate_reference(path: Path) -> tuple[dict[str, Any], Path, dict[str, Any]]:
    _relative(path)
    report = _read_json(path)
    _require(isinstance(report, dict), "Reference report root must be an object")
    _require(report.get("schema_version") == 1, "Reference schema must equal 1")
    _require(report.get("suite_id") == REFERENCE_SUITE_ID, "Unexpected reference suite")
    _require(report.get("passed") is True, "Compact product/reference report did not pass")
    checks = report.get("checks")
    _require(
        isinstance(checks, dict) and checks and all(value is True for value in checks.values()),
        "Compact reference report contains a failed check",
    )
    r_check = report.get("r_cross_check")
    _require(
        isinstance(r_check, dict)
        and r_check.get("required") is True
        and r_check.get("status") == "passed"
        and r_check.get("passed") is True,
        "Required independent R cross-check did not pass",
    )
    product_comparison = report.get("product_comparison")
    _require(
        isinstance(product_comparison, dict)
        and product_comparison.get("required") is True
        and product_comparison.get("status") == "passed"
        and product_comparison.get("passed") is True,
        "Production-path comparison did not pass",
    )
    identities = report.get("identities")
    _require(isinstance(identities, dict), "Reference identities are missing")
    for definition in CELLS:
        identity = identities.get(definition.key)
        _require(isinstance(identity, dict), f"Missing identity for {definition.key}")
        expected = {
            "capability_id": definition.owner,
            "cell_id": definition.cell_id,
            "capability_version": definition.capability_version,
            "method_version": definition.analytical_method_version,
        }
        for field, value in expected.items():
            _require(
                identity.get(field) == value,
                f"Reference {definition.key}.{field} identity mismatch",
            )
        if definition.operation_version is not None:
            _require(
                identity.get("operation_version") == definition.operation_version,
                f"Reference {definition.key}.operation_version identity mismatch",
            )

    product_source = product_comparison.get("source")
    _require(isinstance(product_source, str) and product_source, "Product report path missing")
    product_path = Path(product_source).resolve()
    _relative(product_path)
    _require(product_path.is_file(), "Production-path report is missing")
    product_digest = _sha256_bytes(product_path.read_bytes())
    _require(
        product_comparison.get("source_sha256") == product_digest,
        "Production-path report digest differs from compact reference binding",
    )
    product = _read_json(product_path)
    _require(
        isinstance(product, dict)
        and product.get("schema_version") == 1
        and product.get("suite_id") == PRODUCT_SUITE_ID
        and product.get("passed") is True,
        "Production-path report identity or state is invalid",
    )
    return report, product_path, product


SOURCE_BINDING_EXCLUDED_PREFIXES = (
    ".git/",
    ".vite/",
    "dist/",
    "node_modules/",
    "target/",
    "validation/results/",
)


def _current_source_binding() -> dict[str, Any]:
    listed = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout.splitlines()
    paths = sorted(
        {
            value.replace("\\", "/")
            for value in listed
            if value
            and not value.replace("\\", "/").startswith(
                SOURCE_BINDING_EXCLUDED_PREFIXES
            )
        }
    )
    files: list[dict[str, Any]] = []
    records: list[str] = []
    for relative in paths:
        path = ROOT / relative
        if not path.is_file():
            descriptor = {"path": relative, "size": -1, "sha256": "missing"}
        else:
            data = path.read_bytes()
            descriptor = {
                "path": relative,
                "size": len(data),
                "sha256": _sha256_bytes(data),
            }
        files.append(descriptor)
        records.append(
            f"{descriptor['path']}\t{descriptor['size']}\t{descriptor['sha256']}\n"
        )
    head = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout.strip()
    return {
        "algorithm": "git_tracked_and_untracked_source_manifest_sha256_v1",
        "git_head": head,
        "excluded_prefixes": list(SOURCE_BINDING_EXCLUDED_PREFIXES),
        "file_count": len(files),
        "manifest_sha256": _sha256_bytes("".join(records).encode("utf-8")),
        "files": files,
    }


def _validate_remediation(
    path: Path,
    consolidated_path: Path,
    consolidated: Mapping[str, Any],
    consolidated_steps: Mapping[str, Mapping[str, Any]],
) -> tuple[dict[str, Any], dict[str, Any]]:
    _relative(path)
    report = _read_json(path)
    _require(isinstance(report, dict), "Remediation report root must be an object")
    _require(report.get("schema_version") == 1, "Remediation schema must equal 1")
    _require(report.get("suite_id") == REMEDIATION_SUITE_ID, "Unexpected remediation suite")
    _require(report.get("version") == "2.53.0", "Remediation version must be 2.53.0")
    _require(
        report.get("passed") is True and report.get("promotion_eligible") is True,
        "Failed-step remediation is not promotion-eligible",
    )
    receipt_path = path.with_suffix(".receipt.json")
    _require(receipt_path.is_file(), "Detached remediation SHA-256 receipt is missing")
    receipt = _read_json(receipt_path)
    _require(
        isinstance(receipt, dict)
        and receipt.get("schema_version") == 1
        and receipt.get("receipt_kind")
        == "quickpls_v253_failed_step_remediation_sha256_v1"
        and receipt.get("report") == _descriptor(path),
        "Detached remediation SHA-256 receipt does not bind the report",
    )
    _require(
        Path(str(report.get("repository_root", ""))).resolve() == ROOT.resolve()
        and Path(str(consolidated.get("repository_root", ""))).resolve() == ROOT.resolve(),
        "Consolidated/remediation repository roots differ from the current repository",
    )

    expected_baseline = _descriptor(consolidated_path)
    baseline = report.get("baseline_report")
    _require(
        isinstance(baseline, dict)
        and all(baseline.get(field) == value for field, value in expected_baseline.items()),
        "Remediation report does not hash-bind the supplied consolidated report",
    )
    baseline_failed = sorted(
        step_id
        for step_id, row in consolidated_steps.items()
        if row.get("status") != "passed" or row.get("exit_code") != 0
    )
    selected = sorted(str(value) for value in report.get("selected_step_ids", []))
    _require(
        selected == baseline_failed,
        "Remediation selection is not exactly the original failed-step set",
    )
    steps = report.get("steps")
    _require(isinstance(steps, list) and steps, "Remediation steps are missing")
    step_by_id = {
        row.get("id"): row for row in steps if isinstance(row, dict) and row.get("id")
    }
    _require(
        len(step_by_id) == len(steps) and sorted(step_by_id) == selected,
        "Remediation step identities are duplicate, missing, or out of scope",
    )
    deferred = {
        step_id
        for step_id, row in step_by_id.items()
        if row.get("status") == "deferred"
    }
    _require(
        deferred == PREPROMOTION_DEFERRED_STEP_IDS,
        "Remediation must defer exactly the two Registry-blocked archive checks",
    )
    for step_id, row in step_by_id.items():
        if step_id in PREPROMOTION_DEFERRED_STEP_IDS:
            _require(
                row.get("status") == "deferred"
                and row.get("exit_code") == 101
                and row.get("reason") == "prepromotion_capability_unavailable"
                and row.get("stderr_contains_capability_unavailable") is True,
                f"Deferred archive check lacks the exact CapabilityUnavailable boundary: {step_id}",
            )
        else:
            _require(
                row.get("status") == "passed" and row.get("exit_code") == 0,
                f"Non-deferred remediation step did not pass: {step_id}",
            )
    summary = report.get("summary")
    _require(
        isinstance(summary, dict)
        and summary.get("total") == len(selected)
        and summary.get("passed") == len(selected) - len(PREPROMOTION_DEFERRED_STEP_IDS)
        and summary.get("deferred") == len(PREPROMOTION_DEFERRED_STEP_IDS)
        and summary.get("failed") == 0
        and summary.get("skipped") == 0
        and set(summary.get("deferred_step_ids", []))
        == PREPROMOTION_DEFERRED_STEP_IDS,
        "Remediation summary is inconsistent",
    )

    source_binding = report.get("source_binding")
    _require(
        isinstance(source_binding, dict) and source_binding == _current_source_binding(),
        "Current source/worktree bytes differ from the remediation binding",
    )
    original_binding = consolidated.get("source_binding")
    if original_binding is not None:
        _require(
            original_binding == source_binding,
            "Consolidated and remediation source/worktree bindings differ",
        )
    return report, step_by_id


def _validate_postpromotion(
    path: Path, promotion_path: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    _relative(path)
    report = _read_json(path)
    _require(isinstance(report, dict), "Post-promotion report root must be an object")
    _require(report.get("schema_version") == 1, "Post-promotion schema must equal 1")
    _require(
        report.get("suite_id") == POSTPROMOTION_SUITE_ID,
        "Unexpected post-promotion archive suite",
    )
    _require(report.get("version") == "2.53.0", "Post-promotion version must be 2.53.0")
    _require(
        report.get("passed") is True
        and report.get("release_verification_complete") is True,
        "Post-promotion archive verification did not pass",
    )
    receipt_path = path.with_suffix(".receipt.json")
    _require(receipt_path.is_file(), "Detached post-promotion SHA-256 receipt is missing")
    receipt = _read_json(receipt_path)
    _require(
        isinstance(receipt, dict)
        and receipt.get("schema_version") == 1
        and receipt.get("receipt_kind")
        == "quickpls_v253_postpromotion_archive_verification_sha256_v1"
        and receipt.get("report") == _descriptor(path),
        "Detached post-promotion SHA-256 receipt does not bind the report",
    )
    _require(
        report.get("promotion_report") == _descriptor(promotion_path),
        "Post-promotion report does not hash-bind the activation report",
    )
    selected = set(str(value) for value in report.get("selected_step_ids", []))
    _require(
        selected == PREPROMOTION_DEFERRED_STEP_IDS,
        "Post-promotion report must contain exactly the two deferred archive IDs",
    )
    steps = report.get("steps")
    _require(isinstance(steps, list), "Post-promotion steps are missing")
    step_by_id = {
        row.get("id"): row for row in steps if isinstance(row, dict) and row.get("id")
    }
    _require(
        len(step_by_id) == len(steps)
        and set(step_by_id) == PREPROMOTION_DEFERRED_STEP_IDS,
        "Post-promotion archive step identities are duplicate or incomplete",
    )
    _require(
        all(
            row.get("status") == "passed" and row.get("exit_code") == 0
            for row in step_by_id.values()
        ),
        "A post-promotion archive check did not pass",
    )
    summary = report.get("summary")
    _require(
        isinstance(summary, dict)
        and summary.get("total") == 2
        and summary.get("passed") == 2
        and summary.get("deferred") == 0
        and summary.get("failed") == 0
        and summary.get("skipped") == 0,
        "Post-promotion summary is inconsistent",
    )
    _require(
        report.get("source_binding") == _current_source_binding(),
        "Current source/worktree bytes differ from post-promotion verification",
    )
    return report, step_by_id


def _identity_verification() -> dict[str, Any]:
    return {
        "kind": "identity_report",
        "identity_pointers": {
            "passed": "/passed",
            "feature_id": "/feature_id",
            "method_version": "/method_version",
            "catalogue_snapshot_date": "/catalogue_snapshot_date",
        },
        "source_artifacts_pointer": "/source_artifacts",
        "generated_at_pointer": "/generated_at_utc",
    }


def _prepare_method_manifest(
    definition: CellDefinition, report_path: Path
) -> dict[str, Any]:
    manifest = _read_json(ROOT / definition.method_manifest)
    feature = manifest.get("feature", {})
    _require(feature.get("id") == definition.cell_id, "Method feature identity drift")
    _require(
        feature.get("method_version") == definition.capability_version,
        "Method capability version drift",
    )
    qualification = manifest.get("qualification", {})
    _require(
        qualification.get("declared_state") == "absent"
        and qualification.get("target_state") == "engine_only",
        f"{definition.cell_id} is not at the expected pre-promotion lifecycle",
    )
    qualification["declared_state"] = "release_qualified"
    qualification["target_state"] = "release_qualified"
    qualification["evidence"] = {
        stage: [
            {
                "path": _relative(report_path),
                "roles": roles,
                "verification": _identity_verification(),
            }
        ]
        for stage, roles in STAGE_ROLES.items()
    }
    manifest["claim"]["supported_scope"] = definition.standard_scope
    warnings = list(manifest["claim"]["warnings"])
    warnings[0] = (
        f"This exact bounded cell is scoped Standard under {PROFILE_ID}; "
        "all exclusions and corrective fail-closed behavior remain in force."
    )
    manifest["claim"]["warnings"] = warnings
    return manifest


def _prepare_cell_manifest(definition: CellDefinition) -> dict[str, Any]:
    manifest = _read_json(ROOT / definition.cell_manifest)
    feature = manifest.get("feature", {})
    _require(feature.get("id") == definition.cell_id, "Cell feature identity drift")
    _require(
        feature.get("method_version") == definition.capability_version,
        "Cell capability version drift",
    )
    _require(
        manifest.get("evidence_state") == "absent"
        and manifest.get("surface") == "labs"
        and manifest.get("availability") == "unavailable",
        f"{definition.cell_id} is not a fail-closed Labs cell",
    )
    manifest["evidence_state"] = "release_qualified"
    manifest["surface"] = "standard"
    manifest["availability"] = "standard"
    manifest["qualification_ready"] = True
    manifest["promotion_allowed"] = True
    manifest["remaining_qualification"] = [
        f"Scoped Standard under {PROFILE_ID}; the compact independent "
        "product/reference report and one consolidated automated integration "
        "pass replace repeated historical matrices and per-cell packaged matrices."
    ]
    return manifest


def _prepare_registry() -> dict[str, Any]:
    registry = _read_json(REGISTRY_PATH)
    by_id = {definition.cell_id: definition for definition in CELLS}
    promoted: set[str] = set()
    changed_rows: dict[str, dict[str, Any]] = {}
    for capability in registry["capabilities"]:
        for cell in capability["option_cells"]:
            definition = by_id.get(cell.get("cell_id"))
            if definition is None:
                continue
            _require(
                capability.get("capability_id") == definition.owner
                and cell.get("capability_id") == definition.owner
                and cell.get("capability_version") == definition.capability_version,
                f"Registry identity drift for {definition.cell_id}",
            )
            _require(
                cell.get("evidence_state") == "absent" and cell.get("surface") == "labs",
                f"Registry cell {definition.cell_id} is not pre-promotion Labs/absent",
            )
            cell["evidence_state"] = "release_qualified"
            cell["surface"] = "standard"
            cell["known_differences"] = [
                f"Scoped Standard under {PROFILE_ID} for only the exact documented "
                "predicate; this is not unrestricted SmartPLS breadth or a numerical-identity claim.",
                "All exclusions in the linked capability-cell and method contracts remain in force.",
            ]
            promoted.add(definition.cell_id)
            changed_rows[definition.owner] = capability

    _require(promoted == set(by_id), "Registry does not contain every requested exact cell")

    moderation = changed_rows["smartpls.moderation"]
    moderation_scope = (
        "Bounded standardized two-stage PLS moderation: scoped Standard single, "
        "simultaneous two-way, and one-term true three-way point/full-model "
        "bootstrap cells. Diagram anchors remain presentation-only and "
        "interaction_v2 remains the scientific authority."
    )
    moderation["scope_statement"] = moderation_scope
    moderation["known_differences"] = [
        f"The one-term three-way point and bootstrap cells are independently scoped Standard under {PROFILE_ID}; established two-way identities and evidence are unchanged.",
        "The historical single-interaction qpls3.pls.moderation cell remains separately archive-qualified in Labs, so the mixed parent row still projects conservatively to Labs.",
    ]
    moderation["legacy_row"]["quickpls_scope"] = moderation_scope
    moderation["legacy_row"]["implementation_evidence"] = [
        "validation/general_sem_v253_reference.py",
        "validation/v253_mediation_moderation_release_audit.py",
        "docs/methods/GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_V1.md",
        "docs/methods/GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_V1.md",
        _relative(CELLS[1].report_path),
        _relative(CELLS[2].report_path),
    ]
    moderation["legacy_row"]["remaining_gap"] = (
        "Fourth-order or multiple three-way terms, HOC interactions, three-way "
        "moderated mediation, arbitrary probes, Johnson-Neyman output, groups, "
        "weights, and broader missing-data handling remain excluded."
    )

    mediation = changed_rows["smartpls.mediation"]
    mediation["known_differences"] = [
        f"The exact single-path bootstrap cell is independently scoped Standard under {PROFILE_ID}; existing point, multiple-path, and moderated-mediation identities are unchanged.",
        "Causal identification, broader counterfactual mediation, cyclic paths, groups, weights, and unsupported missing-data handling remain excluded.",
    ]
    mediation["legacy_row"]["implementation_evidence"] = [
        "validation/general_sem_v253_reference.py",
        "validation/v253_mediation_moderation_release_audit.py",
        "docs/methods/GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_V1.md",
        _relative(CELLS[0].report_path),
    ]
    mediation["legacy_row"]["remaining_gap"] = (
        "Causal identification, broader counterfactual mediation, cyclic paths, "
        "groups, weights, and unsupported missing-data handling remain excluded."
    )

    for capability in changed_rows.values():
        capability.update(derive_row_projection(capability))
        capability["legacy_row"]["status"] = derive_legacy_status(capability)

    surfaces = {name: 0 for name in ("standard", "labs", "legacy", "internal")}
    for capability in registry["capabilities"]:
        surfaces[capability["surface"]] += 1
    registry["surface_contract"]["baseline_counts"] = surfaces
    return registry


def _prepare_complexity(registry: Mapping[str, Any]) -> dict[str, Any]:
    complexity = _read_json(COMPLEXITY_PATH)
    entries = active_registry_entries(registry)
    references = [reference for reference, _, _ in entries]
    active_rows = {row.get("capability_id") for _, row, _ in entries}
    binding = complexity["registry_binding"]
    binding["registry_version"] = registry["registry_version"]
    binding["registry_sha256"] = registry_sha256(registry)
    binding["expected_active_row_count"] = len(active_rows)
    binding["expected_active_option_cell_count"] = len(references)
    binding["derived_reference_set_sha256"] = complexity_sha256(references)
    return complexity


def _replace_once(text: str, old: str, new: str, path: str) -> str:
    count = text.count(old)
    _require(count == 1, f"Expected one documentation marker in {path}, found {count}")
    return text.replace(old, new, 1)


def _prepare_docs() -> dict[Path, bytes]:
    replacements: dict[str, list[tuple[str, str]]] = {
        "docs/RELEASE_NOTES_V2_53_0.md": [
            (
                "Status: **implementation in progress; consolidated verification, cell\nqualification, version promotion, Windows candidate, and publication are\npending**. No 2.53 executable or release-qualified claim exists yet.",
                "Status: **workflow implementation and the three bounded exact-cell\nqualifications are complete; version promotion, the unsigned Windows candidate,\npackaged smoke, and publication remain pending**.",
            ),
            ("## New exact cells — not yet qualified", "## New exact cells — scoped Standard"),
            (
                "Three additive cells are registered fail-closed on the Labs surface:",
                f"Three additive cells are independently scoped Standard under `{PROFILE_ID}`:",
            ),
            ("| `smartpls.mediation` | `qpls3.pls.general_sem_single_mediation_bootstrap` | `general_sem_pls_single_mediation_full_model_case_bootstrap_v1` | `absent` |", "| `smartpls.mediation` | `qpls3.pls.general_sem_single_mediation_bootstrap` | `general_sem_pls_single_mediation_full_model_case_bootstrap_v1` | `release_qualified` |"),
            ("| `smartpls.moderation` | `qpls3.pls.general_sem_three_way_moderation_point` | `general_sem_pls_three_way_moderation_point_v1` | `absent` |", "| `smartpls.moderation` | `qpls3.pls.general_sem_three_way_moderation_point` | `general_sem_pls_three_way_moderation_point_v1` | `release_qualified` |"),
            ("| `smartpls.moderation` | `qpls3.pls.general_sem_three_way_moderation_bootstrap` | `general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1` | `absent` |", "| `smartpls.moderation` | `qpls3.pls.general_sem_three_way_moderation_bootstrap` | `general_sem_pls_three_way_moderation_full_model_case_bootstrap_v1` | `release_qualified` |"),
            (
                "Their method manifests target only `engine_only`. Their capability contracts set\n`availability = unavailable`, `qualification_ready = false`, and\n`promotion_allowed = false`. They must not become executable or Standard until\nthe focused reference and consolidated product evidence are accepted. Existing\nmultiple-mediation, simultaneous two-way moderation, and bounded\nmoderated-mediation identities retain their established states.",
                "Their method manifests and capability contracts now bind the accepted compact\nPython/R/product reference and the single consolidated integration pass. The\ncontracts set `availability = standard`, `qualification_ready = true`, and\n`promotion_allowed = true`. Existing multiple-mediation, simultaneous two-way\nmoderation, and bounded moderated-mediation identities retain their established\nstates.",
            ),
            (
                "## Verification still required\n\nBefore 2.53.0 can be declared complete:\n\n1. Run one compact independent Python/R reference matrix for single-path\n   mediation and three-way point, fixed-probe, and bootstrap replay.\n2. Run the consolidated Rust/frontend/typecheck/build and workflow diagnostic.\n3. Correct collected failures as one batch and rerun that same suite once.\n4. Promote each new exact cell independently only if its focused evidence passes.\n5. Bump the product version, build one unsigned Windows candidate, and run one\n   automated moderation create → calculate → Results → save/reopen journey.\n6. Capture the required Canvas, Calculate, progress, Results, and reopen\n   screenshots and confirm C: and D: each retain more than 20 GB free space.",
                "## Remaining release steps\n\nThe compact reference, bounded failed-step remediation, and independent exact-\ncell activation are complete. Two Registry-dependent archive checks run once\nimmediately after activation and must pass before the final qualification receipt.\nThe remaining product steps are then to bump the version, build one unsigned\nWindows candidate, run the single packaged moderation create → calculate →\nResults → save/reopen journey, capture its screenshots, and confirm C: and D:\neach retain more than 20 GB free space.",
            ),
        ],
        "docs/CAPABILITY_REGISTRY_FRONTEND.md": [
            (
                "The three additive 2.53 mediation/moderation cells are intentionally parsed as\nLabs with `evidence_state=absent`. They therefore remain unavailable even when\nExperimental Labs is enabled. Their method manifests target `engine_only`; an\naccepted focused evidence receipt and an explicit Registry update are required\nbefore the adapter may expose them. The existing exact two-way and multiple-\nmediation cells continue to resolve by their own identities.",
                f"The three additive 2.53 mediation/moderation cells are independently parsed as\nscoped Standard with `evidence_state=release_qualified` under\n`{PROFILE_ID}`. Their exact predicates remain fail-closed, and the existing\ntwo-way and multiple-mediation cells continue to resolve by their own identities."
            ),
        ],
        "docs/CAPABILITY_REGISTRY_V2.md": [
            ("Option-cell evidence is 19 absent, one engine-only, one archive-qualified,\nzero native-qualified, and 38 release-qualified. Row evidence is 18 absent,\none engine-only, zero archive-qualified, zero native-qualified, and 26\nrelease-qualified; the row projection remains conservative when independently\ngoverned cells on one official row differ.", "Option-cell evidence is 16 absent, one engine-only, one archive-qualified,\nzero native-qualified, and 41 release-qualified. Row evidence is 16 absent,\none engine-only, one archive-qualified, zero native-qualified, and 27\nrelease-qualified; the row projection remains conservative when independently\ngoverned cells on one official row differ."),
            ("Version 2.53 adds three separately governed, fail-closed Labs registrations:\none exactly-one-path mediation full-model bootstrap cell and one true three-way\nmoderation point/bootstrap pair. All three currently have `evidence_state =\nabsent`, `availability = unavailable` in their capability-cell contracts, and\nmethod manifests targeting only `engine_only`. They must not be described as\nexecutable or promoted until the compact independent reference and consolidated\nworkflow pass produce accepted immutable evidence. The existing exact\nmultiple-mediation, simultaneous two-way moderation, and moderated-mediation\ncells retain their own Standard states and historical identities.", f"Version 2.53 adds three separately governed scoped Standard cells: one\nexactly-one-path mediation full-model bootstrap cell and one true three-way\nmoderation point/bootstrap pair. All three have `evidence_state =\nrelease_qualified`, `availability = standard`, and source-bound method reports\nunder `{PROFILE_ID}`. The existing exact multiple-mediation, simultaneous\ntwo-way moderation, and moderated-mediation cells retain their own Standard\nstates and historical identities."),
        ],
        "docs/METHOD_COMPATIBILITY.md": [
            ("The new exactly-one-path bootstrap identity is registered fail-closed in Labs with absent evidence pending its focused 2.53 reference; it does not relabel or demote the established cells.", "The separately versioned exactly-one-path bootstrap identity is scoped Standard under the streamlined 2.53 profile; it does not relabel or demote the established cells."),
            ("Established point and multiple-path bootstrap cells: yes. New single-path bootstrap cell: unavailable pending qualification.", "Established point and multiple-path bootstrap cells: yes. Single-path bootstrap cell: yes, for its exact scoped Standard predicate."),
            ("Version 2.53 adds fail-closed Labs registrations for one true three-way point/bootstrap term under strong hierarchy; these have absent evidence and remain unavailable pending focused qualification.", "Version 2.53 adds scoped Standard point/bootstrap cells for one true three-way term under strong hierarchy."),
            ("Established two-way point/bootstrap cells: yes. New three-way point/bootstrap cells: unavailable pending qualification.", "Established two-way point/bootstrap cells: yes. Three-way point/bootstrap cells: yes, for their exact scoped Standard predicates."),
        ],
        "docs/FAQ.md": [
            ("These cells are\nregistered fail-closed in Labs with absent evidence while the compact\nindependent reference and consolidated workflow pass are pending; this is not\nyet a Standard or release-qualified claim.", f"These cells are independently scoped Standard under\n`{PROFILE_ID}` after their compact independent reference and consolidated\nworkflow pass succeeded."),
        ],
        "docs/QUICK_START.md": [
            ("The newly registered three-way point cell remains fail-closed until focused qualification succeeds.", "The bounded three-way point cell is Standard for its exact one-term strong-hierarchy predicate."),
            ("The new exactly-one-path mediation and three-way bootstrap cells remain fail-closed until their focused qualification succeeds.", "The exactly-one-path mediation and bounded three-way bootstrap cells are Standard for their exact predicates."),
        ],
        "docs/USER_GUIDE.md": [
            ("Its point/bootstrap Registry cells remain fail-closed pending focused qualification.", "Its point/bootstrap Registry cells are scoped Standard for this exact predicate."),
            ("- three-way conditional effects and two-dimensional simple-slope output when its exact Labs cell is qualified and enabled;", "- three-way conditional effects and two-dimensional simple-slope output for the exact scoped Standard cell;"),
        ],
        "docs/methods/GENERAL_SEM_PLS_SINGLE_MEDIATION_BOOTSTRAP_V1.md": [
            ("Status: **Labs registration; qualification pending**. The source cell is\nregistered fail-closed and is not a Standard claim until its compact independent\nreference and consolidated product pass have produced accepted evidence.", f"Status: **Scoped Standard under `{PROFILE_ID}`**. The exact cell retains its\nindependent identity and every documented boundary."),
            ("Before the cell can advance beyond fail-closed Labs registration, one focused\nindependent reference must verify its point effect, full-model resampling replay,\nfixed seed/worker behavior, failed-replicate ledger, archive identity, routing,\nresult grouping, export source equality, and save/close/reopen path. Earlier\nmultiple-mediation qualification is informative but is not reused as evidence\nfor this new identity.", "The compact independent Python/R/product reference and consolidated routing,\nnative, and export evidence permit activation. The exact schema-6 append/reopen\ncheck runs once against that activated cell and is mandatory for the final\ncell-specific receipt. Earlier multiple-mediation qualification remains separate."),
        ],
        "docs/methods/GENERAL_SEM_PLS_THREE_WAY_MODERATION_POINT_V1.md": [
            ("Status: **Labs registration; qualification pending**. This bounded source cell\nis fail-closed until its focused independent reference passes.", f"Status: **Scoped Standard under `{PROFILE_ID}`**. This remains a bounded\none-term strong-hierarchy cell."),
            ("Promotion requires a\ncompact independent Python/R matrix for the joint coefficients and fixed probes,\nplus routing, canonical result, archive, and native workflow evidence. Until\nthat evidence is accepted, the cell remains unavailable outside its fail-closed\nLabs registration.", "Its activation receipt binds the compact Python/R/product matrix, routing,\ncanonical result, and native workflow evidence. The exact archive append/reopen\ncheck runs once after activation and is mandatory for finalization."),
        ],
        "docs/methods/GENERAL_SEM_PLS_THREE_WAY_MODERATION_BOOTSTRAP_V1.md": [
            ("Status: **Labs registration; qualification pending**. This supplemental source\ncell is fail-closed and does not inherit the maturity of the existing two-way\nbootstrap cell.", f"Status: **Scoped Standard under `{PROFILE_ID}`**. This supplemental cell\nretains a separate identity from the existing two-way bootstrap cell."),
            ("Promotion requires the compact independent reference to replay the complete\nbootstrap, verify point and fixed-probe estimates, reconcile the failure ledger,\nand demonstrate deterministic seed/worker behavior. The consolidated pass must\nalso verify cancellation, persistence, result navigation, export source equality,\nand the packaged create-to-reopen journey. Until then the Registry remains\nfail-closed.", "Its activation receipt binds the compact complete-bootstrap replay, fixed\nprobes, failure ledger, deterministic worker, cancellation, result-navigation,\nand export evidence. The exact archive append/reopen check runs once after\nactivation and is mandatory for finalization; packaged smoke remains a separate\nproduct release step."),
        ],
        "docs/methods/PLS_MEDIATION_V1.md": [
            ("The new single-path cell is a\nfail-closed Labs registration with absent evidence until its focused 2.53\nqualification passes; it does not change this established point identity.", "The single-path cell is independently scoped Standard under the streamlined\n2.53 profile; it does not change this established point identity."),
        ],
        "docs/methods/PLS_TWO_STAGE_MODERATION_V1.md": [
            ("They remain\nfail-closed Labs registrations with absent evidence until their compact\nindependent reference and consolidated workflow pass succeed; this established\nsingle-interaction method does not inherit or lend them qualification evidence.", "They are independently scoped Standard under the streamlined 2.53 profile;\nthis established single-interaction method did not lend them qualification\nevidence."),
        ],
    }
    outputs: dict[Path, bytes] = {}
    for relative, edits in replacements.items():
        path = ROOT / relative
        text = path.read_text(encoding="utf-8")
        for old, new in edits:
            text = _replace_once(text, old, new, relative)
        outputs[path.resolve()] = text.encode("utf-8")
    return outputs


def _source_descriptors(
    definition: CellDefinition,
    manifest: Mapping[str, Any],
    overrides: Mapping[Path, bytes],
) -> list[dict[str, Any]]:
    governance = manifest["governance"]
    required = {
        governance["manifest_path"],
        governance["schema_path"],
        governance["validator_path"],
        governance["focused_test_path"],
        definition.cell_manifest,
        _relative(REGISTRY_PATH),
    }
    for paths in manifest["qualification"]["source_requirements"].values():
        required.update(paths)
    return [_descriptor(ROOT / relative, overrides) for relative in sorted(required)]


def _prepare_outputs(
    consolidated_path: Path,
    reference_path: Path,
    product_path: Path,
    remediation_path: Path | None,
) -> tuple[dict[Path, bytes], dict[str, Any]]:
    generated_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    outputs = _prepare_docs()

    method_manifests: dict[str, dict[str, Any]] = {}
    cell_manifests: dict[str, dict[str, Any]] = {}
    for definition in CELLS:
        method = _prepare_method_manifest(definition, definition.report_path)
        cell = _prepare_cell_manifest(definition)
        method_manifests[definition.key] = method
        cell_manifests[definition.key] = cell
        outputs[(ROOT / definition.method_manifest).resolve()] = _json_bytes(method)
        outputs[(ROOT / definition.cell_manifest).resolve()] = _json_bytes(cell)

    registry = _prepare_registry()
    outputs[REGISTRY_PATH.resolve()] = _json_bytes(registry)
    complexity = _prepare_complexity(registry)
    outputs[COMPLEXITY_PATH.resolve()] = _json_bytes(complexity)
    catalogue = generate_legacy_catalogue(registry)
    outputs[CATALOGUE_PATH.resolve()] = _json_bytes(catalogue)

    input_descriptors = {
        "consolidated": _descriptor(consolidated_path),
        "reference": _descriptor(reference_path),
        "product": _descriptor(product_path),
    }
    if remediation_path is not None:
        input_descriptors["remediation"] = _descriptor(remediation_path)
        input_descriptors["remediation_receipt"] = _descriptor(
            remediation_path.with_suffix(".receipt.json")
        )
    method_report_bytes: dict[str, bytes] = {}
    for definition in CELLS:
        manifest = method_manifests[definition.key]
        report = {
            "schema_version": 1,
            "report_kind": "general_sem_v253_streamlined_method_identity_v1",
            "passed": True,
            "feature_id": manifest["feature"]["id"],
            "method_version": manifest["feature"]["method_version"],
            "catalogue_snapshot_date": manifest["feature"]["catalogue_snapshot_date"],
            "generated_at_utc": generated_at,
            "qualification_profile_id": PROFILE_ID,
            "representative_integration_evidence_acknowledged": True,
            "postpromotion_archive_verification_required": True,
            "prepromotion_deferred_step_ids": sorted(
                PREPROMOTION_DEFERRED_STEP_IDS
            ),
            "cell_identity": {
                "capability_id": definition.owner,
                "cell_id": definition.cell_id,
                "capability_version": definition.capability_version,
                "analytical_method_version": definition.analytical_method_version,
                "operation_version": definition.operation_version,
            },
            "accepted_evidence": input_descriptors,
            "source_artifacts": _source_descriptors(definition, manifest, outputs),
            "excluded_repetition": [
                "historical two-way moderation qualification matrices",
                "historical multiple-mediation qualification matrices",
                "per-cell packaged and performance matrices",
            ],
        }
        encoded = _json_bytes(report)
        method_report_bytes[definition.key] = encoded
        outputs[definition.report_path.resolve()] = encoded

    summary = {
        "schema_version": 1,
        "report_kind": "general_sem_v253_streamlined_standard_promotion_v1",
        "passed": False,
        "promotion_eligible": True,
        "promotion_completed": True,
        "release_complete": False,
        "postpromotion_verification_required": True,
        "qualification_profile_id": PROFILE_ID,
        "generated_at_utc": generated_at,
        "cell_atomic": True,
        "standard_active_cell_ids": sorted(definition.cell_id for definition in CELLS),
        "accepted_evidence": input_descriptors,
        "method_reports": {
            definition.cell_id: {
                "path": _relative(definition.report_path),
                "size": len(method_report_bytes[definition.key]),
                "sha256": _sha256_bytes(method_report_bytes[definition.key]),
            }
            for definition in CELLS
        },
        "registry_sha256": registry_sha256(registry),
        "registry_surface_counts": registry["surface_contract"]["baseline_counts"],
        "remaining_release_steps": [
            "run the two exact post-promotion archive reopen tests",
            "finalize the cell-specific release receipts",
            "promote product version authorities to 2.53.0",
            "build one unsigned Windows candidate",
            "run one automated packaged create-to-reopen smoke journey",
        ],
    }
    outputs[PROMOTION_SUMMARY_PATH.resolve()] = _json_bytes(summary)
    return outputs, summary


def _post_commit_validation() -> dict[str, Any]:
    manifest_reports = [
        validate_manifest(ROOT / definition.method_manifest, ROOT) for definition in CELLS
    ]
    _require(
        all(
            report.get("passed") is True
            and report.get("derived_state") == "release_qualified"
            for report in manifest_reports
        ),
        "A promoted method manifest did not derive release_qualified",
    )
    registry = _read_json(REGISTRY_PATH)
    registry_report = validate_registry_document(
        registry, repository_root=ROOT, check_references=False
    )
    _require(registry_report.get("passed") is True, "Registry structural validation failed")
    _require(
        _read_json(CATALOGUE_PATH) == generate_legacy_catalogue(registry),
        "Generated legacy catalogue differs from Registry V2",
    )
    complexity = _read_json(COMPLEXITY_PATH)
    complexity_report = validate_contract_documents(
        complexity,
        registry,
        load_complexity_json(COMPLEXITY_SCHEMA_PATH),
        load_complexity_json(DEFAULT_MEASUREMENT_SCHEMA_PATH),
    )
    _require(
        complexity_report.get("contract_valid") is True,
        "Complexity/Registry binding validation failed",
    )
    for definition in CELLS:
        cell = _read_json(ROOT / definition.cell_manifest)
        _require(
            cell.get("evidence_state") == "release_qualified"
            and cell.get("surface") == "standard"
            and cell.get("availability") == "standard"
            and cell.get("qualification_ready") is True
            and cell.get("promotion_allowed") is True,
            f"Capability-cell contract did not promote: {definition.cell_id}",
        )
    return {
        "manifest_reports": manifest_reports,
        "registry": registry_report,
        "complexity": complexity_report,
    }


def _rollback_protected_commit(
    outputs: Mapping[Path, bytes], validate: Callable[[], dict[str, Any]]
) -> dict[str, Any]:
    originals = {
        path: path.read_bytes() if path.is_file() else None for path in outputs
    }
    staged: dict[Path, Path] = {}
    try:
        for path, data in outputs.items():
            path.parent.mkdir(parents=True, exist_ok=True)
            handle, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
            with os.fdopen(handle, "wb") as stream:
                stream.write(data)
                stream.flush()
                os.fsync(stream.fileno())
            staged[path] = Path(temporary)
        for path in sorted(outputs, key=lambda item: item.as_posix()):
            os.replace(staged[path], path)
        return validate()
    except Exception:
        for temporary in staged.values():
            if temporary.exists():
                temporary.unlink()
        for path, original in originals.items():
            if original is None:
                if path.exists():
                    path.unlink()
                continue
            handle, temporary = tempfile.mkstemp(prefix=f".{path.name}.rollback.", dir=path.parent)
            with os.fdopen(handle, "wb") as stream:
                stream.write(original)
                stream.flush()
                os.fsync(stream.fileno())
            os.replace(temporary, path)
        raise


def promote(
    consolidated_path: Path,
    reference_path: Path,
    remediation_path: Path | None = None,
) -> dict[str, Any]:
    consolidated_path = consolidated_path.resolve()
    reference_path = reference_path.resolve()
    remediation_path = remediation_path.resolve() if remediation_path is not None else None
    consolidated, consolidated_steps = _validate_consolidated(
        consolidated_path, require_passed=remediation_path is None
    )
    effective_steps = dict(consolidated_steps)
    evidence_owner: Mapping[str, Any] = consolidated
    if remediation_path is not None:
        remediation, remediation_steps = _validate_remediation(
            remediation_path,
            consolidated_path,
            consolidated,
            consolidated_steps,
        )
        effective_steps.update(remediation_steps)
        evidence_owner = remediation
    not_green = []
    for step_id in sorted(REQUIRED_CONSOLIDATED_STEPS):
        row = effective_steps[step_id]
        passed = row.get("status") == "passed" and row.get("exit_code") == 0
        deferred_for_promotion = (
            remediation_path is not None
            and step_id in PREPROMOTION_DEFERRED_STEP_IDS
            and row.get("status") == "deferred"
            and row.get("exit_code") == 101
            and row.get("reason") == "prepromotion_capability_unavailable"
            and row.get("stderr_contains_capability_unavailable") is True
        )
        if not passed and not deferred_for_promotion:
            not_green.append(step_id)
    _require(
        not not_green,
        "Consolidated plus remediation evidence is not green: " + ", ".join(not_green),
    )
    _, product_path, _ = _validate_reference(reference_path)

    bound_reference = Path(
        str(evidence_owner.get("evidence", {}).get("compact_reference", ""))
    ).resolve()
    _require(
        bound_reference == reference_path,
        "Effective diagnostic evidence does not bind the supplied compact reference report",
    )
    _require(
        effective_steps["reference"].get("status") == "passed"
        and effective_steps["production_reference"].get("status") == "passed",
        "Effective product/reference steps are not passed",
    )

    outputs, summary = _prepare_outputs(
        consolidated_path, reference_path, product_path, remediation_path
    )
    validation = _rollback_protected_commit(outputs, _post_commit_validation)
    return {**summary, "post_commit_validation": validation}


def finalize(promotion_path: Path, postpromotion_path: Path) -> dict[str, Any]:
    promotion_path = promotion_path.resolve()
    postpromotion_path = postpromotion_path.resolve()
    _relative(promotion_path)
    promotion = _read_json(promotion_path)
    _require(
        isinstance(promotion, dict)
        and promotion.get("report_kind")
        == "general_sem_v253_streamlined_standard_promotion_v1"
        and promotion.get("promotion_completed") is True
        and promotion.get("promotion_eligible") is True
        and promotion.get("release_complete") is False,
        "Promotion checkpoint is missing or not eligible for finalization",
    )
    _validate_postpromotion(postpromotion_path, promotion_path)
    _post_commit_validation()

    generated_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    post_descriptor = _descriptor(postpromotion_path)
    post_receipt_descriptor = _descriptor(
        postpromotion_path.with_suffix(".receipt.json")
    )
    outputs: dict[Path, bytes] = {}
    method_reports: dict[str, dict[str, Any]] = {}
    for definition in CELLS:
        report_path = definition.report_path.resolve()
        report = _read_json(report_path)
        _require(
            isinstance(report, dict)
            and report.get("passed") is True
            and report.get("feature_id") == definition.cell_id
            and report.get("postpromotion_archive_verification_required") is True,
            f"Method promotion report is not awaiting finalization: {definition.cell_id}",
        )
        accepted = dict(report.get("accepted_evidence", {}))
        accepted["postpromotion_archive"] = post_descriptor
        accepted["postpromotion_archive_receipt"] = post_receipt_descriptor
        report["accepted_evidence"] = accepted
        report["postpromotion_archive_verification_required"] = False
        report["postpromotion_archive_verification_complete"] = True
        report["finalized_at_utc"] = generated_at
        encoded = _json_bytes(report)
        outputs[report_path] = encoded
        method_reports[definition.cell_id] = {
            "path": _relative(report_path),
            "size": len(encoded),
            "sha256": _sha256_bytes(encoded),
        }

    accepted_evidence = dict(promotion.get("accepted_evidence", {}))
    accepted_evidence["postpromotion_archive"] = post_descriptor
    accepted_evidence["postpromotion_archive_receipt"] = post_receipt_descriptor
    summary = {
        "schema_version": 1,
        "report_kind": "general_sem_v253_streamlined_standard_finalization_v1",
        "passed": True,
        "promotion_eligible": True,
        "promotion_completed": True,
        "release_complete": True,
        "postpromotion_verification_required": False,
        "postpromotion_verification_complete": True,
        "qualification_profile_id": PROFILE_ID,
        "generated_at_utc": generated_at,
        "cell_atomic": True,
        "standard_active_cell_ids": sorted(definition.cell_id for definition in CELLS),
        "promotion_report": _descriptor(promotion_path),
        "postpromotion_archive_report": post_descriptor,
        "postpromotion_archive_receipt": post_receipt_descriptor,
        "accepted_evidence": accepted_evidence,
        "method_reports": method_reports,
        "remaining_release_steps": [
            "promote product version authorities to 2.53.0",
            "build one unsigned Windows candidate",
            "run one automated packaged create-to-reopen smoke journey",
        ],
    }
    outputs[DEFAULT_SUMMARY_PATH.resolve()] = _json_bytes(summary)
    validation = _rollback_protected_commit(outputs, _post_commit_validation)
    return {**summary, "post_commit_validation": validation}


def _arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("promote", "finalize"))
    parser.add_argument("--consolidated-report", type=Path)
    parser.add_argument("--reference-report", type=Path)
    parser.add_argument(
        "--remediation-report",
        type=Path,
        help=(
            "Optional passed bounded rerun whose selected steps exactly equal the "
            "original consolidated report's failed-step set."
        ),
    )
    parser.add_argument(
        "--promotion-report",
        type=Path,
        default=PROMOTION_SUMMARY_PATH,
    )
    parser.add_argument("--postpromotion-report", type=Path)
    return parser.parse_args()


def main() -> int:
    args = _arguments()
    if args.phase == "promote":
        _require(
            args.consolidated_report is not None and args.reference_report is not None,
            "promote requires --consolidated-report and --reference-report",
        )
        result = promote(
            args.consolidated_report,
            args.reference_report,
            args.remediation_report,
        )
        succeeded = result.get("promotion_completed") is True
    else:
        _require(
            args.postpromotion_report is not None,
            "finalize requires --postpromotion-report",
        )
        result = finalize(args.promotion_report, args.postpromotion_report)
        succeeded = result.get("passed") is True
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0 if succeeded else 1


if __name__ == "__main__":
    raise SystemExit(main())
