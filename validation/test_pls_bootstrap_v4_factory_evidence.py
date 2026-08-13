from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import pytest


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import validate_manifest, validate_manifest_document  # noqa: E402
from pls_bootstrap_release_boundary_gate import (  # noqa: E402
    MONTE_CARLO_MARGIN,
    NUMERICAL_ALLOWANCE,
    exhaustive_distribution_bijection,
    monte_carlo_equivalent,
)
from pls_bootstrap_v4_factory_common import (  # noqa: E402
    MANIFEST_PATH,
    REPORT_ROOT,
    ROOT,
    DuplicateKeyError,
    strict_load_json,
)


def test_strict_json_loader_rejects_duplicate_and_nonfinite_evidence(tmp_path: Path) -> None:
    duplicate = tmp_path / "duplicate.json"
    duplicate.write_text('{"passed": true, "passed": false}\n', encoding="utf-8")
    with pytest.raises(DuplicateKeyError):
        strict_load_json(duplicate)

    nonfinite = tmp_path / "nonfinite.json"
    nonfinite.write_text('{"value": NaN}\n', encoding="utf-8")
    with pytest.raises(ValueError, match="non-finite"):
        strict_load_json(nonfinite)


def test_exhaustive_small_sample_bootstrap_distribution_is_relabeling_invariant() -> None:
    report = exhaustive_distribution_bijection()
    assert report["passed"]
    assert report["case_count"] == 4
    assert report["ordered_draw_count"] == 4**4
    assert report["exact_distribution_equal_after_relabeling"] is True
    assert report["one_frequency_mutation_detected"] is True


def test_monte_carlo_envelope_detects_material_row_order_drift() -> None:
    seed_drift = 0.002
    boundary = MONTE_CARLO_MARGIN * seed_drift + NUMERICAL_ALLOWANCE
    assert monte_carlo_equivalent(boundary, seed_drift)
    assert not monte_carlo_equivalent(boundary + 1e-9, seed_drift)
    assert monte_carlo_equivalent(NUMERICAL_ALLOWANCE, 0.0)
    assert not monte_carlo_equivalent(NUMERICAL_ALLOWANCE * 2.0, 0.0)
    assert not monte_carlo_equivalent(-1.0, seed_drift)


def test_native_identity_reports_bind_complete_exact_sources() -> None:
    roles = (
        "method_spec",
        "independent_reference",
        "simulation_report",
        "boundary_report",
        "persistence_report",
        "frontend_report",
        "export_report",
    )
    manifest = strict_load_json(MANIFEST_PATH)
    governance = manifest["governance"]
    for role in roles:
        report = strict_load_json(REPORT_ROOT / f"{role}.identity.json")
        assert report["passed"] is True
        assert report["feature_id"] == "qpls3.inference.bootstrap"
        assert report["method_version"] == "indexed_resampling_v4"
        descriptors = {row["path"]: row for row in report["source_artifacts"]}
        required = {
            governance["manifest_path"],
            governance["schema_path"],
            governance["validator_path"],
            governance["focused_test_path"],
            *manifest["qualification"]["source_requirements"][role],
        }
        assert required <= set(descriptors)
        for relative, descriptor in descriptors.items():
            path = ROOT / relative
            assert path.is_file()
            assert descriptor["size"] == path.stat().st_size
            assert len(descriptor["sha256"]) == 64


def test_manifest_mutations_cannot_inherit_native_qualification() -> None:
    document = strict_load_json(MANIFEST_PATH)
    document["qualification"]["evidence"]["release_qualified"] = []
    current = validate_manifest_document(
        document, ROOT, manifest_path=MANIFEST_PATH, verify_evidence=True
    )
    assert current["passed"], json.dumps(current, indent=2)
    assert current["derived_state"] == "native_qualified"

    relabeled = copy.deepcopy(document)
    relabeled["feature"]["method_version"] = "indexed_resampling_v999"
    invalid_identity = validate_manifest_document(
        relabeled, ROOT, manifest_path=MANIFEST_PATH, verify_evidence=True
    )
    assert not invalid_identity["passed"]
    assert invalid_identity["derived_state"] == "absent"

    missing_boundary = copy.deepcopy(document)
    missing_boundary["qualification"]["evidence"]["engine_only"] = [
        artifact
        for artifact in missing_boundary["qualification"]["evidence"]["engine_only"]
        if "boundary_report" not in artifact["roles"]
    ]
    invalid_roles = validate_manifest_document(
        missing_boundary, ROOT, manifest_path=MANIFEST_PATH, verify_evidence=True
    )
    assert not invalid_roles["passed"]
    assert invalid_roles["derived_state"] == "absent"


def test_factory_requires_separate_packaged_release_claim() -> None:
    common = (VALIDATION / "pls_bootstrap_v4_factory_common.py").read_text(encoding="utf-8")
    evidence = (VALIDATION / "pls_bootstrap_v4_factory_evidence.py").read_text(
        encoding="utf-8"
    )
    audit = (VALIDATION / "pls_bootstrap_v4_factory_audit.py").read_text(encoding="utf-8")
    manifest = strict_load_json(MANIFEST_PATH)
    assert '"built_by_factory": False' in common
    assert '"gui_runtime_claimed": False' in evidence
    assert 'roles") == ["packaged_acceptance"]' in audit
    release_roles = sorted(
        role
        for artifact in manifest["qualification"]["evidence"]["release_qualified"]
        for role in artifact["roles"]
    )
    assert release_roles == ["method_audit", "packaged_acceptance"]
    assert "validation/pls_bootstrap_v4_packaged_acceptance.py" in manifest["qualification"]["source_requirements"]["packaged_acceptance"]
    assert manifest["qualification"]["target_state"] == "release_qualified"


def test_lightweight_evidence_derives_exact_native_qualification_before_packaged_run() -> None:
    document = strict_load_json(MANIFEST_PATH)
    document["qualification"]["evidence"]["release_qualified"] = []
    report = validate_manifest_document(
        document, ROOT, manifest_path=MANIFEST_PATH, verify_evidence=True
    )
    assert report["passed"], json.dumps(report, indent=2)
    assert report["declared_state"] == "native_qualified"
    assert report["derived_state"] == "native_qualified"
    assert report["target_state"] == "release_qualified"
