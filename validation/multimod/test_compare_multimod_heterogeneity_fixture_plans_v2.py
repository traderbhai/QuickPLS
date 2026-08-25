#!/usr/bin/env python3
"""Semantic contract tests for heterogeneity qualification fixture plans."""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
REPOSITORY = HERE.parent.parent
sys.path.insert(0, str(HERE))

import compare_multimod_heterogeneity_qualification_v2 as comparator


EXPECTED_BOOTSTRAP_FIXTURE_PLAN = {
    "schema_version": 1,
    "plan_id": "qpls.multimod.heterogeneity.k2-fixed-bootstrap-n80-dual-outcome.v1",
    "purpose": "fixed_k_full_pipeline_bootstrap_inference_and_ledger_qualification",
    "selected_k": 2,
    "observations_per_fixture": 80,
    "expected_cases_per_true_class": 40,
    "interaction_fixture_design": "dual_endogenous_anchor_v1",
    "requested_replicates": 500,
    "performance_scope": "n80_fixed_k_bootstrap_not_a_500_draw_n400_runtime_claim",
}


class HeterogeneityFixturePlanTests(unittest.TestCase):
    def test_point_recovery_and_bootstrap_fixture_identities_are_separate(self) -> None:
        report = {
            "scale": "qualification",
            "metamorphism": "baseline",
            "sign_columns": None,
            "workers": 1,
            "fixture_observations": 400,
            "multiclass_point_fixture_plan": comparator.MULTICLASS_POINT_FIXTURE_PLAN,
            "bootstrap_fixture_plan": EXPECTED_BOOTSTRAP_FIXTURE_PLAN,
            "campaign_seed": 42,
            "seed": 42,
        }
        self.assertTrue(comparator.is_baseline_qualification_matrix(report))
        self.assertTrue(comparator.is_bootstrap_fixture_plan(report))

        point_matrix_changed = dict(report, fixture_observations=80)
        self.assertFalse(
            comparator.is_baseline_qualification_matrix(point_matrix_changed)
        )
        self.assertTrue(comparator.is_bootstrap_fixture_plan(point_matrix_changed))

        bootstrap_plan_changed = dict(report)
        bootstrap_plan_changed["bootstrap_fixture_plan"] = dict(
            EXPECTED_BOOTSTRAP_FIXTURE_PLAN,
            observations_per_fixture=400,
        )
        self.assertTrue(
            comparator.is_baseline_qualification_matrix(bootstrap_plan_changed)
        )
        self.assertFalse(comparator.is_bootstrap_fixture_plan(bootstrap_plan_changed))

    def test_all_seven_bootstrap_cells_are_bound_to_the_typed_fixture(self) -> None:
        cells = []
        for cell_id in sorted(comparator.EXPECTED_BOOTSTRAP_CELL_IDS):
            if cell_id in {
                "fimix-p0-fixed-k-bootstrap",
                "pos-published-p0-fixed-k-bootstrap",
            }:
                profile = "p0_structural"
                variables = [{"id": "construct:x"}, {"id": "construct:y"}]
                relations = [
                    {
                        "kind": "structural",
                        "source": "construct:x",
                        "target": "construct:y",
                    }
                ]
            else:
                profile = (
                    "p23_all_current"
                    if "p23" in cell_id
                    else "p2_multi_two_way"
                )
                variables = [
                    {"id": f"construct:{construct}"}
                    for construct in ("x", "z", "w", "y", "v")
                ]
                relations = [
                    {
                        "kind": "structural",
                        "source": f"construct:{source}",
                        "target": "construct:v",
                    }
                    for source in ("x", "z", "w")
                ]
                relations.extend(
                    [
                        {
                            "kind": "structural",
                            "source": "construct:x",
                            "target": "construct:y",
                        },
                        {
                            "kind": "structural",
                            "source": "derived:x_by_z",
                            "target": "construct:y",
                        },
                    ]
                )
            cells.append(
                {
                    "cell_id": cell_id,
                    "profile": profile,
                    "dataset_rows": 80,
                    "true_classes": [0] * 40 + [1] * 40,
                    "config": {"bootstrap": {"resamples": 500}},
                    "analysis": {"locked_k": 2},
                    "sem_model_authority": {
                        "variables": variables,
                        "relations": relations,
                    },
                    "evidence": {
                        "raw_preparation": [
                            {"fimix_input": {"metric": {"observation_count": 80}}}
                        ]
                    },
                }
            )
        report = {
            "bootstrap_fixture_plan": EXPECTED_BOOTSTRAP_FIXTURE_PLAN,
        }
        valid, receipt = comparator.bootstrap_fixture_matrix_receipt(report, cells)
        self.assertTrue(valid)
        self.assertEqual(len(receipt), 7)
        design_valid, design_receipt = comparator.bootstrap_outcome_design_receipt(
            cells
        )
        self.assertTrue(design_valid)
        self.assertEqual(len(design_receipt), 7)

        cells[0]["dataset_rows"] = 400
        valid, _ = comparator.bootstrap_fixture_matrix_receipt(report, cells)
        self.assertFalse(valid)

        interaction_cell = next(
            cell for cell in cells if cell["profile"] == "p2_multi_two_way"
        )
        interaction_cell["sem_model_authority"]["relations"].append(
            {
                "kind": "structural",
                "source": "derived:x_by_z",
                "target": "construct:v",
            }
        )
        design_valid, _ = comparator.bootstrap_outcome_design_receipt(cells)
        self.assertFalse(design_valid)

    def test_catalog_freezes_n80_dual_outcome_plan_and_worker_topology(self) -> None:
        catalog = json.loads(
            (
                HERE / "multimod_science_fixture_catalog_v1.json"
            ).read_text(encoding="utf-8")
        )["heterogeneity"]
        self.assertEqual(catalog["observations_per_fixture"], 400)
        self.assertEqual(
            catalog["observations_per_fixture_scope"],
            "point_and_recovery_matrix_only",
        )
        self.assertEqual(
            catalog["bootstrap_fixture_plan"], EXPECTED_BOOTSTRAP_FIXTURE_PLAN
        )
        self.assertEqual(
            catalog["bootstrap_outcome_design"]["ordinary_auxiliary_paths"],
            ["x->v", "z->v", "w->v"],
        )
        self.assertEqual(
            catalog["bootstrap_outcome_design"]["interaction_terms_target"], "y"
        )
        self.assertEqual(catalog["default_max_parallel_bootstrap_shards"], 4)
        self.assertEqual(
            catalog["pos_bootstrap_worker_topology"],
            "bounded_four_producer_processes_with_three_threads_each_on_twelve_logical_cores",
        )

    def test_method_spec_states_the_non_n400_performance_scope(self) -> None:
        method_spec = (
            REPOSITORY / "docs" / "methods" / "PLS_HETEROGENEITY_V2.md"
        ).read_text(encoding="utf-8")
        self.assertIn(EXPECTED_BOOTSTRAP_FIXTURE_PLAN["plan_id"], method_spec)
        self.assertIn(EXPECTED_BOOTSTRAP_FIXTURE_PLAN["purpose"], method_spec)
        self.assertIn(
            EXPECTED_BOOTSTRAP_FIXTURE_PLAN["performance_scope"], method_spec
        )
        self.assertIn("400-row seed-42 point/recovery identity", method_spec)
        self.assertIn("four producer processes with three threads each", method_spec)
        self.assertIn("P0 bootstrap fixtures remain single-outcome", method_spec)
        self.assertIn("`x -> v`, `z -> v`, and `w -> v`", method_spec)
        self.assertIn("interaction remains\non `y` only", method_spec)


if __name__ == "__main__":
    unittest.main()
