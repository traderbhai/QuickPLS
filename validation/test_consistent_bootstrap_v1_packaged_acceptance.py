#!/usr/bin/env python3
"""Focused source-contract checks for PLSc-bootstrap packaged acceptance."""

from __future__ import annotations

import hashlib
import json
import math
import os
import shutil
import subprocess
import unittest
from unittest.mock import Mock

try:
    from validation.consistent_bootstrap_v1_packaged_acceptance import (
        EXPECTED_CHECK_IDS,
        MANIFEST,
        ROOT,
        AdapterError,
        finite_parameter_map,
        read_network_samples,
    )
except ModuleNotFoundError:
    from consistent_bootstrap_v1_packaged_acceptance import (  # type: ignore[no-redef]
        EXPECTED_CHECK_IDS,
        MANIFEST,
        ROOT,
        AdapterError,
        finite_parameter_map,
        read_network_samples,
    )


class ConsistentBootstrapPackagedContractTests(unittest.TestCase):
    def test_supervisor_imports_and_qualifies_receipt_hashing_before_launch(self) -> None:
        wrapper = (ROOT / "validation/run_v247_plsc_bootstrap_native_acceptance.ps1").read_text(encoding="utf-8")
        module_import = "Import-Module Microsoft.PowerShell.Utility -MaximumVersion 5.1 -ErrorAction Stop"
        qualified_hash = "Microsoft.PowerShell.Utility\\Get-FileHash"
        self.assertEqual(wrapper.count(module_import), 1)
        self.assertEqual(wrapper.count(qualified_hash), 1)
        self.assertLess(wrapper.index(module_import), wrapper.index("$application = Start-Process"))

    @unittest.skipUnless(os.name == "nt" and shutil.which("powershell.exe"), "Windows PowerShell is required")
    def test_real_descriptor_hashes_deterministically_in_a_nested_pipeline(self) -> None:
        wrapper_path = ROOT / "validation/run_v247_plsc_bootstrap_native_acceptance.ps1"
        environment = os.environ.copy()
        environment["QUICKPLS_HASH_SMOKE_WRAPPER"] = str(wrapper_path)
        command = r"""
$ErrorActionPreference = 'Stop'
Import-Module Microsoft.PowerShell.Management -ErrorAction Stop
Import-Module Microsoft.PowerShell.Utility -MaximumVersion 5.1 -ErrorAction Stop
$wrapperPath = [System.IO.Path]::GetFullPath($env:QUICKPLS_HASH_SMOKE_WRAPPER)
$repositoryRoot = [System.IO.Path]::GetFullPath(
    [System.IO.Path]::GetDirectoryName([System.IO.Path]::GetDirectoryName($wrapperPath))
)
$tokens = $null
$parseErrors = $null
$ast = [System.Management.Automation.Language.Parser]::ParseFile($wrapperPath, [ref]$tokens, [ref]$parseErrors)
if ($parseErrors.Count -ne 0) { throw ($parseErrors | ForEach-Object Message) -join '; ' }
$descriptorFunction = $ast.Find({
    param($node)
    $node -is [System.Management.Automation.Language.FunctionDefinitionAst] `
        -and $node.Name -eq 'Get-ArtifactDescriptor'
}, $true)
if ($null -eq $descriptorFunction) { throw 'Get-ArtifactDescriptor was not found.' }
Microsoft.PowerShell.Utility\Invoke-Expression $descriptorFunction.Extent.Text
$descriptors = @(1..3 | ForEach-Object { Get-ArtifactDescriptor -Path $wrapperPath })
[pscustomobject]@{
    hashes = @($descriptors | ForEach-Object sha256)
    sizes = @($descriptors | ForEach-Object size)
} | Microsoft.PowerShell.Utility\ConvertTo-Json -Compress
"""
        completed = subprocess.run(
            ["powershell.exe", "-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", command],
            cwd=ROOT,
            env=environment,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(completed.returncode, 0, completed.stderr or completed.stdout)
        result = json.loads(completed.stdout.strip())
        expected_hash = hashlib.sha256(wrapper_path.read_bytes()).hexdigest()
        self.assertEqual(result["hashes"], [expected_hash] * 3)
        self.assertEqual(result["sizes"], [wrapper_path.stat().st_size] * 3)

    def test_witness_parameter_maps_fail_closed(self) -> None:
        self.assertTrue(finite_parameter_map({"path:[\"X\",\"Y\"]": 0.42}))
        self.assertFalse(finite_parameter_map({}))
        self.assertFalse(finite_parameter_map({"path": math.nan}))
        self.assertFalse(finite_parameter_map({"": 0.42}))
        self.assertFalse(finite_parameter_map({"path": True}))

    def test_network_observation_records_platform_egress_without_promoting_zero_egress(self) -> None:
        clean = Mock()
        clean.read_text.return_value = json.dumps({
            "root_present": True,
            "observation": "sampled_exact_process_tree_tcp_v1",
            "remote_connections": [],
        }) + "\n"
        clean_observation = read_network_samples(clean)
        self.assertFalse(clean_observation["platform_background_egress_observed"])
        self.assertTrue(clean_observation["commercial_zero_egress_passed"])

        remote = Mock()
        remote.read_text.return_value = json.dumps({
            "root_present": True,
            "observation": "sampled_exact_process_tree_tcp_v1",
            "remote_connections": [{"remote_address": "52.110.15.135", "remote_port": 443}],
        }) + "\n"
        observed = read_network_samples(remote)
        self.assertTrue(observed["passed"])
        self.assertTrue(observed["platform_background_egress_observed"])
        self.assertFalse(observed["commercial_zero_egress_passed"])
        self.assertEqual(observed["remote_connections"], [{"remote_address": "52.110.15.135", "remote_port": 443}])

        malformed = Mock()
        malformed.read_text.return_value = json.dumps({
            "root_present": True,
            "observation": "sampled_exact_process_tree_tcp_v1",
        }) + "\n"
        with self.assertRaises(AdapterError):
            read_network_samples(malformed)

    def test_manifest_binds_the_method_scoped_package_contract(self) -> None:
        document = json.loads(MANIFEST.read_text(encoding="utf-8"))
        sources = set(document["qualification"]["source_requirements"]["packaged_acceptance"])
        self.assertEqual(
            EXPECTED_CHECK_IDS,
            tuple(sorted(EXPECTED_CHECK_IDS)),
        )
        self.assertTrue({
            "validation/consistent_bootstrap_v1_packaged_acceptance.py",
            "validation/run_v247_plsc_bootstrap_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            "validation/monitor_quickpls_network.ps1",
        } <= sources)

    def test_final_audit_gates_truthful_platform_disclosure_not_zero_egress(self) -> None:
        audit = (ROOT / "validation/consistent_bootstrap_v1_factory_audit.py").read_text(encoding="utf-8")
        self.assertNotIn("functional_and_process_offline", audit)
        self.assertIn('"method_functional_offline"', audit)
        self.assertIn('"platform_background_egress_recorded_truthfully"', audit)
        self.assertIn('"commercial_zero_egress_passed": commercial_zero_egress_passed', audit)

    def test_harness_and_supervisor_cover_the_frozen_packaged_flow(self) -> None:
        harness = (ROOT / "validation/v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        wrapper = (ROOT / "validation/run_v247_plsc_bootstrap_native_acceptance.ps1").read_text(encoding="utf-8")
        for token in (
            "runFocusedPlscBootstrapAcceptance",
            "plscBootstrapInvalidSetup",
            "does not contain any constructs",
            "does not contain any assigned indicators",
            "plscBootstrapCancellation",
            "Attempted preplanned full-PLSc refits",
            "Replayable successful-refit witnesses",
            "jackknifeFailureDisclosure",
            "plscBootstrapExport",
            "plscBootstrapSaveReopen",
            "plscBootstrapPackagedViewports",
            "plscBootstrapFunctionalOffline",
        ):
            self.assertIn(token, harness)
        for token in (
            "QUICKPLS_ACCEPTANCE_SCOPE = \"plsc_bootstrap\"",
            "[AllowEmptyString()][string]$Value",
            "monitor_quickpls_network.ps1",
            "sampled_process_tree_zero_egress",
            "platform_background_egress_observation",
            "commercial_zero_egress_passed",
            "graceful_process_cleanup_verified",
            "forced_process_cleanup_used",
        ):
            self.assertIn(token, wrapper)
        focused = harness[harness.index("async function runFocusedPlscBootstrapAcceptance"):harness.index("\ntry {", harness.index("async function runFocusedPlscBootstrapAcceptance"))]
        self.assertIn("passed: externalRequests.length === 0", focused)
        self.assertIn("strictZeroProcessEgressClaimed: false", focused)

    def test_cancellation_is_prearmed_and_requested_before_screenshot_capture(self) -> None:
        harness = (ROOT / "validation/v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        start = harness.index("const cancellationArchiveBefore = await inspectMediationArchiveRunState(plscBootstrapProjectPath);")
        end = harness.index("const cancelled = cancellationSetup.dialog.locator", start)
        cancellation = harness[start:end]
        self.assertIn("const plscBootstrapSamples = 10_000;", harness)
        self.assertIn("const plscBootstrapCancellationSamples = plscBootstrapSamples;", harness)
        self.assertIn("const cancellationTerminalStatePromise = page.waitForFunction", cancellation)
        self.assertIn("const cancellationRequestPromise = page.waitForFunction", cancellation)
        self.assertIn("cancelButtons[0].click();", cancellation)
        self.assertIn('outcome: "cancel_requested"', cancellation)
        self.assertIn("completion_won_race", cancellation)
        self.assertLess(cancellation.index("const cancellationRequestPromise"), cancellation.index("await cancellationSetup.start.click()"))
        self.assertLess(cancellation.index("cancelButtons[0].click();"), cancellation.index('capture("173-tauri-native-plsc-bootstrap-cancellation-running-1440x900.png")'))

    def test_provenance_is_verified_in_export_without_inventing_a_results_tree_node(self) -> None:
        harness = (ROOT / "validation/v247_tauri_native_acceptance.mjs").read_text(encoding="utf-8")
        start = harness.index("async function runFocusedPlscBootstrapAcceptance")
        end = harness.index("\ntry {", start)
        focused = harness[start:end]
        self.assertNotIn('openResultTable("Run provenance")', focused)
        self.assertIn('"Run provenance",', focused)
        self.assertIn("expectedSharedStrings", focused)
        self.assertIn("plscBootstrapMethodVersion", focused)


if __name__ == "__main__":
    unittest.main()
