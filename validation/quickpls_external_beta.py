"""Fail-closed QuickPLS 3 external-beta program validator.

The empty planned contract is valid but never beta-ready. Real beta graduation
requires privacy-safe participant/workflow evidence, defect and discrepancy
closure, and a final signed-candidate lifecycle rerun after all beta activity.
"""

from __future__ import annotations

import argparse
import json
import math
import re
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any

try:
    from validation.quickpls_3_release_readiness import (
        SIGNING_IDENTITY_RECORD,
        ContractError,
        _validate_artifact_descriptor,
        _validate_candidate_manifest,
        _validate_signing_identity_policy,
    )
except ModuleNotFoundError:
    import sys
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation.quickpls_3_release_readiness import (  # type: ignore[no-redef]
        SIGNING_IDENTITY_RECORD,
        ContractError,
        _validate_artifact_descriptor,
        _validate_candidate_manifest,
        _validate_signing_identity_policy,
    )


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CONTRACT = Path(__file__).with_name("quickpls_external_beta.json")
ID = re.compile(r"^[a-z][a-z0-9_-]{2,63}$")
RECORD_ID = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._:/-]{11,127}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
SEMVER_BETA = re.compile(r"^[0-9]+\.[0-9]+\.[0-9]+-beta(?:\.[0-9]+)?$")
MAX_FUTURE_SKEW = timedelta(minutes=5)
EXPECTED_JOURNEYS = ["import_data", "author_model", "calculate", "interpret", "export", "save_reopen"]
EXPECTED_LIFECYCLE_PHASES = [
    "clean_offline_install",
    "portable_launch",
    "same_version_reinstall",
    "supported_upgrade",
    "interrupted_update_recovery",
    "signature_failure_rejected",
    "rollback",
    "offline_full_installer_recovery",
    "uninstall_cleanup",
    "archive_save_reopen",
]
SIGNING_POLICY = {
    "record": SIGNING_IDENTITY_RECORD,
    "candidate_binding": "exact_record_sha256_and_identity_id",
    "leaf_verification": "windows_authenticode_subject_and_sha1_thumbprint",
    "caller_supplied_patterns": "prohibited",
}
EXPECTED_CLAIM_POLICY = {
    "allowed_positioning": "closed_signed_beta_testing_only",
    "functional_offline_statement": "analytical_workflows_require_no_internet_account_or_cloud",
    "application_network_statement": "quickpls_application_and_page_make_no_external_requests",
    "strict_process_tree_claim": "prohibited_pending_os_enforced_fixed_webview2_containment",
}


class BetaContractError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise BetaContractError(message)


def strict_json(path: Path) -> Any:
    def pairs(rows: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in rows:
            if key in result:
                raise BetaContractError(f"duplicate JSON key: {key}")
            result[key] = value
        return result

    def finite(value: str) -> float:
        parsed = float(value)
        if not math.isfinite(parsed):
            raise BetaContractError("non-finite JSON number")
        return parsed

    try:
        return json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(BetaContractError(f"non-finite JSON token: {value}")),
            parse_float=finite,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise BetaContractError(f"cannot load beta contract {path}: {error}") from error


def nonempty(value: object, label: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{label} must be a non-empty string")
    return value.strip()


def timestamp(value: object, label: str, now: datetime) -> datetime:
    text = nonempty(value, label)
    try:
        parsed = datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError as error:
        raise BetaContractError(f"{label} must be ISO-8601") from error
    require(parsed.tzinfo is not None, f"{label} must include a timezone")
    require(parsed.astimezone(timezone.utc) <= now.astimezone(timezone.utc) + MAX_FUTURE_SKEW, f"{label} is in the future")
    return parsed


def exact_keys(value: object, keys: set[str], label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label} must be an object")
    require(set(value) == keys, f"{label} keys do not match the frozen contract")
    return value


def validate_contract(
    contract: object,
    *,
    now: datetime | None = None,
    repository_root: Path = ROOT,
) -> dict[str, Any]:
    now = now or datetime.now(timezone.utc)
    root = exact_keys(contract, {
        "schema_version", "program_id", "target_release", "target_channel", "status",
        "claim_policy", "privacy_contract", "cohort_contract", "core_journeys", "exit_thresholds", "evidence", "decision",
    }, "contract")
    require(root["schema_version"] == 1, "schema_version must be 1")
    require(root["program_id"] == "quickpls_3_external_beta_v1", "program_id is not frozen")
    release = nonempty(root["target_release"], "target_release")
    require(bool(SEMVER_BETA.fullmatch(release)), "target_release must be a 3.0.0-style beta version")
    require(root["target_channel"] == "beta", "target_channel must be beta")
    require(root["status"] in {"planned", "running", "completed"}, "status is invalid")

    claims = exact_keys(root["claim_policy"], set(EXPECTED_CLAIM_POLICY), "claim_policy")
    require(
        claims == EXPECTED_CLAIM_POLICY,
        "claim_policy must preserve functional-offline facts and block strict process-tree claims",
    )

    privacy = exact_keys(root["privacy_contract"], {
        "collect_names", "collect_emails", "collect_raw_datasets", "participant_ids_are_pseudonymous",
        "institution_ids_are_pseudonymous", "consent_record_required", "deletion_request_route",
    }, "privacy_contract")
    for key in ("collect_names", "collect_emails", "collect_raw_datasets"):
        require(privacy[key] is False, f"privacy_contract.{key} must remain false")
    for key in ("participant_ids_are_pseudonymous", "institution_ids_are_pseudonymous", "consent_record_required"):
        require(privacy[key] is True, f"privacy_contract.{key} must remain true")
    nonempty(privacy["deletion_request_route"], "privacy_contract.deletion_request_route")

    cohort = exact_keys(root["cohort_contract"], {
        "minimum_participants", "maximum_participants", "minimum_institutions",
        "minimum_experienced_smartpls_users", "minimum_new_sem_users", "minimum_real_workflows",
    }, "cohort_contract")
    expected_cohort = {
        "minimum_participants": 15, "maximum_participants": 25, "minimum_institutions": 5,
        "minimum_experienced_smartpls_users": 5, "minimum_new_sem_users": 5, "minimum_real_workflows": 30,
    }
    require(cohort == expected_cohort, "cohort thresholds do not match the frozen program")

    journeys = root["core_journeys"]
    require(isinstance(journeys, list) and len(journeys) == len(EXPECTED_JOURNEYS), "core_journeys are incomplete")
    journey_ids: list[str] = []
    for index, row in enumerate(journeys):
        item = exact_keys(row, {"id", "label", "required"}, f"core_journeys[{index}]")
        journey_id = nonempty(item["id"], f"core_journeys[{index}].id")
        require(bool(ID.fullmatch(journey_id)), f"core_journeys[{index}].id is invalid")
        nonempty(item["label"], f"core_journeys[{index}].label")
        require(item["required"] is True, f"core_journeys[{index}] must be required")
        journey_ids.append(journey_id)
    require(journey_ids == EXPECTED_JOURNEYS, "core_journeys order or identity drifted")

    thresholds = exact_keys(root["exit_thresholds"], {
        "minimum_unassisted_journey_completion_rate", "maximum_unresolved_p0", "maximum_unresolved_p1",
        "maximum_reproducible_data_loss", "maximum_archive_corruption",
        "material_scientific_discrepancies_must_be_disposed", "final_signed_lifecycle_rerun_required",
    }, "exit_thresholds")
    require(thresholds == {
        "minimum_unassisted_journey_completion_rate": 0.9,
        "maximum_unresolved_p0": 0, "maximum_unresolved_p1": 0,
        "maximum_reproducible_data_loss": 0, "maximum_archive_corruption": 0,
        "material_scientific_discrepancies_must_be_disposed": True,
        "final_signed_lifecycle_rerun_required": True,
    }, "exit thresholds do not match the frozen program")

    evidence = exact_keys(root["evidence"], {
        "participants", "workflows", "defects", "scientific_discrepancies", "final_candidate", "final_lifecycle_rerun",
    }, "evidence")
    participants = evidence["participants"]
    workflows = evidence["workflows"]
    defects = evidence["defects"]
    discrepancies = evidence["scientific_discrepancies"]
    for value, label in ((participants, "participants"), (workflows, "workflows"), (defects, "defects"), (discrepancies, "scientific_discrepancies")):
        require(isinstance(value, list), f"evidence.{label} must be a list")

    participant_map: dict[str, dict[str, Any]] = {}
    institution_ids: set[str] = set()
    experience_counts = {"experienced_smartpls": 0, "new_to_sem": 0, "other": 0}
    latest_activity: datetime | None = None
    for index, row in enumerate(participants):
        item = exact_keys(row, {"participant_id", "institution_id", "experience", "consent_record_id", "enrolled_at"}, f"participants[{index}]")
        participant_id = nonempty(item["participant_id"], f"participants[{index}].participant_id")
        institution_id = nonempty(item["institution_id"], f"participants[{index}].institution_id")
        require(bool(ID.fullmatch(participant_id)) and bool(ID.fullmatch(institution_id)), "participant/institution IDs must be pseudonymous stable IDs")
        require(participant_id not in participant_map, "participant IDs must be unique")
        experience = item["experience"]
        require(experience in experience_counts, f"participants[{index}].experience is invalid")
        record_id = nonempty(item["consent_record_id"], f"participants[{index}].consent_record_id")
        require(bool(RECORD_ID.fullmatch(record_id)), "consent_record_id is invalid")
        enrolled_at = timestamp(item["enrolled_at"], f"participants[{index}].enrolled_at", now)
        latest_activity = enrolled_at if latest_activity is None or enrolled_at > latest_activity else latest_activity
        participant_map[participant_id] = item
        institution_ids.add(institution_id)
        experience_counts[experience] += 1

    workflow_ids: set[str] = set()
    completed_journeys = 0
    unassisted_completed_journeys = 0
    real_workflows = 0
    candidate_ids: set[str] = set()
    for index, row in enumerate(workflows):
        item = exact_keys(row, {
            "workflow_id", "participant_id", "candidate_id", "completed_at", "real_workflow", "privacy_safe",
            "reproducible_data_loss", "archive_corruption", "journeys", "scientific_discrepancy_ids",
        }, f"workflows[{index}]")
        workflow_id = nonempty(item["workflow_id"], f"workflows[{index}].workflow_id")
        require(bool(ID.fullmatch(workflow_id)) and workflow_id not in workflow_ids, "workflow_id is invalid or duplicated")
        workflow_ids.add(workflow_id)
        participant_id = nonempty(item["participant_id"], f"workflows[{index}].participant_id")
        require(participant_id in participant_map, "workflow references an unknown participant")
        candidate_id = nonempty(item["candidate_id"], f"workflows[{index}].candidate_id")
        require(bool(SHA256.fullmatch(candidate_id)), "workflow candidate_id must be lowercase SHA-256")
        candidate_ids.add(candidate_id)
        completed_at = timestamp(item["completed_at"], f"workflows[{index}].completed_at", now)
        latest_activity = completed_at if latest_activity is None or completed_at > latest_activity else latest_activity
        require(item["real_workflow"] is True and item["privacy_safe"] is True, "beta workflows must be real and privacy-safe")
        require(item["reproducible_data_loss"] is False, "workflow reports reproducible data loss")
        require(item["archive_corruption"] is False, "workflow reports archive corruption")
        journey_rows = item["journeys"]
        require(isinstance(journey_rows, list) and len(journey_rows) == len(EXPECTED_JOURNEYS), "workflow journey set is incomplete")
        found: list[str] = []
        for journey_index, journey_row in enumerate(journey_rows):
            journey = exact_keys(journey_row, {"journey_id", "completed", "developer_assistance"}, f"workflows[{index}].journeys[{journey_index}]")
            journey_id = nonempty(journey["journey_id"], "journey_id")
            require(journey["completed"] in {True, False} and journey["developer_assistance"] in {True, False}, "journey booleans are invalid")
            found.append(journey_id)
            if journey["completed"]:
                completed_journeys += 1
                if not journey["developer_assistance"]:
                    unassisted_completed_journeys += 1
        require(found == EXPECTED_JOURNEYS, "workflow journey identity/order drifted")
        discrepancy_ids = item["scientific_discrepancy_ids"]
        require(isinstance(discrepancy_ids, list) and len(discrepancy_ids) == len(set(discrepancy_ids)), "workflow discrepancy IDs must be unique")
        for discrepancy_id in discrepancy_ids:
            require(isinstance(discrepancy_id, str) and bool(ID.fullmatch(discrepancy_id)), "workflow discrepancy ID is invalid")
        real_workflows += 1

    defect_ids: set[str] = set()
    unresolved_p0 = unresolved_p1 = 0
    for index, row in enumerate(defects):
        item = exact_keys(row, {"defect_id", "severity", "status", "data_loss", "archive_corruption", "closed_at"}, f"defects[{index}]")
        defect_id = nonempty(item["defect_id"], f"defects[{index}].defect_id")
        require(bool(ID.fullmatch(defect_id)) and defect_id not in defect_ids, "defect_id is invalid or duplicated")
        defect_ids.add(defect_id)
        require(item["severity"] in {"P0", "P1", "P2", "P3"}, "defect severity is invalid")
        require(item["status"] in {"open", "closed", "duplicate", "not_reproducible"}, "defect status is invalid")
        require(item["data_loss"] in {True, False} and item["archive_corruption"] in {True, False}, "defect flags are invalid")
        if item["status"] == "closed":
            closed_at = timestamp(item["closed_at"], f"defects[{index}].closed_at", now)
            latest_activity = closed_at if latest_activity is None or closed_at > latest_activity else latest_activity
        else:
            require(item["closed_at"] is None, "unclosed defect must not have closed_at")
        if item["status"] == "open" and item["severity"] == "P0": unresolved_p0 += 1
        if item["status"] == "open" and item["severity"] == "P1": unresolved_p1 += 1

    discrepancy_ids: set[str] = set()
    unresolved_material = 0
    for index, row in enumerate(discrepancies):
        item = exact_keys(row, {"discrepancy_id", "material", "status", "disposition", "closed_at"}, f"scientific_discrepancies[{index}]")
        discrepancy_id = nonempty(item["discrepancy_id"], f"scientific_discrepancies[{index}].discrepancy_id")
        require(bool(ID.fullmatch(discrepancy_id)) and discrepancy_id not in discrepancy_ids, "discrepancy_id is invalid or duplicated")
        discrepancy_ids.add(discrepancy_id)
        require(item["material"] in {True, False}, "discrepancy material flag is invalid")
        require(item["status"] in {"open", "fixed", "explained", "claim_narrowed", "release_blocking"}, "discrepancy status is invalid")
        nonempty(item["disposition"], f"scientific_discrepancies[{index}].disposition")
        if item["status"] in {"fixed", "explained", "claim_narrowed"}:
            closed_at = timestamp(item["closed_at"], f"scientific_discrepancies[{index}].closed_at", now)
            latest_activity = closed_at if latest_activity is None or closed_at > latest_activity else latest_activity
        else:
            require(item["closed_at"] is None, "open/blocking discrepancy must not have closed_at")
            if item["material"]: unresolved_material += 1
    referenced_discrepancies = {item for row in workflows for item in row["scientific_discrepancy_ids"]}
    require(referenced_discrepancies <= discrepancy_ids, "workflow references an unknown scientific discrepancy")

    final_candidate = evidence["final_candidate"]
    final_rerun = evidence["final_lifecycle_rerun"]
    final_candidate_id: str | None = None
    final_rerun_at: datetime | None = None
    candidate_binding: dict[str, object] | None = None
    if final_candidate is not None:
        candidate = exact_keys(final_candidate, {"candidate_id", "candidate_manifest"}, "final_candidate")
        final_candidate_id = nonempty(candidate["candidate_id"], "final_candidate.candidate_id")
        require(bool(SHA256.fullmatch(final_candidate_id)), "final candidate ID must be lowercase SHA-256")
        try:
            signer = _validate_signing_identity_policy(SIGNING_POLICY, repository_root=repository_root, require_approved=True)
            candidate_binding = _validate_candidate_manifest(
                candidate["candidate_manifest"],
                candidate_id=final_candidate_id,
                target_release=release,
                repository_root=repository_root,
                verify_authenticode=True,
                approved_signer=signer,
                authenticode_cache={},
                label="external beta final candidate",
            )
        except ContractError as error:
            raise BetaContractError(f"final candidate trust validation failed: {error}") from error
    if final_rerun is not None:
        require(candidate_binding is not None and final_candidate_id is not None, "lifecycle rerun requires a validated final candidate")
        try:
            rerun_descriptor = _validate_artifact_descriptor(
                final_rerun, repository_root=repository_root, label="external beta final lifecycle report"
            )
        except ContractError as error:
            raise BetaContractError(f"final lifecycle report descriptor is invalid: {error}") from error
        report_path = repository_root / str(rerun_descriptor["path"])
        rerun = strict_json(report_path)
        rerun = exact_keys(rerun, {
            "schema_version", "report_type", "target_release", "candidate_id", "candidate_manifest_sha256",
            "candidate_artifact_digests", "signature_report_digests", "signing_identity_id",
            "performed_at", "passed", "environment", "phases",
        }, "final_lifecycle_rerun report")
        require(rerun["schema_version"] == 1 and rerun["report_type"] == "quickpls_signed_candidate_lifecycle", "final lifecycle report identity is invalid")
        require(rerun["target_release"] == release and rerun["candidate_id"] == final_candidate_id, "lifecycle rerun must bind the final candidate")
        require(rerun["candidate_manifest_sha256"] == candidate_binding["candidate_manifest"]["sha256"], "lifecycle rerun manifest hash does not match")
        artifact_digests = {str(row["role"]): str(row["sha256"]) for row in candidate_binding["artifacts"]}
        signature_digests = {str(row["role"]): str(row["report"]["sha256"]) for row in candidate_binding["signature_evidence"]}
        require(rerun["candidate_artifact_digests"] == artifact_digests, "lifecycle rerun artifact digests do not match")
        require(rerun["signature_report_digests"] == signature_digests, "lifecycle rerun signature-report digests do not match")
        require(rerun["signing_identity_id"] == signer["identity_id"], "lifecycle rerun signer identity does not match")
        require(rerun["passed"] is True, "final lifecycle rerun must pass")
        environment = exact_keys(rerun["environment"], {"windows_version", "install_scope", "network_disconnected"}, "final_lifecycle_rerun.environment")
        nonempty(environment["windows_version"], "final_lifecycle_rerun.environment.windows_version")
        require(environment["install_scope"] in {"per_user", "per_machine"}, "final lifecycle install scope is invalid")
        require(environment["network_disconnected"] is True, "final lifecycle must exercise disconnected recovery")
        phases = rerun["phases"]
        require(isinstance(phases, list) and len(phases) == len(EXPECTED_LIFECYCLE_PHASES), "final lifecycle phase set is incomplete")
        phase_ids: list[str] = []
        for index, row in enumerate(phases):
            phase = exact_keys(row, {"id", "passed", "summary", "measurements"}, f"final_lifecycle_rerun.phases[{index}]")
            phase_ids.append(nonempty(phase["id"], f"final_lifecycle_rerun.phases[{index}].id"))
            require(phase["passed"] is True, f"final lifecycle phase {phase_ids[-1]} did not pass")
            nonempty(phase["summary"], f"final_lifecycle_rerun.phases[{index}].summary")
            require(isinstance(phase["measurements"], dict) and phase["measurements"], f"final lifecycle phase {phase_ids[-1]} has no measurements")
        require(phase_ids == EXPECTED_LIFECYCLE_PHASES, "final lifecycle phase order or identity drifted")
        final_rerun_at = timestamp(rerun["performed_at"], "final_lifecycle_rerun.performed_at", now)
        require(latest_activity is None or final_rerun_at >= latest_activity, "final lifecycle rerun predates beta activity or fixes")

    total_journeys = len(workflows) * len(EXPECTED_JOURNEYS)
    unassisted_rate = unassisted_completed_journeys / total_journeys if total_journeys else 0.0
    thresholds_met = {
        "participant_count": 15 <= len(participants) <= 25,
        "institution_count": len(institution_ids) >= 5,
        "experienced_count": experience_counts["experienced_smartpls"] >= 5,
        "new_sem_count": experience_counts["new_to_sem"] >= 5,
        "workflow_count": real_workflows >= 30,
        "unassisted_completion_rate": unassisted_rate >= 0.9,
        "zero_unresolved_p0_p1": unresolved_p0 == 0 and unresolved_p1 == 0,
        "zero_data_loss_or_corruption": all(not row["data_loss"] and not row["archive_corruption"] for row in defects),
        "scientific_discrepancies_disposed": unresolved_material == 0,
        "one_candidate": len(candidate_ids) <= 1 and (not candidate_ids or final_candidate_id in candidate_ids),
        "signed_final_candidate": final_candidate_id is not None,
        "post_fix_lifecycle_rerun": final_rerun_at is not None,
    }
    evidence_complete = bool(thresholds_met) and all(thresholds_met.values())

    decision = exact_keys(root["decision"], {"status", "approved_by", "approved_at", "record_id"}, "decision")
    require(decision["status"] in {"pending", "approved", "rejected"}, "decision.status is invalid")
    approved = decision["status"] == "approved"
    if approved:
        require(root["status"] == "completed" and evidence_complete, "approval requires completed passing beta evidence")
        nonempty(decision["approved_by"], "decision.approved_by")
        approved_at = timestamp(decision["approved_at"], "decision.approved_at", now)
        require(final_rerun_at is not None and approved_at >= final_rerun_at, "approval must follow the final lifecycle rerun")
        record_id = nonempty(decision["record_id"], "decision.record_id")
        require(bool(RECORD_ID.fullmatch(record_id)), "decision.record_id is invalid")
    else:
        require(decision["approved_by"] is None and decision["approved_at"] is None and decision["record_id"] is None, "unapproved decision metadata must be null")
    beta_ready = root["status"] == "completed" and evidence_complete and approved
    return {
        "passed": True,
        "program_id": root["program_id"],
        "target_release": release,
        "program_status": root["status"],
        "beta_ready": beta_ready,
        "claim_policy": claims,
        "counts": {
            "participants": len(participants), "institutions": len(institution_ids), "workflows": len(workflows),
            "experienced_smartpls": experience_counts["experienced_smartpls"], "new_to_sem": experience_counts["new_to_sem"],
            "unresolved_p0": unresolved_p0, "unresolved_p1": unresolved_p1, "unresolved_material_discrepancies": unresolved_material,
        },
        "unassisted_journey_completion_rate": unassisted_rate,
        "thresholds": thresholds_met,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--contract", type=Path, default=DEFAULT_CONTRACT)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument("--require-ready", action="store_true")
    args = parser.parse_args()
    try:
        report = validate_contract(strict_json(args.contract), repository_root=args.repo_root)
    except BetaContractError as error:
        print(json.dumps({"passed": False, "beta_ready": False, "errors": [str(error)]}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 1 if args.require_ready and not report["beta_ready"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
