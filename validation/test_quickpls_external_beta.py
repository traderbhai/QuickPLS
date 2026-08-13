from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

from validation import quickpls_3_release_readiness as readiness
from validation.quickpls_external_beta import (
    BetaContractError,
    DEFAULT_CONTRACT,
    EXPECTED_LIFECYCLE_PHASES,
    strict_json,
    validate_contract,
)
from validation.test_quickpls_3_release_readiness import (
    SIGNER_ID,
    build_candidate,
    sha256,
    trusted_cms_execution,
    trusted_signtool_execution,
    trusted_windows_identity,
    write_artifact,
)


NOW = datetime(2026, 8, 13, 12, 0, tzinfo=timezone.utc)
JOURNEYS = ["import_data", "author_model", "calculate", "interpret", "export", "save_reopen"]


def lifecycle_report(root: Path, candidate: tuple[str, dict], *, performed_at: str = "2026-08-12T09:00:00+00:00") -> dict:
    candidate_id, manifest_descriptor = candidate
    manifest = json.loads((root / manifest_descriptor["path"]).read_text(encoding="utf-8"))
    artifacts = {row["role"]: row["sha256"] for row in manifest["artifacts"]}
    signatures = {row["role"]: row["report"]["sha256"] for row in manifest["signature_evidence"]}
    report = {
        "schema_version": 1,
        "report_type": "quickpls_signed_candidate_lifecycle",
        "target_release": "3.0.0-beta",
        "candidate_id": candidate_id,
        "candidate_manifest_sha256": manifest_descriptor["sha256"],
        "candidate_artifact_digests": artifacts,
        "signature_report_digests": signatures,
        "signing_identity_id": SIGNER_ID,
        "performed_at": performed_at,
        "passed": True,
        "environment": {"windows_version": "Windows 11 24H2", "install_scope": "per_user", "network_disconnected": True},
        "phases": [
            {"id": phase, "passed": True, "summary": f"Verified {phase}.", "measurements": {"failures": 0}}
            for phase in EXPECTED_LIFECYCLE_PHASES
        ],
    }
    return write_artifact(root, "validation/results/external-beta/final-lifecycle.json", json.dumps(report).encode())


def passing_contract(root: Path) -> dict:
    contract = strict_json(DEFAULT_CONTRACT)
    contract["status"] = "completed"
    candidate = build_candidate(root, "external-beta", target_release="3.0.0-beta", channel="beta")
    participants = []
    for index in range(15):
        participants.append({
            "participant_id": f"participant_{index:02d}",
            "institution_id": f"institution_{index % 5:02d}",
            "experience": "experienced_smartpls" if index < 5 else "new_to_sem" if index < 10 else "other",
            "consent_record_id": f"consent:external-beta:{index:02d}",
            "enrolled_at": "2026-08-10T09:00:00+00:00",
        })
    workflows = []
    for index in range(30):
        workflows.append({
            "workflow_id": f"workflow_{index:02d}",
            "participant_id": f"participant_{index % 15:02d}",
            "candidate_id": candidate[0],
            "completed_at": "2026-08-11T09:00:00+00:00",
            "real_workflow": True,
            "privacy_safe": True,
            "reproducible_data_loss": False,
            "archive_corruption": False,
            "journeys": [{"journey_id": item, "completed": True, "developer_assistance": False} for item in JOURNEYS],
            "scientific_discrepancy_ids": [],
        })
    contract["evidence"]["participants"] = participants
    contract["evidence"]["workflows"] = workflows
    contract["evidence"]["defects"] = []
    contract["evidence"]["scientific_discrepancies"] = []
    contract["evidence"]["final_candidate"] = {"candidate_id": candidate[0], "candidate_manifest": candidate[1]}
    contract["evidence"]["final_lifecycle_rerun"] = lifecycle_report(root, candidate)
    contract["decision"] = {
        "status": "approved",
        "approved_by": "Independent beta release board",
        "approved_at": "2026-08-12T10:00:00+00:00",
        "record_id": "approval:external-beta:final",
    }
    return contract


def rewrite_lifecycle(root: Path, contract: dict, mutation) -> None:
    descriptor = contract["evidence"]["final_lifecycle_rerun"]
    path = root / descriptor["path"]
    report = json.loads(path.read_text(encoding="utf-8"))
    mutation(report)
    path.write_text(json.dumps(report), encoding="utf-8")
    descriptor["size"] = path.stat().st_size
    descriptor["sha256"] = sha256(path)


class QuickPlsExternalBetaTests(unittest.TestCase):
    def setUp(self) -> None:
        self.stack = []
        for target, side_effect in (
            (readiness._run_signtool, trusted_signtool_execution),
            (readiness._run_windows_file_identity, trusted_windows_identity),
            (readiness._run_windows_cms_verification, trusted_cms_execution),
        ):
            patcher = mock.patch.object(readiness, target.__name__, side_effect=side_effect)
            patcher.start()
            self.stack.append(patcher)
        self.addCleanup(lambda: [patcher.stop() for patcher in reversed(self.stack)])

    def test_planned_contract_is_valid_but_not_ready(self) -> None:
        report = validate_contract(strict_json(DEFAULT_CONTRACT), now=NOW)
        self.assertTrue(report["passed"])
        self.assertFalse(report["beta_ready"])
        self.assertEqual(
            report["claim_policy"]["strict_process_tree_claim"],
            "prohibited_pending_os_enforced_fixed_webview2_containment",
        )

    def test_beta_contract_cannot_authorize_a_strict_offline_claim(self) -> None:
        contract = strict_json(DEFAULT_CONTRACT)
        contract["claim_policy"]["strict_process_tree_claim"] = "authorized"
        with self.assertRaises(BetaContractError):
            validate_contract(contract, now=NOW)

    def test_exact_threshold_fixture_is_ready(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            report = validate_contract(passing_contract(root), now=NOW, repository_root=root)
        self.assertTrue(report["beta_ready"])
        self.assertEqual(report["counts"]["workflows"], 30)

    def assert_rejected(self, mutation) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            candidate = passing_contract(root)
            mutation(candidate, root)
            with self.assertRaises(BetaContractError):
                validate_contract(candidate, now=NOW, repository_root=root)

    def test_rejects_privacy_cohort_and_missing_journeys(self) -> None:
        self.assert_rejected(lambda value, _root: value["privacy_contract"].__setitem__("collect_raw_datasets", True))
        self.assert_rejected(lambda value, _root: value["cohort_contract"].__setitem__("minimum_participants", 5))
        self.assert_rejected(lambda value, _root: value["evidence"]["workflows"][0]["journeys"].pop())

    def test_rejects_data_loss_open_priority_and_material_science(self) -> None:
        self.assert_rejected(lambda value, _root: value["evidence"]["workflows"][0].__setitem__("reproducible_data_loss", True))
        def priority(value, _root):
            value["evidence"]["defects"] = [{
                "defect_id": "defect_p1", "severity": "P1", "status": "open", "data_loss": False,
                "archive_corruption": False, "closed_at": None,
            }]
        self.assert_rejected(priority)
        def science(value, _root):
            value["evidence"]["scientific_discrepancies"] = [{
                "discrepancy_id": "scientific_gap", "material": True, "status": "release_blocking",
                "disposition": "Blocks release", "closed_at": None,
            }]
        self.assert_rejected(science)

    def test_rejects_candidate_or_manifest_drift(self) -> None:
        self.assert_rejected(lambda value, _root: value["evidence"]["workflows"][0].__setitem__("candidate_id", "b" * 64))
        def tamper_manifest(value, root):
            descriptor = value["evidence"]["final_candidate"]["candidate_manifest"]
            (root / descriptor["path"]).write_text("{}", encoding="utf-8")
        self.assert_rejected(tamper_manifest)

    def test_rejects_lifecycle_opaque_or_wrong_exact_bindings(self) -> None:
        self.assert_rejected(lambda value, _root: value["evidence"].__setitem__("final_lifecycle_rerun", {"report_id": "opaque"}))
        self.assert_rejected(lambda value, root: rewrite_lifecycle(root, value, lambda report: report.__setitem__("candidate_manifest_sha256", "0" * 64)))
        self.assert_rejected(lambda value, root: rewrite_lifecycle(root, value, lambda report: report["phases"].pop()))

    def test_rejects_pre_activity_lifecycle_and_early_approval(self) -> None:
        self.assert_rejected(lambda value, root: rewrite_lifecycle(root, value, lambda report: report.__setitem__("performed_at", "2026-08-09T09:00:00+00:00")))
        self.assert_rejected(lambda value, _root: value["decision"].__setitem__("approved_at", "2026-08-11T10:00:00+00:00"))


if __name__ == "__main__":
    unittest.main()
