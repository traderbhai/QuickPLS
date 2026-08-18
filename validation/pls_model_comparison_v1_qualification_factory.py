#!/usr/bin/env python3
"""Fail-closed QualificationSpec V2 work factory for PLS comparison v1.

The factory preregisters the complete scientific, scenario, operational, and
evidence contract for the exact ``smartpls.pls_model_comparison`` capability
cell.  It only produces source-bound work descriptors.  It never executes
QuickPLS, edits the registry or v1 manifest, emits a qualification receipt, or
grants promotion authority.
"""

from __future__ import annotations

import argparse
import ast
import hashlib
import json
import sys
from pathlib import Path
from typing import Any, Iterable

import jsonschema


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
SPEC_PATH = (
    VALIDATION
    / "qualification_v2"
    / "pls_model_comparison_v1.qualification.json"
)
REGISTRY_PATH = VALIDATION / "capabilities" / "capability_registry_v2.json"
MANIFEST_PATH = VALIDATION / "methods" / "pls_model_comparison_v1.manifest.json"
FACTORY_SCHEMA_PATH = (
    VALIDATION / "pls_model_comparison_v1_qualification_factory.schema.json"
)
ORACLE_WORK_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "pls_model_comparison_v1"
    / "work"
    / "independent_oracle_work.json"
)
AUDIT_PATH = (
    VALIDATION
    / "results"
    / "method_factory"
    / "pls_model_comparison_v1"
    / "qualification_factory_audit.json"
)

sys.path.insert(0, str(VALIDATION))

import pls_model_comparison_v1_oracle as oracle  # noqa: E402
from qualification_spec_v2 import (  # noqa: E402
    canonical_sha256,
    strict_load_json,
    validate_spec_path,
)


QUALIFICATION_ID = "qpls3.comparison.pls_models.qualification_v2"
CAPABILITY_ID = "smartpls.pls_model_comparison"
CELL_ID = "qpls3.comparison.pls_models"
METHOD_VERSION = "pls_model_comparison_v1"
SPEC_FROZEN_AT_UTC = "2026-08-15T00:21:46Z"

EXPECTED_REQUIRED_ROLES = (
    "method_contract",
    "kernel_execution",
    "oracle_independence",
    "generative_recovery",
    "adversarial_boundaries",
    "runner_integration",
    "archive_persistence",
    "canonical_result_projection",
    "gui_contract",
    "cli_contract",
    "cross_format_export",
    "packaged_windows_e2e",
    "accessibility",
    "performance_scale_soak",
    "scientific_review",
)

ROLE_STAGES = {
    "method_contract": "contract",
    "kernel_execution": "kernel",
    "oracle_independence": "oracle",
    "generative_recovery": "generative",
    "adversarial_boundaries": "adversarial",
    "runner_integration": "integration",
    "archive_persistence": "persistence_export",
    "canonical_result_projection": "persistence_export",
    "gui_contract": "product_surface",
    "cli_contract": "product_surface",
    "cross_format_export": "persistence_export",
    "packaged_windows_e2e": "packaged_windows",
    "accessibility": "packaged_windows",
    "performance_scale_soak": "scale_reliability",
    "scientific_review": "review",
}

SOURCE_PATHS = (
    "crates/qpls-estimation/src/lib.rs",
    "crates/qpls-estimation/src/pls_model_comparison.rs",
    "crates/qpls-runner/src/lib.rs",
    "crates/qpls-runner/src/pls_model_comparison_execution.rs",
    "src-tauri/src/lib.rs",
    "src-tauri/src/pls_model_comparison_jobs.rs",
    "src-tauri/src/recipe_v4_jobs.rs",
    "crates/qpls-project/tests/pls_model_comparison_schema6.rs",
    "docs/methods/PLS_MODEL_COMPARISON_V1.md",
    "validation/pls_model_comparison_v1_oracle.py",
    "validation/test_pls_model_comparison_v1_oracle.py",
    "validation/pls_model_comparison_v1_qualification_factory.py",
    "validation/pls_model_comparison_v1_qualification_factory.schema.json",
    "validation/test_pls_model_comparison_v1_qualification_factory.py",
    "validation/qualification_spec_v2.py",
    "validation/qualification_v2/qualification_spec_v2.schema.json",
    "validation/qualification_v2/pls_model_comparison_v1.qualification.json",
    "validation/methods/pls_model_comparison_v1.manifest.json",
    "validation/capabilities/capability_registry_v2.json",
)

FULL_BLOCKERS = (
    "archive.schema6_roundtrip_qualification_receipt_and_historical_future_recovery_campaign_missing",
    "canonical_result_document_v2.immutable_projection_reopen_qualification_receipt_missing",
    "cancellation.prediction_aggregation_bic_and_serialization_checkpoint_coverage_unproven",
    "cli.real_runner_same_run_and_invalid_setup_evidence_missing",
    "contract.bic_failure_partial_availability_policy_unresolved",
    "contract.q_squared_and_cvpat_epsilon_thresholds_differ_from_strict_document_wording",
    "determinism.worker_count_control_and_invariance_missing",
    "engine.failed_fold_accounting_aborts_and_success_payload_always_reports_zero_failures",
    "export.csv_xlsx_html_svg_pdf_png_semantic_readback_missing",
    "frontend.gui_setup_results_comparison_and_run_linkage_missing",
    "generative.selection_recovery_bias_type_i_power_and_failure_campaign_missing",
    "limits.extreme_fold_and_repeat_bounds_missing",
    "metamorphic.graph_id_relabel_scientific_identity_unproven",
    "metamorphic.physical_row_reorder_fold_assignment_invariance_unproven",
    "oracle.full_pipeline_product_to_reference_comparison_missing",
    "oracle.second_independently_maintained_implementation_missing",
    "packaged_windows.installed_portable_offline_matrix_missing",
    "performance.applied_maximum_compound_cancellation_leak_and_soak_missing",
    "preflight.cycle_and_semantic_graph_failures_not_all_rejected_before_estimation",
    "product.accessibility_keyboard_focus_scaling_and_noncolor_evidence_missing",
    "result.strict_unknown_fields_capability_identity_settings_digest_and_run_reference_missing",
    "review.independent_pls_advanced_scientific_review_missing",
    "runner.recipe_v4_progress_cancellation_atomic_commit_qualification_receipt_missing",
    "scope.schema_v3_raw_listwise_unweighted_recursive_two_candidate_boundaries_not_full_parity",
    "simulation.monte_carlo_half_width_and_failed_fit_accounting_missing",
    "surface.internal_labs_result_label_has_no_qualified_canonical_registry_mapping",
)


def _axis(
    identifier: str,
    label: str,
    values: tuple[tuple[str, str], ...],
) -> dict[str, Any]:
    return {
        "id": identifier,
        "label": label,
        "values": [
            {"id": value_id, "description": description}
            for value_id, description in values
        ],
    }


def _profile(
    identifier: str,
    description: str,
    *,
    rows: int,
    indicators: int,
    constructs: int,
    resamples: int,
) -> dict[str, Any]:
    return {
        "id": identifier,
        "description": description,
        "applicability": "required",
        "not_applicable_reason": None,
        "workload": {
            "rows": rows,
            "indicators": indicators,
            "constructs": constructs,
            "resamples": resamples,
            "groups": 1,
            "candidate_models": 2,
        },
    }


def _combination(
    identifier: str,
    profile_id: str,
    coverage: str,
    purpose: str,
    selections: dict[str, list[str]],
    stressed_dimensions: Iterable[str] = (),
) -> dict[str, Any]:
    return {
        "id": identifier,
        "profile_id": profile_id,
        "coverage": coverage,
        "purpose": purpose,
        "stressed_dimensions": list(stressed_dimensions),
        "selections": selections,
    }


def _comparison(
    output_id: str,
    rule: str,
    rationale: str,
    **parameters: Any,
) -> dict[str, Any]:
    return {
        "output_id": output_id,
        "rule": rule,
        "rationale": rationale,
        **parameters,
    }


def _budget(
    profile_id: str,
    hardware_class_id: str,
    elapsed: float,
    memory: int,
    result_bytes: int,
) -> dict[str, Any]:
    return {
        "profile_id": profile_id,
        "hardware_class_id": hardware_class_id,
        "maximum_elapsed_seconds": elapsed,
        "maximum_peak_working_set_bytes": memory,
        "maximum_result_bytes": result_bytes,
        "maximum_cancellation_latency_seconds": 1.0,
    }


def build_spec() -> dict[str, Any]:
    estimand_ids = (
        "shared_fold_contract",
        "plspredict_outcome_comparison",
        "paired_model_cvpat",
        "equation_prediction_oriented_bic",
        "bic_akaike_weights",
    )
    axes = [
        _axis(
            "model_topology",
            "Competing structural topology",
            (
                ("nested_same_targets", "Nested recursive candidates with identical endogenous targets."),
                ("nonnested_same_targets", "Nonnested recursive candidates with identical endogenous targets."),
                ("cyclic_or_target_mismatch_rejected", "Feedback or unequal targets are rejected before folds are created."),
            ),
        ),
        _axis(
            "measurement_model",
            "Measurement configuration",
            (
                ("reflective_targets_mode_a", "Reflective endogenous targets and Mode A exogenous composites."),
                ("reflective_targets_mixed_exogenous_modes", "Reflective targets with supported mixed exogenous composite modes."),
                ("formative_target_rejected", "A formative endogenous target yields a typed applicability failure."),
            ),
        ),
        _axis(
            "construct_type",
            "Construct and derived-term type",
            (
                ("plain_composites", "Plain PLS composite constructs only."),
                ("controls_interactions_higher_order_rejected", "Controls, interactions, or higher-order terms are rejected in v1."),
            ),
        ),
        _axis(
            "data_distribution",
            "Indicator distribution",
            (
                ("gaussian_homoscedastic", "Approximately Gaussian homoscedastic indicators."),
                ("skewed_heavy_tail_heteroscedastic", "Skewed, heavy-tailed, heteroscedastic indicators and outliers."),
            ),
        ),
        _axis(
            "missingness",
            "Complete-case policy",
            (
                ("complete", "All union-model indicators are observed and finite."),
                ("union_listwise_mcar", "MCAR values require one listwise pool across the union of both models."),
                ("extreme_or_nonfinite_insufficient", "Extreme missingness or nonfinite values leave too few complete cases."),
            ),
        ),
        _axis(
            "input_type",
            "Input representation",
            (
                ("raw_numeric_rows", "Raw numeric observations with a stable dataset fingerprint."),
                ("covariance_correlation_rejected", "Covariance or correlation matrices are rejected before comparison."),
            ),
        ),
        _axis(
            "sample_size",
            "Analytical sample size",
            (
                ("minimum_complete_case_boundary", "The exact minimum and one-below-minimum boundary."),
                ("applied_research_size", "A typical applied PLS research sample."),
                ("large_n", "Large row count with bounded model breadth."),
            ),
        ),
        _axis(
            "variable_count",
            "Indicator and construct breadth",
            (
                ("micro", "Hand-checkable indicator and construct count."),
                ("applied", "Typical published-research breadth."),
                ("wide", "Large indicator and construct counts including n less than p boundaries."),
            ),
        ),
        _axis(
            "groups",
            "Grouping and leakage boundary",
            (
                ("ungrouped_random_folds", "One ungrouped observation pool uses deterministic random-style folds."),
                ("group_or_time_leakage_strategy_unavailable", "Grouped or time-ordered leakage controls are unavailable in v1."),
            ),
        ),
        _axis(
            "estimator_options",
            "PLS estimator settings",
            (
                ("identical_supported_settings", "Candidates share weighting, preprocessing, tolerance, and iteration settings."),
                ("mismatched_or_specialized_estimator_rejected", "Mismatched settings, PLSc, weights, or resampling recipes are rejected."),
            ),
        ),
        _axis(
            "effect_strength",
            "Predictive difference",
            (
                ("null_equal_predictive_ability", "Candidates have equal population predictive ability."),
                ("established_better", "The established candidate has lower population loss."),
                ("alternative_better", "The alternative candidate has lower population loss."),
            ),
        ),
        _axis(
            "seed",
            "Fold seed",
            (
                ("frozen_seed_47", "The cross-language digest microcase uses seed 47."),
                ("independent_seed_streams", "Additional preregistered seeds test exact repeatability and plan change."),
            ),
        ),
        _axis(
            "fold_design",
            "Shared cross-validation design",
            (
                ("official_10_by_10", "Ten folds and ten repetitions."),
                ("bounded_micro_plan", "A smaller explicit plan used only for hand and development fixtures."),
                ("invalid_fold_plan_rejected", "Invalid folds, repetitions, confidence, or case counts are rejected."),
            ),
        ),
        _axis(
            "candidate_complexity",
            "Equation complexity",
            (
                ("equal_parameter_counts", "The compared equation has equal predictor counts."),
                ("different_parameter_counts", "The compared equation has different predictor counts and BIC penalties."),
            ),
        ),
        _axis(
            "criterion_boundary",
            "CVPAT and BIC numerical boundary",
            (
                ("ordinary_positive_variance_and_sse", "Positive CVPAT case-difference variance and positive equation SSE."),
                ("zero_or_near_zero_cvpat_variance", "Zero and near-zero case-difference variance require a frozen unavailable threshold."),
                ("nonpositive_or_perfect_bic_sse", "Nonpositive or perfect equation SSE requires a typed failure or separately frozen unavailable policy."),
            ),
        ),
        _axis(
            "workload",
            "Comparison calculation workload",
            (
                ("formula_and_fold_microcases", "Formula, fold, and typed-boundary microcases."),
                ("full_two_candidate_refits", "Both candidates refit on every common fold plus full-sample BIC fits."),
            ),
        ),
        _axis(
            "workers",
            "Worker scheduling",
            (
                ("one_worker", "Serial execution."),
                ("multiple_workers", "Supported parallel execution with identical results and ledgers."),
            ),
        ),
    ]
    profiles = [
        _profile(
            "micro_exact",
            "Hand-checkable two-candidate equations, shared folds, and typed failures.",
            rows=40,
            indicators=6,
            constructs=3,
            resamples=16,
        ),
        _profile(
            "applied",
            "Typical applied two-model comparison using the official 10-by-10 design.",
            rows=500,
            indicators=30,
            constructs=8,
            resamples=200,
        ),
        _profile(
            "large",
            "Large research comparison with full fold, case-loss, and equation ledgers.",
            rows=10_000,
            indicators=80,
            constructs=20,
            resamples=400,
        ),
        _profile(
            "maximum_axis",
            "Separate maximum row, indicator, construct, and fold-refit axes.",
            rows=100_000,
            indicators=300,
            constructs=100,
            resamples=1_000,
        ),
        _profile(
            "compound_stress",
            "Combined high row count, model breadth, repetitions, ledgers, and cancellation.",
            rows=50_000,
            indicators=150,
            constructs=50,
            resamples=1_000,
        ),
    ]
    all_selections = {
        axis["id"]: [value["id"] for value in axis["values"]] for axis in axes
    }
    first_selections = {
        axis["id"]: [axis["values"][0]["id"]] for axis in axes
    }
    last_selections = {
        axis["id"]: [axis["values"][-1]["id"]] for axis in axes
    }
    combinations = [
        _combination(
            "applied_pairwise_all_values",
            "applied",
            "pairwise",
            "A preregistered covering array must exercise every pair of scenario values.",
            all_selections,
        ),
        _combination(
            "micro_exact_formula_fold_and_failure",
            "micro_exact",
            "targeted",
            "Hand equations, frozen fold digest, comparison direction, and typed failures.",
            first_selections,
        ),
        _combination(
            "large_nonnormal_worker_repeatability",
            "large",
            "targeted",
            "Large nonnormal comparison with serial/parallel repeatability and full ledgers.",
            last_selections,
        ),
        *[
            _combination(
                f"maximum_{dimension}",
                "maximum_axis",
                "targeted",
                f"Stress only the {dimension} maximum while the other dimensions remain controlled.",
                all_selections,
                (dimension,),
            )
            for dimension in ("rows", "indicators", "constructs", "resamples")
        ],
        _combination(
            "compound_rows_indicators_constructs_resamples",
            "compound_stress",
            "compound",
            "Combine large data, model breadth, repeated refits, cancellation, persistence, and exports.",
            all_selections,
            ("rows", "indicators", "constructs", "resamples"),
        ),
    ]
    preprocessing = [
        {
            "id": "bind_two_exact_dataset_recipes",
            "order": 0,
            "operation": "Bind exactly two preregistered point-estimate PLS recipes to one immutable raw-data fingerprint and a persisted established-minus-alternative direction.",
            "parameters": {"candidate_models": 2, "post_hoc_candidate_generation": False},
            "applies_to": ["dataset", "established_recipe", "alternative_recipe"],
        },
        {
            "id": "validate_bounded_recipe_and_target_compatibility",
            "order": 1,
            "operation": "Require distinct scientific model digests, identical supported settings, and identical nonempty reflective endogenous target IDs and indicators.",
            "parameters": {"same_model": "error", "unequal_targets": "error"},
            "applies_to": ["candidate_models"],
        },
        {
            "id": "derive_union_complete_case_pool",
            "order": 2,
            "operation": "Resolve numeric indicators across the union of both models and apply one finite listwise pool before any fold assignment.",
            "parameters": {"policy": "union_listwise", "silent_imputation": False},
            "applies_to": ["raw_numeric_rows"],
        },
        {
            "id": "assign_exact_shared_folds",
            "order": 3,
            "operation": "For each repetition rank source rows by SHA-256 over the version, seed, repetition, and source row, then allocate round-robin and persist the complete ledger and digest.",
            "parameters": {
                "version": "seeded_sha256_shared_complete_rows_round_robin_v1",
                "official_folds": 10,
                "official_repeats": 10,
            },
            "applies_to": ["complete_case_rows"],
        },
        {
            "id": "fit_both_candidates_on_every_training_partition",
            "order": 4,
            "operation": "Fit both actual PLS models on each exact shared training partition and retain any typed pair failure without substituting saved-run results.",
            "parameters": {"saved_run_reuse": False, "silent_retry": False},
            "applies_to": ["shared_fold_plan"],
        },
        {
            "id": "predict_common_holdout_indicators",
            "order": 5,
            "operation": "Apply training transforms, outer weights, recursive structural coefficients, and training indicator regressions to the identical holdout rows for both candidates.",
            "parameters": {"prediction_scale": "raw_indicator", "targets": "common_reflective_endogenous"},
            "applies_to": ["candidate_fold_fits", "holdout_rows"],
        },
        {
            "id": "aggregate_shared_prediction_losses",
            "order": 6,
            "operation": "Aggregate indicator SSE, RMSE, MAE, indicator-average benchmark, Q-squared-predict, fold losses, and per-case repeated mean squared losses.",
            "parameters": {"case_loss": "mean_squared_error_over_common_targets"},
            "applies_to": ["holdout_predictions"],
        },
        {
            "id": "calculate_paired_model_cvpat",
            "order": 7,
            "operation": "Compute alternative-minus-established case-loss differences, sample variance with N minus one, t statistic, lower-tail directional probability, two-sided probability, and interval; zero variance is explicitly unavailable.",
            "parameters": {"direction": "alternative_minus_established", "zero_variance": "unavailable"},
            "applies_to": ["paired_case_losses"],
        },
        {
            "id": "fit_full_sample_equations_for_bic",
            "order": 8,
            "operation": "Fit both candidates on the same complete pool and compute each common endogenous structural-score equation SSE and predictor count plus intercept.",
            "parameters": {"parameter_count": "incoming_predictors_plus_intercept", "epsilon_floor": False},
            "applies_to": ["full_complete_case_fits"],
        },
        {
            "id": "calculate_prediction_oriented_bic_and_weights",
            "order": 9,
            "operation": "Calculate equation-level BIC as N log(SSE divided by N) plus p log(N), then normalize exp of minus one-half BIC delta across exactly two candidates.",
            "parameters": {"whole_model_bic": False, "generic_bic_substitution": False, "gm": False},
            "applies_to": ["common_endogenous_equations"],
        },
        {
            "id": "assemble_nonpromotional_result",
            "order": 10,
            "operation": "Assemble method versions, dataset and scientific-model identities, ledgers, warnings, Internal/Labs surface, and qualified false without committing a partial result.",
            "parameters": {"surface": "internal_labs", "qualified": False},
            "applies_to": ["comparison_result"],
        },
    ]
    estimands = [
        {
            "id": "shared_fold_contract",
            "label": "Exact common prediction-task identity",
            "definition": "The two scientific model identities, immutable data identity, union complete-case set, deterministic shared-fold assignment, and exact pair accounting used by every downstream comparison.",
            "unit": "typed identities, row/fold ledger, digest, and integer counts",
            "output_ids": ["model_identity", "shared_fold_plan", "fold_failure_accounting"],
        },
        {
            "id": "plspredict_outcome_comparison",
            "label": "Two-candidate PLSpredict outcome comparison",
            "definition": "Actual common-holdout indicator errors, indicator-average benchmark, Q-squared-predict values, fold losses, and case losses after both models are fully refit on every exact shared training partition.",
            "unit": "raw indicator units, squared indicator units, dimensionless Q-squared, and typed ledgers",
            "output_ids": [
                "indicator_prediction_numeric_metrics",
                "indicator_prediction_status",
                "fold_loss_numeric_table",
                "fold_test_row_ledger",
                "case_loss_numeric_table",
                "case_loss_row_identity",
            ],
        },
        {
            "id": "paired_model_cvpat",
            "label": "Paired model CVPAT",
            "definition": "Alternative-minus-established repeated case-loss mean, sample variance, standard error, t statistic, N minus one degrees of freedom, directional and two-sided probabilities, and interval.",
            "unit": "mean squared prediction loss, probability, t scale, and decision",
            "output_ids": ["cvpat_numeric_statistics", "cvpat_status_and_decision"],
        },
        {
            "id": "equation_prediction_oriented_bic",
            "label": "Prediction-oriented equation BIC",
            "definition": "For each common endogenous structural equation, N log(SSE divided by N) plus p log(N), where p is incoming predictors plus one; no generic or whole-model BIC is substituted.",
            "unit": "equation-level information-criterion value",
            "output_ids": ["equation_bic_numeric_vector", "equation_bic_identity_vector"],
        },
        {
            "id": "bic_akaike_weights",
            "label": "Two-candidate BIC-derived Akaike weights",
            "definition": "Within each common endogenous equation, normalized exp of minus one-half BIC delta for exactly two candidates.",
            "unit": "delta BIC and unit-sum model weight",
            "output_ids": ["equation_akaike_weight_vector", "akaike_weight_status_vector"],
        },
    ]
    model_predicates = [
        {
            "id": "exactly_two_distinct_pls_models",
            "expression": "Exactly two schema-v3 point-estimate PLS algorithm recipes have different canonical scientific model digests after names, UUIDs, and declaration order are removed.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.model.two_distinct_required",
        },
        {
            "id": "same_reflective_endogenous_targets",
            "expression": "Both candidates expose the exact same nonempty reflective endogenous construct IDs and indicator sets.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.model.common_targets_required",
        },
        {
            "id": "bounded_plain_recursive_pls_scope",
            "expression": "Both candidates are recursive and omit observed controls, higher-order constructs, interactions, formative endogenous targets, case weights, PLSc, and resampling recipes.",
            "on_violation": "not_applicable",
            "diagnostic_code": "pls_model_comparison.model.unsupported_v1_feature",
        },
        {
            "id": "identical_estimation_settings",
            "expression": "Weighting, preprocessing, missing-data policy, convergence tolerance, and maximum iterations match exactly between candidates.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.model.settings_mismatch",
        },
    ]
    data_predicates = [
        {
            "id": "same_raw_dataset_fingerprint",
            "expression": "Both recipes bind to the exact raw Dataset fingerprint; covariance and correlation input are unavailable.",
            "on_violation": "not_applicable",
            "diagnostic_code": "pls_model_comparison.data.same_raw_dataset_required",
        },
        {
            "id": "numeric_unique_union_indicators",
            "expression": "Every union-model indicator is numeric, exists once per model, and has finite values on retained rows.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.data.indicator_contract_invalid",
        },
        {
            "id": "sufficient_union_complete_cases",
            "expression": "The shared union-listwise pool has at least max of twenty and twice the fold count complete cases.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.data.insufficient_complete_cases",
        },
        {
            "id": "valid_shared_fold_configuration",
            "expression": "Folds are at least two, repetitions at least one, confidence is strictly between zero and one, and every planned train/test partition is nonempty.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.data.invalid_fold_configuration",
        },
        {
            "id": "valid_prediction_oriented_bic_inputs",
            "expression": "Every common equation has N at least three, finite SSE greater than zero, and parameter count at least one.",
            "on_violation": "error",
            "diagnostic_code": "pls_model_comparison.data.invalid_bic_equation",
        },
    ]
    oracles = [
        {
            "id": "liengaard_et_al_2021",
            "kind": "primary_literature",
            "citation": "Liengaard et al. (2021), Prediction: Coveted, Yet Forsaken? Introducing a Cross-Validated Predictive Ability Test in Partial Least Squares Path Modeling, DOI 10.1111/deci.12445.",
            "locator": "docs/methods/PLS_MODEL_COMPARISON_V1.md",
            "independence_group": "primary_cvpat_2021",
            "runtime_policy": "no_runtime_dependency",
            "implementation": None,
            "covered_estimand_ids": ["shared_fold_contract", "plspredict_outcome_comparison", "paired_model_cvpat"],
        },
        {
            "id": "shmueli_et_al_plspredict",
            "kind": "primary_literature",
            "citation": "Shmueli et al. (2016, 2019), PLSpredict training and holdout prediction principles.",
            "locator": "docs/methods/PLS_MODEL_COMPARISON_V1.md",
            "independence_group": "primary_plspredict",
            "runtime_policy": "no_runtime_dependency",
            "implementation": None,
            "covered_estimand_ids": ["shared_fold_contract", "plspredict_outcome_comparison"],
        },
        {
            "id": "sharma_et_al_2019",
            "kind": "primary_literature",
            "citation": "Sharma et al. (2019), PLS-Based Model Selection: The Role of Alternative Explanations in Information Systems Research, DOI 10.17005/1jais.00538.",
            "locator": "docs/methods/PLS_MODEL_COMPARISON_V1.md",
            "independence_group": "primary_prediction_bic",
            "runtime_policy": "no_runtime_dependency",
            "implementation": None,
            "covered_estimand_ids": ["equation_prediction_oriented_bic"],
        },
        {
            "id": "danks_sharma_sarstedt_2020",
            "kind": "primary_literature",
            "citation": "Danks, Sharma, and Sarstedt (2020), Model Selection Uncertainty and Multimodel Inference in PLS-SEM, DOI 10.1016/j.jbusres.2020.03.019.",
            "locator": "docs/methods/PLS_MODEL_COMPARISON_V1.md",
            "independence_group": "primary_multimodel_weights",
            "runtime_policy": "no_runtime_dependency",
            "implementation": None,
            "covered_estimand_ids": ["bic_akaike_weights"],
        },
        {
            "id": "python_standard_library_micro_oracle_v1",
            "kind": "independent_implementation",
            "citation": "Transparent standard-library implementation of shared-fold assignment, paired CVPAT algebra, prediction-oriented equation BIC, and two-candidate weights; validation only.",
            "locator": "validation/pls_model_comparison_v1_oracle.py",
            "independence_group": "python_stdlib_micro_oracle",
            "runtime_policy": "development_validation_only",
            "implementation": {
                "name": "QuickPLS independent Python PLS comparison micro-oracle",
                "version": "1",
                "maintainer": "QuickPLS validation authors",
            },
            "covered_estimand_ids": ["shared_fold_contract", "paired_model_cvpat", "equation_prediction_oriented_bic", "bic_akaike_weights"],
        },
        {
            "id": "frozen_hand_formula_cases",
            "kind": "hand_calculation",
            "citation": "Frozen hand calculations for fold counts, alternative-minus-established CVPAT moments, equation BIC, BIC deltas, and normalized weights.",
            "locator": "validation/test_pls_model_comparison_v1_oracle.py",
            "independence_group": "hand_formula_microcases",
            "runtime_policy": "development_validation_only",
            "implementation": None,
            "covered_estimand_ids": ["shared_fold_contract", "paired_model_cvpat", "equation_prediction_oriented_bic", "bic_akaike_weights"],
        },
        {
            "id": "required_second_maintained_plspredict_implementation",
            "kind": "independent_implementation",
            "citation": "Qualification requirement: select and freeze a second independently maintained full PLSpredict/model-comparison implementation before evidence admission; currently missing.",
            "locator": "docs/methods/PLS_MODEL_COMPARISON_V1.md#verification",
            "independence_group": "required_external_full_pipeline",
            "runtime_policy": "development_validation_only",
            "implementation": {
                "name": "Unselected independently maintained PLS comparison implementation",
                "version": "required_before_qualification",
                "maintainer": "independent maintainer not yet selected",
            },
            "covered_estimand_ids": list(estimand_ids),
        },
        {
            "id": "required_published_two_model_fixture",
            "kind": "published_fixture",
            "citation": "Qualification requirement: acquire and freeze a published two-model raw-data fixture with independently reproduced fold-level predictions; currently missing.",
            "locator": "docs/methods/PLS_MODEL_COMPARISON_V1.md#verification",
            "independence_group": "required_published_full_pipeline_fixture",
            "runtime_policy": "development_validation_only",
            "implementation": None,
            "covered_estimand_ids": ["plspredict_outcome_comparison"],
        },
    ]
    comparisons = [
        _comparison("model_identity", "exact", "Dataset and canonical scientific model identities must match exactly."),
        _comparison("shared_fold_plan", "exact", "Every complete row, repeat, fold, assignment version, and digest must match exactly."),
        _comparison("fold_failure_accounting", "exact", "Requested, completed, failed, and typed failure identities must match exactly."),
        _comparison("indicator_prediction_numeric_metrics", "matrix_norm", "SSE, absolute error, RMSE, MAE, benchmark, and Q-squared-predict must agree in stable target order.", absolute_tolerance=1e-8, relative_tolerance=1e-7, norm="maximum", elementwise_tolerance=1e-8),
        _comparison("indicator_prediction_status", "exact", "Target identities, observation counts, unavailable values, and lower-RMSE/MAE labels are exact typed state."),
        _comparison("fold_loss_numeric_table", "matrix_norm", "Established, alternative, and signed fold losses must agree after exact fold identity is established.", absolute_tolerance=1e-8, relative_tolerance=1e-7, norm="maximum", elementwise_tolerance=1e-8),
        _comparison("fold_test_row_ledger", "exact", "Repeat, fold, and exact test source-row identities must match."),
        _comparison("case_loss_numeric_table", "matrix_norm", "Per-case repeated mean losses and signed differences must agree in source-row order.", absolute_tolerance=1e-8, relative_tolerance=1e-7, norm="maximum", elementwise_tolerance=1e-8),
        _comparison("case_loss_row_identity", "exact", "Source-row identity and repeat count must match exactly."),
        _comparison("cvpat_numeric_statistics", "abs_relative", "CVPAT moments, probabilities, t statistic, and interval must agree after the exact case-loss ledger matches.", absolute_tolerance=1e-8, relative_tolerance=1e-7),
        _comparison("cvpat_status_and_decision", "exact", "Direction, degrees of freedom, lower-loss model, unavailable status, reason, and decision are categorical outputs."),
        _comparison("equation_bic_numeric_vector", "matrix_norm", "Equation SSE, BIC, and BIC deltas must agree without an epsilon floor.", absolute_tolerance=1e-10, relative_tolerance=1e-9, norm="maximum", elementwise_tolerance=1e-10),
        _comparison("equation_bic_identity_vector", "exact", "Construct identity, sample size, parameter counts, and preferred candidate must match exactly."),
        _comparison("equation_akaike_weight_vector", "matrix_norm", "Two weights must agree and sum to one per equation.", absolute_tolerance=1e-12, relative_tolerance=1e-10, norm="maximum", elementwise_tolerance=1e-12),
        _comparison("akaike_weight_status_vector", "exact", "Candidate order, equation identity, and availability status must match exactly."),
    ]
    hardware = [
        {
            "id": "standard",
            "os_family": "windows",
            "architecture": "x86_64",
            "minimum_logical_cores": 6,
            "minimum_memory_gib": 16,
            "notes": "Product-finalization standard Windows reference class.",
        },
        {
            "id": "workstation",
            "os_family": "windows",
            "architecture": "x86_64",
            "minimum_logical_cores": 12,
            "minimum_memory_gib": 32,
            "notes": "Product-finalization workstation Windows reference class.",
        },
    ]
    standard_budgets = [
        _budget("micro_exact", "standard", 10.0, 512 * 1024**2, 16 * 1024**2),
        _budget("applied", "standard", 600.0, 8 * 1024**3, 256 * 1024**2),
        _budget("large", "standard", 3_600.0, 12 * 1024**3, 1024 * 1024**2),
        _budget("maximum_axis", "standard", 14_400.0, 12 * 1024**3, 2 * 1024**3),
        _budget("compound_stress", "standard", 14_400.0, 12 * 1024**3, 2 * 1024**3),
    ]
    workstation_budgets = [
        _budget("micro_exact", "workstation", 10.0, 1024 * 1024**2, 16 * 1024**2),
        _budget("applied", "workstation", 300.0, 12 * 1024**3, 256 * 1024**2),
        _budget("large", "workstation", 1_800.0, 24 * 1024**3, 1024 * 1024**2),
        _budget("maximum_axis", "workstation", 7_200.0, 24 * 1024**3, 2 * 1024**3),
        _budget("compound_stress", "workstation", 7_200.0, 24 * 1024**3, 2 * 1024**3),
    ]
    return {
        "schema_version": 2,
        "identity": {
            "qualification_id": QUALIFICATION_ID,
            "method_version": METHOD_VERSION,
            "execution_kind": "hybrid",
            "potentially_long_running": True,
            "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": CAPABILITY_ID,
                "capability_version": METHOD_VERSION,
                "cell_id": CELL_ID,
            },
        },
        "migration": {
            "source_kind": "qualification_v1_manifest",
            "source_schema_version": 1,
            "source_manifest_path": "validation/methods/pls_model_comparison_v1.manifest.json",
            "status": "compatibility_only",
            "unresolved_items": [
                "The v1 manifest predates the frozen active parity contract, references an absent Python comparison file, and still includes GM; it is read-only compatibility input and is not qualification evidence.",
                "The Internal/Labs engine is not connected to the runner, recipe v4, project archive, CanonicalResultDocumentV2, GUI, CLI, or export surfaces.",
                "The current independent Python oracle covers formula and fold microcases but not a complete independent PLS refit and holdout-prediction pipeline.",
                "A second independently maintained computational implementation and a published two-model fixture have not been selected and frozen.",
                "Qualification-sized simulations, adversarial product executions, packaged Windows, accessibility, performance, soak, and independent scientific-review evidence are missing.",
            ],
        },
        "scientific_contract": {
            "estimands": estimands,
            "preprocessing": preprocessing,
            "model_predicates": model_predicates,
            "data_predicates": data_predicates,
            "oracles": oracles,
            "oracle_exception": None,
        },
        "scenario_contract": {
            "axes": axes,
            "complexity_profiles": profiles,
            "mandatory_combinations": combinations,
            "monte_carlo_policy": {
                "confidence_level": 0.95,
                "maximum_half_width": 0.01,
                "failed_fits_in_denominator": True,
            },
        },
        "comparison_contract": {"outputs": comparisons},
        "operational_contract": {
            "performance": {
                "hardware_classes": hardware,
                "baseline_policy": {
                    "warmup_runs": 1,
                    "measured_runs": 5,
                    "statistic": "median",
                    "maximum_runtime_regression_percent": 20.0,
                    "maximum_memory_regression_percent": 20.0,
                },
                "budgets": [*standard_budgets, *workstation_budgets],
            },
            "archive": {
                "current_schema_version": 6,
                "readable_schema_versions": [1, 2, 3, 4, 5, 6],
                "writable_schema_versions": [6],
                "future_schema_policy": "verified_read_only",
                "corruption_cases": [
                    "feature_identity",
                    "method_version",
                    "dataset_fingerprint",
                    "checksum",
                    "duplicate_entry",
                    "malformed_payload",
                    "legacy_reinterpretation",
                    "interrupted_save",
                ],
            },
            "export": {
                "formats": ["csv", "xlsx", "html", "svg", "pdf", "png"],
                "semantic_readback_formats": ["csv", "xlsx", "html", "svg", "pdf"],
                "canonical_projection_id": "canonical_result_document_v2_pls_model_comparison_projection",
                "same_run_required": True,
                "provenance_required": True,
                "validation_witness_excluded": True,
            },
            "windows": {
                "package_kinds": ["installed", "portable"],
                "viewports": ["1024x700", "1280x720", "1440x900"],
                "display_scale_percent": [100, 125, 150, 200],
                "offline_required": True,
                "keyboard_only_required": True,
                "accessible_tables_required": True,
                "real_pointer_required": True,
            },
            "cancellation": {
                "required_for_potentially_long_operations": True,
                "maximum_latency_seconds": 1.0,
                "phases": [
                    {"phase": phase, "applicability": "required", "not_applicable_reason": None}
                    for phase in ("validate", "estimate", "resample", "compare", "export")
                ],
                "no_partial_visible_result": True,
                "no_partial_committed_result": True,
                "archive_unchanged": True,
                "same_settings_retry": True,
            },
        },
        "evidence_contract": {
            "required_roles": list(EXPECTED_REQUIRED_ROLES),
            "receipt_contract": {
                "hash_algorithm": "sha256",
                "identity_fields": [
                    "qualification_id",
                    "capability_id",
                    "cell_id",
                    "method_version",
                    "source_set_sha256",
                    "scenario_set_sha256",
                    "build_fingerprint",
                ],
                "source_descriptors_required": True,
                "hardware_fingerprint_required": True,
                "scenario_set_hash_required": True,
            },
            "receipts": [],
        },
    }


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_descriptors() -> list[dict[str, Any]]:
    descriptors = []
    for relative in SOURCE_PATHS:
        path = ROOT / relative
        if not path.is_file():
            raise FileNotFoundError(f"PLS comparison qualification source missing: {relative}")
        descriptors.append(
            {
                "path": relative,
                "size_bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return descriptors


def _artifact_descriptor(path: Path) -> dict[str, Any]:
    return {
        "path": path.relative_to(ROOT).as_posix(),
        "size_bytes": path.stat().st_size,
        "sha256": sha256_file(path),
    }


def _registry_cell() -> tuple[dict[str, Any], dict[str, Any]]:
    registry = strict_load_json(REGISTRY_PATH)
    for capability in registry["capabilities"]:
        if capability["capability_id"] != CAPABILITY_ID:
            continue
        for cell in capability["option_cells"]:
            if cell["cell_id"] == CELL_ID:
                return capability, cell
    raise ValueError("PLS model-comparison cell is missing from CapabilityRegistryV2")


def _oracle_imports() -> list[str]:
    tree = ast.parse(
        (VALIDATION / "pls_model_comparison_v1_oracle.py").read_text(encoding="utf-8")
    )
    imports = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            imports.update(alias.name.split(".", 1)[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom) and node.module:
            imports.add(node.module.split(".", 1)[0])
    return sorted(imports)


def build_oracle_work_report(
    sources: list[dict[str, Any]],
    source_set_sha256: str,
    scenario_set_sha256: str,
) -> dict[str, Any]:
    oracle_result = oracle.build_report()
    oracle.check_report(oracle_result)
    imports = _oracle_imports()
    checks = {
        "frozen_oracle_report_passes": True,
        "standard_library_only": set(imports)
        <= {"argparse", "dataclasses", "hashlib", "json", "math", "struct", "__future__"},
        "no_quickpls_runtime_import": not any(
            name.startswith(("qpls", "quickpls")) for name in imports
        ),
        "cross_language_fold_digest_frozen": oracle_result["fold_assignment_digest"]
        == "sha256:b08f53b2641bc2a2bc8eef4c46c56a5b4f5ad3a413fc195f210ec68212a25c74",
        "cvpat_bic_and_weights_microcases_present": all(
            key in oracle_result for key in ("cvpat", "bic", "weights")
        ),
        "full_pls_refit_oracle_explicitly_not_claimed": True,
        "qualification_and_promotion_explicitly_false": True,
    }
    return {
        "schema_version": 1,
        "report_kind": "pls_model_comparison_v1_independent_oracle_work",
        "work_id": "pls_model_comparison_v1.oracle_micro_work.v1",
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "source_artifacts": sources,
        "source_set_sha256": source_set_sha256,
        "scenario_set_sha256": scenario_set_sha256,
        "oracle_source": next(
            row
            for row in sources
            if row["path"] == "validation/pls_model_comparison_v1_oracle.py"
        ),
        "oracle_test_source": next(
            row
            for row in sources
            if row["path"] == "validation/test_pls_model_comparison_v1_oracle.py"
        ),
        "imports": imports,
        "oracle_result": oracle_result,
        "oracle_result_sha256": canonical_sha256(oracle_result),
        "checks": checks,
        "passed_work_checks": all(checks.values()),
        "work_evidence_only": True,
        "qualification_role_satisfied": False,
        "receipt_eligible": False,
        "qualification_ready": False,
        "promotion_allowed": False,
        "blockers": [
            "The micro-oracle does not independently fit both PLS models or reproduce holdout indicator predictions.",
            "No current product-to-oracle comparison artifact is attached.",
            "A second independently maintained implementation and published two-model fixture are missing.",
        ],
    }


def _work_descriptor(
    role: str,
    status: str,
    *,
    source_set_sha256: str,
    scenario_set_sha256: str,
    required_check_ids: tuple[str, ...],
    required_artifact_classes: tuple[str, ...],
    blockers: tuple[str, ...],
    current_artifacts: list[dict[str, Any]] | None = None,
) -> dict[str, Any]:
    return {
        "descriptor_id": f"pls_model_comparison_v1.{role}.work_v1",
        "role": role,
        "stage": ROLE_STAGES[role],
        "status": status,
        "source_set_sha256": source_set_sha256,
        "scenario_set_sha256": scenario_set_sha256,
        "required_check_ids": list(required_check_ids),
        "required_artifact_classes": list(required_artifact_classes),
        "current_artifacts": current_artifacts or [],
        "blockers": list(blockers),
        "candidate_receipt_emitted": False,
        "qualification_ready": False,
        "promotion_allowed": False,
    }


def build_work_descriptors(
    *,
    source_set_sha256: str,
    scenario_set_sha256: str,
    oracle_work_descriptor: dict[str, Any],
) -> list[dict[str, Any]]:
    definitions = {
        "method_contract": (
            "work_evidence_only",
            ("spec.schema", "spec.semantics", "spec.registry_identity", "spec.zero_receipts"),
            ("qualification_spec_v2", "official_contract_snapshot"),
            ("No immutable contract qualification receipt is attached.",),
        ),
        "kernel_execution": (
            "blocked",
            ("kernel.real_two_model_refit", "kernel.typed_failures", "kernel.cancel_progress", "kernel.no_nan_fallback"),
            ("product_execution_envelope", "cross_language_product_oracle_comparison"),
            ("No hash-current immutable product execution envelope or full pipeline oracle comparison exists.",),
        ),
        "oracle_independence": (
            "work_evidence_only",
            ("oracle.micro_formulas", "oracle.fold_digest", "oracle.full_pipeline", "oracle.second_maintained"),
            ("independent_micro_oracle", "second_maintained_implementation", "published_fixture"),
            ("The micro-oracle is partial; the full independent pipeline, second maintained implementation, and published fixture are missing.",),
        ),
        "generative_recovery": (
            "blocked",
            ("simulation.null_type_i", "simulation.selection_recovery", "simulation.bias_power", "simulation.failures_counted"),
            ("preregistered_simulation_report",),
            ("No qualification-sized simulation or Monte Carlo precision report exists.",),
        ),
        "adversarial_boundaries": (
            "work_evidence_only",
            ("boundary.metamorphic", "boundary.degenerate", "boundary.tamper", "boundary.nonconvergence"),
            ("source_boundary_tests", "product_adversarial_report"),
            ("Source micro-boundaries exist, but no full product adversarial, tamper, worker, GUI/CLI, or archive campaign exists.",),
        ),
        "runner_integration": (
            "work_evidence_only",
            ("runner.recipe_v4", "runner.progress", "runner.cancel", "runner.atomic_commit"),
            ("real_runner_report",),
            (
                "The Internal/Labs runner, progress, cancellation, and atomic publication source tests exist, but no immutable build-bound qualification receipt is attached.",
            ),
        ),
        "archive_persistence": (
            "work_evidence_only",
            ("archive.save_reopen", "archive.history", "archive.future_readonly", "archive.tamper_recovery"),
            ("archive_roundtrip_report",),
            (
                "A focused schema-6 append/reopen source test exists, but no immutable build-bound qualification receipt or historical/future recovery and tamper campaign is attached.",
            ),
        ),
        "canonical_result_projection": (
            "work_evidence_only",
            ("canonical.typed_projection", "canonical.saved_comparison", "canonical.run_identity"),
            ("canonical_result_document_v2_report",),
            (
                "CanonicalResultDocumentV2 projection and exact schema-6 reopen source tests exist, but no immutable build-bound qualification receipt is attached.",
            ),
        ),
        "gui_contract": (
            "blocked",
            ("gui.setup", "gui.preflight", "gui.results", "gui.run_linkage"),
            ("native_gui_interaction_report",),
            ("No real GUI setup, preflight, result, warning, or run-linkage implementation/evidence exists.",),
        ),
        "cli_contract": (
            "blocked",
            ("cli.valid_run", "cli.invalid_setup", "cli.gui_parity", "cli.cancel_retry"),
            ("cli_execution_report",),
            ("No CLI method route or same-result GUI/CLI execution report exists.",),
        ),
        "cross_format_export": (
            "blocked",
            ("export.csv", "export.xlsx", "export.html", "export.svg", "export.pdf", "export.png", "export.semantic_readback"),
            ("cross_format_export_readback_report",),
            ("No canonical CSV, XLSX, HTML, SVG, PDF, or PNG generation and semantic readback evidence exists.",),
        ),
        "packaged_windows_e2e": (
            "blocked",
            ("windows.installed", "windows.portable", "windows.offline", "windows.cleanup"),
            ("packaged_windows_matrix_report",),
            ("No installed/portable offline Windows acceptance matrix exists.",),
        ),
        "accessibility": (
            "blocked",
            ("a11y.keyboard", "a11y.focus", "a11y.tables", "a11y.scaling", "a11y.noncolor"),
            ("accessibility_matrix_report",),
            ("No keyboard, focus, accessible-table, scaling, or noncolor evidence exists.",),
        ),
        "performance_scale_soak": (
            "blocked",
            ("perf.applied", "perf.maximum", "perf.compound", "perf.cancel_latency", "perf.memory_soak"),
            ("performance_baseline_report", "soak_report"),
            ("No applied/maximum/compound performance, cancellation-latency, leak, or soak evidence exists.",),
        ),
        "scientific_review": (
            "blocked",
            ("review.pls_advanced_independent", "review.discrepancies_resolved"),
            ("signed_scientific_review_record",),
            ("No independent PLS advanced/group scientific review is recorded.",),
        ),
    }
    descriptors = []
    for role in EXPECTED_REQUIRED_ROLES:
        status, check_ids, artifact_classes, blockers = definitions[role]
        current_artifacts = (
            [oracle_work_descriptor] if role == "oracle_independence" else []
        )
        descriptors.append(
            _work_descriptor(
                role,
                status,
                source_set_sha256=source_set_sha256,
                scenario_set_sha256=scenario_set_sha256,
                required_check_ids=check_ids,
                required_artifact_classes=artifact_classes,
                blockers=blockers,
                current_artifacts=current_artifacts,
            )
        )
    return descriptors


def build_audit() -> dict[str, Any]:
    spec = strict_load_json(SPEC_PATH)
    manifest = strict_load_json(MANIFEST_PATH)
    validation = validate_spec_path(
        SPEC_PATH,
        repository_root=ROOT,
        registry_path=REGISTRY_PATH,
        require_registry=True,
    )
    capability, cell = _registry_cell()
    sources = source_descriptors()
    source_hash = canonical_sha256(sources)
    scenario_hash = canonical_sha256(spec["scenario_contract"])
    oracle_work = strict_load_json(ORACLE_WORK_PATH)
    oracle_work_descriptor = _artifact_descriptor(ORACLE_WORK_PATH)
    work_descriptors = build_work_descriptors(
        source_set_sha256=source_hash,
        scenario_set_sha256=scenario_hash,
        oracle_work_descriptor=oracle_work_descriptor,
    )
    manifest_evidence = manifest["qualification"]["evidence"]
    checks = {
        "qualification_spec_schema_semantics_and_registry_pass": (
            validation["passed"]
            and validation["schema_valid"]
            and validation["semantic_valid"]
            and validation["registry_verified"]
        ),
        "qualification_spec_is_compatibility_only": (
            spec["migration"]["status"] == "compatibility_only"
            and bool(spec["migration"]["unresolved_items"])
        ),
        "qualification_spec_has_zero_receipts": spec["evidence_contract"]["receipts"] == [],
        "registry_exact_cell_remains_absent_labs": (
            capability["coverage_state"] == "absent"
            and capability["evidence_state"] == "absent"
            and capability["surface"] == "labs"
            and cell["coverage_state"] == "absent"
            and cell["evidence_state"] == "absent"
            and cell["surface"] == "labs"
            and cell["capability_version"] == METHOD_VERSION
        ),
        "manifest_remains_absent_with_empty_evidence": (
            manifest["feature"]["id"] == CELL_ID
            and manifest["feature"]["method_version"] == METHOD_VERSION
            and manifest["qualification"]["declared_state"] == "absent"
            and all(not rows for rows in manifest_evidence.values())
        ),
        "oracle_work_is_current_partial_and_nonpromotional": (
            oracle_work
            == build_oracle_work_report(sources, source_hash, scenario_hash)
            and oracle_work["passed_work_checks"]
            and oracle_work["qualification_role_satisfied"] is False
            and oracle_work["receipt_eligible"] is False
            and oracle_work["qualification_ready"] is False
            and oracle_work["promotion_allowed"] is False
        ),
        "all_work_descriptors_are_hash_bound": all(
            row["source_set_sha256"] == source_hash
            and row["scenario_set_sha256"] == scenario_hash
            and row["qualification_ready"] is False
            and row["promotion_allowed"] is False
            and row["candidate_receipt_emitted"] is False
            for row in work_descriptors
        ),
        "required_roles_are_exact_and_unique": (
            tuple(row["role"] for row in work_descriptors) == EXPECTED_REQUIRED_ROLES
            and len({row["descriptor_id"] for row in work_descriptors})
            == len(EXPECTED_REQUIRED_ROLES)
        ),
        "full_blocker_set_is_preregistered": (
            len(set(FULL_BLOCKERS)) == len(FULL_BLOCKERS)
            and all(
                fragment in " ".join(FULL_BLOCKERS)
                for fragment in (
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
            )
        ),
        "no_candidate_or_attached_receipts": True,
    }
    return {
        "schema_version": 1,
        "report_kind": "pls_model_comparison_v1_qualification_factory_audit",
        "qualification_id": QUALIFICATION_ID,
        "capability_id": CAPABILITY_ID,
        "cell_id": CELL_ID,
        "method_version": METHOD_VERSION,
        "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
        "spec_descriptor": _artifact_descriptor(SPEC_PATH),
        "oracle_work_report_descriptor": oracle_work_descriptor,
        "source_artifacts": sources,
        "source_set_sha256": source_hash,
        "scenario_set_sha256": scenario_hash,
        "checks": checks,
        "passed": all(checks.values()),
        "role_matrix": [
            {
                "role": row["role"],
                "status": row["status"],
                "candidate_receipt_emitted": False,
                "qualification_ready": False,
            }
            for row in work_descriptors
        ],
        "work_descriptors": work_descriptors,
        "candidate_receipt_descriptors": [],
        "attached_receipt_count": 0,
        "registry_mutated": False,
        "manifest_mutated": False,
        "qualification_spec_receipts_mutated": False,
        "scientific_review_satisfied": False,
        "qualification_ready": False,
        "promotion_allowed": False,
        "remaining_blockers": list(FULL_BLOCKERS),
        "note": (
            "Source-bound validation work only. A green factory audit means the "
            "missing work is preregistered and the current micro-oracle is unchanged; "
            "it is never capability evidence or promotion authority."
        ),
    }


def _write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )


def _validate_factory_schema(document: dict[str, Any]) -> None:
    schema = strict_load_json(FACTORY_SCHEMA_PATH)
    jsonschema.Draft202012Validator(schema).validate(document)


def write_factory_artifacts() -> dict[str, Any]:
    protected_before = {
        REGISTRY_PATH: sha256_file(REGISTRY_PATH),
        MANIFEST_PATH: sha256_file(MANIFEST_PATH),
    }
    _write_json(SPEC_PATH, build_spec())
    sources = source_descriptors()
    source_hash = canonical_sha256(sources)
    scenario_hash = canonical_sha256(build_spec()["scenario_contract"])
    work = build_oracle_work_report(sources, source_hash, scenario_hash)
    _validate_factory_schema(work)
    _write_json(ORACLE_WORK_PATH, work)
    audit = build_audit()
    _validate_factory_schema(audit)
    protected_after = {path: sha256_file(path) for path in protected_before}
    if protected_before != protected_after:
        raise RuntimeError("PLS comparison qualification factory mutated registry or manifest")
    _write_json(AUDIT_PATH, audit)
    return audit


def verify_checked_in_factory() -> dict[str, Any]:
    errors = []
    try:
        if strict_load_json(SPEC_PATH) != build_spec():
            errors.append("qualification_spec_missing_or_stale")
        validation = validate_spec_path(
            SPEC_PATH,
            repository_root=ROOT,
            registry_path=REGISTRY_PATH,
            require_registry=True,
        )
        if not validation["passed"]:
            errors.append("qualification_spec_schema_or_semantics_invalid")
        sources = source_descriptors()
        source_hash = canonical_sha256(sources)
        scenario_hash = canonical_sha256(build_spec()["scenario_contract"])
        expected_work = build_oracle_work_report(sources, source_hash, scenario_hash)
        actual_work = strict_load_json(ORACLE_WORK_PATH)
        _validate_factory_schema(actual_work)
        if actual_work != expected_work:
            errors.append("oracle_work_report_missing_or_stale")
        expected_audit = build_audit()
        actual_audit = strict_load_json(AUDIT_PATH)
        _validate_factory_schema(actual_audit)
        if actual_audit != expected_audit:
            errors.append("qualification_factory_audit_missing_or_stale")
        if actual_audit.get("candidate_receipt_descriptors") != []:
            errors.append("factory_emitted_candidate_receipt")
        if actual_audit.get("attached_receipt_count") != 0:
            errors.append("factory_attached_receipt")
        if (
            actual_audit.get("qualification_ready") is not False
            or actual_audit.get("promotion_allowed") is not False
        ):
            errors.append("factory_makes_qualification_or_promotion_claim")
        if set(actual_audit.get("remaining_blockers", [])) != set(FULL_BLOCKERS):
            errors.append("factory_blocker_set_incomplete")
    except (OSError, ValueError, json.JSONDecodeError, jsonschema.ValidationError) as error:
        errors.append(f"factory_verification_error:{type(error).__name__}:{error}")
    return {
        "passed": not errors,
        "errors": errors,
        "qualification_ready": False,
        "promotion_allowed": False,
        "candidate_receipt_count": 0,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write", action="store_true")
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    if args.write == args.verify:
        parser.error("select exactly one of --write or --verify")
    result = write_factory_artifacts() if args.write else verify_checked_in_factory()
    print(json.dumps(result, indent=2, sort_keys=True, allow_nan=False))
    return 0 if result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
