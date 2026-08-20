#!/usr/bin/env python3
"""Shared frozen scenario/spec factory for General SEM Rank 0 qualification.

The module owns no production code and writes no evidence.  It provides:

* deterministic synthetic indicator-level mediation and moderation scenarios;
* four QualificationSpec V2 documents for the exact Rank 0 capability cells;
* a fast scientific micro-harness suitable for source iteration; and
* an explicit qualification-scale workload that remains pending until all
  compared sources are stable.

Static ``*.qualification.json`` files are the reviewable contracts.  The
factory exists to prevent those four contracts from silently drifting apart.
"""

from __future__ import annotations

import argparse
import json
import math
import random
from dataclasses import dataclass
from pathlib import Path
from types import MappingProxyType
from typing import Mapping, Sequence

from general_sem_rank0_independent_pls_oracle import (
    BlockSpec,
    InteractionSpec,
    PathSpec,
    PlsModelSpec,
    bootstrap_mediation,
    bootstrap_moderation,
    fit_pls_pm,
    fit_simultaneous_moderation,
    mediation_effects,
    standardize,
)


ROOT = Path(__file__).resolve().parents[1]
SPEC_DIRECTORY = ROOT / "validation" / "qualification_v2"
SPEC_FROZEN_AT_UTC = "2026-08-19T00:00:00Z"
QUALIFICATION_SPEC_VERSION = 2
MINIMUM_WORST_CASE_BINOMIAL_TRIALS = 9_604
PLAN4B_DECISION_POLICY_VERSION = (
    "general_sem_rank0_decision_boundary_fixed_n_continuation_v1"
)
# Plan 4B retains the original 95% Wilson +/-1 percentage-point requirement
# where a metric can pass.  The old 9,604 global minimum was sized at p=.50,
# even though every frozen acceptance interval lies close to zero or one.
# These minima are calculated at the least-favourable passing boundary.  The
# execution targets are whole 64-trial shards and are frozen before Plan 4B
# scientific outcomes are inspected.
PLAN4B_METRIC_TRIAL_POLICY: Mapping[str, Mapping[str, object]] = MappingProxyType(
    {
        "effect_recovery_rate": MappingProxyType(
            {
                "decision_rate": 0.90,
                "minimum_trials": 3_465,
                "execution_target_trials": 9_604,
            }
        ),
        "empirical_coverage": MappingProxyType(
            {
                "decision_rate": 0.90,
                "minimum_trials": 3_465,
                "execution_target_trials": 3_520,
            }
        ),
        "null_rejection_rate": MappingProxyType(
            {
                "decision_rate": 0.08,
                "minimum_trials": 2_835,
                "execution_target_trials": 2_880,
            }
        ),
        "failure_classification_rate": MappingProxyType(
            {
                "decision_rate": 0.95,
                "minimum_trials": 1_839,
                "execution_target_trials": 9_604,
            }
        ),
        "worker_replay_rate": MappingProxyType(
            {
                "decision_rate": 0.99,
                "minimum_trials": 473,
                "execution_target_trials": 1_024,
            }
        ),
        "seed_replay_rate": MappingProxyType(
            {
                "decision_rate": 0.99,
                "minimum_trials": 473,
                "execution_target_trials": 1_024,
            }
        ),
    }
)
# These two coverage prefixes were complete when Rank 0 was paused.  They are
# frozen from completion state only (never from event rates) so Plan 4B carries
# every already-accepted shard while still executing no additional coverage.
PLAN4B_SCENARIO_TRIAL_OVERRIDES: Mapping[str, int] = MappingProxyType(
    {
        "coverage.mediation_bootstrap": 9_604,
        "coverage.moderation_bootstrap": 4_480,
    }
)
PERFORMANCE_HARDWARE_PROFILE_ID = "standard_windows_6c16g"
PERFORMANCE_PROFILE_MANIFEST = (
    ROOT
    / "validation"
    / "capabilities"
    / "complexity_performance_profiles_v2.manifest.json"
)

# These values are frozen before qualification output exists.  Changing them
# is a scientific-contract revision, never an evidence-regeneration detail.
FROZEN_THRESHOLDS = MappingProxyType(
    {
        "deterministic_absolute_tolerance": 1.0e-8,
        "independent_absolute_tolerance": 1.0e-6,
        "independent_relative_tolerance": 1.0e-5,
        "monte_carlo_confidence_level": 0.95,
        "monte_carlo_maximum_half_width": 0.01,
        "recovery_acceptance_interval": (0.90, 1.00),
        "mediation_recovery_absolute_bias_maximum": 0.05,
        "mediation_recovery_rmse_maximum": 0.15,
        "moderation_recovery_absolute_bias_maximum": 0.08,
        "moderation_recovery_rmse_maximum": 0.20,
        "coverage_acceptance_interval": (0.90, 0.99),
        "null_rejection_acceptance_interval": (0.00, 0.08),
        "failure_classification_acceptance_interval": (0.95, 1.00),
        "worker_replay_acceptance_interval": (0.99, 1.00),
        "seed_replay_acceptance_interval": (0.99, 1.00),
        "minimum_worst_case_binomial_trials": MINIMUM_WORST_CASE_BINOMIAL_TRIALS,
    }
)


@dataclass(frozen=True)
class CellContract:
    key: str
    filename: str
    capability_id: str
    cell_id: str
    method_version: str
    analytical_method_version: str
    execution_kind: str
    stochastic: bool
    family: str
    source_manifest: str
    method_document: str
    arithmetic_reference: str
    topology_values: tuple[tuple[str, str], ...]


@dataclass(frozen=True)
class GeneralSemScenario:
    scenario_id: str
    rows: tuple[Mapping[str, float | None], ...]
    model: PlsModelSpec
    source_id: str | None = None
    target_id: str | None = None
    interactions: tuple[InteractionSpec, ...] = ()
    expected_gamma_signs: Mapping[str, int] | None = None


CELL_CONTRACTS: Mapping[str, CellContract] = MappingProxyType(
    {
        "mediation_point": CellContract(
            key="mediation_point",
            filename="mediation_v1.qualification.json",
            capability_id="smartpls.mediation",
            cell_id="qpls3.pls.mediation",
            method_version="pls_mediation_v1",
            analytical_method_version="pls_mediation_v1",
            execution_kind="iterative",
            stochastic=False,
            family="mediation",
            source_manifest="validation/methods/mediation_v1.manifest.json",
            method_document="docs/methods/PLS_MEDIATION_V1.md",
            arithmetic_reference="validation/mediation_reference.py",
            topology_values=(
                ("parallel_mediation", "Two or more parallel indirect paths."),
                ("serial_mediation", "A recursive serial indirect path."),
                ("mixed_mediation", "Parallel and serial paths in one DAG."),
            ),
        ),
        "mediation_bootstrap": CellContract(
            key="mediation_bootstrap",
            filename="general_sem_pls_multiple_mediation_bootstrap_v1.qualification.json",
            capability_id="smartpls.mediation",
            cell_id="qpls3.pls.general_sem_multiple_mediation_bootstrap",
            method_version="general_sem_pls_full_model_case_bootstrap_v1",
            analytical_method_version="general_sem_pls_full_model_case_bootstrap_v1",
            execution_kind="stochastic",
            stochastic=True,
            family="mediation",
            source_manifest="validation/methods/general_sem_pls_multiple_mediation_bootstrap_v1.manifest.json",
            method_document="docs/methods/GENERAL_SEM_PLS_MULTIPLE_MEDIATION_BOOTSTRAP_V1.md",
            arithmetic_reference="validation/general_sem_pls_multiple_mediation_bootstrap_v1_reference.py",
            topology_values=(
                ("parallel_mediation", "Two or more parallel indirect paths."),
                ("serial_mediation", "A recursive serial indirect path."),
                ("mixed_mediation", "Parallel and serial paths in one DAG."),
            ),
        ),
        "moderation_point": CellContract(
            key="moderation_point",
            filename="general_sem_pls_multiple_moderation_point_v1.qualification.json",
            capability_id="smartpls.moderation",
            cell_id="qpls3.pls.general_sem_multiple_two_way_moderation_point",
            method_version="general_sem_pls_multiple_two_way_moderation_point_v1",
            analytical_method_version="qpls.general-sem-pls.multiple-two-way.point.v1",
            execution_kind="iterative",
            stochastic=False,
            family="moderation",
            source_manifest="validation/methods/general_sem_pls_multiple_moderation_point_v1.manifest.json",
            method_document="docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_POINT_V1.md",
            arithmetic_reference="validation/general_sem_pls_multiple_moderation_point_v1_reference.py",
            topology_values=(
                (
                    "same_focal_simultaneous",
                    "Two interactions share one focal predictor.",
                ),
                (
                    "different_focal_simultaneous",
                    "Interactions use different focal predictors.",
                ),
            ),
        ),
        "moderation_bootstrap": CellContract(
            key="moderation_bootstrap",
            filename="general_sem_pls_multiple_moderation_bootstrap_v1.qualification.json",
            capability_id="smartpls.moderation",
            cell_id="qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap",
            method_version="general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1",
            analytical_method_version="qpls.general-sem-pls.multiple-two-way.full-model-case-bootstrap.v1",
            execution_kind="stochastic",
            stochastic=True,
            family="moderation",
            source_manifest="validation/methods/general_sem_pls_multiple_moderation_bootstrap_v1.manifest.json",
            method_document="docs/methods/GENERAL_SEM_PLS_MULTIPLE_MODERATION_BOOTSTRAP_V1.md",
            arithmetic_reference="validation/general_sem_pls_multiple_moderation_bootstrap_v1_reference.py",
            topology_values=(
                (
                    "same_focal_simultaneous",
                    "Two interactions share one focal predictor.",
                ),
                (
                    "different_focal_simultaneous",
                    "Interactions use different focal predictors.",
                ),
            ),
        ),
    }
)


SPEC_PATHS: Mapping[str, Path] = MappingProxyType(
    {key: SPEC_DIRECTORY / cell.filename for key, cell in CELL_CONTRACTS.items()}
)


def _axis(
    identifier: str, label: str, values: Sequence[tuple[str, str]]
) -> dict[str, object]:
    return {
        "id": identifier,
        "label": label,
        "values": [
            {"id": value_id, "description": description}
            for value_id, description in values
        ],
    }


def _scenario_axes(cell: CellContract) -> list[dict[str, object]]:
    workload = (
        (
            ("full_refit_all_usable", "All indexed full-model refits are usable."),
            (
                "controlled_failures_retained",
                "Typed failed refits remain in the requested denominator.",
            ),
        )
        if cell.stochastic
        else (
            ("single_fit", "One complete point-estimator invocation."),
            (
                "batched_qualification_trials",
                f"At least {MINIMUM_WORST_CASE_BINOMIAL_TRIALS} frozen indexed trials.",
            ),
        )
    )
    axes = [
        _axis("model_topology", "General SEM topology", cell.topology_values),
        _axis(
            "measurement_model",
            "Measurement model",
            (
                ("all_mode_a", "Every construct is estimated with Mode A."),
                (
                    "mixed_mode_a_b",
                    "At least one supported Mode B block is combined with Mode A blocks.",
                ),
            ),
        ),
        _axis(
            "data_distribution",
            "Data distribution",
            (
                (
                    "gaussian",
                    "Well-conditioned Gaussian latent and measurement disturbances.",
                ),
                ("skewed_heavy_tailed", "Skewed and heavy-tailed finite disturbances."),
            ),
        ),
        _axis(
            "missingness",
            "Missingness",
            (
                ("complete", "No missing observations."),
                (
                    "listwise_mcar_five_percent",
                    "Five percent MCAR rows removed model-wide before fitting.",
                ),
            ),
        ),
        _axis(
            "input_type",
            "Input representation",
            (
                ("raw_rows", "Supported raw continuous row-level observations."),
                (
                    "summary_matrix_rejection",
                    "Summary matrices are rejected as not applicable.",
                ),
            ),
        ),
        _axis("workload", "Execution workload", workload),
        _axis(
            "metamorphism",
            "Mapped scientific invariance",
            (
                ("identity", "Canonical declaration and row order."),
                (
                    "component_declaration_reorder",
                    "Construct declarations are reversed while identifiers are retained.",
                ),
                (
                    "relation_declaration_reorder",
                    "Path and interaction declarations are reversed while identifiers are retained.",
                ),
                (
                    "indicator_declaration_reorder",
                    "Indicator declarations within every block are reversed while identifiers are retained.",
                ),
                (
                    "row_reverse",
                    "Observation order is reversed with exact row mapping.",
                ),
                (
                    "positive_affine_indicators",
                    "Every finite indicator receives a deterministic positive affine transform.",
                ),
            ),
        ),
    ]
    if cell.stochastic:
        axes.append(
            _axis(
                "workers",
                "Worker scheduling",
                (
                    ("one_worker", "One indexed bootstrap worker."),
                    ("two_workers", "Two indexed bootstrap workers."),
                    ("four_workers", "Four indexed bootstrap workers."),
                    (
                        "maximum_available_workers",
                        "The producer records and uses the runtime maximum available worker count.",
                    ),
                ),
            )
        )
    return axes


def _workload(
    rows: int,
    indicators: int,
    constructs: int,
    resamples: int,
) -> dict[str, int]:
    return {
        "rows": rows,
        "indicators": indicators,
        "constructs": constructs,
        "resamples": resamples,
        "groups": 1,
        "candidate_models": 1,
    }


def _complexity_profiles(stochastic: bool) -> list[dict[str, object]]:
    resamples = (199, 5_000, 5_000, 10_000, 10_000) if stochastic else (0, 0, 0, 0, 0)
    definitions = (
        (
            "micro_exact",
            "Small hand-checkable exact-contract model.",
            _workload(80, 15, 5, resamples[0]),
        ),
        (
            "applied",
            "Typical applied General SEM model.",
            _workload(1_000, 40, 10, resamples[1]),
        ),
        (
            "large",
            "Large routine desktop model.",
            _workload(10_000, 80, 20, resamples[2]),
        ),
        (
            "maximum_axis",
            "Declared maximum dimensions, each exercised independently.",
            _workload(100_000, 300, 100, resamples[3]),
        ),
        (
            "compound_stress",
            "Several high dimensions combined for reliability qualification.",
            _workload(25_000, 150, 50, resamples[4]),
        ),
    )
    return [
        {
            "id": identifier,
            "description": description,
            "applicability": "required",
            "not_applicable_reason": None,
            "workload": workload,
        }
        for identifier, description, workload in definitions
    ]


def _selection_values(axes: Sequence[Mapping[str, object]]) -> dict[str, list[str]]:
    return {
        str(axis["id"]): [str(value["id"]) for value in axis["values"]]  # type: ignore[index]
        for axis in axes
    }


def _first_selections(axes: Sequence[Mapping[str, object]]) -> dict[str, list[str]]:
    return {
        str(axis["id"]): [str(axis["values"][0]["id"])]  # type: ignore[index]
        for axis in axes
    }


def _mandatory_combinations(
    axes: Sequence[Mapping[str, object]],
    profiles: Sequence[Mapping[str, object]],
    *,
    stochastic: bool,
) -> list[dict[str, object]]:
    first = _first_selections(axes)
    combinations: list[dict[str, object]] = [
        {
            "id": "applied_pairwise_matrix",
            "profile_id": "applied",
            "coverage": "pairwise",
            "purpose": "Deterministic pairwise covering array over every required scenario-axis value.",
            "stressed_dimensions": [],
            "selections": _selection_values(axes),
        },
        {
            "id": "micro_exact_contract",
            "profile_id": "micro_exact",
            "coverage": "targeted",
            "purpose": "Hand-check iteration, orientation, effects, inference arithmetic, and typed failures.",
            "stressed_dimensions": [],
            "selections": first,
        },
    ]
    combinations.append(
        {
            "id": "mapped_metamorphic_invariance",
            "profile_id": "micro_exact",
            "coverage": "targeted",
            "purpose": "Require mapped equality under component, relation, indicator, row-order, and positive-affine transformations.",
            "stressed_dimensions": [],
            "selections": {
                **first,
                "metamorphism": [
                    "identity",
                    "component_declaration_reorder",
                    "relation_declaration_reorder",
                    "indicator_declaration_reorder",
                    "row_reverse",
                    "positive_affine_indicators",
                ],
            },
        }
    )
    if stochastic:
        combinations.extend(
            [
                {
                    "id": "large_worker_replay",
                    "profile_id": "large",
                    "coverage": "targeted",
                    "purpose": "Compare semantic worker axes one, two, four, and runtime maximum with indexed replay.",
                    "stressed_dimensions": [],
                    "selections": {
                        **first,
                        "workers": [
                            "one_worker",
                            "two_workers",
                            "four_workers",
                            "maximum_available_workers",
                        ],
                    },
                },
                {
                    "id": "micro_seed_replay",
                    "profile_id": "micro_exact",
                    "coverage": "targeted",
                    "purpose": "Repeat the exact seed and indexed bootstrap plan and require byte-equivalent scientific output.",
                    "stressed_dimensions": [],
                    "selections": {**first, "workers": ["one_worker"]},
                },
            ]
        )
    else:
        combinations.append(
            {
                "id": "large_point_replay",
                "profile_id": "large",
                "coverage": "targeted",
                "purpose": "Repeat the deterministic point-primary estimator at the large profile; worker scheduling is not applicable.",
                "stressed_dimensions": [],
                "selections": first,
            }
        )
    by_id = {str(row["id"]): row for row in profiles}
    applied = by_id["applied"]["workload"]
    maximum = by_id["maximum_axis"]["workload"]
    compound = by_id["compound_stress"]["workload"]
    maximum_dimensions = [
        field
        for field in ("rows", "indicators", "constructs", "resamples")
        if maximum[field] > applied[field]  # type: ignore[index]
    ]
    for dimension in maximum_dimensions:
        combinations.append(
            {
                "id": f"maximum_{dimension}",
                "profile_id": "maximum_axis",
                "coverage": "targeted",
                "purpose": f"Exercise the declared maximum {dimension} axis with other dimensions controlled.",
                "stressed_dimensions": [dimension],
                "selections": first,
            }
        )
    compound_dimensions = [
        field
        for field in ("rows", "indicators", "constructs", "resamples")
        if compound[field] > applied[field]  # type: ignore[index]
    ]
    combinations.append(
        {
            "id": "compound_reliability",
            "profile_id": "compound_stress",
            "coverage": "compound",
            "purpose": "Combine high dimensions with cancellation, memory, deterministic retry, and result-size checks.",
            "stressed_dimensions": compound_dimensions,
            "selections": first,
        }
    )
    return combinations


def _estimands(cell: CellContract) -> list[dict[str, object]]:
    measurement = {
        "id": "indicator_level_pls_solution",
        "label": "Indicator-level PLS solution",
        "definition": "Converged Mode A or supported Mode B weights, loadings, oriented scores, and complete joint structural coefficients.",
        "unit": "standardized coefficient or score",
        "output_ids": [
            "outer_weights",
            "outer_loadings",
            "construct_scores",
            "path_coefficients",
        ],
    }
    if cell.family == "mediation":
        method = {
            "id": "mediation_effect_solution",
            "label": "Mediation effects",
            "definition": "Stable specific indirect path products plus source-target direct, total-indirect, and total effects.",
            "unit": "standardized path-product coefficient",
            "output_ids": [
                "specific_indirect_effects",
                "total_indirect_effects",
                "total_effects",
            ],
        }
    else:
        method = {
            "id": "simultaneous_moderation_solution",
            "label": "Simultaneous two-way moderation effects",
            "definition": "Complete joint-stage direct coefficients, standardized-product coefficients, scientific gamma, and fixed minus-one/zero/plus-one slopes.",
            "unit": "standardized coefficient, gamma, or conditional slope",
            "output_ids": [
                "joint_direct_coefficients",
                "standardized_product_coefficients",
                "scientific_gammas",
                "fixed_probe_slopes",
            ],
        }
    estimands = [measurement, method]
    if cell.stochastic:
        estimands.extend(
            [
                {
                    "id": "full_model_case_bootstrap_inference",
                    "label": "Full-model case-bootstrap inference",
                    "definition": "Indexed complete PLS refits with sign alignment, Type-7 intervals, B-minus-one standard errors, plus-one probabilities, and the exact usable gate.",
                    "unit": "coefficient, interval endpoint, standard error, or probability",
                    "output_ids": [
                        "bootstrap_means",
                        "bootstrap_biases",
                        "bootstrap_standard_errors",
                        "percentile_lower_bounds",
                        "percentile_upper_bounds",
                        "plus_one_probabilities",
                    ],
                },
                {
                    "id": "bootstrap_execution_accounting",
                    "label": "Bootstrap execution accounting",
                    "definition": "Requested, usable, and failed counts with ordered typed failures, usable-index identity, and worker-invariant replay.",
                    "unit": "count, categorical reason, digest, or proportion",
                    "output_ids": [
                        "requested_usable_failed_counts",
                        "failure_ledger",
                        "usable_index_digest",
                    ],
                },
            ]
        )
    estimands.append(
        {
            "id": "qualification_recovery",
            "label": "Qualification-scale scientific recovery",
            "definition": "Predeclared sign recovery, continuous bias/RMSE recovery, coverage, null calibration, failure classification, mapped invariance, and replay gates with failed fits retained.",
            "unit": "proportion or standardized coefficient-error unit",
            "output_ids": (
                [
                    "effect_recovery_rate",
                    "effect_recovery_absolute_bias",
                    "effect_recovery_rmse",
                    "empirical_coverage",
                    "null_rejection_rate",
                    "failure_classification_rate",
                    "worker_replay_rate",
                    "seed_replay_rate",
                    "metamorphic_invariance_rate",
                ]
                if cell.stochastic
                else [
                    "effect_recovery_rate",
                    "effect_recovery_absolute_bias",
                    "effect_recovery_rmse",
                    "failure_classification_rate",
                    "metamorphic_invariance_rate",
                ]
            ),
        }
    )
    return estimands


def _preprocessing(cell: CellContract) -> list[dict[str, object]]:
    steps: list[dict[str, object]] = [
        {
            "id": "model_wide_complete_cases",
            "order": 0,
            "operation": "Retain model-wide finite complete rows under the declared listwise policy.",
            "parameters": {
                "policy": "listwise_deletion",
                "failed_rows_retained_in_trial_denominator": True,
            },
            "applies_to": ["raw_continuous_rows"],
        },
        {
            "id": "sample_standardization",
            "order": 1,
            "operation": "Center every used indicator and scale with sample standard deviation denominator n minus one.",
            "parameters": {"ddof": 1, "zero_variance": "error"},
            "applies_to": ["complete_case_indicator_columns"],
        },
        {
            "id": "independent_pls_pm_refit",
            "order": 2,
            "operation": "Fit path-weighted Mode A or supported Mode B PLS-PM and orient every block by its first nonzero outer weight.",
            "parameters": {
                "weighting": "path",
                "mode_a": "covariance",
                "mode_b": "joint_ols",
            },
            "applies_to": ["standardized_indicator_columns"],
        },
        {
            "id": "cell_estimand_projection",
            "order": 3,
            "operation": (
                "Enumerate typed specific paths and aggregate mediation effects."
                if cell.family == "mediation"
                else "Rebuild sample-standardized products and fit complete joint outcome equations before scientific-gamma and fixed-probe projection."
            ),
            "parameters": {"family": cell.family, "isolated_equations": False},
            "applies_to": ["oriented_construct_scores", "complete_structural_solution"],
        },
    ]
    if cell.stochastic:
        steps.append(
            {
                "id": "indexed_full_model_case_bootstrap",
                "order": 4,
                "operation": "For every fixed case-resample index, refit all PLS blocks, align complete score vectors, rebuild estimands, retain typed failures, and summarize only after the 90 percent usable gate.",
                "parameters": {
                    "interval": "type7_percentile_two_sided",
                    "standard_error": "sample_b_minus_one",
                    "probability": "null_centered_plus_one_two_sided",
                    "minimum_usable_fraction": 0.9,
                },
                "applies_to": ["complete_case_sampling_frame"],
            }
        )
    return steps


def _scientific_contract(cell: CellContract) -> dict[str, object]:
    estimands = _estimands(cell)
    estimand_ids = [str(row["id"]) for row in estimands]
    primary_citation = (
        "Wold (1982) and Nitzl, Roldan, and Cepeda (2016), mediation analysis in partial least squares path modeling."
        if cell.family == "mediation"
        else "Wold (1982) and Chin, Marcolin, and Newsted (2003), a partial least squares latent variable modeling approach for measuring interaction effects."
    )
    return {
        "estimands": estimands,
        "preprocessing": _preprocessing(cell),
        "model_predicates": [
            {
                "id": "recursive_general_sem_composite_model",
                "expression": "The structural graph is an identified recursive DAG of supported composite Mode A or Mode B blocks.",
                "on_violation": "error",
                "diagnostic_code": "general_sem.rank0.model.recursive_composites_required",
            },
            {
                "id": "exact_rank0_cell_scope",
                "expression": (
                    "The request contains typed mediation paths without interactions or higher-order constructs."
                    if cell.family == "mediation"
                    else "The request contains one or more strong-hierarchy two-stage two-way interactions on direct-only outcome equations, without mediation estimands."
                ),
                "on_violation": "not_applicable",
                "diagnostic_code": f"general_sem.rank0.model.{cell.key}_scope_required",
            },
        ],
        "data_predicates": [
            {
                "id": "raw_continuous_single_group_rows",
                "expression": "Input is unweighted single-group raw continuous row-level data; summary matrices, transformed lineage, and noncontinuous scales are excluded.",
                "on_violation": "not_applicable",
                "diagnostic_code": "general_sem.rank0.data.raw_continuous_rows_required",
            },
            {
                "id": "finite_complete_case_support",
                "expression": "Listwise filtering retains sufficient finite nonconstant observations for every block and structural equation.",
                "on_violation": "error",
                "diagnostic_code": "general_sem.rank0.data.complete_case_support_required",
            },
        ],
        "oracles": [
            {
                "id": "general_sem_rank0_primary_methods",
                "kind": "primary_literature",
                "citation": primary_citation,
                "locator": cell.method_document,
                "independence_group": "primary_general_sem_pls_methods",
                "runtime_policy": "no_runtime_dependency",
                "implementation": None,
                "covered_estimand_ids": estimand_ids,
            },
            {
                "id": "general_sem_rank0_independent_python_pls_pm",
                "kind": "independent_implementation",
                "citation": "Transparent standard-library indicator-level PLS-PM, effect, moderation, and indexed full-refit bootstrap oracle maintained outside the production estimator.",
                "locator": "validation/general_sem_rank0_independent_pls_oracle.py",
                "independence_group": "general_sem_rank0_python_oracle_v1",
                "runtime_policy": "development_validation_only",
                "implementation": {
                    "name": "General SEM Rank 0 independent Python PLS-PM oracle",
                    "version": "rank0_v1",
                    "maintainer": "QuickPLS validation lane; production-independent implementation",
                },
                "covered_estimand_ids": estimand_ids,
            },
            {
                "id": "general_sem_rank0_independent_csem_base_r",
                "kind": "independent_implementation",
                "citation": "cSEM 0.6.1 independently estimates indicator-level PLS-PM; base R independently recomputes oriented mediation and simultaneous-moderation targets, full-model case bootstraps, product rescaling, and frozen summaries.",
                "locator": "validation/general_sem_rank0_csem_oracle.py",
                "independence_group": "general_sem_rank0_csem_base_r_oracle_v1",
                "runtime_policy": "development_validation_only",
                "implementation": {
                    "name": "General SEM Rank 0 cSEM and base-R oracle",
                    "version": "csem_0_6_1_base_r_v1",
                    "maintainer": "QuickPLS validation lane; production-independent R implementation",
                },
                "covered_estimand_ids": estimand_ids,
            },
            {
                "id": f"{cell.key}_arithmetic_microreference",
                "kind": "hand_calculation",
                "citation": "Existing production-independent arithmetic fixture for the bounded method contract; it does not replace either indicator-level oracle.",
                "locator": cell.arithmetic_reference,
                "independence_group": f"{cell.key}_existing_arithmetic_reference",
                "runtime_policy": "development_validation_only",
                "implementation": None,
                "covered_estimand_ids": [
                    str(row["id"])
                    for row in estimands
                    if row["id"] != "indicator_level_pls_solution"
                ],
            },
        ],
        "oracle_exception": None,
    }


def _comparison(output_id: str, cell: CellContract) -> dict[str, object]:
    mc_intervals = {
        "effect_recovery_rate": FROZEN_THRESHOLDS["recovery_acceptance_interval"],
        "empirical_coverage": FROZEN_THRESHOLDS["coverage_acceptance_interval"],
        "null_rejection_rate": FROZEN_THRESHOLDS["null_rejection_acceptance_interval"],
        "failure_classification_rate": FROZEN_THRESHOLDS[
            "failure_classification_acceptance_interval"
        ],
        "worker_replay_rate": FROZEN_THRESHOLDS["worker_replay_acceptance_interval"],
        "seed_replay_rate": FROZEN_THRESHOLDS["seed_replay_acceptance_interval"],
    }
    if output_id in mc_intervals:
        return {
            "output_id": output_id,
            "rule": "monte_carlo_interval",
            "rationale": "A frozen binomial scientific-recovery proportion requires predeclared interval and precision gates with failed fits retained.",
            "confidence_level": FROZEN_THRESHOLDS["monte_carlo_confidence_level"],
            "maximum_half_width": FROZEN_THRESHOLDS["monte_carlo_maximum_half_width"],
            "acceptance_interval": list(mc_intervals[output_id]),
        }
    if output_id == "metamorphic_invariance_rate":
        return {
            "output_id": output_id,
            "rule": "exact",
            "rationale": "Mapped declaration, row-order, and positive-affine transformations must preserve every compared scientific output.",
        }
    if output_id in {
        "effect_recovery_absolute_bias",
        "effect_recovery_rmse",
    }:
        statistic = (
            "absolute_bias" if output_id == "effect_recovery_absolute_bias" else "rmse"
        )
        threshold_key = f"{cell.family}_recovery_{statistic}_maximum"
        return {
            "output_id": output_id,
            "rule": "bounded_moment",
            "rationale": "Exact per-target n, sum(error), and sum(error squared) are aggregated before applying the preregistered finite-sample recovery bound.",
            "statistic": statistic,
            "maximum": FROZEN_THRESHOLDS[threshold_key],
            "grouping_keys": ["family", "target_id"],
        }
    if output_id in {"outer_weights", "construct_scores"}:
        return {
            "output_id": output_id,
            "rule": "sign_orientation",
            "rationale": "Construct solutions are compared only after the frozen deterministic anchor orientation is applied.",
            "absolute_tolerance": FROZEN_THRESHOLDS["independent_absolute_tolerance"],
            "relative_tolerance": FROZEN_THRESHOLDS["independent_relative_tolerance"],
            "orientation_keys": ["construct_id", "anchor_indicator_id"],
        }
    if output_id in {
        "requested_usable_failed_counts",
        "failure_ledger",
        "usable_index_digest",
    }:
        return {
            "output_id": output_id,
            "rule": "exact",
            "rationale": "Indexed execution identities, counts, and typed failures are categorical deterministic contract outputs.",
        }
    return {
        "output_id": output_id,
        "rule": "abs_relative",
        "rationale": "Independent implementations must agree elementwise within frozen absolute and relative floating-point tolerances.",
        "absolute_tolerance": FROZEN_THRESHOLDS["independent_absolute_tolerance"],
        "relative_tolerance": FROZEN_THRESHOLDS["independent_relative_tolerance"],
    }


def _operational_contract(cell: CellContract) -> dict[str, object]:
    profiles = ("micro_exact", "applied", "large", "maximum_axis", "compound_stress")
    elapsed = (
        (30, 900, 7_200, 27_000, 36_000)
        if cell.stochastic
        else (5, 60, 600, 3_600, 7_200)
    )
    memory = (1, 4, 10, 12, 12) if cell.stochastic else (1, 2, 6, 12, 12)
    result_mib = (
        (50, 500, 2_048, 8_192, 8_192)
        if cell.stochastic
        else (25, 100, 500, 2_048, 4_096)
    )
    phases = []
    for phase in ("validate", "estimate", "resample", "compare", "export"):
        required = phase in {"validate", "estimate", "export"} or (
            phase == "resample" and cell.stochastic
        )
        reason = None
        if not required:
            reason = (
                "This point-estimate cell does not resample."
                if phase == "resample"
                else "Rank 0 estimates one declared model and does not compare candidates."
            )
        phases.append(
            {
                "phase": phase,
                "applicability": "required" if required else "not_applicable",
                "not_applicable_reason": reason,
            }
        )
    return {
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
            "semantic_readback_formats": ["csv", "xlsx", "html", "svg", "pdf", "png"],
            "canonical_projection_id": f"canonical_result_document_v2.{cell.key}",
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
            "maximum_latency_seconds": 1,
            "phases": phases,
            "no_partial_visible_result": True,
            "no_partial_committed_result": True,
            "archive_unchanged": True,
            "same_settings_retry": True,
        },
        "performance": {
            "hardware_classes": [
                {
                    "id": PERFORMANCE_HARDWARE_PROFILE_ID,
                    "os_family": "windows",
                    "architecture": "x86_64",
                    "minimum_logical_cores": 6,
                    "minimum_memory_gib": 16,
                    "notes": "Qualification reference hardware, not a customer minimum-system claim.",
                }
            ],
            "baseline_policy": {
                "warmup_runs": 1,
                "measured_runs": 5,
                "statistic": "median",
                "maximum_runtime_regression_percent": 20,
                "maximum_memory_regression_percent": 20,
            },
            "budgets": [
                {
                    "profile_id": profile,
                    "hardware_class_id": PERFORMANCE_HARDWARE_PROFILE_ID,
                    "maximum_elapsed_seconds": elapsed[index],
                    "maximum_peak_working_set_bytes": memory[index] * 1_073_741_824,
                    "maximum_result_bytes": result_mib[index] * 1_048_576,
                    "maximum_cancellation_latency_seconds": 1,
                }
                for index, profile in enumerate(profiles)
            ],
        },
    }


def build_qualification_spec(cell_key: str) -> dict[str, object]:
    """Build one strict, receipt-free QualificationSpec V2 contract."""

    try:
        cell = CELL_CONTRACTS[cell_key]
    except KeyError as error:
        raise KeyError(f"unknown Rank 0 cell {cell_key!r}") from error
    scientific = _scientific_contract(cell)
    axes = _scenario_axes(cell)
    profiles = _complexity_profiles(cell.stochastic)
    output_ids = [
        str(output_id)
        for estimand in scientific["estimands"]  # type: ignore[index]
        for output_id in estimand["output_ids"]
    ]
    return {
        "schema_version": QUALIFICATION_SPEC_VERSION,
        "identity": {
            "qualification_id": f"{cell.cell_id}.qualification_v2",
            "method_version": cell.method_version,
            "analytical_method_version": cell.analytical_method_version,
            "execution_kind": cell.execution_kind,
            "potentially_long_running": True,
            "spec_frozen_at_utc": SPEC_FROZEN_AT_UTC,
            "capability_cell": {
                "registry_schema_version": 2,
                "capability_id": cell.capability_id,
                "capability_version": cell.method_version,
                "cell_id": cell.cell_id,
            },
        },
        "migration": {
            "source_kind": "qualification_v1_manifest",
            "source_schema_version": 1,
            "source_manifest_path": cell.source_manifest,
            "status": "compatibility_only",
            "unresolved_items": [
                "scientific.independent_python_and_csem_base_r_qualification_scale_comparison_shards_not_run",
                "scientific.qualification_scale_recovery_coverage_null_and_failure_shards_not_run",
                "scientific.independent_review_not_recorded",
                "evidence.current_product_comparison_receipts_not_minted_until_sources_stable",
                "evidence.archive_export_frontend_packaged_and_scale_receipts_pending_other_lanes",
            ],
        },
        "scientific_contract": scientific,
        "scenario_contract": {
            "axes": axes,
            "complexity_profiles": profiles,
            "mandatory_combinations": _mandatory_combinations(
                axes, profiles, stochastic=cell.stochastic
            ),
            "monte_carlo_policy": {
                "confidence_level": FROZEN_THRESHOLDS["monte_carlo_confidence_level"],
                "maximum_half_width": FROZEN_THRESHOLDS[
                    "monte_carlo_maximum_half_width"
                ],
                "failed_fits_in_denominator": True,
                "decision_boundary_trial_policy": {
                    "policy_version": PLAN4B_DECISION_POLICY_VERSION,
                    "confidence_method": "wilson_score_two_sided_v1",
                    "selection_rule": (
                        "fixed_before_plan4b_outcome_review_contiguous_prefix_v1"
                    ),
                    "global_worst_case_trials": (MINIMUM_WORST_CASE_BINOMIAL_TRIALS),
                    "metric_budgets": {
                        metric: dict(policy)
                        for metric, policy in PLAN4B_METRIC_TRIAL_POLICY.items()
                    },
                    "scenario_trial_overrides": dict(PLAN4B_SCENARIO_TRIAL_OVERRIDES),
                },
            },
        },
        "comparison_contract": {
            "outputs": [_comparison(output_id, cell) for output_id in output_ids]
        },
        "operational_contract": _operational_contract(cell),
        "evidence_contract": {
            "required_roles": [
                "method_contract",
                "kernel_execution",
                "oracle_independence",
                "generative_recovery",
                "adversarial_boundaries",
                "archive_persistence",
                "cross_format_export",
                "frontend_contract",
                "packaged_windows_e2e",
                "performance_scale",
            ],
            "receipt_contract": {
                "hash_algorithm": "sha256",
                "identity_fields": [
                    "qualification_id",
                    "capability_id",
                    "cell_id",
                    "method_version",
                    "source_set_sha256",
                    "scenario_set_sha256",
                    "qualification_contract_sha256",
                    "build_fingerprint",
                ],
                "source_descriptors_required": True,
                "hardware_fingerprint_required": True,
                "scenario_set_hash_required": True,
                "payload_contract": {
                    "contract_id": "quickpls.general_sem.rank0.receipt_payload.v1",
                    "schema_version": 1,
                    "schema_path": "validation/qualification_v2/general_sem_rank0_receipt_payload_v1.schema.json",
                    "validator_path": "validation/general_sem_rank0_receipt_payload_v1.py",
                },
            },
            "receipts": [],
        },
    }


def _noise(generator: random.Random, distribution: str) -> float:
    value = generator.gauss(0.0, 1.0)
    if distribution == "gaussian":
        return value
    if distribution != "skewed_heavy_tailed":
        raise ValueError(f"unknown distribution {distribution!r}")
    tail = 3.0 if generator.random() < 0.08 else 1.0
    skew = 0.25 * (generator.expovariate(1.0) - 1.0)
    return tail * value + skew


def _indicator_rows(
    latents: Mapping[str, Sequence[float]],
    modes: Mapping[str, str],
    *,
    distribution: str,
    missingness: str,
    seed: int,
) -> tuple[tuple[Mapping[str, float | None], ...], tuple[BlockSpec, ...]]:
    generator = random.Random(seed)
    construct_ids = list(latents)
    row_count = len(next(iter(latents.values())))
    rows: list[dict[str, float | None]] = [dict() for _ in range(row_count)]
    blocks: list[BlockSpec] = []
    all_indicators: list[str] = []
    for construct in construct_ids:
        indicators = tuple(f"{construct}{index}" for index in (1, 2, 3))
        all_indicators.extend(indicators)
        blocks.append(BlockSpec(construct, indicators, modes[construct]))  # type: ignore[arg-type]
        coefficients = (
            (0.82, 0.74, 0.88) if modes[construct] == "A" else (1.00, 0.55, -0.20)
        )
        noise_scales = (
            (0.42, 0.55, 0.35) if modes[construct] == "A" else (0.55, 0.80, 0.90)
        )
        for row_index, latent in enumerate(latents[construct]):
            for indicator, coefficient, noise_scale in zip(
                indicators, coefficients, noise_scales, strict=True
            ):
                rows[row_index][indicator] = (
                    coefficient * latent + noise_scale * _noise(generator, distribution)
                )
    if missingness == "listwise_mcar_five_percent":
        for row_index in range(0, row_count, 20):
            rows[row_index][all_indicators[(row_index // 20) % len(all_indicators)]] = (
                None
            )
    elif missingness != "complete":
        raise ValueError(f"unknown missingness {missingness!r}")
    return tuple(rows), tuple(blocks)


def _modes(constructs: Sequence[str], measurement_model: str) -> dict[str, str]:
    if measurement_model == "all_mode_a":
        return {construct: "A" for construct in constructs}
    if measurement_model != "mixed_mode_a_b":
        raise ValueError(f"unknown measurement model {measurement_model!r}")
    return {
        construct: "B" if index % 3 == 1 else "A"
        for index, construct in enumerate(constructs)
    }


def make_mediation_scenario(
    topology: str,
    *,
    measurement_model: str = "all_mode_a",
    distribution: str = "gaussian",
    missingness: str = "complete",
    effect_pattern: str = "mixed_sign",
    rows: int = 240,
    seed: int = 20_260_819,
) -> GeneralSemScenario:
    if effect_pattern not in {"mixed_sign", "positive", "broken_stage_null"}:
        raise ValueError(f"unknown effect pattern {effect_pattern!r}")
    generator = random.Random(seed)
    x = standardize([_noise(generator, distribution) for _ in range(rows)])
    m1 = standardize(
        [0.58 * value + 0.78 * _noise(generator, distribution) for value in x]
    )
    if topology == "parallel_mediation":
        x_to_m2 = {
            "mixed_sign": -0.36,
            "positive": 0.36,
            "broken_stage_null": 0.0,
        }[effect_pattern]
        m2 = standardize(
            [x_to_m2 * value + 0.86 * _noise(generator, distribution) for value in x]
        )
        y = standardize(
            [
                0.18 * xv
                + 0.46 * first
                + 0.52 * second
                + 0.72 * _noise(generator, distribution)
                for xv, first, second in zip(x, m1, m2, strict=True)
            ]
        )
        paths = (
            PathSpec("x", "m1"),
            PathSpec("x", "m2"),
            PathSpec("x", "y"),
            PathSpec("m1", "y"),
            PathSpec("m2", "y"),
        )
    elif topology == "serial_mediation":
        m1_to_m2 = 0.0 if effect_pattern == "broken_stage_null" else 0.48
        m2 = standardize(
            [m1_to_m2 * first + 0.82 * _noise(generator, distribution) for first in m1]
        )
        y = standardize(
            [
                0.16 * xv + 0.55 * second + 0.76 * _noise(generator, distribution)
                for xv, second in zip(x, m2, strict=True)
            ]
        )
        paths = (
            PathSpec("x", "m1"),
            PathSpec("m1", "m2"),
            PathSpec("m2", "y"),
            PathSpec("x", "y"),
        )
    elif topology == "mixed_mediation":
        x_to_m2 = {
            "mixed_sign": -0.28,
            "positive": 0.28,
            "broken_stage_null": 0.0,
        }[effect_pattern]
        m2 = standardize(
            [
                x_to_m2 * xv + 0.38 * first + 0.78 * _noise(generator, distribution)
                for xv, first in zip(x, m1, strict=True)
            ]
        )
        y = standardize(
            [
                0.15 * xv
                + 0.32 * first
                + 0.49 * second
                + 0.68 * _noise(generator, distribution)
                for xv, first, second in zip(x, m1, m2, strict=True)
            ]
        )
        paths = (
            PathSpec("x", "m1"),
            PathSpec("x", "m2"),
            PathSpec("m1", "m2"),
            PathSpec("x", "y"),
            PathSpec("m1", "y"),
            PathSpec("m2", "y"),
        )
    else:
        raise ValueError(f"unknown mediation topology {topology!r}")
    latents = {"x": x, "m1": m1, "m2": m2, "y": y}
    scenario_rows, blocks = _indicator_rows(
        latents,
        _modes(tuple(latents), measurement_model),
        distribution=distribution,
        missingness=missingness,
        seed=seed + 101,
    )
    return GeneralSemScenario(
        scenario_id=f"{topology}:{measurement_model}:{distribution}:{missingness}:{effect_pattern}",
        rows=scenario_rows,
        model=PlsModelSpec(blocks, paths),
        source_id="x",
        target_id="y",
    )


def make_moderation_scenario(
    topology: str,
    *,
    measurement_model: str = "all_mode_a",
    distribution: str = "gaussian",
    missingness: str = "complete",
    effect_pattern: str = "mixed_sign",
    rows: int = 260,
    seed: int = 20_260_820,
) -> GeneralSemScenario:
    generator = random.Random(seed)
    common = standardize([_noise(generator, distribution) for _ in range(rows)])
    x = standardize(
        [0.72 * value + 0.70 * _noise(generator, distribution) for value in common]
    )
    w = standardize(
        [0.35 * value + 0.92 * _noise(generator, distribution) for value in common]
    )
    z = standardize(
        [
            0.22 * base + 0.25 * moderator + 0.88 * _noise(generator, distribution)
            for base, moderator in zip(common, w, strict=True)
        ]
    )
    a = standardize(
        [0.18 * value + 0.96 * _noise(generator, distribution) for value in common]
    )
    if effect_pattern == "mixed_sign":
        gamma_one, gamma_two = 0.30, -0.24
    elif effect_pattern == "positive":
        gamma_one, gamma_two = 0.28, 0.20
    elif effect_pattern == "null":
        gamma_one, gamma_two = 0.0, 0.0
    else:
        raise ValueError(f"unknown effect pattern {effect_pattern!r}")
    if topology == "same_focal_simultaneous":
        y = standardize(
            [
                0.27 * xv
                + 0.13 * wv
                - 0.11 * zv
                + gamma_one * xv * wv
                + gamma_two * xv * zv
                + 0.72 * _noise(generator, distribution)
                for xv, wv, zv in zip(x, w, z, strict=True)
            ]
        )
        latents = {"x": x, "w": w, "z": z, "y": y}
        paths = (
            PathSpec("x", "y"),
            PathSpec("w", "y"),
            PathSpec("z", "y"),
        )
        interactions = (
            InteractionSpec("x_by_w", "x", "w", "y"),
            InteractionSpec("x_by_z", "x", "z", "y"),
        )
    elif topology == "different_focal_simultaneous":
        y = standardize(
            [
                0.25 * xv
                + 0.12 * wv
                - 0.18 * av
                + 0.14 * zv
                + gamma_one * xv * wv
                + gamma_two * av * zv
                + 0.70 * _noise(generator, distribution)
                for xv, wv, av, zv in zip(x, w, a, z, strict=True)
            ]
        )
        latents = {"x": x, "w": w, "a": a, "z": z, "y": y}
        paths = (
            PathSpec("x", "y"),
            PathSpec("w", "y"),
            PathSpec("a", "y"),
            PathSpec("z", "y"),
        )
        interactions = (
            InteractionSpec("x_by_w", "x", "w", "y"),
            InteractionSpec("a_by_z", "a", "z", "y"),
        )
    else:
        raise ValueError(f"unknown moderation topology {topology!r}")
    scenario_rows, blocks = _indicator_rows(
        latents,
        _modes(tuple(latents), measurement_model),
        distribution=distribution,
        missingness=missingness,
        seed=seed + 103,
    )
    signs = {
        interactions[0].interaction_id: 0
        if gamma_one == 0
        else (1 if gamma_one > 0 else -1),
        interactions[1].interaction_id: 0
        if gamma_two == 0
        else (1 if gamma_two > 0 else -1),
    }
    return GeneralSemScenario(
        scenario_id=f"{topology}:{measurement_model}:{distribution}:{missingness}:{effect_pattern}",
        rows=scenario_rows,
        model=PlsModelSpec(blocks, paths),
        interactions=interactions,
        expected_gamma_signs=signs,
    )


def run_micro_harness() -> dict[str, object]:
    """Run fast contract checks without claiming qualification-scale evidence."""

    checks: dict[str, bool] = {}
    mediation_counts = {
        "parallel_mediation": 2,
        "serial_mediation": 1,
        "mixed_mediation": 3,
    }
    for index, (topology, expected_count) in enumerate(mediation_counts.items()):
        scenario = make_mediation_scenario(
            topology,
            measurement_model="mixed_mode_a_b"
            if topology == "mixed_mediation"
            else "all_mode_a",
            rows=180,
            seed=20_260_900 + index,
        )
        fit = fit_pls_pm(scenario.rows, scenario.model)
        effects = mediation_effects(
            fit, scenario.model, scenario.source_id or "x", scenario.target_id or "y"
        )
        checks[f"{topology}_converged"] = fit.convergence_change <= 1.0e-10
        checks[f"{topology}_specific_path_inventory"] = (
            sum(key.startswith("specific:") for key in effects) == expected_count
        )
        checks[f"{topology}_finite_effects"] = all(
            math.isfinite(value) for value in effects.values()
        )
    for index, topology in enumerate(
        ("same_focal_simultaneous", "different_focal_simultaneous")
    ):
        scenario = make_moderation_scenario(
            topology,
            measurement_model="mixed_mode_a_b" if index else "all_mode_a",
            rows=220,
            seed=20_260_910 + index,
        )
        fit = fit_pls_pm(scenario.rows, scenario.model)
        moderation = fit_simultaneous_moderation(
            fit, scenario.model, scenario.interactions
        )
        checks[f"{topology}_joint_inventory"] = set(moderation.scientific_gammas) == {
            interaction.interaction_id for interaction in scenario.interactions
        }
        checks[f"{topology}_gamma_signs"] = all(
            (
                0
                if abs(moderation.scientific_gammas[key]) <= 1.0e-12
                else (1 if moderation.scientific_gammas[key] > 0 else -1)
            )
            == expected
            for key, expected in (scenario.expected_gamma_signs or {}).items()
        )
        checks[f"{topology}_product_scales"] = all(
            value > 0 and math.isfinite(value)
            for value in moderation.product_scales.values()
        )
    mediation = make_mediation_scenario("mixed_mediation", rows=150, seed=20_260_920)
    mediation_bootstrap = bootstrap_mediation(
        mediation.rows,
        mediation.model,
        "x",
        "y",
        requested=11,
        seed=91,
    )
    mediation_reordered = bootstrap_mediation(
        mediation.rows,
        mediation.model,
        "x",
        "y",
        requested=11,
        seed=91,
        evaluation_order=tuple(reversed(range(11))),
    )
    checks["mediation_full_model_bootstrap_published"] = mediation_bootstrap.published
    checks["mediation_indexed_schedule_invariant"] = (
        mediation_bootstrap == mediation_reordered
    )
    moderation_scenario = make_moderation_scenario(
        "same_focal_simultaneous", rows=170, seed=20_260_921
    )
    moderation_bootstrap = bootstrap_moderation(
        moderation_scenario.rows,
        moderation_scenario.model,
        moderation_scenario.interactions,
        requested=11,
        seed=92,
    )
    checks["moderation_full_model_bootstrap_published"] = moderation_bootstrap.published
    checks["moderation_gamma_only_inventory"] = set(moderation_bootstrap.summaries) == {
        interaction.interaction_id for interaction in moderation_scenario.interactions
    }
    return {
        "schema_version": 1,
        "report_kind": "general_sem_rank0_scientific_micro_harness",
        "passed": all(checks.values()),
        "qualification_ready": False,
        "sources_stable_required_before_receipts": True,
        "checks": checks,
        "frozen_thresholds": dict(FROZEN_THRESHOLDS),
        "remaining_qualification": {
            "decision_boundary_trial_policy": {
                "policy_version": PLAN4B_DECISION_POLICY_VERSION,
                "metric_budgets": {
                    metric: dict(policy)
                    for metric, policy in PLAN4B_METRIC_TRIAL_POLICY.items()
                },
                "scenario_trial_overrides": dict(PLAN4B_SCENARIO_TRIAL_OVERRIDES),
            },
            "full_pairwise_and_complexity_scenarios_pending": True,
            "external_r_oracle_supplied": True,
            "external_r_qualification_shards_pending": True,
            "current_product_comparison_pending": True,
            "immutable_receipts_pending": True,
        },
    }


def _canonical_json(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group()
    modes.add_argument("--print-spec", choices=tuple(CELL_CONTRACTS))
    modes.add_argument("--list-spec-paths", action="store_true")
    arguments = parser.parse_args(argv)
    if arguments.print_spec:
        print(_canonical_json(build_qualification_spec(arguments.print_spec)), end="")
        return 0
    if arguments.list_spec_paths:
        print(
            _canonical_json(
                {
                    key: path.relative_to(ROOT).as_posix()
                    for key, path in SPEC_PATHS.items()
                }
            ),
            end="",
        )
        return 0
    report = run_micro_harness()
    print(_canonical_json(report), end="")
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
