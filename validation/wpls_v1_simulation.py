"""Fresh deterministic WPLS simulation matrix against the independent reference."""

from __future__ import annotations

import argparse
import math
import random
from dataclasses import dataclass
from typing import Any

from wpls_reference import estimate_reference, weighted_cov
from wpls_v1_factory_common import WORK_ROOT, construct, run_model, write_csv, write_identity_report


SOURCE = "validation/wpls_v1_simulation.py"
REFERENCE_SOURCE = "validation/wpls_reference.py"
TOLERANCE = 1e-6


@dataclass(frozen=True)
class Scenario:
    name: str
    observations: int
    seed: int
    weighting_scheme: str
    weight_pattern: str
    missing: bool = False
    non_gaussian: bool = False


def scenarios() -> list[Scenario]:
    return [
        Scenario("uniform_path", 180, 20_260_851, "path", "uniform"),
        Scenario("moderate_path", 240, 20_260_852, "path", "moderate"),
        Scenario("strong_factor", 280, 20_260_853, "factor", "strong"),
        Scenario("listwise_missing", 260, 20_260_854, "path", "moderate", missing=True),
        Scenario("nongaussian_factor", 320, 20_260_855, "factor", "strong", non_gaussian=True),
    ]


def _noise(rng: random.Random, non_gaussian: bool) -> float:
    if not non_gaussian:
        return rng.gauss(0.0, 1.0)
    value = rng.gauss(0.0, 1.0)
    return math.copysign(abs(value) ** 1.35, value)


def generate(scenario: Scenario) -> tuple[list[dict[str, float | None]], list[float], list[float]]:
    rng = random.Random(scenario.seed)
    rows: list[dict[str, float | None]] = []
    latent_x: list[float] = []
    latent_y: list[float] = []
    for index in range(scenario.observations):
        x = _noise(rng, scenario.non_gaussian)
        y = 0.66 * x + 0.58 * _noise(rng, scenario.non_gaussian)
        if scenario.weight_pattern == "uniform":
            weight = 3.0
        elif scenario.weight_pattern == "moderate":
            weight = (0.65, 1.0, 1.45, 2.1)[index % 4]
        elif scenario.weight_pattern == "strong":
            weight = 0.25 if index % 5 else 4.75
        else:
            raise ValueError(scenario.weight_pattern)
        row: dict[str, float | None] = {
            "x1": 0.94 * x + 0.16 * _noise(rng, scenario.non_gaussian),
            "x2": 0.84 * x + 0.22 * _noise(rng, scenario.non_gaussian),
            "y1": 0.92 * y + 0.17 * _noise(rng, scenario.non_gaussian),
            "y2": 0.80 * y + 0.24 * _noise(rng, scenario.non_gaussian),
            "case_wt": weight,
        }
        if scenario.missing and index % 31 == 7:
            row["x2"] = None
        rows.append(row)
        latent_x.append(x)
        latent_y.append(y)
    return rows, latent_x, latent_y


def _retained(rows: list[dict[str, float | None]]) -> list[dict[str, float]]:
    names = ("x1", "x2", "y1", "y2", "case_wt")
    return [
        {name: float(row[name]) for name in names}
        for row in rows
        if all(row[name] is not None for name in names)
    ]


def _maximum(values: list[float]) -> float:
    return max(values) if values else 0.0


def compare(run: dict[str, Any], expected: dict[str, Any]) -> dict[str, Any]:
    estimation = run["estimation"]
    observed_paths = {(row["source"], row["target"]): row["coefficient"] for row in estimation["paths"]}
    observed_outer = {
        (row["construct"], row["indicator"]): row for row in estimation["outer_estimates"]
    }
    path_error = _maximum(
        [abs(observed_paths[key] - value) for key, value in expected["paths"].items()]
    )
    loading_error = _maximum(
        [abs(observed_outer[key]["loading"] - value) for key, value in expected["loadings"].items()]
    )
    outer_weight_error = _maximum(
        [abs(observed_outer[key]["weight"] - value) for key, value in expected["weights"].items()]
    )
    weighted_r2_error = _maximum(
        [abs(estimation["r_squared"][key] - value) for key, value in expected["r_squared"].items()]
    )
    wpls = run["wpls"]
    metadata_errors = {
        "weight_sum": abs(wpls["weight_sum"] - expected["weight_sum"]),
        "effective_sample_size": abs(
            wpls["effective_sample_size"] - expected["effective_sample_size"]
        ),
    }
    max_error = max(
        path_error,
        loading_error,
        outer_weight_error,
        weighted_r2_error,
        *metadata_errors.values(),
    )
    return {
        "passed": run["passed"] and max_error <= TOLERANCE,
        "max_abs_error": max_error,
        "path_error": path_error,
        "loading_error": loading_error,
        "outer_weight_error": outer_weight_error,
        "weighted_r2_error": weighted_r2_error,
        "metadata_errors": metadata_errors,
        "quickpls_iterations": estimation["iterations"],
        "reference_iterations": expected["iterations"],
    }


def run_matrix() -> dict[str, Any]:
    results: list[dict[str, Any]] = []
    for scenario in scenarios():
        rows, latent_x, latent_y = generate(scenario)
        retained = _retained(rows)
        csv_path = WORK_ROOT / f"simulation_{scenario.name}.csv"
        write_csv(
            csv_path,
            ["x1", "x2", "y1", "y2", "case_wt"],
            [[row[name] for name in ("x1", "x2", "y1", "y2", "case_wt")] for row in rows],
        )
        run = run_model(
            name=f"factory_simulation_{scenario.name}",
            csv_path=csv_path,
            constructs=[construct("x", ["x1", "x2"]), construct("y", ["y1", "y2"])],
            paths=[{"source": "x", "target": "y"}],
            weighting_scheme=scenario.weighting_scheme,
        )
        reference = estimate_reference(retained)
        parity = compare(run, reference)
        retained_indices = [
            index
            for index, row in enumerate(rows)
            if all(row[name] is not None for name in ("x1", "x2", "y1", "y2", "case_wt"))
        ]
        weights = [float(rows[index]["case_wt"]) for index in retained_indices]
        truth_path = weighted_cov(
            [latent_x[index] for index in retained_indices],
            [latent_y[index] for index in retained_indices],
            weights,
        ) / weighted_cov(
            [latent_x[index] for index in retained_indices],
            [latent_x[index] for index in retained_indices],
            weights,
        )
        observed_path = run["estimation"]["paths"][0]["coefficient"]
        recovery_error = abs(observed_path - truth_path)
        results.append(
            {
                "name": scenario.name,
                "observations": scenario.observations,
                "retained_observations": len(retained),
                "weighting_scheme": scenario.weighting_scheme,
                "weight_pattern": scenario.weight_pattern,
                "missing": scenario.missing,
                "non_gaussian": scenario.non_gaussian,
                "latent_path_recovery_error": recovery_error,
                "effective_sample_size": run["wpls"]["effective_sample_size"],
                "output": run["output"],
                "output_sha256": run["output_sha256"],
                **parity,
                "passed": parity["passed"] and recovery_error <= 0.16,
            }
        )
    coverage = {
        "scenario_count": len(results),
        "weight_patterns": sorted({row["weight_pattern"] for row in results}),
        "weighting_schemes": sorted({row["weighting_scheme"] for row in results}),
        "includes_listwise_missingness": any(row["missing"] for row in results),
        "includes_nongaussian_data": any(row["non_gaussian"] for row in results),
        "includes_uniform_reduction_fixture": any(row["weight_pattern"] == "uniform" for row in results),
    }
    return {
        "passed": all(row["passed"] for row in results),
        "independent_reference": REFERENCE_SOURCE,
        "reference_tolerance": TOLERANCE,
        "latent_path_recovery_max_abs_error": 0.16,
        "coverage": coverage,
        "scenarios": results,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--detail-only", action="store_true")
    args = parser.parse_args()
    detail = run_matrix()
    if not args.detail_only:
        path = write_identity_report(
            "simulation_report",
            passed=detail["passed"],
            checks=detail,
            extras=[SOURCE, REFERENCE_SOURCE, "crates/qpls-estimation/src/pls.rs"],
        )
        print(f"wrote {path} | passed={detail['passed']}")
    return 0 if detail["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
