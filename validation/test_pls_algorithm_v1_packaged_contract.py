from __future__ import annotations

import copy
import json
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

from pls_algorithm_v1_packaged_acceptance import verify_native_report


STARTED = datetime(2026, 8, 13, 11, 59, tzinfo=timezone.utc)
REQUIRED_TITLES = [
    "Direct effects",
    "Specific indirect effects",
    "Total indirect effects",
    "Total effects",
]


def valid_report() -> dict:
    empty_archive = {
        "recipeCount": 0,
        "resultCount": 0,
        "runCount": 0,
        "recipeIds": [],
        "resultIds": [],
        "runIds": [],
    }
    return {
        "generatedAt": "2026-08-13T12:00:00Z",
        "passed": True,
        "focusedRun": None,
        "checks": {
            "runtime": {
                "tauriRuntime": True,
                "viewport": {"width": 1440, "height": 900},
            },
            "mediationPlsDialog": {
                "selectedMethod": "PLS-SEM Algorithm",
                "startEnabled": True,
                "blockers": [],
            },
            "plsAlgorithmInvalidSetup": {
                "attempted": True,
                "startEnabled": False,
                "blockers": ["Create at least one construct."],
                "archiveBefore": copy.deepcopy(empty_archive),
                "archiveAfter": copy.deepcopy(empty_archive),
                "runStateUnchanged": True,
                "resultCreated": False,
            },
            "mediationPlsResult": {
                "runId": "algorithm-run",
                "runLabel": "PLS-SEM Algorithm run",
                "navigation": {
                    "requiredTitles": REQUIRED_TITLES,
                    "rowCounts": {title: 1 for title in REQUIRED_TITLES},
                    "bootstrapTreeItems": 0,
                },
            },
            "mediationBootstrapResult": {
                "runId": "bootstrap-run",
                "runLabel": "PLS-SEM Bootstrapping run",
            },
            "mediationExport": {
                "selectedRunId": "algorithm-run",
                "xlsxEnabled": True,
                "nativeXlsx": {
                    "helper": {
                        "completion": {
                            "passed": True,
                            "workbook": {
                                "sheetNames": ["Run provenance"],
                                "path": "algorithm.xlsx",
                                "sha256": "a" * 64,
                            },
                        }
                    }
                },
                "bootstrap": {
                    "selectedRunId": "bootstrap-run",
                    "xlsxEnabled": True,
                },
            },
            "mediationSaveReopen": {
                "hasPlsAlgorithm": True,
                "hasBootstrap": True,
                "selectedPlsRunId": "algorithm-run",
                "selectedRunId": "bootstrap-run",
                "runOptions": [
                    "PLS-SEM Algorithm run",
                    "PLS-SEM Bootstrapping run",
                ],
            },
        },
        "failures": [],
        "consoleErrors": [],
    }


def verify(document: dict) -> dict:
    with tempfile.TemporaryDirectory() as directory:
        report_path = Path(directory) / "native.json"
        report_path.write_text(json.dumps(document), encoding="utf-8")
        with patch(
            "pls_algorithm_v1_packaged_acceptance.RAW_REPORT", report_path
        ):
            return verify_native_report(STARTED, cumulative_wrapper_passed=True)


class PlsAlgorithmPackagedContractTests(unittest.TestCase):
    def test_valid_contract_passes(self) -> None:
        result = verify(valid_report())
        self.assertTrue(result["passed"], result["checks"])

    def test_exact_run_and_invalid_archive_mutations_fail_closed(self) -> None:
        mutations = {
            "algorithm_export_replaced_by_bootstrap": (
                ("checks", "mediationExport", "selectedRunId"),
                "bootstrap-run",
                "export_is_same_pls_algorithm_run",
            ),
            "reopened_algorithm_replaced_by_bootstrap": (
                ("checks", "mediationSaveReopen", "selectedPlsRunId"),
                "bootstrap-run",
                "reopen_selects_same_pls_algorithm_run",
            ),
            "invalid_setup_created_result": (
                ("checks", "plsAlgorithmInvalidSetup", "resultCreated"),
                True,
                "invalid_pls_setup_is_blocked",
            ),
            "invalid_archive_gained_result": (
                (
                    "checks",
                    "plsAlgorithmInvalidSetup",
                    "archiveAfter",
                    "resultCount",
                ),
                1,
                "invalid_pls_archive_has_no_run",
            ),
            "bootstrap_export_replaced_by_algorithm": (
                ("checks", "mediationExport", "bootstrap", "selectedRunId"),
                "algorithm-run",
                "bootstrap_evidence_preserved",
            ),
        }
        for name, (path, value, expected_red_check) in mutations.items():
            with self.subTest(name=name):
                document = valid_report()
                target = document
                for key in path[:-1]:
                    target = target[key]
                target[path[-1]] = value
                result = verify(document)
                self.assertFalse(result["passed"])
                self.assertFalse(result["checks"][expected_red_check])


if __name__ == "__main__":
    unittest.main()
