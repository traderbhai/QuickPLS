"""Final fail-closed release audit for PLS Bootstrap v4."""

from __future__ import annotations

import json
from typing import Any

from method_promotion_manifest import _verify_artifact
from pls_bootstrap_v4_factory_common import ROOT, manifest, strict_load_json, write_identity_report


SOURCE = "validation/pls_bootstrap_v4_factory_audit.py"
PRIOR_STAGES = ("engine_only", "archive_qualified", "native_qualified")


def evaluate_audit_inputs(document: dict[str, Any] | None = None) -> dict[str, Any]:
    document = document or manifest()
    feature = document["feature"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    rows: list[dict[str, Any]] = []
    for stage in PRIOR_STAGES:
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
            rows.append({"stage": stage, "roles": artifact["roles"], "path": artifact["path"], "passed": passed, "errors": errors})

    release = document["qualification"]["evidence"]["release_qualified"]
    roles = [role for artifact in release for role in artifact.get("roles", [])]
    release_contract = (
        len(release) == 2 and sorted(roles) == ["method_audit", "packaged_acceptance"]
        and all(len(artifact.get("roles", [])) == 1 for artifact in release)
    )
    candidates = [artifact for artifact in release if artifact.get("roles") == ["packaged_acceptance"]]
    packaged: dict[str, Any] = {}
    if len(candidates) == 1:
        artifact = candidates[0]
        passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
        try:
            packaged = strict_load_json(ROOT / artifact["path"])
        except (OSError, UnicodeError, ValueError) as error:
            passed = False
            errors = [*errors, f"cannot load packaged report: {error}"]
        rows.append({"stage": "release_qualified", "roles": ["packaged_acceptance"], "path": artifact["path"], "passed": passed, "errors": errors})
    else:
        rows.append({"stage": "release_qualified", "roles": ["packaged_acceptance"], "path": None, "passed": False, "errors": ["release evidence must contain exactly one packaged_acceptance artifact"]})

    packaged_checks = packaged.get("checks", {}) if isinstance(packaged, dict) else {}
    freshness = packaged_checks.get("source_freshness", {})
    before, after = freshness.get("before", {}), freshness.get("after", {})
    native = packaged_checks.get("native", {})
    semantic = {
        "packaged_report_passes": packaged.get("passed") is True,
        "bootstrap_native_workflow_passes": native.get("passed") is True,
        "invalid_setup_fail_closed": native.get("checks", {}).get("invalid_setup_blocked_without_run") is True,
        "selected_run_native_xlsx_passes": native.get("checks", {}).get("selected_run_native_xlsx") is True,
        "same_run_reopen_passes": native.get("checks", {}).get("same_run_reopened") is True,
        "cancellation_retry_identity_passes": native.get("checks", {}).get("cancel_retry_identity") is True,
        "three_packaged_viewports_pass": native.get("checks", {}).get("exact_three_responsive_viewports") is True,
        "functional_offline_claim_is_bounded": native.get("checks", {}).get("functional_offline_without_zero_egress_overclaim") is True,
        "archive_identity_passes": packaged_checks.get("archive", {}).get("passed") is True,
        "runner_cleanup_verified": packaged_checks.get("runner_cleanup_verified") is True,
        "receipt_and_release_cli_binding_passes": (
            freshness.get("passed") is True and freshness.get("source_stable_during_gate") is True
            and before.get("desktop_receipt_exact") is True and after.get("desktop_receipt_exact") is True
            and before.get("release_cli_newer_build_sources") == []
            and after.get("release_cli_newer_build_sources") == []
        ),
    }
    return {
        "passed": release_contract and all(row["passed"] for row in rows) and all(semantic.values()),
        "release_evidence_contract_passes": release_contract,
        "verified_artifacts": rows,
        **semantic,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }


def main() -> int:
    checks = evaluate_audit_inputs()
    paths = [row["path"] for row in checks["verified_artifacts"] if row.get("path")]
    report = write_identity_report("method_audit", passed=checks["passed"], checks=checks, extras=[SOURCE, *paths])
    print(f"wrote {report} | passed={checks['passed']}")
    if not checks["passed"]:
        print(json.dumps(checks, indent=2, sort_keys=True))
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
