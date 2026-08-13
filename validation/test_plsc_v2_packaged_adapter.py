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

from plsc_v2_packaged_acceptance import EXPECTED_CUMULATIVE_CHECKS, ROOT, verify_native_report


STARTED = datetime(2026, 8, 13, 12, 0, tzinfo=timezone.utc)
RUN_ID = "plsc-run"
EXPORT = "validation/results/test-plsc.xlsx"


def valid_report() -> dict:
    zero = {"recipeCount": 0, "resultCount": 0, "runCount": 0, "recipeIds": [], "resultIds": [], "runIds": []}
    checks = {
        "runtime": {"tauriRuntime": True},
        "plscInvalidSetup": {"attempted": True, "startEnabled": False, "blockers": ["Consistent PLS requires at least two indicators per construct"], "underspecifiedReflectiveBlocker": True, "archiveBefore": copy.deepcopy(zero), "archiveAfter": copy.deepcopy(zero), "runStateUnchanged": True, "resultCreated": False},
        "plscDialog": {"selectedMethod": "Consistent PLS", "startEnabled": True, "blockers": []},
        "plscProgress": {"completedRunProof": {"runId": RUN_ID, "matched": True}},
        "plscResult": {"runId": RUN_ID, "runLabel": "Consistent PLS run", "reliabilityRows": 2, "correlationRows": 1, "recordedSeedLabel": 0},
        "plscExport": {"selectedRunId": RUN_ID, "expectedRunId": RUN_ID, "xlsxEnabled": True, "nativeXlsx": {"attempted": True, "targetPath": str((ROOT / EXPORT).resolve()), "file": {"isFile": True, "size": 123}, "workbookSheets": ["PLSc correction reliability", "PLSc construct correlations", "Run provenance"], "methodSheetsPresentExactlyOnce": True, "helper": {"completion": {"passed": True, "workbook": {"sha256": "a" * 64, "size": 123, "requiredSharedStrings": ["rho_A"]}}}}},
        "plscSaveReopen": {"sameRunRestored": True, "expectedRunId": RUN_ID, "selectedRunId": RUN_ID, "reliabilityRows": 2, "correlationRows": 1, "immutableLabelsRestored": True},
    }
    for index in range(EXPECTED_CUMULATIVE_CHECKS - len(checks)):
        checks[f"other_{index:03d}"] = {"passed": True}
    return {"generatedAt": "2026-08-13T12:01:00Z", "passed": True, "focusedRun": {"scope": "regression_bootstrap", "completedAt": "2026-08-13T12:02:00Z"}, "checks": checks, "failures": [], "consoleErrors": []}


def verify(document: dict) -> dict:
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "report.json"
        path.write_text(json.dumps(document), encoding="utf-8")
        with patch("plsc_v2_packaged_acceptance.RAW_REPORT", path):
            return verify_native_report(STARTED, {"plsc_export": {"path": EXPORT, "sha256": "a" * 64, "size": 123}})


class PlscPackagedAdapterTests(unittest.TestCase):
    def test_exact_contract_passes(self) -> None:
        result = verify(valid_report())
        self.assertTrue(result["passed"], result["checks"])

    def test_identity_and_boundary_mutations_fail_closed(self) -> None:
        mutations = {
            "invalid_started": (("checks", "plscInvalidSetup", "startEnabled"), True, "invalid_scope_blocked"),
            "invalid_created_result": (("checks", "plscInvalidSetup", "resultCreated"), True, "invalid_scope_created_no_state"),
            "export_other_run": (("checks", "plscExport", "selectedRunId"), "other", "export_bound_to_exact_run"),
            "xlsx_missing_method_sheet": (("checks", "plscExport", "nativeXlsx", "workbookSheets"), ["Run provenance"], "xlsx_method_identity"),
            "reopen_other_run": (("checks", "plscSaveReopen", "selectedRunId"), "other", "same_run_reopened"),
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
