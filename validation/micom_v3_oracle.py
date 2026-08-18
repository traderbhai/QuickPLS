#!/usr/bin/env python3
"""Transparent, validation-only MICOM v3 oracle.

The module implements only measurement invariance of composite models (MICOM).
It deliberately does not calculate structural-path permutation MGA and it does
not implement consistent permutation/PLSc.  Step 1 is a qualitative researcher
review gate.  Steps 2 and 3 use a deterministic, group-size-preserving
permutation plan with exactly one attempt per requested replicate.

The numerical PLS adapter is imported from ``micom_v2_reference``.  That module
is an independent NumPy implementation and does not import QuickPLS product
code.  The orchestration, stable partition plan, typed failures, orientation
alignment, hierarchy, and ledger in this file are independent v3 contracts.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import json
import math
import sys
from dataclasses import asdict, dataclass
from enum import Enum
from pathlib import Path
from typing import Any, Hashable, Mapping, Protocol, Sequence

import numpy as np


ROOT = Path(__file__).resolve().parents[1]
VALIDATION = ROOT / "validation"
RESULTS = VALIDATION / "results"
FIXTURE_PATH = RESULTS / "micom_v2_reference.csv"
WORK_REPORT_PATH = (
    RESULTS
    / "method_factory"
    / "micom_v3"
    / "work"
    / "independent_oracle.json"
)

sys.path.insert(0, str(VALIDATION))
import micom_v2_reference as independent_pls  # noqa: E402


ORACLE_VERSION = "micom_numpy_oracle_v3.1.0"
METHOD_VERSION = "micom_v3"
DEFAULT_SEED = 20_260_815
DEFAULT_CONFIDENCE_LEVEL = 0.95
DEFAULT_MINIMUM_GROUP_SIZE = 10
DEFAULT_MAXIMUM_GROUP_RATIO = 10.0
DEFAULT_MINIMUM_USABLE_PERMUTATIONS = 19
NUMERICAL_EPSILON = 1e-12


class MicomFailureCode(str, Enum):
    """Stable failure identities used by qualification and product comparison."""

    CONFIGURAL_REVIEW_REQUIRED = "micom.configural_review_required"
    CONFIGURAL_INVARIANCE_NOT_CONFIRMED = "micom.configural_invariance_not_confirmed"
    INVALID_MATRIX = "micom.invalid_matrix"
    NONFINITE_DATA = "micom.nonfinite_data"
    INVALID_CASE_IDS = "micom.invalid_case_ids"
    GROUP_VALUES_IDENTICAL = "micom.group_values_identical"
    UNEXPECTED_GROUP = "micom.unexpected_group"
    EMPTY_GROUP = "micom.empty_group"
    GROUP_TOO_SMALL = "micom.group_too_small"
    EXTREME_GROUP_IMBALANCE = "micom.extreme_group_imbalance"
    DEGENERATE_INDICATOR = "micom.degenerate_indicator"
    OBSERVED_MODEL_FIT_FAILED = "micom.observed_model_fit_failed"
    SCORE_CONTRACT_INVALID = "micom.score_contract_invalid"
    DEGENERATE_COMPOSITE_SCORE = "micom.degenerate_composite_score"
    ORIENTATION_UNDEFINED = "micom.orientation_undefined"
    INSUFFICIENT_USABLE_PERMUTATIONS = "micom.insufficient_usable_permutations"


class MicomOracleError(ValueError):
    """Typed fail-closed error with JSON-safe diagnostic context."""

    def __init__(
        self,
        code: MicomFailureCode,
        message: str,
        *,
        context: Mapping[str, Any] | None = None,
    ) -> None:
        super().__init__(message)
        self.code = code
        self.context = dict(context or {})

    def as_dict(self) -> dict[str, Any]:
        return {
            "code": self.code.value,
            "message": str(self),
            "context": self.context,
        }


@dataclass(frozen=True)
class ConfiguralReview:
    """Qualitative MICOM Step 1 attestation; never inferred from the data."""

    identical_indicators: bool
    equivalent_indicator_meaning: bool
    identical_coding: bool
    identical_data_treatment: bool
    identical_algorithm_settings: bool
    reviewed_by: str

    def unconfirmed_items(self) -> list[str]:
        fields = (
            "identical_indicators",
            "equivalent_indicator_meaning",
            "identical_coding",
            "identical_data_treatment",
            "identical_algorithm_settings",
        )
        missing = [field for field in fields if not getattr(self, field)]
        if not self.reviewed_by.strip():
            missing.append("reviewed_by")
        return missing


@dataclass(frozen=True)
class MicomConfig:
    permutations: int = 99
    seed: int = DEFAULT_SEED
    confidence_level: float = DEFAULT_CONFIDENCE_LEVEL
    minimum_group_size: int = DEFAULT_MINIMUM_GROUP_SIZE
    maximum_group_ratio: float = DEFAULT_MAXIMUM_GROUP_RATIO
    minimum_usable_permutations: int = DEFAULT_MINIMUM_USABLE_PERMUTATIONS

    def validate(self) -> None:
        if self.permutations < 19:
            raise ValueError("MICOM requires at least 19 requested permutations")
        if not 0.5 < self.confidence_level < 1.0:
            raise ValueError("confidence_level must be strictly between 0.5 and 1")
        if self.minimum_group_size < 2:
            raise ValueError("minimum_group_size must be at least two")
        if not math.isfinite(self.maximum_group_ratio) or self.maximum_group_ratio < 1.0:
            raise ValueError("maximum_group_ratio must be finite and at least one")
        if not 1 <= self.minimum_usable_permutations <= self.permutations:
            raise ValueError(
                "minimum_usable_permutations must be between one and permutations"
            )


class CompositeEstimator(Protocol):
    """Small interface that keeps MICOM orchestration estimator-independent."""

    @property
    def construct_ids(self) -> Sequence[str]: ...

    def fit(self, raw: np.ndarray) -> Any: ...

    def scores(self, raw_pooled: np.ndarray, fit: Any) -> Mapping[str, np.ndarray]: ...


class IndependentPlsEstimator:
    """Adapter for the transparent NumPy PLS reference used by the fixed fixture."""

    def __init__(self, construct_order: Sequence[str] | None = None) -> None:
        known = tuple(construct for construct, _ in independent_pls.CONSTRUCTS)
        requested = tuple(construct_order or known)
        if set(requested) != set(known) or len(requested) != len(known):
            raise ValueError("construct_order must contain every fixture construct once")
        self._construct_ids = requested

    @property
    def construct_ids(self) -> Sequence[str]:
        return self._construct_ids

    def fit(self, raw: np.ndarray) -> independent_pls.Fit:
        return independent_pls.estimate_pls(raw)

    def scores(
        self,
        raw_pooled: np.ndarray,
        fit: independent_pls.Fit,
    ) -> Mapping[str, np.ndarray]:
        indices = {
            construct: index
            for index, (construct, _) in enumerate(independent_pls.CONSTRUCTS)
        }
        return {
            construct: _standardize(
                independent_pls.effective_scores(raw_pooled, fit, indices[construct])
            )
            for construct in self._construct_ids
        }


def _stable_token(value: Hashable) -> str:
    return f"{type(value).__module__}.{type(value).__qualname__}:{value!r}"


def _standardize(values: np.ndarray) -> np.ndarray:
    vector = np.asarray(values, dtype=float)
    centered = vector - float(np.mean(vector))
    scale = float(np.std(vector, ddof=1))
    if not math.isfinite(scale) or scale <= NUMERICAL_EPSILON:
        raise MicomOracleError(
            MicomFailureCode.DEGENERATE_COMPOSITE_SCORE,
            "a composite score has zero or nonfinite variance",
        )
    return centered / scale


def _correlation(left: np.ndarray, right: np.ndarray) -> float:
    left_standardized = _standardize(left)
    right_standardized = _standardize(right)
    value = float(np.dot(left_standardized, right_standardized) / (len(left) - 1))
    return max(-1.0, min(1.0, value))


def _sample_variance(values: np.ndarray) -> float:
    value = float(np.var(values, ddof=1))
    if not math.isfinite(value) or value <= NUMERICAL_EPSILON:
        raise MicomOracleError(
            MicomFailureCode.DEGENERATE_COMPOSITE_SCORE,
            "a group composite score has zero or nonfinite variance",
        )
    return value


def _type7_quantile(values: Sequence[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if not ordered:
        raise ValueError("a quantile requires at least one value")
    position = max(0.0, min(1.0, probability)) * (len(ordered) - 1)
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] + fraction * (ordered[upper] - ordered[lower])


def _lower_tail_p(values: Sequence[float], observed: float) -> float:
    return (sum(value <= observed for value in values) + 1.0) / (len(values) + 1.0)


def _two_tail_p(values: Sequence[float], observed: float) -> float:
    lower = (sum(value <= observed for value in values) + 1.0) / (len(values) + 1.0)
    upper = (sum(value >= observed for value in values) + 1.0) / (len(values) + 1.0)
    return min(1.0, 2.0 * min(lower, upper))


def _canonical_rows(
    raw: np.ndarray,
    labels: Sequence[Hashable],
    case_ids: Sequence[Hashable],
) -> tuple[np.ndarray, np.ndarray, tuple[str, ...]]:
    if len(labels) != len(raw) or len(case_ids) != len(raw):
        raise MicomOracleError(
            MicomFailureCode.INVALID_CASE_IDS,
            "raw rows, group labels, and stable case IDs must have equal length",
        )
    tokens = tuple(_stable_token(value) for value in case_ids)
    if len(set(tokens)) != len(tokens):
        raise MicomOracleError(
            MicomFailureCode.INVALID_CASE_IDS,
            "stable case IDs must be unique",
        )
    order = np.asarray(sorted(range(len(tokens)), key=tokens.__getitem__), dtype=int)
    return raw[order], np.asarray(labels, dtype=object)[order], tuple(tokens[index] for index in order)


def _validate_input(
    raw: np.ndarray,
    labels: np.ndarray,
    group_a: Hashable,
    group_b: Hashable,
    config: MicomConfig,
) -> tuple[np.ndarray, np.ndarray, int, int]:
    matrix = np.asarray(raw, dtype=float)
    if matrix.ndim != 2 or matrix.shape[0] < 2 or matrix.shape[1] < 1:
        raise MicomOracleError(
            MicomFailureCode.INVALID_MATRIX,
            "raw input must be a two-dimensional nonempty observation matrix",
        )
    if not np.all(np.isfinite(matrix)):
        raise MicomOracleError(
            MicomFailureCode.NONFINITE_DATA,
            "MICOM oracle accepts only complete finite raw data",
        )
    if _stable_token(group_a) == _stable_token(group_b):
        raise MicomOracleError(
            MicomFailureCode.GROUP_VALUES_IDENTICAL,
            "Group A and Group B must be different values",
        )
    token_a = _stable_token(group_a)
    token_b = _stable_token(group_b)
    label_tokens = np.asarray([_stable_token(value) for value in labels], dtype=object)
    unexpected = sorted(set(label_tokens) - {token_a, token_b})
    if unexpected:
        raise MicomOracleError(
            MicomFailureCode.UNEXPECTED_GROUP,
            "the selected MICOM pool contains an unexpected group value",
            context={"unexpected_group_tokens": unexpected},
        )
    mask_a = label_tokens == token_a
    mask_b = label_tokens == token_b
    count_a = int(np.count_nonzero(mask_a))
    count_b = int(np.count_nonzero(mask_b))
    if count_a == 0 or count_b == 0:
        raise MicomOracleError(
            MicomFailureCode.EMPTY_GROUP,
            "both selected MICOM groups must contain observations",
            context={"group_a_rows": count_a, "group_b_rows": count_b},
        )
    if min(count_a, count_b) < config.minimum_group_size:
        raise MicomOracleError(
            MicomFailureCode.GROUP_TOO_SMALL,
            "a selected MICOM group is below the declared minimum complete-case size",
            context={
                "group_a_rows": count_a,
                "group_b_rows": count_b,
                "minimum_group_size": config.minimum_group_size,
            },
        )
    ratio = max(count_a, count_b) / min(count_a, count_b)
    if ratio > config.maximum_group_ratio:
        raise MicomOracleError(
            MicomFailureCode.EXTREME_GROUP_IMBALANCE,
            "the group-size ratio exceeds the bounded oracle support policy",
            context={
                "group_a_rows": count_a,
                "group_b_rows": count_b,
                "observed_ratio": ratio,
                "maximum_group_ratio": config.maximum_group_ratio,
                "scientific_note": "This is a bounded implementation guard, not a universal MICOM theorem.",
            },
        )
    variances = np.var(matrix, axis=0, ddof=1)
    degenerate = np.flatnonzero(
        np.logical_or(~np.isfinite(variances), variances <= NUMERICAL_EPSILON)
    )
    if len(degenerate):
        raise MicomOracleError(
            MicomFailureCode.DEGENERATE_INDICATOR,
            "one or more indicators have zero or nonfinite pooled variance",
            context={"column_indices": degenerate.tolist()},
        )
    return matrix, mask_a, count_a, count_b


def _fit_scores(
    estimator: CompositeEstimator,
    raw_fit: np.ndarray,
    raw_pooled: np.ndarray,
    *,
    observed: bool,
) -> dict[str, np.ndarray]:
    try:
        fit = estimator.fit(raw_fit)
        scores = estimator.scores(raw_pooled, fit)
    except MicomOracleError:
        raise
    except (ArithmeticError, RuntimeError, ValueError, np.linalg.LinAlgError) as error:
        code = (
            MicomFailureCode.OBSERVED_MODEL_FIT_FAILED
            if observed
            else MicomFailureCode.SCORE_CONTRACT_INVALID
        )
        raise MicomOracleError(
            code,
            f"composite estimator failed: {type(error).__name__}",
        ) from error
    expected = set(estimator.construct_ids)
    if set(scores) != expected or len(expected) != len(tuple(estimator.construct_ids)):
        raise MicomOracleError(
            MicomFailureCode.SCORE_CONTRACT_INVALID,
            "estimator score identities do not exactly match construct identities",
        )
    result: dict[str, np.ndarray] = {}
    for construct in sorted(expected):
        vector = np.asarray(scores[construct], dtype=float)
        if vector.ndim != 1 or len(vector) != len(raw_pooled) or not np.all(np.isfinite(vector)):
            raise MicomOracleError(
                MicomFailureCode.SCORE_CONTRACT_INVALID,
                "an estimator score vector has an invalid shape or nonfinite value",
                context={"construct": construct},
            )
        try:
            result[construct] = _standardize(vector)
        except MicomOracleError as error:
            error.context["construct"] = construct
            raise
    return result


def _align_to_reference(
    reference: Mapping[str, np.ndarray],
    candidate: Mapping[str, np.ndarray],
) -> tuple[dict[str, np.ndarray], dict[str, int]]:
    aligned: dict[str, np.ndarray] = {}
    signs: dict[str, int] = {}
    for construct in sorted(reference):
        association = _correlation(reference[construct], candidate[construct])
        if abs(association) <= NUMERICAL_EPSILON:
            raise MicomOracleError(
                MicomFailureCode.ORIENTATION_UNDEFINED,
                "a group composite is orthogonal to the pooled orientation reference",
                context={"construct": construct},
            )
        sign = -1 if association < 0.0 else 1
        signs[construct] = sign
        aligned[construct] = candidate[construct] * sign
    return aligned, signs


def _step2_statistics(
    pooled: Mapping[str, np.ndarray],
    group_a: Mapping[str, np.ndarray],
    group_b: Mapping[str, np.ndarray],
) -> tuple[dict[str, float], dict[str, dict[str, int]]]:
    aligned_a, signs_a = _align_to_reference(pooled, group_a)
    aligned_b, signs_b = _align_to_reference(pooled, group_b)
    return (
        {
            construct: _correlation(aligned_a[construct], aligned_b[construct])
            for construct in sorted(pooled)
        },
        {
            construct: {"group_a": signs_a[construct], "group_b": signs_b[construct]}
            for construct in sorted(pooled)
        },
    )


def _step3_statistics(
    pooled: Mapping[str, np.ndarray],
    group_a_mask: np.ndarray,
) -> dict[str, dict[str, float]]:
    result: dict[str, dict[str, float]] = {}
    for construct in sorted(pooled):
        left = pooled[construct][group_a_mask]
        right = pooled[construct][~group_a_mask]
        variance_a = _sample_variance(left)
        variance_b = _sample_variance(right)
        mean_a = float(np.mean(left))
        mean_b = float(np.mean(right))
        result[construct] = {
            "mean_a": mean_a,
            "mean_b": mean_b,
            "mean_difference": mean_a - mean_b,
            "variance_a": variance_a,
            "variance_b": variance_b,
            "log_variance_ratio": math.log(variance_a) - math.log(variance_b),
        }
    return result


def _canonical_partition_mask(
    case_tokens: Sequence[str],
    canonical_group_size: int,
    seed: int,
    replicate: int,
) -> tuple[np.ndarray, str]:
    ranked: list[tuple[bytes, str, int]] = []
    prefix = f"{ORACLE_VERSION}|{seed}|{replicate}|".encode()
    for index, token in enumerate(case_tokens):
        digest = hashlib.blake2b(prefix + token.encode(), digest_size=16).digest()
        ranked.append((digest, token, index))
    ranked.sort()
    selected = sorted(index for _, _, index in ranked[:canonical_group_size])
    mask = np.zeros(len(case_tokens), dtype=bool)
    mask[selected] = True
    identity = hashlib.sha256(
        "\n".join(case_tokens[index] for index in selected).encode()
    ).hexdigest()
    return mask, identity


def run_micom(
    raw: np.ndarray,
    labels: Sequence[Hashable],
    case_ids: Sequence[Hashable],
    *,
    group_a: Hashable,
    group_b: Hashable,
    configural_review: ConfiguralReview | None,
    estimator: CompositeEstimator,
    config: MicomConfig = MicomConfig(),
) -> dict[str, Any]:
    """Run the exact bounded MICOM contract and return its complete ledger."""

    config.validate()
    if configural_review is None:
        raise MicomOracleError(
            MicomFailureCode.CONFIGURAL_REVIEW_REQUIRED,
            "MICOM Step 1 requires an explicit qualitative configural review",
        )
    unconfirmed = configural_review.unconfirmed_items()
    if unconfirmed:
        raise MicomOracleError(
            MicomFailureCode.CONFIGURAL_INVARIANCE_NOT_CONFIRMED,
            "MICOM cannot continue because the Step 1 review is incomplete",
            context={"unconfirmed_items": unconfirmed},
        )

    matrix = np.asarray(raw, dtype=float)
    matrix, ordered_labels, case_tokens = _canonical_rows(matrix, labels, case_ids)
    matrix, observed_a_mask, count_a, count_b = _validate_input(
        matrix, ordered_labels, group_a, group_b, config
    )
    pooled_scores = _fit_scores(estimator, matrix, matrix, observed=True)
    observed_scores_a = _fit_scores(
        estimator, matrix[observed_a_mask], matrix, observed=True
    )
    observed_scores_b = _fit_scores(
        estimator, matrix[~observed_a_mask], matrix, observed=True
    )
    observed_step2, observed_signs = _step2_statistics(
        pooled_scores, observed_scores_a, observed_scores_b
    )
    observed_step3 = _step3_statistics(pooled_scores, observed_a_mask)

    token_a = _stable_token(group_a)
    token_b = _stable_token(group_b)
    canonical_token = min(token_a, token_b)
    canonical_group_size = count_a if token_a == canonical_token else count_b
    requested_a_is_canonical = token_a == canonical_token

    correlation_distributions = {construct: [] for construct in sorted(pooled_scores)}
    mean_distributions = {construct: [] for construct in sorted(pooled_scores)}
    variance_distributions = {construct: [] for construct in sorted(pooled_scores)}
    ledger: list[dict[str, Any]] = []

    for replicate in range(config.permutations):
        canonical_mask, partition_sha256 = _canonical_partition_mask(
            case_tokens,
            canonical_group_size,
            config.seed,
            replicate,
        )
        permutation_a_mask = canonical_mask if requested_a_is_canonical else ~canonical_mask
        row: dict[str, Any] = {
            "replicate": replicate,
            "partition_sha256": partition_sha256,
            "group_a_rows": int(np.count_nonzero(permutation_a_mask)),
            "group_b_rows": int(np.count_nonzero(~permutation_a_mask)),
            "step2_status": "pending",
            "step2_failure_code": None,
            "step3_status": "pending",
            "step3_failure_code": None,
        }
        try:
            permutation_scores_a = _fit_scores(
                estimator, matrix[permutation_a_mask], matrix, observed=False
            )
            permutation_scores_b = _fit_scores(
                estimator, matrix[~permutation_a_mask], matrix, observed=False
            )
            step2, _ = _step2_statistics(
                pooled_scores, permutation_scores_a, permutation_scores_b
            )
            for construct, value in step2.items():
                correlation_distributions[construct].append(value)
            row["step2_status"] = "usable"
        except MicomOracleError as error:
            row["step2_status"] = "failed"
            row["step2_failure_code"] = error.code.value

        try:
            step3 = _step3_statistics(pooled_scores, permutation_a_mask)
            for construct, values in step3.items():
                mean_distributions[construct].append(values["mean_difference"])
                variance_distributions[construct].append(values["log_variance_ratio"])
            row["step3_status"] = "usable"
        except MicomOracleError as error:
            row["step3_status"] = "failed"
            row["step3_failure_code"] = error.code.value
        ledger.append(row)

    step2_usable = sum(row["step2_status"] == "usable" for row in ledger)
    step3_usable = sum(row["step3_status"] == "usable" for row in ledger)
    if min(step2_usable, step3_usable) < config.minimum_usable_permutations:
        raise MicomOracleError(
            MicomFailureCode.INSUFFICIENT_USABLE_PERMUTATIONS,
            "too few requested permutations produced usable MICOM statistics",
            context={
                "requested_permutations": config.permutations,
                "step2_usable": step2_usable,
                "step3_usable": step3_usable,
                "minimum_usable_permutations": config.minimum_usable_permutations,
                "ledger": ledger,
            },
        )

    alpha = 1.0 - config.confidence_level
    constructs: dict[str, dict[str, Any]] = {}
    for construct in sorted(pooled_scores):
        correlations = correlation_distributions[construct]
        means = mean_distributions[construct]
        variances = variance_distributions[construct]
        correlation_lower = _type7_quantile(correlations, alpha)
        mean_lower = _type7_quantile(means, alpha / 2.0)
        mean_upper = _type7_quantile(means, 1.0 - alpha / 2.0)
        variance_lower = _type7_quantile(variances, alpha / 2.0)
        variance_upper = _type7_quantile(variances, 1.0 - alpha / 2.0)
        observed_correlation = observed_step2[construct]
        observed_mean = observed_step3[construct]["mean_difference"]
        observed_variance = observed_step3[construct]["log_variance_ratio"]
        compositional = observed_correlation + NUMERICAL_EPSILON >= correlation_lower
        equal_means = mean_lower <= observed_mean <= mean_upper
        equal_variances = variance_lower <= observed_variance <= variance_upper
        constructs[construct] = {
            "step1": {
                "status": "confirmed_by_researcher_review",
                "computed": False,
            },
            "step2": {
                "compositional_correlation": observed_correlation,
                "permutation_lower_quantile": correlation_lower,
                "lower_tail_p_value": _lower_tail_p(correlations, observed_correlation),
                "orientation_signs": observed_signs[construct],
                "compositional_invariance": compositional,
                "usable_permutations": len(correlations),
            },
            "step3": {
                **observed_step3[construct],
                "mean_difference_interval": [mean_lower, mean_upper],
                "mean_two_tailed_p_value": _two_tail_p(means, observed_mean),
                "equal_means": equal_means,
                "log_variance_ratio_interval": [variance_lower, variance_upper],
                "variance_two_tailed_p_value": _two_tail_p(
                    variances, observed_variance
                ),
                "equal_variances": equal_variances,
                "usable_permutations": len(means),
                "official_interval_decision": (
                    "the obtained difference is compared with the permutation interval"
                ),
            },
            "partial_measurement_invariance": compositional,
            "full_measurement_invariance": (
                compositional and equal_means and equal_variances
            ),
            "step3_interpretable": compositional,
        }

    return {
        "schema_version": 1,
        "oracle_version": ORACLE_VERSION,
        "method_version": METHOD_VERSION,
        "scope": {
            "method": "micom",
            "structural_path_permutation_mga": False,
            "consistent_permutation_plsc": False,
        },
        "group_direction": {
            "group_a": _stable_token(group_a),
            "group_b": _stable_token(group_b),
            "group_a_rows": count_a,
            "group_b_rows": count_b,
        },
        "config": asdict(config),
        "step1": {
            "status": "confirmed_by_researcher_review",
            "computed": False,
            "review": asdict(configural_review),
        },
        "constructs": constructs,
        "accounting": {
            "requested_permutations": config.permutations,
            "attempted_permutations": len(ledger),
            "retry_policy": "none",
            "step2_usable_permutations": step2_usable,
            "step2_failed_permutations": config.permutations - step2_usable,
            "step3_usable_permutations": step3_usable,
            "step3_failed_permutations": config.permutations - step3_usable,
        },
        "ledger": ledger,
    }


def load_fixed_fixture() -> tuple[np.ndarray, np.ndarray, tuple[str, ...]]:
    with FIXTURE_PATH.open(newline="", encoding="utf-8-sig") as handle:
        rows = list(csv.DictReader(handle))
    raw = np.asarray(
        [
            [float(row[indicator]) for indicator in independent_pls.INDICATORS]
            for row in rows
        ],
        dtype=float,
    )
    labels = np.asarray([row["group"] for row in rows], dtype=object)
    case_ids = tuple(f"micom-case-{index:04d}" for index in range(len(rows)))
    return raw, labels, case_ids


def _analytical_projection(result: Mapping[str, Any]) -> dict[str, Any]:
    return {
        "config": result["config"],
        "step1": result["step1"],
        "constructs": result["constructs"],
        "accounting": result["accounting"],
        "ledger": result["ledger"],
    }


def _maximum_delta(values: Sequence[float]) -> float:
    return max((abs(float(value)) for value in values), default=0.0)


def _capture_failure(call: Any) -> str:
    try:
        call()
    except MicomOracleError as error:
        return error.code.value
    raise AssertionError("boundary call unexpectedly succeeded")


def build_work_report(permutations: int = 39) -> dict[str, Any]:
    """Build source-only work evidence; this never runs or qualifies QuickPLS."""

    raw, labels, case_ids = load_fixed_fixture()
    review = ConfiguralReview(True, True, True, True, True, "validation fixture review")
    config = MicomConfig(permutations=permutations, minimum_usable_permutations=19)
    estimator = IndependentPlsEstimator()
    forward = run_micom(
        raw,
        labels,
        case_ids,
        group_a="A",
        group_b="B",
        configural_review=review,
        estimator=estimator,
        config=config,
    )
    reverse = run_micom(
        raw,
        labels,
        case_ids,
        group_a="B",
        group_b="A",
        configural_review=review,
        estimator=estimator,
        config=config,
    )
    order = np.arange(len(raw))[::-1]
    reordered = run_micom(
        raw[order],
        labels[order],
        tuple(case_ids[index] for index in order),
        group_a="A",
        group_b="B",
        configural_review=review,
        estimator=estimator,
        config=config,
    )
    repeated = run_micom(
        raw,
        labels,
        case_ids,
        group_a="A",
        group_b="B",
        configural_review=review,
        estimator=estimator,
        config=config,
    )
    declaration_reordered = run_micom(
        raw,
        labels,
        case_ids,
        group_a="A",
        group_b="B",
        configural_review=review,
        estimator=IndependentPlsEstimator(
            tuple(reversed(tuple(estimator.construct_ids)))
        ),
        config=config,
    )
    alternate_seed = run_micom(
        raw,
        labels,
        case_ids,
        group_a="A",
        group_b="B",
        configural_review=review,
        estimator=estimator,
        config=MicomConfig(
            permutations=permutations,
            seed=config.seed + 1,
            minimum_usable_permutations=19,
        ),
    )

    signed = []
    invariant = []
    decisions = []
    for construct in sorted(forward["constructs"]):
        left = forward["constructs"][construct]
        right = reverse["constructs"][construct]
        signed.extend(
            [
                left["step3"]["mean_difference"]
                + right["step3"]["mean_difference"],
                left["step3"]["log_variance_ratio"]
                + right["step3"]["log_variance_ratio"],
            ]
        )
        invariant.extend(
            [
                left["step2"]["compositional_correlation"]
                - right["step2"]["compositional_correlation"],
                left["step2"]["lower_tail_p_value"]
                - right["step2"]["lower_tail_p_value"],
                left["step3"]["mean_two_tailed_p_value"]
                - right["step3"]["mean_two_tailed_p_value"],
                left["step3"]["variance_two_tailed_p_value"]
                - right["step3"]["variance_two_tailed_p_value"],
            ]
        )
        decisions.extend(
            [
                left["partial_measurement_invariance"]
                == right["partial_measurement_invariance"],
                left["full_measurement_invariance"]
                == right["full_measurement_invariance"],
            ]
        )

    common = {
        "group_a": "A",
        "group_b": "B",
        "configural_review": review,
        "estimator": estimator,
    }
    small_labels = np.asarray(["A"] * 9 + ["B"] * (len(raw) - 9), dtype=object)
    imbalanced_raw = raw[:111]
    imbalanced_labels = np.asarray(["A"] * 10 + ["B"] * 101, dtype=object)
    degenerate = raw.copy()
    degenerate[:, 0] = 1.0
    boundary_codes = {
        "missing_configural_review": _capture_failure(
            lambda: run_micom(
                raw,
                labels,
                case_ids,
                group_a="A",
                group_b="B",
                configural_review=None,
                estimator=estimator,
                config=config,
            )
        ),
        "empty_group": _capture_failure(
            lambda: run_micom(
                raw,
                np.asarray(["A"] * len(raw), dtype=object),
                case_ids,
                config=config,
                **common,
            )
        ),
        "small_group": _capture_failure(
            lambda: run_micom(
                raw,
                small_labels,
                case_ids,
                config=config,
                **common,
            )
        ),
        "extreme_imbalance": _capture_failure(
            lambda: run_micom(
                imbalanced_raw,
                imbalanced_labels,
                case_ids[:111],
                config=config,
                **common,
            )
        ),
        "degenerate_indicator": _capture_failure(
            lambda: run_micom(
                degenerate,
                labels,
                case_ids,
                config=config,
                **common,
            )
        ),
    }
    expected_boundary_codes = {
        "missing_configural_review": MicomFailureCode.CONFIGURAL_REVIEW_REQUIRED.value,
        "empty_group": MicomFailureCode.EMPTY_GROUP.value,
        "small_group": MicomFailureCode.GROUP_TOO_SMALL.value,
        "extreme_imbalance": MicomFailureCode.EXTREME_GROUP_IMBALANCE.value,
        "degenerate_indicator": MicomFailureCode.DEGENERATE_INDICATOR.value,
    }
    forward_projection = _analytical_projection(forward)
    checks = {
        "step1_is_review_not_computation": (
            forward["step1"]["computed"] is False
            and forward["step1"]["status"] == "confirmed_by_researcher_review"
        ),
        "micom_only_scope": (
            forward["scope"]["method"] == "micom"
            and forward["scope"]["structural_path_permutation_mga"] is False
            and forward["scope"]["consistent_permutation_plsc"] is False
        ),
        "exact_no_retry_accounting": (
            forward["accounting"]["requested_permutations"] == permutations
            and forward["accounting"]["attempted_permutations"] == permutations
            and forward["accounting"]["retry_policy"] == "none"
            and len(forward["ledger"]) == permutations
            and [row["replicate"] for row in forward["ledger"]]
            == list(range(permutations))
        ),
        "group_swap_signed_reversal": _maximum_delta(signed) <= 2e-12,
        "group_swap_probabilities_equal": _maximum_delta(invariant) <= 2e-12,
        "group_swap_decisions_equal": all(decisions),
        "group_swap_partition_and_accounting_equal": (
            [row["partition_sha256"] for row in forward["ledger"]]
            == [row["partition_sha256"] for row in reverse["ledger"]]
            and forward["accounting"] == reverse["accounting"]
        ),
        "row_reorder_invariant": (
            forward_projection == _analytical_projection(reordered)
        ),
        "construct_declaration_reorder_invariant": (
            forward_projection == _analytical_projection(declaration_reordered)
        ),
        "same_seed_repeat_exact": (
            forward_projection == _analytical_projection(repeated)
        ),
        "different_seed_changes_plan": (
            [row["partition_sha256"] for row in forward["ledger"]]
            != [row["partition_sha256"] for row in alternate_seed["ledger"]]
        ),
        "typed_boundaries_exact": boundary_codes == expected_boundary_codes,
    }
    return {
        "schema_version": 1,
        "report_kind": "micom_v3_independent_oracle_work_report",
        "oracle_version": ORACLE_VERSION,
        "method_version": METHOD_VERSION,
        "fixture_path": str(FIXTURE_PATH.relative_to(ROOT)).replace("\\", "/"),
        "permutations": permutations,
        "work_evidence_only": True,
        "qualification_ready": False,
        "promotion_requested": False,
        "checks": checks,
        "passed": all(checks.values()),
        "maximum_group_swap_signed_residual": _maximum_delta(signed),
        "maximum_group_swap_probability_delta": _maximum_delta(invariant),
        "boundary_codes": boundary_codes,
        "accounting": forward["accounting"],
        "remaining_blockers": [
            "No current QuickPLS product result has been compared with this v3.1 no-retry MICOM oracle.",
            "No frozen exact-product fixture yet binds micom_v3_1 output to this oracle under the same canonical partition plan.",
            "The source fixture covers one bounded reflective path-weighting model; Mode B/mixed composites, controls, interactions, higher-order constructs, missing-data policies, and broader eligible PLS shapes remain unqualified.",
            "Arbitrary declared-group workflows and deterministic pairwise orchestration beyond one selected A/B comparison remain unqualified.",
            "Qualification-sized 5,000/10,000-run simulations, calibration, power, and failure-rate evidence are absent.",
            "A second maintained computational MICOM implementation has not been attached as independent evidence.",
            "Archive, cross-format export, frontend, packaged Windows, accessibility, performance, soak, and independent scientific-review receipts are absent.",
        ],
        "references": [
            {
                "kind": "primary",
                "citation": "Henseler, Ringle, and Sarstedt (2016), DOI 10.1108/IMR-09-2014-0304",
                "url": "https://doi.org/10.1108/IMR-09-2014-0304",
            },
            {
                "kind": "official_current_behavior",
                "citation": "SmartPLS Measurement Invariance Assessment (MICOM)",
                "url": "https://smartpls.com/documentation/algorithms-and-techniques/heterogeneity-and-multigroup/micom/",
            },
            {
                "kind": "official_current_resampling",
                "citation": "SmartPLS Permutation",
                "url": "https://smartpls.com/documentation/algorithms-and-techniques/resampling-and-inference/permutation/",
            },
        ],
    }


def write_work_report(path: Path = WORK_REPORT_PATH, permutations: int = 39) -> dict[str, Any]:
    report = build_work_report(permutations)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--permutations", type=int, default=39)
    parser.add_argument("--write", action="store_true")
    args = parser.parse_args()
    report = (
        write_work_report(permutations=args.permutations)
        if args.write
        else build_work_report(args.permutations)
    )
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
