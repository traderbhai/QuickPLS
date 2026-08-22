from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from v255_product_completion_audit import (  # noqa: E402
    final_named_evidence_report_checks,
)
from v255_release_waiver import (  # noqa: E402
    DPI_WAIVER_CASE_ID,
    DPI_WAIVER_EXPECTED,
    DPI_WAIVER_METADATA,
    exact_case_waiver_receipt_matches_observation,
    exact_cross_report_waiver_binding,
    exact_release_waiver_matches_observation,
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def waiver_fixture() -> tuple[dict, dict, dict, dict]:
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
        "screenshot": {"path": "actual.png", "sha256": "1" * 64},
    }
    release_receipt = {
        "case_id": DPI_WAIVER_CASE_ID,
        "status": "waived",
        "assertion_passed": False,
        **DPI_WAIVER_METADATA,
        "expected": DPI_WAIVER_EXPECTED,
        "observed": observed,
    }
    cross_report = {
        "schema_version": 1,
        "suite_id": "quickpls_v255_cross_method_candidate_wrapper_v1",
        "target_release": "2.55.0",
        "passed": True,
        "qualification_status": "passed_with_waiver",
        "release_waivers": [release_receipt],
        "dpi": {
            "requirement_status": "waived",
            "effective_dpi": 120,
            "required_dpi": 192,
            "device_pixel_ratio": 1.25,
            "display_settings_changed": False,
            "forced_scale_argument_present": False,
            "profile_was_fresh": True,
        },
    }
    case_receipt = {
        "status": "waived",
        "case_id": DPI_WAIVER_CASE_ID,
        "operation": "exercise_accessibility",
        "waiver": DPI_WAIVER_METADATA,
        "assertion": observation["assertion"],
        "screenshot": {"member": "named/dpi.png", "sha256": "1" * 64},
    }
    return observation, release_receipt, cross_report, case_receipt


class V255WaiverProvenanceTests(unittest.TestCase):
    def test_every_waiver_layer_must_describe_the_same_observation(self) -> None:
        observation, release_receipt, cross_report, case_receipt = waiver_fixture()
        self.assertTrue(
            exact_release_waiver_matches_observation(release_receipt, observation)
        )
        self.assertTrue(exact_cross_report_waiver_binding(cross_report, observation))
        self.assertTrue(
            exact_case_waiver_receipt_matches_observation(case_receipt, observation)
        )

        mismatched_release = copy.deepcopy(release_receipt)
        mismatched_release["observed"]["effective_dpi"] = 144
        self.assertFalse(
            exact_release_waiver_matches_observation(
                mismatched_release, observation
            )
        )
        mismatched_cross = copy.deepcopy(cross_report)
        mismatched_cross["dpi"]["device_pixel_ratio"] = 1.5
        self.assertFalse(
            exact_cross_report_waiver_binding(mismatched_cross, observation)
        )
        mismatched_case = copy.deepcopy(case_receipt)
        mismatched_case["screenshot"]["sha256"] = "2" * 64
        self.assertFalse(
            exact_case_waiver_receipt_matches_observation(
                mismatched_case, observation
            )
        )

    def test_final_audit_accepts_strict_or_exact_waived_population(self) -> None:
        case_ids = {DPI_WAIVER_CASE_ID} | {f"case:{index}" for index in range(54)}
        for waived in (False, True):
            with self.subTest(waived=waived), tempfile.TemporaryDirectory() as raw:
                directory = Path(raw)
                bundle = directory / "evidence.zip"
                bundle.write_bytes(b"bound bundle")
                report_path = directory / "named.json"
                installed_path = directory / "installed.json"
                cases = [
                    {
                        "id": case_id,
                        "status": (
                            "waived"
                            if waived and case_id == DPI_WAIVER_CASE_ID
                            else "passed"
                        ),
                        "checks": {"bound": True},
                    }
                    for case_id in sorted(case_ids)
                ]
                report = {
                    "schema_version": 1,
                    "suite_id": "quickpls_v255_named_evidence_verifier_v1",
                    "target_release": "2.55.0",
                    "stage": "publication",
                    "passed": True,
                    "failures": [],
                    "checks": {"complete": True},
                    "sources": {
                        "matrix_sha256": "a" * 64,
                        "index_sha256": "b" * 64,
                        "bundle_manifest_sha256": "c" * 64,
                        "evidence_bundle_sha256": sha256(bundle),
                    },
                    "cases": cases,
                    "summary": {
                        "required": 55,
                        "cross_method_required": 29,
                        "specialized_result_required": 26,
                        "verified": 54 if waived else 55,
                        "waived": 1 if waived else 0,
                        "pending": 0,
                    },
                }
                report_path.write_text(json.dumps(report), encoding="utf-8")
                observation, release_receipt, _, _ = waiver_fixture()
                self.assertTrue(observation)
                installed = {
                    "qualification_status": (
                        "passed_with_waiver" if waived else "passed"
                    ),
                    "release_waivers": [release_receipt] if waived else [],
                    "named_evidence_stage": "publication",
                    "named_evidence_verified": True,
                    "named_evidence_report_sha256": sha256(report_path),
                    "evidence_bundle_sha256": sha256(bundle),
                }
                installed_path.write_text(json.dumps(installed), encoding="utf-8")
                checks, _ = final_named_evidence_report_checks(
                    report_path,
                    installed_path,
                    bundle,
                    case_ids,
                    "a" * 64,
                    "b" * 64,
                    "c" * 64,
                )
                self.assertTrue(all(checks.values()), checks)


if __name__ == "__main__":
    unittest.main()
