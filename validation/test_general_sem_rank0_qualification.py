#!/usr/bin/env python3
"""Contract and scenario-factory tests for General SEM Rank 0 Lane A."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path


VALIDATION = Path(__file__).resolve().parent
ROOT = VALIDATION.parent
sys.path.insert(0, str(VALIDATION))

import general_sem_rank0_qualification as qualification  # noqa: E402
from qualification_spec_v2 import strict_load_json, validate_spec_document  # noqa: E402


class GeneralSemRank0QualificationTests(unittest.TestCase):
    def test_four_exact_cell_contracts_are_static_and_semantically_valid(self) -> None:
        self.assertEqual(
            set(qualification.CELL_CONTRACTS),
            {
                "mediation_point",
                "mediation_bootstrap",
                "moderation_point",
                "moderation_bootstrap",
            },
        )
        for key, path in qualification.SPEC_PATHS.items():
            with self.subTest(cell=key):
                static = strict_load_json(path)
                generated = qualification.build_qualification_spec(key)
                self.assertEqual(static, generated)
                report = validate_spec_document(static)
                self.assertTrue(report["passed"], report["errors"])
                self.assertTrue(report["schema_valid"])
                self.assertTrue(report["semantic_valid"])
                self.assertFalse(report["qualification_ready"])
                self.assertEqual(static["evidence_contract"]["receipts"], [])

    def test_contract_identities_match_the_exact_rank0_cells(self) -> None:
        expected = {
            "mediation_point": (
                "qpls3.pls.mediation",
                "pls_mediation_v1",
                "pls_mediation_v1",
            ),
            "mediation_bootstrap": (
                "qpls3.pls.general_sem_multiple_mediation_bootstrap",
                "general_sem_pls_full_model_case_bootstrap_v1",
                "general_sem_pls_full_model_case_bootstrap_v1",
            ),
            "moderation_point": (
                "qpls3.pls.general_sem_multiple_two_way_moderation_point",
                "general_sem_pls_multiple_two_way_moderation_point_v1",
                "qpls.general-sem-pls.multiple-two-way.point.v1",
            ),
            "moderation_bootstrap": (
                "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
                "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
                "qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1",
            ),
        }
        for key, identity in expected.items():
            spec = qualification.build_qualification_spec(key)
            cell = spec["identity"]["capability_cell"]
            self.assertEqual(
                (cell["cell_id"], cell["capability_version"]), identity[:2]
            )
            self.assertEqual(spec["identity"]["method_version"], identity[1])
            self.assertEqual(spec["identity"]["analytical_method_version"], identity[2])

    def test_thresholds_were_frozen_before_receipts(self) -> None:
        thresholds = qualification.FROZEN_THRESHOLDS
        self.assertEqual(thresholds["monte_carlo_confidence_level"], 0.95)
        self.assertEqual(thresholds["monte_carlo_maximum_half_width"], 0.01)
        self.assertEqual(thresholds["minimum_worst_case_binomial_trials"], 9_604)
        self.assertEqual(thresholds["recovery_acceptance_interval"], (0.90, 1.00))
        self.assertEqual(thresholds["mediation_recovery_absolute_bias_maximum"], 0.05)
        self.assertEqual(thresholds["mediation_recovery_rmse_maximum"], 0.15)
        self.assertEqual(thresholds["moderation_recovery_absolute_bias_maximum"], 0.08)
        self.assertEqual(thresholds["moderation_recovery_rmse_maximum"], 0.20)
        self.assertEqual(thresholds["coverage_acceptance_interval"], (0.90, 0.99))
        self.assertEqual(thresholds["null_rejection_acceptance_interval"], (0.00, 0.08))
        for key in qualification.CELL_CONTRACTS:
            spec = qualification.build_qualification_spec(key)
            self.assertEqual(
                spec["identity"]["spec_frozen_at_utc"], qualification.SPEC_FROZEN_AT_UTC
            )
            self.assertEqual(spec["migration"]["status"], "compatibility_only")
            self.assertTrue(
                any(
                    "not_minted_until_sources_stable" in item
                    for item in spec["migration"]["unresolved_items"]
                )
            )
            comparisons = {
                row["output_id"]: row for row in spec["comparison_contract"]["outputs"]
            }
            self.assertEqual(
                comparisons["effect_recovery_absolute_bias"]["rule"],
                "bounded_moment",
            )
            self.assertEqual(
                comparisons["effect_recovery_rmse"]["grouping_keys"],
                ["family", "target_id"],
            )
            policy = spec["scenario_contract"]["monte_carlo_policy"][
                "decision_boundary_trial_policy"
            ]
            self.assertEqual(
                policy["policy_version"],
                qualification.PLAN4B_DECISION_POLICY_VERSION,
            )
            self.assertEqual(
                policy["metric_budgets"]["null_rejection_rate"],
                {
                    "decision_rate": 0.08,
                    "minimum_trials": 2_835,
                    "execution_target_trials": 2_880,
                },
            )
            self.assertEqual(
                policy["scenario_trial_overrides"],
                {
                    "coverage.mediation_bootstrap": 9_604,
                    "coverage.moderation_bootstrap": 4_480,
                },
            )

    def test_every_contract_has_full_scenario_and_operational_obligations(self) -> None:
        required_axes = {
            "model_topology",
            "measurement_model",
            "data_distribution",
            "missingness",
            "input_type",
            "workload",
            "metamorphism",
        }
        required_profiles = {
            "micro_exact",
            "applied",
            "large",
            "maximum_axis",
            "compound_stress",
        }
        for key in qualification.CELL_CONTRACTS:
            with self.subTest(cell=key):
                spec = qualification.build_qualification_spec(key)
                scenario = spec["scenario_contract"]
                expected_axes = set(required_axes)
                if qualification.CELL_CONTRACTS[key].stochastic:
                    expected_axes.add("workers")
                self.assertEqual({row["id"] for row in scenario["axes"]}, expected_axes)
                worker_axes = {
                    row["id"]: [value["id"] for value in row["values"]]
                    for row in scenario["axes"]
                }
                if qualification.CELL_CONTRACTS[key].stochastic:
                    self.assertEqual(
                        worker_axes["workers"],
                        [
                            "one_worker",
                            "two_workers",
                            "four_workers",
                            "maximum_available_workers",
                        ],
                    )
                else:
                    self.assertNotIn("workers", worker_axes)
                self.assertEqual(
                    {row["id"] for row in scenario["complexity_profiles"]},
                    required_profiles,
                )
                self.assertTrue(
                    scenario["monte_carlo_policy"]["failed_fits_in_denominator"]
                )
                export = spec["operational_contract"]["export"]
                self.assertEqual(
                    set(export["formats"]), {"csv", "xlsx", "html", "svg", "pdf", "png"}
                )
                self.assertEqual(
                    set(export["semantic_readback_formats"]),
                    {"csv", "xlsx", "html", "svg", "pdf", "png"},
                )
                windows = spec["operational_contract"]["windows"]
                self.assertEqual(
                    set(windows["package_kinds"]), {"installed", "portable"}
                )
                self.assertEqual(
                    set(windows["display_scale_percent"]), {100, 125, 150, 200}
                )

    def test_performance_receipts_bind_the_authoritative_hardware_profile(self) -> None:
        manifest = strict_load_json(qualification.PERFORMANCE_PROFILE_MANIFEST)
        manifest_ids = {
            row["hardware_profile_id"] for row in manifest["hardware_profiles"]
        }
        self.assertIn(qualification.PERFORMANCE_HARDWARE_PROFILE_ID, manifest_ids)
        self.assertEqual(
            qualification.PERFORMANCE_HARDWARE_PROFILE_ID,
            "standard_windows_6c16g",
        )
        for key in qualification.CELL_CONTRACTS:
            with self.subTest(cell=key):
                spec = qualification.build_qualification_spec(key)
                performance = spec["operational_contract"]["performance"]
                self.assertEqual(
                    {row["id"] for row in performance["hardware_classes"]},
                    {qualification.PERFORMANCE_HARDWARE_PROFILE_ID},
                )
                self.assertEqual(
                    {row["hardware_class_id"] for row in performance["budgets"]},
                    {qualification.PERFORMANCE_HARDWARE_PROFILE_ID},
                )

    def test_oracle_contract_binds_the_supplied_second_indicator_level_source(
        self,
    ) -> None:
        for key in qualification.CELL_CONTRACTS:
            spec = qualification.build_qualification_spec(key)
            oracles = {row["id"]: row for row in spec["scientific_contract"]["oracles"]}
            self.assertEqual(
                oracles["general_sem_rank0_independent_python_pls_pm"][
                    "implementation"
                ]["version"],
                "rank0_v1",
            )
            self.assertEqual(
                oracles["general_sem_rank0_independent_csem_base_r"]["implementation"][
                    "version"
                ],
                "csem_0_6_1_base_r_v1",
            )
            self.assertIsNone(spec["scientific_contract"]["oracle_exception"])

    def test_scenario_factory_is_replayable_and_covers_supported_mode_b(self) -> None:
        first = qualification.make_mediation_scenario(
            "mixed_mediation",
            measurement_model="mixed_mode_a_b",
            distribution="skewed_heavy_tailed",
            missingness="listwise_mcar_five_percent",
            rows=120,
            seed=90,
        )
        replay = qualification.make_mediation_scenario(
            "mixed_mediation",
            measurement_model="mixed_mode_a_b",
            distribution="skewed_heavy_tailed",
            missingness="listwise_mcar_five_percent",
            rows=120,
            seed=90,
        )
        self.assertEqual(first, replay)
        self.assertTrue(any(block.mode == "B" for block in first.model.blocks))
        same = qualification.make_moderation_scenario(
            "same_focal_simultaneous", rows=100, seed=91
        )
        different = qualification.make_moderation_scenario(
            "different_focal_simultaneous", rows=100, seed=92
        )
        self.assertEqual(len(same.interactions), 2)
        self.assertEqual(len(different.interactions), 2)
        self.assertEqual(same.interactions[0].focal_id, same.interactions[1].focal_id)
        self.assertNotEqual(
            different.interactions[0].focal_id,
            different.interactions[1].focal_id,
        )

        positive = qualification.make_mediation_scenario(
            "parallel_mediation", effect_pattern="positive", rows=100, seed=93
        )
        null = qualification.make_mediation_scenario(
            "parallel_mediation", effect_pattern="broken_stage_null", rows=100, seed=93
        )
        self.assertNotEqual(positive.rows, null.rows)
        self.assertTrue(positive.scenario_id.endswith(":positive"))
        self.assertTrue(null.scenario_id.endswith(":broken_stage_null"))

    def test_micro_harness_passes_without_claiming_qualification(self) -> None:
        report = qualification.run_micro_harness()
        self.assertTrue(report["passed"], report["checks"])
        self.assertFalse(report["qualification_ready"])
        self.assertTrue(report["sources_stable_required_before_receipts"])
        self.assertEqual(
            report["remaining_qualification"]["decision_boundary_trial_policy"][
                "policy_version"
            ],
            qualification.PLAN4B_DECISION_POLICY_VERSION,
        )


if __name__ == "__main__":
    unittest.main()
