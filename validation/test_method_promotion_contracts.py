#!/usr/bin/env python3
"""Tests for portable, explicitly non-claiming contract validation."""

from __future__ import annotations

import copy
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from validation import method_promotion_contracts as contracts
from validation import method_promotion_manifest as factory


class MethodPromotionContractTests(unittest.TestCase):
    def test_current_contract_set_passes_without_reading_runtime_evidence(self) -> None:
        with patch.object(
            factory,
            "_verify_artifact",
            side_effect=AssertionError("portable validation read runtime evidence"),
        ):
            report = contracts.validate_contracts()

        self.assertTrue(report["passed"], report)
        self.assertFalse(report["claim_authorized"])
        self.assertFalse(report["evidence_verified"])
        self.assertEqual(report["manifest_count"], 40)

    def test_contract_errors_still_fail_closed(self) -> None:
        source = factory.strict_load_json(
            factory.MANIFEST_DIR / "history" / "pls_sample_size_power_v1.manifest.json"
        )
        source = copy.deepcopy(source)
        source["qualification"]["target_state"] = "invented_state"
        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            manifest = root / "invalid.manifest.json"
            manifest.write_text(
                __import__("json").dumps(source),
                encoding="utf-8",
            )
            report = contracts.validate_contracts([manifest], factory.REPOSITORY_ROOT)

        self.assertFalse(report["passed"])
        self.assertFalse(report["claim_authorized"])
        self.assertFalse(report["evidence_verified"])


if __name__ == "__main__":
    unittest.main()
