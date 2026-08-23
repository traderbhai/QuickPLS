from __future__ import annotations

import json
import re
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "validation" / "run_v247_cumulative_native_acceptance.ps1"
HARNESS = ROOT / "validation" / "v247_tauri_native_acceptance.mjs"
NCA_WRAPPER = ROOT / "validation" / "run_v247_nca_native_acceptance.ps1"
PLS_PACKAGED_GATE = ROOT / "validation" / "pls_algorithm_v1_packaged_acceptance.py"
ACCEPTANCE_CONTRACT = ROOT / "validation/capabilities/packaged_windows_acceptance_v2.manifest.json"
CLOSE_HELPER = ROOT / "validation/close_tauri_test_window.mjs"
DESKTOP_LIB = ROOT / "src-tauri/src/lib.rs"
ASSEMBLER = ROOT / "validation/assemble_v247_cumulative_native_acceptance.py"


class CumulativeNativeAcceptanceSupervisorSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8")
        cls.harness_source = HARNESS.read_text(encoding="utf-8")
        cls.nca_wrapper_source = NCA_WRAPPER.read_text(encoding="utf-8")
        cls.pls_packaged_gate_source = PLS_PACKAGED_GATE.read_text(encoding="utf-8")
        cls.acceptance_contract = json.loads(ACCEPTANCE_CONTRACT.read_text(encoding="utf-8"))
        cls.close_helper_source = CLOSE_HELPER.read_text(encoding="utf-8")
        cls.desktop_lib_source = DESKTOP_LIB.read_text(encoding="utf-8")
        cls.assembler_source = ASSEMBLER.read_text(encoding="utf-8")

    def test_powershell_51_parser_accepts_supervisor(self) -> None:
        command = (
            "$tokens=$null; $errors=$null; "
            f"[System.Management.Automation.Language.Parser]::ParseFile('{SCRIPT}', [ref]$tokens, [ref]$errors) | Out-Null; "
            "if ($errors.Count -ne 0) { $errors | ForEach-Object { Write-Error $_.Message }; exit 1 }"
        )
        completed = subprocess.run(
            ["powershell.exe", "-NoProfile", "-Command", command],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)

    def test_focused_nca_wrapper_is_scoped_and_fail_closed(self) -> None:
        command = (
            "$tokens=$null; $errors=$null; "
            f"[System.Management.Automation.Language.Parser]::ParseFile('{NCA_WRAPPER}', [ref]$tokens, [ref]$errors) | Out-Null; "
            "if ($errors.Count -ne 0) { exit 1 }"
        )
        completed = subprocess.run(
            ["powershell.exe", "-NoProfile", "-Command", command],
            cwd=ROOT,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)
        self.assertIn('$env:QUICKPLS_ACCEPTANCE_SCOPE = "nca"', self.nca_wrapper_source)
        self.assertIn("QUICKPLS_NCA_NATIVE_EXPORT_PATH", self.nca_wrapper_source)
        self.assertIn('$report.focusedRun.scope -ne "nca"', self.nca_wrapper_source)

    def test_full_scope_is_fresh_and_binds_all_required_exports(self) -> None:
        self.assertIn('SetEnvironmentVariable("QUICKPLS_ACCEPTANCE_SCOPE", "full"', self.source)
        for variable in (
            "QUICKPLS_NATIVE_EXPORT_PATH",
            "QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH",
            "QUICKPLS_PLSC_NATIVE_EXPORT_PATH",
            "QUICKPLS_WPLS_NATIVE_EXPORT_PATH",
            "QUICKPLS_MGA_NATIVE_EXPORT_PATH",
            "QUICKPLS_CCA_NATIVE_EXPORT_PATH",
            "QUICKPLS_IPMA_NATIVE_EXPORT_PATH",
            "QUICKPLS_NCA_NATIVE_EXPORT_PATH",
        ):
            self.assertIn(variable, self.source)
        self.assertIn('ExpectedScope $null', self.source)
        self.assertIn('QUICKPLS_CLI_PATH", $cliExecutable', self.source)
        self.assertIn('QUICKPLS_DESKTOP_EXE_PATH", $desktopExecutable', self.source)

    def test_close_helper_exits_the_application_instead_of_only_destroying_the_window(self) -> None:
        self.assertIn('invoke("exit_desktop_application")', self.close_helper_source)
        self.assertNotIn('invoke("plugin:window|destroy"', self.close_helper_source)
        self.assertIn("fn exit_desktop_application(app: tauri::AppHandle)", self.desktop_lib_source)
        self.assertIn("app.exit(0);", self.desktop_lib_source)
        self.assertIn("exit_desktop_application,", self.desktop_lib_source)

    def test_focused_wrappers_are_exactly_ordered(self) -> None:
        ordered_scripts = re.findall(
            r'Script = "(run_v247_[a-z_]+_native_acceptance\.ps1)"', self.source
        )
        self.assertEqual(
            ordered_scripts,
            [
                "run_v247_prediction_native_acceptance.ps1",
                "run_v247_hoc_native_acceptance.ps1",
                "run_v247_pca_native_acceptance.ps1",
                "run_v247_ols_native_acceptance.ps1",
                "run_v247_cbsem_native_acceptance.ps1",
                "run_v247_gsca_native_acceptance.ps1",
                "run_v247_logistic_native_acceptance.ps1",
                "run_v247_regression_bootstrap_native_acceptance.ps1",
            ],
        )
        self.assertIn('"-OlsExportPath"', self.source)
        self.assertIn('"-LogisticExportPath"', self.source)
        self.assertIn(
            '"-ReceiptPath", (Join-Path $resultsDirectory "v247_gsca_scoped_native_acceptance_receipt_$runStamp.json")',
            self.source,
        )
        pca_stage = re.search(
            r'Name = "pca";.*?Arguments = @\((.*?)\); Exports',
            self.source,
        )
        self.assertIsNotNone(pca_stage)
        self.assertEqual(pca_stage.group(1).count('"-PreserveMainReport"'), 1)
        self.assertEqual(self.source.count('"-PreserveMainReport"'), 1)

    def test_final_gate_is_exact_and_fail_closed(self) -> None:
        self.assertIsNone(re.search(r"\$expectedFinalCheckCount\s*=\s*\d+", self.source))
        self.assertIn("$expectedFinalCheckNames = @($acceptanceContract.ordered_check_sets", self.source)
        self.assertIn("$expectedFinalCheckCount = $expectedFinalCheckNames.Count", self.source)
        self.assertIn("$report.passed -ne $true", self.source)
        self.assertIn("$check.passed -ne $true", self.source)
        self.assertIn("@($report.failures).Count -ne 0", self.source)
        self.assertIn("@($report.consoleErrors).Count -ne 0", self.source)
        self.assertIn("HashSet[string]", self.source)
        self.assertIn("duplicate check name", self.source)
        self.assertIn("acceptance changed the preserved full acceptance report", self.source)
        self.assertIn("ExpectedCheckCount $expectedFinalCheckCount", self.source)
        self.assertIn("-FullReportBaseline $fullReportBaseline", self.source)
        self.assertIn("assemble_v247_cumulative_native_acceptance.py", self.source)
        self.assertIn("Cumulative acceptance assembly failed", self.source)
        self.assertIn("$missingRequiredChecks", self.source)
        self.assertIn("$unexpectedChecks", self.source)
        self.assertNotIn("check order differs from the manifest", self.source)
        self.assertIn("acceptance_contract = [pscustomobject]", self.source)
        self.assertIn("-NotBeforeUtc $supervisorStartedUtc", self.source)

    def test_phase2_release_checks_are_the_exact_frozen_fourteen_check_union(self) -> None:
        expected = {
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
        }
        self.assertEqual(
            set(self.acceptance_contract["phase2_release_required_check_ids"]), expected
        )
        self.assertIn(
            "$phase2ReleaseCheckNames = @($acceptanceContract.phase2_release_required_check_ids)",
            self.source,
        )
        direct_assignments = re.findall(
            r"evidence\.checks\.([A-Za-z0-9]+)\s*=\s*\{", self.harness_source
        )
        viewport_names = {name for name in expected if name.endswith("PackagedViewports")}
        direct_names = expected - viewport_names
        self.assertEqual({name for name in direct_assignments if name in direct_names}, direct_names)
        for name in direct_names:
            self.assertEqual(direct_assignments.count(name), 1, name)
        for name in viewport_names:
            self.assertEqual(self.harness_source.count(f'checkName: "{name}"'), 1, name)
        self.assertIn(
            "$phase2ReleaseCheckNames | Where-Object { $final.CheckNames -notcontains $_ }",
            self.source,
        )
        self.assertIn("$missingPhase2ReleaseChecks.Count -ne 0", self.source)
        self.assertIn("const ncaPermutationSamples = 9_999;", self.harness_source)
        self.assertIn("const ncaObservations = 1_024;", self.harness_source)
        self.assertIn("repeatedReferencePattern: referenceRows", self.harness_source)
        self.assertIn('scopeValues["Analyzed observations"] !== String(ncaObservations)', self.harness_source)
        self.assertIn("contract.usedObservations !== ncaObservations", self.harness_source)
        self.assertIn("contract.observations !== ncaObservations", self.harness_source)
        self.assertIn(
            "const isolatedFocusedOnly = mgaOnly || hocOnly || predictionOnly || cbsemOnly || pcaOnly || olsOnly",
            self.harness_source,
        )
        self.assertIn(
            "|| logisticOnly || regressionBootstrapOnly || ncaOnly || ctaPlsOnly || processV2Only",
            self.harness_source,
        )
        self.assertIn(
            "|| structuralPathRandomizationOnly || gscaOnly;",
            self.harness_source,
        )
        self.assertIn("const ncaTerminalStatePromise = page.waitForFunction", self.harness_source)
        self.assertIn("completion_won_race: NCA reached ${ncaTerminalOutcome}", self.harness_source)
        self.assertIn('ncaTerminalOutcome !== "cancelled"', self.harness_source)
        self.assertIn("const predictionObservations = 8_192;", self.harness_source)
        self.assertIn("provisionPredictionReferenceFixture", self.harness_source)
        self.assertIn("repeatedReferenceRows: referenceRows.length", self.harness_source)
        self.assertIn("evidence.checks.fixtureProvisioning.predictionReferenceFixture = predictionReferenceFixture;", self.harness_source)
        self.assertNotIn("evidence.checks.predictionReferenceFixture =", self.harness_source)
        self.assertIn("const predictionTerminalStatePromise = page.waitForFunction", self.harness_source)
        self.assertIn("completion_won_race: PLSpredict / CVPAT reached ${predictionTerminalOutcome}", self.harness_source)
        self.assertIn('predictionTerminalOutcome !== "cancelled"', self.harness_source)
        prediction_start = self.harness_source.index("async function runFocusedPredictionAcceptance()")
        prediction_end = self.harness_source.index("\ntry {", prediction_start)
        prediction_source = self.harness_source[prediction_start:prediction_end]
        self.assertLess(
            prediction_source.index("const predictionCancellationRequestPromise = page.waitForFunction"),
            prediction_source.index("await start.click();"),
        )
        self.assertLess(
            prediction_source.index("await start.click();"),
            prediction_source.index('capture(predictionCaptureName("92a", "cancellation-running"))'),
        )
        self.assertIn('message !== "Native engine accepted the calculation job."', prediction_source)
        self.assertIn("cancelButtons[0].click();", prediction_source)
        self.assertIn("{ timeout: 5_000 }", prediction_source)
        self.assertNotIn("captureActiveCalculation(\n    dialog,\n    predictionCaptureName(\"92a\"", prediction_source)
        self.assertIn('scopeValues["CB-SEM bootstrap"] !== "Not requested"', self.harness_source)
        self.assertIn("reopenedModificationRows !== 50 || reopenedScopeRows !== 15", self.harness_source)

    def test_shared_harness_derives_a_unique_execution_adapter_order(self) -> None:
        self.assertNotIn("EXPECTED_NATIVE_CALCULATION_KIND_ORDER", self.harness_source)
        self.assertIn(
            "execution-adapter order must be non-empty and unique",
            self.harness_source,
        )

    def test_packaged_viewports_resize_the_actual_window_not_an_emulated_page(self) -> None:
        start = self.harness_source.index("async function captureActualTauriViewportMatrix")
        end = self.harness_source.index("\nasync function openMenuItem", start)
        helper = self.harness_source[start:end]
        for command in (
            'Target.getTargetInfo',
            'Browser.getWindowForTarget',
            'Browser.getWindowBounds',
            'Browser.setWindowBounds',
            'Emulation.clearDeviceMetricsOverride',
        ):
            self.assertIn(command, helper)
        self.assertNotIn("page.setViewportSize", helper)
        self.assertNotIn("page.setViewportSize", self.harness_source)
        self.assertIn("page.viewportSize()", helper)
        self.assertIn("page.viewportSize()", self.harness_source)
        self.assertIn("setActualTauriClientViewport(viewport", self.harness_source)
        self.assertIn("pageSetViewportSizeUsed: false", helper)
        self.assertIn("actualTauriWindow: true", helper)
        self.assertIn("contract.restoredFinalWindowState?.passed === true", helper)
        self.assertIn('const leftState = left.windowState ?? "normal";', self.harness_source)
        self.assertIn('const rightState = right.windowState ?? "normal";', self.harness_source)
        self.assertIn("function windowBoundsEqual(left, right, tolerancePixels = 0)", self.harness_source)
        self.assertIn("Math.abs(left[key] - right[key]) <= tolerancePixels", self.harness_source)
        self.assertIn("tolerancePixels: 1", self.harness_source)
        self.assertIn("windowBoundsEqual(initialWindow.bounds, restored, 1)", self.harness_source)
        self.assertIn("evidence.screenshotArtifacts = (await Promise.all(evidence.screenshots.map(artifactDigest))).filter(Boolean)", self.harness_source)
        descriptor_start = self.harness_source.index("async function writeAcceptanceEvidence()")
        descriptor_end = self.harness_source.index("\nasync function artifactDigest", descriptor_start)
        descriptor_block = self.harness_source[descriptor_start:descriptor_end]
        self.assertIn("evidence.failures.push(screenshotDescriptorFailure);", descriptor_block)
        self.assertIn(
            "evidence.passed = evidence.failures.length === 0 && evidence.consoleErrors.length === 0;",
            descriptor_block,
        )
        self.assertLess(
            descriptor_block.index("evidence.failures.push(screenshotDescriptorFailure);"),
            descriptor_block.index("evidence.passed = evidence.failures.length === 0"),
        )

    def test_phase2_invalid_and_cancellation_checks_are_fail_closed(self) -> None:
        for method in ("gsca", "cca", "ipma", "cbsem", "prediction", "nca"):
            token = f"evidence.checks.{method}InvalidSetup"
            start = self.harness_source.index(f"{token} = {{")
            end = self.harness_source.index("\n  }", start) + 4
            block = self.harness_source[start:end]
            self.assertIn("attempted: true", block)
            self.assertIn("startEnabled:", block)
            self.assertIn("archiveBefore:", block)
            self.assertIn("archiveAfter:", block)
            self.assertIn("archiveStateUnchanged:", block)
            self.assertIn("resultCreated:", block)
        for method in ("prediction", "nca"):
            token = f"evidence.checks.{method}CancellationRetry"
            start = self.harness_source.index(f"{token} = {{")
            end = self.harness_source.index("\n  };", start) + 5
            block = self.harness_source[start:end]
            for required in (
                "passed:",
                "cancelledMethod:",
                "cancelledSettings:",
                "noPartialVisibleResult:",
                "noPartialCommittedResult:",
                "archiveStateUnchanged:",
                "archiveBefore:",
                "archiveAfter:",
                "retrySettings:",
                "retryEnabled:",
                "completedRetryRunId:",
            ):
                self.assertIn(required, block)

    def test_shared_harness_uses_only_current_micom_and_mga_v4_identity(self) -> None:
        for assignment in (
            'const mgaMethodVersion = "pls_mga_two_group_v4";',
            'const mgaPermutationMethodVersion = "pls_mga_permutation_v4";',
            'const micomMethodVersion = "micom_v4";',
        ):
            self.assertIn(assignment, self.harness_source)
        self.assertIn(
            'contract.recipe?.status !== "validated_micom_v4_and_permutation_mga_v4_fixed_plan_scope"',
            self.harness_source,
        )
        self.assertGreaterEqual(
            self.harness_source.count(
                "/(?:pls_mga_two_group|pls_mga_permutation|micom)_v[1-3]$/"
            ),
            2,
        )
        for legacy_assignment in (
            'const mgaMethodVersion = "pls_mga_two_group_v2";',
            'const mgaPermutationMethodVersion = "pls_mga_permutation_v2";',
            'const micomMethodVersion = "micom_v2";',
        ):
            self.assertNotIn(legacy_assignment, self.harness_source)

    def test_focused_screenshot_inheritance_replaces_letter_suffixed_states(self) -> None:
        for pattern in (
            r"(?:84|85|86|87|88|89)[a-z]?-tauri-native-nca-",
            r"9[0-7][a-z]?-tauri-native-prediction-",
            r"13[0-6][a-z]?-tauri-native-cbsem-",
            r"14[0-6][a-z]?-tauri-native-gsca-",
        ):
            self.assertIn(pattern, self.harness_source)

    def test_empty_redirected_logs_are_safe_on_windows_powershell_51(self) -> None:
        read_log = self.source[
            self.source.index("function Read-LogText") : self.source.index(
                "function Assert-AcceptanceReport"
            )
        ]
        self.assertIn('if ($null -eq $text) { return "" }', read_log)
        self.assertLess(
            read_log.index('if ($null -eq $text) { return "" }'),
            read_log.index("return $text.Trim()"),
        )

    def test_focused_reports_are_boundedly_awaited_and_logs_survive_failure(self) -> None:
        self.assertIn("function Wait-AcceptanceReportPublished", self.source)
        self.assertIn("[int]$TimeoutMilliseconds = 5000", self.source)
        self.assertIn("wrapper exited 0 without publishing a fresh scoped report", self.source)
        read_logs = self.source.index("$stdout = Read-LogText -Path $stdoutPath")
        publication_wait = self.source.index("$scopedPublished = Wait-AcceptanceReportPublished")
        remove_logs = self.source.index(
            "Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force",
            read_logs,
        )
        self.assertLess(read_logs, publication_wait)
        self.assertLess(publication_wait, remove_logs)

    def test_focused_runs_preserve_the_fresh_full_report(self) -> None:
        full_capture = self.source.index("$fullReportBaseline = Assert-AcceptanceReport")
        focused_loop = self.source.index("foreach ($stage in $focusedStages)")
        final_gate = self.source.index("$final = Assert-AcceptanceReport")
        self.assertLess(full_capture, focused_loop)
        self.assertLess(focused_loop, final_gate)
        self.assertNotIn("$cumulativePublished = Wait-AcceptanceReportPublished", self.source)
        self.assertNotIn("scoped and cumulative reports are not byte-identical", self.source)
        self.assertIn("final_scope = [string]$acceptanceContract.final_scope", self.source)

    def test_cumulative_assembler_selects_only_contract_checks_from_clean_scoped_sources(self) -> None:
        self.assertIn("for check_id in required_check_ids", self.assembler_source)
        self.assertIn("checks[check_id] = deepcopy(source_checks[check_id])", self.assembler_source)
        self.assertIn('"supplementalCheckIds": supplemental_check_ids', self.assembler_source)
        self.assertIn("os.replace(temporary, output_path)", self.assembler_source)
        self.assertIn('"cumulativeAssembly": True', self.assembler_source)

    def test_cleanup_tracks_exact_pid_identity_and_never_broad_kills(self) -> None:
        self.assertIn("CreationDate", self.source)
        self.assertIn("Add-TrackedProcessTree", self.source)
        self.assertIn("Stop-ExactTrackedProcesses", self.source)
        self.assertIn("Stop-Process -Id $processId", self.source)
        self.assertNotRegex(self.source, r"Stop-Process\s+-Name")
        self.assertNotRegex(self.source, r"taskkill|Stop-Process\s+-Id\s+\(Get-Process")
        self.assertIn("required forced exact-PID cleanup", self.source)
        self.assertIn("Assert-CleanLaunchBoundary", self.source)
        self.assertIn("Wait-CdpClosed", self.source)
        self.assertIn("$wrapperExitCode -ne 0", self.source)

    def test_focused_desktop_identity_survives_only_a_terminal_wmi_path_gap(self) -> None:
        self.assertIn("$acceptedDesktopIdentities = @{}", self.source)
        self.assertIn("$unresolvedDesktopIdentities = @{}", self.source)
        self.assertIn(
            '$identityKey = "$($descriptor.ProcessId)|$($descriptor.CreationDate)|$($descriptor.Name)"',
            self.source,
        )
        self.assertIn(
            "if (-not $acceptedDesktopIdentities.ContainsKey($identityKey))",
            self.source,
        )
        self.assertIn("$acceptedDesktopIdentities[$identityKey] = $descriptor", self.source)
        self.assertIn("$unresolvedDesktopIdentities.Remove($identityKey)", self.source)
        self.assertIn(
            "could not authenticate every observed QuickPLS executable",
            self.source,
        )
        self.assertIn(
            "observed an unexpected QuickPLS executable",
            self.source,
        )

    def test_unique_non_overwriting_artifacts_and_stop_on_red(self) -> None:
        self.assertIn("duplicate export path", self.source)
        self.assertIn("refuses to overwrite an existing export", self.source)
        full_call = self.source.index("Invoke-FreshFullAcceptance -ExportPaths $exports")
        focused_loop = self.source.index("foreach ($stage in $focusedStages)")
        final_gate = self.source.index("$final = Assert-AcceptanceReport")
        self.assertLess(full_call, focused_loop)
        self.assertLess(focused_loop, final_gate)
        self.assertNotIn("-ErrorAction Continue", self.source)

    def test_success_receipt_is_written_only_after_final_cleanup_gate(self) -> None:
        receipt_cleanup = self.source.index(
            "Remove-Item -LiteralPath $cumulativeReceiptPath"
        )
        full_call = self.source.index("Invoke-FreshFullAcceptance -ExportPaths $exports")
        final_cleanup = self.source.index(
            'Assert-CleanLaunchBoundary -Stage "Completed cumulative native acceptance"'
        )
        receipt_write = self.source.index(
            "Set-Content -LiteralPath $cumulativeReceiptPath"
        )
        self.assertLess(receipt_cleanup, full_call)
        self.assertLess(final_cleanup, receipt_write)
        for token in (
            'kind = "quickpls_v247_cumulative_native_acceptance_receipt"',
            "graceful_process_cleanup_verified = $true",
            "report_sha256 = $final.Sha256",
            "assembler = [pscustomobject]",
            "full_report = [pscustomobject]",
            "exports = $exportDescriptors",
        ):
            self.assertIn(token, self.source)

    def test_pls_packaged_contract_binds_invalid_export_and_reopen_to_exact_runs(self) -> None:
        invalid = self.harness_source.index("evidence.checks.plsAlgorithmInvalidSetup")
        model_build = self.harness_source.index("await buildThreeConstructMediationModel();")
        pls_export = self.harness_source.index("evidence.checks.mediationExport = {")
        bootstrap_start = self.harness_source.index("const mediationBootstrapDialog")
        self.assertLess(invalid, model_build)
        self.assertLess(pls_export, bootstrap_start)
        self.assertIn("runStateUnchanged: invalidPlsRunStateUnchanged", self.harness_source)
        self.assertIn("selectedRunId: await page.locator(\".nd-run-select select\").inputValue()", self.harness_source)
        self.assertIn("evidence.checks.mediationExport.bootstrap = {", self.harness_source)
        self.assertIn("evidence.checks.bootstrapInvalidSetup = {", self.harness_source)
        self.assertIn("evidence.checks.bootstrapCancellationRetry = {", self.harness_source)
        self.assertIn("evidence.checks.bootstrapResponsiveViewports = {", self.harness_source)
        self.assertIn("evidence.checks.bootstrapFunctionalOffline = {", self.harness_source)
        self.assertIn("QUICKPLS_BOOTSTRAP_NATIVE_EXPORT_PATH", self.harness_source)
        self.assertIn("selectedPlsRunId: reopenedMediationPlsRunId", self.harness_source)
        self.assertIn('reopened_run_id = save_reopen.get("selectedPlsRunId")', self.pls_packaged_gate_source)
        self.assertIn('bootstrap_export = export.get("bootstrap", {})', self.pls_packaged_gate_source)

    def test_plsc_and_wpls_packaged_contracts_are_exact_and_fail_closed(self) -> None:
        initial_model = self.harness_source.index("evidence.checks.initialEditableModelCreation")
        plsc_invalid = self.harness_source.index("evidence.checks.plscInvalidSetup")
        wpls_invalid = self.harness_source.index("evidence.checks.wplsInvalidSetup")
        model_completion = self.harness_source.index(
            "await buildTwoConstructModel({ firstIndicatorAlreadyAssigned: true });"
        )
        plsc_export = self.harness_source.index("evidence.checks.plscExport = {")
        wpls_export = self.harness_source.index("evidence.checks.wplsExport = {")
        plsc_reopen = self.harness_source.index("evidence.checks.plscSaveReopen = {")
        wpls_reopen = self.harness_source.index("evidence.checks.wplsSaveReopen = {")
        self.assertLess(initial_model, plsc_invalid)
        self.assertLess(plsc_invalid, wpls_invalid)
        self.assertLess(wpls_invalid, model_completion)
        self.assertLess(plsc_export, wpls_export)
        self.assertLess(wpls_export, plsc_reopen)
        self.assertLess(plsc_reopen, wpls_reopen)
        for token in (
            "underspecifiedReflectiveBlocker",
            "missingWeightBlocker",
            "runStateUnchanged",
            "resultCreated",
            "QUICKPLS_PLSC_NATIVE_EXPORT_PATH",
            "QUICKPLS_WPLS_NATIVE_EXPORT_PATH",
            "methodSheetsPresentExactlyOnce",
            "sameRunRestored: reopenedPlscRunId === plscRunId",
            "sameRunRestored: reopenedWplsRunId === wplsRunId",
        ):
            self.assertIn(token, self.harness_source)

    def test_live_packaged_launcher_opens_every_typed_bundled_sample(self) -> None:
        required = (
            "async function inspectBundledSample(sample)",
            '.nd-launcher[aria-label="Project launcher"]',
            '.nd-sample-project-list button[data-sample-id]',
            'launcher.locator(`.nd-sample-project-list button[data-sample-id="${sample.id}"]`)',
            '["corporate_reputation", "simple_pls", "mediation", "organizational_identification"]',
            'id: "corporate_reputation"',
            'id: "simple_pls"',
            'id: "mediation"',
            'id: "organizational_identification"',
            "await openResultTable(sample.pathTable)",
            "await structuralPaths().count()",
            "evidence.checks.bundledSampleGallery",
            "liveLauncher: true",
            "typedSelector: true",
            "completedCanonicalResults: true",
            'statusItems.nth(2)',
            'statusItems.nth(3)',
            'statusItems.nth(4)',
        )
        for token in required:
            self.assertIn(token, self.harness_source)
        sample_contracts_start = self.harness_source.index(
            "const bundledSampleContracts = ["
        )
        sample_contracts_end = self.harness_source.index(
            "    ];", sample_contracts_start
        )
        sample_contracts = self.harness_source[
            sample_contracts_start:sample_contracts_end
        ]
        self.assertEqual(
            re.findall(
                r'id: "(corporate_reputation|simple_pls|mediation|organizational_identification)"[\s\S]*?pathTable: "([^"]+)"',
                sample_contracts,
            ),
            [
                ("corporate_reputation", "Path coefficients"),
                ("simple_pls", "Path coefficients"),
                ("mediation", "Direct effects"),
                ("organizational_identification", "Path coefficients"),
            ],
        )
        self.assertNotIn(".first().click() // bundled sample", self.harness_source)

    def test_project_saved_toast_waits_bind_the_newest_visible_toast(self) -> None:
        ambiguous = (
            'page.locator(".nd-toast").filter({ hasText: /Project saved/i })'
            '.waitFor('
        )
        newest = (
            'page.locator(".nd-toast").filter({ hasText: /Project saved/i })'
            '.last().waitFor('
        )
        self.assertNotIn(ambiguous, self.harness_source)
        self.assertGreaterEqual(self.harness_source.count(newest), 1)

    def test_pls_and_bootstrap_xlsx_helpers_have_distinct_same_run_contracts(self) -> None:
        pls_start = self.harness_source.index(
            "const nativeSaveHelper = startWindowsNativeSaveExportHelper({",
            self.harness_source.index("evidence.checks.mediationExport = {")
        )
        bootstrap_start = self.harness_source.index(
            "const bootstrapSaveHelper = startWindowsNativeSaveExportHelper({",
            pls_start,
        )
        pls_contract = self.harness_source[pls_start:bootstrap_start]
        bootstrap_contract = self.harness_source[bootstrap_start:]
        for token in (
            '"Direct effects"',
            '"Specific indirect effects"',
            '"Construct cross-validated redun"',
            '"Run provenance"',
        ):
            self.assertIn(token, pls_contract)
        self.assertNotIn('"Aggregate mediation effects boo"', pls_contract)
        self.assertIn('"Aggregate mediation effects boo"', bootstrap_contract)


if __name__ == "__main__":
    unittest.main()
