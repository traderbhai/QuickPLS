"""Final fail-closed method audit for the PLS Algorithm v1 factory slice.

The audit intentionally verifies every prerequisite except its own output. The
packaged adapter writes this audit and only then asks the manifest validator to
derive the final release-qualified state.
"""

from __future__ import annotations

import json
from typing import Any

from method_promotion_manifest import _verify_artifact
from pls_algorithm_v1_factory_common import (
    ROOT,
    manifest,
    strict_load_json,
    write_identity_report,
)


SOURCE = "validation/pls_algorithm_v1_factory_audit.py"
PRIOR_STAGES = ("engine_only", "archive_qualified", "native_qualified")


def evaluate_audit_inputs(document: dict[str, Any] | None = None) -> dict[str, Any]:
    """Verify prerequisite artifacts without consuming ``method_audit``."""

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
            passed, errors = _verify_artifact(
                artifact,
                document,
                ROOT,
                expected_identity,
            )
            rows.append(
                {
                    "stage": stage,
                    "roles": artifact["roles"],
                    "path": artifact["path"],
                    "passed": passed,
                    "errors": errors,
                }
            )

    release_artifacts = document["qualification"]["evidence"]["release_qualified"]
    release_roles = [
        role for artifact in release_artifacts for role in artifact.get("roles", [])
    ]
    release_contract_passed = (
        len(release_artifacts) == 2
        and sorted(release_roles) == ["method_audit", "packaged_acceptance"]
        and all(len(artifact.get("roles", [])) == 1 for artifact in release_artifacts)
    )
    packaged_candidates = [
        artifact
        for artifact in release_artifacts
        if artifact.get("roles") == ["packaged_acceptance"]
    ]
    packaged: dict[str, Any] = {}
    if len(packaged_candidates) == 1:
        packaged_artifact = packaged_candidates[0]
        packaged_passed, packaged_errors = _verify_artifact(
            packaged_artifact,
            document,
            ROOT,
            expected_identity,
        )
        try:
            packaged = strict_load_json(ROOT / packaged_artifact["path"])
        except (OSError, UnicodeError, ValueError) as error:
            packaged_passed = False
            packaged_errors = [*packaged_errors, f"cannot load packaged report: {error}"]
        rows.append(
            {
                "stage": "release_qualified",
                "roles": ["packaged_acceptance"],
                "path": packaged_artifact["path"],
                "passed": packaged_passed,
                "errors": packaged_errors,
            }
        )
    else:
        rows.append(
            {
                "stage": "release_qualified",
                "roles": ["packaged_acceptance"],
                "path": None,
                "passed": False,
                "errors": [
                    "release evidence must contain exactly one packaged_acceptance artifact"
                ],
            }
        )

    packaged_checks = packaged.get("checks", {}) if isinstance(packaged, dict) else {}
    freshness = packaged_checks.get("source_freshness", {})
    freshness_before = freshness.get("before", {})
    freshness_after = freshness.get("after", {})
    source_binding_passed = (
        freshness.get("passed") is True
        and freshness.get("source_stable_during_gate") is True
        and freshness_before.get("desktop_receipt_exact") is True
        and freshness_after.get("desktop_receipt_exact") is True
        and freshness_before.get("release_cli_newer_build_sources") == []
        and freshness_after.get("release_cli_newer_build_sources") == []
    )
    semantic_checks = {
        "packaged_report_passes": packaged.get("passed") is True,
        "native_workflow_passes": packaged_checks.get("native", {}).get("passed") is True,
        "three_required_viewports_pass": packaged_checks.get(
            "responsive_viewports", {}
        ).get("passed")
        is True,
        "runner_cleanup_verified": packaged_checks.get("runner_cleanup_verified") is True,
        "receipt_and_release_cli_source_binding_passes": source_binding_passed,
    }
    return {
        "passed": (
            release_contract_passed
            and all(row["passed"] for row in rows)
            and all(semantic_checks.values())
        ),
        "release_evidence_contract_passes": release_contract_passed,
        "verified_artifacts": rows,
        **semantic_checks,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }


def main() -> int:
    document = manifest()
    checks = evaluate_audit_inputs(document)
    prerequisite_paths = [
        row["path"] for row in checks["verified_artifacts"] if row.get("path")
    ]
    report = write_identity_report(
        "method_audit",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE, *prerequisite_paths],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    if not checks["passed"]:
        print(json.dumps(checks, indent=2, sort_keys=True))
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
