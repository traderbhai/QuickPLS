from __future__ import annotations

import json
import sys
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from method_promotion_manifest import strict_load_json, validate_manifest, validate_manifest_document  # noqa: E402
from phase3_workflow_factory import report_root  # noqa: E402


MANIFEST = ROOT / "validation/methods/higher_order_v1.manifest.json"


def test_hoc_manifest_uses_the_five_identity_paths_written_by_phase3_factory() -> None:
    document = strict_load_json(MANIFEST)
    evidence = document["qualification"]["evidence"]
    expected = {
        (report_root("higher_order_v1") / "engine_evidence.identity.json").relative_to(ROOT).as_posix(),
        (report_root("higher_order_v1") / "persistence_report.identity.json").relative_to(ROOT).as_posix(),
        (report_root("higher_order_v1") / "native_evidence.identity.json").relative_to(ROOT).as_posix(),
        (report_root("higher_order_v1") / "packaged_acceptance.identity.json").relative_to(ROOT).as_posix(),
        (report_root("higher_order_v1") / "method_audit.identity.json").relative_to(ROOT).as_posix(),
    }
    observed = {
        artifact["path"] for artifacts in evidence.values() for artifact in artifacts
    }
    assert observed == expected
    assert not any("evidence_truth_reconciliation_v1" in path for path in observed)

    roles = {
        role for artifacts in evidence.values() for artifact in artifacts for role in artifact["roles"]
    }
    assert roles == {
        "method_spec",
        "independent_reference",
        "simulation_report",
        "boundary_report",
        "persistence_report",
        "frontend_report",
        "export_report",
        "packaged_acceptance",
        "method_audit",
    }


def test_hoc_release_topology_is_valid_but_stale_or_missing_receipts_fail_closed() -> None:
    document = strict_load_json(MANIFEST)
    assert document["qualification"]["declared_state"] == "release_qualified"
    assert document["qualification"]["target_state"] == "release_qualified"
    contract = validate_manifest_document(
        document,
        ROOT,
        manifest_path=MANIFEST,
        verify_evidence=False,
    )
    assert contract["passed"], json.dumps(contract, indent=2)
    assert contract["derived_state"] == "release_qualified"

    current = validate_manifest(MANIFEST)
    assert current["passed"] is False
    assert current["derived_state"] != "release_qualified"
    assert any(
        "evidence file is missing" in error
        or "source sha256 mismatch" in error
        or "source size mismatch" in error
        for error in current["errors"]
    )
