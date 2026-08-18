from __future__ import annotations

import contextlib
import copy
import io
import json
import shutil
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest import mock

from validation.quickpls_beta_operations_kit import (
    BetaKitError,
    DEFAULT_KIT,
    ROOT,
    main,
    strict_json,
    validate_kit,
)


NOW = datetime(2026, 8, 14, 12, 0, tzinfo=timezone.utc)


class QuickPlsBetaOperationsKitTests(unittest.TestCase):
    def workspace(self) -> tuple[tempfile.TemporaryDirectory[str], Path, Path]:
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        root = Path(temp.name)
        shutil.copytree(ROOT / "validation" / "beta_kit", root / "validation" / "beta_kit")
        shutil.copy2(
            ROOT / "validation" / "quickpls_external_beta.json",
            root / "validation" / "quickpls_external_beta.json",
        )
        kit = root / "validation" / "beta_kit" / "quickpls_beta_operations_kit.json"
        return temp, root, kit

    def rewrite(self, path: Path, mutation) -> None:
        value = json.loads(path.read_text(encoding="utf-8"))
        mutation(value)
        path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")

    def test_repository_kit_is_complete_but_not_beta_ready(self) -> None:
        report = validate_kit(DEFAULT_KIT, now=NOW)
        self.assertTrue(report["passed"])
        self.assertTrue(report["dry_run_ready"])
        self.assertFalse(report["beta_ready"])
        self.assertEqual(report["program_status"], "planned")
        self.assertEqual(report["templates_validated"], 7)
        self.assertEqual(report["canonical_beta"]["participants"], 0)
        self.assertEqual(report["canonical_beta"]["workflows"], 0)
        self.assertEqual(len(report["external_blockers"]), 10)

    def test_require_ready_fails_closed(self) -> None:
        output = io.StringIO()
        with mock.patch("sys.argv", ["quickpls_beta_operations_kit.py", "--require-ready"]), contextlib.redirect_stdout(output):
            code = main()
        self.assertEqual(code, 1)
        self.assertFalse(json.loads(output.getvalue())["beta_ready"])

    def test_participant_template_rejects_filled_or_identifying_values(self) -> None:
        _temp, root, kit = self.workspace()
        participant = root / "validation" / "beta_kit" / "templates" / "participant_record.template.json"
        self.rewrite(participant, lambda value: value["record"].__setitem__("participant_id", "participant_claimed"))
        with self.assertRaisesRegex(BetaKitError, "template data is not evidence"):
            validate_kit(kit, repository_root=root, now=NOW)

        shutil.copy2(ROOT / "validation" / "beta_kit" / "templates" / participant.name, participant)
        self.rewrite(participant, lambda value: value["repository_privacy_attestation"].__setitem__("contains_email", True))
        with self.assertRaisesRegex(BetaKitError, "private data"):
            validate_kit(kit, repository_root=root, now=NOW)

    def test_workflow_rejects_rubric_drift_and_claimed_outcome(self) -> None:
        _temp, root, kit = self.workspace()
        workflow = root / "validation" / "beta_kit" / "templates" / "workflow_observation.template.json"
        self.rewrite(workflow, lambda value: value["journeys"].reverse())
        with self.assertRaisesRegex(BetaKitError, "journey identity or order"):
            validate_kit(kit, repository_root=root, now=NOW)

        shutil.copy2(ROOT / "validation" / "beta_kit" / "templates" / workflow.name, workflow)
        self.rewrite(workflow, lambda value: value["journeys"][0].__setitem__("outcome", "completed"))
        with self.assertRaisesRegex(BetaKitError, "must remain null"):
            validate_kit(kit, repository_root=root, now=NOW)

    def test_privacy_request_rejects_repository_identity_handling(self) -> None:
        _temp, root, kit = self.workspace()
        privacy = root / "validation" / "beta_kit" / "templates" / "privacy_request.template.json"
        self.rewrite(
            privacy,
            lambda value: value["handling_policy"].__setitem__(
                "direct_identity_verification_occurs_outside_repository", False
            ),
        )
        with self.assertRaisesRegex(BetaKitError, "safeguard is disabled"):
            validate_kit(kit, repository_root=root, now=NOW)

    def test_candidate_lifecycle_and_exit_placeholders_cannot_claim_success(self) -> None:
        _temp, root, kit = self.workspace()
        lifecycle = root / "validation" / "beta_kit" / "templates" / "signed_candidate_lifecycle.template.json"
        self.rewrite(lifecycle, lambda value: value.__setitem__("candidate_id", "a" * 64))
        with self.assertRaisesRegex(BetaKitError, "must remain null"):
            validate_kit(kit, repository_root=root, now=NOW)

        shutil.copy2(ROOT / "validation" / "beta_kit" / "templates" / lifecycle.name, lifecycle)
        exit_form = root / "validation" / "beta_kit" / "templates" / "exit_decision.template.json"
        self.rewrite(exit_form, lambda value: value["decision"].__setitem__("status", "approved"))
        with self.assertRaisesRegex(BetaKitError, "must remain pending"):
            validate_kit(kit, repository_root=root, now=NOW)

    def test_canonical_contract_must_remain_empty_planned_and_pending(self) -> None:
        _temp, root, kit = self.workspace()
        contract = root / "validation" / "quickpls_external_beta.json"

        def add_claimed_participant(value: dict) -> None:
            value["evidence"]["participants"] = [copy.deepcopy({
                "participant_id": "participant_claimed",
                "institution_id": "institution_claimed",
                "experience": "other",
                "consent_record_id": "consent:claimed:record",
                "enrolled_at": "2026-08-14T00:00:00+00:00",
            })]

        self.rewrite(contract, add_claimed_participant)
        with self.assertRaisesRegex(BetaKitError, "contains claimed evidence"):
            validate_kit(kit, repository_root=root, now=NOW)

        shutil.copy2(ROOT / "validation" / "quickpls_external_beta.json", contract)
        self.rewrite(contract, lambda value: value.__setitem__("status", "running"))
        with self.assertRaisesRegex(BetaKitError, "must remain planned"):
            validate_kit(kit, repository_root=root, now=NOW)

    def test_manifest_paths_and_external_gates_are_frozen_pending(self) -> None:
        _temp, root, kit = self.workspace()
        self.rewrite(kit, lambda value: value["templates"][0].__setitem__("path", "../participant.json"))
        with self.assertRaisesRegex(BetaKitError, "roles or paths drifted"):
            validate_kit(kit, repository_root=root, now=NOW)

        shutil.copy2(DEFAULT_KIT, kit)
        self.rewrite(kit, lambda value: value["external_gates"][0].__setitem__("status", "complete"))
        with self.assertRaisesRegex(BetaKitError, "must remain pending"):
            validate_kit(kit, repository_root=root, now=NOW)

    def test_strict_json_rejects_duplicate_keys_and_nonfinite_numbers(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            duplicate = Path(temp) / "duplicate.json"
            duplicate.write_text('{"status":"planned","status":"complete"}', encoding="utf-8")
            with self.assertRaisesRegex(BetaKitError, "duplicate JSON key"):
                strict_json(duplicate)
            nonfinite = Path(temp) / "nonfinite.json"
            nonfinite.write_text('{"value":NaN}', encoding="utf-8")
            with self.assertRaisesRegex(BetaKitError, "non-finite JSON token"):
                strict_json(nonfinite)


if __name__ == "__main__":
    unittest.main()
