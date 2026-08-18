from __future__ import annotations

import sys
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from evidence_truth_reconciliation_v1 import (  # noqa: E402
    EXPECTED_FINAL_CELL_EVIDENCE_COUNTS,
    EXPECTED_FINAL_ROW_EVIDENCE_COUNTS,
    RECONCILED_MANIFEST_NAMES,
    build_reconciliation_plan,
)
from method_promotion_manifest import validate_all  # noqa: E402


class EvidenceTruthReconciliationV1Tests(unittest.TestCase):
    def test_current_repository_is_idempotently_reconciled(self) -> None:
        plan = build_reconciliation_plan(ROOT)

        self.assertTrue(plan["passed"], plan["errors"])
        self.assertEqual(
            plan["after_evidence_counts"]["rows"],
            EXPECTED_FINAL_ROW_EVIDENCE_COUNTS,
        )
        self.assertEqual(
            plan["after_evidence_counts"]["cells"],
            EXPECTED_FINAL_CELL_EVIDENCE_COUNTS,
        )
        self.assertEqual(plan["manifest_changes"], [])
        self.assertEqual(plan["cell_changes"], [])
        self.assertEqual(plan["claim_language_change_count"], 0)
        self.assertEqual(plan["legacy_evidence_pointer_clear_count"], 0)
        self.assertEqual(plan["archive_requalification_methods"], [])
        self.assertFalse(plan["historical_artifacts_modified"])

    def test_micom_manifest_is_not_in_the_write_set(self) -> None:
        self.assertNotIn(
            "micom_permutation_mga_v3.manifest.json",
            RECONCILED_MANIFEST_NAMES,
        )

    def test_all_live_method_manifests_now_validate(self) -> None:
        report = validate_all(repository_root=ROOT)

        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["manifest_count"], 40)
        self.assertTrue(all(item["passed"] for item in report["manifests"]))


if __name__ == "__main__":
    unittest.main()
