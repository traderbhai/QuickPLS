from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import ols_v1_packaged_acceptance as adapter  # noqa: E402


RUN_ID = "ols-run"
NOW = datetime(2026, 8, 14, 1, 0, tzinfo=timezone.utc)


def valid_report(project: Path, workbook: Path, screenshots: list[Path]) -> dict:
    expected_sheets = [
        "Coefficients",
        "Model fit",
        "Calculation scope",
        "Fitted values and residuals",
        "Run provenance",
    ]
    payload = workbook.read_bytes()
    import hashlib

    workbook_row = {"size": len(payload), "sha256": hashlib.sha256(payload).hexdigest()}
    return {
        "generatedAt": "2026-08-14T01:00:00Z",
        "passed": True,
        "focusedRun": {"scope": "ols", "completedAt": "2026-08-14T01:01:00Z"},
        "failures": [],
        "consoleErrors": [],
        "screenshots": [str(path.resolve()) for path in screenshots],
        "checks": {
            "olsDialog": {
                "catalogCount": 18,
                "selectedMethod": "Regression",
                "outcome": "y",
                "selectedPredictors": ["x", "m"],
                "selectedControls": ["z"],
                "startEnabled": True,
                "unsupportedControls": 0,
                "blockers": [],
            },
            "olsProgress": {"captured": False, "completedBeforeCapture": True},
            "olsResult": {
                "runId": RUN_ID,
                "initialSelectedTable": "ols_coefficients",
                "coefficients": {"rows": 4},
                "fit": {"rows": 1},
                "scope": {"rows": 12},
                "scopeValues": {
                    "Method version": adapter.METHOD_VERSION,
                    "Validated scope": adapter.OLS_VALIDATED_SCOPE,
                },
            },
            "olsExport": {
                "nativeXlsx": {
                    "attempted": True,
                    "targetPath": str(workbook.resolve()),
                    "file": {"isFile": True},
                    "workbookSheets": expected_sheets,
                    "helper": {
                        "completion": {
                            "passed": True,
                            "workbook": {
                                "sheetNames": expected_sheets,
                                "size": workbook_row["size"],
                                "sha256": workbook_row["sha256"],
                            },
                        }
                    },
                }
            },
            "olsSaveReopen": {
                "sameRunRestored": True,
                "expectedRunId": RUN_ID,
                "selectedRunId": RUN_ID,
                "archive": {
                    "provenanceMethodVersion": adapter.METHOD_VERSION,
                    "regressionMethodVersion": adapter.METHOD_VERSION,
                    "payloadKind": "pls_pm_v1",
                    "coefficientContract": True,
                    "predictionContract": True,
                    "fitContract": True,
                    "recipe": {
                        "status": "validated_regression_ols_v1_bounded_scope",
                        "robustSe": "hc3",
                        "bootstrapSamples": 0,
                    },
                },
            },
            "olsFixture": {"projectPath": str(project.resolve())},
            "olsFunctionalOffline": {
                "passed": True,
                "analyticalWorkflowRequiresInternet": False,
                "strictZeroProcessEgressClaimed": False,
                "platformBackgroundEgressOutsidePageRequestScope": True,
                "observedRequestCount": 7,
                "externalRequestCount": 0,
                "origins": ["http://ipc.localhost", "http://tauri.localhost"],
                "externalRequests": [],
            },
        },
    }


class OlsV1PackagedAcceptanceTests(unittest.TestCase):
    def test_focused_harness_emits_ols_scoped_offline_evidence(self):
        source = (VALIDATION / "v247_tauri_native_acceptance.mjs").read_text(
            encoding="utf-8"
        )
        start = source.index("async function runFocusedOlsAcceptance()")
        end = source.index("async function runFocusedLogisticAcceptance()", start)
        ols_flow = source[start:end]
        self.assertIn("evidence.checks.olsFunctionalOffline = {", ols_flow)
        self.assertIn("passed: olsExternalRequests.length === 0", ols_flow)
        self.assertIn("externalRequestCount: olsExternalRequests.length", ols_flow)
        self.assertIn("platformBackgroundEgressOutsidePageRequestScope: true", ols_flow)
        self.assertIn("strictZeroProcessEgressClaimed: false", ols_flow)

    def test_current_cumulative_contract_count_is_manifest_derived(self):
        expected = sum(
            len(check_set["required_check_ids"])
            for check_set in adapter.PACKAGED_ACCEPTANCE_CONTRACT["ordered_check_sets"]
        )
        self.assertEqual(adapter.EXPECTED_CUMULATIVE_CHECKS, expected)

    def test_cumulative_cleanup_requires_exact_current_check_count(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            results = root / "validation/results"
            results.mkdir(parents=True)
            report = results / "v247_tauri_native_acceptance.json"
            exported = results / "ols.xlsx"
            receipt_path = results / "receipt.json"
            report.write_text(
                json.dumps(
                    {
                        "checks": {
                            check_id: {"passed": True}
                            for check_id in adapter.PACKAGED_ACCEPTANCE_CONTRACT[
                                "ordered_check_sets"
                            ][0]["required_check_ids"]
                        }
                    }
                ),
                encoding="utf-8",
            )
            report_document = json.loads(report.read_text(encoding="utf-8"))
            report_document["checks"] = {
                check_id: {"passed": True}
                for check_set in adapter.PACKAGED_ACCEPTANCE_CONTRACT["ordered_check_sets"]
                for check_id in check_set["required_check_ids"]
            }
            report.write_text(json.dumps(report_document), encoding="utf-8")
            exported.write_bytes(b"ols-xlsx")
            receipt = {
                "schema_version": 2,
                "kind": "quickpls_v247_cumulative_native_acceptance_receipt",
                "passed": True,
                "supervisor_started_at_utc": "2026-08-14T01:00:00Z",
                "completed_at_utc": "2026-08-14T01:01:00Z",
                "report": report.relative_to(root).as_posix(),
                "checks": adapter.EXPECTED_CUMULATIVE_CHECKS,
                "unique_checks": adapter.EXPECTED_CUMULATIVE_CHECKS,
                "failures": 0,
                "console_errors": 0,
                "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
                "report_size": report.stat().st_size,
                "final_scope": "regression_bootstrap",
                "graceful_process_cleanup_verified": True,
                "acceptance_contract": {
                    "path": "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
                    "contract_id": adapter.PACKAGED_ACCEPTANCE_CONTRACT["contract_id"],
                    "contract_version": adapter.PACKAGED_ACCEPTANCE_CONTRACT["contract_version"],
                    "required_check_count": adapter.EXPECTED_CUMULATIVE_CHECKS,
                    "sha256": adapter.CONTRACT_FILE_SHA256,
                },
                "exports": [
                    {
                        "role": "ols",
                        "path": exported.relative_to(root).as_posix(),
                        "size": exported.stat().st_size,
                        "sha256": hashlib.sha256(exported.read_bytes()).hexdigest(),
                    }
                ],
            }

            def evaluate(document: dict):
                receipt_path.write_text(json.dumps(document), encoding="utf-8")
                with patch.object(adapter, "ROOT", root), patch.object(
                    adapter, "CUMULATIVE_RECEIPT", receipt_path
                ):
                    return adapter.cumulative_cleanup(NOW)

            detail, artifacts = evaluate(receipt)
            self.assertTrue(detail["passed"])
            self.assertEqual(len(artifacts), 3)

            receipt["unique_checks"] = adapter.EXPECTED_CUMULATIVE_CHECKS - 1
            with self.assertRaisesRegex(adapter.AdapterError, "packaged acceptance manifest"):
                evaluate(receipt)

    def fixture(self):
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        project = root / "ols.qpls"
        workbook = root / "ols.xlsx"
        project.write_bytes(b"QPLS")
        workbook.write_bytes(b"XLSX")
        screens = []
        for number, state in (
            (120, "fixture-data"),
            (121, "dialog"),
            (122, "running"),
            (123, "results"),
            (124, "export"),
            (125, "reopened"),
        ):
            path = root / f"{number}-tauri-native-ols-{state}-1536x794.png"
            path.write_bytes(b"PNG" + bytes([number]))
            screens.append(path)
        return temporary, project, workbook, screens

    def test_exact_packaged_contract_passes(self):
        temporary, project, workbook, screens = self.fixture()
        with temporary:
            document = valid_report(project, workbook, screens)
            report_path = project.parent / "scoped.json"
            report_path.write_text(json.dumps(document), encoding="utf-8")
            with patch.object(adapter, "ROOT", project.parent), patch.object(
                adapter, "SCOPED_REPORT", report_path
            ):
                # Re-root screenshot construction to the fixture hierarchy.
                expected = project.parent / "validation/results/screens/v247-native-desktop-acceptance"
                expected.mkdir(parents=True)
                for source in screens:
                    (expected / source.name).write_bytes(source.read_bytes())
                document["screenshots"] = [str((expected / source.name).resolve()) for source in screens]
                result, _, completed = adapter.packaged_contract(document)
            self.assertTrue(result["passed"])
            self.assertEqual(completed, datetime(2026, 8, 14, 1, 1, tzinfo=timezone.utc))

            document["checks"]["olsFunctionalOffline"]["observedRequestCount"] = 0
            document["checks"]["olsFunctionalOffline"]["origins"] = []
            with patch.object(adapter, "ROOT", project.parent), patch.object(
                adapter, "SCOPED_REPORT", report_path
            ):
                zero_request_result, _, _ = adapter.packaged_contract(document)
            self.assertTrue(zero_request_result["passed"])

    def test_identity_and_scope_mutations_fail_closed(self):
        temporary, project, workbook, screens = self.fixture()
        with temporary:
            baseline = valid_report(project, workbook, screens)
            root = project.parent
            expected = root / "validation/results/screens/v247-native-desktop-acceptance"
            expected.mkdir(parents=True)
            for source in screens:
                (expected / source.name).write_bytes(source.read_bytes())
            baseline["screenshots"] = [str((expected / source.name).resolve()) for source in screens]
            mutations = {
                "other_method": (("checks", "olsDialog", "selectedMethod"), "PROCESS"),
                "other_run": (("checks", "olsSaveReopen", "selectedRunId"), "other"),
                "other_version": (("checks", "olsSaveReopen", "archive", "regressionMethodVersion"), "other"),
                "altered_validated_scope": (("checks", "olsResult", "scopeValues", "Validated scope"), "Raw numeric OLS."),
                "negative_observed_requests": (("checks", "olsFunctionalOffline", "observedRequestCount"), -1),
                "external_request_count": (("checks", "olsFunctionalOffline", "externalRequestCount"), 1),
                "external_request_ledger": (("checks", "olsFunctionalOffline", "externalRequests"), [{"origin": "https://example.test"}]),
                "zero_egress_overclaim": (("checks", "olsFunctionalOffline", "strictZeroProcessEgressClaimed"), True),
                "platform_scope_overclaim": (("checks", "olsFunctionalOffline", "platformBackgroundEgressOutsidePageRequestScope"), False),
            }
            for name, (path, value) in mutations.items():
                with self.subTest(name=name):
                    document = copy.deepcopy(baseline)
                    target = document
                    for key in path[:-1]:
                        target = target[key]
                    target[path[-1]] = value
                    with patch.object(adapter, "ROOT", root):
                        with self.assertRaises(adapter.AdapterError):
                            adapter.packaged_contract(document)

    def test_source_freshness_compares_receipt_content_not_mtime_metadata(self):
        temporary = tempfile.TemporaryDirectory()
        with temporary:
            root = Path(temporary.name)
            desktop = root / "target/release/quickpls-desktop.exe"
            cli = root / "target/release/qpls.exe"
            receipt_path = root / "validation/results/receipt.json"
            source = root / "crates/qpls-cli/src/main.rs"
            for path, payload in (
                (desktop, b"desktop"),
                (source, b"source"),
                (cli, b"cli"),
            ):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(payload)
            receipt_path.parent.mkdir(parents=True, exist_ok=True)
            import hashlib

            receipt = {
                "tested_desktop": {
                    "path": "target/release/quickpls-desktop.exe",
                    "size": len(b"desktop"),
                    "sha256": hashlib.sha256(b"desktop").hexdigest(),
                    "mtime_ns": desktop.stat().st_mtime_ns,
                },
                "build_finished_at_utc": "2026-08-14T00:00:00Z",
            }
            receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
            with (
                patch.object(adapter, "ROOT", root),
                patch.object(adapter, "BUILD_RECEIPT", receipt_path),
                patch.object(adapter, "validate_build_receipt"),
                patch.object(adapter, "cli_source_paths", return_value=["crates/qpls-cli/src/main.rs"]),
            ):
                fresh, _ = adapter.source_freshness()
            self.assertTrue(fresh["desktop_receipt_exact"])

            receipt["tested_desktop"]["sha256"] = "0" * 64
            receipt_path.write_text(json.dumps(receipt), encoding="utf-8")
            with (
                patch.object(adapter, "ROOT", root),
                patch.object(adapter, "BUILD_RECEIPT", receipt_path),
                patch.object(adapter, "validate_build_receipt"),
                patch.object(adapter, "cli_source_paths", return_value=["crates/qpls-cli/src/main.rs"]),
            ):
                with self.assertRaises(adapter.AdapterError):
                    adapter.source_freshness()


if __name__ == "__main__":
    unittest.main()
