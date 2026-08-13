"""Final method-scoped CTA-PLS v1 audit over current factory evidence."""

from __future__ import annotations

from typing import Any

from cta_pls_v1_factory_common import ROOT, manifest, strict_load_json, write_identity_report
from method_promotion_manifest import _verify_artifact


SOURCE = "validation/cta_pls_v1_factory_audit.py"


def main() -> int:
    document = manifest()
    feature = document["feature"]
    expected_identity = {
        "passed": True,
        "feature_id": feature["id"],
        "method_version": feature["method_version"],
        "catalogue_snapshot_date": feature["catalogue_snapshot_date"],
    }
    rows: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, expected_identity)
            rows.append({"stage": stage, "roles": artifact["roles"], "path": artifact["path"], "passed": passed, "errors": errors})
    packaged_artifact = next(
        artifact
        for artifact in document["qualification"]["evidence"]["release_qualified"]
        if artifact["roles"] == ["packaged_acceptance"]
    )
    packaged_passed, packaged_errors = _verify_artifact(packaged_artifact, document, ROOT, expected_identity)
    packaged = strict_load_json(ROOT / packaged_artifact["path"])
    packaged_checks = packaged.get("checks", {})
    method_functional_offline = packaged_checks.get("method_functional_offline", {})
    platform_egress = packaged_checks.get("platform_background_egress_observation", {})
    platform_egress_observed = platform_egress.get("platform_background_egress_observed")
    commercial_zero_egress_passed = platform_egress.get("commercial_zero_egress_passed")
    remote_connections = platform_egress.get("remote_connections")
    platform_egress_labeling_truthful = (
        isinstance(platform_egress_observed, bool)
        and isinstance(commercial_zero_egress_passed, bool)
        and isinstance(remote_connections, list)
        and bool(remote_connections) is platform_egress_observed
        and commercial_zero_egress_passed is (not platform_egress_observed)
    )
    rows.append({
        "stage": "release_qualified",
        "roles": ["packaged_acceptance"],
        "path": packaged_artifact["path"],
        "passed": packaged_passed,
        "errors": packaged_errors,
    })
    checks = {
        "passed": all(row["passed"] for row in rows)
        and packaged_checks.get("passed") is True
        and packaged_checks.get("same_run", {}).get("passed") is True
        and packaged_checks.get("exact_archive", {}).get("passed") is True
        and packaged_checks.get("exact_xlsx", {}).get("passed") is True
        and packaged_checks.get("responsive_viewports", {}).get("passed") is True
        and method_functional_offline.get("passed") is True
        and method_functional_offline.get("runtime_network_dependency") is False
        and platform_egress.get("passed") is True
        and platform_egress_labeling_truthful
        and packaged_checks.get("cleanup", {}).get("passed") is True
        and packaged_checks.get("fail_closed_mutations", {}).get("passed") is True,
        "stage_artifacts": rows,
        "packaged_report_current": packaged_passed,
        "same_run_archive_ui_xlsx_exact": packaged_checks.get("same_run", {}).get("passed") is True
        and packaged_checks.get("exact_archive", {}).get("passed") is True
        and packaged_checks.get("exact_xlsx", {}).get("passed") is True,
        "three_required_viewports_pass": packaged_checks.get("responsive_viewports", {}).get("passed") is True,
        "method_functional_offline_and_cleanup_pass": method_functional_offline.get("passed") is True
        and method_functional_offline.get("runtime_network_dependency") is False
        and packaged_checks.get("cleanup", {}).get("passed") is True,
        "platform_background_egress_recorded_truthfully": platform_egress.get("passed") is True
        and platform_egress_labeling_truthful,
        "commercial_zero_egress_passed": commercial_zero_egress_passed,
        "all_six_archive_mutations_fail_closed": packaged_checks.get("fail_closed_mutations", {}).get("passed") is True,
        "release_scope_remains_descriptive_only": True,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }
    report = write_identity_report(
        "method_audit",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE, packaged_artifact["path"]],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
