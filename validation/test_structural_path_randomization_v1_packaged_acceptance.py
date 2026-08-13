#!/usr/bin/env python3
"""Fail-closed source/schema tests for the packaged randomization v1 slice."""

from __future__ import annotations

import copy
import json
import re
import unittest
from pathlib import Path

from jsonschema import Draft202012Validator


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
HARNESS = VALIDATION / "v247_tauri_native_acceptance.mjs"
WRAPPER = VALIDATION / "run_v247_structural_path_randomization_native_acceptance.ps1"
SCHEMA = VALIDATION / "structural_path_randomization_v1_packaged_acceptance.schema.json"

FEATURE_ID = "qpls3.inference.structural_path_randomization"
METHOD_VERSION = "freedman_lane_permutation_v1"
EVIDENCE_KIND = "quickpls3_scoped_tauri_structural_path_randomization_v1_acceptance"
RAW_REPORT = "validation/results/v247_tauri_native_acceptance_structural_path_randomization.json"
WARNING = (
    "Candidate output: single-model Freedman-Lane randomization holds the original PLS construct scores fixed "
    "and reports unadjusted pathwise two-sided plus-one p values. Interpret these as conditional, approximate "
    "inference under exchangeable reduced-model residuals. Measurement-score uncertainty is not re-estimated, "
    "no multiplicity adjustment is applied, and current calibration covers homoscedastic Gaussian errors only."
)
EXPECTED_SHARED_STRINGS = [
    "Structural path randomization",
    "Run provenance",
    "experimental",
    WARNING,
    "Randomization method",
    METHOD_VERSION,
    "Randomization operation",
    "pls_pm_freedman_lane_v1",
    "Randomized structural paths",
    "2",
    "Requested path permutations",
    "10000",
    "Randomization estimand",
    "Structural path coefficients conditional on fixed original PLS construct scores",
    "Pathwise probability",
    "Conditional/approximate two-sided plus-one probability under exchangeable reduced-model residuals; no multiplicity adjustment",
    "Qualification status",
    "Internal candidate/experimental product label; method-specific qualification evidence is tracked separately",
]
CHECK_NAMES = [
    "runtimePreflight",
    "structuralPathRandomizationFixtureProvisioning",
    "structuralPathRandomizationSetup",
    "structuralPathRandomizationCancellation",
    "structuralPathRandomizationResults",
    "structuralPathRandomizationExport",
    "structuralPathRandomizationArchive",
    "structuralPathRandomizationSaveReopen",
    "resources",
    "cleanup",
]
WORKBOOK_SHEETS = [
    "Path coefficients",
    "Outer loadings",
    "Outer weights",
    "R-square",
    "Total effects",
    "Construct reliability and valid",
    "Cross loadings",
    "Fornell-Larcker criterion",
    "HTMT+",
    "Original HTMT",
    "Structural model",
    "Inner VIF values",
    "f-square effect sizes",
    "Model fit",
    "Construct cross-validated redun",
    "Structural path randomization",
    "Run provenance",
]


def artifact(path: str = "validation/results/artifact.bin") -> dict[str, object]:
    return {"path": path, "size": 12, "sha256": "a" * 64}


def qualified_page() -> dict[str, object]:
    return {
        "index": 0,
        "url": "http://tauri.localhost/",
        "origin": "http://tauri.localhost",
        "title": "QuickPLS",
        "shellVisible": True,
        "tauriRuntime": True,
    }


def valid_report() -> dict[str, object]:
    setup_contract = {
        "catalogCount": 14,
        "selectedMethod": "Structural Path Randomization",
        "permutations": {"count": 1, "type": "number", "minimum": "99", "maximum": "10000", "step": "1", "value": "10000"},
        "workers": {"count": 1, "type": "number", "minimum": "1", "maximum": "64", "value": "4"},
        "seed": {"count": 1, "type": "number", "minimum": "0", "maximum": "4294967295", "value": "20260718"},
        "bootstrapControls": 0,
        "groupControls": 0,
        "scopeLabel": "Candidate scope",
        "scope": WARNING,
        "blockers": [],
        "startLabel": "Start path randomization",
        "startEnabled": True,
    }
    cancellation_setup_contract = copy.deepcopy(setup_contract)
    cancellation_setup_contract["workers"]["value"] = "1"
    runtime = {
        "passed": True,
        "expectedOrigin": "http://tauri.localhost",
        "enumeratedPages": [qualified_page()],
        "qualifyingPageCount": 1,
        "preReload": qualified_page(),
        "reloadCount": 1,
        "postReload": qualified_page(),
        "sameOrigin": True,
    }
    rows = [
        ["X -> Y", "0.500000", "99", "10000", str(100 / 10001)],
        ["Z -> Y", "0.250000", "499", "10000", str(500 / 10001)],
    ]
    archive = {
        "passed": True,
        "manifest": {"schemaVersion": 5, "checksumAlgorithm": "sha256", "projectChecksumMatches": True},
        "resultSchemaVersion": 1,
        "payloadKind": "pls_pm_v3",
        "bootstrapAbsent": True,
        "recipeSchemaVersion": 3,
        "recipeMethodConfigExact": True,
        "pathOrderExact": True,
        "permutation": {
            "methodVersion": METHOD_VERSION,
            "operation": "pls_pm_freedman_lane_v1",
            "permutations": 10000,
            "masterSeed": 20260718,
            "parameterCount": 2,
            "parameterContract": True,
        },
        "run": {"method": "Structural Path Randomization", "status": "completed", "snapshotNodes": 3, "snapshotEdges": 2},
    }
    return {
        "schema_version": "quickpls.packaged_acceptance.v1",
        "kind": EVIDENCE_KIND,
        "passed": True,
        "generated_at_utc": "2026-08-13T00:00:00Z",
        "completed_at_utc": "2026-08-13T00:01:00Z",
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": "2026-08-12",
        "target": "windows_10_11_x64_packaged_tauri",
        "runtime": "tauri-webview2-cdp",
        "endpoint": "http://127.0.0.1:9222",
        "generator": "validation/v247_tauri_native_acceptance.mjs",
        "acceptance_scope": "structural_path_randomization",
        "tested_product": {
            "qpls_cli_exe": artifact("target/release/qpls.exe"),
            "quickpls_desktop_exe": artifact("target/release/quickpls-desktop.exe"),
            "dist_bundle": {
                "path": "dist", "size": 12, "file_count": 1, "sha256": "b" * 64,
                "manifest": [artifact("assets/app.js")],
            },
        },
        "checks": {
            "runtimePreflight": runtime,
            "structuralPathRandomizationFixtureProvisioning": {
                "passed": True, "fixture": {}, "project": {},
                "model_name": "Structural Path Randomization Model", "status": "180 cases",
                "columns": ["#", "group", "x1", "x2", "z1", "z2", "y1", "y2"],
                "observations": 180, "initialArchive": {},
            },
            "structuralPathRandomizationSetup": {
                "passed": True, "model": {}, **setup_contract,
                "feature_id": FEATURE_ID, "method_version": METHOD_VERSION,
                "catalogue_snapshot_date": "2026-08-12",
            },
            "structuralPathRandomizationCancellation": {
                "passed": True, "activeLifecycleCaptured": True,
                "activeState": {
                    "ariaBusy": "true", "status": "queued", "phase": "Queued",
                    "message": "Native engine accepted the calculation job.",
                    "progressValue": "0", "progressMax": "100", "logEntries": 3,
                },
                "cancelButtonCount": 1, "cancelButtonEnabled": True,
                "cancelClickDispatched": True, "terminalOutcome": "cancelled",
                "completionWonRace": False,
                "cancelledState": {
                    "ariaBusy": "false", "status": "cancelled", "phase": "Cancelled",
                    "message": "Calculation cancelled.",
                    "progressValue": "0", "progressMax": "100", "logEntries": 5,
                    "logMessages": ["Calculation cancelled.", "Cancellation requested.", "Running", "Queued"],
                },
                "cancelledMessage": "Calculation cancelled.",
                "cancellationLogNewest": ["Calculation cancelled.", "Cancellation requested."],
                "cancellationLogExact": True,
                "archiveBeforeSnapshot": {
                    "phase": "before",
                    "artifact": artifact("validation/results/v247-native-structural-path-randomization-1-2-cancellation-before.qpls"),
                    "sourcePath": "validation/results/v247-native-structural-path-randomization-1-2.qpls",
                    "sourceSize": 12, "sourceSha256": "a" * 64,
                    "sourceMtimeNsBefore": "100", "sourceMtimeNsAfter": "100",
                    "sourceStableDuringCopy": True, "snapshotMatchesSource": True,
                    "datasetCount": 1, "modelCount": 1,
                    "modelName": "Structural Path Randomization Model",
                    "constructLabels": ["X", "Z", "Y"], "pathLabels": ["X -> Y", "Z -> Y"],
                    "recipeCount": 0, "resultCount": 0, "runCount": 0,
                    "recipeIds": [], "resultIds": [], "runIds": [],
                },
                "archiveAfterSnapshot": {
                    "phase": "after",
                    "artifact": artifact("validation/results/v247-native-structural-path-randomization-1-2-cancellation-after.qpls"),
                    "sourcePath": "validation/results/v247-native-structural-path-randomization-1-2.qpls",
                    "sourceSize": 12, "sourceSha256": "a" * 64,
                    "sourceMtimeNsBefore": "100", "sourceMtimeNsAfter": "100",
                    "sourceStableDuringCopy": True, "snapshotMatchesSource": True,
                    "datasetCount": 1, "modelCount": 1,
                    "modelName": "Structural Path Randomization Model",
                    "constructLabels": ["X", "Z", "Y"], "pathLabels": ["X -> Y", "Z -> Y"],
                    "recipeCount": 0, "resultCount": 0, "runCount": 0,
                    "recipeIds": [], "resultIds": [], "runIds": [],
                },
                "archiveSnapshotsByteIdentical": True, "zeroResultRecipeRunDelta": True,
                "noPartialResult": True,
                "cancellationSetup": cancellation_setup_contract,
                "exactFrozenSetupOnRetry": True,
                "retrySetup": {**cancellation_setup_contract, "startLabel": "Retry path randomization"},
                "completionSetup": {**setup_contract, "startLabel": "Retry path randomization"},
                "completionSetupExact": True,
                "retryCompleted": True, "retryRunId": "run-1", "retryNewIdentity": True,
            },
            "structuralPathRandomizationResults": {
                "passed": True, "runId": "run-1", "selectedRunId": "run-1",
                "selectedRunLabel": "Structural Path Randomization", "initialSelectedTable": "model_estimates",
                "group": "Inference", "tableId": "permutation", "title": "Structural path randomization",
                "warning": WARNING,
                "columns": ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
                "rows": rows, "pathOrder": ["X -> Y", "Z -> Y"],
                "plusOneProbabilityGridExact": True, "noBootstrapTables": True,
                "noPlaceholderValues": True, "activeLifecycle": {},
                "runDetails": {
                    "properties": {
                        "Run": "Structural Path Randomization 1",
                        "Method": "Structural Path Randomization",
                        "Created": "8/13/2026, 12:01:00 AM",
                        "Recorded seed": "20260718",
                        "Dataset fingerprint": "fingerprint-1",
                        "Recipe": "recipe-1",
                        "Engine": "2.45.0",
                        "Method version": "pls_pm_v1+pls_mediation_v1+pls_assessment_v7+freedman_lane_permutation_v1",
                        "Weighting": "path",
                        "Preprocessing": "standardized",
                    },
                    "logEntries": 2,
                },
            },
            "structuralPathRandomizationExport": {
                "passed": True, "xlsxEnabled": True, "buttonCount": 5,
                "expectedSheets": ["Structural path randomization", "Run provenance"],
                "expectedSharedStrings": list(EXPECTED_SHARED_STRINGS),
                "nativeXlsx": {
                    "attempted": True, "targetPath": "D:/QuickPLS/validation/results/result.xlsx",
                    "helper": {
                        "ready": {
                            "event": "ready", "passed": True, "phase": "main_window_binding",
                            "targetPath": "D:/QuickPLS/validation/results/result.xlsx",
                            "resultsRoot": "D:/QuickPLS/validation/results",
                            "mainWindow": {
                                "pid": 1234, "handle": 5678, "title": "QuickPLS",
                                "executable": "D:/QuickPLS/target/release/quickpls-desktop.exe",
                            },
                        },
                        "completion": {
                            "event": "complete", "passed": True, "phase": "xlsx_creation_and_readback",
                            "targetPath": "D:/QuickPLS/validation/results/result.xlsx",
                            "mainWindow": {
                                "pid": 1234, "handle": 5678, "title": "QuickPLS",
                                "executable": "D:/QuickPLS/target/release/quickpls-desktop.exe",
                            },
                            "dialog": {},
                            "boundControls": {"filenameEditControlId": 1001, "saveButtonControlId": 1},
                            "saveSubmission": {},
                            "workbook": {
                                "path": "D:/QuickPLS/validation/results/result.xlsx",
                                "size": 2048,
                                "sha256": "c" * 64,
                                "sheetNames": list(WORKBOOK_SHEETS),
                                "requiredSharedStrings": list(EXPECTED_SHARED_STRINGS),
                                "requiredSheets": ["Structural path randomization", "Run provenance"],
                            },
                            "transport": {
                                "exitCode": 0, "signal": None, "stderr": "",
                                "events": [{"event": "ready"}, {"event": "complete"}],
                                "protocolErrors": [],
                            },
                        },
                    },
                    "appFeedback": "Saved result.xlsx.", "file": {"size": 2048, "isFile": True},
                    "workbookSheets": list(WORKBOOK_SHEETS),
                    "exactRequiredSheets": True, "noBootstrapSheets": True,
                },
            },
            "structuralPathRandomizationArchive": archive,
            "structuralPathRandomizationSaveReopen": {
                "passed": True, "expectedRunId": "run-1", "reopenedRunId": "run-1",
                "selectedRunId": "run-1", "sameRunRestored": True,
                "initialSelectedTable": "model_estimates", "rows": 2,
                "columns": ["Path", "Original", "Exceedances", "Permutations", "Raw two-sided p"],
                "values": rows, "warning": WARNING, "archiveChecksumMatches": True,
            },
            "resources": {
                "passed": True, "sample_count": 20, "peak_total_working_set_bytes": 500_000_000,
                "peak_total_private_memory_bytes": 400_000_000, "peak_total_handle_count": 500,
                "peak_total_thread_count": 80, "peak_process_count": 8,
                "peak_working_set_under_2_gib": True, "zero_other_descendants": True,
                "scope": "Single packaged cancellation/retry/completion/save/reopen run; this is a bounded process footprint report, not a sustained no-leak claim.",
                "disk": {"xlsx_bytes": 1000, "source_report_bytes": 1000},
                "artifacts": {"samples": artifact(), "report": artifact()},
            },
            "cleanup": {
                "passed": True, "launched_pid": 1234, "graceful_close_exit_code": 0,
                "graceful_exit_confirmed": True, "forced_parent_termination": False,
                "forced_descendant_pids": [], "parent_exit_confirmed": True,
                "lingering_descendant_pids": [], "resource_monitor_exit_confirmed": True,
                "resource_monitor_exit_code": 0, "forced_resource_monitor_termination": False,
                "artifact": artifact(),
            },
        },
        "artifacts": {
            "xlsx": artifact(), "project_archive": artifact(), "resource_samples": artifact(),
            "resource_report": artifact(), "cleanup_report": artifact(),
            "cancellation_archive_before": artifact(
                "validation/results/v247-native-structural-path-randomization-1-2-cancellation-before.qpls"
            ),
            "cancellation_archive_after": artifact(
                "validation/results/v247-native-structural-path-randomization-1-2-cancellation-after.qpls"
            ),
            "screenshots": [artifact(f"validation/results/screens/v247-native-desktop-acceptance/19{index}-tauri-native-structural-path-randomization-state.png") for index in range(6)],
        },
        "console_errors": [], "failures": [], "source_report": RAW_REPORT,
    }


class StructuralPathRandomizationPackagedAcceptanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        cls.validator = Draft202012Validator(cls.schema)
        cls.harness = HARNESS.read_text(encoding="utf-8")
        cls.wrapper = WRAPPER.read_text(encoding="utf-8")

    def assert_invalid(self, mutate) -> None:
        report = valid_report()
        mutate(report)
        self.assertTrue(list(self.validator.iter_errors(report)))

    def test_schema_is_draft_2020_and_accepts_only_complete_evidence(self) -> None:
        Draft202012Validator.check_schema(self.schema)
        self.assertEqual(list(self.validator.iter_errors(valid_report())), [])
        mutations = [
            lambda report: report.update(passed=False),
            lambda report: report.update(feature_id="generic.bootstrap"),
            lambda report: report.update(method_version="freedman_lane_permutation_v0"),
            lambda report: report.update(source_report="validation/results/v247_tauri_native_acceptance.json"),
            lambda report: report["checks"].pop("structuralPathRandomizationArchive"),
            lambda report: report["checks"]["structuralPathRandomizationSetup"]["permutations"].update(value="999"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"].update(noPartialResult=False),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"].update(cancelButtonEnabled=False),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"].update(terminalOutcome="completed"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"]["cancellationSetup"]["workers"].update(value="4"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"]["retrySetup"]["workers"].update(value="4"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"]["completionSetup"]["workers"].update(value="1"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"].update(completionSetupExact=False),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"]["activeState"].update(status="completed"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"]["cancelledState"].update(status="completed"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"].update(archiveSnapshotsByteIdentical=False),
            lambda report: report["artifacts"]["cancellation_archive_before"].update(path="validation/results/wrong.qpls"),
            lambda report: report["checks"]["structuralPathRandomizationCancellation"].update(retryNewIdentity=False),
            lambda report: report["checks"]["structuralPathRandomizationResults"].update(pathOrder=["Z -> Y", "X -> Y"]),
            lambda report: report["checks"]["structuralPathRandomizationResults"].update(plusOneProbabilityGridExact=False),
            lambda report: report["checks"]["structuralPathRandomizationResults"]["runDetails"]["properties"].update(Method="PLS-SEM Algorithm"),
            lambda report: report["checks"]["structuralPathRandomizationResults"]["runDetails"]["properties"].update(**{"Method version": "freedman_lane_permutation_v0"}),
            lambda report: report["checks"]["structuralPathRandomizationExport"]["expectedSharedStrings"].__setitem__(15, "generic p value"),
            lambda report: report["checks"]["structuralPathRandomizationExport"]["nativeXlsx"]["helper"]["completion"].update(passed=False),
            lambda report: report["checks"]["structuralPathRandomizationExport"]["nativeXlsx"]["helper"].pop("completion"),
            lambda report: report["checks"]["structuralPathRandomizationExport"]["nativeXlsx"]["helper"]["completion"]["workbook"]["sheetNames"].reverse(),
            lambda report: report["checks"]["structuralPathRandomizationExport"]["nativeXlsx"]["workbookSheets"].reverse(),
            lambda report: report["checks"]["structuralPathRandomizationExport"]["nativeXlsx"]["helper"]["completion"]["transport"]["protocolErrors"].append("unexpected"),
            lambda report: report["checks"]["structuralPathRandomizationArchive"].update(bootstrapAbsent=False),
            lambda report: report["checks"]["structuralPathRandomizationArchive"]["permutation"].update(operation="pls_permutation"),
            lambda report: report["checks"]["structuralPathRandomizationSaveReopen"].update(sameRunRestored=False),
            lambda report: report["checks"]["resources"].update(peak_working_set_under_2_gib=False),
            lambda report: report["checks"]["cleanup"].update(forced_parent_termination=True),
            lambda report: report["console_errors"].append("boom"),
        ]
        for mutate in mutations:
            with self.subTest(mutation=mutate):
                self.assert_invalid(mutate)

    def test_harness_uses_an_isolated_scope_and_exact_scientific_contract(self) -> None:
        required = [
            'const structuralPathRandomizationOnly = acceptanceScope === "structural_path_randomization";',
            "const isolatedFocusedOnly = processV2Only || structuralPathRandomizationOnly;",
            "if (isolatedFocusedOnly && scopedReportPath !== reportPath)",
            "if (structuralPathRandomizationOnly) await writeStructuralPathRandomizationPackagedEvidence();",
            "async function runFocusedStructuralPathRandomizationAcceptance()",
            '"#nd-calculation-permutations"', '"#nd-calculation-workers"', '"#nd-calculation-seed"',
            'name: "Start path randomization"', 'name: "Retry path randomization"',
            "structuralPathRandomizationPermutations = 10_000",
            "structuralPathRandomizationSeed = 20_260_718",
            "structuralPathRandomizationCancellationWorkers = 1",
            "structuralPathRandomizationWorkers = 4",
            'initialSelectedTable === "model_estimates"',
            'tableId: await tableItem.getAttribute("data-result-tree-item-id")',
            "plusOneProbabilityGridExact: pGridExact",
            'runDetails.properties.Method === "Structural Path Randomization"',
            'runDetails.properties["Recorded seed"] === String(structuralPathRandomizationSeed)',
            'runDetails.properties.Engine === packageVersion',
            "structuralPathRandomizationProbabilityDisclosure",
            "structuralPathRandomizationQualificationDisclosure",
            "inspectSavedStructuralPathRandomizationArchive",
            "resultSchemaVersion === 1",
            "recipeSchemaVersion === 3",
            "contract.bootstrapAbsent",
            "parameter.p_value_two_sided, expectedP",
            '.nd-run-progress[aria-busy="true"]:is(.queued,.validating,.running)',
            "cancelButtonCount !== 1",
            "!cancelButtonEnabled",
            "cancel.click({ timeout: 1_000 })",
            'terminalOutcome !== "cancelled"',
            "completion_won_race",
            'return "results_surface"',
            'return "dialog_detached"',
            '.nd-run-progress.cancelled[aria-busy="false"]',
            "zeroResultRecipeRunDelta",
            "snapshotStructuralPathRandomizationCancellationArchive",
            "archiveSnapshotsByteIdentical",
            "cancellationSetup: cancelledSetup.contract",
            "completionSetupExact",
            "const completionProofPromise = page.waitForFunction",
            "allowTerminalTransitionAfterCapture: true",
            "completion proof did not bind the exact completed result identity",
            "retryNewIdentity",
        ]
        for token in required:
            self.assertIn(token, self.harness)
        self.assertNotRegex(
            self.harness,
            re.compile(r"structuralPathRandomizationOnly[\s\S]{0,120}priorEvidence\?\.checks"),
        )
        cancellation_block = self.harness.split("const cancellationArchiveBefore", 1)[1].split(
            "const activeCompletionPromise", 1
        )[0]
        self.assertNotIn("captureActiveCalculation(", cancellation_block)
        self.assertNotIn("optionalCancellingState", cancellation_block)
        self.assertNotIn("cancellingState", cancellation_block)
        ordered_tokens = [
            "await activeProgress.waitFor", "const activeCancellation = await cancelledSetup.dialog.evaluate",
            "const terminalStatePromise = page.waitForFunction", "cancel.click({ timeout: 1_000 })",
            'if (terminalOutcome !== "cancelled")', "const cancelledLifecycle = await cancelledState.evaluate",
            "const cancellationArchiveAfterSnapshot", "const zeroResultRecipeRunDelta",
            "const retrySetup = await readSetup", "await completionWorkers.fill",
            "const completionSetup = await readSetup", "evidence.checks.structuralPathRandomizationSetup =",
        ]
        offsets = [cancellation_block.index(token) for token in ordered_tokens]
        self.assertEqual(offsets, sorted(offsets))
        completion_block = self.harness.split("const activeCompletionPromise", 1)[1].split(
            "const selectedRunId", 1
        )[0]
        completion_tokens = [
            "captureActiveCalculation(", "allowTerminalTransitionAfterCapture: true",
            "const completionProofPromise = page.waitForFunction", "await retry.click()",
            "const completionProof = await completionProofPromise", "const activeCompletion = await activeCompletionPromise",
            "completion proof did not bind the exact completed result identity",
        ]
        completion_offsets = [completion_block.index(token) for token in completion_tokens]
        self.assertEqual(completion_offsets, sorted(completion_offsets))

    def test_wrapper_is_exact_pid_fail_closed_and_never_broad_kills(self) -> None:
        required = [
            '$env:QUICKPLS_ACCEPTANCE_SCOPE = "structural_path_randomization"',
            '$env:QUICKPLS_STRUCTURAL_PATH_RANDOMIZATION_EXPORT_PATH = $ExportPath',
            '$resourceMonitorHandle = $resourceMonitor.Handle',
            '$cleanup.graceful_exit_confirmed = $application.WaitForExit(10000)',
            'Stop-Process -Id $application.Id -Force',
            '$cleanup.resource_monitor_exit_confirmed = $resourceMonitor.WaitForExit(5000)',
            '$resourceMonitor.WaitForExit()',
            '$cleanup.resource_monitor_exit_code = [int]$resourceMonitor.ExitCode',
            '$cleanup.forced_parent_termination',
            '$cleanup.forced_descendant_pids',
            '$cleanup.lingering_descendant_pids',
            '$resourceReport.peak_working_set_under_2_gib',
            'cancellation_archive_before = $sourcePackaged.artifacts.cancellation_archive_before',
            'cancellation_archive_after = $sourcePackaged.artifacts.cancellation_archive_after',
            '"Single packaged cancellation/retry/completion/save/reopen run; this is a bounded process footprint report, not a sustained no-leak claim."',
        ]
        for token in required:
            self.assertIn(token, self.wrapper)
        self.assertNotRegex(self.wrapper, re.compile(r"Stop-Process\s+-Name", re.IGNORECASE))
        self.assertNotIn("IsPathFullyQualified", self.wrapper)
        self.assertIn('Get-Process -Name "quickpls-desktop"', self.wrapper)

    def test_frozen_names_are_identical_across_harness_schema_and_wrapper(self) -> None:
        schema_text = SCHEMA.read_text(encoding="utf-8")
        for token in [FEATURE_ID, METHOD_VERSION, EVIDENCE_KIND, RAW_REPORT, *CHECK_NAMES]:
            self.assertIn(token, schema_text)
            self.assertIn(token, self.harness if token != RAW_REPORT else schema_text)
        self.assertEqual(list(valid_report()["checks"]), CHECK_NAMES)


if __name__ == "__main__":
    unittest.main()
