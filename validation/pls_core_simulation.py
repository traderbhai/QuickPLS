"""Fresh deterministic simulation matrix for the frozen PLS-PM v1 scope.

The NumPy reference below is intentionally self-contained and never imports
QuickPLS.  It follows the equations frozen in ``docs/methods/PLS_PM_V1.md``
and is used together with data-generating truth to distinguish implementation
agreement from ordinary finite-sample recovery error.
"""

from __future__ import annotations

import argparse
import math
from dataclasses import dataclass
from typing import Any, Sequence

import numpy as np

from pls_algorithm_v1_factory_common import (
    WORK_ROOT,
    run_pls,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/pls_core_simulation.py"
TOLERANCE = 1e-6


@dataclass(frozen=True)
class Scenario:
    name: str
    observations: int
    weighting_scheme: str
    modes: tuple[str, str, str]
    indicators: tuple[int, int, int]
    seed: int
    missing: bool = False
    non_gaussian: bool = False


def _sample_sd(values: np.ndarray) -> float:
    return float(np.std(values, ddof=1))


def _standardize(values: np.ndarray) -> np.ndarray:
    centered = values - np.mean(values)
    scale = _sample_sd(centered)
    if not math.isfinite(scale) or scale <= 1e-14:
        raise ValueError("cannot standardize a constant vector")
    return centered / scale


def _correlation(left: np.ndarray, right: np.ndarray) -> float:
    return float(np.dot(_standardize(left), _standardize(right)) / (len(left) - 1))


def _preprocess(matrix: np.ndarray, kind: str) -> np.ndarray:
    if kind == "unstandardized":
        return matrix.copy()
    centered = matrix - matrix.mean(axis=0)
    if kind == "mean_centered":
        return centered
    if kind != "standardized":
        raise ValueError(kind)
    scale = matrix.std(axis=0, ddof=1)
    if np.any(~np.isfinite(scale)) or np.any(scale <= 1e-14):
        raise ValueError("constant indicator")
    return centered / scale


def _oriented_unit_score_weight(block: np.ndarray, weight: np.ndarray) -> np.ndarray:
    score = block @ weight
    scale = _sample_sd(score)
    if not math.isfinite(scale) or scale <= 1e-14:
        raise ValueError("zero-variance construct proxy")
    weight = weight / scale
    for value in weight:
        if abs(float(value)) > 1e-14:
            if value < 0:
                weight = -weight
            break
    return weight


def _pca_weight(block: np.ndarray) -> np.ndarray:
    centered = block - block.mean(axis=0)
    covariance = centered.T @ centered / (len(block) - 1)
    values, vectors = np.linalg.eigh(covariance)
    weight = vectors[:, int(np.argmax(values))]
    score = block @ weight
    unit_reference = block @ np.ones(block.shape[1])
    association = float(
        np.dot(score - score.mean(), unit_reference - unit_reference.mean())
        / (len(block) - 1)
    )
    if association < -1e-15 or (abs(association) <= 1e-15 and weight.sum() < 0):
        weight = -weight
    return _oriented_unit_score_weight(block, weight)


def independent_pls(
    rows: Sequence[Sequence[float | None]],
    variables: Sequence[str],
    constructs: Sequence[dict[str, Any]],
    paths: Sequence[dict[str, str]],
    *,
    weighting_scheme: str,
    preprocessing: str = "standardized",
    tolerance: float = 1e-10,
    max_iterations: int = 10_000,
) -> dict[str, Any]:
    complete_indices = [
        index for index, row in enumerate(rows) if all(value is not None for value in row)
    ]
    matrix = np.asarray(
        [[float(value) for value in rows[index]] for index in complete_indices],
        dtype=float,
    )
    transformed = _preprocess(matrix, preprocessing)
    variable_index = {name: index for index, name in enumerate(variables)}
    construct_index = {row["id"]: index for index, row in enumerate(constructs)}
    blocks = [
        transformed[:, [variable_index[name] for name in construct["indicators"]]]
        for construct in constructs
    ]
    modes = [construct["mode"] for construct in constructs]
    predecessor: list[list[int]] = [[] for _ in constructs]
    successor: list[list[int]] = [[] for _ in constructs]
    for edge in paths:
        source = construct_index[edge["source"]]
        target = construct_index[edge["target"]]
        predecessor[target].append(source)
        successor[source].append(target)

    if weighting_scheme == "pca":
        weights = [_pca_weight(block) for block in blocks]
        iterations = 1
    else:
        weights = [
            _oriented_unit_score_weight(block, np.ones(block.shape[1], dtype=float))
            for block in blocks
        ]
        for iterations in range(1, max_iterations + 1):
            scores = np.column_stack(
                [_standardize(block @ weight) for block, weight in zip(blocks, weights)]
            )
            inner = np.zeros_like(scores)
            for target in range(len(constructs)):
                if weighting_scheme == "factor":
                    for adjacent in predecessor[target] + successor[target]:
                        inner[:, target] += (
                            _correlation(scores[:, target], scores[:, adjacent])
                            * scores[:, adjacent]
                        )
                elif weighting_scheme == "path":
                    if predecessor[target]:
                        design = scores[:, predecessor[target]]
                        coefficients = np.linalg.lstsq(
                            design, scores[:, target], rcond=None
                        )[0]
                        inner[:, target] += design @ coefficients
                    for adjacent in successor[target]:
                        inner[:, target] += (
                            _correlation(scores[:, target], scores[:, adjacent])
                            * scores[:, adjacent]
                        )
                else:
                    raise ValueError(weighting_scheme)
                inner[:, target] = _standardize(inner[:, target])

            updated: list[np.ndarray] = []
            for block, mode, proxy in zip(blocks, modes, inner.T):
                if mode == "reflective":
                    centered = block - block.mean(axis=0)
                    weight = centered.T @ proxy / (len(block) - 1)
                elif mode == "formative":
                    centered = block - block.mean(axis=0)
                    weight = np.linalg.lstsq(centered, proxy, rcond=None)[0]
                else:
                    raise ValueError(mode)
                updated.append(_oriented_unit_score_weight(block, weight))
            difference = max(
                float(np.max(np.abs(before - after)))
                for before, after in zip(weights, updated)
            )
            weights = updated
            if difference <= tolerance:
                break
        else:
            raise RuntimeError("independent PLS reference did not converge")

    scores = np.column_stack(
        [_standardize(block @ weight) for block, weight in zip(blocks, weights)]
    )
    path_rows: list[dict[str, Any]] = []
    r_squared: dict[str, float] = {}
    coefficient_matrix = np.zeros((len(constructs), len(constructs)))
    for target, sources in enumerate(predecessor):
        if not sources:
            continue
        coefficients = np.linalg.lstsq(scores[:, sources], scores[:, target], rcond=None)[0]
        fitted = scores[:, sources] @ coefficients
        residual = scores[:, target] - fitted
        r_squared[constructs[target]["id"]] = float(
            1.0 - np.dot(residual, residual) / np.dot(scores[:, target], scores[:, target])
        )
        for source, coefficient in zip(sources, coefficients):
            coefficient_matrix[target, source] = coefficient
            path_rows.append(
                {
                    "source": constructs[source]["id"],
                    "target": constructs[target]["id"],
                    "coefficient": float(coefficient),
                }
            )

    outer: list[dict[str, Any]] = []
    for construct, block, weight, score in zip(constructs, blocks, weights, scores.T):
        for position, indicator in enumerate(construct["indicators"]):
            outer.append(
                {
                    "construct": construct["id"],
                    "indicator": indicator,
                    "loading": _correlation(block[:, position], score),
                    "weight": float(weight[position]),
                }
            )
    return {
        "converged": True,
        "iterations": iterations,
        "used_observations": len(complete_indices),
        "omitted_observations": len(rows) - len(complete_indices),
        "complete_indices": complete_indices,
        "paths": path_rows,
        "outer_estimates": outer,
        "r_squared": r_squared,
        "construct_scores": {
            construct["id"]: [float(value) for value in scores[:, index]]
            for index, construct in enumerate(constructs)
        },
        "coefficient_matrix": coefficient_matrix,
    }


def _constructs(indicators: tuple[int, int, int], modes: tuple[str, str, str]) -> tuple[list[str], list[dict[str, Any]]]:
    variables: list[str] = []
    constructs: list[dict[str, Any]] = []
    for label, count, mode in zip(("x", "m", "y"), indicators, modes):
        names = [f"{label}{index + 1}" for index in range(count)]
        variables.extend(names)
        constructs.append(
            {
                "id": label,
                "name": label.upper(),
                "short_name": label.upper(),
                "mode": mode,
                "indicators": names,
            }
        )
    return variables, constructs


def generate_scenario(scenario: Scenario) -> dict[str, Any]:
    rng = np.random.default_rng(scenario.seed)
    n = scenario.observations
    if scenario.non_gaussian:
        noise = np.sign(rng.normal(size=(n, 10))) * np.abs(rng.normal(size=(n, 10))) ** 1.4
    else:
        noise = rng.normal(size=(n, 10))

    x_sources = rng.normal(size=(n, max(3, scenario.indicators[0])))
    if scenario.modes[0] == "formative":
        x = _standardize(
            x_sources[:, :3] @ np.asarray([0.55, 0.30, 0.15], dtype=float)
        )
    else:
        x = _standardize(noise[:, 0])
    m = _standardize(0.58 * x + math.sqrt(1.0 - 0.58**2) * noise[:, 1])
    y_linear = 0.26 * x + 0.55 * m + 0.62 * noise[:, 2]
    y = _standardize(y_linear)
    latent = {"x": x, "m": m, "y": y}

    variables, constructs = _constructs(scenario.indicators, scenario.modes)
    columns: list[np.ndarray] = []
    truth_loadings: dict[tuple[str, str], float] = {}
    noise_column = 3
    for construct in constructs:
        label = construct["id"]
        count = len(construct["indicators"])
        if construct["mode"] == "formative":
            if label != "x":
                raise ValueError("the frozen simulation matrix uses formative mode only for x")
            source = x_sources[:, :count]
            for indicator_index, indicator in enumerate(construct["indicators"]):
                column = source[:, indicator_index]
                columns.append(column)
                truth_loadings[(label, indicator)] = _correlation(column, latent[label])
        else:
            for indicator_index, indicator in enumerate(construct["indicators"]):
                if count == 1:
                    column = latent[label].copy()
                else:
                    loading = 0.90 - 0.04 * indicator_index
                    column = (
                        loading * latent[label]
                        + math.sqrt(1.0 - loading**2) * noise[:, noise_column % noise.shape[1]]
                    )
                    noise_column += 1
                columns.append(column)
                truth_loadings[(label, indicator)] = _correlation(column, latent[label])
    matrix = np.column_stack(columns)
    rows: list[list[float | None]] = matrix.tolist()
    if scenario.missing:
        for row_index in range(7, n, 37):
            rows[row_index][row_index % len(variables)] = None
    complete = np.asarray(
        [index for index, row in enumerate(rows) if all(value is not None for value in row)]
    )
    latent_complete = np.column_stack([x[complete], m[complete], y[complete]])
    truth_paths: dict[tuple[str, str], float] = {}
    truth_paths[("x", "m")] = float(
        np.linalg.lstsq(latent_complete[:, [0]], latent_complete[:, 1], rcond=None)[0][0]
    )
    coefficients = np.linalg.lstsq(latent_complete[:, [0, 1]], latent_complete[:, 2], rcond=None)[0]
    truth_paths[("x", "y")] = float(coefficients[0])
    truth_paths[("m", "y")] = float(coefficients[1])
    return {
        "variables": variables,
        "constructs": constructs,
        "paths": [
            {"source": "x", "target": "m"},
            {"source": "x", "target": "y"},
            {"source": "m", "target": "y"},
        ],
        "rows": rows,
        "truth_paths": truth_paths,
        "truth_loadings": truth_loadings,
    }


def compare_to_reference(
    quick: dict[str, Any],
    reference: dict[str, Any],
    truth_paths: dict[tuple[str, str], float],
    truth_loadings: dict[tuple[str, str], float],
) -> dict[str, Any]:
    estimation = quick["estimation"]
    quick_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in estimation["paths"]
    }
    reference_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in reference["paths"]
    }
    quick_outer = {
        (row["construct"], row["indicator"]): row
        for row in estimation["outer_estimates"]
    }
    reference_outer = {
        (row["construct"], row["indicator"]): row
        for row in reference["outer_estimates"]
    }
    path_parity = max(
        abs(quick_paths[key] - reference_paths[key]) for key in reference_paths
    )
    loading_parity = max(
        abs(quick_outer[key]["loading"] - reference_outer[key]["loading"])
        for key in reference_outer
    )
    weight_parity = max(
        abs(quick_outer[key]["weight"] - reference_outer[key]["weight"])
        for key in reference_outer
    )
    r_squared_parity = max(
        abs(estimation["r_squared"][key] - value)
        for key, value in reference["r_squared"].items()
    )
    score_parity = max(
        abs(observed - expected)
        for construct, expected_values in reference["construct_scores"].items()
        for observed, expected in zip(
            estimation["construct_scores"][construct], expected_values
        )
    )
    path_bias = max(
        abs(quick_paths[key] - truth) for key, truth in truth_paths.items()
    )
    reflective_truth = {
        key: value
        for key, value in truth_loadings.items()
        if key in quick_outer and value > 0.75
    }
    loading_bias = max(
        abs(quick_outer[key]["loading"] - truth)
        for key, truth in reflective_truth.items()
    )
    maxima = {
        "path_parity": path_parity,
        "loading_parity": loading_parity,
        "weight_error": weight_parity,
        "r_squared_parity": r_squared_parity,
        "score_parity": score_parity,
        "path_bias": path_bias,
        "loading_bias": loading_bias,
    }
    return {
        "passed": (
            quick["passed"]
            and estimation["converged"] is True
            and estimation["used_observations"] == reference["used_observations"]
            and estimation["omitted_observations"] == reference["omitted_observations"]
            and max(path_parity, loading_parity, weight_parity, r_squared_parity, score_parity)
            <= TOLERANCE
            and path_bias <= 0.14
            and loading_bias <= 0.15
        ),
        "max_abs_errors": maxima,
        "quickpls_iterations": estimation["iterations"],
        "reference_iterations": reference["iterations"],
        "used_observations": estimation["used_observations"],
        "omitted_observations": estimation["omitted_observations"],
        "output": quick["output"],
        "output_sha256": quick["output_sha256"],
    }


def simulation_scenarios() -> list[Scenario]:
    return [
        Scenario("path_reflective", 360, "path", ("reflective",) * 3, (3, 3, 3), 20_260_813),
        Scenario("factor_reflective", 420, "factor", ("reflective",) * 3, (2, 3, 2), 20_260_814),
        Scenario("pca_reflective", 380, "pca", ("reflective",) * 3, (3, 2, 3), 20_260_815),
        Scenario("mixed_mode_b", 520, "path", ("formative", "reflective", "reflective"), (3, 3, 3), 20_260_816),
        Scenario("single_item_blocks", 300, "path", ("reflective",) * 3, (1, 1, 1), 20_260_817),
        Scenario("listwise_missing", 460, "path", ("reflective",) * 3, (3, 3, 3), 20_260_818, missing=True),
        Scenario("nongaussian_factor", 640, "factor", ("reflective",) * 3, (3, 3, 3), 20_260_819, non_gaussian=True),
    ]


def run_simulation_matrix() -> dict[str, Any]:
    rows_out: list[dict[str, Any]] = []
    for scenario in simulation_scenarios():
        fixture = generate_scenario(scenario)
        csv_path = WORK_ROOT / f"simulation_{scenario.name}.csv"
        write_csv(csv_path, fixture["variables"], fixture["rows"])
        quick = run_pls(
            name=f"factory_simulation_{scenario.name}",
            csv_path=csv_path,
            constructs=fixture["constructs"],
            paths=fixture["paths"],
            weighting_scheme=scenario.weighting_scheme,
        )
        reference = independent_pls(
            fixture["rows"],
            fixture["variables"],
            fixture["constructs"],
            fixture["paths"],
            weighting_scheme=scenario.weighting_scheme,
        )
        comparison = compare_to_reference(
            quick,
            reference,
            fixture["truth_paths"],
            fixture["truth_loadings"],
        )
        rows_out.append(
            {
                "name": scenario.name,
                "observations": scenario.observations,
                "weighting_scheme": scenario.weighting_scheme,
                "modes": list(scenario.modes),
                "indicator_counts": list(scenario.indicators),
                "missing": scenario.missing,
                "non_gaussian": scenario.non_gaussian,
                **comparison,
            }
        )
    coverage = {
        "scenario_count": len(rows_out),
        "weighting_schemes": sorted({row.weighting_scheme for row in simulation_scenarios()}),
        "includes_mode_b": any("formative" in row.modes for row in simulation_scenarios()),
        "includes_single_item_blocks": any(1 in row.indicators for row in simulation_scenarios()),
        "includes_listwise_missingness": any(row.missing for row in simulation_scenarios()),
        "includes_nongaussian_data": any(row.non_gaussian for row in simulation_scenarios()),
    }
    return {
        "passed": all(row["passed"] for row in rows_out),
        "independent_parity_tolerance": TOLERANCE,
        "path_recovery_max_abs_bias": 0.14,
        "loading_recovery_max_abs_bias": 0.15,
        "coverage": coverage,
        "scenarios": rows_out,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--detail-only", action="store_true")
    args = parser.parse_args()
    detail = run_simulation_matrix()
    if not args.detail_only:
        path = write_identity_report(
            "simulation_report",
            passed=detail["passed"],
            checks=detail,
            extras=[
                SOURCE,
                "crates/qpls-core/src/contract.rs",
                "crates/qpls-estimation/src/pls.rs",
                "crates/qpls-runner/src/lib.rs",
                "crates/qpls-cli/src/main.rs",
            ],
        )
        print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
