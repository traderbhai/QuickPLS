"""Final method-scoped PCA v1 audit over current factory evidence."""

from __future__ import annotations

from typing import Any

from method_promotion_manifest import _verify_artifact
from pca_v1_factory_common import ROOT, manifest, strict_load_json, write_identity_report


SOURCE = "validation/pca_v1_factory_audit.py"


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
    packaged_artifact = next(
        artifact
        for artifact in document["qualification"]["evidence"]["release_qualified"]
        if artifact["roles"] == ["packaged_acceptance"]
    )
    packaged_passed, packaged_errors = _verify_artifact(
        packaged_artifact,
        document,
        ROOT,
        expected_identity,
    )
    packaged = strict_load_json(ROOT / packaged_artifact["path"])
    packaged_checks = packaged.get("checks", {})
    rows.append(
        {
            "stage": "release_qualified",
            "roles": ["packaged_acceptance"],
            "path": packaged_artifact["path"],
            "passed": packaged_passed,
            "errors": packaged_errors,
        }
    )
    exact = packaged_checks.get("exact_values", {})
    mutations = packaged_checks.get("fail_closed_mutations", {})
    checks = {
        "passed": (
            all(row["passed"] for row in rows)
            and exact.get("passed") is True
            and mutations.get("passed") is True
            and packaged_checks.get("focused_scope", {}).get("scope") == "pca"
        ),
        "stage_artifacts": rows,
        "scientific_and_boundary_reports_current": all(
            row["passed"] for row in rows if row["stage"] == "engine_only"
        ),
        "archive_report_current": all(
            row["passed"] for row in rows if row["stage"] == "archive_qualified"
        ),
        "frontend_and_export_reports_current": all(
            row["passed"] for row in rows if row["stage"] == "native_qualified"
        ),
        "packaged_report_current": packaged_passed,
        "same_run_archive_ui_xlsx_values_exact": exact.get("passed") is True,
        "all_six_archive_mutations_fail_closed": mutations.get("passed") is True,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }
    report = write_identity_report(
        "method_audit",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/results/method_factory/pca_v1/packaged_acceptance.identity.json",
        ],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
