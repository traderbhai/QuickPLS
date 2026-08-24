#!/usr/bin/env python3
"""Independent semantic oracle for the production raw-data MGA qualification receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import struct
import sys
from collections import Counter, defaultdict
from pathlib import Path
from statistics import NormalDist
from typing import Any, Iterable


SUITE_ID = "qpls.multimod.mga.production-qualification.v1"
EXPECTED_GROUPS = (2, 3, 5, 20)
EXPECTED_PROFILE_FIXTURES = {
    "multiple_two_way",
    "bounded_three_way",
    "bounded_two_way_moderated_mediation",
    "multiple_nonnested_hoc",
    "case_weighted_pls",
    "reflective_plsc",
}
PAIRWISE_PERMUTATION = "pairwise_permutation_two_tailed_or_predeclared_direction_v1"
HENSELER = "henseler_pls_mga_directional_probability_v1"
BC = "bootstrap_difference_bc_zero_acceleration_v1"
POOLED = "parametric_pooled_variance_sensitivity_v1"
WELCH = "parametric_welch_satterthwaite_sensitivity_v1"
OMNIBUS = "max_spread_omnibus_permutation_v1"
WALD = "inverse_variance_wald_sensitivity_v1"
TOL = 2.5e-10
FREQUENCY_PROCEDURE_ALIASES = {
    "frequency_count_space_pairwise_permutation_v1": PAIRWISE_PERMUTATION,
    "frequency_count_space_henseler_pls_mga_directional_probability_v1": HENSELER,
    "frequency_count_space_bootstrap_difference_bc_zero_acceleration_v1": BC,
}
KS_ALPHA_001_CRITICAL = 1.95
QUALIFICATION_RESAMPLES = 5_000
QUALIFICATION_MINIMUM_USABLE = 4_500
FREQUENCY_PAIRWISE_METHOD = "qpls.mga.frequency-count-space.pairwise-permutation.v1"
EXPANDED_PAIRWISE_METHOD = "mga_multigroup_pairwise_permutation_v1"
FREQUENCY_BOOTSTRAP_METHOD = "qpls.mga.frequency-count-space.bootstrap-bank.v1"
EXPANDED_BOOTSTRAP_METHOD = "mga_multigroup_group_bootstrap_bank_v1"

# Exact stable identities consumed by downstream campaign gates. Keeping this
# list in the producer comparator makes the dependency binding source-auditable
# even when an individual check is emitted from a formatted profile/pair loop.
BOUND_GATE_CHECK_IDS = {
    "mga.group_matrix.g2.pairwise_probability_from_raw_null",
    "mga.group_matrix.g2.micom_step2_from_raw_null",
    "mga.group_matrix.g2.micom_step3_from_raw_null",
    "mga.group_matrix.g2.henseler_probability_from_raw_bootstrap",
    "mga.group_matrix.g3.omnibus_precedes_pairwise",
    "mga.group_matrix.g3.omnibus_probability_from_raw_null",
    "mga.group_matrix.g20.pair_inventory.pairwise_permutation_two_tailed_or_predeclared_direction_v1",
    "mga.group_matrix.g20.micom_permutation_shared_partition_authority",
    "mga.group_matrix.g3.multiplicity_formula.holm",
    "mga.group_matrix.g3.multiplicity_formula.bonferroni",
    "mga.group_matrix.g3.multiplicity_formula.sidak",
    "mga.group_matrix.g3.multiplicity_formula.benjamini_hochberg",
    "mga.group_matrix.g3.multiplicity_formula.none",
    "mga.group_matrix.g3.public_holm_adjusted_probabilities",
    "mga.inference.parametric_sensitivity.parametric_formula.pooled_equal_residual_variance.0_1",
    "mga.inference.parametric_sensitivity.parametric_formula.welch_satterthwaite.0_1",
    "mga.inference.parametric_sensitivity.wald_formula.0",
    "mga.label_reversal.partition_sign_tail_invariance",
    "mga.label_reversal.public_result_invariance",
}


def close(left: float, right: float, tolerance: float = TOL) -> bool:
    return math.isfinite(left) and math.isfinite(right) and abs(left - right) <= tolerance * max(
        1.0, abs(left), abs(right)
    )


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for block in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def sha256_f64_series(values: Iterable[float]) -> str:
    materialized = [float(value) for value in values]
    digest = hashlib.sha256()
    digest.update(struct.pack("<Q", len(materialized)))
    for value in materialized:
        digest.update(struct.pack("<d", value))
    return digest.hexdigest()


def is_lower_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in "0123456789abcdef" for character in value)
    )


def is_dataset_fingerprint(value: Any) -> bool:
    return is_lower_sha256(value) or (
        isinstance(value, str)
        and value.startswith("v2:")
        and is_lower_sha256(value.removeprefix("v2:"))
    )


def is_exact_weight_binding(value: Any, kind: str) -> bool:
    return value == {
        "kind": kind,
        "variable": "observed:multimod_weight",
    }


def two_sample_ks_distance(left: list[float], right: list[float]) -> float:
    """Return the two-sample Kolmogorov-Smirnov distance without SciPy."""
    if not left or not right:
        return math.inf
    left_sorted = sorted(float(value) for value in left)
    right_sorted = sorted(float(value) for value in right)
    if not all(math.isfinite(value) for value in left_sorted + right_sorted):
        return math.inf
    left_index = 0
    right_index = 0
    distance = 0.0
    while left_index < len(left_sorted) or right_index < len(right_sorted):
        if right_index == len(right_sorted) or (
            left_index < len(left_sorted) and left_sorted[left_index] <= right_sorted[right_index]
        ):
            boundary = left_sorted[left_index]
        else:
            boundary = right_sorted[right_index]
        while left_index < len(left_sorted) and left_sorted[left_index] <= boundary:
            left_index += 1
        while right_index < len(right_sorted) and right_sorted[right_index] <= boundary:
            right_index += 1
        distance = max(
            distance,
            abs(left_index / len(left_sorted) - right_index / len(right_sorted)),
        )
    return distance


def ks_alpha_001_limit(left_count: int, right_count: int) -> float:
    if left_count <= 0 or right_count <= 0:
        return 0.0
    return KS_ALPHA_001_CRITICAL * math.sqrt(
        (left_count + right_count) / (left_count * right_count)
    )


class Checks:
    def __init__(self) -> None:
        self.rows: list[dict[str, Any]] = []

    def add(
        self,
        check_id: str,
        passed: bool,
        *,
        observed: Any = None,
        expected: Any = None,
        detail: str = "",
    ) -> None:
        self.rows.append(
            {
                "check_id": check_id,
                "status": "passed" if passed else "failed",
                "observed": observed,
                "expected": expected,
                "detail": detail,
            }
        )

    def require(
        self,
        check_id: str,
        predicate: bool,
        *,
        observed: Any = None,
        expected: Any = None,
        detail: str = "",
    ) -> None:
        self.add(
            check_id,
            bool(predicate),
            observed=observed,
            expected=expected,
            detail=detail,
        )

    @property
    def passed(self) -> bool:
        return all(row["status"] == "passed" for row in self.rows)


def ordered_pairs(group_count: int) -> set[tuple[str, str]]:
    return {
        (f"group_{left + 1:02}", f"group_{right + 1:02}")
        for left in range(group_count)
        for right in range(left + 1, group_count)
    }


def pair_indices(value: dict[str, Any]) -> tuple[int, int]:
    pair = value["pair"]
    return int(pair["group_a"]), int(pair["group_b"])


def target_id(parameter: dict[str, Any]) -> str:
    return str(parameter["stable_id"])


def type7(values: Iterable[float], probability: float) -> float:
    ordered = sorted(float(value) for value in values)
    if len(ordered) == 1:
        return ordered[0]
    position = (len(ordered) - 1) * min(1.0, max(0.0, probability))
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def independent_bc(point: float, draws: list[float], confidence: float) -> tuple[float, float]:
    less = sum(value < point for value in draws)
    ties = sum(value == point for value in draws)
    corrected_rank = (less + 0.5 * ties + 0.5) / (len(draws) + 1.0)
    normal = NormalDist()
    epsilon = sys.float_info.epsilon
    z0 = normal.inv_cdf(min(1.0 - epsilon, max(epsilon, corrected_rank)))
    alpha = 1.0 - confidence
    lower_probability = normal.cdf(2.0 * z0 + normal.inv_cdf(alpha / 2.0))
    upper_probability = normal.cdf(2.0 * z0 + normal.inv_cdf(1.0 - alpha / 2.0))
    return type7(draws, lower_probability), type7(draws, upper_probability)


def beta_continued_fraction(a: float, b: float, x: float) -> float:
    maximum_iterations = 400
    epsilon = 3.0e-14
    floor = 1.0e-300
    qab = a + b
    qap = a + 1.0
    qam = a - 1.0
    c = 1.0
    d = 1.0 - qab * x / qap
    d = floor if abs(d) < floor else d
    d = 1.0 / d
    result = d
    for iteration in range(1, maximum_iterations + 1):
        twice = 2 * iteration
        numerator = iteration * (b - iteration) * x / ((qam + twice) * (a + twice))
        d = 1.0 + numerator * d
        d = floor if abs(d) < floor else d
        c = 1.0 + numerator / c
        c = floor if abs(c) < floor else c
        d = 1.0 / d
        result *= d * c
        numerator = -(a + iteration) * (qab + iteration) * x / (
            (a + twice) * (qap + twice)
        )
        d = 1.0 + numerator * d
        d = floor if abs(d) < floor else d
        c = 1.0 + numerator / c
        c = floor if abs(c) < floor else c
        d = 1.0 / d
        delta = d * c
        result *= delta
        if abs(delta - 1.0) <= epsilon:
            return result
    raise ArithmeticError("incomplete beta continued fraction did not converge")


def regularized_beta(x: float, a: float, b: float) -> float:
    if x <= 0.0:
        return 0.0
    if x >= 1.0:
        return 1.0
    log_front = (
        math.lgamma(a + b)
        - math.lgamma(a)
        - math.lgamma(b)
        + a * math.log(x)
        + b * math.log1p(-x)
    )
    front = math.exp(log_front)
    if x < (a + 1.0) / (a + b + 2.0):
        return front * beta_continued_fraction(a, b, x) / a
    return 1.0 - front * beta_continued_fraction(b, a, 1.0 - x) / b


def student_t_two_sided(statistic: float, degrees_of_freedom: float) -> float:
    x = degrees_of_freedom / (degrees_of_freedom + statistic * statistic)
    return regularized_beta(x, degrees_of_freedom / 2.0, 0.5)


def gamma_q(shape: float, x: float) -> float:
    if x < 0.0 or shape <= 0.0:
        raise ValueError("invalid incomplete-gamma arguments")
    if x == 0.0:
        return 1.0
    epsilon = 3.0e-14
    floor = 1.0e-300
    if x < shape + 1.0:
        term = 1.0 / shape
        total = term
        ap = shape
        for _ in range(1, 10000):
            ap += 1.0
            term *= x / ap
            total += term
            if abs(term) <= abs(total) * epsilon:
                p = total * math.exp(-x + shape * math.log(x) - math.lgamma(shape))
                return min(1.0, max(0.0, 1.0 - p))
        raise ArithmeticError("incomplete-gamma series did not converge")
    b = x + 1.0 - shape
    c = 1.0 / floor
    d = 1.0 / b
    result = d
    for iteration in range(1, 10000):
        an = -iteration * (iteration - shape)
        b += 2.0
        d = an * d + b
        d = floor if abs(d) < floor else d
        c = b + an / c
        c = floor if abs(c) < floor else c
        d = 1.0 / d
        delta = d * c
        result *= delta
        if abs(delta - 1.0) <= epsilon:
            return result * math.exp(-x + shape * math.log(x) - math.lgamma(shape))
    raise ArithmeticError("incomplete-gamma continued fraction did not converge")


def adjust_probabilities(
    hypotheses: list[dict[str, Any]], method: str
) -> dict[str, float]:
    count = len(hypotheses)
    raw = [float(row["raw_probability"]) for row in hypotheses]
    identities = [str(row["hypothesis_id"]) for row in hypotheses]
    output = [0.0] * count
    if method == "none":
        output = raw[:]
    elif method == "bonferroni":
        output = [min(1.0, value * count) for value in raw]
    elif method == "sidak":
        output = [1.0 if value >= 1.0 else -math.expm1(count * math.log1p(-value)) for value in raw]
    elif method == "holm":
        order = sorted(range(count), key=lambda index: (raw[index], identities[index]))
        running = 0.0
        for rank, index in enumerate(order):
            running = min(1.0, max(running, raw[index] * (count - rank)))
            output[index] = running
    elif method == "benjamini_hochberg":
        order = sorted(range(count), key=lambda index: (raw[index], identities[index]))
        running = 1.0
        for rank_zero in reversed(range(count)):
            index = order[rank_zero]
            running = min(running, raw[index] * count / (rank_zero + 1), 1.0)
            output[index] = running
    else:
        raise ValueError(f"unknown multiplicity method {method}")
    return dict(zip(identities, output, strict=True))


def validate_compilation(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    receipt = cell["compiler_receipt"]
    plan = cell["compiled_plan"]
    execution = cell["execution_plan"]
    checks.require(
        f"{prefix}.production_compiler_target",
        receipt.get("target") == "mga_multigroup_v1"
        and plan.get("kind") == "mga_multigroup_v1"
        and execution.get("contract") == "qpls.mga.execution_plan.v1",
        observed={
            "target": receipt.get("target"),
            "plan_kind": plan.get("kind"),
            "execution_contract": execution.get("contract"),
        },
        expected="compiled raw MGA production authority",
    )
    hashes = [
        receipt.get("recipe_analytical_sha256", ""),
        receipt.get("config_sha256", ""),
        receipt.get("model_scientific_sha256", ""),
        receipt.get("dataset_fingerprint", ""),
        receipt.get("plan_sha256", ""),
        receipt.get("analytical_identity_sha256", ""),
        execution.get("plan_sha256", ""),
        cell.get("finalized_cache_sha256", ""),
    ]
    checks.require(
        f"{prefix}.stable_hash_receipts",
        is_dataset_fingerprint(hashes[3])
        and all(is_lower_sha256(value) for index, value in enumerate(hashes) if index != 3),
        observed=hashes,
        expected="seven bare lowercase SHA-256 identities plus one bare/v2 dataset fingerprint",
    )
    checks.require(
        f"{prefix}.resumable_no_retry_plan",
        execution.get("retry_policy") == "none"
        and len(execution.get("shards", [])) > cell["group_count"],
        observed={
            "retry_policy": execution.get("retry_policy"),
            "shards": len(execution.get("shards", [])),
        },
        expected="deterministic non-retrying sharded plan",
    )


def validate_general_model(cell: dict[str, Any], checks: Checks) -> None:
    model = cell["sem_model_authority"]
    variables = {row["id"]: row for row in model["variables"]}
    relations = model["relations"]
    effect_counts = Counter(
        row["construct"] for row in relations if row["kind"] == "measurement_effect"
    )
    causal_counts = Counter(
        row["composite"] for row in relations if row["kind"] == "measurement_causal"
    )
    control_rows = [
        row
        for row in relations
        if row["kind"] == "structural" and row.get("role", "structural") == "control"
    ]
    observed_group = variables.get("observed:multimod_group", {})
    checks.require(
        "mga.profile.general_sem.mixed_measurement_authority",
        variables["construct:x"].get("weighting", {}).get("kind") == "mode_a"
        and variables["construct:z"].get("weighting", {}).get("kind") == "mode_b"
        and variables["construct:y"].get("weighting", {}).get("kind") == "mode_a"
        and effect_counts["construct:x"] == 1
        and effect_counts["construct:y"] == 3
        and causal_counts["construct:z"] == 3
        and len(control_rows) == 1,
        observed={
            "effect_counts": effect_counts,
            "causal_counts": causal_counts,
            "control_count": len(control_rows),
        },
        expected="single-item Mode A + three-indicator Mode B + reflective outcome + control",
    )
    indicator_sources = {
        variables[row["indicator"]]["source_column"]
        for row in relations
        if row["kind"] in {"measurement_effect", "measurement_causal"}
    }
    checks.require(
        "mga.profile.grouping_column_not_indicator",
        observed_group.get("source_column") == "group" and "group" not in indicator_sources,
        observed={"group_source": observed_group.get("source_column"), "indicators": sorted(indicator_sources)},
        expected="group is directly observed and absent from indicators",
    )


def validate_evidence_cell(
    cell: dict[str, Any],
    checks: Checks,
    prefix: str,
    *,
    expect_fixture_exclusions: bool = True,
) -> None:
    group_count = int(cell["group_count"])
    expected_pairs = group_count * (group_count - 1) // 2
    evidence = cell["evidence"]
    permutations = evidence["pairwise_permutation"]
    micom = evidence["micom"]
    checks.require(
        f"{prefix}.pairwise_evidence_inventory",
        len(permutations) == expected_pairs and len(micom) == expected_pairs,
        observed={"permutations": len(permutations), "micom": len(micom)},
        expected=expected_pairs,
    )
    permutation_by_pair = {pair_indices(row): row for row in permutations}
    micom_by_pair = {pair_indices(row): row for row in micom}
    shared = True
    complete = True
    for pair, permutation in permutation_by_pair.items():
        micom_row = micom_by_pair.get(pair)
        shared &= micom_row is not None and (
            micom_row["partition_plan_sha256"] == permutation["plan_sha256"]
        )
        complete &= (
            permutation["requested"] == permutation["attempted"] == 5000
            and permutation["usable"] >= permutation["minimum_usable"] >= 4500
            and permutation["retry_policy"] == "none"
            and permutation["availability"] == "available"
            and micom_row is not None
            and micom_row["requested_permutations"] == 5000
            and micom_row["usable_permutations"] >= micom_row["minimum_usable_permutations"] >= 4500
            and micom_row["complete"] is True
        )
    checks.require(
        f"{prefix}.micom_permutation_shared_partition_authority",
        shared,
        observed=len(permutation_by_pair),
        expected="identical pair-specific partition plan hashes",
    )
    checks.require(
        f"{prefix}.usable_draw_contract",
        complete,
        observed="all pairwise MGA and MICOM ledgers inspected",
        expected="5000 attempted, >=4500 usable, no retries",
    )
    exclusions = Counter(row["reason"] for row in cell["analysis"]["excluded_rows"])
    tokens = [row["stable_row_token"] for row in cell["analysis"]["excluded_rows"]]
    expected_exclusions = (
        Counter(
            {
                "unselected_group_value": 1,
                "missing_group_value": 1,
                "missing_model_value": 1,
            }
        )
        if expect_fixture_exclusions
        else Counter()
    )
    checks.require(
        f"{prefix}.row_exclusion_receipts",
        exclusions == expected_exclusions and len(tokens) == len(set(tokens)),
        observed={"reasons": exclusions, "tokens": tokens},
        expected=(
            "three unique stable exclusion receipts"
            if expect_fixture_exclusions
            else "no exclusions in the mechanically expanded reference"
        ),
    )
    public_rows = cell["analysis"]["pairwise"]
    comparability_coherent = all(
        row["measurement_comparability_satisfied"] == (not row["interpretation_blocked"])
        for row in public_rows
    )
    henseler_rows = [row for row in public_rows if "henseler" in row["procedure"]]
    checks.require(
        f"{prefix}.comparability_gate_coherent",
        comparability_coherent,
        observed=len(public_rows),
        expected="each affected contrast is published or blocked by MICOM consistently",
    )
    checks.require(
        f"{prefix}.henseler_directional_semantics",
        bool(henseler_rows)
        and all(
            row.get("raw_p_value") is None
            and row.get("directional_probability") is not None
            and 0.0 <= row["directional_probability"] <= 1.0
            for row in henseler_rows
        ),
        observed=len(henseler_rows),
        expected="directional probability only; never an ordinary p-value",
    )


def validate_raw_inference_reconstruction(
    cell: dict[str, Any], checks: Checks, prefix: str
) -> None:
    pairwise = next(
        (
            row
            for row in cell["evidence"]["pairwise_permutation"]
            if set(pair_indices(row)) == {0, 1} and row.get("audit_null_difference")
        ),
        None,
    )
    checks.require(
        f"{prefix}.raw_pairwise_null_trace_present",
        pairwise is not None,
        observed=pairwise is not None,
        expected="production-refit null differences for the canonical first pair",
    )
    if pairwise is not None:
        audit = pairwise["audit_null_difference"]
        values = [float(value) for value in audit["null_differences"]]
        observed = float(audit["observed_difference_a_minus_b"])
        parameter_id = target_id(audit["parameter"])
        parameter = next(
            row for row in pairwise["parameters"] if target_id(row["parameter"]) == parameter_id
        )
        usable = len(values)
        p_two_sided = (1 + sum(abs(value) >= abs(observed) for value in values)) / (usable + 1)
        p_greater = (1 + sum(value >= observed for value in values)) / (usable + 1)
        p_less = (1 + sum(value <= observed for value in values)) / (usable + 1)
        public = next(
            row
            for row in cell["analysis"]["pairwise"]
            if normalize_frequency_procedure(row["procedure"]) == PAIRWISE_PERMUTATION
            and row["target_id"] == parameter_id
            and {row["left_group_id"], row["right_group_id"]}
            == {"group_01", "group_02"}
        )
        checks.require(
            f"{prefix}.pairwise_probability_from_raw_null",
            usable == pairwise["usable"]
            and sha256_f64_series(values) == audit["null_differences_sha256"]
            and audit["null_differences_sha256"] == parameter["null_differences_sha256"]
            and close(p_two_sided, parameter["p_value_two_sided"])
            and close(p_greater, parameter["p_value_greater"])
            and close(p_less, parameter["p_value_less"])
            and close(public["raw_p_value"], p_two_sided),
            observed={
                "usable": usable,
                "p_two_sided": p_two_sided,
                "p_greater": p_greater,
                "p_less": p_less,
            },
            expected="plus-one tails reconstructed from production-refit null values",
        )

    micom = next(
        (
            row
            for row in cell["evidence"]["micom"]
            if set(pair_indices(row)) == {0, 1} and row.get("audit_step2")
        ),
        None,
    )
    checks.require(
        f"{prefix}.raw_micom_step2_trace_present",
        micom is not None,
        observed=micom is not None,
        expected="production-refit compositional-correlation null series",
    )
    if micom is not None:
        audit = micom["audit_step2"]
        values = [float(value) for value in audit["permutation_compositional_correlations"]]
        observed = float(audit["observed_compositional_correlation"])
        construct = next(
            row for row in micom["constructs"] if row["construct_id"] == audit["construct_id"]
        )
        lower = type7(values, 0.05)
        probability = (1 + sum(value <= observed for value in values)) / (len(values) + 1)
        invariant = observed >= lower
        mean_values = [float(value) for value in audit["permutation_mean_differences"]]
        variance_values = [float(value) for value in audit["permutation_log_variance_ratios"]]
        observed_mean = float(audit["observed_mean_difference_a_minus_b"])
        observed_variance = float(audit["observed_log_variance_ratio_a_minus_b"])
        mean_probability = (
            1 + sum(abs(value) >= abs(observed_mean) for value in mean_values)
        ) / (len(mean_values) + 1)
        variance_probability = (
            1 + sum(abs(value) >= abs(observed_variance) for value in variance_values)
        ) / (len(variance_values) + 1)
        checks.require(
            f"{prefix}.micom_step2_from_raw_null",
            len(values) == micom["usable_permutations"]
            and sha256_f64_series(values) == audit["permutation_values_sha256"]
            and audit["permutation_values_sha256"]
            == construct["permutation_compositional_correlations_sha256"]
            and close(lower, construct["compositional_lower_quantile"])
            and close(probability, construct["compositional_invariance_probability"])
            and construct["compositional_invariance"] is invariant
            and construct["partial_measurement_invariance"]
            is (micom["complete"] and invariant),
            observed={"lower": lower, "probability": probability, "invariant": invariant},
            expected="Type-7 lower quantile, plus-one probability, and Step-2 decision",
        )
        checks.require(
            f"{prefix}.micom_step3_from_raw_null",
            len(mean_values) == len(variance_values) == micom["usable_permutations"]
            and sha256_f64_series(mean_values)
            == audit["permutation_mean_differences_sha256"]
            and sha256_f64_series(variance_values)
            == audit["permutation_log_variance_ratios_sha256"]
            and close(mean_probability, construct["mean_difference_two_sided_probability"])
            and close(
                variance_probability, construct["variance_difference_two_sided_probability"]
            )
            and construct["equal_means"] is (mean_probability >= 0.05)
            and construct["equal_variances"] is (variance_probability >= 0.05)
            and construct["full_measurement_invariance"]
            is (
                micom["complete"]
                and invariant
                and mean_probability >= 0.05
                and variance_probability >= 0.05
            ),
            observed={
                "mean_probability": mean_probability,
                "variance_probability": variance_probability,
            },
            expected="plus-one two-sided Step-3 mean/variance decisions for bounded audit construct",
        )

    banks = cell["evidence"]["bootstrap_banks"]
    has_raw_bootstrap = bool(
        banks
        and banks[0].get("groups")
        and banks[0]["groups"][0].get("replicate_estimates")
    )
    checks.require(
        f"{prefix}.raw_bootstrap_trace_present",
        has_raw_bootstrap,
        observed=has_raw_bootstrap,
        expected="shared production-refit bootstrap bank with retained replicate estimates",
    )
    if has_raw_bootstrap:
        bank = banks[0]
        left = bank["groups"][0]
        right = bank["groups"][1]
        parameter_id = target_id(bank["parameters"][0])
        differences = [
            left_draw[0] - right_draw[0]
            for left_draw, right_draw in zip(
                left["replicate_estimates"], right["replicate_estimates"], strict=True
            )
            if left_draw is not None and right_draw is not None
        ]
        greater = sum(value > 0.0 for value in differences)
        equal = sum(value == 0.0 for value in differences)
        probability = (greater + 0.5 * equal) / len(differences)
        public = next(
            row
            for row in cell["analysis"]["pairwise"]
            if normalize_frequency_procedure(row["procedure"]) == HENSELER
            and row["target_id"] == parameter_id
            and row["left_group_id"] == "group_01"
            and row["right_group_id"] == "group_02"
        )
        checks.require(
            f"{prefix}.henseler_probability_from_raw_bootstrap",
            len(differences) >= bank["minimum_usable"]
            and close(probability, public["directional_probability"]),
            observed={"usable": len(differences), "probability": probability},
            expected="Pr(A*>B*) plus half of ties from the shared bootstrap bank",
        )


def validate_general_target_families(cell: dict[str, Any], checks: Checks) -> None:
    families = {
        parameter["parameter"]["family"]
        for row in cell["evidence"]["pairwise_permutation"]
        for parameter in row["parameters"]
    }
    checks.require(
        "mga.profile.general_sem.full_target_family_inventory",
        families == {"structural_path", "outer_loading", "outer_weight", "r_squared"},
        observed=sorted(families),
        expected=["outer_loading", "outer_weight", "r_squared", "structural_path"],
    )


def validate_omnibus(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    group_count = int(cell["group_count"])
    evidence = cell["evidence"]["omnibus_permutation"]
    if group_count == 2:
        checks.require(
            f"{prefix}.no_two_group_omnibus",
            evidence == [] and cell["analysis"]["omnibus"] == [],
            observed={"evidence": len(evidence), "rows": len(cell["analysis"]["omnibus"])},
            expected="no invented two-group omnibus claim",
        )
        return
    checks.require(
        f"{prefix}.omnibus_precedes_pairwise",
        len(evidence) == 1
        and evidence[0]["requested"] == evidence[0]["attempted"] == 5000
        and evidence[0]["usable"] >= evidence[0]["minimum_usable"] >= 4500,
        observed=len(evidence),
        expected="one complete max-spread omnibus permutation",
    )
    if not evidence:
        return
    omnibus = evidence[0]
    all_tail_reconstructions_match = bool(omnibus["parameters"])
    for index, parameter in enumerate(omnibus["parameters"]):
        values = [float(group["values"][index]) for group in omnibus["group_point_estimates"]]
        recomputed = max(values) - min(values)
        null_spreads = [
            float(value) for value in parameter["null_maximum_pairwise_spreads"]
        ]
        probability = (
            1 + sum(value >= recomputed for value in null_spreads)
        ) / (len(null_spreads) + 1)
        parameter_id = target_id(parameter["parameter"])
        public = next(
            row
            for row in cell["analysis"]["omnibus"]
            if normalize_frequency_procedure(row["procedure"]) == OMNIBUS
            and row["target_id"] == parameter_id
        )
        reconstruction_matches = (
            close(recomputed, float(parameter["observed_maximum_pairwise_spread"]))
            and len(null_spreads) == omnibus["usable"]
            and all(math.isfinite(value) and value >= 0.0 for value in null_spreads)
            and close(probability, float(parameter["p_value_right_tailed"]))
            and close(probability, float(public["p_value"]))
        )
        all_tail_reconstructions_match &= reconstruction_matches
        checks.require(
            f"{prefix}.max_spread.{parameter_id}",
            reconstruction_matches,
            observed={
                "spread": parameter["observed_maximum_pairwise_spread"],
                "usable": len(null_spreads),
                "probability": probability,
                "public_probability": public["p_value"],
            },
            expected={
                "spread": recomputed,
                "usable": omnibus["usable"],
                "probability": "plus-one right tail reconstructed from retained null spreads",
            },
            detail="independent max-minus-min and omnibus null-tail calculation",
        )
    checks.require(
        f"{prefix}.omnibus_probability_from_raw_null",
        all_tail_reconstructions_match,
        observed=len(omnibus["parameters"]),
        expected="every omnibus target probability independently reconstructed from its raw null spreads",
    )


def validate_multiplicity(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    replay = cell["multiplicity_replays"]
    hypotheses = replay["hypotheses"]
    expected_methods = {"holm", "bonferroni", "sidak", "benjamini_hochberg", "none"}
    actual_methods = {row["method"] for row in replay["methods"]}
    checks.require(
        f"{prefix}.multiplicity_method_inventory",
        actual_methods == expected_methods,
        observed=sorted(actual_methods),
        expected=sorted(expected_methods),
    )
    for method_row in replay["methods"]:
        method = method_row["method"]
        expected = adjust_probabilities(hypotheses, method)
        actual = {
            row["hypothesis_id"]: float(row["adjusted_probability"])
            for row in method_row["probabilities"]
        }
        checks.require(
            f"{prefix}.multiplicity_formula.{method}",
            actual.keys() == expected.keys()
            and all(close(actual[key], expected[key]) for key in expected),
            observed=actual,
            expected=expected,
            detail="independent Holm/Bonferroni/Sidak/BH/none calculation",
        )
    public_hypotheses = []
    public_by_identity = {}
    for row in cell["analysis"]["pairwise"]:
        if row.get("raw_p_value") is None:
            continue
        identity = (
            f"{row['procedure']}:{row['left_group_id']}:"
            f"{row['right_group_id']}:{row['target_id']}"
        )
        public_hypotheses.append(
            {"hypothesis_id": identity, "raw_probability": row["raw_p_value"]}
        )
        public_by_identity[identity] = row.get("adjusted_p_value")
    expected_public_holm = adjust_probabilities(public_hypotheses, "holm")
    checks.require(
        f"{prefix}.public_holm_adjusted_probabilities",
        public_by_identity.keys() == expected_public_holm.keys()
        and all(
            public_by_identity[identity] is not None
            and close(float(public_by_identity[identity]), probability)
            for identity, probability in expected_public_holm.items()
        ),
        observed=public_by_identity,
        expected=expected_public_holm,
        detail="independent Holm calculation checked against the published analysis rows",
    )


def validate_parametric(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    evidence = cell["evidence"]
    group_receipts: dict[tuple[str, int], dict[str, Any]] = {}
    for row in evidence["parametric_group_se"]:
        group_receipts[(target_id(row["parameter"]), int(row["group"]))] = row["receipt"]
        receipt = row["receipt"]
        reconstructed = (
            receipt["residual_sum_of_squares"]
            / receipt["variance_degrees_of_freedom"]
            * receipt["coefficient_variance_factor"]
        )
        checks.require(
            f"{prefix}.score_conditional_se.{target_id(row['parameter'])}.group_{row['group']}",
            close(math.sqrt(reconstructed), float(receipt["standard_error"])),
            observed=receipt["standard_error"],
            expected=math.sqrt(reconstructed),
        )
    for ordinal, test in enumerate(evidence["parametric"]):
        pair = pair_indices(test)
        # The raw evidence is ordered by the sole selected structural target.
        parameter_ids = {key[0] for key in group_receipts}
        if len(parameter_ids) != 1:
            checks.require(
                f"{prefix}.parametric_parameter_identity.{ordinal}",
                False,
                observed=sorted(parameter_ids),
                expected="one selected structural target",
            )
            continue
        parameter = next(iter(parameter_ids))
        left = group_receipts[(parameter, pair[0])]
        right = group_receipts[(parameter, pair[1])]
        difference = left["estimate"] - right["estimate"]
        if test["method"] == "pooled_equal_residual_variance":
            df = left["variance_degrees_of_freedom"] + right["variance_degrees_of_freedom"]
            pooled = (left["residual_sum_of_squares"] + right["residual_sum_of_squares"]) / df
            se = math.sqrt(
                pooled
                * (
                    left["coefficient_variance_factor"]
                    + right["coefficient_variance_factor"]
                )
            )
        else:
            variance_left = left["standard_error"] ** 2
            variance_right = right["standard_error"] ** 2
            combined = variance_left + variance_right
            se = math.sqrt(combined)
            df = combined**2 / (
                variance_left**2 / left["variance_degrees_of_freedom"]
                + variance_right**2 / right["variance_degrees_of_freedom"]
            )
        statistic = difference / se
        p_two_sided = student_t_two_sided(statistic, df)
        checks.require(
            f"{prefix}.parametric_formula.{test['method']}.{pair[0]}_{pair[1]}",
            close(difference, test["difference_a_minus_b"])
            and close(se, test["standard_error_of_difference"])
            and close(df, test["degrees_of_freedom"])
            and close(statistic, test["t_statistic"])
            and close(p_two_sided, test["p_value_two_sided"], 2.0e-9),
            observed=test,
            expected={"difference": difference, "se": se, "df": df, "t": statistic, "p": p_two_sided},
        )
    for ordinal, wald in enumerate(evidence["wald"]):
        parameter_ids = {key[0] for key in group_receipts}
        if len(parameter_ids) != 1:
            continue
        parameter = next(iter(parameter_ids))
        receipts = [group_receipts[(parameter, int(group))] for group in wald["groups"]]
        weights = [1.0 / row["standard_error"] ** 2 for row in receipts]
        mean = sum(row["estimate"] * weight for row, weight in zip(receipts, weights, strict=True)) / sum(weights)
        chi_square = sum(
            (row["estimate"] - mean) ** 2 / row["standard_error"] ** 2 for row in receipts
        )
        df = len(receipts) - 1
        probability = gamma_q(df / 2.0, chi_square / 2.0)
        checks.require(
            f"{prefix}.wald_formula.{ordinal}",
            close(mean, wald["inverse_variance_weighted_mean"])
            and close(chi_square, wald["chi_square"])
            and df == wald["degrees_of_freedom"]
            and close(probability, wald["p_value_right_tailed"], 2.0e-9),
            observed=wald,
            expected={"mean": mean, "chi_square": chi_square, "df": df, "p": probability},
        )


def validate_bc(cell: dict[str, Any], checks: Checks, prefix: str) -> None:
    banks = cell["evidence"]["bootstrap_banks"]
    has_raw_bootstrap = bool(
        banks
        and banks[0].get("groups")
        and banks[0]["groups"][0].get("replicate_estimates")
    )
    checks.require(
        f"{prefix}.bc_raw_bootstrap_trace_present",
        has_raw_bootstrap,
        observed=has_raw_bootstrap,
        expected="retained shared bootstrap bank required for independent BC reconstruction",
    )
    if not has_raw_bootstrap:
        return
    bank = banks[0]
    parameter_ids = [target_id(parameter) for parameter in bank["parameters"]]
    rows = {
        (row["left_group_id"], row["right_group_id"], row["target_id"]): row
        for row in cell["analysis"]["pairwise"]
        if "bootstrap_difference_bc" in row["procedure"]
    }
    for left in range(len(bank["groups"])):
        for right in range(left + 1, len(bank["groups"])):
            left_bank = bank["groups"][left]
            right_bank = bank["groups"][right]
            for parameter_index, parameter in enumerate(parameter_ids):
                draws = [
                    left_draw[parameter_index] - right_draw[parameter_index]
                    for left_draw, right_draw in zip(
                        left_bank["replicate_estimates"],
                        right_bank["replicate_estimates"],
                        strict=True,
                    )
                    if left_draw is not None and right_draw is not None
                ]
                point = left_bank["point_estimates"][parameter_index] - right_bank["point_estimates"][parameter_index]
                lower, upper = independent_bc(point, draws, 0.95)
                public = rows[(f"group_{left + 1:02}", f"group_{right + 1:02}", parameter)]
                interval = public["interval"]
                checks.require(
                    f"{prefix}.bc_zero_acceleration_type7.{left}_{right}.{parameter}",
                    interval["family"] == "bias_corrected_zero_acceleration_bc"
                    and "bca" not in interval["family"].lower()
                    and close(lower, interval["lower"], 2.0e-9)
                    and close(upper, interval["upper"], 2.0e-9),
                    observed=interval,
                    expected={"family": "BC, not BCa", "lower": lower, "upper": upper},
                )


def validate_profile_matrix(report: dict[str, Any], checks: Checks) -> None:
    cells = report["profile_matrix"]
    actual = {cell["profile_fixture"] for cell in cells}
    checks.require(
        "mga.profile.non_cartesian_inventory",
        actual == EXPECTED_PROFILE_FIXTURES,
        observed=sorted(actual),
        expected=sorted(EXPECTED_PROFILE_FIXTURES),
    )
    expected_interactions = {
        "multiple_two_way": 2,
        "bounded_three_way": 4,
        "bounded_two_way_moderated_mediation": 1,
        "multiple_nonnested_hoc": 0,
        "case_weighted_pls": 0,
        "reflective_plsc": 0,
    }
    expected_hocs = {name: 0 for name in EXPECTED_PROFILE_FIXTURES}
    expected_hocs["multiple_nonnested_hoc"] = 4
    for cell in cells:
        fixture = cell["profile_fixture"]
        plan = cell["compiled_plan"]
        checks.require(
            f"mga.profile.{fixture}.compiled_identity",
            len(plan["interactions"]) == expected_interactions[fixture]
            and len(plan["hocs"]) == expected_hocs[fixture],
            observed={"interactions": len(plan["interactions"]), "hocs": len(plan["hocs"])},
            expected={"interactions": expected_interactions[fixture], "hocs": expected_hocs[fixture]},
        )
        model = cell["sem_model_authority"]
        if fixture == "case_weighted_pls":
            weight = model["data_binding"].get("weight")
            checks.require(
                "mga.profile.case_weighted.typed_binding",
                is_exact_weight_binding(weight, "case"),
                observed=weight,
                expected={
                    "kind": "case",
                    "variable": "observed:multimod_weight",
                },
            )
        if fixture == "reflective_plsc":
            constructs = [row for row in model["variables"] if row["kind"] == "composite"]
            effect_counts = Counter(
                row["construct"] for row in model["relations"] if row["kind"] == "measurement_effect"
            )
            checks.require(
                "mga.profile.reflective_plsc.full_refit_authority",
                all(row["weighting"]["kind"] == "mode_a" and effect_counts[row["id"]] >= 2 for row in constructs)
                and all(row["interpretation"] == "composite_invariance" for row in cell["analysis"]["micom_pairs"]),
                observed={"constructs": len(constructs), "micom_rows": len(cell["analysis"]["micom_pairs"])},
                expected="multi-indicator Mode-A PLSc with composite-invariance wording",
            )
        validate_compilation(cell, checks, f"mga.profile.{fixture}")
        validate_evidence_cell(cell, checks, f"mga.profile.{fixture}")
        validate_multiplicity(cell, checks, f"mga.profile.{fixture}")
        validate_raw_inference_reconstruction(cell, checks, f"mga.profile.{fixture}")
        validate_bc(cell, checks, f"mga.profile.{fixture}")


def normalize_frequency_procedure(value: str) -> str:
    return FREQUENCY_PROCEDURE_ALIASES.get(value, value)


def point_map_receipt(values: dict[tuple[str, str], float]) -> list[dict[str, Any]]:
    return [
        {"group_id": group_id, "target_id": target_id_value, "estimate": estimate}
        for (group_id, target_id_value), estimate in sorted(values.items())
    ]


def bootstrap_pair_differences(
    bank: dict[str, Any], parameter_index: int
) -> list[float]:
    left, right = bank["groups"][:2]
    return [
        float(left_draw[parameter_index]) - float(right_draw[parameter_index])
        for left_draw, right_draw in zip(
            left["replicate_estimates"], right["replicate_estimates"], strict=True
        )
        if left_draw is not None and right_draw is not None
    ]


def frequency_bootstrap_bank_contract(
    banks: Any, expected_method: str
) -> tuple[bool, dict[str, Any] | None, list[str], dict[str, Any]]:
    receipt: dict[str, Any] = {
        "bank_count": len(banks) if isinstance(banks, list) else None,
        "expected_method": expected_method,
    }
    if not isinstance(banks, list) or len(banks) != 1 or not isinstance(banks[0], dict):
        return False, None, [], receipt
    bank = banks[0]
    try:
        parameters = bank["parameters"]
        groups = bank["groups"]
        parameter_ids = [target_id(parameter) for parameter in parameters]
        group_ids = [int(group["group"]) for group in groups]
        attempted = int(bank["attempted"])
        minimum_usable = int(bank["minimum_usable"])
        valid = (
            bank["method_version"] == expected_method
            and int(bank["seed"]) == 42
            and int(bank["requested"]) == attempted == QUALIFICATION_RESAMPLES
            and minimum_usable >= QUALIFICATION_MINIMUM_USABLE
            and bank["retry_policy"] == "none"
            and bank["availability"] == "available"
            and bool(parameter_ids)
            and len(parameter_ids) == len(set(parameter_ids)) == len(parameters)
            and group_ids == [0, 1]
        )
        group_receipts = []
        for group in groups:
            replicate_estimates = group["replicate_estimates"]
            usable = int(group["usable"])
            failed = int(group["failed"])
            point_estimates = group["point_estimates"]
            present = [draw for draw in replicate_estimates if draw is not None]
            vectors_valid = all(
                isinstance(draw, list)
                and len(draw) == len(parameter_ids)
                and all(math.isfinite(float(value)) for value in draw)
                for draw in present
            )
            group_valid = (
                len(replicate_estimates) == attempted
                and usable + failed == attempted
                and len(present) == usable >= minimum_usable
                and len(point_estimates) == len(parameter_ids)
                and all(math.isfinite(float(value)) for value in point_estimates)
                and vectors_valid
            )
            valid &= group_valid
            group_receipts.append(
                {
                    "group": int(group["group"]),
                    "usable": usable,
                    "failed": failed,
                    "ledger_rows": len(replicate_estimates),
                    "valid": group_valid,
                }
            )
        receipt.update(
            {
                "method_version": bank["method_version"],
                "seed": int(bank["seed"]),
                "attempted": attempted,
                "minimum_usable": minimum_usable,
                "parameter_count": len(parameter_ids),
                "groups": group_receipts,
            }
        )
        return valid, bank, parameter_ids, receipt
    except (KeyError, TypeError, ValueError, OverflowError) as error:
        receipt["contract_error"] = f"{type(error).__name__}: {error}"
        return False, None, [], receipt


def validate_frequency_expansion(report: dict[str, Any], checks: Checks) -> None:
    section = report["frequency_expansion"]
    compact = section["compact_frequency_run"]
    expanded = section["physically_expanded_unweighted_run"]
    validate_evidence_cell(compact, checks, "mga.frequency")
    validate_evidence_cell(
        expanded,
        checks,
        "mga.frequency_expanded_reference",
        expect_fixture_exclusions=False,
    )
    compact_point_rows = compact["analysis"]["group_parameters"]
    expanded_point_rows = expanded["analysis"]["group_parameters"]
    compact_points = {
        (row["group_id"], row["parameter"]["target_id"]): row["parameter"]["estimate"]
        for row in compact_point_rows
    }
    expanded_points = {
        (row["group_id"], row["parameter"]["target_id"]): row["parameter"]["estimate"]
        for row in expanded_point_rows
    }
    compact_targets = Counter(target for _, target in compact_points)
    expanded_targets = Counter(target for _, target in expanded_points)
    point_inventory_matches = (
        bool(compact_points)
        and len(compact_points) == len(compact_point_rows)
        and len(expanded_points) == len(expanded_point_rows)
        and compact_points.keys() == expanded_points.keys()
        and set(group for group, _ in compact_points) == {"group_01", "group_02"}
        and all(count == 2 for count in compact_targets.values())
        and compact_targets == expanded_targets
    )
    checks.require(
        "mga.frequency.expanded_row_point_equivalence",
        point_inventory_matches
        and all(close(compact_points[key], expanded_points[key], 5.0e-9) for key in compact_points),
        observed=point_map_receipt(compact_points),
        expected=point_map_receipt(expanded_points),
    )
    compact_public_rows = compact["analysis"]["pairwise"]
    expanded_public_rows = expanded["analysis"]["pairwise"]
    compact_rows = {
        (
            normalize_frequency_procedure(row["procedure"]),
            row["left_group_id"],
            row["right_group_id"],
            row["target_id"],
        ): row
        for row in compact_public_rows
    }
    expanded_rows = {
        (row["procedure"], row["left_group_id"], row["right_group_id"], row["target_id"]): row
        for row in expanded_public_rows
    }
    procedure_inventory_matches = (
        bool(compact_rows)
        and len(compact_rows) == len(compact_public_rows)
        and len(expanded_rows) == len(expanded_public_rows)
        and compact_rows.keys() == expanded_rows.keys()
    )
    point_differences_match = point_inventory_matches and procedure_inventory_matches
    for key in compact_rows.keys() & expanded_rows.keys():
        _, left_group, right_group, parameter_id = key
        compact_expected = compact_points.get((left_group, parameter_id))
        compact_right = compact_points.get((right_group, parameter_id))
        expanded_expected = expanded_points.get((left_group, parameter_id))
        expanded_right = expanded_points.get((right_group, parameter_id))
        if None in (compact_expected, compact_right, expanded_expected, expanded_right):
            point_differences_match = False
            continue
        compact_difference = float(compact_expected) - float(compact_right)
        expanded_difference = float(expanded_expected) - float(expanded_right)
        point_differences_match &= close(
            float(compact_rows[key]["difference_left_minus_right"]),
            compact_difference,
            5.0e-9,
        ) and close(
            float(expanded_rows[key]["difference_left_minus_right"]),
            expanded_difference,
            5.0e-9,
        ) and close(
            compact_difference,
            expanded_difference,
            5.0e-9,
        )

    compact_pairwise = next(
        (
            row
            for row in compact["evidence"]["pairwise_permutation"]
            if pair_indices(row) == (0, 1) and row.get("audit_null_difference")
        ),
        None,
    )
    expanded_pairwise = next(
        (
            row
            for row in expanded["evidence"]["pairwise_permutation"]
            if pair_indices(row) == (0, 1) and row.get("audit_null_difference")
        ),
        None,
    )
    permutation_receipt: dict[str, Any] = {"available": False}
    permutation_law_matches = False
    if compact_pairwise is not None and expanded_pairwise is not None:
        compact_audit = compact_pairwise["audit_null_difference"]
        expanded_audit = expanded_pairwise["audit_null_difference"]
        compact_null = [
            float(value)
            for value in compact_audit["null_differences"]
        ]
        expanded_null = [
            float(value)
            for value in expanded_audit["null_differences"]
        ]
        distance = two_sample_ks_distance(compact_null, expanded_null)
        limit = ks_alpha_001_limit(len(compact_null), len(expanded_null))
        compact_parameter_id = target_id(compact_audit["parameter"])
        expanded_parameter_id = target_id(expanded_audit["parameter"])
        compact_observed = compact_points.get(("group_01", compact_parameter_id))
        compact_observed_right = compact_points.get(("group_02", compact_parameter_id))
        expanded_observed = expanded_points.get(("group_01", expanded_parameter_id))
        expanded_observed_right = expanded_points.get(("group_02", expanded_parameter_id))
        permutation_law_matches = (
            compact_pairwise["method_version"] == FREQUENCY_PAIRWISE_METHOD
            and expanded_pairwise["method_version"] == EXPANDED_PAIRWISE_METHOD
            and compact_pairwise["seed"] == expanded_pairwise["seed"] == 42
            and compact_pairwise["requested"]
            == compact_pairwise["attempted"]
            == expanded_pairwise["requested"]
            == expanded_pairwise["attempted"]
            == QUALIFICATION_RESAMPLES
            and compact_parameter_id == expanded_parameter_id
            and None
            not in (
                compact_observed,
                compact_observed_right,
                expanded_observed,
                expanded_observed_right,
            )
            and close(
                float(compact_audit["observed_difference_a_minus_b"]),
                float(compact_observed) - float(compact_observed_right),
                5.0e-9,
            )
            and close(
                float(expanded_audit["observed_difference_a_minus_b"]),
                float(expanded_observed) - float(expanded_observed_right),
                5.0e-9,
            )
            and len(compact_null) == compact_pairwise["usable"]
            and len(expanded_null) == expanded_pairwise["usable"]
            and len(compact_null) >= QUALIFICATION_MINIMUM_USABLE
            and len(expanded_null) >= QUALIFICATION_MINIMUM_USABLE
            and distance <= limit
        )
        permutation_receipt = {
            "available": True,
            "compact_draws": len(compact_null),
            "expanded_draws": len(expanded_null),
            "ks_distance": distance,
            "alpha_0_001_limit": limit,
        }
    checks.require(
        "mga.frequency.expanded_row_permutation_law_equivalence",
        permutation_law_matches,
        observed=permutation_receipt,
        expected="independent count-space and expanded-row null vectors pass the predeclared alpha .001 KS compatibility bound",
    )

    compact_banks = compact["evidence"]["bootstrap_banks"]
    expanded_banks = expanded["evidence"]["bootstrap_banks"]
    bootstrap_receipts: list[dict[str, Any]] = []
    compact_bank_valid, compact_bank, compact_parameter_ids, compact_bank_receipt = (
        frequency_bootstrap_bank_contract(compact_banks, FREQUENCY_BOOTSTRAP_METHOD)
    )
    expanded_bank_valid, expanded_bank, expanded_parameter_ids, expanded_bank_receipt = (
        frequency_bootstrap_bank_contract(expanded_banks, EXPANDED_BOOTSTRAP_METHOD)
    )
    checks.require(
        "mga.frequency.compact_bootstrap_envelope",
        compact_bank_valid,
        observed=compact_bank_receipt,
        expected="one canonical 5,000-draw frequency count-space bank",
    )
    checks.require(
        "mga.frequency.expanded_reference_bootstrap_envelope",
        expanded_bank_valid,
        observed=expanded_bank_receipt,
        expected="one canonical 5,000-draw physical-row bootstrap bank",
    )
    bootstrap_law_matches = (
        compact_bank_valid
        and expanded_bank_valid
        and compact_parameter_ids == expanded_parameter_ids
        and compact_bank is not None
        and expanded_bank is not None
    )
    if bootstrap_law_matches:
        for parameter_index, parameter_id in enumerate(compact_parameter_ids):
            compact_draws = bootstrap_pair_differences(compact_bank, parameter_index)
            expanded_draws = bootstrap_pair_differences(expanded_bank, parameter_index)
            distance = two_sample_ks_distance(compact_draws, expanded_draws)
            limit = ks_alpha_001_limit(len(compact_draws), len(expanded_draws))
            row_matches = (
                len(compact_draws) >= QUALIFICATION_MINIMUM_USABLE
                and len(expanded_draws) >= QUALIFICATION_MINIMUM_USABLE
                and distance <= limit
            )
            bootstrap_law_matches &= row_matches
            bootstrap_receipts.append(
                {
                    "target_id": parameter_id,
                    "compact_draws": len(compact_draws),
                    "expanded_draws": len(expanded_draws),
                    "ks_distance": distance,
                    "alpha_0_001_limit": limit,
                    "matches": row_matches,
                }
            )
    checks.require(
        "mga.frequency.expanded_row_bootstrap_law_equivalence",
        bootstrap_law_matches,
        observed=bootstrap_receipts,
        expected="every independent count-space and expanded-row bootstrap target passes the predeclared alpha .001 KS compatibility bound",
    )
    checks.require(
        "mga.frequency.expanded_row_inference_equivalence",
        procedure_inventory_matches
        and point_differences_match
        and permutation_law_matches
        and bootstrap_law_matches,
        observed={
            "procedure_inventory_matches": procedure_inventory_matches,
            "point_differences_match": point_differences_match,
            "permutation": permutation_receipt,
            "bootstrap": bootstrap_receipts,
        },
        expected="exact point/refit equivalence plus compatible independent count-space and expanded-row inference laws",
    )
    weight = compact["sem_model_authority"]["data_binding"].get("weight")
    checks.require(
        "mga.profile.frequency_weighted.typed_binding",
        is_exact_weight_binding(weight, "frequency")
        and section["compact_source_rows"] == 30
        and section["represented_rows"] == 60,
        observed={"weight": weight, "represented_rows": section["represented_rows"]},
        expected={
            "weight": {
                "kind": "frequency",
                "variable": "observed:multimod_weight",
            },
            "represented_rows": 60,
        },
    )
    validate_bc(compact, checks, "mga.frequency")
    validate_multiplicity(compact, checks, "mga.frequency")
    validate_raw_inference_reconstruction(compact, checks, "mga.frequency")
    validate_bc(expanded, checks, "mga.frequency_expanded_reference")
    validate_multiplicity(expanded, checks, "mga.frequency_expanded_reference")
    validate_raw_inference_reconstruction(
        expanded, checks, "mga.frequency_expanded_reference"
    )


def validate_label_reversal(report: dict[str, Any], checks: Checks) -> None:
    forward = report["label_reversal"]["forward"]
    reverse = report["label_reversal"]["reverse"]
    forward_evidence = forward["evidence"]["pairwise_permutation"][0]
    reverse_evidence = reverse["evidence"]["pairwise_permutation"][0]
    evidence_ok = (
        pair_indices(forward_evidence) == (0, 1)
        and pair_indices(reverse_evidence) == (1, 0)
        and forward_evidence["plan_sha256"] == reverse_evidence["plan_sha256"]
        and forward_evidence["ledger_partition_list_sha256"]
        == reverse_evidence["ledger_partition_list_sha256"]
    )
    for left, right in zip(
        forward_evidence["parameters"], reverse_evidence["parameters"], strict=True
    ):
        evidence_ok &= target_id(left["parameter"]) == target_id(right["parameter"])
        evidence_ok &= close(left["difference_a_minus_b"], -right["difference_a_minus_b"])
        evidence_ok &= close(left["p_value_two_sided"], right["p_value_two_sided"])
        evidence_ok &= close(left["p_value_greater"], right["p_value_less"])
        evidence_ok &= close(left["p_value_less"], right["p_value_greater"])
    checks.require(
        "mga.label_reversal.partition_sign_tail_invariance",
        evidence_ok,
        observed={
            "forward_plan": forward_evidence["plan_sha256"],
            "reverse_plan": reverse_evidence["plan_sha256"],
        },
        expected="same partitions, reversed signs, same two-sided p, swapped directional tails",
    )
    forward_rows = {
        (row["procedure"], row["target_id"]): row for row in forward["analysis"]["pairwise"]
    }
    reverse_rows = {
        (row["procedure"], row["target_id"]): row for row in reverse["analysis"]["pairwise"]
    }
    public_ok = forward_rows.keys() == reverse_rows.keys()
    for key in forward_rows.keys() & reverse_rows.keys():
        left = forward_rows[key]
        right = reverse_rows[key]
        public_ok &= close(left["difference_left_minus_right"], -right["difference_left_minus_right"])
        if left.get("raw_p_value") is not None:
            public_ok &= close(left["raw_p_value"], right["raw_p_value"])
        if left.get("directional_probability") is not None:
            public_ok &= close(left["directional_probability"] + right["directional_probability"], 1.0)
    checks.require(
        "mga.label_reversal.public_result_invariance",
        public_ok,
        observed=len(forward_rows),
        expected="all public signed contrasts reverse and nondirectional probabilities remain fixed",
    )
    validate_bc(forward, checks, "mga.label_forward")


def validate_boundaries(report: dict[str, Any], checks: Checks) -> None:
    boundaries = report["boundaries"]
    too_small = boundaries["minimum_complete_cases"]
    ten = boundaries["ten_case_boundary"]
    warning = boundaries["below_thirty_warning"]
    imbalance = boundaries["above_ten_to_one"]
    checks.require(
        "mga.boundary.minimum_ten_complete_cases",
        not too_small["eligible"]
        and any(row["code"] == "insufficient_complete_cases" for row in too_small["blockers"])
        and ten["eligible"],
        observed={"nine": too_small, "ten": ten},
        expected="9 blocked; 10 eligible",
    )
    checks.require(
        "mga.boundary.warn_below_thirty",
        warning["eligible"]
        and sum(row["code"] == "small_group" for row in warning["warnings"]) == 2,
        observed=warning["warnings"],
        expected="small-group warning for both groups",
    )
    checks.require(
        "mga.boundary.reject_above_ten_to_one",
        not imbalance["eligible"]
        and any(row["code"] == "extreme_group_imbalance" for row in imbalance["blockers"]),
        observed=imbalance,
        expected="11:1 blocked",
    )
    heavy_error = boundaries["heavy_run_without_confirmation"]
    checks.require(
        "mga.boundary.twenty_group_heavy_confirmation",
        isinstance(heavy_error, dict) and "heavy" in heavy_error.get("code", ""),
        observed=heavy_error,
        expected="stable heavy-run confirmation blocker",
    )
    directional = boundaries["directional_predeclarations"]
    checks.require(
        "mga.boundary.directional_alternatives_explicit",
        {row["alternative"] for row in directional} == {"less", "greater"}
        and all(row["validation"] is None for row in directional),
        observed=directional,
        expected="valid explicit less/greater predeclarations",
    )


def validate_report(report: dict[str, Any], checks: Checks) -> None:
    checks.require(
        "mga.receipt.identity",
        report.get("schema_version") == 1
        and report.get("suite_id") == SUITE_ID
        and report.get("scale") == "qualification"
        and report.get("qualification_claim") == "raw_sut_facts_for_independent_comparison_only",
        observed={key: report.get(key) for key in ("schema_version", "suite_id", "scale", "qualification_claim")},
        expected="qualification-scale raw-SUT receipt",
    )
    group_matrix = report["group_matrix"]
    group_counts = tuple(cell["group_count"] for cell in group_matrix)
    checks.require(
        "mga.group_matrix.exact_2_3_5_20",
        group_counts == EXPECTED_GROUPS,
        observed=group_counts,
        expected=EXPECTED_GROUPS,
    )
    for cell in group_matrix:
        group_count = int(cell["group_count"])
        prefix = f"mga.group_matrix.g{group_count}"
        expected_pair_count = group_count * (group_count - 1) // 2
        eligibility = cell["analysis"]["group_eligibility"]
        checks.require(
            f"{prefix}.group_and_pair_counts",
            len(eligibility) == group_count
            and all(row["eligible"] for row in eligibility)
            and cell["expected_pair_count"] == expected_pair_count
            and (cell["heavy_run_confirmed"] is (group_count == 20)),
            observed={
                "groups": len(eligibility),
                "pairs": cell["expected_pair_count"],
                "heavy": cell["heavy_run_confirmed"],
            },
            expected={"groups": group_count, "pairs": expected_pair_count, "heavy": group_count == 20},
        )
        expected_pair_ids = ordered_pairs(group_count)
        for procedure in (PAIRWISE_PERMUTATION, HENSELER, BC):
            pair_ids = {
                (row["left_group_id"], row["right_group_id"])
                for row in cell["analysis"]["pairwise"]
                if row["procedure"] == procedure
            }
            checks.require(
                f"{prefix}.pair_inventory.{procedure}",
                pair_ids == expected_pair_ids,
                observed=len(pair_ids),
                expected=len(expected_pair_ids),
            )
        validate_compilation(cell, checks, prefix)
        validate_evidence_cell(cell, checks, prefix)
        validate_omnibus(cell, checks, prefix)
        validate_multiplicity(cell, checks, prefix)
        validate_raw_inference_reconstruction(cell, checks, prefix)
        validate_bc(cell, checks, prefix)
        if group_count == 2:
            validate_general_model(cell, checks)
            validate_general_target_families(cell, checks)
        if group_count >= 3:
            procedures = {row["procedure"] for row in cell["analysis"]["omnibus"]}
            checks.require(
                f"{prefix}.omnibus_procedure_inventory",
                procedures == {OMNIBUS},
                observed=sorted(procedures),
                expected=[OMNIBUS],
            )
    parametric = report["parametric_sensitivity"]
    prefix = "mga.inference.parametric_sensitivity"
    checks.require(
        f"{prefix}.identity",
        parametric["profile_fixture"] == "general_sem_pls_parametric_sensitivity"
        and parametric["group_count"] == 3,
        observed={
            "profile": parametric["profile_fixture"],
            "groups": parametric["group_count"],
        },
        expected="structural-only three-group sensitivity cell",
    )
    expected_pairs = ordered_pairs(3)
    for procedure in (PAIRWISE_PERMUTATION, HENSELER, BC, POOLED, WELCH):
        pair_ids = {
            (row["left_group_id"], row["right_group_id"])
            for row in parametric["analysis"]["pairwise"]
            if row["procedure"] == procedure
        }
        checks.require(
            f"{prefix}.pair_inventory.{procedure}",
            pair_ids == expected_pairs,
            observed=len(pair_ids),
            expected=len(expected_pairs),
        )
    omnibus_procedures = {row["procedure"] for row in parametric["analysis"]["omnibus"]}
    checks.require(
        f"{prefix}.omnibus_inventory",
        omnibus_procedures == {OMNIBUS, WALD},
        observed=sorted(omnibus_procedures),
        expected=sorted({OMNIBUS, WALD}),
    )
    validate_compilation(parametric, checks, prefix)
    validate_evidence_cell(parametric, checks, prefix)
    validate_omnibus(parametric, checks, prefix)
    validate_multiplicity(parametric, checks, prefix)
    validate_parametric(parametric, checks, prefix)
    validate_raw_inference_reconstruction(parametric, checks, prefix)
    validate_profile_matrix(report, checks)
    validate_frequency_expansion(report, checks)
    validate_label_reversal(report, checks)
    validate_boundaries(report, checks)
    emitted = {row["check_id"] for row in checks.rows}
    checks.require(
        "mga.binding.required_check_inventory",
        BOUND_GATE_CHECK_IDS.issubset(emitted),
        observed=sorted(BOUND_GATE_CHECK_IDS - emitted),
        expected="every exact downstream scientific-check identity is emitted",
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    report = json.loads(args.input.read_text(encoding="utf-8"))
    checks = Checks()
    try:
        validate_report(report, checks)
    except Exception as error:  # Preserve a deterministic comparator failure receipt.
        checks.add(
            "mga.comparator.unhandled_exception",
            False,
            observed=f"{type(error).__name__}: {error}",
            expected="complete independent comparison",
        )
    result = {
        "schema_version": 1,
        "suite_id": "qpls.multimod.mga.independent-comparison.v1",
        "status": "passed" if checks.passed else "failed",
        "input_sha256": sha256_file(args.input),
        "results": {
            "mga": {
                "status": "passed" if checks.passed else "failed",
                "checks": checks.rows,
            }
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0 if checks.passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
