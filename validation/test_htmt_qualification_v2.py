from __future__ import annotations

import json
import sys
from pathlib import Path

import pytest


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from htmt_native_export_audit import source_contract as native_source_contract  # noqa: E402
from htmt_packaged_acceptance import (  # noqa: E402
    required_case_ids as packaged_case_ids,
    source_contract as packaged_source_contract,
)
from htmt_plus_v1_factory_common import (  # noqa: E402
    MANIFEST_PATH,
    QUALIFICATION_SPEC_PATH,
    REGISTRY_PATH,
    DuplicateKeyError,
    build_identity_report,
    manifest,
    qualification_spec,
    source_descriptors,
    strict_load_json,
)
from htmt_qualification_v2 import build_spec, verify  # noqa: E402
from htmt_release_boundary_gate import (  # noqa: E402
    REQUIRED_CASE_IDS as BOUNDARY_CASE_IDS,
    source_boundary_checks,
    validate_qualification_report as validate_boundary_report,
)
from htmt_release_persistence_gate import (  # noqa: E402
    REQUIRED_CASE_IDS as PERSISTENCE_CASE_IDS,
    source_persistence_checks,
)
from htmt_release_simulation import (  # noqa: E402
    compare_bootstrap_references,
    compare_point_references,
    preregistration_check,
)
from method_promotion_manifest import validate_manifest  # noqa: E402


def test_frozen_v2_spec_is_generated_exactly_and_remains_unqualified() -> None:
    frozen = qualification_spec()
    assert frozen == build_spec()
    result = verify(frozen)
    assert result["passed"], result
    assert result["registry_verified"], result
    assert not result["qualification_ready"], result
    assert frozen["migration"]["status"] == "compatibility_only"
    assert frozen["migration"]["unresolved_items"]
    assert frozen["evidence_contract"]["receipts"] == []


def test_registry_and_v1_manifest_remain_absent_labs_with_no_evidence() -> None:
    registry = strict_load_json(REGISTRY_PATH)
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
    assert (
        capability["coverage_state"],
        capability["evidence_state"],
        capability["surface"],
    ) == (
        "absent",
        "absent",
        "labs",
    )
    assert (cell["coverage_state"], cell["evidence_state"], cell["surface"]) == (
        "absent",
        "absent",
        "labs",
    )
    document = manifest()
    assert document["qualification"]["declared_state"] == "absent"
    assert all(not rows for rows in document["qualification"]["evidence"].values())
    report = validate_manifest(MANIFEST_PATH, ROOT)
    assert report["passed"], report
    assert report["derived_state"] == "absent"


def test_every_manifest_source_requirement_exists_and_is_hashable() -> None:
    requirements = manifest()["qualification"]["source_requirements"]
    for role, paths in requirements.items():
        assert paths, role
        assert len(paths) == len(set(paths)), role
        descriptors = source_descriptors(paths)
        assert [row["path"] for row in descriptors] == sorted(paths)
        assert all(
            len(row["sha256"]) == 64 and row["size_bytes"] > 0 for row in descriptors
        )


def test_factory_loader_rejects_duplicate_and_nonfinite_json(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"passed":true,"passed":false}\n', encoding="utf-8")
    with pytest.raises(DuplicateKeyError):
        strict_load_json(duplicate)
    nonfinite = tmp_path / "nonfinite.json"
    nonfinite.write_text('{"value":NaN}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="non-finite"):
        strict_load_json(nonfinite)


def test_identity_factory_cannot_label_blocked_source_audit_as_evidence() -> None:
    with pytest.raises(ValueError, match="no blockers"):
        build_identity_report(
            "method_spec",
            stage="contract",
            passed=True,
            checks={"source_complete": True},
            blockers=["qualification_not_run"],
            qualification_evidence=True,
        )
    report = build_identity_report(
        "method_spec",
        stage="contract",
        passed=True,
        checks={"source_complete": True},
        blockers=["qualification_not_run"],
    )
    assert report["passed"] is True
    assert report["qualification_evidence"] is False
    assert report["qualification_ready"] is False
    assert report["declared_manifest_state"] == "absent"


def test_two_transparent_oracles_and_preregistration_agree() -> None:
    point = compare_point_references()
    bootstrap = compare_bootstrap_references()
    preregistration = preregistration_check()
    assert point["passed"], point
    assert point["maximum_absolute_error"] <= 1e-10
    assert bootstrap["passed"], bootstrap
    assert bootstrap["maximum_absolute_error"] <= 1e-10
    assert preregistration["passed"], preregistration
    assert preregistration["minimum_worst_case_replications"] >= 9604
    assert preregistration["maximum_half_width"] <= 0.01
    assert preregistration["failed_fits_in_denominator"] is True


def test_boundary_and_persistence_source_contracts_are_complete() -> None:
    boundary = source_boundary_checks()
    persistence = source_persistence_checks()
    assert boundary["passed"], boundary
    assert set(boundary["qualification_case_ids"]) == set(BOUNDARY_CASE_IDS)
    assert persistence["passed"], persistence
    assert set(persistence["qualification_case_ids"]) == set(PERSISTENCE_CASE_IDS)


def test_boundary_report_requires_exact_case_membership(tmp_path: Path) -> None:
    incomplete = {
        "qualification_id": "qpls3.assessment.htmt.qualification_v2",
        "method_version": "ringle_et_al_htmt_plus_v1",
        "failed_cases": [],
        "untyped_failures": 0,
        "cases": [{"id": BOUNDARY_CASE_IDS[0], "passed": True}],
    }
    path = tmp_path / "incomplete.json"
    path.write_text(json.dumps(incomplete), encoding="utf-8")
    result = validate_boundary_report(path)
    assert not result["passed"]
    assert result["cases"]["missing_case_ids"]


def test_native_export_and_packaged_source_contracts_are_explicit() -> None:
    native = native_source_contract()
    packaged = packaged_source_contract()
    assert native["passed"], native
    assert packaged["passed"], packaged
    assert len(packaged_case_ids()) == len(set(packaged_case_ids()))
    assert any(case.startswith("installed_") for case in packaged_case_ids())
    assert any(case.startswith("portable_") for case in packaged_case_ids())
    assert "cancel_archive_unchanged" in packaged_case_ids()


def test_spec_paths_and_schema_are_real_repository_artifacts() -> None:
    assert QUALIFICATION_SPEC_PATH.is_file()
    assert (VALIDATION / "qualification_v2/qualification_spec_v2.schema.json").is_file()
    assert (VALIDATION / "qualification_spec_v2.py").is_file()
