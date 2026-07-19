#!/usr/bin/env python3
"""Create an execution manifest for sharded studentized qualification runs."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_OUTPUT = ROOT / "validation" / "results" / "studentized_qualification_shards" / "manifest.json"
SCENARIOS = [
    "coverage_beta_0_35",
    "null_beta_0",
    "heavy_tail_coverage_beta_0_35",
    "heavy_tail_null_beta_0",
]


def relative(path: Path) -> str:
    return path.as_posix().removeprefix(ROOT.as_posix() + "/")


def shard_name(scenario: str, offset: int, simulations: int) -> str:
    last = offset + simulations - 1
    return f"{scenario}_{offset:04d}_{last:04d}"


def command_for(scenario: str, offset: int, simulations: int, output: Path) -> list[str]:
    return [
        "cargo",
        "run",
        "--release",
        "--manifest-path",
        "validation/monte_carlo/Cargo.toml",
        "--",
        "--mode",
        "studentized-qualification",
        "--scenario",
        scenario,
        "--simulations",
        str(simulations),
        "--simulation-offset",
        str(offset),
        "--output",
        relative(output),
    ]


def powershell_command(command: list[str]) -> str:
    return " ".join(command)


def build_manifest(total: int, chunk: int, output: Path) -> dict[str, object]:
    shard_dir = output.parent
    shards = []
    for scenario in SCENARIOS:
        for offset in range(0, total, chunk):
            simulations = min(chunk, total - offset)
            name = shard_name(scenario, offset, simulations)
            shard_output = shard_dir / f"{name}.json"
            command = command_for(scenario, offset, simulations, shard_output)
            shards.append(
                {
                    "name": name,
                    "scenario": scenario,
                    "simulation_offset": offset,
                    "simulations": simulations,
                    "output": relative(shard_output),
                    "command": command,
                    "powershell": powershell_command(command),
                }
            )
    return {
        "schema_version": 1,
        "kind": "studentized_qualification_shard_manifest_v1",
        "mode": "studentized-qualification",
        "total_simulations_per_scenario": total,
        "chunk_size": chunk,
        "scenario_count": len(SCENARIOS),
        "shard_count": len(shards),
        "requested_inner_fits_per_simulation": 999 * 99,
        "requested_inner_fits_total": len(SCENARIOS) * total * 999 * 99,
        "scenarios": SCENARIOS,
        "shards": shards,
        "aggregate_command": [
            "python",
            "validation/aggregate_studentized_qualification.py",
            "--input",
            relative(shard_dir),
            "--output",
            "validation/results/monte_carlo_studentized_qualification.json",
        ],
        "note": "Run every shard command on documented hardware, then run aggregate_command. The final aggregate is accepted only when every preregistered scenario reaches 1,000 completed simulations with zero failures and all qualification checks pass.",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--total", type=int, default=1_000)
    parser.add_argument("--chunk", type=int, default=100)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args()
    if args.total <= 0:
        raise SystemExit("--total must be positive")
    if args.chunk <= 0:
        raise SystemExit("--chunk must be positive")
    manifest = build_manifest(args.total, args.chunk, args.output)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    print(
        f"wrote {args.output} | shards={manifest['shard_count']} | requested_inner_fits={manifest['requested_inner_fits_total']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
