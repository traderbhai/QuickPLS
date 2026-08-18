#!/usr/bin/env python3
"""Measure the non-product exact-CFA analytic-studentization Rust example.

This harness records one warm-up and five measured process runs. It deliberately
does not derive, recommend, or enforce product caps and is not qualification or
promotion evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import tempfile
from pathlib import Path
from typing import Any, Sequence

from jsonschema import Draft202012Validator

try:
    from validation.complexity_performance_measure import measure_command_once
    from validation.complexity_performance_v2 import aggregate_runs
except ModuleNotFoundError:
    from complexity_performance_measure import measure_command_once
    from complexity_performance_v2 import aggregate_runs


ROOT = Path(__file__).resolve().parents[1]
SCHEMA = ROOT / "validation/cbsem_exact_case_bootstrap_studentized_benchmark_v1.schema.json"
DEFAULT_OUTPUT = (
    ROOT
    / "validation/results/cbsem_exact_case_bootstrap_studentized_benchmark_v1.json"
)
KIND = "cbsem_exact_case_bootstrap_studentized_benchmark_v1"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def default_binary() -> Path:
    name = "cbsem_exact_case_bootstrap_studentized_benchmark"
    if os.name == "nt":
        name += ".exe"
    return ROOT / "target/release/examples" / name


def command(
    binary: Path,
    result_path: Path,
    *,
    rows: int,
    factors: int,
    replicates: int,
    workers: int,
    seed: int,
    cancel_after: int | None = None,
) -> list[str]:
    argv = [
        str(binary),
        "--repo-root",
        str(ROOT),
        "--output",
        str(result_path),
        "--rows",
        str(rows),
        "--factors",
        str(factors),
        "--replicates",
        str(replicates),
        "--workers",
        str(workers),
        "--seed",
        str(seed),
    ]
    if cancel_after is not None:
        argv.extend(("--cancel-after", str(cancel_after)))
    return argv


def measured_run(
    argv: Sequence[str], result_path: Path, phase: str, index: int
) -> tuple[dict[str, Any], dict[str, Any]]:
    observation = measure_command_once(
        argv,
        cwd=ROOT,
        result_path=result_path,
        phase=phase,
        index=index,
        remove_prior_result=index > 0 or phase != "warmup",
    )
    if observation["exit_code"] != 0:
        raise RuntimeError(f"benchmark child failed: {observation}")
    payload = json.loads(result_path.read_text(encoding="utf-8"))
    observation["scientific_result_sha256"] = payload.get(
        "scientific_result_sha256"
    )
    return observation, payload


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    binary = args.binary.resolve()
    if not binary.is_file():
        raise FileNotFoundError(
            f"{binary} is missing; build it explicitly with "
            "`cargo build --release -p qpls-resampling --example "
            "cbsem_exact_case_bootstrap_studentized_benchmark`"
        )
    with tempfile.TemporaryDirectory(prefix="qpls-cbsem-studentized-benchmark-") as raw:
        result_path = Path(raw) / "run.json"
        argv = command(
            binary,
            result_path,
            rows=args.rows,
            factors=args.factors,
            replicates=args.replicates,
            workers=args.workers,
            seed=args.seed,
        )
        warmup, _ = measured_run(argv, result_path, "warmup", 0)
        measured: list[dict[str, Any]] = []
        representative: dict[str, Any] | None = None
        for index in range(5):
            observation, payload = measured_run(
                argv, result_path, "measured", index
            )
            measured.append(observation)
            representative = payload
        assert representative is not None
        digests = {run["scientific_result_sha256"] for run in measured}
        if None in digests or len(digests) != 1:
            raise RuntimeError("five measured runs are not scientifically deterministic")

        comparison_workers = 1 if args.workers != 1 else 2
        comparison_argv = command(
            binary,
            result_path,
            rows=args.rows,
            factors=args.factors,
            replicates=args.replicates,
            workers=comparison_workers,
            seed=args.seed,
        )
        comparison_observation, comparison = measured_run(
            comparison_argv, result_path, "measured", 5
        )
        worker_invariant = (
            representative["scientific_result_sha256"]
            == comparison["scientific_result_sha256"]
        )
        if not worker_invariant:
            raise RuntimeError("S2 result changed across worker counts")

        cancellation_argv = command(
            binary,
            result_path,
            rows=args.rows,
            factors=args.factors,
            replicates=args.replicates,
            workers=args.workers,
            seed=args.seed,
            cancel_after=min(10, args.replicates - 1),
        )
        cancellation_observation, cancellation = measured_run(
            cancellation_argv, result_path, "measured", 6
        )
        if cancellation.get("status") != "cancelled_as_requested":
            raise RuntimeError("cancellation probe did not reach the typed cancelled outcome")

    aggregates = aggregate_runs(measured)
    return {
        "schema_version": 1,
        "kind": KIND,
        "status": "measurement_only_no_caps_or_promotion",
        "source_binding": {
            "binary": str(binary.relative_to(ROOT)).replace("\\", "/"),
            "binary_sha256": sha256(binary),
            "rust_example": "crates/qpls-resampling/examples/cbsem_exact_case_bootstrap_studentized_benchmark.rs",
            "rust_example_sha256": sha256(
                ROOT
                / "crates/qpls-resampling/examples/cbsem_exact_case_bootstrap_studentized_benchmark.rs"
            ),
            "schema": str(SCHEMA.relative_to(ROOT)).replace("\\", "/"),
            "fixture": representative["fixture"],
        },
        "case": representative["case"],
        "warmup_runs": [warmup],
        "measured_runs": measured,
        "aggregates": aggregates,
        "representative_metrics": representative["metrics"],
        "worker_invariance": {
            "primary_workers": args.workers,
            "comparison_workers": comparison_workers,
            "scientific_result_sha256": representative[
                "scientific_result_sha256"
            ],
            "comparison_elapsed_seconds": comparison_observation["elapsed_seconds"],
            "exact_match": worker_invariant,
        },
        "cancellation": {
            "trigger_after_completed_refits": min(10, args.replicates - 1),
            "terminal_latency_seconds": cancellation[
                "cancellation_latency_seconds"
            ],
            "process_elapsed_seconds": cancellation_observation["elapsed_seconds"],
            "typed_terminal_status": cancellation["status"],
        },
        "cap_decision": {
            "status": "not_evaluated",
            "reason": "A single local measurement cannot establish product N/P/B/worker/byte caps; a predeclared hardware and workload matrix plus accepted baselines is still required.",
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=default_binary())
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--rows", type=int, default=180)
    parser.add_argument("--factors", type=int, choices=(1, 2, 3), default=3)
    parser.add_argument("--replicates", type=int, default=1_000)
    parser.add_argument("--workers", type=int, default=max(1, min(4, os.cpu_count() or 1)))
    parser.add_argument("--seed", type=int, default=91)
    args = parser.parse_args()
    if not 10 <= args.rows <= 180:
        parser.error("--rows must be between 10 and the fixture maximum 180")
    if not 500 <= args.replicates <= 10_000:
        parser.error("--replicates must be between the S2 bounds 500 and 10000")
    if not 1 <= args.workers <= 64:
        parser.error("--workers must be between 1 and 64")
    if args.output.exists():
        parser.error("--output already exists; measurements are append-by-new-path only")
    report = build_report(args)
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    Draft202012Validator(schema).validate(report)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(report, indent=2, sort_keys=True, allow_nan=False) + "\n",
        encoding="utf-8",
    )
    print(f"wrote {args.output} | status={report['status']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
