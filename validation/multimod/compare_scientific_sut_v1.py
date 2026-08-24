"""Independent comparator for the deterministic MultiMod SUT probe.

The Rust example emits facts from production kernels.  This script imports no
QuickPLS code and independently recomputes the frozen mathematical identities.
It returns stable failure codes and deterministic JSON suitable for a campaign
stdout artifact.  Passing this bounded slice is necessary, never sufficient,
for release qualification.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from reference_oracles_v1 import (
    adjust_probabilities,
    compile_path_polynomial,
    conditional_path_derivative,
    conditional_path_effect,
    conditional_probe_contrast,
    interventional_g_computation,
    log_sum_exp,
)


SCHEMA_VERSION = 1
ABSOLUTE_TOLERANCE = 1.0e-9


@dataclass
class CheckLedger:
    checks: list[dict[str, Any]] = field(default_factory=list)
    failures: list[str] = field(default_factory=list)

    def check(
        self,
        check_id: str,
        condition: bool,
        failure_code: str,
        detail: str,
    ) -> None:
        status = "passed" if condition else "failed"
        self.checks.append(
            {
                "check_id": check_id,
                "status": status,
                "failure_code": None if condition else failure_code,
                "detail": detail,
            }
        )
        if not condition:
            self.failures.append(failure_code)

    def close(self) -> dict[str, Any]:
        return {
            "status": "passed" if not self.failures else "failed",
            "checks": self.checks,
            "failure_codes": sorted(set(self.failures)),
        }


def close(left: float, right: float, tolerance: float = ABSOLUTE_TOLERANCE) -> bool:
    return math.isclose(float(left), float(right), rel_tol=tolerance, abs_tol=tolerance)


def canonical_digest(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode()
    return "sha256:" + hashlib.sha256(encoded).hexdigest()


def enum_codes(receipt: dict[str, Any]) -> set[str]:
    return {str(code) for code in receipt.get("codes", [])}


def ols_slope(x: list[float], y: list[float], rows: list[int]) -> float:
    mean_x = math.fsum(x[index] for index in rows) / len(rows)
    mean_y = math.fsum(y[index] for index in rows) / len(rows)
    denominator = math.fsum((x[index] - mean_x) ** 2 for index in rows)
    return math.fsum(
        (x[index] - mean_x) * (y[index] - mean_y) for index in rows
    ) / denominator


def validate_mga(section: dict[str, Any]) -> dict[str, Any]:
    ledger = CheckLedger()
    eligibility = {row["groups"]: row for row in section["eligibility"]}
    for groups in (2, 3, 5, 20):
        row = eligibility.get(groups, {})
        counts = row.get("group_counts", [])
        ledger.check(
            f"mga.eligibility.{groups}_groups",
            row.get("eligible") is True
            and len(counts) == groups
            and all(item.get("complete_cases") == 10 for item in counts)
            and row.get("pairwise_comparisons") == groups * (groups - 1) // 2,
            f"MMQ.MGA.ELIGIBILITY.{groups}_GROUPS",
            "2, 3, 5, and 20 typed groups must remain eligible at ten complete cases each",
        )

    boundary = section["boundary_failures"]
    insufficient_codes = {
        blocker["code"]
        for blocker in boundary["insufficient_complete_cases"].get("blockers", [])
    }
    imbalance_codes = {
        blocker["code"]
        for blocker in boundary["imbalance_above_ten_to_one"].get("blockers", [])
    }
    ledger.check(
        "mga.boundary.minimum_complete_cases",
        "insufficient_complete_cases" in insufficient_codes,
        "MMQ.MGA.BOUNDARY.MINIMUM_CASES",
        "nine complete cases must fail with the stable minimum-case blocker",
    )
    ledger.check(
        "mga.boundary.imbalance",
        "extreme_group_imbalance" in imbalance_codes,
        "MMQ.MGA.BOUNDARY.IMBALANCE",
        "imbalance strictly above ten-to-one must fail closed",
    )

    reversal = section["label_reversal"]
    x = [float(value) for value in reversal["x"]]
    y = [float(value) for value in reversal["y"]]
    a_rows = [int(value) for value in reversal["group_a_rows"]]
    b_rows = [int(value) for value in reversal["group_b_rows"]]
    expected_a = ols_slope(x, y, a_rows)
    expected_b = ols_slope(x, y, b_rows)
    forward_point = reversal["forward"]["point"][0]
    reverse_point = reversal["reverse"]["point"][0]
    forward_inference = reversal["forward"]["inference"][0]
    reverse_inference = reversal["reverse"]["inference"][0]
    ledger.check(
        "mga.label_reversal.independent_point",
        close(forward_point["estimate_a"], expected_a)
        and close(forward_point["estimate_b"], expected_b)
        and close(forward_point["difference_a_minus_b"], expected_a - expected_b),
        "MMQ.MGA.LABEL_REVERSAL.POINT_REFERENCE",
        "SUT A-minus-B point estimates must match independent OLS slopes",
    )
    ledger.check(
        "mga.label_reversal.sign_and_tail",
        close(
            forward_inference["difference_a_minus_b"],
            -float(reverse_inference["difference_a_minus_b"]),
        )
        and close(
            forward_inference["p_value_two_sided"],
            reverse_inference["p_value_two_sided"],
        )
        and close(
            forward_inference["p_value_greater"], reverse_inference["p_value_less"]
        )
        and close(
            forward_inference["p_value_less"], reverse_inference["p_value_greater"]
        )
        and reversal["forward"]["plan_sha256"] == reversal["reverse"]["plan_sha256"],
        "MMQ.MGA.LABEL_REVERSAL.INVARIANCE",
        "reversing group labels must negate signs, swap tails, and preserve two-sided evidence",
    )
    ledger.check(
        "mga.permutation.usable_ledger",
        reversal["forward"]["usable"] == 5_000
        and reversal["forward"]["failed"] == 0
        and reversal["reverse"]["usable"] == 5_000
        and reversal["reverse"]["failed"] == 0,
        "MMQ.MGA.PERMUTATION.USABLE",
        "the bounded slope fixture must retain every requested partition without replacement",
    )

    hypotheses = [
        {"id": row["hypothesis_id"], "probability": row["raw_probability"]}
        for row in section["multiplicity_input"]
    ]
    method_aliases = {
        "none": "none",
        "holm": "holm",
        "bonferroni": "bonferroni",
        "sidak": "sidak",
        "benjamini_hochberg": "benjamini_hochberg",
    }
    for receipt in section["multiplicity"]:
        method = receipt["method"]
        expected = adjust_probabilities(hypotheses, method_aliases[method])
        actual = {
            row["hypothesis_id"]: row["adjusted_probability"]
            for row in receipt["probabilities"]
        }
        ledger.check(
            f"mga.multiplicity.{method}",
            set(actual) == set(expected)
            and all(close(actual[key], expected[key], 1.0e-12) for key in expected),
            f"MMQ.MGA.MULTIPLICITY.{method.upper()}",
            f"{method} adjustments must match the independent frozen family calculation",
        )
    return ledger.close()


def adjusted_rand_index(left: list[int], right: list[int]) -> float:
    if len(left) != len(right) or not left:
        raise ValueError("ARI partitions must be non-empty and aligned")
    left_values = sorted(set(left))
    right_values = sorted(set(right))
    table = {
        (a, b): sum(1 for x, y in zip(left, right, strict=True) if x == a and y == b)
        for a in left_values
        for b in right_values
    }

    def pairs(count: int) -> int:
        return count * (count - 1) // 2

    sum_cells = sum(pairs(count) for count in table.values())
    sum_left = sum(pairs(left.count(value)) for value in left_values)
    sum_right = sum(pairs(right.count(value)) for value in right_values)
    total_pairs = pairs(len(left))
    expected = sum_left * sum_right / total_pairs
    maximum = 0.5 * (sum_left + sum_right)
    if close(maximum, expected, 1.0e-15):
        return 1.0
    return (sum_cells - expected) / (maximum - expected)


def recompute_fimix(section: dict[str, Any], ledger: CheckLedger) -> None:
    fit = section["fit"]
    ledger.check(
        "fimix.strong_separation.completed",
        fit.get("status") == "fit",
        "MMQ.FIMIX.RECOVERY.NO_FIT",
        "the deterministic strong-separation pilot must produce a stable fitted optimum",
    )
    if fit.get("status") != "fit":
        return
    result = fit["result"]
    equation = section["input"]["equations"][0]
    design = equation["design"]
    outcome = equation["outcome"]
    row_log_likelihoods: list[float] = []
    expected_posteriors: list[list[float]] = []
    for row_index, predictor_row in enumerate(design):
        log_joints = []
        for class_fit in result["classes"]:
            fitted_equation = class_fit["equations"][0]
            coefficients = {
                item["parameter_id"]: item["estimate"]
                for item in fitted_equation["coefficients"]
            }
            predicted = coefficients.get("(intercept)", 0.0)
            for predictor_id, predictor in zip(
                equation["predictor_ids"], predictor_row, strict=True
            ):
                predicted += coefficients[predictor_id] * predictor
            variance = fitted_equation["residual_variance"]
            residual = outcome[row_index] - predicted
            log_joints.append(
                math.log(class_fit["proportion"])
                - 0.5
                * (
                    math.log(2.0 * math.pi)
                    + math.log(variance)
                    + residual * residual / variance
                )
            )
        normalizer = log_sum_exp(log_joints)
        row_log_likelihoods.append(normalizer)
        expected_posteriors.append(
            [math.exp(value - normalizer) for value in log_joints]
        )
    expected_log_likelihood = math.fsum(row_log_likelihoods)
    posterior_difference = max(
        abs(expected - actual)
        for expected_row, actual_row in zip(
            expected_posteriors, result["posteriors"], strict=True
        )
        for expected, actual in zip(expected_row, actual_row, strict=True)
    )
    ledger.check(
        "fimix.likelihood.independent",
        close(result["log_likelihood"], expected_log_likelihood, 1.0e-8),
        "MMQ.FIMIX.LIKELIHOOD.IDENTITY",
        "stored observed-data likelihood must match independent log-sum-exp evaluation",
    )
    ledger.check(
        "fimix.posterior.independent",
        posterior_difference <= 1.0e-9
        and all(close(math.fsum(row), 1.0, 1.0e-10) for row in result["posteriors"]),
        "MMQ.FIMIX.POSTERIOR.IDENTITY",
        "every N-by-K posterior row must normalize and match independent Bayes probabilities",
    )

    criteria = result["criteria"]
    parameter_count = criteria["parameter_count"]
    observations = result["observations"]
    deviance = -2.0 * expected_log_likelihood
    expected_criteria = {
        "aic": deviance + 2.0 * parameter_count,
        "aic3": deviance + 3.0 * parameter_count,
        "aic4": deviance + 4.0 * parameter_count,
        "bic": deviance + parameter_count * math.log(observations),
        "caic": deviance + parameter_count * (math.log(observations) + 1.0),
        "hq": deviance + 2.0 * parameter_count * math.log(math.log(observations)),
    }
    ledger.check(
        "fimix.criteria.independent",
        parameter_count == 7
        and all(close(criteria[key], value, 1.0e-8) for key, value in expected_criteria.items()),
        "MMQ.FIMIX.CRITERIA.IDENTITY",
        "AIC/AIC3/AIC4/BIC/CAIC/HQ must use the optimized likelihood and full count",
    )
    raw_entropy = -math.fsum(
        probability * math.log(probability)
        for row in result["posteriors"]
        for probability in row
        if probability > 0.0
    )
    certainty = 1.0 - raw_entropy / (observations * math.log(len(result["classes"])))
    ledger.check(
        "fimix.entropy.independent",
        close(result["entropy"]["raw"], raw_entropy, 1.0e-9)
        and close(result["entropy"]["normalized_certainty"], certainty, 1.0e-9),
        "MMQ.FIMIX.ENTROPY.IDENTITY",
        "entropy and normalized classification certainty must match posterior identities",
    )
    tolerance = section["config"]["likelihood_decrease_tolerance"]
    traces_monotone = all(
        all(
            later["log_likelihood"] + tolerance >= earlier["log_likelihood"]
            for earlier, later in zip(start["trace"], start["trace"][1:])
        )
        for start in result["starts"]
        if start["converged"]
    )
    ledger.check(
        "fimix.trace.monotonicity",
        traces_monotone,
        "MMQ.FIMIX.LIKELIHOOD.MONOTONICITY",
        "every converged start must remain non-decreasing within the declared tolerance",
    )
    ari = adjusted_rand_index(
        [int(value) for value in section["true_assignments"]],
        [int(value) for value in result["hard_assignments"]],
    )
    ledger.check(
        "fimix.recovery.strong_separation_pilot",
        ari >= section["development_acceptance"]["minimum_ari"],
        "MMQ.FIMIX.RECOVERY.ARI",
        f"strong-separation pilot ARI was {ari:.12g}; minimum is 0.80",
    )
    ledger.check(
        "fimix.multistart.stability",
        result["stability"]["stable"] is True
        and len(result["stability"]["reproducing_start_indices"])
        >= result["stability"]["required_reproducing_starts"],
        "MMQ.FIMIX.MULTISTART.STABILITY",
        "at least two aligned starts must reproduce the selected optimum",
    )


def validate_fimix(section: dict[str, Any]) -> dict[str, Any]:
    ledger = CheckLedger()
    recompute_fimix(section, ledger)
    collapse = section["collapse_boundary"]
    codes = set(collapse.get("failure_codes", []))
    ledger.check(
        "fimix.collapse.variance_floor",
        collapse.get("status") == "blocked"
        and collapse.get("error_kind") == "no_converged_fimix_start"
        and collapse.get("start_count") == section["config"]["starts"]
        and "variance_collapse" in codes,
        "MMQ.FIMIX.COLLAPSE.VARIANCE",
        "an impossible residual-variance floor must fail starts with explicit collapse evidence",
    )
    return ledger.close()


def ols_fit(x: list[float], y: list[float], rows: list[int]) -> tuple[float, float, float]:
    mean_x = math.fsum(x[row] for row in rows) / len(rows)
    mean_y = math.fsum(y[row] for row in rows) / len(rows)
    ss_x = math.fsum((x[row] - mean_x) ** 2 for row in rows)
    slope = math.fsum(
        (x[row] - mean_x) * (y[row] - mean_y) for row in rows
    ) / ss_x
    intercept = mean_y - slope * mean_x
    total = math.fsum((y[row] - mean_y) ** 2 for row in rows)
    residual = math.fsum(
        (y[row] - intercept - slope * x[row]) ** 2 for row in rows
    )
    return intercept, slope, max(0.0, min(1.0, 1.0 - residual / total))


def validate_pos(section: dict[str, Any]) -> dict[str, Any]:
    ledger = CheckLedger()
    starts = section["start_plan"]
    minimum = section["config"]["minimum_segment_size"]
    starts_valid = len(starts) == 10 and all(
        len(start) == len(section["x"])
        and set(start) == {0, 1}
        and all(start.count(segment) >= minimum for segment in (0, 1))
        for start in starts
    )
    ledger.check(
        "pos.start_plan.identity",
        starts_valid
        and section["same_seed_start_plan_equal"] is True
        and section["same_k_partition_is_tenth_start"] is True,
        "MMQ.POS.START_PLAN.IDENTITY",
        "ten starts must reproduce by seed and preserve the supplied same-K tenth partition",
    )
    fit = section["fit"]
    ledger.check(
        "pos.fit.completed",
        fit.get("status") == "fit",
        "MMQ.POS.FIT.NO_COMPLETED_STABLE_OPTIMUM",
        "the deterministic full-refit P0 fixture must produce a stable POS result",
    )
    if fit.get("status") != "fit":
        return ledger.close()
    result = fit["result"]
    history = result["objective_history"]
    ledger.check(
        "pos.objective.monotonicity",
        len(history) == result["accepted_moves"] + 1
        and all(later > earlier for earlier, later in zip(history, history[1:])),
        "MMQ.POS.OBJECTIVE.MONOTONICITY",
        "every accepted move must strictly improve the complete refit objective",
    )
    x = [float(value) for value in section["x"]]
    y = [float(value) for value in section["y"]]
    assignments = [int(value) for value in result["assignments"]]
    independent_objective = 0.0
    segment_matches = True
    for segment_index, segment in enumerate(result["segments"]):
        rows = [row for row, assignment in enumerate(assignments) if assignment == segment_index]
        intercept, slope, r_squared = ols_fit(x, y, rows)
        signature = segment["fit"]["parameter_signature"]
        observed_r_squared = segment["fit"]["r_squared"][0]["r_squared"]
        independent_objective += r_squared
        segment_matches &= (
            segment["observations"] == len(rows)
            and close(signature[0], intercept, 1.0e-10)
            and close(signature[1], slope, 1.0e-10)
            and close(observed_r_squared, r_squared, 1.0e-10)
            and close(segment["objective_contribution"], r_squared, 1.0e-10)
        )
    ledger.check(
        "pos.full_refit.independent",
        segment_matches and close(result["objective"], independent_objective, 1.0e-10),
        "MMQ.POS.FULL_REFIT.IDENTITY",
        "segment coefficients, R-squared values, and the unweighted objective must reproduce",
    )
    ledger.check(
        "pos.multistart.stability",
        len(result["reproducing_start_indices"])
        >= section["config"]["required_reproducing_starts"],
        "MMQ.POS.MULTISTART.STABILITY",
        "two or more starts must reproduce the canonical partition and objective",
    )
    ari = adjusted_rand_index(
        [int(value) for value in section["true_assignments"]], assignments
    )
    ledger.check(
        "pos.identity.strong_fixture",
        ari >= 0.80,
        "MMQ.POS.IDENTITY.ARI",
        f"the deterministic slope-separation fixture returned ARI {ari:.12g}",
    )
    return ledger.close()


def oracle_edges(path: dict[str, Any]) -> list[dict[str, Any]]:
    translated = []
    for edge in path["edges"]:
        terms = [{"coefficient": edge["intercept"], "powers": {}}]
        terms.extend(
            {
                "coefficient": coefficient["estimate"],
                "powers": {coefficient["moderator_id"]: 1},
            }
            for coefficient in edge.get("linear_coefficients", [])
        )
        terms.extend(
            {
                "coefficient": coefficient["estimate"],
                "powers": {
                    coefficient["first_moderator_id"]: 1,
                    coefficient["second_moderator_id"]: 1,
                },
            }
            for coefficient in edge.get("pairwise_coefficients", [])
        )
        translated.append({"terms": terms})
    return translated


def sut_polynomial_terms(
    polynomial: dict[str, Any], moderators: list[str]
) -> dict[tuple[int, ...], float]:
    terms: dict[tuple[int, ...], float] = {}
    for term in polynomial["terms"]:
        powers = {item["moderator_id"]: item["exponent"] for item in term["powers"]}
        exponent = tuple(int(powers.get(moderator, 0)) for moderator in moderators)
        terms[exponent] = terms.get(exponent, 0.0) + float(term["coefficient"])
    return terms


def derivative_key(row: dict[str, Any]) -> tuple[str, str, str | None]:
    return (
        row["kind"],
        row["first_moderator_id"],
        row.get("second_moderator_id"),
    )


def validate_conditional_paths(section: dict[str, Any], ledger: CheckLedger) -> None:
    expected_lengths = {
        "first_stage": 2,
        "second_stage": 2,
        "both_stage": 2,
        "three_way": 2,
        "long_path_six_edges": 6,
    }
    for case in section["path_cases"]:
        label = case["label"]
        moderators = list(case["polynomial"]["moderator_ids"])
        reference = compile_path_polynomial(moderators, oracle_edges(case["input"]))
        sut = sut_polynomial_terms(case["polynomial"], moderators)
        polynomial_matches = set(reference) == set(sut) and all(
            close(reference[key], sut[key], 1.0e-12) for key in reference
        )
        ledger.check(
            f"conditional.{label}.polynomial",
            polynomial_matches
            and len(case["input"]["edges"]) == expected_lengths[label],
            f"MMQ.CONDITIONAL.{label.upper()}.POLYNOMIAL",
            "explicit edge functions must multiply into the independent path polynomial",
        )
        probe = case["joint_probe"]["standardized_values"]
        expected_effect = conditional_path_effect(reference, moderators, probe)
        ledger.check(
            f"conditional.{label}.effect",
            close(case["effect"]["estimate"], expected_effect, 1.0e-11),
            f"MMQ.CONDITIONAL.{label.upper()}.EFFECT",
            "conditional path effect must match independent polynomial evaluation",
        )
        derivative_matches = True
        for derivative in case["derivatives"]:
            key = derivative_key(derivative)
            orders = {key[1]: 1 if key[0] != "second" else 2}
            if key[0] == "cross":
                orders[key[2]] = 1
            expected = conditional_path_derivative(reference, moderators, probe, orders)
            derivative_matches &= close(derivative["estimate"], expected, 1.0e-11)
        ledger.check(
            f"conditional.{label}.derivatives",
            derivative_matches,
            f"MMQ.CONDITIONAL.{label.upper()}.DERIVATIVES",
            "first, pure-second, and cross derivatives must match symbolic differentiation",
        )
        expected_contrast = conditional_probe_contrast(
            reference,
            moderators,
            case["left_probe"]["standardized_values"],
            case["right_probe"]["standardized_values"],
        )
        ledger.check(
            f"conditional.{label}.contrast",
            close(case["contrast"]["estimate"], expected_contrast, 1.0e-11),
            f"MMQ.CONDITIONAL.{label.upper()}.CONTRAST",
            "the explicit finite contrast must remain left minus right",
        )
        scalar = case["scalar_index"]
        if label in {"first_stage", "second_stage"}:
            expected_index = reference.get((1,), 0.0)
            scalar_ok = scalar["status"] == "available" and close(
                scalar["value"]["estimate"], expected_index, 1.0e-12
            )
        elif label == "both_stage":
            scalar_ok = scalar["status"] == "blocked" and "ScalarIndexNotAffine" in scalar["error"]
        else:
            scalar_ok = scalar["status"] == "not_requested"
        ledger.check(
            f"conditional.{label}.scalar_index_boundary",
            scalar_ok,
            f"MMQ.CONDITIONAL.{label.upper()}.SCALAR_INDEX",
            "only a one-moderator affine selected-path effect may receive a scalar index",
        )


def sample_standardize(values: list[float]) -> tuple[list[float], float, float]:
    mean = math.fsum(values) / len(values)
    standard_deviation = math.sqrt(
        math.fsum((value - mean) ** 2 for value in values) / (len(values) - 1)
    )
    return [(value - mean) / standard_deviation for value in values], mean, standard_deviation


def solve_linear(matrix: list[list[float]], right: list[float]) -> list[float]:
    augmented = [row[:] + [value] for row, value in zip(matrix, right, strict=True)]
    size = len(augmented)
    for column in range(size):
        pivot = max(range(column, size), key=lambda row: abs(augmented[row][column]))
        augmented[column], augmented[pivot] = augmented[pivot], augmented[column]
        divisor = augmented[column][column]
        if abs(divisor) <= 1.0e-15:
            raise ValueError("independent normal equations are singular")
        for value_index in range(column, size + 1):
            augmented[column][value_index] /= divisor
        for row in range(size):
            if row == column:
                continue
            factor = augmented[row][column]
            for value_index in range(column, size + 1):
                augmented[row][value_index] -= factor * augmented[column][value_index]
    return [augmented[row][-1] for row in range(size)]


def edge_lookup(result: dict[str, Any], relation_id: str) -> dict[str, Any]:
    return next(edge for edge in result["edges"] if edge["relation_id"] == relation_id)


def validate_frequency_equivalence(section: dict[str, Any], ledger: CheckLedger) -> None:
    case = section["frequency_expanded_equivalence"]
    compact = case["compact_result"]
    expanded = case["expanded_result"]
    counts = [int(value) for value in case["counts"]]
    expanded_from_compact: dict[str, list[float]] = {}
    for variable_id, values in case["compact_scores"].items():
        expanded_from_compact[variable_id] = [
            float(value)
            for value, count in zip(values, counts, strict=True)
            for _ in range(count)
        ]
    ledger.check(
        "conditional.frequency.expansion_fixture",
        expanded_from_compact == case["expanded_scores"],
        "MMQ.CONDITIONAL.FREQUENCY.FIXTURE_EXPANSION",
        "the expanded reference rows must exactly reproduce positive integer counts",
    )
    compact_edges = {
        edge["relation_id"]: edge for edge in compact["edges"]
    }
    expanded_edges = {
        edge["relation_id"]: edge for edge in expanded["edges"]
    }
    edge_equivalence = set(compact_edges) == set(expanded_edges)
    for relation_id in compact_edges:
        left = compact_edges[relation_id]
        right = expanded_edges[relation_id]
        edge_equivalence &= close(left["intercept"], right["intercept"], 1.0e-11)
        left_gamma = left.get("linear_coefficients", [])
        right_gamma = right.get("linear_coefficients", [])
        edge_equivalence &= len(left_gamma) == len(right_gamma)
        edge_equivalence &= all(
            a["moderator_id"] == b["moderator_id"]
            and close(a["estimate"], b["estimate"], 1.0e-11)
            for a, b in zip(left_gamma, right_gamma, strict=True)
        )
    compact_scale = compact["product_scale_receipts"][0]
    expanded_scale = expanded["product_scale_receipts"][0]
    ledger.check(
        "conditional.frequency.compact_equals_expanded",
        edge_equivalence
        and close(
            compact_scale["weighted_product_mean"],
            expanded_scale["weighted_product_mean"],
            1.0e-12,
        )
        and close(
            compact_scale["weighted_product_standard_deviation"],
            expanded_scale["weighted_product_standard_deviation"],
            1.0e-12,
        )
        and compact["receipt"]["exact_frequency_row_expansion_equivalence"] is True
        and compact["receipt"]["represented_observation_count"] == sum(counts),
        "MMQ.CONDITIONAL.FREQUENCY.EXPANDED_EQUIVALENCE",
        "count-space moments and joint coefficients must equal physical row expansion",
    )

    x, _, _ = sample_standardize(expanded_from_compact["x"])
    z, _, _ = sample_standardize(expanded_from_compact["z"])
    y, _, _ = sample_standardize(expanded_from_compact["y"])
    raw_product = [left * right for left, right in zip(x, z, strict=True)]
    product, product_mean, product_sd = sample_standardize(raw_product)
    predictors = [x, z, product]
    normal = [
        [math.fsum(a * b for a, b in zip(left, right, strict=True)) for right in predictors]
        for left in predictors
    ]
    right = [
        math.fsum(a * b for a, b in zip(predictor, y, strict=True))
        for predictor in predictors
    ]
    beta_x, beta_z, beta_product = solve_linear(normal, right)
    expected_gamma = beta_product / product_sd
    x_edge = edge_lookup(compact, "rel:x:y")
    z_edge = edge_lookup(compact, "rel:z:y")
    observed_gamma = x_edge["linear_coefficients"][0]["estimate"]
    ledger.check(
        "conditional.frequency.independent_joint_fit",
        close(x_edge["intercept"], beta_x, 1.0e-10)
        and close(z_edge["intercept"], beta_z, 1.0e-10)
        and close(observed_gamma, expected_gamma, 1.0e-10)
        and close(compact_scale["weighted_product_mean"], product_mean, 1.0e-12)
        and close(
            compact_scale["weighted_product_standard_deviation"], product_sd, 1.0e-12
        ),
        "MMQ.CONDITIONAL.FREQUENCY.INDEPENDENT_FIT",
        "count-space beta and scientific gamma must match independent expanded-row OLS",
    )


def validate_conditional(section: dict[str, Any]) -> dict[str, Any]:
    ledger = CheckLedger()
    validate_conditional_paths(section, ledger)
    validate_frequency_equivalence(section, ledger)
    return ledger.close()


def validate_causal(section: dict[str, Any]) -> dict[str, Any]:
    ledger = CheckLedger()
    input_columns = {
        column["variable_id"]: column["values"] for column in section["input"]["columns"]
    }
    specification = {
        "treatment_id": "x",
        "contrast": {"x0": 0.0, "x1": 1.0},
        "baseline_rows": [{"c": value} for value in input_columns["c"]],
        "mediator_equations": [
            {
                "outcome": "m",
                "intercept": 1.0,
                "terms": [
                    {"coefficient": 2.0, "factors": ["x"]},
                    {"coefficient": 0.5, "factors": ["c"]},
                ],
            }
        ],
        "outcome_equation": {
            "intercept": 3.0,
            "terms": [
                {"coefficient": 1.0, "factors": ["x"]},
                {"coefficient": 4.0, "factors": ["m"]},
                {"coefficient": 0.25, "factors": ["c"]},
            ],
        },
    }
    expected = interventional_g_computation(specification)
    actual = section["known_target"]
    pairs = {
        "outcome_mean_x0_g_x0": "outcome_mean_x0_g_x0",
        "outcome_mean_x1_g_x0": "outcome_mean_x1_g_x0",
        "outcome_mean_x1_g_x1": "outcome_mean_x1_g_x1",
        "interventional_direct_effect": "interventional_direct_effect",
        "joint_interventional_indirect_effect": "joint_interventional_indirect_effect",
        "total_interventional_contrast": "total_interventional_contrast",
        "additive_decomposition_residual": "decomposition_residual",
    }
    ledger.check(
        "causal.g_computation.known_target",
        all(close(actual[left], expected[right], 1.0e-10) for left, right in pairs.items())
        and actual["positivity"]["x0_support_count"] == 20
        and actual["positivity"]["x1_support_count"] == 20,
        "MMQ.CAUSAL.G_COMPUTATION.KNOWN_TARGET",
        "observed OLS g-computation must recover the independent 1/8/9 decomposition",
    )
    interpretation = actual["interpretation"].lower()
    ledger.check(
        "causal.wording.assumption_dependent",
        "assumption-dependent" in interpretation and "causality is not established" in interpretation,
        "MMQ.CAUSAL.WORDING",
        "the module must not present an assumption-dependent estimate as established causality",
    )
    failures = section["assumption_failures"]
    expected_codes = {
        "incomplete_checklist": "identification_checklist_incomplete",
        "positivity": "positivity_failure",
        "unsupported_natural_effect": "unsupported_role_or_feature",
    }
    for failure_id, expected_code in expected_codes.items():
        receipt = failures[failure_id]
        ledger.check(
            f"causal.failure.{failure_id}",
            receipt.get("status") == "blocked" and expected_code in enum_codes(receipt),
            f"MMQ.CAUSAL.FAILURE.{failure_id.upper()}",
            f"{failure_id} must fail closed with {expected_code}",
        )
    return ledger.close()


VALIDATORS = {
    "mga": validate_mga,
    "fimix": validate_fimix,
    "pos": validate_pos,
    "conditional": validate_conditional,
    "causal": validate_causal,
}


def gate_input_material(gate: str, section: dict[str, Any]) -> dict[str, Any]:
    """Return fixture/config material only, never SUT estimates or decisions."""

    if gate == "mga":
        reversal = section["label_reversal"]
        return {
            "eligibility_group_counts": [2, 3, 5, 20],
            "complete_cases_per_group": 10,
            "x": reversal["x"],
            "y": reversal["y"],
            "group_a_rows": reversal["group_a_rows"],
            "group_b_rows": reversal["group_b_rows"],
            "permutations": 5_000,
            "seed": 42,
            "multiplicity_input": section["multiplicity_input"],
        }
    if gate == "fimix":
        return {
            "input": section["input"],
            "true_assignments": section["true_assignments"],
            "config": section["config"],
            "collapse_residual_variance_floor": 2.0,
        }
    if gate == "pos":
        return {
            "x": section["x"],
            "y": section["y"],
            "true_assignments": section["true_assignments"],
            "config": section["config"],
            "seed": 42,
        }
    if gate == "conditional":
        frequency = section["frequency_expanded_equivalence"]
        return {
            "paths": [case["input"] for case in section["path_cases"]],
            "joint_probes": [case["joint_probe"] for case in section["path_cases"]],
            "left_probes": [case["left_probe"] for case in section["path_cases"]],
            "right_probes": [case["right_probe"] for case in section["path_cases"]],
            "frequency_counts": frequency["counts"],
            "frequency_compact_scores": frequency["compact_scores"],
        }
    if gate == "causal":
        return {"input": section["input"]}
    raise ValueError(f"no input-material projection for gate {gate}")


SCOPE_GAPS = [
    "This bounded slice does not qualify any capability or promote any manifest.",
    "MGA checks one simple structural parameter; measurement, MICOM, omnibus, parametric, and weighted profile matrices remain separate gates.",
    "FIMIX recovery is one deterministic pilot seed; qualification still requires predeclared multi-seed median recovery, null, overlap, and boundary simulations.",
    "PLS-POS uses a validation identity-score OLS full-refit adapter; production measurement-scoring and common-metric contrast gates remain separate.",
    "Conditional polynomial and frequency identities do not replace full resampling-ledger, BCa, studentized, group, HOC, or weighted runner tests.",
    "Causal checks one exact linear observed-data DGP; broader continuous-treatment, positivity-strata, and simulation coverage remains separate.",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sut-json", type=Path, required=True)
    parser.add_argument(
        "--gate", choices=["all", *VALIDATORS], default="all"
    )
    parser.add_argument(
        "--expected-scale", choices=["development", "qualification"], required=True
    )
    parser.add_argument("--output", type=Path)
    return parser.parse_args()


def main() -> int:
    arguments = parse_args()
    try:
        document = json.loads(arguments.sut_json.read_text(encoding="utf-8"))
        if document.get("schema_version") != SCHEMA_VERSION:
            raise ValueError("unexpected SUT probe schema version")
        if document.get("scale") != arguments.expected_scale:
            raise ValueError("SUT probe scale does not match the command binding")
        requested = list(VALIDATORS) if arguments.gate == "all" else [arguments.gate]
        missing = [gate for gate in requested if gate not in document.get("sections", {})]
        if missing:
            raise ValueError(f"SUT probe omitted requested sections: {missing}")
        results = {
            gate: VALIDATORS[gate](document["sections"][gate]) for gate in requested
        }
        failure_codes = sorted(
            {
                code
                for result in results.values()
                for code in result["failure_codes"]
            }
        )
        input_material = {
            gate: gate_input_material(gate, document["sections"][gate])
            for gate in requested
        }
        output = {
            "schema_version": SCHEMA_VERSION,
            "gate_id": "qpls.multimod.scientific_sut_vs_reference.v1",
            "requested_gate": arguments.gate,
            "scale": arguments.expected_scale,
            "status": "passed" if not failure_codes else "failed",
            "seed": document.get("seed"),
            "input_digest": canonical_digest(input_material),
            "failure_codes": failure_codes,
            "results": results,
            "scope_gaps": SCOPE_GAPS,
        }
        rendered = json.dumps(output, indent=2, sort_keys=True, allow_nan=False) + "\n"
        if arguments.output:
            arguments.output.write_text(rendered, encoding="utf-8")
        else:
            sys.stdout.write(rendered)
        return 0 if not failure_codes else 1
    except Exception as error:  # deterministic harness failure envelope
        output = {
            "schema_version": SCHEMA_VERSION,
            "gate_id": "qpls.multimod.scientific_sut_vs_reference.v1",
            "requested_gate": arguments.gate,
            "scale": arguments.expected_scale,
            "status": "harness_error",
            "failure_codes": ["MMQ.HARNESS.INVALID_INPUT_OR_REFERENCE"],
            "detail": f"{type(error).__name__}: {error}",
            "scope_gaps": SCOPE_GAPS,
        }
        rendered = json.dumps(output, indent=2, sort_keys=True) + "\n"
        if arguments.output:
            arguments.output.write_text(rendered, encoding="utf-8")
        else:
            sys.stdout.write(rendered)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
