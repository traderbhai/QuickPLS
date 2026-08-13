from __future__ import annotations

import copy
import json
import tempfile
import unittest
import sys
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from wpls_v1_packaged_acceptance import EXPECTED_CUMULATIVE_CHECKS, ROOT, verify_native_report


STARTED = datetime(2026, 8, 13, 12, 0, tzinfo=timezone.utc)
RUN_ID = "wpls-run"
EXPORT = "validation/results/test-wpls.xlsx"


def valid_report() -> dict:
    zero = {"recipeCount": 0, "resultCount": 0, "runCount": 0, "recipeIds": [], "resultIds": [], "runIds": []}
    checks = {
        "runtime": {"tauriRuntime": True},
        "wplsInvalidSetup": {"attempted": True, "caseWeightColumn": "", "startEnabled": False, "blockers": ["Choose a positive numeric case-weight variable"], "missingWeightBlocker": True, "archiveBefore": copy.deepcopy(zero), "archiveAfter": copy.deepcopy(zero), "runStateUnchanged": True, "resultCreated": False},
        "wplsDialog": {"selectedMethod": "Weighted PLS", "caseWeightColumn": "case_wt", "standardized": "Standardized (fixed)", "startEnabled": True, "blockers": []},
        "wplsProgress": {"completedRunProof": {"runId": RUN_ID, "matched": True}},
        "wplsResult": {"runId": RUN_ID, "runLabel": "Weighted PLS run", "pathRows": 1},
        "wpls_weights": {"rows": 4, "caseWeightColumnVisible": True},
        "wplsExport": {"selectedRunId": RUN_ID, "expectedRunId": RUN_ID, "xlsxEnabled": True, "nativeXlsx": {"attempted": True, "targetPath": str((ROOT / EXPORT).resolve()), "file": {"isFile": True, "size": 123}, "workbookSheets": ["WPLS case-weight diagnostics", "Run provenance"], "methodSheetsPresentExactlyOnce": True, "helper": {"completion": {"passed": True, "workbook": {"sha256": "b" * 64, "size": 123, "requiredSharedStrings": ["case_wt"]}}}}},
        "wplsSaveReopen": {"sameRunRestored": True, "expectedRunId": RUN_ID, "selectedRunId": RUN_ID, "pathRows": 1, "diagnosticRows": 4, "immutableLabelsRestored": True, "caseWeightColumnRestored": True},
    }
    for index in range(EXPECTED_CUMULATIVE_CHECKS - len(checks)):
        checks[f"other_{index:03d}"] = {"passed": True}
    return {"generatedAt": "2026-08-13T12:01:00Z", "passed": True, "focusedRun": {"scope": "regression_bootstrap", "completedAt": "2026-08-13T12:02:00Z"}, "checks": checks, "failures": [], "consoleErrors": []}


def verify(document: dict) -> dict:
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "report.json"
        path.write_text(json.dumps(document), encoding="utf-8")
        with patch("wpls_v1_packaged_acceptance.RAW_REPORT", path):
            return verify_native_report(STARTED, {"wpls_export": {"path": EXPORT, "sha256": "b" * 64, "size": 123}})


class WplsPackagedAdapterTests(unittest.TestCase):
    def test_exact_contract_passes(self) -> None:
        result = verify(valid_report())
        self.assertTrue(result["passed"], result["checks"])

    def test_identity_and_boundary_mutations_fail_closed(self) -> None:
        mutations = {
            "weight_selected_in_invalid_setup": (("checks", "wplsInvalidSetup", "caseWeightColumn"), "case_wt", "invalid_weight_blocked"),
            "invalid_created_result": (("checks", "wplsInvalidSetup", "resultCreated"), True, "invalid_weight_created_no_state"),
            "valid_weight_drift": (("checks", "wplsDialog", "caseWeightColumn"), "other", "valid_setup_selected"),
            "export_other_run": (("checks", "wplsExport", "selectedRunId"), "other", "export_bound_to_exact_run"),
            "xlsx_missing_method_sheet": (("checks", "wplsExport", "nativeXlsx", "workbookSheets"), ["Run provenance"], "xlsx_method_identity"),
            "reopen_other_run": (("checks", "wplsSaveReopen", "selectedRunId"), "other", "same_run_reopened"),
        }
        for name, (keys, value, expected) in mutations.items():
            with self.subTest(name=name):
                document = valid_report(); target = document
                for key in keys[:-1]: target = target[key]
                target[keys[-1]] = value
                result = verify(document)
                self.assertFalse(result["passed"])
                self.assertFalse(result["checks"][expected])


if __name__ == "__main__":
    unittest.main()
