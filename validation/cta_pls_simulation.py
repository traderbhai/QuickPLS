"""Fresh deterministic simulation matrix for bounded descriptive CTA-PLS v1."""

from __future__ import annotations

import itertools
import math
import random
from typing import Any, Sequence

from cta_pls_v1_factory_common import (
    WORK_ROOT,
    construct,
    run_cta_pls,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/cta_pls_simulation.py"
TOLERANCE = 1e-6
PAIRINGS = (
    "ab_cd_minus_ac_bd",
    "ac_bd_minus_ad_bc",
    "ad_bc_minus_ab_cd",
)


def _complete_rows(rows: Sequence[Sequence[float | None]]) -> list[list[float]]:
    return [
        [float(value) for value in row]
        for row in rows
        if all(value is not None for value in row)
    ]


def _preprocess(
    rows: Sequence[Sequence[float | None]], preprocessing: str
) -> list[list[float]]:
    matrix = _complete_rows(rows)
    if len(matrix) < 3:
        raise ValueError("CTA-PLS reference requires at least three complete rows")
    columns = list(zip(*matrix))
    means = [sum(column) / len(column) for column in columns]
    scales = []
    for index, column in enumerate(columns):
        centered = [value - means[index] for value in column]
        variance = sum(value * value for value in centered) / (len(column) - 1)
        if not math.isfinite(variance) or variance <= 0.0:
            raise ValueError(f"independent reference column {index} has zero variance")
        scales.append(math.sqrt(variance))
    if preprocessing == "standardized":
        return [
            [(value - means[index]) / scales[index] for index, value in enumerate(row)]
            for row in matrix
        ]
    if preprocessing == "mean_centered":
        return [
            [value - means[index] for index, value in enumerate(row)]
            for row in matrix
        ]
    if preprocessing == "none":
        return matrix
    raise ValueError(f"unsupported independent preprocessing: {preprocessing}")


def _covariance(left: Sequence[float], right: Sequence[float]) -> float:
    left_mean = sum(left) / len(left)
    right_mean = sum(right) / len(right)
    return sum(
        (a - left_mean) * (b - right_mean) for a, b in zip(left, right)
    ) / (len(left) - 1)


def independent_tetrads(
    rows: Sequence[Sequence[float | None]],
    variables: Sequence[str],
    blocks: dict[str, Sequence[str]],
    preprocessing: str,
) -> dict[str, Any]:
    matrix = _preprocess(rows, preprocessing)
    columns = {
        variable: [row[index] for row in matrix]
        for index, variable in enumerate(variables)
    }
    estimates: dict[tuple[str, str, str, str, str, str], float] = {}
    maxima: dict[str, float] = {}
    for construct_id, indicators in blocks.items():
        construct_max = 0.0
        for a, b, c, d in itertools.combinations(indicators, 4):
            cov_ab = _covariance(columns[a], columns[b])
            cov_ac = _covariance(columns[a], columns[c])
            cov_ad = _covariance(columns[a], columns[d])
            cov_bc = _covariance(columns[b], columns[c])
            cov_bd = _covariance(columns[b], columns[d])
            cov_cd = _covariance(columns[c], columns[d])
            values = (
                cov_ab * cov_cd - cov_ac * cov_bd,
                cov_ac * cov_bd - cov_ad * cov_bc,
                cov_ad * cov_bc - cov_ab * cov_cd,
            )
            for pairing, value in zip(PAIRINGS, values):
                key = (construct_id, a, b, c, d, pairing)
                if key in estimates:
                    raise ValueError(f"duplicate independent tetrad key: {key}")
                estimates[key] = value
                construct_max = max(construct_max, abs(value))
        maxima[construct_id] = construct_max
    return {
        "used_observations": len(matrix),
        "omitted_observations": len(rows) - len(matrix),
        "estimates": estimates,
        "maxima": maxima,
    }


def compare_run(run: dict[str, Any], reference: dict[str, Any]) -> dict[str, Any]:
    observed: dict[tuple[str, str, str, str, str, str], float] = {}
    absolute_errors: list[float] = []
    absolute_value_errors: list[float] = []
    for row in run["cta"]["estimates"]:
        key = (
            row["construct"],
            row["indicator_a"],
            row["indicator_b"],
            row["indicator_c"],
            row["indicator_d"],
            row["pairing"],
        )
        if key in observed:
            raise ValueError(f"duplicate QuickPLS tetrad key: {key}")
        observed[key] = float(row["tetrad"])
        absolute_value_errors.append(
            abs(float(row["absolute_tetrad"]) - abs(float(row["tetrad"])))
        )
    expected = reference["estimates"]
    for key in set(observed) & set(expected):
        absolute_errors.append(abs(observed[key] - expected[key]))
    maximum_errors = [
        abs(float(run["cta"]["max_absolute_tetrad_by_construct"].get(key, math.inf)) - value)
        for key, value in reference["maxima"].items()
    ]
    exact_membership = set(observed) == set(expected)
    max_tetrad_error = max(absolute_errors, default=math.inf)
    max_absolute_error = max(absolute_value_errors, default=math.inf)
    max_summary_error = max(maximum_errors, default=math.inf)
    count_passed = (
        run["estimation"]["used_observations"] == reference["used_observations"]
        and run["estimation"]["omitted_observations"] == reference["omitted_observations"]
    )
    return {
        "passed": (
            run["passed"]
            and exact_membership
            and bool(observed)
            and count_passed
            and max_tetrad_error <= TOLERANCE
            and max_absolute_error <= TOLERANCE
            and max_summary_error <= TOLERANCE
        ),
        "exact_pairing_membership": exact_membership,
        "expected_pairing_count": len(expected),
        "observed_pairing_count": len(observed),
        "complete_case_counts_match": count_passed,
        "expected_used_observations": reference["used_observations"],
        "observed_used_observations": run["estimation"]["used_observations"],
        "expected_omitted_observations": reference["omitted_observations"],
        "observed_omitted_observations": run["estimation"]["omitted_observations"],
        "max_abs_tetrad_error": max_tetrad_error,
        "max_abs_absolute_value_error": max_absolute_error,
        "max_abs_construct_summary_error": max_summary_error,
        "missing_from_quickpls": [list(key) for key in sorted(set(expected) - set(observed))],
        "unexpected_from_quickpls": [list(key) for key in sorted(set(observed) - set(expected))],
        "output": run["output"],
        "output_sha256": run["output_sha256"],
    }


def generated_rows(
    *,
    seed: int,
    sample_size: int,
    x_count: int,
    y_count: int,
    non_gaussian: bool = False,
    correlated_error: bool = False,
    missing: bool = False,
) -> tuple[list[str], list[list[float | None]]]:
    rng = random.Random(seed)
    variables = [*(f"x{index + 1}" for index in range(x_count)), *(f"y{index + 1}" for index in range(y_count))]
    rows: list[list[float | None]] = []
    for row_index in range(sample_size):
        latent_x = rng.gauss(0.0, 1.0)
        innovation = rng.gauss(0.0, 0.65)
        if non_gaussian:
            latent_x = math.copysign(abs(latent_x) ** 1.45, latent_x)
            innovation = math.copysign(abs(innovation) ** 1.35, innovation)
        latent_y = 0.58 * latent_x + innovation
        shared_error = rng.gauss(0.0, 0.45) if correlated_error else 0.0
        values: list[float | None] = []
        for indicator_index in range(x_count):
            loading = 0.92 - 0.055 * indicator_index
            residual = rng.gauss(0.0, 0.18 + 0.025 * indicator_index)
            if correlated_error and indicator_index in (0, 1):
                residual += shared_error
            values.append(loading * latent_x + residual)
        for indicator_index in range(y_count):
            loading = 0.90 - 0.06 * indicator_index
            values.append(loading * latent_y + rng.gauss(0.0, 0.20 + 0.02 * indicator_index))
        if missing and row_index % 13 == 4:
            values[(row_index // 13) % len(values)] = None
        rows.append(values)
    return variables, rows


def run_simulation_matrix() -> dict[str, Any]:
    scenarios = [
        {"name": "four_vanishing_small", "sample_size": 40, "x_count": 4, "y_count": 2, "preprocessing": "standardized", "non_gaussian": False, "correlated_error": False, "missing": False, "population_shape": "single_factor_vanishing"},
        {"name": "four_nonvanishing", "sample_size": 80, "x_count": 4, "y_count": 2, "preprocessing": "standardized", "non_gaussian": False, "correlated_error": True, "missing": False, "population_shape": "correlated_residual_nonvanishing"},
        {"name": "five_mean_centered", "sample_size": 120, "x_count": 5, "y_count": 2, "preprocessing": "mean_centered", "non_gaussian": False, "correlated_error": False, "missing": False, "population_shape": "single_factor_vanishing"},
        {"name": "six_nongaussian", "sample_size": 180, "x_count": 6, "y_count": 2, "preprocessing": "standardized", "non_gaussian": True, "correlated_error": False, "missing": False, "population_shape": "bounded_nongaussian_single_factor"},
        {"name": "five_listwise_missing", "sample_size": 100, "x_count": 5, "y_count": 2, "preprocessing": "standardized", "non_gaussian": False, "correlated_error": False, "missing": True, "population_shape": "single_factor_vanishing"},
        {"name": "two_eligible_blocks", "sample_size": 140, "x_count": 4, "y_count": 4, "preprocessing": "mean_centered", "non_gaussian": True, "correlated_error": True, "missing": False, "population_shape": "mixed_block_tetrads"},
    ]
    results: list[dict[str, Any]] = []
    for index, scenario in enumerate(scenarios):
        variables, rows = generated_rows(
            seed=20_260_813 + index,
            sample_size=scenario["sample_size"],
            x_count=scenario["x_count"],
            y_count=scenario["y_count"],
            non_gaussian=scenario["non_gaussian"],
            correlated_error=scenario["correlated_error"],
            missing=scenario["missing"],
        )
        x_indicators = variables[: scenario["x_count"]]
        y_indicators = variables[scenario["x_count"] :]
        blocks = {"x": x_indicators}
        if len(y_indicators) >= 4:
            blocks["y"] = y_indicators
        csv_path = WORK_ROOT / f"simulation_{scenario['name']}.csv"
        write_csv(csv_path, variables, rows)
        run = run_cta_pls(
            name=f"factory_simulation_{scenario['name']}",
            csv_path=csv_path,
            constructs=[construct("x", x_indicators), construct("y", y_indicators)],
            paths=[{"source": "x", "target": "y"}],
            preprocessing=scenario["preprocessing"],
        )
        reference = independent_tetrads(rows, variables, blocks, scenario["preprocessing"])
        comparison = compare_run(run, reference)
        results.append({**scenario, **comparison})
    coverage = {
        "sample_sizes": sorted({row["sample_size"] for row in scenarios}),
        "indicator_counts": sorted({row["x_count"] for row in scenarios}),
        "preprocessing_modes": sorted({row["preprocessing"] for row in scenarios}),
        "population_shapes": sorted({row["population_shape"] for row in scenarios}),
        "includes_missing_rows": any(row["missing"] for row in scenarios),
        "includes_non_gaussian_data": any(row["non_gaussian"] for row in scenarios),
        "includes_multiple_eligible_blocks": any(row["y_count"] >= 4 for row in scenarios),
    }
    return {
        "passed": len(results) == 6 and all(row["passed"] for row in results),
        "tolerance": TOLERANCE,
        "scenario_count": len(results),
        "coverage": coverage,
        "scenarios": results,
    }


def main() -> int:
    detail = run_simulation_matrix()
    report = write_identity_report(
        "simulation_report",
        passed=detail["passed"],
        checks=detail,
        extras=[
            SOURCE,
            "crates/qpls-core/src/contract.rs",
            "crates/qpls-core/src/validation.rs",
            "crates/qpls-estimation/src/pls.rs",
            "crates/qpls-runner/src/lib.rs",
            "crates/qpls-cli/src/main.rs",
        ],
    )
    print(f"wrote {report} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
