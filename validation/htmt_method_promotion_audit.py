#!/usr/bin/env python3
"""Honest source-completeness audit for the unpromoted HTMT lane."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from htmt_plus_v1_factory_common import (
    MANIFEST_PATH,
    QUALIFICATION_SPEC_PATH,
    REGISTRY_PATH,
    REPORT_ROOT,
    ROOT,
    manifest,
    qualification_spec,
    sha256_file,
    strict_load_json,
    write_identity_report,
)
from htmt_release_simulation import (
    compare_bootstrap_references,
    compare_point_references,
)
from method_promotion_manifest import validate_manifest
from qualification_spec_v2 import validate_spec_document


SOURCE = "validation/htmt_method_promotion_audit.py"
SOURCE_AUDIT_ROLES = (
    "method_spec",
    "independent_reference",
    "simulation_report",
    "boundary_report",
    "persistence_report",
    "frontend_report",
    "export_report",
    "packaged_acceptance",
)


def _write_contract_source_audits() -> list[Path]:
    spec = qualification_spec()
    registry = strict_load_json(REGISTRY_PATH)
    validation = validate_spec_document(
        spec,
        registry_document=registry,
        require_registry=True,
    )
    method_report = write_identity_report(
        "method_spec",
        stage="contract",
        passed=validation["passed"] and not validation["qualification_ready"],
        checks={
            "qualification_spec_validation": validation,
            "compatibility_only": spec["migration"]["status"] == "compatibility_only",
            "receipt_count": len(spec["evidence_contract"]["receipts"]),
            "unresolved_items": spec["migration"]["unresolved_items"],
        },
        blockers=spec["migration"]["unresolved_items"],
        extras=[
            SOURCE,
            "validation/htmt_qualification_v2.py",
            "docs/methods/PLS_HTMT_V1.md",
            "docs/methods/PLS_HTMT_RELEASE_V1.md",
        ],
    )
    point = compare_point_references()
    bootstrap = compare_bootstrap_references()
    csem = strict_load_json(ROOT / "validation/results/htmt_csem_comparison.json")
    seminr = strict_load_json(ROOT / "validation/results/htmt_seminr_comparison.json")
    oracle_passed = (
        point["passed"]
        and bootstrap["passed"]
        and csem.get("status") == "passed"
        and seminr.get("status") == "passed"
    )
    oracle_report = write_identity_report(
        "independent_reference",
        stage="oracle",
        passed=oracle_passed,
        checks={
            "standard_library_vs_numpy_scipy_point": point,
            "standard_library_vs_numpy_scipy_bootstrap": bootstrap,
            "csem_original_htmt_fixture_passed": csem.get("status") == "passed",
            "seminr_htmt_plus_fixture_passed": seminr.get("status") == "passed",
            "runtime_product_dependency": False,
        },
        blockers=[
            "fresh_current_build_against_both_external_oracles_not_executed",
            "independent_oracle_bootstrap_coverage_beyond_microcases_not_executed",
        ],
        extras=[
            SOURCE,
            "validation/htmt_reference.py",
            "validation/htmt_scipy_reference.py",
            "validation/htmt_bootstrap_inference_reference.py",
            "validation/results/htmt_reference.json",
            "validation/results/htmt_scipy_reference.json",
            "validation/results/htmt_bootstrap_inference_reference.json",
            "validation/results/htmt_csem_comparison.json",
            "validation/results/htmt_seminr_comparison.json",
        ],
    )
    return [method_report, oracle_report]


def _registry_cell(registry: dict[str, Any]) -> tuple[dict[str, Any], dict[str, Any]]:
    capability = next(
        row
        for row in registry["capabilities"]
        if row["capability_id"] == "smartpls.htmt"
    )
    cell = next(
        row
        for row in capability["option_cells"]
        if row["cell_id"] == "qpls3.assessment.htmt"
    )
    return capability, cell


def run_audit() -> dict[str, Any]:
    _write_contract_source_audits()
    legacy = manifest()
    spec = qualification_spec()
    registry = strict_load_json(REGISTRY_PATH)
    capability, cell = _registry_cell(registry)
    v1 = validate_manifest(MANIFEST_PATH, ROOT)
    v2 = validate_spec_document(
        spec,
        registry_document=registry,
        require_registry=True,
    )
    source_requirements = legacy["qualification"]["source_requirements"]
    missing_sources = sorted(
        {
            path
            for paths in source_requirements.values()
            for path in paths
            if not (ROOT / path).is_file()
        }
    )
    evidence = legacy["qualification"]["evidence"]
    admitted_counts = {stage: len(rows) for stage, rows in evidence.items()}
    source_audits: dict[str, Any] = {}
    missing_audits: list[str] = []
    invalid_audits: list[str] = []
    blockers: set[str] = set()
    for role in SOURCE_AUDIT_ROLES:
        path = REPORT_ROOT / f"{role}.source_audit.json"
        if not path.is_file():
            missing_audits.append(role)
            continue
        report = strict_load_json(path)
        source_artifacts = report.get("source_artifacts", [])
        source_bytes_current = bool(source_artifacts)
        for artifact in source_artifacts:
            if not isinstance(artifact, dict) or not isinstance(
                artifact.get("path"), str
            ):
                source_bytes_current = False
                continue
            source_path = ROOT / artifact["path"]
            source_bytes_current = source_bytes_current and (
                source_path.is_file()
                and source_path.stat().st_size == artifact.get("size_bytes")
                and sha256_file(source_path) == artifact.get("sha256")
            )
        identity_current = (
            report.get("report_kind") == "quickpls_htmt_qualification_identity_report"
            and report.get("role") == role
            and report.get("feature_id") == "qpls3.assessment.htmt"
            and report.get("qualification_id")
            == "qpls3.assessment.htmt.qualification_v2"
            and report.get("method_version") == "ringle_et_al_htmt_plus_v1"
            and report.get("manifest_sha256") == sha256_file(MANIFEST_PATH)
            and report.get("qualification_spec_sha256")
            == sha256_file(QUALIFICATION_SPEC_PATH)
        )
        source_audits[role] = {
            "path": path.relative_to(ROOT).as_posix(),
            "sha256": sha256_file(path),
            "passed": report.get("passed"),
            "qualification_evidence": report.get("qualification_evidence"),
            "blockers": report.get("blockers"),
            "identity_current": identity_current,
            "source_bytes_current": source_bytes_current,
        }
        if (
            report.get("passed") is not True
            or report.get("qualification_evidence") is not False
            or not report.get("blockers")
            or not identity_current
            or not source_bytes_current
        ):
            invalid_audits.append(role)
        for blocker in report.get("blockers", []):
            if isinstance(blocker, str):
                blockers.add(blocker)
    expected_absent = (
        capability["coverage_state"] == "absent"
        and capability["evidence_state"] == "absent"
        and capability["surface"] == "labs"
        and cell["coverage_state"] == "absent"
        and cell["evidence_state"] == "absent"
        and cell["surface"] == "labs"
        and legacy["qualification"]["declared_state"] == "absent"
        and v1["derived_state"] == "absent"
        and all(count == 0 for count in admitted_counts.values())
    )
    source_complete = (
        not missing_sources
        and not missing_audits
        and not invalid_audits
        and v1["passed"]
        and v2["passed"]
        and v2["registry_verified"]
        and not v2["qualification_ready"]
        and expected_absent
    )
    return {
        "schema_version": 1,
        "kind": "quickpls_htmt_qualification_gap_report",
        "generated_at_utc": datetime.now(timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "passed": source_complete,
        "source_complete": source_complete,
        "qualification_ready": False,
        "promotion_authority": False,
        "feature_id": "qpls3.assessment.htmt",
        "qualification_id": "qpls3.assessment.htmt.qualification_v2",
        "capability_id": "smartpls.htmt",
        "cell_id": "qpls3.assessment.htmt",
        "method_version": "ringle_et_al_htmt_plus_v1",
        "manifest_sha256": sha256_file(MANIFEST_PATH),
        "qualification_spec_sha256": sha256_file(QUALIFICATION_SPEC_PATH),
        "registry_sha256": sha256_file(REGISTRY_PATH),
        "state": {
            "coverage_state": cell["coverage_state"],
            "evidence_state": cell["evidence_state"],
            "surface": cell["surface"],
            "v1_declared_state": legacy["qualification"]["declared_state"],
            "v1_derived_state": v1["derived_state"],
            "v2_migration_status": spec["migration"]["status"],
            "v2_receipt_count": len(spec["evidence_contract"]["receipts"]),
        },
        "admitted_evidence_counts": admitted_counts,
        "required_v1_roles": sorted(source_requirements),
        "required_v2_receipt_roles": sorted(
            spec["evidence_contract"]["required_roles"]
        ),
        "source_audits": source_audits,
        "missing_sources": missing_sources,
        "missing_source_audits": missing_audits,
        "invalid_source_audits": invalid_audits,
        "product_breadth_blockers": spec["migration"]["unresolved_items"],
        "execution_and_evidence_blockers": sorted(blockers),
        "decision": "remain_absent_labs_unpromoted",
    }


def main() -> int:
    report = run_audit()
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    path = REPORT_ROOT / "qualification_gap.json"
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "passed": report["passed"],
                "source_complete": report["source_complete"],
                "qualification_ready": report["qualification_ready"],
                "decision": report["decision"],
                "report": path.relative_to(ROOT).as_posix(),
                "missing_sources": report["missing_sources"],
                "missing_source_audits": report["missing_source_audits"],
                "invalid_source_audits": report["invalid_source_audits"],
            },
            indent=2,
            sort_keys=True,
        )
    )
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
