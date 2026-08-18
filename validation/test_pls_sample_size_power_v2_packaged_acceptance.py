#!/usr/bin/env python3
"""Fast source-contract checks for prospective PLS power v2 packaging."""

from __future__ import annotations

import json
import unittest
from unittest.mock import Mock

try:
    from validation.pls_sample_size_power_v2_packaged_acceptance import (
        AdapterError,
        EXPECTED_CHECK_IDS,
        HARNESS,
        MANIFEST,
        WRAPPER,
        read_network_samples,
    )
except ModuleNotFoundError:
    from pls_sample_size_power_v2_packaged_acceptance import (  # type: ignore[no-redef]
        AdapterError,
        EXPECTED_CHECK_IDS,
        HARNESS,
        MANIFEST,
        WRAPPER,
        read_network_samples,
    )


class PlsSampleSizePowerV2PackagedContractTests(unittest.TestCase):
    def test_supervisor_imports_and_qualifies_receipt_hashing_before_launch(self) -> None:
        wrapper = WRAPPER.read_text(encoding="utf-8")
        module_import = "Import-Module Microsoft.PowerShell.Utility -MaximumVersion 5.1 -ErrorAction Stop"
        qualified_hash = "Microsoft.PowerShell.Utility\\Get-FileHash"
        self.assertEqual(wrapper.count(module_import), 1)
        self.assertEqual(wrapper.count(qualified_hash), 1)
        self.assertLess(wrapper.index(module_import), wrapper.index("$application = Start-Process"))

    def test_scoped_check_family_is_exact_and_sorted(self) -> None:
        self.assertEqual(EXPECTED_CHECK_IDS, tuple(sorted(EXPECTED_CHECK_IDS)))
        self.assertEqual(len(EXPECTED_CHECK_IDS), len(set(EXPECTED_CHECK_IDS)))
        self.assertIn("plsSampleSizePowerCancellation", EXPECTED_CHECK_IDS)
        self.assertIn("plsSampleSizePowerPackagedViewports", EXPECTED_CHECK_IDS)

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

    def test_manifest_binds_v2_without_relabeling_v1(self) -> None:
        document = json.loads(MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(document["feature"]["method_version"], "pls_sample_size_power_v2")
        sources = set(document["qualification"]["source_requirements"]["packaged_acceptance"])
        self.assertTrue({
            "validation/pls_sample_size_power_v2_packaged_acceptance.py",
            "validation/run_v247_pls_sample_size_power_native_acceptance.ps1",
            "validation/v247_tauri_native_acceptance.mjs",
            "validation/monitor_quickpls_network.ps1",
            "validation/test_pls_sample_size_power_v2_packaged_acceptance.py",
        } <= sources)
        self.assertEqual(document["product_contract"]["persistence"]["legacy_policy"],
                         "pls_sample_size_power_v1 remains readable under its historical identity and is non-executable; only exact v2 recipes may create new results.")

    def test_final_audit_gates_truthful_platform_disclosure_not_zero_egress(self) -> None:
        audit = (MANIFEST.parents[1] / "pls_sample_size_power_v2_factory_audit.py").read_text(encoding="utf-8")
        self.assertNotIn("functional_and_process_offline", audit)
        self.assertIn('"method_functional_offline"', audit)
        self.assertIn('"platform_background_egress_recorded_truthfully"', audit)
        self.assertIn('"commercial_zero_egress_passed": commercial_zero_egress_passed', audit)

    def test_harness_and_supervisor_cover_the_real_v2_workflow(self) -> None:
        harness = HARNESS.read_text(encoding="utf-8")
        wrapper = WRAPPER.read_text(encoding="utf-8")
        for token in (
            "runFocusedPlsSampleSizePowerAcceptance",
            "plsSampleSizePowerInvalidSetup",
            "plsSampleSizePowerCancellation",
            "Bootstrap tail accounting",
            "outerAccountingCloses",
            "tailAccountingCloses",
            "plsSampleSizePowerExport",
            "plsSampleSizePowerSaveReopen",
            "plsSampleSizePowerPackagedViewports",
            "plsSampleSizePowerFunctionalOffline",
            "inspectSavedPlsSampleSizePowerArchive",
        ):
            self.assertIn(token, harness)
        for token in (
            '$env:QUICKPLS_ACCEPTANCE_SCOPE = "pls_sample_size_power"',
            "[AllowEmptyString()][string]$Value",
            "monitor_quickpls_network.ps1",
            "sampled_process_tree_zero_egress",
            "platform_background_egress_observation",
            "commercial_zero_egress_passed",
            "graceful_process_cleanup_verified",
            "forced_process_cleanup_used",
        ):
            self.assertIn(token, wrapper)
        focused = harness[harness.index("async function runFocusedPlsSampleSizePowerAcceptance"):harness.index("async function runFocusedPlscBootstrapAcceptance")]
        self.assertIn("passed: externalRequests.length === 0", focused)
        self.assertIn("strictZeroProcessEgressClaimed: false", focused)

    def test_reopened_viewport_binds_the_last_selected_tail_accounting_table(self) -> None:
        harness = HARNESS.read_text(encoding="utf-8")
        start = harness.index("const reopenedPowerRows = await openResultTable", harness.index("async function runFocusedPlsSampleSizePowerAcceptance"))
        end = harness.index("const internalOrigins = new Set", start)
        reopened = harness[start:end]
        self.assertLess(
            reopened.index('openResultTable("Power by sample size")'),
            reopened.index('openResultTable("Bootstrap tail accounting")'),
        )
        self.assertIn('checkName: "plsSampleSizePowerPackagedViewports"', reopened)
        self.assertIn('expectedTableId: "pls_power_bootstrap_tail_accounting"', reopened)


if __name__ == "__main__":
    unittest.main()
