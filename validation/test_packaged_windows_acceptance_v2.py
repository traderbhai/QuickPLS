from __future__ import annotations

import copy
import hashlib
import json
import unittest
from pathlib import Path

from validation.packaged_windows_acceptance_v2 import (
    CONTRACT,
    EXPECTED_CHECK_COUNT,
    EXPECTED_CHECK_IDS,
    PHASE2_RELEASE_CHECK_IDS,
    PackagedAcceptanceContractError,
    receipt_binds_packaged_acceptance_contract,
    validate_packaged_acceptance_contract,
    validate_required_report_checks,
)


ROOT = Path(__file__).resolve().parents[1]
REPORT = ROOT / "validation/results/v247_tauri_native_acceptance.json"
CONTRACT_BOUND_METHOD_MANIFESTS = (
    "cbsem_ml_v1",
    "cca_residuals_v1",
    "gsca_als_v2",
    "ipma_v1",
    "logistic_regression_v2",
    "nca_v2",
    "ols_v1",
    "pls_algorithm_v1",
    "pls_bootstrap_v4",
    "plsc_v2",
    "plspredict_cvpat_v2",
    "process_v2",
    "regression_bootstrap_v1",
    "structural_path_randomization_v1",
    "wpls_v1",
)
CONTRACT_SOURCES = {
    "validation/packaged_windows_acceptance_v2.py",
    "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
}


class PackagedWindowsAcceptanceV2Tests(unittest.TestCase):
    def test_required_ids_and_count_are_manifest_derived(self) -> None:
        self.assertEqual(EXPECTED_CHECK_COUNT, len(EXPECTED_CHECK_IDS))
        self.assertGreater(EXPECTED_CHECK_COUNT, 0)
        self.assertEqual(len(EXPECTED_CHECK_IDS), len(set(EXPECTED_CHECK_IDS)))
        self.assertTrue(set(PHASE2_RELEASE_CHECK_IDS).issubset(EXPECTED_CHECK_IDS))
        self.assertEqual(CONTRACT["final_scope"], CONTRACT["ordered_check_sets"][-1]["scope"])

    def test_current_packaged_report_matches_every_required_id_in_order(self) -> None:
        report = json.loads(REPORT.read_text(encoding="utf-8"))
        result = validate_required_report_checks(CONTRACT, report.get("checks"))
        self.assertTrue(result["passed"], result)

    def test_same_count_with_replacement_fails_and_reordering_is_semantically_equal(self) -> None:
        report = json.loads(REPORT.read_text(encoding="utf-8"))
        checks = report["checks"]
        replaced = dict(checks)
        first = next(iter(replaced))
        value = replaced.pop(first)
        replaced["unexpectedReplacement"] = value
        replacement_result = validate_required_report_checks(CONTRACT, replaced)
        self.assertFalse(replacement_result["passed"])
        self.assertEqual(replacement_result["actual_count"], EXPECTED_CHECK_COUNT)
        self.assertEqual(replacement_result["missing"], (first,))
        self.assertEqual(replacement_result["unexpected"], ("unexpectedReplacement",))

        reversed_result = validate_required_report_checks(
            CONTRACT, dict(reversed(list(checks.items())))
        )
        self.assertTrue(reversed_result["passed"])
        self.assertFalse(reversed_result["order_matches"])

    def test_duplicate_or_unknown_manifest_checks_fail_closed(self) -> None:
        duplicate = copy.deepcopy(CONTRACT)
        duplicate["ordered_check_sets"][1]["required_check_ids"].append(
            duplicate["ordered_check_sets"][0]["required_check_ids"][0]
        )
        with self.assertRaisesRegex(PackagedAcceptanceContractError, "globally unique"):
            validate_packaged_acceptance_contract(duplicate)

        unknown = copy.deepcopy(CONTRACT)
        unknown["phase2_release_required_check_ids"].append("notInFullContract")
        with self.assertRaisesRegex(PackagedAcceptanceContractError, "not in the full contract"):
            validate_packaged_acceptance_contract(unknown)

    def test_receipt_binding_requires_current_manifest_identity_and_hash(self) -> None:
        receipt = json.loads(
            (ROOT / "validation/results/v247_cumulative_native_acceptance_receipt.json").read_text(
                encoding="utf-8-sig"
            )
        )
        historical = {**receipt, "schema_version": 1}
        self.assertFalse(
            receipt_binds_packaged_acceptance_contract(historical),
            "the historical schema-v1 receipt must not satisfy the V2 manifest contract",
        )
        self.assertTrue(
            receipt_binds_packaged_acceptance_contract(receipt),
            "the fresh cumulative receipt must bind the current V2 manifest contract",
        )
        current = {
            **receipt,
            "schema_version": 2,
            "acceptance_contract": {
                "path": "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
                "contract_id": CONTRACT["contract_id"],
                "contract_version": CONTRACT["contract_version"],
                "required_check_count": EXPECTED_CHECK_COUNT,
                "sha256": hashlib.sha256(
                    (ROOT / "validation/capabilities/packaged_windows_acceptance_v2.manifest.json").read_bytes()
                ).hexdigest(),
            },
        }
        self.assertTrue(receipt_binds_packaged_acceptance_contract(current))
        current["acceptance_contract"]["sha256"] = "0" * 64
        self.assertFalse(receipt_binds_packaged_acceptance_contract(current))

    def test_every_cumulative_release_identity_binds_the_contract_sources(self) -> None:
        for method in CONTRACT_BOUND_METHOD_MANIFESTS:
            with self.subTest(method=method):
                manifest = json.loads(
                    (ROOT / "validation/methods" / f"{method}.manifest.json").read_text(
                        encoding="utf-8"
                    )
                )
                sources = manifest["qualification"]["source_requirements"]
                for role in ("packaged_acceptance", "method_audit"):
                    self.assertTrue(
                        CONTRACT_SOURCES.issubset(sources[role]),
                        f"{method} {role} does not bind the packaged acceptance contract",
                    )


if __name__ == "__main__":
    unittest.main()
