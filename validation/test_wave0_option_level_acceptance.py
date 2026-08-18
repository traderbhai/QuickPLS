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

from capability_registry_v2 import load_json  # noqa: E402
from wave0_option_level_acceptance import (  # noqa: E402
    DEFAULT_MATRIX_PATH,
    DEFAULT_REGISTRY_PATH,
    DEFAULT_REPORT_PATH,
    DEFAULT_SCHEMA_PATH,
    REQUIRED_DIMENSIONS,
    build_acceptance_report,
    check_saved_report,
    load_matrix_with_fragments,
)


class Wave0OptionLevelAcceptanceTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.matrix, cls.fragment_bindings = load_matrix_with_fragments(
            DEFAULT_MATRIX_PATH
        )
        cls.registry = load_json(DEFAULT_REGISTRY_PATH)
        cls.schema = load_json(DEFAULT_SCHEMA_PATH)

    def build(self, matrix=None, registry=None):
        return build_acceptance_report(
            self.matrix if matrix is None else matrix,
            self.registry if registry is None else registry,
            schema=self.schema,
            fragment_bindings=self.fragment_bindings,
        )

    def first_active_row(self):
        return next(
            row
            for row in self.registry["capabilities"]
            if row["official_lifecycle"] == "active"
        )

    @staticmethod
    def final_cell_override(link):
        return {
            "qualification_link": copy.deepcopy(link),
            "parity_obligation": {
                "capture_state": "captured",
                "parity_role": "active_parity",
            },
        }

    @staticmethod
    def attach_option_cells(row, *, coverage, evidence, surface):
        row["option_cells"] = [
            {
                "capability_id": link["capability_id"],
                "cell_id": link["cell_id"],
                "capability_version": link["capability_version"],
                "coverage_state": coverage,
                "evidence_state": evidence,
                "surface": surface,
                "qualification_spec": {
                    "links": [copy.deepcopy(link)],
                },
            }
            for link in row["qualification_links"]
        ]

    @staticmethod
    def captured_dimension(row, dimension, *, cell_id=None, reference=None):
        link = row["qualification_links"][0]
        return {
            "capability_id": row["capability_id"],
            "dimension": dimension,
            "capture_state": "captured",
            "acceptance_items": [
                {
                    "item_id": f"{row['capability_id']}.{dimension}.fixture",
                    "description": "Synthetic contract fixture for the validator test.",
                    "trace_cells": [
                        (
                            copy.deepcopy(link)
                            if cell_id is None
                            else {**copy.deepcopy(link), "cell_id": cell_id}
                        )
                    ],
                    "official_references": [reference or row["official_url"]],
                    "acceptance_criteria": [
                        "The captured contract has a deterministic testable outcome."
                    ],
                }
            ],
        }

    def test_open_baseline_contract_passes_but_finalization_gate_fails(self):
        report = self.build()
        self.assertTrue(report["contract_passed"], report["errors"])
        self.assertFalse(report["finalization_ready"])
        self.assertEqual(report["catalogue_counts"]["catalogue_rows"], 45)
        self.assertEqual(report["catalogue_counts"]["active_rows"], 43)
        self.assertEqual(report["catalogue_counts"]["explicit_exclusions"], 2)
        self.assertEqual(report["catalogue_counts"]["active_cell_identities"], 45)
        self.assertEqual(report["dimension_counts"]["expected_traces"], 387)
        self.assertEqual(report["dimension_counts"]["complete_traces"], 387)
        self.assertEqual(report["dimension_counts"]["captured"], 387)
        self.assertEqual(report["dimension_counts"]["open"], 0)
        self.assertEqual(
            report["cell_role_counts"],
            {"active_parity": 44, "beyond_parity": 1, "open": 0},
        )
        self.assertIn(
            "ACTIVE_PARITY_CELL_NOT_FINAL",
            {item["code"] for item in report["blockers"]},
        )

    def test_official_source_fragments_are_loaded_and_content_bound(self):
        self.assertEqual(len(self.fragment_bindings), 6)
        self.assertEqual(
            [binding["path"] for binding in self.fragment_bindings],
            [
                "validation/parity/fragments/pls_power_posthoc_v1.json",
                "validation/parity/fragments/pls_core_resampling_official_v1.json",
                "validation/parity/fragments/cbsem_cfa_official_v1.json",
                "validation/parity/fragments/pls_assessment_prediction_official_v1.json",
                "validation/parity/fragments/pls_advanced_groups_official_v1.json",
                "validation/parity/fragments/other_analytical_families_official_v1.json",
            ],
        )
        for binding in self.fragment_bindings:
            self.assertRegex(binding["sha256"], r"^[0-9a-f]{64}$")
        report = self.build()
        self.assertEqual(report["source_bindings"]["override_fragment_count"], 6)
        self.assertEqual(
            report["source_bindings"]["override_fragments"],
            self.fragment_bindings,
        )

        with tempfile.TemporaryDirectory() as directory:
            bad = copy.deepcopy(load_json(DEFAULT_MATRIX_PATH))
            bad["override_fragments"] = ["../outside.json"]
            path = Path(directory) / "matrix.json"
            path.write_text(json.dumps(bad), encoding="utf-8")
            with self.assertRaisesRegex(
                ValueError, "outside validation/parity/fragments"
            ):
                load_matrix_with_fragments(path)

    def test_json_schema_is_valid_and_accepts_the_baseline(self):
        if importlib.util.find_spec("jsonschema") is None:
            self.skipTest(
                "jsonschema is not installed; manual contract tests still run"
            )
        import jsonschema

        jsonschema.Draft202012Validator.check_schema(self.schema)
        errors = sorted(
            jsonschema.Draft202012Validator(self.schema).iter_errors(self.matrix),
            key=lambda error: tuple(str(part) for part in error.absolute_path),
        )
        self.assertEqual([], [error.message for error in errors])

    def test_all_active_rows_and_dimensions_have_generated_cell_and_official_traces(
        self,
    ):
        report = self.build()
        self.assertEqual(len(report["rows"]), 43)
        self.assertEqual(
            [row["catalogue_position"] for row in report["rows"]],
            [position for position in range(1, 46) if position not in {8, 15}],
        )
        for row in report["rows"]:
            with self.subTest(capability_id=row["capability_id"]):
                self.assertTrue(row["cells"])
                self.assertEqual(set(row["dimensions"]), set(REQUIRED_DIMENSIONS))
                self.assertTrue(row["trace_complete"])
                for dimension in row["dimensions"].values():
                    self.assertEqual(dimension["capture_state"], "captured")
                    self.assertTrue(dimension["trace_cells"])
                    self.assertTrue(
                        all(
                            set(trace)
                            == {
                                "registry_schema_version",
                                "capability_id",
                                "cell_id",
                                "capability_version",
                            }
                            for trace in dimension["trace_cells"]
                        )
                    )
                    self.assertTrue(dimension["official_references"])
                    self.assertTrue(
                        all(
                            reference.startswith("https://smartpls.com/")
                            for reference in dimension["official_references"]
                        )
                    )

    def test_blindfolding_and_gof_are_exact_explicit_exclusions(self):
        report = self.build()
        self.assertEqual(
            {item["capability_id"] for item in report["exclusions"]},
            {"smartpls.blindfolding", "smartpls.gof"},
        )
        for exclusion in report["exclusions"]:
            self.assertEqual(exclusion["decision"], "intentionally_excluded")
            self.assertTrue(exclusion["cells"])
            self.assertTrue(
                exclusion["official_reference"].startswith("https://smartpls.com/")
            )

        changed = copy.deepcopy(self.matrix)
        changed["exclusions"][1]["capability_id"] = "smartpls.pls_algorithm"
        failed = self.build(changed)
        self.assertFalse(failed["contract_passed"])
        self.assertTrue(
            any("exactly Blindfolding and GoF" in error for error in failed["errors"])
        )

    def test_cell_overrides_require_exact_active_registry_identity(self):
        row = self.first_active_row()
        changed = copy.deepcopy(self.matrix)
        override = self.final_cell_override(row["qualification_links"][0])
        override["qualification_link"]["capability_version"] = "unknown_v99"
        changed["overrides"]["cell_assessments"].append(override)
        report = self.build(changed)
        self.assertFalse(report["contract_passed"])
        self.assertTrue(
            any(
                "identity is not an active registry cell" in error
                for error in report["errors"]
            ),
            report["errors"],
        )

    def test_release_evidence_never_infers_full_coverage(self):
        row = self.first_active_row()
        changed = copy.deepcopy(self.matrix)
        changed["overrides"]["cell_assessments"] = [
            assessment
            for assessment in changed["overrides"]["cell_assessments"]
            if assessment["qualification_link"]["capability_id"]
            != row["capability_id"]
        ]
        changed["overrides"]["cell_assessments"].append(
            self.final_cell_override(row["qualification_links"][0])
        )
        registry = copy.deepcopy(self.registry)
        registry_row = next(
            item
            for item in registry["capabilities"]
            if item["capability_id"] == row["capability_id"]
        )
        self.attach_option_cells(
            registry_row,
            coverage="partial",
            evidence="release_qualified",
            surface="labs",
        )
        # Deliberately contradict the row projection. Authoritative cell state
        # must survive unchanged and row state must remain informational only.
        registry_row["coverage_state"] = "absent"
        registry_row["evidence_state"] = "absent"
        registry_row["surface"] = "internal"
        report = self.build(changed, registry)
        self.assertTrue(report["contract_passed"], report["errors"])
        report_row = next(
            item
            for item in report["rows"]
            if item["capability_id"] == row["capability_id"]
        )
        cell = report_row["cells"][0]
        self.assertEqual(cell["coverage"], "partial")
        self.assertEqual(cell["evidence"], "release_qualified")
        self.assertEqual(cell["surface"], "labs")
        self.assertFalse(report_row["finalization_ready"])
        self.assertTrue(
            report["coverage_evidence_independence"]["evidence_never_infers_coverage"]
        )
        self.assertEqual(
            report["coverage_evidence_independence"][
                "registry_release_qualified_without_full_count"
            ],
            0,
        )
        self.assertGreater(
            report["coverage_evidence_independence"][
                "option_cells_release_qualified_without_full_count"
            ],
            0,
        )
        self.assertIn(
            row["qualification_links"][0],
            report["coverage_evidence_independence"][
                "option_cells_release_qualified_without_full"
            ],
        )

    def test_captured_dimension_requires_row_cell_and_recorded_official_reference(self):
        row = self.first_active_row()
        changed = copy.deepcopy(self.matrix)
        changed["overrides"]["dimension_assessments"] = [
            assessment
            for assessment in changed["overrides"]["dimension_assessments"]
            if not (
                assessment["capability_id"] == row["capability_id"]
                and assessment["dimension"] == "settings"
            )
        ]
        open_report = self.build(changed)
        self.assertTrue(open_report["contract_passed"], open_report["errors"])
        self.assertEqual(open_report["dimension_counts"]["captured"], 386)
        self.assertEqual(open_report["dimension_counts"]["open"], 1)
        changed["overrides"]["dimension_assessments"].append(
            self.captured_dimension(row, "settings")
        )
        report = self.build(changed)
        self.assertTrue(report["contract_passed"], report["errors"])
        self.assertEqual(report["dimension_counts"]["captured"], 387)
        self.assertEqual(report["dimension_counts"]["open"], 0)

        bad_cell = copy.deepcopy(changed)
        bad_cell["overrides"]["dimension_assessments"][-1]["acceptance_items"][0][
            "trace_cells"
        ][0]["cell_id"] = "qpls3.not.a.row.cell"
        failed_cell = self.build(bad_cell)
        self.assertFalse(failed_cell["contract_passed"])
        self.assertTrue(any("non-row cell" in error for error in failed_cell["errors"]))

        bad_reference = copy.deepcopy(changed)
        bad_reference["overrides"]["dimension_assessments"][-1]["acceptance_items"][0][
            "official_references"
        ] = ["https://example.com/not-official"]
        failed_reference = self.build(bad_reference)
        self.assertFalse(failed_reference["contract_passed"])
        self.assertTrue(
            any(
                "not an official SmartPLS URL" in error
                for error in failed_reference["errors"]
            )
        )

    def test_multi_cell_rows_remain_independent_and_are_not_collapsed(self):
        report = self.build()
        limitations = {
            item["capability_id"]: item for item in report["multi_cell_row_limitations"]
        }
        self.assertEqual(
            set(limitations), {"smartpls.pls_power_analysis", "smartpls.permutation"}
        )
        power = next(
            row
            for row in report["rows"]
            if row["capability_id"] == "smartpls.pls_power_analysis"
        )
        self.assertEqual(
            [cell["qualification_link"]["cell_id"] for cell in power["cells"]],
            [
                "qpls3.pls.posthoc_technical_minimum_sample_size",
                "qpls3.pls.sample_size_power",
            ],
        )
        self.assertTrue(
            all(
                cell["state_sources"]["coverage"]
                in {"explicit_open_default", "registry_option_cell"}
                for cell in power["cells"]
            )
        )
        self.assertTrue(
            all(
                "registry_row" not in set(cell["state_sources"].values())
                for cell in power["cells"]
            )
        )
        self.assertIn(
            "mixed option-cell states",
            limitations[power["capability_id"]]["limitation"],
        )

    def test_duplicate_official_pca_rows_are_preserved(self):
        report = self.build()
        pca_rows = [
            row for row in report["rows"] if "PCA" in row["official_method"].upper()
        ]
        self.assertEqual([row["catalogue_position"] for row in pca_rows], [5, 45])
        self.assertNotEqual(pca_rows[0]["capability_id"], pca_rows[1]["capability_id"])

    def test_integrated_source_slices_retain_conservative_wave0_states(self):
        report = self.build()
        cells = {
            cell["qualification_link"]["cell_id"]: cell
            for row in report["rows"]
            for cell in row["cells"]
        }
        expected = {
            "qpls3.pls.posthoc_technical_minimum_sample_size": (
                "partial",
                "engine_only",
                "labs",
                "active_parity",
            ),
            "qpls3.pls.sample_size_power": (
                "absent",
                "absent",
                "labs",
                "beyond_parity",
            ),
            "qpls3.inference.consistent_bootstrap": (
                "partial",
                "absent",
                "labs",
                "active_parity",
            ),
            "qpls3.assessment.htmt": (
                "absent",
                "absent",
                "labs",
                "active_parity",
            ),
            "qpls3.assessment.model_fit": (
                "partial",
                "absent",
                "labs",
                "active_parity",
            ),
            "qpls3.cbsem.ml": (
                "partial",
                "absent",
                "labs",
                "active_parity",
            ),
        }

        for cell_id, state in expected.items():
            with self.subTest(cell_id=cell_id):
                cell = cells[cell_id]
                self.assertEqual(
                    (
                        cell["coverage"],
                        cell["evidence"],
                        cell["surface"],
                        cell["parity_obligation"],
                    ),
                    state,
                )
                self.assertEqual(
                    set(cell["state_sources"].values())
                    - {"matrix_cell_assessment"},
                    {"registry_option_cell"},
                )

        self.assertEqual(report["catalogue_counts"]["active_rows"], 43)
        self.assertEqual(report["catalogue_counts"]["explicit_exclusions"], 2)
        self.assertFalse(report["finalization_ready"])

    def test_finalization_gate_can_only_open_after_all_cells_and_dimensions_are_explicit(
        self,
    ):
        changed = copy.deepcopy(self.matrix)
        changed["overrides"] = {"cell_assessments": [], "dimension_assessments": []}
        registry = copy.deepcopy(self.registry)
        for row in registry["capabilities"]:
            if row["official_lifecycle"] != "active":
                continue
            self.attach_option_cells(
                row,
                coverage="full",
                evidence="release_qualified",
                surface="standard",
            )
            for link in row["qualification_links"]:
                changed["overrides"]["cell_assessments"].append(
                    self.final_cell_override(link)
                )
            for dimension in REQUIRED_DIMENSIONS:
                changed["overrides"]["dimension_assessments"].append(
                    self.captured_dimension(row, dimension)
                )
        report = self.build(changed, registry)
        self.assertTrue(report["contract_passed"], report["errors"])
        self.assertTrue(report["finalization_ready"], report["blockers"])
        self.assertEqual(report["dimension_counts"]["open"], 0)
        self.assertEqual(report["cell_role_counts"]["active_parity"], 45)

    def test_saved_report_is_the_exact_deterministic_projection(self):
        if not DEFAULT_REPORT_PATH.exists():
            self.fail(f"saved report is missing: {DEFAULT_REPORT_PATH}")
        expected = self.build()
        actual = load_json(DEFAULT_REPORT_PATH)
        check = check_saved_report(expected, actual)
        self.assertTrue(check["passed"], check["errors"])
        self.assertEqual(check["expected_sha256"], check["actual_sha256"])


if __name__ == "__main__":
    unittest.main()
