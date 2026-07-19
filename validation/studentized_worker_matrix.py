#!/usr/bin/env python3
"""Run and compare the bounded 999x99 studentized bootstrap across worker counts."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import subprocess
import sys
import time
from pathlib import Path
from typing import Any


BASE_WORKERS = (1, 2, 4)
BOOTSTRAP_SAMPLES = 999
STUDENTIZED_INNER_SAMPLES = 99


def repo_root() -> Path:
    return Path(__file__).resolve().parents[1]


def stable_json_hash(value: Any) -> str:
    encoded = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False).encode(
        "utf-8"
    )
    return hashlib.sha256(encoded).hexdigest()


def detected_max_workers() -> int:
    return max(1, os.cpu_count() or 1)


def planned_workers() -> tuple[int, ...]:
    workers = [*BASE_WORKERS, detected_max_workers()]
    return tuple(dict.fromkeys(worker for worker in workers if worker >= 1))


def max_abs_difference(left: Any, right: Any) -> float:
    if isinstance(left, bool) or isinstance(right, bool):
        return 0.0
    if isinstance(left, (int, float)) and isinstance(right, (int, float)):
        return abs(float(left) - float(right))
    if isinstance(left, list) and isinstance(right, list):
        if len(left) != len(right):
            return float("inf")
        return max((max_abs_difference(a, b) for a, b in zip(left, right)), default=0.0)
    if isinstance(left, dict) and isinstance(right, dict):
        if set(left) != set(right):
            return float("inf")
        return max((max_abs_difference(left[key], right[key]) for key in left), default=0.0)
    return 0.0 if left == right else float("inf")


def run_worker(root: Path, output_dir: Path, workers: int) -> dict[str, Any]:
    output = output_dir / f"studentized_worker_{workers}.json"
    command = [
        "cargo",
        "run",
        "-p",
        "qpls-cli",
        "--",
        "run",
        "validation/fixtures/simple_reflective.recipe.json",
        "--data",
        "validation/fixtures/simple_reflective.csv",
        "--output",
        str(output),
        "--allow-experimental",
        "--bootstrap-samples",
        str(BOOTSTRAP_SAMPLES),
        "--studentized-inner-samples",
        str(STUDENTIZED_INNER_SAMPLES),
        "--workers",
        str(workers),
    ]
    started = time.perf_counter()
    completed = subprocess.run(command, cwd=root, text=True, capture_output=True, check=False)
    elapsed = time.perf_counter() - started
    if completed.returncode != 0:
        raise RuntimeError(
            f"studentized run failed for workers={workers}\n"
            f"stdout:\n{completed.stdout}\n\nstderr:\n{completed.stderr}"
        )
    with output.open("r", encoding="utf-8") as handle:
        envelope = json.load(handle)
    payload = envelope.get("payload")
    bootstrap = payload.get("bootstrap", {}) if isinstance(payload, dict) else {}
    studentized = bootstrap.get("studentized", {}) if isinstance(bootstrap, dict) else {}
    parameters = studentized.get("parameters") if isinstance(studentized, dict) else None
    unavailable = [
        row
        for row in parameters or []
        if isinstance(row, dict) and row.get("unavailable_reason") is not None
    ]
    return {
        "workers": workers,
        "output": str(output),
        "elapsed_seconds": elapsed,
        "status": envelope.get("status"),
        "settings_workers": envelope.get("provenance", {}).get("settings", {}).get("workers"),
        "usable_replicates": bootstrap.get("usable_replicates"),
        "studentized_method_version": studentized.get("method_version"),
        "studentized_inner_replicates": studentized.get("inner_replicates"),
        "studentized_failure": studentized.get("failure"),
        "studentized_parameter_count": len(parameters or []),
        "studentized_available_parameter_count": len(parameters or []) - len(unavailable),
        "studentized_unavailable_parameter_count": len(unavailable),
        "payload_hash": stable_json_hash(payload),
        "diagnostics_hash": stable_json_hash(envelope.get("diagnostics")),
        "payload": payload,
        "diagnostics": envelope.get("diagnostics"),
    }


def build_report(root: Path, output: Path) -> dict[str, Any]:
    artifact_dir = root / "validation" / "results" / "studentized_worker_matrix"
    artifact_dir.mkdir(parents=True, exist_ok=True)
    workers = planned_workers()
    runs = [run_worker(root, artifact_dir, worker_count) for worker_count in workers]
    baseline = runs[0]
    comparisons = []
    for run in runs[1:]:
        payload_equal = run["payload_hash"] == baseline["payload_hash"]
        diagnostics_equal = run["diagnostics_hash"] == baseline["diagnostics_hash"]
        comparisons.append(
            {
                "baseline_workers": baseline["workers"],
                "comparison_workers": run["workers"],
                "payload_equal": payload_equal,
                "diagnostics_equal": diagnostics_equal,
                "max_payload_abs_difference": max_abs_difference(
                    baseline["payload"], run["payload"]
                ),
            }
        )
    summary_runs = [
        {key: value for key, value in run.items() if key not in {"payload", "diagnostics"}}
        for run in runs
    ]
    min_elapsed = min(run["elapsed_seconds"] for run in runs)
    max_elapsed = max(run["elapsed_seconds"] for run in runs)
    passed = all(run["status"] == "completed" for run in runs) and all(
        comparison["payload_equal"] and comparison["diagnostics_equal"]
        for comparison in comparisons
    )
    report = {
        "schema_version": 1,
        "kind": "studentized_worker_matrix_v1",
        "passed": passed,
        "fixture": {
            "recipe": "validation/fixtures/simple_reflective.recipe.json",
            "data": "validation/fixtures/simple_reflective.csv",
            "bootstrap_samples": BOOTSTRAP_SAMPLES,
            "studentized_inner_samples": STUDENTIZED_INNER_SAMPLES,
            "workers": list(workers),
            "detected_max_workers": detected_max_workers(),
        },
        "runs": summary_runs,
        "comparisons": comparisons,
        "performance": {
            "min_elapsed_seconds": min_elapsed,
            "max_elapsed_seconds": max_elapsed,
            "max_to_min_elapsed_ratio": max_elapsed / min_elapsed if min_elapsed else None,
        },
        "artifact_directory": str(artifact_dir),
        "note": "Bounded worker-count and timing evidence for the 999x99 nested studentized bootstrap fixture; this does not replace broader coverage and stress qualification.",
    }
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2)
        handle.write("\n")
    return report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=repo_root() / "validation" / "results" / "studentized_worker_matrix.json",
    )
    args = parser.parse_args()
    root = repo_root()
    report = build_report(root, args.output)
    print(
        f"wrote {args.output} | passed={report['passed']} | "
        f"max_elapsed_seconds={report['performance']['max_elapsed_seconds']:.3f}"
    )
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
