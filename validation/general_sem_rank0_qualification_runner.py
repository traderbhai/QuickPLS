#!/usr/bin/env python3
"""Resumable process-sharded prequalification for General SEM Rank 0.

This validation-only runner surrounds the frozen independent Python PLS-PM
oracle. It invokes the current Rust product only through a serialized Cargo
producer, never changes the capability Registry, and never mints source-set
hashes or identity receipts.

The deterministic plan covers the frozen pairwise, maximum-axis,
compound-stress, failure-classification, worker-replay, recovery, coverage,
and null-calibration obligations. The supplied cSEM/base-R implementation is
the second independent oracle. Maximum-axis and compound execution are bound
to the production performance lane rather than redundantly interpreted in
Python/R. Final qualification still requires immutable receipts and review.
"""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import math
import os
import shutil
import socket
import subprocess
import sys
import time
import uuid
from concurrent.futures import FIRST_COMPLETED, ProcessPoolExecutor, wait
from dataclasses import replace
from functools import lru_cache
from pathlib import Path
from statistics import NormalDist
from typing import Callable, Iterable, Mapping, Sequence

from general_sem_rank0_independent_pls_oracle import (
    BlockSpec,
    ModelContractError,
    NumericalOracleError,
    OracleError,
    PathSpec,
    PlsModelSpec,
    bootstrap_mediation,
    bootstrap_moderation,
    canonicalize_model,
    fit_pls_pm,
    fit_simultaneous_moderation,
    mediation_effects,
    standardize,
)
from general_sem_rank0_csem_oracle import build_request as build_csem_request
from general_sem_rank0_csem_oracle import run_csem_oracle
from general_sem_rank0_qualification import (
    CELL_CONTRACTS,
    FROZEN_THRESHOLDS,
    GeneralSemScenario,
    MINIMUM_WORST_CASE_BINOMIAL_TRIALS,
    PERFORMANCE_HARDWARE_PROFILE_ID,
    PLAN4B_DECISION_POLICY_VERSION,
    PLAN4B_METRIC_TRIAL_POLICY,
    PLAN4B_SCENARIO_TRIAL_OVERRIDES,
    build_qualification_spec,
    make_mediation_scenario,
    make_moderation_scenario,
)
from general_sem_rank0_receipt_payload_v1 import (
    ReceiptPayloadError,
    canonical_sha256 as receipt_canonical_sha256,
    unified_rank0_source_receipt,
    validate_unified_rank0_source_receipt,
)


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT_ROOT = (
    ROOT / "validation" / "results" / "general_sem_rank0_qualification_v1"
)
PLAN_FILENAME = "plan.json"
PLAN4B_POLICY_FILENAME = "plan4b-continuation-policy.json"
MATRIX_VERSION = "general_sem_rank0_independent_prequalification_v1"
PLAN_KIND = "general_sem_rank0_qualification_plan_v1"
PLAN4B_POLICY_KIND = "general_sem_rank0_plan4b_continuation_policy_v1"
SHARD_KIND = "general_sem_rank0_qualification_shard_v1"
AGGREGATE_KIND = "general_sem_rank0_qualification_aggregate_v1"
PLAN4B_AGGREGATE_KIND = "general_sem_rank0_qualification_aggregate_plan4b_v1"
PLAN4B_SKIPPED_SUITES = frozenset({"pairwise"})
SEED_DERIVATION = "sha256_matrix_scenario_trial_stream_first_u53_v1"
INTEGRITY_SCOPE = "prequalification_integrity_only_not_source_or_identity_receipt"
PRODUCT_REQUEST_KIND = "general_sem_rank0_current_product_request_v1"
PRODUCT_BUNDLE_KIND = "general_sem_rank0_current_product_bundle_v2"
PRODUCT_PRODUCER_CONTRACT = "qpls_runner_production_api_adapter_v1"
PRODUCT_EXECUTION_RECEIPT_KIND = "general_sem_rank0_product_execution_receipt_v2"
PRODUCT_EXECUTION_NONCE_ENV = "QPLS_RANK0_PRODUCT_EXECUTION_NONCE"
PRODUCT_CARGO_PACKAGE = "qpls-runner"
PRODUCT_CARGO_EXAMPLE = "general_sem_rank0_product_comparison"
PRODUCT_INDEX_AUTHORITY = "qpls_resampling_bootstrap_indices_exact_operation_v1"
MAX_SAFE_GENERAL_SEM_SEED = (1 << 53) - 1
DEFAULT_CONCURRENCY = 4
MAX_CONCURRENCY = 32
DEFAULT_SHARD_SIZE = 64
MINIMUM_C_FREE_GIB = 25.0
MINIMUM_D_FREE_GIB = 25.0
RESOURCE_CHECK_INTERVAL = 8
DEFAULT_STALE_CLAIM_SECONDS = 600.0
POPULATION_REFERENCE_ROWS = 20_000
SAFE_ID_CHARACTERS = frozenset(
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._-"
)
TRIAL_SUITES = frozenset(
    {
        "failure_classification",
        "worker_replay",
        "seed_replay",
        "recovery",
        "coverage",
        "null_calibration",
    }
)
DETERMINISTIC_SUITES = frozenset(
    {
        "pairwise",
        "maximum_axis",
        "compound_stress",
        "independent_oracle_comparison",
        "current_product_comparison",
        "metamorphic_invariance",
    }
)
ALL_SUITES = tuple(sorted(TRIAL_SUITES | DETERMINISTIC_SUITES))
EXTERNAL_EXECUTION_SUITES = frozenset(
    {"maximum_axis", "compound_stress", "current_product_comparison"}
)
METRIC_TO_THRESHOLD = {
    "effect_recovery_rate": "recovery_acceptance_interval",
    "empirical_coverage": "coverage_acceptance_interval",
    "null_rejection_rate": "null_rejection_acceptance_interval",
    "failure_classification_rate": "failure_classification_acceptance_interval",
    "worker_replay_rate": "worker_replay_acceptance_interval",
    "seed_replay_rate": "seed_replay_acceptance_interval",
}
METAMORPHISMS = (
    "component_declaration_reorder",
    "relation_declaration_reorder",
    "indicator_declaration_reorder",
    "row_reverse",
    "positive_affine_indicators",
)
POINT_WORKER_AXES = ("not_applicable",)
BOOTSTRAP_WORKER_AXES = ("1", "2", "4", "max")
INDEPENDENT_ORACLE_ABSOLUTE_TOLERANCE = float(
    FROZEN_THRESHOLDS["independent_absolute_tolerance"]
)
INDEPENDENT_ORACLE_RELATIVE_TOLERANCE = float(
    FROZEN_THRESHOLDS["independent_relative_tolerance"]
)


class QualificationRunnerError(RuntimeError):
    """Base class for runner failures that must not be treated as evidence."""


class PlanValidationError(QualificationRunnerError):
    """The deterministic matrix plan is missing, inconsistent, or altered."""


class ArtifactTamperError(QualificationRunnerError):
    """A plan or completed shard differs from its bound integrity contract."""


class ResourceGuardError(QualificationRunnerError):
    """The C or D drive crossed the predeclared stop boundary."""


class ClaimBusyError(QualificationRunnerError):
    """Another live process currently owns a shard claim."""


def canonical_bytes(value: object) -> bytes:
    return (
        json.dumps(
            value,
            sort_keys=True,
            separators=(",", ":"),
            ensure_ascii=False,
            allow_nan=False,
        )
        + "\n"
    ).encode("utf-8")


def canonical_sha256(value: object) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def _is_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _u63(*parts: object) -> int:
    material = "\0".join(str(part) for part in parts).encode("utf-8")
    return int.from_bytes(hashlib.sha256(material).digest()[:8], "big") & (
        (1 << 63) - 1
    )


def _u53(*parts: object) -> int:
    """Return a JavaScript-safe seed accepted by GeneralSemConfigV1."""

    material = "\0".join(str(part) for part in parts).encode("utf-8")
    return int.from_bytes(hashlib.sha256(material).digest()[:8], "big") & (
        (1 << 53) - 1
    )


def trial_seed(seed_base: int, trial_index: int, stream: str = "scenario") -> int:
    return _u53(SEED_DERIVATION, seed_base, trial_index, stream)


def _safe_id(value: str) -> bool:
    return (
        bool(value)
        and value[0].isalnum()
        and all(character in SAFE_ID_CHARACTERS for character in value)
    )


def _profile_workloads(cell_key: str) -> dict[str, dict[str, int]]:
    spec = build_qualification_spec(cell_key)
    return {
        str(profile["id"]): {
            key: int(value) for key, value in profile["workload"].items()
        }
        for profile in spec["scenario_contract"]["complexity_profiles"]
    }


def _axis_values(cell_key: str) -> dict[str, tuple[str, ...]]:
    spec = build_qualification_spec(cell_key)
    return {
        str(axis["id"]): tuple(str(value["id"]) for value in axis["values"])
        for axis in spec["scenario_contract"]["axes"]
    }


def greedy_pairwise_cases(
    axes: Mapping[str, Sequence[str]],
) -> tuple[dict[str, str], ...]:
    """Return a deterministic greedy covering array over every value pair."""

    axis_ids = tuple(axes)
    if len(axis_ids) < 2 or any(not axes[axis_id] for axis_id in axis_ids):
        raise PlanValidationError(
            "pairwise coverage requires at least two nonempty axes"
        )
    candidates = tuple(
        tuple(values)
        for values in itertools.product(*(tuple(axes[axis_id]) for axis_id in axis_ids))
    )
    uncovered: set[tuple[str, str, str, str]] = set()
    for left_index, left_axis in enumerate(axis_ids):
        for right_axis in axis_ids[left_index + 1 :]:
            uncovered.update(
                (left_axis, left_value, right_axis, right_value)
                for left_value in axes[left_axis]
                for right_value in axes[right_axis]
            )
    selected: list[tuple[str, ...]] = []
    while uncovered:
        best: tuple[str, ...] | None = None
        best_covered: set[tuple[str, str, str, str]] = set()
        for candidate in candidates:
            selection = dict(zip(axis_ids, candidate, strict=True))
            covered = {
                (left_axis, selection[left_axis], right_axis, selection[right_axis])
                for left_index, left_axis in enumerate(axis_ids)
                for right_axis in axis_ids[left_index + 1 :]
            } & uncovered
            if len(covered) > len(best_covered) or (
                len(covered) == len(best_covered)
                and covered
                and (best is None or candidate < best)
            ):
                best = candidate
                best_covered = covered
        if best is None or not best_covered:
            raise PlanValidationError("pairwise covering algorithm stalled")
        selected.append(best)
        uncovered.difference_update(best_covered)
    return tuple(dict(zip(axis_ids, candidate, strict=True)) for candidate in selected)


def uncovered_pairs(
    axes: Mapping[str, Sequence[str]], cases: Sequence[Mapping[str, str]]
) -> set[tuple[str, str, str, str]]:
    axis_ids = tuple(axes)
    expected = {
        (left_axis, left_value, right_axis, right_value)
        for left_index, left_axis in enumerate(axis_ids)
        for right_axis in axis_ids[left_index + 1 :]
        for left_value in axes[left_axis]
        for right_value in axes[right_axis]
    }
    covered = {
        (left_axis, case[left_axis], right_axis, case[right_axis])
        for case in cases
        for left_index, left_axis in enumerate(axis_ids)
        for right_axis in axis_ids[left_index + 1 :]
    }
    return expected - covered


def _scenario_definition(
    *,
    scenario_id: str,
    suite: str,
    cell_key: str,
    parameters: Mapping[str, str],
    workload: Mapping[str, int],
    trial_count: int,
    metric: str,
    contract_combination_id: str,
    workload_adjustments: Sequence[str] = (),
) -> dict[str, object]:
    cell = CELL_CONTRACTS[cell_key]
    return {
        "scenario_id": scenario_id,
        "suite": suite,
        "cell_key": cell_key,
        "cell_id": cell.cell_id,
        "method_version": cell.method_version,
        "analytical_method_version": cell.analytical_method_version,
        "family": cell.family,
        "stochastic": cell.stochastic,
        "contract_combination_id": contract_combination_id,
        "parameters": dict(sorted(parameters.items())),
        "workload": {key: int(value) for key, value in sorted(workload.items())},
        "workload_adjustments": list(workload_adjustments),
        "trial_count": int(trial_count),
        "metrics": [metric],
        "seed_base": _u63(MATRIX_VERSION, scenario_id),
        "prequalification_only": True,
    }


def _build_scenarios(
    qualification_trials: int,
    included_suites: frozenset[str],
) -> list[dict[str, object]]:
    scenarios: list[dict[str, object]] = []
    pairwise_axis_ids = (
        "model_topology",
        "measurement_model",
        "data_distribution",
        "missingness",
    )
    for cell_key, cell in CELL_CONTRACTS.items():
        axes = _axis_values(cell_key)
        workloads = _profile_workloads(cell_key)
        base_parameters = {
            "topology": axes["model_topology"][0],
            "measurement_model": axes["measurement_model"][0],
            "distribution": axes["data_distribution"][0],
            "missingness": axes["missingness"][0],
            "effect_pattern": "mixed_sign",
        }
        if "pairwise" in included_suites:
            selected_axes = {axis_id: axes[axis_id] for axis_id in pairwise_axis_ids}
            pairwise = greedy_pairwise_cases(selected_axes)
            if uncovered_pairs(selected_axes, pairwise):
                raise PlanValidationError(
                    "pairwise plan failed to cover every value pair"
                )
            for case_index, case in enumerate(pairwise):
                parameters = {
                    "topology": case["model_topology"],
                    "measurement_model": case["measurement_model"],
                    "distribution": case["data_distribution"],
                    "missingness": case["missingness"],
                    "effect_pattern": "mixed_sign",
                }
                scenarios.append(
                    _scenario_definition(
                        scenario_id=f"pairwise.{cell_key}.{case_index:03d}",
                        suite="pairwise",
                        cell_key=cell_key,
                        parameters=parameters,
                        workload=workloads["applied"],
                        trial_count=1,
                        metric="deterministic_contract_rate",
                        contract_combination_id="applied_pairwise_matrix",
                    )
                )
        challenge_parameters = dict(base_parameters)
        challenge_parameters.update(
            {
                "topology": (
                    "mixed_mediation"
                    if cell.family == "mediation"
                    else "different_focal_simultaneous"
                ),
                "measurement_model": "mixed_mode_a_b",
                "distribution": "skewed_heavy_tailed",
                "missingness": "listwise_mcar_five_percent",
            }
        )
        challenge_workload = {
            "rows": 140,
            "indicators": 12 if cell.family == "mediation" else 15,
            "constructs": 4 if cell.family == "mediation" else 5,
            "resamples": 199 if cell.stochastic else 0,
            "groups": 1,
            "candidate_models": 1,
        }
        if "independent_oracle_comparison" in included_suites:
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"independent_oracle_comparison.{cell_key}",
                    suite="independent_oracle_comparison",
                    cell_key=cell_key,
                    parameters=challenge_parameters,
                    workload=challenge_workload,
                    trial_count=1,
                    metric="deterministic_contract_rate",
                    contract_combination_id="micro_exact_contract",
                )
            )
        if "current_product_comparison" in included_suites:
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"current_product_comparison.{cell_key}",
                    suite="current_product_comparison",
                    cell_key=cell_key,
                    parameters=challenge_parameters,
                    workload=challenge_workload,
                    trial_count=1,
                    metric="deterministic_contract_rate",
                    contract_combination_id="micro_exact_contract",
                )
            )
        if "metamorphic_invariance" in included_suites:
            metamorphic_workload = {
                **workloads["micro_exact"],
                "rows": 140,
                "indicators": 15 if cell.family == "moderation" else 12,
                "constructs": 5 if cell.family == "moderation" else 4,
                "resamples": 0,
            }
            for metamorphism in METAMORPHISMS:
                metamorphic_parameters = dict(base_parameters)
                metamorphic_parameters["metamorphism"] = metamorphism
                scenarios.append(
                    _scenario_definition(
                        scenario_id=(
                            f"metamorphic_invariance.{cell_key}.{metamorphism}"
                        ),
                        suite="metamorphic_invariance",
                        cell_key=cell_key,
                        parameters=metamorphic_parameters,
                        workload=metamorphic_workload,
                        trial_count=1,
                        metric="deterministic_contract_rate",
                        contract_combination_id="mapped_metamorphic_invariance",
                    )
                )
        if "maximum_axis" in included_suites:
            applied = workloads["applied"]
            maximum = workloads["maximum_axis"]
            for dimension in ("rows", "indicators", "constructs", "resamples"):
                if maximum[dimension] <= applied[dimension]:
                    continue
                resolved = dict(applied)
                resolved[dimension] = maximum[dimension]
                adjustments: list[str] = []
                if resolved["indicators"] < resolved["constructs"]:
                    resolved["indicators"] = resolved["constructs"]
                    adjustments.append(
                        "indicators_raised_to_one_per_construct_for_model_coherence"
                    )
                scenarios.append(
                    _scenario_definition(
                        scenario_id=f"maximum_axis.{cell_key}.{dimension}",
                        suite="maximum_axis",
                        cell_key=cell_key,
                        parameters=base_parameters,
                        workload=resolved,
                        trial_count=1,
                        metric="deterministic_contract_rate",
                        contract_combination_id=f"maximum_{dimension}",
                        workload_adjustments=adjustments,
                    )
                )
        if "compound_stress" in included_suites:
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"compound_stress.{cell_key}",
                    suite="compound_stress",
                    cell_key=cell_key,
                    parameters=base_parameters,
                    workload=workloads["compound_stress"],
                    trial_count=1,
                    metric="deterministic_contract_rate",
                    contract_combination_id="compound_reliability",
                )
            )
        micro = dict(workloads["micro_exact"])
        micro.update(
            {
                "rows": 80,
                "indicators": 15 if cell.family == "moderation" else 12,
                "constructs": 5 if cell.family == "moderation" else 4,
                "resamples": 11 if cell.stochastic else 0,
            }
        )
        if "failure_classification" in included_suites:
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"failure_classification.{cell_key}",
                    suite="failure_classification",
                    cell_key=cell_key,
                    parameters=base_parameters,
                    workload=micro,
                    trial_count=qualification_trials,
                    metric="failure_classification_rate",
                    contract_combination_id="micro_exact_contract",
                )
            )
        if "recovery" in included_suites:
            recovery_parameters = dict(base_parameters)
            recovery_parameters["topology"] = (
                "parallel_mediation"
                if cell.family == "mediation"
                else "same_focal_simultaneous"
            )
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"recovery.{cell_key}",
                    suite="recovery",
                    cell_key=cell_key,
                    parameters=recovery_parameters,
                    workload={**micro, "rows": 240},
                    trial_count=qualification_trials,
                    metric="effect_recovery_rate",
                    contract_combination_id="micro_exact_contract",
                )
            )
        if cell.stochastic and "worker_replay" in included_suites:
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"worker_replay.{cell_key}",
                    suite="worker_replay",
                    cell_key=cell_key,
                    parameters=base_parameters,
                    workload={**micro, "rows": 80, "resamples": 11},
                    trial_count=qualification_trials,
                    metric="worker_replay_rate",
                    contract_combination_id="large_worker_replay",
                )
            )
        if cell.stochastic and "seed_replay" in included_suites:
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"seed_replay.{cell_key}",
                    suite="seed_replay",
                    cell_key=cell_key,
                    parameters=base_parameters,
                    workload={**micro, "rows": 80, "resamples": 11},
                    trial_count=qualification_trials,
                    metric="seed_replay_rate",
                    contract_combination_id="micro_seed_replay",
                )
            )
        if cell.stochastic and "coverage" in included_suites:
            coverage_parameters = dict(base_parameters)
            coverage_parameters.update(
                {
                    "topology": (
                        "parallel_mediation"
                        if cell.family == "mediation"
                        else "same_focal_simultaneous"
                    ),
                    "effect_pattern": "positive",
                }
            )
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"coverage.{cell_key}",
                    suite="coverage",
                    cell_key=cell_key,
                    parameters=coverage_parameters,
                    workload={**micro, "rows": 240, "resamples": 199},
                    trial_count=qualification_trials,
                    metric="empirical_coverage",
                    contract_combination_id="micro_exact_contract",
                )
            )
        if cell.stochastic and "null_calibration" in included_suites:
            null_parameters = dict(base_parameters)
            null_parameters.update(
                {
                    "topology": (
                        "parallel_mediation"
                        if cell.family == "mediation"
                        else "same_focal_simultaneous"
                    ),
                    "effect_pattern": (
                        "broken_stage_null" if cell.family == "mediation" else "null"
                    ),
                }
            )
            scenarios.append(
                _scenario_definition(
                    scenario_id=f"null_calibration.{cell_key}",
                    suite="null_calibration",
                    cell_key=cell_key,
                    parameters=null_parameters,
                    workload={**micro, "rows": 240, "resamples": 199},
                    trial_count=qualification_trials,
                    metric="null_rejection_rate",
                    contract_combination_id="micro_exact_contract",
                )
            )
    return sorted(scenarios, key=lambda row: str(row["scenario_id"]))


def build_plan(
    *,
    qualification_trials: int = MINIMUM_WORST_CASE_BINOMIAL_TRIALS,
    shard_size: int = DEFAULT_SHARD_SIZE,
    included_suites: Iterable[str] = ALL_SUITES,
) -> dict[str, object]:
    if qualification_trials < 1:
        raise PlanValidationError("qualification trial count must be positive")
    if shard_size < 1:
        raise PlanValidationError("shard size must be positive")
    suites = frozenset(included_suites)
    unknown = suites - set(ALL_SUITES)
    if unknown or not suites:
        raise PlanValidationError(
            f"unknown or empty suite selection: {sorted(unknown)}"
        )
    scenarios = _build_scenarios(qualification_trials, suites)
    shards: list[dict[str, object]] = []
    ordinal = 0
    for scenario in scenarios:
        scenario_sha256 = canonical_sha256(scenario)
        trial_count = int(scenario["trial_count"])
        effective_size = 1 if scenario["suite"] in DETERMINISTIC_SUITES else shard_size
        for start in range(0, trial_count, effective_size):
            stop = min(start + effective_size, trial_count)
            seed_base = int(scenario["seed_base"])
            shard_id = f"shard-{ordinal:06d}"
            shards.append(
                {
                    "shard_id": shard_id,
                    "scenario_id": scenario["scenario_id"],
                    "scenario_sha256": scenario_sha256,
                    "trial_start_inclusive": start,
                    "trial_stop_exclusive": stop,
                    "first_trial_seed": trial_seed(seed_base, start),
                    "last_trial_seed": trial_seed(seed_base, stop - 1),
                }
            )
            ordinal += 1
    body: dict[str, object] = {
        "schema_version": 1,
        "kind": PLAN_KIND,
        "matrix_version": MATRIX_VERSION,
        "integrity_scope": INTEGRITY_SCOPE,
        "qualification_boundary": {
            "prequalification_only": True,
            "qualification_ready": False,
            "identity_receipts_minted": False,
            "source_hashes_minted": False,
            "registry_promotion_allowed": False,
            "external_r_oracle_supplied": True,
            "external_r_comparison_shards_required": True,
            "current_product_comparison_pending": True,
            "external_performance_receipt_pending": True,
        },
        "frozen_thresholds": {
            key: list(value) if isinstance(value, tuple) else value
            for key, value in FROZEN_THRESHOLDS.items()
        },
        "seed_contract": {
            "derivation": SEED_DERIVATION,
            "trial_ranges": "zero_based_half_open_exact_nonoverlapping",
        },
        "execution_policy": {
            "default_concurrency": DEFAULT_CONCURRENCY,
            "maximum_configurable_concurrency": MAX_CONCURRENCY,
            "default_shard_size": shard_size,
            "minimum_c_free_gib": MINIMUM_C_FREE_GIB,
            "minimum_d_free_gib": MINIMUM_D_FREE_GIB,
            "accepted_shard_overwrite": "forbidden",
            "publication": "exclusive_atomic_hard_link_v1",
        },
        "external_performance_receipt_contract": {
            "execution_owner": "production_performance_lane",
            "index_kind": "general_sem_rank0_method_performance_index",
            "required_hardware_profile_id": PERFORMANCE_HARDWARE_PROFILE_ID,
            "required_profiles": {
                "maximum_axis": [
                    "maximum_rows_100000",
                    "maximum_indicators_300",
                    "maximum_constructs_100",
                    "maximum_resamples_10000",
                ],
                "compound_stress": ["compound_stress"],
            },
            "required_warmup_runs": 1,
            "required_measured_runs": 5,
            "required_receipt_complete": True,
            "ingest_policy": "exact_content_and_descriptor_hash_bound_fail_closed_v1",
        },
        "current_product_comparison_contract": {
            "execution_owner": "serialized_root_cargo_runner",
            "automatic_parallel_cargo_invocation": False,
            "required_independent_oracles": [
                "general_sem_rank0_python_oracle_v1",
                "general_sem_rank0_csem_base_r_oracle_v1",
            ],
            "required_bindings": [
                "cell_id",
                "method_version",
                "analytical_method_version",
                "scenario_sha256",
                "seed",
                "workers",
                "outer_weights_and_loadings",
                "point_paths_and_effects",
                "joint_stage_two_coefficients_and_scientific_gamma",
                "bootstrap_summaries",
                "requested_usable_minimum_and_failed_counts",
                "ordered_failure_ledger",
                "usable_replicate_indices",
                "cryptographic_producer_executable_sha256",
            ],
            "point_required_worker_axes": list(POINT_WORKER_AXES),
            "bootstrap_required_worker_axes": list(BOOTSTRAP_WORKER_AXES),
            "maximum_worker_resolution": "std_thread_available_parallelism_v1",
            "absolute_tolerance": INDEPENDENT_ORACLE_ABSOLUTE_TOLERANCE,
            "relative_tolerance": INDEPENDENT_ORACLE_RELATIVE_TOLERANCE,
            "product_shard_ingest": "exclusive_validated_normalized_bundle_v1",
            "producer_contract_version": PRODUCT_PRODUCER_CONTRACT,
            "cargo_package": PRODUCT_CARGO_PACKAGE,
            "cargo_example": PRODUCT_CARGO_EXAMPLE,
        },
        "included_suites": sorted(suites),
        "qualification_trials": qualification_trials,
        "scenarios": scenarios,
        "shards": shards,
    }
    return {**body, "plan_sha256": canonical_sha256(body)}


def _plan_body(plan: Mapping[str, object]) -> dict[str, object]:
    return {key: value for key, value in plan.items() if key != "plan_sha256"}


def validate_plan(plan: Mapping[str, object]) -> None:
    required = {
        "schema_version",
        "kind",
        "matrix_version",
        "integrity_scope",
        "qualification_boundary",
        "frozen_thresholds",
        "seed_contract",
        "execution_policy",
        "external_performance_receipt_contract",
        "current_product_comparison_contract",
        "included_suites",
        "qualification_trials",
        "scenarios",
        "shards",
        "plan_sha256",
    }
    if set(plan) != required:
        raise PlanValidationError("plan fields differ from the frozen schema")
    if (
        plan["schema_version"] != 1
        or plan["kind"] != PLAN_KIND
        or plan["matrix_version"] != MATRIX_VERSION
        or plan["integrity_scope"] != INTEGRITY_SCOPE
        or plan["plan_sha256"] != canonical_sha256(_plan_body(plan))
    ):
        raise PlanValidationError("plan identity or digest differs")
    expected_thresholds = {
        key: list(value) if isinstance(value, tuple) else value
        for key, value in FROZEN_THRESHOLDS.items()
    }
    if plan["frozen_thresholds"] != expected_thresholds:
        raise PlanValidationError("plan thresholds differ from the frozen contract")
    if plan["seed_contract"] != {
        "derivation": SEED_DERIVATION,
        "trial_ranges": "zero_based_half_open_exact_nonoverlapping",
    }:
        raise PlanValidationError("plan seed contract differs")
    policy = plan["execution_policy"]
    if not isinstance(policy, Mapping) or (
        policy.get("default_concurrency") != DEFAULT_CONCURRENCY
        or policy.get("maximum_configurable_concurrency") != MAX_CONCURRENCY
        or policy.get("accepted_shard_overwrite") != "forbidden"
    ):
        raise PlanValidationError("plan execution policy differs")
    scenarios = plan["scenarios"]
    shards = plan["shards"]
    if not isinstance(scenarios, list) or not isinstance(shards, list):
        raise PlanValidationError("plan scenarios and shards must be arrays")
    scenario_by_id: dict[str, Mapping[str, object]] = {}
    for scenario in scenarios:
        if not isinstance(scenario, Mapping):
            raise PlanValidationError("scenario is not an object")
        expected_scenario_fields = {
            "scenario_id",
            "suite",
            "cell_key",
            "cell_id",
            "method_version",
            "analytical_method_version",
            "family",
            "stochastic",
            "contract_combination_id",
            "parameters",
            "workload",
            "workload_adjustments",
            "trial_count",
            "metrics",
            "seed_base",
            "prequalification_only",
        }
        if set(scenario) != expected_scenario_fields:
            raise PlanValidationError("scenario fields differ from the frozen schema")
        scenario_id = scenario.get("scenario_id")
        if not isinstance(scenario_id, str) or not _safe_id(scenario_id):
            raise PlanValidationError("scenario identifier is unsafe")
        if scenario_id in scenario_by_id:
            raise PlanValidationError("scenario identifiers are not unique")
        if scenario.get("suite") not in plan["included_suites"]:
            raise PlanValidationError("scenario suite is outside the plan")
        cell_key = scenario.get("cell_key")
        contract = CELL_CONTRACTS.get(str(cell_key))
        if (
            contract is None
            or scenario.get("cell_id") != contract.cell_id
            or scenario.get("method_version") != contract.method_version
            or scenario.get("analytical_method_version")
            != contract.analytical_method_version
            or scenario.get("family") != contract.family
            or scenario.get("stochastic") is not contract.stochastic
            or scenario.get("prequalification_only") is not True
        ):
            raise PlanValidationError(
                "scenario capability or analytical identity differs"
            )
        scenario_by_id[scenario_id] = scenario
    shards_by_scenario: dict[str, list[Mapping[str, object]]] = {
        scenario_id: [] for scenario_id in scenario_by_id
    }
    seen_shards: set[str] = set()
    for shard in shards:
        if not isinstance(shard, Mapping):
            raise PlanValidationError("shard is not an object")
        shard_id = shard.get("shard_id")
        scenario_id = shard.get("scenario_id")
        if (
            not isinstance(shard_id, str)
            or not _safe_id(shard_id)
            or shard_id in seen_shards
            or scenario_id not in scenario_by_id
        ):
            raise PlanValidationError("shard identifier or scenario binding differs")
        seen_shards.add(shard_id)
        scenario = scenario_by_id[str(scenario_id)]
        start = shard.get("trial_start_inclusive")
        stop = shard.get("trial_stop_exclusive")
        if (
            type(start) is not int
            or type(stop) is not int
            or not 0 <= start < stop <= int(scenario["trial_count"])
            or shard.get("scenario_sha256") != canonical_sha256(scenario)
            or shard.get("first_trial_seed")
            != trial_seed(int(scenario["seed_base"]), start)
            or shard.get("last_trial_seed")
            != trial_seed(int(scenario["seed_base"]), stop - 1)
            or int(shard.get("first_trial_seed", -1)) > MAX_SAFE_GENERAL_SEM_SEED
            or int(shard.get("last_trial_seed", -1)) > MAX_SAFE_GENERAL_SEM_SEED
        ):
            raise PlanValidationError("shard range, seed, or scenario digest differs")
        shards_by_scenario[str(scenario_id)].append(shard)
    for scenario_id, scenario in scenario_by_id.items():
        ordered = sorted(
            shards_by_scenario[scenario_id],
            key=lambda row: int(row["trial_start_inclusive"]),
        )
        cursor = 0
        for shard in ordered:
            if shard["trial_start_inclusive"] != cursor:
                raise PlanValidationError("shard ranges contain a gap or overlap")
            cursor = int(shard["trial_stop_exclusive"])
        if cursor != int(scenario["trial_count"]):
            raise PlanValidationError("shard ranges do not cover the scenario")


def validate_frozen_full_plan(plan: Mapping[str, object]) -> None:
    """Require the exact default qualification matrix, not a focused test plan."""

    validate_plan(plan)
    if canonical_bytes(plan) != canonical_bytes(build_plan()):
        raise PlanValidationError("plan differs from the exact frozen full matrix")


def _plan4b_policy_body(policy: Mapping[str, object]) -> dict[str, object]:
    return {key: value for key, value in policy.items() if key != "policy_sha256"}


def build_plan4b_policy(
    plan: Mapping[str, object], output_root: Path | None = None
) -> dict[str, object]:
    """Select fixed, whole-shard Plan 4B prefixes from the frozen V1 plan.

    The policy changes no estimator, threshold, seed, scenario, or accepted
    artifact.  It narrows only the number of trials required for metrics whose
    frozen pass regions are far from the p=.50 worst case, and omits the
    prequalification-only pairwise matrix from the Standard completion gate.
    """

    validate_frozen_full_plan(plan)
    scenario_by_id = _scenario_map(plan)
    shards_by_scenario: dict[str, list[Mapping[str, object]]] = {
        scenario_id: [] for scenario_id in scenario_by_id
    }
    for shard in plan["shards"]:
        shards_by_scenario[str(shard["scenario_id"])].append(shard)

    scenario_targets: list[dict[str, object]] = []
    required_ids: set[str] = set()
    for scenario_id in sorted(scenario_by_id):
        scenario = scenario_by_id[scenario_id]
        if scenario["suite"] in PLAN4B_SKIPPED_SUITES:
            continue
        metrics = scenario["metrics"]
        if not isinstance(metrics, list) or len(metrics) != 1:
            raise PlanValidationError("Plan 4B requires one metric per scenario")
        metric = str(metrics[0])
        if scenario["suite"] in TRIAL_SUITES:
            metric_policy = PLAN4B_METRIC_TRIAL_POLICY.get(metric)
            if metric_policy is None:
                raise PlanValidationError(
                    f"Plan 4B metric {metric!r} has no frozen trial policy"
                )
            minimum_trials = int(metric_policy["minimum_trials"])
            target_trials = int(
                PLAN4B_SCENARIO_TRIAL_OVERRIDES.get(
                    scenario_id, metric_policy["execution_target_trials"]
                )
            )
        else:
            minimum_trials = int(scenario["trial_count"])
            target_trials = int(scenario["trial_count"])
        if not minimum_trials <= target_trials <= int(scenario["trial_count"]):
            raise PlanValidationError("Plan 4B trial target is outside the parent plan")

        ordered = sorted(
            shards_by_scenario[scenario_id],
            key=lambda row: int(row["trial_start_inclusive"]),
        )
        selected: list[Mapping[str, object]] = []
        cursor = 0
        for shard in ordered:
            stop = int(shard["trial_stop_exclusive"])
            if stop > target_trials:
                break
            if int(shard["trial_start_inclusive"]) != cursor:
                raise PlanValidationError(
                    "Plan 4B parent shard prefix is not contiguous"
                )
            selected.append(shard)
            cursor = stop
        if cursor != target_trials:
            raise PlanValidationError(
                "Plan 4B target must align to an exact parent shard boundary"
            )
        selected_ids = [str(shard["shard_id"]) for shard in selected]
        required_ids.update(selected_ids)
        scenario_targets.append(
            {
                "scenario_id": scenario_id,
                "scenario_sha256": canonical_sha256(scenario),
                "suite": scenario["suite"],
                "metric": metric,
                "parent_trial_count": int(scenario["trial_count"]),
                "minimum_trials": minimum_trials,
                "required_trial_count": target_trials,
                "required_shard_ids": selected_ids,
            }
        )

    ordered_required_ids = [
        str(shard["shard_id"])
        for shard in plan["shards"]
        if str(shard["shard_id"]) in required_ids
    ]
    excluded_ids = [
        str(shard["shard_id"])
        for shard in plan["shards"]
        if str(shard["shard_id"]) not in required_ids
    ]
    continued_shards: list[dict[str, object]] = []
    if output_root is not None:
        output_root = output_root.resolve()
        parent_shards = _shard_map(plan)
        shard_directory = output_root / "shards"
        if shard_directory.exists():
            for path in sorted(shard_directory.glob("shard-*.json")):
                if path.is_symlink():
                    raise ArtifactTamperError(
                        "Plan 4B cannot continue from a symlinked shard"
                    )
                shard_id = path.stem
                if shard_id not in parent_shards:
                    raise ArtifactTamperError(
                        "Plan 4B found an accepted shard outside the parent plan"
                    )
                if shard_id not in required_ids:
                    # Valid parent-plan artifacts outside the Plan 4B completion
                    # set are harmless cache entries and are deliberately ignored.
                    load_validated_shard(path, plan, parent_shards[shard_id])
                    continue
                artifact = load_validated_shard(path, plan, parent_shards[shard_id])
                payload = path.read_bytes()
                continued_shards.append(
                    {
                        "shard_id": shard_id,
                        "path": f"shards/{path.name}",
                        "size": len(payload),
                        "sha256": hashlib.sha256(payload).hexdigest(),
                        "artifact_sha256": artifact["artifact_sha256"],
                    }
                )
    continued_by_id = {str(row["shard_id"]): row for row in continued_shards}
    continued_shards = [
        continued_by_id[shard_id]
        for shard_id in ordered_required_ids
        if shard_id in continued_by_id
    ]
    pending_ids = [
        shard_id for shard_id in ordered_required_ids if shard_id not in continued_by_id
    ]
    shard_by_id = _shard_map(plan)
    external_pending_count = sum(
        scenario_by_id[str(shard_by_id[shard_id]["scenario_id"])]["suite"]
        in EXTERNAL_EXECUTION_SUITES
        for shard_id in pending_ids
    )
    body: dict[str, object] = {
        "schema_version": 1,
        "kind": PLAN4B_POLICY_KIND,
        "policy_version": PLAN4B_DECISION_POLICY_VERSION,
        "parent_plan_sha256": plan["plan_sha256"],
        "parent_plan_kind": plan["kind"],
        "matrix_version": plan["matrix_version"],
        "confidence_method": "wilson_score_two_sided_v1",
        "confidence_level": FROZEN_THRESHOLDS["monte_carlo_confidence_level"],
        "maximum_half_width": FROZEN_THRESHOLDS["monte_carlo_maximum_half_width"],
        "selection_rule": "fixed_contiguous_parent_shard_prefix_v1",
        "budget_derivation_uses_outcome_values": False,
        "metric_trial_policy": {
            metric: dict(policy)
            for metric, policy in PLAN4B_METRIC_TRIAL_POLICY.items()
        },
        "scenario_trial_overrides": dict(PLAN4B_SCENARIO_TRIAL_OVERRIDES),
        "scenario_targets": scenario_targets,
        "required_shard_ids": ordered_required_ids,
        "excluded_parent_shard_ids": excluded_ids,
        "continued_shards": continued_shards,
        "pending_shard_ids": pending_ids,
        "execution_inventory": {
            "required_shards": len(ordered_required_ids),
            "continued_shards": len(continued_shards),
            "pending_shards": len(pending_ids),
            "pending_internal_shards": len(pending_ids) - external_pending_count,
            "pending_external_shards": external_pending_count,
            "excluded_parent_shards": len(excluded_ids),
        },
    }
    return {**body, "policy_sha256": canonical_sha256(body)}


def validate_plan4b_policy(
    policy: Mapping[str, object],
    plan: Mapping[str, object],
    *,
    output_root: Path | None = None,
) -> None:
    validate_frozen_full_plan(plan)
    required = {
        "schema_version",
        "kind",
        "policy_version",
        "parent_plan_sha256",
        "parent_plan_kind",
        "matrix_version",
        "confidence_method",
        "confidence_level",
        "maximum_half_width",
        "selection_rule",
        "budget_derivation_uses_outcome_values",
        "metric_trial_policy",
        "scenario_trial_overrides",
        "scenario_targets",
        "required_shard_ids",
        "excluded_parent_shard_ids",
        "continued_shards",
        "pending_shard_ids",
        "execution_inventory",
        "policy_sha256",
    }
    if set(policy) != required:
        raise PlanValidationError("Plan 4B policy fields differ")
    if policy.get("policy_sha256") != canonical_sha256(_plan4b_policy_body(policy)):
        raise PlanValidationError("Plan 4B policy digest differs")
    expected = build_plan4b_policy(plan)
    dynamic_fields = {
        "continued_shards",
        "pending_shard_ids",
        "execution_inventory",
        "policy_sha256",
    }
    for key in set(expected) - dynamic_fields:
        if policy.get(key) != expected[key]:
            raise PlanValidationError("Plan 4B policy differs from the frozen contract")

    required_ids = [str(value) for value in policy["required_shard_ids"]]
    continued = policy["continued_shards"]
    pending_ids = policy["pending_shard_ids"]
    if not isinstance(continued, list) or not isinstance(pending_ids, list):
        raise PlanValidationError("Plan 4B continuation partition is invalid")
    continued_ids: list[str] = []
    shard_by_id = _shard_map(plan)
    scenario_by_id = _scenario_map(plan)
    for row in continued:
        if not isinstance(row, Mapping) or set(row) != {
            "shard_id",
            "path",
            "size",
            "sha256",
            "artifact_sha256",
        }:
            raise PlanValidationError("Plan 4B continued shard descriptor differs")
        shard_id = row.get("shard_id")
        if (
            not isinstance(shard_id, str)
            or shard_id in continued_ids
            or shard_id not in shard_by_id
            or row.get("path") != f"shards/{shard_id}.json"
            or type(row.get("size")) is not int
            or int(row["size"]) < 1
            or not _is_sha256(row.get("sha256"))
            or not _is_sha256(row.get("artifact_sha256"))
        ):
            raise PlanValidationError("Plan 4B continued shard identity differs")
        continued_ids.append(shard_id)
        if output_root is not None:
            root = output_root.resolve()
            path = root / "shards" / f"{shard_id}.json"
            if path.is_symlink() or not path.is_file():
                raise ArtifactTamperError("Plan 4B continued shard path is invalid")
            resolved = path.resolve()
            try:
                resolved.relative_to(root)
            except ValueError as error:
                raise ArtifactTamperError(
                    "Plan 4B continued shard leaves its evidence root"
                ) from error
            payload = resolved.read_bytes()
            if (
                len(payload) != row["size"]
                or hashlib.sha256(payload).hexdigest() != row["sha256"]
            ):
                raise ArtifactTamperError("Plan 4B continued shard bytes differ")
            artifact = load_validated_shard(resolved, plan, shard_by_id[shard_id])
            if artifact.get("artifact_sha256") != row["artifact_sha256"]:
                raise ArtifactTamperError(
                    "Plan 4B continued shard artifact identity differs"
                )
    normalized_pending = [str(value) for value in pending_ids]
    if (
        continued_ids
        != [shard_id for shard_id in required_ids if shard_id in set(continued_ids)]
        or normalized_pending
        != [shard_id for shard_id in required_ids if shard_id not in set(continued_ids)]
        or set(continued_ids) & set(normalized_pending)
        or set(continued_ids) | set(normalized_pending) != set(required_ids)
    ):
        raise PlanValidationError("Plan 4B continuation partition differs")
    external_pending = sum(
        scenario_by_id[str(shard_by_id[shard_id]["scenario_id"])]["suite"]
        in EXTERNAL_EXECUTION_SUITES
        for shard_id in normalized_pending
    )
    expected_inventory = {
        "required_shards": len(required_ids),
        "continued_shards": len(continued_ids),
        "pending_shards": len(normalized_pending),
        "pending_internal_shards": len(normalized_pending) - external_pending,
        "pending_external_shards": external_pending,
        "excluded_parent_shards": len(policy["excluded_parent_shard_ids"]),
    }
    if policy.get("execution_inventory") != expected_inventory:
        raise PlanValidationError("Plan 4B execution inventory differs")


def load_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def _free_gib(path: str) -> float:
    return shutil.disk_usage(path).free / (1024.0**3)


def resource_snapshot(
    provider: Callable[[str], float] | None = None,
) -> dict[str, float]:
    source = _free_gib if provider is None else provider
    try:
        return {"C": float(source("C:\\")), "D": float(source("D:\\"))}
    except OSError as error:
        raise ResourceGuardError(
            f"cannot read required drive capacity: {error}"
        ) from error


def enforce_resource_guard(
    provider: Callable[[str], float] | None = None,
) -> dict[str, float]:
    snapshot = resource_snapshot(provider)
    if snapshot["C"] < MINIMUM_C_FREE_GIB or snapshot["D"] < MINIMUM_D_FREE_GIB:
        raise ResourceGuardError(
            "resource guard stopped execution: "
            f"C={snapshot['C']:.2f} GiB (minimum {MINIMUM_C_FREE_GIB:.2f}), "
            f"D={snapshot['D']:.2f} GiB (minimum {MINIMUM_D_FREE_GIB:.2f})"
        )
    return snapshot


def _exclusive_atomic_publish(path: Path, payload: bytes) -> str:
    """Publish bytes atomically without ever replacing an existing target."""

    enforce_resource_guard()
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.parent / f".{path.name}.{os.getpid()}.{uuid.uuid4().hex}.tmp"
    descriptor = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb", closefd=True) as handle:
            handle.write(payload)
            handle.flush()
            os.fsync(handle.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            if path.read_bytes() != payload:
                raise ArtifactTamperError(
                    f"exclusive target already exists with different bytes: {path}"
                )
        return "published" if path.read_bytes() == payload else "invalid"
    finally:
        try:
            temporary.unlink()
        except FileNotFoundError:
            pass


def publish_plan(plan: Mapping[str, object], output_root: Path) -> Path:
    validate_plan(plan)
    path = output_root / PLAN_FILENAME
    payload = canonical_bytes(plan)
    if path.exists():
        existing = load_json(path)
        if not isinstance(existing, Mapping):
            raise ArtifactTamperError("existing plan is not an object")
        try:
            validate_plan(existing)
        except PlanValidationError as error:
            raise ArtifactTamperError(str(error)) from error
        if canonical_bytes(existing) != payload:
            raise ArtifactTamperError(
                "existing accepted plan differs; overwrite refused"
            )
        return path
    _exclusive_atomic_publish(path, payload)
    return path


def publish_plan4b_policy(
    policy: Mapping[str, object], plan: Mapping[str, object], output_root: Path
) -> Path:
    validate_plan4b_policy(policy, plan, output_root=output_root)
    path = output_root / PLAN4B_POLICY_FILENAME
    payload = canonical_bytes(policy)
    if path.exists():
        existing = load_json(path)
        if not isinstance(existing, Mapping):
            raise ArtifactTamperError("existing Plan 4B policy is not an object")
        try:
            validate_plan4b_policy(existing, plan, output_root=output_root)
        except PlanValidationError as error:
            raise ArtifactTamperError(str(error)) from error
        if canonical_bytes(existing) != payload:
            raise ArtifactTamperError(
                "existing Plan 4B policy differs; overwrite refused"
            )
        return path
    _exclusive_atomic_publish(path, payload)
    return path


def _scenario_map(plan: Mapping[str, object]) -> dict[str, Mapping[str, object]]:
    return {
        str(scenario["scenario_id"]): scenario
        for scenario in plan["scenarios"]  # type: ignore[index]
    }


def _shard_map(plan: Mapping[str, object]) -> dict[str, Mapping[str, object]]:
    return {
        str(shard["shard_id"]): shard
        for shard in plan["shards"]  # type: ignore[index]
    }


def _expanded_scenario(
    base: GeneralSemScenario,
    *,
    target_constructs: int,
    target_indicators: int,
    seed: int,
) -> GeneralSemScenario:
    """Expand a frozen base topology with connected zero-effect nuisance blocks."""

    base_constructs = len(base.model.blocks)
    base_indicators = sum(len(block.indicator_ids) for block in base.model.blocks)
    if target_constructs < base_constructs or target_indicators < base_indicators:
        raise ModelContractError("workload is smaller than the frozen base topology")
    if target_indicators < target_constructs:
        raise ModelContractError("every construct requires at least one indicator")
    rows = [dict(row) for row in base.rows]
    blocks = list(base.model.blocks)
    paths = list(base.model.paths)
    target_id = base.target_id or (
        base.interactions[0].outcome_id if base.interactions else "y"
    )
    import random

    generator = random.Random(seed)
    while len(blocks) < target_constructs:
        index = len(blocks) - base_constructs
        construct_id = f"aux{index:03d}"
        indicator_id = f"{construct_id}_1"
        values = standardize([generator.gauss(0.0, 1.0) for _ in rows])
        for row, value in zip(rows, values, strict=True):
            row[indicator_id] = value
        mode = "B" if len(blocks) % 3 == 1 else "A"
        blocks.append(BlockSpec(construct_id, (indicator_id,), mode))
        paths.append(PathSpec(construct_id, target_id))
    current_indicators = sum(len(block.indicator_ids) for block in blocks)
    block_indicators = {
        block.construct_id: list(block.indicator_ids) for block in blocks
    }
    block_modes = {block.construct_id: block.mode for block in blocks}
    block_ids = [block.construct_id for block in blocks]
    extra_index = 0
    while current_indicators < target_indicators:
        construct_id = block_ids[extra_index % len(block_ids)]
        anchor_id = block_indicators[construct_id][0]
        indicator_id = f"{construct_id}_extra_{extra_index:04d}"
        raw = [
            (0.0 if row[anchor_id] is None else float(row[anchor_id]))
            + 0.45 * generator.gauss(0.0, 1.0)
            for row in rows
        ]
        values = standardize(raw)
        for row, value in zip(rows, values, strict=True):
            row[indicator_id] = value
        block_indicators[construct_id].append(indicator_id)
        current_indicators += 1
        extra_index += 1
    expanded_blocks = tuple(
        BlockSpec(
            identifier, tuple(block_indicators[identifier]), block_modes[identifier]
        )
        for identifier in block_ids
    )
    return replace(
        base,
        rows=tuple(rows),
        model=PlsModelSpec(expanded_blocks, tuple(paths)),
    )


def make_runner_scenario(
    scenario: Mapping[str, object], trial_index: int
) -> GeneralSemScenario:
    parameters = scenario["parameters"]
    workload = scenario["workload"]
    if not isinstance(parameters, Mapping) or not isinstance(workload, Mapping):
        raise PlanValidationError("scenario parameters or workload are invalid")
    seed = trial_seed(int(scenario["seed_base"]), trial_index)
    rows = int(workload["rows"])
    common = {
        "measurement_model": str(parameters["measurement_model"]),
        "distribution": str(parameters["distribution"]),
        "missingness": str(parameters["missingness"]),
        "effect_pattern": str(parameters["effect_pattern"]),
        "rows": rows,
        "seed": seed,
    }
    if scenario["family"] == "mediation":
        base = make_mediation_scenario(str(parameters["topology"]), **common)
    else:
        base = make_moderation_scenario(str(parameters["topology"]), **common)
    return _expanded_scenario(
        base,
        target_constructs=int(workload["constructs"]),
        target_indicators=int(workload["indicators"]),
        seed=trial_seed(int(scenario["seed_base"]), trial_index, "expansion"),
    )


def _specific_effects(values: Mapping[str, float]) -> dict[str, float]:
    return {
        identifier: value
        for identifier, value in values.items()
        if identifier.startswith("specific:")
    }


def _point_targets(scenario: GeneralSemScenario, family: str) -> dict[str, float]:
    fit = fit_pls_pm(scenario.rows, scenario.model)
    if family == "mediation":
        return _specific_effects(
            mediation_effects(
                fit,
                scenario.model,
                scenario.source_id or "x",
                scenario.target_id or "y",
            )
        )
    return dict(
        fit_simultaneous_moderation(
            fit, scenario.model, scenario.interactions
        ).scientific_gammas
    )


@lru_cache(maxsize=8)
def population_reference(
    family: str, effect_pattern: str = "positive"
) -> dict[str, float]:
    """Return the deterministic large-sample pseudo-true oracle targets."""

    reference_seed = _u63(
        MATRIX_VERSION, "population_reference", family, effect_pattern
    )
    if family == "mediation":
        base = make_mediation_scenario(
            "parallel_mediation",
            measurement_model="all_mode_a",
            distribution="gaussian",
            missingness="complete",
            effect_pattern=effect_pattern,
            rows=POPULATION_REFERENCE_ROWS,
            seed=reference_seed,
        )
        expanded = _expanded_scenario(
            base,
            target_constructs=4,
            target_indicators=12,
            seed=_u63(reference_seed, "expansion"),
        )
    elif family == "moderation":
        base = make_moderation_scenario(
            "same_focal_simultaneous",
            measurement_model="all_mode_a",
            distribution="gaussian",
            missingness="complete",
            effect_pattern=effect_pattern,
            rows=POPULATION_REFERENCE_ROWS,
            seed=reference_seed,
        )
        expanded = _expanded_scenario(
            base,
            target_constructs=5,
            target_indicators=15,
            seed=_u63(reference_seed, "expansion"),
        )
    else:
        raise PlanValidationError(f"unknown family {family!r}")
    return _point_targets(expanded, family)


def _bootstrap_result(
    definition: Mapping[str, object],
    generated: GeneralSemScenario,
    trial_index: int,
    *,
    index_plan: Sequence[Sequence[int]] | None = None,
):
    workload = definition["workload"]
    if not isinstance(workload, Mapping):
        raise PlanValidationError("scenario workload is invalid")
    requested = int(workload["resamples"])
    seed = trial_seed(int(definition["seed_base"]), trial_index, "bootstrap")
    if definition["family"] == "mediation":
        return bootstrap_mediation(
            generated.rows,
            generated.model,
            generated.source_id or "x",
            generated.target_id or "y",
            requested=requested,
            seed=seed,
            index_plan=index_plan,
        )
    return bootstrap_moderation(
        generated.rows,
        generated.model,
        generated.interactions,
        requested=requested,
        seed=seed,
        index_plan=index_plan,
    )


def _failure_reason_code(reason: object) -> str:
    """Map implementation-specific prose to one scientific failure class."""

    normalized = str(reason).strip().lower()
    patterns = (
        ("did not converge", "estimation_nonconvergence"),
        ("nonconvergence", "estimation_nonconvergence"),
        ("constant indicator", "constant_indicator"),
        ("insufficient", "insufficient_observations"),
        ("rank", "rank_deficient"),
        ("isolated", "isolated_construct"),
        ("indeterminate", "indeterminate_score_sign"),
        ("constant construct", "constant_construct_score"),
        ("constant interaction", "constant_interaction_product"),
        ("numerical", "numerical_failure"),
    )
    for pattern, code in patterns:
        if pattern in normalized:
            return code
    return "unclassified:" + hashlib.sha256(normalized.encode("utf-8")).hexdigest()


def _bootstrap_failure_reason_totals(failures: Iterable[object]) -> dict[str, int]:
    totals: dict[str, int] = {}
    for failure in failures:
        reason = getattr(failure, "reason", failure)
        code = _failure_reason_code(reason)
        totals[code] = totals.get(code, 0) + 1
    return dict(sorted(totals.items()))


def _maximum_absolute_numeric_difference(
    differences: Iterable[Mapping[str, object]],
) -> float:
    maximum = 0.0
    for difference in differences:
        if difference.get("kind") != "number":
            continue
        try:
            magnitude = abs(float(difference["left"]) - float(difference["right"]))
        except (KeyError, TypeError, ValueError):
            continue
        if math.isfinite(magnitude):
            maximum = max(maximum, magnitude)
    return maximum


def _python_normalized_result(
    definition: Mapping[str, object],
    generated: GeneralSemScenario,
    trial_index: int,
    *,
    index_plan: Sequence[Sequence[int]] | None = None,
) -> dict[str, object]:
    model = canonicalize_model(generated.model)
    fit = fit_pls_pm(generated.rows, model)
    weights = {
        block.construct_id: {
            indicator: fit.weights[block.construct_id][index]
            for index, indicator in enumerate(block.indicator_ids)
        }
        for block in model.blocks
    }
    loadings = {
        block.construct_id: {
            indicator: fit.loadings[block.construct_id][index]
            for index, indicator in enumerate(block.indicator_ids)
        }
        for block in model.blocks
    }
    structural = {
        f"{source}->{target}": value
        for (source, target), value in fit.path_coefficients.items()
    }
    if definition["family"] == "mediation":
        point_values: dict[str, object] = {
            "values": mediation_effects(
                fit,
                model,
                generated.source_id or "x",
                generated.target_id or "y",
            ),
            "structural": structural,
        }
    else:
        moderation = fit_simultaneous_moderation(fit, model, generated.interactions)
        point_values = {
            "structural": {
                f"{source}->{target}": value
                for (source, target), value in moderation.direct_coefficients.items()
            },
            "standardized_product_coefficients": dict(
                moderation.standardized_product_coefficients
            ),
            "scientific_gammas": dict(moderation.scientific_gammas),
            "product_means": dict(moderation.product_means),
            "product_scales": dict(moderation.product_scales),
            "fixed_probe_slopes": {
                identifier: list(values)
                for identifier, values in moderation.fixed_probe_slopes.items()
            },
        }
    bootstrap_payload: dict[str, object] | None = None
    if bool(definition["stochastic"]):
        result = _bootstrap_result(
            definition, generated, trial_index, index_plan=index_plan
        )
        bootstrap_payload = {
            "requested": result.requested,
            "usable": result.usable,
            "minimum_usable": result.minimum_usable,
            "published": result.published,
            "summaries": {
                identifier: {
                    "original": summary.original,
                    "mean": summary.mean,
                    "bias": summary.bias,
                    "standard_error": summary.standard_error,
                    "lower": summary.lower,
                    "upper": summary.upper,
                    "exceedances": summary.exceedances,
                    "plus_one_two_sided_probability": summary.plus_one_two_sided_probability,
                }
                for identifier, summary in result.summaries.items()
            },
            "failures": [
                {
                    "replicate_index": failure.replicate_index,
                    "reason_code": _failure_reason_code(failure.reason),
                }
                for failure in result.failures
            ],
            "usable_indices": list(result.usable_indices),
            "sign_corrections": result.sign_corrections,
        }
    return {
        "point": {"weights": weights, "loadings": loadings, "values": point_values},
        "bootstrap": bootstrap_payload,
    }


def _compare_trees(
    left: object,
    right: object,
    *,
    path: str = "$",
    differences: list[dict[str, object]] | None = None,
) -> list[dict[str, object]]:
    rows = [] if differences is None else differences
    if isinstance(left, Mapping) and isinstance(right, Mapping):
        if set(left) != set(right):
            rows.append(
                {
                    "path": path,
                    "kind": "key_set",
                    "left": sorted(map(str, left)),
                    "right": sorted(map(str, right)),
                }
            )
            return rows
        for key in sorted(left, key=str):
            _compare_trees(
                left[key], right[key], path=f"{path}.{key}", differences=rows
            )
        return rows
    left_sequence = isinstance(left, Sequence) and not isinstance(left, (str, bytes))
    right_sequence = isinstance(right, Sequence) and not isinstance(right, (str, bytes))
    if left_sequence and right_sequence:
        if len(left) != len(right):  # type: ignore[arg-type]
            rows.append(
                {
                    "path": path,
                    "kind": "length",
                    "left": len(left),  # type: ignore[arg-type]
                    "right": len(right),  # type: ignore[arg-type]
                }
            )
            return rows
        for index, (left_value, right_value) in enumerate(
            zip(left, right, strict=True)  # type: ignore[arg-type]
        ):
            _compare_trees(
                left_value,
                right_value,
                path=f"{path}[{index}]",
                differences=rows,
            )
        return rows
    if (
        isinstance(left, (int, float))
        and not isinstance(left, bool)
        and isinstance(right, (int, float))
        and not isinstance(right, bool)
    ):
        equal = (
            left == right
            if isinstance(left, int) and isinstance(right, int)
            else math.isclose(
                float(left),
                float(right),
                abs_tol=INDEPENDENT_ORACLE_ABSOLUTE_TOLERANCE,
                rel_tol=INDEPENDENT_ORACLE_RELATIVE_TOLERANCE,
            )
        )
        if not equal:
            rows.append({"path": path, "kind": "number", "left": left, "right": right})
        return rows
    if left != right:
        rows.append({"path": path, "kind": "exact", "left": left, "right": right})
    return rows


def _evaluate_independent_oracle_comparison(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    python_result = _python_normalized_result(definition, generated, trial_index)
    r_normalized, runtime, operation = _r_normalized_result(
        definition, generated, trial_index
    )
    differences = _compare_trees(python_result, r_normalized)
    return {
        "counts": {
            "deterministic_contract_rate": {
                "event_count": int(not differences),
                "eligible_count": 1,
            }
        },
        "observation": {
            "python_oracle": "general_sem_rank0_python_oracle_v1",
            "r_oracle": "general_sem_rank0_csem_base_r_oracle_v1",
            "operation": operation,
            "difference_count": len(differences),
            "maximum_absolute_difference": _maximum_absolute_numeric_difference(
                differences
            ),
            "difference_witnesses": differences[:20],
            "absolute_tolerance": INDEPENDENT_ORACLE_ABSOLUTE_TOLERANCE,
            "relative_tolerance": INDEPENDENT_ORACLE_RELATIVE_TOLERANCE,
            "r_runtime": runtime,
        },
    }


def _point_only_definition(definition: Mapping[str, object]) -> dict[str, object]:
    workload = definition.get("workload")
    if not isinstance(workload, Mapping):
        raise PlanValidationError("point-only workload is invalid")
    return {
        **definition,
        "stochastic": False,
        "workload": {**workload, "resamples": 0},
    }


def _metamorphic_scenario(
    scenario: GeneralSemScenario, metamorphism: str
) -> GeneralSemScenario:
    if metamorphism == "component_declaration_reorder":
        return replace(
            scenario,
            model=PlsModelSpec(
                tuple(reversed(scenario.model.blocks)), scenario.model.paths
            ),
        )
    if metamorphism == "relation_declaration_reorder":
        return replace(
            scenario,
            model=PlsModelSpec(
                scenario.model.blocks, tuple(reversed(scenario.model.paths))
            ),
            interactions=tuple(reversed(scenario.interactions)),
        )
    if metamorphism == "indicator_declaration_reorder":
        return replace(
            scenario,
            model=PlsModelSpec(
                tuple(
                    replace(block, indicator_ids=tuple(reversed(block.indicator_ids)))
                    for block in scenario.model.blocks
                ),
                scenario.model.paths,
            ),
        )
    if metamorphism == "row_reverse":
        return replace(scenario, rows=tuple(reversed(scenario.rows)))
    if metamorphism == "positive_affine_indicators":
        indicators = tuple(
            sorted(
                {
                    indicator
                    for block in scenario.model.blocks
                    for indicator in block.indicator_ids
                },
                key=lambda value: value.encode("utf-8"),
            )
        )
        transforms = {
            indicator: (
                1.25 + 0.05 * (index % 5),
                0.30 * ((index % 5) - 2),
            )
            for index, indicator in enumerate(indicators)
        }
        rows: list[dict[str, float | None]] = []
        for source in scenario.rows:
            row = dict(source)
            for indicator, (scale, offset) in transforms.items():
                value = row.get(indicator)
                if value is not None:
                    row[indicator] = scale * float(value) + offset
            rows.append(row)
        return replace(scenario, rows=tuple(rows))
    raise PlanValidationError(f"unknown metamorphism {metamorphism!r}")


def _evaluate_metamorphic_invariance(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    parameters = definition.get("parameters")
    metamorphism = (
        parameters.get("metamorphism") if isinstance(parameters, Mapping) else None
    )
    if metamorphism not in METAMORPHISMS:
        raise PlanValidationError("metamorphic scenario transformation is invalid")
    transformed = _metamorphic_scenario(generated, str(metamorphism))
    point_definition = _point_only_definition(definition)
    python_baseline = _python_normalized_result(
        point_definition, generated, trial_index
    )
    python_transformed = _python_normalized_result(
        point_definition, transformed, trial_index
    )
    r_baseline, r_runtime_baseline, operation = _r_normalized_result(
        point_definition, generated, trial_index
    )
    r_transformed, r_runtime_transformed, _ = _r_normalized_result(
        point_definition, transformed, trial_index
    )
    differences: list[dict[str, object]] = []
    for comparison, left, right in (
        ("python_mapped_invariance", python_baseline, python_transformed),
        ("r_mapped_invariance", r_baseline, r_transformed),
        ("baseline_python_vs_r", python_baseline, r_baseline),
        ("transformed_python_vs_r", python_transformed, r_transformed),
    ):
        differences.extend(
            {**row, "comparison": comparison} for row in _compare_trees(left, right)
        )
    return {
        "counts": {
            "deterministic_contract_rate": {
                "event_count": int(not differences),
                "eligible_count": 1,
            }
        },
        "observation": {
            "metamorphism": metamorphism,
            "mapping": "stable_construct_indicator_path_interaction_ids_v1",
            "operation": operation,
            "difference_count": len(differences),
            "maximum_absolute_difference": _maximum_absolute_numeric_difference(
                differences
            ),
            "difference_witnesses": differences[:20],
            "r_runtime_baseline": r_runtime_baseline,
            "r_runtime_transformed": r_runtime_transformed,
        },
    }


def _r_normalized_result(
    definition: Mapping[str, object],
    generated: GeneralSemScenario,
    trial_index: int,
    *,
    index_plan: Sequence[Sequence[int]] | None = None,
) -> tuple[dict[str, object], Mapping[str, object], str]:
    operation = (
        f"{definition['family']}_{'bootstrap' if definition['stochastic'] else 'point'}"
    )
    workload = definition["workload"]
    requested = int(workload["resamples"]) if definition["stochastic"] else None  # type: ignore[index]
    bootstrap_seed = trial_seed(int(definition["seed_base"]), trial_index, "bootstrap")
    r_result = run_csem_oracle(
        generated,
        operation,  # type: ignore[arg-type]
        requested=requested,
        seed=bootstrap_seed,
        timeout_seconds=900,
        index_plan=(
            None if index_plan is None else [list(indices) for indices in index_plan]
        ),
    )
    bootstrap_payload = json.loads(json.dumps(r_result["bootstrap"], allow_nan=False))
    if isinstance(bootstrap_payload, dict) and isinstance(
        bootstrap_payload.get("failures"), list
    ):
        bootstrap_payload["failures"] = [
            {
                "replicate_index": failure.get("replicate_index"),
                "reason_code": _failure_reason_code(failure.get("reason")),
            }
            for failure in bootstrap_payload["failures"]
            if isinstance(failure, dict)
        ]
    normalized: dict[str, object] = {
        "point": {
            "weights": r_result["point"]["weights"],
            "loadings": r_result["point"]["loadings"],
            "values": r_result["point"]["values"],
        },
        "bootstrap": bootstrap_payload,
    }
    return normalized, r_result["runtime"], operation


def _evaluate_deterministic(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    targets = _point_targets(generated, str(definition["family"]))
    checks = {
        "target_inventory_nonempty": bool(targets),
        "targets_finite": all(math.isfinite(value) for value in targets.values()),
        "workload_rows_exact": len(generated.rows)
        == int(definition["workload"]["rows"]),  # type: ignore[index]
        "workload_constructs_exact": len(generated.model.blocks)
        == int(definition["workload"]["constructs"]),  # type: ignore[index]
        "workload_indicators_exact": sum(
            len(block.indicator_ids) for block in generated.model.blocks
        )
        == int(definition["workload"]["indicators"]),  # type: ignore[index]
    }
    observation: dict[str, object] = {
        "checks": checks,
        "target_ids": sorted(targets),
    }
    if bool(definition["stochastic"]):
        bootstrap = _bootstrap_result(definition, generated, trial_index)
        checks.update(
            {
                "bootstrap_published": bootstrap.published,
                "bootstrap_target_inventory_exact": set(bootstrap.summaries)
                == set(targets),
                "bootstrap_accounting_exact": bootstrap.usable + len(bootstrap.failures)
                == bootstrap.requested,
            }
        )
        observation["bootstrap"] = {
            "requested": bootstrap.requested,
            "usable": bootstrap.usable,
            "failed": len(bootstrap.failures),
            "minimum_usable": bootstrap.minimum_usable,
            "failure_reason_totals": _bootstrap_failure_reason_totals(
                bootstrap.failures
            ),
        }
    success = all(checks.values())
    return {
        "counts": {
            "deterministic_contract_rate": {
                "event_count": int(success),
                "eligible_count": 1,
            }
        },
        "observation": observation,
    }


def _expected_failure_case(
    definition: Mapping[str, object], trial_index: int
) -> tuple[str, type[OracleError]]:
    return (
        (
            "constant_indicator",
            NumericalOracleError,
        ),
        (
            "all_missing_indicator",
            NumericalOracleError,
        ),
        (
            "cyclic_structural_graph",
            ModelContractError,
        ),
        (
            "duplicate_indicator_assignment",
            ModelContractError,
        ),
    )[trial_index % 4]


def _evaluate_failure_classification(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    case, expected = _expected_failure_case(definition, trial_index)
    rows = [dict(row) for row in generated.rows]
    model = generated.model
    first_indicator = model.blocks[0].indicator_ids[0]
    if case == "constant_indicator":
        for row in rows:
            row[first_indicator] = 1.0
    elif case == "all_missing_indicator":
        for row in rows:
            row[first_indicator] = None
    elif case == "cyclic_structural_graph":
        source = generated.source_id or model.paths[0].source_id
        target = generated.target_id or (
            generated.interactions[0].outcome_id
            if generated.interactions
            else model.paths[0].target_id
        )
        model = PlsModelSpec(model.blocks, (*model.paths, PathSpec(target, source)))
    elif case == "duplicate_indicator_assignment":
        duplicate = replace(
            model.blocks[1],
            indicator_ids=(first_indicator, *model.blocks[1].indicator_ids),
        )
        model = PlsModelSpec(
            (model.blocks[0], duplicate, *model.blocks[2:]), model.paths
        )
    try:
        fit_pls_pm(rows, model)
    except OracleError as error:
        success = isinstance(error, expected)
        observed = type(error).__name__
        message = str(error)
    else:
        success = False
        observed = "no_error"
        message = "invalid scenario was unexpectedly accepted"
    return {
        "counts": {
            "failure_classification_rate": {
                "event_count": int(success),
                "eligible_count": 1,
            }
        },
        "observation": {
            "case": case,
            "expected_error": expected.__name__,
            "observed_error": observed,
            "message": message,
        },
    }


def _evaluate_recovery(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    targets = _point_targets(generated, str(definition["family"]))
    parameters = definition.get("parameters")
    if not isinstance(parameters, Mapping):
        raise PlanValidationError("recovery scenario parameters are absent")
    truth = population_reference(
        str(definition["family"]), str(parameters["effect_pattern"])
    )
    common = sorted(set(targets) & set(truth))
    if not common:
        raise NumericalOracleError("recovery target inventory is empty")
    errors = {
        identifier: targets[identifier] - truth[identifier] for identifier in common
    }
    events = sum(
        (1 if targets[identifier] > 0 else -1 if targets[identifier] < 0 else 0)
        == (1 if truth[identifier] > 0 else -1 if truth[identifier] < 0 else 0)
        for identifier in common
    )
    return {
        "counts": {
            "effect_recovery_rate": {
                "event_count": events,
                "eligible_count": len(common),
            }
        },
        "observation": {
            "target_ids": common,
            "truth": {identifier: truth[identifier] for identifier in common},
            "estimates": {identifier: targets[identifier] for identifier in common},
            "recovery_moments": [
                {
                    "target_id": identifier,
                    "n": 1,
                    "sum_error": errors[identifier],
                    "sum_squared_error": errors[identifier] ** 2,
                }
                for identifier in common
            ],
            "observed_signs": {
                identifier: (
                    1
                    if targets[identifier] > 0
                    else -1
                    if targets[identifier] < 0
                    else 0
                )
                for identifier in common
            },
            "expected_signs": {
                identifier: (
                    1 if truth[identifier] > 0 else -1 if truth[identifier] < 0 else 0
                )
                for identifier in common
            },
        },
    }


def _evaluate_coverage(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    bootstrap = _bootstrap_result(definition, generated, trial_index)
    parameters = definition.get("parameters")
    if not isinstance(parameters, Mapping):
        raise PlanValidationError("coverage scenario parameters are absent")
    truth = population_reference(
        str(definition["family"]), str(parameters["effect_pattern"])
    )
    common = sorted(set(bootstrap.summaries) & set(truth))
    if not bootstrap.published or not common:
        raise NumericalOracleError("coverage bootstrap did not publish bound targets")
    covered = sum(
        bootstrap.summaries[identifier].lower
        <= truth[identifier]
        <= bootstrap.summaries[identifier].upper
        for identifier in common
    )
    return {
        "counts": {
            "empirical_coverage": {
                "event_count": covered,
                "eligible_count": len(common),
            }
        },
        "observation": {
            "target_ids": common,
            "truth": {identifier: truth[identifier] for identifier in common},
            "intervals": {
                identifier: [
                    bootstrap.summaries[identifier].lower,
                    bootstrap.summaries[identifier].upper,
                ]
                for identifier in common
            },
            "requested": bootstrap.requested,
            "usable": bootstrap.usable,
            "failed": len(bootstrap.failures),
            "failure_reason_totals": _bootstrap_failure_reason_totals(
                bootstrap.failures
            ),
        },
    }


def _evaluate_null_calibration(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    bootstrap = _bootstrap_result(definition, generated, trial_index)
    if not bootstrap.published:
        raise NumericalOracleError("null-calibration bootstrap did not publish")
    if definition["family"] == "mediation":
        target_ids = ["specific:x->m2->y"]
    else:
        target_ids = sorted(bootstrap.summaries)
    if any(identifier not in bootstrap.summaries for identifier in target_ids):
        raise NumericalOracleError("null-calibration target inventory differs")
    rejected = sum(
        bootstrap.summaries[identifier].plus_one_two_sided_probability <= 0.05
        for identifier in target_ids
    )
    return {
        "counts": {
            "null_rejection_rate": {
                "event_count": rejected,
                "eligible_count": len(target_ids),
            }
        },
        "observation": {
            "target_ids": target_ids,
            "probabilities": {
                identifier: bootstrap.summaries[
                    identifier
                ].plus_one_two_sided_probability
                for identifier in target_ids
            },
            "requested": bootstrap.requested,
            "usable": bootstrap.usable,
            "failed": len(bootstrap.failures),
            "failure_reason_totals": _bootstrap_failure_reason_totals(
                bootstrap.failures
            ),
        },
    }


def _evaluate_worker_replay(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    workload = definition["workload"]
    if not isinstance(workload, Mapping):
        raise PlanValidationError("worker replay workload is invalid")
    requested = int(workload["resamples"])
    seed = trial_seed(int(definition["seed_base"]), trial_index, "bootstrap")
    order = tuple(reversed(range(requested)))
    if definition["family"] == "mediation":
        first = bootstrap_mediation(
            generated.rows,
            generated.model,
            generated.source_id or "x",
            generated.target_id or "y",
            requested=requested,
            seed=seed,
        )
        second = bootstrap_mediation(
            generated.rows,
            generated.model,
            generated.source_id or "x",
            generated.target_id or "y",
            requested=requested,
            seed=seed,
            evaluation_order=order,
        )
    else:
        first = bootstrap_moderation(
            generated.rows,
            generated.model,
            generated.interactions,
            requested=requested,
            seed=seed,
        )
        second = bootstrap_moderation(
            generated.rows,
            generated.model,
            generated.interactions,
            requested=requested,
            seed=seed,
            evaluation_order=order,
        )
    equal = first == second and first.published and second.published
    return {
        "counts": {
            "worker_replay_rate": {
                "event_count": int(equal),
                "eligible_count": 1,
            }
        },
        "observation": {
            "equal": equal,
            "requested": requested,
            "usable": first.usable,
            "failed": len(first.failures),
            "failure_reason_totals": _bootstrap_failure_reason_totals(first.failures),
        },
    }


def _evaluate_seed_replay(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    generated = make_runner_scenario(definition, trial_index)
    first = _bootstrap_result(definition, generated, trial_index)
    second = _bootstrap_result(definition, generated, trial_index)
    equal = first == second and first.published and second.published

    def digest(result: object) -> str:
        return hashlib.sha256(repr(result).encode("utf-8")).hexdigest()

    return {
        "counts": {
            "seed_replay_rate": {
                "event_count": int(equal),
                "eligible_count": 1,
            }
        },
        "observation": {
            "equal": equal,
            "replay_kind": "exact_seed_and_index_plan",
            "requested": first.requested,
            "usable": first.usable,
            "failed": len(first.failures),
            "failure_reason_totals": _bootstrap_failure_reason_totals(first.failures),
            "first_result_sha256": digest(first),
            "second_result_sha256": digest(second),
        },
    }


def evaluate_trial(
    definition: Mapping[str, object], trial_index: int
) -> dict[str, object]:
    suite = str(definition["suite"])
    if suite == "independent_oracle_comparison":
        return _evaluate_independent_oracle_comparison(definition, trial_index)
    if suite == "current_product_comparison":
        raise PlanValidationError(
            "current-product comparison requires a serialized normalized Cargo bundle"
        )
    if suite == "metamorphic_invariance":
        return _evaluate_metamorphic_invariance(definition, trial_index)
    if suite in DETERMINISTIC_SUITES:
        return _evaluate_deterministic(definition, trial_index)
    if suite == "failure_classification":
        return _evaluate_failure_classification(definition, trial_index)
    if suite == "recovery":
        return _evaluate_recovery(definition, trial_index)
    if suite == "coverage":
        return _evaluate_coverage(definition, trial_index)
    if suite == "null_calibration":
        return _evaluate_null_calibration(definition, trial_index)
    if suite == "worker_replay":
        return _evaluate_worker_replay(definition, trial_index)
    if suite == "seed_replay":
        return _evaluate_seed_replay(definition, trial_index)
    raise PlanValidationError(f"unknown suite {suite!r}")


def _failure_denominator(definition: Mapping[str, object]) -> int:
    suite = definition["suite"]
    family = definition["family"]
    if suite in {"recovery", "coverage"}:
        return 2
    if suite == "null_calibration":
        return 1 if family == "mediation" else 2
    return 1


def _recovery_target_ids(family: str) -> tuple[str, str]:
    if family == "mediation":
        return ("specific:x->m1->y", "specific:x->m2->y")
    if family == "moderation":
        return ("x_by_w", "x_by_z")
    raise PlanValidationError(f"unknown recovery family {family!r}")


def build_shard_artifact(
    plan: Mapping[str, object], shard: Mapping[str, object]
) -> dict[str, object]:
    validate_plan(plan)
    scenarios = _scenario_map(plan)
    scenario = scenarios[str(shard["scenario_id"])]
    metrics = [str(metric) for metric in scenario["metrics"]]
    counts = {metric: {"event_count": 0, "eligible_count": 0} for metric in metrics}
    ordered_trial_digests: list[str] = []
    first_observation: object | None = None
    last_observation: object | None = None
    unexpected_failures: list[dict[str, object]] = []
    unexpected_failure_count = 0
    unexpected_error_totals: dict[str, int] = {}
    classified_failure_totals: dict[tuple[str, str, str], int] = {}
    bootstrap_failure_totals: dict[str, int] = {}
    comparison_observation_count = 0
    total_difference_count = 0
    maximum_absolute_difference = 0.0
    worker_ordering_comparison_count = 0
    worker_ordering_equal_count = 0
    bootstrap_observation_count = 0
    bootstrap_requested_total = 0
    bootstrap_usable_total = 0
    bootstrap_failed_total = 0
    recovery_moment_totals = {
        target_id: {
            "target_id": target_id,
            "n": 0,
            "failed_count": 0,
            "sum_error": 0.0,
            "sum_squared_error": 0.0,
        }
        for target_id in (
            _recovery_target_ids(str(scenario["family"]))
            if scenario["suite"] == "recovery"
            else ()
        )
    }
    start = int(shard["trial_start_inclusive"])
    stop = int(shard["trial_stop_exclusive"])
    for offset, trial_index in enumerate(range(start, stop)):
        if offset % RESOURCE_CHECK_INTERVAL == 0:
            enforce_resource_guard()
        try:
            result = evaluate_trial(scenario, trial_index)
        except Exception as error:  # fail closed while retaining the denominator
            unexpected_failure_count += 1
            denominator = _failure_denominator(scenario)
            result = {
                "counts": {
                    metric: {"event_count": 0, "eligible_count": denominator}
                    for metric in metrics
                },
                "observation": {
                    "unexpected_error_type": type(error).__name__,
                    "unexpected_error_message": str(error),
                },
            }
            if len(unexpected_failures) < 20:
                unexpected_failures.append(
                    {
                        "trial_index": trial_index,
                        "error_type": type(error).__name__,
                        "message": str(error),
                    }
                )
        result_counts = result["counts"]
        if not isinstance(result_counts, Mapping) or set(result_counts) != set(metrics):
            raise PlanValidationError(
                "trial metric inventory differs from the scenario"
            )
        for metric in metrics:
            row = result_counts[metric]
            if not isinstance(row, Mapping):
                raise PlanValidationError("trial count row is invalid")
            counts[metric]["event_count"] += int(row["event_count"])
            counts[metric]["eligible_count"] += int(row["eligible_count"])
        observation = result["observation"]
        if isinstance(observation, Mapping):
            unexpected_type = observation.get("unexpected_error_type")
            if isinstance(unexpected_type, str):
                unexpected_error_totals[unexpected_type] = (
                    unexpected_error_totals.get(unexpected_type, 0) + 1
                )
            if scenario["suite"] == "failure_classification":
                classification = (
                    str(observation.get("case")),
                    str(observation.get("expected_error")),
                    str(observation.get("observed_error")),
                )
                classified_failure_totals[classification] = (
                    classified_failure_totals.get(classification, 0) + 1
                )
            if scenario["suite"] == "recovery":
                moment_rows = observation.get("recovery_moments")
                if isinstance(moment_rows, list):
                    observed_targets: set[str] = set()
                    for moment in moment_rows:
                        if not isinstance(moment, Mapping):
                            raise PlanValidationError("recovery moment row is invalid")
                        target_id = moment.get("target_id")
                        if (
                            target_id not in recovery_moment_totals
                            or target_id in observed_targets
                            or moment.get("n") != 1
                        ):
                            raise PlanValidationError(
                                "recovery moment target inventory differs"
                            )
                        error_sum = moment.get("sum_error")
                        squared_sum = moment.get("sum_squared_error")
                        if (
                            not isinstance(error_sum, (int, float))
                            or not isinstance(squared_sum, (int, float))
                            or not math.isfinite(float(error_sum))
                            or not math.isfinite(float(squared_sum))
                            or squared_sum < 0
                        ):
                            raise PlanValidationError(
                                "recovery moment values are invalid"
                            )
                        observed_targets.add(str(target_id))
                        accumulator = recovery_moment_totals[str(target_id)]
                        accumulator["n"] += 1
                        accumulator["sum_error"] += float(error_sum)
                        accumulator["sum_squared_error"] += float(squared_sum)
                    if observed_targets != set(recovery_moment_totals):
                        raise PlanValidationError(
                            "recovery moment target inventory is incomplete"
                        )
                elif isinstance(observation.get("unexpected_error_type"), str):
                    for accumulator in recovery_moment_totals.values():
                        accumulator["failed_count"] += 1
                else:
                    raise PlanValidationError("recovery observation omits moments")
            difference_count = observation.get("difference_count")
            if type(difference_count) is int and difference_count >= 0:
                comparison_observation_count += 1
                total_difference_count += difference_count
                maximum_absolute_difference = max(
                    maximum_absolute_difference,
                    float(observation.get("maximum_absolute_difference", 0.0)),
                )
            ordering_equal = observation.get("equal")
            if isinstance(ordering_equal, bool):
                worker_ordering_comparison_count += 1
                worker_ordering_equal_count += int(ordering_equal)
            bootstrap_observation = observation.get("bootstrap")
            if not isinstance(bootstrap_observation, Mapping) and all(
                field in observation for field in ("requested", "usable", "failed")
            ):
                bootstrap_observation = observation
            if isinstance(bootstrap_observation, Mapping):
                requested = bootstrap_observation.get("requested")
                usable = bootstrap_observation.get("usable")
                failed = bootstrap_observation.get("failed")
                if all(
                    type(value) is int and value >= 0
                    for value in (requested, usable, failed)
                ):
                    bootstrap_observation_count += 1
                    bootstrap_requested_total += int(requested)
                    bootstrap_usable_total += int(usable)
                    bootstrap_failed_total += int(failed)
                reasons = bootstrap_observation.get("failure_reason_totals")
                if isinstance(reasons, Mapping):
                    for reason, count in reasons.items():
                        if (
                            isinstance(reason, str)
                            and type(count) is int
                            and count >= 0
                        ):
                            bootstrap_failure_totals[reason] = (
                                bootstrap_failure_totals.get(reason, 0) + count
                            )
        if first_observation is None:
            first_observation = observation
        last_observation = observation
        ordered_trial_digests.append(
            canonical_sha256(
                {
                    "trial_index": trial_index,
                    "trial_seed": trial_seed(int(scenario["seed_base"]), trial_index),
                    "result": result,
                }
            )
        )
    body: dict[str, object] = {
        "schema_version": 1,
        "kind": SHARD_KIND,
        "matrix_version": MATRIX_VERSION,
        "integrity_scope": INTEGRITY_SCOPE,
        "status": "complete",
        "qualification_ready": False,
        "identity_receipt": None,
        "source_set_sha256": None,
        "plan_sha256": plan["plan_sha256"],
        "shard_id": shard["shard_id"],
        "scenario_id": scenario["scenario_id"],
        "scenario_sha256": shard["scenario_sha256"],
        "trial_start_inclusive": start,
        "trial_stop_exclusive": stop,
        "first_trial_seed": shard["first_trial_seed"],
        "last_trial_seed": shard["last_trial_seed"],
        "attempted_trials": stop - start,
        "metric_counts": counts,
        "unexpected_failure_count": unexpected_failure_count,
        "unexpected_failure_witnesses": unexpected_failures,
        "typed_failure_totals": {
            "unexpected_errors": dict(sorted(unexpected_error_totals.items())),
            "classified_failures": [
                {
                    "case": case,
                    "expected_error": expected,
                    "observed_error": observed,
                    "count": count,
                }
                for (case, expected, observed), count in sorted(
                    classified_failure_totals.items()
                )
            ],
            "bootstrap_failures": dict(sorted(bootstrap_failure_totals.items())),
        },
        "comparison_summary": {
            "observation_count": comparison_observation_count,
            "difference_count": total_difference_count,
            "maximum_absolute_difference": maximum_absolute_difference,
        },
        "worker_ordering_summary": {
            "comparison_count": worker_ordering_comparison_count,
            "equal_count": worker_ordering_equal_count,
            "unequal_count": worker_ordering_comparison_count
            - worker_ordering_equal_count,
        },
        "bootstrap_totals": {
            "observation_count": bootstrap_observation_count,
            "requested": bootstrap_requested_total,
            "usable": bootstrap_usable_total,
            "failed": bootstrap_failed_total,
        },
        "recovery_moments": [
            dict(recovery_moment_totals[target_id])
            for target_id in sorted(recovery_moment_totals)
        ],
        "first_observation": first_observation,
        "last_observation": last_observation,
        "ordered_trial_results_sha256": canonical_sha256(ordered_trial_digests),
    }
    return {**body, "artifact_sha256": canonical_sha256(body)}


def _artifact_body(artifact: Mapping[str, object]) -> dict[str, object]:
    return {key: value for key, value in artifact.items() if key != "artifact_sha256"}


def validate_shard_artifact(
    artifact: Mapping[str, object],
    plan: Mapping[str, object],
    shard: Mapping[str, object],
) -> None:
    required = {
        "schema_version",
        "kind",
        "matrix_version",
        "integrity_scope",
        "status",
        "qualification_ready",
        "identity_receipt",
        "source_set_sha256",
        "plan_sha256",
        "shard_id",
        "scenario_id",
        "scenario_sha256",
        "trial_start_inclusive",
        "trial_stop_exclusive",
        "first_trial_seed",
        "last_trial_seed",
        "attempted_trials",
        "metric_counts",
        "unexpected_failure_count",
        "unexpected_failure_witnesses",
        "typed_failure_totals",
        "comparison_summary",
        "worker_ordering_summary",
        "bootstrap_totals",
        "recovery_moments",
        "first_observation",
        "last_observation",
        "ordered_trial_results_sha256",
        "artifact_sha256",
    }
    if set(artifact) != required:
        raise ArtifactTamperError("shard fields differ from the frozen schema")
    scenario = _scenario_map(plan)[str(shard["scenario_id"])]
    if (
        artifact["schema_version"] != 1
        or artifact["kind"] != SHARD_KIND
        or artifact["matrix_version"] != MATRIX_VERSION
        or artifact["integrity_scope"] != INTEGRITY_SCOPE
        or artifact["status"] != "complete"
        or artifact["qualification_ready"] is not False
        or artifact["identity_receipt"] is not None
        or artifact["source_set_sha256"] is not None
        or artifact["plan_sha256"] != plan["plan_sha256"]
        or artifact["shard_id"] != shard["shard_id"]
        or artifact["scenario_id"] != shard["scenario_id"]
        or artifact["scenario_sha256"] != shard["scenario_sha256"]
        or artifact["trial_start_inclusive"] != shard["trial_start_inclusive"]
        or artifact["trial_stop_exclusive"] != shard["trial_stop_exclusive"]
        or artifact["first_trial_seed"] != shard["first_trial_seed"]
        or artifact["last_trial_seed"] != shard["last_trial_seed"]
        or artifact["attempted_trials"]
        != int(shard["trial_stop_exclusive"]) - int(shard["trial_start_inclusive"])
        or artifact["artifact_sha256"] != canonical_sha256(_artifact_body(artifact))
    ):
        raise ArtifactTamperError("shard identity, range, or digest differs")
    digest = artifact["ordered_trial_results_sha256"]
    if not isinstance(digest, str) or len(digest) != 64:
        raise ArtifactTamperError("shard ordered-trial digest is invalid")
    counts = artifact["metric_counts"]
    expected_metrics = set(scenario["metrics"])
    if not isinstance(counts, Mapping) or set(counts) != expected_metrics:
        raise ArtifactTamperError("shard metric inventory differs")
    for metric, row in counts.items():
        if (
            not isinstance(metric, str)
            or not isinstance(row, Mapping)
            or set(row)
            != {
                "event_count",
                "eligible_count",
            }
        ):
            raise ArtifactTamperError("shard metric count row is invalid")
        events = row["event_count"]
        eligible = row["eligible_count"]
        if (
            type(events) is not int
            or type(eligible) is not int
            or not 0 <= events <= eligible
            or eligible < int(artifact["attempted_trials"])
        ):
            raise ArtifactTamperError("shard metric counts are incoherent")
    failures = artifact["unexpected_failure_count"]
    witnesses = artifact["unexpected_failure_witnesses"]
    if (
        type(failures) is not int
        or not 0 <= failures <= int(artifact["attempted_trials"])
        or not isinstance(witnesses, list)
        or len(witnesses) > min(failures, 20)
    ):
        raise ArtifactTamperError("shard unexpected-failure accounting differs")
    typed = artifact.get("typed_failure_totals")
    comparison = artifact.get("comparison_summary")
    ordering = artifact.get("worker_ordering_summary")
    bootstrap_totals = artifact.get("bootstrap_totals")
    recovery_moments = artifact.get("recovery_moments")
    if (
        not isinstance(typed, Mapping)
        or set(typed)
        != {"unexpected_errors", "classified_failures", "bootstrap_failures"}
        or not isinstance(typed.get("unexpected_errors"), Mapping)
        or not isinstance(typed.get("classified_failures"), list)
        or not isinstance(typed.get("bootstrap_failures"), Mapping)
        or not isinstance(comparison, Mapping)
        or set(comparison)
        != {"observation_count", "difference_count", "maximum_absolute_difference"}
        or not isinstance(ordering, Mapping)
        or set(ordering) != {"comparison_count", "equal_count", "unequal_count"}
        or not isinstance(bootstrap_totals, Mapping)
        or set(bootstrap_totals)
        != {"observation_count", "requested", "usable", "failed"}
        or not isinstance(recovery_moments, list)
    ):
        raise ArtifactTamperError("shard compact summary fields differ")
    unexpected_total = 0
    for error_type, count in typed["unexpected_errors"].items():
        if not isinstance(error_type, str) or type(count) is not int or count < 0:
            raise ArtifactTamperError("shard unexpected error totals differ")
        unexpected_total += count
    classified_total = 0
    prior_classification: tuple[str, str, str] | None = None
    for row in typed["classified_failures"]:
        if not isinstance(row, Mapping) or set(row) != {
            "case",
            "expected_error",
            "observed_error",
            "count",
        }:
            raise ArtifactTamperError("shard classified failure totals differ")
        identity = (
            str(row["case"]),
            str(row["expected_error"]),
            str(row["observed_error"]),
        )
        if (
            prior_classification is not None
            and identity <= prior_classification
            or type(row["count"]) is not int
            or row["count"] < 0
        ):
            raise ArtifactTamperError("shard classified failure ordering differs")
        prior_classification = identity
        classified_total += row["count"]
    bootstrap_failure_total = 0
    for reason, count in typed["bootstrap_failures"].items():
        if not isinstance(reason, str) or type(count) is not int or count < 0:
            raise ArtifactTamperError("shard bootstrap failure totals differ")
        bootstrap_failure_total += count
    attempted = int(artifact["attempted_trials"])
    comparison_values = (
        comparison["observation_count"],
        comparison["difference_count"],
        ordering["comparison_count"],
        ordering["equal_count"],
        ordering["unequal_count"],
        bootstrap_totals["observation_count"],
        bootstrap_totals["requested"],
        bootstrap_totals["usable"],
        bootstrap_totals["failed"],
    )
    maximum_difference = comparison["maximum_absolute_difference"]
    if (
        any(type(value) is not int or value < 0 for value in comparison_values)
        or not isinstance(maximum_difference, (int, float))
        or not math.isfinite(float(maximum_difference))
        or maximum_difference < 0
        or ordering["equal_count"] + ordering["unequal_count"]
        != ordering["comparison_count"]
        or bootstrap_totals["usable"] + bootstrap_totals["failed"]
        != bootstrap_totals["requested"]
        or bootstrap_failure_total != bootstrap_totals["failed"]
        or unexpected_total != failures
        or (
            scenario["suite"] == "failure_classification"
            and classified_total + failures != attempted
        )
        or (scenario["suite"] != "failure_classification" and classified_total != 0)
    ):
        raise ArtifactTamperError("shard compact summary accounting differs")
    expected_recovery_targets = (
        _recovery_target_ids(str(scenario["family"]))
        if scenario["suite"] == "recovery"
        else ()
    )
    if [
        row.get("target_id") for row in recovery_moments if isinstance(row, Mapping)
    ] != list(expected_recovery_targets) or len(recovery_moments) != len(
        expected_recovery_targets
    ):
        raise ArtifactTamperError("shard recovery moment inventory differs")
    for moment in recovery_moments:
        if (
            not isinstance(moment, Mapping)
            or set(moment)
            != {"target_id", "n", "failed_count", "sum_error", "sum_squared_error"}
            or type(moment["n"]) is not int
            or type(moment["failed_count"]) is not int
            or moment["n"] < 0
            or moment["failed_count"] < 0
            or moment["n"] + moment["failed_count"] != attempted
            or not isinstance(moment["sum_error"], (int, float))
            or not isinstance(moment["sum_squared_error"], (int, float))
            or not math.isfinite(float(moment["sum_error"]))
            or not math.isfinite(float(moment["sum_squared_error"]))
            or moment["sum_squared_error"] < 0
        ):
            raise ArtifactTamperError("shard recovery moment accounting differs")
    if scenario["suite"] == "current_product_comparison":
        if artifact["first_observation"] != artifact["last_observation"]:
            raise ArtifactTamperError("product shard observations differ")
        _validate_embedded_product_observation(
            artifact["first_observation"], plan=plan, shard=shard
        )
    _validate_compact_shard_ledger_row(
        _compact_shard_ledger_row(artifact, scenario),
        plan=plan,
        shard=shard,
        scenario=scenario,
    )


def shard_path(output_root: Path, shard_id: str) -> Path:
    if not _safe_id(shard_id):
        raise PlanValidationError("unsafe shard identifier")
    return output_root / "shards" / f"{shard_id}.json"


def claim_path(output_root: Path, shard_id: str) -> Path:
    if not _safe_id(shard_id):
        raise PlanValidationError("unsafe shard identifier")
    return output_root / "claims" / f"{shard_id}.claim.json"


def load_validated_shard(
    path: Path, plan: Mapping[str, object], shard: Mapping[str, object]
) -> Mapping[str, object]:
    loaded = load_json(path)
    if not isinstance(loaded, Mapping):
        raise ArtifactTamperError(f"completed shard is not an object: {path}")
    validate_shard_artifact(loaded, plan, shard)
    return loaded


def _pid_is_alive(pid: int) -> bool:
    if pid <= 0:
        return False
    try:
        os.kill(pid, 0)
    except OSError:
        return False
    return True


def _claim_is_stale(claim: Mapping[str, object], stale_seconds: float) -> bool:
    created = claim.get("created_unix_ns")
    host = claim.get("host")
    pid = claim.get("pid")
    age_stale = type(created) is int and time.time_ns() - created >= int(
        stale_seconds * 1_000_000_000
    )
    dead_local = (
        host == socket.gethostname() and type(pid) is int and not _pid_is_alive(pid)
    )
    return bool(age_stale or dead_local)


def _quarantine_claim(path: Path, claim: Mapping[str, object]) -> None:
    abandoned = path.parent / "abandoned"
    abandoned.mkdir(parents=True, exist_ok=True)
    suffix = canonical_sha256(claim)[:16]
    destination = abandoned / f"{path.stem}.{suffix}.json"
    try:
        os.rename(path, destination)
    except FileExistsError:
        destination = abandoned / f"{path.stem}.{suffix}.{uuid.uuid4().hex}.json"
        os.rename(path, destination)


def _acquire_claim(
    path: Path,
    *,
    plan_sha256: str,
    shard_id: str,
    stale_seconds: float,
) -> dict[str, object]:
    path.parent.mkdir(parents=True, exist_ok=True)
    for _ in range(3):
        token = uuid.uuid4().hex
        claim: dict[str, object] = {
            "schema_version": 1,
            "kind": "general_sem_rank0_shard_claim_v1",
            "plan_sha256": plan_sha256,
            "shard_id": shard_id,
            "host": socket.gethostname(),
            "pid": os.getpid(),
            "created_unix_ns": time.time_ns(),
            "token": token,
        }
        try:
            descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        except FileExistsError:
            try:
                existing = load_json(path)
            except (OSError, ValueError, json.JSONDecodeError) as error:
                raise ClaimBusyError(
                    f"claim exists but cannot be read: {path}"
                ) from error
            if not isinstance(existing, Mapping) or not _claim_is_stale(
                existing, stale_seconds
            ):
                raise ClaimBusyError(f"shard is owned by a live claim: {shard_id}")
            _quarantine_claim(path, existing)
            continue
        with os.fdopen(descriptor, "wb", closefd=True) as handle:
            handle.write(canonical_bytes(claim))
            handle.flush()
            os.fsync(handle.fileno())
        return claim
    raise ClaimBusyError(
        f"could not acquire shard after stale-claim recovery: {shard_id}"
    )


def _release_claim(path: Path, claim: Mapping[str, object]) -> None:
    try:
        current = load_json(path)
    except FileNotFoundError:
        return
    if isinstance(current, Mapping) and current.get("token") == claim.get("token"):
        path.unlink()


def execute_shard(
    plan: Mapping[str, object],
    shard_id: str,
    output_root: Path,
    *,
    stale_claim_seconds: float = DEFAULT_STALE_CLAIM_SECONDS,
) -> dict[str, object]:
    validate_plan(plan)
    enforce_resource_guard()
    shards = _shard_map(plan)
    if shard_id not in shards:
        raise PlanValidationError(f"unknown shard {shard_id!r}")
    shard = shards[shard_id]
    scenario = _scenario_map(plan)[str(shard["scenario_id"])]
    if scenario["suite"] == "current_product_comparison":
        return {
            "shard_id": shard_id,
            "status": "requires_serialized_cargo_bundle",
            "path": None,
        }
    if scenario["suite"] in {"maximum_axis", "compound_stress"}:
        return {
            "shard_id": shard_id,
            "status": "requires_external_performance_receipt",
            "path": None,
        }
    destination = shard_path(output_root, shard_id)
    if destination.exists():
        load_validated_shard(destination, plan, shard)
        return {
            "shard_id": shard_id,
            "status": "accepted_existing",
            "path": str(destination),
        }
    claim_file = claim_path(output_root, shard_id)
    claim = _acquire_claim(
        claim_file,
        plan_sha256=str(plan["plan_sha256"]),
        shard_id=shard_id,
        stale_seconds=stale_claim_seconds,
    )
    try:
        if destination.exists():
            load_validated_shard(destination, plan, shard)
            return {
                "shard_id": shard_id,
                "status": "accepted_existing",
                "path": str(destination),
            }
        artifact = build_shard_artifact(plan, shard)
        validate_shard_artifact(artifact, plan, shard)
        _exclusive_atomic_publish(destination, canonical_bytes(artifact))
        load_validated_shard(destination, plan, shard)
        return {"shard_id": shard_id, "status": "published", "path": str(destination)}
    finally:
        _release_claim(claim_file, claim)


def build_product_request(
    plan: Mapping[str, object], shard_id: str
) -> dict[str, object]:
    validate_plan(plan)
    shard = _shard_map(plan).get(shard_id)
    if shard is None:
        raise PlanValidationError(f"unknown shard {shard_id!r}")
    definition = _scenario_map(plan)[str(shard["scenario_id"])]
    if definition["suite"] != "current_product_comparison":
        raise PlanValidationError("shard is not a current-product comparison")
    trial_index = int(shard["trial_start_inclusive"])
    generated = make_runner_scenario(definition, trial_index)
    operation = (
        f"{definition['family']}_{'bootstrap' if definition['stochastic'] else 'point'}"
    )
    workload = definition["workload"]
    requested = int(workload["resamples"]) if definition["stochastic"] else None  # type: ignore[index]
    bootstrap_seed = trial_seed(int(definition["seed_base"]), trial_index, "bootstrap")
    oracle_request = build_csem_request(
        generated,
        operation,  # type: ignore[arg-type]
        requested=requested,
        seed=bootstrap_seed,
    )
    required_worker_axes = (
        plan["current_product_comparison_contract"][  # type: ignore[index]
            "bootstrap_required_worker_axes"
        ]
        if definition["stochastic"]
        else plan["current_product_comparison_contract"][  # type: ignore[index]
            "point_required_worker_axes"
        ]
    )
    bootstrap_input = (
        {
            "confidence_level": oracle_request["bootstrap"]["confidence_level"],
            "requested": requested,
            "index_plan_authority": PRODUCT_INDEX_AUTHORITY,
        }
        if requested is not None
        else None
    )
    product_input = {
        "schema_version": 1,
        "scenario_id": definition["scenario_id"],
        "columns": oracle_request["columns"],
        "blocks": oracle_request["blocks"],
        "paths": oracle_request["paths"],
        "interactions": oracle_request["interactions"],
        "effect_target": oracle_request["effect_target"],
        "bootstrap": bootstrap_input,
    }
    body: dict[str, object] = {
        "schema_version": 1,
        "kind": PRODUCT_REQUEST_KIND,
        "integrity_scope": INTEGRITY_SCOPE,
        "plan_sha256": plan["plan_sha256"],
        "shard_id": shard_id,
        "scenario_id": definition["scenario_id"],
        "scenario_sha256": shard["scenario_sha256"],
        "cell_id": definition["cell_id"],
        "method_version": definition["method_version"],
        "analytical_method_version": definition["analytical_method_version"],
        "operation": operation,
        "scenario_seed": trial_seed(int(definition["seed_base"]), trial_index),
        "bootstrap_seed": bootstrap_seed if definition["stochastic"] else None,
        "required_worker_axes": list(required_worker_axes),
        "product_input": product_input,
        "required_production_result": {
            "raw_typed_result": True,
            "exact_bootstrap_indices": True,
            "point_bindings": [
                "weights",
                "loadings",
                "paths_and_effects_or_joint_stage_two",
                "scientific_gamma_and_product_scaling",
            ],
            "bootstrap_bindings": (
                [
                    "seed",
                    "workers",
                    "requested",
                    "usable",
                    "minimum_usable",
                    "summaries",
                    "ordered_failure_ledger",
                    "usable_replicate_indices",
                    "complete_model_refit_receipts",
                ]
                if definition["stochastic"]
                else None
            ),
        },
        "cargo_execution": {
            "serialized": True,
            "owner": "root_integration_lane",
            "automatic_parallel_invocation_forbidden": True,
            "package": PRODUCT_CARGO_PACKAGE,
            "example": PRODUCT_CARGO_EXAMPLE,
            "bundle_kind": PRODUCT_BUNDLE_KIND,
        },
    }
    return {**body, "request_sha256": canonical_sha256(body)}


def publish_product_requests(
    plan: Mapping[str, object], output_root: Path
) -> list[Path]:
    validate_plan(plan)
    scenario_by_id = _scenario_map(plan)
    paths: list[Path] = []
    for shard in plan["shards"]:
        definition = scenario_by_id[str(shard["scenario_id"])]
        if definition["suite"] != "current_product_comparison":
            continue
        request = build_product_request(plan, str(shard["shard_id"]))
        path = output_root / "product_requests" / f"{shard['shard_id']}.json"
        _exclusive_atomic_publish(path, canonical_bytes(request))
        paths.append(path)
    return paths


def _strict_sha256(value: object) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def _product_source_receipt() -> dict[str, object]:
    try:
        return dict(unified_rank0_source_receipt(ROOT))
    except ReceiptPayloadError as error:
        raise ArtifactTamperError(
            "unified Rank 0 source inventory is invalid"
        ) from error


def _validate_product_source_receipt(
    receipt: Mapping[str, object],
    *,
    accepted_historical_source_sha256: str | None = None,
) -> None:
    if accepted_historical_source_sha256 is not None:
        files = receipt.get("files")
        if (
            set(receipt) != {"scope", "file_count", "files", "source_set_sha256"}
            or receipt.get("scope") != "quickpls_general_sem_rank0_unified_sources_v2"
            or not isinstance(files, list)
            or receipt.get("file_count") != len(files)
            or receipt.get("source_set_sha256") != receipt_canonical_sha256(files)
            or receipt.get("source_set_sha256") != accepted_historical_source_sha256
        ):
            raise ArtifactTamperError(
                "historical product source receipt identity differs"
            )
        return
    try:
        validate_unified_rank0_source_receipt(
            receipt, ROOT, subject="product source receipt"
        )
    except ReceiptPayloadError as error:
        raise ArtifactTamperError("product source receipt identity differs") from error


def _product_executable_path() -> Path:
    target = os.environ.get("CARGO_TARGET_DIR")
    target_root = Path(target).resolve() if target else ROOT / "target"
    suffix = ".exe" if os.name == "nt" else ""
    return (
        target_root / "debug" / "examples" / f"{PRODUCT_CARGO_EXAMPLE}{suffix}"
    ).resolve()


def _path_descriptor(path: Path) -> dict[str, object]:
    resolved = path.resolve()
    try:
        relative = resolved.relative_to(ROOT).as_posix()
    except ValueError as error:
        raise ArtifactTamperError("product artifact leaves the repository") from error
    if not resolved.is_file() or resolved.is_symlink():
        raise ArtifactTamperError(f"product artifact is unavailable: {relative}")
    payload = resolved.read_bytes()
    return {
        "path": relative,
        "size": len(payload),
        "sha256": hashlib.sha256(payload).hexdigest(),
    }


def _validate_product_execution_receipt(
    receipt: Mapping[str, object],
    *,
    plan: Mapping[str, object],
    request: Mapping[str, object],
    bundle: Mapping[str, object],
    output_root: Path,
) -> None:
    expected_fields = {
        "schema_version",
        "kind",
        "plan_sha256",
        "shard_id",
        "request_sha256",
        "cargo_command",
        "working_directory",
        "started_unix_ns",
        "finished_unix_ns",
        "cargo_exit_code",
        "stdout_sha256",
        "stderr_sha256",
        "execution_nonce",
        "source_receipt",
        "executable_descriptor",
        "bundle_descriptor",
        "bundle_content_sha256",
        "producer_executable_sha256",
        "maximum_available_workers",
        "receipt_sha256",
    }
    if set(receipt) != expected_fields:
        raise ArtifactTamperError("product execution receipt fields differ")
    body = {key: value for key, value in receipt.items() if key != "receipt_sha256"}
    source_receipt = receipt.get("source_receipt")
    executable = receipt.get("executable_descriptor")
    bundle_descriptor = receipt.get("bundle_descriptor")
    started = receipt.get("started_unix_ns")
    finished = receipt.get("finished_unix_ns")
    expected_command = [
        "cargo",
        "run",
        "--locked",
        "-p",
        PRODUCT_CARGO_PACKAGE,
        "--example",
        PRODUCT_CARGO_EXAMPLE,
        "--",
        str(
            (output_root / "product_requests" / f"{request['shard_id']}.json").resolve()
        ),
        str(
            (output_root / "product_bundles" / f"{request['shard_id']}.json").resolve()
        ),
    ]
    if (
        receipt.get("schema_version") != 2
        or receipt.get("kind") != PRODUCT_EXECUTION_RECEIPT_KIND
        or receipt.get("plan_sha256") != plan["plan_sha256"]
        or receipt.get("shard_id") != request["shard_id"]
        or receipt.get("request_sha256") != request["request_sha256"]
        or receipt.get("cargo_command") != expected_command
        or receipt.get("working_directory") != str(ROOT)
        or type(started) is not int
        or type(finished) is not int
        or finished < started
        or receipt.get("cargo_exit_code") != 0
        or not _strict_sha256(receipt.get("stdout_sha256"))
        or not _strict_sha256(receipt.get("stderr_sha256"))
        or not isinstance(receipt.get("execution_nonce"), str)
        or len(receipt["execution_nonce"]) != 32
        or any(
            character not in "0123456789abcdef"
            for character in receipt["execution_nonce"]
        )
        or receipt.get("execution_nonce") != bundle.get("execution_nonce")
        or not isinstance(source_receipt, Mapping)
        or not isinstance(executable, Mapping)
        or not isinstance(bundle_descriptor, Mapping)
        or receipt.get("bundle_content_sha256") != canonical_sha256(bundle)
        or receipt.get("producer_executable_sha256")
        != bundle.get("producer_executable_sha256")
        or receipt.get("maximum_available_workers")
        != bundle.get("maximum_available_workers")
        or type(receipt.get("maximum_available_workers")) is not int
        or receipt["maximum_available_workers"] < 1
        or receipt.get("receipt_sha256") != canonical_sha256(body)
    ):
        raise ArtifactTamperError("product execution receipt identity differs")
    _validate_product_source_receipt(source_receipt)
    for descriptor in (executable, bundle_descriptor):
        if set(descriptor) != {"path", "size", "sha256"}:
            raise ArtifactTamperError("product artifact descriptor fields differ")
        resolved = (ROOT / str(descriptor["path"])).resolve()
        if _path_descriptor(resolved) != descriptor:
            raise ArtifactTamperError("product artifact descriptor content differs")
    expected_bundle_path = (
        output_root / "product_bundles" / f"{request['shard_id']}.json"
    )
    if executable != _path_descriptor(
        _product_executable_path()
    ) or bundle_descriptor != _path_descriptor(expected_bundle_path):
        raise ArtifactTamperError("product artifact descriptor path differs")
    if receipt["producer_executable_sha256"] != executable["sha256"]:
        raise ArtifactTamperError(
            "producer executable SHA differs from the executed binary"
        )


def _validate_embedded_product_observation(
    observation: object,
    *,
    plan: Mapping[str, object],
    shard: Mapping[str, object],
    accepted_historical_source_sha256: str | None = None,
) -> None:
    """Deeply revalidate the compact, source-bound product execution evidence."""

    if not isinstance(observation, Mapping):
        raise ArtifactTamperError("product observation is absent")
    expected_fields = {
        "operation",
        "cell_id",
        "method_version",
        "analytical_method_version",
        "scenario_seed",
        "bootstrap_seed",
        "bootstrap_index_plan_sha256",
        "cargo_invocation",
        "producer_executable_sha256",
        "maximum_available_workers",
        "producer_contract_version",
        "execution_receipt_sha256",
        "execution_nonce",
        "product_source_set_sha256",
        "executable_descriptor",
        "execution_receipt",
        "worker_comparisons",
        "difference_count",
        "maximum_absolute_difference",
        "difference_witnesses",
        "python_result_sha256",
        "r_result_sha256",
        "r_runtime",
        "required_bindings",
    }
    if set(observation) != expected_fields:
        raise ArtifactTamperError("product observation fields differ")
    definition = _scenario_map(plan)[str(shard["scenario_id"])]
    request = build_product_request(plan, str(shard["shard_id"]))
    receipt = observation.get("execution_receipt")
    if not isinstance(receipt, Mapping):
        raise ArtifactTamperError("embedded product execution receipt is absent")
    expected_receipt_fields = {
        "schema_version",
        "kind",
        "plan_sha256",
        "shard_id",
        "request_sha256",
        "cargo_command",
        "working_directory",
        "started_unix_ns",
        "finished_unix_ns",
        "cargo_exit_code",
        "stdout_sha256",
        "stderr_sha256",
        "execution_nonce",
        "source_receipt",
        "executable_descriptor",
        "bundle_descriptor",
        "bundle_content_sha256",
        "producer_executable_sha256",
        "maximum_available_workers",
        "receipt_sha256",
    }
    if set(receipt) != expected_receipt_fields:
        raise ArtifactTamperError("embedded product execution receipt fields differ")
    receipt_body = {
        key: value for key, value in receipt.items() if key != "receipt_sha256"
    }
    source_receipt = receipt.get("source_receipt")
    executable = receipt.get("executable_descriptor")
    bundle_descriptor = receipt.get("bundle_descriptor")
    command = receipt.get("cargo_command")
    started = receipt.get("started_unix_ns")
    finished = receipt.get("finished_unix_ns")
    if (
        receipt.get("schema_version") != 2
        or receipt.get("kind") != PRODUCT_EXECUTION_RECEIPT_KIND
        or receipt.get("plan_sha256") != plan["plan_sha256"]
        or receipt.get("shard_id") != shard["shard_id"]
        or receipt.get("request_sha256") != request["request_sha256"]
        or receipt.get("working_directory") != str(ROOT)
        or type(started) is not int
        or type(finished) is not int
        or finished < started
        or receipt.get("cargo_exit_code") != 0
        or not _strict_sha256(receipt.get("stdout_sha256"))
        or not _strict_sha256(receipt.get("stderr_sha256"))
        or not _strict_sha256(receipt.get("bundle_content_sha256"))
        or receipt.get("receipt_sha256") != canonical_sha256(receipt_body)
        or receipt.get("receipt_sha256") != observation["execution_receipt_sha256"]
        or receipt.get("execution_nonce") != observation["execution_nonce"]
        or receipt.get("producer_executable_sha256")
        != observation["producer_executable_sha256"]
        or receipt.get("maximum_available_workers")
        != observation["maximum_available_workers"]
        or receipt.get("executable_descriptor") != observation["executable_descriptor"]
        or not isinstance(source_receipt, Mapping)
        or source_receipt.get("source_set_sha256")
        != observation["product_source_set_sha256"]
        or not isinstance(executable, Mapping)
        or not isinstance(bundle_descriptor, Mapping)
        or not isinstance(command, list)
        or command[:8]
        != [
            "cargo",
            "run",
            "--locked",
            "-p",
            PRODUCT_CARGO_PACKAGE,
            "--example",
            PRODUCT_CARGO_EXAMPLE,
            "--",
        ]
        or len(command) != 10
    ):
        raise ArtifactTamperError("embedded product execution receipt identity differs")
    _validate_product_source_receipt(
        source_receipt,
        accepted_historical_source_sha256=accepted_historical_source_sha256,
    )
    for descriptor in (executable, bundle_descriptor):
        if (
            set(descriptor) != {"path", "size", "sha256"}
            or type(descriptor.get("size")) is not int
            or descriptor["size"] <= 0
            or not _strict_sha256(descriptor.get("sha256"))
        ):
            raise ArtifactTamperError("embedded product artifact descriptor differs")
    expected_executable_path = _product_executable_path().relative_to(ROOT).as_posix()
    if (
        executable["path"] != expected_executable_path
        or receipt["producer_executable_sha256"] != executable["sha256"]
        or Path(str(command[8])).name != f"{shard['shard_id']}.json"
        or Path(str(command[9])).name != f"{shard['shard_id']}.json"
        or Path(str(command[8])).parent.name != "product_requests"
        or Path(str(command[9])).parent.name != "product_bundles"
        or Path(str(bundle_descriptor["path"])).as_posix()
        != (
            Path("validation/results/general_sem_rank0_qualification_v1")
            / "product_bundles"
            / f"{shard['shard_id']}.json"
        ).as_posix()
    ):
        raise ArtifactTamperError("embedded product artifact path binding differs")
    expected_seed = (
        trial_seed(
            int(definition["seed_base"]),
            int(shard["trial_start_inclusive"]),
            "bootstrap",
        )
        if definition["stochastic"]
        else None
    )
    workers = observation.get("worker_comparisons")
    witnesses = observation.get("difference_witnesses")
    if (
        observation.get("operation") != request["operation"]
        or observation.get("cell_id") != definition["cell_id"]
        or observation.get("method_version") != definition["method_version"]
        or observation.get("analytical_method_version")
        != definition["analytical_method_version"]
        or observation.get("scenario_seed") != request["scenario_seed"]
        or observation.get("bootstrap_seed") != expected_seed
        or observation.get("producer_contract_version") != PRODUCT_PRODUCER_CONTRACT
        or not _strict_sha256(observation.get("producer_executable_sha256"))
        or type(observation.get("maximum_available_workers")) is not int
        or observation["maximum_available_workers"] < 1
        or not str(observation.get("cargo_invocation", "")).startswith(
            f"cargo run --locked -p {PRODUCT_CARGO_PACKAGE} --example {PRODUCT_CARGO_EXAMPLE} -- "
        )
        or observation.get("required_bindings")
        != plan["current_product_comparison_contract"]["required_bindings"]
        or (
            definition["stochastic"]
            and not _strict_sha256(observation.get("bootstrap_index_plan_sha256"))
        )
        or (
            not definition["stochastic"]
            and observation.get("bootstrap_index_plan_sha256") is not None
        )
        or not _strict_sha256(observation.get("python_result_sha256"))
        or not _strict_sha256(observation.get("r_result_sha256"))
        or not isinstance(observation.get("r_runtime"), Mapping)
        or type(observation.get("difference_count")) is not int
        or observation["difference_count"] < 0
        or not isinstance(witnesses, list)
        or len(witnesses) > min(int(observation["difference_count"]), 20)
        or not isinstance(workers, list)
        or len(workers) != len(request["required_worker_axes"])
    ):
        raise ArtifactTamperError("embedded product comparison binding differs")
    total_differences = 0
    for index, row in enumerate(workers):
        production_receipts = (
            row.get("production_receipts") if isinstance(row, Mapping) else None
        )
        if (
            not isinstance(row, Mapping)
            or set(row)
            != {
                "workers",
                "worker_axis",
                "product_result_sha256",
                "product_vs_python_difference_count",
                "product_vs_r_difference_count",
                "worker_replay_difference_count",
                "production_receipts",
            }
            or row.get("worker_axis") != request["required_worker_axes"][index]
            or type(row.get("workers")) is not int
            or row["workers"] < 1
            or not _strict_sha256(row.get("product_result_sha256"))
            or any(
                type(row.get(field)) is not int or row[field] < 0
                for field in (
                    "product_vs_python_difference_count",
                    "product_vs_r_difference_count",
                    "worker_replay_difference_count",
                )
            )
            or not isinstance(production_receipts, Mapping)
            or set(production_receipts)
            != {
                "production_result_sha256",
                "compiled_plan_sha256",
                "artifact_identity_sha256",
                "requested_analytical_method_version",
                "point_analytical_method_version",
                "bootstrap",
            }
            or not _strict_sha256(production_receipts.get("production_result_sha256"))
            or not _strict_sha256(production_receipts.get("compiled_plan_sha256"))
            or not _strict_sha256(production_receipts.get("artifact_identity_sha256"))
            or production_receipts.get("requested_analytical_method_version")
            != definition["analytical_method_version"]
            or production_receipts.get("point_analytical_method_version")
            != CELL_CONTRACTS[f"{definition['family']}_point"].analytical_method_version
            or bool(production_receipts.get("bootstrap") is not None)
            is not bool(definition["stochastic"])
        ):
            raise ArtifactTamperError("embedded product worker comparison differs")
        bootstrap_receipt = production_receipts["bootstrap"]
        if definition["stochastic"]:
            expected_primary, expected_supplemental = (
                _expected_product_capability_authorities(definition)
            )
            if (
                not isinstance(bootstrap_receipt, Mapping)
                or set(bootstrap_receipt)
                != {
                    "operation",
                    "requested",
                    "usable",
                    "failed",
                    "failure_reason_totals",
                    "failure_ledger_sha256",
                    "usable_indices_sha256",
                    "production_usable_indices_sha256",
                    "authority_sha256",
                    "primary_capability_cell",
                    "supplemental_capability_cell",
                    "supplemental_method_version",
                }
                or bootstrap_receipt["requested"] != definition["workload"]["resamples"]
                or type(bootstrap_receipt["usable"]) is not int
                or type(bootstrap_receipt["failed"]) is not int
                or bootstrap_receipt["usable"] + bootstrap_receipt["failed"]
                != bootstrap_receipt["requested"]
                or bootstrap_receipt["usable"] * 10 < bootstrap_receipt["requested"] * 9
                or not isinstance(bootstrap_receipt["failure_reason_totals"], Mapping)
                or any(
                    not isinstance(reason, str)
                    or not reason
                    or type(count) is not int
                    or count <= 0
                    for reason, count in bootstrap_receipt[
                        "failure_reason_totals"
                    ].items()
                )
                or sum(bootstrap_receipt["failure_reason_totals"].values())
                != bootstrap_receipt["failed"]
                or any(
                    not _strict_sha256(bootstrap_receipt[field])
                    for field in (
                        "failure_ledger_sha256",
                        "usable_indices_sha256",
                        "production_usable_indices_sha256",
                        "authority_sha256",
                    )
                )
                or bootstrap_receipt["primary_capability_cell"] != expected_primary
                or bootstrap_receipt["supplemental_capability_cell"]
                != expected_supplemental
                or bootstrap_receipt["supplemental_method_version"]
                != definition["analytical_method_version"]
            ):
                raise ArtifactTamperError(
                    "embedded product bootstrap accounting differs"
                )
        elif bootstrap_receipt is not None:
            raise ArtifactTamperError(
                "embedded point product carries bootstrap accounting"
            )
        if (
            row["worker_axis"] == "not_applicable"
            and row["workers"] != 1
            or row["worker_axis"] in {"1", "2", "4"}
            and row["workers"] != int(row["worker_axis"])
            or row["worker_axis"] == "max"
            and row["workers"] != observation["maximum_available_workers"]
        ):
            raise ArtifactTamperError("embedded product worker-axis resolution differs")
        total_differences += sum(
            int(row[field])
            for field in (
                "product_vs_python_difference_count",
                "product_vs_r_difference_count",
                "worker_replay_difference_count",
            )
        )
    if total_differences != observation["difference_count"]:
        raise ArtifactTamperError("embedded product difference accounting differs")


def run_product_comparison(
    plan: Mapping[str, object],
    shard_id: str,
    output_root: Path,
    *,
    cargo_program: str = "cargo",
    timeout_seconds: int = 3_600,
) -> dict[str, object]:
    """Run one serialized production adapter and ingest its bound output."""

    validate_plan(plan)
    enforce_resource_guard()
    if cargo_program != "cargo":
        raise PlanValidationError(
            "current-product execution requires the exact locked Cargo program"
        )
    request = build_product_request(plan, shard_id)
    request_path = output_root / "product_requests" / f"{shard_id}.json"
    _exclusive_atomic_publish(request_path, canonical_bytes(request))
    bundle_path = output_root / "product_bundles" / f"{shard_id}.json"
    execution_receipt_path = (
        output_root / "product_execution_receipts" / f"{shard_id}.json"
    )
    accepted_path = shard_path(output_root, shard_id)
    if (
        bundle_path.exists()
        or execution_receipt_path.exists()
        or accepted_path.exists()
    ):
        raise ArtifactTamperError(
            "preexisting product bundle, execution receipt, or accepted shard is refused"
        )
    command = [
        cargo_program,
        "run",
        "--locked",
        "-p",
        PRODUCT_CARGO_PACKAGE,
        "--example",
        PRODUCT_CARGO_EXAMPLE,
        "--",
        str(request_path.resolve()),
        str(bundle_path.resolve()),
    ]
    source_before = _product_source_receipt()
    execution_nonce = uuid.uuid4().hex
    cargo_environment = os.environ.copy()
    cargo_environment[PRODUCT_EXECUTION_NONCE_ENV] = execution_nonce
    started = time.time_ns()
    completed = subprocess.run(
        command,
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=timeout_seconds,
        check=False,
        env=cargo_environment,
    )
    finished = time.time_ns()
    source_after = _product_source_receipt()
    if source_after != source_before:
        raise ArtifactTamperError(
            "product Rust/Cargo source set changed during locked execution"
        )
    if completed.returncode != 0 or not bundle_path.is_file():
        detail = (completed.stderr or completed.stdout).strip()
        raise QualificationRunnerError(
            f"serialized current-product Cargo producer failed "
            f"({completed.returncode}): {detail[-4000:]}"
        )
    loaded = load_json(bundle_path)
    if not isinstance(loaded, Mapping):
        raise ArtifactTamperError("current-product Cargo bundle is not an object")
    executable_descriptor = _path_descriptor(_product_executable_path())
    producer_executable_sha256 = loaded.get("producer_executable_sha256")
    if producer_executable_sha256 != executable_descriptor["sha256"]:
        raise ArtifactTamperError(
            "product bundle producer SHA differs from the exact Cargo executable"
        )
    receipt_body: dict[str, object] = {
        "schema_version": 2,
        "kind": PRODUCT_EXECUTION_RECEIPT_KIND,
        "plan_sha256": plan["plan_sha256"],
        "shard_id": shard_id,
        "request_sha256": request["request_sha256"],
        "cargo_command": command,
        "working_directory": str(ROOT),
        "started_unix_ns": started,
        "finished_unix_ns": finished,
        "cargo_exit_code": completed.returncode,
        "stdout_sha256": hashlib.sha256(completed.stdout.encode("utf-8")).hexdigest(),
        "stderr_sha256": hashlib.sha256(completed.stderr.encode("utf-8")).hexdigest(),
        "execution_nonce": execution_nonce,
        "source_receipt": source_before,
        "executable_descriptor": executable_descriptor,
        "bundle_descriptor": _path_descriptor(bundle_path),
        "bundle_content_sha256": canonical_sha256(loaded),
        "producer_executable_sha256": producer_executable_sha256,
        "maximum_available_workers": loaded["maximum_available_workers"],
    }
    execution_receipt = {
        **receipt_body,
        "receipt_sha256": canonical_sha256(receipt_body),
    }
    _validate_product_execution_receipt(
        execution_receipt,
        plan=plan,
        request=request,
        bundle=loaded,
        output_root=output_root,
    )
    _exclusive_atomic_publish(
        execution_receipt_path, canonical_bytes(execution_receipt)
    )
    shard_path_value = ingest_product_bundle(
        plan,
        loaded,
        output_root,
        execution_receipt=execution_receipt,
    )
    return {
        "shard_id": shard_id,
        "request_path": str(request_path),
        "bundle_path": str(bundle_path),
        "execution_receipt_path": str(execution_receipt_path),
        "accepted_shard_path": str(shard_path_value),
        "cargo_command": command,
        "qualification_ready": False,
    }


def _bare_production_id(value: object) -> str:
    identifier = str(value)
    for prefix in ("construct:", "interaction:", "variable:"):
        if identifier.startswith(prefix):
            return identifier[len(prefix) :]
    return identifier


def _product_capability_reference(cell_key: str) -> dict[str, object]:
    contract = CELL_CONTRACTS[cell_key]
    return {
        "registry_schema_version": 2,
        "capability_id": contract.capability_id,
        "cell_id": contract.cell_id,
        "capability_version": contract.method_version,
    }


def _expected_product_capability_authorities(
    definition: Mapping[str, object],
) -> tuple[dict[str, object], dict[str, object] | None]:
    family = str(definition["family"])
    primary = _product_capability_reference(f"{family}_point")
    supplemental = (
        _product_capability_reference(f"{family}_bootstrap")
        if bool(definition["stochastic"])
        else None
    )
    return primary, supplemental


def _expected_product_analytical_method(definition: Mapping[str, object]) -> str:
    contract = CELL_CONTRACTS.get(str(definition.get("cell_key")))
    if (
        contract is None
        or definition.get("analytical_method_version")
        != contract.analytical_method_version
    ):
        raise ArtifactTamperError("product analytical method authority differs")
    return contract.analytical_method_version


def _product_complete_case_count(generated: GeneralSemScenario) -> int:
    indicators = tuple(
        indicator
        for block in generated.model.blocks
        for indicator in block.indicator_ids
    )
    return sum(
        all(row.get(indicator) is not None for indicator in indicators)
        for row in generated.rows
    )


def _validated_product_index_plan(
    bundle: Mapping[str, object],
    request: Mapping[str, object],
    generated: GeneralSemScenario,
) -> tuple[tuple[int, ...], ...] | None:
    value = bundle.get("bootstrap_index_plan")
    product_input = request["product_input"]
    if not isinstance(product_input, Mapping):
        raise ArtifactTamperError("current-product input is not an object")
    bootstrap = product_input.get("bootstrap")
    if bootstrap is None:
        if value is not None:
            raise ArtifactTamperError("point product bundle carries a bootstrap plan")
        return None
    if not isinstance(bootstrap, Mapping) or not isinstance(value, Mapping):
        raise ArtifactTamperError("bootstrap product index plan is absent")
    if set(value) != {
        "authority",
        "operation",
        "complete_case_count",
        "requested",
        "seed",
        "replicate_indices",
    }:
        raise ArtifactTamperError("bootstrap product index-plan fields differ")
    cases = _product_complete_case_count(generated)
    requested = int(bootstrap["requested"])
    seed = request["bootstrap_seed"]
    raw_indices = value.get("replicate_indices")
    if (
        value.get("authority") != "qpls_resampling::bootstrap_indices"
        or not isinstance(value.get("operation"), str)
        or not str(value["operation"]).strip()
        or value.get("complete_case_count") != cases
        or value.get("requested") != requested
        or value.get("seed") != seed
        or not isinstance(raw_indices, list)
        or len(raw_indices) != requested
    ):
        raise ArtifactTamperError("bootstrap product index-plan binding differs")
    resolved: list[tuple[int, ...]] = []
    for replicate in raw_indices:
        if (
            not isinstance(replicate, list)
            or len(replicate) != cases
            or any(
                type(index) is not int or index < 0 or index >= cases
                for index in replicate
            )
        ):
            raise ArtifactTamperError(
                "bootstrap product index plan leaves the complete-case frame"
            )
        resolved.append(tuple(replicate))
    return tuple(resolved)


def _production_relation_map(
    worker: Mapping[str, object],
) -> dict[str, tuple[str, str]]:
    rows = worker.get("relation_identities")
    if not isinstance(rows, list) or not rows:
        raise ArtifactTamperError("production relation identity map is absent")
    result: dict[str, tuple[str, str]] = {}
    for row in rows:
        if not isinstance(row, Mapping) or set(row) != {
            "relation_id",
            "source_id",
            "target_id",
        }:
            raise ArtifactTamperError("production relation identity fields differ")
        relation_id = str(row["relation_id"])
        if relation_id in result:
            raise ArtifactTamperError("production relation identity is duplicated")
        result[relation_id] = (
            _bare_production_id(row["source_id"]),
            _bare_production_id(row["target_id"]),
        )
    return result


def _specific_effect_key(
    ordered_relation_ids: object,
    relation_map: Mapping[str, tuple[str, str]],
) -> str:
    if not isinstance(ordered_relation_ids, list) or len(ordered_relation_ids) < 2:
        raise ArtifactTamperError("production specific path is invalid")
    edges: list[tuple[str, str]] = []
    for relation_id in ordered_relation_ids:
        if str(relation_id) not in relation_map:
            raise ArtifactTamperError("production specific path relation is unknown")
        edges.append(relation_map[str(relation_id)])
    if any(left[1] != right[0] for left, right in zip(edges, edges[1:])):
        raise ArtifactTamperError("production specific path is discontinuous")
    route = [edges[0][0], *(edge[1] for edge in edges)]
    return "specific:" + "->".join(route)


def _production_bootstrap_summary(row: Mapping[str, object]) -> dict[str, object]:
    required = {
        "original",
        "bootstrap_mean",
        "bootstrap_bias",
        "standard_error",
        "lower",
        "upper",
        "p_value_two_sided",
        "usable_replicates",
        "two_sided_exceedances",
    }
    if not required <= set(row):
        raise ArtifactTamperError("production bootstrap summary fields are incomplete")
    return {
        "original": row["original"],
        "mean": row["bootstrap_mean"],
        "bias": row["bootstrap_bias"],
        "standard_error": row["standard_error"],
        "lower": row["lower"],
        "upper": row["upper"],
        "exceedances": row["two_sided_exceedances"],
        "plus_one_two_sided_probability": row["p_value_two_sided"],
    }


def _normalize_production_result(
    definition: Mapping[str, object],
    generated: GeneralSemScenario,
    worker: Mapping[str, object],
    index_plan: Sequence[Sequence[int]] | None,
) -> tuple[dict[str, object], dict[str, object]]:
    raw = worker.get("production_result")
    if not isinstance(raw, Mapping):
        raise ArtifactTamperError("typed production result is absent")
    required_root = {
        "schema_version",
        "adapter_version",
        "capability_cell",
        "compilation_artifact_identity_sha256",
        "compiled_plan_sha256",
        "recipe_analytical_sha256",
        "model_scientific_sha256",
        "stage_one_model_scientific_sha256",
        "source_dataset_fingerprint",
        "general_sem_config_sha256",
        "point_estimation",
        "requested_effects",
    }
    allowed_root = required_root | {
        "interaction_point_estimation",
        "bootstrap_inference",
        "moderation_bootstrap_inference",
    }
    expected_primary, expected_supplemental = _expected_product_capability_authorities(
        definition
    )
    expected_analytical_method = _expected_product_analytical_method(definition)
    primary_analytical_method = CELL_CONTRACTS[
        f"{definition['family']}_point"
    ].analytical_method_version
    if (
        raw.get("schema_version") != 1
        or not required_root <= set(raw)
        or not set(raw) <= allowed_root
        or not isinstance(raw.get("capability_cell"), Mapping)
        or dict(raw["capability_cell"]) != expected_primary
    ):
        raise ArtifactTamperError("typed production result authority differs")
    point = raw["point_estimation"]
    if not isinstance(point, Mapping) or not isinstance(
        point.get("estimation"), Mapping
    ):
        raise ArtifactTamperError("production point estimation is absent")
    estimation = point["estimation"]
    model = canonicalize_model(generated.model)
    weights = {block.construct_id: {} for block in model.blocks}
    loadings = {block.construct_id: {} for block in model.blocks}
    outer = estimation.get("outer_estimates")
    if not isinstance(outer, list):
        raise ArtifactTamperError("production outer estimates are absent")
    for estimate in outer:
        if not isinstance(estimate, Mapping):
            raise ArtifactTamperError("production outer estimate is invalid")
        construct = _bare_production_id(estimate.get("construct"))
        indicator = _bare_production_id(estimate.get("indicator"))
        if construct not in weights or indicator in weights[construct]:
            raise ArtifactTamperError("production outer estimate identity differs")
        weights[construct][indicator] = estimate.get("weight")
        loadings[construct][indicator] = estimate.get("loading")
    expected_outer = {
        (block.construct_id, indicator)
        for block in model.blocks
        for indicator in block.indicator_ids
    }
    actual_outer = {
        (construct, indicator)
        for construct, estimates in weights.items()
        for indicator in estimates
    }
    if actual_outer != expected_outer:
        raise ArtifactTamperError("production outer estimate inventory differs")
    relation_map = _production_relation_map(worker)
    stage_one_paths = estimation.get("paths")
    if not isinstance(stage_one_paths, list):
        raise ArtifactTamperError("production structural paths are absent")
    structural = {
        f"{_bare_production_id(row['source'])}->{_bare_production_id(row['target'])}": row[
            "coefficient"
        ]
        for row in stage_one_paths
        if isinstance(row, Mapping)
    }
    effect_key_by_product_id: dict[str, str] = {}
    if definition["family"] == "mediation":
        source_id = generated.source_id or "x"
        target_id = generated.target_id or "y"
        values: dict[str, object] = {}
        requested_effects = raw.get("requested_effects")
        if not isinstance(requested_effects, list):
            raise ArtifactTamperError("production requested effects are absent")
        for effect in requested_effects:
            if not isinstance(effect, Mapping):
                raise ArtifactTamperError("production requested effect is invalid")
            source = _bare_production_id(effect.get("source_id"))
            target = _bare_production_id(effect.get("target_id"))
            if (source, target) != (source_id, target_id):
                continue
            kind = effect.get("kind")
            if kind == "specific_indirect":
                key = _specific_effect_key(
                    effect.get("ordered_relation_ids"), relation_map
                )
                product_id = str(effect.get("path_identity"))
            elif kind == "total_indirect":
                key = f"total_indirect:{source}->{target}"
                product_id = str(effect.get("estimand_id"))
            elif kind == "total_effect":
                key = f"total:{source}->{target}"
                product_id = str(effect.get("estimand_id"))
            else:
                raise ArtifactTamperError("production requested effect kind differs")
            if key in values or product_id in effect_key_by_product_id:
                raise ArtifactTamperError("production requested effect is duplicated")
            values[key] = effect.get("coefficient")
            effect_key_by_product_id[product_id] = key
        direct_key = f"direct:{source_id}->{target_id}"
        path_key = f"{source_id}->{target_id}"
        values[direct_key] = structural.get(path_key, 0.0)
        point_values: dict[str, object] = {
            "values": values,
            "structural": structural,
        }
    else:
        interaction_point = raw.get("interaction_point_estimation")
        if (
            not isinstance(interaction_point, Mapping)
            or interaction_point.get("method_version") != primary_analytical_method
        ):
            raise ArtifactTamperError("production joint moderation point is absent")
        structural_rows = interaction_point.get("structural_coefficients")
        interaction_rows = interaction_point.get("interaction_coefficients")
        scale_rows = interaction_point.get("product_scale_receipts")
        slope_rows = interaction_point.get("simple_slopes")
        if not all(
            isinstance(rows, list)
            for rows in (structural_rows, interaction_rows, scale_rows, slope_rows)
        ):
            raise ArtifactTamperError("production moderation point sections are absent")
        joint_structural = {
            f"{_bare_production_id(row['source_id'])}->{_bare_production_id(row['target_id'])}": row[
                "estimate"
            ]
            for row in structural_rows
            if isinstance(row, Mapping)
        }
        standardized = {
            _bare_production_id(row["interaction_id"]): row[
                "standardized_product_estimate"
            ]
            for row in interaction_rows
            if isinstance(row, Mapping)
        }
        gammas = {
            _bare_production_id(row["interaction_id"]): row["raw_product_estimate"]
            for row in interaction_rows
            if isinstance(row, Mapping)
        }
        means = {
            _bare_production_id(row["interaction_id"]): row[
                "unstandardized_product_mean"
            ]
            for row in scale_rows
            if isinstance(row, Mapping)
        }
        scales = {
            _bare_production_id(row["interaction_id"]): row[
                "unstandardized_product_sample_standard_deviation"
            ]
            for row in scale_rows
            if isinstance(row, Mapping)
        }
        slopes: dict[str, list[tuple[float, object]]] = {}
        for row in slope_rows:
            if not isinstance(row, Mapping):
                raise ArtifactTamperError("production simple slope is invalid")
            interaction_id = _bare_production_id(row["interaction_id"])
            slopes.setdefault(interaction_id, []).append(
                (float(row["moderator_value_standardized"]), row["estimate"])
            )
        fixed_slopes = {
            interaction_id: [value for _, value in sorted(rows)]
            for interaction_id, rows in slopes.items()
        }
        expected_interactions = {
            interaction.interaction_id for interaction in generated.interactions
        }
        if not all(
            set(section) == expected_interactions
            for section in (standardized, gammas, means, scales, fixed_slopes)
        ) or any(
            [probe for probe, _ in sorted(rows)] != [-1.0, 0.0, 1.0]
            for rows in slopes.values()
        ):
            raise ArtifactTamperError("production moderation target inventory differs")
        point_values = {
            "structural": joint_structural,
            "standardized_product_coefficients": standardized,
            "scientific_gammas": gammas,
            "product_means": means,
            "product_scales": scales,
            "fixed_probe_slopes": fixed_slopes,
        }
    normalized_bootstrap: dict[str, object] | None = None
    bootstrap_receipt: dict[str, object] | None = None
    authority_receipt = worker.get("bootstrap_authority_receipt")
    if bool(definition["stochastic"]):
        bootstrap_field = (
            "bootstrap_inference"
            if definition["family"] == "mediation"
            else "moderation_bootstrap_inference"
        )
        bootstrap = raw.get(bootstrap_field)
        if not isinstance(bootstrap, Mapping) or index_plan is None:
            raise ArtifactTamperError("production bootstrap result is absent")
        expected_authority_fields = {
            "compiled_primary_capability_cell",
            "analytical_primary_capability_cell",
            "supplemental_inference_capability_cell",
            "supplemental_inference_method_version",
            "supplemental_resampling_operation_version",
        }
        if (
            expected_supplemental is None
            or not isinstance(authority_receipt, Mapping)
            or set(authority_receipt) != expected_authority_fields
            or authority_receipt["compiled_primary_capability_cell"] != expected_primary
            or authority_receipt["analytical_primary_capability_cell"]
            != expected_primary
            or authority_receipt["supplemental_inference_capability_cell"]
            != expected_supplemental
            or authority_receipt["supplemental_inference_method_version"]
            != expected_analytical_method
            or authority_receipt["supplemental_resampling_operation_version"]
            != bootstrap.get("resampling_operation_version")
            or bootstrap.get("method_version") != expected_analytical_method
        ):
            raise ArtifactTamperError(
                "production primary/supplemental bootstrap authority differs"
            )
        requested = len(index_plan)
        failures = bootstrap.get("failed_replicates")
        if not isinstance(failures, list):
            raise ArtifactTamperError("production bootstrap failure ledger is absent")
        failure_indices = [
            row.get("replicate_index") for row in failures if isinstance(row, Mapping)
        ]
        if (
            len(failure_indices) != len(failures)
            or failure_indices != sorted(failure_indices)
            or any(
                type(index) is not int or not 0 <= index < requested
                for index in failure_indices
            )
            or len(set(failure_indices)) != len(failure_indices)
        ):
            raise ArtifactTamperError("production bootstrap failure ledger differs")
        usable_indices = [
            index for index in range(requested) if index not in set(failure_indices)
        ]
        summaries: dict[str, object] = {}
        if definition["family"] == "mediation":
            summary_rows = bootstrap.get("effects")
            if not isinstance(summary_rows, list):
                raise ArtifactTamperError("production mediation summaries are absent")
            for row in summary_rows:
                if not isinstance(row, Mapping):
                    raise ArtifactTamperError("production mediation summary is invalid")
                product_id = str(row.get("effect_id"))
                if product_id in effect_key_by_product_id:
                    summaries[effect_key_by_product_id[product_id]] = (
                        _production_bootstrap_summary(row)
                    )
        else:
            summary_rows = bootstrap.get("interaction_gammas")
            if not isinstance(summary_rows, list):
                raise ArtifactTamperError("production moderation summaries are absent")
            for row in summary_rows:
                if not isinstance(row, Mapping) or not isinstance(
                    row.get("target"), Mapping
                ):
                    raise ArtifactTamperError(
                        "production moderation summary is invalid"
                    )
                identifier = _bare_production_id(row["target"].get("interaction_id"))
                summaries[identifier] = _production_bootstrap_summary(row)
        minimum = math.ceil(requested * 0.90)
        expected_seed = worker.get("seed")
        expected_workers = worker.get("workers")
        if (
            bootstrap.get("resamples_requested") != requested
            or bootstrap.get("resamples_usable") != len(usable_indices)
            or bootstrap.get("minimum_usable_resamples") != minimum
            or str(bootstrap.get("seed")) != str(expected_seed)
            or bootstrap.get("workers") != expected_workers
            or bootstrap.get("complete_model_reestimated_per_replicate") is not True
            or len(usable_indices) < minimum
        ):
            raise ArtifactTamperError("production bootstrap accounting differs")
        if definition["family"] == "moderation" and any(
            bootstrap.get(field) is not True
            for field in (
                "shared_stage_one_reestimated_per_replicate",
                "score_vectors_sign_aligned_before_products",
                "product_scaling_recomputed_per_replicate",
                "joint_stage_two_reestimated_per_replicate",
                "complete_joint_point_contract_validated_per_replicate",
            )
        ):
            raise ArtifactTamperError(
                "production joint moderation refit receipt differs"
            )
        normalized_bootstrap = {
            "requested": requested,
            "usable": len(usable_indices),
            "minimum_usable": minimum,
            "published": True,
            "summaries": summaries,
            "failures": [{"replicate_index": index} for index in failure_indices],
            "usable_indices": usable_indices,
        }
        bootstrap_receipt = {
            "operation": bootstrap.get("resampling_operation_version"),
            "requested": requested,
            "usable": len(usable_indices),
            "failed": len(failure_indices),
            "failure_reason_totals": _bootstrap_failure_reason_totals(
                (
                    row.get("reason_code", row.get("reason", row.get("message", "")))
                    for row in failures
                    if isinstance(row, Mapping)
                )
            ),
            "failure_ledger_sha256": canonical_sha256(failures),
            "usable_indices_sha256": canonical_sha256(usable_indices),
            "production_usable_indices_sha256": bootstrap.get(
                "usable_replicate_indices_sha256"
            ),
            "authority_sha256": canonical_sha256(authority_receipt),
            "primary_capability_cell": expected_primary,
            "supplemental_capability_cell": expected_supplemental,
            "supplemental_method_version": authority_receipt[
                "supplemental_inference_method_version"
            ],
        }
    elif authority_receipt is not None:
        raise ArtifactTamperError(
            "point production result carries a bootstrap authority receipt"
        )
    elif expected_analytical_method != primary_analytical_method:
        raise ArtifactTamperError("point analytical method authority differs")
    return (
        {
            "point": {"weights": weights, "loadings": loadings, "values": point_values},
            "bootstrap": normalized_bootstrap,
        },
        {
            "production_result_sha256": canonical_sha256(raw),
            "compiled_plan_sha256": raw["compiled_plan_sha256"],
            "artifact_identity_sha256": raw["compilation_artifact_identity_sha256"],
            "requested_analytical_method_version": expected_analytical_method,
            "point_analytical_method_version": primary_analytical_method,
            "bootstrap": bootstrap_receipt,
        },
    )


def _product_comparison_projection(
    value: Mapping[str, object], *, family: str
) -> dict[str, object]:
    projected = json.loads(json.dumps(value, allow_nan=False))
    bootstrap = projected.get("bootstrap")
    if isinstance(bootstrap, dict):
        bootstrap.pop("sign_corrections", None)
        failures = bootstrap.get("failures")
        if isinstance(failures, list):
            bootstrap["failures"] = [
                {"replicate_index": row.get("replicate_index")}
                for row in failures
                if isinstance(row, dict)
            ]
        if family == "mediation" and isinstance(bootstrap.get("summaries"), dict):
            bootstrap["summaries"] = {
                key: summary
                for key, summary in bootstrap["summaries"].items()
                if not key.startswith("direct:")
            }
    return projected


def ingest_product_bundle(
    plan: Mapping[str, object],
    bundle: Mapping[str, object],
    output_root: Path,
    *,
    execution_receipt: Mapping[str, object] | None,
) -> Path:
    """Bind one serialized production-API bundle and compare both oracles."""

    if execution_receipt is None:
        raise ArtifactTamperError(
            "current-product bundle lacks a new-run execution receipt"
        )
    validate_plan(plan)
    expected_fields = {
        "schema_version",
        "kind",
        "producer_contract_version",
        "request_sha256",
        "plan_sha256",
        "shard_id",
        "scenario_id",
        "scenario_sha256",
        "cell_id",
        "method_version",
        "analytical_method_version",
        "scenario_seed",
        "bootstrap_seed",
        "cargo_invocation",
        "cargo_exit_code",
        "producer_executable_sha256",
        "maximum_available_workers",
        "execution_nonce",
        "bootstrap_index_plan",
        "worker_runs",
    }
    if set(bundle) != expected_fields:
        raise ArtifactTamperError("current-product bundle fields differ")
    shard_id = bundle.get("shard_id")
    if not isinstance(shard_id, str) or shard_id not in _shard_map(plan):
        raise ArtifactTamperError("current-product bundle shard is unknown")
    shard = _shard_map(plan)[shard_id]
    definition = _scenario_map(plan)[str(shard["scenario_id"])]
    request = build_product_request(plan, shard_id)
    _validate_product_execution_receipt(
        execution_receipt,
        plan=plan,
        request=request,
        bundle=bundle,
        output_root=output_root,
    )
    trial_index = int(shard["trial_start_inclusive"])
    expected_seed = (
        trial_seed(int(definition["seed_base"]), trial_index, "bootstrap")
        if definition["stochastic"]
        else None
    )
    if (
        definition["suite"] != "current_product_comparison"
        or bundle["schema_version"] != 2
        or bundle["kind"] != PRODUCT_BUNDLE_KIND
        or bundle["producer_contract_version"] != PRODUCT_PRODUCER_CONTRACT
        or bundle["request_sha256"] != request["request_sha256"]
        or bundle["plan_sha256"] != plan["plan_sha256"]
        or bundle["scenario_id"] != definition["scenario_id"]
        or bundle["scenario_sha256"] != shard["scenario_sha256"]
        or bundle["cell_id"] != definition["cell_id"]
        or bundle["method_version"] != definition["method_version"]
        or bundle["analytical_method_version"]
        != definition["analytical_method_version"]
        or bundle["scenario_seed"]
        != trial_seed(int(definition["seed_base"]), trial_index)
        or bundle["bootstrap_seed"] != expected_seed
        or bundle["cargo_exit_code"] != 0
        or bundle["execution_nonce"] != execution_receipt["execution_nonce"]
        or not str(bundle["cargo_invocation"]).startswith(
            f"cargo run --locked -p {PRODUCT_CARGO_PACKAGE} --example {PRODUCT_CARGO_EXAMPLE} -- "
        )
        or not _strict_sha256(bundle["producer_executable_sha256"])
        or type(bundle["maximum_available_workers"]) is not int
        or bundle["maximum_available_workers"] < 1
    ):
        raise ArtifactTamperError("current-product execution binding differs")
    generated = make_runner_scenario(definition, trial_index)
    index_plan = _validated_product_index_plan(bundle, request, generated)
    worker_runs = bundle["worker_runs"]
    expected_worker_axes = request["required_worker_axes"]
    if (
        not isinstance(worker_runs, list)
        or len(worker_runs) != len(expected_worker_axes)
        or [row.get("worker_axis") for row in worker_runs if isinstance(row, Mapping)]
        != expected_worker_axes
    ):
        raise ArtifactTamperError("current-product worker inventory differs")
    python_result = _product_comparison_projection(
        _python_normalized_result(
            definition, generated, trial_index, index_plan=index_plan
        ),
        family=str(definition["family"]),
    )
    r_raw, r_runtime, operation = _r_normalized_result(
        definition, generated, trial_index, index_plan=index_plan
    )
    r_result = _product_comparison_projection(r_raw, family=str(definition["family"]))
    comparison_rows: list[dict[str, object]] = []
    all_differences: list[dict[str, object]] = []
    first_product: dict[str, object] | None = None
    for row in worker_runs:
        expected_worker_fields = {
            "workers",
            "worker_axis",
            "seed",
            "relation_identities",
            "production_result",
        }
        if bool(definition["stochastic"]):
            expected_worker_fields.add("bootstrap_authority_receipt")
        if (
            not isinstance(row, Mapping)
            or set(row) != expected_worker_fields
            or row["seed"] != expected_seed
            or type(row["workers"]) is not int
            or row["workers"] < 1
            or (row["worker_axis"] == "not_applicable" and row["workers"] != 1)
            or (
                row["worker_axis"] in {"1", "2", "4"}
                and row["workers"] != int(row["worker_axis"])
            )
            or (
                row["worker_axis"] == "max"
                and row["workers"] != bundle["maximum_available_workers"]
            )
        ):
            raise ArtifactTamperError("current-product worker row differs")
        normalized, receipts = _normalize_production_result(
            definition, generated, row, index_plan
        )
        product = _product_comparison_projection(
            normalized, family=str(definition["family"])
        )
        bootstrap_receipt = receipts.get("bootstrap")
        if isinstance(bootstrap_receipt, Mapping) and index_plan is not None:
            plan_operation = str(bundle["bootstrap_index_plan"]["operation"])
            if bootstrap_receipt.get("operation") != plan_operation:
                raise ArtifactTamperError(
                    "production result and exact bootstrap index operation differ"
                )
        python_differences = _compare_trees(product, python_result)
        r_differences = _compare_trees(product, r_result)
        replay_differences = (
            [] if first_product is None else _compare_trees(product, first_product)
        )
        if first_product is None:
            first_product = product
        all_differences.extend(
            {**difference, "comparison": comparison, "workers": row["workers"]}
            for comparison, differences in (
                ("product_vs_python", python_differences),
                ("product_vs_r", r_differences),
                ("product_worker_replay", replay_differences),
            )
            for difference in differences
        )
        comparison_rows.append(
            {
                "workers": row["workers"],
                "worker_axis": row["worker_axis"],
                "product_result_sha256": canonical_sha256(product),
                "product_vs_python_difference_count": len(python_differences),
                "product_vs_r_difference_count": len(r_differences),
                "worker_replay_difference_count": len(replay_differences),
                "production_receipts": receipts,
            }
        )
    observation = {
        "operation": operation,
        "cell_id": definition["cell_id"],
        "method_version": definition["method_version"],
        "analytical_method_version": definition["analytical_method_version"],
        "scenario_seed": bundle["scenario_seed"],
        "bootstrap_seed": bundle["bootstrap_seed"],
        "bootstrap_index_plan_sha256": (
            canonical_sha256(bundle["bootstrap_index_plan"])
            if index_plan is not None
            else None
        ),
        "cargo_invocation": bundle["cargo_invocation"],
        "producer_executable_sha256": bundle["producer_executable_sha256"],
        "maximum_available_workers": bundle["maximum_available_workers"],
        "producer_contract_version": bundle["producer_contract_version"],
        "execution_receipt_sha256": execution_receipt["receipt_sha256"],
        "execution_nonce": execution_receipt["execution_nonce"],
        "product_source_set_sha256": execution_receipt["source_receipt"][
            "source_set_sha256"
        ],
        "executable_descriptor": execution_receipt["executable_descriptor"],
        "execution_receipt": execution_receipt,
        "worker_comparisons": comparison_rows,
        "difference_count": len(all_differences),
        "maximum_absolute_difference": _maximum_absolute_numeric_difference(
            all_differences
        ),
        "difference_witnesses": all_differences[:20],
        "python_result_sha256": canonical_sha256(python_result),
        "r_result_sha256": canonical_sha256(r_result),
        "r_runtime": r_runtime,
        "required_bindings": plan["current_product_comparison_contract"][
            "required_bindings"
        ],
    }
    product_bootstrap_receipts = [
        row["production_receipts"]["bootstrap"]
        for row in comparison_rows
        if isinstance(row.get("production_receipts"), Mapping)
        and isinstance(row["production_receipts"].get("bootstrap"), Mapping)
    ]
    product_bootstrap_failure_totals: dict[str, int] = {}
    for receipt in product_bootstrap_receipts:
        for reason, count in receipt["failure_reason_totals"].items():
            product_bootstrap_failure_totals[str(reason)] = (
                product_bootstrap_failure_totals.get(str(reason), 0) + int(count)
            )
    body: dict[str, object] = {
        "schema_version": 1,
        "kind": SHARD_KIND,
        "matrix_version": MATRIX_VERSION,
        "integrity_scope": INTEGRITY_SCOPE,
        "status": "complete",
        "qualification_ready": False,
        "identity_receipt": None,
        "source_set_sha256": None,
        "plan_sha256": plan["plan_sha256"],
        "shard_id": shard_id,
        "scenario_id": definition["scenario_id"],
        "scenario_sha256": shard["scenario_sha256"],
        "trial_start_inclusive": shard["trial_start_inclusive"],
        "trial_stop_exclusive": shard["trial_stop_exclusive"],
        "first_trial_seed": shard["first_trial_seed"],
        "last_trial_seed": shard["last_trial_seed"],
        "attempted_trials": 1,
        "metric_counts": {
            "deterministic_contract_rate": {
                "event_count": int(not all_differences),
                "eligible_count": 1,
            }
        },
        "unexpected_failure_count": 0,
        "unexpected_failure_witnesses": [],
        "typed_failure_totals": {
            "unexpected_errors": {},
            "classified_failures": [],
            "bootstrap_failures": dict(
                sorted(product_bootstrap_failure_totals.items())
            ),
        },
        "comparison_summary": {
            "observation_count": len(comparison_rows) * 3,
            "difference_count": len(all_differences),
            "maximum_absolute_difference": observation["maximum_absolute_difference"],
        },
        "worker_ordering_summary": {
            "comparison_count": len(comparison_rows),
            "equal_count": sum(
                row["worker_replay_difference_count"] == 0 for row in comparison_rows
            ),
            "unequal_count": sum(
                row["worker_replay_difference_count"] != 0 for row in comparison_rows
            ),
        },
        "bootstrap_totals": {
            "observation_count": len(product_bootstrap_receipts),
            "requested": sum(
                int(receipt["requested"]) for receipt in product_bootstrap_receipts
            ),
            "usable": sum(
                int(receipt["usable"]) for receipt in product_bootstrap_receipts
            ),
            "failed": sum(
                int(receipt["failed"]) for receipt in product_bootstrap_receipts
            ),
        },
        "recovery_moments": [],
        "first_observation": observation,
        "last_observation": observation,
        "ordered_trial_results_sha256": canonical_sha256(
            {
                "bundle": bundle,
                "execution_receipt": execution_receipt,
            }
        ),
    }
    artifact = {**body, "artifact_sha256": canonical_sha256(body)}
    validate_shard_artifact(artifact, plan, shard)
    destination = shard_path(output_root, shard_id)
    if destination.exists():
        existing = load_validated_shard(destination, plan, shard)
        if canonical_bytes(existing) != canonical_bytes(artifact):
            raise ArtifactTamperError(
                "accepted product shard differs; overwrite refused"
            )
        return destination
    _exclusive_atomic_publish(destination, canonical_bytes(artifact))
    return destination


def _performance_case_id(definition: Mapping[str, object]) -> tuple[str, str]:
    suite = str(definition["suite"])
    if suite == "compound_stress":
        return "compound_stress", "compound_stress"
    if suite != "maximum_axis":
        raise PlanValidationError("scenario is not delegated to the performance lane")
    dimension = str(definition["contract_combination_id"]).removeprefix("maximum_")
    case_ids = {
        "rows": "maximum_rows_100000",
        "indicators": "maximum_indicators_300",
        "constructs": "maximum_constructs_100",
        "resamples": "maximum_resamples_10000",
    }
    if dimension not in case_ids:
        raise PlanValidationError("maximum-axis scenario dimension is unknown")
    return "maximum_axis", case_ids[dimension]


def _qualification_cell_key_for_reference(
    reference: Mapping[str, object],
) -> str:
    matches = [
        key
        for key, contract in CELL_CONTRACTS.items()
        if reference
        == {
            "registry_schema_version": 2,
            "capability_id": contract.capability_id,
            "cell_id": contract.cell_id,
            "capability_version": contract.method_version,
        }
    ]
    if len(matches) != 1:
        raise PlanValidationError(
            "performance row does not resolve to one exact Rank 0 cell"
        )
    return matches[0]


def _validate_cell_specific_performance_workload(
    row: Mapping[str, object],
) -> None:
    reference = row.get("capability_reference")
    workload = row.get("workload")
    if not isinstance(reference, Mapping) or not isinstance(workload, Mapping):
        raise PlanValidationError("performance workload authority is incomplete")
    if set(workload) != {
        "rows",
        "indicators",
        "constructs",
        "resamples",
        "groups",
        "candidate_models",
    } or any(type(value) is not int for value in workload.values()):
        raise PlanValidationError("performance workload fields are not exact integers")
    cell_key = _qualification_cell_key_for_reference(reference)
    spec = build_qualification_spec(cell_key)
    profiles = {
        profile["id"]: profile["workload"]
        for profile in spec["scenario_contract"]["complexity_profiles"]
    }
    profile_id = str(row.get("profile_id"))
    case_id = str(row.get("case_id"))
    if profile_id not in profiles:
        raise PlanValidationError(
            "performance workload profile is absent from the QualificationSpec"
        )
    frozen = profiles[profile_id]
    if profile_id != "maximum_axis":
        if dict(workload) != frozen:
            raise PlanValidationError(
                "performance workload differs from its cell-specific QualificationSpec"
            )
    else:
        stressed_dimensions = {
            "maximum_rows_100000": "rows",
            "maximum_indicators_300": "indicators",
            "maximum_constructs_100": "constructs",
            "maximum_resamples_10000": "resamples",
        }
        stressed = stressed_dimensions.get(case_id)
        if stressed is None:
            raise PlanValidationError("maximum-axis performance case is not exact")
        if (
            workload[stressed] != frozen[stressed]
            or workload["resamples"] != frozen["resamples"]
            or workload["groups"] != 1
            or workload["candidate_models"] != 1
            or any(
                workload[field] <= 0 or workload[field] > frozen[field]
                for field in ("rows", "indicators", "constructs")
            )
        ):
            raise PlanValidationError(
                "maximum-axis workload violates its cell-specific frozen bounds"
            )
    document = {
        "schema_version": row.get("schema_version"),
        "workload_kind": row.get("workload_kind"),
        "variant_id": row.get("variant_id"),
        "capability_reference": dict(reference),
        "profile_id": profile_id,
        "case_id": case_id,
        "workload": dict(workload),
    }
    if (
        row.get("schema_version") != 1
        or row.get("workload_kind") != "general_sem_rank0_performance_workload"
        or row.get("workload_fingerprint") != canonical_sha256(document)
    ):
        raise PlanValidationError(
            "performance workload fingerprint does not bind its resolved document"
        )


def ingest_external_performance_index(
    plan: Mapping[str, object],
    performance_index_path: Path,
    output_root: Path,
) -> list[Path]:
    """Convert exact validated performance rows into compact bound shards."""

    validate_plan(plan)
    try:
        from general_sem_rank0_performance import (
            HARDWARE_PROFILE_ID as PERFORMANCE_LANE_HARDWARE_PROFILE_ID,
            build_plan as build_performance_plan,
            canonical_sha256 as performance_canonical_sha256,
            validate_method_receipt,
        )
    except ImportError as error:
        raise QualificationRunnerError(
            f"production performance validator is unavailable: {error}"
        ) from error
    loaded = load_json(performance_index_path)
    expected_index_fields = {
        "schema_version",
        "evidence_kind",
        "measurement_role",
        "contract_id",
        "contract_sha256",
        "rank0_contract_sha256",
        "hardware_profile_id",
        "build_fingerprint",
        "generated_at_utc",
        "case_count",
        "cases",
        "passed",
    }
    if (
        not isinstance(loaded, Mapping)
        or set(loaded) != expected_index_fields
        or loaded.get("schema_version") != 1
        or loaded.get("evidence_kind")
        != plan["external_performance_receipt_contract"]["index_kind"]
        or loaded.get("hardware_profile_id")
        != plan["external_performance_receipt_contract"]["required_hardware_profile_id"]
        or loaded.get("passed") is not True
        or not isinstance(loaded.get("cases"), list)
        or loaded.get("case_count") != len(loaded["cases"])
        or not str(loaded.get("build_fingerprint", "")).strip()
    ):
        raise ArtifactTamperError("external performance index contract differs")
    if PERFORMANCE_LANE_HARDWARE_PROFILE_ID != PERFORMANCE_HARDWARE_PROFILE_ID:
        raise PlanValidationError(
            "qualification and performance-lane hardware identities differ"
        )
    manifest, context, expected_rows = build_performance_plan()
    for expected_row in expected_rows:
        _validate_cell_specific_performance_workload(expected_row)
    expected_by_key = {
        (
            row["capability_reference"]["cell_id"],
            row["capability_reference"]["capability_version"],
            row["profile_id"],
            row["case_id"],
        ): row
        for row in expected_rows
    }
    observed_by_key: dict[tuple[str, str, str, str], Mapping[str, object]] = {}
    for row in loaded["cases"]:
        if not isinstance(row, Mapping) or not isinstance(
            row.get("capability_reference"), Mapping
        ):
            raise ArtifactTamperError("external performance case is invalid")
        reference = row["capability_reference"]
        key = (
            str(reference.get("cell_id")),
            str(reference.get("capability_version")),
            str(row.get("profile_id")),
            str(row.get("case_id")),
        )
        if key in observed_by_key:
            raise ArtifactTamperError("external performance case is duplicated")
        observed_by_key[key] = row
    performance_root = performance_index_path.resolve().parent
    performance_sha256 = canonical_sha256(loaded)
    scenario_by_id = _scenario_map(plan)
    delegated = [
        shard
        for shard in plan["shards"]
        if scenario_by_id[str(shard["scenario_id"])]["suite"]
        in {"maximum_axis", "compound_stress"}
    ]
    if len(delegated) != 18:
        raise PlanValidationError(
            "external performance contract requires exactly 18 applicable max/compound shards"
        )
    published: list[Path] = []
    for shard in delegated:
        definition = scenario_by_id[str(shard["scenario_id"])]
        profile_id, case_id = _performance_case_id(definition)
        key = (
            str(definition["cell_id"]),
            str(definition["method_version"]),
            profile_id,
            case_id,
        )
        row = observed_by_key.get(key)
        expected = expected_by_key.get(key)
        if row is None or expected is None:
            raise ArtifactTamperError(
                f"external performance receipt row is missing: {key!r}"
            )
        required_row_fields = {
            "variant_id",
            "capability_reference",
            "profile_id",
            "case_id",
            "workload_fingerprint",
            "warmup_runs",
            "measured_runs",
            "cancellation_observed",
            "memory_soak_accepted_runs",
            "receipt",
            "baseline",
            "receipt_complete",
        }
        descriptor = row.get("receipt")
        if (
            set(row) != required_row_fields
            or row.get("workload_fingerprint") != expected["workload_fingerprint"]
            or row.get("warmup_runs")
            != plan["external_performance_receipt_contract"]["required_warmup_runs"]
            or row.get("measured_runs")
            != plan["external_performance_receipt_contract"]["required_measured_runs"]
            or row.get("receipt_complete") is not True
            or not isinstance(descriptor, Mapping)
            or set(descriptor) != {"path", "size", "sha256"}
        ):
            raise ArtifactTamperError("external performance case binding differs")
        receipt_path = (performance_root / str(descriptor["path"])).resolve()
        try:
            receipt_path.relative_to(performance_root)
        except ValueError as error:
            raise ArtifactTamperError(
                "external performance receipt leaves its result root"
            ) from error
        if (
            not receipt_path.is_file()
            or receipt_path.is_symlink()
            or receipt_path.stat().st_size != descriptor["size"]
            or hashlib.sha256(receipt_path.read_bytes()).hexdigest()
            != descriptor["sha256"]
        ):
            raise ArtifactTamperError("external performance receipt descriptor differs")
        receipt = load_json(receipt_path)
        if not isinstance(receipt, Mapping):
            raise ArtifactTamperError("external performance receipt is not an object")
        baseline: Mapping[str, object] | None = None
        baseline_descriptor = row.get("baseline")
        if baseline_descriptor is not None:
            if not isinstance(baseline_descriptor, Mapping) or set(
                baseline_descriptor
            ) != {"path", "measurement_id", "sha256"}:
                raise ArtifactTamperError(
                    "external performance baseline binding differs"
                )
            baseline_value = load_json(Path(str(baseline_descriptor["path"])))
            if (
                not isinstance(baseline_value, Mapping)
                or performance_canonical_sha256(baseline_value)
                != baseline_descriptor["sha256"]
            ):
                raise ArtifactTamperError(
                    "external performance baseline digest differs"
                )
            baseline = baseline_value
        validate_method_receipt(
            receipt,
            role=str(loaded["measurement_role"]),
            row=expected,
            manifest=manifest,
            context=context,
            baseline=baseline,
        )
        observation = {
            "execution_owner": "production_performance_lane",
            "performance_index_path": str(performance_index_path.resolve()),
            "performance_index_sha256": performance_sha256,
            "build_fingerprint": loaded["build_fingerprint"],
            "profile_id": profile_id,
            "case_id": case_id,
            "workload_fingerprint": row["workload_fingerprint"],
            "receipt_descriptor": dict(descriptor),
            "receipt_canonical_sha256": canonical_sha256(receipt),
            "pure_python_or_r_execution_skipped": True,
        }
        body: dict[str, object] = {
            "schema_version": 1,
            "kind": SHARD_KIND,
            "matrix_version": MATRIX_VERSION,
            "integrity_scope": INTEGRITY_SCOPE,
            "status": "complete",
            "qualification_ready": False,
            "identity_receipt": None,
            "source_set_sha256": None,
            "plan_sha256": plan["plan_sha256"],
            "shard_id": shard["shard_id"],
            "scenario_id": definition["scenario_id"],
            "scenario_sha256": shard["scenario_sha256"],
            "trial_start_inclusive": shard["trial_start_inclusive"],
            "trial_stop_exclusive": shard["trial_stop_exclusive"],
            "first_trial_seed": shard["first_trial_seed"],
            "last_trial_seed": shard["last_trial_seed"],
            "attempted_trials": 1,
            "metric_counts": {
                "deterministic_contract_rate": {
                    "event_count": 1,
                    "eligible_count": 1,
                }
            },
            "unexpected_failure_count": 0,
            "unexpected_failure_witnesses": [],
            "typed_failure_totals": {
                "unexpected_errors": {},
                "classified_failures": [],
                "bootstrap_failures": {},
            },
            "comparison_summary": {
                "observation_count": 0,
                "difference_count": 0,
                "maximum_absolute_difference": 0.0,
            },
            "worker_ordering_summary": {
                "comparison_count": 0,
                "equal_count": 0,
                "unequal_count": 0,
            },
            "bootstrap_totals": {
                "observation_count": 0,
                "requested": 0,
                "usable": 0,
                "failed": 0,
            },
            "recovery_moments": [],
            "first_observation": observation,
            "last_observation": observation,
            "ordered_trial_results_sha256": canonical_sha256(
                {"index": performance_sha256, "case": row}
            ),
        }
        artifact = {**body, "artifact_sha256": canonical_sha256(body)}
        validate_shard_artifact(artifact, plan, shard)
        destination = shard_path(output_root, str(shard["shard_id"]))
        if destination.exists():
            existing = load_validated_shard(destination, plan, shard)
            if canonical_bytes(existing) != canonical_bytes(artifact):
                raise ArtifactTamperError(
                    "accepted performance shard differs; overwrite refused"
                )
        else:
            _exclusive_atomic_publish(destination, canonical_bytes(artifact))
        published.append(destination)
    return published


def _worker_execute_shard(
    plan_path_value: str,
    shard_id: str,
    output_root_value: str,
    stale_claim_seconds: float,
) -> dict[str, object]:
    loaded = load_json(Path(plan_path_value))
    if not isinstance(loaded, Mapping):
        raise PlanValidationError("worker plan is not an object")
    try:
        return execute_shard(
            loaded,
            shard_id,
            Path(output_root_value),
            stale_claim_seconds=stale_claim_seconds,
        )
    except ClaimBusyError:
        return {"shard_id": shard_id, "status": "busy", "path": None}


def run_shards(
    plan_path_value: Path,
    output_root: Path,
    *,
    concurrency: int = DEFAULT_CONCURRENCY,
    suites: Iterable[str] | None = None,
    max_shards: int | None = None,
    required_shard_ids: Iterable[str] | None = None,
    stale_claim_seconds: float = DEFAULT_STALE_CLAIM_SECONDS,
    progress: Callable[[Mapping[str, object]], None] | None = None,
) -> dict[str, object]:
    if not 1 <= concurrency <= MAX_CONCURRENCY:
        raise PlanValidationError(f"concurrency must be in [1, {MAX_CONCURRENCY}]")
    if max_shards is not None and max_shards < 1:
        raise PlanValidationError("max_shards must be positive")
    enforce_resource_guard()
    loaded = load_json(plan_path_value)
    if not isinstance(loaded, Mapping):
        raise PlanValidationError("plan is not an object")
    validate_plan(loaded)
    scenario_by_id = _scenario_map(loaded)
    required_id_filter = (
        None
        if required_shard_ids is None
        else {str(value) for value in required_shard_ids}
    )
    if required_id_filter is not None and not required_id_filter <= set(
        _shard_map(loaded)
    ):
        raise PlanValidationError("required shard selection is outside the plan")
    suite_filter = set(ALL_SUITES if suites is None else suites)
    if suite_filter - set(loaded["included_suites"]):
        raise PlanValidationError("requested suites are outside the plan")
    selected = [
        shard
        for shard in loaded["shards"]
        if scenario_by_id[str(shard["scenario_id"])]["suite"] in suite_filter
        and (required_id_filter is None or str(shard["shard_id"]) in required_id_filter)
    ]
    pending: list[Mapping[str, object]] = []
    accepted_existing = 0
    for shard in selected:
        destination = shard_path(output_root, str(shard["shard_id"]))
        if destination.exists():
            load_validated_shard(destination, loaded, shard)
            accepted_existing += 1
        else:
            pending.append(shard)
    if max_shards is not None:
        pending = pending[:max_shards]
    results: list[Mapping[str, object]] = []
    executor = ProcessPoolExecutor(max_workers=concurrency)
    futures: dict[object, str] = {}
    iterator = iter(pending)
    try:
        for _ in range(min(concurrency, len(pending))):
            shard = next(iterator)
            future = executor.submit(
                _worker_execute_shard,
                str(plan_path_value),
                str(shard["shard_id"]),
                str(output_root),
                stale_claim_seconds,
            )
            futures[future] = str(shard["shard_id"])
        while futures:
            done, _ = wait(tuple(futures), return_when=FIRST_COMPLETED)
            for future in done:
                futures.pop(future)
                result = future.result()
                results.append(result)
                if progress is not None:
                    progress(result)
                enforce_resource_guard()
                try:
                    next_shard = next(iterator)
                except StopIteration:
                    continue
                next_future = executor.submit(
                    _worker_execute_shard,
                    str(plan_path_value),
                    str(next_shard["shard_id"]),
                    str(output_root),
                    stale_claim_seconds,
                )
                futures[next_future] = str(next_shard["shard_id"])
    except BaseException:
        for future in futures:
            future.cancel()
        executor.shutdown(wait=True, cancel_futures=True)
        raise
    else:
        executor.shutdown(wait=True)
    return {
        "selected_shards": len(selected),
        "accepted_existing": accepted_existing,
        "attempted_this_run": len(pending),
        "published_this_run": sum(row["status"] == "published" for row in results),
        "busy_this_run": sum(row["status"] == "busy" for row in results),
        "requires_serialized_cargo_bundle": sum(
            row["status"] == "requires_serialized_cargo_bundle" for row in results
        ),
        "default_concurrency": DEFAULT_CONCURRENCY,
        "used_concurrency": concurrency,
    }


def wilson_interval(
    event_count: int, eligible_count: int, confidence_level: float
) -> tuple[float, float]:
    if (
        type(event_count) is not int
        or type(eligible_count) is not int
        or not 0 <= event_count <= eligible_count
        or eligible_count < 1
        or not 0.0 < confidence_level < 1.0
    ):
        raise PlanValidationError("invalid Wilson interval inputs")
    proportion = event_count / eligible_count
    z = NormalDist().inv_cdf(0.5 + confidence_level / 2.0)
    denominator = 1.0 + z * z / eligible_count
    center = (proportion + z * z / (2.0 * eligible_count)) / denominator
    radius = (
        z
        * math.sqrt(
            proportion * (1.0 - proportion) / eligible_count
            + z * z / (4.0 * eligible_count * eligible_count)
        )
        / denominator
    )
    return max(0.0, center - radius), min(1.0, center + radius)


def _metric_report(
    metric: str,
    counts: Mapping[str, object],
    *,
    completed: bool,
    completed_trials: int,
    required_trials: int,
    thresholds: Mapping[str, object],
) -> dict[str, object]:
    events = int(counts["event_count"])
    eligible = int(counts["eligible_count"])
    if eligible < 1:
        return {
            "metric": metric,
            "status": "incomplete",
            "passed": False,
            "reasons": ["no_eligible_events"],
            "event_count": events,
            "eligible_count": eligible,
        }
    rate = events / eligible
    if metric == "deterministic_contract_rate":
        passed = completed and events == eligible
        return {
            "metric": metric,
            "status": "passed" if passed else "failed" if completed else "incomplete",
            "passed": passed,
            "event_count": events,
            "eligible_count": eligible,
            "rate": rate,
            "acceptance_interval": [1.0, 1.0],
            "confidence_interval": None,
            "confidence_half_width": None,
            "minimum_trial_gate_met": completed_trials >= required_trials,
            "reasons": [] if passed else ["not_every_deterministic_check_passed"],
        }
    threshold_key = METRIC_TO_THRESHOLD.get(metric)
    if threshold_key is None:
        raise PlanValidationError(f"metric {metric!r} has no frozen threshold")
    acceptance = thresholds[threshold_key]
    if not isinstance(acceptance, list) or len(acceptance) != 2:
        raise PlanValidationError("frozen acceptance interval is invalid")
    lower_acceptance, upper_acceptance = map(float, acceptance)
    confidence = float(thresholds["monte_carlo_confidence_level"])
    maximum_half_width = float(thresholds["monte_carlo_maximum_half_width"])
    lower, upper = wilson_interval(events, eligible, confidence)
    half_width = (upper - lower) / 2.0
    minimum_gate = completed_trials >= required_trials
    interval_contained = lower >= lower_acceptance and upper <= upper_acceptance
    rate_inside = lower_acceptance <= rate <= upper_acceptance
    precision_met = half_width <= maximum_half_width
    passed = bool(
        completed
        and minimum_gate
        and interval_contained
        and rate_inside
        and precision_met
    )
    reasons: list[str] = []
    if not completed:
        reasons.append("scenario_shards_incomplete")
    if not minimum_gate:
        reasons.append("minimum_trial_gate_not_met")
    if not rate_inside:
        reasons.append("point_rate_outside_frozen_interval")
    if not interval_contained:
        reasons.append("confidence_interval_not_contained_in_frozen_interval")
    if not precision_met:
        reasons.append("monte_carlo_half_width_exceeds_frozen_maximum")
    return {
        "metric": metric,
        "status": "passed" if passed else "failed" if completed else "incomplete",
        "passed": passed,
        "event_count": events,
        "eligible_count": eligible,
        "rate": rate,
        "acceptance_interval": [lower_acceptance, upper_acceptance],
        "confidence_method": "wilson_score_two_sided_v1",
        "confidence_level": confidence,
        "confidence_interval": [lower, upper],
        "confidence_half_width": half_width,
        "maximum_half_width": maximum_half_width,
        "completed_trials": completed_trials,
        "required_trials": required_trials,
        "minimum_trial_gate_met": minimum_gate,
        "reasons": reasons,
    }


def _expected_metric_eligible_count(
    metric: str, scenario: Mapping[str, object], attempted: int
) -> int:
    if metric in {"effect_recovery_rate", "empirical_coverage"}:
        return attempted * 2
    if metric == "null_rejection_rate":
        return attempted * (1 if scenario["family"] == "mediation" else 2)
    return attempted


def _expected_bootstrap_observation_count(
    scenario: Mapping[str, object], attempted: int, plan: Mapping[str, object]
) -> int:
    suite = str(scenario["suite"])
    if not bool(scenario["stochastic"]):
        return 0
    if suite == "current_product_comparison":
        return len(
            plan["current_product_comparison_contract"][
                "bootstrap_required_worker_axes"
            ]
        )
    if suite in {
        "pairwise",
        "coverage",
        "null_calibration",
        "worker_replay",
        "seed_replay",
    }:
        return attempted
    return 0


def _recovery_report(
    scenario: Mapping[str, object],
    rows: Sequence[Mapping[str, object]],
    *,
    completed: bool,
    completed_trials: int,
    required_trials: int,
    thresholds: Mapping[str, object],
) -> list[dict[str, object]]:
    if scenario["suite"] != "recovery":
        if rows:
            raise ArtifactTamperError("non-recovery scenario carries recovery moments")
        return []
    totals = {
        target_id: {
            "n": 0,
            "failed_count": 0,
            "sum_error": 0.0,
            "sum_squared_error": 0.0,
        }
        for target_id in _recovery_target_ids(str(scenario["family"]))
    }
    for row in rows:
        target_id = str(row["target_id"])
        if target_id not in totals:
            raise ArtifactTamperError("aggregate recovery target differs")
        totals[target_id]["n"] += int(row["n"])
        totals[target_id]["failed_count"] += int(row["failed_count"])
        totals[target_id]["sum_error"] += float(row["sum_error"])
        totals[target_id]["sum_squared_error"] += float(row["sum_squared_error"])
    family = str(scenario["family"])
    bias_maximum = float(thresholds[f"{family}_recovery_absolute_bias_maximum"])
    rmse_maximum = float(thresholds[f"{family}_recovery_rmse_maximum"])
    reports: list[dict[str, object]] = []
    for target_id in sorted(totals):
        values = totals[target_id]
        n = int(values["n"])
        failed = int(values["failed_count"])
        signed_bias = float(values["sum_error"]) / n if n else None
        absolute_bias = abs(signed_bias) if signed_bias is not None else None
        rmse = math.sqrt(float(values["sum_squared_error"]) / n) if n else None
        passed = bool(
            completed
            and completed_trials >= required_trials
            and n == required_trials
            and failed == 0
            and absolute_bias is not None
            and rmse is not None
            and absolute_bias <= bias_maximum
            and rmse <= rmse_maximum
        )
        reasons: list[str] = []
        if not completed:
            reasons.append("scenario_shards_incomplete")
        if completed_trials < required_trials:
            reasons.append("minimum_trial_gate_not_met")
        if n != required_trials or failed:
            reasons.append("continuous_recovery_denominator_incomplete")
        if absolute_bias is None or absolute_bias > bias_maximum:
            reasons.append("absolute_bias_exceeds_frozen_maximum")
        if rmse is None or rmse > rmse_maximum:
            reasons.append("rmse_exceeds_frozen_maximum")
        reports.append(
            {
                "family": family,
                "target_id": target_id,
                "n": n,
                "failed_count": failed,
                "sum_error": values["sum_error"],
                "sum_squared_error": values["sum_squared_error"],
                "signed_bias": signed_bias,
                "absolute_bias": absolute_bias,
                "rmse": rmse,
                "absolute_bias_maximum": bias_maximum,
                "rmse_maximum": rmse_maximum,
                "passed": passed,
                "reasons": reasons,
            }
        )
    return reports


def _compact_shard_ledger_row(
    artifact: Mapping[str, object], scenario: Mapping[str, object]
) -> dict[str, object]:
    return {
        "shard_id": artifact["shard_id"],
        "scenario_id": artifact["scenario_id"],
        "scenario_sha256": artifact["scenario_sha256"],
        "trial_start_inclusive": artifact["trial_start_inclusive"],
        "trial_stop_exclusive": artifact["trial_stop_exclusive"],
        "first_trial_seed": artifact["first_trial_seed"],
        "last_trial_seed": artifact["last_trial_seed"],
        "attempted_trials": artifact["attempted_trials"],
        "metric_counts": artifact["metric_counts"],
        "unexpected_failure_count": artifact["unexpected_failure_count"],
        "typed_failure_totals": artifact["typed_failure_totals"],
        "comparison_summary": artifact["comparison_summary"],
        "worker_ordering_summary": artifact["worker_ordering_summary"],
        "bootstrap_totals": artifact["bootstrap_totals"],
        "recovery_moments": artifact["recovery_moments"],
        "artifact_sha256": artifact["artifact_sha256"],
        "ordered_trial_results_sha256": artifact["ordered_trial_results_sha256"],
        "product_observation": (
            artifact["first_observation"]
            if scenario["suite"] == "current_product_comparison"
            else None
        ),
    }


def _validate_compact_shard_ledger_row(
    row: Mapping[str, object],
    *,
    plan: Mapping[str, object],
    shard: Mapping[str, object],
    scenario: Mapping[str, object],
    accepted_historical_product_source_sha256: str | None = None,
) -> None:
    required = {
        "shard_id",
        "scenario_id",
        "scenario_sha256",
        "trial_start_inclusive",
        "trial_stop_exclusive",
        "first_trial_seed",
        "last_trial_seed",
        "attempted_trials",
        "metric_counts",
        "unexpected_failure_count",
        "typed_failure_totals",
        "comparison_summary",
        "worker_ordering_summary",
        "bootstrap_totals",
        "recovery_moments",
        "artifact_sha256",
        "ordered_trial_results_sha256",
        "product_observation",
    }
    attempted = int(shard["trial_stop_exclusive"]) - int(shard["trial_start_inclusive"])
    if (
        set(row) != required
        or row.get("shard_id") != shard["shard_id"]
        or row.get("scenario_id") != shard["scenario_id"]
        or row.get("scenario_sha256") != shard["scenario_sha256"]
        or row.get("trial_start_inclusive") != shard["trial_start_inclusive"]
        or row.get("trial_stop_exclusive") != shard["trial_stop_exclusive"]
        or row.get("first_trial_seed") != shard["first_trial_seed"]
        or row.get("last_trial_seed") != shard["last_trial_seed"]
        or row.get("attempted_trials") != attempted
        or not _strict_sha256(row.get("artifact_sha256"))
        or not _strict_sha256(row.get("ordered_trial_results_sha256"))
    ):
        raise ArtifactTamperError("compact shard ledger identity differs")
    metrics = row.get("metric_counts")
    if not isinstance(metrics, Mapping) or set(metrics) != set(scenario["metrics"]):
        raise ArtifactTamperError("compact shard ledger metric inventory differs")
    for metric, count_row in metrics.items():
        expected_eligible = _expected_metric_eligible_count(
            str(metric), scenario, attempted
        )
        if (
            not isinstance(count_row, Mapping)
            or set(count_row) != {"event_count", "eligible_count"}
            or type(count_row["event_count"]) is not int
            or type(count_row["eligible_count"]) is not int
            or not 0 <= count_row["event_count"] <= count_row["eligible_count"]
            or count_row["eligible_count"] != expected_eligible
        ):
            raise ArtifactTamperError("compact shard ledger metric counts differ")
    unexpected = row.get("unexpected_failure_count")
    typed = row.get("typed_failure_totals")
    comparison = row.get("comparison_summary")
    ordering = row.get("worker_ordering_summary")
    bootstrap = row.get("bootstrap_totals")
    recovery_moments = row.get("recovery_moments")
    if (
        type(unexpected) is not int
        or not 0 <= unexpected <= attempted
        or not isinstance(typed, Mapping)
        or set(typed)
        != {"unexpected_errors", "classified_failures", "bootstrap_failures"}
        or not isinstance(typed.get("unexpected_errors"), Mapping)
        or not isinstance(typed.get("classified_failures"), list)
        or not isinstance(typed.get("bootstrap_failures"), Mapping)
        or not isinstance(comparison, Mapping)
        or set(comparison)
        != {"observation_count", "difference_count", "maximum_absolute_difference"}
        or not isinstance(ordering, Mapping)
        or set(ordering) != {"comparison_count", "equal_count", "unequal_count"}
        or not isinstance(bootstrap, Mapping)
        or set(bootstrap) != {"observation_count", "requested", "usable", "failed"}
        or not isinstance(recovery_moments, list)
    ):
        raise ArtifactTamperError("compact shard ledger summaries differ")
    unexpected_total = 0
    for error_type, count in typed["unexpected_errors"].items():
        if (
            not isinstance(error_type, str)
            or not error_type
            or type(count) is not int
            or count <= 0
        ):
            raise ArtifactTamperError("compact shard unexpected errors differ")
        unexpected_total += count
    classified_total = 0
    prior: tuple[str, str, str] | None = None
    for failure in typed["classified_failures"]:
        if not isinstance(failure, Mapping) or set(failure) != {
            "case",
            "expected_error",
            "observed_error",
            "count",
        }:
            raise ArtifactTamperError("compact shard classified failures differ")
        identity = (
            str(failure["case"]),
            str(failure["expected_error"]),
            str(failure["observed_error"]),
        )
        if (
            prior is not None
            and identity <= prior
            or type(failure["count"]) is not int
            or failure["count"] <= 0
        ):
            raise ArtifactTamperError("compact shard classified ordering differs")
        prior = identity
        classified_total += failure["count"]
    bootstrap_failure_total = 0
    for reason, count in typed["bootstrap_failures"].items():
        if (
            not isinstance(reason, str)
            or not reason
            or type(count) is not int
            or count <= 0
        ):
            raise ArtifactTamperError("compact shard bootstrap failures differ")
        bootstrap_failure_total += count
    integer_summary_values = (
        comparison["observation_count"],
        comparison["difference_count"],
        ordering["comparison_count"],
        ordering["equal_count"],
        ordering["unequal_count"],
        bootstrap["observation_count"],
        bootstrap["requested"],
        bootstrap["usable"],
        bootstrap["failed"],
    )
    maximum = comparison["maximum_absolute_difference"]
    suite = str(scenario["suite"])
    comparison_expected = (
        attempted - unexpected
        if suite in {"independent_oracle_comparison", "metamorphic_invariance"}
        else len(
            plan["current_product_comparison_contract"]["point_required_worker_axes"]
        )
        * 3
        if suite == "current_product_comparison" and not scenario["stochastic"]
        else len(
            plan["current_product_comparison_contract"][
                "bootstrap_required_worker_axes"
            ]
        )
        * 3
        if suite == "current_product_comparison"
        else 0
    )
    ordering_expected = (
        attempted - unexpected
        if suite in {"worker_replay", "seed_replay"}
        else len(
            plan["current_product_comparison_contract"]["point_required_worker_axes"]
        )
        if suite == "current_product_comparison" and not scenario["stochastic"]
        else len(
            plan["current_product_comparison_contract"][
                "bootstrap_required_worker_axes"
            ]
        )
        if suite == "current_product_comparison"
        else 0
    )
    bootstrap_expected = _expected_bootstrap_observation_count(
        scenario, attempted - unexpected, plan
    )
    expected_requested = bootstrap_expected * int(scenario["workload"]["resamples"])
    metric_row = next(iter(metrics.values()))
    if (
        any(type(value) is not int or value < 0 for value in integer_summary_values)
        or not isinstance(maximum, (int, float))
        or not math.isfinite(float(maximum))
        or maximum < 0
        or (comparison["difference_count"] == 0 and float(maximum) != 0.0)
        or comparison["observation_count"] != comparison_expected
        or ordering["comparison_count"] != ordering_expected
        or bootstrap["observation_count"] != bootstrap_expected
        or bootstrap["requested"] != expected_requested
        or ordering["equal_count"] + ordering["unequal_count"]
        != ordering["comparison_count"]
        or bootstrap["usable"] + bootstrap["failed"] != bootstrap["requested"]
        or bootstrap_failure_total != bootstrap["failed"]
        or unexpected_total != unexpected
        or (
            scenario["suite"] == "failure_classification"
            and classified_total + unexpected != attempted
        )
        or (scenario["suite"] != "failure_classification" and classified_total != 0)
        or (
            suite in {"independent_oracle_comparison", "metamorphic_invariance"}
            and unexpected == 0
            and (
                (metric_row["event_count"] == metric_row["eligible_count"])
                != (comparison["difference_count"] == 0)
            )
        )
        or (
            suite in {"worker_replay", "seed_replay"}
            and ordering["equal_count"] != metric_row["event_count"]
        )
        or (
            bootstrap["requested"] > 0
            and metric_row["event_count"] == metric_row["eligible_count"]
            and bootstrap["usable"] * 10 < bootstrap["requested"] * 9
        )
    ):
        raise ArtifactTamperError("compact shard ledger accounting differs")
    expected_targets = (
        _recovery_target_ids(str(scenario["family"])) if suite == "recovery" else ()
    )
    if len(recovery_moments) != len(expected_targets) or [
        moment.get("target_id")
        for moment in recovery_moments
        if isinstance(moment, Mapping)
    ] != list(expected_targets):
        raise ArtifactTamperError("compact recovery moment inventory differs")
    for moment in recovery_moments:
        if (
            not isinstance(moment, Mapping)
            or set(moment)
            != {"target_id", "n", "failed_count", "sum_error", "sum_squared_error"}
            or type(moment["n"]) is not int
            or type(moment["failed_count"]) is not int
            or moment["n"] < 0
            or moment["failed_count"] < 0
            or moment["n"] + moment["failed_count"] != attempted
            or not isinstance(moment["sum_error"], (int, float))
            or not isinstance(moment["sum_squared_error"], (int, float))
            or not math.isfinite(float(moment["sum_error"]))
            or not math.isfinite(float(moment["sum_squared_error"]))
            or moment["sum_squared_error"] < 0
        ):
            raise ArtifactTamperError("compact recovery moment accounting differs")
    product_observation = row.get("product_observation")
    if scenario["suite"] == "current_product_comparison":
        _validate_embedded_product_observation(
            product_observation,
            plan=plan,
            shard=shard,
            accepted_historical_source_sha256=(
                accepted_historical_product_source_sha256
            ),
        )
        product_bootstraps = [
            worker["production_receipts"]["bootstrap"]
            for worker in product_observation["worker_comparisons"]
            if worker["production_receipts"]["bootstrap"] is not None
        ]
        if not isinstance(product_observation, Mapping) or (
            comparison["difference_count"] != product_observation["difference_count"]
            or float(comparison["maximum_absolute_difference"])
            != float(product_observation["maximum_absolute_difference"])
            or ordering["comparison_count"]
            != len(product_observation["worker_comparisons"])
            or ordering["equal_count"]
            != sum(
                worker["worker_replay_difference_count"] == 0
                for worker in product_observation["worker_comparisons"]
            )
            or bootstrap["observation_count"] != len(product_bootstraps)
            or bootstrap["requested"]
            != sum(receipt["requested"] for receipt in product_bootstraps)
            or bootstrap["usable"]
            != sum(receipt["usable"] for receipt in product_bootstraps)
            or bootstrap["failed"]
            != sum(receipt["failed"] for receipt in product_bootstraps)
        ):
            raise ArtifactTamperError("compact product comparison summary differs")
    elif product_observation is not None:
        raise ArtifactTamperError("non-product ledger row carries product evidence")


def aggregate_plan(
    plan: Mapping[str, object],
    output_root: Path,
    *,
    plan4b_policy: Mapping[str, object] | None = None,
) -> dict[str, object]:
    """Validate and combine every shard independently of completion order."""

    validate_plan(plan)
    if plan4b_policy is not None:
        validate_plan4b_policy(plan4b_policy, plan, output_root=output_root)
    enforce_resource_guard()
    scenario_by_id = _scenario_map(plan)
    required_ids = (
        {str(value) for value in plan4b_policy["required_shard_ids"]}
        if plan4b_policy is not None
        else {str(shard["shard_id"]) for shard in plan["shards"]}
    )
    shards_by_scenario: dict[str, list[Mapping[str, object]]] = {
        scenario_id: [] for scenario_id in scenario_by_id
    }
    missing_shards: list[str] = []
    accepted_shards: list[str] = []
    artifacts: dict[str, Mapping[str, object]] = {}
    current_product_source_set_sha256: str | None = None
    for shard in sorted(plan["shards"], key=lambda row: str(row["shard_id"])):
        shard_id = str(shard["shard_id"])
        if shard_id not in required_ids:
            continue
        shards_by_scenario[str(shard["scenario_id"])].append(shard)
        path = shard_path(output_root, shard_id)
        if not path.exists():
            missing_shards.append(shard_id)
            continue
        artifact = load_validated_shard(path, plan, shard)
        scenario = scenario_by_id[str(shard["scenario_id"])]
        if scenario["suite"] == "current_product_comparison":
            if current_product_source_set_sha256 is None:
                current_product_source_set_sha256 = str(
                    _product_source_receipt()["source_set_sha256"]
                )
            observation = artifact.get("first_observation")
            if (
                not isinstance(observation, Mapping)
                or observation.get("product_source_set_sha256")
                != current_product_source_set_sha256
            ):
                raise ArtifactTamperError(
                    "accepted product shard differs from the current Rust/Cargo source set"
                )
        artifacts[shard_id] = artifact
        accepted_shards.append(shard_id)
    shard_content_ledger = [
        _compact_shard_ledger_row(
            artifacts[str(shard["shard_id"])],
            scenario_by_id[str(shard["scenario_id"])],
        )
        for shard in plan["shards"]
        if str(shard["shard_id"]) in artifacts
    ]
    for row in shard_content_ledger:
        shard = _shard_map(plan)[str(row["shard_id"])]
        _validate_compact_shard_ledger_row(
            row,
            plan=plan,
            shard=shard,
            scenario=scenario_by_id[str(shard["scenario_id"])],
        )
    return _aggregate_from_compact_ledger(
        plan, shard_content_ledger, plan4b_policy=plan4b_policy
    )


def _aggregate_from_compact_ledger(
    plan: Mapping[str, object],
    ledger: object,
    *,
    plan4b_policy: Mapping[str, object] | None = None,
    accepted_historical_product_source_sha256: str | None = None,
) -> dict[str, object]:
    validate_plan(plan)
    if plan4b_policy is not None:
        validate_plan4b_policy(plan4b_policy, plan)
    if not isinstance(ledger, list):
        raise ArtifactTamperError("aggregate compact shard ledger is absent")
    scenario_by_id = _scenario_map(plan)
    shard_by_id = _shard_map(plan)
    required_ids = (
        {str(value) for value in plan4b_policy["required_shard_ids"]}
        if plan4b_policy is not None
        else set(shard_by_id)
    )
    scenario_targets = (
        {str(row["scenario_id"]): row for row in plan4b_policy["scenario_targets"]}
        if plan4b_policy is not None
        else {}
    )
    ledger_by_id: dict[str, Mapping[str, object]] = {}
    ledger_ids: list[str] = []
    for row in ledger:
        if not isinstance(row, Mapping) or not isinstance(row.get("shard_id"), str):
            raise ArtifactTamperError("aggregate compact shard ledger row is invalid")
        shard_id = str(row["shard_id"])
        if (
            shard_id in ledger_by_id
            or shard_id not in shard_by_id
            or shard_id not in required_ids
        ):
            raise ArtifactTamperError("aggregate compact shard identity differs")
        shard = shard_by_id[shard_id]
        scenario = scenario_by_id[str(shard["scenario_id"])]
        _validate_compact_shard_ledger_row(
            row,
            plan=plan,
            shard=shard,
            scenario=scenario,
            accepted_historical_product_source_sha256=(
                accepted_historical_product_source_sha256
            ),
        )
        ledger_by_id[shard_id] = row
        ledger_ids.append(shard_id)
    expected_order = [
        str(shard["shard_id"])
        for shard in plan["shards"]
        if str(shard["shard_id"]) in required_ids
        and str(shard["shard_id"]) in ledger_by_id
    ]
    if ledger_ids != expected_order:
        raise ArtifactTamperError("aggregate compact shard ledger order differs")
    missing_shards = [
        str(shard["shard_id"])
        for shard in plan["shards"]
        if str(shard["shard_id"]) in required_ids
        and str(shard["shard_id"]) not in ledger_by_id
    ]
    scenario_reports: list[dict[str, object]] = []
    for scenario_id in sorted(scenario_by_id):
        scenario = scenario_by_id[scenario_id]
        if plan4b_policy is not None and scenario_id not in scenario_targets:
            continue
        expected_shards = [
            shard
            for shard in plan["shards"]
            if shard["scenario_id"] == scenario_id
            and str(shard["shard_id"]) in required_ids
        ]
        present = [
            ledger_by_id[str(shard["shard_id"])]
            for shard in expected_shards
            if str(shard["shard_id"]) in ledger_by_id
        ]
        completed = len(present) == len(expected_shards)
        metric_counts = {
            str(metric): {"event_count": 0, "eligible_count": 0}
            for metric in scenario["metrics"]
        }
        completed_trials = 0
        unexpected_failures = 0
        recovery_rows: list[Mapping[str, object]] = []
        for row in present:
            completed_trials += int(row["attempted_trials"])
            unexpected_failures += int(row["unexpected_failure_count"])
            for metric, counts in row["metric_counts"].items():
                metric_counts[str(metric)]["event_count"] += int(counts["event_count"])
                metric_counts[str(metric)]["eligible_count"] += int(
                    counts["eligible_count"]
                )
            recovery_rows.extend(row["recovery_moments"])
        required_trials = (
            int(scenario_targets[scenario_id]["required_trial_count"])
            if plan4b_policy is not None
            else (
                int(plan["qualification_trials"])
                if scenario["suite"] in TRIAL_SUITES
                else int(scenario["trial_count"])
            )
        )
        reports = [
            _metric_report(
                metric,
                metric_counts[metric],
                completed=completed,
                completed_trials=completed_trials,
                required_trials=required_trials,
                thresholds=plan["frozen_thresholds"],
            )
            for metric in sorted(metric_counts)
        ]
        recovery_reports = _recovery_report(
            scenario,
            recovery_rows,
            completed=completed,
            completed_trials=completed_trials,
            required_trials=required_trials,
            thresholds=plan["frozen_thresholds"],
        )
        scenario_passed = completed and all(
            report["passed"] for report in [*reports, *recovery_reports]
        )
        scenario_reports.append(
            {
                "scenario_id": scenario_id,
                "scenario_sha256": canonical_sha256(scenario),
                "suite": scenario["suite"],
                "cell_key": scenario["cell_key"],
                "cell_id": scenario["cell_id"],
                "method_version": scenario["method_version"],
                "analytical_method_version": scenario["analytical_method_version"],
                "status": (
                    "passed"
                    if scenario_passed
                    else "failed"
                    if completed
                    else "incomplete"
                ),
                "passed": scenario_passed,
                "expected_shards": len(expected_shards),
                "accepted_shards": len(present),
                "completed_trials": completed_trials,
                "required_trials": required_trials,
                "unexpected_failure_count": unexpected_failures,
                "metrics": reports,
                "recovery_moments": recovery_reports,
            }
        )
    complete = not missing_shards
    scientific_thresholds_passed = complete and all(
        row["passed"] for row in scenario_reports
    )
    independent_passed = all(
        row["passed"]
        for row in scenario_reports
        if row["suite"] == "independent_oracle_comparison"
    ) and any(
        row["suite"] == "independent_oracle_comparison" for row in scenario_reports
    )
    product_passed = all(
        row["passed"]
        for row in scenario_reports
        if row["suite"] == "current_product_comparison"
    ) and any(row["suite"] == "current_product_comparison" for row in scenario_reports)
    performance_rows = [
        row
        for row in scenario_reports
        if row["suite"] in {"maximum_axis", "compound_stress"}
    ]
    performance_passed = len(performance_rows) == 18 and all(
        row["passed"] for row in performance_rows
    )
    body: dict[str, object] = {
        "schema_version": 2 if plan4b_policy is not None else 1,
        "kind": PLAN4B_AGGREGATE_KIND if plan4b_policy is not None else AGGREGATE_KIND,
        "matrix_version": MATRIX_VERSION,
        "integrity_scope": INTEGRITY_SCOPE,
        "plan_sha256": plan["plan_sha256"],
        "status": (
            "passed"
            if scientific_thresholds_passed
            else "failed"
            if complete
            else "incomplete"
        ),
        "passed": scientific_thresholds_passed,
        "scientific_thresholds_passed": scientific_thresholds_passed,
        "qualification_ready": False,
        "prequalification_only": True,
        "identity_receipts_minted": False,
        "source_hashes_minted": False,
        "registry_promotion_allowed": False,
        "external_r_oracle_supplied": True,
        "independent_oracle_comparison_passed": independent_passed,
        "current_product_comparison_passed": product_passed,
        "current_product_comparison_pending": not product_passed,
        "external_performance_receipt_passed": performance_passed,
        "external_performance_receipt_pending": not performance_passed,
        "expected_shard_count": len(plan["shards"]),
        "accepted_shard_count": len(ledger),
        "missing_shard_ids": missing_shards,
        "accepted_shard_ids_sha256": canonical_sha256(sorted(ledger_ids)),
        "shard_content_ledger": ledger,
        "shard_content_ledger_sha256": canonical_sha256(ledger),
        "scenario_reports": scenario_reports,
        "failure_policy": {
            "missing_invalid_or_tampered_shard": "fail_closed",
            "failed_fits_retained_in_denominator": True,
            "confidence_interval_must_be_contained_in_acceptance_interval": True,
        },
    }
    if plan4b_policy is not None:
        body["plan4b_policy_sha256"] = plan4b_policy["policy_sha256"]
        body["excluded_parent_shard_count"] = len(
            plan4b_policy["excluded_parent_shard_ids"]
        )
    return {**body, "artifact_sha256": canonical_sha256(body)}


def validate_aggregate(
    aggregate: Mapping[str, object],
    plan: Mapping[str, object],
    *,
    plan4b_policy: Mapping[str, object] | None = None,
    accepted_historical_product_source_sha256: str | None = None,
) -> None:
    expected = _aggregate_from_compact_ledger(
        plan,
        aggregate.get("shard_content_ledger"),
        plan4b_policy=plan4b_policy,
        accepted_historical_product_source_sha256=(
            accepted_historical_product_source_sha256
        ),
    )
    if canonical_bytes(aggregate) != canonical_bytes(expected):
        raise ArtifactTamperError(
            "aggregate differs from its exact compact shard ledger recomputation"
        )


def validate_frozen_full_aggregate(
    aggregate: Mapping[str, object], plan: Mapping[str, object]
) -> None:
    """Require exact full-matrix coverage and replayable compact evidence."""

    validate_frozen_full_plan(plan)
    validate_aggregate(aggregate, plan)
    ledger = aggregate.get("shard_content_ledger")
    expected_ids = [str(shard["shard_id"]) for shard in plan["shards"]]
    if (
        not isinstance(ledger, list)
        or [row.get("shard_id") for row in ledger if isinstance(row, Mapping)]
        != expected_ids
        or len(ledger) != len(expected_ids)
        or aggregate.get("expected_shard_count") != len(expected_ids)
        or aggregate.get("accepted_shard_count") != len(expected_ids)
        or aggregate.get("missing_shard_ids") != []
    ):
        raise ArtifactTamperError(
            "aggregate is not the exact frozen full-matrix ledger"
        )


def validate_frozen_plan4b_aggregate(
    aggregate: Mapping[str, object],
    plan: Mapping[str, object],
    plan4b_policy: Mapping[str, object],
) -> None:
    """Require the complete exact Plan 4B continuation ledger."""

    validate_frozen_full_plan(plan)
    validate_plan4b_policy(plan4b_policy, plan)
    validate_aggregate(aggregate, plan, plan4b_policy=plan4b_policy)
    ledger = aggregate.get("shard_content_ledger")
    expected_ids = [str(value) for value in plan4b_policy["required_shard_ids"]]
    if (
        not isinstance(ledger, list)
        or [row.get("shard_id") for row in ledger if isinstance(row, Mapping)]
        != expected_ids
        or len(ledger) != len(expected_ids)
        or aggregate.get("expected_shard_count") != len(expected_ids)
        or aggregate.get("accepted_shard_count") != len(expected_ids)
        or aggregate.get("missing_shard_ids") != []
        or aggregate.get("plan4b_policy_sha256") != plan4b_policy["policy_sha256"]
    ):
        raise ArtifactTamperError(
            "aggregate is not the exact frozen Plan 4B continuation ledger"
        )


def publish_aggregate(
    aggregate: Mapping[str, object],
    plan: Mapping[str, object],
    output_root: Path,
    *,
    plan4b_policy: Mapping[str, object] | None = None,
) -> Path:
    validate_aggregate(aggregate, plan, plan4b_policy=plan4b_policy)
    digest = str(aggregate["artifact_sha256"])
    path = output_root / "aggregates" / f"aggregate-{digest}.json"
    _exclusive_atomic_publish(path, canonical_bytes(aggregate))
    loaded = load_json(path)
    if not isinstance(loaded, Mapping):
        raise ArtifactTamperError("published aggregate is not an object")
    validate_aggregate(loaded, plan, plan4b_policy=plan4b_policy)
    return path


def _canonical_pretty(value: object) -> str:
    return json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + "\n"


def _load_plan_path(path: Path) -> Mapping[str, object]:
    loaded = load_json(path)
    if not isinstance(loaded, Mapping):
        raise PlanValidationError("plan is not an object")
    validate_plan(loaded)
    return loaded


def _progress_to_stderr(result: Mapping[str, object]) -> None:
    print(
        f"{result['shard_id']}: {result['status']}",
        file=sys.stderr,
        flush=True,
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--output-root",
        type=Path,
        default=DEFAULT_OUTPUT_ROOT,
        help="Dedicated prequalification result directory.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("plan", help="Create or verify the full frozen plan.")
    subparsers.add_parser(
        "plan4b", help="Create or verify the frozen Plan 4B continuation policy."
    )
    run_parser = subparsers.add_parser("run", help="Execute missing shards.")
    run_parser.add_argument("--concurrency", type=int, default=DEFAULT_CONCURRENCY)
    run_parser.add_argument(
        "--suite", action="append", choices=ALL_SUITES, dest="suites"
    )
    run_parser.add_argument("--max-shards", type=int)
    run_parser.add_argument(
        "--stale-claim-seconds", type=float, default=DEFAULT_STALE_CLAIM_SECONDS
    )
    run4b_parser = subparsers.add_parser(
        "run-plan4b", help="Execute only missing shards required by Plan 4B."
    )
    run4b_parser.add_argument("--concurrency", type=int, default=DEFAULT_CONCURRENCY)
    run4b_parser.add_argument(
        "--suite", action="append", choices=ALL_SUITES, dest="suites"
    )
    run4b_parser.add_argument("--max-shards", type=int)
    run4b_parser.add_argument(
        "--stale-claim-seconds", type=float, default=DEFAULT_STALE_CLAIM_SECONDS
    )
    subparsers.add_parser(
        "aggregate", help="Validate shards and publish a content-addressed report."
    )
    subparsers.add_parser("status", help="Print an aggregate without publishing it.")
    subparsers.add_parser(
        "aggregate-plan4b",
        help="Validate required Plan 4B shards and publish a report.",
    )
    subparsers.add_parser(
        "status-plan4b", help="Print the Plan 4B aggregate without publishing it."
    )
    subparsers.add_parser(
        "product-requests",
        help="Materialize exclusive inputs for the serialized root-owned Cargo runner.",
    )
    ingest_parser = subparsers.add_parser(
        "ingest-product",
        help="Validate one normalized Cargo bundle against Python and R.",
    )
    ingest_parser.add_argument("--bundle", type=Path, required=True)
    ingest_parser.add_argument("--execution-receipt", type=Path, required=True)
    product_run_parser = subparsers.add_parser(
        "run-product",
        help="Run and ingest one serialized production-API comparison shard.",
    )
    product_run_parser.add_argument("--shard-id", required=True)
    product_run_parser.add_argument("--cargo-program", default="cargo")
    product_run_parser.add_argument("--timeout-seconds", type=int, default=3_600)
    performance_parser = subparsers.add_parser(
        "ingest-performance",
        help="Bind exact maximum-axis/compound rows from the production performance lane.",
    )
    performance_parser.add_argument("--performance-index", type=Path, required=True)
    arguments = parser.parse_args(argv)
    output_root = arguments.output_root.resolve()
    enforce_resource_guard()
    plan_path_value = output_root / PLAN_FILENAME
    if arguments.command == "plan":
        plan = build_plan()
        path = publish_plan(plan, output_root)
        print(
            _canonical_pretty(
                {
                    "path": str(path),
                    "plan_sha256": plan["plan_sha256"],
                    "scenarios": len(plan["scenarios"]),
                    "shards": len(plan["shards"]),
                    "default_concurrency": DEFAULT_CONCURRENCY,
                    "qualification_ready": False,
                }
            ),
            end="",
        )
        return 0
    if not plan_path_value.exists():
        publish_plan(build_plan(), output_root)
    plan = _load_plan_path(plan_path_value)
    if arguments.command == "plan4b":
        policy = build_plan4b_policy(plan, output_root)
        path = publish_plan4b_policy(policy, plan, output_root)
        print(
            _canonical_pretty(
                {
                    "path": str(path),
                    "parent_plan_sha256": plan["plan_sha256"],
                    "policy_sha256": policy["policy_sha256"],
                    "execution_inventory": policy["execution_inventory"],
                    "qualification_ready": False,
                }
            ),
            end="",
        )
        return 0
    plan4b_policy: Mapping[str, object] | None = None
    if arguments.command in {"run-plan4b", "aggregate-plan4b", "status-plan4b"}:
        policy_path = output_root / PLAN4B_POLICY_FILENAME
        if not policy_path.exists():
            publish_plan4b_policy(
                build_plan4b_policy(plan, output_root), plan, output_root
            )
        loaded_policy = load_json(policy_path)
        if not isinstance(loaded_policy, Mapping):
            raise PlanValidationError("Plan 4B policy is not an object")
        validate_plan4b_policy(loaded_policy, plan, output_root=output_root)
        plan4b_policy = loaded_policy
    if arguments.command == "product-requests":
        paths = publish_product_requests(plan, output_root)
        print(
            _canonical_pretty(
                {
                    "request_count": len(paths),
                    "paths": [str(path) for path in paths],
                    "cargo_execution": "serialized_root_owned",
                    "qualification_ready": False,
                }
            ),
            end="",
        )
        return 0
    if arguments.command == "ingest-product":
        loaded_bundle = load_json(arguments.bundle.resolve())
        loaded_execution_receipt = load_json(arguments.execution_receipt.resolve())
        if not isinstance(loaded_bundle, Mapping):
            raise ArtifactTamperError("current-product bundle is not an object")
        if not isinstance(loaded_execution_receipt, Mapping):
            raise ArtifactTamperError(
                "current-product execution receipt is not an object"
            )
        path = ingest_product_bundle(
            plan,
            loaded_bundle,
            output_root,
            execution_receipt=loaded_execution_receipt,
        )
        print(
            _canonical_pretty(
                {
                    "path": str(path),
                    "status": "ingested",
                    "qualification_ready": False,
                }
            ),
            end="",
        )
        return 0
    if arguments.command == "run-product":
        result = run_product_comparison(
            plan,
            arguments.shard_id,
            output_root,
            cargo_program=arguments.cargo_program,
            timeout_seconds=arguments.timeout_seconds,
        )
        print(_canonical_pretty(result), end="")
        return 0
    if arguments.command == "ingest-performance":
        paths = ingest_external_performance_index(
            plan, arguments.performance_index.resolve(), output_root
        )
        print(
            _canonical_pretty(
                {
                    "accepted_shard_count": len(paths),
                    "paths": [str(path) for path in paths],
                    "pure_python_or_r_execution_skipped": True,
                    "qualification_ready": False,
                }
            ),
            end="",
        )
        return 0
    if arguments.command in {"run", "run-plan4b"}:
        report = run_shards(
            plan_path_value,
            output_root,
            concurrency=arguments.concurrency,
            suites=arguments.suites,
            max_shards=arguments.max_shards,
            required_shard_ids=(
                plan4b_policy["required_shard_ids"]
                if plan4b_policy is not None
                else None
            ),
            stale_claim_seconds=arguments.stale_claim_seconds,
            progress=_progress_to_stderr,
        )
        print(_canonical_pretty(report), end="")
        return 0
    aggregate = aggregate_plan(plan, output_root, plan4b_policy=plan4b_policy)
    if arguments.command in {"aggregate", "aggregate-plan4b"}:
        path = publish_aggregate(
            aggregate, plan, output_root, plan4b_policy=plan4b_policy
        )
        result = {
            "path": str(path),
            "artifact_sha256": aggregate["artifact_sha256"],
            "status": aggregate["status"],
            "passed": aggregate["passed"],
            "qualification_ready": False,
        }
        print(_canonical_pretty(result), end="")
    else:
        print(_canonical_pretty(aggregate), end="")
    return 0 if aggregate["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
