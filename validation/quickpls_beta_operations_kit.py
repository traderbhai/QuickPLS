"""Validate the privacy-safe QuickPLS 3 external-beta preparation kit.

This validator is intentionally limited to a pre-beta dry run. It proves that
the operating templates are complete and inert, and that the canonical beta
contract still contains no claimed external evidence. It cannot make the beta
ready; real evidence is accepted only by ``quickpls_external_beta.py``.
"""

from __future__ import annotations

import argparse
import json
import math
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    from validation import quickpls_external_beta
except ModuleNotFoundError:
    import sys

    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
    from validation import quickpls_external_beta  # type: ignore[no-redef]


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_KIT = ROOT / "validation" / "beta_kit" / "quickpls_beta_operations_kit.json"
CANONICAL_CONTRACT = "validation/quickpls_external_beta.json"
EXPECTED_JOURNEYS = [
    "import_data",
    "author_model",
    "calculate",
    "interpret",
    "export",
    "save_reopen",
]
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
EXPECTED_EXIT_CHECKS = [
    "cohort_thresholds",
    "six_journey_threshold",
    "p0_p1_closed",
    "zero_data_loss_or_corruption",
    "scientific_discrepancies_disposed",
    "independent_reviews_disposed",
    "support_routes_exercised",
    "exact_signed_candidate_bound",
    "post_beta_lifecycle_passed",
]
EXPECTED_TEMPLATES = {
    "participant_record": "validation/beta_kit/templates/participant_record.template.json",
    "workflow_observation": "validation/beta_kit/templates/workflow_observation.template.json",
    "privacy_request": "validation/beta_kit/templates/privacy_request.template.json",
    "defect_register": "validation/beta_kit/templates/defect_register.template.json",
    "scientific_discrepancy_register": "validation/beta_kit/templates/scientific_discrepancy_register.template.json",
    "signed_candidate_lifecycle": "validation/beta_kit/templates/signed_candidate_lifecycle.template.json",
    "exit_decision": "validation/beta_kit/templates/exit_decision.template.json",
}
EXPECTED_EXTERNAL_GATES = [
    "approved_signing_identity",
    "signed_beta_candidate",
    "windows_lifecycle_entry_gate",
    "independent_entry_reviews",
    "support_operations",
    "real_beta_cohort",
    "real_workflow_evidence",
    "beta_findings_disposition",
    "post_beta_signed_lifecycle",
    "independent_exit_approval",
]


class BetaKitError(ValueError):
    """Raised when the preparation kit can no longer be trusted as inert."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise BetaKitError(message)


def strict_json(path: Path) -> Any:
    def pairs(rows: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, value in rows:
            if key in result:
                raise BetaKitError(f"duplicate JSON key in {path}: {key}")
            result[key] = value
        return result

    def finite(value: str) -> float:
        parsed = float(value)
        if not math.isfinite(parsed):
            raise BetaKitError(f"non-finite JSON number in {path}")
        return parsed

    try:
        return json.loads(
            path.read_text(encoding="utf-8"),
            object_pairs_hook=pairs,
            parse_constant=lambda value: (_ for _ in ()).throw(
                BetaKitError(f"non-finite JSON token in {path}: {value}")
            ),
            parse_float=finite,
        )
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise BetaKitError(f"cannot load {path}: {error}") from error


def exact(value: object, keys: set[str], label: str) -> dict[str, Any]:
    require(isinstance(value, dict), f"{label} must be an object")
    require(set(value) == keys, f"{label} keys do not match the frozen beta-kit contract")
    return value


def nonempty(value: object, label: str) -> str:
    require(isinstance(value, str) and bool(value.strip()), f"{label} must be a non-empty string")
    return value.strip()


def all_null(value: dict[str, Any], label: str, *, empty_lists: set[str] | None = None) -> None:
    empty_lists = empty_lists or set()
    for key, item in value.items():
        if key in empty_lists:
            require(item == [], f"{label}.{key} must remain an empty template list")
        else:
            require(item is None, f"{label}.{key} must remain null; template data is not evidence")


def template_path(repository_root: Path, relative: str) -> Path:
    root = repository_root.resolve()
    target = (root / relative).resolve()
    try:
        target.relative_to(root)
    except ValueError as error:
        raise BetaKitError(f"template path escapes repository root: {relative}") from error
    require(target.is_file() and not target.is_symlink(), f"template is missing or not a regular file: {relative}")
    return target


def validate_participant(value: object) -> None:
    root = exact(value, {
        "schema_version", "record_type", "template_only", "identifier_protocol",
        "record", "allowed_experience_values", "repository_privacy_attestation",
    }, "participant template")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_beta_participant_template", "participant template identity is invalid")
    require(root["template_only"] is True, "participant template must remain template-only")
    protocol = exact(root["identifier_protocol"], {
        "participant_id_format", "institution_id_format", "stable_scope", "generated_outside_repository",
        "direct_identifier_mapping_stored_outside_repository", "identifiers_must_not_encode_name_email_or_institution_name",
    }, "participant identifier protocol")
    require(protocol == {
        "participant_id_format": "participant_<lowercase_random_token>",
        "institution_id_format": "institution_<lowercase_random_token>",
        "stable_scope": "single_beta_program_only",
        "generated_outside_repository": True,
        "direct_identifier_mapping_stored_outside_repository": True,
        "identifiers_must_not_encode_name_email_or_institution_name": True,
    }, "participant pseudonym protocol drifted")
    record = exact(root["record"], {
        "participant_id", "institution_id", "experience", "consent_record_id", "consent_version", "consented_at", "enrolled_at",
    }, "participant record")
    all_null(record, "participant record")
    require(root["allowed_experience_values"] == ["experienced_smartpls", "new_to_sem", "other"], "participant experience values drifted")
    attestation = exact(root["repository_privacy_attestation"], {
        "contains_name", "contains_email", "contains_direct_identifier", "contains_raw_dataset", "contains_identifier_mapping",
    }, "participant privacy attestation")
    require(all(item is False for item in attestation.values()), "participant template claims repository-held private data")


def validate_workflow(value: object) -> None:
    root = exact(value, {
        "schema_version", "record_type", "template_only", "record", "assistance_scale",
        "journeys", "allowed_outcomes", "canonical_mapping",
    }, "workflow template")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_beta_workflow_observation_template", "workflow template identity is invalid")
    require(root["template_only"] is True, "workflow template must remain template-only")
    record = exact(root["record"], {
        "workflow_id", "participant_id", "candidate_id", "started_at", "completed_at", "real_workflow", "privacy_safe",
        "reproducible_data_loss", "archive_corruption", "scientific_discrepancy_ids",
    }, "workflow record")
    all_null(record, "workflow record", empty_lists={"scientific_discrepancy_ids"})
    assistance = root["assistance_scale"]
    require(isinstance(assistance, list) and len(assistance) == 4, "workflow assistance scale is incomplete")
    expected_assistance = [
        ("none", False), ("prompted", True), ("guided", True), ("developer_intervention", True),
    ]
    observed_assistance: list[tuple[str, bool]] = []
    for index, item in enumerate(assistance):
        row = exact(item, {"value", "developer_assistance", "meaning"}, f"assistance_scale[{index}]")
        observed_assistance.append((nonempty(row["value"], "assistance value"), row["developer_assistance"]))
        nonempty(row["meaning"], "assistance meaning")
    require(observed_assistance == expected_assistance, "workflow assistance mapping drifted")
    journeys = root["journeys"]
    require(isinstance(journeys, list) and len(journeys) == 6, "workflow must contain all six journey rubrics")
    ids: list[str] = []
    for index, item in enumerate(journeys):
        row = exact(item, {
            "journey_id", "required", "success_criterion", "outcome", "assistance_level", "elapsed_minutes", "privacy_safe_evidence_note",
        }, f"journeys[{index}]")
        ids.append(nonempty(row["journey_id"], f"journeys[{index}].journey_id"))
        require(row["required"] is True, f"journey {ids[-1]} must remain required")
        nonempty(row["success_criterion"], f"journey {ids[-1]} success criterion")
        for key in ("outcome", "assistance_level", "elapsed_minutes", "privacy_safe_evidence_note"):
            require(row[key] is None, f"journey {ids[-1]}.{key} must remain null in the template")
    require(ids == EXPECTED_JOURNEYS, "workflow journey identity or order drifted")
    require(root["allowed_outcomes"] == ["completed", "incomplete", "blocked", "not_attempted"], "workflow outcomes drifted")
    mapping = exact(root["canonical_mapping"], {"completed", "developer_assistance", "unassisted_success"}, "canonical journey mapping")
    require(mapping == {
        "completed": "outcome equals completed",
        "developer_assistance": "assistance_level is not none",
        "unassisted_success": "outcome equals completed and assistance_level equals none",
    }, "canonical journey mapping drifted")


def validate_privacy_request(value: object) -> None:
    root = exact(value, {
        "schema_version", "record_type", "template_only", "record", "allowed_request_types", "allowed_status_values", "handling_policy",
    }, "privacy-request template")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_beta_privacy_request_template", "privacy-request template identity is invalid")
    require(root["template_only"] is True, "privacy-request template must remain template-only")
    record = exact(root["record"], {
        "request_id", "participant_id", "request_type", "received_at", "verified_at", "completed_at", "external_record_id", "status",
    }, "privacy-request record")
    all_null(record, "privacy-request record")
    require(root["allowed_request_types"] == ["access", "correction", "withdrawal", "deletion"], "privacy request types drifted")
    require(root["allowed_status_values"] == ["received", "identity_verified_externally", "completed", "rejected_with_reason"], "privacy request statuses drifted")
    policy = exact(root["handling_policy"], {
        "route", "direct_identity_verification_occurs_outside_repository", "consent_and_identity_mapping_updates_occur_outside_repository",
        "repository_record_contains_only_pseudonymous_id_and_external_record_id", "raw_request_content_committed",
        "completion_evidence_required_before_marking_completed",
    }, "privacy handling policy")
    require(policy["route"] == "privacy@quickpls.org", "privacy route drifted from the canonical beta contract")
    for key in (
        "direct_identity_verification_occurs_outside_repository",
        "consent_and_identity_mapping_updates_occur_outside_repository",
        "repository_record_contains_only_pseudonymous_id_and_external_record_id",
        "completion_evidence_required_before_marking_completed",
    ):
        require(policy[key] is True, f"privacy handling safeguard is disabled: {key}")
    require(policy["raw_request_content_committed"] is False, "raw privacy-request content must not be committed")


def validate_defects(value: object) -> None:
    root = exact(value, {
        "schema_version", "record_type", "template_only", "records", "record_template", "allowed_severities", "allowed_status_values", "exit_rules",
    }, "defect-register template")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_beta_defect_register_template", "defect-register template identity is invalid")
    require(root["template_only"] is True and root["records"] == [], "defect register must remain an empty template")
    record = exact(root["record_template"], {
        "defect_id", "workflow_id", "candidate_id", "severity", "status", "data_loss", "archive_corruption", "privacy_safe_summary",
        "synthetic_reproduction_id", "opened_at", "closed_at", "closure_evidence_record_id",
    }, "defect record template")
    all_null(record, "defect record template")
    require(root["allowed_severities"] == ["P0", "P1", "P2", "P3"], "defect severities drifted")
    require(root["allowed_status_values"] == ["open", "closed", "duplicate", "not_reproducible"], "defect statuses drifted")
    rules = exact(root["exit_rules"], {
        "maximum_unresolved_p0", "maximum_unresolved_p1", "maximum_reproducible_data_loss", "maximum_archive_corruption", "closed_items_require_closure_evidence",
    }, "defect exit rules")
    require(rules == {
        "maximum_unresolved_p0": 0,
        "maximum_unresolved_p1": 0,
        "maximum_reproducible_data_loss": 0,
        "maximum_archive_corruption": 0,
        "closed_items_require_closure_evidence": True,
    }, "defect exit rules drifted")


def validate_discrepancies(value: object) -> None:
    root = exact(value, {
        "schema_version", "record_type", "template_only", "records", "record_template", "allowed_status_values", "exit_rule", "beta_does_not_replace_method_promotion",
    }, "scientific-discrepancy template")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_beta_scientific_discrepancy_register_template", "scientific-discrepancy template identity is invalid")
    require(root["template_only"] is True and root["records"] == [], "scientific-discrepancy register must remain an empty template")
    record = exact(root["record_template"], {
        "discrepancy_id", "workflow_id", "candidate_id", "method_id", "material", "status", "privacy_safe_summary",
        "independent_reference_record_id", "disposition", "opened_at", "closed_at",
    }, "scientific-discrepancy record template")
    all_null(record, "scientific-discrepancy record template")
    require(root["allowed_status_values"] == ["open", "fixed", "explained", "claim_narrowed", "release_blocking"], "scientific-discrepancy statuses drifted")
    nonempty(root["exit_rule"], "scientific-discrepancy exit rule")
    require(root["beta_does_not_replace_method_promotion"] is True, "beta must not replace scientific method promotion")


def validate_lifecycle(value: object) -> None:
    root = exact(value, {
        "schema_version", "record_type", "template_only", "eligible_for_release_evidence", "target_release", "candidate_id",
        "candidate_manifest", "final_lifecycle_report", "signing_identity_id", "performed_at", "passed", "environment", "phases", "activation_rule",
    }, "signed-candidate lifecycle placeholder")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_signed_candidate_lifecycle_placeholder", "lifecycle placeholder identity is invalid")
    require(root["template_only"] is True and root["eligible_for_release_evidence"] is False, "lifecycle placeholder cannot be release evidence")
    require(root["target_release"] == "3.0.0-beta", "lifecycle target release drifted")
    for key in ("candidate_id", "candidate_manifest", "final_lifecycle_report", "signing_identity_id", "performed_at", "passed"):
        require(root[key] is None, f"lifecycle placeholder {key} must remain null")
    environment = exact(root["environment"], {"windows_version", "install_scope", "network_disconnected"}, "lifecycle environment")
    all_null(environment, "lifecycle environment")
    phases = root["phases"]
    require(isinstance(phases, list) and len(phases) == len(EXPECTED_LIFECYCLE_PHASES), "lifecycle placeholder phases are incomplete")
    phase_ids: list[str] = []
    for index, item in enumerate(phases):
        row = exact(item, {"id", "passed", "evidence_record_id"}, f"lifecycle phases[{index}]")
        phase_ids.append(nonempty(row["id"], f"lifecycle phases[{index}].id"))
        require(row["passed"] is None and row["evidence_record_id"] is None, "lifecycle placeholders cannot claim phase evidence")
    require(phase_ids == EXPECTED_LIFECYCLE_PHASES, "lifecycle phase identity or order drifted")
    nonempty(root["activation_rule"], "lifecycle activation rule")


def validate_exit(value: object) -> None:
    root = exact(value, {"schema_version", "record_type", "template_only", "checklist", "decision", "beta_ready", "activation_rule"}, "exit-decision template")
    require(root["schema_version"] == 1 and root["record_type"] == "quickpls_beta_exit_decision_template", "exit-decision template identity is invalid")
    require(root["template_only"] is True and root["beta_ready"] is False, "exit template must remain pending and not beta-ready")
    checklist = root["checklist"]
    require(isinstance(checklist, list) and len(checklist) == len(EXPECTED_EXIT_CHECKS), "exit checklist is incomplete")
    check_ids: list[str] = []
    for index, item in enumerate(checklist):
        row = exact(item, {"id", "status", "evidence_record_id"}, f"exit checklist[{index}]")
        check_ids.append(nonempty(row["id"], f"exit checklist[{index}].id"))
        require(row["status"] == "pending" and row["evidence_record_id"] is None, "dry-run exit checks must remain pending without evidence")
    require(check_ids == EXPECTED_EXIT_CHECKS, "exit checklist identity or order drifted")
    decision = exact(root["decision"], {"status", "approved_by", "approved_at", "record_id"}, "exit decision")
    require(decision == {"status": "pending", "approved_by": None, "approved_at": None, "record_id": None}, "exit decision template must remain pending")
    nonempty(root["activation_rule"], "exit activation rule")


TEMPLATE_VALIDATORS = {
    "participant_record": validate_participant,
    "workflow_observation": validate_workflow,
    "privacy_request": validate_privacy_request,
    "defect_register": validate_defects,
    "scientific_discrepancy_register": validate_discrepancies,
    "signed_candidate_lifecycle": validate_lifecycle,
    "exit_decision": validate_exit,
}


def validate_kit(
    kit_path: Path = DEFAULT_KIT,
    *,
    repository_root: Path = ROOT,
    now: datetime | None = None,
) -> dict[str, Any]:
    now = now or datetime.now(timezone.utc)
    root = exact(strict_json(kit_path), {
        "schema_version", "kit_id", "program_status", "operating_mode", "canonical_contract",
        "evidence_policy", "privacy_policy", "journey_ids", "templates", "external_gates",
    }, "beta operations kit")
    require(root["schema_version"] == 1 and root["kit_id"] == "quickpls_3_external_beta_operations_v1", "beta operations kit identity is invalid")
    require(root["program_status"] == "planned" and root["operating_mode"] == "preparation_only", "beta kit may only describe planned preparation")
    require(root["canonical_contract"] == CANONICAL_CONTRACT, "beta kit must reference the canonical external-beta contract")
    policy = exact(root["evidence_policy"], {
        "fabricated_evidence_prohibited", "template_values_are_evidence", "repository_may_store_direct_identifiers",
        "repository_may_store_raw_participant_data", "beta_ready",
    }, "beta-kit evidence policy")
    require(policy == {
        "fabricated_evidence_prohibited": True,
        "template_values_are_evidence": False,
        "repository_may_store_direct_identifiers": False,
        "repository_may_store_raw_participant_data": False,
        "beta_ready": False,
    }, "beta-kit evidence policy drifted")
    privacy = exact(root["privacy_policy"], {
        "pseudonym_scope", "direct_identifier_mapping_location", "consent_record_location", "deletion_request_route",
        "deletion_completion_must_be_recorded_externally",
    }, "beta-kit privacy policy")
    require(privacy == {
        "pseudonym_scope": "single_beta_program_only",
        "direct_identifier_mapping_location": "external_access_controlled_system",
        "consent_record_location": "external_access_controlled_system",
        "deletion_request_route": "privacy@quickpls.org",
        "deletion_completion_must_be_recorded_externally": True,
    }, "beta-kit privacy policy drifted")
    require(root["journey_ids"] == EXPECTED_JOURNEYS, "beta-kit journey list drifted")

    templates = root["templates"]
    require(isinstance(templates, list) and len(templates) == len(EXPECTED_TEMPLATES), "beta-kit template list is incomplete")
    observed_templates: dict[str, str] = {}
    for index, item in enumerate(templates):
        row = exact(item, {"role", "path"}, f"templates[{index}]")
        role = nonempty(row["role"], f"templates[{index}].role")
        relative = nonempty(row["path"], f"templates[{index}].path")
        require(role not in observed_templates, f"duplicate beta-kit template role: {role}")
        observed_templates[role] = relative
    require(observed_templates == EXPECTED_TEMPLATES, "beta-kit template roles or paths drifted")
    for role, relative in EXPECTED_TEMPLATES.items():
        TEMPLATE_VALIDATORS[role](strict_json(template_path(repository_root, relative)))

    gates = root["external_gates"]
    require(isinstance(gates, list) and len(gates) == len(EXPECTED_EXTERNAL_GATES), "external beta-gate list is incomplete")
    gate_ids: list[str] = []
    external_blockers: list[dict[str, str]] = []
    for index, item in enumerate(gates):
        row = exact(item, {"id", "status", "required_evidence"}, f"external_gates[{index}]")
        gate_id = nonempty(row["id"], f"external_gates[{index}].id")
        require(row["status"] == "pending_external", f"external gate {gate_id} must remain pending")
        required_evidence = nonempty(row["required_evidence"], f"external_gates[{index}].required_evidence")
        gate_ids.append(gate_id)
        external_blockers.append({"id": gate_id, "required_evidence": required_evidence})
    require(gate_ids == EXPECTED_EXTERNAL_GATES, "external beta-gate identity or order drifted")

    contract_path = template_path(repository_root, CANONICAL_CONTRACT)
    contract = strict_json(contract_path)
    require(isinstance(contract, dict), "canonical beta contract must be an object")
    require(contract.get("status") == "planned", "canonical beta contract must remain planned during kit dry run")
    evidence = exact(contract.get("evidence"), {
        "participants", "workflows", "defects", "scientific_discrepancies", "final_candidate", "final_lifecycle_rerun",
    }, "canonical beta evidence")
    require(evidence == {
        "participants": [],
        "workflows": [],
        "defects": [],
        "scientific_discrepancies": [],
        "final_candidate": None,
        "final_lifecycle_rerun": None,
    }, "canonical beta contract contains claimed evidence; the dry-run kit cannot validate real evidence")
    decision = exact(contract.get("decision"), {"status", "approved_by", "approved_at", "record_id"}, "canonical beta decision")
    require(decision == {"status": "pending", "approved_by": None, "approved_at": None, "record_id": None}, "canonical beta decision must remain pending during dry run")
    try:
        canonical_report = quickpls_external_beta.validate_contract(contract, now=now, repository_root=repository_root)
    except quickpls_external_beta.BetaContractError as error:
        raise BetaKitError(f"canonical external-beta contract failed validation: {error}") from error
    require(canonical_report.get("passed") is True, "canonical external-beta contract did not pass structural validation")
    require(canonical_report.get("program_status") == "planned", "canonical external-beta report is not planned")
    require(canonical_report.get("beta_ready") is False, "dry-run kit must fail closed when canonical beta is ready")

    return {
        "passed": True,
        "dry_run_ready": True,
        "beta_ready": False,
        "kit_id": root["kit_id"],
        "program_status": "planned",
        "operating_mode": "preparation_only",
        "templates_validated": len(EXPECTED_TEMPLATES),
        "journeys_validated": EXPECTED_JOURNEYS,
        "canonical_beta": {
            "passed": True,
            "program_status": canonical_report["program_status"],
            "beta_ready": False,
            "participants": canonical_report["counts"]["participants"],
            "workflows": canonical_report["counts"]["workflows"],
            "decision_status": decision["status"],
        },
        "external_blockers": external_blockers,
        "next_validator_for_real_evidence": "python validation/quickpls_external_beta.py --require-ready",
    }


def main() -> int:
    parser = argparse.ArgumentParser(description="Dry-run the QuickPLS 3 external-beta operations kit")
    parser.add_argument("--kit", type=Path, default=DEFAULT_KIT)
    parser.add_argument("--repo-root", type=Path, default=ROOT)
    parser.add_argument("--require-ready", action="store_true", help="Fail unless beta_ready is true (expected to fail for this preparation-only kit)")
    args = parser.parse_args()
    try:
        report = validate_kit(args.kit, repository_root=args.repo_root)
    except BetaKitError as error:
        print(json.dumps({"passed": False, "dry_run_ready": False, "beta_ready": False, "errors": [str(error)]}, indent=2))
        return 1
    print(json.dumps(report, indent=2))
    return 1 if args.require_ready and report["beta_ready"] is not True else 0


if __name__ == "__main__":
    raise SystemExit(main())
