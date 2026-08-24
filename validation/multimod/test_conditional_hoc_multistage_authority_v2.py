#!/usr/bin/env python3
"""No-build contracts for the bounded conditional multiple-HOC adapter."""

from __future__ import annotations

import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
RUNNER = ROOT / "crates" / "qpls-runner" / "src" / "multimod_conditional_raw_v2.rs"
QUALIFICATION = (
    ROOT
    / "crates"
    / "qpls-runner"
    / "examples"
    / "multimod_conditional_qualification_v1.rs"
)


def section(source: str, start: str, end: str) -> str:
    start_index = source.index(start)
    return source[start_index : source.index(end, start_index)]


class ConditionalHocMultistageAuthorityContracts(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.runner = RUNNER.read_text(encoding="utf-8")
        cls.qualification = QUALIFICATION.read_text(encoding="utf-8")

    def test_base_stage_uses_the_bounded_multimod_projection(self) -> None:
        helper = section(
            self.runner,
            "fn project_hoc_conditional_base_model_v2(",
            "fn built_in_hoc_conditional_authority_v2(",
        )
        self.assertIn(
            "compile_pls_higher_order_lower_order_projection_multimod_v2(scientific_model)",
            helper,
        )
        self.assertNotIn("compile_pls_higher_order_lower_order_projection_v1", helper)

        authority = section(
            self.runner,
            "fn built_in_hoc_conditional_authority_v2(",
            "fn hoc_conditional_alias_specs_v2(",
        )
        self.assertIn(
            "let base_model = project_hoc_conditional_base_model_v2(&scientific_model)?;",
            authority,
        )
        self.assertIn("project_general_sem_pls_base_recipe_v1(recipe)", authority)
        self.assertNotIn("project_general_sem_pls_stage_one_recipe_v1", authority)
        self.assertIn("if !(1..=4).contains(&hocs.len())", authority)

    def test_dependency_stages_finish_before_interaction_products(self) -> None:
        refitter = section(
            self.runner,
            "impl ConditionalProcessFullRefitterV2 for BuiltInHocConditionalRefitterV2<'_>",
            "impl ConditionalProcessFullRefitterV2 for BuiltInWeightedConditionalRefitterV2<'_>",
        )
        self.assertLess(
            refitter.index("let fit = self.fit_sample_v2("),
            refitter.index("estimate_multimod_conditional_interactions_v2_with_control("),
        )
        self.assertIn("hoc_dependency_stages_refit: true", refitter)
        self.assertIn("interaction_products_rebuilt: true", refitter)

    def test_full_four_hoc_two_interaction_qualification_fixture_is_unchanged(self) -> None:
        fixture = section(
            self.qualification,
            "fn run_hoc_case(scale: Scale)",
            "fn grouped_or_weighted_base(",
        )
        self.assertIn("for index in 1..=4", fixture)
        self.assertIn("for index in 1..=2", fixture)
        self.assertIn(
            "profile: ConditionalProcessProfileV2::MultipleHocPercentile",
            fixture,
        )
        self.assertIn(
            'hoc_ids: (1..=4).map(|index| format!("term:hoc{index}")).collect()',
            fixture,
        )
        self.assertIn('"multiple_hoc:four_disjoint"', fixture)

    def test_rust_regression_fixture_exercises_four_hocs(self) -> None:
        regression = section(
            self.runner,
            "fn four_disjoint_hoc_projection_fixture()",
            "fn row_mask_binds_membership_not_runtime_weight_representation()",
        )
        self.assertIn(
            "fn conditional_hoc_base_projection_keeps_the_four_hoc_envelope()",
            regression,
        )
        self.assertIn("project_hoc_conditional_base_model_v2(&scientific_model)", regression)
        self.assertIn("assert!(base_model.derived_terms.is_empty())", regression)


if __name__ == "__main__":
    unittest.main()
