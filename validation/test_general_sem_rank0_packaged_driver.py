from __future__ import annotations

import argparse
import unittest
from pathlib import Path

from validation.windows_native_owned_file_dialog import GateFailure, validate_target


ROOT = Path(__file__).resolve().parents[1]


class GeneralSemRank0PackagedDriverContracts(unittest.TestCase):
    def test_all_six_formats_use_native_publication_and_save_cancel_is_zero_file(
        self,
    ) -> None:
        source = (
            ROOT / "validation/general_sem_rank0_packaged_acceptance.mjs"
        ).read_text(encoding="utf-8")
        self.assertIn(
            'for (const format of ["csv", "xlsx", "html", "pdf", "svg", "png"])',
            source,
        )
        self.assertNotIn('waitForEvent("download"', source)
        self.assertIn('mode: "save-cancel"', source)
        self.assertIn("cancelledBeforePublication", source)
        self.assertIn(".toLowerCase().includes", source)
        self.assertIn(
            'feedbackText.includes("Semantic readback passed before publication.")',
            source,
        )
        self.assertIn('workspace.locator(".nd-cbsem-v4-results")', source)
        self.assertNotIn('.locator(".nd-canonical-result-v2")', source)
        self.assertIn("publication?.file?.sha256 !== file.sha256", source)

    def test_all_four_cells_cancel_with_latency_and_exact_schema6_archive_identity(
        self,
    ) -> None:
        source = (
            ROOT / "validation/general_sem_rank0_packaged_acceptance.mjs"
        ).read_text(encoding="utf-8")
        for required in (
            "terminalLatencySeconds",
            "jobCompletedBeforeCancel",
            "archiveBefore.byte_length === archiveAfter.byte_length",
            "archiveBefore.sha256 === archiveAfter.sha256",
            "archiveBefore.canonical_result_attachment_count === archiveAfter.canonical_result_attachment_count",
            "exactSameSettingsRetry",
            "const cancellation = await cancelAndVerify",
            "variant.bootstrap ? 1_000 : 100_000",
        ):
            self.assertIn(required, source)

    def test_supervisor_freezes_two_packages_four_variants_and_four_scales(
        self,
    ) -> None:
        source = (
            ROOT / "validation/run_general_sem_rank0_packaged_acceptance.ps1"
        ).read_text(encoding="utf-8")
        for variant in (
            "mediation_point",
            "multiple_mediation_bootstrap",
            "multiple_two_way_moderation_point",
            "multiple_two_way_moderation_bootstrap",
        ):
            self.assertIn(f'"{variant}"', source)
        self.assertIn(
            "$packages = [ordered]@{ installed = $installed; portable = $portable }",
            source,
        )
        self.assertIn("foreach ($scale in @(100, 125, 150, 200))", source)
        self.assertIn("forced_termination = $forced", source)
        self.assertIn("cdp_endpoint_closed = $cdpClosed", source)
        for required in (
            "raw-package-identities.json",
            "windows_pe_package_identity_v1",
            "launched_executable_path",
            "launched_executable_sha256",
            "package_set_fingerprint",
            "$buildFingerprint = $portableHash",
            '"--require-standard-access"',
            "hardware_fingerprint",
            "Win32_Processor",
            "Win32_PhysicalMemory",
            '"--variant-id", $VariantId',
        ):
            self.assertIn(required, source)

    def test_bundled_performance_driver_runs_the_real_package_contract(self) -> None:
        driver = (
            ROOT / "validation/general_sem_rank0_performance_driver.mjs"
        ).read_text(encoding="utf-8")
        orchestrator = (ROOT / "validation/general_sem_rank0_performance.py").read_text(
            encoding="utf-8"
        )
        for required in (
            'new Set(["prepare", "measure", "observe"])',
            'launchPackage(args["quickpls-executable"])',
            "openExactProject(session.page, preparedProject)",
            "await runCalculation(session.page, loaded.variant.bootstrap, true)",
            "await cancelAndVerify(",
            "index < acceptedRuns",
            "processTreeSnapshot(session.child.pid)",
            'window.__TAURI_INTERNALS__.invoke("exit_desktop_application")',
            "QPLS_PERFORMANCE_PROGRESS_PATH",
            "QPLS_PERFORMANCE_RESULT_PATH",
            'workspace.locator(".nd-cbsem-v4-monitor")',
            'workspace.locator(".nd-cbsem-v4-results")',
            'throw new Error("The exact General SEM progress monitor is missing or ambiguous.")',
            'throw new Error("The exact General SEM result surface is missing or ambiguous.")',
            "EXACT_CASE_DIMENSIONS",
            "performance_driver_sha256",
            "qualification_contract_sha256",
            "qualificationContractProjection",
            ".payloads",
        ):
            self.assertIn(required, driver)
        self.assertIn("BUNDLED_DRIVER", orchestrator)
        self.assertIn(
            "executable_sha256 = hashlib.sha256(quickpls_executable.read_bytes()).hexdigest()",
            orchestrator,
        )
        self.assertIn("build_fingerprint != executable_sha256", orchestrator)
        self.assertIn("package_executable_sha256", driver)
        self.assertIn('"prepare",', orchestrator)
        self.assertIn("len(rows) != 30", orchestrator)
        self.assertIn("len(max_and_compound) != 18", orchestrator)
        self.assertIn("if driver_program is not None or driver_args", orchestrator)
        self.assertIn("_validate_result_payloads", orchestrator)

    def test_save_cancel_target_must_be_new_and_inside_explicit_root(self) -> None:
        allowed = (ROOT / "validation/results").resolve()
        target = allowed / "rank0-owned-dialog-never-created.csv"
        self.assertFalse(target.exists())
        args = argparse.Namespace(
            mode="save-cancel",
            target=str(target),
            allowed_root=str(allowed),
            extensions=["csv"],
        )
        observed, root, extensions = validate_target(args)
        self.assertEqual(observed, target)
        self.assertEqual(root, allowed)
        self.assertEqual(extensions, ("csv",))
        args.target = str((ROOT.parent / "outside.csv").resolve())
        with self.assertRaisesRegex(GateFailure, "remain below"):
            validate_target(args)


if __name__ == "__main__":
    unittest.main()
