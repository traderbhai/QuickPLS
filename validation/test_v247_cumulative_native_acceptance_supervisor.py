from __future__ import annotations

import re
import subprocess
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "validation" / "run_v247_cumulative_native_acceptance.ps1"


class CumulativeNativeAcceptanceSupervisorSourceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8")

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
        self.assertIn("$expectedFinalCheckCount = 166", self.source)
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

    def test_unique_non_overwriting_artifacts_and_stop_on_red(self) -> None:
        self.assertIn("duplicate export path", self.source)
        self.assertIn("refuses to overwrite an existing export", self.source)
        full_call = self.source.index("Invoke-FreshFullAcceptance -ExportPaths $exports")
        focused_loop = self.source.index("foreach ($stage in $focusedStages)")
        final_gate = self.source.index("$final = Assert-AcceptanceReport")
        self.assertLess(full_call, focused_loop)
        self.assertLess(focused_loop, final_gate)
        self.assertNotIn("-ErrorAction Continue", self.source)


if __name__ == "__main__":
    unittest.main()
