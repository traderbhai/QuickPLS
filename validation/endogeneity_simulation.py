"""Deterministic null/signal simulation for bounded endogeneity diagnostics."""

from __future__ import annotations

import argparse
import math
import random
from typing import Any

import numpy as np
from scipy.stats import t as student_t

from endogeneity_factory_common import (
    METHOD_VERSION,
    WORK_ROOT,
    optionally_write_identity_report,
    run_endogeneity,
)
from endogeneity_reference import (
    estimate_pls,
    rankit_inverse_normal,
    regression_stats,
    skewness,
)


SOURCE = "validation/endogeneity_simulation.py"
TOLERANCE = 1e-6


def generated_rows(seed: int, n: int, copula_signal: float) -> list[dict[str, float]]:
    """Generate a skewed regressor whose rankit identifies a frozen signal.

    ``copula_signal=0`` is the exogenous null.  Positive signal adds the latent
    Gaussian driver of the lognormal predictor to the outcome disturbance.
    """

    rng = random.Random(seed)
    rows: list[dict[str, float]] = []
    for _ in range(n):
        latent_rank = rng.gauss(0.0, 1.0)
        x = math.exp(0.8 * latent_rank) - math.exp(0.32)
        z = 0.15 * x + rng.gauss(0.0, 1.0)
        y = (
            0.35 * x
            + 0.25 * z
            + copula_signal * latent_rank
            + rng.gauss(0.0, 0.65)
        )
        row = {
            "x1": x + rng.gauss(0.0, 0.05),
            "x2": 0.95 * x + rng.gauss(0.0, 0.07),
            "z1": z + rng.gauss(0.0, 0.08),
            "z2": 0.92 * z + rng.gauss(0.0, 0.09),
            "y1": y + rng.gauss(0.0, 0.06),
            "y2": 0.96 * y + rng.gauss(0.0, 0.07),
        }
        # The CLI consumes the twelve-decimal CSV written by the factory.
        rows.append({key: float(f"{value:.12f}") for key, value in row.items()})
    return rows


def independent_diagnostics(
    rows: list[dict[str, float]], recipe: dict[str, Any]
) -> dict[tuple[str, str], dict[str, Any]]:
    columns = {
        name: np.asarray([row[name] for row in rows], dtype=float) for name in rows[0]
    }
    reference = estimate_pls(columns, recipe)
    scores = {
        construct_id: np.asarray(values, dtype=float)
        for construct_id, values in reference["scores"].items()
    }
    expected: dict[tuple[str, str], dict[str, Any]] = {}
    for target in ("x", "z", "y"):
        predecessors = [
            path["source"]
            for path in recipe["model"]["paths"]
            if path["target"] == target
        ]
        if not predecessors:
            continue
        copulas = [rankit_inverse_normal(scores[source]) for source in predecessors]
        beta, standard_errors, t_statistics = regression_stats(
            [scores[source] for source in predecessors] + copulas,
            scores[target],
        )
        residual_degrees_of_freedom = len(rows) - 2 * len(predecessors) - 1
        for offset, source in enumerate(predecessors):
            index = len(predecessors) + offset
            source_skewness = skewness(scores[source])
            t_value = float(t_statistics[index])
            expected[(source, target)] = {
                "copula_coefficient": float(beta[index]),
                "standard_error": float(standard_errors[index]),
                "t_statistic": t_value,
                "p_value_two_sided": float(
                    2.0 * student_t.sf(abs(t_value), residual_degrees_of_freedom)
                ),
                "predictor_skewness": source_skewness,
                "applicable": abs(source_skewness) >= 0.5,
            }
    return expected


def compare_scenario(
    result: dict[str, Any], expected: dict[tuple[str, str], dict[str, Any]]
) -> dict[str, Any]:
    observed = {
        (row["source"], row["target"]): row for row in result["analysis"]["estimates"]
    }
    keys_match = set(observed) == set(expected)
    numeric_fields = (
        "copula_coefficient",
        "standard_error",
        "t_statistic",
        "p_value_two_sided",
        "predictor_skewness",
    )
    maxima = {
        field: max(
            (
                abs(float(observed[key][field]) - float(expected[key][field]))
                for key in expected
                if key in observed
            ),
            default=math.inf,
        )
        for field in numeric_fields
    }
    applicability_equal = keys_match and all(
        bool(observed[key]["applicable"]) == bool(expected[key]["applicable"])
        for key in expected
    )
    passed = (
        result["passed"]
        and keys_match
        and applicability_equal
        and all(value <= TOLERANCE for value in maxima.values())
    )
    x_row = observed.get(("x", "y"), {})
    return {
        "passed": passed,
        "keys_match": keys_match,
        "applicability_equal": applicability_equal,
        "max_abs_errors": maxima,
        "x_copula_p_value": x_row.get("p_value_two_sided"),
        "x_applicability": x_row.get("applicable"),
        "output": result.get("output"),
        "output_sha256": result.get("output_sha256"),
    }


def run_simulation() -> dict[str, Any]:
    scenarios: list[dict[str, Any]] = []
    for n in (120, 240):
        for copula_signal in (0.0, 0.4):
            for replicate in range(4):
                seed = 2026081400 + n * 10 + int(copula_signal * 100) + replicate
                name = f"simulation_n{n}_g{int(copula_signal * 10)}_r{replicate}"
                rows = generated_rows(seed, n, copula_signal)
                result = run_endogeneity(name=name, rows=rows)
                expected = independent_diagnostics(rows, result["recipe_document"])
                comparison = compare_scenario(result, expected)
                scenarios.append(
                    {
                        "name": name,
                        "sample_size": n,
                        "copula_signal": copula_signal,
                        "seed": seed,
                        **comparison,
                    }
                )
    null_rows = [row for row in scenarios if row["copula_signal"] == 0.0]
    signal_rows = [row for row in scenarios if row["copula_signal"] > 0.0]
    null_flags = sum(float(row["x_copula_p_value"]) < 0.05 for row in null_rows)
    signal_flags = sum(float(row["x_copula_p_value"]) < 0.05 for row in signal_rows)
    null_flag_rate = null_flags / len(null_rows)
    signal_detection_rate = signal_flags / len(signal_rows)
    behavioral_rules = {
        "null_flag_rate_at_most_0_25": null_flag_rate <= 0.25,
        "signal_detection_rate_at_least_0_75": signal_detection_rate >= 0.75,
        "skew_applicability_present_for_x": all(
            row["x_applicability"] is True for row in scenarios
        ),
    }
    return {
        "passed": all(row["passed"] for row in scenarios)
        and all(behavioral_rules.values()),
        "method_version": METHOD_VERSION,
        "tolerance": TOLERANCE,
        "scenario_count": len(scenarios),
        "coverage": {
            "sample_sizes": [120, 240],
            "copula_signals": [0.0, 0.4],
            "replicates_per_cell": 4,
        },
        "independent_numeric_agreement": all(row["passed"] for row in scenarios),
        "null_flag_rate": null_flag_rate,
        "signal_detection_rate": signal_detection_rate,
        "behavioral_rules": behavioral_rules,
        "scenarios": scenarios,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write-identity", action="store_true")
    args = parser.parse_args()
    detail = run_simulation()
    report = optionally_write_identity_report(
        "simulation_report",
        write_identity=args.write_identity,
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            "validation/endogeneity_reference.py",
            "validation/higher_order_reference.py",
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
    )
    print(
        "endogeneity simulation "
        f"passed={detail['passed']} scenarios={detail['scenario_count']} "
        f"null_rate={detail['null_flag_rate']:.3f} "
        f"signal_rate={detail['signal_detection_rate']:.3f} "
        f"identity={report or 'not-written'}"
    )
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
