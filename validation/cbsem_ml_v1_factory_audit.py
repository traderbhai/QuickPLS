"""Final method-scoped CB-SEM ML v1 audit over current factory evidence."""

from __future__ import annotations

from typing import Any

from method_promotion_manifest import _verify_artifact
from cbsem_ml_v1_factory_common import ROOT, manifest, strict_load_json, write_identity_report


SOURCE = "validation/cbsem_ml_v1_factory_audit.py"


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
            rows.append(
                {
                    "stage": stage,
                    "roles": artifact["roles"],
                    "path": artifact["path"],
                    "passed": passed,
                    "errors": errors,
                }
            )
    packaged_path = (
        ROOT
        / "validation/results/method_factory/cbsem_ml_v1/packaged_acceptance.identity.json"
    )
    packaged = strict_load_json(packaged_path)
    packaged_checks = packaged.get("checks", {})
    workflow = packaged_checks.get("workflow", {})
    archive = packaged_checks.get("archive", {})
    visual = workflow.get("visual", {})
    source_freshness = packaged_checks.get("source_freshness", {})
    cumulative = workflow.get("ancillary", {})
    checks = {
        "passed": all(row["passed"] for row in rows)
        and packaged.get("passed") is True
        and workflow.get("passed") is True
        and archive.get("passed") is True,
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
        "packaged_report_current": packaged.get("passed") is True,
        "complete_packaged_workflow": workflow.get("passed") is True,
        "same_run_typed_archive": archive.get("passed") is True,
        "source_receipt_and_cli_current": source_freshness.get("passed") is True
        and source_freshness.get("desktop_receipt_exact") is True
        and source_freshness.get("release_cli_newer_sources") == [],
        "exact_required_check_receipt": cumulative.get("exact_required_check_contract") is True,
        "invalid_setup_no_state_mutation": workflow.get("workflow_steps", {}).get("invalid_setup_blocked") is True,
        "actual_packaged_viewport_matrix": visual.get("passed") is True
        and visual.get("actual_tauri_window") is True
        and visual.get("viewport_emulation") is False
        and visual.get("viewports") == ["1024x700", "1280x720", "1440x900"],
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }
    checks["passed"] = checks["passed"] and all(
        checks[key]
        for key in (
            "source_receipt_and_cli_current",
            "exact_required_check_receipt",
            "invalid_setup_no_state_mutation",
            "actual_packaged_viewport_matrix",
        )
    )
    path = write_identity_report(
        "method_audit",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/cbsem_ml_v1_packaged_adapter.py",
            "validation/results/method_factory/cbsem_ml_v1/packaged_acceptance.identity.json",
        ],
    )
    print(f"wrote {path} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
