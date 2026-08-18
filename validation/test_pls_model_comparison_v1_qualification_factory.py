#!/usr/bin/env python3
"""Focused fail-closed tests for the PLS model-comparison work factory."""

from __future__ import annotations

import copy
import sys
import unittest
from pathlib import Path

import jsonschema


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import pls_model_comparison_v1_qualification_factory as factory  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_document,
    validate_spec_path,
)


class PlsModelComparisonV1QualificationFactoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.spec = strict_load_json(factory.SPEC_PATH)
        cls.schema = strict_load_json(factory.FACTORY_SCHEMA_PATH)
        cls.oracle_work = strict_load_json(factory.ORACLE_WORK_PATH)
        cls.audit = strict_load_json(factory.AUDIT_PATH)
        cls.registry = strict_load_json(factory.REGISTRY_PATH)
        cls.manifest = strict_load_json(factory.MANIFEST_PATH)

    def test_spec_is_exact_schema_semantic_registry_valid_but_not_ready(self) -> None:
        self.assertEqual(self.spec, factory.build_spec())
        result = validate_spec_path(
            factory.SPEC_PATH,
            repository_root=ROOT,
            registry_path=factory.REGISTRY_PATH,
            require_registry=True,
        )
        self.assertTrue(result["schema_valid"], result["errors"])
        self.assertTrue(result["semantic_valid"], result["errors"])
        self.assertTrue(result["registry_verified"], result["errors"])
        self.assertTrue(result["passed"], result["errors"])
        self.assertFalse(result["qualification_ready"])
        self.assertEqual(self.spec["migration"]["status"], "compatibility_only")
        self.assertEqual(self.spec["evidence_contract"]["receipts"], [])

    def test_identity_binds_exact_absent_labs_cell_without_mutation(self) -> None:
        identity = self.spec["identity"]
        self.assertEqual(identity["qualification_id"], factory.QUALIFICATION_ID)
        self.assertEqual(identity["method_version"], factory.METHOD_VERSION)
        self.assertEqual(
            identity["capability_cell"],
            {
                "registry_schema_version": 2,
                "capability_id": "smartpls.pls_model_comparison",
                "capability_version": "pls_model_comparison_v1",
                "cell_id": "qpls3.comparison.pls_models",
            },
        )
        capability = next(
            row
            for row in self.registry["capabilities"]
            if row["capability_id"] == factory.CAPABILITY_ID
        )
        cell = next(
            row
            for row in capability["option_cells"]
            if row["cell_id"] == factory.CELL_ID
        )
        self.assertEqual(
            (cell["coverage_state"], cell["evidence_state"], cell["surface"]),
            ("absent", "absent", "labs"),
        )
        self.assertFalse(self.audit["registry_mutated"])
        self.assertFalse(self.audit["manifest_mutated"])

    def test_scenario_axes_and_scale_combinations_are_fully_preregistered(self) -> None:
        scenario = self.spec["scenario_contract"]
        axes = {row["id"] for row in scenario["axes"]}
        self.assertTrue(
            {
                "model_topology",
                "measurement_model",
                "construct_type",
                "data_distribution",
                "missingness",
                "input_type",
                "sample_size",
                "variable_count",
                "groups",
                "estimator_options",
                "effect_strength",
                "seed",
                "fold_design",
                "candidate_complexity",
                "workload",
                "workers",
            }
            <= axes
        )
        pairwise = [
            row
            for row in scenario["mandatory_combinations"]
            if row["coverage"] == "pairwise"
        ]
        self.assertTrue(pairwise)
        self.assertTrue(all(set(row["selections"]) == axes for row in pairwise))
        maximum = {
            tuple(row["stressed_dimensions"])
            for row in scenario["mandatory_combinations"]
            if row["profile_id"] == "maximum_axis"
        }
        self.assertEqual(
            maximum,
            {("rows",), ("indicators",), ("constructs",), ("resamples",)},
        )
        profiles = {row["id"]: row for row in scenario["complexity_profiles"]}
        self.assertTrue(
            all(row["workload"]["candidate_models"] == 2 for row in profiles.values())
        )
        self.assertTrue(all(row["workload"]["groups"] == 1 for row in profiles.values()))
        self.assertTrue(scenario["monte_carlo_policy"]["failed_fits_in_denominator"])
        self.assertEqual(scenario["monte_carlo_policy"]["maximum_half_width"], 0.01)

    def test_oracle_work_is_current_hash_bound_partial_and_nonpromotional(self) -> None:
        sources = factory.source_descriptors()
        source_hash = canonical_sha256(sources)
        scenario_hash = canonical_sha256(self.spec["scenario_contract"])
        self.assertEqual(
            self.oracle_work,
            factory.build_oracle_work_report(sources, source_hash, scenario_hash),
        )
        self.assertEqual(self.oracle_work["source_artifacts"], sources)
        self.assertEqual(self.oracle_work["source_set_sha256"], source_hash)
        self.assertEqual(self.oracle_work["scenario_set_sha256"], scenario_hash)
        self.assertTrue(self.oracle_work["passed_work_checks"])
        self.assertTrue(self.oracle_work["work_evidence_only"])
        self.assertFalse(self.oracle_work["qualification_role_satisfied"])
        self.assertFalse(self.oracle_work["receipt_eligible"])
        self.assertFalse(self.oracle_work["qualification_ready"])
        self.assertFalse(self.oracle_work["promotion_allowed"])
        self.assertNotIn("qpls_estimation", self.oracle_work["imports"])
        self.assertNotIn("quickpls", self.oracle_work["imports"])
        self.assertIn("does not independently fit", " ".join(self.oracle_work["blockers"]))

    def test_checked_in_work_artifacts_validate_against_fail_closed_schema(self) -> None:
        validator = jsonschema.Draft202012Validator(
            self.schema,
            format_checker=jsonschema.FormatChecker(),
        )
        validator.validate(self.oracle_work)
        validator.validate(self.audit)

    def test_every_work_descriptor_is_source_scenario_bound_and_never_a_receipt(self) -> None:
        descriptors = self.audit["work_descriptors"]
        self.assertEqual(
            tuple(row["role"] for row in descriptors),
            factory.EXPECTED_REQUIRED_ROLES,
        )
        self.assertEqual(len({row["descriptor_id"] for row in descriptors}), len(descriptors))
        for descriptor in descriptors:
            self.assertEqual(
                descriptor["source_set_sha256"], self.audit["source_set_sha256"]
            )
            self.assertEqual(
                descriptor["scenario_set_sha256"], self.audit["scenario_set_sha256"]
            )
            self.assertFalse(descriptor["candidate_receipt_emitted"])
            self.assertFalse(descriptor["qualification_ready"])
            self.assertFalse(descriptor["promotion_allowed"])
            self.assertTrue(descriptor["required_check_ids"])
            self.assertTrue(descriptor["required_artifact_classes"])
            self.assertTrue(descriptor["blockers"])
        self.assertEqual(self.audit["candidate_receipt_descriptors"], [])
        self.assertEqual(self.audit["attached_receipt_count"], 0)

    def test_runner_canonical_and_schema6_work_is_bound_but_not_promoted(self) -> None:
        source_paths = {row["path"] for row in self.audit["source_artifacts"]}
        self.assertTrue(
            {
                "crates/qpls-runner/src/pls_model_comparison_execution.rs",
                "src-tauri/src/pls_model_comparison_jobs.rs",
                "src-tauri/src/recipe_v4_jobs.rs",
                "crates/qpls-project/tests/pls_model_comparison_schema6.rs",
            }
            <= source_paths
        )
        descriptors = {row["role"]: row for row in self.audit["work_descriptors"]}
        for role in (
            "runner_integration",
            "archive_persistence",
            "canonical_result_projection",
        ):
            self.assertEqual(descriptors[role]["status"], "work_evidence_only")
            self.assertFalse(descriptors[role]["candidate_receipt_emitted"])
            self.assertFalse(descriptors[role]["qualification_ready"])
            self.assertFalse(descriptors[role]["promotion_allowed"])

    def test_full_product_evidence_blockers_remain_explicit(self) -> None:
        blockers = set(self.audit["remaining_blockers"])
        self.assertEqual(blockers, set(factory.FULL_BLOCKERS))
        required_fragments = (
            "runner",
            "archive",
            "canonical_result_document_v2",
            "frontend",
            "cli",
            "export",
            "second_independently_maintained",
            "generative",
            "simulation",
            "packaged_windows",
            "accessibility",
            "performance",
            "review",
        )
        joined = " ".join(blockers)
        for fragment in required_fragments:
            self.assertIn(fragment, joined)

    def test_source_hash_mutation_invalidates_work_binding(self) -> None:
        descriptors = factory.source_descriptors()
        self.assertEqual(canonical_sha256(descriptors), self.audit["source_set_sha256"])
        changed = copy.deepcopy(descriptors)
        changed[0]["sha256"] = "0" * 64
        self.assertNotEqual(canonical_sha256(changed), self.audit["source_set_sha256"])

    def test_compatibility_manifest_cannot_satisfy_v2_or_promote(self) -> None:
        self.assertEqual(self.manifest["qualification"]["declared_state"], "absent")
        self.assertTrue(
            all(
                not rows
                for rows in self.manifest["qualification"]["evidence"].values()
            )
        )
        candidate = copy.deepcopy(self.spec)
        result = validate_spec_document(
            candidate,
            repository_root=ROOT,
            registry_document=self.registry,
            require_registry=True,
        )
        self.assertTrue(result["passed"], result["errors"])
        self.assertFalse(result["qualification_ready"])
        self.assertFalse(self.audit["scientific_review_satisfied"])
        self.assertFalse(self.audit["qualification_ready"])
        self.assertFalse(self.audit["promotion_allowed"])

    def test_checked_in_factory_verifies_fail_closed(self) -> None:
        result = factory.verify_checked_in_factory()
        self.assertTrue(result["passed"], result["errors"])
        self.assertFalse(result["qualification_ready"])
        self.assertFalse(result["promotion_allowed"])
        self.assertEqual(result["candidate_receipt_count"], 0)


if __name__ == "__main__":
    unittest.main()
