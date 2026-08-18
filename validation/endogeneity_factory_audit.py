"""Final fail-closed audit for Gaussian-copula endogeneity evidence."""

from __future__ import annotations

import argparse
from typing import Any

from endogeneity_factory_common import (
    ROOT,
    manifest,
    optionally_write_identity_report,
)
from method_promotion_manifest import _verify_artifact


SOURCE = "validation/endogeneity_factory_audit.py"


def audit() -> dict[str, Any]:
    document = manifest()
    feature = document["feature"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    expected_roles = {
        "engine_only": {
            "method_spec",
            "independent_reference",
            "simulation_report",
            "boundary_report",
        },
        "archive_qualified": {"persistence_report"},
        "native_qualified": {"frontend_report", "export_report"},
    }
    rows: list[dict[str, Any]] = []
    for stage, roles in expected_roles.items():
        observed: set[str] = set()
        for artifact in document["qualification"]["evidence"][stage]:
            observed.update(artifact["roles"])
            passed, errors = _verify_artifact(
                artifact, document, ROOT, expected_identity
            )
            rows.append(
                {
                    "stage": stage,
                    "path": artifact["path"],
                    "roles": artifact["roles"],
                    "passed": passed,
                    "errors": errors,
                }
            )
        if observed != roles:
            rows.append(
                {
                    "stage": stage,
                    "passed": False,
                    "errors": [
                        f"expected roles {sorted(roles)}, found {sorted(observed)}"
                    ],
                }
            )

    packaged_candidates = [
        artifact
        for artifact in document["qualification"]["evidence"]["release_qualified"]
        if artifact["roles"] == ["packaged_acceptance"]
    ]
    if len(packaged_candidates) == 1:
        packaged_passed, packaged_errors = _verify_artifact(
            packaged_candidates[0], document, ROOT, expected_identity
        )
        packaged_row = {
            "stage": "release_qualified",
            "path": packaged_candidates[0]["path"],
            "roles": ["packaged_acceptance"],
            "passed": packaged_passed,
            "errors": packaged_errors,
        }
    else:
        packaged_passed = False
        packaged_row = {
            "stage": "release_qualified",
            "roles": ["packaged_acceptance"],
            "passed": False,
            "errors": [
                "exactly one current focused packaged acceptance identity is required"
            ],
        }
    rows.append(packaged_row)
    prior_passed = all(row["passed"] for row in rows[:-1])
    return {
        "passed": prior_passed and packaged_passed,
        "stage_artifacts": rows,
        "scientific_archive_native_evidence_current": prior_passed,
        "packaged_acceptance_current": packaged_passed,
        "release_claim_blocked_without_packaged_acceptance": not packaged_passed,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-identity", action="store_true")
    args = parser.parse_args()
    checks = audit()
    # Never write a passing audit unless every prerequisite, including the
    # focused packaged workflow, is current and exact.
    report = optionally_write_identity_report(
        "method_audit",
        write_identity=args.write_identity and checks["passed"],
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE, "validation/run_endogeneity_native_acceptance.ps1"],
    )
    print(
        f"endogeneity final audit passed={checks['passed']} "
        f"identity={report or 'not-written'}"
    )
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
