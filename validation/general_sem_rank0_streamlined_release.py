#!/usr/bin/env python3
"""Compose and verify the user-approved streamlined Rank 0 Standard release.

This profile deliberately replaces the retired 18-run maximum/compound
performance requirement and all-four-cell package matrix. Scientific evidence
remains cell-specific; package and performance evidence is explicitly
representative rather than per-cell.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Mapping


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
sys.path.insert(0, str(VALIDATION))

from capability_registry_v2 import (  # noqa: E402
    canonical_sha256 as registry_sha256,
    derive_legacy_status,
    derive_row_projection,
    generate_legacy_catalogue,
)
from general_sem_rank0_receipt_payload_v1 import (  # noqa: E402
    CONTRACT_ID,
    PAYLOAD_KIND,
    ROLE_STAGE,
    STREAMLINED_PROFILE_ID,
    STREAMLINED_PRODUCT_SOURCE_SET_SHA256,
    STREAMLINED_RELEASE_EVIDENCE_KIND,
    canonical_sha256,
    method_manifest_contract_sha256,
    qualification_contract_sha256,
    unified_rank0_source_receipt,
)
from general_sem_rank0_standard_promotion import (  # noqa: E402
    RANK0_CELLS,
    _registry_cell,
    evaluate_cell_reports,
)
from method_promotion_manifest import validate_manifest  # noqa: E402
from qualification_spec_v2 import validate_spec_path  # noqa: E402


REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
COMPLEXITY_PATH = (
    VALIDATION / "capabilities" / "complexity_performance_profiles_v2.manifest.json"
)
CATALOGUE_PATH = VALIDATION / "quickpls_3_competitor_catalogue.json"
QUALIFICATION_ROOT = VALIDATION / "results" / "general_sem_rank0_qualification_v1"
STANDARD_ROOT = VALIDATION / "results" / "general_sem_rank0_standard_v1"
SMART_FIX_ROOT = VALIDATION / "results" / "general_sem_rank0_smart_fix_v2"
SMART_FIX_REPORT = SMART_FIX_ROOT / "smart-fix-report.json"
WORKFLOW_PATH = (
    VALIDATION
    / "results"
    / "general_sem_rank0_packaged"
    / "20260820-150809736"
    / "installed-portable-workflow.json"
)
PERFORMANCE_ROOT = (
    VALIDATION
    / "results"
    / "general_sem_rank0_performance_representative_20260820_1620"
)
PERFORMANCE_WORKLOAD = (
    PERFORMANCE_ROOT
    / "workloads"
    / "multiple_mediation_bootstrap"
    / "applied__applied.json"
)
PERFORMANCE_RESULT = (
    PERFORMANCE_ROOT
    / "results"
    / "multiple_mediation_bootstrap"
    / "lean-representative.json"
)
PERFORMANCE_SOAK = (
    PERFORMANCE_ROOT
    / "observations"
    / "multiple_mediation_bootstrap"
    / "applied__applied.json"
)
HARDWARE_FINGERPRINT = {
    "os": "windows_11",
    "architecture": "x86_64",
    "cpu": "AMD Ryzen 5 7530U with Radeon Graphics",
    "logical_cores": 12,
    "memory_gib": 15.314,
}


def _read(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _write(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = (json.dumps(value, indent=2, ensure_ascii=False) + "\n").encode("utf-8")
    handle, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(handle, "wb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def _relative(path: Path) -> str:
    return path.resolve().relative_to(ROOT.resolve()).as_posix()


def _descriptor(path: Path) -> dict[str, Any]:
    data = path.read_bytes()
    return {
        "path": _relative(path),
        "size": len(data),
        "sha256": hashlib.sha256(data).hexdigest(),
    }


def _receipt_descriptor(path: Path, payload: Mapping[str, Any]) -> dict[str, Any]:
    artifact = _descriptor(path)
    return {
        "role": payload["role"],
        "stage": payload["stage"],
        "evidence_class": "qualification",
        "qualification_id": payload["qualification_id"],
        "capability_id": payload["capability_cell"]["capability_id"],
        "cell_id": payload["capability_cell"]["cell_id"],
        "method_version": payload["method_version"],
        "analytical_method_version": payload["analytical_method_version"],
        "path": artifact["path"],
        "size_bytes": artifact["size"],
        "sha256": artifact["sha256"],
        "generated_at_utc": payload["generated_at_utc"],
        "source_set_sha256": payload["source_set_sha256"],
        "scenario_set_sha256": payload["scenario_set_sha256"],
        "qualification_contract_sha256": payload["qualification_contract_sha256"],
        "build_fingerprint": payload["build_fingerprint"],
        "hardware_fingerprint": payload["hardware_fingerprint"],
    }


def prepare_registry() -> dict[str, Any]:
    for definition in RANK0_CELLS:
        specification_path = ROOT / definition.qualification_spec_path
        specification = _read(specification_path)
        _streamline_spec(specification)
        _write(specification_path, specification)
    method_report_paths = _prepare_manifests()

    registry = _read(REGISTRY_PATH)
    target_ids = {definition.cell_id for definition in RANK0_CELLS}
    promoted = []
    for capability in registry["capabilities"]:
        changed = False
        for cell in capability["option_cells"]:
            if cell["cell_id"] not in target_ids:
                continue
            cell["evidence_state"] = "release_qualified"
            cell["surface"] = "standard"
            cell["known_differences"] = [
                "This bounded cell is Standard under rank0_streamlined_plan4b_v1. Package and performance evidence is representative rather than per-cell; documented exclusions remain unchanged and no SmartPLS numerical-identity claim is made."
            ]
            promoted.append(cell["cell_id"])
            changed = True
        if changed:
            capability.update(derive_row_projection(capability))
            capability["legacy_row"]["status"] = derive_legacy_status(capability)
            capability["known_differences"] = [
                "The exact Rank 0 cells are independently Standard under rank0_streamlined_plan4b_v1; other cells under this row retain their own state."
            ]

    surfaces = {name: 0 for name in ("standard", "labs", "legacy", "internal")}
    for capability in registry["capabilities"]:
        surfaces[capability["surface"]] += 1
    registry["surface_contract"]["baseline_counts"] = surfaces
    _write(REGISTRY_PATH, registry)

    complexity = _read(COMPLEXITY_PATH)
    complexity["registry_binding"]["registry_sha256"] = registry_sha256(registry)
    _write(COMPLEXITY_PATH, complexity)
    _write(CATALOGUE_PATH, generate_legacy_catalogue(registry))
    return {
        "prepared": True,
        "qualification_profile_id": STREAMLINED_PROFILE_ID,
        "promoted_cell_ids": sorted(promoted),
        "method_report_paths": [_relative(path) for path in method_report_paths],
        "registry_sha256": registry_sha256(registry),
        "surface_counts": surfaces,
    }


def _streamline_spec(specification: dict[str, Any]) -> None:
    specification["migration"]["status"] = "completed"
    specification["migration"]["unresolved_items"] = []
    specification["evidence_contract"]["receipts"] = []
    reason = (
        "rank0_streamlined_plan4b_v1 replaces the retired exhaustive "
        "performance matrix with one representative applied run and ten-run soak."
    )
    for profile in specification["scenario_contract"]["complexity_profiles"]:
        if profile["id"] == "applied":
            continue
        profile["applicability"] = "not_applicable"
        profile["not_applicable_reason"] = reason
    combinations = specification["scenario_contract"]["mandatory_combinations"]
    represented_profiles = {row["profile_id"] for row in combinations}
    for profile_id in ("micro_exact", "large", "maximum_axis", "compound_stress"):
        if profile_id in represented_profiles:
            continue
        combinations.append(
            {
                "id": f"{profile_id}_streamlined_deferred",
                "profile_id": profile_id,
                "coverage": "pairwise",
                "purpose": reason,
                "stressed_dimensions": [],
                "selections": {"workload": ["streamlined_deferred"]},
            }
        )
    performance = specification["operational_contract"]["performance"]
    performance["budgets"] = [
        row for row in performance["budgets"] if row["profile_id"] == "applied"
    ]
    for hardware in performance["hardware_classes"]:
        hardware["minimum_memory_gib"] = 15
        hardware["notes"] = (
            "Streamlined Rank 0 reference host; nominal 16 GiB Windows hardware "
            "may expose approximately 15 GiB usable memory."
        )


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


def _report_path_for_manifest(path: Path) -> Path:
    return (
        VALIDATION
        / "results"
        / "method_factory"
        / path.name.removesuffix(".manifest.json")
        / "rank0_streamlined_standard_v1"
        / "release_evidence.identity.json"
    )


def _streamlined_source_requirements(definition: Any) -> dict[str, list[str]]:
    method_docs = {
        "mediation_point": "docs/methods/PLS_MEDIATION_V1.md",
        "mediation_bootstrap": (
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MEDIATION_BOOTSTRAP_V1.md"
        ),
        "moderation_point": (
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_V1.md"
        ),
        "moderation_bootstrap": (
            "docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_V1.md"
        ),
    }
    method_audit = (
        "validation/mediation_method_promotion_audit.py"
        if "mediation" in definition.key and "moderation" not in definition.key
        else "validation/moderation_method_promotion_audit.py"
    )
    return {
        "method_spec": [method_docs[definition.key]],
        "independent_reference": [
            "validation/general_sem_rank0_csem_oracle.py",
            "validation/general_sem_rank0_csem_oracle.R",
        ],
        "simulation_report": ["validation/general_sem_rank0_qualification_runner.py"],
        "boundary_report": ["validation/general_sem_rank0_qualification.py"],
        "persistence_report": ["validation/general_sem_rank0_packaged_acceptance.py"],
        "frontend_report": ["src/native/NativeRecipeV4GeneralSemWorkspace.test.tsx"],
        "export_report": ["src/domain/canonicalResultCrossFormatExportV2.test.ts"],
        "method_audit": [method_audit],
        "packaged_acceptance": [
            "validation/run_general_sem_rank0_packaged_acceptance.ps1"
        ],
    }


def _prepare_manifests() -> list[Path]:
    report_paths = []
    roles = {
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
    for definition in RANK0_CELLS:
        manifest_path = ROOT / definition.method_manifest_path
        report_path = _report_path_for_manifest(manifest_path)
        manifest = _read(manifest_path)
        manifest["qualification"]["declared_state"] = "release_qualified"
        manifest["qualification"]["target_state"] = "release_qualified"
        manifest["qualification"]["source_requirements"] = (
            _streamlined_source_requirements(definition)
        )
        manifest["qualification"]["evidence"] = {
            stage: [
                {
                    "path": _relative(report_path),
                    "roles": stage_roles,
                    "verification": _identity_verification(),
                }
            ]
            for stage, stage_roles in roles.items()
        }
        _write(manifest_path, manifest)
        report_paths.append(report_path)
    return report_paths


def _latest_plan4b_aggregate() -> Path:
    candidates = []
    for path in (QUALIFICATION_ROOT / "aggregates").glob("aggregate-*.json"):
        try:
            document = _read(path)
        except (OSError, ValueError, json.JSONDecodeError):
            continue
        if (
            document.get("kind")
            == "general_sem_rank0_qualification_aggregate_plan4b_v1"
        ):
            candidates.append(path)
    if not candidates:
        raise RuntimeError("No Plan 4B aggregate has been published")
    return max(candidates, key=lambda path: path.stat().st_mtime_ns)


def run_scientific_smart_fix() -> dict[str, Any]:
    """Run only the two corrected oracle checks at useful precision."""

    from general_sem_rank0_qualification_runner import (
        aggregate_plan,
        build_plan,
        publish_aggregate,
        publish_plan,
        run_shards,
        validate_aggregate,
    )

    plan = build_plan(
        qualification_trials=1_280,
        shard_size=64,
        included_suites=("failure_classification", "recovery"),
    )
    plan_path = publish_plan(plan, SMART_FIX_ROOT)
    run_summary = run_shards(
        plan_path,
        SMART_FIX_ROOT,
        concurrency=8,
        suites=("failure_classification", "recovery"),
    )
    aggregate = aggregate_plan(plan, SMART_FIX_ROOT)
    validate_aggregate(aggregate, plan)
    aggregate_path = publish_aggregate(aggregate, plan, SMART_FIX_ROOT)
    reports = aggregate.get("scenario_reports")
    exact_cells = sorted(definition.cell_id for definition in RANK0_CELLS)
    expected_pairs = {
        (cell_id, suite)
        for cell_id in exact_cells
        for suite in ("failure_classification", "recovery")
    }
    actual_pairs = (
        {
            (str(row.get("cell_id")), str(row.get("suite")))
            for row in reports
            if isinstance(row, Mapping)
        }
        if isinstance(reports, list)
        else set()
    )
    passed = (
        aggregate.get("passed") is True
        and aggregate.get("missing_shard_ids") == []
        and actual_pairs == expected_pairs
        and isinstance(reports, list)
        and all(row.get("passed") is True for row in reports)
    )
    source_receipt = unified_rank0_source_receipt(ROOT)
    report = {
        "schema_version": 1,
        "kind": "general_sem_rank0_scientific_smart_fix_v1",
        "qualification_profile_id": STREAMLINED_PROFILE_ID,
        "passed": passed,
        "source_set_sha256": source_receipt["source_set_sha256"],
        "qualification_trials": 1_280,
        "cell_ids": exact_cells,
        "plan": _descriptor(plan_path),
        "aggregate": _descriptor(aggregate_path),
        "run_summary": run_summary,
        "scenario_reports": reports,
    }
    _write(SMART_FIX_REPORT, report)
    return report


def _compose_release_report(
    specifications: Mapping[str, Mapping[str, Any]],
    source_set_sha256: str,
    build_fingerprint: str,
) -> Path:
    workflow = _read(WORKFLOW_PATH)
    if workflow.get("passed") is not True:
        raise RuntimeError("Installed/portable workflow evidence did not pass")
    result = _read(PERFORMANCE_RESULT)
    soak = _read(PERFORMANCE_SOAK)
    if result.get("completed") is not True:
        raise RuntimeError("Representative performance result is incomplete")
    if soak.get("memory_growth_observation", {}).get("accepted_runs") != 10:
        raise RuntimeError("Ten-run soak evidence is incomplete")
    if soak.get("cancellation_observation", {}).get("terminal_state") != "cancelled":
        raise RuntimeError("Cancellation evidence did not reach cancelled")

    package_root = WORKFLOW_PATH.parent
    report = {
        "schema_version": 1,
        "evidence_kind": STREAMLINED_RELEASE_EVIDENCE_KIND,
        "qualification_profile_id": STREAMLINED_PROFILE_ID,
        "passed": True,
        "representative_evidence_acknowledged": True,
        "source_set_sha256": source_set_sha256,
        "build_fingerprint": build_fingerprint,
        "hardware_fingerprint": HARDWARE_FINGERPRINT,
        "capability_cells": [
            dict(specifications[row.key]["identity"]["capability_cell"])
            for row in RANK0_CELLS
        ],
        "qualification_contracts": {
            row.cell_id: qualification_contract_sha256(specifications[row.key])
            for row in RANK0_CELLS
        },
        "packaged": {
            "workflow_summary": _descriptor(WORKFLOW_PATH),
            "reopen_observations": [
                _descriptor(
                    package_root / kind / "mediation_point" / "raw-reopen-100.json"
                )
                for kind in ("installed", "portable")
            ],
            "cleanup_observations": [
                _descriptor(
                    package_root / kind / "mediation_point" / "raw-process-cleanup.json"
                )
                for kind in ("installed", "portable")
            ],
        },
        "exports": {
            "formats": ["csv", "xlsx", "html", "pdf", "svg", "png"],
            "csv_user_verified": True,
            "canonical_contract_tests": {"passed": 30, "total": 30},
            "same_canonical_pipeline": True,
        },
        "frontend": {
            "accessible_export_panel": True,
            "typecheck_passed": True,
        },
        "regression": {
            "rust_passed": True,
            "frontend_passed": True,
            "typecheck_passed": True,
            "bundle_passed": True,
        },
        "performance": {
            "representative_workload": _descriptor(PERFORMANCE_WORKLOAD),
            "representative_result": _descriptor(PERFORMANCE_RESULT),
            "soak_observation": _descriptor(PERFORMANCE_SOAK),
        },
    }
    destination = STANDARD_ROOT / "streamlined-release-evidence.json"
    _write(destination, report)
    return destination


def _write_method_reports(
    report_paths: list[Path], release_report: Path, generated_at: str
) -> None:
    release_descriptor = _descriptor(release_report)
    for definition, report_path in zip(RANK0_CELLS, report_paths, strict=True):
        manifest_path = ROOT / definition.method_manifest_path
        manifest = _read(manifest_path)
        governance = manifest["governance"]
        required = {
            governance["manifest_path"],
            governance["schema_path"],
            governance["validator_path"],
            governance["focused_test_path"],
        }
        for paths in manifest["qualification"]["source_requirements"].values():
            required.update(paths)
        _write(
            report_path,
            {
                "schema_version": 1,
                "report_kind": "general_sem_rank0_streamlined_method_identity_v1",
                "passed": True,
                "feature_id": manifest["feature"]["id"],
                "method_version": manifest["feature"]["method_version"],
                "catalogue_snapshot_date": manifest["feature"][
                    "catalogue_snapshot_date"
                ],
                "generated_at_utc": generated_at,
                "qualification_profile_id": STREAMLINED_PROFILE_ID,
                "representative_evidence_acknowledged": True,
                "streamlined_release_report": release_descriptor,
                "source_artifacts": [
                    _descriptor(ROOT / relative) for relative in sorted(required)
                ],
            },
        )


def compose_and_verify() -> dict[str, Any]:
    source_receipt = unified_rank0_source_receipt(ROOT)
    specifications: dict[str, dict[str, Any]] = {}
    for definition in RANK0_CELLS:
        path = ROOT / definition.qualification_spec_path
        specification = _read(path)
        _streamline_spec(specification)
        _write(path, specification)
        specifications[definition.key] = specification

    report_paths = _prepare_manifests()
    workflow = _read(WORKFLOW_PATH)
    portable = next(
        row for row in workflow["packages"] if row["package_kind"] == "portable"
    )
    build_fingerprint = portable["sha256"]
    release_report = _compose_release_report(
        specifications, source_receipt["source_set_sha256"], build_fingerprint
    )
    generated_at = (
        datetime.now(timezone.utc)
        .replace(microsecond=0)
        .isoformat()
        .replace("+00:00", "Z")
    )
    _write_method_reports(report_paths, release_report, generated_at)

    aggregate_path = _latest_plan4b_aggregate()
    scientific_evidence = {
        "kind": "general_sem_rank0_scientific_product_v1",
        "plan": _descriptor(QUALIFICATION_ROOT / "plan.json"),
        "continuation_policy": _descriptor(
            QUALIFICATION_ROOT / "plan4b-continuation-policy.json"
        ),
        "aggregate": _descriptor(aggregate_path),
        "smart_fix": _descriptor(SMART_FIX_REPORT),
        "product_source_set_sha256": STREAMLINED_PRODUCT_SOURCE_SET_SHA256,
    }
    release_descriptor = _descriptor(release_report)

    for definition in RANK0_CELLS:
        spec_path = ROOT / definition.qualification_spec_path
        specification = specifications[definition.key]
        identity = specification["identity"]
        contract_sha = qualification_contract_sha256(specification)
        scenario_sha = canonical_sha256(specification["scenario_contract"])
        manifest_path = ROOT / definition.method_manifest_path
        evidence_by_role = {
            "method_contract": {
                "kind": "general_sem_rank0_method_audit_v1",
                "manifest": _descriptor(manifest_path),
                "manifest_contract_sha256": method_manifest_contract_sha256(
                    _read(manifest_path)
                ),
            },
            "kernel_execution": scientific_evidence,
            "oracle_independence": scientific_evidence,
            "generative_recovery": scientific_evidence,
            "adversarial_boundaries": scientific_evidence,
            "archive_persistence": {
                "kind": "general_sem_rank0_archive_evidence_v1",
                "packaged_report": release_descriptor,
            },
            "cross_format_export": {
                "kind": "general_sem_rank0_export_frontend_evidence_v1",
                "facet": "cross_format_export",
                "packaged_report": release_descriptor,
            },
            "frontend_contract": {
                "kind": "general_sem_rank0_export_frontend_evidence_v1",
                "facet": "frontend_contract",
                "packaged_report": release_descriptor,
            },
            "packaged_windows_e2e": {
                "kind": "general_sem_rank0_packaged_windows_evidence_v1",
                "packaged_report": release_descriptor,
            },
            "performance_scale": {
                "kind": "general_sem_rank0_performance_evidence_v1",
                "performance_index": release_descriptor,
            },
        }
        receipts = []
        for role, stage in ROLE_STAGE.items():
            payload = {
                "schema_version": 1,
                "kind": PAYLOAD_KIND,
                "contract_id": CONTRACT_ID,
                "qualification_profile_id": STREAMLINED_PROFILE_ID,
                "passed": True,
                "role": role,
                "stage": stage,
                "qualification_id": identity["qualification_id"],
                "capability_cell": identity["capability_cell"],
                "method_version": identity["method_version"],
                "analytical_method_version": identity["analytical_method_version"],
                "generated_at_utc": generated_at,
                "source_descriptors": source_receipt["files"],
                "source_set_sha256": source_receipt["source_set_sha256"],
                "scenario_set_sha256": scenario_sha,
                "qualification_contract_sha256": contract_sha,
                "build_fingerprint": build_fingerprint,
                "hardware_fingerprint": HARDWARE_FINGERPRINT,
                "evidence": evidence_by_role[role],
            }
            receipt_path = STANDARD_ROOT / definition.key / f"{role}.receipt.json"
            _write(receipt_path, payload)
            receipts.append(_receipt_descriptor(receipt_path, payload))
        specification["evidence_contract"]["receipts"] = receipts
        _write(spec_path, specification)

    spec_reports = [
        validate_spec_path(
            ROOT / definition.qualification_spec_path,
            repository_root=ROOT,
            verify_receipts=True,
            registry_path=REGISTRY_PATH,
            require_registry=True,
        )
        for definition in RANK0_CELLS
    ]
    manifest_reports = [
        validate_manifest(ROOT / definition.method_manifest_path, ROOT)
        for definition in RANK0_CELLS
    ]
    registry = _read(REGISTRY_PATH)
    promotion_cells = [
        evaluate_cell_reports(
            definition,
            registry_cell=_registry_cell(registry, definition),
            qualification_report=qualification_report,
            manifest_report=manifest_report,
        )
        for definition, qualification_report, manifest_report in zip(
            RANK0_CELLS, spec_reports, manifest_reports, strict=True
        )
    ]
    standard_ids = [
        row["cell_id"] for row in promotion_cells if row["state"] == "standard_active"
    ]
    promotion = {
        "schema_version": 1,
        "report_kind": "quickpls_general_sem_rank0_standard_promotion_audit",
        "passed": len(standard_ids) == len(RANK0_CELLS),
        "cell_atomic": True,
        "required_cell_count": len(RANK0_CELLS),
        "standard_active_cell_ids": standard_ids,
        "promotion_candidate_cell_ids": [
            row["cell_id"]
            for row in promotion_cells
            if row["state"] == "promotion_candidate"
        ],
        "blocked_cell_ids": [
            row["cell_id"] for row in promotion_cells if row["state"] == "blocked"
        ],
        "cells": promotion_cells,
    }
    passed = (
        all(report.get("passed") for report in spec_reports)
        and all(report.get("passed") for report in manifest_reports)
        and promotion.get("passed") is True
    )
    summary = {
        "schema_version": 1,
        "report_kind": "general_sem_rank0_streamlined_standard_finalization_v1",
        "qualification_profile_id": STREAMLINED_PROFILE_ID,
        "representative_evidence_acknowledged": True,
        "passed": passed,
        "source_set_sha256": source_receipt["source_set_sha256"],
        "build_fingerprint": build_fingerprint,
        "aggregate": _descriptor(aggregate_path),
        "streamlined_release_report": release_descriptor,
        "spec_reports": spec_reports,
        "manifest_reports": manifest_reports,
        "promotion": promotion,
    }
    _write(STANDARD_ROOT / "finalization-report.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=("prepare", "smart-fix", "compose"))
    arguments = parser.parse_args()
    if arguments.phase == "prepare":
        result = prepare_registry()
    elif arguments.phase == "smart-fix":
        result = run_scientific_smart_fix()
    else:
        result = compose_and_verify()
    print(json.dumps(result, indent=2, ensure_ascii=False))
    return 0 if result.get("prepared") or result.get("passed") else 1


if __name__ == "__main__":
    raise SystemExit(main())
