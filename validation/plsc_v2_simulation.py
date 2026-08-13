"""Preregistered deterministic PLSc v2 recovery and reference simulations."""

from __future__ import annotations

import math
import random
from pathlib import Path
from typing import Any

import numpy as np

from plsc_v2_factory_common import (
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    repository_path,
    run_plsc,
    strict_load_json,
    write_csv,
    write_identity_report,
)
from plsc_v2_factory_evidence import _independent_expected, _max_delta


SOURCE = "validation/plsc_v2_simulation.py"
REFERENCE_TOLERANCE = 1e-6
RECOVERY_TOLERANCE = 0.10


SCENARIOS = (
    {
        "id": "strong_reliability_large_path",
        "seed": 20_260_901,
        "n": 600,
        "indicators": 3,
        "beta": 0.55,
        "measurement_noise": 0.12,
        "weighting": "path",
        "distribution": "gaussian",
    },
    {
        "id": "moderate_reliability_path",
        "seed": 20_260_902,
        "n": 360,
        "indicators": 4,
        "beta": 0.40,
        "measurement_noise": 0.32,
        "weighting": "path",
        "distribution": "gaussian",
    },
    {
        "id": "two_indicator_limited_information",
        "seed": 20_260_903,
        "n": 240,
        "indicators": 2,
        "beta": 0.32,
        "measurement_noise": 0.14,
        "weighting": "path",
        "distribution": "gaussian",
    },
    {
        "id": "factor_inner_weighting",
        "seed": 20_260_904,
        "n": 420,
        "indicators": 3,
        "beta": 0.48,
        "measurement_noise": 0.22,
        "weighting": "factor",
        "distribution": "gaussian",
    },
    {
        "id": "nongaussian_reflective",
        "seed": 20_260_905,
        "n": 480,
        "indicators": 3,
        "beta": 0.50,
        "measurement_noise": 0.18,
        "weighting": "path",
        "distribution": "laplace",
    },
)


def _latent_draw(rng: random.Random, distribution: str) -> float:
    if distribution == "gaussian":
        return rng.gauss(0.0, 1.0)
    if distribution == "laplace":
        return (rng.expovariate(1.0) - rng.expovariate(1.0)) / math.sqrt(2.0)
    raise ValueError(distribution)


def generated_scenario(scenario: dict[str, Any]) -> tuple[list[str], list[list[float]], float]:
    rng = random.Random(scenario["seed"])
    variables = [
        *[f"x{index + 1}" for index in range(scenario["indicators"])],
        *[f"y{index + 1}" for index in range(scenario["indicators"])],
    ]
    rows: list[list[float]] = []
    latent_x: list[float] = []
    latent_y: list[float] = []
    beta = float(scenario["beta"])
    residual_scale = math.sqrt(1.0 - beta**2)
    for _ in range(scenario["n"]):
        x = _latent_draw(rng, scenario["distribution"])
        error = _latent_draw(rng, scenario["distribution"])
        y = beta * x + residual_scale * error
        latent_x.append(x)
        latent_y.append(y)
        noise = float(scenario["measurement_noise"])
        x_rows = [
            (1.0 - 0.03 * index) * x + rng.gauss(0.0, noise * (1.0 + 0.08 * index))
            for index in range(scenario["indicators"])
        ]
        y_rows = [
            (0.98 - 0.025 * index) * y + rng.gauss(0.0, noise * (1.0 + 0.07 * index))
            for index in range(scenario["indicators"])
        ]
        rows.append([*x_rows, *y_rows])
    latent_sample_path = float(np.corrcoef(latent_x, latent_y)[0, 1])
    return variables, rows, latent_sample_path


def _run_scenario(scenario: dict[str, Any]) -> dict[str, Any]:
    variables, rows, latent_sample_path = generated_scenario(scenario)
    csv_path = WORK_ROOT / f"simulation_{scenario['id']}.csv"
    write_csv(csv_path, variables, rows)
    indicators = int(scenario["indicators"])
    run = run_plsc(
        name=f"factory_simulation_{scenario['id']}",
        csv_path=csv_path,
        constructs=[
            construct("x", [f"x{index + 1}" for index in range(indicators)]),
            construct("y", [f"y{index + 1}" for index in range(indicators)]),
        ],
        paths=[{"source": "x", "target": "y"}],
        weighting_scheme=scenario["weighting"],
    )
    recipe = strict_load_json(ROOT / run["recipe"])
    expected = _independent_expected(csv_path, recipe)
    plsc = run["plsc"]
    actual_rho = {row["construct"]: row["rho_a"] for row in plsc["reliabilities"]}
    actual_paths = {
        (row["source"], row["target"]): row["coefficient"]
        for row in plsc["corrected_paths"]
    }
    actual_loadings = {
        (row["construct"], row["indicator"]): row["loading"]
        for row in plsc["corrected_outer_loadings"]
    }
    actual_path = actual_paths[("x", "y")]
    metrics = {
        "rho_a_reference_delta": _max_delta(actual_rho, expected["rho_a"]),
        "corrected_path_reference_delta": _max_delta(actual_paths, expected["paths"]),
        "corrected_loading_reference_delta": _max_delta(actual_loadings, expected["loadings"]),
        "corrected_r_squared_reference_delta": _max_delta(
            plsc["corrected_r_squared"], expected["r_squared"]
        ),
        "latent_sample_path_recovery_error": abs(actual_path - latent_sample_path),
    }
    passed = (
        run["passed"]
        and all(
            metrics[key] <= REFERENCE_TOLERANCE
            for key in (
                "rho_a_reference_delta",
                "corrected_path_reference_delta",
                "corrected_loading_reference_delta",
                "corrected_r_squared_reference_delta",
            )
        )
        and metrics["latent_sample_path_recovery_error"] <= RECOVERY_TOLERANCE
    )
    return {
        "id": scenario["id"],
        "passed": passed,
        "design": scenario,
        "metrics": metrics,
        "latent_sample_path": latent_sample_path,
        "corrected_path": actual_path,
        "rho_a": actual_rho,
        "artifacts": {
            "dataset": repository_path(csv_path),
            "recipe": run["recipe"],
            "result": run["output"],
        },
        "execution": run["execution"],
    }


def _inadmissible_control() -> dict[str, Any]:
    source_recipe = strict_load_json(
        ROOT / "validation" / "fixtures" / "corporate_reputation.recipe.json"
    )
    run = run_plsc(
        name="factory_simulation_inadmissible_small_sample",
        csv_path=ROOT / "validation" / "fixtures" / "corporate_reputation.csv",
        constructs=source_recipe["model"]["constructs"],
        paths=source_recipe["model"]["paths"],
        expect_success=False,
    )
    text = (run["execution"]["stdout_tail"] + run["execution"]["stderr_tail"]).lower()
    diagnostics = [
        "corrected construct correlation is outside [-1, 1]",
        "invalid plsc rho_a",
    ]
    observed = [diagnostic for diagnostic in diagnostics if diagnostic in text]
    return {
        "id": "inadmissible_small_sample_control",
        "passed": run["passed"] and bool(observed),
        "expected_failure": True,
        "result_not_written": run["result_not_written"],
        "accepted_inadmissibility_diagnostics": diagnostics,
        "diagnostics_observed": observed,
        "recipe": run["recipe"],
        "execution": run["execution"],
    }


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    scenario_results = [_run_scenario(scenario) for scenario in SCENARIOS]
    inadmissible = _inadmissible_control()
    checks = {
        "passed": all(row["passed"] for row in scenario_results) and inadmissible["passed"],
        "preregistered_before_execution": True,
        "scenario_count": len(scenario_results),
        "reference_tolerance": REFERENCE_TOLERANCE,
        "latent_recovery_tolerance": RECOVERY_TOLERANCE,
        "dimensions_covered": {
            "sample_sizes": sorted({row["n"] for row in SCENARIOS}),
            "indicator_counts": sorted({row["indicators"] for row in SCENARIOS}),
            "weighting_schemes": sorted({row["weighting"] for row in SCENARIOS}),
            "distributions": sorted({row["distribution"] for row in SCENARIOS}),
            "two_indicator_case": True,
            "inadmissible_control": True,
        },
        "scenarios": scenario_results,
        "inadmissible_control": inadmissible,
        "inadmissible_cases": 1,
        "inadmissible_cases_rejected": 1 if inadmissible["passed"] else 0,
    }
    generated = [
        path
        for row in scenario_results
        for path in row["artifacts"].values()
    ] + [inadmissible["recipe"]]
    report = write_identity_report(
        "simulation_report",
        passed=checks["passed"],
        checks=checks,
        extras=[
            SOURCE,
            "validation/plsc_v2_factory_evidence.py",
            "validation/plsc_reference.py",
            "validation/higher_order_reference.py",
            "validation/fixtures/corporate_reputation.csv",
            "validation/fixtures/corporate_reputation.recipe.json",
            *generated,
        ],
    )
    print(f"wrote {report} | passed={checks['passed']} | scenarios={len(scenario_results)}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
