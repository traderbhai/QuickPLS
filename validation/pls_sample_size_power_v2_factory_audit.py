#!/usr/bin/env python3
"""Final fail-closed release audit for prospective PLS power v2."""

from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

try:
    from validation.method_promotion_manifest import _verify_artifact
    from validation.phase2_release_packaged_common import AdapterError, ROOT, output_path, role_sources, strict_json, write_identity
    from validation.pls_sample_size_power_v2_packaged_acceptance import CONTRACT, MANIFEST
except ModuleNotFoundError:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.method_promotion_manifest import _verify_artifact  # type: ignore[no-redef]
    from validation.phase2_release_packaged_common import AdapterError, ROOT, output_path, role_sources, strict_json, write_identity  # type: ignore[no-redef]
    from validation.pls_sample_size_power_v2_packaged_acceptance import CONTRACT, MANIFEST  # type: ignore[no-redef]


def evaluate() -> dict[str, Any]:
    document = strict_json(MANIFEST)
    identity = {
        "passed": True,
        "feature_id": CONTRACT.feature_id,
        "method_version": CONTRACT.method_version,
        "catalogue_snapshot_date": CONTRACT.catalogue_date,
    }
    prior_rows: list[dict[str, Any]] = []
    for stage in ("engine_only", "archive_qualified", "native_qualified"):
        for artifact in document["qualification"]["evidence"][stage]:
            passed, errors = _verify_artifact(artifact, document, ROOT, identity)
            prior_rows.append({"stage": stage, "path": artifact["path"], "roles": artifact["roles"], "passed": passed, "errors": errors})
    release = document["qualification"]["evidence"]["release_qualified"]
    release_roles = [role for artifact in release for role in artifact.get("roles", [])]
    release_contract = len(release) == 2 and sorted(release_roles) == ["method_audit", "packaged_acceptance"] and all(len(artifact.get("roles", [])) == 1 for artifact in release)
    packaged_rows = [artifact for artifact in release if artifact.get("roles") == ["packaged_acceptance"]]
    if len(packaged_rows) != 1:
        raise AdapterError("prospective-power v2 manifest must contain one packaged acceptance identity")
    packaged_passed, packaged_errors = _verify_artifact(packaged_rows[0], document, ROOT, identity)
    packaged = strict_json(ROOT / packaged_rows[0]["path"])
    checks = packaged.get("checks", {})
    workflow = checks.get("workflow", {})
    archive = checks.get("archive", {})
    tail = workflow.get("tail_accounting", {})
    viewports = workflow.get("packaged_viewports", {})
    method_offline = checks.get("method_functional_offline", {})
    platform_egress = checks.get("platform_background_egress_observation", {})
    platform_egress_observed = platform_egress.get("platform_background_egress_observed")
    commercial_zero_egress_passed = platform_egress.get("commercial_zero_egress_passed")
    remote_connections = platform_egress.get("remote_connections")
    platform_egress_labeling_truthful = (
        platform_egress.get("passed") is True
        and platform_egress.get("observation_kind") == "sampled_exact_process_tree_tcp_v1"
        and isinstance(platform_egress.get("sample_count"), int)
        and platform_egress.get("sample_count", 0) > 0
        and platform_egress.get("root_present_every_sample") is True
        and isinstance(platform_egress_observed, bool)
        and isinstance(commercial_zero_egress_passed, bool)
        and isinstance(remote_connections, list)
        and bool(remote_connections) is platform_egress_observed
        and commercial_zero_egress_passed is (not platform_egress_observed)
        and checks.get("scoped_receipt", {}).get("sampled_process_tree_zero_egress") is commercial_zero_egress_passed
    )
    semantic = {
        "packaged_identity_passes": packaged.get("passed") is True and checks.get("passed") is True,
        "prior_factory_native": checks.get("prior_factory") == "native_qualified",
        "source_build_and_cli_current": checks.get("source_freshness", {}).get("passed") is True and checks.get("source_freshness", {}).get("desktop_receipt_exact") is True and checks.get("source_freshness", {}).get("release_cli_newer_sources") == [],
        "exact_scoped_receipt_and_clean_shutdown": checks.get("scoped_receipt", {}).get("passed") is True and checks.get("scoped_receipt", {}).get("exact_required_checks") is True and checks.get("scoped_receipt", {}).get("cleanup_verified") is True and checks.get("scoped_receipt", {}).get("forced_cleanup_used") is False,
        "invalid_setup_fails_closed": workflow.get("invalid_setup", {}).get("passed") is True and workflow.get("invalid_setup", {}).get("archive_state_unchanged") is True,
        "cancellation_retry_passes": workflow.get("cancellation_retry", {}).get("passed") is True and workflow.get("cancellation_retry", {}).get("same_plan_retried") is True,
        "outer_and_inner_tail_accounting_close": tail == {"outer_rows": 2, "tail_rows": 2, "outer_closed": True, "inner_closed": True},
        "selected_run_xlsx_and_reopen": workflow.get("selected_run_xlsx") is True and workflow.get("same_run_reopened") is True and checks.get("xlsx", {}).get("passed") is True,
        "archive_matches_visible_run": archive.get("passed") is True and archive.get("run_id") == workflow.get("run_id") and archive.get("method_version") == CONTRACT.method_version and archive.get("grid") == [30, 40] and archive.get("outer_requested") == 200 and archive.get("outer_attempted") == 200 and archive.get("outer_usable", -1) + archive.get("outer_failed", -1) == 200 and archive.get("inner_usable", -1) + archive.get("inner_failed", -1) == archive.get("inner_requested"),
        "three_actual_packaged_viewports": viewports.get("passed") is True and viewports.get("actual_tauri_window") is True and viewports.get("viewport_emulation") is False and viewports.get("viewports") == ["1024x700", "1280x720", "1440x900"],
        "method_functional_offline": workflow.get("functional_offline") is True
        and method_offline.get("passed") is True
        and method_offline.get("externalRequestCount") == 0
        and method_offline.get("externalRequests") == []
        and method_offline.get("analyticalWorkflowRequiresInternet") is False
        and method_offline.get("strictZeroProcessEgressClaimed") is False,
        "platform_background_egress_recorded_truthfully": platform_egress_labeling_truthful,
    }
    return {
        "passed": release_contract and all(row["passed"] for row in prior_rows) and packaged_passed and not packaged_errors and all(semantic.values()),
        "release_evidence_contract_passes": release_contract,
        "verified_prior_artifacts": prior_rows,
        "packaged_artifact": {"path": packaged_rows[0]["path"], "passed": packaged_passed, "errors": packaged_errors},
        **semantic,
        "commercial_zero_egress_passed": commercial_zero_egress_passed,
        "bounded_claim": document["claim"]["supported_scope"],
        "excluded_scope": document["claim"]["exclusions"],
    }


def main() -> int:
    try:
        checks = evaluate()
        if not checks["passed"]:
            raise AdapterError(f"prospective-power v2 final audit failed: {checks}")
        packaged = output_path(CONTRACT, "packaged_acceptance")
        sources = role_sources(CONTRACT, "method_audit", [Path(__file__).resolve(), ROOT / CONTRACT.adapter_script, packaged])
        target = write_identity(CONTRACT, "method_audit", {"final_release_audit": checks}, sources)
    except (AdapterError, OSError, KeyError, TypeError, ValueError) as error:
        print(json.dumps({"passed": False, "method": CONTRACT.slug, "error": f"{type(error).__name__}: {error}"}, indent=2))
        return 1
    print(json.dumps({"passed": True, "method_audit_identity": str(target)}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
