#!/usr/bin/env python3
from __future__ import annotations

import json
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
sys.path.insert(0, str(VALIDATION))

from phase3_workflow_factory import (  # noqa: E402
    DuplicateKeyError,
    _scientific_checks,
    report_root,
    sha256_bytes,
    strict_load,
    upgrade_recipe_v3,
)


class Phase3WorkflowFactoryTests(unittest.TestCase):
    def test_requalified_archive_methods_use_new_append_only_generation(self) -> None:
        self.assertEqual(
            report_root("mediation_v1").name,
            "evidence_truth_reconciliation_v2",
        )
        self.assertEqual(
            report_root("moderation_v1").name,
            "evidence_truth_reconciliation_v2",
        )
        self.assertNotEqual(
            report_root("higher_order_v1").name,
            "evidence_truth_reconciliation_v2",
        )

    def test_strict_json_rejects_duplicate_keys(self) -> None:
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "duplicate.json"
            path.write_text('{"passed":true,"passed":false}\n', encoding="utf-8")
            with self.assertRaises(DuplicateKeyError):
                strict_load(path)

    def test_schema_v3_upgrade_is_explicit_and_non_mutating(self) -> None:
        source = {
            "schema_version": 2,
            "model": {"constructs": [], "paths": []},
            "settings": {"method": "pls_pm"},
            "metadata": {},
        }
        upgraded = upgrade_recipe_v3(source, "pls_algorithm")
        self.assertEqual(source["schema_version"], 2)
        self.assertEqual(upgraded["schema_version"], 3)
        self.assertEqual(upgraded["method_config"], {"kind": "pls_algorithm"})
        self.assertEqual(upgraded["model"]["controls"], [])
        self.assertEqual(upgraded["settings"]["case_weight_column"], None)

    def test_checksum_mutation_is_detectable(self) -> None:
        original = b'{"results":[]}\n'
        tampered = b'{"results":[1]}\n'
        self.assertNotEqual(sha256_bytes(original), sha256_bytes(tampered))
        with tempfile.TemporaryDirectory() as folder:
            archive_path = Path(folder) / "fixture.qpls"
            with zipfile.ZipFile(archive_path, "w") as archive:
                archive.writestr("project.json", tampered)
                archive.writestr(
                    "manifest.json",
                    json.dumps({"checksums": {"project.json": sha256_bytes(original)}}),
                )
            with zipfile.ZipFile(archive_path) as archive:
                manifest = json.loads(archive.read("manifest.json"))
                self.assertNotEqual(
                    sha256_bytes(archive.read("project.json")),
                    manifest["checksums"]["project.json"],
                )

    def test_scientific_gate_can_require_the_raw_report_to_pass(self) -> None:
        passed, failures = _scientific_checks(
            {"passed": False, "checks": {"point_only_scope_warning_present": True}},
            set(),
            require_report_passed=True,
        )
        self.assertFalse(passed)
        self.assertEqual(failures, {"report_passed": False})


if __name__ == "__main__":
    unittest.main()
