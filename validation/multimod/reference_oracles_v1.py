"""Independent, deterministic reference calculations for MultiMod qualification.

This module is validation-only.  It imports no QuickPLS code and uses only the
Python standard library so that the equations can be audited independently of
the shipped Rust implementation.  The functions intentionally cover frozen,
small identities; they are not production estimators and do not qualify a
QuickPLS capability without a separate system-under-test comparison.
"""

from __future__ import annotations

import itertools
import math
from collections.abc import Iterable, Mapping, Sequence
from typing import Any


class ReferenceOracleError(ValueError):
    """Raised when a frozen reference input violates the oracle contract."""


def _finite(value: Any, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ReferenceOracleError(f"{label} must be numeric")
    converted = float(value)
    if not math.isfinite(converted):
        raise ReferenceOracleError(f"{label} must be finite")
    return converted


def _ols_slope(x: Sequence[float], y: Sequence[float], indices: Sequence[int]) -> float:
    if len(indices) < 2:
        raise ReferenceOracleError("an OLS slope needs at least two rows")
    x_values = [_finite(x[index], f"x[{index}]") for index in indices]
    y_values = [_finite(y[index], f"y[{index}]") for index in indices]
    mean_x = math.fsum(x_values) / len(x_values)
    mean_y = math.fsum(y_values) / len(y_values)
    denominator = math.fsum((value - mean_x) ** 2 for value in x_values)
    if denominator <= 0.0:
        raise ReferenceOracleError("the permuted group has a singular OLS design")
    numerator = math.fsum(
        (x_value - mean_x) * (y_value - mean_y)
        for x_value, y_value in zip(x_values, y_values, strict=True)
    )
    return numerator / denominator


def exhaustive_pairwise_slope_permutation(
    x: Sequence[float],
    y: Sequence[float],
    group_a_indices: Sequence[int],
    group_b_indices: Sequence[int],
) -> dict[str, Any]:
    """Enumerate every fixed-size partition for an A-minus-B OLS path slope.

    Probabilities use the frozen add-one convention ``(extreme + 1) /
    (usable + 1)``.  No failed partition is replaced.
    """

    if len(x) != len(y) or not x:
        raise ReferenceOracleError("x and y must be non-empty and have equal length")
    all_rows = tuple(range(len(x)))
    group_a = tuple(sorted(group_a_indices))
    group_b = tuple(sorted(group_b_indices))
    if (
        not group_a
        or not group_b
        or set(group_a).intersection(group_b)
        or tuple(sorted(group_a + group_b)) != all_rows
    ):
        raise ReferenceOracleError("groups must be non-empty, disjoint, and cover all rows")

    estimate_a = _ols_slope(x, y, group_a)
    estimate_b = _ols_slope(x, y, group_b)
    observed = estimate_a - estimate_b
    absolute_extremes = 0
    greater_extremes = 0
    less_extremes = 0
    differences: list[float] = []
    all_row_set = set(all_rows)

    for selected_a in itertools.combinations(all_rows, len(group_a)):
        selected_a_set = set(selected_a)
        selected_b = tuple(sorted(all_row_set.difference(selected_a_set)))
        difference = _ols_slope(x, y, selected_a) - _ols_slope(x, y, selected_b)
        differences.append(difference)
        if abs(difference) >= abs(observed):
            absolute_extremes += 1
        if difference >= observed:
            greater_extremes += 1
        if difference <= observed:
            less_extremes += 1

    usable = len(differences)

    def add_one(extremes: int) -> float:
        return (extremes + 1.0) / (usable + 1.0)

    return {
        "estimate_a": estimate_a,
        "estimate_b": estimate_b,
        "difference_a_minus_b": observed,
        "usable_partitions": usable,
        "failed_partitions": 0,
        "absolute_extremes": absolute_extremes,
        "greater_extremes": greater_extremes,
        "less_extremes": less_extremes,
        "p_value_two_sided": add_one(absolute_extremes),
        "p_value_greater": add_one(greater_extremes),
        "p_value_less": add_one(less_extremes),
        "permuted_differences": differences,
    }


def adjust_probabilities(
    hypotheses: Sequence[Mapping[str, Any]], method: str
) -> dict[str, float]:
    """Adjust one explicit probability family without inferring membership."""

    parsed: list[tuple[str, float]] = []
    identities: set[str] = set()
    for index, hypothesis in enumerate(hypotheses):
        identity = hypothesis.get("id")
        if not isinstance(identity, str) or not identity.strip() or identity in identities:
            raise ReferenceOracleError("hypothesis identities must be unique non-empty strings")
        probability = _finite(hypothesis.get("probability"), f"hypotheses[{index}].probability")
        if not 0.0 <= probability <= 1.0:
            raise ReferenceOracleError("probabilities must be between zero and one")
        identities.add(identity)
        parsed.append((identity, probability))
    if not parsed:
        raise ReferenceOracleError("at least one hypothesis is required")

    count = len(parsed)
    adjusted: dict[str, float] = {}
    if method == "none":
        return dict(parsed)
    if method == "bonferroni":
        return {identity: min(1.0, probability * count) for identity, probability in parsed}
    if method == "sidak":
        return {
            identity: (
                1.0
                if probability >= 1.0
                else min(1.0, max(0.0, -math.expm1(count * math.log1p(-probability))))
            )
            for identity, probability in parsed
        }

    ordered = sorted(parsed, key=lambda item: (item[1], item[0]))
    if method == "holm":
        running = 0.0
        for rank, (identity, probability) in enumerate(ordered):
            running = min(1.0, max(running, probability * (count - rank)))
            adjusted[identity] = running
        return adjusted
    if method == "benjamini_hochberg":
        running = 1.0
        for reverse_rank in range(count - 1, -1, -1):
            identity, probability = ordered[reverse_rank]
            running = min(1.0, running, probability * count / (reverse_rank + 1))
            adjusted[identity] = running
        return adjusted
    raise ReferenceOracleError(f"unknown multiplicity method: {method}")


Polynomial = dict[tuple[int, ...], float]


def _powers_tuple(
    moderators: Sequence[str], powers: Mapping[str, Any], label: str
) -> tuple[int, ...]:
    unknown = set(powers).difference(moderators)
    if unknown:
        raise ReferenceOracleError(f"{label} names unknown moderators: {sorted(unknown)}")
    result: list[int] = []
    for moderator in moderators:
        exponent = powers.get(moderator, 0)
        if isinstance(exponent, bool) or not isinstance(exponent, int) or exponent < 0:
            raise ReferenceOracleError(f"{label}.{moderator} must be a non-negative integer")
        result.append(exponent)
    return tuple(result)


def compile_path_polynomial(
    moderators: Sequence[str], edges: Sequence[Mapping[str, Any]]
) -> Polynomial:
    """Multiply the explicitly selected edge coefficient polynomials."""

    if not moderators or len(set(moderators)) != len(moderators):
        raise ReferenceOracleError("moderators must be unique non-empty identities")
    if not edges:
        raise ReferenceOracleError("an explicit path must contain at least one edge")
    path: Polynomial = {(0,) * len(moderators): 1.0}
    for edge_index, edge in enumerate(edges):
        edge_terms = edge.get("terms")
        if not isinstance(edge_terms, list) or not edge_terms:
            raise ReferenceOracleError(f"edges[{edge_index}] has no terms")
        edge_polynomial: Polynomial = {}
        for term_index, term in enumerate(edge_terms):
            if not isinstance(term, Mapping):
                raise ReferenceOracleError("polynomial terms must be objects")
            coefficient = _finite(
                term.get("coefficient"),
                f"edges[{edge_index}].terms[{term_index}].coefficient",
            )
            powers = term.get("powers", {})
            if not isinstance(powers, Mapping):
                raise ReferenceOracleError("term powers must be an object")
            exponent = _powers_tuple(moderators, powers, "term powers")
            edge_polynomial[exponent] = edge_polynomial.get(exponent, 0.0) + coefficient

        multiplied: Polynomial = {}
        for left_exponent, left_coefficient in path.items():
            for right_exponent, right_coefficient in edge_polynomial.items():
                exponent = tuple(
                    left + right
                    for left, right in zip(left_exponent, right_exponent, strict=True)
                )
                multiplied[exponent] = (
                    multiplied.get(exponent, 0.0) + left_coefficient * right_coefficient
                )
        path = multiplied
    return path


def differentiate_polynomial(
    polynomial: Polynomial,
    moderators: Sequence[str],
    derivative_orders: Mapping[str, Any],
) -> Polynomial:
    orders = _powers_tuple(moderators, derivative_orders, "derivative orders")
    result: Polynomial = {}
    for exponent, coefficient in polynomial.items():
        if any(order > power for order, power in zip(orders, exponent, strict=True)):
            continue
        factor = coefficient
        remaining: list[int] = []
        for power, order in zip(exponent, orders, strict=True):
            for offset in range(order):
                factor *= power - offset
            remaining.append(power - order)
        remaining_tuple = tuple(remaining)
        result[remaining_tuple] = result.get(remaining_tuple, 0.0) + factor
    return result


def evaluate_polynomial(
    polynomial: Polynomial, moderators: Sequence[str], probe: Mapping[str, Any]
) -> float:
    if set(probe) != set(moderators):
        raise ReferenceOracleError("a probe must supply exactly every declared moderator")
    values = tuple(_finite(probe[moderator], f"probe.{moderator}") for moderator in moderators)
    return math.fsum(
        coefficient
        * math.prod(value**power for value, power in zip(values, exponent, strict=True))
        for exponent, coefficient in polynomial.items()
    )


def conditional_path_effect(
    polynomial: Polynomial, moderators: Sequence[str], probe: Mapping[str, Any]
) -> float:
    return evaluate_polynomial(polynomial, moderators, probe)


def conditional_path_derivative(
    polynomial: Polynomial,
    moderators: Sequence[str],
    probe: Mapping[str, Any],
    derivative_orders: Mapping[str, Any],
) -> float:
    derivative = differentiate_polynomial(polynomial, moderators, derivative_orders)
    return evaluate_polynomial(derivative, moderators, probe)


def conditional_probe_contrast(
    polynomial: Polynomial,
    moderators: Sequence[str],
    left_probe: Mapping[str, Any],
    right_probe: Mapping[str, Any],
) -> float:
    """Return the frozen left-minus-right finite probe contrast."""

    return conditional_path_effect(polynomial, moderators, left_probe) - conditional_path_effect(
        polynomial, moderators, right_probe
    )


def _evaluate_linear_equation(
    equation: Mapping[str, Any], context: Mapping[str, float]
) -> float:
    value = _finite(equation.get("intercept"), "equation.intercept")
    terms = equation.get("terms")
    if not isinstance(terms, list):
        raise ReferenceOracleError("equation terms must be a list")
    for index, term in enumerate(terms):
        coefficient = _finite(term.get("coefficient"), f"equation.terms[{index}].coefficient")
        factors = term.get("factors")
        if not isinstance(factors, list) or not factors:
            raise ReferenceOracleError("every linear-equation term needs one or more factors")
        try:
            product = math.prod(_finite(context[factor], f"context.{factor}") for factor in factors)
        except KeyError as error:
            raise ReferenceOracleError(f"missing equation factor: {error.args[0]}") from error
        value += coefficient * product
    if not math.isfinite(value):
        raise ReferenceOracleError("linear-equation prediction is non-finite")
    return value


def _g_computation_mean(
    specification: Mapping[str, Any],
    outcome_treatment_value: float,
    mediator_distribution_treatment_value: float,
) -> float:
    treatment_id = specification.get("treatment_id")
    if not isinstance(treatment_id, str) or not treatment_id:
        raise ReferenceOracleError("treatment_id is required")
    baseline_rows = specification.get("baseline_rows")
    mediator_equations = specification.get("mediator_equations")
    outcome_equation = specification.get("outcome_equation")
    if (
        not isinstance(baseline_rows, list)
        or not baseline_rows
        or not isinstance(mediator_equations, list)
        or not isinstance(outcome_equation, Mapping)
    ):
        raise ReferenceOracleError("g-computation equations and baseline rows are required")

    predictions: list[float] = []
    for row in baseline_rows:
        if not isinstance(row, Mapping):
            raise ReferenceOracleError("baseline rows must be objects")
        context = {key: _finite(value, f"baseline.{key}") for key, value in row.items()}
        context[treatment_id] = mediator_distribution_treatment_value
        for equation in mediator_equations:
            outcome_id = equation.get("outcome")
            if not isinstance(outcome_id, str) or not outcome_id:
                raise ReferenceOracleError("mediator equations need an outcome identity")
            context[outcome_id] = _evaluate_linear_equation(equation, context)
        context[treatment_id] = outcome_treatment_value
        predictions.append(_evaluate_linear_equation(outcome_equation, context))
    return math.fsum(predictions) / len(predictions)


def interventional_g_computation(specification: Mapping[str, Any]) -> dict[str, float]:
    """Compute an observed linear interventional direct/indirect decomposition."""

    contrast = specification.get("contrast")
    if not isinstance(contrast, Mapping):
        raise ReferenceOracleError("an explicit treatment contrast is required")
    x0 = _finite(contrast.get("x0"), "contrast.x0")
    x1 = _finite(contrast.get("x1"), "contrast.x1")
    if x0 == x1:
        raise ReferenceOracleError("x0 and x1 must differ")
    y00 = _g_computation_mean(specification, x0, x0)
    y10 = _g_computation_mean(specification, x1, x0)
    y11 = _g_computation_mean(specification, x1, x1)
    direct = y10 - y00
    indirect = y11 - y10
    total = y11 - y00
    return {
        "outcome_mean_x0_g_x0": y00,
        "outcome_mean_x1_g_x0": y10,
        "outcome_mean_x1_g_x1": y11,
        "interventional_direct_effect": direct,
        "joint_interventional_indirect_effect": indirect,
        "total_interventional_contrast": total,
        "decomposition_residual": total - direct - indirect,
    }


def _approximately_equal(left: float, right: float) -> bool:
    return math.isclose(left, right, rel_tol=1e-12, abs_tol=1e-12)


def assess_observed_treatment_positivity(
    treatment_values: Sequence[float],
    contrast: Mapping[str, Any],
    policy: Mapping[str, Any],
) -> dict[str, Any]:
    """Apply a necessary observed-support screen; this does not prove positivity."""

    values = [_finite(value, f"treatment[{index}]") for index, value in enumerate(treatment_values)]
    if not values:
        raise ReferenceOracleError("observed treatment values are required")
    kind = contrast.get("kind")
    x0 = _finite(contrast.get("x0"), "contrast.x0")
    x1 = _finite(contrast.get("x1"), "contrast.x1")
    blockers: list[str] = []
    if _approximately_equal(x0, x1):
        blockers.append("positivity.invalid_treatment_contrast")

    minimum = min(values)
    maximum = max(values)
    x0_count = 0
    x1_count = 0
    if kind == "binary":
        minimum_arm_count = int(policy.get("minimum_binary_arm_count", 1))
        maximum_arm_ratio = _finite(
            policy.get("maximum_binary_arm_ratio", 10.0), "maximum_binary_arm_ratio"
        )
        unexpected = 0
        for value in values:
            if _approximately_equal(value, x0):
                x0_count += 1
            elif _approximately_equal(value, x1):
                x1_count += 1
            else:
                unexpected += 1
        if unexpected:
            blockers.append("positivity.binary_unexpected_value")
        if x0_count < minimum_arm_count or x1_count < minimum_arm_count:
            blockers.append("positivity.binary_minimum_arm_count")
        arm_ratio = max(x0_count, x1_count) / max(1, min(x0_count, x1_count))
        if arm_ratio > maximum_arm_ratio:
            blockers.append("positivity.binary_arm_ratio")
    elif kind == "continuous_contrast":
        fraction = _finite(
            policy.get("continuous_neighborhood_fraction_of_range", 0.05),
            "continuous_neighborhood_fraction_of_range",
        )
        minimum_neighborhood_count = int(policy.get("minimum_continuous_neighborhood_count", 1))
        if minimum == maximum or x0 < minimum or x0 > maximum or x1 < minimum or x1 > maximum:
            blockers.append("positivity.continuous_outside_support")
        radius = abs(maximum - minimum) * fraction
        x0_count = sum(abs(value - x0) <= radius for value in values)
        x1_count = sum(abs(value - x1) <= radius for value in values)
        if x0_count < minimum_neighborhood_count or x1_count < minimum_neighborhood_count:
            blockers.append("positivity.continuous_neighborhood_count")
    else:
        raise ReferenceOracleError(f"unknown observed treatment kind: {kind}")

    return {
        "eligible": not blockers,
        "blocker_codes": blockers,
        "observed_minimum": minimum,
        "observed_maximum": maximum,
        "x0_support_count": x0_count,
        "x1_support_count": x1_count,
        "wording": "necessary observed-support screen; does not prove causal positivity",
    }


def log_sum_exp(values: Iterable[float]) -> float:
    parsed = [_finite(value, "log-sum-exp input") for value in values]
    if not parsed:
        raise ReferenceOracleError("log-sum-exp requires at least one value")
    maximum = max(parsed)
    return maximum + math.log(math.fsum(math.exp(value - maximum) for value in parsed))


def fimix_gaussian_identities(specification: Mapping[str, Any]) -> dict[str, Any]:
    """Evaluate a frozen Gaussian mixture-regression likelihood and criteria."""

    proportions = [_finite(value, "class proportion") for value in specification.get("proportions", [])]
    if len(proportions) < 2 or any(value <= 0.0 for value in proportions):
        raise ReferenceOracleError("FIMIX needs at least two positive class proportions")
    if not math.isclose(math.fsum(proportions), 1.0, rel_tol=1e-12, abs_tol=1e-12):
        raise ReferenceOracleError("class proportions must sum to one")
    equations = specification.get("equations")
    if not isinstance(equations, list) or not equations:
        raise ReferenceOracleError("at least one FIMIX equation is required")

    classes = len(proportions)
    observations: int | None = None
    per_class_parameter_count = 0
    parsed_equations: list[tuple[list[list[float]], list[float], list[list[float]], list[float]]] = []
    for equation_index, equation in enumerate(equations):
        design = equation.get("design")
        outcome = equation.get("outcome")
        coefficients = equation.get("class_coefficients")
        variances = equation.get("class_variances")
        if (
            not isinstance(design, list)
            or not design
            or not isinstance(outcome, list)
            or not isinstance(coefficients, list)
            or not isinstance(variances, list)
            or len(design) != len(outcome)
            or len(coefficients) != classes
            or len(variances) != classes
        ):
            raise ReferenceOracleError(f"equations[{equation_index}] dimensions are invalid")
        width = len(design[0])
        if width == 0 or any(not isinstance(row, list) or len(row) != width for row in design):
            raise ReferenceOracleError("FIMIX design rows must have one fixed positive width")
        if any(not isinstance(row, list) or len(row) != width for row in coefficients):
            raise ReferenceOracleError("each class coefficient vector must match the design width")
        if observations is None:
            observations = len(design)
        elif observations != len(design):
            raise ReferenceOracleError("all FIMIX equations must use the same rows")
        parsed_design = [
            [_finite(value, f"design[{row_index}]") for value in row]
            for row_index, row in enumerate(design)
        ]
        parsed_outcome = [
            _finite(value, f"outcome[{row_index}]") for row_index, value in enumerate(outcome)
        ]
        parsed_coefficients = [
            [_finite(value, f"coefficients[{class_index}]") for value in row]
            for class_index, row in enumerate(coefficients)
        ]
        parsed_variances = [_finite(value, "class variance") for value in variances]
        if any(value <= 0.0 for value in parsed_variances):
            raise ReferenceOracleError("class variances must be positive")
        parsed_equations.append(
            (parsed_design, parsed_outcome, parsed_coefficients, parsed_variances)
        )
        per_class_parameter_count += width + 1

    assert observations is not None
    log_joint: list[list[float]] = []
    log_normalizers: list[float] = []
    posteriors: list[list[float]] = []
    gaussian_constant = math.log(2.0 * math.pi)
    for row_index in range(observations):
        row_joint: list[float] = []
        for class_index, proportion in enumerate(proportions):
            components = [math.log(proportion)]
            for design, outcome, coefficients, variances in parsed_equations:
                prediction = math.fsum(
                    value * coefficient
                    for value, coefficient in zip(
                        design[row_index], coefficients[class_index], strict=True
                    )
                )
                residual = outcome[row_index] - prediction
                variance = variances[class_index]
                components.append(
                    -0.5 * (gaussian_constant + math.log(variance) + residual * residual / variance)
                )
            row_joint.append(math.fsum(components))
        normalizer = log_sum_exp(row_joint)
        posterior = [math.exp(value - normalizer) for value in row_joint]
        log_joint.append(row_joint)
        log_normalizers.append(normalizer)
        posteriors.append(posterior)

    log_likelihood = math.fsum(log_normalizers)
    parameter_count = classes * per_class_parameter_count + classes - 1
    deviance = -2.0 * log_likelihood
    log_n = math.log(observations)
    criteria = {
        "aic": deviance + 2.0 * parameter_count,
        "aic3": deviance + 3.0 * parameter_count,
        "aic4": deviance + 4.0 * parameter_count,
        "bic": deviance + parameter_count * log_n,
        "caic": deviance + parameter_count * (log_n + 1.0),
        "hq": deviance + 2.0 * parameter_count * math.log(log_n),
    }
    raw_entropy = -math.fsum(
        probability * math.log(probability)
        for row in posteriors
        for probability in row
        if probability > 0.0
    )
    normalized_certainty = 1.0 - raw_entropy / (observations * math.log(classes))
    return {
        "log_joint": log_joint,
        "log_normalizers": log_normalizers,
        "posteriors": posteriors,
        "log_likelihood": log_likelihood,
        "parameter_count": parameter_count,
        "criteria": criteria,
        "entropy": {
            "raw": raw_entropy,
            "normalized_certainty": normalized_certainty,
        },
    }
