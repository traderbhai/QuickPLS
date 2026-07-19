#!/usr/bin/env python3
"""Run or inspect sharded studentized Monte Carlo qualification work.

The full studentized qualification is intentionally expensive. This wrapper
keeps shard execution resumable and auditable: valid completed shards are
skipped by default, failed/invalid shards are reported explicitly, and optional
limits allow controlled batches on documented hardware.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST = (
    ROOT / "validation" / "results" / "studentized_qualification_shards" / "manifest.json"
)
DEFAULT_STATUS = (
    ROOT / "validation" / "results" / "studentized_qualification_shards" / "status.json"
)


def root_path(path: str | Path) -> Path:
    value = Path(path)
    return value if value.is_absolute() else ROOT / value


def relative(path: Path) -> str:
    return path.as_posix().removeprefix(ROOT.as_posix() + "/")


def load_json(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def read_manifest(path: Path) -> dict[str, Any]:
    manifest = load_json(path)
    if manifest.get("kind") != "studentized_qualification_shard_manifest_v1":
        raise SystemExit(f"{path}: not a studentized qualification shard manifest")
    if manifest.get("mode") != "studentized-qualification":
        raise SystemExit(f"{path}: manifest mode must be studentized-qualification")
    if not isinstance(manifest.get("shards"), list):
        raise SystemExit(f"{path}: manifest must contain a shards array")
    return manifest


def shard_report_status(shard: dict[str, Any]) -> dict[str, Any]:
    output = root_path(shard["output"])
    base = {
        "name": shard["name"],
        "scenario": shard["scenario"],
        "simulation_offset": shard["simulation_offset"],
        "simulations": shard["simulations"],
        "output": relative(output),
        "status": "missing",
        "completed_simulations": 0,
        "failed_simulations": None,
        "elapsed_seconds": None,
        "message": None,
    }
    if not output.exists():
        return base
    try:
        report = load_json(output)
    except Exception as error:  # noqa: BLE001 - status report should retain parse errors.
        return {**base, "status": "invalid", "message": f"invalid JSON: {error}"}
    if report.get("mode") != "studentized-qualification":
        return {**base, "status": "invalid", "message": "wrong report mode"}
    configuration = report.get("configuration", {})
    if int(configuration.get("simulation_offset", -1)) != int(shard["simulation_offset"]):
        return {**base, "status": "invalid", "message": "simulation_offset mismatch"}
    if int(configuration.get("simulations_per_scenario", -1)) != int(shard["simulations"]):
        return {**base, "status": "invalid", "message": "simulation count mismatch"}
    scenarios = report.get("scenarios")
    if not isinstance(scenarios, list) or len(scenarios) != 1:
        return {**base, "status": "invalid", "message": "expected exactly one scenario"}
    scenario = scenarios[0]
    if scenario.get("name") != shard["scenario"]:
        return {**base, "status": "invalid", "message": "scenario mismatch"}
    completed = int(scenario.get("completed_simulations", 0))
    failed = int(scenario.get("failed_simulations", 0))
    elapsed = report.get("elapsed_seconds")
    if failed > 0:
        status = "failed"
        message = "shard report contains failed simulations"
    elif completed == int(shard["simulations"]):
        status = "complete"
        message = None
    else:
        status = "incomplete"
        message = "completed simulations do not match requested simulations"
    return {
        **base,
        "status": status,
        "completed_simulations": completed,
        "failed_simulations": failed,
        "elapsed_seconds": elapsed,
        "message": message,
    }


def selected_shards(
    shards: list[dict[str, Any]],
    names: set[str] | None,
    scenarios: set[str] | None,
) -> list[dict[str, Any]]:
    selected = []
    for shard in shards:
        if names is not None and shard["name"] not in names:
            continue
        if scenarios is not None and shard["scenario"] not in scenarios:
            continue
        selected.append(shard)
    return selected


def build_status(manifest: dict[str, Any], shards: list[dict[str, Any]]) -> dict[str, Any]:
    statuses = [shard_report_status(shard) for shard in shards]
    by_status: dict[str, int] = {}
    completed_by_scenario: dict[str, int] = {}
    elapsed_by_scenario: dict[str, float] = {}
    for row in statuses:
        by_status[row["status"]] = by_status.get(row["status"], 0) + 1
        completed_by_scenario[row["scenario"]] = completed_by_scenario.get(row["scenario"], 0) + int(
            row["completed_simulations"] or 0
        )
        elapsed_by_scenario[row["scenario"]] = elapsed_by_scenario.get(row["scenario"], 0.0) + float(
            row["elapsed_seconds"] or 0.0
        )
    expected_per_scenario = int(manifest["total_simulations_per_scenario"])
    progress_by_scenario = {
        scenario: {
            "completed_simulations": completed_by_scenario.get(scenario, 0),
            "expected_simulations": expected_per_scenario,
            "completion_rate": completed_by_scenario.get(scenario, 0) / expected_per_scenario
            if expected_per_scenario
            else None,
            "elapsed_seconds": elapsed_by_scenario.get(scenario, 0.0),
        }
        for scenario in manifest["scenarios"]
    }
    return {
        "schema_version": 1,
        "kind": "studentized_qualification_shard_status_v1",
        "manifest": relative(root_path(manifest.get("_manifest_path", ""))),
        "mode": manifest["mode"],
        "total_shards": len(shards),
        "status_counts": by_status,
        "progress_by_scenario": progress_by_scenario,
        "ready_to_aggregate": by_status.get("complete", 0) == len(shards),
        "shards": statuses,
    }


def runnable(status: dict[str, Any], rerun_failed: bool) -> bool:
    if status["status"] in {"missing", "incomplete", "invalid"}:
        return True
    if rerun_failed and status["status"] == "failed":
        return True
    return False


def run_shard(shard: dict[str, Any], dry_run: bool) -> tuple[str, float | None]:
    command = [str(part) for part in shard["command"]]
    print(f"{'[dry-run] ' if dry_run else ''}{shard['name']}: {' '.join(command)}")
    if dry_run:
        return "dry_run", None
    started = time.perf_counter()
    completed = subprocess.run(command, cwd=ROOT, text=True, check=False)
    elapsed = time.perf_counter() - started
    if completed.returncode != 0:
        return f"command_failed:{completed.returncode}", elapsed
    return "executed", elapsed


def write_status(path: Path | None, report: dict[str, Any]) -> None:
    if path is None:
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--status-output", type=Path, default=DEFAULT_STATUS)
    parser.add_argument("--status-only", action="store_true")
    parser.add_argument("--execute", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--rerun-failed", action="store_true")
    parser.add_argument("--max-shards", type=int)
    parser.add_argument("--name", action="append")
    parser.add_argument("--scenario", action="append")
    args = parser.parse_args()

    manifest = read_manifest(args.manifest)
    manifest["_manifest_path"] = str(args.manifest)
    shards = selected_shards(
        manifest["shards"],
        set(args.name) if args.name else None,
        set(args.scenario) if args.scenario else None,
    )
    if not shards:
        raise SystemExit("no shards selected")

    before = build_status(manifest, shards)
    write_status(args.status_output, before)
    print(
        f"status: total={before['total_shards']} counts={before['status_counts']} ready_to_aggregate={before['ready_to_aggregate']}"
    )
    if args.status_only or (not args.execute and not args.dry_run):
        return 0 if before["status_counts"].get("invalid", 0) == 0 else 1

    pending = [shard for shard in shards if runnable(shard_report_status(shard), args.rerun_failed)]
    if args.max_shards is not None:
        if args.max_shards <= 0:
            raise SystemExit("--max-shards must be positive")
        pending = pending[: args.max_shards]
    actions = []
    for shard in pending:
        action, elapsed = run_shard(shard, args.dry_run)
        actions.append({"name": shard["name"], "action": action, "elapsed_seconds": elapsed})
        if action.startswith("command_failed"):
            break

    after = build_status(manifest, shards)
    after["actions"] = actions
    write_status(args.status_output, after)
    print(
        f"after: total={after['total_shards']} counts={after['status_counts']} ready_to_aggregate={after['ready_to_aggregate']}"
    )
    failed_actions = [action for action in actions if str(action["action"]).startswith("command_failed")]
    return 1 if failed_actions or after["status_counts"].get("invalid", 0) else 0


if __name__ == "__main__":
    sys.exit(main())
