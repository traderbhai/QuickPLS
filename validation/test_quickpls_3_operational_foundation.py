from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path

from validation import quickpls_3_operational_foundation as operations


ROOT = Path(__file__).resolve().parents[1]
CONTRACT_PATH = ROOT / "validation" / "quickpls_3_operational_foundation.json"


class QuickPls3OperationalFoundationTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = operations.load_contract(CONTRACT_PATH)

    def test_current_foundation_is_strict_and_does_not_claim_release_readiness(self) -> None:
        report = operations.validate_contract(self.contract, repository_root=ROOT)

        self.assertTrue(report["structurally_valid"])
        self.assertTrue(report["foundation_ready"])
        self.assertFalse(report["commercial_ready"])
        self.assertEqual(report["document_count"], 8)
        self.assertEqual(report["control_count"], 10)
        self.assertEqual(
            report["state_counts"],
            {
                "external_review_required": 2,
                "implementation_required": 5,
                "policy_defined": 3,
            },
        )
        self.assertEqual(
            report["pending_release_requirements"],
            [
                "supply_chain.provenance",
                "supply_chain.sbom_licenses",
                "support.docs_diagnostics",
                "support.operations",
                "trust.legal",
                "trust.privacy_telemetry",
                "trust.security",
            ],
        )

    def test_document_bytes_and_headings_are_bound(self) -> None:
        digest_change = copy.deepcopy(self.contract)
        digest_change["documents"][0]["sha256"] = "0" * 64
        with self.assertRaisesRegex(operations.ContractError, "does not match the document bytes"):
            operations.validate_contract(digest_change, repository_root=ROOT)

        missing_heading = copy.deepcopy(self.contract)
        missing_heading["documents"][0]["required_headings"].append("## Invented Completion Claim")
        with self.assertRaisesRegex(operations.ContractError, "missing required heading"):
            operations.validate_contract(missing_heading, repository_root=ROOT)

    def test_document_traversal_and_unknown_artifacts_are_rejected(self) -> None:
        traversal = copy.deepcopy(self.contract)
        traversal["documents"][0]["path"] = "docs/../SECURITY.md"
        with self.assertRaisesRegex(operations.ContractError, "must not traverse"):
            operations.validate_contract(traversal, repository_root=ROOT)

        unknown = copy.deepcopy(self.contract)
        unknown["documents"][0]["id"] = "unreviewed_policy"
        with self.assertRaisesRegex(operations.ContractError, "document IDs differ"):
            operations.validate_contract(unknown, repository_root=ROOT)

    def test_control_mappings_and_pending_evidence_fail_closed(self) -> None:
        borrowed = copy.deepcopy(self.contract)
        borrowed["controls"][0]["release_requirement_ids"] = ["trust.legal"]
        with self.assertRaisesRegex(operations.ContractError, "frozen mapping"):
            operations.validate_contract(borrowed, repository_root=ROOT)

        no_remaining_evidence = copy.deepcopy(self.contract)
        no_remaining_evidence["controls"][0]["remaining_release_evidence"] = []
        with self.assertRaisesRegex(operations.ContractError, "non-empty array"):
            operations.validate_contract(no_remaining_evidence, repository_root=ROOT)

        promoted = copy.deepcopy(self.contract)
        promoted["controls"][0]["state"] = "verified"
        with self.assertRaisesRegex(operations.ContractError, "invalid for this pre-certificate control"):
            operations.validate_contract(promoted, repository_root=ROOT)

    def test_operational_manifest_cannot_replace_the_commercial_gate(self) -> None:
        claimed = copy.deepcopy(self.contract)
        claimed["release_claim"]["commercial_ready"] = True
        with self.assertRaisesRegex(operations.ContractError, "must not claim commercial readiness"):
            operations.validate_contract(claimed, repository_root=ROOT)

        redirected = copy.deepcopy(self.contract)
        redirected["release_claim"]["authoritative_gate"] = "docs/SUPPORT_POLICY.md"
        with self.assertRaisesRegex(operations.ContractError, "authoritative_gate is invalid"):
            operations.validate_contract(redirected, repository_root=ROOT)

    def test_duplicate_keys_and_nonfinite_json_are_rejected(self) -> None:
        cases = [
            '{"schema_version":1,"schema_version":1}',
            '{"schema_version":NaN}',
        ]
        for payload in cases:
            with self.subTest(payload=payload), tempfile.TemporaryDirectory() as temporary_directory:
                path = Path(temporary_directory) / "contract.json"
                path.write_text(payload, encoding="utf-8")
                with self.assertRaises(operations.ContractError):
                    operations.load_contract(path)


if __name__ == "__main__":
    unittest.main()
