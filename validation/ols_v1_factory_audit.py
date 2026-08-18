#!/usr/bin/env python3
"""Independent, non-circular final release audit for bounded OLS v1."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

try:
    from validation.method_promotion_manifest import _verify_artifact
    from validation.ols_v1_factory import (
        MANIFEST,
        ROOT,
        strict_json,
        write_identity,
    )
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.method_promotion_manifest import _verify_artifact  # type: ignore[no-redef]
    from validation.ols_v1_factory import (  # type: ignore[no-redef]
        MANIFEST,
        ROOT,
        strict_json,
        write_identity,
    )


SOURCE = "validation/ols_v1_factory_audit.py"


def evaluate_audit_inputs(document: dict[str, Any] | None = None) -> dict[str, Any]:
    document = document or strict_json(MANIFEST)
    feature = document["feature"]
    identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    verified: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            verified.append(
                {
                    "stage": stage,
                    "path": artifact["path"],
                    "roles": artifact["roles"],
                    "passed": passed,
                    "errors": errors,
                }
            )

    release = document["qualification"]["evidence"]["release_qualified"]
    role_rows = [role for artifact in release for role in artifact.get("roles", [])]
    release_contract = (
        len(release) == 2
        and sorted(role_rows) == ["method_audit", "packaged_acceptance"]
        and all(len(artifact.get("roles", [])) == 1 for artifact in release)
    )
    packaged_rows = [
        artifact for artifact in release if artifact.get("roles") == ["packaged_acceptance"]
    ]
    packaged: dict[str, Any] = {}
    packaged_passed = False
    packaged_errors = ["missing packaged_acceptance"]
    packaged_path: str | None = None
    if len(packaged_rows) == 1:
        packaged_row = packaged_rows[0]
        packaged_path = packaged_row["path"]
        packaged_passed, packaged_errors = _verify_artifact(
            packaged_row, document, ROOT, identity
        )
        try:
            packaged = strict_json(ROOT / packaged_path)
        except (OSError, UnicodeError, ValueError) as error:
            packaged_passed = False
            packaged_errors = [*packaged_errors, f"{type(error).__name__}: {error}"]
    verified.append(
        {
            "stage": "release_qualified",
            "path": packaged_path,
            "roles": ["packaged_acceptance"],
            "passed": packaged_passed,
            "errors": packaged_errors,
        }
    )

    checks = packaged.get("checks", {}) if isinstance(packaged, dict) else {}
    semantic = {
        "packaged_identity_passes": packaged.get("passed") is True,
        "prior_factory_native": checks.get("prior_factory") == "native_qualified",
        "source_receipt_and_cli_current": checks.get("source_freshness", {}).get("passed") is True
        and checks.get("source_freshness", {}).get("desktop_receipt_exact") is True
        and checks.get("source_freshness", {}).get("release_cli_newer_sources") == [],
        "bounded_packaged_workflow_passes": checks.get("packaged", {}).get("passed") is True
        and checks.get("packaged", {}).get("same_run_reopened") is True,
        "functional_offline_not_zero_egress": checks.get("packaged", {}).get("functional_offline") is True
        and checks.get("packaged", {}).get("strict_zero_process_egress_claimed") is False,
        "three_viewports_pass": checks.get("visual", {}).get("passed") is True
        and checks.get("visual", {}).get("viewports")
        == ["1024x700", "1280x720", "1440x900"],
        "cumulative_cleanup_passes": checks.get("cleanup", {}).get("passed") is True,
    }
    return {
        "passed": release_contract
        and all(row["passed"] for row in verified)
        and all(semantic.values()),
        "release_evidence_contract_passes": release_contract,
        "verified_artifacts": verified,
        **semantic,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }


def main() -> int:
    checks = evaluate_audit_inputs()
    extras = [
        SOURCE,
        *[
            row["path"]
            for row in checks["verified_artifacts"]
            if isinstance(row.get("path"), str)
        ],
    ]
    output = write_identity(
        "method_audit.identity.json",
        ["method_audit"],
        {"final_release_audit": {"passed": checks["passed"], **checks}},
        extras=extras,
    )
    print(f"wrote {output} | passed={checks['passed']}")
    if not checks["passed"]:
        print(json.dumps(checks, indent=2, sort_keys=True))
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
