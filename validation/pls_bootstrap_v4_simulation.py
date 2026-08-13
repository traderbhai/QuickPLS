"""Independent simulation and frozen Monte Carlo replay for bootstrap v4."""

from __future__ import annotations

import json
import math
import random
import sys
from pathlib import Path
from typing import Any

from pls_bootstrap_v4_factory_common import (
    REPORT_ROOT,
    ROOT,
    WORK_ROOT,
    construct,
    finite_interval,
    parameter_row,
    repository_path,
    run_bootstrap,
    run_command,
    sha256_file,
    strict_load_json,
    write_csv,
    write_identity_report,
)


SOURCE = "validation/pls_bootstrap_v4_simulation.py"
AGGREGATOR = "validation/aggregate_studentized_qualification.py"
FROZEN_REPORT = "validation/results/monte_carlo_studentized_qualification.json"
SHARD_DIR = ROOT / "validation" / "results" / "studentized_qualification_shards"
EXPECTED_SCENARIOS = {
    "coverage_beta_0_35",
    "null_beta_0",
    "heavy_tail_coverage_beta_0_35",
    "heavy_tail_null_beta_0",
}


def _replay_frozen_study() -> tuple[dict[str, Any], list[str]]:
    destination = WORK_ROOT / "recomputed_studentized_qualification.json"
    completed, execution = run_command(
        [
            sys.executable,
            AGGREGATOR,
            "--input",
            repository_path(SHARD_DIR),
            "--output",
            repository_path(destination),
        ],
        timeout=180,
    )
    frozen = strict_load_json(ROOT / FROZEN_REPORT)
    replay = strict_load_json(destination) if completed.returncode == 0 else {}
    comparable_keys = (
        "schema_version",
        "harness_version",
        "engine_versions",
        "mode",
        "configuration",
        "dgp",
        "scenarios",
        "qualification",
    )
    exact_replay = all(frozen.get(key) == replay.get(key) for key in comparable_keys)
    scenarios = frozen.get("scenarios", [])
    scenario_names = {row.get("name") for row in scenarios}
    checks = frozen.get("qualification", {}).get("checks", [])
    shard_sources = sorted(
        repository_path(Path(path))
        for path in replay.get("aggregation", {}).get("sources", [])
    )
    study = {
        "passed": (
            completed.returncode == 0
            and exact_replay
            and frozen.get("engine_versions", {}).get("resampling")
            == "indexed_resampling_v4"
            and scenario_names == EXPECTED_SCENARIOS
            and len(scenarios) == 4
            and all(row.get("completed_simulations") == 1000 for row in scenarios)
            and all(row.get("failed_simulations") == 0 for row in scenarios)
            and frozen.get("qualification", {}).get("evaluated") is True
            and frozen.get("qualification", {}).get("passed") is True
            and len(checks) >= 13
            and all(row.get("passed") is True for row in checks)
            and len(shard_sources) == 40
        ),
        "frozen_report": FROZEN_REPORT,
        "frozen_sha256": sha256_file(ROOT / FROZEN_REPORT),
        "recomputed_report": repository_path(destination),
        "recomputed_sha256": sha256_file(destination) if destination.is_file() else None,
        "exact_scientific_payload_replay": exact_replay,
        "scenario_names": sorted(str(name) for name in scenario_names),
        "simulations_per_scenario": [row.get("completed_simulations") for row in scenarios],
        "qualification_checks": checks,
        "source_shard_count": len(shard_sources),
        "execution": execution,
    }
    return study, shard_sources


def _fixture(beta: float, seed: int, sample_size: int = 180) -> tuple[list[str], list[list[float]]]:
    rng = random.Random(seed)
    rows: list[list[float]] = []
    residual_scale = math.sqrt(max(0.0, 1.0 - beta * beta))
    for _ in range(sample_size):
        x = rng.gauss(0.0, 1.0)
        y = beta * x + residual_scale * rng.gauss(0.0, 1.0)
        rows.append(
            [
                x + 0.04 * rng.gauss(0.0, 1.0),
                x + 0.04 * rng.gauss(0.0, 1.0),
                x + 0.04 * rng.gauss(0.0, 1.0),
                y + 0.04 * rng.gauss(0.0, 1.0),
                y + 0.04 * rng.gauss(0.0, 1.0),
                y + 0.04 * rng.gauss(0.0, 1.0),
            ]
        )
    return ["x1", "x2", "x3", "y1", "y2", "y3"], rows


def _fresh_scenario(name: str, beta: float, seed: int) -> dict[str, Any]:
    variables, rows = _fixture(beta, seed)
    csv_path = WORK_ROOT / f"simulation_{name}.csv"
    write_csv(csv_path, variables, rows)
    run = run_bootstrap(
        name=f"factory_simulation_{name}",
        csv_path=csv_path,
        constructs=[construct("x", variables[:3]), construct("y", variables[3:])],
        paths=[{"source": "x", "target": "y"}],
        bootstrap_samples=399,
        seed=seed + 1000,
        workers=1,
    )
    percentile = parameter_row(run["bootstrap"], "percentile", "path", "x", "y")
    bca = parameter_row(run["bootstrap"], "bca", "path", "x", "y")
    point_error = abs(float(percentile["original"]) - beta)
    percentile_contains = percentile["lower"] <= beta <= percentile["upper"]
    bca_contains = bca["lower"] <= beta <= bca["upper"]
    passed = (
        run["passed"]
        and run["bootstrap"]["usable_replicates"] >= math.ceil(399 * 0.9)
        and not run["bootstrap"]["failed_replicates"]
        and finite_interval(percentile)
        and finite_interval(bca)
        and point_error <= 0.12
        and percentile_contains
        and bca_contains
    )
    return {
        "passed": passed,
        "name": name,
        "true_path": beta,
        "observed_path": percentile["original"],
        "absolute_recovery_error": point_error,
        "percentile_interval": [percentile["lower"], percentile["upper"]],
        "percentile_contains_truth": percentile_contains,
        "bca_interval": [bca["lower"], bca["upper"]],
        "bca_contains_truth": bca_contains,
        "requested_replicates": 399,
        "usable_replicates": run["bootstrap"]["usable_replicates"],
        "failed_replicates": len(run["bootstrap"]["failed_replicates"]),
        "dataset": repository_path(csv_path),
        "dataset_sha256": sha256_file(csv_path),
        "recipe": run["recipe"],
        "result": run["output"],
        "result_sha256": run["output_sha256"],
        "execution": run["execution"],
    }


def run_simulation_gate() -> tuple[dict[str, Any], list[str]]:
    frozen, shard_sources = _replay_frozen_study()
    fresh = [
        _fresh_scenario("normal_alternative", 0.35, 20_260_821),
        _fresh_scenario("normal_null", 0.0, 20_260_822),
    ]
    generated = [
        row[key]
        for row in fresh
        for key in ("dataset", "recipe", "result")
    ]
    checks = {
        "passed": frozen["passed"] and all(row["passed"] for row in fresh),
        "frozen_preregistered_study_recomputed": frozen,
        "fresh_current_engine_scenarios": fresh,
        "fresh_scenario_scope": (
            "Two source-bound current-engine recovery probes supplement, but do not replace, "
            "the 4,000 completed preregistered normal/heavy-tail simulations."
        ),
        "gui_runtime_claimed": False,
        "build_performed": False,
    }
    return checks, [*shard_sources, *generated, repository_path(WORK_ROOT / "recomputed_studentized_qualification.json")]


def main() -> int:
    REPORT_ROOT.mkdir(parents=True, exist_ok=True)
    checks, extras = run_simulation_gate()
    report = write_identity_report(
        "simulation_report",
        passed=checks["passed"],
        checks=checks,
        extras=[SOURCE, AGGREGATOR, FROZEN_REPORT, *extras, *[f"{source}" for source in ()]],
    )
    print(f"wrote {report} | passed={checks['passed']}")
    return 0 if checks["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
