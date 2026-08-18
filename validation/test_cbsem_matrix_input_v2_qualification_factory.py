#!/usr/bin/env python3
"""Fail-closed tests for the CB-SEM matrix-input V2 work factory."""

from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import cbsem_matrix_input_v2_oracle as oracle  # noqa: E402
import cbsem_matrix_input_v2_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import strict_load_json, validate_spec_path  # noqa: E402


class CbsemMatrixInputV2QualificationFactoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.spec = strict_load_json(factory.SPEC_PATH)
        cls.audit = strict_load_json(factory.AUDIT_PATH)
        cls.oracle_report = strict_load_json(factory.ORACLE_REPORT_PATH)

    def test_spec_is_current_and_semantically_valid_but_not_ready(self) -> None:
        self.assertEqual(self.spec, factory.build_spec())
        result = validate_spec_path(
            factory.SPEC_PATH,
            repository_root=ROOT,
            registry_path=factory.REGISTRY_PATH,
            require_registry=True,
        )
        self.assertTrue(result["passed"], result["errors"])
        self.assertTrue(result["registry_verified"])
        self.assertFalse(result["qualification_ready"])
        self.assertEqual(self.spec["migration"]["status"], "compatibility_only")
        self.assertTrue(self.spec["migration"]["unresolved_items"])
        self.assertEqual(self.spec["evidence_contract"]["receipts"], [])

    def test_matrix_input_cell_is_registered_without_promoting_evidence(self) -> None:
        registry = strict_load_json(
            VALIDATION / "capabilities" / "capability_registry_v2.json"
        )
        capability = next(
            row
            for row in registry["capabilities"]
            if row["capability_id"] == factory.CAPABILITY_ID
        )
        cell = next(
            row for row in capability["option_cells"] if row["cell_id"] == factory.CELL_ID
        )
        self.assertEqual(cell["capability_version"], factory.METHOD_VERSION)
        self.assertEqual(cell["coverage_state"], "partial")
        self.assertEqual(cell["evidence_state"], "absent")
        self.assertEqual(cell["surface"], "labs")
        self.assertEqual(
            cell["qualification_spec"]["links"],
            [
                {
                    "registry_schema_version": 2,
                    "capability_id": factory.CAPABILITY_ID,
                    "cell_id": factory.CELL_ID,
                    "capability_version": factory.METHOD_VERSION,
                }
            ],
        )
        self.assertIn(
            factory.CELL_ID,
            {row["cell_id"] for row in capability["qualification_links"]},
        )
        self.assertFalse(self.audit["registry_mutated"])

    def test_oracle_work_is_current_but_never_admitted_as_receipt(self) -> None:
        self.assertEqual(self.oracle_report, oracle.build_report())
        self.assertTrue(self.oracle_report["passed_work_checks"])
        self.assertFalse(self.oracle_report["qualification_role_satisfied"])
        self.assertFalse(self.oracle_report["receipt_eligible"])
        self.assertEqual(self.audit["candidate_receipt_descriptors"], [])
        self.assertFalse(self.audit["qualification_ready"])
        self.assertFalse(self.audit["promotion_allowed"])

    def test_all_roles_are_explicitly_work_only_or_blocked(self) -> None:
        rows = {row["role"]: row for row in self.audit["role_matrix"]}
        self.assertEqual(tuple(rows), factory.EXPECTED_REQUIRED_ROLES)
        for role in (
            "method_contract",
            "oracle_independence",
            "generative_recovery",
            "adversarial_boundaries",
        ):
            self.assertEqual(rows[role]["status"], "work_evidence_only")
            self.assertFalse(rows[role]["candidate_receipt_emitted"])
        for role in (
            "kernel_execution",
            "archive_persistence",
            "cross_format_export",
            "frontend_contract",
            "packaged_windows_e2e",
            "performance_scale",
        ):
            self.assertEqual(rows[role]["status"], "blocked")
            self.assertFalse(rows[role]["candidate_receipt_emitted"])

    def test_scenario_contract_is_preregistered_and_covers_required_shapes(self) -> None:
        scenario = self.spec["scenario_contract"]
        self.assertEqual(
            self.audit["scenario_set_sha256"],
            factory.canonical_sha256(scenario),
        )
        combinations = scenario["mandatory_combinations"]
        self.assertTrue(any(row["coverage"] == "pairwise" for row in combinations))
        maximum = {
            tuple(row["stressed_dimensions"])
            for row in combinations
            if row["profile_id"] == "maximum_axis"
        }
        self.assertEqual(maximum, {("rows",), ("indicators",), ("constructs",)})
        compound = next(
            row for row in combinations if row["profile_id"] == "compound_stress"
        )
        self.assertEqual(
            set(compound["stressed_dimensions"]),
            {"rows", "indicators", "constructs"},
        )

    def test_mutated_oracle_report_fails_current_work_check(self) -> None:
        changed = copy.deepcopy(self.oracle_report)
        changed["passed_work_checks"] = False
        with tempfile.TemporaryDirectory(dir=VALIDATION) as directory:
            path = Path(directory) / "oracle.json"
            path.write_text(json.dumps(changed), encoding="utf-8")
            with patch.object(factory, "ORACLE_REPORT_PATH", path):
                result = factory._oracle_work()  # noqa: SLF001
        self.assertTrue(result["exists"])
        self.assertFalse(result["current"])
        self.assertFalse(result["passed_work_checks"])

    def test_checked_in_factory_is_exactly_source_bound(self) -> None:
        result = factory.verify_checked_in_factory()
        self.assertTrue(result["passed"], result["errors"])
        self.assertEqual(
            self.audit["source_artifacts"], factory.source_descriptors()
        )
        self.assertEqual(
            self.audit["source_set_sha256"],
            factory.canonical_sha256(factory.source_descriptors()),
        )
        self.assertFalse(self.audit["manifest_mutated"])
        self.assertFalse(self.audit["qualification_spec_receipts_mutated"])


if __name__ == "__main__":
    unittest.main()
