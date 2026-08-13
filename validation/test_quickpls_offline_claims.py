from __future__ import annotations

import json
import shutil
import tempfile
import unittest
from pathlib import Path

from validation.quickpls_offline_claims import (
    EXTERNAL_BETA,
    GOVERNED_DOCS,
    READINESS,
    RELEASE_CHANNELS,
    ROOT,
    OfflineClaimError,
    validate_offline_claims,
)


def fixture_root(destination: Path) -> Path:
    for relative in (*GOVERNED_DOCS, READINESS, EXTERNAL_BETA, RELEASE_CHANNELS):
        target = destination / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(ROOT / relative, target)
    return destination


def mutate_json(root: Path, relative: Path, mutation) -> None:
    path = root / relative
    value = json.loads(path.read_text(encoding="utf-8"))
    mutation(value)
    path.write_text(json.dumps(value, indent=2) + "\n", encoding="utf-8")


class QuickPlsOfflineClaimsTests(unittest.TestCase):
    def test_repository_contract_keeps_functional_and_strict_claims_separate(self) -> None:
        report = validate_offline_claims(ROOT)
        self.assertTrue(report["passed"])
        self.assertEqual(report["functional_offline_claim"], "authorized")
        self.assertEqual(report["strict_process_tree_claim"], "prohibited")
        self.assertEqual(report["strict_gate_status"], "pending")

    def assert_rejected(self, mutation) -> None:
        with tempfile.TemporaryDirectory() as temp:
            root = fixture_root(Path(temp))
            mutation(root)
            with self.assertRaises(OfflineClaimError):
                validate_offline_claims(root)

    def test_rejects_a_premature_strict_gate_pass(self) -> None:
        self.assert_rejected(
            lambda root: mutate_json(
                root,
                READINESS,
                lambda value: value["product_policy"]["strict_zero_egress_claim_gate"].__setitem__(
                    "status", "passed"
                ),
            )
        )

    def test_rejects_removed_no_telemetry_guard(self) -> None:
        self.assert_rejected(
            lambda root: mutate_json(
                root,
                READINESS,
                lambda value: value["product_policy"]["prohibited_claims"].remove(
                    "no_telemetry_without_os_enforced_fixed_webview2_containment"
                ),
            )
        )

    def test_rejects_beta_or_stable_channel_claim_bypass(self) -> None:
        self.assert_rejected(
            lambda root: mutate_json(
                root,
                EXTERNAL_BETA,
                lambda value: value["claim_policy"].__setitem__("strict_process_tree_claim", "authorized"),
            )
        )
        self.assert_rejected(
            lambda root: mutate_json(
                root,
                RELEASE_CHANNELS,
                lambda value: value["channels"]["stable"].__setitem__(
                    "competitor_claims_policy", "authorized"
                ),
            )
        )

    def test_rejects_an_unqualified_public_fully_offline_claim(self) -> None:
        def mutation(root: Path) -> None:
            path = root / "README.md"
            path.write_text(path.read_text(encoding="utf-8") + "\nQuickPLS is fully offline.\n", encoding="utf-8")

        self.assert_rejected(mutation)


if __name__ == "__main__":
    unittest.main()
