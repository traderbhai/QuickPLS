"""Final fail-closed release audit for bounded PLSc v2."""

from __future__ import annotations

import json
from typing import Any

from method_promotion_manifest import _verify_artifact
from plsc_v2_factory_common import ROOT, manifest, strict_load_json, write_identity_report


SOURCE = "validation/plsc_v2_factory_audit.py"


def evaluate_audit_inputs(document: dict[str, Any] | None = None) -> dict[str, Any]:
    document = document or manifest()
    feature = document["feature"]
    identity = {"passed": True, "feature_id": feature["id"], "method_version": feature["method_version"], "catalogue_snapshot_date": feature["catalogue_snapshot_date"]}
    rows: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            rows.append({"stage": stage, "roles": artifact["roles"], "path": artifact["path"], "passed": passed, "errors": errors})
    release = document["qualification"]["evidence"]["release_qualified"]
    roles = [role for artifact in release for role in artifact.get("roles", [])]
    contract = len(release) == 2 and sorted(roles) == ["method_audit", "packaged_acceptance"] and all(len(row.get("roles", [])) == 1 for row in release)
    candidates = [row for row in release if row.get("roles") == ["packaged_acceptance"]]
    packaged: dict[str, Any] = {}
    packaged_passed, packaged_errors, packaged_path = False, ["missing packaged_acceptance"], None
    if len(candidates) == 1:
        packaged_path = candidates[0]["path"]
        packaged_passed, packaged_errors = _verify_artifact(candidates[0], document, ROOT, identity)
        try:
            packaged = strict_load_json(ROOT / packaged_path)
        except (OSError, UnicodeError, ValueError) as error:
            packaged_passed, packaged_errors = False, [*packaged_errors, str(error)]
    rows.append({"stage": "release_qualified", "roles": ["packaged_acceptance"], "path": packaged_path, "passed": packaged_passed, "errors": packaged_errors})
    checks = packaged.get("checks", {}) if isinstance(packaged, dict) else {}
    native, visual, freshness = checks.get("native", {}), checks.get("responsive_viewports", {}), checks.get("source_freshness", {})
    semantic = {
        "packaged_report_passes": packaged.get("passed") is True,
        "invalid_scope_fails_closed": native.get("checks", {}).get("invalid_scope_blocked") is True and native.get("checks", {}).get("invalid_scope_created_no_state") is True,
        "exact_run_export_and_reopen_pass": all(native.get("checks", {}).get(name) is True for name in ("export_bound_to_exact_run", "xlsx_created_and_read_back", "xlsx_method_identity", "xlsx_receipt_exact", "same_run_reopened", "reopened_tables_exact")),
        "three_required_viewports_pass": visual.get("passed") is True,
        "receipt_and_cleanup_pass": checks.get("cumulative_receipt", {}).get("passed") is True and checks.get("runner_cleanup_verified") is True,
        "source_binding_stable": freshness.get("passed") is True and freshness.get("source_stable_during_gate") is True,
    }
    return {"passed": contract and all(row["passed"] for row in rows) and all(semantic.values()), "release_evidence_contract_passes": contract, "verified_artifacts": rows, **semantic, "bounded_claim": document["claim"]["supported_scope"], "excluded_scope": document["claim"]["exclusions"]}


def main() -> int:
    checks = evaluate_audit_inputs()
    extras = [SOURCE, *[row["path"] for row in checks["verified_artifacts"] if row.get("path")]]
    report = write_identity_report("method_audit", passed=checks["passed"], checks=checks, extras=extras)
    print(f"wrote {report} | passed={checks['passed']}")
    if not checks["passed"]: print(json.dumps(checks, indent=2, sort_keys=True))
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
