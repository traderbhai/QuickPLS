from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

import pytest


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from cbsem_exact_case_bootstrap_v1_factory import (  # noqa: E402
    EXPECTED_PACKAGED_CHECKS,
    FACTORY_SOURCE,
    FACTORY_TEST,
    MANIFEST_PATH,
    PRIOR_AUDIT_ROLES,
    REPORT_ROOT,
    FactoryError,
    audit_prior_roles,
    source_descriptors,
    strict_load_json,
    validate_packaged_contract,
    verify_descriptors,
    write_identity_report,
)
from method_promotion_manifest import validate_manifest, validate_manifest_document  # noqa: E402


def _packaged_contract() -> dict[str, object]:
    checks = {name: {"passed": True} for name in EXPECTED_PACKAGED_CHECKS}
    checks["offline"] = {
        "passed": True,
        "evidence": {
            "passed": True,
            "analyticalWorkflowRequiresInternet": False,
            "strictZeroProcessEgressClaimed": False,
            "platformBackgroundEgressOutsidePageRequestScope": True,
            "externalRequestCount": 0,
            "externalRequests": [],
        },
    }
    checks["process_cleanup"] = {
        "passed": True,
        "sampled_process_tree_zero_egress": False,
        "network_sample_count": 1,
        "platform_background_egress_observation": {
            "passed": True,
            "observation_kind": "sampled_exact_process_tree_tcp_v1",
            "sample_count": 1,
            "root_present_every_sample": True,
            "platform_background_egress_observed": True,
            "commercial_zero_egress_passed": False,
            "remote_connections": [
                {"remote_address": "52.110.15.135", "remote_port": 443}
            ],
        },
    }
    return {
        "passed": True,
        "feature_id": "qpls3.cbsem.bootstrap",
        "method_version": "cbsem_exact_case_bootstrap_v1",
        "catalogue_snapshot_date": "2026-08-12",
        "scope": "cbsem_exact_case_bootstrap_v1",
        "generated_at_utc": "2026-08-18T03:00:00Z",
        "checks": checks,
        "binary_artifacts": {
            "desktop": {"path": "target/release/quickpls-desktop.exe", "sha256": "0" * 64},
            "cli": {"path": "target/release/qpls.exe", "sha256": "1" * 64},
        },
    }


def test_manifest_has_complete_release_factory_topology_without_claiming_missing_receipts() -> None:
    document = strict_load_json(MANIFEST_PATH)
    qualification = document["qualification"]
    assert qualification["declared_state"] == "release_qualified"
    assert qualification["target_state"] == "release_qualified"

    roles = [
        role
        for artifacts in qualification["evidence"].values()
        for artifact in artifacts
        for role in artifact["roles"]
    ]
    assert sorted(roles) == sorted((*PRIOR_AUDIT_ROLES, "method_audit"))
    assert all(
        artifact["path"].startswith(
            "validation/results/method_factory/cbsem_exact_case_bootstrap_v1/"
        )
        for artifacts in qualification["evidence"].values()
        for artifact in artifacts
    )
    contract = validate_manifest_document(
        document,
        ROOT,
        manifest_path=MANIFEST_PATH,
        verify_evidence=False,
    )
    assert contract["passed"], json.dumps(contract, indent=2)
    assert contract["derived_state"] == "release_qualified"

    current = validate_manifest(MANIFEST_PATH)
    assert current["passed"] is False
    assert current["derived_state"] == "absent"
    assert any("evidence file is missing" in error for error in current["errors"])


def test_packaged_adapter_rejects_identity_check_and_binary_contract_mutations() -> None:
    valid = _packaged_contract()
    assert validate_packaged_contract(valid) == (True, [])

    relabeled = copy.deepcopy(valid)
    relabeled["method_version"] = "cbsem_bootstrap_v2"
    assert validate_packaged_contract(relabeled)[0] is False

    missing_check = copy.deepcopy(valid)
    del missing_check["checks"]["execute_bca"]
    assert validate_packaged_contract(missing_check)[0] is False

    failed_check = copy.deepcopy(valid)
    failed_check["checks"]["save_reopen_same_run"]["passed"] = False
    assert validate_packaged_contract(failed_check)[0] is False

    external_app_request = copy.deepcopy(valid)
    external_app_request["checks"]["offline"]["evidence"]["externalRequestCount"] = 1
    assert validate_packaged_contract(external_app_request)[0] is False

    hidden_external_app_request = copy.deepcopy(valid)
    hidden_external_app_request["checks"]["offline"]["evidence"]["externalRequests"] = [{"origin": "https://example.invalid"}]
    assert validate_packaged_contract(hidden_external_app_request)[0] is False

    mislabeled_platform_egress = copy.deepcopy(valid)
    mislabeled_platform_egress["checks"]["process_cleanup"]["platform_background_egress_observation"]["commercial_zero_egress_passed"] = True
    assert validate_packaged_contract(mislabeled_platform_egress)[0] is False

    treated_platform_egress_as_method_failure = copy.deepcopy(valid)
    treated_platform_egress_as_method_failure["checks"]["process_cleanup"]["passed"] = False
    assert validate_packaged_contract(treated_platform_egress_as_method_failure)[0] is False

    missing_binary = copy.deepcopy(valid)
    del missing_binary["binary_artifacts"]["cli"]
    assert validate_packaged_contract(missing_binary)[0] is False


def test_source_descriptor_verification_detects_drift_and_missing_roles() -> None:
    descriptors = source_descriptors(ROOT, [FACTORY_SOURCE])
    assert verify_descriptors(ROOT, descriptors, [FACTORY_SOURCE]) == (True, [])

    tampered = copy.deepcopy(descriptors)
    tampered[0]["sha256"] = "0" * 64
    assert verify_descriptors(ROOT, tampered, [FACTORY_SOURCE])[0] is False
    assert verify_descriptors(ROOT, descriptors, [FACTORY_SOURCE, "missing.py"])[0] is False


def test_factory_and_compatibility_audit_fail_closed_before_refresh() -> None:
    checks = audit_prior_roles()
    assert checks["passed"] is False
    assert set(checks["role_counts"]) == set(PRIOR_AUDIT_ROLES)
    with pytest.raises(FactoryError, match="refusing to mint failed"):
        write_identity_report("method_spec", {"passed": False})

    factory = (ROOT / FACTORY_SOURCE).read_text(encoding="utf-8")
    wrapper = (ROOT / "validation/run_cbsem_exact_case_bootstrap_v1_native_acceptance.ps1").read_text(encoding="utf-8")
    assert '["cargo", "build"' not in factory
    assert "npm run build" not in factory
    assert "--adapt-packaged" in wrapper
    assert FACTORY_TEST in factory
    assert REPORT_ROOT.name == "cbsem_exact_case_bootstrap_v1"
