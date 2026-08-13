import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import pca_v1_packaged_acceptance as packaged


class PcaV1PackagedAdapterTests(unittest.TestCase):
    def _write(self, root: Path, relative: str, content: bytes, mtime_ns: int) -> Path:
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        os.utime(path, ns=(mtime_ns, mtime_ns))
        return path

    def test_gate_only_change_does_not_stale_frozen_binaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = self._write(root, "crates/qpls-cli/src/main.rs", b"source", 100)
            gate = self._write(root, "validation/v247_tauri_native_acceptance.mjs", b"gate", 400)
            desktop = self._write(root, "target/release/quickpls-desktop.exe", b"desktop", 200)
            cli = self._write(root, "target/release/qpls.exe", b"cli", 200)
            receipt = self._write(root, "validation/results/receipt.json", b"{}", 150)
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "GATE_SOURCES", {gate.relative_to(root).as_posix()}),
                patch.object(packaged, "cli_source_paths", return_value=[source.relative_to(root).as_posix()]),
                patch.object(packaged, "strict_load_json", return_value={"schema_version": "receipt-v1"}),
                patch.object(packaged, "validate_build_receipt"),
            ):
                result = packaged.source_freshness()
            self.assertTrue(result["passed"], result)
            self.assertEqual([], result["release_cli_newer_build_sources"])
            self.assertEqual(gate.relative_to(root).as_posix(), result["gate_sources_excluded_from_binary_freshness"][0]["path"])

    def test_cli_build_source_change_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = self._write(root, "crates/qpls-cli/src/main.rs", b"new source", 300)
            gate = self._write(root, "validation/v247_tauri_native_acceptance.mjs", b"gate", 100)
            desktop = self._write(root, "target/release/quickpls-desktop.exe", b"desktop", 200)
            cli = self._write(root, "target/release/qpls.exe", b"cli", 200)
            receipt = self._write(root, "validation/results/receipt.json", b"{}", 150)
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "GATE_SOURCES", {gate.relative_to(root).as_posix()}),
                patch.object(packaged, "cli_source_paths", return_value=[source.relative_to(root).as_posix()]),
                patch.object(packaged, "strict_load_json", return_value={"schema_version": "receipt-v1"}),
                patch.object(packaged, "validate_build_receipt"),
            ):
                result = packaged.source_freshness()
            self.assertFalse(result["passed"])
            self.assertEqual([source.relative_to(root).as_posix()], result["release_cli_newer_build_sources"])

    def test_receipt_validation_failure_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            desktop = self._write(root, "target/release/quickpls-desktop.exe", b"desktop", 200)
            cli = self._write(root, "target/release/qpls.exe", b"cli", 200)
            receipt = self._write(root, "validation/results/receipt.json", b"{}", 150)
            with (
                patch.object(packaged, "ROOT", root),
                patch.object(packaged, "DESKTOP", desktop),
                patch.object(packaged, "RELEASE_CLI", cli),
                patch.object(packaged, "BUILD_RECEIPT", receipt),
                patch.object(packaged, "strict_load_json", return_value={}),
                patch.object(packaged, "validate_build_receipt", side_effect=packaged.SourceManifestFailure("receipt drift")),
            ):
                result = packaged.source_freshness()
            self.assertFalse(result["passed"])
            self.assertIn("receipt drift", result["error"])

    def test_wrapper_retries_removal_of_prior_shared_reports(self):
        wrapper = (packaged.ROOT / "validation" / "run_v247_pca_native_acceptance.ps1").read_text(
            encoding="utf-8"
        )
        self.assertIn("function Remove-FileWithRetry", wrapper)
        self.assertIn("catch [System.IO.IOException]", wrapper)
        self.assertIn("[DateTime]::UtcNow.AddSeconds(5)", wrapper)
        self.assertIn("foreach ($priorReport in @($mainReportPath, $scopedReportPath))", wrapper)


if __name__ == "__main__":
    unittest.main()
