import hashlib
import json
import os
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

import pls_algorithm_v1_factory_audit as audit
import pls_algorithm_v1_packaged_acceptance as packaged
from packaged_windows_acceptance_v2 import packaged_acceptance_contract_descriptor


class PlsAlgorithmV1PackagedAdapterTests(unittest.TestCase):
    def test_current_cumulative_contract_count_is_manifest_derived(self):
        expected = sum(
            len(check_set["required_check_ids"])
            for check_set in packaged.PACKAGED_ACCEPTANCE_CONTRACT["ordered_check_sets"]
        )
        self.assertEqual(packaged.EXPECTED_CUMULATIVE_CHECKS, expected)

    def _write(self, root: Path, relative: str, content: bytes, mtime_ns: int) -> Path:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        os.utime(path, ns=(mtime_ns, mtime_ns))
        return path

    def _freshness_fixture(self, source_mtime: int, gate_mtime: int = 400):
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        source = self._write(root, "crates/qpls-cli/src/main.rs", b"source", source_mtime)
        gate = self._write(root, "validation/gate.py", b"gate", gate_mtime)
        desktop = self._write(
            root, "target/release/quickpls-desktop.exe", b"desktop", 200
        )
        release_cli = self._write(root, "target/release/qpls.exe", b"cli", 200)
        receipt = self._write(
            root, "validation/results/diagnostic_bundle_build_receipt.json", b"{}", 150
        )
        return temporary, root, source, gate, desktop, release_cli, receipt

    def _cumulative_receipt_fixture(self):
        temporary = tempfile.TemporaryDirectory()
        root = Path(temporary.name)
        not_before = datetime(2026, 8, 13, 15, 0, tzinfo=timezone.utc)
        workbook = self._write(
            root,
            "validation/results/v247-native-full-reuse.xlsx",
            b"exact PLS export bytes",
            200,
        )
        workbook_sha = hashlib.sha256(workbook.read_bytes()).hexdigest()
        checks = {
            check_id: {}
            for check_set in packaged.PACKAGED_ACCEPTANCE_CONTRACT["ordered_check_sets"]
            for check_id in check_set["required_check_ids"]
        }
        checks["mediationExport"] = {
            "nativeXlsx": {
                "helper": {
                    "completion": {
                        "workbook": {
                            "path": str(workbook),
                            "size": workbook.stat().st_size,
                            "sha256": workbook_sha,
                        }
                    }
                }
            }
        }
        report_document = {
            "passed": True,
            "focusedRun": {
                "scope": "regression_bootstrap",
                "completedAt": (not_before + timedelta(seconds=1))
                .isoformat()
                .replace("+00:00", "Z"),
            },
            "checks": checks,
            "failures": [],
            "consoleErrors": [],
        }
        report = root / "validation/results/v247_tauri_native_acceptance.json"
        report.parent.mkdir(parents=True, exist_ok=True)
        report.write_text(
            json.dumps(report_document, sort_keys=True), encoding="utf-8"
        )
        receipt_document = {
            "schema_version": 2,
            "kind": "quickpls_v247_cumulative_native_acceptance_receipt",
            "passed": True,
            "supervisor_started_at_utc": (not_before - timedelta(seconds=1))
            .isoformat()
            .replace("+00:00", "Z"),
            "completed_at_utc": (not_before + timedelta(seconds=2))
            .isoformat()
            .replace("+00:00", "Z"),
            "report": "validation/results/v247_tauri_native_acceptance.json",
            "checks": packaged.EXPECTED_CUMULATIVE_CHECKS,
            "unique_checks": packaged.EXPECTED_CUMULATIVE_CHECKS,
            "failures": 0,
            "console_errors": 0,
            "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
            "report_size": report.stat().st_size,
            "final_scope": "regression_bootstrap",
            "graceful_process_cleanup_verified": True,
            "acceptance_contract": {
                **packaged_acceptance_contract_descriptor(),
                "contract_id": packaged.PACKAGED_ACCEPTANCE_CONTRACT["contract_id"],
                "contract_version": packaged.PACKAGED_ACCEPTANCE_CONTRACT["contract_version"],
                "required_check_count": packaged.EXPECTED_CUMULATIVE_CHECKS,
            },
            "exports": [
                {
                    "role": "generic",
                    "path": workbook.relative_to(root).as_posix(),
                    "size": workbook.stat().st_size,
                    "sha256": workbook_sha,
                }
            ],
        }
        receipt = root / "validation/results/v247_cumulative_native_acceptance_receipt.json"
        receipt.write_text(json.dumps(receipt_document), encoding="utf-8")
        return (
            temporary,
            root,
            not_before,
            report,
            report_document,
            receipt,
            receipt_document,
            workbook,
        )

    def _verify_cumulative_fixture(
        self,
        *,
        receipt_mutation=None,
        report_mutation=None,
        refresh_receipt_report_bytes=False,
    ):
        (
            temporary,
            root,
            not_before,
            report,
            report_document,
            receipt,
            receipt_document,
            _workbook,
        ) = self._cumulative_receipt_fixture()
        with temporary:
            if report_mutation is not None:
                report_mutation(report_document)
                report.write_text(
                    json.dumps(report_document, sort_keys=True), encoding="utf-8"
                )
            if refresh_receipt_report_bytes:
                receipt_document["report_sha256"] = hashlib.sha256(
                    report.read_bytes()
                ).hexdigest()
                receipt_document["report_size"] = report.stat().st_size
            if receipt_mutation is not None:
                receipt_mutation(receipt_document)
            receipt.write_text(json.dumps(receipt_document), encoding="utf-8")
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "RAW_REPORT", report),
                patch.object(packaged, "CUMULATIVE_RECEIPT", receipt),
            ):
                return packaged.verify_reusable_cumulative_receipt(not_before)

    def test_gate_only_change_does_not_stale_frozen_binaries(self):
        temporary, root, source, gate, desktop, release_cli, receipt = (
            self._freshness_fixture(100)
        )
        with temporary:
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", release_cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "GATE_SOURCES", {gate.relative_to(root).as_posix()}),
                patch.object(
                    packaged,
                    "cli_source_paths",
                    return_value=[source.relative_to(root).as_posix()],
                ),
                patch.object(
                    packaged,
                    "strict_load_json",
                    return_value={"schema_version": "receipt-v1"},
                ),
                patch.object(packaged, "validate_build_receipt"),
            ):
                result = packaged.source_freshness()
        self.assertTrue(result["passed"], result)
        self.assertTrue(result["desktop_receipt_exact"])
        self.assertEqual([], result["release_cli_newer_build_sources"])
        self.assertEqual(
            gate.relative_to(root).as_posix(),
            result["gate_sources_excluded_from_binary_freshness"][0]["path"],
        )

    def test_newer_cli_build_source_fails_closed(self):
        temporary, root, source, gate, desktop, release_cli, receipt = (
            self._freshness_fixture(300, gate_mtime=100)
        )
        with temporary:
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", release_cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "GATE_SOURCES", {gate.relative_to(root).as_posix()}),
                patch.object(
                    packaged,
                    "cli_source_paths",
                    return_value=[source.relative_to(root).as_posix()],
                ),
                patch.object(packaged, "strict_load_json", return_value={}),
                patch.object(packaged, "validate_build_receipt"),
            ):
                result = packaged.source_freshness()
        self.assertFalse(result["passed"])
        self.assertEqual(
            [source.relative_to(root).as_posix()],
            result["release_cli_newer_build_sources"],
        )

    def test_receipt_validation_failure_fails_closed(self):
        temporary, root, source, gate, desktop, release_cli, receipt = (
            self._freshness_fixture(100)
        )
        with temporary:
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", release_cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "strict_load_json", return_value={}),
                patch.object(
                    packaged,
                    "validate_build_receipt",
                    side_effect=packaged.SourceManifestFailure("receipt drift"),
                ),
            ):
                result = packaged.source_freshness()
        self.assertFalse(result["passed"])
        self.assertFalse(result["desktop_receipt_exact"])
        self.assertIn("receipt drift", result["error"])

    def test_release_cli_source_closure_includes_compile_time_catalogue(self):
        self.assertIn("validation/development_slices.json", packaged.cli_source_paths())

    def test_exact_cumulative_receipt_is_reusable(self):
        result = self._verify_cumulative_fixture()
        self.assertTrue(result["passed"], result)
        self.assertTrue(all(result["checks"].values()), result)

    def test_cumulative_receipt_mutations_fail_closed(self):
        old_start = "2026-08-13T14:59:57Z"
        before_completion = "2026-08-13T14:59:59Z"
        mutations = {
            "start_outside_two_second_tolerance": (
                lambda receipt: receipt.__setitem__(
                    "supervisor_started_at_utc", old_start
                ),
                None,
                False,
                "start_within_two_second_reuse_tolerance",
            ),
            "completion_before_not_before": (
                lambda receipt: receipt.__setitem__(
                    "completed_at_utc", before_completion
                ),
                None,
                False,
                "completed_after_reuse_boundary",
            ),
            "report_sha_drift": (
                lambda receipt: receipt.__setitem__("report_sha256", "0" * 64),
                None,
                False,
                "exact_report_bytes",
            ),
            "report_size_drift": (
                lambda receipt: receipt.__setitem__("report_size", 1),
                None,
                False,
                "exact_report_bytes",
            ),
            "receipt_check_count_drift": (
                lambda receipt: receipt.__setitem__(
                    "checks", packaged.EXPECTED_CUMULATIVE_CHECKS - 1
                ),
                None,
                False,
                "exact_required_checks",
            ),
            "actual_report_check_count_drift": (
                None,
                lambda report: report["checks"].pop("ncaReferenceFixture"),
                True,
                "exact_required_checks",
            ),
            "final_scope_drift": (
                lambda receipt: receipt.__setitem__("final_scope", "pca"),
                None,
                False,
                "final_scope_regression_bootstrap",
            ),
            "cleanup_not_verified": (
                lambda receipt: receipt.__setitem__(
                    "graceful_process_cleanup_verified", False
                ),
                None,
                False,
                "graceful_cleanup_verified",
            ),
            "generic_export_hash_drift": (
                lambda receipt: receipt["exports"][0].__setitem__(
                    "sha256", "0" * 64
                ),
                None,
                False,
                "generic_pls_export_exact_bytes",
            ),
            "report_workbook_hash_drift": (
                None,
                lambda report: report["checks"]["mediationExport"]["nativeXlsx"]
                ["helper"]["completion"]["workbook"].__setitem__(
                    "sha256", "f" * 64
                ),
                True,
                "generic_pls_export_bound_to_report",
            ),
        }
        for name, (
            receipt_mutation,
            report_mutation,
            refresh_report_bytes,
            expected_check,
        ) in mutations.items():
            with self.subTest(name=name):
                result = self._verify_cumulative_fixture(
                    receipt_mutation=receipt_mutation,
                    report_mutation=report_mutation,
                    refresh_receipt_report_bytes=refresh_report_bytes,
                )
                self.assertFalse(result["passed"], result)
                self.assertFalse(result["checks"][expected_check], result)

    def test_audit_does_not_consume_its_own_output(self):
        document = packaged.manifest()
        packaged_report = {
            "passed": True,
            "checks": {
                "native": {"passed": True},
                "responsive_viewports": {"passed": True},
                "runner_cleanup_verified": True,
                "source_freshness": {
                    "passed": True,
                    "source_stable_during_gate": True,
                    "before": {
                        "desktop_receipt_exact": True,
                        "release_cli_newer_build_sources": [],
                    },
                    "after": {
                        "desktop_receipt_exact": True,
                        "release_cli_newer_build_sources": [],
                    },
                },
            },
        }
        observed_roles = []

        def verify(artifact, *_args):
            observed_roles.extend(artifact["roles"])
            return True, []

        with (
            patch.object(audit, "_verify_artifact", side_effect=verify),
            patch.object(audit, "strict_load_json", return_value=packaged_report),
        ):
            result = audit.evaluate_audit_inputs(document)
        self.assertTrue(result["passed"], result)
        self.assertIn("packaged_acceptance", observed_roles)
        self.assertNotIn("method_audit", observed_roles)

    def test_failed_audit_stops_before_manifest_derivation(self):
        with (
            patch.object(
                packaged,
                "run_command",
                return_value=(SimpleNamespace(returncode=1), {"returncode": 1}),
            ),
            patch.object(packaged, "validate_manifest") as validate,
        ):
            result = packaged.finalize_release_promotion()
        self.assertFalse(result["passed"])
        self.assertEqual("method_audit", result["phase"])
        validate.assert_not_called()

    def test_final_manifest_must_derive_release_qualified(self):
        with (
            patch.object(
                packaged,
                "run_command",
                return_value=(SimpleNamespace(returncode=0), {"returncode": 0}),
            ),
            patch.object(
                packaged,
                "validate_manifest",
                return_value={"passed": True, "derived_state": "native_qualified"},
            ),
        ):
            result = packaged.finalize_release_promotion()
        self.assertFalse(result["passed"])
        self.assertEqual("final_manifest", result["phase"])

    def test_final_manifest_release_qualification_passes(self):
        with (
            patch.object(
                packaged,
                "run_command",
                return_value=(SimpleNamespace(returncode=0), {"returncode": 0}),
            ),
            patch.object(
                packaged,
                "validate_manifest",
                return_value={"passed": True, "derived_state": "release_qualified"},
            ),
        ):
            result = packaged.finalize_release_promotion()
        self.assertTrue(result["passed"], result)


if __name__ == "__main__":
    unittest.main()
