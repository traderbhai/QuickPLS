#!/usr/bin/env python3
"""Static contracts for the packaged installed/portable hard time budget."""

from __future__ import annotations

import json
import re
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
WRAPPER = HERE / "run_multimod_packaged_offline_smoke_v1.ps1"
DRIVER = HERE / "multimod_packaged_smoke_driver_v1.mjs"
SCHEMA = HERE / "multimod_runtime_promotion_smoke_v1.schema.json"
BINDINGS = HERE / "multimod_gate_bindings_v1.json"


def assigned_integer(source: str, variable: str) -> int:
    match = re.search(rf"^\${re.escape(variable)}\s*=\s*(\d+)\s*$", source, re.MULTILINE)
    if match is None:
        raise AssertionError(f"missing integer assignment for ${variable}")
    return int(match.group(1))


class PackagedOfflineTimeBudgetContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.wrapper = WRAPPER.read_text(encoding="utf-8")
        cls.driver = DRIVER.read_text(encoding="utf-8")
        cls.schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        cls.bindings = json.loads(BINDINGS.read_text(encoding="utf-8"))

    def test_wrapper_reserves_cleanup_and_finalization_inside_6480_seconds(self) -> None:
        wrapper_maximum = assigned_integer(self.wrapper, "wrapperMaximumSeconds")
        cleanup_reserve = assigned_integer(self.wrapper, "minimumCleanupReserveSeconds")
        finalization_reserve = assigned_integer(self.wrapper, "finalizationReserveSeconds")
        self.assertLessEqual(wrapper_maximum, 6480)
        self.assertGreaterEqual(cleanup_reserve, 1020)
        self.assertGreater(finalization_reserve, 0)
        self.assertLess(cleanup_reserve + finalization_reserve, wrapper_maximum)
        self.assertEqual(
            wrapper_maximum - cleanup_reserve - finalization_reserve,
            5340,
        )
        self.assertIn(
            "$scientificMaximumSeconds = $wrapperMaximumSeconds - $postScienceReserveSeconds",
            self.wrapper,
        )
        self.assertIn(
            "$cleanupDeadlineSeconds = $wrapperMaximumSeconds - $finalizationReserveSeconds",
            self.wrapper,
        )
        self.assertIn('Save-SupervisedProcessLogs -Job $Job -Phase "science"', self.wrapper)
        self.assertIn('Save-SupervisedProcessLogs -Job $Job -Phase "cleanup"', self.wrapper)
        self.assertIn('Assert-PhaseBudget -Phase "wrapper"', self.wrapper)

    def test_node_and_candidate_trees_are_supervised_before_uninstall(self) -> None:
        for fragment in (
            "[Diagnostics.ProcessStartInfo]::new()",
            "$Process.Kill($true)",
            "function Wait-SupervisedDriver",
            "function Stop-SupervisedDriver",
            '"--scientific-deadline-epoch-ms"',
            "active_work_cancelled_via_candidate_termination",
            "candidate_termination_before_uninstall",
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, self.wrapper)
        cleanup = self.wrapper[self.wrapper.index("$cleanupClock =") :]
        self.assertLess(
            cleanup.index('Stop-SupervisedDriver -Job $nodeJob'),
            cleanup.index('Stop-ExactProcessTree -Process $process'),
        )
        self.assertLess(
            cleanup.index('Stop-ExactProcessTree -Process $process'),
            cleanup.index('$uninstall = Start-Process'),
        )
        self.assertNotIn("taskkill.exe", self.wrapper)

    def test_driver_clamps_every_job_poll_and_rejects_late_completion(self) -> None:
        for fragment in (
            '"scientific-deadline-epoch-ms"',
            "const deadline = Math.min(familyDeadlineEpochMs, scientificDeadlineEpochMs);",
            "remainingScientificMilliseconds(operation)",
            "completedEpochMs <= scientificDeadlineEpochMs",
            "poll_deadlines_clamped_to_family_and_driver: true",
            "late_exit_rejection_enabled: true",
        ):
            with self.subTest(fragment=fragment):
                self.assertIn(fragment, self.driver)

    def test_runtime_receipt_schema_requires_timing_and_cleanup_provenance(self) -> None:
        required = set(self.schema["required"])
        self.assertIn("timing_provenance", required)
        self.assertIn("cleanup_provenance", required)
        timing = self.schema["$defs"]["timingProvenance"]
        cleanup = self.schema["$defs"]["cleanupProvenance"]
        self.assertEqual(timing["properties"]["wrapper_maximum_seconds"]["const"], 6480)
        self.assertEqual(timing["properties"]["scientific_maximum_seconds"]["const"], 5340)
        self.assertEqual(timing["properties"]["minimum_cleanup_reserve_seconds"]["const"], 1020)
        self.assertEqual(timing["properties"]["finalization_reserve_seconds"]["const"], 120)
        for field in (
            "node_process_tree_supervised",
            "node_process_tree_terminated",
            "candidate_process_tree_termination_requested",
            "candidate_termination_before_uninstall",
        ):
            self.assertEqual(cleanup["properties"][field]["const"], True)

    def test_installed_and_portable_binding_caps_are_6600_seconds(self) -> None:
        by_id = {gate["gate_id"]: gate for gate in self.bindings["gates"]}
        for gate_id in ("installed.offline.smoke", "portable.offline.smoke"):
            with self.subTest(gate_id=gate_id):
                gate = by_id[gate_id]
                self.assertEqual(len(gate["steps"]), 1)
                self.assertEqual(gate["steps"][0]["maximum_seconds"], 6600)
                self.assertIn(
                    "validation/multimod/test_multimod_packaged_offline_time_budget_v1.py",
                    gate["input_artifacts"],
                )


if __name__ == "__main__":
    unittest.main()
