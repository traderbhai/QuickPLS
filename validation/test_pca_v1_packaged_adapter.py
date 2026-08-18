import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import pca_v1_packaged_acceptance as packaged


class PcaV1PackagedAdapterTests(unittest.TestCase):
    def test_wrapper_parses_under_windows_powershell_51(self):
        wrapper_path = packaged.ROOT / "validation" / "run_v247_pca_native_acceptance.ps1"
        command = (
            "$tokens=$null; $errors=$null; "
            f"[System.Management.Automation.Language.Parser]::ParseFile('{wrapper_path}', [ref]$tokens, [ref]$errors) | Out-Null; "
            "if ($errors.Count -ne 0) { exit 1 }"
        )
        completed = __import__("subprocess").run(
            ["powershell.exe", "-NoProfile", "-Command", command],
            cwd=packaged.ROOT,
            check=False,
        )
        self.assertEqual(completed.returncode, 0)

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

    def test_wrapper_retries_removal_and_preserves_main_only_when_requested(self):
        wrapper = (packaged.ROOT / "validation" / "run_v247_pca_native_acceptance.ps1").read_text(
            encoding="utf-8"
        )
        self.assertIn("function Remove-FileWithRetry", wrapper)
        self.assertIn("catch [System.IO.IOException]", wrapper)
        self.assertIn("[DateTime]::UtcNow.AddSeconds(5)", wrapper)
        self.assertIn("[switch]$PreserveMainReport", wrapper)
        self.assertIn("$priorReports = @($scopedReportPath)", wrapper)
        self.assertIn("if (-not $PreserveMainReport)", wrapper)
        self.assertIn("$priorReports += $mainReportPath", wrapper)
        self.assertIn("foreach ($priorReport in $priorReports)", wrapper)

    def test_adapter_preserves_the_shared_cumulative_report(self):
        adapter = (packaged.ROOT / "validation" / "pca_v1_packaged_acceptance.py").read_text(
            encoding="utf-8"
        )
        invocation = adapter.rindex('"validation/run_v247_pca_native_acceptance.ps1"')
        self.assertIn('"-PreserveMainReport"', adapter[invocation : invocation + 400])

    def test_factory_snapshot_retries_transient_windows_mapping_and_rehashes(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "fresh.json"
            destination = root / "factory.json"
            source.write_bytes(b'{"passed":true}')
            real_copy = packaged.shutil.copy2
            calls = 0

            def transient_copy(left, right):
                nonlocal calls
                calls += 1
                if calls == 1:
                    raise OSError(1224, "user-mapped section open")
                return real_copy(left, right)

            with (
                patch.object(packaged.shutil, "copy2", side_effect=transient_copy),
                patch.object(packaged.time, "sleep"),
            ):
                packaged.copy2_with_retry(source, destination, attempts=2)
            self.assertEqual(2, calls)
            self.assertEqual(source.read_bytes(), destination.read_bytes())


if __name__ == "__main__":
    unittest.main()
