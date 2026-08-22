from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WRAPPERS = (
    ROOT / "validation/run_v255_installed_portable_smoke.ps1",
    ROOT / "validation/run_v255_cross_method_candidate_smoke.ps1",
)
INSTALLED_WRAPPER, CROSS_METHOD_WRAPPER = WRAPPERS


def run_cleanup_behavior_contract(wrapper: Path) -> subprocess.CompletedProcess[str]:
    script = r"""
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$wrapper = $env:QUICKPLS_CLEANUP_CONTRACT_WRAPPER
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($wrapper, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw "Wrapper parse failed: $($parseErrors.Message -join '; ')" }
foreach ($functionName in @("Test-OwnedProcessIdentity", "Update-OwnedTreeSnapshot")) {
    $definition = @($ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $functionName
    }, $true))
    if ($definition.Count -ne 1) { throw "Expected one $functionName definition." }
    Invoke-Expression $definition[0].Extent.Text
}

$script:ownedCreation = ([DateTime]"2026-08-22T06:18:41Z").ToUniversalTime().ToString("o")
$script:cimMode = "exact"
function Get-CimInstance {
    param([string]$ClassName, [string]$Filter, $ErrorAction)
    if ($script:cimMode -eq "absent") { return $null }
    $creation = if ($script:cimMode -eq "reused") { [DateTime]"2026-08-22T06:18:42Z" } else { [DateTime]"2026-08-22T06:18:41Z" }
    $executable = if ($script:cimMode -eq "other_executable") { "C:\Other\unrelated.exe" } else { "C:\QuickPLS\candidate.exe" }
    [pscustomobject]@{ ProcessId = 380; CreationDate = $creation; ExecutablePath = $executable }
}

$owned = [pscustomobject]@{ pid = 380; parent_pid = 1; creation_time = $script:ownedCreation; executable = "C:\QuickPLS\candidate.exe" }
if ((Test-OwnedProcessIdentity $owned) -ne $true) { throw "Exact captured identity was not recognized." }
foreach ($mode in @("reused", "other_executable", "absent")) {
    $script:cimMode = $mode
    if (Test-OwnedProcessIdentity $owned) { throw "A $mode PID was still classified as wrapper-owned." }
}
$script:cimMode = "exact"
$incomplete = [pscustomobject]@{ pid = 380; parent_pid = 1; creation_time = $script:ownedCreation; executable = $null }
if (Test-OwnedProcessIdentity $incomplete) { throw "An incomplete captured identity was classified as wrapper-owned." }

$script:treeCalls = 0
$script:treeMode = "retry"
function Get-ProcessTree {
    param([int]$RootPid)
    $script:treeCalls += 1
    if ($script:treeMode -eq "forbidden") { throw "Exited-root traversal occurred." }
    if ($script:treeMode -eq "always_incomplete") {
        return [pscustomobject]@{ pid = $RootPid; parent_pid = 1; creation_time = $null; executable = $null }
    }
    if ($script:treeCalls -eq 1) {
        return @(
            [pscustomobject]@{ pid = $RootPid; parent_pid = 1; creation_time = $null; executable = $null },
            [pscustomobject]@{ pid = 381; parent_pid = $RootPid; creation_time = $null; executable = $null }
        )
    }
    return @(
        [pscustomobject]@{ pid = $RootPid; parent_pid = 1; creation_time = $script:ownedCreation; executable = "C:\QuickPLS\candidate.exe" },
        [pscustomobject]@{ pid = 381; parent_pid = $RootPid; creation_time = $script:ownedCreation; executable = "C:\QuickPLS\webview.exe" }
    )
}

$live = [pscustomobject]@{
    Id = 380
    HasExited = $false
    QuickPlsOwnedTree = @(
        [pscustomobject]@{ pid = 380; parent_pid = 1; creation_time = $null; executable = $null },
        [pscustomobject]@{ pid = 381; parent_pid = 380; creation_time = $null; executable = $null }
    )
}
$updated = @(Update-OwnedTreeSnapshot $live)
if ($script:treeCalls -ne 2) { throw "Complete root identity was not retried exactly once in the contract fixture." }
$updatedRoot = @($updated | Where-Object { $_.pid -eq 380 })[0]
$updatedChild = @($updated | Where-Object { $_.pid -eq 381 })[0]
if (-not $updatedRoot.creation_time -or -not $updatedRoot.executable -or -not $updatedChild.creation_time -or -not $updatedChild.executable) {
    throw "Incomplete cached identities were not upgraded from the complete live tree."
}

$script:treeCalls = 0
$script:treeMode = "always_incomplete"
$unverifiedRootRejected = $false
try {
    $null = Update-OwnedTreeSnapshot ([pscustomobject]@{ Id = 382; HasExited = $false })
} catch {
    $unverifiedRootRejected = $_.Exception.Message -like "*complete creation/executable identity*"
}
if (-not $unverifiedRootRejected -or $script:treeCalls -ne 10) {
    throw "A live root without complete identity was not rejected after bounded retries."
}

$script:treeCalls = 0
$script:treeMode = "forbidden"
$exited = [pscustomobject]@{ Id = 380; HasExited = $true; QuickPlsOwnedTree = @($owned) }
$savedOnly = @(Update-OwnedTreeSnapshot $exited)
if ($script:treeCalls -ne 0 -or $savedOnly.Count -ne 1 -or $savedOnly[0].pid -ne 380) {
    throw "Exited root did not return only its previously captured tree."
}
"passed"
"""
    return subprocess.run(
        [
            "powershell.exe",
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
        env={**os.environ, "QUICKPLS_CLEANUP_CONTRACT_WRAPPER": str(wrapper)},
    )


def run_report_behavior_contract(
    wrapper: Path, shell: str, output: Path
) -> subprocess.CompletedProcess[str]:
    script = r"""
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest
$wrapper = $env:QUICKPLS_REPORT_CONTRACT_WRAPPER
$tokens = $null
$parseErrors = $null
$ast = [Management.Automation.Language.Parser]::ParseFile($wrapper, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw "Wrapper parse failed: $($parseErrors.Message -join '; ')" }
foreach ($functionName in @("Test-ExactEmptyArrayProperty", "Write-Utf8NoBom")) {
    $definition = @($ast.FindAll({
        param($node)
        $node -is [Management.Automation.Language.FunctionDefinitionAst] -and $node.Name -eq $functionName
    }, $true))
    if ($definition.Count -ne 1) { throw "Expected one $functionName definition." }
    Invoke-Expression $definition[0].Extent.Text
}

$valid = '{"console_errors":[]}' | ConvertFrom-Json
if (-not (Test-ExactEmptyArrayProperty $valid "console_errors")) { throw "Exact empty JSON array was rejected." }
foreach ($invalidJson in @(
    '{}',
    '{"console_errors":null}',
    '{"console_errors":{}}',
    '{"console_errors":[{"type":"pageerror","message":"boom"}]}'
)) {
    $invalid = $invalidJson | ConvertFrom-Json
    if (Test-ExactEmptyArrayProperty $invalid "console_errors") { throw "Non-exact empty array was accepted: $invalidJson" }
}

Write-Utf8NoBom $env:QUICKPLS_REPORT_CONTRACT_OUTPUT '{"passed":true}'
"passed"
"""
    return subprocess.run(
        [
            shell,
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
        timeout=30,
        env={
            **os.environ,
            "QUICKPLS_REPORT_CONTRACT_WRAPPER": str(wrapper),
            "QUICKPLS_REPORT_CONTRACT_OUTPUT": str(output),
        },
    )


class V255PackagedProcessCleanupContractTests(unittest.TestCase):
    def test_identity_and_exited_root_behavior_is_fail_closed(self) -> None:
        for wrapper in WRAPPERS:
            with self.subTest(wrapper=wrapper.name):
                result = run_cleanup_behavior_contract(wrapper)
                self.assertEqual(0, result.returncode, result.stderr or result.stdout)
                self.assertIn("passed", result.stdout)

    def test_every_taskkill_remains_behind_exact_identity_or_process_handle_scope(self) -> None:
        for wrapper in WRAPPERS:
            with self.subTest(wrapper=wrapper.name):
                source = wrapper.read_text(encoding="utf-8")
                self.assertIn("if (Test-OwnedProcessIdentity $row)", source)
                self.assertIn(
                    "if ($rootRow.Count -ne 1 -or -not (Test-OwnedProcessIdentity $rootRow[0]))",
                    source,
                )
                self.assertNotIn("no longer has the captured", source)
                self.assertIn("if (-not $Process.HasExited)", source)

    def test_installed_wrapper_preserves_operation_and_cleanup_failures(self) -> None:
        source = INSTALLED_WRAPPER.read_text(encoding="utf-8")
        self.assertIn("$candidateOutcome = $null", source)
        self.assertIn("$operationFailure = $null", source)
        self.assertIn("$cleanupFailure = $null", source)
        self.assertIn("$candidateOutcome = [ordered]@{", source)
        self.assertIn("$operationFailure = $_", source)
        self.assertIn("$cleanupFailure = $_", source)
        self.assertIn(
            'throw "$Name candidate operation failed: $($operationFailure.Exception.Message); exact cleanup also failed: $($cleanupFailure.Exception.Message)"',
            source,
        )
        self.assertIn("if ($operationFailure) { throw $operationFailure }", source)
        self.assertIn("return $candidateOutcome", source)

    def test_installed_wrapper_uses_fresh_exact_pids_for_serial_attach_phases(self) -> None:
        source = INSTALLED_WRAPPER.read_text(encoding="utf-8")
        method_call = source.index("& $node $driver @crawlerArguments")
        method_console_guard = source.index(
            'Test-ExactEmptyArrayProperty $methodReport "console_errors"', method_call
        )
        method_stop = source.index(
            "Stop-IsolatedCandidate $process $endpoint", method_console_guard
        )
        method_null = source.index("$process = $null", method_stop)
        portable_guard = source.index('if ($Name -eq "portable") {', method_null)
        frozen_start = source.index(
            "$process = Start-IsolatedCandidate $candidateFull $endpoint",
            portable_guard,
        )
        frozen_pid = source.index("$launchedPids.Add($process.Id)", frozen_start)
        frozen_call = source.index("& $node @frozenArguments", frozen_pid)
        frozen_console_guard = source.index(
            'Test-ExactEmptyArrayProperty $aggregate "console_errors"', frozen_call
        )
        frozen_stop = source.index(
            "Stop-IsolatedCandidate $process $endpoint", frozen_console_guard
        )
        frozen_null = source.index("$process = $null", frozen_stop)
        named_guard = source.index("if ($namedCaseManifestReady) {", frozen_null)
        named_start = source.index(
            "$process = Start-IsolatedCandidate $candidateFull $endpoint", named_guard
        )
        named_pid = source.index("$launchedPids.Add($process.Id)", named_start)
        named_call = source.index("& $node $namedCaseDriver", named_pid)
        named_console_guard = source.index(
            'Test-ExactEmptyArrayProperty $namedCaseReport "console_errors"',
            named_call,
        )
        self.assertEqual(
            sorted(
                [
                    method_call,
                    method_console_guard,
                    method_stop,
                    method_null,
                    portable_guard,
                    frozen_start,
                    frozen_pid,
                    frozen_call,
                    frozen_console_guard,
                    frozen_stop,
                    frozen_null,
                    named_guard,
                    named_start,
                    named_pid,
                    named_call,
                    named_console_guard,
                ]
            ),
            [
                method_call,
                method_console_guard,
                method_stop,
                method_null,
                portable_guard,
                frozen_start,
                frozen_pid,
                frozen_call,
                frozen_console_guard,
                frozen_stop,
                frozen_null,
                named_guard,
                named_start,
                named_pid,
                named_call,
                named_console_guard,
            ],
        )

    def test_renderer_error_receipts_are_exact_arrays_and_cross_report_is_no_bom(self) -> None:
        installed = INSTALLED_WRAPPER.read_text(encoding="utf-8")
        cross = CROSS_METHOD_WRAPPER.read_text(encoding="utf-8")
        for payload in (
            "$lifecycleReport",
            "$methodReport",
            "$posthocExecute",
            "$posthocReopen",
            "$aggregate",
            "$namedCaseReport",
            "$crossMethodReport",
        ):
            self.assertIn(
                f'Test-ExactEmptyArrayProperty {payload} "console_errors"', installed
            )
        self.assertGreaterEqual(
            cross.count('Test-ExactEmptyArrayProperty $report "console_errors"'), 1
        )
        generic_renderer_phases = (
            "imports",
            "exports",
            "archives",
            "legacy_reopen",
            "autosave_seed",
            "autosave_recover",
            "unsaved_close_seed",
        )
        self.assertEqual(
            len(generic_renderer_phases),
            cross.count('$active = Invoke-Driver -Phase "'),
        )
        for phase in generic_renderer_phases:
            self.assertIn(f'$active = Invoke-Driver -Phase "{phase}"', cross)
        self.assertIn(
            'Test-ExactEmptyArrayProperty $dpiReport "console_errors"', cross
        )
        self.assertIn(
            '$closeReport.suite_id -ne "quickpls_v255_windows_unsaved_close_guard_v1"',
            cross,
        )
        self.assertIn('$closeReport.candidate.pid -ne $active.process.Id', cross)
        self.assertIn(
            '$closeReport.candidate.sha256 -ne $script:portableHash', cross
        )
        self.assertNotIn(
            'Test-ExactEmptyArrayProperty $closeReport "console_errors"', cross
        )
        self.assertIn("console_errors = @()", cross)
        self.assertIn("Write-Utf8NoBom $reportPath", cross)
        self.assertNotIn(
            "Set-Content -LiteralPath $reportPath -Encoding UTF8", cross
        )

    def test_exact_empty_array_and_no_bom_helpers_work_in_ps5_and_ps7(self) -> None:
        shells = [shell for shell in ("powershell.exe", "pwsh.exe") if shutil.which(shell)]
        self.assertIn("powershell.exe", shells)
        for wrapper in WRAPPERS:
            for shell in shells:
                with self.subTest(wrapper=wrapper.name, shell=shell):
                    with tempfile.TemporaryDirectory() as temporary:
                        output = Path(temporary) / "report.json"
                        result = run_report_behavior_contract(wrapper, shell, output)
                        self.assertEqual(
                            0, result.returncode, result.stderr or result.stdout
                        )
                        self.assertIn("passed", result.stdout)
                        self.assertEqual(b'{"passed":true}', output.read_bytes())


if __name__ == "__main__":
    unittest.main()
