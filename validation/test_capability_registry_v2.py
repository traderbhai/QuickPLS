from __future__ import annotations

import copy
import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from capability_registry_v2 import (  # noqa: E402
    DEFAULT_LEGACY_CATALOGUE_PATH,
    DEFAULT_REGISTRY_PATH,
    DEFAULT_SCHEMA_PATH,
    EXPECTED_COVERAGE_COUNTS,
    build_qualification_link_index,
    canonical_sha256,
    check_legacy_catalogue,
    cross_validate_manifest_evidence,
    cross_validate_qualification_links,
    derive_legacy_status,
    derive_row_projection,
    generate_legacy_catalogue,
    load_json,
    load_registry,
    lookup_capability,
    lookup_option_cell,
    lookup_qualification_link,
    qualification_link_identity,
    resolve_customer_visibility,
    validate_registry_document,
    validate_schema_contract,
    write_generated_legacy_catalogue,
)
from qualification_spec_v2 import validate_spec_path  # noqa: E402


class CapabilityRegistryV2Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.registry = load_registry(DEFAULT_REGISTRY_PATH)
        cls.schema = load_json(DEFAULT_SCHEMA_PATH)
        cls.legacy = load_json(DEFAULT_LEGACY_CATALOGUE_PATH)

    def test_authoritative_baseline_and_local_references_pass(self):
        report = validate_registry_document(
            self.registry,
            repository_root=ROOT,
            check_references=True,
            schema=self.schema,
        )
        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["capability_row_count"], 45)
        self.assertEqual(report["active_row_count"], 43)
        self.assertEqual(report["coverage_counts"], EXPECTED_COVERAGE_COUNTS)
        self.assertEqual(report["qualification_link_count"], 50)
        self.assertEqual(report["option_cell_count"], 50)
        self.assertEqual(
            report["option_cell_coverage_counts"],
            {"full": 0, "partial": 37, "absent": 11, "intentionally_excluded": 2},
        )
        self.assertEqual(
            report["evidence_counts"],
            {
                "absent": 16,
                "engine_only": 3,
                "archive_qualified": 0,
                "native_qualified": 0,
                "release_qualified": 26,
            },
        )
        self.assertEqual(
            report["option_cell_evidence_counts"],
            {
                "absent": 16,
                "engine_only": 3,
                "archive_qualified": 2,
                "native_qualified": 0,
                "release_qualified": 29,
            },
        )
        self.assertTrue(report["manifest_evidence_check"]["passed"])
        self.assertEqual(report["manifest_evidence_check"]["mapped_cell_count"], 47)
        self.assertEqual(report["manifest_evidence_check"]["unique_manifest_count"], 41)
        self.assertEqual(
            report["manifest_evidence_check"]["derived_cell_evidence_counts"],
            {
                "absent": 18,
                "engine_only": 2,
                "archive_qualified": 2,
                "native_qualified": 0,
                "release_qualified": 25,
            },
        )
        self.assertEqual(
            report["surface_counts"],
            {"standard": 26, "labs": 17, "legacy": 2, "internal": 0},
        )
        self.assertEqual(
            report["option_cell_surface_counts"],
            {"standard": 29, "labs": 19, "legacy": 2, "internal": 0},
        )

    def test_general_sem_multiple_mediation_bootstrap_is_exact_labs_engine_cell(self):
        cell_id = "qpls3.pls.general_sem_multiple_mediation_bootstrap"
        cell = lookup_option_cell(self.registry, "smartpls.mediation", cell_id)
        self.assertIsNotNone(cell)
        self.assertEqual(cell["capability_version"], "general_sem_pls_full_model_case_bootstrap_v1")
        self.assertEqual(cell["coverage_state"], "partial")
        self.assertEqual(cell["evidence_state"], "engine_only")
        self.assertEqual(cell["surface"], "labs")
        quickpls_predicates = cell["supported_model_predicate"]["quickpls"]
        self.assertIn("indirect_paths:min_2", quickpls_predicates)
        self.assertIn("inference:full_model_case_bootstrap", quickpls_predicates)
        self.assertIn("interval:percentile_type7", quickpls_predicates)
        self.assertIn("tail:two_sided", quickpls_predicates)

        link = cell["qualification_spec"]["links"][0]
        self.assertEqual(
            cell["qualification_spec"]["references"],
            [
                "validation/methods/general_sem_pls_multiple_mediation_bootstrap_v1.manifest.json",
            ],
        )
        owner = lookup_qualification_link(self.registry, link)
        self.assertIsNotNone(owner)
        self.assertEqual(owner["capability_id"], "smartpls.mediation")

        labs = resolve_customer_visibility(
            self.registry,
            "smartpls.mediation",
            cell_id,
        )
        self.assertEqual(labs["channel"], "labs")
        self.assertTrue(labs["requires_opt_in"])
        self.assertTrue(labs["available"])

    def test_general_sem_multiple_moderation_point_is_exact_labs_engine_cell(self):
        cell_id = "qpls3.pls.general_sem_multiple_two_way_moderation_point"
        cell = lookup_option_cell(self.registry, "smartpls.moderation", cell_id)
        self.assertIsNotNone(cell)
        self.assertEqual(
            cell["capability_version"],
            "general_sem_pls_multiple_two_way_moderation_point_v1",
        )
        self.assertEqual(cell["coverage_state"], "partial")
        self.assertEqual(cell["evidence_state"], "engine_only")
        self.assertEqual(cell["surface"], "labs")
        self.assertEqual(
            cell["supported_model_predicate"]["quickpls"],
            [
                "schema_generation:general_sem_v1",
                "derived_term:interaction_v2",
                "interaction_order:two_way",
                "construction:two_stage",
                "hierarchy:strong",
                "interaction_count:one_or_more",
                "structural_topology:direct_only_acyclic",
                "derived_term_scope:interaction_v2_only",
                "higher_order_constructs:none",
                "requested_effect_estimands:none",
                "authored_conditional_effect_probes:none",
                "inference:point_only",
            ],
        )
        self.assertEqual(
            cell["supported_data_predicate"]["quickpls"],
            [
                "input:raw_continuous",
                "missing_data:listwise",
                "weights:none",
                "groups:single",
            ],
        )

        manifest = load_json(
            ROOT
            / "validation/capabilities/general_sem_pls_multiple_moderation_point_v1.cell.manifest.json"
        )
        self.assertEqual(manifest["contract_kind"], "capability_cell_contract")
        self.assertEqual(manifest["owner_capability_id"], "smartpls.moderation")
        self.assertEqual(manifest["feature"]["id"], cell_id)
        self.assertEqual(
            manifest["feature"]["method_version"], cell["capability_version"]
        )
        self.assertEqual(
            manifest["feature"]["analytical_method_version"],
            "qpls.general-sem-pls.multiple-two-way.point.v1",
        )
        self.assertFalse(manifest["qualification_ready"])
        self.assertFalse(manifest["promotion_allowed"])
        self.assertEqual(
            manifest["exact_predicate"],
            {
                "schema_generation": "general_sem_v1",
                "derived_term": "interaction_v2",
                "interaction_order": "two_way",
                "construction": "two_stage",
                "hierarchy": "strong",
                "interaction_count": "one_or_more",
                "structural_topology": "direct_only_acyclic",
                "derived_term_scope": "interaction_v2_only",
                "higher_order_constructs": "none",
                "requested_effect_estimands": "none",
                "authored_conditional_effect_probes": "none",
                "inference": "point_only",
                "input": "raw_continuous",
                "missing_data": "listwise",
                "weights": "none",
                "groups": "single",
            },
        )
        self.assertEqual(
            cell["qualification_spec"]["references"],
            [
                "validation/methods/general_sem_pls_multiple_moderation_point_v1.manifest.json",
            ],
        )

        labs = resolve_customer_visibility(
            self.registry,
            "smartpls.moderation",
            cell_id,
        )
        self.assertEqual(labs["channel"], "labs")
        self.assertTrue(labs["requires_opt_in"])
        self.assertTrue(labs["available"])

    def test_current_status_policy_cannot_delegate_to_the_historical_parity_ledger(self):
        policy = self.registry["comparison_policy"]["status_source_of_truth"]
        self.assertIn("Capability Registry V2 is the current option-cell authority", policy)
        self.assertIn("fail-closed against the live state", policy)
        self.assertIn("historical parity ledger", policy)
        self.assertIn("non-authoritative", policy)
        self.assertNotIn("derive status from that evidence-backed ledger", policy)

    def test_schema_contract_is_frozen_and_registry_matches_json_schema(self):
        report = validate_schema_contract(self.schema)
        self.assertTrue(report["passed"], report["errors"])
        if importlib.util.find_spec("jsonschema") is None:
            self.skipTest("jsonschema is not installed; internal schema contract passed")
        import jsonschema

        jsonschema.Draft202012Validator.check_schema(self.schema)
        errors = sorted(
            jsonschema.Draft202012Validator(self.schema).iter_errors(self.registry),
            key=lambda error: tuple(str(part) for part in error.absolute_path),
        )
        self.assertEqual([], [error.message for error in errors])

    def test_legacy_projection_is_exact_and_deterministic(self):
        first = generate_legacy_catalogue(self.registry)
        second = generate_legacy_catalogue(self.registry)
        self.assertEqual(first, second)
        self.assertEqual(first, self.legacy)
        self.assertEqual(canonical_sha256(first), canonical_sha256(second))
        report = check_legacy_catalogue(self.registry, self.legacy)
        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["generated_sha256"], report["actual_sha256"])

    def test_legacy_status_is_derived_from_v2_coverage_and_evidence(self):
        engine = lookup_capability(self.registry, "smartpls.nonlinear_relationships")
        self.assertEqual(engine["evidence_state"], "engine_only")
        self.assertEqual(derive_legacy_status(engine), "engine-preview")

        archived = copy.deepcopy(engine)
        archived["option_cells"][0]["evidence_state"] = "archive_qualified"
        self.assertEqual(derive_legacy_status(archived), "engine-preview")
        changed = copy.deepcopy(self.registry)
        changed_engine = lookup_capability(
            changed, "smartpls.nonlinear_relationships"
        )
        changed_engine["option_cells"][0]["evidence_state"] = "archive_qualified"
        changed_engine["legacy_row"]["status"] = "release-qualified"
        projected = generate_legacy_catalogue(changed)
        self.assertEqual(
            projected["methods"][changed_engine["catalogue_position"] - 1]["status"],
            "engine-preview",
        )

        excluded = lookup_capability(self.registry, "smartpls.gof")
        self.assertEqual(excluded["evidence_state"], "absent")
        self.assertEqual(derive_legacy_status(excluded), "deferred")

    def test_legacy_drift_is_detected_without_writing(self):
        before = DEFAULT_LEGACY_CATALOGUE_PATH.read_bytes()
        changed = copy.deepcopy(self.legacy)
        changed["methods"][0]["remaining_gap"] += " drift"
        report = check_legacy_catalogue(self.registry, changed)
        self.assertFalse(report["passed"])
        self.assertIn("legacy row 1 differs", report["errors"][0])
        self.assertEqual(before, DEFAULT_LEGACY_CATALOGUE_PATH.read_bytes())

    def test_legacy_catalogue_is_an_atomic_generated_report(self):
        with tempfile.TemporaryDirectory() as directory:
            target = Path(directory) / "generated" / "competitor_catalogue.json"
            first = write_generated_legacy_catalogue(self.registry, target)
            first_bytes = target.read_bytes()
            second = write_generated_legacy_catalogue(self.registry, target)

            self.assertTrue(first["passed"])
            self.assertEqual(first["row_count"], 45)
            self.assertEqual(first["sha256"], second["sha256"])
            self.assertEqual(first_bytes, target.read_bytes())
            self.assertEqual(load_json(target), self.legacy)
            self.assertFalse(list(target.parent.glob("*.tmp")))

    def test_qualification_link_shape_lookup_and_external_cross_validation(self):
        row = lookup_capability(self.registry, "smartpls.pls_algorithm")
        self.assertIsNotNone(row)
        link = row["qualification_links"][0]
        self.assertEqual(
            set(link),
            {
                "registry_schema_version",
                "capability_id",
                "cell_id",
                "capability_version",
            },
        )
        self.assertEqual(
            qualification_link_identity(link),
            (2, "smartpls.pls_algorithm", "qpls3.pls.algorithm", "pls_pm_v1"),
        )
        self.assertEqual(lookup_qualification_link(self.registry, link), row)
        report = cross_validate_qualification_links(self.registry, [link])
        self.assertTrue(report["passed"], report["errors"])
        self.assertEqual(report["external_link_count"], 1)

    def test_pls_sample_size_row_has_two_distinct_scoped_standard_cells(self):
        row = lookup_capability(self.registry, "smartpls.pls_power_analysis")
        self.assertEqual(row["coverage_state"], "partial")
        self.assertEqual(row["evidence_state"], "release_qualified")
        self.assertEqual(row["surface"], "standard")

        links = {link["cell_id"]: link for link in row["qualification_links"]}
        self.assertEqual(
            set(links),
            {
                "qpls3.pls.posthoc_technical_minimum_sample_size",
                "qpls3.pls.sample_size_power",
            },
        )
        posthoc = links["qpls3.pls.posthoc_technical_minimum_sample_size"]
        prospective = links["qpls3.pls.sample_size_power"]
        posthoc_cell = lookup_option_cell(
            self.registry,
            "smartpls.pls_power_analysis",
            "qpls3.pls.posthoc_technical_minimum_sample_size",
        )
        prospective_cell = lookup_option_cell(
            self.registry,
            "smartpls.pls_power_analysis",
            "qpls3.pls.sample_size_power",
        )
        self.assertEqual(posthoc_cell["coverage_state"], "partial")
        self.assertEqual(posthoc_cell["evidence_state"], "release_qualified")
        self.assertEqual(posthoc_cell["surface"], "standard")
        self.assertEqual(prospective_cell["capability_version"], "pls_sample_size_power_v2")
        self.assertEqual(prospective_cell["coverage_state"], "partial")
        self.assertEqual(prospective_cell["evidence_state"], "release_qualified")
        self.assertEqual(prospective_cell["surface"], "standard")
        self.assertEqual(
            derive_row_projection(row),
            {
                "coverage_state": "partial",
                "evidence_state": "release_qualified",
                "surface": "standard",
            },
        )
        self.assertEqual(
            posthoc["capability_version"],
            "pls_posthoc_technical_minimum_sample_size_v2",
        )
        self.assertEqual(prospective["capability_version"], "pls_sample_size_power_v2")
        self.assertIs(lookup_qualification_link(self.registry, prospective), row)
        self.assertIs(lookup_qualification_link(self.registry, posthoc), row)
        self.assertIn(
            "docs/methods/PLS_POSTHOC_TECHNICAL_MINIMUM_SAMPLE_SIZE_V2.md",
            row["result_references"],
        )
        self.assertIn(
            "docs/methods/PLS_SAMPLE_SIZE_POWER_V2.md",
            row["result_references"],
        )
        self.assertTrue(
            any("complete two-sided" in item for item in row["known_differences"])
        )
        self.assertEqual(row["legacy_row"]["status"], "release-qualified")
        self.assertEqual(row["legacy_row"]["target_release"], "current")
        self.assertEqual(row["legacy_row"]["implementation_evidence"], [])
        self.assertTrue(
            (ROOT / "validation/pls_posthoc_technical_sample_size_contract.py").is_file()
        )

        contract_path = (
            ROOT
            / "validation/capabilities/pls_posthoc_technical_minimum_sample_size_v2.cell.manifest.json"
        )
        contract = load_json(contract_path)
        self.assertEqual(contract["contract_kind"], "capability_cell_contract")
        self.assertEqual(contract["owner_capability_id"], row["capability_id"])
        self.assertEqual(
            contract["execution_context"],
            "pls_algorithm_result_with_optional_bootstrap_inference",
        )
        self.assertEqual(contract["feature"]["id"], posthoc["cell_id"])
        self.assertEqual(
            contract["feature"]["method_version"], posthoc["capability_version"]
        )
        self.assertEqual(contract["coverage_state"], "partial")
        self.assertEqual(contract["evidence_state"], "release_qualified")
        self.assertEqual(contract["availability"], "scoped_standard")
        self.assertEqual(
            contract["distinct_from_cell_id"], "qpls3.pls.sample_size_power"
        )

    def test_source_qualification_specs_are_linked_exactly_at_their_declared_surfaces(self):
        cases = (
            (
                "smartpls.pls_power_analysis",
                "qpls3.pls.posthoc_technical_minimum_sample_size",
                "pls_posthoc_technical_minimum_sample_size_v2",
                "partial",
                "release_qualified",
                "standard",
                "validation/qualification_v2/pls_posthoc_technical_minimum_sample_size_v2.qualification.json",
            ),
            (
                "smartpls.consistent_bootstrapping",
                "qpls3.inference.consistent_bootstrap",
                "plsc_bootstrap_v1",
                "partial",
                "release_qualified",
                "standard",
                "validation/methods/consistent_bootstrap_v1.manifest.json",
            ),
            (
                "smartpls.htmt",
                "qpls3.assessment.htmt",
                "ringle_et_al_htmt_plus_v1",
                "absent",
                "absent",
                "labs",
                "validation/qualification_v2/htmt_plus_v1.qualification.json",
            ),
            (
                "smartpls.model_fit",
                "qpls3.assessment.model_fit",
                "pls_model_fit_v2",
                "partial",
                "absent",
                "labs",
                "validation/qualification_v2/pls_model_fit_exact_v1.qualification.json",
            ),
        )

        for capability_id, cell_id, version, coverage, evidence, surface, reference in cases:
            with self.subTest(cell_id=cell_id):
                row = lookup_capability(self.registry, capability_id)
                cell = lookup_option_cell(self.registry, capability_id, cell_id)
                self.assertEqual(cell["capability_version"], version)
                self.assertEqual(cell["coverage_state"], coverage)
                self.assertEqual(cell["evidence_state"], evidence)
                self.assertEqual(cell["surface"], surface)
                self.assertIn(reference, cell["qualification_spec"]["references"])
                self.assertIn(reference, row["qualification_references"])

                specification = load_json(ROOT / reference)
                self.assertEqual(specification["evidence_contract"]["receipts"], [])
                report = validate_spec_path(
                    ROOT / reference,
                    repository_root=ROOT,
                    registry_path=DEFAULT_REGISTRY_PATH,
                    require_registry=True,
                )
                self.assertTrue(report["passed"], report["errors"])
                self.assertTrue(report["registry_verified"], report["errors"])
                self.assertFalse(report["receipts_verified"])
                self.assertFalse(report["qualification_ready"])

    def test_consistent_permutation_source_foundation_remains_absent(self):
        row = lookup_capability(self.registry, "smartpls.consistent_permutation")
        cell = lookup_option_cell(
            self.registry,
            "smartpls.consistent_permutation",
            "qpls3.inference.consistent_permutation",
        )

        self.assertEqual(row["coverage_state"], "absent")
        self.assertEqual(row["evidence_state"], "absent")
        self.assertEqual(cell["coverage_state"], "absent")
        self.assertEqual(cell["evidence_state"], "release_qualified")
        self.assertEqual(cell["surface"], "standard")
        self.assertTrue(
            any("two explicit groups" in item for item in row["known_differences"])
        )

        bounded_source_files = {
            "crates/qpls-resampling/src/consistent_permutation.rs",
            "crates/qpls-core/src/validation.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-project/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
            "src/native/nativeConsistentPermutation.ts",
            "src/native/nativeConsistentPermutation.test.ts",
            "docs/methods/CONSISTENT_PERMUTATION_V1.md",
            "validation/methods/consistent_permutation_v1.manifest.json",
        }
        self.assertEqual(row["legacy_row"]["implementation_evidence"], [])
        for relative_path in bounded_source_files:
            self.assertTrue((ROOT / relative_path).is_file(), relative_path)

        manifest = load_json(
            ROOT / "validation/methods/consistent_permutation_v1.manifest.json"
        )
        self.assertEqual(manifest["qualification"]["declared_state"], "absent")
        self.assertTrue(
            all(
                not receipts
                for receipts in manifest["qualification"]["evidence"].values()
            )
        )

    def test_cbsem_matrix_source_isolated_in_registered_absent_cell(self):
        row = lookup_capability(self.registry, "smartpls.cbsem")
        cell = lookup_option_cell(self.registry, "smartpls.cbsem", "qpls3.cbsem.ml")
        matrix_cell = lookup_option_cell(
            self.registry, "smartpls.cbsem", "qpls3.cbsem.ml.matrix_input"
        )
        accepted_data = [
            "availability:partial",
            "scope:bounded",
            "input:raw_numeric",
            "scale:continuous",
            "missing_data:listwise_deletion",
            "groups:single",
        ]

        self.assertEqual(cell["coverage_state"], "partial")
        self.assertEqual(cell["evidence_state"], "release_qualified")
        self.assertEqual(cell["surface"], "standard")
        self.assertEqual(cell["supported_data_predicate"]["quickpls"], accepted_data)
        self.assertEqual(row["data_predicates"]["quickpls"], accepted_data)
        self.assertTrue(
            any(
                "separate partial/absent Labs option cell" in item
                for item in row["known_differences"]
            )
        )
        self.assertTrue(
            any(
                "compatibility-only QualificationSpec V2" in item
                for item in row["known_differences"]
            )
        )
        self.assertEqual(
            [item["cell_id"] for item in row["option_cells"]],
            ["qpls3.cbsem.ml", "qpls3.cbsem.ml.matrix_input"],
        )
        self.assertEqual(matrix_cell["coverage_state"], "partial")
        self.assertEqual(matrix_cell["evidence_state"], "absent")
        self.assertEqual(matrix_cell["surface"], "labs")
        self.assertEqual(
            matrix_cell["supported_data_predicate"]["quickpls"],
            [
                "availability:partial",
                "scope:bounded",
                "input:raw_or_covariance_or_scaled_correlation",
                "scale:continuous",
                "missing_data:listwise_for_raw_and_complete_for_matrix",
                "groups:single",
            ],
        )

    def test_qualification_link_rejects_extra_key_unknown_version_and_duplicates(self):
        row = lookup_capability(self.registry, "smartpls.pls_algorithm")
        link = copy.deepcopy(row["qualification_links"][0])
        link["customer_label"] = "internal leak"
        with self.assertRaisesRegex(ValueError, "exactly"):
            qualification_link_identity(link)

        unknown = copy.deepcopy(row["qualification_links"][0])
        unknown["capability_version"] = "not_registered_v99"
        report = cross_validate_qualification_links(self.registry, [unknown])
        self.assertFalse(report["passed"])
        self.assertIn("identity not found", report["errors"][0])

        changed = copy.deepcopy(self.registry)
        changed["capabilities"][0]["qualification_links"].append(
            copy.deepcopy(changed["capabilities"][0]["qualification_links"][0])
        )
        with self.assertRaisesRegex(ValueError, "duplicate"):
            build_qualification_link_index(changed)

    def test_manifest_version_cross_validation_is_fail_closed(self):
        changed = copy.deepcopy(self.registry)
        changed_row = changed["capabilities"][0]
        changed_row["qualification_links"][0]["capability_version"] = "pls_pm_v999"
        changed_row["qualification_spec"]["links"][0]["capability_version"] = "pls_pm_v999"
        changed_row["option_cells"][0]["capability_version"] = "pls_pm_v999"
        changed_row["option_cells"][0]["qualification_spec"]["links"][0][
            "capability_version"
        ] = "pls_pm_v999"
        report = validate_registry_document(
            changed, repository_root=ROOT, check_references=True, schema=self.schema
        )
        self.assertFalse(report["passed"])
        self.assertTrue(
            any("method_version" in error for error in report["errors"]),
            report["errors"],
        )

    def test_registry_evidence_cannot_exceed_live_manifest_derivation(self):
        baseline = cross_validate_manifest_evidence(self.registry, ROOT)
        self.assertTrue(baseline["passed"], baseline["errors"])

        changed = copy.deepcopy(self.registry)
        cell = lookup_option_cell(
            changed, "smartpls.moderation", "qpls3.pls.moderation"
        )
        cell["evidence_state"] = "native_qualified"
        report = cross_validate_manifest_evidence(changed, ROOT)
        self.assertFalse(report["passed"])
        self.assertTrue(
            any(
                "registry evidence native_qualified exceeds current "
                "manifest-derived evidence archive_qualified" in error
                for error in report["errors"]
            ),
            report["errors"],
        )

    def test_coverage_and_evidence_are_independent_and_counts_are_frozen(self):
        row = lookup_capability(self.registry, "smartpls.pls_algorithm")
        self.assertEqual(row["coverage_state"], "partial")
        self.assertEqual(row["evidence_state"], "release_qualified")
        changed = copy.deepcopy(self.registry)
        changed["capabilities"][0]["coverage_state"] = "full"
        changed["capabilities"][0]["option_cells"][0]["coverage_state"] = "full"
        report = validate_registry_document(changed, check_references=False)
        self.assertFalse(report["passed"])
        self.assertTrue(any("coverage counts differ" in item for item in report["errors"]))

        row_only = copy.deepcopy(self.registry)
        row_only["capabilities"][0]["coverage_state"] = "full"
        row_only_report = validate_registry_document(row_only, check_references=False)
        self.assertFalse(row_only_report["passed"])
        self.assertTrue(
            any("derived option-cell projection" in item for item in row_only_report["errors"]),
            row_only_report["errors"],
        )
        self.assertEqual(row_only_report["coverage_counts"], EXPECTED_COVERAGE_COUNTS)

    def test_customer_visibility_routes_release_qualified_documented_scope_to_standard(self):
        for capability_id in ("smartpls.pca_core", "smartpls.pca_cbsem"):
            pca = resolve_customer_visibility(
                self.registry, capability_id, "qpls3.standalone.pca"
            )
            self.assertEqual(pca["channel"], "standard")
            self.assertFalse(pca["requires_opt_in"])
            self.assertTrue(pca["available"])

        for capability_id in ("smartpls.process", "smartpls.process_bootstrapping"):
            process = resolve_customer_visibility(
                self.registry, capability_id, "qpls3.standalone.process"
            )
            self.assertEqual(process["channel"], "standard")
            self.assertFalse(process["requires_opt_in"])
            self.assertTrue(process["available"])

        regression_bootstrap = resolve_customer_visibility(
            self.registry,
            "smartpls.regression_bootstrapping",
            "qpls3.standalone.regression_bootstrap",
        )
        self.assertEqual(regression_bootstrap["channel"], "standard")
        self.assertFalse(regression_bootstrap["requires_opt_in"])
        self.assertTrue(regression_bootstrap["available"])

        ipma = resolve_customer_visibility(
            self.registry, "smartpls.ipma", "qpls3.assessment.ipma"
        )
        self.assertEqual(ipma["channel"], "standard")
        self.assertFalse(ipma["requires_opt_in"])
        self.assertTrue(ipma["available"])

        bootstrap = resolve_customer_visibility(
            self.registry,
            "smartpls.pls_bootstrapping",
            "qpls3.inference.bootstrap",
        )
        self.assertEqual(bootstrap["channel"], "standard")
        self.assertFalse(bootstrap["requires_opt_in"])
        self.assertTrue(bootstrap["available"])

        higher_order = resolve_customer_visibility(
            self.registry,
            "smartpls.higher_order_models",
            "qpls3.pls.higher_order_two_stage",
        )
        self.assertEqual(higher_order["channel"], "standard")
        self.assertFalse(higher_order["requires_opt_in"])
        self.assertTrue(higher_order["available"])

        for capability_id in ("smartpls.micom", "smartpls.mga"):
            group_workflow = resolve_customer_visibility(
                self.registry,
                capability_id,
                "qpls3.groups.micom_permutation_mga",
            )
            self.assertEqual(group_workflow["channel"], "standard")
            self.assertFalse(group_workflow["requires_opt_in"])
            self.assertTrue(group_workflow["available"])

        for cell_id in (
            "qpls3.groups.micom_permutation_mga",
            "qpls3.inference.structural_path_randomization",
        ):
            permutation = resolve_customer_visibility(
                self.registry, "smartpls.permutation", cell_id
            )
            self.assertEqual(permutation["channel"], "standard")
            self.assertFalse(permutation["requires_opt_in"])
            self.assertTrue(permutation["available"])

        for capability_id in ("smartpls.plspredict", "smartpls.cvpat"):
            prediction = resolve_customer_visibility(
                self.registry,
                capability_id,
                "qpls3.prediction.plspredict_cvpat",
            )
            self.assertEqual(prediction["channel"], "standard")
            self.assertFalse(prediction["requires_opt_in"])
            self.assertTrue(prediction["available"])

        plsc = resolve_customer_visibility(
            self.registry, "smartpls.plsc", "qpls3.pls.consistent"
        )
        self.assertEqual(plsc["channel"], "standard")
        self.assertFalse(plsc["requires_opt_in"])
        self.assertTrue(plsc["available"])

        cta_pls = resolve_customer_visibility(
            self.registry, "smartpls.cta_pls", "qpls3.assessment.cta_pls"
        )
        self.assertEqual(cta_pls["channel"], "standard")
        self.assertFalse(cta_pls["requires_opt_in"])
        self.assertTrue(cta_pls["available"])

        scoped_pls = resolve_customer_visibility(
            self.registry, "smartpls.pls_algorithm", "qpls3.pls.algorithm"
        )
        self.assertEqual(scoped_pls["channel"], "standard")
        self.assertFalse(scoped_pls["requires_opt_in"])
        self.assertTrue(scoped_pls["available"])
        self.assertEqual(scoped_pls["evidence_label_visibility"], "internal_only")
        self.assertNotIn("evidence_state", scoped_pls)

        model_fit = lookup_capability(self.registry, "smartpls.model_fit")
        self.assertEqual(model_fit["coverage_state"], "partial")
        self.assertEqual(model_fit["evidence_state"], "absent")
        self.assertEqual(model_fit["surface"], "labs")
        self.assertEqual(model_fit["legacy_row"]["status"], "absent")
        self.assertEqual(
            model_fit["option_cells"][0]["capability_version"],
            "pls_model_fit_v2",
        )
        self.assertEqual(model_fit["legacy_row"]["implementation_evidence"], [])
        for source in (
            "validation/pls_model_fit_full_refit_oracle.py",
            "validation/pls_model_fit_v2_qualification_evidence.py",
            "validation/pls_model_fit_v2_qualification_factory.py",
            "docs/methods/PLS_MODEL_FIT_FULL_REFIT_ORACLE_V1.md",
            "docs/methods/PLS_MODEL_FIT_QUALIFICATION_STATUS_V2.md",
        ):
            self.assertTrue((ROOT / source).is_file(), source)
        self.assertTrue(
            any(
                "validation-only" in item
                for item in model_fit["known_differences"]
            )
        )
        model_fit_visibility = resolve_customer_visibility(
            self.registry, "smartpls.model_fit", "qpls3.assessment.model_fit"
        )
        self.assertEqual(model_fit_visibility["channel"], "labs")
        self.assertTrue(model_fit_visibility["requires_opt_in"])
        self.assertFalse(model_fit_visibility["available"])

        exact_bootstrap = resolve_customer_visibility(
            self.registry, "smartpls.cbsem_bootstrapping", "qpls3.cbsem.bootstrap"
        )
        self.assertEqual(exact_bootstrap["channel"], "standard")
        self.assertTrue(exact_bootstrap["available"])

        excluded = resolve_customer_visibility(
            self.registry, "smartpls.gof", "qpls3.exclusion.gof"
        )
        self.assertEqual(excluded["channel"], "legacy")
        self.assertFalse(excluded["requires_opt_in"])
        self.assertFalse(excluded["available"])

        mismatched = copy.deepcopy(self.registry)
        mismatched["capabilities"][0]["option_cells"][0]["evidence_state"] = "absent"
        fallback = resolve_customer_visibility(
            mismatched, "smartpls.pls_algorithm", "qpls3.pls.algorithm"
        )
        self.assertEqual(fallback["channel"], "hidden")
        self.assertFalse(fallback["available"])

        coverage_only = copy.deepcopy(self.registry)
        coverage_only["capabilities"][0]["option_cells"][0]["coverage_state"] = "full"
        coverage_only["capabilities"][0]["option_cells"][0]["evidence_state"] = "absent"
        coverage_only["capabilities"][0]["option_cells"][0]["surface"] = "labs"
        still_labs = resolve_customer_visibility(
            coverage_only, "smartpls.pls_algorithm", "qpls3.pls.algorithm"
        )
        self.assertEqual(still_labs["channel"], "labs")

        promoted = copy.deepcopy(self.registry)
        promoted["capabilities"][0]["option_cells"][0]["coverage_state"] = "full"
        promoted["capabilities"][0]["option_cells"][0][
            "evidence_state"
        ] = "release_qualified"
        promoted["capabilities"][0]["option_cells"][0]["surface"] = "standard"
        standard = resolve_customer_visibility(
            promoted, "smartpls.pls_algorithm", "qpls3.pls.algorithm"
        )
        self.assertEqual(standard["channel"], "standard")
        self.assertFalse(standard["requires_opt_in"])
        self.assertTrue(standard["available"])

    def test_blindfolding_and_gof_are_the_only_explicit_exclusions(self):
        excluded = {
            row["capability_id"]: row
            for row in self.registry["capabilities"]
            if row["coverage_state"] == "intentionally_excluded"
        }
        self.assertEqual(
            set(excluded), {"smartpls.blindfolding", "smartpls.gof"}
        )
        for row in excluded.values():
            self.assertEqual(row["official_lifecycle"], "legacy")
            self.assertEqual(row["evidence_state"], "absent")
            self.assertEqual(row["surface"], "legacy")
            self.assertTrue(row["official_url"].startswith("https://smartpls.com/"))
            self.assertGreaterEqual(len(row["known_differences"]), 2)

    def test_all_rows_have_surface_predicate_and_reference_contracts(self):
        for row in self.registry["capabilities"]:
            with self.subTest(capability_id=row["capability_id"]):
                self.assertEqual(
                    row["product_areas"],
                    [
                        "diagram",
                        "data",
                        "settings",
                        "calculation",
                        "results",
                        "reporting",
                    ],
                )
                self.assertTrue(row["model_predicates"]["official"])
                self.assertTrue(row["model_predicates"]["quickpls"])
                self.assertTrue(row["data_predicates"]["official"])
                self.assertTrue(row["data_predicates"]["quickpls"])
                self.assertEqual(
                    row["supported_model_predicate"], row["model_predicates"]
                )
                self.assertEqual(
                    row["supported_data_predicate"], row["data_predicates"]
                )
                self.assertIn(row["official_url"], row["settings_references"])
                self.assertIn(row["official_url"], row["result_references"])
                self.assertEqual(
                    row["settings_schema"],
                    {"references": row["settings_references"]},
                )
                self.assertEqual(
                    row["result_schema"],
                    {"references": row["result_references"]},
                )
                self.assertEqual(row["documentation_reference"], row["official_url"])
                self.assertTrue(row["qualification_references"])
                self.assertEqual(
                    row["qualification_spec"],
                    {
                        "references": row["qualification_references"],
                        "links": row["qualification_links"],
                    },
                )
                self.assertTrue(row["known_differences"])
                self.assertTrue(row["option_cells"])
                self.assertEqual(
                    row["qualification_links"],
                    [cell["qualification_spec"]["links"][0] for cell in row["option_cells"]],
                )
                for cell in row["option_cells"]:
                    self.assertEqual(cell["capability_id"], row["capability_id"])
                    self.assertEqual(cell["documentation_reference"], row["official_url"])
                    self.assertEqual(len(cell["qualification_spec"]["links"]), 1)

    def test_registry_files_are_strict_json(self):
        for path in (DEFAULT_REGISTRY_PATH, DEFAULT_SCHEMA_PATH):
            with self.subTest(path=path):
                parsed = json.loads(path.read_text(encoding="utf-8"))
                self.assertIsInstance(parsed, dict)
                self.assertEqual(parsed, load_json(path))


if __name__ == "__main__":
    unittest.main()
