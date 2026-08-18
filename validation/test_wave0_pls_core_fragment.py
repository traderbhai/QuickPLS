from __future__ import annotations

import copy
import sys
import unittest
from collections import Counter, defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = Path(__file__).resolve().parent
if str(VALIDATION) not in sys.path:
    sys.path.insert(0, str(VALIDATION))

from capability_registry_v2 import (  # noqa: E402
    load_json,
    qualification_link_identity,
)
from wave0_option_level_acceptance import (  # noqa: E402
    DEFAULT_MATRIX_PATH,
    DEFAULT_REGISTRY_PATH,
    DEFAULT_SCHEMA_PATH,
    REQUIRED_DIMENSIONS,
    build_acceptance_report,
)


FRAGMENT_PATH = (
    ROOT / "validation/parity/fragments/pls_core_resampling_official_v1.json"
)
EXPECTED_CAPABILITY_IDS = (
    "smartpls.pls_algorithm",
    "smartpls.wpls",
    "smartpls.plsc",
    "smartpls.pca_core",
    "smartpls.pls_bootstrapping",
    "smartpls.consistent_bootstrapping",
    "smartpls.permutation",
    "smartpls.consistent_permutation",
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


class Wave0PlsCoreFragmentTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.fragment = load_json(FRAGMENT_PATH)
        cls.matrix = load_json(DEFAULT_MATRIX_PATH)
        cls.registry = load_json(DEFAULT_REGISTRY_PATH)
        cls.schema = load_json(DEFAULT_SCHEMA_PATH)
        cls.registry_rows = {
            row["capability_id"]: row for row in cls.registry["capabilities"]
        }

    def test_fragment_has_frozen_wire_shape_and_expected_rows(self) -> None:
        self.assertEqual(set(self.fragment), FRAGMENT_KEYS)
        self.assertEqual(self.fragment["fragment_schema_version"], 1)
        self.assertEqual(
            self.fragment["fragment_id"],
            "quickpls.wave0.pls_core_resampling_official.v1",
        )
        self.assertEqual(self.fragment["frozen_on"], "2026-08-14")
        self.assertEqual(
            tuple(self.fragment["capability_ids"]), EXPECTED_CAPABILITY_IDS
        )
        self.assertEqual(len(set(self.fragment["capability_ids"])), 8)

    def test_every_registry_cell_is_exact_and_active_parity(self) -> None:
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
            self.assertTrue(
                {"coverage", "evidence", "surface"}.isdisjoint(assessment),
                "The fragment must not infer QuickPLS state from official behavior.",
            )
            actual.add(qualification_link_identity(assessment["qualification_link"]))

        self.assertEqual(actual, expected)
        self.assertEqual(len(actual), 9)

    def test_all_nine_dimensions_are_captured_as_testable_obligations(self) -> None:
        assessments = self.fragment["dimension_assessments"]
        self.assertEqual(len(assessments), 8 * len(REQUIRED_DIMENSIONS))
        counts = Counter(value["capability_id"] for value in assessments)
        self.assertEqual(
            counts, Counter({value: 9 for value in EXPECTED_CAPABILITY_IDS})
        )

        by_capability: dict[str, set[str]] = defaultdict(set)
        seen_item_ids: set[str] = set()
        fact_kinds: dict[str, set[str]] = defaultdict(set)
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
                    all(
                        isinstance(criterion, str) and criterion.strip()
                        for criterion in item["acceptance_criteria"]
                    )
                )

                if ".documented." in item["item_id"]:
                    self.assertTrue(item["description"].startswith("Documented fact:"))
                    fact_kinds[capability_id].add("documented")
                elif ".open_live_app." in item["item_id"]:
                    self.assertTrue(
                        item["description"].startswith("Open live-app detail:")
                    )
                    fact_kinds[capability_id].add("open_live_app")
                    self.assertTrue(
                        any(
                            "capture" in criterion.lower()
                            or "ledger" in criterion.lower()
                            or "matrix" in criterion.lower()
                            or "fixture" in criterion.lower()
                            for criterion in item["acceptance_criteria"]
                        ),
                        "An open detail must name the evidence artifact that closes it.",
                    )
                else:
                    self.fail(
                        f"{item['item_id']} does not identify documented versus open evidence"
                    )

        for capability_id in EXPECTED_CAPABILITY_IDS:
            self.assertEqual(by_capability[capability_id], set(REQUIRED_DIMENSIONS))
            self.assertEqual(fact_kinds[capability_id], {"documented", "open_live_app"})

    def test_fragment_is_accepted_by_wave0_contract_without_promoting_state(
        self,
    ) -> None:
        effective = copy.deepcopy(self.matrix)
        # Test this fragment in isolation so it remains deterministic even when
        # the main matrix loads other disjoint official-source fragments.
        effective["override_fragments"] = []
        effective["overrides"] = {
            "cell_assessments": copy.deepcopy(self.fragment["cell_assessments"]),
            "dimension_assessments": copy.deepcopy(
                self.fragment["dimension_assessments"]
            ),
        }
        report = build_acceptance_report(
            effective,
            self.registry,
            schema=self.schema,
        )
        self.assertTrue(report["contract_passed"], report["errors"])
        self.assertEqual(report["dimension_counts"]["captured"], 72)
        self.assertEqual(report["dimension_counts"]["open"], 315)
        self.assertEqual(report["cell_role_counts"]["active_parity"], 9)
        self.assertEqual(report["cell_role_counts"]["open"], 36)
        self.assertFalse(report["finalization_ready"])


if __name__ == "__main__":
    unittest.main()
