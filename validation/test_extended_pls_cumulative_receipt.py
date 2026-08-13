from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import patch


VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

import plsc_v2_packaged_acceptance as plsc  # noqa: E402
import wpls_v1_packaged_acceptance as wpls  # noqa: E402


STARTED = datetime(2026, 8, 13, 12, 0, tzinfo=timezone.utc)


class ExtendedPlsCumulativeReceiptTests(unittest.TestCase):
    def exercise(self, module, role: str, result_key: str) -> None:
        results = module.ROOT / "validation/results"
        with tempfile.TemporaryDirectory(dir=results) as directory:
            temp = Path(directory)
            report = temp / "report.json"
            exported = temp / f"{role}.xlsx"
            receipt_path = temp / "receipt.json"
            report.write_text('{"passed":true}\n', encoding="utf-8")
            exported.write_bytes(b"xlsx-evidence")
            relative_report = report.relative_to(module.ROOT).as_posix()
            relative_export = exported.relative_to(module.ROOT).as_posix()
            receipt = {
                "schema_version": 1,
                "kind": "quickpls_v247_cumulative_native_acceptance_receipt",
                "passed": True,
                "supervisor_started_at_utc": "2026-08-13T12:00:00Z",
                "completed_at_utc": "2026-08-13T12:05:00Z",
                "report": relative_report,
                "checks": module.EXPECTED_CUMULATIVE_CHECKS,
                "unique_checks": module.EXPECTED_CUMULATIVE_CHECKS,
                "failures": 0,
                "console_errors": 0,
                "report_sha256": hashlib.sha256(report.read_bytes()).hexdigest(),
                "report_size": report.stat().st_size,
                "final_scope": "regression_bootstrap",
                "graceful_process_cleanup_verified": True,
                "exports": [{"role": role, "path": relative_export, "size": exported.stat().st_size, "sha256": hashlib.sha256(exported.read_bytes()).hexdigest()}],
            }

            def evaluate(document: dict) -> dict:
                receipt_path.write_text(json.dumps(document), encoding="utf-8")
                with patch.object(module, "RAW_REPORT", report), patch.object(module, "CUMULATIVE_RECEIPT", receipt_path):
                    return module.verify_cumulative_receipt(STARTED)

            baseline = evaluate(receipt)
            self.assertTrue(baseline["passed"], baseline)
            self.assertEqual(baseline[result_key]["role"], role)
            for name, mutation, expected in (
                ("cleanup", ("graceful_process_cleanup_verified", False), "graceful_cleanup_verified"),
                ("report_hash", ("report_sha256", "0" * 64), "report_hash_and_size_exact"),
                ("stale", ("supervisor_started_at_utc", "2026-08-13T11:00:00Z"), "fresh_for_invocation"),
            ):
                with self.subTest(role=role, mutation=name):
                    changed = dict(receipt); changed[mutation[0]] = mutation[1]
                    result = evaluate(changed)
                    self.assertFalse(result["passed"])
                    self.assertFalse(result["checks"][expected])

    def test_plsc_receipt_fails_closed(self) -> None:
        self.exercise(plsc, "plsc", "plsc_export")

    def test_wpls_receipt_fails_closed(self) -> None:
        self.exercise(wpls, "wpls", "wpls_export")


if __name__ == "__main__":
    unittest.main()
