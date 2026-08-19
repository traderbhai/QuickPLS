from __future__ import annotations

import unittest

from validation.general_sem_rank0_standard_promotion import (
    RANK0_CELLS,
    evaluate_cell_reports,
)


def _qualification(definition, *, ready: bool = True):
    return {
        "passed": ready,
        "qualification_ready": ready,
        "capability_id": definition.capability_id,
        "cell_id": definition.cell_id,
        "method_version": definition.capability_version,
        "receipt_payload_contract_id": "quickpls.general_sem.rank0.receipt_payload.v1",
        "receipt_payload_contract_verified": ready,
        "errors": [] if ready else ["missing receipt"],
    }


def _manifest(definition, *, ready: bool = True):
    return {
        "passed": ready,
        "feature_id": definition.cell_id,
        "method_version": definition.capability_version,
        "declared_state": "release_qualified" if ready else "engine_only",
        "derived_state": "release_qualified" if ready else "engine_only",
        "target_state": "release_qualified",
        "errors": [] if ready else ["missing release evidence"],
    }


def _registry(*, standard: bool):
    return {
        "coverage_state": "partial",
        "evidence_state": "release_qualified" if standard else "engine_only",
        "surface": "standard" if standard else "labs",
    }


class Rank0StandardPromotionTests(unittest.TestCase):
    def test_each_cell_can_become_an_independent_promotion_candidate(self) -> None:
        for definition in RANK0_CELLS:
            with self.subTest(cell=definition.cell_id):
                report = evaluate_cell_reports(
                    definition,
                    registry_cell=_registry(standard=False),
                    qualification_report=_qualification(definition),
                    manifest_report=_manifest(definition),
                )
                self.assertEqual(report["state"], "promotion_candidate")
                self.assertTrue(report["evidence_ready"])
                self.assertFalse(report["registry_standard"])
                self.assertEqual(report["errors"], [])

    def test_standard_requires_current_qualification_and_manifest_evidence(self) -> None:
        definition = RANK0_CELLS[1]
        report = evaluate_cell_reports(
            definition,
            registry_cell=_registry(standard=True),
            qualification_report=_qualification(definition, ready=False),
            manifest_report=_manifest(definition, ready=False),
        )
        self.assertEqual(report["state"], "blocked")
        self.assertFalse(report["evidence_ready"])
        self.assertTrue(report["registry_standard"])
        self.assertTrue(any("over-promotion" in error for error in report["errors"]))

    def test_failed_bootstrap_does_not_change_qualified_point_cell_state(self) -> None:
        point = RANK0_CELLS[0]
        bootstrap = RANK0_CELLS[1]
        point_report = evaluate_cell_reports(
            point,
            registry_cell=_registry(standard=True),
            qualification_report=_qualification(point),
            manifest_report=_manifest(point),
        )
        bootstrap_report = evaluate_cell_reports(
            bootstrap,
            registry_cell=_registry(standard=False),
            qualification_report=_qualification(bootstrap, ready=False),
            manifest_report=_manifest(bootstrap, ready=False),
        )
        self.assertEqual(point_report["state"], "standard_active")
        self.assertEqual(bootstrap_report["state"], "blocked")

    def test_identity_drift_fails_closed(self) -> None:
        definition = RANK0_CELLS[3]
        qualification = _qualification(definition)
        qualification["cell_id"] = "wrong.cell"
        report = evaluate_cell_reports(
            definition,
            registry_cell=_registry(standard=False),
            qualification_report=qualification,
            manifest_report=_manifest(definition),
        )
        self.assertEqual(report["state"], "blocked")
        self.assertTrue(any("cell_id mismatch" in error for error in report["errors"]))

    def test_rank0_promotion_refuses_legacy_hash_only_receipts(self) -> None:
        definition = RANK0_CELLS[0]
        qualification = _qualification(definition)
        qualification["receipt_payload_contract_id"] = None
        qualification["receipt_payload_contract_verified"] = False
        report = evaluate_cell_reports(
            definition,
            registry_cell=_registry(standard=False),
            qualification_report=qualification,
            manifest_report=_manifest(definition),
        )
        self.assertEqual(report["state"], "blocked")
        self.assertTrue(any("strict Rank 0" in error for error in report["errors"]))


if __name__ == "__main__":
    unittest.main()
