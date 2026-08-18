from __future__ import annotations

import copy
import hashlib
import json
import sys
import tempfile
import unittest
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from unittest.mock import patch


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import pls_bootstrap_v4_packaged_acceptance as packaged  # noqa: E402


def baseline_report() -> dict:
    run_id = "bootstrap-run"
    viewports = [
        {"id": value, "passed": True}
        for value in ("1024x700", "1280x720", "1440x900")
    ]
    return {
        "passed": True,
        "generatedAt": "2026-08-13T16:00:00Z",
        "focusedRun": {"scope": "regression_bootstrap", "completedAt": "2026-08-13T16:01:00Z"},
        "failures": [],
        "consoleErrors": [],
        "checks": {
            "runtime": {"tauriRuntime": True},
            "bootstrapInvalidSetup": {
                "attempted": True, "startEnabled": False, "blockers": ["model required"],
                "runStateUnchanged": True, "resultCreated": False,
                "archiveBefore": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
            },
            "mediationBootstrapDialog": {
                "selectedMethod": "PLS-SEM Bootstrapping", "bootstrapSamples": "100",
                "studentizedInnerSamples": "0", "confidenceLevel": "95", "seed": "20260718",
                "workers": "1", "startEnabled": True, "blockers": [],
            },
            "mediationBootstrapResult": {
                "runId": run_id, "runLabel": "PLS-SEM Bootstrapping run",
                "navigation": {"bootstrapTreeItems": 1, "rowCounts": {"Aggregate mediation effects bootstrap inference": 6}},
            },
            "mediationExport": {"bootstrap": {
                "selectedRunId": run_id,
                "nativeXlsx": {
                    "attempted": True, "selectedRunId": run_id, "targetPath": "x.xlsx",
                    "file": {"isFile": True, "size": 20},
                    "helper": {"completion": {"passed": True, "workbook": {"sheetNames": ["Bootstrapping", "Run provenance"]}}},
                },
            }},
            "mediationSaveReopen": {"hasBootstrap": True, "selectedRunId": run_id, "expectedBootstrapRunId": run_id},
            "bootstrapCancellationRetry": {
                "passed": True, "cancelledPartialRunVisible": 0, "completedRetryRunId": run_id,
                "exportedRunId": run_id, "reopenedRunId": run_id,
            },
            "bootstrapResponsiveViewports": {"passed": True, "setup": viewports, "results": copy.deepcopy(viewports)},
            "bootstrapFunctionalOffline": {
                "passed": True, "analyticalWorkflowRequiresInternet": False,
                "strictZeroProcessEgressClaimed": False, "externalRequestCount": 0,
                "observedRequestCount": 12,
            },
        },
    }


class BootstrapV4PackagedAdapterTests(unittest.TestCase):
    def test_current_cumulative_contract_count_is_manifest_derived(self) -> None:
        expected = sum(
            len(check_set["required_check_ids"])
            for check_set in packaged.PACKAGED_ACCEPTANCE_CONTRACT["ordered_check_sets"]
        )
        self.assertEqual(packaged.EXPECTED_CUMULATIVE_CHECKS, expected)

    def test_archive_requires_exactly_one_resampling_provenance_token(self) -> None:
        setup = {
            "bootstrapSamples": "100",
            "studentizedInnerSamples": "0",
            "confidenceLevel": "95",
            "seed": "20260718",
            "workers": "1",
        }
        run_id = "bootstrap-run"

        def write_archive(path: Path, provenance_version: str) -> None:
            project = {
                "results": [
                    {
                        "id": run_id,
                        "provenance": {
                            "recipe_id": "bootstrap-recipe",
                            "method_version": provenance_version,
                        },
                        "payload": {
                            "kind": "pls_pm_v3",
                            "bootstrap": {
                                "method_version": "indexed_resampling_v4",
                                "plan": {
                                    "replicates": 100,
                                    "master_seed": 20260718,
                                },
                                "usable_replicates": 100,
                                "failed_replicates": [],
                            },
                        },
                    }
                ],
                "recipes": [
                    {
                        "id": "bootstrap-recipe",
                        "method_config": {"kind": "pls_bootstrap"},
                        "settings": {
                            "bootstrap_samples": 100,
                            "studentized_inner_samples": 0,
                            "seed": 20260718,
                            "workers": 1,
                            "confidence_level": 0.95,
                        },
                    }
                ],
                "layouts": {
                    "workspace": {
                        "runs": [
                            {
                                "id": run_id,
                                "status": "completed",
                                "method": "PLS-SEM Bootstrapping",
                            }
                        ]
                    }
                },
            }
            project_bytes = json.dumps(project, sort_keys=True).encode("utf-8")
            manifest = {
                "schema_version": 3,
                "checksums": {
                    "project.json": hashlib.sha256(project_bytes).hexdigest()
                },
            }
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("project.json", project_bytes)
                archive.writestr("manifest.json", json.dumps(manifest))

        versions = {
            "trailing_htmt": (
                "pls_pm_v1+indexed_resampling_v4+ringle_et_al_htmt_plus_v1",
                True,
            ),
            "missing": ("pls_pm_v1+ringle_et_al_htmt_plus_v1", False),
            "duplicate": (
                "pls_pm_v1+indexed_resampling_v4+indexed_resampling_v4"
                "+ringle_et_al_htmt_plus_v1",
                False,
            ),
        }
        with tempfile.TemporaryDirectory() as directory:
            for name, (version, expected) in versions.items():
                with self.subTest(name=name):
                    path = Path(directory) / f"{name}.qpls"
                    write_archive(path, version)
                    inspected = packaged.inspect_bootstrap_archive(path, run_id, setup)
                    self.assertEqual(
                        inspected["checks"]["provenance_version"], expected
                    )
                    self.assertEqual(inspected["passed"], expected)

    def _receipt_fixture(self):
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        not_before = datetime(2026, 8, 13, 16, 0, tzinfo=timezone.utc)
        workbook = root / "validation/results/v247-native-pls-bootstrap-v4-reuse.xlsx"
        workbook.parent.mkdir(parents=True, exist_ok=True)
        workbook.write_bytes(b"exact Bootstrap v4 workbook")
        workbook_sha = hashlib.sha256(workbook.read_bytes()).hexdigest()
        checks = {
            check_id: {}
            for check_set in packaged.PACKAGED_ACCEPTANCE_CONTRACT["ordered_check_sets"]
            for check_id in check_set["required_check_ids"]
        }
        checks["mediationExport"] = {
            "bootstrap": {
                "nativeXlsx": {
                    "targetPath": str(workbook),
                    "helper": {
                        "completion": {
                            "workbook": {
                                "path": str(workbook),
                                "size": workbook.stat().st_size,
                                "sha256": workbook_sha,
                            }
                        }
                    }
                }
            }
        }
        report_document = {
            "passed": True,
            "focusedRun": {
                "scope": "regression_bootstrap",
                "completedAt": (not_before + timedelta(seconds=1))
                .isoformat()
                .replace("+00:00", "Z"),
            },
            "checks": checks,
            "failures": [],
            "consoleErrors": [],
        }
        report = root / "validation/results/v247_tauri_native_acceptance.json"
        report.write_text(json.dumps(report_document, sort_keys=True), encoding="utf-8")
        receipt_document = {
            "schema_version": 2,
            "kind": "quickpls_v247_cumulative_native_acceptance_receipt",
            "passed": True,
            "supervisor_started_at_utc": (not_before - timedelta(seconds=1))
            .isoformat()
            .replace("+00:00", "Z"),
            "completed_at_utc": (not_before + timedelta(seconds=2))
            .isoformat()
            .replace("+00:00", "Z"),
            "report": "validation/results/v247_tauri_native_acceptance.json",
            "checks": packaged.EXPECTED_CUMULATIVE_CHECKS,
            "unique_checks": packaged.EXPECTED_CUMULATIVE_CHECKS,
            "failures": 0,
            "console_errors": 0,
            "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
            "report_size": report.stat().st_size,
            "final_scope": "regression_bootstrap",
            "graceful_process_cleanup_verified": True,
            "acceptance_contract": {
                "path": "validation/capabilities/packaged_windows_acceptance_v2.manifest.json",
                "contract_id": packaged.PACKAGED_ACCEPTANCE_CONTRACT["contract_id"],
                "contract_version": packaged.PACKAGED_ACCEPTANCE_CONTRACT["contract_version"],
                "required_check_count": packaged.EXPECTED_CUMULATIVE_CHECKS,
                "sha256": packaged.CONTRACT_FILE_SHA256,
            },
            "exports": [
                {
                    "role": "bootstrap",
                    "path": workbook.relative_to(root).as_posix(),
                    "size": workbook.stat().st_size,
                    "sha256": workbook_sha,
                }
            ],
        }
        receipt = root / "validation/results/v247_cumulative_native_acceptance_receipt.json"
        receipt.write_text(json.dumps(receipt_document), encoding="utf-8")
        return temporary, root, not_before, report, report_document, receipt, receipt_document

    def _verify_receipt(
        self,
        *,
        receipt_mutation=None,
        report_mutation=None,
        refresh_report_bytes=False,
    ):
        (
            temporary,
            root,
            not_before,
            report,
            report_document,
            receipt,
            receipt_document,
        ) = self._receipt_fixture()
        with temporary:
            if report_mutation is not None:
                report_mutation(report_document)
                report.write_text(
                    json.dumps(report_document, sort_keys=True), encoding="utf-8"
                )
            if refresh_report_bytes:
                receipt_document["report_sha256"] = hashlib.sha256(
                    report.read_bytes()
                ).hexdigest()
                receipt_document["report_size"] = report.stat().st_size
            if receipt_mutation is not None:
                receipt_mutation(receipt_document)
            receipt.write_text(json.dumps(receipt_document), encoding="utf-8")
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "RAW_REPORT", report),
                patch.object(packaged, "CUMULATIVE_RECEIPT", receipt),
            ):
                return packaged.verify_reusable_cumulative_receipt(not_before)

    def test_complete_report_passes(self) -> None:
        result = packaged.evaluate_native_report(
            baseline_report(), datetime(2026, 8, 13, 15, tzinfo=timezone.utc),
            cumulative_wrapper_passed=True,
        )
        self.assertTrue(result["passed"], json.dumps(result, indent=2))

    def test_each_release_boundary_mutation_fails_closed(self) -> None:
        mutations = {
            "stale_chain": (("generatedAt",), "2026-08-13T14:00:00Z", "fresh_cumulative_chain"),
            "invalid_setup_created_result": (("checks", "bootstrapInvalidSetup", "resultCreated"), True, "invalid_setup_blocked_without_run"),
            "wrong_setup": (("checks", "mediationBootstrapDialog", "bootstrapSamples"), "99", "exact_valid_setup"),
            "wrong_method": (("checks", "mediationBootstrapResult", "runLabel"), "PLS-SEM Algorithm run", "completed_bootstrap_result"),
            "wrong_export_run": (("checks", "mediationExport", "bootstrap", "nativeXlsx", "selectedRunId"), "other", "selected_run_native_xlsx"),
            "wrong_reopen_run": (("checks", "mediationSaveReopen", "selectedRunId"), "other", "same_run_reopened"),
            "cancelled_partial_result": (("checks", "bootstrapCancellationRetry", "cancelledPartialRunVisible"), 1, "cancel_retry_identity"),
            "missing_viewport": (("checks", "bootstrapResponsiveViewports", "results"), [{"id": "1440x900", "passed": True}], "exact_three_responsive_viewports"),
            "external_request": (("checks", "bootstrapFunctionalOffline", "externalRequestCount"), 1, "functional_offline_without_zero_egress_overclaim"),
            "zero_egress_overclaim": (("checks", "bootstrapFunctionalOffline", "strictZeroProcessEgressClaimed"), True, "functional_offline_without_zero_egress_overclaim"),
        }
        for name, (path, value, failed_check) in mutations.items():
            with self.subTest(name=name):
                report = baseline_report()
                target = report
                for key in path[:-1]:
                    target = target[key]
                target[path[-1]] = value
                result = packaged.evaluate_native_report(
                    report, datetime(2026, 8, 13, 15, tzinfo=timezone.utc),
                    cumulative_wrapper_passed=True,
                )
                self.assertFalse(result["passed"])
                self.assertFalse(result["checks"][failed_check])

    def test_wrapper_success_is_not_inferred_from_a_green_json(self) -> None:
        result = packaged.evaluate_native_report(
            baseline_report(), datetime(2026, 8, 13, 15, tzinfo=timezone.utc),
            cumulative_wrapper_passed=False,
        )
        self.assertFalse(result["passed"])
        self.assertFalse(result["checks"]["fresh_cumulative_chain"])

    def test_release_cli_source_closure_includes_development_slices(self) -> None:
        self.assertIn("validation/development_slices.json", packaged.cli_source_paths())

    def test_exact_cumulative_receipt_is_reusable(self) -> None:
        result = self._verify_receipt()
        self.assertTrue(result["passed"], result)
        self.assertTrue(all(result["checks"].values()), result)

    def test_cumulative_receipt_mutations_fail_closed(self) -> None:
        mutations = {
            "start_too_old": (
                lambda receipt: receipt.__setitem__(
                    "supervisor_started_at_utc", "2026-08-13T15:59:57Z"
                ),
                None,
                False,
                "start_within_two_second_reuse_tolerance",
            ),
            "completion_too_old": (
                lambda receipt: receipt.__setitem__(
                    "completed_at_utc", "2026-08-13T15:59:59Z"
                ),
                None,
                False,
                "completed_after_reuse_boundary",
            ),
            "report_sha_drift": (
                lambda receipt: receipt.__setitem__("report_sha256", "0" * 64),
                None,
                False,
                "exact_report_bytes",
            ),
            "report_size_drift": (
                lambda receipt: receipt.__setitem__("report_size", 1),
                None,
                False,
                "exact_report_bytes",
            ),
            "receipt_count_drift": (
                lambda receipt: receipt.__setitem__(
                    "unique_checks", packaged.EXPECTED_CUMULATIVE_CHECKS - 1
                ),
                None,
                False,
                "exact_required_checks",
            ),
            "actual_count_drift": (
                None,
                lambda report: report["checks"].pop("ncaReferenceFixture"),
                True,
                "exact_required_checks",
            ),
            "final_scope_drift": (
                lambda receipt: receipt.__setitem__("final_scope", "pca"),
                None,
                False,
                "final_scope_regression_bootstrap",
            ),
            "cleanup_not_verified": (
                lambda receipt: receipt.__setitem__(
                    "graceful_process_cleanup_verified", False
                ),
                None,
                False,
                "graceful_cleanup_verified",
            ),
            "bootstrap_role_hash_drift": (
                lambda receipt: receipt["exports"][0].__setitem__(
                    "sha256", "0" * 64
                ),
                None,
                False,
                "bootstrap_export_exact_bytes",
            ),
            "report_workbook_hash_drift": (
                None,
                lambda report: report["checks"]["mediationExport"]["bootstrap"]
                ["nativeXlsx"]["helper"]["completion"]["workbook"].__setitem__(
                    "sha256", "f" * 64
                ),
                True,
                "bootstrap_export_bound_to_report",
            ),
        }
        for name, (
            receipt_mutation,
            report_mutation,
            refresh_report_bytes,
            expected_check,
        ) in mutations.items():
            with self.subTest(name=name):
                result = self._verify_receipt(
                    receipt_mutation=receipt_mutation,
                    report_mutation=report_mutation,
                    refresh_report_bytes=refresh_report_bytes,
                )
                self.assertFalse(result["passed"], result)
                self.assertFalse(result["checks"][expected_check], result)

    def test_receipt_validation_failure_is_fail_closed(self) -> None:
        with patch.object(packaged, "validate_build_receipt", side_effect=packaged.SourceManifestFailure("receipt drift")):
            result = packaged.source_freshness()
        self.assertFalse(result["passed"])
        self.assertFalse(result["desktop_receipt_exact"])
        self.assertIn("receipt drift", result["error"])

    def test_manifest_declares_exact_release_roles_and_sources(self) -> None:
        document = packaged.manifest()
        release = document["qualification"]["evidence"]["release_qualified"]
        roles = sorted(role for artifact in release for role in artifact["roles"])
        self.assertEqual(roles, ["method_audit", "packaged_acceptance"])
        requirements = document["qualification"]["source_requirements"]["packaged_acceptance"]
        self.assertIn(packaged.SOURCE, requirements)
        self.assertIn("validation/run_v247_cumulative_native_acceptance.ps1", requirements)
        self.assertIn("validation/v247_tauri_native_acceptance.mjs", requirements)


if __name__ == "__main__":
    unittest.main()
