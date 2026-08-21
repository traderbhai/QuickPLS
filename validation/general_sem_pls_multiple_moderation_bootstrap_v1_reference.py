#!/usr/bin/env python3
"""Independent observed-score reference for moderation-bootstrap gamma inference.

This standard-library-only program deliberately does not import or invoke any
QuickPLS production module.  It exercises the downstream mathematics of the
General SEM simultaneous two-way moderation bootstrap with deterministic case
resampling.  The score columns are observed proxies: this is not an
indicator-level PLS score oracle and is not release-qualification evidence.
"""

from __future__ import annotations

import hashlib
import json
import math
from dataclasses import dataclass
from typing import Iterable, Mapping, Sequence


FEATURE_ID = "qpls3.pls.general_sem_multiple_two_way_moderation_bootstrap"
METHOD_VERSION = (
    "general_sem_pls_multiple_two_way_moderation_full_model_case_bootstrap_v1"
)
REFERENCE_STREAM_VERSION = "independent_sha256_indexed_case_stream_v1"
REFERENCE_SCOPE = "independent_observed_score_gamma_only_smoke_v1"
CONFIDENCE_LEVEL = 0.95
RESAMPLES = 199
TOLERANCE = 1e-10
PROBES = (-1.0, 0.0, 1.0)


@dataclass(frozen=True)
class Target:
    target_id: str
    interaction_id: str
    focal_relation_id: str
    focal_predictor_id: str
    moderator_id: str
    outcome_id: str

    def as_dict(self) -> dict[str, str]:
        return {
            "kind": "interaction_scientific_rescaled_gamma",
            "target_id": self.target_id,
            "interaction_id": self.interaction_id,
            "focal_relation_id": self.focal_relation_id,
            "focal_predictor_id": self.focal_predictor_id,
            "moderator_id": self.moderator_id,
            "outcome_id": self.outcome_id,
        }


@dataclass(frozen=True)
class Scenario:
    scenario_id: str
    outcome_id: str
    ordinary_ids: tuple[str, ...]
    interactions: tuple[tuple[Target, str, str], ...]


@dataclass(frozen=True)
class PointFit:
    ordinary_coefficients: Mapping[str, float]
    standardized_product_coefficients: Mapping[str, float]
    scientific_gammas: Mapping[str, float]
    product_scales: Mapping[str, float]
    fixed_probe_slopes: Mapping[str, tuple[float, float, float]]
    maximum_normal_equation_error: float
    corrected_sign_count: int


def _mean(values: Sequence[float]) -> float:
    if not values:
        raise ValueError("mean requires at least one value")
    return math.fsum(values) / len(values)


def _sample_sd(values: Sequence[float]) -> float:
    if len(values) < 2:
        raise ValueError("sample standard deviation requires two values")
    center = _mean(values)
    return math.sqrt(
        math.fsum((value - center) ** 2 for value in values) / (len(values) - 1)
    )


def _standardize(values: Sequence[float]) -> list[float]:
    if any(not math.isfinite(value) for value in values):
        raise ValueError("cannot standardize a nonfinite column")
    center = _mean(values)
    scale = _sample_sd(values)
    if not math.isfinite(scale) or scale <= 1e-14:
        raise ValueError("cannot standardize a constant column")
    return [(value - center) / scale for value in values]


def _solve(matrix: Sequence[Sequence[float]], vector: Sequence[float]) -> list[float]:
    size = len(vector)
    if size == 0 or len(matrix) != size or any(len(row) != size for row in matrix):
        raise ValueError("invalid linear-system dimensions")
    augmented = [list(row) + [value] for row, value in zip(matrix, vector, strict=True)]
    for column in range(size):
        pivot = max(range(column, size), key=lambda row: abs(augmented[row][column]))
        if abs(augmented[pivot][column]) <= 1e-12:
            raise ValueError("singular joint stage-two equation")
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        divisor = augmented[column][column]
        augmented[column] = [value / divisor for value in augmented[column]]
        for row in range(size):
            if row == column:
                continue
            factor = augmented[row][column]
            augmented[row] = [
                value - factor * basis
                for value, basis in zip(
                    augmented[row], augmented[column], strict=True
                )
            ]
    solution = [augmented[row][-1] for row in range(size)]
    if any(not math.isfinite(value) for value in solution):
        raise ValueError("joint stage-two equation produced a nonfinite coefficient")
    return solution


def _fit_joint(
    columns: Sequence[Sequence[float]], outcome: Sequence[float]
) -> tuple[list[float], float]:
    width = len(columns)
    if width == 0 or any(len(column) != len(outcome) for column in columns):
        raise ValueError("invalid joint stage-two column domain")
    gram = [
        [
            math.fsum(
                left * right
                for left, right in zip(columns[row], columns[column], strict=True)
            )
            for column in range(width)
        ]
        for row in range(width)
    ]
    rhs = [
        math.fsum(value * target for value, target in zip(column, outcome, strict=True))
        for column in columns
    ]
    coefficients = _solve(gram, rhs)
    normal_error = max(
        abs(
            math.fsum(gram[row][column] * coefficients[column] for column in range(width))
            - rhs[row]
        )
        for row in range(width)
    )
    return coefficients, normal_error


def _covariance_sum(left: Sequence[float], right: Sequence[float]) -> float:
    left_mean = _mean(left)
    right_mean = _mean(right)
    return math.fsum(
        (a - left_mean) * (b - right_mean)
        for a, b in zip(left, right, strict=True)
    )


def _align_scores(
    candidate: Sequence[float], sampled_original: Sequence[float]
) -> tuple[list[float], bool]:
    covariance = _covariance_sum(candidate, sampled_original)
    absolute_sum = math.fsum(
        abs((a - _mean(candidate)) * (b - _mean(sampled_original)))
        for a, b in zip(candidate, sampled_original, strict=True)
    )
    tolerance = 64.0 * math.ulp(1.0) * max(absolute_sum, abs(covariance), 1.0)
    if covariance > tolerance:
        return list(candidate), False
    if covariance < -tolerance:
        return [-value for value in candidate], True
    raise ValueError("indeterminate score-vector sign")


def _product_column(
    left: Sequence[float], right: Sequence[float]
) -> tuple[list[float], float]:
    raw_product = [a * b for a, b in zip(left, right, strict=True)]
    scale = _sample_sd(raw_product)
    if not math.isfinite(scale) or scale <= 1e-14:
        raise ValueError("constant or nonfinite interaction product")
    return _standardize(raw_product), scale


def _case_positions(row_count: int, seed: int, replicate: int) -> list[int]:
    positions: list[int] = []
    for draw in range(row_count):
        message = (
            f"{REFERENCE_STREAM_VERSION}|{seed}|{replicate}|{draw}|{row_count}"
        ).encode("ascii")
        positions.append(int.from_bytes(hashlib.sha256(message).digest()[:8], "big") % row_count)
    return positions


def _base_columns() -> dict[str, list[float]]:
    row_values = [index - 39.5 for index in range(80)]
    raw_x = [math.sin(0.17 * row) + 0.013 * row + 0.0007 * row * row for row in row_values]
    raw_w = [math.cos(0.23 * row) + 0.23 * math.sin(0.071 * row) - 0.004 * row for row in row_values]
    raw_z = [0.58 * raw_w[index] + math.sin(0.31 * row) + 0.006 * row for index, row in enumerate(row_values)]
    raw_a = [math.cos(0.137 * row) - 0.32 * math.sin(0.19 * row) + 0.005 * row for row in row_values]
    x = _standardize(raw_x)
    w = _standardize(raw_w)
    z = _standardize(raw_z)
    a = _standardize(raw_a)
    noise = _standardize(
        [math.sin(0.61 * row) + 0.3 * math.cos(0.43 * row) for row in row_values]
    )
    y_same = [
        0.31 * x_i
        + 0.17 * w_i
        - 0.12 * z_i
        + 0.24 * x_i * w_i
        - 0.18 * x_i * z_i
        + 0.08 * error
        for x_i, w_i, z_i, error in zip(x, w, z, noise, strict=True)
    ]
    y_different = [
        0.27 * x_i
        + 0.13 * w_i
        - 0.21 * a_i
        + 0.15 * z_i
        + 0.20 * x_i * w_i
        - 0.16 * a_i * z_i
        + 0.08 * error
        for x_i, w_i, a_i, z_i, error in zip(x, w, a, z, noise, strict=True)
    ]
    return {
        "X": raw_x,
        "W": raw_w,
        "Z": raw_z,
        "A": raw_a,
        "Y_SAME": y_same,
        "Y_DIFFERENT": y_different,
    }


def _scenarios() -> tuple[Scenario, Scenario]:
    same = Scenario(
        scenario_id="same_focal_joint_equation",
        outcome_id="Y_SAME",
        ordinary_ids=("X", "W", "Z"),
        interactions=(
            (
                Target("rel_XW_Y", "int_XW", "rel_X_Y", "X", "W", "Y_SAME"),
                "X",
                "W",
            ),
            (
                Target("rel_XZ_Y", "int_XZ", "rel_X_Y", "X", "Z", "Y_SAME"),
                "X",
                "Z",
            ),
        ),
    )
    different = Scenario(
        scenario_id="different_focal_joint_equation",
        outcome_id="Y_DIFFERENT",
        ordinary_ids=("X", "W", "A", "Z"),
        interactions=(
            (
                Target("rel_XW_Y2", "int_XW_2", "rel_X_Y2", "X", "W", "Y_DIFFERENT"),
                "X",
                "W",
            ),
            (
                Target("rel_AZ_Y2", "int_AZ", "rel_A_Y2", "A", "Z", "Y_DIFFERENT"),
                "A",
                "Z",
            ),
        ),
    )
    return same, different


def _point_fit(
    scenario: Scenario,
    raw_columns: Mapping[str, Sequence[float]],
    original_scores: Mapping[str, Sequence[float]],
    positions: Sequence[int],
    replicate: int,
) -> PointFit:
    required = set(scenario.ordinary_ids) | {scenario.outcome_id}
    scores: dict[str, list[float]] = {}
    corrected_sign_count = 0
    for ordinal, column_id in enumerate(sorted(required)):
        sampled = [raw_columns[column_id][position] for position in positions]
        candidate = _standardize(sampled)
        # Deliberately introduce a deterministic latent-score sign ambiguity,
        # then resolve it against the sampled original orientation before any
        # interaction product is constructed.
        if (replicate + ordinal) % 3 == 1:
            candidate = [-value for value in candidate]
        sampled_original = [original_scores[column_id][position] for position in positions]
        aligned, corrected = _align_scores(candidate, sampled_original)
        scores[column_id] = aligned
        corrected_sign_count += int(corrected)

    joint_columns = [scores[column_id] for column_id in scenario.ordinary_ids]
    product_scales: dict[str, float] = {}
    for target, left_id, right_id in scenario.interactions:
        product, scale = _product_column(scores[left_id], scores[right_id])
        joint_columns.append(product)
        product_scales[target.target_id] = scale

    coefficients, normal_error = _fit_joint(joint_columns, scores[scenario.outcome_id])
    ordinary_count = len(scenario.ordinary_ids)
    ordinary = dict(zip(scenario.ordinary_ids, coefficients[:ordinary_count], strict=True))
    product_betas: dict[str, float] = {}
    gammas: dict[str, float] = {}
    slopes: dict[str, tuple[float, float, float]] = {}
    for offset, (target, _left_id, _right_id) in enumerate(scenario.interactions):
        beta = coefficients[ordinary_count + offset]
        gamma = beta / product_scales[target.target_id]
        focal_beta = ordinary[target.focal_predictor_id]
        product_betas[target.target_id] = beta
        gammas[target.target_id] = gamma
        slopes[target.target_id] = tuple(focal_beta + gamma * probe for probe in PROBES)

    all_values: Iterable[float] = (
        list(ordinary.values())
        + list(product_betas.values())
        + list(gammas.values())
        + list(product_scales.values())
        + [value for target_slopes in slopes.values() for value in target_slopes]
        + [normal_error]
    )
    if any(not math.isfinite(value) for value in all_values):
        raise ValueError("complete joint point contract contains a nonfinite value")
    if normal_error > TOLERANCE:
        raise ValueError("joint stage-two normal equations did not reconcile")
    return PointFit(
        ordinary_coefficients=ordinary,
        standardized_product_coefficients=product_betas,
        scientific_gammas=gammas,
        product_scales=product_scales,
        fixed_probe_slopes=slopes,
        maximum_normal_equation_error=normal_error,
        corrected_sign_count=corrected_sign_count,
    )


def type7_quantile(sorted_values: Sequence[float], probability: float) -> float:
    if not sorted_values or not 0.0 <= probability <= 1.0:
        raise ValueError("invalid Type-7 quantile input")
    position = (len(sorted_values) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return sorted_values[lower]
    return sorted_values[lower] + (position - lower) * (
        sorted_values[upper] - sorted_values[lower]
    )


def minimum_usable_replicates(requested: int) -> int:
    if requested < 2:
        raise ValueError("at least two resamples are required")
    return max(2, math.ceil(0.9 * requested))


def summarize_gamma(
    target: Target,
    original: float,
    values: Sequence[float],
    confidence_level: float = CONFIDENCE_LEVEL,
) -> dict[str, object]:
    if len(values) < 2 or any(not math.isfinite(value) for value in values):
        raise ValueError("gamma summary requires at least two finite replicates")
    ordered = sorted(values)
    mean = _mean(ordered)
    standard_error = math.sqrt(
        math.fsum((value - mean) ** 2 for value in ordered) / (len(ordered) - 1)
    )
    alpha = 1.0 - confidence_level
    exceedances = sum(abs(value - original) >= abs(original) for value in ordered)
    return {
        "target": target.as_dict(),
        "original": original,
        "bootstrap_mean": mean,
        "bootstrap_bias": mean - original,
        "standard_error": standard_error,
        "lower": type7_quantile(ordered, alpha / 2.0),
        "upper": type7_quantile(ordered, 1.0 - alpha / 2.0),
        "p_value_two_sided": (exceedances + 1) / (len(ordered) + 1),
        "usable_replicates": len(ordered),
        "two_sided_exceedances": exceedances,
    }


def _run_scenario(
    scenario: Scenario,
    raw_columns: Mapping[str, Sequence[float]],
    original_scores: Mapping[str, Sequence[float]],
    resamples: int,
    seed: int,
    evaluation_order: Sequence[int],
) -> tuple[PointFit, dict[int, PointFit]]:
    row_count = len(next(iter(raw_columns.values())))
    original = _point_fit(
        scenario,
        raw_columns,
        original_scores,
        list(range(row_count)),
        replicate=0,
    )
    replicates: dict[int, PointFit] = {}
    for replicate in evaluation_order:
        positions = _case_positions(row_count, seed, replicate)
        replicates[replicate] = _point_fit(
            scenario,
            raw_columns,
            original_scores,
            positions,
            replicate=replicate + 1,
        )
    if set(replicates) != set(range(resamples)):
        raise ValueError("evaluation order did not cover the indexed resampling plan")
    return original, replicates


def _max_difference(left: Sequence[float], right: Sequence[float]) -> float:
    return max(abs(a - b) for a, b in zip(left, right, strict=True))


def run_reference(resamples: int = RESAMPLES) -> dict[str, object]:
    raw_columns = _base_columns()
    original_scores = {
        column_id: _standardize(values) for column_id, values in raw_columns.items()
    }
    scenario_reports: list[dict[str, object]] = []
    all_inference: list[dict[str, object]] = []
    replay_differences: list[float] = []
    product_scale_ranges: list[float] = []
    corrected_signs = 0
    maximum_normal_error = 0.0

    for scenario_index, scenario in enumerate(_scenarios()):
        seed = 2026081901 + scenario_index
        forward = list(range(resamples))
        reverse = list(reversed(forward))
        original, replicates = _run_scenario(
            scenario, raw_columns, original_scores, resamples, seed, forward
        )
        replay_original, replay = _run_scenario(
            scenario, raw_columns, original_scores, resamples, seed, reverse
        )
        target_reports: list[dict[str, object]] = []
        for target, _left, _right in scenario.interactions:
            target_id = target.target_id
            values = [replicates[index].scientific_gammas[target_id] for index in forward]
            replay_values = [replay[index].scientific_gammas[target_id] for index in forward]
            replay_differences.append(_max_difference(values, replay_values))
            inference = summarize_gamma(
                target,
                original.scientific_gammas[target_id],
                values,
            )
            all_inference.append(inference)
            scales = [replicates[index].product_scales[target_id] for index in forward]
            scale_range = max(scales) - min(scales)
            product_scale_ranges.append(scale_range)
            target_reports.append(
                {
                    "target": target.as_dict(),
                    "gamma_inference": inference,
                    "product_scale_minimum": min(scales),
                    "product_scale_maximum": max(scales),
                    "product_scale_range": scale_range,
                }
            )
        corrected_signs += sum(point.corrected_sign_count for point in replicates.values())
        maximum_normal_error = max(
            maximum_normal_error,
            original.maximum_normal_equation_error,
            replay_original.maximum_normal_equation_error,
            *(point.maximum_normal_equation_error for point in replicates.values()),
            *(point.maximum_normal_equation_error for point in replay.values()),
        )
        scenario_reports.append(
            {
                "scenario_id": scenario.scenario_id,
                "layout": "same_focal" if scenario_index == 0 else "different_focal",
                "joint_interaction_count": len(scenario.interactions),
                "resamples_requested": resamples,
                "resamples_usable": resamples,
                "full_joint_point_contract_validated_per_replicate": True,
                "ordinary_coefficients_and_fixed_probe_slopes_point_only": True,
                "gamma_targets": target_reports,
            }
        )

    type7_check = abs(type7_quantile([1.0, 2.0, 4.0, 8.0], 0.25) - 1.75) <= TOLERANCE
    se_summary = summarize_gamma(
        Target("micro", "micro", "micro", "X", "W", "Y"),
        2.0,
        [1.0, 2.0, 4.0],
    )
    expected_se = math.sqrt(7.0 / 3.0)
    gate_accepts_exact_boundary = len([0.0] * 18) >= minimum_usable_replicates(20)
    gate_rejects_below_boundary = len([0.0] * 17) < minimum_usable_replicates(20)
    constant_rejected = False
    singular_rejected = False
    try:
        _product_column([1.0, 1.0, 1.0], [2.0, 2.0, 2.0])
    except ValueError:
        constant_rejected = True
    try:
        _fit_joint([[1.0, 2.0, 3.0], [1.0, 2.0, 3.0]], [1.0, 2.0, 3.0])
    except ValueError:
        singular_rejected = True

    checks = {
        "same_focal_complete_joint_case_bootstrap": scenario_reports[0]["resamples_usable"] == resamples,
        "different_focal_complete_joint_case_bootstrap": scenario_reports[1]["resamples_usable"] == resamples,
        "gamma_only_inference_inventory": len(all_inference) == 4
        and all(row["target"]["kind"] == "interaction_scientific_rescaled_gamma" for row in all_inference),
        "type7_percentile_microcase": type7_check,
        "sample_standard_error_b_minus_one_microcase": abs(se_summary["standard_error"] - expected_se) <= TOLERANCE,
        "null_centered_plus_one_p_microcase": se_summary["two_sided_exceedances"] == 1
        and abs(se_summary["p_value_two_sided"] - 0.5) <= TOLERANCE,
        "exact_ninety_percent_gate_accepts_18_of_20": gate_accepts_exact_boundary,
        "exact_ninety_percent_gate_rejects_17_of_20": gate_rejects_below_boundary,
        "indexed_replay_and_evaluation_order_invariant": max(replay_differences) <= TOLERANCE,
        "score_vector_sign_alignment_precedes_products": corrected_signs > 0,
        "product_scale_recomputed_per_replicate": min(product_scale_ranges) > 1e-4,
        "complete_joint_point_contract_reconciled_per_replicate": maximum_normal_error <= TOLERANCE,
        "constant_product_rejected": constant_rejected,
        "singular_joint_equation_rejected": singular_rejected,
    }
    return {
        "schema_version": 1,
        "reference_kind": "independent_observed_score_moderation_bootstrap_reference",
        "reference_scope": REFERENCE_SCOPE,
        "feature_id": FEATURE_ID,
        "method_version": METHOD_VERSION,
        "catalogue_snapshot_date": "2026-08-19",
        "passed": all(checks.values()),
        "qualification_ready": False,
        "promotion_allowed": False,
        "inferential_target_policy": "scientific_rescaled_gamma_only",
        "resampling": {
            "stream_version": REFERENCE_STREAM_VERSION,
            "unit": "case",
            "replacement": "with_replacement",
            "resamples": resamples,
            "confidence_level": CONFIDENCE_LEVEL,
            "interval": "percentile_type7",
            "standard_error": "sample_b_minus_one",
            "p_value": "two_sided_null_centered_plus_one",
            "minimum_usable": minimum_usable_replicates(resamples),
        },
        "checks": checks,
        "metrics": {
            "maximum_replay_difference": max(replay_differences),
            "minimum_product_scale_range": min(product_scale_ranges),
            "corrected_score_vector_sign_count": corrected_signs,
            "maximum_normal_equation_error": maximum_normal_error,
        },
        "scenarios": scenario_reports,
        "gamma_inference": all_inference,
        "limitations": [
            "The reference never imports or invokes QuickPLS production Rust, TypeScript, native, resampling, or estimation code.",
            "Observed construct-score proxies are resampled and restandardized; indicators, PLS weights, loadings, iterations, and full stage-one PLS score recovery are outside this reference.",
            "Artificial sign flips verify independent score-vector orientation arithmetic but do not qualify the production PLS sign oracle.",
            "The SHA-256 indexed stream is independently deterministic and makes no claim of random-stream identity with QuickPLS.",
            "Only scientific rescaled gamma receives bootstrap inference. Standardized-product beta, ordinary coefficients, fixed probes, and plot points are recomputed for fit validation but remain point-only.",
            "This bounded smoke reference does not establish nominal coverage, power, null calibration, SmartPLS numerical parity, archive/native/export/package behavior, or release qualification.",
        ],
    }


def main() -> int:
    report = run_reference()
    print(json.dumps(report, indent=2, sort_keys=True, allow_nan=False))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
