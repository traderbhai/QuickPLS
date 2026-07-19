#!/usr/bin/env python3
"""Aggregate sharded studentized Monte Carlo qualification reports."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_INPUT = ROOT / "validation" / "results" / "studentized_qualification_shards"
DEFAULT_OUTPUT = ROOT / "validation" / "results" / "monte_carlo_studentized_qualification.json"
EXPECTED_SCENARIOS = [
    "coverage_beta_0_35",
    "null_beta_0",
    "heavy_tail_coverage_beta_0_35",
    "heavy_tail_null_beta_0",
]
THRESHOLDS = {
    "coverage_lower": 0.925,
    "coverage_upper": 0.975,
    "type_i_lower": 0.025,
    "type_i_upper": 0.075,
    "maximum_absolute_bias": 0.03,
    "minimum_bca_availability": 1.0,
    "minimum_studentized_availability": 0.99,
    "minimum_normal_reference_availability": 1.0,
    "minimum_usable_bootstrap_rate": 0.99,
}


def rate(numerator: int, denominator: int) -> float | None:
    return numerator / denominator if denominator else None


def count_from_rate(value: float | None, denominator: int) -> int:
    if value is None:
        return 0
    return round(value * denominator)


def passes_coverage(value: float | None) -> bool:
    return value is not None and THRESHOLDS["coverage_lower"] <= value <= THRESHOLDS["coverage_upper"]


def passes_type_i(value: float | None) -> bool:
    return value is not None and THRESHOLDS["type_i_lower"] <= value <= THRESHOLDS["type_i_upper"]


def passes_bias(value: float | None) -> bool:
    return value is not None and value <= THRESHOLDS["maximum_absolute_bias"]


def passes_minimum(value: float | None) -> bool:
    return value is not None and value >= THRESHOLDS["minimum_usable_bootstrap_rate"]


def scenario_accumulator(scenario: dict[str, Any]) -> dict[str, Any]:
    completed = int(scenario["completed_simulations"])
    failed = int(scenario["failed_simulations"])
    percentile_available = int(scenario["percentile"]["available"])
    bca_available = int(scenario["bca"]["available"])
    studentized_available = int(scenario["studentized"]["available"])
    normal_available = int(scenario["normal_reference_available"])
    return {
        "name": scenario["name"],
        "error_distribution": scenario["error_distribution"],
        "true_path": float(scenario["true_path"]),
        "requested_simulations": int(scenario["requested_simulations"]),
        "completed_simulations": completed,
        "failed_simulations": failed,
        "estimate_sum": float(scenario["mean_estimate"]) * completed if completed else 0.0,
        "usable_rate_sum": float(scenario["mean_usable_bootstrap_rate"]) * completed
        if completed
        else 0.0,
        "percentile_available": percentile_available,
        "percentile_coverage_count": count_from_rate(
            scenario["percentile"]["coverage_rate"], percentile_available
        ),
        "percentile_exclusion_count": count_from_rate(
            scenario["percentile"]["exclusion_of_zero_rate"], percentile_available
        ),
        "bca_available": bca_available,
        "bca_coverage_count": count_from_rate(scenario["bca"]["coverage_rate"], bca_available),
        "bca_exclusion_count": count_from_rate(
            scenario["bca"]["exclusion_of_zero_rate"], bca_available
        ),
        "studentized_available": studentized_available,
        "studentized_coverage_count": count_from_rate(
            scenario["studentized"]["coverage_rate"], studentized_available
        ),
        "studentized_exclusion_count": count_from_rate(
            scenario["studentized"]["exclusion_of_zero_rate"], studentized_available
        ),
        "normal_reference_available": normal_available,
        "normal_type_i_count": count_from_rate(
            scenario["normal_reference_type_i_rate"], normal_available
        ),
        "failures": list(scenario.get("failures", [])),
    }


def merge_accumulators(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    if (
        left["name"] != right["name"]
        or left["error_distribution"] != right["error_distribution"]
        or left["true_path"] != right["true_path"]
    ):
        raise ValueError("cannot merge mismatched scenarios")
    merged = dict(left)
    for key in [
        "requested_simulations",
        "completed_simulations",
        "failed_simulations",
        "estimate_sum",
        "usable_rate_sum",
        "percentile_available",
        "percentile_coverage_count",
        "percentile_exclusion_count",
        "bca_available",
        "bca_coverage_count",
        "bca_exclusion_count",
        "studentized_available",
        "studentized_coverage_count",
        "studentized_exclusion_count",
        "normal_reference_available",
        "normal_type_i_count",
    ]:
        merged[key] += right[key]
    merged["failures"] = (left["failures"] + right["failures"])[:20]
    return merged


def finalize_scenario(acc: dict[str, Any]) -> dict[str, Any]:
    completed = acc["completed_simulations"]
    mean_estimate = acc["estimate_sum"] / completed if completed else None
    return {
        "name": acc["name"],
        "error_distribution": acc["error_distribution"],
        "true_path": acc["true_path"],
        "requested_simulations": acc["requested_simulations"],
        "completed_simulations": completed,
        "failed_simulations": acc["failed_simulations"],
        "mean_estimate": mean_estimate,
        "bias": mean_estimate - acc["true_path"] if mean_estimate is not None else None,
        "mean_usable_bootstrap_rate": acc["usable_rate_sum"] / completed if completed else None,
        "percentile": {
            "available": acc["percentile_available"],
            "coverage_rate": rate(acc["percentile_coverage_count"], acc["percentile_available"]),
            "exclusion_of_zero_rate": rate(
                acc["percentile_exclusion_count"], acc["percentile_available"]
            ),
        },
        "bca": {
            "available": acc["bca_available"],
            "coverage_rate": rate(acc["bca_coverage_count"], acc["bca_available"]),
            "exclusion_of_zero_rate": rate(acc["bca_exclusion_count"], acc["bca_available"]),
        },
        "studentized": {
            "available": acc["studentized_available"],
            "coverage_rate": rate(
                acc["studentized_coverage_count"], acc["studentized_available"]
            ),
            "exclusion_of_zero_rate": rate(
                acc["studentized_exclusion_count"], acc["studentized_available"]
            ),
        },
        "normal_reference_available": acc["normal_reference_available"],
        "normal_reference_type_i_rate": rate(
            acc["normal_type_i_count"], acc["normal_reference_available"]
        )
        if acc["true_path"] == 0.0
        else None,
        "failures": acc["failures"],
    }


def check(metric: str, observed: float | None, passed: bool) -> dict[str, Any]:
    return {"metric": metric, "observed": observed, "passed": passed}


def qualification(scenarios: list[dict[str, Any]]) -> dict[str, Any]:
    by_name = {scenario["name"]: scenario for scenario in scenarios}
    enough = all(
        name in by_name
        and by_name[name]["completed_simulations"] >= 1_000
        and by_name[name]["failed_simulations"] == 0
        for name in EXPECTED_SCENARIOS
    )
    normal_alt = by_name.get("coverage_beta_0_35")
    normal_null = by_name.get("null_beta_0")
    heavy_alt = by_name.get("heavy_tail_coverage_beta_0_35")
    heavy_null = by_name.get("heavy_tail_null_beta_0")
    rows = []
    if normal_alt and normal_null:
        rows.extend(
            [
                check(
                    "percentile_coverage",
                    normal_alt["percentile"]["coverage_rate"],
                    passes_coverage(normal_alt["percentile"]["coverage_rate"]),
                ),
                check(
                    "bca_coverage",
                    normal_alt["bca"]["coverage_rate"],
                    passes_coverage(normal_alt["bca"]["coverage_rate"]),
                ),
                check(
                    "percentile_type_i",
                    normal_null["percentile"]["exclusion_of_zero_rate"],
                    passes_type_i(normal_null["percentile"]["exclusion_of_zero_rate"]),
                ),
                check(
                    "bca_type_i",
                    normal_null["bca"]["exclusion_of_zero_rate"],
                    passes_type_i(normal_null["bca"]["exclusion_of_zero_rate"]),
                ),
                check(
                    "studentized_coverage",
                    normal_alt["studentized"]["coverage_rate"],
                    passes_coverage(normal_alt["studentized"]["coverage_rate"]),
                ),
                check(
                    "studentized_type_i",
                    normal_null["studentized"]["exclusion_of_zero_rate"],
                    passes_type_i(normal_null["studentized"]["exclusion_of_zero_rate"]),
                ),
                check("absolute_bias", abs(normal_alt["bias"]), passes_bias(abs(normal_alt["bias"]))),
                check(
                    "alternative_studentized_availability",
                    rate(normal_alt["studentized"]["available"], normal_alt["completed_simulations"]),
                    passes_minimum(
                        rate(normal_alt["studentized"]["available"], normal_alt["completed_simulations"])
                    ),
                ),
                check(
                    "null_studentized_availability",
                    rate(normal_null["studentized"]["available"], normal_null["completed_simulations"]),
                    passes_minimum(
                        rate(normal_null["studentized"]["available"], normal_null["completed_simulations"])
                    ),
                ),
            ]
        )
    if heavy_alt and heavy_null:
        rows.extend(
            [
                check(
                    "heavy_tail_studentized_coverage",
                    heavy_alt["studentized"]["coverage_rate"],
                    passes_coverage(heavy_alt["studentized"]["coverage_rate"]),
                ),
                check(
                    "heavy_tail_studentized_type_i",
                    heavy_null["studentized"]["exclusion_of_zero_rate"],
                    passes_type_i(heavy_null["studentized"]["exclusion_of_zero_rate"]),
                ),
                check(
                    "heavy_tail_alternative_studentized_availability",
                    rate(heavy_alt["studentized"]["available"], heavy_alt["completed_simulations"]),
                    passes_minimum(
                        rate(heavy_alt["studentized"]["available"], heavy_alt["completed_simulations"])
                    ),
                ),
                check(
                    "heavy_tail_null_studentized_availability",
                    rate(heavy_null["studentized"]["available"], heavy_null["completed_simulations"]),
                    passes_minimum(
                        rate(heavy_null["studentized"]["available"], heavy_null["completed_simulations"])
                    ),
                ),
            ]
        )
    if not enough:
        for row in rows:
            row["passed"] = None
    return {
        "evaluated": enough,
        "passed": all(row["passed"] is True for row in rows) if enough else None,
        "minimum_simulations_per_scenario": 1_000,
        "thresholds": {
            "coverage_lower": THRESHOLDS["coverage_lower"],
            "coverage_upper": THRESHOLDS["coverage_upper"],
            "type_i_lower": THRESHOLDS["type_i_lower"],
            "type_i_upper": THRESHOLDS["type_i_upper"],
            "maximum_absolute_bias": THRESHOLDS["maximum_absolute_bias"],
            "minimum_bca_availability": THRESHOLDS["minimum_bca_availability"],
            "minimum_studentized_availability": THRESHOLDS["minimum_studentized_availability"],
            "minimum_normal_reference_availability": THRESHOLDS[
                "minimum_normal_reference_availability"
            ],
            "minimum_usable_bootstrap_rate": THRESHOLDS["minimum_usable_bootstrap_rate"],
        },
        "checks": rows,
        "note": "Aggregated studentized qualification shards; thresholds are evaluated only after each preregistered scenario reaches at least 1,000 completed simulations with zero failures.",
    }


def input_files(path: Path) -> list[Path]:
    if path.is_dir():
        manifest = path / "manifest.json"
        if manifest.exists():
            report = json.loads(manifest.read_text(encoding="utf-8"))
            if report.get("kind") == "studentized_qualification_shard_manifest_v1":
                return [
                    ROOT / shard["output"]
                    for shard in report.get("shards", [])
                    if (ROOT / shard["output"]).exists()
                ]
        return sorted(path.glob("*.json"))
    return [path]


def is_shard_report(report: dict[str, Any]) -> bool:
    scenarios = report.get("scenarios")
    return (
        report.get("mode") == "studentized-qualification"
        and "aggregation" not in report
        and isinstance(scenarios, list)
        and all(isinstance(scenario, dict) for scenario in scenarios)
    )


def scenario_range(report: dict[str, Any], scenario: dict[str, Any], file: Path) -> tuple[int, int]:
    configuration = report.get("configuration", {})
    offset = int(configuration.get("simulation_offset", 0))
    requested = int(scenario["requested_simulations"])
    if offset < 0:
        raise SystemExit(f"{file}: simulation_offset must be non-negative")
    if requested <= 0:
        raise SystemExit(f"{file}: requested_simulations must be positive")
    return offset, offset + requested


def record_range(
    ranges: dict[str, list[tuple[int, int, str]]],
    scenario_name: str,
    current: tuple[int, int],
    file: Path,
) -> None:
    start, end = current
    for previous_start, previous_end, previous_file in ranges.get(scenario_name, []):
        if start < previous_end and previous_start < end:
            raise SystemExit(
                f"overlapping shards for {scenario_name}: "
                f"{file} covers [{start}, {end}) and "
                f"{previous_file} covers [{previous_start}, {previous_end})"
            )
    ranges.setdefault(scenario_name, []).append((start, end, str(file)))


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--allow-incomplete", action="store_true")
    args = parser.parse_args()
    files = input_files(args.input)
    if not files:
        raise SystemExit(f"no shard JSON files found under {args.input}")
    accumulators: dict[str, dict[str, Any]] = {}
    shard_ranges: dict[str, list[tuple[int, int, str]]] = {}
    reports = []
    skipped_sources = []
    for file in files:
        report = json.loads(file.read_text(encoding="utf-8"))
        if not is_shard_report(report):
            skipped_sources.append(str(file))
            continue
        reports.append(report)
        for scenario in report["scenarios"]:
            range_start, range_end = scenario_range(report, scenario, file)
            record_range(shard_ranges, scenario["name"], (range_start, range_end), file)
            acc = scenario_accumulator(scenario)
            name = acc["name"]
            accumulators[name] = (
                merge_accumulators(accumulators[name], acc) if name in accumulators else acc
            )
    if not reports:
        raise SystemExit(f"no studentized-qualification shard JSON files found under {args.input}")
    scenarios = [finalize_scenario(accumulators[name]) for name in sorted(accumulators)]
    first = reports[0]
    simulations_per_scenario = min(
        (
            scenario["completed_simulations"]
            for scenario in scenarios
            if scenario["name"] in EXPECTED_SCENARIOS
        ),
        default=0,
    )
    output = {
        "schema_version": first["schema_version"],
        "harness_version": first["harness_version"],
        "engine_versions": first["engine_versions"],
        "mode": "studentized-qualification",
        "configuration": {
            **first["configuration"],
            "simulations_per_scenario": simulations_per_scenario,
            "simulation_offset": 0,
        },
        "dgp": first["dgp"],
        "scenarios": scenarios,
        "qualification": qualification(scenarios),
        "elapsed_seconds": sum(float(report["elapsed_seconds"]) for report in reports),
        "aggregation": {
            "kind": "studentized_qualification_shard_aggregation_v1",
            "source_count": len(files),
            "shard_count": len(reports),
            "sources": [str(file) for file in files],
            "skipped_sources": skipped_sources,
            "scenario_ranges": {
                name: [
                    {"start": start, "end": end, "source": source}
                    for start, end, source in sorted(ranges)
                ]
                for name, ranges in sorted(shard_ranges.items())
            },
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(output, indent=2) + "\n", encoding="utf-8")
    print(
        f"wrote {args.output} | evaluated={output['qualification']['evaluated']} | passed={output['qualification']['passed']}"
    )
    return 0 if output["qualification"]["evaluated"] or args.allow_incomplete else 1


if __name__ == "__main__":
    raise SystemExit(main())
