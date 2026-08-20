#!/usr/bin/env python3
"""Fail-closed, cell-atomic Rank 0 Standard promotion audit.

The audit never edits the Capability Registry, method manifests, qualification
specifications, or evidence.  It distinguishes three states for each exact
cell:

* ``blocked``: current immutable evidence is incomplete or invalid;
* ``promotion_candidate``: evidence is complete, but the Registry still keeps
  the cell in Labs;
* ``standard_active``: evidence is complete and the exact Registry cell is
  ``partial|full / release_qualified / standard``.

That split lets the release coordinator promote cells independently without
allowing a failed bootstrap qualification to hold back a qualified point cell.
Conversely, a Standard Registry label without current evidence is an explicit
over-promotion failure.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Mapping, Sequence


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import validate_manifest  # noqa: E402
from qualification_spec_v2 import strict_load_json, validate_spec_path  # noqa: E402
from general_sem_rank0_receipt_payload_v1 import CONTRACT_ID  # noqa: E402


REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"


@dataclass(frozen=True)
class Rank0Cell:
    key: str
    capability_id: str
    cell_id: str
    capability_version: str
    qualification_spec_path: str
    method_manifest_path: str


RANK0_CELLS: tuple[Rank0Cell, ...] = (
    Rank0Cell(
        key="mediation_point",
        capability_id="smartpls.mediation",
        cell_id="qpls3.pls.mediation",
        capability_version="pls_mediation_v1",
        qualification_spec_path="validation/qualification_v2/mediation_v1.qualification.json",
        method_manifest_path="validation/methods/mediation_v1.manifest.json",
    ),
    Rank0Cell(
        key="mediation_bootstrap",
        capability_id="smartpls.mediation",
        cell_id="qpls3.pls.general_sem_multiple_mediation_bootstrap",
        capability_version="general_sem_pls_full_model_case_bootstrap_v1",
        qualification_spec_path=(
            "validation/qualification_v2/"
            "general_sem_pls_multiple_mediation_bootstrap_v1.qualification.json"
        ),
        method_manifest_path=(
            "validation/methods/"
            "general_sem_pls_multiple_mediation_bootstrap_v1.manifest.json"
        ),
    ),
    Rank0Cell(
        key="moderation_point",
        capability_id="smartpls.moderation",
        cell_id="qpls3.pls.general_sem_multiple_two_way_moderation_point",
        capability_version="general_sem_pls_multiple_two_way_moderation_point_v1",
        qualification_spec_path=(
            "validation/qualification_v2/"
            "general_sem_pls_multiple_moderation_point_v1.qualification.json"
        ),
        method_manifest_path=(
            "validation/methods/general_sem_pls_multiple_moderation_point_v1.manifest.json"
        ),
    ),
    Rank0Cell(
        key="moderation_bootstrap",
        capability_id="smartpls.moderation",
        cell_id="qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
        capability_version=(
            "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
        ),
        qualification_spec_path=(
            "validation/qualification_v2/"
            "general_sem_pls_multiple_moderation_bootstrap_v1.qualification.json"
        ),
        method_manifest_path=(
            "validation/methods/"
            "general_sem_pls_multiple_moderation_bootstrap_v1.manifest.json"
        ),
    ),
)


class PromotionAuditError(ValueError):
    """Raised when the Registry cannot be inspected unambiguously."""


def _registry_cell(
    registry: Mapping[str, Any], definition: Rank0Cell
) -> Mapping[str, Any]:
    matches: list[Mapping[str, Any]] = []
    rows = registry.get("capabilities")
    if not isinstance(rows, list):
        raise PromotionAuditError("Capability Registry has no capabilities array")
    for row in rows:
        if not isinstance(row, Mapping):
            continue
        option_cells = row.get("option_cells")
        if not isinstance(option_cells, list):
            continue
        for option in option_cells:
            if not isinstance(option, Mapping):
                continue
            if (
                option.get("capability_id") == definition.capability_id
                and option.get("cell_id") == definition.cell_id
                and option.get("capability_version") == definition.capability_version
            ):
                matches.append(option)
    if len(matches) != 1:
        raise PromotionAuditError(
            f"{definition.key}: expected one exact Registry cell, found {len(matches)}"
        )
    return matches[0]


def evaluate_cell_reports(
    definition: Rank0Cell,
    *,
    registry_cell: Mapping[str, Any],
    qualification_report: Mapping[str, Any],
    manifest_report: Mapping[str, Any],
) -> dict[str, Any]:
    """Evaluate one exact cell from already-produced validator reports."""

    errors: list[str] = []
    expected_identity = {
        "capability_id": definition.capability_id,
        "cell_id": definition.cell_id,
        "method_version": definition.capability_version,
    }
    for field, expected in expected_identity.items():
        if qualification_report.get(field) != expected:
            errors.append(
                f"QualificationSpec {field} mismatch: expected {expected!r}, "
                f"found {qualification_report.get(field)!r}"
            )
    if qualification_report.get("receipt_payload_contract_id") != CONTRACT_ID:
        errors.append("QualificationSpec does not select the strict Rank 0 receipt payload contract")
    if qualification_report.get("receipt_payload_contract_verified") is not True:
        errors.append("strict Rank 0 receipt payloads were not verified")
    if manifest_report.get("feature_id") != definition.cell_id:
        errors.append("method manifest feature_id does not match the exact cell")
    if manifest_report.get("method_version") != definition.capability_version:
        errors.append("method manifest version does not match the exact cell")

    qualification_ready = bool(
        qualification_report.get("passed")
        and qualification_report.get("qualification_ready")
    )
    manifest_ready = bool(
        manifest_report.get("passed")
        and manifest_report.get("declared_state") == "release_qualified"
        and manifest_report.get("derived_state") == "release_qualified"
        and manifest_report.get("target_state") == "release_qualified"
    )
    evidence_ready = qualification_ready and manifest_ready and not errors

    coverage = registry_cell.get("coverage_state")
    evidence = registry_cell.get("evidence_state")
    surface = registry_cell.get("surface")
    registry_standard = (
        coverage in {"partial", "full"}
        and evidence == "release_qualified"
        and surface == "standard"
    )
    registry_labs = (
        coverage in {"partial", "full"}
        and evidence != "absent"
        and surface == "labs"
    )
    if surface == "standard" and not registry_standard:
        errors.append(
            "Registry Standard surface is not paired with release_qualified evidence"
        )
    if registry_standard and not evidence_ready:
        errors.append(
            "Registry over-promotion: Standard is active without current immutable evidence"
        )

    if errors or not evidence_ready:
        state = "blocked"
    elif registry_standard:
        state = "standard_active"
    elif registry_labs:
        state = "promotion_candidate"
    else:
        state = "blocked"
        errors.append(
            "evidence is ready, but the Registry is neither an exact Labs candidate nor Standard"
        )

    return {
        "key": definition.key,
        "capability_id": definition.capability_id,
        "cell_id": definition.cell_id,
        "capability_version": definition.capability_version,
        "state": state,
        "evidence_ready": evidence_ready,
        "registry_standard": registry_standard,
        "qualification": dict(qualification_report),
        "method_manifest": dict(manifest_report),
        "registry": {
            "coverage_state": coverage,
            "evidence_state": evidence,
            "surface": surface,
        },
        "errors": errors,
    }


def audit_rank0_standard_promotion(
    repository_root: Path = ROOT,
    *,
    registry_path: Path | None = None,
    definitions: Sequence[Rank0Cell] = RANK0_CELLS,
) -> dict[str, Any]:
    """Run current-byte strict evidence, manifest, and Registry validation."""

    root = repository_root.resolve()
    registry_file = (registry_path or (root / REGISTRY_PATH.relative_to(ROOT))).resolve()
    registry = strict_load_json(registry_file)
    if not isinstance(registry, Mapping):
        raise PromotionAuditError("Capability Registry root must be an object")

    cell_reports: list[dict[str, Any]] = []
    for definition in definitions:
        spec_path = root / definition.qualification_spec_path
        manifest_path = root / definition.method_manifest_path
        qualification_report = validate_spec_path(
            spec_path,
            repository_root=root,
            verify_receipts=True,
            registry_path=registry_file,
            require_registry=True,
        )
        manifest_report = validate_manifest(
            manifest_path, root, verify_evidence=True
        )
        cell_reports.append(
            evaluate_cell_reports(
                definition,
                registry_cell=_registry_cell(registry, definition),
                qualification_report=qualification_report,
                manifest_report=manifest_report,
            )
        )

    standard_ids = [
        row["cell_id"] for row in cell_reports if row["state"] == "standard_active"
    ]
    candidate_ids = [
        row["cell_id"] for row in cell_reports if row["state"] == "promotion_candidate"
    ]
    blocked_ids = [
        row["cell_id"] for row in cell_reports if row["state"] == "blocked"
    ]
    return {
        "schema_version": 1,
        "report_kind": "quickpls_general_sem_rank0_standard_promotion_audit",
        "passed": len(standard_ids) == len(definitions),
        "cell_atomic": True,
        "required_cell_count": len(definitions),
        "standard_active_cell_ids": standard_ids,
        "promotion_candidate_cell_ids": candidate_ids,
        "blocked_cell_ids": blocked_ids,
        "cells": cell_reports,
    }


def _pretty(value: Any) -> str:
    return json.dumps(value, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repository-root", type=Path, default=ROOT)
    parser.add_argument("--registry", type=Path)
    parser.add_argument(
        "--require-complete",
        action="store_true",
        help="Exit nonzero unless all four exact cells are Standard-active.",
    )
    arguments = parser.parse_args(argv)
    try:
        report = audit_rank0_standard_promotion(
            arguments.repository_root,
            registry_path=arguments.registry,
        )
    except (OSError, UnicodeError, ValueError, json.JSONDecodeError) as error:
        report = {
            "schema_version": 1,
            "report_kind": "quickpls_general_sem_rank0_standard_promotion_audit",
            "passed": False,
            "errors": [f"{type(error).__name__}: {error}"],
        }
    print(_pretty(report), end="")
    return 0 if report.get("passed") or not arguments.require_complete else 1


if __name__ == "__main__":
    raise SystemExit(main())
