"""Preregistered coverage, null, and failure simulations for CB-SEM bootstrap v2.

Qualification mode is intentionally expensive: it executes independent data
sets and a minimum of 1,000 full ML bootstrap refits per data set. A smoke
profile is available only for wiring checks and can never pass qualification.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import random
import subprocess
import tempfile
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
CLI = ROOT / "target" / "debug" / "qpls.exe"
OUTPUT = ROOT / "validation" / "results" / "cbsem_bootstrap_v2_release_simulation.json"
DESIGN_VERSION = "cbsem_bootstrap_v2_preregistered_simulation_v1"
MASTER_SEED = 2026081425
BOOTSTRAP_REPLICATES = 1_000
QUALIFICATION_DATASETS_PER_SCENARIO = 200
MINIMUM_USABLE = 1_000


@dataclass(frozen=True)
class Scenario:
    id: str
    purpose: str
    sample_size: int
    loading: float
    path: float
    distribution: str
    expected_failure: bool = False


SCENARIOS = (
    Scenario("coverage_normal_n150", "coverage", 150, 0.80, 0.35, "normal"),
    Scenario("coverage_normal_n300", "coverage", 300, 0.70, 0.20, "normal"),
    Scenario("coverage_heavy_tail_n300", "coverage", 300, 0.80, 0.35, "t5"),
    Scenario("null_normal_n150", "null", 150, 0.80, 0.00, "normal"),
    Scenario("null_heavy_tail_n300", "null", 300, 0.75, 0.00, "t5"),
    Scenario(
        "failure_singular_duplicate_indicators",
        "failure",
        120,
        1.00,
        0.35,
        "singular",
        True,
    ),
)

PREREGISTERED_LIMITS = {
    "coverage_lower": 0.88,
    "coverage_upper": 0.99,
    "null_rejection_upper": 0.10,
    "relative_se_bias_upper": 0.20,
    "successful_run_usable_rate_lower": 1.00,
    "failure_scenario_rejection_rate_lower": 0.95,
    "worker_payload_mismatch_count": 0,
}


def qpls(args: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    if not CLI.is_file():
        raise FileNotFoundError("build the coordinated source-frozen debug qpls.exe first")
    return subprocess.run(
        [str(CLI), *args],
        cwd=ROOT,
        check=check,
        capture_output=True,
        text=True,
    )


def innovation(rng: random.Random, distribution: str) -> float:
    if distribution == "normal":
        return rng.gauss(0.0, 1.0)
    if distribution == "t5":
        return rng.gauss(0.0, 1.0) / math.sqrt(
            sum(rng.gauss(0.0, 1.0) ** 2 for _ in range(5)) / 5.0
        )
    return rng.gauss(0.0, 1.0)


def write_data(path: Path, scenario: Scenario, data_index: int) -> None:
    rng = random.Random(MASTER_SEED + 10_000 * SCENARIOS.index(scenario) + data_index)
    columns = ["x1", "x2", "x3", "y1", "y2", "y3"]
    residual_scale = math.sqrt(max(1.0 - scenario.loading**2, 1e-8))
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=columns)
        writer.writeheader()
        for _ in range(scenario.sample_size):
            x = innovation(rng, scenario.distribution)
            y = scenario.path * x + math.sqrt(max(1.0 - scenario.path**2, 1e-8)) * innovation(
                rng, scenario.distribution
            )
            if scenario.distribution == "singular":
                x1 = x2 = x3 = x
            else:
                x1 = scenario.loading * x + residual_scale * innovation(rng, scenario.distribution)
                x2 = scenario.loading * x + residual_scale * innovation(rng, scenario.distribution)
                x3 = scenario.loading * x + residual_scale * innovation(rng, scenario.distribution)
            row = {
                "x1": x1,
                "x2": x2,
                "x3": x3,
                "y1": scenario.loading * y + residual_scale * innovation(rng, scenario.distribution),
                "y2": scenario.loading * y + residual_scale * innovation(rng, scenario.distribution),
                "y3": scenario.loading * y + residual_scale * innovation(rng, scenario.distribution),
            }
            writer.writerow({key: f"{value:.12g}" for key, value in row.items()})


def recipe(fingerprint: str, workers: int, seed: int) -> dict[str, Any]:
    return {
        "schema_version": 3,
        "id": "00000000-0000-0000-0000-000000252601",
        "created_at": "2026-08-14T00:00:00Z",
        "dataset_fingerprint": fingerprint,
        "model": {
            "id": "00000000-0000-0000-0000-000000252602",
            "name": "CB-SEM bootstrap v2 simulation",
            "constructs": [
                {
                    "id": "x",
                    "name": "X",
                    "short_name": "X",
                    "mode": "reflective",
                    "indicators": ["x1", "x2", "x3"],
                },
                {
                    "id": "y",
                    "name": "Y",
                    "short_name": "Y",
                    "mode": "reflective",
                    "indicators": ["y1", "y2", "y3"],
                },
            ],
            "paths": [{"source": "x", "target": "y"}],
            "controls": [],
            "higher_order_constructs": [],
            "interactions": [],
        },
        "settings": {
            "method": "cbsem",
            "weighting_scheme": "path",
            "tolerance": 1e-7,
            "max_iterations": 3_000,
            "bootstrap_samples": 0,
            "studentized_inner_samples": 0,
            "permutation_samples": 0,
            "seed": seed,
            "workers": workers,
            "confidence_level": 0.95,
            "preprocessing": "standardized",
            "missing_data": "listwise_deletion",
            "case_weight_column": None,
        },
        "method_config": {
            "kind": "cbsem",
            "model_type": "sem",
            "estimator": "ml",
            "input": "raw",
            "mean_structure": False,
            "bootstrap_samples": BOOTSTRAP_REPLICATES,
        },
        "metadata": {"status": DESIGN_VERSION},
    }


def analytical_payload(result: dict[str, Any]) -> dict[str, Any]:
    return result["payload"]["estimation"]["cbsem"]["bootstrap_v2"]


def run_dataset(
    work: Path, scenario: Scenario, data_index: int, *, worker_check: bool
) -> dict[str, Any]:
    data = work / f"{scenario.id}-{data_index}.csv"
    project = work / f"{scenario.id}-{data_index}.qpls"
    write_data(data, scenario, data_index)
    qpls(["import", str(data), str(project), "--name", scenario.id])
    inspected = json.loads(qpls(["inspect", str(project), "--json"]).stdout)
    fingerprint = inspected["datasets"][0]["fingerprint"]
    seed = MASTER_SEED + 1_000_000 + 10_000 * SCENARIOS.index(scenario) + data_index

    payloads: dict[int, dict[str, Any]] = {}
    errors: dict[int, str] = {}
    worker_values = (1, 4) if worker_check else (4,)
    for workers in worker_values:
        recipe_path = work / f"{scenario.id}-{data_index}-w{workers}.recipe.json"
        result_path = work / f"{scenario.id}-{data_index}-w{workers}.result.json"
        recipe_path.write_text(
            json.dumps(recipe(fingerprint, workers, seed), indent=2) + "\n",
            encoding="utf-8",
        )
        run = qpls(
            [
                "run",
                str(recipe_path),
                "--data",
                str(data),
                "--output",
                str(result_path),
                "--allow-experimental",
            ],
            check=False,
        )
        if run.returncode != 0:
            errors[workers] = (run.stderr or run.stdout)[-2_000:]
        else:
            payloads[workers] = analytical_payload(json.loads(result_path.read_text(encoding="utf-8")))

    primary = payloads.get(4) or payloads.get(1)
    if primary is None:
        return {
            "scenario": scenario.id,
            "data_index": data_index,
            "completed": False,
            "errors": errors,
            "worker_equal": None,
        }
    path_row = next(row for row in primary["intervals"] if row["parameter"] == "y~x")
    covered = float(path_row["percentile_lower"]) <= scenario.path <= float(
        path_row["percentile_upper"]
    )
    rejected_null = not (
        float(path_row["percentile_lower"]) <= 0.0 <= float(path_row["percentile_upper"])
    )
    return {
        "scenario": scenario.id,
        "data_index": data_index,
        "completed": True,
        "usable_replicates": primary["usable_replicates"],
        "failed_replicates": primary["failed_replicates"],
        "covered": covered,
        "rejected_null": rejected_null,
        "path_original": path_row["original"],
        "path_bootstrap_se": path_row["standard_error"],
        "worker_equal": payloads.get(1) == payloads.get(4) if worker_check else None,
    }


def summarize(records: list[dict[str, Any]], profile: str) -> dict[str, Any]:
    scenarios = []
    for scenario in SCENARIOS:
        rows = [row for row in records if row["scenario"] == scenario.id]
        completed = [row for row in rows if row["completed"]]
        rejected = len(rows) - len(completed)
        coverage = (
            sum(bool(row["covered"]) for row in completed) / len(completed)
            if completed
            else None
        )
        null_rejection = (
            sum(bool(row["rejected_null"]) for row in completed) / len(completed)
            if completed
            else None
        )
        usable_rate = (
            sum(int(row["usable_replicates"]) >= MINIMUM_USABLE for row in completed)
            / len(completed)
            if completed
            else None
        )
        if len(completed) >= 2:
            originals = [float(row["path_original"]) for row in completed]
            original_mean = sum(originals) / len(originals)
            monte_carlo_se = math.sqrt(
                sum((value - original_mean) ** 2 for value in originals)
                / (len(originals) - 1)
            )
            mean_bootstrap_se = sum(
                float(row["path_bootstrap_se"]) for row in completed
            ) / len(completed)
            relative_se_bias = (
                abs(mean_bootstrap_se / monte_carlo_se - 1.0)
                if monte_carlo_se > 0.0
                else None
            )
        else:
            monte_carlo_se = None
            mean_bootstrap_se = None
            relative_se_bias = None
        scenarios.append(
            {
                **asdict(scenario),
                "datasets": len(rows),
                "completed": len(completed),
                "rejected": rejected,
                "coverage": coverage,
                "null_rejection": null_rejection,
                "successful_run_usable_rate": usable_rate,
                "monte_carlo_path_se": monte_carlo_se,
                "mean_bootstrap_path_se": mean_bootstrap_se,
                "relative_se_bias": relative_se_bias,
            }
        )
    coverage_pass = all(
        row["coverage"] is not None
        and PREREGISTERED_LIMITS["coverage_lower"] <= row["coverage"]
        <= PREREGISTERED_LIMITS["coverage_upper"]
        for row in scenarios
        if row["purpose"] == "coverage"
    )
    null_pass = all(
        row["null_rejection"] is not None
        and row["null_rejection"] <= PREREGISTERED_LIMITS["null_rejection_upper"]
        for row in scenarios
        if row["purpose"] == "null"
    )
    standard_error_pass = all(
        row["relative_se_bias"] is not None
        and row["relative_se_bias"]
        <= PREREGISTERED_LIMITS["relative_se_bias_upper"]
        for row in scenarios
        if row["purpose"] in {"coverage", "null"}
    )
    usable_pass = all(
        row["successful_run_usable_rate"] is not None
        and row["successful_run_usable_rate"]
        >= PREREGISTERED_LIMITS["successful_run_usable_rate_lower"]
        for row in scenarios
        if row["purpose"] in {"coverage", "null"}
    )
    failure_pass = all(
        row["rejected"] / row["datasets"]
        >= PREREGISTERED_LIMITS["failure_scenario_rejection_rate_lower"]
        for row in scenarios
        if row["purpose"] == "failure"
    )
    worker_mismatches = sum(row.get("worker_equal") is False for row in records)
    enough = profile == "qualification" and all(
        row["datasets"] == QUALIFICATION_DATASETS_PER_SCENARIO for row in scenarios
    )
    return {
        "passed": enough
        and coverage_pass
        and null_pass
        and standard_error_pass
        and usable_pass
        and failure_pass
        and worker_mismatches
        == PREREGISTERED_LIMITS["worker_payload_mismatch_count"],
        "qualification_profile_complete": enough,
        "checks": {
            "coverage": coverage_pass,
            "null": null_pass,
            "standard_error_bias": standard_error_pass,
            "usable_replicate_rate": usable_pass,
            "failure": failure_pass,
            "worker_invariance": worker_mismatches == 0,
        },
        "worker_payload_mismatch_count": worker_mismatches,
        "scenarios": scenarios,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=("qualification", "smoke"), default="qualification")
    parser.add_argument("--output", type=Path, default=OUTPUT)
    args = parser.parse_args()
    datasets = QUALIFICATION_DATASETS_PER_SCENARIO if args.profile == "qualification" else 1
    records = []
    with tempfile.TemporaryDirectory(prefix="cbsem_bootstrap_v2_simulation_") as directory:
        work = Path(directory)
        for scenario in SCENARIOS:
            for data_index in range(datasets):
                records.append(
                    run_dataset(
                        work,
                        scenario,
                        data_index,
                        worker_check=data_index == 0,
                    )
                )
    summary = summarize(records, args.profile)
    report = {
        "kind": DESIGN_VERSION,
        "passed": summary["passed"],
        "profile": args.profile,
        "master_seed": MASTER_SEED,
        "bootstrap_replicates": BOOTSTRAP_REPLICATES,
        "datasets_per_scenario": datasets,
        "preregistered_limits": PREREGISTERED_LIMITS,
        "summary": summary,
        "records": records,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, allow_nan=False) + "\n", encoding="utf-8"
    )
    print(json.dumps({"passed": report["passed"], "output": str(args.output)}, indent=2))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
