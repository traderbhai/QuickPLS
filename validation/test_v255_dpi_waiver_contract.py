from __future__ import annotations

import copy
import json
import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from v255_named_evidence_collector import validate_candidate_report  # noqa: E402
from v255_named_evidence_verifier import validate_index_shape  # noqa: E402
from v255_release_waiver import (  # noqa: E402
    DPI_WAIVER_CASE_ID,
    DPI_WAIVER_CONTRACT,
    DPI_WAIVER_EXPECTED,
    DPI_WAIVER_MANIFEST_DECLARATION,
    DPI_WAIVER_METADATA,
    exact_release_waiver_receipt,
    exact_waived_observation,
)


def load(relative: str) -> dict:
    return json.loads((ROOT / relative).read_text(encoding="utf-8"))


class V255DpiWaiverContractTests(unittest.TestCase):
    def setUp(self) -> None:
        self.matrix = load("validation/v255_method_evidence_matrix.json")
        self.index = load("validation/v255_named_evidence_index.json")
        self.cross = load("validation/v255_cross_method_case_manifest.json")

    def test_frozen_case_sets_and_source_verifier_remain_intact(self) -> None:
        expected, entries, failures, checks = validate_index_shape(
            self.matrix, self.index, publication=False
        )
        self.assertEqual(55, len(expected))
        self.assertEqual(55, len(entries))
        self.assertEqual(29, sum(row["scope"] == "cross_method" for row in expected))
        self.assertEqual([], failures)
        self.assertTrue(all(checks.values()))
        self.assertEqual(17, len(self.cross["cases"]))

    def test_only_exact_dpi_case_has_the_approved_waiver(self) -> None:
        self.assertEqual(
            DPI_WAIVER_CONTRACT,
            self.index["collector_contract"]["approved_release_waiver"],
        )
        waived_cases = [case for case in self.cross["cases"] if "approved_waiver" in case]
        self.assertEqual(1, len(waived_cases))
        case = waived_cases[0]
        self.assertEqual(DPI_WAIVER_CASE_ID, case["id"])
        self.assertEqual(DPI_WAIVER_EXPECTED, case["expected"])
        self.assertEqual(DPI_WAIVER_METADATA, case["approved_waiver"])
        manifest = load("validation/v255_evidence_bundle_manifest.json")
        self.assertEqual(
            DPI_WAIVER_MANIFEST_DECLARATION,
            manifest["named_evidence"]["approved_release_waiver"],
        )

    def test_exact_waiver_stays_false_and_retains_observed_reality(self) -> None:
        observed = {
            "effective_dpi": 120,
            "device_pixel_ratio": 1.25,
            "clean_profile": True,
            "forced_scale_argument_present": False,
        }
        observation = {
            "schema_version": 1,
            "case_id": DPI_WAIVER_CASE_ID,
            "operation": "exercise_accessibility",
            "status": "waived",
            "waiver": DPI_WAIVER_METADATA,
            "assertion": {
                "id": f"exercise_accessibility:{DPI_WAIVER_CASE_ID}",
                "passed": False,
                "expected": DPI_WAIVER_EXPECTED,
                "observed": observed,
            },
            "screenshot": {"path": "evidence.png", "sha256": "0" * 64},
        }
        receipt = {
            "case_id": DPI_WAIVER_CASE_ID,
            "status": "waived",
            "assertion_passed": False,
            **DPI_WAIVER_METADATA,
            "expected": DPI_WAIVER_EXPECTED,
            "observed": observed,
        }
        self.assertTrue(exact_waived_observation(observation))
        self.assertTrue(exact_release_waiver_receipt(receipt))
        for field, replacement in (
            ("case_id", "cross_method:accessibility:1024x700"),
            ("status", "passed"),
        ):
            mutated = copy.deepcopy(observation)
            mutated[field] = replacement
            self.assertFalse(exact_waived_observation(mutated))
        mutated = copy.deepcopy(observation)
        mutated["assertion"]["passed"] = True
        self.assertFalse(exact_waived_observation(mutated))
        mutated = copy.deepcopy(observation)
        mutated["assertion"]["observed"] = DPI_WAIVER_EXPECTED
        self.assertFalse(exact_waived_observation(mutated))

    def test_opt_in_switch_is_explicit_and_default_dpi_gate_remains(self) -> None:
        cross_wrapper = (ROOT / "validation/run_v255_cross_method_candidate_smoke.ps1").read_text(encoding="utf-8")
        installed_wrapper = (ROOT / "validation/run_v255_installed_portable_smoke.ps1").read_text(encoding="utf-8")
        driver = (ROOT / "validation/v255_cross_method_candidate_driver.mjs").read_text(encoding="utf-8")
        self.assertIn("[switch]$WaiveActualWindows200PercentScaling", cross_wrapper)
        self.assertIn("[switch]$WaiveActualWindows200PercentScaling", installed_wrapper)
        self.assertIn("$effectiveDpi -ne 192", cross_wrapper)
        self.assertIn("Add-WaivedDpiObservation", cross_wrapper)
        self.assertIn('passed = $false', cross_wrapper)
        self.assertIn('"--waive-actual-windows-200-percent-scaling", "true"', cross_wrapper)
        self.assertIn('"waive-actual-windows-200-percent-scaling"', driver)
        self.assertIn("device_pixel_ratio === 2", driver)
        dpi_arguments = next(
            line for line in cross_wrapper.splitlines() if '$dpiBrowserArgs = ' in line
        )
        self.assertNotIn("force-device-scale-factor", dpi_arguments)

    def test_candidate_waiver_state_is_explicit_and_fail_closed(self) -> None:
        observed = {
            "effective_dpi": 120,
            "device_pixel_ratio": 1.25,
            "clean_profile": True,
            "forced_scale_argument_present": False,
        }
        receipt = {
            "case_id": DPI_WAIVER_CASE_ID,
            "status": "waived",
            "assertion_passed": False,
            **DPI_WAIVER_METADATA,
            "expected": DPI_WAIVER_EXPECTED,
            "observed": observed,
        }
        commit = "a" * 40
        candidate = {
            "schema_version": 3,
            "suite_id": "quickpls_v255_installed_portable_smoke_v3",
            "target_release": "2.55.0",
            "passed": True,
            "source_worktree_clean": True,
            "named_evidence_stage": "source",
            "named_evidence_verified": True,
            "candidate_build_source_commit": commit,
            "qualification_status": "passed_with_waiver",
            "release_waivers": [receipt],
            "outcomes": [
                {
                    "name": name,
                    "status": "passed",
                    "executable_sha256": digit * 64,
                    "product_version": "2.55.0",
                    "build_source_commit": commit,
                }
                for name, digit in (("portable", "1"), ("installed", "2"))
            ],
        }
        source_commit, outcomes = validate_candidate_report(candidate)
        self.assertEqual(commit, source_commit)
        self.assertEqual({"portable", "installed"}, set(outcomes))
        normal = copy.deepcopy(candidate)
        normal["qualification_status"] = "passed"
        normal["release_waivers"] = []
        self.assertEqual(commit, validate_candidate_report(normal)[0])
        for field, value in (
            ("qualification_status", "passed"),
            ("release_waivers", []),
        ):
            mutated = copy.deepcopy(candidate)
            mutated[field] = value
            with self.assertRaises(ValueError):
                validate_candidate_report(mutated)


if __name__ == "__main__":
    unittest.main()
