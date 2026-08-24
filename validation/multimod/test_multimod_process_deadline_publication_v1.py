#!/usr/bin/env python3
"""No-build contracts for bounded MultiMod launch and package publication."""

from __future__ import annotations

import json
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
PACKAGE = HERE / "package_multimod_candidate_v1.ps1"
GATE = HERE / "invoke_multimod_gate_v1.ps1"
CAMPAIGN = HERE.parent / "run_v256_multimod_qualification.ps1"
BINDINGS = HERE / "multimod_gate_bindings_v1.json"


class ProcessDeadlinePublicationContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.package = PACKAGE.read_text(encoding="utf-8")
        cls.gate = GATE.read_text(encoding="utf-8")
        cls.campaign = CAMPAIGN.read_text(encoding="utf-8")
        cls.bindings = json.loads(BINDINGS.read_text(encoding="utf-8"))

    def test_package_has_explicit_internal_and_outer_deadlines(self) -> None:
        package_gate = next(
            gate for gate in self.bindings["gates"] if gate["gate_id"] == "package.candidate"
        )
        self.assertEqual(len(package_gate["steps"]), 1)
        step = package_gate["steps"][0]
        self.assertEqual(step["maximum_seconds"], 6600)
        timeout_index = step["arguments"].index("-OverallTimeoutSeconds")
        self.assertEqual(step["arguments"][timeout_index + 1], "6480")
        self.assertIn("[ValidateRange(600, 6480)]", self.package)
        self.assertIn("$publicationReserveSeconds = 120", self.package)
        self.assertIn(
            "$buildWorkDeadlineSeconds = $OverallTimeoutSeconds - $publicationReserveSeconds",
            self.package,
        )
        self.assertIn('-MaximumSeconds 60', self.package)

    def test_all_three_supervisors_use_exact_argument_lists_and_tree_cleanup(self) -> None:
        for name, source in (
            ("package", self.package),
            ("gate", self.gate),
            ("campaign", self.campaign),
        ):
            with self.subTest(script=name):
                self.assertNotIn("Start-Process", source)
                self.assertIn("[Diagnostics.ProcessStartInfo]::new()", source)
                self.assertIn(".ArgumentList.Add(", source)
                self.assertIn("function Stop-VerifiedProcessTree", source)
                self.assertIn("$Process.Kill($true)", source)
                self.assertIn('"/PID", $Process.Id.ToString(), "/T", "/F"', source)

    def test_npm_and_npx_shims_are_bypassed_through_node_cli(self) -> None:
        for name, source in (("package", self.package), ("gate", self.gate)):
            with self.subTest(script=name):
                self.assertIn('"npm.cmd" { "npm-cli.js" }', source)
                self.assertIn('"npx.cmd" { "npx-cli.js" }', source)
                self.assertIn('LaunchKind = "node_cli_argument_list"', source)
        self.assertIn("node_modules\\npm\\bin", self.package)
        self.assertIn("node_modules\\npm\\bin", self.gate)
        self.assertIn("'{\"build\":{\"beforeBuildCommand\":\"\"}}'", self.package)
        self.assertNotIn("cmd.exe", self.package.lower())
        self.assertNotIn("cmd.exe", self.gate.lower())

    def test_gate_clock_and_evidence_deadline_contract_are_fail_closed(self) -> None:
        self.assertLess(
            self.gate.index("$gateClock = [Diagnostics.Stopwatch]::StartNew()"),
            self.gate.index("Get-Content -LiteralPath $catalogPath"),
        )
        self.assertLess(
            self.gate.index("sha256 = Get-LowerSha256 -Path $resolvedOutput"),
            self.gate.index("$budgetExceeded = $timedOut"),
        )
        for fragment in (
            'launch_kind = "not_started_gate_budget_exhausted"',
            "effective_maximum_seconds = 0",
            "gate_budget_limited = $true",
            "gate_deadline_checked_after_evidence_hashing = $true",
            "$prestartExpectedOutputs",
            "$prestartMissingOutputs",
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, self.gate)
        self.assertIn("step_not_started_contract", self.campaign)
        self.assertIn("step_execution_deadline_contract", self.campaign)

    def test_package_publication_is_one_atomic_directory_then_receipt(self) -> None:
        rename_index = self.package.index(
            "[IO.Directory]::Move($stagingFull, $finalCandidateDirectory)"
        )
        receipt_index = self.package.index(
            "Write-JsonAtomic -Path $outputPath -Value $receipt"
        )
        self.assertLess(rename_index, receipt_index)
        self.assertIn("_attempt_history\\package-", self.package)
        self.assertIn("$candidateDirectoryName", self.package)
        self.assertIn("package_receipt_is_commit_marker = $true", self.package)
        self.assertEqual(
            self.package.rstrip().splitlines()[-1].strip(),
            "Write-JsonAtomic -Path $outputPath -Value $receipt",
        )

    def test_campaign_deadline_is_capped_and_detects_late_completion(self) -> None:
        self.assertIn("$maximumQualificationStepSeconds = 7200", self.campaign)
        self.assertIn("$campaignCleanupReserveMilliseconds = 60000L", self.campaign)
        self.assertIn(
            "$campaignGateClock.Elapsed.TotalMilliseconds -ge $outerBudgetMilliseconds",
            self.campaign,
        )
        for gate in self.bindings["gates"]:
            for step in gate["steps"]:
                self.assertGreaterEqual(step["maximum_seconds"], 1)
                self.assertLessEqual(step["maximum_seconds"], 7200)


if __name__ == "__main__":
    unittest.main()
