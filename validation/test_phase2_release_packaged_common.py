from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
import zipfile
from dataclasses import replace
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch

from validation import phase2_release_packaged_common as adapter
from validation.method_promotion_manifest import validate_manifest


ROOT = Path(__file__).resolve().parents[1]


class Phase2ReleasePackagedSourceTests(unittest.TestCase):
    def test_gsca_runner_defaults_to_the_new_append_only_scoped_receipt(self) -> None:
        source = (ROOT / "validation/run_v247_gsca_native_acceptance.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("v247_gsca_scoped_native_acceptance_receipt_v2.json", source)
        self.assertNotIn('"validation\\results\\v247_gsca_scoped_native_acceptance_receipt.json"', source)

    def test_nca_runner_defaults_to_the_new_append_only_scoped_receipt(self) -> None:
        source = (ROOT / "validation/run_v247_nca_native_acceptance.ps1").read_text(encoding="utf-8-sig")
        self.assertIn("v247_nca_scoped_native_acceptance_receipt_v2.json", source)
        self.assertNotIn('"validation\\results\\v247_nca_scoped_native_acceptance_receipt.json"', source)

    def test_exact_six_method_contract_and_fourteen_new_check_union(self) -> None:
        self.assertEqual(
            set(adapter.METHODS),
            {
                "gsca_als_v2",
                "cca_residuals_v1",
                "ipma_v1",
                "cbsem_ml_v1",
                "plspredict_cvpat_v2",
                "nca_v2",
            },
        )
        self.assertEqual(adapter.EXPECTED_CHECK_COUNT, len(adapter.EXPECTED_CHECK_IDS))
        invalid = {contract.invalid_key for contract in adapter.METHODS.values()}
        cancellation = {
            contract.cancellation_key
            for contract in adapter.METHODS.values()
            if contract.cancellation_key is not None
        }
        viewports = {f"{contract.visual_key}PackagedViewports" for contract in adapter.METHODS.values()}
        self.assertEqual(
            invalid | cancellation | viewports,
            {
                "gscaInvalidSetup",
                "ccaInvalidSetup",
                "ipmaInvalidSetup",
                "cbsemInvalidSetup",
                "predictionInvalidSetup",
                "ncaInvalidSetup",
                "predictionCancellationRetry",
                "ncaCancellationRetry",
                "gscaPackagedViewports",
                "ccaPackagedViewports",
                "ipmaPackagedViewports",
                "cbsemPackagedViewports",
                "predictionPackagedViewports",
                "ncaPackagedViewports",
            },
        )
        self.assertEqual(
            {slug for slug, contract in adapter.METHODS.items() if contract.cancellation_key},
            {"plspredict_cvpat_v2", "nca_v2"},
        )

    def test_reviewed_manifests_freeze_truthful_declared_contract(self) -> None:
        expected_declared_states = {
            "gsca_als_v2": "release_qualified",
            "cca_residuals_v1": "native_qualified",
            "ipma_v1": "native_qualified",
            "cbsem_ml_v1": "absent",
            "plspredict_cvpat_v2": "absent",
            "nca_v2": "release_qualified",
        }
        for slug, contract in adapter.METHODS.items():
            with self.subTest(slug=slug):
                path = ROOT / f"validation/methods/{slug}.manifest.json"
                document = json.loads(path.read_text(encoding="utf-8-sig"))
                qualification = document["qualification"]
                expected_state = expected_declared_states[slug]
                self.assertEqual(qualification["declared_state"], expected_state)
                self.assertEqual(qualification["target_state"], "release_qualified")
                release = qualification["evidence"]["release_qualified"]
                if slug in {"gsca_als_v2", "nca_v2"}:
                    self.assertEqual(len(release), 2)
                    self.assertEqual(
                        {tuple(row["roles"]) for row in release},
                        {("packaged_acceptance",), ("method_audit",)},
                    )
                    self.assertEqual(
                        {row["path"] for row in release},
                        {
                            f"validation/results/method_factory/{slug}/packaged_acceptance.identity.json",
                            f"validation/results/method_factory/{slug}/method_audit.identity.json",
                        },
                    )
                else:
                    self.assertEqual(release, [])
                sources = qualification["source_requirements"]
                self.assertIn("validation/run_phase2_release_packaged_closure.ps1", sources["packaged_acceptance"])
                self.assertIn("validation/test_phase2_release_packaged_common.py", sources["packaged_acceptance"])
                self.assertIn("validation/test_v247_cumulative_native_acceptance_supervisor.py", sources["packaged_acceptance"])
                self.assertNotIn("validation/v247_native_desktop_visual_acceptance.mjs", sources["packaged_acceptance"])
                self.assertIn("validation/test_phase2_release_packaged_common.py", sources["method_audit"])
                self.assertIn("validation/phase2_release_packaged_common.py", sources["packaged_acceptance"])
                self.assertIn("validation/phase2_release_packaged_common.py", sources["method_audit"])
                self.assertIn("validation/packaged_windows_acceptance_v2.py", sources["packaged_acceptance"])
                self.assertIn("validation/packaged_windows_acceptance_v2.py", sources["method_audit"])
                self.assertIn("validation/capabilities/packaged_windows_acceptance_v2.manifest.json", sources["packaged_acceptance"])
                self.assertIn("validation/capabilities/packaged_windows_acceptance_v2.manifest.json", sources["method_audit"])
                result = validate_manifest(path, ROOT, verify_evidence=False)
                self.assertTrue(result["passed"], result["errors"])
                self.assertEqual(result["derived_state"], expected_state)

    def test_closure_script_runs_each_adapter_once_from_manifest_receipt_floor(self) -> None:
        source = (ROOT / "validation/run_phase2_release_packaged_closure.ps1").read_text(encoding="utf-8")
        self.assertNotIn(f"[int]$receipt.checks -ne {adapter.EXPECTED_CHECK_COUNT}", source)
        self.assertIn("$expectedCheckCount", source)
        self.assertIn("$receipt.acceptance_contract.sha256 -ne $acceptanceContractSha256", source)
        self.assertIn('$receipt.acceptance_contract.bundled_sample_catalog.path -ne "src/data/bundledSampleProjects.v1.json"', source)
        self.assertIn("$receipt.acceptance_contract.bundled_sample_catalog.size -ne $bundledSampleCatalogSize", source)
        self.assertIn("$receipt.acceptance_contract.bundled_sample_catalog.sha256 -ne $bundledSampleCatalogSha256", source)
        self.assertIn("$notBeforeUtc = [string]$receipt.supervisor_started_at_utc", source)
        for contract in adapter.METHODS.values():
            self.assertEqual(source.count(f'Script = "{contract.adapter_script}"'), 1)
        self.assertIn("--not-before-utc $notBeforeUtc", source)
        self.assertNotIn('declared_state = "release_qualified"', source)
        self.assertIn("already declare release_qualified", source)

    def test_cbsem_adapter_does_not_substitute_preview_or_synthetic_packaged_viewports(self) -> None:
        source = (ROOT / "validation/cbsem_ml_v1_packaged_adapter.py").read_text(encoding="utf-8")
        self.assertIn("packaged_viewport_contract", source)
        self.assertIn("EXPECTED_CHECK_COUNT", source)
        self.assertIn("validate_required_report_checks", source)
        self.assertNotIn("Chromium via Vite production preview", source)
        self.assertIn('visual_check["actual_tauri_window"] is True', source)
        self.assertIn('visual_check["viewport_emulation"] is False', source)
        self.assertIn('parser.add_argument("--not-before-utc", required=True)', source)
        self.assertIn("max(not_before, build_finished, harness_changed)", source)
        self.assertIn('"release_cli_source_closure"', source)
        self.assertIn("final_freshness != freshness", source)
        self.assertIn("return read_strict_archive(path)", source)
        self.assertIn('== "validation/results/v247_tauri_native_acceptance.json"', source)
        self.assertIn('report.get("runtime") != "tauri-webview2-cdp"', source)

    def test_shared_adapter_hash_binds_cli_source_closure_and_rechecks_it(self) -> None:
        source = (ROOT / "validation/phase2_release_packaged_common.py").read_text(encoding="utf-8")
        self.assertIn('"cli_source_closure": cli_source_rows', source)
        self.assertIn("evidence_floor = max(not_before, build_finished, harness_changed)", source)
        self.assertIn("final_fresh == fresh", source)
        self.assertIn("source descriptors changed during release audit", source)


class Phase2ReleasePackagedSemanticTests(unittest.TestCase):
    def test_functional_check_contract_accepts_only_exact_success_shapes(self) -> None:
        accepted = (
            True,
            {},
            {"kind": "evidence"},
            {"passed": True},
        )
        rejected = (
            False,
            None,
            1,
            0,
            "true",
            [],
            {"passed": False},
            {"passed": None},
        )
        for row in accepted:
            with self.subTest(row=row):
                self.assertTrue(adapter.functional_check_passed(row))
        for row in rejected:
            with self.subTest(row=row):
                self.assertFalse(adapter.functional_check_passed(row))

    def test_packaged_workflow_requires_each_method_specific_result_contract(self) -> None:
        for slug, contract in adapter.METHODS.items():
            with self.subTest(slug=slug):
                self.assertTrue(contract.exact_result_values)
                self.assertTrue(contract.minimum_result_values)
                self.assertEqual(contract.captures[1][1], "invalid-setup")
                self.assertEqual(contract.captures[-1][1], "reopened")
                self.assertEqual(contract.progress_kind in {"captured", "active"}, True)
                self.assertNotEqual(contract.dialog_key, contract.result_key)
                self.assertNotEqual(contract.result_key, contract.reopen_key)

    def test_invalid_setup_requires_exact_archive_equality(self) -> None:
        contract = adapter.METHODS["gsca_als_v2"]
        baseline = {
            contract.invalid_key: {
                "attempted": True,
                "selectedMethod": contract.selected_method,
                "startEnabled": False,
                "emptyModelBlocker": True,
                "archiveBefore": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
                "archiveAfter": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
                "archiveStateUnchanged": True,
                "resultCreated": False,
            }
        }
        self.assertTrue(adapter.invalid_setup_contract(contract, baseline)["passed"])
        for name, mutate in {
            "start_enabled": lambda row: row.update(startEnabled=True),
            "result_created": lambda row: row.update(resultCreated=True),
            "archive_mutated": lambda row: row["archiveAfter"].update(runCount=1),
            "wrong_method": lambda row: row.update(selectedMethod="Other"),
        }.items():
            with self.subTest(name=name):
                candidate = copy.deepcopy(baseline)
                mutate(candidate[contract.invalid_key])
                with self.assertRaises(adapter.AdapterError):
                    adapter.invalid_setup_contract(contract, candidate)

    def test_prediction_cancellation_binds_settings_cleanup_and_retry_run(self) -> None:
        contract = adapter.METHODS["plspredict_cvpat_v2"]
        run_id = "prediction-run"
        row = {
            "passed": True,
            "cancelledMethod": contract.selected_method,
            "cancelledSettings": {"plan": "10x10", "seed": 20260718},
            "activeLifecycleCaptured": True,
            "activeLifecycle": {"captured": True, "status": "running", "ariaBusy": "true"},
            "terminalMessage": "Calculation cancelled.",
            "noPartialVisibleResult": True,
            "noPartialCommittedResult": True,
            "archiveStateUnchanged": True,
            "archiveBefore": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
            "archiveAfter": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
            "retrySettings": {"selectedMethod": contract.selected_method, "plan": "10x10", "seed": 20260718},
            "retryEnabled": True,
            "completedRetryRunId": run_id,
        }
        checks = {contract.cancellation_key: row}
        self.assertTrue(adapter.cancellation_contract(contract, checks, run_id)["passed"])
        for path, value in (
            (("retrySettings", "seed"), 7),
            (("archiveAfter", "runCount"), 1),
            (("activeLifecycle", "captured"), False),
            (("archiveStateUnchanged",), False),
            (("completedRetryRunId",), "other-run"),
        ):
            candidate = copy.deepcopy(checks)
            target = candidate[contract.cancellation_key]
            for part in path[:-1]:
                target = target[part]
            target[path[-1]] = value
            with self.assertRaises(adapter.AdapterError):
                adapter.cancellation_contract(contract, candidate, run_id)

    def test_nca_cancellation_binds_all_requested_settings(self) -> None:
        contract = adapter.METHODS["nca_v2"]
        run_id = "nca-run"
        settings = {"x": "x", "y": "y", "ceiling": "both", "permutations": "999", "seed": "20260811"}
        row = {
            "passed": True,
            "cancelledMethod": contract.selected_method,
            "cancelledSettings": settings,
            "activeLifecycle": {"status": "running", "cancelVisible": True},
            "terminalMessage": "Calculation cancelled.",
            "noPartialVisibleResult": True,
            "noPartialCommittedResult": True,
            "archiveStateUnchanged": True,
            "archiveBefore": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
            "archiveAfter": {"recipeCount": 0, "resultCount": 0, "runCount": 0},
            "retrySettings": {"selectedMethod": contract.selected_method, **settings},
            "retryEnabled": True,
            "completedRetryRunId": run_id,
        }
        checks = {contract.cancellation_key: row}
        result = adapter.cancellation_contract(contract, checks, run_id)
        self.assertEqual(result["cancelled_settings"], settings)
        candidate = copy.deepcopy(checks)
        candidate[contract.cancellation_key]["retrySettings"]["permutations"] = "19"
        with self.assertRaises(adapter.AdapterError):
            adapter.cancellation_contract(contract, candidate, run_id)

    def test_archive_contract_binds_nca_retry_settings_to_completed_result(self) -> None:
        contract = adapter.METHODS["nca_v2"]
        cancellation = {
            "required": True,
            "cancelled_settings": {"x": "x", "y": "y", "ceiling": "both", "permutations": "999", "seed": "20260811"},
        }
        project = {
            "results": [{
                "id": "run-1",
                "status": "completed",
                "provenance": {"method": "nca", "method_version": "nca_v2", "recipe_id": "recipe-1"},
                "payload": {"kind": "pls_pm_v1", "estimation": {"method_version": "nca_v2", "nca": {"method_version": "nca_v2"}}},
            }],
            "recipes": [{
                "id": "recipe-1",
                "method_config": {"kind": "nca", "condition": "x", "outcome": "y", "ceiling": "both", "permutation_samples": 999},
                "settings": {"method": "nca", "seed": 20260811},
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "nca.qpls"
            project_bytes = json.dumps(project).encode("utf-8")
            archive_manifest = {"engine_version": "test", "checksums": {"project.json": hashlib.sha256(project_bytes).hexdigest()}}
            with zipfile.ZipFile(archive_path, "w") as archive:
                archive.writestr("project.json", project_bytes)
                archive.writestr("manifest.json", json.dumps(archive_manifest).encode("utf-8"))
            self.assertTrue(adapter.archive_contract(contract, archive_path, "run-1", cancellation)["passed"])
            cancellation["cancelled_settings"]["permutations"] = "19"
            with self.assertRaises(adapter.AdapterError):
                adapter.archive_contract(contract, archive_path, "run-1", cancellation)

    def test_archive_contract_binds_prediction_retry_plan_to_completed_result(self) -> None:
        contract = adapter.METHODS["plspredict_cvpat_v2"]
        cancellation = {
            "required": True,
            "cancelled_settings": {"plan": "10x10", "seed": 20260718},
        }
        project = {
            "results": [{
                "id": "run-1",
                "status": "completed",
                "provenance": {"method": "predict", "method_version": "plspredict_indicator_v2", "recipe_id": "recipe-1"},
                "payload": {"kind": "pls_pm_v1", "estimation": {
                    "method_version": "plspredict_indicator_v2",
                    "predict": {
                        "method_version": "plspredict_indicator_v2",
                        "repeated_kfold": {
                            "method_version": "plspredict_repeated_kfold_indicator_v2",
                            "folds": 10,
                            "repeats": 10,
                            "seed": 20260718,
                        },
                    },
                }},
            }],
            "recipes": [{
                "id": "recipe-1",
                "method_config": {"kind": "predict"},
                "settings": {"method": "predict", "seed": 20260718},
            }],
        }
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "prediction.qpls"

            def write_archive() -> None:
                project_bytes = json.dumps(project).encode("utf-8")
                archive_manifest = {"engine_version": "test", "checksums": {"project.json": hashlib.sha256(project_bytes).hexdigest()}}
                with zipfile.ZipFile(archive_path, "w") as archive:
                    archive.writestr("project.json", project_bytes)
                    archive.writestr("manifest.json", json.dumps(archive_manifest).encode("utf-8"))

            write_archive()
            self.assertTrue(adapter.archive_contract(contract, archive_path, "run-1", cancellation)["passed"])
            project["results"][0]["payload"]["estimation"]["predict"]["repeated_kfold"]["repeats"] = 9
            write_archive()
            with self.assertRaises(adapter.AdapterError):
                adapter.archive_contract(contract, archive_path, "run-1", cancellation)

    def test_cumulative_receipt_requires_exact_hash_and_manifest_check_ids(self) -> None:
        contract = adapter.METHODS["gsca_als_v2"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            report_path = root / "validation/results/v247_tauri_native_acceptance.json"
            workbook = root / "validation/results/gsca.xlsx"
            report_path.parent.mkdir(parents=True)
            workbook.write_bytes(b"xlsx")
            checks = {check_id: {"passed": True} for check_id in adapter.EXPECTED_CHECK_IDS}
            report = {
                "runtime": "tauri-webview2-cdp",
                "passed": True,
                "failures": [],
                "consoleErrors": [],
                "checks": checks,
                "focusedRun": {"scope": "regression_bootstrap", "completedAt": "2026-08-14T01:10:00Z"},
            }
            report_path.write_text(json.dumps(report), encoding="utf-8")
            report_bytes = report_path.read_bytes()
            receipt = {
                "schema_version": 2,
                "kind": "quickpls_v247_cumulative_native_acceptance_receipt",
                "passed": True,
                "supervisor_started_at_utc": "2026-08-14T01:00:00Z",
                "completed_at_utc": "2026-08-14T01:11:00Z",
                "report": "validation/results/v247_tauri_native_acceptance.json",
                "checks": adapter.EXPECTED_CHECK_COUNT,
                "unique_checks": adapter.EXPECTED_CHECK_COUNT,
                "failures": 0,
                "console_errors": 0,
                "report_sha256": hashlib.sha256(report_bytes).hexdigest(),
                "report_size": len(report_bytes),
                "final_scope": "regression_bootstrap",
                "graceful_process_cleanup_verified": True,
                "acceptance_contract": {
                    **adapter.packaged_acceptance_contract_descriptor(),
                    "contract_id": adapter.PACKAGED_ACCEPTANCE_CONTRACT["contract_id"],
                    "contract_version": adapter.PACKAGED_ACCEPTANCE_CONTRACT["contract_version"],
                    "required_check_count": adapter.EXPECTED_CHECK_COUNT,
                },
                "exports": [{"role": "gsca", "path": "validation/results/gsca.xlsx", "size": 4, "sha256": hashlib.sha256(b"xlsx").hexdigest()}],
            }
            floor = datetime(2026, 8, 14, 1, 0, tzinfo=timezone.utc)
            with patch.object(adapter, "ROOT", root):
                result, _, _ = adapter.validate_cumulative_receipt(contract, receipt, report, floor)
                self.assertTrue(result["exact_required_checks"])
                receipt["unique_checks"] = adapter.EXPECTED_CHECK_COUNT - 1
                with self.assertRaises(adapter.AdapterError):
                    adapter.validate_cumulative_receipt(contract, receipt, report, floor)

    def test_focused_gsca_receipt_binds_exact_checks_files_offline_and_cleanup(self) -> None:
        contract = adapter.METHODS["gsca_als_v2"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            results = root / "validation/results"
            desktop = root / "target/release/quickpls-desktop.exe"
            results.mkdir(parents=True)
            desktop.parent.mkdir(parents=True)
            desktop.write_bytes(b"desktop")
            workbook = results / "gsca.xlsx"
            project = results / "gsca.qpls"
            workbook.write_bytes(b"xlsx")
            project.write_bytes(b"project")
            screenshots = []
            for index in range(11):
                path = results / f"screen-{index:02}.png"
                path.write_bytes(f"screen-{index}".encode("ascii"))
                screenshots.append(str(path.resolve()))

            def row(path: Path) -> dict[str, object]:
                payload = path.read_bytes()
                return {
                    "path": path.relative_to(root).as_posix(),
                    "size": len(payload),
                    "sha256": hashlib.sha256(payload).hexdigest(),
                }

            checks = {check_id: {"passed": True} for check_id in adapter.GSCA_SCOPED_CHECK_IDS}
            offline = {
                "passed": True,
                "analyticalWorkflowRequiresInternet": False,
                "strictZeroProcessEgressClaimed": False,
                "platformBackgroundEgressOutsidePageRequestScope": True,
                "observedRequestCount": 7,
                "externalRequestCount": 0,
                "origins": ["http://tauri.localhost"],
                "externalRequests": [],
            }
            checks["gscaFunctionalOffline"] = offline
            checks["gscaExport"] = {"nativeXlsx": {"targetPath": str(workbook.resolve())}}
            checks["gscaFixture"] = {"projectPath": str(project.resolve())}
            report = {
                "runtime": "tauri-webview2-cdp",
                "passed": True,
                "failures": [],
                "consoleErrors": [],
                "focusedRun": {"scope": "gsca", "completedAt": "2026-08-16T01:01:00Z"},
                "checks": checks,
                "screenshots": screenshots,
                "screenshotArtifacts": [row(Path(path)) for path in screenshots],
            }
            report_path = results / "v247_tauri_native_acceptance_gsca.json"
            report_path.write_text(json.dumps(report), encoding="utf-8")
            receipt = {
                "schema_version": 1,
                "kind": "quickpls_v247_gsca_scoped_native_acceptance_receipt",
                "passed": True,
                "supervisor_started_at_utc": "2026-08-16T01:00:00Z",
                "completed_at_utc": "2026-08-16T01:02:00Z",
                "scope": "gsca",
                "feature_id": contract.feature_id,
                "method_version": contract.method_version,
                "report": row(report_path),
                "executable": row(desktop),
                "export": row(workbook),
                "project_archive": row(project),
                "screenshots": [row(Path(path)) for path in screenshots],
                "checks": len(adapter.GSCA_SCOPED_CHECK_IDS),
                "unique_checks": len(adapter.GSCA_SCOPED_CHECK_IDS),
                "check_ids": list(adapter.GSCA_SCOPED_CHECK_IDS),
                "failures": 0,
                "console_errors": 0,
                "runtime": "tauri-webview2-cdp",
                "cdp_endpoint": "http://127.0.0.1:9222",
                "cdp_loopback_only": True,
                "functional_offline": offline,
                "observed_process_tree": [{"name": "quickpls-desktop.exe", "executable_path": str(desktop.resolve())}],
                "graceful_process_cleanup_verified": True,
                "forced_process_cleanup_used": False,
                "orphan_processes": 0,
            }
            floor = datetime(2026, 8, 16, 1, 0, tzinfo=timezone.utc)
            with patch.object(adapter, "ROOT", root):
                validated, actual_report, actual_workbook = adapter.validate_gsca_scoped_receipt(contract, receipt, report, floor)
                self.assertTrue(validated["functional_offline_verified"])
                self.assertEqual(actual_report, report_path)
                self.assertEqual(actual_workbook, workbook)
                receipt["functional_offline"] = {**offline, "externalRequestCount": 1}
                with self.assertRaises(adapter.AdapterError):
                    adapter.validate_gsca_scoped_receipt(contract, receipt, report, floor)

    def test_focused_nca_receipt_binds_exact_checks_files_offline_and_cleanup(self) -> None:
        contract = adapter.METHODS["nca_v2"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary).resolve()
            results = root / "validation/results"
            desktop = root / "target/release/quickpls-desktop.exe"
            results.mkdir(parents=True)
            desktop.parent.mkdir(parents=True)
            desktop.write_bytes(b"desktop")
            workbook = results / "nca.xlsx"
            project = results / "nca.qpls"
            workbook.write_bytes(b"xlsx")
            project.write_bytes(b"project")
            screenshots = []
            for index in range(11):
                path = results / f"screen-{index:02}.png"
                path.write_bytes(f"screen-{index}".encode("ascii"))
                screenshots.append(str(path.resolve()))

            def row(path: Path) -> dict[str, object]:
                payload = path.read_bytes()
                return {
                    "path": path.relative_to(root).as_posix(),
                    "size": len(payload),
                    "sha256": hashlib.sha256(payload).hexdigest(),
                }

            checks = {check_id: {"passed": True} for check_id in adapter.NCA_SCOPED_CHECK_IDS}
            offline = {
                "passed": True,
                "analyticalWorkflowRequiresInternet": False,
                "strictZeroProcessEgressClaimed": False,
                "platformBackgroundEgressOutsidePageRequestScope": True,
                "observedRequestCount": 9,
                "externalRequestCount": 0,
                "origins": ["http://tauri.localhost"],
                "externalRequests": [],
            }
            checks["ncaFunctionalOffline"] = offline
            checks["ncaExport"] = {"nativeXlsx": {"targetPath": str(workbook.resolve())}}
            checks["ncaFixtureProvisioning"] = {"project": str(project.resolve())}
            report = {
                "runtime": "tauri-webview2-cdp",
                "passed": True,
                "failures": [],
                "consoleErrors": [],
                "focusedRun": {"scope": "nca", "completedAt": "2026-08-16T01:01:00Z"},
                "checks": checks,
                "screenshots": screenshots,
                "screenshotArtifacts": [row(Path(path)) for path in screenshots],
            }
            report_path = results / "v247_tauri_native_acceptance_nca.json"
            report_path.write_text(json.dumps(report), encoding="utf-8")
            receipt = {
                "schema_version": 1,
                "kind": "quickpls_v247_nca_scoped_native_acceptance_receipt",
                "passed": True,
                "supervisor_started_at_utc": "2026-08-16T01:00:00Z",
                "completed_at_utc": "2026-08-16T01:02:00Z",
                "scope": "nca",
                "feature_id": contract.feature_id,
                "method_version": contract.method_version,
                "report": row(report_path),
                "executable": row(desktop),
                "export": row(workbook),
                "project_archive": row(project),
                "screenshots": [row(Path(path)) for path in screenshots],
                "checks": len(adapter.NCA_SCOPED_CHECK_IDS),
                "unique_checks": len(adapter.NCA_SCOPED_CHECK_IDS),
                "check_ids": list(adapter.NCA_SCOPED_CHECK_IDS),
                "failures": 0,
                "console_errors": 0,
                "runtime": "tauri-webview2-cdp",
                "cdp_endpoint": "http://127.0.0.1:9222",
                "cdp_loopback_only": True,
                "functional_offline": offline,
                "observed_process_tree": [{"name": "quickpls-desktop.exe", "executable_path": str(desktop.resolve())}],
                "graceful_process_cleanup_verified": True,
                "forced_process_cleanup_used": False,
                "orphan_processes": 0,
            }
            floor = datetime(2026, 8, 16, 1, 0, tzinfo=timezone.utc)
            with patch.object(adapter, "ROOT", root):
                validated, actual_report, actual_workbook = adapter.validate_nca_scoped_receipt(contract, receipt, report, floor)
                self.assertTrue(validated["functional_offline_verified"])
                self.assertEqual(actual_report, report_path)
                self.assertEqual(actual_workbook, workbook)
                receipt["observed_process_tree"] = []
                with self.assertRaises(adapter.AdapterError):
                    adapter.validate_nca_scoped_receipt(contract, receipt, report, floor)

    def test_packaged_viewport_matrix_rejects_emulation_and_binds_screenshot_bytes(self) -> None:
        contract = adapter.METHODS["gsca_als_v2"]
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            screen_root = root / "validation/results/screens/v247-native-desktop-acceptance"
            screen_root.mkdir(parents=True)
            screenshot_paths = []
            screenshot_artifacts = []
            rows = []
            previous = {"left": 10, "top": 20, "width": 1200, "height": 800, "windowState": "normal"}
            for viewport in adapter.EXPECTED_VIEWPORTS:
                width, height = map(int, viewport.split("x"))
                path = screen_root / f"146v-tauri-native-gsca-packaged-viewport-{viewport}.png"
                payload = viewport.encode("ascii")
                path.write_bytes(payload)
                screenshot_paths.append(str(path.resolve()))
                screenshot_artifacts.append({"path": path.relative_to(root).as_posix(), "size": len(payload), "sha256": hashlib.sha256(payload).hexdigest()})
                after = {"left": 10, "top": 20, "width": width + 16, "height": height + 39, "windowState": "normal"}
                rows.append({
                    "id": viewport,
                    "requestedClientViewport": {"width": width, "height": height},
                    "domInnerDimensions": {"width": width, "height": height},
                    "outerBoundsBefore": previous,
                    "outerBoundsAfter": after,
                    "outerBoundsChanged": True,
                    "resizeAttempts": [{"attempt": 1}],
                    "origin": "http://tauri.localhost",
                    "tauriRuntime": True,
                    "surface": "results",
                    "noHorizontalOverflow": True,
                    "methodRunLinkage": True,
                    "methodVersionEvidenceBound": True,
                    "selectedRunId": "run-1",
                    "selectedRunLabel": "GSCA run",
                    "selectedTableId": "gsca_fit",
                    "resultRows": 12,
                    "screenshot": str(path.resolve()),
                    "passed": True,
                })
                previous = after
            report = {
                "checks": {"gscaPackagedViewports": {
                    "passed": True,
                    "actualTauriWindow": True,
                    "resizeMechanism": "Browser.setWindowBounds",
                    "targetIdentity": {"targetId": "target-1", "windowId": 11, "lookupCommand": "Browser.getWindowForTarget", "origin": "http://tauri.localhost"},
                    "deviceMetricsOverride": {"clearCommand": "Emulation.clearDeviceMetricsOverride", "cleared": True, "playwrightViewportBefore": None, "pageSetViewportSizeUsed": False, "emulationOnly": False},
                    "method": {"slug": contract.slug, "version": contract.method_version, "expectedRunId": "run-1", "expectedRunLabel": "GSCA run", "expectedTableId": "gsca_fit"},
                    "outerBoundsBefore": rows[0]["outerBoundsBefore"],
                    "exactViewports": rows,
                    "restoredFinalWindowState": {"passed": True, "tolerancePixels": 1, "requested": rows[0]["outerBoundsBefore"], "actual": rows[0]["outerBoundsBefore"]},
                }},
                "screenshots": screenshot_paths,
                "screenshotArtifacts": screenshot_artifacts,
            }
            with patch.object(adapter, "ROOT", root), patch.object(adapter, "SCREEN_ROOT", screen_root):
                result, _ = adapter.packaged_viewport_contract(contract, report, "run-1")
                self.assertTrue(result["actual_tauri_window"])
                self.assertFalse(result["viewport_emulation"])
                report["checks"]["gscaPackagedViewports"]["deviceMetricsOverride"]["pageSetViewportSizeUsed"] = True
                with self.assertRaises(adapter.AdapterError):
                    adapter.packaged_viewport_contract(contract, report, "run-1")
                report["checks"]["gscaPackagedViewports"]["deviceMetricsOverride"]["pageSetViewportSizeUsed"] = False
                report["checks"]["gscaPackagedViewports"]["deviceMetricsOverride"]["playwrightViewportBefore"] = {"width": 1440, "height": 900}
                with self.assertRaises(adapter.AdapterError):
                    adapter.packaged_viewport_contract(contract, report, "run-1")
                report["checks"]["gscaPackagedViewports"]["deviceMetricsOverride"]["playwrightViewportBefore"] = None
                report["checks"]["gscaPackagedViewports"]["restoredFinalWindowState"]["passed"] = False
                with self.assertRaises(adapter.AdapterError):
                    adapter.packaged_viewport_contract(contract, report, "run-1")
                report["checks"]["gscaPackagedViewports"]["restoredFinalWindowState"]["passed"] = True
                restored = report["checks"]["gscaPackagedViewports"]["restoredFinalWindowState"]
                restored["actual"] = {**restored["requested"], "width": restored["requested"]["width"] + 1}
                result, _ = adapter.packaged_viewport_contract(contract, report, "run-1")
                self.assertTrue(result["actual_tauri_window"])
                restored["actual"] = {**restored["requested"], "width": restored["requested"]["width"] + 2}
                with self.assertRaises(adapter.AdapterError):
                    adapter.packaged_viewport_contract(contract, report, "run-1")
                restored["actual"] = dict(restored["requested"])
                report["screenshotArtifacts"][0]["sha256"] = "0" * 64
                with self.assertRaises(adapter.AdapterError):
                    adapter.packaged_viewport_contract(contract, report, "run-1")

    def test_packaged_viewport_matrix_accepts_only_exact_consistent_bootstrap_slug_alias(self) -> None:
        contract = replace(
            adapter.METHODS["gsca_als_v2"],
            slug="consistent_bootstrap_v1",
            feature_id="qpls3.inference.consistent_bootstrap",
            method_version="plsc_bootstrap_v1",
        )
        self.assertEqual(adapter.packaged_viewport_method_slug(contract), "plsc_bootstrap_v1")
        self.assertEqual(
            adapter.packaged_viewport_method_slug(adapter.METHODS["gsca_als_v2"]),
            "gsca_als_v2",
        )
        with self.assertRaises(adapter.AdapterError):
            adapter.packaged_viewport_method_slug(replace(contract, feature_id="qpls3.inference.other"))


if __name__ == "__main__":
    unittest.main()
