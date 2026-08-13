from __future__ import annotations

import re
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "validation" / "run_v247_cumulative_native_acceptance.ps1"
HARNESS = ROOT / "validation" / "v247_tauri_native_acceptance.mjs"
PLS_PACKAGED_GATE = ROOT / "validation" / "pls_algorithm_v1_packaged_acceptance.py"


class CumulativeNativeAcceptanceSupervisorSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8")
        cls.harness_source = HARNESS.read_text(encoding="utf-8")
        cls.pls_packaged_gate_source = PLS_PACKAGED_GATE.read_text(encoding="utf-8")

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

    def test_final_gate_is_exact_and_fail_closed(self) -> None:
        self.assertIn("$expectedFinalCheckCount = 177", self.source)
        self.assertIn("$report.passed -ne $true", self.source)
        self.assertIn("$check.passed -ne $true", self.source)
        self.assertIn("@($report.failures).Count -ne 0", self.source)
        self.assertIn("@($report.consoleErrors).Count -ne 0", self.source)
        self.assertIn("HashSet[string]", self.source)
        self.assertIn("duplicate check name", self.source)
        self.assertIn("scoped and cumulative reports are not byte-identical", self.source)
        self.assertIn("ExpectedCheckCount $expectedFinalCheckCount", self.source)
        self.assertIn("-NotBeforeUtc $supervisorStartedUtc", self.source)

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
        self.assertIn("wrapper exited 0 without publishing fresh reports", self.source)
        read_logs = self.source.index("$stdout = Read-LogText -Path $stdoutPath")
        publication_wait = self.source.index("$scopedPublished = Wait-AcceptanceReportPublished")
        remove_logs = self.source.index(
            "Remove-Item -LiteralPath $stdoutPath, $stderrPath -Force",
            read_logs,
        )
        self.assertLess(read_logs, publication_wait)
        self.assertLess(publication_wait, remove_logs)

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
            '["corporate_reputation", "simple_pls", "mediation"]',
            'id: "corporate_reputation"',
            'id: "simple_pls"',
            'id: "mediation"',
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
                r'id: "(corporate_reputation|simple_pls|mediation)"[\s\S]*?pathTable: "([^"]+)"',
                sample_contracts,
            ),
            [
                ("corporate_reputation", "Direct effects"),
                ("simple_pls", "Path coefficients"),
                ("mediation", "Direct effects"),
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
