#!/usr/bin/env python3
"""Focused fail-closed tests for split result/preparation metamorphic evidence."""

from __future__ import annotations

import copy
import importlib.util
import unittest
from pathlib import Path


HERE = Path(__file__).resolve().parent
MODULE_PATH = HERE / "verify_multimod_metamorphic_qualification_v1.py"
SPEC = importlib.util.spec_from_file_location("multimod_metamorphic_verifier", MODULE_PATH)
if SPEC is None or SPEC.loader is None:  # pragma: no cover - import bootstrap guard
    raise RuntimeError(f"cannot load verifier from {MODULE_PATH}")
verifier = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(verifier)


def preparation_case(profile: str = "p2_multi_two_way") -> dict:
    contract = verifier.COMMON_METRIC_PREPARATION_CONTRACT[profile]
    algorithm = "pls_pos_destination_scored_interactions_v2"
    common_metric = {
        "method_version": "qpls.pos-common-metric.runner.v1",
        "gate_input": {"segments": 2},
        "gate_result": {
            "status": "passed",
            "inferential_gamma_delta_slope_contrasts_allowed": True,
            "blockers": [],
        },
        "micom_pairs": [{"left_group_id": "segment_1", "right_group_id": "segment_2"}],
        "common_metric_parameters": [{"target_id": "segment_1:path:x->y"}],
    }
    raw_preparation = {"fimix_input": {"interaction_profile": profile}}
    pos_result = {
        "method_version": "qpls.pls-pos.destination-scored-interactions.v2",
        "scoring_contract": {
            "destination_scored_interactions": {"profile": profile}
        },
        "objective": 1.0,
    }
    return {
        "cell_id": contract["cell_id"],
        "execution_scope": verifier.COMMON_METRIC_PREPARATION_SCOPE,
        "profile": profile,
        "dataset_rows": 80,
        "config": {
            "profile": profile,
            "bootstrap": {"resamples": 500},
            "phase": {
                "kind": "inference",
                "lock": {
                    "discovery_candidate_k": [2],
                    "discovery_algorithms": [algorithm],
                    "selected_algorithm": algorithm,
                    "selected_k": 2,
                    "analyst_lock_confirmed": True,
                },
            },
            "pos_common_metric": {
                "request_segment_contrasts": True,
                "permutation_samples": 5000,
                "require_partial_compositional_invariance": True,
            },
        },
        "compiled_plan": {
            "kind": "pls_heterogeneity_v2",
            "profile": profile,
            "algorithms": [algorithm],
            "candidate_k": [2],
        },
        "profile_preparation": {
            "prepared_point": {
                "fimix_input": {"interaction_profile": profile},
                "pos_start_features": [[1.0, 2.0], [3.0, 4.0]],
                "pos_common_metric_gate": common_metric["gate_input"],
                "pos_common_metric_parameters": common_metric[
                    "common_metric_parameters"
                ],
                "pos_common_metric_micom_pairs": common_metric["micom_pairs"],
            },
            "raw_preparation_receipt": raw_preparation,
            "point_pass": {
                "locked": {
                    "algorithm": algorithm,
                    "k": 2,
                },
                "pooled_baseline": {},
                "fimix_candidates": [],
                "pos_candidates": [
                    {
                        "algorithm": algorithm,
                        "k": 2,
                        "result": pos_result,
                    }
                ],
            },
            "common_metric_evidence": common_metric,
            "reference": {
                "algorithm": algorithm,
                "k": 2,
                "complete_source_row_tokens": [100, 200],
                "reference_assignments": [0, 1],
                "use_pooled_common_metric": True,
                "heterogeneity_plan": {"requested_replicates": 500},
            },
        },
        "evidence": {
            "fimix": [],
            "pos": [{"k": 2, "result": pos_result}],
            "pooled_baseline": [{}],
            "raw_preparation": [raw_preparation],
            "common_metric": [common_metric],
            "bootstrap": [],
        },
        "bootstrap_dependency": {
            "configured_but_not_executed_here": True,
            "configured_resamples": 500,
            "global_metamorphic_matrix_scope": (
                "locked_point_and_common_metric_preparation_only"
            ),
            "dependency_status": verifier.COMMON_METRIC_DEPENDENCY_STATUS,
            "dependency_gate_id": verifier.COMMON_METRIC_DEPENDENCY_GATE,
            "dependency_step_id": verifier.COMMON_METRIC_DEPENDENCY_STEP,
            "dedicated_full_bootstrap_shard_id": contract["bootstrap_shard_id"],
        },
    }


class PreparationContractTests(unittest.TestCase):
    def test_execution_plan_scope_is_not_misclassified_as_preparation(self) -> None:
        execution_plan = {
            "execution_scope": "full_requested_multigroup_analysis",
            "analysis": {"estimate": 0.5},
            "dataset_rows": 80,
            "cell_id": "ordinary-result",
        }
        report = {"execution_plan": execution_plan}

        self.assertFalse(verifier.is_common_metric_preparation(execution_plan))
        self.assertEqual(set(verifier.result_cases(report)), {"ordinary-result"})
        preparations, profiles, failures = verifier.preparation_cases(report)
        self.assertEqual(preparations, {})
        self.assertEqual(profiles, {})
        self.assertEqual(failures, [])

    def test_provenance_hash_vectors_do_not_create_scientific_differences(self) -> None:
        baseline = {
            "estimate": 0.5,
            "record_identity_sha256s": ["a" * 64, "b" * 64],
        }
        transformed = {
            "estimate": 0.5,
            "record_identity_sha256s": ["c" * 64, "d" * 64],
        }

        self.assertEqual(
            verifier.normalize(baseline, row_count=2, row_reverse=False),
            verifier.normalize(transformed, row_count=2, row_reverse=False),
        )

    def test_row_reverse_normalizes_row_bound_columns_weights_and_start_order(self) -> None:
        transformed = {
            "columns": [{"variable_id": "x", "values": [20.0, 10.0]}],
            "case_weights": [2.0, 1.0],
            "completed_starts": [
                {"start_index": 1, "objective": 0.5},
                {"start_index": 0, "objective": 1.0},
            ],
        }
        baseline = {
            "columns": [{"variable_id": "x", "values": [10.0, 20.0]}],
            "case_weights": [1.0, 2.0],
            "completed_starts": [
                {"start_index": 0, "objective": 1.0},
                {"start_index": 1, "objective": 0.5},
            ],
        }

        self.assertEqual(
            verifier.normalize(baseline, row_count=2, row_reverse=False),
            verifier.normalize(transformed, row_count=2, row_reverse=True),
        )
        probe_grid = {
            "variable_id": "x",
            "values": [-1.0, 0.0, 1.0],
            "kind": "standardized_probe_grid",
        }
        self.assertEqual(
            verifier.normalize(probe_grid, row_count=3, row_reverse=True)["values"],
            [-1.0, 0.0, 1.0],
        )

    def test_group_reversal_canonicalizes_nested_micom_and_directional_evidence(self) -> None:
        forward = {
            "pair": {"group_a": 0, "group_b": 1},
            "audit_step2": {
                "observed_mean_difference_a_minus_b": -0.5,
                "permutation_log_variance_ratios": [0.2, -0.1],
            },
            "parameters": [
                {
                    "difference_a_minus_b": -0.25,
                    "estimate_a": 0.5,
                    "estimate_b": 0.75,
                    "p_value_greater": 0.8,
                    "p_value_less": 0.2,
                    "p_value_two_sided": 0.4,
                }
            ],
        }
        reverse = {
            "pair": {"group_a": 1, "group_b": 0},
            "audit_step2": {
                "observed_mean_difference_a_minus_b": 0.5,
                "permutation_log_variance_ratios": [-0.2, 0.1],
            },
            "parameters": [
                {
                    "difference_a_minus_b": 0.25,
                    "estimate_a": 0.75,
                    "estimate_b": 0.5,
                    "p_value_greater": 0.2,
                    "p_value_less": 0.8,
                    "p_value_two_sided": 0.4,
                }
            ],
        }

        self.assertEqual(
            verifier.canonical_group_contrast_orientation(forward),
            verifier.canonical_group_contrast_orientation(reverse),
        )
        reverse["parameters"][0]["p_value_two_sided"] = 0.5
        self.assertNotEqual(
            verifier.canonical_group_contrast_orientation(forward),
            verifier.canonical_group_contrast_orientation(reverse),
        )

    def test_valid_preparation_is_compared_but_never_counted_as_result(self) -> None:
        case = preparation_case()
        report = {"compact_common_metric_profile_executions": [case]}

        self.assertEqual(verifier.result_cases(report), {})
        preparations, profiles, failures = verifier.preparation_cases(report)
        self.assertEqual(failures, [])
        self.assertEqual(set(preparations), {case["cell_id"]})
        self.assertEqual(
            profiles[case["cell_id"]], "pos.common_metric.p2_multi_two_way.v1"
        )
        self.assertIn(
            "pos.common_metric.p2_multi_two_way.v1",
            verifier.executed_profile_inventory("heterogeneity", report),
        )

        comparison = verifier.compare_axis(
            "heterogeneity", report, copy.deepcopy(report), "seed_repeat"
        )
        self.assertEqual(comparison["status"], "passed")
        self.assertEqual(comparison["baseline_completed_result_count"], 0)
        self.assertEqual(comparison["baseline_preparation_count"], 1)
        self.assertFalse(comparison["preparation_is_completed_result"])

    def test_preparation_rejects_result_alias_without_leaking_into_results(self) -> None:
        case = preparation_case()
        case["analysis"] = {"descriptive_only": False}
        report = {"cells": [case]}

        self.assertEqual(verifier.result_cases(report), {})
        preparations, _, failures = verifier.preparation_cases(report)
        self.assertEqual(preparations, {})
        self.assertTrue(any("result_alias_forbidden" in row for row in failures))

    def test_preparation_requires_a_passed_common_metric_gate(self) -> None:
        case = preparation_case()
        case["evidence"]["common_metric"][0]["gate_result"]["status"] = (
            "descriptive_only"
        )
        case["profile_preparation"]["common_metric_evidence"] = copy.deepcopy(
            case["evidence"]["common_metric"][0]
        )
        report = {"cells": [case]}

        preparations, _, failures = verifier.preparation_cases(report)
        self.assertEqual(preparations, {})
        self.assertTrue(any("common_metric_gate_not_passed" in row for row in failures))
        self.assertNotIn(
            "pos.common_metric.p2_multi_two_way.v1",
            verifier.executed_profile_inventory("heterogeneity", report),
        )

    def test_preparation_rejects_a_completed_bootstrap_or_unbound_dependency(self) -> None:
        case = preparation_case("p23_all_current")
        case["evidence"]["bootstrap"] = [{"qualification": "qualified"}]
        case["bootstrap_dependency"]["dependency_gate_id"] = "metamorphic.global"

        preparations, _, failures = verifier.preparation_cases({"cells": [case]})
        self.assertEqual(preparations, {})
        self.assertTrue(any("bootstrap_result_forbidden" in row for row in failures))
        self.assertTrue(any("bootstrap_dependency_contract" in row for row in failures))

    def test_preparation_profile_identity_is_bound_through_config_plan_and_raw_inputs(self) -> None:
        case = preparation_case("p2_multi_two_way")
        p23 = verifier.COMMON_METRIC_PREPARATION_CONTRACT["p23_all_current"]
        case["profile"] = "p23_all_current"
        case["cell_id"] = p23["cell_id"]
        case["bootstrap_dependency"]["dedicated_full_bootstrap_shard_id"] = p23[
            "bootstrap_shard_id"
        ]

        preparations, _, failures = verifier.preparation_cases({"cells": [case]})
        self.assertEqual(preparations, {})
        self.assertTrue(any("config_profile_identity" in row for row in failures))
        self.assertTrue(any("compiled_plan_profile_identity" in row for row in failures))
        self.assertTrue(any("prepared_profile_identity" in row for row in failures))
        self.assertTrue(any("prepared_pos_candidate_identity" in row for row in failures))

    def test_row_reverse_maps_rows_without_reversing_matrix_columns(self) -> None:
        baseline = preparation_case()
        baseline["dataset_rows"] = 2
        baseline_preparation = baseline["profile_preparation"]
        baseline_preparation["point_pass"]["locked"]["assignments"] = [0, 1]
        baseline_preparation["point_pass"]["pos_candidates"][0]["result"].update(
            {
                "assignments": [0, 1],
                "posteriors": [[0.9, 0.1], [0.2, 0.8]],
                "multistart_evidence": {
                    "completed_starts": [
                        {
                            "canonical_assignments": [0, 1],
                            "canonical_posteriors": [[0.9, 0.1], [0.2, 0.8]],
                        }
                    ]
                },
                "segments": [
                    {
                        "fit": {
                            "outcome_fit_audits": [
                                {
                                    "source_row_indices": [0, 1],
                                    "observed_scores": [10.0, 20.0],
                                    "fitted_scores": [11.0, 21.0],
                                }
                            ]
                        }
                    }
                ],
            }
        )
        baseline["evidence"]["pos"][0]["result"] = copy.deepcopy(
            baseline_preparation["point_pass"]["pos_candidates"][0]["result"]
        )

        transformed = copy.deepcopy(baseline)
        transformed_preparation = transformed["profile_preparation"]
        transformed_preparation["prepared_point"]["pos_start_features"] = [
            [3.0, 4.0],
            [1.0, 2.0],
        ]
        transformed_preparation["reference"]["reference_assignments"] = [1, 0]
        transformed_preparation["reference"]["complete_source_row_tokens"] = [200, 100]
        transformed_preparation["point_pass"]["locked"]["assignments"] = [1, 0]
        transformed_result = transformed_preparation["point_pass"]["pos_candidates"][0][
            "result"
        ]
        transformed_result["assignments"] = [1, 0]
        transformed_result["posteriors"] = [[0.2, 0.8], [0.9, 0.1]]
        completed = transformed_result["multistart_evidence"]["completed_starts"][0]
        completed["canonical_assignments"] = [1, 0]
        completed["canonical_posteriors"] = [[0.2, 0.8], [0.9, 0.1]]
        audit = transformed_result["segments"][0]["fit"]["outcome_fit_audits"][0]
        audit["source_row_indices"] = [0, 1]
        audit["observed_scores"] = [20.0, 10.0]
        audit["fitted_scores"] = [21.0, 11.0]
        transformed["evidence"]["pos"][0]["result"] = copy.deepcopy(transformed_result)

        comparison = verifier.compare_axis(
            "heterogeneity",
            {"cells": [baseline]},
            {"cells": [transformed]},
            "row_reverse",
        )
        self.assertEqual(comparison["status"], "passed", comparison["failures"])
        normalized = verifier.normalize(
            [[3.0, 4.0], [1.0, 2.0]],
            row_count=2,
            row_reverse=True,
            parent_key="pos_start_features",
        )
        self.assertEqual(normalized, [[1.0, 2.0], [3.0, 4.0]])

    def test_row_bound_metric_hashes_are_provenance_not_scientific_estimates(self) -> None:
        first = "a" * 64
        second = "b" * 64
        baseline = {
            "metric_id": f"qpls.heterogeneity.pooled-standardized-metric.v2:{first}",
            "pooled_metric_id": (
                f"qpls.heterogeneity.pooled-standardized-metric.v2:{first}"
            ),
            "metric": f"qpls.pos.pooled-common-metric.v1:{first}",
        }
        transformed = {
            "metric_id": f"qpls.heterogeneity.pooled-standardized-metric.v2:{second}",
            "pooled_metric_id": (
                f"qpls.heterogeneity.pooled-standardized-metric.v2:{second}"
            ),
            "metric": f"qpls.pos.pooled-common-metric.v1:{second}",
        }
        self.assertEqual(
            verifier.normalize(baseline, row_count=2, row_reverse=False),
            verifier.normalize(transformed, row_count=2, row_reverse=True),
        )

    def test_result_shaped_common_metric_evidence_cannot_substitute_for_preparation(self) -> None:
        report = {
            "dataset_rows": 80,
            "profile": "p2_multi_two_way",
            "analysis": {"locked_k": 2},
            "evidence": {"pos": [{}], "common_metric": [{"gate_result": {"status": "passed"}}]},
        }
        inventory = verifier.executed_profile_inventory("heterogeneity", report)
        self.assertIn("pos.destination_scored.p2_multi_two_way.v2", inventory)
        self.assertNotIn("pos.common_metric.p2_multi_two_way.v1", inventory)

    def test_compact_heterogeneity_contract_declares_preparation_facts(self) -> None:
        report = {
            "suite_id": "qpls.multimod.heterogeneity.production-qualification.v2",
            "execution_contract": (
                "public_recipe_v4_compiler_plus_raw_fimix_pos_runner_and_"
                "declared_profile_preparation"
            ),
            "qualification_claim": (
                "raw_sut_results_and_preparation_facts_for_independent_comparison_only"
            ),
        }
        self.assertTrue(verifier.production_contract_valid("heterogeneity", report))
        report["qualification_claim"] = "raw_sut_facts_for_independent_comparison_only"
        self.assertFalse(verifier.production_contract_valid("heterogeneity", report))


if __name__ == "__main__":
    unittest.main()
