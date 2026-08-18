from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from capability_registry_v2 import (  # noqa: E402
    canonical_sha256,
    load_json,
    qualification_link_identity,
)
from wave0_option_level_acceptance import (  # noqa: E402
    DEFAULT_MATRIX_PATH,
    DEFAULT_REGISTRY_PATH,
    DEFAULT_SCHEMA_PATH,
    REQUIRED_DIMENSIONS,
    build_acceptance_report,
    load_matrix_with_fragments,
)


FRAGMENT_RELATIVE_PATH = (
    "validation/parity/fragments/pls_assessment_prediction_official_v1.json"
)
FRAGMENT_PATH = ROOT / FRAGMENT_RELATIVE_PATH
EXPECTED_CAPABILITY_IDS = (
    "smartpls.cvpat",
    "smartpls.cca",
    "smartpls.cta_pls",
    "smartpls.htmt",
    "smartpls.model_fit",
    "smartpls.pls_model_comparison",
    "smartpls.prediction_oriented_model_selection",
    "smartpls.plspredict",
    "smartpls.ipma",
    "smartpls.nca",
)
FRAGMENT_KEYS = {
    "fragment_schema_version",
    "fragment_id",
    "frozen_on",
    "capability_ids",
    "cell_assessments",
    "dimension_assessments",
}
CELL_OVERRIDE_KEYS = {"qualification_link", "parity_obligation"}
ITEM_KEYS = {
    "item_id",
    "description",
    "trace_cells",
    "official_references",
    "acceptance_criteria",
}


class Wave0PlsAssessmentPredictionFragmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fragment = load_json(FRAGMENT_PATH)
        cls.matrix = load_json(DEFAULT_MATRIX_PATH)
        cls.registry = load_json(DEFAULT_REGISTRY_PATH)
        cls.schema = load_json(DEFAULT_SCHEMA_PATH)
        cls.registry_rows = {
            row["capability_id"]: row for row in cls.registry["capabilities"]
        }

    def test_fragment_has_frozen_wire_shape_and_expected_disjoint_rows(self) -> None:
        self.assertEqual(set(self.fragment), FRAGMENT_KEYS)
        self.assertEqual(self.fragment["fragment_schema_version"], 1)
        self.assertEqual(
            self.fragment["fragment_id"],
            "quickpls.wave0.pls_assessment_prediction_official.v1",
        )
        self.assertEqual(self.fragment["frozen_on"], "2026-08-14")
        self.assertEqual(
            tuple(self.fragment["capability_ids"]), EXPECTED_CAPABILITY_IDS
        )

        existing_fragment_capabilities: set[str] = set()
        for relative_path in self.matrix["override_fragments"]:
            if relative_path == FRAGMENT_RELATIVE_PATH:
                continue
            existing = load_json(ROOT / relative_path)
            existing_fragment_capabilities.update(existing["capability_ids"])
        self.assertTrue(
            set(EXPECTED_CAPABILITY_IDS).isdisjoint(existing_fragment_capabilities)
        )

    def test_every_registry_cell_is_exact_active_parity_without_state_inference(
        self,
    ) -> None:
        expected = set()
        for capability_id in EXPECTED_CAPABILITY_IDS:
            row = self.registry_rows[capability_id]
            self.assertEqual(row["official_lifecycle"], "active")
            expected.update(
                qualification_link_identity(cell["qualification_spec"]["links"][0])
                for cell in row["option_cells"]
            )

        actual = set()
        for assessment in self.fragment["cell_assessments"]:
            self.assertEqual(set(assessment), CELL_OVERRIDE_KEYS)
            self.assertEqual(
                assessment["parity_obligation"],
                {"capture_state": "captured", "parity_role": "active_parity"},
            )
            self.assertTrue({"coverage", "evidence", "surface"}.isdisjoint(assessment))
            actual.add(qualification_link_identity(assessment["qualification_link"]))

        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), 10)

    def test_all_nine_dimensions_have_official_traceable_acceptance(self) -> None:
        assessments = self.fragment["dimension_assessments"]
        self.assertEqual(len(assessments), 10 * len(REQUIRED_DIMENSIONS))
        self.assertEqual(
            Counter(value["capability_id"] for value in assessments),
            Counter({value: 9 for value in EXPECTED_CAPABILITY_IDS}),
        )

        by_capability: dict[str, set[str]] = defaultdict(set)
        fact_kinds: dict[str, set[str]] = defaultdict(set)
        seen_item_ids: set[str] = set()
        for assessment in assessments:
            capability_id = assessment["capability_id"]
            row = self.registry_rows[capability_id]
            by_capability[capability_id].add(assessment["dimension"])
            self.assertEqual(assessment["capture_state"], "captured")
            self.assertTrue(assessment["acceptance_items"])
            expected_cells = {
                qualification_link_identity(cell["qualification_spec"]["links"][0])
                for cell in row["option_cells"]
            }

            for item in assessment["acceptance_items"]:
                self.assertEqual(set(item), ITEM_KEYS)
                self.assertNotIn(item["item_id"], seen_item_ids)
                seen_item_ids.add(item["item_id"])
                self.assertEqual(
                    set(map(qualification_link_identity, item["trace_cells"])),
                    expected_cells,
                )
                self.assertEqual(item["official_references"], [row["official_url"]])
                self.assertTrue(item["acceptance_criteria"])
                self.assertTrue(
                    all(value.strip() for value in item["acceptance_criteria"])
                )

                if ".documented." in item["item_id"]:
                    self.assertTrue(item["description"].startswith("Documented fact:"))
                    fact_kinds[capability_id].add("documented")
                elif ".open_live_app." in item["item_id"]:
                    self.assertTrue(
                        item["description"].startswith("Open live-app detail:")
                    )
                    self.assertTrue(
                        any(
                            term in " ".join(item["acceptance_criteria"]).lower()
                            for term in ("fixture", "inventory", "ledger", "matrix")
                        )
                    )
                    fact_kinds[capability_id].add("open_live_app")
                else:
                    self.fail(
                        f"{item['item_id']} does not classify its evidence boundary"
                    )

        for capability_id in EXPECTED_CAPABILITY_IDS:
            self.assertEqual(by_capability[capability_id], set(REQUIRED_DIMENSIONS))
            self.assertEqual(fact_kinds[capability_id], {"documented", "open_live_app"})

    def test_loader_accepts_fragment_and_contract_remains_nonfinal(self) -> None:
        isolated = copy.deepcopy(self.matrix)
        isolated["override_fragments"] = [FRAGMENT_RELATIVE_PATH]
        isolated["overrides"] = {
            "cell_assessments": [],
            "dimension_assessments": [],
        }
        with tempfile.TemporaryDirectory() as directory:
            matrix_path = Path(directory) / "matrix.json"
            matrix_path.write_text(
                json.dumps(isolated, sort_keys=True), encoding="utf-8"
            )
            effective, bindings = load_matrix_with_fragments(matrix_path)

        self.assertEqual(
            bindings,
            [
                {
                    "path": FRAGMENT_RELATIVE_PATH,
                    "fragment_id": self.fragment["fragment_id"],
                    "sha256": canonical_sha256(self.fragment),
                }
            ],
        )
        report = build_acceptance_report(
            effective,
            self.registry,
            schema=self.schema,
        )
        self.assertTrue(report["contract_passed"], report["errors"])
        self.assertEqual(report["dimension_counts"]["captured"], 90)
        self.assertEqual(report["dimension_counts"]["open"], 297)
        self.assertEqual(report["cell_role_counts"]["active_parity"], 10)
        self.assertEqual(report["cell_role_counts"]["open"], 35)
        self.assertFalse(report["finalization_ready"])


if __name__ == "__main__":
    unittest.main()
