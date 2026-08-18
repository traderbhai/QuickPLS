#!/usr/bin/env python3
"""Fail-closed contract tests for the isolated QualificationSpec V2 lane."""

from __future__ import annotations

import json
import hashlib
import sys
import tempfile
import unittest
from copy import deepcopy
from pathlib import Path
from typing import Any

from jsonschema import Draft202012Validator


VALIDATION_DIR = Path(__file__).resolve().parent
REPOSITORY_ROOT = VALIDATION_DIR.parent
FIXTURE_DIR = VALIDATION_DIR / "qualification_v2" / "fixtures"
MIGRATED_FIXTURE = FIXTURE_DIR / "pls_algorithm_v1.migrated.json"
REGISTRY_FIXTURE = FIXTURE_DIR / "capability_registry_v2.fixture.json"
AUTHORITATIVE_REGISTRY = VALIDATION_DIR / "capabilities" / "capability_registry_v2.json"
LEGACY_MANIFEST = VALIDATION_DIR / "methods" / "pls_algorithm_v1.manifest.json"
SCHEMA_PATH = VALIDATION_DIR / "qualification_v2" / "qualification_spec_v2.schema.json"
sys.path.insert(0, str(VALIDATION_DIR))

from qualification_spec_v2 import (  # noqa: E402
    DuplicateKeyError,
    adapt_v1_manifest_report,
    canonical_sha256,
    strict_load_json,
    validate_spec_document,
    validate_spec_path,
)


COMPARISON_RULES: dict[str, dict[str, Any]] = {
    "exact": {},
    "abs_relative": {
        "absolute_tolerance": 1e-8,
        "relative_tolerance": 1e-6,
    },
    "matrix_norm": {
        "absolute_tolerance": 1e-8,
        "relative_tolerance": 1e-6,
        "norm": "frobenius",
        "elementwise_tolerance": 1e-7,
    },
    "sign_orientation": {
        "absolute_tolerance": 1e-8,
        "relative_tolerance": 1e-6,
        "orientation_keys": ["construct_id", "anchor_indicator_id"],
    },
    "subspace": {
        "maximum_principal_angle_degrees": 0.1,
        "projector_tolerance": 1e-8,
    },
    "label_permutation": {
        "assignment_metric": "hungarian_l2",
        "absolute_tolerance": 1e-8,
        "relative_tolerance": 1e-6,
    },
    "monte_carlo_interval": {
        "confidence_level": 0.95,
        "maximum_half_width": 0.02,
        "acceptance_interval": [0.9, 1.0],
    },
}


class QualificationSpecV2Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.document = strict_load_json(MIGRATED_FIXTURE)
        cls.registry = strict_load_json(REGISTRY_FIXTURE)
        cls.legacy = strict_load_json(LEGACY_MANIFEST)

    def _result(self, document: dict[str, Any]) -> dict[str, Any]:
        return validate_spec_document(document)

    def _strict_result(self, document: dict[str, Any]) -> dict[str, Any]:
        return validate_spec_document(
            document,
            repository_root=REPOSITORY_ROOT,
            verify_receipts=True,
            registry_document=self.registry,
            require_registry=True,
        )

    def _rebind_scenario_receipts(self, document: dict[str, Any]) -> None:
        digest = canonical_sha256(document["scenario_contract"])
        for receipt in document["evidence_contract"]["receipts"]:
            receipt["scenario_set_sha256"] = digest

    def _completed(self) -> dict[str, Any]:
        document = deepcopy(self.document)
        document["migration"]["status"] = "completed"
        document["migration"]["unresolved_items"] = []
        return document

    def assertRejected(
        self, document: dict[str, Any], needle: str | None = None
    ) -> None:  # noqa: N802
        result = self._result(document)
        self.assertFalse(result["passed"], result)
        if needle is not None:
            self.assertTrue(
                any(needle in error for error in result["errors"]),
                result,
            )

    def test_schema_is_valid_draft_2020_12(self) -> None:
        schema = strict_load_json(SCHEMA_PATH)
        Draft202012Validator.check_schema(schema)
        self.assertEqual(
            schema["$schema"], "https://json-schema.org/draft/2020-12/schema"
        )
        self.assertEqual(schema["properties"]["schema_version"]["const"], 2)

    def test_realistic_migration_fixture_is_valid_but_explicitly_not_promoted(
        self,
    ) -> None:
        result = self._result(self.document)

        self.assertTrue(result["passed"], result)
        self.assertTrue(result["schema_valid"], result)
        self.assertTrue(result["semantic_valid"], result)
        self.assertFalse(result["qualification_ready"], result)
        self.assertEqual(self.document["migration"]["status"], "compatibility_only")
        self.assertGreater(len(self.document["migration"]["unresolved_items"]), 0)

    def test_strict_fixture_verifies_link_and_bytes_without_promoting_compatibility(
        self,
    ) -> None:
        result = self._strict_result(self.document)

        self.assertTrue(result["passed"], result)
        self.assertTrue(result["registry_verified"], result)
        self.assertTrue(result["receipts_verified"], result)
        self.assertFalse(result["qualification_ready"], result)

    def test_method_cell_link_matches_authoritative_capability_registry_v2(
        self,
    ) -> None:
        if not AUTHORITATIVE_REGISTRY.is_file():
            self.skipTest(
                "Capability Registry V2 is not present in this isolated checkout"
            )
        registry = strict_load_json(AUTHORITATIVE_REGISTRY)

        result = validate_spec_document(
            self.document,
            registry_document=registry,
            require_registry=True,
        )

        self.assertTrue(result["passed"], result)
        self.assertTrue(result["registry_verified"], result)
        self.assertFalse(result["qualification_ready"], result)

    def test_explicit_completed_migration_needs_both_external_verifications(
        self,
    ) -> None:
        completed = self._completed()

        non_strict = self._result(completed)
        self.assertFalse(non_strict["passed"], non_strict)
        self.assertTrue(
            any(
                "compatibility fixture receipts" in error
                for error in non_strict["errors"]
            ),
            non_strict,
        )

        candidate = deepcopy(completed)
        for receipt in candidate["evidence_contract"]["receipts"]:
            receipt["evidence_class"] = "qualification"
        candidate_non_strict = self._result(candidate)
        fixture_strict = self._strict_result(candidate)

        self.assertTrue(candidate_non_strict["passed"], candidate_non_strict)
        self.assertFalse(
            candidate_non_strict["qualification_ready"], candidate_non_strict
        )
        self.assertFalse(fixture_strict["passed"], fixture_strict)
        self.assertFalse(fixture_strict["qualification_ready"], fixture_strict)
        self.assertTrue(
            any("fixture-only" in error for error in fixture_strict["errors"]),
            fixture_strict,
        )

        with tempfile.TemporaryDirectory() as temporary_directory:
            root = Path(temporary_directory)
            for receipt in candidate["evidence_contract"]["receipts"]:
                relative = f"receipts/{receipt['role']}.json"
                content = (
                    json.dumps(
                        {
                            "receipt_schema_version": 2,
                            "role": receipt["role"],
                            "stage": receipt["stage"],
                            "test_generated": True,
                        },
                        sort_keys=True,
                    )
                    + "\n"
                ).encode("utf-8")
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(content)
                receipt["path"] = relative
                receipt["size_bytes"] = len(content)
                receipt["sha256"] = hashlib.sha256(content).hexdigest()

            strict = validate_spec_document(
                candidate,
                repository_root=root,
                verify_receipts=True,
                registry_document=self.registry,
                require_registry=True,
            )

        self.assertTrue(strict["passed"], strict)
        self.assertTrue(strict["qualification_ready"], strict)

    def test_report_only_adapter_never_promotes_legacy_release_qualified(self) -> None:
        # The live legacy manifest is evidence-reconciled and may truthfully be
        # absent.  Keep this adversarial adapter case synthetic so the test
        # continues to prove that even a legacy release claim has no V2
        # promotion authority without coupling the assertion to live state.
        legacy_release_qualified = deepcopy(self.legacy)
        legacy_release_qualified["qualification"]["declared_state"] = (
            "release_qualified"
        )
        self.assertEqual(
            legacy_release_qualified["qualification"]["declared_state"],
            "release_qualified",
        )

        report = adapt_v1_manifest_report(
            legacy_release_qualified,
            source_manifest_path="test-fixtures/legacy-release-qualified.manifest.json",
        )

        self.assertEqual(
            report["source_identity"]["declared_state"], "release_qualified"
        )
        self.assertTrue(report["source_declared_state_is_informational_only"])
        self.assertEqual(report["v2_coverage_status"], "unassessed")
        self.assertFalse(report["promotion_authority"])
        self.assertFalse(report["qualification_ready"])
        self.assertIsNone(report["capability_cell_candidate"])
        self.assertGreater(len(report["unresolved_v2_requirements"]), 0)
        self.assertNotIn("schema_version", report)
        projected_validation = validate_spec_document(report)
        self.assertFalse(projected_validation["schema_valid"], projected_validation)

    def test_adapter_rejects_malformed_legacy_contracts_and_invalid_cell_links(
        self,
    ) -> None:
        malformed = deepcopy(self.legacy)
        malformed["scientific_contract"] = []
        with self.assertRaisesRegex(ValueError, "scientific_contract"):
            adapt_v1_manifest_report(malformed, source_manifest_path="legacy.json")

        with self.assertRaisesRegex(ValueError, "registry_schema_version"):
            adapt_v1_manifest_report(
                self.legacy,
                source_manifest_path="legacy.json",
                capability_cell={
                    "registry_schema_version": 1,
                    "capability_id": "smartpls.pls_algorithm",
                    "capability_version": "pls_pm_v1",
                    "cell_id": "qpls3.pls.algorithm",
                },
            )

    def test_every_comparison_rule_wire_shape_is_supported(self) -> None:
        for rule, parameters in COMPARISON_RULES.items():
            with self.subTest(rule=rule):
                document = deepcopy(self.document)
                row = next(
                    item
                    for item in document["comparison_contract"]["outputs"]
                    if item["output_id"] == "r_squared"
                )
                row.clear()
                row.update(
                    {
                        "output_id": "r_squared",
                        "rule": rule,
                        "rationale": f"Unit-test rationale for the {rule} wire shape.",
                        **parameters,
                    }
                )

                result = self._result(document)

                self.assertTrue(result["passed"], result)

    def test_comparison_rules_reject_missing_and_irrelevant_parameters(self) -> None:
        missing = deepcopy(self.document)
        loading = next(
            row
            for row in missing["comparison_contract"]["outputs"]
            if row["output_id"] == "outer_loadings"
        )
        del loading["absolute_tolerance"]
        self.assertRejected(missing, "lacks parameters")

        irrelevant = deepcopy(self.document)
        convergence = next(
            row
            for row in irrelevant["comparison_contract"]["outputs"]
            if row["output_id"] == "convergence_status"
        )
        convergence["absolute_tolerance"] = 0
        self.assertRejected(irrelevant, "irrelevant parameters")

        monte_carlo = deepcopy(self.document)
        recovery = next(
            row
            for row in monte_carlo["comparison_contract"]["outputs"]
            if row["output_id"] == "recovery_coverage"
        )
        recovery["confidence_level"] = 0.9
        self.assertRejected(monte_carlo, "must match the scenario Monte Carlo policy")

        unexplained = deepcopy(self.document)
        unexplained["comparison_contract"]["outputs"][0]["rationale"] = "   "
        self.assertRejected(unexplained, "schema")

    def test_scientific_contract_mutations_fail_closed(self) -> None:
        duplicate_output = deepcopy(self.document)
        duplicate_output["scientific_contract"]["estimands"][1]["output_ids"].append(
            "outer_weights"
        )
        self.assertRejected(duplicate_output, "assigned to multiple estimands")

        unordered = deepcopy(self.document)
        unordered["scientific_contract"]["preprocessing"][1]["order"] = 0
        self.assertRejected(unordered, "orders must be unique")

        empty_preprocessing_scope = deepcopy(self.document)
        empty_preprocessing_scope["scientific_contract"]["preprocessing"][0][
            "applies_to"
        ] = []
        self.assertRejected(empty_preprocessing_scope, "schema")

        duplicate_diagnostic = deepcopy(self.document)
        duplicate_diagnostic["scientific_contract"]["data_predicates"][0][
            "diagnostic_code"
        ] = duplicate_diagnostic["scientific_contract"]["model_predicates"][0][
            "diagnostic_code"
        ]
        self.assertRejected(
            duplicate_diagnostic, "diagnostic codes must be globally unique"
        )

        one_group = deepcopy(self.document)
        one_group["scientific_contract"]["oracles"][2]["independence_group"] = (
            one_group["scientific_contract"]["oracles"][1]["independence_group"]
        )
        self.assertRejected(one_group, "two computational sources")

        duplicate_implementation = deepcopy(self.document)
        duplicate_implementation["scientific_contract"]["oracles"][2][
            "implementation"
        ] = deepcopy(
            duplicate_implementation["scientific_contract"]["oracles"][1][
                "implementation"
            ]
        )
        self.assertRejected(duplicate_implementation, "same implementation identity")

        missing_primary = deepcopy(self.document)
        missing_primary["scientific_contract"]["oracles"][0][
            "covered_estimand_ids"
        ].remove("recovery_quality")
        self.assertRejected(missing_primary, "lacks a primary-literature oracle")

    def test_scenario_axes_profiles_and_combinations_fail_closed(self) -> None:
        missing_axis = deepcopy(self.document)
        workers = next(
            row
            for row in missing_axis["scenario_contract"]["axes"]
            if row["id"] == "workers"
        )
        workers["id"] = "worker_count"
        self._rebind_scenario_receipts(missing_axis)
        self.assertRejected(missing_axis, "missing mandatory scenario axes")

        duplicate_profile = deepcopy(self.document)
        duplicate_profile["scenario_contract"]["complexity_profiles"][2]["id"] = (
            "applied"
        )
        self._rebind_scenario_receipts(duplicate_profile)
        self.assertRejected(duplicate_profile, "complexity profile id")

        pairwise_gap = deepcopy(self.document)
        pairwise = next(
            row
            for row in pairwise_gap["scenario_contract"]["mandatory_combinations"]
            if row["coverage"] == "pairwise"
        )
        pairwise["selections"]["workers"] = ["one_worker"]
        self._rebind_scenario_receipts(pairwise_gap)
        self.assertRejected(pairwise_gap, "pairwise coverage misses")

        singleton_axis = deepcopy(self.document)
        topology = next(
            row
            for row in singleton_axis["scenario_contract"]["axes"]
            if row["id"] == "model_topology"
        )
        topology["values"] = topology["values"][:1]
        pairwise = next(
            row
            for row in singleton_axis["scenario_contract"]["mandatory_combinations"]
            if row["coverage"] == "pairwise"
        )
        pairwise["selections"]["model_topology"] = ["chain"]
        self._rebind_scenario_receipts(singleton_axis)
        self.assertRejected(singleton_axis, "requires at least two values")

        missing_compound = deepcopy(self.document)
        compound = next(
            row
            for row in missing_compound["scenario_contract"]["mandatory_combinations"]
            if row["profile_id"] == "compound_stress"
        )
        compound["coverage"] = "targeted"
        self._rebind_scenario_receipts(missing_compound)
        self.assertRejected(missing_compound, "explicit compound mandatory combination")

        missing_maximum_dimension = deepcopy(self.document)
        missing_maximum_dimension["scenario_contract"]["mandatory_combinations"] = [
            row
            for row in missing_maximum_dimension["scenario_contract"][
                "mandatory_combinations"
            ]
            if row["id"] != "maximum_constructs_limit"
        ]
        self._rebind_scenario_receipts(missing_maximum_dimension)
        self.assertRejected(
            missing_maximum_dimension,
            "maximum-axis combinations omit stressed dimensions",
        )

        weak_compound = deepcopy(self.document)
        compound = next(
            row
            for row in weak_compound["scenario_contract"]["mandatory_combinations"]
            if row["profile_id"] == "compound_stress"
        )
        compound["stressed_dimensions"] = ["rows"]
        self._rebind_scenario_receipts(weak_compound)
        self.assertRejected(weak_compound, "must name at least two stressed dimensions")

        non_monotonic = deepcopy(self.document)
        applied = next(
            row
            for row in non_monotonic["scenario_contract"]["complexity_profiles"]
            if row["id"] == "applied"
        )
        applied["workload"]["rows"] = 1
        self._rebind_scenario_receipts(non_monotonic)
        self.assertRejected(non_monotonic, "must be non-decreasing")

    def test_performance_archive_export_windows_and_cancellation_fail_closed(
        self,
    ) -> None:
        missing_budget = deepcopy(self.document)
        missing_budget["operational_contract"]["performance"]["budgets"].pop()
        self.assertRejected(missing_budget, "required performance budget is missing")

        duplicate_budget = deepcopy(self.document)
        duplicate_budget["operational_contract"]["performance"]["budgets"].append(
            deepcopy(
                duplicate_budget["operational_contract"]["performance"]["budgets"][0]
            )
        )
        self.assertRejected(duplicate_budget, "is duplicated")

        weak_maximum_budget = deepcopy(self.document)
        maximum_budget = next(
            row
            for row in weak_maximum_budget["operational_contract"]["performance"][
                "budgets"
            ]
            if row["profile_id"] == "maximum_axis"
        )
        maximum_budget["maximum_elapsed_seconds"] = 100
        self.assertRejected(weak_maximum_budget, "must not be below large")

        bad_archive = deepcopy(self.document)
        bad_archive["operational_contract"]["archive"]["writable_schema_versions"] = [4]
        self.assertRejected(bad_archive, "subset of readable")

        future_archive = deepcopy(self.document)
        future_archive["operational_contract"]["archive"][
            "readable_schema_versions"
        ].append(4)
        self.assertRejected(future_archive, "cannot exceed the current version")

        missing_png = deepcopy(self.document)
        missing_png["operational_contract"]["export"]["formats"].remove("png")
        self.assertRejected(missing_png, "export contract is missing formats")

        missing_readback = deepcopy(self.document)
        missing_readback["operational_contract"]["export"][
            "semantic_readback_formats"
        ].remove("pdf")
        self.assertRejected(
            missing_readback, "semantic read-back contract is missing formats"
        )

        missing_scale = deepcopy(self.document)
        missing_scale["operational_contract"]["windows"][
            "display_scale_percent"
        ].remove(200)
        self.assertRejected(missing_scale, "missing 100, 125, 150, or 200")

        uncancellable_estimate = deepcopy(self.document)
        estimate = next(
            row
            for row in uncancellable_estimate["operational_contract"]["cancellation"][
                "phases"
            ]
            if row["phase"] == "estimate"
        )
        estimate["applicability"] = "not_applicable"
        estimate["not_applicable_reason"] = "mutation"
        self.assertRejected(
            uncancellable_estimate, "require cancellation during estimation"
        )

        omitted_phase = deepcopy(self.document)
        omitted_phase["operational_contract"]["cancellation"]["phases"] = [
            row
            for row in omitted_phase["operational_contract"]["cancellation"]["phases"]
            if row["phase"] != "compare"
        ]
        self.assertRejected(omitted_phase, "cancellation contract omits phases")

        slow_cancel = deepcopy(self.document)
        slow_cancel["operational_contract"]["performance"]["budgets"][0][
            "maximum_cancellation_latency_seconds"
        ] = 2
        self.assertRejected(slow_cancel, "allows cancellation slower")

    def test_receipt_chain_is_identity_bound_and_fail_closed(self) -> None:
        wrong_identity = deepcopy(self.document)
        wrong_identity["evidence_contract"]["receipts"][0]["cell_id"] = (
            "qpls3.other.cell"
        )
        self.assertRejected(wrong_identity, "cell_id mismatch")

        wrong_scenarios = deepcopy(self.document)
        wrong_scenarios["evidence_contract"]["receipts"][0]["scenario_set_sha256"] = (
            "0" * 64
        )
        self.assertRejected(
            wrong_scenarios, "does not bind the frozen scenario contract"
        )

        inconsistent_build = deepcopy(self.document)
        inconsistent_build["evidence_contract"]["receipts"][0]["build_fingerprint"] = (
            "other-build"
        )
        self.assertRejected(inconsistent_build, "disagree on build_fingerprint")

        incomplete_identity_contract = self._completed()
        incomplete_identity_contract["evidence_contract"]["receipt_contract"][
            "identity_fields"
        ].remove("source_set_sha256")
        self.assertRejected(
            incomplete_identity_contract,
            "receipt identity fields must be the complete mandatory set",
        )

        reused_path = deepcopy(self.document)
        reused_path["evidence_contract"]["receipts"][1]["path"] = reused_path[
            "evidence_contract"
        ]["receipts"][0]["path"]
        self.assertRejected(reused_path, "reused by multiple roles")

        non_windows = deepcopy(self.document)
        packaged = next(
            receipt
            for receipt in non_windows["evidence_contract"]["receipts"]
            if receipt["stage"] == "packaged_windows"
        )
        packaged["hardware_fingerprint"]["os"] = "Linux"
        self.assertRejected(non_windows, "must be captured on Windows")

        predates_freeze = deepcopy(self.document)
        predates_freeze["evidence_contract"]["receipts"][0]["generated_at_utc"] = (
            "2026-08-13T19:59:59Z"
        )
        self.assertRejected(predates_freeze, "predates the frozen")

        future_receipt = deepcopy(self.document)
        future_receipt["evidence_contract"]["receipts"][0]["generated_at_utc"] = (
            "2099-01-01T00:00:00Z"
        )
        self.assertRejected(future_receipt, "implausible future timestamp")

    def test_completed_migration_requires_every_receipt_stage(self) -> None:
        completed = self._completed()
        completed["evidence_contract"]["receipts"] = [
            row
            for row in completed["evidence_contract"]["receipts"]
            if row["stage"] != "contract"
        ]
        completed["evidence_contract"]["required_roles"].remove("method_contract")

        self.assertRejected(completed, "must cover every qualification stage")

    def test_strict_registry_and_receipt_mutations_fail_closed(self) -> None:
        completed = self._completed()
        bad_registry = deepcopy(self.registry)
        bad_row = bad_registry["capabilities"][0]
        bad_row["qualification_links"][0]["capability_version"] = "other_method"
        bad_row["option_cells"][0]["capability_version"] = "other_method"
        bad_row["option_cells"][0]["qualification_spec"]["links"][0][
            "capability_version"
        ] = "other_method"
        registry_result = validate_spec_document(
            completed,
            repository_root=REPOSITORY_ROOT,
            verify_receipts=True,
            registry_document=bad_registry,
            require_registry=True,
        )
        self.assertFalse(registry_result["passed"], registry_result)
        self.assertFalse(registry_result["qualification_ready"], registry_result)
        self.assertTrue(
            any(
                "exactly one qualification link" in error
                for error in registry_result["errors"]
            ),
            registry_result,
        )

        bad_receipt = deepcopy(completed)
        bad_receipt["evidence_contract"]["receipts"][0]["sha256"] = "0" * 64
        receipt_result = self._strict_result(bad_receipt)
        self.assertFalse(receipt_result["passed"], receipt_result)
        self.assertFalse(receipt_result["receipts_verified"], receipt_result)
        self.assertTrue(
            any("SHA-256 mismatch" in error for error in receipt_result["errors"]),
            receipt_result,
        )

        path_escape = deepcopy(completed)
        path_escape["evidence_contract"]["receipts"][0]["path"] = "../outside.json"
        escape_result = self._strict_result(path_escape)
        self.assertFalse(escape_result["passed"], escape_result)
        self.assertFalse(escape_result["qualification_ready"], escape_result)

    def test_migration_state_mutations_fail_closed(self) -> None:
        empty_unresolved = deepcopy(self.document)
        empty_unresolved["migration"]["unresolved_items"] = []
        self.assertRejected(empty_unresolved, "must name unresolved V2 requirements")

        false_completion = deepcopy(self.document)
        false_completion["migration"]["status"] = "completed"
        self.assertRejected(false_completion, "cannot retain unresolved")

        false_native = deepcopy(self.document)
        false_native["migration"]["source_kind"] = "native_v2"
        false_native["migration"]["status"] = "native"
        self.assertRejected(false_native, "no legacy source manifest")

    def test_duplicate_keys_and_non_finite_json_are_rejected_before_validation(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            duplicate_path = Path(temporary_directory) / "duplicate.json"
            duplicate_path.write_text(
                '{"schema_version": 2, "schema_version": 2}',
                encoding="utf-8",
            )
            with self.assertRaises(DuplicateKeyError):
                strict_load_json(duplicate_path)
            duplicate_report = validate_spec_path(duplicate_path)
            self.assertFalse(duplicate_report["passed"], duplicate_report)
            self.assertTrue(
                any(
                    "DuplicateKeyError" in error for error in duplicate_report["errors"]
                ),
                duplicate_report,
            )

            nan_path = Path(temporary_directory) / "nan.json"
            nan_path.write_text('{"value": NaN}', encoding="utf-8")
            nan_report = validate_spec_path(nan_path)
            self.assertFalse(nan_report["passed"], nan_report)
            self.assertTrue(
                any("non-finite JSON value" in error for error in nan_report["errors"]),
                nan_report,
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)
