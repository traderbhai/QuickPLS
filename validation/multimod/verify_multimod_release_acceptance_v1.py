#!/usr/bin/env python3
"""Fail-closed final acceptance check for an unmerged MultiMod campaign."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from materialize_multimod_live_manifests_v1 import (
    assert_runtime_promotion_mechanism,
    verify as verify_manifest_set,
)


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MAIN = Path(r"D:\QuickPLS")
FROZEN_MAIN = "6ed46cc422917fc9fc9c463302ca4ff1e9ea01a4"
PLAN = HERE / "v256_multimod_qualification_plan_v1.json"
CATALOG = HERE / "multimod_gate_bindings_v1.json"
INDEX = HERE / "multimod_capability_index_v1.json"
RUNTIME_PROMOTION_FAMILIES = {
    "qpls.multimod.mga_multigroup_v1",
    "qpls.multimod.pls_heterogeneity_v2",
    "qpls.multimod.general_sem_conditional_process_v2",
    "qpls.multimod.interventional_causal_mediation_v1",
}


def read_json(path: Path) -> Any:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git(root: Path, *arguments: str) -> str:
    result = subprocess.run(
        ["git", "-C", str(root), *arguments],
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def free_gib(root: str) -> float:
    return round(shutil.disk_usage(root).free / (1024**3), 2)


def validate_runtime_promotion_receipt(
    campaign_root: Path,
    lane: str,
    candidate_commit: str,
    candidate_version: str,
    plan_sha256: str,
    authority_document_sha256: str,
    authority_binding_sha256: str,
    prepackage_manifest_set_sha256: str,
) -> list[str]:
    """Validate a hash-bound installed/portable qualified-runtime smoke."""
    errors: list[str] = []
    gate_id = f"{lane}.offline.smoke"
    gate_receipt_path = campaign_root / gate_id / "gate_receipt.json"
    if not gate_receipt_path.is_file():
        return [f"{lane} runtime-promotion gate receipt is missing"]
    gate_receipt = read_json(gate_receipt_path)
    if (
        gate_receipt.get("receipt_kind") != "qpls_multimod_gate_receipt_v1"
        or gate_receipt.get("gate_id") != gate_id
        or gate_receipt.get("status") != "passed"
        or gate_receipt.get("coverage_binding_state") != "executed_real_commands"
        or gate_receipt.get("candidate_commit_sha") != candidate_commit
        or gate_receipt.get("candidate_version") != candidate_version
        or gate_receipt.get("plan_sha256") != plan_sha256
        or gate_receipt.get("binding_sha256") != sha256(CATALOG)
    ):
        return [f"{lane} runtime-promotion gate receipt is stale or invalid"]

    bound_outputs: list[dict[str, Any]] = []
    for step in gate_receipt.get("steps", []):
        bound_outputs.extend(step.get("expected_outputs", []))
    candidates = [
        row
        for row in bound_outputs
        if Path(str(row.get("path", ""))).name == "runtime-promotion-receipt.json"
    ]
    if len(candidates) != 1:
        return [f"{lane} gate must hash exactly one runtime-promotion-receipt.json output"]
    row = candidates[0]
    receipt_path = Path(str(row.get("path", ""))).resolve()
    try:
        receipt_path.relative_to(campaign_root)
    except ValueError:
        return [f"{lane} runtime-promotion receipt escapes the campaign root"]
    if (
        not receipt_path.is_file()
        or sha256(receipt_path) != row.get("sha256")
        or receipt_path.stat().st_size != row.get("size")
    ):
        return [f"{lane} runtime-promotion receipt is missing or hash-stale"]

    receipt = read_json(receipt_path)
    exact = {
        "schema_version": 1,
        "receipt_kind": "qpls_multimod_runtime_promotion_smoke_v1",
        "lane": lane,
        "candidate_commit_sha": candidate_commit,
        "candidate_version": candidate_version,
        "plan_sha256": plan_sha256,
        "binding_sha256": sha256(CATALOG),
        "authority_document_sha256": authority_document_sha256,
        "authority_binding_sha256": authority_binding_sha256,
        "manifest_set_sha256": prepackage_manifest_set_sha256,
        "offline": True,
        "console_window_absent": True,
        "qualification_state": "release_qualified_candidate",
        "unqualified_authority_fails_closed": True,
        "post_evidence_source_change_required": False,
        "cancellation_recovery_verified": True,
    }
    for key, expected in exact.items():
        if receipt.get(key) != expected:
            errors.append(f"{lane} runtime-promotion receipt has invalid {key}")
    if (
        receipt.get("qualification_coverage_complete") is not True
        or receipt.get("qualification_harness_contract")
        != "qpls.v256.multimod.build-only-packaged-qualification-harness.v1"
        or receipt.get("unmerged_review_candidate") is not True
        or receipt.get("later_harness_disabled_rebuild_covered") is not False
    ):
        errors.append(f"{lane} runtime-promotion receipt lacks the exact review-candidate harness boundary")
    package_path = campaign_root / "packages" / "package-artifacts.json"
    package_digest = sha256(package_path) if package_path.is_file() else None
    package_document = read_json(package_path) if package_path.is_file() else {}
    if receipt.get("package_receipt_sha256") != package_digest:
        errors.append(f"{lane} runtime-promotion receipt is not bound to the exact package receipt")
    harness = package_document.get("qualification_harness", {})
    if (
        harness.get("contract")
        != "qpls.v256.multimod.build-only-packaged-qualification-harness.v1"
        or harness.get("cargo_feature") != "multimod-qualification-harness"
        or harness.get("compile_time_only") is not True
        or harness.get("embedded_candidate_authority_required") is not True
        or harness.get("request_or_runtime_environment_authority_forbidden") is not True
        or harness.get("executable_specific_smoke_required") is not True
        or harness.get("later_harness_disabled_rebuild_not_covered") is not True
        or harness.get("unmerged_review_candidate") is not True
    ):
        errors.append(f"{lane} package receipt lacks the exact build-only harness boundary")
    evidence_value = receipt.get("production_workflow_evidence_path")
    evidence_path = Path(str(evidence_value or "")).resolve()
    evidence: dict[str, Any] = {}
    try:
        evidence_path.relative_to(campaign_root)
    except ValueError:
        errors.append(f"{lane} production-workflow evidence escapes the campaign root")
    else:
        if (
            not evidence_path.is_file()
            or sha256(evidence_path) != receipt.get("production_workflow_evidence_sha256")
        ):
            errors.append(f"{lane} production-workflow evidence is missing or hash-stale")
        else:
            evidence = read_json(evidence_path)
    if evidence and (
        evidence.get("report_id")
        != "qpls.v256.multimod.packaged-offline-production-smoke.v1"
        or evidence.get("passed") is not True
        or evidence.get("qualification_coverage_complete") is not True
        or evidence.get("package_kind") != lane
        or evidence.get("candidate_commit_sha") != candidate_commit
        or evidence.get("candidate_version") != candidate_version
        or evidence.get("plan_sha256") != plan_sha256
        or evidence.get("binding_sha256") != sha256(CATALOG)
        or evidence.get("authority_document_sha256") != authority_document_sha256
        or evidence.get("authority_binding_sha256") != authority_binding_sha256
        or evidence.get("manifest_set_sha256") != prepackage_manifest_set_sha256
        or evidence.get("package_receipt_sha256") != package_digest
        or evidence.get("candidate_receipt_tamper_failed_closed") is not True
        or evidence.get("cancellation_recovery_verified") is not True
        or evidence.get("offline", {}).get("passed") is not True
        or evidence.get("offline", {}).get("functional_network_requests") != []
        or evidence.get("offline", {}).get("remote_resource_urls") != []
        or evidence.get("console_errors") != []
        or evidence.get("accessibility", {}).get("unnamed_control_count") != 0
        or evidence.get("keyboard_focus_reached") is not True
        or evidence.get("harness", {}).get("compile_time_feature_required") is not True
        or evidence.get("harness", {}).get("embedded_candidate_authority_required") is not True
        or evidence.get("harness", {}).get("runtime_or_request_authority_injection") is not False
        or evidence.get("harness", {}).get("unmerged_review_candidate") is not True
        or evidence.get("harness", {}).get("later_harness_disabled_rebuild_covered") is not False
    ):
        errors.append(f"{lane} production-workflow evidence identity, offline boundary, accessibility, or harness contract is invalid")
    if evidence:
        candidate = evidence.get("candidate", {})
        if (
            candidate.get("path") != receipt.get("executable_path")
            or candidate.get("sha256") != receipt.get("executable_sha256")
        ):
            errors.append(f"{lane} runtime receipt and production evidence identify different executable bytes")
        evidence_families = evidence.get("families", [])
        if len(evidence_families) != 4 or {
            row.get("family_id") for row in evidence_families if isinstance(row, dict)
        } != RUNTIME_PROMOTION_FAMILIES:
            errors.append(f"{lane} production evidence family matrix is incomplete or duplicated")
        for family in evidence_families:
            if not isinstance(family, dict):
                continue
            base_formats = {
                row.get("format")
                for row in family.get("exports", [])
                if isinstance(row, dict)
                and row.get("exactSemanticReadback") is True
                and row.get("digestReadback") is True
                and row.get("renderedSurfaceReadback") is True
            }
            integrity = family.get("integrity_variants", {})
            raw = family.get("raw_sidecar_export", {})
            if (
                not {"csv", "xlsx", "json", "html", "pdf"}.issubset(base_formats)
                or raw.get("strictReopenValidated") is not True
                or raw.get("noReplacePublication") is not True
                or integrity.get("productionStrictReopenRejectedMissing") is not True
                or integrity.get("productionStrictReopenRejectedTamper") is not True
                or family.get("authority_receipt_verified") is not True
            ):
                errors.append(f"{lane} {family.get('family_id')} lacks semantic/raw/integrity production evidence")
    if lane == "installed":
        if receipt.get("isolated_nsis_install_verified") is not True:
            errors.append("installed runtime-promotion receipt lacks isolated NSIS evidence")
        if receipt.get("uninstall_cleanup_verified") is not True:
            errors.append("installed runtime-promotion receipt lacks uninstall cleanup evidence")
        if evidence and (
            evidence.get("isolated_install", {}).get("installed_executable")
            != receipt.get("executable_path")
            or evidence.get("isolated_install", {}).get("installed_executable_sha256")
            != receipt.get("executable_sha256")
            or evidence.get("isolated_install", {})
            .get("installed_portable_equivalence", {})
            .get("passed")
            is not True
            or evidence.get("process_cleanup", {}).get("install_root_removed") is not True
            or evidence.get("process_cleanup", {}).get("registration_removed") is not True
        ):
            errors.append("installed runtime-promotion evidence lacks exact NSIS equivalence or cleanup")
    elif receipt.get("isolated_nsis_install_verified") is not False or receipt.get(
        "uninstall_cleanup_verified"
    ) is not False:
        errors.append("portable runtime-promotion receipt must not claim an NSIS install or uninstall")
    if lane == "portable":
        portable_rows = [
            row for row in package_document.get("artifacts", []) if row.get("role") == "portable"
        ]
        if len(portable_rows) != 1 or (
            str(Path(str(portable_rows[0].get("path", ""))).resolve())
            != str(Path(str(receipt.get("executable_path", ""))).resolve())
            or portable_rows[0].get("sha256") != receipt.get("executable_sha256")
        ):
            errors.append("portable runtime-promotion receipt differs from the exact package artifact")
    families = receipt.get("families", [])
    observed_families = {
        row.get("family_id") for row in families if isinstance(row, dict)
    }
    if len(families) != 4 or observed_families != RUNTIME_PROMOTION_FAMILIES:
        errors.append(f"{lane} runtime-promotion family matrix is incomplete or duplicated")
    for family in families:
        if not isinstance(family, dict):
            errors.append(f"{lane} runtime-promotion family row is invalid")
            continue
        for field in (
            "valid_run_completed",
            "result_identity_verified",
            "archive_reopened",
            "semantic_export_readback",
            "sidecar_integrity_verified",
        ):
            if family.get(field) is not True:
                errors.append(
                    f"{lane} {family.get('family_id')} runtime-promotion receipt lacks {field}"
                )
    if not isinstance(receipt.get("executable_path"), str) or not receipt["executable_path"]:
        errors.append(f"{lane} runtime-promotion receipt lacks executable_path")
    if not isinstance(receipt.get("executable_sha256"), str) or len(receipt["executable_sha256"]) != 64:
        errors.append(f"{lane} runtime-promotion receipt lacks executable_sha256")
    return errors


def audit(campaign_root: Path, candidate_commit: str, plan_sha256: str) -> dict[str, Any]:
    errors: list[str] = []
    runtime_promotion_mechanism_verified = False
    campaign_root = campaign_root.resolve()
    plan = read_json(PLAN)
    if sha256(PLAN) != plan_sha256:
        errors.append("qualification plan digest is stale")
    if git(ROOT, "rev-parse", "HEAD") != candidate_commit:
        errors.append("candidate commit differs from current HEAD")
    if git(ROOT, "branch", "--show-current") != plan["candidate"]["branch"]:
        errors.append("candidate branch differs from the frozen branch")
    if git(ROOT, "status", "--porcelain=v1", "--untracked-files=all"):
        errors.append("candidate worktree is dirty")
    if not MAIN.is_dir() or git(MAIN, "rev-parse", "HEAD") != FROZEN_MAIN:
        errors.append("main repository moved from the frozen base")
    elif git(MAIN, "status", "--porcelain=v1", "--untracked-files=all"):
        errors.append("main repository is dirty")
    try:
        if git(ROOT, "merge-base", candidate_commit, FROZEN_MAIN) != FROZEN_MAIN or candidate_commit == FROZEN_MAIN:
            errors.append("candidate is not an unmerged descendant of the frozen main commit")
    except subprocess.CalledProcessError:
        errors.append("candidate/main merge-base could not be established")
    if git(ROOT, "tag", "--points-at", candidate_commit):
        errors.append("candidate commit is tagged")
    try:
        remote_contains = git(ROOT, "branch", "-r", "--contains", candidate_commit)
    except subprocess.CalledProcessError:
        remote_contains = ""
    if remote_contains:
        errors.append(f"candidate commit already exists on a remote branch: {remote_contains}")

    state_path = campaign_root / "campaign_state.json"
    issue_path = campaign_root / "issue_inventory.json"
    prepackage_manifest_set_path = campaign_root / "prepackage-authority" / "manifest-set.json"
    prepackage_authority_path = campaign_root / "prepackage-authority" / "candidate-authority.json"
    manifest_set_path = campaign_root / "live-manifests" / "manifest-set.json"
    package_path = campaign_root / "packages" / "package-artifacts.json"
    for required in (
        state_path,
        issue_path,
        prepackage_manifest_set_path,
        prepackage_authority_path,
        manifest_set_path,
        package_path,
    ):
        if not required.is_file():
            errors.append(f"required acceptance artifact is missing: {required}")
    if not errors or state_path.is_file():
        state = read_json(state_path)
        if (
            state.get("candidate_commit_sha") != candidate_commit
            or state.get("candidate_version") != plan["candidate"]["final_version"]
            or state.get("plan_sha256") != plan_sha256
            or state.get("binding_sha256") != sha256(CATALOG)
        ):
            errors.append("campaign state binding is stale")
        expected_prior = [gate["gate_id"] for gate in plan["gates"] if gate["gate_id"] != "release.acceptance"]
        observed = {
            gate.get("gate_id"): (gate.get("status"), gate.get("evidence_valid"))
            for gate in state.get("gates", [])
        }
        for gate_id in expected_prior:
            if observed.get(gate_id) != ("passed", True):
                errors.append(f"required prior gate did not pass with valid evidence: {gate_id}")
    if issue_path.is_file():
        inventory = read_json(issue_path)
        if (
            inventory.get("candidate_commit_sha") != candidate_commit
            or inventory.get("candidate_version") != plan["candidate"]["final_version"]
            or inventory.get("plan_sha256") != plan_sha256
            or inventory.get("binding_sha256") != sha256(CATALOG)
        ):
            errors.append("issue inventory binding is stale")
        if inventory.get("issues"):
            errors.append("issue inventory is not empty")
    if package_path.is_file():
        package = read_json(package_path)
        if (
            package.get("candidate_commit_sha") != candidate_commit
            or package.get("version") != plan["candidate"]["final_version"]
            or package.get("isolated_candidate") is not True
            or package.get("public_artifacts_replaced") is not False
            or package.get("pushed") is not False
            or package.get("tagged") is not False
            or package.get("published") is not False
            or len(str(package.get("authority_binding_sha256", ""))) != 64
            or any(
                character not in "0123456789abcdef"
                for character in str(package.get("authority_binding_sha256", ""))
            )
        ):
            errors.append("candidate package receipt overstates or mismatches release isolation")
        if prepackage_manifest_set_path.is_file() and (
            package.get("prepackage_manifest_set_sha256")
            != sha256(prepackage_manifest_set_path)
        ):
            errors.append("candidate package receipt differs from the prepackage manifest set")
        if prepackage_authority_path.is_file() and (
            package.get("prepackage_authority_sha256")
            != sha256(prepackage_authority_path)
        ):
            errors.append("candidate package receipt differs from the prepackage authority")
        for artifact in package.get("artifacts", []):
            path = Path(artifact.get("path", "")).resolve()
            try:
                path.relative_to(campaign_root)
            except ValueError:
                errors.append(f"candidate artifact escapes campaign evidence root: {path}")
                continue
            if not path.is_file() or sha256(path) != artifact.get("sha256") or path.stat().st_size != artifact.get("size"):
                errors.append(f"candidate artifact is missing or stale: {artifact.get('role')}")
    prepackage_manifest_set_sha256 = ""
    if prepackage_manifest_set_path.is_file() and prepackage_authority_path.is_file():
        prepackage_manifest_set_sha256 = sha256(prepackage_manifest_set_path)
        try:
            verify_manifest_set(
                campaign_root,
                candidate_commit,
                plan_sha256,
                prepackage_manifest_set_path,
                "prepackage_authority",
                prepackage_authority_path,
            )
        except Exception as error:
            errors.append(
                f"prepackage authority verification failed: {type(error).__name__}:{error}"
            )
    if manifest_set_path.is_file():
        try:
            verify_manifest_set(
                campaign_root,
                candidate_commit,
                plan_sha256,
                manifest_set_path,
                "final_live",
            )
        except Exception as error:
            errors.append(f"live manifest verification failed: {type(error).__name__}:{error}")
    try:
        assert_runtime_promotion_mechanism()
        runtime_promotion_mechanism_verified = True
    except Exception as error:
        errors.append(f"runtime promotion mechanism is unavailable: {type(error).__name__}:{error}")
    package_authority_document_sha256 = ""
    package_authority_binding_sha256 = ""
    if package_path.is_file():
        package_document = read_json(package_path)
        package_authority_document_sha256 = str(
            package_document.get("prepackage_authority_sha256", "")
        )
        package_authority_binding_sha256 = str(
            package_document.get("authority_binding_sha256", "")
        )
    if (
        prepackage_manifest_set_sha256
        and len(package_authority_document_sha256) == 64
        and len(package_authority_binding_sha256) == 64
    ):
        for lane in ("installed", "portable"):
            errors.extend(
                validate_runtime_promotion_receipt(
                    campaign_root,
                    lane,
                    candidate_commit,
                    str(plan["candidate"]["final_version"]),
                    plan_sha256,
                    package_authority_document_sha256,
                    package_authority_binding_sha256,
                    prepackage_manifest_set_sha256,
                )
            )

    index = read_json(INDEX)
    policy = index.get("policy", {})
    if (
        policy.get("surface") != "labs"
        or policy.get("evidence_state") != "absent"
        or policy.get("promotion_allowed") is not False
    ):
        errors.append("tracked capability index exceeds its Labs/absent policy")
    for template_relative in plan.get("manifests", []):
        template = read_json(ROOT / template_relative)
        if (
            template.get("surface") != "labs"
            or template.get("declared_evidence_state") != "absent"
            or template.get("promotion_allowed") is not False
            or template.get("source_binding", {}).get("status") != "pending"
        ):
            errors.append(
                f"tracked capability manifest exceeds Labs/absent policy: {template_relative}"
            )
    free = {"C": free_gib("C:\\"), "D": free_gib("D:\\")}
    if free["C"] < float(plan["disk_thresholds_gib"]["C"]):
        errors.append("C drive is below its retained-evidence threshold")
    if free["D"] < float(plan["disk_thresholds_gib"]["D"]):
        errors.append("D drive is below its retained-evidence threshold")

    return {
        "schema_version": 1,
        "report_id": "qpls.v256.multimod.release-acceptance.v1",
        "passed": not errors,
        "candidate_commit_sha": candidate_commit,
        "main_commit_sha": FROZEN_MAIN,
        "plan_sha256": plan_sha256,
        "candidate_unmerged": not any("unmerged descendant" in error for error in errors),
        "candidate_untagged": not any("tagged" in error for error in errors),
        "candidate_unpushed": not any("remote branch" in error for error in errors),
        "candidate_unpublished": not any("package receipt" in error for error in errors),
        "runtime_promotion_mechanism_verified": runtime_promotion_mechanism_verified,
        "free_gib": free,
        "errors": errors,
        "generated_at_utc": datetime.now(timezone.utc).isoformat(),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--campaign-root", type=Path, required=True)
    parser.add_argument("--candidate-commit", required=True)
    parser.add_argument("--plan-sha256", required=True)
    parser.add_argument("--output", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        report = audit(arguments.campaign_root, arguments.candidate_commit, arguments.plan_sha256)
    except Exception as error:
        report = {
            "schema_version": 1,
            "report_id": "qpls.v256.multimod.release-acceptance.v1",
            "passed": False,
            "candidate_commit_sha": arguments.candidate_commit,
            "plan_sha256": arguments.plan_sha256,
            "errors": [f"harness_error:{type(error).__name__}:{error}"],
            "generated_at_utc": datetime.now(timezone.utc).isoformat(),
        }
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_bytes((json.dumps(report, indent=2, sort_keys=True) + "\n").encode("utf-8"))
    print(json.dumps(report, sort_keys=True))
    return 0 if report.get("passed") is True else 1


if __name__ == "__main__":
    raise SystemExit(main())
